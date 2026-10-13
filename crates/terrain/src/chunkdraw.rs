//! The arithmetic of the chunk and liquid draw passes, `CWorld::Initialize`'s doodad distance,
//! `CMapChunk::Update` (`systems/terrain/cull.md#chunk-level-and-idle-release`) and the particle
//! quads of `DrawDriftCloud` (`0x68efe0`).
//!
//! The `faddp` sums stay in the register at 53 bits; only the stores round.

/// `[0x7ffa24]`: 0.5.
pub const HALF: f32 = 0.5;
/// `[0x867958]`: 70.0, the detail doodad distance.
pub const DOODAD_DIST: f32 = f32::from_bits(0x428c_0000);

/// One component of a position less the camera's (`0xc7cf20`, `0xc7cf24`, `0xc7cf28`), `fld; fsub;
/// fstp`: in `DrawChunkPass` (`0x684510`) at `0x684693`, `0x6846a6` and `0x6846b2`, and in
/// `DrawRiverPass` (`0x6851b0`) at `0x6854f9`, `0x68550a` and `0x685512`.
pub fn cam_relative_delta(coord: f32, cam: f32) -> f32 {
    (f64::from(coord) - f64::from(cam)) as f32
}

/// `CWorld::Initialize` (`0x66f6c0`) at `0x66f850`: `f32(70·70)` stored to `0xc62514`, which
/// nothing reads.
pub fn doodad_dist_sq() -> f32 {
    (f64::from(DOODAD_DIST) * f64::from(DOODAD_DIST)) as f32
}

/// `CMapChunk::Update` (`0x6afad0`) from `0x6afad6` to `0x6afb03`: the chunk's depth against the
/// view plane, `f32((((nb·vb + nc·vc) + na·va) + d) − e)` with the plane `0xc7bcb0` to `0xc7bcbc`
/// and the chunk's `+0x5c` to `+0x68`, stored to `+0x78`.
#[allow(clippy::too_many_arguments)]
pub fn plane_signed_dist(
    n_a: f32,
    n_b: f32,
    n_c: f32,
    d: f32,
    v_a: f32,
    v_b: f32,
    v_c: f32,
    e: f32,
) -> f32 {
    let p1 = f64::from(n_b) * f64::from(v_b);
    let p2 = f64::from(n_c) * f64::from(v_c);
    let mut acc = p1 + p2;
    let p3 = f64::from(n_a) * f64::from(v_a);
    acc += p3;
    acc += f64::from(d);
    acc -= f64::from(e);
    acc as f32
}

/// `0x6afad0` at `0x6afbeb`, `0x6afc8a`, `0x6afc95` and `0x6afc9a`: `f32(delta + slot)`, the frame
/// time `[0xc62510]` added to a liquid slot's idle time (`+0x30`) or to the chunk's `+0x6c` to
/// `+0x74`.
pub fn accumulate_add(delta: f32, slot: f32) -> f32 {
    (f64::from(delta) + f64::from(slot)) as f32
}

/// `0x6afad0` from `0x6afd3d` to `0x6afd6f`: one component of the centre, `f32((lo + hi)·0.5)`.
pub fn center_half(lo: f32, hi: f32) -> f32 {
    let sum = f64::from(lo) + f64::from(hi);
    (sum * f64::from(HALF)) as f32
}

/// `DrawDriftCloud` (`0x68efe0`) at `0x68f181`, `0x68f199` and `0x68f1b1`: one row of the view
/// matrix times the particle's direction (`[esi-4]`, `[esi]`, `[esi+4]`), `f32((m_hi·c1 + m_mid·c0)
/// + m_lo·c_m1)`.
pub fn billboard_dot(m_hi: f32, m_mid: f32, m_lo: f32, c1: f32, c0: f32, c_m1: f32) -> f32 {
    let t1 = f64::from(m_hi) * f64::from(c1);
    let t2 = f64::from(m_mid) * f64::from(c0);
    let mut acc = t1 + t2;
    let t3 = f64::from(m_lo) * f64::from(c_m1);
    acc += t3;
    acc as f32
}

/// `0x68efe0` at `0x68f1ee`: the third dot negated, `fld; fchs; fstp [ebp-0x18]`, the cone test's
/// lower bound.
pub fn cone_negate(fvar6: f32) -> f32 {
    (-f64::from(fvar6)) as f32
}

/// `0x68efe0` from `0x68f240` to `0x68f269`: one corner component, `f32(tex·size + dot)`, the size
/// `[esi+8]` and the dot `[ebp-0x10]` for x or `[ebp-0xc]` for y.
pub fn quad_corner(tex: f32, size: f32, dot: f32) -> f32 {
    (f64::from(tex) * f64::from(size) + f64::from(dot)) as f32
}
