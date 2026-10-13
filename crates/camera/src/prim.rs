//! The particle density clamp and an angle's cosine and sine.

#![allow(clippy::float_cmp)]
#![allow(clippy::excessive_precision)]

/// `SetParticleDensity` (`0x7adfb0`, `systems/camera.md#particle-density`) clamps to `[0, 1]` and
/// stores to `0x87d5fc`. Two `fcomp; test ah,5; jp`: below 0 gives 0, below 1 gives `x`, and 1 or
/// more, or NaN, gives 1.
#[must_use]
pub fn clamp01(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x < 1.0 {
        x
    } else {
        1.0
    }
}

/// An angle property's cosine and sine, `fld angle; fsincos` in `InitAngleProperty` (`0x7acbc0`)
/// and `StoreAngleProperty` (`0x7acc40`), stored to `+0x20` and `+0x24`
/// (`systems/camera.md#angle-properties`). `fsincos` differs by CPU; the host's `f64` cosine and
/// sine stand in for it.
#[must_use]
pub fn angle_cossin(angle: f32) -> (f32, f32) {
    let a = f64::from(angle);
    (a.cos() as f32, a.sin() as f32)
}
