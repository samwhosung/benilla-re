//! The text extent, wrap, fit and step kernels of CGxFont. Each `pub fn` cites its source address.
//!
//! The x87 runs at 53-bit precision, so every `f*` operation rounds to `f64` and an `fstp m32` store
//! rounds to `f32`. Each operation is transcribed as `f64` arithmetic with an `as f32` at exactly the
//! exe's f32 stores; integer loads (`fild`) are `i32 as f64`; the CRT `__ftol` (truncate toward zero,
//! `0x40a2b0`) is [`crate::ftol`].
//!
//! ## The rounding helpers
//! The CRT helpers at **`0x73fdf5`** and **`0x73ff3f`** are `ceil` (x87 control word `0x1b3f`,
//! RC = round toward +inf) and `floor` (control word `0x173f`, RC = round toward -inf). So the
//! extra-spacing pixel quantize (`0x73fdf5`) and the proportional wrap threshold (`0x73fdf5`) are
//! `ceil`; the viewport-rect snap (`0x73ff3f`) is `floor`. `f64::ceil` and `f64::floor` match
//! `frndint` with that rounding control for these integer-valued, in-range results.
//!
//! ## Composition (these kernels call sub-functions tested in `metric`)
//! The measure loops resolve, per glyph, an advance (`char_advance`, 0x5c6b70, whose glyph-metric
//! leg is [`crate::metric::glyph_width_scaled`], 0x5cb080) and an escapement (the previous glyph's
//! advance plus kern, 0x5ca2d0 and 0x5ca4b0). Those sub-functions are tested in `metric`; here they
//! are parameters of [`FontCtx`], so each `pub fn` transcribes the kernel's own logic: the
//! decode and classify walk ([`crate::markup::decode_quoted_code`], 0x5c2810), the run and line-max
//! accumulation, the wrap-threshold ceiling, the per-char cumulative array, and the device scaling.

// `!(a > b)` is the precise transcription of the binary's `fcomp` + conditional jump: the *unordered*
// (NaN) case must take the same branch as `b >= a` (the validation/degenerate path), which `a <= b`
// would not (it is false for NaN). Each such site mirrors a specific `ja`/`jbe`/`jp` — keep the form.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::ftol;
use crate::markup::decode_quoted_code;
use crate::metric::glyph_width_scaled;
use crate::screen_pixel::{one_to_one_height, screen_to_pixel_height, screen_to_pixel_width};

// ── module constants ──
/// `0x808120` = `0x3f7fff58` ≈ 0.99999 — the ceil bias added before `__ftol` in the wrap-threshold.
const CEIL_BIAS: f32 = f32::from_bits(0x3f7f_ff58);
/// `0x7ff9d8` = 1.0 — unit numerator (wrap hf clamp / proportional `1/WIDTH`).
const ONE: f32 = 1.0;
/// `0x80306c` = 4.0 — line/advance pad when font.flags(+0x180) bit3 (0x8, outline) set.
const PAD_OUTLINE: f32 = 4.0;
/// `0x801628` = 2.0 — line/advance pad when font.flags bit0 (0x1, shadow) set.
const PAD_SHADOW: f32 = 2.0;
/// `0x8026bc` = `0x35800000` = 2⁻²⁰ — the height-fit epsilon (0x5c21c0).
const HEIGHT_FIT_EPS: f32 = f32::from_bits(0x3580_0000);

/// The font + viewport environment a measure kernel reads through its sub-calls, bundled so the kernel
/// `pub fn`s carry only their measure parameters. `glyph` models `0x5cabd0` (NewCodeDesc): `Some((width
/// metric @desc+0x50, integer pixel advance @desc+0x48))` when the glyph resolves, `None` for the
/// no-glyph sentinel. `escapement(prev, cur)` models `0x5ca2d0`/`0x5ca4b0` (the previous glyph's
/// advance+kern step), returned as the raw `st0` (`f64`).
pub struct FontCtx<'a> {
    /// `font[0x180]` — pad/style flags (bit3 0x8 outline → +4.0, bit0 0x1 shadow → +2.0).
    pub font_flags: u32,
    /// `font[0x24c]` — rendered pixel/em size (`0x5cae90`); denominator of the device scale.
    pub pixel_size: i32,
    /// `0xc2b9a4` — viewport WIDTH px.
    pub screen_w: i32,
    /// `0xc2b9a0` — viewport HEIGHT px.
    pub screen_h: i32,
    /// `0x5cabd0` NewCodeDesc → `(width_metric, advance_px)` or `None`.
    pub glyph: &'a dyn Fn(u32) -> Option<(f32, i32)>,
    /// `0x5ca2d0`/`0x5ca4b0` escapement(prev, cur) → step (`st0`, f64).
    pub escapement: &'a dyn Fn(u32, u32) -> f64,
}

// ── CRT helpers transcribed inline (the decoder's UTF-8 + isspace; mirror markup.rs) ─────────────

