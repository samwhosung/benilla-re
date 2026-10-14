//! `CosineInterpolate` (`0x5b7bb0`, `systems/net/layout.md#shared-helpers`).

/// `0x5b7bb0` (three `f32` on the stack, `ret 0xc`; the result left in `st0`): `a + (b − a)·((1 −
/// cos(π·t))·0.5)` at 53 bits, `π` the `f32` `0x40490fdb` widened and `cos` the `fcos` the host's
/// cosine stands in for (`METHOD.md`). The result is the register, not narrowed.
#[must_use]
pub fn cosine_smoothstep(a: f32, b: f32, t: f32) -> f64 {
    let pi = f64::from(f32::from_bits(0x4049_0fdb));
    let weight = (1.0 - (pi * f64::from(t)).cos()) * 0.5;
    (f64::from(b) - f64::from(a)) * weight + f64::from(a)
}
