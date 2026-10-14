//! Arithmetic of the base frame, slider and mail scripts
//! (`systems/ui/visibility.md#alpha`, `systems/ui/slider.md#lua-methods`,
//! `systems/ui/mail.md#pending-mail`).

/// `0x7ff9d8`, 1.0.
const ONE: f64 = 1.0;
/// `0x7ffe58`, 255.0 as an `f32`.
const SCALE_255: f64 = 255.0;

/// `Frame::SetAlpha` (`0x774e90`, slice `0x774f32` to `0x774f64`): the Lua number is clamped to
/// `[0, 1]` against the `f32` constants at `0x7ffd74` and `0x7ff9d8` with the value kept live in
/// `st0`, then multiplied by 255.0 and left live for `__ftol`. The clamp is a `max` against 0.0 and a
/// `min` against 1.0; a NaN passes the first (only `x < 0` clamps) and takes the second's bound,
/// so it gives 1.0.
pub fn set_alpha_scaled(x: f64) -> f64 {
    let lo_clamped = if x < 0.0 { 0.0 } else { x };
    let hi_clamped = if lo_clamped <= ONE { lo_clamped } else { ONE };
    hi_clamped * SCALE_255
}

/// The alpha byte `Frame::SetAlpha` stores: the scaled product truncated by `__ftol`. The clamp keeps
/// the product in `[0, 255]`, so the truncation is exact.
pub fn set_alpha_value(x: f64) -> i32 {
    benilla_re_cmath::x87::ftol(set_alpha_scaled(x))
}

/// `Slider::GetMinMaxValues` (`0x7906e0`, `0x790778`): the maximum is rebuilt as the stored range
/// (`+0x31c`) plus the stored minimum (`+0x318`) with `fadd`, both `f32` widened to 53 bits, and
/// pushed as an f64.
pub fn max_value(min: f32, range: f32) -> f64 {
    f64::from(range) + f64::from(min)
}

/// The x87 condition bits `(C3, C2, C0)` of `fld [x]; fabs; fcomp [thr]` in `HasNewMail` (`0x4afea0`):
/// C3 is set for equal, C2 for unordered, C0 for less; an unordered compare sets all three.
pub fn fabs_cmp(x: f32, thr: f32) -> (bool, bool, bool) {
    let a = f64::from(x).abs();
    let t = f64::from(thr);
    if a.is_nan() || t.is_nan() {
        (true, true, true)
    } else if a < t {
        (false, false, true)
    } else if a > t {
        (false, false, false)
    } else {
        (true, false, false)
    }
}
