//! The ribbon emitters (`systems/models/ribbons.md`): `CRibbonEmitter::Initialize` (`0x7b73a0`),
//! `CRibbonEmitter::SetColor` (`0x7b7a80`), `CRibbonEmitter::SetAlpha` (`0x7b7b10`),
//! `CRibbonEmitter::SetFrame` (`0x7b76c0`), `CRibbonEmitter::BuildNodeFrame` (`0x7b6990`),
//! `CRibbonEmitter::EmitSegment` (`0x7b6c30`), `RibbonRingAdvance` (`0x7b6f00`),
//! `CRibbonEmitter::SetTextureCell` (`0x7b6f30`), `CRibbonEmitter::Update` (`0x7b7e60`),
//! `CRibbonEmitter::Render` (`0x7b80c0`) and `CRibbonEmitter::TransformVertices` (`0x7b7bc0`,
//! `systems/models/model.md#reference-frame`).

use crate::x87::ftol;
use benilla_re_cmath::transform::transform_point as transform_point4;

/// 0.0 (`0x7ffd74`).
pub const ZERO: f32 = 0.0;
/// 1.0 (`0x7ff9d8`; `0x81d87c` in the node frame).
pub const ONE: f32 = 1.0;
/// 0.25 (`0x81d878`): the least lifespan.
pub const SPAN_CLAMP: f32 = f32::from_bits(0x3e80_0000);
/// 2.0 (`0x801628`).
pub const TWO: f64 = 2.0;
/// 255.0 (`0x7ffe58`).
const F255: f64 = 255.0;
/// 0.5 (`0x7ffa24`).
const HALF: f64 = 0.5;

/// `0x7b73a0`'s lifespan: raised to 0.25 when 0.25 compares above it (`test ah,0x41; jne`, so NaN
/// stays).
pub fn clamp_span(span: f32) -> f32 {
    if span < SPAN_CLAMP {
        SPAN_CLAMP
    } else {
        span
    }
}

/// `0x7b73dd..0x7b73f1`: the ring size `ftol(ceil(rate·L) + 2.0)` with `rate` the stored
/// `ceil(+0x94)` and `L` the clamped lifespan (`ceil` is `0x73fdf5`).
pub fn init_buffer_count(fv11: f32, span: f32) -> i32 {
    ftol((f64::from(fv11) * f64::from(span)).ceil() + TWO)
}

/// What `0x7b74f5..0x7b7562` stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RibbonFractions {
    /// `+0x58`: `1/L`.
    pub inv_span: f32,
    /// `+0x5c`: `(p3 − p1)/n10` (`fidiv`).
    pub step_uv: f32,
    /// `+0x60`: `(p2 − p0)/n9`.
    pub step_pos: f32,
    /// `+0x64`, `+0x68`: `1/` the stored `+0x5c` and `+0x60`.
    pub inv_step_uv: f32,
    pub inv_step_pos: f32,
}

/// `0x7b74f5..0x7b7562`: from `L`, the rectangle `p` and the grid sizes `n9`, `n10`.
pub fn ribbon_fractions(span: f32, p: [f32; 4], n9: i32, n10: i32) -> RibbonFractions {
    let one = f64::from(ONE);
    let step_uv = ((f64::from(p[3]) - f64::from(p[1])) / f64::from(n10)) as f32;
    let step_pos = ((f64::from(p[2]) - f64::from(p[0])) / f64::from(n9)) as f32;
    RibbonFractions {
        inv_span: (one / f64::from(span)) as f32,
        step_uv,
        step_pos,
        inv_step_uv: (one / f64::from(step_uv)) as f32,
        inv_step_pos: (one / f64::from(step_pos)) as f32,
    }
}

/// `x·255 + 0.5` through `__ftol`, the low byte.
fn ftol_byte(x: f64) -> u8 {
    ftol(x) as u8
}

