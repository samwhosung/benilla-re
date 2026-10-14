//! Reimplements the arithmetic of the world-interaction functions: `CheckCorpseRange`
//! (`0x492130`), `WorldClickNothing` (`0x492d30`), `0x492e90`, `ApplyAutoUiScale` (`0x492f70`)
//! and `SetInteractionTarget` (`0x4930d0`). The x87 runs at 53-bit precision, so each chain is
//! `f64` with `as f32` at the `fstp dword` stores; the tests in `tests/world_interaction.rs` diff
//! each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `DAT_0080297c` = 3.0 (`0x40400000`), the 0x492e90/0x492f70 `·3.0` scale.
pub const K3_BITS: u32 = 0x4040_0000;
/// `DAT_0080306c` = 4.0 (`0x40800000`), the 0x492e90/0x492f70 compare bound and the 0x4930d0 fold.
pub const K4_BITS: u32 = 0x4080_0000;
/// `DAT_008012cc` = 0.75 (`0x3f400000`), the 0x492e90/0x492f70 `·0.75` scale.
pub const K075_BITS: u32 = 0x3f40_0000;
/// `DAT_00801360` = 0.001 (`0x3a83126f`), the 0x492e90/0x492f70 `|G − x|` change-epsilon bound.
pub const EPS_BITS: u32 = 0x3a83_126f;
/// `DAT_0080458c` = 768.0 (`0x44400000`), the 0x492f70 reciprocal-unit numerator.
pub const K768_BITS: u32 = 0x4440_0000;
/// `DAT_008044b0` = 10.0 (`0x41200000`), the 0x4930d0 fold base.
pub const K10_BITS: u32 = 0x4120_0000;

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// (1) 0x492130 / (2) 0x492d30 — squared distance / squared edge length, `(Δz²+Δx²)+Δy²` order.
// ===========================================================================================

/// The shared squared-magnitude reduction of both `0x492130` (point − 3 globals) and `0x492d30`
/// (`p1 − p0` edge delta). Given the three deltas `(dx, dy, dz)` already formed by the three
/// `fsub`s, the x87 reduction is (disasm order, both bodies identical):
/// `fld dz; fmul st,st → dz²; fld dx; fmul → dx²; faddp → dz²+dx²; fld dy; fmul → dy²; faddp →
/// (dz²+dx²)+dy²`. The whole chain stays PC_53 `f64`, rounding to `f32` only when the caller
/// stores/compares the result. Returns the `f64` sum (un-narrowed); the per-site wrappers apply
/// the terminal `as f32`.
#[inline]
pub fn sqlen_f64(dx: f64, dy: f64, dz: f64) -> f64 {
    ((dz * dz) + (dx * dx)) + (dy * dy)
}

/// `0x492130`'s owned arithmetic (`0x49216c`..`0x492196`): a fetched point `p = [eax+0/4/8]` minus
/// the three coordinate globals `(gx, gy, gz) = [0xb4e284]/[0xb4e288]/[0xb4e28c]`, reduced to the
/// squared distance `(Δz²+Δx²)+Δy²`. The result feeds `fcomp [0xb4e2ac]` (a radius² threshold)
/// downstream. The three deltas stay live on the x87 stack (no per-delta `fstp dword`), so each is
/// `f64` through the reduction; the terminal `as f32` mirrors the value the orchestration observes.
/// Returns the squared distance.
pub fn sqdist130(px: f32, py: f32, pz: f32, gx: f32, gy: f32, gz: f32) -> f32 {
    let dx = f64::from(px) - f64::from(gx);
    let dy = f64::from(py) - f64::from(gy);
    let dz = f64::from(pz) - f64::from(gz);
    sqlen_f64(dx, dy, dz) as f32
}

