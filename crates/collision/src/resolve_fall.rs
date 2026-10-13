//! The fall time for a drop and the apex time (`systems/collision/falling.md`).

#![allow(clippy::neg_cmp_op_on_partial_ord)]

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// `[0x80e020]`: 1/g.
const INV_G: f32 = f32::from_bits(0x3d54_536a);
/// 2/g (`0x3dd4536a`).
const TWO_OVER_G: f32 = f32::from_bits(0x3dd4_536a);
/// 2g (`0x421a542f`).
const TWO_G: f32 = f32::from_bits(0x421a_542f);
/// `[0x81da60]`: g/2.
const HALF_G: f32 = f32::from_bits(0x411a_542f);
/// 2^-22, below which the fall starts from rest.
const LEN_GUARD: f64 = 2.384_185_8e-7_f32 as f64;

/// `CMovement::GetFallTime` (`0x7c5f50`, `systems/collision/falling.md#fall-time-for-a-drop`): the
/// time to fall `d` from the speed `fa0`, capped at the terminal speed for flags & 0x20000000. From
/// rest it is `√(2d/g)`, or past the terminal distance the time to reach it plus the rest at that
/// speed; otherwise from `vf = √(v² + 2gd)`, with the same crossover. A nonzero `flag` returns the
/// earlier root `(−v − vf)/g`, stored as `f32`, or 0 when negative. Unrounded.
pub fn fall_time_solve(
    fa0: f32,
    flags: u32,
    vmax_land: f32,
    vmax_swim: f32,
    d: f32,
    flag: u32,
) -> f64 {
    let vmax = if flags & 0x2000_0000 != 0 {
        vmax_swim
    } else {
        vmax_land
    };
    let mut v = fa0;
    if !(f(v) <= f(vmax)) && !f(v).is_nan() {
        v = vmax;
    }
    if f(v).abs() < LEN_GUARD {
        let t_term = f(INV_G) * f(vmax);
        let d_term = (t_term * f(vmax)) * 0.5;
        if !(f(d) < d_term) || f(d).is_nan() {
            return (f(d) - d_term) / f(vmax) + t_term;
        }

        if !(f(d) > 0.0) {
            return 0.0;
        }
        (f(d) * f(TWO_OVER_G)).sqrt()
    } else {
        let s = f(v) * f(v) + f(d) * f(TWO_G);
        let vf = if !(s > 0.0) { 0.0 } else { s.sqrt() };
        let ta = ((-f(v) - vf) * f(INV_G)) as f32;
        let t1 = (vf - f(v)) * f(INV_G);
        let t_term2 = ((f(vmax) - f(v)) * f(INV_G)) as f32;
        let time = if !(t1 > f(t_term2)) || t1.is_nan() {
            t1
        } else {
            let d1 = (f(t_term2) * f(HALF_G) + f(v)) * f(t_term2);
            (f(d) - d1) / f(vmax) + f(t_term2)
        };
        if flag == 0 {
            time
        } else if !(f(ta) < 0.0) || f(ta).is_nan() {
            f(ta)
        } else {
            0.0
        }
    }
}

/// `CMovement::GetApexTime` (`0x7c5da0`, `systems/collision/falling.md#apex-time-and-rising-test`):
/// `fa0·(−1/g)` (`[0x81da5c]`) with flags & 0x2000, else 0; unrounded.
pub fn swim_fall_vel(fa0: f32, flags: u32) -> f64 {
    if flags & 0x2000 != 0 {
        f(fa0) * f(f32::from_bits(0xbd54_536a))
    } else {
        0.0
    }
}
