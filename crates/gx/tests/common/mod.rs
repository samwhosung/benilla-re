//! Helpers shared by the gx tests: a float compare that accepts any NaN for any NaN, the LCG and
//! its draws, and the condition bits a slice reads.

#![allow(dead_code)]

use benilla_re_difftest::Emu;

/// A stack pointer inside the harness's stack, for slices that push.
pub const SLICE_ESP: u64 = 0x7008_0000;

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

/// A random `f32` in `[lo, hi)` from the high bits.
pub fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// `ah` after `fnstsw ax`: the condition bits C0 (`0x01`), C2 (`0x04`) and C3 (`0x40`).
pub fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(benilla_re_difftest::RegisterX86::EAX) >> 8) & 0xff) as u8
}