/// `0x41aab0` — UTF-8 next-char decode (mirror of `markup`'s private copy). Returns `(codepoint,
/// bytes_consumed)`; reads past the slice are NUL. Used by the fixed/proportional trailing-whitespace skip.
fn utf8_next(text: &[u8], pos: usize) -> (u32, i32) {
    let b = |i: usize| -> u32 { u32::from(text.get(pos + i).copied().unwrap_or(0)) };
    let lead = b(0);
    if lead == 0 {
        return (0xffff_ffff, 0);
    }
    let (n, mut acc): (i32, u32) = if lead & 0xfe == 0xfc {
        (5, lead & 0x01)
    } else if lead & 0xfc == 0xf8 {
        (4, lead & 0x03)
    } else if lead & 0xf8 == 0xf0 {
        (3, lead & 0x07)
    } else if lead & 0xf0 == 0xe0 {
        (2, lead & 0x0f)
    } else if lead & 0xe0 == 0xc0 {
        (1, lead & 0x1f)
    } else if lead & 0x80 == 0x80 {
        return (0x8000_0000, 1);
    } else {
        return (lead, 1);
    };
    let mut len: i32 = 1;
    let mut count: i32 = 0;
    while count < n {
        let cb = b((count + 1) as usize);
        if cb == 0 {
            return (0xffff_ffff, len);
        }
        len += 1;
        if cb & 0xc0 != 0x80 {
            return (0x8000_0000, len);
        }
        acc = (acc << 6) | (cb & 0x3f);
        count += 1;
    }
    (acc, len)
}

/// `0x74006f` — C-locale `isspace` over the static ctype table (`{0x09..0x0d, 0x20}` for `c<0x100`;
/// the runtime-locale `c>=0x100` path is modelled false, as in `markup`).
fn isspace_c(c: u32) -> bool {
    matches!(c & 0xffff, 0x09..=0x0d | 0x20)
}

/// The shared trailing-whitespace skip (0x5c7300 5c741f / 0x5c7470 5c7710): advance past UTF-8
/// whitespace from `cursor`, returning the new cursor offset.
fn skip_trailing_ws(text: &[u8], mut cursor: usize) -> usize {
    if text.get(cursor).copied().unwrap_or(0) == 0 {
        return cursor;
    }
    loop {
        let (cp, consumed) = utf8_next(text, cursor);
        if !isspace_c(cp) {
            return cursor; // non-space: outNext is BEFORE it
        }
        let next_byte = text.get(cursor + consumed as usize).copied().unwrap_or(0);
        cursor += consumed as usize;
        if next_byte == 0 {
            return cursor;
        }
    }
}

/// The per-line pen seed shared by text_extent / wrap / proportional (e.g. 0x5c6940 `5c698d..5c69e2`):
/// `tmp = (extra>0 ? ceil(ScreenToPixelWidth(extra)) : 0)` then `+4.0` (font.flags&0x8) or `+2.0`
/// (font.flags&0x1). ScreenToPixelWidth is always called with `already_pixels=false` here (`xor ecx,ecx`).
fn pen_seed(extra_spacing: f32, screen_w: i32, font_flags: u32) -> f32 {
    let mut tmp: f32 = 0.0;
    if extra_spacing > 0.0 {
        let spw = screen_to_pixel_width(false, extra_spacing, screen_w); // 0x5c7010
        tmp = f64::from(spw).ceil() as f32; // 0x73fdf5 = ceil; fstp dword
    }
    if font_flags & 0x8 != 0 {
        tmp = (f64::from(tmp) + f64::from(PAD_OUTLINE)) as f32; // fadd [0x80306c]
    } else if font_flags & 0x1 != 0 {
        tmp = (f64::from(tmp) + f64::from(PAD_SHADOW)) as f32; // fadd [0x801628]
    }
    tmp
}

/// `CGxFont::GetCharAdvance` (0x5c6b70): the per-glyph advance in raster pixels. Walks leading control
/// codes from `text` (the cursor *after* the measured token); on the first literal it computes the
/// explicit-kern lookahead `escapement(cur, next)` and returns it when non-zero; otherwise (or on a
/// newline / end-of-string before any literal) returns the glyph metric
/// `glyph_width_scaled((flags>>7)&1, width, size, pixel_size) + (i32)advance` (0x5cb080 + `fiadd`).
/// `cur==0 ⇒ 0.0`; glyph-miss ⇒ the last (zero) kern. Result is the raw `st0` (`f64`), consumed unrounded.
fn char_advance(cur: u32, text: &[u8], size: f32, flags: u32, ctx: &FontCtx) -> f64 {
    if cur == 0 {
        return 0.0; // 5c6b7e: fld [0x7ffd74]
    }
    let mut kern: f64 = 0.0; // [ebp-8]
    if text.first().copied().unwrap_or(0) != 0 {
        let mut pos = 0usize;
        loop {
            let dec = decode_quoted_code(&text[pos..], flags); // 5c6bad
            pos += dec.consumed as usize; // esi += consumed
            if dec.class == 6 {
                kern = (ctx.escapement)(cur, dec.out_code); // 5c6bc6 escapement(cur, next)
                if kern != 0.0 {
                    return kern; // 5c6bf9: explicit-kern return
                }
                break; // 5c6bfb → glyph metric
            } else if dec.class == 2 {
                break; // 5c6bba: newline → glyph metric
            } else if text.get(pos).copied().unwrap_or(0) == 0 {
                break; // 5c6bbf: control code, no more text → glyph metric
            }
        }
    }
    match (ctx.glyph)(cur) {
        // 5c6bfd: 0x5cabd0
        None => kern, // 5c6c3d: returns [ebp-8] (= 0.0)
        Some((w, adv)) => {
            let scaled = flags & 0x80 != 0; // (flags>>7)&1
            glyph_width_scaled(scaled, w, size, ctx.pixel_size) + f64::from(adv)
            // 0x5cb080 + fiadd [+0x48]
        }
    }
}

