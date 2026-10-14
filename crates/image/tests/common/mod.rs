//! Helpers shared by the image tests: the LCG (a choice uses its high bits, since the low bits
//! repeat quickly) and integer fuzzers.

#![allow(dead_code)]

/// The standard LCG step.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A fuzzed u32 in `[0, hi)` from the LCG high bits.
pub fn fuzz_u32(s: &mut u32, hi: u32) -> u32 {
    ((u64::from(next(s) >> 8) * u64::from(hi)) >> 24) as u32
}
