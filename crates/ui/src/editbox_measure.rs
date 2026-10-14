//! Reimplements the arithmetic of the edit-box measuring functions
//! `CSimpleEditBox::GetIndexAtPoint` (`0x77d0d0`), `0x77d4d0`, `0x77da80` and `0x77de70`. The x87
//! runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the
//! tests in `tests/editbox_measure.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact). The `[esi+0x7c]` scale, the `[+0x3e8]`/
// `[+0x3f0]` insets, and the piece-array pixel values are RUNTIME state, supplied by the difftest.
// ===========================================================================================

/// 0.5 (`DAT_007ffa24`), the centre-of-piece half factor.
pub const HALF_BITS: u32 = 0x3f00_0000;
/// 0.25 (`DAT_008029b0`), the re-window fraction when the cursor fell before the window.
pub const QUARTER_BITS: u32 = 0x3e80_0000;
/// 0.75 (`DAT_008012cc`), the re-window fraction when the cursor fell after the window.
pub const THREE_QUARTER_BITS: u32 = 0x3f40_0000;
/// 1024.0 (`DAT_007ffd68`), the value-changed-signal device→logical broadcast scale.
pub const SCALE1024_BITS: u32 = 0x4480_0000;
/// 0.0 (`DAT_007ffd74`), the clamp/settle pivot.
pub const ZERO_BITS: u32 = 0x0000_0000;
/// norm-ε = 2⁻²² (`DAT_008029d4`), the 0x77de70 change-detect epsilon.
pub const EPS_BITS: u32 = 0x3480_0000; // 2.384185791015625e-07

#[inline]
fn half() -> f64 {
    f64::from(f32::from_bits(HALF_BITS))
}
#[inline]
fn quarter() -> f64 {
    f64::from(f32::from_bits(QUARTER_BITS))
}
#[inline]
fn three_quarter() -> f64 {
    f64::from(f32::from_bits(THREE_QUARTER_BITS))
}
#[inline]
fn scale1024() -> f64 {
    f64::from(f32::from_bits(SCALE1024_BITS))
}
#[inline]
fn eps() -> f64 {
    f64::from(f32::from_bits(EPS_BITS))
}

// ===========================================================================================
// Shared shapes — the two-operand f32 products/sums recurring across the family.
// ===========================================================================================

/// The generic `g · [esi+0x7c]` scale leg (0x77d143/0x77d156, 0x77daf0/0x77db03, 0x77deb5/0x77dec8):
/// a getter result `g` (`0x7727b0` return, PC_53 st0) times the per-node scale `[esi+0x7c]`,
/// `fstp`'d → f32 (the value handed to the `__ftol` converter `0x766750`). `g` is the live st0 the
/// getter pushes; here it enters as f32 (the difftest's thunk pushes an f32 cell).
pub fn scale_leg(g: f32, scale: f32) -> f32 {
    (f64::from(g) * f64::from(scale)) as f32
}

/// The full head transform of 0x77da80/0x77de70 (`0x77daf0`..`0x77db33` / `0x77deb5`..`0x77def2`):
/// `out = (ftol(g0·scale) + ftol(g1·scale)) / scale`, where `ftol(x)` is the converter `0x766750`'s
/// f32 result (an eps-gated identity-or-trunc — diffed as the converter's own output, supplied here
/// as `c0`/`c1`). The first converted leg `c0` is `fstp`'d to a slot (f32), the second leg `c1` is
/// live st0; `c1 + c0` folds at PC_53 (live + reloaded-f32), then `÷ scale` at PC_53, `fstp` → f32.
pub fn head_combine(c0: f32, c1: f32, scale: f32) -> f32 {
    let sum = f64::from(c1) + f64::from(c0); // fadd [slot]: live c1 + reloaded-f32 c0
    (sum / f64::from(scale)) as f32 // fdiv [+0x7c]; fstp → f32
}

/// 0x77d0d0's head (`0x77d143`..`0x77d164`): `out = ftol(g0·scale) ; combined = ftol(g1·scale) +
/// out`, both legs `fstp`'d → f32. Unlike 0x77da80/70 there is NO `÷scale` here — the combined sum
/// is `fstp`'d → f32 directly. `c0`/`c1` are the converter results.
pub fn head_add(c0: f32, c1: f32) -> f32 {
    (f64::from(c1) + f64::from(c0)) as f32 // fadd [-0x8]; fstp [-0x4] → f32
}

// ===========================================================================================
// (1) 0x77d0d0 — multi-region scrollbar layout.
// ===========================================================================================

