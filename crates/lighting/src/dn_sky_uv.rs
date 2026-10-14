//! The two projections of the cloud sphere: `ProjectCloudLight` (`0x6cf9c0`,
//! `systems/lighting/clouds.md#cloud-colours`) and `ProjectToCloudTexture` (`0x6cf870`,
//! `clouds.md#cover-projection`). The x87 runs at 53 bits, `fst dword` stores a slot as `f32` and
//! keeps the register, and a later `fld` of the slot reloads the `f32`. The root solve is
//! `benilla_re_fastmath::quadratic` and `acos` is the host's (`0x741820` is the C runtime's).

use benilla_re_fastmath::quadratic::quadratic_roots;

/// `[0x81159c]`, 0.25.
pub const QUARTER: f32 = 0.25;
/// `[0x811508]`, π.
pub const PI_F32: f32 = std::f32::consts::PI;
/// `[0x811570]`, 0.
pub const ZERO_811570: f32 = 0.0;
/// `[0x7ffd74]`, 0.
pub const ZERO: f32 = 0.0;
/// `[0x7ffa24]`, 0.5.
pub const HALF: f32 = 0.5;
/// `[0x7ff9d8]`, 1.
pub const ONE: f32 = 1.0;
/// `[0x8015b8]`, 1 as an `f64`, whose square root the projection multiplies by.
pub const ONE_F64: f64 = 1.0;
/// `[0x811600]`, 1e-5 as an `f64`: the horizontal length below which the direction is 0.
pub const HYP_EPS: f64 = 1e-05;

/// The point `[0xce9b78]`, `[0xce9b7c]`, `[0xce9b80]` that `0x6cf870` subtracts and `0x6cf9c0`
/// adds.
pub const ORIGIN_GLOBALS: [u64; 3] = [0xce9b78, 0xce9b7c, 0xce9b80];

/// A vector of three `f32`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `−cos(0.25·π)`, with the product kept wide for `fcos` and then `fchs`.
fn neg_cos_quarter_pi() -> f64 {
    let arg = f64::from(QUARTER) * f64::from(PI_F32);
    -arg.cos()
}

/// `0x6cf9c0(p, v, out)` (`ret 0xc`): the hit of the ray of direction `v` with the cloud sphere.
/// With `k = −cos(0.25·π)`, the quadratic has `a = (vx² + vy²) + vz²`, `b = (−k·vz)` doubled and `c =
/// k² − 1`, each stored as `f32`, and `0x454f40` gives its larger root `t2`. The output is `t2·v +
/// p + O` per component, rounded differently: x is `f32(f32(t2·vx + px) + Ox)`; y is `f32((t2·vy +
/// py) + Oy)` with nothing stored between; z is `f32((f32(t2·vz) + pz) + Oz)`. Returns none when
/// the solver finds no root.
pub fn sky_dome_ray_point(p: Vec3, v: Vec3, origin: [f32; 3]) -> Option<Vec3> {
    let f = f64::from;
    let k = neg_cos_quarter_pi();

    let a = (f(v.x) * f(v.x) + f(v.y) * f(v.y) + f(v.z) * f(v.z)) as f32;
    let b = {
        let neg_k_vz = -k * f(v.z);
        (neg_k_vz + neg_k_vz) as f32
    };
    let c = (k * k - f(ONE)) as f32;

    let t2 = quadratic_roots(a, b, c)?.1;

    let t2 = f(t2);
    let out_x = {
        let plus_p = (t2 * f(v.x) + f(p.x)) as f32;
        (f(plus_p) + f(origin[0])) as f32
    };
    let out_y = (t2 * f(v.y) + f(p.y) + f(origin[1])) as f32;
    let out_z = {
        let t2vz = (t2 * f(v.z)) as f32;
        (f(t2vz) + f(p.z) + f(origin[2])) as f32
    };
    Some(Vec3 {
        x: out_x,
        y: out_y,
        z: out_z,
    })
}

/// The two texture coordinates `0x6cf870` writes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudUv {
    pub u: f32,
    pub v: f32,
}

/// `0x6cf870(vin, out)` (`ecx` = the cloud object, `ret 8`), with `n` the integer at `+0x1c`:
/// - `d0 = f32(vin.x − Ox)`, `d1 = f32(vin.y − Oy)`, and `dz = vin.z − Oz` wide;
/// - `qpi = 0.25·π` wide, also stored as `f32`; `kq = −cos(qpi)`; `c = f32(dz − kq)`;
/// - `cval = ((d1 + d0)·0 + c)` (the product with 0 keeps NaN), `len = √((c² + d1²) + d0²)`, and
///   `arg = cval/(len·√1)` as the wide quotient;
/// - `theta = acos(arg)`, replaced by the `f32` `qpi` when above it (`fcom`, `test ah,0x41; jne`,
///   which keeps `theta` for NaN), and `phase = ((theta − 0)/(qpi − 0))·0.5`;
/// - `hyp = √(d0² + d1²)`; above 1e-5 (NaN keeps zero) `inv = f32(1/hyp)` and the direction is
///   `(inv·d0, inv·d1)`, otherwise `(0, 0)`;
/// - `u = (cx·phase + 0.5)·n` and `v = (cy·phase + 0.5)·n`, each stored as `f32`.
pub fn sky_cloud_uv_scroll(vin: Vec3, origin: [f32; 3], n: i32) -> CloudUv {
    let f = f64::from;
    let d0 = (f(vin.x) - f(origin[0])) as f32;
    let d1 = (f(vin.y) - f(origin[1])) as f32;
    let dz = f(vin.z) - f(origin[2]);

    let quarter_pi_arg = f(QUARTER) * f(PI_F32);
    let quarter_pi_f32 = quarter_pi_arg as f32;
    let kq = -quarter_pi_arg.cos();
    let c = (dz - kq) as f32;

    let a = d0;
    let b = d1;
    let cval = (f(b) + f(a)) * f(ZERO) + f(c);
    let len = (f(c) * f(c) + f(b) * f(b) + f(a) * f(a)).sqrt();
    let denom = len * ONE_F64.sqrt();
    let arg = cval / denom;

    let theta = arg.acos();

    let clamped = if theta > f(quarter_pi_f32) {
        f(quarter_pi_f32)
    } else {
        theta
    };
    let phase = (clamped - f(ZERO)) / (f(quarter_pi_f32) - f(ZERO)) * f(HALF);

    let hyp = (f(d0) * f(d0) + f(d1) * f(d1)).sqrt();
    let (cx, cy) = if hyp > HYP_EPS {
        let inv = (f(ONE) / hyp) as f32;
        (f(inv) * f(d0), f(inv) * f(d1))
    } else {
        (f(ZERO), f(ZERO))
    };

    let n_f = f64::from(n);
    let uv = |chan: f64| -> f32 { ((chan * phase + f(HALF)) * n_f) as f32 };
    CloudUv {
        u: uv(cx),
        v: uv(cy),
    }
}
