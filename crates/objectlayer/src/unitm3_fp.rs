//! Float kernels of unit auto-move and placement code in `[0x60b270, 0x613ef0]`:
//! the mover size gate `0x60b270`, `CGUnit_C::UpdateNamePlateVisibility` (`0x60f600`),
//! `CGUnit_C::WalkToAttack` (`0x60fcc0`), `CGUnit_C::WalkAlongHeading` (`0x610100`),
//! `CGUnit_C::AutoMovePitchToward` (`0x610520`), `CGUnit_C::GetAutoMoveTarget` (`0x610e40`),
//! `CGUnit_C::GetAutoMoveStopDistance` (`0x611240`), `CGUnit_C::DrawAutoMoveTarget` (`0x611400`)
//! and the world transform `0x613ef0` (`systems/objectlayer/unitmovement.md`,
//! `#starting-and-ending-auto-move`, `#steering-an-auto-move`, `#auto-move-modes`,
//! `unitnames.md#nameplates`, `systems/collision/mover.md#collision-size`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a value spilled to a `dword` and reloaded is rounded to `f32` at the spill;
//! `fild` and `fidiv` widen an integer, and `fsin` is the host's.

/// `_DAT_007ff9d8` (`0x3f800000`): 1.0.
pub const ONE: f32 = 1.0;
/// `_DAT_007ffd74` (`0x00000000`): 0.0, the `fmul` that zeroes the first term of the aim key.
pub const ZERO: f32 = 0.0;
/// `_DAT_0080b058` (`0x3faaaaab`): 1.3333334, added to the two combat reaches in `0x60fcc0` and
/// `0x611240`.
pub const REACH_SLACK: f32 = f32::from_bits(0x3faa_aaab);
/// `_DAT_0080a1e8` (`0x40a00000`): 5.0, the least combat reach.
pub const REACH_FLOOR: f32 = 5.0;
/// `_DAT_008026bc` (`0x35800000`): 2^-20, the size below which `0x60b270` takes a dimension as 0.
pub const EPS_2EM20: f32 = f32::from_bits(0x3580_0000);
/// `_DAT_00801360` (`0x3a83126f`): 0.001, the dead band of the pitch step in `0x610520`.
pub const EPS_1EM3: f32 = f32::from_bits(0x3a83_126f);
/// `_DAT_0080c5f0` (`0x3ec90fdb`): π/8, the pitch error past which `0x610520` reports not arrived.
pub const PITCH_ARRIVED: f32 = f32::from_bits(0x3ec9_0fdb);
/// `_DAT_00804588` (`0x3f666666`): 0.9, the scale of the spell range in `0x611240` action 9.
pub const SCALE_0P9: f32 = f32::from_bits(0x3f66_6666);
/// `_DAT_0080295c` (`0x40c90fdb`): 2π as `f32`, the phase scale of the click ring in `0x611400`.
pub const TAU: f32 = f32::from_bits(0x40c9_0fdb);
/// `_DAT_0080c4d0` (`0x40e00000`): 7.0, the speed divisor of the arrival radius in `0x610e40`.
pub const SEVEN: f32 = 7.0;
/// `_DAT_00860ae4` (`0x3f000000`): 0.5, the low end of the ring pulse.
pub const RING_LO: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_00860ae8` (`0x3f400000`): 0.75, the high end of the ring pulse.
pub const RING_HI: f32 = f32::from_bits(0x3f40_0000);
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the factor on `sin + 1` in the ring pulse.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_00860ae0`: 500, the period in milliseconds of the click ring.
pub const RING_PERIOD: i32 = 500;

/// One size test of the mover in `0x60b270`, at `0x60b325` for `[esi+0x38]`, `0x60b33b` for
/// `[esi+0x3c]` and `0x60b351` for the height `[ebp-4]`: `fld v; fabs; fcomp 2^-20; test ah,5`.
/// Returns whether the parity flag is set, an even count of C0 and C2, which is `|v| >= 2^-20` and
/// NaN; `false` is `|v| < 2^-20`. The first two tests `jnp` to `0x60b3f7` on `false`, and the third
/// `jp` to `0x60b393` on `true`.
pub fn box_dim_pf_even(v: f32) -> bool {
    let a = f64::from(v).abs();
    let eps = f64::from(EPS_2EM20);
    if a.is_nan() {
        return true;
    }
    a >= eps
}

