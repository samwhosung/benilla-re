//! `console_scale_init` scale-seam difftest (0x63b460): run the scale slice over a synthesized
//! viewport rect (placed in a stack frame at `[ebp-0x10..-0x4]`) and assert `scaleX`/`scaleY` match.

mod common;

use benilla_re_console::scale::console_scale;
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::{assert_f32, fuzz_unit, next};

const SCALE_BEGIN: u64 = 0x63b48a; // `fld [ebp-0x4]` (rect.bottom)
const SCALE_END: u64 = 0x63b4e6; // `push esi`, after both scale stores
const SCALE_X: u64 = 0x00c4_eac0;
const SCALE_Y: u64 = 0x00c4_ea8c;

#[test]
fn console_scale_init_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let region = e.alloc(0x80);
    let ebp = region + 0x40; // locals at [ebp-0x10..-0x4] sit inside the region

    let mut s: u32 = 0x5ca1_0001;
    for i in 0..4000u32 {
        // rect = {left, top, right, bottom} at [ebp-0x10], [ebp-0xc], [ebp-0x8], [ebp-0x4].
        let mut r = [
            fuzz_unit(&mut s, 2.0) - 1.0,
            fuzz_unit(&mut s, 2.0) - 1.0,
            fuzz_unit(&mut s, 2.0) - 1.0,
            fuzz_unit(&mut s, 2.0) - 1.0,
        ];
        // occasionally force a zero range to hit the div-guard (height: r[3]==r[1]; width: r[2]==r[0])
        match (next(&mut s) >> 8) % 5 {
            0 => r[3] = r[1],
            1 => r[2] = r[0],
            _ => {}
        }
        for (k, &v) in r.iter().enumerate() {
            e.write_u32(ebp - 0x10 + 4 * k as u64, v.to_bits());
        }
        e.write_u32(SCALE_X, 0xdead_beef);
        e.write_u32(SCALE_Y, 0xdead_beef);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(SCALE_BEGIN, SCALE_END);

        assert_f32(
            e.read_f32(SCALE_X),
            console_scale(r[3], r[1]),
            &format!("scale_x[{i}] rect={r:?}"),
        );
        assert_f32(
            e.read_f32(SCALE_Y),
            console_scale(r[2], r[0]),
            &format!("scale_y[{i}] rect={r:?}"),
        );
    }
}
