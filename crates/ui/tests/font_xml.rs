//! Differential tests of the arithmetic of the font loaders `0x77e8d0`, `0x77f0c0` and
//! `CFontObject::LoadXML` (`0x783c30`), each run against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fxch`/`fnstsw`
//! excluded), counted in each `[addr, true_ret)`:
//!
//! * `0x77e8d0`  [.,0x77f0b6 ret 4] : `fchs` ×8 (0x77e956/0x77e978/0x77e986/0x77e9b1/0x77ea47/
//!   0x77eaf1/0x77ebb2/0x77ec5c). The eight sites are the byte-identical opcode `d9 e0`.
//! * `0x77f0c0`  [.,0x77f3fe ret 4] : Block 1 = fdiv×1 + fsub×4 + fmul×1 + fmulp×1 (=7);
//!   Block 2 = fsub×2 + fdivrp×2 (=4).  Total 11.
//! * `0x783c30`  [.,0x78411e ret 8] : fsubr×2 + fabs×2 + fmul×1 + fdivr×1 (=6).
//!
//! Diff strategy per op:
//! * `0x77e8d0` `fchs` — a single **copied-byte island** `fld qword [in]; <d9 e0>; fstp qword [out]`,
//!   the `d9 e0` byte-verified `== ALL 8` source sites (else circular). The sign-of-zero runs
//!   natively in the island; the boundary probe sweeps ±0 and signed values.
//! * `0x77f0c0` Block 1 / Block 2 — own-address **slices** (`Emu::run`) over the binary's own bytes
//!   (so the LIVE `fmul st,st(1)`/`fmulp`, and the `fdivrp` SWAP all execute as the binary does),
//!   feeding `esi`/`edi` + the scale fields and reading the f32 results the binary stores to stack.
//! * `0x783c30` — **copied-byte islands** reconstructing the owned arith with absolute operands: the
//!   operand-free register/abs ops (`fabs` `d9 e1`, `fsubr st,st(1)` `d8 e9`, `fmul dword [abs]`
//!   `d8 0d …`) are copied VERBATIM and byte-verified == source; the ebp-relative `fsubr`/`fdivr`
//!   are reconstructed as the same opcode+reg-field (`d8 /5`, `d8 /7`) with an absolute disp32 and
//!   the source's opcode-byte + reg-field byte-verified (so reverse direction is the source's,
//!   not asserted). Each island ends with a real `fstp qword [out]` read back as f64.
//!
//! Fuzz reads the LCG's HIGH bits. The mandatory `==`-boundary probes
//! sit on each op's pivots: `fchs` ±0; the clamp/divisor `== 0.0` pivots; the epsilon `2^-22` deltas.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::font_xml::{
    block1_offsets, block2_divisor, block2_quotients, height_delta_abs, negate, spacing_delta_abs,
    spacing_quotient, spacing_quotient_f32,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x200);
    let ebp = scratch + 0x100; // room for [ebp-0x100 .. ebp+0x40]
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1f00;
    Frame { ebp, esp }
}

// ===========================================================================================
// Row 0x77e8d0 — fchs ×8 via a copied-byte island.
//   fld qword [in]; <source d9 e0 (fchs)>; fstp qword [out]
// The `d9 e0` is byte-verified == ALL 8 source sites; the island captures every site's owned op.
// ===========================================================================================
#[test]
fn glue_geom_77e8d0_negate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xddu8, 0x05]; // fld qword [in_cell]  = DD 05 <disp32>
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let fchs_off = code.len() as u64;
    code.extend_from_slice(&[0xd9, 0xe0]); // copied source fchs
    code.push(0xdd); // fstp qword [out_cell]  = DD 1D <disp32>
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // byte-verify the island's fchs == EVERY source site (else the island proves nothing).
    let isl_fchs = isl + fchs_off;
    for src in [
        0x77e956u64,
        0x77e978,
        0x77e986,
        0x77e9b1,
        0x77ea47,
        0x77eaf1,
        0x77ebb2,
        0x77ec5c,
    ] {
        assert_eq!(
            emu.read(isl_fchs, 2),
            emu.read(src, 2),
            "fchs island byte != source @ {src:#x}"
        );
    }

    // The fchs operand is f32(field) widened to f64 then narrowed back on the f32 store. Feed the
    // f32 value through the island as a qword (PC_53 widens exactly), read the negated f64, narrow.
    let run = |emu: &mut Emu, v: f32| -> f32 {
        emu.write(in_cell, &f64::from(v).to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap())) as f32
    };

    let mut total = 0u32;
    // boundary: ±0 (sign-of-zero pivot), ±1, representative magnitudes, inf.
    let probes = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1234.5,
        -1234.5,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &v in &probes {
        assert_eq!(
            run(&mut emu, v).to_bits(),
            negate(v).to_bits(),
            "negate v={v}"
        );
        total += 1;
    }
    let mut s = 0x77e8_d000u32;
    for _ in 0..4000 {
        let v = rngf(&mut s, -1e6, 1e6) as f32;
        assert_eq!(
            run(&mut emu, v).to_bits(),
            negate(v).to_bits(),
            "negate fuzz v={v}"
        );
        total += 1;
    }
    eprintln!("glue_geom_77e8d0_negate: {total} bit-exact (fchs island == all 8 sites)");
}

