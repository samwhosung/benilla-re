//! The arithmetic of the world's map object visibility and traces: the view rectangles of
//! `SeedInteriorVisibility` (`0x6b3b20`) and `SeedExteriorVisibility` (`0x6b3dd0`), the portal
//! reach `MaxPortalDistance` (`0x6b4680`), the debug normals of `DrawGroupNormals` (`0x6b5f90`),
//! the hit point of `CMapObjDef::TraceGroup` (`0x6a2600`), the plane of `TraceMapObjectPlanes`
//! (`0x6aa1f0`) and the hit compare of `IntersectMapObjects` (`0x6be250`).

/// `2x − 1` with `2x` kept in the register (`fadd st,st`), stored as `f32`.
fn bias_live(x: f32) -> f32 {
    (2.0 * f64::from(x) - 1.0) as f32
}

/// `2x − 1` with `2x` stored as `f32` first.
fn bias_reload(x: f32) -> f32 {
    (2.0 * x) - 1.0
}

/// `0x6b3b20` from `0x6b3ca9` to `0x6b3cf7`: a screen rectangle in `[0, 1]` mapped to `[−1, 1]` as
/// `2x − 1`; the first two doubles stay in the register, the last two are stored as `f32` first.
pub fn camera_inside_bias(x: [f32; 4]) -> [f32; 4] {
    [
        bias_live(x[0]),
        bias_live(x[1]),
        bias_reload(x[2]),
        bias_reload(x[3]),
    ]
}

/// `0x6b3dd0` from `0x6b3e96` to `0x6b3eea`: the same mapping as [`camera_inside_bias`].
pub fn camera_outside_bias(x: [f32; 4]) -> [f32; 4] {
    camera_inside_bias(x)
}

/// `0x6b4680(portal)`: the largest distance of a portal's vertices from the plane at `0xca7ef0`,
/// from 0. Each `d = ((C·z + A·x) + B·y) + D` is stored as `f32` by `fst` and compared unrounded;
/// `d` above the largest (`test ah,0x41; jne` at `0x6b46d6`, so not on a tie or NaN) replaces it
/// with the stored `d`. The result is left in `st0`.
pub fn portal_plane_maxdist(verts: &[[f32; 3]], plane: [f32; 4]) -> f64 {
    let [a, b, c, d] = plane.map(f64::from);
    let mut max = 0.0f64;
    for v in verts {
        let [x, y, z] = v.map(f64::from);
        let dist = ((c * z + a * x) + b * y) + d;
        if max < dist {
            max = f64::from(dist as f32);
        }
    }
    max
}

/// 0.75 (`0x8012cc`): the length of a debug normal.
pub const NORMAL_LEN: f32 = 0.75;

/// `0x6b5f90` from `0x6b6098` to `0x6b60f5`: a debug normal's end `pos + n·0.75`; the x and y
/// products stay in the register, the z product is stored as `f32` first; each sum is stored as
/// `f32`.
pub fn cmapobjgroup_normal_endpoint(pos: [f32; 3], n: [f32; 3]) -> [f32; 3] {
    let c = f64::from(NORMAL_LEN);
    [
        (f64::from(pos[0]) + f64::from(n[0]) * c) as f32,
        (f64::from(pos[1]) + f64::from(n[1]) * c) as f32,
        pos[2] + n[2] * NORMAL_LEN,
    ]
}

/// `0x6a2600` from `0x6a2652` to `0x6a26a4`: the hit `p0 + (p1 − p0)·t`. The x product is stored
/// as `f32` before its sum; y stays in the register; the z difference is stored as `f32` before its
/// product. Each result is stored as `f32`.
pub fn segpt_interp(p0: [f32; 3], p1: [f32; 3], t: f32) -> [f32; 3] {
    let t = f64::from(t);
    let x = ((f64::from(p1[0]) - f64::from(p0[0])) * t) as f32 + p0[0];
    let y = ((f64::from(p1[1]) - f64::from(p0[1])) * t + f64::from(p0[1])) as f32;
    let z = (f64::from(p1[2] - p0[2]) * t + f64::from(p0[2])) as f32;
    [x, y, z]
}

/// `0x6aa1f0` from `0x6aa3a1` to `0x6aa3a6`: the plane offset `DotVector3` (`0x602630`) leaves in
/// `st0`, negated and stored as `f32`.
pub fn surface_plane_negate(d: f32) -> f32 {
    -d
}

/// `0x6be250` from `0x6be421` to `0x6be429`: a candidate hit less the best, kept in the register
/// for the compare with the margin at `0x80c4f4`.
pub fn nearest_param_delta(cand: f32, best: f32) -> f64 {
    f64::from(cand) - f64::from(best)
}
