//! A group's liquid grid (`systems/models/wmo.md#queries`): the height query
//! `CMapObjGroup::LiquidHeight` (`0x6b9f10`), and the arithmetic of the vertex fill `CMapObjGroup::BuildLiquidVertices` (`0x6ba9b0`), the ray test
//! `CMapObjGroup::RayHitLiquid` (`0x6baa70`) and the collectors
//! `CMapObjGroup::CollectLiquidSegment` (`0x6bb170`) and `CMapObjGroup::CollectLiquidVolume`
//! (`0x6bb6b0`).
//!
//! The grid is anchored at the group's `+0x108`, `+0x10c`; its vertices are 4.1666665 apart and a
//! coordinate maps to a cell as `floor(f32(0.24·(coord − base)))`, with 0.24 the global `0xce26dc`.
//! The group's grid fields: `+0xf8` vertices per row, `+0x100` and `+0x104` the cell columns and
//! rows (`+0x100` also the tile stride), `+0x118` the 8-byte vertex records (height at `+4`),
//! `+0x11c` the tile bytes.

use crate::x87::floor_ftol;

/// 4.1666665 (`0x810d84`, `0x40855555`): the spacing of the grid's vertices.
pub const LIQUID_TILE_SIZE: f32 = f32::from_bits(0x4085_5555);

/// `0xce26dc`: `1/4.1666665` stored as `f32` by `InitMapObjGroupInvTileSize` (`0x6bcf90`),
/// 0.24000001 (`0x3e75c290`).
pub const MLIQ_GRID_STEP: f32 = f32::from_bits(0x3e75_c290);

/// 0.01 (`0x8029d0`): the margin of the height test on a tile of type 0.
pub const MLIQ_HEIGHT_EPS: f32 = f32::from_bits(0x3c23_d70a);

/// 0.019444443 (`0x8101b8`): the bias added to a height difference before its sign is read.
pub const MLIQ_STRADDLE_BIAS: f32 = f32::from_bits(0x3c9f_49f4);

/// `0x6bcf90`: `1.0/4.1666665` divided in the register and stored as `f32`.
pub fn inv_tile_size() -> f32 {
    (1.0 / f64::from(LIQUID_TILE_SIZE)) as f32
}

/// A coordinate on the grid: `g = f32(step·(coord − base))` with the difference in the register,
/// the cell `floor(g)` through `__ftol` (a value past `i32` keeps its low dword), and
/// `frac = f32(g − cell)`. Returns `(g, cell, frac)`.
pub fn mliq_grid_coord(coord: f32, base: f32, grid_step: f32) -> (f32, i32, f32) {
    let g = (f64::from(grid_step) * (f64::from(coord) - f64::from(base))) as f32;
    let cell = floor_ftol(g);
    let frac = (f64::from(g) - f64::from(cell)) as f32;
    (g, cell, frac)
}

/// The result of [`mliq_liquid_height`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MliqHeight {
    /// The height, stored through the third argument on a hit.
    pub height: f32,
    /// The type, stored through the second argument on a hit.
    pub liquid_type: u32,
    /// The return value.
    pub hit: bool,
}

/// `0x6b9f10(pos, type_out, height_out, normal_out)`, on a group whose `+0x10` has 0x1000 (a
/// group without it returns 0).
///
/// A group whose `+0x38` is not 0xf returns 1 with that type and the height `FLT_MAX`. Otherwise
/// the point's cell is found as in [`mliq_grid_coord`]; outside the grid, or on a tile whose low
/// nibble is 0xf, it returns 0. The nibble's low two bits pick the case through the tables at
/// `0x6ba160` and `0x6ba16c`: 1 returns 0; 0, 2 and 3 interpolate the four corner heights
/// bilinearly in the register,
/// `h = r0 + (r1 − r0)·fy` with `r0 = h00 + (h10 − h00)·fx` and `r1 = (h11 − h01)·fx + h01`, and
/// hit when `pos.z < 0.01 + h` (type 0, `0x6ba08d`) or `pos.z < h` (types 2 and 3, `0x6ba109`),
/// each ordered: a NaN height misses. A hit stores `f32(h)`, the type `nibble & 3` and a zero
/// normal, and returns 1.
#[allow(clippy::too_many_arguments)]
pub fn mliq_liquid_height(
    type_override: u32,
    pos: [f32; 3],
    base: [f32; 2],
    grid_step: f32,
    cols: i32,
    rows: i32,
    tile_stride: i32,
    tile_types: &[u8],
    row: i32,
    heights: &[f32],
    height_eps: f32,
) -> MliqHeight {
    let miss = MliqHeight {
        height: 0.0,
        liquid_type: 0,
        hit: false,
    };
    if type_override != 0xf {
        return MliqHeight {
            height: f32::from_bits(0x7f7f_ffff),
            liquid_type: type_override,
            hit: true,
        };
    }
    let (_, cell_x, frac_x) = mliq_grid_coord(pos[0], base[0], grid_step);
    let (_, cell_y, frac_y) = mliq_grid_coord(pos[1], base[1], grid_step);
    if cell_x < 0 || cell_y < 0 || cell_x >= cols || cell_y >= rows {
        return miss;
    }
    let nibble = u32::from(tile_types[(tile_stride * cell_y + cell_x) as usize] & 0xf);
    if nibble == 0xf {
        return miss;
    }
    let liquid_type = nibble & 3;
    if liquid_type == 1 {
        return miss;
    }

    let idx00 = (row * cell_y + cell_x) as usize;
    let idx01 = idx00 + row as usize;
    let h00 = f64::from(heights[2 * idx00 + 1]);
    let h10 = f64::from(heights[2 * idx00 + 3]);
    let h01 = f64::from(heights[2 * idx01 + 1]);
    let h11 = f64::from(heights[2 * idx01 + 3]);
    let fx = f64::from(frac_x);
    let fy = f64::from(frac_y);
    let row0 = (h10 - h00) * fx + h00;
    let row1 = (h11 - h01) * fx + h01;
    let h = (row1 - row0) * fy + row0;

    let limit = if liquid_type == 0 {
        f64::from(height_eps) + h
    } else {
        h
    };
    MliqHeight {
        height: h as f32,
        liquid_type,
        hit: f64::from(pos[2]) < limit,
    }
}

