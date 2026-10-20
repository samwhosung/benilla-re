//! Differential tests of the arithmetic of the minimap blip projection and render functions
//! `0x4ebc00`, `0x4ebf20`, `0x4ec090`, `0x4ec440` and `MinimapFrameRender` (`0x4ec950`), each run
//! against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed as an **instruction slice** (`Emu::run`) over the binary's own
//! bytes — registers/memory/stack synthesized, the stored `fstp/fst [m32]` value read back. Every
//! in-body FP-arith opcode (the arithmetic opcode set) lands in a diffed slice; the large driver
//! 0x4ec950's 56 ops are covered by Slice A (unproject, 38), Slice B (scale loop, 5), and Slices
//! C/D (the two fsincos rotations, 8 each — diffed independently, NOT clone-inherited, since the
//! integer entry glue swaps the cos/sin slots). Fuzz reads the LCG's HIGH bits. Every
//! slice carries a deterministic boundary probe: the matrix transforms probe an exact
//! identity/zero configuration; the live-vs-reload reciprocal probes a value where the f32 and f64
//! reciprocals differ in the low bit.
//!
//! Host double (`docs/METHOD.md#the-harness`): `glue_4ec950_rotate` runs `fsincos` at `0x4ed452`
//! and `0x4ed4f7`. The emulator computes each in host `double`, so where its result reaches a
//! compared value the host's math stands in for the x87's, and the test shows the code around it.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::minimap_blips::{
    delta3, gradient_row, half_scale, mat_m, mat_n, negate3, percent_scale, persp_map_x,
    persp_map_y, recip_scale, rotate, scale_offset, zoom_scale, zoom_span,
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
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x400);
    let sc = emu.alloc(0x400);
    let ebp = sc + 0x200;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1c00;
    Frame { this, ebp, esp }
}

// ===========================================================================================
// 0x4ebc00 — gradient loop body: out0=B+A·m30, out1=C+A·m28, out2=A·m2c (fmul×3+fadd×2).
// ===========================================================================================
#[test]
fn glue_4ebc00_gradient() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let a_cell = emu.alloc(4);

    // Run 0x4ebd62..0x4ebdba (the three fld setups + iteration 1 of the inner loop). [ebp-8] points
    // at A's cell; [ebp-0x20]=B; C is supplied via EDX (0x4ebd67 `mov [ebp-0x1c],edx` writes it).
    // The first fmul (0x4ebd78) reads [eax+0xbc8230] with eax=0 → [0xbc8230]; eax then +=0xc
    // (0x4ebd7e) BEFORE the next two fmuls, so 0x4ebd86 reads [0xc+0xbc8228]=[0xbc8234] and 0x4ebd8e
    // reads [0xc+0xbc822c]=[0xbc8238]. End at 0x4ebdba (after the `fstp [ebp-0x24]` at 0x4ebdb7);
    // 0x4ebda9 would be mid-instruction (the fstp at 0x4ebda7 spans to 0x4ebdaa) → never halts.
    let run =
        |emu: &mut Emu, a: f32, b: f32, c: f32, m30: f32, m28: f32, m2c: f32| -> (f32, f32, f32) {
            emu.write_u32(a_cell, a.to_bits());
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write_u32(f.ebp - 0x8, a_cell as u32); // [ebp-8] = ptr to A
            emu.write_u32(f.ebp - 0x20, b.to_bits()); // B
            emu.set_reg(RegisterX86::EDX, u64::from(c.to_bits())); // → [ebp-0x1c] = C
            emu.write_u32(0xbc8230, m30.to_bits()); // m30 (eax=0)
            emu.write_u32(0xbc8234, m28.to_bits()); // m28 (eax=0xc)
            emu.write_u32(0xbc8238, m2c.to_bits()); // m2c (eax=0xc)
            emu.run(0x4ebd62, 0x4ebdba);
            (
                emu.read_f32(f.ebp - 0x2c), // out0 = B + A·m30
                emu.read_f32(f.ebp - 0x28), // out1 = C + A·m28
                emu.read_f32(f.ebp - 0x24), // out2 = A·m2c
            )
        };

    let mut total = 0u32;
    // Deterministic probe: A=1, m's = identity-ish, B/C exact — out0=B+m30, out1=C+m28, out2=m2c.
    for (a, b, c, m30, m28, m2c) in [
        (1.0f32, 0.0f32, 0.0f32, 1.0f32, 1.0f32, 1.0f32),
        (2.0, 3.0, 5.0, 0.5, 0.25, 0.125),
        (16_777_217.0, 1.0, 1.0, 1.0, 1.0, 1.0), // A non-representable exactly in f32 chain
        (-1.5, -2.5, 3.5, 2.0, -2.0, 0.0),
    ] {
        let got = run(&mut emu, a, b, c, m30, m28, m2c);
        let want = gradient_row(a, b, c, m30, m28, m2c);
        assert_eq!(
            got.0.to_bits(),
            want.0.to_bits(),
            "out0 a={a} b={b} m30={m30}"
        );
        assert_eq!(
            got.1.to_bits(),
            want.1.to_bits(),
            "out1 a={a} c={c} m28={m28}"
        );
        assert_eq!(got.2.to_bits(), want.2.to_bits(), "out2 a={a} m2c={m2c}");
        total += 3;
    }
    let mut s = 0x4ebc_0000u32;
    for _ in 0..3000 {
        let a = rng(&mut s, -1000.0, 1000.0);
        let b = rng(&mut s, -1000.0, 1000.0);
        let c = rng(&mut s, -1000.0, 1000.0);
        let m30 = rng(&mut s, -4.0, 4.0);
        let m28 = rng(&mut s, -4.0, 4.0);
        let m2c = rng(&mut s, -4.0, 4.0);
        let got = run(&mut emu, a, b, c, m30, m28, m2c);
        let want = gradient_row(a, b, c, m30, m28, m2c);
        assert_eq!(got.0.to_bits(), want.0.to_bits(), "out0 fuzz");
        assert_eq!(got.1.to_bits(), want.1.to_bits(), "out1 fuzz");
        assert_eq!(got.2.to_bits(), want.2.to_bits(), "out2 fuzz");
        total += 3;
    }
    eprintln!("glue_4ebc00_gradient: {total} bit-exact (B+A·m30, C+A·m28, A·m2c)");
}

