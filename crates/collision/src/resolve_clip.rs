//! The sweep geometry (`systems/collision/contact.md`): the polygon clip, the volume's planes,
//! vertices and faces, the face prisms, the contact distance, the facet gather, the hover drop and
//! the earliest-contact sweep, and the crease helpers
//! (`systems/collision/airborne.md#crease-helpers`). The world query is passed in as `TraceWorld`.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::resolve_toolkit::Plane;

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80dfec]`: 1/720.
const SKIN: f32 = f32::from_bits(0x3ab6_0b61);
/// `[0x80dff0]`: −1/720.
const NEG_SKIN: f32 = f32::from_bits(0xbab6_0b61);

/// A clip polygon: up to 15 vertices, the plane index each came from, and the count; 0xf4 bytes.
#[derive(Clone)]
pub struct ClipPoly {
    pub verts: [[f32; 3]; 15],
    pub attrs: [i32; 15],
    pub count: u32,
}

impl Default for ClipPoly {
    fn default() -> Self {
        ClipPoly {
            verts: [[0.0; 3]; 15],
            attrs: [0; 15],
            count: 0,
        }
    }
}

/// `ClipPolygonByPlane` (`0x6318c0`, `systems/collision/contact.md#polygon-clip`): clips a polygon
/// to the inside of a plane with a 1/720 band, tagging the vertices it adds.
pub fn poly_plane_clip(poly: &mut ClipPoly, plane: &Plane, provenance: i32) {
    let n = poly.count as usize;
    if n == 0 {
        return;
    }

    let mut s = [0f32; 15];
    let mut maxd = f32::from_bits(0xff7f_ffff);
    let mut mind = f32::from_bits(0x7f7f_ffff);
    for (i, si) in s.iter_mut().enumerate().take(n) {
        let v = &poly.verts[i];
        *si = (-((((f(v[0]) * f(plane.n[0])) + f(v[2]) * f(plane.n[2])) + f(v[1]) * f(plane.n[1]))
            + f(plane.d))) as f32;
        if f(*si) > f(maxd) {
            maxd = *si;
        }
        if f(*si) < f(mind) {
            mind = *si;
        }
    }

    if f(mind) > f(NEG_SKIN) {
        return;
    }
    if f(maxd) < f(SKIN) {
        poly.count = 0;
        return;
    }

    let old_v = poly.verts;
    let old_a = poly.attrs;
    let mut out = 0usize;
    let emit_crossing =
        |out: &mut usize, poly: &mut ClipPoly, pi: usize, ci: usize, sp: f32, sc: f32| {
            let t = (f(sp) / (f(sc) - f(sp))) as f32;
            let (pv, cv) = (&old_v[pi], &old_v[ci]);
            let ex_t = ((f(cv[0]) - f(pv[0])) * f(t)) as f32;
            let ey_t = (f(cv[1]) - f(pv[1])) * f(t);
            let ez32 = (f(cv[2]) - f(pv[2])) as f32;
            let ez_t = f(ez32) * f(t);
            poly.verts[*out] = [
                (f(pv[0]) - f(ex_t)) as f32,
                (f(pv[1]) - ey_t) as f32,
                (f(pv[2]) - ez_t) as f32,
            ];
            poly.attrs[*out] = provenance;
            *out += 1;
        };
    let mut prev = n - 1;
    for cur in 0..n {
        let (sp, sc) = (s[prev], s[cur]);
        let p_in = !(f(sp) < 0.0);
        let c_in = !(f(sc) < 0.0);
        match (p_in, c_in) {
            (false, false) => {}
            (false, true) => {
                if f(sc) > f(SKIN) {
                    emit_crossing(&mut out, poly, prev, cur, sp, sc);
                }
                poly.verts[out] = old_v[cur];
                poly.attrs[out] = old_a[cur];
                out += 1;
            }
            (true, true) => {
                poly.verts[out] = old_v[cur];
                poly.attrs[out] = old_a[cur];
                out += 1;
            }
            (true, false) => {
                if f(sp) > f(SKIN) {
                    let t = (f(sp) / (f(sc) - f(sp))) as f32;
                    let (pv, cv) = (&old_v[prev], &old_v[cur]);
                    let te: [f32; 3] =
                        core::array::from_fn(|k| ((f(cv[k]) - f(pv[k])) * f(t)) as f32);
                    poly.verts[out] = core::array::from_fn(|k| (f(pv[k]) - f(te[k])) as f32);
                    poly.attrs[out] = provenance;
                    out += 1;
                }
            }
        }
        prev = cur;
    }
    poly.count = if out < 3 { 0 } else { out as u32 };
}

