//! Bit-exact tests of the viewport pixel-dim recompute (0x5c2b50) and the per-font raster-metric
//! derivation (0x5ca030) against `WoW.exe`. The device viewport query (0x5c2b50 calls `0x58a240`, a
//! vtable dispatch through `[0xc0ed38]`) and the rasterizer load (`0x7ce760`, FreeType and GDI) are
//! stubbed; every sub-function that can run does: `0x5caea0` (min-metric, count 0 returns reqSize),
//! `0x5c6fa0` (ScreenToPixelHeight), the `0x5d0370` and `0x5cae90` accessors, and the CRT `__ftol`
//! (0x40a2b0).

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_font::setup::{font_raster_metrics, viewport_pixel_dims};
use common::{assert_f32, fuzz_unit, g, next};

// ── primitive addresses ──
const VIEWPORT_PIXEL_DIMS: u64 = 0x5c2b50;
const FONT_RASTER_METRICS: u64 = 0x5ca030;
const RASTER_LOAD: u64 = 0x7ce760; // 0x7ce760 rasterizer load — stubbed

// ── viewport globals ──
const DEVICE_PTR: u64 = 0x00c0_ed38; // global device ptr 0x58a240 dereferences (→ vtable[0x5c])
const VIEWPORT_GUARD: u64 = 0x00c2_b828; // one-time-init guard (bit0); set to skip the init+callback
const CACHE_X0: u64 = 0x00c2_b82c; // cached last-seen rect (x0,x1,x2,x3 at +0,+4,+8,+0xc)
const FONT_REGISTRY: u64 = 0x00c2_b97c; // live-CGxFont registry head (relayout walk); 0 = empty

// ── CGxFont / face offsets ──
const F_RASTER_PX: u64 = 0x178;
const F_LOAD_PARAM: u64 = 0x17c;
const F_FLAGS: u64 = 0x180;
const F_REQ_SIZE: u64 = 0x184;
const F_ADV_ADJUST: u64 = 0x188;
const F_PIXEL_SIZE: u64 = 0x24c;
const F_FACE_HANDLE: u64 = 0x70;
const F_SIZE: u64 = 0x250;
const HANDLE_FACE: u64 = 0x24; // 0x5d0370 returns *(handle+0x24) = the face
const FACE_COUNT: u64 = 0x1c; // 0x5caea0 min-metric count (0 ⇒ returns reqSize)
const FACE_ADV_DEN: u64 = 0x44; // u16 advance-adjust denominator
const FACE_ASCENDER: u64 = 0x46; // s16
const FACE_DESCENDER: u64 = 0x48; // s16
const FACE_SUB: u64 = 0x58; // ptr to sub-struct; +0xc = u16 advance-adjust numerator
const SUB_ADV_NUM: u64 = 0xc;

const RES: &[i32] = &[480, 600, 768, 1024, 1200];

/// A fuzzed f32 in `[-hi, hi)` from the LCG high bits.
fn fuzz_signed(s: &mut u32, hi: f32) -> f32 {
    fuzz_unit(s, 2.0 * hi) - hi
}

/// Run 0x5c2b50 once on a synthesized device rect and assert the two computed globals byte-match.
fn check_viewport(e: &mut Emu, rect_va: u64, r: [f32; 4], tag: &str) {
    for (i, &v) in r.iter().enumerate() {
        e.write_u32(rect_va + i as u64 * 4, v.to_bits());
        e.write_u32(CACHE_X0 + i as u64 * 4, 0); // reset cache ⇒ change-detection always recomputes
    }
    e.call(VIEWPORT_PIXEL_DIMS, 0, 0, &[]);
    let bin_h = e.read_u32(g::SCREEN_H) as i32;
    let bin_w = e.read_u32(g::SCREEN_W) as i32;
    let (h, w) = viewport_pixel_dims(r);
    assert_eq!(
        bin_h, h,
        "{tag}: screen_h binary={bin_h} ours={h} rect={r:?}"
    );
    assert_eq!(
        bin_w, w,
        "{tag}: screen_w binary={bin_w} ours={w} rect={r:?}"
    );
}

