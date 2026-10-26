//! Text to a `double` (`systems/crt/strtod.md`): `___strgtold12` scans a decimal numeral into up
//! to 24 digits and a power of ten, `___mtold12` makes the digits a 12-byte real, `___multtenpow12`
//! scales it, `__ld12cvt` rounds it to the double, and `__fltin2` and `strtod` read the flags.

use crate::ld12::{add12, multtenpow12, shl12, Ld12, TenPowers};

/// The digits `___strgtold12` keeps: 25 are stored, 24 converted.
const KEPT: usize = 25;

/// `___strgtold12`'s answer: the 12-byte real, the index after the numeral, and its flags: 1 an
/// exponent below -5200, 2 one above 5200, 4 no digit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scan {
    pub ld: Ld12,
    pub end: usize,
    pub flags: u32,
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn nonzero_digit(c: u8) -> bool {
    (b'1'..=b'9').contains(&c)
}

fn exponent_mark(c: u8) -> bool {
    matches!(c, b'D' | b'E' | b'd' | b'e')
}

/// `___strgtold12` (`0x4111f0`) over `s`, read as a C string: its bytes, then NULs. `mult12`,
/// `scale`, `decpt` and `implicit_e` are its last four arguments, all 0 from `__fltin2`.
pub fn strgtold12(
    s: &[u8],
    mult12: bool,
    scale: i32,
    decpt: i32,
    implicit_e: bool,
    t: &TenPowers,
) -> Scan {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut p = 0usize;
    // The index a failed numeral ends at: the start, before the blanks, until an exponent mark.
    let mut saved = 0usize;
    while matches!(at(p), b' ' | b'\t' | b'\n' | b'\r') {
        p += 1;
    }
    let mut digits = [0u8; KEPT];
    let mut manlen = 0usize;
    let mut manexp: i32 = 0;
    let (mut found_digit, mut found_point, mut found_exp) = (false, false, false);
    let mut sign: u16 = 0;
    let mut exp_neg = false;
    let mut exp: i32 = 0;
    let mut state = 0;
    loop {
        let mut c = at(p);
        p += 1;
        match state {
            0 => {
                if nonzero_digit(c) {
                    state = 3;
                    p -= 1;
                } else if c == b'.' {
                    state = 5;
                } else if c == b'+' {
                    sign = 0;
                    state = 2;
                } else if c == b'-' {
                    sign = 0x8000;
                    state = 2;
                } else if c == b'0' {
                    state = 1;
                } else {
                    p -= 1;
                    break;
                }
            }
            1 => {
                found_digit = true;
                if nonzero_digit(c) {
                    state = 3;
                    p -= 1;
                } else if c == b'.' {
                    state = 4;
                } else if c == b'+' || c == b'-' {
                    p -= 1;
                    state = 11;
                } else if c == b'0' {
                    state = 1;
                } else if exponent_mark(c) {
                    state = 6;
                } else {
                    p -= 1;
                    break;
                }
            }
            2 => {
                if nonzero_digit(c) {
                    state = 3;
                    p -= 1;
                } else if c == b'.' {
                    state = 5;
                } else if c == b'0' {
                    state = 1;
                } else {
                    p = saved;
                    break;
                }
            }
            3 | 4 => {
                found_digit = true;
                if state == 4 {
                    found_point = true;
                    if manlen == 0 {
                        while c == b'0' {
                            manexp -= 1;
                            c = at(p);
                            p += 1;
                        }
                    }
                }
                while is_digit(c) {
                    if manlen < KEPT {
                        digits[manlen] = c - b'0';
                        manlen += 1;
                        if state == 4 {
                            manexp -= 1;
                        }
                    } else if state == 3 {
                        manexp += 1;
                    }
                    c = at(p);
                    p += 1;
                }
                if state == 3 && c == b'.' {
                    state = 4;
                } else if c == b'+' || c == b'-' {
                    p -= 1;
                    state = 11;
                } else if exponent_mark(c) {
                    state = 6;
                } else {
                    p -= 1;
                    break;
                }
            }
            5 => {
                found_point = true;
                if is_digit(c) {
                    state = 4;
                    p -= 1;
                } else {
                    p = saved;
                    break;
                }
            }
            6 => {
                saved = p - 2;
                if nonzero_digit(c) {
                    state = 9;
                    p -= 1;
                } else if c == b'+' {
                    state = 7;
                } else if c == b'-' {
                    exp_neg = true;
                    state = 7;
                } else if c == b'0' {
                    state = 8;
                } else {
                    p = saved;
                    break;
                }
            }
            7 => {
                if nonzero_digit(c) {
                    state = 9;
                    p -= 1;
                } else if c == b'0' {
                    state = 8;
                } else {
                    p = saved;
                    break;
                }
            }
            8 => {
                found_exp = true;
                while c == b'0' {
                    c = at(p);
                    p += 1;
                }
                if nonzero_digit(c) {
                    state = 9;
                    p -= 1;
                } else {
                    p -= 1;
                    break;
                }
            }
            9 => {
                found_exp = true;
                let mut e: i32 = 0;
                while is_digit(c) {
                    e = e * 10 + i32::from(c - b'0');
                    if e > 5200 {
                        e = 5201;
                        break;
                    }
                    c = at(p);
                    p += 1;
                }
                exp = e;
                while is_digit(c) {
                    c = at(p);
                    p += 1;
                }
                p -= 1;
                break;
            }
            _ => {
                // State 11, a sign after the mantissa: an exponent only with `implicit_e`.
                if !implicit_e {
                    p -= 1;
                    break;
                }
                saved = p - 1;
                if c == b'+' {
                    state = 7;
                } else if c == b'-' {
                    exp_neg = true;
                    state = 7;
                } else {
                    p = saved;
                    break;
                }
            }
        }
    }
    let end = p;
    if !found_digit {
        let mut ld = [0; 6];
        ld[5] |= sign;
        return Scan { ld, end, flags: 4 };
    }
    if manlen > 24 {
        // The 24th digit is raised when it is 5 or more; the 25th is dropped.
        if digits[23] >= 5 {
            digits[23] += 1;
        }
        manlen = 24;
        manexp += 1;
    }
    let mut flags = 0;
    let mut ld: Ld12 = [0; 6];
    if manlen > 0 {
        while digits[manlen - 1] == 0 {
            manlen -= 1;
            manexp += 1;
        }
        ld = mtold12(&digits[..manlen]);
        let mut e = if exp_neg { -exp } else { exp } + manexp;
        if !found_exp {
            e += scale;
        }
        if !found_point {
            e -= decpt;
        }
        if e > 5200 {
            ld = [0, 0, 0, 0, 0x8000, 0x7fff];
            flags = 2;
        } else if e < -5200 {
            ld = [0; 6];
            flags = 1;
        } else {
            multtenpow12(&mut ld, e, mult12, t);
        }
    }
    ld[5] |= sign;
    Scan { ld, end, flags }
}

