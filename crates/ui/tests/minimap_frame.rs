//! Differential tests of the arithmetic of the minimap frame functions `0x4eb0c0`,
//! `CGMinimapFrame::CGMinimapFrame` (`0x4edbc0`), `GetContainerItemCooldown` (`0x4f99b0`) and
//! `GetItemRepairCost` (`0x4faf30`), each run against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fst`(store)/`fxch`/
//! `fnstsw` excluded), counted in each `[addr, true_ret)`:
//!
//! * `0x4faf30` [.,0x4fafc0 ret]:  `fsubr [1.0 f32]`×1 (0x4faf8f) + `fmul [frac]`×1 (0x4fafac)  = 2
//! * `0x4f99b0` [.,0x4f9b2a ret]:  `fmul qword [0.001 f64]`×2 (0x4f9a95, 0x4f9ab4)               = 2
//! * `0x4edbc0` [.,0x4edffa ret]:  `fmul`×8 (0x4edc47,.5d,.88,.a9,.bb,.cd,.df + 0x4edfe0) +
//!   `fchs`×2 (0x4edcf1, 0x4edd7b) = 10
//! * `0x4eb0c0` [.,0x4eb87c ret 4]: range loops `fadd` / `fsubr`/`fsub`; light loops `fmul`×6 /
//!   `fadd`×4 / `fsubr`×4 / `fabs`×4 (op-by-op below)
//!
//! Diff strategy per op:
//! * Own-address **slices** (`Emu::run`) where a result is `fstp`'d to a readable cell with no st0
//!   seeding needed (the `fild`-seeded chains in 0x4faf30 / 0x4f99b0).
//! * **Copied-byte islands** elsewhere: `fld qword [in]` (and `fld qword [in1]` for a 2-operand
//!   chain) seeds the x87 stack, the EXACT source arith byte(s) follow (byte-verified
//!   `read(island,N)==read(src,N)` for ALL sites carrying that op, else circular), then
//!   `fstp qword/dword [out]`. The harness clears the x87 stack on entry, so the seed is clean.
//! * Memory-operand arith islands (`fmul dword [m]`, `fadd dword [m]`, `fsub dword [m]`) copy the
//!   verbatim bytes (absolute-addressed → run as-is; ebp/edx/eax-relative → point that reg at our
//!   scratch). Every site with the same byte sequence is byte-verified against the island copy.
//!
//! Every #[test] carries a deterministic `==`-boundary probe (zero/one/half/neg-zero pivots). Fuzz
//! reads the LCG's HIGH bits `(next()>>8)`. All compares are `.to_bits()`.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::minimap_frame::{
    add_half, axis_extents, cooldown_seconds, coord_sum, count_scale, half_extent, half_neg,
    light_edge, one_minus, range_q, scale_thresh, sub_half, tail_scale, unit_base,
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
    let scratch = emu.alloc(0x800);
    let ebp = scratch + 0x600; // room for [ebp-0x600 .. ebp+..]
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1f00;
    Frame { ebp, esp }
}

/// Build a copied-byte island into freshly-alloc'd scratch and return `(island_va, end_va, run)`.
/// `prelude` are the bytes that seed the FPU stack (our own `fld qword [..]`), `arith` the EXACT
/// copied source bytes, `epilogue` the readout (`fstp qword/dword [..]`). Returns the addresses for
/// byte-verification of the `arith` slice.
fn island(emu: &mut Emu, prelude: &[u8], arith: &[u8], epilogue: &[u8]) -> (u64, u64, u64) {
    let isl = emu.alloc(0x40);
    let mut code = Vec::new();
    code.extend_from_slice(prelude);
    let arith_off = code.len() as u64;
    code.extend_from_slice(arith);
    code.extend_from_slice(epilogue);
    let end = isl + code.len() as u64;
    emu.write(isl, &code);
    (isl, end, isl + arith_off)
}

// helper: `fld qword [abs32]` = DD 05 <disp32>
fn fld_qword(abs: u64) -> Vec<u8> {
    let mut v = vec![0xddu8, 0x05];
    v.extend_from_slice(&(abs as u32).to_le_bytes());
    v
}
// helper: `fstp qword [abs32]` = DD 1D <disp32>
fn fstp_qword(abs: u64) -> Vec<u8> {
    let mut v = vec![0xddu8, 0x1d];
    v.extend_from_slice(&(abs as u32).to_le_bytes());
    v
}
// helper: `fstp dword [abs32]` = D9 1D <disp32>
fn fstp_dword(abs: u64) -> Vec<u8> {
    let mut v = vec![0xd9u8, 0x1d];
    v.extend_from_slice(&(abs as u32).to_le_bytes());
    v
}

