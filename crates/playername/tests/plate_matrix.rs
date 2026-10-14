//! Bit-exact diff for `playername::plate_matrix` — the arithmetic slices of `0x6c6e90`
//! (`pn_plate_render`: depth-fade scalar, text-width product, atlas-cell + billboard corner-quad,
//! screen-Y accumulate) and `0x6c8d00` (`pn_matrix_scale`: the ortho-rect the world-text matrix
//! is built from) — vs `WoW.exe`'s own bytes under the Unicorn harness.
//!
//! The composed billboard draw (gx VB lock/draw, console GxText, cmath) and the gxumath ortho
//! matrix multiply (`0x5c3d90`) are NOT transcribed — they belong to other layers. Each test
//! here runs a **slice of the function itself** over exactly the FP-arith block (feeding the LUTs,
//! viewport rect and device-scale floats; stubbing nothing inside the arithmetic — `__ftol` and
//! `0x73ff3f` run natively in-slice).

#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_playername::plate_matrix::{
    atlas_cell, billboard_quad, depth_fade_scale, ortho_rect, screen_y_accumulate,
    text_width_times_scale, ANCHOR_CORNER_LUT, ICON_UV_LUT,
};

// shared LCG fuzz (high bits only)
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

// ───────────────────────────────────────────────────────────────────────────────────────────
// `0x6c6e90` pn_plate_render — slice (1): the depth-fade scalar `[0x6c6f10, 0x6c6f43)`.
// ───────────────────────────────────────────────────────────────────────────────────────────
//   6c6f10 fld [ebp-0x24]; 6c6f13 fsub [eax+8]; 6c6f16 mov [ebp-4],0.2;
//   6c6f1d fcom [0x8112a8]=4.0; fnstsw; test ah,0x41; jne 6c6f41;
//   6c6f2a fdiv 4.0; fmul 1.5; fmul 0.2; fstp [ebp-4]  /  else fstp st(0). Halt at 0x6c6f43.
#[test]
fn pn_plate_render() {
    // This is the depth-fade scalar; the billboard-quad, text-width and screen-Y slices of
    // 0x6c6e90 are covered by the sibling tests below.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let objp = emu.alloc(0x20); // eax -> object; reads [eax+8] = bound_z

    let run = |emu: &mut Emu, cam_depth: f32, bound_z: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::EAX, objp);
        emu.write_u32(ebp - 0x24, cam_depth.to_bits()); // local_28 (camDepth)
        emu.write_u32(objp + 0x8, bound_z.to_bits()); // obj.bound[+8]
        emu.write_u32(ebp - 0x4, 0); // clear the scale slot
        emu.run(0x6c6f10, 0x6c6f43);
        emu.read_f32(ebp - 0x4)
    };

    let mut total = 0u32;
    // boundary probes: the exact == 4.0 edge (keeps 0.2), just-above, signs, 0/-0.
    let probes: [(f32, f32); 12] = [
        (4.0, 0.0),       // camRel == 4.0 -> default 0.2
        (8.0, 4.0),       // camRel == 4.0 (via subtract) -> default 0.2
        (4.000_001, 0.0), // just above -> divide branch
        (5.0, 0.0),
        (100.0, 0.0),
        (0.0, 0.0),  // camRel 0 -> 0.2
        (0.0, -0.0), // -0 handling
        (-0.0, 0.0),
        (3.999_999, 0.0), // just below -> 0.2
        (1000.0, 4.0),
        (-100.0, 0.0), // negative -> 0.2
        (f32::from_bits(0x40800000), 0.0),
    ];
    for &(cd, bz) in &probes {
        assert_eq!(
            run(&mut emu, cd, bz).to_bits(),
            depth_fade_scale(cd, bz).to_bits(),
            "depth_fade_scale cam={cd} bound={bz}"
        );
        total += 1;
    }
    let mut s = 0x6c6f_1000u32;
    for _ in 0..6000 {
        let cd = rngf(&mut s, -200.0, 200.0) as f32;
        let bz = rngf(&mut s, -200.0, 200.0) as f32;
        assert_eq!(
            run(&mut emu, cd, bz).to_bits(),
            depth_fade_scale(cd, bz).to_bits(),
            "depth_fade_scale fuzz cam={cd} bound={bz}"
        );
        total += 1;
    }
    eprintln!("pn_plate_render_depth_fade_scale: {total} bit-exact (fld;fsub;fcom 4.0;fdiv;fmul 1.5;fmul 0.2)");
}

