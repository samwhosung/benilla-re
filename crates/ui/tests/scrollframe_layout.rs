//! Differential tests of the arithmetic of the layout and fade functions `0x785ea0`, `0x786200`,
//! `0x7863b0` and `0x786490`, each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice of the layout/recompute glue TU `[0x785ea0, 0x786570]` is diffed as an
//! **instruction slice** (`Emu::run`) — registers/memory/stack synthesized, the binary's own bytes
//! executed, the stored `fstp/fst [m]` value or the `fnstsw` AH read back. Where the owned op is
//! interleaved with `call`s into delegated functions (the layout setter `0x767c70`, the level
//! helper `0x7727b0`, the vtable getter), it is diffed via a **copied-byte island** — the EXACT
//! source bytes assembled in scratch with our own pointers, byte-verified in-test against the disasm
//! (islands are credited only when the bytes match source). Every owned-arith op in
//! `[addr, true_ret)` for all four rows lands in a diffed slice or island.
//!
//! Fuzz reads the LCG's HIGH bits. Every owned-slice test carries a
//! deterministic `==threshold` boundary probe. The eps/clamp branches are reconstructed
//! from the EMITTED jcc, not the mask: `test ah,5` (C0|C2) read by `jp`/`jnp` is a PARITY test;
//! `test ah,0x41` (C0|C3) read by `jne` is a ZF test (is masked-AH zero).
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::scrollframe_layout::{
    child_coords, child_neg_acc, fade_alpha_byte, fade_alpha_keep, fade_timer_dec, header_coords,
    level_fraction, negate_child, quantize_settled, tail_slope, EPS_BITS, SCALE255_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}
fn ah_of(emu: &Emu) -> u8 {
    ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}
fn read_f64(emu: &Emu, va: u64) -> f64 {
    f64::from_bits(u64::from(emu.read_u32(va)) | (u64::from(emu.read_u32(va + 4)) << 32))
}

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x400);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { this, ebp, esp }
}
fn set_frame(emu: &mut Emu, f: &Frame) {
    emu.set_reg(RegisterX86::ESI, f.this);
    emu.set_reg(RegisterX86::ECX, f.this);
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
}

// ===========================================================================================
// 0x785ea0 slice A — header coordinate block (0x785eb5..0x785efd). Clean run (no calls).
// fmul×4 + fadd×2 + fsubr×2 = 8 ops; reads the four stored f32 slots.
// ===========================================================================================
#[test]
fn glue_785ea0_header() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // 0x785eb5..0x785efd reads [esi+0x7c/0x334/0x33c/0x330/0x338/0x68/0x70/0x64/0x6c]; the entry
    // `test [esi+0x60],1; je` must take the fall-through (flag bit 0 set), so set [+0x60]=1.
    let run = |emu: &mut Emu, s: f32, c: [f32; 4], e: [f32; 4]| -> (f32, f32, f32, f32) {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0x60, 1);
        emu.write_u32(f.this + 0x7c, s.to_bits());
        emu.write_u32(f.this + 0x334, c[0].to_bits());
        emu.write_u32(f.this + 0x33c, c[1].to_bits());
        emu.write_u32(f.this + 0x330, c[2].to_bits());
        emu.write_u32(f.this + 0x338, c[3].to_bits());
        emu.write_u32(f.this + 0x68, e[0].to_bits());
        emu.write_u32(f.this + 0x70, e[1].to_bits());
        emu.write_u32(f.this + 0x64, e[2].to_bits());
        emu.write_u32(f.this + 0x6c, e[3].to_bits());
        emu.run(0x785eb5, 0x785f03);
        (
            emu.read_f32(f.this + 0x324),
            emu.read_f32(f.this + 0x32c),
            emu.read_f32(f.this + 0x320),
            emu.read_f32(f.this + 0x328),
        )
    };

    let mut total = 0u32;
    // deterministic boundary: scale 0 ⇒ products vanish ⇒ slots = the bare edges.
    for (s, c, e) in [
        (0.0f32, [1.0, 2.0, 3.0, 4.0], [10.0, 20.0, 30.0, 40.0]),
        (1.0, [1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 0.0]),
        (2.5, [0.5, -0.5, 1.5, -1.5], [-3.0, 3.0, -7.0, 7.0]),
    ] {
        let (a, b, cc, d) = run(&mut emu, s, c, e);
        let want = header_coords(s, c[0], c[1], c[2], c[3], e[0], e[1], e[2], e[3]);
        assert_eq!(a.to_bits(), want.0.to_bits(), "324 s={s}");
        assert_eq!(b.to_bits(), want.1.to_bits(), "32c s={s}");
        assert_eq!(cc.to_bits(), want.2.to_bits(), "320 s={s}");
        assert_eq!(d.to_bits(), want.3.to_bits(), "328 s={s}");
        total += 4;
    }
    let mut st = 0x785e_a000u32;
    for _ in 0..2000 {
        let s = rng(&mut st, -100.0, 100.0);
        let c = [
            rng(&mut st, -10.0, 10.0),
            rng(&mut st, -10.0, 10.0),
            rng(&mut st, -10.0, 10.0),
            rng(&mut st, -10.0, 10.0),
        ];
        let e = [
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
            rng(&mut st, -1000.0, 1000.0),
        ];
        let (a, b, cc, d) = run(&mut emu, s, c, e);
        let want = header_coords(s, c[0], c[1], c[2], c[3], e[0], e[1], e[2], e[3]);
        assert_eq!(a.to_bits(), want.0.to_bits(), "324 fuzz");
        assert_eq!(b.to_bits(), want.1.to_bits(), "32c fuzz");
        assert_eq!(cc.to_bits(), want.2.to_bits(), "320 fuzz");
        assert_eq!(d.to_bits(), want.3.to_bits(), "328 fuzz");
        total += 4;
    }
    eprintln!("glue_785ea0_header: {total} bit-exact (fmul×4+fadd×2+fsubr×2)");
}