/// `0x5c6940` — measure the maximal wrapped **horizontal extent** of `text[0..max_bytes]`, returning
/// the screen-fraction width (raw px when `flags&0x80`). Covers: size-default (`size==0 || flags&4`), the
/// per-line pen seed, the decode/classify walk (class 2 / a `0x0a` glyph = newline → line-max reset;
/// class 3 = escaped `|` glyph; classes 0/1/4/5 = zero-width skip; class 6 = glyph), the
/// `run += escapement(prev,cur)` accumulation, `lastAdv = char_advance`, the per-line
/// `max(lineMax, lastAdv+run)`, and the final `(ScreenToPixelHeight(size)/pixelSize)·width` (÷WIDTH
/// unless `0x80`). Returns the value written to `*out`.
#[allow(clippy::too_many_arguments)]
pub fn text_extent_measure(
    ctx: &FontCtx,
    text: &[u8],
    max_bytes: i32,
    size: f32,
    flags: u32,
    extra_spacing: f32,
) -> f32 {
    let mut size = size;
    if size == 0.0 || flags & 0x4 != 0 {
        size = one_to_one_height(ctx.pixel_size, ctx.screen_h); // 0x5c2540
    }
    let mut run = pen_seed(extra_spacing, ctx.screen_w, ctx.font_flags); // [ebp+0x14]
    let mut prev: u32 = 0; // [ebp-0x10]
    let mut last_adv: f32 = 0.0; // [ebp-4]
    let mut line_max: f32 = 0.0; // [ebp-8]
    let mut cursor = 0usize;
    let mut remaining = max_bytes;
    if text.first().copied().unwrap_or(0) != 0 {
        loop {
            if remaining == 0 {
                break; // 5c6a03
            }
            let dec = decode_quoted_code(&text[cursor..], flags); // 5c6a1a
            remaining = remaining.wrapping_sub(dec.consumed);
            cursor += dec.consumed as usize;
            let mut newline = dec.class == 2;
            let glyph = dec.class == 3 || dec.class == 6; // jump table @0x5c6b4c
            if glyph && dec.out_code == 0x0a {
                newline = true; // 5c6a3b: a 0x0a glyph is a newline
            } else if glyph && (ctx.glyph)(dec.out_code).is_some() {
                // 5c6a40: 0x5cabd0 != 0
                let kern = if prev != 0 {
                    (ctx.escapement)(prev, dec.out_code)
                } else {
                    0.0
                };
                let adv = char_advance(dec.out_code, &text[cursor..], size, flags, ctx);
                last_adv = adv as f32; // 5c6a8c
                run = (kern + f64::from(run)) as f32; // 5c6a95: run += kern
                prev = dec.out_code;
            }
            if newline {
                // 5c6aa0
                let line_w = f64::from(last_adv) + f64::from(run);
                if line_w > f64::from(line_max) {
                    line_max = line_w as f32; // strict >; equal/NaN keep
                }
                run = 0.0;
                last_adv = 0.0; // prev is NOT reset
            }
            if text.get(cursor).copied().unwrap_or(0) == 0 {
                break; // 5c6ac5
            }
        }
    }
    let last_line = f64::from(last_adv) + f64::from(run); // 5c6ace
    let width: f32 = if last_line > f64::from(line_max) {
        last_line as f32
    } else {
        line_max
    };
    let already = flags & 0x80 != 0;
    let h = f64::from(screen_to_pixel_height(already, size, ctx.screen_h)); // 0x5c6fa0
    let scaled = (h / f64::from(ctx.pixel_size)) * f64::from(width); // fdivr px; fmul width
    if already {
        scaled as f32
    } else {
        (scaled / f64::from(ctx.screen_w)) as f32 // ÷ 0xc2b9a4
    }
}

