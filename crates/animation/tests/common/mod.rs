//! Helpers the animation tests share: a float compare that accepts any NaN for any NaN, the LCG,
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

/// A random `f32`: in `[-1, 1)`, `[-50, 50)`, `[0, 1e6)` or `[0, 1e-6)`, from the high bits.
pub fn fuzz_f32(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 4;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32, // [-1, 1)
        1 => (m * 100.0 - 50.0) as f32,
        2 => (m * 1.0e6) as f32,
        _ => (m * 1.0e-6) as f32,
    }
}
