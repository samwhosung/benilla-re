//! Bit-exact tests of the glyph-to-atlas blits and the atlas cell and UV geometry, each run as the
//! exe's own code in the emulator on synthesized inputs and compared with the transcription in
//! `benilla_re_font::raster`. Choices are fuzzed on the LCG's high bits.
//!
//! - **Blits** (0x5cf220, 0x5ce910, 0x5ce440, 0x5cea30): a fuzzed glyph bitmap and a canary-filled
//!   destination atlas region in scratch; `call` the blit (`__thiscall(this, src_desc, dst)`), read
//!   back the destination texels and compare the whole region with the transcription over a
//!   canary-filled copy (coverage 0, 0xff and mid; the 1-bit, outline and AA-LUT paths; the shadow
//!   and outline cell-height pad and the bottom-pad-absent case).
//! - **Atlas and cell arithmetic** (0x5cf360, 0x5cf5f0, 0x5c7f80, 0x5c8010, 0x5c8030, 0x5d1360):
//!   integer and float reads taken whole through `call` and `call_f32` (or the dispatcher-arg stub
//!   for the texel address).
//! - **0x5d1120 `rasterize_glyph`**: the FreeType calls (0x7ce960 lib-live, 0x5d1300 load and render)
//!   and the bitmap allocator (0x6462e0) are stubbed, and the FT glyph slot is synthesized in
//!   scratch, so the 26.6-to-pixel metric arithmetic, the blank-glyph synthesis and the
//!   `glyph_vplace` integration run in isolation.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_font::raster::{
    atlas_rows, atlas_texel_addr, cell_free_after, cell_free_before, glyph_blit_aa_outline,
    glyph_blit_argb4444, glyph_blit_mono, glyph_blit_outline, glyph_uv_rect, glyph_vplace,
    rasterize_glyph, RasterMetrics, ATLAS_ROW_FREE_WIDTH, ATLAS_ROW_STRIDE,
};
use common::{assert_f32, next};

const GLYPH_BLIT_ARGB4444: u64 = 0x5cf220;
const GLYPH_BLIT_MONO: u64 = 0x5ce910;
const GLYPH_BLIT_OUTLINE: u64 = 0x5ce440;
const GLYPH_BLIT_AA_OUTLINE: u64 = 0x5cea30;
const TEXCACHE_SET_CELLSIZE: u64 = 0x5cf360;
const ATLAS_TEXEL_ADDR: u64 = 0x5cf5f0;
const BLIT_DISPATCH: u64 = 0x5cf310;
const GLYPH_UV_RECT: u64 = 0x5c7f80;
const CELL_FREE_AFTER: u64 = 0x5c8010;
const CELL_FREE_BEFORE: u64 = 0x5c8030;
const GLYPH_VPLACE: u64 = 0x5d1360;
const RASTERIZE_GLYPH: u64 = 0x5d1120;
const FT_LIB_LIVE: u64 = 0x7ce960;
const FT_LOAD_RENDER: u64 = 0x5d1300;
const SMEM_ALLOC: u64 = 0x6462e0;

// CGxFont, glyph-desc and FT-slot offsets the functions read.
const FONT_RASTER_PX: u64 = 0x178; // font[0x178] = atlas cell size
const FONT_FLAGS: u64 = 0x180; // font[0x180] = flags (bit3 0x8 = outline)
const STAGING_BASE: u32 = 0x00c2_ba48; // 0xc2ba48 (atlas_texel_addr base)

const CANARY: u16 = 0xa5c3;
const DST_U16: usize = 0x2400; // 36 atlas rows × 256 texels — covers every blit case

// ── small fuzz helpers (gated on the LCG's high bits) ──
fn pick(s: &mut u32, lo: usize, hi_inclusive: usize) -> usize {
    lo + (next(s) >> 8) as usize % (hi_inclusive - lo + 1)
}
fn pick_i32(s: &mut u32, lo: i32, hi_inclusive: i32) -> i32 {
    lo + ((next(s) >> 8) as i32).rem_euclid(hi_inclusive - lo + 1)
}
fn read_u16s(e: &Emu, va: u32, n: usize) -> Vec<u16> {
    e.read(u64::from(va), n * 2)
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect()
}