/// 0x77d0d0's clamp loop (`0x77d17c`..`0x77d1a6`): `acc = ([-0x10] − [-0x18]) − ([+0xc] − [-0x18])`
/// (`fld [-0x10]; fsub [-0x18]; fld [+0xc]; fsub [-0x18]; fsubp st(1),st`), then while `acc >= step`
/// (`step = [-0x4]`) and the loop counter is below the limit, `acc -= step` (`fsub [-0x4]`). The
/// `fcom [-0x4]; test ah,0x41; jne/je` predicate: `fcom(acc, step)` sets C0 iff `acc < step`. mask
/// 0x41 = {C0,C3}; the first `jne 0x77d1a6` (ZF: masked-AH != 0) is taken (exit, no subtract) iff
/// `acc < step` OR `acc == step` OR unordered; the in-loop `je 0x77d194` (ZF: masked-AH == 0)
/// continues subtracting iff `acc > step` ordered-strict. So it subtracts `step` while `acc > step`
/// (ordered), up to `limit` times. Returns the residual `acc` (PC_53 f64; the binary leaves it on
/// st0, then `fstp st(0)` discards it — it is consumed only via the `fcom`; we expose the value).
pub fn clamp_loop(a10: f32, a18: f32, c0c: f32, step: f32, limit: i32) -> f64 {
    let stepv = f64::from(step);
    // acc = ([-0x10] − [-0x18]) − ([+0xc] − [-0x18]) — fsubp st(1),st = st(1) − st(0).
    let mut acc = (f64::from(a10) - f64::from(a18)) - (f64::from(c0c) - f64::from(a18));
    let mut i = 0i32;
    // subtract while acc > step (ordered-strict) AND i < limit (unsigned cmp edi<ecx).
    while i < limit && acc > stepv {
        acc -= stepv;
        i += 1;
    }
    acc
}

/// 0x77d0d0's case-2 block (`0x77d1d3`..`0x77d1f1`): a per-piece centre recompute. With
/// `e = [-0xc]`, `f = [-0x14]`:
/// * `mid = (e − f) · 0.5` (`fld [-0xc]; fsub [-0x14]; fsubrp` — NOTE `fsubrp st(1),st` after a
///   getter push `g` in st0 means `g − (e−f)`? — see below; then `fmul 0.5`).
///
/// Disasm: `call 0x772ae0` (g→st0); `fld [-0xc]; fsub [-0x14]` (st0 = e−f, st1 = g);
/// `fsubrp st(1),st` (st1 = (e−f) − g, pop → st0 = (e−f)−g); `fmul 0.5` (·0.5);
/// `fld [-0x14]; fadd st,st(1)` (newF = f + half, st1 = half); `fstp [-0x14]`;
/// `fld [-0xc]; fsub st,st(1)` (newE = e − half); `fstp [-0xc]`; `fstp st(0)`.
/// So `half = ((e − f) − g)·0.5`, `newF = f + half`, `newE = e − half`. `g` is the `0x772ae0`
/// pixel→logical getter (live PC_53 st0). Returns `(newE, newF)` as f32 (each `fstp`'d → f32).
pub fn case2_recenter(e: f32, f: f32, g: f32) -> (f32, f32) {
    let ev = f64::from(e);
    let fv = f64::from(f);
    let half = ((ev - fv) - f64::from(g)) * half(); // (e−f)−g, ·0.5
    let new_f = (fv + half) as f32; // fld [-0x14]; fadd st,st(1); fstp [-0x14]
    let new_e = (ev - half) as f32; // fld [-0xc]; fsub st,st(1); fstp [-0xc]
    (new_e, new_f)
}

/// 0x77d0d0's tail offset+clamp (`0x77d1f3`..`0x77d233`): `pos = [+8] − [-0x14]`
/// (`fld [+8]; fsub [-0x14]`), `fst [+8]` (store, st0 LIVE), `fcomp 0.0; test ah,5; jp`.
/// `fcomp(pos, 0.0)` sets C0 iff `pos < 0`. mask 5 = {C0,C2}; `jp` (PF=1 ⟺ EVEN) taken iff
/// `pos >= 0` / unordered (even) → early-out (returns the clamp `pos = max(pos, 0)`-style 0 store);
/// NOT taken iff `pos < 0` (1 bit, odd) → continues to the second clamp `fld [+8]; fcomp [-0xc];
/// test ah,0x41; jne`: if `pos >= [-0xc]` (`jne` not-taken path falls through) it reloads `[-0xc]`
/// into `[+8]`. We expose the primary offset `pos = [+8] − [-0x14]` (the f32 `fst`'d to `[+8]`),
/// which is the owned arithmetic; the clamp selects are integer `mov`s past the `fcom`.
pub fn tail_offset(arg8: f32, f14: f32) -> f32 {
    (f64::from(arg8) - f64::from(f14)) as f32 // fld [+8]; fsub [-0x14]; fst [+8] → f32
}

