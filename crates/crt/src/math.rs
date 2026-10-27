//! The C runtime's `frexp`, `fmod` and the non-finite cases of `pow` (`systems/crt/math.md`):
//! `_frexp` (`0x7fd8db`), `__CIfmod` (`0x73f90a`) through the dispatcher `0x7436c0`, and
//! `__CIpow` (`0x73f940`) with `__powhlp` (`0x744c8c`).
//!
//! Every answer is a double's bits and whether the runtime set `errno` to `EDOM` (33) on the way.
//! No `fmod` or `pow` operand is a signalling NaN: a double never puts one on the x87 stack, since
//! the load quiets it. Exceptions are masked, so a domain error returns the indefinite, `0xfff8000000000000`.

/// The indefinite `0x82f4d8` holds, the answer to every domain error here.
pub const INDEFINITE: u64 = 0xfff8_0000_0000_0000;
const INF: u64 = 0x7ff0_0000_0000_0000;
const SIGN: u64 = 1 << 63;

/// What a routine answers: the double's bits and whether `errno` was set to `EDOM`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Answer {
    pub bits: u64,
    pub edom: bool,
}

fn is_nan(b: u64) -> bool {
    b & !SIGN > INF
}

fn is_inf(b: u64) -> bool {
    b & !SIGN == INF
}

fn is_zero(b: u64) -> bool {
    b & !SIGN == 0
}

/// `faddp st(1), st` over two quiet NaNs, or one: a lone NaN is the answer; of two the larger
/// significand is, and of equal significands the positive one. `first` is the register below the
/// top, the destination.
fn nan_add(first: u64, second: u64) -> u64 {
    match (is_nan(first), is_nan(second)) {
        (true, false) => first,
        (false, true) => second,
        _ => {
            let (a, b) = (first & !(0xfff << 52), second & !(0xfff << 52));
            if a < b || (a == b && second >> 63 < first >> 63) {
                second
            } else {
                first
            }
        }
    }
}

/// `_frexp` (`0x7fd8db`): the mantissa in [0.5, 1) with the sign of `x` and the exponent `e` with
/// `x = m * 2^e`. A zero of either sign answers `+0` and exponent 0, losing the sign. An infinity
/// answers the indefinite, a quiet NaN itself and a signalling one quieted, all with exponent -1
/// and `errno` `EDOM`.
pub fn frexp(x: u64) -> (Answer, i32) {
    if x >> 52 & 0x7ff == 0x7ff {
        // A signalling NaN is quieted by the `x + 1.0` the exe computes.
        let bits = if is_inf(x) { INDEFINITE } else { x | 1 << 51 };
        return (Answer { bits, edom: true }, -1);
    }
    let edom = false;
    if is_zero(x) {
        return (Answer { bits: 0, edom }, 0);
    }
    let sign = x & SIGN;
    let (frac, e) = if x >> 52 & 0x7ff == 0 {
        // `__decomp` (`0x40c6e6`) shifts a subnormal left until bit 52 is set, one exponent step
        // each from -1021, and clears that bit.
        let mut v = x & !SIGN;
        let mut e = -1021;
        while v & (1 << 52) == 0 {
            v <<= 1;
            e -= 1;
        }
        (v & !(1 << 52), e)
    } else {
        (x & ((1 << 52) - 1), (x >> 52 & 0x7ff) as i32 - 0x3fe)
    };
    // `__set_exp` (`0x40c5ff`) writes the biased exponent 0x3fe over the field.
    let bits = sign | 0x3fe << 52 | frac;
    (Answer { bits, edom }, e)
}

