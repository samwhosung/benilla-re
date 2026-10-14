//! The progress-bar quad geometry, a slice of `0x407150`.

const HALF: f32 = f32::from_bits(0x3f00_0000);

/// The quad of `0x407150` for one bar descriptor `{cx, cy, w, h, isFill}`.
///
/// Returns the corners `[x0, y0, x1, y1]`: `x0 = cx - w * 0.5`, `y0 = cy - h * 0.5`,
/// `y1 = h * 0.5 + cy`, and `x1 = w * 0.5 + cx` for the border or `x1 = progress * w + x0` for the
/// fill quad, which reveals in proportion to the loading fraction `0x882be4`. `w * 0.5` and `h * 0.5`
/// stay in `f64` across each pair of corners; each corner stores as `f32`.
pub fn progress_bar_quad(
    cx: f32,
    cy: f32,
    w: f32,
    h: f32,
    is_fill: bool,
    progress: f32,
) -> [f32; 4] {
    let whalf = w as f64 * HALF as f64;
    let hhalf = h as f64 * HALF as f64;
    let x0 = (cx as f64 - whalf) as f32;
    let y0 = (cy as f64 - hhalf) as f32;
    let y1 = (hhalf + cy as f64) as f32;
    let x1 = if is_fill {
        (progress as f64 * w as f64 + x0 as f64) as f32
    } else {
        (whalf + cx as f64) as f32
    };
    [x0, y0, x1, y1]
}
