//! Helpers shared by the loading screen tests: a float compare that accepts any NaN for any NaN, the
//! LCG (a choice reads its high bits, since the low bits repeat quickly), a float fuzzer and the
//! loading screen's state globals.

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

/// One LCG step. A choice uses the high bits.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32` in `[-hi, hi)` from the LCG's high bits.
pub fn fuzz(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    ((m * 2.0 - 1.0) * f64::from(hi)) as f32
}

/// The loading screen's state globals.
pub mod g {
    pub const PROGRESS_PHASE: u64 = 0x0088_2894; // f32 phase flag (0/1)
    pub const PROGRESS_A: u64 = 0x0088_2bd0; // f32 progress input A
    pub const PROGRESS_B: u64 = 0x0088_2df4; // f32 progress input B
    pub const BAR_FILL: u64 = 0x0088_2be4; // f32 combined fill [0,1] (output)
}
