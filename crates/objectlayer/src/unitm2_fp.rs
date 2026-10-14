//! Float kernels of unit methods in `[0x60a110, 0x611130]`: `CGUnit_C::BuildSwimTransform`
//! (`0x60a110`), `CGUnit_C::IsPointAtLiquidSurface` (`0x60a740`), `CGUnit_C::ComputeSelectionRadius`
//! (`0x60aee0`), `CGUnit_C::UpdateTint` (`0x60cb50`), `CGUnit_C::GetPulseColor` (`0x60cbf0`),
//! `CGUnit_C::GetSpellCastTime` (`0x60d670`), `CGUnit_C::WalkToGameObject` (`0x610300`),
//! `CGUnit_C::AutoMoveTurnToward` (`0x6103d0`), `PitchTowardPoint` (`0x610da0`) and
//! `CGUnit_C::StartAutoMove` (`0x611130`) (`systems/objectlayer/unitmovement.md#facing`,
//! `#starting-and-ending-auto-move` and `#steering-an-auto-move`, `unitbody.md#liquid-and-swimming`,
//! `unitdisplay.md#selection-circle-and-colour`, `auras.md#kit-character-procedures` and
//! `#precast-and-spell-visual-choice`).
//!
//! The x87 runs at 53 bits: each `f32` operand is widened to `f64`, the arithmetic is `f64` in the
//! code's order, and a result is rounded to `f32` only at an `fst dword` or `fstp dword`; `fild`
//! and `fimul` widen an integer. The calls to the angle wrap `0x455b30`, `__CIasin` (`0x7405a0`)
//! and the render maths `0x7bf4b0` and `0x7bdd00` are other code: the tests stub them or run their
//! bytes, and the kernels here are the arithmetic around them.

use crate::x87::{fistp, ftol};

/// `_DAT_00804588` (`0x3f666666`): 0.9, the scale of the arrival radius in `0x610300`.
pub const SCALE_0P9: f32 = f32::from_bits(0x3f66_6666);
/// `_DAT_008026c8` (`0x3b808081`): 1/255, the scale of a colour byte in `0x60cb50`.
pub const INV_255: f32 = f32::from_bits(0x3b80_8081);
/// `_DAT_007ff9d8` (`0x3f800000`): 1.0.
pub const ONE: f32 = 1.0;
/// `_DAT_007ffa24` (`0x3f000000`): 0.5, the factor under the square roots of `0x60aee0`.
pub const HALF: f32 = f32::from_bits(0x3f00_0000);
/// `_DAT_00801360` (`0x3a83126f`): 0.001, the dead band of the facing and of `0x610da0`.
pub const EPS_1EM3: f32 = f32::from_bits(0x3a83_126f);
/// `_DAT_008029d4` (`0x34800000`): 2^-22, the distance from 1.0 within which `0x60d670` keeps the
/// base.
pub const RATE_EPS: f32 = f32::from_bits(0x3480_0000);
/// `_DAT_008029d0` (`0x3c23d70a`): 0.01, added to the probe z in `0x60a740`.
pub const Z_BIAS_0P01: f32 = f32::from_bits(0x3c23_d70a);
/// `_DAT_0080abfc` (`0x3f2aaaab`): 2/3, the height `0x60a740` allows above a point.
pub const TWO_THIRDS: f32 = f32::from_bits(0x3f2a_aaab);
/// `_DAT_007ffe58` (`0x437f0000`): 255.0, the span of the fade parameter in `0x60cbf0`.
pub const C255: f32 = f32::from_bits(0x437f_0000);
/// `_DAT_0080295c` (`0x40c90fdb`): 2π as `f32`.
pub const TAU: f32 = f32::from_bits(0x40c9_0fdb);
/// `_DAT_008030b0` (`0x40490fdb`): π as `f32`.
pub const PI: f32 = f32::from_bits(0x4049_0fdb);
/// `_DAT_00808acc` (`0x3fc90fdb`): π/2, the pitch `0x610da0` returns for a rise.
pub const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
/// `_DAT_0080c5e4` (`0xbfc90fdb`): −π/2, the pitch `0x610da0` returns otherwise.
pub const NEG_HALF_PI: f32 = f32::from_bits(0xbfc9_0fdb);
/// `0x3f99999a`: 1.2, the model scale `0x60aee0` writes when both extents are flat.
pub const DEFAULT_SCALE: f32 = f32::from_bits(0x3f99_999a);
/// The double 2π (`0x401921fb60000000`) that `0x455b30` pushes with `fld qword [0x803090]`.
pub const TAU_F64: f64 = f64::from_bits(0x4019_21fb_6000_0000);

