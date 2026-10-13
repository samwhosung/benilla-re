//! A map object's bounds and vertex colours (`systems/models/wmo.md`): the bounding spheres
//! `CMapObj::GetBoundingSphere` (`0x6a4310`) and `CMapObj::GetGroupBoundingSphere` (`0x6a44b0`),
//! the volume test `PointInsidePlanes` (`0x6a4990`), the colour sample
//! `CMapObjGroup::SampleVertexColor` (`0x6b9a50`), and the portal fade
//! `CMapObjGroup::FixVertexAlpha` (`0x6c43d0`) with its distance `MapObjPortalPointDistance`
//! (`0x6c4240`).

use benilla_re_cmath::raymesh::dominant_axis;

use crate::x87::fistp;

/// 0.5 (`0x7ffa24`).
const HALF: f64 = 0.5;
/// 255.0 (`0x7ffe58`).
const C255: f64 = 255.0;
/// 512.0 (`0x8029cc`): added before a byte is read from a float's bits.
const ADD_512: f64 = 512.0;
/// 6.6666665 (`0x811110`): the fade's reach.
const FOG_CAP: f32 = f32::from_bits(0x40d5_5555);
/// 0.15 (`0x7ffd6c`): the fade per unit of distance.
const FOG_COEFF: f32 = f32::from_bits(0x3e19_999a);
/// 1/36 (`0x7ff9d0`): the squared distance from a portal's plane under which the point is tested
/// against the portal itself.
const NEAR_EPS_SQ: f32 = f32::from_bits(0x3ce3_8e39);

/// `0x6a4310` and `0x6a44b0` (`systems/models/wmo.md#bounding-spheres-and-volume`): with the map
/// object loaded (`+0x1d4`), the centre `f32(f32(max + min)·0.5)` and the radius
/// `√((dz² + dy²) + dx²)` with `d = max − centre` in the register; unloaded, zero. `0x6a4310` takes
/// the box at `+0x1a0`, `0x6a44b0` the box of the group record `+0x130 + 32·index`. Returns
/// `(centre, radius)`.
pub fn wmo_bounding_sphere(loaded: bool, min: [f32; 3], max: [f32; 3]) -> ([f32; 3], f32) {
    if !loaded {
        return ([0.0; 3], 0.0);
    }
    let center = [0, 1, 2].map(|i| (f64::from(max[i] + min[i]) * HALF) as f32);
    let [dx, dy, dz] = [0, 1, 2].map(|i| f64::from(max[i]) - f64::from(center[i]));
    let radius = ((dz * dz + dy * dy) + dx * dx).sqrt() as f32;
    (center, radius)
}

/// One plane of a map object's convex volume: `+0x15c`, 16 bytes each, `+0x194` of them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConvexPlane {
    /// The normal.
    pub n: [f32; 3],
    /// The distance term.
    pub d: f32,
}

/// `0x6a4990` (`systems/models/wmo.md#bounding-spheres-and-volume`): an unloaded map object returns
/// 0; otherwise 0 at the first plane with `((ny·py + nz·pz) + nx·px) + d > 0`, ordered (`test
/// ah,0x41; je` at `0x6a49e2`), so a zero or NaN distance continues; else 1.
pub fn wmo_point_in_convex_volume(loaded: bool, planes: &[ConvexPlane], point: [f32; 3]) -> u32 {
    if !loaded {
        return 0;
    }
    for pl in planes {
        let dist = ((f64::from(pl.n[1]) * f64::from(point[1])
            + f64::from(pl.n[2]) * f64::from(point[2]))
            + f64::from(pl.n[0]) * f64::from(point[0]))
            + f64::from(pl.d);
        if dist > 0.0 {
            return 0;
        }
    }
    1
}

/// The group arrays `0x6b9a50` reads.
pub trait WmoGroup {
    /// The index at a byte offset of `+0xc8`, the 16-bit vertex indices.
    fn movi(&self, byte_off: u32) -> u16;
    /// One coordinate of a vertex of `+0xd0`, 12 bytes each.
    fn movt(&self, vert: u16, axis: usize) -> f32;
    /// One byte of a vertex colour of `+0xf4`, 4 bytes each.
    fn mocv(&self, vert: u16, chan: usize) -> u8;
    /// The flag byte of a face of `+0xc4`, 2 bytes each.
    fn mopy_flag(&self, tri: u16) -> u8;
}