/// The follow gate of `UpdateNamePlateVisibility` (`0x60f600`), `0x60f798` to `0x60f7c4`: `a` is
/// this unit's position and `b` the other's, left on the stack with the sum `(dz² + dx²) + dy²`.
/// `fcomp [0xc4d988]; test ah,0x41; jp 0x60f740` aborts on an even count of C0 and C3, which is
/// greater and NaN. Returns whether it goes on to `0x6086e0`, `d² <= thresh`.
pub fn follow_dist_proceed(a: [f32; 3], b: [f32; 3], thresh: f32) -> bool {
    let dx = f64::from(a[0]) - f64::from(b[0]);
    let dy = f64::from(a[1]) - f64::from(b[1]);
    let dz = f64::from(a[2]) - f64::from(b[2]);
    let dist2 = (dz * dz + dx * dx) + dy * dy;
    let t = f64::from(thresh);
    dist2 <= t
}

/// The reach of `WalkToAttack` (`0x60fcc0`), `0x60fce5` to `0x60fd1b`: `(target reach + caster
/// reach) + 4/3` on the stack, `fld 5.0; fcomp st(1); test ah,0x41; jne` keeps the sum when `5.0 <=
/// sum` and for NaN, and otherwise stores 5.0; `fstp dword [ebp-4]` rounds it.
pub fn cast_reach_floor(target_reach: f32, caster_reach: f32) -> f32 {
    let reach_sum = (f64::from(target_reach) + f64::from(caster_reach)) + f64::from(REACH_SLACK);
    let five = f64::from(REACH_FLOOR);
    let pick_reach = reach_sum.is_nan() || five < reach_sum || five == reach_sum;
    if pick_reach {
        reach_sum as f32
    } else {
        REACH_FLOOR
    }
}

/// The range test of `0x60fcc0`, `0x60fd2f` to `0x60fd61`: `a` is the target's position and `b` the
/// caster's, with the sum `(dz² + dy²) + dx²` on the stack; `fld [ebp-4]; fmul [ebp-4]` is
/// `reach²`. `fcompp` and `test ah,0x41; jne 0x60fd90` fall through to the `0x6100a0` call only for
/// `r² > d²`; equal, less and NaN take the plane projection. Returns whether it falls through.
pub fn cast_in_range(a: [f32; 3], b: [f32; 3], reach: f32) -> bool {
    let dx = f64::from(a[0]) - f64::from(b[0]);
    let dy = f64::from(a[1]) - f64::from(b[1]);
    let dz = f64::from(a[2]) - f64::from(b[2]);
    let dist2 = (dz * dz + dy * dy) + dx * dx;
    let reach2 = f64::from(reach) * f64::from(reach);
    reach2 > dist2
}

/// The plane projection of `0x60fcc0`, `0x60fe37` to `0x60fe51`: with `dx` and `dy` the caster's x
/// and y (`[ebp-0x14]`, `[ebp-0x10]`) and `plane` the four floats at `0xc4d868`, `fld dy; fmul
/// plane[1]; fld dx; fmul plane[0]; faddp; fadd plane[3]; fdiv plane[2]; fchs; fstp dword`.
pub fn cast_plane_z(dx: f32, dy: f32, plane: [f32; 4]) -> f32 {
    let num = f64::from(dx) * f64::from(plane[0]) + f64::from(dy) * f64::from(plane[1]);
    let z = (num + f64::from(plane[3])) / f64::from(plane[2]);
    (-z) as f32
}

/// The aim vector of `WalkAlongHeading` (`0x610100`), `0x610161` to `0x6101b7`, from the vector
/// `(v0, v1, v2)` at `[ebp-0x20]`: the length is `sqrt((v1² + v0²) + v2²)`, `inv = 1.0 / len` stays
/// on the stack, and each component is `fmul`'d and stored as a dword. `key = fabs((v1n + v0n)·0.0
/// + v2n)` reloads the stored components; the `0.0` product is NaN for an infinite sum. Returns
/// `(v0n, v1n, v2n, key)`.
pub fn cast_aim_normalize(v0: f32, v1: f32, v2: f32) -> (f32, f32, f32, f32) {
    let len2 = (f64::from(v1) * f64::from(v1) + f64::from(v0) * f64::from(v0))
        + f64::from(v2) * f64::from(v2);
    let inv = f64::from(ONE) / len2.sqrt();
    let v0n = (f64::from(v0) * inv) as f32;
    let v1n = (f64::from(v1) * inv) as f32;
    let v2n = (inv * f64::from(v2)) as f32;
    let key = (f64::from(v1n) + f64::from(v0n)) * f64::from(ZERO) + f64::from(v2n);
    (v0n, v1n, v2n, key.abs() as f32)
}

