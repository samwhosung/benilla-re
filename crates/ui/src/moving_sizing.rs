//! Reimplements the arithmetic of `CSimpleTop::UpdateMovingOrSizing` (`0x7655b0`), the pixel
//! snaps `PixelAlignX` (`0x7666a0`) and `PixelAlignY` (`0x766750`), and
//! `CLayoutFrame::PinToParentOffset` (`0x768430`). The x87 runs at 53-bit precision, so each
//! chain is `f64` with `as f32` at the `fstp dword` stores; the tests in `tests/moving_sizing.rs`
//! diff each against the exe.

#![allow(clippy::float_cmp)]

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact). The ÷0.8/÷0.6 leaf divisors live in `.data`
// (`0x832a44`/`0x832a48`); the difftest reads them from the mapped image and asserts these values.
// ===========================================================================================

/// `[0x808120]` = 0.9999899864196777 (`0x3f7fff58`), the round-up bias added before `__ftol` in
/// 0x7666a0/0x766750 when the `ratio·delta` product is strictly positive.
pub const BIAS_BITS: u32 = 0x3f7f_ff58;
/// `[0x7ff9d8]` = 1.0 (`0x3f800000`), the reciprocal numerator of 0x768430's `1/scale`.
pub const ONE_BITS: u32 = 0x3f80_0000;
/// `[0x832a44]` = 0.8 (`0x3f4ccccd`), the 0x7666a0 (X) unit divisor (`0x41ae40` `fdiv`).
pub const DIV08_BITS: u32 = 0x3f4c_cccd;
/// `[0x832a48]` = 0.6 (`0x3f19999a`), the 0x766750 (Y) unit divisor (`0x41ae50` `fdiv`).
pub const DIV06_BITS: u32 = 0x3f19_999a;

// ===========================================================================================
// (1) 0x7655b0 — two coordinate deltas (fsub ×2) + two global re-centres (fadd ×2).
// ===========================================================================================
//
// `eax = [ebp+8]` is a coordinate struct (`[eax+0x24]` = A, `[eax+0x28]` = B); `esi = ecx` is the
// `this` whose `[esi+0xd08]`/`[esi+0xd0c]` are two running coordinate globals `g0`/`g1`.
//   fld [eax+0x24]; fld [eax+0x28]; fxch          ; st0 = A, st1 = B
//   fsub [esi+0xd08]; fstp [ebp-0x8]              ; [ebp-0x8] = (A − g0) as f32   = dx
//   fsub [esi+0xd0c]; fstp [ebp-0x4]              ; [ebp-0x4] = (B − g1) as f32   = dy
//   (compares dx/dy vs 0 → conditional setter call `0x76a660` — orchestration, no owned arith)
//   fld [ebp-0x8]; fadd [esi+0xd08]; fstp [esi+0xd08]   ; g0 += dx → g0' (= A, numerically)
//   fld [ebp-0x4]; fadd [esi+0xd0c]; fstp [esi+0xd0c]   ; g1 += dy → g1'

/// One coordinate delta: `(a − g) as f32` (`fld [coord]; fsub [global]; fstp [m32]`). `a` is the
/// struct coordinate, `g` the running global. Returns the stored f32 delta.
pub fn delta(a: f32, g: f32) -> f32 {
    (f64::from(a) - f64::from(g)) as f32
}

/// One global re-centre: `(delta_f32 + g) as f32` (`fld [ebp−d]; fadd [global]; fstp [global]`).
/// The `delta` arg is the f32 value previously stored by [`delta`] (reloaded from the stack as f32,
/// widened), added to the same-as-before global `g`. Returns the new f32 global.
pub fn recenter(delta_f32: f32, g: f32) -> f32 {
    (f64::from(delta_f32) + f64::from(g)) as f32
}

/// The full owned arithmetic of 0x7655b0: from struct coords `(a, b)` and globals `(g0, g1)`,
/// produce the two stored deltas `(dx, dy)` and the two re-centred globals `(g0', g1')`.
pub fn deltas_recenter(a: f32, b: f32, g0: f32, g1: f32) -> (f32, f32, f32, f32) {
    let dx = delta(a, g0);
    let dy = delta(b, g1);
    (dx, dy, recenter(dx, g0), recenter(dy, g1))
}

