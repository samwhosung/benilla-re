//! Differential tests of the arithmetic of the unit-conversion and colour functions `0x788750`,
//! `0x78ab40`, `0x78bae0` and `0x78bcf0`, each run against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fxch`/`fnstsw`
//! excluded), counted in each `[addr, true_ret)`:
//!
//! * `0x788750` [.,0x788ab7 ret 4] : fsub ×2 (0x7887c4,0x788908) + fdiv ×2 (0x7887d4,0x788915)
//!   + fadd ×1 (0x7889ad) + fsub ×1 (0x7889f9)  = 6
//! * `0x78ab40` [.,0x78ada5 ret 8] : fmul ×2 (0x78abba,0x78abf2) + fdivr ×2 (0x78abc1,0x78abf9)
//!   + fsubr ×1 (0x78ad15)  = 5
//! * `0x78bae0` [.,0x78bb9c ret 0] : fmul ×3 (0x78bb62,0x78bb72,0x78bb7f)  = 3
//! * `0x78bcf0` [.,0x78bd52 ret 0] : fmul ×1 (0x78bd19)  = 1
//!
//! Diff strategy per op:
//! * **own-address slices** (`Emu::run`) over the ops whose `st0` is loaded from memory (`fld`/
//!   `fild`): 0x788750's two `(hi−lo)/cell` ratio blocks (fsub+fdiv each), its per-iter `fsub`, and
//!   0x78bae0's three `fild;fmul qword;fstp qword` channel normalizes. The verbatim source bytes
//!   read my seeded `[esi+..]`/`[ebp+..]`/`[esp]` cells.
//! * **copied-byte islands** (`fld qword [in]` seeds the call-result `st0`, then the EXACT source
//!   arith bytes — byte-verified == the source site(s), else circular — then a `fstp` I read back)
//!   for the ops whose `st0` comes from a delegated **call**: 0x788750's `fadd` (the `0x7729b0`
//!   leaf-sum), 0x78ab40's `fmul;fdivr` unit-converts and its `fsubr` size-subtract (the `0x41ad70`
//!   / vtable-getter results), 0x78bcf0's `fmul` alpha·scale (the vtable-getter result).
//!
//! Fuzz reads the LCG's HIGH bits. Each `#[test]` carries a
//! deterministic `==`-boundary probe (zeros, ±1, the byte domain `0..=255`, and exact-cancellation
//! pivots where `hi==lo` → `0.0`).
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::framexml_units::{
    acc_subtract, bounds_ratio, color_channel, size_subtract, size_sum, thumb_height_product,
    unit_convert, INV255_F64_BITS, PX1024_F32_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f32 {
    (lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)) as f32
}

// A scratch frame: a wide structure region (for [esi/edi+disp] cells up to +0x380), an ebp-window,
// and a stack. All three sit in the mapped scratch arena.
struct Env {
    strukt: u64, // base for [esi+..]/[edi+..]
    ebp: u64,
    esp: u64,
}
fn env(emu: &mut Emu) -> Env {
    let strukt = emu.alloc(0x400);
    let win = emu.alloc(0x200);
    let ebp = win + 0x100; // room for [ebp-0x100 .. ebp+0x100]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Env { strukt, ebp, esp }
}

// ===========================================================================================
// Constants — assert the owned constants against the mapped image.
// ===========================================================================================
#[test]
fn glue_geom_78b_constants() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let emu = Emu::new(&wow);
    let inv255 = u64::from_le_bytes(emu.read(0x804578, 8).try_into().unwrap());
    assert_eq!(inv255, INV255_F64_BITS, "[0x804578] 1/255 f64");
    let px1024 = u32::from_le_bytes(emu.read(0x7ffd68, 4).try_into().unwrap());
    assert_eq!(px1024, PX1024_F32_BITS, "[0x7ffd68] 1024.0 f32");
    eprintln!("glue_geom_78b_constants: 1/255 f64 + 1024.0 f32 match image");
}