/// The endpoint of `0x610100`, `0x61023b` to `0x61025a`: the position `pos` from `GetPosition` plus
/// the normalised aim `vn`, each component `fld vn; fadd pos; fstp dword`.
pub fn cast_aim_endpoint(vn: [f32; 3], pos: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(vn[0]) + f64::from(pos[0])) as f32,
        (f64::from(vn[1]) + f64::from(pos[1])) as f32,
        (f64::from(vn[2]) + f64::from(pos[2])) as f32,
    ]
}

/// The pitch step of `AutoMovePitchToward` (`0x610520`), `0x6105f2` to `0x610680`, from `cur`
/// (`this[+0x9c8]`), `goal` (`[ebp+0x10]`) and `rate` (`[ebp+0x14]`). `fabs` of `goal - cur` with
/// `fcomp 0.001; test ah,5; jp` returns `None` below the dead band. `fld goal; fcomp cur; test
/// ah,0x41; jne` takes the downward arm for `goal <= cur` and NaN. Upward the step is `cur + rate`,
/// stored as a dword and kept when below the goal; downward it is `cur - rate`, kept when above it;
/// otherwise the goal is used. The byte written to `*[ebp+0x1c]` is 1 when `|stepped - goal| > π/8`
/// (`fcomp; test ah,0x41; jne` writes 0 for less, equal and NaN). Returns `(stepped, byte)`.
pub fn pitch_interp(cur: f32, goal: f32, rate: f32) -> Option<(f32, bool)> {
    let eps = f64::from(EPS_1EM3);
    let delta = (f64::from(goal) - f64::from(cur)).abs();
    if !delta.is_nan() && delta < eps {
        return None;
    }
    let goal_le_cur = goal.is_nan() || cur.is_nan() || f64::from(goal) <= f64::from(cur);
    let stepped = if !goal_le_cur {
        let s0 = (f64::from(cur) + f64::from(rate)) as f32;
        let s0d = f64::from(s0);
        if !s0d.is_nan() && !f64::from(goal).is_nan() && s0d < f64::from(goal) {
            s0
        } else {
            goal
        }
    } else {
        let s0 = (f64::from(cur) - f64::from(rate)) as f32;
        let s0d = f64::from(s0);
        let use_goal = s0d.is_nan() || f64::from(goal).is_nan() || s0d <= f64::from(goal);
        if use_goal {
            goal
        } else {
            s0
        }
    };
    let d = (f64::from(stepped) - f64::from(goal)).abs();
    let out_byte = d > f64::from(PITCH_ARRIVED);
    Some((stepped, out_byte))
}

/// The squared distances of `GetAutoMoveTarget` (`0x610e40`), `0x610f7a` to `0x610fb6`, from the
/// target `tgt` and this unit's position `self_pos`. The z difference is stored to a dword at
/// `0x610f7a` and reloaded, so its square uses the `f32`; `dy²` and `dx²` are each stored as
/// dwords. Returns `(dist3d_sq, horiz_sq)`: `(dz² + dy²) + dx²` and `dy² + dx²`, the latter for the
/// move-flag `0x200000` path.
pub fn move_dist_squares(tgt: [f32; 3], self_pos: [f32; 3]) -> (f32, f32) {
    let dx = f64::from(tgt[0]) - f64::from(self_pos[0]);
    let dy = f64::from(tgt[1]) - f64::from(self_pos[1]);
    let dz_f32 = (f64::from(tgt[2]) - f64::from(self_pos[2])) as f32;
    let dy2 = (dy * dy) as f32;
    let dx2 = (dx * dx) as f32;
    let dz = f64::from(dz_f32);
    let dist3d = (dz * dz + f64::from(dy2) + f64::from(dx2)) as f32;
    let horiz = (f64::from(dy2) + f64::from(dx2)) as f32;
    (dist3d, horiz)
}

