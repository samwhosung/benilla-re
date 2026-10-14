//! The arithmetic of three functions whose bodies call foreign code, run as slices: `UpdateGlare`
//! (`0x6cf490`, `systems/lighting/celestial.md#glare-update`), `BuildSceneFog` (`0x6cee30`,
//! `systems/lighting/fog.md#scene-fog`) and `BlendAreaLights` (`0x6d2d00`,
//! `systems/lighting/daynight.md#area-lights`). Each slice is the exe's own instructions between
//! two addresses; where an operand is left on the x87 stack by a call, the test supplies it as a
//! double and runs the same opcode bytes. The x87 runs at 53 bits and stores `f32` at each `fstp
//! dword`; an `fst dword` stores the slot and keeps the wide value, which the next multiply may
//! use.

use crate::x87::fistp;

const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x7ffe58]`, 255.
const C_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]`, 512.
const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x8026c8]`, 1/255.
const C_INV255: f32 = f32::from_bits(0x3b80_8081);
/// `[0x811630]`, 2880.
const C_2880: f32 = f32::from_bits(0x4534_0000);
/// `[0x86f2c8]`, 0.5.
const C_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x8115b0]`, 0.25.
const C_QUARTER: f32 = f32::from_bits(0x3e80_0000);
/// `[0x8115fc]`, 7.2135 (`0x40e6d4ca`): the phase-fade angular scale.
const C_PHASE_K: f32 = f32::from_bits(0x40e6_d4ca);

/// The normalized direction and `1/len` that `sky_dir_normalize` stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyDirNorm {
    /// `[ebp-0x10]` = `dx · inv` (dx held LIVE f64 into the multiply).
    pub nx: f32,
    /// `[ebp-0xc]` = `dy · inv` (dy held LIVE f64 into the multiply).
    pub ny: f32,
    /// `[ebp-8]` = `dz_f32 · inv` (dz RELOADED from its f32 slot — NOT the live f64).
    pub nz: f32,
    /// `[ebp-4]` = `1/len`.
    pub inv: f32,
}

/// `UpdateGlare` from `0x6cf4a0` to `0x6cf4f1`: `d = pos − origin` (the camera at `0xce9b78`), `dz`
/// stored as `f32` with `fst` and kept wide; `len = √(((dz·f32(dz)) + dy²) + dx²)`; `inv = 1/len`
/// stored as `f32`; `nx = f32(dx·inv)`, `ny = f32(dy·inv)` from the wide `dx`, `dy`, and `nz =
/// f32(f32(dz)·inv)`.
pub fn sky_dir_normalize(pos: [f32; 3], origin: [f32; 3]) -> SkyDirNorm {
    let f = f64::from;
    let dx = f(pos[0]) - f(origin[0]);
    let dy = f(pos[1]) - f(origin[1]);
    let dz = f(pos[2]) - f(origin[2]);

    let dz_f32 = dz as f32;
    let len2 = (dz * f(dz_f32) + dy * dy) + dx * dx;
    let len = len2.sqrt();
    let inv = f(C_ONE) / len;
    let inv_f32 = inv as f32;

    SkyDirNorm {
        nx: (dx * f(inv_f32)) as f32,
        ny: (dy * f(inv_f32)) as f32,
        nz: (f(dz_f32) * f(inv_f32)) as f32,
        inv: inv_f32,
    }
}

/// `UpdateGlare`'s `fmul dword [mem]` on a value left on the stack, stored as `f32`.
pub fn sky_factor_mul(st0: f64, mem: f32) -> f32 {
    (st0 * f64::from(mem)) as f32
}

/// `UpdateGlare` from `0x6cf557` to `0x6cf56a`: `f32((1 − m)·s)`, with `s` the wide stack value.
pub fn sky_band_scale(m: f32, s: f64) -> f32 {
    ((f64::from(C_ONE) - f64::from(m)) * s) as f32
}

/// `UpdateGlare` from `0x6cf5a8` to `0x6cf5b1`: `f32(x·scale + base)`.
pub fn sky_clamp_up(x: f32, scale: f32, base: f32) -> f32 {
    (f64::from(x) * f64::from(scale) + f64::from(base)) as f32
}

