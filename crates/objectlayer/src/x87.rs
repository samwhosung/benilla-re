//! The x87 conversions the object layer stores through.

pub use benilla_re_cmath::x87::{fistp, fistp64, ftol, ftol64};

/// `fcomp; fnstsw ax; test ah,0x41; jne`: true when `a <= b` or the operands are unordered (the C0,
/// C2 or C3 flag is set), false only when `a > b`.
#[inline]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn not_above(a: f64, b: f64) -> bool {
    !(a > b)
}

/// The limit of the x87 `fsin`, `fcos` and `fsincos` argument, `2^63`: beyond it the instruction
/// sets C2 and leaves the operand as it is.
const TRIG_RANGE: f64 = 9_223_372_036_854_775_808.0;

/// `fsin`: the host's sine within range, the operand itself for a finite magnitude above `2^63`,
/// and NaN for an infinity. The instruction differs by CPU in its last bits, so the host's is the
/// stand-in the tests compare with; the emulator leaves an infinity unchanged, which the tests
/// leave out.
#[inline]
pub fn fsin(x: f64) -> f64 {
    if x.is_infinite() {
        f64::NAN
    } else if x.abs() > TRIG_RANGE {
        x
    } else {
        x.sin()
    }
}

/// `fcos`; see [`fsin`].
#[inline]
pub fn fcos(x: f64) -> f64 {
    if x.is_infinite() {
        f64::NAN
    } else if x.abs() > TRIG_RANGE {
        x
    } else {
        x.cos()
    }
}

/// The status byte `fnstsw ax` leaves in `ah` after `fcom`, `fcomp` or `fcompp` of `a` (`st0`)
/// against `b`: C0 is bit 0 (`0x01`), C2 bit 2 (`0x04`) and C3 bit 6 (`0x40`). `a > b` gives 0, `a
/// < b` C0, `a == b` C3, and unordered C0, C2 and C3.
#[inline]
pub fn fcom_ah(a: f64, b: f64) -> u8 {
    if a.is_nan() || b.is_nan() {
        0x01 | 0x04 | 0x40
    } else if a < b {
        0x01
    } else if a == b {
        0x40
    } else {
        0x00
    }
}
