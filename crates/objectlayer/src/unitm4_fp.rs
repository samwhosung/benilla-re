//! Float kernels of the unit frame update: the display facing smoother in
//! `CGUnit_C::UpdateDisplayFacing` (`0x600cd0`), the deep-liquid test `CGUnit_C::IsInDeepLiquid`
//! (`0x605650`) and the turn, lean and spin-cast arithmetic of `CGUnit_C::Update` (`0x607ed0`)
//! (`systems/objectlayer/unitmovement.md#facing`, `unitbody.md#liquid-and-swimming`,
//! `#the-per-frame-update` and `#body-turn-and-spine-twist`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`; a value
//! spilled to a `dword` and reloaded is rounded at the spill. `WrapAngleToPi` (`0x6084f0`) is
//! `crate::unit_fp::angle_wrap_pi`, and the slices that call it run its code.

use crate::x87::ftol;

/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, in the spin-cast fraction `1.0 - frac`.
pub const ONE: f32 = 1.0;
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the half reach of `0x605650` and the half step of
/// `0x600cd0`.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_008029b0` (`0x3e800000`): 0.25, the box filter divisor of `0x600cd0` and the lean
/// smoothing weight of `0x607ed0`.
pub const QUARTER: f32 = f32::from_bits(0x3e80_0000);
/// `_DAT_0080295c` (`0x40c90fdb`): 2π as `f32`, the facing wrap modulus of `0x600cd0`.
pub const TAU: f32 = f32::from_bits(0x40c9_0fdb);
/// `_DAT_008030b0` (`0x40490fdb`): π as `f32`, the upper bound of the facing wrap.
pub const PI: f32 = f32::from_bits(0x4049_0fdb);
/// `_DAT_0080c5b8` (`0xc0490fdb`): −π as `f32`, the lower bound of the facing wrap.
pub const NEG_PI: f32 = f32::from_bits(0xc049_0fdb);
/// `_DAT_008029d0` (`0x3c23d70a`): 0.01, the dead band of the display facing.
pub const FACING_EPS: f32 = f32::from_bits(0x3c23_d70a);
/// `_DAT_0080c5cc` (`0x3b03126f`): 0.002, the fraction step of the spin-cast indicator.
pub const SPIN_STEP: f32 = f32::from_bits(0x3b03_126f);
/// `_DAT_0080653c` (`0x43000000`): 128.0, the byte scale of the spin-cast indicator.
pub const SPIN_SCALE: f32 = 128.0;
/// `_DAT_00801360` (`0x3a83126f`): 0.001, the milliseconds-to-seconds factor of the turn step.
pub const MS_TO_S: f32 = f32::from_bits(0x3a83_126f);
/// `_DAT_00807b10` (`0x41000000`): 8.0, the gain of the turn step.
pub const TURN_GAIN: f32 = 8.0;
/// `_DAT_0080c4e4` (`0x3f490fdb`): π/4, the narrow lean angle.
pub const PI_4: f32 = f32::from_bits(0x3f49_0fdb);
/// `_DAT_00808acc` (`0x3fc90fdb`): π/2, the wide lean angle.
pub const PI_2: f32 = f32::from_bits(0x3fc9_0fdb);
/// `_DAT_007ffd74` (`0x00000000`): 0.0, the start of the box filter sum.
pub const ZERO_INIT: f32 = 0.0;

/// The facing delta of `UpdateDisplayFacing` (`0x600cd0`), `0x600ea5` to `0x600eb0`: `fld a; fld
/// st0; fsub prev; fst dword [ebp-4]`, with `a` the goal facing and `prev` `[esi+0xc98]`.
pub fn facing_delta(a: f32, prev: f32) -> f32 {
    (f64::from(a) - f64::from(prev)) as f32
}

/// The dead band of `0x600cd0`, `0x600eb3` to `0x600ec0`: `fabs; fcomp 0.01; test ah,0x41; jp
/// 0x600ed8` falls through on an odd count of C0 and C3, which is `|d| < 0.01` and `|d| == 0.01`,
/// and snaps the facing to `a`; greater and NaN go on to smooth. Returns whether it snaps.
pub fn facing_dead_band(a: f32, prev: f32) -> bool {
    let d = (f64::from(a) - f64::from(prev)).abs();
    !d.is_nan() && d <= f64::from(FACING_EPS)
}

