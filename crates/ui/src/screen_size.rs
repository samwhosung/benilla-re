//! The screen size scripts (`GetScreenWidth` `0x48b480`, `GetScreenHeight` `0x48b4d0`,
//! `GetCursorPosition` `0x48b820`) and the video mode list's aspect filter (`0x48bc90`).
//!
//! The scripts report device lengths in layout units, 1024 wide, through the leaf helpers
//! `0x41ad70` (the aspect base 1.0 at `0x832a4c`), `0x41ae40` (divides by the X scale 0.8 at
//! `0x832a44`), `0x41ae60` (multiplies by it) and `0x41ae70` (multiplies by the Y scale 0.6 at
//! `0x832a48`). `systems/ui/uiscale.md#reading-the-scale` has the entry. The cursor position takes
//! the length conversion of [`crate::ui_scale::point_to_logical`].

use crate::ui_scale::{LOGICAL_BASE, LOGICAL_WIDTH, UI_SCALE as UI_SCALE_X};

/// The Y scale `0x832a48`, 0.6.
pub const UI_SCALE_Y: f32 = f32::from_bits(0x3f19_999a);
/// `0x804570`, 1.248 as an `f32`: the least aspect ratio the mode list keeps.
const ASPECT_THRESHOLD: f32 = f32::from_bits(0x3f9f_be77);

/// The screen getter (`0x48b485` to `0x48b4be` for the width, `0x48b4d5` to `0x48b50e` for the
/// height). `scale_axis` is the axis scale the leading leaf multiplies by 1.0 (0.8 for the width, 0.6
/// for the height); `s` is the device scale of `0x494590` (the UI parent's `+0x7c`, or 1.0 when it
/// has none). The three `fstp dword` stores narrow `scale_axis * 1.0`, its quotient by `s`, and
/// `1.0 * 1024.0 * quotient` to `f32`; the closing division by 0.8 stays at 53 bits and is pushed as
/// an f64.
pub fn screen_to_logical(scale_axis: f64, s: f32) -> f64 {
    let scaled = (scale_axis * f64::from(LOGICAL_BASE)) as f32;
    let ratio = (f64::from(scaled) / f64::from(s)) as f32;
    let inner = (f64::from(LOGICAL_BASE) * f64::from(LOGICAL_WIDTH) * f64::from(ratio)) as f32;
    f64::from(inner) / f64::from(UI_SCALE_X)
}

/// `GetScreenWidth`: the axis scale is the X scale 0.8.
pub fn get_screen_width(s: f32) -> f64 {
    screen_to_logical(f64::from(UI_SCALE_X), s)
}

/// `GetScreenHeight`: the leading leaf is `0x41ae70`, so the axis scale is the Y scale 0.6; the
/// closing division is still by the X scale.
pub fn get_screen_height(s: f32) -> f64 {
    screen_to_logical(f64::from(UI_SCALE_Y), s)
}

/// The ratio of the mode list filter (`0x48bcfa` to `0x48bd03`): `fild` of the width divided by
/// `fidiv` of the height, at 53 bits.
pub fn aspect_ratio(width: i32, height: i32) -> f64 {
    f64::from(width) / f64::from(height)
}

/// Whether the filter keeps a mode (`0x48bd03` to `0x48bd0b`): `fcomp` against 1.248 sets C0 when the
/// ratio is smaller and `test ah,1; jne` skips the mode, so a mode is kept when the ratio is at
/// least 1.248, equality included.
pub fn aspect_ratio_kept(width: i32, height: i32) -> bool {
    aspect_ratio(width, height) >= f64::from(ASPECT_THRESHOLD)
}
