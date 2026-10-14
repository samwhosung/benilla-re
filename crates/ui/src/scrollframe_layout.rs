//! Reimplements the arithmetic of the layout and fade functions `0x785ea0`, `0x786200`,
//! `0x7863b0` and `0x786490`. The x87 runs at 53-bit precision, so each chain is `f64` with `as
//! f32` at the `fstp dword` stores; the tests in `tests/scrollframe_layout.rs` diff each against
//! the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact). The `[esi+0x3xx]` / `[edi+0x1x]` operands are
// RUNTIME state fields, supplied as inputs by the difftest, not constants.
// ===========================================================================================

/// 1.0 (`DAT_007ff9d8`), the reciprocal numerator of slice D (`fld 1.0; fdiv [+0x7c]`).
pub const ONE_BITS: u32 = 0x3f80_0000;
/// 0.0 (`DAT_007ffd74`), the clamp/change-detect sentinel.
pub const ZERO_BITS: u32 = 0x0000_0000;
/// norm-ε = 2⁻²² (`DAT_008029d4`), the slice-C change-detect epsilon.
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07
/// 255.0 (`DAT_007ffe58`), the 0..255 alpha-byte scale of `0x786200`.
pub const SCALE255_BITS: u32 = 0x437f_0000;
/// 0.5 (`DAT_00803078`, an **f64** constant), the level-fraction bias of `0x7863b0`.
pub const HALF_BITS_F64: u64 = 0x3fe0_0000_0000_0000;

#[inline]
fn one() -> f64 {
    f64::from(f32::from_bits(ONE_BITS))
}
#[inline]
fn eps() -> f64 {
    f64::from(f32::from_bits(EPS_BITS))
}

// ===========================================================================================
// (1) 0x785ea0 — slice A: the header coordinate block (0x785ebe..0x785efd).
// ===========================================================================================

/// Slice A of `0x785ea0` (`0x785eb5`..`0x785efd`), gated by `test [esi+0x60],1`. Computes the four
/// coordinate slots from the per-node scale `s = [+0x7c]`, the four coefficients
/// `c324 = [+0x334]`, `c32c = [+0x33c]`, `c320 = [+0x330]`, `c328 = [+0x338]`, and the four edges
/// `e68 = [+0x68]`, `e70 = [+0x70]`, `e64 = [+0x64]`, `e6c = [+0x6c]`:
/// * `[+0x324] = s·c324 + e68`   (`fld [+0x7c]; fmul [+0x334]; fadd [+0x68]; fstp [+0x324]`),
/// * `[+0x32c] = e70 − c32c·s`   (`fld [+0x33c]; fmul [+0x7c]; fsubr [+0x70]; fstp [+0x32c]`),
/// * `[+0x320] = c320·s + e64`   (`fld [+0x330]; fmul [+0x7c]; fadd [+0x64]; fstp [+0x320]`),
/// * `[+0x328] = e6c − c328·s`   (`fld [+0x338]; fmul [+0x7c]; fsubr [+0x6c]; fstp [+0x328]`).
///
/// Each product/sum folds at PC_53 (f32 operands widen exactly); each slot is `fstp`'d → f32.
/// Returns `(slot324, slot32c, slot320, slot328)` as f32.
#[allow(clippy::too_many_arguments)]
pub fn header_coords(
    s: f32,
    c324: f32,
    c32c: f32,
    c320: f32,
    c328: f32,
    e68: f32,
    e70: f32,
    e64: f32,
    e6c: f32,
) -> (f32, f32, f32, f32) {
    let sv = f64::from(s);
    let slot324 = ((sv * f64::from(c324)) + f64::from(e68)) as f32;
    let slot32c = (f64::from(e70) - (f64::from(c32c) * sv)) as f32; // fsubr: [+0x70] − c32c·s
    let slot320 = ((f64::from(c320) * sv) + f64::from(e64)) as f32;
    let slot328 = (f64::from(e6c) - (f64::from(c328) * sv)) as f32; // fsubr: [+0x6c] − c328·s
    (slot324, slot32c, slot320, slot328)
}

// ===========================================================================================
// (2) 0x785ea0 — slice C: fild-quantize change-detect (the eps gate that bounds [+0x318]).
// ===========================================================================================