/// `0x7b7a80`: the colour word `+0x12c` from three floats and the stored alpha byte `a`, whose
/// byte is `ftol(a·255·255 + 0.5)`: `(a << 24) | (r << 16) | (g << 8) | b`.
pub fn ribbon_color_pack(r: f32, g: f32, b: f32, alpha: u8) -> u32 {
    let a = u32::from(ftol_byte(f64::from(alpha) * F255 * F255 + HALF));
    let [rb, gb, bb] = [r, g, b].map(|c| u32::from(byte_setter(c)));
    (a << 24) | (rb << 16) | (gb << 8) | bb
}

/// `0x7b7b10`: the alpha byte `+0x12f`, `ftol(v·255 + 0.5)`.
pub fn byte_setter(v: f32) -> u8 {
    ftol_byte(f64::from(v) * F255 + HALF)
}

/// `0x7b76e8..0x7b770a`: the matrix's row 3 plus the offset, each stored as `f32`.
pub fn node_place_translate(mat_t: [f32; 3], vec: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| (f64::from(mat_t[k]) + f64::from(vec[k])) as f32)
}

/// `0x7b815d..0x7b8180`: the frame's row 3 less the offset `+0x2c`, each stored as `f32`.
pub fn render_world_translate(mat_t: [f32; 3], origin: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| (f64::from(mat_t[k]) - f64::from(origin[k])) as f32)
}

/// The two nodes `0x7b6990` reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RibbonFrame {
    /// `+0x14c`: the current position.
    pub p0: [f32; 3],
    /// `+0x20`: the last position.
    pub p1: [f32; 3],
    /// `+0x7c`, `+0x88`: the last and current up.
    pub va: [f32; 3],
    pub vb: [f32; 3],
    /// `+0x94`, `+0xa0`: the last and current forward.
    pub wc: [f32; 3],
    pub wd: [f32; 3],
    /// `+0x160`: below.
    pub s: f32,
    /// `+0x15c`: above.
    pub s2: f32,
}

/// What `0x7b6990` stores.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RibbonFrameOut {
    /// `+0xc4`, `+0xd0`: the bottoms `p1 − s·va` and `p0 − s·vb`.
    pub corner0: [f32; 3],
    pub corner1: [f32; 3],
    /// `+0xdc`, `+0xe8`: the tops `p1 + s2·va` and `p0 + s2·vb`.
    pub corner2: [f32; 3],
    pub corner3: [f32; 3],
    /// `+0xac`, `+0xb8`: the tangents `len·wc` and `len·wd`.
    pub width_c: [f32; 3],
    pub width_d: [f32; 3],
}

/// `0x7b6990` (ecx = the ribbon): `d = p1 − p0` stored as `f32`, `len = f32(√((x² + y²) + z²)·1.0)`
/// (`0x81d87c`), the ends with each scaled vector stored as `f32` before the sum, except the top
/// of the last node, `s2·va + p1`, in one register expression; the tangents `len·w`.
pub fn ribbon_frame_build(f: RibbonFrame) -> RibbonFrameOut {
    let s = f64::from(f.s);
    let s2 = f64::from(f.s2);
    let d = [0, 1, 2].map(|k| (f64::from(f.p1[k]) - f64::from(f.p0[k])) as f32);
    let [dx, dy, dz] = d.map(f64::from);
    let len = (((dx * dx + dy * dy) + dz * dz).sqrt() * f64::from(ONE)) as f32;
    let corner_stored = |base: [f32; 3], scale: f64, v: [f32; 3], add: bool| {
        [0, 1, 2].map(|i| {
            let prod = (scale * f64::from(v[i])) as f32;
            if add {
                (f64::from(prod) + f64::from(base[i])) as f32
            } else {
                (f64::from(base[i]) - f64::from(prod)) as f32
            }
        })
    };
    let scale_len = |w: [f32; 3]| w.map(|c| (f64::from(len) * f64::from(c)) as f32);
    RibbonFrameOut {
        corner0: corner_stored(f.p1, s, f.va, false),
        corner1: corner_stored(f.p0, s, f.vb, false),
        corner2: [0, 1, 2].map(|i| (s2 * f64::from(f.va[i]) + f64::from(f.p1[i])) as f32),
        corner3: corner_stored(f.p0, s2, f.vb, true),
        width_c: scale_len(f.wc),
        width_d: scale_len(f.wd),
    }
}

