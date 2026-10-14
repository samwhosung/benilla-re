//! The console slide (`0x63bd50`) against the exe, whole function, on synthesized state: the new
//! `textTop` must match bit for bit. The deferred-command counter is held at 0 (so the `0x63ce00`
//! branch never runs) and the viewport commit (`0x4429c0`) is stubbed to a no-op.

mod common;

use benilla_re_console::slide::slide_animate;
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, fuzz_unit, g, next};

const SLIDE_ANIMATE: u64 = 0x63bd50;
const GX_COMMIT: u64 = 0x4429c0; // CGxDevice viewport commit stubbed to a no-op

#[test]
fn slide_animate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(GX_COMMIT, 0, 0); // __fastcall, no stack args
    let dt_ptr = e.alloc(4);

    let mut s: u32 = 0x5c0_0001;
    for i in 0..4000u32 {
        let open = next(&mut s) & 1;
        let snap = if (next(&mut s) >> 8) & 7 == 0 { 1 } else { 0 };
        let line_metric = fuzz_unit(&mut s, 1.2);
        // text_top across [-0.1, 1.5): below/at/above the target, incl. the exact-equal case.
        let text_top = match (next(&mut s) >> 8) % 4 {
            0 => fuzz_unit(&mut s, 1.5),
            1 => 1.0 - line_metric, // == the open target (the no-change path)
            2 => 1.0,               // == the closed target
            _ => fuzz_unit(&mut s, 1.5) - 0.1,
        };
        let dt = fuzz_unit(&mut s, 0.1);

        // install state
        e.write_u32(g::OPEN_FLAG, open);
        e.write_u32(g::SNAP_FLAG, snap);
        e.write_u32(g::LINE_METRIC, line_metric.to_bits());
        e.write_u32(g::TEXT_TOP, text_top.to_bits());
        e.write_u32(g::IDLE_EXEC_CTR, 0); // no deferred Execute
        e.write_u32(g::IDLE_EXEC_CH, 0);
        e.write_u32(dt_ptr, dt.to_bits());

        e.call(SLIDE_ANIMATE, dt_ptr as u32, 0, &[]);
        let bin = e.read_f32(g::TEXT_TOP);

        let ours = slide_animate(open, line_metric, snap, text_top, dt);
        assert_f32(
            bin,
            ours,
            &format!(
                "slide_animate[{i}] open={open} snap={snap} lm={line_metric} top={text_top} dt={dt}"
            ),
        );
    }
}
