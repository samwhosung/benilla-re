//! The screen size `CGlueMgr::Resume` stores (`systems/glue/manager.md#screen-size-and-aspect`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_glue::init::viewport_extent;
use common::*;

/// The two `fld; fsub; call __ftol` pairs of `CGlueMgr::Resume`, to the call of `GetScreenAspect`.
const SEAM: u64 = 0x46a7f7;
const SEAM_END: u64 = 0x46a817;
/// The stored width and height.
const SCREEN_W: u64 = 0x00b4_1e1c;
const SCREEN_H: u64 = 0x00b4_1e20;

/// Runs `0x46a7f7` to `0x46a817` of `CGlueMgr::Resume` (`0x46a7b0`) with the four rectangle edges in
/// the frame and `__ftol` running for real, and compares the width and height stored at `0xb41e1c`
/// and `0xb41e20`. The extents include fractions, negative differences and values past the `i32` range.
#[test]
fn glue_init_viewport_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80) + 0x40; // ebp: the rectangle's floats at [ebp-0x1c], [ebp-0x24], [ebp-0x20], [ebp-0x28]
    let stack = e.alloc(0x100) + 0x80; // esp, with room below for the frame of `__ftol`

    // Extents past the `i32` range, where `__ftol` keeps the low 32 bits of the 64-bit result, past
    // 64 bits, where it gives 0, and NaN: `(hi_a, lo_a, hi_b, lo_b)`.
    let probes: [(f32, f32, f32, f32); 6] = [
        (3.0e9, 0.0, -3.0e9, 0.0),
        (6.0e9, 1.0, 2147483648.0, 0.0),
        (1.0e19, 0.0, f32::NAN, 0.0),
        (f32::INFINITY, 0.0, 0.0, f32::INFINITY),
        (0.5, 0.0, -0.5, 0.0),
        (1.5, 0.25, -1.5, 0.25),
    ];
    let mut s: u32 = 0x1ce4_0001;
    for i in 0..probes.len() as u32 + 6000 {
        let (hi_a, lo_a, hi_b, lo_b) = if let Some(&p) = probes.get(i as usize) {
            p
        } else {
            // Each extent is a high edge less a low one: 0 to 8192 with fractions, sometimes negative.
            let lo_a = fuzz_unit(&mut s, 4096.0);
            let hi_a = lo_a + (fuzz_unit(&mut s, 8192.0) - 1024.0);
            let lo_b = fuzz_unit(&mut s, 4096.0);
            let hi_b = lo_b + (fuzz_unit(&mut s, 8192.0) - 1024.0);
            (hi_a, lo_a, hi_b, lo_b)
        };
        e.write_u32(frame - 0x1c, hi_a.to_bits());
        e.write_u32(frame - 0x24, lo_a.to_bits());
        e.write_u32(frame - 0x20, hi_b.to_bits());
        e.write_u32(frame - 0x28, lo_b.to_bits());
        e.write_u32(SCREEN_W, 0xdead_beef);
        e.write_u32(SCREEN_H, 0xdead_beef);
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESP, stack);
        e.run(SEAM, SEAM_END);

        let bin_a = e.read_u32(SCREEN_W) as i32;
        let bin_b = e.read_u32(SCREEN_H) as i32;
        assert_eq!(
            bin_a,
            viewport_extent(hi_a, lo_a),
            "glue_init_viewport[{i}] extent A hi={hi_a} lo={lo_a}"
        );
        assert_eq!(
            bin_b,
            viewport_extent(hi_b, lo_b),
            "glue_init_viewport[{i}] extent B hi={hi_b} lo={lo_b}"
        );
    }
}
