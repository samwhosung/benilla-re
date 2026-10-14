//! Reimplements the arithmetic of the movie and animated-texture functions `0x78c360`, `0x78cf30`
//! and `0x78d1a0`. The x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the
//! `fstp dword` stores; the tests in `tests/movieframe.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

#[inline]
fn k(bits: u32) -> f64 {
    f64::from(f32::from_bits(bits))
}

// ===========================================================================================
// Owned constants (read from `WoW.exe`, bit-exact; cited by VA).
// ===========================================================================================

/// `[0x801360]` (`.rdata`) = `0.0010000000474974513` as a **dword** `f32` (`0x3a83126f`), the
/// 0x78cf30 milliseconds→seconds time multiplier (`fmul dword [0x801360]`). Widened to f64.
pub const MS_TO_S_F32_BITS: u32 = 0x3a83_126f;
/// `[0x7ffa24]` (`.rdata`) = `0.5` as a **dword** `f32` (`0x3f000000`), the 0x78cf30 round-to-nearest
/// bias added before `__ftol` (`fadd dword [0x7ffa24]`). (Same `0.5` constant as `luabind_76e`.)
pub const HALF_F32_BITS: u32 = 0x3f00_0000;

// ===========================================================================================
// (1) 0x78c360 — one integer width/height ratio (fidiv ×1).
// ===========================================================================================
//
// On the chunk-parse path that records a stream's pixel aspect, the parser fills two table fields and
// the owned arithmetic at 0x78c5ae..0x78c5c9 is:
//   mov [ebp-0x38], [esi+ebx+0x18]   ; low dword of the qword numerator (a u32)
//   mov [ebp-0x34], eax              ; high dword = eax = 0 (the 0x64a4c0 return on this path)
//   fild qword [ebp-0x38]            ; numerator = (u64) low dword           (hi forced 0)
//   mov [ebp-0x40], [esi+ebx+0x14]   ; the i32 divisor
//   mov eax, [ebp-0x10]              ; the `this` store base
//   fidiv dword [ebp-0x40]           ; q = numerator / (i32 divisor)    OWNED arithmetic
//   fstp dword [eax+0x33c]           ; store q as f32 into [this+0x33c]
// The decomp reads `(float)*(uint*)(buf+0x20) / (float)*(int*)(buf+0x1c)` — uint num, int den.

/// `0x78c360`'s sole owned arithmetic (`fild qword [ebp-0x38]; fidiv dword [ebp-0x40]; fstp dword
/// [eax+0x33c]`): the **zero-extended u32** numerator divided by the **signed i32** divisor, stored as
/// f32 into `[this+0x33c]`. `num` is the `[esi+ebx+0x18]` u32 (the `fild`'d qword's low dword, high
/// dword 0); `den` is the `[esi+ebx+0x14]` i32 divisor. The division is the masked-IEEE f64 divide
/// (FPCW=0x027F masks `#Z`, so `den==0` yields ±inf / NaN exactly as the binary), then narrowed to
/// f32 by the `fstp dword`. Returns the f32 stored at `[this+0x33c]`.
pub fn aspect_ratio(num: u32, den: i32) -> f32 {
    // `fild qword` of a value with high-dword 0 = the u32 widened to a non-negative i64 = f64::from(u32).
    if num == 0 && den == 0 {
        // 0/0 raises x87 #IA (invalid); masked (FPCW=0x027F masks #IA), `fidiv` yields the x87
        // real-indefinite QNaN — the NEGATIVE indefinite `0xFFF8…` (f64), which `fstp dword` narrows
        // to the f32 `0xFFC00000`. Rust's `0.0/0.0` is the POSITIVE QNaN (`0x7FC00000`), so the
        // op-for-op match forces the x87 indefinite here. (Out of the function's real domain — the
        // divisor is a frame dimension — but transcribed bit-exact, never fudged.)
        return f32::from_bits(0xffc0_0000);
    }
    (f64::from(num) / f64::from(den)) as f32 // fidiv dword (i32 den); fstp dword
}

