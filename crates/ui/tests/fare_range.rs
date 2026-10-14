//! Differential tests of the arithmetic of `GetTaxiNodeFare` (`0x4dc3d0`), `ActionSlotHasRange`
//! (`0x4e5810`), the negated fans of `0x4ea790` and `0x4ea850`, and the halving of `0x4ee100`,
//! each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed against the binary's own bytes:
//! * **0x4dc3d0** — `(1.0 - r)·n` then `fstp`. The owned `fsubr` + `fmul` are diffed via a COPIED-BYTE
//!   ISLAND: the source interleaves unmapped object-graph derefs (`mov eax,[0xbb4a80]; mov ecx,[eax]; …`)
//!   between the `fsubr` and the `fild`/`fmul`, so an isolated slice would fault. The island loads `r`
//!   (`fld qword`) and `n` (`fild dword`) into the x87 stack, then runs the EXACT source `fsubr`+`fmul`
//!   bytes (asserted byte-equal to 0x4dc462/0x4dc47e — and to block #2 0x4dc4a7/0x4dc4b3, which are
//!   byte-identical) and `fstp`s the f32 product. fsubr ×2 + fmul ×2 covered.
//! * **0x4e5810** — `|x| >= eps OR |y| >= eps`. Diffed as an instruction SLICE `0x4e5880 → ret`: set the
//!   two getter outputs `[ebp-8]`/`[ebp-0xc]`, run the two `fld;fabs;fcomp[eps];fnstsw;test ah,5;j(n)p`
//!   blocks natively, read back EAX (0/1). fabs ×2 + both `fcomp ah,5` branches covered. Carries the
//!   ==-boundary probe at exactly `eps` and the NaN/Inf unordered-path probe.
//! * **0x4ea790 / 0x4ea850** — `fan g/-g`. 0x4ea850 is the byte-clone (verified here). The representative
//!   0x4ea790 is diffed as a WHOLE-FN SLICE `0x4ea790 → 0x4ea816 ret` (no prologue, no faulting derefs —
//!   every operand is an absolute mapped global): write `g` into `[0xbc762c]`, run, read the 8 f32 stores
//!   + 4 zeroed cells. fchs ×1 covered (and inherited by the clone).
//! * **0x4ee100** — two getter floats `·0.5`. COPIED-BYTE ISLAND (source interleaves unmapped vtable
//!   dispatches `call [eax+0x20]` / `call [edx+0x1c]` that leave the float on `st0`): the island loads `v`
//!   (`fld qword`) then runs the EXACT source `fmul [0.5]` bytes (asserted byte-equal to 0x4ee11b AND
//!   0x4ee133, which are identical) and `fstp`s. fmul ×2 covered.
//!
//! Fuzz reads the LCG's HIGH bits. Bit-exact = `.to_bits() == .to_bits()`.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::fare_range::{
    fan_neg, half, mag_ge_eps, near_zero_pair, EPS_BITS, HALF_BITS, ONE_BITS,
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
// 0x4dc3d0 — `(1.0 - r)·n → f32`. Copied-byte island of the two owned ops `fsubr [1.0]` + `fmul`.
// Source FP op bytes (verbatim): fsubr = d8 2d d8 f9 7f 00 ; fmul st,st(1) = d8 c9.
// ===========================================================================================
#[test]
fn glue_4dc_4dc3d0_fade_step() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let cell_r = emu.alloc(8); // f64 input r (the callee's st0)
    let cell_n = emu.alloc(4); // i32 input n (the fild operand)
    let cell_out = emu.alloc(4); // f32 product store
    let cell_int = emu.alloc(4); // i32 from the reload and `fistp`

    // Island: fld qword [cell_r]; <SRC fsubr>; fild dword [cell_n]; <SRC fmul>; fstp dword [cell_out].
    let isl = emu.alloc(0x20);
    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&[0xdd, 0x05]); // fld qword [cell_r]
    code.extend_from_slice(&(cell_r as u32).to_le_bytes());
    let fsubr_off = code.len();
    code.extend_from_slice(&[0xd8, 0x2d, 0xd8, 0xf9, 0x7f, 0x00]); // SRC fsubr [0x7ff9d8]
    code.extend_from_slice(&[0xdb, 0x05]); // fild dword [cell_n]
    code.extend_from_slice(&(cell_n as u32).to_le_bytes());
    let fmul_off = code.len();
    code.extend_from_slice(&[0xd8, 0xc9]); // SRC fmul st,st(1)
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [cell_out]
    code.extend_from_slice(&(cell_out as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x05]); // fld dword [cell_out]   == src 0x4dc485
    code.extend_from_slice(&(cell_out as u32).to_le_bytes());
    code.extend_from_slice(&[0xdb, 0x1d]); // fistp dword [cell_int] == src 0x4dc488
    code.extend_from_slice(&(cell_int as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // an island is credited only when its FP-op bytes match the real source. Verify the
    // copied fsubr/fmul against BOTH blocks (block #1 0x4dc462/0x4dc47e, block #2 0x4dc4a7/0x4dc4b3 —
    // byte-identical, so the one island covers all 4 owned ops).
    assert_eq!(
        &emu.read(isl + fsubr_off as u64, 6),
        &emu.read(0x4dc462, 6),
        "island fsubr != src block#1"
    );
    assert_eq!(
        &emu.read(0x4dc462, 6),
        &emu.read(0x4dc4a7, 6),
        "block#1 fsubr != block#2"
    );
    assert_eq!(
        &emu.read(isl + fmul_off as u64, 2),
        &emu.read(0x4dc47e, 2),
        "island fmul != src block#1"
    );
    assert_eq!(
        &emu.read(0x4dc47e, 2),
        &emu.read(0x4dc4b3, 2),
        "block#1 fmul != block#2"
    );

    // the binary's own 1.0 constant cell drives `fsubr` — the transcription reads the same bits.
    assert_eq!(emu.read_u32(0x7ff9d8), ONE_BITS, "1.0 const drift");

    // The reload and `fistp` bytes are the source's (`fld dword [ebp-4]; fistp dword [ebp-8]`) with
    // the operands re-pointed.
    assert_eq!(&emu.read(0x4dc485, 2), &[0xd9, 0x45], "src reload");
    assert_eq!(&emu.read(0x4dc488, 2), &[0xdb, 0x5d], "src fistp");

    let run = |emu: &mut Emu, r: f64, n: i32| -> (f32, i32) {
        emu.write(cell_r, &r.to_bits().to_le_bytes());
        emu.write_u32(cell_n, n as u32);
        emu.run(isl, isl_end);
        (emu.read_f32(cell_out), emu.read_u32(cell_int) as i32)
    };
    // A NaN product compares as NaN: the x87's default NaN is negative and the host's is not.
    let same = |a: f32, b: f32| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());

    let mut total = 0u32;
    // deterministic probes: r = the callee's reachable outputs (0.0, 0.1, 0.15, 0.2 — f32 sums in f64),
    // plus edge r and a spread of n (incl. negatives, large, the alpha range).
    let r_probes = [
        0.0f64,
        f64::from(f32::from_bits(0x3dcccccd)), // 0.1
        f64::from(f32::from_bits(0x3dcccccd)) + f64::from(f32::from_bits(0x3d4ccccd)), // 0.1+0.05
        f64::from(f32::from_bits(0x3dcccccd)) + 2.0 * f64::from(f32::from_bits(0x3d4ccccd)),
        1.0,
        0.5,
        -0.25,
        1.0e10,
        // NaN, the infinities and values whose product with `n` passes `i32` and `i64`: the
        // `fistp` stores `0x80000000`.
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e30,
        -1.0e30,
        -3.0e9,
    ];
    let n_probes = [
        0i32,
        1,
        2,
        255,
        256,
        -1,
        -255,
        1000,
        65535,
        -65536,
        i32::MAX,
        i32::MIN,
    ];
    for &r in &r_probes {
        for &n in &n_probes {
            let (got, got_int) = run(&mut emu, r, n);
            let want = benilla_re_ui::fare_range::fade_step(r, n);
            assert!(same(got, want), "fade_step r={r} n={n}");
            assert_eq!(
                got_int,
                benilla_re_ui::fare_range::fade_step_round(r, n),
                "fade_step_round r={r} n={n}"
            );
            total += 1;
        }
    }
    // fuzz r in [0,1] (the natural alpha range) and a wider band; n across the integer spread.
    let mut s = 0x4dc3_d000u32;
    for _ in 0..6000 {
        let r = f64::from(rng(&mut s, -2.0, 2.0));
        let n = (next(&mut s) >> 8) as i32 - 0x40_0000;
        let (got, got_int) = run(&mut emu, r, n);
        let want = benilla_re_ui::fare_range::fade_step(r, n);
        assert_eq!(got.to_bits(), want.to_bits(), "fade_step fuzz r={r} n={n}");
        assert_eq!(
            got_int,
            benilla_re_ui::fare_range::fade_step_round(r, n),
            "fade_step_round fuzz r={r} n={n}"
        );
        total += 1;
    }
    eprintln!("glue_4dc_4dc3d0_fade_step: {total} bit-exact (island fsubr[1.0]+fmul, 4 owned ops)");
}

