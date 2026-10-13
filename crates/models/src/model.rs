//! A model instance (`systems/models/model.md`): the span copies of `CM2Model::InitializeLoaded`
//! (`0x70ebd0`), the base transform of `CM2Model::SetBaseTransformConformed` (`0x7106c0`) and
//! `M2ConformRows` (`0x710a80`), the key bone's frame of `CM2Model::GetKeyBoneAnimState`
//! (`0x711fe0`), the attachment arithmetic of `0x711a20` and `CM2Model::GetAttachmentMatrix`
//! (`0x712de0`), the bounding sphere `CM2Model::GetBoundingSphere` (`0x713680`) and the facet frame
//! of `CM2Model::GatherCollisionFacets` (`0x7137c0`).

use crate::x87::ftol;

/// 2.3841858e-7 (`0x8029d4`): the least length a vector is normalized at.
const NORM_EPS: f32 = f32::from_bits(0x3480_0000);

/// `0x70ebd0` from `0x70ff3f` to `0x70ff6f` (`systems/models/model.md#instance-arrays`): the
/// record's `+0x188` and `+0x18c` copied and their difference stored as `f32`.
pub fn part_span_block1(src_188: f32, src_18c: f32) -> (f32, f32, f32) {
    (src_188, src_18c, src_18c - src_188)
}

/// `0x70ebd0` from `0x70ff7d` to `0x70ff99`: `+0x19c` copied and `+0x1a8 − +0x19c`.
pub fn part_span_block2(src_19c: f32, src_1a8: f32) -> (f32, f32) {
    (src_19c, src_1a8 - src_19c)
}

/// `0x70ebd0` from `0x70ff9b` to `0x70ffb7`: `+0x1a0` copied and `+0x1ac − +0x1a0`.
pub fn part_span_block3(src_1a0: f32, src_1ac: f32) -> (f32, f32) {
    (src_1a0, src_1ac - src_1a0)
}

/// `0x70ebd0` from `0x70ffb9` to `0x70ffd5`: `+0x1a4` copied and `+0x1b0 − +0x1a4`.
pub fn part_span_block4(src_1a4: f32, src_1b0: f32) -> (f32, f32) {
    (src_1a4, src_1b0 - src_1a4)
}

/// `(x² + y²) + z²` as `0x4549f0` leaves it.
fn len2(v: [f32; 3]) -> f64 {
    let [x, y, z] = v.map(f64::from);
    (x * x + y * y) + z * z
}

/// A stored vector normalized in place the way `0x7106c0`, `0x710a80` and `0x7137c0` do it:
/// unless its length is below 2.3841858e-7 (ordered, `test ah,5; jnp`, so a NaN length
/// normalizes), each component times `1/len` kept in the register.
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = len2(v).sqrt();
    if len.abs() < f64::from(NORM_EPS) {
        return v;
    }
    let inv = 1.0 / len;
    v.map(|c| (f64::from(c) * inv) as f32)
}

/// `a × b` as `0x7106c0` forms it, `(a1·b2 − a2·b1, a2·b0 − b2·a0, b1·a0 − a1·b0)`, each stored
/// as `f32`.
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    let [a0, a1, a2] = a.map(f64::from);
    let [b0, b1, b2] = b.map(f64::from);
    [
        (a1 * b2 - a2 * b1) as f32,
        (a2 * b0 - b2 * a0) as f32,
        (b1 * a0 - a1 * b0) as f32,
    ]
}

/// The rows [`billboard_tangent_frame`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TangentFrame {
    /// `+0xcc`.
    pub up: [f32; 3],
    /// `+0xbc`.
    pub right: [f32; 3],
    /// `+0xdc`.
    pub fwd: [f32; 3],
}

