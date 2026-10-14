//! Reimplements the arithmetic of the rectangle-change tests and fade functions `0x788310`,
//! `0x788460`, `0x78a320` and `0x78adb0`. The x87 runs at 53-bit precision, so each chain is
//! `f64` with `as f32` at the `fstp dword` stores; the tests in `tests/frame_rect_change.rs` diff
//! each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact).
// ===========================================================================================

/// `DAT_0080c5c8` = 1e-5 (`0x3727c5ac`) — the "did the rect move?" epsilon (0x788310 / 0x78a320).
pub const EPS_BITS: u32 = 0x3727_c5ac;
/// `DAT_007ffd74` = 0.0 (`0x00000000`) — the field-vs-zero pivot in 0x788460.
pub const ZERO_BITS: u32 = 0x0000_0000;
/// `DAT_007ffe58` = 255.0 (`0x437f0000`) — the byte-alpha scale in 0x788460.
pub const SCALE255_BITS: u32 = 0x437f_0000;

#[inline]
fn w(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

/// PF (even-parity) of the masked status-word low byte — the bit the `jp`/`jnp` after a
/// `test ah,MASK` actually tests. `jp` taken ⇔ this is `true`; `jnp` taken ⇔ this is `false`.
#[inline]
pub fn parity_even(ah: u8, mask: u8) -> bool {
    (ah & mask).count_ones().is_multiple_of(2)
}

/// "Is `v` NOT strictly less than `eps`?" — the exact outcome of the `fcomp eps; test ah,5; jp`
/// branch the rect-changed predicates use: `jp` is taken (= "changed") on `>`, `==`, AND the
/// unordered (NaN) case; only the strict-`<` ("clean") case falls through. `v >= eps` would
/// disagree on NaN (it yields `false`, but the binary treats an unordered compare as changed), so
/// the faithful test is "not (`v < eps`)", written via `partial_cmp` to make the NaN case explicit.
#[inline]
fn not_less(v: f64, eps: f64) -> bool {
    !matches!(v.partial_cmp(&eps), Some(core::cmp::Ordering::Less))
}

// ===========================================================================================
// (1) 0x788310 — "rect changed?" predicate, recomputing the 4 edges first.
// ===========================================================================================

/// The four object edges (`this = esi`) recomputed by 0x788310 from anchor factors `f0..f3`
/// (`[esi+0x380]`/`[esi+0x388]`/`[esi+0x37c]`/`[esi+0x384]`), a shared scale `s = [esi+0x7c]`, and
/// four offsets (`[esi+0x68]`/`[esi+0x70]`/`[esi+0x64]`/`[esi+0x6c]`). Stored, in disasm order, to
/// `[esi+0x370]`/`[esi+0x378]`/`[esi+0x36c]`/`[esi+0x374]`:
/// * `e370 = f0·s + off68`           (`fmul`,`fadd`)
/// * `e378 = off70 - f1·s`           (`fmul`,`fsubr`)
/// * `e36c = f2·s + off64`           (`fmul`,`fadd`)
/// * `e374 = off6c - f3·s`           (`fmul`,`fsubr`)
///
/// Each is an independent `fld;fmul;fadd|fsubr;fstp` (no register carry), so order is irrelevant to
/// the values. Returns `(e370, e378, e36c, e374)` as the stored f32s.
#[allow(clippy::too_many_arguments)]
pub fn recompute_edges(
    f0: f32,
    f1: f32,
    f2: f32,
    f3: f32,
    s: f32,
    off68: f32,
    off70: f32,
    off64: f32,
    off6c: f32,
) -> (f32, f32, f32, f32) {
    let sd = f64::from(s);
    let e370 = (f64::from(f0) * sd + f64::from(off68)) as f32; // fmul; fadd
    let e378 = (f64::from(off70) - f64::from(f1) * sd) as f32; // fmul; fsubr
    let e36c = (f64::from(f2) * sd + f64::from(off64)) as f32; // fmul; fadd
    let e374 = (f64::from(off6c) - f64::from(f3) * sd) as f32; // fmul; fsubr
    (e370, e378, e36c, e374)
}

/// `0x788310`'s X-axis "changed" magnitude (`0x788321`..`0x788392`): the OLD width
/// `old_dx = [esi+0x378] - [esi+0x370]` is computed FIRST and held LIVE on the x87 stack across the
/// whole 4-edge recompute, then the NEW width `new_dx = e378 - e370` is formed and
/// `|old_dx - new_dx|` taken (`fsub`, the `fsubp st(1),st`, `fabs`). The compare is the full PC_53
/// f64 magnitude vs `eps`. Returns that f64 `|Δx|`.
pub fn dx_magnitude(old_e378: f32, old_e370: f32, new_e378: f32, new_e370: f32) -> f64 {
    let old_dx = f64::from(old_e378) - f64::from(old_e370); // fld[378]; fsub[370]  (LIVE st0)
    let new_dx = f64::from(new_e378) - f64::from(new_e370); // fld[378]; fsub[370]  (the NEW edges)
    (old_dx - new_dx).abs() // fsubp st(1),st ; fabs
}

/// `0x788310`'s Y-axis "changed" magnitude (`0x7883a1`..`0x7883b0`): `old_dy = [ebp-4]` (spilled at
/// `0x788339` = `[esi+0x374]_old - [esi+0x36c]_old`); the NEW height `new_dy = e374 - e36c`; then
/// `|old_dy - new_dy|` via `fsubr [ebp-4]` (st0 = `old_dy - new_dy`), `fabs`. Returns f64 `|Δy|`.
pub fn dy_magnitude(old_dy: f32, new_e374: f32, new_e36c: f32) -> f64 {
    let new_dy = f64::from(new_e374) - f64::from(new_e36c); // fld[374]; fsub[36c]
    (f64::from(old_dy) - new_dy).abs() // fsubr [ebp-4] ; fabs
}

/// The full 0x788310 "did the rect move?" predicate, given the OLD edges (`o370/o378/o36c/o374`)
/// and the anchor inputs that recompute the NEW edges. The disasm: `jp 0x7883bf` after the X
/// compare jumps straight to "do update" when PF=1 (`|Δx| >= eps`, `==`, or NaN); else falls to the
/// Y compare whose `jnp 0x78842c` jumps to "exit/clean" when PF=0 (`|Δy| < eps`); else updates. So
/// **changed = (`|Δx| >= eps`) OR (`|Δy| >= eps`)** (NaN counts as changed, exact-eps counts as
/// changed). Returns `(changed, new_edges)`.
#[allow(clippy::too_many_arguments)]
pub fn rect_changed_788310(
    o370: f32,
    o378: f32,
    o36c: f32,
    o374: f32,
    f0: f32,
    f1: f32,
    f2: f32,
    f3: f32,
    s: f32,
    off68: f32,
    off70: f32,
    off64: f32,
    off6c: f32,
) -> (bool, (f32, f32, f32, f32)) {
    // old_dy is the value `fstp`'d to [ebp-4] BEFORE the recompute, from the OLD 0x374/0x36c.
    let old_dy = (f64::from(o374) - f64::from(o36c)) as f32; // fld[374]; fsub[36c]; fstp[ebp-4]
    let (e370, e378, e36c, e374) = recompute_edges(f0, f1, f2, f3, s, off68, off70, off64, off6c);
    let dx = dx_magnitude(o378, o370, e378, e370);
    let dy = dy_magnitude(old_dy, e374, e36c);
    let eps = w(EPS_BITS);
    // `|Δ| >= eps` is "(NOT |Δ| < eps)"; a NaN |Δ| is unordered → not `< eps` → counts as changed,
    // matching the binary's parity branch (PF=1 on `>`,`==`,unordered for the X jp;
    // `|Δ| < eps` is the only "clean" outcome).
    let changed = not_less(dx, eps) || not_less(dy, eps);
    (changed, (e370, e378, e36c, e374))
}

// ===========================================================================================
// (2) 0x788460 — per-item fade/alpha loop step.
// ===========================================================================================

/// `0x788460`'s field-0xc decrement (`0x7884d1`..`0x7884d7`): `[esi+0xc] - step` stored back to
/// `[esi+0xc]` as f32 (the `fst` leaves st0 live for the subsequent `fcomp 0`, but the observable
/// is the stored f32). Returns the stored f32.
pub fn decrement_field_0c(field_0c: f32, step: f32) -> f32 {
    (f64::from(field_0c) - f64::from(step)) as f32 // fld [esi+0xc]; fsub [ebp+8]; fst [esi+0xc]
}

/// `0x788460`'s alpha leg (`0x788510`..`0x788558`, the `(field0x10 - step) >= 0` path): the live
/// difference `d = [esi+0x10] - step` is stored to `[esi+0x10]` as f32 (LEAVING st0 LIVE at PC_53),
/// then `al = (i32)trunc( d / [ebx+0x368] · 255.0 ) & 0xff` via `fdiv`,`fmul`,`__ftol` (truncate
/// toward zero). The division and ·255 operate on the **un-narrowed f64** `d`, not the stored f32.
/// Returns `(stored_f10, alpha_byte)`.
pub fn alpha_step(field_10: f32, step: f32, divisor: f32) -> (f32, u8) {
    let d = f64::from(field_10) - f64::from(step); // fld [esi+0x10]; fsub [ebp+8]  (LIVE st0)
    let stored = d as f32; // fst [esi+0x10]
    let alpha_f = d / f64::from(divisor) * w(SCALE255_BITS); // fdiv [ebx+0x368]; fmul [255.0]
    let alpha = benilla_re_cmath::x87::ftol(alpha_f); // __ftol = truncate toward zero
    (stored, (alpha & 0xff) as u8) // mov [esi+0x7], al
}

// ===========================================================================================
// (3) 0x78a320 — "rect changed?" predicate (no recompute).
// ===========================================================================================

/// `0x78a320`'s Y magnitude (`0x78a330`..`0x78a33e`):
/// `|([edi+0xc] - [edi+0x4]) - ([esi+0x4c] - [esi+0x44])|` via two `fsub`s, `fsubp st(1),st`,
/// `fabs`. Compared (full PC_53 f64) vs `eps`. `(a_hi,a_lo)` = `[edi+0xc]`/`[edi+0x4]`,
/// `(b_hi,b_lo)` = `[esi+0x4c]`/`[esi+0x44]`. Returns f64 `|Δy|`.
pub fn dy_magnitude_78a320(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> f64 {
    let da = f64::from(a_hi) - f64::from(a_lo); // fld[edi+0xc]; fsub[edi+0x4]
    let db = f64::from(b_hi) - f64::from(b_lo); // fld[esi+0x4c]; fsub[esi+0x44]
    (da - db).abs() // fsubp st(1),st ; fabs
}

/// `0x78a320`'s X magnitude (`0x78a34d`..`0x78a35e`):
/// `|([edi+0x8] - [edi]) - ([esi+0x48] - [esi+0x40])|`. Same shape. Returns f64 `|Δx|`.
pub fn dx_magnitude_78a320(a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32) -> f64 {
    let da = f64::from(a_hi) - f64::from(a_lo); // fld[edi+0x8]; fsub[edi]
    let db = f64::from(b_hi) - f64::from(b_lo); // fld[esi+0x48]; fsub[esi+0x40]
    (da - db).abs() // fsubp st(1),st ; fabs
}

/// The full 0x78a320 predicate. The disasm: `jp 0x78a369` after the Y compare jumps to "set dirty"
/// when PF=1 (`|Δy| >= eps`, `==`, NaN); else falls to the X compare whose `jnp 0x78a373` jumps to
/// the clean epilogue when PF=0 (`|Δx| < eps`); else sets dirty. So
/// **dirty = (`|Δy| >= eps`) OR (`|Δx| >= eps`)**. `(ay_hi,ay_lo,by_hi,by_lo)` feeds Y,
/// `(ax_hi,ax_lo,bx_hi,bx_lo)` feeds X. Returns whether the dirty flag `[esi+0x34c]` is set to 1.
#[allow(clippy::too_many_arguments)]
pub fn rect_changed_78a320(
    ay_hi: f32,
    ay_lo: f32,
    by_hi: f32,
    by_lo: f32,
    ax_hi: f32,
    ax_lo: f32,
    bx_hi: f32,
    bx_lo: f32,
) -> bool {
    let dy = dy_magnitude_78a320(ay_hi, ay_lo, by_hi, by_lo);
    let dx = dx_magnitude_78a320(ax_hi, ax_lo, bx_hi, bx_lo);
    let eps = w(EPS_BITS);
    not_less(dy, eps) || not_less(dx, eps)
}

// ===========================================================================================
// (4) 0x78adb0 — re-attach widget; the ONE owned arith op is a sign-negate.
// ===========================================================================================

/// `0x78adb0`'s owned arithmetic (`0x78aec0`): the float returned in `st0` by `0x766750` is negated
/// (`fchs`) and `fstp`'d to `[edi+0x34c]` as f32. `fchs` is an exact sign-bit flip (no rounding),
/// so the f32 round-trip is exact. `queried` is the `0x766750` result. Returns `-queried` as f32.
pub fn negate_stp(queried: f32) -> f32 {
    (-f64::from(queried)) as f32 // fchs ; fstp dword [edi+0x34c]
}
