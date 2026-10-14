//! The zoom radius, the cull and placement radius of the blips for a zoom level
//! (`systems/ui/minimap.md#the-zoom-radius`).
//!
//! Two byte-identical functions compute it, `0x6da9b0` and `0x6dab70`. With the byte `0xceaa60` set
//! (`locked`) they return the radius table entry; clear, they derive the radius from the visible-chunk
//! count.

/// Visible-radius table (`0xceaa60` set), `0x8116e8`: {150,120,90,60,40,25} yd.
const RADIUS_TABLE: [f32; 6] = [
    f32::from_bits(0x4316_0000), // 150.0
    f32::from_bits(0x42f0_0000), // 120.0
    f32::from_bits(0x42b4_0000), // 90.0
    f32::from_bits(0x4270_0000), // 60.0
    f32::from_bits(0x4220_0000), // 40.0
    f32::from_bits(0x41c8_0000), // 25.0
];
/// Visible-chunk-count table (`0xceaa60` clear), `0x8116d0`: {14,12,10,8,6,4}.
const CHUNK_TABLE: [u32; 6] = [14, 12, 10, 8, 6, 4];
/// 0.5 (`0x7ffa24`) and 33.333 sub-chunk yd (`0x80fee4`).
const HALF: f32 = f32::from_bits(0x3f00_0000);
const SUBCHUNK: f32 = f32::from_bits(0x4205_5555);

/// The zoom radius of `0x6da9b0` and its twin `0x6dab70`.
///
/// With `0xceaa60` set: `radiusTable[zoom_locked]` (the index at `0x86f69c`). Clear: `(float)chunkTable[zoom] * 0.5 * 33.333`,
/// half the chunk count times the sub-chunk size in yards. The `fild`, `fmul`, `fmul` chain rounds to
/// `f64` and the result store to `f32`. `zoom` and `zoom_locked` run 0 to 5; `SetZoom` caps at 5.
pub fn zoom_to_scale(locked: bool, zoom: u32, zoom_locked: u32) -> f32 {
    if locked {
        RADIUS_TABLE[zoom_locked as usize]
    } else {
        ((CHUNK_TABLE[zoom as usize] as f64 * HALF as f64) * SUBCHUNK as f64) as f32
    }
}