/// `0x7106c0` from `0x710769` to `0x7108a9`, header flag mode 1
/// (`systems/models/model.md#base-transform`): the rows rebuilt round the normal `dir`. `up = (z·0
/// − y, x − z·0, y·0 − x·0)` normalized; `right = up × n` normalized, with `n` the second
/// argument's vector; `fwd = right × up`.
pub fn billboard_tangent_frame(dir: [f32; 3], param_6: [f32; 3]) -> TangentFrame {
    let [a, b, c] = dir.map(f64::from);
    let up = normalize([
        (c * 0.0 - b) as f32,
        (a - c * 0.0) as f32,
        (b * 0.0 - a * 0.0) as f32,
    ]);
    let right = normalize(cross(up, param_6));
    let fwd = cross(right, up);
    TangentFrame { up, right, fwd }
}

/// `0x7106c0` at `0x710968` and `0x710971`: a 64-bit count over a `f32`, left in the register for
/// `__ftol`.
pub fn spline_param_fdiv(num: i64, denom: f32) -> f64 {
    num as f64 / f64::from(denom)
}

/// `0x7106c0` at `0x71098b` and `0x71098e`: `fild` of the truncated quotient, `fidiv` by a step.
pub fn spline_param_fidiv(iquot: i32, istep: i32) -> f64 {
    f64::from(iquot) / f64::from(istep)
}

/// `0x7106c0` from `0x7109db` to `0x7109e4` and from `0x710a05` to `0x710a17`: `1 − p` stored as
/// `f32`.
pub fn spline_complement(param: f32) -> f32 {
    1.0 - param
}

/// What [`m2_orthonormal_basis`] stores through `edx`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrthonormalBasis {
    /// `+0x0`, over the reference vector.
    pub up: [f32; 3],
    /// `+0x10`.
    pub right: [f32; 3],
    /// `+0x20`: `ecx`'s vector copied.
    pub fwd: [f32; 3],
}

/// `0x710a80` (`systems/models/model.md#base-transform`): `right = fwd × up` (`(u2·f1 − u1·f2,
/// u0·f2 − u2·f0, u1·f0 − f1·u0)`) stored and normalized (`jnp` at `0x710b00`, so a NaN length
/// normalizes); then `up' = right × fwd` (`(r1·f2 − f1·r2, f0·r2 − r0·f2, r0·f1 − f0·r1)`) over the
/// reference, and `fwd` copied. Each component is stored as `f32`.
pub fn m2_orthonormal_basis(fwd: [f32; 3], ref_up: [f32; 3]) -> OrthonormalBasis {
    let [ax, ay, az] = fwd.map(f64::from);
    let [bx, by, bz] = ref_up.map(f64::from);
    let right = normalize([
        (bz * ay - by * az) as f32,
        (bx * az - bz * ax) as f32,
        (by * ax - ay * bx) as f32,
    ]);
    let [rx, ry, rz] = right.map(f64::from);
    OrthonormalBasis {
        up: [
            (ry * az - ay * rz) as f32,
            (ax * rz - rx * az) as f32,
            (rx * ay - ax * ry) as f32,
        ],
        right,
        fwd,
    }
}

/// `0x711fe0` from `0x712041` to `0x712061` (`systems/models/model.md#base-transform`): the key
/// bone's frame, `__ftol((now − start)·rate) + base`, with `now` the instance's time, `start` the
/// bone's `+0xa8`, `rate` its `+0xb0` and `base` its `+0xb8`.
pub fn query_attach_event(curtime: i32, a8: i32, scale: f32, b8: i32) -> i32 {
    let delta = curtime.wrapping_sub(a8);
    ftol(f64::from(delta) * f64::from(scale)).wrapping_add(b8)
}

/// What [`attach_world_transform`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttachTransform {
    /// The record's scale times the length of the base rotation's first row.
    pub scale: f32,
    /// The translation the matrix call takes: `+0x30 + +0x24`, `+0x34 + +0x28`, `+0x38 + +0x2c`.
    pub off_x: f32,
    pub off_y: f32,
    pub off_z: f32,
}

