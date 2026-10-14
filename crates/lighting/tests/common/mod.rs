#![allow(dead_code)]

//! Shared difftest helpers: the f32 assert in which any two NaN are equal, the LCG that the fuzz
//! draws from (its high bits are used, the low bits being periodic), and the f32 values where a
//! product stored as f32 floors differently from the wide product.

/// f32 NaN-pair-equal assert: NaN==NaN passes, everything else compares raw bits.
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

/// One LCG step; callers take the high bits.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A fuzzed f32 in a sane mixed range: finite magnitudes across signs and scales, drawn from the
/// LCG's high bits.
#[allow(dead_code)] // the fan-out families consume it as they land
pub fn fuzz_f32(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 5;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32, // [-1, 1)
        1 => (m * 200.0) as f32,     // [0, 200) — typical light radii
        2 => (-(m * 200.0)) as f32,  // negative distances
        3 => (m * 1.0e30) as f32,    // huge
        _ => (m * 1.0e-30) as f32,   // tiny
    }
}

/// Values of `y` where the product `scale * y` stored as f32 floors differently from the wide
/// product, each with the floor of the stored one.
pub fn straddling_y(scale: f32) -> Vec<(f32, i32)> {
    let mut out = Vec::new();
    for k in 2..60 {
        let centre = (f64::from(k) / f64::from(scale)) as f32;
        for d in -4i32..=4 {
            let y = f32::from_bits((centre.to_bits() as i32 + d) as u32);
            let wide = f64::from(scale) * f64::from(y);
            let stored = f64::from(wide as f32);
            if wide.floor() != stored.floor() {
                out.push((y, stored.floor() as i32));
            }
        }
    }
    out
}