/// `0x5c6c50` — count the glyphs that fit in `box_width_frac` on one line, returning `(count, *outWidth,
/// *outBytes)` and (when `out_cum` is `Some`) the per-char cumulative widths. Covers: size-default (skipped
/// when `flags&0x80`), pen seed, the integer wrap **threshold** `ceil(pixelSize·WIDTH·boxFrac /
/// max(ScreenToPixelHeight(size),1))` (`__ftol(q + 0.99999)`, the bias added only when `q>0`), the device
/// `scale = ScreenToPixelHeight(size)/pixelSize`, the per-glyph fit test `(kern + adv + run) > threshold`
/// (stop, don't count, rewind the cursor), and on accept `run += kern; lastAdv = adv; count++; cum[i] =
/// run·scale` (÷WIDTH unless `0x80`). A newline (class 2) ends the line; class 3 is a glyph.
#[allow(clippy::too_many_arguments)]
pub fn wrap_max_chars(
    ctx: &FontCtx,
    text: &[u8],
    size: f32,
    box_width_frac: f32,
    max_bytes: i32,
    flags: u32,
    extra_spacing: f32,
    mut out_cum: Option<&mut Vec<f32>>,
) -> (i32, f32, i32) {
    let raw = flags & 0x80 != 0; // [ebp-0xc]
    let mut count: i32 = 0; // [ebp-4]
    let mut size = size;
    if !raw && (size == 0.0 || flags & 0x4 != 0) {
        size = one_to_one_height(ctx.pixel_size, ctx.screen_h);
    }
    let mut run = pen_seed(extra_spacing, ctx.screen_w, ctx.font_flags); // [ebp+0x18]

    // threshold (5c6d30..5c6d7b)
    let px = f64::from(ctx.pixel_size);
    let maxw = (px * f64::from(ctx.screen_w)) * f64::from(box_width_frac); // fild px; fimul WIDTH; fmul box
    let hf = f64::from(screen_to_pixel_height(raw, size, ctx.screen_h)); // 0x5c6fa0
    let hf = if hf > f64::from(ONE) {
        hf
    } else {
        f64::from(ONE)
    }; // max(hf, 1.0)
    let q = maxw / hf; // fdivr
                       // `__ftol` then `fild qword{lo, hi=0}` in the fit test (5c6e6f) ZERO-EXTENDS the int → compared as u32.
    let threshold: u32 = if q > 0.0 {
        ftol(q + f64::from(CEIL_BIAS)) as u32
    } else {
        ftol(q) as u32
    };

    // scale (5c6d85..5c6db1): hf2/px, stored to f32 (`fstp dword [ebp-0x20]`) before use.
    let hf2 = f64::from(screen_to_pixel_height(raw, size, ctx.screen_h));
    let scale = (hf2 / px) as f32; // fdivr px; fstp dword

    let mut prev: u32 = 0; // [ebp-0x14]
    let mut last_adv: f32 = 0.0; // [ebp-0x1c]
    let mut cursor = 0usize;
    let mut max_bytes = max_bytes;
    if text.first().copied().unwrap_or(0) != 0 {
        loop {
            if max_bytes == 0 {
                break; // 5c6dc0
            }
            let dec = decode_quoted_code(&text[cursor..], flags); // 5c6dd7
            max_bytes = max_bytes.wrapping_sub(dec.consumed);
            cursor += dec.consumed as usize;
            if dec.class == 2 {
                break; // 5c6de6: newline ends the line
            }
            let glyph = dec.class == 3 || dec.class == 6; // byte table [0,0,1,1,0,0]
            if !glyph {
                if text.get(cursor).copied().unwrap_or(0) == 0 {
                    break;
                }
                continue; // 5c6ec8: skip control code
            }
            match (ctx.glyph)(dec.out_code) {
                None => {
                    count += 1; // 5c6e14: count a no-glyph, no width
                }
                Some(_) => {
                    let kern_raw = if prev != 0 {
                        (ctx.escapement)(prev, dec.out_code)
                    } else {
                        0.0
                    };
                    let kern = f64::from(kern_raw as f32); // [ebp+0xc] f32 store+reload
                    let adv = char_advance(dec.out_code, &text[cursor..], size, flags, ctx); // raw st0
                    let sum = (kern + adv) + f64::from(run); // fit test (5c6e5c)
                    if sum > f64::from(threshold) {
                        cursor -= dec.consumed as usize; // 5c6ed8: rewind the rejected token
                        break;
                    }
                    run = (kern + f64::from(run)) as f32; // run += kern
                    last_adv = adv as f32;
                    prev = dec.out_code;
                    count += 1;
                    if let Some(arr) = out_cum.as_deref_mut() {
                        let cw = f64::from(run) * f64::from(scale); // 5c6e9e
                        arr.push(if raw {
                            cw as f32
                        } else {
                            (cw / f64::from(ctx.screen_w)) as f32
                        });
                    }
                }
            }
            if text.get(cursor).copied().unwrap_or(0) == 0 {
                break; // 5c6ec8
            }
        }
    }
    let lw = (f64::from(last_adv) + f64::from(run)) * f64::from(scale); // 5c6eda
    let out_width = if raw {
        lw as f32
    } else {
        (lw / f64::from(ctx.screen_w)) as f32
    };
    (count, out_width, cursor as i32)
}

