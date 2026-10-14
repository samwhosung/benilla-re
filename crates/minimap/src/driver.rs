//! The tile-window snap in the minimap update `0x6d93a0`.
//!
//! The 1770-byte update sequences the orientation, the coordinates, the blips, the tile streaming
//! and the draw; the block at `0x6d95f8` to `0x6d96b1` aligns the streaming window to the player's
//! chunk grid and writes the window bounds `0xcea9c4` to `0xcea9d8`.

/// Visible-radius (chunk-width) table (`0x8116e8`), indexed by `0x86f69c`: {150,120,90,60,40,25}.
const RADIUS_TABLE: [f32; 6] = [
    f32::from_bits(0x4316_0000),
    f32::from_bits(0x42f0_0000),
    f32::from_bits(0x42b4_0000),
    f32::from_bits(0x4270_0000),
    f32::from_bits(0x4220_0000),
    f32::from_bits(0x41c8_0000),
];
/// 0.5 grid bias (`0x86f690`) and 0.5 half-chunk (`0x7ffa24`).
const K: f32 = f32::from_bits(0x3f00_0000);
const HALF: f32 = f32::from_bits(0x3f00_0000);

/// The snapped tile-streaming window (globals 0xcea9c4–0xcea9d8: min.xyz then max.xyz).
#[derive(Debug, PartialEq)]
pub struct SnapWindow {
    pub min: [f32; 3], // cea9c4, cea9c8, cea9cc
    pub max: [f32; 3], // cea9d0, cea9d4, cea9d8
}

/// The tile-window snap, the slice `0x6d95f8` to `0x6d96b1` of `0x6d93a0`.
///
/// `chunkW = radiusTable[zoom]`, `inv = 1 / chunkW`. The window snaps the player to the chunk grid:
/// `min.x = round(inv * px - 0.5) * chunkW`, `min.y = round(inv * py - 0.5) * chunkW`,
/// `min.z = pz - chunkW * 0.5`, `max = min + chunkW` in x and y, `max.z = chunkW * 0.5 + min.z`. The
/// y axis uses `inv` at full `f64`; the x axis uses the `f32` copy `fst dword` stored and reloads. The
/// rounds are `fistp`, half to even. `max.z` adds `chunkW * 0.5` (an `f64`) to `min.z` reloaded as an
/// `f32`, so it is near `pz` and not equal to it.
pub fn minimap_update_snap(player: [f32; 3], zoom_locked: u32) -> SnapWindow {
    let chunk_w = RADIUS_TABLE[zoom_locked as usize];
    let inv_full = 1.0_f64 / chunk_w as f64; // st0, full f64
    let inv_f32 = inv_full as f32; // fst dword [ebp-0x10]
    let py_term = (inv_full * player[1] as f64) as f32; // py path: full-precision inv
    let ny = benilla_re_cmath::x87::fistp((py_term as f64) - K as f64);
    let px_term = (inv_f32 as f64 * player[0] as f64) as f32; // px path: f32-rounded inv
    let nx = benilla_re_cmath::x87::fistp((px_term as f64) - K as f64);
    let mnx = (nx as f64 * chunk_w as f64) as f32;
    let mny = (ny as f64 * chunk_w as f64) as f32;
    let h = chunk_w as f64 * HALF as f64; // chunkW·0.5, kept in f64
    let mnz = (player[2] as f64 - h) as f32;
    let mxx = (mnx as f64 + chunk_w as f64) as f32;
    let mxy = (mny as f64 + chunk_w as f64) as f32;
    let mxz = (h + mnz as f64) as f32;
    SnapWindow {
        min: [mnx, mny, mnz],
        max: [mxx, mxy, mxz],
    }
}
