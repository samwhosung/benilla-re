//! Reimplements the arithmetic of `GetTaxiNodeFare` (`0x4dc3d0`), `ActionSlotHasRange`
//! (`0x4e5810`), the negated fans of `0x4ea790` and `0x4ea850`, and the halving of `0x4ee100`.
//! The x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword`
//! stores; the tests in `tests/fare_range.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `DAT_007ff9d8` = 1.0 (`0x3f800000`), the 0x4dc3d0 `fsubr` reverse-subtract operand (`1.0 - r`).
pub const ONE_BITS: u32 = 0x3f80_0000;
/// `DAT_008029d4` = 2.384185791015625e-07 = `2^-22` (`0x34800000`), the 0x4e5810 near-zero epsilon.
pub const EPS_BITS: u32 = 0x3480_0000;
/// `DAT_007ffa24` = 0.5 (`0x3f000000`), the 0x4ee100 halving constant.
pub const HALF_BITS: u32 = 0x3f00_0000;

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// (1) 0x4dc3d0 — the taxi node cost (`TaxiNodeCost`'s worker): `(1.0 - r) · n` then round-to-nearest
//     int, ×2 identical blocks.
// ===========================================================================================

/// The owned arithmetic of `0x4dc3d0`'s two parallel blocks (`0x4dc462`..`0x4dc480` and
/// `0x4dc4a7`..`0x4dc4b5`), each the same shape:
/// * the vendor price discount `0x612b80(flight master, player)` leaves `r` on `st0` (a PC_53 `f64`,
///   one of `{0, 0.05f, 0.1f, 0.1f+0.05f, 0.1f+2·0.05f}`);
/// * `fsubr [1.0]` → `st0 = 1.0 - r`;
/// * `fild [n]` pushes the price `n` (block #1 = the direct `TaxiPath.price`, `[path+0xc]`; block #2 =
///   the slot's undiscounted route sum `[0xbb4f20 + slot·0x20]`), so `st0 = n`, `st1 = 1.0 - r`;
/// * `fmul st, st(1)` → `st0 = n · (1.0 - r)`;
/// * `fstp dword [m]` stores the product as `f32` (the arithmetic result this row owns).
///
/// Downstream (orchestration past the owned slice) the stored `f32` is reloaded and `fistp`'d back
/// to an `i32` (round-to-nearest, FPCW default) and returned in `EAX` — see [`fade_step_round`].
///
/// `r` is the callee result (fed as the full `f64` the binary leaves on `st0`); `n` the `fild`'d
/// integer. Returns the `f32` product `n · (1.0 - r)`.
pub fn fade_step(r: f64, n: i32) -> f32 {
    let one_minus = k(ONE_BITS) - r; // fsubr [1.0]: st0 = 1.0 - r   (PC_53 f64)
    (f64::from(n) * one_minus) as f32 // fild [n]; fmul st,st(1); fstp dword [m]
}

/// The full block value of `0x4dc3d0`: [`fade_step`] then the downstream `fld [m]; fistp [m2]`
/// reload-and-round (round-to-nearest-even, the FPCW-default `fistp` rounding). The reload is of the
/// already-stored `f32`, so the int is `round(product_f32)`. Returns the `i32` written to `EAX`.
pub fn fade_step_round(r: f64, n: i32) -> i32 {
    let product = fade_step(r, n); // the f32 stored at [ebp-4] / [ebp-8]
                                   // fld dword [m] reloads the f32 (widen to f64), fistp converts round-to-nearest-even.
    round_nearest_even(f64::from(product))
}

/// x87 `fistp` with the FPCW-default rounding mode (round-to-nearest, ties-to-even).
#[inline]
fn round_nearest_even(v: f64) -> i32 {
    v.round_ties_even() as i32
}

// ===========================================================================================
// (2) 0x4e5810 — `|x| >= eps  OR  |y| >= eps`   (eps = 2^-22).
// ===========================================================================================

