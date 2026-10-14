//! Helpers shared by the minimap tests: float and integer compares, the LCG (a choice reads its high
//! bits, since the low bits repeat quickly) and the minimap's state globals.

#![allow(dead_code)]

/// Asserts two floats have the same bits, or are both NaN.
pub fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// Asserts two integers are equal.
pub fn assert_i32(binary: i32, ours: i32, ctx: &str) {
    assert_eq!(binary, ours, "{ctx}: binary={binary} ours={ours}");
}

/// One LCG step. A choice uses the high bits.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A fuzzed f32 in `[-hi, hi)` from the LCG high bits.
pub fn fuzz(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24); // [0,1)
    ((m * 2.0 - 1.0) * f64::from(hi)) as f32
}

/// A fuzzed i32 in `[-hi, hi)`.
pub fn fuzz_i32(s: &mut u32, hi: i32) -> i32 {
    (next(s) >> 8) as i32 % (2 * hi) - hi
}

/// The minimap's state globals.
pub mod g {
    pub const ORIGIN_Y: u64 = 0x00ce_aa88; // f32 per-map Y origin (tile_to_world out.y)
    pub const ZOOM_LOCKED_FLAG: u64 = 0x00ce_aa60; // u8 byte selecting the radius table
    pub const ZOOM: u64 = 0x0086_f698; // u32 unlocked zoom index 0..=5
    pub const ZOOM_L: u64 = 0x0086_f69c; // u32 locked zoom index 0..=5
}