/// `___mtold12` (`0x411129`): the digits as a 96-bit integer, `x = x * 10 + d` by `x << 2 + x`
/// then `<< 1`, normalized by 16 and then by 1 until bit 79 is set, the exponent from 0x404e less
/// the shifts. The digits are not all zero.
pub fn mtold12(digits: &[u8]) -> Ld12 {
    let mut x: Ld12 = [0; 6];
    for &d in digits {
        let copy = x;
        shl12(&mut x);
        shl12(&mut x);
        add12(&mut x, &copy);
        shl12(&mut x);
        add12(&mut x, &[u16::from(d), 0, 0, 0, 0, 0]);
    }
    let mut e: u16 = 0x404e;
    while x[4] == 0 && x[5] == 0 {
        x = [0, x[0], x[1], x[2], x[3], x[4]];
        e = e.wrapping_sub(16);
    }
    while x[4] & 0x8000 == 0 {
        shl12(&mut x);
        e = e.wrapping_sub(1);
    }
    x[5] = e;
    x
}

/// A format `__ld12cvt` rounds to: the double's at `0x831928`, the float's at `0x831940`.
#[derive(Clone, Copy)]
pub struct Format {
    pub max_exp: i32,
    pub min_exp: i32,
    pub precision: u32,
    pub exp_width: u32,
    pub width: u32,
    pub bias: i32,
}

pub const DOUBLE: Format = Format {
    max_exp: 1024,
    min_exp: -1023,
    precision: 53,
    exp_width: 11,
    width: 64,
    bias: 1023,
};

pub const FLOAT: Format = Format {
    max_exp: 128,
    min_exp: -127,
    precision: 24,
    exp_width: 8,
    width: 32,
    bias: 127,
};

/// The mantissa `__ld12cvt` works in: 96 bits, word 0 the highest, bit `n` counted from its top.
type Man = [u32; 3];

/// `__ZeroTail` (`0x40fa65`): whether every bit after bit `n` is clear.
fn zero_tail(m: &Man, n: u32) -> bool {
    let i = (n / 32) as usize;
    let below = !(u32::MAX << (31 - n % 32));
    m[i] & below == 0 && m[i + 1..].iter().all(|&w| w == 0)
}

/// `__IncMan` (`0x40faae`): adds 1 at bit `n`, the carry through the higher words; the carry out
/// of word 0.
fn inc_man(m: &mut Man, n: u32) -> bool {
    let mut i = (n / 32) as usize;
    let (v, mut carry) = m[i].overflowing_add(1 << (31 - n % 32));
    m[i] = v;
    while carry && i > 0 {
        i -= 1;
        (m[i], carry) = m[i].overflowing_add(1);
    }
    carry
}

/// `__RoundMan` (`0x40fb04`): with bit `prec` set and a set bit after bit `prec + 1`, adds 1 at
/// bit `prec - 1`; bit `prec + 1` alone does not round. Clears every bit after bit `prec`; the
/// carry out of the top.
fn round_man(m: &mut Man, prec: u32) -> bool {
    let i = (prec / 32) as usize;
    let mut carry = false;
    if m[i] & (1 << (31 - prec % 32)) != 0 && !zero_tail(m, prec + 1) {
        carry = inc_man(m, prec - 1);
    }
    m[i] &= u32::MAX << (31 - prec % 32);
    for w in &mut m[i + 1..] {
        *w = 0;
    }
    carry
}

