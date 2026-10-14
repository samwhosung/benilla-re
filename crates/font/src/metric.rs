//! The per-glyph advance, kerning, step and width arithmetic of CGxFont. Each `pub fn` cites its
//! source address.
//!
//! The x87 runs at 53-bit precision, so every `f*` operation rounds to `f64` and an `fstp m32` store
//! rounds to `f32`. Each operation is transcribed as `f64` arithmetic with an `as f32` at exactly the
//! exe's f32 stores; integer loads (`fild`) are `i32 as f64`; the CRT `__ftol` (truncate toward zero,
//! `0x40a2b0`) is [`crate::ftol`].
//!
//! ## The rounding helpers
//! The CRT helpers at **`0x73fdf5`** and **`0x73ff3f`** are `ceil` (sets the x87 control word
//! `0x1b3f`, RC = round toward +inf, then `frndint`) and `floor` (control word `0x173f`, RC = round
//! toward -inf). So `glyph_width_scaled`'s pixel mode and `compute_step`'s final round are `ceil`.
//! The tests run the exe's own `0x73fdf5`; `f64::ceil` matches `frndint` with RC up for these
//! integer-valued, in-range results.

use crate::ftol;

/// `CGxFont::GlyphStepBase` (0x5ca2b0): a glyph's step-base metric (`CharCodeDesc+0x4c`) with the
/// outline `+1.0` bias. Bytes: `fld dword[desc+0x4c]; test [font+0x180],0x8; (set) fadd [0x7ff9d8]=1.0`.
/// Returned in `st0` (consumed unrounded by `0x5ca240`→the step kernels), so transcribed as `f64`.
pub fn glyph_metric_outline_bias(step_metric: f32, outline: bool) -> f64 {
    let m = f64::from(step_metric); // fld dword[desc+0x4c]
    if outline {
        m + 1.0 // fadd dword[0x7ff9d8] (0x7ff9d8 = 1.0) when font.flags & 0x8
    } else {
        m
    }
}

/// `CGxFont::GlyphWidth` (0x5cb080): the per-glyph width metric (`CharCodeDesc+0x50`), either rounded
/// (pixel mode) or scaled by `renderSize / font[0x24c]`. Bytes:
/// `mode==0: fld [desc+0x50]; call 0x73fdf5` (= **ceil**, see module note);
/// `mode!=0: ScreenToPixelHeight(1,renderSize)→st0 (passthrough); fidiv dword[font+0x24c]; fmul [desc+0x50]`.
/// Returned in `st0` (consumed unrounded by `char_advance`), so transcribed as `f64`.
pub fn glyph_width_scaled(
    scaled: bool,
    width_metric: f32,
    render_size: f32,
    font_pixel_size: i32,
) -> f64 {
    if scaled {
        // (renderSize / font.renderedPixelSize) · widthMetric — fidiv then fmul, both at PC_53.
        (f64::from(render_size) / f64::from(font_pixel_size)) * f64::from(width_metric)
    } else {
        f64::from(width_metric).ceil() // 0x73fdf5 = ceil
    }
}

/// `CGxFont::ComputeStep` (0x5ca2d0), cache-miss arithmetic: the proportional advance + kerning step.
/// `step = ceil( (f32)( glyphStepBase(cur) + min(rawKern,0) · font.advanceAdjust ) )`. Bytes:
/// negative-clamp `(-1<k)-1 & k` = `min(k,0)`; `call 0x5ca240`→stepbase (st1);
/// `fild [negKern]; fmul [font+0x188]; faddp; fstp dword[ebp-8]` (the `(f32)` store);
/// `fld [ebp-8]; call 0x73fdf5` (= **ceil**). The raw kern is sourced from the OS face (BOUNDARY); font
/// owns the clamp, the `×advanceAdjust`, the add, the `(f32)` narrowing and the ceil.
pub fn compute_step(step_metric: f32, outline: bool, raw_kern: i32, advance_adjust: f32) -> f64 {
    let stepbase = glyph_metric_outline_bias(step_metric, outline);
    let neg = raw_kern.min(0); // KEEP ONLY NEGATIVE kern
    let inter = (stepbase + f64::from(neg) * f64::from(advance_adjust)) as f32; // fstp dword[ebp-8]
    f64::from(inter).ceil() // 0x73fdf5
}