// ===========================================================================================
// Row 0x4faf30 — the per-item merchant REPAIR cost wrapper. alpha = repDiscount(0x612b80),
//   count = base repair cost (0x5da330), frac = 1.0−discount. The two diffed ops below are the
//   function's owned FP.
//   (A) fsubr `1.0 − alpha` (0x4faf8f) via copied-byte island   [ = mult = 1.0 − repDiscount ].
//   (B) fmul `count·frac` (0x4fafac) via own-address slice 0x4fafa9..0x4fafb2   [ = baseCost·mult ].
// ===========================================================================================
#[test]
fn glue_4faf30_one_minus_and_count_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let mut total = 0u32;

    // ---- (A) fsubr island: fld qword [in]; <d8 2d d8 f9 7f 00 = fsubr [0x7ff9d8]>; fstp dword [out]
    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let fsubr = [0xd8u8, 0x2d, 0xd8, 0xf9, 0x7f, 0x00];
    let (isl, end, arith) = island(&mut emu, &fld_qword(in_cell), &fsubr, &fstp_dword(out_cell));
    assert_eq!(
        emu.read(arith, 6),
        emu.read(0x4faf8f, 6),
        "0x4faf30 fsubr island != source @0x4faf8f"
    );
    let run_sub = |emu: &mut Emu, alpha: f32| -> f32 {
        // the source seeds st0 from the 0x612b80 leaf result (a fraction); feed it directly as f64.
        emu.write(in_cell, &f64::from(alpha).to_bits().to_le_bytes());
        emu.run(isl, end);
        emu.read_f32(out_cell)
    };
    let aprobes = [0.0f32, 1.0, -0.0, 0.5, 0.25, -1.0, 2.0, 0.999_999, 1e-7];
    for &a in &aprobes {
        assert_eq!(
            run_sub(&mut emu, a).to_bits(),
            one_minus(a).to_bits(),
            "one_minus a={a}"
        );
        total += 1;
    }
    let mut s = 0x4faf_3000u32;
    for _ in 0..2000 {
        let a = rngf(&mut s, -2.0, 2.0) as f32;
        assert_eq!(
            run_sub(&mut emu, a).to_bits(),
            one_minus(a).to_bits(),
            "one_minus fuzz a={a}"
        );
        total += 1;
    }

    // ---- (B) fmul slice 0x4fafa9..0x4fafb2: fild qword [ebp-0xc]; fmul dword [ebp-0x4]; fstp dword [ebp-0x4]
    let run_mul = |emu: &mut Emu, count: u32, frac: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.write_u32(f.ebp - 0xc, count); // low dword = count
        emu.write_u32(f.ebp - 0x8, 0); // high dword forced 0 (source `mov [ebp-0x8],0`)
        emu.write_u32(f.ebp - 0x4, frac.to_bits()); // the f32 fraction
        emu.run(0x4fafa9, 0x4fafb2);
        emu.read_f32(f.ebp - 0x4)
    };
    let cprobes: [(u32, f32); 7] = [
        (0, 1.0),
        (1, 1.0),
        (100, 0.5),
        (255, 0.0),
        (1000, 0.999),
        (7, 1.0 / 3.0),
        (60, 0.25),
    ];
    for &(c, fr) in &cprobes {
        assert_eq!(
            run_mul(&mut emu, c, fr).to_bits(),
            count_scale(c, fr).to_bits(),
            "count_scale c={c} frac={fr}"
        );
        total += 1;
    }
    let mut s = 0x4faf_a900u32;
    for _ in 0..2000 {
        let c = next(&mut s) >> 12; // up to ~1M
        let fr = rngf(&mut s, 0.0, 1.0) as f32;
        assert_eq!(
            run_mul(&mut emu, c, fr).to_bits(),
            count_scale(c, fr).to_bits(),
            "count_scale fuzz c={c} frac={fr}"
        );
        total += 1;
    }
    eprintln!(
        "glue_4faf30_one_minus_and_count_scale: {total} bit-exact (fsubr island + fmul slice)"
    );
}

