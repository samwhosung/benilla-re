//! The snow and sand intensity setters `Snow::SetIntensity` (`0x6776c0`), `Sand::SetIntensity`
//! (`0x679380`), the rain tail gate `GetRainTailLength` (`0x674bf0`) and the box the three effect
//! constructors stamp (`systems/lighting/precipitation.md#snow`, `#sand`, `#rain-draw` and
//! `#effect-objects`). Values are `f64` in the register and `f32` at each `fstp dword`.
//!
//! An effect's byte 0 is its shader flag: the constructor sets it to 1, then to 0 when the shader
//! resource or the point-sprite capability is absent or `useWeatherShaders` is off. It picks the
//! second constant of each pair below, and the draw leg. The mist rate goes to `Mists::SetRate`
//! (`0x67a980`), which stores it at `+0x3c`.

/// `[0x8680ec]`, the weather scale K at start-up: 0.66 (`0x3f28f5c3`), the value for
/// `weatherDensity` 2 (the CVar's default is `"2"`). `OnWeatherDensityChanged` (`0x67b870`)
/// rewrites the cell from `DENSITY_K_TABLE`, so K is a parameter of each setter.
pub const DENSITY_K: f32 = f32::from_bits(0x3f28_f5c3);

/// The values `0x67b870` stores in `[0x8680ec]` for `weatherDensity` 0 to 3, through the arms
/// `0x67b88c`, `0x67b898`, `0x67b8a4` and `0x67b8b0`: 0.1, 0.33, 0.66 and 1.0.
pub const DENSITY_K_TABLE: [f32; 4] = [
    f32::from_bits(0x3dcc_cccd),
    f32::from_bits(0x3ea8_f5c3),
    f32::from_bits(0x3f28_f5c3),
    f32::from_bits(0x3f80_0000),
];
/// `[0x80ffa4]`, 1/6144 (`0x392aaaab`).
pub const DENSITY_C: f32 = f32::from_bits(0x392a_aaab);
/// `[0x7ffa24]`, 0.5: the snow mist's `x − 0.5` bias and the box's half scale.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x7ffd74]`, 0: the replacement of the `x − 0.5` gate and the windy value of the tail gate.
pub const ZERO: f32 = 0.0;

/// `[0x80ffdc]`, the snow's P without shaders: 1300.
pub const SNOW_P_FIXEDFUNC: f32 = f32::from_bits(0x44a2_8000);
/// `[0x80ffe0]`, the snow's P with shaders: 14000.
pub const SNOW_P_SHADER: f32 = f32::from_bits(0x465a_c000);
/// `[0x80732c]`, the snow's Q without shaders: 24.
pub const SNOW_Q_FIXEDFUNC: f32 = f32::from_bits(0x41c0_0000);
/// `[0x80ffd8]`, the snow's Q with shaders: 48.
pub const SNOW_Q_SHADER: f32 = f32::from_bits(0x4240_0000);

/// What `Snow::SetIntensity` writes: the intensity at `+0xa4`, the rates at `+0xa0` and `+0x94`,
/// and the mist rate at `+0x3c`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnowDensityParams {
    /// `+0xa4`: the argument's bits, copied as an integer.
    pub intensity_raw: u32,
    /// `+0xa0`: `(K·P)·x`.
    pub rate_a0: f32,
    /// `+0x94`: `P·(1/6144)`.
    pub rate_94: f32,
    /// `+0x3c`: `((m + m)·K)·Q` with `m = x − 0.5` unless that is below 0.
    pub ramp_arg: f32,
}

/// `0x6776c0(x)`, `__thiscall`, `ret 4`: the rain setter's steps (`weather_init`) with the snow's
/// offsets and constants. The mist gate keeps `x − 0.5` unless it is below 0 (`fcomp` of 0 against
/// it, `jne` on `test ah,0x41`), so 0 and NaN are kept.
pub fn snow_density_param_set(shader_leg: bool, k: f32, intensity: f32) -> SnowDensityParams {
    let f = f64::from;
    let (p, q) = if shader_leg {
        (SNOW_P_SHADER, SNOW_Q_SHADER)
    } else {
        (SNOW_P_FIXEDFUNC, SNOW_Q_FIXEDFUNC)
    };
    let rate_a0 = ((f(k) * f(p)) * f(intensity)) as f32;
    let rate_94 = (f(p) * f(DENSITY_C)) as f32;
    let id = f(intensity) - f(HALF);
    let m = if id < 0.0 { 0.0 } else { id };
    let two_m = m + m;
    let ramp_arg = ((two_m * f(k)) * f(q)) as f32;
    SnowDensityParams {
        intensity_raw: intensity.to_bits(),
        rate_a0,
        rate_94,
        ramp_arg,
    }
}

