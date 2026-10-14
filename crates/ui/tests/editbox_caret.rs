//! Differential tests of the arithmetic of the edit-box update and layout functions `0x77a790`,
//! `0x77a9f0`, `0x77ad10` and `0x77b8c0` (caret width, fill and text placement), each run against
//! `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed by executing the binary's OWN bytes — either an in-place
//! instruction **slice** (`Emu::run`, registers/memory synthesized, the stored value read back) or
//! a **copied-byte island** where the source interleaves an object-graph deref / `push` between the
//! FP ops and the result is `fstp`'d to `[esp]` (so it can't be read in place). An island copies the
//! source's EXACT FP-op bytes contiguously and appends `fstp [cell]` (a `D9 1D`+disp32, same store
//! width as the source's `D9 1C 24`); each FP op's bytes are byte-verified against its REAL source
//! address (islands credited only when the bytes match source). The `0x77b8c0` region-D
//! blend runs as a native slice INCLUDING its two helper calls (`0x41ad70` = `fld 1.0; ret`,
//! `0x41ae60` = `fld 0.8; fmul [arg]; ret 4`), which execute as real functions — so the `fmul`/
//! `fdivr` it owns are covered by construction. Fuzz reads the LCG's HIGH bits.
//!
//! Coverage map (every arithmetic op in `[addr,true_ret)` lands in an executed slice/island):
//!
//! ```text
//!   0x77a790  fadd@0x77a7dc                                        slice  → fill_accumulate test
//!   0x77a9f0  fmul@0x77aa9d                                        island → height_x10 test
//!   0x77ad10  fmul@0x77ada5/0x77adab + faddp@0x77adb1              island ┐
//!             fmul@0x77adeb + fchs@0x77adee                        island ┼ row_geom test
//!             fmul@0x77ae2b + fchs@0x77ae2e                        island ┘
//!   0x77b8c0  fsub@0x77b8e2 + fdiv@0x77b8f0 + fadd@0x77b8fa + fsubp@0x77b900   island ┐ (A)
//!             fsub@0x77b928 + fdiv@0x77b92c + fadd@0x77b935 + fsubp@0x77b93b   island ┼ (B)
//!             fchs@0x77b94b                                                    island ┤ (C)
//!             fmul@0x77ba32 + fdivr@0x77ba39                          native slice ┘ (D)  fill_edges
//! ```
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::editbox_caret::{
    blend_const, fill_accumulate, fill_edge, height_x10, neg_anchor, row_extent, row_offset_neg,
    K1024_BITS, ONE_BITS, TENTH_BITS, TEN_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

struct Frame {
    ebp: u64,
    esp: u64,
    esi: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    let esi = emu.alloc(0x800); // 0x77b8c0 / region-D read [esi+disp] up to +0x3f4
    Frame { ebp, esp, esi }
}

/// Append a `fstp dword ptr [cell]` (`D9 1D` + disp32) to an island body.
fn append_fstp(body: &mut Vec<u8>, cell: u64) {
    body.push(0xd9);
    body.push(0x1d);
    body.extend_from_slice(&(cell as u32).to_le_bytes());
}

// ===========================================================================================
// 0x77a790 — fill accumulate: fld [ebp+8]; fadd [esi+0x374]; fst [esi+0x374]. The two operands
// are memory we control and the result is stored to [esi+0x374] in place (no interleaved deref),
// so it is diffed at its OWN address. fadd ×1 @0x77a7dc.
// ===========================================================================================
#[test]
fn glue_77a_77a790_fill_accumulate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    const F_FILL: u64 = 0x374;

    let run = |emu: &mut Emu, field: f32, arg: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.write_u32(f.esi + F_FILL, field.to_bits());
        emu.run(0x77a7d9, 0x77a7e8); // fld; fadd; fst [esi+0x374] — halt before mov eax,[abs]
        emu.read_f32(f.esi + F_FILL)
    };

    let mut total = 0u32;
    for (field, arg) in [
        (0.0f32, 0.0f32),
        (1.0, 0.0),
        (0.0, 1.0),
        (100.0, 0.5),
        (-5.0, 5.0),  // sum 0
        (1.0e7, 1.0), // arg lost below f32 ulp of field
        (0.3, 0.3),   // non-representable both
        (16_777_216.0, 1.0),
    ] {
        assert_eq!(
            run(&mut emu, field, arg).to_bits(),
            fill_accumulate(field, arg).to_bits(),
            "fill_accumulate field={field} arg={arg}"
        );
        total += 1;
    }
    let mut s = 0x77a7_9000u32;
    for _ in 0..4000 {
        let field = rng(&mut s, -10000.0, 10000.0);
        let arg = rng(&mut s, -10000.0, 10000.0);
        assert_eq!(
            run(&mut emu, field, arg).to_bits(),
            fill_accumulate(field, arg).to_bits(),
            "fill_accumulate fuzz field={field} arg={arg}"
        );
        total += 1;
    }
    eprintln!("glue_77a_77a790_fill_accumulate: {total} bit-exact (fadd field+=arg)");
}