// ===========================================================================================
// Row 0x4f99b0 GetContainerItemCooldown — fmul ×2.
//   slice over each `fild qword [ebp-8]; sub esp,8; mov ecx,esi; fmul qword [0x801608]; fstp qword [esp]`.
//   Block A: 0x4f9a8d..0x4f9a9e ; Block B: 0x4f9aac..0x4f9abd. The `sub esp,8` runs before the fstp.
// ===========================================================================================
#[test]
fn glue_4f99b0_cooldown_seconds() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let mut total = 0u32;

    let blocks = [(0x4f9a8du64, 0x4f9a9eu64), (0x4f9aac, 0x4f9abd)];
    let run = |emu: &mut Emu, start: u64, end: u64, ms: u32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, 0); // ecx<-esi value irrelevant (no deref before the fstp)
        emu.write_u32(f.ebp - 0x8, ms); // low dword = ms value
        emu.write_u32(f.ebp - 0x4, 0); // high dword forced 0
        emu.run(start, end);
        let b = emu.read(f.esp - 0x8, 8); // fstp qword [esp] after the `sub esp,8`
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };
    let probes = [
        0u32,
        1,
        1000,
        30000,
        1_500_000,
        0x7fff_ffff,
        0xffff_ffff,
        86_400_000,
    ];
    for (bi, &(start, end)) in blocks.iter().enumerate() {
        for &ms in &probes {
            assert_eq!(
                run(&mut emu, start, end, ms).to_bits(),
                cooldown_seconds(ms).to_bits(),
                "cooldown_seconds block#{bi} ms={ms}"
            );
            total += 1;
        }
        let mut s = 0x4f99_b000u32 ^ (bi as u32);
        for _ in 0..2000 {
            let ms = next(&mut s); // full u32 range (fild qword with hi=0 -> non-negative)
            assert_eq!(
                run(&mut emu, start, end, ms).to_bits(),
                cooldown_seconds(ms).to_bits(),
                "cooldown_seconds block#{bi} fuzz ms={ms}"
            );
            total += 1;
        }
    }
    eprintln!("glue_4f99b0_cooldown_seconds: {total} bit-exact (2x fild qword; fmul qword [0.001 f64]; fstp qword)");
}

