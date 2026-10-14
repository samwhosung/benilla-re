//! Bit-exact tests of the text extent, wrap, fixed and proportional step, and the GxuFont C-API fit
//! kernels against `WoW.exe`.
//!
//! ## Deterministic environment (no FreeType, no gx device)
//! The kernels resolve, per glyph, an advance (`char_advance` 0x5c6b70, then `GlyphWidth` 0x5cb080) and
//! an escapement (0x5ca2d0 and 0x5ca4b0). Both are pinned:
//! - `0x5cabd0` (NewCodeDesc) is stubbed to return one synthesized `CharCodeDesc` (so `GlyphWidth`
//!   runs for real on its +0x50 width and +0x48 advance; the glyph hash is bypassed).
//! - the kerning hash holds one cached `KernNode` for the pair `(G,G)` (flags bits 0x1 and 0x2 set,
//!   cached step at +0x1c and +0x20), so the escapement returns it directly, with no face, no GDI
//!   and no allocation.
//!
//! Every test string is built from a single glyph byte `G` in `{A,-,B}` plus control codes that
//! decode to a non-glyph class (|c colour, |r restore, |h link-close, |n, CR and LF newlines), so
//! the only glyph pair the kernels ever query is `(G,G)`, which the cached node answers. (The decode
//! gate bits 0x100 to 0x800 are kept out of the fuzzed `flags` so those sequences stay active.)
//! Sub-functions that run for real: `0x5c2810` decode, `0x5cb080` width, `0x5c6fa0` and `0x5c7010`
//! screen-to-pixel, `0x5cae90` pixel size, `0x5c2540` default, `0x73fdf5` ceil, `0x40a2b0` ftol and
//! `0x5c7780` line-break.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_font::measure::{
    font_height_fit, font_textblock_height, font_width_fit, measure_run_fixedwidth,
    measure_run_proportional, text_extent_measure, wrap_max_chars, FontCtx,
};
use common::{assert_f32, fuzz_unit, g, next};

// ── primitive addresses ──
const TEXT_EXTENT: u64 = 0x5c6940;
const WRAP_MAX_CHARS: u64 = 0x5c6c50;
const MEASURE_FIXED: u64 = 0x5c7300;
const MEASURE_PROP: u64 = 0x5c7470;
const FONT_TEXTBLOCK_HEIGHT: u64 = 0x5c2070;
const FONT_HEIGHT_FIT: u64 = 0x5c21c0;
const FONT_WIDTH_FIT: u64 = 0x5c2320;
const NEW_CODE_DESC: u64 = 0x5cabd0; // stubbed → the synthesized desc

// ── CGxFont offsets ──
const F_GLYPH_BUCKETS: u64 = 0x30;
const F_GLYPH_MASK: u64 = 0x38;
const F_KERN_BUCKETS: u64 = 0x58;
const F_KERN_MASK: u64 = 0x60;
const F_FLAGS: u64 = 0x180;
const F_ADV_ADJUST: u64 = 0x188;
const F_PIXEL_SIZE: u64 = 0x24c;
const F_SIZE: u64 = 0x250;
// ── CharCodeDesc offsets ──
const D_KEY: u64 = 0x00;
const D_NEXT: u64 = 0x20;
const D_ADVANCE_PX: u64 = 0x48;
const D_STEP_METRIC: u64 = 0x4c;
const D_WIDTH_METRIC: u64 = 0x50;
const D_SIZE: u64 = 0x70;
// ── KernNode offsets ──
const K_KEY: u64 = 0x00;
const K_PAIRKEY: u64 = 0x14;
const K_FLAGS: u64 = 0x18;
const K_STEP_2D0: u64 = 0x1c; // cached step read by 0x5ca2d0
const K_STEP_4B0: u64 = 0x20; // cached step read by 0x5ca4b0
const K_SIZE: u64 = 0x40;
// ── bucket entry ──
const BUCKET_LINKOFF: u64 = 0x0;
const BUCKET_HEAD: u64 = 0x8;
const BUCKET_SIZE: u64 = 0xc;

const RES: &[(i32, i32)] = &[
    (640, 480),
    (800, 600),
    (1024, 768),
    (1280, 1024),
    (1600, 1200),
];
const PIXEL_SIZES: &[i32] = &[8, 11, 12, 16, 24, 32];
const GLYPHS: &[u8] = b"A-B";

/// A fuzzed f32 in `[-hi, hi)`.
fn fuzz_signed(s: &mut u32, hi: f32) -> f32 {
    fuzz_unit(s, 2.0 * hi) - hi
}