// ===========================================================================================
// 0x4ebf20 — zoom-span reciprocal: recip=0.5/span (fdivr), complement=1.0-recip (fsub). ×2 spans.
// ===========================================================================================
#[test]
fn glue_4ebf20_zoom_span() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Slice 0x4ebfca..0x4ec012 computes both spans: span1=[ebp-0x80], span2=[ebp-0x7c] (set via the
    // 0x58ada0 call normally; we inject them at the slot the fild reads). The fild reads [ebp-0x8]
    // (low) and [ebp-0x4]=0 (high), which the code copies from [ebp-0x80]/[ebp-0x7c] just before each
    // fild. Run from 0x4ebfbd (mov eax,[ebp-0x80]; mov [ebp-8],eax; ...) so the copy-then-fild chain
    // executes with our injected [ebp-0x80]/[ebp-0x7c].
    let run = |emu: &mut Emu, span1: u32, span2: u32| -> (f32, f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x80, span1);
        emu.write_u32(f.ebp - 0x7c, span2);
        emu.run(0x4ebfbd, 0x4ec01e);
        (
            emu.read_f32(f.ebp - 0x10), // recip1
            emu.read_f32(f.ebp - 0x8),  // complement1
            emu.read_f32(f.ebp - 0xc),  // recip2
            emu.read_f32(f.ebp - 0x4),  // complement2
        )
    };

    let mut total = 0u32;
    for (s1, s2) in [(1u32, 2u32), (4, 8), (1024, 1), (1_000_000, 3), (7, 13)] {
        let (r1, c1, r2, c2) = run(&mut emu, s1, s2);
        let (wr1, wc1) = zoom_span(s1);
        let (wr2, wc2) = zoom_span(s2);
        assert_eq!(r1.to_bits(), wr1.to_bits(), "recip1 s={s1}");
        assert_eq!(c1.to_bits(), wc1.to_bits(), "comp1 s={s1}");
        assert_eq!(r2.to_bits(), wr2.to_bits(), "recip2 s={s2}");
        assert_eq!(c2.to_bits(), wc2.to_bits(), "comp2 s={s2}");
        total += 4;
    }
    let mut s = 0x4ebf_2000u32;
    for _ in 0..3000 {
        let s1 = (next(&mut s) >> 8) % 4_000_000 + 1;
        let s2 = (next(&mut s) >> 8) % 4_000_000 + 1;
        let (r1, c1, r2, c2) = run(&mut emu, s1, s2);
        let (wr1, wc1) = zoom_span(s1);
        let (wr2, wc2) = zoom_span(s2);
        assert_eq!(r1.to_bits(), wr1.to_bits(), "recip1 fuzz s={s1}");
        assert_eq!(c1.to_bits(), wc1.to_bits(), "comp1 fuzz");
        assert_eq!(r2.to_bits(), wr2.to_bits(), "recip2 fuzz s={s2}");
        assert_eq!(c2.to_bits(), wc2.to_bits(), "comp2 fuzz");
        total += 4;
    }
    eprintln!("glue_4ebf20_zoom_span: {total} bit-exact (0.5/span, 1.0-recip ×2)");
}

