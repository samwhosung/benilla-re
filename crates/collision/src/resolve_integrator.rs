//! The airborne resolver loop (`systems/collision/airborne.md#airborne-resolver-loop`) and the
//! fall-away probe (`systems/collision/walking.md#fall-away-lookahead`). The sweeps, the world
//! query and the callees with effects outside the mover are passed in, and their calls are
//! recorded.

#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]

use crate::resolve_clip::HitRecord;
use crate::resolve_fallstep::fall_step;
use crate::resolve_state::{ground_land_snap, LandSnapState};
use crate::resolve_substep::{walk_slide_substep, SweepOut};
use crate::resolve_toolkit::Plane;

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x801360]`: 0.001.
const MS_TO_S: f32 = f32::from_bits(0x3a83_126f);
/// `[0x8022cc]`: 1000.0.
const THOUSAND: f32 = f32::from_bits(0x447a_0000);
/// `[0x7ffa24]`: 0.5.
const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x8026bc]`: 2^-20.
const NZ_GATE: f32 = f32::from_bits(0x3580_0000);
/// `[0x80c698]`: 0.0005.
const EPS: f32 = f32::from_bits(0x3a03_126f);
/// `[0x80e018]`: 1.1866661.
const SLOPE: f32 = f32::from_bits(0x3f97_e4ad);
/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x801620]`: 0.1.
const BIAS_01: f32 = f32::from_bits(0x3dcc_cccd);
/// `[0x80e020]`: 1/g.
const INV_G: f32 = f32::from_bits(0x3d54_536a);
/// `[0x80e01c]`: cos 10°.
const COS10: f32 = f32::from_bits(0x3f7c_1c5c);

/// `__ftol` (`0x40a2b0`, `systems/camera.md#angle-properties`): converts st0 to a 64-bit integer by
/// truncation and returns its low dword.
fn ftol_round(x: f64) -> i32 {
    let biased = if x > 0.0 { x + f(HALF) } else { x - f(HALF) };
    benilla_re_cmath::x87::ftol(biased)
}

/// The mover fields `0x635600` reads and writes.
#[derive(Clone, Copy, Debug, Default)]
pub struct IntegState {
    pub pos: [f32; 3],
    pub flags: u32,
    pub f58: i32,
    pub support: [f32; 3],
    pub f68: f32,
    pub f6c: f32,
    pub f78: i32,
    pub f7c: f32,
    pub f84: f32,
    pub fa0: f32,
    pub radius: f32,
    pub vmax_land: f32,
    pub vmax_swim: f32,
}

/// The calls `0x635600` makes whose effects lie outside the mover, recorded in order.
#[derive(Clone, Copy, Debug, Default)]
pub struct IntegSeam {
    pub not_falling_ret: u32,
}

/// What a sweep `0x632ba0` gives back.
#[derive(Clone, Copy)]
pub struct SubstepSweep {
    pub sweep: SweepOut,
    pub manifold: [Plane; 7],
    pub hit: HitRecord,
    pub row: [f32; 2],
}

enum FinSource {
    Dts,
    Consumed,
}

