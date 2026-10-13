//! The walk resolver loop (`systems/collision/walking.md#walk-resolver-loop`). The sweeps, the
//! world query and the callees with effects outside the mover are passed in, and their calls are
//! recorded.

#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]

use crate::resolve_clip::HitRecord;
use crate::resolve_multipass::{step_up_resolve, SweepRes};
use crate::resolve_slope::walkable_slope;
use crate::resolve_walk::{feature_classify, speed_select, vector_slide};

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x8026bc]`: 2^-20.
const NZ_GATE: f32 = f32::from_bits(0x3580_0000);
/// `[0x8029d4]`: 2^-22.
const LEN_GUARD: f32 = f32::from_bits(0x3480_0000);
/// `[0x801360]`: 0.001.
const MS_TO_S: f32 = f32::from_bits(0x3a83_126f);
/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x7ff9d0]`: 1/36.
const STEP_FLOOR: f32 = f32::from_bits(0x3ce3_8e39);
/// `[0x80c740]`: 1.8493990.
const RING: f32 = f32::from_bits(0x3fec_b91b);
/// `[0x80dffc]`: cos 50°.
const COS50: f32 = f32::from_bits(0x3f24_8dbb);
/// `[0x80e000]`: cos 80°.
const COS80: f32 = f32::from_bits(0x3e31_d0d4);
/// Flags `0x40000000`.
const WATER: u32 = 0x4000_0000;
/// Flags `0x4000000`.
const GROUNDED: u32 = 0x0400_0000;

/// The calls `0x6367b0` makes whose effects lie outside the mover, recorded in order.
#[derive(Clone, Copy, Debug, Default)]
pub struct WalkSeam {
    pub start_falling: bool,
    pub begin_move: bool,
    pub set_facing: Option<[f32; 2]>,
}

enum Exit {
    Finalize,
    GroundSettle,
}