/// The wrap of the delta in `0x600cd0`, `0x600eda` to `0x600f0b`, one `±2π` and not `fmod`. `fcomp
/// π; test ah,0x41; jne 0x600ef5` falls through only for `Δ > π`, which stores `Δ - 2π`. At
/// `0x600ef5` `fcomp -π; test ah,5; jp 0x600f0e` falls through only for `Δ < -π`, which stores `Δ +
/// 2π`. Equal and NaN keep `Δ`.
pub fn facing_wrap_pi_delta(delta: f32) -> f32 {
    let d = f64::from(delta);
    if !d.is_nan() && d > f64::from(PI) {
        return (d - f64::from(TAU)) as f32;
    }
    if !d.is_nan() && d < f64::from(NEG_PI) {
        return (d + f64::from(TAU)) as f32;
    }
    delta
}

/// The smoothing of `0x600cd0`, `0x600f7b` to `0x600fee`, from `delta_in`, which is stored as
/// `hist[0]`, the three older samples `h1` to `h3` (`this[+0xca0]` to `this[+0xca8]` after the
/// shift) and `prev` (`this[+0xc98]`). The average is `(((0.0 + delta_in) + h1) + h2) + h3` times
/// 0.25. The select keeps the smaller of `delta_in` and the average for `delta_in > 0` and the
/// larger otherwise, taking the average when the compare is unordered; it is stored to a dword and
/// reloaded. The new facing is `selected·0.5 + prev`, wrapped into `[0, 2π)` by a single add or
/// subtract and stored as a dword.
pub fn facing_box_apply(delta_in: f32, h1: f32, h2: f32, h3: f32, prev: f32) -> f32 {
    let sum = (((f64::from(ZERO_INIT) + f64::from(delta_in)) + f64::from(h1)) + f64::from(h2))
        + f64::from(h3);
    let avg = sum * f64::from(QUARTER);
    let d = f64::from(delta_in);
    // For `delta_in > 0` the `jne` is not taken and `test ah,5; jnp` keeps the delta on an odd
    // count, `delta_in < avg`; otherwise the average is stored. For `delta_in <= 0` and NaN the
    // `jne` is taken and `test ah,0x41; je` keeps the delta only for `delta_in > avg`.
    let selected = if !d.is_nan() && d > 0.0 {
        if !avg.is_nan() && d < avg {
            d
        } else {
            avg
        }
    } else if !d.is_nan() && !avg.is_nan() && d > avg {
        d
    } else {
        avg
    };
    let selected_f32 = f64::from(selected as f32);
    let new = selected_f32 * f64::from(HALF) + f64::from(prev);
    // `fcom 0.0; test ah,5; jp 0x600ffa` falls through for `new < 0`, which adds 2π; at 0x600ffa
    // `fcom 2π; test ah,1; jne 0x60100d` skips a value below 2π and NaN, and otherwise subtracts 2π.
    if !new.is_nan() && new < 0.0 {
        (new + f64::from(TAU)) as f32
    } else if !new.is_nan() && new >= f64::from(TAU) {
        (new - f64::from(TAU)) as f32
    } else {
        new as f32
    }
}

/// The airborne test of `CGUnit_C::IsInDeepLiquid` (`0x605650`), `0x6056b7` to `0x6056d8`: `height
/// = self_z - hit_z` (`fld [ebp-4]; fsub [eax+8]`; with no ground hit the height is the zero in
/// `[ebp-0xc]`, which `self_z == hit_z` models), `fcompp` of it with `reach·0.5`, and `test
/// ah,0x41; jne` returns 0 unless `height > reach·0.5`; equal, below and NaN return 0.
pub fn airborne_reach_gate(self_z: f32, hit_z: f32, reach: f32) -> bool {
    let height = f64::from(self_z) - f64::from(hit_z);
    let half_reach = f64::from(reach) * f64::from(HALF);
    !height.is_nan() && !half_reach.is_nan() && height > half_reach
}

