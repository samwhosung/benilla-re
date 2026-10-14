//! The CGxString quad, justify and clamp geometry of the text-layout pipeline. Each `pub fn` cites
//! its source address.
//!
//! The x87 runs at 53-bit precision, so every `f*` operation rounds to `f64` and an `fstp m32` store
//! rounds to `f32`. Each operation is transcribed as `f64` arithmetic with an `as f32` at exactly the
//! exe's f32 stores; integer loads (`fild`) are `i32 as f64`; the CRT `__ftol` (truncate toward zero,
//! `0x40a2b0`) is [`crate::ftol`].
//! The CRT `floor` is `0x73ff3f`, matched here by `f64::floor` for the in-range integer-valued
//! results these functions floor.
//!
//! ## Scope: the eight CGxString layout and quad functions
//! `resolve_clamp_size` (0x5ccb80), `cgxstring_size_clamp` (0x5cd6d0), `anchor_justify_snap` (0x5cdf70),
//! `hyperlink_region_rect` (0x5cd310), `selection_highlight_fade` (0x5cd4d0), `glyph_quad_emit`
//! (0x5c8710), plus the two multi-line drivers `glyph_quad_build` (0x5ccbe0, the per-glyph textured
//! quad) and `string_layout_lines` (0x5cdc20, the line-step, horizontal-justify and wrap-px driver).
//! The two drivers compose upstream kernels that run for real in the emulator, `0x5c2810`
//! markup-classify ([`crate::markup::decode_quoted_code`]) and `0x5c7260` line-measure
//! ([`crate::measure::measure_run_fixedwidth`], [`measure_run_proportional`](crate::measure)), so
//! they are tested whole: `glyph_quad_build` reads back the four AUVERT vertices the exe writes (the
//! gx vertex-buffer lock skipped through a pre-sized colour array, the glyph lookup `0x5cabd0`
//! stubbed to a synthesized CharCodeDesc), and `string_layout_lines` reads back the line count plus,
//! composed through `glyph_quad_build` (run for real), each line's emitted quad geometry.

// `!(a < b)` / `!(a >= b)` are the precise transcription of the binary's `fcomp` + conditional jump:
// the *unordered* (NaN) case must take the same branch as the binary's `jp` (terminate / degenerate),
// which `a >= b` / `a < b` would not (false for NaN). Each such site mirrors a specific `jp` — keep the
// form. (Same rationale + allow as `measure`.)
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use crate::ftol;
use crate::markup::decode_quoted_code;
use crate::measure::{measure_run_fixedwidth, measure_run_proportional, FontCtx, PropStep};
use crate::metric::glyph_width_scaled;
use crate::screen_pixel::{screen_to_pixel_height, screen_to_pixel_width};

// ── module float constants (raw f32 bits from `WoW.exe` .rdata) ───────────────────────────────────
/// `0x801628` = 2.0 — size-clamp numerator (`2.0/viewH`).
const C_2_0: f32 = 2.0; // 0x40000000
/// `0x7ff9d8` = 1.0 — hyperlink-rect normalizer numerator (`1.0/viewH`, `1.0/viewW`).
const C_1_0: f32 = 1.0; // 0x3f800000
/// `0x7ffa24` = 0.5 — justify-center factor.
const C_0_5: f32 = 0.5; // 0x3f000000
/// `0x808120` = 0.99999 — ceil-via-truncation bias (`x + 0.99999` then `__ftol`).
const C_CEIL_BIAS: f32 = f32::from_bits(0x3f7f_ff58); // 0x808120

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// size clamp — `clamped = max(size, 2.0/viewH)`
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The shared minimum-size floor (the arithmetic of both `0x5ccb80` and `0x5cd6d0`). Bytes
/// (`5ccb9a`/`5cd7ee`): `fild qword[screenH]; fdivr dword[0x801628]=2.0` → `2.0/viewH`;
/// `fld dword[+0x18]` (size); `fcomp st(1); fnstsw; test ah,0x41`. The `test` keeps the floor on
/// `st0` unless `size > floor` strictly (ties and unordered keep the floor), then `fstp dword[+0x1c]`.
/// So `clamped = if size > 2.0/viewH { size } else { 2.0/viewH }`.
fn min_size_floor(size: f32, view_h: i32) -> f32 {
    let floor = f64::from(C_2_0) / f64::from(view_h); // fdivr [2.0] / (double)viewH
    if f64::from(size) > floor {
        size // fld [+0x18]; fstp dword[+0x1c]  (identity round-trip)
    } else {
        floor as f32 // fstp dword[+0x1c]
    }
}

/// `CGxString::ResolveClampSize` (`0x5ccb80`): resolve the size (default via `0x5c2540` when
/// flags&4) then floor it to a two-device-pixel minimum, written to `+0x1c`. The clamp is
/// the transcribed part; `size` here is the already-resolved value at `+0x18` (the flags&4 default
/// pick, `GxuFontGetOneToOneHeight` = `pixelSize/viewH`, is upstream of it).
pub fn resolve_clamp_size(size: f32, view_h: i32) -> f32 {
    min_size_floor(size, view_h)
}

