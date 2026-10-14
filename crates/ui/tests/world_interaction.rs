//! Differential tests of the arithmetic of the world-interaction functions: `CheckCorpseRange`
//! (`0x492130`), `WorldClickNothing` (`0x492d30`), `0x492e90`, `ApplyAutoUiScale` (`0x492f70`)
//! and `SetInteractionTarget` (`0x4930d0`), each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed by executing the binary's OWN bytes — either a real instruction
//! `Emu::run` slice (when the FP result lands in a mappable store: the 0x492d30 deltas to
//! `[ebp-x]`, the 0x4930d0 fold to BSS `[0xb4e2c0]`) or a COPIED-BYTE ISLAND (when the result is
//! consumed by an `fcomp` and never stored: the squared-distance reductions, the `·3.0`/`·0.75`/
//! `|G−x|` clamp steps). Every island's FP bytes are `assert_eq!`-verified against their source
//! addresses (— islands credited only when the bytes match source). Every in-body
//! arithmetic opcode lands in a diffed slice. Fuzz reads the LCG's HIGH bits (low bits periodic,
//! ). Threshold probes land the squared-distance `fcomp` exactly on its
//! radius² boundary and read the x87 status word (`fnstsw ax`) at the branch.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::world_interaction::{
    abs_diff, fold_196, mul075, mul3, recip_unit, sqdist130, sqlen_d30,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// The three coordinate globals 0x492130 subtracts (radius-test center).
const G_X: u64 = 0xb4e284;
const G_Y: u64 = 0xb4e288;
const G_Z: u64 = 0xb4e28c;
// 0x492e90/0x492f70 work on the single global G = [0xb4e30c]; 0x4930d0 stores 196.0 to [0xb4e2c0].
const G_FRAC: u64 = 0xb4e30c;
const G_THRESH: u64 = 0xb4e2c0;

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

