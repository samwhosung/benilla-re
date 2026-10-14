//! The sky band functions: `UpdateDisc` (`0x6d1960`, `systems/lighting/celestial.md#disc-draw`),
//! `BuildSkyDome` (`0x6d0d10`, `systems/lighting/sky.md#sky-dome`) and the arithmetic of
//! `ColourSkyDome` (`0x6d0f50`, `sky.md#sky-colours`), whose body is run as slices around the
//! leaves it calls (`LerpChannel` `0x6d15f0`, `EvalSkyTrack` `0x6cf6c0`, and the colour packers).
//! The x87 runs at 53 bits and stores `f32` at each `fstp dword`; a value kept after an `fst dword`
//! stays wide, and `fsin` and `fcos` are the host's.

use crate::x87::fistp;

/// `[0x7ffd74]`, 0.0.
const C_ZERO: f32 = f32::from_bits(0x0000_0000);
/// `[0x7ff9d8]`, 1.0.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x7ffe58]`, 255.0.
const C_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]`, 512.0.
const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x80c5c0]`, 0.4 (the second clip plane depth).
const C_PLANE: f32 = f32::from_bits(0x3ecc_cccd);
/// `[0x801360]`, 0.001 (clip / ramp epsilon).
const C_EPS: f32 = f32::from_bits(0x3a83_126f);
/// `[0x803088]`, 2.5 (the depth→alpha ramp slope).
const C_RAMP: f32 = f32::from_bits(0x4020_0000);

/// The byte `((v·255 + 512) stored as f32) >> 14`, low byte (`fmul 255; fadd 512; fstp; shr 0xe`),
/// with the product and sum at 53 bits.
fn float_to_ramp_byte(v: f64) -> u8 {
    let f = ((v * f64::from(C_255)) + f64::from(C_512)) as f32;
    ((f.to_bits() >> 14) & 0xff) as u8
}

/// The vertex depths of the strip record `UpdateDisc` clips, at `+8 + 0xc·i`. `v[0]` and `v[2]` are
/// the endpoints; `v[2]` to `v[5]` also take the crossings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClipParam1 {
    pub v: [f32; 6],
}

/// The coordinates carried through the clip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClipParam2 {
    /// `+0x4`, read as the base of the second crossing.
    pub p04: f32,
    /// `+0x14`, read for the second crossing and written with the first crossing's parameter.
    pub p14: f32,
    /// `+0x1c`, written with the first crossing's parameter.
    pub p1c: f32,
    /// `+0x24`, written with the second crossing's coordinate.
    pub p24: f32,
    /// `+0x2c`, the same.
    pub p2c: f32,
}

/// What `UpdateDisc` leaves: the two records, the count `*param5` (0, 4 or 6), `*param6` (6 when
/// the second plane is crossed), the edge triple copied from `[0x86f418]` on that crossing, and the
/// ramp bytes, one per produced vertex or none where the vertex is not ramped.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClipResult {
    pub p1: ClipParam1,
    pub p2: ClipParam2,
    pub flag: u32,
    pub flag6: Option<u32>,
    pub edge_idx: Option<[u32; 3]>,
    pub ramp: Vec<Option<u8>>,
}

