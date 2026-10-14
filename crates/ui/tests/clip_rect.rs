//! Differential tests of the arithmetic of the clip-rectangle functions `0x509520`, `0x509720`,
//! `0x509dd0`, `0x509ec0` and `0x509fe0`, each run against `WoW.exe` in the emulator.
//!
//! Strategy per row (preference order):
//! * `0x509720` — owned-arith **slice** `0x509772..0x50978a` (`fld;fadd;fmul[0.5];fstp[edi]`): the
//!   fadd + fmul land in one diffed slice, the result read back from `[edi]`.
//! * `0x509dd0` — **whole-fn `call`** (best): both `fsub`+`fdivr` ratio chains and both `__ftol`
//!   truncations are covered by construction; the `0x41ae60`/`0x41ae70` helpers and `__ftol` run
//!   NATIVELY (pure x87). The two runtime globals `[0x832a44]`/`[0x832a48]` are written per case.
//! * `0x509ec0` — five owned-arith **slices** (A `0x509ee0..efd`, B `0x509f12..f37`, C
//!   `0x509f63..f6c`, D `0x509fa6..faf`, E `0x509fd4..a00a`) covering all ten FP ops; every operand
//!   is an ebp-local or an `[arg]` field synthesized directly, so no callee/vtable FP return is
//!   needed inside any slice.
//!
//! Every in-body FP-arith opcode (the arithmetic opcode set) lands in a diffed slice or the whole-fn
//! call; `fcomp`/`fild`/loads/stores are not arith and ride along for free. Fuzz reads the LCG's
//! HIGH bits. Deterministic boundary probes: the `__ftol`
//! truncate-toward-zero pivot of 0x509dd0; sign/magnitude edges of the halve/sum stores.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::clip_rect::{
    build_struct, clamp_add, clamp_half_extents, clamp_sub, extent_x, extent_y, finalize,
    halve_pair, midpoint, ratio_product,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// The two runtime globals 0x509dd0/0x509ec0's helpers (0x41ae60/0x41ae70) read.
const G44: u64 = 0x832a44;
const G48: u64 = 0x832a48;

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    // ebp with 0x40 below (locals down to ebp-0x3c) and room above (ebp+0xc arg).
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x60;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

// ===========================================================================================
// 0x509720 — midpoint `(m[-0x4] + m[-0xc]) · 0.5` → [edi]. Slice 0x509772..0x50978a covers
// fadd @0x509778 + fmul @0x509780. The interleaved `mov ecx,[ebp+0xc]; movzx eax,al;
// mov [ecx],eax` writes a flag byte through the arg-pointer — point [ebp+0xc] at a scratch cell.
// ===========================================================================================
#[test]
fn glue_509_509720_midpoint() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let edi_cell = emu.alloc(0x10); // [edi] / [edi+4] store target
    let flag_cell = emu.alloc(0x10); // [ebp+0xc] points here (the `mov [ecx],eax` write)

    let run = |emu: &mut Emu, a: f32, b: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EDI, edi_cell);
        emu.set_reg(RegisterX86::EAX, 0); // movzx eax,al — value irrelevant to the FP store
        emu.write_u32(f.ebp - 0x4, a.to_bits());
        emu.write_u32(f.ebp - 0xc, b.to_bits());
        emu.write_u32(f.ebp + 0xc, flag_cell as u32); // arg pointer for `mov [ecx],eax`
        emu.run(0x509772, 0x50978a); // fld;fadd;fmul[0.5];fstp[edi] — halt before fld[ebp-0x10]
        emu.read_f32(edi_cell)
    };

    let mut total = 0u32;
    for (a, b) in [
        (0.0f32, 0.0),
        (1.0, 3.0),
        (-2.0, 2.0),
        (10.0, -4.0),
        (1.0e6, 1.0),
        (-1.0e6, 1.0e6),
        (0.1, 0.2),
        (f32::from_bits(0x4b00_0001), 1.0),
    ] {
        assert_eq!(
            run(&mut emu, a, b).to_bits(),
            midpoint(a, b).to_bits(),
            "midpoint a={a} b={b}"
        );
        total += 1;
    }
    let mut s = 0x5097_2000u32;
    for _ in 0..4000 {
        let a = rng(&mut s, -50000.0, 50000.0);
        let b = rng(&mut s, -50000.0, 50000.0);
        assert_eq!(
            run(&mut emu, a, b).to_bits(),
            midpoint(a, b).to_bits(),
            "midpoint fuzz a={a} b={b}"
        );
        total += 1;
    }
    eprintln!("glue_509_509720_midpoint: {total} bit-exact (fadd + fmul[0.5])");
}

