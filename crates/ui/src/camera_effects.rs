//! The camera's effects (`systems/uicamera/effects.md`): evaluating one shake, the walking bob and
//! the slope probe that tilts the view on hills.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fst`/`fstp dword` is an
//! `as f32`. `fsin`, `fcos` and `fsincos` are the host's `sin`, `cos` and `sin_cos`, and the shake's
//! `2^x` (`fldl2e`, `f2xm1`, `fscale`) is `exp2`; the emulator routes them through the host's maths
//! library as well, so the results match bit for bit.

/// The constants the shake and the bob read, each an `f32` in `.rdata`.
mod c {
    /// 2π at `0x80295c`.
    pub const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);
    /// π/2 at `0x808acc`.
    pub const HALF_PI: f32 = f32::from_bits(0x3fc9_0fdb);
    /// 0.001 at `0x801360`: seconds per tick.
    pub const DT: f32 = f32::from_bits(0x3a83_126f);
    /// 1/36 at `0x7ff9d0`: the bob's amplitude scale.
    pub const INV36: f32 = f32::from_bits(0x3ce3_8e39);
    /// 0.5 at `0x8089a0`: the bob speed ratio's lower bound.
    pub const SPEED_LO: f32 = f32::from_bits(0x3f00_0000);
    /// 1.5 at `0x80899c`: the bob speed ratio's upper bound.
    pub const SPEED_HI: f32 = f32::from_bits(0x3fc0_0000);
    /// 7.2 at `0x808a08`: the speed that gives a ratio of 1.
    pub const SPEED_DIV: f32 = f32::from_bits(0x40e6_6666);
}

/// Which way a shake's amplitude is applied: the switch `0x511653` makes on the eighth argument
/// of `0x511610`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShakeAxis {
    /// 0: adds `(a cos φ, a sin φ)` to `(x, y)`.
    Cos,
    /// 1: adds the same turned by π/2, with `φ + π/2` rounded to `f32` first.
    CosQuarter,
    /// 2: adds `a` to `z`.
    Z,
    /// Any other value adds nothing.
    None,
}

/// `EvaluateShake` (`0x511610`): adds one shake's value to `out` (`effects#evaluating-one-shake`).
///
/// `a = sin(freq * t * 2π) * amp`, rounded to `f32`. With `decay`, `a` is multiplied by
/// `2^(-(freq * decay_t) * log2(e))` (`fldl2e`, `f2xm1`, `fscale`) and rounded again. Then the axis
/// adds `a` to `out`, each component rounded to `f32`; the cosine and the sine of `phase` are
/// each rounded to `f32` and reloaded before they multiply.
#[allow(clippy::too_many_arguments)]
pub fn evaluate_shake(
    mut out: [f32; 3],
    phase: f32,
    amp: f32,
    freq: f32,
    t: f32,
    decay: bool,
    decay_t: f32,
    axis: ShakeAxis,
) -> [f32; 3] {
    let mut a =
        ((f64::from(freq) * f64::from(t) * f64::from(c::TWO_PI)).sin() * f64::from(amp)) as f32;
    if decay {
        let x = -(f64::from(freq) * f64::from(decay_t)) * std::f64::consts::LOG2_E;
        a = (x.exp2() * f64::from(a)) as f32;
    }
    match axis {
        ShakeAxis::Z => {
            out[2] = (f64::from(a) + f64::from(out[2])) as f32;
        }
        ShakeAxis::None => {}
        ShakeAxis::Cos | ShakeAxis::CosQuarter => {
            let arg = match axis {
                ShakeAxis::CosQuarter => {
                    f64::from((f64::from(phase) + f64::from(c::HALF_PI)) as f32)
                }
                _ => f64::from(phase),
            };
            let cos = arg.cos() as f32;
            let sin = arg.sin() as f32;
            out[0] = (f64::from(cos) * f64::from(a) + f64::from(out[0])) as f32;
            out[1] = (f64::from(sin) * f64::from(a) + f64::from(out[1])) as f32;
        }
    }
    out
}

