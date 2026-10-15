//! Helpers shared by the tests of the script bindings and widget arithmetic: the LCG that draws
//! the fuzz inputs, uniform floats, and a read of an `f64` out of emulated memory.

#![allow(dead_code)]

use benilla_re_difftest::Emu;

/// One LCG step. Its low bits repeat quickly, so a choice uses the high bits.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A uniform `f32` in `[lo, hi)` from the high 24 bits.
pub fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

/// A uniform `f64` in `[lo, hi)` from the high 24 bits.
pub fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

/// The `f64` at `va` in emulated memory.
pub fn read_f64(emu: &Emu, va: u64) -> f64 {
    let b = emu.read(va, 8);
    f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
}

/// The install's `Data` folder (`benilla_re_difftest::data_dir`): a test without it skips with one
/// line, and fails under `BENILLA_RE_REQUIRE_DATA`.
pub fn data_fixture() -> Option<std::path::PathBuf> {
    benilla_re_difftest::data_dir()
}
