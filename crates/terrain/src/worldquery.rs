//! The arithmetic of the world queries: `GatherWorldFacets` (`0x6aa8b0`,
//! `systems/terrain/collide.md#terrain-gather`), `GetSurfaceHeight` (`0x6b7070`,
//! `systems/terrain/queries.md#surface-height`), `TraceMapObjectFloor` (`0x6b7790`) and `TestChunkBox`
//! (`0x6ad610`, `systems/terrain/collide.md#other-world-queries`).

use crate::worldcoord::MAP_CENTER_F32;
use crate::x87::fistp;

/// `[0x810ae4]` and `[0x810d2c]`: 0.24 (`0x3e75c28f`), units to cells.
pub const CELL_SCALE: f32 = f32::from_bits(0x3e75_c28f);
/// `[0x86aa2c]` and `[0x86b6c0]`: 0.5, subtracted before `fistp`.
const FISTP_BIAS: f32 = 0.5;

/// `GatherWorldFacets` (`0x6aa8b0`) from `0x6aa930` to `0x6aa967`: `−(c − 17066.666)` for the box's
/// max x (`+0xc`), max y (`+0x10`), min x (`+0`) and min y (`+4`), stored in that order to
/// `[ebp-0x34]` up. Max y is stored with `fst` and its unrounded value is the first bounds compare.
pub fn aabb_query_negates(corners: [f32; 4]) -> [f32; 4] {
    let center = f64::from(MAP_CENTER_F32);
    corners.map(|c| (-(f64::from(c) - center)) as f32)
}

/// `0x6aa8b0` from `0x6aa9b4` to `0x6aaa14`: each of those stored values times 0.24, stored and
/// reloaded, less 0.5 and `fistp`. In and out in the order of `aabb_query_negates`; the client writes
/// them to `[ebp-0x8]`, `[ebp-0xc]`, `[ebp-0x10]` and `[ebp-0x14]`.
pub fn aabb_query_cell_indices(negs: [f32; 4]) -> [i32; 4] {
    let scale = f64::from(CELL_SCALE);
    let bias = f64::from(FISTP_BIAS);
    negs.map(|n| {
        let prod = (scale * f64::from(n)) as f32;
        fistp(f64::from(prod) - bias)
    })
}

/// `GetSurfaceHeight` (`0x6b7070`) from `0x6b70c1` to `0x6b7100`: per axis `−(p − 17066.666)·0.24`
/// in the register, stored, less 0.5 and `fistp`; y first. Returns `[y, x]`.
pub fn sphere_cell_indices(px: f32, py: f32) -> [i32; 2] {
    let center = f64::from(MAP_CENTER_F32);
    let scale = f64::from(CELL_SCALE);
    let bias = f64::from(FISTP_BIAS);
    let n_y = ((-(f64::from(py) - center)) * scale) as f32;
    let n_x = ((-(f64::from(px) - center)) * scale) as f32;
    [fistp(f64::from(n_y) - bias), fistp(f64::from(n_x) - bias)]
}

/// `0x6b7070` at `0x6b71a9`: the segment's top, `range + z`, stored to `[ebp-0x24]`.
pub fn sphere_seg_top(z_query: f32, pt_z: f32) -> f32 {
    (f64::from(z_query) + f64::from(pt_z)) as f32
}

/// `0x6b7070` at `0x6b71bd`: the segment's bottom, `z − range`, stored to `[ebp-0x30]`.
pub fn sphere_seg_height(z_query: f32, pt_z: f32) -> f32 {
    (f64::from(pt_z) - f64::from(z_query)) as f32
}

/// `0x6b7070` from `0x6b7225` to `0x6b7237`, taken when the hit fraction is below 1.0: `top −
/// (range + range)·t` in the register (`fadd st,st`), stored to the output. The unrounded value is
/// then compared with the terrain height.
pub fn sphere_compose_z(z_query: f32, t: f32, seg_top: f32) -> f32 {
    let two_z = f64::from(z_query) + f64::from(z_query);
    (f64::from(seg_top) - two_z * f64::from(t)) as f32
}

/// `TraceMapObjectFloor` (`0x6b7790`, edx = the segment's top, the bottom on the stack) from
/// `0x6b779f` to `0x6b77b5`: `1/(top.z − bottom.z)`, stored to `[ebp-0x18]` with `fst` and to
/// `[ebp-0x8]` with `fstp`, the same `f32` in both. `a_z` is the bottom, `b_z` the top.
pub fn ray_aabb_inv(a_z: f32, b_z: f32) -> f32 {
    (1.0_f64 / (f64::from(b_z) - f64::from(a_z))) as f32
}

/// `0x6b7790` from `0x6b7885` to `0x6b789c`, and the same bytes on other slots from `0x6b7a2c`: the
/// hit converted along the segment, `maxt − ((1 − inv·t)·inv + inv)` in the register, stored to
/// `[edi]` with `fst`.
pub fn ray_t_attenuation(inv: f32, t: f32, maxt: f32) -> f32 {
    let i = f64::from(inv);
    let acc = (1.0_f64 - i * f64::from(t)) * i + i;
    (f64::from(maxt) - acc) as f32
}

/// `TestChunkBox` (`0x6ad610`) from `0x6ad676` to `0x6ad6b0`: the copied box's first three floats
/// plus the chunk's origin `[ebx+0x6c]` negated, the negation stored and reloaded, each sum stored.
pub fn chunk_box_translate(box3: [f32; 3], origin3: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|c| {
        let neg = (-f64::from(origin3[c])) as f32;
        (f64::from(box3[c]) + f64::from(neg)) as f32
    })
}
