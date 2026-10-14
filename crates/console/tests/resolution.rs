//! `resolution_validate` aspect-filter difftest (0x63aa00): run the `fild width; fidiv height;
//! fcomp minAspect; fnstsw` slice with a synthesized (width, height) and compare the x87 C0 bit
//! (aspect-below-min) against our transcription.

mod common;

use benilla_re_console::resolution::resolution_aspect_below_min;
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::next;

const ASPECT_BEGIN: u64 = 0x63aa8a; // `fild [ebp-0x14]` (width)
const ASPECT_END: u64 = 0x63aa9b; // right after `fnstsw ax`

#[test]
fn resolution_validate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100) + 0x80;

    let mut s: u32 = 0x4e50_0001;
    // Real display modes (640×480 … 1920×1200) plus fuzzed pairs spanning the 1.248 boundary.
    let modes: &[(i32, i32)] = &[
        (640, 480),
        (800, 600),
        (1024, 768),
        (1280, 1024),
        (1280, 800),
        (1600, 1200),
        (1920, 1200),
        (320, 240),
        (1, 1),
    ];
    for i in 0..6000u32 {
        let (width, height) = if (i as usize) < modes.len() {
            modes[i as usize]
        } else {
            let w = 1 + ((next(&mut s) >> 8) % 2200) as i32;
            let h = 1 + ((next(&mut s) >> 8) % 1700) as i32;
            (w, h)
        };
        e.write_u32(frame - 0x14, width as u32); // [ebp-0x14] = width
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::EAX, height as u64); // mov [ebp-0x24], eax → height
        e.run(ASPECT_BEGIN, ASPECT_END);

        let status = e.get_reg(RegisterX86::EAX) as u32;
        let bin_c0 = (status & 0x100) != 0; // x87 C0 = aspect < minAspect
        let ours = resolution_aspect_below_min(width, height);
        assert_eq!(
            bin_c0, ours,
            "resolution_aspect[{i}] {width}x{height}: bin_C0={bin_c0} ours={ours} status={status:#06x}"
        );
    }
}
