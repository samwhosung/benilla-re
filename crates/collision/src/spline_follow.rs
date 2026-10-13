//! Stretches of `CMovement::FollowSpline` (`0x7c5490`,
//! `systems/collision/kinematics.md#spline-follower`): the spline's time fractions, the transport
//! heading, the turn toward the next point, the rotated offset and the output point.

#![allow(clippy::manual_clamp)]

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// 1.0.
const ONE: f64 = 1.0;

/// `[0x8029d4]`: 2^-22, the length below which a vector is left as it is.
const EPS_LEN: f64 = f32::from_bits(0x3480_0000) as f64;

/// `[0x808acc]`: π/2.
const HALF_PI_F32: f32 = f32::from_bits(0x3fc9_0fdb);
/// `[0x80c5e4]`: −π/2.
const NEG_HALF_PI_F32: f32 = f32::from_bits(0xbfc9_0fdb);

/// π (`0x40490fdb`).
const PI_F32: f32 = f32::from_bits(0x4049_0fdb);
/// `[0xcf5d00]`: 2π as its static initializer computes it, π + π stored as `f32`.
pub const TWO_PI_F32: f32 = (PI_F32 as f64 + PI_F32 as f64) as f32;

/// `0x7c5509`: `numer / denom` (`fild; fidiv`), stored as `f32`.
pub fn frac1(numer: i32, denom: i32) -> f32 {
    ((numer as f64) / (denom as f64)) as f32
}

/// `0x7c562c`: the same quotient clamped to `[0, 1]` in the register, stored as `f32`.
pub fn frac2(numer: i32, denom: i32) -> f32 {
    let q = (numer as f64) / (denom as f64);
    if q < 0.0 {
        0.0
    } else if q > 1.0 {
        1.0
    } else {
        q as f32
    }
}

/// `0x7c55a5`: `atan2(y, x)` (`fpatan`) stored at `+0x1c`; below 0 (tested in the register, after
/// the `fst`) it adds 2π and stores again.
pub fn facing_wrap(y: f32, x: f32) -> f32 {
    let a = f(y).atan2(f(x));
    if a < 0.0 {
        (a + f(TWO_PI_F32)) as f32
    } else {
        a as f32
    }
}

/// `Vector2LengthSquared` (`0x454980`): `x·x + y·y`, unrounded.
#[inline]
fn dot2(x: f32, y: f32) -> f64 {
    f(x) * f(x) + f(y) * f(y)
}

/// The turn's stores: the half-angle at `[ebp+8]` and the two unit vectors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurnResult {
    pub dir: [f32; 2],
    pub delta: [f32; 2],
    pub angle: f32,
}

/// `0x7c569c`: `delta = sampled − xform` stored as `f32`; `dir` and `delta` normalized unless
/// shorter than 2^-22, `1/len` kept in the register; `dot` clamped to `[−1, 1]`, the angle
/// `2·acos(dot)`, negated unless the cross product is below 0, then clamped to ±π/2.
pub fn turn(dir: [f32; 2], sampled: [f32; 2], xform: [f32; 2]) -> TurnResult {
    let mut dl = [
        (f(sampled[0]) - f(xform[0])) as f32,
        (f(sampled[1]) - f(xform[1])) as f32,
    ];

    let mut d = dir;
    {
        let len = dot2(d[0], d[1]).sqrt();
        if len.abs() >= EPS_LEN {
            let inv = ONE / len;
            d[0] = (f(d[0]) * inv) as f32;
            d[1] = (f(d[1]) * inv) as f32;
        }
    }

    {
        let len = dot2(dl[0], dl[1]).sqrt();
        if len.abs() >= EPS_LEN {
            let inv = ONE / len;
            dl[0] = (f(dl[0]) * inv) as f32;
            dl[1] = (f(dl[1]) * inv) as f32;
        }
    }

    let dot = (f(dl[1]) * f(d[1]) + f(dl[0]) * f(d[0])) as f32;
    let cross = f(dl[1]) * f(d[0]) - f(d[1]) * f(dl[0]);

    let dot_c: f32 = if dot < -1.0 {
        -1.0
    } else if dot > 1.0 {
        1.0
    } else {
        dot
    };

    let acos = f(dot_c).acos();
    let angle_pre = if cross < 0.0 { acos } else { -acos };
    let angle = angle_pre + angle_pre;

    let angle_f32: f32 = if angle < f(NEG_HALF_PI_F32) {
        NEG_HALF_PI_F32
    } else if angle > f(HALF_PI_F32) {
        HALF_PI_F32
    } else {
        angle as f32
    };

    TurnResult {
        dir: d,
        delta: dl,
        angle: angle_f32,
    }
}

/// `0x7c57d9`: the rotation `0x7be490` built times `v`, each output `v.x·m + v.y·m + v.z·m` in the
/// register, stored as `f32`.
pub fn mat3_mul_vec(m: &[f32; 9], v: &[f32; 3]) -> [f32; 3] {
    let vx = f(v[0]);
    let vy = f(v[1]);
    let vz = f(v[2]);
    let out_x = (vx * f(m[6]) + vy * f(m[3]) + vz * f(m[0])) as f32;
    let out_y = (vx * f(m[7]) + vy * f(m[4]) + vz * f(m[1])) as f32;
    let out_z = (vx * f(m[8]) + vy * f(m[5]) + vz * f(m[2])) as f32;
    [out_x, out_y, out_z]
}

/// `0x7c5841`: `pos − reference` (`+0x44`) per component, stored as `f32`.
pub fn relative_out(pos: &[f32; 3], reference: &[f32; 3]) -> [f32; 3] {
    [
        (f(pos[0]) - f(reference[0])) as f32,
        (f(pos[1]) - f(reference[1])) as f32,
        (f(pos[2]) - f(reference[2])) as f32,
    ]
}