// ===========================================================================================
// 0x492130 — squared distance `(Δz²+Δx²)+Δy²` of `p=[eax+0/4/8]` vs globals, then `fcomp` vs the
// radius² threshold `[0xb4e2ac]`. The deltas stay live on x87 (no per-delta store) and the sum is
// consumed by `fcomp`, so the value is diffed via a COPIED-BYTE ISLAND of the whole reduction
// `[0x49216c, 0x492196)` (fsub×3 + fmul×3 + faddp×2 = all 8 arithmetic ops) with the source's
// `fcomp` replaced by `fstp [cell]`. A separate threshold probe runs the REAL fcomp slice.
// ===========================================================================================
#[test]
fn glue_492_492130_sqdist_radius() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // The point buffer the source reads via [eax+0/4/8].
    let pbuf = emu.alloc(0x10);
    let cell = emu.alloc(4);

    // Island: copy the reduction bytes [0x49216c, 0x492196) VERBATIM, append `fstp [cell]`.
    let isl = emu.alloc(0x40);
    let src = emu.read(0x49216c, 0x492196 - 0x49216c); // fld[eax]..faddp (8 arithmetic + the flds)
    let mut code = src.clone();
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [cell]
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    // byte-verify the island's FP region == source.
    assert_eq!(
        &emu.read(isl, src.len()),
        &emu.read(0x49216c, src.len()),
        "492130 island bytes != src"
    );

    let run = |emu: &mut Emu, p: [f32; 3], g: [f32; 3]| -> f32 {
        emu.set_reg(RegisterX86::EAX, pbuf);
        emu.set_reg(RegisterX86::ESP, f.esp);
        for (i, v) in p.iter().enumerate() {
            emu.write_u32(pbuf + 4 * i as u64, v.to_bits());
        }
        emu.write_u32(G_X, g[0].to_bits());
        emu.write_u32(G_Y, g[1].to_bits());
        emu.write_u32(G_Z, g[2].to_bits());
        emu.run(isl, isl_end);
        emu.read_f32(cell)
    };

    let mut total = 0u32;
    let probes: [([f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([1.0, 2.0, 3.0], [0.0, 0.0, 0.0]),
        ([10.0, -10.0, 5.0], [1.5, 2.5, -3.5]),
        ([-100.0, 0.25, 1e6], [1.0, 1.0, 1.0]),
        ([3.0, 4.0, 0.0], [0.0, 0.0, 0.0]), // 9+16 = 25
        ([7.0, 24.0, 0.0], [0.0, 0.0, 0.0]),
    ];
    for (p, g) in probes {
        assert_eq!(
            run(&mut emu, p, g).to_bits(),
            sqdist130(p[0], p[1], p[2], g[0], g[1], g[2]).to_bits(),
            "sqdist130 p={p:?} g={g:?}"
        );
        total += 1;
    }
    let mut s = 0x4921_3000u32;
    for _ in 0..4000 {
        let p = [
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
        ];
        let g = [
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
        ];
        assert_eq!(
            run(&mut emu, p, g).to_bits(),
            sqdist130(p[0], p[1], p[2], g[0], g[1], g[2]).to_bits(),
            "sqdist130 fuzz p={p:?} g={g:?}"
        );
        total += 1;
    }

    // ---- THRESHOLD PROBE: the real `fcomp [0xb4e2ac]` boundary. Run [0x49216c, 0x49219e) (the
    // reduction + `fcomp` + `fstp st(0)`), then [0x49219e, 0x4921a0) is `fnstsw ax`. We run through
    // `fnstsw ax` (halt at 0x4921a2, the `test ah,0x41`) and read AH. fcomp st0(=sqdist) vs thr:
    //   st0 > thr → C3C2C0 = 000 (ah&0x41 = 0x00, even parity, PF=1)
    //   st0 < thr → C0=1     (ah&0x41 = 0x01, odd parity,  PF=0)
    //   st0 == thr→ C3=1     (ah&0x41 = 0x40, odd parity,  PF=0)
    // The emitted branch is `jp 0x4921ae` (skip → esi stays 0). So "hit" (esi←1) iff PF=0 iff
    // sqdist <= thr. We probe the three orderings at the boundary and assert AH directly.
    let probe = |emu: &mut Emu, sqd_input: [f32; 3], thr: f32| -> u8 {
        // place a point so that sqdist == sqd_input components squared sum; we just set p=delta, g=0.
        emu.set_reg(RegisterX86::EAX, pbuf);
        emu.set_reg(RegisterX86::ESP, f.esp);
        for (i, v) in sqd_input.iter().enumerate() {
            emu.write_u32(pbuf + 4 * i as u64, v.to_bits());
        }
        emu.write_u32(G_X, 0.0f32.to_bits());
        emu.write_u32(G_Y, 0.0f32.to_bits());
        emu.write_u32(G_Z, 0.0f32.to_bits());
        emu.write_u32(0xb4e2ac, thr.to_bits());
        emu.run(0x49216c, 0x4921a2); // reduction + fcomp + fstp st0 + fnstsw ax
        ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
    };
    // dist² = 25.0 exactly (3,4,0). Probe thr just-below / equal / just-above.
    let below = probe(
        &mut emu,
        [3.0, 4.0, 0.0],
        f32::from_bits(25.0f32.to_bits() - 1),
    ); // thr<25 ⇒ st0>thr
    assert_eq!(below & 0x41, 0x00, "probe st0>thr: C3=C0=0");
    total += 1;
    let equal = probe(&mut emu, [3.0, 4.0, 0.0], 25.0);
    assert_eq!(equal & 0x41, 0x40, "probe st0==thr: C3=1");
    total += 1;
    let above = probe(
        &mut emu,
        [3.0, 4.0, 0.0],
        f32::from_bits(25.0f32.to_bits() + 1),
    ); // thr>25 ⇒ st0<thr
    assert_eq!(above & 0x41, 0x01, "probe st0<thr: C0=1");
    total += 1;

    eprintln!(
        "glue_492_492130_sqdist_radius: {total} bit-exact (island fsub×3+fmul×3+faddp×2; fcomp probe ×3)"
    );
}

// ===========================================================================================
// 0x492d30 — squared edge length. The 3 deltas `(p1−p0)` are `fstp`'d to [ebp-0x10/-0xc/-8] (so
// each narrows to f32), then reloaded & reduced `(Δz²+Δx²)+Δy²`, `fcomp` vs 0.0. Two diffs:
//   (a) REAL slice [0x492d6c, 0x492d8a) — the 3 fsub+fstp deltas, read from [ebp-x] (fsub×3).
//   (b) ISLAND of the reduction [0x492d8c, 0x492da2) + `fstp [cell]`, deltas fed via [ebp-x]
//       (fmul×3 + faddp×2). Verified byte-for-byte against source.
// ===========================================================================================
#[test]
fn glue_492_492d30_sqlen_edge() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // esi → an edge struct: p0 at [esi+0/4/8], p1 at [esi+0xc/0x10/0x14].
    let edge = emu.alloc(0x20);
    let cell = emu.alloc(4);

    // (a) the delta-formation slice (fsub×3, each `fstp dword [ebp-x]`).
    let run_deltas = |emu: &mut Emu, p0: [f32; 3], p1: [f32; 3]| -> [f32; 3] {
        emu.set_reg(RegisterX86::ESI, edge);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        for (i, v) in p0.iter().enumerate() {
            emu.write_u32(edge + 4 * i as u64, v.to_bits());
        }
        for (i, v) in p1.iter().enumerate() {
            emu.write_u32(edge + 0xc + 4 * i as u64, v.to_bits());
        }
        emu.run(0x492d6c, 0x492d8a); // fld;fsub;fstp ×3
        [
            emu.read_f32(f.ebp - 0x10), // Δx
            emu.read_f32(f.ebp - 0xc),  // Δy
            emu.read_f32(f.ebp - 0x8),  // Δz
        ]
    };

    // (b) the reduction island [0x492d8c, 0x492da2) + fstp [cell]. Reads [ebp-0x8/-0x10/-0xc].
    let isl = emu.alloc(0x40);
    let red = emu.read(0x492d8c, 0x492da2 - 0x492d8c);
    let mut code = red.clone();
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        &emu.read(isl, red.len()),
        &emu.read(0x492d8c, red.len()),
        "492d30 reduction island bytes != src"
    );
    let run_reduce = |emu: &mut Emu, d: [f32; 3]| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x10, d[0].to_bits()); // Δx
        emu.write_u32(f.ebp - 0xc, d[1].to_bits()); // Δy
        emu.write_u32(f.ebp - 0x8, d[2].to_bits()); // Δz
        emu.run(isl, isl_end);
        emu.read_f32(cell)
    };

    let mut total = 0u32;
    let probes: [([f32; 3], [f32; 3]); 5] = [
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]),
        ([1.5, 2.5, 3.5], [-1.0, 10.0, 0.25]),
        ([1e5, -1e5, 0.0], [0.0, 0.0, 1e5]),
        ([-7.0, 7.0, 7.0], [7.0, -7.0, -7.0]),
    ];
    for (p0, p1) in probes {
        // (a) deltas must match the binary's narrowed (p1-p0) as f32.
        let d = run_deltas(&mut emu, p0, p1);
        let want_d = [
            (f64::from(p1[0]) - f64::from(p0[0])) as f32,
            (f64::from(p1[1]) - f64::from(p0[1])) as f32,
            (f64::from(p1[2]) - f64::from(p0[2])) as f32,
        ];
        for i in 0..3 {
            assert_eq!(
                d[i].to_bits(),
                want_d[i].to_bits(),
                "d30 delta[{i}] p0={p0:?} p1={p1:?}"
            );
            total += 1;
        }
        // (b) reduction over the (binary-produced) deltas.
        let got = run_reduce(&mut emu, d);
        let want = sqlen_d30(p0, p1);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "d30 sqlen p0={p0:?} p1={p1:?}"
        );
        total += 1;
    }
    let mut s = 0x492d_3000u32;
    for _ in 0..3000 {
        let p0 = [
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
        ];
        let p1 = [
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
            rng(&mut s, -5000.0, 5000.0),
        ];
        let d = run_deltas(&mut emu, p0, p1);
        let want_d = [
            (f64::from(p1[0]) - f64::from(p0[0])) as f32,
            (f64::from(p1[1]) - f64::from(p0[1])) as f32,
            (f64::from(p1[2]) - f64::from(p0[2])) as f32,
        ];
        for i in 0..3 {
            assert_eq!(d[i].to_bits(), want_d[i].to_bits(), "d30 delta fuzz[{i}]");
        }
        let got = run_reduce(&mut emu, d);
        assert_eq!(got.to_bits(), sqlen_d30(p0, p1).to_bits(), "d30 sqlen fuzz");
        total += 4;
    }
    eprintln!("glue_492_492d30_sqlen_edge: {total} bit-exact (fsub×3 deltas + reduction island fmul×3+faddp×2)");
}

