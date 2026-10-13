//! The resolver's substep loop (`systems/collision/resolve.md#substep-loop`): the query, the
//! dispatch to the walk, fall, airborne or free-move resolver, and the commit. The world query and
//! the callees with effects outside the mover are passed in, and their calls are recorded.

#![allow(clippy::neg_cmp_op_on_partial_ord)]
#![allow(clippy::too_many_arguments)]

use crate::resolve_clip::HitRecord;
use crate::resolve_integrator::{IntegSeam, IntegState, SubstepSweep};
use crate::resolve_modes::{fall_mode_resolve, transport_mode_resolve, TransportSweep};
use crate::resolve_multipass::SweepRes;
use crate::resolve_query::{swept_world_query, Transport};
use crate::resolve_walkmode::{resolve_walk_mode, WalkSeam};

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x801360]`: 0.001.
const MS_TO_S: f32 = f32::from_bits(0x3a83_126f);

/// `[0x8026bc]`: 2^-20.
const NZ_GATE: f32 = f32::from_bits(0x3580_0000);

/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);

/// The mover fields `0x634040` reads and writes.
#[derive(Clone, Copy, Debug, Default)]
pub struct EntryState {
    pub integ: IntegState,
    pub height: f32,
    pub floor: f32,
    pub guid_lo: u32,
    pub guid_hi: u32,
    pub support_pt: [f32; 3],
    pub transport_obj_flags: Option<u32>,
}

impl EntryState {
    #[inline]
    fn guid_zero(&self) -> bool {
        (self.guid_lo | self.guid_hi) == 0
    }
}

/// The callees `0x634040` reaches, as the caller passes them.
pub struct Boundary<'a> {
    pub trace_hit: bool,
    pub coll_height: f32,
    pub fall_dist: f32,
    pub obj_pred: bool,
    pub swim_terminal: f32,
    pub transport: Option<&'a Transport>,
    pub hit_count: u32,
    pub liquid_count: u32,
    pub slide_vel_rows: &'a [[f32; 2]],
    pub feas_this_c: u32,
    pub feas_pred: bool,
}

/// The calls `0x634040` makes whose effects lie outside the mover, recorded in order.
#[derive(Clone, Debug, Default)]
pub struct EntrySeam {
    pub walk: WalkSeam,
    pub free_advance: Vec<i32>,
    pub broadcast: Vec<[u32; 5]>,
    pub set_active_speed: u32,
    pub support_update: bool,
    pub commit: bool,
}

/// `CMovement::ResolveDisplacement` (`0x634040`, `systems/collision/resolve.md#substep-loop`):
/// resolves a displacement over ms against the world in substeps and returns the milliseconds
/// resolved.
pub fn resolve_entry<WS, WF, FS, TS>(
    st: &mut EntryState,
    base_pos: i32,
    budget_ms: i32,
    disp: &[f32; 3],
    b: &Boundary,
    main_recs: &mut [HitRecord],
    liquid_recs: &mut [HitRecord],
    seam: &mut EntrySeam,
    mut walk_sweep: WS,
    mut walk_fall_land: WF,
    mut fall_sweep: FS,
    mut transport_sweep: TS,
) -> u32
where
    WS: FnMut(&[f32; 3], f32, &[f32; 3]) -> SweepRes,
    WF: FnMut(&[f32; 2], &[f32; 2], f32, &[f32; 3]) -> u32,
    FS: FnMut(&[f32; 3], &[f32; 3], f32) -> SubstepSweep,
    TS: FnMut(&[f32; 3], &[f32; 3], f32, bool) -> TransportSweep,
{
    if budget_ms == 0 {
        return 0;
    }

    let len =
        ((f(disp[0]) * f(disp[0]) + f(disp[1]) * f(disp[1])) + f(disp[2]) * f(disp[2])).sqrt();

    let budget_f64 = f64::from(budget_ms as u32);
    let speed = (len / (budget_f64 * f(MS_TO_S))) as f32;

    let mut unit = [0.0f32; 3];
    if !(len.abs() < f(NZ_GATE)) {
        let inv = f(ONE) / len;
        unit = [
            (inv * f(disp[0])) as f32,
            (inv * f(disp[1])) as f32,
            (inv * f(disp[2])) as f32,
        ];
    }

    let mut consumed: i32 = 0;
    let mut blocked_free_advance = false;

    while st.integ.flags & 0x200f != 0 {
        let remaining = budget_ms.wrapping_sub(consumed);
        let offset = base_pos.wrapping_add(consumed);

        let substep_dist = ((f64::from(remaining as u32) * f(MS_TO_S)) * f(speed)) as f32;

        let mut q_dir = unit;
        let (contact, _scr) = swept_world_query(
            st.integ.pos,
            st.integ.radius,
            st.height,
            st.integ.flags,
            st.guid_zero(),
            substep_dist,
            b.fall_dist,
            b.coll_height,
            &mut q_dir,
            b.trace_hit,
            b.transport,
            main_recs,
            liquid_recs,
        );

        if !contact {
            seam.free_advance.push(remaining);
            st.integ.f58 = st.integ.f58.wrapping_sub(remaining);
            consumed = budget_ms;
            blocked_free_advance = true;
            break;
        }

        let flags = st.integ.flags;
        let falling = flags & 0x2000 != 0;

        let fall_up: u32 = if falling && (f(st.integ.fa0) != 0.0 || f(st.integ.fa0).is_nan()) {
            1
        } else {
            0
        };
        let saved_guid = (st.guid_lo, st.guid_hi);
        let saved_speed = st.integ.f84;
        let gz = st.guid_zero();

        let obj_route = match st.transport_obj_flags {
            Some(of) => of & 4 == 0 && of & 0x200 != 0,
            None => false,
        };
        let transport_dispatch = obj_route || flags & 0x0020_0800 != 0;

        let advanced: u32 = if transport_dispatch {
            let mut t_dir = unit;
            transport_mode_resolve(
                &mut st.integ.pos,
                flags,
                st.transport_obj_flags,
                gz,
                b.feas_this_c,
                b.feas_pred,
                b.hit_count,
                b.liquid_count,
                remaining,
                substep_dist,
                &mut t_dir,
                &mut transport_sweep,
            )
        } else if falling {
            let f_dir = [unit[0], unit[1], 0.0];
            let integseam = IntegSeam::default();
            fall_mode_resolve(
                &mut st.integ,
                &mut st.support_pt,
                gz,
                b.feas_this_c,
                b.feas_pred,
                b.hit_count,
                offset,
                remaining,
                substep_dist,
                &f_dir,
                b.fall_dist,
                b.obj_pred,
                &integseam,
                &mut fall_sweep,
            )
        } else {
            resolve_walk_mode(
                &mut st.integ.pos,
                st.integ.radius,
                st.height,
                &mut st.integ.flags,
                &mut st.floor,
                remaining,
                substep_dist,
                [unit[0], unit[1]],
                b.coll_height,
                b.obj_pred,
                b.swim_terminal,
                main_recs,
                b.hit_count,
                b.slide_vel_rows,
                &mut seam.walk,
                &mut walk_sweep,
                &mut walk_fall_land,
            )
        };

        consumed = consumed.wrapping_add(advanced as i32);

        let moved: u32 = u32::from(saved_guid.0 != st.guid_lo || saved_guid.1 != st.guid_hi);
        let remaining_after = budget_ms.wrapping_sub(consumed);

        seam.broadcast.push([
            offset.wrapping_add(advanced as i32) as u32,
            remaining_after as u32,
            flags,
            fall_up,
            moved,
        ]);

        let speed_check = match st.transport_obj_flags {
            None => true,
            Some(of) => of & 4 != 0,
        };
        if speed_check && (f(st.integ.f84) != f(saved_speed) || f(st.integ.f84).is_nan()) {
            seam.set_active_speed += 1;
            break;
        }

        if st.integ.f58 == 0 && st.integ.flags & 0xf != 0 {
            st.integ.f58 = remaining_after;
        }

        if consumed >= budget_ms {
            break;
        }
    }

    let skip_support = matches!(st.transport_obj_flags, Some(of) if of & 4 == 0 && of & 0x200 != 0);
    if !skip_support {
        seam.support_update = true;
    }

    seam.commit = true;
    let _ = blocked_free_advance;
    consumed as u32
}