// ===========================================================================================
// Row 0x788750 — the two `(hi − lo) / cell` ratio blocks (fsub+fdiv each), own-address slices.
//   block1: 0x7887be fld [esi+0x374]; fsub [esi+0x36c]; <ints>; fdiv [esi+0x7c]; fstp [ebp-0x14]
//           → slice 0x7887be..0x7887da, read [ebp-0x14].   (push ebx in the slice; esp irrelevant)
//   block2: 0x788902 fld [esi+0x378]; fsub [esi+0x370]; <ints>; fdiv [esi+0x7c]; fstp [esp]
//           → slice 0x788902..0x78891b, push ecx before fstp → result at esp_initial-4.
// ===========================================================================================
#[test]
fn glue_geom_788750_bounds_ratios() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let e = env(&mut emu);

    // block1: hi=[esi+0x374] lo=[esi+0x36c] cell=[esi+0x7c] -> [ebp-0x14]
    let run1 = |emu: &mut Emu, hi: f32, lo: f32, cell: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, e.strukt);
        emu.set_reg(RegisterX86::EBP, e.ebp);
        emu.set_reg(RegisterX86::ESP, e.esp);
        emu.write_u32(e.strukt + 0x374, hi.to_bits());
        emu.write_u32(e.strukt + 0x36c, lo.to_bits());
        emu.write_u32(e.strukt + 0x7c, cell.to_bits());
        emu.run(0x7887be, 0x7887da);
        emu.read_f32(e.ebp - 0x14)
    };
    // block2: hi=[esi+0x378] lo=[esi+0x370] cell=[esi+0x7c] -> [esp-4]
    let run2 = |emu: &mut Emu, hi: f32, lo: f32, cell: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, e.strukt);
        emu.set_reg(RegisterX86::EBP, e.ebp);
        emu.set_reg(RegisterX86::ESP, e.esp);
        emu.set_reg(RegisterX86::EDI, e.strukt); // [edi+0x24] read by an int mov in the slice
        emu.write_u32(e.strukt + 0x378, hi.to_bits());
        emu.write_u32(e.strukt + 0x370, lo.to_bits());
        emu.write_u32(e.strukt + 0x7c, cell.to_bits());
        emu.run(0x788902, 0x78891b);
        emu.read_f32(e.esp - 0x4)
    };

    let mut total = 0u32;
    let probes: [(f32, f32, f32); 8] = [
        (1.0, 0.0, 1.0),
        (0.0, 0.0, 1.0), // hi==lo -> exact 0.0
        (5.0, 5.0, 2.5), // hi==lo -> 0.0
        (100.0, 10.0, 9.0),
        (-3.0, 4.0, 0.5),
        (1024.0, 0.0, 1024.0),
        (0.5, 0.25, 0.125),
        (1e6, -1e6, 7.0),
    ];
    for &(hi, lo, cell) in &probes {
        assert_eq!(
            run1(&mut emu, hi, lo, cell).to_bits(),
            bounds_ratio(hi, lo, cell).to_bits(),
            "bounds_ratio block1 hi={hi} lo={lo} cell={cell}"
        );
        assert_eq!(
            run2(&mut emu, hi, lo, cell).to_bits(),
            bounds_ratio(hi, lo, cell).to_bits(),
            "bounds_ratio block2 hi={hi} lo={lo} cell={cell}"
        );
        total += 2;
    }
    let mut s = 0x7887_5000u32;
    for _ in 0..3000 {
        let hi = rngf(&mut s, -1000.0, 1000.0);
        let lo = rngf(&mut s, -1000.0, 1000.0);
        let mut cell = rngf(&mut s, -100.0, 100.0);
        if cell == 0.0 {
            cell = 1.0;
        }
        assert_eq!(
            run1(&mut emu, hi, lo, cell).to_bits(),
            bounds_ratio(hi, lo, cell).to_bits(),
            "bounds_ratio block1 fuzz hi={hi} lo={lo} cell={cell}"
        );
        assert_eq!(
            run2(&mut emu, hi, lo, cell).to_bits(),
            bounds_ratio(hi, lo, cell).to_bits(),
            "bounds_ratio block2 fuzz hi={hi} lo={lo} cell={cell}"
        );
        total += 2;
    }
    eprintln!("glue_geom_788750_bounds_ratios: {total} bit-exact (2 slices: fsub+fdiv each)");
}

