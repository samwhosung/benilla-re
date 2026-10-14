//! `BuildFrustumCorners` (`0x5c43b0`, `systems/camera.md#unprojection`).

use benilla_re_cmath::matrix4::{det4, mat4_inverse_from_det};
use benilla_re_cmath::multiply::multiply_matrix4;
use benilla_re_cmath::transform::transform_vec4;

/// `0x5c43b0` (`ecx` = the view matrix, `edx` = the projection, the output of eight vectors of three
/// on the stack): the eight world-space corners of the frustum, near corners then far corners, each
/// four in the order `(−,−)`, `(−,+)`, `(+,+)`, `(+,−)` of x and y.
///
/// `inv(VP) = inv(proj) · inv(view)`, each inverse `InvertMatrix4WithDeterminant` (`0x7bd6c0`) of
/// the matrix with its `Determinant4` (`0x7bcf90`) stored as `f32`, and the product by
/// `MultiplyMatrix4`. A projection with `|m15 − 1| < 2^-22` (the `f32` at `0x8029d4`) is an
/// orthographic one: the corners are the cube `±1` with `w = 1`, near at `z = −1`, far at `z = 1`.
/// Otherwise `n = −m14/(m10 + 1)` and `f = −m14/(m10 − 1)` (53 bits, stored as `f32`) give the clip
/// corners `(±n, ±n, −n, n)` for the near plane and `(∓f, ∓f, f, f)` for the far one, so `x/w` and
/// `y/w` are `±1`. Each corner goes through `inv(VP)` by `TransformVector4` (`0x7bcb40`) and its
/// first three components are kept.
#[must_use]
pub fn frustum_corners(view: &[f32; 16], proj: &[f32; 16]) -> [[f32; 3]; 8] {
    let inverse = |m: &[f32; 16]| mat4_inverse_from_det(m, det4(m));
    let inv_view = inverse(view);
    let inv_proj = inverse(proj);
    let inv_vp = multiply_matrix4(&inv_proj, &inv_view);

    const EPSILON: f32 = f32::from_bits(0x3480_0000);
    let diff = (f64::from(proj[15]) - 1.0).abs();

    let clip: [[f32; 4]; 8] = if diff < f64::from(EPSILON) {
        [
            [-1.0, -1.0, -1.0, 1.0],
            [-1.0, 1.0, -1.0, 1.0],
            [1.0, 1.0, -1.0, 1.0],
            [1.0, -1.0, -1.0, 1.0],
            [-1.0, -1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [1.0, -1.0, 1.0, 1.0],
        ]
    } else {
        let m10 = f64::from(proj[10]);
        let m14 = f64::from(proj[14]);
        let n = (-(m14 / (m10 + 1.0))) as f32;
        let f = (-(m14 / (m10 - 1.0))) as f32;
        let nn = -n;
        let ff = -f;
        [
            [nn, nn, nn, n],
            [nn, n, nn, n],
            [n, n, nn, n],
            [n, nn, nn, n],
            [ff, ff, f, f],
            [ff, f, f, f],
            [f, f, f, f],
            [f, ff, f, f],
        ]
    };

    clip.map(|c| {
        let w = transform_vec4(&c, &inv_vp);
        [w[0], w[1], w[2]]
    })
}
