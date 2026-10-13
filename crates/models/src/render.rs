//! Drawing a model (`systems/models/render.md`): software skinning `M2SkinVertices` (`0x71a720`),
//! `M2SkinVerticesTwoTexCoords` (`0x71a460`) and `M2SkinVerticesSse` (`0x71a9e0`), the projected
//! bounds of `CM2Model::GetSkinnedBounds` (`0x719c20`), the lighting and material state of
//! `M2DrawSetLighting` (`0x70baf0`) and `M2DrawSetMaterial` (`0x70c190`), the emitter offset of
//! `M2DrawBeginEmitter` (`0x70ca50`), the texture matrix of `M2DrawProjectedBatch` (`0x70cf70`) and
//! the unit-length test of `M2DrawBatchedRun` (`0x70d330`).

use crate::lights::light_pack_triple3;

/// 1/255 (`0x8026c8`, `0x3b808081`): a bone weight's scale.
const INV_255: f32 = f32::from_bits(0x3b80_8081);
/// 2.3841858e-7 (`0x8029d4`): the least extent a projection is built on.
const DECAL_EPS: f32 = f32::from_bits(0x3480_0000);
/// 0.001 (`0x801360`, `0x3a83126f`): how far from 1 a squared length may be and count as unit.
pub const UNIT_LEN_DELTA: f32 = f32::from_bits(0x3a83_126f);

/// A blended bone matrix: the six entries the x87 routines keep in registers and the six they keep
/// on the stack as `f32`.
struct Blend {
    m0: f64,
    m1: f64,
    m2: f64,
    m4: f64,
    m5: f64,
    m6: f64,
    m8: f32,
    m9: f32,
    m10: f32,
    m12: f32,
    m13: f32,
    m14: f32,
}

/// The bone blend of `0x71a720` and `0x71a460` (`systems/models/render.md#software-skinning`). With
/// every weight zero the seeds stay: identity. The first bone replaces each entry: `w =
/// byte·(1/255)` in the register, stored as `f32` by `fst`; entry 0 takes the unrounded weight,
/// every other entry the stored one. Each later bone up to the first zero weight adds `w·m` with
/// `w` unrounded; the register entries sum unrounded, the stack entries are stored as `f32` each
/// time.
fn blend_bones(weights: [u8; 4], indices: [u8; 4], bones: &dyn Fn(u8) -> [f32; 16]) -> Blend {
    let w0 = f64::from(weights[0]) * f64::from(INV_255);
    let w0s = f64::from(w0 as f32);
    let m = bones(indices[0]).map(f64::from);
    let mut b = Blend {
        m0: w0 * m[0],
        m1: w0s * m[1],
        m2: w0s * m[2],
        m4: w0s * m[4],
        m5: w0s * m[5],
        m6: w0s * m[6],
        m8: (w0s * m[8]) as f32,
        m9: (w0s * m[9]) as f32,
        m10: (w0s * m[10]) as f32,
        m12: (w0s * m[12]) as f32,
        m13: (w0s * m[13]) as f32,
        m14: (w0s * m[14]) as f32,
    };
    for k in 1..4 {
        if weights[k] == 0 {
            break;
        }
        let w = f64::from(weights[k]) * f64::from(INV_255);
        let m = bones(indices[k]).map(f64::from);
        b.m0 += w * m[0];
        b.m1 += w * m[1];
        b.m2 += w * m[2];
        b.m4 += w * m[4];
        b.m5 += w * m[5];
        b.m6 += w * m[6];
        b.m8 = (w * m[8] + f64::from(b.m8)) as f32;
        b.m9 = (w * m[9] + f64::from(b.m9)) as f32;
        b.m10 = (w * m[10] + f64::from(b.m10)) as f32;
        b.m12 = (w * m[12] + f64::from(b.m12)) as f32;
        b.m13 = (w * m[13] + f64::from(b.m13)) as f32;
        b.m14 = (w * m[14] + f64::from(b.m14)) as f32;
    }
    b
}

