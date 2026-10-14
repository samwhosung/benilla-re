//! Float kernels at the head of the unit class: `CGUnit_C::CGUnit_C` (`0x5fad50`),
//! `CGUnit_C::PostInit` (`0x5fb880`), `CGUnit_C::OnFootstep` (`0x5fbf70`),
//! `CGUnit_C::ResolveMoveAnimation` (`0x5fd100`), `CGUnit_C::PlayAnimation` (`0x5fe2f0`) and
//! `CGUnit_C::OnModelEvent` (`0x5ffbd0`) (`systems/objectlayer/units.md#the-unit-object` and
//! `#lifecycle`, `unitanim.md#model-events`, `#choosing-the-base-animation` and
//! `#playing-an-animation`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! left on the stack is not rounded; a value spilled to a `dword` and reloaded is rounded at the
//! spill. `CMovement::GetCurrentSpeed` (`0x7c4c90`) leaves a float on `st0`; its result is an input
//! here.

/// 0.2 (`0x80679c`, `0x3e4ccccd`): the health ratio under which the constructor starts a bleed.
pub const LOW_HP_RATIO: f32 = f32::from_bits(0x3e4c_cccd);
/// 0.75 (`0x8012cc`): the `PostInit` ground-snap scale of the unit's reach.
pub const REACH_SCALE_075: f32 = f32::from_bits(0x3f40_0000);
/// 0.5 (`0x7ffa24`): the `OnFootstep` height gate scale of the unit's reach.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// 2500.0 (`0x80c5b4`): the footstep's outer squared distance from the camera (50 yards).
pub const DIST2_OUTER: f32 = f32::from_bits(0x451c_4000);
/// 625.0 (`0x80c5b0`): its inner squared distance (25 yards).
pub const DIST2_INNER: f32 = f32::from_bits(0x441c_4000);
/// 11.0 (`0x80c484`): the speed at and over which `ResolveMoveAnimation` picks animation 0x8f.
pub const TIMER_THRESH: f32 = f32::from_bits(0x4130_0000);
/// 0.01 (`0x8029d0`, `0x3c23d70a`): the level fraction step of `OnModelEvent`. The `f32` is a shade
/// under 0.01.
pub const LEVEL_STEP: f32 = f32::from_bits(0x3c23_d70a);
/// 0.5 (`0x80c488`): the level fraction under which `OnModelEvent` takes its low band.
pub const LEVEL_THRESH: f32 = f32::from_bits(0x3f00_0000);

/// `CGUnit_C::CGUnit_C` (`0x5fad50`), the tail at `0x5fb145`: `fild [view+0x40]` (health), `fidiv
/// [view+0x58]` (maximum), `fcomp 0.2; test ah,5; jp 0x5fb170`. The jump skips for an even count of
/// C0 and C2, so the bleed check runs only for a ratio below 0.2, ordered: equal, greater and the
/// NaN of 0/0 skip it. Returns whether the check (`0x605f30` then `SetBleed` `0x5fa710`) runs.
pub fn low_hp_gate(health: i32, max_health: i32) -> bool {
    let ratio = f64::from(health) / f64::from(max_health);
    ratio < f64::from(LOW_HP_RATIO)
}

/// `CGUnit_C::PostInit` (`0x5fb880`), the gate at `0x5fb8ff`: the raycast hit `ground_z`
/// (`[ebp-4]`) less the unit's z (`[eax+8]`) is `fcompp`'d with `reach * 0.75` (`+0xa5c`), and
/// `test ah,0x41; jne 0x5fb948` skips for less, equal and unordered. It snaps to the ground only
/// for `height > reach * 0.75`, ordered. The shape is `IsInDeepLiquid`'s (`0x605650`) with 0.75.
pub fn ground_snap_gate(ground_z: f32, self_z: f32, reach: f32) -> bool {
    let height = f64::from(ground_z) - f64::from(self_z);
    let gate = f64::from(reach) * f64::from(REACH_SCALE_075);
    !height.is_nan() && !gate.is_nan() && height > gate
}

/// The squared distance of `OnFootstep` (`0x5fbf70`, `0x5fbfdb` to `0x5fc015`) from the footstep
/// event position `goal` (`[ebx+8]`) to the active camera's eye `pos` (`0x4818f0`, then `+8`). The
/// differences stay at 53 bits and the squares sum as `(dy^2 + dz^2) + dx^2` (`fld st(0)`, `fmul`,
/// `fld st(2)`, `fmul`, `faddp`, `fld st(3)`, `fmul`, `faddp`); `fstp [ebp-4]` stores the sum as
/// `f32`, which both gates read back.
pub fn placement_dist2(goal: [f32; 3], pos: [f32; 3]) -> f32 {
    let dx = f64::from(goal[0]) - f64::from(pos[0]);
    let dy = f64::from(goal[1]) - f64::from(pos[1]);
    let dz = f64::from(goal[2]) - f64::from(pos[2]);
    let sum = (dy * dy + dz * dz) + dx * dx;
    sum as f32
}