/// The result of [`wmo_mocv_barycentric_color`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MocvSample {
    /// The colour, alpha 0xff.
    pub bgra: [u8; 4],
    /// Bit 0 of the face's flags.
    pub side_flag: u8,
}

/// `0x6b9a50(point, face, colour_out, flag_out)`: the vertex colour at a point of a face, blended
/// with integer barycentric weights.
///
/// The face's normal `(v1 − v0) × (v2 − v0)` picks, through `DominantAxis` (`0x7bf680`), the plane
/// it is projected on: axis 0 to (y, z), 1 to (z, x), 2 to (x, y) (the tables at `0xcde62c`). On
/// it, with `e1 = v1 − v0`, `e2 = v2 − v0` and `qa` stored as `f32`, `qb` kept in the register
/// (`0x6b9b9a`), `inv = 1/(e2b·e1a − e1b·e2a)` and the weights
/// `w1 = fistp(f32(256·f32((e2b·qa − qb·e2a)·inv)) − 0.5)`,
/// `w2 = fistp(f32(256·f32((qb·e1a − e1b·qa)·inv)) − 0.5)`, `w0 = 256 − w2 − w1`. A negative
/// weight is spread over the other two by integer division, in the order w1, w2, w0
/// (`0x6b9c16` to `0x6b9c6e`). Each channel is `(c2·w2 + c0·w0 + c1·w1) >> 8` with the arithmetic
/// wrapping as `imul` does.
pub fn wmo_mocv_barycentric_color(
    group: &dyn WmoGroup,
    point: [f32; 3],
    tri: u16,
    proj_axes: ([usize; 2], [usize; 2], [usize; 2]),
) -> MocvSample {
    let base = u32::from(tri) * 6;
    let i0 = group.movi(base);
    let i1 = group.movi(base + 2);
    let i2 = group.movi(base + 4);
    let v = |i: u16, ax: usize| f64::from(group.movt(i, ax));

    let e2x = v(i2, 0) - v(i0, 0);
    let e2y = v(i2, 1) - v(i0, 1);
    let e2z = v(i2, 2) - v(i0, 2);
    let e1x = f64::from((v(i1, 0) - v(i0, 0)) as f32);
    let e1y = f64::from((v(i1, 1) - v(i0, 1)) as f32);
    let e1z = v(i1, 2) - v(i0, 2);
    let cross = [
        (e1y * e2z - e1z * e2y) as f32,
        (e1z * e2x - e1x * e2z) as f32,
        (e1x * e2y - e1y * e2x) as f32,
    ];
    let [pa, pb] = match dominant_axis(&cross) {
        0 => proj_axes.0,
        1 => proj_axes.1,
        _ => proj_axes.2,
    };

    let v0a = v(i0, pa);
    let v0b = v(i0, pb);
    let e1a = f64::from((v(i1, pa) - v0a) as f32);
    let e1b = f64::from((v(i1, pb) - v0b) as f32);
    let e2a = f64::from((v(i2, pa) - v0a) as f32);
    let e2b = f64::from((v(i2, pb) - v0b) as f32);
    let qa = f64::from((f64::from(point[pa]) - v0a) as f32);
    let qb = f64::from(point[pb]) - v0b;

    let inv = 1.0 / (e2b * e1a - e1b * e2a);
    let w1raw = ((e2b * qa - qb * e2a) * inv) as f32;
    let w2raw = ((qb * e1a - e1b * qa) * inv) as f32;
    let weight = |raw: f32| fistp(f64::from((f64::from(raw) * 256.0) as f32) - HALF);
    let mut w1 = weight(w1raw);
    let mut w2 = weight(w2raw);
    let mut w0 = 0x100i32.wrapping_sub(w2).wrapping_sub(w1);

    if w1 < 0 {
        let r = w2.wrapping_mul(w1) / w2.wrapping_add(w0);
        w2 = w2.wrapping_add(r);
        w0 = w0.wrapping_add(w1.wrapping_sub(r));
        w1 = 0;
    }
    if w2 < 0 {
        let r = w2.wrapping_mul(w1) / w0.wrapping_add(w1);
        w1 = w1.wrapping_add(r);
        w0 = w0.wrapping_add(w2.wrapping_sub(r));
        w2 = 0;
    }
    if w0 < 0 {
        let r = w0.wrapping_mul(w1) / w2.wrapping_add(w1);
        w2 = w2.wrapping_add(w0.wrapping_sub(r));
        w1 = w1.wrapping_add(r);
        w0 = 0;
    }

    let chan = |c: usize| -> u8 {
        let acc = (i32::from(group.mocv(i2, c)).wrapping_mul(w2))
            .wrapping_add(i32::from(group.mocv(i0, c)).wrapping_mul(w0))
            .wrapping_add(i32::from(group.mocv(i1, c)).wrapping_mul(w1));
        (acc >> 8) as u8
    };
    MocvSample {
        bgra: [chan(0), chan(1), chan(2), 0xff],
        side_flag: group.mopy_flag(tri) & 1,
    }
}

