//! Shared by the spline tests: the `f32` fuzz stream and edge set, and the bit-exact comparisons
//! (any NaN equals any NaN).
#![allow(dead_code)]

pub use benilla_re_difftest::{Emu, Wow};

/// A linear-congruential step; the branch choices read the high bits, since the low ones are periodic.
pub fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32` bit pattern that is not NaN: finite or infinite. NaN is excluded as an input
/// since `fld` and `fstp` would quiet a signalling one; a NaN result is still compared, as any NaN.
pub fn fuzz(s: &mut u32) -> f32 {
    loop {
        let f = f32::from_bits(lcg(s));
        if !f.is_nan() {
            return f;
        }
    }
}

/// An `f32` in `[-range, range)`, for the inputs that must stay finite: angles and parameters.
pub fn fuzz_bounded(s: &mut u32, range: f32) -> f32 {
    let u = (lcg(s) >> 8) as f32 / (1u32 << 24) as f32; // [0,1)
    (u * 2.0 - 1.0) * range
}

/// The `f32` edge set: signed zeros, ±1 and ±0.5, a subnormal, the largest values, the infinities
/// and two ordinary values.
pub const EDGES: &[f32] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    3.0,
    1e30,
    -1e30,
    f32::MIN_POSITIVE,
    f32::from_bits(1),
    f32::MAX,
    f32::INFINITY,
    f32::NEG_INFINITY,
    123.456,
    -789.012,
];

/// `f64` bit equality, with any NaN equal to any NaN: the x87 indefinite and the host's NaN differ
/// only in sign and payload. A NaN against a number fails.
pub fn eqf64(got: f64, want: f64, ctx: &str) {
    if got.is_nan() && want.is_nan() {
        return;
    }
    assert_eq!(
        got.to_bits(),
        want.to_bits(),
        "{ctx}: got={:#018x} want={:#018x}",
        got.to_bits(),
        want.to_bits()
    );
}

/// `f32` bit equality, with any NaN equal to any NaN.
pub fn eqf32(got: f32, want: f32, ctx: &str) {
    if got.is_nan() && want.is_nan() {
        return;
    }
    assert_eq!(
        got.to_bits(),
        want.to_bits(),
        "{ctx}: got={:#010x} want={:#010x}",
        got.to_bits(),
        want.to_bits()
    );
}

/// Integer equality.
pub fn eqi(got: u32, want: u32, ctx: &str) {
    assert_eq!(got, want, "{ctx}: got={got:#x} want={want:#x}");
}
