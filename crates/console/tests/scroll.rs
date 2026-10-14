//! The scrollbar drag and the mouse-to-line map against the exe. `mouse_down` (`0x63c0b0`) and
//! `mouse_move` (`0x63c400`) run whole with their callees stubbed; `line_at_mouse` (`0x63c220`) runs
//! the index arithmetic slice `0x63c223` to `0x63c23d`, through the embedded `_ftol`, with the mouse
//! Y in a set-up frame, and reads the truncated index from `eax`.

mod common;

use benilla_re_console::scroll::{
    line_at_mouse_index, mouse_down_drag, mouse_move_drag_line_metric, MouseDown,
};
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::{assert_f32, fuzz_unit, g, next};

const MOUSE_MOVE_DRAG: u64 = 0x63c400;
const MOUSE_DOWN_DRAG: u64 = 0x63c0b0;
const FOCUS_CHECK: u64 = 0x41f8f0; // returns 0 when the console has focus (enter the body)
const SCROLL_RENDER: u64 = 0x63c290; // re-render, stubbed to a no-op
const GET_LINE_AT_MOUSE: u64 = 0x63c220; // returns the selected CONSOLELINE* (stdcall, 1 arg)
const TEXT_COPY: u64 = 0x64a5a0; // SStrCopy, stubbed to a no-op
const SCROLL_CLEAR: u64 = 0x63c070; // clear scroll-drag state (no-op)
const G_OPEN: u64 = 0x00c4_eac8;
const G_SNAP: u64 = 0x00c4_ec38; // also the scroll-drag flag (mouse_down thumb path)
const G_SELECT: u64 = 0x00c4_ec3c;
const G_BAND_TOP: u64 = 0x0086_4570;
const G_BAND_BOTTOM: u64 = 0x0086_4568;

/// `(line_metric, font_size, mouse_y)` for which the line index `(lineMetric − (1 − mouseY)) /
/// fontSize` is NaN, infinite or past `i32` and `i64`, so `_ftol` returns the low dword of its
/// qword, 0 for the indefinite.
const WIDE_INDEX: [(f32, f32, f32); 8] = [
    (0.5, 0.02, f32::NAN),
    (0.5, f32::NAN, 0.7),
    (0.5, 0.0, 0.7),
    (0.5, 1.0e-30, 0.7),
    (0.5, 1.0e-12, 0.7),
    (0.5, 1.0e-8, 0.7),
    (0.5, -1.0e-30, 0.7),
    (f32::INFINITY, 0.02, 0.7),
];

#[test]
fn mouse_down_drag_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(FOCUS_CHECK, 0, 0); // focused
    e.stub(SCROLL_CLEAR, 0, 0);
    e.stub(TEXT_COPY, 0, 0);
    e.stub(SCROLL_RENDER, 0, 0);
    let line_buf = e.alloc(0x20);
    e.write_u32(line_buf + 8, line_buf as u32); // [line+8] = a readable text ptr
    e.stub(GET_LINE_AT_MOUSE, line_buf as u32, 1); // always return a valid line (select path)
    let param = e.alloc(0x20);

    let mut s: u32 = 0x0d0_0d001u32;
    for i in 0..6000u32 + WIDE_INDEX.len() as u32 {
        let mut line_metric = fuzz_unit(&mut s, 0.8) + 0.1;
        let mut font_size = fuzz_unit(&mut s, 0.04) + 0.01;
        // fVar2 = 1.0 − mouseY spans [-0.1, lineMetric·1.3] → off-region / thumb / select coverage.
        let fvar2_target = fuzz_unit(&mut s, line_metric * 1.3 + 0.1) - 0.1;
        let mut mouse_y = 1.0 - fvar2_target;
        if let Some(&(lm, fs, my)) = WIDE_INDEX.get((i as usize).wrapping_sub(6000)) {
            (line_metric, font_size, mouse_y) = (lm, fs, my);
        }
        let drag_x = next(&mut s);
        e.write_u32(param + 0x18, mouse_y.to_bits());
        e.write_u32(param + 0x14, drag_x);
        e.write_u32(G_OPEN, 1);
        e.write_u32(g::LINE_METRIC, line_metric.to_bits());
        e.write_u32(g::FONT_SIZE, font_size.to_bits());
        // sentinels
        e.write_u32(G_SNAP, 0xdead_beef);
        e.write_u32(G_SELECT, 0xdead_beef);
        e.write_u32(G_BAND_TOP, 0xdead_beef);
        e.write_u32(G_BAND_BOTTOM, 0xdead_beef);

        e.call(MOUSE_DOWN_DRAG, param as u32, 0, &[]);
        let ours = mouse_down_drag(line_metric, font_size, mouse_y);
        let ctx = format!(
            "mouse_down_drag[{i}] lm={line_metric} fs={font_size} my={mouse_y} -> {ours:?}"
        );
        match ours {
            MouseDown::OffRegion => {
                assert_eq!(
                    e.read_u32(G_SNAP),
                    0xdead_beef,
                    "{ctx}: off-region must not set scroll-drag"
                );
                assert_eq!(
                    e.read_u32(G_SELECT),
                    0xdead_beef,
                    "{ctx}: off-region must not select"
                );
            }
            MouseDown::ThumbDrag => {
                assert_eq!(e.read_u32(G_SNAP), 1, "{ctx}: thumb must set scroll-drag");
            }
            MouseDown::Select {
                band_top,
                band_bottom,
            } => {
                assert_eq!(e.read_u32(G_SELECT), 1, "{ctx}: select flag");
                assert_f32(
                    e.read_f32(G_BAND_TOP),
                    f32::from_bits(band_top),
                    &format!("{ctx} bandTop"),
                );
                assert_f32(
                    e.read_f32(G_BAND_BOTTOM),
                    f32::from_bits(band_bottom),
                    &format!("{ctx} bandBottom"),
                );
            }
        }
    }
}
const G_DRAG_CUR: u64 = 0x00c4_ec44;

