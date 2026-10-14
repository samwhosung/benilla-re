//! Float kernels of eight functions in the object code: the game object placement `0x5f7910`, the
//! game object use test `0x5f3130`, `CGUnit_C::OnHealthFieldChanged` (`0x6046f0`),
//! `CGUnit_C::Mount` (`0x607a00`), the object update `0x614a90`, `PlayBowPull` (`0x624cc0`), the
//! item repair cost `0x5da330` and `LogExhaustionXpGain` (`0x637ac0`)
//! (`systems/objectlayer/gameobjects.md#world-transform-and-box` and `#usable`, `units.md#health-
//! death-and-resurrection`, `unitdisplay.md#mounts`, `unitcombat.md#the-ranged-reload-pose`,
//! `items.md#repair-cost`, `combatlogmessages.md#experience`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`. `fild`
//! reads a signed integer and `__ftol` (`0x40a2b0`) truncates toward zero into 64 bits.

use crate::x87::ftol64;

/// `_DAT_007ff9d8` (`0x3f800000`): 1.0.
pub const ONE: f32 = 1.0;
/// `_DAT_007ffd74` (`0x00000000`): 0.0.
pub const ZERO: f32 = 0.0;
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the rounding bias before `__ftol` in `0x5da330`.
pub const HALF: f32 = 0.5;
/// `_DAT_0080679c` (`0x3e4ccccd`): 0.2, the health fraction below which `0x6046f0` plays the
/// low-health animation.
pub const LOW_HP_FRAC: f32 = f32::from_bits(0x3e4c_cccd);
/// `_DAT_007ff9d0` (`0x3ce38e39`): 1/36, the scale of the display sizes in `0x607a00`.
pub const INV_36: f32 = f32::from_bits(0x3ce3_8e39);
/// `_DAT_0080c698` (`0x3a03126f`): 0.0005, the milliseconds-to-seconds factor of `0x614a90`.
pub const MS_SCALE: f32 = f32::from_bits(0x3a03_126f);
/// `_DAT_0080c610` (`0x40490fdb`): π, the phase factor of `0x614a90`.
pub const PI_C: f32 = f32::from_bits(0x4049_0fdb);
/// `_DAT_008029d4` (`0x34800000`): 2^-22, the distance from 0 and 1 within which `0x637ac0` skips
/// the rest multiplier.
pub const REST_EPS: f32 = f32::from_bits(0x3480_0000);

/// The extent of `CGGameObject_C::Animate` (`0x5f7910`), `0x5f79ad` to `0x5f79be`: `fld blk[5];
/// fsub blk[2]; fstp dword [esi+0xbc]`, the copied 6-float block's high less its low on one axis.
pub fn animate_extent(blk5: f32, blk2: f32) -> f32 {
    (f64::from(blk5) - f64::from(blk2)) as f32
}

/// The corner sums of `0x5f7910`, `0x5f79df` to `0x5f79f9`: each `fadd` is stored as an argument of
/// `0x4549a0`. With `e` the six floats at `eax` it returns the arguments `[e0 + e3, e4 + e1, e5 +
/// e2]`; the halving is `0x5f8cf0`'s.
pub fn animate_corner_sums(e: [f32; 6]) -> [f32; 3] {
    let s0 = (f64::from(e[0]) + f64::from(e[3])) as f32;
    let s1 = (f64::from(e[4]) + f64::from(e[1])) as f32;
    let s2 = (f64::from(e[5]) + f64::from(e[2])) as f32;
    [s0, s1, s2]
}

/// The use range of `CGGameObject_C::CanUse` (`0x5f3130`), `0x5f3346` to `0x5f3370`: `a` is this
/// object's position and `b` the other's, left on the stack with the sum `(dx² + dz²) + dy²`, and
/// `radius·radius` is formed from `[esi+0xc]`. `fcompp` compares it with the sum and `test ah,5;
/// jp` takes the success arm unless `r² < d²`, so equal and NaN are in range.
pub fn canuse_in_range(a: [f32; 3], b: [f32; 3], radius: f32) -> bool {
    let dx = f64::from(a[0]) - f64::from(b[0]);
    let dy = f64::from(a[1]) - f64::from(b[1]);
    let dz = f64::from(a[2]) - f64::from(b[2]);
    let dist2 = (dx * dx + dz * dz) + dy * dy;
    let r2 = f64::from(radius) * f64::from(radius);
    dist2 <= r2 || dist2.is_nan() || r2.is_nan()
}

/// The low-health test of `OnHealthFieldChanged` (`0x6046f0`), `0x604726` to `0x604750`: `fild
/// [block+0x40]; fidiv [block+0x58]; fcomp 0.2; test ah,5; jp 0x604750`. The jump skips the
/// animation unless C0 alone is set, which is a ratio below 0.2; a `maxhealth` of 0 divides to
/// infinity or NaN and skips. Returns whether the animation runs.
pub fn low_health(health: i32, maxhealth: i32) -> bool {
    let ratio = f64::from(health) / f64::from(maxhealth);
    ratio < f64::from(LOW_HP_FRAC)
}