// ===========================================================================================
// 0x509dd0 — whole-fn call. EAX = (trunc(G44/(c-b)) + 1) · (trunc(G48/(a-d)) + 1), with
// a=[esi], b=[esi+4], c=[esi+0xc], d=[esi+8]. fsub ×2 + fdivr ×2 (+ both __ftol) covered by
// construction. The helpers 0x41ae60/0x41ae70 (global·1.0) and __ftol run natively.
// ==threshold probe: feed ratios just below / at an integer (the truncate-toward-zero pivot).
// ===========================================================================================
#[test]
fn glue_509_509dd0_ratio_product() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let st = emu.alloc(0x20); // the 4-float struct (ecx/esi)

    let run = |emu: &mut Emu, g44: f32, g48: f32, a: f32, b: f32, c: f32, d: f32| -> i32 {
        emu.write_u32(G44, g44.to_bits());
        emu.write_u32(G48, g48.to_bits());
        emu.write_u32(st, a.to_bits()); // [esi]
        emu.write_u32(st + 0x4, b.to_bits()); // [esi+4]
        emu.write_u32(st + 0x8, d.to_bits()); // [esi+8]
        emu.write_u32(st + 0xc, c.to_bits()); // [esi+0xc]
        emu.call(0x509dd0, st as u32, 0, &[]) as i32
    };

    let mut total = 0u32;
    // deterministic: ratios that land just below / at / above integers (truncate pivot), both signs,
    // and varied global numerators. Denominators kept nonzero (the binary divides; we never feed 0).
    let cases: &[(f32, f32, f32, f32, f32, f32)] = &[
        // g44, g48, a, b, c, d   →   denom1 = c-b, denom2 = a-d
        (10.0, 10.0, 5.0, 0.0, 5.0, 0.0),  // 10/5=2 ; 10/5=2 → 3*3=9
        (9.999, 10.0, 5.0, 0.0, 5.0, 0.0), // 1.9998→1 ; 2 → 2*3=6
        (10.001, 10.0, 5.0, 0.0, 5.0, 0.0), // 2.0002→2 ; 2
        (-10.0, 10.0, 5.0, 0.0, 5.0, 0.0), // -2 ; 2
        (10.0, -10.0, 5.0, 0.0, 5.0, 0.0), // 2 ; -2
        (7.0, 3.0, 4.0, 1.0, 8.0, 2.0),    // 7/6 ; 3/3
        (1.0, 1.0, 2.0, 1.0, 2.0, 1.0),    // 1/1 ; 1/1
        (100.0, 50.0, 10.0, 2.0, 30.0, 5.0),
        (0.5, 0.25, 1.5, 0.5, 3.5, 1.0),
        (1000.0, 1.0, -3.0, 1.0, 2.0, -7.0),
    ];
    for &(g44, g48, a, b, c, d) in cases {
        assert_eq!(
            run(&mut emu, g44, g48, a, b, c, d),
            ratio_product(g44, g48, a, b, c, d),
            "ratio g44={g44} g48={g48} a={a} b={b} c={c} d={d}"
        );
        total += 1;
    }
    let mut s = 0x509d_d000u32;
    for _ in 0..4000 {
        let g44 = rng(&mut s, -2000.0, 2000.0);
        let g48 = rng(&mut s, -2000.0, 2000.0);
        let a = rng(&mut s, -500.0, 500.0);
        let b = rng(&mut s, -500.0, 500.0);
        // keep denom1 = c-b and denom2 = a-d away from zero by construction.
        let c = b + rng(&mut s, 1.0, 500.0);
        let d = a - rng(&mut s, 1.0, 500.0);
        assert_eq!(
            run(&mut emu, g44, g48, a, b, c, d),
            ratio_product(g44, g48, a, b, c, d),
            "ratio fuzz g44={g44} g48={g48} a={a} b={b} c={c} d={d}"
        );
        total += 1;
    }
    eprintln!("glue_509_509dd0_ratio_product: {total} bit-exact (fsub×2 + fdivr×2 + __ftol×2)");
}