/// `__CIfmod` (`0x73f90a`) of `a` by `b`, the dividend `a` the register below the top: the
/// dispatcher `0x7436c0` classes each operand as normal (a double subnormal included, as the
/// x87 widens it), zero, NaN or infinite, and jumps through the table `0x875eb0`.
///
/// A normal over a normal is `fprem`'s remainder, the sign of the dividend. A zero over a normal
/// is `+0` (`0x743a34`). A normal, zero or infinite dividend over a zero or an infinity, and a
/// zero or infinite dividend over a normal or a zero or an infinity, is the indefinite
/// (`0x743ad6`) with `EDOM`. A NaN gives itself with `EDOM`, of two the larger significand.
pub fn fmod(a: u64, b: u64) -> Answer {
    let domain = Answer {
        bits: INDEFINITE,
        edom: true,
    };
    if is_nan(a) && is_nan(b) {
        return Answer {
            bits: nan_add(b, a),
            edom: true,
        };
    }
    if is_nan(a) {
        return Answer {
            bits: nan_add(b, a),
            edom: true,
        };
    }
    if is_nan(b) {
        return Answer {
            bits: nan_add(a, b),
            edom: true,
        };
    }
    if is_zero(b) || is_inf(b) || is_inf(a) {
        return domain;
    }
    if is_zero(a) {
        return Answer {
            bits: 0,
            edom: false,
        };
    }
    Answer {
        bits: (f64::from_bits(a) % f64::from_bits(b)).to_bits(),
        edom: false,
    }
}

/// `__d_inttype` (`0x744dbc`): 0 for a subnormal or a value that is not a whole number, 1 for an
/// odd whole number, 2 for an even one. An infinity is even; a NaN compares equal to itself in
/// the exe's test and is even too.
pub fn d_inttype(y: u64) -> u32 {
    let f = f64::from_bits(y);
    if y >> 52 & 0x7ff == 0 && !is_zero(y) {
        return 0;
    }
    if is_nan(y) || is_inf(y) {
        return 2;
    }
    if f != f.round_ties_even() {
        return 0;
    }
    if f / 2.0 == (f / 2.0).round_ties_even() {
        2
    } else {
        1
    }
}

/// `__powhlp` (`0x744c8c`) for a base `x` and exponent `y` one of which is infinite; `None` where
/// the helper stores nothing (neither is infinite, or a base NaN with a zero low word).
/// `Some((bits, error))` is the stored result and whether the helper answered 1.
pub fn powhlp(x: u64, y: u64) -> Option<(u64, bool)> {
    let ax = f64::from_bits(x & !SIGN);
    let one = 1.0f64;
    let inf = INF;
    let ind = (INDEFINITE, true);
    if y == inf {
        // |x| above 1, then below 1 (or unordered); exactly 1 is the indefinite.
        return Some(if ax > one {
            (inf, false)
        } else if ax < one || ax.is_nan() {
            (0, false)
        } else {
            ind
        });
    }
    if y == inf | SIGN {
        return Some(if ax > one {
            (0, false)
        } else if ax >= one {
            ind
        } else {
            (inf, false)
        });
    }
    if x == inf {
        let yf = f64::from_bits(y);
        return Some(if yf > 0.0 {
            (inf, false)
        } else if yf >= 0.0 {
            (1.0f64.to_bits(), false)
        } else {
            (0, false)
        });
    }
    if x == inf | SIGN {
        let t = d_inttype(y);
        let yf = f64::from_bits(y);
        return Some(if yf > 0.0 {
            (if t == 1 { inf | SIGN } else { inf }, false)
        } else if yf >= 0.0 {
            (1.0f64.to_bits(), false)
        } else if t == 1 {
            (SIGN, false)
        } else {
            (0, false)
        });
    }
    None
}

/// `__CIpow` (`0x73f940`) of `x` to the power `y` where a NaN or an infinity is an operand;
/// `None` for two finite operands, which the entry does not cover. A NaN operand is the answer
/// (`faddp`, `0x73fa0c`) with `EDOM`; otherwise `__powhlp` answers, and a 1 from it sets `EDOM`.
pub fn pow_nonfinite(x: u64, y: u64) -> Option<Answer> {
    if is_nan(x) || is_nan(y) {
        return Some(Answer {
            bits: nan_add(y, x),
            edom: true,
        });
    }
    let (bits, error) = powhlp(x, y)?;
    Some(Answer { bits, edom: error })
}