/// The arrival radius of `CGUnit_C::WalkToGameObject` (`0x610300`), `0x610376` to `0x610383`: `fld
/// dist; fmul 0.9; fstp dword [0xc4d998]`.
pub fn interact_arrival_scale(dist: f32) -> f32 {
    (f64::from(dist) * f64::from(SCALE_0P9)) as f32
}

/// The two tests of `CGUnit_C::IsPointAtLiquidSurface` (`0x60a740`), `0x60a7a3` to `0x60a7ca`, from
/// the probe height `probe_z` (`[ebp-4]`) and `cam_z` (`[ecx+8]`). `fld probe; fadd 0.01; fcomp
/// cam; test ah,0x41; jne` fails for `probe + 0.01 <= cam` and NaN. `fld probe; fsub cam; fcomp
/// 2/3; test ah,5; jp` fails for a difference of 2/3 or more, and NaN: only a difference below 2/3
/// passes. Both sums stay on the stack unrounded. Returns whether it passes.
pub fn occlusion_z_gate(probe_z: f32, cam_z: f32) -> bool {
    let a = f64::from(probe_z) + f64::from(Z_BIAS_0P01);
    let cz = f64::from(cam_z);
    let test1_fail = a.is_nan() || cz.is_nan() || a <= cz;
    if test1_fail {
        return false;
    }
    let d = f64::from(probe_z) - cz;
    let t = f64::from(TWO_THIRDS);
    !d.is_nan() && !t.is_nan() && d < t
}

/// The rescale of `CGUnit_C::GetSpellCastTime` (`0x60d670`), `0x60d70b` to `0x60d738`, from `rate =
/// [fields+0x22c]` and the integer `base`. `fld rate; fsub 1.0; fabs; fcomp 2^-22; test ah,5; jnp`
/// skips for `|rate - 1| < 2^-22` and keeps `base`; otherwise `fld rate; fimul base; fstp dword
/// [ebp+8]; fld dword [ebp+8]; fistp dword [ebp-4]` rounds the `f32` product to the nearest
/// integer, ties to even.
pub fn anim_timer_rate(rate: f32, base: i32) -> i32 {
    let diff = (f64::from(rate) - f64::from(ONE)).abs();
    if diff < f64::from(RATE_EPS) {
        return base;
    }
    let scaled_f32 = (f64::from(rate) * f64::from(base)) as f32;
    fistp(f64::from(scaled_f32))
}

/// The goal radius of `CGUnit_C::StartAutoMove` (`0x611130`), `0x6111a3` to `0x611205`: `fld
/// [0xc4d9c0]; fsqrt; fstp dword [0xc4d910]`, the root of the squared radius `0x611240` stored.
pub fn move_cast_radius(radius_sq: f32) -> f32 {
    f64::from(radius_sq).sqrt() as f32
}

/// The fade parameter of `CGUnit_C::GetPulseColor` (`0x60cbf0`), `0x60cc48` to `0x60cc60`: `fild
/// qword` of `dt` (`now` less the start and the hold), `fidiv [this+0xd00]`, `fsubr 1.0`, `fmul
/// 255.0` and `__ftol`, so `(1.0 - dt / dur)·255.0` at 53 bits truncated toward zero.
pub fn color_fade_t(dt: i64, dur: i32) -> i32 {
    let ratio = dt as f64 / f64::from(dur);
    let v = (f64::from(ONE) - ratio) * f64::from(C255);
    ftol(v)
}

