//! Reimplements the arithmetic of the slider functions `0x7898f0`, `0x789930`, `0x789a60`,
//! `0x789aa0` and `CSimpleSlider::OnMouseMove` (`0x789ba0`). The x87 runs at 53-bit precision, so
//! each chain is `f64` with `as f32` at the `fstp dword` stores; the tests in `tests/slider.rs`
//! diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// 0.0 (`DAT_007ffd74`), the quantize-step `step==0` sentinel and the round-direction pivot.
pub const ZERO_BITS: u32 = 0x0000_0000;
/// 0.5 (`DAT_007ffa24`), the round-to-nearest bias and the thumb half-extent factor.
pub const HALF_BITS: u32 = 0x3f00_0000;
/// norm-ε = 2⁻²² (`DAT_008029d4`), the degenerate-range guard of `0x789aa0`.
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07

#[inline]
fn half() -> f64 {
    f64::from(f32::from_bits(HALF_BITS))
}
#[inline]
fn eps() -> f64 {
    f64::from(f32::from_bits(EPS_BITS))
}

// ===========================================================================================
// (1) 0x7898f0 — span = end − start (the SetMinMax-style range writer).
// ===========================================================================================

/// `0x7898f0`'s only owned FP-arith (`0x7898f9`..`0x78990d`): `field0x31c = param3 − param2`
/// (`fld [ebp+0xc]; fsub [ebp+8]; fstp [ecx+0x31c]`). `param2`/`start` is stored verbatim to
/// `[+0x318]`; the difference (the span/range) is `fstp`'d → f32. Inputs `start = [ebp+8]`,
/// `end = [ebp+0xc]`. Returns the f32 span.
pub fn span(start: f32, end: f32) -> f32 {
    (f64::from(end) - f64::from(start)) as f32
}

// ===========================================================================================
// (2) 0x789930 — clamp-low + clamp-high select + round-to-step quantize (the SetValue kernel).
// ===========================================================================================

/// `0x789930`'s clamp+quantize (`0x789947`..`0x789a06`). Given the requested value `v = [ebp+8]`,
/// the range fields `min = [+0x318]`, `span = [+0x31c]`, `step = [+0x324]`:
///
/// 1. **clamp-low** (`0x789947`..`0x78995f`): `fld min; fcomp v; test ah,0x41; jne`. `fcomp(min,v)`
///    sets C0 iff `min < v`. mask 0x41 = {C0,C3}; `jne` (ZF: masked-AH != 0) taken iff `min < v` OR
///    `min == v` OR unordered → loads `v`; falls through (loads `min`) iff `min > v` (ordered
///    strict). Net `lo = max(min, v)` with NaN → v.
/// 2. `bound = span + min` (`0x789962 fld span; 0x789968 fadd min`) — the upper edge (LIVE).
/// 3. **clamp-high** (`0x78996e`..`0x78999c`): `fst [ebp-4]` spills `bound` to f32 (st0 LIVE),
///    `fcomp st(1)` compares `bound` vs `lo`, `test ah,5; jp`. `fcomp(bound, lo)` sets C0 iff
///    `bound < lo`. mask 5 = {C0,C2}; `jp` (PF=1 ⟺ EVEN set-bits) NOT taken iff `bound < lo`
///    (1 bit, odd) → reload `bound` from `[ebp-4]` (f32) → `hi = bound`. Taken (`bound >= lo` /
///    NaN, even) → recompute `clamp-low` → `hi = max(min, v)`. So `hi = bound_f32` iff `bound < lo`
///    ordered; else `hi = clamp-low`.
/// 4. **quantize** (`0x78999c`..`0x789a06`): only if `step != 0` (a `fcomp(step,0.0); test ah,0x44;
///    jnp` skip-test — `jnp` taken on ODD set-bits = `step==0`). `diff = hi − min`
///    (`0x7899af fsub [+0x318]`), then round-half-away-from-zero by `step`: `fld step; fmul 0.5`
///    (`= halfstep`, st0; `diff` is st1), and `fcom(diff, 0.0); test ah,0x41; jne`. `jne` taken iff
///    `diff < 0` / `== 0` / unordered → the round-DOWN path `fsubr st,st(1)` = `diff − halfstep`,
///    `/step`; falls through iff `diff > 0` ordered → the round-UP path `fadd st,st(1)` =
///    `halfstep + diff`, `/step`. `__ftol` (truncate toward zero) → i32 `n`, then `hi =
///    (f32)n · step + min` (`fild n; fmul step; fadd min`).
///
/// Returns the quantized/clamped value as f32 (the value that lands in `[+0x320]`). The store,
/// flag updates, and the `0x7026f0` value-changed signal are orchestration past this slice.
pub fn set_value(v: f32, min: f32, span: f32, step: f32) -> f32 {
    let minv = f64::from(min);
    let vv = f64::from(v);

    // clamp-low: lo = max(min, v) with NaN routing to v (the `jne` path loads v).
    let lo = clamp_low(vv, minv);

    // bound = span + min (LIVE), spilled to f32 at [ebp-4].
    let bound = f64::from(span) + minv;
    let bound_f32 = bound as f32; // fst [ebp-4]

    // clamp-high: hi = bound (reloaded f32) iff `bound < lo` ordered; else clamp-low again.
    let mut hi = if matches!(bound.partial_cmp(&lo), Some(core::cmp::Ordering::Less)) {
        f64::from(bound_f32) // fld [ebp-4] → f32 reload
    } else {
        clamp_low(vv, minv)
    };

    // quantize to step (only if step != 0.0 exactly).
    if f64::from(step) != f64::from(f32::from_bits(ZERO_BITS)) {
        let stepv = f64::from(step);
        let diff = hi - minv; // fsub [+0x318]
        let halfstep = stepv * half(); // fld step; fmul 0.5  (st0=halfstep, st1=diff)
                                       // fcom(diff,0.0); jne: diff > 0 ordered ⇒ round-up; diff <= 0 / NaN ⇒ round-down.
        let scaled = if diff > 0.0 {
            (halfstep + diff) / stepv // fadd st,st(1) = halfstep+diff; fdiv step
        } else {
            (diff - halfstep) / stepv // fsubr st,st(1) = diff−halfstep; fdiv step
        };
        let n = scaled.trunc() as i32; // __ftol = truncate toward zero
        hi = (f64::from(n) * stepv) + minv; // fild n; fmul step; fadd min
    }
    hi as f32
}