// ===========================================================================================
// 0x4ec090 — percent-scale (fmul[0x80308c]; fchs) + 3× negate (fchs×3).
// ===========================================================================================
#[test]
fn glue_4ec090_scale_neg() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // percent-scale 0x4ec130..0x4ec159: fld [0xbc830c]=g; fmul [0x80308c]=k; fst [ebp-4]; fchs;
    // fstp [ebp-8]. Read scaled=[ebp-4], neg=[ebp-8].
    let run_scale = |emu: &mut Emu, g: f32, k: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(0xbc830c, g.to_bits());
        emu.write_u32(0x80308c, k.to_bits());
        emu.run(0x4ec130, 0x4ec15c);
        (emu.read_f32(f.ebp - 0x4), emu.read_f32(f.ebp - 0x8))
    };
    // 3× negate 0x4ec3bb..0x4ec3da: fld [ebp-0x20]; fchs; fstp [ebp-0x14]; fld [ebp-0x1c]; fchs;
    // fstp [ebp-0x10]; fld [ebp-0x18]; fchs; fstp [ebp-0xc].
    let run_neg = |emu: &mut Emu, x: f32, y: f32, z: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x20, x.to_bits());
        emu.write_u32(f.ebp - 0x1c, y.to_bits());
        emu.write_u32(f.ebp - 0x18, z.to_bits());
        // End at 0x4ec3db (after the `fstp [ebp-0xc]` at 0x4ec3d8); 0x4ec3da is mid-instruction.
        emu.run(0x4ec3bb, 0x4ec3db);
        (
            emu.read_f32(f.ebp - 0x14),
            emu.read_f32(f.ebp - 0x10),
            emu.read_f32(f.ebp - 0xc),
        )
    };

    let mut total = 0u32;
    for (g, k) in [(1.0f32, 0.01f32), (100.0, 0.01), (-3.5, 2.0), (0.0, 5.0)] {
        let (sc, ng) = run_scale(&mut emu, g, k);
        let (wsc, wng) = percent_scale(g, k);
        assert_eq!(sc.to_bits(), wsc.to_bits(), "scale g={g} k={k}");
        assert_eq!(ng.to_bits(), wng.to_bits(), "neg g={g} k={k}");
        total += 2;
    }
    for (x, y, z) in [(1.0f32, -2.0f32, 0.0f32), (-3.5, 4.5, -5.5)] {
        let got = run_neg(&mut emu, x, y, z);
        let want = negate3(x, y, z);
        assert_eq!(got.0.to_bits(), want.0.to_bits(), "neg x={x}");
        assert_eq!(got.1.to_bits(), want.1.to_bits(), "neg y={y}");
        assert_eq!(got.2.to_bits(), want.2.to_bits(), "neg z={z}");
        total += 3;
    }
    let mut s = 0x4ec0_9000u32;
    for _ in 0..2000 {
        let g = rng(&mut s, -1000.0, 1000.0);
        let k = rng(&mut s, -10.0, 10.0);
        let (sc, ng) = run_scale(&mut emu, g, k);
        let (wsc, wng) = percent_scale(g, k);
        assert_eq!(sc.to_bits(), wsc.to_bits(), "scale fuzz");
        assert_eq!(ng.to_bits(), wng.to_bits(), "neg fuzz");
        let x = rng(&mut s, -1000.0, 1000.0);
        let y = rng(&mut s, -1000.0, 1000.0);
        let z = rng(&mut s, -1000.0, 1000.0);
        let got = run_neg(&mut emu, x, y, z);
        let want = negate3(x, y, z);
        assert_eq!(got.0.to_bits(), want.0.to_bits(), "neg3 fuzz");
        assert_eq!(got.1.to_bits(), want.1.to_bits(), "neg3 fuzz");
        assert_eq!(got.2.to_bits(), want.2.to_bits(), "neg3 fuzz");
        total += 5;
    }
    eprintln!("glue_4ec090_scale_neg: {total} bit-exact (fmul+fchs, fchs×3)");
}

