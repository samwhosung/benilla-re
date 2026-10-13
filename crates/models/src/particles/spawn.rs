//! Spawning and substeps (`systems/models/particles.md#spawn-count`,
//! `systems/models/particles.md#substeps`, `systems/models/particles.md#emitter-classes`,
//! `systems/models/particles.md#model-particles`): `CParticleEmitter2::Spawn` (`0x7b5550`),
//! `CParticleEmitter2::InitParticle` (`0x7b22b0`), `CParticleEmitter2::Advance` (`0x7b5880`),
//! `CParticleEmitter2::InitModelParticle` (`0x7b2420`) and `CParticleEmitter2::Preallocate`
//! (`0x7b1dd0`).

use crate::x87::ftol;

/// 50.0 (`0x80fed4`).
pub const DIST_OFFSET: f32 = 50.0;
/// 0.02 (`0x7ffdac`, `0x3ca3d70a`).
pub const DIST_SCALE: f32 = f32::from_bits(0x3ca3_d70a);
/// 1.0 (`0x7ff9d8`).
pub const ONE: f32 = 1.0;
/// 0.0 (`0x7ffd74`).
pub const ZERO: f32 = 0.0;
/// 0.25 (`0x8029b0`): the least distance factor.
pub const CLAMP_LO: f32 = 0.25;
/// The density setting `0x7ae000` returns (`[0x87d5fc]`, 1.0 in the image).
pub const VEL_GLOBAL: f32 = 1.0;
/// 0.5 (`0x7ffa24`).
pub const HALF: f32 = 0.5;
/// 1e-8 (`0x81d86c`, `0x322bcc77`).
pub const FLUSH_EPS: f32 = f32::from_bits(0x322b_cc77);
/// 0.1 (`0x81d828`, `0x3dcccccd`): the substep.
pub const PERIOD: f32 = f32::from_bits(0x3dcc_cccd);
/// 255.0 (`0x7ffe58`): the most substeps.
pub const COUNT_MAX: f32 = 255.0;
/// 512.0 (`0x8029cc`).
pub const BIAS_512: f32 = 512.0;
/// 2.3841858e-7 (`0x8029d4`): the unit-length tolerance of the model particle normalizes.
pub const NORM_EPS: f32 = f32::from_bits(0x3480_0000);

/// The distance factor's clamp at `0x7b5576`: under 0.25 (`test ah,5; jp`) it is 0.25; otherwise,
/// reloaded, it is 1 unless under 1 (`test ah,5; jnp`), so 1.0 and NaN become 1.
pub fn clamp_t(t: f32) -> f32 {
    if t < CLAMP_LO {
        CLAMP_LO
    } else if t < ONE {
        t
    } else {
        ONE
    }
}

/// `0x7b5556..0x7b55be`: `t = 1 − (d − 50)·0.02` from the distance `[0xcf58e8]`, stored as `f32`,
/// clamped, then `density·rate·t` (`+0xa8` the rate) stored as `f32`.
pub fn t_scalar(g0: f32, field_a8: f32) -> f32 {
    let t =
        (f64::from(ONE) - (f64::from(g0) - f64::from(DIST_OFFSET)) * f64::from(DIST_SCALE)) as f32;
    let t = clamp_t(t);
    (f64::from(VEL_GLOBAL) * f64::from(field_a8) * f64::from(t)) as f32
}

/// `0x7b565d..0x7b566d`: the accumulator `rate·dt + acc` as `fst` stores it at `+0x8`.
pub fn pos_integrate(vel: f32, dt: f32, pos: f32) -> f32 {
    (f64::from(vel) * f64::from(dt) + f64::from(pos)) as f32
}

/// The spawn count: `__ftol` of the unrounded `rate·dt + acc` plus 0.5 (`0x7b56b0`, `0x7b57d0`).
pub fn integrate_count(vel: f32, dt: f32, pos: f32) -> i32 {
    ftol(f64::from(vel) * f64::from(dt) + f64::from(pos) + f64::from(HALF))
}

/// `0x7b5689..0x7b56b0`, flag 0x1000: the spawn point less the last position `+0x248`, each
/// stored as `f32`.
pub fn rel_delta(src: [f32; 3], origin: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| (f64::from(src[k]) - f64::from(origin[k])) as f32)
}

/// `0x7b56f3..0x7b5730`: `d·(r − 1) + last` with `r` the draw in `[1, 2)` and `r − 1` kept in the
/// register.
pub fn weighted_point(r: f32, d: [f32; 3], origin: [f32; 3]) -> [f32; 3] {
    let w = f64::from(r) - f64::from(ONE);
    [0, 1, 2].map(|k| (f64::from(d[k]) * w + f64::from(origin[k])) as f32)
}

/// `0x7b586e..0x7b5877`: `acc − count` with the count read as an unsigned `fild qword`.
pub fn count_subtract(pos: f32, count: i32) -> f32 {
    (f64::from(pos) - f64::from(count as u32)) as f32
}

/// `0x7b22d5..0x7b2301`: the age `(r − 1)·dt` from the draw `r`, kept unless 0 compares above it
/// (`test ah,0x41; jne`), so a NaN and -0 stay and a negative, however small, becomes +0. The
/// test is on the bits: an optimizer reads `x < 0 ? 0 : x` as `x <= 0 ? 0 : x`, which turns -0
/// into +0.
pub fn spawn_random_scale(rnd_unit: f32, scale: f32) -> f32 {
    let product = (f64::from(rnd_unit) - f64::from(ONE)) * f64::from(scale);
    let magnitude = product.to_bits() & !(1 << 63);
    let below_zero =
        product.is_sign_negative() && magnitude != 0 && magnitude <= f64::INFINITY.to_bits();
    if below_zero {
        ZERO
    } else {
        product as f32
    }
}

