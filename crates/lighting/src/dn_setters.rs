//! Small functions of the sky and light state: `GetGlareDepthFactor` (`0x6cf800`,
//! `systems/lighting/celestial.md#glare-occlusion`), `SampleCloudCover` (`0x6cfa90`,
//! `systems/lighting/clouds.md#cover-projection`), `BuildCloudCurve` (`0x6d0900`,
//! `clouds.md#cloud-cover`), `UpdateStars` (`0x6d1b50`, `celestial.md#stars`), `SetSpellScreenTint`
//! (`0x6d45c0`, `daynight.md#screen-tint`), `GetNightFraction` (`0x6d4860`,
//! `daynight.md#night-fraction`) and `ScaleFogDistance` (`0x6d6090`,
//! `systems/lighting/tables.md#fog-distances-at-load`). The x87 runs at 53 bits, `fstp dword`
//! stores `f32`, and `__ftol` truncates toward zero (`crate::x87::ftol`).

use crate::dn_bands::dn_track_eval;
use crate::x87::ftol;

/// `[0x801620]`, 0.1.
const C_TENTH: f32 = f32::from_bits(0x3dcc_cccd);
/// `[0x8026c8]`, 1/255.
const C_INV255: f32 = f32::from_bits(0x3b80_8081);
/// `[0x80a8ac]`, 1/256.
const C_INV256: f32 = f32::from_bits(0x3b80_0000);
/// `[0x811618]`, 254.
const C_254: f32 = f32::from_bits(0x437e_0000);
/// `[0x7ffe58]`, 255.
const C_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]`, 512.
const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x7ff9d8]`, 1.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x7ff9d0]`, 1/36.
const C_INV36: f32 = f32::from_bits(0x3ce3_8e39);

/// The low byte of `__ftol`, which the callers keep in `al`.
fn ftol_low_byte(v: f64) -> u8 {
    ftol(v) as u8
}

/// The clamp of `0x6cf800` to [0, 1]: below 0 it is 0 (`fcom` with 0, `test ah,5; jp` falls through
/// only for a negative), and anything not below 1, NaN included, is 1 (`jnp` after the compare with
/// 1 is taken only for a value below it).
fn clamp_unit_f64(v: f64) -> f64 {
    if v < 0.0 {
        0.0
    } else if v < 1.0 {
        v
    } else {
        1.0
    }
}

/// `0x6cf800()`, `__cdecl`, the result in `st0`. `0x672470` writes `[0xc7bc60]` through `ecx` and
/// returns the mode word `[0xc7f288]`. A mode word of exactly 0xf returns 1.0. Otherwise `v =
/// g·0.1` at 53 bits and the result is `1 − clamp(v)`: a negative `v` gives 1, `v` of 1 or more and
/// NaN give 0.
pub fn dn_fade_factor(g_param: f32, mode: u32) -> f32 {
    if mode == 0xf {
        return 1.0;
    }
    let v = f64::from(g_param) * f64::from(C_TENTH);
    (f64::from(C_ONE) - clamp_unit_f64(v)) as f32
}

/// `0x6cfa90(vector)` (`ecx` = the cover object, `ret 8`, the result in `st0`): when `[ecx+0x2c]`
/// is 0 the result is 0. Otherwise `0x6cf870` projects the vector to a column and a row (each
/// `__ftol`), the byte `[ecx+0x44 + (row << [ecx+0x20]) + col]` is read, and the result is the byte
/// times 1/255 at 53 bits. This function takes the gate and the byte.
pub fn dn_sky_tex_sample(gate: bool, looked_up_byte: u8) -> f32 {
    if !gate {
        return 0.0;
    }
    (f64::from(looked_up_byte) * f64::from(C_INV255)) as f32
}