/// kern pair-key = `(prev<<16) ^ (cur & 0xffff)` (0x5ca2d0/0x5ca4b0).
fn pair_key(prev: u32, cur: u32) -> u32 {
    (prev << 16) ^ (cur & 0xffff)
}

/// The synthesized world: a CGxFont with a stubbed glyph desc + one cached kern node.
struct World {
    font: u64,
    cdesc: u64,
    knode: u64,
    text: u64,
    o0: u64,
    o1: u64,
    o2: u64,
    o3: u64,
    cum: u64,
    cum_end: u64,
    state: u64,
}

const TEXT_CAP: u64 = 0x100;
const CUM_CAP: usize = 64;

fn setup(e: &mut Emu) -> World {
    let font = e.alloc(F_SIZE);
    let gbuckets = e.alloc(BUCKET_SIZE);
    let cdesc = e.alloc(D_SIZE);
    let kbuckets = e.alloc(BUCKET_SIZE);
    let knode = e.alloc(K_SIZE);
    let text = e.alloc(TEXT_CAP);
    let o0 = e.alloc(0x10);
    let o1 = e.alloc(0x10);
    let o2 = e.alloc(0x10);
    let o3 = e.alloc(0x10);
    let cum = e.alloc((CUM_CAP as u64) * 4);
    let cum_end = cum + (CUM_CAP as u64) * 4;
    let state = e.alloc(0x10);
    e.write_u32(state, 0); // *state = 0 → extraSpacing 0 in textblock_height

    // glyph hash empty (bypassed by the 0x5cabd0 stub) — keep it well-formed/non-crashing.
    e.write_u32(font + F_GLYPH_MASK, 0xffff_ffff);
    e.write_u32(font + F_GLYPH_BUCKETS, gbuckets as u32);
    e.write_u32(gbuckets + BUCKET_LINKOFF, D_NEXT as u32);
    e.write_u32(gbuckets + BUCKET_HEAD, cdesc as u32);
    e.write_u32(cdesc + D_NEXT, 0);

    // kerning hash: single bucket (mask 0), one cached node for (G,G) (G written per case).
    e.write_u32(font + F_KERN_MASK, 0);
    e.write_u32(font + F_KERN_BUCKETS, kbuckets as u32);
    e.write_u32(kbuckets + BUCKET_LINKOFF, BUCKET_SIZE as u32);
    e.write_u32(kbuckets + BUCKET_HEAD, knode as u32);
    e.write_u32(knode + K_FLAGS, 0x3); // bit0 (0x5ca4b0) + bit1 (0x5ca2d0) = cached

    e.write_u32(font + F_ADV_ADJUST, 0); // irrelevant on the cached path
                                         // 0x5cabd0: thiscall ecx=font, 1 stack arg (code) → desc.
    e.stub(NEW_CODE_DESC, cdesc as u32, 1);
    World {
        font,
        cdesc,
        knode,
        text,
        o0,
        o1,
        o2,
        o3,
        cum,
        cum_end,
        state,
    }
}

/// Per-case knobs (the fuzzed environment + measure params).
struct Case {
    g: u8,
    width_metric: f32,
    advance_px: i32,
    k: f32,
    pixel_size: i32,
    font_flags: u32,
    screen_w: i32,
    screen_h: i32,
}

fn gen_case(s: &mut u32) -> Case {
    let g = GLYPHS[(next(s) >> 8) as usize % GLYPHS.len()];
    let (screen_w, screen_h) = RES[(next(s) >> 8) as usize % RES.len()];
    Case {
        g,
        width_metric: fuzz_signed(s, 48.0),
        advance_px: (next(s) >> 8) as i32 % 250 - 80,
        k: fuzz_signed(s, 24.0),
        pixel_size: PIXEL_SIZES[(next(s) >> 8) as usize % PIXEL_SIZES.len()],
        // font pad flags: 0 / shadow(0x1) / outline(0x8) / both(0x9)
        font_flags: [0u32, 0x1, 0x8, 0x9][(next(s) >> 8) as usize % 4],
        screen_w,
        screen_h,
    }
}

