//! A `double` to its decimal digits (`systems/crt/decimal.md`): `___dtold` widens it to the 80-bit
//! real, `$I10_OUTPUT` makes up to 21 digits of that, and `_fltout2` keeps 17 of them as the
//! `STRFLT` the conversions read.

use crate::ld12::{add12, ld12mul, multtenpow12, shl12, shr12, Ld12, TenPowers};

/// The 80-bit real: the 64-bit mantissa and the sign over the 15-bit exponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ld10 {
    pub man: u64,
    pub exp: u16,
}

/// `___dtold` (`0x40ffcc`): the double's sign, its exponent rebiased by 0x3c00 (0x7fff for 0x7ff),
/// its 52 bits under an explicit leading 1, and a subnormal normalized from exponent 0x3c01. Either
/// zero is all zero bits, the sign of -0 dropped.
pub fn dtold(bits: u64) -> Ld10 {
    let sign = ((bits >> 48) as u16) & 0x8000;
    let e = ((bits >> 52) & 0x7ff) as u32;
    let frac = bits & ((1 << 52) - 1);
    if e == 0 && frac == 0 {
        return Ld10 { man: 0, exp: 0 };
    }
    let (mut x, lead) = match e {
        0 => (0x3c01u32, 0),
        0x7ff => (0x7fff, 1u64 << 63),
        _ => (e + 0x3c00, 1 << 63),
    };
    let mut man = frac << 11 | lead;
    while man >> 63 == 0 {
        man <<= 1;
        x = x.wrapping_add(0xffff);
    }
    Ld10 {
        man,
        exp: sign | x as u16,
    }
}

/// `$I10_OUTPUT`'s result, the `FOS` it fills: the decimal exponent (the value is
/// `0.d1d2... * 10^exp`), the sign as `-` or a space, the digit count and the 22-byte digit
/// buffer, a NUL after the kept digits and the digits it made past them left in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fos {
    pub exp: i16,
    pub sign: u8,
    pub len: u8,
    pub man: [u8; 22],
}

impl Fos {
    pub fn digits(&self) -> &[u8] {
        let end = self.man.iter().position(|&b| b == 0).unwrap_or(22);
        &self.man[..end]
    }
}

/// 0.1 as `$I10_OUTPUT` builds it on its stack: `0xcccc` in every mantissa word, truncated.
const TENTH: Ld12 = [0xcccc, 0xcccc, 0xcccc, 0xcccc, 0xcccc, 0x3ffb];

fn put(fos: &mut Fos, at: usize, s: &[u8]) {
    fos.man[at..at + s.len()].copy_from_slice(s);
    fos.man[at + s.len()] = 0;
}

/// The zero answer: exponent 0, a space, the one digit `0`.
fn zero(fos: &mut Fos) -> i32 {
    fos.exp = 0;
    fos.sign = b' ';
    fos.len = 1;
    fos.man[0] = b'0';
    fos.man[1] = 0;
    1
}