// ===========================================================================================
// 0x4ec440 — 0.5-scale (fmul; fchs), reciprocal·scale (fdiv; fmul[live]), 0.5-scale tail (fmul).
// ===========================================================================================
#[test]
fn glue_4ec440_scale_recip() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // leading 0.5-scale 0x4ec4d0..0x4ec4fd: fld [ebp+8]; fmul [0x7ffa24]=0.5; fst [ebp+8]; fchs;
    // fstp [ebp-0x10]. scaled=[ebp+8], neg=[ebp-0x10].
    let run_zoom = |emu: &mut Emu, arg: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(0x4ec4d0, 0x4ec500);
        (emu.read_f32(f.ebp + 0x8), emu.read_f32(f.ebp - 0x10))
    };
    // reciprocal·scale 0x4ec60c..0x4ec62c: fld 1.0; fdiv [ebp+8]=arg; fst [ebp+0xc]; fmul [0x807a40]=k;
    // fst[ebp-0xc]; fst[ebp-8]; fstp[ebp-4]. recip=[ebp+0xc], scaled=[ebp-0xc].
    let run_recip = |emu: &mut Emu, arg: f32, k: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.write_u32(0x807a40, k.to_bits());
        emu.run(0x4ec60c, 0x4ec62c);
        (emu.read_f32(f.ebp + 0xc), emu.read_f32(f.ebp - 0xc))
    };
    // tail 0.5-scale 0x4ec6dd..0x4ec6f7: fld [ebp+0xc]; fmul [0x7ffa24]=0.5; fst[ebp-0xc]; fst[ebp-8];
    // fstp[ebp-4]. product=[ebp-0xc].
    let run_half = |emu: &mut Emu, v: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0xc, v.to_bits());
        emu.run(0x4ec6dd, 0x4ec6f7);
        emu.read_f32(f.ebp - 0xc)
    };

    let mut total = 0u32;
    for arg in [1.0f32, 2.0, -4.0, 1024.0, 0.5] {
        let (sc, ng) = run_zoom(&mut emu, arg);
        let (wsc, wng) = zoom_scale(arg);
        assert_eq!(sc.to_bits(), wsc.to_bits(), "zoom arg={arg}");
        assert_eq!(ng.to_bits(), wng.to_bits(), "zoom-neg arg={arg}");
        let hv = run_half(&mut emu, arg);
        assert_eq!(hv.to_bits(), half_scale(arg).to_bits(), "half arg={arg}");
        total += 3;
    }
    // reciprocal probe at arg=3.0 (1/3 is not f32-exact, so live-vs-f32 reciprocal paths can differ).
    for (arg, k) in [(3.0f32, 7.0f32), (1.0, 1.0), (-2.0, 0.5), (1000.0, 0.001)] {
        let (r, sc) = run_recip(&mut emu, arg, k);
        let (wr, wsc) = recip_scale(arg, k);
        assert_eq!(r.to_bits(), wr.to_bits(), "recip arg={arg}");
        assert_eq!(sc.to_bits(), wsc.to_bits(), "recip-scale arg={arg} k={k}");
        total += 2;
    }
    let mut s = 0x4ec4_4000u32;
    for _ in 0..2000 {
        let arg = rng(&mut s, 0.25, 4096.0);
        let k = rng(&mut s, -10.0, 10.0);
        let (sc, ng) = run_zoom(&mut emu, arg);
        let (wsc, wng) = zoom_scale(arg);
        assert_eq!(sc.to_bits(), wsc.to_bits(), "zoom fuzz");
        assert_eq!(ng.to_bits(), wng.to_bits(), "zoom-neg fuzz");
        let (r, rsc) = run_recip(&mut emu, arg, k);
        let (wr, wrsc) = recip_scale(arg, k);
        assert_eq!(r.to_bits(), wr.to_bits(), "recip fuzz");
        assert_eq!(rsc.to_bits(), wrsc.to_bits(), "recip-scale fuzz");
        assert_eq!(
            run_half(&mut emu, arg).to_bits(),
            half_scale(arg).to_bits(),
            "half fuzz"
        );
        total += 5;
    }
    eprintln!("glue_4ec440_scale_recip: {total} bit-exact (0.5-scale, 1/arg·k live, 0.5-scale)");
}

