//! `PlayerName.cpp` per-category **timing and scale** maths: the functions that turn an elapsed-ms
//! count and a WORLDTEXTSTRING category into the rising-text offset, the scale curve and the alpha and
//! colour fade bytes. Each `pub fn` follows the exe op for op and is compared with it in `tests/timing.rs`.
//!
//! ## The per-category config table (`.bss` `0xce8828`, stride `0x1c`)
//! Init `0x6c79a0` fills ~5 rows; `row = this[8] * 0x1c` where `this[8]` is the WORLDTEXTSTRING category.
//! The fields these primitives read (offsets are from the table base, added to `0xce8828`):
//!   * `+0x00` f32 **rise-span**  (`0xce8828 + row`)
//!   * `+0x04` i32 **fade-in-end-ms**  (`0xce882c + row`)
//!   * `+0x08` i32 **fade-out-start-ms**  (`0xce8830 + row`)
//!   * `+0x0c` i32 **total-duration-ms**  (`0xce8834 + row`)
//!   * `+0x10` f32 **value-lo**  (`0xce8838 + row`)
//!   * `+0x14` f32 **value-hi**  (`0xce883c + row`)
//!
//! ## PC_53 / f64 discipline
//! WinMain sets `_controlfp(_PC_53)`; every x87 chain here is therefore an `f64` computation, with an
//! `as f32` applied exactly where the binary does an `fstp dword` (narrow-to-f32) and an f32/i32 memory
//! operand widened/converted exactly at each `fld dword`/`fild`/`fidiv dword`/`fmul dword`. `fild qword`
//! of a value whose high dword is zero reads an unsigned 32-bit count.
#![allow(clippy::float_cmp)]

/// The per-category config row this timing math reads (the six fields each primitive consumes; the
/// remaining `+0x18` default-color dword is the WORLDTEXTSTRING ctor's, not ours). Field names match
/// the table-offset comments above.
#[derive(Clone, Copy, Debug)]
pub struct Category {
    /// `+0x00` f32 rise-span (`fmul dword [0xce8828+row]` in `pn_interp_param`).
    pub rise_span: f32,
    /// `+0x04` i32 fade-in-end-ms (`[0xce882c+row]`).
    pub fade_in_end_ms: i32,
    /// `+0x08` i32 fade-out-start-ms (`[0xce8830+row]`).
    pub fade_out_start_ms: i32,
    /// `+0x0c` i32 total-duration-ms (`fidiv dword [0xce8834+row]`).
    pub duration_ms: i32,
    /// `+0x10` f32 value-lo (`[0xce8838+row]`).
    pub value_lo: f32,
    /// `+0x14` f32 value-hi (`[0xce883c+row]`).
    pub value_hi: f32,
}

/// `0x40a2b0` `__ftol`: truncate-toward-zero to a 64-bit int (RC=11 via `or ah,0xc`; `fistp qword`),
/// returning the low dword. The binary uses only the low 32 bits (`eax`); we model the truncated value
/// modulo 2^32. On x87 an out-of-`i64`-range operand yields the "indefinite" `0x8000…0000`, but
/// every call site here has already clamped its operand into `[0,255]`-ish before the relevant byte
/// compare, so the low dword is the truncated magnitude.
#[inline]
fn ftol(v: f64) -> u32 {
    (v.trunc() as i64) as u32
}

/// `0x6c8070` **pn_interp_param** — the rising-text vertical offset for one elapsed-ms sample.
///
/// Op trace (`ecx`=this, `[esp+4]`=elapsed u32; result live on st0, the caller `fadd`s it):
/// ```text
///   imul eax, [ecx+8], 0x1c        ; row = cat * 0x1c
///   mov  [ebp-8], elapsed ; xor → [ebp-4]=0   ; build a u64 with hi-dword 0
///   fild qword [ebp-8]             ; st0 = (f64) (unsigned) elapsed
///   fidiv dword [0xce8834 + row]   ; / duration_ms (i32)
///   fmul dword  [0xce8828 + row]   ; * rise_span (f32)
///   ret 4                          ; st0 left live
/// ```
/// Returns the value left on st0 as the full PC_53 `f64` (the caller consumes it un-narrowed). The
/// test reads it through the harness's `call_f64`, an `fstp qword` trampoline.
pub fn interp_param(elapsed_ms: u32, cat: &Category) -> f64 {
    // fild qword (unsigned: hi dword is 0) → f64; fidiv dword (i32 duration); fmul dword (f32 span).
    let t = f64::from(elapsed_ms) / f64::from(cat.duration_ms);
    t * f64::from(cat.rise_span)
}