/// Install a case into the emulator world + return a matching [`FontCtx`] closure environment.
fn apply(e: &mut Emu, w: &World, c: &Case) -> (f32, i32, f32) {
    e.write_u32(w.cdesc + D_KEY, u32::from(c.g));
    e.write_u32(w.cdesc + D_WIDTH_METRIC, c.width_metric.to_bits());
    e.write_u32(w.cdesc + D_ADVANCE_PX, c.advance_px as u32);
    e.write_u32(w.cdesc + D_STEP_METRIC, 0); // unused (cached escapement)
    e.write_u32(w.knode + K_KEY, u32::from(c.g));
    e.write_u32(
        w.knode + K_PAIRKEY,
        pair_key(u32::from(c.g), u32::from(c.g)),
    );
    e.write_u32(w.knode + K_STEP_2D0, c.k.to_bits());
    e.write_u32(w.knode + K_STEP_4B0, c.k.to_bits());
    e.write_u32(w.font + F_PIXEL_SIZE, c.pixel_size as u32);
    e.write_u32(w.font + F_FLAGS, c.font_flags);
    e.write_u32(g::SCREEN_W, c.screen_w as u32);
    e.write_u32(g::SCREEN_H, c.screen_h as u32);
    (c.width_metric, c.advance_px, c.k)
}

/// Build a NUL-terminated test string from glyph `g` + non-glyph control/newline tokens, and write it
/// into the emulator at `w.text`. Returns the bytes (without the NUL).
fn build_text(s: &mut u32, e: &mut Emu, w: &World, g: u8, max_tokens: u32) -> Vec<u8> {
    let mut v = Vec::new();
    let n = (next(s) >> 20) % (max_tokens + 1);
    for _ in 0..n {
        if v.len() > 40 {
            break;
        }
        match (next(s) >> 22) % 9 {
            0..=3 => v.push(g),
            4 => v.extend_from_slice(b"|cffffffff"), // class 0
            5 => v.extend_from_slice(b"|r"),         // class 1
            6 => v.extend_from_slice(b"|h"),         // class 5
            7 => v.push(b'\n'),                      // class 2 (LF)
            _ => v.extend_from_slice(b"|n"),         // class 2 (markup newline)
        }
    }
    assert!((v.len() as u64) < TEXT_CAP);
    e.write(w.text, &v);
    e.write_u32(w.text + v.len() as u64, 0); // NUL terminate
    v
}

/// Build the [`FontCtx`] for the Rust transcription (closures mirror the cached desc + kern node).
macro_rules! ctx_for {
    ($c:expr, $width:expr, $adv:expr, $k:expr) => {{
        let glyph = move |_code: u32| -> Option<(f32, i32)> { Some(($width, $adv)) };
        let escapement = move |_p: u32, _cu: u32| -> f64 { f64::from($k) };
        (glyph, escapement)
    }};
}

#[test]
fn text_extent_measure_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let w = setup(&mut e);
    let mut s: u32 = 0x5c69_4001;
    for i in 0..6000u32 {
        let c = gen_case(&mut s);
        let (width, adv, k) = apply(&mut e, &w, &c);
        let bytes = build_text(&mut s, &mut e, &w, c.g, 9);
        let flags = next(&mut s) & 0xd6; // only safe bits {0x2,0x4,0x10,0x40,0x80}
        let size = if next(&mut s) >> 28 == 0 {
            0.0
        } else {
            fuzz_unit(&mut s, 0.6)
        };
        let extra = fuzz_signed(&mut s, 4.0);
        let max_bytes = if next(&mut s) >> 30 == 0 {
            ((next(&mut s) >> 8) % (bytes.len() as u32 + 2)) as i32
        } else {
            bytes.len() as i32 + 4
        };

        e.write_u32(w.o0, 0); // *out
        e.call(
            TEXT_EXTENT,
            w.font as u32,
            w.text as u32,
            &[
                max_bytes as u32,
                size.to_bits(),
                w.o0 as u32,
                flags,
                extra.to_bits(),
            ],
        );
        let bin = e.read_f32(w.o0);

        let (glyph, escapement) = ctx_for!(c, width, adv, k);
        let ctx = FontCtx {
            font_flags: c.font_flags,
            pixel_size: c.pixel_size,
            screen_w: c.screen_w,
            screen_h: c.screen_h,
            glyph: &glyph,
            escapement: &escapement,
        };
        let ours = text_extent_measure(&ctx, &bytes, max_bytes, size, flags, extra);
        assert_f32(
            bin,
            ours,
            &format!(
                "text_extent[{i}] g={} flags={flags:#x} size={size} extra={extra} mb={max_bytes} text={:?}",
                c.g as char, String::from_utf8_lossy(&bytes)
            ),
        );
    }
}

