//! Arithmetic of `0x67ae20` Mists::Render (`systems/lighting/precipitation.md#mist`): the half
//! extent, the corner frame of a puff quad from the view matrix, the per-cell node accumulation and
//! phase clock, the hash to node count, the cell colour interpolation and its drift clamp, the life
//! and fog ramps and the camera distance of a corner. The rest is calls (the device and ambient
//! getters, the buffer calls, the ramps `0x67ad60`, `0x67ad90`, `0x67add0`, `0x674600`, the hash
//! `0x4531e0`, the node setup `0x67a7a0`).
//!
//! Values are f64 on the x87 stack and round to f32 at each `fstp dword`; a value left on the stack
//! stays wide (the colour interpolation reaches its compare that way).

use crate::x87::fistp;

/// `[0x7ffa24]` = 0.5: the half scale of the extent and the colour bias.
const C_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x7ff9d8]` = 1.0.
const C_ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x8680e8]` = 0.5: subtracted before the node index is rounded.
const C_HALF_B: f32 = f32::from_bits(0x3f00_0000);
/// `[0x8029cc]` = 512.0: the bias of the colour words.
const C_512: f32 = f32::from_bits(0x4400_0000);
/// `[0x80655c]` = `0x3fd55555` (5/3): added to the cell phase each frame.
const C_FIVE_THIRDS: f32 = f32::from_bits(0x3fd5_5555);
/// `[0x8029b0]` = 0.25: the per-frame cap on the extent drift, times the extent.
const C_QUARTER: f32 = f32::from_bits(0x3e80_0000);
/// `[0x7ffe58]` = 255.0.
const C_255: f32 = f32::from_bits(0x437f_0000);

/// `0x67ae20` from `0x67ae66` to `0x67ae96`: the half extent `[edi+0x1c] * 0.5`, rounded at the
/// `fstp [ebp-0x18]`.
#[must_use]
pub fn mistr_half_scale(extent: f32) -> f32 {
    (f64::from(extent) * f64::from(C_HALF)) as f32
}

/// The two scaled rows of the corner frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerBasis {
    /// `[ebp-0x54]`: `r0.x * half`.
    pub a0: f32,
    /// `[ebp-0x50]`: `r0.y * half`.
    pub a1: f32,
    /// `[ebp-0x4c]`: `r0.z * half`.
    pub a2: f32,
    /// `[ebp-0x40]`: `r1.x * half`.
    pub b0: f32,
    /// `[ebp-0x3c]`: `r1.y * half`.
    pub b1: f32,
    /// `[ebp-0x38]`: `r1.z * half`.
    pub b2: f32,
}

/// `0x67ae20` from `0x67af1d` to `0x67af7b`: the view matrix rows `r0` (`[ebp-0xc4]`, `[ebp-0xb4]`,
/// `[ebp-0xa4]`) and `r1` (`[ebp-0xc0]`, `[ebp-0xb0]`, `[ebp-0xa0]`) times `half`, each rounded.
#[must_use]
pub fn mistr_corner_basis(r0: [f32; 3], r1: [f32; 3], half: f32) -> CornerBasis {
    let f = f64::from;
    let h = f(half);
    CornerBasis {
        a0: (f(r0[0]) * h) as f32,
        a1: (f(r0[1]) * h) as f32,
        a2: (f(r0[2]) * h) as f32,
        b0: (f(r1[0]) * h) as f32,
        b1: (f(r1[1]) * h) as f32,
        b2: (f(r1[2]) * h) as f32,
    }
}

/// `0x67ae20` at `0x67af7b` to `0x67b098`: one component of a quad corner, `base + add` rounded at
/// its store. Corners 0 and 2 add a negated component to the other row's, corners 1 and 3 add the
/// rows (corner 3 the stored corner 1).
#[must_use]
pub fn mistr_corner_add(base: f32, add: f32) -> f32 {
    (f64::from(base) + f64::from(add)) as f32
}

/// `0x67ae20` at the `fchs` of `0x67af7e` and `0x67b000`: the negated component, an exact sign
/// flip.
#[must_use]
pub fn mistr_negate(x: f32) -> f32 {
    -x
}

/// `0x67ae20` from `0x67b11b` to `0x67b138`: `half * half`, stored at `[ebp-0x74]`.
#[must_use]
pub fn mistr_fade_mag(half: f32) -> f32 {
    (f64::from(half) * f64::from(half)) as f32
}

/// `0x67ae20` from `0x67b141` to `0x67b172`: the phase clock `dt * rate + clock` (`dt` the frame
/// time `[0xc62510]`, `rate` `[edi+0x20]`, `clock` `[edi+0x2c]`), rounded at the `fst`.
#[must_use]
pub fn mistr_time_accum(dt: f32, rate: f32, clock: f32) -> f32 {
    (f64::from(dt) * f64::from(rate) + f64::from(clock)) as f32
}

/// `0x67ae20` from `0x67b261` to `0x67b272`: the clock less 1.0, rounded at the store.
#[must_use]
pub fn mistr_phase_wrap(clock: f32) -> f32 {
    (f64::from(clock) - f64::from(C_ONE)) as f32
}