// ===========================================================================================
// 0x77a9f0 — fld [ebp-4]; fmul [10.0]; (lea/mov/push) fstp [esp]. The fmul's result is stored to
// [esp] after an interleaved object deref, so it is diffed via a COPIED-BYTE ISLAND of the two FP
// ops (byte-verified) + an appended fstp [cell]. fmul ×1 @0x77aa9d.
// ===========================================================================================
#[test]
fn glue_77a_77a9f0_height_x10() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let cell = emu.alloc(4);
    let isl = emu.alloc(0x20);

    // fld [ebp-4] (D9 45 FC) ; fmul [0x801624] (D8 0D 24 16 80 00)
    let mut body = vec![0xd9u8, 0x45, 0xfc];
    body.extend_from_slice(&[0xd8, 0x0d, 0x24, 0x16, 0x80, 0x00]);
    append_fstp(&mut body, cell);
    let isl_end = isl + body.len() as u64;
    emu.write(isl, &body);
    // byte-verify each FP op against its real source address.
    assert_eq!(&emu.read(isl, 3), &emu.read(0x77aa9a, 3), "isl fld != src");
    assert_eq!(
        &emu.read(isl + 3, 6),
        &emu.read(0x77aa9d, 6),
        "isl fmul != src"
    );

    let run = |emu: &mut Emu, v: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.write_u32(f.ebp - 0x4, v.to_bits());
        emu.run(isl, isl_end);
        emu.read_f32(cell)
    };

    let mut total = 0u32;
    for v in [
        0.0f32,
        1.0,
        0.1,
        -2.5,
        1.0e6,
        9.81,
        f32::from_bits(0x4b00_0001),
    ] {
        assert_eq!(
            run(&mut emu, v).to_bits(),
            height_x10(v).to_bits(),
            "height_x10 v={v}"
        );
        total += 1;
    }
    let mut s = 0x77a9_f000u32;
    for _ in 0..4000 {
        let v = rng(&mut s, -10000.0, 10000.0);
        assert_eq!(
            run(&mut emu, v).to_bits(),
            height_x10(v).to_bits(),
            "height_x10 fuzz v={v}"
        );
        total += 1;
    }
    eprintln!(
        "glue_77a_77a9f0_height_x10: {total} bit-exact (copied-byte island fld;fmul[10.0];fstp)"
    );
}

