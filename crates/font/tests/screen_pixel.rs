//! Bit-exact tests of the screen-to-pixel family against `WoW.exe`. These are register and
//! stack-argument functions read whole through `call_f32`; the viewport dims are set as globals
//! (0xc2b9a0 height, 0xc2b9a4 width).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_font::screen_pixel::{
    one_to_one_height, pixel_to_screen_height, screen_to_pixel_height, screen_to_pixel_width,
};
use common::{assert_f32, fuzz_unit, g, next};

const SCREEN_TO_PIXEL_HEIGHT: u64 = 0x5c6fa0;
const SCREEN_TO_PIXEL_WIDTH: u64 = 0x5c7010;
const PIXEL_TO_SCREEN_HEIGHT: u64 = 0x5c7080;
const ONE_TO_ONE_HEIGHT: u64 = 0x5c2540;

const RES: &[i32] = &[480, 600, 768, 1024, 1200];

/// Sizes whose product with the screen size is NaN, infinite, past `i32` or past `i64`: `__ftol`
/// returns the low dword of its qword, 0 for the indefinite.
const WIDE: [f32; 9] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    1.0e30,
    -1.0e30,
    1.0e7,
    -1.0e7,
    4.0e6,
    1.0e17,
];

#[test]
fn screen_to_pixel_height_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s: u32 = 0x5c6f_0001;
    for &h in RES {
        e.write_u32(g::SCREEN_H, h as u32);
        // explicit ±0.5 ties (round-half-away-from-zero): t = ±(N+0.5)
        for &(v, _tag) in &[
            (1.5f32 / h as f32, "tie+1.5"),
            (-1.5f32 / h as f32, "tie-1.5"),
            (2.5f32 / h as f32, "tie+2.5"),
            (0.0f32, "zero"),
        ] {
            let bin = e.call_f32(SCREEN_TO_PIXEL_HEIGHT, 0, 0, &[v.to_bits()]);
            assert_f32(
                bin,
                screen_to_pixel_height(false, v, h),
                &format!("s2ph tie h={h} v={v}"),
            );
        }
        for &v in &WIDE {
            let bin = e.call_f32(SCREEN_TO_PIXEL_HEIGHT, 0, 0, &[v.to_bits()]);
            assert_f32(
                bin,
                screen_to_pixel_height(false, v, h),
                &format!("s2ph wide h={h} v={v}"),
            );
        }
        for i in 0..2000u32 {
            let v = fuzz_unit(&mut s, 4.0) - 2.0; // [-2, 2)
            let bin = e.call_f32(SCREEN_TO_PIXEL_HEIGHT, 0, 0, &[v.to_bits()]);
            assert_f32(
                bin,
                screen_to_pixel_height(false, v, h),
                &format!("s2ph[{i}] h={h} v={v}"),
            );
            // passthrough (already_pixels != 0)
            let binp = e.call_f32(SCREEN_TO_PIXEL_HEIGHT, 1, 0, &[v.to_bits()]);
            assert_f32(
                binp,
                screen_to_pixel_height(true, v, h),
                &format!("s2ph-pass[{i}] v={v}"),
            );
        }
    }
}

#[test]
fn screen_to_pixel_width_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s: u32 = 0x5c70_0001;
    for &w in RES {
        e.write_u32(g::SCREEN_W, w as u32);
        for &v in &WIDE {
            let bin = e.call_f32(SCREEN_TO_PIXEL_WIDTH, 0, 0, &[v.to_bits()]);
            assert_f32(
                bin,
                screen_to_pixel_width(false, v, w),
                &format!("s2pw wide w={w} v={v}"),
            );
        }
        for i in 0..2000u32 {
            let v = fuzz_unit(&mut s, 4.0) - 2.0;
            let bin = e.call_f32(SCREEN_TO_PIXEL_WIDTH, 0, 0, &[v.to_bits()]);
            assert_f32(
                bin,
                screen_to_pixel_width(false, v, w),
                &format!("s2pw[{i}] w={w} v={v}"),
            );
            let binp = e.call_f32(SCREEN_TO_PIXEL_WIDTH, 1, 0, &[v.to_bits()]);
            assert_f32(
                binp,
                screen_to_pixel_width(true, v, w),
                &format!("s2pw-pass[{i}] v={v}"),
            );
        }
    }
}

#[test]
fn pixel_to_screen_height_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mut s: u32 = 0x5c70_8001;
    for &h in RES {
        e.write_u32(g::SCREEN_H, h as u32);
        for i in 0..3000u32 {
            let pixels = (next(&mut s) >> 8) as i32 % 2400 - 200; // [-200, 2200)
            let bin = e.call_f32(PIXEL_TO_SCREEN_HEIGHT, pixels as u32, 0, &[]);
            assert_f32(
                bin,
                pixel_to_screen_height(pixels, h),
                &format!("p2sh[{i}] h={h} px={pixels}"),
            );
        }
    }
}

#[test]
fn one_to_one_height_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let font = e.alloc(0x250);
    let mut s: u32 = 0x5c25_4001;
    for &h in RES {
        e.write_u32(g::SCREEN_H, h as u32);
        for i in 0..1000u32 {
            let px = (next(&mut s) >> 8) as i32 % 256; // rendered pixel size [0, 256)
            e.write_u32(font + 0x24c, px as u32);
            let bin = e.call_f32(ONE_TO_ONE_HEIGHT, font as u32, 0, &[]);
            assert_f32(
                bin,
                one_to_one_height(px, h),
                &format!("o2oh[{i}] h={h} px={px}"),
            );
        }
    }
}