#[test]
fn viewport_pixel_dims_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // 0x58a240 does `mov eax,ecx; push eax; call [[0xc0ed38]+0x5c]` — the rect out-pointer is
    // stack arg #0 at that vtable call. Plant a fake device→vtable→island and write-through the rect.
    let dev = e.alloc(0x10);
    let vt = e.alloc(0x100);
    let island = e.alloc(0x10);
    let rect = e.alloc(0x10);
    e.write_u32(DEVICE_PTR, dev as u32);
    e.write_u32(dev, vt as u32);
    e.write_u32(vt + 0x5c, island as u32);
    e.stub_write_through(island, 0, rect, 16, 0, 1); // arg0 = &rect; vtable fn is `ret 4` (1 arg)
    e.write_u32(VIEWPORT_GUARD, 1); // skip the one-time init (avoids the onexit-callback register)
    e.write_u32(FONT_REGISTRY, 0); // empty registry ⇒ the relayout walk skips

    // explicit edges: default-substitution (zero extent) + valid rects + fractional truncation + neg.
    for &(r, tag) in &[
        ([0.0f32, 0.0, 0.0, 0.0], "all-zero→default"),
        ([0.0, 0.0, 480.0, 640.0], "default-valued"),
        ([10.0, 20.0, 490.0, 660.0], "shifted 480x640"),
        ([0.5, 0.25, 480.5, 640.75], "fractional trunc"),
        ([5.0, 5.0, 5.0, 5.0], "both-extent-0→default"),
        ([0.0, 0.0, 0.0, 640.0], "ext_h=0→default"),
        ([0.0, 0.0, 480.0, 0.0], "ext_w=0→default"),
        ([100.0, 100.0, 99.0, 640.0], "negative ext_h"),
        ([0.0, 0.0, 1080.0, 1920.0], "1920x1080"),
        ([1.9, 0.1, 482.9, 640.1], "trunc both"),
        // NaN, the infinities and extents past `i32` and `i64`: `__ftol` returns the low dword of
        // its qword, 0 for the indefinite, and a zero extent takes the default.
        ([0.0, 0.0, f32::NAN, 640.0], "ext_h NaN"),
        ([0.0, 0.0, 480.0, f32::NAN], "ext_w NaN"),
        ([f32::NEG_INFINITY, 0.0, 480.0, 640.0], "ext_h +inf"),
        ([0.0, f32::INFINITY, 480.0, 640.0], "ext_w -inf"),
        ([0.0, 0.0, 3.0e9, 5.0e9], "extents past i32"),
        ([0.0, 0.0, 1.0e30, -1.0e30], "extents past i64"),
        (
            [0.0, 0.0, 4_294_967_296.0, 8_589_934_592.0],
            "extents 2^32 and 2^33",
        ),
    ] {
        check_viewport(&mut e, rect, r, tag);
    }

    // fuzz: realistic device rects (positive, large extents).
    let mut s: u32 = 0x5c2b_0001;
    for i in 0..3000u32 {
        let r0 = fuzz_unit(&mut s, 100.0);
        let r1 = fuzz_unit(&mut s, 100.0);
        let r2 = fuzz_unit(&mut s, 2000.0);
        let r3 = fuzz_unit(&mut s, 2000.0);
        check_viewport(&mut e, rect, [r0, r1, r2, r3], &format!("fuzz[{i}]"));
    }
    // fuzz: tight signed rects — frequent negative extents + heavy truncation coverage.
    for i in 0..3000u32 {
        let r0 = fuzz_signed(&mut s, 50.0);
        let r1 = fuzz_signed(&mut s, 50.0);
        let r2 = fuzz_signed(&mut s, 80.0);
        let r3 = fuzz_signed(&mut s, 80.0);
        check_viewport(&mut e, rect, [r0, r1, r2, r3], &format!("tight[{i}]"));
    }
}