/// `CMovement::ResolveAirborneMove` (`0x635600`,
/// `systems/collision/airborne.md#airborne-resolver-loop`): resolves a displacement holding a fall
/// against the world in steps and returns the milliseconds used.
pub fn fall_integrator<F>(
    st: &mut IntegState,
    base_pos: i32,
    dt_ms: i32,
    delta_in: [f32; 3],
    flag: i32,
    obj_pred: bool,
    seam: &IntegSeam,
    mut sweep_fn: F,
) -> u32
where
    F: FnMut(&[f32; 3], &[f32; 3], f32) -> SubstepSweep,
{
    let dt_s = (f64::from(dt_ms as u32) * f(MS_TO_S)) as f32;
    let fallaccum_s = (f64::from(st.f78 as u32) * f(MS_TO_S)) as f32;
    let mut consumed: f32 = 0.0;

    let snap_f84 = st.f84;
    let snap_f68 = st.f68;
    let snap_f6c = st.f6c;
    let snap_support = st.support;
    let start_pos = [st.pos[0], st.pos[1]];

    let mut delta = delta_in;

    let mut remaining = (((f(delta[0]) * f(delta[0]) + f(delta[1]) * f(delta[1]))
        + f(delta[2]) * f(delta[2]))
    .sqrt()) as f32;

    let horiz_len_init = ((f(delta[0]) * f(delta[0]) + f(delta[1]) * f(delta[1])).sqrt()) as f32;
    let dz_init = f(delta[2]).abs() as f32;
    let mut horiz_len = horiz_len_init;

    let mut ebx: u32 = 0;
    let mut begin_move = false;
    let mut strip_flag = false;
    let mut last_flag_b: i32 = 0;
    let mut last_ovb = [0.0f32; 2];

    let fin_source: FinSource;
    let mut do_landreset = false;
    let mut do_abort = false;
    let mut do_damage = false;

    'main: loop {
        if f(remaining).abs() < f(NZ_GATE) {
            fin_source = FinSource::Dts;
            break;
        }

        let inv_l = f(ONE) / f(remaining);
        let dir = [
            (inv_l * f(delta[0])) as f32,
            (inv_l * f(delta[1])) as f32,
            (inv_l * f(delta[2])) as f32,
        ];

        let mut contact_dist = remaining;

        let t_param = (f(fallaccum_s) + f(consumed)) as f32;

        let ss = sweep_fn(&st.pos, &dir, remaining);

        let mut ova = [0.0f32; 2];
        let mut ovb = [0.0f32; 2];
        let mut fa: i32 = 0;
        let mut fb: i32 = 0;
        let eax = walk_slide_substep(
            &st.pos,
            st.radius,
            st.f84,
            st.fa0,
            st.flags,
            obj_pred,
            t_param,
            &mut delta,
            &mut contact_dist,
            &mut ova,
            &mut ovb,
            &mut fa,
            &mut fb,
            &ss.sweep,
            &ss.manifold,
            &ss.hit,
            ss.row,
        );
        last_flag_b = fb;
        last_ovb = ovb;

        let remdt = f(dt_s) - f(consumed);

        if eax == 2 {
            do_damage = true;
            fin_source = FinSource::Dts;
            break;
        }

        let stcons: f64;
        if eax == 1 {
            let mut out_len = contact_dist;
            let fs = fall_step(
                st.fa0,
                st.flags,
                st.vmax_land,
                st.vmax_swim,
                st.f7c,
                st.pos[2],
                st.f84,
                remdt as f32,
                t_param,
                f32::from_bits(fb as u32),
                horiz_len,
                &mut delta,
                &mut out_len,
            );
            contact_dist = out_len;
            stcons = fs.consumed;
        } else {
            stcons = remdt;
        }

        st.pos[0] = (f(delta[0]) + f(st.pos[0])) as f32;
        st.pos[1] = (f(delta[1]) + f(st.pos[1])) as f32;
        st.pos[2] = (f(delta[2]) + f(st.pos[2])) as f32;
        consumed = (f(consumed) + stcons) as f32;

        if last_flag_b != 0 {
            do_landreset = true;
            fin_source = FinSource::Consumed;
            break;
        }
        if fa != 0 {
            do_abort = true;
            fin_source = FinSource::Consumed;
            break;
        }

        if !(f(consumed) + f(EPS) < f(dt_s)) {
            fin_source = FinSource::Dts;
            break;
        }

        remaining = (f(remaining) - f(contact_dist)) as f32;
        begin_move = true;

        if !(stcons > f(EPS)) {
            ebx += 1;
            if ebx > 5 {
                if strip_flag {
                    do_abort = true;
                    fin_source = FinSource::Consumed;
                    break;
                }
                if f(horiz_len).abs() < f(NZ_GATE) {
                    do_abort = true;
                    fin_source = FinSource::Consumed;
                    break;
                }

                st.f84 = 0.0;
                let dz = (f(dir[2]) * f(remaining)) as f32;
                delta = [0.0, 0.0, dz];
                ebx = 0;
                strip_flag = true;
                remaining = f(dz).abs() as f32;
                continue 'main;
            }
        } else {
            ebx = 1;
        }

        let rdx = (f(dir[0]) * f(remaining) + f(ova[0])) as f32;
        let rdy = (f(dir[1]) * f(remaining) + f(ova[1])) as f32;
        let rdz = (f(dir[2]) * f(remaining)) as f32;
        delta = [rdx, rdy, rdz];

        if ebx > 1 {
            let dpx = f(st.pos[0]) - f(start_pos[0]);
            let dpy = f(st.pos[1]) - f(start_pos[1]);

            let a = (dpx + f(delta[0])) as f32;
            let b = dpy + f(delta[1]);
            let h = (b * b + f(a) * f(a)).sqrt();

            if h > f(horiz_len_init) + f(NZ_GATE) && h > f(dz_init) * f(SLOPE) {
                do_abort = true;
                fin_source = FinSource::Consumed;
                break;
            }
        }

        let (dx, dy, dz) = (delta[0], delta[1], delta[2]);
        let lensq_xy = f(dy) * f(dy) + f(dx) * f(dx);
        let horiz_new = lensq_xy.sqrt();
        horiz_len = horiz_new as f32;
        let (nx, ny);
        if horiz_new.abs() < f(NZ_GATE) {
            nx = dx;
            ny = dy;
            st.f84 = 0.0;
        } else {
            let inv_h = f(ONE) / f(horiz_len);
            let nxv = (f(dx) * inv_h) as f32;
            let ny53 = inv_h * f(dy);
            let nyv = ny53 as f32;

            let f84val = ((ny53 * f(st.f6c)) + (f(nxv) * f(st.f68))) * f(st.f84);
            st.f84 = f84val as f32;
            if f84val < 0.0 {
                st.f84 = 0.0;
            }
            nx = nxv;
            ny = nyv;
        }
        st.f68 = nx;
        st.f6c = ny;
        st.support = [nx, ny, 0.0];
        remaining = ((lensq_xy + f(dz) * f(dz)).sqrt()) as f32;

        continue 'main;
    }

    if do_damage {
        let dmg = ftol_round((f(dt_s) - f(consumed)) * f(THOUSAND));
        st.f58 = st.f58.wrapping_sub(dmg);
        st.f78 = st.f78.wrapping_sub(dmg);
    }
    // `CMovement::RestartFallAtRest` (`0x633220`) runs here when `do_landreset` is set and
    // `CMovement::StopFalling` (`0x7c6290`) when `do_abort` is; the caller supplies both.
    let _ = (do_landreset, do_abort);

    let x = match fin_source {
        FinSource::Dts => f(dt_s),
        FinSource::Consumed => f(consumed),
    };
    let edi = ftol_round(x * f(THOUSAND));

    if last_flag_b == 0 {
        st.f78 = st.f78.wrapping_add(edi);
    }

    if flag != 0 {
        if last_flag_b != 0 {
            let _ = edi.wrapping_add(base_pos);
        }

        // A nonzero last push changes nothing on this path.
        let _ = last_ovb;

        if st.flags & 0x2000 == 0 {
            if seam.not_falling_ret != 0 {
                return edi as u32;
            }
        } else {
            let mut ls = LandSnapState {
                flags: st.flags,
                cur_z: st.pos[2],
                fall_accum: st.f78 as u32,
                fall_start_z: st.f7c,
                fall_vel: st.fa0,
            };
            ground_land_snap(&mut ls);
            st.flags = ls.flags;

            if st.flags & 0x2000 != 0 && f(st.fa0) != 0.0 && st.flags & 0xf != 0 {
                st.f84 = snap_f84;
                st.f68 = snap_f68;
                st.f6c = snap_f6c;
                st.support = snap_support;
            }
        }
    }

    let _ = begin_move;
    edi as u32
}

