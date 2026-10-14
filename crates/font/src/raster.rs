//! The glyph-to-atlas blits and the atlas cell and UV geometry of the glyph texture cache: the four
//! glyph-to-atlas-texel blits (plain, mono, outline, antialiased outline), the atlas grid and
//! cell-to-texel-address arithmetic, the per-glyph UV rectangle, the atlas-row free-span gaps, the
//! glyph vertical-placement clip, and the 26.6-to-pixel metric conversions of the FreeType-driving
//! glyph rasterizer. Each public item transcribes one client function, and its doc-comment cites the
//! source address.
//!
//! The atlas is a 256x256 ARGB4444 texture (`u16` texels). A glyph's staging buffer holds 256-texel
//! rows (stride `0x200` bytes = `0x100` u16). All blits write into a sub-region whose top-left is the
//! destination pointer and advance one row (`0x100` u16) per source row. The x87 runs at 53-bit
//! precision (control word `0x027F`): every float operation is transcribed as `f64` arithmetic with an
//! `as f32` at the exe's `fstp m32` stores.

/// `0x80a8ac` (0x80a8ac) = `00 00 80 3b` = **1/256** — the atlas-page UV normalization constant.
const UV_SCALE: f32 = f32::from_bits(0x3b80_0000);
/// `0x80a92c` (0x80a92c) = `00 00 80 3c` = **1/64** — the 26.6 fixed-point → float-pixel scale.
const FIXED_2_6_SCALE: f32 = f32::from_bits(0x3c80_0000);
/// `0x7ff9d8` (0x7ff9d8) = `00 00 80 3f` = **1.0** — the per-glyph advance +1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);

/// `0xc2ba48` — the global glyph-staging texel buffer base (256-texel rows, 2 B/texel).
pub const ATLAS_STAGING_BUFFER: u32 = 0x00c2_ba48;
/// Atlas texel row stride in `u16` texels (256 texels × 2 B = `0x200` B per row).
pub const ATLAS_ROW_STRIDE: usize = 0x100;
/// Initial per-bucket-row free width set by [`texcache_set_cellsize`] (256 px of free width per row).
pub const ATLAS_ROW_FREE_WIDTH: u32 = 0x100;

/// The AA neighbour-count → 4-bit outline-alpha LUT `0x80a8ec` (`.rdata` bytes
/// `00 01 01 03 05 07 09 0b 0d 0f`): neighbour count `0..=9` → alpha nibble.
pub const AA_NEIGHBOUR_LUT: [u8; 10] = [0x00, 0x01, 0x01, 0x03, 0x05, 0x07, 0x09, 0x0b, 0x0d, 0x0f];

// ───────────────────────────── A. glyph → atlas-texel blits ─────────────────────────────

/// `glyph_blit_argb4444` (0x5cf220): the plain (non-shadow, non-outline) blit. Lays an 8-bit-coverage
/// glyph bitmap into the atlas staging buffer as ARGB4444, one cell column.
///
/// Per the byte-cited pack (`mov ch,[eax+edi]; or ecx,0xfff; mov [esi+2*eax],cx`): each glyph texel =
/// `(src8 << 8) | 0x0fff` — ARGB4444 with **A = src>>4** (the 4-bit-quantised coverage) and **RGB = white**.
/// `top_pad` blank rows then the glyph's `rows` rows (each `cols` texels written) then the trailing pad to
/// the cell height `cellsize` (= font+0x178). Pad rows zero-fill `clear_w` texels; `dst` advances one atlas
/// row (`0x100` u16) per source row. `src[r*stride + c]` is the coverage byte for glyph (row r, col c).
#[allow(clippy::too_many_arguments)] // each argument is a distinct glyph-desc field the binary reads
pub fn glyph_blit_argb4444(
    src: &[u8],
    cols: usize,
    rows: usize,
    clear_w: usize,
    stride: usize,
    top_pad: usize,
    cellsize: i32,
    dst: &mut [u16],
) {
    // top-pad rows: zero `clear_w` texels each.
    for i in 0..top_pad {
        let base = i * ATLAS_ROW_STRIDE;
        for k in 0..clear_w {
            dst[base + k] = 0;
        }
    }
    // glyph rows: texel = (coverage << 8) | 0x0fff.
    for r in 0..rows {
        let row_base = (top_pad + r) * ATLAS_ROW_STRIDE;
        let src_base = r * stride;
        for c in 0..cols {
            dst[row_base + c] = ((u16::from(src[src_base + c])) << 8) | 0x0fff;
        }
    }
    // trailing pad to cell height: cellsize - top_pad - rows rows, `clear_w` texels each (only if > 0).
    let bottom = cellsize - top_pad as i32 - rows as i32;
    for j in 0..bottom.max(0) as usize {
        let base = (top_pad + rows + j) * ATLAS_ROW_STRIDE;
        for k in 0..clear_w {
            dst[base + k] = 0;
        }
    }
}