/// The spin-cast indicator of `CGUnit_C::Update` (`0x607ed0`), `0x607f9b` to `0x607fb9`: `fild
/// [ebp-0xc]; fmul 0.002`, then `fsubr 1.0` when `[0xc4daa4]` is non-zero, `fmul 128.0` and
/// `__ftol`; `invert` is that flag. Returns `eax`, of which the code keeps `al`.
pub fn spin_cast_byte(n: i32, invert: bool) -> i32 {
    let mut x = f64::from(n) * f64::from(SPIN_STEP);
    if invert {
        x = f64::from(ONE) - x;
    }
    let frac = x * f64::from(SPIN_SCALE);
    ftol(frac)
}

/// The turn step of `0x607ed0`, `0x6081dd` to `0x608204`: `fild qword [ebp-0x18]` is the
/// millisecond delta, and `· 0.001 · turn_rate · 8.0` in that order is the step, with `turn_rate`
/// `this[+0xa44]`. `fcom |Δ|; test ah,0x41; jne` keeps the step for less, equal and NaN and takes
/// `abs_delta` (`[ebp-4]`) when the step is greater. Returns the dword stored at `[ebp-0xc]`.
pub fn turn_step_cap(tick_delta: i64, turn_rate: f32, abs_delta: f32) -> f32 {
    let step =
        ((tick_delta as f64) * f64::from(MS_TO_S) * f64::from(turn_rate)) * f64::from(TURN_GAIN);
    let ad = f64::from(abs_delta);
    let keep_step = step.is_nan() || ad.is_nan() || step <= ad;
    if keep_step {
        step as f32
    } else {
        ad as f32
    }
}

/// The lean smoothing of `0x607ed0`, `0x60809a` to `0x6080f0`, from `flags = this[+0x9e8]`, `facing
/// = this[+0xc98]` and `displayed = this[+0xc94]`. The angle is π/4 when `flags & 3` is non-zero
/// and π/2 otherwise, and `fchs` negates it when bit 1 of `flags` equals bit 2. The result is
/// `wrap(wrap( (facing - displayed) + angle)·0.25 + displayed)` with `wrap` = `0x6084f0`. Returns
/// the new `this[+0xc94]`.
pub fn lean_smooth(flags: u32, facing: f32, displayed: f32) -> f32 {
    let mut thr = if flags & 3 != 0 { PI_4 } else { PI_2 };
    let bit1 = flags & 2;
    let bit2 = (flags >> 1) & 2;
    if bit1 == bit2 {
        thr = -thr;
    }
    let pre = (f64::from(facing) - f64::from(displayed)) + f64::from(thr);
    let w1 = f64::from(crate::unit_fp::angle_wrap_pi(pre as f32));
    let mid = (w1 * f64::from(QUARTER) + f64::from(displayed)) as f32;
    crate::unit_fp::angle_wrap_pi(mid)
}

/// The turn delta of `0x607ed0`, `0x608119` to `0x60813a`: `wrap(facing - displayed)` with `fst
/// dword [ebp-8]` rounding it, and `fabs; fstp dword [ebp-4]` its magnitude. Returns `(signed,
/// abs)`.
pub fn turn_delta(facing: f32, displayed: f32) -> (f32, f32) {
    let pre = (f64::from(facing) - f64::from(displayed)) as f32;
    let signed = crate::unit_fp::angle_wrap_pi(pre);
    let abs = f64::from(signed).abs() as f32;
    (signed, abs)
}

/// The turn applied in `0x607ed0`, `0x608227` to `0x608239`: `wrap([ebp+8] + displayed)` with the
/// `0x6084f0` wrap, stored to `this[+0xc94]`.
pub fn turn_apply_step(accum: f32, displayed: f32) -> f32 {
    let pre = (f64::from(accum) + f64::from(displayed)) as f32;
    crate::unit_fp::angle_wrap_pi(pre)
}