/// `0x789930`'s clamp-low select (`fld min; fcomp v; test ah,0x41; jne`): `max(min, v)` with NaN
/// routing to `v` (the binary's `jne`-loads-`v` branch on unordered). Returns the live f64.
fn clamp_low(v: f64, min: f64) -> f64 {
    // fcomp(min, v): falls through (uses `min`) iff `min > v` ordered-strict; else uses `v`.
    if matches!(min.partial_cmp(&v), Some(core::cmp::Ordering::Greater)) {
        min
    } else {
        v
    }
}

// ===========================================================================================
// (3) 0x789a60 — CSimpleSlider::SetValueStep: writes the step, then re-pushes (min, span+min).
// The setter's downstream effect (re-clamp / re-quantize / OnValueChanged) is NOT arithmetic this
// module owns; it is pinned by an executed probe in the difftest.
// ===========================================================================================

/// `0x789a60`'s only owned FP-arith (`0x789a75`..`0x789a84`): `arg = span + min`
/// (`fld [+0x31c]; fadd [+0x318]; fstp [esp+4]`), the upper-edge value re-pushed to `0x7898f0` by
/// the plain (NOT tail) `call` at `0x789a91`, under the `[+0x314] & 2` guard at `0x789a6c`.
/// `[+0x324]` (the step) is set from `param2` by an integer `mov` (not FP). Returns the f32 sum.
/// Note `0x7898f0` then recovers `span' = f32(arg - min)`, which is NOT the identity in f32.
pub fn range_plus_min(span: f32, min: f32) -> f32 {
    (f64::from(span) + f64::from(min)) as f32
}

// ===========================================================================================
// (4) 0x789aa0 — value → thumb-pixel OFFSET map (the OnUpdate thumb reposition). Two owned slices share the normalize prefix.
// ===========================================================================================

