//! The cull volumes and the portal rectangle: `BuildFrustumVolume` (`0x682490`),
//! `BuildPortalVolume` (`0x682930`), `TestOrientedBoxPlanes` (`0x6869c0`) and `ProjectPortal`
//! (`0x6b46f0`). A volume is six planes 16 bytes apart at `+0x0` and eight corners 12 bytes apart
//! at `+0x60` (`crate::frustum::QueryVolume`).
//!
//! The two builders own the corners. They hand them to `0x6865f0`, which copies them to `+0x60`
//! and builds the planes with `0x686640`, and then `qsort` a local copy of them, which nothing
//! reads afterwards.

use benilla_re_cmath::raymesh::{dominant_axis, point_in_poly};
use benilla_re_cmath::transform::transform_vec4;
use benilla_re_objectlayer::spline_anchor::dot3;

use crate::clip::poly_clip;

/// `[0x8101b4]`: −0.019444 (`0xbc9f49f4`), how far behind a plane a box may reach.
const EPS_NEG: f32 = f32::from_bits(0xbc9f_49f4);

/// `[0x8029d0]`: 0.01 (`0x3c23d70a`), the eye's distance at which it is on the portal's plane.
const EYE_ON_PLANE_EPS: f32 = f32::from_bits(0x3c23_d70a);

/// `[0x801360]`: 0.001 (`0x3a83126f`), the least `|w|` divided by.
const W_CLAMP_EPS: f32 = f32::from_bits(0x3a83_126f);

/// 1e-5 (`0x3727c5ac`), the `w` used in place of a smaller one.
const W_CLAMP_VAL: f32 = f32::from_bits(0x3727_c5ac);

/// A vector of three components, each stored as `f32` before it is passed to `0x4549a0`.
type V3 = [f32; 3];

/// `a − b`, each component stored as `f32`.
#[inline]
fn sub3(a: V3, b: V3) -> V3 {
    [
        (f64::from(a[0]) - f64::from(b[0])) as f32,
        (f64::from(a[1]) - f64::from(b[1])) as f32,
        (f64::from(a[2]) - f64::from(b[2])) as f32,
    ]
}

/// `a + b`, each component stored as `f32`.
#[inline]
fn add3(a: V3, b: V3) -> V3 {
    [
        (f64::from(a[0]) + f64::from(b[0])) as f32,
        (f64::from(a[1]) + f64::from(b[1])) as f32,
        (f64::from(a[2]) + f64::from(b[2])) as f32,
    ]
}

/// `v·s`, each component stored as `f32`.
#[inline]
fn scale3(v: V3, s: f32) -> V3 {
    [
        (f64::from(v[0]) * f64::from(s)) as f32,
        (f64::from(v[1]) * f64::from(s)) as f32,
        (f64::from(v[2]) * f64::from(s)) as f32,
    ]
}

/// The frustum's corner rays at `0xc7bcd8`, eight vectors 12 bytes apart.
pub type CornerRays = [V3; 8];

/// Four corners narrowed to the rectangle `[x0, y1, x2, y3]` from two rays, `a` from `base_a` to
/// `tip_a` and `b` likewise: `lo = ray·y1 + base` and `hi = ray·y3 + base` on each, then `lo_b +
/// (lo_a − lo_b)·x0`, `… ·x2`, and on the high pair `·x2` before `·x0`, every step stored.
#[allow(clippy::too_many_arguments)]
#[inline]
fn narrow_quad(base_a: V3, tip_a: V3, base_b: V3, tip_b: V3, rect: &[f32; 4]) -> [V3; 4] {
    let ray_a = sub3(tip_a, base_a);
    let lo_a = add3(scale3(ray_a, rect[1]), base_a);
    let hi_a = add3(scale3(ray_a, rect[3]), base_a);
    let ray_b = sub3(tip_b, base_b);
    let lo_b = add3(scale3(ray_b, rect[1]), base_b);
    let hi_b = add3(scale3(ray_b, rect[3]), base_b);
    let edge_lo = sub3(lo_a, lo_b);
    let c0 = add3(scale3(edge_lo, rect[0]), lo_b);
    let c1 = add3(scale3(edge_lo, rect[2]), lo_b);
    let edge_hi = sub3(hi_a, hi_b);
    let c2 = add3(scale3(edge_hi, rect[2]), hi_b); // `+0x18`, through `0x5f8cf0` and `0x621160`
    let c3 = add3(scale3(edge_hi, rect[0]), hi_b); // `+0x24`
    [c0, c1, c2, c3]
}

/// The eight corners in two passes, the second 0x30 bytes further into the rays: pass `k` reads
/// rays `4k` to `4k + 3` as `base_b`, `base_a`, `tip_a`, `tip_b` and writes corners `4k` to `4k + 3`.
fn build_corners(rays: &CornerRays, rect: &[f32; 4]) -> [V3; 8] {
    let mut corners = [[0.0f32; 3]; 8];
    for k in 0..2usize {
        let base_b = rays[4 * k];
        let base_a = rays[4 * k + 1];
        let tip_a = rays[4 * k + 2];
        let tip_b = rays[4 * k + 3];
        let q = narrow_quad(base_a, tip_a, base_b, tip_b, rect);
        corners[4 * k..4 * k + 4].copy_from_slice(&q);
    }
    corners
}