/// `ClipPolygonByPlanesExcept` (`0x632980`, `systems/collision/contact.md#polygon-clip`): clips a
/// polygon by every plane but one, returning 0 once it is empty.
pub fn clip_confirm(poly: &mut ClipPoly, planes: &[Plane], self_idx: usize) -> bool {
    for (i, pl) in planes.iter().enumerate() {
        if i == self_idx {
            continue;
        }
        poly_plane_clip(poly, pl, i as i32);
        if poly.count == 0 {
            return false;
        }
    }
    true
}

/// `CMovement::BuildVolume` (`0x631be0`, `systems/collision/contact.md#volume-vertices-and-faces`):
/// writes the volume's nine planes, nine vertices and 32 face index bytes at a position.
pub fn swept_prism_verts(pos: &[f32; 3], height: f32, radius: f32) -> [[f32; 3]; 9] {
    const R_SCALE: f32 = f32::from_bits(0x3fec_b91b);
    let (x, y, z) = (f(pos[0]), f(pos[1]), f(pos[2]));
    let r = f(radius);
    let rp = r * f(R_SCALE);
    let h = f(height);
    [
        *pos,
        [(x - r) as f32, (y - r) as f32, (z + rp) as f32],
        [(x - r) as f32, (y + r) as f32, (z + rp) as f32],
        [(x + r) as f32, (y + r) as f32, (z + rp) as f32],
        [(x + r) as f32, (y - r) as f32, (z + rp) as f32],
        [(x - r) as f32, (y - r) as f32, (z + h) as f32],
        [(x - r) as f32, (y + r) as f32, (z + h) as f32],
        [(x + r) as f32, (y + r) as f32, (z + h) as f32],
        [(x + r) as f32, (y - r) as f32, (z + h) as f32],
    ]
}

/// The volume's face table `0x631be0` reads: the vertex indices of each face.
pub const PRISM_FACE_TABLE: [u8; 0x20] = [
    0x01, 0x02, 0x06, 0x05, 0x03, 0x04, 0x08, 0x07, 0x02, 0x03, 0x07, 0x06, 0x04, 0x01, 0x05, 0x08,
    0x05, 0x06, 0x07, 0x08, 0x00, 0x01, 0x02, 0x00, 0x03, 0x04, 0x00, 0x02, 0x03, 0x00, 0x04, 0x01,
];

/// `[0x80dff4]`: −2^-20.
const NEG_TINY: f64 = -9.536_743e-7_f32 as f64;
/// `[0x7ff9c8]`: −1/36.
const NEG_36TH: f64 = f32::from_bits(0xbce3_8e39) as f64;

/// `PolygonContactDistance` (`0x632830`, `systems/collision/contact.md#contact-distance`): measures
/// the travel until a volume face reaches a polygon: 0 when touching, none past 1/36 behind.
pub fn polygon_toi(
    poly: &mut ClipPoly,
    dir: &[f32; 3],
    planes: &[Plane],
    self_idx: usize,
    out_t: &mut f32,
) -> bool {
    use crate::resolve_toolkit::ray_plane_t;
    let mut tbest = *out_t;
    let mut flag = true;
    for i in 0..poly.count as usize {
        let t = ray_plane_t(&poly.verts[i], &planes[self_idx], dir);
        if t < f(tbest) {
            tbest = if t > 0.0 { t as f32 } else { 0.0 };
        }

        if !(t <= NEG_TINY) && !t.is_nan() {
            flag = false;
        }
    }
    if flag {
        if !clip_confirm(poly, planes, self_idx) {
            return false;
        }
        for i in 0..poly.count as usize {
            let t = ray_plane_t(&poly.verts[i], &planes[self_idx], dir);
            if !(t <= NEG_36TH) && !t.is_nan() {
                flag = false;
            }
        }
        if flag {
            return false;
        }
    }

    if f(tbest) < f(*out_t) {
        *out_t = tbest;
        return true;
    }
    false
}

/// `[0x80e004]`: 1/12.
const TWELFTH: f32 = f32::from_bits(0x3daa_aaab);

