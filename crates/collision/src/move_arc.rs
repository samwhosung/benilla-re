//! The arc displacements and the direction basis (`systems/collision/kinematics.md`). Each function
//! is `__thiscall` on the mover; `CMovement` holds the fields they read and write. The emulator
//! evaluates `fsincos` with the host's maths library, as `f64` does here.

use crate::move_integrate::{get_pitch_rate, get_strafe_speed};

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `x ≥ eps`, or NaN: the fall-through of `fcomp; test ah,5; jp`.
#[inline]
fn not_below(x: f64, eps: f64) -> bool {
    x >= eps || x.is_nan()
}

/// `[0x808acc]`: π/2, the pitch's upper clamp.
const POS_HALF_PI_BITS: u32 = 0x3fc9_0fdb;

/// `[0x80c5e4]`: −π/2, the lower clamp.
const NEG_HALF_PI_BITS: u32 = 0xbfc9_0fdb;

/// `[0x8026bc]`: 2^-20.
const EPS_BITS: u32 = 0x3580_0000;

/// `[0x7ffa24]`: 0.5.
const HALF_BITS: u32 = 0x3f00_0000;

/// `[0x7ff9d8]`: 1.0.
const ONE_BITS: u32 = 0x3f80_0000;

/// `[0x81da54]`: 1/√2, the diagonal scale.
const INV_SQRT2_BITS: u32 = 0x3f35_04f3;

/// Flags `0x200000`: the pitch tilts the basis.
const PITCH_ENABLE_FLAG: u32 = 0x0020_0000;

/// The mover fields: flags `+0x40`, facing `+0x50`, pitch `+0x54`, direction `+0x5c`, the basis
/// `+0x68`, `+0x6c`, `+0x70`, `+0x74`, the speed `+0x84` and the turn speed `+0x9c`.
#[derive(Clone, Copy, Debug, Default)]
pub struct CMovement {
    pub flags: u32,
    pub facing: f32,
    pub pitch: f32,
    pub dir: [f32; 3],
    pub b68: f32,
    pub b6c: f32,
    pub b70: f32,
    pub b74: f32,
    pub speed: f32,
    pub turn_speed: f32,
}

/// `CMovement::TurnPitchDisplacement` (`0x7c4fd0`,
/// `systems/collision/kinematics.md#turn-and-pitch-displacement`): the displacement over `dt`
/// turning at the given rate, or `0x7c5c50`'s when null, and pitching at `0x7c5c90`'s, with the
/// pitch clamped to ±π/2 and the z term corrected for the time past the clamp.
pub fn turn_pitch_arc(m: &CMovement, dt: f32, turn_rate: Option<f32>) -> [f32; 3] {
    let speed = f(m.speed);
    let turn_r = match turn_rate {
        Some(v) => v,
        None => get_strafe_speed(m.flags, m.turn_speed) as f32,
    };
    let pitch_rate = get_pitch_rate(m.flags, m.turn_speed);
    let r = (speed / f(turn_r)) as f32;
    let inv_pr = f(f32::from_bits(ONE_BITS)) / pitch_rate;
    let inv_pr_f32 = inv_pr as f32;
    let sp_pr = (inv_pr * speed) as f32;
    let prod = pitch_rate * f(dt);
    let pr_dt = prod as f32;
    let new_pitch = prod + f(m.pitch);

    let pos = f(f32::from_bits(POS_HALF_PI_BITS));
    let neg = f(f32::from_bits(NEG_HALF_PI_BITS));

    let (clamp_time, pitch_delta): (f32, f32) = if new_pitch > pos {
        (
            ((new_pitch - pos) * f(inv_pr_f32)) as f32,
            (pos - f(m.pitch)) as f32,
        )
    } else if new_pitch < neg {
        (
            ((new_pitch + pos) * f(inv_pr_f32)) as f32,
            (neg - f(m.pitch)) as f32,
        )
    } else {
        (0.0, pr_dt)
    };

    let theta = ((f(dt) - f(clamp_time)) * f(turn_r)) as f32;
    let cos_t = f(theta).cos() as f32;
    let sin_t = f(theta).sin() as f32;
    let cos_pd = f(pitch_delta).cos() as f32;
    let sin_pd = f(pitch_delta).sin() as f32;

    let r_f = f(r);
    let sp = f(sp_pr);
    let p = f(sin_pd) * sp;
    let q = f(sin_t) * r_f;
    let a = f(f32::from_bits(HALF_BITS)) * (q + p);
    let b = f(cos_t) * r_f - r_f;
    let c = (sp - f(cos_pd) * sp) as f32;

    let m68 = f(m.b68);
    let m6c = f(m.b6c);
    let m70 = f(m.b70);
    let m74 = f(m.b74);
    let e = (a * m68 - b * m6c) as f32;
    let d = b * m68 + a * m6c;
    let c_f = f(c);
    let e_f = f(e);

    let out0 = (e_f * m70 - c_f * m74) as f32;
    let out1 = d as f32;
    let out2_reg = c_f * m70 + e_f * m74;
    let mut out = [out0, out1, out2_reg as f32];

    let eps = f(f32::from_bits(EPS_BITS));
    if not_below(f(clamp_time).abs(), eps) {
        let clamped_pitch = f(pitch_delta) + f(m.pitch);
        let decide = (clamped_pitch - pos).abs();
        let cs = f(clamp_time) * speed;
        out[2] = if decide < eps {
            (cs + out2_reg) as f32
        } else {
            (out2_reg - cs) as f32
        };
    }
    out
}