// ───────────────────────────── glyph blits ─────────────────────────────

/// A fuzzed source glyph + cell descriptor.
struct BlitCase {
    cols: usize,
    rows: usize,
    clear_w: usize,
    stride: usize,
    top_pad: usize,
    cellsize: usize,
    flags: u32,
    src: Vec<u8>,
}

/// Fuzz one source byte covering coverage edges 0 / 0xff / mid and randoms.
fn fuzz_byte(s: &mut u32) -> u8 {
    match (next(s) >> 8) % 4 {
        0 => 0x00,
        1 => 0xff,
        2 => 0x80,
        _ => (next(s) >> 16) as u8,
    }
}

/// Shared scratch for the blits: `(this, font, src_desc, dst)` allocated once at max size.
struct BlitScratch {
    this: u32,
    font: u32,
    src_buf: u32,
    desc: u32,
    dst: u32,
}
fn blit_scratch(e: &mut Emu) -> BlitScratch {
    let font = e.alloc(0x250) as u32;
    let this = e.alloc(0x10) as u32;
    e.write_u32(u64::from(this) + 4, font);
    BlitScratch {
        this,
        font,
        src_buf: e.alloc(0x400) as u32,
        desc: e.alloc(0x40) as u32,
        dst: e.alloc((DST_U16 * 2) as u64) as u32,
    }
}

/// Write `c`'s inputs into the scratch and reset the dest canary; return the canary dest as a `Vec`.
fn blit_load(e: &mut Emu, sc: &BlitScratch, c: &BlitCase) -> Vec<u16> {
    e.write_u32(u64::from(sc.font) + FONT_RASTER_PX, c.cellsize as u32);
    e.write_u32(u64::from(sc.font) + FONT_FLAGS, c.flags);
    e.write(u64::from(sc.src_buf), &c.src);
    let d = u64::from(sc.desc);
    e.write_u32(d, sc.src_buf); // +0x00 src buffer
    e.write_u32(d + 0x08, c.cols as u32); // +0x08 glyph cols
    e.write_u32(d + 0x0c, c.rows as u32); // +0x0c glyph rows
    e.write_u32(d + 0x10, c.clear_w as u32); // +0x10 clear width (texels)
    e.write_u32(d + 0x1c, c.stride as u32); // +0x1c src row stride
    e.write_u32(d + 0x24, c.top_pad as u32); // +0x24 top blank rows
    let canary = vec![CANARY; DST_U16];
    let bytes: Vec<u8> = canary.iter().flat_map(|t| t.to_le_bytes()).collect();
    e.write(u64::from(sc.dst), &bytes);
    canary
}

#[test]
fn glyph_blit_argb4444_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let sc = blit_scratch(&mut e);
    let mut s: u32 = 0x5cf2_2001;
    for _ in 0..600u32 {
        let cellsize = pick(&mut s, 1, 32);
        let top_pad = pick(&mut s, 0, 8);
        let rows = pick(&mut s, 0, 16);
        let cols = pick(&mut s, 0, 16);
        let clear_w = pick(&mut s, 0, 16);
        let stride = cols.max(1) + pick(&mut s, 0, 2);
        let src: Vec<u8> = (0..rows * stride + cols + 8)
            .map(|_| fuzz_byte(&mut s))
            .collect();
        let c = BlitCase {
            cols,
            rows,
            clear_w,
            stride,
            top_pad,
            cellsize,
            flags: 0,
            src,
        };
        let mut ours = blit_load(&mut e, &sc, &c);
        glyph_blit_argb4444(
            &c.src,
            c.cols,
            c.rows,
            c.clear_w,
            c.stride,
            c.top_pad,
            c.cellsize as i32,
            &mut ours,
        );
        e.call(GLYPH_BLIT_ARGB4444, sc.this, 0, &[sc.desc, sc.dst]);
        let bin = read_u16s(&e, sc.dst, DST_U16);
        assert_eq!(
            bin, ours,
            "argb4444 cell={} pad={} rows={} cols={} clr={} stride={}",
            cellsize, top_pad, rows, cols, clear_w, stride
        );
    }
}

