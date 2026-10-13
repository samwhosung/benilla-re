//! The straight and turning displacements, the facing and pitch integration, and the turn and pitch
//! rates (`systems/collision/kinematics.md`). Each function is `__thiscall` on the mover;
//! `CMovement` holds the fields they read.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0xcf5d00]`, 2π, which a static initializer stores.
const TWO_PI_BITS: u32 = 0x40c9_0fdb;

/// `[0x808acc]`: π/2.
const POS_HALF_PI_BITS: u32 = 0x3fc9_0fdb;

/// `[0x80c5e4]`: −π/2.
const NEG_HALF_PI_BITS: u32 = 0xbfc9_0fdb;

/// `[0x8012cc]`: 0.75.
const SPEED_SCALE_BITS: u32 = 0x3f40_0000;

/// The mover fields: flags `+0x40`, facing `+0x50`, pitch `+0x54`, direction `+0x5c`, the basis
/// `+0x68`, `+0x6c`, `+0x70` and `+0x74`, the speed `+0x84` and the turn speed `+0x9c`.
#[derive(Clone, Copy, Debug, Default)]
pub struct CMovement {
    pub flags: u32,
    pub facing: f32,
    pub pitch: f32,
    pub move_dir: [f32; 3],
    pub turn_basis: [f32; 3],
    pub turn_scale_z: f32,
    pub speed: f32,
    pub turn_speed: f32,
}

/// `CMovement::GetTurnRate` (`0x7c5c50`, `systems/collision/kinematics.md#turn-and-pitch-rates`):
/// `+0x9c` with flag 0x10, its negation with 0x20, else 0; times 0.75 with flags & 0x200f;
/// unrounded.
pub fn get_strafe_speed(flags: u32, speed: f32) -> f64 {
    let mut st = if flags & 0x10 != 0 {
        f(speed)
    } else if flags & 0x20 != 0 {
        -f(speed)
    } else {
        0.0
    };
    if flags & 0x200f != 0 {
        st *= f(f32::from_bits(SPEED_SCALE_BITS));
    }
    st
}

/// `CMovement::GetPitchRate` (`0x7c5c90`): the same with flags 0x40 and 0x80.
pub fn get_pitch_rate(flags: u32, speed: f32) -> f64 {
    let mut st = if flags & 0x40 != 0 {
        f(speed)
    } else if flags & 0x80 != 0 {
        -f(speed)
    } else {
        0.0
    };
    if flags & 0x200f != 0 {
        st *= f(f32::from_bits(SPEED_SCALE_BITS));
    }
    st
}

/// `CMovement::LinearDisplacement` (`0x7c4ed0`,
/// `systems/collision/kinematics.md#straight-line-displacement`): `(dt·dir)·speed` per axis; z
/// stores `dt·dir.z` as `f32` first.
pub fn linear_move(m: &CMovement, dt: f32) -> [f32; 3] {
    let s = f(m.speed);
    let ax = f(dt) * f(m.move_dir[0]);
    let bx = f(dt) * f(m.move_dir[1]);
    let cz = (f(dt) * f(m.move_dir[2])) as f32;
    [(ax * s) as f32, (bx * s) as f32, (f(cz) * s) as f32]
}

/// `CMovement::IntegrateFacing` (`0x7c4f30`,
/// `systems/collision/kinematics.md#facing-and-pitch-integration`): `fmod(rate·dt + facing, 2π)`,
/// plus 2π when negative; the rate is the argument's, or `0x7c5c50`'s when null.
pub fn strafe_turn(m: &CMovement, dt: f32, input: Option<f32>) -> f32 {
    let speed = match input {
        Some(v) => v,
        None => get_strafe_speed(m.flags, m.turn_speed) as f32,
    };
    let two_pi = f(f32::from_bits(TWO_PI_BITS));
    let angle = f(speed) * f(dt) + f(m.facing);

    let mut r = angle % two_pi;
    if r == 0.0 {
        r = if angle.is_sign_negative() && angle == 0.0 {
            -0.0
        } else {
            0.0
        };
    }
    if r < 0.0 {
        r += two_pi;
    }
    r as f32
}

/// `CMovement::IntegratePitch` (`0x7c4f80`): `rate·dt + pitch` clamped to ±π/2, NaN to π/2.
pub fn pitch_integrate(m: &CMovement, dt: f32) -> f32 {
    let rate = get_pitch_rate(m.flags, m.turn_speed);
    let p = rate * f(dt) + f(m.pitch);
    if p < f(f32::from_bits(NEG_HALF_PI_BITS)) {
        f32::from_bits(NEG_HALF_PI_BITS)
    } else if p < f(f32::from_bits(POS_HALF_PI_BITS)) {
        p as f32
    } else {
        f32::from_bits(POS_HALF_PI_BITS)
    }
}

/// `CMovement::TurnArcDisplacement` (`0x7c52c0`,
/// `systems/collision/kinematics.md#turn-arc-displacement`): with `R = speed/rate` and `θ =
/// rate·dt`, both stored as `f32`, the offset `(sin θ·R, R − cos θ·R)` through the basis, and z
/// `speed·+0x74·dt`.
pub fn simple_turn_move(m: &CMovement, dt: f32, turn_rate: Option<f32>) -> [f32; 3] {
    let s = f(m.speed);
    let tr = match turn_rate {
        Some(v) => f(v),
        None => get_strafe_speed(m.flags, m.turn_speed),
    };
    let r = (s / tr) as f32;
    let theta = (tr * f(dt)) as f32;
    let cos_t = f(theta).cos() as f32;
    let sin_t = f(theta).sin() as f32;
    let aa = f(sin_t) * f(r);
    let bb = f(r) - f(cos_t) * f(r);
    let m68 = f(m.turn_basis[0]);
    let m6c = f(m.turn_basis[1]);
    let m70 = f(m.turn_basis[2]);
    [
        ((aa * m68 - bb * m6c) * m70) as f32,
        ((bb * m68 + aa * m6c) * m70) as f32,
        ((s * f(m.turn_scale_z)) * f(dt)) as f32,
    ]
}
