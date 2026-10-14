//! Bit-exact diffs for the two render functions of `PlayerName.cpp` vs `WoW.exe`:
//! `pn_glyph_quad_build` (`0x6c7f10`'s `SWorldTextIcon` record floats) and `pn_render_driver`
//! (`0x6c8820`'s arithmetic slices — the `xyz·32` quad scale, the `{0,1,3,3,1,2}` index
//! pattern, and the `(alpha<<24)|0xffffff` color-pack).

#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_playername::render::{
    glyph_quad_build, icon_color_pack, quad_corners, quad_vertex_scale, WorldTextIcon,
    QUAD_INDICES, QUAD_POS_LUT,
};

// shared LCG fuzz (high bits only)
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

const ICE88DC: u64 = 0x00ce_88dc; // glyph/icon gx texture-handle table (type 5..=19)

// ===================================================================================
// 0x6c7f10  pn_glyph_quad_build — the SWorldTextIcon record floats
// ===================================================================================
//
// FP slice of the function itself, [0x6c8009, 0x6c8064): at entry eax = record base, ebx = `this`,
// [ebp-0x4]=local_4=0.0, [ebp-0x8]=local_8=(type-5), [ebp-0xc]=local_c=0.0. The slice reads
// this[0x10]/0x20/0x30/0x34/0x38 and 0xce88dc[type-5], and writes the 6-dword record. We read the
// produced dwords back and diff against `glyph_quad_build`.
#[test]
fn pn_glyph_quad_build() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let this = emu.alloc(0x100); // ebx — the WORLDTEXTSTRING `this`
    let rec = emu.alloc(0x40); // eax — the SWorldTextIcon record (stride 0x18)
    let frame = emu.alloc(0x80); // a stack frame for [ebp-0xc .. ]
    let ebp = frame + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00; // room for the slice's `pop esi`

    let run = |emu: &mut Emu,
               typ: u32,
               t10: f32,
               t30: f32,
               t34: f32,
               t38: f32,
               color: u32,
               tex: u32|
     -> WorldTextIcon {
        let idx = typ - 5; // local_8 / edx
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::EAX, rec);
        emu.set_reg(RegisterX86::EBX, this);
        // the `this` fields the slice reads
        emu.write_u32(this + 0x10, t10.to_bits());
        emu.write_u32(this + 0x20, color);
        emu.write_u32(this + 0x30, t30.to_bits());
        emu.write_u32(this + 0x34, t34.to_bits());
        emu.write_u32(this + 0x38, t38.to_bits());
        // the texture-handle table slot for this type
        emu.write_u32(ICE88DC + u64::from(idx) * 4, tex);
        // the three frame scalars: local_4 = local_c = 0.0, local_8 = type-5
        emu.write_u32(ebp - 0x4, 0.0f32.to_bits());
        emu.write_u32(ebp - 0x8, idx);
        emu.write_u32(ebp - 0xc, 0.0f32.to_bits());
        emu.run(0x6c8009, 0x6c8064);
        WorldTextIcon {
            x_bits: emu.read_u32(rec),
            y_bits: emu.read_u32(rec + 0x4),
            z_bits: emu.read_u32(rec + 0x8),
            rec3_bits: emu.read_u32(rec + 0xc),
            color_bits: emu.read_u32(rec + 0x10),
            tex_bits: emu.read_u32(rec + 0x14),
        }
    };

    let mut total = 0u32;
    // Deterministic boundary probes: the full icon-type range 5..=19, signed t10 (drives the sign
    // of the ·0.0 rec[3] zero), signed anchor px, the -0.0 inputs.
    let anchors: [(f32, f32, f32); 7] = [
        (0.0, 0.0, 0.0),
        (1.0, 2.0, 3.0),
        (-1.0, -2.0, -3.0),
        (-0.0, -0.0, -0.0),
        (123.5, -456.25, 789.0),
        (f32::from_bits(0x7f7f_ffff), -1e30, 1e-30), // huge/small px
        (1e9, -1e9, 0.5),
    ];
    let t10s: [f32; 6] = [0.0, -0.0, 1.0, -1.0, 1e30, -1e30];
    for typ in 5u32..=19 {
        for &t10 in &t10s {
            for (i, &(t30, t34, t38)) in anchors.iter().enumerate() {
                let color = 0x1122_3344u32.wrapping_add(typ).wrapping_mul(7);
                let tex = 0xdead_0000u32 | (typ << 8) | (i as u32);
                let got = run(&mut emu, typ, t10, t30, t34, t38, color, tex);
                let want = glyph_quad_build(t10, t30, t34, t38, color, tex);
                assert_eq!(got, want, "glyph type={typ} t10={t10} anchor#{i}");
                total += 1;
            }
        }
    }
    // fuzz
    let mut s = 0x6c7f_1000u32;
    for _ in 0..4000 {
        let typ = 5 + (next(&mut s) % 15);
        let t10 = rngf(&mut s, -1e6, 1e6) as f32;
        let t30 = rngf(&mut s, -5000.0, 5000.0) as f32;
        let t34 = rngf(&mut s, -5000.0, 5000.0) as f32;
        let t38 = rngf(&mut s, -5000.0, 5000.0) as f32;
        let color = next(&mut s);
        let tex = next(&mut s);
        let got = run(&mut emu, typ, t10, t30, t34, t38, color, tex);
        let want = glyph_quad_build(t10, t30, t34, t38, color, tex);
        assert_eq!(got, want, "glyph fuzz type={typ} t10={t10}");
        total += 1;
    }
    eprintln!("pn_glyph_quad_build: {total} records bit-exact (·0.0 idiom ×3, fsubp, -40/16, tex)");
}

