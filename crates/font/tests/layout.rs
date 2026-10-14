//! Bit-exact tests of the CGxString quad, justify and clamp geometry against `WoW.exe`. The pure
//! arithmetic functions (`resolve_clamp_size`, `anchor_justify_snap`, `hyperlink_region_rect`) run
//! with only their CRT `__ftol` and `floor` sub-calls executing for real; `cgxstring_size_clamp` lets
//! its strlen, strcpy and default-size sub-calls run (the realloc is skipped through a large
//! capacity); `selection_highlight_fade` lets its `SetCell` and `GetCell` sub-calls run against a
//! synthesized page and vertex array; `glyph_quad_emit` stubs only the device-state query
//! (`0x58a230`). The setup of each test is documented inline.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_font::layout::{
    anchor_justify_snap, cgxstring_size_clamp, glyph_quad_build, glyph_quad_emit_color,
    glyph_quad_emit_pen, glyph_quad_emit_vertex, hyperlink_region_rect, resolve_clamp_size,
    selection_highlight_fade, string_layout_lines, AnchorState, EmitVertexIn, EmitVertexOut,
    GlyphQuadInputs, LayoutState,
};
use benilla_re_font::measure::FontCtx;
use common::{assert_f32, fuzz_unit, g, next};

// ── primitive addresses ──
const RESOLVE_CLAMP_SIZE: u64 = 0x5ccb80;
const CGXSTRING_SIZE_CLAMP: u64 = 0x5cd6d0;
const ANCHOR_JUSTIFY_SNAP: u64 = 0x5cdf70;
const HYPERLINK_REGION_RECT: u64 = 0x5cd310;
const SELECTION_HIGHLIGHT_FADE: u64 = 0x5cd4d0;
const GLYPH_QUAD_EMIT: u64 = 0x5c8710;
// ── stubbed / one-to-one sub-calls ──
const INVALIDATE: u64 = 0x5cdef0; // CGxString reset-build-state (orchestration) — stubbed
const DEVICE_QUERY: u64 = 0x58a230; // gx device-state getter (color channel-swap flag) — stubbed

// ── CGxString offsets ──
const S_SIZE: u64 = 0xc4;
const S_RESOLVED_SIZE: u64 = 0x18; // f32 resolved size
const S_CLAMPED_SIZE: u64 = 0x1c; // f32 clamped = max(size, 2.0/viewH)
const S_ANCHOR: u64 = 0x20; // anchor x/y/z (3 f32)
const S_ALPHA_BYTE: u64 = 0x2f; // selection base-alpha byte
const S_POS_X: u64 = 0x3c;
const S_POS_Y: u64 = 0x40;
const S_FONT: u64 = 0x44;
const S_TEXT: u64 = 0x48;
const S_TEXT_CAP: u64 = 0x4c;
const S_JUSTIFY_V: u64 = 0x50;
const S_JUSTIFY_H: u64 = 0x54;
const S_WRAP: u64 = 0x58;
const S_FLAGS: u64 = 0x5c;
const S_SEL_START: u64 = 0x68; // cached selection range
const S_SEL_END: u64 = 0x6c;
const S_ANCHOR_OUT: u64 = 0x70; // computed anchor x/y/z (3 f32)
const S_HLINK_ARR: u64 = 0x7c; // hyperlink growable array {cap, count, buf}
const S_VERT_COUNT_C: u64 = 0xc; // glyph_quad_emit: laid vertex count (+0xc)
const S_VERT_BUF_10: u64 = 0x10; // glyph_quad_emit: laid vertex buffer (+0x10)
const S_GRAD_1C: u64 = 0x1c; // glyph_quad_emit: gradient gate (+0x1c)
const S_GRAD_BUF_20: u64 = 0x20; // glyph_quad_emit: gradient buffer (+0x20)
const S_SEL_COUNT: u64 = 0x90; // selection cell count
const S_SEL_CELLS: u64 = 0x94; // selection cell array (stride 8)
const S_BUILT: u64 = 0x9c; // laid-out count guard
const S_PAGE_LISTS: u64 = 0xa0; // 8 atlas-page vertex lists

// ── CGxFont offsets ──
const F_SIZE: u64 = 0x250;
const F_STRLIST_LINKOFF: u64 = 0x8;
const F_STRLIST_HEAD: u64 = 0xc;
const F_PIXEL_SIZE: u64 = 0x24c;

// ── page (TextLineTexture) offsets used by SetCell/GetCell ──
const P_VERT_COUNT: u64 = 0x1c;
const P_VERT_BUF: u64 = 0x20;

// ── glyph_quad_build (0x5ccbe0) / string_layout_lines (0x5cdc20) ──
const GLYPH_QUAD_BUILD: u64 = 0x5ccbe0;
const STRING_LAYOUT_LINES: u64 = 0x5cdc20;
const NEW_CODE_DESC: u64 = 0x5cabd0; // glyph lookup (orchestration + FreeType raster) — stubbed → desc
const ANCHOR_SNAP: u64 = 0x5cdf70; // CGxString::ComputeAnchor — stubbed (separately diffed)
                                   // CGxString extra offsets
const S_ANCHOR_Z: u64 = 0x28; // anchor.z (glyph origin depth)
const S_COLOR_2C: u64 = 0x2c; // packed colour (per-vertex colour state)
                              // page glyph-quad output arrays: AUVERT geometry {cap@8,count@c,buf@10}; colour {cap@18,count@1c,buf@20}