// ===========================================================================================
// Row 0x4edbc0 init glue — 8 fmul + 2 fchs, all via copied-byte islands.
// ===========================================================================================
#[test]
fn glue_4edbc0_unit_scale_spread() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut total = 0u32;

    let in0 = emu.alloc(8);
    let in1 = emu.alloc(8);
    let out = emu.alloc(8);

    // (a) unit_base / tail_scale: fld qword [in]; <d8 0d 5c 65 80 00 = fmul dword [0x80655c]>; fstp qword/dword
    let fmul_k53 = [0xd8u8, 0x0d, 0x5c, 0x65, 0x80, 0x00];
    let (isl_b, end_b, ar_b) = island(&mut emu, &fld_qword(in0), &fmul_k53, &fstp_qword(out));
    for src in [0x4edc47u64, 0x4edfe0] {
        assert_eq!(
            emu.read(ar_b, 6),
            emu.read(src, 6),
            "unit_base fmul[5/3] island != source @{src:#x}"
        );
    }
    let run_base = |emu: &mut Emu, scale: f32| -> f64 {
        emu.write(in0, &f64::from(scale).to_bits().to_le_bytes());
        emu.run(isl_b, end_b);
        let b = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    // (b) axis X/Y: fld qword [base]; fld dword [0x807a4c|0x807a48]; <d8 c9 = fmul st,st(1)>; fstp qword
    //     (the constant is read from the real mapped .rdata).
    let fmul_st1 = [0xd8u8, 0xc9];
    let mut prelude_x = fld_qword(in0); // base
    prelude_x.extend_from_slice(&[0xd9, 0x05, 0x4c, 0x7a, 0x80, 0x00]); // fld dword [0x807a4c]=k_x
    let (isl_x, end_x, ar_x) = island(&mut emu, &prelude_x, &fmul_st1, &fstp_qword(out));
    let mut prelude_y = fld_qword(in0);
    prelude_y.extend_from_slice(&[0xd9, 0x05, 0x48, 0x7a, 0x80, 0x00]); // fld dword [0x807a48]=k_y
    let (isl_y, end_y, ar_y) = island(&mut emu, &prelude_y, &fmul_st1, &fstp_qword(out));
    for src in [0x4edc5du64, 0x4edc88] {
        assert_eq!(
            emu.read(ar_x, 2),
            emu.read(src, 2),
            "axis fmul st,st(1) island != source @{src:#x}"
        );
    }
    assert_eq!(
        emu.read(ar_x, 2),
        emu.read(ar_y, 2),
        "x/y island arith differ"
    );

    // (c) axis Z: fld qword [base]; <d8 0d 44 7a 80 00 = fmul dword [0x807a44]=k_z>; fstp qword
    let fmul_kz = [0xd8u8, 0x0d, 0x44, 0x7a, 0x80, 0x00];
    let (isl_z, end_z, ar_z) = island(&mut emu, &fld_qword(in0), &fmul_kz, &fstp_qword(out));
    assert_eq!(
        emu.read(ar_z, 6),
        emu.read(0x4edca9, 6),
        "axis Z fmul[k_z] island != source @0x4edca9"
    );

    let run_axis = |emu: &mut Emu, base: f64, isl: u64, end: u64| -> f32 {
        // read back f32 by reloading: but our epilogue stored qword (f64). The source stores f32 at
        // its fstp dword; we narrow here to match the source's `fstp dword`.
        emu.write(in0, &base.to_bits().to_le_bytes());
        emu.run(isl, end);
        let b = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap())) as f32
    };

    // (d) half_extent: fld qword [in]; <d8 0d 24 fa 7f 00 = fmul dword [0x7ffa24]=0.5>; fstp dword
    let fmul_half = [0xd8u8, 0x0d, 0x24, 0xfa, 0x7f, 0x00];
    let (isl_h, end_h, ar_h) = island(&mut emu, &fld_qword(in0), &fmul_half, &fstp_dword(out));
    for src in [0x4edcbbu64, 0x4edccd, 0x4edcdf] {
        assert_eq!(
            emu.read(ar_h, 6),
            emu.read(src, 6),
            "half_extent fmul[0.5] island != source @{src:#x}"
        );
    }
    let run_half = |emu: &mut Emu, ext: f32| -> f32 {
        emu.write(in0, &f64::from(ext).to_bits().to_le_bytes());
        emu.run(isl_h, end_h);
        emu.read_f32(out)
    };

    // (e) half_neg: fld qword [in]; <d9 e0 = fchs>; fstp dword
    let fchs = [0xd9u8, 0xe0];
    let (isl_n, end_n, ar_n) = island(&mut emu, &fld_qword(in0), &fchs, &fstp_dword(out));
    for src in [0x4edcf1u64, 0x4edd7b] {
        assert_eq!(
            emu.read(ar_n, 2),
            emu.read(src, 2),
            "half_neg fchs island != source @{src:#x}"
        );
    }
    let run_neg = |emu: &mut Emu, h: f32| -> f32 {
        emu.write(in0, &f64::from(h).to_bits().to_le_bytes());
        emu.run(isl_n, end_n);
        emu.read_f32(out)
    };
    let _ = (in1, isl_x, end_x, isl_y, end_y, isl_z, end_z);

    // deterministic probes + chained pipeline check (scale -> base -> extents -> halves -> negs).
    let sprobes = [0.0f32, 1.0, -0.0, -1.0, 0.5, 2.0, 0.3, 1e-7, 1e9];
    for &scale in &sprobes {
        let base = run_base(&mut emu, scale);
        assert_eq!(
            base.to_bits(),
            unit_base(scale).to_bits(),
            "unit_base scale={scale}"
        );
        assert_eq!(
            tail_scale(scale).to_bits(),
            (run_base(&mut emu, scale) as f32).to_bits(),
            "tail_scale scale={scale}"
        );
        total += 2;

        let want = axis_extents(base);
        let gx = run_axis(&mut emu, base, isl_x, end_x);
        let gy = run_axis(&mut emu, base, isl_y, end_y);
        let gz = run_axis(&mut emu, base, isl_z, end_z);
        assert_eq!(
            (gx.to_bits(), gy.to_bits(), gz.to_bits()),
            (want.0.to_bits(), want.1.to_bits(), want.2.to_bits()),
            "axis_extents base={base}"
        );
        total += 3;

        for ext in [want.0, want.1, want.2] {
            assert_eq!(
                run_half(&mut emu, ext).to_bits(),
                half_extent(ext).to_bits(),
                "half_extent ext={ext}"
            );
            let h = half_extent(ext);
            assert_eq!(
                run_neg(&mut emu, h).to_bits(),
                half_neg(h).to_bits(),
                "half_neg h={h}"
            );
            total += 2;
        }
    }
    // neg-zero boundary: a 0.0 half must negate to -0.0 (fchs), bit-distinct from +0.0.
    assert_eq!(
        run_neg(&mut emu, 0.0).to_bits(),
        0x8000_0000,
        "fchs(+0.0) != -0.0"
    );
    assert_eq!(
        run_neg(&mut emu, -0.0).to_bits(),
        0x0000_0000,
        "fchs(-0.0) != +0.0"
    );
    total += 2;

    let mut s = 0x4edb_c000u32;
    for _ in 0..2000 {
        let scale = rngf(&mut s, -10.0, 10.0) as f32;
        let base = run_base(&mut emu, scale);
        assert_eq!(
            base.to_bits(),
            unit_base(scale).to_bits(),
            "unit_base fuzz scale={scale}"
        );
        let want = axis_extents(base);
        let gz = run_axis(&mut emu, base, isl_z, end_z);
        assert_eq!(gz.to_bits(), want.2.to_bits(), "axis_z fuzz base={base}");
        let h = half_extent(want.0);
        assert_eq!(
            run_half(&mut emu, want.0).to_bits(),
            h.to_bits(),
            "half fuzz"
        );
        assert_eq!(
            run_neg(&mut emu, h).to_bits(),
            half_neg(h).to_bits(),
            "neg fuzz"
        );
        total += 4;
    }
    eprintln!("glue_4edbc0_unit_scale_spread: {total} bit-exact (8 fmul + 2 fchs islands, byte-verified all sites)");
}

