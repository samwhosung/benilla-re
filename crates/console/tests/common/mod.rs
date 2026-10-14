//! Helpers shared by the console tests: a float compare that accepts any NaN for any NaN, the LCG
//! (a choice reads its high bits, since the low bits repeat quickly) and the console's state globals.

/// Asserts two floats have the same bits, or are both NaN.
#[allow(dead_code)]
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

/// One LCG step. A choice uses the high bits.
#[allow(dead_code)]
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32` in `[0, hi)` from the LCG's high bits; the console's layout values are
/// device-space fractions near `[0, 2]`.
#[allow(dead_code)]
pub fn fuzz_unit(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (m * f64::from(hi)) as f32
}

// The console's state globals.
#[allow(dead_code)]
pub mod g {
    pub const FONT_SIZE: u64 = 0x0086_455c; // f32 master font size (device units)
    pub const LINE_RATIO: u64 = 0x0086_4560; // f32 derived line-height ratio
    pub const LINE_METRIC: u64 = 0x00c4_ec10; // f32 lineRatio·fontSize (derived)
    pub const TEXT_TOP: u64 = 0x0086_4548; // f32 animated drop-down top Y (also visibility gate)
    pub const OPEN_FLAG: u64 = 0x00c4_eac8; // u32 console-open flag
    pub const SNAP_FLAG: u64 = 0x00c4_ec38; // u32 snap-to-target (drag) flag
    pub const IDLE_EXEC_CTR: u64 = 0x00c4_eabc; // u32 deferred-Execute counter (0 ⇒ no Execute)
    pub const IDLE_EXEC_CH: u64 = 0x00c4_e960; // u8 deferred-Execute char gate
    pub const FONT_HANDLE: u64 = 0x00c4_ec34; // font handle (from the TextBlock kernel)
}