/// `CGxString::Setup`/`Initialize` (`0x5cd6d0`): the size-clamp the SetText path owns. After the
/// strcpy / param-store / list-splice orchestration, `+0x18` is set to the default
/// (`0x5c2540`, when `flags&4 && (s8)flags >= 0`) or to the `size` argument, then floored to
/// `+0x1c = max(+0x18, 2.0/viewH)` — byte-identical to [`resolve_clamp_size`]'s clamp
/// (`5cd7ee..5cd818` == `5ccb9a..5ccbc5`). `size` here is the already-resolved `+0x18`.
pub fn cgxstring_size_clamp(size: f32, view_h: i32) -> f32 {
    min_size_floor(size, view_h)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// anchor / justify / pixel-snap — `0x5cdf70`
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The CGxString fields `anchor_justify_snap` reads (struct offsets in parens).
#[derive(Clone, Copy, Debug)]
pub struct AnchorState {
    /// normalized anchor position x/y/z (`+0x20`/`+0x24`/`+0x28`).
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub anchor_z: f32,
    /// laid-out count guard (`+0x9c`): 0 ⇒ snap skipped (anchor = raw copy).
    pub n: i32,
    /// flags byte (`+0x5c`): sign bit (0x80 "pre-snapped") ⇒ snap skipped.
    pub flags: u8,
    /// **vertical** justify (`+0x50`) — selects the anchor.y offset (block-height centering).
    pub justify_v: u32,
    /// **horizontal** justify (`+0x54`) — selects the anchor.x offset (pos_x·factor).
    pub justify_h: u32,
    /// position x/y (`+0x3c`/`+0x40`).
    pub pos_x: f32,
    pub pos_y: f32,
    /// clamped size (`+0x1c`) and wrap-box width (`+0x58`).
    pub clamped_size: f32,
    pub wrap_width: f32,
}

/// `CGxString::ComputeAnchor` (`0x5cdf70`): the laid-out string's origin → justify offsets →
/// viewW/viewH pixel-snapped screen anchor (`+0x70`/`+0x74`/`+0x78`). Returns `(x, y, z)`.
///
/// Bytes: copy `+0x20..+0x28` → `+0x70..+0x78` (raw); bail (anchor = copy) if `+0x9c == 0`
/// (`5cdf9e`) or `(s8)+0x5c < 0` (`5cdfa9`). Horizontal (uses `justify_h`/`+0x54`):
/// `==2 → anchor.x += pos_x`; `==1 → anchor.x += pos_x*0.5`; else unchanged (`5cdfb2..5cdfcc`).
/// Block height `h = (linesPx/viewH)*(n-1) + n*clampedSize`, where
/// `linesPx = __ftol(viewH*wrap + 0.99999)` (`5cdfde..5ce00e`). Vertical (uses `justify_v`/`+0x50`):
/// `==0 → anchor.y = pos_y + anchor.y`; `==1 → anchor.y = ((pos_y - h)*0.5 + anchor.y) + h`;
/// else `anchor.y = h + anchor.y` (`5ce000..5ce07e`). Snap: `anchor.x = floor(viewW·anchor.x)`,
/// `anchor.y = floor(viewH·anchor.y)`, `anchor.z = 0` (`5ce01d..5ce05c`; floor = `0x73ff3f`).
pub fn anchor_justify_snap(s: &AnchorState, view_w: i32, view_h: i32) -> (f32, f32, f32) {
    // 5cdf83: raw copy of the normalized anchor +0x20/24/28 → +0x70/74/78.
    let copy_x = s.anchor_x;
    let copy_y = s.anchor_y;
    let copy_z = s.anchor_z;
    // 5cdf9e / 5cdfa9: nothing built, or pre-snapped (flags sign bit) ⇒ anchor = copy.
    if s.n == 0 || (s.flags as i8) < 0 {
        return (copy_x, copy_y, copy_z);
    }

    // 5cdfb2: horizontal justify (justify_h) → anchor.x (f32 store).
    let ax: f32 = match s.justify_h {
        2 => (f64::from(s.pos_x) + f64::from(copy_x)) as f32, // fld pos_x; fadd anchor.x
        1 => (f64::from(s.pos_x) * f64::from(C_0_5) + f64::from(copy_x)) as f32,
        _ => copy_x, // unchanged
    };

    // 5cdfde: linesPx = __ftol(viewH*wrap + 0.99999); h = (linesPx/viewH)*(n-1) + n*clamped.
    let lines_px = ftol(f64::from(view_h) * f64::from(s.wrap_width) + f64::from(C_CEIL_BIAS));
    let h = (f64::from(lines_px) / f64::from(view_h)) * f64::from(s.n.wrapping_sub(1))
        + f64::from(s.n) * f64::from(s.clamped_size);

    // 5ce000: vertical justify (justify_v) → anchor.y (f32 store).
    let ay: f32 = match s.justify_v {
        0 => (f64::from(s.pos_y) + f64::from(copy_y)) as f32, // pos_y + anchor.y (h discarded)
        1 => ((((f64::from(s.pos_y) - h) * f64::from(C_0_5)) + f64::from(copy_y)) + h) as f32,
        _ => (h + f64::from(copy_y)) as f32, // ==2 (and any other) → h + anchor.y
    };

    // 5ce01d: pixel snap (floor = 0x73ff3f), anchor.z = 0.
    let snap_x = (f64::from(view_w) * f64::from(ax)).floor() as f32;
    let snap_y = (f64::from(view_h) * f64::from(ay)).floor() as f32;
    (snap_x, snap_y, 0.0)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// hyperlink hit-region rect — `0x5cd310`
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// `CGxString::AddHyperlinkRegion` (`0x5cd310`): the normalization of a hyperlink hit-region
/// rect before it is appended (as a 32-byte record) to the `+0x7c` growable array. Bytes
/// (`5cd316..5cd369`): `invH = 1.0/viewH`, `invW = 1.0/viewW` (each `fdivr dword[0x7ff9d8]=1.0`),
/// then `a*=invH` (`+0x08`), `c*=invH` (`+0x10`), `b*=invW` (`+0x0c`), `d*=invW` (`+0x14`). The four
/// inputs `(a,b,c,d)` are the rect coords; the y-coords (`a,c`) divide by viewH, the x-coords (`b,d`)
/// by viewW. Returns `(a/viewH, b/viewW, c/viewH, d/viewW)` — the array record's first four floats
/// (the remaining four dwords of the record are copied through unchanged by the `rep movsd`).
///
/// Note `a * (1.0/viewH)` is NOT `a / viewH` — the `1.0/viewH` reciprocal rounds first, then
/// multiplies (the binary's `fdivr` + `fmul`), so the reciprocal is computed once and reused.
pub fn hyperlink_region_rect(
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    view_h: i32,
    view_w: i32,
) -> (f32, f32, f32, f32) {
    let inv_h = f64::from(C_1_0) / f64::from(view_h); // fild viewH; fdivr [1.0]
    let na = (f64::from(a) * inv_h) as f32; // +0x08
    let nc = (f64::from(c) * inv_h) as f32; // +0x10
    let inv_w = f64::from(C_1_0) / f64::from(view_w); // fild viewW; fdivr [1.0]
    let nb = (f64::from(b) * inv_w) as f32; // +0x0c
    let nd = (f64::from(d) * inv_w) as f32; // +0x14
    (na, nb, nc, nd)
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// selection-highlight alpha fill with per-pair fade — `0x5cd4d0`
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// The outcome of [`selection_highlight_fade`]: the binary's return code plus the per-cell alpha byte
/// it writes into each of the `cell_count` selection cells (via `SetCell` 0x5cd460).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionFill {
    /// Return value: `2` for an early-out (negative col, or `4*startCol >= cell_count`); otherwise
    /// `(cell_count < 4*startCol + 4*endCol) as u32`.
    pub ret: u32,
    /// Alpha byte written to cell `i` (`i in 0..cell_count`).
    pub alpha: Vec<u8>,
}

/// `CGxString::FillSelection` (`0x5cd4d0`): the selection-rectangle alpha fill with per-pair fade.
/// This transcribes the **active** path (`startCol >= 0`, `endCol >= 0`, range changed, `+0x90 != 0`,
/// `+0x9c != 0`): the early-out / cached-range / rebuild branches are orchestration around it.
///
/// Bytes: `bVar2 = +0x2f` (base alpha); `bVar4 = __ftol((double)bVar2 / endCol)` (the fade step,
/// `5cd55c`); `fourStart = startCol*4`. Over `i in 0..cellCount` (`5cd581`): if `i < fourStart` →
/// write `bVar2` unchanged; else if `bVar2 == 0` or `fourStart + 4*endCol <= i` → write `bVar2`
/// (re-paint trailing cells, breaking if the cell's current byte is 0); else (inside selection) →
/// write `bVar2`, and on odd `i` fade `bVar2 = if bVar4 < bVar2 { bVar2 - bVar4 } else { 0 }`.
/// Return (`5cd615`): `2` if `fourStart >= cellCount`, else `(cellCount < fourStart + 4*endCol)`.
///
/// `endCol == 0` divides by zero: `bVar2/endCol` is +inf (NaN for a base alpha of 0), and `__ftol`
/// returns the low dword of its qword, 0, for either, so the fade step is 0.
pub fn selection_highlight_fade(
    start_col: i32,
    end_col: i32,
    base_alpha: u8,
    cell_count: i32,
) -> SelectionFill {
    if start_col < 0 || end_col < 0 {
        return SelectionFill {
            ret: 2,
            alpha: Vec::new(),
        };
    }
    let n = cell_count.max(0) as usize;
    let mut alpha = vec![0u8; n];
    let mut bvar2 = base_alpha;
    // 5cd55c: bVar4 = __ftol((double)baseAlpha / (double)endCol), low byte.
    let bvar4 = (ftol(f64::from(u32::from(base_alpha)) / f64::from(end_col)) as u32) as u8;
    let four_start = start_col.wrapping_mul(4);
    let four_end = end_col.wrapping_mul(4);

    if cell_count > 0 {
        let mut i: i32 = 0;
        loop {
            // The write value is bVar2 as it stands at the SetCell call (before the odd-i fade).
            let writeval;
            if i < four_start {
                writeval = bvar2; // before selection: base alpha, unchanged
            } else if bvar2 == 0 || four_start.wrapping_add(four_end) <= i {
                // after selection / exhausted: re-paint (GetCell != 0 in-domain ⇒ no break)
                writeval = bvar2;
            } else {
                // inside selection
                writeval = bvar2;
                if i & 1 != 0 {
                    // 5cd5ca: `cmp bVar2,bVar4; ja sub; else 0` = `bVar2 > bVar4 ? bVar2-bVar4 : 0`
                    // (equivalently `bVar2.saturating_sub(bVar4)`: they agree at bVar2==bVar4 → 0).
                    bvar2 = bvar2.saturating_sub(bvar4);
                }
            }
            alpha[i as usize] = writeval;
            i += 1;
            if i >= cell_count {
                break;
            }
        }
    }

    let ret = if four_start < cell_count {
        u32::from(cell_count < four_start.wrapping_add(four_end))
    } else {
        2
    };
    SelectionFill { ret, alpha }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// per-glyph quad emit (pen offset + colour) — `0x5c8710`
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// One source (pre-laid) glyph-quad vertex (AUVERT, stride 0x14): `{x, y, z, u, v}`.
#[derive(Clone, Copy, Debug)]
pub struct EmitVertexIn {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub u: f32,
    pub v: f32,
}

/// One emitted output vertex (stride 0x18): `{x, y, z, color, u, v}`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmitVertexOut {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: u32,
    pub u: f32,
    pub v: f32,
}

/// `CGxStringBatch::EmitGlyphQuads` pen base (`0x5c8710`, `5c8775..5c87c5`): the integer pen origin
/// the glyph quads are translated by. Bytes: `penX = floor(ScreenToPixelWidth(snap.x)) + pen.x`,
/// `penY = floor(ScreenToPixelHeight(snap.y)) + pen.y` — `ScreenToPixelWidth`/`Height` are `0x5c7010`
/// /`0x5c6fa0` (the `0x80` raw-px flag clear), floor is `0x73ff3f`. `snap` is the sub-pixel snap
/// vector (`arg4`), `pen` is the pen offset (`arg6`). The floored width is round-tripped through
/// an `f32` store before the add (`5c879b`); the height is not (`5c879e`) — both identity for the
/// integer-valued floor results.
pub fn glyph_quad_emit_pen(
    snap_x: f32,
    snap_y: f32,
    pen_x: f32,
    pen_y: f32,
    view_w: i32,
    view_h: i32,
) -> (f32, f32) {
    let spw = screen_to_pixel_width(false, snap_x, view_w); // 0x5c7010
    let penx_floor = f64::from(spw).floor() as f32; // 0x73ff3f; fstp dword[+0x10]
    let penx = (f64::from(penx_floor) + f64::from(pen_x)) as f32; // fld [+0x10]; fadd pen.x
    let sph = screen_to_pixel_height(false, snap_y, view_h); // 0x5c6fa0
    let peny = (f64::from(sph).floor() + f64::from(pen_y)) as f32; // floor; fadd pen.y
    (penx, peny)
}

/// `CGxStringBatch::EmitGlyphQuads` per-vertex transform (`0x5c8710`, `5c8800..5c8842`): translate a
/// pre-laid quad vertex by the pen origin and copy its UVs. `out.x = in.x + penX`,
/// `out.y = penY + in.y`, `out.z = in.z`, `out.u = in.u`, `out.v = in.v`. (Colour is assembled by
/// [`glyph_quad_emit_color`].)
pub fn glyph_quad_emit_vertex(
    v: EmitVertexIn,
    pen_x: f32,
    pen_y: f32,
    color: u32,
) -> EmitVertexOut {
    EmitVertexOut {
        x: (f64::from(v.x) + f64::from(pen_x)) as f32, // in.x + penX
        y: (f64::from(pen_y) + f64::from(v.y)) as f32, // penY + in.y
        z: v.z,
        color,
        u: v.u,
        v: v.v,
    }
}

/// `CGxStringBatch::EmitGlyphQuads` per-vertex colour (`0x5c8710`, `5c8847..5c88af`), the no-fade
/// case (`arg8 == 0` or no gradient). The base colour `arg5[0]` is written as-is unless the
/// device wants a channel swap (`(*0x5c8230())[+0x1c] == 1`): then byte0 and byte2 (R↔B) are
/// exchanged (`out = {c2, c1, c0, c3}`), byte3 (alpha) preserved.
pub fn glyph_quad_emit_color(base: u32, swap: bool) -> u32 {
    if swap {
        let b0 = base & 0xff;
        let b1 = (base >> 8) & 0xff;
        let b2 = (base >> 16) & 0xff;
        let b3 = (base >> 24) & 0xff;
        b2 | (b1 << 8) | (b0 << 16) | (b3 << 24)
    } else {
        base
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// per-glyph textured quad — `glyph_quad_build` (0x5ccbe0)
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// `0x80306c` = 4.0 — line-height pad when font.flags(+0x180) bit3 (outline) set.
const C_4_0: f32 = 4.0; // 0x40800000
/// `0xc2b9f8`/`9fc` = `+1/512` (`0x3b000000`) — shadow/pre-snapped UV texel nudge (v_bottom, u_left).
const C_TEXEL_POS: f32 = f32::from_bits(0x3b00_0000); // +0.001953125
/// `0xc2ba00`/`a04` = `-1/512` (`0xbb000000`) — shadow/pre-snapped UV texel nudge (v_top, u_right).
const C_TEXEL_NEG: f32 = f32::from_bits(0xbb00_0000); // -0.001953125

/// One emitted AUVERT quad vertex (stride 0x14): `{x, y, z, u, v}`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub u: f32,
    pub v: f32,
}

/// The 4-vertex textured quad [`glyph_quad_build`] emits for one glyph, plus the per-vertex colour word.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphQuad {
    /// The 4 corners in the binary's emit order: `v0`=top-left, `v1`=bottom-left, `v2`=top-right,
    /// `v3`=bottom-right (`u` = `u_left`/`u_right` per column, `v` = `v_top`/`v_bottom` per row).
    pub verts: [QuadVertex; 4],
    /// The colour word written 4× into the page colour list (`= *colorPtr`, verbatim).
    pub color: u32,
}

/// The resolved per-glyph inputs `glyph_quad_build` reads (CGxString `+off` / CGxFont `+off` /
/// CharCodeDesc `+off`).
#[derive(Clone, Copy, Debug)]
pub struct GlyphQuadInputs {
    /// Pen X = `origin[0]` (already includes any prior advance; first glyph of a run has prev=0 so the
    /// escapement·scale advance is 0, and the pen is `origin[0]` unchanged — the diffed single-glyph case).
    pub pen_x: f32,
    /// `origin[1]` — the line baseline Y.
    pub origin_y: f32,
    /// `origin[2]` — the depth, copied to every vertex's `z`.
    pub origin_z: f32,
    /// CGxString `+0x1c` clamped size (the render size fed to ScreenToPixelHeight + GlyphWidth).
    pub clamped_size: f32,
    /// CGxFont `+0x24c` rendered pixel size (the scale denominator).
    pub pixel_size: i32,
    /// CGxString `+0x5c` flags. Bit7 (`0x80`, the "pre-snapped/raw-px" path) selects: ScreenToPixelHeight/GlyphWidth raw-px passthrough, the `size_metric` (not
    /// `line_height`) vertical extent, the skipped cell-width floor, and the ±1/512 UV texel nudge.
    pub string_flags: u8,
    /// CGxFont `+0x180` flags. Bit3 (`0x8`, outline) → `line_height += 4.0`, `vpad = 2.0`; else bit0
    /// (`0x1`, shadow) → `line_height += 2.0`, `vpad = 1.0`.
    pub font_flags: u32,
    /// CharCodeDesc `+0x48` integer pixel advance (cell width before scaling; FILD m64int — read as u32).
    pub desc_advance: i32,
    /// CharCodeDesc `+0x50` glyph width metric (the GlyphWidth/placement input, 0x5cb080).
    pub desc_width: f32,
    /// CharCodeDesc `+0x58` glyph top bearing in rows (FILD m32int — signed).
    pub desc_top: i32,
    /// CharCodeDesc atlas-UV rect: `+0x60` v_bottom, `+0x64` u_left, `+0x68` v_top, `+0x6c` u_right.
    pub uv_v_bottom: f32,
    pub uv_u_left: f32,
    pub uv_v_top: f32,
    pub uv_u_right: f32,
    /// `*colorPtr` — the per-vertex colour word (the markup/string colour state; emitted verbatim).
    pub color: u32,
    /// `0xc2b9a0` — viewport pixel height.
    pub screen_h: i32,
}

/// `CGxString::BuildGlyphQuad` (`0x5ccbe0`): the per-glyph textured quad — corner positions
/// (pen+placement, cell width, line height), the atlas-UV copy with the pre-snapped ±1/512 nudge, and the
/// per-vertex colour. This transcribes the literal-glyph emit path (the `class > 5` jump-table fall-through
/// at `5ccec3`) for a single glyph of a run (prev glyph absent ⇒ the escapement·scale advance is 0).
///
/// Setup (`5ccc59..5ccccd`): `size_metric = ScreenToPixelHeight(raw, clampedSize)` (`5ccc74`),
/// `scale = size_metric / pixelSize` (`5ccc8a`: `fild qword pixelSize; fdivr size_metric`),
/// `line_height = size_metric` then `+4.0` (font outline) / `+2.0` (font shadow) (`5ccca0..5ccccd`).
/// `raw = (stringFlags & 0x80) != 0`.
///
/// Per glyph (`5cd039..5cd274`): `proto.x = GlyphWidth(raw,width,clampedSize,pixelSize) + penX`
/// (`5cd099`, 0x5cb080 = [`glyph_width_scaled`]); `cell_w = (u32)advance · scale`, floored when not raw
/// (`5cd0b6..5cd0d2`, 0x73ff3f = floor); `proto.y = top·scale + (originY − vpad)` where `vpad` = 2.0
/// (font outline) / 1.0 (font shadow) / 0 (`5cd0d8..5cd113`). The vertical extent `H` = `line_height`
/// (not raw) or `size_metric` (raw); corners `v0=(px,py)`, `v1=(px,py+H)`, `v2=(px+cell_w,py)`,
/// `v3=(px+cell_w,py+H)`, all `z = originZ` (`5cd1aa`/`5cd1cf`). UV copy (`5cd1fd..5cd274`): per-column
/// `u_left`/`u_right`, per-row `v_top`/`v_bottom`, with the raw nudge `v_bottom,u_left += 1/512` and
/// `v_top,u_right += -1/512` (`5cd21b..5cd248`). Colour = `*colorPtr` written 4× (`5ccf62`).
pub fn glyph_quad_build(inp: &GlyphQuadInputs) -> GlyphQuad {
    // 5ccc60/5cccd: `raw` = stringFlags bit7 — the "pre-snapped/raw-px" path.
    let raw = inp.string_flags & 0x80 != 0;

    // 5ccc74 / 5ccca0: size_metric = ScreenToPixelHeight(raw, clampedSize) (called twice, identical).
    let size_metric = screen_to_pixel_height(raw, inp.clamped_size, inp.screen_h);
    // 5ccc8a: scale = size_metric / pixelSize (fild qword pixelSize; fdivr size_metric; fstp dword).
    let scale = (f64::from(size_metric) / f64::from(inp.pixel_size)) as f32;

    // 5ccca5..5ccccd: line_height = size_metric (+ font-flag pad).
    let mut line_height = size_metric;
    if inp.font_flags & 0x8 != 0 {
        line_height = (f64::from(line_height) + f64::from(C_4_0)) as f32; // +4.0 outline
    } else if inp.font_flags & 0x1 != 0 {
        line_height = (f64::from(line_height) + f64::from(C_2_0)) as f32; // +2.0 shadow
    }

    // 5cd099: proto.x = GlyphWidth + penX (escapement·scale advance is 0 for the first glyph of a run).
    let glyph_w = glyph_width_scaled(raw, inp.desc_width, inp.clamped_size, inp.pixel_size);
    let proto_x = (glyph_w + f64::from(inp.pen_x)) as f32;

    // 5cd0b6: cell_w = (u32)advance · scale (FILD m64int, hi=0 ⇒ zero-extended), floored when not raw.
    let mut cell_w = (f64::from(inp.desc_advance as u32) * f64::from(scale)) as f32;
    if !raw {
        cell_w = f64::from(cell_w).floor() as f32; // 5cd0ca: 0x73ff3f = floor
    }

    // 5cd0d8: proto.y = top·scale + (originY − vpad).
    let vpad: f64 = if inp.font_flags & 0x8 != 0 {
        f64::from(C_2_0) // 5cd0e5: fsub 2.0 (outline)
    } else if inp.font_flags & 0x1 != 0 {
        f64::from(C_1_0) // 5cd0f1: fsub 1.0 (shadow)
    } else {
        0.0
    };
    let oy = f64::from(inp.origin_y) - vpad;
    let proto_y = (f64::from(inp.desc_top) * f64::from(scale) + oy) as f32;

    // 5cd1aa/5cd1cf: corner positions. Vertical extent H = line_height (not raw) or size_metric (raw).
    let h = if raw { size_metric } else { line_height };
    let x_left = proto_x;
    let x_right = (f64::from(cell_w) + f64::from(proto_x)) as f32;
    let y_top = proto_y;
    let y_bot = (f64::from(h) + f64::from(proto_y)) as f32;
    let z = inp.origin_z;

    // 5cd21b: raw ⇒ nudge v_bottom,u_left += 1/512 ; v_top,u_right += -1/512.
    let (vb, ul, vt, ur) = if raw {
        (
            (f64::from(inp.uv_v_bottom) + f64::from(C_TEXEL_POS)) as f32,
            (f64::from(inp.uv_u_left) + f64::from(C_TEXEL_POS)) as f32,
            (f64::from(inp.uv_v_top) + f64::from(C_TEXEL_NEG)) as f32,
            (f64::from(inp.uv_u_right) + f64::from(C_TEXEL_NEG)) as f32,
        )
    } else {
        (inp.uv_v_bottom, inp.uv_u_left, inp.uv_v_top, inp.uv_u_right)
    };

    GlyphQuad {
        verts: [
            QuadVertex {
                x: x_left,
                y: y_top,
                z,
                u: ul,
                v: vt,
            }, // v0 top-left
            QuadVertex {
                x: x_left,
                y: y_bot,
                z,
                u: ul,
                v: vb,
            }, // v1 bottom-left
            QuadVertex {
                x: x_right,
                y: y_top,
                z,
                u: ur,
                v: vt,
            }, // v2 top-right
            QuadVertex {
                x: x_right,
                y: y_bot,
                z,
                u: ur,
                v: vb,
            }, // v3 bottom-right
        ],
        color: inp.color,
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// multi-line layout driver — `string_layout_lines` (0x5cdc20)
// ─────────────────────────────────────────────────────────────────────────────────────────────────

/// One content line's origin, as `string_layout_lines` passes it (the `origin` vec3) to
/// `glyph_quad_build` (0x5ccbe0): the horizontal-justify X-offset, the running baseline Y, and the depth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineOrigin {
    /// `origin[0]` — the per-line horizontal-justify X offset (`5cde28`, fed through ScreenToPixelWidth).
    pub vjust_x: f32,
    /// `origin[1]` — the running baseline Y (`line_metric` minus `k·line_step`).
    pub baseline_y: f32,
    /// `origin[2]` — the depth (anchor.z, or 0 when rotated).
    pub origin_z: f32,
}

/// The outputs of [`string_layout_lines`]: the laid-out line count (`+0x9c`), the constant per-line
/// baseline step / initial line metric, and each content line's origin (the args it hands `glyph_quad_build`).
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutLines {
    /// `+0x9c` — total laid-out line count (content + newline-only lines).
    pub count: i32,
    /// `[ebp-0x8]` — the constant baseline Y decrement applied per line.
    pub line_step: f32,
    /// `[ebp-0x44]` initial — the first line's baseline Y.
    pub line_metric: f32,
    /// One entry per CONTENT line (those that emit a quad via `glyph_quad_build`).
    pub lines: Vec<LineOrigin>,
}

/// The CGxString fields [`string_layout_lines`] reads (struct offsets).
#[derive(Clone, Copy, Debug)]
pub struct LayoutState {
    /// `+0x1c` clamped size.
    pub clamped_size: f32,
    /// `+0x58` wrap-box width.
    pub wrap_width: f32,
    /// `+0x3c` — the box width fed to the line measure (0x5c7260 arg2).
    pub box_width: f32,
    /// `+0x40` — the box-bottom the running pen-Y is tested against (loop terminates at `penY >= this`).
    pub box_bottom: f32,
    /// `+0x54` **horizontal** justify (selects the per-line X offset: 2 → lineWidth, 1 → lineWidth·0.5, else 0).
    pub justify_h: u32,
    /// `+0x5c` flags. Bit7 (`0x80`) = rotated/raw; bit0 (`0x1`) feeds `+0x34` as the measure extraSpacing;
    /// bit1 (`0x2`) = stop after the first content line.
    pub flags: u32,
    /// `+0x28` anchor.z (origin depth; forced 0 when rotated).
    pub anchor_z: f32,
    /// `+0x34` — the extraSpacing fed to the line measure when `flags & 1`.
    pub extra_spacing: f32,
    /// `0xc2b9a4` viewport width / `0xc2b9a0` viewport height.
    pub screen_w: i32,
    pub screen_h: i32,
}

/// The line measure dispatch `0x5c7260` (`compute_step_dispatch` in `measure`, re-inlined here since
/// that fn is private): route to fixed-width / proportional, or zero the outputs for the degenerate guard.
/// Identical to the binary `string_layout_lines` calls at `5cddd3`.
fn measure_line(
    ctx: &FontCtx,
    text: &[u8],
    size: f32,
    box_width: f32,
    extra: f32,
    flags: u32,
    broke_in: u8,
) -> PropStep {
    if !(size >= 0.0) || box_width < 0.0 || text.first().copied().unwrap_or(0) == 0 {
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
        measure_run_proportional(ctx, text, size, box_width, extra, flags, broke_in)
    }
}

/// `CGxString::LayoutLines` (`0x5cdc20`): the multi-line layout driver. Owns the line-Y step
/// (`clamped + lineStepFrac`), the per-line horizontal-justify X offset, the wrap-px math, and the line
/// count; delegates the line measure to `0x5c7260` ([`measure_line`]) and the per-glyph quads to
/// `glyph_quad_build` (0x5ccbe0). `ctx`/`text` drive the real measure (the binary's runs for real).
///
/// Setup: `lineMetric = ScreenToPixelHeight(raw, −clampedSize)` (`5cdc55`, negated). Not rotated
/// (`flags & 0x80 == 0`): `wrapPx = __ftol(screenH·wrap + 0.99999)` (`5cdcdc`), `lineStepFrac =
/// wrapPx / screenH` (`5cdd01`, `fdiv` by screenH), `lineStep = ScreenToPixelHeight(false, clampedSize) +
/// (f32)wrapPx` (`5cdd08`), `originZ = anchor.z`. Rotated: `lineMetric = 0`, `lineStepFrac = wrap`,
/// `lineStep = wrap + clampedSize` (`5cdcc9`), `originZ = 0`.
///
/// Per-line loop (`5cdd37..5cdeb6`): terminate when `penY >= boxBottom` (or unordered); else
/// `penY += clampedSize + lineStepFrac`; terminate on empty cursor. A newline token (class 2) is a
/// newline-only line: `count++`, `baseline -= lineStep`. Else measure the line, terminate on no progress
/// (`outNext == cursor`, or `outBytes == 0 && *outNext == 0`); compute `vjust_x = ScreenToPixelWidth(raw,
/// −offset)` with `offset` = `lineWidth` (justify_h 2) / `lineWidth·0.5` (justify_h 1) / 0; emit the line
/// (origin = `vjust_x, baseline, originZ`); `count++`; `baseline -= lineStep`; stop if `flags & 2`.
pub fn string_layout_lines(ctx: &FontCtx, text: &[u8], s: &LayoutState) -> LayoutLines {
    // 5cdc67: raw = flags bit7 (rotated / raw-px). The measure & glyph paths share it.
    let raw = s.flags & 0x80 != 0;

    // 5cdc55: lineMetric = ScreenToPixelHeight(raw, −clampedSize). Rotated ⇒ 0 (5cdc87).
    // 5cdc8f: originZ = anchor.z (not rotated) / 0 (rotated).
    let (line_metric, origin_z): (f32, f32) = if raw {
        (0.0, 0.0)
    } else {
        (
            screen_to_pixel_height(false, -s.clamped_size, s.screen_h),
            s.anchor_z,
        )
    };

    // line_step_frac ([ebp-0xc], the penY increment) and line_step ([ebp-0x8], the baseline decrement).
    let (line_step_frac, line_step): (f32, f32) = if raw {
        // 5cdcc9: rotated — frac = wrap, step = wrap + clampedSize.
        (
            s.wrap_width,
            (f64::from(s.wrap_width) + f64::from(s.clamped_size)) as f32,
        )
    } else {
        // 5cdcdc: wrapPx = __ftol(screenH·wrap + 0.99999).
        // The result goes to `[ebp-0x3c]` with a zero high dword (`5cdcef`, `5cdcf2`), and `fild
        // qword` reads it back zero-extended, so a negative count reads as a `u32`.
        let wrap_px =
            ftol(f64::from(s.screen_h) * f64::from(s.wrap_width) + f64::from(C_CEIL_BIAS)) as u32;
        // 5cdd01: lineStepFrac = wrapPx / screenH (fild wrapPx; fdiv by the screenH still on st1).
        let frac = (f64::from(wrap_px) / f64::from(s.screen_h)) as f32;
        // 5cdd08: lineStep = ScreenToPixelHeight(false, clampedSize) + (f32)wrapPx.
        let sph = screen_to_pixel_height(false, s.clamped_size, s.screen_h);
        let step = (f64::from(sph) + f64::from(wrap_px as f32)) as f32;
        (frac, step)
    };

    let mut count: i32 = 0; // +0x9c
    let mut baseline = line_metric; // [ebp-0x44]
    let mut pen_y: f32 = 0.0; // [ebp-0x14]
    let mut cursor = 0usize; // esi
    let mut broke: u8 = 1; // [ebp-0x1] (init 1, persists into each measure)
    let mut lines: Vec<LineOrigin> = Vec::new();

    // 5cdd31: empty text bails before the loop (the +0x9c / +0x48 guards are the caller's domain).
    if text.first().copied().unwrap_or(0) != 0 {
        loop {
            // 5cdd37: terminate when penY >= boxBottom (the `ja`/`jp` keeps going only while penY < bottom).
            if !(f64::from(pen_y) < f64::from(s.box_bottom)) {
                break;
            }
            let cur_char = text.get(cursor).copied().unwrap_or(0); // cl (5cdd20 / reloaded 5cdea3)
                                                                   // 5cdd4d: penY += clampedSize + lineStepFrac.
            pen_y =
                ((f64::from(pen_y) + f64::from(s.clamped_size)) + f64::from(line_step_frac)) as f32;
            // 5cdd56: terminate on the empty cursor.
            if cur_char == 0 {
                break;
            }
            let dec = decode_quoted_code(&text[cursor..], s.flags); // 5cdd6a
            if dec.class == 2 {
                // 5cdd7b: newline-only line — baseline -= step, advance past it, count it.
                baseline = (f64::from(baseline) - f64::from(line_step)) as f32;
                cursor += dec.consumed as usize;
                count += 1;
                if text.get(cursor).copied().unwrap_or(0) == 0 {
                    break; // 5cdea3/5cdeb6: next char NUL ends the walk
                }
                continue;
            }
            // 5cddb1: measure the line. extraSpacing = +0x34 when flags&1, else 0.
            let extra = if s.flags & 1 != 0 {
                s.extra_spacing
            } else {
                0.0
            };
            let step = measure_line(
                ctx,
                &text[cursor..],
                s.clamped_size,
                s.box_width,
                extra,
                s.flags,
                broke,
            );
            broke = step.broke;
            let out_next = cursor + step.next as usize; // [ebp-0x1c]
                                                        // 5cddd8: no progress (outNext == cursor).
            if step.next == 0 {
                break;
            }
            // 5cddeb: outBytes == 0 && *outNext == 0.
            if step.out_bytes == 0 && text.get(out_next).copied().unwrap_or(0) == 0 {
                break;
            }
            // 5cddf9: horizontal-justify X offset (justify_h selects the width factor; negated; SPW).
            let vjust_x: f32 = match s.justify_h {
                2 => screen_to_pixel_width(raw, (-f64::from(step.width)) as f32, s.screen_w),
                1 => screen_to_pixel_width(
                    raw,
                    (-(f64::from(step.width) * f64::from(C_0_5))) as f32,
                    s.screen_w,
                ),
                _ => 0.0, // 5cdc80: default origin.x = 0
            };
            // 5cde50: emit the line's quads (origin = vjust_x, baseline, originZ).
            lines.push(LineOrigin {
                vjust_x,
                baseline_y: baseline,
                origin_z,
            });
            count += 1; // 5cde55
            baseline = (f64::from(baseline) - f64::from(line_step)) as f32; // 5cde82
            if s.flags & 0x2 != 0 {
                break; // 5cde99: stop after the first content line
            }
            cursor = out_next; // 5cde91: esi = outNext
            if text.get(cursor).copied().unwrap_or(0) == 0 {
                break; // 5cdea3/5cdeb6
            }
        }
    }

    LayoutLines {
        count,
        line_step,
        line_metric,
        lines,
    }
}
