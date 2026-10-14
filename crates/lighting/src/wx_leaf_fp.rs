//! Float leaves of the rain, snow and sand drivers and the ground grid recentre: the emit count and
//! phase (`0x6754a0`, `0x677df0`, `0x679930`, `systems/lighting/precipitation.md#emission-budget`),
//! the rain drop start and patter (`0x6754a0`, `precipitation.md#drop-landing`), the snow patter
//! value (`0x677df0`), the frame quota and interval of the updates (`0x6756a0`, `0x679b00`,
//! `precipitation.md#particle-packets`), the adapter's fallback height (`0x67c7f0`) and the tile of
//! `0x67ce50` (`systems/lighting/weather.md#ground-grid`).
//!
//! Each runs the exe's instructions of one stretch of its function. Values are f64 on the x87 stack
//! and round to f32 at each `fstp dword`; a value stored with `fst` is rounded in its slot while
//! st0 stays wide.

use crate::x87::{fistp, ftol_floor};

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x8680e8]` = 0.5: subtracted from the emit count before the `fistp`.
pub const ROUND_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x7ff9d8]` = 1.0: subtracted from the `[1, 2)` draw.
pub const ONE: f32 = 1.0;
/// `[0x80ff74]` = 2.0: the numerator of the rain drop's `t` reciprocal.
pub const TWO: f32 = f32::from_bits(0x4000_0000);
/// `[0x801628]` = 2.0: the rain wind gate on the squared length.
pub const WIND_MAG2_GATE: f32 = f32::from_bits(0x4000_0000);
/// `[0x8029b0]` = 0.25: added to the snow patter value.
pub const SNOW_PATTER_BIAS: f32 = f32::from_bits(0x3e80_0000);
/// `[0x7ffd74]` = 0.0.
pub const ZERO: f32 = 0.0;
/// `[0x80ffcc]` = `0x3c88893b` (1/60): the cap on the frame time of the rain and sand updates.
pub const DT_CAP: f32 = f32::from_bits(0x3c88_893b);
/// `[0x80ffc8]` = 6144.0: the numerator of the update interval.
pub const INTERVAL_NUM: f32 = f32::from_bits(0x45c0_0000);
/// `[0x810008]` = 200.0: `0x67c7f0` subtracts it from z when the ground query finds nothing.
pub const HEIGHT_BIAS: f32 = f32::from_bits(0x4348_0000);
/// `[0x80ff90]` = 0.03: the tile scale of `0x67ce50`.
pub const TILE_SCALE: f32 = f32::from_bits(0x3cf5_c28f);

/// The emit count of the rain (`0x6754cc` to `0x6754f2`), snow (`0x677e1c` to `0x677e42`) and sand
/// (`0x67995c` to `0x679982`) emitters (`systems/lighting/precipitation.md#emission-budget`): `fild
/// qword slots; fcom requested; test ah,5; jp` takes `slots` (stored as f32, exact) only when
/// `slots < requested`; equal, greater and NaN keep `requested`. The count is `fistp(count - 0.5)`:
/// ties to even, so an exact odd count comes out one low, and `0x80000000` for NaN or out of range.
/// The caller skips the loop when the count is 0 or less.
#[must_use]
pub fn emit_count_clamp(slots: i64, requested: f32) -> i32 {
    let count = if (slots as f64) < f(requested) {
        slots as f64
    } else {
        f(requested)
    };
    fistp(count - f(ROUND_HALF))
}

/// The phase argument of a spawn at the three emitters (rain `0x67551c`, snow `0x677f21`, sand
/// `0x679a5c`): `((m - 1.0) * g + phase)` wide, rounded at the `fstp dword [esp]`; `m` is the `[1,
/// 2)` draw, `g` the frame time `[0xc62510]`.
#[must_use]
pub fn emit_phase_inc(m: f32, g: f32, phase: f32) -> f32 {
    ((f(m) - f(ONE)) * f(g) + f(phase)) as f32
}

/// The phase after the loop (rain `0x675686`, snow `0x677fd0`, sand `0x679ae5`): `g + phase`
/// rounded at the `fstp dword [phase]`.
#[must_use]
pub fn emit_phase_advance(phase: f32, g: f32) -> f32 {
    (f(g) + f(phase)) as f32
}

/// The start parameter of a rain drop (`0x675548` to `0x67555c`): `-(2.0 / drop[+0x14]) +
/// drop[+0x20]`, the quotient wide, rounded at the `fstp dword`.
#[must_use]
pub fn rain_drop_t(field14: f32, field20: f32) -> f32 {
    (-(f(TWO) / f(field14)) + f(field20)) as f32
}