// ===========================================================================================
// Row 0x788750 — the per-iteration `fsub` (own-address slice) and the `0x7729b0` leaf `fadd`
// (copied-byte island, since st0 is a call result).
//   fsub: 0x7889f3 fld [ebp-0x14]; <ints>; fsub [ebp+8]; fstp [ebp-0x14]  → slice ..0x788a03.
//   fadd: island `fld qword [in]; <source d8 45 08>; fstp dword [out]`, byte-verified == 0x7889ad,
//         ebp set so [ebp+8] = my cell.
// ===========================================================================================
#[test]
fn glue_geom_788750_acc_and_sum() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let e = env(&mut emu);

    // ---- (A) per-iter fsub: own-address slice ----
    let run_sub = |emu: &mut Emu, acc: f32, total: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, e.ebp);
        emu.set_reg(RegisterX86::ESP, e.esp);
        emu.write_u32(e.ebp - 0x14, acc.to_bits()); // [ebp-0x14] = acc
        emu.write_u32(e.ebp + 0x8, total.to_bits()); // [ebp+8] = total
        emu.run(0x7889f3, 0x788a03);
        emu.read_f32(e.ebp - 0x14)
    };

    // ---- (B) fadd island ----
    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xddu8, 0x05]; // fld qword [in_cell]  (seed st0 = a)
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let fadd_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x45, 0x08]); // copied source `fadd dword [ebp+8]`
    code.push(0xd9); // fstp dword [out_cell] = D9 1D <disp32>
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        emu.read(isl + fadd_off, 3),
        emu.read(0x7889ad, 3),
        "fadd island bytes != source @ 0x7889ad"
    );
    let run_sum = |emu: &mut Emu, a: f32, b: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, e.ebp);
        emu.write(in_cell, &f64::from(a).to_bits().to_le_bytes()); // st0 = a (the 0x7729b0 result)
        emu.write_u32(e.ebp + 0x8, b.to_bits()); // [ebp+8] = b (the stored 0x766750 f32)
        emu.run(isl, isl_end);
        emu.read_f32(out_cell)
    };

    let mut total = 0u32;
    let probes: [(f32, f32); 7] = [
        (0.0, 0.0),
        (1.0, -1.0), // exact cancellation -> 0.0
        (1.0, 2.0),
        (-5.0, 5.0),
        (0.1, 0.2),
        (1e7, 1.0),
        (123.456, -0.001),
    ];
    for &(a, b) in &probes {
        assert_eq!(
            run_sub(&mut emu, a, b).to_bits(),
            acc_subtract(a, b).to_bits(),
            "acc_subtract a={a} b={b}"
        );
        assert_eq!(
            run_sum(&mut emu, a, b).to_bits(),
            size_sum(a, b).to_bits(),
            "size_sum a={a} b={b}"
        );
        total += 2;
    }
    let mut s = 0x7889_0000u32;
    for _ in 0..3000 {
        let a = rngf(&mut s, -1e5, 1e5);
        let b = rngf(&mut s, -1e5, 1e5);
        assert_eq!(
            run_sub(&mut emu, a, b).to_bits(),
            acc_subtract(a, b).to_bits(),
            "acc_subtract fuzz a={a} b={b}"
        );
        assert_eq!(
            run_sum(&mut emu, a, b).to_bits(),
            size_sum(a, b).to_bits(),
            "size_sum fuzz a={a} b={b}"
        );
        total += 2;
    }
    eprintln!(
        "glue_geom_788750_acc_and_sum: {total} bit-exact (fsub slice + fadd island == 0x7889ad)"
    );
}