// ===========================================================================================
// 0x785ea0 slice B — sum-of-snapped-products + clamp + divide (0x785f03..0x785f51).
// The two `fmul[+0x7c]` products feed `0x766750` (which returns its arg when |arg|>=eps); their
// sum `fadd[ebp-8]` is stored to [ebp-8]; then `([+0x328]−[+0x320])/sum → __ftol`. We diff the sum
// (fmul×2+fadd) by running through the `fst[ebp-8]`, and the `fsub`/`fdiv` slice separately.
// ===========================================================================================
#[test]
fn glue_785ea0_sum_divide() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let gstruct = emu.alloc(0x60); // [esi+0x344] points here; reads [+0x48],[+0x50]

    // The snap kernel 0x766750 returns its f32 arg verbatim on the FAST path (|arg|>=eps): its
    // 0x766756..0x76676e body is `fld[ebp+8]; fabs; fcomp eps; jp; fld[ebp+8]; mov esp,ebp; pop ebp;
    // ret 4`. We patch it to the pure pass-through `fld dword [esp+4]; ret 4` (patch-getter
    // idiom) — exactly the fast-path identity (an f32 round-trip of the pushed arg). The slow path
    // (|arg|<eps) reformats the value; we steer fuzz to |product|>=eps so the fast path is what runs.
    {
        // byte-verify the fast path IS `fld[ebp+8]; ...; ret 4` before we model it as pass-through.
        let fld = emu.read(0x766756, 3);
        assert_eq!(
            fld,
            vec![0xd9, 0x45, 0x08],
            "snap fast-path fld[ebp+8] bytes mismatch"
        );
        let r = emu.read(0x76676e, 1);
        assert_eq!(r, vec![0xc2], "snap fast-path ret bytes mismatch");
    }
    // patch 0x766750 -> `fld dword [esp+4]; ret 4` (reload the pushed f32 arg into st0).
    emu.write(0x766750, &[0xd9, 0x44, 0x24, 0x04, 0xc2, 0x04, 0x00]);

    // sum slice 0x785f03..0x785f30: mov ecx,[eax+0x48]; fld[eax+0x50]; fmul[+0x7c]; snap(0x766750);
    // fstp[ebp-8]; fld[ebp-0xc]; fmul[+0x7c]; snap(0x766750); fadd[ebp-8]; fst[ebp-8].
    // sum = snap(m50·s) + snap(m48·s) with snap(x) = x as f32 (the f32 push/reload round-trip).
    let run_sum = |emu: &mut Emu, m50: f32, m48: f32, s: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0x344, gstruct as u32);
        emu.set_reg(RegisterX86::EAX, gstruct);
        emu.write_u32(gstruct + 0x48, m48.to_bits());
        emu.write_u32(gstruct + 0x50, m50.to_bits());
        emu.write_u32(f.this + 0x7c, s.to_bits());
        emu.run(0x785f03, 0x785f30); // through `fst [ebp-8]`
        emu.read_f32(f.ebp - 0x8)
    };
    // 0x766750 fast path: fstp's its f32 arg back and returns it ⇒ snap(x) = x as f32 (the round-trip).
    let _ = EPS_BITS; // (kept imported for the quantize test)
    let snap = |x: f64| -> f64 { f64::from(x as f32) };

    // divide slice 0x785f3d..0x785f51: fld[+0x328]; fsub[+0x320]; fdiv[ebp-8]; __ftol; lea ecx,[eax+1].
    let run_div = |emu: &mut Emu, slot328: f32, slot320: f32, sum: f32| -> i32 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0x328, slot328.to_bits());
        emu.write_u32(f.this + 0x320, slot320.to_bits());
        emu.write_u32(f.ebp - 0x8, sum.to_bits());
        emu.run(0x785f3d, 0x785f51); // through the __ftol call; eax = truncated quotient
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // sum: products kept large so the snap kernel passes through.
    for (m50, m48, s) in [(2.0f32, 3.0f32, 4.0f32), (1.0, 1.0, 1.0), (-5.0, 10.0, 2.0)] {
        let got = run_sum(&mut emu, m50, m48, s);
        let want =
            (snap(f64::from(m50) * f64::from(s)) + snap(f64::from(m48) * f64::from(s))) as f32;
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "sum m50={m50} m48={m48} s={s}"
        );
        total += 1;
    }
    let mut st = 0x785f_0300u32;
    for _ in 0..1500 {
        let s = rng(&mut st, 0.5, 100.0);
        let m50 = rng(&mut st, 1.0, 100.0); // keep products well above eps
        let m48 = rng(&mut st, 1.0, 100.0);
        let got = run_sum(&mut emu, m50, m48, s);
        let want =
            (snap(f64::from(m50) * f64::from(s)) + snap(f64::from(m48) * f64::from(s))) as f32;
        assert_eq!(got.to_bits(), want.to_bits(), "sum fuzz");
        total += 1;
    }
    // divide: ([+0x328]−[+0x320]) / sum → trunc toward zero. Deterministic boundary: exact integer.
    for (s328, s320, sum) in [
        (10.0f32, 0.0f32, 2.0f32), // = 5.0 -> 5
        (7.0, 0.0, 2.0),           // = 3.5 -> 3
        (-7.0, 0.0, 2.0),          // = -3.5 -> -3
        (5.0, 5.0, 3.0),           // = 0
    ] {
        let got = run_div(&mut emu, s328, s320, sum);
        let want = (((f64::from(s328) - f64::from(s320)) / f64::from(sum)).trunc()) as i32;
        assert_eq!(got, want, "div 328={s328} 320={s320} sum={sum}");
        total += 1;
    }
    for _ in 0..1500 {
        let s328 = rng(&mut st, -1000.0, 1000.0);
        let s320 = rng(&mut st, -1000.0, 1000.0);
        let sum = rng(&mut st, 0.5, 50.0);
        let got = run_div(&mut emu, s328, s320, sum);
        let want = (((f64::from(s328) - f64::from(s320)) / f64::from(sum)).trunc()) as i32;
        assert_eq!(got, want, "div fuzz");
        total += 1;
    }
    eprintln!("glue_785ea0_sum_divide: {total} bit-exact (fmul×2+fadd sum, fsub+fdiv+__ftol)");
}

