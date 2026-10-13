//! The screen aspect scale, `0x41ad10` to `0x41ae70`: four `.data` globals set from the screen's
//! aspect `s`, and helpers that multiply or divide coordinates by them
//! (`systems/input.md#click-versus-drag`).
//!
//! The x87 runs at 53-bit precision, so a chain in registers is `f64`, an `f32` operand widens
//! exactly, and each `fst`/`fstp dword` is an `as f32`. The globals: `[0x832a40] = s`,
//! `[0x832a44] = s/√(s²+1)` (G44), `[0x832a48] = 1/√(s²+1)` (G48), `[0x832a4c] = s·0.75`.
#![allow(clippy::float_cmp)]

/// `[0x8012cc]`, 0.75, the factor `SetScreenAspect` applies to `s`.
pub const C075_BITS: u32 = 0x3f40_0000;
/// `[0x7ff9d8]`, 1.0, in `SetScreenAspect`'s `s²+1` and `1/√`.
pub const ONE_BITS: u32 = 0x3f80_0000;

/// The four globals `SetScreenAspect(s)` sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleState {
    /// `[0x832a40]`: `s` as passed.
    pub s: f32,
    /// `[0x832a44]`: `s/√(s²+1)`, the x scale (G44).
    pub g44: f32,
    /// `[0x832a48]`: `1/√(s²+1)`, the y scale (G48).
    pub g48: f32,
    /// `[0x832a4c]`: `s·0.75`.
    pub s075: f32,
}

/// `SetScreenAspect` (`0x41ad10`): copies `s` to `[0x832a40]`, stores `s·0.75` to `[0x832a4c]`, and
/// computes `invlen = 1/√(s²+1)` on the x87 stack. `fst [0x832a48]` stores it rounded to `f32`
/// without popping, and `[0x832a44]` is `s` times the unrounded `invlen` still in `st0`.
pub fn set_scale(s: f32) -> ScaleState {
    let one = f64::from(f32::from_bits(ONE_BITS));
    let s075 = (f64::from(s) * f64::from(f32::from_bits(C075_BITS))) as f32; // fmul [0x8012cc]; fstp
    let invlen = one / (f64::from(s) * f64::from(s) + one).sqrt(); // fld s; fmul s; fadd 1; fsqrt; fdivr 1
    let g48 = invlen as f32; // fst [0x832a48], no pop
    let g44 = (invlen * f64::from(s)) as f32; // fmul s; fstp [0x832a44], from the unrounded invlen
    ScaleState { s, g44, g48, s075 }
}

/// `MultiplyByScreenScaleX` (`0x41ae60`): `G44·v` (`fld [0x832a44]; fmul [arg]`).
pub fn scale_x(g44: f32, v: f32) -> f32 {
    (f64::from(g44) * f64::from(v)) as f32
}
/// `NDCToDDCHeight` (`0x41ae70`): `G48·v`.
pub fn scale_y(g48: f32, v: f32) -> f32 {
    (f64::from(g48) * f64::from(v)) as f32
}
/// `DivideByScreenScaleX` (`0x41ae40`): `v/G44` (`fld [arg]; fdiv [0x832a44]`).
pub fn unscale_x(g44: f32, v: f32) -> f32 {
    (f64::from(v) / f64::from(g44)) as f32
}
/// `0x41ae50`: `v/G48`.
pub fn unscale_y(g48: f32, v: f32) -> f32 {
    (f64::from(v) / f64::from(g48)) as f32
}

/// `ScaleScreenDelta` (`0x41ad80`): `(G44·x, G48·y)`, each stored only through a non-null pointer.
pub fn scale_xy(g44: f32, g48: f32, x: f32, y: f32) -> (f32, f32) {
    (scale_x(g44, x), scale_y(g48, y))
}
/// `0x41ade0`: `(x/G44, y/G48)`.
pub fn unscale_xy(g44: f32, g48: f32, x: f32, y: f32) -> (f32, f32) {
    (unscale_x(g44, x), unscale_y(g48, y))
}

/// `0x41adb0`: a four-float rect, `out = [G44·in0, G48·in1, G44·in2, G48·in3]`, stored in the order
/// `[edx]`, `[edx+8]`, `[edx+4]`, `[edx+0xc]`.
pub fn scale_rect(g44: f32, g48: f32, r: [f32; 4]) -> [f32; 4] {
    [
        scale_x(g44, r[0]),
        scale_y(g48, r[1]),
        scale_x(g44, r[2]),
        scale_y(g48, r[3]),
    ]
}
/// `0x41ae10`: `out = [in0/G44, in1/G48, in2/G44, in3/G48]`.
pub fn unscale_rect(g44: f32, g48: f32, r: [f32; 4]) -> [f32; 4] {
    [
        unscale_x(g44, r[0]),
        unscale_y(g48, r[1]),
        unscale_x(g44, r[2]),
        unscale_y(g48, r[3]),
    ]
}
