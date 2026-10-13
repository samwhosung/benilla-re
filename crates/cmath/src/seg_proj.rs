//! The squared distance from a point to a segment (`0x7c46d0`), with the clamped parameter.
//!
//! `__thiscall`: `ecx` = the segment `{a at +0, b at +0xc}`, `edx` = the point `q`, `[ebp+8]` =
//! `out_t`, which may be null; `ret 4`; the squared distance is left in `st0`. Its one caller,
//! `0x6c4300`, takes the square root of `st0` unrounded. `q − a` and `b − a` are rounded to `f32`
//! first, and so is the dot product.

#[inline]
fn f(x: f32) -> f64 {
    f64::from(x)
}

/// What `0x7c46d0` gives: `t` as `out_t` stores it, and the squared distance as `st0` holds it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegProj {
    pub t: f32,
    pub dist_sq: f64,
}

/// `0x7c46d0`: `dot = (q − a)·(b − a)`. `dot ≤ 0` gives `t = 0` and the distance to `a`;
/// otherwise (also NaN) `dot > |b − a|²` gives `t = 1` and the distance to `b`, and anything else
/// `t = dot/|b − a|²`. `seg` is `[a, b]`.
pub fn seg_proj(seg: &[f32; 6], q: &[f32; 3]) -> SegProj {
    let qa = [
        (f(q[0]) - f(seg[0])) as f32,
        (f(q[1]) - f(seg[1])) as f32,
        (f(q[2]) - f(seg[2])) as f32,
    ];
    let d = [
        (f(seg[3]) - f(seg[0])) as f32,
        (f(seg[4]) - f(seg[1])) as f32,
        (f(seg[5]) - f(seg[2])) as f32,
    ];

    let (ax, ay, az) = (f(qa[0]), f(qa[1]), f(qa[2]));
    let (dx, dy, dz) = (f(d[0]), f(d[1]), f(d[2]));

    let dot = f((((dz * az) + (dy * ay)) + (ax * dx)) as f32);

    // fcomp 0.0; test ah,0x41; jp: dot > 0 or NaN goes on to the length.
    if dot > 0.0 || dot.is_nan() {
        interior_or_end(dot, ax, ay, az, dx, dy, dz)
    } else {
        finish(0.0, ax, ay, az)
    }
}

/// From `0x7c474a`: `|b − a|²`, then `t = 1` for `dot > |b − a|²` and `dot/|b − a|²` otherwise,
/// NaN included (`test ah,0x41; jne`).
#[inline]
fn interior_or_end(dot: f64, ax: f64, ay: f64, az: f64, dx: f64, dy: f64, dz: f64) -> SegProj {
    let len_sq = ((dz * dz) + (dy * dy)) + (dx * dx);

    if dot > len_sq {
        finish(1.0, ax - dx, ay - dy, az - dz)
    } else {
        let t = dot / len_sq;
        // dx·t stays in the register; dy·t and dz·t are stored as f32 and reloaded.
        let rel_x = ax - (dx * t);
        let rel_y = ay - f((dy * t) as f32);
        let rel_z = az - f((dz * t) as f32);
        finish(t, rel_x, rel_y, rel_z)
    }
}

/// `0x7c47b1`: `out_t` is `t` rounded to `f32`; the squared distance `(z² + x²) + y²` stays in
/// `st0`.
#[inline]
fn finish(t: f64, rel_x: f64, rel_y: f64, rel_z: f64) -> SegProj {
    let dist_sq = ((rel_z * rel_z) + (rel_x * rel_x)) + (rel_y * rel_y);
    SegProj {
        t: t as f32,
        dist_sq,
    }
}
