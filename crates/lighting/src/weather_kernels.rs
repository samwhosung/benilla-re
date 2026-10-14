//! Small arithmetic leaves of the weather code: the particle clock (`0x6745d0`, `0x674600`,
//! `systems/lighting/precipitation.md#particle-clock`), the clamped lerp and the intensity ramp
//! (`0x674ba0`, `0x67bc70`, `systems/lighting/weather.md#weather-ramps`), the fade ramp
//! (`0x67ad60`, `0x67ad90`, `0x67add0`, `precipitation.md#mist`) and the rain tilt matrix
//! (`0x674a70`, `precipitation.md#rain-draw`).
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`; a value left in st0 by a
//! function stays wide for its caller.

use benilla_re_cmath::multiply::multiply_matrix3 as mat3_mul;
use benilla_re_cmath::rotation::axis_angle_mat3;

use crate::x87::fistp;

/// `[0x7ffd68]` = 1024.0: ticks per second.
pub const FIXED_QUANTIZE_SCALE: f32 = 1024.0;
/// `[0x8680e8]` = 0.5: subtracted before the rounding.
pub const FIXED_QUANTIZE_BIAS: f32 = 0.5;

/// `0x6745d0` SecondsToWeatherTicks (`systems/lighting/precipitation.md#particle-clock`), the whole
/// function: `x * 1024` is stored as f32 (`fstp [ebp+8]`), `- 0.5` is taken wide, and `fistp`
/// rounds to nearest with ties to even and gives `0x80000000` for NaN and out of range. So an exact
/// odd integer (`2k + 1` ticks) minus 0.5 is a tie that rounds to the even `2k`, one low.
#[must_use]
pub fn weather_fixed_quantize(x: f32) -> i32 {
    let scaled = (f64::from(x) * f64::from(FIXED_QUANTIZE_SCALE)) as f32;
    fistp(f64::from(scaled) - f64::from(FIXED_QUANTIZE_BIAS))
}

/// `[0x8090d8]` = 1/1024: seconds per tick.
pub const TICK_SCALE: f32 = 0.0009765625;

/// `0x674600` WeatherTicksToSeconds (`systems/lighting/precipitation.md#particle-clock`), the whole
/// function: the tick count in `ecx` is stored with a zero high dword and loaded with `fild qword`
/// (so it is unsigned), times 1/1024, left wide in st0.
#[must_use]
pub fn weather_tick_scale(ticks: u32) -> f32 {
    (f64::from(ticks) * f64::from(TICK_SCALE)) as f32
}

/// `0x674ba0` LerpClamped (`systems/lighting/weather.md#weather-ramps`), the whole function: `(b -
/// a) * tc + a` wide with `tc` the clamped `t`. `fcom 0.0; test ah,5; jp` takes the lerp with `tc =
/// 0` only for `t < 0` (-0 and NaN go on); then `fcom 1.0; test ah,5; jnp` keeps `t` only below 1,
/// so 1, above 1 and NaN give `tc = 1.0`.
#[must_use]
pub fn weather_clamped_lerp(a: f32, b: f32, t: f32) -> f32 {
    weather_clamped_lerp_f64(a, b, t) as f32
}

/// [`weather_clamped_lerp`] as it leaves st0, before a caller's store rounds it.
fn weather_clamped_lerp_f64(a: f32, b: f32, t: f32) -> f64 {
    let tc = clamp_unit(t);
    (f64::from(b) - f64::from(a)) * f64::from(tc) + f64::from(a)
}

/// The clamp of `0x674ba0` on the f32 argument: below 0 gives 0, NaN and above 1 give 1.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn clamp_unit(t: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if !(t <= 1.0) {
        1.0
    } else {
        t
    }
}

/// The same clamp on the wide st0 value of the ramp evaluators.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn clamp_unit_f64(v: f64) -> f64 {
    if v < 0.0 {
        0.0
    } else if !(v <= 1.0) {
        1.0
    } else {
        v
    }
}

