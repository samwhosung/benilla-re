//! Reimplements the arithmetic of `CSimpleFrame::LoadXML` (`0x769820`, the alpha quantise) and
//! `CSimpleFrame::SetParentScale` (`0x76ac50`, the near-equal guard). The x87 runs at 53-bit
//! precision, so each chain is `f64` with `as f32` at the `fstp dword` stores; the tests in
//! `tests/frame_xml_load.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact; cited by VA).
// ===========================================================================================

/// `[0x7ffe58]` = `255.0` as a **dword** `f32` (`0x437f0000`), the SetAlpha quantize multiplier
/// (`fmul dword [0x7ffe58]` @ 0x769a76).
pub const C255_F32_BITS: u32 = 0x437f_0000;
/// `[0x7ff9d8]` = `1.0` (`0x3f800000`) and `[0x7ffd74]` = `0.0` (`0x00000000`), the SetAlpha
/// clamp-high / clamp-low operands (used in the `fcomp` compares and selected by the two `fld`s —
/// NOT arithmetic; documented for context).
pub const CLAMP_HI_F32_BITS: u32 = 0x3f80_0000;
pub const CLAMP_LO_F32_BITS: u32 = 0x0000_0000;
/// `[0x8029d4]` = `2.384185791015625e-07` (`0x34800000` = `2^-22`), the 0x76ac50 near-equal epsilon
/// (a `fcomp` compare operand — NOT arithmetic; documented for context).
pub const EPS_F32_BITS: u32 = 0x3480_0000;

// ===========================================================================================
// (1) 0x769820 SetAlpha — clamp-to-[0,1] → `· 255.0` → `__ftol` truncate (NO +0.5 bias).
// ===========================================================================================
//
// The parsed `alpha` value `v` (`0x64aaa0`, on st0) is clamped onto st0 by three `fcomp` compares:
//   call 0x64aaa0; fcomp [1.0]; test ah,5; jp ...      ; (v < 1.0)?
//   call 0x64aaa0; fcomp [0.0]; test ah,0x41; jne ...  ; (v > 0.0)?
//   call 0x64aaa0; fcomp [1.0]; test ah,5; jp ...
//   <one of>:  call 0x64aaa0      (st0 = raw v)
//              fld [1.0]          (st0 = 1.0)
//              fld [0.0]          (st0 = 0.0)
//   fmul [0x7ffe58]=255.0         ; OWNED — the clamped value · 255
//   call __ftol                   ; truncate toward zero → the color byte
// So `byte = (i32)(clamp(v, 0, 1) · 255)` truncate. The clamp itself is compares (NOT arith); only
// the `· 255` is owned. (`__ftol` is the delegated CRT truncate, run natively in the difftest.)

/// 0x769820's owned SetAlpha quantize: the clamped-to-`[0,1]` value `v` (a PC_53 `f64` — either the
/// raw `0x64aaa0` parse on st0, or the clamp constant `1.0`/`0.0` loaded as f32 then widened)
/// `· 255.0`, then the CRT `__ftol` truncates toward zero; the low byte is the stored alpha byte.
/// `v` is the clamped value. Returns the alpha byte (`0..=255` over the clamped domain).
///
/// NB **no `+0.5`** round bias here (SetFogColor's quantize has one; SetAlpha does not) — this is a
/// plain truncate of `v·255`.
pub fn alpha_quantize(v: f64) -> u8 {
    let p = v * k(C255_F32_BITS); // fmul dword [0x7ffe58]=255.0  (PC_53 f64)
    (p.trunc() as i64 as i32) as u8 // __ftol truncate toward zero, low byte
}

// ===========================================================================================
// (2) 0x76ac50 — guarded near-equal scalar setter: `|new − field|` (fsubr + fabs).
// ===========================================================================================
//
//   test dl,dl; jne set                    ; if flag != 0 -> unconditional set (skip guard)
//   fld dword [ecx+0xbc]                    ; st0 = field (f32 widened to f64)
//   fsubr dword [ebp+0x8]                   ; OWNED — st0 = new − field   (fsubr = mem − st0)
//   fabs                                    ; OWNED — st0 = |new − field|
//   fcomp dword [0x8029d4]=eps; fnstsw;     ; compare vs eps (NOT arith)
//   test ah,5; jp set                       ; jp taken unless (|delta| < eps): see below
//   xor al,al; ret 8                        ; early-return AL=0 (no change)
//  set:
//   mov [ecx+0xbc], new; call [this+0x8c]   ; store + delegated notify (orchestration)
//
// The `test ah,5; jp` reads PF of (C0|C2): |delta| < eps -> C0=1 -> AH&5=1 (1 bit, odd) -> PF=0 ->
// `jp` NOT taken -> the `xor al,al` early-return fires. |delta| >= eps (or ==, or NaN) -> PF=1 ->
// `jp` taken -> the set path. So the guard early-returns exactly when `|new − field| < eps`.

/// 0x76ac50's owned subtract+magnitude: `|new − field|` computed op-for-op as the binary does it —
/// `fld [field]; fsubr [new]; fabs`. The `fsubr` is **reverse** subtract (`mem − st0` =
/// `new − field`, NOT `field − new`); both `new` and `field` are `f32` memory operands widened to
/// the PC_53 `f64` chain. `new` is `[ebp+0x8]`, `field` is `[this+0xbc]`. Returns `|new − field|`
/// (the value `fabs` produces on st0 immediately before the `fcomp` vs eps).
pub fn delta_abs(new: f32, field: f32) -> f64 {
    let d = f64::from(new) - f64::from(field); // fld [field]; fsubr [new]  (new − field)
    d.abs() // fabs
}

/// The 0x76ac50 near-equal guard outcome: does `|new − field| < eps` (the no-change early-return)?
/// The `fcomp` vs `eps` is a compare; this models the `test ah,5; jp` disposition. `eps_bits` is the
/// binary's epsilon (`0x34800000` = `2^-22`). Returns `true` iff the guard fires (early-return,
/// AL = 0 / no setter call). NB on `flag != 0` the guard is skipped entirely (always sets) — that
/// is the caller's branch, not part of this owned-FP slice.
pub fn near_equal(new: f32, field: f32, eps_bits: u32) -> bool {
    delta_abs(new, field) < k(eps_bits)
}