/// `CCamera::ComputeBob` (`0x511920`): the bob offset written to `+0x110..+0x118`
/// (`effects#bobbing`), from the body reached once the unit lookup (`0x468460`) and the gate
/// (`0x5105e0`) have passed.
///
/// `phase` is the unit's facing (`[unit+0xc98]`); `amp_h_cvar`, `freq_cvar` and `amp_v_cvar` are the
/// three bobbing CVars; `speed_raw` is the movement speed (`0x7c4c90`'s result); `tick` is the
/// clock (`0x42c010`) and `last` the bob's start time (`+0x238`).
///
/// `amp_h = (amp_h_cvar / 36) as f32` and `amp_v = (amp_v_cvar / 36) as f32`;
/// `speed = clamp(speed_raw / 7.2, 0.5, 1.5)`; `freq = (speed * freq_cvar) as f32`;
/// `t = (tick - last) as u32 * 0.001`, rounded to `f32`. `x` and `y` are
/// `sin(freq * t * 2π) * amp_h` times the cosine and sine of `phase + π/2` (the sum rounded to
/// `f32`), and `z = sin((freq + freq) * t * 2π) * amp_v`; each is rounded to `f32`.
pub fn compute_bob(
    phase: f32,
    amp_h_cvar: f32,
    freq_cvar: f32,
    amp_v_cvar: f32,
    speed_raw: f32,
    tick: u32,
    last: u32,
) -> [f32; 3] {
    let amp_h = (f64::from(amp_h_cvar) * f64::from(c::INV36)) as f32;
    let amp_v = (f64::from(amp_v_cvar) * f64::from(c::INV36)) as f32;

    // The low test is a strict ordered `<`; the high test is `!(speed < 1.5)`, so a ratio of 1.5
    // or more and a NaN both take 1.5.
    let mut speed = f64::from(speed_raw) / f64::from(c::SPEED_DIV);
    if speed < f64::from(c::SPEED_LO) {
        speed = f64::from(c::SPEED_LO);
    } else if !matches!(
        speed.partial_cmp(&f64::from(c::SPEED_HI)),
        Some(std::cmp::Ordering::Less)
    ) {
        speed = f64::from(c::SPEED_HI);
    }
    let freq = (speed * f64::from(freq_cvar)) as f32;

    // `fild` of a quadword with a zero high dword: the tick difference is read as unsigned.
    let dticks = tick.wrapping_sub(last);
    let t = (f64::from(dticks) * f64::from(c::DT)) as f32;

    let horiz =
        ((f64::from(freq) * f64::from(t) * f64::from(c::TWO_PI)).sin() * f64::from(amp_h)) as f32;

    let arg = f64::from((f64::from(phase) + f64::from(c::HALF_PI)) as f32);
    let cos = arg.cos() as f32;
    let sin = arg.sin() as f32;
    let x = (f64::from(cos) * f64::from(horiz)) as f32;
    let y = (f64::from(sin) * f64::from(horiz)) as f32;

    // The doubled frequency is `fadd st, st` at register precision.
    let two_freq = f64::from(freq) + f64::from(freq);
    let z = ((two_freq * f64::from(t) * f64::from(c::TWO_PI)).sin() * f64::from(amp_v)) as f32;

    [x, y, z]
}

// ---- the slope probe ----

const TERR_LO: f64 = f32::from_bits(0xbeb2_b8c2) as f64; // -20 degrees, 0x8089c4
const TERR_HI: f64 = f32::from_bits(0x3eb2_b8c2) as f64; // +20 degrees, 0x8089c0
/// 10/3 at `0x808a34`: how far ahead the first ray goes.
const FWD: f64 = f32::from_bits(0x4055_5555) as f64;
/// 5/3 at `0x808a38`: the height above the feet the ray starts at.
const EYE_Z: f64 = f32::from_bits(0x3fd5_5555) as f64;
/// 5/18 at `0x808ab8`: how far the hit point backs off.
const BACKOFF: f64 = f32::from_bits(0x3e8e_38e4) as f64;
/// 64/9 at `0x808ab4`: how far down the second ray goes.
const DROP: f64 = f32::from_bits(0x40e3_8e39) as f64;
/// 0 at `0x808a90`: the table's base angle.
const PITCH_TBL_BASE: f64 = 0.0;
/// The table at `0x808a40`: ten rows of (slope key, angle in radians), from 0 to 45 degrees.
const SLOPE_TBL: [(f32, f32); 10] = [
    (f32::from_bits(0x0000_0000), f32::from_bits(0x0000_0000)), // 0.00, 0 degrees
    (f32::from_bits(0x3db8_51ec), f32::from_bits(0x3db2_b8c2)), // 0.09, 5
    (f32::from_bits(0x3e38_51ec), f32::from_bits(0x3e32_b8c2)), // 0.18, 10
    (f32::from_bits(0x3e8a_3d71), f32::from_bits(0x3e86_0a92)), // 0.27, 15
    (f32::from_bits(0x3eb8_51ec), f32::from_bits(0x3eb2_b8c2)), // 0.36, 20
    (f32::from_bits(0x3ef0_a3d7), f32::from_bits(0x3edf_53e0)), // 0.47, 25
    (f32::from_bits(0x3f14_7ae1), f32::from_bits(0x3f06_0a92)), // 0.58, 30
    (f32::from_bits(0x3f33_3333), f32::from_bits(0x3f1c_6e3e)), // 0.70, 35
    (f32::from_bits(0x3f56_8f5c), f32::from_bits(0x3f32_b8c2)), // 0.84, 40
    (f32::from_bits(0x3f80_0000), f32::from_bits(0x3f49_0fdb)), // 1.00, 45
];