/// A position `((m8·z + m4·y) + m0·x) + m12` per row and a normal `(m0·x + m8·z) + m4·y`, each
/// summed in the register and stored as `f32` (`0x71a912` to `0x71a9a5`).
fn apply_blend(b: &Blend, pos: [f32; 3], normal: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let [x, y, z] = pos.map(f64::from);
    let row = |r0: f64, r4: f64, m8: f32, m12: f32| {
        (((f64::from(m8) * z + r4 * y) + r0 * x) + f64::from(m12)) as f32
    };
    let [nx, ny, nz] = normal.map(f64::from);
    let nrow = |r0: f64, r4: f64, m8: f32| ((r0 * nx + f64::from(m8) * nz) + r4 * ny) as f32;
    (
        [
            row(b.m0, b.m4, b.m8, b.m12),
            row(b.m1, b.m5, b.m9, b.m13),
            row(b.m2, b.m6, b.m10, b.m14),
        ],
        [
            nrow(b.m0, b.m4, b.m8),
            nrow(b.m1, b.m5, b.m9),
            nrow(b.m2, b.m6, b.m10),
        ],
    )
}

/// A skinned vertex: position, normal, texture coordinates.
pub type TransformOut = [f32; 8];

/// `0x71a720` (`systems/models/render.md#software-skinning`) on one vertex: the blended bone matrix
/// applied to the position and normal; the texture coordinates copied.
pub fn m2_vertex_transform(
    pos: [f32; 3],
    normal: [f32; 3],
    weights: [u8; 4],
    indices: [u8; 4],
    texcoord: [f32; 2],
    bones: &dyn Fn(u8) -> [f32; 16],
) -> TransformOut {
    let (p, n) = apply_blend(&blend_bones(weights, indices, bones), pos, normal);
    [p[0], p[1], p[2], n[0], n[1], n[2], texcoord[0], texcoord[1]]
}

/// A vertex skinned by `0x71a460`: position, normal, then four dwords copied.
pub type SkinOut = [f32; 10];

/// `0x71a460` (`systems/models/render.md#software-skinning`) on one vertex:
/// [`m2_vertex_transform`]'s arithmetic with four dwords copied after the normal.
pub fn m2_vertex_skin(
    pos: [f32; 3],
    normal: [f32; 3],
    weights: [u8; 4],
    indices: [u8; 4],
    passthrough: [u32; 4],
    bones: &dyn Fn(u8) -> [f32; 16],
) -> SkinOut {
    let (p, n) = apply_blend(&blend_bones(weights, indices, bones), pos, normal);
    let t = passthrough.map(f32::from_bits);
    [p[0], p[1], p[2], n[0], n[1], n[2], t[0], t[1], t[2], t[3]]
}

/// `0x71a9e0` (`systems/models/render.md#software-skinning`) on one vertex with an aligned output:
/// the same blend in single precision (`mulps`, `addps`), the weight `byte·(1/255)` as `f32`. A
/// position row is `((x·c0 + c3) + y·c1) + z·c2`, a normal row `(x·c0 + y·c1) + z·c2`.
pub fn m2_vertex_transform_sse(
    pos: [f32; 3],
    normal: [f32; 3],
    weights: [u8; 4],
    indices: [u8; 4],
    texcoord: [f32; 2],
    bones: &dyn Fn(u8) -> [f32; 16],
) -> TransformOut {
    let mut acc = [[0f32; 4]; 4];
    let w0 = f32::from(weights[0]) * INV_255;
    let m = bones(indices[0]);
    for col in 0..4 {
        for row in 0..4 {
            acc[col][row] = m[col * 4 + row] * w0;
        }
    }
    for k in 1..4 {
        if weights[k] == 0 {
            break;
        }
        let w = f32::from(weights[k]) * INV_255;
        let m = bones(indices[k]);
        for col in 0..4 {
            for row in 0..4 {
                acc[col][row] += m[col * 4 + row] * w;
            }
        }
    }
    let p: [f32; 3] = std::array::from_fn(|r| {
        ((pos[0] * acc[0][r] + acc[3][r]) + pos[1] * acc[1][r]) + pos[2] * acc[2][r]
    });
    let n: [f32; 3] = std::array::from_fn(|r| {
        (normal[0] * acc[0][r] + normal[1] * acc[1][r]) + normal[2] * acc[2][r]
    });
    [p[0], p[1], p[2], n[0], n[1], n[2], texcoord[0], texcoord[1]]
}

