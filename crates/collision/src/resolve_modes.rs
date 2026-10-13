//! The fall step (`systems/collision/walking.md#fall-step`) and the free-move resolver
//! (`systems/collision/resolve.md#free-move-resolver`). The sweeps, the world query and the callees
//! with effects outside the mover are passed in.

#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]

use crate::resolve_integrator::{fall_integrator, IntegSeam, IntegState, SubstepSweep};
use crate::resolve_state::reflect_velocity;
use crate::resolve_state::{ground_land_snap, LandSnapState};
use crate::resolve_walk::feasibility;

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x8026bc]`: 2^-20.
const NZ_GATE: f32 = f32::from_bits(0x3580_0000);
/// `[0x8029d4]`: 2^-22.
const PAR_GUARD: f32 = f32::from_bits(0x3480_0000);
/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);
/// 1.0, the support normal's z that `+0x2c` is reset to.
const SUPPORT_UP: f32 = f32::from_bits(0x3f80_0000);

/// `CMovement::ResolveFall` (`0x635b00`, `systems/collision/walking.md#fall-step`): moves a falling
/// mover along its fall curve for ms, freely or through 0x635600, and returns the ms consumed.
pub fn fall_mode_resolve<F>(
    st: &mut IntegState,
    support_pt: &mut [f32; 3],
    guid_zero: bool,
    feas_this_c: u32,
    feas_pred: bool,
    hit_count: u32,
    base_pos: i32,
    dt_ms: i32,
    substep_dist: f32,
    dir: &[f32; 3],
    fall_dist: f32,
    obj_pred: bool,
    seam: &IntegSeam,
    sweep_fn: F,
) -> u32
where
    F: FnMut(&[f32; 3], &[f32; 3], f32) -> SubstepSweep,
{
    if !guid_zero && feasibility(feas_this_c, feas_pred) {
        return 0;
    }

    if dt_ms == 0 {
        return 0;
    }

    let dx = (f(substep_dist) * f(dir[0])) as f32;
    let dy = (f(substep_dist) * f(dir[1])) as f32;
    let neg_fd = -f(fall_dist);
    let neg_fd_f32 = neg_fd as f32;

    let fd_sq = neg_fd * f(neg_fd_f32);
    let mag = ((fd_sq + f(dy) * f(dy)) + f(dx) * f(dx)).sqrt();

    if mag.abs() < f(PAR_GUARD) {
        st.f78 = st.f78.wrapping_add(dt_ms);
        return dt_ms as u32;
    }

    let eax = if hit_count == 0 {
        st.pos[0] = (f(dx) + f(st.pos[0])) as f32;
        st.pos[1] = (f(dy) + f(st.pos[1])) as f32;
        st.pos[2] = (f(neg_fd_f32) + f(st.pos[2])) as f32;
        st.f78 = st.f78.wrapping_add(dt_ms);

        let mut ls = LandSnapState {
            flags: st.flags,
            cur_z: st.pos[2],
            fall_accum: st.f78 as u32,
            fall_start_z: st.f7c,
            fall_vel: st.fa0,
        };
        ground_land_snap(&mut ls);
        st.flags = ls.flags;
        dt_ms as u32
    } else {
        fall_integrator(
            st,
            base_pos,
            dt_ms,
            [dx, dy, neg_fd_f32],
            1,
            obj_pred,
            seam,
            sweep_fn,
        )
    };

    *support_pt = [0.0, 0.0, SUPPORT_UP];
    eax
}

/// What a sweep `0x632ba0` gives back.
#[derive(Clone, Copy, Default)]
pub struct TransportSweep {
    pub contact: bool,
    pub idx: u32,
    pub t: f32,
    pub hit_normal: [f32; 3],
}