// ===========================================================================================
// (2,3) 0x7666a0 (X, ÷0.8) and 0x766750 (Y, ÷0.6) — near-zero guards + ratio·delta → round → /delta.
// ===========================================================================================
//
// After the `0x58a250` call fills a 4-`f32` buffer `buf[0..4]` (`[ebp-0x14]`..`[ebp-0x8]`):
//   fld buf[hi]; fsub buf[lo]; fst [ebp-0x4]   ; d = buf[hi] − buf[lo]  → [ebp-0x4] as f32 (live)
//   fabs; fcomp [eps]                          ; near-zero guard on |d|  (owned fabs, compare-only)
//   call 0x41ae40/0x41ae50                     ; r = arg / divisor   (leaf helper, st0)
//   fmul [ebp-0x4]                             ; p = r · d           (d reloaded as f32)
//   fcom 0; jne skip                           ; bias iff p > 0
//   fadd [bias]                                ; p += 0.99999  (round-up toward +inf for p>0)
//   call __ftol                                ; n = trunc(p toward zero)
//   fild n; fdiv [ebp-0x4]                     ; q = n / d
//   fstp [esp]; call 0x41ae60/0x41ae70         ; (leaf scales q by divisor — NOT owned)
// 0x7666a0: hi=3, lo=1, divisor=0.8.   0x766750: hi=2, lo=0, divisor=0.6.

/// The owned `fsub` of 0x7666a0/0x766750: `d = (buf_hi − buf_lo) as f32`, the f32 the `fst`
/// stores to `[ebp-0x4]` and the `fmul`/`fdiv` later reload.
pub fn buf_delta(buf_hi: f32, buf_lo: f32) -> f32 {
    (f64::from(buf_hi) - f64::from(buf_lo)) as f32
}

/// `|x| < eps`? — the owned `fabs; fcomp [eps]` near-zero guard (eps = `[0x8029d4]` = 2^-22). The
/// `fcomp` here is `<` (the `jp` after `test ah,5` takes the early-return when unordered or below).
/// `eps_bits` is the binary's epsilon (`0x34800000`). Returns whether the guard fires (early-out).
pub fn near_zero(x: f32, eps_bits: u32) -> bool {
    f64::from(x).abs() < k(eps_bits)
}

/// The x87 status-word `C3:C2:C0` condition bits a `fabs; fcomp [eps]` leaves (the prologue
/// near-zero guard of 0x7666a0/0x766750), packed as the binary's `fnstsw` writes them into AX:
/// C0 = bit 8 (`|x| < eps`), C2 = bit 10 (unordered — never, for finite inputs), C3 = bit 14
/// (`|x| == eps`). `eps_bits` is the binary's epsilon. Returns the 16-bit AX value.
pub fn prologue_flags(x: f32, eps_bits: u32) -> u16 {
    let a = f64::from(x).abs(); // fabs
    let e = k(eps_bits);
    let c0 = u16::from(a < e) << 8;
    let c3 = u16::from(a == e) << 14;
    c0 | c3
}

/// 0x7666a0/0x766750's owned `r·d → bias → __ftol` (the `fmul [ebp-0x4]; fcom 0; (fadd [bias]);
/// call __ftol`). `r = arg / divisor` is the leaf helper's quotient (`fld [arg]; fdiv [divisor]`,
/// a PC_53 f64 in st0); `d` is the reloaded f32 delta. The bias (`0x808120` ≈ 1.0) is added iff the
/// product is **strictly positive** (`jne` skips the `fadd` when `p < 0` OR `p == 0`); `__ftol`
/// then truncates toward zero. `arg` is `[ebp+8]`, `cbits` the divisor (0.8 or 0.6), `d` the f32
/// delta. Returns the truncated `i32`.
pub fn ratio_round(arg: f32, cbits: u32, d: f32) -> i32 {
    let r = f64::from(arg) / k(cbits); // leaf `fld [arg]; fdiv [divisor]`, st0 (f64)
    let p = r * f64::from(d); // owned `fmul [ebp-0x4]`
    let biased = if p > 0.0 { p + k(BIAS_BITS) } else { p }; // owned conditional `fadd [bias]`
    biased.trunc() as i32 // __ftol = truncate toward zero
}

/// 0x7666a0/0x766750's owned final `fdiv`: `(n / d) as f32` (`fild n; fdiv [ebp-0x4]; fstp [esp]`).
/// `n` is the [`ratio_round`] result (reloaded via `fild` as f64), `d` the f32 delta. The `fstp
/// [esp]` rounds to f32 (this f32 is then handed to the ÷-leaf — not owned). Returns the f32 stored.
pub fn final_div(n: i32, d: f32) -> f32 {
    (f64::from(n) / f64::from(d)) as f32
}