/// `0x6c80b0` **pn_keyframe_interp** — the per-category scale curve, clamped to a `0.001` floor.
///
/// `t = elapsed/dur` is computed in f64 then **spilled to f32 and reloaded** (`fstp dword [ebp+8]` /
/// `fld dword [ebp+8]`), so every later use of `t` is the f32-narrowed value widened back to f64.
///
/// * **cat != 2** — affine: `(value_hi − value_lo) · t + value_lo`
///   (`fld [0xce883c]; fsub [0xce8838]; fmul [ebp+8]; fadd [0xce8838]`).
/// * **cat == 2** — 3-segment piecewise over the keyframe table `0x8112dc` (stride `0x10`:
///   seg_lo, seg_hi, val_lo, val_hi), the matching segment giving
///   `(val_hi − val_lo)·((t − seg_lo)/(seg_hi − seg_lo)) + val_lo`; no segment matches (`t ≥ 1`) →
///   the default `1.0` (`[0x7ff9d8]`). The segment value is then `· [edi+0xce883c]`. with
///   `edi = cat·0x1c = 2·0x1c = 0x38`, `[edi+0xce883c]` resolves to `[0xce8874]` — which is *exactly*
///   row 2's own `value_hi` slot (`0xce883c + 0x38`). So the cat==2 multiplier IS `cat.value_hi`
///   (the real init value there is `0.0275`, `0x3ce147ad`). Rows: `[0,.1]→.1..2.0`, `[.1,.2]→2.0..1.0`,
///   `[.2,1]→1.0..1.0`.
///
/// Final clamp (`fcom [0x801360]=0.001; test ah,0x41; jne`): the result is the computed value if it is
/// strictly `> 0.001`, else exactly `0.001` (`0x3a83126f`). The binary narrows the chosen value to f32
/// (`fstp dword [ebp+8]`) before relaying it to `0x6c82c0`; we return that f32.
pub fn keyframe_interp(elapsed_ms: u32, cat_index: i32, cat: &Category) -> f32 {
    // t = elapsed/dur, spilled to f32 then reloaded (widened back to f64 for every later use).
    let t = (f64::from(elapsed_ms) / f64::from(cat.duration_ms)) as f32;
    let tf = f64::from(t); // the f32-reloaded t, widened to PC_53 f64

    let v: f64 = if cat_index != 2 {
        // affine: (value_hi - value_lo) * t + value_lo
        (f64::from(cat.value_hi) - f64::from(cat.value_lo)) * tf + f64::from(cat.value_lo)
    } else {
        // 3-segment piecewise over 0x8112dc. The loop matches seg `i` when seg_lo[i] <= t < seg_hi[i].
        // Comparisons use the f32-reloaded t directly (the table operands are f32 widened to f64). The
        // f32 bits are read verbatim from `.rdata` (`0x3dcccccd`=0.1f, `0x3e4ccccd`=0.2f, etc.) so the
        // widened-to-f64 operands are bit-exact to the binary's.
        const TABLE: [[u32; 4]; 3] = [
            // seg_lo, seg_hi, val_lo, val_hi  (0x8112dc, stride 0x10)
            [0x0000_0000, 0x3dcc_cccd, 0x3dcc_cccd, 0x4000_0000], // row0: [0,.1] → .1..2.0
            [0x3dcc_cccd, 0x3e4c_cccd, 0x4000_0000, 0x3f80_0000], // row1: [.1,.2] → 2.0..1.0
            [0x3e4c_cccd, 0x3f80_0000, 0x3f80_0000, 0x3f80_0000], // row2: [.2,1] → 1.0..1.0
        ];
        let mut seg_val: f64 = 1.0; // default if no segment matches (t >= 1) → [0x7ff9d8]=1.0
        for row in &TABLE {
            let seg_lo = f64::from(f32::from_bits(row[0]));
            let seg_hi = f64::from(f32::from_bits(row[1]));
            // `fld t; fcomp seg_lo; test ah,1; jne` → skip if t < seg_lo; then `fcomp seg_hi;
            // test ah,5; jnp` → match if t < seg_hi. So: seg_lo <= t AND t < seg_hi.
            if seg_lo <= tf && tf < seg_hi {
                let val_lo = f64::from(f32::from_bits(row[2]));
                let val_hi = f64::from(f32::from_bits(row[3]));
                // (val_hi - val_lo) * ((t - seg_lo)/(seg_hi - seg_lo)) + val_lo
                seg_val = (val_hi - val_lo) * ((tf - seg_lo) / (seg_hi - seg_lo)) + val_lo;
                break;
            }
        }
        seg_val * f64::from(cat.value_hi) // * [edi+0xce883c] = [0xce8874] = row2.value_hi (0.0275)
    };

    // Final clamp: result if v > 0.001 (strictly), else 0.001. The binary stores the chosen value as
    // f32 (`fstp dword [ebp+8]`) before relaying it.
    const FLOOR_BITS: u32 = 0x3a83_126f; // 0.001 (the [0x801360] compare const and the clamp imm are equal)
    let floor = f64::from(f32::from_bits(FLOOR_BITS));
    if v > floor {
        v as f32
    } else {
        f32::from_bits(FLOOR_BITS)
    }
}

