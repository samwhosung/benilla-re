//! `draw_caret` difftest (0x63b990): run the caret-geometry slice on a synthesized pen position and
//! `scaleX`/`fontSize`, then read the two derived corners from the vertex locals. `ecx` points at
//! the `{pen_x, pen_y}` pair; the slice starts at `fld [scaleX]` and stops before the gx submit.

mod common;

use benilla_re_console::caret::{caret_blink, draw_caret, paint_baseline};
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::{assert_f32, fuzz_unit, g, next};

const BLINK_BEGIN: u64 = 0x63b806; // `fld [0x864548]` (visibility gate)
const BLINK_END: u64 = 0x63b868; // `push ebx` — after the blink state machine
const BASELINE_BEGIN: u64 = 0x63b87e; // `fld [0x86455c]`
const BASELINE_END: u64 = 0x63b89c; // `call 0x5c1f00` (the gx batch begin)
const CARET_ACCUM: u64 = 0x00c4_e6f4;
const CARET_PHASE: u64 = 0x00c4_e6f8;
const TEXT_TOP: u64 = 0x0086_4548;

#[test]
fn paint_text_layout_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80) + 0x40;

    // caret-blink state machine (textTop < 1.0 so the visibility gate is taken).
    let mut s: u32 = 0x6b11_0001;
    for i in 0..6000u32 {
        let accum = fuzz_unit(&mut s, 0.5);
        let phase = (next(&mut s) >> 8) & 1;
        let dt = fuzz_unit(&mut s, 0.12);
        let text_top = fuzz_unit(&mut s, 0.9); // < 1.0
        e.write_u32(frame + 0xc, dt.to_bits()); // [ebp+0xc] = param_2 (dt)
        e.write_u32(TEXT_TOP, text_top.to_bits());
        e.write_u32(CARET_ACCUM, accum.to_bits());
        e.write_u32(CARET_PHASE, phase);
        e.set_reg(RegisterX86::EBP, frame);
        e.run(BLINK_BEGIN, BLINK_END);
        let (acc_o, ph_o) = caret_blink(accum, phase, dt);
        assert_f32(
            e.read_f32(CARET_ACCUM),
            acc_o,
            &format!("caret_blink accum[{i}] a={accum} ph={phase} dt={dt}"),
        );
        assert_eq!(
            e.read_u32(CARET_PHASE),
            ph_o,
            "caret_blink phase[{i}] a={accum} ph={phase} dt={dt}"
        );
    }

    // input-line baseline Y.
    for i in 0..4000u32 {
        let font_size = fuzz_unit(&mut s, 0.06);
        let text_top = fuzz_unit(&mut s, 1.0) - 0.5;
        e.write_u32(g::FONT_SIZE, font_size.to_bits());
        e.write_u32(TEXT_TOP, text_top.to_bits());
        e.set_reg(RegisterX86::EBP, frame);
        e.run(BASELINE_BEGIN, BASELINE_END);
        let bin = e.read_f32(frame - 0x8); // [ebp-0x8] = baseline
        assert_f32(
            bin,
            paint_baseline(font_size, text_top),
            &format!("paint_baseline[{i}] fs={font_size} tt={text_top}"),
        );
    }
}

const CARET_GEOM_BEGIN: u64 = 0x63b9af; // `fld [0xc4eac0]` (scaleX)
const CARET_GEOM_END: u64 = 0x63ba01; // `call 0x589f40` (the gx submit)
const SCALE_X: u64 = 0x00c4_eac0;
const CARET_RIGHT_LOCAL: i64 = -0x2c; // [ebp-0x2c] = caretRight
const CARET_BOTTOM_LOCAL: i64 = -0x1c; // [ebp-0x1c] = caretBottom

#[test]
fn draw_caret_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let region = e.alloc(0x80);
    let ebp = region + 0x40; // vertex locals at [ebp-0x38..] sit inside the region
    let pen = e.alloc(8); // {pen_x, pen_y}

    let mut s: u32 = 0x0ca7_0001;
    for i in 0..4000u32 {
        let pen_x = fuzz_unit(&mut s, 2.0) - 1.0;
        let pen_y = fuzz_unit(&mut s, 2.0) - 1.0;
        let scale_x = fuzz_unit(&mut s, 4.0); // device-space, ~1.0 or 1.0/range
        let font_size = fuzz_unit(&mut s, 0.06);
        e.write_u32(pen, pen_x.to_bits());
        e.write_u32(pen + 4, pen_y.to_bits());
        e.write_u32(SCALE_X, scale_x.to_bits());
        e.write_u32(g::FONT_SIZE, font_size.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ECX, pen);
        e.run(CARET_GEOM_BEGIN, CARET_GEOM_END);

        let bin_right = e.read_f32((ebp as i64 + CARET_RIGHT_LOCAL) as u64);
        let bin_bottom = e.read_f32((ebp as i64 + CARET_BOTTOM_LOCAL) as u64);
        let (right, bottom) = draw_caret(pen_x, pen_y, scale_x, font_size);
        assert_f32(
            bin_right,
            right,
            &format!("draw_caret right[{i}] sx={scale_x} px={pen_x}"),
        );
        assert_f32(
            bin_bottom,
            bottom,
            &format!("draw_caret bottom[{i}] fs={font_size} py={pen_y}"),
        );
    }
}
