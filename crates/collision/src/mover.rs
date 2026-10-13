//! The per-frame mover's arithmetic (`systems/collision/mover.md`) and the remote mover's facing
//! rate and position blend (`systems/collision/status.md`). Each function takes the fields and the
//! callees' results it reads as arguments and returns what it stores; the list walks and the calls
//! around them are left out.

#![allow(clippy::neg_cmp_op_on_partial_ord, clippy::too_many_arguments)]

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x801360]`: 0.001, milliseconds to seconds.
const C001: f32 = f32::from_bits(0x3a83_126f);
/// `[0x80c734]`: 3600.0, sixty yards per second, squared.
const C3600: f32 = f32::from_bits(0x4561_0000);
/// `[0x80c5bc]`: 9.0, three yards, squared.
const C9: f32 = f32::from_bits(0x4110_0000);
/// 17066.666 (`0x46855555`), 32 × 533.33333.
const HALF_EXT: f32 = f32::from_bits(0x4685_5555);
/// `[0x801628]`: 2.0, the margin inside the world's edge.
const TWO: f32 = f32::from_bits(0x4000_0000);
/// `[0x80c738]`: 34131.332 (`0x47055355`).
const FULL_SPAN: f32 = f32::from_bits(0x4705_5355);
/// `[0x80295c]`: 2π (`0x40c90fdb`).
const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
/// `[0x7ffa24]`: 0.5.
const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x80c740]`: 1.8493990 (`0x3fecb91b`).
const SLOPE: f32 = f32::from_bits(0x3fec_b91b);
/// `[0x80c73c]`: 0.54071623 (`0x3f0a6c61`).
const TARGET: f32 = f32::from_bits(0x3f0a_6c61);
/// `[0x8029d4]`: 2^-22.
const EPS: f32 = f32::from_bits(0x3480_0000);
/// `[0x8030b0]`: π.
const PI: f32 = f32::from_bits(0x4049_0fdb);
/// `[0x80c5b8]`: −π.
const NEG_PI: f32 = f32::from_bits(0xc049_0fdb);
/// `[0x8026bc]`: 2^-20.
const FLOOR: f32 = f32::from_bits(0x3580_0000);
/// `[0x80c744]`: (1/36)² (`0x3a4a4588`).
const TOL: f32 = f32::from_bits(0x3a4a_4588);

/// What `0x616af0` returns and leaves: `eax`, the position and `+0x7c`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AntiWarp {
    pub eax: u32,
    pub pos: [f32; 3],
    pub fall_z: f32,
}

/// `CMovement::GetSplineStep` (`0x616af0`, `systems/collision/mover.md#spline-warp-gate`): with the
/// spline step `delta` that `0x7c5490` returned (`integrate_ok`), returns 1 unless the mover moves
/// (flags & 0x200f) and the spline's `+0x18` lacks 0x200; then `target = anchor + delta`, and `D /
/// T² > 3600` or `D > 9.0` (either true when NaN) stores `target` as the position, refreshes
/// `+0x7c` from `0x7c5f30` when falling, and returns 0.
pub fn anti_warp(
    integrate_ok: bool,
    flags: u32,
    mi_flags: u32,
    base: [f32; 3],
    delta: [f32; 3],
    pos: [f32; 3],
    dt_ms: u32,
    fall_z_pred: f32,
    cur_fall_z: f32,
) -> AntiWarp {
    if !integrate_ok {
        return AntiWarp {
            eax: 0,
            pos,
            fall_z: cur_fall_z,
        };
    }
    if flags & 0x200f == 0 {
        return AntiWarp {
            eax: 1,
            pos,
            fall_z: cur_fall_z,
        };
    }
    if mi_flags & 0x200 != 0 {
        return AntiWarp {
            eax: 1,
            pos,
            fall_z: cur_fall_z,
        };
    }

    let newpos = [
        (f(base[0]) + f(delta[0])) as f32,
        (f(base[1]) + f(delta[1])) as f32,
        (f(base[2]) + f(delta[2])) as f32,
    ];

    let dx = f(newpos[0]) - f(pos[0]);
    let dy = f(newpos[1]) - f(pos[1]);
    let dist2 = dx * dx + dy * dy;
    let dt_s = (i64::from(dt_ms) as f64) * f(C001);
    let speed2 = dist2 / (dt_s * dt_s);

    let snap = !(speed2 <= f(C3600)) || !(dist2 <= f(C9));
    if !snap {
        return AntiWarp {
            eax: 1,
            pos,
            fall_z: cur_fall_z,
        };
    }
    let fall_z = if flags & 0x2000 != 0 {
        fall_z_pred
    } else {
        cur_fall_z
    };
    AntiWarp {
        eax: 0,
        pos: newpos,
        fall_z,
    }
}