const P_AUV_CAP: u64 = 0x8;
const P_AUV_COUNT: u64 = 0xc;
const P_AUV_BUF: u64 = 0x10;
const P_COL_CAP: u64 = 0x18;
const P_COL_COUNT: u64 = 0x1c;
const P_COL_BUF: u64 = 0x20;
const AUV_STRIDE: u64 = 0x14;
// CharCodeDesc offsets (the glyph metric record 0x5cabd0 returns)
const D_PAGE_IDX: u64 = 0x28;
const D_ADVANCE: u64 = 0x48;
const D_WIDTH: u64 = 0x50;
const D_TOP: u64 = 0x58;
const D_V_BOTTOM: u64 = 0x60;
const D_U_LEFT: u64 = 0x64;
const D_V_TOP: u64 = 0x68;
const D_U_RIGHT: u64 = 0x6c;
// CGxFont flags (+0x180): bit0 shadow (+2 pad), bit3 outline (+4 pad)
const F_FLAGS_180: u64 = 0x180;
// texel-nudge globals (one-time-init guard + the ±1/512 constants glyph_quad_build's raw path reads)
const G_TEXEL_GUARD: u64 = 0x00c2_b9f0;
const G_TEXEL_VB: u64 = 0x00c2_b9f8;
const G_TEXEL_UL: u64 = 0x00c2_b9fc;
const G_TEXEL_VT: u64 = 0x00c2_ba00;
const G_TEXEL_UR: u64 = 0x00c2_ba04;

const RES_H: &[i32] = &[480, 600, 768, 1024, 1200];
const RES_W: &[i32] = &[640, 800, 1024, 1280, 1600];