/// `0x67ae20` from `0x67b287` to `0x67b2b5`: the node index for the hash float `h` in [1, 2) and
/// the cell `count`: `(h - 1) * count` (`fimul`) is stored as f32, `- 0.5` is taken wide and
/// `fistp` rounds to nearest with ties to even (`0x80000000` for NaN and out of range). So an exact
/// odd product minus 0.5 is a tie that rounds down.
#[must_use]
pub fn mistr_hash_count(h: f32, count: i32) -> i32 {
    let f = f64::from;
    let v = ((f(h) - f(C_ONE)) * f64::from(count)) as f32;
    fistp(f(v) - f(C_HALF_B))
}

/// The three accumulators `0x67ae20` stores at `[esi+0x108]` to `[esi+0x110]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeAccum {
    /// `[esi+0x108]`.
    pub acc0: f32,
    /// `[esi+0x10c]`.
    pub acc1: f32,
    /// `[esi+0x110]`.
    pub acc2: f32,
}

/// `0x67ae20` from `0x67b2e1` to `0x67b357`: the accumulators for the node basis `n` (`[esi+0x114]`
/// to `[esi+0x11c]`), `half`, `fade_mag` (`[ebp-0x74]`), `scal` (`[esi+0x128]`) and the running
/// `acc`. With `dmix = f32(fade_mag * scal * 0.5)` each component is `acc + (n * half + dmix)`: the
/// x sum is stored as f32 before it is added, the y sum stays wide, and the z product is stored as
/// f32 before `dmix`.
#[must_use]
pub fn mistr_node_accum(
    n: [f32; 3],
    half: f32,
    fade_mag: f32,
    scal: f32,
    acc: [f32; 3],
) -> NodeAccum {
    let f = f64::from;
    let h = f(half);
    let s0 = f(n[0]) * h;
    let s1 = f(n[1]) * h;
    let s2 = (f(n[2]) * h) as f32;
    let dmix = (f(fade_mag) * f(scal) * f(C_HALF)) as f32;
    let mix0 = (s0 + f(dmix)) as f32;
    let mix1 = s1 + f(dmix);
    let mix2 = f(s2) + f(dmix);
    NodeAccum {
        acc0: (f(mix0) + f(acc[0])) as f32,
        acc1: (mix1 + f(acc[1])) as f32,
        acc2: (mix2 + f(acc[2])) as f32,
    }
}

/// `0x67ae20` from `0x67b372` to `0x67b386`: with the weather tick value `q` left on the stack by
/// `0x674600`, `inv = f32(1 / spacing)` (`[ebp-0x2c]`) and `q * inv`, each stored.
#[must_use]
pub fn mistr_spacing_recip(q: f64, spacing: f32) -> (f32, f32) {
    let inv = (f64::from(C_ONE) / f64::from(spacing)) as f32;
    let prod = (q * f64::from(inv)) as f32;
    (inv, prod)
}

/// The values of the cell index selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellRound {
    /// `[ebp-0x4]` after `0x67b3b9`: `frac * count`.
    pub n_clamped: f32,
    /// `[ebp-0x8]` after `0x67b3c2`: `frac * count + 512`.
    pub round_a: f32,
    /// `[ebp-0x8]` after `0x67b3d9`: `count * clamped + 512`.
    pub round_b: f32,
}

/// `0x67ae20` from `0x67b38e` to `0x67b3dc`, with `q` left on the stack by `0x674600`: `clamped =
/// min(arg * q, 1.0)` (`fcom 1.0; test ah,5; jnp` keeps the product only below 1; equal, above and
/// NaN give 1.0), `n_clamped = frac * count` with `count` from `fild`, `round_a = n_clamped + 512`
/// and `round_b = count * clamped + 512`, each stored as f32; the bits `>> 14` of the two words are
/// the ramp indices.
#[must_use]
pub fn mistr_cell_round(q: f64, arg: f32, count: i32, frac: f32) -> CellRound {
    let f = f64::from;
    let prod = f(arg) * q;
    let clamped = clamp_min_one(prod);
    let n = f64::from(count);
    let n_clamped = f(frac) * n;
    let round_a = n_clamped + f(C_512);
    let prod2 = n * clamped;
    let round_b = prod2 + f(C_512);
    CellRound {
        n_clamped: n_clamped as f32,
        round_a: round_a as f32,
        round_b: round_b as f32,
    }
}

/// The clamp at `0x67b393`: `v` below 1.0 stays, 1.0 and above and NaN give 1.0.
#[inline]
fn clamp_min_one(v: f64) -> f64 {
    if v < f64::from(C_ONE) {
        v
    } else {
        f64::from(C_ONE)
    }
}