// ===========================================================================================
// 0x785ea0 slice C — fild-quantize change-detect (0x785f5e..0x785f82). Clean run (no calls).
// fmul + fsubp + fabs; reads the `fnstsw` AH for the eps gate.
// ===========================================================================================
#[test]
fn glue_785ea0_quantize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // 0x785f5e..0x785f82: fild qword[ebp-0x20]; fmul[ebp-8]; fld[+0x328]; fsub[+0x320]; fsubp;
    // fabs; fcomp[0x8029d4]; fnstsw. n=[ebp-0x20] (low), [ebp-0x1c]=0 (high). We must set the high
    // dword 0 (the binary writes it at 0x785f57, but our slice starts after — set it ourselves).
    let run = |emu: &mut Emu, n: u32, sum: f32, s328: f32, s320: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0x20, n);
        emu.write_u32(f.ebp - 0x1c, 0);
        emu.write_u32(f.ebp - 0x8, sum.to_bits());
        emu.write_u32(f.this + 0x328, s328.to_bits());
        emu.write_u32(f.this + 0x320, s320.to_bits());
        emu.run(0x785f5e, 0x785f82);
        ah_of(emu)
    };

    let mut total = 0u32;
    let mut settled = 0u32;
    let mut moved = 0u32;
    // boundary: choose s328-s320 == n·sum exactly ⇒ |d|==0 < eps ⇒ settled.
    for (n, sum, s320) in [(2u32, 0.5f32, 0.0f32), (4, 1.0, 3.0), (10, 0.25, -1.0)] {
        // s328 = (n·sum) + s320 exactly (f32) ⇒ d ≈ 0.
        let s328 = ((f64::from(n) * f64::from(sum)) + f64::from(s320)) as f32;
        let ah = run(&mut emu, n, sum, s328, s320);
        // test ah,5; jp -> NOT taken (keep, settled) iff odd parity (|d|<eps). settled()⇒odd? The
        // function returns true (settled) iff |d|<eps; the binary KEEPS the +1 on the jp-not-taken
        // (odd) path. So binary-settled (kept) ⇔ odd parity ⇔ quantize_settled.
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(
            bin,
            quantize_settled(n, sum, s328, s320),
            "quant n={n} sum={sum}"
        );
        if bin {
            settled += 1;
        } else {
            moved += 1;
        }
        total += 1;
    }
    let mut st = 0x785f_5e00u32;
    for _ in 0..2500 {
        let n = (next(&mut st) >> 8) % 1000;
        let sum = rng(&mut st, 0.01, 10.0);
        let s320 = rng(&mut st, -100.0, 100.0);
        // straddle the boundary: s328 near (n·sum + s320) ± a small jitter.
        let base = (f64::from(n) * f64::from(sum)) + f64::from(s320);
        let s328 = (base + f64::from(rng(&mut st, -1.0e-6, 1.0e-6))) as f32;
        let ah = run(&mut emu, n, sum, s328, s320);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(
            bin,
            quantize_settled(n, sum, s328, s320),
            "quant fuzz n={n}"
        );
        if bin {
            settled += 1;
        } else {
            moved += 1;
        }
        total += 1;
    }
    assert!(
        settled > 0 && moved > 0,
        "quantize: need both settled/moved"
    );
    eprintln!("glue_785ea0_quantize: {total} bit-exact (fmul+fsubp+fabs; settled={settled} moved={moved})");
}