/// The entries `0x719c20`'s blend writes: three of each row, leaving the last.
const DECAL_BLEND_SLOTS: [usize; 12] = [0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14];

/// `0x719c20`'s bone blend into `m` in place: the first bone replaces, later ones up to the first
/// zero weight add; every entry is stored as `f32`.
fn decal_blend_bones_inplace(
    m: &mut [f32; 16],
    weights: [u8; 4],
    indices: [u8; 4],
    bones: &dyn Fn(u8) -> [f32; 16],
) {
    let w0 = f64::from(weights[0]) * f64::from(INV_255);
    let bm = bones(indices[0]);
    for &s in &DECAL_BLEND_SLOTS {
        m[s] = (w0 * f64::from(bm[s])) as f32;
    }
    for k in 1..4 {
        if weights[k] == 0 {
            break;
        }
        let w = f64::from(weights[k]) * f64::from(INV_255);
        let bm = bones(indices[k]);
        for &s in &DECAL_BLEND_SLOTS {
            m[s] = (w * f64::from(bm[s]) + f64::from(m[s])) as f32;
        }
    }
}

/// `0x74a7c8`: the 4×4 product `a·b` with each entry's four products summed in the routine's
/// order and stored as `f32`.
fn mat4_mul_colmajor(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let af = |off: usize| f64::from(a[off / 4]);
    let bf = |off: usize| f64::from(b[off / 4]);
    let mut o = [0f32; 16];
    o[0] = (af(0xc) * bf(0x30) + af(0) * bf(0) + af(8) * bf(0x20) + bf(0x10) * af(4)) as f32;
    o[1] = (af(4) * bf(0x14) + af(0xc) * bf(0x34) + af(8) * bf(0x24) + bf(4) * af(0)) as f32;
    o[2] = (af(0xc) * bf(0x38) + bf(8) * af(0) + af(8) * bf(0x28) + bf(0x18) * af(4)) as f32;
    o[3] = (af(0xc) * bf(0x3c) + bf(0x2c) * af(8) + af(0) * bf(0xc) + bf(0x1c) * af(4)) as f32;
    o[4] =
        (af(0x1c) * bf(0x30) + af(0x10) * bf(0) + bf(0x10) * af(0x14) + af(0x18) * bf(0x20)) as f32;
    o[5] =
        (af(0x18) * bf(0x24) + af(0x1c) * bf(0x34) + bf(0x14) * af(0x14) + bf(4) * af(0x10)) as f32;
    o[6] =
        (bf(0x18) * af(0x14) + af(0x1c) * bf(0x38) + bf(8) * af(0x10) + af(0x18) * bf(0x28)) as f32;
    o[7] = (bf(0x1c) * af(0x14) + af(0x1c) * bf(0x3c) + af(0x18) * bf(0x2c) + af(0x10) * bf(0xc))
        as f32;
    o[8] =
        (af(0x2c) * bf(0x30) + af(0x20) * bf(0) + bf(0x10) * af(0x24) + af(0x28) * bf(0x20)) as f32;
    o[9] =
        (af(0x2c) * bf(0x34) + af(0x28) * bf(0x24) + bf(0x14) * af(0x24) + bf(4) * af(0x20)) as f32;
    o[0xa] =
        (bf(0x18) * af(0x24) + bf(8) * af(0x20) + af(0x2c) * bf(0x38) + af(0x28) * bf(0x28)) as f32;
    o[0xb] = (af(0x2c) * bf(0x3c) + af(0x28) * bf(0x2c) + bf(0x1c) * af(0x24) + af(0x20) * bf(0xc))
        as f32;
    o[0xc] =
        (af(0x3c) * bf(0x30) + af(0x38) * bf(0x20) + af(0x30) * bf(0) + bf(0x10) * af(0x34)) as f32;
    o[0xd] =
        (af(0x3c) * bf(0x34) + bf(0x14) * af(0x34) + af(0x38) * bf(0x24) + bf(4) * af(0x30)) as f32;
    o[0xe] =
        (af(0x38) * bf(0x28) + bf(0x18) * af(0x34) + bf(8) * af(0x30) + af(0x3c) * bf(0x38)) as f32;
    o[0xf] = (af(0x3c) * bf(0x3c) + bf(0x1c) * af(0x34) + af(0x30) * bf(0xc) + bf(0x2c) * af(0x38))
        as f32;
    o
}