#[test]
fn wrap_max_chars_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let w = setup(&mut e);
    let mut s: u32 = 0x5c6c_5001;
    for i in 0..6000u32 {
        let c = gen_case(&mut s);
        let (width, adv, k) = apply(&mut e, &w, &c);
        let bytes = build_text(&mut s, &mut e, &w, c.g, 9);
        let flags = next(&mut s) & 0xd6;
        let size = if next(&mut s) >> 28 == 0 {
            0.0
        } else {
            fuzz_unit(&mut s, 0.6)
        };
        let extra = fuzz_signed(&mut s, 4.0);
        let box_frac = fuzz_unit(&mut s, 1.5);
        let max_bytes = bytes.len() as i32 + 4;

        e.write_u32(w.o0, 0); // outWidth
        e.write_u32(w.o1, 0); // outBytes
        let cnt = e.call(
            WRAP_MAX_CHARS,
            w.font as u32,
            w.text as u32,
            &[
                size.to_bits(),
                box_frac.to_bits(),
                max_bytes as u32,
                w.o0 as u32,
                flags,
                extra.to_bits(),
                w.o1 as u32,
                w.cum as u32,
                w.cum_end as u32,
            ],
        ) as i32;
        let bin_width = e.read_f32(w.o0);
        let bin_bytes = e.read_u32(w.o1) as i32;
        let bin_cum: Vec<f32> = (0..cnt.max(0) as u64)
            .map(|j| e.read_f32(w.cum + j * 4))
            .collect();

        let (glyph, escapement) = ctx_for!(c, width, adv, k);
        let ctx = FontCtx {
            font_flags: c.font_flags,
            pixel_size: c.pixel_size,
            screen_w: c.screen_w,
            screen_h: c.screen_h,
            glyph: &glyph,
            escapement: &escapement,
        };
        let mut cum = Vec::new();
        let (ocnt, owidth, obytes) = wrap_max_chars(
            &ctx,
            &bytes,
            size,
            box_frac,
            max_bytes,
            flags,
            extra,
            Some(&mut cum),
        );
        let ctx_s = format!(
            "wrap[{i}] g={} flags={flags:#x} size={size} box={box_frac} extra={extra} text={:?}",
            c.g as char,
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(cnt, ocnt, "{ctx_s}: count");
        assert_f32(bin_width, owidth, &format!("{ctx_s}: width"));
        assert_eq!(bin_bytes, obytes, "{ctx_s}: bytes");
        assert_eq!(bin_cum.len(), cum.len(), "{ctx_s}: cum len");
        for (j, (b, o)) in bin_cum.iter().zip(cum.iter()).enumerate() {
            assert_f32(*b, *o, &format!("{ctx_s}: cum[{j}]"));
        }
    }
}

#[test]
fn measure_run_fixedwidth_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let w = setup(&mut e);
    let mut s: u32 = 0x5c73_0001;
    for i in 0..6000u32 {
        let c = gen_case(&mut s);
        let (width, adv, k) = apply(&mut e, &w, &c);
        let bytes = build_text(&mut s, &mut e, &w, c.g, 9);
        let flags = next(&mut s) & 0xd6;
        // size > 0 here: the `size==0` zero-return (which writes a literal NULL outNext, not a pointer)
        // is probed separately below to avoid the offset conversion ambiguity.
        let size = fuzz_unit(&mut s, 0.6) + 0.001;

        e.write_u32(w.o0, 0); // outBytes
        e.write_u32(w.o1, 0); // outWidth
        e.write_u32(w.o2, 0); // outNext
        e.call(
            MEASURE_FIXED,
            w.font as u32,
            w.text as u32,
            &[size.to_bits(), w.o0 as u32, w.o1 as u32, w.o2 as u32, flags],
        );
        let bin_bytes = e.read_u32(w.o0) as i32;
        let bin_width = e.read_f32(w.o1);
        let bin_next = (e.read_u32(w.o2) as u64).wrapping_sub(w.text) as i32;

        let (glyph, escapement) = ctx_for!(c, width, adv, k);
        let ctx = FontCtx {
            font_flags: c.font_flags,
            pixel_size: c.pixel_size,
            screen_w: c.screen_w,
            screen_h: c.screen_h,
            glyph: &glyph,
            escapement: &escapement,
        };
        let (obytes, owidth, onext) = measure_run_fixedwidth(&ctx, &bytes, size, flags);
        let ctx_s = format!(
            "fixed[{i}] g={} flags={flags:#x} size={size} text={:?}",
            c.g as char,
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(bin_bytes, obytes, "{ctx_s}: bytes");
        assert_f32(bin_width, owidth, &format!("{ctx_s}: width"));
        assert_eq!(bin_next, onext, "{ctx_s}: next");
    }
    // size == 0 probe: the binary zeroes all three outs (outNext = literal 0).
    {
        let c = gen_case(&mut s);
        let (_w, _a, _k) = apply(&mut e, &w, &c);
        e.write(w.text, b"AAA\0");
        e.write_u32(w.o0, 0xdead);
        e.write_u32(w.o1, 0xdead);
        e.write_u32(w.o2, 0xdead);
        e.call(
            MEASURE_FIXED,
            w.font as u32,
            w.text as u32,
            &[0u32, w.o0 as u32, w.o1 as u32, w.o2 as u32, 0],
        );
        assert_eq!(e.read_u32(w.o0), 0, "fixed size0: bytes");
        assert_eq!(e.read_u32(w.o1), 0, "fixed size0: width");
        assert_eq!(e.read_u32(w.o2), 0, "fixed size0: next");
    }
}