/// `glyph_blit_mono` (0x5ce910): the 1-bit glyph bitmap → atlas cell. Identical row structure to
/// [`glyph_blit_argb4444`] but each glyph texel is **0xffff** (set bit, ARGB4444 white-opaque) or **0**.
///
/// The source bit for glyph (row r, col c) is `(src[r*stride + (c>>3)] >> (7 - (c & 7))) & 1` (MSB-first
/// 1-bit packing). Pad rows zero-fill `clear_w` texels; trailing pad fills to cell height `cellsize`.
#[allow(clippy::too_many_arguments)] // each argument is a distinct glyph-desc field the binary reads
pub fn glyph_blit_mono(
    src: &[u8],
    cols: usize,
    rows: usize,
    clear_w: usize,
    stride: usize,
    top_pad: usize,
    cellsize: i32,
    dst: &mut [u16],
) {
    for i in 0..top_pad {
        let base = i * ATLAS_ROW_STRIDE;
        for k in 0..clear_w {
            dst[base + k] = 0;
        }
    }
    for r in 0..rows {
        let row_base = (top_pad + r) * ATLAS_ROW_STRIDE;
        let src_base = r * stride;
        for c in 0..cols {
            let bit = (src[src_base + (c >> 3)] >> (7 - (c & 7))) & 1;
            dst[row_base + c] = if bit != 0 { 0xffff } else { 0 };
        }
    }
    let bottom = cellsize - top_pad as i32 - rows as i32;
    for j in 0..bottom.max(0) as usize {
        let base = (top_pad + rows + j) * ATLAS_ROW_STRIDE;
        for k in 0..clear_w {
            dst[base + k] = 0;
        }
    }
}

/// The dilation scratch grid: a flat 256-stride `u16` map. The binary's stack scratch is `0x2400` u16
/// (36 atlas rows); a cell `(row, col)` maps to `map[row*0x100 + col]`. The glyph is rasterized one atlas
/// row down and one (or two, with the outline flag) columns right of the grid origin so the dilation has a
/// 1-px border to grow into.
const DILATE_MAP: usize = 0x2400;

/// Is any in-bounds 8-neighbour of cell `(row, col)` set under `mask`? (Equivalent to the binary's nested
/// corner/edge branches, which enumerate exactly the 8 surrounding cells inside the `nrows × ncols` grid.)
fn any_neighbour_set(
    map: &[u16],
    row: usize,
    col: usize,
    nrows: usize,
    ncols: usize,
    mask: u16,
) -> bool {
    for dr in -1i32..=1 {
        for dc in -1i32..=1 {
            if dr == 0 && dc == 0 {
                continue;
            }
            let r = row as i32 + dr;
            let c = col as i32 + dc;
            if r < 0 || r >= nrows as i32 || c < 0 || c >= ncols as i32 {
                continue;
            }
            let nb = (r as usize) * ATLAS_ROW_STRIDE + c as usize;
            if (mask & map[nb]) != 0 {
                return true;
            }
        }
    }
    false
}