/// `0x6d1960` (`ecx` = the strip, whose depth at `+8` is read; six stack arguments, `ret 0x18`),
/// clipping a strip against the planes at depth 0 and 0.4 and writing a fog ramp byte per vertex.
/// `base = f32(strip depth − [0xce9b80])`; `d0 = f32(base + v0)` and `d1 = f32(base + v2)`. Both
/// above 0 (`fcomp`, `test ah,0x41; jne`, which also sends 0 and NaN on) leave `*param5 = 4` and no
/// crossing; both below 0 (`test ah,5; jp` and `jnp`) leave 0 and return; anything else is a
/// crossing, `*param5 = 4`, `t1 = f32(d0/(d0 − d1))`, `c = f32((v2 − v0)·t1 + v0)` stored in `v[2]`
/// and `v[3]`, and `t1` in `p14` and `p1c`. The second plane takes `g0 = f32((base + v0) − 0.4)`
/// and `g1 = f32((base + v2) − 0.4)` with the clipped `v2`; when `g0 > 0.001` and `g1 < 0.001` it
/// sets `*param5` and `*param6` to 6, copies the edge triple, and with `t2 = f32(g0/(g0 − g1))`
/// stores `f32((v2 − v0)·t2 + v0)` in `v[4]` and `v[5]` and `f32((p14 − p04)·t2 + p04)` in `p24`
/// and `p2c`. The ramp then runs over `*param5` vertices: `r = (base + v[i]) − 0.4` stays wide, and
/// below 0.001 (NaN skips) the byte is `float_to_ramp_byte` of `(0.4 + r)·2.5` clamped to [0, 1]
/// (`fcom` with 0, `test ah,5; jp`, which clamps only a negative, then `fcom` with 1, `jnp`, which
/// clamps 1 and above and NaN).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn fog_plane_clip(
    strip_depth: f32,
    p1_in: ClipParam1,
    p2_in: ClipParam2,
    fog_depth_global: f32,
    flag6_in: u32,
) -> ClipResult {
    let f = f64::from;
    let mut v = p1_in.v;
    let mut p2 = p2_in;

    let base = (f(strip_depth) - f(fog_depth_global)) as f32;
    let d0 = (f(base) + f(v[0])) as f32;
    let d1 = (f(base) + f(v[2])) as f32;

    let mut flag: u32;
    let both_positive = d0 > 0.0 && d1 > 0.0;
    if both_positive {
        flag = 4;
    } else {
        if d0 < 0.0 && d1 < 0.0 {
            return ClipResult {
                p1: ClipParam1 { v },
                p2,
                flag: 0,
                flag6: None,
                edge_idx: None,
                ramp: Vec::new(),
            };
        }
        flag = 4;
        let t1 = (f(d0) / (f(d0) - f(d1))) as f32;
        let c = ((f(v[2]) - f(v[0])) * f(t1) + f(v[0])) as f32;
        v[2] = c;
        v[3] = c;
        p2.p14 = t1;
        p2.p1c = t1;
    }

    let g0 = ((f(base) + f(v[0])) - f(C_PLANE)) as f32;
    let g1 = ((f(base) + f(v[2])) - f(C_PLANE)) as f32;
    let mut flag6 = None;
    let mut edge_idx = None;
    if g0 > C_EPS && g1 < C_EPS {
        flag = 6;
        flag6 = Some(6);
        edge_idx = Some([0x0001_0000, 0x0005_0004, 0x0003_0002]);
        let t2 = (f(g0) / (f(g0) - f(g1))) as f32;
        let c1 = ((f(v[2]) - f(v[0])) * f(t2) + f(v[0])) as f32;
        let c2 = ((f(p2.p14) - f(p2.p04)) * f(t2) + f(p2.p04)) as f32;
        v[4] = c1;
        v[5] = c1;
        p2.p24 = c2;
        p2.p2c = c2;
    } else {
        let _ = flag6_in;
    }

    let mut ramp = Vec::with_capacity(flag as usize);
    for &pv in v.iter().take(flag as usize) {
        let r = (f(base) + f(pv)) - f(C_PLANE);
        if r < f(C_EPS) {
            let frac = (f(C_PLANE) - -r) * f(C_RAMP);
            let clamped = if frac < f(C_ZERO) {
                f(C_ZERO)
            } else if frac < f(C_ONE) {
                frac
            } else {
                f(C_ONE)
            };
            ramp.push(Some(float_to_ramp_byte(clamped)));
        } else {
            ramp.push(None);
        }
    }

    ClipResult {
        p1: ClipParam1 { v },
        p2,
        flag,
        flag6,
        edge_idx,
        ramp,
    }
}