/// `0x5c7300` — measure one fixed-width step (the whole remaining run, no box) returning `(*outBytes,
/// *outWidth, *outNext)`. `size==0 ⇒ zeros`. Covers: per-glyph `escSum += escapement(prev,cur)` (raw `st0`,
/// added unrounded), `lastAdv = char_advance`, end `*outWidth = (escSum + lastAdv)·(size/pixelSize)`
/// (note: raw `size`/px, not ScreenToPixel), `*outBytes = cursor`, then a trailing-whitespace skip to
/// `*outNext`. A newline (class 2) ends the run; class 3 is a glyph.
pub fn measure_run_fixedwidth(
    ctx: &FontCtx,
    text: &[u8],
    size: f32,
    flags: u32,
) -> (i32, f32, i32) {
    if size == 0.0 {
        return (0, 0.0, 0); // 5c731c
    }
    let mut prev: u32 = 0; // edi
    let mut last_adv: f32 = 0.0; // [ebp-0x14]
    let mut esc_sum: f32 = 0.0; // [ebp-0x10]
    let mut cursor = 0usize;
    if text.first().copied().unwrap_or(0) != 0 {
        loop {
            let dec = decode_quoted_code(&text[cursor..], flags); // 5c7364
            if dec.class == 2 {
                break; // 5c7369: newline ends the run (cursor NOT advanced past it)
            }
            let glyph = dec.class == 3 || dec.class == 6;
            if glyph && (ctx.glyph)(dec.out_code).is_some() {
                // 5c7387: 0x5cabd0 != 0
                let kern = if prev != 0 {
                    (ctx.escapement)(prev, dec.out_code) // raw st0
                } else {
                    0.0 // 5c7395: fld 0.0
                };
                esc_sum = (kern + f64::from(esc_sum)) as f32; // 5c73c1: escSum += kern
                let after = cursor + dec.consumed as usize;
                let adv = char_advance(dec.out_code, &text[after..], size, flags, ctx);
                last_adv = adv as f32; // 5c73da
                prev = dec.out_code;
            }
            cursor += dec.consumed as usize; // 5c73e0
            if text.get(cursor).copied().unwrap_or(0) == 0 {
                break; // 5c73e3
            }
        }
    }
    let px = f64::from(ctx.pixel_size); // 0x5cae90
    let out_bytes = cursor as i32; // 5c740a: cursor - textStart
    let out_width = (f64::from(esc_sum) + f64::from(last_adv)) * (f64::from(size) / px); // 5c73ff..741b
    let next = skip_trailing_ws(text, cursor); // 5c741f
    (out_bytes, out_width as f32, next as i32)
}

/// One proportional (word-wrap) step result: `(*out bytes that fit, *outWidth, *outNext, *outBrokeFlag)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropStep {
    pub out_bytes: i32,
    pub width: f32,
    pub next: i32,
    pub broke: u8,
}

