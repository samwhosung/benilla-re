//! Shared by the gxumath tests: the bit-exact comparisons and the random streams.
#![allow(dead_code)]

/// `f32` bit equality, with any NaN equal to any NaN.
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

/// Bit equality of two arrays of `f32`, with any NaN equal to any NaN.
pub fn assert_arr(binary: &[f32], ours: &[f32], ctx: &str) {
    assert_eq!(binary.len(), ours.len(), "{ctx}: length");
    for (i, (&b, &o)) in binary.iter().zip(ours).enumerate() {
        assert_f32(b, o, &format!("{ctx}[{i}]"));
    }
}

/// A linear-congruential step; branch choices read the high bits, since the low ones are periodic.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// An `f32` in `[-hi, hi)` from the high bits: a position or a matrix entry.
pub fn fuzz(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24); // [0,1)
    ((m * 2.0 - 1.0) * f64::from(hi)) as f32
}

/// An `f32` in `(0, hi]`, for the arguments the functions require to be positive.
pub fn fuzz_pos(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from((next(s) >> 8) | 1) / f64::from(1u32 << 24);
    (m * f64::from(hi)) as f32
}
