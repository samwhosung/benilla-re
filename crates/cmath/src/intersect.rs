//! The segment and triangle test (Möller and Trumbore) behind two entry points: `0x7c29f0` with
//! `u16` vertex indices and `IntersectRayTriangle` (`0x7c2c40`) with `u32` ones. The bodies are
//! the same apart from the index width.
//!
//! `ecx` = the ray (origin, then direction, which need not be unit), `edx` = the vertices (12
//! bytes each), then the three indices, `out_t`, `out_uv` (either may be null) and the
//! barycentric slack `eps`. The determinant test is two-sided at ±1e-6 (`[0x80e2e4]`).

/// `[0x80e2e4]`: 1e-6 as `f32`, widened at each compare.
const EPS_DET: f64 = 1e-6_f32 as f64;

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// A ray: origin at `+0`, direction at `+0xc`.
#[derive(Clone, Copy, Default)]
pub struct Ray {
    pub origin: [f32; 3],
    pub dir: [f32; 3],
}

/// The shared body. `−eps` and `eps + 1` are rounded to `f32`; so are the edges, `P = dir × e2`,
/// `1/det`, `T = origin − v0` and `Q = T × e1`. `u` is tested against `−eps` in the register and
/// against `1 + eps` as stored; `v` the same, with `u + v` summed from the stored values. `t` sums
/// its dot product in a different order from `u` and `v`.
fn mt_margined(
    ray: &Ray,
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
    out_t: Option<&mut f32>,
    out_uv: Option<&mut [f32; 2]>,
    eps: f32,
) -> bool {
    let neg_eps = -eps;
    let one_eps = (f(eps) + 1.0) as f32;

    let e1: [f32; 3] = core::array::from_fn(|i| (f(v1[i]) - f(v0[i])) as f32);
    let e2: [f32; 3] = core::array::from_fn(|i| (f(v2[i]) - f(v0[i])) as f32);

    let p = [
        (f(e2[2]) * f(ray.dir[1]) - f(e2[1]) * f(ray.dir[2])) as f32,
        (f(e2[0]) * f(ray.dir[2]) - f(e2[2]) * f(ray.dir[0])) as f32,
        (f(e2[1]) * f(ray.dir[0]) - f(e2[0]) * f(ray.dir[1])) as f32,
    ];

    let det = (f(p[2]) * f(e1[2]) + f(p[1]) * f(e1[1])) + f(p[0]) * f(e1[0]);

    // Go on for det ≤ −1e-6, det ≥ 1e-6 or NaN: `test ah,0x41; jne` at `0x7c2d37`, then `test
    // ah,5; jp` at `0x7c2d44`, which jumps on an equal compare too.
    if !(det <= -EPS_DET || det >= EPS_DET || det.is_nan()) {
        return false;
    }

    let inv_det = (1.0 / det) as f32;

    let t3: [f32; 3] = core::array::from_fn(|i| (f(ray.origin[i]) - f(v0[i])) as f32);

    let u64 = ((f(t3[2]) * f(p[2]) + f(t3[1]) * f(p[1])) + f(t3[0]) * f(p[0])) * f(inv_det);
    let u = u64 as f32;
    if u64 < f(neg_eps) {
        return false;
    }
    if f(u) > f(one_eps) {
        return false;
    }

    let q = [
        (f(t3[1]) * f(e1[2]) - f(t3[2]) * f(e1[1])) as f32,
        (f(t3[2]) * f(e1[0]) - f(e1[2]) * f(t3[0])) as f32,
        (f(e1[1]) * f(t3[0]) - f(t3[1]) * f(e1[0])) as f32,
    ];

    let v64 = ((f(q[2]) * f(ray.dir[2]) + f(q[1]) * f(ray.dir[1])) + f(q[0]) * f(ray.dir[0]))
        * f(inv_det);
    let v = v64 as f32;
    if v64 < f(neg_eps) {
        return false;
    }
    if f(v) + f(u) > f(one_eps) {
        return false;
    }

    if let Some(t) = out_t {
        *t = (((f(q[1]) * f(e2[1]) + f(q[2]) * f(e2[2])) + f(q[0]) * f(e2[0])) * f(inv_det)) as f32;
    }
    if let Some(uv) = out_uv {
        uv[0] = u;
        uv[1] = v;
    }
    true
}

/// `0x7c29f0`: the test on the vertices `u16` indices name.
pub fn segment_tri_moller(
    ray: &Ray,
    verts: &[[f32; 3]],
    idx: &[u16; 3],
    out_t: Option<&mut f32>,
    out_uv: Option<&mut [f32; 2]>,
    eps: f32,
) -> bool {
    mt_margined(
        ray,
        verts[idx[0] as usize],
        verts[idx[1] as usize],
        verts[idx[2] as usize],
        out_t,
        out_uv,
        eps,
    )
}

/// `IntersectRayTriangle` (`0x7c2c40`): the test on the vertices `u32` indices name.
pub fn segment_tri_moller_liquid(
    ray: &Ray,
    verts: &[[f32; 3]],
    idx: &[u32; 3],
    out_t: Option<&mut f32>,
    out_uv: Option<&mut [f32; 2]>,
    eps: f32,
) -> bool {
    mt_margined(
        ray,
        verts[idx[0] as usize],
        verts[idx[1] as usize],
        verts[idx[2] as usize],
        out_t,
        out_uv,
        eps,
    )
}