/// The outer gate of `OnFootstep` (`0x5fc00a`): `fcomp 2500.0; test ah,0x41; je` leaves for `dist^2
/// > 2500`, ordered. Returns whether it goes on, `!(dist^2 > 2500)`; NaN goes on.
pub fn placement_in_outer_range(dist2: f32) -> bool {
    crate::x87::not_above(f64::from(dist2), f64::from(DIST2_OUTER))
}

/// The inner gate of `OnFootstep` (`0x5fc0b3`) on the same `f32`: `!(dist^2 > 625)`.
pub fn placement_in_inner_range(dist2: f32) -> bool {
    crate::x87::not_above(f64::from(dist2), f64::from(DIST2_INNER))
}

/// The lifted z of `OnFootstep` (`0x5fc157` to `0x5fc194`): the spray's z is the event's (`goal_z`,
/// copied to `[ebp-0x1c]`) plus the ground clearance `ground_z - self_z` (`[ebp-0x10]` less
/// `[eax+8]`). The clearance stays on the stack through the height gate's `fcompp` and is added
/// unrounded; `fstp [ebp-0x1c]` stores the sum as `f32`.
pub fn placement_ground_z(goal_z: f32, ground_z: f32, self_z: f32) -> f32 {
    let height = f64::from(ground_z) - f64::from(self_z);
    (f64::from(goal_z) + height) as f32
}

/// The height gate of `OnFootstep` (`0x5fc169` to `0x5fc16f`): `height = ground_z - self_z` against
/// `reach * 0.5` by `fcompp; test ah,5; jp 0x5fc303`. The jump skips the spray for an even count of
/// C0 and C2, so it goes on only for `height < reach * 0.5`, ordered; equal, greater and NaN skip.
pub fn placement_height_gate(ground_z: f32, self_z: f32, reach: f32) -> bool {
    let height = f64::from(ground_z) - f64::from(self_z);
    let gate = f64::from(reach) * f64::from(HALF);
    !height.is_nan() && !gate.is_nan() && height < gate
}

/// `CGUnit_C::ResolveMoveAnimation` (`0x5fd100`), the speed tests from `0x5fd1fd`. `timer` is the
/// current speed `0x7c4c90` leaves on `st0` and `swing_time` is `this[+0xa30]`. It returns the
/// animation written to `*out`:
/// - `fcom 11.0; test ah,1; jne`: C0 is a speed under 11 or unordered, which goes on to the next
///   test; a speed of 11 or more gives `0x8f`;
/// - `2 * swing_time` (`fadd st,st`) against the speed by `fcompp; test ah,0x41; jne`: a speed
///   above twice it, ordered, gives `0x5`; less, equal and NaN give `0x4`.
pub fn fall_timer_anim(timer: f32, swing_time: f32) -> u32 {
    let t = f64::from(timer);
    if t >= f64::from(TIMER_THRESH) {
        return 0x8f;
    }
    let two_swing = f64::from(swing_time) + f64::from(swing_time);
    if !t.is_nan() && !two_swing.is_nan() && t > two_swing {
        0x5
    } else {
        0x4
    }
}

/// `CGUnit_C::PlayAnimation` (`0x5fe2f0`), `0x5fe53c`: `0x7c4c90` leaves the current speed on `st0`
/// and `fdiv [ebp-0x15c]; fstp [ebp-0xc]` stores `timer / divisor` as `f32`.
pub fn anim_progress(timer: f32, divisor: f32) -> f32 {
    (f64::from(timer) / f64::from(divisor)) as f32
}

/// `CGUnit_C::OnModelEvent` (`0x5ffbd0`), the level band at `0x60001a`: the level byte at
/// `[this+0xe68]+0x1d` is cut to 100 (`cmp 0x64; jb`), `fild`'d and multiplied by 0.01. The product
/// stays live and `fcomp 0.5; test ah,1; jne 0x60004d` takes the low band for C0, a fraction under
/// 0.5. It returns whether the high band runs (`edi = 7`), a fraction of 0.5 or more. The `f32`
/// 0.01 is under 0.01, so level 50 gives 0.4999999 and the high band starts at 51.
pub fn anim_event_level_band(level_byte: u8) -> bool {
    let clamped = level_byte.min(0x64);
    let frac = f64::from(u32::from(clamped)) * f64::from(LEVEL_STEP);
    frac >= f64::from(LEVEL_THRESH)
}
