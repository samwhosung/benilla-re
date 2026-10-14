//! The camera's clearance solver (`systems/uicamera/solver.md`): the near box clip and the solver
//! that shortens the orbit distance and the pivot height until they clear the world.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fst`/`fstp dword` is an
//! `as f32`. The swept world queries (`0x672170`, `0x686db0`) are inputs.

mod c {
    /// 0.001 at `0x801360`.
    pub const DT: f32 = f32::from_bits(0x3a83_126f);
    /// 1/9 at `0x808a04`: the back-off after a hit, and the lower bound of a shortened height.
    pub const NINTH: f32 = f32::from_bits(0x3de3_8e39);
    /// 5/6 at `0x8089cc`: the base height above the pivot, and the lowest cap.
    pub const FIVE_SIXTHS: f32 = f32::from_bits(0x3f55_5555);
    /// 2/9 at `0x8089d0`: the surface band's offset.
    pub const TWO_NINTHS: f32 = f32::from_bits(0x3e63_8e39);
    /// 2^-20 at `0x8026bc`: a height or distance within this of the near clip is not probed.
    pub const EPS_DIST: f32 = f32::from_bits(0x3580_0000);
    /// 2^-22 at `0x8029d4`: a shortening of at least this triggers the back-off.
    pub const EPS_BACK: f32 = f32::from_bits(0x3480_0000);
}

/// What `CCamera::ClipNearBox` (`0x50de90`) returns: the distance after the clip and `eax`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipResult {
    /// The new `*distance`, stored as `f32`.
    pub dist: f32,
    /// 1 for the early exit or a clip, 0 when no volume was touched.
    pub ret: u32,
}

/// `CCamera::ClipNearBox` (`0x50de90`): shortens the distance so the near end of the view volume
/// clears geometry (`solver#the-near-box-clip`).
///
/// `delta = dist - near_clip` (`this+0x38`) is stored as `f32`, but the early exit compares the
/// register value: `delta < 0.001` sets `*dist = 0` and returns 1. Otherwise the swept query
/// (`0x686db0`) lists the contacts' fractions (`+8` of each); `max_frac` is their running maximum
/// from 0, replaced only when strictly smaller, so a NaN never enters. If a volume was touched
/// (`hit`), `*dist = dist - max_frac * delta` is stored as `f32` and, when that register value is
/// negative, replaced by 0; the return is 1. Without a hit `*dist` is unchanged and the return is
/// 0. The box setup between the early exit and the query changes none of these.
///
/// The clip tests the register value of the new distance (`fcomp 0.0` after `fst` without a pop,
/// `0x50e48d` and `0x50e48f`) and stores the rounded one, so a result in (-2^-150, 0) is stored as
/// -0.0 and not replaced.
pub fn clip_near_box(dist: f32, near_clip: f32, contacts: &[f32], hit: bool) -> ClipResult {
    let delta64 = f64::from(dist) - f64::from(near_clip);
    if delta64 < f64::from(c::DT) {
        return ClipResult { dist: 0.0, ret: 1 };
    }
    let delta_f32 = delta64 as f32;
    let mut maxfrac = 0.0f32;
    for &f in contacts {
        if maxfrac < f {
            maxfrac = f;
        }
    }
    if hit {
        let new_live = f64::from(dist) - f64::from(maxfrac) * f64::from(delta_f32);
        let mut new = new_live as f32;
        if new_live < 0.0 {
            new = 0.0;
        }
        ClipResult { dist: new, ret: 1 }
    } else {
        ClipResult { dist, ret: 0 }
    }
}

/// One swept query of the solver (`0x672170`): whether it touched anything and the fraction of the
/// segment it leaves (1.0 when it touched nothing).
#[derive(Clone, Copy, Debug)]
pub struct Sweep {
    /// The query's `char` return; nonzero means it touched something.
    pub hit: bool,
    /// The fraction written back through the query's out parameter, in `[0, 1]`.
    pub frac: f32,
}

/// The camera words `CCamera::SolveLimits` reads.
#[derive(Clone, Copy, Debug)]
pub struct SolverFields {
    /// The distance target, `+0x198`.
    pub distance_target: f32,
    /// The live distance, `+0xec`.
    pub distance_live: f32,
    /// The height target, `+0x1c8`.
    pub height_target: f32,
    /// The live pivot height, `+0xfc`.
    pub height_live: f32,
    /// The near clip, `+0x38`.
    pub near_clip: f32,
    /// The mode word, `+0x90`: 0x100000 is the surface band and 0x200000 the submerged band.
    pub mode: u32,
}

/// What `CCamera::SolveLimits` stores and returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolveResult {
    /// `*distance`: the largest distance that clears the world.
    pub distance: f32,
    /// `*height`: the largest pivot height that clears the world.
    pub height: f32,
    /// `eax`: 0x20000 when the vertical probe hit, 0x10000 when the boom sweep or the clip did.
    pub mask: u32,
}