/// `glyph_blit_outline` (0x5ce440): a 1-bit glyph → outlined atlas cell via two-pass neighbour dilation.
///
/// The glyph is rasterized into a scratch map (1 = glyph) at origin `(row=1+top_pad, col=1 or 2)`. Pass 0
/// (mask 1, write 4) marks every cell adjacent to a glyph cell `4` (guarded so glyph cells stay `1`); pass 1
/// (only when `outline_flag`; mask 4, write 2, gated on empty cells) grows a second ring `2` around the
/// `4`-ring. The final pack writes `cellsize × clear_w` texels: `1 → 0xffff`, `2 → 0x7000`, `4 → 0xf000`,
/// else `0`. `dst[row*0x100 + col]`.
#[allow(clippy::too_many_arguments)] // each argument is a distinct glyph-desc field the binary reads
pub fn glyph_blit_outline(
    src: &[u8],
    cols: usize,
    rows: usize,
    clear_w: usize,
    stride: usize,
    top_pad: usize,
    cellsize: usize,
    outline_flag: bool,
    dst: &mut [u16],
) {
    let mut map = [0u16; DILATE_MAP];
    let col_off = if outline_flag { 2 } else { 1 };
    let origin = (1 + top_pad) * ATLAS_ROW_STRIDE + col_off;

    // rasterize: 1 where the 1-bit glyph is set, 0 elsewhere.
    for r in 0..rows {
        let row_base = origin + r * ATLAS_ROW_STRIDE;
        let src_base = r * stride;
        for c in 0..cols {
            let bit = (src[src_base + (c >> 3)] >> (7 - (c & 7))) & 1;
            map[row_base + c] = u16::from(bit != 0);
        }
    }

    // dilation passes.
    let passes = if outline_flag { 2 } else { 1 };
    for pass in 0..passes {
        let (mask, write, start) = if pass == 0 {
            (1u16, 4u16, top_pad)
        } else {
            (4u16, 2u16, top_pad.saturating_sub(1))
        };
        for row in start..cellsize {
            for col in 0..clear_w {
                let c = row * ATLAS_ROW_STRIDE + col;
                if pass == 1 && map[c] != 0 {
                    continue; // pass-1 gate: only empty cells
                }
                if any_neighbour_set(&map, row, col, cellsize, clear_w, mask) && map[c] != 1 {
                    map[c] = write;
                }
            }
        }
    }

    // final pack into the atlas texels.
    for row in 0..cellsize {
        let dst_base = row * ATLAS_ROW_STRIDE;
        let map_base = row * ATLAS_ROW_STRIDE;
        for col in 0..clear_w {
            let v = map[map_base + col];
            dst[dst_base + col] = match v {
                1 => 0xffff,
                2 => 0x7000,
                4 => 0xf000,
                _ => 0,
            };
        }
    }
}

