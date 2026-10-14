//! The viewport pixel-dimension recompute (0x5c2b50) and the per-font rasterization-metric
//! derivation (0x5ca030). Each `pub fn` cites its source address.
//!
//! The x87 runs at 53-bit precision, so every `f*` operation rounds to `f64` and an `fstp m32` store
//! rounds to `f32`. Each operation is transcribed as `f64` arithmetic with an `as f32` at exactly the
//! exe's f32 stores; integer loads (`fild`) are `i32 as f64`; the CRT `__ftol` (truncate toward zero,
//! `0x40a2b0`) is [`crate::ftol`].

use crate::ftol;
use crate::screen_pixel::screen_to_pixel_height;

/// The default device viewport rect the binary substitutes when the device query yields a zero
/// extent: `{0, 0, 480.0, 640.0}` ⇒ 480×640 (`0x43f00000`=480.0 @ 5c2bfd, `0x44200000`=640.0 @
/// 5c2c07).
const DEFAULT_RECT: [f32; 4] = [0.0, 0.0, 480.0, 640.0];

/// `recompute screen pixel dims` (0x5c2b50): derive the viewport pixel HEIGHT (`0xc2b9a0`) and
/// WIDTH (`0xc2b9a4`) from the device viewport `rect` (4 f32, filled by the device query
/// `0x58a240` through its out-pointer). If **either** extent is zero the default `{0,0,480,640}`
/// is substituted (the device 0.0-sentinel guard, 5c2bc3–5c2be7). Bytes (recompute, 5c2c5e–5c2ca5):
/// `screenW = __ftol(rect[3] - rect[1])` then `screenH = __ftol(rect[2] - rect[0])` — the two
/// x87-subtracted extents truncated toward zero. Both subtractions are `f32`→x87 loads, exact at
/// `PC_53`. (The cache/change-detection and the live-font relayout walk are orchestration around this
/// arithmetic.) Returns `(screen_h, screen_w)`, matching the `(0xc2b9a0, 0xc2b9a4)` global order.
pub fn viewport_pixel_dims(rect: [f32; 4]) -> (i32, i32) {
    // device 0.0-sentinel: a zero in EITHER extent ⇒ substitute the default rect (5c2bc3-5c2be7).
    let ext_w = f64::from(rect[3]) - f64::from(rect[1]); // fld[ebp-4]; fsub[ebp-0xc]
    let ext_h = f64::from(rect[2]) - f64::from(rect[0]); // fld[ebp-8]; fsub[ebp-0x10]
    let r = if ext_w == 0.0 || ext_h == 0.0 {
        DEFAULT_RECT
    } else {
        rect
    };
    let screen_w = ftol(f64::from(r[3]) - f64::from(r[1])); // 5c2c61-5c2c92 → [0xc2b9a4]
    let screen_h = ftol(f64::from(r[2]) - f64::from(r[0])); // 5c2c8c-5c2ca5 → [0xc2b9a0]
    (screen_h, screen_w)
}

/// The four CGxFont fields [`font_raster_metrics`] derives, in `WoW.exe` write order.
#[derive(Clone, Copy, Debug)]
pub struct RasterMetrics {
    /// `font+0x24c` rendered pixel size = `min(32, __ftol(ScreenToPixelHeight(s)))`.
    pub pixel_size: i32,
    /// `font+0x17c` rasterizer load param = round-half-up ascender ratio scaled to render height.
    pub load_param: i32,
    /// `font+0x178` atlas cell = render height + the outline/shadow pad.
    pub atlas_cell: i32,
    /// `font+0x188` bold/outline advance-adjust ratio (f32).
    pub advance_adjust: f32,
}