// slice (1) tail: desc[+0x30] = textWidth * scale  `[0x6c6fe4, 0x6c6fed)`.
//   6c6fe4 fild qword [ebp-0xc]; 6c6fe7 fmul [ebp-4]; 6c6fea fstp [esi+0x30].
#[test]
fn pn_plate_render_text_width_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let esi = emu.alloc(0x40); // this -> desc; writes [esi+0x30]

    let run = |emu: &mut Emu, w: u32, scale: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(ebp - 0xc, w); // textWidth (low dword of the qword fild)
        emu.write_u32(ebp - 0x8, 0); // high dword = 0 (unsigned width)
        emu.write_u32(ebp - 0x4, scale.to_bits()); // the depth-fade scalar
        emu.write_u32(esi + 0x30, 0);
        emu.run(0x6c6fe4, 0x6c6fed);
        emu.read_f32(esi + 0x30)
    };

    let mut total = 0u32;
    let scales = [0.2f32, 0.3, 0.123_45, 1.5, 0.000_01, 1.0];
    let widths = [0u32, 1, 7, 16, 100, 255, 1024, 65535, 0x10_0000];
    for &sc in &scales {
        for &w in &widths {
            assert_eq!(
                run(&mut emu, w, sc).to_bits(),
                text_width_times_scale(w, sc).to_bits(),
                "text_width_times_scale w={w} scale={sc}"
            );
            total += 1;
        }
    }
    let mut s = 0x77_7711u32;
    for _ in 0..3000 {
        let w = next(&mut s) >> 8; // 0..2^24
        let sc = rngf(&mut s, 0.0, 2.0) as f32;
        assert_eq!(
            run(&mut emu, w, sc).to_bits(),
            text_width_times_scale(w, sc).to_bits(),
            "text_width_times_scale fuzz w={w} scale={sc}"
        );
        total += 1;
    }
    eprintln!("pn_plate_render_text_width_scale: {total} bit-exact (fild qword; fmul; fstp)");
}

// slice (2): atlas-cell + billboard corner-quad `[0x6c71c1, 0x6c724a)`.
//   col=(id&3)*0.25, row=(id>>2)*0.25 (live st0/st1); 4-corner loop copies anchor LUT into pos,
//   writes u=col+iconUV[i].u, v=row+iconUV[i].v (stride 0x14). Diffs the full vertex buffer.
#[test]
fn pn_plate_render_billboard_quad() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let vbuf = emu.alloc(0x80); // vertex buffer (4 verts × 0x14 stride)

    // Initialise the two `.data` LUTs the slice reads (zero in the mapped image).
    // 0xce875c anchor corner LUT (4×vec3), 0xce873c icon UV LUT (4×vec2).
    for (i, c) in ANCHOR_CORNER_LUT.iter().enumerate() {
        for (j, v) in c.iter().enumerate() {
            emu.write_u32(0xce875c + (i * 0xc + j * 4) as u64, v.to_bits());
        }
    }
    for (i, uv) in ICON_UV_LUT.iter().enumerate() {
        for (j, v) in uv.iter().enumerate() {
            emu.write_u32(0xce873c + (i * 8 + j * 4) as u64, v.to_bits());
        }
    }

    let run = |emu: &mut Emu, tex_id: u32| -> [[f32; 5]; 4] {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESI, tex_id as u64);
        emu.set_reg(RegisterX86::EAX, vbuf);
        // edi=0 (the binary's `xor edi,edi` at 0x6c70d2 precedes this slice): it is the high dword
        // written before each `fild qword [ebp-0xc]`, so it MUST be zero for the unsigned widen.
        emu.set_reg(RegisterX86::EDI, 0);
        for k in 0..0x80 / 4 {
            emu.write_u32(vbuf + (k * 4) as u64, 0);
        }
        emu.run(0x6c71c1, 0x6c724a);
        let mut out = [[0.0f32; 5]; 4];
        for (i, vert) in out.iter_mut().enumerate() {
            let base = vbuf + (i * 0x14) as u64;
            for (j, slot) in vert.iter_mut().enumerate() {
                *slot = emu.read_f32(base + (j * 4) as u64);
            }
        }
        out
    };

    let mut total = 0u32;
    // every atlas cell index 0..16 plus a few above (id&3, id>>2 cover the full 4×4 grid).
    let ids: Vec<u32> = (0u32..20).chain([255, 256, 1000].iter().copied()).collect();
    for &id in &ids {
        let got = run(&mut emu, id);
        let (col, row) = atlas_cell(id);
        let want = billboard_quad(col, row);
        assert_eq!(
            got.iter()
                .flatten()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>(),
            want.iter()
                .flatten()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>(),
            "billboard_quad id={id}"
        );
        total += 1;
    }
    let mut s = 0xa70_2200u32;
    for _ in 0..2000 {
        let id = next(&mut s) >> 16;
        let got = run(&mut emu, id);
        let (col, row) = atlas_cell(id);
        let want = billboard_quad(col, row);
        assert_eq!(
            got.iter()
                .flatten()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>(),
            want.iter()
                .flatten()
                .map(|f| f.to_bits())
                .collect::<Vec<_>>(),
            "billboard_quad fuzz id={id}"
        );
        total += 1;
    }
    eprintln!("pn_plate_render_billboard_quad: {total} bit-exact (atlas col/row *0.25; 4× UV fadd; pos copy)");
}