/// `[0x811614]`, 0.15915494 (1/2π).
const C_INV_2PI: f32 = f32::from_bits(0x3e22_f983);
/// `[0x8029b0]`, 0.25.
const C_QUARTER: f32 = f32::from_bits(0x3e80_0000);
/// `[0x8030a0]`, -1.0.
const C_NEG_ONE: f32 = f32::from_bits(0xbf80_0000);

/// `ColourSkyDome` from `0x6d0fa0` to `0x6d0fcc`: the alpha table index `(1 − g)·255 + 512` at 53
/// bits, stored as `f32`, from `g = [0xce99d0]`. The byte is `sky_alpha_byte` of it.
pub fn sky_alpha_index(g: f32) -> f32 {
    let f = f64::from;
    let a = f(C_ONE) - f(g);
    let b = a * f(C_255);
    (b + f(C_512)) as f32
}

/// The byte `(bits >> 14) & 0xff` of the stored index.
pub fn sky_alpha_byte(stored: f32) -> u8 {
    ((stored.to_bits() >> 14) & 0xff) as u8
}

/// `ColourSkyDome` from `0x6d10fa` to `0x6d110e`: `−1/count` (`fild` of the signed count at
/// `+0x24`, `fdivr` of −1), stored as `f32`.
pub fn sky_row_step(count: i32) -> f32 {
    (f64::from(C_NEG_ONE) / f64::from(count)) as f32
}

/// The row angle and whether the wrap subtracted 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyRowAngle {
    pub angle: f32,
    pub wrapped: bool,
}

/// `ColourSkyDome` from `0x6d1111` to `0x6d113f`: `a = g·(1/2π) + 0.25` at 53 bits from the wind
/// heading `g = [0xce9b90]`, stored as `f32` with `fst`, which leaves the wide `a` on the stack.
/// The compare with 1 (`fcomp`, `test ah,0x41; jne`) is of the wide `a`, so the wrap fires for `a >
/// 1` and not for 1, below it or NaN; it subtracts 1 from the stored `f32`.
pub fn sky_row_angle(g: f32) -> SkyRowAngle {
    let f = f64::from;
    let a_pc53 = f(g) * f(C_INV_2PI) + f(C_QUARTER);
    let a_f32 = a_pc53 as f32;
    if a_pc53 > f64::from(C_ONE) {
        SkyRowAngle {
            angle: (f(a_f32) - f(C_ONE)) as f32,
            wrapped: true,
        }
    } else {
        SkyRowAngle {
            angle: a_f32,
            wrapped: false,
        }
    }
}

/// `ColourSkyDome` from `0x6d1150` to `0x6d116c`: when the `f32` angle is below 0 (`test ah,5; jp`
/// skips 0 and NaN), `f32(a + 1)`.
pub fn sky_angle_wrap(a: f32) -> f32 {
    if a < 0.0 {
        (f64::from(a) + f64::from(C_ONE)) as f32
    } else {
        a
    }
}

/// `ColourSkyDome` from `0x6d1484` to `0x6d1497`: `f32(a + step)`.
pub fn sky_angle_accum(a: f32, step: f32) -> f32 {
    (f64::from(a) + f64::from(step)) as f32
}

/// `[0x811540]`, 0.25 (the reference angle fraction).
const C_BAND_FRAC: f32 = f32::from_bits(0x3e80_0000);
/// `[0x811508]`, π.
const C_PI: f32 = f32::from_bits(0x4049_0fdb);
/// `[0x8115bc]`, 0.041666668 (the j refinement step).
const C_J_STEP: f32 = f32::from_bits(0x3d2a_aaab);
/// `[0x8030b0]`, π (the |φ-π| convergence target).
const C_PI_30B0: f32 = f32::from_bits(0x4049_0fdb);
/// `[0x8026bc]`, 9.536743e-7 (convergence epsilon).
const C_CONV_EPS: f32 = f32::from_bits(0x3580_0000);

/// The seven band fractions at `[0x81152c]` to `[0x811544]`: 0, 0.17, 0.2, 0.23, 0.24, 0.25 and 1.
pub const BAND_ANGLES: [f32; 7] = [
    f32::from_bits(0x0000_0000),
    f32::from_bits(0x3e2e_147b),
    f32::from_bits(0x3e4c_cccd),
    f32::from_bits(0x3e6b_851f),
    f32::from_bits(0x3e75_c28f),
    f32::from_bits(0x3e80_0000),
    f32::from_bits(0x3f80_0000),
];

