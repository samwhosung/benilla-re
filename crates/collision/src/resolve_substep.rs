//! One airborne step's sweep (`systems/collision/airborne.md#airborne-step-sweep`) and its two
//! plane tests. The sweep, the world query and the callees with effects outside the mover are
//! passed in.

#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]

use crate::resolve_clip::HitRecord;
use crate::resolve_fall::swim_fall_vel;
use crate::resolve_toolkit::Plane;
use crate::resolve_walk::{ground_at_point, slide_response};

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x8026bc]`: 2^-20.
const NZ_GATE: f64 = 9.536_743e-7_f32 as f64;

/// `[0x8029d4]`: 2^-22.
const LEN_GUARD: f64 = 2.384_185_8e-7_f32 as f64;

/// 1.0, the normal z of an upward plane.
const UP_Z: f32 = f32::from_bits(0x3f80_0000);
/// `[0x80e014]`: −0.4756366, the normal z of the volume's bevels.
const BEVEL_Z: f32 = f32::from_bits(0xbef3_86a4);

/// `HasUpwardPlane` (`0x6353d0`, `systems/collision/airborne.md#airborne-step-sweep`): returns 1
/// when a plane's normal z lies within 2^-20 of 1.
pub fn any_plane_up(planes: &[Plane], count: u32) -> bool {
    planes
        .iter()
        .take(count as usize)
        .any(|p| (f(p.n[2]) - f(UP_Z)).abs() < NZ_GATE)
}

/// `HasBevelPlane` (`0x635410`, `systems/collision/airborne.md#airborne-step-sweep`): returns 1
/// when a plane's normal z lies within 2^-20 of -0.4756366, the volume bevels' z.
pub fn any_plane_bevel(planes: &[Plane], count: u32) -> bool {
    planes
        .iter()
        .take(count as usize)
        .any(|p| (f(p.n[2]) - f(BEVEL_Z)).abs() < NZ_GATE)
}

/// What the sweep `0x632ba0` gives back.
#[derive(Clone, Copy, Debug)]
pub struct SweepOut {
    pub contact: bool,
    pub idx: u32,
    pub t: f32,
    pub count: u32,
    pub hit_count: u32,
}

/// `CMovement::SweepAirborneStep` (`0x6351a0`,
/// `systems/collision/airborne.md#airborne-step-sweep`): sweeps one airborne step; returns 0 clear,
/// 1 hit (landing, head strike or push), 2 failed.
pub fn walk_slide_substep(
    this_pos: &[f32; 3],
    this_radius: f32,
    this_f84: f32,
    this_fa0: f32,
    this_flags: u32,
    obj_pred: bool,
    t_param: f32,
    move_vec: &mut [f32; 3],
    contact_dist: &mut f32,
    out_vec_a: &mut [f32; 2],
    out_vec_b: &mut [f32; 2],
    flag_a: &mut i32,
    flag_b: &mut i32,
    sweep: &SweepOut,
    manifold: &[Plane],
    hit: &HitRecord,
    slide_vel_row: [f32; 2],
) -> u32 {
    *flag_a = 0;
    *flag_b = 0;

    let len_in = *contact_dist;
    if f(len_in).abs() < LEN_GUARD {
        *out_vec_b = [0.0, 0.0];
        return 0;
    }

    let inv_l = 1.0 / f(len_in);
    let dir: [f32; 3] = [
        (inv_l * f(move_vec[0])) as f32,
        (inv_l * f(move_vec[1])) as f32,
        (inv_l * f(move_vec[2])) as f32,
    ];

    *contact_dist = sweep.t;

    if !sweep.contact {
        *move_vec = [0.0, 0.0, 0.0];
        return 2;
    }

    if sweep.idx == sweep.hit_count {
        *out_vec_b = [0.0, 0.0];
        return 0;
    }

    let t = sweep.t;
    move_vec[0] = (f(dir[0]) * f(t)) as f32;
    move_vec[1] = (f(dir[1]) * f(t)) as f32;
    move_vec[2] = (f(t) * f(dir[2])) as f32;

    let offset = *move_vec;
    let ground = ground_at_point(this_pos, &offset, hit.normal[2], &hit.basis, obj_pred);
    if ground {
        *flag_a = 1;
        if any_plane_bevel(manifold, sweep.count) {
            *out_vec_b = slide_vel_row;
        } else {
            *out_vec_b = [0.0, 0.0];
        }
        return 1;
    }

    *out_vec_b = [0.0, 0.0];

    if f(dir[2]) > 0.0 {
        let r = swim_fall_vel(this_fa0, this_flags) - f(t_param);

        if r >= 0.0 {
            let lhs = r * f(this_f84);
            if lhs >= f(*contact_dist) && any_plane_up(manifold, sweep.count) {
                *flag_b = 1;
                return 1;
            }
        }
    }

    let feature = [manifold[0], manifold[1]];
    let mut sout = [0.0f32; 2];
    slide_response(
        this_pos,
        this_radius,
        &dir,
        t,
        len_in,
        hit.normal[2],
        &hit.normal,
        hit.d,
        &hit.basis,
        &feature,
        sweep.count,
        &mut sout,
    );
    *out_vec_a = sout;
    1
}