/// The display sizes of `CGUnit_C::Mount` (`0x607a00`), `0x607ae0` to `0x607b02`: the record's
/// `+0x20` and `+0x1c` are each `fmul`'d by 1/36 and stored as dwords, `+0x20` first. Returns the
/// values for `[esi+0xcc4]` and `[esi+0xcc8]`.
pub fn mount_display_scale(r20: f32, r1c: f32) -> [f32; 2] {
    let sx = (f64::from(r20) * f64::from(INV_36)) as f32;
    let sy = (f64::from(r1c) * f64::from(INV_36)) as f32;
    [sx, sy]
}

/// The scale ratio of `0x607a00`, `0x607b49` to `0x607b86`: `[esi+0x98] / [esi+0x9c]`, rounded to
/// `f32` for the matrix diagonal; a zero divisor gives infinity or NaN.
pub fn mount_scale_ratio(a: f32, b: f32) -> f32 {
    (f64::from(a) / f64::from(b)) as f32
}

/// The ease weight of `0x614a90` (`0x614b3b` to `0x614b79`) from the fraction `t`, which `fst dword
/// [ebp-4]` rounds to `f32`. `fcomp 0.0; test ah,5; jp` loads 0.0 for `t < 0`; `fcomp 1.0; test
/// ah,0x41; jne` loads 1.0 only for `t > 1`; less, equal and NaN take the cube `t·t·t`, which stays
/// on the stack at 53 bits.
pub fn cubic_ease_f64(t: f32) -> f64 {
    let tf = f64::from(t);
    if tf < 0.0 {
        return 0.0;
    }
    if tf > 1.0 {
        return 1.0;
    }
    tf * tf * tf
}

/// The ease weight rounded to `f32`.
pub fn cubic_ease(t: f32) -> f32 {
    cubic_ease_f64(t) as f32
}

/// The interpolation of `0x614a90`, `0x614b3b` to `0x614b9c`: `fld target; fsub prev; fmul eased;
/// fadd prev; fstp dword [esi+0xf4]` with `eased` from [`cubic_ease_f64`] unrounded, so the result
/// is rounded to `f32` once.
pub fn anim_interp(t: f32, target: f32, prev: f32) -> f32 {
    let eased = cubic_ease_f64(t);
    let acc = (f64::from(target) - f64::from(prev)) * eased + f64::from(prev);
    acc as f32
}

/// The oscillation phase of `0x614a90`, `0x614bdb` to `0x614bee`: `fild elapsed; fmul 0.0005; fmul
/// π; fstp dword`, in that order.
pub fn osc_phase_angle(elapsed: i32) -> f32 {
    let a = f64::from(elapsed) * f64::from(MS_SCALE) * f64::from(PI_C);
    a as f32
}

/// The frame index of `PlayBowPull` (`0x624cc0`), `0x624d75` to `0x624d7f`: `fild qword [ebp-8];
/// fdiv dword [ebp-0x24]` and `__ftol`; a zero divisor gives the integer indefinite.
pub fn castbar_frame_index(num: i64, den: f32) -> i64 {
    let q = num as f64 / f64::from(den);
    ftol64(q)
}

/// The animation speed of `0x624cc0`, `0x624e12` to `0x624e26`: `fild qword [ebp-0x10]; fidiv dword
/// [ebp-4]; fstp dword [esp]`.
pub fn castbar_anim_speed(frames: i64, total: i32) -> f32 {
    let s = frames as f64 / f64::from(total);
    s as f32
}

/// The repair cost of `CGItem_C` `0x5da330`, `0x5da4ab` to `0x5da4d4`: `fild n; fmul factor; fimul
/// classval` on the stack, `fcom 0.0; test ah,0x41; jne` adds 0.5 only for a product above 0 and
/// subtracts it for zero, below and NaN, and `__ftol` truncates, so the result rounds half away
/// from zero.
pub fn random_prop_delta(n: i32, factor: f32, classval: i32) -> i64 {
    let val = f64::from(n) * f64::from(factor) * f64::from(classval);
    let biased = if val > 0.0 {
        val + f64::from(HALF)
    } else {
        val - f64::from(HALF)
    };
    ftol64(biased)
}

/// The rest multiplier gate of `LogExhaustionXpGain` (`0x637ac0`), `0x637b3e` to `0x637b7b`: `fld
/// [ecx+8]; fabs; fcomp 2^-22` and `fld [ecx+8]; fsub 1.0; fabs; fcomp 2^-22`, each with `test
/// ah,5; jnp 0x637ca4`, which skips the line for `|mult| < 2^-22` and for `|mult - 1| < 2^-22`;
/// equal and NaN go on. Returns whether it goes on.
pub fn xpgain_rest_meaningful(mult: f32) -> bool {
    let m = f64::from(mult);
    let eps = f64::from(REST_EPS);
    if m.abs() < eps {
        return false;
    }
    if (m - 1.0).abs() < eps {
        return false;
    }
    true
}

/// The member count of `0x637ac0`, `0x637b70` to `0x637b7b`: `fild dword [ebp+0xc]; fdiv dword
/// [ebp+0x10]` and `__ftol`.
pub fn xpgain_count_ratio(p2: i32, p3: f32) -> i64 {
    let q = f64::from(p2) / f64::from(p3);
    ftol64(q)
}
