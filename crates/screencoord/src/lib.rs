//! The screen aspect scale, `0x41ad10` to `0x41ae70`: four `.data` globals set from the screen's
//! aspect `s`, the helpers that multiply or divide coordinates by them, and the conversion every
//! script method applies between script units and layout units (`systems/screencoord.md`).
//!
//! The x87 runs at 53-bit precision, so a chain in registers is `f64`, an `f32` operand widens
//! exactly, and each `fst`/`fstp dword` is an `as f32`. The globals: `[0x832a40] = s`,
//! `[0x832a44] = K1 = s/√(s²+1)`, `[0x832a48] = K2 = 1/√(s²+1)`, `[0x832a4c] = A = s·0.75`.
#![allow(clippy::float_cmp)]

/// `[0x8012cc]`, 0.75, the factor `SetScreenAspect` applies to `s`.
pub const C075_BITS: u32 = 0x3f40_0000;
/// `[0x7ff9d8]`, 1.0, in `SetScreenAspect`'s `s²+1` and `1/√`.
pub const ONE_BITS: u32 = 0x3f80_0000;
/// `[0x7ffd68]`, 1024.0, the factor every script conversion applies to `A`.
pub const SCRIPT_SPAN_BITS: u32 = 0x4480_0000;

/// The four globals `SetScreenAspect(s)` sets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScaleState {
    /// `[0x832a40]`: `s` as passed.
    pub s: f32,
    /// `[0x832a44]`: `K1 = s/√(s²+1)`, the screen width in layout units.
    pub g44: f32,
    /// `[0x832a48]`: `K2 = 1/√(s²+1)`, the screen height in layout units.
    pub g48: f32,
    /// `[0x832a4c]`: `A = s·0.75`.
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

impl ScaleState {
    /// `A·1024` as the x87 holds it after `call 0x41ad70; fmul [0x7ffd68]`: exact, a power of two
    /// times a float.
    pub fn script_factor(self) -> f64 {
        f64::from(self.s075) * f64::from(f32::from_bits(SCRIPT_SPAN_BITS))
    }

    /// A layout length `l` as a getter answers it: `RN24(1024·A·l) / K1`, the quotient at 53 bits
    /// (`fmul [l]; fstp dword [esp]; call 0x41ae40`).
    pub fn to_script(self, l: f32) -> f64 {
        let t = (self.script_factor() * f64::from(l)) as f32;
        unscale_x(self.g44, t)
    }

    /// A rectangle edge `e` of a region whose layout scale is `scale`, as `GetLeft` and the other
    /// edge getters answer it: `e/scale` stays at 53 bits into the product,
    /// `RN24(1024·A·(e/scale)) / K1` (`fld e; fdiv scale; fmulp`).
    pub fn to_script_over(self, e: f32, scale: f32) -> f64 {
        let t = (self.script_factor() * (f64::from(e) / f64::from(scale))) as f32;
        unscale_x(self.g44, t)
    }

    /// The midpoint of `near` and `far` as `GetCenter` answers it: `(far − near)·0.5 + near` at
    /// 53 bits, then as [`ScaleState::to_script_over`].
    pub fn to_script_mid(self, near: f32, far: f32, scale: f32) -> f64 {
        let m = (f64::from(far) - f64::from(near)) * 0.5 + f64::from(near);
        let t = (self.script_factor() * (m / f64::from(scale))) as f32;
        unscale_x(self.g44, t)
    }

    /// A script number `v` as a setter stores it in layout units:
    /// `RN24(K1 · RN24(RN24(v) / (1024·A)))` (`fstp dword [v]; fdivr [v]; fstp dword; call 0x41ae60;
    /// fstp dword`).
    pub fn to_layout(self, v: f64) -> f32 {
        let v32 = v as f32;
        let q = (f64::from(v32) / self.script_factor()) as f32;
        scale_x(self.g44, q) as f32
    }
}

/// `MultiplyByScreenScaleX` (`0x41ae60`): `K1·v` in `st0` at 53 bits (`fld [0x832a44]; fmul [arg]`).
pub fn scale_x(g44: f32, v: f32) -> f64 {
    f64::from(g44) * f64::from(v)
}
/// `MultiplyByScreenScaleY` (`0x41ae70`): `K2·v` in `st0` at 53 bits.
pub fn scale_y(g48: f32, v: f32) -> f64 {
    f64::from(g48) * f64::from(v)
}
/// `DivideByScreenScaleX` (`0x41ae40`): `v/K1` in `st0` at 53 bits (`fld [arg]; fdiv [0x832a44]`).
pub fn unscale_x(g44: f32, v: f32) -> f64 {
    f64::from(v) / f64::from(g44)
}
/// `DivideByScreenScaleY` (`0x41ae50`): `v/K2` in `st0` at 53 bits.
pub fn unscale_y(g48: f32, v: f32) -> f64 {
    f64::from(v) / f64::from(g48)
}

/// `ScaleScreenDelta` (`0x41ad80`): `(K1·x, K2·y)`, each stored as a float only through a non-null
/// pointer.
pub fn scale_xy(g44: f32, g48: f32, x: f32, y: f32) -> (f32, f32) {
    (scale_x(g44, x) as f32, scale_y(g48, y) as f32)
}
/// `UnscaleScreenXY` (`0x41ade0`): `(x/K1, y/K2)`, each stored as a float.
pub fn unscale_xy(g44: f32, g48: f32, x: f32, y: f32) -> (f32, f32) {
    (unscale_x(g44, x) as f32, unscale_y(g48, y) as f32)
}

/// `0x41adb0`: a four-float rect, `out = [K1·in0, K2·in1, K1·in2, K2·in3]`, stored in the order
/// `[edx]`, `[edx+8]`, `[edx+4]`, `[edx+0xc]`.
pub fn scale_rect(g44: f32, g48: f32, r: [f32; 4]) -> [f32; 4] {
    [
        scale_x(g44, r[0]) as f32,
        scale_y(g48, r[1]) as f32,
        scale_x(g44, r[2]) as f32,
        scale_y(g48, r[3]) as f32,
    ]
}
/// `0x41ae10`: `out = [in0/K1, in1/K2, in2/K1, in3/K2]`.
pub fn unscale_rect(g44: f32, g48: f32, r: [f32; 4]) -> [f32; 4] {
    [
        unscale_x(g44, r[0]) as f32,
        unscale_y(g48, r[1]) as f32,
        unscale_x(g44, r[2]) as f32,
        unscale_y(g48, r[3]) as f32,
    ]
}