// ===========================================================================================
// 0x509ec0 — five owned-arith slices covering all ten FP ops. Each slice loads ebp-locals (and
// [eax]/[eax+4] for slice B, where eax = [ebp+0xc]) and stores f32 results we read back.
// ===========================================================================================
#[test]
fn glue_509_509ec0_axis_resolve() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let argpair = emu.alloc(0x10); // [ebp+0xc] -> {P0=[eax], P1=[eax+4]}

    // ---- slice A: halve_pair. in: [ebp-0x8]=V1, [ebp-0xc]=V2. out: [ebp-0x4]=h1, [ebp-0x8]=h2.
    // (V1 is consumed before [ebp-0x8] is overwritten with h2 — order-safe.)
    let run_a = |emu: &mut Emu, v1: f32, v2: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x8, v1.to_bits());
        emu.write_u32(f.ebp - 0xc, v2.to_bits());
        emu.run(0x509ee0, 0x509efd); // fld;fmul[0.5];fstp ×2 (halt before call 0x41ae60)
        (emu.read_f32(f.ebp - 0x4), emu.read_f32(f.ebp - 0x8))
    };

    // ---- slice B: build_struct. in: [ebp+0xc]->{P0,P1}, [ebp-0x4]=h1, [ebp-0xc]=v2.
    // out: [ebp-0x28]=s1, [ebp-0x24]=s2, [ebp-0x20]=s3.
    let run_b = |emu: &mut Emu, p0: f32, p1: f32, h1: f32, v2: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(argpair, p0.to_bits()); // [eax]
        emu.write_u32(argpair + 0x4, p1.to_bits()); // [eax+4]
        emu.write_u32(f.ebp + 0xc, argpair as u32); // [ebp+0xc] = pointer
        emu.write_u32(f.ebp - 0x4, h1.to_bits());
        emu.write_u32(f.ebp - 0xc, v2.to_bits());
        emu.run(0x509f12, 0x509f37); // fld..fsub/fadd ×3 stores (halt before call 0x509e20)
        (
            emu.read_f32(f.ebp - 0x28),
            emu.read_f32(f.ebp - 0x24),
            emu.read_f32(f.ebp - 0x20),
        )
    };

    // ---- slice C: extent_x = G44 - h1. in: [ebp-0x10]=G44, [ebp-0x4]=h1. out: [ebp+0xc] scratch.
    let run_c = |emu: &mut Emu, g44: f32, h1: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x10, g44.to_bits());
        emu.write_u32(f.ebp - 0x4, h1.to_bits());
        emu.run(0x509f63, 0x509f6c); // fld[ebp-0x10];fsub[ebp-0x4];fstp[ebp+0xc]
        emu.read_f32(f.ebp + 0xc)
    };

    // ---- slice D: extent_y = G48 - h2. in: [ebp-0x14]=G48, [ebp-0x8]=h2. out: [ebp+0xc] scratch.
    let run_d = |emu: &mut Emu, g48: f32, h2: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x14, g48.to_bits());
        emu.write_u32(f.ebp - 0x8, h2.to_bits());
        emu.run(0x509fa6, 0x509faf); // fld[ebp-0x14];fsub[ebp-0x8];fstp[ebp+0xc]
        emu.read_f32(f.ebp + 0xc)
    };

    // ---- slice E: finalize. in: [ebp-0x1c]=X, [ebp-0x18]=Y, [ebp-0x4]=h1, [ebp-0xc]=v2.
    // out: [ebp-0x38]=r1, [ebp-0x34]=r2, [ebp-0x30]=r3.
    let run_e = |emu: &mut Emu, x: f32, y: f32, h1: f32, v2: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x1c, x.to_bits());
        emu.write_u32(f.ebp - 0x18, y.to_bits());
        emu.write_u32(f.ebp - 0x4, h1.to_bits());
        emu.write_u32(f.ebp - 0xc, v2.to_bits());
        emu.run(0x509fd4, 0x50a00a); // copy + fsub/fsub/fadd ×3 stores (halt before mov eax,[-0x30])
        (
            emu.read_f32(f.ebp - 0x38),
            emu.read_f32(f.ebp - 0x34),
            emu.read_f32(f.ebp - 0x30),
        )
    };

    let mut total = 0u32;
    let probes = [
        0.0f32,
        1.0,
        -1.0,
        2.5,
        -7.25,
        1024.0,
        1.0e6,
        -1.0e6,
        0.1,
        f32::from_bits(0x4b00_0001),
    ];
    // deterministic cross-product over probes for each slice.
    for &p in &probes {
        for &q in &probes {
            // A
            let (ha, hb) = run_a(&mut emu, p, q);
            let (wha, whb) = halve_pair(p, q);
            assert_eq!(ha.to_bits(), wha.to_bits(), "halve h1 v1={p} v2={q}");
            assert_eq!(hb.to_bits(), whb.to_bits(), "halve h2 v1={p} v2={q}");
            total += 2;
            // C
            assert_eq!(
                run_c(&mut emu, p, q).to_bits(),
                extent_x(p, q).to_bits(),
                "extent_x g44={p} h1={q}"
            );
            total += 1;
            // D
            assert_eq!(
                run_d(&mut emu, p, q).to_bits(),
                extent_y(p, q).to_bits(),
                "extent_y g48={p} h2={q}"
            );
            total += 1;
        }
    }
    // B and E take four args — fuzz them (cross-product would be 10^4).
    let mut s = 0x509e_c000u32;
    for _ in 0..3000 {
        let p0 = rng(&mut s, -20000.0, 20000.0);
        let p1 = rng(&mut s, -20000.0, 20000.0);
        let h1 = rng(&mut s, -20000.0, 20000.0);
        let v2 = rng(&mut s, -20000.0, 20000.0);
        let (s1, s2, s3) = run_b(&mut emu, p0, p1, h1, v2);
        let (ws1, ws2, ws3) = build_struct(p0, p1, h1, v2);
        assert_eq!(s1.to_bits(), ws1.to_bits(), "build s1 p0={p0} h1={h1}");
        assert_eq!(s2.to_bits(), ws2.to_bits(), "build s2 p1={p1} v2={v2}");
        assert_eq!(s3.to_bits(), ws3.to_bits(), "build s3 h1={h1} p0={p0}");
        total += 3;

        let x = rng(&mut s, -20000.0, 20000.0);
        let y = rng(&mut s, -20000.0, 20000.0);
        let (r1, r2, r3) = run_e(&mut emu, x, y, h1, v2);
        let (wr1, wr2, wr3) = finalize(x, y, h1, v2);
        assert_eq!(r1.to_bits(), wr1.to_bits(), "final r1 x={x} h1={h1}");
        assert_eq!(r2.to_bits(), wr2.to_bits(), "final r2 y={y} v2={v2}");
        assert_eq!(r3.to_bits(), wr3.to_bits(), "final r3 x={x} h1={h1}");
        total += 3;
    }
    // also fuzz A/C/D more widely.
    let mut s = 0x509e_c0a0u32;
    for _ in 0..2000 {
        let p = rng(&mut s, -50000.0, 50000.0);
        let q = rng(&mut s, -50000.0, 50000.0);
        let (ha, hb) = run_a(&mut emu, p, q);
        let (wha, whb) = halve_pair(p, q);
        assert_eq!(ha.to_bits(), wha.to_bits(), "halve fuzz h1 v1={p} v2={q}");
        assert_eq!(hb.to_bits(), whb.to_bits(), "halve fuzz h2 v1={p} v2={q}");
        assert_eq!(
            run_c(&mut emu, p, q).to_bits(),
            extent_x(p, q).to_bits(),
            "extent_x fuzz g44={p} h1={q}"
        );
        assert_eq!(
            run_d(&mut emu, p, q).to_bits(),
            extent_y(p, q).to_bits(),
            "extent_y fuzz g48={p} h2={q}"
        );
        total += 4;
    }
    eprintln!("glue_509_509ec0_axis_resolve: {total} bit-exact (fmul×2 + fsub×6 + fadd×2)");
}

