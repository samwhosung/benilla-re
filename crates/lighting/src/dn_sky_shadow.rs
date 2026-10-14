//! The arithmetic of `DrawBlobShadow` (`0x6d7920`, `systems/lighting/shadows.md#blob-shadow`),
//! which also dispatches gx calls, an allocator chain and runtime globals: the four corner
//! transforms of the shadow quad, the reciprocals of the box extents, the rotation by the shadow
//! direction's sine and cosine, the vector length, the gate compares and the colour quantize. Each
//! is a slice of the function's own instructions, tested against the exe. The x87 runs at 53 bits
//! and stores `f32` at each `fstp dword`; a multiply by the 0.0 constant is kept, since it carries
//! NaN, infinity and the sign of a zero.

use crate::x87::ftol;

/// `[0x7ffd74]`, 0.
const C_ZERO: f32 = f32::from_bits(0x0000_0000);
/// `[0x7ff9d8]` and `[0x811698]`, 1.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);

/// The bits of `x` stored as `f32`.
#[inline]
fn f32_bits(x: f64) -> u32 {
    (x as f32).to_bits()
}

/// `[0x8029d4]`, 2.384186e-7: the extent below which a gate stops.
const C_EPS: f32 = f32::from_bits(0x3480_0000);

/// The gate compares `|x − y|` with `[0x8029d4]` (2.384e-7) and the projection goes on unless the
/// difference is below it (`fabs; fcomp; test ah,5; jnp`), so NaN goes on. Three sites use it: the
/// entry gates at `0x6d7965` and `0x6d7984`, and `|len − 1|` at `0x6d7ad9`.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn shadow_extent_gate(x: f32, y: f32) -> bool {
    let f = f64::from;
    let d = (f(x) - f(y)).abs();
    !(d < f(C_EPS))
}

/// Each of the four corner transforms of the quad (`0x6d7b1f`, `0x6d7b81`, `0x6d7be3`, `0x6d7c45`,
/// each 0x5d bytes): with `(p, q)` the corner's two operands and `m0` to `m5` and `t0` to `t2` the
/// rows, `z = (m0·q + m3·p) + t2·0`, `y = (m1·q + m4·p) + t1·0`, `x = (m2·q + m5·p) + t0·0` at 53
/// bits, stored as `f32`.
#[allow(clippy::too_many_arguments)]
pub fn shadow_quad_corner_transform(
    p: f32,
    q: f32,
    m0: f32,
    m1: f32,
    m2: f32,
    m3: f32,
    m4: f32,
    m5: f32,
    t0: f32,
    t1: f32,
    t2: f32,
) -> (f32, f32, f32) {
    let f = f64::from;
    let z = f(m0) * f(q) + f(m3) * f(p) + f(t2) * f(C_ZERO);
    let y = f(m1) * f(q) + f(m4) * f(p) + f(t1) * f(C_ZERO);
    let x = f(m2) * f(q) + f(m5) * f(p) + f(t0) * f(C_ZERO);
    (
        f32::from_bits(f32_bits(x)),
        f32::from_bits(f32_bits(y)),
        f32::from_bits(f32_bits(z)),
    )
}

