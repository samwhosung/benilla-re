//! Reimplements the arithmetic of the attribute readers `XmlReadDimension` (`0x6f1eb0`),
//! `0x6f1d60`, `0x6f20a0` and `0x6f23d0`: the scale division of a metric and the packing of a
//! colour. The x87 runs at 53-bit precision, so each chain is `f64` with `as f32` at the `fstp
//! dword` stores; the tests in `tests/framexml_attrs.rs` diff each against the exe.

#![allow(clippy::float_cmp)]

// ===========================================================================================
// Constants — read from WoW.exe (bit-exact; cited by VA).
// ===========================================================================================

/// `0x7ffd68` (`.rdata`) = `1024.0`, the metric scale-divide denominator factor
/// (`fmul [0x7ffd68]`). The metric chain divides `atof(value)` by `(scale · 1024.0)`.
pub const SCALE1024_BITS: u32 = 0x4480_0000; // 1024.0
/// `0x832a4c` (`.data`, runtime-init UI scale; default `1.0`) = the scale global loaded by the
/// helper `0x41ad70` (`fld [0x832a4c]; ret`) at the head of the metric chain. `scale` is that runtime
/// factor (the difftest feeds chosen values directly into the diffed slice's operands).
pub const SCALE_GLOBAL_DEFAULT_BITS: u32 = 0x3f80_0000; // 1.0 (default)
/// `0x7ffd74` (`.rdata`) = `0.0`, the colour clamp-low bound (`fcomp [0x7ffd74]`).
pub const CLAMP_LO_BITS: u32 = 0x0000_0000; // 0.0
/// `0x7ff9d8` (`.rdata`) = `1.0`, the colour clamp-high bound (`fcomp [0x7ff9d8]`).
pub const CLAMP_HI_BITS: u32 = 0x3f80_0000; // 1.0
/// `0x7ffe58` (`.rdata`) = `255.0`, the colour-channel quantize multiplier (`fmul [0x7ffe58]`).
pub const C255_BITS: u32 = 0x437f_0000; // 255.0
/// `0x7ffa24` (`.rdata`) = `0.5`, the round-half bias before the truncating `__ftol` (`fadd`).
pub const HALF_BITS: u32 = 0x3f00_0000; // 0.5

#[inline]
fn scale1024() -> f64 {
    f64::from(f32::from_bits(SCALE1024_BITS))
}
#[inline]
fn c255() -> f64 {
    f64::from(f32::from_bits(C255_BITS))
}
#[inline]
fn half() -> f64 {
    f64::from(f32::from_bits(HALF_BITS))
}

// ===========================================================================================
// (1) the metric scale-divide kernel — shared by 0x6f1d60 (2×), 0x6f1eb0 (4×), 0x6f20a0 (8×)
// ===========================================================================================

/// The per-attribute owned slice of the metric parsers (`0x6f1d99`..`0x6f1da0` and every clone):
/// `q = atof(value) / (scale · 1024.0)`.
///
/// Disasm per attribute (e.g. `0x6f1d8c`..`0x6f1da3`):
/// ```text
/// call 0x64aaa0          ; st0 = atof(value)   (DELEGATED: CRT atof)
/// fstp dword [ebp-8]     ; save atof value as f32
/// call 0x41ad70          ; st0 = [0x832a4c]    (DELEGATED: scale-global load)
/// fmul dword [0x7ffd68]  ; st0 = scale · 1024.0          <-- OWNED
/// fdivr dword [ebp-8]    ; st0 = atof_value / (scale·1024.0)   <-- OWNED (mem / st0)
/// fstp dword [esp]       ; push as the logical_to_device arg, f32
/// call 0x41ae60          ; · screen-scale  (DELEGATED: logical_to_device)
/// ```
/// The owned ops are the `fmul [1024.0]` then `fdivr` — `fdivr [m]` computes `[m] / st0`, i.e.
/// `atof_value / (scale·1024.0)`. The `atof` value enters as an `f32` (`fstp` then reload), and the
/// quotient is `fstp`'d to `f32` before the delegated `logical_to_device`. So both the divide's
/// memory operand and its result narrow at `f32`; the PC_53 divide itself is f64.
///
/// `atof_value` = the parsed attribute string value (the `f32` in `[ebp-8]`); `scale` = the runtime
/// UI scale global `0x832a4c` (default `1.0`).
pub fn metric_scale_divide(atof_value: f32, scale: f32) -> f32 {
    let denom = f64::from(scale) * scale1024(); // fmul [0x7ffd68]
    (f64::from(atof_value) / denom) as f32 // fdivr [ebp-8] then fstp dword
}