#[test]
fn glyph_blit_mono_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let sc = blit_scratch(&mut e);
    let mut s: u32 = 0x5ce9_1001;
    for _ in 0..600u32 {
        let cellsize = pick(&mut s, 1, 32);
        let top_pad = pick(&mut s, 0, 8);
        let rows = pick(&mut s, 0, 16);
        let cols = pick(&mut s, 0, 16);
        let clear_w = pick(&mut s, 0, 16);
        let stride = cols.div_ceil(8).max(1) + pick(&mut s, 0, 2);
        let src: Vec<u8> = (0..rows * stride + 8).map(|_| fuzz_byte(&mut s)).collect();
        let c = BlitCase {
            cols,
            rows,
            clear_w,
            stride,
            top_pad,
            cellsize,
            flags: 0,
            src,
        };
        let mut ours = blit_load(&mut e, &sc, &c);
        glyph_blit_mono(
            &c.src,
            c.cols,
            c.rows,
            c.clear_w,
            c.stride,
            c.top_pad,
            c.cellsize as i32,
            &mut ours,
        );
        e.call(GLYPH_BLIT_MONO, sc.this, 0, &[sc.desc, sc.dst]);
        let bin = read_u16s(&e, sc.dst, DST_U16);
        assert_eq!(
            bin, ours,
            "mono cell={} pad={} rows={} cols={} clr={} stride={}",
            cellsize, top_pad, rows, cols, clear_w, stride
        );
    }
}

/// Build a small in-grid case for the dilation blits (glyph fits in cell with a 1-px border).
fn dilation_case(s: &mut u32, bits_1: bool) -> BlitCase {
    let cellsize = pick(s, 8, 14);
    let clear_w = pick(s, 4, 9);
    let rows = pick(s, 1, 3);
    let top_pad = pick(s, 0, cellsize - rows - 1);
    let cols = pick(s, 1, 6);
    let outline = (next(s) >> 8) & 1 == 1;
    let flags = ((next(s) >> 12) & !0x8) | if outline { 0x8 } else { 0 };
    let stride = if bits_1 {
        cols.div_ceil(8).max(1) + pick(s, 0, 1)
    } else {
        cols + pick(s, 0, 1)
    };
    let src: Vec<u8> = (0..rows * stride + cols + 8)
        .map(|_| fuzz_byte(s))
        .collect();
    BlitCase {
        cols,
        rows,
        clear_w,
        stride,
        top_pad,
        cellsize,
        flags,
        src,
    }
}

#[test]
fn glyph_blit_outline_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let sc = blit_scratch(&mut e);
    let mut s: u32 = 0x5ce4_4001;
    for _ in 0..500u32 {
        let c = dilation_case(&mut s, true);
        let mut ours = blit_load(&mut e, &sc, &c);
        glyph_blit_outline(
            &c.src,
            c.cols,
            c.rows,
            c.clear_w,
            c.stride,
            c.top_pad,
            c.cellsize,
            c.flags & 0x8 != 0,
            &mut ours,
        );
        e.call(GLYPH_BLIT_OUTLINE, sc.this, 0, &[sc.desc, sc.dst]);
        let bin = read_u16s(&e, sc.dst, DST_U16);
        assert_eq!(
            bin, ours,
            "outline cell={} pad={} rows={} cols={} clr={} flags={:#x}",
            c.cellsize, c.top_pad, c.rows, c.cols, c.clear_w, c.flags
        );
    }
}

#[test]
fn glyph_blit_aa_outline_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let sc = blit_scratch(&mut e);
    let mut s: u32 = 0x5cea_3001;
    for _ in 0..500u32 {
        let c = dilation_case(&mut s, false);
        let mut ours = blit_load(&mut e, &sc, &c);
        glyph_blit_aa_outline(
            &c.src,
            c.cols,
            c.rows,
            c.clear_w,
            c.stride,
            c.top_pad,
            c.cellsize,
            c.flags & 0x8 != 0,
            &mut ours,
        );
        e.call(GLYPH_BLIT_AA_OUTLINE, sc.this, 0, &[sc.desc, sc.dst]);
        let bin = read_u16s(&e, sc.dst, DST_U16);
        assert_eq!(
            bin, ours,
            "aa_outline cell={} pad={} rows={} cols={} clr={} flags={:#x}",
            c.cellsize, c.top_pad, c.rows, c.cols, c.clear_w, c.flags
        );
    }
}

