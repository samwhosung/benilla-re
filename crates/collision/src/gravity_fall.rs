//! Gravity and falling (`systems/collision/falling.md`): the fall speed at a time, the distance
//! fallen, the drop since the fall began and the height above the apex. Each function is
//! `__thiscall` on the mover and reads the fields `Cm` holds.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x81da58]`: g, 19.291105 (`0x419a542f`).
const G: f64 = f32::from_bits(0x419a542f) as f64;

/// `[0x81da60]`: g/2, 9.645553.
const HALF_G: f64 = f32::from_bits(0x411a542f) as f64;

/// `[0x80e020]`: 1/g, 0.0518374.
const INV_G: f64 = f32::from_bits(0x3d54536a) as f64;

/// `[0x81da6c]`: 1/(2g), 0.0259187.
const INV_2G: f64 = f32::from_bits(0x3cd4536a) as f64;

/// `[0x81da70]`: −2^-22, the speed below which the apex term is added.
const NEG_EPS: f64 = f32::from_bits(0xb4800000) as f64;

/// `[0x801360]`: 0.001.
const MS_TO_S: f64 = f32::from_bits(0x3a83126f) as f64;

/// The mover fields gravity reads: flags `+0x40`, the fall time in milliseconds `+0x78`, the fall
/// speed `+0xa0`, the start height `+0x18`, `+0x7c`, and the two terminal speeds the code reads
/// from `.data` globals, chosen by flags & 0x20000000.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cm {
    pub flags: u32,
    pub fall_time_ms: u32,
    pub fall_vel: f32,
    pub field_18: f32,
    pub field_7c: f32,
    pub terminal_land: f32,
    pub terminal_swim: f32,
}

impl Cm {
    #[inline]
    fn terminal(&self) -> f64 {
        if self.flags & 0x2000_0000 != 0 {
            f(self.terminal_swim)
        } else {
            f(self.terminal_land)
        }
    }
}

/// Milliseconds to seconds, rounded to `f32`: `fild qword; fmul 0.001; fstp dword`.
#[inline]
fn ms_to_dt(ms: u32) -> f32 {
    (f64::from(ms) * MS_TO_S) as f32
}

/// `CMovement::GetFallSpeed` (`0x7c5d20`, `systems/collision/falling.md#fall-speed-at-a-time`): the
/// fall speed capped at the terminal speed (only when strictly above it), plus `dt·g`, stored as
/// `f32`; the terminal speed when that exceeds it.
pub fn gravity_integrate(cm: &Cm, dt: f32) -> f32 {
    let t = cm.terminal();
    let v = f(cm.fall_vel);
    let a = if v > t { t } else { v };
    let new = (f(dt) * G + a) as f32;
    if t < f(new) {
        t as f32
    } else {
        new
    }
}

/// `CMovement::GetCurrentFallSpeed` (`0x7c5d70`): `0x7c5d20` at the fall time `+0x78`.
pub fn gravity_over_falltime(cm: &Cm) -> f32 {
    gravity_integrate(cm, ms_to_dt(cm.fall_time_ms))
}

/// `CMovement::GetApexTime` (`0x7c5da0`, `systems/collision/falling.md#apex-time-and-rising-test`):
/// see `swim_fall_vel`.
pub fn fallvel_scale(cm: &Cm) -> f64 {
    crate::resolve_fall::swim_fall_vel(cm.fall_vel, cm.flags)
}

/// `CMovement::IsRising` (`0x7c5de0`): with flags & 0x2000 and a nonzero fall speed, whether the
/// apex time exceeds the fall time in seconds (true when NaN).
pub fn fall_complete(cm: &Cm) -> bool {
    if cm.flags & 0x2000 == 0 {
        return false;
    }
    let v = f(cm.fall_vel);
    if v == 0.0 {
        return false;
    }
    let t_sec = (f64::from(cm.fall_time_ms) * MS_TO_S) as f32;
    let s = fallvel_scale(cm);

    matches!(
        s.partial_cmp(&f(t_sec)),
        None | Some(core::cmp::Ordering::Greater)
    )
}

/// `CMovement::GetFallDistance` (`0x7c5e70`, `systems/collision/falling.md#fall-distance`): the
/// distance fallen in `dt`, `(dt·g/2 + v)·dt` while below the terminal speed, else the part up to
/// it plus the rest at the terminal speed; unrounded.
pub fn fall_displacement(cm: &Cm, dt: f32) -> f64 {
    let t = cm.terminal();
    let v = f(cm.fall_vel);
    let vc = if v > t { t } else { v };
    let dtf = f(dt);
    let vend = dtf * G + vc;
    if vend > t || vend.is_nan() {
        let t1 = (t - vc) * INV_G;
        let accel = (vc + HALF_G * t1) * t1;
        let term = (dtf - t1) * t;
        accel + term
    } else {
        (dtf * HALF_G + vc) * dtf
    }
}

/// `CMovement::GetFallDistanceMs` (`0x7c5f00`): `0x7c5e70` at `ms·0.001`.
pub fn fall_displacement_ms(cm: &Cm, ms: u32) -> f64 {
    fall_displacement(cm, ms_to_dt(ms))
}

/// `CMovement::GetFallStartZ` (`0x7c5f30`): `0x7c5f00` plus `+0x18`.
pub fn fall_displacement_ms_plus_base(cm: &Cm, ms: u32) -> f64 {
    fall_displacement_ms(cm, ms) + f(cm.field_18)
}

/// `CMovement::GetFallHeight` (`0x7c60c0`,
/// `systems/collision/falling.md#fall-height-from-the-apex`): `+0x7c − +0x18`, plus `v²/(2g)` when
/// `v < −2^-22`, and 0 when negative.
pub fn fall_distance_quick(cm: &Cm) -> f64 {
    let a = f(cm.field_7c) - f(cm.field_18);
    let v = f(cm.fall_vel);
    let r = if v < NEG_EPS { a + (v * v) * INV_2G } else { a };
    if r < 0.0 {
        0.0
    } else {
        r
    }
}

/// `CMovement::GetFallDropFrom` (`0x7c6110`): `0x7c5f00`, plus `arg − +0x7c` when flags & 0x2000.
pub fn fall_displacement_ms_transport(cm: &Cm, ms: u32, arg2: f32) -> f64 {
    let disp = fall_displacement_ms(cm, ms);
    if cm.flags & 0x2000 == 0 {
        disp
    } else {
        disp + (f(arg2) - f(cm.field_7c))
    }
}
