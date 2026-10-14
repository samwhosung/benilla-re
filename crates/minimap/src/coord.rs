//! World and ADT-tile coordinates on the 64 by 64 grid, a tile being 533.333 yd. The exe's
//! constants: the tile size `0x80654c`, the map origin 17066.666 = 32 * 533.333 `0x7ffab4`, and the
//! reciprocal 0.001875 = 1 / 533.333 `0x811734`.

/// 533.333 yd — one ADT tile (`0x80654c`).
const TILE_YD: f32 = f32::from_bits(0x4405_5555);
/// 17066.666 = 32·533.333 — the map origin (`0x7ffab4`).
const ORIGIN: f32 = f32::from_bits(0x4685_5555);
/// 0.001875 = 1/533.333 (`0x811734`).
const INV_TILE: f32 = f32::from_bits(0x3af5_c290);

/// The tile to world map, `0x6da5b0` (`__fastcall (out ecx, tile edx)`).
///
/// Tile (row, column) to world (x, y, z): `out.x = 17066.666 - tile[1] * 533.333`,
/// `out.y = originY - tile[0] * 533.333` (`originY` is the per-map global `0xceaa88`, normally
/// 17066.666) and `out.z = 0`. The `fild`, `fmul`, `fsubr` chain rounds to `f64`, the `fstp dword`
/// to `f32`.
pub fn tile_to_world(tile: [i32; 2], origin_y: f32) -> [f32; 3] {
    let x = (ORIGIN as f64 - tile[1] as f64 * TILE_YD as f64) as f32;
    let y = (origin_y as f64 - tile[0] as f64 * TILE_YD as f64) as f32;
    [x, y, 0.0]
}

/// The world to tile map, `0x6da540` (`__fastcall (out ecx, world edx)`, two integers out).
///
/// World (x, y) to tile indices. Per axis `t = (17066.666 - world) * 0.001875`, floored by the exe's
/// idiom: subtract 1.0 when `t < 0`, then truncate toward zero (`__ftol`). The axes swap as the exe
/// stores them: `out[0]` is the y axis, `out[1]` the x axis.
pub fn world_to_tile(world: [f32; 2]) -> [i32; 2] {
    [floor_axis(world[1]), floor_axis(world[0])]
}

fn floor_axis(w: f32) -> i32 {
    let mut t = (ORIGIN as f64 - w as f64) * INV_TILE as f64;
    if t < 0.0 {
        t -= 1.0;
    }
    benilla_re_cmath::x87::ftol(t) // __ftol: truncate toward zero
}