/// The signed divide by 12 as the exe makes it: the high dword of `x·0x2aaaaaab`, `sar 1`, plus the
/// sign bit.
fn imul_div6(x: i32) -> i32 {
    let prod = (i64::from(x) * 0x2aaa_aaab_i64) as u64;
    let mut edx = (prod >> 32) as i32;
    edx >>= 1;
    let sign = ((edx as u32) >> 31) as i32;
    edx.wrapping_add(sign)
}

/// What `BuildSkyDome` writes: the band records, the ring of coordinate pairs, and the two words at
/// `+0x28` and `+0x2a`.
#[derive(Clone, Debug, PartialEq)]
pub struct SkyBandSetup {
    /// Records of three floats at `[this+8]`: a sine term, a cosine term and `cos(θ)·arg + K`.
    pub bands: Vec<[f32; 3]>,
    /// Pairs of `u16` at `[this+0x20]`: six rows of 25, the first band having none.
    pub coords: Vec<(u16, u16)>,
    /// `[this+0x28]`: the record count.
    pub count_word: u16,
    /// `[this+0x2a]`: 0x12c.
    pub tail_word: u16,
}

/// `0x6d0d10(arg)` (`ecx` = the object, `ret 4`) with its three vectors already sized, so the grow
/// calls are skipped. With `K = −cos(0.25·π)` kept wide, each band fraction `A` gives `θ = π·A`
/// wide, and `sin θ` stored as `f32`; the records are written for `j = 0, 1, …` with `φ =
/// 2·(j·0.041666668·π)`, `[sin φ·sin θ·arg, cos φ·sin θ·arg, f32(cos θ·arg + K)]`, ending after the
/// record for which `|θ| < ε` (the `f32` of `|θ|`) or `|θ − π| < ε` (wide; `test ah,5; jnp` exits
/// only below ε), and at 24. For bands after the first, 25 pairs: `x = (i mod 24) + previous block
/// start` (0 for the mod when the previous `θ`, stored as `f32`, is within ε of 0) and `y = (i mod
/// 24) + this block start` (0 when `θ` is within ε of π). The block start is `(cursor − base)/12`.
/// The count is the final such index.
pub fn sky_band_orient_setup(arg: f32, base_off: u32) -> SkyBandSetup {
    let f = f64::from;
    let argf = f(arg);

    let k = -(f(C_BAND_FRAC) * f(C_PI)).cos();

    let mut ebx: u32 = base_off;
    let mut bands: Vec<[f32; 3]> = Vec::new();
    let mut coords: Vec<(u16, u16)> = Vec::new();
    let mut prev_block_idx: i32 = 0;
    let mut prev_theta: f32 = 0.0;

    for (idx, &a) in BAND_ANGLES.iter().enumerate() {
        let theta = f(C_PI) * f(a);
        let block_idx = imul_div6((ebx.wrapping_sub(base_off)) as i32);

        let cos_t = theta.cos();
        let sin_t_f32 = theta.sin() as f32;
        let third = (cos_t * argf + k) as f32;
        let abs_theta = (theta.abs()) as f32;

        let mut j: i32 = 0;
        loop {
            let phi0 = f64::from(j) * f(C_J_STEP) * f(C_PI);
            let phi = phi0 + phi0;
            let band_sin = (phi.sin() * f(sin_t_f32) * argf) as f32;
            let band_cos = (phi.cos() * f(sin_t_f32) * argf) as f32;
            bands.push([band_sin, band_cos, third]);
            ebx = ebx.wrapping_add(0xc);

            if f(abs_theta) < f(C_CONV_EPS) {
                break;
            }
            if (theta - f(C_PI_30B0)).abs() < f(C_CONV_EPS) {
                break;
            }
            j += 1;
            if j >= 0x18 {
                break;
            }
        }

        if idx > 0 {
            let prev_abs = prev_theta.abs();
            let this_minus_pi = (theta - f(C_PI_30B0)).abs();
            let x_zero = (f(prev_abs)) < f(C_CONV_EPS);
            let y_zero = this_minus_pi < f(C_CONV_EPS);
            for i in 0..0x19_i32 {
                let xi = if x_zero { 0 } else { i % 0x18 };
                let yi = if y_zero { 0 } else { i % 0x18 };
                let x = (xi.wrapping_add(prev_block_idx)) as u16;
                let y = (yi.wrapping_add(block_idx)) as u16;
                coords.push((x, y));
            }
        }

        prev_block_idx = block_idx;
        prev_theta = theta as f32;
    }

    let count_word = imul_div6((ebx.wrapping_sub(base_off)) as i32) as u16;

    SkyBandSetup {
        bands,
        coords,
        count_word,
        tail_word: 0x12c,
    }
}

