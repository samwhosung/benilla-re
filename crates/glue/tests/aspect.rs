//! The glue screen's aspect (`systems/glue/manager.md#screen-size-and-aspect`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_glue::aspect::{glue_aspect, glue_onresize_changed};
use common::*;

/// The division of `GetScreenAspect`: `fild [ebp-0xc]` (the width) at `0x46ab97`, `fidiv [ebp-0x8]`,
/// then the epilogue at `0x46ab9e`, which the test overwrites by an `fstp dword` (6 bytes) so that
/// the result can be read; the run stops after it.
const ASPECT_FILD: u64 = 0x46ab97;
const ASPECT_EPILOGUE: u64 = 0x46ab9e;
const ASPECT_HALT: u64 = 0x46aba4;
/// The aspect test of `CheckGlueScreenSize`, and the globals it reads and writes: the stored aspect
/// and the rebuild flag.
const RESIZE_CMP: u64 = 0x46ac5b;
const RESIZE_CMP_END: u64 = 0x46ac86;
const OLD_ASPECT: u64 = 0x00b4_1e24;
const RESIZE_DIRTY: u64 = 0x00b4_1d8c;

/// Runs the division of `GetScreenAspect` (`0x46ab40`), `0x46ab97` to `0x46ab9e`, on widths and heights
/// including 4:3, small values, resolutions and large values, with the epilogue replaced to read `st0`
/// as an `f32`.
#[test]
fn glue_aspect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the width at [ebp-0xc], the height at [ebp-0x8]
    let stack = e.alloc(0x40) + 0x20; // esp, with room for `pop esi`
    let result = e.alloc(4);
    let mut fstp = vec![0xd9, 0x1d]; // fstp dword [result]
    fstp.extend_from_slice(&(result as u32).to_le_bytes());
    e.write(ASPECT_EPILOGUE, &fstp);

    let mut s: u32 = 0x4a17_0001;
    for i in 0..6000u32 {
        // The 4:3 default, small pairs, resolutions and large values.
        let (w, h) = match (next(&mut s) >> 8) % 4 {
            0 => (4i32, 3i32),
            1 => (
                1 + ((next(&mut s) >> 8) % 4096) as i32,
                1 + ((next(&mut s) >> 8) % 4096) as i32,
            ),
            2 => (
                1 + ((next(&mut s) >> 8) % 32) as i32,
                1 + ((next(&mut s) >> 8) % 32) as i32,
            ),
            _ => (
                1 + ((next(&mut s) >> 8) % 0x10_0000) as i32,
                1 + ((next(&mut s) >> 8) % 0x10_0000) as i32,
            ),
        };
        e.write_u32(frame - 0xc, w as u32);
        e.write_u32(frame - 0x8, h as u32);
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESP, stack);
        e.run(ASPECT_FILD, ASPECT_HALT);
        let bin = e.read_f32(result);
        assert_f32(
            bin,
            glue_aspect(w, h),
            &format!("glue_aspect[{i}] w={w} h={h}"),
        );
    }
}

/// Runs the aspect test of `CheckGlueScreenSize` (`0x46abe0`), `0x46ac5b` to `0x46ac86`, with the new
/// aspect at `[ebp-4]` and the stored one at `0xb41e24`, on differences of 0, about 0.01, within 0.01 and
/// larger. It compares the rebuild flag at `0xb41d8c` and the stored aspect.
#[test]
fn glue_onresize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40) + 0x20; // ebp: the new aspect at [ebp-0x4]
    let stack = e.alloc(0x40) + 0x20;

    let mut s: u32 = 0x77c9_0001;
    for i in 0..8000u32 {
        let old = fuzz_unit(&mut s, 2.5) + 0.5; // 0.5 to 3.0
                                                // The differences straddle 0.01, with exactly 0.01 and 0, so both outcomes occur.
        let new = match (next(&mut s) >> 8) % 4 {
            0 => old,                                    // no difference
            1 => (f64::from(old) + 0.01) as f32,         // about 0.01
            2 => old + (fuzz_unit(&mut s, 0.02) - 0.01), // within 0.01
            _ => old + (fuzz_unit(&mut s, 0.5) - 0.25),  // larger
        };
        e.write_u32(frame - 0x4, new.to_bits());
        e.write_u32(OLD_ASPECT, old.to_bits());
        e.write_u32(RESIZE_DIRTY, 0xdead_beef);
        e.set_reg(RegisterX86::EBP, frame);
        e.set_reg(RegisterX86::ESP, stack);
        e.run(RESIZE_CMP, RESIZE_CMP_END);

        let ctx = format!("glue_onresize[{i}] old={old} new={new}");
        match glue_onresize_changed(old, new) {
            None => {
                assert_eq!(
                    e.read_u32(RESIZE_DIRTY),
                    0xdead_beef,
                    "{ctx}: sub-epsilon must not set dirty"
                );
                assert_eq!(
                    e.read_f32(OLD_ASPECT).to_bits(),
                    old.to_bits(),
                    "{ctx}: sub-epsilon must not overwrite stored aspect"
                );
            }
            Some(stored) => {
                assert_eq!(
                    e.read_u32(RESIZE_DIRTY),
                    1,
                    "{ctx}: significant Δ sets dirty"
                );
                assert_f32(
                    e.read_f32(OLD_ASPECT),
                    stored,
                    &format!("{ctx} stored aspect"),
                );
            }
        }
    }
}