// ===========================================================================================
// Row 0x77f0c0 — own-address slices over the binary's own bytes.
//
// Block 1 slice 0x77f0fb..0x77f125: fld 1.0; fdiv [esi+0x48]; fld [edi+0xc]; fsub [edi+4];
//   fmul st,st(1); fsub [0x801628]; fstp [ebp+8]; fld [edi+8]; fsub [edi]; fmulp; fsub [0x801628];
//   fst [ebp-4]; (halt at 0x77f125, before the reload+clamp). Read [ebp+8]=fVar2, [ebp-4]=out10.
//
// Block 2 slice 0x77f175..0x77f1e8: divisor selects + two fdivrp; halt at the fxch (0x77f1e8),
//   after `fst [ebp-8]`=B and `fstp [ebp-0xc]`=A both run. Read [ebp-8]=B(=out3), [ebp-0xc]=A(=out4).
// ===========================================================================================
#[test]
fn glue_geom_77f0c0_scale_offset_divide() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let this = emu.alloc(0x80); // esi base (scale at +0x48, alt at +0x4c)
    let rect = emu.alloc(0x10); // edi base: p0..p3

    let setup = |emu: &mut Emu, scale: f32, alt: f32, p: [f32; 4]| {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::EDI, rect);
        emu.write(this + 0x48, &scale.to_bits().to_le_bytes());
        emu.write(this + 0x4c, &alt.to_bits().to_le_bytes());
        for (i, &v) in p.iter().enumerate() {
            emu.write(rect + (i as u64) * 4, &v.to_bits().to_le_bytes());
        }
    };

    let run_b1 = |emu: &mut Emu, scale: f32, p: [f32; 4]| -> (f32, f32) {
        setup(emu, scale, 1.0, p);
        emu.run(0x77f0fb, 0x77f125);
        (emu.read_f32(f.ebp + 0x8), emu.read_f32(f.ebp - 0x4))
    };
    let run_b2 = |emu: &mut Emu, scale: f32, alt: f32, p: [f32; 4]| -> (f32, f32) {
        setup(emu, scale, alt, p);
        emu.run(0x77f175, 0x77f1e8);
        // B (=out3) at [ebp-8]; A (=out4) at [ebp-0xc].
        let b = emu.read_f32(f.ebp - 0x8);
        let a = emu.read_f32(f.ebp - 0xc);
        (a, b)
    };

    let mut total = 0u32;

    // ---- Block 1 (fdiv, fsub×4, fmul[st,st1], fmulp) -------------------------------------
    let b1_probes: [(f32, [f32; 4]); 6] = [
        (1.0, [0.0, 0.0, 0.0, 0.0]),       // all zero -> products 0; −2.0 each
        (1.0, [10.0, 20.0, 110.0, 220.0]), // recip=1; (220-20)=200, (110-10)=100
        (2.0, [1.0, 2.0, 3.0, 4.0]),       // recip=0.5
        (0.8, [5.5, 1.25, 9.0, 3.0]),
        (256.0, [0.0, 0.0, 512.0, 768.0]), // recip tiny
        (3.0, [-4.0, 4.0, -8.0, 8.0]),     // negative coords
    ];
    for &(scale, p) in &b1_probes {
        let got = run_b1(&mut emu, scale, p);
        let want = block1_offsets(scale, p[0], p[1], p[2], p[3]);
        assert_eq!(
            (got.0.to_bits(), got.1.to_bits()),
            (want.0.to_bits(), want.1.to_bits()),
            "block1 scale={scale} p={p:?}"
        );
        total += 1;
    }
    let mut s = 0x77f0_c001u32;
    for _ in 0..3000 {
        let scale = rngf(&mut s, 0.05, 4096.0) as f32;
        let p = [
            rngf(&mut s, -2048.0, 2048.0) as f32,
            rngf(&mut s, -2048.0, 2048.0) as f32,
            rngf(&mut s, -2048.0, 2048.0) as f32,
            rngf(&mut s, -2048.0, 2048.0) as f32,
        ];
        let got = run_b1(&mut emu, scale, p);
        let want = block1_offsets(scale, p[0], p[1], p[2], p[3]);
        assert_eq!(
            (got.0.to_bits(), got.1.to_bits()),
            (want.0.to_bits(), want.1.to_bits()),
            "block1 fuzz scale={scale} p={p:?}"
        );
        total += 1;
    }

    // ---- Block 2 (fsub×2, fdivrp×2 SWAP, divisor select) ---------------------------------
    // boundary: alt == 0.0 (the divisor-select pivot -> use scale), alt != 0, ±0 forms.
    let b2_probes: [(f32, f32, [f32; 4]); 7] = [
        (2.0, 0.0, [10.0, 20.0, 110.0, 220.0]), // alt==0 -> divisor=scale=2
        (2.0, -0.0, [10.0, 20.0, 110.0, 220.0]), // alt==-0 -> divisor=scale
        (2.0, 4.0, [10.0, 20.0, 110.0, 220.0]), // alt!=0 -> divisor=alt=4
        (0.5, 0.0, [1.0, 2.0, 3.0, 4.0]),
        (256.0, 64.0, [0.0, 0.0, 128.0, 256.0]),
        (1.0, 0.0, [-4.0, 4.0, -8.0, 8.0]),
        (3.0, 0.6, [5.5, 1.25, 9.0, 3.0]),
    ];
    for &(scale, alt, p) in &b2_probes {
        // sanity on the model's divisor selection itself
        let _ = block2_divisor(alt, scale);
        let got = run_b2(&mut emu, scale, alt, p);
        let want = block2_quotients(alt, scale, p[0], p[1], p[2], p[3]);
        assert_eq!(
            (got.0.to_bits(), got.1.to_bits()),
            (want.0.to_bits(), want.1.to_bits()),
            "block2 scale={scale} alt={alt} p={p:?}"
        );
        total += 1;
    }
    let mut s = 0x77f0_c002u32;
    for _ in 0..3000 {
        let scale = rngf(&mut s, 0.05, 4096.0) as f32;
        // half the time force alt==0 to exercise the select pivot
        let alt = if next(&mut s) & 1 == 0 {
            0.0
        } else {
            rngf(&mut s, -1024.0, 1024.0) as f32
        };
        let p = [
            rngf(&mut s, -2048.0, 2048.0) as f32,
            rngf(&mut s, -2048.0, 2048.0) as f32,
            rngf(&mut s, -2048.0, 2048.0) as f32,
            rngf(&mut s, -2048.0, 2048.0) as f32,
        ];
        let got = run_b2(&mut emu, scale, alt, p);
        let want = block2_quotients(alt, scale, p[0], p[1], p[2], p[3]);
        assert_eq!(
            (got.0.to_bits(), got.1.to_bits()),
            (want.0.to_bits(), want.1.to_bits()),
            "block2 fuzz scale={scale} alt={alt} p={p:?}"
        );
        total += 1;
    }

    eprintln!(
        "glue_geom_77f0c0_scale_offset_divide: {total} bit-exact (Block1 7 ops slice; Block2 4 ops slice incl fdivrp SWAP)"
    );
}

