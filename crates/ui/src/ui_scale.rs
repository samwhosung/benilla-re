//! The layout scale the scripts apply to lengths, and the widget methods that convert through it
//! (`systems/ui/uiscale.md#reading-the-scale`, `systems/ui/methods.md#coordinates-the-methods-convert`).
//!
//! A script length is in script units, 1024 across a 4:3 screen; the widgets keep layout lengths.
//! These functions hold the cells `0x41ad10` stores for 4:3, the file's values for the two used
//! here; [`crate::region_units`] takes any aspect. Three leaf helpers carry the factors: `0x41ad70`
//! pushes `A`, 1.0 (`0x832a4c`), `0x41ae40` divides its argument by `K1`, 0.8 (`0x832a44`), and
//! `0x41ae60` multiplies by it.
//! A getter multiplies the base by 1024 and the device length, narrows to `f32` as the argument of
//! `0x41ae40`, and pushes the quotient as an f64. A setter divides the script value by the base times
//! 1024, narrows to `f32` as the argument of `0x41ae60`, and stores the product as an `f32`.

/// The aspect base `0x832a4c` that `0x41ad70` pushes, 1.0.
pub const LOGICAL_BASE: f32 = f32::from_bits(0x3f80_0000);
/// The X scale `0x832a44` that `0x41ae40` divides by and `0x41ae60` multiplies by, 0.8.
pub const UI_SCALE: f32 = f32::from_bits(0x3f4c_cccd);
/// `0x7ffd68`, 1024.0: the factor every script conversion applies to `A`.
pub const LOGICAL_WIDTH: f32 = f32::from_bits(0x4480_0000);
/// `0x7ffa24`, 0.5: the midpoint factor of `GetCenter`.
const HALF: f32 = f32::from_bits(0x3f00_0000);

/// A length the widget keeps, as a script sees it, with no widget scale in the chain:
/// `(1.0 * 1024.0 * coord) as f32 / 0.8` pushed as an f64.
pub fn point_to_logical(coord: f32) -> f64 {
    let inner = (f64::from(LOGICAL_BASE) * f64::from(LOGICAL_WIDTH) * f64::from(coord)) as f32;
    f64::from(inner) / f64::from(UI_SCALE)
}

/// A script length as the widget keeps it: `((lua_arg / (1.0 * 1024.0)) as f32 * 0.8) as f32`.
pub fn logical_to_device(lua_arg: f32) -> f32 {
    let inner = (f64::from(lua_arg) / (f64::from(LOGICAL_BASE) * f64::from(LOGICAL_WIDTH))) as f32;
    (f64::from(inner) * f64::from(UI_SCALE)) as f32
}

/// A rect edge as a script sees it: the widget's scale `+0x7c` divides the product,
/// `(1.0 * 1024.0 * coord / scale) as f32 / 0.8`.
pub fn device_to_logical(coord: f32, scale: f32) -> f64 {
    let inner = (f64::from(LOGICAL_BASE) * f64::from(LOGICAL_WIDTH) * f64::from(coord)
        / f64::from(scale)) as f32;
    f64::from(inner) / f64::from(UI_SCALE)
}

/// The centre of `GetCenter`: the midpoint `(hi - lo) * 0.5 + lo` stays an f64 on the stack into the
/// scaling of [`device_to_logical`].
pub fn center_to_logical(hi: f64, lo: f64, scale: f32) -> f64 {
    let center = (hi - lo) * f64::from(HALF) + lo;
    let inner =
        (f64::from(LOGICAL_BASE) * f64::from(LOGICAL_WIDTH) * center / f64::from(scale)) as f32;
    f64::from(inner) / f64::from(UI_SCALE)
}

/// `Region::GetCenter` (`0x7a1810`, x block `0x7a18ed` to `0x7a1916`, y block `0x7a1928` to
/// `0x7a1951`): the x centre of the right and left edges, then the y centre of the top and bottom.
pub fn get_center(right: f32, left: f32, top: f32, bottom: f32, scale: f32) -> (f64, f64) {
    (
        center_to_logical(f64::from(right), f64::from(left), scale),
        center_to_logical(f64::from(top), f64::from(bottom), scale),
    )
}

/// `Region::GetLeft` (`0x7a1980`, slice `0x7a1a59` to `0x7a1a76`).
pub fn get_left(left: f32, scale: f32) -> f64 {
    device_to_logical(left, scale)
}

/// `Region::GetRight` (`0x7a1aa0`, `0x7a1b79` to `0x7a1b96`).
pub fn get_right(right: f32, scale: f32) -> f64 {
    device_to_logical(right, scale)
}