// ===========================================================================================
// 0x492e90 — single-axis fraction clamp/commit on G=[0xb4e30c]. Owned FP: `G·3.0`(0x492ec8),
// `G·0.75`(0x492ee1), `|G−x|`(0x492f21 fsub + 0x492f24 fabs). All three results feed an `fcomp`
// (consumed), so each is diffed via a COPIED-BYTE ISLAND of just its FP ops + `fstp [cell]`.
// fmul×2 + fsub×1 + fabs×1 = 4 arithmetic ops, all covered.
// ===========================================================================================
#[test]
fn glue_492_492e90_fraction_clamp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let cell = emu.alloc(4);

    // island helper: copy `len` source bytes at `srcva`, append `fstp [cell]`.
    let build = |emu: &mut Emu, srcva: u64, len: usize, label: &str| -> (u64, u64) {
        let isl = emu.alloc(0x20);
        let src = emu.read(srcva, len);
        let mut code = src.clone();
        code.extend_from_slice(&[0xd9, 0x1d]);
        code.extend_from_slice(&(cell as u32).to_le_bytes());
        let end = isl + code.len() as u64;
        emu.write(isl, &code);
        assert_eq!(
            &emu.read(isl, len),
            &emu.read(srcva, len),
            "{label} island bytes != src"
        );
        (isl, end)
    };

    // G·3.0:  fld [G] (0x492ec2, 6) ; fmul [3.0] (0x492ec8, 6)  → 12 bytes contiguous.
    let (i3, i3e) = build(&mut emu, 0x492ec2, 12, "mul3");
    // G·0.75: fld [G] (0x492edb, 6) ; fmul [0.75] (0x492ee1, 6) → 12 bytes contiguous.
    let (i75, i75e) = build(&mut emu, 0x492edb, 12, "mul075");
    // |G−x|:  fld [G] (0x492f1b, 6) ; fsub [ebp-4] (0x492f21, 3) ; fabs (0x492f24, 2) → 11 bytes.
    let (iad, iade) = build(&mut emu, 0x492f1b, 11, "abs_diff");

    let run1 = |emu: &mut Emu, isl: u64, end: u64, g: f32| -> f32 {
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(G_FRAC, g.to_bits());
        emu.run(isl, end);
        emu.read_f32(cell)
    };
    let run_ad = |emu: &mut Emu, g: f32, x: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(G_FRAC, g.to_bits());
        emu.write_u32(f.ebp - 0x4, x.to_bits());
        emu.run(iad, iade);
        emu.read_f32(cell)
    };

    let mut total = 0u32;
    let gs = [0.0f32, 0.25, 0.5, 0.75, 0.9, 1.0, 1.3333334, -0.5, 100.0];
    for g in gs {
        assert_eq!(
            run1(&mut emu, i3, i3e, g).to_bits(),
            mul3(g).to_bits(),
            "mul3 g={g}"
        );
        assert_eq!(
            run1(&mut emu, i75, i75e, g).to_bits(),
            mul075(g).to_bits(),
            "mul075 g={g}"
        );
        total += 2;
    }
    let pairs = [
        (0.5f32, 0.5f32),
        (0.5, 0.501),
        (0.9, 0.0),
        (1.0, 0.999),
        (-3.0, 3.0),
        (0.6399999, 0.64),
    ];
    for (g, x) in pairs {
        assert_eq!(
            run_ad(&mut emu, g, x).to_bits(),
            abs_diff(g, x).to_bits(),
            "abs_diff g={g} x={x}"
        );
        total += 1;
    }
    let mut s = 0x492e_9000u32;
    for _ in 0..3000 {
        let g = rng(&mut s, -50.0, 50.0);
        assert_eq!(
            run1(&mut emu, i3, i3e, g).to_bits(),
            mul3(g).to_bits(),
            "mul3 fuzz g={g}"
        );
        assert_eq!(
            run1(&mut emu, i75, i75e, g).to_bits(),
            mul075(g).to_bits(),
            "mul075 fuzz g={g}"
        );
        let x = rng(&mut s, -50.0, 50.0);
        assert_eq!(
            run_ad(&mut emu, g, x).to_bits(),
            abs_diff(g, x).to_bits(),
            "abs_diff fuzz g={g} x={x}"
        );
        total += 3;
    }
    eprintln!("glue_492_492e90_fraction_clamp: {total} bit-exact (islands: fmul[3.0], fmul[0.75], fsub+fabs)");
}