// Checks that the transcription reproduces rec[0] = -40.0 and rec[1] = 16.0, independent of the
// inputs since local_4 = local_c = +0.0 (the constant fold). It runs no exe code.
#[test]
fn pn_glyph_quad_build_invariants() {
    // rec[3] with non-negative t10 is +0.0; rec[0] = -40.0, rec[1] = -(0-32)·0.5 = 16.0.
    let r = glyph_quad_build(1.0, 7.0, 9.0, 11.0, 0xaabbccdd, 0x12345678);
    assert_eq!(f32::from_bits(r.x_bits), -40.0, "rec[0] must be -40.0");
    assert_eq!(f32::from_bits(r.y_bits), 16.0, "rec[1] must be 16.0");
    assert_eq!(f32::from_bits(r.rec3_bits), 0.0, "rec[3] must be 0.0");
    assert_eq!(r.z_bits, 11.0f32.to_bits(), "rec[2] = this[0x38] copy");
    assert_eq!(r.color_bits, 0xaabbccdd, "rec[4] = this[0x20] copy");
    assert_eq!(r.tex_bits, 0x12345678, "rec[5] = tex copy");
    // negative t10 makes rec[3] = -0.0 (signed-zero idiom); rec[0]/rec[1] unchanged.
    let n = glyph_quad_build(-1.0, 7.0, 9.0, 11.0, 0, 0);
    assert_eq!(
        n.rec3_bits,
        (-0.0f32).to_bits(),
        "rec[3] = -0.0 for negative t10"
    );
    assert_eq!(f32::from_bits(n.x_bits), -40.0);
    assert_eq!(f32::from_bits(n.y_bits), 16.0);
}