/// `glyph_blit_aa_outline` (0x5cea30): an antialiased glyph + outline → ARGB4444 atlas cell.
///
/// Three scratch grids: an outline map (1 where coverage non-zero), a coverage map (the raw 8-bit coverage),
/// and an alpha map. The outline map is dilated (pass 0: mask 1 → 2; pass 1 when `outline_flag`: mask 3 → 4),
/// then each cell's alpha nibble = `AA_NEIGHBOUR_LUT[count]` where `count` is the number of non-zero
/// outline-map cells in the in-bounds 3×3 box (centre included). Final pack (`cellsize × clear_w` texels):
/// - outline cell `0` → `0`;
/// - coverage `0` (outline-only) → `alpha << 12` (black outline, A = alpha);
/// - else `fill = (coverage·0xff) >> 12`, texel = ARGB4444 `{A = alpha, R = G = B = fill}`.
#[allow(clippy::too_many_arguments)] // each argument is a distinct glyph-desc field the binary reads
pub fn glyph_blit_aa_outline(
    src: &[u8],
    cols: usize,
    rows: usize,
    clear_w: usize,
    stride: usize,
    top_pad: usize,
    cellsize: usize,
    outline_flag: bool,
    dst: &mut [u16],
) {
    let mut outline = [0u16; DILATE_MAP];
    let mut cov = [0u16; DILATE_MAP];
    let mut alpha = [0u16; DILATE_MAP];
    let col_off = if outline_flag { 2 } else { 1 };
    let origin = (1 + top_pad) * ATLAS_ROW_STRIDE + col_off;

    // rasterize: coverage byte into `cov`, a 1 marker into `outline`, only where coverage non-zero.
    for r in 0..rows {
        let row_base = origin + r * ATLAS_ROW_STRIDE;
        let src_base = r * stride;
        for c in 0..cols {
            let b = src[src_base + c];
            if b != 0 {
                outline[row_base + c] = 1;
                cov[row_base + c] = u16::from(b);
            }
        }
    }

    // dilation passes into the outline map.
    let passes = if outline_flag { 2 } else { 1 };
    let start = top_pad.saturating_sub(1);
    for pass in 0..passes {
        let (mask, write) = if pass == 0 {
            (1u16, 2u16)
        } else {
            (3u16, 4u16)
        };
        for row in start..cellsize {
            for col in 0..clear_w {
                let c = row * ATLAS_ROW_STRIDE + col;
                if (mask & outline[c]) != 0 {
                    continue; // gate: only cells without the mask bits
                }
                if any_neighbour_set(&outline, row, col, cellsize, clear_w, mask) {
                    outline[c] = write;
                }
            }
        }
    }

    // neighbour-count → alpha nibble (in-bounds 3×3 box of the outline map, centre included).
    for row in 0..cellsize {
        for col in 0..clear_w {
            let c = row * ATLAS_ROW_STRIDE + col;
            let mut count = 0usize;
            for dr in -1i32..=1 {
                for dc in -1i32..=1 {
                    let r = row as i32 + dr;
                    let cc = col as i32 + dc;
                    if r < 0 || r >= cellsize as i32 || cc < 0 || cc >= clear_w as i32 {
                        continue;
                    }
                    if outline[(r as usize) * ATLAS_ROW_STRIDE + cc as usize] != 0 {
                        count += 1;
                    }
                }
            }
            alpha[c] = u16::from(AA_NEIGHBOUR_LUT[count]);
        }
    }

    // final ARGB4444 pack.
    for row in 0..cellsize {
        let base = row * ATLAS_ROW_STRIDE;
        for col in 0..clear_w {
            let c = base + col;
            let texel = if outline[c] == 0 {
                0
            } else if cov[c] == 0 {
                alpha[c] << 0xc
            } else {
                let fill = ((u32::from(cov[c] & 0xff) * 0xff) >> 0xc) as u16;
                // ARGB4444 {A=alpha, R=G=B=fill}; the binary's `(ushort)` truncation is the u16 width.
                let a = (alpha[c] << 4) | fill;
                let b = (a << 4) | fill;
                (b << 4) | fill
            };
            dst[c] = texel;
        }
    }
}

// ───────────────────────────── B. atlas grid + cell math ─────────────────────────────

/// `texcache_set_cellsize` (0x5cf360): the atlas bucket-row count = **256 / cellsize** (`mov eax,0x100;
/// div [cellsize]`). The 256×256 atlas is partitioned into `256/cellsize` horizontal bucket-rows, each
/// `cellsize` px tall; each new row's free width is initialized to [`ATLAS_ROW_FREE_WIDTH`] (256).
/// `cellsize` must be non-zero (the binary divides without guarding).
pub fn atlas_rows(cellsize: u32) -> u32 {
    256 / cellsize
}

/// `atlas_texel_addr` (0x5cf5f0): the staging-buffer byte address of a glyph's top-left cell texel =
/// `0xc2ba48 + 2·((page·cellsize)·256 + x0)` (`imul; shl 8; add x0; lea [2*esi + 0xc2ba48]`), where
/// `page` = glyph atlas page/row index (desc+0x2c), `cellsize` = font raster pixel size, `x0` = cell x-start
/// (desc+0x30). 256 texels/row, 2 B/texel.
pub fn atlas_texel_addr(page: i32, cellsize: i32, x0: i32) -> u32 {
    let inner = (page.wrapping_mul(cellsize) << 8).wrapping_add(x0);
    ATLAS_STAGING_BUFFER.wrapping_add((inner as u32).wrapping_mul(2))
}