/// `CMovement::PitchArcDisplacement` (`0x7c5180`,
/// `systems/collision/kinematics.md#pitch-arc-displacement`): the same with no turn.
pub fn pitch_arc(m: &CMovement, dt: f32) -> [f32; 3] {
    let speed = f(m.speed);
    let pitch_rate = get_pitch_rate(m.flags, m.turn_speed);
    let inv_pr = f(f32::from_bits(ONE_BITS)) / pitch_rate;
    let inv_pr_f32 = inv_pr as f32;
    let sp_pr = (inv_pr * speed) as f32;
    let prod = pitch_rate * f(dt);
    let pr_dt = prod as f32;
    let new_pitch = prod + f(m.pitch);

    let pos = f(f32::from_bits(POS_HALF_PI_BITS));
    let neg = f(f32::from_bits(NEG_HALF_PI_BITS));
    let (clamp_time, pitch_delta): (f32, f32) = if new_pitch > pos {
        (
            ((new_pitch - pos) * f(inv_pr_f32)) as f32,
            (pos - f(m.pitch)) as f32,
        )
    } else if new_pitch < neg {
        (
            ((new_pitch + pos) * f(inv_pr_f32)) as f32,
            (neg - f(m.pitch)) as f32,
        )
    } else {
        (0.0, pr_dt)
    };

    let cos_pd = f(pitch_delta).cos() as f32;
    let sin_pd = f(pitch_delta).sin() as f32;

    let sp = f(sp_pr);
    let p = f(sin_pd) * sp;
    let c = sp - f(cos_pd) * sp;

    let m68 = f(m.b68);
    let m6c = f(m.b6c);
    let m70 = f(m.b70);
    let m74 = f(m.b74);
    let w = p * m70 - c * m74;
    let out0 = (w * m68) as f32;
    let out1 = (w * m6c) as f32;
    let out2_reg = c * m70 + p * m74;
    let mut out = [out0, out1, out2_reg as f32];

    let eps = f(f32::from_bits(EPS_BITS));
    if not_below(f(clamp_time).abs(), eps) {
        let clamped_pitch = f(pitch_delta) + f(m.pitch);
        let decide = (clamped_pitch - pos).abs();
        let cs = f(clamp_time) * speed;
        out[2] = if decide < eps {
            (cs + out2_reg) as f32
        } else {
            (out2_reg - cs) as f32
        };
    }
    out
}