/// `CGxFont::ComputeRasterMetrics` (0x5ca030): from the requested size + the face's vertical metrics,
/// derive the rendered pixel size, the rasterizer load param, the atlas cell size, and the
/// advance-adjust ratio. `__fastcall(this/ecx)`; runs at the tail of Init (0x5ca1d0). The floating-point
/// (objdump 5ca053–5ca151):
/// - **`m`** is the sub-call `0x5caea0(face, reqSize)` (= `max(reqSize, faceMinMetric/screenH)`,
///   its own function [`crate::metric::min_metric_screen_height`]); passed here as the already-fetched
///   `st0` so this fn owns only the arithmetic downstream of it.
/// - **`s`** = `max(m, 2.0/screenH)` stored as f32 (`fdivr`+`fcom`+keep-larger, 5ca063-5ca086) — a
///   ≥2-device-pixel floor.
/// - **`pixel_size`** (font+0x24c) = `min(32, __ftol(ScreenToPixelHeight(0, s)))` — the **unsigned**
///   `cmp 0x20; jbe` clamp over the rounded pixel height (5ca086-5ca0a0).
/// - **`load_param`** (font+0x17c) = `__ftol( renderH·(asc / (|desc| + asc)) + 0.5 )` — round-half-up
///   (`+0.5` then `__ftol`), `asc`=`(s16)face[0x46]`, `desc`=`(s16)face[0x48]`, `renderH`=pixel_size
///   (5ca0a6-5ca0ef). The denominator `|desc|+asc` is narrowed to f32 (`fstp m32`).
/// - **`atlas_cell`** (font+0x178) = `renderH` then **+4 if flags&8 (outline)** else **+2 if flags&1
///   (shadow)** — outline takes priority (5ca0fa-5ca118).
/// - **`advance_adjust`** (font+0x188, f32) = `(f32)( (u16)*(face[0x58]+0xc) / (u16)face[0x44] )`
///   (`fild`/`fidiv`/`fstp m32`, 5ca130-5ca151).
///
/// (The function also returns `0x7ce760(...) == 0` — the rasterizer-load BOUNDARY; that is not
/// arithmetic and is not modelled here.)
#[allow(clippy::too_many_arguments)]
pub fn font_raster_metrics(
    m: f64,
    screen_h: i32,
    flags: u32,
    ascender: i16,
    descender: i16,
    adv_num: u16,
    adv_den: u16,
) -> RasterMetrics {
    // s = max(m, 2.0/screenH), stored as f32 (5ca053-5ca086).
    let floor = (2.0_f64 / f64::from(screen_h)) as f32; // fild screenH; fdivr [0x801628]=2.0; fstp m32
    let s: f32 = if m > f64::from(floor) {
        m as f32 // fstp dword[ebp-4] of the float10 m
    } else {
        floor // keep the ≥2px floor
    };
    // pixel_size = min(32, __ftol(ScreenToPixelHeight(0, s))) — UNSIGNED clamp (5ca086-5ca0a0).
    let px = ftol(f64::from(screen_to_pixel_height(false, s, screen_h))); // 0x5c6fa0 then 0x40a2b0
    let pixel_size = if (px as u32) > 0x20 { 0x20 } else { px }; // cmp eax,0x20; jbe
    let render_h = pixel_size; // 0x5cae90 = font[0x24c] (re-read, == pixel_size)

    // load_param = __ftol( renderH·(asc/(|desc|+asc)) + 0.5 ) — round-half-up (5ca0a6-5ca0ef).
    let asc = f64::from(ascender); // movsx word[face+0x46]
    let desc = f64::from(descender); // movsx word[face+0x48]
    let denom = (desc.abs() + asc) as f32; // fild desc; fabs; fiadd asc; fstp m32
    let load_param = ftol(f64::from(render_h) * (asc / f64::from(denom)) + 0.5);
    // atlas_cell = renderH + (outline ? +4 : shadow ? +2 : 0) — outline (0x8) wins (5ca0fa-5ca118).
    let atlas_cell = render_h
        + if flags & 0x8 != 0 {
            4
        } else if flags & 0x1 != 0 {
            2
        } else {
            0
        };
    // advance_adjust = (f32)( faceSub.u16[0xc] / face.u16[0x44] ) (5ca130-5ca151).
    let advance_adjust = (f64::from(adv_num) / f64::from(adv_den)) as f32;
    RasterMetrics {
        pixel_size,
        load_param,
        atlas_cell,
        advance_adjust,
    }
}
