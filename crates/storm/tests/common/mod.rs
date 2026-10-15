//! Helpers shared by the storm tests: the install's `Data` folder as a fixture, the buffers to
//! hash, and an emulator with a context, a message buffer and a digest output.

#![allow(dead_code)]

use benilla_re_difftest::Emu;
pub use benilla_re_difftest::Wow;
use std::path::PathBuf;

/// The install's `Data` folder (`benilla_re_difftest::data_dir`): a test without it skips with one
/// line, and fails under `BENILLA_RE_REQUIRE_DATA`.
pub fn data_fixture() -> Option<PathBuf> {
    benilla_re_difftest::data_dir()
}

/// Byte buffers: the empty message, the standard vectors, every length from 0 to 130 (across the
/// padding boundary at 56 mod 64 and the block size) and a fuzz of other lengths.
pub fn synth_buffers() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"abc".to_vec(),
        b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".to_vec(),
        vec![b'a'; 1000],
    ];
    for len in 0..=130usize {
        v.push((0..len).map(|i| (i * 7 + 1) as u8).collect());
    }
    let mut s: u32 = 0x9e37_79b9;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..400 {
        let len = (next() >> 8) as usize % 300;
        v.push((0..len).map(|_| (next() >> 16) as u8).collect());
    }
    v
}

/// An emulator with a context (0x80 bytes), a message buffer (0x800) and a digest output (0x40).
pub struct Dev<'a> {
    pub emu: Emu<'a>,
    pub ctx: u64,
    pub io: u64,
    pub out: u64,
}

impl<'a> Dev<'a> {
    pub fn new(wow: &'a Wow) -> Dev<'a> {
        let mut emu = Emu::new(wow);
        let ctx = emu.alloc(0x80);
        let io = emu.alloc(0x800);
        let out = emu.alloc(0x40);
        Dev { emu, ctx, io, out }
    }

    /// Reads `n` bytes of emulator memory.
    pub fn read_bytes(&self, va: u64, n: usize) -> Vec<u8> {
        self.emu.read(va, n)
    }
}
