//! Differential tests of the arithmetic of the movie and animated-texture functions `0x78c360`,
//! `0x78cf30` and `0x78d1a0`, each run against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fxch` excluded),
//! counted in each `[addr, true_ret)`:
//!
//! * `0x78c360` [.,0x78c795 ret 4]: `fidiv` ×1 @0x78c5ba
//! * `0x78cf30` [.,0x78d19f ret]  : `fmul` ×4 (0x78cf9f/0x78cfa5/0x78cfdb/0x78cfe1),
//!   `fadd` ×2 (0x78cfab/0x78cfe7)  (two byte-identical blocks)
//! * `0x78d1a0` [.,0x78d53b ret]  : `fmul` ×2 (0x78d480 [field+0xc], 0x78d4ae [field+0x8])
//!
//! Diff strategy per op:
//! * `0x78c360` — own-address **slice** `0x78c5ae..0x78c5c9` over `fild qword; fidiv dword; fstp
//!   dword [eax+0x33c]`, reading back the stored f32. (1 fidiv.)
//! * `0x78cf30` — own-address **slices**, ONE PER BLOCK (`0x78cf9c..0x78cfb6` and
//!   `0x78cfd8..0x78cff2`), each running `fild qword; fmul; fmul; fadd; call __ftol` and reading EAX.
//!   The constants `[0x801360]`/`[0x7ffa24]` are read live from the mapped image; the slices cover
//!   ALL 4 fmul + 2 fadd. (Byte-verified the two blocks are identical FP.)
//! * `0x78d1a0` — a **copied-byte island** per field offset: `fld dword [scale]; <source fmul bytes>;
//!   fstp dword [out]`, the source `fmul [eax+0xc]` (`d8 48 0c`) / `fmul [eax+0x8]` (`d8 48 08`)
//!   byte-verified == 0x78d480 / 0x78d4ae respectively (else circular). eax points at a field cell.
//!
//! Fuzz reads the LCG's HIGH bits. Mandatory deterministic
//! `==`-boundary probes sit on the `__ftol` truncate pivots and exact divide/multiply boundaries.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::movieframe::{
    aspect_ratio, quad_scale, time_to_index, HALF_F32_BITS, MS_TO_S_F32_BITS,
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
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0xc0; // room for [ebp-0xc0 .. ebp]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

