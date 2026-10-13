//! A scene's per-frame collection and its ray queries (`systems/models/scene.md`): the view rebase
//! and plane test of `M2SceneAnimateAndCollect` (`0x707680`), the ray hits of `M2SceneRayHit`
//! (`0x7089c0`) and the segment collision of `M2SceneSegmentCollision` (`0x7099e0`).

/// 9.9999997e-6 (`0x80c5c8`): the least determinant a triangle is tested at.
const DET_EPS: f32 = f32::from_bits(0x3727_c5ac);
/// 1/255 (`0x8026c8`).
const INV_255: f32 = f32::from_bits(0x3b80_8081);

/// `0x707680` from `0x7076cb` to `0x707752` (`systems/models/scene.md#view-rebase`): the view's
/// translation moved to the origin `p`, `t − ((m10·py + m20·pz) + m00·px)`, `t − ((m11·py + m01·px)
/// + m21·pz)` and `t − ((m12·py + m02·px) + m22·pz)` (`m` the view at `+0x9c`, `t` at `+0xcc`),
/// each sum in the register, stored as `f32`.
pub fn view_rebase(m: &[f32; 12], t: [f32; 3], p: [f32; 3]) -> [f32; 3] {
    let [px, py, pz] = p.map(f64::from);
    let mm = m.map(f64::from);
    let sums = [
        (mm[4] * py + mm[8] * pz) + mm[0] * px,
        (mm[5] * py + mm[1] * px) + mm[9] * pz,
        (mm[6] * py + mm[2] * px) + mm[10] * pz,
    ];
    [0, 1, 2].map(|k| (f64::from(t[k]) - sums[k]) as f32)
}

/// `(x² + y²) + z²` stored as `f32`, as `0x707680` takes a row's length at `0x7078fc`,
/// `0x707d8f` and `0x70841d`.
pub fn sum_of_squares(x: f32, y: f32, z: f32) -> f32 {
    let [x, y, z] = [x, y, z].map(f64::from);
    ((x * x + y * y) + z * z) as f32
}

/// `fld; fadd; fstp` of two `f32`, as `0x707680` sums the header's box at `0x707915` and
/// `0x70843a`.
pub fn vec_add(a: f32, b: f32) -> f32 {
    a + b
}

/// `0x707680` at `0x70795c` and `0x70847b` (`systems/models/scene.md#plane-classification`):
/// `√sumsq` in the register times the header's radius, stored as `f32`.
pub fn radius(sumsq: f32, scale: f32) -> f32 {
    (f64::from(sumsq).sqrt() * f64::from(scale)) as f32
}

/// `0x707680` from `0x707987` and `0x7084a0`: the centre's distance from the clip plane,
/// `((z·pz + y·py) + x·px) + pw`, in the register for the compare.
pub fn plane_dist(tv_x: f32, tv_y: f32, tv_z: f32, p_x: f32, p_y: f32, p_z: f32, p_w: f32) -> f64 {
    let pz = f64::from(tv_z) * f64::from(p_z);
    let py = f64::from(tv_y) * f64::from(p_y);
    let px = f64::from(tv_x) * f64::from(p_x);
    ((pz + py) + px) + f64::from(p_w)
}

/// `0x707680` at `0x7079c6` and `0x7084d2`: the radius negated in the register.
pub fn negate(radius: f32) -> f64 {
    -f64::from(radius)
}

/// `fld; fmul; fstp` of two `f32`, as `0x707680` scales an alpha at `0x707b0b`, `0x707b33` and
/// `0x7080e8`.
pub fn scale_mul(acc: f32, factor: f32) -> f32 {
    acc * factor
}

/// `0x7089c0` from `0x708a17` to `0x708ab6` and `0x7099e0` from `0x709a38` to `0x709ad7`
/// (`systems/models/scene.md#ray-hits`, `systems/models/scene.md#segment-collision`): the segment
/// `d = p1 − p0` with x kept in the register, y stored as `f32`, z stored by `fst` and kept; `len =
/// √((z·z_f32 + y·y) + x·x)` stored as `f32`; `inv = 1/len` kept and stored; the direction `(inv·x,
/// inv_f32·y, inv_f32·z)`. Returns `(len, inv, dir)`.
pub fn ray_setup(p0: [f32; 3], p1: [f32; 3]) -> (f32, f32, [f32; 3]) {
    let dx = f64::from(p1[0]) - f64::from(p0[0]);
    let dy = f64::from((f64::from(p1[1]) - f64::from(p0[1])) as f32);
    let dz_live = f64::from(p1[2]) - f64::from(p0[2]);
    let dz = f64::from(dz_live as f32);
    let len = ((dz_live * dz + dy * dy) + dx * dx).sqrt() as f32;
    let inv_live = 1.0 / f64::from(len);
    let inv = inv_live as f32;
    let invf = f64::from(inv);
    (
        len,
        inv,
        [
            (inv_live * dx) as f32,
            (invf * dy) as f32,
            (invf * dz) as f32,
        ],
    )
}

