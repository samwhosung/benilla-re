//! The colour wheel of `ColorSelect` (`systems/ui/colorselect.md`): the polar lookup texture, the
//! thumb's position, and the cursor drag that sets hue, saturation and value.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fstp dword` is an `as f32`.
//! `fpatan`, `fsincos` and `fsqrt` are the host's `atan2`, `sin_cos` and `sqrt`; the emulator
//! routes them through the host's maths library as well, so they compare bit for bit.

use benilla_re_cmath::colour::{color_pack_rgb, hsv_to_rgb};

/// π, the `f32` at `0x804214` and `0x81d0d0`.
const PI: f64 = f32::from_bits(0x4049_0fdb) as f64;
/// π/180, the `f32` at `0x7ffaac`.
const DEG2RAD: f64 = f32::from_bits(0x3c8e_fa35) as f64;
/// 180/π, the `f32` at `0x803a78`.
const RAD2DEG: f64 = f32::from_bits(0x4265_2ee1) as f64;
/// 1/64, the `f32` at `0x80a92c`: texels to saturation.
const INV64: f64 = f32::from_bits(0x3c80_0000) as f64;

/// One texel inside the disc: the polar position of `(x, y)` as a hue in degrees and a
/// saturation, run through HSV to RGB at value 1.0 and packed. `r2 = x^2 + y^2` is the integer the
/// binary loads with `fild` for the radius. The colour steps are `0x7bbd60` and `0x7bbec0`, which
/// `benilla_re_cmath` implements.
fn texel(x: i32, y: i32, r2: i32) -> u32 {
    // hue: (atan2(y, x) + π) * 180/π, stored as `f32` (`fstp [ebp-0x2c]`)
    let angle = ((f64::from(y).atan2(f64::from(x)) + PI) * RAD2DEG) as f32;
    // saturation: sqrt(x^2 + y^2) / 64, stored as `f32` (`fstp [ebp-0x28]`)
    let radius = (f64::from(r2).sqrt() * INV64) as f32;
    color_pack_rgb(&hsv_to_rgb(&[angle, radius, 1.0]))
}

/// The wheel texture generator `0x78b580` (`colorselect#a-missing-wheel-faults`): fills a 128 by
/// 128 ARGB texture, row-major. The binary walks `y` from 63 down to -64 and `x` from -64 up to 63
/// and stores one dword per texel, so `out[0]` is `(x = -64, y = 63)`. A texel inside the disc
/// `x^2 + y^2 <= 0x1000` (`cmp ..., 0x1000; jg`) gets [`texel`]; the rest are 0, preset before the
/// disc test. The texture's allocation (`0x526930`) and registration (`0x58a980`, `0x449bf0`) are
/// not part of the fill.
pub fn build_wheel_texture(out: &mut [u32; 128 * 128]) {
    let mut idx = 0usize;
    for y in (-64i32..=63).rev() {
        for x in -64i32..=63 {
            let r2 = x * x + y * y;
            out[idx] = if r2 > 0x1000 { 0 } else { texel(x, y, r2) };
            idx += 1;
        }
    }
}

/// The offset `0x78bc20` gives the thumb's anchor (`0x767c70`, the thumb's point 4 to the wheel's
/// point 4; `colorselect#the-thumbs`): places the thumb for the hue `deg` (`this+0x328`) and the saturation `radius_frac` (`this+0x32c`). `cb` is
/// the wheel frame's width, the result of the getter at vtable slot `+0x1c` on `this+0x318`.
///
/// `theta = (deg * π/180)` is stored as `f32` (`fstp [ebp-4]`), and its cosine and sine, from
/// `fsincos`, each as `f32`. `m = cb * 0.5 * radius_frac` stays at register precision, and
/// `mx = (cos * m) as f32`, `my = (m * sin) as f32`. The call receives `-mx` and `-my`. The early
/// exits for a missing wheel (`this+0x318`) or thumb (`this+0x31c`) are not modelled.
pub fn thumb_offset(deg: f32, radius_frac: f32, cb: f32) -> [f32; 2] {
    let theta = (f64::from(deg) * DEG2RAD) as f32;
    let (sin, cos) = f64::from(theta).sin_cos();
    let cos = cos as f32;
    let sin = sin as f32;
    let m = f64::from(cb) * 0.5 * f64::from(radius_frac);
    let mx = (f64::from(cos) * m) as f32;
    let my = (m * f64::from(sin)) as f32;
    [-mx, -my]
}

/// What `0x78bd80` writes to the colour select frame.
#[derive(Clone, Copy, Debug)]
pub struct Drag {
    /// The hue in degrees, stored at `this+0x328` when the wheel mode flag `this+0x314` is set.
    pub angle_deg: f32,
    /// The saturation, stored at `this+0x32c` with the hue.
    pub radius_frac: f32,
    /// The value, stored at `this+0x330` when the slider flag `this+0x315` is set.
    pub slider_t: f32,
}

/// The drag handler `0x78bd80` (`colorselect#dragging`): the hue, saturation and value for the
/// cursor `(px, py)` (`+0x24`, `+0x28` of the event), read against the frame rectangle `rect` =
/// `[R0, R1, R2, R3]` that `0x768320` copies from the wheel frame (`+0x318`) and the slider frame
/// (`+0x320`). The flags only gate the stores, not the values.
///
/// The wheel: the centres `mA = (R1 + R3) * 0.5` and `mB = (R0 + R2) * 0.5` and the half-spans
/// `hA = (R3 - R1) * 0.5` and `hB = (R2 - R0) * 0.5` at register precision;
/// `dxv = (px - mA) as f32` (`fstp [ebp-8]`) and `dyv = py - mB`; `nx = dxv / hA`, `ny = dyv / hB`.
/// `mag = sqrt(ny^2 + nx^2)` is stored as `f32` and is used for the saturation only, not for the
/// angle: `angle_deg = ((atan2(ny, nx) + π) * 180/π) as f32`, `radius_frac = mag` if `mag < 1`, else
/// 1.0 (including a NaN).
///
/// The slider: `t = (py - R0) / (R2 - R0)` in one `f64` chain with no test for a zero divisor,
/// clamped: `t < 0` gives 0.0, `t < 1` gives `t`, and `t >= 1` or NaN gives 1.0; stored as `f32`.
pub fn drag_colour(px: f32, py: f32, rect: [f32; 4]) -> Drag {
    let [r0, r1, r2, r3] = rect.map(f64::from);
    let m_a = (r1 + r3) * 0.5;
    let m_b = (r0 + r2) * 0.5;
    let h_a = (r3 - r1) * 0.5;
    let h_b = (r2 - r0) * 0.5;
    let dxv = (f64::from(px) - m_a) as f32;
    let dyv = f64::from(py) - m_b;
    let nx = f64::from(dxv) / h_a;
    let ny = dyv / h_b;
    let magf = (nx * nx + ny * ny).sqrt() as f32;
    let angle_deg = ((ny.atan2(nx) + PI) * RAD2DEG) as f32;
    let radius_frac = if magf < 1.0 { magf } else { 1.0 };
    let tv = (f64::from(py) - r0) / (r2 - r0);
    let slider_t = if tv < 0.0 {
        0.0
    } else if tv < 1.0 {
        tv as f32
    } else {
        1.0
    };
    Drag {
        angle_deg,
        radius_frac,
        slider_t,
    }
}