/// The tint of `CGUnit_C::UpdateTint` (`0x60cb50`), `0x60cba5` to `0x60cbd6`: three colour bytes,
/// each `fild; fmul 1/255; fstp dword`, in the order the model light receives them. Returns the
/// three channels.
pub fn fade_model_rgb(bytes_in_light_order: [u8; 3]) -> [f32; 3] {
    let chan = |b: u8| (f64::from(i32::from(b)) * f64::from(INV_255)) as f32;
    bytes_in_light_order.map(chan)
}

/// The scale of `CGUnit_C::ComputeSelectionRadius` (`0x60aee0`), `0x60af4f` to `0x60afac`, from the
/// bounds dwords `[ebp-0xc]` (`xhi`), `[ebp-0x18]` (`xlo`), `[ebp-8]` (`yhi`) and `[ebp-0x14]`
/// (`ylo`). Each `fcomp a, b; test ah,0x44; jp` goes on to the computation for an even count of C3
/// and C2, which is greater, less and NaN; only equal falls through. The default 1.2 is stored when
/// both extents are equal. The computation is `sqrt(0.5·sqrt(dy² + dx²))`.
pub fn camera_model_scale(xhi: f32, xlo: f32, yhi: f32, ylo: f32) -> f32 {
    if fcomp_jp_even(f64::from(xhi), f64::from(xlo)) {
        return compute_scale(xhi, xlo, yhi, ylo);
    }
    if fcomp_jp_even(f64::from(yhi), f64::from(ylo)) {
        return compute_scale(xhi, xlo, yhi, ylo);
    }
    DEFAULT_SCALE
}

fn compute_scale(xhi: f32, xlo: f32, yhi: f32, ylo: f32) -> f32 {
    let dx = f64::from(xhi) - f64::from(xlo);
    let dy = f64::from(yhi) - f64::from(ylo);
    let sum = dy * dy + dx * dx;
    ((sum.sqrt() * f64::from(HALF)).sqrt()) as f32
}

/// `PitchTowardPoint` (`0x610da0`) with `__CIasin` (`0x7405a0`) left out: the argument it would
/// take is returned. `d = b - a`; `fst dword [ebp-8]` rounds `dz` and keeps the register, so `dz²`
/// is `dz·dz_f32`; the length is `sqrt((dz² + dy²) + dx²)` and `inv = 1.0 / len` is stored to a
/// dword. `nx = inv·dx` uses the register, `ny = inv_f32·dy` the stored value and the asin argument
/// is `inv_f32·dz_f32`. Past the tests `fabs nx; fcomp 0.001; test ah,5; jp` and the same for `ny`,
/// which leave for the asin unless both are below 0.001, the result is `+π/2` when `b.z > a.z` and
/// `-π/2` otherwise (`fcomp; test ah,0x41; jne`).
pub fn vec_to_facing(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = f64::from(b[0]) - f64::from(a[0]);
    let dy = f64::from(b[1]) - f64::from(a[1]);
    let dz = f64::from(b[2]) - f64::from(a[2]);
    let dz_f32 = dz as f32;
    let dz_sq = dz * f64::from(dz_f32);
    let len = (dz_sq + dy * dy + dx * dx).sqrt();
    let inv = f64::from(ONE) / len;
    let inv_f32 = inv as f32;
    let nx = (inv * dx) as f32;
    let ny = f64::from(inv_f32) * dy;
    let atan2_input = (f64::from(inv_f32) * f64::from(dz_f32)) as f32;
    let eps = f64::from(EPS_1EM3);
    let nx_tiny = (f64::from(nx)).abs() < eps;
    let ny_tiny = ny.abs() < eps;
    if nx_tiny && ny_tiny {
        let bz = f64::from(b[2]);
        let az = f64::from(a[2]);
        if bz.is_nan() || az.is_nan() || bz <= az {
            NEG_HALF_PI
        } else {
            HALF_PI
        }
    } else {
        atan2_input
    }
}

/// The angle wrap `0x455b30` into `[0, 2π)`: `fmod(x, 2π)` with the double 2π, and 2π added when
/// the result is below 0.
pub fn angle_to_0_2pi(x: f32) -> f32 {
    angle_to_0_2pi_f64(x) as f32
}

