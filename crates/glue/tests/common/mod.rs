//! Helpers shared by the glue tests: the float comparisons, the LCG and the fuzz.

#![allow(dead_code)]

/// Bit-exact for finite values and infinities; two NaNs compare equal, and a NaN against a number
/// fails.
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

/// The same for an `f64`, the value `lua_pushnumber` takes.
pub fn assert_f64(binary: f64, ours: f64, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#018x} ours={:#018x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// One LCG step. Its low bits repeat quickly, so a choice uses the high bits.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32` in `[0, hi)` from the high bits.
pub fn fuzz_unit(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (m * f64::from(hi)) as f32
}