// ===================================================================================
// 0x6c8820  pn_render_driver — arithmetic slices
// ===================================================================================
//
// (1) the text-quad vertex build loop [0x6c8ae7, 0x6c8b3b): per corner, dst.xyz = src_lut.xyz·32.
//     Slice of the function itself: ecx = 0xce87c8 (xyz src base+8), edx = 0xce87a0 (uv src), eax = vertbase,
//     esi = eax+0xc, edi = eax, [ebp-0x14] = 4. We feed arbitrary src floats into the LUT region
//     and read the 4 scaled vertices (stride 0x14) back, diffing the FP against `quad_vertex_scale`.
#[test]
fn pn_render_driver() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let xyz = 0x00ce_87c0u64; // the icon-xform / quad-position LUT (4×vec3, stride 0xc)
    let uv = 0x00ce_87a0u64; // the quad-uv LUT (4×vec2, stride 0x8)
    let verts = emu.alloc(0x100); // 4 vertices, stride 0x14
    let frame = emu.alloc(0x80);
    let ebp = frame + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    // Run the 4-iteration scale loop over given src vec3s, returning the 4 scaled corners.
    let run = |emu: &mut Emu, src: [[f32; 3]; 4]| -> [[f32; 3]; 4] {
        for (i, v) in src.iter().enumerate() {
            let b = xyz + (i as u64) * 0xc;
            emu.write_u32(b, v[0].to_bits());
            emu.write_u32(b + 0x4, v[1].to_bits());
            emu.write_u32(b + 0x8, v[2].to_bits());
        }
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::ECX, xyz + 0x8); // ecx start = 0xce87c8
        emu.set_reg(RegisterX86::EDX, uv); // edx start = 0xce87a0
        emu.set_reg(RegisterX86::EAX, verts); // eax = vert write ptr
        emu.set_reg(RegisterX86::ESI, verts + 0xc); // esi = uv write ptr (eax+0xc)
        emu.set_reg(RegisterX86::EDI, verts); // edi = eax mirror
        emu.write_u32(ebp - 0x14, 4); // loop counter
        emu.run(0x6c8ae7, 0x6c8b3b);
        let mut out = [[0f32; 3]; 4];
        for (i, o) in out.iter_mut().enumerate() {
            let b = verts + (i as u64) * 0x14;
            o[0] = emu.read_f32(b);
            o[1] = emu.read_f32(b + 0x4);
            o[2] = emu.read_f32(b + 0x8);
        }
        out
    };

    let mut total = 0u32;
    // (a) the real LUT — the in-game quad corners.
    let real = run(&mut emu, QUAD_POS_LUT);
    let want_real = quad_corners();
    for c in 0..4 {
        for k in 0..3 {
            assert_eq!(
                real[c][k].to_bits(),
                want_real[c][k].to_bits(),
                "real LUT corner {c} comp {k}"
            );
            total += 1;
        }
    }

    // (b) arbitrary src floats — proves dst = src·32 component-wise (boundary + fuzz).
    let probes: [[f32; 3]; 8] = [
        [0.0, 1.0, -1.0],
        [-0.0, 0.5, 1e30],
        [1e-30, -1e30, 123.5],
        [f32::from_bits(0x7f7f_ffff), 0.0, -0.0], // f32::MAX → inf after ·32
        [0.25, 0.75, -0.125],
        [3.0, -3.0, 7.5],
        [1.0 / 3.0, 2.0 / 3.0, -2.0 / 3.0],
        [1e9, -1e9, 1e-9],
    ];
    let mut s = 0x6c88_2000u32;
    for chunk in 0..8 {
        let mut src = [[0f32; 3]; 4];
        for (j, v) in src.iter_mut().enumerate() {
            v[0] = probes[(chunk + j) & 7][0];
            v[1] = rngf(&mut s, -1e6, 1e6) as f32;
            v[2] = rngf(&mut s, -1e6, 1e6) as f32;
        }
        let got = run(&mut emu, src);
        for c in 0..4 {
            let want = quad_vertex_scale(src[c]);
            for k in 0..3 {
                assert_eq!(
                    got[c][k].to_bits(),
                    want[k].to_bits(),
                    "xyz·32 chunk={chunk} corner={c} comp={k} src={:?}",
                    src[c]
                );
                total += 1;
            }
        }
    }
    eprintln!("pn_render_driver: {total} xyz·32 components bit-exact (fmul [0x806534]=32.0)");
}

