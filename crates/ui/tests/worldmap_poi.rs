//! Differential tests of the arithmetic of the map-point and taxi functions: the percent byte of
//! `SendChatMessage` (`0x49f1e0`), the point transform of `BuildMapLists` (`0x4a5d00`), and the
//! bound normalisation and route cost of `OpenTaxiMap` (`0x4db710`) and `RelaxTaxiRoute`
//! (`0x4dbce0`), each run against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fxch`/`fnstsw`
//! excluded), counted in each `[addr, true_ret)`:
//!
//! * `0x49f1e0` SendChatMessage [.,0x49f794 ret]: `fmul dword [0.01 f32]` ×1
//! * `0x4a5d00` worldmap-POI     [.,0x4a6383 ret]: 1 block (0x4a5f42..0x4a5fe4) =
//!   `fmul[0x806534]`×1, `fsub st,st(1)`×1, `fadd [eax+0x18]`×1, `fsub st,st(2)`×1,
//!   `fadd [eax+0x1c]`×1  (prelude = 5)  +  per-output {`fmul[eax+0x20]`, `fadd`, `fmul`} ×4
//!   = 12  → 17 total
//! * `0x4db710` region-bound    [.,0x4dbac0 ret 4]: 2 bound `fsub` + 2 ratios {`fsub`,`fsub`,`fdivp`}
//!   = 2 + 6 = 8
//! * `0x4dbce0` region-cost     [.,0x4dc002 ret 4]: `fadd dword [edi+0xbb4f2c]` ×1
//!
//! Diff strategy per op:
//! * `0x49f1e0` — own-address **slice** (`Emu::run`) 0x49f63f..0x49f651 (`fild;fmul;fstp dword`),
//!   feeding the int at `[ebp-0x8]`, reading the f32 at `[esp]`. (1 fmul.)
//! * `0x4a5d00` — own-address **slice** 0x4a5f42..0x4a5fe4 (the whole transform block), feeding the
//!   POI record at `eax`, `ebx=0`, `[0xb6e668]`=out base; reading the four f32 edges. (all 17.)
//! * `0x4db710` ratios — own-address **slice** 0x4db953..0x4db99b (both `fsub;fsub;fdivp` blocks),
//!   feeding the record at `ecx`, the globals `0xb73a30/34/2c/28`, `[0xbb4aa8]=0`; reading the two
//!   f32 ratios at `0xbb4f14`/`0xbb4f18`. (6 ops.)
//! * `0x4db710` bound subtracts — two copied-byte **islands** `<src fld><src fsub>; fstp qword [out]`,
//!   the `fld`+`fsub` bytes byte-verified == the two source sites (0x4db839/0x4db86a). (2 fsub.)
//! * `0x4dbce0` — a copied-byte **island** `fld qword [in]; <src fadd>; fstp qword [out]`, the `fadd`
//!   bytes byte-verified == the source site (0x4dbe06), seeding `edi=0` so `[edi+0xbb4f2c]` is the
//!   written field. (1 fadd.)
//!
//! Fuzz reads the LCG's HIGH bits. Each test carries a deterministic
//! `==threshold` boundary probe (the percent integer pivots, the transform zero/identity cases, the
//! ratio `den==0` and `num==0` boundaries, the cost exact-cancel).
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::worldmap_poi::{
    bound_diff, bounds_norm, cost_add_f64, percent_byte, poi_transform, INV100_F32_BITS,
    K806524_F32_BITS, K806528_F32_BITS, K80652C_F32_BITS, K806530_F32_BITS, K806534_F32_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}
fn rngf32(s: &mut u32, lo: f64, hi: f64) -> f32 {
    rngf(s, lo, hi) as f32
}