// ===========================================================================================
// 0x4e5810 — `|x| >= eps OR |y| >= eps` (eps = 0x34800000 = 2^-22). Instruction slice
// 0x4e5880 → ret, reading EAX. fld;fabs;fcomp[eps];fnstsw;test ah,5;jp  ×2 (the second jnp).
// ===========================================================================================
#[test]
fn glue_4dc_4e5810_near_zero_pair() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // a frame: EBP with [ebp-8]=x, [ebp-0xc]=y, [ebp]=old-ebp, [ebp+4]=halt sentinel.
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let sentinel: u64 = 0x6f6f_6f6f; // unmapped halt target the epilogue `ret` lands on
    let stack = emu.alloc(0x1000);
    let esp = stack + 0x800;

    // the binary's own eps constant cell drives both `fcomp` — the transcription reads the same bits.
    assert_eq!(emu.read_u32(0x8029d4), EPS_BITS, "eps const drift");

    let run = |emu: &mut Emu, x: f32, y: f32| -> i32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(ebp, 0xdead_beef); // old ebp (popped by epilogue)
        emu.write_u32(ebp + 4, sentinel as u32); // return addr → halt
        emu.write_u32(ebp - 0x8, x.to_bits()); // [ebp-8] = x
        emu.write_u32(ebp - 0xc, y.to_bits()); // [ebp-0xc] = y
                                               // 0x4e5880 fld [ebp-8] … through the epilogue `ret` which jumps to `sentinel`.
        emu.run(0x4e5880, sentinel);
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let eps = f32::from_bits(EPS_BITS);
    let just_below = f32::from_bits(EPS_BITS - 1); // largest f32 < eps
    let just_above = f32::from_bits(EPS_BITS + 1); // smallest f32 > eps
    let mut total = 0u32;

    // ==-boundary + special probes (each as x, and as y with x=0).
    let probes = [
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
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x0000_0001), // smallest subnormal
        f32::MIN_POSITIVE,
    ];
    // exhaustive cross of probes as (x,y) — covers short-circuit (x large) and fallthrough (x tiny).
    for &x in &probes {
        for &y in &probes {
            let got = run(&mut emu, x, y);
            let want = near_zero_pair(x, y);
            assert_eq!(got, want, "near_zero x={x} y={y}");
            // and the per-value helper against the binary via x-only (y forced tiny → only x decides
            // the short-circuit-true case; the fallthrough case is exercised by x=0 above).
            total += 1;
        }
    }
    // explicit single-predicate corroboration: feed x, y=0.0 → result == (mag_ge_eps(x) as i32).
    for &x in &probes {
        let got = run(&mut emu, x, 0.0);
        assert_eq!(got, i32::from(mag_ge_eps(x)), "mag_ge_eps via x={x}");
        total += 1;
    }
    // fuzz near the threshold (tight band around eps) and broad.
    let mut s = 0x4e58_1000u32;
    for _ in 0..6000 {
        let x = rng(&mut s, -3.0e-7, 3.0e-7); // straddles eps=2.38e-7
        let y = rng(&mut s, -10.0, 10.0);
        let got = run(&mut emu, x, y);
        assert_eq!(got, near_zero_pair(x, y), "near_zero fuzz x={x} y={y}");
        total += 1;
    }
    eprintln!("glue_4dc_4e5810_near_zero_pair: {total} bit-exact (|x|>=eps OR |y|>=eps; fabs×2)");
}