/// The two dwords of a fade ramp, written by `0x67ad60` to `ecx`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RampCoeffs {
    /// `+0`.
    pub slope: f32,
    /// `+4`.
    pub intercept: f32,
}

/// `[0x8030a0]` = -1.0.
pub const RAMP_SETUP_NEG_ONE: f32 = -1.0;
/// `[0x7ff9d8]` = 1.0.
pub const RAMP_ONE: f32 = 1.0;
/// `[0x7ffd74]` = 0.0.
pub const RAMP_ZERO: f32 = 0.0;

/// `0x67ad60` SetFadeRamp (`systems/lighting/precipitation.md#mist`), the whole function: with
/// `recip = 1 / (hi - lo)` wide, the slope `-1.0 * recip` and the intercept `recip * hi` are each
/// rounded at their store; the intercept multiplies the wide `recip`, not the stored slope.
#[must_use]
pub fn weather_ramp_setup(lo: f32, hi: f32) -> RampCoeffs {
    let recip = f64::from(RAMP_ONE) / (f64::from(hi) - f64::from(lo));
    let slope = (f64::from(RAMP_SETUP_NEG_ONE) * recip) as f32;
    let intercept = (recip * f64::from(hi)) as f32;
    RampCoeffs { slope, intercept }
}

/// `0x67ad90` EvalFadeRamp (`systems/lighting/precipitation.md#mist`), the whole function: `v = x *
/// slope + intercept` wide, clamped as in [`weather_clamped_lerp`] (below 0 gives 0, NaN and 1 or
/// more give 1).
#[must_use]
pub fn weather_ramp_eval(c: RampCoeffs, x: f32) -> f32 {
    let v = f64::from(x) * f64::from(c.slope) + f64::from(c.intercept);
    clamp_unit_f64(v) as f32
}

/// `0x67add0` EvalFadeRampInverse (`systems/lighting/precipitation.md#mist`), the whole function:
/// `1.0 - clamp(v)` with the clamp and `v` of [`weather_ramp_eval`].
#[must_use]
pub fn weather_ramp_eval_inv(c: RampCoeffs, x: f32) -> f32 {
    let v = f64::from(x) * f64::from(c.slope) + f64::from(c.intercept);
    (f64::from(RAMP_ONE) - clamp_unit_f64(v)) as f32
}

/// The wind vector of the weather manager (`[0xc6326c]` at `+0x50`) that `0x674a70` reads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeatherWind {
    /// `+0x50`.
    pub x: f32,
    /// `+0x54`.
    pub y: f32,
    /// `+0x58`, read and not used.
    pub z: f32,
}

/// What `0x674a70` leaves in its matrix argument.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindTiltResult {
    /// The 3 by 3 matrix: `R * M_in` when the wind tilts it, `M_in` otherwise.
    pub matrix: [f32; 9],
    /// Whether the tilt was applied.
    pub applied: bool,
}

/// `[0x8680f0]` = 30.0: the wind speed that reads as one degree of tilt per unit.
pub const WIND_TILT_ANGLE_DIV: f32 = 30.0;
/// `[0x8680f4]` = 45.0: the largest tilt in degrees.
pub const WIND_TILT_ANGLE_MAX: f32 = 45.0;
/// `[0x801360]` = `0x3a83126f` (0.001): the gate on the squared lengths.
pub const WIND_TILT_EPS: f32 = 0.001;
/// `[0x7ff9d8]` = 1.0.
pub const WIND_TILT_ONE: f32 = 1.0;
/// `[0x7ffd74]` = 0.0.
pub const WIND_TILT_ZERO: f32 = 0.0;
/// `[0x7ffaac]` = `0x3c8efa35`: radians per degree.
pub const WIND_TILT_DEG2RAD: f32 = 0.017_453_292;

