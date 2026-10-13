//! Helpers shared by the models tests: a float compare that accepts any NaN for any NaN, the LCG
//! and the draws made from it, and the reads and checks the slices use.

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

/// Asserts two doubles have the same bits, or are both NaN.
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

/// A random `f32` in `[lo, hi)` from the high bits.
pub fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Three draws of [`frand`].
pub fn frand3(s: &mut u32, lo: f32, hi: f32) -> [f32; 3] {
    [frand(s, lo, hi), frand(s, lo, hi), frand(s, lo, hi)]
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
pub fn read_f64(e: &Emu, va: u64) -> f64 {
    f64::from_le_bytes(e.read(va, 8).try_into().unwrap())
}

/// Asserts the copied bytes at `island` are the exe's at `src`, so the copy runs the client's own
/// instructions.
pub fn island_bytes_match(e: &Emu, island: u64, src: u64, len: usize) {
    let got = e.read(island, len);
    let want = e.read(src, len);
    assert_eq!(
        got, want,
        "island bytes at {island:#x} != WoW.exe[{src:#x}..] (len {len})"
    );
}

/// Builds `fld dword [src]`, `body`, `fstp dword [dst]` at `isl` and returns the end address.
pub fn island_f32(e: &mut Emu, isl: u64, src: u64, body: &[u8], dst: u64) -> u64 {
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(src as u32).to_le_bytes());
    code.extend_from_slice(body);
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(dst as u32).to_le_bytes());
    e.write(isl, &code);
    isl + code.len() as u64
}

/// A finite `f32` over five ranges: `[-1, 1)`, `[-10, 10)`, `[-1000, 1000)`, up to 1e6 and up to
/// 1e-6.
pub fn fuzz_scales(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 5;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 20.0 - 10.0) as f32,
        2 => (m * 2000.0 - 1000.0) as f32,
        3 => (m * 1.0e6) as f32,
        _ => (m * 1.0e-6) as f32,
    }
}

/// `ah` after `fnstsw ax`: the condition bits C0 (`0x01`), C2 (`0x04`) and C3 (`0x40`).
pub fn read_ah(e: &Emu) -> u8 {
    ((e.get_reg(benilla_re_difftest::RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// The LCG as a value, with the draws the tests make from it.
pub struct Lcg(pub u32);

impl Lcg {
    /// One step.
    pub fn next(&mut self) -> u32 {
        next(&mut self.0)
    }

    /// [`fuzz_scales`].
    pub fn f32_mixed(&mut self) -> f32 {
        fuzz_scales(&mut self.0)
    }

    /// [`fuzz_i32`].
    pub fn i32_mixed(&mut self) -> i32 {
        fuzz_i32(&mut self.0)
    }

    /// A byte from bits 16 to 23.
    pub fn byte(&mut self) -> u8 {
        ((self.next() >> 16) & 0xff) as u8
    }

    /// A colour channel: in `[0, 1)`, in `[0, 1.5)`, in `(-1, 0]`, or [`Lcg::f32_mixed`].
    pub fn clamp_chan(&mut self) -> f32 {
        let pick = self.next() >> 30;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => m as f32,
            1 => (m * 1.5) as f32,
            2 => -(m as f32),
            _ => self.f32_mixed(),
        }
    }
}

/// An `i32` from one of four ranges: `[0, 1000)`, `(-1000, 0]`, `[-50000, 50000)`, or any.
pub fn fuzz_i32(s: &mut u32) -> i32 {
    let pick = (next(s) >> 8) % 4;
    let r = next(s);
    match pick {
        0 => (r % 1000) as i32,
        1 => -((r % 1000) as i32),
        2 => (r % 100_000) as i32 - 50000,
        _ => r as i32,
    }
}

/// The install's `base` archive and the patches over it in order, as the client reads a file.
pub fn mount(dir: &std::path::Path, base: &str) -> wow_mpq::PatchChain {
    let mut chain = wow_mpq::PatchChain::new();
    chain
        .add_archive(dir.join(base), 0)
        .expect("mount the base archive");
    let mut patches: Vec<_> = std::fs::read_dir(dir)
        .expect("read Data")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(str::to_ascii_lowercase)
                .is_some_and(|n| n.starts_with("patch") && n.ends_with(".mpq"))
        })
        .collect();
    patches.sort_by_key(|p| p.file_name().map(|n| n.to_ascii_lowercase()));
    for (i, patch) in patches.iter().enumerate() {
        chain
            .add_archive(patch, 100 + i as i32)
            .expect("mount a patch");
    }
    chain
}

/// The chain's files ending in `ext`, one per name whatever its case, without the zero-length
/// entries a patch uses to remove a file.
pub fn list_files(chain: &mut wow_mpq::PatchChain, ext: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut paths: Vec<String> = chain
        .list()
        .expect("list the archives")
        .into_iter()
        .filter(|e| e.size > 0 && e.name.to_ascii_lowercase().ends_with(ext))
        .map(|e| e.name)
        .filter(|n| seen.insert(n.to_ascii_uppercase().replace('/', "\\")))
        .collect();
    paths.sort();
    paths
}