// ───────────────────────────── atlas grid + cell math ─────────────────────────────

/// `texcache_set_cellsize` (0x5cf360): set the current row count to our predicted `256/cellsize` so the
/// binary takes the no-realloc path, then assert it (a) stores font at this+4 and (b) inits exactly that
/// many bucket-rows' free width to 0x100 (a divergent count would miss the no-op path and fault).
#[test]
fn texcache_set_cellsize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x20) as u32; // texcache: +0x10 row count, +0x14 row array
    let rows_buf = e.alloc(0x100 * 0x10) as u32; // up to 256 rows × 0x10 bytes
    let canary: u32 = 0xdead_beef;
    let mut s: u32 = 0x5cf3_6001;
    for _ in 0..256u32 {
        let cellsize = pick(&mut s, 1, 256) as u32;
        let font: u32 = (next(&mut s) | 1) & 0x00ff_ffff; // any nonzero (only stored, not deref'd)
        let n = atlas_rows(cellsize);
        // pre-arm the no-realloc path + a canary row just past the predicted count.
        e.write_u32(u64::from(this) + 0x10, n);
        e.write_u32(u64::from(this) + 0x14, rows_buf);
        for i in 0..=n {
            e.write_u32(u64::from(rows_buf) + u64::from(i) * 0x10, canary);
        }
        e.call(TEXCACHE_SET_CELLSIZE, this, 0, &[font, cellsize]);
        assert_eq!(
            e.read_u32(u64::from(this) + 4),
            font,
            "cellsize={cellsize}: font@+4"
        );
        for i in 0..n {
            assert_eq!(
                e.read_u32(u64::from(rows_buf) + u64::from(i) * 0x10),
                ATLAS_ROW_FREE_WIDTH,
                "cellsize={cellsize} row {i} free-width"
            );
        }
        assert_eq!(
            e.read_u32(u64::from(rows_buf) + u64::from(n) * 0x10),
            canary,
            "cellsize={cellsize}: row count == 256/cellsize (canary at row {n} untouched)"
        );
    }
}

/// `atlas_texel_addr` (0x5cf5f0): stub the blit dispatcher and read back the dest pointer it would be
/// handed = `0xc2ba48 + 2·((page·cellsize)·256 + x0)`.
#[test]
fn atlas_texel_addr_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(BLIT_DISPATCH, 0, 2); // dispatcher: ret 8 (src, dst)
    let row = e.alloc(0x10) as u32; // cache-row: +0x00 nonzero, +0x04 font
    let font = e.alloc(0x250) as u32;
    let glyph = e.alloc(0x40) as u32;
    e.write_u32(u64::from(row), 1); // [row]!=0 guard
    e.write_u32(u64::from(row) + 4, font); // [row+4]=font
    let mut s: u32 = 0x5cf5_f001;
    for _ in 0..2000u32 {
        let page = pick_i32(&mut s, 0, 12);
        let cellsize = pick_i32(&mut s, 1, 32);
        let x0 = pick_i32(&mut s, 0, 255);
        e.write_u32(u64::from(font) + FONT_RASTER_PX, cellsize as u32);
        e.write_u32(u64::from(glyph) + 0x2c, page as u32); // +0x2c page index
        e.write_u32(u64::from(glyph) + 0x30, x0 as u32); // +0x30 cell x-start
        e.clear_calls();
        e.call(ATLAS_TEXEL_ADDR, row, 0, &[glyph]);
        let calls = e.calls();
        assert_eq!(
            calls.len(),
            1,
            "page={page} cell={cellsize} x0={x0}: one dispatch"
        );
        let dest = calls[0].args[1];
        assert_eq!(
            dest,
            atlas_texel_addr(page, cellsize, x0),
            "atlas_texel_addr page={page} cell={cellsize} x0={x0} base={STAGING_BASE:#x}"
        );
    }
}