/// `[0x8026bc]`: 2^-20.
const DEGEN: f64 = 9.536_743e-7_f32 as f64;

/// `BuildTrianglePrism` (`0x632460`, `systems/collision/contact.md#face-prisms`): writes the edge
/// planes of a triangle swept along a vector, and its far cap; 0 for a degenerate edge.
fn tri_edge_planes(verts: &[[f32; 3]; 3], extrude: &[f32; 3]) -> ([Plane; 4], bool) {
    use crate::resolve_toolkit::plane_from_points;
    const ROT1: [usize; 3] = [1, 2, 0];
    const ROT2: [usize; 3] = [2, 0, 1];

    let mut out = [Plane {
        n: [0.0, 0.0, 1.0],
        d: 0.0,
    }; 4];
    for i in 0..3 {
        let vc = &verts[i];
        let vr = &verts[ROT1[i]];
        let sum: [f32; 3] = core::array::from_fn(|k| (f(vc[k]) + f(extrude[k])) as f32);
        let e1: [f32; 3] = core::array::from_fn(|k| (f(sum[k]) - f(vc[k])) as f32);
        let e2: [f32; 3] = core::array::from_fn(|k| (f(vr[k]) - f(vc[k])) as f32);
        let n = [
            (f(e1[1]) * f(e2[0]) - f(e2[1]) * f(e1[0])) as f32,
            (f(e1[2]) * f(e2[1]) - f(e2[2]) * f(e1[1])) as f32,
            (f(e1[0]) * f(e2[2]) - f(e2[0]) * f(e1[2])) as f32,
        ];
        let n2 = ((f(n[0]) * f(n[0]) + f(n[1]) * f(n[1])) + f(n[2]) * f(n[2])).abs();
        if n2 <= DEGEN {
            return (out, false);
        }
        let mut pl = plane_from_points(vc, vr, &sum);
        let vo = &verts[ROT2[i]];
        let t: [f32; 3] = core::array::from_fn(|k| (f(vo[k]) - f(vc[k])) as f32);
        let dot = (f(t[2]) * f(pl.n[2]) + f(t[1]) * f(pl.n[1])) + f(t[0]) * f(pl.n[0]);

        if dot > 0.0 {
            pl.n = [-pl.n[0], -pl.n[1], -pl.n[2]];
            pl.d = -pl.d;
        }
        out[i] = pl;
    }

    (out, true)
}

/// The cap plane `BuildTrianglePrism` (`0x632460`) and `BuildQuadPrism` (`0x632f80`) both write:
/// the reference normal, and `d = −((n.z·s.z + n.y·s.y) + n.x·s.x)` with `s = v0 + extrude`.
fn face_plane(v0: &[f32; 3], extrude: &[f32; 3], ref_n: &[f32; 3]) -> Plane {
    let s: [f32; 3] = core::array::from_fn(|k| (f(v0[k]) + f(extrude[k])) as f32);
    let d = (-((f(ref_n[2]) * f(s[2]) + f(ref_n[1]) * f(s[1])) + f(ref_n[0]) * f(s[0]))) as f32;
    Plane { n: *ref_n, d }
}

/// `IsPointOverListedTriangle` (`0x6335d0`, `systems/collision/airborne.md#triangle-class-tests`):
/// returns 1 when a point lies at most 1/12 outside three planes 0x632460 builds for a listed
/// triangle.
pub fn point_in_tri(basis: &[[f32; 3]; 3], p: &[f32; 3]) -> bool {
    let (planes, _) = tri_edge_planes(basis, &[0.0, 0.0, 1.0]);
    for pl in &planes[..3] {
        let sd = ((f(pl.n[0]) * f(p[0]) + f(pl.n[2]) * f(p[2])) + f(pl.n[1]) * f(p[1])) + f(pl.d);
        if sd > f(TWELFTH) {
            return false;
        }
    }
    true
}

/// `ClipPolygonByPlanes` (`0x631870`, `systems/collision/contact.md#polygon-clip`): clips a polygon
/// by each plane in turn, returning 0 once it is empty.
pub fn multi_plane_clip(poly: &mut ClipPoly, planes: &[Plane]) -> bool {
    for (i, pl) in planes.iter().enumerate() {
        poly_plane_clip(poly, pl, i as i32);
        if poly.count == 0 {
            return false;
        }
    }
    true
}