/// `0x7b234a..0x7b238c`: the velocity `speed·dir + burst`, z's product stored as `f32` first.
pub fn spawn_transform(scalar: f32, src: [f32; 3], base: [f32; 3]) -> [f32; 3] {
    let s = f64::from(scalar);
    let out0 = (s * f64::from(src[0]) + f64::from(base[0])) as f32;
    let out1 = (s * f64::from(src[1]) + f64::from(base[1])) as f32;
    let prod2 = (s * f64::from(src[2])) as f32;
    let out2 = (f64::from(prod2) + f64::from(base[2])) as f32;
    [out0, out1, out2]
}

/// `0x7b2395..0x7b240f`: the flush of `systems/models/particles.md#particle-motion` on each
/// velocity component.
pub fn spawn_flush_gate(v: f32) -> f32 {
    if v != ZERO && v.abs() < FLUSH_EPS {
        0.0
    } else {
        v
    }
}

/// `0x7b58fe` and `0x7b5924`: `x/0.1`, passed to `floor` as a double.
pub fn substep_floor_arg(x: f32) -> f64 {
    f64::from(x) / f64::from(PERIOD)
}

/// `0x7b590f..0x7b591e`: `dt − 0.1·n` with `n` the stored floor.
pub fn substep_remainder(param2: f32, floor_count: f32) -> f32 {
    (f64::from(param2) - f64::from(PERIOD) * f64::from(floor_count)) as f32
}

/// `0x7b595f..0x7b5974`: `n + 512.0` stored as `f32`, whose bits 14 to 21 are the count.
pub fn substep_count_bias(param3: f32) -> f32 {
    (f64::from(param3) + f64::from(BIAS_512)) as f32
}

/// `0x7b598a..0x7b59ba`: `1/(n + 1)` kept in the register times the displacement `+0x26c`.
pub fn substep_tangent(n_plus_1: i64, axis: [f32; 3]) -> [f32; 3] {
    let frac = f64::from(ONE) / (n_plus_1 as f64);
    axis.map(|a| (frac * f64::from(a)) as f32)
}

/// The test both model particle normalizes skip on: `|len² − 1| < 2.3841858e-7` (`test ah,5;
/// jnp`), so a NaN length normalizes.
pub fn norm_skip(len2: f64) -> bool {
    (f64::from(ONE) - len2).abs() < f64::from(NORM_EPS)
}

/// `0x7b243e..0x7b24ca`: the spawn matrix's three rows scaled by `1/|row 0|`, with `len² = (z² +
/// y²) + x²` of row 0 and the reciprocal stored as `f32` for the scale (`0x672d80`).
pub fn normalize_basis9(s: [f32; 9]) -> [f32; 9] {
    let len2 = {
        let [x, y, z] = [s[0], s[1], s[2]].map(f64::from);
        (z * z + y * y) + x * x
    };
    if norm_skip(len2) {
        return s;
    }
    let recip = (f64::from(ONE) / len2.sqrt()) as f32;
    s.map(|c| (f64::from(c) * f64::from(recip)) as f32)
}

/// `0x7b24d8..0x7b2540`: the quaternion normalize, `len² = ((x² + y²) + z²) + w²` and the
/// reciprocal kept in the register.
pub fn normalize_quat4(q: [f32; 4]) -> [f32; 4] {
    let [x, y, z, w] = q.map(f64::from);
    let len2 = ((x * x + y * y) + z * z) + w * w;
    if norm_skip(len2) {
        return q;
    }
    let recip = f64::from(ONE) / len2.sqrt();
    q.map(|c| (recip * f64::from(c)) as f32)
}

/// `0x7b257f..0x7b25dd`: the spin `(r2 − 1)·range + min`, `((r1 − 1) + 1)·+0x1d4` and `((r0 − 1) +
/// 1)·+0x1dc` from three draws in `[1, 2)`.
pub fn spawn_random_axes(r: [f32; 3], min: f32, range: f32, d4: f32, dc: f32) -> [f32; 3] {
    let one = f64::from(ONE);
    let axis0 = ((f64::from(r[2]) - one) * f64::from(range) + f64::from(min)) as f32;
    let axis1 = (((f64::from(r[1]) - one) + one) * f64::from(d4)) as f32;
    let axis2 = (((f64::from(r[0]) - one) + one) * f64::from(dc)) as f32;
    [axis0, axis1, axis2]
}

/// `0x7b2624..0x7b2648`, flag 0x8000: each spin component times ±1.0 by bit 0 of a draw.
pub fn spawn_sign_flip(axes: [f32; 3], signs: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|k| (f64::from(signs[k]) * f64::from(axes[k])) as f32)
}

/// `0x7b1dd0`: `+0xac · +0xa8 · 1.15` (`0x81d868`) in the register, the emitter's particle count
/// before `__ftol`.
pub fn prealloc_product_f64(scale_ac: f32, scale_a8: f32, k: f32) -> f64 {
    f64::from(scale_ac) * f64::from(scale_a8) * f64::from(k)
}

/// `0x7b1dd0`'s count: `__ftol` of [`prealloc_product_f64`].
pub fn prealloc_count(scale_ac: f32, scale_a8: f32, k: f32) -> i32 {
    ftol(prealloc_product_f64(scale_ac, scale_a8, k))
}
