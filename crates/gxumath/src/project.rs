//! The screen-to-world map `0x5c4f30`, which reads the live device (`0xc0ed38`) through getters:
//! the viewport rectangle (`0x58a240`), the viewport scalars (`0x58b060`), the combined model-view
//! matrix (`0x58b0c0`) and the inverse projection (`0x58b280`). They are the function's inputs here.

use benilla_re_cmath::multiply::multiply_matrix4;
use benilla_re_cmath::transform::transform_vec4;

/// The z range the map reads from the device (`0x58b060`, device `+0xf48` and `+0xf4c`). The x and
/// y origin and extent come from the rectangle.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnprojectViewport {
    /// Device `+0xf48`, the term the z map subtracts and adds back.
    pub z_near: f32,
    /// Device `+0xf4c`, the scale the z map applies.
    pub z_far: f32,
}

/// The `f32` at `0x801360`, 0.001, against which `|w'|` is compared.
const W_EPSILON: f32 = 0.001;

/// `0x5c4f30` (`ecx` = the point count, `edx` = the points, the output on the stack): the points
/// `(x, y, z)` mapped from the screen to the world, or `None` where the function returns 0.
///
/// `M = proj_inv · combined_m` by `MultiplyMatrix4`. For each point `(x, y, z, 1)` goes through `M`
/// by `TransformVector4` to `(x', y', z', w')`, and the whole call ends with `None` at the first
/// point whose `|w'| < 0.001` (strictly; NaN does not reject). Otherwise, with `iw = 1/w'` and `rect`
/// the viewport rectangle: `out.x = ((x'·iw + 1)·0.5·(rect[3] − rect[1]) + rect[1])`, `out.y =
/// ((y'·iw + 1)·0.5·(rect[2] − rect[0]) + rect[2])` and `out.z = ((z'·iw + 1)·0.5·z_far − z_near +
/// z_near)`, at 53 bits and each stored as `f32`.
#[must_use]
pub fn unproject_viewport(
    points: &[[f32; 3]],
    combined_m: &[f32; 16],
    proj_inv: &[f32; 16],
    vp: &UnprojectViewport,
    rect: &[f32; 4],
) -> Option<Vec<[f32; 3]>> {
    let f = f64::from;
    let m_total = multiply_matrix4(proj_inv, combined_m);
    let w_eps = f(W_EPSILON);
    let z_near = f(vp.z_near);
    let z_far = f(vp.z_far);
    let [r0, r1, r2, r3] = rect.map(f);
    let mut out = Vec::with_capacity(points.len());
    for p in points {
        let clip = transform_vec4(&[p[0], p[1], p[2], 1.0], &m_total);
        let w = f(clip[3]);
        if w.abs() < w_eps {
            return None;
        }
        let invw = 1.0 / w;
        out.push([
            ((f(clip[0]) * invw + 1.0) * 0.5 * (r3 - r1) + r1) as f32,
            ((f(clip[1]) * invw + 1.0) * 0.5 * (r2 - r0) + r2) as f32,
            ((f(clip[2]) * invw + 1.0) * 0.5 * z_far - z_near + z_near) as f32,
        ]);
    }
    Some(out)
}
