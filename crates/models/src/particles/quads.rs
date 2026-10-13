//! The quads one particle writes (`systems/models/particles.md#head-quads`,
//! `systems/models/particles.md#tail-quads`): `CParticleEmitter2::WriteQuads` (`0x7b2a50`). It
//! takes the particle's position through the particle matrix `0xcf5b68` (`0x7bca80`), samples the
//! gradients (`0x7b9b10`), swaps the colour bytes when the device caps ask, and writes the vertices
//! into four streams (position, normal, colour, texture coordinates), each advanced by its stride,
//! with a running count.
//!
//! The legs: flag 0x4 without spin (`+0x18c` compares equal to 0, `test ah,0x44; jp`) writes the
//! head square, or under flag 0x2000 the flat corners at `0xcf5b34`; with spin the square turns by
//! `fsincos`, or the flat corners turn by `BuildAxisRotation` (`0x7be490`) about `+0x284`. Flag 0x8
//! writes the tail. The spin's `fsincos` is the host's here.

/// The head square's corners at `0x87d714`.
const CORNER_XY: [f32; 8] = [-1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0, -1.0];
/// Their texture corners at `0x87d734`.
const CORNER_UV: [f32; 8] = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0];
/// The flat corners' texture corners, read at stride 8 from `0x87d734`: the same pairs.
const TILED_UV: [f32; 8] = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0];
/// 7.716e-4 (`0x80c744`, `0x3a4a4588`, 1/1296): under this squared length the tail is a square.
pub const STRETCH_EPS: f32 = f32::from_bits(0x3a4a_4588);

/// The vertex streams (`esi`): four write pointers, their strides, and the count at `+0x20`.
#[derive(Clone, Copy, Debug)]
pub struct QuadStreams {
    pub ptr: [u32; 4],
    pub stride: [u32; 4],
    pub count: u32,
}

/// One vertex as written: position, the three normal words (`0xcf5878` to `0xcf5880`), colour and
/// texture coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadVertex {
    pub pos: [f32; 3],
    pub normal_const: [u32; 3],
    pub color: u32,
    pub uv: [f32; 2],
}

/// What the head square is written from.
#[derive(Clone, Copy, Debug)]
pub struct BillboardInputs {
    /// The view-space position (`[ebp-0x30]`).
    pub center: [f32; 3],
    /// The size (`[ebp-0x10]`), its multipliers applied.
    pub scale: f32,
    /// The colour (`[ebp-0x18]`), already swapped.
    pub color: u32,
    /// The cell's corner (`[ebp-0x3c]`, `[ebp-0x38]`), from [`tex_base`].
    pub tex_base_u: f32,
    pub tex_base_v: f32,
    /// The cell size `+0x10`, `+0x14`.
    pub cell_u: f32,
    pub cell_v: f32,
    pub normal_const: [u32; 3],
}

/// The texture coordinates of corner `uv`: `u·cell_u + base_u` and `cell_v·v + base_v`.
fn corner_uv(uv: (f32, f32), cell: (f64, f64), base: (f64, f64)) -> [f32; 2] {
    [
        (f64::from(uv.0) * cell.0 + base.0) as f32,
        (cell.1 * f64::from(uv.1) + base.1) as f32,
    ]
}

/// `0x7b2bc3..0x7b2d0c`, the head square: corner `k` at `(s·x + cx, s·y + cy, cz)`.
pub fn quad_expand_billboard(inp: &BillboardInputs) -> [QuadVertex; 4] {
    let scale = f64::from(inp.scale);
    let (cx, cy) = (f64::from(inp.center[0]), f64::from(inp.center[1]));
    let cell = (f64::from(inp.cell_u), f64::from(inp.cell_v));
    let base = (f64::from(inp.tex_base_u), f64::from(inp.tex_base_v));
    core::array::from_fn(|k| QuadVertex {
        pos: [
            (scale * f64::from(CORNER_XY[2 * k]) + cx) as f32,
            (scale * f64::from(CORNER_XY[2 * k + 1]) + cy) as f32,
            inp.center[2],
        ],
        normal_const: inp.normal_const,
        color: inp.color,
        uv: corner_uv((CORNER_UV[2 * k], CORNER_UV[2 * k + 1]), cell, base),
    })
}

/// The colour swap at `0x7b2b4b` when caps `+0x1c` is 1: bytes 0 and 2 exchanged.
pub fn caps_swap_color(c: u32) -> u32 {
    (c & 0xff00_ff00) | ((c & 0xff) << 16) | ((c >> 16) & 0xff)
}