/// `0x492d30`'s owned arithmetic. The edge `(p0, p1)` is `p0 = [esi+0/4/8]`, `p1 = [esi+0xc/0x10/
/// 0x14]`; the three deltas `(Δx, Δy, Δz) = (p1 − p0)` are each `fstp`'d to `[ebp-0x10/-0xc/-8]`
/// (so each delta narrows to `f32` BEFORE the reduction — unlike 0x492130's fully-live chain), then
/// reloaded and reduced `(Δz²+Δx²)+Δy²` and compared against `[0x7ffd74] = 0.0`. The
/// narrow-then-reload is faithful: each `Δ` is `(p1−p0) as f32`, then the reduction widens those
/// `f32`s back to `f64`. Returns the squared edge length.
pub fn sqlen_d30(p0: [f32; 3], p1: [f32; 3]) -> f32 {
    // Each delta is `fld p1.k; fsub p0.k; fstp dword [m]` → it narrows to f32 at the store
    // ([ebp-0x10]/[ebp-0xc]/[ebp-0x8] for Δx/Δy/Δz).
    let dx = (f64::from(p1[0]) - f64::from(p0[0])) as f32;
    let dy = (f64::from(p1[1]) - f64::from(p0[1])) as f32;
    let dz = (f64::from(p1[2]) - f64::from(p0[2])) as f32;
    // The reduction reloads the f32 deltas (widen to f64), products + sums PC_53, then `fcomp`.
    sqlen_f64(f64::from(dx), f64::from(dy), f64::from(dz)) as f32
}

// ===========================================================================================
// (3) 0x492e90 / (4) 0x492f70 — single-axis fraction clamp/commit primitives.
// ===========================================================================================

/// The `G · 3.0` step shared by `0x492e90` (`0x492ec8`) and `0x492f70` (`0x492fd3`): `fld [G];
/// fmul [3.0]`. The product feeds `fcomp [4.0]` (consumed). `g` is the global `[0xb4e30c]`
/// (0x492e90) or the local `[ebp-4]` (0x492f70). Returns the f32 product.
pub fn mul3(g: f32) -> f32 {
    (f64::from(g) * k(K3_BITS)) as f32
}

/// The `G · 0.75` step shared by `0x492e90` (`0x492ee1`) and `0x492f70` (`0x492fe9`): `fld [G];
/// fmul [0.75]`. The product feeds `fcomp st(1)` (consumed). Returns the f32 product.
pub fn mul075(g: f32) -> f32 {
    (f64::from(g) * k(K075_BITS)) as f32
}

/// The `|G − x|` change-magnitude shared by `0x492e90` (`0x492f1b`) and `0x492f70` (`0x493022`):
/// `fld [G]; fsub [x]; fabs`. The result feeds `fcomp [0.001]` (a "did it move?" epsilon). `g` is
/// `[0xb4e30c]`, `x` the committed `[ebp-4]`. Both widen to `f64`; `fabs` is exact. Returns `|G − x|`.
pub fn abs_diff(g: f32, x: f32) -> f32 {
    (f64::from(g) - f64::from(x)).abs() as f32
}

/// `0x492f70`'s reciprocal-unit step (`0x492fc4`..`0x492fcd`, the `count > 0x300` path): `fild
/// [ebp-8] = count; fdivr [768.0]; fstp [ebp+8]`. `fdivr` is `st0 = mem / st0`, i.e. `768.0 /
/// (i32)count`. The `count` is `fild`'d (i32 → exact f64), the divide is PC_53, the `fstp dword`
/// narrows to `f32`. `count` is the signed i32 the caller stores at `[ebp-8]`. Returns the quotient.
pub fn recip_unit(count: i32) -> f32 {
    (k(K768_BITS) / f64::from(count)) as f32
}

// ===========================================================================================
// (5) 0x4930d0 — threshold setter: the EXACT constant fold `(10 + 4)² = 196.0`.
// ===========================================================================================

/// `0x4930d0`'s owned constant fold (`0x49315e`..`0x493171`): `fld [10.0]; fadd [4.0]; fld st(0);
/// fmul st,st(1); fstp [0xb4e2c0]`. Computes `t = 10.0 + 4.0 = 14.0` then `t · t = 196.0` and stores
/// it to the radius² global `[0xb4e2c0]` — the threshold the 0x492130 / 0x492d30 squared-distance
/// compares read. All operands are exact-representable `f32`s, so the PC_53 fold is `196.0` exactly.
/// Returns the stored f32 (`196.0`).
pub fn fold_196() -> f32 {
    let t = k(K10_BITS) + k(K4_BITS); // fld [10.0]; fadd [4.0]  → 14.0 (PC_53, live)
    (t * t) as f32 // fld st(0); fmul st,st(1); fstp [m32]
}
