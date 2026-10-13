//! The coordinate arithmetic of `IsInChunkShadow` (`0x69b350`,
//! `systems/terrain/queries.md#shadow-test`), `ProbeGroundBelow` (`0x672b60`) and
//! `BuildProjectorBasis` (`0x687080`).

use crate::x87::fistp;

/// `[0x7ffab4]`: 17066.666 (`0x46855555`).
pub const MAP_CENTER_F32: f32 = f32::from_bits(0x4685_5555);
/// `[0x810740]`: 0.03 (`0x3cf5c28f`), units to chunks.
pub const TILE_SCALE_COARSE: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x810744]`: 1.92 (`0x3ff5c28f`), units to shadow texels.
pub const TILE_SCALE_FINE: f32 = f32::from_bits(0x3ff5_c28f);
/// `[0x86a750]`: 0.5, subtracted before `fistp`.
const TILE_BIAS: f32 = 0.5;
/// `[0x804478]`: 20.0, how far below the probe reaches.
pub const PROBE_DEPTH: f32 = 20.0;

/// `0x69b350` from `0x69b356`: `−(p − 17066.666)` per axis, stored.
pub fn tile_neg_axis(p: f32) -> f32 {
    (-(f64::from(p) - f64::from(MAP_CENTER_F32))) as f32
}

/// `0x69b350` at `0x69b3c1` and `0x69b3da`: the chunk, `fistp(f32(0.03·n) − 0.5)`, the product stored
/// and reloaded.
pub fn tile_index_coarse(neg: f32) -> i32 {
    let prod = (f64::from(TILE_SCALE_COARSE) * f64::from(neg)) as f32;
    fistp(f64::from(prod) - f64::from(TILE_BIAS))
}

/// `0x69b350` at `0x69b42f` and `0x69b448`: the texel, `fistp(f32(1.92·n) − 0.5)`.
pub fn tile_index_fine(neg: f32) -> i32 {
    let prod = (f64::from(TILE_SCALE_FINE) * f64::from(neg)) as f32;
    fistp(f64::from(prod) - f64::from(TILE_BIAS))
}

/// `ProbeGroundBelow` (`0x672b60`) at `0x672b77`: the trace's end, `z − 20.0`, stored.
pub fn probe_segment_bottom_z(pt_z: f32) -> f32 {
    (f64::from(pt_z) - f64::from(PROBE_DEPTH)) as f32
}

/// `0x672b60` from `0x672c1c`: the height at the hit, `(end − start)·t + start` in the register, to
/// `[edi]`.
pub fn probe_lerp_z(end_z: f32, start_z: f32, t: f32) -> f32 {
    ((f64::from(end_z) - f64::from(start_z)) * f64::from(t) + f64::from(start_z)) as f32
}

/// `BuildProjectorBasis` (`0x687080`): the rows `this+0x84`, `this+0x6c` and `this+0x90` less the
/// base `this+0x60`, each component stored, to rows 0 to 2 of the matrix.
pub fn projector_basis_rows(corners: [[f32; 3]; 3], base: [f32; 3]) -> [[f32; 3]; 3] {
    corners.map(|row| std::array::from_fn(|c| (f64::from(row[c]) - f64::from(base[c])) as f32))
}
