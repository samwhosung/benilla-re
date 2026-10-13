//! The draw side (`systems/models/particles.md#particle-view-space`,
//! `systems/models/particles.md#draw-order-and-batches`,
//! `systems/models/particles.md#model-particles`, `systems/models/particles.md#head-quads`,
//! `systems/models/model.md#reference-frame`): `CParticleEmitter2::Render` (`0x7b3d20`),
//! `CParticleEmitter2::WriteAllQuads` (`0x7b3a10`), `CParticleEmitter2::DrawModelParticles`
//! (`0x7b4840`), `CParticleEmitter2::DrawModelParticle` (`0x7b4510`), `CParticleEmitter2::SetFrame`
//! (`0x7b5160`), `CParticleEmitter2::SetTextureGrid` (`0x7b4ed0`) and
//! `CParticleEmitter2::TransformParticles` (`0x7b5e60`).

/// 1.0 (`0x7ff9d8`).
const ONE: f64 = 1.0;
/// 512.0 (`0x8029cc`).
const LOD_BIAS: f64 = 512.0;
/// 1/255 (`0x8026c8`, `0x3b808081`).
const INV_255: f64 = f32::from_bits(0x3b80_8081) as f64;
/// 2.3841858e-7 (`0x8029d4`): the least length the quad-plane normal is normalized at.
const NORM_EPS: f64 = f32::from_bits(0x3480_0000) as f64;
/// 1.0 (`0x7ff9d8`), the numerator of the grid reciprocals.
pub const RECIP_ONE: f32 = 1.0;

/// `0x7b4549..0x7b455d`: the twinkle index `+0x1b0·age + 512.0`, stored as `f32`.
pub fn lod_index_scale(scale: f32, radius: f32) -> f32 {
    (f64::from(scale) * f64::from(radius) + LOD_BIAS) as f32
}

/// `0x7b45f8..0x7b460e`: `(noise·+0x1c0 + +0x1b8)·size`, one register chain stored as `f32`.
pub fn alpha_distance_scale(table: f32, k: f32, bias: f32, param: f32) -> f32 {
    ((f64::from(table) * f64::from(k) + f64::from(bias)) * f64::from(param)) as f32
}

/// `0x7b461c..0x7b4625`, flag 0x200: `size·+0x264`.
pub fn flag200_scale(param: f32, mul: f32) -> f32 {
    (f64::from(param) * f64::from(mul)) as f32
}

/// `0x7b4628..0x7b46ee`: the rotation of the particle's quaternion `[x, y, z, w]` (`+0x20`) passed
/// to `0x5f8d20`. `2x` and `2y` stay in the register, `2z` and the products `2x·w`, `2y·w`, `2z·w`,
/// `2x·x`, `2y·x`, `2z·x`, `2z·z` are stored as `f32`, `2y·y` and `2z·y` stay in the register:
/// `[1 − (2yy + 2xx), 2zy − 2xw, 2zx + 2yw, 2zy + 2xw, 1 − (2zz + 2xx), 2yx − 2zw, 2zx − 2yw,
/// 2yx + 2zw, 1 − (2zz + 2yy)]`.
pub fn quat_billboard_matrix(q: [f32; 4]) -> [f32; 9] {
    let [x, y, z, w] = q.map(f64::from);
    let a = x + x;
    let b = y + y;
    let c = f64::from((z + z) as f32);
    let r = |v: f64| f64::from(v as f32);
    let (aw, bw, cw) = (r(a * w), r(b * w), r(c * w));
    let (ax, bx, cx, cz) = (r(a * x), r(b * x), r(c * x), r(c * z));
    let by = b * y;
    let cy = c * y;
    [
        (ONE - (by + ax)) as f32,
        (cy - aw) as f32,
        (cx + bw) as f32,
        (cy + aw) as f32,
        (ONE - (cz + ax)) as f32,
        (bx - cw) as f32,
        (cx - bw) as f32,
        (bx + cw) as f32,
        (ONE - (cz + by)) as f32,
    ]
}

/// `0x7b47c3..0x7b480e`: a colour byte times 1/255.
pub fn color_byte_to_f32(byte: u32) -> f32 {
    (f64::from(byte) * INV_255) as f32
}

/// `0x7b3e51..0x7b3e90`: the translation row `−+0x23c` to `−+0x244` (`fchs`).
pub fn render_negate(v: [f32; 3]) -> [f32; 3] {
    [-v[0], -v[1], -v[2]]
}