/// `0x7b2bd5..0x7b2c02`: a cell's corner `(((C − 1) & c)·cell_u, (c >> log2 C)·cell_v)` (`+0x19c`
/// the columns, `+0xc` the shift), each from an unsigned `fild qword` and stored as `f32`.
pub fn tex_base(
    tex_index: u32,
    atlas_dim: u32,
    vshift: u32,
    cell_u: f32,
    cell_v: f32,
) -> (f32, f32) {
    let col = atlas_dim.wrapping_sub(1) & tex_index;
    let row = tex_index >> (vshift & 0x1f);
    (
        (f64::from(col) * f64::from(cell_u)) as f32,
        (f64::from(row) * f64::from(cell_v)) as f32,
    )
}

/// What the flat square is written from: the head inputs and the four flat corners at `0xcf5b34`
/// (`mat3x4_transform` of `particles::draw`).
#[derive(Clone, Copy, Debug)]
pub struct TiledInputs {
    pub center: [f32; 3],
    pub scale: f32,
    pub color: u32,
    pub tex_base_u: f32,
    pub tex_base_v: f32,
    pub cell_u: f32,
    pub cell_v: f32,
    pub normal_const: [u32; 3],
    pub corner: [[f32; 3]; 4],
}

/// `0x7b2c25..0x7b2cfe`, flag 0x2000 without spin: corner `k` at `(s·x + cx, s·y + cy, f32(s·z) +
/// cz)`.
pub fn quad_expand_tiled(inp: &TiledInputs) -> [QuadVertex; 4] {
    let scale = f64::from(inp.scale);
    let [cx, cy, cz] = inp.center.map(f64::from);
    let cell = (f64::from(inp.cell_u), f64::from(inp.cell_v));
    let base = (f64::from(inp.tex_base_u), f64::from(inp.tex_base_v));
    core::array::from_fn(|k| {
        let c = inp.corner[k];
        let zterm = (scale * f64::from(c[2])) as f32;
        QuadVertex {
            pos: [
                (scale * f64::from(c[0]) + cx) as f32,
                (scale * f64::from(c[1]) + cy) as f32,
                (f64::from(zterm) + cz) as f32,
            ],
            normal_const: inp.normal_const,
            color: inp.color,
            uv: corner_uv((TILED_UV[2 * k], TILED_UV[2 * k + 1]), cell, base),
        }
    })
}

/// `0x7b2f49..0x7b3041`, spin without flag 0x2000: `c = f32(f32(cos a)·s)` and `s' = f32(f32(sin
/// a)·s)`, corner `k` at `((x·c + cx) − y·s', (x·s' + y·c) + cy, cz)`.
pub fn quad_expand_rotated(inp: &BillboardInputs, angle: f32) -> [QuadVertex; 4] {
    let (sin_a, cos_a) = f64::from(angle).sin_cos();
    let c = (f64::from(cos_a as f32) * f64::from(inp.scale)) as f32;
    let s = (f64::from(sin_a as f32) * f64::from(inp.scale)) as f32;
    let (cf, sf) = (f64::from(c), f64::from(s));
    let (cx, cy) = (f64::from(inp.center[0]), f64::from(inp.center[1]));
    let cell = (f64::from(inp.cell_u), f64::from(inp.cell_v));
    let base = (f64::from(inp.tex_base_u), f64::from(inp.tex_base_v));
    core::array::from_fn(|k| {
        let (xx, yy) = (f64::from(CORNER_XY[2 * k]), f64::from(CORNER_XY[2 * k + 1]));
        QuadVertex {
            pos: [
                (xx * cf + cx - yy * sf) as f32,
                (xx * sf + yy * cf + cy) as f32,
                inp.center[2],
            ],
            normal_const: inp.normal_const,
            color: inp.color,
            uv: corner_uv((CORNER_UV[2 * k], CORNER_UV[2 * k + 1]), cell, base),
        }
    })
}

/// `0x7b2dda..0x7b2df4`: the turn `f32(+0x18c·age)`, negated when flag 0x8000 is set (`test dh,dh;
/// jns`) and bit 5 of the particle's address is (`test bl,0x20`).
pub fn rotation_angle(rate: f32, age: f32, runtime_flags: u32, particle_ptr: u32) -> f32 {
    let angle = (f64::from(rate) * f64::from(age)) as f32;
    if runtime_flags & 0x8000 != 0 && particle_ptr & 0x20 != 0 {
        -angle
    } else {
        angle
    }
}

/// The flat square with spin: the flat inputs, the axis `+0x284` and the turn.
#[derive(Clone, Copy, Debug)]
pub struct TiledRotatedInputs {
    pub base: TiledInputs,
    pub axis: [f32; 3],
    pub angle: f32,
}