/// `CMovement::IsPositionValid` (`0x616bf0`, `systems/collision/mover.md#position-validity`): all
/// three finite, and `17066.666 − y` (unrounded) and `fl(17066.666 − x)` in `[2.0, 34131.332)`.
pub fn pos_valid(pos: [f32; 3]) -> bool {
    if !pos[0].is_finite() || !pos[1].is_finite() || !pos[2].is_finite() {
        return false;
    }

    let ty = f(HALF_EXT) - f(pos[1]);
    if ty < f(TWO) {
        return false;
    }
    if !(ty < f(FULL_SPAN)) {
        return false;
    }

    let tx = (f(HALF_EXT) - f(pos[0])) as f32;
    if f(tx) < f(TWO) {
        return false;
    }
    if !(f(tx) < f(FULL_SPAN)) {
        return false;
    }
    true
}

/// `CMovement::ResolveStep` (`0x616cb0`, `systems/collision/mover.md#substep-apply-and-commit`):
/// `anchor + offset − position`, x and y from the unrounded target and z from the target rounded
/// first; z is 0 unless the spline is live with 0x200 and without 4 in its `+0x18`, or flags &
/// 0x200800.
pub fn build_delta(
    base: [f32; 3],
    off: [f32; 3],
    pos: [f32; 3],
    mi_present: bool,
    mi_flags: u32,
    flags: u32,
) -> [f32; 3] {
    let dx = (f(base[0]) + f(off[0]) - f(pos[0])) as f32;
    let dy = (f(base[1]) + f(off[1]) - f(pos[1])) as f32;
    let newpos_z = (f(base[2]) + f(off[2])) as f32;
    let mut dz = (f(newpos_z) - f(pos[2])) as f32;
    let keep = if mi_present && mi_flags & 4 == 0 && mi_flags & 0x200 != 0 {
        true
    } else {
        flags & 0x0020_0800 != 0
    };
    if !keep {
        dz = 0.0;
    }
    [dx, dy, dz]
}

/// `CMovement::SnapIfFar` (`0x616d60`, `systems/collision/mover.md#snap-beyond-three-yards`):
/// stores `p` as the position unless `(dz² + dx²) + dy² < 9.0`.
pub fn dead_band(newpos: [f32; 3], pos: [f32; 3]) -> (bool, [f32; 3]) {
    let dx = f(newpos[0]) - f(pos[0]);
    let dy = f(newpos[1]) - f(pos[1]);
    let dz = f(newpos[2]) - f(pos[2]);
    let dist2 = (dz * dz + dx * dx) + dy * dy;
    if !(dist2 < f(C9)) {
        (true, newpos)
    } else {
        (false, pos)
    }
}

/// One queued event as `0x617080` reads it: the type `+0xc`, the byte `+0x4c` and the facing
/// `+0x1c`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WrapNode {
    pub typ: u32,
    pub active: u8,
    pub facing: f32,
}

/// `CMovement::RotateQueuedFacings` (`0x617080`,
/// `systems/collision/mover.md#queued-facing-rotation`): for events of type 16 or with `+0x4c` set,
/// `f + facing`, less 2π above 2π, plus 2π below 0, stored as `f32`.
pub fn angle_wrap(arg: f32, nodes: &mut [WrapNode]) {
    for node in nodes.iter_mut() {
        if node.typ != 0x10 && node.active == 0 {
            continue;
        }
        let a = f(arg) + f(node.facing);
        let r = if a > f(TWO_PI) {
            a - f(TWO_PI)
        } else if !(a < 0.0) {
            a
        } else {
            a + f(TWO_PI)
        };
        node.facing = r as f32;
    }
}

/// `CMovement::TestCollisionSize` (`0x617460`, `systems/collision/mover.md#collision-size`): `a =
/// fl(w·s·0.5)` and `b = fl(h·s)`, with `a = fl(s·0.54071623)` when `a·1.8493990 > s`; the pair
/// passed to `0x631670`.
pub fn turn_clamp(a0: f32, a1: f32, a2: f32) -> (f32, f32) {
    let a0p = ((f(a0) * f(a2)) * f(HALF)) as f32;
    let a1p = (f(a1) * f(a2)) as f32;
    let a0p = if f(a0p) * f(SLOPE) > f(a2) {
        (f(a2) * f(TARGET)) as f32
    } else {
        a0p
    };
    (a0p, a1p)
}

/// `CMovement::SetCollisionSize` (`0x6174b0`, `systems/collision/mover.md#collision-size`):
/// `[+0xb0, +0xb4, +0xb8]` = `fl(w·s·0.5)`, `fl(h·s)`, and 1.0 when `|d| < 2^-22`, else `fl(s/d)`.
pub fn param_set(a0: f32, a1: f32, a2: f32, a3: f32) -> [f32; 3] {
    let b8 = if f(a3).abs() < f(EPS) {
        1.0_f32
    } else {
        (f(a2) / f(a3)) as f32
    };
    let b0 = ((f(a0) * f(a2)) * f(HALF)) as f32;
    let b4 = (f(a1) * f(a2)) as f32;
    [b0, b4, b8]
}