// ===========================================================================================
// Sanity: assert each owned f32 constant the module hard-codes equals the mapped-image dword (a
// stale literal can't pass; the constants ride the slices, but pin them explicitly).
// ===========================================================================================
#[test]
fn glue_geom_49f_constants_match_image() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let emu = Emu::new(&wow);
    let rd = |va: u64| u32::from_le_bytes(emu.read(va, 4).try_into().unwrap());
    assert_eq!(rd(0x8029d0), INV100_F32_BITS, "0x8029d0 (1/100 f32)");
    assert_eq!(rd(0x806534), K806534_F32_BITS, "0x806534");
    assert_eq!(rd(0x806530), K806530_F32_BITS, "0x806530");
    assert_eq!(rd(0x80652c), K80652C_F32_BITS, "0x80652c");
    assert_eq!(rd(0x806528), K806528_F32_BITS, "0x806528");
    assert_eq!(rd(0x806524), K806524_F32_BITS, "0x806524");
    // 0x7ffd74 = 0.0 (the bound-diff `fcomp` operand; a compare, documented for context).
    assert_eq!(rd(0x7ffd74), 0x0000_0000, "0x7ffd74 (0.0)");
    eprintln!("glue_geom_49f_constants_match_image: all owned f32 constants bit-exact vs image");
}

// ===========================================================================================
// Row 0x49f1e0 SendChatMessage — slice 0x49f63f..0x49f651:
//   fild dword [ebp-0x8]; (lea ecx); fmul dword [0x8029d0]=0.01(f32); fstp dword [esp]
// fmul @0x49f648 (×1). Feed the clamped byte (zero-extended) at [ebp-0x8], read the f32 at [esp].
// ===========================================================================================
#[test]
fn glue_geom_49f1e0_percent_byte() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    let run = |emu: &mut Emu, byte: u8| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(ebp - 0x8, u32::from(byte)); // movzx-style zero-extended clamped byte
        emu.run(0x49f63f, 0x49f651);
        emu.read_f32(esp) // fstp dword [esp]
    };

    let mut total = 0u32;
    // the source clamps [edx+0x1d] to <= 0x64 (100), so the live domain is 0..=100; but the
    // arithmetic is domain-agnostic — sweep the whole byte range plus the integer pivots.
    for byte in 0u32..=255 {
        assert_eq!(
            run(&mut emu, byte as u8).to_bits(),
            percent_byte(byte as u8).to_bits(),
            "percent_byte byte={byte}"
        );
        total += 1;
    }
    // `==threshold` probe: 100 (the clamp ceiling) -> 100·0.01 = 1.0 exactly? (f32 0.01 ≠ 1/100, so
    // probe the exact bit pattern the binary produces — the assert above already covers it, but pin
    // the clamp ceiling explicitly).
    assert_eq!(
        run(&mut emu, 100).to_bits(),
        percent_byte(100).to_bits(),
        "percent_byte ceiling 100"
    );
    total += 1;
    eprintln!("glue_geom_49f1e0_percent_byte: {total} bit-exact (fild dword; fmul dword [0.01 f32]; fstp dword)");
}

