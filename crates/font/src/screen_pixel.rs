//! The screen↔pixel scaling family — the converters between a screen-space fraction (the layout's
//! device-independent unit) and integer raster pixels, plus the font's natural 1:1 height. All read
//! the viewport pixel dims (`0xc2b9a0` = height, `0xc2b9a4` = width), here passed as inputs.

use crate::ftol;

/// `ScreenToPixelHeight` (0x5c6fa0): convert a screen-height fraction `v` to integer pixels against the
/// viewport HEIGHT, round-half-away-from-zero; passthrough `v` when `already_pixels` (the caller's 0x80
/// "raw px" flag). Bytes: `fild qword[screenH]; fmul v; fcom 0.0; t>0 ? +0.5 : -0.5; __ftol; (i32)→float`.
pub fn screen_to_pixel_height(already_pixels: bool, v: f32, screen_h: i32) -> f32 {
    if already_pixels {
        return v;
    }
    let t = f64::from(screen_h) * f64::from(v); // fild qword[screenH]; fmul dword[v]
    let r = if t > 0.0 { t + 0.5 } else { t - 0.5 }; // fcom 0.0; fadd/fsub [0x7ffa24]=0.5
    ftol(r) as f32 // __ftol; fild (i32)→st0
}

/// `ScreenToPixelWidth` (0x5c7010): identical to [`screen_to_pixel_height`] but against the viewport
/// WIDTH (`0xc2b9a4`).
pub fn screen_to_pixel_width(already_pixels: bool, v: f32, screen_w: i32) -> f32 {
    if already_pixels {
        return v;
    }
    let t = f64::from(screen_w) * f64::from(v);
    let r = if t > 0.0 { t + 0.5 } else { t - 0.5 };
    ftol(r) as f32
}

/// pixel→screen height (0x5c7080): inverse — `pixels / viewportHeight` as a screen fraction. Bytes:
/// `fild dword[pixels]; fidiv dword[screenH]` (integer divide), returned in `st0`.
pub fn pixel_to_screen_height(pixels: i32, screen_h: i32) -> f32 {
    (f64::from(pixels) / f64::from(screen_h)) as f32
}

/// `GxuFontGetOneToOneHeight` (0x5c2540): the font's natural 1:1 pixel height as a screen-height
/// fraction = `pixelSize / viewportHeight`, where `pixelSize` = `font[0x24c]` (the rendered em pixel
/// size). Bytes: `fild qword[pixelSize]; fidiv dword[screenH]`. The binary returns 0.0 for a null font
/// (caller-checked); here `pixel_size` is the already-fetched `font[0x24c]`.
pub fn one_to_one_height(pixel_size: i32, screen_h: i32) -> f32 {
    (f64::from(pixel_size) / f64::from(screen_h)) as f32
}