// ===========================================================================================
// Row 0x78c360 aspect_ratio — slice 0x78c5ae..0x78c5c9:
//   fild qword [ebp-0x38]; mov [ebp-0x40],edx; mov [ebp-0x3c],eax; mov eax,[ebp-0x10];
//   fidiv dword [ebp-0x40]; mov edx,[ebp-0x18]; mov [ebp-0x30],edx; fstp dword [eax+0x33c].
// fidiv @0x78c5ba (×1). Feed the u32 numerator (qword: low=num, high=0) at [ebp-0x38], the i32
// divisor at [ebp-0x40], the store base at [ebp-0x10], and read the f32 at [base+0x33c].
// ===========================================================================================
#[test]
fn glue_geom_78c360_aspect_ratio() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let base = emu.alloc(0x400); // the `this` store base; [base+0x33c] holds the result

    let run = |emu: &mut Emu, num: u32, den: i32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        // The slice's `mov [ebp-0x40], edx` (0x78c5b1) writes the divisor from EDX (loaded just
        // before the slice from [esi+ebx+0x14] in the full fn) — set EDX, not the slot directly.
        emu.set_reg(RegisterX86::EDX, den as u32 as u64); // i32 divisor (-> [ebp-0x40])
        emu.write_u32(f.ebp - 0x38, num); // low dword of the qword numerator (set before slice)
        emu.write_u32(f.ebp - 0x34, 0); // high dword forced 0 (the eax==0 path; set before slice)
        emu.write_u32(f.ebp - 0x10, base as u32); // store base for fstp [eax+0x33c]
        emu.write_u32(f.ebp - 0x18, 0); // scratch (mov [ebp-0x30],[ebp-0x18])
        emu.run(0x78c5ae, 0x78c5c9);
        emu.read_f32(base + 0x33c)
    };

    let mut total = 0u32;
    // deterministic boundaries: exact-divide cases, equal num/den (==1.0), 0 numerator, the i32 sign.
    let probes: [(u32, i32); 14] = [
        (0, 1),
        (1, 1),
        (2, 1),
        (1, 2),
        (1, 3),      // 0.333… — exact-ratio rounding pivot
        (4, 3),      // 1.333… (the typical 4:3 aspect)
        (16, 9),     // 1.777…
        (1024, 768), // a real pixel ratio
        (100, 7),    // 14.285…
        (0xffffffff, 1),
        (0xffffffff, 0xffff), // large num, large den
        (1, -1),              // negative divisor
        (7, -3),              // negative non-exact
        (0x80000000, 3),
    ];
    for &(num, den) in &probes {
        assert_eq!(
            run(&mut emu, num, den).to_bits(),
            aspect_ratio(num, den).to_bits(),
            "aspect_ratio num={num} den={den}"
        );
        total += 1;
    }
    // dense small grid
    for num in 0u32..=64 {
        for den in 1i32..=64 {
            assert_eq!(
                run(&mut emu, num, den).to_bits(),
                aspect_ratio(num, den).to_bits(),
                "aspect_ratio grid num={num} den={den}"
            );
            total += 1;
        }
    }
    // fuzz the u32 numerator x i32 divisor (high LCG bits); avoid den==0 (masked-#Z inf is exact
    // either way, but keep the diff over finite results dense; one explicit den==0 probe below).
    let mut s = 0x78c3_6000u32;
    for _ in 0..6000 {
        let num = next(&mut s);
        let mut den = next(&mut s) as i32;
        if den == 0 {
            den = 1;
        }
        assert_eq!(
            run(&mut emu, num, den).to_bits(),
            aspect_ratio(num, den).to_bits(),
            "aspect_ratio fuzz num={num} den={den}"
        );
        total += 1;
    }
    // explicit den==0: masked-#Z. fild qword(num) / 0 -> +inf for num>0, NaN for num==0 (0/0).
    for &num in &[0u32, 1, 1000, 0xffffffff] {
        assert_eq!(
            run(&mut emu, num, 0).to_bits(),
            aspect_ratio(num, 0).to_bits(),
            "aspect_ratio den=0 num={num}"
        );
        total += 1;
    }
    eprintln!(
        "glue_geom_78c360_aspect_ratio: {total} bit-exact (fild qword; fidiv dword; fstp dword)"
    );
}

