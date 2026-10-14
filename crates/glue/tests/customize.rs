//! The ghost kit's tint and alpha and the select screen's pet scale
//! (`systems/glue/select.md#ghost-kit`, `systems/glue/select.md#the-pet`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_glue::customize::{clamp01, color_channel, frame_interp, scale_product};
use common::*;

/// The tint of `ApplyCharSelectGhostKit`, from the `__ftol` at `0x4728ec` to the call of
/// `CM2Model::SetDiffuseColor` at `0x472939`, which does not run.
const COLOR_BEGIN: u64 = 0x4728ec;
const COLOR_END: u64 = 0x472939;
/// The alpha of `ApplyCharSelectGhostKit`, to the end of its call of `CM2Model::SetAlpha`
/// (`0x710cb0`), which is stubbed.
const CLAMP_BEGIN: u64 = 0x4728a4;
const CLAMP_END: u64 = 0x47293e;
const APPLY_SCALAR: u64 = 0x710cb0;
/// The pet scale of `BuildCharSelectModel`: the product, and the level ramp.
const SCALE_BEGIN: u64 = 0x472dc6;
const SCALE_END: u64 = 0x472dd7;
const INTERP_BEGIN: u64 = 0x472e73;
const INTERP_END: u64 = 0x472e8a;

/// Runs the tint of `ApplyCharSelectGhostKit` (`0x4727f0`), `0x4728ec` to `0x472939`, on colours below
/// 2^24, which an `f32` holds exactly, with `__ftol` running for real. It reads the three channels
/// at `[ebp-0x14]`, `[ebp-0x10]` and `[ebp-0xc]`: blue, green and red.
#[test]
fn char_customize_color_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the channels at [ebp-0x14], [ebp-0x10], [ebp-0xc]
    let valptr = e.alloc(4); // esi: the attribute float
    let stack = e.alloc(0x80) + 0x40; // esp, with room for `__ftol` and a `push ecx`

    let mut s: u32 = 0x0c01_0001;
    for i in 0..8000u32 {
        let n = (next(&mut s) >> 8) & 0x00ff_ffff; // 24 bits
        let (r, g, b) = (
            (n & 0xff) as u8,
            ((n >> 8) & 0xff) as u8,
            ((n >> 16) & 0xff) as u8,
        );
        e.write_u32(valptr, (n as f32).to_bits());
        e.set_reg(RegisterX86::ESI, valptr);
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESP, stack);
        e.run(COLOR_BEGIN, COLOR_END);
        // [ebp-0x14] is byte 2 (blue), [ebp-0x10] byte 1 (green), [ebp-0xc] byte 0 (red), times 1/255.
        assert_f32(
            e.read_f32(frame - 0x14),
            color_channel(b),
            &format!("char_customize color[{i}] n={n:#08x} B={b}"),
        );
        assert_f32(
            e.read_f32(frame - 0x10),
            color_channel(g),
            &format!("char_customize color[{i}] n={n:#08x} G={g}"),
        );
        assert_f32(
            e.read_f32(frame - 0xc),
            color_channel(r),
            &format!("char_customize color[{i}] n={n:#08x} R={r}"),
        );
    }
}

/// Runs the alpha of `ApplyCharSelectGhostKit` (`0x4727f0`), `0x4728a4` to `0x47293e`, with
/// `CM2Model::SetAlpha` (`0x710cb0`) stubbed to capture the value it receives. The values are below 0,
/// inside, above 1, and exactly 0 and 1.
#[test]
fn char_customize_clamp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(APPLY_SCALAR, 0, 1); // SetAlpha: the alpha is args[0]; pops 4
    let frame = e.alloc(0x40) + 0x20; // ebp: the value at [ebp-0x4]
    let stack = e.alloc(0x40) + 0x20;

    // -0.0, NaN, the infinities, the least subnormals and the floats beside 1.
    let probes = [
        -0.0,
        f32::NAN,
        f32::from_bits(0xffc0_0001),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x3f7f_ffff),
        f32::from_bits(0x3f80_0001),
    ];
    let mut s: u32 = 0x4d09_0001;
    for i in 0..8000u32 {
        let value = match probes.get(i as usize) {
            Some(&probe) => probe,
            None => match (next(&mut s) >> 8) % 5 {
                0 => -(fuzz_unit(&mut s, 2.0)),    // below 0
                1 => fuzz_unit(&mut s, 1.0),       // inside
                2 => 1.0 + fuzz_unit(&mut s, 2.0), // above 1
                3 => 0.0,                          // exactly 0
                _ => 1.0,                          // exactly 1
            },
        };
        e.write_u32(frame - 0x4, value.to_bits());
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESP, stack);
        e.set_reg(RegisterX86::EDI, 0); // the object `SetAlpha` is called on, unused
        e.clear_calls();
        e.run(CLAMP_BEGIN, CLAMP_END);
        let calls = e.calls();
        assert_eq!(
            calls.len(),
            1,
            "clamp[{i}] value={value}: one apply call expected"
        );
        let clamped = f32::from_bits(calls[0].args[0]);
        assert_f32(
            clamped,
            clamp01(value),
            &format!("char_customize clamp[{i}] value={value}"),
        );
    }
}

