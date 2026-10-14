//! The glue screen's aspect and the test that decides whether a resize changes it
//! (`systems/glue/manager.md#screen-size-and-aspect`).

/// The threshold 0.01, the float `0x3c23d70a` at `0x8029d0`.
pub const ASPECT_EPS: f32 = f32::from_bits(0x3c23_d70a);

/// `GetScreenAspect` (`0x46ab40`), the division at `0x46ab97` to `0x46ab9e`: `fild width; fidiv
/// height` at 53 bits, rounded to `f32` when the caller stores it. The width and height come from
/// the `gxResolution` string; without `widescreen` the aspect is 4/3 and this division is not run.
pub fn glue_aspect(width: i32, height: i32) -> f32 {
    (f64::from(width) / f64::from(height)) as f32
}

/// The aspect test of `CheckGlueScreenSize` (`0x46abe0`, `0x46ac5b` to `0x46ac86`): `Some(new)` when
/// `|old - new| >= 0.01`, or when the difference is NaN, which is when the client stores `new` at
/// `0xb41e24` and requests a rebuild (`0xb41d8c`); `None` otherwise. The difference is at 53 bits.
pub fn glue_onresize_changed(old_aspect: f32, new_aspect: f32) -> Option<f32> {
    let d = (f64::from(old_aspect) - f64::from(new_aspect)).abs();
    if d < f64::from(ASPECT_EPS) {
        None
    } else {
        Some(new_aspect)
    }
}