/// The node frame `0x7b6c30` reads back (`+0xac` to `+0xf0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RibbonEmitFrame {
    pub width_c: [f32; 3],
    pub width_d: [f32; 3],
    pub corner0: [f32; 3],
    pub corner1: [f32; 3],
    pub corner2: [f32; 3],
    pub corner3: [f32; 3],
}

/// What `0x7b6c30` writes: the bottom vertex at the head record `+0x0`, the top at `+0x14`, and
/// the age at `+0xc + 4·head`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct RibbonSegmentOut {
    pub vert_a: [f32; 3],
    pub vert_b: [f32; 3],
    pub tex: f32,
}

/// An index of the ring and the ring's size `+0x8`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RibbonLiveIndex {
    pub index: u32,
    pub cap: u32,
}

/// `0x7b6f00`: the index plus the step, less the size once when not below it (`jb`).
pub fn ribbon_live_index_advance(li: RibbonLiveIndex, inc: u32) -> u32 {
    let next = li.index.wrapping_add(inc);
    if next >= li.cap {
        next.wrapping_sub(li.cap)
    } else {
        next
    }
}

/// `0x7b6c30` (ecx = the ribbon; age, `t`, commit; `ret 0xc`): with `s = 1 − t`, the bottom
/// `t·(P1 − s·T1) + s·(P0 + t·T0)` and the top the same about `+0xe8` and `+0xdc`, every product
/// and sum stored as `f32`. The bottom's `s·T1` takes the `1 − t` left in the register; every
/// other `s` its `f32` copy.
pub fn ribbon_segment_emit(f: RibbonEmitFrame, tex: f32, t: f32) -> RibbonSegmentOut {
    let w_reg = f64::from(ONE) - f64::from(t);
    let w_f32 = f64::from(w_reg as f32);
    let tf = f64::from(t);
    let scale = |k: f64, v: [f32; 3]| v.map(|c| (k * f64::from(c)) as f32);
    let sub =
        |a: [f32; 3], b: [f32; 3]| [0, 1, 2].map(|i| (f64::from(a[i]) - f64::from(b[i])) as f32);
    let add =
        |a: [f32; 3], b: [f32; 3]| [0, 1, 2].map(|i| (f64::from(a[i]) + f64::from(b[i])) as f32);
    let a_c = scale(tf, sub(f.corner1, scale(w_reg, f.width_d)));
    let a_f = scale(w_f32, add(scale(tf, f.width_c), f.corner0));
    let b_c = scale(tf, sub(f.corner3, scale(w_f32, f.width_d)));
    let b_f = scale(w_f32, add(scale(tf, f.width_c), f.corner2));
    RibbonSegmentOut {
        vert_a: add(a_c, a_f),
        vert_b: add(b_f, b_c),
        tex,
    }
}

/// What `0x7b6f30` reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileUvIn {
    /// `+0x158`: the cell.
    pub frame: u32,
    /// `+0x144`: the columns.
    pub cols: u32,
    /// `+0x5c`, `+0x60`: the cell size.
    pub cell_w: f32,
    pub cell_h: f32,
    /// `+0x134`, `+0x130`: the rectangle's corner.
    pub base_u: f32,
    pub base_v: f32,
}

/// What `0x7b6f30` stores: `u0` `+0x70`, `v0` `+0x6c`, `u1` `+0x78`, `v1` `+0x74`.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TileUvRect {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