/// `0x7b2e00..0x7b2f44`, spin with flag 0x2000: each flat corner through `BuildAxisRotation(axis,
/// angle, 1)`, rows in the register; corner `k` at `(f32(f32(rx·s) + cx), f32(ry·s + cy),
/// f32(f32(rz)·s + cz))`.
pub fn quad_expand_tiled_rotated(inp: &TiledRotatedInputs) -> [QuadVertex; 4] {
    let m = benilla_re_cmath::rotation::axis_angle_mat3(&inp.axis, inp.angle, true).map(f64::from);
    let b = &inp.base;
    let scale = f64::from(b.scale);
    let [cx, cy, cz] = b.center.map(f64::from);
    let cell = (f64::from(b.cell_u), f64::from(b.cell_v));
    let base = (f64::from(b.tex_base_u), f64::from(b.tex_base_v));
    core::array::from_fn(|k| {
        let [c0, c1, c2] = b.corner[k].map(f64::from);
        let rx = m[0] * c0 + m[1] * c1 + m[2] * c2;
        let ry = m[3] * c0 + m[4] * c1 + m[5] * c2;
        let rz = (m[6] * c0 + m[7] * c1 + m[8] * c2) as f32;
        let rxs = (rx * scale) as f32;
        QuadVertex {
            pos: [
                (f64::from(rxs) + cx) as f32,
                (ry * scale + cy) as f32,
                (f64::from(rz) * scale + cz) as f32,
            ],
            normal_const: b.normal_const,
            color: b.color,
            uv: corner_uv((TILED_UV[2 * k], TILED_UV[2 * k + 1]), cell, base),
        }
    })
}

/// What the tail is written from.
#[derive(Clone, Copy, Debug)]
pub struct StretchedInputs {
    pub center: [f32; 3],
    pub scale: f32,
    pub color: u32,
    /// The tail cell's corner, from [`tex_base`] of the second cell.
    pub tex_base_u: f32,
    pub tex_base_v: f32,
    pub cell_u: f32,
    pub cell_v: f32,
    pub normal_const: [u32; 3],
    /// `0x7bcb40` of the particle matrix and `−v`.
    pub vproj: [f32; 3],
    /// `+0xb4`, already capped at the age under flag 0x10000.
    pub len: f32,
}

/// `0x7b3041..0x7b33fa`, flag 0x8: `T = len·vproj` stored as `f32`, the tip `B = c + T` (x and y in
/// the register, z stored as `f32`), `sq = T.y² + T.x²` stored as `f32`, `nf = s/√sq` in the
/// register, `a = f32(nf·T.x)` and `b = nf·T.y`; corners `(c.x − b, c.y + a)`, `(c.x + b, c.y −
/// a)`, `(B.x − b, B.y + a)`, `(B.x + b, B.y − a)`.
pub fn quad_expand_stretched(inp: &StretchedInputs) -> [QuadVertex; 4] {
    let scale = f64::from(inp.scale);
    let [cx, cy, cz] = inp.center.map(f64::from);
    let len = f64::from(inp.len);
    let [lvx, lvy, lvz] = inp.vproj.map(|v| (len * f64::from(v)) as f32);
    let bxf = f64::from(lvx) + cx;
    let byf = f64::from(lvy) + cy;
    let bz = (f64::from(lvz) + cz) as f32;
    let sq = (f64::from(lvy) * f64::from(lvy) + f64::from(lvx) * f64::from(lvx)) as f32;
    let nf = scale / f64::from(sq).sqrt();
    let pxf = f64::from((nf * f64::from(lvx)) as f32);
    let pyf = nf * f64::from(lvy);
    let cell = (f64::from(inp.cell_u), f64::from(inp.cell_v));
    let base = (f64::from(inp.tex_base_u), f64::from(inp.tex_base_v));
    let mk = |x: f64, y: f64, z: f32, uv: (f32, f32)| QuadVertex {
        pos: [x as f32, y as f32, z],
        normal_const: inp.normal_const,
        color: inp.color,
        uv: corner_uv(uv, cell, base),
    };
    [
        mk(cx - pyf, cy + pxf, cz as f32, (0.0, 0.0)),
        mk(cx + pyf, cy - pxf, cz as f32, (0.0, 1.0)),
        mk(bxf - pyf, byf + pxf, bz, (1.0, 0.0)),
        mk(bxf + pyf, byf - pxf, bz, (1.0, 1.0)),
    ]
}

/// `0x7b310c..0x7b311a`: the tail is a square when the squared length, compared in the register
/// after `fst`, is under [`STRETCH_EPS`] or NaN (`test ah,1; jne`).
pub fn quad_expand_stretched_is_degenerate(inp: &StretchedInputs) -> bool {
    let len = f64::from(inp.len);
    let lvx = (len * f64::from(inp.vproj[0])) as f32;
    let lvy = (len * f64::from(inp.vproj[1])) as f32;
    let sq = f64::from(lvy) * f64::from(lvy) + f64::from(lvx) * f64::from(lvx);
    sq < f64::from(STRETCH_EPS) || sq.is_nan()
}

/// `0x7b33fa`: the square a short tail draws, with the tail cell.
pub fn quad_expand_stretched_degenerate(inp: &BillboardInputs) -> [QuadVertex; 4] {
    quad_expand_billboard(inp)
}
