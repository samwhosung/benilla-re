//! Bit-exact tests of the per-glyph advance, kerning, step and width functions against `WoW.exe`.
//! The hash-container methods and the face, GDI and lock sub-calls that cannot run in isolation are
//! stubbed (`stub`, `stub_write_through`); the arithmetic under test, and every CRT or hash
//! sub-function that can run (`0x5ca240` glyph-hash walk, `0x5cb080`, `0x5c7080`, `0x73fdf5` ceil),
//! executes for real. The setup of each test is documented inline.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_font::metric::{
    char_advance, compute_step, compute_step_fixedwidth, glyph_metric_outline_bias,
    glyph_width_scaled, min_metric_screen_height,
};
use common::{assert_f32, fuzz_unit, g, next};

// ── primitive addresses ──
const GLYPH_METRIC_OUTLINE_BIAS: u64 = 0x5ca2b0;
const GLYPH_WIDTH_SCALED: u64 = 0x5cb080;
const COMPUTE_STEP: u64 = 0x5ca2d0;
const COMPUTE_STEP_FIXEDWIDTH: u64 = 0x5ca4b0;
const CHAR_ADVANCE: u64 = 0x5c6b70;
const MIN_METRIC_SCREEN_HEIGHT: u64 = 0x5caea0;
// ── stubbed sub-calls ──
const NEW_CODE_DESC: u64 = 0x5cabd0; // glyph-desc lookup/alloc/raster (orchestration) — stubbed
const FACE_GET_CTX: u64 = 0x5d0370; // shaping-ctx getter (face/HDC) — stubbed
const GDI_GLYPH_INDEX: u64 = 0x7ce960; // glyph index for a codepoint — stubbed
const GDI_KERN_QUERY: u64 = 0x7ce830; // raw kern amount (out-param) — stub_write_through

// ── CGxFont offsets ──
const F_GLYPH_BUCKETS: u64 = 0x30;
const F_GLYPH_MASK: u64 = 0x38;
const F_KERN_BUCKETS: u64 = 0x58;
const F_KERN_MASK: u64 = 0x60;
const F_FACE: u64 = 0x70;
const F_RASTER_PX: u64 = 0x178;
const F_FLAGS: u64 = 0x180;
const F_ADV_ADJUST: u64 = 0x188;
const F_PIXEL_SIZE: u64 = 0x24c;
const F_SIZE: u64 = 0x250;
const FLAG_OUTLINE: u32 = 0x8;
// ── CharCodeDesc offsets ──
const D_KEY: u64 = 0x00;
const D_GLYPH_LINK: u64 = 0x1c; // glyph-hash chain link offset (next = *(desc+0x1c+4) = desc+0x20)
const D_NEXT: u64 = 0x20; // (= D_GLYPH_LINK + 4) the chain "next" slot the walk reads
const D_ADVANCE_PX: u64 = 0x48;
const D_STEP_METRIC: u64 = 0x4c;
const D_WIDTH_METRIC: u64 = 0x50;
const D_SIZE: u64 = 0x70;
// ── KernNode offsets ──
const K_KEY: u64 = 0x00;
const K_PAIRKEY: u64 = 0x14;
const K_FLAGS: u64 = 0x18;
const K_SIZE: u64 = 0x40;
// ── intrusive-hash bucket entry: [+0]=link offset, [+8]=chain head; stride 0xc ──
const BUCKET_LINKOFF: u64 = 0x0;
const BUCKET_HEAD: u64 = 0x8;
const BUCKET_SIZE: u64 = 0xc;

const RES: &[i32] = &[480, 600, 768, 1024, 1200];
const PIXEL_SIZES: &[i32] = &[8, 11, 12, 16, 24, 32, 48, 64];

