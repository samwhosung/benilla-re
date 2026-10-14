//! Blip / tile-LOD geometry primitives.

/// 0.5 (`0x7ffa24`).
const HALF: f32 = f32::from_bits(0x3f00_0000);

/// The tile record's LOD-select distance, from `0x6d9e30` (`__fastcall (records ecx, matchKey edx,
/// key3 [ebp+8], bias [ebp+0xc])`).
///
/// The function finds the record in the `0xa4`-byte array (at most 512) whose map id, column and row
/// match `key3` (`+0`, `+4`, `+8`), then writes `rec[0x8c] = (key3[0x20] + key3[0x14]) * 0.5 - bias`,
/// the tile box's z midpoint less the caller's bias: `f32` operands, an `f64` intermediate, an `fstp` to
/// `f32`. When the record's tile id (`rec[0xa0]`) equals `matchKey` it stores `FLT_MAX` instead, the
/// current tile having no bias. This is the midpoint.
pub fn blip_lod_distance(z14: f32, z20: f32, bias: f32) -> f32 {
    ((z20 as f64 + z14 as f64) * HALF as f64 - bias as f64) as f32
}