/// `glyph_uv_rect` (0x5c7f80): fill desc+0x60..0x6c with the UV rect and diff each f32.
#[test]
fn glyph_uv_rect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let desc = e.alloc(0x80) as u32;
    let mut s: u32 = 0x5c7f_8001;
    for _ in 0..3000u32 {
        let a = pick_i32(&mut s, 0, 4096);
        let b = pick_i32(&mut s, 1, 64);
        let cell_x0 = pick_i32(&mut s, 0, 255);
        let advance_px = pick_i32(&mut s, 0, 255);
        e.write_u32(u64::from(desc) + 0x30, cell_x0 as u32);
        e.write_u32(u64::from(desc) + 0x48, advance_px as u32);
        e.call(GLYPH_UV_RECT, desc, 0, &[a as u32, b as u32]);
        let ours = glyph_uv_rect(a, b, cell_x0, advance_px);
        for (i, (off, name)) in [
            (0x60u64, "v_bottom"),
            (0x64, "u_left"),
            (0x68, "v_top"),
            (0x6c, "u_right"),
        ]
        .iter()
        .enumerate()
        {
            assert_f32(
                e.read_f32(u64::from(desc) + off),
                ours[i],
                &format!("uv_rect {name} a={a} b={b} x0={cell_x0} adv={advance_px}"),
            );
        }
    }
}

/// `cell_free_after` (0x5c8010): free span right of a glyph — real next, null, and tagged-link variants.
#[test]
fn cell_free_after_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let desc = e.alloc(0x40) as u32;
    let next_node = e.alloc(0x40) as u32;
    let mut s: u32 = 0x5c80_1001;
    for _ in 0..3000u32 {
        let this_x1 = pick_i32(&mut s, 0, 255);
        let next_x0 = pick_i32(&mut s, 0, 255);
        let variant = (next(&mut s) >> 8) % 3;
        let next_link = match variant {
            0 => next_node,     // real node (aligned ⇒ low bit clear)
            1 => 0,             // null
            _ => next_node | 1, // tagged sentinel
        };
        e.write_u32(u64::from(desc) + 0x34, this_x1 as u32);
        e.write_u32(u64::from(desc) + 0x1c, next_link);
        e.write_u32(u64::from(next_node) + 0x30, next_x0 as u32);
        let bin = e.call(CELL_FREE_AFTER, desc, 0, &[]) as i32;
        assert_eq!(
            bin,
            cell_free_after(next_link, next_x0, this_x1),
            "cell_free_after v={variant} this_x1={this_x1} next_x0={next_x0} link={next_link:#x}"
        );
    }
}

/// `cell_free_before` (0x5c8030): free span left of a glyph — real prev, null, and tagged-link variants.
#[test]
fn cell_free_before_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let desc = e.alloc(0x40) as u32;
    let head_slot = e.alloc(0x10) as u32; // [desc+0x18] -> head_slot
    let node = e.alloc(0x40) as u32; // [head_slot] -> node; [node+4] = prev link
    let prev = e.alloc(0x40) as u32;
    let mut s: u32 = 0x5c80_3001;
    e.write_u32(u64::from(desc) + 0x18, head_slot);
    e.write_u32(u64::from(head_slot), node);
    for _ in 0..3000u32 {
        let this_x0 = pick_i32(&mut s, 0, 255);
        let prev_x1 = pick_i32(&mut s, 0, 255);
        let variant = (next(&mut s) >> 8) % 3;
        let prev_link = match variant {
            0 => prev,
            1 => 0,
            _ => prev | 1,
        };
        e.write_u32(u64::from(desc) + 0x30, this_x0 as u32);
        e.write_u32(u64::from(node) + 4, prev_link);
        e.write_u32(u64::from(prev) + 0x34, prev_x1 as u32);
        let bin = e.call(CELL_FREE_BEFORE, desc, 0, &[]) as i32;
        assert_eq!(
            bin,
            cell_free_before(prev_link, prev_x1, this_x0),
            "cell_free_before v={variant} this_x0={this_x0} prev_x1={prev_x1} link={prev_link:#x}"
        );
    }
}