/// Run 0x5ca030 once and assert the four derived fields byte-match. `face` has metric-count 0 so the
/// `0x5caea0` sub-call returns reqSize unchanged ⇒ `m == reqSize` (full control of the outer max).
#[allow(clippy::too_many_arguments)]
fn check_raster(
    e: &mut Emu,
    font: u64,
    face: u64,
    face_sub: u64,
    screen_h: i32,
    flags: u32,
    req: f32,
    asc: i16,
    desc: i16,
    num: u16,
    den: u16,
    tag: &str,
) {
    e.write_u32(g::SCREEN_H, screen_h as u32);
    e.write_u32(font + F_FLAGS, flags);
    e.write_u32(font + F_REQ_SIZE, req.to_bits());
    e.write(face + FACE_ADV_DEN, &den.to_le_bytes()); // u16 (packed against ascender at +0x46)
    e.write(face + FACE_ASCENDER, &asc.to_le_bytes()); // s16
    e.write(face + FACE_DESCENDER, &desc.to_le_bytes()); // s16
    e.write(face_sub + SUB_ADV_NUM, &num.to_le_bytes()); // u16
    let ret = e.call(FONT_RASTER_METRICS, font as u32, 0, &[]);
    assert_eq!(ret, 1, "{tag}: return (0x7ce760==0) expected 1");
    let ours = font_raster_metrics(f64::from(req), screen_h, flags, asc, desc, num, den);
    assert_eq!(
        e.read_u32(font + F_PIXEL_SIZE) as i32,
        ours.pixel_size,
        "{tag}: pixel_size sh={screen_h} req={req}"
    );
    assert_eq!(
        e.read_u32(font + F_LOAD_PARAM) as i32,
        ours.load_param,
        "{tag}: load_param sh={screen_h} req={req} asc={asc} desc={desc}"
    );
    assert_eq!(
        e.read_u32(font + F_RASTER_PX) as i32,
        ours.atlas_cell,
        "{tag}: atlas_cell flags={flags:#x} sh={screen_h} req={req}"
    );
    assert_f32(
        e.read_f32(font + F_ADV_ADJUST),
        ours.advance_adjust,
        &format!("{tag}: advance_adjust num={num} den={den}"),
    );
}

#[test]
fn font_raster_metrics_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let font = e.alloc(F_SIZE);
    let handle = e.alloc(0x30);
    let face = e.alloc(0x60);
    let face_sub = e.alloc(0x10);
    e.write_u32(font + F_FACE_HANDLE, handle as u32);
    e.write_u32(handle + HANDLE_FACE, face as u32);
    e.write_u32(face + FACE_COUNT, 0); // min-metric count 0 ⇒ 0x5caea0 returns reqSize ⇒ m=req
    e.write_u32(face + FACE_SUB, face_sub as u32);
    e.stub(RASTER_LOAD, 0, 1); // 0x7ce760 → success(0); thiscall + 1 stack arg (ret 4)

    let mut s: u32 = 0x5ca0_3001;
    for &sh in RES {
        // explicit reqSize edges (relative to screenH) hitting every clamp + half-away tie.
        let shf = sh as f32;
        for &(req, tag) in &[
            (0.0, "req0→floor(2px)"),
            (1.0 / shf, "below-floor→2px"),
            (2.0 / shf, "==floor→2px"),
            (2.5 / shf, "tie 2.5→3px"),
            (12.0 / shf, "12px"),
            (31.5 / shf, "tie 31.5→32px(cap)"),
            (32.0 / shf, "32px(cap edge)"),
            (32.5 / shf, "tie 32.5→cap32"),
            (100.0 / shf, "big→cap32"),
            // NaN, the infinities, and sizes whose pixel height passes `i32` and `i64`: `__ftol`
            // returns the low dword of its qword, 0 for the indefinite. The `load_param`
            // conversion takes a render height of at most 32, so only the pixel size reaches one.
            (f32::NAN, "req NaN"),
            (f32::INFINITY, "req +inf"),
            (f32::NEG_INFINITY, "req -inf"),
            (1.0e30, "req 1e30"),
            (-1.0e30, "req -1e30"),
            (4.0e6, "req past i32 px"),
            (-4.0e6, "req past -i32 px"),
        ] {
            for &flags in &[0u32, 0x1, 0x4, 0x8, 0x9, 0xd] {
                check_raster(
                    &mut e,
                    font,
                    face,
                    face_sub,
                    sh,
                    flags,
                    req,
                    1638,
                    -410,
                    1024,
                    2048,
                    &format!("edge {tag}"),
                );
            }
        }
        // fuzz the full domain.
        for i in 0..2500u32 {
            let req = fuzz_unit(&mut s, 0.3); // spans below-floor … above-32-cap
            let flags = next(&mut s) & 0xd; // exercise shadow(0x1)/AA(0x4)/outline(0x8) combos
            let asc = ((next(&mut s) >> 8) % 2048 + 1) as i16; // positive ascender ⇒ denom ≥ 1
            let desc = (((next(&mut s) >> 8) % 4096) as i32 - 2048) as i16;
            let num = (next(&mut s) >> 16) as u16; // [0, 65535]
            let den = ((next(&mut s) >> 16) % 65535 + 1) as u16; // [1, 65535] (nonzero)
            check_raster(
                &mut e,
                font,
                face,
                face_sub,
                sh,
                flags,
                req,
                asc,
                desc,
                num,
                den,
                &format!("fuzz[{i}]"),
            );
        }
    }
}