/// `[0x86f2c8]`, 0.5 (the round-bias before fistp).
const C_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x7ffd7c]`, 0.7 (the θ<0 sun-glow factor).
const C_07: f32 = f32::from_bits(0x3f33_3333);

/// `ColourSkyDome` from `0x6d0f6b` to `0x6d0f90`: `f32(t0·k)`, `t0` the wide return of
/// `EvalSkyTrack` on the track at `0xce9b2c` and `k = [0xce9c5c]`.
pub fn sky_scale_init(t0: f64, k: f32) -> f32 {
    (t0 * f64::from(k)) as f32
}

/// The channel byte `fistp(lerp − 0.5)` the 13 `fld; fsub 0.5; fistp` sites of `ColourSkyDome`
/// share (`0x6d1020`, `0x6d1059`, `0x6d1092`, `0x6d11d1`, `0x6d1208`, `0x6d123f`, `0x6d12b1`,
/// `0x6d1303`, `0x6d133a`, `0x6d1371`, `0x6d13c7`, `0x6d13fb`, `0x6d142f`) on the `f32` the lerp
/// leaf stored: round to nearest, ties to even, so an exact odd integer comes out one low; NaN and
/// values beyond `i32` give `0x80000000`.
pub fn lerp_result_to_byte(lerp_f32: f32) -> i32 {
    let biased = f64::from(lerp_f32) - f64::from(C_HALF);
    fistp(biased)
}

/// `ColourSkyDome` from `0x6d116c` to `0x6d11aa`, reached when `fcom` of the track value `T` with 0
/// (`test ah,1; jne`) does not branch, so for `T` not below 0 and not NaN: `f32((1 − T)·scale)`,
/// with the wide `T` and the `f32` scale.
pub fn sky_alpha_one_minus(t: f64, scale: f32) -> f32 {
    let one_minus = f64::from(C_ONE) - t;
    (one_minus * f64::from(scale)) as f32
}

/// The three values of the `T < 0` path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyAlphaNeg {
    /// `f32((−T)·scale)`.
    pub neg_weight: f32,
    /// `fistp(f32(scale·255) − 0.5)`.
    pub alpha255_idx: i32,
    /// `f32(scale·0.7)`.
    pub glow07: f32,
}

/// `ColourSkyDome` from `0x6d1293` to `0x6d12cf`, reached for `T` below 0 or NaN (`fcom`, `test
/// ah,1; jne`): `f32((−T)·scale)`, `fistp(f32(scale·255) − 0.5)` and `f32(scale·0.7)`.
pub fn sky_alpha_neg(t: f64, scale: f32) -> SkyAlphaNeg {
    let f = f64::from;
    let neg_weight = ((-t) * f(scale)) as f32;
    let s255 = (f(scale) * f(C_255)) as f32;
    let alpha255_idx = fistp(f(s255) - f(C_HALF));
    let glow07 = (f(scale) * f(C_07)) as f32;
    SkyAlphaNeg {
        neg_weight,
        alpha255_idx,
        glow07,
    }
}
