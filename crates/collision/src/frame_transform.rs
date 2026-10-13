//! Stretches of `CMovement::TransformFrame` (`0x7c6620`,
//! `systems/collision/falling.md#frame-transform`): the two facings it turns, the vertical affine,
//! the matrix product and the 2D normalization.

/// `[0xcf5d00]`: 2π.
pub const TWO_PI: f32 = f32::from_bits(0x40c9_0fdb);

/// `[0x7ffd74]`: 0.0.
pub const ZERO: f32 = f32::from_bits(0x0000_0000);

/// `[0x8029d4]`: 2^-22.
pub const NRM_EPS: f32 = f32::from_bits(0x3480_0000);

/// `[0x7ff9d8]`: 1.0.
pub const ONE: f32 = f32::from_bits(0x3f80_0000);

/// One step into `[0, 2π]`: less 2π above 2π (`test ah,0x41; jne`), plus 2π below 0 (`test ah,5;
/// jp`), stored as `f32`; NaN passes.
#[inline]
fn wrap_two_pi(v: f64) -> f32 {
    let two_pi = f64::from(TWO_PI);

    if v > two_pi {
        return (v - two_pi) as f32;
    }

    if v < f64::from(ZERO) {
        (v + two_pi) as f32
    } else {
        v as f32
    }
}

/// `0x7c6645`: `wrap(arg + +0x1c)`.
pub fn facing_advance_a(arg: f32, facing: f32) -> f32 {
    wrap_two_pi(f64::from(arg) + f64::from(facing))
}

/// `0x7c6676`: `wrap(arg + +0x50)`.
pub fn facing_advance_b(arg: f32, facing: f32) -> f32 {
    wrap_two_pi(f64::from(arg) + f64::from(facing))
}

/// `0x7c66a7`: `src·scale + offset` in the register, stored as `f32`; once with `+0x7c` and once
/// with `+0x80`.
pub fn transport_delta_affine(src: f32, scale: f32, offset: f32) -> f32 {
    (f64::from(src) * f64::from(scale) + f64::from(offset)) as f32
}

/// `0x7c66ca`: the matrix at `[edi]` (rows of four) times `v`, each row summed in the order below
/// and stored as `f32`.
pub fn matvec3(m: [f32; 16], v: [f32; 3]) -> [f32; 3] {
    let vx = f64::from(v[0]);
    let vy = f64::from(v[1]);
    let vz = f64::from(v[2]);
    let mm = |i: usize| -> f64 { f64::from(m[i]) };

    let out0 = ((mm(8) * vz) + (mm(4) * vy)) + (vx * mm(0));

    let out1 = ((mm(1) * vx) + (mm(9) * vz)) + (mm(5) * vy);

    let out2 = ((mm(2) * vx) + (mm(6) * vy)) + (vz * mm(10));
    [out0 as f32, out1 as f32, out2 as f32]
}

/// `0x7c672d`: the normalization runs unless `|len| < 2^-22`; NaN runs it.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn normalize2_fires(len: f64) -> bool {
    !(len.abs() < f64::from(NRM_EPS))
}

/// `√(x² + y²)` of `+0x68` and `+0x6c`, unrounded.
pub fn normalize2_len(x: f32, y: f32) -> f64 {
    let xx = f64::from(x) * f64::from(x);
    let yy = f64::from(y) * f64::from(y);
    (xx + yy).sqrt()
}

/// `0x7c672d`: `(x, y)` times `1/len` from the register, each stored as `f32`, or unchanged.
pub fn normalize2(x: f32, y: f32) -> [f32; 2] {
    let len = normalize2_len(x, y);
    if !normalize2_fires(len) {
        return [x, y];
    }

    let inv = f64::from(ONE) / len;
    [(inv * f64::from(x)) as f32, (inv * f64::from(y)) as f32]
}
