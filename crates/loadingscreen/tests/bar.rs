//! The progress-bar quad against the exe: the slice `0x4071d1` to `0x407229` of `0x407150` (the quad
//! geometry), run in a set-up `ebp` frame with `esi` the bar descriptor and `0x882be4` the progress;
//! the four corner locals `(x0, y0, x1, y1)` are read back. Both the border (`isFill` 0, `x1 = w * 0.5 + cx`)
//! and the fill (`x1 = progress * w + x0`) are covered.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_loadingscreen::bar::progress_bar_quad;
use common::{assert_f32, fuzz, next};

const SLICE_BEGIN: u64 = 0x4071d1;
const SLICE_END: u64 = 0x407229;
const BAR_FILL: u64 = 0x0088_2be4; // progress

#[test]
fn progress_bar_quad_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let desc = e.alloc(0x20); // {tex@0, isFill@4, cx@8, cy@c, w@10, h@14}
    let ebp = frame + 0x80;
    let mut s: u32 = 0x4071_0001;
    for i in 0..6000u32 {
        let cx = fuzz(&mut s, 1.0);
        let cy = fuzz(&mut s, 1.0);
        let w = fuzz(&mut s, 1.0) + 0.1;
        let h = fuzz(&mut s, 0.5) + 0.1;
        let is_fill = (next(&mut s) >> 8) & 1 == 1;
        let progress = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1.5) as f32;
        e.write_u32(desc + 0x4, if is_fill { 1 } else { 0 });
        e.write_u32(desc + 0x8, cx.to_bits());
        e.write_u32(desc + 0xc, cy.to_bits());
        e.write_u32(desc + 0x10, w.to_bits());
        e.write_u32(desc + 0x14, h.to_bits());
        e.write_u32(BAR_FILL, progress.to_bits());
        e.set_reg(RegisterX86::ESI, desc);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, ebp - 0x40);
        e.set_reg(RegisterX86::FPCW, 0x027F);
        e.run(SLICE_BEGIN, SLICE_END);
        let bin = [
            e.read_f32(ebp - 0x4),  // x0
            e.read_f32(ebp - 0x30), // y0
            e.read_f32(ebp - 0x28), // x1
            e.read_f32(ebp - 0x18), // y1
        ];
        let ours = progress_bar_quad(cx, cy, w, h, is_fill, progress);
        let names = ["x0", "y0", "x1", "y1"];
        for k in 0..4 {
            assert_f32(
                bin[k],
                ours[k],
                &format!("bar[{i}] fill={is_fill} {}", names[k]),
            );
        }
    }
}