/// `IntersectThreePlanes` (`0x634fc0`, `systems/collision/airborne.md#crease-helpers`): writes the
/// point common to three planes through a 3x3 inverse.
pub fn three_plane_point(a: &Plane, b: &Plane, c: &Plane) -> [f32; 3] {
    use benilla_re_cmath::matrix3::{det3b, inv3};
    let m: [f32; 9] = [
        a.n[0], a.n[1], a.n[2], b.n[0], b.n[1], b.n[2], c.n[0], c.n[1], c.n[2],
    ];
    let bv = [-a.d, -b.d, -c.d];
    let det = det3b(&m);
    let im = inv3(&m, det);
    core::array::from_fn(|r| {
        ((f(im[r * 3 + 2]) * f(bv[2]) + f(im[r * 3 + 1]) * f(bv[1])) + f(im[r * 3]) * f(bv[0]))
            as f32
    })
}

/// `PointToLineDistanceSquared` (`0x634f40`, `systems/collision/airborne.md#crease-helpers`):
/// returns the squared distance from a point to the line through an origin along a unit direction.
pub fn ray_point_dist_sq(p: &[f32; 3], q: &[f32; 3], dir: &[f32; 3]) -> f64 {
    let v: [f64; 3] = core::array::from_fn(|k| f(p[k]) - f(q[k]));
    let proj = (v[2] * f(dir[2]) + v[1] * f(dir[1])) + v[0] * f(dir[0]);

    let px = proj * f(dir[0]);
    let py = (proj * f(dir[1])) as f32;
    let pz = (proj * f(dir[2])) as f32;
    let cx = px + f(q[0]);
    let cy = f(py) + f(q[1]);
    let cz = (f(pz) + f(q[2])) as f32;
    let ex = (f(p[0]) - cx) as f32;
    let ey = f(p[1]) - cy;
    let ez = f(p[2]) - f(cz);

    (ez * ez + ey * ey) + f(ex) * f(ex)
}

/// `[0x8029d4]`: 2^-22.
const LEN_GUARD: f64 = 2.384_185_8e-7_f32 as f64;

/// `[0x8026bc]`: 2^-20.
const PAR_GUARD: f64 = 9.536_743e-7_f32 as f64;

/// `FindNearestTriangleEdge` (`0x634da0`, `systems/collision/airborne.md#crease-helpers`): returns
/// the unit edge of a listed triangle nearest the line through a point along a crease.
pub fn closest_tri_edge(basis: &[[f32; 3]; 3], point: &[f32; 3], n: &[f32; 3]) -> [f32; 3] {
    const ROT: [usize; 3] = [1, 2, 0];
    let mut best = f32::from_bits(0x7f7f_ffff);
    let mut acc = [0f32; 3];
    for i in 0..3 {
        let e: [f64; 3] = core::array::from_fn(|k| f(basis[i][k]) - f(basis[ROT[i]][k]));
        let len = (e[0] * e[0] + e[1] * e[1] + e[2] * e[2]).sqrt();
        if len.abs() < LEN_GUARD {
            continue;
        }
        let inv = 1.0 / len;
        let eh: [f32; 3] = core::array::from_fn(|k| (e[k] * inv) as f32);
        let dot1 = (f(eh[2]) * f(n[2]) + f(eh[1]) * f(n[1])) + f(eh[0]) * f(n[0]);
        let metric = if (dot1 - 1.0).abs() < PAR_GUARD {
            ray_point_dist_sq(&basis[i], point, n)
        } else {
            let c = [
                (f(eh[1]) * f(n[2]) - f(eh[2]) * f(n[1])) as f32,
                (f(eh[2]) * f(n[0]) - f(eh[0]) * f(n[2])) as f32,
                (f(eh[0]) * f(n[1]) - f(eh[1]) * f(n[0])) as f32,
            ];
            let s = -((f(c[2]) * f(point[2]) + f(c[1]) * f(point[1])) + f(c[0]) * f(point[0]));
            let v = s
                + ((f(c[2]) * f(basis[i][2]) + f(c[1]) * f(basis[i][1]))
                    + f(c[0]) * f(basis[i][0]));
            v * v
        };

        if metric < f(best) {
            best = metric as f32;
            acc = eh;
        }
    }
    acc
}

