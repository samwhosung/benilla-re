//! The Region size and edge methods and the game's screen-size globals, as the float sequences
//! they run between script units and layout units, for any screen aspect
//! (`systems/ui/methods.md#coordinates-the-methods-convert`, `systems/screencoord.md`).
//!
//! The conversion itself is [`ScaleState`]'s: a getter answers `RN24(1024·A·l) / K1`, a setter
//! stores `RN24(K1·RN24(RN24(v)/(1024·A)))`, on both axes. This module adds what each method reads
//! from the region's layout part and what it divides first.

use benilla_re_screencoord::{scale_xy, ScaleState};

/// Flag 1 of the layout part's `+0x3c`: the resolved rectangle is valid.
pub const RECT_VALID: u32 = 1;

/// What the methods read from a region's layout part (`CScriptRegion+0x24`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutPart {
    /// `+0x3c`, the flags; only [`RECT_VALID`] changes a result.
    pub flags: u32,
    /// `+0x40`, the resolved bottom, left, top and right.
    pub rect: [f32; 4],
    /// `+0x50`, the width slot 0x1c answers.
    pub width: f32,
    /// `+0x54`, the height slot 0x20 answers.
    pub height: f32,
    /// `+0x58` (the region's `+0x7c`), the layout scale.
    pub scale: f32,
}

impl LayoutPart {
    fn rect(&self) -> Option<[f32; 4]> {
        (self.flags & RECT_VALID != 0).then_some(self.rect)
    }
}

/// The stored size, or for a size equal to 0 the rectangle's span `RN24(far − near)` when the
/// rectangle is valid (`0x7a1e99` to `0x7a1eec`). A NaN size is kept, as `jp` takes it.
fn size_or_span(stored: f32, rect: Option<[f32; 4]>, near: usize, far: usize) -> f32 {
    if stored == 0.0 {
        if let Some(r) = rect {
            return (f64::from(r[far]) - f64::from(r[near])) as f32;
        }
    }
    stored
}

/// `Region:GetWidth()` (`0x7a1e00`): the span is not divided by the layout scale.
pub fn get_width(st: ScaleState, l: &LayoutPart) -> f64 {
    st.to_script(size_or_span(l.width, l.rect(), 1, 3))
}

/// `Region:GetHeight()` (`0x7a2030`).
pub fn get_height(st: ScaleState, l: &LayoutPart) -> f64 {
    st.to_script(size_or_span(l.height, l.rect(), 0, 2))
}

/// `Region:GetLeft()`, `GetRight()`, `GetTop()` and `GetBottom()` (`0x7a1980`, `0x7a1aa0`,
/// `0x7a1bc0`, `0x7a1ce0`) for the rectangle index 1, 3, 2 and 0: nil for an invalid rectangle.
pub fn get_edge(st: ScaleState, l: &LayoutPart, index: usize) -> Option<f64> {
    l.rect().map(|r| st.to_script_over(r[index], l.scale))
}

/// `Region:GetCenter()` (`0x7a1810`): x from left and right, then y from bottom and top.
pub fn get_center(st: ScaleState, l: &LayoutPart) -> Option<(f64, f64)> {
    l.rect().map(|r| {
        (
            st.to_script_mid(r[1], r[3], l.scale),
            st.to_script_mid(r[0], r[2], l.scale),
        )
    })
}

/// `Region:SetWidth(v)` and `SetHeight(v)` (`0x7a1f20`, `0x7a2150`): the float passed to the
/// layout part's slot 0x14 or 0x18. Both axes scale by `K1`.
pub fn set_size(st: ScaleState, v: f64) -> f32 {
    st.to_layout(v)
}

/// The game's `GetScreenWidth()` and `GetScreenHeight()` (`0x48b480`, `0x48b4d0`): the screen in
/// layout units, `K1` or `K2`, over the layout scale of `UIParent` (1.0 without it), rounded to a
/// float, then as a getter.
pub fn ui_screen_size(st: ScaleState, ui_parent_scale: Option<f32>, height: bool) -> f64 {
    let extent = if height { st.g48 } else { st.g44 }; // push 1.0; call 0x41ae70 or 0x41ae60; fstp
    let s = ui_parent_scale.unwrap_or(1.0); // 0x494590
    let u = (f64::from(extent) / f64::from(s)) as f32; // fdivr [ebp-4]; fstp
    st.to_script(u)
}

/// The game's `GetCursorPosition()` (`0x48b820`): the screen root's cursor dwords `fx` and `fy`
/// (`+0x1118`, `+0x111c`) through `ScaleScreenDelta`, `RN24(K1·fx)` and `RN24(K2·fy)`, then each
/// as a getter, both over `K1`.
pub fn cursor_position(st: ScaleState, fx: f32, fy: f32) -> (f64, f64) {
    let (x, y) = scale_xy(st.g44, st.g48, fx, fy);
    (st.to_script(x), st.to_script(y))
}