// ===========================================================================================
// 0x4ea790 / 0x4ea850 — fan g/-g. Clone verified, representative 0x4ea790 diffed whole-fn-slice.
// ===========================================================================================
#[test]
fn glue_4dc_4ea790_fan_neg() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // --- clone verdict: 0x4ea850 is byte-identical to 0x4ea790 modulo masked disp32 operands. ---
    // Every store/load operand is a disp32 absolute; the source global differs (0xbc762c vs 0xbc7630)
    // and the destination cells differ (0xbc81f8.. vs 0xbc8230..). Mask each disp32 (and the mov-imm32,
    // which is 0 in both) and assert the remainder is identical — proving the inherited fchs.
    let a = emu.read(0x4ea790, 134);
    let b = emu.read(0x4ea850, 134);
    let mut am = a.clone();
    let mut bm = b.clone();
    // disp32/imm32 byte ranges within the 134-byte body (offsets, from the disasm):
    let masks: [(usize, usize); 17] = [
        (2, 6),
        (8, 16), // mov [abs],imm : disp32 + imm32
        (20, 28),
        (32, 40),
        (44, 52),
        (56, 60),
        (62, 66),
        (68, 72),
        (78, 82),
        (84, 88),
        (90, 94),
        (98, 102),
        (104, 108),
        (110, 114),
        (116, 120),
        (124, 128),
        (130, 134),
    ];
    for (s, e) in masks {
        for i in s..e {
            am[i] = 0;
            bm[i] = 0;
        }
    }
    assert_eq!(am, bm, "0x4ea850 is NOT a masked-disp32 clone of 0x4ea790");
    // and assert every byte that DID differ fell inside a masked range (no un-accounted diff).
    for i in 0..134 {
        if a[i] != b[i] {
            assert!(
                masks.iter().any(|&(s, e)| s <= i && i < e),
                "unmasked diff at off {i}"
            );
        }
    }

    // --- diff the representative 0x4ea790 as a whole-fn slice (no prologue, no faulting deref). ---
    const G: u64 = 0x00bc_762c; // source global the fld reads
    const BASE: u64 = 0x00bc_81f8; // first cell of the 12-slot block ([+4·i] for i in 0..12)

    let run = |emu: &mut Emu, g: f32| -> [f32; 12] {
        // pre-poison the 12 destination cells so a missing store / wrong slot can't masquerade as
        // a match (the 8 FP stores + 4 integer `mov [m],0` must each overwrite the poison).
        for i in 0..12u64 {
            emu.write_u32(BASE + 4 * i, 0x7fc0_0000); // qNaN poison
        }
        emu.write_u32(G, g.to_bits());
        emu.run(0x4ea790, 0x4ea816); // through the ret (not executed)
        let mut cells = [0f32; 12];
        for (i, c) in cells.iter_mut().enumerate() {
            *c = emu.read_f32(BASE + 4 * i as u64);
        }
        cells
    };

    let mut total = 0u32;
    let probes = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -123.456,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
        1.0e30,
    ];
    let check = |emu: &mut Emu, g: f32, total: &mut u32| {
        let cells = run(emu, g);
        let want = fan_neg(g);
        for i in 0..12 {
            assert_eq!(cells[i].to_bits(), want[i].to_bits(), "fan cell[{i}] g={g}");
            *total += 1;
        }
    };
    for g in probes {
        check(&mut emu, g, &mut total);
    }
    let mut s = 0x4ea7_9000u32;
    for _ in 0..3000 {
        let g = rng(&mut s, -1.0e6, 1.0e6);
        check(&mut emu, g, &mut total);
    }
    eprintln!(
        "glue_4dc_4ea790_fan_neg: {total} bit-exact (fchs fan g/-g into 12-cell block; clone 0x4ea850 inherited)"
    );
}