/// One vertex `0x719c20` reads.
#[derive(Clone, Copy)]
pub struct DecalVert {
    pub pos: [f32; 3],
    pub weights: [u8; 4],
    pub indices: [u8; 4],
    /// `+0x20`, `+0x24`.
    pub texcoord: [f32; 2],
}

/// What `0x719c20` writes: the box through its second argument, and the texture matrix through
/// its third when it returns 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecalProjection {
    pub accepted: bool,
    pub aabb_min: [f32; 3],
    pub aabb_max: [f32; 3],
    pub texgen: [f32; 16],
}

/// The least, greatest and remaining of three values as `0x719c20` picks them (`0x71a093` to
/// `0x71a104`): the least starts at 0 unless `v0 < v1` fails (ordered), then becomes 2 unless
/// `v[lo] < v2`; the greatest starts at 0 only when `v0 > v1`, then becomes 2 unless `v[hi] > v2`.
/// A NaN fails each compare.
fn dominant_axis(v: [f32; 3]) -> (usize, usize, usize) {
    let mut lo = if v[0] < v[1] { 0 } else { 1 };
    if !(v[lo] < v[2]) {
        lo = 2;
    }
    let mut hi = if v[0] > v[1] { 0 } else { 1 };
    if !(v[hi] > v[2]) {
        hi = 2;
    }
    (lo, hi, (3 - hi).wrapping_sub(lo))
}

