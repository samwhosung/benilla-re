//! The step-up probe (`systems/collision/walking.md#step-up-probe`). The sweeps and the world query
//! are passed in.

#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]

use crate::resolve_clip::HitRecord;
use crate::resolve_slope::walkable_slope;
use crate::resolve_toolkit::Plane;
use crate::resolve_walk::{feature_classify, speed_select};

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80dfec]`: 1/720.
const SKIN: f32 = f32::from_bits(0x3ab6_0b61);

/// `[0x80e008]`: tan 50°.
const TAN50: f32 = f32::from_bits(0x3f98_8b62);

/// `[0x80dffc]`: cos 50°.
const COS50: f32 = f32::from_bits(0x3f24_8dbb);

/// `[0x8029d4]`: 2^-22.
const LEN_GUARD: f64 = 2.384_185_8e-7_f32 as f64;

/// `[0x8026bc]`: 2^-20.
const NZ_GATE: f64 = 9.536_743e-7_f32 as f64;

/// `[0x8029d0]`: 0.01.
const DOWN_BIAS: f32 = f32::from_bits(0x3c23_d70a);

/// Flags `0x4000000`.
const GROUNDED: u32 = 0x0400_0000;

/// What a sweep `0x632ba0` gives back.
#[derive(Clone, Copy)]
pub struct SweepRes {
    pub contact: bool,
    pub idx: u32,
    pub t: f32,
    pub count: u32,
    pub manifold: [Plane; 7],
}

/// `CMovement::ProbeStepUp` (`0x636100`, `systems/collision/walking.md#step-up-probe`): returns 2
/// when a step over a face is possible, 1 when blocked, 0 when a sweep fails.
pub fn step_up_resolve(
    this_pos: &mut [f32; 3],
    this_radius: f32,
    this_height: f32,
    this_flags: &mut u32,
    this_floor: &mut f32,
    h: f32,
    obj_pred: bool,
    input_dir: [f32; 2],
    vel: &mut [f32; 3],
    hits: &[HitRecord],
    hit_count: u32,
    mut sweep: impl FnMut(&[f32; 3], f32, &[f32; 3]) -> SweepRes,
    mut fall_land: impl FnMut(&[f32; 2], &[f32; 2], f32, &[f32; 3]) -> u32,
) -> u32 {
    let saved = *this_pos;

    let r_plus = (f(this_radius) + f(SKIN)) as f32;
    let step_rise: f32 = if f(h) * f(TAN50) > f(r_plus) {
        (f(h) * f(TAN50)) as f32
    } else {
        r_plus
    };

    let vz = vel[2];
    let do_stepup = !(f(vz) > f(COS50) || f(vz).is_nan() || f(vz) < 0.0);

    let (mut sx, mut sy) = (input_dir[0], input_dir[1]);

    if do_stepup {
        let vx = vel[0];
        let vy = vel[1];
        let lh = (f(vy) * f(vy) + f(vx) * f(vx)).sqrt();
        let inv_l = 1.0 / lh;
        let nx = (inv_l * f(vx)) as f32;
        let ny = (f(vy) * inv_l) as f32;
        sx = -nx;
        sy = -ny;
        let back_dir = [sx, sy, 0.0];
        let res = sweep(&back_dir, step_rise, this_pos);

        if !res.contact {
            return 0;
        }

        let keep = res.idx != hit_count && {
            let rec = &hits[res.idx as usize];
            f(rec.normal[0]) == f(vel[0])
                && f(rec.normal[1]) == f(vel[1])
                && f(rec.normal[2]) == f(vel[2])
        };
        if !keep {
            sx = input_dir[0];
            sy = input_dir[1];
        }
    }

    let res_b = sweep(&[0.0, 0.0, 1.0], h, this_pos);
    if !res_b.contact {
        return 0;
    }

    let up_t = res_b.t;
    vel[2] = up_t;
    if f(up_t).abs() < LEN_GUARD {
        return 1;
    }

    this_pos[2] = (f(this_pos[2]) + f(up_t)) as f32;
    let res_c = sweep(&[sx, sy, 0.0], step_rise, this_pos);
    if !res_c.contact {
        return 0;
    }
    let out_t = res_c.t;

    this_pos[0] = (f(this_pos[0]) + f(sx) * f(out_t)) as f32;
    this_pos[1] = (f(this_pos[1]) + f(sy) * f(out_t)) as f32;

    let mut last_idx = res_c.idx;
    let mut accum_dist = out_t;

    if !((f(out_t) - f(step_rise)).abs() < NZ_GATE) {
        let rec_c = &hits[res_c.idx as usize];
        let mut out_steep = 0u32;
        let walkable = walkable_slope(
            this_flags,
            this_pos,
            this_radius,
            this_height,
            obj_pred,
            &rec_c.normal,
            rec_c.d,
            &rec_c.basis,
            &mut out_steep,
        );

        if walkable != 0 {
            let remaining = (f(step_rise) - f(out_t)) as f32;

            let mut feat = [0.0f32; 3];
            feature_classify(&res_c.manifold, res_c.count, &mut feat);

            let w = [sx, sy, 0.0];
            let mut speed = remaining;
            let mut dir_io = rec_c.normal;
            let mut out_vec = [0.0f32; 3];
            speed_select(
                *this_flags,
                h,
                &w,
                &mut speed,
                &mut dir_io,
                1,
                &feat,
                &mut out_vec,
            );

            let nv_x = (f(out_vec[0]) + f(sx) * f(speed)) as f32;
            let nv_y = (f(out_vec[1]) + f(sy) * f(speed)) as f32;
            let nv_z = out_vec[2];

            let len = ((f(nv_x) * f(nv_x) + f(nv_y) * f(nv_y)) + f(nv_z) * f(nv_z)).sqrt();
            let len_f32 = len as f32;

            if !(len.abs() < LEN_GUARD) {
                let inv = 1.0 / f(len_f32);
                let nd_x = (f(nv_x) * inv) as f32;
                let nd_y = (f(nv_y) * inv) as f32;
                let nd_z = (inv * f(nv_z)) as f32;
                let res_c2 = sweep(&[nd_x, nd_y, nd_z], len_f32, this_pos);
                if !res_c2.contact {
                    return 0;
                }
                let t2 = res_c2.t;

                let dx = f(nd_x) * f(t2);
                let dy = f(nd_y) * f(t2);
                let dz = (f(nd_z) * f(t2)) as f32;
                let horizdist = (dx * dx + dy * dy).sqrt();
                accum_dist = (f(out_t) + horizdist) as f32;
                this_pos[0] = (f(this_pos[0]) + dx) as f32;
                this_pos[1] = (f(this_pos[1]) + dy) as f32;
                this_pos[2] = (f(this_pos[2]) + f(dz)) as f32;
                vel[2] = (f(vel[2]) + f(dz)) as f32;
                last_idx = res_c2.idx;
            }
        }
    }

    let go_downprobe = if last_idx == hit_count {
        true
    } else {
        f(accum_dist) > f(this_radius)
    };
    if !go_downprobe {
        *this_pos = saved;
        return 1;
    }

    let dn_len = (f(vel[2]) + f(DOWN_BIAS)) as f32;
    let res_d = sweep(&[0.0, 0.0, -1.0], dn_len, this_pos);
    if !res_d.contact {
        return 0;
    }
    let down_t = res_d.t;

    this_pos[2] = (f(this_pos[2]) - f(down_t)) as f32;

    if res_d.idx == hit_count {
        return finalize_return_2(this_pos, &saved, this_flags, this_floor);
    }

    let rec_d = &hits[res_d.idx as usize];
    if f(rec_d.normal[2]) > f(COS50) || f(rec_d.normal[2]).is_nan() {
        return finalize_return_2(this_pos, &saved, this_flags, this_floor);
    }

    let drop = (f(vel[2]) - f(down_t)) as f32;
    if fall_land(&[sx, sy], &[saved[0], saved[1]], drop, this_pos) != 0 {
        finalize_return_2(this_pos, &saved, this_flags, this_floor)
    } else {
        *this_pos = saved;
        1
    }
}

/// `0x6365df` in `CMovement::ProbeStepUp` (`0x636100`,
/// `systems/collision/walking.md#step-up-probe`).
#[inline]
fn finalize_return_2(
    this_pos: &mut [f32; 3],
    saved: &[f32; 3],
    this_flags: &u32,
    this_floor: &mut f32,
) -> u32 {
    *this_pos = *saved;
    if *this_flags & GROUNDED == 0 {
        *this_floor = saved[2];
    }
    2
}