/// `0x7099e0` from `0x709c22` to `0x709c4f` and from `0x709c95`: the box's corner sum times 0.5,
/// stored as `f32`.
pub fn half_scale(v: [f32; 3]) -> [f32; 3] {
    v.map(|c| c * 0.5)
}

/// `0x7089c0` from `0x708c4f` and from `0x708cc0`: the two corners of a box summed, stored as
/// `f32` (z, y, x).
pub fn aabb_corner_sum(hi: [f32; 3], lo: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| hi[k] + lo[k])
}

/// A sphere against the ray.
#[derive(Clone, Copy, Debug)]
pub struct SphereHit {
    /// The squared distance of the line from the centre, in the register.
    pub perp2: f64,
    /// The squared radius, in the register.
    pub radius2: f64,
    /// [`sphere_meets`].
    pub hit: bool,
    /// `proj + √(r² − perp²)` in the register; stored as `f32`.
    pub t_far_live: f64,
    pub t_far: f32,
    /// `proj − √(r² − perp²)` in the register; stored as `f32`.
    pub t_near_live: f64,
    pub t_near: f32,
}

/// The line meets the sphere unless `perp²` is above `r²` (`fcom st(1); test ah,0x41; je`), so a
/// tangent and a NaN meet it.
pub fn sphere_meets(perp2: f64, radius2: f64) -> bool {
    !(perp2 > radius2)
}

/// The far point is not behind the start: `t_far` not below 0 (`test ah,5; jnp`), NaN included.
pub fn sphere_ahead(t_far: f64) -> bool {
    !(t_far < 0.0)
}

/// The near point is before the end: `t_near` not above `len` (`test ah,0x41; je`), NaN included.
pub fn sphere_within(t_near: f64, len: f32) -> bool {
    !(t_near > f64::from(len))
}

/// The near distance a candidate keeps (`0x709df3` to `0x709e10`): `t_near` when above 0, else 0
/// (`test ah,0x41; jne`, so NaN gives 0).
pub fn sphere_near(t_near: f32) -> f32 {
    if t_near > 0.0 {
        t_near
    } else {
        0.0
    }
}

/// The sphere arithmetic both queries share: `oc = c − o` stored as `f32`; `r² = ((ex² + ey²) +
/// ez²)·s·s` in the register; `proj = (oz·dz + oy·dy) + ox·dx` stored; the closest point's z
/// stored, `perp = closest − oc` with x stored; `perp²` summed as `order` gives it.
fn sphere(
    center: [f32; 3],
    origin: [f32; 3],
    extents: [f32; 3],
    scale: f32,
    ndir: [f32; 3],
    order: fn(f64, f64, f64) -> f64,
) -> SphereHit {
    let oc = [0, 1, 2].map(|k| f64::from(center[k] - origin[k]));
    let [ex, ey, ez] = extents.map(f64::from);
    let s = f64::from(scale);
    let radius2 = (ex * ex + ey * ey + ez * ez) * s * s;
    let [nx, ny, nz] = ndir.map(f64::from);
    let proj = f64::from(((oc[2] * nz + oc[1] * ny) + oc[0] * nx) as f32);
    let cz = f64::from((nz * proj) as f32);
    let px = f64::from((nx * proj - oc[0]) as f32);
    let py = ny * proj - oc[1];
    let pz = cz - oc[2];
    let perp2 = order(px * px, py * py, pz * pz);
    let hit = sphere_meets(perp2, radius2);
    let sq = (radius2 - perp2).sqrt();
    let (t_far_live, t_near_live) = (proj + sq, proj - sq);
    SphereHit {
        perp2,
        radius2,
        hit,
        t_far_live,
        t_far: t_far_live as f32,
        t_near_live,
        t_near: t_near_live as f32,
    }
}