// ===========================================================================================
// (2) the colour parser — 0x6f23d0 (r,g,b,a → packed 0xAARRGGBB)
// ===========================================================================================

/// The colour clamp **outcome** on the `atof`'d channel value, narrowed to a stored `f32` (all four
/// channels go `fstp dword [ebp-N]` or `mov [ebp-N], imm` then `fld [ebp-N]`, so the kept value is
/// `f32` for every channel — unlike `luabind_79a`'s live-`f64` alpha):
/// `value < 0 → 0.0`; `0 <= value < 1 → value`; `value >= 1` **or NaN** `→ 1.0`.
///
/// Branch ladder (`0x6f2414`..`0x6f2459`, per channel):
/// ```text
/// fcomp [0x7ff9d8]=1.0 ; test ah,5 ; jp  0x..34   ; PF=parity(C0,C2): v<1 ⇒ 1 bit ⇒ NOT taken
/// fcomp [0x7ffd74]=0.0 ; test ah,0x41 ; jne 0x..5b ; (reached iff v<1) masked≠0 ⇒ v<0 OR v==0 ⇒ store 0
/// fcomp [0x7ff9d8]=1.0 ; test ah,5 ; jp  0x..52   ; (reached iff v>=1 / NaN / v∈(0,1)) v<1 ⇒ store v; else 1.0
/// ```
/// Net: `v<0 → 0.0`, `v∈[0,1) → v`, `v>=1 or NaN → 1.0`. (The `v==0` case routes to the 0.0 store via
/// `jne` on `{C0,C3}` — value already 0, so storing 0.0 is identity; probed at the boundary.)
pub fn clamp01_unit(value: f32) -> f32 {
    let v = f64::from(value);
    let kept = if v < 0.0 {
        0.0 // mov [slot], 0
    } else if v < 1.0 {
        v // fstp dword [slot]
    } else {
        1.0 // mov [slot], 0x3f800000 (also NaN)
    };
    kept as f32
}

/// One channel quantize (`fld c; fmul [0x7ffe58]=255.0; fadd [0x7ffa24]=0.5; __ftol`): `c·255 + 0.5`
/// at f64 (PC_53), then `__ftol` (`0x40a2b0`) rounds-toward-zero (`fistp` after `or ah,0xc` sets
/// RC=truncate) = truncate. Faithful to `(i32)(c·255.0 + 0.5)`; the low byte is the channel. `c` is
/// the clamped channel `f32` reloaded from its slot.
pub fn quantize_channel(c: f32) -> i32 {
    let p = f64::from(c) * c255() + half(); // fmul 255.0; fadd 0.5  (OWNED)
    benilla_re_cmath::x87::ftol(p) // __ftol: round-toward-zero fistp qword, low dword
}

/// Pack four `[0,1]` channels into the u32 `0xAARRGGBB` exactly as `0x6f23d0` assembles it
/// (`0x6f25a3`..`0x6f261b`): each channel is clamped (→ `f32` slot), reloaded, quantized, and the
/// quantize **order** is `a, r, g, b` while the byte **placement** is the standard ARGB:
/// `ch = a` (into bits 8-15 first), then `cl = r`, `shl 8`, `or g`, `shl 8`, `or b` — yielding
/// `a<<24 | r<<16 | g<<8 | b`. Inputs are the raw `atof`'d channel values (`r`,`g`,`b` from the
/// matched attrs; `a` defaults to `1.0` when the `"a"` attr is absent — `[ebp-0x10]` seeded
/// `0x3f800000` at `0x6f23f7`).
pub fn pack_rgba(r: f32, g: f32, b: f32, a: f32) -> u32 {
    let q = |v: f32| -> u32 { quantize_channel(clamp01_unit(v)) as u32 & 0xff };
    let aq = q(a);
    let rq = q(r);
    let gq = q(g);
    let bq = q(b);
    (aq << 24) | (rq << 16) | (gq << 8) | bq
}