/// `CMovement::CalcBaseDirection` (`0x7c5880`,
/// `systems/collision/kinematics.md#direction-and-heading-basis`): `+0x68` and `+0x6c` from the
/// facing's cosine and sine; with flags & 0x200000 and `|pitch| ≥ 2^-20` the pitch's cosine and
/// sine at `+0x70` and `+0x74` and the tilted direction, else 1, 0 and the flat direction.
pub fn velocity_basis(m: &mut CMovement) {
    let cos_f = f(m.facing).cos() as f32;
    let sin_f = f(m.facing).sin() as f32;
    m.b68 = cos_f;
    m.b6c = sin_f;

    let eps = f(f32::from_bits(EPS_BITS));
    let pitched = (m.flags & PITCH_ENABLE_FLAG) != 0 && not_below(f(m.pitch).abs(), eps);
    if pitched {
        let cos_p = f(m.pitch).cos() as f32;
        let sin_p = f(m.pitch).sin() as f32;
        m.b70 = cos_p;
        m.b74 = sin_p;
        m.dir[0] = (f(cos_f) * f(cos_p)) as f32;
        m.dir[1] = (f(sin_f) * f(cos_p)) as f32;
        m.dir[2] = sin_p;
    } else {
        m.dir[0] = cos_f;
        m.dir[1] = sin_f;
        m.dir[2] = 0.0;
        m.b74 = 0.0;
        m.b70 = f32::from_bits(ONE_BITS);
    }
}

/// The strafe direction: the basis turned a quarter, toward flag 4's side.
fn swap_negate(m: &mut CMovement) {
    let old68 = m.b68;
    let old6c = m.b6c;
    if (m.flags & 0x4) != 0 {
        m.b68 = -old6c;
        m.b6c = old68;
    } else {
        m.b68 = old6c;
        m.b6c = -old68;
    }
}

/// `x`, negated when moving backward.
#[inline]
fn sgn(backward: bool, x: f32) -> f32 {
    if backward {
        -x
    } else {
        x
    }
}

/// `CMovementData::CalcDirection` (`0x7c5a20`,
/// `systems/collision/kinematics.md#direction-and-heading-basis`): nothing with flags & 0x2000 and
/// a null `arg`; otherwise `0x7c5880`, then the direction from the move bits: forward or back (1,
/// 2), strafe (4, 8), or both summed and scaled by 1/√2.
pub fn velocity_vector(m: &mut CMovement, arg: u32) {
    if (m.flags & 0x2000) != 0 && arg == 0 {
        return;
    }
    velocity_basis(m);

    let flags = m.flags;
    let fb = flags & 0x3;
    let strafe = flags & 0xc;
    let backward = (flags & 0x2) != 0;

    if fb != 0 && strafe != 0 {
        let l5c = sgn(backward, m.dir[0]);
        let l60 = sgn(backward, m.dir[1]);
        let l64 = sgn(backward, m.dir[2]);
        let l68 = sgn(backward, m.b68);
        let l6c = sgn(backward, m.b6c);
        swap_negate(m);
        let p68 = m.b68;
        let p6c = m.b6c;
        let nv5c = (f(p68) + f(l5c)) as f32;
        let nv60 = (f(l60) + f(p6c)) as f32;
        let nv64 = l64;
        let nv68 = (f(l68) + f(p68)) as f32;
        let nv6c = (f(l6c) + f(p6c)) as f32;
        let k = f(f32::from_bits(INV_SQRT2_BITS));
        m.dir[0] = (f(nv5c) * k) as f32;
        m.dir[1] = (f(nv60) * k) as f32;
        m.dir[2] = (f(nv64) * k) as f32;
        m.b68 = (f(nv68) * k) as f32;
        m.b6c = (f(nv6c) * k) as f32;
    } else if fb != 0 {
        if backward {
            m.dir[0] = -m.dir[0];
            m.dir[1] = -m.dir[1];
            m.dir[2] = -m.dir[2];
            m.b68 = -m.b68;
            m.b6c = -m.b6c;
        }
    } else if strafe != 0 {
        swap_negate(m);
        m.dir[0] = m.b68;
        m.dir[1] = m.b6c;
        m.dir[2] = 0.0;
    }
}