/// `CGxFont::ComputeStepFixedWidth` (0x5ca4b0), the glyph-found centering branch (monospace): each glyph
/// is centred in a cell of width `R = font.rasterPixelSize(+0x178)`. All arithmetic is **integer-domain
/// and UNSIGNED** (`__ftol` → unsigned `cmp`/`shr`/`sub`/`add` → `fild qword{lo,0}` back, i.e. the result
/// reinterpreted as an unsigned 32-bit), the metric inputs/outputs FP. Bytes (5ca63a..):
/// ```text
/// m1 = ftol(glyphStepBase(g1))
/// if (u32)m1 >= R: return (f64)R
/// d1 = R - m1; if d1&1 { d1 -= 1 }; center = R - (d1 >> 1)
/// m2 = ftol(glyphStepBase(g2))            // g2 absent in glyph-hash ⇒ stepbase 0 ⇒ m2 = 0
/// if (u32)m2 >= R: return (f64)center
/// d2 = R - m2; half2 = (d2&1 ? (d2-1)>>1 : d2>>1); return (f64)(center + half2)
/// ```
/// `g2_metric == None` models g2 missing from the glyph hash (`0x5ca240` returns the 0.0 sentinel). The
/// outline bias is font-global, so the same `outline` applies to both glyphs. (The glyph-MISS path —
/// which returns 0.0 and latches a cached 0 — is orchestration; this transcribes the centering.)
pub fn compute_step_fixedwidth(
    g1_metric: f32,
    g2_metric: Option<f32>,
    outline: bool,
    raster_px: u32,
) -> f64 {
    let r = raster_px;
    let m1 = ftol(glyph_metric_outline_bias(g1_metric, outline)) as u32;
    if m1 >= r {
        return f64::from(r);
    }
    let d1 = r.wrapping_sub(m1);
    let d1 = if d1 & 1 != 0 { d1.wrapping_sub(1) } else { d1 };
    let center = r.wrapping_sub(d1 >> 1);
    let m2 = match g2_metric {
        Some(metric) => ftol(glyph_metric_outline_bias(metric, outline)) as u32,
        None => 0, // ftol(0.0 sentinel) = 0
    };
    if m2 >= r {
        return f64::from(center);
    }
    let d2 = r.wrapping_sub(m2);
    let half2 = if d2 & 1 != 0 {
        d2.wrapping_sub(1) >> 1
    } else {
        d2 >> 1
    };
    f64::from(center.wrapping_add(half2))
}

/// `CGxFont::GetCharAdvance` (0x5c6b70), the glyph-metric path: total advance in raster pixels =
/// `GlyphWidth(desc,…) + (i32)desc[0x48]`. Bytes: `call 0x5cb080`→`m` (st0);
/// `mov ecx,[desc+0x48]; fiadd dword[ebp-0xc]` (float-add of the integer pixel advance). Returned in
/// `st0`, so transcribed as `f64`. (The function also returns `0.0` for `curCode==0`, and — when an
/// explicit kerning-pair entry resolves a leading literal — returns that kern value, which is
/// [`compute_step`]/[`compute_step_fixedwidth`]; both are tested through their own functions.)
pub fn char_advance(
    scaled: bool,
    width_metric: f32,
    advance_px: i32,
    render_size: f32,
    font_pixel_size: i32,
) -> f64 {
    let m = glyph_width_scaled(scaled, width_metric, render_size, font_pixel_size);
    m + f64::from(advance_px) // fiadd dword[desc+0x48]
}

/// min-metric clamped screen height (0x5caea0): `max(floorFrac, minMetric / viewportHeight)`, where
/// `minMetric` = the **signed** min over an `i16` array (one value per 4-byte slot at `obj+0x20`, count
/// `obj+0x1c`), seeded `0x7fffffff`. Bytes: `movsx`/`cmp ecx,val`/keep-smaller loop; `call 0x5c7080`
/// (= `min / 0xc2b9a0`, the only FP arithmetic, delegated); `fcomp`→max-clamp (ties return the
/// computed fraction). Empty array (count 0) returns `floorFrac` unchanged.
pub fn min_metric_screen_height(metrics: &[i16], screen_h: i32, floor_frac: f32) -> f32 {
    if metrics.is_empty() {
        return floor_frac; // obj[0x1c] == 0
    }
    let mut m: i32 = 0x7fff_ffff;
    for &v in metrics {
        let v = i32::from(v); // movsx eax, word[edx]
        if m >= v {
            m = v; // cmp ecx,val; jl keep ⇒ ecx = min(ecx, val)
        }
    }
    let f = f64::from(m) / f64::from(screen_h); // 0x5c7080: fild min; fidiv [0xc2b9a0]
    if f64::from(floor_frac) > f {
        floor_frac
    } else {
        f as f32
    }
}