/// `glyph_vplace` (0x5d1360): seat the glyph so `baseline = cellTop + ascender`. The ascender is the
/// `[ebp+0x10]` stack arg (= `CGxFont+0x17c` load_param); `[[ctx+0x54]+0x68]` is the per-glyph FT
/// `bitmap_top`, which the emulator writes into the synthesized slot at +0x68. Deterministic probes
/// first: rows a step below, at and a step above the cell height (the unsigned `jbe` at `0x5d1371`),
/// rows far above it and a negative cell, each with the bitmap top below, at and above the ascender;
/// rows above the cell must give `gap = pad = cell − rows`. Then a fuzz.
#[test]
fn glyph_vplace_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ctx = e.alloc(0x60) as u32;
    let slot = e.alloc(0x80) as u32;
    let out_a = e.alloc(0x10) as u32;
    let out_b = e.alloc(0x10) as u32;
    e.write_u32(u64::from(ctx) + 0x54, slot);
    let mut run = |bitmap_top: i32, p4: i32, ascender: i32, p6: i32| -> (i32, i32) {
        e.write_u32(u64::from(slot) + 0x68, bitmap_top as u32);
        e.write_u32(u64::from(out_a), 0);
        e.write_u32(u64::from(out_b), 0);
        e.call(
            GLYPH_VPLACE,
            ctx,
            out_a,
            &[out_b, p4 as u32, ascender as u32, p6 as u32],
        );
        let got = (
            e.read_u32(u64::from(out_a)) as i32,
            e.read_u32(u64::from(out_b)) as i32,
        );
        let (ea, eb) = glyph_vplace(bitmap_top, p4, ascender, p6);
        assert_eq!(
            got.0, ea,
            "vplace.out top={bitmap_top} p4={p4} asc={ascender} p6={p6}"
        );
        assert_eq!(
            got.1, eb,
            "vplace.rows top={bitmap_top} p4={p4} asc={ascender} p6={p6}"
        );
        got
    };
    for (p4, p6) in [(12, 11), (12, 12), (12, 13), (12, 40), (0, 1), (-3, 5)] {
        for (bitmap_top, ascender) in [(5, 9), (9, 9), (14, 9), (-4, 9)] {
            let (gap, pad) = run(bitmap_top, p4, ascender, p6);
            if (p6 as u32) > (p4 as u32) {
                assert_eq!((gap, pad), (p4 - p6, p4 - p6), "rows above the cell");
            }
        }
    }
    let mut s: u32 = 0x5d13_6001;
    for _ in 0..4000u32 {
        let bitmap_top = pick_i32(&mut s, -40, 120); // [[ctx+0x54]+0x68], per-glyph FT bitmap_top
        let p4 = pick_i32(&mut s, -10, 200); // requested cell height
        let ascender = pick_i32(&mut s, -40, 120); // [ebp+0x10] stack arg = CGxFont+0x17c load_param
        let p6 = pick_i32(&mut s, -10, 200); // glyph rows
        run(bitmap_top, p4, ascender, p6);
    }
}

// ───────────────────────────── glyph rasterizer (FreeType-boundary stubbed) ─────────────────────────────