/// The wrap as `0x455b30` leaves it in `st0`, at 53 bits and before the caller rounds it.
pub fn angle_to_0_2pi_f64(x: f32) -> f64 {
    let r = f64::from(x) % TAU_F64;
    if r < 0.0 {
        r + TAU_F64
    } else {
        r
    }
}

/// The new facing of `CGUnit_C::AutoMoveTurnToward` (`0x6103d0`) handed to `0x60e940`, from the
/// facing `cur` (`[vtable+0x18]`), the `goal` and `turn_rate`, or `None` when it returns false.
/// `delta = wrap(goal - cur)`, with `fst dword [ebp+8]` keeping the register. `fabs; fcomp 0.001;
/// test ah,5; jnp` returns false for a live `|delta| < 0.001`, and the same on the stored delta
/// less 2π. The step is `2π - delta` for `delta > π` and `delta` otherwise (`fcomp π; test ah,0x41;
/// jne`), and `fcom rate; test ah,5; jp` stores it as the rate only when it is below the rate. The
/// new facing is `wrap(cur - capped)` for `delta > π` and `wrap(cur + capped)` otherwise. The fill
/// of the cached rate `[0xc4d93c]` from a console variable at `0x61042c` does not touch the result.
pub fn facing_interp(cur: f32, goal: f32, turn_rate: f32) -> Option<f32> {
    let eps = f64::from(EPS_1EM3);
    let delta0 = (f64::from(goal) - f64::from(cur)) as f32;
    let delta_live = angle_to_0_2pi_f64(delta0);
    let delta = delta_live as f32;
    if delta_live.abs() < eps {
        return None;
    }
    if (f64::from(delta) - f64::from(TAU)).abs() < eps {
        return None;
    }
    let step_live = if f64::from(delta) > f64::from(PI) {
        f64::from(TAU) - f64::from(delta)
    } else {
        f64::from(delta)
    };
    let capped = if step_live < f64::from(turn_rate) {
        step_live as f32
    } else {
        turn_rate
    };
    let new = if f64::from(delta) > f64::from(PI) {
        (f64::from(cur) - f64::from(capped)) as f32
    } else {
        (f64::from(cur) + f64::from(capped)) as f32
    };
    Some(angle_to_0_2pi(new))
}

/// The three scalars `CGUnit_C::BuildSwimTransform` (`0x60a110`) passes to the render maths,
/// `0x60a11a`, `0x60a175` and `0x60a209` to `0x60a228`: `arg1 = 2π - facing` (`facing =
/// this[+0x9c8]`), `arg2 = -(2π - vcall44)` (`fsubr 2π; fchs` on the `[vtable+0x44]` scalar) and
/// `arg3 = (s_xy_a·s_xy_b)·s_z` with `s_xy_a = this[+0x94]`, `s_xy_b = this[+0x90]` and `s_z` the
/// `this[+0x9c]` for a non-zero `this[+0xdc]` and `this[+0x98]` otherwise. Each is stored as a
/// dword on the stack.
pub fn swing_transform_args(
    facing: f32,
    vcall44: f32,
    s_xy_a: f32,
    s_xy_b: f32,
    s_z: f32,
) -> (f32, f32, f32) {
    let arg1 = (f64::from(TAU) - f64::from(facing)) as f32;
    let arg2 = (-(f64::from(TAU) - f64::from(vcall44))) as f32;
    let arg3 = ((f64::from(s_xy_a) * f64::from(s_xy_b)) * f64::from(s_z)) as f32;
    (arg1, arg2, arg3)
}

/// `fcomp a ? b` then `test ah,0x44; jp <target>`: whether the `jp` is taken, an even count of C3
/// (`0x40`) and C2 (`0x04`). Greater and less are 0 bits, equal is C3 alone (odd), and unordered is
/// C3 and C2 (even).
fn fcomp_jp_even(a: f64, b: f64) -> bool {
    if a.is_nan() || b.is_nan() {
        return true;
    }
    a != b
}
