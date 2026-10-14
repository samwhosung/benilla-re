//! The hue, saturation and value conversions (`0x7bbc80`, `0x7bbd60`). A colour is three `f32`
//! channels; hue is in degrees.

use crate::raymesh::{dominant_axis, minor_axis};

/// `0x7bbc80` (`ecx` = rgb, `edx` = out): `[hue, saturation, value]`. The value is the channel
/// `DominantAxis` picks and the minor channel `0x7bf700` picks; saturation is `(value − minor) /
/// value`, or 0 when the value is exactly 0 (NaN is not 0). Hue is −1 when the saturation is
/// exactly 0 (NaN is not); otherwise, with `c = value − minor` at 53 bits, the sector is `(g − b)
/// / c`, `(b − r) / c + 2` or `(r − g) / c + 4` for a dominant red, green or blue, stored as `f32`,
/// times 60 and stored as `f32`, plus 360 when below 0 (not −0, not NaN).
pub fn rgb_to_hsv(rgb: &[f32; 3]) -> [f32; 3] {
    let f = f64::from;
    let dom = dominant_axis(rgb);
    let minor = minor_axis(rgb);
    let value = rgb[dom];
    let sat = if value == 0.0 {
        0.0
    } else {
        ((f(value) - f(rgb[minor])) / f(value)) as f32
    };
    let hue = if sat == 0.0 {
        -1.0
    } else {
        let chroma = f(value) - f(rgb[minor]);
        let sector = match dom {
            0 => ((f(rgb[1]) - f(rgb[2])) / chroma) as f32,
            1 => ((f(rgb[2]) - f(rgb[0])) / chroma + 2.0) as f32,
            _ => ((f(rgb[0]) - f(rgb[1])) / chroma + 4.0) as f32,
        };
        let degrees = (f(sector) * 60.0) as f32;
        if degrees < 0.0 {
            (f(degrees) + 360.0) as f32
        } else {
            degrees
        }
    };
    [hue, sat, value]
}

/// `0x7bbd60` (`ecx` = hsv, `edx` = out). Saturation exactly 0 (NaN is not) gives `[v, v, v]`.
/// Otherwise a hue of exactly 360 becomes 0, and the sector float is `h · 0.016666668` (the `f32`
/// `0x3c888889`) at 53 bits. The sector is byte 1 of the `f32` store of `sector + 512`
/// (`bits >> 14`, low byte), capped at 5, and the fraction is `sector float − sector` stored as
/// `f32`. With `p = (1 − s)·v` kept at 53 bits, `q = (1 − frac·s)·v` and `t = (1 − (1 − frac)·s)·v`
/// stored as `f32`, sectors 0 to 5 give `(v, t, p)`, `(q, v, p)`, `(p, v, t)`, `(p, q, v)`,
/// `(t, p, v)` and `(v, p, q)`.
pub fn hsv_to_rgb(hsv: &[f32; 3]) -> [f32; 3] {
    let f = f64::from;
    let (h, s, v) = (hsv[0], hsv[1], hsv[2]);
    if s == 0.0 {
        return [v, v, v];
    }
    let hue = if h == 360.0 { 0.0 } else { h };
    let sector_float = f(hue) * f(f32::from_bits(0x3c88_8889));
    let magic = (sector_float + 512.0) as f32;
    let sector = ((magic.to_bits() >> 14) & 0xff).min(5);
    let frac = (sector_float - f64::from(sector)) as f32;
    let p = (1.0 - f(s)) * f(v);
    let q = ((1.0 - f(frac) * f(s)) * f(v)) as f32;
    let t = ((1.0 - (1.0 - f(frac)) * f(s)) * f(v)) as f32;
    let p = p as f32;
    match sector {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}