/// The `(alpha, color)` byte pair `pn_time_alpha_fade` produces (`alpha` → `this[0x23]`; `color` → the
/// high byte of the dword relayed to `0x5c2750`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlphaFade {
    pub alpha: u8,
    pub color: u8,
}

/// `0x6c82e0` **pn_time_alpha_fade** — the alpha + color fade bytes for one elapsed-ms sample.
///
/// `t = elapsed/dur` (`fild qword [elapsed,0]; fidiv dword [0xce8834+row]`). Three regimes on the
/// **unsigned** `elapsed`:
/// * **fade-IN** (`elapsed < fade_in_end_ms`): `alpha = clamp(__ftol(255.0·t), 0, 255)`,
///   `color = clamp(__ftol(127.0·t), 0, 127)` (255.0 @`0x7ffe58`, 127.0 @`0x811310`). The 255-side uses
///   `cmp eax,0xff; jbe` (`> 0xff → 0xff`), the 127-side `cmp eax,0x7f; jbe` (`> 0x7f → 0x7f`).
/// * **mid** (`fade_in_end_ms ≤ elapsed < fade_out_start_ms`): `alpha = 0xff`, `color = 0x7f`
///   (the entry defaults `bl=0xff`, `al=0x7f`, untouched on this path).
/// * **fade-OUT** (`elapsed ≥ fade_out_start_ms`): mirror of fade-in over the remaining window
///   `t = (elapsed − fade_out_start) / (duration − fade_out_start)`, producing
///   `alpha = __ftol(255.0 − clamp(255.0·t, 0, 255))` and `color = __ftol(255.0 − clamp(127.0·t, 0, 127))`
///   (clamps via 0.0 @`0x7ffd74`, 255.0 @`0x7ffe58`, 127.0 @`0x811310`; the final subtrahend is 255.0 on
///   BOTH sides). `__ftol` POPS its operand, so the alpha curve (`255·t`) and color curve (`127·t`) are
///   independent re-multiplications of `t`. A degenerate window (`duration == fade_out_start`, i.e.
///   `esi-ecx==0` → `je 0x6c8383`) loads `t = 0.0`.
///
/// Comparisons are on `param_2` (the unsigned elapsed) vs the i32 table fields, matching `cmp`/`jae`/
/// `jb` (`jae`=above-or-equal unsigned, `jb`=below unsigned). Two `__ftol` per active branch.
pub fn time_alpha_fade(elapsed_ms: u32, cat: &Category) -> AlphaFade {
    let dur = cat.duration_ms;
    let in_end = cat.fade_in_end_ms as u32; // [0xce882c] compared unsigned (cmp/jae)
    let out_start = cat.fade_out_start_ms as u32; // [0xce8830] compared unsigned (cmp/jb)

    if elapsed_ms < in_end {
        // fade-IN: t = elapsed/dur (unsigned fild qword).
        let t = f64::from(elapsed_ms) / f64::from(dur);
        // alpha = __ftol(255*t), then `cmp eax,0xff; jbe` (> 0xff → 0xff).
        let a = ftol(255.0 * t);
        let alpha = if a <= 0xff { a as u8 } else { 0xff };
        // color = __ftol(127*t), then `cmp eax,0x7f; jbe` (> 0x7f → 0x7f).
        let c = ftol(127.0 * t);
        let color = if c <= 0x7f { c as u8 } else { 0x7f };
        AlphaFade { alpha, color }
    } else if elapsed_ms >= out_start {
        // fade-OUT. esi holds dur(i32) from the row load; the binary reuses esi = (dur - out_start) as
        // the window denominator (`sub esi,ecx`).
        let span = dur.wrapping_sub(out_start as i32); // esi - ecx
        let t = if span == 0 {
            0.0_f64 // je 0x6c8383: degenerate window → t = 0.0 [0x7ffd74]
        } else {
            // t = (elapsed - out_start) / (dur - out_start) — both subtractions marshalled into a
            // u64 (hi=0) for `fild qword`.
            f64::from(elapsed_ms.wrapping_sub(out_start)) / f64::from(span)
        };
        // alpha side: a = 255.0*t (`fmul st,st(1)`), clamp a into [0,255], then `255.0 - a`, `__ftol`.
        // `__ftol` POPS, and `t` is preserved at the bottom of the x87 stack throughout.
        let a = 255.0 * t;
        let a = clamp_f(a, 0.0, 255.0);
        let alpha = ftol(255.0 - a) as u8; // mov bl,al — low byte
                                           // color side: independent re-mul of t: b = 127.0*t (`fmul [0x811310]`), clamp into [0,127], then
                                           // `255.0 - b` (the subtrahend is 255.0 on this side too), `__ftol`.
        let b = 127.0 * t;
        let b = clamp_f(b, 0.0, 127.0);
        let color = ftol(255.0 - b) as u8;
        AlphaFade { alpha, color }
    } else {
        // mid: defaults bl=0xff, al=0x7f.
        AlphaFade {
            alpha: 0xff,
            color: 0x7f,
        }
    }
}

