//! The rain effect's intensity setter `Rain::SetIntensity` (`0x6749e0`,
//! `systems/lighting/precipitation.md#rain-rates`) and the patter record setter `SetPatter`
//! (`0x675280`, `systems/lighting/precipitation.md#patter`). Values are `f64` in the register and
//! `f32` at each `fstp dword`.

/// `[0x8680ec]`, the weather scale K at start-up: 0.66 (`0x3f28f5c3`), the value for
/// `weatherDensity` 2. `OnWeatherDensityChanged` (`0x67b870`) rewrites the cell with the entries of
/// `weather_scalars::DENSITY_K_TABLE`, so K is a parameter of `rain_density_param_set`.
pub const RAIN_K: f32 = f32::from_bits(0x3f28_f5c3);
/// `[0x80ffa4]`, 1/6144 (`0x392aaaab`): P times it is the packet fill time.
pub const RAIN_C: f32 = f32::from_bits(0x392a_aaab);
/// `[0x80ffa8]`, P without shaders: 6500.
pub const RAIN_P_FIXEDFUNC: f32 = f32::from_bits(0x45cb_2000);
/// `[0x80ffac]`, P with shaders: 35000.
pub const RAIN_P_SHADER: f32 = f32::from_bits(0x4708_b800);
/// `[0x80ff9c]`, Q without shaders: 18.
pub const RAIN_Q_FIXEDFUNC: f32 = f32::from_bits(0x4190_0000);
/// `[0x80ffa0]`, Q with shaders: 38.
pub const RAIN_Q_SHADER: f32 = f32::from_bits(0x4218_0000);
/// `[0x7ffa24]`, 0.5.
pub const RAIN_S: f32 = f32::from_bits(0x3f00_0000);

/// What `Rain::SetIntensity` writes: the intensity at `+0xd0`, the two rates at `+0xcc` and
/// `+0xb4`, and the mist rate `Mists::SetRate` (`0x67a980`) stores at `+0x3c`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RainDensityParams {
    /// `+0xd0`: the argument's bits, copied as an integer.
    pub intensity_raw: u32,
    /// `+0xcc`: `(K·P)·x`.
    pub rate_cc: f32,
    /// `+0xb4`: `P·(1/6144)`.
    pub rate_b4: f32,
    /// `+0x3c`: `((m + m)·K)·Q` with `m = x − 0.5` unless that is below 0.
    pub ramp_arg: f32,
}

/// `0x6749e0(x)`, `__thiscall`, `ret 4`. `shader_leg` is byte 0 of the effect: non-zero picks
/// `[0x80ffac]` for P and `[0x80ffa0]` for Q, zero `[0x80ffa8]` and `[0x80ff9c]`. `P` stays in the
/// register across the `+0xcc` store. The mist rate takes `x − 0.5` at 53 bits and replaces it with
/// 0 only when it is below 0: `fcomp` of 0 against it and `jne` on `test ah,0x41` keep it when it
/// is above 0, equal to 0 or NaN.
pub fn rain_density_param_set(shader_leg: bool, k: f32, intensity: f32) -> RainDensityParams {
    let f = f64::from;
    let (p, q) = if shader_leg {
        (RAIN_P_SHADER, RAIN_Q_SHADER)
    } else {
        (RAIN_P_FIXEDFUNC, RAIN_Q_FIXEDFUNC)
    };
    let rate_cc = ((f(k) * f(p)) * f(intensity)) as f32;
    let rate_b4 = (f(p) * f(RAIN_C)) as f32;
    let id = f(intensity) - f(RAIN_S);
    let m = if id < 0.0 { 0.0 } else { id };
    let two_m = m + m;
    let ramp_arg = ((two_m * f(k)) * f(q)) as f32;
    RainDensityParams {
        intensity_raw: intensity.to_bits(),
        rate_cc,
        rate_b4,
        ramp_arg,
    }
}

/// `[0x8029b0]`, 0.25 (`0x3e800000`): the lifetime a patter adds to its time.
pub const PATTER_LIFETIME_ADDEND: f32 = f32::from_bits(0x3e80_0000);

/// What `SetPatter` writes into the record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PatterRecord {
    /// `+0x0`, `+0x4`, `+0x8`: the point, copied as integers.
    pub pos: [u32; 3],
    /// `+0x10`: the time through `fld` and `fstp`, which keeps a finite value, an infinity and a
    /// quiet NaN as they are.
    pub val_raw: u32,
    /// `+0x14`: `val + 0.25` at 53 bits, stored as `f32`.
    pub lifetime: f32,
}

/// `0x675280(record, val, point)`, `__cdecl`, `ret 0xc`. `+0xc` is left alone.
pub fn patter_record_init(val: f32, src_pos: [u32; 3]) -> PatterRecord {
    let lifetime = (f64::from(val) + f64::from(PATTER_LIFETIME_ADDEND)) as f32;
    PatterRecord {
        pos: src_pos,
        val_raw: val.to_bits(),
        lifetime,
    }
}