// ===========================================================================================
// (4) 0x768430 — edge-rect builder: reciprocal scale reused live ×2, then two clamped converts.
// ===========================================================================================
//
// `esi = this`; `[esi+0x58]` = scale. Block A (`0x7684a4`..`0x7684d5`):
//   fld [one]; fdiv [esi+0x58]                  ; recip = 1.0 / scale   (LIVE st1, reused ×2)
//   fld [ebp-0x8]; fadd [ebp+0x18]; fsubr [ebp-0x18]; fchs; fmul st,st(1); fstp [esp+4]
//                                               ; pa = ((u + p18) − e18) · recip → [esp+4] as f32
//   fld [ebp-0xc]; fadd [ebp+0x14]; fsubr [ebp-0x1c]; fchs; fmul st,st(1); fstp [esp]
//                                               ; pb = ((v + p14) − e1c) · recip → [esp]   as f32
//   (the two `fstp` feed the layout setter `0x767c70` — orchestration past the owned arith)
// Block B (`0x7684f1`..`0x7684fd`, guarded by `[ebp+0xc] != 0`):
//   fld [ebp-0x4]; fsub [ebp-0xc]; fdiv [esi+0x58]; fstp [ebp+0xc]   ; ([ebp-0x4]−[ebp-0xc]) / scale
// Block C (`0x768518`..`0x768524`, guarded by `[ebp+0x10] != 0`):
//   fld [ebp-0x8]; fsub [ebp-0x10]; fdiv [esi+0x58]; fstp [ebp+0x10] ; ([ebp-0x8]−[ebp-0x10]) / scale

/// 0x768430's reciprocal scale: `recip = 1.0 / scale`, kept as f64 (`fld [one]; fdiv [esi+0x58]`,
/// LIVE on st0 and reused for both Block-A products WITHOUT an intervening f32 round). `scale` is
/// `[esi+0x58]`. Returns the f64 reciprocal.
pub fn recip_scale(scale: f32) -> f64 {
    k(ONE_BITS) / f64::from(scale)
}

/// One Block-A edge product of 0x768430, transcribed op-for-op so the **sign of zero** matches the
/// binary: `fadd [offset]; fsubr [edge]; fchs; fmul st,st(1)`. `fsubr` computes `edge − (coord +
/// offset)`; `fchs` negates it (so `+0.0` becomes `−0.0`, which `· recip` keeps signed); the live
/// `recip` then multiplies. Numerically `recip · ((coord + offset) − edge)`, but the explicit
/// `fsubr`-then-`fchs` reproduces the negative-zero a direct subtract would lose. `recip` is the
/// live [`recip_scale`] (f64, un-rounded between the two products). Returns the f32 stored.
pub fn edge_product(coord: f32, offset: f32, edge: f32, recip: f64) -> f32 {
    let s = f64::from(coord) + f64::from(offset); // fadd [offset]
    let r = f64::from(edge) - s; // fsubr [edge]  (edge − s)
    let neg = -r; // fchs
    (neg * recip) as f32 // fmul st,st(1) on the live recip
}

/// Both Block-A products of 0x768430 with the shared LIVE reciprocal: `pa` (→ `[esp+4]`) and
/// `pb` (→ `[esp]`). `(u, p18, e18)` are `[ebp-0x8]`/`[ebp+0x18]`/`[ebp-0x18]`; `(v, p14, e1c)`
/// are `[ebp-0xc]`/`[ebp+0x14]`/`[ebp-0x1c]`; `scale` is `[esi+0x58]`. Returns `(pa, pb)`.
pub fn edge_products(
    u: f32,
    p18: f32,
    e18: f32,
    v: f32,
    p14: f32,
    e1c: f32,
    scale: f32,
) -> (f32, f32) {
    let recip = recip_scale(scale);
    (
        edge_product(u, p18, e18, recip),
        edge_product(v, p14, e1c, recip),
    )
}

/// One Block-B/Block-C clamped convert of 0x768430: `((hi − lo) / scale) as f32`
/// (`fld [hi]; fsub [lo]; fdiv [esi+0x58]; fstp [m32]`). Returns the f32 stored.
pub fn local_convert(hi: f32, lo: f32, scale: f32) -> f32 {
    ((f64::from(hi) - f64::from(lo)) / f64::from(scale)) as f32
}