#[test]
fn mouse_move_drag_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(FOCUS_CHECK, 0, 0); // focused → enter the body
    e.stub(SCROLL_RENDER, 0, 0);
    let param = e.alloc(0x20);

    let mut s: u32 = 0x3072_0001;
    for i in 0..5000u32 {
        let snap = (next(&mut s) >> 8) & 1;
        let mouse_y = fuzz_unit(&mut s, 1.4) - 0.2; // device Y, spanning the clamp boundary
        let font_size = fuzz_unit(&mut s, 0.04) + 0.01;
        let line_metric_in = fuzz_unit(&mut s, 1.0);
        let drag_x = next(&mut s);
        e.write_u32(param + 0x18, mouse_y.to_bits()); // [esi+0x18] = mouseY
        e.write_u32(param + 0x14, drag_x); // [esi+0x14] = dragX
        e.write_u32(G_OPEN, 1);
        e.write_u32(G_SNAP, snap);
        e.write_u32(G_SELECT, 0); // skip the re-render branch
        e.write_u32(g::LINE_METRIC, line_metric_in.to_bits());
        e.write_u32(g::FONT_SIZE, font_size.to_bits());
        let _ = G_DRAG_CUR;

        e.call(MOUSE_MOVE_DRAG, param as u32, 0, &[]);
        let bin_lm = e.read_f32(g::LINE_METRIC);
        let ours = mouse_move_drag_line_metric(snap, line_metric_in, font_size, mouse_y);
        assert_f32(
            bin_lm,
            ours,
            &format!(
                "mouse_move_drag[{i}] snap={snap} my={mouse_y} fs={font_size} lm={line_metric_in}"
            ),
        );
    }
}

const LAM_BEGIN: u64 = 0x63c223; // `fld [0x7ff9d8]` (after the prologue)
const LAM_END: u64 = 0x63c23d; // `mov edx,eax`, right after `call _ftol` returns

#[test]
fn line_at_mouse_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let region = e.alloc(0x200);
    let frame = region + 0x100; // ebp; the mouseY arg sits at [ebp+0x8], _ftol pushes below esp

    let mut s: u32 = 0x1a30_0001;
    for i in 0..6000u32 + WIDE_INDEX.len() as u32 {
        // mouseY ∈ [0,1] (device Y); lineMetric ∈ [0,1.2]; fontSize ∈ [0.01,0.05], then the
        // wide-index probes
        let mut mouse_y = fuzz_unit(&mut s, 1.0);
        let mut line_metric = fuzz_unit(&mut s, 1.2);
        let mut font_size = fuzz_unit(&mut s, 0.04) + 0.01;
        if let Some(&(lm, fs, my)) = WIDE_INDEX.get((i as usize).wrapping_sub(6000)) {
            (line_metric, font_size, mouse_y) = (lm, fs, my);
        }
        e.write_u32(frame + 0x8, mouse_y.to_bits()); // [ebp+0x8] = mouseY arg
        e.write_u32(g::LINE_METRIC, line_metric.to_bits());
        e.write_u32(g::FONT_SIZE, font_size.to_bits());
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESP, frame - 0x40); // room for the _ftol call's pushes
        e.run(LAM_BEGIN, LAM_END);

        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        let ours = line_at_mouse_index(mouse_y, line_metric, font_size);
        assert_eq!(
            bin, ours,
            "line_at_mouse[{i}] mouseY={mouse_y} lm={line_metric} fs={font_size}: bin={bin} ours={ours}"
        );
    }
}
