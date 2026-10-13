//! Helpers shared by the terrain tests: a float compare that accepts any NaN for any NaN, the LCG
//! and the draws made from it, and the reads and checks the slices use.

#![allow(dead_code)]

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

/// A stack pointer inside the harness's stack, for slices that push.
pub const SLICE_ESP: u64 = 0x7008_0000;

/// A random `f32` in `[lo, hi)` from the high bits.
pub fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Three draws of [`frand`].
pub fn frand3(s: &mut u32, lo: f32, hi: f32) -> [f32; 3] {
    [frand(s, lo, hi), frand(s, lo, hi), frand(s, lo, hi)]
}

/// A finite `f32` over six ranges, both signs, from 1e-6 to 1e30.
pub fn fuzz_wide(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 20.0 - 10.0) as f32,
        2 => (m * 2000.0 - 1000.0) as f32,
        3 => (m * 1.0e6 - 5.0e5) as f32,
        4 => (m * 1.0e-6) as f32,
        _ => (m * 1.0e30 - 5.0e29) as f32,
    }
}

/// The `f64` at `va`.
pub fn read_f64(e: &benilla_re_difftest::Emu, va: u64) -> f64 {
    f64::from_le_bytes(e.read(va, 8).try_into().unwrap())
}

/// Asserts the copied bytes at `island` are the exe's at `src`.
pub fn island_bytes_match(e: &benilla_re_difftest::Emu, island: u64, src: u64, len: usize) {
    assert_eq!(
        e.read(island, len),
        e.read(src, len),
        "island bytes at {island:#x} != WoW.exe[{src:#x}..] (len {len})"
    );
}

/// A finite `f32` over five ranges: `[-1, 1)`, `[0, 200)`, `(-200, 0]`, up to 1e30 and up to 1e-30.
pub fn fuzz_f32(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 5;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 200.0) as f32,
        2 => (-(m * 200.0)) as f32,
        3 => (m * 1.0e30) as f32,
        _ => (m * 1.0e-30) as f32,
    }
}

/// The LCG as a struct, with a coordinate drawn over four ranges from the high bits.
pub struct Lcg(pub u32);

impl Lcg {
    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    pub fn coord(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 4;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 40.0 - 20.0) as f32,
            2 => (m * 4.0 - 2.0) as f32,
            _ => (m * 100.0 - 50.0) as f32,
        }
    }

    pub fn f32_finite(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 5;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        let v = match pick {
            0 => m * 2.0 - 1.0,
            1 => m * 100.0,
            2 => -(m * 100.0),
            3 => m * 1.0e6 - 5e5,
            _ => (m * 2.0 - 1.0) * 1.0e-6,
        };
        v as f32
    }
}