/// `0x789aa0`'s normalize prefix (`0x789adc`..`0x789b16`): from the value `value = [+0x320]`,
/// `min = [+0x318]`, `span = [+0x31c]`, compute the fraction of the value within the range:
/// * `range = (span + min) − min` (`fld span; fadd min; fsub min` — a PC_53 round-trip, NOT
///   algebraically simplified, since each op rounds at PC_53; transcribed faithfully as the chain).
/// * guard: `fld st(0); fabs; fcomp eps; test ah,5; jp`. `fcomp(|range|, eps)` sets C0 iff
///   `|range| < eps`. mask 5 = {C0,C2}; `jp` (PF=1 ⟺ EVEN) NOT taken iff `|range| < eps` (1 bit,
///   odd) → `frac = 0`; taken (`|range| >= eps` / `== eps` / unordered, even) → `frac = (value −
///   min) / range`. So `frac = (value − min)/range` iff `|range| >= eps` (ordered) or unordered,
///   else `0`.
///
/// **The quotient is `fstp`'d to the f32 slot `[ebp+8]` (`0x789b16`) and RELOADED as f32 by the
/// per-orientation multiply (`fmul [ebp+8]`), so `frac` enters that multiply as an f32, not the
/// live PC_53 quotient.** Returns the f32 `frac`. The `else` path stores `+0.0f` (an integer
/// `mov [ebp+8],0`). `range` stays live; `value − min` is the dividend; both fold at PC_53.
pub fn thumb_frac(value: f32, min: f32, span: f32) -> f32 {
    let minv = f64::from(min);
    let range = (f64::from(span) + minv) - minv; // fld span; fadd min; fsub min
    let a = range.abs();
    // |range| >= eps ordered, OR unordered (NaN) ⇒ divide (even-parity jp taken); else +0.0.
    if a.is_nan() || a >= eps() {
        ((f64::from(value) - minv) / range) as f32 // (value−min)/range; fstp [ebp+8] → f32
    } else {
        0.0
    }
}

/// `0x789aa0`'s horizontal thumb-offset slice (`0x789b21`..`0x789b4b`, `[+0x32c]==0`): given the per-node
/// scale `s = [+0x7c]`, the track endpoints `hi = [+0x70]`, `lo = [+0x68]`, the thumb position
/// getter result `g = vt[0x1c]()`, and the normalize `frac` ([`thumb_frac`]):
/// `out = ((hi − lo)/s − g) · frac`. The track term `(hi − lo)/s` is `fstp`'d → f32 (`[ebp-4]`)
/// then RELOADED (`fsubr [ebp-4]` = `[ebp-4] − g`), so it enters the subtract as f32 while `g`
/// stays LIVE PC_53. `frac` is the f32 from [`thumb_frac`] (reloaded via `fmul [ebp+8]`). `out` is
/// `fstp`'d → f32. Returns the f32 value-delta.
pub fn thumb_offset_horiz(s: f32, hi: f32, lo: f32, g: f32, frac: f32) -> f32 {
    let track = ((f64::from(hi) - f64::from(lo)) / f64::from(s)) as f32; // fstp [ebp-4] → f32
    let term = f64::from(track) - f64::from(g); // fsubr [ebp-4]: spilled f32 − live g
    (term * f64::from(frac)) as f32 // fmul [ebp+8]=frac(f32); fstp [ebp+8]
}

/// `0x789aa0`'s vertical thumb-offset slice (`0x789b59`..`0x789b77`, `[+0x32c]!=0`): identical shape but
/// the endpoints are `[+0x6c]`/`[+0x64]`, the getter is `vt[0x20]()`, and the result is NEGATED
/// (`fchs`): `out = −(((hi − lo)/s − g) · frac)`. Same live-vs-f32-reload track term; `frac` is the
/// f32 from [`thumb_frac`].
pub fn thumb_offset_vert(s: f32, hi: f32, lo: f32, g: f32, frac: f32) -> f32 {
    let track = ((f64::from(hi) - f64::from(lo)) / f64::from(s)) as f32; // fstp [ebp-4] → f32
    let term = f64::from(track) - f64::from(g); // fsubr [ebp-4]
    (-(term * f64::from(frac))) as f32 // fmul [ebp+8]=frac(f32); fchs; fstp [esp]
}

// ===========================================================================================
// (5) 0x789ba0 — cursor-pixel → value map (press/drag; the inverse of 0x789aa0).
// ===========================================================================================