/// `0x7b6f30`: `col = c mod C`, `row = c / C` (unsigned `div`), `u0 = col·w + base_u` and `v0 =
/// row·h + base_v` stored with `fst`, and `u1`, `v1` adding the size to the register values.
pub fn particle_tile_uv_rect(t: TileUvIn) -> TileUvRect {
    let col = t.frame % t.cols;
    let row = t.frame / t.cols;
    let u0 = f64::from(col) * f64::from(t.cell_w) + f64::from(t.base_u);
    let v0 = f64::from(row) * f64::from(t.cell_h) + f64::from(t.base_v);
    TileUvRect {
        u0: u0 as f32,
        v0: v0 as f32,
        u1: (u0 + f64::from(t.cell_w)) as f32,
        v1: (v0 + f64::from(t.cell_h)) as f32,
    }
}

/// `0x7b7e69..0x7b7e8c` and `0x7b8099`: `dt` is 0 when negative (`test ah,5; jp`), kept when
/// below `L` (`test ah,5; jnp`), and `L` otherwise, a NaN `dt` included.
pub fn dt_clamp(dt: f32, life: f32) -> f32 {
    if dt < ZERO {
        ZERO
    } else if dt < life {
        dt
    } else {
        life
    }
}

/// `0x7b7e9b..0x7b7ea1`: the tail edge's age plus `dt`, kept in the register.
pub fn expire_sum(dt: f32, edge_age: f32) -> f64 {
    f64::from(dt) + f64::from(edge_age)
}

/// `0x7b7ea1..0x7b7eac`: the tail edge is dropped when [`expire_sum`] compares above `L` (`test
/// ah,0x41; jne`, so NaN keeps it).
pub fn edge_expired(dt: f32, edge_age: f32, life: f32) -> bool {
    expire_sum(dt, edge_age) > f64::from(life)
}

/// `0x7b7ee0..0x7b7ef5`: `e = dt·rate + carry` (`+0xf4`, `+0x1c`), stored as `f32`.
pub fn interp_factor(dt: f32, rate: f32, phase: f32) -> f32 {
    (f64::from(dt) * f64::from(rate) + f64::from(phase)) as f32
}

/// `0x7b7f0d..0x7b7f1c`: `r = 1/(e − carry)`, stored as `f32`.
pub fn recip_span(factor: f32, phase: f32) -> f32 {
    (f64::from(ONE) / (f64::from(factor) - f64::from(phase))) as f32
}

/// `0x7b7f1f..0x7b7f28`: `e − 1`, passed to `floor` as a double.
pub fn floor_arg(factor: f32) -> f64 {
    f64::from(factor) - f64::from(ONE)
}

/// `0x7b7f40..0x7b7f4b`: `t_j = (j − carry)·r`, stored as `f32`.
pub fn segment_fraction(counter: f32, phase: f32, recip: f32) -> f32 {
    ((f64::from(counter) - f64::from(phase)) * f64::from(recip)) as f32
}

/// `0x7b7f51..0x7b7f5d`: the age `−(t_j·dt)` passed to `0x7b6c30`.
pub fn segment_emit_arg(segment_fraction: f32, dt: f32) -> f32 {
    (-(f64::from(segment_fraction) * f64::from(dt))) as f32
}

/// `0x7b7f66..0x7b7f6f`: `j + 1`.
pub fn next_counter(counter: f32) -> f32 {
    (f64::from(counter) + f64::from(ONE)) as f32
}

/// `0x7b7f75..0x7b7f95`: the carry `e − floor(e)` (`floor` is `0x73ff3f`), stored as `f32`.
pub fn phase_frac(factor: f32) -> f32 {
    let f = f64::from(factor);
    (f - f.floor()) as f32
}

/// `0x7b8002..0x7b8015`: `((age + age) + dt)·dt·gravity` (`+0x164`), kept in the register.
pub fn gravity_term(edge_age: f32, dt: f32, gravity: f32) -> f64 {
    let a = f64::from(edge_age);
    ((a + a) + f64::from(dt)) * f64::from(dt) * f64::from(gravity)
}