/// What `0x618f80` stores at `+0x144` and `+0x148`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FacingInterp {
    pub f144: f32,
    pub f148: f32,
}

/// `CMovement::ComputeFacingRate` (`0x618f80`,
/// `systems/collision/status.md#facing-rate-toward-the-head`): `dt = (t1 − t0)·0.001`, 0 below
/// 2^-22, which keeps the old rate; otherwise the facing difference folded into ±π over `dt`. A
/// rate below 2^-20 in size zeroes `+0x148`.
pub fn facing_interp(t0: u32, h0: f32, t1: u32, h1: f32, cur_144: f32) -> FacingInterp {
    let dt = t1.wrapping_sub(t0);
    let dt_s_raw = (i64::from(dt) as f64) * f(C001);
    let dt_s = if dt_s_raw.abs() < f(EPS) {
        0.0
    } else {
        dt_s_raw
    };
    let f148_second = dt_s as f32;
    let f144 = if dt_s == 0.0 {
        cur_144
    } else {
        let dh = f(h1) - f(h0);
        let dh = if dh > f(PI) {
            dh - f(TWO_PI)
        } else if !(dh < f(NEG_PI)) {
            dh
        } else {
            dh + f(TWO_PI)
        };
        (dh / dt_s) as f32
    };
    let f148 = if f(f144).abs() < f(FLOOR) {
        0.0
    } else {
        f148_second
    };
    FacingInterp { f144, f148 }
}

/// `CMovement::ArmPositionBlend` (`0x619090`, `systems/collision/status.md#position-blend-arming`):
/// the window stored at `+0x14c`, or 0 on a null or inactive head, opcode 0x26, a facing or pitch
/// that differs, or a failed `0x7c5360`; otherwise the predicted point (x unrounded, y and z
/// stored) against the event's, in 3D with flags & 0x200000 and else in 2D, arms unless the squared
/// distance is below (1/36)².
pub fn reconcile_gate(
    head_null: bool,
    active: u8,
    opcode: u32,
    yaw_self: u32,
    yaw_ev: u32,
    pitch_self: u32,
    pitch_ev: u32,
    extrap_ok: bool,
    extrap: [f32; 3],
    flag_3axis: bool,
    base: [f32; 3],
    ev_pos: [f32; 3],
    window: i32,
) -> i32 {
    if head_null
        || active == 0
        || opcode == 0x26
        || yaw_self != yaw_ev
        || pitch_self != pitch_ev
        || !extrap_ok
    {
        return 0;
    }
    let pred_x = f(extrap[0]) + f(base[0]);
    let pred_y = (f(extrap[1]) + f(base[1])) as f32;
    let pred_z = (f(extrap[2]) + f(base[2])) as f32;
    let dist2 = if flag_3axis {
        let dx = pred_x - f(ev_pos[0]);
        let dy = f(pred_y) - f(ev_pos[1]);
        let dz = f(pred_z) - f(ev_pos[2]);
        (dz * dz + dy * dy) + dx * dx
    } else {
        let dx = pred_x - f(ev_pos[0]);
        let dy = f(pred_y) - f(ev_pos[1]);
        dy * dy + dx * dx
    };
    if !(dist2 < f(TOL)) {
        window
    } else {
        0
    }
}

/// What `0x6191c0` returns and leaves: `eax`, the output and `+0x14c`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PosLerp {
    pub eax: u32,
    pub out: [f32; 3],
    pub f14c: i32,
}

/// `CMovement::BlendTowardHead` (`0x6191c0`, `systems/collision/status.md#position-blend`): with
/// `frac = (ev_ts − now)/win`, `out = (cur − target)·frac + target`; x and z store the product
/// first, z also the difference. A null head or a past event returns 0 and zeroes `+0x14c`.
pub fn pos_lerp(
    head_null: bool,
    ev_ts: i32,
    now: i32,
    win: i32,
    cur: [f32; 3],
    tgt: [f32; 3],
    cur_14c: i32,
) -> PosLerp {
    if head_null {
        return PosLerp {
            eax: 0,
            out: cur,
            f14c: 0,
        };
    }
    let diff = ev_ts.wrapping_sub(now);
    if diff < 0 {
        return PosLerp {
            eax: 0,
            out: cur,
            f14c: 0,
        };
    }
    let frac = f64::from(diff) / f64::from(win);

    let tmp_x = ((f(cur[0]) - f(tgt[0])) * frac) as f32;
    let out_x = (f(tmp_x) + f(tgt[0])) as f32;

    let out_y = ((f(cur[1]) - f(tgt[1])) * frac + f(tgt[1])) as f32;

    let dz = (f(cur[2]) - f(tgt[2])) as f32;
    let tmp_z = (f(dz) * frac) as f32;
    let out_z = (f(tmp_z) + f(tgt[2])) as f32;
    PosLerp {
        eax: 1,
        out: [out_x, out_y, out_z],
        f14c: cur_14c,
    }
}
