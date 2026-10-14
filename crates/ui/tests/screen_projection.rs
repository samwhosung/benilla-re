//! The world to screen projection `0x483ee0` run from the exe.
//!
//! The matrix transform `0x7bcb40` is stubbed to return a pointer to a chosen clip vector
//! `[x, y, z, w]`, so the test exercises the near cull, the perspective divide, the viewport map,
//! the frame-origin shift and the on-screen test on that vector. The output floats are read back
//! and compared by their bits (two NaNs compare equal, as the quiet NaN's payload is not part of the
//! specification). Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::screen_rect::{project_to_screen as proj, ScreenProj};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// A finite float in `[-range, range)` from the LCG's high bits.
fn ff(s: &mut u32, range: f32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
    m * 2.0 * range - range
}

/// A non-zero, finite magnitude spanning moderate / tiny (large inv) / very large (tiny inv).
fn nzval(s: &mut u32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
    let mag = match (next(s) >> 8) % 4 {
        0 => m * 1000.0 + 0.5,
        1 => m * 0.02 + 1e-4,
        2 => m * 1.0e6 + 1.0,
        _ => m * 200.0 + 0.125,
    };
    if next(s) & 0x800 != 0 {
        -mag
    } else {
        mag
    }
}

/// Compare a transcribed `f32` against the binary's stored `f32`: bit-exact, both-NaN passes.
fn assert_bits(mine: f32, bin: f32, ctx: &str) {
    if mine.is_nan() && bin.is_nan() {
        return;
    }
    assert_eq!(
        mine.to_bits(),
        bin.to_bits(),
        "{ctx}: rust {:#010x} ({mine}) vs binary {:#010x} ({bin})",
        mine.to_bits(),
        bin.to_bits()
    );
}

// ----------------------------------------------------------------------------------------------
// geo_483ee0 — world→screen
// ----------------------------------------------------------------------------------------------

/// The DPI-scale globals the binary multiplies the viewport-mapped coords by (`FUN_0041ad80`).
const SCALE_X_VA: u64 = 0x0083_2a44;

const SCALE_Y_VA: u64 = 0x0083_2a48;

#[derive(Clone, Copy)]
struct ProjCase {
    clip: [f32; 4],
    znear: f32,
    vp: [f32; 4], // [+0x3a0, +0x3a4, +0x3a8, +0x3ac]
    scale: [f32; 2],
    rect: [f32; 4], // [+0x64, +0x68, +0x6c, +0x70]
}