/// `$I10_OUTPUT` (`0x4116fd`): the digits of `ld` into `fos`, `ndigits` of them (21 at most,
/// one more made for the rounding), or with bit 0 of `flags` `ndigits` after the point. It
/// returns 1, or 0 for a non-finite value, which it spells `1#INF`, `1#IND`, `1#QNAN` or `1#SNAN`
/// with exponent 1. `fos` comes in with whatever bytes its caller left there.
pub fn i10_output(ld: Ld10, ndigits: i32, flags: u32, fos: &mut Fos, t: &TenPowers) -> i32 {
    let sign = ld.exp & 0x8000;
    let expn = ld.exp & 0x7fff;
    let hi = (ld.man >> 32) as u32;
    let lo = ld.man as u32;
    fos.sign = if sign != 0 { b'-' } else { b' ' };
    if expn == 0 && hi == 0 && lo == 0 {
        return zero(fos);
    }
    if expn == 0x7fff {
        fos.exp = 1;
        let text: &[u8] = if (hi != 0x8000_0000 || lo != 0) && hi & 0x4000_0000 == 0 {
            b"1#SNAN"
        } else if sign != 0 && hi == 0xc000_0000 {
            if lo != 0 {
                b"1#QNAN"
            } else {
                b"1#IND"
            }
        } else if hi == 0x8000_0000 && lo == 0 {
            b"1#INF"
        } else {
            b"1#QNAN"
        };
        put(fos, 0, text);
        fos.len = text.len() as u8;
        return 0;
    }
    // The estimate of the decimal exponent: a 16.16 fixed-point log10 of the value, low.
    let est = (u32::from(expn) >> 8)
        .wrapping_add((hi >> 24).wrapping_mul(2))
        .wrapping_mul(0x4d)
        .wrapping_add(u32::from(expn).wrapping_mul(0x4d10))
        .wrapping_sub(0x1343_12f4);
    let mut r = ((est as i32) >> 16) as i16;
    let mut v: Ld12 = [
        0,
        lo as u16,
        (lo >> 16) as u16,
        hi as u16,
        (hi >> 16) as u16,
        expn,
    ];
    multtenpow12(&mut v, -i32::from(r), true, t);
    if v[5] >= 0x3fff {
        r = r.wrapping_add(1);
        ld12mul(&mut v, &TENTH);
    }
    fos.exp = r;
    let mut n = ndigits;
    if flags & 1 != 0 {
        n = n.wrapping_add(i32::from(r));
        if n <= 0 {
            return zero(fos);
        }
    }
    if n > 21 {
        n = 21;
    }
    // To 8.88 fixed point: the exponent word cleared and the 96 bits shifted up a byte, then down
    // by how far the exponent is below 0x3ffe.
    let shift = i32::from(v[5]) - 0x3ffe;
    v[5] = 0;
    for _ in 0..8 {
        shl12(&mut v);
    }
    if shift < 0 {
        for _ in 0..((-shift) & 0xff) {
            shr12(&mut v);
        }
    }
    // Each digit is the top byte after the fraction is multiplied by ten: (4v + v) * 2.
    let count = n + 1;
    let mut p = 0usize;
    if count > 0 {
        for _ in 0..count {
            let copy = v;
            shl12(&mut v);
            shl12(&mut v);
            add12(&mut v, &copy);
            shl12(&mut v);
            let digit = (v[5] >> 8) as u8;
            v[5] &= 0x00ff;
            fos.man[p] = digit.wrapping_add(b'0');
            p += 1;
        }
    }
    // Round on the last digit made: half up, nines carrying; else drop the trailing zeros.
    if p == 0 {
        // No digit made (a negative `ndigits` without the flag): the byte before the digits is
        // the count, read as the last digit and, when 5 or more, incremented as the carry.
        if fos.len >= b'5' {
            fos.exp = fos.exp.wrapping_add(1);
            fos.len = 0;
            fos.man[0] = 0;
            return 1;
        }
        return zero(fos);
    }
    let last = fos.man[p - 1];
    let mut i = p as isize - 2;
    if last >= b'5' {
        while i >= 0 && fos.man[i as usize] == b'9' {
            fos.man[i as usize] = b'0';
            i -= 1;
        }
        if i < 0 {
            i += 1;
            fos.exp = fos.exp.wrapping_add(1);
        }
        fos.man[i as usize] = fos.man[i as usize].wrapping_add(1);
    } else {
        while i >= 0 && fos.man[i as usize] == b'0' {
            i -= 1;
        }
        if i < 0 {
            return zero(fos);
        }
    }
    let len = (i + 1) as u8;
    fos.len = len;
    fos.man[len as usize] = 0;
    1
}

/// `_fltout2`'s `STRFLT`: the sign as `-` or a space, the decimal exponent, `$I10_OUTPUT`'s return,
/// and the digits it copied into its caller's buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrFlt {
    pub sign: u8,
    pub decpt: i32,
    pub flag: i32,
    pub mantissa: Vec<u8>,
}

/// `_fltout2` (`0x40ff70`): `___dtold` of the double, `$I10_OUTPUT` of that for 17 digits with no
/// flags, and the digits copied out.
pub fn fltout2(bits: u64, t: &TenPowers) -> StrFlt {
    let ld = dtold(bits);
    let mut fos = Fos {
        exp: 0,
        sign: 0,
        len: 0,
        man: [0; 22],
    };
    let flag = i10_output(ld, 17, 0, &mut fos, t);
    StrFlt {
        sign: fos.sign,
        decpt: i32::from(fos.exp),
        flag,
        mantissa: fos.digits().to_vec(),
    }
}