/// Slice C of `0x785ea0` (`0x785f5e`..`0x785f85`): given the integer count `n = [ebp-0x20]` (the
/// `__ftol` result + 1, read as the low dword of an i64 with high dword forced 0), the sum
/// `sum = [ebp-0x8]`, and the slot pair `slot328 = [+0x328]`, `slot320 = [+0x320]`:
/// * `q = (i64)n · sum`           (`fild qword {n,0}; fmul [ebp-8]`),
/// * `d = (slot328 − slot320) − q`(`fld [+0x328]; fsub [+0x320]; fsubp st(1),st`),
/// * `|d| vs eps`                 (`fabs; fcomp [0x8029d4]=2⁻²²; fnstsw; test ah,5; jp`).
///
/// `test ah,5` masks {C0,C2}; `jp` (PF=1 ⟺ EVEN set-bits) is taken iff EVEN. After `fcomp(|d|,eps)`:
/// C0=1 iff `|d| < eps`. `|d| < eps` → C0=1 (1 bit, odd) → jp NOT taken → keeps `[+0x318] = ecx`
/// (the incremented bound); `|d| >= eps` / `== eps` / unordered → even → jp TAKEN → leaves
/// `[+0x318]` at the pre-increment value. So the bound is **kept incremented** iff `|d| < eps`
/// (ordered). Returns `true` ⇒ `|d| < eps` (false on NaN) ⇒ the binary keeps the +1.
///
/// `n` is the i64 widen of an i32 (the high dword is forced to 0, so the **unsigned** 32-bit value
/// zero-extends to i64). `sum`/`slot328`/`slot320` enter as f32 and widen exactly.
pub fn quantize_settled(n: u32, sum: f32, slot328: f32, slot320: f32) -> bool {
    let q = (i64::from(n) as f64) * f64::from(sum); // fild qword {n,0}; fmul [ebp-8]
    let d = (f64::from(slot328) - f64::from(slot320)) - q; // fsub; fsubp
    let m = d.abs();
    !m.is_nan() && m < eps()
}

// ===========================================================================================
// (3) 0x785ea0 — slice D: the tail recompute loop (0x786070..0x78618a).
// ===========================================================================================

/// Slice D's leading reciprocal + slope + inv·sum of `0x785ea0` (`0x786070`..`0x7860d4`): the inverse
/// scale and the two LIVE-st0 products it derives. `sum = [ebp-0x8]` is slice-B's stored sum-of-
/// snapped-products (`fst [ebp-0x8]` at 0x785f2d):
/// * `inv = 1.0 / s`                       (`fld [0x7ff9d8]=1.0; fdiv [+0x7c]`),
/// * `slope = (slot32c − slot324) · inv`   (`fld [+0x32c]; fsub [+0x324]; fmul st,st(1); fstp [ebp-0x14]`),
/// * `inv_sum = inv · sum`                 (`fmul [ebp-0x8]; fstp [ebp-0x18]`).
///
/// The live-st0 distinction: `inv` is computed ONCE and stays LIVE on the x87 stack across BOTH
/// products — the `fmul st,st(1)` for `slope` uses the LIVE PC_53 difference and the LIVE `inv` (st1
/// is left intact), then `fstp [ebp-0x14]` pops `slope`, leaving `inv` on st0; the `fmul [ebp-0x8]`
/// for `inv_sum` multiplies that SAME un-rounded PC_53 `inv` by the (reloaded f32) `sum`. So neither
/// product rounds `inv` to f32. Both results `fstp`'d → f32. Returns `(inv, slope_f32, inv_sum_f32)`:
/// `inv` as the PC_53 f64, `slope` (`[ebp-0x14]`) and `inv_sum` (`[ebp-0x18]`) as the stored f32s.
pub fn tail_slope(s: f32, slot32c: f32, slot324: f32, sum: f32) -> (f64, f32, f32) {
    let inv = one() / f64::from(s); // fld 1.0; fdiv [+0x7c]
    let diff = f64::from(slot32c) - f64::from(slot324); // fld [+0x32c]; fsub [+0x324]
    let slope = (diff * inv) as f32; // fmul st,st(1): live diff × live inv; fstp [ebp-0x14] → f32
    let inv_sum = (inv * f64::from(sum)) as f32; // fmul [ebp-0x8]: live inv × f32 sum; fstp [ebp-0x18]
    (inv, slope, inv_sum)
}

