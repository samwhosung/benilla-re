//! Differential tests of the scale and gate arithmetic of the world-map functions `SetMap`
//! (`0x4a67a0`), `GetSelectedTextureHeight` (`0x4a6d50`), `0x4a7a80`, `CenterArrowModel`
//! (`0x4a7b20`) and `CreateMiniArrowModel` (`0x4a7b80`), each run against `WoW.exe` in the
//! emulator.
//!
//! Each owned FP slice is diffed as the binary's own bytes:
//! * `0x4a67a0` — six `fld [m]; fabs; fcomp [eps]` near-zero gates. The `fabs` *value* is diffed via
//!   a copied-byte island (`fld cell; <src fabs>; fstp cell`, the `fabs` byte asserted == source),
//!   and the real `fld; fabs; fcomp [eps]; fnstsw` slice is run at the true site so the `fcomp`
//!   branch outcome (`|x| < eps`) is diffed bit-for-bit incl. the `|x| == eps` boundary probe.
//! * `0x4a6d50` — `(c-d)·1002.0/(a-b)` then `__ftol`: the `fld;fsub;fld;fsub;…;fmul[1002];fdiv;
//!   call __ftol` slice run at the true site (covers `fsub ×2 + fmul + fdiv`, `__ftol` native),
//!   reading `EAX`.
//! * `0x4a7a80`/`0x4a7b20`/`0x4a7b80` — the owned `fmul [const]` operand sits on an `st0` produced by
//!   a delegated call (the `0x41ae70` helper / vtable getters), so each `fmul` is diffed via a
//!   copied-byte island (`fld cell; <src fmul[const]>; fstp cell`, the `fmul` bytes asserted ==
//!   source). `0x4a7b20`'s two `fmul [0.5]` are byte-identical (one island, twin asserted);
//!   `0x4a7b80` is a near-clone of `0x4a7a80` (only the `fmul` constant differs among the FP ops).
//!
//! Fuzz reads the LCG's HIGH bits. PASS = every `.to_bits()` equal.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::worldmap_scale::{
    aspect_ftol, below_eps, fabs, scale, EPS_BITS, FIVE_THIRDS_BITS, HALF_BITS, TEN_NINTHS_BITS,
};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// ===========================================================================================
// 0x4a67a0 — six `fld [m]; fabs; fcomp [eps]` near-zero gates (fabs ×6 @ 0x4a686b/687d/69db/
// 69ed/6a30/6a42). The six sites share the byte-identical `fabs` (d9 e1) + `fcomp [0x8029d4]`
// (d8 1d d4 29 80 00); only the `fld` source operand differs. We diff:
//   (a) the fabs VALUE via a copied-byte island (fld cell; <src fabs>; fstp cell), and
//   (b) the real `fld [m]; fabs; fcomp [eps]; fnstsw` slice at site #1 (0x4a6868), reading AH and
//       checking the `test ah,5; jp` outcome (|x| < eps) — incl. the |x| == eps boundary probe.
// ===========================================================================================
#[test]
fn glue_4a6_4a67a0_near_zero_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut total = 0u32;

    // --- (a) fabs value island: fld [cell] (D9 05 + disp32); <src fabs d9 e1>; fstp [cell2]. ---
    let cell_in = emu.alloc(4);
    let cell_out = emu.alloc(4);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [cell_in]
    code.extend_from_slice(&(cell_in as u32).to_le_bytes());
    let fabs_at = code.len();
    code.extend_from_slice(&[0xd9, 0xe1]); // fabs (copied verbatim below; verified vs source)
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [cell_out]
    code.extend_from_slice(&(cell_out as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    // byte-verify the island `fabs` against EVERY source site (all six must be the same opcode).
    for &src in &[
        0x4a686bu64,
        0x4a687d,
        0x4a69db,
        0x4a69ed,
        0x4a6a30,
        0x4a6a42,
    ] {
        assert_eq!(
            &emu.read(isl + fabs_at as u64, 2),
            &emu.read(src, 2),
            "island fabs != src @ {src:#x}"
        );
    }
    let run_fabs = |emu: &mut Emu, x: f32| -> f32 {
        emu.write_u32(cell_in, x.to_bits());
        emu.run(isl, isl_end);
        emu.read_f32(cell_out)
    };
    for x in [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        1234.5,
        -1234.5,
        f32::from_bits(EPS_BITS),
        -f32::from_bits(EPS_BITS),
        f32::MIN_POSITIVE,
        f32::MAX,
        -f32::MAX,
    ] {
        assert_eq!(
            run_fabs(&mut emu, x).to_bits(),
            fabs(x).to_bits(),
            "fabs x={x}"
        );
        total += 1;
    }
    let mut s = 0x4a67_a000u32;
    for _ in 0..2000 {
        let x = rng(&mut s, -1.0e6, 1.0e6);
        assert_eq!(
            run_fabs(&mut emu, x).to_bits(),
            fabs(x).to_bits(),
            "fabs fuzz x={x}"
        );
        total += 1;
    }

    // --- (b) real near-zero gate at site #1: 0x4a6868 fld [ebp-0xc]; fabs; fcomp [eps]; fnstsw. ---
    // Run 0x4a6868 -> 0x4a6875 (after `fnstsw ax`, before `test ah,5`); read AH; the `test ah,5; jp`
    // takes the work branch when |x| >= eps (PF=1), falls through (near-zero) when |x| < eps (PF=0).
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    let run_gate = |emu: &mut Emu, x: f32| -> bool {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(ebp - 0xc, x.to_bits()); // [ebp-0xc] is site #1's fld operand
        emu.run(0x4a6868, 0x4a6875);
        let ah = (emu.get_reg(RegisterX86::EAX) >> 8) & 0xff;
        // `test ah,5; jp` -> jp taken iff PF=1 (even parity of ah&5). Fall-through (below_eps) = PF=0.
        ((ah & 0x05).count_ones() % 2) == 1
    };
    // ==-boundary probe: |x| exactly at eps (C3 set -> not below), and just-below / just-above.
    let eps = f32::from_bits(EPS_BITS);
    let just_below = f32::from_bits(EPS_BITS - 1);
    let just_above = f32::from_bits(EPS_BITS + 1);
    for x in [
        0.0f32,
        -0.0,
        eps,
        -eps,
        just_below,
        -just_below,
        just_above,
        -just_above,
        1.0,
        -1.0,
        1e-9,
        -1e-9,
        1e6,
        -1e6,
    ] {
        assert_eq!(run_gate(&mut emu, x), below_eps(x), "near-zero gate x={x}");
        total += 1;
    }
    let mut s = 0x4a68_6800u32;
    for _ in 0..2000 {
        // spread across the eps boundary: most around it, some far.
        let x = if next(&mut s) & 1 == 0 {
            rng(&mut s, -8.0e-7, 8.0e-7)
        } else {
            rng(&mut s, -1.0e4, 1.0e4)
        };
        assert_eq!(
            run_gate(&mut emu, x),
            below_eps(x),
            "near-zero gate fuzz x={x}"
        );
        total += 1;
    }
    eprintln!(
        "glue_4a6_4a67a0_near_zero_gate: {total} bit-exact (fabs value island + |x|<eps gate)"
    );
}

// ===========================================================================================
// 0x4a6d50 — (c-d)·1002.0 / (a-b) then __ftol. Slice 0x4a6e18..0x4a6e4d (the `edx != -1` fld
// path), reading EAX after __ftol returns. Covers fsub ×2 (0x4a6e1b/6e21) + fmul (0x4a6e40
// [1002.0]) + fdiv (0x4a6e46 st/st1). __ftol (0x40a2b0) runs natively (truncate toward zero).
// ecx -> struct {[+0x10]=a, [+0x14]=b, [+0x18]=c, [+0x1c]=d}.
// ===========================================================================================
#[test]
fn glue_4a6_4a6d50_aspect_ftol() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let st = emu.alloc(0x40);
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    let ebp = stack + 0xf80; // __ftol uses ebp/esp; point at mapped stack

    let run = |emu: &mut Emu, a: f32, b: f32, c: f32, d: f32| -> i32 {
        emu.set_reg(RegisterX86::ECX, st);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.write_u32(st + 0x10, a.to_bits());
        emu.write_u32(st + 0x14, b.to_bits());
        emu.write_u32(st + 0x18, c.to_bits());
        emu.write_u32(st + 0x1c, d.to_bits());
        emu.run(0x4a6e18, 0x4a6e4d); // fld;fsub;fld;fsub;...;fmul;fdiv;call __ftol — halt after call
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // deterministic probes: a-b and c-d nonzero (P==0 skips the leg — exercised separately below).
    for &(a, b, c, d) in &[
        (1.0f32, 0.0, 1.0, 0.0), // (1·1002)/1 = 1002
        (2.0, 0.0, 1.0, 0.0),    // (1·1002)/2 = 501
        (1.0, 0.0, 2.0, 0.0),    // (2·1002)/1 = 2004
        (4.0, 2.0, 3.0, 1.0),    // (2·1002)/2 = 1002
        (1.0, 3.0, 5.0, 2.0),    // (3·1002)/(-2) = -1503
        (10.0, 0.0, 1.0, 0.0),   // 100.2 -> 100
        (3.0, 0.0, 1.0, 0.0),    // 334.0 -> 334
        (7.0, 0.0, 1.0, 0.0),    // 143.14.. -> 143
        (-2.0, 0.0, 1.0, 0.0),   // (1002)/(-2) = -501
        (1.0, 0.0, -1.0, 0.0),   // -1002
        (1000.0, 0.0, 1.0, 0.0), // 1.002 -> 1
        (1001.0, 0.0, 1.0, 0.0), // 1.0009.. -> 1
        (1.0, 0.0, 0.0009, 0.0), // 0.9018 -> 0
    ] {
        assert_eq!(
            run(&mut emu, a, b, c, d),
            aspect_ftol(a, b, c, d),
            "aspect a={a} b={b} c={c} d={d}"
        );
        total += 1;
    }
    // P==0 boundary: a==b means P=0; the binary's `fcom 0.0; jnp 0x4a6e55` skips the fmul/fdiv leg
    // (so this slice would NOT reach the call). The transcription path is gated upstream in the
    // binary — we therefore fuzz only P != 0 (and document the boundary).
    let mut s = 0x4a6d_5000u32;
    for _ in 0..4000 {
        let a = rng(&mut s, -1000.0, 1000.0);
        let mut b = rng(&mut s, -1000.0, 1000.0);
        if (f64::from(a) - f64::from(b)).abs() < 1e-3 {
            b += 5.0; // keep P safely away from 0 and the quotient within i32 range
        }
        let c = rng(&mut s, -1000.0, 1000.0);
        let d = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, a, b, c, d),
            aspect_ftol(a, b, c, d),
            "aspect fuzz a={a} b={b} c={c} d={d}"
        );
        total += 1;
    }
    eprintln!(
        "glue_4a6_4a6d50_aspect_ftol: {total} bit-exact ((c-d)·1002/(a-b) then __ftol truncate)"
    );
}