// ===========================================================================================
// Row 0x4a5d00 worldmap-POI transform — slice 0x4a5f42..0x4a5fe4 (the whole x87 block, 17 FP-arith
// ops). eax = POI record; ebx = 0; [0xb6e668] = out base (ecx/edx reload it inside). Fields written
// at the record; the four f32 edges read at out+0x10018/0x10020/0x10014/0x1001c.
// ===========================================================================================
#[test]
fn glue_geom_4a5d00_poi_transform() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let record = emu.alloc(0x40); // POI record (fields up to +0x20)
    let out = emu.alloc(0x10030); // store base + 0x10020 + 8
    let frame = emu.alloc(0x100); // ebp frame: [ebp-0xc] (l10 spill), [ebp-0x18] (int scratch)
    let ebp = frame + 0x80;

    let run = |emu: &mut Emu,
               scale: f32,
               bias_x: f32,
               bias_y: f32,
               ix0: i32,
               iy0: i32,
               ix1: i32,
               iy1: i32|
     -> (f32, f32, f32, f32) {
        emu.set_reg(RegisterX86::EAX, record);
        emu.set_reg(RegisterX86::EBX, 0);
        emu.set_reg(RegisterX86::EBP, ebp); // [ebp-0xc] l10 spill, [ebp-0x18] int scratch
        emu.write_u32(0xb6e668, out as u32); // ecx/edx <- [0xb6e668] inside the block
        emu.write_u32(record + 0x20, scale.to_bits());
        emu.write_u32(record + 0x18, bias_x.to_bits());
        emu.write_u32(record + 0x1c, bias_y.to_bits());
        emu.write_u32(record + 0x8, ix0 as u32);
        emu.write_u32(record + 0xc, ix1 as u32);
        emu.write_u32(record + 0x10, iy0 as u32);
        emu.write_u32(record + 0x14, iy1 as u32);
        emu.run(0x4a5f42, 0x4a5fe4);
        (
            emu.read_f32(out + 0x10018),
            emu.read_f32(out + 0x10020),
            emu.read_f32(out + 0x10014),
            emu.read_f32(out + 0x1001c),
        )
    };

    let check = |emu: &mut Emu,
                 total: &mut u32,
                 sc: f32,
                 bx: f32,
                 by: f32,
                 ix0: i32,
                 iy0: i32,
                 ix1: i32,
                 iy1: i32| {
        let want = poi_transform(sc, bx, by, ix0, iy0, ix1, iy1);
        let got = run(emu, sc, bx, by, ix0, iy0, ix1, iy1);
        assert_eq!(
            (
                got.0.to_bits(),
                got.1.to_bits(),
                got.2.to_bits(),
                got.3.to_bits()
            ),
            (
                want.0.to_bits(),
                want.1.to_bits(),
                want.2.to_bits(),
                want.3.to_bits()
            ),
            "poi_transform sc={sc} bx={bx} by={by} ix0={ix0} iy0={iy0} ix1={ix1} iy1={iy1}"
        );
        *total += 1;
    };

    let mut total = 0u32;
    // `==threshold` / identity probes: zero scale, zero bias, identity-ish coordinates.
    check(&mut emu, &mut total, 0.0, 0.0, 0.0, 0, 0, 0, 0); // all-zero
    check(&mut emu, &mut total, 1.0, 0.0, 0.0, 0, 0, 0, 0); // scale 1, zero coords
    check(&mut emu, &mut total, 1.0, 32.0, 31.3125, 0, 0, 0, 0); // bias = the X/Y origins exactly
    check(&mut emu, &mut total, 0.5, 10.0, 20.0, 100, 200, 99, 199);
    check(&mut emu, &mut total, 2.0, -5.0, -7.5, -3, -9, -3, -9);

    // representative grid of scales/biases/coords.
    let scales = [0.0f32, 0.25, 0.5, 1.0, 2.5, 1.0 / 3.0];
    let biases = [0.0f32, 1.0, 32.0, -16.5];
    let ints = [0i32, 1, 7, 63, 255, 1024, -1, -8];
    for &sc in &scales {
        for &b in &biases {
            for &iv in &ints {
                check(&mut emu, &mut total, sc, b, b, iv, iv, iv, iv);
            }
        }
    }
    // fuzz: random scale/bias (f32) + random ints.
    let mut s = 0x4a5d_0000u32;
    for _ in 0..3000 {
        let sc = rngf32(&mut s, -8.0, 8.0);
        let bx = rngf32(&mut s, -64.0, 64.0);
        let by = rngf32(&mut s, -64.0, 64.0);
        let ix0 = (next(&mut s) >> 8) as i32 - 0x800000;
        let iy0 = (next(&mut s) >> 8) as i32 - 0x800000;
        let ix1 = (next(&mut s) >> 8) as i32 - 0x800000;
        let iy1 = (next(&mut s) >> 8) as i32 - 0x800000;
        check(&mut emu, &mut total, sc, bx, by, ix0, iy0, ix1, iy1);
    }
    eprintln!("glue_geom_4a5d00_poi_transform: {total} bit-exact (block 0x4a5f42..0x4a5fe4 = all 17 FP-arith ops)");
}