// ===========================================================================================
// 0x492f70 — as 0x492e90 plus the reciprocal-unit step `768.0 / (i32)count` (0x492fc4 fild;
// 0x492fc7 fdivr; 0x492fcd fstp [ebp+8]). The fdivr stores to a mappable local, so it is a REAL
// slice; the `·3.0`/`·0.75`/`|G−x|` are the SAME shared primitives (islands again). arithmetic:
// fdivr×1 (slice) + fmul×2 + fsub×1 + fabs×1 (islands) = 5, all covered.
// ===========================================================================================
#[test]
fn glue_492_492f70_recip_unit() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let cell = emu.alloc(4);

    // (a) the reciprocal-unit REAL slice: set [ebp-8]=count(i32); run fild;fdivr[768.0];fstp[ebp+8].
    let run_recip = |emu: &mut Emu, count: i32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x8, count as u32);
        emu.run(0x492fc4, 0x492fd0); // fild [ebp-8]; fdivr [768.0]; fstp [ebp+8]
        emu.read_f32(f.ebp + 0x8)
    };

    // (b) shared islands (same bytes/shapes as 0x492e90, re-verified at 0x492f70's own addresses).
    let build = |emu: &mut Emu, srcva: u64, len: usize, label: &str| -> (u64, u64) {
        let isl = emu.alloc(0x20);
        let src = emu.read(srcva, len);
        let mut code = src.clone();
        code.extend_from_slice(&[0xd9, 0x1d]);
        code.extend_from_slice(&(cell as u32).to_le_bytes());
        let end = isl + code.len() as u64;
        emu.write(isl, &code);
        assert_eq!(
            &emu.read(isl, len),
            &emu.read(srcva, len),
            "{label} island bytes != src"
        );
        (isl, end)
    };
    // G·3.0:  fld [ebp-4] (0x492fd0, 3) ; fmul [3.0] (0x492fd3, 6) → 9 bytes.
    let (i3, i3e) = build(&mut emu, 0x492fd0, 9, "mul3");
    // G·0.75: fld [ebp-4] (0x492fe6, 3) ; fmul [0.75] (0x492fe9, 6) → 9 bytes.
    let (i75, i75e) = build(&mut emu, 0x492fe6, 9, "mul075");
    // |G−x|:  fld [G] (0x493022, 6) ; fsub [ebp-4] (0x493028, 3) ; fabs (0x49302b, 2) → 11 bytes.
    let (iad, iade) = build(&mut emu, 0x493022, 11, "abs_diff");

    let run_mul = |emu: &mut Emu, isl: u64, end: u64, g: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x4, g.to_bits()); // [ebp-4] is the source operand here
        emu.run(isl, end);
        emu.read_f32(cell)
    };
    let run_ad = |emu: &mut Emu, g: f32, x: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(G_FRAC, g.to_bits());
        emu.write_u32(f.ebp - 0x4, x.to_bits());
        emu.run(iad, iade);
        emu.read_f32(cell)
    };

    let mut total = 0u32;
    // recip: the path entry is count > 0x300 (=768); probe boundary + spread.
    for count in [
        769i32,
        770,
        800,
        1024,
        1536,
        3072,
        100_000,
        2_000_000,
        0x7fff_ffff,
    ] {
        assert_eq!(
            run_recip(&mut emu, count).to_bits(),
            recip_unit(count).to_bits(),
            "recip count={count}"
        );
        total += 1;
    }
    let gs = [0.0f32, 0.25, 0.5, 0.75, 0.9, 1.0, 1.3333334, -0.5, 100.0];
    for g in gs {
        assert_eq!(
            run_mul(&mut emu, i3, i3e, g).to_bits(),
            mul3(g).to_bits(),
            "mul3 g={g}"
        );
        assert_eq!(
            run_mul(&mut emu, i75, i75e, g).to_bits(),
            mul075(g).to_bits(),
            "mul075 g={g}"
        );
        total += 2;
    }
    let pairs = [(0.5f32, 0.5f32), (0.9, 0.0), (1.0, 0.999), (-3.0, 3.0)];
    for (g, x) in pairs {
        assert_eq!(
            run_ad(&mut emu, g, x).to_bits(),
            abs_diff(g, x).to_bits(),
            "abs_diff g={g} x={x}"
        );
        total += 1;
    }
    let mut s = 0x492f_7000u32;
    for _ in 0..3000 {
        let count = 0x301 + ((next(&mut s) >> 8) % 4_000_000) as i32;
        assert_eq!(
            run_recip(&mut emu, count).to_bits(),
            recip_unit(count).to_bits(),
            "recip fuzz count={count}"
        );
        let g = rng(&mut s, -50.0, 50.0);
        assert_eq!(
            run_mul(&mut emu, i3, i3e, g).to_bits(),
            mul3(g).to_bits(),
            "mul3 fuzz g={g}"
        );
        assert_eq!(
            run_mul(&mut emu, i75, i75e, g).to_bits(),
            mul075(g).to_bits(),
            "mul075 fuzz g={g}"
        );
        let x = rng(&mut s, -50.0, 50.0);
        assert_eq!(
            run_ad(&mut emu, g, x).to_bits(),
            abs_diff(g, x).to_bits(),
            "abs_diff fuzz g={g} x={x}"
        );
        total += 4;
    }
    eprintln!("glue_492_492f70_recip_unit: {total} bit-exact (slice fdivr[768]; islands fmul[3.0],fmul[0.75],fsub+fabs)");
}