/// `CMovement::ComputeWallNormal` (`0x634ae0`,
/// `systems/collision/airborne.md#wall-normal-from-contacts`): picks a hit's wall normal from its
/// contact planes: the triangle's, the first plane's, or a crease's.
#[allow(clippy::too_many_arguments)]
pub fn slide_normal_select(
    pos: &[f32; 3],
    radius: f32,
    hit_normal: &[f32; 3],
    hit_d: f32,
    hit_basis: &[[f32; 3]; 3],
    feature: &[Plane; 2],
    feature_type: u32,
) -> [f32; 3] {
    let e0 = &feature[0].n;
    match feature_type {
        3 | 4 => *hit_normal,
        2 => {
            let e1 = &feature[1].n;
            let c0 = [
                (f(e0[1]) * f(e1[2]) - f(e0[2]) * f(e1[1])) as f32,
                (f(e0[2]) * f(e1[0]) - f(e0[0]) * f(e1[2])) as f32,
                (f(e0[0]) * f(e1[1]) - f(e0[1]) * f(e1[0])) as f32,
            ];
            let len = ((f(c0[0]) * f(c0[0]) + f(c0[1]) * f(c0[1])) + f(c0[2]) * f(c0[2])).sqrt();
            let inv = 1.0 / len;
            let cf64: [f64; 3] = core::array::from_fn(|k| f(c0[k]) * inv);
            let c: [f32; 3] = core::array::from_fn(|k| cf64[k] as f32);

            if cf64[2].abs() < PAR_GUARD {
                return *hit_normal;
            }

            let dcn = (f(c[2]) * f(hit_normal[2]) + f(c[1]) * f(hit_normal[1]))
                + f(c[0]) * f(hit_normal[0]);
            if dcn.abs() < PAR_GUARD {
                return *hit_normal;
            }

            if (f(c[2]) - 1.0).abs() >= PAR_GUARD {
                use crate::resolve_state::prism_lower_flush_test;
                if prism_lower_flush_test(pos, radius, hit_normal, hit_d) {
                    return *hit_normal;
                }
            }

            let hit_plane = Plane {
                n: *hit_normal,
                d: hit_d,
            };
            let corner = three_plane_point(&hit_plane, &feature[0], &feature[1]);
            let a2 = closest_tri_edge(hit_basis, &corner, &c);
            let m0 = [
                (f(a2[1]) * f(c[2]) - f(a2[2]) * f(c[1])) as f32,
                (f(a2[2]) * f(c[0]) - f(a2[0]) * f(c[2])) as f32,
                (f(a2[0]) * f(c[1]) - f(a2[1]) * f(c[0])) as f32,
            ];
            let ml = ((f(m0[0]) * f(m0[0]) + f(m0[1]) * f(m0[1])) + f(m0[2]) * f(m0[2])).sqrt();
            if ml > LEN_GUARD {
                let mi = 1.0 / ml;
                let m: [f32; 3] = core::array::from_fn(|k| (f(m0[k]) * mi) as f32);
                let d = (f(m[2]) * f(e0[2]) + f(m[1]) * f(e0[1])) + f(m[0]) * f(e0[0]);
                if d > 0.0 {
                    [-m[0], -m[1], -m[2]]
                } else {
                    m
                }
            } else {
                [-e0[0], -e0[1], -e0[2]]
            }
        }
        _ => [-e0[0], -e0[1], -e0[2]],
    }
}

/// `[0x80c5c4]`: −1e-5.
const NEG_1E5: f64 = f32::from_bits(0xb727_c5ac) as f64;

/// A listed triangle: its normal, plane `d` and three vertices, the 0x34-byte record.
#[derive(Clone, Copy, Default)]
pub struct HitRecord {
    pub normal: [f32; 3],
    pub d: f32,
    pub basis: [[f32; 3]; 3],
}

/// The edge planes and the cap of `BuildTrianglePrism` (`0x632460`,
/// `systems/collision/contact.md#face-prisms`), as its test compares them.
pub fn test_tri_edge_planes(
    verts: &[[f32; 3]; 3],
    extrude: &[f32; 3],
    ref_n: &[f32; 3],
) -> ([Plane; 4], bool) {
    let (mut p, ok) = tri_edge_planes(verts, extrude);
    if ok {
        p[3] = face_plane(&verts[0], extrude, ref_n);
    }
    (p, ok)
}