/// `[0x81000c]`, the sand's P without shaders: 6000.
pub const SAND_P_FIXEDFUNC: f32 = f32::from_bits(0x45bb_8000);
/// `[0x810010]`, the sand's P with shaders: 32000.
pub const SAND_P_SHADER: f32 = f32::from_bits(0x46fa_0000);
/// `[0x806534]`, the sand's Q without shaders: 32.
pub const SAND_Q_FIXEDFUNC: f32 = f32::from_bits(0x4200_0000);
/// `[0x808de4]`, the sand's Q with shaders: 64.
pub const SAND_Q_SHADER: f32 = f32::from_bits(0x4280_0000);

/// What `Sand::SetIntensity` writes. It stores no copy of the intensity, and its mist rate has no
/// `x − 0.5` gate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SandDensityParams {
    /// `+0x94`: `C·P`.
    pub rate_94: f32,
    /// `+0x9c`: `(K·P)·x`.
    pub rate_9c: f32,
    /// `+0x3c`: `(K·Q)·x`.
    pub ramp_arg: f32,
}

/// `0x679380(x)`, `__thiscall`, `ret 4`. `P` stays in the register across the `+0x94` store and is
/// dropped after the `+0x9c` store; `Q` is loaded for the mist rate.
pub fn sand_density_param_set(shader_leg: bool, k: f32, intensity: f32) -> SandDensityParams {
    let f = f64::from;
    let (p, q) = if shader_leg {
        (SAND_P_SHADER, SAND_Q_SHADER)
    } else {
        (SAND_P_FIXEDFUNC, SAND_Q_FIXEDFUNC)
    };
    let rate_94 = (f(DENSITY_C) * f(p)) as f32;
    let rate_9c = ((f(k) * f(p)) * f(intensity)) as f32;
    let ramp_arg = ((f(k) * f(q)) * f(intensity)) as f32;
    SandDensityParams {
        rate_94,
        rate_9c,
        ramp_arg,
    }
}

/// `[0x801360]`, 0.001 (`0x3a83126f`): the squared wind speed below which the air is calm.
pub const WIND_GATE_EPS: f32 = f32::from_bits(0x3a83_126f);
/// `[0x80ff74]`, 2.0: the tail length returned for calm air.
pub const WIND_GATE_CALM: f32 = f32::from_bits(0x4000_0000);

/// `0x674bf0()`: the wind vector at `[0xc6326c]+0x50`, `+0x54` and `+0x58` gives `m = (y² + z²) +
/// x²` at 53 bits. The result is 2.0, loaded first and left on the stack, when `|m|` is below
/// 0.001, which is `fcomp` then `test ah,5; jnp` (taken only for the below-case, C0 alone); at or
/// above 0.001 and for NaN it drops the 2.0 and returns 0. The result is in `st0`.
pub fn wind_speed_gate(x: f32, y: f32, z: f32) -> f32 {
    let f = f64::from;
    let mag2 = (f(y) * f(y) + f(z) * f(z)) + f(x) * f(x);
    if mag2.abs() < f(WIND_GATE_EPS) {
        WIND_GATE_CALM
    } else {
        ZERO
    }
}

/// The box the rain (`0x674620`), snow (`0x677420`) and sand (`0x679140`) constructors stamp at
/// `+0x4` to `+0x18`, from the same instructions in each: `a = f32(arg0·0.5)` and `b =
/// f32(arg1·0.5)`, stored as the corners `(−a, −a, −b)` and `(a, a, b)`. Returns `[−a, −a, −b, a,
/// a, b]`.
pub fn weather_effect_halfbox(arg0: f32, arg1: f32) -> [f32; 6] {
    let a = (f64::from(arg0) * f64::from(HALF)) as f32;
    let b = (f64::from(arg1) * f64::from(HALF)) as f32;
    [-a, -a, -b, a, a, b]
}