/// The rain wind's squared length (`0x6755a9` to `0x6755c0`), `(x² + y²) + z²` of the manager's
/// `+0x5c`, `+0x60`, `+0x64`, wide and compared unrounded.
#[must_use]
pub fn rain_wind_mag2(x: f32, y: f32, z: f32) -> f64 {
    (f(x) * f(x) + f(y) * f(y)) + f(z) * f(z)
}

/// The rain patter gate (`0x6755c2` to `0x6755d3`): `fcomp 2.0; test ah,0x41; je` skips the patter
/// only when the squared length is above 2.0, so a length of 2.0 or below and NaN emit it.
#[must_use]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn rain_wind_emits_patter(mag2: f64) -> bool {
    !(mag2 > f(WIND_MAG2_GATE))
}

/// A rain drop's fields that the patter reads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RainDrop {
    /// `+0x00` to `+0x08`, the position.
    pub pos: [f32; 3],
    /// `+0x0c` to `+0x14`, the direction.
    pub dir: [f32; 3],
    /// `+0x1c`, the time at the start of the drop's life.
    pub f1c: f32,
    /// `+0x20`, the time at its end.
    pub f20: f32,
}

/// The rain patter point (`0x6755d9` to `0x67562c`): `delta = f20 - f1c` wide, then `delta * dir +
/// pos` per axis. The x product stays wide through the add; the y and z products are stored as f32
/// (`fstp dword`) before theirs, so those two round twice.
#[must_use]
pub fn rain_patter_vec(drop: &RainDrop) -> [f32; 3] {
    let delta = f(drop.f20) - f(drop.f1c);
    let out_x = (delta * f(drop.dir[0]) + f(drop.pos[0])) as f32;
    let prod_y = (delta * f(drop.dir[1])) as f32;
    let prod_z = (delta * f(drop.dir[2])) as f32;
    let out_y = (f(prod_y) + f(drop.pos[1])) as f32;
    let out_z = (f(prod_z) + f(drop.pos[2])) as f32;
    [out_x, out_y, out_z]
}

/// The snow patter value (`0x677f61` to `0x677f88`): `fcomp 0.0; test ah,0x41; jne` loads 0.0 for
/// `field <= 0` and NaN, otherwise `field`; plus 0.25, rounded at the store.
#[must_use]
pub fn snow_patter_value(field1c: f32) -> f32 {
    let v = if field1c > 0.0 { f(field1c) } else { f(ZERO) };
    (v + f(SNOW_PATTER_BIAS)) as f32
}

/// The frame's spawn quota of the rain (`0x675863`), snow (`0x67846f`) and sand (`0x679c77`)
/// updates: `min(g, 1/60) * field`, `fcomp; test ah,5; jp` taking `g` only when `g < 1/60` (NaN
/// takes the cap), rounded at the `fst dword`. Nothing carries over between frames.
#[must_use]
pub fn update_frame_quota(g: f32, field: f32) -> f32 {
    let clamped = if f(g) < f(DT_CAP) { f(g) } else { f(DT_CAP) };
    (clamped * f(field)) as f32
}

/// The update interval of the rain (`0x675944`, `0x675966`) and sand (`0x679da8`, `0x679dca`):
/// `6144.0 / field`, rounded at the `fstp dword`.
#[must_use]
pub fn update_interval_div(field: f32) -> f32 {
    (f(INTERVAL_NUM) / f(field)) as f32
}

/// `0x67c7f0` GetWeatherGroundHeight (`systems/lighting/weather.md#ground-grid`), the leg where the
/// ground query `0x6b7070` finds nothing: `world.z - 200.0` rounded at the return.
#[must_use]
pub fn height_sample_bias(world_z: f32) -> f32 {
    (f(world_z) - f(HEIGHT_BIAS)) as f32
}

/// `0x67ce50` RecenterWeatherGrid (`systems/lighting/weather.md#ground-grid`) from `0x67ce56` to
/// `0x67ce96`: the tile of a point. `0.03 * y` is stored as f32 (`fstp dword [ebp+8]`) and `0.03 *
/// x` is passed on as a double; each is floored (`0x73ff3f`, then `0x40a2b0`).
#[must_use]
pub fn recenter_tile_coords(world_x: f32, world_y: f32) -> (i32, i32) {
    let tx = ftol_floor(f(TILE_SCALE) * f(world_x));
    let ty = ftol_floor(f((f(TILE_SCALE) * f(world_y)) as f32));
    (tx, ty)
}