/// `0x5c7470` — find how much of `text` fits one line within `box_width_frac`, honoring break
/// opportunities (`line_break_class`, 0x5c7780). `broke_in` is the caller's initial `*outBrokeFlag` byte
/// (it gates whether the first break records its position). Covers: size-default, pen seed, threshold
/// `ceil((pixelSize/ScreenToPixelHeight(size))·boxFrac·WIDTH)` (0x73fdf5 = **ceil**), the fit test
/// `(kern + adv + run) > threshold` (stop, don't consume), the break-opportunity tracking (break_pos +
/// break_width), and the three end modes (hard end at newline/eos; break at a recorded opportunity;
/// hard break mid-word setting `*outBrokeFlag = (flags>>6)&1`), each scaled by `ScreenToPixelHeight(size)/
/// pixelSize` over `(…)/WIDTH`.
#[allow(clippy::too_many_arguments)]
pub fn measure_run_proportional(
    ctx: &FontCtx,
    text: &[u8],
    size: f32,
    box_width_frac: f32,
    extra_spacing: f32,
    flags: u32,
    broke_in: u8,
) -> PropStep {
    let mut size = size;
    if size == 0.0 || flags & 0x4 != 0 {
        size = one_to_one_height(ctx.pixel_size, ctx.screen_h);
    }
    let mut run = pen_seed(extra_spacing, ctx.screen_w, ctx.font_flags); // [ebp-4]

    // threshold (5c74f5..5c7530): ceil((px / ScreenToPixelHeight(size)) · boxFrac · WIDTH)
    let hf = f64::from(screen_to_pixel_height(false, size, ctx.screen_h)); // ecx=0 always
    let px = f64::from(ctx.pixel_size);
    let threshold = ((px / hf) * f64::from(box_width_frac) * f64::from(ctx.screen_w)).ceil() as f32;

    let mut prev: u32 = 0; // [ebp-0x18]
    let mut last_adv: f32 = 0.0; // [ebp-0x20]
    let mut break_width: f32 = 0.0; // [ebp+0x1c]
    let mut break_pos: i32 = 0; // edi
    let mut broke = broke_in; // *outBrokeFlag
    let mut class: u32 = 2; // [ebp-0x24]
    let mut cursor = 0usize;

    if text.first().copied().unwrap_or(0) != 0 {
        loop {
            let dec = decode_quoted_code(&text[cursor..], flags); // 5c756f
            class = dec.class;
            if class == 2 {
                // 5c764d: newline — record width, advance past it, set break_pos
                break_width = (f64::from(last_adv) + f64::from(run)) as f32;
                cursor += dec.consumed as usize;
                break_pos = cursor as i32;
                break;
            }
            let glyph = class == 3 || class == 6;
            if !glyph {
                cursor += dec.consumed as usize; // 5c763f: skip control code
                if text.get(cursor).copied().unwrap_or(0) == 0 {
                    break;
                }
                continue;
            }
            if (ctx.glyph)(dec.out_code).is_none() {
                cursor += dec.consumed as usize; // 5c763f: glyph miss
                if text.get(cursor).copied().unwrap_or(0) == 0 {
                    break;
                }
                continue;
            }
            let kern_raw = if prev != 0 {
                (ctx.escapement)(prev, dec.out_code)
            } else {
                0.0 // 5c75ad
            };
            let kern = f64::from(kern_raw as f32); // [ebp-0x14] f32

            // break opportunity (5c75d8..5c7600)
            if crate::markup::line_break_class(prev, dec.out_code) != 0 {
                if broke != 0 || break_pos != 0 {
                    break_pos = cursor as i32; // edi = esi - textStart
                }
                broke = 1;
                break_width = (f64::from(last_adv) + f64::from(run)) as f32;
            }

            let after = cursor + dec.consumed as usize;
            let adv = char_advance(dec.out_code, &text[after..], size, flags, ctx); // raw st0
            let sum = (kern + adv) + f64::from(run); // 5c761b
            if sum > f64::from(threshold) {
                break; // 5c762b je 5c7664: stop, cursor NOT advanced past this token
            }
            run = (kern + f64::from(run)) as f32; // 5c762d: run += kern
            last_adv = adv as f32;
            prev = dec.out_code;
            cursor += dec.consumed as usize; // 5c763f
            if text.get(cursor).copied().unwrap_or(0) == 0 {
                break;
            }
        }
    }

    // END (5c7666)
    let width_div = f64::from(ctx.screen_w);
    let w1 = ((f64::from(last_adv) + f64::from(run)) * (f64::from(ONE) / width_div)) as f32; // [ebp-4]
    let w2 = (f64::from(break_width) * (f64::from(ONE) / width_div)) as f32; // [ebp+0x1c]
    let hf_end = f64::from(screen_to_pixel_height(false, size, ctx.screen_h)); // 0x5c6fa0, ecx=0
    let scale = hf_end / px; // hf/px; kept full in st0 for scaled_w1, also `fst dword [ebp+8]` for Case B
    let scale_f32 = scale as f32; // [ebp+8] (the f32 copy Case B multiplies w2 by)
    let scaled_w1 = (scale * f64::from(w1)) as f32; // 5c76be: full scale × w1
    let at_end = text.get(cursor).copied().unwrap_or(0) == 0;

    if class == 2 || at_end {
        // Case A (5c7744): hard end at newline / consumed everything
        PropStep {
            out_bytes: cursor as i32,
            width: scaled_w1,
            next: cursor as i32,
            broke,
        }
    } else if break_pos != 0 {
        // Case B (5c76d0): break at a recorded opportunity (uses the f32 scale copy × w2)
        let next = skip_trailing_ws(text, break_pos as usize);
        PropStep {
            out_bytes: break_pos,
            width: (f64::from(scale_f32) * f64::from(w2)) as f32,
            next: next as i32,
            broke,
        }
    } else {
        // Case C (5c76ea): hard break mid-word, no opportunity
        let next = skip_trailing_ws(text, cursor);
        PropStep {
            out_bytes: cursor as i32,
            width: scaled_w1,
            next: next as i32,
            broke: ((flags >> 6) & 1) as u8,
        }
    }
}

/// `0x5c7260` — ComputeStep dispatch + validation: routes to [`measure_run_fixedwidth`] (when
/// `flags&0x80`) or [`measure_run_proportional`], or zeroes the outputs when `size<0 || box<0 || text
/// empty`. Returned as a [`PropStep`] (the fixed leg leaves `broke = broke_in`, since 0x5c7300 writes no
/// break flag). This is the line-measure the C-API kernels (0x5c2070/0x5c21c0) drive in a loop.
#[allow(clippy::too_many_arguments)]
fn compute_step_dispatch(
    ctx: &FontCtx,
    text: &[u8],
    size: f32,
    box_width_frac: f32,
    extra_spacing: f32,
    flags: u32,
    broke_in: u8,
) -> PropStep {
    // 5c7263: size>=0 && !(box<0) && text && *text
    if !(size >= 0.0) || box_width_frac < 0.0 || text.first().copied().unwrap_or(0) == 0 {
        return PropStep {
            out_bytes: 0,
            width: 0.0,
            next: 0,
            broke: broke_in,
        };
    }
    if flags & 0x80 != 0 {
        let (out_bytes, width, next) = measure_run_fixedwidth(ctx, text, size, flags);
        PropStep {
            out_bytes,
            width,
            next,
            broke: broke_in,
        }
    } else {
        measure_run_proportional(
            ctx,
            text,
            size,
            box_width_frac,
            extra_spacing,
            flags,
            broke_in,
        )
    }
}