// ===========================================================================================
// 0x4ec950 Slice A — unproject/pick: delta3, mat_m (M·d), mat_n (N·outA), persp X (live invD) /
// Y (f32 invD). fsub×3 + (fmul×9+faddp×6) + (fmul×6+faddp×4) + (fdiv+fmul×4+fadd×2) = 38 ops.
// ===========================================================================================
#[test]
fn glue_4ec950_unproject() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Slice 0x4ecf04..0x4ed011 (one straight-line block; je-target entry). Globals:
    //   far  = [0xbc831c/0xbc8320/0xbc8324], near = [0xbc8310/0xbc8314/0xbc8318]
    //   M cols: row0 [0xbc8368/78/88], row1 [0xbc836c/7c/8c], row2 [0xbc8370/80/90]
    //   N rows: px [0xbc8328/38/48], py [0xbc832c/3c/4c]
    //   depth = [0xbc830c]
    // Outputs: screenX = [ebp-0x54], screenY = [ebp-0x50].
    // The block also reads [ebp-0x11] (a color byte) → packs to [ebp-0x10]; irrelevant to FP.
    let write_globals = |emu: &mut Emu,
                         far: [f32; 3],
                         near: [f32; 3],
                         m: [[f32; 3]; 3],
                         n: [[f32; 3]; 2],
                         depth: f32| {
        emu.write_u32(0xbc831c, far[0].to_bits());
        emu.write_u32(0xbc8320, far[1].to_bits());
        emu.write_u32(0xbc8324, far[2].to_bits());
        emu.write_u32(0xbc8310, near[0].to_bits());
        emu.write_u32(0xbc8314, near[1].to_bits());
        emu.write_u32(0xbc8318, near[2].to_bits());
        // M row i = (col_dx, col_dy, col_dz) at (0xbc8368+4i, 0xbc8378+4i, 0xbc8388+4i).
        for (i, row) in m.iter().enumerate() {
            let i = i as u64;
            emu.write_u32(0xbc8368 + 4 * i, row[0].to_bits());
            emu.write_u32(0xbc8378 + 4 * i, row[1].to_bits());
            emu.write_u32(0xbc8388 + 4 * i, row[2].to_bits());
        }
        // N row j = (col_a0, col_a1, col_a2) at (0xbc8328+4j, 0xbc8338+4j, 0xbc8348+4j).
        for (j, row) in n.iter().enumerate() {
            let j = j as u64;
            emu.write_u32(0xbc8328 + 4 * j, row[0].to_bits());
            emu.write_u32(0xbc8338 + 4 * j, row[1].to_bits());
            emu.write_u32(0xbc8348 + 4 * j, row[2].to_bits());
        }
        emu.write_u32(0xbc830c, depth.to_bits());
    };
    let run = |emu: &mut Emu,
               far: [f32; 3],
               near: [f32; 3],
               m: [[f32; 3]; 3],
               n: [[f32; 3]; 2],
               depth: f32|
     -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        write_globals(emu, far, near, m, n, depth);
        // End at 0x4ed014 (AFTER the `fstp [ebp-0x50]` screenY store at 0x4ed011, which `until` would
        // otherwise exclude).
        emu.run(0x4ecf04, 0x4ed014);
        (emu.read_f32(f.ebp - 0x54), emu.read_f32(f.ebp - 0x50))
    };
    let predict = |far: [f32; 3],
                   near: [f32; 3],
                   m: [[f32; 3]; 3],
                   n: [[f32; 3]; 2],
                   depth: f32|
     -> (f32, f32) {
        let d = delta3(far, near);
        let out_a = mat_m(d, m);
        let (px, py) = mat_n(out_a, n);
        (persp_map_x(depth, px), persp_map_y(depth, py))
    };

    let mut total = 0u32;
    // Deterministic probe: identity matrices, depth=3 (1/3 not f32-exact → exercises live-vs-f32 invD).
    let id3 = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let id2 = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0]];
    for (far, near, depth) in [
        ([2.0f32, 3.0, 5.0], [1.0f32, 1.0, 1.0], 3.0f32),
        ([1.0, 0.0, 0.0], [0.0, 0.0, 0.0], 1.0),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], 7.0),
    ] {
        let got = run(&mut emu, far, near, id3, id2, depth);
        let want = predict(far, near, id3, id2, depth);
        assert_eq!(
            got.0.to_bits(),
            want.0.to_bits(),
            "screenX probe far={far:?}"
        );
        assert_eq!(
            got.1.to_bits(),
            want.1.to_bits(),
            "screenY probe far={far:?}"
        );
        total += 2;
    }
    let mut s = 0x4ec9_5000u32;
    for _ in 0..4000 {
        let far = [
            rng(&mut s, -100.0, 100.0),
            rng(&mut s, -100.0, 100.0),
            rng(&mut s, -100.0, 100.0),
        ];
        let near = [
            rng(&mut s, -100.0, 100.0),
            rng(&mut s, -100.0, 100.0),
            rng(&mut s, -100.0, 100.0),
        ];
        let mut m = [[0.0f32; 3]; 3];
        for row in m.iter_mut() {
            for v in row.iter_mut() {
                *v = rng(&mut s, -2.0, 2.0);
            }
        }
        let mut n = [[0.0f32; 3]; 2];
        for row in n.iter_mut() {
            for v in row.iter_mut() {
                *v = rng(&mut s, -2.0, 2.0);
            }
        }
        // depth away from 0 (avoid inf; the binary's 1/depth would also produce inf — fine, but skip
        // to keep the diff meaningful on finite values).
        let depth = {
            let d = rng(&mut s, -50.0, 50.0);
            if d.abs() < 0.5 {
                d.signum() * 0.5 + d
            } else {
                d
            }
        };
        let got = run(&mut emu, far, near, m, n, depth);
        let want = predict(far, near, m, n, depth);
        assert_eq!(got.0.to_bits(), want.0.to_bits(), "screenX fuzz");
        assert_eq!(got.1.to_bits(), want.1.to_bits(), "screenY fuzz");
        total += 2;
    }
    eprintln!("glue_4ec950_unproject: {total} bit-exact (delta3+M·d+N·outA+persp X[live]/Y[f32])");
}