/// The fade-out clamp idiom `max(lo, min(hi, v))` exactly as the binary's `fcomp`-driven
/// `test ah,0x41 / test ah,0x05` ladder builds it (see `time_alpha_fade` fade-OUT). All in PC_53 f64.
#[inline]
fn clamp_f(v: f64, lo: f64, hi: f64) -> f64 {
    // The binary tests `hi <= v` then `v < hi` (net `v >= hi → hi`), then a `lo`-side test
    // (`v < lo → lo`). Equivalent to the standard min/max clamp.
    if v >= hi {
        hi
    } else if v < lo {
        lo
    } else {
        v
    }
}

/// `0x6c82c0` **pn_width_dirty_guard** — `param != this[0x2c]`: store + relay only when the new width
/// differs from the cached one.
///
/// Op trace (`ecx`=this, `[esp+4]`=param f32):
/// ```text
///   fld   dword [ebp+8]        ; param
///   fcomp dword [ecx+0x2c]     ; compare to cached width this[0x2c]
///   fnstsw ax ; test ah,0x44 ; jnp 0x6c82db   ; ah bits C3(0x40)|C2(0x04)
///   ; (jnp taken → skip to ret; not-taken → store this[0x2c]=param; call 0x6c81a0 relayout)
/// ```
/// `test ah,0x44; jnp`: PF reflects the parity of the masked bits (`C3`,`C2`). `jnp` jumps when PF=0
/// (odd parity). Equal → C3=1,C2=0 (one bit, odd → PF=0 → `jnp` TAKEN → **skip**, no store).
/// Less/greater → C3=0,C2=0 (zero bits, even → PF=1 → not taken → **store + relay**). Unordered (NaN) →
/// C3=1,C2=1 (two bits, even → PF=1 → not taken → **store + relay**). So the store-and-relay path runs
/// **iff `param != cached`** (the binary's floating `!=` predicate); the function returns whether it
/// stored (the inequality result). NaN is "different", so it relays. Returns `true` when it stored.
pub fn width_dirty_guard(param: f32, cached: f32) -> bool {
    // Store + relay iff NOT equal — the FP `!=`. NaN (unordered) is "different" and relays: Rust's
    // `!=` already yields `true` whenever either operand is NaN, matching the `jnp`-not-taken path.
    f64::from(param) != f64::from(cached)
}