/// `CCamera::SolveLimits` (`0x50e570`): the largest distance and pivot height that clear the world
/// (`solver#the-solver`, `solver#the-boom-sweep`).
///
/// Both start from the larger of target and live (`distance_target` and `distance_live`;
/// `height_target` and `height_live`). The height is solved first. The trace mask is 0x100171 plus
/// 0xf0000 with `water_collision`; `surface` is `d`, the height of the liquid above the unit
/// (`0x511ad0`). If `height - near_clip > 2^-20`:
///
/// - with `water_collision`, a surface band (mode 0x100000) sets the base `local8`, 5/6 by default,
///   to `(2/9 + surface)` as `f32` and raises the cap `local10`, the live height by default, to at
///   least it; a submerged band (0x200000) sets the cap to `(surface - 5/6)` as `f32`, or to 5/6
///   when that is not above 5/6;
/// - `l14 = (height - local8)` as `f32`, at least 1/9; a vertical probe that hit scales it by its
///   fraction and sets 0x20000; a shortening of at least 2^-22 backs off 1/9 (not below 1/9);
///   then `height = local8 + height`.
///
/// The height is then clamped to `[local8, local10]`. For the distance: if
/// `distance - near_clip > 2^-20`, a boom sweep that hit scales it by its fraction and sets
/// 0x10000; if it is still above 2^-20, `boom_clip` (the outcome of `PlaceEye` and `ClipNearBox`
/// on the boom) replaces it, setting 0x10000 when it returned 1, and a shortening of at least 2^-22
/// backs off 1/9 (not below 0). Last, the distance is limited to `distance_live`.
#[allow(clippy::too_many_arguments)]
pub fn solve_limits(
    fields: SolverFields,
    surface: f32,
    water_collision: bool,
    height_sweep: Sweep,
    boom_sweep: Sweep,
    boom_clip: ClipResult,
) -> SolveResult {
    let mut mask: u32 = 0;
    let mut distance = if fields.distance_target <= fields.distance_live {
        fields.distance_live
    } else {
        fields.distance_target
    };
    let mut height = if fields.height_target <= fields.height_live {
        fields.height_live
    } else {
        fields.height_target
    };

    let trace_mask: u32 = if water_collision { 0xf0000 } else { 0 } + 0x100171;
    let mut local8 = f64::from(c::FIVE_SIXTHS);
    let mut local10 = f64::from(fields.height_live);

    // The height.
    if f64::from(c::EPS_DIST) < f64::from(height) - f64::from(fields.near_clip) {
        if (trace_mask & 0xf0000) != 0 {
            if (fields.mode & 0x100000) == 0 {
                if (fields.mode & 0x200000) != 0 {
                    // 0x50e666 to 0x50e684: the cap is stored as `f32`, but the compare with 5/6
                    // reads the register value.
                    let cand = f64::from(surface) - f64::from(c::FIVE_SIXTHS);
                    local10 = f64::from(cand as f32);
                    if cand <= f64::from(c::FIVE_SIXTHS) {
                        local10 = f64::from(c::FIVE_SIXTHS);
                    }
                }
            } else {
                // 0x50e63e: the base is stored as `f32`; the compare reloads it.
                local8 = f64::from((f64::from(c::TWO_NINTHS) + f64::from(surface)) as f32);
                if local10 <= local8 {
                    local10 = local8;
                }
            }
        }
        let mut l14 = (f64::from(height) - local8) as f32;
        if f64::from(l14) <= f64::from(c::NINTH) {
            l14 = c::NINTH;
        }
        height = l14;
        if height_sweep.hit {
            mask = 0x20000;
            height = (f64::from(height_sweep.frac) * f64::from(height)) as f32;
        }
        if f64::from(c::EPS_BACK) <= (f64::from(l14) - f64::from(height)).abs() {
            let mut v = (f64::from(height) - f64::from(c::NINTH)) as f32;
            if f64::from(v) <= f64::from(c::NINTH) {
                v = c::NINTH;
            }
            height = v;
        }
        height = (local8 + f64::from(height)) as f32;
    }
    if f64::from(height) < local8 {
        height = local8 as f32;
    }
    if local10 < f64::from(height) {
        height = local10 as f32;
    }

    // The distance.
    let distance_before = distance;
    if f64::from(c::EPS_DIST) < f64::from(distance) - f64::from(fields.near_clip) {
        if boom_sweep.hit {
            mask |= 0x10000;
            distance = (f64::from(boom_sweep.frac) * f64::from(distance)) as f32;
        }
        if f64::from(c::EPS_DIST) < f64::from(distance) {
            if boom_clip.ret != 0 {
                mask |= 0x10000;
            }
            distance = boom_clip.dist;
            if f64::from(c::EPS_BACK) <= (f64::from(distance_before) - f64::from(distance)).abs() {
                let mut v = (f64::from(distance) - f64::from(c::NINTH)) as f32;
                if f64::from(v) <= 0.0 {
                    v = 0.0;
                }
                distance = v;
            }
        }
    }
    if f64::from(fields.distance_live) < f64::from(distance) {
        distance = fields.distance_live;
    }

    SolveResult {
        distance,
        height,
        mask,
    }
}