// (2)+(3): the integer index pattern and colour pack, compared with the values the driver emits.
// The driver writes the indices as constant `word`s ({0,1,3,3,1,2}) and packs each icon colour as
// (alpha<<24)|0x00ffffff; both fold to a fixed pattern. It runs no exe code.
#[test]
fn pn_render_driver_index_color() {
    // index pattern {0,1,3,3,1,2} — the exact 6 word writes at 0x6c8b78..0x6c8b8d.
    assert_eq!(QUAD_INDICES, [0u16, 1, 3, 3, 1, 2], "quad index list");
    // color-pack over the full alpha-byte range 0..=255.
    for a in 0u16..=255 {
        let a = a as u8;
        let want = (u32::from(a) << 24) | 0x00ff_ffff;
        assert_eq!(icon_color_pack(a), want, "color-pack alpha={a}");
    }
    // boundary alphas explicit.
    assert_eq!(icon_color_pack(0x00), 0x00ff_ffff);
    assert_eq!(icon_color_pack(0xff), 0xffff_ffff);
    assert_eq!(icon_color_pack(0x80), 0x80ff_ffff);
}

// 0x6c7f5c / 0x6c7f75 — 0x6c7f10's two delta `fsub`s the rec-build slice skipped:
// `[ebp-0xc] = [ebp-0x1c] - [ebp-0x24]` and `[ebp-0x4] = [ebp-0x20] - [ebp-0x28]` (the local_c/local_4
// the rec floats consume). Slice 0x6c7f59..0x6c7f7b (the interleaved int movs read the
// SWorldTextIcon ring count/cap at 0xce87f0/0xce87f4 — written to 0 so the cmp is harmless).
#[test]
fn pn_glyph_quad_build_deltas() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x40;
    let run = |emu: &mut Emu, a: f32, b: f32, c: f32, d: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.write_u32(ebp - 0x1c, a.to_bits());
        emu.write_u32(ebp - 0x24, b.to_bits());
        emu.write_u32(ebp - 0x20, c.to_bits());
        emu.write_u32(ebp - 0x28, d.to_bits());
        emu.write_u32(0x00ce_87f4, 0); // ring count (read by the interleaved int movs)
        emu.write_u32(0x00ce_87f0, 0); // ring cap
        emu.run(0x6c7f59, 0x6c7f7b);
        (emu.read_f32(ebp - 0xc), emu.read_f32(ebp - 0x4))
    };
    let want = |a: f32, b: f32, c: f32, d: f32| -> (f32, f32) {
        (
            (f64::from(a) - f64::from(b)) as f32,
            (f64::from(c) - f64::from(d)) as f32,
        )
    };
    let mut total = 0u32;
    for &(a, b, c, d) in &[
        (0.0f32, 0.0, 0.0, 0.0), // the real runtime case (operands zeroed)
        (0.0, -0.0, -0.0, 0.0),
        (1.0, 1.0, 2.5, 2.5), // exact cancellation -> +0.0
        (5.0, 2.0, -3.0, 4.0),
        (1e9, 1.0, -1e9, 1.0),
    ] {
        let (g0, g1) = run(&mut emu, a, b, c, d);
        let (w0, w1) = want(a, b, c, d);
        assert_eq!(
            (g0.to_bits(), g1.to_bits()),
            (w0.to_bits(), w1.to_bits()),
            "deltas {a},{b},{c},{d}"
        );
        total += 1;
    }
    let mut s = 0x6c7f_5c00u32;
    for _ in 0..4000 {
        let a = rngf(&mut s, -1000.0, 1000.0) as f32;
        let b = rngf(&mut s, -1000.0, 1000.0) as f32;
        let c = rngf(&mut s, -1000.0, 1000.0) as f32;
        let d = rngf(&mut s, -1000.0, 1000.0) as f32;
        let (g0, g1) = run(&mut emu, a, b, c, d);
        let (w0, w1) = want(a, b, c, d);
        assert_eq!(
            (g0.to_bits(), g1.to_bits()),
            (w0.to_bits(), w1.to_bits()),
            "deltas fuzz"
        );
        total += 1;
    }
    eprintln!("pn_glyph_quad_build_deltas: {total} bit-exact (2× fld; fsub; fstp)");
}