/// `rasterize_glyph` (0x5d1120): with the FreeType lib-live / load+render boundary and the bitmap allocator
/// stubbed, the metric arithmetic runs over a synthesized FT glyph slot. Compares the out-struct fields
/// (sizes, width/clip/pitch, advance & bearing 26.6→px floats, and the embedded `glyph_vplace` results) over
/// both the real-bitmap and blank-glyph paths.
#[test]
fn rasterize_glyph_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.stub(FT_LIB_LIVE, 1, 0); // FT lib live ⇒ true
    e.stub(FT_LOAD_RENDER, 1, 1); // FT load+render ⇒ true (ret 4)
    let bump = e.alloc(0x1000) as u32;
    e.stub_alloc(SMEM_ALLOC, u64::from(bump), 0x800, 4); // bitmap alloc (ret 0x10)

    let face = e.alloc(0x60) as u32;
    let slot = e.alloc(0x80) as u32;
    let buffer = e.alloc(0x400) as u32; // FT bitmap source
    let out = e.alloc(0x40) as u32;
    e.write_u32(u64::from(face) + 0x54, slot);

    let mut s: u32 = 0x5d11_2001;
    for _ in 0..2000u32 {
        let blank = (next(&mut s) >> 8) & 1 == 0;
        let code = pick_i32(&mut s, 1, 32);
        let rows = pick_i32(&mut s, 1, 8);
        let width = if blank { 0 } else { pick_i32(&mut s, 1, 8) };
        let pitch = pick_i32(&mut s, 1, 8);
        let buf_nonzero = !blank;
        let advance = pick_i32(&mut s, -4096, 4096);
        let bearing = pick_i32(&mut s, -4096, 4096);
        let bitmap_top = pick_i32(&mut s, -40, 120); // slot+0x68, per-glyph FT bitmap_top
        let ascender = pick_i32(&mut s, -10, 64); // [ebp+0xc] arg = CGxFont+0x17c load_param
        let render_flag = (next(&mut s) >> 8) & 1 == 1;
        let adv_adj = pick_i32(&mut s, -8, 8);

        // synthesize the FT glyph-slot.
        e.write_u32(u64::from(slot) + 0x4c, rows as u32);
        e.write_u32(u64::from(slot) + 0x50, width as u32);
        e.write_u32(u64::from(slot) + 0x54, pitch as u32);
        e.write_u32(u64::from(slot) + 0x58, if buf_nonzero { buffer } else { 0 });
        e.write_u32(u64::from(slot) + 0x28, advance as u32);
        e.write_u32(u64::from(slot) + 0x20, bearing as u32);
        e.write_u32(u64::from(slot) + 0x68, bitmap_top as u32);
        for j in 0..0x100u64 {
            e.write_u32(u64::from(buffer) + j * 4, next(&mut s));
        }
        // clear the out struct so unwritten fields read 0.
        e.write(u64::from(out), &[0u8; 0x40]);
        e.reset_bump(u64::from(bump));

        e.call(
            RASTERIZE_GLYPH,
            face,
            code as u32,
            &[
                0,
                ascender as u32,
                out,
                u32::from(render_flag),
                adv_adj as u32,
            ],
        );

        let ours = rasterize_glyph(
            code,
            render_flag,
            adv_adj,
            ascender,
            rows,
            width,
            pitch,
            buf_nonzero,
            advance,
            bearing,
            bitmap_top,
        );
        let bin = RasterMetrics {
            size: e.read_u32(u64::from(out) + 0x04),
            width: e.read_u32(u64::from(out) + 0x08) as i32,
            clipped_rows: e.read_u32(u64::from(out) + 0x0c) as i32,
            width_plus_adj: e.read_u32(u64::from(out) + 0x10) as i32,
            pitch: e.read_u32(u64::from(out) + 0x1c) as i32,
            vplace: (
                e.read_u32(u64::from(out) + 0x20) as i32,
                e.read_u32(u64::from(out) + 0x24) as i32,
            ),
            adv_metric_bits: e.read_u32(u64::from(out) + 0x14),
            bearing_metric_bits: e.read_u32(u64::from(out) + 0x18),
        };
        let ctx = format!(
            "rasterize blank={blank} code={code} rows={rows} w={width} pitch={pitch} adv={advance} \
             bear={bearing} asc={ascender} bt={bitmap_top} rf={render_flag} adj={adv_adj}"
        );
        assert_eq!(bin.size, ours.size, "{ctx}: size");
        assert_eq!(bin.width, ours.width, "{ctx}: width");
        assert_eq!(bin.clipped_rows, ours.clipped_rows, "{ctx}: clipped_rows");
        assert_eq!(bin.width_plus_adj, ours.width_plus_adj, "{ctx}: width+adj");
        assert_eq!(bin.pitch, ours.pitch, "{ctx}: pitch");
        assert_eq!(bin.vplace, ours.vplace, "{ctx}: vplace");
        assert_f32(
            f32::from_bits(bin.adv_metric_bits),
            f32::from_bits(ours.adv_metric_bits),
            &format!("{ctx}: adv_metric"),
        );
        assert_f32(
            f32::from_bits(bin.bearing_metric_bits),
            f32::from_bits(ours.bearing_metric_bits),
            &format!("{ctx}: bearing_metric"),
        );
    }
}

// Keep the `ATLAS_ROW_STRIDE` re-export referenced (the blit row stride the transcription documents).
const _: () = assert!(ATLAS_ROW_STRIDE == 0x100);
