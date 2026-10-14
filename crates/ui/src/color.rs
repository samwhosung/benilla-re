//! The colour arithmetic of the script bindings: how a script's colour channels become the bytes a
//! widget stores, and how the stored bytes become the numbers a getter pushes.
//!
//! A setter clamps each channel to `[0, 1]`, multiplies by 255.0 (`0x7ffe58`), adds 0.5 (`0x7ffa24`)
//! and truncates with `__ftol` (`0x40a2b0`); a getter multiplies a byte by one of three constants
//! and pushes an f64. Which of the stack value and the stored `f32` a step reads decides the last
//! bit, so each function names the store it follows. The 255.0 of a bare `SetAlpha` is the f64 at
//! `0x806498`, with no clamp and no 0.5.
//! `systems/ui/colorselect.md`, `systems/ui/chatcolors.md` and `systems/ui/fontstring.md#lua-methods`
//! hold the entries.

/// `0x7ffe58`, 255.0: the channel scale of the setters.
const SCALE_255_F32: f32 = f32::from_bits(0x437f_0000);
/// `0x7ffa24`, 0.5: the rounding bias before `__ftol`.
const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `0x806498`, 255.0 as an f64: the scale of the setters that clamp nothing and add nothing.
const SCALE_255_F64: f64 = 255.0;
/// `0x81c6b4`, the float `0x3b80806e`: the multiplier of the `Texture`, `FontString`, `Font`,
/// `Button` and backdrop colour getters, `GetFogColor` and `GetStatusBarColor`. It is 0.003921560,
/// not 1/255, so a byte of 255 gives 0.9999978.
const CHANNEL_SCALE: f32 = f32::from_bits(0x3b80_806e);
/// `0x804578`, 1/255 rounded to an f64.
const INV255_F64: f64 = f64::from_bits(0x3f70_1010_1010_1010);
/// `0x8026c8`, 1/255 rounded to an `f32`: the multiplier of the chat colours, the light colour and
/// the PvP rank progress.
const INV255_F32: f32 = f32::from_bits(0x3b80_8081);

/// The channel clamp of a setter on the f64 that `lua_tonumber` leaves in `st0`, for the red, green
/// and blue channels: a value below 0 gives 0.0, a value in `[0, 1)` is stored as an `f32`
/// (`fstp dword`), and anything else, NaN included, gives 1.0. The two `fcom` tests are
/// `test ah,5; jp`, whose parity is odd only for "less".
pub fn clamp01_rgb(value: f64) -> f32 {
    if value < 0.0 {
        0.0
    } else if value < 1.0 {
        value as f32
    } else {
        1.0
    }
}

/// The same clamp for a channel the code leaves on the stack, which stays an f64: the alpha of a
/// `SetVertexColor`-style call and the third channel of a gradient colour.
pub fn clamp01_alpha(value: f64) -> f64 {
    if value < 0.0 {
        0.0
    } else if value < 1.0 {
        value
    } else {
        1.0
    }
}

/// The clamp of a channel that is already an `f32`: NaN gives 1.0.
pub fn clamp01(value: f32) -> f32 {
    if value < 0.0 {
        0.0
    } else if value < 1.0 {
        value
    } else {
        1.0
    }
}

/// One channel of a setter: `c * 255.0 + 0.5` at 53 bits, then `__ftol` truncates toward zero.
pub fn quantize_channel(c: f64) -> i32 {
    let p = c * f64::from(SCALE_255_F32) + f64::from(HALF);
    benilla_re_cmath::x87::ftol(p)
}

/// [`quantize_channel`] stored as a byte.
pub fn quantize_byte(v: f64) -> u8 {
    quantize_channel(v) as u8
}

/// `0xAARRGGBB` from four script channels (alpha is argument 5, 1.0 when absent), as
/// `Texture::SetVertexColor` (`0x79abd0`), `FontString::SetTextColor` (`0x79d9c0`) and the other
/// colour setters assemble it: alpha quantizes first, then red, green and blue, with
/// `bh = A; bl = R; shl 8; or G; shl 8; or B`. Red, green and blue quantize from the `f32` clamp;
/// alpha from the f64 still on the stack.
pub fn pack_color(r: f64, g: f64, b: f64, a: f64) -> u32 {
    let rgb = |v: f64| quantize_channel(f64::from(clamp01_rgb(v))) as u32 & 0xff;
    let aq = quantize_channel(clamp01_alpha(a)) as u32 & 0xff;
    let (rq, gq, bq) = (rgb(r), rgb(g), rgb(b));
    (((aq << 8 | rq) << 8 | gq) << 8) | bq
}

/// One gradient colour of `Texture::SetGradient` (`0x79ae30`): alpha is forced to `0xff`
/// (`or edi,0xffffff00`), red and green quantize from the `f32` clamp, and the third channel keeps
/// the f64 as the alpha does.
pub fn pack_gradient_color(r: f64, g: f64, b: f64) -> u32 {
    let rgb = |v: f64| quantize_channel(f64::from(clamp01_rgb(v))) as u32 & 0xff;
    let (rq, gq) = (rgb(r), rgb(g));
    let bq = quantize_channel(clamp01_alpha(b)) as u32 & 0xff;
    ((((0xffff_ff00u32 | rq) << 8) | gq) << 8) | bq
}

