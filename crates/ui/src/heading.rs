//! `HeadingBetweenPoints` (`0x47f220`, `systems/objectlayer/movement.md#heading-between-two-points`).
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and `fpatan` is the host's `atan2`.

/// π, the `f32` at `0x804214` and `0x81d0d0`.
const PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
/// 1.5, the `f32` at `0x80308c`.
const C_1_5: f64 = f32::from_bits(0x3fc0_0000) as f64;
/// 2^-22, the `f32` at `0x8029d4`.
const EPS: f64 = f32::from_bits(0x3480_0000) as f64;

/// `HeadingBetweenPoints` (`0x47f220`), `__fastcall(p1, p2)` with two `[f32; 2]` pointers: the
/// angle of `p2 - p1`, returned in `st0` and consumed unrounded, so as an `f64`. On the axis-aligned
/// guard paths the angle is in `[0, 2π)` and is not the bare `fpatan`.
///
/// `dx = (p2.x - p1.x)` is stored as `f32` (`fstp [ebp-0xc]`); `dy = p2.y - p1.y` stays at register
/// precision. `fcomp; fnstsw; test ah, 5; jp` falls through only when the first operand is strictly
/// below the second, so:
///
/// - `|dx| < eps` (vertical): `1.5 * π` when `dy < 0`, else `0.5 * π` (including a NaN `dy`);
/// - else `|dy| < eps` (horizontal): `π` when the raw `p2.x < p1.x`, else 0, the test reading the
///   `f32` coordinates and not `dx`;
/// - else, and for a NaN `dy`, `atan2(dy, dx)` with the stored `f32` `dx`.
///
/// A NaN `dx` fails `|dx| < eps` and takes the `atan2`.
pub fn heading_between_points(p1: [f32; 2], p2: [f32; 2]) -> f64 {
    let dx = (f64::from(p2[0]) - f64::from(p1[0])) as f32;
    let dy = f64::from(p2[1]) - f64::from(p1[1]);
    let dxf = f64::from(dx);
    if dxf.abs() < EPS {
        if dy < 0.0 {
            C_1_5 * PI
        } else {
            0.5 * PI
        }
    } else if dy.abs() >= EPS || dy.is_nan() {
        dy.atan2(dxf)
    } else if f64::from(p2[0]) < f64::from(p1[0]) {
        PI
    } else {
        0.0
    }
}