/// `Region::GetTop` (`0x7a1bc0`, `0x7a1c99` to `0x7a1cb6`).
pub fn get_top(top: f32, scale: f32) -> f64 {
    device_to_logical(top, scale)
}

/// `Region::GetBottom` (`0x7a1ce0`, `0x7a1db9` to `0x7a1dd6`).
pub fn get_bottom(bottom: f32, scale: f32) -> f64 {
    device_to_logical(bottom, scale)
}

/// `Region::GetWidth` (`0x7a1e00`, slice `0x7a1ee6` to `0x7a1f0c`): `right - left` narrows to `f32`
/// at `fstp [ebp-4]`, and a length takes no widget scale, so [`point_to_logical`] converts it.
pub fn get_width(right: f32, left: f32) -> f64 {
    let width = (f64::from(right) - f64::from(left)) as f32;
    point_to_logical(width)
}

/// `Region::GetHeight` (`0x7a2030`, `0x7a2116` to `0x7a213e`): `top - bottom`, then as the width.
pub fn get_height(top: f32, bottom: f32) -> f64 {
    let height = (f64::from(top) - f64::from(bottom)) as f32;
    point_to_logical(height)
}

/// `Region::GetPoint` (`0x7a2340`; `0x7a2483` and `0x7a24ac`, or `0x7a24e7` and `0x7a250b`): an
/// anchor offset is stored device-scaled, so both pushes take no widget scale.
pub fn get_point(x: f32, y: f32) -> (f64, f64) {
    (point_to_logical(x), point_to_logical(y))
}

/// `Region::SetWidth` (`0x7a1f20`, `0x7a1fcb` to `0x7a1fe9`).
pub fn set_width(lua_arg: f32) -> f32 {
    logical_to_device(lua_arg)
}

/// `Region::SetHeight` (`0x7a2150`, `0x7a21fb` to `0x7a2219`).
pub fn set_height(lua_arg: f32) -> f32 {
    logical_to_device(lua_arg)
}

/// `Region::SetPoint` (`0x7a2540`; `0x7a28a3` to `0x7a28bd` for x, `0x7a28c9` to `0x7a28e3` for y).
pub fn set_point(x_arg: f32, y_arg: f32) -> (f32, f32) {
    (logical_to_device(x_arg), logical_to_device(y_arg))
}

/// `Frame::GetHitRectInsets` (`0x775bf0`): the four insets in push order.
pub fn get_hit_rect_insets(s0: f32, s1: f32, s2: f32, s3: f32) -> (f64, f64, f64, f64) {
    (
        point_to_logical(s0),
        point_to_logical(s1),
        point_to_logical(s2),
        point_to_logical(s3),
    )
}

/// `Frame::SetHitRectInsets` (`0x775d50`): the four script arguments, each converted and stored as
/// an `f32`.
pub fn set_hit_rect_insets(a0: f32, a1: f32, a2: f32, a3: f32) -> (f32, f32, f32, f32) {
    (
        logical_to_device(a0),
        logical_to_device(a1),
        logical_to_device(a2),
        logical_to_device(a3),
    )
}

/// `Frame::GetMinResize` (`0x775f20`) and `Frame::GetMaxResize` (`0x7761a0`): two of the four
/// stored bounds, width then height.
pub fn get_resize(first: f32, second: f32) -> (f64, f64) {
    (point_to_logical(first), point_to_logical(second))
}

/// `Frame::SetMinResize` (`0x776020`) and `Frame::SetMaxResize` (`0x7762a0`): two script arguments.
pub fn set_resize(a0: f32, a1: f32) -> (f32, f32) {
    (logical_to_device(a0), logical_to_device(a1))
}

/// `Frame::GetBackdrop` (`0x777370`): the two tile sizes (`0x7774da`, `0x77751c`), then the four edge
/// insets (`0x7775da`, `0x777616`, `0x777652`, `0x77768e`), in push order.
pub fn get_backdrop(
    tile1: f32,
    tile2: f32,
    inset_a: f32,
    inset_b: f32,
    inset_c: f32,
    inset_d: f32,
) -> [f64; 6] {
    [tile1, tile2, inset_a, inset_b, inset_c, inset_d].map(point_to_logical)
}