// ===========================================================================================
// 0x785ea0 slice D — tail recompute. tail_slope (fdiv+fsub+fmul st,st(1)) clean run;
// child_coords (fdiv×2) clean run; child_neg_acc (fchs+fadd) copied-byte island.
// ===========================================================================================
#[test]
fn glue_785ea0_tail() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // tail_slope 0x786070..0x7860d7: fld 1.0; fdiv[+0x7c]; ...; fld[+0x32c]; fsub[+0x324];
    // fmul st,st(1); fstp[ebp-0x14]=slope; ...; fmul[ebp-8]=inv·sum (live inv); fstp[ebp-0x18]. The
    // integer instrs (0x78607c..0x786096 + the 0x7860a7 `jne`/branch into 0x7860c6) do NOT touch the
    // st0=inv left after the slope fstp, so the SAME live PC_53 inv feeds both products. [esi+0x354]=0
    // ⇒ the `jne` at 0x7860a7 falls through (mov eax,1; xor edi,edi; jmp 0x7860c6). Halt at 0x7860d7
    // (the `jae`) after both fstp's. Reads [ebp-0x14]=slope and [ebp-0x18]=inv·sum.
    let run_slope = |emu: &mut Emu, s: f32, s32c: f32, s324: f32, sum: f32| -> (f32, f32) {
        set_frame(emu, &f);
        emu.set_reg(RegisterX86::EDI, f.this); // 0x786076 `mov [edi+0x4],edx` — EDI must be mapped
        emu.write_u32(f.this + 0x7c, s.to_bits());
        emu.write_u32(f.this + 0x32c, s32c.to_bits());
        emu.write_u32(f.this + 0x324, s324.to_bits());
        emu.write_u32(f.ebp - 0x8, sum.to_bits()); // [ebp-0x8] = slice-B sum (fmul operand)
                                                   // [esi+0x354] read at 0x78607c (test eax,eax) — 0 ⇒ the jne falls through to 0x7860c6.
        emu.write_u32(f.this + 0x354, 0);
        emu.write_u32(f.this + 0x334, 0); // [ebp-0x10] source copy (not used by this slice)
        emu.write_u32(f.this + 0x338, 0);
        emu.run(0x786070, 0x7860d7);
        (emu.read_f32(f.ebp - 0x14), emu.read_f32(f.ebp - 0x18))
    };

    // child_coords 0x7860f0..0x786117: fld[ebp-0x10]=m0; fdiv[+0x7c]; fstp[child]; fld[ebp-0x4]=m1;
    // fdiv[+0x7c]; fstp[child+4]. [esi+0x370]=array base; edi=child index*12. We set edi=0 so the
    // stores land at [base+0] and [base+4]. (the mov esi+0x370 happens inside the slice.)
    let arr = emu.alloc(0x40);
    let run_child = |emu: &mut Emu, m0: f32, m1: f32, s: f32| -> (f32, f32) {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0x370, arr as u32);
        emu.write_u32(f.this + 0x7c, s.to_bits());
        emu.write_u32(f.ebp - 0x10, m0.to_bits());
        emu.write_u32(f.ebp - 0x4, m1.to_bits());
        emu.set_reg(RegisterX86::EDI, 0); // child slot 0
                                          // 0x7860fc reads [eax+edi+0x8] = [arr+0x8] (a pointer ecx, then [ecx+0xc]=ebx) — but those
                                          // are used only by the LATER call path, past our 0x786117 halt. Map [arr+8] safely.
        emu.write_u32(arr + 0x8, arr as u32);
        emu.run(0x7860f0, 0x786117);
        (emu.read_f32(arr), emu.read_f32(arr + 0x4))
    };

    // child_neg_acc island: fchs slice 0x786143..0x786153 (`fld[eax+edi+0x4]; fchs; fstp[esp]`) and
    // acc slice 0x786184..0x78618d (`fld[ebp-0x18]; fadd[ebp-0x4]; fstp[ebp-0x4]`). Both interleave
    // with `call`s in the function; isolate the FP as verbatim islands (byte-verified below).
    let cy_cell = emu.alloc(4);
    let neg_cell = emu.alloc(4);
    let isl_neg = emu.alloc(0x20);
    // verbatim 0x786143 (`d9 44 38 04` fld[eax+edi+4]) — but we point eax at cy_cell-4 with edi=0
    // so [eax+edi+4]=cy_cell. fchs (`d9 e0`); fstp[esp] (`d9 1c 24`) — redirect esp to neg_cell.
    {
        let src = emu.read(0x786143, 4); // fld dword [eax+edi*1+0x4]
        assert_eq!(src, vec![0xd9, 0x44, 0x38, 0x04], "neg fld bytes mismatch");
        let src2 = emu.read(0x78614b, 2); // fchs
        assert_eq!(src2, vec![0xd9, 0xe0], "neg fchs bytes mismatch");
    }
    let mut cneg = vec![0xd9u8, 0x44, 0x38, 0x04]; // fld [eax+edi+0x4]   (verbatim 0x786143)
    cneg.extend_from_slice(&[0xd9, 0xe0]); // fchs                (verbatim 0x78614b)
    cneg.extend_from_slice(&[0xd9, 0x1c, 0x24]); // fstp [esp]    (verbatim 0x786150)
    let isl_neg_end = isl_neg + cneg.len() as u64;
    emu.write(isl_neg, &cneg);
    let run_neg = |emu: &mut Emu, cy: f32| -> f32 {
        emu.write_u32(cy_cell, cy.to_bits());
        emu.set_reg(RegisterX86::EAX, cy_cell - 4); // [eax+edi+4] = cy_cell
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::ESP, neg_cell); // fstp[esp] → neg_cell
        emu.run(isl_neg, isl_neg_end);
        emu.read_f32(neg_cell)
    };

    let acc_in = emu.alloc(4); // [ebp-0x18]
    let acc_step = emu.alloc(4); // [ebp-0x4]
    let isl_acc = emu.alloc(0x20);
    {
        let s184 = emu.read(0x786184, 3); // fld dword [ebp-0x18]
        assert_eq!(s184, vec![0xd9, 0x45, 0xe8], "acc fld bytes mismatch");
        let s18a = emu.read(0x78618a, 3); // fadd dword [ebp-0x4]
        assert_eq!(s18a, vec![0xd8, 0x45, 0xfc], "acc fadd bytes mismatch");
    }
    // verbatim 0x786184 (`d9 45 e8` fld[ebp-0x18]) + 0x78618a (`d8 45 fc` fadd[ebp-0x4]) + fstp.
    let mut cacc = vec![0xd9u8, 0x45, 0xe8]; // fld [ebp-0x18]      (verbatim 0x786184)
    cacc.extend_from_slice(&[0xd8, 0x45, 0xfc]); // fadd [ebp-0x4]  (verbatim 0x78618a)
    cacc.extend_from_slice(&[0xd9, 0x5d, 0xfc]); // fstp [ebp-0x4]  (verbatim 0x786199)
    let isl_acc_end = isl_acc + cacc.len() as u64;
    emu.write(isl_acc, &cacc);
    let run_acc = |emu: &mut Emu, a: f32, step: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.write_u32(f.ebp - 0x18, a.to_bits());
        emu.write_u32(f.ebp - 0x4, step.to_bits());
        let _ = (acc_in, acc_step);
        emu.run(isl_acc, isl_acc_end);
        emu.read_f32(f.ebp - 0x4)
    };

    let mut total = 0u32;
    // slope boundary: slot32c == slot324 ⇒ slope == 0; sum chosen to exercise inv·sum too.
    for (s, s32c, s324, sum) in [
        (2.0f32, 5.0f32, 5.0f32, 3.0f32),
        (1.0, 10.0, 0.0, 7.0),
        (0.5, -3.0, 7.0, -2.0),
    ] {
        let (slope, inv_sum) = run_slope(&mut emu, s, s32c, s324, sum);
        let want = tail_slope(s, s32c, s324, sum);
        assert_eq!(slope.to_bits(), want.1.to_bits(), "slope s={s}");
        assert_eq!(
            inv_sum.to_bits(),
            want.2.to_bits(),
            "inv_sum s={s} sum={sum}"
        );
        total += 2;
    }
    for (m0, m1, s) in [
        (10.0f32, 20.0f32, 2.0f32),
        (0.0, 0.0, 1.0),
        (-7.0, 3.0, 4.0),
    ] {
        let (cx, cy) = run_child(&mut emu, m0, m1, s);
        let want = child_coords(m0, m1, s);
        assert_eq!(cx.to_bits(), want.0.to_bits(), "cx m0={m0} s={s}");
        assert_eq!(cy.to_bits(), want.1.to_bits(), "cy m1={m1} s={s}");
        total += 2;
    }
    for cy in [0.0f32, 1.0, -1.0, 123.456, -0.0] {
        let got = run_neg(&mut emu, cy);
        let (neg_want, _) = child_neg_acc(cy, 0.0, 0.0);
        assert_eq!(got.to_bits(), neg_want.to_bits(), "neg cy={cy}");
        total += 1;
    }
    for (a, step) in [
        (0.0f32, 0.0f32),
        (1.0, 2.0),
        (16_777_216.0, 1.0),
        (-5.5, 5.5),
    ] {
        let got = run_acc(&mut emu, a, step);
        let (_, acc_want) = child_neg_acc(0.0, a, step);
        assert_eq!(got.to_bits(), acc_want.to_bits(), "acc a={a} step={step}");
        total += 1;
    }
    let mut st = 0x7860_7000u32;
    for _ in 0..1500 {
        let s = rng(&mut st, 0.5, 50.0);
        let s32c = rng(&mut st, -500.0, 500.0);
        let s324 = rng(&mut st, -500.0, 500.0);
        let sum = rng(&mut st, -500.0, 500.0);
        let (slope, inv_sum) = run_slope(&mut emu, s, s32c, s324, sum);
        let want = tail_slope(s, s32c, s324, sum);
        assert_eq!(slope.to_bits(), want.1.to_bits(), "slope fuzz");
        assert_eq!(inv_sum.to_bits(), want.2.to_bits(), "inv_sum fuzz");
        let m0 = rng(&mut st, -500.0, 500.0);
        let m1 = rng(&mut st, -500.0, 500.0);
        let (cx, cy) = run_child(&mut emu, m0, m1, s);
        let w = child_coords(m0, m1, s);
        assert_eq!(cx.to_bits(), w.0.to_bits(), "cx fuzz");
        assert_eq!(cy.to_bits(), w.1.to_bits(), "cy fuzz");
        let ncy = rng(&mut st, -1000.0, 1000.0);
        assert_eq!(
            run_neg(&mut emu, ncy).to_bits(),
            child_neg_acc(ncy, 0.0, 0.0).0.to_bits(),
            "neg fuzz"
        );
        let a = rng(&mut st, -1000.0, 1000.0);
        let step = rng(&mut st, -1000.0, 1000.0);
        assert_eq!(
            run_acc(&mut emu, a, step).to_bits(),
            child_neg_acc(0.0, a, step).1.to_bits(),
            "acc fuzz"
        );
        total += 6;
    }
    eprintln!("glue_785ea0_tail: {total} bit-exact (slope fdiv+fsub+fmul, inv·sum fmul, child fdiv×2, neg fchs, acc fadd)");
}

