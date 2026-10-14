//! Caret geometry + the per-frame caret-blink and baseline maths (`paint_text_layout` 0x63b800,
//! `draw_caret` 0x63b990). The gx glyph submit + the line walk are delegated; what is here is the
//! caret-blink state machine, the line baseline, and the caret quad corners.

const BLINK_ON: f32 = f32::from_bits(0x3e99_999a); // [0x7ffd78] = 0.3 s (period while the caret is on)
const BLINK_OFF: f32 = f32::from_bits(0x3e4c_cccd); // [0x80679c] = 0.2 s (period while off)
const BASELINE_FACTOR: f32 = f32::from_bits(0x3f40_0000); // [0x8012cc] = 0.75 (first-line baseline)

/// `paint_text_layout`'s caret-blink state machine (0x63b800, slice 0x63b806..0x63b868). Each frame
/// the blink accumulator advances by the frame `dt`; when it reaches the on/off period (0.3 s on,
/// 0.2 s off) the caret phase toggles and the accumulator resets. The accumulator+dt is stored as
/// f32 but the **unrounded** f64 sum drives the period compare. Runs only while the console is
/// visible (`textTop < 1.0`). Returns the new `(accum, phase)`.
pub fn caret_blink(accum: f32, phase: u32, dt: f32) -> (f32, u32) {
    let raw = f64::from(accum) + f64::from(dt); // fld accum; fadd dt (fst stores f32, compare uses raw)
    let period = if phase != 0 {
        f64::from(BLINK_ON)
    } else {
        f64::from(BLINK_OFF)
    };
    if raw >= period {
        (0.0, u32::from(phase == 0)) // toggle: accum=0, phase=!phase
    } else {
        (raw as f32, phase)
    }
}

/// `paint_text_layout`'s input-line baseline Y (0x63b800, slice 0x63b87e..0x63b89c):
/// `baseline = fontSize · 0.75 + textTop`. Each subsequent visible line advances by `+ fontSize`
/// (the same `fadd`), walking up the screen while `baseline < 1.0`.
pub fn paint_baseline(font_size: f32, text_top: f32) -> f32 {
    (f64::from(font_size) * f64::from(BASELINE_FACTOR) + f64::from(text_top)) as f32
}

/// `draw_caret`'s corners (0x63b990, geom slice 0x63b9af..0x63ba01). From the pen position
/// `(pen_x, pen_y)` (handed back by the font kernel after the input text), the caret quad spans
/// `[pen_x, caretRight] × [pen_y, caretBottom]` where **`caretRight = 2·scaleX + pen_x`**
/// (`fadd st,st; fadd pen_x`) and **`caretBottom = fontSize + pen_y`**. Returns `(caretRight,
/// caretBottom)`; the quad's other corners are `pen_x`/`pen_y` unchanged.
pub fn draw_caret(pen_x: f32, pen_y: f32, scale_x: f32, font_size: f32) -> (f32, f32) {
    let caret_right = (f64::from(scale_x) + f64::from(scale_x) + f64::from(pen_x)) as f32;
    let caret_bottom = (f64::from(font_size) + f64::from(pen_y)) as f32;
    (caret_right, caret_bottom)
}
