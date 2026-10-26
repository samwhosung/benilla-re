//! The window procedures' `WM_SIZING` arithmetic (`src/window.rs`) against the exe. Each helper
//! `0x5910a0` to `0x591200` is called whole, with the ratio `[0xc0ef60]` and the minimum size
//! `[0xc0ef64]`, `[0xc0ef68]` written first; `__ftol` (`0x40a2b0`) runs inside. The two
//! procedures' sizing blocks run as slices: Direct3D's `0x5989d7` to `0x598af1` and OpenGL's
//! `0x58c841` to `0x58c95d`, each from the load of the `RECT` through its case table to the
//! write-back, for rectangles with a non-zero width and height (the zero ones leave the slice).

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_gx::window::{
    fit_bottom, fit_left, fit_right, fit_top, min_bottom, min_left, min_right, min_top, sizing,
    SizingRect,
};

mod common;
use common::{next, SLICE_ESP};

const RATIO: u64 = 0xc0ef60;
const MIN_W: u64 = 0xc0ef64;
const MIN_H: u64 = 0xc0ef68;

type Helper = fn(&mut SizingRect, f32, i32, i32);

/// The eight helpers by address, each wrapped to take the ratio and both minimums.
const HELPERS: [(u64, &str, Helper); 8] = [
    (0x5910a0, "fit_top", |r, q, _, _| fit_top(r, q)),
    (0x5910e0, "fit_bottom", |r, q, _, _| fit_bottom(r, q)),
    (0x591120, "fit_left", |r, q, _, _| fit_left(r, q)),
    (0x591160, "fit_right", |r, q, _, _| fit_right(r, q)),
    (0x5911a0, "min_left", |r, _, w, _| min_left(r, w)),
    (0x5911c0, "min_right", |r, _, w, _| min_right(r, w)),
    (0x5911e0, "min_top", |r, _, _, h| min_top(r, h)),
    (0x591200, "min_bottom", |r, _, _, h| min_bottom(r, h)),
];

/// Ratios across every regime the fits distinguish: zero of both signs (no fit), the usual
/// aspects, tiny and huge values, the infinities and a NaN (which fits).
fn ratios() -> Vec<f32> {
    let mut v = vec![
        0.0,
        -0.0,
        4.0 / 3.0,
        16.0 / 9.0,
        1.25,
        1.6,
        1.0,
        0.75,
        -1.5,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        1e-30,
        1e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        (1024.0f64 / 768.0) as f32,
        (1920.0f64 / 1080.0) as f32,
        (1280.0f64 / 1024.0) as f32,
    ];
    let mut s = 0x5a17_e2c3u32;
    for _ in 0..40 {
        v.push((next(&mut s) >> 9) as f32 / 4_194_304.0 + 0.25);
    }
    v
}

/// Rectangles: sizes at, one under and one over 320 by 240 in either orientation, typical
/// windows, inverted and wrapping ones, and fuzz.
fn rects() -> Vec<SizingRect> {
    let mut v: Vec<SizingRect> = Vec::new();
    for &(w, h) in &[
        (320, 240),
        (319, 239),
        (321, 241),
        (320, 239),
        (319, 240),
        (1024, 768),
        (1920, 1080),
        (100, 2000),
        (2000, 100),
        (1, 1),
        (-5, 7),
        (7, -5),
    ] {
        for &(top, left) in &[(0, 0), (100, 200), (-50, -3000)] {
            v.push([top, left, top + h, left + w]);
        }
    }
    v.push([i32::MIN, i32::MIN, i32::MAX, i32::MAX]);
    v.push([i32::MAX, i32::MAX, i32::MIN, i32::MIN]);
    let mut s = 0x0bad_5eedu32;
    for _ in 0..400 {
        let top = (next(&mut s) >> 20) as i32 - 2048;
        let left = (next(&mut s) >> 20) as i32 - 2048;
        let h = (next(&mut s) >> 21) as i32 - 64;
        let w = (next(&mut s) >> 21) as i32 - 64;
        v.push([top, left, top.wrapping_add(h), left.wrapping_add(w)]);
    }
    v
}

const MINS: [(i32, i32); 3] = [(320, 240), (0, 0), (-1, 1000)];