// ===========================================================================================
// 0x77ad10 — three FP regions, each ending fstp [esp] after an interleaved push/deref:
//   r1 (0x77ada1..0x77adb3): fild qword [ebp-0xc]; (push ecx); fmul [ebp+8]; fld [ebp+8];
//       fmul [0.1]; faddp st(1),st  → n·scale + scale·0.1
//   r2 (0x77ade8..0x77adf0): fild qword [ebp-0xc]; fmul [ebp+8]; fchs  → -(n·scale)
//   r3 (0x77ae28..0x77ae30): same shape as r2 at a different address
// All three diffed via COPIED-BYTE ISLANDS (r1's `push ecx` interleaved between fild and fmul; r2/r3
// store to [esp]). fmul ×4 + faddp ×1 + fchs ×2.
// ===========================================================================================
#[test]
fn glue_77a_77ad10_row_geom() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // --- region 1 island: fild qword [ebp-0xc]; fmul [ebp+8]; fld [ebp+8]; fmul [0x801620]; faddp.
    let cell1 = emu.alloc(4);
    let isl1 = emu.alloc(0x20);
    let mut b1 = vec![0xdfu8, 0x6d, 0xf4]; // fild qword [ebp-0xc]
    b1.extend_from_slice(&[0xd8, 0x4d, 0x08]); // fmul [ebp+8]
    b1.extend_from_slice(&[0xd9, 0x45, 0x08]); // fld [ebp+8]
    b1.extend_from_slice(&[0xd8, 0x0d, 0x20, 0x16, 0x80, 0x00]); // fmul [0x801620]
    b1.extend_from_slice(&[0xde, 0xc1]); // faddp st(1),st
    append_fstp(&mut b1, cell1);
    let isl1_end = isl1 + b1.len() as u64;
    emu.write(isl1, &b1);
    // byte-verify against source (the source interleaves `push ecx` between fild and fmul, so each
    // op is verified against its own source span — not a contiguous window).
    assert_eq!(&emu.read(isl1, 3), &emu.read(0x77ada1, 3), "r1 fild != src");
    assert_eq!(
        &emu.read(isl1 + 3, 3),
        &emu.read(0x77ada5, 3),
        "r1 fmul1 != src"
    );
    assert_eq!(
        &emu.read(isl1 + 6, 3),
        &emu.read(0x77ada8, 3),
        "r1 fld != src"
    );
    assert_eq!(
        &emu.read(isl1 + 9, 6),
        &emu.read(0x77adab, 6),
        "r1 fmul2 != src"
    );
    assert_eq!(
        &emu.read(isl1 + 15, 2),
        &emu.read(0x77adb1, 2),
        "r1 faddp != src"
    );

    // --- region 2 island: fild qword [ebp-0xc]; fmul [ebp+8]; fchs.
    let cell2 = emu.alloc(4);
    let isl2 = emu.alloc(0x20);
    let mut b2 = vec![0xdfu8, 0x6d, 0xf4]; // fild qword [ebp-0xc]
    b2.extend_from_slice(&[0xd8, 0x4d, 0x08]); // fmul [ebp+8]
    b2.extend_from_slice(&[0xd9, 0xe0]); // fchs
    append_fstp(&mut b2, cell2);
    let isl2_end = isl2 + b2.len() as u64;
    emu.write(isl2, &b2);
    assert_eq!(&emu.read(isl2, 3), &emu.read(0x77ade8, 3), "r2 fild != src");
    assert_eq!(
        &emu.read(isl2 + 3, 3),
        &emu.read(0x77adeb, 3),
        "r2 fmul != src"
    );
    assert_eq!(
        &emu.read(isl2 + 6, 2),
        &emu.read(0x77adee, 2),
        "r2 fchs != src"
    );

    // --- region 3 island: identical shape at 0x77ae28 (verified against its OWN source addresses).
    let cell3 = emu.alloc(4);
    let isl3 = emu.alloc(0x20);
    let mut b3 = vec![0xdfu8, 0x6d, 0xf4];
    b3.extend_from_slice(&[0xd8, 0x4d, 0x08]);
    b3.extend_from_slice(&[0xd9, 0xe0]);
    append_fstp(&mut b3, cell3);
    let isl3_end = isl3 + b3.len() as u64;
    emu.write(isl3, &b3);
    assert_eq!(&emu.read(isl3, 3), &emu.read(0x77ae28, 3), "r3 fild != src");
    assert_eq!(
        &emu.read(isl3 + 3, 3),
        &emu.read(0x77ae2b, 3),
        "r3 fmul != src"
    );
    assert_eq!(
        &emu.read(isl3 + 6, 2),
        &emu.read(0x77ae2e, 2),
        "r3 fchs != src"
    );

    // write n (i64 little-endian) at [ebp-0xc] (low) / [ebp-8] (high) and scale at [ebp+8].
    let set_inputs = |emu: &mut Emu, n: i64, scale: f32| {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        let bytes = n.to_le_bytes();
        emu.write(f.ebp - 0xc, &bytes[0..4]);
        emu.write(f.ebp - 0x8, &bytes[4..8]);
        emu.write_u32(f.ebp + 0x8, scale.to_bits());
    };
    let run_extent = |emu: &mut Emu, n: i64, scale: f32| -> f32 {
        set_inputs(emu, n, scale);
        emu.run(isl1, isl1_end);
        emu.read_f32(cell1)
    };
    let run_neg2 = |emu: &mut Emu, n: i64, scale: f32| -> f32 {
        set_inputs(emu, n, scale);
        emu.run(isl2, isl2_end);
        emu.read_f32(cell2)
    };
    let run_neg3 = |emu: &mut Emu, n: i64, scale: f32| -> f32 {
        set_inputs(emu, n, scale);
        emu.run(isl3, isl3_end);
        emu.read_f32(cell3)
    };

    let mut total = 0u32;
    // n is {count+1, 0} / row indices — non-negative in this band. Probe small + large.
    let ns: [i64; 9] = [0, 1, 2, 3, 7, 8, 100, 1000, 1_000_000];
    let scales = [0.0f32, 1.0, 12.0, 0.5, -3.5, 18.0, 0.1];
    for &n in &ns {
        for &scale in &scales {
            assert_eq!(
                run_extent(&mut emu, n, scale).to_bits(),
                row_extent(n, scale).to_bits(),
                "row_extent n={n} scale={scale}"
            );
            assert_eq!(
                run_neg2(&mut emu, n, scale).to_bits(),
                row_offset_neg(n, scale).to_bits(),
                "row_offset_neg(r2) n={n} scale={scale}"
            );
            assert_eq!(
                run_neg3(&mut emu, n, scale).to_bits(),
                row_offset_neg(n, scale).to_bits(),
                "row_offset_neg(r3) n={n} scale={scale}"
            );
            total += 3;
        }
    }
    let mut s = 0x77ad_1000u32;
    for _ in 0..2000 {
        let n = i64::from(next(&mut s) >> 8); // non-negative, fits 24 bits
        let scale = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run_extent(&mut emu, n, scale).to_bits(),
            row_extent(n, scale).to_bits(),
            "row_extent fuzz n={n} scale={scale}"
        );
        assert_eq!(
            run_neg2(&mut emu, n, scale).to_bits(),
            row_offset_neg(n, scale).to_bits(),
            "row_offset_neg(r2) fuzz n={n} scale={scale}"
        );
        assert_eq!(
            run_neg3(&mut emu, n, scale).to_bits(),
            row_offset_neg(n, scale).to_bits(),
            "row_offset_neg(r3) fuzz n={n} scale={scale}"
        );
        total += 3;
    }
    eprintln!(
        "glue_77a_77ad10_row_geom: {total} bit-exact (3 copied-byte islands: fmul×4+faddp+fchs×2)"
    );
}

