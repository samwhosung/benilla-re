//! The text block bridge against the exe (`systems/font/api.md`, Text block fonts and Text block
//! measures), as slices with the frame set directly:
//! - `TextBlockGenerateFont` (`0x44d040`): the size, `[0x44d06f, 0x44d08f)`, with
//!   `DivideByScreenScaleY` (`0x41ae50`) running and `s_screenScaleY` written at each case; and the
//!   flag map, `[0x44d1fa, 0x44d219)`;
//! - the flag maps of `TextBlockGetTextExtent` (`0x44d670`), `[0x44d6a4, 0x44d701)`, and
//!   `TextBlockGetMaxCharsWithinHeight` (`0x44d960`), `[0x44d996, 0x44d9f3)`.
//!
//! The sizes include both zeros, the infinities, NaN, the denormals, and quotients at 1.0 and a step
//! either side.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_font::text_block::{font_flags, font_size, measure_flags};
use common::{assert_f32, next};

const SCALE_Y: u64 = 0x832a48;
const SLICE_ESP: u64 = 0x7008_0000;

fn frame(e: &mut Emu) -> u64 {
    let f = e.alloc(0x100);
    f + 0x80
}

#[test]
fn text_block_font_size_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = frame(&mut e);
    let mut s = 0x44d0_6f00u32;
    let scales = [0.6f32, 0.8, 1.0, 0.447_213_6, 0.5, 1.0e-3, 3.0];
    let sizes: Vec<f32> = vec![
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        1.0e-45,
        -1.0e-45,
        f32::MAX,
        -1.0,
        0.012,
        0.018,
    ];
    let mut n = 0u32;
    for &scale in &scales {
        let mut these = sizes.clone();
        // At, and a step either side of, a quotient of 1.0, and a size just below the scale.
        for k in [-2i32, -1, 0, 1, 2] {
            these.push(f32::from_bits((scale.to_bits() as i32 + k) as u32));
        }
        these.push((f64::from(scale) * (1.0 - 1.0e-9)) as f32);
        for _ in 0..3000 {
            let r = next(&mut s);
            let v = f32::from_bits(r);
            these.push(if v.is_nan() { 0.5 } else { v });
            // A quotient in [0, 2).
            these.push(scale * (next(&mut s) >> 8) as f32 / (1u32 << 23) as f32);
        }
        for &size in &these {
            e.write_u32(SCALE_Y, scale.to_bits());
            e.write_u32(ebp + 8, size.to_bits());
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(0x44d06f, 0x44d08f);
            let bin = f32::from_bits(e.read_u32(ebp + 8));
            assert_f32(
                bin,
                font_size(size, scale),
                &format!("size {size:e} scale {scale}"),
            );
            n += 1;
        }
    }
    eprintln!("text_block_font_size: {n} cases bit-exact vs WoW.exe @ 0x44d06f..0x44d08f");
}

#[test]
fn text_block_font_flags_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = frame(&mut e);
    let mut s = 0x44d1_fa00u32;
    let mut n = 0u32;
    for i in 0..1024u32 {
        let flags = if i < 256 { i } else { next(&mut s) };
        e.write_u32(ebp - 8, flags);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x44d1fa, 0x44d219);
        let bin = e.get_reg(RegisterX86::EAX) as u32;
        assert_eq!(bin, font_flags(flags), "flags {flags:#x}");
        n += 1;
    }
    eprintln!("text_block_font_flags: {n} cases bit-exact vs WoW.exe @ 0x44d1fa..0x44d219");
}

#[test]
fn text_block_measure_flags_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ebp = frame(&mut e);
    let mut s = 0x44d6_a400u32;
    let mut n = 0u32;
    for i in 0..0x1_0400u32 {
        let flags = if i < 0x1_0000 { i } else { next(&mut s) };
        for &(begin, end, slot) in &[
            (0x44d6a4u64, 0x44d701u64, 0x1cu64),
            (0x44d996, 0x44d9f3, 0x20),
        ] {
            e.write_u32(ebp + slot, flags);
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(begin, end);
            let bin = e.get_reg(RegisterX86::EAX) as u32;
            assert_eq!(bin, measure_flags(flags), "{begin:#x} flags {flags:#x}");
            n += 1;
        }
    }
    eprintln!("text_block_measure_flags: {n} cases bit-exact vs WoW.exe @ 0x44d6a4, 0x44d996");
}
