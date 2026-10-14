//! Colour bytes to floats and back, and the arithmetic around them: `ConsoleSetShadowColor`
//! (`0x673290`), `GatherSlotLights` (`0x68df70`), `DrawInteriorGroupBatches` (`0x6b5190`) and
//! `UpdateFootprints` (`0x69a3e0`).
//!
//! `fild` loads an integer exactly; a product rounds at 53 bits and a store to `f32`.

/// `[0x8026c8]`: 1/255 (`0x3b808081`).
pub const INV_255: f32 = f32::from_bits(0x3b80_8081);
/// `[0x7ffe58]`: 255.0.
pub const COLOR_SCALE: f32 = 255.0;
/// `[0x7ff9d8]`: 1.0.
pub const ONE: f32 = 1.0;
/// `[0x7ffa24]`: 0.5.
pub const HALF_7FFA24: f32 = 0.5;
/// `[0x86a704]`: 0.5, subtracted before `fistp`.
pub const HALF_86A704: f32 = 0.5;
/// `[0x8106dc]`: 1/6000 (`0x392ec33e`), the footprint's fade per millisecond.
pub const INV_6000: f32 = f32::from_bits(0x392e_c33e);
/// `[0x8106d8]`: 0.00048828870 (`0x3a000080`).
pub const SCALE_8106D8: f32 = f32::from_bits(0x3a00_0080);

/// `ConsoleSetShadowColor` (`0x673290`) at `0x6732e2`, `0x6732f0`, `0x673302` and `0x673315`: one of
/// the four parsed floats times 255, kept in the register for `__ftol` (`0x40a2b0`).
pub fn color_scale255(v: f32) -> f64 {
    f64::from(v) * f64::from(COLOR_SCALE)
}

/// `f32(byte·(1/255))`, `fild; fmul [0x8026c8]; fstp`: in `GatherSlotLights` (`0x68df70`) at
/// `0x68e0b8`, `0x68e0cb` and `0x68e0d8` on the bytes at `+0x70` to `+0x72` of the object
/// `0x6d48b0` returns, and twelve times in `DrawInteriorGroupBatches` (`0x6b5190`), from `0x6b5326` to
/// `0x6b53ad` and from `0x6b56d5` to `0x6b5756`.
pub fn color_normalize(byte: u8) -> f32 {
    (f64::from(byte) * f64::from(INV_255)) as f32
}

/// `0x68df70` from `0x68df92` to `0x68dfb6`: `f32((hi − lo)·0.5 + lo)` of its argument's `+8` and
/// `+4`, stored to `[ebp-8]`.
pub fn light_mid_y(hi: f32, lo: f32) -> f32 {
    let d = f64::from(hi) - f64::from(lo);
    let m = d * f64::from(HALF_7FFA24);
    let r = m + f64::from(lo);
    r as f32
}

/// `0x68df70` from `0x68e0fd` to `0x68e125`: `(((z60·b4 + mid·b8) + b0·z) + bc) − bias − bias` at
/// 53 bits, the plane `0xc7bcb0` to `0xc7bcbc` and `bias` `[ebp-0x14]`, 23.570166 (`0x41bc8fb3`),
/// left for the `fcomp [0xc62964]`.
#[allow(clippy::too_many_arguments)]
pub fn doodad_plane_value(
    z: f32,
    z60: f32,
    mid: f32,
    b0: f32,
    b4: f32,
    b8: f32,
    bc: f32,
    bias: f32,
) -> f64 {
    let t0 = f64::from(z60) * f64::from(b4);
    let t1 = f64::from(mid) * f64::from(b8);
    let s0 = t0 + t1;
    let t2 = f64::from(b0) * f64::from(z);
    let s1 = s0 + t2;
    let s2 = s1 + f64::from(bc);
    let s3 = s2 - f64::from(bias);
    s3 - f64::from(bias)
}

/// `UpdateFootprints` (`0x69a3e0`) from `0x69a431` to `0x69a43a`: `1 − age·(1/6000)` at 53 bits, the
/// age a 64-bit tick count (`fild qword`).
pub fn footprint_fade_frac(age: i64) -> f64 {
    let scaled = (age as f64) * f64::from(INV_6000);
    f64::from(ONE) - scaled
}

/// `0x69a440` to `0x69a44b`: `fcom [0x7ffd74]; test ah,5; jp 0x69a474` from the exe's AH. The jump
/// to the scaling is taken on an even count of C0 and C2: the fraction at or above 0 scales, below
/// expires.
pub fn footprint_gate_scales(ah: u8) -> bool {
    (ah & 0x5).count_ones().is_multiple_of(2)
}

/// `0x69a474` to `0x69a4ad`: `min(fistp(f32(frac·255) − 0.5), 127)`; the product is stored by `fst`
/// and `fstp` before the subtraction.
pub fn footprint_alpha(frac: f64) -> i32 {
    let s = (frac * f64::from(COLOR_SCALE)) as f32;
    let a = crate::x87::fistp(f64::from(s) - f64::from(HALF_86A704));
    a.min(0x7f)
}

/// `0x69a53f`: `f32([ecx+0x24]·0.00048828870)`.
pub fn footprint_jitter(v: f32) -> f32 {
    (f64::from(v) * f64::from(SCALE_8106D8)) as f32
}

/// `0x69a555`, `0x69a56a`, `0x69a57b`: the camera's position (`0xc7cf20` to `0xc7cf28`) negated,
/// `fld; fchs; fstp`.
pub fn corner_ray_negate(g: f32) -> f32 {
    (-f64::from(g)) as f32
}