/// 0x77d0d0's case-A reciprocal (`0x77d256`..`0x77d27f`, `[+0x120]&7 == 4`): with `e = [-0xc]`,
/// `f = [-0x14]`, `a8 = [+8]`, and the per-node scale `sc = [ecx+0x7c]`:
/// `q = ((e − f) − a8) / sc` (`fld [-0xc]; fsub [-0x14]; fsub [+8]; fld [ecx+0x7c]; fxch;
/// fdiv st,st(1)`), `fstp [esp]` → f32 (the float arg to the converter `0x772c30`). Returns the
/// f32 quotient.
pub fn case_a_recip(e: f32, f: f32, a8: f32, sc: f32) -> f32 {
    let num = (f64::from(e) - f64::from(f)) - f64::from(a8); // fld [-0xc]; fsub [-0x14]; fsub [+8]
    (num / f64::from(sc)) as f32 // fdiv st,st(1); fstp [esp] → f32
}

/// 0x77d0d0's case-B reciprocal (`0x77d2bf`..`0x77d2de`, `[+0x120]&7 ∈ {1,2}`): `q = [+8] / sc`
/// (`fld [ecx+0x7c]; fld [+8]; fdiv st,st(1)`), `fstp [esp]` → f32 (the float arg to `0x772bc0`).
/// `fdiv st,st(1)` = st0 / st1 = `[+8] / sc`. Returns the f32 quotient.
pub fn case_b_recip(a8: f32, sc: f32) -> f32 {
    (f64::from(a8) / f64::from(sc)) as f32
}

// ===========================================================================================
// (2) 0x77d4d0 — the EditBox relayout.
// ===========================================================================================

/// 0x77d4d0's re-window width (`0x77d7d4`..`0x77d7e8`), the `N` it passes to `0x77d320`: the
/// FontString's width `v = [ebp−0xc]` (`0x77d4e6 call [eax+0x1c]`) times `0.25` when the cursor fell
/// before the display window (`0x77d7c2 jl`, `before = true`) or `0.75` when it fell after
/// (`0x77d7d2 jge`); `fstp`'d → f32. Skipped for a password box (`0x77d769`).
pub fn rewindow_width(v: f32, before: bool) -> f32 {
    let k = if before { quarter() } else { three_quarter() };
    (f64::from(v) * k) as f32
}

/// 0x77d4d0's multi-line height (`0x77d893`..`0x77d8a8`): `h_box = (top + bottom) + g`
/// (`fld [+0x3f0]; fadd [+0x3e8]; fadd st,st(1)`), `top`/`bottom` the text insets and `g` the live
/// st0 — the FontString's measured text height `0x7729b0`, or its line height `0x7727b0(1)` when that
/// is exactly 0.0. `fstp [esp]` → f32, the argument of the BOX's own `SetHeight` (`[E+0x24]` vtable
/// `0x81c8e0` slot `+0x18` = `0x7683f0`, `0x77d8ad`). Reached only for a multi-line box.
pub fn multiline_height(top: f32, bottom: f32, g: f32) -> f32 {
    let s = f64::from(top) + f64::from(bottom); // fld [+0x3f0]; fadd [+0x3e8]
    (s + f64::from(g)) as f32 // fadd st,st(1)=g; fstp [esp] → f32
}

// ===========================================================================================
// (3) 0x77da80 — per-piece scrollbar layout (compass shape).
// ===========================================================================================

/// 0x77da80's compass-piece loop step (`0x77db38`..`0x77db52`): `acc = [-0x8]`, then while the
/// piece index is below the limit, `acc -= [-0x14]` per iteration (`fld [-0x8]; ...; fsub [-0x14];
/// ...; fstp [-0x8]`). `[-0x14]` is the per-piece logical width = `head_combine` output ÷ pieces
/// (the loaded `div [+0x7c]` result, reloaded f32). Returns the residual `acc` (the accumulated
/// per-piece offset). `start`/`step` are `[-0x8]`/`[-0x14]`; `n` the iteration count (`edi < eax`).
pub fn piece_loop_sub(start: f32, step: f32, n: i32) -> f32 {
    let mut acc = f64::from(start); // fld [-0x8]
    for _ in 0..n.max(0) {
        acc -= f64::from(step); // fsub [-0x14]
    }
    acc as f32 // fstp [-0x8] → f32
}

/// 0x77da80's centre-of-span (`0x77dc65`..`0x77dc7b`, `[+0x120]&7 == 2`): a span endpoint pixel
/// converted via `0x772ae0` (g, live st0) is halved and subtracted from a stored offset:
/// `out = [-0x4] − (g·0.5)` (`fmul 0.5; fsubr [-0x4]`). `fsubr` = `[-0x4] − (g·0.5)`. `fstp [-0x4]`
/// → f32. `stored` = `[-0x4]`, `g` = the converter result.
pub fn half_subr(stored: f32, g: f32) -> f32 {
    let h = f64::from(g) * half(); // fmul [0x7ffa24]
    (f64::from(stored) - h) as f32 // fsubr [-0x4]; fstp [-0x4] → f32
}

