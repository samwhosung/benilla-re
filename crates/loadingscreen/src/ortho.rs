//! The letterboxed full-frame ortho rect, a slice of `0x406a60`.

const HALF: f32 = f32::from_bits(0x3f00_0000);

/// The letterbox arithmetic of `0x406a60`, the block between the aspect getter `0x41ad70`
/// (`fld [0x832a4c]`) and the ortho set `0x58af60`.
///
/// `aspect` is the screen width over height (`0x832a4c`). Returns the six ortho arguments
/// `(xmin, xmax, ymin, ymax, near = 0, far = 1)`:
/// - `a > 1`: `w = 1 / a`, `xlo = (1 - w) * 0.5`, `ylo = 0`, `h = 1` (pillarbox);
/// - `a < 1`: `h = a`, `ylo = (1 - a) * 0.5`, `xlo = 0`, `w = 1` (letterbox);
/// - `a == 1`: `xlo = 0`, `w = 1`, `ylo = 0`, `h = 1`.
///
/// `xmax = w + xlo` and `ymax = h + ylo`: `f64` intermediates, each local stored as `f32`.
pub fn aspect_fit_ortho(aspect: f32) -> [f32; 6] {
    let (mut w, mut xlo, mut ylo, mut h) = (1.0f32, 0.0f32, 0.0f32, 1.0f32);
    if aspect > 1.0 {
        w = (1.0_f64 / aspect as f64) as f32;
        xlo = ((1.0_f64 - w as f64) * HALF as f64) as f32;
    } else if aspect < 1.0 {
        h = aspect; // fst m32 (aspect already f32)
        ylo = ((1.0_f64 - aspect as f64) * HALF as f64) as f32;
    }
    let xmax = (w as f64 + xlo as f64) as f32;
    let ymax = (h as f64 + ylo as f64) as f32;
    [xlo, xmax, ylo, ymax, 0.0, 1.0]
}