/// `__ShrMan` (`0x40fbd2`): the 96 bits shifted right by `n`, zeros in at the top.
fn shr_man(m: &mut Man, n: u32) {
    let v = (u128::from(m[0]) << 64 | u128::from(m[1]) << 32 | u128::from(m[2])) >> n.min(96);
    *m = [(v >> 64) as u32, (v >> 32) as u32, v as u32];
}

/// `__ld12cvt` (`0x40fc5f`): the 12-byte real in `f`'s format, as its bits (a float's in the low
/// 32), and 0, or 1 for an overflow to the infinity, or 2 for a result below the normal range,
/// a subnormal or zero.
pub fn ld12cvt(x: &Ld12, f: &Format) -> (u64, u32) {
    let sign = x[5] & 0x8000 != 0;
    let mut e = i32::from(x[5] & 0x7fff) - 0x3fff;
    let mut m: Man = [
        u32::from(x[3]) | u32::from(x[4]) << 16,
        u32::from(x[1]) | u32::from(x[2]) << 16,
        u32::from(x[0]) << 16,
    ];
    let (biased, ret) = if e == -0x3fff {
        if m.iter().all(|&w| w == 0) {
            (0, 0)
        } else {
            m = [0; 3];
            (0, 2)
        }
    } else {
        let save = m;
        if round_man(&mut m, f.precision) {
            e += 1;
        }
        if e < f.min_exp - f.precision as i32 {
            m = [0; 3];
            (0, 2)
        } else if e <= f.min_exp {
            m = save;
            shr_man(&mut m, (f.min_exp - e) as u32);
            round_man(&mut m, f.precision);
            shr_man(&mut m, f.exp_width + 1);
            (0, 2)
        } else if e >= f.max_exp {
            m = [0x8000_0000, 0, 0];
            shr_man(&mut m, f.exp_width);
            (f.bias + f.max_exp, 1)
        } else {
            m[0] &= 0x7fff_ffff;
            shr_man(&mut m, f.exp_width);
            (f.bias + e, 0)
        }
    };
    let hi = (biased as u32) << (31 - f.exp_width) | if sign { 0x8000_0000 } else { 0 } | m[0];
    let bits = if f.width == 64 {
        u64::from(hi) << 32 | u64::from(m[1])
    } else {
        u64::from(hi)
    };
    (bits, ret)
}

/// `__fltin2`'s `FLT` (`0x747560`): its flags (`0x80` overflow, `0x100` underflow, `0x200` no
/// digit), the bytes read and the double.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flt {
    pub flags: u32,
    pub nbytes: usize,
    pub dval: u64,
}

/// `__fltin2` (`0x747560`): `___strgtold12` with no scale, then `_ld12tod` (`0x40fdcb`).
pub fn fltin2(s: &[u8], t: &TenPowers) -> Flt {
    let scan = strgtold12(s, false, 0, 0, false, t);
    let (flags, dval) = if scan.flags & 4 != 0 {
        (0x200, 0)
    } else {
        let (bits, ret) = ld12cvt(&scan.ld, &DOUBLE);
        let mut flags = 0;
        if scan.flags & 2 != 0 || ret == 1 {
            flags = 0x80;
        }
        if scan.flags & 1 != 0 || ret == 2 {
            flags |= 0x100;
        }
        (flags, bits)
    };
    Flt {
        flags,
        nbytes: scan.end,
        dval,
    }
}

/// `strtod`'s answer: the double's bits, the index `*endptr` is set to, and whether it set
/// `errno` to `ERANGE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strtod {
    pub bits: u64,
    pub end: usize,
    pub erange: bool,
}

/// `strtod` (`0x741e39`): C-locale blanks skipped, then `__fltin2`. No digit is `+0` with the end
/// at the start; an overflow is the infinity of the sign the first byte after the blanks gives;
/// an underflow, a subnormal result among them, is `+0`; both set `ERANGE`.
pub fn strtod(s: &[u8], t: &TenPowers) -> Strtod {
    let blank = |c: u8| c == b' ' || (0x09..=0x0d).contains(&c);
    let ws = s.iter().take_while(|&&c| blank(c)).count();
    let rest = &s[ws..];
    let flt = fltin2(rest, t);
    let end = ws + flt.nbytes;
    if flt.flags & 0x240 != 0 {
        return Strtod {
            bits: 0,
            end: 0,
            erange: false,
        };
    }
    if flt.flags & 0x81 != 0 {
        let neg = rest.first() == Some(&b'-');
        let inf = 0x7ff0_0000_0000_0000u64;
        return Strtod {
            bits: if neg { inf | 1 << 63 } else { inf },
            end,
            erange: true,
        };
    }
    if flt.flags & 0x100 != 0 {
        return Strtod {
            bits: 0,
            end,
            erange: true,
        };
    }
    Strtod {
        bits: flt.dval,
        end,
        erange: false,
    }
}