/// `0x7099e0` from `0x709cf2` to `0x709de8` (`systems/models/scene.md#segment-collision`): an
/// instance's sphere, `perp² = (z² + y²) + x²`. A candidate is kept when [`sphere_meets`],
/// [`sphere_ahead`] and [`sphere_within`] all hold.
pub fn segment_sphere(
    center: [f32; 3],
    origin: [f32; 3],
    extents: [f32; 3],
    scale: f32,
    ndir: [f32; 3],
) -> SphereHit {
    sphere(center, origin, extents, scale, ndir, |x2, y2, z2| {
        (z2 + y2) + x2
    })
}

/// `0x7089c0` from `0x708d4c` to `0x708e3d` (`systems/models/scene.md#ray-hits`): the same with
/// `perp² = (y² + x²) + z²`.
pub fn hit_sphere(
    center: [f32; 3],
    origin: [f32; 3],
    extents: [f32; 3],
    scale: f32,
    ndir: [f32; 3],
) -> SphereHit {
    sphere(center, origin, extents, scale, ndir, |x2, y2, z2| {
        (y2 + x2) + z2
    })
}

/// `0x7099e0` from `0x709f1a` to `0x709fa8`: the segment in an instance's space, `d = tp1 − tp0`
/// stored as `f32`, `1/√((dz² + dy²) + dx²)` kept in the register and stored, and `d` times it.
/// Returns `(d, inv_len, dir)`.
pub fn tri_invray(tp0: [f32; 3], tp1: [f32; 3]) -> ([f32; 3], f32, [f32; 3]) {
    let d = [0, 1, 2].map(|k| tp1[k] - tp0[k]);
    let [dx, dy, dz] = d.map(f64::from);
    let inv_len = 1.0 / ((dz * dz + dy * dy) + dx * dx).sqrt();
    (
        d,
        inv_len as f32,
        [dx, dy, dz].map(|c| (c * inv_len) as f32),
    )
}

/// `0x7099e0` from `0x709fde` to `0x709fe7`: a hit's fraction in world units, `inv·t·len`, in the
/// register for the compare with the best.
pub fn tri_t_world(inv_len_local: f32, t_local: f32, len_world: f32) -> f64 {
    f64::from(inv_len_local) * f64::from(t_local) * f64::from(len_world)
}

/// `0x7089c0` at `0x708f5f` and `0x7099e0` at `0x709e7e`: with the incoming fraction below 1
/// (ordered, `test ah,5; jp`), the best is seeded `len·f`, stored as `f32`.
pub fn tbest_init(len: f32, ray_param: f32) -> f32 {
    len * ray_param
}

/// `0x7089c0` from `0x7099c4` and `0x7099e0` from `0x70a037`: the best fraction back in the
/// segment's units, `inv·t`, stored as `f32`.
pub fn world_distance(inv: f32, t_best: f32) -> f32 {
    inv * t_best
}

/// `0x7089c0` from `0x708eba` to `0x708f02`: `(dz·p0z + dx·p0x) + dy·p0y`, stored as `f32`.
pub fn scale_t_dot(ndir: [f32; 3], p0: [f32; 3]) -> f32 {
    let [nx, ny, nz] = ndir.map(f64::from);
    let [x, y, z] = p0.map(f64::from);
    ((nz * z + nx * x) + ny * y) as f32
}

/// `0x7089c0` at `0x709099` and `0x7090be`: the instance's alpha `+0x19c` times a unit's colour
/// and transparency, in the register; a unit at or below 0 is skipped (`0x7090ca` to `0x7090de`).
pub fn alpha_scale(base: f32, modifier: f32) -> f64 {
    f64::from(base) * f64::from(modifier)
}

/// `0x7089c0` from `0x709224` to `0x7092aa`: the first bone's weight `byte/255` in the register
/// times the twelve rotation and translation entries, stored as `f32`.
pub fn skin_first(weight_byte: u8, m: [f32; 12]) -> [f32; 12] {
    let w = f64::from(weight_byte) * f64::from(INV_255);
    m.map(|c| (w * f64::from(c)) as f32)
}

/// `0x7089c0` from `0x7092d0` to `0x7093a6`: a further bone's `w·m` added to each entry, stored as
/// `f32`.
pub fn skin_accum(weight_byte: u8, m: [f32; 12], acc: [f32; 12]) -> [f32; 12] {
    let w = f64::from(weight_byte) * f64::from(INV_255);
    std::array::from_fn(|k| (w * f64::from(m[k]) + f64::from(acc[k])) as f32)
}