/// One vertex colour, `[B, G, R, A]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FogVertex {
    /// The four bytes.
    pub bgra: [u8; 4],
}

/// What [`wmo_vertex_fog_blend`] does to a vertex colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FogResult {
    /// The colour becomes 0xffffffff.
    White,
    /// The colour is faded to these bytes.
    Blended(FogVertex),
    /// The colour is kept.
    Untouched,
}

/// One vertex of `0x6c43d0` (`systems/models/wmo.md#portal-vertex-fade`), given the stored distance
/// `MapObjPortalDistance` (`0x6c4340`) returns: at `d = 0` (`test ah,0x44; jp` at `0x6c443c`, so
/// only an ordered zero) the colour becomes white; at `d < 6.6666665` (ordered, `0x6c4457`) with
/// alpha 0, `t = 1 − 0.15·d` in the register and each of r, g, b becomes the bits of
/// `f32((255 − c)·t + c + 512) >> 14`, alpha those of `f32(255·t + 512) >> 14`; otherwise nothing.
pub fn wmo_vertex_fog_blend(slot: FogVertex, dist: f32) -> FogResult {
    if dist == 0.0 {
        return FogResult::White;
    }
    if !(dist < FOG_CAP && slot.bgra[3] == 0) {
        return FogResult::Untouched;
    }
    let t = 1.0 - f64::from(dist) * f64::from(FOG_COEFF);
    let fixed = |ch: u8| -> u8 {
        let c = f64::from(ch);
        let s = ((C255 - c) * t + c + ADD_512) as f32;
        (s.to_bits() >> 14) as u8
    };
    let alpha = ((t * C255 + ADD_512) as f32).to_bits() >> 14;
    FogResult::Blended(FogVertex {
        bgra: [
            fixed(slot.bgra[0]),
            fixed(slot.bgra[1]),
            fixed(slot.bgra[2]),
            alpha as u8,
        ],
    })
}

/// `0x6c4240(point, portal)` (`systems/models/wmo.md#portal-vertex-fade`): the point's distance to
/// a portal. `pd = ((nz·pz + ny·py) + nx·px) + d` stays in the register; when `pd² < 1/36`
/// (ordered, `jp` at `0x6c428c`) and `in_portal` (`PointInPolygonProjected`, `0x7c23e0`, along
/// `DominantAxis`) holds, it returns 0. Otherwise it returns the least of `√dsq` over the portal's
/// edges, `dsq` from `PointSegmentDistanceSquared` (`0x7c46d0`), keeping a root only when it is
/// below the stored least (`jp` at `0x6c430f`: an equal or NaN root keeps it), from `FLT_MAX`.
pub fn aabsp_nearest_dist(
    point: [f32; 3],
    plane: [f32; 4],
    face_count: u16,
    dist_sq: &[f32],
    in_portal: &mut dyn FnMut() -> bool,
) -> f32 {
    let [px, py, pz] = point.map(f64::from);
    let [nx, ny, nz, d] = plane.map(f64::from);
    let pd = ((nz * pz + ny * py) + nx * px) + d;
    if pd * pd < f64::from(NEAR_EPS_SQ) && in_portal() {
        return 0.0;
    }
    let mut min = f32::MAX;
    for &dsq in dist_sq.iter().take(face_count as usize) {
        let root = f64::from(dsq).sqrt();
        if root < f64::from(min) {
            min = root as f32;
        }
    }
    min
}
