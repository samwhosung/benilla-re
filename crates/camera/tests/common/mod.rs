//! Helpers shared by the camera tests: a float compare that accepts any NaN for any NaN, the LCG,
//! and random floats.

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

/// One LCG step. Its low bits repeat quickly, so a choice uses the high bits.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32` in `[-hi, hi)` from the high bits.
pub fn fuzz(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24); // [0,1)
    ((m * 2.0 - 1.0) * f64::from(hi)) as f32
}

/// A random positive `f32` in `(0, hi]`.
pub fn fuzz_pos(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from((next(s) >> 8) | 1) / f64::from(1u32 << 24);
    (m * f64::from(hi)) as f32
}