/// `CMovement::ProbeFallAway` (`0x635f80`, `systems/collision/walking.md#fall-away-lookahead`):
/// returns 1 when a simulated fall carries the mover a radius from start, within 10 degrees of dir.
pub fn resolve_fall_land<F>(
    st: &mut IntegState,
    dir: &[f32; 2],
    saved: &[f32; 2],
    drop_height: f32,
    obj_pred: bool,
    seam: &IntegSeam,
    mut sweep_fn: F,
) -> u32
where
    F: FnMut(&[f32; 3], &[f32; 3], f32) -> SubstepSweep,
{
    let h = f(drop_height) + f(BIAS_01);
    let v = (((h + h) * f(INV_G)).sqrt()) * f(THOUSAND);
    let ms = ftol_round(v);

    let snap = (st.f78, st.support, st.f68, st.f6c, st.flags, st.f84);

    let vvec = [st.f68, st.f6c, f32::from_bits(0x8000_0000)];

    let ebxu = ms as u32;
    let mut edi: u32 = 0;
    if ebxu != 0 {
        loop {
            if st.flags & 0x2000 == 0 {
                break;
            }
            let dt = ebxu.wrapping_sub(edi) as i32;
            let eax = fall_integrator(st, 0, dt, vvec, 0, obj_pred, seam, &mut sweep_fn);
            edi = edi.wrapping_add(eax);
            if edi >= ebxu {
                break;
            }
        }
    }

    let (s_f78, s_support, s_f68, s_f6c, s_flags, s_f84) = snap;
    st.f78 = s_f78;
    st.support = s_support;
    st.f68 = s_f68;
    st.f6c = s_f6c;
    st.flags = s_flags;
    st.f84 = s_f84;

    let dpx = (f(st.pos[0]) - f(saved[0])) as f32;
    let dpy53 = f(st.pos[1]) - f(saved[1]);
    let dpy = dpy53 as f32;
    let dist = ((dpy53 * f(dpy)) + f(dpx) * f(dpx)).sqrt();

    if !(dist >= f(st.radius) || dist.is_nan()) {
        return 0;
    }
    let inv = f(ONE) / dist;
    let ndx = f(dpx) * inv;
    let ndy = inv * f(dpy);

    let dot = (f(dir[1]) * ndy) + (ndx * f(dir[0]));
    if dot > f(COS10) {
        1
    } else {
        0
    }
}