/// 0x77da80's negate (`0x77dbae`): a converter result (`0x772ae0`, live st0) is `fchs`'d and
/// `fstp`'d → f32 (`[-0x4]`), the negated per-piece offset. Exact sign flip.
pub fn neg(g: f32) -> f32 {
    (-f64::from(g)) as f32
}

/// 0x77da80's value-changed-signal broadcast (`0x77dda5`..`0x77de06`): four device→logical products
/// pushed to the signal `0x7026f0`. For each of the four stored offsets `v ∈ {[-0x14], [-0x10],
/// [-0x8], [-0x4]}`: `arg = (1024.0 · sc) · v` where `sc` is a per-call scale from `0x41ad70` (a
/// reciprocal-of-device-scale getter, live st0). Disasm per leg: `call 0x41ad70` (sc→st0);
/// `fmul [0x7ffd68]=1024` (sc·1024); `fmul [v]` ((sc·1024)·v); `fstp [esp]` (→ `0x41ae40`, an f32→
/// f64 promote pushed as qword). So `arg = (sc · 1024) · v`. The `·1024` is on the LIVE sc (PC_53),
/// then `·v` on the LIVE product. Returns the f32-equivalent product fed to the promote.
///
/// (The `0x41ad70`/`0x41ae40` pair is a device-pixel→logical-unit scale getter+promote; `sc` is its
/// reciprocal-scale return, supplied by the difftest's thunk. The owned arithmetic is the two
/// `fmul`s; the promote is `0x41ae40`'s. `fstp [esp]` stores the PRODUCT as f32 before the promote.)
pub fn signal_scale(sc: f32, v: f32) -> f32 {
    let p = f64::from(sc) * scale1024(); // fmul [0x7ffd68]=1024
    (p * f64::from(v)) as f32 // fmul [v]; fstp [esp] → f32
}

// ===========================================================================================
// (4) 0x77de70 — per-piece scrollbar layout (single-piece) + eps change-detect tail.
// ===========================================================================================

/// 0x77de70's single-piece loop step (`0x77def7`..`0x77df0f`): identical shape to
/// [`piece_loop_sub`] but on slots `[-0x4]`/`[-0x10]`: `acc = [-0x4]`, then `acc -= [-0x10]` per
/// iteration. Returns the residual `acc` → f32.
pub fn piece_loop_sub_2(start: f32, step: f32, n: i32) -> f32 {
    let mut acc = f64::from(start); // fld [-0x4]
    for _ in 0..n.max(0) {
        acc -= f64::from(step); // fsub [-0x10]
    }
    acc as f32 // fstp [-0x4] → f32
}

/// 0x77de70's centre-of-span (`0x77e06f`..`0x77e085`, `[+0x120]&7 == 2`): identical to
/// [`half_subr`] but stored slot is `[+0xc]`: `out = [+0xc] − (g·0.5)` (`fmul 0.5; fsubr [+0xc]`).
pub fn half_subr_2(stored: f32, g: f32) -> f32 {
    let h = f64::from(g) * half();
    (f64::from(stored) - h) as f32
}

/// 0x77de70's negate (`0x77dfbd`): same as [`neg`] on slot `[+0xc]`. Exact sign flip.
pub fn neg_2(g: f32) -> f32 {
    (-f64::from(g)) as f32
}

/// 0x77de70's eps change-detect tail (`0x77e189`..`0x77e199`): after a final getter+resolve leaves
/// the residual in `[-0x10]`, `fld [-0x10]; fabs; fcomp eps(2⁻²²); fnstsw; test ah,5; jnp`.
/// `fcomp(|d|, eps)` sets C0 iff `|d| < eps`. mask 5 = {C0,C2}; `jnp` (PF=0 ⟺ ODD) taken iff
/// `|d| < eps` (C0=1, 1 bit, odd) → jumps to `0x77e1b3` (the "settled, flag=0" path). NOT taken
/// (`|d| >= eps` / `== eps` / NaN, even) → falls through to `0x77e19b` (flag=1, "changed"). So the
/// control "changed" (flag=1) iff `|d| >= eps` ordered (or unordered). Returns `true` ⇒ `|d| < eps`
/// (false on NaN) ⇒ the binary takes the SETTLED path (flag cleared). `d` = `[-0x10]`.
pub fn settled_lt_eps(d: f32) -> bool {
    let m = f64::from(d).abs(); // fld [-0x10]; fabs
    !m.is_nan() && m < eps() // fcomp eps; jnp taken (settled) iff |d| < eps ordered
}