// ===========================================================================================
// 0x509520 — CRect viewport clamp. Four owned-arith slices (the clamp ROUTING between them is
// integer cmov on fcomp + the 0x509720/0x509e20 callees — composition, not owned arith). Each
// slice synthesizes its operands directly; the half-width reload (slice A) and the single
// sub/add ops (B/C/D) are diffed bit-exact.
//   A 0x509529..0x509558  half-extents (fsub×2,fabs×2,fmul×2)
//   B 0x509598..0x50959e  G44 − hh   |  C 0x5095db..0x5095e1  G48 − hw
//   D 0x509611..0x50963a  X−hh, Y−W, X+hh
// ===========================================================================================
#[test]
fn glue_509_509520_viewport_clamp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let rect = emu.alloc(0x10); // esi: CRect (l,t,r,b)
    let edx_cell = emu.alloc(0x10); // slice D: [edx]/[edx+4]/[edx+8] integer stores

    // slice A — half-extents. in: [esi]=l,[esi+4]=t,[esi+8]=r,[esi+0xc]=b. out: W,hh,hw.
    let run_a = |emu: &mut Emu, l: f32, t: f32, r: f32, b: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, rect);
        emu.set_reg(RegisterX86::EDI, 0); // pushed @0x50952c
        emu.set_reg(RegisterX86::ECX, 0); // mov edi,ecx
        emu.write_u32(rect, l.to_bits());
        emu.write_u32(rect + 4, t.to_bits());
        emu.write_u32(rect + 8, r.to_bits());
        emu.write_u32(rect + 0xc, b.to_bits());
        emu.run(0x509529, 0x509558); // halt before call 0x41ae60
        (
            emu.read_f32(f.ebp - 0x10),
            emu.read_f32(f.ebp - 0x4),
            emu.read_f32(f.ebp - 0x8),
        )
    };
    // slice B — G44 − hh. in: [ebp-0xc]=G44,[ebp-0x4]=hh. out: [ebp-0xc].
    let run_b = |emu: &mut Emu, g44: f32, hh: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0xc, g44.to_bits());
        emu.write_u32(f.ebp - 0x4, hh.to_bits());
        emu.run(0x509598, 0x5095a1); // fld;fsub;fstp[ebp-0xc] (until = next instr after fstp)
        emu.read_f32(f.ebp - 0xc)
    };
    // slice C — G48 − hw. in: [ebp-0x14]=G48,[ebp-0x8]=hw. out: [ebp-0xc].
    let run_c = |emu: &mut Emu, g48: f32, hw: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x14, g48.to_bits());
        emu.write_u32(f.ebp - 0x8, hw.to_bits());
        emu.run(0x5095db, 0x5095e4); // fld;fsub;fstp[ebp-0xc] (until = next instr after fstp)
        emu.read_f32(f.ebp - 0xc)
    };
    // slice D — X−hh, Y−W, X+hh. in: [ebp-0x20]=X,[ebp-0x1c]=Y,[ebp-0x4]=hh,[ebp-0x10]=W,
    // [ebp-0x30] scratch (copied to [edx]). out: [ebp-0x2c],[ebp-0x28],[ebp-0x24]. edx → cell.
    let run_d = |emu: &mut Emu, x: f32, y: f32, hh: f32, w: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EDX, edx_cell);
        emu.write_u32(f.ebp - 0x20, x.to_bits());
        emu.write_u32(f.ebp - 0x1c, y.to_bits());
        emu.write_u32(f.ebp - 0x4, hh.to_bits());
        emu.write_u32(f.ebp - 0x10, w.to_bits());
        emu.write_u32(f.ebp - 0x30, 0); // copied to [edx] (no arith)
        emu.run(0x509611, 0x50963d); // through fstp[ebp-0x24]
        (
            emu.read_f32(f.ebp - 0x2c),
            emu.read_f32(f.ebp - 0x28),
            emu.read_f32(f.ebp - 0x24),
        )
    };

    let mut total = 0u32;
    let probes = [
        0.0f32,
        1.0,
        -1.0,
        2.5,
        -7.25,
        1024.0,
        1.0e6,
        -1.0e6,
        0.1,
        f32::from_bits(0x4b00_0001),
    ];
    // slice A: deterministic CRect probes (both orientations of l<r / t<b).
    for &p in &probes {
        for &q in &probes {
            let (w, hh, hw) = run_a(&mut emu, p, q, q, p);
            let (ww, whh, whw) = clamp_half_extents(p, q, q, p);
            assert_eq!(w.to_bits(), ww.to_bits(), "W l={p} t={q}");
            assert_eq!(hh.to_bits(), whh.to_bits(), "hh l={p} t={q}");
            assert_eq!(hw.to_bits(), whw.to_bits(), "hw l={p} t={q}");
            // B and C reuse the half-extents as operands (g·1.0 globals synthesized as probes).
            assert_eq!(
                run_b(&mut emu, p, hh).to_bits(),
                clamp_sub(p, hh).to_bits(),
                "B g44={p} hh={hh}"
            );
            assert_eq!(
                run_c(&mut emu, q, hw).to_bits(),
                clamp_sub(q, hw).to_bits(),
                "C g48={q} hw={hw}"
            );
            total += 5;
        }
    }
    // slice D fuzz (four args).
    let mut s = 0x5095_2000u32;
    for _ in 0..4000 {
        let x = rng(&mut s, -50000.0, 50000.0);
        let y = rng(&mut s, -50000.0, 50000.0);
        let hh = rng(&mut s, -50000.0, 50000.0);
        let w = rng(&mut s, -50000.0, 50000.0);
        let (d0, d1, d2) = run_d(&mut emu, x, y, hh, w);
        assert_eq!(
            d0.to_bits(),
            clamp_sub(x, hh).to_bits(),
            "D X-hh x={x} hh={hh}"
        );
        assert_eq!(d1.to_bits(), clamp_sub(y, w).to_bits(), "D Y-W y={y} w={w}");
        assert_eq!(
            d2.to_bits(),
            clamp_add(x, hh).to_bits(),
            "D X+hh x={x} hh={hh}"
        );
        total += 3;
    }
    // half-extents wide fuzz (the |.| and the W-reload halving).
    let mut s = 0x5095_20a0u32;
    for _ in 0..4000 {
        let l = rng(&mut s, -50000.0, 50000.0);
        let t = rng(&mut s, -50000.0, 50000.0);
        let r = rng(&mut s, -50000.0, 50000.0);
        let b = rng(&mut s, -50000.0, 50000.0);
        let (w, hh, hw) = run_a(&mut emu, l, t, r, b);
        let (ww, whh, whw) = clamp_half_extents(l, t, r, b);
        assert_eq!(w.to_bits(), ww.to_bits(), "W fuzz l={l} r={r}");
        assert_eq!(hh.to_bits(), whh.to_bits(), "hh fuzz t={t} b={b}");
        assert_eq!(hw.to_bits(), whw.to_bits(), "hw fuzz l={l} r={r}");
        total += 3;
    }
    eprintln!("glue_509_509520_viewport_clamp: {total} bit-exact (half-extents + bound-subs + corner stores)");
}
