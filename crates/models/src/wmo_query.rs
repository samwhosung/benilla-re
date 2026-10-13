//! The arithmetic of a map object's segment and box queries (`systems/models/wmo.md#queries`):
//! `SegmentDirection` (`0x6bc190`), the reach and inverse length of
//! `CMapObjGroup::IntersectSegment` (`0x6b92b0`) and `CMapObjGroup::SegmentAnyHit` (`0x6b95e0`),
//! the hit point of `CMapObj::RayHitGroup` (`0x6a4cc0`), the face normal and its scaled dot of
//! `CMapObj::IntersectSegment` (`0x6a37b0`), and the box test of `CMapObj::GatherBoxGroups`
//! (`0x6a5020`).

/// `0x6bc190`: a segment's end minus its start, each component stored as `f32`.
pub fn segment_direction(seg: &[f32; 6]) -> [f32; 3] {
    [seg[3] - seg[0], seg[4] - seg[1], seg[5] - seg[2]]
}

/// `len = √((x² + y²) + z²)` through `Vector3LengthSquared` (`0x4549f0`), kept in the register;
/// then `f32(len·reach)` and `f32(1/len)`. `0x6b92b0` from `0x6b9354` to `0x6b9382` and `0x6b95e0`
/// from `0x6b9685` to `0x6b96c0`; a zero direction gives an infinite inverse. Returns `(len·reach,
/// 1/len)`.
pub fn segment_reach(dir: [f32; 3], reach: f32) -> (f32, f32) {
    let [x, y, z] = dir.map(f64::from);
    let len = (x * x + y * y + z * z).sqrt();
    ((len * f64::from(reach)) as f32, (1.0 / len) as f32)
}

/// `0x6b95e0` from `0x6b9685` to `0x6b96dd`: [`segment_reach`], then each component times the
/// stored inverse, stored as `f32`. Returns `(len·reach, 1/len, unit)`.
pub fn seg_normalize(dir: [f32; 3], reach: f32) -> (f32, f32, [f32; 3]) {
    let (scaled, inv) = segment_reach(dir, reach);
    let unit = dir.map(|c| (f64::from(c) * f64::from(inv)) as f32);
    (scaled, inv, unit)
}

/// `0x6a4cc0` from `0x6a4d3c` to `0x6a4da2`: `dir = p1 − p0` stored as `f32`, then
/// `hit = dir·t + p0` with the product kept in the register, stored as `f32`.
pub fn cmapobj_seg_hit_point(p0: [f32; 3], p1: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|c| {
        let dir = p1[c] - p0[c];
        (f64::from(dir) * f64::from(t) + f64::from(p0[c])) as f32
    })
}

/// `0x6a37b0` from `0x6a3920` to `0x6a39ab`: a hit face's normal `e2 × e1` with `e1 = v0 − v2`
/// kept in the register and `e2 = v1 − v2`, whose x and y are stored as `f32` and z kept:
/// `(e2.y·e1.z − e2.z·e1.y, e2.z·e1.x − e1.z·e2.x, e2.x·e1.y − e2.y·e1.x)`, each stored as `f32`.
/// `v0`, `v1` and `v2` are the face's third, second and first vertices.
pub fn seg_face_cross(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> [f32; 3] {
    let e1x = f64::from(v0[0]) - f64::from(v2[0]);
    let e1y = f64::from(v0[1]) - f64::from(v2[1]);
    let e1z = f64::from(v0[2]) - f64::from(v2[2]);
    let e2x = f64::from(v1[0] - v2[0]);
    let e2y = f64::from(v1[1] - v2[1]);
    let e2z = f64::from(v1[2]) - f64::from(v2[2]);
    [
        (e2y * e1z - e2z * e1y) as f32,
        (e2z * e1x - e1z * e2x) as f32,
        (e2x * e1y - e2y * e1x) as f32,
    ]
}

/// `0x6a37b0` from `0x6a39bd` to `0x6a39d5`: `(n.z·d.x + n.y·d.y) + n.x·d.z` in the register,
/// stored as `f32`, with `n` the face normal and `d` the segment's direction in the group's frame.
pub fn seg_face_dot(cross: [f32; 3], tdir: [f32; 3]) -> f32 {
    let p0 = f64::from(cross[2]) * f64::from(tdir[0]);
    let p1 = f64::from(cross[1]) * f64::from(tdir[1]);
    let p2 = f64::from(cross[0]) * f64::from(tdir[2]);
    ((p0 + p1) + p2) as f32
}

/// `0x6a37b0` from `0x6a3a11` to `0x6a3a2d`: the stored dot over `√((n.z² + n.y²) + n.x²)`, stored
/// as `f32`; a zero normal divides by zero.
pub fn seg_normal_scale(cross: [f32; 3], dot: f32) -> f32 {
    let [r0, r1, r2] = cross.map(f64::from);
    (f64::from(dot) / ((r2 * r2 + r1 * r1) + r0 * r0).sqrt()) as f32
}

/// `0x6a5020` from `0x6a5140` to `0x6a51a9`: a corner `q` against the query box `b`, six
/// differences stored as `f32` whose sign bits make the corner's outcode: `q − b.min` per axis,
/// then `b.max − q`.
pub fn box_overlap_deltas(b: [f32; 6], q: [f32; 3]) -> [f32; 6] {
    [
        q[0] - b[0],
        q[1] - b[1],
        q[2] - b[2],
        b[3] - q[0],
        b[4] - q[1],
        b[5] - q[2],
    ]
}