#[test]
fn geo_483ee0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // `this` must span +0x65b8 (the camera-sub-struct pointer). Pure field reads — never written.
    let this_va = emu.alloc(0x6600);
    let camera_va = emu.alloc(0x40); // +0x08 campos (subtracted, discarded), +0x38 znear
    let clip_va = emu.alloc(0x10); // FUN_007bcb40's controlled clip-space output
    let input_va = emu.alloc(0x10); // the world position (read, then fed to the stubbed transform)
    let out_va = emu.alloc(0x10);

    emu.write_u32(this_va + 0x65b8, camera_va as u32);
    emu.write_u32(this_va + 0x3b0, 0); // matrix ptr — read & pushed, but FUN_007bcb40 is stubbed
    for k in 0..3 {
        emu.write_u32(input_va + (k as u64) * 4, 0); // world pos (irrelevant: transform stubbed)
        emu.write_u32(camera_va + 0x08 + (k as u64) * 4, 0); // campos (irrelevant)
    }
    // Stub the 4×4 transform: return a pointer to our controlled clip vector (it cleans 1 stack arg).
    emu.stub(0x7bcb40, clip_va as u32, 1);

    // Explicit probes straddling cull / on-screen / each off-screen edge / origin shift / w≈0.
    let mut cases: Vec<ProjCase> = vec![
        // on-screen centre: ndc=(0.5,0.5) -> screen=(512,384) inside a 1024×768 rect.
        ProjCase {
            clip: [0.0, 0.0, 5.0, 1.0],
            znear: 1.0,
            vp: [0.0, 0.0, 768.0, 1024.0],
            scale: [1.0, 1.0],
            rect: [0.0, 0.0, 768.0, 1024.0],
        },
        // near-plane cull: znear > clip.z.
        ProjCase {
            clip: [0.0, 0.0, 5.0, 1.0],
            znear: 10.0,
            vp: [0.0, 0.0, 768.0, 1024.0],
            scale: [1.0, 1.0],
            rect: [0.0, 0.0, 768.0, 1024.0],
        },
        // off-screen left: ndc.x < 0 -> screen_x < 0.
        ProjCase {
            clip: [-2.0, 0.0, 5.0, 1.0],
            znear: 1.0,
            vp: [0.0, 0.0, 768.0, 1024.0],
            scale: [1.0, 1.0],
            rect: [0.0, 0.0, 768.0, 1024.0],
        },
        // off-screen right: ndc.x > 1 -> screen_x > width.
        ProjCase {
            clip: [3.0, 0.0, 5.0, 1.0],
            znear: 1.0,
            vp: [0.0, 0.0, 768.0, 1024.0],
            scale: [1.0, 1.0],
            rect: [0.0, 0.0, 768.0, 1024.0],
        },
        // off-screen below: ndc.y > 1 -> screen_y > height.
        ProjCase {
            clip: [0.0, 4.0, 5.0, 1.0],
            znear: 1.0,
            vp: [0.0, 0.0, 768.0, 1024.0],
            scale: [1.0, 1.0],
            rect: [0.0, 0.0, 768.0, 1024.0],
        },
        // negative frame origins exercise the X (80-bit) / Y (f32) origin-shift legs.
        ProjCase {
            clip: [0.1, -0.2, 7.0, 1.3],
            znear: 0.5,
            vp: [10.0, 20.0, 800.0, 1100.0],
            scale: [1.25, 0.875],
            rect: [-30.5, -17.25, 760.0, 1050.0],
        },
        // clip.w near zero -> huge invW -> ndc/screen blow up (off-screen, finite or inf).
        ProjCase {
            clip: [0.3, 0.4, 9.0, 1e-6],
            znear: 1.0,
            vp: [0.0, 0.0, 768.0, 1024.0],
            scale: [1.0, 1.0],
            rect: [0.0, 0.0, 768.0, 1024.0],
        },
        // equal-to-near (znear == clip.z): NOT culled (cull is strict `>`).
        ProjCase {
            clip: [0.0, 0.0, 5.0, 1.0],
            znear: 5.0,
            vp: [0.0, 0.0, 768.0, 1024.0],
            scale: [1.0, 1.0],
            rect: [0.0, 0.0, 768.0, 1024.0],
        },
    ];

    let mut s = 0x483e_00e0u32;
    for _ in 0..6000 {
        // Bias toward a sane screen-ish viewport/rect so on-screen hits actually occur, but with
        // wide clip / scale / znear to straddle cull and both off-screen edges.
        let w = 256.0 + ff(&mut s, 1.0).abs() * 1600.0;
        let h = 256.0 + ff(&mut s, 1.0).abs() * 1200.0;
        cases.push(ProjCase {
            clip: [
                ff(&mut s, 4.0),
                ff(&mut s, 4.0),
                ff(&mut s, 12.0),
                nzval(&mut s),
            ],
            znear: ff(&mut s, 8.0),
            vp: [
                ff(&mut s, 30.0),
                ff(&mut s, 30.0),
                ff(&mut s, 30.0) + h,
                ff(&mut s, 30.0) + w,
            ],
            scale: [0.5 + ff(&mut s, 0.5).abs(), 0.5 + ff(&mut s, 0.5).abs()],
            rect: [
                ff(&mut s, 40.0),
                ff(&mut s, 40.0),
                ff(&mut s, 40.0) + h,
                ff(&mut s, 40.0) + w,
            ],
        });
    }

    let mut tally = [0u32; 3]; // [culled, on_screen, off_screen]
    for (i, c) in cases.iter().enumerate() {
        for k in 0..4 {
            emu.write_u32(clip_va + (k as u64) * 4, c.clip[k].to_bits());
        }
        emu.write_u32(camera_va + 0x38, c.znear.to_bits());
        emu.write_u32(this_va + 0x3a0, c.vp[0].to_bits());
        emu.write_u32(this_va + 0x3a4, c.vp[1].to_bits());
        emu.write_u32(this_va + 0x3a8, c.vp[2].to_bits());
        emu.write_u32(this_va + 0x3ac, c.vp[3].to_bits());
        emu.write_u32(SCALE_X_VA, c.scale[0].to_bits());
        emu.write_u32(SCALE_Y_VA, c.scale[1].to_bits());
        emu.write_u32(this_va + 0x64, c.rect[0].to_bits());
        emu.write_u32(this_va + 0x68, c.rect[1].to_bits());
        emu.write_u32(this_va + 0x6c, c.rect[2].to_bits());
        emu.write_u32(this_va + 0x70, c.rect[3].to_bits());
        // poison the out slots to confirm the cull path leaves them untouched.
        emu.write_u32(out_va, 0xdead_beef);
        emu.write_u32(out_va + 4, 0xdead_beef);

        let ret = emu.call(
            0x483ee0,
            this_va as u32,
            0,
            &[input_va as u32, out_va as u32],
        ) & 0xff;

        let mine = proj(
            c.clip, c.znear, c.vp[0], c.vp[1], c.vp[2], c.vp[3], c.scale[0], c.scale[1], c.rect[0],
            c.rect[1], c.rect[2], c.rect[3],
        );
        match mine {
            ScreenProj::Culled => {
                assert_eq!(
                    ret, 0,
                    "geo_483ee0 case {i}: culled but binary returned {ret}"
                );
                assert_eq!(
                    emu.read_u32(out_va),
                    0xdead_beef,
                    "geo_483ee0 case {i}: cull wrote out[0]"
                );
                assert_eq!(
                    emu.read_u32(out_va + 4),
                    0xdead_beef,
                    "geo_483ee0 case {i}: cull wrote out[1]"
                );
                tally[0] += 1;
            }
            ScreenProj::Mapped { x, y, on_screen } => {
                assert_bits(
                    x,
                    emu.read_f32(out_va),
                    &format!("geo_483ee0 case {i} out[0]"),
                );
                assert_bits(
                    y,
                    emu.read_f32(out_va + 4),
                    &format!("geo_483ee0 case {i} out[1]"),
                );
                assert_eq!(
                    ret, on_screen as u32,
                    "geo_483ee0 case {i}: on_screen {on_screen} vs binary al {ret}"
                );
                tally[if on_screen { 1 } else { 2 }] += 1;
            }
        }
    }

    eprintln!(
        "geo_483ee0: {} bit-exact (perspective divide + viewport map + clip test); \
         [culled={} on_screen={} off_screen={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_483ee0 outcome bucket {i} never hit");
    }
}