/// Slice D's per-child coordinate divide of `0x785ea0` (`0x7860f0`..`0x786113`): two field copies
/// divided by the scale `s = [+0x7c]`:
/// * `cx = m0 / s`  (`fld [ebp-0x10]=m0; fdiv [+0x7c]; fstp [child]`),
/// * `cy = m1 / s`  (`fld [ebp-0x4]=m1;  fdiv [+0x7c]; fstp [child+4]`),
///
/// where `m0 = [+0x334]` / `m1 = [+0x338]` were copied into `[ebp-0x10]`/`[ebp-0x4]` at
/// `0x786084`/`0x78608a`. Each quotient `fstp`'d → f32. Returns `(cx, cy)` as f32.
pub fn child_coords(m0: f32, m1: f32, s: f32) -> (f32, f32) {
    let sv = f64::from(s);
    let cx = (f64::from(m0) / sv) as f32; // fdiv [+0x7c]
    let cy = (f64::from(m1) / sv) as f32;
    (cx, cy)
}

/// Slice D's per-child negate + accumulate of `0x785ea0` (`0x786143`..`0x78618d`): the child's
/// `cy = [child+4]` is reloaded and negated to feed the layout setter `0x767c70`
/// (`fld [eax+edi+0x4]; fchs; fstp [esp]`), and the loop accumulator advances
/// `acc = [ebp-0x18] + [ebp-0x4]` (`fld [ebp-0x18]; fadd [ebp-0x4]; fstp [ebp-0x4]`).
/// Returns `(neg_cy, acc)` as f32. `neg_cy` is the exact sign flip of `cy`; `acc` folds at PC_53.
pub fn child_neg_acc(cy: f32, acc_in: f32, step: f32) -> (f32, f32) {
    let neg_cy = (-f64::from(cy)) as f32; // fld [child+4]; fchs; fstp [esp]
    let acc = (f64::from(acc_in) + f64::from(step)) as f32; // fld [ebp-0x18]; fadd [ebp-0x4]
    (neg_cy, acc)
}

// ===========================================================================================
// (4) 0x786200 — per-child fade/alpha update (the [+0x10]/[+0x14] timer→alpha-byte driver).
// ===========================================================================================

/// `0x786200`'s alpha-byte compute (`0x78633c`..`0x786369`), the owned arithmetic of the fade
/// update. Given the child timer `f14 = [edi+0x14]`, the per-frame decrement `arg = [ebp+8]`, and
/// the fade duration `dur = [esi+0x350]`:
/// * `diff = f14 − arg`             (`fld [edi+0x14]; fsub [ebp+8]`),
/// * (a `fcom 0.0` selects the path — handled by [`fade_alpha_keep`]),
/// * on the keep path: `byte = __ftol( (diff_LIVE / dur) · 255.0 )`
///   (`fst [edi+0x14]; fdiv [+0x350]; fmul [0x7ffe58]=255.0; __ftol`).
///
/// **The live-st0 trap:** `fst [edi+0x14]` stores `diff` to the f32 timer slot but leaves st0
/// LIVE, so the `fdiv` divides the **un-rounded PC_53** `diff`, not the f32 reload. `__ftol`
/// truncates toward zero (the CRT `__ftol` sets round-toward-zero in its own FPCW). Returns the
/// alpha byte `al` (the low byte of the i32 truncation, stored at `[edi+0xb]`).
pub fn fade_alpha_byte(f14: f32, arg: f32, dur: f32) -> u8 {
    let diff = f64::from(f14) - f64::from(arg); // fld; fsub (live st0, no f32 spill before fdiv)
    let v = (diff / f64::from(dur)) * f64::from(f32::from_bits(SCALE255_BITS));
    (benilla_re_cmath::x87::ftol(v)) as u8 // __ftol = truncate toward zero; mov [edi+0xb],al
}

