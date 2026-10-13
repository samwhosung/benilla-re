//! The lighting a model is given: `ApplyEntityLighting` (`0x672a20`), `CMapDoodadDef::AddLights`
//! (`0x6a8050`), `GetMapObjectFog` (`0x6c31e0`) and `UpdateDoodadDef` (`0x6952a0`).
//!
//! A colour byte becomes `f32(byte·(1/255))` (`crate::colornorm::color_normalize`); a product of two
//! `f32` loads rounds at 53 bits and again at its `fstp dword`.

use crate::colornorm::color_normalize;

/// The three bytes in the order the client stores them, each `byte·(1/255)`.
fn color3(a: u8, b: u8, c: u8) -> [f32; 3] {
    [color_normalize(a), color_normalize(b), color_normalize(c)]
}

/// `ApplyEntityLighting` (`0x672a20`) with no context: the ambient colour it passes to `0x71bc70`,
/// the bytes `+0x17a`, `+0x179` and `+0x178` of the object `0x6d48b0` returns, stored to
/// `[ebp-0xc]` up.
pub fn daynight_ambient_color(b17a: u8, b179: u8, b178: u8) -> [f32; 3] {
    color3(b17a, b179, b178)
}

/// `0x672a20` with no context: the directional colour it passes to `0x71bce0`, the bytes `+0x176`,
/// `+0x175` and `+0x174`.
pub fn daynight_directional_color(b176: u8, b175: u8, b174: u8) -> [f32; 3] {
    color3(b176, b175, b174)
}

/// `0x672a20` with a context, `0x672a39` to the `jp` at `0x672a4a`: `[ctx+0x78] − [ctx+0x68]` at 53
/// bits against `[0xc62964]`, `test ah,5; jp 0x672a54`. The jump skips the call to `[vtbl+8]` on an
/// even count of C0 and C2, so the call is made only when the difference is below; equal, above
/// and NaN skip it. True when the call is made.
pub fn ctx_fog_gate_dispatch_second(field_78: f32, field_68: f32, fog_end: f32) -> bool {
    let diff = f64::from(field_78) - f64::from(field_68);
    let thr = f64::from(fog_end);
    let unordered = diff.is_nan() || thr.is_nan();
    let c0 = (diff < thr) || unordered;
    let c2 = unordered;
    (u8::from(c0) + u8::from(c2)) % 2 == 1
}

/// `CMapDoodadDef::AddLights` (`0x6a8050`) with `[this+0xc] & 2` set: the ambient colour, the bytes
/// `+0x9e`, `+0x9d` and `+0x9c`, stored to `[ebp-0x10]` up.
pub fn lightstate_interior_ambient(b9e: u8, b9d: u8, b9c: u8) -> [f32; 3] {
    color3(b9e, b9d, b9c)
}

/// `0x6a8050` with `[this+0xc] & 2` set: the directional colour, the bytes `+0xa2`, `+0xa1` and
/// `+0xa0`.
pub fn lightstate_interior_directional(ba2: u8, ba1: u8, ba0: u8) -> [f32; 3] {
    color3(ba2, ba1, ba0)
}

/// `0x6a8050` with the flag clear, `0x6a8134` to `0x6a8181`: the light block's (`[0xc89ef0]`)
/// `+0xb8`, `+0xbc` and `+0xc0` each times `[this+0xa4]`, loaded once and kept in the register
/// (`fmul st,st(1)`), each product stored as `f32`.
pub fn lightstate_exterior_scaled_directional(intensity: f32, comps: [f32; 3]) -> [f32; 3] {
    let i = f64::from(intensity);
    [
        (f64::from(comps[0]) * i) as f32,
        (f64::from(comps[1]) * i) as f32,
        (f64::from(comps[2]) * i) as f32,
    ]
}

/// `GetMapObjectFog` (`0x6c31e0`) inside a group with `[grp+8] & 0x10` and `[grp+0x98]` set
/// (`0x6c3233`): the fog colour from the bytes `+0x82`, `+0x81` and `+0x80` of the object
/// `0x6d48b0` returns, stored to `[ebp-0x10]` up.
pub fn fog_indoor_color(b82: u8, b81: u8, b80: u8) -> [f32; 3] {
    color3(b82, b81, b80)
}

/// `0x6c31e0` outdoors (`0x6c32b8`) or when no group qualifies (`0x6c327a`): the bytes `+0x72`,
/// `+0x71` and `+0x70`.
pub fn fog_outdoor_color(b72: u8, b71: u8, b70: u8) -> [f32; 3] {
    color3(b72, b71, b70)
}

/// `UpdateDoodadDef` (`0x6952a0`) at `0x69538f`: `[this+0x68] = f32([ebp-0x1c]·[this+0x30])`, the
/// fade from the transform `0x7bca80` wrote, times the scale.
pub fn chunk_fade_scale(fade: f32, scale: f32) -> f32 {
    (f64::from(fade) * f64::from(scale)) as f32
}