/// `0x5c2070` — `GxuFontGetTextBlockHeight`: the multi-line block height. Counts lines `N` by driving
/// [`compute_step_dispatch`] (a newline token advances by its consumed length; any other token measures a
/// line and jumps to `*outNext`; a no-progress result, `flags&2`, or a NULL continuation ends the walk),
/// then returns **`N·size + (N−1)·ceil(lineGap·screenH)/screenH`** in screen-fraction units (`N==0 ⇒ 0`).
/// `state_extra` is the `*state` continuation the binary forwards as the line-measure's extraSpacing.
#[allow(clippy::too_many_arguments)]
pub fn font_textblock_height(
    ctx: &FontCtx,
    text: &[u8],
    size: f32,
    box_width: f32,
    state_extra: f32,
    line_gap: f32,
    flags: u32,
) -> f32 {
    let mut size = size;
    if flags & 0x4 != 0 {
        size = one_to_one_height(ctx.pixel_size, ctx.screen_h);
    }
    let mut lines: i32 = 0; // edi
    let mut cursor = 0usize;
    let mut broke: u8 = 1; // [ebp+0x1b] init 1, persists across lines
    loop {
        if text.get(cursor).copied().unwrap_or(0) == 0 {
            break; // 5c20c0
        }
        let dec = decode_quoted_code(&text[cursor..], flags); // 5c20d1
        let next_cursor: usize = if dec.class == 2 {
            cursor + dec.consumed as usize // 5c20db: newline advance
        } else {
            let step = compute_step_dispatch(
                ctx,
                &text[cursor..],
                size,
                box_width,
                state_extra,
                flags,
                broke,
            );
            broke = step.broke; // the brokeFlag byte persists into the next line measure
            let n = cursor + step.next as usize;
            if n == cursor {
                break; // 5c2112: no progress
            }
            n
        };
        lines += 1; // 5c2116
        cursor = next_cursor;
        if flags & 0x2 != 0 {
            break; // 5c2117
        }
        if cursor == 0 {
            break; // 5c211e (never trips for a real cursor)
        }
    }
    if lines == 0 {
        return 0.0; // 5c2129
    }
    // 5c2137: gap_px = ceil(lineGap · screenH); height = N·size + gap_px/screenH · (N−1)
    let sh = f64::from(ctx.screen_h);
    let gap_px = ftol(f64::from(line_gap) * sh + f64::from(CEIL_BIAS)); // 0.99999 bias → ceil
    let term = (f64::from(gap_px) / sh) * f64::from(lines - 1); // fidivr screenH; fimul (N-1)
    let height = f64::from(lines) * f64::from(size) + term; // fmul size; faddp
    height as f32
}

/// `0x5c21c0` — `GxuFontGetMaxCharsWithinHeight`: how many chars fit in a height box. Guards
/// (`lineH/wrapW/boxH/p6/screenH` all non-zero, text non-empty) ⇒ 0. Quantizes `gap` to a whole-pixel
/// fraction `ceil(gap·screenH)/screenH`, then lays lines via [`compute_step_dispatch`]: stop when a line
/// consumes 0, or when `boxH + 2⁻²⁰ < accumH + lineH`; else `charCount += consumed`, `accumH += lineH +
/// gap`, and stop on a hard end (`brokeFlag==0`) or end-of-string. `broke_in` is the initial brokeFlag (1
/// in the binary). Returns `charCount`.
#[allow(clippy::too_many_arguments)]
pub fn font_height_fit(
    ctx: &FontCtx,
    text: &[u8],
    line_h: f32,
    wrap_w: f32,
    box_h: f32,
    p6: i32,
    p7_extra: f32,
    gap: f32,
    flags: u32,
    broke_in: u8,
) -> i32 {
    if text.first().copied().unwrap_or(0) == 0
        || line_h == 0.0
        || wrap_w == 0.0
        || box_h == 0.0
        || p6 == 0
        || ctx.screen_h == 0
    {
        return 0;
    }
    let mut line_h = line_h;
    if flags & 0x4 != 0 {
        line_h = one_to_one_height(ctx.pixel_size, ctx.screen_h);
    }
    // quantize gap (5c2247): `__ftol(gap·screenH + 0.99999)` then **`fild qword{lo=eax, hi=0}`** — the
    // int is ZERO-EXTENDED to 64-bit, so a negative ftol result reads back as a huge u32 (a faithful
    // binary quirk; cf. textblock_height's `fidivr m32int`, which is signed). Then `/screenH`.
    let sh = f64::from(ctx.screen_h);
    let gap_int = ftol(f64::from(gap) * sh + f64::from(CEIL_BIAS)) as u32;
    let gap_q = (f64::from(gap_int) / sh) as f32;
    let mut accum_h: f32 = 0.0; // [ebp-4]
    let mut char_count: i32 = 0; // [ebp-8]
    let mut cursor = 0usize;
    let mut broke = broke_in; // [ebp+0x17] init 1, persists across lines
    loop {
        let step =
            compute_step_dispatch(ctx, &text[cursor..], line_h, wrap_w, p7_extra, flags, broke);
        broke = step.broke;
        let consumed = step.next; // outNext - cursor
        if consumed == 0 {
            break; // 5c22c0
        }
        // 5c22c8: break when boxH + eps < accumH + lineH
        let lhs = f64::from(box_h) + f64::from(HEIGHT_FIT_EPS);
        let rhs = f64::from(accum_h) + f64::from(line_h);
        if lhs < rhs {
            break;
        }
        char_count += consumed; // 5c22e3
        accum_h = (f64::from(line_h) + f64::from(gap_q) + f64::from(accum_h)) as f32; // 5c22da
        cursor += consumed as usize;
        if step.broke == 0 {
            break; // 5c22f5: hard end
        }
        if text.get(cursor).copied().unwrap_or(0) == 0 {
            break; // 5c22f7
        }
    }
    char_count
}

