//! Float kernels of `LogXpGainLine` (`0x637cd0`) and the chair seat placement `0x47cf60`
//! (`systems/objectlayer/combatlogmessages.md#experience`,
//! `systems/objectlayer/gameobjecttypes.md#chairs`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fstp dword`. A value spilled to a
//! `dword` and reloaded is rounded at the spill. `fild` reads a signed integer, or for a `qword`
//! the 64 bits it is given, and `__ftol` (`0x40a2b0`) truncates toward zero into 64 bits.

use crate::x87::ftol;

/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, subtracted from the seat count.
pub const ONE: f32 = 1.0;
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the factor on the count less one.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_00803fac` (`0x3f000000`): 0.5, the base `0x47cf60` doubles into its step.
pub const STEP_BASE: f32 = f32::from_bits(0x3f00_0000);

/// The XP of `LogXpGainLine` (`0x637cd0`), `0x637cd8` to `0x637ce9`: `fild` of `amount` (`edx`,
/// stored to `[ebp-4]`) divided by the f32 `rate` (`[ebp+8]`) on the stack, then `__ftol`. The
/// quotient is not spilled to a dword, so it reaches `__ftol` at 53 bits; NaN, an infinity or a
/// quotient outside `i64` gives the low dword of the indefinite, 0.
pub fn combatlog_xp_scaled(amount: i32, rate: f32) -> i32 {
    ftol(f64::from(amount) / f64::from(rate))
}

/// The first seat's local y of `0x47cf60`, `0x47cf6f` to `0x47cf94`: `mov [ebp-4],0; mov
/// [ebp-8],edx` makes `[ebp-8]` the count `n` zero-extended to 64 bits, `fild qword` reads it (a
/// high bit is magnitude), `fsub 1.0`, `fmul 0.5`, `fchs`, and `fstp [ebp-0x10]` rounds to `f32`.
pub fn transport_segment_offset0(n: u32) -> f32 {
    let v = (f64::from(n) - f64::from(ONE)) * f64::from(HALF);
    (-v) as f32
}

/// The seat spacing of `0x47cf60`, `0x47cf97` to `0x47cf9f`: `fld base; fadd st,st; fstp [ebp-4]`
/// with `base` the dword at `0x803fac`.
pub fn transport_segment_step(base: f32) -> f32 {
    (f64::from(base) + f64::from(base)) as f32
}

/// The local y of seat `k` in `0x47cf60`, `0x47cfbc` to `0x47cfd1`: each pass does `fld [ebp-0x10];
/// fadd [ebp-4]; fstp [ebp-0x10]`, so the offset is rounded to `f32` at every advance. `k = 0` is
/// `offset0`.
pub fn transport_segment_local_y(offset0: f32, step: f32, k: u32) -> f32 {
    let mut off = offset0;
    for _ in 0..k {
        off = (f64::from(off) + f64::from(step)) as f32;
    }
    off
}