// ===========================================================================================
// (2) 0x78cf30 — two byte-identical time→frame-index quantize blocks (fmul ×4, fadd ×2).
// ===========================================================================================
//
// Both blocks (0x78cf9c.. and 0x78cfd8..) are byte-for-byte identical:
//   mov [ebp-0x8], eax ; mov [ebp-0x4], 0      ; the int time value, hi dword forced 0 (a u32)
//   fild qword [ebp-0x8]                        ; t = (u64) the time value           (hi 0)
//   fmul dword [esi+0x33c]                      ; · rate (the f32 field set by 0x78c360's fidiv)
//   fmul dword [0x801360]=0.001(f32)            ; · 0.001  (ms → s)
//   fadd dword [0x7ffa24]=0.5(f32)              ; + 0.5    (round-to-nearest bias)
//   call __ftol (0x40a2b0)                      ; n = trunc(... toward zero)
// The two blocks differ only in the `eax`/`edi` register bookkeeping around them; the FP is identical.

/// One 0x78cf30 time→frame-index quantize block (`fild qword [ebp-0x8]; fmul dword [esi+0x33c];
/// fmul dword [0x801360]=0.001; fadd dword [0x7ffa24]=0.5; call __ftol`): the **zero-extended u32**
/// time value `t` (the `fild`'d qword, high dword 0) `· rate · 0.001 + 0.5`, then `__ftol` truncates
/// toward zero. `t` is the `[ebp-0x8]` time value (a u32); `rate` is the `[esi+0x33c]` f32 field
/// (set by 0x78c360 / `aspect_ratio`). Returns the truncated `i32` (the `__ftol` result). Both the
/// 0x78cf9c and 0x78cfd8 blocks compute exactly this (identical bytes).
pub fn time_to_index(t: u32, rate: f32) -> i32 {
    // `fild qword` (hi dword 0) = u32 widened exactly to f64.
    let p = f64::from(t) * f64::from(rate); // fmul dword [esi+0x33c]   (rate widens f32->f64)
    let p = p * k(MS_TO_S_F32_BITS); // fmul dword [0x801360]=0.001
    let p = p + k(HALF_F32_BITS); // fadd dword [0x7ffa24]=0.5
    p.trunc() as i64 as i32 // __ftol = truncate toward zero
}

// ===========================================================================================
// (3) 0x78d1a0 — two glyph-quad coord scales (fmul ×2). the gx getter is not transcribed.
// ===========================================================================================
//
// The gx scalar getter `0x41ad70` (call; fstp dword [ebp-0x4]) yields the scale `[ebp-0x4]` (an f32);
// that BOUNDARY row is NOT transcribed here. The two owned products (0x78d478.. / 0x78d4a6..) are:
//   fld dword [ebp-0x4]                         ; scale (the gx-getter f32)
//   fmul dword [eax+0xc]  (then +0x8)           ; · glyph-quad field   OWNED arithmetic
//   fst  dword [ebp-0x48] (then -0x30)          ; store product as f32 (no pop, st0 live)
//   fstp dword [ebp-0x3c] (then -0x24)          ; store SAME product as f32 (pop)
// No arithmetic consumes the live st0 between the `fst`/`fstp`, so both slots get the identical f32.

/// One 0x78d1a0 glyph-quad coord scale (`fld dword [ebp-0x4]; fmul dword [eax+off]; fst/fstp dword`):
/// the gx-getter scale `· field`, narrowed to f32. `scale` is the `[ebp-0x4]` f32 (the gx getter
/// `0x41ad70`'s result — a BOUNDARY value handed in, NOT computed here); `field` is the glyph-quad
/// geometry-table f32 (`[eax+0xc]` for the first product, `[eax+0x8]` for the second). The PC_53 f64
/// product is stored to two f32 slots, both narrowing the SAME value. Returns the f32 stored.
pub fn quad_scale(scale: f32, field: f32) -> f32 {
    (f64::from(scale) * f64::from(field)) as f32 // fmul dword [eax+off]; fst/fstp dword
}