/// `0x7089c0` from `0x7093c8` to `0x709442`: the blended rotation times the normal, each row
/// `(m[0]·vz + m[1]·vy) + m[2]·vx` in the register, added to `acc`; the third row is stored as
/// `f32` first.
pub fn mat_x_dir(m: [f32; 9], v: [f32; 3], acc: [f32; 3]) -> [f32; 3] {
    let [vx, vy, vz] = v.map(f64::from);
    let row = |i: usize| {
        (f64::from(m[i * 3]) * vz + f64::from(m[i * 3 + 1]) * vy) + f64::from(m[i * 3 + 2]) * vx
    };
    [
        (row(0) + f64::from(acc[0])) as f32,
        (row(1) + f64::from(acc[1])) as f32,
        acc[2] + row(2) as f32,
    ]
}

/// `0x7089c0` from `0x709445` to `0x7094a8` and from `0x709777` to `0x7097dd`: a vertex projected
/// onto the plane across the ray, `t = ((a·b) + (c·d)) + (e·f) − dot` in the register (the pairs in
/// `dot_pairs` order), then `acc − dir·t` for two components, and `t`, each stored as `f32`.
pub fn tworld_recover(
    dot_pairs: [(f32, f32); 3],
    org_pairs: [(f32, f32); 2],
    dot: f32,
) -> [f32; 3] {
    let p = |x: (f32, f32)| f64::from(x.0) * f64::from(x.1);
    let t = (p(dot_pairs[0]) + p(dot_pairs[1])) + p(dot_pairs[2]) - f64::from(dot);
    [
        (f64::from(org_pairs[0].1) - f64::from(org_pairs[0].0) * t) as f32,
        (f64::from(org_pairs[1].1) - f64::from(org_pairs[1].0) * t) as f32,
        t as f32,
    ]
}

/// What [`tri_predicate`] finds.
#[derive(Clone, Copy, Debug)]
pub struct TriHit {
    pub u: f32,
    pub v: f32,
    pub t: f32,
    pub accept: bool,
}

/// `0x7089c0` from `0x70951f` to `0x70960c` and from `0x70983e` to `0x70992b`
/// (`systems/models/scene.md#ray-hits`): a projected triangle against the ray's point `p0`. The
/// determinant of the edges from `v0`, in the register, misses below 9.9999997e-6 in magnitude;
/// `1/det` is stored. `u` from the stored deltas of `v1` and `v2`, `v` and `w` with `v0 − p0` kept
/// in the register, each times `1/det`; `t = (w·v2.z + v·v1.z) + u·v0.z` with `u`, `v` stored. Each
/// of `u`, `v`, `w` and `t` misses below 0; every compare is `test ah,5; jnp`, ordered, so a NaN
/// passes.
pub fn tri_predicate(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3], p0: [f32; 3]) -> TriHit {
    let miss = |u, v, t| TriHit {
        u,
        v,
        t,
        accept: false,
    };
    let det = (f64::from(v2[1]) - f64::from(v0[1])) * (f64::from(v1[0]) - f64::from(v0[0]))
        - (f64::from(v2[0]) - f64::from(v0[0])) * (f64::from(v1[1]) - f64::from(v0[1]));
    if det.abs() < f64::from(DET_EPS) {
        return miss(0.0, 0.0, 0.0);
    }
    let d_v2y = f64::from(v2[1] - p0[1]);
    let d_v1x = f64::from(v1[0] - p0[0]);
    let d_v1y = f64::from(v1[1] - p0[1]);
    let d_v2x = f64::from(v2[0] - p0[0]);
    let recip = f64::from((1.0 / det) as f32);
    let u_live = (d_v1x * d_v2y - d_v2x * d_v1y) * recip;
    let u = u_live as f32;
    if u_live < 0.0 {
        return miss(u, 0.0, 0.0);
    }
    let v0y = f64::from(v0[1]) - f64::from(p0[1]);
    let v0x = f64::from(v0[0]) - f64::from(p0[0]);
    let v_live = (v0y * d_v2x - v0x * d_v2y) * recip;
    let v = v_live as f32;
    if v_live < 0.0 {
        return miss(u, v, 0.0);
    }
    let w_live = (v0x * d_v1y - v0y * d_v1x) * recip;
    if w_live < 0.0 {
        return miss(u, v, 0.0);
    }
    let t_live = (w_live * f64::from(v2[2]) + f64::from(v) * f64::from(v1[2]))
        + f64::from(u) * f64::from(v0[2]);
    TriHit {
        u,
        v,
        t: t_live as f32,
        accept: !(t_live < 0.0),
    }
}