/// `0x67ae20` from `0x67b3f5` to `0x67b414`: the interpolated extent of a cell, left wide on st0:
/// `(ramp_hi - ramp_lo) * (frac - frac_i) + ramp_lo + extent * 0.5` (`fild` of `frac_i`; the
/// difference is taken `[ebp-4] - frac_i`).
#[must_use]
pub fn mistr_cell_lerp(ramp_lo: f32, ramp_hi: f32, frac_i: i32, frac: f32, extent: f32) -> f64 {
    let f = f64::from;
    let d = f(ramp_hi) - f(ramp_lo);
    let t = f(frac) - f64::from(frac_i);
    let v = d * t + f(ramp_lo);
    v + f(extent) * f(C_HALF)
}

/// `0x67ae20` at `0x67b414`: `fcom [esi+0x110]; test ah,0x41; jne` skips the drift for a lerp at or
/// below the current extent and for NaN, so the cell advances only when the wide `lerp > old`.
#[must_use]
pub fn mistr_drift_gate(lerp: f64, old: f32) -> bool {
    lerp > f64::from(old)
}

/// `0x67ae20` from `0x67b421` to `0x67b433`: the cell phase plus 5/3, rounded at the store.
#[must_use]
pub fn mistr_phase_advance(phase: f32) -> f32 {
    (f64::from(phase) + f64::from(C_FIVE_THIRDS)) as f32
}

/// `0x67ae20` from `0x67b433` to `0x67b45a`: the new extent of a cell when the gate passes. `delta =
/// lerp - old` wide, `cap = f32(extent * 0.25)` is stored, `fcom cap; test ah,5; jnp` keeps `delta`
/// only below the cap (equal, above and NaN take the cap) and the result is that plus `old`,
/// rounded at the store.
#[must_use]
pub fn mistr_drift_clamp(lerp: f64, old: f32, extent: f32) -> f32 {
    let f = f64::from;
    let delta = lerp - f(old);
    let cap = (f(extent) * f(C_QUARTER)) as f32;
    let chosen = clamp_to_cap(delta, f(cap));
    (chosen + f(old)) as f32
}

/// The clamp at `0x67b445`: `delta` below `cap` stays, otherwise `cap`.
#[inline]
fn clamp_to_cap(delta: f64, cap: f64) -> f64 {
    if delta < cap {
        delta
    } else {
        cap
    }
}

/// `0x67ae20` from `0x67b46d` to `0x67b488`: `life - lo` (`[ebp-0x2c] - [edi+0x30]`), the argument
/// of the ramp setup, rounded at the store.
#[must_use]
pub fn mistr_life_span(life: f32, lo: f32) -> f32 {
    (f64::from(life) - f64::from(lo)) as f32
}

/// `0x67ae20` at `0x67b4ad`: `e * v` for the double `e` that `0x67add0` returns on st0 and the f32
/// `v` the ramp evaluator stored at `[ebp-8]`, rounded at the store to `[ebp-0x64]`.
#[must_use]
pub fn mistr_ramp_product(e: f64, v: f32) -> f32 {
    (e * f64::from(v)) as f32
}

/// The values of a corner's camera-relative position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerDist {
    /// `[ebp-0x14]`: `wx - cam.x`.
    pub d14: f32,
    /// `[ebp-0x10]`: `wy - cam.y`.
    pub d10: f32,
    /// `[ebp-0xc]`: `wz - cam.z`.
    pub dc: f32,
    /// `[esp]`: the distance.
    pub dist: f32,
}

/// `0x67ae20` from `0x67b4d0` to `0x67b526`: a corner offset plus the cell centre, less the camera
/// `[0xc7cf20]`. The x and y sums stay wide into the subtraction and the z sum is stored as f32
/// first. `dz` is stored with `fst` and the next multiply squares the wide `dz` by the stored one;
/// the sum is `(dz² + dy²) + dx²` of those, and the distance is its square root.
#[must_use]
pub fn mistr_corner_dist(corner: [f32; 3], center: [f32; 3], cam: [f32; 3]) -> CornerDist {
    let f = f64::from;
    let wx = f(corner[0]) + f(center[0]);
    let wy = f(corner[1]) + f(center[1]);
    let wz = (f(corner[2]) + f(center[2])) as f32;
    let d14 = (wx - f(cam[0])) as f32;
    let d10 = (wy - f(cam[1])) as f32;
    let dz_f64 = f(wz) - f(cam[2]);
    let dc = dz_f64 as f32;
    let sq_z = dz_f64 * f(dc);
    let sq_y = f(d10) * f(d10);
    let sq_x = f(d14) * f(d14);
    let sum = sq_z + sq_y + sq_x;
    let dist = sum.sqrt() as f32;
    CornerDist { d14, d10, dc, dist }
}

/// `0x67ae20` from `0x67b542` to `0x67b551`: the colour word `(e * v) * 255 + 512` with `e` the
/// double `0x67add0` returns and `v` the stored product, rounded at the `fstp [ebp-4]`; its bits
/// `>> 14` are the colour byte.
#[must_use]
pub fn mistr_color_repack(e: f64, brightness: f32) -> f32 {
    let f = f64::from;
    ((e * f(brightness)) * f(C_255) + f(C_512)) as f32
}
