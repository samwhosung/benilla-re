//! The world to minimap-screen scale, the tail of `0x6da380`: after it has ensured the tile window
//! (through the world to tile map, the tile shift and the tile to world map), the function emits the
//! screen scale from the snapped tile's world corner and the player position.

/// 1066.666 = 2 * 533.333 (`0x86f6b8` and `0x86f6bc` hold the same value; the exe uses each on a
/// different axis: `off0` multiplies by the reciprocal, `off1` divides directly).
const DENOM: f32 = f32::from_bits(0x4485_5555);
/// 33.333 sub-chunk yd (`0x80fee4`).
const SUBCHUNK: f32 = f32::from_bits(0x4205_5555);
/// Visible-chunk-count table (`0x8116d0`).
const CHUNK_TABLE: [u32; 6] = [14, 12, 10, 8, 6, 4];

/// The screen scale of `0x6da380`, after the tile-window setup.
///
/// With `inv = 1 / 1066.666`, it returns `(scale, [off0, off1])`:
/// `scale = (float)(chunkTable[zoom] >> 1) * 33.333 * inv` (stored to the fifth argument's target),
/// `off0 = (tileWorld.y - player.y) * inv` (the fourth argument's first float) and
/// `off1 = (tileWorld.x - player.x) / 1066.666` (its second). `chunk >> 1` loads through `fild qword`;
/// the arithmetic is `f64` and the stores `f32`. `inv` is stored and reloaded as an `f32`
/// (`fstp dword [ebp+0x10]`) before the two multiplies, so it is `f32`-rounded.
pub fn world_to_screen_scale(tile_world: [f32; 3], player: [f32; 2], zoom: u32) -> (f32, [f32; 2]) {
    let inv = ((1.0_f64 / DENOM as f64) as f32) as f64; // fdiv → fstp m32 (f32 round) → reload
    let half_chunk = (CHUNK_TABLE[zoom as usize] >> 1) as i64;
    let scale = ((half_chunk as f64 * SUBCHUNK as f64) * inv) as f32;
    let off0 = ((tile_world[1] as f64 - player[1] as f64) * inv) as f32;
    let off1 = ((tile_world[0] as f64 - player[0] as f64) / DENOM as f64) as f32;
    (scale, [off0, off1])
}