fn write_rect(emu: &mut Emu, at: u64, r: &SizingRect) {
    for (i, &x) in r.iter().enumerate() {
        emu.write_u32(at + 4 * i as u64, x as u32);
    }
}

fn read_rect(emu: &Emu, at: u64) -> SizingRect {
    let mut r = [0i32; 4];
    for (i, x) in r.iter_mut().enumerate() {
        *x = emu.read_u32(at + 4 * i as u64) as i32;
    }
    r
}

fn set_globals(emu: &mut Emu, ratio: f32, min_w: i32, min_h: i32) {
    emu.write_u32(RATIO, ratio.to_bits());
    emu.write_u32(MIN_W, min_w as u32);
    emu.write_u32(MIN_H, min_h as u32);
}

/// `0x5910a0` to `0x591200`, each called whole on its `{top, left, bottom, right}` in `ecx`.
#[test]
fn sizing_rect_helpers_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let rect = emu.alloc(0x10);
    let (rs, qs) = (rects(), ratios());
    let mut checked = 0u64;
    for &(addr, name, ours_fn) in &HELPERS {
        for &(min_w, min_h) in &MINS {
            for &q in &qs {
                for r0 in &rs {
                    set_globals(&mut emu, q, min_w, min_h);
                    write_rect(&mut emu, rect, r0);
                    emu.call(addr, rect as u32, 0, &[]);
                    let binary = read_rect(&emu, rect);
                    let mut ours = *r0;
                    ours_fn(&mut ours, q, min_w, min_h);
                    assert_eq!(
                        binary, ours,
                        "{name} @ {addr:#x} r={r0:?} ratio={q:?} min=({min_w},{min_h})"
                    );
                    checked += 1;
                }
            }
        }
    }
    eprintln!("sizing helpers: {checked} cases bit-exact vs WoW.exe");
}

/// Runs one procedure's sizing block on `lParam = rect` and `wParam = edge`, with `ebp` a frame
/// whose `[ebp+0x10]` and `[ebp+0x14]` are the two message words.
fn run_block(emu: &mut Emu, frame: u64, rect: u64, edge: u32, begin: u64, until: u64) {
    let ebp = frame + 0x40;
    emu.write_u32(ebp + 0x10, edge);
    emu.write_u32(ebp + 0x14, rect as u32);
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, SLICE_ESP);
    emu.run(begin, until);
}

/// The `WM_SIZING` blocks of both procedures: `0x5989d7` to `0x598af1` (Direct3D, table
/// `0x598b30`) and `0x58c841` to `0x58c95d` (OpenGL, table `0x58ca34`), for every edge code 0 to 9.
#[test]
fn sizing_wm_sizing_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let frame = emu.alloc(0x80);
    let rect = emu.alloc(0x10);
    let qs = [0.0f32, 4.0 / 3.0, 16.0 / 9.0, 1.25, 0.6, f32::NAN];
    let mut checked = 0u64;
    for &(begin, until, name) in &[
        (0x5989d7u64, 0x598af1u64, "d3d"),
        (0x58c841, 0x58c95d, "opengl"),
    ] {
        for r0 in rects() {
            // The block returns early for a zero width or height, outside this slice.
            if r0[3] == r0[1] || r0[2] == r0[0] {
                continue;
            }
            for &q in &qs {
                for edge in 0..10u32 {
                    set_globals(&mut emu, q, 320, 240);
                    // The procedure reads a RECT: left, top, right, bottom.
                    let win = [r0[1], r0[0], r0[3], r0[2]];
                    for (i, &x) in win.iter().enumerate() {
                        emu.write_u32(rect + 4 * i as u64, x as u32);
                    }
                    run_block(&mut emu, frame, rect, edge, begin, until);
                    let out = read_rect(&emu, rect);
                    let binary = [out[1], out[0], out[3], out[2]];
                    let mut ours = r0;
                    sizing(&mut ours, edge, q, 320, 240);
                    assert_eq!(
                        binary, ours,
                        "{name} edge={edge} r={r0:?} ratio={q:?} (top, left, bottom, right)"
                    );
                    checked += 1;
                }
            }
        }
    }
    eprintln!("WM_SIZING blocks: {checked} cases bit-exact vs WoW.exe");
}