/// `0x7b8015..0x7b8020`: the term added to both z values (`+0x8`, `+0x1c`), each stored as `f32`.
pub fn add_gravity(field_v8: f32, field_v1c: f32, term: f64) -> (f32, f32) {
    (
        (f64::from(field_v8) + term) as f32,
        (f64::from(field_v1c) + term) as f32,
    )
}

/// `0x7b8026..0x7b802e`: the age plus `dt`, after the gravity read the old age.
pub fn advance_sim(edge_age: f32, dt: f32) -> f32 {
    (f64::from(dt) + f64::from(edge_age)) as f32
}

/// `0x7b8030..0x7b803c`: `u = age·+0x5c·+0x58 + +0x70` of the new age, kept in the register.
pub fn record_value(age_updated: f32, c5c: f32, c58: f32, b70: f32) -> f64 {
    f64::from(age_updated) * f64::from(c5c) * f64::from(c58) + f64::from(b70)
}

/// `0x7b803f..0x7b8066`: the edge's texture words: `u` (`fst`, then the register stored again),
/// `v0` `+0x6c`, `u`, `v1` `+0x74`.
pub fn record_dwords(v: f64, f6c: f32, f74: f32) -> (f32, f32, f32, f32) {
    (v as f32, f6c, v as f32, f74)
}

/// One component of `0x7b7bc0`'s turn: `(a0·b0 + a1·b1) + a2·b2` in the register.
fn rot_component(a0: f32, b0: f32, a1: f32, b1: f32, a2: f32, b2: f32) -> f32 {
    let p = |a: f32, b: f32| f64::from(a) * f64::from(b);
    ((p(a0, b0) + p(a1, b1)) + p(a2, b2)) as f32
}

/// The four direction vectors `0x7b7bc0` turns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bases {
    /// `+0x94`.
    pub b94: [f32; 3],
    /// `+0x7c`.
    pub b7c: [f32; 3],
    /// `+0xa0`.
    pub ba0: [f32; 3],
    /// `+0x88`.
    pub b88: [f32; 3],
}

/// `0x7b7bda..0x7b7dd6`: the four vectors through the matrix's 3×3 (`mRC = m[4·C + R]`), each
/// component's terms in the exe's order.
pub fn rotate_bases(b: Bases, m: &[f32; 16]) -> Bases {
    let (m00, m01, m02) = (m[0], m[4], m[8]);
    let (m10, m11, m12) = (m[1], m[5], m[9]);
    let (m20, m21, m22) = (m[2], m[6], m[10]);
    let v = b.b94;
    let b94 = [
        rot_component(v[1], m01, v[2], m02, v[0], m00),
        rot_component(v[2], m12, v[1], m11, v[0], m10),
        rot_component(v[2], m22, v[0], m20, v[1], m21),
    ];
    let v = b.b7c;
    let b7c = [
        rot_component(v[2], m02, v[1], m01, v[0], m00),
        rot_component(v[2], m12, v[1], m11, v[0], m10),
        rot_component(v[0], m20, v[2], m22, v[1], m21),
    ];
    let v = b.ba0;
    let ba0 = [
        rot_component(v[1], m01, v[2], m02, v[0], m00),
        rot_component(v[2], m12, v[1], m11, v[0], m10),
        rot_component(v[0], m20, v[2], m22, v[1], m21),
    ];
    let v = b.b88;
    let b88 = [
        rot_component(v[0], m00, v[2], m02, v[1], m01),
        rot_component(v[0], m10, v[2], m12, v[1], m11),
        rot_component(v[2], m22, v[1], m21, v[0], m20),
    ];
    Bases { b94, b7c, ba0, b88 }
}

/// `0x7b7bc0`'s points (`+0x20`, `+0x14c` and each edge vertex) through `TransformPointInPlace`
/// (`0x7bcc60`).
pub fn transform_point(p: [f32; 3], m: &[f32; 16]) -> [f32; 3] {
    transform_point4(&p, m)
}