/// `0x711a20` from `0x711ab6` to `0x711b17`: `√((c0² + c1²) + c2²)` of the instance's `+0xbc` row
/// (`c0` `+0xc0`, `c1` `+0xc4`, `c2` `+0xbc`) times the record's `+0xc`, in the register, stored as
/// `f32`; and the three sums of the record's translations, each stored as `f32`.
#[allow(clippy::too_many_arguments)]
pub fn attach_world_transform(
    c0: f32,
    c1: f32,
    c2: f32,
    rec_scale: f32,
    rec24: f32,
    rec28: f32,
    rec2c: f32,
    rec30: f32,
    rec34: f32,
    rec38: f32,
) -> AttachTransform {
    let [c0, c1, c2] = [c0, c1, c2].map(f64::from);
    let len = ((c0 * c0 + c1 * c1) + c2 * c2).sqrt();
    AttachTransform {
        scale: (len * f64::from(rec_scale)) as f32,
        off_x: rec30 + rec24,
        off_y: rec34 + rec28,
        off_z: rec38 + rec2c,
    }
}

/// `0x712de0` (`systems/models/model.md#attachments`): the attachment's offset `v` taken through
/// its bone matrix before the scene matrix: each translation row `((m[8+k]·v2 + m[4+k]·v1) +
/// m[k]·v0) + m[12+k]` in the register, stored as `f32`.
pub fn attach_fold_712de0(m: [f32; 16], v: [f32; 3]) -> [f32; 16] {
    let mut out = m;
    let [v0, v1, v2] = v.map(f64::from);
    for k in 0..3 {
        let dot = (f64::from(m[8 + k]) * v2 + f64::from(m[4 + k]) * v1) + f64::from(m[k]) * v0;
        out[12 + k] = (dot + f64::from(m[12 + k])) as f32;
    }
    out
}

/// What [`bbox_center`] returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBoxCenter {
    pub center: [f32; 3],
    pub radius: f32,
}

/// `0x713680` (`systems/models/model.md#bounds`) on the header's box from `+0xb4` and radius
/// `+0xcc`: x and y `(min + max)·0.5` with the sum in the register; z's sum is stored as `f32`
/// first.
pub fn bbox_center(bbox: &[f32; 14]) -> BBoxCenter {
    let sum = |k: usize| f64::from(bbox[k]) + f64::from(bbox[3 + k]);
    BBoxCenter {
        center: [
            (sum(0) * 0.5) as f32,
            (sum(1) * 0.5) as f32,
            (sum(2) as f32) * 0.5,
        ],
        radius: bbox[6],
    }
}

/// A normalized axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axis3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// `0x7137c0` from `0x713996` to `0x713a4a` (`systems/models/model.md#collision-facets`): the three
/// rows of the matrix's rotation each normalized in place.
pub fn shadow_normalize_axes(axis0: [f32; 3], axis1: [f32; 3], axis2: [f32; 3]) -> [Axis3; 3] {
    [axis0, axis1, axis2].map(|a| {
        let [x, y, z] = normalize(a);
        Axis3 { x, y, z }
    })
}

/// A facet's plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneEq {
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
    /// `−(n'·v0)`.
    pub w: f32,
}

/// `0x7137c0` from `0x713bc6` to `0x713c3f`: a triangle's normal through the normalized rows,
/// `(a2·n.z + a1·n.y) + a0·n.x` per component in the register, stored as `f32`; and
/// `w = −((n'.z·v.z + n'.y·v.y) + n'.x·v.x)`.
pub fn shadow_transform_normal(basis: [Axis3; 3], n: [f32; 3], v0: [f32; 3]) -> PlaneEq {
    let [a0, a1, a2] = basis;
    let [nx, ny, nz] = n.map(f64::from);
    let row = |c0: f32, c1: f32, c2: f32| {
        (f64::from(c2) * nz + f64::from(c1) * ny + f64::from(c0) * nx) as f32
    };
    let (ox, oy, oz) = (
        row(a0.x, a1.x, a2.x),
        row(a0.y, a1.y, a2.y),
        row(a0.z, a1.z, a2.z),
    );
    let [vx, vy, vz] = v0.map(f64::from);
    let dot = f64::from(oz) * vz + f64::from(oy) * vy + f64::from(ox) * vx;
    PlaneEq {
        nx: ox,
        ny: oy,
        nz: oz,
        w: (-dot) as f32,
    }
}
