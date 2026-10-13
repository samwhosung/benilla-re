//! `NormalizeAngle` (`0x7abbe0`, `systems/camera.md#angle-properties`): an angle less its whole
//! turns of 2π, the turns truncated toward zero, and one turn fewer for a negative angle.
//!
//! `__stdcall`, one `f32` argument, the result in `st0`, `ret 4`:
//!
//! ```text
//!   fld    [0xcf5730]     ; 1/2π
//!   fmul   [ebp+8]        ; x/2π
//!   call   0x40a2b0       ; __ftol, toward zero: n
//!   fild   [ebp-4]        ; n
//!   fmul   [0xcf5734]     ; m = n·2π
//!   fld    [ebp+8]
//!   fcomp  [0x7ffd74]     ; x against 0.0
//!   fnstsw ax
//!   test   ah,5           ; C0 and C2
//!   jp     .skip          ; even parity: x >= 0, x == 0 or NaN
//!   fsub   [0xcf5734]     ; x < 0: m − 2π
//! .skip:
//!   fsubr  [ebp+8]        ; x − m
//! ```
//!
//! `jp` falls through only for `x < 0` (C0 alone, odd parity); `x > 0`, `x == 0` (C3) and NaN (all
//! three) take it. `__ftol` (`0x40a2b0`) sets truncation, `fistp`s a 64-bit integer and returns the
//! low dword; NaN, the infinities and anything past 32 bits give `0x80000000`. The globals are set
//! by the static initializers `0x7ac5b0` (`0xcf5734` = π + π, `0x40c90fdb`) and `0x7ac5d0`
//! (`0xcf5730` = 1/2π, `0x3e22f983`). The chain is `f64`; the caller's store rounds to `f32`.

#![allow(clippy::excessive_precision)]

/// `[0xcf5730]`: 1/2π as `f32`.
const RECIP_TWO_PI: f32 = f32::from_bits(0x3e22_f983);
/// `[0xcf5734]`: 2π as `f32`.
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

/// `NormalizeAngle` (`0x7abbe0`): `n = trunc(x·1/2π)`, `m = n·2π`; `x − m` for `x >= 0` and NaN,
/// `x − (m − 2π)` for `x < 0`.
#[must_use]
pub fn normalize_2pi(x: f32) -> f32 {
    let recip = f64::from(RECIP_TWO_PI);
    let period = f64::from(TWO_PI);

    // fld [0xcf5730]; fmul [ebp+8]
    let q = recip * f64::from(x);
    // call __ftol
    let n = ftol(q);
    // fild; fmul [0xcf5734]
    let m = f64::from(n) * period;

    // fcomp 0.0; test ah,5; jp: only x < 0 falls through to the fsub; `<` is false for NaN.
    let r = if x < 0.0 { m - period } else { m };

    // fsubr [ebp+8]
    (f64::from(x) - r) as f32
}

/// `__ftol` (`0x40a2b0`): truncation toward zero, the low dword; `0x80000000` outside `i32`.
#[inline]
fn ftol(q: f64) -> i32 {
    // [-2^31, 2^31) truncates; NaN, the infinities and the rest give the indefinite integer.
    if (-2_147_483_648.0..2_147_483_648.0).contains(&q) {
        q as i32
    } else {
        i32::MIN
    }
}