// ===========================================================================================
// Row 0x783c30 — copied-byte islands for the owned arith.
//
// (A) fsubr [m32]; fabs (0x783e48/0x783e4b): island
//       fld dword [cur]; fsubr dword [new]; fabs; fstp qword [out]
//     where `fsubr dword [new]` is the SAME opcode d8 + reg-field /5 as source 0x783e48 (verified),
//     reconstructed with an absolute disp32; `fabs` (d9 e1) copied VERBATIM == 0x783e4b/0x783f1a.
//
// (B) fmul [abs]; fdivr [m32] (0x783f03/0x783f0a): island
//       fld dword [scale]; <source d8 0d 68 fd 7f 00 (fmul [0x7ffd68])>; fdivr dword [spacing];
//       fstp qword [out]
//     fmul copied VERBATIM (absolute operand) == source 0x783f03; `fdivr dword [spacing]` is the
//     SAME opcode d8 + reg-field /7 as source 0x783f0a (verified), absolute disp32.
//
// (C) fsubr st,st(1); fabs (0x783f18/0x783f1a): island
//       fld dword [old]; fld dword [new]; fxch st(1); <source d8 e9>; <source d9 e1>; fstp qword [out]
//     We need st0=old, st1=new before the fsubr (source: old from fld [esi+0x50] on st0, new the gx
//     leaf result already on the stack as st1). Build that with fld old; fld new; fxch. `d8 e9` and
//     `d9 e1` copied VERBATIM == source 0x783f18/0x783f1a.
// ===========================================================================================
#[test]
fn glue_geom_783c30_fontheight_spacing() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // shared cells
    let a0 = emu.alloc(8);
    let a1 = emu.alloc(8);
    let out = emu.alloc(8);

    // -- island (A): height_delta_abs ----------------------------------------------------
    let isl_a = emu.alloc(0x20);
    let mut ca = vec![0xd9u8, 0x05]; // fld dword [a0]=cur  (D9 05 disp32)
    ca.extend_from_slice(&(a0 as u32).to_le_bytes());
    let a_fsubr = ca.len() as u64;
    ca.extend_from_slice(&[0xd8, 0x2d]); // fsubr dword [a1]=new  (D8 /5 abs = D8 2D disp32)
    ca.extend_from_slice(&(a1 as u32).to_le_bytes());
    let a_fabs = ca.len() as u64;
    ca.extend_from_slice(&[0xd9, 0xe1]); // fabs (copied verbatim)
    ca.push(0xdd); // fstp qword [out]  (DD 1D disp32)
    ca.push(0x1d);
    ca.extend_from_slice(&(out as u32).to_le_bytes());
    let isl_a_end = isl_a + ca.len() as u64;
    emu.write(isl_a, &ca);
    // verify: source fsubr opcode + reg-field /5; fabs verbatim
    let src_a_fsubr = emu.read(0x783e48, 1);
    assert_eq!(src_a_fsubr[0], 0xd8, "regionA fsubr opcode != d8");
    let src_a_modrm = emu.read(0x783e49, 1)[0];
    assert_eq!((src_a_modrm >> 3) & 7, 5, "regionA fsubr reg-field != /5");
    assert_eq!(
        emu.read(isl_a + a_fsubr, 1)[0],
        0xd8,
        "island A fsubr opcode"
    );
    assert_eq!(
        (emu.read(isl_a + a_fsubr + 1, 1)[0] >> 3) & 7,
        5,
        "island A fsubr reg-field"
    );
    assert_eq!(
        emu.read(isl_a + a_fabs, 2),
        emu.read(0x783e4b, 2),
        "regionA fabs verbatim != source 0x783e4b"
    );

    let run_a = |emu: &mut Emu, cur: f32, new: f32| -> f64 {
        emu.write(a0, &cur.to_bits().to_le_bytes());
        emu.write_u32(a0 + 4, 0);
        emu.write(a1, &new.to_bits().to_le_bytes());
        emu.write_u32(a1 + 4, 0);
        emu.run(isl_a, isl_a_end);
        let b = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    // -- island (B): spacing_quotient ----------------------------------------------------
    let isl_b = emu.alloc(0x20);
    let mut cb = vec![0xd9u8, 0x05]; // fld dword [a0]=scale_factor
    cb.extend_from_slice(&(a0 as u32).to_le_bytes());
    let b_fmul = cb.len() as u64;
    cb.extend_from_slice(&[0xd8, 0x0d, 0x68, 0xfd, 0x7f, 0x00]); // copied source fmul dword [0x7ffd68]
    let b_fdivr = cb.len() as u64;
    cb.extend_from_slice(&[0xd8, 0x3d]); // fdivr dword [a1]=spacing  (D8 /7 abs = D8 3D disp32)
    cb.extend_from_slice(&(a1 as u32).to_le_bytes());
    cb.push(0xdd); // fstp qword [out]
    cb.push(0x1d);
    cb.extend_from_slice(&(out as u32).to_le_bytes());
    let isl_b_end = isl_b + cb.len() as u64;
    emu.write(isl_b, &cb);
    // verify: fmul copied verbatim == source; fdivr opcode + reg-field /7
    assert_eq!(
        emu.read(isl_b + b_fmul, 6),
        emu.read(0x783f03, 6),
        "regionB fmul verbatim != source 0x783f03"
    );
    let src_b_fdivr = emu.read(0x783f0a, 1)[0];
    assert_eq!(src_b_fdivr, 0xd8, "regionB fdivr opcode != d8");
    let src_b_modrm = emu.read(0x783f0b, 1)[0];
    assert_eq!((src_b_modrm >> 3) & 7, 7, "regionB fdivr reg-field != /7");
    assert_eq!(
        emu.read(isl_b + b_fdivr, 1)[0],
        0xd8,
        "island B fdivr opcode"
    );
    assert_eq!(
        (emu.read(isl_b + b_fdivr + 1, 1)[0] >> 3) & 7,
        7,
        "island B fdivr reg-field"
    );

    let run_b = |emu: &mut Emu, scale_factor: f32, spacing: f32| -> f64 {
        emu.write(a0, &scale_factor.to_bits().to_le_bytes());
        emu.write_u32(a0 + 4, 0);
        emu.write(a1, &spacing.to_bits().to_le_bytes());
        emu.write_u32(a1 + 4, 0);
        emu.run(isl_b, isl_b_end);
        let b = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    // -- island (C): spacing_delta_abs (fsubr st,st(1); fabs) ----------------------------
    let isl_c = emu.alloc(0x20);
    let mut cc = vec![0xd9u8, 0x05]; // fld dword [a0]=old   -> st0=old
    cc.extend_from_slice(&(a0 as u32).to_le_bytes());
    cc.extend_from_slice(&[0xd9, 0x05]); // fld dword [a1]=new   -> st0=new, st1=old
    cc.extend_from_slice(&(a1 as u32).to_le_bytes());
    cc.extend_from_slice(&[0xd9, 0xc9]); // fxch st(1)           -> st0=old, st1=new
    let c_fsubr = cc.len() as u64;
    cc.extend_from_slice(&[0xd8, 0xe9]); // copied source fsubr st,st(1)  -> st0 = st1 - st0 = new - old
    let c_fabs = cc.len() as u64;
    cc.extend_from_slice(&[0xd9, 0xe1]); // copied source fabs
    cc.push(0xdd); // fstp qword [out]
    cc.push(0x1d);
    cc.extend_from_slice(&(out as u32).to_le_bytes());
    let isl_c_end = isl_c + cc.len() as u64;
    emu.write(isl_c, &cc);
    assert_eq!(
        emu.read(isl_c + c_fsubr, 2),
        emu.read(0x783f18, 2),
        "regionC fsubr st,st(1) verbatim != source 0x783f18"
    );
    assert_eq!(
        emu.read(isl_c + c_fabs, 2),
        emu.read(0x783f1a, 2),
        "regionC fabs verbatim != source 0x783f1a"
    );

    let run_c = |emu: &mut Emu, old: f32, new: f32| -> f64 {
        emu.write(a0, &old.to_bits().to_le_bytes());
        emu.write_u32(a0 + 4, 0);
        emu.write(a1, &new.to_bits().to_le_bytes());
        emu.write_u32(a1 + 4, 0);
        emu.run(isl_c, isl_c_end);
        let b = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;

    // (A) height_delta_abs — boundary: equal (Δ=0), ±eps pivots, signed.
    let eps = f32::from_bits(0x3480_0000); // 2^-22
    let a_probes: [(f32, f32); 8] = [
        (12.0, 12.0),       // Δ = 0
        (12.0, 12.0 + eps), // Δ = +eps (the == pivot)
        (12.0, 12.0 - eps),
        (0.0, 0.0),
        (-5.0, 5.0),
        (5.0, -5.0),
        (16.0, 14.5),
        (100.0, 0.0),
    ];
    for &(cur, new) in &a_probes {
        assert_eq!(
            run_a(&mut emu, cur, new).to_bits(),
            height_delta_abs(cur, new).to_bits(),
            "height_delta_abs cur={cur} new={new}"
        );
        total += 1;
    }
    let mut s = 0x783c_3001u32;
    for _ in 0..2500 {
        let cur = rngf(&mut s, 0.0, 200.0) as f32;
        let new = rngf(&mut s, 0.0, 200.0) as f32;
        assert_eq!(
            run_a(&mut emu, cur, new).to_bits(),
            height_delta_abs(cur, new).to_bits(),
            "height_delta_abs fuzz cur={cur} new={new}"
        );
        total += 1;
    }

    // (B) spacing_quotient — boundary: spacing 0, scale_factor 1 (product 1024), etc.
    let b_probes: [(f32, f32); 6] = [
        (1.0, 0.0),     // spacing 0 -> 0 / 1024
        (1.0, 1024.0),  // 1024 / 1024 = 1
        (1.0, 1.0),     // 1 / 1024
        (0.5, 512.0),   // 512 / 512 = 1
        (2.0, 4096.0),  // 4096 / 2048
        (0.25, -100.0), // negative spacing
    ];
    for &(sf, sp) in &b_probes {
        // diff the full f64 quotient...
        assert_eq!(
            run_b(&mut emu, sf, sp).to_bits(),
            spacing_quotient(sf, sp).to_bits(),
            "spacing_quotient sf={sf} sp={sp}"
        );
        // ...and the f32-narrowed value the source's fstp [esp] hands the gx leaf.
        assert_eq!(
            (run_b(&mut emu, sf, sp) as f32).to_bits(),
            spacing_quotient_f32(sf, sp).to_bits(),
            "spacing_quotient_f32 sf={sf} sp={sp}"
        );
        total += 1;
    }
    let mut s = 0x783c_3002u32;
    for _ in 0..2500 {
        let sf = rngf(&mut s, 0.05, 8.0) as f32;
        let sp = rngf(&mut s, -64.0, 64.0) as f32;
        assert_eq!(
            run_b(&mut emu, sf, sp).to_bits(),
            spacing_quotient(sf, sp).to_bits(),
            "spacing_quotient fuzz sf={sf} sp={sp}"
        );
        total += 1;
    }

    // (C) spacing_delta_abs — boundary: equal, ±eps, signed.
    let c_probes: [(f32, f32); 7] = [
        (1.0, 1.0),
        (1.0, 1.0 + eps),
        (1.0, 1.0 - eps),
        (0.0, 0.0),
        (-2.0, 3.0),
        (3.0, -2.0),
        (0.75, 0.5),
    ];
    for &(old, new) in &c_probes {
        assert_eq!(
            run_c(&mut emu, old, new).to_bits(),
            spacing_delta_abs(old, new).to_bits(),
            "spacing_delta_abs old={old} new={new}"
        );
        total += 1;
    }
    let mut s = 0x783c_3003u32;
    for _ in 0..2500 {
        let old = rngf(&mut s, -8.0, 8.0) as f32;
        let new = rngf(&mut s, -8.0, 8.0) as f32;
        assert_eq!(
            run_c(&mut emu, old, new).to_bits(),
            spacing_delta_abs(old, new).to_bits(),
            "spacing_delta_abs fuzz old={old} new={new}"
        );
        total += 1;
    }

    eprintln!(
        "glue_geom_783c30_fontheight_spacing: {total} bit-exact (islands: fsubr[/5];fabs, fmul[abs];fdivr[/7], fsubr st,st(1);fabs)"
    );
}