/// `FindNearestFacet` (`0x632700`, `systems/collision/contact.md#nearest-facet-per-face`): finds
/// the facet, among those facing against a direction, that a volume face reaches first.
pub fn per_face_toi(
    hits: &[HitRecord],
    dir: &[f32; 3],
    clip_planes: &[Plane],
    prism_planes: &[Plane],
    self_idx: usize,
    best: &mut f32,
    rec_idx: &mut u32,
) -> bool {
    let mut committed = false;
    for (i, rec) in hits.iter().enumerate() {
        let dot = (f(rec.normal[2]) * f(dir[2]) + f(rec.normal[1]) * f(dir[1]))
            + f(rec.normal[0]) * f(dir[0]);

        if dot > NEG_1E5 {
            continue;
        }
        let mut poly = ClipPoly::default();
        poly.verts[..3].copy_from_slice(&rec.basis);
        poly.attrs[..3].copy_from_slice(&[-1, -1, -1]);
        poly.count = 3;
        if !multi_plane_clip(&mut poly, clip_planes) {
            continue;
        }
        let mut t = f32::from_bits(0x7f7f_ffff);
        if !polygon_toi(&mut poly, dir, prism_planes, self_idx, &mut t) {
            continue;
        }

        if !(f(t) > f(*best)) && !f(t).is_nan() {
            *best = t;
            *rec_idx = i as u32;
            committed = true;
        }
    }
    committed
}

/// `CMovement::FindBevelContacts` (`0x632280`,
/// `systems/collision/contact.md#bevel-contacts-and-the-band`): collects the leading bevel faces
/// that reach a facet first, within 1/720 of the nearest.
#[allow(clippy::too_many_arguments)]
pub fn candidate_collect(
    hits: &[HitRecord],
    dir: &[f32; 3],
    disp: &[f32; 3],
    t_span: f32,
    prism_planes: &[Plane; 9],
    prism_verts: &[[f32; 3]; 9],
    face_table: &[u8; 0x20],
    manifold: &mut [Plane; 5],
    count: &mut u32,
    best: &mut f32,
    rec_idx: &mut u32,
) {
    const SKIN_B: f32 = f32::from_bits(0x3ab6_0b61);
    for b in 0..4usize {
        let bevel = &prism_planes[5 + b];

        let dot =
            (f(bevel.n[2]) * f(disp[2]) + f(bevel.n[0]) * f(disp[0])) + f(bevel.n[1]) * f(disp[1]);

        if !(dot > 0.0) && !dot.is_nan() {
            continue;
        }
        let tri_idx = &face_table[0x14 + b * 3..0x14 + b * 3 + 3];
        let tri: [[f32; 3]; 3] = core::array::from_fn(|k| prism_verts[tri_idx[k] as usize]);
        let (mut clip, ok) = tri_edge_planes(&tri, disp);
        if !ok {
            continue;
        }
        clip[3] = face_plane(&tri[0], disp, &bevel.n);

        let mut t = t_span;
        if !per_face_toi(
            hits,
            dir,
            &clip[..3],
            &prism_planes[..],
            5 + b,
            &mut t,
            rec_idx,
        ) {
            continue;
        }

        if f(t) < f(*best) - f(SKIN_B) {
            manifold[0] = *bevel;
            *count = 1;
        } else if f(t) < f(*best) + f(SKIN_B) {
            manifold[*count as usize] = *bevel;
            *count += 1;
        }
        if f(t) < f(*best) {
            *best = t;
            if *count > 1 {
                manifold.swap(*count as usize - 1, 0);
            }
        }
    }
}

/// 1/6 (`0x3e2aaaab`), the box's growth.
const SIXTH: f32 = f32::from_bits(0x3e2a_aaab);

/// The world query `QueryWorldTriangles` (`0x6721b0`,
/// `systems/terrain/collide.md#world-triangle-query`) as the caller passes it.
pub struct TraceWorld {
    pub cache_box: crate::resolve_toolkit::Aabb,
    pub solid: Vec<HitRecord>,
    pub liquid: Vec<HitRecord>,
    pub trace_ok: bool,
}