/// `0x719c20` (`systems/models/render.md#projected-bounds`,
/// `systems/models/render.md#grouped-and-projected-draws`): the box of a submesh's skinned vertices
/// and the planar texture matrix through its first three. `world` is the scene matrix `+0xdc`,
/// `pivot` the scene's `+0x10c`.
///
/// Each vertex whose weights or indices differ from the last (the indices compared as a float, so
/// `0` equals `−0` and NaN never matches) is blended into the persistent matrix and multiplied by
/// `world` in place. The position row `((m0·x + m8·z) + m4·y) + m12`: x stays in the register for
/// its box compare and its offset from the pivot; y and z are stored as `f32` first. Each box
/// compare is ordered, so a NaN never moves it.
///
/// Two passes then pick the least, greatest and remaining of the first three on y, then on x; when
/// `|least − greatest|` (in the register) is below 2.3841858e-7 (ordered, `0x71a131` and
/// `0x71a2ff`) the result is 0.
pub fn m2_decal_projection(
    count: u16,
    verts: &[DecalVert],
    bones: &dyn Fn(u8) -> [f32; 16],
    world: &[f32; 16],
    pivot: [f32; 3],
) -> DecalProjection {
    let reject = |aabb_min, aabb_max| DecalProjection {
        accepted: false,
        aabb_min,
        aabb_max,
        texgen: [0.0; 16],
    };
    let mut aabb_min = [f32::MAX; 3];
    let mut aabb_max = [f32::MIN; 3];
    if count < 4 {
        return reject(aabb_min, aabb_max);
    }
    let mut ref_pos = [[0f32; 3]; 3];
    let mut ref_uv = [[0f32; 2]; 3];
    let mut m: [f32; 16] = std::array::from_fn(|i| if i % 5 == 0 { 1.0 } else { 0.0 });
    let mut cache_w: u32 = 0;
    let mut cache_i: f32 = 0.0;

    for (i, v) in verts.iter().take(usize::from(count)).enumerate() {
        let w_dword = u32::from_le_bytes(v.weights);
        let i_float = f32::from_bits(u32::from_le_bytes(v.indices));
        if !(cache_w == w_dword && cache_i == i_float) {
            decal_blend_bones_inplace(&mut m, v.weights, v.indices, bones);
            m = mat4_mul_colmajor(&m, world);
            cache_w = w_dword;
            cache_i = i_float;
        }
        let [x, y, z] = v.pos.map(f64::from);
        let row = |k: usize| {
            ((f64::from(m[k]) * x + f64::from(m[8 + k]) * z) + f64::from(m[4 + k]) * y)
                + f64::from(m[12 + k])
        };
        let px_live = row(0);
        let p = [px_live as f32, row(1) as f32, row(2) as f32];
        for k in 0..3 {
            if p[k] < aabb_min[k] {
                aabb_min[k] = p[k];
            }
            if aabb_max[k] < p[k] {
                aabb_max[k] = p[k];
            }
        }
        if i < 3 {
            ref_pos[i] = [
                (px_live - f64::from(pivot[0])) as f32,
                (f64::from(p[1]) - f64::from(pivot[1])) as f32,
                (f64::from(p[2]) - f64::from(pivot[2])) as f32,
            ];
            ref_uv[i] = v.texcoord;
        }
    }

    let rp = |r: usize, c: usize| f64::from(ref_pos[r][c]);
    let ru = |r: usize, c: usize| f64::from(ref_uv[r][c]);
    let degenerate =
        |lo: usize, hi: usize, c: usize| (rp(lo, c) - rp(hi, c)).abs() < f64::from(DECAL_EPS);

    // On y: x and the coordinates at the remaining vertex's place between the extremes. The x
    // product and v's span are stored as `f32` before use; u's are not.
    let (lo1, hi1, mid1) = dominant_axis([0, 1, 2].map(|r| ref_pos[r][1]));
    if degenerate(lo1, hi1, 1) {
        return reject(aabb_min, aabb_max);
    }
    let f1 = (rp(mid1, 1) - rp(lo1, 1)) / (rp(hi1, 1) - rp(lo1, 1));
    let xc = (f64::from(((rp(hi1, 0) - rp(lo1, 0)) * f1) as f32) + rp(lo1, 0)) as f32;
    let uc0 = ((ru(hi1, 0) - ru(lo1, 0)) * f1 + ru(lo1, 0)) as f32;
    let v_span1 = ref_uv[hi1][1] - ref_uv[lo1][1];
    let uc1 = (f64::from((f64::from(v_span1) * f1) as f32) + ru(lo1, 1)) as f32;
    let inv_x = 1.0 / (f64::from(xc) - rp(mid1, 0));
    let p4_0 = ((f64::from(uc0) - ru(mid1, 0)) * inv_x) as f32;
    let p4_1 = ((f64::from(uc1) - ru(mid1, 1)) * inv_x) as f32;

    // On x, the same for y.
    let (lo2, hi2, mid2) = dominant_axis([0, 1, 2].map(|r| ref_pos[r][0]));
    if degenerate(lo2, hi2, 0) {
        return reject(aabb_min, aabb_max);
    }
    let f2 = (rp(mid2, 0) - rp(lo2, 0)) / (rp(hi2, 0) - rp(lo2, 0));
    let yc = ((rp(hi2, 1) - rp(lo2, 1)) * f2 + rp(lo2, 1)) as f32;
    let vc0 = ((ru(hi2, 0) - ru(lo2, 0)) * f2 + ru(lo2, 0)) as f32;
    let v_span2 = f64::from(ref_uv[hi2][1] - ref_uv[lo2][1]);
    let vc1 = (f64::from((v_span2 * f2) as f32) + ru(lo2, 1)) as f32;
    let inv_y = 1.0 / (f64::from(yc) - rp(mid2, 1));
    // `[4]` is stored by `fst` and its register value used for `[0xc]`; `[5]` is reloaded.
    let p4_4_live = (f64::from(vc0) - ru(mid2, 0)) * inv_y;
    let p4_5 = ((f64::from(vc1) - ru(mid2, 1)) * inv_y) as f32;

    let mut texgen = [0f32; 16];
    texgen[0] = p4_0;
    texgen[1] = p4_1;
    texgen[4] = p4_4_live as f32;
    texgen[5] = p4_5;
    texgen[0xf] = 1.0;
    texgen[0xc] = ((ru(0, 0) - rp(0, 0) * f64::from(p4_0)) - p4_4_live * rp(0, 1)) as f32;
    texgen[0xd] = ((ru(0, 1) - rp(0, 0) * f64::from(p4_1)) - f64::from(p4_5) * rp(0, 1)) as f32;
    DecalProjection {
        accepted: true,
        aabb_min,
        aabb_max,
        texgen,
    }
}

