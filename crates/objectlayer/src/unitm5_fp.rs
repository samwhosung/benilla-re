//! Float kernels of unit frame code in `[0x60a7f0, 0x611ff0]`: `CGUnit_C::Tick` (`0x60a7f0`),
//! `PlayKitCharProcs` (`0x60d7c0`), `CGUnit_C::CheckAutoMoveRange` (`0x6110a0`) and
//! `CGUnit_C::DrawBowstring` (`0x611ff0`) (`systems/objectlayer/unitbody.md#the-unit-tick`,
//! `auras.md#kit-character-procedures`, `unitmovement.md#auto-move-modes`,
//! `unitdisplay.md#hand-items-and-weapons`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. A value
//! left on the stack is not rounded; a value spilled to a `dword` and reloaded is rounded at the
//! spill. `__ftol` (`0x40a2b0`) takes the live value and truncates.

use crate::x87::ftol;

/// `_DAT_007ffd74` (`0x00000000`): 0.0.
pub const ZERO: f32 = 0.0;
/// `_DAT_008029d0` (`0x3c23d70a`): 0.01, the step of the level fraction in `Tick`.
pub const MOUNT_STEP: f32 = f32::from_bits(0x3c23_d70a);
/// `_DAT_0080295c` (`0x40c90fdb`): 2π as `f32`, the modulus of the facing wrap in `Tick`.
pub const TAU: f32 = f32::from_bits(0x40c9_0fdb);
/// `_DAT_0080306c` (`0x40800000`): 4.0, the lean pitch gain in `Tick`.
pub const LEAN_GAIN: f32 = 4.0;
/// `_DAT_008022cc` (`0x447a0000`): 1000.0, the scale of the colour event fields in
/// `PlayKitCharProcs`.
pub const COLOR_SCALE_1000: f32 = 1000.0;
/// `_DAT_008029cc` (`0x44000000`): 512.0, added to the footstep parameter in `PlayKitCharProcs`;
/// `shr 0xe` of the result's bits then extracts the integer part.
pub const FOOTSTEP_BIAS: f32 = f32::from_bits(0x4400_0000);
/// `_DAT_007ff9d8` (`0x3f800000`): 1.0, the upper bound of a trail colour in `DrawBowstring`.
pub const ONE: f32 = 1.0;
/// `_DAT_007ffe58` (`0x437f0000`): 255.0, the byte scale of a trail colour.
pub const COLOR_255: f32 = 255.0;
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the rounding bias of a trail colour and the weight of the
/// trail midpoint.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);

/// The level fraction of `CGUnit_C::Tick` (`0x60a7f0`), `0x60a9c2` to `0x60a9f6`: the level byte at
/// `[esi+0xe68]+0x1d` is cut to 100 (`cmp 0x64; jb`), stored to a dword, and `fild; fmul 0.01; fst
/// dword [ebp+8]` rounds the product to `f32`. `fcomp 0.0; test ah,0x41; jne` only chooses whether
/// the value is used.
pub fn mount_level_fraction(level_byte: u8) -> f32 {
    let clamped = level_byte.min(0x64);
    (f64::from(u32::from(clamped)) * f64::from(MOUNT_STEP)) as f32
}

/// The facing wrap of `Tick`, `0x60aa76` to `0x60aaad`: `fld [edx+0x1c]; fadd [ebp-4]` is the sum;
/// `fst dword [ebp+8]` rounds it to `f32` and keeps the register. `fcomp 0.0; test ah,5; jp
/// 0x60aa97` falls through only for C0 alone, a live sum below 0, and then adds 2π to the stored
/// `f32`. At `0x60aa97` `fcomp 2π; test ah,1; jne 0x60aab0` leaves a stored sum below 2π and NaN
/// unchanged, and otherwise subtracts 2π. Each result is stored as a dword.
pub fn move_facing_wrap(move_orient: f32, delta: f32) -> f32 {
    let sum_live = f64::from(move_orient) + f64::from(delta);
    let sum = sum_live as f32;
    let s = f64::from(sum);
    if !sum_live.is_nan() && sum_live < 0.0 {
        return (s + f64::from(TAU)) as f32;
    }
    if !s.is_nan() && s >= f64::from(TAU) {
        return (s - f64::from(TAU)) as f32;
    }
    sum
}

/// The lean pitch of `Tick`, `0x60aaea` to `0x60aafe`: `fld [edx+0x20]; fld [ebp-4]; fmul 4.0; fadd
/// st,st(1); fstp dword [esp]`, so `delta·4.0 + pitch_rate` at 53 bits and rounded to `f32` once.
/// The clamp call `0x60aba0` that takes it only compares.
pub fn lean_pitch(pitch_rate: f32, delta: f32) -> f32 {
    ((f64::from(delta) * f64::from(LEAN_GAIN)) + f64::from(pitch_rate)) as f32
}

