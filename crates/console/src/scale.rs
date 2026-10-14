//! The scale factors `ConsoleScreenInitialize` (`0x63b460`, slice `0x63b48a` to `0x63b4e6`) derives
//! from the viewport rect it fetches with `0x58a240`: `scaleX` (`0xc4eac0`) from the rect height and
//! `scaleY` (`0xc4ea8c`) from the rect width, each `1.0 / range` unless the range is exactly 0, then
//! 1.0. `scaleX` is the caret's width unit in `draw_caret`.

/// `scale = (a − b == 0) ? 1.0 : 1.0 / (a − b)`. The subtraction stays in the x87 register (f64,
/// unrounded) before the `fcom 0`/`fdiv`, so the divide is `1.0 / (f64(a) − f64(b))`, rounded to f32
/// at the store. Used as `scaleX = console_scale(rect.bottom, rect.top)` and
/// `scaleY = console_scale(rect.right, rect.left)`.
pub fn console_scale(a: f32, b: f32) -> f32 {
    let range = f64::from(a) - f64::from(b);
    if range == 0.0 {
        1.0
    } else {
        (1.0_f64 / range) as f32
    }
}