// ===========================================================================================
// Row 0x4db710 (B) normalized ratios — slice 0x4db953..0x4db99b (both `fsub;fsub;fdivp` blocks).
//   ecx = POI record; globals 0xb73a30(hi_x)/0xb73a34(hi_y)/0xb73a2c(lo_y)/0xb73a28(lo_x);
//   [0xbb4aa8] = 0 (=> eax=0). Read f32 ratios at 0xbb4f14 (v) and 0xbb4f18 (u).
//     ratio_v = (hi_x − coord_v[ecx+0xc]) / (hi_y − lo_y)
//     ratio_u = (coord_u[ecx+8] − lo_y)   / (hi_x − lo_x)
// fsub@0x4db967, fsub@0x4db970, fdivp@0x4db976; fsub@0x4db981, fsub@0x4db98d, fdivp@0x4db993 (×6).
// ===========================================================================================
#[test]
fn glue_geom_4db710_bounds_norm() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let record = emu.alloc(0x40); // [ecx+8]=coord_u, [ecx+0xc]=coord_v (record's [eax+...] write at +0x10 by mov; harmless)

    let run = |emu: &mut Emu,
               hi_x: f32,
               hi_y: f32,
               lo_y: f32,
               lo_x: f32,
               coord_u: f32,
               coord_v: f32|
     -> (f32, f32) {
        emu.set_reg(RegisterX86::ECX, record);
        emu.write_u32(0xbb4aa8, 0); // eax <- [0xbb4aa8] => 0; stores at absolute 0xbb4f14/18
        emu.write_u32(0xb73a30, hi_x.to_bits());
        emu.write_u32(0xb73a34, hi_y.to_bits());
        emu.write_u32(0xb73a2c, lo_y.to_bits());
        emu.write_u32(0xb73a28, lo_x.to_bits());
        emu.write_u32(record + 0x8, coord_u.to_bits());
        emu.write_u32(record + 0xc, coord_v.to_bits());
        emu.run(0x4db953, 0x4db99b);
        (emu.read_f32(0xbb4f14), emu.read_f32(0xbb4f18))
    };

    let check =
        |emu: &mut Emu, total: &mut u32, hx: f32, hy: f32, ly: f32, lx: f32, cu: f32, cv: f32| {
            let want_v = bounds_norm(hx, cv, hy, ly); // (hi_x − coord_v)/(hi_y − lo_y)
            let want_u = bounds_norm(cu, ly, hx, lx); // (coord_u − lo_y)/(hi_x − lo_x)
            let (gv, gu) = run(emu, hx, hy, ly, lx, cu, cv);
            assert_eq!(
                gv.to_bits(),
                want_v.to_bits(),
                "ratio_v hx={hx} hy={hy} ly={ly} lx={lx} cv={cv}"
            );
            assert_eq!(
                gu.to_bits(),
                want_u.to_bits(),
                "ratio_u hx={hx} lx={lx} ly={ly} cu={cu}"
            );
            *total += 1;
        };

    let mut total = 0u32;
    // `==threshold` probes: num==0 (coord on a bound) and a wide den. (den==0 would NaN/inf — the
    // source guards it away via the (A) `!=0` checks, but the bare arithmetic still matches; probe a
    // near-degenerate den, not exact 0, to keep a defined IEEE value both sides agree on.)
    check(&mut emu, &mut total, 64.0, 48.0, 16.0, 8.0, 8.0, 64.0); // coord on a bound -> num 0
    check(&mut emu, &mut total, 64.0, 48.0, 16.0, 8.0, 36.0, 32.0);
    check(&mut emu, &mut total, 100.0, 100.0, 0.0, 0.0, 50.0, 50.0); // den = 100 exactly
    check(&mut emu, &mut total, 1.0, 1.0, 0.0, 0.0, 0.5, 0.5);
    check(&mut emu, &mut total, 64.0, 48.0, 16.000001, 8.0, 36.0, 32.0); // tiny den_v

    let mut s = 0x4db7_1000u32;
    for _ in 0..4000 {
        let hx = rngf32(&mut s, -1000.0, 1000.0);
        let hy = rngf32(&mut s, -1000.0, 1000.0);
        let ly = rngf32(&mut s, -1000.0, 1000.0);
        let lx = rngf32(&mut s, -1000.0, 1000.0);
        let cu = rngf32(&mut s, -1000.0, 1000.0);
        let cv = rngf32(&mut s, -1000.0, 1000.0);
        check(&mut emu, &mut total, hx, hy, ly, lx, cu, cv);
    }
    eprintln!("glue_geom_4db710_bounds_norm: {total} bit-exact (slice 0x4db953..0x4db99b = 4 fsub + 2 fdivp)");
}

