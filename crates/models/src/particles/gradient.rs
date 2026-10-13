//! The gradients that colour and size a particle over its life
//! (`systems/models/particles.md#gradients`): `ParticleGradientSample` (`0x7b9b10`),
//! `ParticleGradientSetLifespan` (`0x7b9cf0`), `ParticleGradientSetHeadCells` (`0x7b9da0`),
//! `ParticleGradientSetTailCells` (`0x7b9de0`) and `ParticleGradientSetScales` (`0x7b9e20`).

/// 1.0 (`0x7ff9d8`).
const ONE: f32 = 1.0;
/// 512.0 (`0x8029cc`): added before a byte is read from bits 14 to 21 of the sum.
const ADD_BIAS: f32 = f32::from_bits(0x4400_0000);
/// 0.99 (`0x808aac`, `0x3f7d70a4`).
const GRAD_SLOPE: f32 = f32::from_bits(0x3f7d_70a4);
/// 0.005 (`0x807a3c`, `0x3ba3d70a`).
const GRAD_BIAS: f32 = f32::from_bits(0x3ba3_d70a);
/// 0.001 (`0x801360`, `0x3a83126f`): the least lifespan, and the value a shorter one becomes.
pub const LIFESPAN_MIN: f32 = f32::from_bits(0x3a83_126f);

/// A gradient record, the `this` of `0x7b9b10`.
#[derive(Clone, Copy, Debug, Default)]
pub struct GradientRecord {
    /// `+0x00` to `+0x03`: the start colour bytes.
    pub start_bytes: [u8; 4],
    /// `+0x04`, `+0x08`, `+0x0c`, `+0x10`: the byte deltas of channels 3, 2, 1 and 0.
    pub color_delta: [i32; 4],
    /// `+0x14`, `+0x18`: the head cells' base and count (negative for a falling range).
    pub p5: [i32; 2],
    /// `+0x1c`, `+0x20`: the tail cells' base and count.
    pub p6: [i32; 2],
    /// `+0x24`, `+0x28`: the size and its delta.
    pub scale_base: f32,
    pub scale_delta: f32,
    /// `+0x2c`: the span's start time; `+0x30`: the reciprocal of its length.
    pub age_origin: f32,
    pub age_slope: f32,
    /// `+0x50`: the cell repeat as float bits; 1.0 takes no `fmod`.
    pub curve: u32,
}

/// What `0x7b9b10` writes: the colour bytes, the head and tail cells, and the size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientSample {
    pub color: [u8; 4],
    pub p5: u32,
    pub p6: u32,
    pub scale: f32,
}

/// `f = (age − t0)·(1/(t1 − t0))·0.99 + 0.005`, stored as `f32` before every use.
fn gradient_age(rec: &GradientRecord, age: f32) -> f32 {
    let v = (f64::from(age) - f64::from(rec.age_origin)) * f64::from(rec.age_slope);
    ((v * f64::from(GRAD_SLOPE)) + f64::from(GRAD_BIAS)) as f32
}

/// One channel: `fild delta; fmul coef; fiadd start`, times `scale` for alpha, `+ 512.0` stored as
/// `f32`, and bits 14 up of the stored word. `coef` is the stored `f` on the plain path and the
/// `fmod` result left in the register on the repeat path.
fn quant_channel(delta: i32, coef: f64, start: i32, scale: Option<f32>) -> u32 {
    let mut sum = f64::from(delta) * coef + f64::from(start);
    if let Some(s) = scale {
        sum *= f64::from(s);
    }
    ((sum + f64::from(ADD_BIAS)) as f32).to_bits() >> 0xe
}