/// Runs the pet scale product of `BuildCharSelectModel` (`0x472950`), `0x472dc6` to `0x472dd7`, with the
/// model row in `edi` and the display row in `ebx`, each holding its float at `+0x10`.
#[test]
fn charcreate_blend_scale_product_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the result at [ebp-0x4]
    let a_obj = e.alloc(0x20); // edi: the model row, its float at +0x10
    let b_obj = e.alloc(0x20); // ebx: the display row, its float at +0x10
    let this = e.alloc(0x200); // ecx, which an interleaved `mov eax,[ecx+0xf8]` reads

    let mut s: u32 = 0x21aa_0001;
    for i in 0..6000u32 {
        let a = fuzz_unit(&mut s, 4.0) - 2.0;
        let b = fuzz_unit(&mut s, 4.0) - 2.0;
        e.write_u32(a_obj + 0x10, a.to_bits());
        e.write_u32(b_obj + 0x10, b.to_bits());
        e.set_reg(RegisterX86::EDI, a_obj);
        e.set_reg(RegisterX86::EBX, b_obj);
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::EBP, frame);
        e.run(SCALE_BEGIN, SCALE_END);
        assert_f32(
            e.read_f32(frame - 0x4),
            scale_product(a, b),
            &format!("charcreate scale_product[{i}] a={a} b={b}"),
        );
    }
}

/// Runs the pet scale ramp of `BuildCharSelectModel` (`0x472950`), `0x472e73` to `0x472e8a`, with the family
/// row in `esi` (minimum scale at `+4`, maximum at `+0xc`), the clamped level step at `[ebp-4]` and the
/// range at `[ebp-0xc]`.
#[test]
fn charcreate_blend_frame_interp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the step at [ebp-4], the range at [ebp-0xc]
    let kf = e.alloc(0x20); // esi: the family row, minimum at +4, maximum at +0xc

    // A range of 0, which the client does not guard: `0 / 0` is NaN. `(lo, hi, idx, total)`.
    let probes: [(f32, f32, i32, i32); 3] = [(0.5, 1.5, 0, 0), (1.0, 1.0, 0, 0), (0.0, 2.0, 0, 0)];
    let mut s: u32 = 0x6b15_0001;
    for i in 0..probes.len() as u32 + 6000 {
        let (lo, hi, idx, total) = if let Some(&p) = probes.get(i as usize) {
            p
        } else {
            // The range is 1 to 64 levels, and the step is 0 to the range.
            let total = 1 + ((next(&mut s) >> 8) % 64) as i32;
            let idx = ((next(&mut s) >> 8) % (total as u32 + 1)) as i32;
            let lo = fuzz_unit(&mut s, 2.0);
            let hi = fuzz_unit(&mut s, 2.0);
            (lo, hi, idx, total)
        };
        e.write_u32(frame - 0x4, idx as u32);
        e.write_u32(frame - 0xc, total as u32);
        e.write_u32(kf + 0x4, lo.to_bits());
        e.write_u32(kf + 0xc, hi.to_bits());
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESI, kf);
        e.run(INTERP_BEGIN, INTERP_END);
        assert_f32(
            e.read_f32(frame - 0x4),
            frame_interp(lo, hi, idx, total),
            &format!("charcreate frame_interp[{i}] lo={lo} hi={hi} idx={idx}/{total}"),
        );
    }
}