/// One `0x4e5810` near-zero test (`fld [m]; fabs; fcomp [eps]; fnstsw; test ah,5; j(n)p`): is the
/// `f32` value `v` at least `eps = 2^-22` in magnitude? The binary loads `v` as `f32`, `fabs`es it
/// (exact), then `fcomp`s against `eps` and branches on `PF` of `test ah,5` (= `C0|C2`). Working it
/// through the x87 condition codes: the predicate is TRUE for `|v| > eps`, for `|v| == eps`, and for
/// the **unordered** (NaN) case — i.e. exactly "NOT (`|v| < eps`)". So a `NaN` input counts as "large"
/// (the `fcomp` is unordered → `C2` set → `ah&5` has even parity → `jp`/`!jnp` takes the TRUE path).
/// `+Inf` is `> eps` → TRUE. Mirrored here: `|v| >= eps`, with `NaN ⇒ true`.
#[inline]
pub fn mag_ge_eps(v: f32) -> bool {
    if v.is_nan() {
        return true; // fcomp unordered ⇒ TRUE path (the binary treats NaN as "large")
    }
    f64::from(v.abs()) >= k(EPS_BITS)
}

/// `0x4e5810`'s owned predicate over the two `f32` outputs `x = [ebp-8]`, `y = [ebp-0xc]` of the
/// upstream getter (`0x6e3480`): return `1` iff either magnitude reaches `eps`, else `0`. The binary
/// short-circuits on `x` (if `|x| >= eps` it returns 1 without testing `y`), exactly as the `||`. The
/// upstream gating (`ecx` range, table lookup, the zero-`[ebp-4]` flag) is orchestration outside the
/// owned FP slice. Returns the `i32` the function leaves in `EAX` (0 or 1).
pub fn near_zero_pair(x: f32, y: f32) -> i32 {
    i32::from(mag_ge_eps(x) || mag_ge_eps(y))
}

// ===========================================================================================
// (3) 0x4ea790 / 0x4ea850 — fan g and -g into a block of f32 cells. (4ea850 is the byte-clone.)
// ===========================================================================================

/// The 12-cell `f32` block `0x4ea790` (and its byte-clone `0x4ea850`) initializes — a 4×3 matrix-like
/// layout at `[0xbc81f8 .. 0xbc8224]` (relative slot `[base + 4·i]`, `i` in `0..12`). A single global
/// `g` is loaded and negated once (`fchs`, the only arithmetic op — exact sign flip), then `g` / `-g`
/// are fanned (via `fld st(0)`/`fld st(1)`/`fxch` shuffles + eight `fstp dword`) into the non-zero
/// slots, while every 3rd slot is cleared by a plain integer `mov dword [m], 0` (not FP). The layout,
/// in slot order (traced through the x87 stack, the `mov`-zeroed slots interleaved):
/// ```text
///   [+0]=-g  [+1]=-g  [+2]=0    (row 0)
///   [+3]= g  [+4]=-g  [+5]=0    (row 1)
///   [+6]=-g  [+7]= g  [+8]=0    (row 2)
///   [+9]= g  [+10]=g  [+11]=0   (row 3)
/// ```
/// `g` is the source global (`[0xbc762c]` for 0x4ea790, `[0xbc7630]` for the clone 0x4ea850). Returns
/// all 12 stored `f32`s in slot order. `-g` is `fchs` (an exact sign flip, so `fchs(+0.0) = -0.0`).
pub fn fan_neg(g: f32) -> [f32; 12] {
    let n = -g; // fchs (exact: negates the sign bit, incl. +0.0 -> -0.0)
    let z = 0.0f32;
    [n, n, z, g, n, z, n, g, z, g, g, z]
}

// ===========================================================================================
// (4) 0x4ee100 — two vtable-getter floats, each halved, stored into a struct.
// ===========================================================================================

/// One `0x4ee100` halving (`fmul [0.5]; fstp dword [m]`): a vtable-dispatched getter
/// (`call [eax+0x20]` / `call [edx+0x1c]`) leaves a `float` on `st0` (a PC_53 `f64`), which is
/// multiplied by `0.5` and stored as `f32`. `v` is the getter result (fed as the full `f64` left on
/// `st0`). Returns the `f32` product `v · 0.5`.
#[inline]
pub fn half(v: f64) -> f32 {
    (v * k(HALF_BITS)) as f32
}

/// `0x4ee100`'s owned pair: halve the first getter's result and the second's. The binary stores the
/// second product (`b`) and the first (`a`) into a struct at `[obj+0x338]+0x390` (`+0` = `b`,
/// `+4` = `a`, `+8` = `0`), plus copies of `a` to `[ebp-0xc]`; that placement is orchestration. The
/// owned arithmetic is the two independent `·0.5`. `va`/`vb` are the two getter `st0` results.
/// Returns `(a, b) = (va·0.5, vb·0.5)` as `f32`.
pub fn half_pair(va: f64, vb: f64) -> (f32, f32) {
    (half(va), half(vb))
}
