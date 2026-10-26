//! The C runtime's 12-byte real (`systems/crt/decimal.md`): six 16-bit words, low first, that hold
//! a 16-bit extension, a 64-bit mantissa with its leading 1 and the sign over a 15-bit exponent
//! biased by 0x3fff. The runtime's decimal conversions scale a value by powers of ten in it.

/// A 12-byte real: `[extension, mantissa low .. mantissa high, sign | exponent]`.
pub type Ld12 = [u16; 6];

/// The two tables of powers of ten `___multtenpow12` (`0x413111`) reads: 29 rows each, for
/// 10^1 to 10^7, 10^8 to 10^56 by 8, 10^64 to 10^448 by 64, 10^512 to 10^3584 by 512 and 10^4096,
/// and the same negative powers. The rows are read from the exe at `0x832490` and `0x8325f0`.
pub struct TenPowers {
    pub pos: [Ld12; 29],
    pub neg: [Ld12; 29],
}

/// The low 96 bits of the words, word 0 lowest.
pub fn to_u96(x: &Ld12) -> u128 {
    x.iter().rev().fold(0, |a, &w| a << 16 | u128::from(w))
}

pub fn from_u96(v: u128) -> Ld12 {
    std::array::from_fn(|i| (v >> (16 * i)) as u16)
}

const MASK96: u128 = (1 << 96) - 1;

/// `0x411070`: `x += y` over all 96 bits, the carry out of the top dword dropped.
pub fn add12(x: &mut Ld12, y: &Ld12) {
    *x = from_u96((to_u96(x) + to_u96(y)) & MASK96);
}

/// `0x4110ce`: the 96 bits shifted left by one.
pub fn shl12(x: &mut Ld12) {
    *x = from_u96((to_u96(x) << 1) & MASK96);
}

/// `0x4110fc`: the 96 bits shifted right by one, a zero in at the top.
pub fn shr12(x: &mut Ld12) {
    *x = from_u96(to_u96(x) >> 1);
}

/// `___ld12mul` (`0x412ef1`): `x *= y`. The 80-bit mantissas (extension and mantissa) multiply by
/// 16-bit words, keeping only the partial products whose words sum to 4 or more; the 96-bit sum is
/// normalized, shifted down with a sticky bit below exponent 1, rounded to nearest even on its low
/// word, and its top 80 bits kept. An exponent sum above 0xbffd, an operand of exponent 0x7fff or a
/// rounded exponent of 0x7fff or more gives the infinity; a sum of 0x3fbf or less gives +0.
pub fn ld12mul(x: &mut Ld12, y: &Ld12) {
    let sign = (x[5] ^ y[5]) & 0x8000;
    let ex = x[5] & 0x7fff;
    let ey = y[5] & 0x7fff;
    let inf = |x: &mut Ld12| *x = [0, 0, 0, 0, 0x8000, 0x7fff | sign];
    let mut e = ex.wrapping_add(ey);
    if ex >= 0x7fff || ey >= 0x7fff || e > 0xbffd {
        return inf(x);
    }
    if e <= 0x3fbf {
        *x = [0; 6];
        return;
    }
    // A zero operand is one whose words but the sign are zero.
    let zero = |v: &Ld12| v[..5].iter().all(|&w| w == 0) && v[5] & 0x7fff == 0;
    if ex == 0 {
        e = e.wrapping_add(1);
        if zero(x) {
            x[5] = 0;
            return;
        }
    }
    if ey == 0 {
        e = e.wrapping_add(1);
        if zero(y) {
            *x = [0; 6];
            return;
        }
    }
    let mut t: u128 = 0;
    for (p, &xp) in x[..5].iter().enumerate() {
        for (q, &yq) in y[..5].iter().enumerate().skip(4 - p) {
            t += u128::from(u32::from(xp) * u32::from(yq)) << (16 * (p + q - 4));
        }
    }
    e = e.wrapping_add(0xc002);
    if (e as i16) > 0 {
        while (e as i16) > 0 && t >> 95 == 0 {
            t = (t << 1) & MASK96;
            e = e.wrapping_sub(1);
        }
    }
    if (e as i16) <= 0 {
        e = e.wrapping_sub(1);
        if (e as i16) < 0 {
            let n = -(e as i16 as i32);
            e = 0;
            let mut sticky = false;
            for _ in 0..n {
                sticky |= t & 1 != 0;
                t >>= 1;
            }
            if sticky {
                t |= 1;
            }
        }
    }
    let low = t as u16;
    if low > 0x8000 || (t as u32) & 0x1ffff == 0x18000 {
        let m = t >> 16;
        if m == (1 << 80) - 1 {
            t = 0x8000 << 80;
            e = e.wrapping_add(1);
        } else {
            t = (m + 1) << 16 | u128::from(low);
        }
    }
    if e >= 0x7fff {
        return inf(x);
    }
    let w = from_u96(t);
    *x = [w[1], w[2], w[3], w[4], w[5], e | sign];
}

/// `___multtenpow12` (`0x413111`): `x *= 10^pow`, by the octal digits of `pow` from the low one,
/// each a row of its group: a digit `d` of group `g` multiplies by row `7g + d - 1`. A row whose
/// extension is 0x8000 or more is used with the low dword of its mantissa one lower. With `mult12`
/// clear the extension of `x` is cleared first, unless `pow` is 0.
pub fn multtenpow12(x: &mut Ld12, pow: i32, mult12: bool, t: &TenPowers) {
    if pow == 0 {
        return;
    }
    let (table, mut pow) = if pow < 0 {
        (&t.neg, pow.wrapping_neg())
    } else {
        (&t.pos, pow)
    };
    if !mult12 {
        x[0] = 0;
    }
    let mut group = 0;
    while pow != 0 {
        let digit = (pow & 7) as usize;
        pow >>= 3;
        if digit != 0 {
            let mut row = table[group * 7 + digit - 1];
            if row[0] >= 0x8000 {
                let low = (u32::from(row[1]) | u32::from(row[2]) << 16).wrapping_sub(1);
                row[1] = low as u16;
                row[2] = (low >> 16) as u16;
            }
            ld12mul(x, &row);
        }
        group += 1;
    }
}