/// The colour event field of `PlayKitCharProcs` (`0x60d7c0`), `0x60d919` and `0x60d934`: `fld
/// [edi+off]; fmul 1000.0; fcom 0.0; call __ftol`. The product stays on the stack unrounded and
/// `__ftol` truncates it; the `fcom` is not branched on. Returns `eax`.
pub fn color_field_x1000(field: f32) -> i32 {
    ftol(f64::from(field) * f64::from(COLOR_SCALE_1000))
}

/// The duration field of `PlayKitCharProcs`, `0x60d9ef`: the virtual getter `[eax+0x6c]` leaves
/// `duration` on the stack, `fmul [edi-0x20]` scales it by `field`, and `__ftol` truncates the
/// unrounded product. Returns `eax`.
pub fn duration_field(duration: f32, field: f32) -> i32 {
    ftol(f64::from(duration) * f64::from(field))
}

/// The footstep parameter of `PlayKitCharProcs`, `0x60db19`, `0x60db35` and `0x60db4a`: `fld
/// [edi+off]; fadd 512.0; fstp dword [ebp-N]`, whose bits `shr 0xe` then slices.
pub fn footstep_param(field: f32) -> f32 {
    (f64::from(field) + f64::from(FOOTSTEP_BIAS)) as f32
}

/// The squared distance of `CGUnit_C::CheckAutoMoveRange` (`0x6110a0`), `0x6110d9` to `0x6110f7`:
/// `goal` (`ecx`) less `pos` (`eax`) on each axis, left on the stack, with the sum `(dx² + dz²) +
/// dy²`.
pub fn goal_dist2(goal: [f32; 3], pos: [f32; 3]) -> f64 {
    let dx = f64::from(goal[0]) - f64::from(pos[0]);
    let dy = f64::from(goal[1]) - f64::from(pos[1]);
    let dz = f64::from(goal[2]) - f64::from(pos[2]);
    let dz2 = dz * dz;
    (dx * dx + dz2) + dy * dy
}

/// The range test of `CheckAutoMoveRange`, `0x6110b0` to `0x611122`: a zero enable byte
/// `[esi+0x860a58]` returns 1; `fld [esi+0x860a5c]; fcomp 0.0; test ah,0x44; jnp` returns 1 for a
/// squared radius of 0 (C3 alone) and goes on for positive, negative and NaN. Then `fcomp radius²;
/// test ah,1; jne` returns 1 when C0 is set, which is `d² < r²` and NaN; otherwise it reports the
/// error and returns 0. Returns whether it returns 1.
pub fn in_range_of_goal(enabled: u8, radius2: f32, goal: [f32; 3], pos: [f32; 3]) -> bool {
    if enabled == 0 {
        return true;
    }
    let r2 = f64::from(radius2);
    if r2 == 0.0 {
        return true;
    }
    let dist2 = goal_dist2(goal, pos);
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    {
        !(dist2 >= r2)
    }
}

/// One trail colour channel of `CGUnit_C::DrawBowstring` (`0x611ff0`), `0x612044` to `0x612116`,
/// from `c` at `[edi+0x5c]`, `[edi+0x58]` or `[edi+0x54]`. `fcomp 0.0; test ah,0x41; jp` loads 0.0
/// for `c < 0` and `c = 0`. Above 0, `fcomp 1.0; test ah,1; jne` takes `ftol(c·255 + 0.5)` with the
/// product and sum left on the stack, and `c >= 1` takes `ftol(255.0)`. NaN takes the scale arm,
/// where `fcomp 1.0` is unordered and `__ftol` of the NaN product gives 0, as this returns.
pub fn trail_color_byte(c: f32) -> i32 {
    let cf = f64::from(c);
    if cf > 0.0 {
        if cf < f64::from(ONE) {
            ftol(cf * f64::from(COLOR_255) + f64::from(HALF))
        } else {
            ftol(f64::from(COLOR_255))
        }
    } else {
        0
    }
}

/// The trail midpoint of `DrawBowstring`, `0x612221` to `0x612266`: `q` is `[ecx]` and `p` is
/// `[esi-0x20]`. The sums `q.x + p.x` and `q.y + p.y` stay on the stack through `fmul 0.5`; the sum
/// of z is stored to a dword at `0x612235` and reloaded at `0x612251` before its `fmul 0.5`, so it
/// is rounded to `f32` first. Each result is stored as a dword.
pub fn trail_midpoint(p: [f32; 3], q: [f32; 3]) -> [f32; 3] {
    let rx = ((f64::from(q[0]) + f64::from(p[0])) * f64::from(HALF)) as f32;
    let ry = ((f64::from(q[1]) + f64::from(p[1])) * f64::from(HALF)) as f32;
    let sz = (f64::from(q[2]) + f64::from(p[2])) as f32;
    let rz = (f64::from(sz) * f64::from(HALF)) as f32;
    [rx, ry, rz]
}