/// `UpdateGlare` from `0x6cf5ce` to `0x6cf5d7`: `f32(base − x·scale)`.
pub fn sky_clamp_down(x: f32, scale: f32, base: f32) -> f32 {
    (f64::from(base) - f64::from(x) * f64::from(scale)) as f32
}

/// `UpdateGlare` from `0x6cf5ed` to `0x6cf60c`: the light direction at `0xce9b84` dotted with `n`,
/// as `((lz·nz) + (ly·ny)) + (lx·nx)` at 53 bits, stored as `f32`.
pub fn sky_light_dot(n: [f32; 3], light: [f32; 3]) -> f32 {
    let f = f64::from;
    (((f(light[2]) * f(n[2])) + (f(light[1]) * f(n[1]))) + (f(light[0]) * f(n[0]))) as f32
}

/// The two values of the exposure block: the density at `+0x24` and the packed float.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyExposurePre {
    /// `[esi+0x24]` = `((esi[0x94] - esi[0x90]) · t + esi[0x90]) · esi[0x20]`.
    pub e24: f32,
    /// `[ebp-4]` (f32 bits) = `(((esi[0xa0] - esi[0x9c]) · t + esi[0x9c]) · esi[0x30]) · (byte/255)
    /// · 255 + 512`.
    pub packed: f32,
}

/// `UpdateGlare` from `0x6cf625` to `0x6cf68e`: `t = (d − e98)/(1 − e98)`; `e24 = f32(((e94 −
/// e90)·t + e90)·e20)`; and `p = ((ea0 − e9c)·t + e9c)·e30` kept wide, packed as
/// `f32((p·(byte/255))·255 + 512)`.
#[allow(clippy::too_many_arguments)]
pub fn sky_exposure_pre(
    d: f32,
    e98: f32,
    e94: f32,
    e90: f32,
    e20: f32,
    ea0: f32,
    e9c: f32,
    e30: f32,
    byte: u8,
) -> SkyExposurePre {
    let f = f64::from;
    let t = (f(d) - f(e98)) / (f(C_ONE) - f(e98));
    let e24 = (((f(e94) - f(e90)) * t + f(e90)) * f(e20)) as f32;
    let p = ((f(ea0) - f(e9c)) * t + f(e9c)) * f(e30);
    let bi = f64::from(i32::from(byte));
    let packed = ((p * (bi * f(C_INV255))) * f(C_255) + f(C_512)) as f32;
    SkyExposurePre { e24, packed }
}

/// `BuildSceneFog` from `0x6cee61` to `0x6cee89`: the scene fog start `f32([0xce9c54]·[0xce9bd8])`.
pub fn dn_blue_premul(c54: f32, bd8: f32) -> f32 {
    (f64::from(c54) * f64::from(bd8)) as f32
}

/// `BuildSceneFog` at `0x6cef50`: the target fog start `f32([0xce9bf4]·rec[+4])`.
pub fn dn_case2_premul(bf4: f32, rec4: f32) -> f32 {
    (f64::from(bf4) * f64::from(rec4)) as f32
}

/// The two blend steps of `BuildSceneFog`: `phase·[0xce9b9c]` added to or subtracted from
/// `[0xce9bdc]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DnBlendFactor {
    /// 0x6cefcb path (record present): `0.25 · [0xce9b9c] + [0xce9bdc]` (`fld 0.25; fmul
    /// [0xce9b9c]; fadd [0xce9bdc]`).
    pub add: f32,
    /// 0x6cf011 path (no record): `[0xce9bdc] - 0.25 · [0xce9b9c]` (`fld 0.25; fmul [0xce9b9c];
    /// fsubr [0xce9bdc]`; the `fsubr` reverses: `mem - st0`). Stored via `fst [0xce9bdc]`.
    pub sub: f32,
}

/// `BuildSceneFog` from `0x6cefcb` to `0x6cefdd`: `f32(0.25·c9c + bdc)`.
pub fn dn_blend_add(c9c: f32, bdc: f32) -> f32 {
    (f64::from(C_QUARTER) * f64::from(c9c) + f64::from(bdc)) as f32
}

/// `BuildSceneFog` from `0x6cf011` to `0x6cf023`: `f32(bdc − 0.25·c9c)`.
pub fn dn_blend_sub(c9c: f32, bdc: f32) -> f32 {
    (f64::from(bdc) - f64::from(C_QUARTER) * f64::from(c9c)) as f32
}