// ===========================================================================================
// Row 0x78cf30 time_to_index — two byte-identical blocks, each:
//   fild qword [ebp-0x8]; fmul dword [esi+0x33c]; fmul dword [0x801360]=0.001; fadd dword
//   [0x7ffa24]=0.5; call __ftol.  Slice block1 = 0x78cf9c..0x78cfb6, block2 = 0x78cfd8..0x78cff2.
// fmul @0x78cf9f/0x78cfa5/0x78cfdb/0x78cfe1, fadd @0x78cfab/0x78cfe7 (the two blocks). Feed the u32
// time at [ebp-0x8] (qword, hi dword 0), the rate f32 at [esi+0x33c]; read the __ftol result in EAX.
// ===========================================================================================
#[test]
fn glue_geom_78cf30_time_to_index() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let esi = emu.alloc(0x400); // `this`; [esi+0x33c] holds the rate

    // confirm our transcribed constants match the live image bytes the slice reads.
    assert_eq!(
        u32::from_le_bytes(emu.read(0x801360, 4).try_into().unwrap()),
        MS_TO_S_F32_BITS,
        "MS_TO_S_F32 @ 0x801360"
    );
    assert_eq!(
        u32::from_le_bytes(emu.read(0x7ffa24, 4).try_into().unwrap()),
        HALF_F32_BITS,
        "HALF_F32 @ 0x7ffa24"
    );
    // byte-verify the two FP blocks are identical (the FP run, fild..fadd = 0x78cf9c..0x78cfb1).
    assert_eq!(
        emu.read(0x78cf9c, 0x78cfb1 - 0x78cf9c),
        emu.read(0x78cfd8, 0x78cfb1 - 0x78cf9c),
        "0x78cf30 block1 != block2 FP bytes"
    );

    let blocks = [(0x78cf9cu64, 0x78cfb6u64), (0x78cfd8, 0x78cff2)];
    let run = |emu: &mut Emu, start: u64, end: u64, t: u32, rate: f32| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(f.ebp - 0x8, t); // low dword of the qword time value
        emu.write_u32(f.ebp - 0x4, 0); // high dword forced 0
        emu.write_u32(esi + 0x33c, rate.to_bits());
        emu.run(start, end);
        emu.get_reg(RegisterX86::EAX) as u32 as i32
    };

    let mut total = 0u32;
    // deterministic: __ftol pivots and the exact-product boundaries. rate is the runtime f32 field.
    let rates = [
        0.0f32,
        1.0,
        0.5,
        30.0, // 30 fps-ish rate
        60.0,
        23.976,
        1.0 / 3.0,
        1000.0, // ·0.001 = 1.0
        0.001,
        12345.678,
    ];
    let times = [
        0u32, 1, 2, 499, 500, 501, 999, 1000, 1001, 1500, 33, 16, 0xffff, 0x10000, 1_000_000,
        0x7fffffff, 0x80000000, 0xffffffff,
    ];
    for &(start, end) in &blocks {
        for &rate in &rates {
            for &t in &times {
                assert_eq!(
                    run(&mut emu, start, end, t, rate),
                    time_to_index(t, rate),
                    "time_to_index @ {start:#x} t={t} rate={rate}"
                );
                total += 1;
            }
        }
        // a deterministic ==pivot: t·rate·0.001+0.5 exactly at an integer boundary.
        // rate=2000, t=1 -> 1·2000·0.001 = 2.0, +0.5 = 2.5 -> trunc 2.
        assert_eq!(
            run(&mut emu, start, end, 1, 2000.0),
            time_to_index(1, 2000.0),
            "time_to_index pivot @ {start:#x}"
        );
        // rate=500, t=1 -> 0.5, +0.5 = 1.0 -> trunc 1 (the ==1.0 boundary).
        assert_eq!(
            run(&mut emu, start, end, 1, 500.0),
            time_to_index(1, 500.0),
            "time_to_index ==1.0 pivot @ {start:#x}"
        );
        total += 2;
    }
    let mut s = 0x78cf_3000u32;
    for _ in 0..5000 {
        let start_end = if next(&mut s) & 0x100 != 0 {
            blocks[0]
        } else {
            blocks[1]
        };
        let t = next(&mut s);
        let rate = rngf(&mut s, -100.0, 100.0) as f32;
        assert_eq!(
            run(&mut emu, start_end.0, start_end.1, t, rate),
            time_to_index(t, rate),
            "time_to_index fuzz t={t} rate={rate}"
        );
        total += 1;
    }
    eprintln!(
        "glue_geom_78cf30_time_to_index: {total} bit-exact (2x: fild qword; fmul; fmul[0.001]; fadd[0.5]; __ftol)"
    );
}