/// 224.0 (`0x812034`).
const COLOR_SCALE: f32 = f32::from_bits(0x4360_0000);

/// `0x70c190` at `0x70c259` (`systems/models/render.md#material-state`): the material's `+0x10`
/// times 224, left in the register for `__ftol`.
pub fn color_scale(v: f32) -> f64 {
    f64::from(v) * f64::from(COLOR_SCALE)
}

/// `0x70c190` from `0x70c36a` to `0x70c38a` (and its copy at `0x70c56f`): `|(x² + y²) + z² − 1|`
/// in the register, compared at `0x70c390` with 0.001.
pub fn normalize_residual(x: f32, y: f32, z: f32) -> f64 {
    let (x, y, z) = (f64::from(x), f64::from(y), f64::from(z));
    ((x * x + y * y) + z * z - 1.0).abs()
}

/// `fld; fmul; fstp` with two `f32` operands, as `0x70c190` scales a colour at `0x70c631`,
/// `0x70c63f` and `0x70c64b`.
pub fn scale_f32(a: f32, b: f32) -> f32 {
    a * b
}

/// `fld; fadd; fstp` with two `f32` operands, as `0x70c190` sums colours at `0x70c669`, `0x70c67e`
/// and `0x70c687`.
pub fn add_f32(a: f32, b: f32) -> f32 {
    a + b
}

/// One channel of `0x70c190`'s colour words (`0x70c77e` to `0x70c7c0`, seven more alike;
/// `systems/models/render.md#material-colours`): below 0 (ordered, `test ah,5; jp`) it is 0, above
/// 1 (`test ah,0x41; jne`) 1, otherwise, NaN included, itself; times 255 plus 0.5 in the register
/// for `__ftol`.
pub fn rgba_pack_value(v: f32) -> f64 {
    let c = if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    };
    f64::from(c) * 255.0 + 0.5
}

/// `0x70baf0` from `0x70bdbb` to `0x70bddf` (`systems/models/render.md#lighting-and-fog`): the
/// record's `+0x18c` times −1 and `+0x188` times `+0x18c`, each stored as `f32`.
#[allow(clippy::neg_multiply)] // `fmul` by -1.0, which leaves a NaN's sign
pub fn batch_alpha_resolve(l188: f32, l18c: f32) -> [f32; 2] {
    [l18c * -1.0, l188 * l18c]
}

/// `0x70baf0` from `0x70beb5` to `0x70bfaa`: the record's `+0x198`, `+0x194` and `+0x190` packed as
/// [`light_pack_triple3`].
pub fn batch_clamp_pack(c198: f32, c194: f32, c190: f32) -> u32 {
    light_pack_triple3(c198, c194, c190)
}

/// `0x70baf0` from `0x70c10f` to `0x70c147`: the clip plane negated for list 2.
pub fn batch_negate4(v: [f32; 4]) -> [f32; 4] {
    v.map(|c| -c)
}

/// `0x70ca50` from `0x70ca9b` to `0x70caef`
/// (`systems/models/render.md#ribbon-particle-and-custom-draws`): the view's translation moved by
/// `v`, each row `((m[8+k]·v2 + m[4+k]·v1) + m[k]·v0) + m[12+k]` in the register, stored as `f32`.
pub fn model_matrix_translate(m: [f32; 16], v: [f32; 3]) -> [f32; 16] {
    let mut out = m;
    let [v0, v1, v2] = v.map(f64::from);
    for k in 0..3 {
        let dot = (f64::from(m[8 + k]) * v2 + f64::from(m[4 + k]) * v1) + f64::from(m[k]) * v0;
        out[12 + k] = (dot + f64::from(m[12 + k])) as f32;
    }
    out
}

