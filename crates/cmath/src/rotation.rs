//! `BuildAxisRotation` (`0x7be490`), the rotation about an axis the particle quads and the spline
//! emitter build (`systems/models/emission.md#spline-spawn`,
//! `systems/models/particles.md#head-quads`), its 3×4 and 4×4 forms, and
//! `BuildYawPitchRollBasis` (`0x7bf4b0`).

use crate::multiply::multiply_matrix3;

/// `0x7be490` (ecx = out, edx = the axis; the angle and whether the axis is unit; `ret 8`): unless
/// unit, the axis times `1/√((z² + y²) + x²)`, each stored as `f32`; then with `c` and `s` from
/// `fsincos` and `t = 1 − c`, all stored as `f32`, the matrix `c·I + s·[a]× + t·(a⊗a)`. The
/// products `t·ax·ay`, `t·ay·az` and `az·s` stay in the register; `t·ax·az` is used once from the
/// register (element 2) and once stored as `f32` (element 6); `ax·s` and `ay·s` are stored as
/// `f32`.
pub fn axis_angle_mat3(axis: &[f32; 3], angle: f32, normalized: bool) -> [f32; 9] {
    let f = f64::from;
    let (ax, ay, az) = if normalized {
        (axis[0], axis[1], axis[2])
    } else {
        let inv_len = 1.0
            / (f(axis[2]) * f(axis[2]) + f(axis[1]) * f(axis[1]) + f(axis[0]) * f(axis[0])).sqrt();
        (
            (f(axis[0]) * inv_len) as f32,
            (f(axis[1]) * inv_len) as f32,
            (f(axis[2]) * inv_len) as f32,
        )
    };
    let cos = f(angle).cos() as f32;
    let sin = f(angle).sin() as f32;
    let omc = (1.0 - f(cos)) as f32;
    let axay = f(ax) * f(ay);
    let ayaz = f(ay) * f(az);
    let axaz = f(ax) * f(az);
    let omc_axay = f(omc) * axay;
    let omc_ayaz = f(omc) * ayaz;
    let omc_axaz = f(omc) * axaz;
    let omc_axaz_f32 = (f(omc) * axaz) as f32;
    let lsx = (f(ax) * f(sin)) as f32;
    let lsy = (f(ay) * f(sin)) as f32;
    let azsin = f(az) * f(sin);
    [
        (f(ax) * f(ax) * f(omc) + f(cos)) as f32,
        (omc_axay + azsin) as f32,
        (omc_axaz - f(lsy)) as f32,
        (omc_axay - azsin) as f32,
        (f(ay) * f(ay) * f(omc) + f(cos)) as f32,
        (f(lsx) + omc_ayaz) as f32,
        (f(omc_axaz_f32) + f(lsy)) as f32,
        (omc_ayaz - f(lsx)) as f32,
        (f(az) * f(az) * f(omc) + f(cos)) as f32,
    ]
}

/// `0x7bdb00` (`ecx` = out, `edx` = the axis; the angle and whether the axis is unit; `ret 8`):
/// `axis_angle_mat3` in the upper 3×3 of a 4×4 whose fourth row and column are `(0, 0, 0, 1)`.
pub fn axis_angle_mat4(axis: &[f32; 3], angle: f32, normalized: bool) -> [f32; 16] {
    let r = axis_angle_mat3(axis, angle, normalized);
    [
        r[0], r[1], r[2], 0.0, r[3], r[4], r[5], 0.0, r[6], r[7], r[8], 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

/// `0x7bb860` (`ecx` = out, `edx` = the axis; the angle and whether the axis is unit; `ret 8`):
/// `axis_angle_mat3` as the nine rotation entries of a twelve-entry matrix whose translation
/// (`m[9..12]`) is zero.
#[must_use]
pub fn axis_angle_mat34(axis: &[f32; 3], angle: f32, normalized: bool) -> [f32; 12] {
    let r = axis_angle_mat3(axis, angle, normalized);
    [
        r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7], r[8], 0.0, 0.0, 0.0,
    ]
}

/// `fsincos` of `theta` taken at 53 bits and stored as `f32`.
fn cos_sin(theta: f32) -> (f32, f32) {
    let t = f64::from(theta);
    (t.cos() as f32, t.sin() as f32)
}

/// `BuildYawPitchRollBasis` (`0x7bf4b0`, `ecx` = out, the three angles on the stack; `ret 0xc`,
/// `systems/uicamera/view.md#pitch-units-and-sign`): the product `Rz(yaw) · (Ry(pitch) · Rx(roll))`
/// by `MultiplyMatrix3`, each factor from `fsincos`, written transposed through the 3×3 constructor
/// `0x5f8d20`. The first row of the result is column 0 of the product,
/// `(cos yaw·cos pitch, sin yaw·cos pitch, −sin pitch)`, whatever the roll.
#[must_use]
pub fn euler_to_mat3_f(angles: &[f32; 3]) -> [f32; 9] {
    let (cz, sz) = cos_sin(angles[0]);
    let (cy, sy) = cos_sin(angles[1]);
    let (cx, sx) = cos_sin(angles[2]);
    let z = [cz, -sz, 0.0, sz, cz, 0.0, 0.0, 0.0, 1.0];
    let y = [cy, 0.0, sy, 0.0, 1.0, 0.0, -sy, 0.0, cy];
    let x = [1.0, 0.0, 0.0, 0.0, cx, -sx, 0.0, sx, cx];
    let p = multiply_matrix3(&z, &multiply_matrix3(&y, &x));
    [p[0], p[3], p[6], p[1], p[4], p[7], p[2], p[5], p[8]]
}