/// `glyph_uv_rect` (0x5c7f80): the per-glyph atlas UV rect, normalized to `[0,1]` by the 1/256 page
/// constant. Fills desc+0x60..0x6c:
/// - `+0x60` (v_bottom) = `(a·b) · 1/256`
/// - `+0x64` (u_left)   = `cell_x0 · 1/256`
/// - `+0x68` (v_top)    = `(a·b + b) · 1/256`
/// - `+0x6c` (u_right)  = `(advance_px + cell_x0) · 1/256`
///
/// `a·b` is loaded as a signed dword (`fild dword`); `a·b+b`, `cell_x0` and `advance_px` are zero-extended
/// dwords (`fild qword`, high = 0). Returns `[v_bottom, u_left, v_top, u_right]` (the desc+0x60..0x6c order).
pub fn glyph_uv_rect(a: i32, b: i32, cell_x0: i32, advance_px: i32) -> [f32; 4] {
    let k = f64::from(UV_SCALE); // exactly 1/256
    let ab = a.wrapping_mul(b);
    let v_bottom = (f64::from(ab) * k) as f32; // fild dword (signed)
    let u_left = (f64::from(cell_x0 as u32) * k) as f32; // fild qword (zero-extended)
    let v_top = (f64::from(ab.wrapping_add(b) as u32) * k) as f32; // fild qword
    let u_right = ((f64::from(advance_px as u32) + f64::from(cell_x0 as u32)) * k) as f32;
    [v_bottom, u_left, v_top, u_right]
}

/// `cell_free_after` (0x5c8010): the free span of atlas-row cells to the RIGHT of a glyph. If the next
/// link `[this+0x1c]` is a real node (low bit clear, non-null): `next.cell_x0 - this.cell_x1 - 1`. Else the
/// span to the row end (row width 256, max index 255): `0xff - this.cell_x1`.
pub fn cell_free_after(next_link: u32, next_cell_x0: i32, this_cell_x1: i32) -> i32 {
    if (next_link & 1) == 0 && next_link != 0 {
        next_cell_x0.wrapping_sub(this_cell_x1).wrapping_sub(1)
    } else {
        0xff - this_cell_x1
    }
}

/// `cell_free_before` (0x5c8030): the free span of atlas-row cells to the LEFT of a glyph. If the head
/// node's prev link `[*[this+0x18] + 4]` is a real node (low bit clear, non-null): `this.cell_x0 -
/// prev.cell_x1 - 1`. Else the span from the row start (index 0): `this.cell_x0`.
pub fn cell_free_before(prev_link: u32, prev_cell_x1: i32, this_cell_x0: i32) -> i32 {
    if (prev_link & 1) == 0 && prev_link != 0 {
        this_cell_x0.wrapping_sub(prev_cell_x1).wrapping_sub(1)
    } else {
        this_cell_x0
    }
}

// ───────────────────────────── C. glyph vertical placement ─────────────────────────────