/// f64 NaN-pair-equal assert (mirrors harness `assert_f32` for the `call_f64`/`st0`-observable cases).
fn assert_f64(binary: f64, ours: f64, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#018x} ours={:#018x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// A fuzzed f32 in `[-hi, hi)` from the LCG high bits.
fn fuzz_signed(s: &mut u32, hi: f32) -> f32 {
    fuzz_unit(s, 2.0 * hi) - hi
}

/// pair-key = `(cur<<16) ^ (prev & 0xffff)` (kerning-hash key, byte-exact 0x5ca2d0/0x5ca4b0).
fn pair_key(cur: u32, prev: u32) -> u32 {
    (cur << 16) ^ (prev & 0xffff)
}

#[test]
fn glyph_metric_outline_bias_difftest() {
    // 0x5ca2b0: standalone, no sub-calls. ecx=font (reads flags+0x180); arg = desc+0x38 (reads +0x14).
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let font = e.alloc(F_SIZE);
    let dp = e.alloc(0x20); // the "desc+0x38" region; metric at +0x14
    let mut s: u32 = 0x5ca2_b001;
    for &flags in &[0u32, FLAG_OUTLINE, 0x1, 0x9, 0xd] {
        e.write_u32(font + F_FLAGS, flags);
        let outline = flags & FLAG_OUTLINE != 0;
        // explicit edges around the +1.0 add
        for &m in &[0.0f32, -0.0, 0.5, -0.5, 1.0, -1.0, 7.25, -7.25, 1e7, -1e7] {
            e.write_u32(dp + 0x14, m.to_bits());
            let bin = e.call_f64(GLYPH_METRIC_OUTLINE_BIAS, font as u32, 0, &[dp as u32]);
            assert_f64(
                bin,
                glyph_metric_outline_bias(m, outline),
                &format!("gmob edge flags={flags:#x} m={m}"),
            );
        }
        for i in 0..3000u32 {
            let m = fuzz_signed(&mut s, 256.0);
            e.write_u32(dp + 0x14, m.to_bits());
            let bin = e.call_f64(GLYPH_METRIC_OUTLINE_BIAS, font as u32, 0, &[dp as u32]);
            assert_f64(
                bin,
                glyph_metric_outline_bias(m, outline),
                &format!("gmob[{i}] flags={flags:#x} m={m}"),
            );
        }
    }
}

#[test]
fn glyph_width_scaled_difftest() {
    // 0x5cb080: thiscall ecx=font; args [desc, mode(char), renderSize]. mode==0 → ceil(desc[0x50])
    // (0x73fdf5, runs for real); mode!=0 → (renderSize/font[0x24c])·desc[0x50] (ScreenToPixelHeight
    // passthrough + fidiv + fmul run for real). Diffed on the full st0 via call_f64.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let font = e.alloc(F_SIZE);
    let desc = e.alloc(D_SIZE);
    e.write_u32(g::SCREEN_H, 768); // unused by passthrough, set for hygiene
    let mut s: u32 = 0x5cb0_8001;
    for &px in PIXEL_SIZES {
        e.write_u32(font + F_PIXEL_SIZE, px as u32);
        // pixel mode (ceil) — explicit boundary widths + fuzz
        for &w in &[
            0.0f32, -0.0, 1.0, 2.000_001, 1.999_999, -2.0, -2.5, 2.5, 63.5, -63.5,
        ] {
            e.write_u32(desc + D_WIDTH_METRIC, w.to_bits());
            let bin = e.call_f64(GLYPH_WIDTH_SCALED, font as u32, 0, &[desc as u32, 0, 0]);
            assert_f64(
                bin,
                glyph_width_scaled(false, w, 0.0, px),
                &format!("gws ceil px={px} w={w}"),
            );
        }
        for i in 0..1500u32 {
            let w = fuzz_signed(&mut s, 256.0);
            e.write_u32(desc + D_WIDTH_METRIC, w.to_bits());
            // pixel mode
            let bin0 = e.call_f64(GLYPH_WIDTH_SCALED, font as u32, 0, &[desc as u32, 0, 0]);
            assert_f64(
                bin0,
                glyph_width_scaled(false, w, 0.0, px),
                &format!("gws0[{i}] px={px} w={w}"),
            );
            // scaled mode
            let render = fuzz_signed(&mut s, 256.0);
            let bin1 = e.call_f64(
                GLYPH_WIDTH_SCALED,
                font as u32,
                0,
                &[desc as u32, 1, render.to_bits()],
            );
            assert_f64(
                bin1,
                glyph_width_scaled(true, w, render, px),
                &format!("gws1[{i}] px={px} w={w} r={render}"),
            );
        }
    }
}

#[test]
fn compute_step_difftest() {
    // 0x5ca2d0 recompute path. Kern hash holds ONE matching KernNode (bit1 clear → recompute, not
    // cached-return; node!=0 → the alloc/insert block is skipped), so no kern allocator runs. The
    // glyph hash holds `cur`'s CharCodeDesc so 0x5ca240→0x5ca2b0 (stepbase) runs for real. The face
    // shaping-ctx + GDI glyph-index/kern calls are stubbed; the raw kern is injected via
    // 0x7ce830's out-param. The arithmetic is ceil((f32)(stepbase + min(kern,0)·advanceAdjust)); 0x73fdf5
    // ceil runs for real. Diffed on st0 via call_f64.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let font = e.alloc(F_SIZE);
    let ctx = e.alloc(0x10); // 0x5d0370 result; ctx+8 byte = face has-kerning flag (0x40)
    let kbuckets = e.alloc(BUCKET_SIZE);
    let knode = e.alloc(K_SIZE);
    let gbuckets = e.alloc(BUCKET_SIZE);
    let cdesc = e.alloc(D_SIZE);
    let kern_payload = e.alloc(0x4); // 0x7ce830 writes this into the &local kern slot

    e.write_u32(font + F_KERN_MASK, 0); // bucket = 0 & cur = 0
    e.write_u32(font + F_KERN_BUCKETS, kbuckets as u32);
    e.write_u32(font + F_GLYPH_MASK, 0);
    e.write_u32(font + F_GLYPH_BUCKETS, gbuckets as u32);
    e.write_u32(font + F_FACE, 0xdead_0000);
    e.write_u32(kbuckets + BUCKET_LINKOFF, 0xc); // unused on first-node hit
    e.write_u32(kbuckets + BUCKET_HEAD, knode as u32);
    e.write_u32(gbuckets + BUCKET_LINKOFF, D_GLYPH_LINK as u32);
    e.write_u32(gbuckets + BUCKET_HEAD, cdesc as u32);

    e.stub(FACE_GET_CTX, ctx as u32, 0); // ecx=face ignored; returns ctx
    e.stub(GDI_GLYPH_INDEX, 1, 0); // glyph index — value irrelevant (only a GDI handle)
    e.stub_write_through(GDI_KERN_QUERY, 2, kern_payload, 4, 0, 3); // out-param at stack arg #2

    let mut s: u32 = 0x5ca2_d001;
    for &flags in &[0u32, FLAG_OUTLINE] {
        e.write_u32(font + F_FLAGS, flags);
        let outline = flags & FLAG_OUTLINE != 0;
        for i in 0..900u32 {
            let adj = fuzz_signed(&mut s, 4.0);
            let metric = fuzz_signed(&mut s, 64.0);
            let cur = (next(&mut s) >> 17) | 1; // nonzero codepoint
            let prev = next(&mut s) >> 17;
            let kerning = next(&mut s) >> 31 != 0;
            let raw_kern = (next(&mut s) >> 8) as i32 % 96 - 64; // mix neg/pos

            e.write_u32(font + F_ADV_ADJUST, adj.to_bits());
            e.write_u32(ctx + 0x8, if kerning { 0x40 } else { 0 });
            e.write_u32(kern_payload, raw_kern as u32);
            e.write_u32(cdesc + D_KEY, cur);
            e.write_u32(cdesc + D_STEP_METRIC, metric.to_bits());
            // reset the KernNode each case (clear the step-cached bit set by the prior call)
            e.write_u32(knode + K_KEY, cur);
            e.write_u32(knode + K_PAIRKEY, pair_key(cur, prev));
            e.write_u32(knode + K_FLAGS, 0);

            let bin = e.call_f64(COMPUTE_STEP, font as u32, 0, &[cur, prev]);
            // kerning disabled ⇒ negKern stays 0 (raw kern never read); enabled ⇒ binary clamps min(k,0)
            let eff_kern = if kerning { raw_kern } else { 0 };
            assert_f64(
                bin,
                compute_step(metric, outline, eff_kern, adj),
                &format!(
                    "cstep[{i}] flags={flags:#x} adj={adj} m={metric} kern={kerning} k={raw_kern}"
                ),
            );
        }
    }
}

#[test]
fn compute_step_fixedwidth_difftest() {
    // 0x5ca4b0 glyph-found centering branch. Kern hash EMPTY (font[0x60]=-1) → the top lookup is
    // skipped (no kern node, no alloc). Glyph hash = a 2-node chain descA→descB (link offset 0x1c) so
    // 0x5ca240(g1=codeA) and 0x5ca240(g2=codeB|absent) both resolve for real (→0x5ca2b0 stepbase).
    // No boundary stubs: the centering branch returns before any face/alloc. Diffed on st0 (call_f64).
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let font = e.alloc(F_SIZE);
    let gbuckets = e.alloc(BUCKET_SIZE);
    let desc_a = e.alloc(D_SIZE);
    let desc_b = e.alloc(D_SIZE);
    let code_a: u32 = 0x41; // g1 (always present)
    let code_b: u32 = 0x42; // g2 when present
    let code_absent: u32 = 0x43; // g2 when absent from the hash

    e.write_u32(font + F_KERN_MASK, 0xffff_ffff); // empty kern hash → skip top lookup
    e.write_u32(font + F_GLYPH_MASK, 0);
    e.write_u32(font + F_GLYPH_BUCKETS, gbuckets as u32);
    e.write_u32(gbuckets + BUCKET_LINKOFF, D_GLYPH_LINK as u32);
    e.write_u32(gbuckets + BUCKET_HEAD, desc_a as u32);
    e.write_u32(desc_a + D_KEY, code_a);
    e.write_u32(desc_a + D_NEXT, desc_b as u32); // chain descA → descB
    e.write_u32(desc_b + D_KEY, code_b);
    e.write_u32(desc_b + D_NEXT, 0); // terminate chain

    let mut s: u32 = 0x5ca4_b001;
    for &flags in &[0u32, FLAG_OUTLINE] {
        e.write_u32(font + F_FLAGS, flags);
        let outline = flags & FLAG_OUTLINE != 0;
        for i in 0..1200u32 {
            let r = (next(&mut s) >> 8) % 56 + 8; // raster px 8..63
            let m1 = fuzz_unit(&mut s, r as f32 + 6.0); // span [0, R+) to hit m1>=R sometimes
            let m2 = fuzz_unit(&mut s, r as f32 + 6.0);
            let g2_present = next(&mut s) >> 20 & 1 != 0;

            e.write_u32(font + F_RASTER_PX, r);
            e.write_u32(desc_a + D_STEP_METRIC, m1.to_bits());
            e.write_u32(desc_b + D_STEP_METRIC, m2.to_bits());
            let g2 = if g2_present { code_b } else { code_absent };

            let bin = e.call_f64(COMPUTE_STEP_FIXEDWIDTH, font as u32, 0, &[code_a, g2]);
            let ours =
                compute_step_fixedwidth(m1, if g2_present { Some(m2) } else { None }, outline, r);
            assert_f64(
                bin,
                ours,
                &format!("cstepfw[{i}] flags={flags:#x} R={r} m1={m1} m2={m2} g2={g2_present}"),
            );
        }
    }
}

#[test]
fn char_advance_difftest() {
    // 0x5c6b70 glyph-metric path: pass text="" so the control-code/kern loop is skipped (the explicit
    // kern return is compute_step's, covered there). NewCodeDesc (0x5cabd0, orchestration) is stubbed
    // to return a synthesized CharCodeDesc; GlyphWidth (0x5cb080) then runs for real. The arithmetic =
    // GlyphWidth + (i32)desc[0x48] (fiadd). Diffed on st0 (call_f64). Also checks curCode==0 ⇒ 0.0.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let font = e.alloc(F_SIZE);
    let desc = e.alloc(D_SIZE);
    let text = e.alloc(0x4); // text[0] = 0 → empty string
    e.write_u32(text, 0);
    e.write_u32(g::SCREEN_H, 768);
    e.stub(NEW_CODE_DESC, desc as u32, 1); // thiscall ecx=font, 1 stack arg (code) → ret 4

    let mut s: u32 = 0x5c6b_7001;
    for &px in PIXEL_SIZES {
        e.write_u32(font + F_PIXEL_SIZE, px as u32);
        for i in 0..1500u32 {
            let width = fuzz_signed(&mut s, 256.0);
            let adv = (next(&mut s) >> 8) as i32 % 400 - 200; // signed integer pixel advance
            let render = fuzz_signed(&mut s, 256.0);
            let flags = next(&mut s); // bit7 selects scaled mode in 0x5cb080
            let cur = (next(&mut s) >> 17) | 1; // nonzero codepoint
            let scaled = flags >> 7 & 1 != 0;

            e.write_u32(desc + D_WIDTH_METRIC, width.to_bits());
            e.write_u32(desc + D_ADVANCE_PX, adv as u32);

            let bin = e.call_f64(
                CHAR_ADVANCE,
                cur,
                font as u32,
                &[render.to_bits(), flags, text as u32],
            );
            assert_f64(
                bin,
                char_advance(scaled, width, adv, render, px),
                &format!("cadv[{i}] px={px} scaled={scaled} w={width} adv={adv} r={render}"),
            );
        }
    }
    // curCode == 0 ⇒ fld [0x7ffd74] = 0.0
    let bin0 = e.call_f64(
        CHAR_ADVANCE,
        0,
        font as u32,
        &[1.0f32.to_bits(), 0, text as u32],
    );
    assert_f64(bin0, 0.0, "cadv cur==0");
}

#[test]
fn min_metric_screen_height_difftest() {
    // 0x5caea0: cdecl args [obj, floorFrac]. obj+0x1c = count, obj+0x20 = i16 array (one value per
    // 4-byte slot). 0x5c7080 (min/viewportHeight) runs for real (reads 0xc2b9a0). Returns max(floor,
    // f) with ties→f. Diffed as f32 (call_f32). Also checks count==0 ⇒ floorFrac passthrough.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x30); // fields at +0x1c (count) and +0x20 (array ptr)
    let arr = e.alloc(0x40); // up to 16 int16 values, stride 4
    e.write_u32(obj + 0x20, arr as u32);
    let mut s: u32 = 0x5cae_a001;
    for &h in RES {
        e.write_u32(g::SCREEN_H, h as u32);
        // count == 0 → floorFrac passthrough
        e.write_u32(obj + 0x1c, 0);
        let f0 = 0.375f32;
        let bin0 = e.call_f32(MIN_METRIC_SCREEN_HEIGHT, 0, 0, &[obj as u32, f0.to_bits()]);
        assert_f32(
            bin0,
            min_metric_screen_height(&[], h, f0),
            &format!("mmsh empty h={h}"),
        );
        for i in 0..2000u32 {
            let count = (next(&mut s) >> 8) % 8 + 1; // 1..8
            let mut metrics: Vec<i16> = Vec::with_capacity(count as usize);
            for j in 0..count as u64 {
                let v = (next(&mut s) >> 8) as i16; // full i16 range (signed)
                metrics.push(v);
                e.write_u32(arr + j * 4, v as u16 as u32);
            }
            e.write_u32(obj + 0x1c, count);
            let floor = fuzz_signed(&mut s, 1.5); // straddles the computed fraction both ways
            let bin = e.call_f32(
                MIN_METRIC_SCREEN_HEIGHT,
                0,
                0,
                &[obj as u32, floor.to_bits()],
            );
            assert_f32(
                bin,
                min_metric_screen_height(&metrics, h, floor),
                &format!("mmsh[{i}] h={h} n={count} floor={floor}"),
            );
        }
    }
}
