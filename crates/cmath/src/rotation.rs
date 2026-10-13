//! `BuildAxisRotation` (`0x7be490`), the rotation about an axis the particle quads and the spline
//! emitter build (`systems/models/emission.md#spline-spawn`,
//! `systems/models/particles.md#head-quads`).

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
