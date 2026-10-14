//! Helpers shared by the font tests: the NaN-pair-equal f32 assert, the LCG (a choice uses its high
//! bits, since the low bits repeat quickly), and the font state-global addresses and struct offsets
//! the functions read and write.

/// f32 NaN-pair-equal assert: NaN==NaN passes, everything else compares raw bits.
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

/// One LCG step. A choice reads the high bits, never the low bits.
#[allow(dead_code)]
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A fuzzed f32 in `[0, hi)` drawn from the LCG's high bits.
#[allow(dead_code)]
pub fn fuzz_unit(s: &mut u32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (m * f64::from(hi)) as f32
}

// ── font module globals and struct offsets ──
#[allow(dead_code)]
pub mod g {
    pub const SCREEN_H: u64 = 0x00c2_b9a0; // i32 viewport pixel HEIGHT (480 default)
    pub const SCREEN_W: u64 = 0x00c2_b9a4; // i32 viewport pixel WIDTH (640 default)
    pub const SENTINEL: u64 = 0x007f_fd74; // f32 0.0 "unset" sentinel
}

/// CGxFont struct offsets used by the difftests.
#[allow(dead_code)]
pub mod font {
    pub const SIZE: u64 = 0x250; // total CGxFont allocation
    pub const FLAGS: u64 = 0x180; // u32 flags (0x1 shadow, 0x4 AA, 0x8 outline)
    pub const REQ_SIZE: u64 = 0x184; // f32 requested size
    pub const RASTER_PX: u64 = 0x178; // u32 rasterization pixel/em size
    pub const PIXEL_SIZE: u64 = 0x24c; // u32 rendered pixel size (font[0x24c], 0x5cae90)
}
