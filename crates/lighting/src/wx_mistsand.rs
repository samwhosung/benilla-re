//! Arithmetic of three weather draw and spawn functions: `0x67a1b0` Sand::DrawShader and `0x679e50`
//! Sand::DrawFixed (`systems/lighting/precipitation.md#sand`), and `0x67a7a0` Mists::SpawnPuff
//! (`precipitation.md#mist-ground`). Each is mostly calls (render state, device queries,
//! containers, the ramps `0x67ad60`, `0x67ad90`, `0x67add0`, `0x6745d0`, `0x674600`), so each
//! transcription is one stretch of it.
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`; a value left on the stack
//! stays wide.

use crate::x87::ftol;

/// `[0x7ff9d8]` = 1.0: the reciprocal numerator of the puff basis.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x7ffd74]` = 0.0: multiplied into the basis z.
const C_ZERO: f32 = f32::from_bits(0x0000_0000);
/// `[0x7ffe58]` = 255.0.
const C_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x8029cc]` = 512.0.
const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x80679c]` = `0x3e4ccccd` (0.2): subtracted from the sand clamp span for the ramp.
const C_SAND_BIAS: f32 = f32::from_bits(0x3e4c_cccd);
/// `[0x810028]` = `0x3b23d70a` (0.0025): the texture scroll scale.
const C_MIST_SCROLL: f32 = f32::from_bits(0x3b23_d70a);

/// `0x67a1b0` from `0x67a411` to `0x67a437`: the texture scroll `(dy - dx) * 0.0025` (`dy` at
/// `[ebp-0xc]`, `dx` at `[ebp-0x14]`), rounded at the `fstp [ebp-0x28]`.
#[must_use]
pub fn mist_tex_scroll(dy: f32, dx: f32) -> f32 {
    let f = f64::from;
    ((f(dy) - f(dx)) * f(C_MIST_SCROLL)) as f32
}

/// What `0x67a7a0` computes from the camera basis between `0x67a7f3` and `0x67a872`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MistBasis {
    /// `[ebp-0x14]`: the x step.
    pub step_a: f32,
    /// `[ebp-0x10]`: the y step.
    pub step_b: f32,
    /// `[ebp-0xc]`: the z step, `inv * 0 * q`.
    pub step_z: f32,
    /// `eax`: the node count, before the caller clamps it to 0x40.
    pub count: i32,
}

/// `0x67a7a0` from `0x67a7f3` to `0x67a872`: the step of the puff nodes and their count from the
/// basis components `a` (`[esi+0x114]`) and `b` (`[esi+0x118]`), the scalar `q` (`[ebp+8]`) and the
/// length `s` (`[ebp-4]`). `len = sqrt(a² + b²)` and `inv = 1 / len` are wide, `inv` stored as f32
/// (`fst [ebp+0xc]`) while the register stays wide: the x product uses the wide `inv`, the y and z
/// products the stored one. Each is rounded, multiplied by `q` (x from its stored value, y and z
/// from the wide products) and stored. The count is `__ftol((len * s) / sqrt(step_a² + step_b²))`,
/// the squares of the stored steps.
#[must_use]
pub fn mist_basis_count(a: f32, b: f32, q: f32, s: f32) -> MistBasis {
    let f = f64::from;
    let len = (f(a) * f(a) + f(b) * f(b)).sqrt();
    let inv = f(C_ONE) / len;
    let inv_f32 = inv as f32;
    let step_a0 = (f(a) * inv) as f32;
    let step_b0 = f(b) * f(inv_f32);
    let step_z0 = f(inv_f32) * f(C_ZERO);
    let step_a = (f(step_a0) * f(q)) as f32;
    let step_b = (step_b0 * f(q)) as f32;
    let step_z = (step_z0 * f(q)) as f32;
    let ls = len * f(s);
    let mag = (f(step_a) * f(step_a) + f(step_b) * f(step_b)).sqrt();
    let count = ftol(ls / mag);
    MistBasis {
        step_a,
        step_b,
        step_z,
        count,
    }
}

/// `0x67a7a0` from `0x67a8a4` to `0x67a8cb`: one pass of the node loop, `pos += step` per axis,
/// each rounded at its store.
#[must_use]
pub fn mist_node_accumulate(pos: [f32; 3], step: [f32; 3]) -> [f32; 3] {
    let f = f64::from;
    [
        (f(pos[0]) + f(step[0])) as f32,
        (f(pos[1]) + f(step[1])) as f32,
        (f(pos[2]) + f(step[2])) as f32,
    ]
}