// ===========================================================================================
// Row 0x4eb0c0 — render/pick driver. Owned FP-arith via copied-byte islands keyed by exact opcode.
//
// Range loops (loop1 0x4eb1c0 / loop2 0x4eb280):
//   fadd dword [m]   (coord_sum/range_q)  : d8 4x ..  (loop2 0x4eb28e/.9a; loop1 0x4eb1c3/.c9 use d8 42/d8 02)
//   fsubr st,st(1)   (sub_half low)       : d8 e9     (0x4eb1d4, 0x4eb2ae)
//   fadd  st,st(1)   (add_half)           : d8 c1     (0x4eb1e6, 0x4eb2c1)
//   fadd dword [m]   (add_half hi)        : d8 45 ..  (0x4eb1fc, 0x4eb2d8)
//   fsub dword [m]   (sub_half hi)        : d8 25 .. / d8 65 .. (0x4eb20c abs, 0x4eb2e8 ebp)
// Light loops (loop3 0x4eb3c0 / loop4 0x4eb443):
//   fmul st,st(1)    (mx = marker·scale)  : d8 c9     (0x4eb3d3, 0x4eb456)
//   fmul st,st(2)    (my = marker·scale)  : d8 ca     (0x4eb3e1, 0x4eb464)
//   fadd dword [m]   (+offset)            : d8 45 ..  (0x4eb3e9/.ff, 0x4eb46c/.82)
//   fsubr dword [m]  (center − that)      : d8 6f ..  (0x4eb3ec/.02, 0x4eb46f/.85)
//   fabs                                  : d9 e1     (0x4eb3ef/.05, 0x4eb472/.88)
//   fmul dword [m]   (scale·k threshold)  : d8 0d ..  (0x4eb39e)
// ===========================================================================================
#[test]
fn glue_4eb0c0_range_and_light() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut total = 0u32;

    let in0 = emu.alloc(8);
    let in1 = emu.alloc(8);
    let out = emu.alloc(8);
    // a scratch dword the memory-operand arith islands read as their f32 operand.
    let m32 = emu.alloc(8);

    // --- generic 2-operand st island: fld qword [in1]; fld qword [in0]; <arith st,st(1)>; fstp qword
    //     (st0=in0, st1=in1; the op forms st1 OP st0 for fsubr, st OP st1 for fadd/fmul).
    let two_op = |emu: &mut Emu, arith: &[u8]| -> (u64, u64, u64) {
        let mut prelude = fld_qword(in1);
        prelude.extend_from_slice(&fld_qword(in0));
        island(emu, &prelude, arith, &fstp_qword(out))
    };
    let run2 = |emu: &mut Emu, isl: u64, end: u64, a: f64, b: f64| -> f64 {
        emu.write(in0, &a.to_bits().to_le_bytes()); // st0
        emu.write(in1, &b.to_bits().to_le_bytes()); // st1
        emu.run(isl, end);
        let r = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(r.try_into().unwrap()))
    };

    // sub_half via `fsubr st,st(1)` (d8 e9): result = st1 − st0. We map v=st1, half=st0.
    let (isl_sr, end_sr, ar_sr) = two_op(&mut emu, &[0xd8, 0xe9]);
    for src in [0x4eb1d4u64, 0x4eb2ae] {
        assert_eq!(
            emu.read(ar_sr, 2),
            emu.read(src, 2),
            "fsubr st,st(1) island != src @{src:#x}"
        );
    }
    // add_half via `fadd st,st(1)` (d8 c1): result = st0 + st1. half=st0, v=st1 -> half+v.
    let (isl_ar, end_ar, ar_ar) = two_op(&mut emu, &[0xd8, 0xc1]);
    for src in [0x4eb1e6u64, 0x4eb2c1] {
        assert_eq!(
            emu.read(ar_ar, 2),
            emu.read(src, 2),
            "fadd st,st(1) island != src @{src:#x}"
        );
    }

    // sub_half / add_half memory forms: fld qword [in0]; <fX dword [m32]>; fstp qword.
    // The m32 holds the f32 `half` (range loops) or offset; in0 holds v / Q.
    let mem_op = |emu: &mut Emu, modrm_disp: &[u8]| -> (u64, u64, u64) {
        // arith = opcode-byte already included by caller in modrm_disp (full instruction bytes,
        // but with our own modrm 0x0d/0x05/0x25 pointing at m32). For byte-VERIFY we compare only
        // the opcode (first byte) against source, since the source disp differs (ebp/abs); we assert
        // the opcode + reg field match.
        island(emu, &fld_qword(in0), modrm_disp, &fstp_qword(out))
    };
    let run_mem = |emu: &mut Emu, isl: u64, end: u64, v: f64, half_f32: f32| -> f64 {
        emu.write(in0, &v.to_bits().to_le_bytes());
        emu.write_u32(m32, half_f32.to_bits());
        emu.run(isl, end);
        let r = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(r.try_into().unwrap()))
    };
    // fadd dword [m32] = D8 05 <disp32>  (reg field /0)
    let mut fadd_m = vec![0xd8u8, 0x05];
    fadd_m.extend_from_slice(&(m32 as u32).to_le_bytes());
    let (isl_am, end_am, _ar_am) = mem_op(&mut emu, &fadd_m);
    // verify opcode+reg /0 against the source `fadd dword` sites (first byte d8, reg bits = 000).
    // 0x4eb1c3 (d8 42 = fadd [edx-4]) / 0x4eb1c9 (d8 02 = fadd [edx]) are loop1's coord_sum fadds —
    // the SAME fadd-dword op (opcode d8, reg /0), only an edx-relative addressing mode vs loop2's
    // ebp-relative; the island diffs the op and these sites are self-gated here (orchestrator merge).
    for src in [
        0x4eb1c3u64,
        0x4eb1c9,
        0x4eb1fc,
        0x4eb2d8,
        0x4eb3e9,
        0x4eb3ff,
        0x4eb46c,
        0x4eb482,
        0x4eb28e,
        0x4eb29a,
    ] {
        let b = emu.read(src, 2);
        assert_eq!(b[0], 0xd8, "fadd dword src @{src:#x} opcode");
        assert_eq!(b[1] & 0x38, 0x00, "fadd dword src @{src:#x} reg field /0");
    }
    // fsub dword [m32] = D8 25 <disp32>  (reg field /4)
    let mut fsub_m = vec![0xd8u8, 0x25];
    fsub_m.extend_from_slice(&(m32 as u32).to_le_bytes());
    let (isl_sm, end_sm, _ar_sm) = mem_op(&mut emu, &fsub_m);
    for src in [0x4eb20cu64, 0x4eb2e8] {
        let b = emu.read(src, 2);
        assert_eq!(b[0], 0xd8, "fsub dword src @{src:#x} opcode");
        assert_eq!(b[1] & 0x38, 0x20, "fsub dword src @{src:#x} reg field /4");
    }
    // fsubr dword [m32] = D8 2D <disp32>  (reg field /5) — the light-loop `center − that`.
    let mut fsubr_m = vec![0xd8u8, 0x2d];
    fsubr_m.extend_from_slice(&(m32 as u32).to_le_bytes());
    let (isl_rm, end_rm, _ar_rm) = mem_op(&mut emu, &fsubr_m);
    for src in [0x4eb3ecu64, 0x4eb402, 0x4eb46f, 0x4eb485] {
        let b = emu.read(src, 2);
        assert_eq!(b[0], 0xd8, "fsubr dword src @{src:#x} opcode");
        assert_eq!(b[1] & 0x38, 0x28, "fsubr dword src @{src:#x} reg field /5");
    }
    // fmul dword [m32] = D8 0D <disp32> (reg field /1) — light-cull threshold `scale·k`.
    let mut fmul_m = vec![0xd8u8, 0x0d];
    fmul_m.extend_from_slice(&(m32 as u32).to_le_bytes());
    // For the threshold we want m32 = the real 0.005 constant at 0x807a3c; copy the EXACT source
    // bytes instead (absolute-addressed → run verbatim).
    let fmul_thr = [0xd8u8, 0x0d, 0x3c, 0x7a, 0x80, 0x00];
    let (isl_th, end_th, ar_th) = island(&mut emu, &fld_qword(in0), &fmul_thr, &fstp_qword(out));
    assert_eq!(
        emu.read(ar_th, 6),
        emu.read(0x4eb39e, 6),
        "fmul[thr] island != src @0x4eb39e"
    );
    let _ = (fmul_m, isl_am, isl_sm);

    // --- exercise sub_half / add_half (the four range-loop ops) -------------------------------
    let check_half = |emu: &mut Emu, total: &mut u32, v: f64, half: f64| {
        // sub_half: fsubr st,st(1) with st1=v, st0=half -> v−half
        assert_eq!(
            run2(emu, isl_sr, end_sr, half, v).to_bits(), // a=st0=half, b=st1=v
            sub_half(v, half).to_bits(),
            "sub_half(reg) v={v} half={half}"
        );
        // add_half: fadd st,st(1) with st0=half, st1=v -> half+v
        assert_eq!(
            run2(emu, isl_ar, end_ar, half, v).to_bits(),
            add_half(v, half).to_bits(),
            "add_half(reg) v={v} half={half}"
        );
        // memory forms: add_half via fadd dword[half]; sub_half via fsub dword[half] (v on st0)
        assert_eq!(
            run_mem(emu, isl_am, end_am, v, half as f32).to_bits(),
            add_half(v, f64::from(half as f32)).to_bits(),
            "add_half(mem) v={v} half={half}"
        );
        assert_eq!(
            run_mem(emu, isl_sm, end_sm, v, half as f32).to_bits(),
            sub_half(v, f64::from(half as f32)).to_bits(),
            "sub_half(mem) v={v} half={half}"
        );
        *total += 4;
    };
    let vprobes = [0.0f64, 1.0, -1.0, 0.5, -0.5, 123.456, -123.456, 1e6];
    let hprobes = [0.0f64, 0.5, -0.5, 1.0, 100.0];
    for &v in &vprobes {
        for &h in &hprobes {
            check_half(&mut emu, &mut total, v, h);
        }
    }

    // --- coord_sum / range_q (the fadd that builds P/Q) — exercised through the mem fadd island.
    let check_coord = |emu: &mut Emu, total: &mut u32, c: f32, off: f32| {
        // coord_sum = c + off (f64). Feed v=c (st0), m32=off; fadd dword -> c + off.
        assert_eq!(
            run_mem(emu, isl_am, end_am, f64::from(c), off).to_bits(),
            coord_sum(c, off).to_bits(),
            "coord_sum c={c} off={off}"
        );
        // range_q narrows the same sum to f32.
        assert_eq!(
            (run_mem(emu, isl_am, end_am, f64::from(c), off) as f32).to_bits(),
            range_q(c, off).to_bits(),
            "range_q c={c} off={off}"
        );
        *total += 2;
    };
    let cprobes = [0.0f32, 1.0, -1.0, 0.25, 100.5, -100.5, 1e7];
    for &c in &cprobes {
        for &o in &cprobes {
            check_coord(&mut emu, &mut total, c, o);
        }
    }

    // --- scale_thresh (fmul dword [0x807a3c]) -------------------------------------------------
    let run_thr = |emu: &mut Emu, scale: f32| -> f64 {
        emu.write(in0, &f64::from(scale).to_bits().to_le_bytes());
        emu.run(isl_th, end_th);
        let r = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(r.try_into().unwrap()))
    };
    for &scale in &[0.0f32, 1.0, -1.0, 16.0, 256.0, 0.5, 1024.0] {
        assert_eq!(
            run_thr(&mut emu, scale).to_bits(),
            scale_thresh(scale).to_bits(),
            "scale_thresh scale={scale}"
        );
        total += 1;
    }

    // --- light_edge: the two products (fmul st,st(1) / fmul st,st(2)) + fabs, composed -------
    // fmul st,st(1) (d8 c9): st0·st1. fmul st,st(2) (d8 ca): st0·st2. We diff the products via the
    // two_op island (d8 c9) and a 3-deep island (d8 ca); fabs via a 1-op island. The full
    // light_edge composition is then checked against the assembled scalar pieces.
    let (isl_m1, end_m1, ar_m1) = two_op(&mut emu, &[0xd8, 0xc9]); // st0·st1
    for src in [0x4eb3d3u64, 0x4eb456] {
        assert_eq!(
            emu.read(ar_m1, 2),
            emu.read(src, 2),
            "fmul st,st(1) island != src @{src:#x}"
        );
    }
    // 3-deep: fld qword[in1](->st2 after two more) ... we need st0·st2. Build: fld qword[in1];
    // fld qword[in1]; fld qword[in0]; <d8 ca = fmul st,st(2)>; fstp qword. st0=in0, st1=in1, st2=in1.
    let isl3 = {
        let mut prelude = fld_qword(in1);
        prelude.extend_from_slice(&fld_qword(in1));
        prelude.extend_from_slice(&fld_qword(in0));
        island(&mut emu, &prelude, &[0xd8, 0xca], &fstp_qword(out))
    };
    for src in [0x4eb3e1u64, 0x4eb464] {
        assert_eq!(
            emu.read(isl3.2, 2),
            emu.read(src, 2),
            "fmul st,st(2) island != src @{src:#x}"
        );
    }
    let run3 = |emu: &mut Emu, a: f64, c2: f64| -> f64 {
        emu.write(in0, &a.to_bits().to_le_bytes()); // st0
        emu.write(in1, &c2.to_bits().to_le_bytes()); // st1 & st2
        emu.run(isl3.0, isl3.1);
        let r = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(r.try_into().unwrap()))
    };
    // fabs island (d9 e1): fld qword [in0]; <d9 e1>; fstp qword
    let (isl_ab, end_ab, ar_ab) =
        island(&mut emu, &fld_qword(in0), &[0xd9, 0xe1], &fstp_qword(out));
    for src in [0x4eb3efu64, 0x4eb405, 0x4eb472, 0x4eb488] {
        assert_eq!(
            emu.read(ar_ab, 2),
            emu.read(src, 2),
            "fabs island != src @{src:#x}"
        );
    }
    let run_abs = |emu: &mut Emu, v: f64| -> f64 {
        emu.write(in0, &v.to_bits().to_le_bytes());
        emu.run(isl_ab, end_ab);
        let r = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(r.try_into().unwrap()))
    };

    let check_light = |emu: &mut Emu,
                       total: &mut u32,
                       scale: f32,
                       mxk: f32,
                       myk: f32,
                       cx: f32,
                       cy: f32,
                       px: f32,
                       py: f32| {
        let s = f64::from(scale);
        // mx = marker·scale via fmul st,st(1): st0=marker, st1=scale -> marker·scale.
        let mx = run2(emu, isl_m1, end_m1, f64::from(mxk), s);
        assert_eq!(mx.to_bits(), (f64::from(mxk) * s).to_bits(), "mx product");
        // my = marker·scale via fmul st,st(2), then narrowed to f32 at fstp dword.
        let my_f64 = run3(emu, f64::from(myk), s);
        assert_eq!(
            my_f64.to_bits(),
            (f64::from(myk) * s).to_bits(),
            "my product"
        );
        let my = my_f64 as f32;
        // ex = |center_x − (mx + cx)| ; the inner add via fadd dword, the sub via fsubr dword, then fabs.
        let mx_cx = run_mem(emu, isl_am, end_am, mx, cx); // mx + cx (mx on st0)
        let inner_x = run_mem(emu, isl_rm, end_rm, mx_cx, px); // fsubr dword [px] = px − (mx+cx)
        let ex = run_abs(emu, inner_x);
        let my_cy = run_mem(emu, isl_am, end_am, f64::from(my), cy);
        let inner_y = run_mem(emu, isl_rm, end_rm, my_cy, py);
        let ey = run_abs(emu, inner_y);
        let want = light_edge(scale, mxk, myk, cx, cy, px, py);
        assert_eq!(
            (ex.to_bits(), ey.to_bits()),
            (want.0.to_bits(), want.1.to_bits()),
            "light_edge scale={scale} m=({mxk},{myk}) c=({cx},{cy}) p=({px},{py})"
        );
        *total += 1;
    };
    let lp = [0.0f32, 1.0, -1.0, 0.5, 2.5, -2.5, 10.0];
    for &scale in &[1.0f32, 16.0, 0.5, 100.0] {
        for &mxk in &lp {
            for &myk in &lp {
                check_light(&mut emu, &mut total, scale, mxk, myk, 0.5, -0.5, 1.0, -1.0);
            }
        }
    }
    let mut s = 0x4eb0_c000u32;
    for _ in 0..2000 {
        let scale = rngf(&mut s, 0.0, 64.0) as f32;
        let mxk = rngf(&mut s, -4.0, 4.0) as f32;
        let myk = rngf(&mut s, -4.0, 4.0) as f32;
        let cx = rngf(&mut s, -2.0, 2.0) as f32;
        let cy = rngf(&mut s, -2.0, 2.0) as f32;
        let px = rngf(&mut s, -8.0, 8.0) as f32;
        let py = rngf(&mut s, -8.0, 8.0) as f32;
        check_light(&mut emu, &mut total, scale, mxk, myk, cx, cy, px, py);
    }

    let _ = (isl_rm, end_rm);
    eprintln!("glue_4eb0c0_range_and_light: {total} bit-exact (range fadd/fsub/fsubr + light fmul/fadd/fsubr/fabs islands, byte/opcode-verified)");
}