// ===========================================================================================
// Row 0x78ab40 — width/height unit-convert (`fmul [1024]; fdivr [cell]`) + the `fsubr` size
// subtract, all via copied-byte islands (st0 is a call result each time).
//   width  fdivr byte:  d8 7d f0  (== 0x78abc1, [ebp-0x10])
//   height fdivr byte:  d8 7d ec  (== 0x78abf9, [ebp-0x14])
//   fmul byte (both):   d8 0d 68 fd 7f 00  (== 0x78abba & 0x78abf2, abs [0x7ffd68])
//   fsubr byte:         d8 af 4c 03 00 00  (== 0x78ad15, [edi+0x34c])
// ===========================================================================================
#[test]
fn glue_geom_78ab40_unit_convert_and_subtract() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let e = env(&mut emu);

    // ---- unit-convert islands (one per fdivr disp; the fmul is identical, byte-verified to both) ----
    // island: fld qword [in_px] (st0=px); fmul dword [0x7ffd68]; fdivr dword [ebp-disp]; fstp dword [out]
    let in_px = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let build_uc = |emu: &mut Emu, fdivr_modrm_disp: [u8; 3]| -> (u64, u64, u64) {
        let isl = emu.alloc(0x20);
        let mut code = vec![0xddu8, 0x05];
        code.extend_from_slice(&(in_px as u32).to_le_bytes());
        let fmul_off = code.len() as u64;
        code.extend_from_slice(&[0xd8, 0x0d, 0x68, 0xfd, 0x7f, 0x00]); // fmul dword [0x7ffd68]
        let fdivr_off = code.len() as u64;
        code.extend_from_slice(&fdivr_modrm_disp); // fdivr dword [ebp-disp]
        code.push(0xd9); // fstp dword [out]
        code.push(0x1d);
        code.extend_from_slice(&(out_cell as u32).to_le_bytes());
        emu.write(isl, &code);
        (isl, isl + fmul_off, isl + fdivr_off)
    };
    let (isl_w, fmul_w, fdivr_w) = build_uc(&mut emu, [0xd8, 0x7d, 0xf0]); // [ebp-0x10]
    let (isl_h, fmul_h, fdivr_h) = build_uc(&mut emu, [0xd8, 0x7d, 0xec]); // [ebp-0x14]
                                                                           // byte-verify fmul == both source sites; each fdivr == its source site.
    for src in [0x78abbau64, 0x78abf2] {
        assert_eq!(
            emu.read(fmul_w, 6),
            emu.read(src, 6),
            "uc fmul != src @ {src:#x}"
        );
    }
    assert_eq!(
        emu.read(fmul_h, 6),
        emu.read(0x78abf2, 6),
        "uc fmul(h) != src"
    );
    assert_eq!(
        emu.read(fdivr_w, 3),
        emu.read(0x78abc1, 3),
        "uc fdivr(w) != 0x78abc1"
    );
    assert_eq!(
        emu.read(fdivr_h, 3),
        emu.read(0x78abf9, 3),
        "uc fdivr(h) != 0x78abf9"
    );

    // width fdivr reads [ebp-0x10]; height reads [ebp-0x14]. Set ebp; write dim there.
    // island layout = fld qword(6) + fmul dword(6) + fdivr dword(3) + fstp dword(6) = 21 bytes.
    let run_uc = |emu: &mut Emu, isl: u64, disp: u64, px: f32, dim: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, e.ebp);
        emu.write(in_px, &f64::from(px).to_bits().to_le_bytes());
        emu.write_u32(e.ebp.wrapping_sub(disp), dim.to_bits());
        emu.run(isl, isl + 21);
        emu.read_f32(out_cell)
    };

    let mut total = 0u32;
    let probes: [(f32, f32); 8] = [
        (1.0, 0.0), // dim 0 -> 0.0
        (1.0, 1.0),
        (2.0, 100.0),
        (0.5, 50.0),
        (16.0, 1.0),
        (-1.0, 1.0), // negative px
        (1024.0, 1.0),
        (3.0, -2.0),
    ];
    for &(px, dim) in &probes {
        assert_eq!(
            run_uc(&mut emu, isl_w, 0x10, px, dim).to_bits(),
            unit_convert(px, dim).to_bits(),
            "unit_convert(width) px={px} dim={dim}"
        );
        assert_eq!(
            run_uc(&mut emu, isl_h, 0x14, px, dim).to_bits(),
            unit_convert(px, dim).to_bits(),
            "unit_convert(height) px={px} dim={dim}"
        );
        total += 2;
    }
    let mut s = 0x78ab_4000u32;
    for _ in 0..3000 {
        let mut px = rngf(&mut s, -200.0, 200.0);
        if px == 0.0 {
            px = 1.0;
        }
        let dim = rngf(&mut s, -2000.0, 2000.0);
        assert_eq!(
            run_uc(&mut emu, isl_w, 0x10, px, dim).to_bits(),
            unit_convert(px, dim).to_bits(),
            "unit_convert(width) fuzz px={px} dim={dim}"
        );
        assert_eq!(
            run_uc(&mut emu, isl_h, 0x14, px, dim).to_bits(),
            unit_convert(px, dim).to_bits(),
            "unit_convert(height) fuzz px={px} dim={dim}"
        );
        total += 2;
    }

    // ---- fsubr size-subtract island ----
    // island: fld qword [in_s] (st0=s); fsubr dword [edi+0x34c]; fstp dword [out2]
    let in_s = emu.alloc(8);
    let out2 = emu.alloc(8);
    let isl_sub = emu.alloc(0x20);
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(in_s as u32).to_le_bytes());
    let fsubr_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0xaf, 0x4c, 0x03, 0x00, 0x00]); // fsubr dword [edi+0x34c]
    code.push(0xd9); // fstp dword [out2]
    code.push(0x1d);
    code.extend_from_slice(&(out2 as u32).to_le_bytes());
    let isl_sub_end = isl_sub + code.len() as u64;
    emu.write(isl_sub, &code);
    assert_eq!(
        emu.read(isl_sub + fsubr_off, 6),
        emu.read(0x78ad15, 6),
        "fsubr island bytes != source @ 0x78ad15"
    );
    let run_sub = |emu: &mut Emu, field: f32, s: f32| -> f32 {
        emu.set_reg(RegisterX86::EDI, e.strukt);
        emu.write(in_s, &f64::from(s).to_bits().to_le_bytes()); // st0 = s (the getter result)
        emu.write_u32(e.strukt + 0x34c, field.to_bits()); // [edi+0x34c] = field
        emu.run(isl_sub, isl_sub_end);
        emu.read_f32(out2)
    };
    let sprobes: [(f32, f32); 6] = [
        (0.0, 0.0),
        (5.0, 5.0), // exact cancellation -> 0.0
        (10.0, 3.0),
        (-2.0, 4.0),
        (1e6, 1.0),
        (0.25, 0.5),
    ];
    for &(field, sv) in &sprobes {
        assert_eq!(
            run_sub(&mut emu, field, sv).to_bits(),
            size_subtract(field, sv).to_bits(),
            "size_subtract field={field} s={sv}"
        );
        total += 1;
    }
    for _ in 0..2000 {
        let field = rngf(&mut s, -1e4, 1e4);
        let sv = rngf(&mut s, -1e4, 1e4);
        assert_eq!(
            run_sub(&mut emu, field, sv).to_bits(),
            size_subtract(field, sv).to_bits(),
            "size_subtract fuzz field={field} s={sv}"
        );
        total += 1;
    }
    eprintln!(
        "glue_geom_78ab40_unit_convert_and_subtract: {total} bit-exact (fmul/fdivr islands == 0x78abba/c1/f2/f9; fsubr island == 0x78ad15)"
    );
}