/// `CMovement::ResolveWalk` (`0x6367b0`, `systems/collision/walking.md#walk-resolver-loop`): moves
/// a grounded mover with deflection, steps and settle, or starts its fall, and returns the ms
/// consumed.
pub fn resolve_walk_mode(
    this_pos: &mut [f32; 3],
    this_radius: f32,
    this_height: f32,
    this_flags: &mut u32,
    this_floor: &mut f32,
    budget: i32,
    dist: f32,
    dir: [f32; 2],
    h: f32,
    obj_pred: bool,
    swim_terminal: f32,
    hits: &[HitRecord],
    hit_count: u32,
    slide_vel_rows: &[[f32; 2]],
    seam: &mut WalkSeam,
    mut sweep: impl FnMut(&[f32; 3], f32, &[f32; 3]) -> SweepRes,
    mut fall_land: impl FnMut(&[f32; 2], &[f32; 2], f32, &[f32; 3]) -> u32,
) -> u32 {
    let budget_ret = budget as u32;

    if f(dist).abs() < f(NZ_GATE) {
        return budget_ret;
    }

    if hit_count == 0 {
        seam.start_falling = true;
        return 0;
    }

    let budget_f = f64::from(budget as u32) as f32;
    let water = *this_flags & WATER != 0;

    let mut slide_dir = [dir[0], dir[1], 0.0f32];
    let mut ebp_3c = dir[0];
    let mut ebp_38 = dir[1];
    let mut length = dist;
    let mut dist_acc = dist;
    let mut consumed: f32 = 0.0;
    let mut segbudget = budget_f;
    let mut facing_flag: u32 = 0;
    let mut settled_flag: u32 = 0;
    let mut iter_count: u32 = 0;
    let mut water_probe_t: f32 = 0.0;

    let mut out_t: f32;
    let mut idx: u32;
    let mut count: u32;
    let mut manifold;
    let mut delta = [0.0f32; 3];
    let mut hit_normal;
    let mut slidevel = [0.0f32; 2];

    if water {
        let r = sweep(&[0.0, 0.0, -ONE], ONE, this_pos);
        if !r.contact {
            return budget_ret;
        }
        water_probe_t = r.t;
    }

    {
        let r = sweep(&slide_dir, dist, this_pos);
        if !r.contact {
            return budget_ret;
        }
        out_t = r.t;
        idx = r.idx;
        count = r.count;
        manifold = r.manifold;
    }

    let exit = 'main: loop {
        delta[0] = (f(slide_dir[0]) * f(out_t)) as f32;
        delta[1] = (f(slide_dir[1]) * f(out_t)) as f32;
        delta[2] = (f(slide_dir[2]) * f(out_t)) as f32;
        this_pos[0] = (f(delta[0]) + f(this_pos[0])) as f32;
        this_pos[1] = (f(delta[1]) + f(this_pos[1])) as f32;
        this_pos[2] = (f(delta[2]) + f(this_pos[2])) as f32;

        if idx >= hit_count {
            consumed = (f(segbudget) + f(consumed)) as f32;
            let remaining = f(budget_f) - f(consumed);

            if remaining < 1.0 || remaining.is_nan() {
                break 'main Exit::GroundSettle;
            }
            segbudget = remaining as f32;

            slide_dir = [dir[0], dir[1], 0.0];
            ebp_3c = dir[0];
            ebp_38 = dir[1];
            length = (remaining / f(budget_f) * f(dist)) as f32;
        } else {
            slidevel = slide_vel_rows[idx as usize];
            let consumed_incr = f(out_t) / f(length) * f(segbudget);
            length = (f(length) - f(out_t)) as f32;
            let horizdist = (f(delta[0]) * f(delta[0]) + f(delta[1]) * f(delta[1])).sqrt();
            dist_acc = (f(dist_acc) - horizdist) as f32;

            if consumed_incr < 1.0 || consumed_incr.is_nan() {
                iter_count += 1;
                if iter_count > 5 {
                    facing_flag = 1;
                    break 'main Exit::Finalize;
                }
            } else {
                iter_count = 1;
            }

            consumed = (f(consumed) + consumed_incr) as f32;
            segbudget = (f(segbudget) - consumed_incr) as f32;

            let rem2 = f(budget_f) - f(consumed);
            if rem2 < 1.0 {
                if !((f(budget_f) - f(consumed)).abs() < f(NZ_GATE)) {
                    facing_flag = 1;
                }
                break 'main Exit::Finalize;
            }

            hit_normal = hits[idx as usize].normal;
            let rec = hits[idx as usize];
            let mut out_steep = 0u32;
            let walkable = walkable_slope(
                this_flags,
                this_pos,
                this_radius,
                this_height,
                obj_pred,
                &rec.normal,
                rec.d,
                &rec.basis,
                &mut out_steep,
            );
            settled_flag = out_steep;

            let response: [f32; 3];
            if walkable != 0 {
                let saved_len = length;
                let mut feat = [0.0f32; 3];
                let flag = feature_classify(&manifold, count, &mut feat) as u32;
                let mut dir_io = rec.normal;
                let mut out_ec = [0.0f32; 3];
                speed_select(
                    *this_flags,
                    h,
                    &slide_dir,
                    &mut length,
                    &mut dir_io,
                    flag,
                    &feat,
                    &mut out_ec,
                );
                response = out_ec;

                segbudget = (f(length) / f(saved_len) * f(segbudget)) as f32;
            } else {
                if out_steep != 0 {
                    break 'main Exit::Finalize;
                }
                facing_flag = 1;
                let mut feat = [0.0f32; 3];
                let flag = feature_classify(&manifold, count, &mut feat) as u32;

                let mut vel = hit_normal;
                let code = step_up_resolve(
                    this_pos,
                    this_radius,
                    this_height,
                    this_flags,
                    this_floor,
                    h,
                    obj_pred,
                    [ebp_3c, ebp_38],
                    &mut vel,
                    hits,
                    hit_count,
                    &mut sweep,
                    &mut fall_land,
                );
                if code == 0 {
                    return budget_ret;
                }
                if code == 2 {
                    *this_flags |= GROUNDED;
                    let saved_len = length;
                    let mut dir_io = rec.normal;
                    let mut out_e0 = [0.0f32; 3];
                    speed_select(
                        *this_flags,
                        h,
                        &slide_dir,
                        &mut length,
                        &mut dir_io,
                        flag,
                        &feat,
                        &mut out_e0,
                    );
                    response = out_e0;
                    segbudget = (f(length) / f(saved_len) * f(segbudget)) as f32;
                } else {
                    let mut out_d4 = [0.0f32; 3];
                    vector_slide(&slide_dir, length, &hit_normal, &mut out_d4);
                    response = out_d4;
                }
            }

            let szs = (f(slide_dir[2]) * f(length)) as f32;
            let nvx = (f(slide_dir[0]) * f(length) + f(response[0])) as f32;
            let nvy = (f(response[1]) + f(slide_dir[1]) * f(length)) as f32;
            let nvz = (f(response[2]) + f(szs)) as f32;
            slide_dir = [nvx, nvy, nvz];

            let nvx2 = (f(nvx) * f(nvx)) as f32;
            let nvy2 = (f(nvy) * f(nvy)) as f32;
            let len_new = ((f(nvz) * f(nvz) + f(nvy2)) + f(nvx2)).sqrt();
            length = len_new as f32;

            if len_new < f(MS_TO_S) {
                facing_flag = 1;
                break 'main Exit::Finalize;
            }

            ebp_3c = nvx;
            ebp_38 = nvy;
            let inv_len = f(ONE) / f(length);
            slide_dir[0] = (f(nvx) * inv_len) as f32;
            slide_dir[1] = (f(nvy) * inv_len) as f32;
            slide_dir[2] = (inv_len * f(nvz)) as f32;

            let dist_acc_old = f(dist_acc);
            let horiz_len_new = (f(nvy2) + f(nvx2)).sqrt();
            dist_acc = horiz_len_new as f32;
            if !(horiz_len_new.abs() < f(LEN_GUARD)) {
                let inv_h = f(ONE) / f(dist_acc);
                ebp_3c = (f(nvx) * inv_h) as f32;
                ebp_38 = (f(nvy) * inv_h) as f32;
            }

            if dist_acc_old - horiz_len_new > f(NZ_GATE) {
                facing_flag = 1;
            }

            if *this_flags & GROUNDED != 0 && f(nvz) > f(NZ_GATE) {
                let a_target = (f(nvz) + f(this_pos[2])) as f32;
                let hf = f(h) + f(*this_floor);

                if hf < f(a_target) {
                    let a = f(h) + f(*this_floor) - f(this_pos[2]);
                    length = (a / f(nvz) * f(length)) as f32;
                    if !((a - f(nvz)).abs() < f(NZ_GATE)) {
                        facing_flag = 1;
                    }

                    if f(length) < f(MS_TO_S) {
                        settled_flag = 1;
                        break 'main Exit::Finalize;
                    }
                }
            }
        }

        let r = sweep(&slide_dir, length, this_pos);
        if !r.contact {
            return budget_ret;
        }
        out_t = r.t;
        idx = r.idx;
        count = r.count;
        manifold = r.manifold;
    };

    if let Exit::GroundSettle = exit {
        let mut arg10 = (f(out_t) * f(RING) + f(STEP_FLOOR)) as f32;
        if water {
            arg10 = (f(arg10) + f(ONE)) as f32;
        }
        if *this_flags & GROUNDED != 0 {
            arg10 = (f(h) + f(arg10)) as f32;
        }

        let r = sweep(&[0.0, 0.0, -ONE], arg10, this_pos);
        if !r.contact {
            return budget_ret;
        }
        arg10 = r.t;
        idx = r.idx;
        count = r.count;
        let _ = count;
        this_pos[2] = (f(this_pos[2]) - f(arg10)) as f32;

        if idx == hit_count {
            slidevel = [0.0, 0.0];
            settled_flag = 1;
            water_adjust(this_pos, water, arg10);
        } else {
            slidevel = slide_vel_rows[idx as usize];
            let nz = hits[idx as usize].normal[2];
            if f(nz) > f(COS50) || f(nz).is_nan() {
                *this_flags &= !GROUNDED;
                water_adjust(this_pos, water, arg10);
            } else {
                let mut vel = hits[idx as usize].normal;
                let code = step_up_resolve(
                    this_pos,
                    this_radius,
                    this_height,
                    this_flags,
                    this_floor,
                    h,
                    obj_pred,
                    [ebp_3c, ebp_38],
                    &mut vel,
                    hits,
                    hit_count,
                    &mut sweep,
                    &mut fall_land,
                );
                if code == 0 {
                    return budget_ret;
                }
                if code == 1 {
                    settled_flag = 1;
                } else {
                    *this_flags |= GROUNDED;
                }
                water_adjust(this_pos, water, arg10);
            }
        }
    }

    if idx >= hit_count || settled_flag != 0 {
        seam.start_falling = true;
        return budget_ret;
    }
    if facing_flag != 0 {
        seam.begin_move = true;
        seam.set_facing = Some(slidevel);
        return budget_ret;
    }

    if !water {
        seam.set_facing = Some(slidevel);
        return budget_ret;
    }

    let r = sweep(&[0.0, 0.0, -ONE], ONE, this_pos);
    if !r.contact {
        return budget_ret;
    }
    let up_t = r.t;
    let idx2 = r.idx;
    if idx2 >= hit_count {
        slidevel = [0.0, 0.0];
    } else {
        let nz = hits[idx2 as usize].normal[2];
        let gate = if obj_pred { COS50 } else { COS80 };

        if f(nz) <= f(gate) && !f(nz).is_nan() {
            seam.start_falling = true;
            return budget_ret;
        }
        slidevel = slide_vel_rows[idx2 as usize];
    }

    if !(f(up_t) < 1.0) {
        seam.set_facing = Some(slidevel);
        return budget_ret;
    }

    let mut arg10 = (f(swim_terminal) * (f(budget_f) * f(MS_TO_S)) + f(water_probe_t)) as f32;
    let t1 = f(ONE) - f(up_t);
    if f(arg10) < 0.0 {
        arg10 = 0.0;
    } else if !(f(arg10) < t1) {
        arg10 = t1 as f32;
    }

    let r = sweep(&[0.0, 0.0, ONE], arg10, this_pos);
    if !r.contact {
        return budget_ret;
    }
    arg10 = r.t;
    this_pos[2] = (f(arg10) + f(this_pos[2])) as f32;
    seam.set_facing = Some(slidevel);
    budget_ret
}

/// `0x636e74` in `CMovement::ResolveWalk` (`0x6367b0`): the move's adjustment in water.
#[inline]
fn water_adjust(this_pos: &mut [f32; 3], water: bool, arg10: f32) {
    if water {
        this_pos[2] = (f(arg10) + f(this_pos[2])) as f32;
        let t = f(arg10) - f(ONE);
        if !(t < 0.0) {
            this_pos[2] = (f(this_pos[2]) - t) as f32;
        }
    }
}