/// `a − b` stored as `f32`: the grid-relative coordinates of `0x6baa70` (`0x6baa9f` to
/// `0x6bab0d`) and `0x6bb170` (`0x6bb18d` to `0x6bb1df`).
pub fn liquid_sub(a: f32, b: f32) -> f32 {
    (f64::from(a) - f64::from(b)) as f32
}

/// Four coordinates times the grid step: the first product stays in the register and is stored as
/// a `f64` for `floor`, the other three are stored as `f32`. `0x6baa70` from `0x6bab12` to
/// `0x6bab51`, `0x6bb170` from `0x6bb1e4` to `0x6bb223` and `0x6bb6b0` from `0x6bb7d0` to
/// `0x6bb80f`. Returns `(first, [second, third, fourth])`.
pub fn liquid_cell_scale(step: f32, coords: [f32; 4]) -> (f64, [f32; 3]) {
    let g = f64::from(step);
    let first = g * f64::from(coords[0]);
    let rest = [1, 2, 3].map(|i| (g * f64::from(coords[i])) as f32);
    (first, rest)
}

/// `0x6baa70` from `0x6babd0` to `0x6bac4e`: the ray's direction `end − start` stored as `f32`,
/// `inv = f32(1/√((dx² + dy²) + dz²))` with the sum in the register, and each component times
/// `inv` stored as `f32`. Returns `(inv, unit)`.
pub fn mliq_ray_normalize(start: [f32; 3], end: [f32; 3]) -> (f32, [f32; 3]) {
    let dir = [0, 1, 2].map(|i| liquid_sub(end[i], start[i]));
    let [dx, dy, dz] = dir.map(f64::from);
    let inv = (1.0 / ((dx * dx + dy * dy) + dz * dz).sqrt()) as f32;
    let unit = dir.map(|d| (f64::from(d) * f64::from(inv)) as f32);
    (inv, unit)
}

/// `(a − b) + bias` stored as `f32`, whose sign the caller reads: the corner tests of `0x6baa70`
/// (`0x6bad30` to `0x6bad69`) and of `0x6bb170` (`0x6bb353` to `0x6bb386`).
pub fn mliq_straddle_delta(a: f32, b: f32, bias: f32) -> f32 {
    ((f64::from(a) - f64::from(b)) + f64::from(bias)) as f32
}

/// The sign of a corner delta as bit 2: `(bits >> 0x1d) & 4`.
pub fn mliq_straddle_bit_lo(delta: f32) -> u32 {
    (delta.to_bits() >> 0x1d) & 4
}

/// The sign of a corner delta as bit 5: `(bits >> 0x1a) & 0x20`.
pub fn mliq_straddle_bit_hi(delta: f32) -> u32 {
    (delta.to_bits() >> 0x1a) & 0x20
}

/// `0x6baa70` at `0x6baf5e`: a triangle hit's `t` times `inv` (`fmul`), stored as `f32` by `fst`
/// and compared, unrounded, with the best so far. Returns `(stored, compared)`.
pub fn mliq_hit_scale(hit_t: f32, inv_len: f32) -> (f32, f64) {
    let prod = f64::from(hit_t) * f64::from(inv_len);
    (prod as f32, prod)
}

/// `a + b` stored as `f32`: the running coordinates of `0x6ba9b0`, x at `0x6baa1c` and y at
/// `0x6baa4d`, each one tile size further.
pub fn liquid_advance(a: f32, b: f32) -> f32 {
    (f64::from(a) + f64::from(b)) as f32
}

/// `−v` stored as `f32`: the volume's translation negated at the start of `0x6bb6b0` (`0x6bb6d8`,
/// `0x6bb76c`, `0x6bb78f`).
pub fn mliq_translate(t: [f32; 3]) -> [f32; 3] {
    t.map(|v| -v)
}