// ===========================================================================================
// Row 0x4db710 (A) bound subtracts — two copied-byte islands capturing each `(hi − lo)` before its
// `fcomp`. Island = <src fld dword [ebp-N]><src fsub dword [ebp-M]>; fstp qword [out]. EBP set to a
// scratch frame; the f32 operands written at [ebp-N]/[ebp-M]. The fld+fsub bytes are byte-verified
// == the two source sites: 0x4db839 (`d9 45 ec` fld [ebp-0x14]; `d8 65 e4` fsub [ebp-0x1c]) and
// 0x4db86a (`d9 45 e8` fld [ebp-0x18]; `d8 65 e0` fsub [ebp-0x20]). fsub @0x4db841, 0x4db86d (×2).
// ===========================================================================================
#[test]
fn glue_geom_4db710_bound_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100);
    let ebp = frame + 0x80; // [ebp-0x14]/[ebp-0x1c]/[ebp-0x18]/[ebp-0x20] all inside `frame`
    let out = emu.alloc(8);

    // Site 1: fld [ebp-0x14]=hi (0x4db839, `d9 45 ec`), fsub [ebp-0x1c]=lo (0x4db841, `d8 65 e4`).
    // In the source the fld and fsub are NOT contiguous (an interleaved `mov` sits between), so
    // byte-verify each opcode against its OWN source address.
    let isl1 = emu.alloc(0x20);
    let mut c1 = vec![0xd9u8, 0x45, 0xec]; // copied fld dword [ebp-0x14]
    let fsub1_off = c1.len() as u64;
    c1.extend_from_slice(&[0xd8, 0x65, 0xe4]); // copied fsub dword [ebp-0x1c]
    c1.push(0xdd); // fstp qword [out] = DD 1D <disp32>
    c1.push(0x1d);
    c1.extend_from_slice(&(out as u32).to_le_bytes());
    let isl1_end = isl1 + c1.len() as u64;
    emu.write(isl1, &c1);
    assert_eq!(
        emu.read(isl1, 3),
        emu.read(0x4db839, 3),
        "bound_diff island#1 fld != source @0x4db839"
    );
    assert_eq!(
        emu.read(isl1 + fsub1_off, 3),
        emu.read(0x4db841, 3),
        "bound_diff island#1 fsub != source @0x4db841"
    );

    // Site 2: fld [ebp-0x18]=hi (0x4db86a, `d9 45 e8`), fsub [ebp-0x20]=lo (0x4db86d, `d8 65 e0`).
    let isl2 = emu.alloc(0x20);
    let mut c2 = vec![0xd9u8, 0x45, 0xe8]; // copied fld dword [ebp-0x18]
    let fsub2_off = c2.len() as u64;
    c2.extend_from_slice(&[0xd8, 0x65, 0xe0]); // copied fsub dword [ebp-0x20]
    c2.push(0xdd);
    c2.push(0x1d);
    c2.extend_from_slice(&(out as u32).to_le_bytes());
    let isl2_end = isl2 + c2.len() as u64;
    emu.write(isl2, &c2);
    assert_eq!(
        emu.read(isl2, 3),
        emu.read(0x4db86a, 3),
        "bound_diff island#2 fld != source @0x4db86a"
    );
    assert_eq!(
        emu.read(isl2 + fsub2_off, 3),
        emu.read(0x4db86d, 3),
        "bound_diff island#2 fsub != source @0x4db86d"
    );

    let run1 = |emu: &mut Emu, hi: f32, lo: f32| -> f64 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.write_u32(ebp - 0x14, hi.to_bits());
        emu.write_u32(ebp - 0x1c, lo.to_bits());
        emu.run(isl1, isl1_end);
        f64::from_bits(u64::from_le_bytes(emu.read(out, 8).try_into().unwrap()))
    };
    let run2 = |emu: &mut Emu, hi: f32, lo: f32| -> f64 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.write_u32(ebp - 0x18, hi.to_bits());
        emu.write_u32(ebp - 0x20, lo.to_bits());
        emu.run(isl2, isl2_end);
        f64::from_bits(u64::from_le_bytes(emu.read(out, 8).try_into().unwrap()))
    };

    let check = |emu: &mut Emu, total: &mut u32, hi: f32, lo: f32| {
        let want = bound_diff(hi, lo).to_bits();
        assert_eq!(
            run1(emu, hi, lo).to_bits(),
            want,
            "bound_diff#1 hi={hi} lo={lo}"
        );
        assert_eq!(
            run2(emu, hi, lo).to_bits(),
            want,
            "bound_diff#2 hi={hi} lo={lo}"
        );
        *total += 2;
    };

    let mut total = 0u32;
    // `==threshold` probe: hi == lo => 0.0 exactly (the `!= 0.0` boundary the source's fcomp tests).
    check(&mut emu, &mut total, 64.0, 64.0); // exact-equal -> 0.0
    check(&mut emu, &mut total, 0.0, 0.0);
    check(&mut emu, &mut total, 64.0, 16.0);
    check(&mut emu, &mut total, 16.0, 64.0); // negative diff
    check(&mut emu, &mut total, -1000.0, 1000.0);
    let mut s = 0x4db7_2000u32;
    for _ in 0..2000 {
        let hi = rngf32(&mut s, -1000.0, 1000.0);
        let lo = rngf32(&mut s, -1000.0, 1000.0);
        check(&mut emu, &mut total, hi, lo);
    }
    eprintln!("glue_geom_4db710_bound_diff: {total} bit-exact (2 islands; fsub == source 0x4db841/0x4db86d)");
}