// ===========================================================================================
// 0x4930d0 — threshold setter. The owned FP is the EXACT const fold (10+4)²=196.0 stored to the
// radius² global [0xb4e2c0]: `fld [10.0]; fadd [4.0]; fld st(0); fmul st,st(1); fstp [0xb4e2c0]`.
// The store target is a mappable BSS global, so this is a REAL slice (fadd×1 + fmul×1 = 2 ops),
// read directly from [0xb4e2c0]. There is NO fcomp here, so no threshold probe applies.
// ===========================================================================================
#[test]
fn glue_492_4930d0_threshold_fold() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // The slice 0x49315e..0x493177 has interleaved `pop edi/esi/ebx` (need valid ESP), runs:
    //   fld [10.0]; pop edi; fadd [4.0]; pop esi; pop ebx; fld st0; fmul st,st1; fstp [0xb4e2c0].
    let mut total = 0u32;
    emu.set_reg(RegisterX86::ESP, f.esp);
    emu.write_u32(G_THRESH, 0xdead_beef); // poison so we know it was written
    emu.run(0x49315e, 0x493177); // through `fstp [0xb4e2c0]`, halt at `fstp st(0)`
    let got = emu.read_f32(G_THRESH);
    assert_eq!(
        got.to_bits(),
        fold_196().to_bits(),
        "fold_196 vs binary slice"
    );
    assert_eq!(got.to_bits(), 196.0f32.to_bits(), "fold == 196.0 exact");
    total += 2;
    eprintln!("glue_492_4930d0_threshold_fold: {total} bit-exact (fadd[4.0] + fmul t·t → 196.0 @ [0xb4e2c0])");
}
