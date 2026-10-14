//! `BuildDecalTransforms` (`0x6d6fa0`, `systems/lighting/shadows.md#projected-decals`), the scale
//! `DrawDecalBatches` (`0x6d7480`) passes to the line-width call. The x87 runs at 53 bits and
//! stores `f32` at each `fstp dword`. The matrix work is `benilla_re_cmath`: `multiply_matrix4`
//! (`0x7bc6a0`), `axis_angle_mat4` (`0x7bdb00`) and `translate_matrix` (`0x7bdc40`).

use benilla_re_cmath::multiply::multiply_matrix4 as mat4_mul;
use benilla_re_cmath::rotation::axis_angle_mat4;
use benilla_re_cmath::transform::translate_matrix;

/// `[0x811694]`, 3.0518044e-5 (1/32768).
const C_SKY_SCALE: f32 = f32::from_bits(0x3800_0080);
/// `[0x801360]`, 0.001.
const C_EPS: f32 = f32::from_bits(0x3a83_126f);
/// `[0x811668]`, π.
const C_PI: f32 = f32::from_bits(0x4049_0fdb);
/// `[0x803068]`, −0.5.
const C_NEG_HALF: f32 = f32::from_bits(0xbf00_0000);
/// `[0x7ffa24]`, 0.5.
const C_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x7ff9d8]`, 1.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);

/// The call `0x6d7480` makes to `0x589e30` from `0x6d7506`: when `[0xcea620]` is exactly 0 (`fcomp`
/// with 0, `test ah,0x44; jnp`, which also skips -0 but not NaN) there is none. Otherwise the
/// argument is `(g + g)·3.0518044e-5` at 53 bits, stored as `f32` at `[esp]`.
pub fn sky_element_draw_scale(g: f32) -> Option<f32> {
    if g == 0.0 {
        return None;
    }
    Some(((f64::from(g) + f64::from(g)) * f64::from(C_SKY_SCALE)) as f32)
}

/// The camera position `0x481900` returns (`[0xb4b2bc]`, then `+0x65b8`, then `+8`): used when the
/// flag is clear.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkyCamera {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// What `BuildDecalTransforms` writes: the matrices at `ecx` and `edx`, and whether the box was
/// large enough. When it is not, both are left as they were.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyBillboard {
    /// The texture transform: `M · scale · R` (times the extra matrix when given), `+0.5` on
    /// entries 12 and 13.
    pub out1: [f32; 16],
    /// The height fade: `M · proj`.
    pub out2: [f32; 16],
    pub built: bool,
}

/// `0x6d6fa0(box, extra, fade, flag)` (`ecx` = out1, `edx` = out2, `ret 0x10`), `box` six floats
/// (minimum then maximum corner), `extra` an optional 4×4, `fade` the value for entries 12 and 14
/// of the fade matrix, `flag` whether to drop the camera:
/// 1. `center = 0.5·(min + max)`, each sum and product stored as `f32`; `dir` is the camera minus
///    the center, or the negated center when the flag is set; `M` is the identity with `dir` added
///    to its translation row (`translate_matrix`).
/// 2. `extent = max − min`, stored as `f32`; when `extent.x < 0.001` or `extent.y < 0.001`
///    (`fcomp`, `test ah,5; jnp`, taken only for a value below it, so NaN goes on) it returns with
///    nothing written.
/// 3. `scale = diag(1/ex, 1/ey, 1, 1)`, `R = axis_angle_mat4([0, 0, 1], π·(−0.5))`, `out1 =
///    (M·scale)·R`, then `out1·extra` when given, then 0.5 added to entries 12 and 13.
/// 4. `proj` is zero except entries 8 and 10 = `1/ez`, 12 and 14 = `fade`, 13 = 0.5, 15 = 1; `out2 =
///    M·proj`.
pub fn sky_billboard_build(
    cam: SkyCamera,
    param_3: &[f32; 6],
    param_4: Option<&[f32; 16]>,
    param_5: f32,
    param_6_camera: bool,
    out1_in: &[f32; 16],
    out2_in: &[f32; 16],
) -> SkyBillboard {
    let f = f64::from;
    let identity: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    let sum = [
        (f(param_3[0]) + f(param_3[3])) as f32,
        (f(param_3[1]) + f(param_3[4])) as f32,
        (f(param_3[2]) + f(param_3[5])) as f32,
    ];
    let center = [
        (f(C_HALF) * f(sum[0])) as f32,
        (f(C_HALF) * f(sum[1])) as f32,
        (f(C_HALF) * f(sum[2])) as f32,
    ];

    let dir = if param_6_camera {
        [-center[0], -center[1], -center[2]]
    } else {
        [
            (f(cam.x) - f(center[0])) as f32,
            (f(cam.y) - f(center[1])) as f32,
            (f(cam.z) - f(center[2])) as f32,
        ]
    };

    let mut m_a = identity;
    translate_matrix(&mut m_a, &dir);

    let extent = [
        (f(param_3[3]) - f(param_3[0])) as f32,
        (f(param_3[4]) - f(param_3[1])) as f32,
        (f(param_3[5]) - f(param_3[2])) as f32,
    ];
    if extent[0] < C_EPS || extent[1] < C_EPS {
        return SkyBillboard {
            out1: *out1_in,
            out2: *out2_in,
            built: false,
        };
    }

    let inv_x = (f(C_ONE) / f(extent[0])) as f32;
    let inv_y = (f(C_ONE) / f(extent[1])) as f32;
    let scale: [f32; 16] = [
        inv_x, 0.0, 0.0, 0.0, 0.0, inv_y, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    let axis = [0.0f32, 0.0, 1.0];
    let angle = (f(C_PI) * f(C_NEG_HALF)) as f32;
    let r = axis_angle_mat4(&axis, angle, true);

    let mut out1 = mat4_mul(&mat4_mul(&m_a, &scale), &r);
    if let Some(p4) = param_4 {
        out1 = mat4_mul(&out1, p4);
    }
    out1[12] = (f(out1[12]) + f(C_HALF)) as f32;
    out1[13] = (f(out1[13]) + f(C_HALF)) as f32;

    let inv_z = (f(C_ONE) / f(extent[2])) as f32;
    let proj: [f32; 16] = [
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, inv_z, 0.0, inv_z, 0.0, param_5, C_HALF, param_5,
        C_ONE,
    ];
    let out2 = mat4_mul(&m_a, &proj);

    SkyBillboard {
        out1,
        out2,
        built: true,
    }
}