/// `0x674a70` BuildRainTiltMatrix (`systems/lighting/precipitation.md#rain-draw`), the whole
/// function with `0x7be730` (the axis-angle matrix times `M_in`, `cmath`) run as its two parts.
/// `mag2 = y² + x²` is wide and `fst`-stored as f32 for the gates and the reciprocal; the tilt is
/// `weather_clamped_lerp(0, 45, f32(sqrt(mag2 wide) / 30))`. `fabs; fcomp 0.001; test ah,5; jnp`
/// returns `M_in` unchanged only for `|mag2| < 0.001`; NaN goes on. The axis is built from `inv = 1
/// / sqrt(mag2)` as `(y * inv - z2, z2 - x * inv)` stored as f32, with `z2 = (inv * 0) * 0`, and a
/// third component `w = x * inv * 0 - y * inv * 0` that stays wide; the second gate is the same
/// test on `(ax0² + ax1²) + w²` of the stored values, and the axis is divided by the square root of
/// that sum (`ax0`, `ax1` and `w` times `1 / sqrt(sum)`, each stored). The matrix is
/// `axis_angle_mat3(axis, f32(tilt * 0.017453292), normalized)` times `M_in`.
#[must_use]
pub fn weather_wind_tilt(wind: WeatherWind, m_in: &[f32; 9]) -> WindTiltResult {
    let f = f64::from;
    let unchanged = WindTiltResult {
        matrix: *m_in,
        applied: false,
    };

    let mag2_reg = f(wind.y) * f(wind.y) + f(wind.x) * f(wind.x);
    let mag2_f32 = mag2_reg as f32;

    let ratio = (mag2_reg.sqrt() / f(WIND_TILT_ANGLE_DIV)) as f32;
    let tilt_deg = weather_clamped_lerp(0.0, WIND_TILT_ANGLE_MAX, ratio);

    if mag2_f32.abs() < WIND_TILT_EPS {
        return unchanged;
    }

    let inv = f(WIND_TILT_ONE) / f(mag2_f32).sqrt();
    let x_inv = f(wind.x) * inv;
    let y_inv = f(wind.y) * inv;
    let z2 = inv * f(WIND_TILT_ZERO) * f(WIND_TILT_ZERO);
    let ax0 = (y_inv - z2) as f32;
    let ax1 = (z2 - x_inv) as f32;
    let w = x_inv * f(WIND_TILT_ZERO) - y_inv * f(WIND_TILT_ZERO);

    let sumsq = f(ax0) * f(ax0) + f(ax1) * f(ax1) + w * w;
    if sumsq.abs() < f(WIND_TILT_EPS) {
        return unchanged;
    }
    let invn = f(WIND_TILT_ONE) / sumsq.sqrt();
    let axis = [
        (f(ax0) * invn) as f32,
        (f(ax1) * invn) as f32,
        (w * invn) as f32,
    ];

    let angle_rad = (f(tilt_deg) * f(WIND_TILT_DEG2RAD)) as f32;
    let rot = axis_angle_mat3(&axis, angle_rad, true);
    let matrix = mat3_mul(&rot, m_in);
    WindTiltResult {
        matrix,
        applied: true,
    }
}

/// The fields of the weather manager that `0x67bc70` reads and writes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeatherManagerIntensity {
    /// `+0x08`, the grade.
    pub chan_a_out: f32,
    /// `+0x0c`, the density ramp's start.
    pub chan_b_from: f32,
    /// `+0x10`, the density ramp's end.
    pub chan_b_to: f32,
    /// `+0x14`, the density.
    pub chan_b_out: f32,
    /// `+0x18`, the millisecond tick the grade ramp started.
    pub start_a: u32,
    /// `+0x1c`, the tick the density ramp started.
    pub start_b: u32,
    /// `+0x28`, the rain effect; nonzero gets the effect argument.
    pub effect_rain: u32,
    /// `+0x2c`, the snow effect.
    pub effect_snow: u32,
    /// `+0x30`, the sand effect.
    pub effect_sand: u32,
}

