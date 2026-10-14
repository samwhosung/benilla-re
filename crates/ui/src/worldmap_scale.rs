//! Reimplements the scale and gate arithmetic of the world-map functions `SetMap` (`0x4a67a0`),
//! `GetSelectedTextureHeight` (`0x4a6d50`), `0x4a7a80`, `CenterArrowModel` (`0x4a7b20`) and
//! `CreateMiniArrowModel` (`0x4a7b80`). The x87 runs at 53-bit precision, so each chain is `f64`
//! with `as f32` at the `fstp dword` stores; the tests in `tests/worldmap_scale.rs` diff each
//! against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `DAT_008029d4` = `2.384185791015625e-07` (`0x34800000`, = 2^-22), the 0x4a67a0 near-zero epsilon
/// the six `fabs` results are `fcomp`'d against.
pub const EPS_BITS: u32 = 0x3480_0000;
/// `DAT_00806538` = 1002.0 (`0x447a8000`), the 0x4a6d50 aspect-ratio numerator scale.
pub const SCALE1002_BITS: u32 = 0x447a_8000;
/// `DAT_0080655c` = `1.6666666269302368` (`0x3fd55555`, ≈ 5/3), the 0x4a7a80 scale.
pub const FIVE_THIRDS_BITS: u32 = 0x3fd5_5555;
/// `DAT_00806560` = `1.1111111640930176` (`0x3f8e38e4`, ≈ 10/9), the 0x4a7b80 scale.
pub const TEN_NINTHS_BITS: u32 = 0x3f8e_38e4;
/// `DAT_007ffa24` = 0.5 (`0x3f000000`), the 0x4a7b20 midpoint scale (applied twice).
pub const HALF_BITS: u32 = 0x3f00_0000;

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// (1) 0x4a67a0 — 6× near-zero magnitude test `fld [m]; fabs; fcomp [eps]`.
// ===========================================================================================

/// The `fabs` value of `0x4a67a0`'s six magnitude tests: `|x|` of an `f32` loaded into the PC_53
/// register (`fld [m]; fabs`), narrowed back to `f32` at the store point. A pure sign-bit clear,
/// so `f32`-exact. Returns `|x|`.
pub fn fabs(x: f32) -> f32 {
    (f64::from(x).abs()) as f32
}

/// The near-zero gate outcome of `0x4a67a0` (the `fld [m]; fabs; fcomp [eps]; fnstsw; test ah,5;
/// jp` shape, shared by all six sites — the `fld` operand differs, the `fabs`+`fcomp [eps]` are
/// byte-identical). Returns `true` when `|x| < eps` (the fall-through "near-zero" branch; the binary
/// takes the `jp` to the work branch when `|x| >= eps`, equality counting as not-below via `C3`).
/// For an ordered finite `x` this is exactly `|x| < eps`.
pub fn below_eps(x: f32) -> bool {
    f64::from(x).abs() < k(EPS_BITS)
}

// ===========================================================================================
// (2) 0x4a6d50 — aspect-ratio scale  (c-d)·1002.0 / (a-b)  then __ftol (truncate).
// ===========================================================================================

/// `0x4a6d50`'s owned arithmetic (`0x4a6e18`..`0x4a6e4d`, the `edx != -1` `fld` path): from a struct
/// `s` four `f32`s are loaded — `a=[+0x10]`, `b=[+0x14]`, `c=[+0x18]`, `d=[+0x1c]` — and
/// `P = a - b` (`fsub`), `Q = c - d` (`fsub`) are formed. After two `fcom 0.0` gates (skip when
/// `P == 0`), the result is `(Q · 1002.0) / P` (`fmul [1002.0]; fdiv st,st(1)`) passed to the CRT
/// `__ftol` (`0x40a2b0`), truncating toward zero. `a`/`b`/`c`/`d` are the four `f32`s. Returns the
/// truncated `i32` (the binary's `EAX` = low 32 of the i64 `fistp`; matches `trunc() as i32` for a
/// quotient in `i32` range).
pub fn aspect_ftol(a: f32, b: f32, c: f32, d: f32) -> i32 {
    let p = f64::from(a) - f64::from(b); // fsub
    let q = f64::from(c) - f64::from(d); // fsub
    let v = (q * k(SCALE1002_BITS)) / p; // fmul [1002.0]; fdiv st,st(1)
    v.trunc() as i32 // __ftol = truncate toward zero
}

// ===========================================================================================
// (3) 0x4a7a80 / 0x4a7b80 — helper(1.0) · scale  → [obj+0x3a0].
//     (4) 0x4a7b20 — two vtable getters each · 0.5.
// The owned op in each is a single `fmul [const]` applied to an `st0` produced by a delegated
// call (the `0x41ae70` helper / a vtable getter), then `fstp dword [m32]`. The transcription is
// the product; the difftest feeds the `fmul` arbitrary inputs via a copied-byte island.
// ===========================================================================================

/// The single owned `fmul [const]; fstp [m32]` shape of `0x4a7a80`/`0x4a7b20`/`0x4a7b80`: an `st0`
/// value `x` (whatever the preceding delegated call left) multiplied by a per-site `f32` constant
/// and stored as `f32`. `x` is the multiplicand, `kbits` the constant. Returns the `f32` product.
pub fn scale(x: f32, kbits: u32) -> f32 {
    (f64::from(x) * k(kbits)) as f32
}