// ===========================================================================================
// Row 0x78bae0 — three packed-color channel normalizes, own-address slices (st0 from `fild`).
//   ch1: 0x78bb5c fild [ebp-0x8]; fmul qword [0x804578]; fstp qword [esp+0x10]  → slice ..0x78bb6c
//   ch2: 0x78bb6c fild [ebp-0x8]; fmul qword [0x804578]; fstp qword [esp+0x8]   → slice ..0x78bb7c
//   ch3: 0x78bb7c fild [ebp-0x8]; fmul qword [0x804578]; fstp qword [esp]       → slice ..0x78bb88
// (ch2/ch3's slices contain a `mov [ebp-0x8],eax` after the fild — harmless to st0 / our fstp.)
// ===========================================================================================
#[test]
fn glue_geom_78bae0_color_channels() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let e = env(&mut emu);

    // (start, end, esp-result-offset) per channel.
    let chans = [
        (0x78bb5cu64, 0x78bb6cu64, 0x10u64),
        (0x78bb6c, 0x78bb7c, 0x08),
        (0x78bb7c, 0x78bb88, 0x00),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, esp_off: u64, byte: u8| -> f64 {
        emu.set_reg(RegisterX86::EBP, e.ebp);
        emu.set_reg(RegisterX86::ESP, e.esp);
        emu.write_u32(e.ebp - 0x8, u32::from(byte)); // [ebp-0x8] = byte (dword, fild dword)
        emu.run(start, end);
        let b = emu.read(e.esp + esp_off, 8); // fstp qword
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    for (ci, &(start, end, esp_off)) in chans.iter().enumerate() {
        for byte in 0u32..=255 {
            assert_eq!(
                run(&mut emu, start, end, esp_off, byte as u8).to_bits(),
                color_channel(byte as u8).to_bits(),
                "color_channel ch#{ci} @ {start:#x} byte={byte}"
            );
            total += 1;
        }
    }
    eprintln!(
        "glue_geom_78bae0_color_channels: {total} bit-exact (3 slices: fild dword; fmul qword [1/255 f64]; fstp qword)"
    );
}

// ===========================================================================================
// Row 0x78bcf0 — the alpha·scale `fmul [esi+0x330]`, copied-byte island (st0 from vtable getter).
//   island: fld qword [in_a] (st0=a); fmul dword [esi+0x330]; fstp dword [out], byte-verified ==
//   0x78bd19, esi set so [esi+0x330] = my scale cell.
// ===========================================================================================
#[test]
fn glue_geom_78bcf0_alpha_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let e = env(&mut emu);

    let in_a = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xddu8, 0x05]; // fld qword [in_a]
    code.extend_from_slice(&(in_a as u32).to_le_bytes());
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x8e, 0x30, 0x03, 0x00, 0x00]); // fmul dword [esi+0x330]
    code.push(0xd9); // fstp dword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        emu.read(isl + fmul_off, 6),
        emu.read(0x78bd19, 6),
        "fmul island bytes != source @ 0x78bd19"
    );
    let run = |emu: &mut Emu, a: f32, scale: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, e.strukt);
        emu.write(in_a, &f64::from(a).to_bits().to_le_bytes()); // st0 = a (getter result)
        emu.write_u32(e.strukt + 0x330, scale.to_bits()); // [esi+0x330] = scale
        emu.run(isl, isl_end);
        emu.read_f32(out_cell)
    };

    let mut total = 0u32;
    let probes: [(f32, f32); 8] = [
        (0.0, 1.0),
        (1.0, 1.0),
        (1.0, 0.0),
        (0.5, 0.5),
        (255.0, 1.0 / 255.0),
        (-1.0, 2.0),
        (123.0, 0.001),
        (2.0, -3.5),
    ];
    for &(a, scale) in &probes {
        assert_eq!(
            run(&mut emu, a, scale).to_bits(),
            thumb_height_product(a, scale).to_bits(),
            "thumb_height_product a={a} scale={scale}"
        );
        total += 1;
    }
    let mut s = 0x78bc_f000u32;
    for _ in 0..3000 {
        let a = rngf(&mut s, -100.0, 100.0);
        let scale = rngf(&mut s, -10.0, 10.0);
        assert_eq!(
            run(&mut emu, a, scale).to_bits(),
            thumb_height_product(a, scale).to_bits(),
            "thumb_height_product fuzz a={a} scale={scale}"
        );
        total += 1;
    }
    eprintln!("glue_geom_78bcf0_alpha_scale: {total} bit-exact (fmul island == 0x78bd19)");
}