/// `0x5c2320` — `GxuFontGetMaxCharsWithinWidth`: how many chars fit a width box, by measuring the
/// string at an effectively-infinite wrap (`10000.0`) via [`wrap_max_chars`] (filling a per-char
/// cumulative array + the total), then a subtractive fit search. Guards (`text`/`*text`/`n_max`/`box_w`)
/// ⇒ 0. If `total ≤ box_w` all `cnt` chars fit (`*outW = total`). Else walk the array for the first `i`
/// with `total − box_w < cum[i]`, returning `cnt − i` and `*outW = total − cum[i]`; no such `i` ⇒ 0,
/// `*outW = 0`. Returns `(count, *outW)`.
#[allow(clippy::too_many_arguments)]
pub fn font_width_fit(
    ctx: &FontCtx,
    text: &[u8],
    size: f32,
    box_w: f32,
    n_max: i32,
    extra_spacing: f32,
    flags: u32,
) -> (u32, f32) {
    if text.first().copied().unwrap_or(0) == 0 || n_max == 0 || box_w == 0.0 {
        return (0, 0.0);
    }
    let mut cum: Vec<f32> = Vec::new();
    let (cnt, total, _bytes) = wrap_max_chars(
        ctx,
        text,
        size,
        10_000.0, // 0x461c4000
        n_max,
        flags,
        extra_spacing,
        Some(&mut cum),
    );
    if !(total > box_w) {
        // 5c239e: total <= box_w (or unordered) → all fit
        return (cnt as u32, total);
    }
    if cnt == 0 {
        return (0, 0.0); // 5c23d4
    }
    let diff = (f64::from(total) - f64::from(box_w)) as f32; // [ebp+0xc] = total - boxW
    for (i, &c) in cum.iter().enumerate().take(cnt as usize) {
        if c > diff {
            // 5c23d8: first cum[i] > total-boxW
            let out_w = (f64::from(total) - f64::from(c)) as f32;
            return ((cnt - i as i32) as u32, out_w);
        }
    }
    (0, 0.0) // 5c23ea
}

/// The current screen-fraction viewport rect a [`compute_viewport_rect`] call snaps, as queried from the
/// gx device (`0x58b060`): the four edge fractions and the half-pixel-offset selector
/// (`0x58a230()[4] == 0` ⇒ offsets `(-0.5, +0.5)`).
#[derive(Clone, Copy, Debug)]
pub struct ViewportQuery {
    pub x0_frac: f32,
    pub x1_frac: f32,
    pub y0_frac: f32,
    pub y1_frac: f32,
    pub half_offset: bool,
}

/// `0x5c70b0` — snap the screen-fraction viewport rect to integer device pixels (the values then
/// installed via `0x5c3d90(x0,x1,y0,y1,-1,+1)`). Floating point: `floor(WIDTH·xfrac)` / `floor(HEIGHT·
/// yfrac)` per edge (0x73ff3f = **floor**), the empty-rect guard (`x1 > x0` and `y1 > y0`, else `None`),
/// and the `offset + rounded` edges. The rect query (`0x58b060`) and the install
/// (`0x5c3d90`) are gx-device calls (`[0xc0ed38]`) that the emulated harness cannot isolate, so this
/// function has no test; the query result is the input here. Returns `Some([x0,x1,y0,y1])` (the install args) or `None` (degenerate).
pub fn compute_viewport_rect(q: &ViewportQuery, screen_w: i32, screen_h: i32) -> Option<[f32; 4]> {
    let rx0 = (f64::from(screen_w) * f64::from(q.x0_frac)).floor() as f32; // 0x73ff3f
    let rx1 = (f64::from(screen_w) * f64::from(q.x1_frac)).floor() as f32;
    if !(rx1 > rx0) {
        return None; // 5c71b1: empty-rect guard
    }
    let ry0 = (f64::from(screen_h) * f64::from(q.y0_frac)).floor() as f32;
    let ry1 = (f64::from(screen_h) * f64::from(q.y1_frac)).floor() as f32;
    if !(ry1 > ry0) {
        return None; // 5c7201
    }
    let (offx, offy) = if q.half_offset {
        (-0.5f32, 0.5f32) // 0xbf000000, 0x3f000000
    } else {
        (0.0f32, 0.0f32)
    };
    Some([
        (f64::from(offx) + f64::from(rx0)) as f32, // 5c723e
        (f64::from(offx) + f64::from(rx1)) as f32, // 5c7234
        (f64::from(offy) + f64::from(ry0)) as f32, // 5c722a
        (f64::from(offy) + f64::from(ry1)) as f32, // 5c7215
    ])
}