// ===========================================================================================
// 0x77b8c0 — four FP regions:
//   A (0x77b8df..0x77b902): fld[0x70];fsub[0x68];fdiv[0x7c];fld[0x3f4];fadd[0x3ec];fsubp;fstp[esp]
//   B (0x77b923..0x77b93d): fld[0x6c];fsub[0x64];fdiv[0x7c];fld[0x3f0];fadd[0x3e8];fsubp;fstp[esp]
//   C (0x77b943..0x77b956): fld[0x3f0];fchs;fstp[esp]
//   D (0x77ba2d..0x77ba50): call 0x41ad70 (→st0=1.0); fmul[1024.0]; fdivr[4.0]; fstp[esp];
//                           call 0x41ae60 (→0.8·arg); fstp[ebp-4]
// A/B/C store to [esp] after interleaved object derefs → COPIED-BYTE ISLANDS (byte-verified).
// D runs as a NATIVE SLICE (the two helper calls execute as real functions; the owned fmul/fdivr
// are covered). fsub×2 + fdiv×2 + fadd×2 + fsubp×2 + fchs×1 + fmul×1 + fdivr×1.
// ===========================================================================================
#[test]
fn glue_77a_77b8c0_fill_edges() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // --- region A island.
    let cell_a = emu.alloc(4);
    let isl_a = emu.alloc(0x20);
    let mut ba = vec![0xd9u8, 0x46, 0x70]; // fld [esi+0x70]
    ba.extend_from_slice(&[0xd8, 0x66, 0x68]); // fsub [esi+0x68]
    ba.extend_from_slice(&[0xd8, 0x76, 0x7c]); // fdiv [esi+0x7c]
    ba.extend_from_slice(&[0xd9, 0x86, 0xf4, 0x03, 0x00, 0x00]); // fld [esi+0x3f4]
    ba.extend_from_slice(&[0xd8, 0x86, 0xec, 0x03, 0x00, 0x00]); // fadd [esi+0x3ec]
    ba.extend_from_slice(&[0xde, 0xe9]); // fsubp st(1),st
    append_fstp(&mut ba, cell_a);
    let isl_a_end = isl_a + ba.len() as u64;
    emu.write(isl_a, &ba);
    assert_eq!(&emu.read(isl_a, 3), &emu.read(0x77b8df, 3), "A fld != src");
    assert_eq!(
        &emu.read(isl_a + 3, 3),
        &emu.read(0x77b8e2, 3),
        "A fsub != src"
    );
    assert_eq!(
        &emu.read(isl_a + 6, 3),
        &emu.read(0x77b8f0, 3),
        "A fdiv != src"
    );
    assert_eq!(
        &emu.read(isl_a + 9, 6),
        &emu.read(0x77b8f4, 6),
        "A fld2 != src"
    );
    assert_eq!(
        &emu.read(isl_a + 15, 6),
        &emu.read(0x77b8fa, 6),
        "A fadd != src"
    );
    assert_eq!(
        &emu.read(isl_a + 21, 2),
        &emu.read(0x77b900, 2),
        "A fsubp != src"
    );

    // --- region B island.
    let cell_b = emu.alloc(4);
    let isl_b = emu.alloc(0x20);
    let mut bb = vec![0xd9u8, 0x46, 0x6c]; // fld [esi+0x6c]
    bb.extend_from_slice(&[0xd8, 0x66, 0x64]); // fsub [esi+0x64]
    bb.extend_from_slice(&[0xd8, 0x76, 0x7c]); // fdiv [esi+0x7c]
    bb.extend_from_slice(&[0xd9, 0x86, 0xf0, 0x03, 0x00, 0x00]); // fld [esi+0x3f0]
    bb.extend_from_slice(&[0xd8, 0x86, 0xe8, 0x03, 0x00, 0x00]); // fadd [esi+0x3e8]
    bb.extend_from_slice(&[0xde, 0xe9]); // fsubp st(1),st
    append_fstp(&mut bb, cell_b);
    let isl_b_end = isl_b + bb.len() as u64;
    emu.write(isl_b, &bb);
    assert_eq!(&emu.read(isl_b, 3), &emu.read(0x77b923, 3), "B fld != src");
    assert_eq!(
        &emu.read(isl_b + 3, 3),
        &emu.read(0x77b928, 3),
        "B fsub != src"
    );
    assert_eq!(
        &emu.read(isl_b + 6, 3),
        &emu.read(0x77b92c, 3),
        "B fdiv != src"
    );
    assert_eq!(
        &emu.read(isl_b + 9, 6),
        &emu.read(0x77b92f, 6),
        "B fld2 != src"
    );
    assert_eq!(
        &emu.read(isl_b + 15, 6),
        &emu.read(0x77b935, 6),
        "B fadd != src"
    );
    assert_eq!(
        &emu.read(isl_b + 21, 2),
        &emu.read(0x77b93b, 2),
        "B fsubp != src"
    );

    // --- region C island.
    let cell_c = emu.alloc(4);
    let isl_c = emu.alloc(0x20);
    let mut bc = vec![0xd9u8, 0x86, 0xf0, 0x03, 0x00, 0x00]; // fld [esi+0x3f0]
    bc.extend_from_slice(&[0xd9, 0xe0]); // fchs
    append_fstp(&mut bc, cell_c);
    let isl_c_end = isl_c + bc.len() as u64;
    emu.write(isl_c, &bc);
    assert_eq!(&emu.read(isl_c, 6), &emu.read(0x77b943, 6), "C fld != src");
    assert_eq!(
        &emu.read(isl_c + 6, 2),
        &emu.read(0x77b94b, 2),
        "C fchs != src"
    );

    // field offsets the islands read through esi.
    const F_64: u64 = 0x64;
    const F_68: u64 = 0x68;
    const F_6C: u64 = 0x6c;
    const F_70: u64 = 0x70;
    const F_7C: u64 = 0x7c;
    const F_3E8: u64 = 0x3e8;
    const F_3EC: u64 = 0x3ec;
    const F_3F0: u64 = 0x3f0;
    const F_3F4: u64 = 0x3f4;

    let run_edge = |emu: &mut Emu,
                    isl: u64,
                    end: u64,
                    cell: u64,
                    hi_off: u64,
                    lo_off: u64,
                    anc_off: u64,
                    off_off: u64,
                    trk_hi: f32,
                    trk_lo: f32,
                    scale: f32,
                    anchor: f32,
                    off: f32|
     -> f32 {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.write_u32(f.esi + hi_off, trk_hi.to_bits());
        emu.write_u32(f.esi + lo_off, trk_lo.to_bits());
        emu.write_u32(f.esi + F_7C, scale.to_bits());
        emu.write_u32(f.esi + anc_off, anchor.to_bits());
        emu.write_u32(f.esi + off_off, off.to_bits());
        emu.run(isl, end);
        emu.read_f32(cell)
    };
    let run_negc = |emu: &mut Emu, anchor: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.write_u32(f.esi + F_3F0, anchor.to_bits());
        emu.run(isl_c, isl_c_end);
        emu.read_f32(cell_c)
    };

    let mut total = 0u32;
    let probes: [(f32, f32, f32, f32, f32); 6] = [
        (1.0, 0.0, 1.0, 0.0, 0.0),
        (10.0, 2.0, 4.0, 1.5, -0.5),
        (0.0, 0.0, 1.0, 0.0, 0.0), // edge = -(anchor+off)
        (5.0, 5.0, 2.0, 3.0, 3.0), // numerator 0
        (100.0, -100.0, 8.0, 0.25, 0.25),
        (-7.5, 7.5, 0.5, -2.0, 2.0),
    ];
    for &(trk_hi, trk_lo, scale, anchor, off) in &probes {
        // region A
        assert_eq!(
            run_edge(
                &mut emu, isl_a, isl_a_end, cell_a, F_70, F_68, F_3F4, F_3EC, trk_hi, trk_lo,
                scale, anchor, off
            )
            .to_bits(),
            fill_edge(trk_hi, trk_lo, scale, anchor, off).to_bits(),
            "fill_edge(A) hi={trk_hi} lo={trk_lo} s={scale} a={anchor} o={off}"
        );
        // region B
        assert_eq!(
            run_edge(
                &mut emu, isl_b, isl_b_end, cell_b, F_6C, F_64, F_3F0, F_3E8, trk_hi, trk_lo,
                scale, anchor, off
            )
            .to_bits(),
            fill_edge(trk_hi, trk_lo, scale, anchor, off).to_bits(),
            "fill_edge(B) hi={trk_hi} lo={trk_lo} s={scale} a={anchor} o={off}"
        );
        // region C
        assert_eq!(
            run_negc(&mut emu, anchor).to_bits(),
            neg_anchor(anchor).to_bits(),
            "neg_anchor a={anchor}"
        );
        total += 3;
    }
    let mut s = 0x77b8_c000u32;
    for _ in 0..2000 {
        let trk_hi = rng(&mut s, -2000.0, 2000.0);
        let trk_lo = rng(&mut s, -2000.0, 2000.0);
        let scale = rng(&mut s, -100.0, 100.0); // can be ±, near-0 (degenerate div) included
        let anchor = rng(&mut s, -2000.0, 2000.0);
        let off = rng(&mut s, -2000.0, 2000.0);
        assert_eq!(
            run_edge(
                &mut emu, isl_a, isl_a_end, cell_a, F_70, F_68, F_3F4, F_3EC, trk_hi, trk_lo,
                scale, anchor, off
            )
            .to_bits(),
            fill_edge(trk_hi, trk_lo, scale, anchor, off).to_bits(),
            "fill_edge(A) fuzz hi={trk_hi} lo={trk_lo} s={scale} a={anchor} o={off}"
        );
        assert_eq!(
            run_edge(
                &mut emu, isl_b, isl_b_end, cell_b, F_6C, F_64, F_3F0, F_3E8, trk_hi, trk_lo,
                scale, anchor, off
            )
            .to_bits(),
            fill_edge(trk_hi, trk_lo, scale, anchor, off).to_bits(),
            "fill_edge(B) fuzz hi={trk_hi} lo={trk_lo} s={scale} a={anchor} o={off}"
        );
        assert_eq!(
            run_negc(&mut emu, anchor).to_bits(),
            neg_anchor(anchor).to_bits(),
            "neg_anchor fuzz a={anchor}"
        );
        total += 3;
    }

    // --- region D: native slice through the two helper calls (all-constant blend).
    // Sanity-pin the helper constants the slice depends on (read from the binary).
    assert_eq!(emu.read_u32(0x832a4c), ONE_BITS, "0x41ad70 const != 1.0");
    assert_eq!(emu.read_u32(0x801620), TENTH_BITS, "[0x801620] != 0.1"); // exercised by r1 above
    assert_eq!(emu.read_u32(0x7ffd68), K1024_BITS, "[0x7ffd68] != 1024.0");
    let run_d = |emu: &mut Emu| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.esi); // mov ecx,[esi+0x368] at 0x77ba47 (load only)
        emu.run(0x77ba2d, 0x77ba50); // call;fmul;fdivr;fstp[esp];call; fstp[ebp-4]
        emu.read_f32(f.ebp - 0x4)
    };
    for _ in 0..16 {
        assert_eq!(
            run_d(&mut emu).to_bits(),
            blend_const().to_bits(),
            "blend_const (region D)"
        );
        total += 1;
    }
    // also confirm TEN_BITS is the 0x77a9f0 const we used in the sibling test (cross-check).
    assert_eq!(emu.read_u32(0x801624), TEN_BITS, "[0x801624] != 10.0");

    eprintln!(
        "glue_77a_77b8c0_fill_edges: {total} bit-exact (3 islands fsub/fdiv/fadd/fsubp/fchs + native-slice fmul/fdivr blend)"
    );
}