/// `BuildFrustumVolume` (`0x682490`, ecx = the rectangle): the root volume's corners from the rays
/// at `0xc7bcd8`, at a fixed address plus 0x30 a pass.
pub fn frustum_root_rect(rays: &CornerRays, rect: &[f32; 4]) -> [V3; 8] {
    build_corners(rays, rect)
}

/// `BuildPortalVolume` (`0x682930`, ecx = the rays, edx = the rectangle): the same corners, the
/// rays passed in ecx (`0x6b41c0` passes `0xc7bcd8`) and walked by pointer.
pub fn frustum_push_rect(rays: &CornerRays, rect: &[f32; 4]) -> [V3; 8] {
    build_corners(rays, rect)
}

/// A box as `0x6869c0` reads it: the two corners (minimum, maximum), the nine floats of its axes,
/// and its centre.
#[derive(Clone, Debug, PartialEq)]
pub struct Obb {
    pub corners: [V3; 2],
    pub axes: [f32; 9],
    pub center: V3,
}

/// `TestOrientedBoxPlanes` (`0x6869c0`): 0 when the box is behind any of six planes, else 3. For
/// each plane the normal is taken into the box's frame through the transpose `0x5f8d20` copies, the
/// corner farthest along it picked per axis by the sign bit (minimum when negative), taken back out
/// through the axes plus the centre, and rejected when `n·p + d` (`0x602630`, then `fadd`) is below
/// −0.019444; NaN does not reject.
pub fn obb_plane_cull(planes: &[[f32; 4]; 6], obb: &Obb) -> u32 {
    // `0x5f8d20` takes the axes pushed as a0, a3, a6, a1, a4, a7, a2, a5, a8.
    let a = &obb.axes;
    let m = [a[0], a[3], a[6], a[1], a[4], a[7], a[2], a[5], a[8]];
    let min_c = obb.corners[0];
    let max_c = obb.corners[1];

    for plane in planes.iter() {
        let n = [plane[0], plane[1], plane[2]];
        // Each component `(m_i·nx + m_{i+6}·nz) + m_{i+3}·ny`, stored before its sign is read.
        let nl = [
            (f64::from(m[0]) * f64::from(n[0])
                + f64::from(m[6]) * f64::from(n[2])
                + f64::from(m[3]) * f64::from(n[1])) as f32,
            (f64::from(m[1]) * f64::from(n[0])
                + f64::from(m[7]) * f64::from(n[2])
                + f64::from(m[4]) * f64::from(n[1])) as f32,
            (f64::from(m[2]) * f64::from(n[0])
                + f64::from(m[8]) * f64::from(n[2])
                + f64::from(m[5]) * f64::from(n[1])) as f32,
        ];
        let pick = |i: usize| -> f32 {
            if (nl[i].to_bits() >> 31) == 1 {
                min_c[i]
            } else {
                max_c[i]
            }
        };
        let s = [pick(0), pick(1), pick(2)];
        // `0x686a72` to `0x686acd`.
        let world = [
            (f64::from(s[2]) * f64::from(a[6])
                + f64::from(s[0]) * f64::from(a[0])
                + f64::from(s[1]) * f64::from(a[3])) as f32,
            (f64::from(s[0]) * f64::from(a[1])
                + f64::from(s[2]) * f64::from(a[7])
                + f64::from(s[1]) * f64::from(a[4])) as f32,
            (f64::from(s[1]) * f64::from(a[5])
                + f64::from(s[0]) * f64::from(a[2])
                + f64::from(s[2]) * f64::from(a[8])) as f32,
        ];
        let w = add3(world, obb.center);
        let dist = dot3(n, w) + f64::from(plane[3]);
        if dist < f64::from(EPS_NEG) {
            return 0;
        }
    }
    3
}

/// The view-projection matrix at `0xcbe2d8`.
pub type ViewProj = [f32; 16];

/// The eye at `0xca7edc`.
pub type EyePos = V3;

/// What `0x6b46f0` writes: bit 0 of the first byte when the clipped polygon is empty, and the
/// rectangle `(min_x, min_y, max_x, max_y)`.
#[derive(Clone, Debug, PartialEq)]
pub struct PortalRect {
    pub culled: bool,
    pub rect: [f32; 4],
}