/// What one `0x67bc70` tick produces.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntensityRampResult {
    /// The grade stored at `+0x08`.
    pub chan_a_out: f32,
    /// The density stored at `+0x14` and at the light state's `+0x40`.
    pub chan_b_out: f32,
    /// The argument `ebx` handed to a live effect.
    pub effect_arg: f32,
    /// The light state's `+0x40`, equal to `chan_b_out`.
    pub dnstate_intensity: f32,
}

/// `[0x801360]` = 0.001: the seconds per millisecond and the bias of the span.
pub const INTENSITY_TIME_SCALE: f32 = 0.001;
/// `[0x801624]` = 10.0: seconds a unit span takes.
pub const INTENSITY_DEN_SCALE: f32 = 10.0;
/// `[0x8029b0]` = 0.25: the grade below which an effect gets 0.
pub const INTENSITY_ADJ_BIAS: f32 = 0.25;
/// `[0x80b058]` = `0x3faaaaab`: 4/3.
pub const INTENSITY_ADJ_GAIN: f32 = 1.333_333_4;
/// `[0x80306c]` = 4.0: the density span's factor.
pub const INTENSITY_CHAN_B_SPAN: f32 = 4.0;

/// `0x67bc70` RampWeather (`systems/lighting/weather.md#weather-ramps`) at tick `now` (`0x42c010`)
/// with the grade ramp `arg_from` to `arg_to`. The elapsed milliseconds are the unsigned `now -
/// start` (`fild qword`, high dword 0) times 0.001, divided by `|span + 0.001| * 10` wide, the
/// quotient stored as f32 for `0x674ba0`. The grade is the lerp from `arg_from` to `arg_to`, stored
/// as f32 at `+0x08` while the wide value goes on: `adj = (g - 0.25) * 4/3`, and `fld 0.0; fcomp
/// adj; test ah,0x41; jne` keeps `adj` unless it is below 0 (NaN is kept), so the effect argument
/// is `adj` as f32 or 0. The density runs the same way from `+0x10` to `+0x0c` (the arguments are
/// pushed in that order) with the span `(from - to) * 4`.
#[must_use]
pub fn weather_intensity_ramp(
    mgr: &WeatherManagerIntensity,
    now: u32,
    arg_from: f32,
    arg_to: f32,
) -> IntensityRampResult {
    let delta_a = now.wrapping_sub(mgr.start_a);
    let num_a = f64::from(delta_a) * f64::from(INTENSITY_TIME_SCALE);
    let den_a = ((f64::from(arg_to) - f64::from(arg_from)) + f64::from(INTENSITY_TIME_SCALE)).abs()
        * f64::from(INTENSITY_DEN_SCALE);
    let t_a = (num_a / den_a) as f32;
    let lerp_a = weather_clamped_lerp_f64(arg_from, arg_to, t_a);
    let chan_a_out = lerp_a as f32;

    let adj = (lerp_a - f64::from(INTENSITY_ADJ_BIAS)) * f64::from(INTENSITY_ADJ_GAIN);
    let effect_arg = if adj < 0.0 { 0.0 } else { adj as f32 };

    let delta_b = now.wrapping_sub(mgr.start_b);
    let num_b = f64::from(delta_b) * f64::from(INTENSITY_TIME_SCALE);
    let den_b = ((f64::from(mgr.chan_b_from) - f64::from(mgr.chan_b_to))
        * f64::from(INTENSITY_CHAN_B_SPAN)
        + f64::from(INTENSITY_TIME_SCALE))
    .abs()
        * f64::from(INTENSITY_DEN_SCALE);
    let t_b = (num_b / den_b) as f32;
    let chan_b_out = weather_clamped_lerp(mgr.chan_b_to, mgr.chan_b_from, t_b);

    IntensityRampResult {
        chan_a_out,
        chan_b_out,
        effect_arg,
        dnstate_intensity: chan_b_out,
    }
}
