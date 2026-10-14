//! The lighting a map entity is drawn with: `ApplyMapLighting` (`0x6a7300`), `AnimateMapEntityLight`
//! (`0x69e770`) and the fade transform of `DrawDetailDoodadBatch` (`0x6b2b80`).
//!
//! A value kept in the register stays at 53 bits; one stored with `fstp dword` or `fst dword` is
//! `f32` from then on, and `fst` leaves the register unrounded.

use crate::colornorm::{color_normalize, INV_255};

/// `ApplyMapLighting` (`0x6a7300`, ecx = this, the target on the stack) with `[this+0xc] & 4` set,
/// `0x6a7349` to `0x6a738b`: the light block's (`[0xc89ef0]`) `+0xb8`, `+0xbc` and `+0xc0` each times
/// `[this+0xa4]`, loaded once and kept in the register (`fmul st,st(1)`), each product stored as
/// `f32`.
pub fn exterior_scaled_directional(intensity: f32, comps: [f32; 3]) -> [f32; 3] {
    let i = f64::from(intensity);
    [
        (f64::from(comps[0]) * i) as f32,
        (f64::from(comps[1]) * i) as f32,
        (f64::from(comps[2]) * i) as f32,
    ]
}

/// `0x6a7300` with the flag clear, `0x6a73bd` to `0x6a743f`: the bytes `+0xa2`, `+0xa1` and `+0xa0`
/// each `byte·(1/255)` times `[this+0xa4]`. The first two products stay in the register; the third
/// is stored to `[ebp-0x2c]` at `0x6a73ff` and reloaded at `0x6a7427`, so it rounds to `f32` first.
pub fn area_directional_color_scaled(bytes: [u8; 3], intensity: f32) -> [f32; 3] {
    let i = f64::from(intensity);
    let norm_live = |b: u8| f64::from(b) * f64::from(INV_255);
    let norm_stored = |b: u8| f64::from(color_normalize(b));
    [
        (norm_live(bytes[0]) * i) as f32,
        (norm_live(bytes[1]) * i) as f32,
        (norm_stored(bytes[2]) * i) as f32,
    ]
}

/// `0x6a7300` when `[this+0x90] & 0x800`, `0x6a7454` to `0x6a74a3`: `(dn_k − global_k)·f32(a3·(1/255))`,
/// the direction at `+0x168` of the object `0x6d48b0` returns less the globals `0xca816c` to
/// `0xca8174`, each difference in the register, times the stored byte `+0xa3` normalized.
pub fn daynight_dir_diff_scaled(dn: [f32; 3], global: [f32; 3], a3: u8) -> [f32; 3] {
    let c = f64::from(color_normalize(a3));
    [
        ((f64::from(dn[0]) - f64::from(global[0])) * c) as f32,
        ((f64::from(dn[1]) - f64::from(global[1])) * c) as f32,
        ((f64::from(dn[2]) - f64::from(global[2])) * c) as f32,
    ]
}

/// `0x6a74a8` to `0x6a74cb`: each of those products plus its global, stored as `f32`.
pub fn daynight_dir_blend_add(diff_scaled: [f32; 3], global: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(diff_scaled[0]) + f64::from(global[0])) as f32,
        (f64::from(diff_scaled[1]) + f64::from(global[1])) as f32,
        (f64::from(diff_scaled[2]) + f64::from(global[2])) as f32,
    ]
}

/// `0x6a74d0` to `0x6a7518`: the blended direction normalized, `1/√((v0² + v1²) + v2²)` kept in the
/// register (`fdivr [0x7ff9d8]` at `0x6a74fa`) and each component times it.
pub fn daynight_dir_normalize(v: [f32; 3]) -> [f32; 3] {
    let sq0 = f64::from(v[0]) * f64::from(v[0]);
    let sq1 = f64::from(v[1]) * f64::from(v[1]);
    let sq2 = f64::from(v[2]) * f64::from(v[2]);
    let recip = 1.0_f64 / ((sq0 + sq1) + sq2).sqrt();
    [
        (f64::from(v[0]) * recip) as f32,
        (f64::from(v[1]) * recip) as f32,
        (f64::from(v[2]) * recip) as f32,
    ]
}

/// `0x6a754b` to `0x6a75a1`: the fog colour passed to `0x71c110`, the bytes `+0x82`, `+0x81` and
/// `+0x80` of the object `0x6d48b0` returns.
pub fn fog_override_color(bytes: [u8; 3]) -> [f32; 3] {
    bytes.map(color_normalize)
}

