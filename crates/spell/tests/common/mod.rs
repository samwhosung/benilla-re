//! Helpers shared by the spell tests: the float comparison, the LCG and the fuzz, and a vector
//! writer.

#![allow(dead_code)]

use benilla_re_difftest::Emu;

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

/// Writes three floats at `va`.
pub fn write_vec3(e: &mut Emu, va: u64, v: [f32; 3]) {
    e.write_u32(va, v[0].to_bits());
    e.write_u32(va + 4, v[1].to_bits());
    e.write_u32(va + 8, v[2].to_bits());
}