/// `0x7b9b10` (ecx = the record; age, the alpha scale, then four out pointers; `ret 0x18`): the
/// colour, the alpha times `p3_scale`, the size `f·delta + base`, and the two cells from `f`, or
/// from `fmod(f·repeat, 1.0)` (`__CIfmod`, `0x73f90a`) when the repeat is not 1.0.
pub fn particle_gradient_sample(rec: &GradientRecord, age: f32, p3_scale: f32) -> GradientSample {
    let a = gradient_age(rec, age);
    let af = f64::from(a);
    let c3 = quant_channel(
        rec.color_delta[0],
        af,
        i32::from(rec.start_bytes[3]),
        Some(p3_scale),
    ) as u8;
    let c2 = quant_channel(rec.color_delta[1], af, i32::from(rec.start_bytes[2]), None) as u8;
    let c1 = quant_channel(rec.color_delta[2], af, i32::from(rec.start_bytes[1]), None) as u8;
    let c0 = quant_channel(rec.color_delta[3], af, i32::from(rec.start_bytes[0]), None) as u8;
    let scale = (af * f64::from(rec.scale_delta) + f64::from(rec.scale_base)) as f32;
    let coef = if rec.curve == 0x3f80_0000 {
        af
    } else {
        (af * f64::from(f32::from_bits(rec.curve))) % 1.0
    };
    let p5 = quant_channel(rec.p5[1], coef, rec.p5[0], None) & 0xff;
    let p6 = quant_channel(rec.p6[1], coef, rec.p6[0], None) & 0xff;
    GradientSample {
        color: [c0, c1, c2, c3],
        p5,
        p6,
        scale,
    }
}

/// A cell range as `0x7b9da0` and `0x7b9de0` store it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellRampRecord {
    /// `+0x3c`, `+0x40` (tail `+0x44`, `+0x48`): the range as given.
    pub raw_lo: i32,
    pub raw_hi: i32,
    /// `+0x14`, `+0x18` (tail `+0x1c`, `+0x20`): the base and count the sample reads.
    pub base: i32,
    pub span: i32,
}

/// `0x7b9da0` and `0x7b9de0` (ecx = the record; first and last cell; `ret 8`): a range `p` to `q`
/// stores `p` and `q − p + 1` for `q ≥ p` (signed `cmp; jl`), else `p + 1` and `q − p − 1`.
pub fn cell_ramp_record(start: i32, end: i32) -> CellRampRecord {
    let (base, span) = if end < start {
        (
            start.wrapping_add(1),
            end.wrapping_sub(start).wrapping_sub(1),
        )
    } else {
        (start, end.wrapping_sub(start).wrapping_add(1))
    };
    CellRampRecord {
        raw_lo: start,
        raw_hi: end,
        base,
        span,
    }
}

/// What `0x7b9cf0` stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientSpans {
    /// `+0x2c`: `L·m0`.
    pub at_2c: f32,
    /// `+0x34`: `L·m1`.
    pub at_34: f32,
    /// `+0x30`: `1/(L·m1 − L·m0)` from the products in the register.
    pub at_30: f32,
    /// `+0x5c`: `L`.
    pub at_5c: f32,
}

/// `0x7b9cf0` (ecx = the record; the lifespan; `ret 4`): a lifespan under 0.001 (`test ah,5; jp`,
/// so NaN stays) becomes `0x3a83126f`; then the span bounds from `m0 = +0x54` and `m1 = +0x58`.
pub fn gradient_record(span: f32, m54: f32, m58: f32) -> GradientSpans {
    let span = if span < LIFESPAN_MIN {
        LIFESPAN_MIN
    } else {
        span
    };
    let a = f64::from(span) * f64::from(m54);
    let b = f64::from(span) * f64::from(m58);
    GradientSpans {
        at_2c: a as f32,
        at_34: b as f32,
        at_30: (f64::from(ONE) / (b - a)) as f32,
        at_5c: span,
    }
}

/// What `0x7b9e20` stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientRange {
    /// `+0x24`, `+0x4c`: the two sizes as given.
    pub at_24: f32,
    pub at_4c: f32,
    /// `+0x28`: `hi − lo`, stored as `f32`.
    pub at_28: f32,
}

/// `0x7b9e20` (ecx = the record; the two sizes; `ret 8`).
pub fn gradient_range(lo: f32, hi: f32) -> GradientRange {
    GradientRange {
        at_24: lo,
        at_4c: hi,
        at_28: (f64::from(hi) - f64::from(lo)) as f32,
    }
}