/// `0x789ba0`'s horizontal numerator (`0x789bd4`..`0x789bfe`, `[+0x32c]==0`): given the track
/// endpoints `hi = [+0x70]`, `lo = [+0x68]`, the two thumb-getter results `g1 = vt[0x1c]()` (track
/// origin) and `g2 = vt[0x1c]()` (thumb extent), and the target pixel `p = [edi+0x24]`:
/// * `trk = hi − lo` (`fld [+0x70]; fsub [+0x68]`) `fstp`'d → f32 (`[ebp+8]`), reloaded.
/// * `den = trk_f32 − g1` (`fsubr [ebp+8]` = `[ebp+8] − g1`) `fstp`'d → f32 (`[ebp+8]`).
/// * `numer = p − (g2·0.5 + lo)` (`fmul 0.5; fadd [+0x68]; fsubr [edi+0x24]`).
///
/// Returns `(numer, den)`: `numer` is the live st0 fed into the tail, `den` is the f32 in
/// `[ebp+8]`. `g1`/`g2` stay LIVE PC_53; `trk` enters `den` as a reloaded f32.
pub fn pos_horiz(hi: f32, lo: f32, g1: f32, g2: f32, p: f32) -> (f64, f32) {
    let trk = (f64::from(hi) - f64::from(lo)) as f32; // fld; fsub; fstp [ebp+8] → f32
    let den = (f64::from(trk) - f64::from(g1)) as f32; // fsubr [ebp+8]; fstp [ebp+8] → f32
                                                       // numer = p − (g2·0.5 + lo): fmul 0.5; fadd [+0x68]; fsubr [edi+0x24].
    let numer = f64::from(p) - ((f64::from(g2) * half()) + f64::from(lo));
    (numer, den)
}

/// `0x789ba0`'s vertical numerator (`0x789c03`..`0x789c2d`, `[+0x32c]!=0`): endpoints `[+0x6c]`/
/// `[+0x64]`, getters `vt[0x20]()`, and:
/// * `trk = [+0x6c] − [+0x64]` `fstp`'d → f32, reloaded.
/// * `den = trk_f32 − g1` `fstp`'d → f32.
/// * `numer = ([+0x6c] − g2·0.5) − [edi+0x28]` (`fmul 0.5; fsubr [+0x6c]; fsub [edi+0x28]`).
///
/// `hi` here is `[+0x6c]` (reused as the `fsubr` minuend). Returns the live `numer` and f32 `den`.
pub fn pos_vert(hi: f32, lo: f32, g1: f32, g2: f32, p: f32) -> (f64, f32) {
    let trk = (f64::from(hi) - f64::from(lo)) as f32; // fld [+0x6c]; fsub [+0x64]; fstp [ebp+8]
    let den = (f64::from(trk) - f64::from(g1)) as f32; // fsubr [ebp+8]; fstp [ebp+8]
                                                       // numer = ([+0x6c] − g2·0.5) − p: fmul 0.5; fsubr [+0x6c]; fsub [edi+0x28].
    let numer = (f64::from(hi) - (f64::from(g2) * half())) - f64::from(p);
    (numer, den)
}

/// `0x789ba0`'s tail (`0x789c30`..`0x789c52`, BOTH orientations join here): given the `numer`
/// (live st0 from [`pos_horiz`]/[`pos_vert`]), the f32 `den` (`[ebp+8]`), and the range fields
/// `min = [+0x318]`, `span = [+0x31c]`:
/// `range = (span + min) − min` (PC_53 round-trip), then `value = range · (numer / den) + min`
/// (`fld span; fadd min; fsub min; fxch; fdiv [ebp+8]; fmulp; fadd [+0x318]`). The `fdiv` divides
/// `numer` (live st0) by the f32 `den`. Result `fstp`'d → f32 (the arg passed to `0x789930`'s
/// SetValue). Returns the f32 value.
pub fn pos_to_value(numer: f64, den: f32, min: f32, span: f32) -> f32 {
    let minv = f64::from(min);
    let range = (f64::from(span) + minv) - minv; // fld span; fadd min; fsub min
    let q = numer / f64::from(den); // fdiv [ebp+8]: live numer / f32 den
    ((range * q) + minv) as f32 // fmulp; fadd [+0x318]; fstp [esp]
}
