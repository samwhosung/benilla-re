//! The two procedural shadow ramps `FillShadowRamp` (`0x6d81a0`) and `FillShadowGreyRamp`
//! (`0x6d82d0`) (`systems/lighting/shadows.md#shadow-textures`). Each fills the ramp buffer
//! `0xce9e0c` with one pixel per column, from the column's position `x = 12·(i/(n − 1))` taken at
//! 53 bits.

use crate::x87::ftol;

/// The darkness at `x`: `x·0.5` below 2, 1 from 2 to below 10, otherwise `(12 − x)·0.5` kept only
/// where above 0 (`fcom` and `test ah,0x41`, so 0 for a negative, zero or NaN). Each threshold
/// compare is `test ah,5; jp`, which falls through only for `x` below it, so NaN goes on to the
/// next test.
fn darkness(x: f64) -> f64 {
    if x < 2.0 {
        x * 0.5
    } else if x < 10.0 {
        1.0
    } else {
        let v = (12.0 - x) * 0.5;
        if v > 0.0 {
            v
        } else {
            0.0
        }
    }
}

/// `0x6d81a0` (mode 1): the pixel of column `i` of `n`, white with the alpha `__ftol(darkness·255)`
/// (`[0x7ffe58]` is 255), `(alpha << 24) | 0x00ffffff`. `i/(n − 1)` is `fild` of two 64-bit
/// integers, so `n = 1` is 0/0 = NaN and gives alpha 0.
pub fn shadow_ramp_pixel(i: u32, n: u32) -> u32 {
    let ratio = f64::from(i) / f64::from(n.wrapping_sub(1));
    let alpha = darkness(ratio * 12.0);
    let byte = ftol(alpha * 255.0) as u8;
    (u32::from(byte) << 24) | 0x00ff_ffff
}

/// `0x6d82d0` (mode 1): the grey pixel of column `i` of `n`. The grey is byte 1 of the `f32` store
/// of `(1 − darkness)·255 + 512` (`bits >> 14`, low byte); the alpha is 0 for a grey of 0xff and
/// 0xff otherwise; the pixel is `(alpha << 24) | (grey << 16) | (grey << 8) | grey`.
pub fn shadow_ramp_gray_pixel(i: u32, n: u32) -> u32 {
    let ratio = f64::from(i) / f64::from(n.wrapping_sub(1));
    let alpha = darkness(ratio * 12.0);
    let m = (((1.0 - alpha) * 255.0) + 512.0) as f32;
    let gray = ((m.to_bits() >> 14) & 0xff) as u8;
    let aflag: u8 = if gray == 0xff { 0 } else { 0xff };
    (u32::from(aflag) << 24) | (u32::from(gray) << 16) | (u32::from(gray) << 8) | u32::from(gray)
}