// slice (3): screen-Y accumulate. Two `fld [ebp-0x24]; fadd dword; fstp [ebp-0x24]` shapes:
//   0x6c706e  screenY += desc[+0x30]  (+esi+0x30)
//   0x6c70d8  screenY += scale        (+ebp-0x4)
// Both are the same f32 fld;fadd;fstp; diff the first (the named `local_28 += desc[+0x30]`).
#[test]
fn pn_plate_render_screen_y_accumulate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let esi = emu.alloc(0x40);

    // 0x6c706e fld [ebp-0x24]; 0x6c7071 fadd [esi+0x30]; 0x6c7074 mov ecx,[ebx]; 0x6c7076 lea edx;
    // 0x6c7079 fstp [ebp-0x24].  The two interleaved integer ops touch ebx — point it at scratch.
    // Halt at 0x6c707c (after the fstp, before the delegated 0x5c2770 call).
    let ebx = emu.alloc(0x10);
    let run = |emu: &mut Emu, y: f32, add: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.write_u32(ebp - 0x24, y.to_bits());
        emu.write_u32(esi + 0x30, add.to_bits());
        emu.run(0x6c706e, 0x6c707c); // fld [ebp-0x24]; fadd [esi+0x30]; (movs); fstp [ebp-0x24]
        emu.read_f32(ebp - 0x24)
    };

    let mut total = 0u32;
    let probes: [(f32, f32); 8] = [
        (0.0, 0.0),
        (-0.0, 0.0),
        (0.0, -0.0),
        (1.0, -1.0), // cancels to 0
        (100.0, 0.2),
        (1e9, 0.5),
        (-1e9, 1e9),
        (123.456, 7.89),
    ];
    for &(y, a) in &probes {
        assert_eq!(
            run(&mut emu, y, a).to_bits(),
            screen_y_accumulate(y, a).to_bits(),
            "screen_y_accumulate y={y} add={a}"
        );
        total += 1;
    }
    let mut s = 0x5_c3700u32;
    for _ in 0..3000 {
        let y = rngf(&mut s, -2000.0, 2000.0) as f32;
        let a = rngf(&mut s, -50.0, 50.0) as f32;
        assert_eq!(
            run(&mut emu, y, a).to_bits(),
            screen_y_accumulate(y, a).to_bits(),
            "screen_y_accumulate fuzz y={y} add={a}"
        );
        total += 1;
    }
    eprintln!("pn_plate_render_screen_y_accumulate: {total} bit-exact (fld;fadd;fstp)");
}