/// The goal radius of `0x610e40`: `t = speed / 7.0` from the speed leaf `0x7c4c90`, `fcomp 1.0;
/// test ah,5; jp` keeps `t` for `t >= 1` and NaN and otherwise loads 1.0, then `· cfg` with `cfg`
/// the scalar at `[ebp-4]`, initialised from `[0x80c4c8]`. Returns `max(speed / 7, 1.0)·cfg`
/// rounded to `f32`.
pub fn move_goal_radius(speed: f32, cfg: f32) -> f32 {
    let t = f64::from(speed) / f64::from(SEVEN);
    let one = f64::from(ONE);
    let t_clamped = if !t.is_nan() && t < one { one } else { t };
    (t_clamped * f64::from(cfg)) as f32
}

/// The reach of `GetAutoMoveStopDistance` (`0x611240`) actions 6 and 10, from `0x6112b6`: as
/// [`cast_reach_floor`] stored to a dword, and for action 10 the stored value less 4/3, rounded
/// again.
pub fn cast_goal_reach(target_reach: f32, caster_reach: f32, action_is_10: bool) -> f32 {
    let reach_sum = (f64::from(target_reach) + f64::from(caster_reach)) + f64::from(REACH_SLACK);
    let five = f64::from(REACH_FLOOR);
    let pick_reach = reach_sum.is_nan() || five < reach_sum || five == reach_sum;
    let floored_f32 = if pick_reach {
        reach_sum as f32
    } else {
        REACH_FLOOR
    };
    if action_is_10 {
        (f64::from(floored_f32) - f64::from(REACH_SLACK)) as f32
    } else {
        floored_f32
    }
}

/// Action 7 of `0x611240`, at `0x611336`: the global `[0xc4d998]` times itself.
pub fn cast_goal_square(g: f32) -> f32 {
    (f64::from(g) * f64::from(g)) as f32
}

/// Action 9 of `0x611240`, at `0x61137d`: `r = range · 0.9` stays on the stack and `r · r` is
/// stored as a dword, so the only `f32` rounding is at the store.
pub fn cast_goal_range(range: f32) -> f32 {
    let r = f64::from(range) * f64::from(SCALE_0P9);
    (r * r) as f32
}

/// The pulse of the click ring in `DrawAutoMoveTarget` (`0x611400`), from `tmod = GetTickCount() %
/// 500`: `fild tmod; fmul 2π; fidiv 500; fsin; fadd 1.0; fmul 0.5` is `osc`, then `fld 0.75; fsub
/// 0.5; fmulp; fadd 0.5` gives `0.5 + 0.25·osc`, and `fstp dword [ebp-4]` rounds it.
pub fn click_ring_pulse(tmod: i32) -> f32 {
    let phase = f64::from(tmod) * f64::from(TAU) / f64::from(RING_PERIOD);
    let osc = (phase.sin() + f64::from(ONE)) * f64::from(HALF);
    let pulse = (f64::from(RING_HI) - f64::from(RING_LO)) * osc + f64::from(RING_LO);
    pulse as f32
}

/// A coordinate of the ring's box in `0x611400`: `c - pulse` and `c + pulse`, each `fld c; fsub` or
/// `fadd pulse; fstp dword`.
pub fn click_ring_corner(c: f32, pulse: f32) -> (f32, f32) {
    (
        (f64::from(c) - f64::from(pulse)) as f32,
        (f64::from(c) + f64::from(pulse)) as f32,
    )
}

/// The model scale of the world transform `0x613ef0`, `0x613fbf` to `0x613fee`: `fld s_z; fld s_a;
/// fmul s_b; fmul st,st(1); fstp dword [esp]` gives `(s_a·s_b)·s_z`, with `s_a = this[+0x94]`, `s_b
/// = this[+0x90]` and `s_z` the `this[+0x9c]` for a non-zero `this[+0xdc]` and `this[+0x98]`
/// otherwise.
pub fn world_model_scale(s_a: f32, s_b: f32, s_z: f32) -> f32 {
    ((f64::from(s_a) * f64::from(s_b)) * f64::from(s_z)) as f32
}

/// The bounds sums of `0x613ef0`, `0x614031` to `0x614050`: `lo` and `hi` are the two vectors
/// `0x713700` fills, and each component `fld; fadd; fstp dword` is an argument of `0x4549a0`.
/// Returns `[lo.x + hi.x, lo.y + hi.y, lo.z + hi.z]`.
pub fn world_bounds_sum(lo: [f32; 3], hi: [f32; 3]) -> [f32; 3] {
    [
        (f64::from(hi[0]) + f64::from(lo[0])) as f32,
        (f64::from(lo[1]) + f64::from(hi[1])) as f32,
        (f64::from(lo[2]) + f64::from(hi[2])) as f32,
    ]
}