/// The two fog distances `BuildSceneFog` writes at `+0x88` and `+0x84`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DnFogColorLerp {
    /// `[0xce9be8]` = block-2 fog **END** = `(a0 - b0)·t + b0`, `a0=[0xce9bf4]` (target end),
    /// `b0=[0xce9bd8]` (block-1 end), `t=[0xce9bdc]`.
    pub be8: f32,
    /// `[0xce9be4]` = block-2 fog **START** = `(a1 - b1)·t + b1`, `a1=[0xce9bf0]` (target start),
    /// `b1=[0xce9bd4]` (block-1 start), `t=[0xce9bdc]`.
    pub be4: f32,
}

/// `BuildSceneFog` from `0x6cf054` to `0x6cf099`: `f32((a − b)·t + b)` for each member.
pub fn dn_fog_color_lerp(a0: f32, b0: f32, a1: f32, b1: f32, t: f32) -> DnFogColorLerp {
    let f = f64::from;
    DnFogColorLerp {
        be8: ((f(a0) - f(b0)) * f(t) + f(b0)) as f32,
        be4: ((f(a1) - f(b1)) * f(t) + f(b1)) as f32,
    }
}

/// `BuildSceneFog` from `0x6cf099` to `0x6cf0ab`: `f32(bdc·255 + 512)`.
pub fn dn_repack_scale(bdc: f32) -> f32 {
    (f64::from(bdc) * f64::from(C_255) + f64::from(C_512)) as f32
}

/// `BuildSceneFog` from `0x6cf11a` to `0x6cf138`: `((b98 − c9e8)/c9d8)·7.2135`, passed to `exp2`
/// (`0x454d40`) as a double.
pub fn dn_phase_fade_arg(b98: f32, c9e8: f32, c9d8: f32) -> f64 {
    let f = f64::from;
    (f(b98) - f(c9e8)) / f(c9d8) * f(C_PHASE_K)
}

/// `BuildSceneFog` from `0x6cf13d` to `0x6cf17e`: `recip = 1/w` for the `exp2` result `w`, clamped
/// to [0, 1]: below 0 it is 0 (`fcom`, `test ah,5; jp`), below 1 it is kept (`jnp`), and 1, above 1
/// and NaN give 1. The result is `f32(1 − clamped)`.
pub fn dn_phase_alpha_delta(w: f64) -> f32 {
    let recip = f64::from(C_ONE) / w;
    let clamped = if recip < 0.0 {
        0.0
    } else if recip < f64::from(C_ONE) {
        recip
    } else {
        f64::from(C_ONE)
    };
    (f64::from(C_ONE) - clamped) as f32
}

/// `BlendAreaLights` from `0x6d2d69` to `0x6d2da0`: `√((dz² + dy²) + dx²)` of the camera minus the
/// light, stored as `f32`.
pub fn cloud_layer_distance(origin: [f32; 3], pos: [f32; 3]) -> f32 {
    let f = f64::from;
    let dx = f(origin[0]) - f(pos[0]);
    let dy = f(origin[1]) - f(pos[1]);
    let dz = f(origin[2]) - f(pos[2]);
    let len2 = (dz * dz + dy * dy) + dx * dx;
    len2.sqrt() as f32
}

/// `BlendAreaLights` from `0x6d2f40` and `0x6d2f98`: `fistp(f32(time·2880) − 0.5)`: round to
/// nearest, ties to even, so an exact odd integer comes out one low; NaN and values beyond `i32`
/// give `0x80000000`.
pub fn cloud_phase_index(time: f32) -> i32 {
    let prod_f32 = (f64::from(time) * f64::from(C_2880)) as f32;
    let biased = f64::from(prod_f32) - f64::from(C_HALF);
    fistp(biased)
}

/// `BlendAreaLights` from `0x6d300e` to `0x6d3025`: `f32(1 − (c3c − r14)/(r18 − r14))`.
pub fn cloud_layer_factor(c3c: f32, r14: f32, r18: f32) -> f32 {
    let f = f64::from;
    let frac = (f(c3c) - f(r14)) / (f(r18) - f(r14));
    (f(C_ONE) - frac) as f32
}