/// `0x6d0900(gamma)` (`ecx` = the cloud object, `ret 4`): stores gamma at `+0x4` and builds the 256
/// bytes at `0xce91d8`. The step is `(255 − b)·(1/256)` stored as `f32`, with `b` the byte at
/// `+0x8`. For each entry, `val = pow(gamma, acc)` (the C runtime `pow`, `0x73f940`), `t = 255 −
/// val·255` at 53 bits, the byte is the low byte of `__ftol(t)`, and `acc` becomes `f32(step +
/// acc)`, so `acc` is the running `f32` sum and not `i·step`. The host `powf` stands in for `pow`;
/// `__ftol` truncation hides its last-bit differences.
pub fn dn_tone_curve(gamma: f32, b: u8) -> [u8; 256] {
    let step = (f64::from(255 - i32::from(b)) * f64::from(C_INV256)) as f32;
    let g = f64::from(gamma);
    let mut acc: f32 = 0.0;
    let mut out = [0u8; 256];
    for slot in out.iter_mut() {
        let val = g.powf(f64::from(acc));
        let t = f64::from(C_255) - val * f64::from(C_255);
        *slot = ftol_low_byte(t);
        acc = (f64::from(step) + f64::from(acc)) as f32;
    }
    out
}

/// What `UpdateStars` writes: the camera triple at `+0xc`, `+0x10` and `+0x14`, and the byte at
/// `+0xb`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CamSkyOut {
    /// `[ecx+0xc]`, a copy of `[0xce9b78]`.
    pub field_c: u32,
    /// `[ecx+0x10]`, a copy of `[0xce9b7c]`.
    pub field_10: u32,
    /// `[ecx+0x14]`, a copy of `[0xce9b80]`.
    pub field_14: u32,
    /// `[ecx+0xb]`.
    pub byte_b: u8,
}

/// `0x6d1b50()` (`ecx` = the stars): copies the camera `[0xce9b78]`, `[0xce9b7c]`, `[0xce9b80]` as
/// integers, then `s = EvalSkyTrack(0xce9a98, 4, [0xce9b64])` and stores the low byte of
/// `__ftol(s·254 + 1)` at `+0xb`.
pub fn dn_cam_sky_setter(track: &[[f32; 2]], cam: [u32; 3], phase: f32) -> CamSkyOut {
    let s = dn_track_eval(track, 4, phase);
    let v = s * f64::from(C_254) + f64::from(C_ONE);
    CamSkyOut {
        field_c: cam[0],
        field_10: cam[1],
        field_14: cam[2],
        byte_b: ftol_low_byte(v),
    }
}

/// What `SetSpellScreenTint` writes: the colour dword at `0xce9ba5` and the byte at `0xce9ba4`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OverrideSkyOut {
    /// `0xce9ba5`: the colour argument.
    pub flag: u32,
    /// `0xce9ba4`: `bits(f32(level·255 + 512)) >> 14`, low byte.
    pub phase_b: u8,
}

/// `0x6d45c0(colour, level)`, `ret 8`: stores the colour at `0xce9ba5` and the byte of `level·255 +
/// 512` (53 bits, stored as `f32`, `shr 0xe`, `cl`) at `0xce9ba4`.
pub fn dn_override_sky_phase(flag: u32, phase: f32) -> OverrideSkyOut {
    let v = (f64::from(phase) * f64::from(C_255) + f64::from(C_512)) as f32;
    OverrideSkyOut {
        flag,
        phase_b: ((v.to_bits() >> 14) & 0xff) as u8,
    }
}

/// `0x6d4860(offset)`, `ret 4`, the result left in `st0`: `EvalSkyTrack(0xce9a34, 4, f32([0xce9b64] +
/// offset))`, the sum taken at 53 bits and stored as the argument.
pub fn dn_track_phase_wrap(track: &[[f32; 2]], master_phase: f32, offset: f32) -> f64 {
    let phase = (f64::from(master_phase) + f64::from(offset)) as f32;
    dn_track_eval(track, 4, phase)
}

/// `0x6d6090(enable)` (`ecx` = the band, `dl` = the flag): with `dl` 0 or a count at `+0x4` of 0 or
/// less nothing changes; otherwise each of the `f32` values from `+0x48` becomes `f32(v·(1/36))`.
pub fn dn_array_scale_36(values: &[f32], enable: bool) -> Vec<f32> {
    if !enable || values.is_empty() {
        return values.to_vec();
    }
    values
        .iter()
        .map(|&v| (f64::from(v) * f64::from(C_INV36)) as f32)
        .collect()
}