/// `0x6a75a6` to `0x6a75ee`: the ambient colour passed to `0x71bc70`, the bytes `+0x9e`, `+0x9d` and
/// `+0x9c`.
pub fn area_ambient_color(bytes: [u8; 3]) -> [f32; 3] {
    bytes.map(color_normalize)
}

/// `AnimateMapEntityLight` (`0x69e770`) from `0x69e776` to `0x69e795`: the step its colour bytes move
/// by, `__ftol` (`0x40a2b0`, towards zero) of `(2.0·dt)·255.0` at 53 bits, at least 1; `dt` is
/// `[0xc62510]`.
pub fn color_step_magnitude(dt: f32) -> i32 {
    let prod = 2.0_f64 * f64::from(dt) * 255.0_f64;
    let step = crate::x87::ftol(prod);
    step.max(1)
}

/// Which way `0x69e770` moved the intensity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntensityRamp {
    /// At the target; unchanged.
    Hold,
    /// Below the target; raised.
    Up,
    /// Above the target, or NaN; lowered.
    Down,
}

/// `0x69e770` from `0x69e880` to `0x69e906`: the intensity `[this+0xa4]` moved toward the target
/// `[this+0xf8]` by `3.3333333·dt` (`[0x810808]`). With `d = i − t` against 0.0: equal holds (`test
/// ah,0x44; jnp` at `0x69e8a5`); below raises to `delta + i` (`0x69e8b4`); above or NaN lowers to
/// `i − delta` (`jp` at `0x69e8b2`). The result is stored with `fst` and the unrounded value compared
/// with the target: a rise past it (`test ah,0x41; jne`) or a fall below it (`test ah,5; jp`) stores
/// the target instead.
pub fn intensity_ramp(i: f32, t: f32, dt: f32) -> (IntensityRamp, f32) {
    let delta = f64::from(f32::from_bits(0x4055_5555)) * f64::from(dt);
    let fi = f64::from(i);
    let ft = f64::from(t);
    let d = fi - ft;
    let c2 = d.is_nan();
    let c3 = (d == 0.0) || c2;
    let c0 = (d < 0.0) || c2;

    // `jnp` jumps on an odd count of C3 and C2: equal alone
    if (u8::from(c3) + u8::from(c2)) % 2 == 1 {
        return (IntensityRamp::Hold, i);
    }

    // `jp` jumps to the fall on an even count of C0 and C2: above, or NaN
    if (u8::from(c0) + u8::from(c2)) % 2 == 0 {
        let ramped = fi - delta;
        let rc2 = ramped.is_nan() || ft.is_nan();
        let rc0 = (ramped < ft) || rc2;
        // `jp` skips the clamp on an even count: at or above the target, or NaN
        if (u8::from(rc0) + u8::from(rc2)) % 2 == 0 {
            (IntensityRamp::Down, ramped as f32)
        } else {
            (IntensityRamp::Down, t)
        }
    } else {
        let ramped = delta + fi;
        // `jne` skips the clamp when C0 or C3 is set: at or below the target, or NaN
        let skip = ramped <= ft || ramped.is_nan() || ft.is_nan();
        if skip {
            (IntensityRamp::Up, ramped as f32)
        } else {
            (IntensityRamp::Up, t)
        }
    }
}

/// `DrawDetailDoodadBatch` (`0x6b2b80`) from `0x6b2c24` to `0x6b2caa`: the scale it passes to
/// `0x7bdd00`, `1/(70.0·0.25)` with 70.0 at `[0x867958]`, stored as `f32`.
pub fn detail_fade_scale() -> f32 {
    let dist = f64::from(70.0_f32) * f64::from(0.25_f32);
    (1.0_f64 / dist) as f32
}

/// `0x6b2cb2` to `0x6b2cc8`: the angle it passes to `0x7bdd60`, `π·0.5` with π at `[0x810c10]`
/// (`0x40490fdb`).
pub fn detail_rot_y() -> f32 {
    (f64::from(f32::from_bits(0x4049_0fdb)) * f64::from(0.5_f32)) as f32
}

/// `0x6b2ce5` to `0x6b2cf8`: the offset it passes to `0x7bdc40`, `70.0·−0.75` (`[0x810c68]`).
pub fn detail_translate() -> f32 {
    (f64::from(70.0_f32) * f64::from(f32::from_bits(0xbf40_0000))) as f32
}