/// `glyph_vplace` (0x5d1360): seat the glyph bitmap vertically in its em-tall cell so the baseline
/// lands at `cellTop + ascender`. Returns `(gap, top_pad)` — the binary's two out-pointers
/// (`edx` → out+0x20, `[ebp+8]` → out+0x24). `top_pad` (out+0x24) is the blank rows above the bitmap;
/// `gap` (out+0x20) is the overflow term (0 in the normal in-bounds case).
///
/// **The two vertical operands:**
/// - **`ascender`** — the face-wide ascent-in-pixels, i.e. `CGxFont+0x17c` (the
///   `round(renderH·asc/(asc+|desc|))` load_param, ≈0.794·em for Friz Quadrata). It arrives as the
///   `[ebp+0x10]` stack arg, threaded `0x5ca160 → rasterize_glyph.[ebp+0xc] → here`. **This is the
///   operand that fixes the baseline row** — the seating quantity. It is NOT FreeType's scaled hhea
///   ascender (≈0.965·em), which never enters this path.
/// - **`bitmap_top`** — the *per-glyph* FreeType `bitmap_top`, read live off the face's current slot as
///   `[[ctx+0x54]+0x68]` (ctx = FT_Face, `+0x54` = glyph slot, `+0x68` = `FT_GlyphSlotRec::bitmap_top`;
///   the FT_Bitmap occupies slot+0x4c..+0x64, `bitmap_left` at +0x64). It is the glyph's own top
///   bearing, NOT an ascender.
///
/// With `cell_h` = the em-tall cell height (renderH = `CGxFont+0x24c`), `glyph_rows` = the clipped
/// glyph row count:
/// - if `glyph_rows > cell_h` (the binary's **unsigned** `cmp; jbe`): `gap 0`, `top_pad 0`;
/// - else if `bitmap_top > ascender`: `gap = bitmap_top - ascender` (glyph overshoots the ascender
///   line), `top_pad 0`;
/// - else: `top_pad = ascender - bitmap_top` (clamped ≥ 0). So
///   `baseline = cellTop + top_pad + bitmap_top = cellTop + ascender`.
///
/// Then if `(cell_h - glyph_rows) < top_pad` (signed): `gap = (cell_h - top_pad) - glyph_rows`,
/// `top_pad = cell_h - glyph_rows` (clip to the cell); else `(gap, top_pad)` as computed. The ascender,
/// clamp, and clip tests are signed; only the `cell_h`/`glyph_rows` test is unsigned.
pub fn glyph_vplace(bitmap_top: i32, cell_h: i32, ascender: i32, glyph_rows: i32) -> (i32, i32) {
    let mut gap = 0i32; // out+0x20 (edx): bitmap_top - ascender overflow, else 0
    let mut top_pad; // out+0x24 (esi): blank rows above the glyph = ascender - bitmap_top
    if (cell_h as u32) < (glyph_rows as u32) {
        top_pad = 0;
    } else if ascender < bitmap_top {
        top_pad = 0;
        gap = bitmap_top.wrapping_sub(ascender);
    } else {
        top_pad = ascender.wrapping_sub(bitmap_top);
    }
    // clamp top_pad to >= 0 (`setle cl; dec ecx; and esi,ecx`).
    if top_pad <= 0 {
        top_pad = 0;
    }
    let d = cell_h.wrapping_sub(glyph_rows); // ecx (signed compare below)
    if d < top_pad {
        (cell_h.wrapping_sub(top_pad).wrapping_sub(glyph_rows), d)
    } else {
        (gap, top_pad)
    }
}

// ───────────────────────────── D. glyph rasterizer metric math ─────────────────────────────

/// The output of [`rasterize_glyph`] — the glyph-bitmap descriptor fields the binary fills past the
/// FreeType delegation (out+0x04..0x24). `out[0]` is the bitmap allocation pointer (a heap address, not
/// fidelity) and is excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RasterMetrics {
    /// out[1] (+0x04) — bitmap byte size.
    pub size: u32,
    /// out[2] (+0x08) — glyph width.
    pub width: i32,
    /// out[3] (+0x0c) — clipped rows.
    pub clipped_rows: i32,
    /// out[4] (+0x10) — width + advance-adjust.
    pub width_plus_adj: i32,
    /// out[7] (+0x1c) — bitmap pitch.
    pub pitch: i32,
    /// out[8]/out[9] (+0x20/+0x24) — [`glyph_vplace`] `(gap, top_pad)` (0,0 for a blank glyph).
    pub vplace: (i32, i32),
    /// out[5] (+0x14) — advance metric `(advance>>6) + 1.0` in float pixels (raw bits, NaN-safe).
    pub adv_metric_bits: u32,
    /// out[6] (+0x18) — bearing metric `bearing · 1/64` in float pixels (raw bits).
    pub bearing_metric_bits: u32,
}