/// `Frame::SetBackdrop` (`0x7776e0`): up to six numeric arguments, each converted and stored back
/// into the backdrop (`0x77792e`, `0x777987`, `0x777a27`, `0x777a80`, `0x777ad9`, `0x777b32`).
pub fn set_backdrop(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> [f32; 6] {
    [a, b, c, d, e, f].map(logical_to_device)
}

/// `Button::SetPushedTextOffset` (`0x782030`; `0x7820eb` for x, `0x782114` for y).
pub fn set_pushed_text_offset(x_arg: f32, y_arg: f32) -> (f32, f32) {
    (logical_to_device(x_arg), logical_to_device(y_arg))
}

/// `Button::GetPushedTextOffset` (`0x782180`; `0x782237` for x, `0x78225b` for y).
pub fn get_pushed_text_offset(x: f32, y: f32) -> (f64, f64) {
    (point_to_logical(x), point_to_logical(y))
}

/// `Button::GetTextWidth` (`0x782290`): the font string's width, narrowed to `f32` by the call that
/// measures it.
pub fn get_text_width(width: f32) -> f64 {
    point_to_logical(width)
}

/// `Button::GetTextHeight` (`0x782390`): as the width, from the vtable slot `+0x20`.
pub fn get_text_height(height: f32) -> f64 {
    point_to_logical(height)
}

/// `ScrollFrame::SetHorizontalScroll` (`0x7912f0`, `0x791399` to `0x7913b3`).
pub fn set_horizontal_scroll(lua_arg: f32) -> f32 {
    logical_to_device(lua_arg)
}

/// `ScrollFrame::SetVerticalScroll` (`0x791400`, `0x7914a9` to `0x7914c3`).
pub fn set_vertical_scroll(lua_arg: f32) -> f32 {
    logical_to_device(lua_arg)
}

/// `ScrollFrame::GetHorizontalScroll` (`0x791530`, `0x7915dc` to `0x7915f9`): the field `+0x324`.
pub fn get_horizontal_scroll(field: f32) -> f64 {
    point_to_logical(field)
}

/// `ScrollFrame::GetVerticalScroll` (`0x791610`, `0x7916bc` to `0x7916d9`): the field `+0x328`.
pub fn get_vertical_scroll(field: f32) -> f64 {
    point_to_logical(field)
}

/// `ScrollFrame::GetHorizontalScrollRange` (`0x7916f0`, `0x79179c` to `0x7917b9`): the field `+0x31c`.
pub fn get_horizontal_scroll_range(field: f32) -> f64 {
    point_to_logical(field)
}

/// `ScrollFrame::GetVerticalScrollRange` (`0x7917d0`, `0x79187c` to `0x791899`): the field `+0x320`.
pub fn get_vertical_scroll_range(field: f32) -> f64 {
    point_to_logical(field)
}

/// One `EditBox::SetTextInsets` inset (`0x798a60`; `fmul` at `0x798b4e`, `0x798b78`, `0x798ba2`,
/// `0x798bcc`, each with its `fdivr`): `inset / (k_scale * 1024.0)` narrowed to `f32`, with `k_scale`
/// the aspect base that `0x41ad70` leaves on the stack and `inset` the argument already narrowed to
/// `f32`. The `fdivr` divides the memory operand by `st0`.
pub fn settextinsets_block(k_scale: f32, inset: f32) -> f32 {
    let denom = f64::from(k_scale) * f64::from(LOGICAL_WIDTH);
    (f64::from(inset) / denom) as f32
}

/// One `EditBox::GetTextInsets` inset (`0x798c30`; `fmul` at `0x798cef`, `0x798d13`, `0x798d37`,
/// `0x798d5b`): `(k_scale * 1024.0) * inset` narrowed to `f32`, the inverse of
/// [`settextinsets_block`].
pub fn gettextinsets_block(k_scale: f32, inset: f32) -> f32 {
    let scaled = f64::from(k_scale) * f64::from(LOGICAL_WIDTH);
    (scaled * f64::from(inset)) as f32
}

/// `GameTooltip::SetPadding` (`0x530f10`): the script padding converted for the padding commit
/// `0x530630`.
pub fn set_padding(lua_arg: f32) -> f32 {
    logical_to_device(lua_arg)
}

/// `GameTooltip::SetOwner` (`0x5310d0`): the x and y anchor offsets (arguments 4 and 5) converted
/// for the anchor commit `0x52ffe0`.
pub fn set_owner(x_arg: f32, y_arg: f32) -> (f32, f32) {
    (logical_to_device(x_arg), logical_to_device(y_arg))
}

/// The static initializer `0x7cca80`: `fld [0xcf6164]; fmul [0x7ff9cc]; fstp [0xcf61a0]`, the global
/// `+0xcf6164` times the `f32` -255.0, stored as an `f32`.
pub fn static_scale(input: f32) -> f32 {
    (f64::from(input) * f64::from(f32::from_bits(0xc37f_0000))) as f32
}