#[test]
fn measure_run_proportional_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let w = setup(&mut e);
    let mut s: u32 = 0x5c74_7001;
    for i in 0..8000u32 {
        let c = gen_case(&mut s);
        let (width, adv, k) = apply(&mut e, &w, &c);
        let bytes = build_text(&mut s, &mut e, &w, c.g, 9);
        let flags = next(&mut s) & 0xd6;
        let size = if next(&mut s) >> 28 == 0 {
            0.0
        } else {
            fuzz_unit(&mut s, 0.6)
        };
        let extra = fuzz_signed(&mut s, 4.0);
        let box_frac = fuzz_unit(&mut s, 1.2);
        let broke_in = (next(&mut s) >> 31) as u8;

        e.write_u32(w.o0, 0); // out
        e.write_u32(w.o1, 0); // outWidth
        e.write_u32(w.o2, 0); // outNext
        e.write_u32(w.o3, u32::from(broke_in)); // brokeFlag byte
        e.call(
            MEASURE_PROP,
            w.font as u32,
            w.text as u32,
            &[
                size.to_bits(),
                box_frac.to_bits(),
                w.o0 as u32,
                w.o1 as u32,
                w.o2 as u32,
                extra.to_bits(),
                flags,
                w.o3 as u32,
            ],
        );
        let bin_out = e.read_u32(w.o0) as i32;
        let bin_width = e.read_f32(w.o1);
        let bin_next = (e.read_u32(w.o2) as u64).wrapping_sub(w.text) as i32;
        let bin_broke = (e.read_u32(w.o3) & 0xff) as u8;

        let (glyph, escapement) = ctx_for!(c, width, adv, k);
        let ctx = FontCtx {
            font_flags: c.font_flags,
            pixel_size: c.pixel_size,
            screen_w: c.screen_w,
            screen_h: c.screen_h,
            glyph: &glyph,
            escapement: &escapement,
        };
        let ps = measure_run_proportional(&ctx, &bytes, size, box_frac, extra, flags, broke_in);
        let ctx_s = format!(
            "prop[{i}] g={} flags={flags:#x} size={size} box={box_frac} extra={extra} broke={broke_in} text={:?}",
            c.g as char,
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(bin_out, ps.out_bytes, "{ctx_s}: out");
        assert_f32(bin_width, ps.width, &format!("{ctx_s}: width"));
        assert_eq!(bin_next, ps.next, "{ctx_s}: next");
        assert_eq!(bin_broke, ps.broke, "{ctx_s}: broke");
    }
}

#[test]
fn font_width_fit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let w = setup(&mut e);
    let mut s: u32 = 0x5c23_2001;
    for i in 0..6000u32 {
        let c = gen_case(&mut s);
        let (width, adv, k) = apply(&mut e, &w, &c);
        let bytes = build_text(&mut s, &mut e, &w, c.g, 8);
        let flags = next(&mut s) & 0xd6;
        let size = if next(&mut s) >> 28 == 0 {
            0.0
        } else {
            fuzz_unit(&mut s, 0.6)
        };
        let extra = fuzz_signed(&mut s, 3.0);
        let box_w = fuzz_unit(&mut s, 1.5);
        let n_max = ((next(&mut s) >> 8) % 40 + 1) as i32;

        e.write_u32(w.o0, 0); // outW
        let cnt = e.call(
            FONT_WIDTH_FIT,
            w.font as u32,
            w.text as u32,
            &[
                size.to_bits(),
                box_w.to_bits(),
                n_max as u32,
                w.o0 as u32,
                extra.to_bits(),
                0,
                flags,
            ],
        );
        let bin_w = e.read_f32(w.o0);

        let (glyph, escapement) = ctx_for!(c, width, adv, k);
        let ctx = FontCtx {
            font_flags: c.font_flags,
            pixel_size: c.pixel_size,
            screen_w: c.screen_w,
            screen_h: c.screen_h,
            glyph: &glyph,
            escapement: &escapement,
        };
        let (ocnt, ow) = font_width_fit(&ctx, &bytes, size, box_w, n_max, extra, flags);
        let ctx_s = format!(
            "widthfit[{i}] g={} flags={flags:#x} size={size} box={box_w} nmax={n_max} text={:?}",
            c.g as char,
            String::from_utf8_lossy(&bytes)
        );
        assert_eq!(cnt, ocnt, "{ctx_s}: count");
        assert_f32(bin_w, ow, &format!("{ctx_s}: outW"));
    }
}