// ===========================================================================================
// 0x4ec950 Slice B — per-element scale loop: outX=S·m0+offX (live), outY=f32(S·m1)+offY, outZ=
// f32(S·m2). fmul×3+fadd×2 (one iteration, ecx=1).
// ===========================================================================================
#[test]
fn glue_4ec950_scale_loop() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let out = emu.alloc(0x40);

    // Slice 0x4ed1c0..0x4ed21f, ecx=1 ⇒ one iteration then `dec ecx; jne` exits. ebx→this (+0x7c=S);
    // eax is the REAL .data matrix base 0xbc8200 (mapped). m0=[eax-8] (0xbc81f8) and m1=[eax-4]
    // (0xbc81fc) are read BEFORE the `add eax,0xc`; m2=[eax-0xc] (0xbc8200) AFTER. The OUTPUT ptr edx is
    // computed inside as `edx = [0xbc8180] + [ebp-0x4] + eax + 4` (with eax=0xbc820c at the store), so
    // we set [0xbc8180] = out − 0xbc8210 (mod 2^32) and [ebp-0x4]=0 ⇒ edx wraps to `out` (32-bit
    // address arithmetic). The f32 results also land at the `fstp` slots [ebp-0x68]/[ebp-0x64]/
    // [ebp-0x60], which we read directly. offX=[ebp-0x54], offY=[ebp-0x50].
    const MAT: u64 = 0xbc8200;
    let run = |emu: &mut Emu, sscale: f32, m: [f32; 3], off_x: f32, off_y: f32| -> [f32; 3] {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EBX, f.this);
        emu.set_reg(RegisterX86::EAX, MAT);
        emu.set_reg(RegisterX86::ECX, 1);
        emu.write_u32(0xbc8180, (out as u32).wrapping_sub(0xbc8210)); // edx wraps to `out`
        emu.write_u32(f.ebp - 0x4, 0); // edx offset addend
        emu.write_u32(f.this + 0x7c, sscale.to_bits());
        emu.write_u32(MAT - 0x8, m[0].to_bits()); // [eax-8]  (eax=MAT)   → 0xbc81f8
        emu.write_u32(MAT - 0x4, m[1].to_bits()); // [eax-4]  (eax=MAT)   → 0xbc81fc
        emu.write_u32(MAT, m[2].to_bits()); // [eax-0xc] (eax=MAT+0xc)    → 0xbc8200
        emu.write_u32(f.ebp - 0x54, off_x.to_bits());
        emu.write_u32(f.ebp - 0x50, off_y.to_bits());
        emu.run(0x4ed1c0, 0x4ed21f);
        // outX=[ebp-0x68], outY=[ebp-0x64], outZ=[ebp-0x60] (the fstp slots).
        [
            emu.read_f32(f.ebp - 0x68),
            emu.read_f32(f.ebp - 0x64),
            emu.read_f32(f.ebp - 0x60),
        ]
    };

    let mut total = 0u32;
    for (sscale, m, ox, oy) in [
        (1.0f32, [1.0f32, 1.0, 1.0], 0.0f32, 0.0f32),
        (2.0, [0.5, 0.25, 0.125], 10.0, 20.0),
        (16_777_217.0, [1.0, 1.0, 1.0], 1.0, 1.0),
        (-1.5, [2.0, -2.0, 0.0], -5.0, 5.0),
    ] {
        let got = run(&mut emu, sscale, m, ox, oy);
        let want = scale_offset(sscale, m, ox, oy);
        assert_eq!(got[0].to_bits(), want[0].to_bits(), "outX s={sscale}");
        assert_eq!(got[1].to_bits(), want[1].to_bits(), "outY s={sscale}");
        assert_eq!(got[2].to_bits(), want[2].to_bits(), "outZ s={sscale}");
        total += 3;
    }
    let mut s = 0x4ed1_c000u32;
    for _ in 0..3000 {
        let sscale = rng(&mut s, -1000.0, 1000.0);
        let m = [
            rng(&mut s, -10.0, 10.0),
            rng(&mut s, -10.0, 10.0),
            rng(&mut s, -10.0, 10.0),
        ];
        let ox = rng(&mut s, -500.0, 500.0);
        let oy = rng(&mut s, -500.0, 500.0);
        let got = run(&mut emu, sscale, m, ox, oy);
        let want = scale_offset(sscale, m, ox, oy);
        assert_eq!(got[0].to_bits(), want[0].to_bits(), "outX fuzz");
        assert_eq!(got[1].to_bits(), want[1].to_bits(), "outY fuzz");
        assert_eq!(got[2].to_bits(), want[2].to_bits(), "outZ fuzz");
        total += 3;
    }
    eprintln!(
        "glue_4ec950_scale_loop: {total} bit-exact (S·m0+offX[live], f32(S·m1)+offY, f32(S·m2))"
    );
}

