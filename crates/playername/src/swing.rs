//! `0x6c6560`'s arithmetic — the **melee swing-trail per-frame emit cadence**
//! (`SWING::Step`, the `__AUSWING__` ring sub-system of PlayerName.cpp).
//!
//! Each frame the swing-trail wants to emit a number of interpolated ring segments proportional to
//! how much real time elapsed since the last evaluation. The slice `[0x6c65c6, 0x6c6606)`
//! computes that count:
//!
//! ```text
//!   6c65c6: db 45 08             fild  dword [ebp+0x8]        ; elapsed = now_ms - swing[+0x614] (i32) -> st0
//!   6c65c9: 89 be 14 06 00 00    mov   [esi+0x614], edi       ; swing[+0x614] = now_ms (store side-effect)
//!   6c65cf: 0f b6 be 0b 06 00 00 movzx edi, byte [esi+0x60b]  ; segcount (u8) -> edi
//!   6c65d6: 89 7d 0c             mov   [ebp+0xc], edi          ; segcount spilled as i32
//!   6c65d9: d8 0d 78 12 81 00    fmul  dword [0x811278]        ; * (1/300 as f32, widened to f64)
//!   6c65df: da 4d 0c             fimul dword [ebp+0xc]          ; * segcount (i32, exact)
//!   6c65e2: d8 15 74 fd 7f 00    fcom  dword [0x7ffd74]         ; compare vs 0.0 (sets flags; no math)
//!   6c65e8: df e0                fnstsw ax
//!   6c65ea: e8 c1 3c d4 ff       call  0x40a2b0                 ; __ftol(st0) -> eax (truncate toward 0)
//!   6c65ef: 3c 01                cmp   al, 0x1
//!   6c65f1: 88 45 08             mov   [ebp+0x8], al
//!   6c65f4: 73 04                jae   0x6c65fa                 ;  al >= 1 ? keep : clamp
//!   6c65f6: c6 45 08 01          mov   byte [ebp+0x8], 0x1      ;  al = max(al, 1)   (BYTE clamp value)
//!   6c65fa: 0f b6 4d 08          movzx ecx, byte [ebp+0x8]      ; divisor = clamp (u8 -> u32)
//!   6c65fe: 8b c7                mov   eax, edi                 ; eax = segcount
//!   6c6600: 99                   cdq                            ; sign-extend (segcount is u8 -> non-negative)
//!   6c6601: f7 f9                idiv  ecx                      ; eax = segcount / clamp   (signed)
//!   6c6603: 83 f8 01             cmp   eax, 0x1                 ; (slice ends here; eax = steps)
//!   6c6606: 7f 0b                jg    0x6c6613                 ; steps > 1 ? emit : (caller returns 0)
//! ```
//!
//! math: `steps = segcount / max( ftol(elapsed · (1/300) · segcount), 1 )`, where:
//! - `1/300` is the **f32** `0x3b5a740e` (`0.0033333334…`, nearest f32 to 1/300) at `0x811278`,
//!   widened exactly to PC_53 f64 by the `fmul dword`.
//! - The x87 chain runs at PC_53 (FPCW=0x027F, the harness invariant): `fild` of an i32 is exact,
//!   `fmul dword` widens the f32 operand exactly, `fimul dword` multiplies by the i32 exactly.
//! - `__ftol` (`0x40a2b0`) truncates toward zero (`(p.trunc() as i64) as i32`); its low byte `al`
//!   is then clamped to `max(.,1)` (the `cmp al,1; jae; mov byte 1`).
//! - The final `idiv` is a signed divide; `segcount` is a `u8` (`0..=255`, so non-negative), and
//!   the clamp divisor is `1..=255`, so the divide can never trap.
//!
//! The `fcom dword [0x7ffd74]=0.0` only sets `fnstsw`'s flags for a sign check the `__ftol` thunk
//! does not consume here; it performs no arithmetic. The slice observes `eax` (= `steps`) at
//! `0x6c6606`. `swing[+0x614] = now_ms` is a store side effect (does not feed `eax`). `elapsed`
//! and `segcount` are the only arithmetic inputs.
#![allow(clippy::float_cmp)]

/// `0x811278` (`.rdata`) — `1/300` as the **f32** `0x3b5a740e` (`0.0033333334…`).
pub const RECIP_300_F32_BITS: u32 = 0x3b5a_740e;

#[inline]
fn recip_300() -> f64 {
    f64::from(f32::from_bits(RECIP_300_F32_BITS)) // fmul dword: f32 operand widened exactly to f64
}

