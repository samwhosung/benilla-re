//! Reimplements the arithmetic of the book-frame functions `0x4b0940`, `0x4b14a0`, `0x4b1600` and
//! the slot reader `ReadBookSlotArgs` (`0x4b3ec0`). The x87 runs at 53-bit precision, so each
//! chain is `f64` with `as f32` at the `fstp dword` stores; the tests in `tests/spellbook.rs`
//! diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `DAT_007ffd68` = 1024.0 (`0x44800000`), the px→UI-unit idiom's `· 1024.0`.
pub const K1024_BITS: u32 = 0x4480_0000;
/// `DAT_008066d8` = 16.0 (`0x41800000`), the idiom's primary `fdivr` numerator.
pub const K16_BITS: u32 = 0x4180_0000;
/// `DAT_0080306c` = 4.0 (`0x40800000`), the idiom's alternate numerator (one site in 0x4b0940).
pub const K4_BITS: u32 = 0x4080_0000;
/// `DAT_008029d4` = 2.384185791015625e-07 (`0x34800000`), the 0x4b14a0 "did-it-move?" epsilon.
pub const EPS_BITS: u32 = 0x3480_0000;
/// `DAT_0080679c` = 0.2 (`0x3e4ccccd`), the 0x4b1600 lower clamp bound.
pub const CLAMP_LO_BITS: u32 = 0x3e4c_cccd;
/// `DAT_008015b8` = 1.0 (`0x3ff0000000000000` f64), the 0x4b3ec0 1-based → 0-based bias (QWORD).
pub const ONE_F64_BITS: u64 = 0x3ff0_0000_0000_0000;

#[inline]
fn kf32(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// The shared px→UI-unit idiom.
// ===========================================================================================

/// One application of the `call 0x41ad70; fmul[1024]; fdivr[k]; fstp[esp]; call 0x41ae60` shape.
///
/// * `l` = `[0x832a4c]` (the UI X-scale getter `0x41ad70` returns, f32).
/// * `m` = `[0x832a44]` (the scale `0x41ae60` multiplies by, f32).
/// * `k_bits` = the `fdivr` numerator (16.0 or 4.0).
///
/// Returns the value left LIVE on st0 as PC_53 f64: `M · (f32)(k / (L·1024.0))`. The inner quotient
/// is narrowed to f32 at the `fstp [esp]` (the call-arg store), then re-widened to f64 by
/// `0x41ae60`'s `fmul [ebp+8]`. The body's downstream `fst/fstp dword [m32]` narrows the returned
/// f64 to f32 — callers that store it should apply `as f32` (or use [`px_to_unit_f32`]).
pub fn px_to_unit_f64(l: f32, m: f32, k_bits: u32) -> f64 {
    let inner = (kf32(k_bits) / (f64::from(l) * kf32(K1024_BITS))) as f32; // fstp [esp] → f32
    f64::from(m) * f64::from(inner) // fld [0x832a44]; fmul [ebp+8] (f32 mem widened)
}

/// [`px_to_unit_f64`] narrowed to f32 — the value the body's `fst/fstp dword [m32]` actually stores.
pub fn px_to_unit_f32(l: f32, m: f32, k_bits: u32) -> f32 {
    px_to_unit_f64(l, m, k_bits) as f32
}

// ===========================================================================================
// (1) 0x4b0940 — 6× the px→UI-unit idiom. Five sites use k=16.0; the sixth (0x4b0acc) uses k=4.0.
// ===========================================================================================

/// The five `k = 16.0` sites of `0x4b0940` (0x4b09d2/9f8/a15/a7d/aa2). Each computes
/// `M · (f32)(16.0 / (L·1024.0))` and stores it as f32. Returns that f32. (Identical arithmetic to
/// every other `k=16` site — they differ only in which object field / call-arg the result is
/// stored to, which is orchestration.)
pub fn idiom16(l: f32, m: f32) -> f32 {
    px_to_unit_f32(l, m, K16_BITS)
}

/// The sixth site of `0x4b0940` (0x4b0acc), which divides into `4.0` instead of `16.0`:
/// `M · (f32)(4.0 / (L·1024.0))`. Returns that f32.
pub fn idiom4(l: f32, m: f32) -> f32 {
    px_to_unit_f32(l, m, K4_BITS)
}

// ===========================================================================================
// (2) 0x4b14a0 — per-axis "did it move?" gate: |new − old| compared to eps (2.384e-07).
// ===========================================================================================

/// `0x4b14a0`'s owned per-axis arithmetic (`fld [new]; fsub [old]; fabs; fcomp [eps]`): the
/// PC_53 magnitude `|new − old|`, where `new`/`old` are f32 (mem operands widened). Returned as
/// f64 so the difftest can compare it against eps with the binary's exact `fcomp` ordering.
pub fn axis_delta(new: f32, old: f32) -> f64 {
    (f64::from(new) - f64::from(old)).abs()
}

/// The branch predicate the binary's `fcomp [eps]; fnstsw; test ah,5; jp` emits for one axis:
/// `jp` is taken (→ "the value moved, do the update") exactly when `|new − old| >= eps` (the
/// `fcomp` C-flags give `st0 > mem` and `st0 == mem` the same `ah&5 = 0` ⇒ PF=1 ⇒ jp). Returns
/// `true` when the `jp` is taken (i.e. moved by at least eps).
pub fn axis_moved(new: f32, old: f32) -> bool {
    axis_delta(new, old) >= kf32(EPS_BITS)
}

// ===========================================================================================
// (3) 0x4b1600 — px→UI-unit idiom, doubled, used as the upper bound of a [0.2, hi] clamp.
// ===========================================================================================

/// `0x4b1600`'s owned arithmetic (0x4b172c..0x4b1754): the px→UI-unit idiom with `k = 16.0`, then
/// `fadd st(0),st` (double the LIVE PC_53 st0), then `fstp [ebp+0x10]` narrowing to f32. Returns
/// the f32 upper bound `2.0 · (M · (f32)(16.0 / (L·1024.0)))`. (The doubling adds st0 to itself
/// while it is still the un-narrowed PC_53 product — transcribed by doubling the f64 before the
/// final `as f32`.)
pub fn hi_bound(l: f32, m: f32) -> f32 {
    let p = px_to_unit_f64(l, m, K16_BITS); // M·(f32)(16/(L·1024)) — LIVE f64 on st0
    (p + p) as f32 // fadd st(0),st ; fstp [ebp+0x10]
}

// ===========================================================================================
// (4) 0x4b3ec0 — Lua arg → (f64)getter − 1.0 → __ftol (1-based → 0-based index, truncate).
// ===========================================================================================

/// `0x4b3ec0`'s owned arithmetic (0x4b3efd..0x4b3f08): `<getter st0>; fsub qword [1.0]; call __ftol`.
/// The getter (`0x6f3620`) returns an f64 on st0; the body subtracts 1.0 (a QWORD f64 operand —
/// NOT narrowed) and `__ftol`-truncates toward zero to an i32 (a 1-based Lua index → 0-based). The
/// result is then range-checked `0 <= idx < 0x400` (orchestration). `v` is the getter's f64 value.
/// Returns the truncated i32 index.
pub fn index_from_f64(v: f64) -> i32 {
    let biased = v - f64::from_bits(ONE_F64_BITS); // fsub qword [0x8015b8] (= 1.0)
    biased.trunc() as i32 // __ftol = truncate toward zero
}
