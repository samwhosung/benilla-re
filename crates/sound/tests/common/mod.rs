//! Helpers shared by the sound tests: the input sets, the `Snd` fixture around the emulator, and the
//! bit-exact comparisons.
//!
//! Most sound functions are `__thiscall` or `__fastcall` on a sound object (`ecx`) or `__stdcall`
//! scalars, and several end in an FSOUND import (`fmod.dll` is not mapped). A result is read from
//! `eax`, from the argument a stubbed import captures, or from a value a stub writes through a
//! pointer.

#![allow(dead_code)]

use benilla_re_difftest::Emu;
pub use benilla_re_difftest::{RegisterX86, Wow};

/// `f32` inputs: edge values, a dense sweep of [0, 1] (the volume curves' range, where the
/// `>> 14` slice of the mantissa varies), and a fuzz of arbitrary bit patterns. NaN is kept: the
/// curves give a specific integer for NaN, which must agree too.
pub fn synth_f32() -> Vec<f32> {
    let mut v = vec![
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.25,
        0.75,
        0.999_999,
        1.000_001,
        2.0,
        255.0,
        1e30,
        -1e30,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        123.456,
        -789.012,
    ];
    for i in 0..=256 {
        v.push(i as f32 / 256.0);
    }
    let mut s: u32 = 0x1234_5678;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..2000 {
        v.push(f32::from_bits(next()));
    }
    v
}

/// Integer levels for the fade rates: the 0 to 255 clamp edges, values either side, and a fuzz.
pub fn synth_i32() -> Vec<i32> {
    let mut v = vec![
        0,
        1,
        127,
        128,
        254,
        255,
        256,
        -1,
        -128,
        1000,
        -1000,
        i32::MIN,
        i32::MAX,
    ];
    let mut s: u32 = 0x5151_2a2a;
    for _ in 0..200 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        v.push((s >> 8) as i32 % 600 - 50);
    }
    v
}

/// Three-component vectors: edge values (signed zeros, units, NaN) and a fuzz.
pub fn synth_vec3() -> Vec<[f32; 3]> {
    let f = synth_f32();
    let mut v: Vec<[f32; 3]> = Vec::new();
    for w in f.windows(3).step_by(3) {
        v.push([w[0], w[1], w[2]]);
    }
    v
}

/// A zero-filled sound object of 0x100 bytes inside an emulator, with two scratch regions. The
/// fields the tests touch: `+0x30` the FMOD channel handle, `+0x74` the volume, `+0x88` the flags
/// that `Sound::IsSuspended` (`0x7a6720`) tests.
pub struct Snd<'a> {
    pub emu: Emu<'a>,
    pub chan: u64,
    /// A scratch out-vector region.
    pub out: u64,
    /// A scratch payload region for `stub_write_through`.
    pub payload: u64,
}

impl<'a> Snd<'a> {
    pub fn new(wow: &Wow) -> Self {
        let mut emu = Emu::new(wow);
        let chan = emu.alloc(0x100);
        let out = emu.alloc(0x40);
        let payload = emu.alloc(0x40);
        Snd {
            emu,
            chan,
            out,
            payload,
        }
    }
    pub fn put_f32(&mut self, va: u64, f: f32) {
        self.emu.write_u32(va, f.to_bits());
    }
    pub fn put_vec3(&mut self, va: u64, v: &[f32; 3]) {
        for (i, &f) in v.iter().enumerate() {
            self.emu.write_u32(va + i as u64 * 4, f.to_bits());
        }
    }
    pub fn read_vec3(&self, va: u64) -> [f32; 3] {
        [
            self.emu.read_f32(va),
            self.emu.read_f32(va + 4),
            self.emu.read_f32(va + 8),
        ]
    }
}

/// Bit-exact for finite values and infinities; two NaNs compare equal (the x87 indefinite and an
/// IEEE NaN differ in sign and payload), and a NaN against a number fails.
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

pub fn assert_vec3(binary: [f32; 3], ours: [f32; 3], ctx: &str) {
    for i in 0..3 {
        assert_f32(binary[i], ours[i], &format!("{ctx}[{i}]"));
    }
}

impl Snd<'_> {
    /// Runs the position build `(x, y, z + 2.0)` of a play function as a slice and reads the three
    /// floats it builds, without the type dispatch or hash lookup ahead of it in `0x457dc0` and
    /// `0x458530`. The build bytes are those of `0x457f20`: `fld; fstp; fld; fstp; fld; fadd
    /// [0x801628]; fstp`.
    ///
    /// `begin..until` is the build. `pt_off` is the `[ebp+pt_off]` slot holding the point pointer
    /// and `out_off` the `[ebp+out_off]` of the first built float, the three being contiguous.
    /// `esp` sits below the outputs so the slice's `push`es cannot overwrite them, and `ecx` points
    /// at a valid object since one slice reads `[ecx+8]`.
    pub fn emit_build_slice(
        &mut self,
        begin: u64,
        until: u64,
        pt_off: i64,
        out_off: i64,
        pt: &[f32; 3],
    ) -> [f32; 3] {
        let frame = self.emu.alloc(0x80);
        let ebp = frame + 0x40;
        let input = self.emu.alloc(0x10);
        self.put_vec3(input, pt);
        self.emu
            .write_u32((ebp as i64 + pt_off) as u64, input as u32);
        self.emu.set_reg(RegisterX86::EBP, ebp);
        self.emu.set_reg(RegisterX86::ESP, ebp - 0x20);
        self.emu.set_reg(RegisterX86::ECX, self.chan);
        self.emu.run(begin, until);
        self.read_vec3((ebp as i64 + out_off) as u64)
    }
}