/// `0x6d7d8d` to `0x6d7e0f`: `1/|a_hi − a_lo|` and `1/|b_hi − b_lo|` (`fdivr` of 1.0), each stored
/// as `f32`. Returns `(inv_b, inv_a)`.
pub fn shadow_extent_inv(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> (f32, f32) {
    let f = f64::from;
    let inv_a = f(C_ONE) / (f(a_hi) - f(a_lo)).abs();
    let inv_b = f(C_ONE) / (f(b_hi) - f(b_lo)).abs();
    (
        f32::from_bits(f32_bits(inv_b)),
        f32::from_bits(f32_bits(inv_a)),
    )
}

/// `0x6d7d23` to `0x6d7d47`: with the shadow direction's sine `[0xcea610]` and cosine `[0xcea60c]`,
/// `pivot − sin·s` and `cos·s + pivot`, each stored as `f32`.
pub fn shadow_axis_rotate(sin: f32, cos: f32, s: f32, pivot: f32) -> (f32, f32) {
    let f = f64::from;
    let out0 = f(pivot) - f(sin) * f(s);
    let out1 = f(cos) * f(s) + f(pivot);
    (
        f32::from_bits(f32_bits(out0)),
        f32::from_bits(f32_bits(out1)),
    )
}

/// `0x6d7a31` to `0x6d7a68`: the length `√((x² + y²) + z²)` stored as `f32`, and that times 1.0.
pub fn shadow_vec3_len_scale(x: f32, y: f32, z: f32) -> (f32, f32) {
    let xf = f64::from(x);
    let yf = f64::from(y);
    let zf = f64::from(z);
    let sumsq = xf * xf + yf * yf + zf * zf;
    let len = sumsq.sqrt();
    let len_f32 = f32::from_bits(f32_bits(len));
    let scaled = f64::from(C_ONE) * f64::from(len_f32);
    (len_f32, f32::from_bits(f32_bits(scaled)))
}

/// `[0x807a40]`, 0.33333334: the multiplier of the leaf `0x6d6e50`, which the call passes 6.0 and
/// gets 2.0.
const C_THIRD: f32 = f32::from_bits(0x3eaa_aaab);
const C_SIX: f32 = f32::from_bits(0x40c0_0000);

/// `0x6d7a7a` to `0x6d7abf`: `(hi − lo)/(6·0.33333334)`, stored as `f32`.
pub fn shadow_half_diff(hi: f32, lo: f32) -> f32 {
    let f = f64::from;
    let span = f(hi) - f(lo);
    let divisor = f(C_SIX) * f(C_THIRD);
    f32::from_bits(f32_bits(span / divisor))
}

/// `0x6d7b01` to `0x6d7b1a`: `1/len` stored as `f32` in three slots.
pub fn shadow_inv_len(len: f32) -> f32 {
    f32::from_bits(f32_bits(f64::from(C_ONE) / f64::from(len)))
}

/// `0x6d7cb7` to `0x6d7d23`: the four sums `ox + a0`, `oy + a1`, `ox + a2` and `oy + a3`, each
/// stored as `f32`.
pub fn shadow_offset_bias(
    ox: f32,
    oy: f32,
    a0: f32,
    a1: f32,
    a2: f32,
    a3: f32,
) -> (f32, f32, f32, f32) {
    let f = f64::from;
    let r0 = f(ox) + f(a0);
    let r1 = f(oy) + f(a1);
    let r2 = f(ox) + f(a2);
    let r3 = f(oy) + f(a3);
    (
        f32::from_bits(f32_bits(r0)),
        f32::from_bits(f32_bits(r1)),
        f32::from_bits(f32_bits(r2)),
        f32::from_bits(f32_bits(r3)),
    )
}

/// `0x6d7d47` to `0x6d7d88`: `a3 − a1` and `a2 − a0`, each stored as `f32`.
pub fn shadow_span_diff(a3: f32, a1: f32, a2: f32, a0: f32) -> (f32, f32) {
    let f = f64::from;
    let d0 = f(a3) - f(a1);
    let d1 = f(a2) - f(a0);
    (f32::from_bits(f32_bits(d0)), f32::from_bits(f32_bits(d1)))
}

/// `0x6d7fd4` to `0x6d8027`: the getter `0x710ca0` leaves `x = [this+0x180]` on the stack; below 0
/// it is 0 (`fcom`, `test ah,5; jp`), above 1 it is 1 (`test ah,0x41; jne`), and NaN is kept; the
/// result is the low dword of `__ftol(x·255 + 0.5)`.
#[allow(clippy::manual_clamp)]
pub fn shadow_color_quantize(x: f32) -> i32 {
    let clamped: f32 = if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    };
    let v = f64::from(clamped) * f64::from(f32::from_bits(0x437f_0000))
        + f64::from(f32::from_bits(0x3f00_0000));
    ftol(v)
}