/// `0x40a2b0` `__ftol`: truncate the (PC_53 f64) accumulator toward zero into an `i32`.
#[inline]
fn ftol(p: f64) -> i32 {
    benilla_re_cmath::x87::ftol(p)
}

/// `0x6c6560` swing step-count: the per-frame emit cadence.
///
/// `elapsed` is the **frame delta** `now_ms − swing[+0x614]` (the i32 already in `[ebp+8]` at the
/// slice entry; the caller's subtraction at `0x6c65c1`, which then stamps `[+0x614] = now` at
/// `0x6c65c9`) — **not** the time since the swing began; feeding the time since the start makes the
/// trail terminate after one frame. `segcount` is `swing[+0x60b]` (the segment-count byte, `u8`). Returns
/// `steps`, the signed `eax` at `0x6c6606` — `segcount / max(ftol(elapsed·(1/300)·segcount), 1)`.
/// The caller emits the trail only when `steps > 1` (else returns 0); this fn returns raw `steps`.
pub fn swing_step_count(elapsed: i32, segcount: u8) -> i32 {
    // fild [elapsed]; fmul [1/300 f32]; fimul [segcount i32]  (PC_53 f64 chain)
    let prod = f64::from(elapsed) * recip_300() * f64::from(i32::from(segcount));
    // __ftol -> eax (truncate toward zero); `al` (its low byte) clamped to max(.,1).
    let ftol_lo = ftol(prod) as u8; // `al` is the low byte of the __ftol result
    let clamp = ftol_lo.max(1); // cmp al,1 / jae / mov byte 1  ->  al = max(al, 1)
                                // movzx ecx, clamp; eax = segcount; cdq; idiv ecx  (signed divide, both operands non-negative)
    i32::from(segcount) / i32::from(clamp)
}

/// `0x7ffa24` (`.rdata`) — the half-extent scale `0.5` (the **f32** `0x3f000000`); the layout tail
/// halves the measured glyph-run width/height. (Same `_DAT_007ffa24` used by the clip-quad and
/// damage-glyph builders — the px↔half basis.)
pub const HALF_F32_BITS: u32 = 0x3f00_0000;

#[inline]
fn half() -> f64 {
    f64::from(f32::from_bits(HALF_F32_BITS)) // fmul dword: f32 0.5 widened exactly to f64
}

/// `0x6c81a0` `string_measure_layout` — the **FP tail** at `[0x6c828f, 0x6c82b4)`.
///
/// `0x6c81a0` is mostly a delegated gx-font measure (`0x44d340`, `0x64a6f0`, `0x5c2050`/`0x5c2070`,
/// `0x5c1c30`, `0x5c1e60` — NOT transcribed). Its **only FP-arith** is the closing tail: the
/// gx layout fills `[ebp-0xc]` (run width `w`) and `[ebp-0x10]` (run height `h`) as f32; the tail
/// halves each into the result struct (`this[+0xc] = w·0.5`, `this[+0x10] = h·0.5`):
///
/// ```text
///   6c828f: d9 45 f4             fld   dword [ebp-0xc]     ; w (f32, widened to PC_53 f64)
///   6c8292: d8 0d 24 fa 7f 00    fmul  dword [0x7ffa24]    ; * 0.5 (f32, widened)
///   6c8299: d9 5d e8             fstp  dword [ebp-0x18]     ; tmp_w = (w·0.5) as f32
///   6c829c: d9 45 f0             fld   dword [ebp-0x10]     ; h (f32, widened)
///   6c82a2: d8 0d 24 fa 7f 00    fmul  dword [0x7ffa24]    ; * 0.5
///   6c82a8: 89 56 0c             mov   [esi+0xc], edx       ; this[+0xc] = tmp_w
///   6c82ab: d9 5d ec             fstp  dword [ebp-0x14]     ; tmp_h = (h·0.5) as f32
///   6c82b1: 89 46 10             mov   [esi+0x10], eax      ; this[+0x10] = tmp_h
///   6c82b4: 5e                   pop   esi                  ; (tail observed here)
/// ```
///
/// Each operand is an f32 widened exactly to f64 by `fmul dword`, the constant `0.5` (`0x3f000000`
/// at `0x7ffa24`) likewise, and the product is rounded back to f32 at the `fstp dword`. Returns
/// `(w·0.5 as f32, h·0.5 as f32)` — the pair stored to `this[+0xc]`/`this[+0x10]`.
pub fn string_measure_layout(w: f32, h: f32) -> (f32, f32) {
    let out_w = (f64::from(w) * half()) as f32; // fld [w]; fmul [0.5]; fstp
    let out_h = (f64::from(h) * half()) as f32; // fld [h]; fmul [0.5]; fstp
    (out_w, out_h)
}