/// `CCamera::ProbeSlope` (`0x50d900`) with both of its rays missing (`effects#probing-the-slope`).
/// The ray `0x672170` leaves its end point as given when nothing is hit, so the geometry follows
/// from `eye` and `heading` alone; the table walk and the clamp are [`slope_to_pitch`].
///
/// The first end point is `A = eye + (cos, sin) * 10/3` at the height `eye.z + 5/3`; it backs off
/// to `B = A - (cos, sin) * 5/18`, and the second ray ends 64/9 below. The slope is
/// `((B.z - A.z) + 5/3) / sqrt((B.x - eye.x)^2 + (B.y - eye.y)^2)`, rounded to `f32`. `cos` and
/// `sin` come from `fsincos(heading)`, each rounded to `f32`; the points are stored as `f32`.
pub fn probe_slope_no_hit(eye: [f32; 3], heading: f32) -> f32 {
    let h = f64::from(heading);
    let c = h.cos() as f32;
    let s = h.sin() as f32;
    let (ex, ey, ez) = (f64::from(eye[0]), f64::from(eye[1]), f64::from(eye[2]));
    let z0 = (EYE_Z + ez) as f32;
    let ax = (ex + f64::from(c) * FWD) as f32;
    let ay = (ey + f64::from(s) * FWD) as f32;
    let bx = (f64::from(ax) - f64::from(c) * BACKOFF) as f32;
    let by = (f64::from(ay) - f64::from(s) * BACKOFF) as f32;
    let bz2 = (f64::from(z0) - DROP) as f32;
    let num = (f64::from(bz2) - f64::from(z0)) + EYE_Z;
    let dxh = f64::from(bx) - ex;
    let dyh = f64::from(by) - ey;
    let den = (dyh * dyh + dxh * dxh).sqrt();
    let slope = (num / den) as f32;
    slope_to_pitch(slope)
}

/// `CCamera::ProbeSlope` (`0x50d900`) from the points the rays write when the first ray hits: the
/// swept query `0x672170` writes the clipped hit point into its segment (`0x50da38`, `0x50daa7`),
/// and the function never tests the query's return. `p1_hit` is the first hit point
/// (`[ebp-0x5c..-0x54]`) and `p2_hit_z` the second hit's height (`[ebp-0x3c]`).
///
/// The first hit backs off by the heading: x by `cos * 5/18`, y by `sin * 5/18` (`0x50da60` to
/// `0x50da6d`), each stored as `f32`. Then
/// `slope = ((p2_hit_z - p1_hit.z) + 5/3) / sqrt((bx - eye.x)^2 + (by - eye.y)^2)`, with the
/// differences, the root and the quotient at register precision and the slope rounded to `f32`
/// (`0x50daf3`). The forward step and the second hit's x and y never reach the slope.
pub fn probe_slope_hit(eye: [f32; 3], heading: f32, p1_hit: [f32; 3], p2_hit_z: f32) -> f32 {
    let h = f64::from(heading);
    let cos = h.cos() as f32;
    let sin = h.sin() as f32;
    let bx = (f64::from(p1_hit[0]) - f64::from(cos) * BACKOFF) as f32;
    let by = (f64::from(p1_hit[1]) - f64::from(sin) * BACKOFF) as f32;
    let dx = f64::from(bx) - f64::from(eye[0]);
    let dy = f64::from(by) - f64::from(eye[1]);
    let den = (dy * dy + dx * dx).sqrt();
    let num = (f64::from(p2_hit_z) - f64::from(p1_hit[2])) + EYE_Z;
    let slope = (num / den) as f32;
    slope_to_pitch(slope)
}

/// The tail of `0x50d900` (`0x50daf8` to `0x50dbb6`) that turns a slope into the stored angle. A
/// slope of 0 or more takes sign -1 and magnitude `slope`; a smaller one (or a NaN) takes sign +1
/// and `-slope` (`fcomp 0.0`, `fchs`, `0x50dafb` to `0x50db17`). The table is scanned from index 9
/// down for the first key not above the magnitude, which index 0 (key 0) always satisfies; the
/// angle is `(row angle - base) * sign` when the row's angle is above the base, else 0. The clamp
/// to ±20 degrees compares in `f64` and stores `f32`.
fn slope_to_pitch(slope: f32) -> f32 {
    let (sign, mag) = if 0.0 <= slope {
        (-1.0f32, slope)
    } else {
        (1.0f32, -slope)
    };
    let mut pitch = 0.0f64;
    for i in (0..10usize).rev() {
        if f64::from(SLOPE_TBL[i].0) <= f64::from(mag) {
            if PITCH_TBL_BASE < f64::from(SLOPE_TBL[i].1) {
                pitch = (f64::from(SLOPE_TBL[i].1) - PITCH_TBL_BASE) * f64::from(sign);
            }
            break;
        }
    }
    if pitch < TERR_LO {
        return TERR_LO as f32;
    }
    if TERR_HI <= pitch {
        return TERR_HI as f32;
    }
    pitch as f32
}