/// The flat corners of `0x7b41a3`: the matrix at `0xcf5888` (`[cf5888, cf588c, cf5890, cf5898,
/// cf589c, cf58a0, cf58a8, cf58ac, cf58b0]`) and the four corners at `0xcf5af8`.
#[derive(Clone, Copy, Debug)]
pub struct Mat3x4Inputs {
    pub m: [f32; 9],
    pub cols: [[f32; 3]; 4],
}

/// `0x7b41a3..0x7b4249`, flag 0x2000: each corner `(i0, i1, i2)` through the matrix, stored at
/// `0xcf5b2c`: `(m3·i1 + m6·i2) + m0·i0`, `(m4·i1 + m1·i0) + m7·i2`, `(m5·i1 + m8·i2) + m2·i0`.
pub fn mat3x4_transform(inp: &Mat3x4Inputs) -> [[f32; 3]; 4] {
    let m = inp.m.map(f64::from);
    inp.cols.map(|col| {
        let [i0, i1, i2] = col.map(f64::from);
        [
            ((m[3] * i1 + m[6] * i2) + m[0] * i0) as f32,
            ((m[4] * i1 + m[1] * i0) + m[7] * i2) as f32,
            ((m[5] * i1 + m[8] * i2) + m[2] * i0) as f32,
        ]
    })
}

/// `0x7b426b..0x7b42a3`: the quad-plane normal `+0x284` scaled by `1/len`, `len =
/// √((x² + y²) + z²)`, unless `|len|` is under 2.3841858e-7 (`test ah,5; jnp`, so NaN scales).
pub fn render_normalize(v: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = v.map(f64::from);
    let len = ((x * x + y * y) + z * z).sqrt();
    if len.abs() < NORM_EPS {
        return v;
    }
    let inv = ONE / len;
    v.map(|c| (f64::from(c) * inv) as f32)
}

/// One row of `0x7b5e60`: `(a0·b0 + a1·b1) + a2·b2` in the register, stored as `f32`.
fn orient_row(a0: f32, b0: f32, a1: f32, b1: f32, a2: f32, b2: f32) -> f32 {
    let p = |a: f32, b: f32| f64::from(a) * f64::from(b);
    ((p(a0, b0) + p(a1, b1)) + p(a2, b2)) as f32
}

/// `0x7b5e60`: a particle's velocity through the frame's 3×3 (`m` the 16 floats):
/// `(m0·vx + m4·vy) + m8·vz`, `(m9·vz + m1·vx) + m5·vy`, `(m10·vz + m6·vy) + m2·vx`.
pub fn orient_velocity(m: &[f32; 16], v: [f32; 3]) -> [f32; 3] {
    let [vx, vy, vz] = v;
    [
        orient_row(m[0], vx, m[4], vy, m[8], vz),
        orient_row(m[9], vz, m[1], vx, m[5], vy),
        orient_row(m[10], vz, m[6], vy, m[2], vx),
    ]
}

/// The sort key of `0x7b3a5b` and `0x7b4840`: `((az·pz + ay·py) + ax·px) + off` with `axis =
/// [ax, ay, az, off]`, stored as `f32`.
pub fn depth_dot(axis: [f32; 4], pos: [f32; 3]) -> f32 {
    let [ax, ay, az, off] = axis.map(f64::from);
    let [px, py, pz] = pos.map(f64::from);
    (((az * pz + ay * py) + ax * px) + off) as f32
}

/// `0x7b51f9..0x7b5219`: the scale `√((x² + y²) + z²)` of the matrix's first row, stored at
/// `+0x264`.
pub fn transform_basis_scale(col0: [f32; 3]) -> f32 {
    let [x, y, z] = col0.map(f64::from);
    ((x * x + y * y) + z * z).sqrt() as f32
}

/// `0x7b4ed0`: both grid sizes powers of two, else the error path `0x64e850`.
pub fn tile_grid_valid(cols: u32, rows: u32) -> bool {
    let pow2 = |d: u32| d != 0 && d & d.wrapping_sub(1) == 0;
    pow2(cols) && pow2(rows)
}

/// `0x7b4ed0`: `(1/rows, 1/cols)` from unsigned `fild qword`, stored at `+0x10` and `+0x14`.
pub fn tile_grid_recip(cols: u32, rows: u32) -> (f32, f32) {
    let one = f64::from(RECIP_ONE);
    (
        (one / f64::from(rows)) as f32,
        (one / f64::from(cols)) as f32,
    )
}