/// A texture matrix entry as a byte, `0x70cf70`'s ladder: below 0 (ordered) 0, above 1 (ordered)
/// 1, otherwise, NaN included, itself; then `__ftol` of `v·255 + 0.5`, its low byte.
fn clamp_pack_component(v: f64) -> u8 {
    let c = if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    };
    crate::x87::ftol(c * 255.0 + 0.5) as u8
}

/// What [`decal_texgen_pack`] stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecalTexgen {
    /// `[ebp-0x20]`, `[ebp-0x14]`: the depth range widened by 6 each way.
    pub plane_min: f32,
    pub plane_max: f32,
    /// `[ebp-0x48]`: `1/(max − min)`.
    pub inv_range: f32,
    /// `[ebp-0x38]`: `(ref − min)·inv_range`.
    pub depth: f32,
    /// `[ebp-0x8]`, `[ebp-0x4]`.
    pub comp_y: f32,
    pub comp_z: f32,
    /// `[ebp-0x10]`: the colour word.
    pub packed: u32,
}

/// `0x70cf70` from `0x70d07e` to `0x70d2c4`
/// (`systems/models/render.md#grouped-and-projected-draws`): the depth range `[min − 6, max + 6]`
/// (`0x802990`), `1/(max − min)` with the difference in the register, the depth offset; the colour
/// `mrow` (times `scale` when `cond`, x kept in the register, y and z stored as `f32`) plus
/// `offset`, and the colour word `b3 << 24 | x << 16 | y << 8 | z` with `comp3` the material's
/// `+0x10`, each through the ladder of [`clamp_pack_component`].
#[allow(clippy::too_many_arguments)]
pub fn decal_texgen_pack(
    aabb_min: f32,
    aabb_max: f32,
    ref_depth: f32,
    mrow: [f32; 3],
    scale: [f32; 3],
    cond: bool,
    offset: [f32; 3],
    comp3: f32,
) -> DecalTexgen {
    let plane_min = aabb_min - 6.0;
    let plane_max = aabb_max + 6.0;
    let inv_range = (1.0 / (f64::from(plane_max) - f64::from(plane_min))) as f32;
    let depth = ((f64::from(ref_depth) - f64::from(plane_min)) * f64::from(inv_range)) as f32;
    let [mut cx, mut cy, mut cz] = mrow.map(f64::from);
    if cond {
        cx *= f64::from(scale[0]);
        cy = f64::from((cy * f64::from(scale[1])) as f32);
        cz = f64::from((cz * f64::from(scale[2])) as f32);
    }
    let comp_x = cx + f64::from(offset[0]);
    let comp_y = (cy + f64::from(offset[1])) as f32;
    let comp_z = (cz + f64::from(offset[2])) as f32;
    let b0 = clamp_pack_component(comp_x);
    let b1 = clamp_pack_component(f64::from(comp_y));
    let b2 = clamp_pack_component(f64::from(comp_z));
    let b3 = clamp_pack_component(f64::from(comp3));
    let packed =
        u32::from(b2) | (u32::from(b1) << 8) | (u32::from(b0) << 16) | (u32::from(b3) << 24);
    DecalTexgen {
        plane_min,
        plane_max,
        inv_range,
        depth,
        comp_y,
        comp_z,
        packed,
    }
}

/// What [`unit_length_gate`] finds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitLengthGate {
    /// `|(x² + y²) + z² − 1|`, in the register.
    pub deviation: f64,
    /// Whether `[ebp-0xc]` is cleared.
    pub flag_cleared: bool,
}

/// `0x70d330` from `0x70d717` to `0x70d751`
/// (`systems/models/render.md#grouped-and-projected-draws`): a bone matrix's first row `v`;
/// `[ebp-0xc]` is cleared unless the deviation, unrounded, is above 0.001 (`test ah,0x41; je` at
/// `0x70d748`), so a deviation equal to it or NaN clears it.
pub fn unit_length_gate(v: [f32; 3]) -> UnitLengthGate {
    let deviation = normalize_residual(v[0], v[1], v[2]);
    UnitLengthGate {
        deviation,
        flag_cleared: !(deviation > f64::from(UNIT_LEN_DELTA)),
    }
}