// ───────────────────────────────────────────────────────────────────────────────────────────
// `0x6c8d00` pn_matrix_scale — the ortho-rect `[0x6c8d74, 0x6c8e96)`.
// ───────────────────────────────────────────────────────────────────────────────────────────
// Start after the gx viewport/scale fetches (their results pre-written into the stack), run the
// arithmetic, and stub `0x5c3d90` (gxumath projection_ortho) to capture its 4 incoming
// float args = the (left,right,bottom,top) rect.  __ftol (0x40a2b0) and 0x73ff3f run natively.
//
// Stack slots the slice reads (filled by the fed inputs):
//   viewport: [ebp-0x24]=left [ebp-0x20]=top [ebp-0x1c]=right [ebp-0x18]=bottom
//   scale:    [ebp-0x30]=s_top [ebp-0x2c]=s_bottom [ebp-0x38]=s_right [ebp-0x28]=s_left
//   bias:     [ebp-0x8]=x_bias [ebp-0x14]=y_bias
#[test]
fn pn_matrix_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // Stub gxumath projection_ortho — capture its float args, don't build the matrix.
    emu.stub(0x5c3d90, 0, 0);

    let scratch = emu.alloc(0x200);
    let ebp = scratch + 0x100;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xe00; // headroom for the slice's `sub esp,8` / `push` arg-marshalling

    #[allow(clippy::too_many_arguments)]
    let run = |emu: &mut Emu,
               vp_l: f32,
               vp_t: f32,
               vp_r: f32,
               vp_b: f32,
               s_left: f32,
               s_right: f32,
               s_bottom: f32,
               s_top: f32,
               half_pixel: bool|
     -> (f32, f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        // viewport rect
        emu.write_u32(ebp - 0x24, vp_l.to_bits());
        emu.write_u32(ebp - 0x20, vp_t.to_bits());
        emu.write_u32(ebp - 0x1c, vp_r.to_bits());
        emu.write_u32(ebp - 0x18, vp_b.to_bits());
        // device-scale floats (local_30=s_top, local_2c=s_bottom, local_38=s_right, local_28=s_left)
        emu.write_u32(ebp - 0x30, s_top.to_bits());
        emu.write_u32(ebp - 0x2c, s_bottom.to_bits());
        emu.write_u32(ebp - 0x38, s_right.to_bits());
        emu.write_u32(ebp - 0x28, s_left.to_bits());
        // half-pixel biases
        let (xb, yb) = if half_pixel {
            (-0.5f32, 0.5f32)
        } else {
            (0.0, 0.0)
        };
        emu.write_u32(ebp - 0x8, xb.to_bits());
        emu.write_u32(ebp - 0x14, yb.to_bits());

        emu.clear_calls();
        // Run the arithmetic THROUGH the (stubbed) projection_ortho call, halting right
        // after it returns. The stub records [esp+4..] = the 4 float args = (left,right,bottom,top).
        emu.run(0x6c8d74, 0x6c8e9b);
        let c = emu
            .calls()
            .iter()
            .find(|c| c.addr == 0x5c3d90)
            .expect("projection_ortho (0x5c3d90) must be called");
        let left = f32::from_bits(c.args[0]);
        let right = f32::from_bits(c.args[1]);
        let bottom = f32::from_bits(c.args[2]);
        let top = f32::from_bits(c.args[3]);
        (left, right, bottom, top)
    };

    // (vp_l, vp_t, vp_r, vp_b, s_left, s_right, s_bottom, s_top, half_pixel)
    type Case = (f32, f32, f32, f32, f32, f32, f32, f32, bool);
    let cases: &[Case] = &[
        (0.0, 0.0, 1024.0, 768.0, 1.0, 1.0, 1.0, 1.0, true),
        (0.0, 0.0, 1024.0, 768.0, 1.0, 1.0, 1.0, 1.0, false),
        (0.0, 0.0, 1920.0, 1080.0, 1.25, 1.25, 1.25, 1.25, true),
        (0.0, 0.0, 800.0, 600.0, 2.0, 2.0, 2.0, 2.0, true),
        (10.0, 20.0, 640.0, 480.0, 0.5, 0.5, 0.5, 0.5, false),
        (0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, true), // 1px viewport
        (0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, true), // zero viewport
        (0.0, 0.0, 1023.9, 767.4, 1.0, 1.0, 1.0, 1.0, true), // __ftol truncation pivots
        (0.0, 0.0, -5.0, -5.0, 1.0, 1.0, 1.0, 1.0, true), // negative dims -> __ftol toward zero
        (0.5, 0.5, 100.5, 100.5, 1.5, 0.75, 1.333, 0.9, true),
        // floor pivots: products land just below/above an integer so floor (73ff3f) bites.
        (0.0, 0.0, 100.0, 100.0, 0.999, 1.001, 1.0, 0.5, false), // 99.9->99, 100.1->100, 50->50
        (0.0, 0.0, 3.0, 3.0, 0.5, 0.5, 0.5, 0.5, true),          // 1.5 -> floor 1
        (0.0, 0.0, 7.0, 7.0, 0.333, 0.333, 0.333, 0.333, false), // 2.331 -> floor 2
        (
            0.0, 0.0, 1000.0, 1000.0, 1.9999, 1.9999, 1.9999, 1.9999, true,
        ),
    ];
    let mut total = 0u32;
    for &(l, t, r, b, sl, sr, sb, st, hp) in cases {
        let got = run(&mut emu, l, t, r, b, sl, sr, sb, st, hp);
        let want = ortho_rect(l, t, r, b, sl, sr, sb, st, hp);
        assert_eq!(
            [
                got.0.to_bits(),
                got.1.to_bits(),
                got.2.to_bits(),
                got.3.to_bits()
            ],
            [
                want.0.to_bits(),
                want.1.to_bits(),
                want.2.to_bits(),
                want.3.to_bits()
            ],
            "ortho_rect vp=({l},{t},{r},{b}) scale=({sl},{sr},{sb},{st}) half={hp}"
        );
        total += 1;
    }
    let mut s = 0x5c3_d900u32;
    for _ in 0..3000 {
        let l = rngf(&mut s, -50.0, 50.0) as f32;
        let t = rngf(&mut s, -50.0, 50.0) as f32;
        let r = rngf(&mut s, 0.0, 4096.0) as f32;
        let b = rngf(&mut s, 0.0, 4096.0) as f32;
        let sl = rngf(&mut s, 0.1, 3.0) as f32;
        let sr = rngf(&mut s, 0.1, 3.0) as f32;
        let sb = rngf(&mut s, 0.1, 3.0) as f32;
        let st = rngf(&mut s, 0.1, 3.0) as f32;
        let hp = (next(&mut s) & 1) == 0;
        let got = run(&mut emu, l, t, r, b, sl, sr, sb, st, hp);
        let want = ortho_rect(l, t, r, b, sl, sr, sb, st, hp);
        assert_eq!(
            [
                got.0.to_bits(),
                got.1.to_bits(),
                got.2.to_bits(),
                got.3.to_bits()
            ],
            [
                want.0.to_bits(),
                want.1.to_bits(),
                want.2.to_bits(),
                want.3.to_bits()
            ],
            "ortho_rect fuzz vp=({l},{t},{r},{b}) scale=({sl},{sr},{sb},{st}) half={hp}"
        );
        total += 1;
    }
    eprintln!("pn_matrix_scale: {total} bit-exact (2×__ftol; fild; 4×0x73ff3f floor; ±0.5 bias)");
}