/// `0x67a7a0` at `0x67a8e4`, `0x67a8f6` and `0x67a908`: the span `hi - lo` of two samples, wide,
/// that goes straight into `fcomp` against 1.0, 0.75 (`[0x8012cc]`) and 0.5 respectively.
#[must_use]
pub fn mist_gate_diff(hi: f32, lo: f32) -> f64 {
    f64::from(hi) - f64::from(lo)
}

/// The three tests at `0x67a8e4` to `0x67a91a`: `fcomp limit; test ah,0x41; je` rejects only when
/// the wide span `hi - lo` is above the limit (equal and NaN pass).
#[must_use]
pub fn mist_gate_rejects(hi: f32, lo: f32, limit: f32) -> bool {
    mist_gate_diff(hi, lo) > f64::from(limit)
}

/// `0x67a7a0` from `0x67a939` to `0x67a946`: the position of node `idx_plus1` along the line,
/// `(idx_plus1 / count) * s` (`fild`, `fidiv` by the count in `[esi+0x104]`), rounded at the `fstp
/// dword [esp]` that feeds `0x6745d0`.
#[must_use]
pub fn mist_node_spacing(idx_plus1: i32, count: i32, s: f32) -> f32 {
    (f64::from(idx_plus1) / f64::from(count) * f64::from(s)) as f32
}

/// `0x679e50` from `0x67a013` to `0x67a01a`: the span `[ebp-0x14] - [ebp-0x18]` that bounds the
/// clamp, rounded at the `fstp [ebp-8]`.
#[must_use]
pub fn sand_clamp_span(mid: f32, lo: f32) -> f32 {
    (f64::from(mid) - f64::from(lo)) as f32
}

/// `0x679e50` from `0x67a01c` to `0x67a022`: the value `[ebp-0xc] - [ebp-0x18]`, rounded at the
/// `fst [ebp-4]`. The ladder after it keeps the stored value; `fcomp 0.0; test ah,5; jp` replaces a
/// value below 0 by 0, and `fcomp span; jnp` keeps a value below the span and replaces any other,
/// NaN included, by the span.
#[must_use]
pub fn sand_clamp_value(hi: f32, lo: f32) -> f32 {
    (f64::from(hi) - f64::from(lo)) as f32
}

/// The three components `0x679e50` stores from `0x67a04e` to `0x67a0a2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SandTransform {
    /// `[ebp-0x40]`: `f32(c * m0 + t0) - o0`.
    pub o40: f32,
    /// `[ebp-0x3c]`: `(c * m1 + t1) - o1`, the sum kept wide.
    pub o3c: f32,
    /// `[ebp-0x38]`: `(f32(c * m2) + t2) - o2`.
    pub o38: f32,
}

/// `0x679e50` from `0x67a04e` to `0x67a0a2`: a grain's position, the clamped scalar `c` (`[ebp-4]`)
/// times the vector `m` (`[esi-8]`, `[esi-4]`, `[esi]`) plus the translation `t` (`[esi-0x14]` to
/// `[esi-0xc]`), less the origin `o` (`[ebp-0x34]` to `[ebp-0x2c]`). The x sum is stored as f32
/// before the subtraction; the y sum stays wide; the z product is stored as f32 and the sum with
/// `t2` stays wide.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn sand_transform(
    c: f32,
    m0: f32,
    m1: f32,
    m2: f32,
    t0: f32,
    t1: f32,
    t2: f32,
    o0: f32,
    o1: f32,
    o2: f32,
) -> SandTransform {
    let f = f64::from;
    let s0 = (f(c) * f(m0) + f(t0)) as f32;
    let s1 = f(c) * f(m1) + f(t1);
    let s2 = f((f(c) * f(m2)) as f32) + f(t2);
    SandTransform {
        o40: (f(s0) - f(o0)) as f32,
        o3c: (s1 - f(o1)) as f32,
        o38: (s2 - f(o2)) as f32,
    }
}

/// `0x679e50` from `0x67a0a5` to `0x67a0b1`: the argument of the ramp setup `0x67ad60`, the span
/// less 0.2, rounded at the `fstp dword [esp]`.
#[must_use]
pub fn sand_ramp_arg(v: f32) -> f32 {
    (f64::from(v) - f64::from(C_SAND_BIAS)) as f32
}

/// `0x679e50` from `0x67a0cf` to `0x67a0e8`: the colour word `(e * v) * 255 + 512` with `e` the
/// double `0x67add0` returns on st0 and `v` the span, rounded at the `fstp [ebp-8]`; its bits `>>
/// 14` are the colour byte.
#[must_use]
pub fn sand_color_repack(st0: f64, v: f32) -> f32 {
    let f = f64::from;
    ((st0 * f(v)) * f(C_255) + f(C_512)) as f32
}