#[test]
fn font_textblock_height_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let w = setup(&mut e);
    let mut s: u32 = 0x5c20_7001;
    for i in 0..6000u32 {
        let c = gen_case(&mut s);
        let (width, adv, k) = apply(&mut e, &w, &c);
        let bytes = build_text(&mut s, &mut e, &w, c.g, 9);
        let flags = next(&mut s) & 0xd6;
        let size = fuzz_unit(&mut s, 0.4) + 0.001; // size > 0 (sensible block height)
        let box_w = fuzz_unit(&mut s, 1.0) + 0.05;
        let gap = fuzz_signed(&mut s, 0.3);

        e.write_u32(w.state, 0); // *state = 0
        let bin = e.call_f32(
            FONT_TEXTBLOCK_HEIGHT,
            w.font as u32,
            w.text as u32,
            &[
                size.to_bits(),
                box_w.to_bits(),
                w.state as u32,
                gap.to_bits(),
                flags,
            ],
        );

        let (glyph, escapement) = ctx_for!(c, width, adv, k);
        let ctx = FontCtx {
            font_flags: c.font_flags,
            pixel_size: c.pixel_size,
            screen_w: c.screen_w,
            screen_h: c.screen_h,
            glyph: &glyph,
            escapement: &escapement,
        };
        let ours = font_textblock_height(&ctx, &bytes, size, box_w, 0.0, gap, flags);
        assert_f32(
            bin,
            ours,
            &format!(
                "textblock[{i}] g={} flags={flags:#x} size={size} box={box_w} gap={gap} text={:?}",
                c.g as char,
                String::from_utf8_lossy(&bytes)
            ),
        );
    }
}

#[test]
fn font_height_fit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let w = setup(&mut e);
    let mut s: u32 = 0x5c21_c001;
    for i in 0..6000u32 {
        let c = gen_case(&mut s);
        let (width, adv, k) = apply(&mut e, &w, &c);
        let bytes = build_text(&mut s, &mut e, &w, c.g, 9);
        let flags = next(&mut s) & 0xd6;
        let line_h = fuzz_unit(&mut s, 0.2) + 0.001;
        let wrap_w = fuzz_unit(&mut s, 1.0) + 0.05;
        let box_h = fuzz_unit(&mut s, 1.5) + 0.001;
        let gap = fuzz_signed(&mut s, 0.3);
        let p6 = 1; // non-zero (guard)
        let p7 = 0u32; // extraSpacing pinned 0

        let bin = e.call(
            FONT_HEIGHT_FIT,
            w.font as u32,
            w.text as u32,
            &[
                line_h.to_bits(),
                wrap_w.to_bits(),
                box_h.to_bits(),
                p6,
                p7,
                gap.to_bits(),
                flags,
            ],
        ) as i32;

        let (glyph, escapement) = ctx_for!(c, width, adv, k);
        let ctx = FontCtx {
            font_flags: c.font_flags,
            pixel_size: c.pixel_size,
            screen_w: c.screen_w,
            screen_h: c.screen_h,
            glyph: &glyph,
            escapement: &escapement,
        };
        let ours = font_height_fit(
            &ctx, &bytes, line_h, wrap_w, box_h, p6 as i32, 0.0, gap, flags, 1,
        );
        assert_eq!(
            bin,
            ours,
            "heightfit[{i}] g={} flags={flags:#x} lineH={line_h} wrapW={wrap_w} boxH={box_h} gap={gap} text={:?}",
            c.g as char,
            String::from_utf8_lossy(&bytes)
        );
    }
}
