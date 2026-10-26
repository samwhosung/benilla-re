//! The window procedures' drag rectangle (`systems/gx/window.md`, Window sizing): the eight
//! helpers `0x5910a0` to `0x591200` that `WM_SIZING` runs on a `{top, left, bottom, right}`
//! rectangle of `i32`, and the sequence each dragged edge runs.
//!
//! The helpers read the aspect ratio `[0xc0ef60]` (an `f32`, 0 unless the format constrains the
//! aspect) and the minimum size `[0xc0ef64]`, `[0xc0ef68]`. Differences are 32-bit and wrap; the
//! minimum tests compare them signed.

use crate::device::{aspect_fit_max_x, aspect_fit_max_y, aspect_fit_min_x, aspect_fit_min_y};

/// A drag rectangle as the procedures hold it: `[top, left, bottom, right]`.
pub type SizingRect = [i32; 4];

const TOP: usize = 0;
const LEFT: usize = 1;
const BOTTOM: usize = 2;
const RIGHT: usize = 3;

/// `0x5910a0`: `top = bottom − ftol(width / ratio)`, unless the ratio is 0 ([`aspect_fit_min_x`]).
pub fn fit_top(r: &mut SizingRect, ratio: f32) {
    *r = aspect_fit_min_x(*r, ratio);
}

/// `0x5910e0`: `bottom = top + ftol(width / ratio)` ([`aspect_fit_max_x`]).
pub fn fit_bottom(r: &mut SizingRect, ratio: f32) {
    *r = aspect_fit_max_x(*r, ratio);
}

/// `0x591120`: `left = right − ftol(height · ratio)` ([`aspect_fit_min_y`]).
pub fn fit_left(r: &mut SizingRect, ratio: f32) {
    *r = aspect_fit_min_y(*r, ratio);
}

/// `0x591160`: `right = left + ftol(height · ratio)` ([`aspect_fit_max_y`]).
pub fn fit_right(r: &mut SizingRect, ratio: f32) {
    *r = aspect_fit_max_y(*r, ratio);
}

/// `0x5911a0`: a width below `min_w` moves the left edge to `right − min_w − 1`.
pub fn min_left(r: &mut SizingRect, min_w: i32) {
    if r[RIGHT].wrapping_sub(r[LEFT]) < min_w {
        r[LEFT] = r[RIGHT].wrapping_sub(min_w).wrapping_sub(1);
    }
}

/// `0x5911c0`: a width below `min_w` moves the right edge to `left + min_w − 1`.
pub fn min_right(r: &mut SizingRect, min_w: i32) {
    if r[RIGHT].wrapping_sub(r[LEFT]) < min_w {
        r[RIGHT] = r[LEFT].wrapping_add(min_w).wrapping_sub(1);
    }
}

/// `0x5911e0`: a height below `min_h` moves the top edge to `bottom − min_h − 1`.
pub fn min_top(r: &mut SizingRect, min_h: i32) {
    if r[BOTTOM].wrapping_sub(r[TOP]) < min_h {
        r[TOP] = r[BOTTOM].wrapping_sub(min_h).wrapping_sub(1);
    }
}

/// `0x591200`: a height below `min_h` moves the bottom edge to `top + min_h − 1`.
pub fn min_bottom(r: &mut SizingRect, min_h: i32) {
    if r[BOTTOM].wrapping_sub(r[TOP]) < min_h {
        r[BOTTOM] = r[TOP].wrapping_add(min_h).wrapping_sub(1);
    }
}

/// What `WM_SIZING` does to the rectangle for the dragged edge `edge` (`wParam`, 1 to 8 the
/// `WMSZ_*` codes): the helpers in the order both case tables (`0x598b30`, `0x58ca34`) call them.
/// Another edge leaves the rectangle unchanged. The zero-width and zero-height refusals of the
/// procedure come before this and are not part of it.
pub fn sizing(r: &mut SizingRect, edge: u32, ratio: f32, min_w: i32, min_h: i32) {
    match edge {
        1 => {
            min_left(r, min_w);
            fit_top(r, ratio);
        }
        2 => {
            min_right(r, min_w);
            fit_bottom(r, ratio);
        }
        3 => {
            min_top(r, min_h);
            fit_left(r, ratio);
        }
        4 => {
            min_top(r, min_h);
            min_left(r, min_w);
            fit_top(r, ratio);
            fit_left(r, ratio);
        }
        5 => {
            min_top(r, min_h);
            min_right(r, min_w);
            fit_top(r, ratio);
            fit_right(r, ratio);
        }
        6 => {
            min_bottom(r, min_h);
            fit_right(r, ratio);
        }
        7 => {
            min_bottom(r, min_h);
            min_left(r, min_w);
            fit_left(r, ratio);
            fit_bottom(r, ratio);
        }
        8 => {
            min_bottom(r, min_h);
            min_right(r, min_w);
            fit_bottom(r, ratio);
            fit_right(r, ratio);
        }
        _ => {}
    }
}