// 0x6c70db — 0x6c6e90's gated second screen-Y accumulate `[ebp-0x24] += [ebp-0x4]` (runs when
// [ebp-0x18]!=0; the recount's 11th FP op of pn_plate_render, sibling of the screen-Y
// accumulate above). Slice 0x6c70d8..0x6c70e1: fld [ebp-0x4]; fadd [ebp-0x24]; fstp.
#[test]
fn pn_plate_render_accumulate2() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x40;
    let run = |emu: &mut Emu, a: f32, b: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.write_u32(ebp - 0x4, a.to_bits());
        emu.write_u32(ebp - 0x24, b.to_bits());
        emu.run(0x6c70d8, 0x6c70e1);
        emu.read_f32(ebp - 0x24)
    };
    let want = |a: f32, b: f32| -> f32 { (f64::from(a) + f64::from(b)) as f32 };
    let mut total = 0u32;
    for &(a, b) in &[
        (0.0f32, 0.0),
        (0.0, -0.0),
        (-0.0, -0.0),
        (1.0, -1.0),
        (1.5, 2.5),
        (1e9, 1.0),
        (-1e9, 1e9),
    ] {
        assert_eq!(
            run(&mut emu, a, b).to_bits(),
            want(a, b).to_bits(),
            "acc2 a={a} b={b}"
        );
        total += 1;
    }
    let mut s = 0x6c70_db00u32;
    for _ in 0..4000 {
        let a = rngf(&mut s, -1000.0, 1000.0) as f32;
        let b = rngf(&mut s, -1000.0, 1000.0) as f32;
        assert_eq!(
            run(&mut emu, a, b).to_bits(),
            want(a, b).to_bits(),
            "acc2 fuzz a={a} b={b}"
        );
        total += 1;
    }
    eprintln!("pn_plate_render_accumulate2: {total} bit-exact (fld; fadd; fstp [ebp-0x24])");
}