// ===========================================================================================
// 0x4ee100 — two getter floats `·0.5`. Copied-byte island of the owned `fmul [0.5]`.
// Source FP op bytes (verbatim, both 0x4ee11b & 0x4ee133): d8 0d 24 fa 7f 00.
// ===========================================================================================
#[test]
fn glue_4dc_4ee100_half_pair() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let cell_in = emu.alloc(8); // f64 input v (the getter's st0)
    let cell_out = emu.alloc(4); // f32 product

    // Island: fld qword [cell_in]; <SRC fmul [0.5]>; fstp dword [cell_out].
    let isl = emu.alloc(0x20);
    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&[0xdd, 0x05]); // fld qword [cell_in]
    code.extend_from_slice(&(cell_in as u32).to_le_bytes());
    let fmul_off = code.len();
    code.extend_from_slice(&[0xd8, 0x0d, 0x24, 0xfa, 0x7f, 0x00]); // SRC fmul [0x7ffa24]
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [cell_out]
    code.extend_from_slice(&(cell_out as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // verify the copied fmul against BOTH source sites (identical → one island covers both).
    assert_eq!(
        &emu.read(isl + fmul_off as u64, 6),
        &emu.read(0x4ee11b, 6),
        "island fmul != src #1 (0x4ee11b)"
    );
    assert_eq!(
        &emu.read(0x4ee11b, 6),
        &emu.read(0x4ee133, 6),
        "src fmul #1 != #2"
    );
    // the binary's own 0.5 constant cell drives `fmul` — the transcription reads the same bits.
    assert_eq!(emu.read_u32(0x7ffa24), HALF_BITS, "0.5 const drift");

    let run = |emu: &mut Emu, v: f64| -> f32 {
        emu.write(cell_in, &v.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        emu.read_f32(cell_out)
    };

    let mut total = 0u32;
    let probes = [
        0.0f64,
        -0.0,
        1.0,
        -1.0,
        0.5,
        2.0,
        3.0,
        f64::from(f32::from_bits(0x4b00_0001)), // 2^23+1: halving an odd integer (round-to-even)
        f64::from(1.0e7f32),
        -123.456,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from(f32::MIN_POSITIVE),
    ];
    for &v in &probes {
        let got = run(&mut emu, v);
        let want = half(v);
        assert_eq!(got.to_bits(), want.to_bits(), "half v={v}");
        total += 1;
    }
    let mut s = 0x4ee1_0000u32;
    for _ in 0..6000 {
        let v = f64::from(rng(&mut s, -1.0e6, 1.0e6));
        let got = run(&mut emu, v);
        let want = half(v);
        assert_eq!(got.to_bits(), want.to_bits(), "half fuzz v={v}");
        total += 1;
    }
    eprintln!("glue_4dc_4ee100_half_pair: {total} bit-exact (island fmul[0.5], 2 owned ops)");
}