/// `CMovement::GatherSweepFacets` (`0x631e70`, `systems/collision/contact.md#facet-gather`): fills
/// the facet lists around a move unless the cached box holds it; returns 0 when the query fails.
#[allow(clippy::too_many_arguments)]
pub fn static_overlap(
    pos: &[f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    disp: &[f32; 3],
    transport: Option<&[[f32; 3]; 4]>,
    mask_inp: &crate::resolve_state::MaskSelectInput,
    world: &mut TraceWorld,
) -> bool {
    use crate::resolve_query::{affine_xform, rot_dir};
    use crate::resolve_toolkit::{
        aabb_union, point_in_aabb, vec3_add_scalar, vec3_sub_scalar, Aabb,
    };

    let (tpos, tdisp);
    let (pos, disp): (&[f32; 3], &[f32; 3]) = match transport {
        Some(m) => {
            tpos = affine_xform(m, pos);
            tdisp = rot_dir(m, disp);
            (&tpos, &tdisp)
        }
        None => (pos, disp),
    };
    let newpos: [f32; 3] = core::array::from_fn(|k| (f(pos[k]) + f(disp[k])) as f32);
    let (r, h) = (f(radius), f(height));
    let aabb = Aabb {
        min: [
            (f(newpos[0]) - r) as f32,
            (f(newpos[1]) - r) as f32,
            newpos[2],
        ],
        max: [
            (f(newpos[0]) + r) as f32,
            (f(newpos[1]) + r) as f32,
            (f(newpos[2]) + h) as f32,
        ],
    };
    if point_in_aabb(&world.cache_box, &aabb.min) && point_in_aabb(&world.cache_box, &aabb.max) {
        return true;
    }
    let mut grown = aabb;
    vec3_sub_scalar(&mut grown.min, SIXTH);
    vec3_add_scalar(&mut grown.max, SIXTH);
    let unioned = aabb_union(&grown, &world.cache_box);
    let _mask = crate::resolve_state::query_mask_select(mask_inp);
    if !world.trace_ok {
        return false;
    }
    if flags & 0x0020_0000 != 0 {
        for rec in world.liquid.iter_mut() {
            rec.d = -rec.d;
            rec.normal = [-rec.normal[0], -rec.normal[1], -rec.normal[2]];
        }
    }
    world.cache_box = unioned;
    true
}

/// `CMovement::MeasureHoverDrop` (`0x632a30`, `systems/collision/contact.md#hover-drop`): measures
/// how far the volume can drop straight down within the drop passed in, 0 below 1/720.
#[allow(clippy::too_many_arguments)]
pub fn step_up_probe(
    this_pos: &[f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    mask_inp: &crate::resolve_state::MaskSelectInput,
    pos_override: Option<&[f32; 3]>,
    hits: &[HitRecord],
    world: &mut TraceWorld,
    face_table: &[u8; 0x20],
    best: &mut f32,
) -> bool {
    const SKIN_C: f32 = f32::from_bits(0x3ab6_0b61);
    let pos = pos_override.unwrap_or(this_pos);
    let planes = crate::resolve_toolkit::capsule_kdop_planes(pos, height, radius);
    let verts = swept_prism_verts(pos, height, radius);
    let down = [0.0f32, 0.0, -1.0];
    if pos_override.is_none()
        && !static_overlap(
            this_pos, radius, height, flags, &down, None, mask_inp, world,
        )
    {
        *best = 0.0;
        return false;
    }
    let mut manifold = [Plane::default(); 5];
    let mut count = 0u32;
    let mut ridx_sink = 0u32;
    candidate_collect(
        hits,
        &down,
        &down,
        1.0,
        &planes,
        &verts,
        face_table,
        &mut manifold,
        &mut count,
        best,
        &mut ridx_sink,
    );

    if f(*best) < f(SKIN_C) {
        *best = 0.0;
    }
    true
}

/// `BuildQuadPrism` (`0x632f80`, `systems/collision/contact.md#face-prisms`): writes the edge
/// planes of a quad swept along a vector, and its far cap; 0 for a degenerate edge.
fn quad_edge_planes(verts: &[[f32; 3]; 9], idx: &[u8], extrude: &[f32; 3]) -> ([Plane; 5], bool) {
    use crate::resolve_toolkit::plane_from_points;
    let mut out = [Plane {
        n: [0.0, 0.0, 1.0],
        d: 0.0,
    }; 5];
    for i in 0..4usize {
        let vc = &verts[idx[i] as usize];
        let vr = &verts[idx[(i + 1) & 3] as usize];
        let sum: [f32; 3] = core::array::from_fn(|k| (f(vc[k]) + f(extrude[k])) as f32);
        let e1: [f32; 3] = core::array::from_fn(|k| (f(sum[k]) - f(vc[k])) as f32);
        let e2: [f32; 3] = core::array::from_fn(|k| (f(vr[k]) - f(vc[k])) as f32);
        let n = [
            (f(e1[1]) * f(e2[0]) - f(e2[1]) * f(e1[0])) as f32,
            (f(e1[2]) * f(e2[1]) - f(e2[2]) * f(e1[1])) as f32,
            (f(e1[0]) * f(e2[2]) - f(e2[0]) * f(e1[2])) as f32,
        ];
        let n2 = ((f(n[0]) * f(n[0]) + f(n[1]) * f(n[1])) + f(n[2]) * f(n[2])).abs();
        if n2 <= DEGEN {
            return (out, false);
        }
        let mut pl = plane_from_points(vc, vr, &sum);
        let vo = &verts[idx[(i + 2) & 3] as usize];
        let t: [f32; 3] = core::array::from_fn(|k| (f(vo[k]) - f(vc[k])) as f32);
        let dot = (f(t[2]) * f(pl.n[2]) + f(t[1]) * f(pl.n[1])) + f(t[0]) * f(pl.n[0]);
        if dot > 0.0 {
            pl.n = [-pl.n[0], -pl.n[1], -pl.n[2]];
            pl.d = -pl.d;
        }
        out[i] = pl;
    }
    (out, true)
}

/// `CMovement::SweepVolume` (`0x632ba0`, `systems/collision/contact.md#earliest-contact-sweep`):
/// returns the travel of the volume along a direction to its first contact, and the leading faces
/// that touch.
#[allow(clippy::too_many_arguments)]
pub fn earliest_contact(
    pos: &[f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    liquid_list: bool,
    dir: &[f32; 3],
    dist: f32,
    mask_inp: &crate::resolve_state::MaskSelectInput,
    hits: &[HitRecord],
    world: &mut TraceWorld,
    face_table: &[u8; 0x20],
    manifold: &mut [Plane; 5],
    count: &mut u32,
    out_t: &mut f32,
    rec_idx: &mut u32,
) -> bool {
    const SKIN_D: f32 = f32::from_bits(0x3ab6_0b61);
    const THIRTY_SIXTH: f32 = f32::from_bits(0x3ce3_8e39);
    const THREE_Q: f32 = 0.75;
    if f(dist).abs() < DEGEN {
        *rec_idx = hits.len() as u32;
        *out_t = 0.0;
        return true;
    }
    let mut h = height;
    if flags & 0x0020_0000 != 0 && liquid_list {
        h = (f(h) * f(THREE_Q)) as f32;
    }

    let d = if !(f(dist) > f(THIRTY_SIXTH)) {
        THIRTY_SIXTH
    } else {
        dist
    };
    let disp: [f32; 3] = core::array::from_fn(|k| (f(d) * f(dir[k])) as f32);
    let planes = crate::resolve_toolkit::capsule_kdop_planes(pos, h, radius);
    let verts = swept_prism_verts(pos, h, radius);

    if !static_overlap(pos, radius, h, flags, &disp, None, mask_inp, world) {
        *out_t = 0.0;
        return false;
    }

    *rec_idx = hits.len() as u32;
    let mut best = d;
    candidate_collect(
        hits, dir, &disp, d, &planes, &verts, face_table, manifold, count, &mut best, rec_idx,
    );

    for a in 0..5usize {
        let axis = &planes[a];
        let dot =
            (f(disp[2]) * f(axis.n[2]) + f(disp[1]) * f(axis.n[1])) + f(disp[0]) * f(axis.n[0]);
        if !(dot > 0.0) && !dot.is_nan() {
            continue;
        }
        let quad = &face_table[a * 4..a * 4 + 4];
        let (clip, ok) = quad_edge_planes(&verts, quad, &disp);
        if !ok {
            continue;
        }

        let mut t = d;
        if !per_face_toi(hits, dir, &clip[..4], &planes[..], a, &mut t, rec_idx) {
            continue;
        }
        if f(t) < f(best) - f(SKIN_D) {
            manifold[0] = *axis;
            *count = 1;
        } else if f(t) < f(best) + f(SKIN_D) {
            manifold[*count as usize] = *axis;
            *count += 1;
        }
        if f(t) < f(best) {
            best = t;
            if *count > 1 {
                manifold.swap(*count as usize - 1, 0);
            }
        }
    }
    *out_t = if f(best) < f(SKIN_D) { 0.0 } else { best };
    true
}