/// `0x786200`'s keep-path predicate (`0x78633c`..`0x78634d`): after `diff = f14 − arg`,
/// `fcom 0.0; fnstsw; test ah,5; jp 0x78635b`. `fcom(diff, 0.0)` sets C0 iff `diff < 0`. mask 5 =
/// {C0,C2}; `jp` (PF=1 ⟺ EVEN set-bits) taken iff EVEN. `diff < 0` → C0=1 (1 bit, odd) → NOT taken
/// → the snap-to-end path (`fstp st(0)`; the child finishes). `diff > 0` ordered → 0 bits (even) →
/// taken → the alpha-byte compute. `diff == 0` → C3 set but masked out → even → taken. NaN → C0,C2
/// → 2 bits even → taken. So the alpha-byte compute fires iff `diff >= 0` (ordered) or unordered.
/// Returns `true` ⇒ the compute path ([`fade_alpha_byte`]) runs ⇔ NOT `diff < 0`.
pub fn fade_alpha_keep(f14: f32, arg: f32) -> bool {
    let diff = f64::from(f14) - f64::from(arg);
    // jp taken (compute) iff NOT (diff < 0 ordered): i.e. diff >= 0 ordered OR unordered (NaN).
    !matches!(diff.partial_cmp(&0.0), Some(core::cmp::Ordering::Less))
}

/// `0x786200`'s first-timer decrement (`0x7862fd`..`0x786303`): the SECOND child timer `f10 =
/// [edi+0x10]` (gated by `[edi+0x10] != 0` at 0x7862ed) is decremented by the same per-frame
/// `arg = [ebp+8]` and `fst [edi+0x10]`'d back: `dec = f10 − arg` (`fld [edi+0x10]; fsub [ebp+8];
/// fst [edi+0x10]`). The stored value is the new timer; a following `fcomp 0.0; test ah,5; jp`
/// decides whether the timer crossed zero (the `[+0x14]` alpha path then runs). Returns the
/// decremented timer as f32 (the value written to `[edi+0x10]`). Same `field − arg` shape as the
/// `[+0x14]` decrement, distinct field/site.
pub fn fade_timer_dec(f10: f32, arg: f32) -> f32 {
    (f64::from(f10) - f64::from(arg)) as f32 // fld [edi+0x10]; fsub [ebp+8]; fst [edi+0x10] → f32
}

// ===========================================================================================
// (5) 0x7863b0 — level-fraction → libm transcendental (the InsertChild/level-bias setter).
// ===========================================================================================

/// `0x7863b0`'s owned level-fraction (`0x78642e`..`0x786447`): given the thumb-position getter
/// result `getter = vt[0x20]()` (stored f32 at `[ebp+8]`) and the level helper's return
/// `divisor = 0x7727b0()` (LIVE st0):
/// * `frac = getter / divisor + 0.5`  (`fst [ebp+8]; fdivr [ebp+8]; fadd [0x803078]=0.5`).
///
/// `fdivr [ebp+8]` = `[ebp+8] / st0` = `getter_f32 / divisor_live` — the getter enters as a
/// reloaded f32 (it was `fstp [ebp+8]`'d at `0x78642e`), the divisor stays LIVE PC_53. The
/// `fadd 0.5` uses the **f64** constant `[0x803078]` and the result is `fstp qword [esp]` → **f64**
/// (NO narrowing — pushed to the CRT libm `0x73ff3f`, then `__ftol`, both calls OUTSIDE the body).
/// Returns the f64 `frac` the transcendental receives.
pub fn level_fraction(getter: f32, divisor: f32) -> f64 {
    let half = f64::from_bits(HALF_BITS_F64); // 0x803078 = 0.5 (f64)
    (f64::from(getter) / f64::from(divisor)) + half // fdivr; fadd qword 0.5; fstp qword (f64)
}

// ===========================================================================================
// (6) 0x786490 — per-child negate (the remove/reflow setter, two fchs sites).
// ===========================================================================================

/// `0x786490`'s per-child negate (`0x7864d8`..`0x7864eb` and the twin `0x786529`..`0x786537`): the
/// child's `cy = [child+4]` is reloaded and negated to feed the layout setter `0x767c70`
/// (`fld [edi+edx+0x4]; fchs; fstp [esp]`). Both sites are the identical sign-flip shape (one in the
/// reflow loop, one in the tail). Returns the exact negation of `cy` as f32.
pub fn negate_child(cy: f32) -> f32 {
    (-f64::from(cy)) as f32 // fld [child+4]; fchs; fstp [esp]
}