/// `ProjectPortal` (`0x6b46f0`), the path without SSE (`0xc9e388` clear): the portal's screen
/// rectangle. Each vertex `(x, y, z, 1)` goes through `TransformVector4` (`0x7bcb40`) with the
/// matrix at `0xcbe2d8`, and `(x, y, w)` is clipped against the planes at `0xc7d0dc`
/// (`ClipPortalPolygon`); nothing left culls it. The plane of the first three vertices, normalized,
/// decides whether the eye is on it (`|n·eye − n·v0| <= 0.01`) and inside the polygon
/// (`IsPointInPolygon`, `0x7c23e0`, along `0x7bf680`'s axis): then the rectangle is the whole
/// screen. Otherwise it is the bounds of `x/w`, `y/w` over the clipped vertices, `w` below 0.001 in
/// size replaced by 1e-5. With SSE the normal is scaled by `rsqrtss`, which differs by CPU.
#[allow(clippy::too_many_arguments)]
pub fn portal_screen_rect(
    verts: &[V3],
    view_proj: &ViewProj,
    guard_planes: &[[f32; 3]; 4],
    eye: &EyePos,
    rsqrt_gate: u32,
) -> PortalRect {
    let count = verts.len();
    let mut clip: Vec<V3> = Vec::with_capacity(count);
    for v in verts {
        let p = transform_vec4(&[v[0], v[1], v[2], 1.0], view_proj);
        clip.push([p[0], p[1], p[3]]);
    }

    let clipped = poly_clip(&clip, guard_planes);
    let Some(clipped) = clipped else {
        return PortalRect {
            culled: true,
            rect: [0.0; 4],
        };
    };

    // The normal `(v2 − v0) × (v1 − v0)` in the function's terms. `v2 − v0` in x and y is stored
    // as `f32` (`0x6b4829`, `0x6b4832`); the other differences stay at 53 bits. Each component is
    // stored.
    let v0 = verts[0];
    let v1 = verts[1];
    let v2 = verts[2];
    let d1x = f64::from(v1[0]) - f64::from(v0[0]);
    let d1y = f64::from(v1[1]) - f64::from(v0[1]);
    let d1z = f64::from(v1[2]) - f64::from(v0[2]);
    let d2x = (f64::from(v2[0]) - f64::from(v0[0])) as f32 as f64;
    let d2y = (f64::from(v2[1]) - f64::from(v0[1])) as f32 as f64;
    let d2z = f64::from(v2[2]) - f64::from(v0[2]);
    let nxf = (d2z * d1y - d1z * d2y) as f32;
    let nyf = (d1z * d2x - d2z * d1x) as f32;
    let nzf = (d2y * d1x - d1y * d2x) as f32;
    // `(nz² + ny²) + nx²` at 53 bits, then `1/√` at 53 bits, nothing rounded to `f32` between
    // (`0x6b4868` to `0x6b48aa`).
    let magsq = (f64::from(nzf) * f64::from(nzf) + f64::from(nyf) * f64::from(nyf))
        + f64::from(nxf) * f64::from(nxf);
    assert!(
        rsqrt_gate == 0,
        "the rsqrtss path (0xc9e388 set) differs by CPU and is not modelled"
    );
    let inv = 1.0f64 / magsq.sqrt();
    let (nxf, nyf, nzf) = (
        (f64::from(nxf) * inv) as f32,
        (f64::from(nyf) * inv) as f32,
        (f64::from(nzf) * inv) as f32,
    );

    // `−((nx·v0x + nz·v0z) + ny·v0y)` is stored as `f32` (`0x6b48fe`); the eye's `(nz·ez + ny·ey) +
    // nx·ex` and that store are summed at 53 bits and compared, unrounded, with 0.01f
    // (`0x6b4901` to `0x6b4925`).
    let plane = (-((f64::from(nxf) * f64::from(v0[0]) + f64::from(nzf) * f64::from(v0[2]))
        + f64::from(nyf) * f64::from(v0[1]))) as f32;
    let d = ((f64::from(nzf) * f64::from(eye[2]) + f64::from(nyf) * f64::from(eye[1]))
        + f64::from(nxf) * f64::from(eye[0]))
        + f64::from(plane);
    let d = d.abs();
    if d <= f64::from(EYE_ON_PLANE_EPS) {
        let flat: Vec<f32> = verts.iter().flatten().copied().collect();
        if point_in_poly(eye, &flat, count, dominant_axis(&[nxf, nyf, nzf])) {
            return PortalRect {
                culled: false,
                rect: [-1.0, -1.0, 1.0, 1.0],
            };
        }
    }

    // The bounds start at ±FLT_MAX; `+4` is min y, `+8` min x, `+0xc` max y, `+0x10` max x.
    let fmax = f32::from_bits(0x7f7f_ffff);
    let fmin = f32::from_bits(0xff7f_ffff);
    let mut min_x = fmax;
    let mut min_y = fmax;
    let mut max_x = fmin;
    let mut max_y = fmin;
    for v in &clipped {
        let mut w = v[2];
        if w.abs() < W_CLAMP_EPS {
            w = W_CLAMP_VAL;
        }
        // `1/w` stays in the register for both products (`0x6b49e0` to `0x6b49f3`).
        let r = f64::from(1.0f32) / f64::from(w);
        let sx = (r * f64::from(v[0])) as f32;
        let sy = (r * f64::from(v[1])) as f32;
        // Each bound moves on a strict compare; NaN leaves it.
        if sx < min_x {
            min_x = sx;
        }
        if max_x < sx {
            max_x = sx;
        }
        if sy < min_y {
            min_y = sy;
        }
        if max_y < sy {
            max_y = sy;
        }
    }
    PortalRect {
        culled: false,
        rect: [min_x, min_y, max_x, max_y],
    }
}