/// A fuzzed f32 in `[-hi, hi)` from the LCG high bits.
fn fuzz_signed(s: &mut u32, hi: f32) -> f32 {
    fuzz_unit(s, 2.0 * hi) - hi
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn resolve_clamp_size_difftest() {
    // 0x5ccb80: thiscall ecx=this. flags&4 clear ⇒ [+0x18] (preset) is the resolved size; the clamp
    // writes [+0x1c] = max(size, 2.0/viewH). The trailing 0x5cdef0 (invalidate) is stubbed (it runs
    // after the clamp store; pure orchestration). flags&4 set ⇒ [+0x18] = 0x5c2540(font)=pixelSize/viewH
    // (runs for real), then the same clamp — covered separately.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(S_SIZE);
    let font = e.alloc(F_SIZE);
    e.write_u32(this + S_FONT, font as u32);
    e.stub(INVALIDATE, 0, 0); // __fastcall(this), no stack args

    let mut s: u32 = 0x5ccb_8001;
    for &h in RES_H {
        e.write_u32(g::SCREEN_H, h as u32);
        let floor = 2.0f32 / h as f32; // approx threshold for choosing edge cases
                                       // flags&4 clear: [+0x18] = preset size; drive both clamp sides + edges.
        e.write_u32(this + S_FLAGS, 0);
        for &size in &[
            0.0f32,
            -0.0,
            floor,
            floor * 0.5,
            floor * 2.0,
            1e-6,
            0.5,
            -0.5,
            12.0,
        ] {
            e.write_u32(this + S_RESOLVED_SIZE, size.to_bits());
            let _ = e.call(RESOLVE_CLAMP_SIZE, this as u32, 0, &[]);
            let clamped = e.read_f32(this + S_CLAMPED_SIZE);
            assert_f32(
                clamped,
                resolve_clamp_size(size, h),
                &format!("rcs edge h={h} size={size}"),
            );
        }
        for i in 0..3000u32 {
            let size = fuzz_signed(&mut s, 0.02); // straddles the 2.0/viewH floor
            e.write_u32(this + S_RESOLVED_SIZE, size.to_bits());
            let _ = e.call(RESOLVE_CLAMP_SIZE, this as u32, 0, &[]);
            let clamped = e.read_f32(this + S_CLAMPED_SIZE);
            assert_f32(
                clamped,
                resolve_clamp_size(size, h),
                &format!("rcs[{i}] h={h} size={size}"),
            );
        }
        // flags&4 set (sign clear): default size = 0x5c2540(font) = pixelSize/viewH; both clamp sides.
        e.write_u32(this + S_FLAGS, 0x4);
        for &px in &[1i32, 2, 3, 12, 48] {
            e.write_u32(font + F_PIXEL_SIZE, px as u32);
            let _ = e.call(RESOLVE_CLAMP_SIZE, this as u32, 0, &[]);
            let resolved = e.read_f32(this + S_RESOLVED_SIZE); // = pixelSize/viewH
            let clamped = e.read_f32(this + S_CLAMPED_SIZE);
            assert_f32(
                clamped,
                resolve_clamp_size(resolved, h),
                &format!("rcs flag4 h={h} px={px} resolved={resolved}"),
            );
        }
    }
}

#[test]
fn cgxstring_size_clamp_difftest() {
    // 0x5cd6d0: thiscall ecx=this; 11 stack args. Capacity preset large ⇒ realloc skipped; strlen
    // (0x64a6f0) + strcpy (0x64a5a0) run for real; the per-font list-splice threads onto a synthesized
    // head; flags&4 (+ sign clear) ⇒ [+0x18] = 0x5c2540(font) default (runs for real), else the size
    // arg. The clamp writes [+0x1c] = max([+0x18], 2.0/viewH). We read both back and diff the
    // clamp against the resolved [+0x18].
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(S_SIZE);
    let font = e.alloc(F_SIZE);
    let listhead = e.alloc(0x10);
    let color = e.alloc(0x10); // 3 colour words
    let render = e.alloc(0x4);
    let text = e.alloc(0x10);
    let dest = e.alloc(0x100);
    // text = "Hi\0"
    e.write(text, b"Hi\0");
    e.write_u32(font + F_STRLIST_LINKOFF, 0x8);
    e.write_u32(font + F_STRLIST_HEAD, listhead as u32);
    e.write_u32(this + S_TEXT_CAP, 0x100); // big ⇒ skip realloc
    e.write_u32(this + S_TEXT, dest as u32);
    e.write_u32(color, 0x1100_2233);
    e.write_u32(color + 4, 0x4455_6677);
    e.write_u32(color + 8, 0x8899_aabb);
    e.write_u32(render, 0xdead_beef);

    let mut s: u32 = 0x5cd6_d001;
    for &h in RES_H {
        e.write_u32(g::SCREEN_H, h as u32);
        // flags=0: resolved = size arg; drive both clamp sides.
        for &size in &[
            0.0f32,
            2.0 / h as f32,
            (2.0 / h as f32) * 0.5,
            (2.0 / h as f32) * 4.0,
            0.5,
            12.0,
        ] {
            e.write_u32(this + 0x8, 0); // [esi+8] = 0 ⇒ skip unlink in splice
            let _ = e.call(
                CGXSTRING_SIZE_CLAMP,
                this as u32,
                0,
                &[
                    size.to_bits(),
                    color as u32,
                    0,
                    0,
                    font as u32,
                    text as u32,
                    0,
                    0,
                    0,
                    0, // flags = 0
                    render as u32,
                ],
            );
            let resolved = e.read_f32(this + S_RESOLVED_SIZE);
            let clamped = e.read_f32(this + S_CLAMPED_SIZE);
            assert_f32(resolved, size, &format!("csc resolved h={h} size={size}"));
            assert_f32(
                clamped,
                cgxstring_size_clamp(resolved, h),
                &format!("csc flag0 h={h} size={size}"),
            );
        }
        // flags=4 (sign clear): resolved = default 0x5c2540(font)=pixelSize/viewH; both clamp sides.
        for &px in &[1i32, 2, 3, 12, 48] {
            e.write_u32(font + F_PIXEL_SIZE, px as u32);
            e.write_u32(this + 0x8, 0);
            let _ = e.call(
                CGXSTRING_SIZE_CLAMP,
                this as u32,
                0,
                &[
                    1.0f32.to_bits(),
                    color as u32,
                    0,
                    0,
                    font as u32,
                    text as u32,
                    0,
                    0,
                    0,
                    0x4, // flags = 4 (default-size pick)
                    render as u32,
                ],
            );
            let resolved = e.read_f32(this + S_RESOLVED_SIZE);
            let clamped = e.read_f32(this + S_CLAMPED_SIZE);
            assert_f32(
                clamped,
                cgxstring_size_clamp(resolved, h),
                &format!("csc flag4 h={h} px={px} resolved={resolved}"),
            );
        }
        // a few fuzzed flag-0 sizes for good measure
        for i in 0..500u32 {
            let size = fuzz_signed(&mut s, 0.02);
            e.write_u32(this + 0x8, 0);
            let _ = e.call(
                CGXSTRING_SIZE_CLAMP,
                this as u32,
                0,
                &[
                    size.to_bits(),
                    color as u32,
                    0,
                    0,
                    font as u32,
                    text as u32,
                    0,
                    0,
                    0,
                    0,
                    render as u32,
                ],
            );
            let resolved = e.read_f32(this + S_RESOLVED_SIZE);
            let clamped = e.read_f32(this + S_CLAMPED_SIZE);
            assert_f32(
                clamped,
                cgxstring_size_clamp(resolved, h),
                &format!("csc[{i}] h={h} size={size}"),
            );
        }
        // (the flag-4 default-size path runs GxuFontGetOneToOneHeight 0x5c2540 for real)
    }
}

#[test]
fn anchor_justify_snap_difftest() {
    // 0x5cdf70: thiscall ecx=this. Reads +0x1c/+0x20..+0x28/+0x3c/+0x40/+0x50/+0x54/+0x58/+0x5c/+0x9c
    // and writes +0x70/+0x74/+0x78. Only __ftol (0x40a2b0) + floor (0x73ff3f) sub-calls, both run for
    // real. Cover: n==0 skip, sign-flag skip, justify_h 0/1/2, justify_v 0/1/2, + fuzz.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(S_SIZE);
    let mut s: u32 = 0x5cdf_7001;

    let run = |e: &mut Emu, this: u64, st: &AnchorState| -> (f32, f32, f32) {
        e.write_u32(this + S_ANCHOR, st.anchor_x.to_bits());
        e.write_u32(this + S_ANCHOR + 4, st.anchor_y.to_bits());
        e.write_u32(this + S_ANCHOR + 8, st.anchor_z.to_bits());
        e.write_u32(this + S_BUILT, st.n as u32);
        e.write_u32(this + S_FLAGS, u32::from(st.flags));
        e.write_u32(this + S_JUSTIFY_V, st.justify_v);
        e.write_u32(this + S_JUSTIFY_H, st.justify_h);
        e.write_u32(this + S_POS_X, st.pos_x.to_bits());
        e.write_u32(this + S_POS_Y, st.pos_y.to_bits());
        e.write_u32(this + S_CLAMPED_SIZE, st.clamped_size.to_bits());
        e.write_u32(this + S_WRAP, st.wrap_width.to_bits());
        // pre-poison the output so a skipped snap is observable
        e.write_u32(this + S_ANCHOR_OUT, 0xdead_0000);
        e.write_u32(this + S_ANCHOR_OUT + 4, 0xdead_0001);
        e.write_u32(this + S_ANCHOR_OUT + 8, 0xdead_0002);
        let _ = e.call(ANCHOR_JUSTIFY_SNAP, this as u32, 0, &[]);
        (
            e.read_f32(this + S_ANCHOR_OUT),
            e.read_f32(this + S_ANCHOR_OUT + 4),
            e.read_f32(this + S_ANCHOR_OUT + 8),
        )
    };

    for (&view_w, &view_h) in RES_W.iter().zip(RES_H.iter()) {
        e.write_u32(g::SCREEN_W, view_w as u32);
        e.write_u32(g::SCREEN_H, view_h as u32);
        // explicit branch coverage
        for &n in &[0i32, 1, 2, 5] {
            for &flags in &[0u8, 0x80, 0x20] {
                for &jh in &[0u32, 1, 2, 3] {
                    for &jv in &[0u32, 1, 2, 3] {
                        let st = AnchorState {
                            anchor_x: 0.3,
                            anchor_y: 0.6,
                            anchor_z: 0.1,
                            n,
                            flags,
                            justify_v: jh,
                            justify_h: jv,
                            pos_x: 0.25,
                            pos_y: 0.75,
                            clamped_size: 0.05,
                            wrap_width: 0.4,
                        };
                        let bin = run(&mut e, this, &st);
                        let ours = anchor_justify_snap(&st, view_w, view_h);
                        assert_f32(
                            bin.0,
                            ours.0,
                            &format!("ajs.x n={n} f={flags:#x} jh={jh} jv={jv}"),
                        );
                        assert_f32(
                            bin.1,
                            ours.1,
                            &format!("ajs.y n={n} f={flags:#x} jh={jh} jv={jv}"),
                        );
                        assert_f32(
                            bin.2,
                            ours.2,
                            &format!("ajs.z n={n} f={flags:#x} jh={jh} jv={jv}"),
                        );
                    }
                }
            }
        }
        for i in 0..4000u32 {
            let st = AnchorState {
                anchor_x: fuzz_signed(&mut s, 2.0),
                anchor_y: fuzz_signed(&mut s, 2.0),
                anchor_z: fuzz_signed(&mut s, 2.0),
                n: ((next(&mut s) >> 24) % 8 + 1) as i32, // 1..8 lines
                flags: (next(&mut s) >> 24) as u8 & 0x20, // sign clear ⇒ active
                justify_v: next(&mut s) >> 30,
                justify_h: next(&mut s) >> 30,
                pos_x: fuzz_signed(&mut s, 2.0),
                pos_y: fuzz_signed(&mut s, 2.0),
                clamped_size: fuzz_unit(&mut s, 0.5),
                wrap_width: fuzz_unit(&mut s, 1.0),
            };
            let bin = run(&mut e, this, &st);
            let ours = anchor_justify_snap(&st, view_w, view_h);
            assert_f32(bin.0, ours.0, &format!("ajs[{i}].x w={view_w} h={view_h}"));
            assert_f32(bin.1, ours.1, &format!("ajs[{i}].y w={view_w} h={view_h}"));
            assert_f32(bin.2, ours.2, &format!("ajs[{i}].z w={view_w} h={view_h}"));
        }
    }
}

#[test]
fn hyperlink_region_rect_difftest() {
    // 0x5cd310: thiscall ecx=this; 8 stack args (ret 0x20). Normalizes args 0/2 by viewH, args 1/3 by
    // viewW, then appends a 32-byte record to the +0x7c growable array. Capacity preset large ⇒ the
    // grow (0x5ce2b0) is skipped; the record is copied via rep movsd. We read the first 4 floats of
    // the appended record and diff them.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(S_SIZE);
    let buf = e.alloc(0x20 * 64); // 64 records of 32 bytes
    e.write_u32(this + S_HLINK_ARR, 64); // capacity
    e.write_u32(this + S_HLINK_ARR + 8, buf as u32); // buffer

    let mut s: u32 = 0x5cd3_1001;
    for (&view_w, &view_h) in RES_W.iter().zip(RES_H.iter()) {
        e.write_u32(g::SCREEN_W, view_w as u32);
        e.write_u32(g::SCREEN_H, view_h as u32);
        for i in 0..3000u32 {
            e.write_u32(this + S_HLINK_ARR + 4, 0); // count = 0 (append at slot 0)
            let a = fuzz_signed(&mut s, 4.0);
            let b = fuzz_signed(&mut s, 4.0);
            let c = fuzz_signed(&mut s, 4.0);
            let d = fuzz_signed(&mut s, 4.0);
            let _ = e.call(
                HYPERLINK_REGION_RECT,
                this as u32,
                0,
                &[
                    a.to_bits(),
                    b.to_bits(),
                    c.to_bits(),
                    d.to_bits(),
                    0,
                    0,
                    0,
                    0,
                ],
            );
            let (na, nb, nc, nd) = (
                e.read_f32(buf),
                e.read_f32(buf + 4),
                e.read_f32(buf + 8),
                e.read_f32(buf + 0xc),
            );
            let ours = hyperlink_region_rect(a, b, c, d, view_h, view_w);
            assert_f32(na, ours.0, &format!("hlr[{i}].a w={view_w} h={view_h}"));
            assert_f32(nb, ours.1, &format!("hlr[{i}].b w={view_w} h={view_h}"));
            assert_f32(nc, ours.2, &format!("hlr[{i}].c w={view_w} h={view_h}"));
            assert_f32(nd, ours.3, &format!("hlr[{i}].d w={view_w} h={view_h}"));
            assert_eq!(
                e.read_u32(this + S_HLINK_ARR + 4),
                1,
                "hlr count incremented"
            );
        }
    }
}

#[test]
fn selection_highlight_fade_difftest() {
    // 0x5cd4d0: thiscall ecx=this; 2 stack args (startCol, endCol). The active path writes a per-cell
    // alpha byte (via SetCell 0x5cd460) into a synthesized cell array (+0x94, stride 8: byte0=page,
    // dword+4=vertex index) whose page (+0xa0[0]) holds a vertex buffer (+0x20, 4 B/vertex, alpha @+3,
    // count @+0x1c). Pre-fill the vertex alphas nonzero so GetCell (0x5cd490) never returns 0 (no
    // break). We read the per-cell alpha back and diff the whole sequence + the return code (EAX).
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(S_SIZE);
    let page = e.alloc(0x40);
    let cells = e.alloc(8 * 256); // up to 256 cells, stride 8
    let vbuf = e.alloc(4 * 512); // 512 vertices, 4 bytes each
    e.write_u32(this + S_PAGE_LISTS, page as u32); // page 0
    e.write_u32(page + P_VERT_BUF, vbuf as u32);
    e.write_u32(this + S_SEL_CELLS, cells as u32);
    e.write_u32(this + S_BUILT, 1); // +0x9c != 0
    e.write_u32(g::SCREEN_H, 768);
    // cell[i] = { byte0 = page 0, dword+4 = vertex index i }
    for i in 0u64..256 {
        e.write_u32(cells + i * 8, 0); // page index 0 (low byte)
        e.write_u32(cells + i * 8 + 4, i as u32); // vertex index
    }

    let mut s: u32 = 0x5cd4_d001;
    for i in 0..3000u32 {
        let count = ((next(&mut s) >> 20) % 200 + 1) as i32; // 1..200 cells
        let start = ((next(&mut s) >> 20) % 60) as i32; // 0..59
        let end = ((next(&mut s) >> 20) % 60 + 1) as i32; // 1..60 (endCol >= 1)
        let base = (next(&mut s) >> 24) as u8;
        e.write_u32(page + P_VERT_COUNT, count as u32 + 8); // vertcount >= cell count
        e.write_u32(this + S_SEL_COUNT, count as u32);
        e.write(this + S_ALPHA_BYTE, &[base]); // +0x2f base alpha byte
        e.write_u32(this + S_SEL_START, 0x7fff_ffff); // cached range != (start,end)
        e.write_u32(this + S_SEL_END, 0x7fff_ffff);
        e.write_u32(this + S_FLAGS, 0);
        // pre-fill all vertex alpha bytes nonzero (GetCell != 0 ⇒ no break)
        for v in 0u64..(count as u64 + 8) {
            e.write_u32(vbuf + v * 4, 0xffff_ffff);
        }
        let ret = e.call(
            SELECTION_HIGHLIGHT_FADE,
            this as u32,
            0,
            &[start as u32, end as u32],
        );
        let ours = selection_highlight_fade(start, end, base, count);
        assert_eq!(
            ret, ours.ret,
            "shf[{i}] ret start={start} end={end} base={base} count={count}"
        );
        for (k, &exp) in ours.alpha.iter().enumerate() {
            let got = e.read(vbuf + (k as u64) * 4 + 3, 1)[0];
            assert_eq!(
                got, exp,
                "shf[{i}] cell={k} start={start} end={end} base={base} count={count}"
            );
        }
    }
}

#[test]
fn glyph_quad_emit_difftest() {
    // 0x5c8710: thiscall ecx=this; 10 args. With arg7=1 (block-1 enable), arg9=0, arg10=count
    // (<= laid count +0xc), block 1 emits all `count` quads via the direct pen-translate copy and block
    // 2 (matrix transform) is skipped (arg10 hits 0). +0x1c = 0 ⇒ no gradient. The device-state
    // query 0x58a230 is stubbed to a scratch struct whose +0x1c picks the colour channel-swap. Pen base
    // = floor(ScreenToPixel{W,H}(snap)) + pen (sub-calls run for real). We read each output vertex
    // (stride 0x18: x,y,z,color,u,v) and diff geometry + colour bit-exact.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(S_SIZE);
    let dev = e.alloc(0x20); // device-state struct; [+0x1c] = swap flag
    let count: u64 = 8;
    let vin = e.alloc(0x14 * count); // input verts (stride 0x14)
    let vout = e.alloc(0x18 * (count + 2)); // output verts (stride 0x18)
    let snap = e.alloc(0x8); // {x, y}
    let pen = e.alloc(0x8); // {x, y}
    let color = e.alloc(0x4); // base colour dword (arg5 → *arg5)
    e.write_u32(this + S_VERT_COUNT_C, count as u32);
    e.write_u32(this + S_VERT_BUF_10, vin as u32);
    e.write_u32(this + S_GRAD_1C, 0); // no gradient (local_18 = 0)
    e.write_u32(this + S_GRAD_BUF_20, 0);
    e.stub(DEVICE_QUERY, dev as u32, 0); // cdecl, no args; returns &dev

    let mut s: u32 = 0x5c87_1001;
    for (&view_w, &view_h) in RES_W.iter().zip(RES_H.iter()) {
        e.write_u32(g::SCREEN_W, view_w as u32);
        e.write_u32(g::SCREEN_H, view_h as u32);
        for &swap in &[0u32, 1] {
            e.write_u32(dev + 0x1c, swap);
            for i in 0..1200u32 {
                // synthesize input verts + the snap/pen/colour
                let mut ins = Vec::with_capacity(count as usize);
                for k in 0..count {
                    let iv = EmitVertexIn {
                        x: fuzz_signed(&mut s, 64.0),
                        y: fuzz_signed(&mut s, 64.0),
                        z: fuzz_signed(&mut s, 4.0),
                        u: fuzz_unit(&mut s, 1.0),
                        v: fuzz_unit(&mut s, 1.0),
                    };
                    let base = vin + k * 0x14;
                    e.write_u32(base, iv.x.to_bits());
                    e.write_u32(base + 4, iv.y.to_bits());
                    e.write_u32(base + 8, iv.z.to_bits());
                    e.write_u32(base + 0xc, iv.u.to_bits());
                    e.write_u32(base + 0x10, iv.v.to_bits());
                    ins.push(iv);
                }
                let snap_x = fuzz_signed(&mut s, 1.0);
                let snap_y = fuzz_signed(&mut s, 1.0);
                let pen_x = fuzz_signed(&mut s, 32.0);
                let pen_y = fuzz_signed(&mut s, 32.0);
                let col = next(&mut s);
                e.write_u32(snap, snap_x.to_bits());
                e.write_u32(snap + 4, snap_y.to_bits());
                e.write_u32(pen, pen_x.to_bits());
                e.write_u32(pen + 4, pen_y.to_bits());
                e.write_u32(color, col);

                let _ = e.call(
                    GLYPH_QUAD_EMIT,
                    this as u32,
                    0,
                    &[
                        vout as u32,  // arg2 outbuf
                        0,            // arg3 (gradient src; unused, +0x1c=0)
                        snap as u32,  // arg4 snap vector
                        color as u32, // arg5 base colour ptr
                        pen as u32,   // arg6 pen
                        1,            // arg7 = 1 (block-1 enable)
                        0,            // arg8 = 0 (no shadow fade)
                        0,            // arg9 = 0 (start)
                        count as u32, // arg10 = count
                    ],
                );

                let (penx, peny) =
                    glyph_quad_emit_pen(snap_x, snap_y, pen_x, pen_y, view_w, view_h);
                let oc = glyph_quad_emit_color(col, swap != 0);
                for (k, iv) in ins.iter().enumerate() {
                    let ours = glyph_quad_emit_vertex(*iv, penx, peny, oc);
                    let o = vout + (k as u64) * 0x18;
                    let got = EmitVertexOut {
                        x: e.read_f32(o),
                        y: e.read_f32(o + 4),
                        z: e.read_f32(o + 8),
                        color: e.read_u32(o + 0xc),
                        u: e.read_f32(o + 0x10),
                        v: e.read_f32(o + 0x14),
                    };
                    let ctx = format!("gqe[{i}] k={k} w={view_w} h={view_h} swap={swap}");
                    assert_f32(got.x, ours.x, &format!("{ctx} .x"));
                    assert_f32(got.y, ours.y, &format!("{ctx} .y"));
                    assert_f32(got.z, ours.z, &format!("{ctx} .z"));
                    assert_f32(got.u, ours.u, &format!("{ctx} .u"));
                    assert_f32(got.v, ours.v, &format!("{ctx} .v"));
                    assert_eq!(got.color, ours.color, "{ctx} .color");
                }
            }
        }
    }
}

/// Pre-set the ±1/512 texel-nudge globals + their one-time-init guard, so `glyph_quad_build`'s init
/// block (which also registers an atexit via 0x409aef) is skipped — the raw/pre-snapped UV path then
/// reads the constants directly. Idempotent; call once per Emu.
fn arm_texel_globals(e: &mut Emu) {
    e.write(G_TEXEL_GUARD, &[1]); // [0xc2b9f0] |= 1 ⇒ init block taken `jne 0x5ccd27`
    e.write_u32(G_TEXEL_VB, 0x3b00_0000); // +1/512
    e.write_u32(G_TEXEL_UL, 0x3b00_0000);
    e.write_u32(G_TEXEL_VT, 0xbb00_0000); // -1/512
    e.write_u32(G_TEXEL_UR, 0xbb00_0000);
}

#[test]
fn glyph_quad_build_difftest() {
    // 0x5ccbe0: thiscall ecx=this; 6 stack args (text, count, &color, &origin, &pageMask, &state),
    // ret 0x18. One literal glyph (text "A\0", count 1): the markup decode (0x5c2810) resolves class 6
    // → the glyph emit path. NewCodeDesc (0x5cabd0) is stubbed → a synthesized CharCodeDesc (so no
    // FreeType raster); GlyphWidth (0x5cb080) + ScreenToPixelHeight (0x5c6fa0) run for real. The page's
    // AUVERT geometry array (page+8 {cap,count,buf}) and colour list (page+0x18 {cap,count,buf}) are
    // pre-sized large ⇒ the gx vertex-buffer lock (0x59af90/0x59afd0/0x503d10) + the array realloc are
    // skipped, and we read the 4 written vertices + colour directly. String flags keep bit3 (outline,
    // would clear the page lists) and bit5 (selection) clear; bit7 (raw/pre-snapped) is fuzzed.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(S_SIZE);
    let font = e.alloc(F_SIZE);
    let desc = e.alloc(0x70);
    let page = e.alloc(0x40);
    let auv = e.alloc(AUV_STRIDE * 64);
    let colbuf = e.alloc(0x4 * 64);
    let colorw = e.alloc(0x4);
    let origin = e.alloc(0xc);
    let pagemask = e.alloc(0x4);
    let state = e.alloc(0x20);
    let text = e.alloc(0x8);

    e.write_u32(this + S_FONT, font as u32);
    e.write_u32(this + S_PAGE_LISTS, page as u32); // page index 0
    e.write_u32(page + P_AUV_CAP, 64);
    e.write_u32(page + P_AUV_BUF, auv as u32);
    e.write_u32(page + P_COL_CAP, 64);
    e.write_u32(page + P_COL_BUF, colbuf as u32);
    e.write_u32(desc + D_PAGE_IDX, 0);
    e.write(text, b"A\0");
    e.stub(NEW_CODE_DESC, desc as u32, 1); // thiscall ecx=font, 1 stack arg (code) → ret 4
    arm_texel_globals(&mut e);

    let mut s: u32 = 0x5ccb_e001;
    for &h in RES_H {
        e.write_u32(g::SCREEN_H, h as u32);
        for &sflags in &[0u8, 0x80] {
            e.write_u32(this + S_FLAGS, u32::from(sflags));
            for &fflags in &[0u32, 0x1, 0x8, 0x9] {
                e.write_u32(font + F_FLAGS_180, fflags);
                for i in 0..300u32 {
                    let clamped = fuzz_unit(&mut s, 0.5) + 1e-3;
                    let px = ((next(&mut s) >> 24) % 60 + 4) as i32;
                    let pen_x = fuzz_signed(&mut s, 8.0);
                    let oy = fuzz_signed(&mut s, 8.0);
                    let oz = fuzz_signed(&mut s, 2.0);
                    let adv = ((next(&mut s) >> 16) % 400) as i32;
                    let width = fuzz_signed(&mut s, 64.0);
                    let top = ((next(&mut s) >> 16) as i32 % 128) - 64;
                    let vb = fuzz_unit(&mut s, 1.0);
                    let ul = fuzz_unit(&mut s, 1.0);
                    let vt = fuzz_unit(&mut s, 1.0);
                    let ur = fuzz_unit(&mut s, 1.0);
                    let color = next(&mut s);

                    e.write_u32(this + S_CLAMPED_SIZE, clamped.to_bits());
                    e.write_u32(font + F_PIXEL_SIZE, px as u32);
                    e.write_u32(desc + D_ADVANCE, adv as u32);
                    e.write_u32(desc + D_WIDTH, width.to_bits());
                    e.write_u32(desc + D_TOP, top as u32);
                    e.write_u32(desc + D_V_BOTTOM, vb.to_bits());
                    e.write_u32(desc + D_U_LEFT, ul.to_bits());
                    e.write_u32(desc + D_V_TOP, vt.to_bits());
                    e.write_u32(desc + D_U_RIGHT, ur.to_bits());
                    e.write_u32(colorw, color);
                    e.write_u32(origin, pen_x.to_bits());
                    e.write_u32(origin + 4, oy.to_bits());
                    e.write_u32(origin + 8, oz.to_bits());
                    e.write_u32(page + P_AUV_COUNT, 0);
                    e.write_u32(page + P_COL_COUNT, 0);
                    e.write_u32(pagemask, 0);
                    e.write_u32(state, 0);

                    let _ = e.call(
                        GLYPH_QUAD_BUILD,
                        this as u32,
                        0,
                        &[
                            text as u32,
                            1,
                            colorw as u32,
                            origin as u32,
                            pagemask as u32,
                            state as u32,
                        ],
                    );

                    let q = glyph_quad_build(&GlyphQuadInputs {
                        pen_x,
                        origin_y: oy,
                        origin_z: oz,
                        clamped_size: clamped,
                        pixel_size: px,
                        string_flags: sflags,
                        font_flags: fflags,
                        desc_advance: adv,
                        desc_width: width,
                        desc_top: top,
                        uv_v_bottom: vb,
                        uv_u_left: ul,
                        uv_v_top: vt,
                        uv_u_right: ur,
                        color,
                        screen_h: h,
                    });
                    assert_eq!(e.read_u32(page + P_AUV_COUNT), 4, "gqb[{i}] auv count");
                    for k in 0..4u64 {
                        let b = auv + k * AUV_STRIDE;
                        let v = &q.verts[k as usize];
                        let ctx = format!("gqb[{i}] h={h} sf={sflags:#x} ff={fflags:#x} v{k}");
                        assert_f32(e.read_f32(b), v.x, &format!("{ctx}.x"));
                        assert_f32(e.read_f32(b + 4), v.y, &format!("{ctx}.y"));
                        assert_f32(e.read_f32(b + 8), v.z, &format!("{ctx}.z"));
                        assert_f32(e.read_f32(b + 0xc), v.u, &format!("{ctx}.u"));
                        assert_f32(e.read_f32(b + 0x10), v.v, &format!("{ctx}.v"));
                        assert_eq!(e.read_u32(colbuf + k * 4), q.color, "{ctx}.color");
                    }
                }
            }
        }
    }
}

#[test]
fn string_layout_lines_difftest() {
    // 0x5cdc20: fastcall ecx=this; no stack args. Drives the multi-line layout: per line it runs the real
    // line measure (0x5c7260 → measure_run_*) and the real per-glyph builder (0x5ccbe0). Text is N
    // newline-separated single glyphs ("A\nA\n…") so each rendered line is one content glyph (prev=0 ⇒ no
    // escapement; only NewCodeDesc 0x5cabd0 is stubbed → a CharCodeDesc, GlyphWidth/ScreenToPixel run for
    // real). The full glyph machinery (page arrays pre-sized as in glyph_quad_build) lets 0x5ccbe0 emit
    // for real; ComputeAnchor (0x5cdf70) is stubbed. We read the line count (+0x9c) and each emitted
    // line's 4 quads, diffing the latter via glyph_quad_build(origin = the line origin our transcription
    // computed) — an end-to-end diff of string_layout_lines ∘ glyph_quad_build. box_bottom (+0x40) is
    // fuzzed to exercise the pen-Y termination; flags bit7 toggles the rotated/raw + fixed-width path.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(S_SIZE);
    let font = e.alloc(F_SIZE);
    let desc = e.alloc(0x70);
    let page = e.alloc(0x40);
    let auv = e.alloc(AUV_STRIDE * 64);
    let colbuf = e.alloc(0x4 * 64);
    let kbuckets = e.alloc(0xc);
    let knode = e.alloc(0x40);
    let text = e.alloc(0x20);

    e.write_u32(this + S_FONT, font as u32);
    e.write_u32(this + S_TEXT, text as u32);
    e.write_u32(this + S_PAGE_LISTS, page as u32);
    e.write_u32(page + P_AUV_CAP, 64);
    e.write_u32(page + P_AUV_BUF, auv as u32);
    e.write_u32(page + P_COL_CAP, 64);
    e.write_u32(page + P_COL_BUF, colbuf as u32);
    e.write_u32(desc + D_PAGE_IDX, 0);
    // Cached kern node for the per-line `\n` token's escapement (0x5ccbe0 calls 0x5ca2d0 with
    // prev='A'(0x41), cur='\n'(0x0a) since each line is "A\n"; its result is DISCARDED — the `\n` is
    // class 2, not rendered — but it must run without touching the FreeType face). A matching node with
    // flags bit1 set takes the cached-return path (`5ca48a..5ca4a0`): key=prev, pairkey=(prev<<16)^cur.
    e.write_u32(font + 0x60, 0); // kern hash mask (bucket idx = mask & cur = 0)
    e.write_u32(font + 0x58, kbuckets as u32); // kern hash buckets
    e.write_u32(kbuckets + 0x8, knode as u32); // chain head
    e.write_u32(knode, 0x41); // key = prev 'A'
    e.write_u32(knode + 0x14, (0x41u32 << 16) ^ 0x0a); // pairkey
    e.write_u32(knode + 0x18, 0x2); // flags bit1 ⇒ cached return (no face query, no alloc)
    e.write_u32(knode + 0x1c, 0); // cached step (discarded)
                                  // text = 6 newline-terminated single-glyph lines (up to 6 content lines). The TRAILING newline
                                  // matters: every line ends on a class-2 break, so the proportional measure never decodes the
                                  // phantom `0xffffffff` glyph at the NUL (which, with prev!=0, would call escapement 0x5ca2d0 and
                                  // read the unset kern hash). End-of-string handling is the measure's domain (diffed in `measure`).
    e.write(text, b"A\nA\nA\nA\nA\nA\n\0");
    e.stub(NEW_CODE_DESC, desc as u32, 1);
    e.stub(ANCHOR_SNAP, 0, 0); // fastcall(this), no stack args
    arm_texel_globals(&mut e);

    let textbytes: &[u8] = b"A\nA\nA\nA\nA\nA\n\0";

    let mut s: u32 = 0x5cdc_2001;
    for (&view_w, &view_h) in RES_W.iter().zip(RES_H.iter()) {
        e.write_u32(g::SCREEN_W, view_w as u32);
        e.write_u32(g::SCREEN_H, view_h as u32);
        for &sflags in &[0u32, 0x80] {
            for &fflags in &[0u32, 0x1, 0x8] {
                e.write_u32(font + F_FLAGS_180, fflags);
                for &jv in &[0u32, 1, 2] {
                    for i in 0..200u32 {
                        let clamped = fuzz_unit(&mut s, 0.2) + 0.01;
                        let wrap = fuzz_unit(&mut s, 0.2);
                        let box_bottom = fuzz_unit(&mut s, 1.6);
                        let px = ((next(&mut s) >> 24) % 60 + 4) as i32;
                        let adv = ((next(&mut s) >> 18) % 200) as i32;
                        let width = fuzz_signed(&mut s, 48.0);
                        let top = ((next(&mut s) >> 16) as i32 % 128) - 64;
                        let vb = fuzz_unit(&mut s, 1.0);
                        let ul = fuzz_unit(&mut s, 1.0);
                        let vt = fuzz_unit(&mut s, 1.0);
                        let ur = fuzz_unit(&mut s, 1.0);
                        let color = next(&mut s);
                        let anchor_z = fuzz_signed(&mut s, 2.0);

                        // CGxString layout fields
                        e.write_u32(this + S_CLAMPED_SIZE, clamped.to_bits());
                        e.write_u32(this + S_WRAP, wrap.to_bits());
                        e.write_u32(this + S_POS_X, 10.0f32.to_bits()); // measure box width (no wrap)
                        e.write_u32(this + S_POS_Y, box_bottom.to_bits()); // pen-Y box bottom
                        e.write_u32(this + S_JUSTIFY_H, jv);
                        e.write_u32(this + S_FLAGS, sflags); // bit3/bit5 clear; bit7 = sflags
                        e.write_u32(this + S_ANCHOR_Z, anchor_z.to_bits());
                        e.write_u32(this + S_COLOR_2C, color);
                        e.write_u32(this + S_BUILT, 0); // not yet built
                        e.write_u32(this + S_SEL_START, 0xffff_ffff);
                        e.write_u32(this + S_SEL_END, 0xffff_ffff);
                        // glyph metrics / font
                        e.write_u32(font + F_PIXEL_SIZE, px as u32);
                        e.write_u32(desc + D_ADVANCE, adv as u32);
                        e.write_u32(desc + D_WIDTH, width.to_bits());
                        e.write_u32(desc + D_TOP, top as u32);
                        e.write_u32(desc + D_V_BOTTOM, vb.to_bits());
                        e.write_u32(desc + D_U_LEFT, ul.to_bits());
                        e.write_u32(desc + D_V_TOP, vt.to_bits());
                        e.write_u32(desc + D_U_RIGHT, ur.to_bits());
                        // reset page output counts
                        e.write_u32(page + P_AUV_COUNT, 0);
                        e.write_u32(page + P_COL_COUNT, 0);

                        let _ = e.call(STRING_LAYOUT_LINES, this as u32, 0, &[]);

                        // our transcription: measure runs through a FontCtx mirroring the real glyph cache.
                        let glyph = |_c: u32| Some((width, adv));
                        let escapement = |_a: u32, _b: u32| 0.0f64;
                        let ctx = FontCtx {
                            font_flags: fflags,
                            pixel_size: px,
                            screen_w: view_w,
                            screen_h: view_h,
                            glyph: &glyph,
                            escapement: &escapement,
                        };
                        let res = string_layout_lines(
                            &ctx,
                            textbytes,
                            &LayoutState {
                                clamped_size: clamped,
                                wrap_width: wrap,
                                box_width: 10.0,
                                box_bottom,
                                justify_h: jv,
                                flags: sflags,
                                anchor_z,
                                extra_spacing: 0.0,
                                screen_w: view_w,
                                screen_h: view_h,
                            },
                        );

                        let bin_count = e.read_u32(this + S_BUILT) as i32;
                        let tag = format!(
                            "sll[{i}] w={view_w} h={view_h} sf={sflags:#x} ff={fflags:#x} jv={jv}"
                        );
                        assert_eq!(bin_count, res.count, "{tag} count");
                        assert_eq!(res.lines.len() as i32, res.count, "{tag} lines==count");
                        assert_eq!(
                            e.read_u32(page + P_AUV_COUNT),
                            res.count as u32 * 4,
                            "{tag} auv count"
                        );

                        // each rendered line's quad = glyph_quad_build(our line origin) — composed diff.
                        for (li, lo) in res.lines.iter().enumerate() {
                            let q = glyph_quad_build(&GlyphQuadInputs {
                                pen_x: lo.vjust_x,
                                origin_y: lo.baseline_y,
                                origin_z: lo.origin_z,
                                clamped_size: clamped,
                                pixel_size: px,
                                string_flags: sflags as u8,
                                font_flags: fflags,
                                desc_advance: adv,
                                desc_width: width,
                                desc_top: top,
                                uv_v_bottom: vb,
                                uv_u_left: ul,
                                uv_v_top: vt,
                                uv_u_right: ur,
                                color,
                                screen_h: view_h,
                            });
                            for k in 0..4u64 {
                                let b = auv + (li as u64 * 4 + k) * AUV_STRIDE;
                                let v = &q.verts[k as usize];
                                let c = format!("{tag} line{li} v{k}");
                                assert_f32(e.read_f32(b), v.x, &format!("{c}.x"));
                                assert_f32(e.read_f32(b + 4), v.y, &format!("{c}.y"));
                                assert_f32(e.read_f32(b + 8), v.z, &format!("{c}.z"));
                                assert_f32(e.read_f32(b + 0xc), v.u, &format!("{c}.u"));
                                assert_f32(e.read_f32(b + 0x10), v.v, &format!("{c}.v"));
                            }
                        }
                    }
                }
            }
        }
    }
}