/// `CMovement::ResolveFreeMove` (`0x634640`, `systems/collision/resolve.md#free-move-resolver`):
/// moves a swimmer, flag-0x800 mover or flying spline along a direction, sliding on the world and
/// the flipped liquid list.
pub fn transport_mode_resolve<F>(
    pos: &mut [f32; 3],
    this_flags: u32,
    transport_obj_flags: Option<u32>,
    guid_zero: bool,
    feas_this_c: u32,
    feas_pred: bool,
    primary_count: u32,
    liquid_count: u32,
    budget: i32,
    substep_dist: f32,
    dir: &mut [f32; 3],
    mut sweep_fn: F,
) -> u32
where
    F: FnMut(&[f32; 3], &[f32; 3], f32, bool) -> TransportSweep,
{
    if !guid_zero && feasibility(feas_this_c, feas_pred) {
        return 0;
    }
    if budget == 0 {
        return 0;
    }

    if f(substep_dist).abs() < f(NZ_GATE) {
        return budget as u32;
    }

    let budget_f = (f64::from(budget as u32)) as f32;

    let mut cur = sweep_fn(pos, dir, substep_dist, false);
    if !cur.contact {
        return budget as u32;
    }

    let mut consumed: f32 = 0.0;
    let mut iter_counter: i32 = 0;
    let mut sweep24: i32 = 0;
    let mut cur_dist = substep_dist;
    let mut sweep_len = substep_dist;

    loop {
        let mut idx = cur.idx;
        let mut out_t = cur.t;
        let mut hit_normal = cur.hit_normal;

        if idx < primary_count {
            sweep24 = 1;
        } else {
            let block_count;

            let liquid_pass = if sweep24 != 0 {
                false
            } else {
                let obj_skip = match transport_obj_flags {
                    Some(of) => of & 4 == 0 && of & 0x200 != 0,
                    None => false,
                };
                if obj_skip {
                    false
                } else {
                    this_flags & 0x800 == 0
                }
            };

            if liquid_pass {
                let ls = sweep_fn(pos, dir, sweep_len, true);
                if !ls.contact {
                    return budget as u32;
                }
                idx = ls.idx;
                out_t = ls.t;
                hit_normal = ls.hit_normal;
                block_count = liquid_count;
            } else {
                block_count = primary_count;
            }

            if idx == block_count {
                let dzt = (f(dir[2]) * f(out_t)) as f32;
                pos[0] = ((f(dir[0]) * f(out_t)) + f(pos[0])) as f32;
                pos[1] = ((f(dir[1]) * f(out_t)) + f(pos[1])) as f32;
                pos[2] = (f(dzt) + f(pos[2])) as f32;
                return budget as u32;
            }
        }

        let dzt = (f(dir[2]) * f(out_t)) as f32;
        pos[0] = ((f(dir[0]) * f(out_t)) + f(pos[0])) as f32;
        pos[1] = ((f(dir[1]) * f(out_t)) + f(pos[1])) as f32;
        pos[2] = (f(dzt) + f(pos[2])) as f32;

        let frac = f(out_t) / f(cur_dist);
        let cons_incr = (f(budget_f) - f(consumed)) * frac;
        cur_dist = (f(cur_dist) - f(out_t)) as f32;

        if f(NZ_GATE) + cons_incr > f(ONE) {
            consumed = (cons_incr + f(consumed)) as f32;
            iter_counter = 1;
        } else {
            iter_counter += 1;
            if iter_counter > 5 {
                return budget as u32;
            }
        }

        if f(budget_f) - f(consumed) < f(ONE) {
            return budget as u32;
        }

        let reflect = reflect_velocity(&hit_normal, dir, cur_dist);
        let dxr = f(dir[0]) * f(cur_dist);
        let dyr = f(dir[1]) * f(cur_dist);
        let dzr = f(dir[2]) * f(cur_dist);
        let ndx = (dxr + f(reflect[0])) as f32;
        let ndy = (f(reflect[1]) + dyr) as f32;
        let ndz = (f(reflect[2]) + dzr) as f32;

        *dir = [ndx, ndy, ndz];
        let len = ((f(ndz) * f(ndz) + f(ndy) * f(ndy)) + f(ndx) * f(ndx)).sqrt() as f32;
        cur_dist = len;

        if f(len).abs() < f(NZ_GATE) {
            return budget as u32;
        }
        let inv = f(ONE) / f(len);
        dir[0] = (f(ndx) * inv) as f32;
        dir[1] = (f(ndy) * inv) as f32;
        dir[2] = (f(ndz) * inv) as f32;

        let ss = sweep_fn(pos, dir, len, false);
        if !ss.contact {
            return budget as u32;
        }
        cur = ss;
        sweep_len = len;
    }
}
