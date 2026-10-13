//! The camera and box negations of the map object passes: `DrawMapObjectPass` (`0x684840`),
//! `DrawGroupLiquidPass` (`0x684cd0`) and `DrawFarBand` (`0x6841a0`, with its two depth
//! subtractions), all `systems/terrain/cull.md#scene-frame`, and the box corners of
//! `GatherMapObjectFacets` (`0x6aaab0`, `systems/terrain/collide.md#terrain-gather`) and
//! `LinkLightToGroups` (`0x6c0cf0`).

/// `[0x8101b0]`: 33.0.
pub const FAR_FSUB1_K: f32 = f32::from_bits(0x4204_0000);
/// `[0x80fec4]`: 0.005 (`0x3ba3d70a`).
pub const FAR_FSUB2_K: f32 = f32::from_bits(0x3ba3_d70a);
/// `[0x80febc]`: 0.96 (`0x3f75c28f`).
pub const FAR_FSUB2_MINUEND: f32 = f32::from_bits(0x3f75_c28f);

/// `fld dword; fchs; fstp dword`: the camera `0xc7cf20` to `0xc7cf28` in `0x684840` (`0x684a05`,
/// `0x684a21`, `0x684a42`), `0x684cd0` (`0x684e63`, `0x684e72`, `0x684e87`) and `0x6841a0`
/// (`0x684379`, `0x684398`, `0x6843c2`), and box corners in `0x6aaab0` (`0x6aaaf8` to `0x6aab17`)
/// and `0x6c0cf0` (`0x6c0d38` to `0x6c0d57`).
pub fn coord_negate(src: f32) -> f32 {
    (-f64::from(src)) as f32
}

/// `DrawFarBand` (`0x6841a0`) at `0x684260`: `[ebp-0x1c] − 33.0`, stored to `[esp]`.
pub fn far_fsub1(depth: f32) -> f32 {
    (f64::from(depth) - f64::from(FAR_FSUB1_K)) as f32
}

/// `0x6841a0` at `0x684356`: `minuend − 0.005`, the minuend `[0x80febc]` (0.96), stored to `[esp]`.
pub fn far_fsub2(minuend: f32) -> f32 {
    (f64::from(minuend) - f64::from(FAR_FSUB2_K)) as f32
}