/// `Texture::SetGradient`: the minimum colour (arguments 3 to 5) and the maximum (6 to 8).
pub fn pack_gradient(r0: f64, g0: f64, b0: f64, r1: f64, g1: f64, b1: f64) -> (u32, u32) {
    (
        pack_gradient_color(r0, g0, b0),
        pack_gradient_color(r1, g1, b1),
    )
}

/// `Texture::SetGradientAlpha` (`0x79b180`): two full colours, each with an optional alpha.
#[allow(clippy::too_many_arguments)]
pub fn pack_color_pair(
    r0: f64,
    g0: f64,
    b0: f64,
    a0: f64,
    r1: f64,
    g1: f64,
    b1: f64,
    a1: f64,
) -> (u32, u32) {
    (pack_color(r0, g0, b0, a0), pack_color(r1, g1, b1, a1))
}

/// A getter's channel: `fild` of the byte times the float at `0x81c6b4`, kept as an f64.
pub fn normalize_channel(byte: u8) -> f64 {
    f64::from(i32::from(byte)) * f64::from(CHANNEL_SCALE)
}

/// A getter's channel that is stored as an `f32` (`fstp dword`) and reloaded before the push.
pub fn normalize_channel_f32(byte: u8) -> f64 {
    f64::from(normalize_channel(byte) as f32)
}

/// A colour getter that pushes all four channels as f64, red first: red is `(c >> 16) & 0xff`,
/// green `(c >> 8) & 0xff`, blue `c & 0xff`, alpha `c >> 24`.
pub fn unpack_color(color: u32) -> [f64; 4] {
    [
        normalize_channel(((color >> 16) & 0xff) as u8),
        normalize_channel(((color >> 8) & 0xff) as u8),
        normalize_channel((color & 0xff) as u8),
        normalize_channel((color >> 24) as u8),
    ]
}

/// `Texture::GetVertexColor` (`0x79aa50`) and `FontString::GetTextColor` (`0x79d840`): the same
/// unpack, but only red is pushed as an f64; green, blue and alpha go through an `f32` store.
pub fn unpack_vertex_color(color: u32) -> [f64; 4] {
    [
        normalize_channel(((color >> 16) & 0xff) as u8),
        normalize_channel_f32(((color >> 8) & 0xff) as u8),
        normalize_channel_f32((color & 0xff) as u8),
        normalize_channel_f32((color >> 24) as u8),
    ]
}

/// A getter that multiplies the byte by the f64 1/255 at `0x804578`: `GetItemQualityColor`,
/// `Frame::GetAlpha`, `Texture::GetAlpha`, `FontString::GetAlpha` and `ColorSelect::GetColorRGB`. Pushed
/// as an f64.
pub fn channel_over_255(byte: u8) -> f64 {
    f64::from(i32::from(byte)) * INV255_F64
}

/// A getter that multiplies the byte by the `f32` 1/255 at `0x8026c8`: the chat window colour, the
/// re-normalized byte of `ChangeChatColor` and the PvP rank progress. Pushed as an f64.
pub fn channel_over_255_f32(byte: u8) -> f64 {
    f64::from(i32::from(byte)) * f64::from(INV255_F32)
}

/// `lua * 255.0` with the f64 at `0x806498`, truncated by `__ftol`, low byte stored: `SetAlpha`
/// (`0x79b580`, `0x79cb70`), `ChangeChatColor`, the chat window setters and the font colour byte.
/// There is no clamp and no 0.5.
pub fn scaled_byte(lua: f64) -> u8 {
    (benilla_re_cmath::x87::ftol(scale_255(lua))) as u8
}

/// The product `x * 255.0` of those setters, as it sits in `st0` entering `__ftol`.
pub fn scale_255(x: f64) -> f64 {
    x * SCALE_255_F64
}

/// The magnitude `Model::SetLight` takes of its intensity before it compares with 2.38e-7 (`fabs`,
/// `0x76e415` and `0x76e5b9`).
pub fn intensity_abs(intensity: f32) -> f64 {
    f64::from(intensity).abs()
}

/// `Model::SetLight`'s colour block (`0x76e54e` to `0x76e596`, and `0x76e71c` to `0x76e765`): three
/// bytes, each times the `f32` 1/255 and then the intensity. The x87 stack keeps the first two
/// products as f64 across the multiply; the third is stored as an `f32` and reloaded first, so it
/// narrows before the intensity. Returns the three `f32` stores.
pub fn light_color_block(c0: u8, c1: u8, c2: u8, intensity: f32) -> (f32, f32, f32) {
    let k = f64::from(INV255_F32);
    let i = f64::from(intensity);
    let p0 = f64::from(i32::from(c0)) * k;
    let p1 = f64::from(i32::from(c1)) * k;
    let p2 = f64::from(i32::from(c2)) * k;
    let p2_f32 = f64::from(p2 as f32);
    ((p0 * i) as f32, (p1 * i) as f32, (p2_f32 * i) as f32)
}