// ===========================================================================================
// Row 0x4dbce0 region-cost — copied-byte island `fld qword [in]; <src fadd>; fstp qword [out]`.
//   src fadd @0x4dbe06 = `d8 87 2c 4f bb 00` (fadd dword [edi+0xbb4f2c]); edi=0 => the field is the
//   absolute 0xbb4f2c we write. Seed st0 via fld qword [in] = the (widened) 0x4dbbd0 call result;
//   capture the f64 product (the FP-arith observable pre-narrow). fadd @0x4dbe06 (×1).
// ===========================================================================================
#[test]
fn glue_geom_4dbce0_cost_add() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let in_cell = emu.alloc(8);
    let out = emu.alloc(8);
    let isl = emu.alloc(0x20);

    let mut code = vec![0xddu8, 0x05]; // fld qword [in_cell] = DD 05 <disp32>
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let fadd_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x87, 0x2c, 0x4f, 0xbb, 0x00]); // copied src fadd dword [edi+0xbb4f2c]
    code.push(0xdd); // fstp qword [out] = DD 1D <disp32>
    code.push(0x1d);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        emu.read(isl + fadd_off, 6),
        emu.read(0x4dbe06, 6),
        "cost_add island fadd != source @0x4dbe06"
    );

    let run = |emu: &mut Emu, c: f64, acc: f32| -> f64 {
        emu.set_reg(RegisterX86::EDI, 0); // [edi+0xbb4f2c] => absolute 0xbb4f2c
        emu.write(in_cell, &c.to_bits().to_le_bytes());
        emu.write_u32(0xbb4f2c, acc.to_bits());
        emu.run(isl, isl_end);
        f64::from_bits(u64::from_le_bytes(emu.read(out, 8).try_into().unwrap()))
    };

    let check = |emu: &mut Emu, total: &mut u32, c: f64, acc: f32| {
        assert_eq!(
            run(emu, c, acc).to_bits(),
            cost_add_f64(c, acc).to_bits(),
            "cost_add c={c} acc={acc}"
        );
        *total += 1;
    };

    let mut total = 0u32;
    // `==threshold` probe: exact cancellation c + (−c) = 0.0 (acc = −c, c representable as f32).
    check(&mut emu, &mut total, 1.5, -1.5); // -> 0.0 exactly
    check(&mut emu, &mut total, 0.0, 0.0);
    check(&mut emu, &mut total, 1234.5, 0.0);
    check(&mut emu, &mut total, 0.0, 9876.5);
    check(&mut emu, &mut total, -2.5, 2.5);
    let mut s = 0x4dbc_e000u32;
    for _ in 0..3000 {
        let c = rngf(&mut s, -1.0e6, 1.0e6); // the 0x4dbbd0 call result (a PC_53 f64 on st0)
        let acc = rngf32(&mut s, -1.0e6, 1.0e6);
        check(&mut emu, &mut total, c, acc);
    }
    eprintln!("glue_geom_4dbce0_cost_add: {total} bit-exact (island; fadd == source 0x4dbe06)");
}