// ===========================================================================================
// 0x4ec950 Slices C/D — fsincos rotation: outX=h-r·u, outY=h+r·v. C: (u,v)=(cos,sin); D:
// (u,v)=(sin,cos). fsincos+fmul×4+fchs+fadd×2 each, diffed independently (no clone-inherit).
// ===========================================================================================
#[test]
fn glue_4ec950_rotate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let cptr = emu.alloc(4); // [ecx] for slice C: output base ptr
    let cbuf = emu.alloc(0x400);
    let dbuf = emu.alloc(0x400);

    // Slice C 0x4ed44f..0x4ed4a8: theta=[ebp-0x8]; eax=[ebp-0x4], edx=[ebp-0xc] (fsincos stores
    // cos→[ebp-0x4], sin→[ebp-0xc]); ecx→cptr, [ecx]→cbuf, writes vec3 at cbuf+0x390. Globals
    // [0xbc8274]=g_radius, [0xbc7970]=g_center.
    let run_c = |emu: &mut Emu, theta: f32, gr: f32, gc: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ECX, cptr);
        // Entry is past the `lea eax,[ebp-0x4]; lea edx,[ebp-0xc]` (0x4ed43a/3d), so set them: fsincos
        // stores cos→[eax]=[ebp-0x4], sin→[edx]=[ebp-0xc] (Slice C slot assignment).
        emu.set_reg(RegisterX86::EAX, f.ebp - 0x4);
        emu.set_reg(RegisterX86::EDX, f.ebp - 0xc);
        emu.write_u32(cptr, cbuf as u32);
        emu.write_u32(f.ebp - 0x8, theta.to_bits());
        emu.write_u32(0xbc8274, gr.to_bits());
        emu.write_u32(0xbc7970, gc.to_bits());
        emu.run(0x4ed44f, 0x4ed4a8);
        let base = cbuf + 0x390;
        (
            emu.read_f32(base),
            emu.read_f32(base + 4),
            emu.read_f32(base + 8),
        )
    };
    // Slice D 0x4ed4f4..0x4ed54c: same but eax=[ebp-0xc], edx=[ebp-0x4] (cos→[ebp-0xc], sin→
    // [ebp-0x4]); esi→dptr-struct, [esi]→dbuf, writes at dbuf+0x390. esi is the base; [esi] read at
    // 0x4ed509/0x4ed54c.
    let dptr = emu.alloc(4);
    let run_d = |emu: &mut Emu, theta: f32, gr: f32, gc: f32| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, dptr);
        // Entry past `lea eax,[ebp-0xc]; lea edx,[ebp-0x4]` (0x4ed4e2/e5): fsincos stores
        // cos→[eax]=[ebp-0xc], sin→[edx]=[ebp-0x4] (Slice D swaps the slots vs C).
        emu.set_reg(RegisterX86::EAX, f.ebp - 0xc);
        emu.set_reg(RegisterX86::EDX, f.ebp - 0x4);
        emu.write_u32(dptr, dbuf as u32);
        emu.write_u32(f.ebp - 0x8, theta.to_bits());
        emu.write_u32(0xbc8274, gr.to_bits());
        emu.write_u32(0xbc7970, gc.to_bits());
        emu.run(0x4ed4f4, 0x4ed54c);
        let base = dbuf + 0x390;
        (
            emu.read_f32(base),
            emu.read_f32(base + 4),
            emu.read_f32(base + 8),
        )
    };

    let mut total = 0u32;
    // Deterministic probes: theta=0 (cos=1,sin=0 → outX=h-r, outY=h), theta=π/2.
    for theta in [
        0.0f32,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        -std::f32::consts::FRAC_PI_2,
    ] {
        let (gr, gc) = (100.0f32, 256.0f32);
        let gc_c = run_c(&mut emu, theta, gr, gc);
        let wc = rotate(theta, gr, gc, true);
        assert_eq!(gc_c.0.to_bits(), wc.0.to_bits(), "C outX theta={theta}");
        assert_eq!(gc_c.1.to_bits(), wc.1.to_bits(), "C outY theta={theta}");
        assert_eq!(gc_c.2.to_bits(), wc.2.to_bits(), "C outZ theta={theta}");
        let gd = run_d(&mut emu, theta, gr, gc);
        let wd = rotate(theta, gr, gc, false);
        assert_eq!(gd.0.to_bits(), wd.0.to_bits(), "D outX theta={theta}");
        assert_eq!(gd.1.to_bits(), wd.1.to_bits(), "D outY theta={theta}");
        assert_eq!(gd.2.to_bits(), wd.2.to_bits(), "D outZ theta={theta}");
        total += 6;
    }
    let mut s = 0x4ed4_5000u32;
    for _ in 0..2000 {
        let theta = rng(&mut s, -6.5, 6.5);
        let gr = rng(&mut s, -500.0, 500.0);
        let gc = rng(&mut s, -1000.0, 1000.0);
        let cc = run_c(&mut emu, theta, gr, gc);
        let wc = rotate(theta, gr, gc, true);
        assert_eq!(cc.0.to_bits(), wc.0.to_bits(), "C outX fuzz");
        assert_eq!(cc.1.to_bits(), wc.1.to_bits(), "C outY fuzz");
        let dd = run_d(&mut emu, theta, gr, gc);
        let wd = rotate(theta, gr, gc, false);
        assert_eq!(dd.0.to_bits(), wd.0.to_bits(), "D outX fuzz");
        assert_eq!(dd.1.to_bits(), wd.1.to_bits(), "D outY fuzz");
        total += 4;
    }
    eprintln!("glue_4ec950_rotate: {total} bit-exact (C:(cos,sin) D:(sin,cos), h∓r·trig)");
}