/// `rasterize_glyph` (0x5d1120): drives FreeType to load+render the glyph (host code, stubbed in the
/// test), copies the bitmap out, and converts the FT 26.6-fixed metrics to float pixels. This models the
/// arithmetic given the already-rendered FT glyph-slot fields:
/// - `clipped_rows` = `min(code, slot_rows)` (unsigned); blank glyph → `code`.
/// - real bitmap iff `slot_width≠0 ∧ clipped_rows≠0 ∧ buffer≠0 ∧ slot_pitch≠0 ∧ slot_rows·slot_pitch≠0`.
/// - blank glyph: `width = (code+3)>>2` (or `code` if that is 0); `pitch = render_flag ? (width+7)&~7 : width`;
///   `size = pitch·code`.
/// - `width_plus_adj = width + adv_adj`.
/// - `adv_metric = (float)((advance + (advance>>31 & 0x3f)) >> 6) + 1.0` (round-toward-zero 26.6→int).
/// - `bearing_metric = (float)bearing · 1/64`.
/// - `vplace = glyph_vplace(bitmap_top, code, ascender, clipped_rows)` for a real bitmap, else `(0, 0)`.
///   `ascender` here is the face `CGxFont+0x17c` load_param, threaded in as the binary's `[ebp+0xc]`
///   arg; `bitmap_top` is the FT slot's per-glyph `bitmap_top` (read from `[[face+0x54]+0x68]` by
///   `glyph_vplace`, modelled here as an explicit arg since FreeType is the stubbed boundary).
#[allow(clippy::too_many_arguments)]
pub fn rasterize_glyph(
    code: i32,
    render_flag: bool,
    adv_adj: i32,
    ascender: i32,
    slot_rows: i32,
    slot_width: i32,
    slot_pitch: i32,
    slot_buffer_nonzero: bool,
    slot_advance: i32,
    slot_bearing: i32,
    bitmap_top: i32,
) -> RasterMetrics {
    // clipped rows = min(code, slot_rows) via unsigned compare (`cmp esi,eax; jb`).
    let clipped = if (code as u32) < (slot_rows as u32) {
        code
    } else {
        slot_rows
    };
    let size_real = slot_rows.wrapping_mul(slot_pitch);
    let real =
        slot_width != 0 && clipped != 0 && slot_buffer_nonzero && slot_pitch != 0 && size_real != 0;

    let (width, clipped_rows, pitch, size);
    if real {
        width = slot_width;
        clipped_rows = clipped;
        pitch = slot_pitch;
        size = size_real as u32;
    } else {
        // blank synthesis.
        let mut bw = ((code as u32).wrapping_add(3) >> 2) as i32; // (code+3)>>2 (logical shr)
        if bw == 0 {
            bw = code;
        }
        let p = if render_flag { (bw + 7) & !7 } else { bw };
        width = bw;
        clipped_rows = code;
        pitch = p;
        size = p.wrapping_mul(code) as u32;
    }

    // 26.6 → float-pixel metric conversions (PC_53 f64 math, f32 store).
    let adv_round = slot_advance.wrapping_add((slot_advance >> 31) & 0x3f) >> 6;
    let adv_metric = (f64::from(adv_round) + f64::from(ONE)) as f32;
    let bearing_metric = (f64::from(slot_bearing) * f64::from(FIXED_2_6_SCALE)) as f32;

    let vplace = if real {
        // code = cell height (renderH); ascender = CGxFont+0x17c load_param; bitmap_top = FT slot.
        glyph_vplace(bitmap_top, code, ascender, clipped_rows)
    } else {
        (0, 0)
    };

    RasterMetrics {
        size,
        width,
        clipped_rows,
        width_plus_adj: width.wrapping_add(adv_adj),
        pitch,
        vplace,
        adv_metric_bits: adv_metric.to_bits(),
        bearing_metric_bits: bearing_metric.to_bits(),
    }
}