// ===========================================================================================
// 0x4a7a80 / 0x4a7b80 / 0x4a7b20 — owned `fmul [const]` on a delegated-call `st0`. Each `fmul` is
// diffed via a copied-byte island: fld [cell] (D9 05 + disp32); <src fmul bytes>; fstp [cell2].
//   0x4a7ad8 fmul [0x80655c] (5/3)   — 0x4a7a80
//   0x4a7bd8 fmul [0x806560] (10/9)  — 0x4a7b80 (near-clone of 0x4a7a80)
//   0x4a7b35 fmul [0x7ffa24] (0.5)   — 0x4a7b20 leg 1  (twin of leg 2)
//   0x4a7b45 fmul [0x7ffa24] (0.5)   — 0x4a7b20 leg 2  (byte-identical to leg 1)
// ===========================================================================================
#[test]
fn glue_4a6_4a7a80_4a7b20_4a7b80_fmul_scales() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // Build one fmul island per distinct site; each `fmul [const]` is 6 bytes (d8 0d + disp32).
    // (src_addr, kbits, label)
    let sites: [(u64, u32, &str); 3] = [
        (0x4a7ad8, FIVE_THIRDS_BITS, "0x4a7a80 ·5/3"),
        (0x4a7bd8, TEN_NINTHS_BITS, "0x4a7b80 ·10/9"),
        (0x4a7b35, HALF_BITS, "0x4a7b20 ·0.5"),
    ];
    // 0x4a7b20's two fmuls are byte-identical — assert the twin matches the representative source.
    assert_eq!(
        &emu.read(0x4a7b35, 6),
        &emu.read(0x4a7b45, 6),
        "0x4a7b20 fmul twin 0x4a7b45 != 0x4a7b35"
    );

    let mut total = 0u32;
    for &(src, kbits, label) in &sites {
        let cell_in = emu.alloc(4);
        let cell_out = emu.alloc(4);
        let isl = emu.alloc(0x20);
        let mut code = vec![0xd9u8, 0x05]; // fld dword [cell_in]
        code.extend_from_slice(&(cell_in as u32).to_le_bytes());
        let fmul_at = code.len();
        // copy the source `fmul [const]` bytes VERBATIM (d8 0d + disp32).
        let src_fmul = emu.read(src, 6);
        code.extend_from_slice(&src_fmul);
        code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [cell_out]
        code.extend_from_slice(&(cell_out as u32).to_le_bytes());
        let isl_end = isl + code.len() as u64;
        emu.write(isl, &code);
        // byte-verify the island fmul == source (islands credited only when bytes match).
        assert_eq!(
            &emu.read(isl + fmul_at as u64, 6),
            &emu.read(src, 6),
            "island fmul != src @ {src:#x}"
        );

        let run = |emu: &mut Emu, x: f32| -> f32 {
            emu.write_u32(cell_in, x.to_bits());
            emu.run(isl, isl_end);
            emu.read_f32(cell_out)
        };
        // probes incl. the binary's real input 0.6 (= 0x41ae70(1.0) = 0.6·1.0) for the 5/3 & 10/9
        // sites, and 1.0 (the vtable-getter inputs are arbitrary, so a broad fuzz covers them).
        for x in [
            0.0f32,
            -0.0,
            0.6,
            1.0,
            -1.0,
            0.5,
            2.0,
            1024.0,
            -1234.5,
            1.0e6,
            f32::from_bits(0x4b00_0001),
        ] {
            assert_eq!(
                run(&mut emu, x).to_bits(),
                scale(x, kbits).to_bits(),
                "{label} x={x}"
            );
            total += 1;
        }
        let mut s = (src as u32).wrapping_mul(0x1357);
        for _ in 0..3000 {
            let x = rng(&mut s, -10000.0, 10000.0);
            assert_eq!(
                run(&mut emu, x).to_bits(),
                scale(x, kbits).to_bits(),
                "{label} fuzz x={x}"
            );
            total += 1;
        }
    }
    eprintln!("glue_4a6_4a7a80_4a7b20_4a7b80_fmul_scales: {total} bit-exact (3 fmul[const] islands; 0x4a7b20 twin asserted)");
}
