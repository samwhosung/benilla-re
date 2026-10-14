//! Six small leaves of the sky code: the two fog and cloud colour modifiers (`0x6cf1a0`,
//! `0x6cf290`, `systems/lighting/fog.md#dead-fog-modifiers`), the clamp-free lerp `0x6d15f0`
//! (`daynight.md#area-overblend`), the third `0x6d6e50` (`shadows.md#blob-shadow`), the storm
//! weight `0x6d4500` (`daynight.md#storm-weight`) and the cloud cover byte `0x6d0970`
//! (`clouds.md#cloud-cover`).
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`.

use crate::x87::ftol;

/// `[0x7ffe58]` = 255.0: the byte scale and the modifiers' blend gain.
const SCALE_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]` = 512.0: the repack bias, which puts the rounded byte in bits 14 to 21 of the f32
/// store.
const REPACK_BIAS: f32 = f32::from_bits(0x4400_0000);

/// The blend byte both modifiers derive from their scale: `bits(f32((1.0 - scale) * 255 + 512)) >>
/// 14` low 8 bits (`fld 1.0; fsub scale; fmul 255; fadd 512; fstp [ebp-4]; shr 0xe`).
fn blend_factor(scale: f32) -> u32 {
    let v = ((1.0_f64 - f64::from(scale)) * f64::from(SCALE_255) + f64::from(REPACK_BIAS)) as f32;
    (v.to_bits() >> 14) & 0xff
}

/// The record `0x6cf1a0` works on at `ecx`: a packed colour dword and two floats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FogColorModulate {
    /// `[+0]`, the packed colour.
    pub color: u32,
    /// `[+4]`, scaled by the fog scale.
    pub f4: f32,
    /// `[+8]`, scaled by the fog scale.
    pub f8: f32,
}

/// One channel of the blend at `0x6cf214` to `0x6cf268`: `cur + (((g - cur) * blend) >> 8)` with
/// the signed `sub` and `imul`, a logical `shr 8` of the product, and the 8-bit `add dl, cl`.
fn fog_blend_channel(g: u8, cur: u8, blend: u32) -> u8 {
    let diff = i32::from(g) - i32::from(cur);
    let prod = diff.wrapping_mul(blend as i32) as u32;
    ((prod >> 8) as u8).wrapping_add(cur)
}

/// `0x6cf1a0` ApplyFogColourModifier (`systems/lighting/fog.md#dead-fog-modifiers`), the whole
/// function: both floats are multiplied by the scale `[0xce99d0]` and rounded; then the blend byte
/// of the scale picks the colour: 0 leaves it, 0xff takes the RGB of `[0xce99f0]` and keeps the
/// alpha, and anything else blends each channel toward it ([`fog_blend_channel`]), alpha kept.
#[must_use]
pub fn fog_color_modulate(rec: FogColorModulate, g_scale: f32, g_color: u32) -> FogColorModulate {
    let f4 = (f64::from(rec.f4) * f64::from(g_scale)) as f32;
    let f8 = (f64::from(rec.f8) * f64::from(g_scale)) as f32;
    let blend = blend_factor(g_scale);
    let color = if blend == 0 {
        rec.color
    } else if blend == 0xff {
        (g_color & 0x00ff_ffff) | (rec.color & 0xff00_0000)
    } else {
        let r = fog_blend_channel((g_color >> 16) as u8, (rec.color >> 16) as u8, blend);
        let g = fog_blend_channel((g_color >> 8) as u8, (rec.color >> 8) as u8, blend);
        let b = fog_blend_channel(g_color as u8, rec.color as u8, blend);
        (rec.color & 0xff00_0000) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
    };
    FogColorModulate { color, f4, f8 }
}

/// One channel of `0x6cf290` at `0x6cf2c0` to `0x6cf2f9`: `((cur * blend + 0xff) >> 8) & 0xff`.
fn cloud_modulate_channel(cur: u8, blend: u32) -> u8 {
    (((u32::from(cur) * blend + 0xff) >> 8) & 0xff) as u8
}

/// `0x6cf290` ScaleFogColourModifier (`systems/lighting/fog.md#dead-fog-modifiers`), the whole
/// function: each colour channel times the blend byte of the scale `[0xce99d4]`, alpha kept.
#[must_use]
pub fn cloud_color_modulate(color: u32, g_scale: f32) -> u32 {
    let blend = blend_factor(g_scale);
    let r = cloud_modulate_channel((color >> 16) as u8, blend);
    let g = cloud_modulate_channel((color >> 8) as u8, blend);
    let b = cloud_modulate_channel(color as u8, blend);
    (color & 0xff00_0000) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// `0x6d15f0` LerpChannel (`systems/lighting/daynight.md#area-overblend`), the whole function. `fld
/// b; fcomp a; test ah,1; jne` takes the second form when `b < a` or either is NaN: `b >= a` gives
/// `(b - a) * t + a`, otherwise `a - (a - b) * t`. The result stays wide in st0.
#[must_use]
pub fn sky_scalar_lerp(a: f32, b: f32, t: f32) -> f32 {
    if b >= a {
        ((f64::from(b) - f64::from(a)) * f64::from(t) + f64::from(a)) as f32
    } else {
        (f64::from(a) - (f64::from(a) - f64::from(b)) * f64::from(t)) as f32
    }
}

/// `[0x807a40]` = `0x3eaaaaab`, the f32 nearest 1/3.
const SCALE_THIRD: f32 = f32::from_bits(0x3eaa_aaab);

/// `0x6d6e50` ThirdOf (`systems/lighting/shadows.md#blob-shadow`), the whole function: `x *
/// [0x807a40]`.
#[must_use]
pub fn sky_scale_third(x: f32) -> f32 {
    (f64::from(x) * f64::from(SCALE_THIRD)) as f32
}

/// `[0x80306c]` = 4.0: the storm weight's density multiplier.
const DENSITY_SPAN: f32 = f32::from_bits(0x4080_0000);

/// `0x6d4500` UpdateStormWeight (`systems/lighting/daynight.md#storm-weight`), the whole function:
/// the value stored to `[0xce9bcc]` for the density `[0xce9ba0]`. `v = density * 4.0` stays wide;
/// `fld 1.0; fcomp v; test ah,5; jp` stores the literal 1.0 only when `1.0 < v` and `v` otherwise,
/// a NaN included.
#[must_use]
pub fn cloud_density_clamp(density: f32) -> f32 {
    let v = f64::from(density) * f64::from(DENSITY_SPAN);
    if 1.0_f64 < v {
        1.0
    } else {
        v as f32
    }
}

/// `0x6d0970` SetCloudCover (`systems/lighting/clouds.md#cloud-cover`), the whole function: the
/// byte stored at `[+8]`. `sel` is the argument when `[+0xc]` is zero (`fcomp 0.0; test ah,0x44;
/// jnp` is taken only for the equal case, so -0 too) and `[+0xc]` otherwise, a NaN included; the
/// byte is the low byte of `__ftol((1.0 - sel) * 255)` (`0x40a2b0`), 0 for a NaN or an overflow.
#[must_use]
pub fn sky_ftol_field_setter(field: f32, arg: f32) -> u8 {
    let sel = if field == 0.0 { arg } else { field };
    let r = (1.0_f64 - f64::from(sel)) * f64::from(SCALE_255);
    ftol(r) as u8
}
