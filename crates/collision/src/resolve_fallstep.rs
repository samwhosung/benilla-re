//! The airborne step's time from the fall arc and the rising test
//! (`systems/collision/airborne.md#step-time-from-the-fall`), and the motion state saved and
//! restored around the fall-away lookahead (`systems/collision/walking.md#fall-away-lookahead`).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::resolve_fall::{fall_time_solve, swim_fall_vel};

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x8026bc]`: 2^-20.
const SMALL_LEN: f64 = f32::from_bits(0x3580_0000) as f64;

/// What `0x635550` reads.
pub struct FallFeas {
    pub eax: u32,
    pub c: f32,
}

/// `CMovement::IsRisingThroughStep` (`0x635550`,
/// `systems/collision/airborne.md#step-time-from-the-fall`): returns 1 when the head flag is set or
/// the unit is still rising through this step.
pub fn fall_feasible(
    fa0: f32,
    flags: u32,
    f84: f32,
    a: f32,
    b: f32,
    vec: [f32; 2],
    c_in: f32,
) -> FallFeas {
    if c_in.to_bits() != 0 {
        return FallFeas { eax: 1, c: c_in };
    }

    if !(f(fa0) < 0.0) {
        return FallFeas { eax: 0, c: c_in };
    }

    let c = swim_fall_vel(fa0, flags) as f32;

    if f(a) + f(b) < f(c) {
        return FallFeas { eax: 1, c };
    }

    if f(a) > f(c) {
        return FallFeas { eax: 0, c };
    }

    let m = (f(c) - f(a)) * f(f84);
    let vsum = f(vec[1]) * f(vec[1]) + f(vec[0]) * f(vec[0]);
    if m * m > vsum {
        FallFeas { eax: 1, c }
    } else {
        FallFeas { eax: 0, c }
    }
}

/// What `0x635450` reads and writes.
pub struct FallStep {
    pub eax: u32,
    pub consumed: f64,
}

/// `CMovement::LimitStepByFall` (`0x635450`,
/// `systems/collision/airborne.md#step-time-from-the-fall`): returns an airborne step's time from
/// the fall arc, shortening the move's horizontal part to fit.
#[allow(clippy::too_many_arguments)]
pub fn fall_step(
    fa0: f32,
    flags: u32,
    vmax_land: f32,
    vmax_swim: f32,
    f7c: f32,
    pos_z: f32,
    f84: f32,
    remaining_dt: f32,
    t: f32,
    scratch: f32,
    len: f32,
    delta: &mut [f32; 3],
    out_len: &mut f32,
) -> FallStep {
    let feas = fall_feasible(
        fa0,
        flags,
        f84,
        t,
        remaining_dt,
        [delta[0], delta[1]],
        scratch,
    );
    let flag = feas.eax;

    let mut frac: f32 = 0.0;
    if f(len) > SMALL_LEN {
        let s = f(delta[1]) * f(delta[1]) + f(delta[0]) * f(delta[0]);
        frac = ((s.sqrt() / f(len)) * f(remaining_dt)) as f32;
    }

    let d = (f(f7c) - f(pos_z) - f(delta[2])) as f32;
    let solved = fall_time_solve(fa0, flags, vmax_land, vmax_swim, d, flag);

    if solved <= f(t) {
        *delta = [0.0, 0.0, 0.0];
        *out_len = 0.0;
        return FallStep {
            eax: 0,
            consumed: 0.0,
        };
    }

    let r = solved - f(t);

    if r > f(remaining_dt) {
        return FallStep {
            eax: flag,
            consumed: f(remaining_dt),
        };
    }

    if r < f(frac) {
        let k = r / f(frac);
        let kdx = k * f(delta[0]);
        let kdy = k * f(delta[1]);
        delta[0] = kdx as f32;
        delta[1] = kdy as f32;
        let dz = f(delta[2]);

        *out_len = ((kdy * kdy + kdx * kdx) + dz * dz).sqrt() as f32;
    }

    FallStep {
        eax: flag,
        consumed: r,
    }
}

/// The mover offsets `0x635ea0` copies, in its order.
pub const SNAPSHOT_FIELDS: [u32; 14] = [
    0x44, 0x48, 0x4c, 0x50, 0x54, 0x58, 0x78, 0x5c, 0x60, 0x64, 0x68, 0x6c, 0x40, 0x84,
];

/// `CMovement::SaveMotionState` (`0x635ea0`, `systems/collision/walking.md#fall-away-lookahead`):
/// copies the anchors, fall time, direction, heading basis, flags and speed into a 56-byte record.
pub fn state_snapshot(src: &[u32; 14]) -> [u32; 14] {
    *src
}

/// `CMovement::RestoreMotionState` (`0x635f10`,
/// `systems/collision/walking.md#fall-away-lookahead`): copies a 56-byte record written by 0x635ea0
/// back into the movement.
pub fn state_restore(buf: &[u32; 14]) -> [u32; 14] {
    *buf
}