// ===========================================================================================
// Row 0x78d1a0 quad_scale — copied-byte ISLAND per field offset:
//   fld dword [scale_cell]; <source fmul [eax+off] bytes>; fstp dword [out_cell].
// The source `fmul [eax+0xc]` (d8 48 0c @0x78d480) and `fmul [eax+0x8]` (d8 48 08 @0x78d4ae) are
// byte-verified against the island's fmul; eax points at a field cell so [eax+off] = field.
// fmul ×2. Feed scale at [scale_cell] and field at [eax+off]; read the f32 product at [out_cell].
// ===========================================================================================
#[test]
fn glue_geom_78d1a0_quad_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let scale_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let field_base = emu.alloc(0x40); // eax; [eax+0xc]/[eax+0x8] = field

    // build one island per offset: fld [scale] (D9 05 disp32); <fmul bytes>; fstp [out] (D9 1D disp32)
    let build = |emu: &mut Emu, fmul_bytes: &[u8]| -> (u64, u64, u64) {
        let isl = emu.alloc(0x20);
        let mut code = vec![0xd9u8, 0x05];
        code.extend_from_slice(&(scale_cell as u32).to_le_bytes());
        let fmul_off = code.len() as u64;
        code.extend_from_slice(fmul_bytes);
        code.push(0xd9);
        code.push(0x1d);
        code.extend_from_slice(&(out_cell as u32).to_le_bytes());
        let isl_end = isl + code.len() as u64;
        emu.write(isl, &code);
        (isl, isl + fmul_off, isl_end)
    };
    // offset 0xc (0x78d480) and offset 0x8 (0x78d4ae)
    let (isl_c, fmul_c, end_c) = build(&mut emu, &[0xd8, 0x48, 0x0c]);
    let (isl_8, fmul_8, end_8) = build(&mut emu, &[0xd8, 0x48, 0x08]);
    // byte-verify each island's fmul == its source site (else the island proves nothing).
    assert_eq!(
        emu.read(fmul_c, 3),
        emu.read(0x78d480, 3),
        "quad_scale island fmul[+0xc] != source @ 0x78d480"
    );
    assert_eq!(
        emu.read(fmul_8, 3),
        emu.read(0x78d4ae, 3),
        "quad_scale island fmul[+0x8] != source @ 0x78d4ae"
    );

    let run = |emu: &mut Emu, isl: u64, end: u64, foff: u64, scale: f32, field: f32| -> f32 {
        emu.set_reg(RegisterX86::EAX, field_base);
        emu.write_u32(scale_cell, scale.to_bits());
        emu.write_u32(field_base + foff, field.to_bits());
        emu.run(isl, end);
        emu.read_f32(out_cell)
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, scale: f32, field: f32| {
        let want = quad_scale(scale, field);
        assert_eq!(
            run(emu, isl_c, end_c, 0xc, scale, field).to_bits(),
            want.to_bits(),
            "quad_scale[+0xc] scale={scale} field={field}"
        );
        assert_eq!(
            run(emu, isl_8, end_8, 0x8, scale, field).to_bits(),
            want.to_bits(),
            "quad_scale[+0x8] scale={scale} field={field}"
        );
        *total += 2;
    };

    // deterministic boundaries: 0, 1, exact powers, signs, and a ==product pivot.
    let scales = [
        0.0f32, -0.0, 1.0, -1.0, 0.5, 2.0, 0.75, 12.0, 1e-7, 1e7, 0.999_999,
    ];
    let fields = [
        0.0f32, -0.0, 1.0, -1.0, 0.5, 256.0, 0.125, 3.5, -0.001, 65535.0,
    ];
    for &scale in &scales {
        for &field in &fields {
            check(&mut emu, &mut total, scale, field);
        }
    }
    // ==pivot: scale·field is exactly representable f32 at one product but rounds at the f32 store.
    check(&mut emu, &mut total, 1.0 / 3.0, 3.0); // 0.333… · 3 -> ~1.0 (rounding boundary)
    check(&mut emu, &mut total, 0.1, 10.0); // 0.1f32 · 10 -> the classic non-exact pivot

    let mut s = 0x78d1_a000u32;
    for _ in 0..4000 {
        let scale = rngf(&mut s, -10.0, 10.0) as f32;
        let field = rngf(&mut s, -1000.0, 1000.0) as f32;
        check(&mut emu, &mut total, scale, field);
    }
    eprintln!(
        "glue_geom_78d1a0_quad_scale: {total} bit-exact (2x island: fld; fmul [eax+off]; fstp)"
    );
}