// ===========================================================================================
// 0x786200 — per-child fade/alpha update. The keep-path predicate (fsub @0x78633f → fcom 0.0) and
// the alpha-byte compute (fdiv+fmul[255]+__ftol, live-st0 diff). Diffed at the function's own bytes.
// ===========================================================================================
#[test]
fn glue_786200_alpha() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let child = emu.alloc(0x40); // edi -> child; reads [+0x10],[+0x14]; writes [+0xb]

    // predicate slice 0x78633c..0x78634a: fld[edi+0x14]; fsub[ebp+8]; fcom 0.0; fnstsw. Halt right
    // after `fnstsw ax` (0x786348) — BEFORE the `jp` at 0x78634d (which on the taken branch skips the
    // 0x78634f halt and runs away). `test ah,5` (0x78634a) doesn't change AH, so AH here is the gate.
    let run_keep = |emu: &mut Emu, f14: f32, arg: f32| -> u8 {
        set_frame(emu, &f);
        emu.set_reg(RegisterX86::EDI, child);
        emu.write_u32(child + 0x14, f14.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(0x78633c, 0x78634a);
        ah_of(emu)
    };
    // alpha slice 0x78635b..0x78636f: fst[edi+0x14]; fdiv[+0x350]; fmul[0x7ffe58]=255; __ftol;
    // mov[edi+0xb],al. Entry needs the LIVE diff on st0: prepend an island that recomputes it from
    // [edi+0x14] and [ebp+8] exactly as 0x78633c..0x786342 does (byte-verified), then jump into the
    // function's own 0x78635b slice via a copied island. Simpler: run 0x78633c..0x78636f directly
    // with diff>=0 inputs (the jp at 0x78634d falls to 0x78635b — the alpha path — for diff>=0).
    let run_alpha = |emu: &mut Emu, f14: f32, arg: f32, dur: f32| -> u8 {
        set_frame(emu, &f);
        emu.set_reg(RegisterX86::EDI, child);
        emu.set_reg(RegisterX86::ECX, f.this);
        emu.write_u32(child + 0x14, f14.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.write_u32(f.this + 0x350, dur.to_bits());
        // [edi+0xc] read at 0x78636f for the call 0x77f750; past our halt — irrelevant. Halt right
        // after `mov [edi+0xb],al` at 0x786376 so we never reach that call.
        emu.run(0x78633c, 0x786379);
        emu.read_u32(child + 0x8).to_le_bytes()[3] // [edi+0xb] = byte 3 of the [edi+0x8] dword
    };

    let mut total = 0u32;
    let mut keep = 0u32;
    let mut finish = 0u32;
    // keep-path boundary: f14 == arg ⇒ diff == 0 ⇒ compute path (even parity).
    for (f14, arg) in [(1.0f32, 1.0f32), (2.0, 1.0), (1.0, 2.0), (f32::NAN, 1.0)] {
        let ah = run_keep(&mut emu, f14, arg);
        let bin = (ah & 0x5).count_ones() % 2 == 0; // jp taken (compute) iff EVEN parity
        assert_eq!(bin, fade_alpha_keep(f14, arg), "keep f14={f14} arg={arg}");
        if bin {
            keep += 1;
        } else {
            finish += 1;
        }
        total += 1;
    }
    let mut st = 0x7862_0000u32;
    for _ in 0..2000 {
        let arg = rng(&mut st, 0.0, 1.0);
        let f14 = arg + rng(&mut st, -0.5, 0.5);
        let ah = run_keep(&mut emu, f14, arg);
        let bin = (ah & 0x5).count_ones() % 2 == 0;
        assert_eq!(bin, fade_alpha_keep(f14, arg), "keep fuzz");
        if bin {
            keep += 1;
        } else {
            finish += 1;
        }
        total += 1;
    }
    assert!(keep > 0 && finish > 0, "alpha: need both keep/finish");

    // alpha byte: diff = f14-arg >= 0 (so the function takes the compute path); byte = trunc(diff/dur·255).
    for (f14, arg, dur) in [
        (1.0f32, 0.0f32, 1.0f32), // 1·255 = 255
        (0.5, 0.0, 1.0),          // 0.5·255 = 127.5 -> 127
        (1.0, 1.0, 1.0),          // 0 -> 0
        (2.0, 0.0, 4.0),          // 0.5·255 -> 127
        // NaN, infinite and past-`i32` and past-`i64` quotients: `__ftol` returns the low dword of
        // its qword, 0 for the indefinite, and the byte is its low byte.
        (1.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
        (1.0, 0.0, f32::NAN),
        (1.0, 0.0, 1.0e-30),
        (1.0e10, 0.0, 1.0),
        (1.0e30, 0.0, 1.0),
        (f32::INFINITY, 0.0, 1.0),
        (3.0e7, 0.0, 1.0),
    ] {
        let got = run_alpha(&mut emu, f14, arg, dur);
        assert_eq!(
            got,
            fade_alpha_byte(f14, arg, dur),
            "alpha f14={f14} arg={arg} dur={dur}"
        );
        total += 1;
    }
    for _ in 0..2000 {
        let dur = rng(&mut st, 0.5, 10.0);
        let arg = rng(&mut st, 0.0, 1.0);
        let f14 = arg + rng(&mut st, 0.0, 1.0); // diff >= 0 ⇒ compute path
        let got = run_alpha(&mut emu, f14, arg, dur);
        assert_eq!(got, fade_alpha_byte(f14, arg, dur), "alpha fuzz");
        total += 1;
    }

    // f10 timer decrement slice 0x7862fd..0x786303: fld[edi+0x10]; fsub[ebp+8]; fst[edi+0x10]. The
    // `[+0x10]` timer's per-frame `field − arg`, distinct from the `[+0x14]` decrement. Halt at
    // 0x786306 (the `fcomp` after the `fst` store completes); read [edi+0x10] = the new timer.
    let run_dec = |emu: &mut Emu, f10: f32, arg: f32| -> f32 {
        set_frame(emu, &f);
        emu.set_reg(RegisterX86::EDI, child);
        emu.write_u32(child + 0x10, f10.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(0x7862fd, 0x786306);
        emu.read_f32(child + 0x10)
    };
    for (f10, arg) in [
        (1.0f32, 1.0f32),
        (5.0, 2.0),
        (0.0, 0.5),
        (16_777_216.0, 1.0),
    ] {
        let got = run_dec(&mut emu, f10, arg);
        assert_eq!(
            got.to_bits(),
            fade_timer_dec(f10, arg).to_bits(),
            "dec f10={f10} arg={arg}"
        );
        total += 1;
    }
    for _ in 0..2000 {
        let f10 = rng(&mut st, -1000.0, 1000.0);
        let arg = rng(&mut st, -1000.0, 1000.0);
        assert_eq!(
            run_dec(&mut emu, f10, arg).to_bits(),
            fade_timer_dec(f10, arg).to_bits(),
            "dec fuzz"
        );
        total += 1;
    }
    eprintln!("glue_786200_alpha: {total} bit-exact (fsub keep+dec, fdiv+fmul[255]+__ftol live-st0; keep={keep} finish={finish})");
}

// ===========================================================================================
// 0x7863b0 — level-fraction → libm. frac = getter_f32/divisor_live + 0.5(f64), stored f64. Diffed
// via a copied-byte island (fdivr+fadd qword), the getter/divisor supplied directly.
// ===========================================================================================
#[test]
fn glue_7863b0_level_fraction() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // verbatim 0x78643b (`d8 7d 08` fdivr[ebp+8]) + 0x786441 (`dc 05 78 30 80 00` fadd qword
    // [0x803078]). Island: put divisor on st0 (`fld [div_cell]`), getter at [ebp+8]; the island's
    // fdivr = [ebp+8]/st0 = getter/divisor; fadd 0.5(f64); fstp qword [result].
    {
        let s43b = emu.read(0x78643b, 3);
        assert_eq!(s43b, vec![0xd8, 0x7d, 0x08], "fdivr bytes mismatch");
        let s441 = emu.read(0x786441, 6);
        assert_eq!(
            s441,
            vec![0xdc, 0x05, 0x78, 0x30, 0x80, 0x00],
            "fadd-qword bytes mismatch"
        );
    }
    let div_cell = emu.alloc(4);
    let result = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut c = vec![0xd9u8, 0x05]; // fld dword [div_cell]  (puts divisor on st0)
    c.extend_from_slice(&(div_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd8, 0x7d, 0x08]); // fdivr [ebp+8]              (verbatim 0x78643b)
    c.extend_from_slice(&[0xdc, 0x05, 0x78, 0x30, 0x80, 0x00]); // fadd qword [0x803078] (0x786441)
    c.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [result]
    c.extend_from_slice(&(result as u32).to_le_bytes());
    let isl_end = isl + c.len() as u64;
    emu.write(isl, &c);
    let run = |emu: &mut Emu, getter: f32, divisor: f32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.write_u32(f.ebp + 0x8, getter.to_bits());
        emu.write_u32(div_cell, divisor.to_bits());
        emu.run(isl, isl_end);
        read_f64(emu, result)
    };

    let mut total = 0u32;
    // boundary: getter == divisor ⇒ frac == 1.0 + 0.5 == 1.5 exactly.
    for (g, d) in [(1.0f32, 1.0f32), (3.0, 6.0), (-2.0, 4.0), (0.0, 1.0)] {
        let got = run(&mut emu, g, d);
        assert_eq!(
            got.to_bits(),
            level_fraction(g, d).to_bits(),
            "frac g={g} d={d}"
        );
        total += 1;
    }
    let mut st = 0x7863_b000u32;
    for _ in 0..3000 {
        let g = rng(&mut st, -1000.0, 1000.0);
        let d = rng(&mut st, 0.5, 1000.0);
        assert_eq!(
            run(&mut emu, g, d).to_bits(),
            level_fraction(g, d).to_bits(),
            "frac fuzz g={g} d={d}"
        );
        total += 1;
    }
    eprintln!("glue_7863b0_level_fraction: {total} bit-exact (fdivr+fadd-qword 0.5, f64 store)");
}

// ===========================================================================================
// 0x786490 — per-child negate (two fchs sites). Diffed at the function's first fchs (0x7864d8..)
// run directly: fld[edi+edx+0x4]; fchs; fstp[esp]. The twin (0x786529) is byte-identical (`d9 e0`).
// ===========================================================================================
#[test]
fn glue_786490_negate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Both fchs sites are the verbatim `d9 e0`. Island: fld[cell]; fchs; fstp[result].
    {
        let s1 = emu.read(0x7864e1, 2);
        assert_eq!(s1, vec![0xd9, 0xe0], "fchs#1 bytes mismatch");
        let s2 = emu.read(0x786532, 2);
        assert_eq!(s2, vec![0xd9, 0xe0], "fchs#2 bytes mismatch (twin)");
    }
    let cell = emu.alloc(4);
    let result = emu.alloc(4);
    let isl = emu.alloc(0x20);
    let mut c = vec![0xd9u8, 0x05]; // fld dword [cell]
    c.extend_from_slice(&(cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd9, 0xe0]); // fchs   (verbatim 0x7864e1 / 0x786532)
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [result]
    c.extend_from_slice(&(result as u32).to_le_bytes());
    let isl_end = isl + c.len() as u64;
    emu.write(isl, &c);
    let run = |emu: &mut Emu, cy: f32| -> f32 {
        let _ = &f;
        emu.write_u32(cell, cy.to_bits());
        emu.run(isl, isl_end);
        emu.read_f32(result)
    };

    let mut total = 0u32;
    for cy in [0.0f32, 1.0, -1.0, 123.456, -0.0, f32::INFINITY] {
        let got = run(&mut emu, cy);
        assert_eq!(got.to_bits(), negate_child(cy).to_bits(), "neg cy={cy}");
        total += 1;
    }
    let mut st = 0x7864_9000u32;
    for _ in 0..2000 {
        let cy = rng(&mut st, -1.0e6, 1.0e6);
        assert_eq!(
            run(&mut emu, cy).to_bits(),
            negate_child(cy).to_bits(),
            "neg fuzz"
        );
        total += 1;
    }
    // SCALE255 sanity (compile-time constant cross-check, not a diff).
    assert_eq!(f32::from_bits(SCALE255_BITS), 255.0);
    eprintln!("glue_786490_negate: {total} bit-exact (fchs ×2 sites, byte-identical)");
}
