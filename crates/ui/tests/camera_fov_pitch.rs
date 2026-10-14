//! Differential tests of the arithmetic of the camera setters `CCamera::SetTarget` (`0x50d0f0`),
//! `SetFovOverride` (`0x50d320`), `LeaveAbsoluteFrame` (`0x50d440`), `PopGroundPitch`
//! (`0x50d520`), `GetFov` (`0x50d6d0`), `ProbeSlope` (`0x50d900`) and `ArmTerrainTilt`
//! (`0x50dbc0`), each run against `WoW.exe` in the emulator.
//!
//! Cluster `[0x50d0f0, 0x50dbc0]` — the FP-bearing camera/transition glue. Each owned FP-arith op
//! is diffed as the binary's own bytes: a whole-function `call_f32` where synthesizable (0x50d6d0),
//! an `Emu::run` instruction slice where the surrounding setup is delegated, or a COPIED-BYTE
//! ISLAND (byte-verified against the disasm) for a `fchs`/clamp sequence that a mid-block `run`
//! would re-enter on a stale TB (mid-block hazard). Fuzz reads the LCG HIGH bits (low bits
//! periodic); every clamp/eps slice carries a deterministic `==threshold` boundary probe.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::camera_fov_pitch::{
    abs_lt_eps, clamp_tail, deadband_eq_any3, deg2rad_set_arg, neg_push, percent_scale,
    pitch_clamp_accum, step_mul, EPS_BITS, PITCH_HI_BITS,
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

// ===========================================================================================
// 0x50d320 — deg→rad set: fild [ebp+8]; fmul [0x7ffaac]=π/180; fstp [esi+0x40]. fmul×1.
// Slice 0x50d336..0x50d351 (after the call 0x512e90), read [esi+0x40].
// ===========================================================================================
#[test]
fn glue_50d320_deg2rad_set() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, arg: i32| -> f32 {
        emu.set_reg(RegisterX86::ESI, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg as u32);
        // 0x50d336 fild [ebp+8]; 0x50d33c..d348 mov-shuffle [esi+0x90]; 0x50d342 fmul; 0x50d34e fstp.
        emu.run(0x50d336, 0x50d351);
        emu.read_f32(f.this + 0x40)
    };

    let mut total = 0u32;
    // deterministic anchors: small ints exercise the fild→fmul rounding exactly.
    for arg in [1i32, 2, 90, 180, 360, -45, -360, 1000] {
        assert_eq!(
            run(&mut emu, arg).to_bits(),
            deg2rad_set_arg(arg).to_bits(),
            "deg2rad arg={arg}"
        );
        total += 1;
    }
    let mut s = 0x50d3_2000u32;
    for _ in 0..3000 {
        let arg = (next(&mut s) >> 8) as i32 % 200_000 - 100_000;
        if arg == 0 {
            continue; // arg==0 takes the π/2 const-store path (a different slice, not this one)
        }
        assert_eq!(
            run(&mut emu, arg).to_bits(),
            deg2rad_set_arg(arg).to_bits(),
            "deg2rad fuzz arg={arg}"
        );
        total += 1;
    }
    eprintln!("glue_50d320_deg2rad_set: {total} bit-exact (fild;fmul[π/180];fstp)");
}

// ===========================================================================================
// 0x50d440 / 0x50d520 — negate-push: fld src; fchs; fstp [out]. fchs×5 (byte-clone slices).
// One copied-byte island covers all five (byte-verified below).
// ===========================================================================================
#[test]
fn glue_50d440_neg_push() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let src_cell = emu.alloc(4);
    let out = emu.alloc(4);

    // Island = the negate-push kernel verbatim: `fld dword [src_cell]; fchs; fstp dword [out]`.
    // The five binary sites are byte-identical modulo the source operand (a [reg+disp]/getter/0.0):
    //   0x50d49f d9 42 1c    fld [edx+0x1c]   ; 0x50d4a3 d9 e0 fchs ; 0x50d4a7 d9 1c 24 fstp [esp]
    //   0x50d4b4 d9 86 8c01  fld [esi+0x18c]  ; 0x50d4bb fchs ; 0x50d4bf fstp [esp]
    //   0x50d4d0 (getter→st0); 0x50d4d3 fchs ; 0x50d4d8 fstp [esp]
    //   0x50d4e5 d9 45 fc    fld [ebp-4]      ; 0x50d4e9 fchs ; 0x50d4ed fstp [esp]
    //   0x50d528 d9 81 0801  fld [ecx+0x108]  ; 0x50d52f fchs ; 0x50d531 fstp [esp]   (0x50d520)
    // The `fchs;fstp` bytes (d9 e0 / d9 1c..) are identical at every site; only the leading `fld`'s
    // modrm/disp differs. The island uses an absolute `fld [src_cell]` (modrm 0x05) + the verbatim
    // `fchs` (d9 e0) + an absolute `fstp [out]` (modrm 0x1d) — same arithmetic op (`fchs`) as all 5.
    let isl = emu.alloc(0x20);
    let mut c = vec![0xd9u8, 0x05]; // fld dword [src_cell]
    c.extend_from_slice(&(src_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd9, 0xe0]); // fchs            (verbatim from every site)
    c.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    c.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + c.len() as u64;
    emu.write(isl, &c);

    // Byte-verify the `fchs` opcode matches the binary at each site (the load-bearing op).
    for site in [0x50d4a3u64, 0x50d4bb, 0x50d4d3, 0x50d4e9, 0x50d52f] {
        assert_eq!(
            emu.read(site, 2),
            vec![0xd9, 0xe0],
            "fchs bytes @ {site:#x}"
        );
    }

    let run = |emu: &mut Emu, src: f32| -> f32 {
        emu.write_u32(src_cell, src.to_bits());
        emu.run(isl, end);
        emu.read_f32(out)
    };

    let mut total = 0u32;
    for src in [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        f32::from_bits(PITCH_HI_BITS),
        123.456,
        -789.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        assert_eq!(
            run(&mut emu, src).to_bits(),
            neg_push(src).to_bits(),
            "neg src={src}"
        );
        total += 1;
    }
    let mut s = 0x50d4_4000u32;
    for _ in 0..3000 {
        let src = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, src).to_bits(),
            neg_push(src).to_bits(),
            "neg fuzz"
        );
        total += 1;
    }
    eprintln!("glue_50d440_neg_push: {total} bit-exact (fchs island; 5 byte-verified sites)");
}

// ===========================================================================================
// 0x50d6d0 — flag-gated source + fadd [ecx+0x40] + clamp to [0.0, 3.1241393]. fadd×1.
// Whole-function __thiscall, no stack args, f32 returned in st0.
// ===========================================================================================
#[test]
fn glue_50d6d0_pitch_clamp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let hi = f32::from_bits(PITCH_HI_BITS);

    // The fn decs [ecx+0xa8] and only does the fadd+clamp when it hits zero; preset [ecx+0xa8]=1 so
    // the dec→0 path (the owned slice) fires. flag_bit3 = bit 3 (0x8) of [ecx+0x90].
    let run = |emu: &mut Emu, source: f32, accum: f32, flag: bool| -> f32 {
        emu.write_u32(f.this + 0xa8, 1);
        emu.write_u32(f.this + 0x90, if flag { 0x8 } else { 0 });
        emu.write_u32(f.this + 0x10c, source.to_bits());
        emu.write_u32(f.this + 0x40, accum.to_bits());
        emu.call_f32(0x50d6d0, f.this as u32, 0, &[])
    };

    let mut total = 0u32;
    let mut lo_n = 0u32;
    let mut hi_n = 0u32;
    let mut mid_n = 0u32;
    // deterministic boundaries at exactly 0.0 and exactly hi (3.1241393).
    for (source, accum, flag, _l) in [
        (0.0f32, 0.0f32, false, "==lo"),
        (hi, 0.0, false, "==hi"),
        (hi, 1.0e-6, false, ">hi"),
        (-1.0, 0.0, false, "<lo"),
        (1.5, 0.0, false, "mid"),
        (5.0, 0.0, true, "flag-zeros-source"), // source ignored ⇒ sum=accum=0 ⇒ lo
        (0.0, hi, true, "flag-accum-hi"),
        (f32::NAN, 0.0, false, "nan-source"), // sum=NaN ⇒ even-parity routing ⇒ hi
        (0.0, f32::NAN, false, "nan-accum"),
    ] {
        let g = run(&mut emu, source, accum, flag);
        assert_eq!(
            g.to_bits(),
            pitch_clamp_accum(source, accum, flag).to_bits(),
            "clamp src={source} acc={accum} flag={flag}"
        );
        total += 1;
    }
    let mut s = 0x50d6_d000u32;
    for _ in 0..3000 {
        let flag = (next(&mut s) >> 8) & 1 == 1;
        let source = rng(&mut s, -1.0, 4.0);
        let accum = rng(&mut s, -1.0, 4.0);
        let g = run(&mut emu, source, accum, flag);
        assert_eq!(
            g.to_bits(),
            pitch_clamp_accum(source, accum, flag).to_bits(),
            "clamp fuzz src={source} acc={accum} flag={flag}"
        );
        let sum = if flag { 0.0 } else { f64::from(source) } + f64::from(accum);
        if sum < 0.0 {
            lo_n += 1;
        } else if sum >= f64::from(hi) {
            hi_n += 1;
        } else {
            mid_n += 1;
        }
        total += 1;
    }
    assert!(
        lo_n > 0 && hi_n > 0 && mid_n > 0,
        "pitch clamp: need all three branches (lo={lo_n} hi={hi_n} mid={mid_n})"
    );
    eprintln!(
        "glue_50d6d0_pitch_clamp: {total} bit-exact (fadd+clamp lo={lo_n} hi={hi_n} mid={mid_n})"
    );
}

// ===========================================================================================
// 0x50d0f0 — percent·0.01 scale: fild [ebp-4]; fmul [0x8029d0]=0.01; fstp [ebp-4]. fmul×1.
// Slice 0x50d1f3..0x50d1ff (the input [ebp-4] is the pre-clamped int min(byte,100)).
// Plus the 3-axis deadband fcomp (no FP-arith) via a copied-byte island.
// ===========================================================================================
#[test]
fn glue_50d0f0_percent_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // The fild reads [ebp-4] (the byte already clamped to <=100 and zero-extended by the binary's
    // `movzx; mov [ebp-4],eax`). We drive the clamped int directly and diff the fild;fmul;fstp.
    let run = |emu: &mut Emu, n: i32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x4, n as u32);
        emu.run(0x50d1f3, 0x50d1ff);
        emu.read_f32(f.ebp - 0x4)
    };

    let mut total = 0u32;
    for pct in [0u8, 1, 50, 99, 100, 101, 200, 255] {
        let n = i32::from(pct.min(100));
        assert_eq!(
            run(&mut emu, n).to_bits(),
            percent_scale(pct).to_bits(),
            "pct={pct} (n={n})"
        );
        total += 1;
    }
    // fuzz the clamped int domain 0..=100 (the only values the binary can present).
    let mut s = 0x50d0_f000u32;
    for _ in 0..2000 {
        let n = ((next(&mut s) >> 8) % 101) as u8; // 0..=100, already in the clamped domain
        assert_eq!(
            run(&mut emu, i32::from(n)).to_bits(),
            percent_scale(n).to_bits(),
            "pct fuzz n={n}"
        );
        total += 1;
    }

    // 3-axis deadband: copied-byte island of 0x50d147..0x50d17f (the three fld;fcomp;test;jcc plus
    // the ecx=1/ecx=0 join), byte-verified. ecx pointers: the axes [ecx+0]/[ecx+4]/[ecx+8];
    // thresholds [edi+0x174]/0x178/0x17c.
    let axes = emu.alloc(0x10);
    let thr = f.this;
    let isl = emu.alloc(0x60);
    // Copy 0x50d147..0x50d17f verbatim: the internal jp/jnp are relative, so they self-adjust
    // inside the island; the trailing `mov ecx,1 / xor ecx,ecx` join is included.
    let src = emu.read(0x50d147, (0x50d17f - 0x50d147) as usize);
    emu.write(isl, &src);
    // Byte-verify the three fcomp + test-ah opcodes are present at their island offsets.
    assert_eq!(emu.read(isl + 2, 1), vec![0xd8], "fcomp[0] @ island"); // 0x50d149 d8 9f ..
    assert_eq!(emu.read(0x50d149, 1), vec![0xd8], "fcomp[0] @ 0x50d149");
    let isl_end = isl + (0x50d17f - 0x50d147) as u64;
    let run_db = |emu: &mut Emu, v: [f32; 3], t: [f32; 3]| -> u32 {
        emu.write_u32(axes, v[0].to_bits());
        emu.write_u32(axes + 4, v[1].to_bits());
        emu.write_u32(axes + 8, v[2].to_bits());
        emu.write_u32(thr + 0x174, t[0].to_bits());
        emu.write_u32(thr + 0x178, t[1].to_bits());
        emu.write_u32(thr + 0x17c, t[2].to_bits());
        emu.set_reg(RegisterX86::ECX, axes);
        emu.set_reg(RegisterX86::EDI, thr);
        emu.run(isl, isl_end);
        emu.get_reg(RegisterX86::ECX) as u32
    };
    let mut diff_n = 0u32;
    let mut same_n = 0u32;
    for (v, t, _l) in [
        ([1.0f32, 2.0, 3.0], [1.0f32, 2.0, 3.0], "all-eq"),
        ([1.0, 2.0, 3.0], [9.0, 2.0, 3.0], "axis0-diff"),
        ([1.0, 2.0, 3.0], [1.0, 2.0, 9.0], "axis2-diff"),
        ([f32::NAN, 2.0, 3.0], [1.0, 2.0, 3.0], "nan"),
    ] {
        let ecx = run_db(&mut emu, v, t);
        assert_eq!(ecx == 1, deadband_eq_any3(v, t), "deadband {v:?} vs {t:?}");
        if ecx == 1 {
            diff_n += 1;
        } else {
            same_n += 1;
        }
    }
    let mut s = 0x50d1_4700u32;
    for _ in 0..1500 {
        let t = [
            rng(&mut s, -5.0, 5.0),
            rng(&mut s, -5.0, 5.0),
            rng(&mut s, -5.0, 5.0),
        ];
        let v = if (next(&mut s) >> 8) & 1 == 0 {
            t // exactly equal -> ecx=0
        } else {
            [t[0], t[1] + rng(&mut s, -1.0, 1.0), t[2]]
        };
        let ecx = run_db(&mut emu, v, t);
        assert_eq!(ecx == 1, deadband_eq_any3(v, t), "deadband fuzz");
        if ecx == 1 {
            diff_n += 1;
        } else {
            same_n += 1;
        }
    }
    assert!(diff_n > 0 && same_n > 0, "deadband: need both outcomes");
    eprintln!(
        "glue_50d0f0_percent_scale: {total} bit-exact (fmul[0.01]); deadband differs={diff_n} same={same_n}"
    );
}

// ===========================================================================================
// 0x50dbc0 — step fmul + |·|<eps fabs + clamp tail (fmul×2). All 4 FP-arith ops diffed.
// ===========================================================================================
#[test]
fn glue_50dbc0_step_clamp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    // (1) step fmul: 0x50dcb9..0x50dccb -> fld [eax+0x24]; fmul [esi+0xa0]; fstp [ebp-4].
    let g = emu.alloc(0x40);
    let run_mul = |emu: &mut Emu, src: f32, rate: f32| -> f32 {
        emu.set_reg(RegisterX86::EAX, g);
        emu.set_reg(RegisterX86::ECX, g); // `mov edx,[ecx+0x24]` only stages bits for a later test
        emu.set_reg(RegisterX86::ESI, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(g + 0x24, src.to_bits());
        emu.write_u32(f.this + 0xa0, rate.to_bits());
        emu.run(0x50dcb9, 0x50dcce); // through fmul + fstp [ebp-4]
        emu.read_f32(f.ebp - 0x4)
    };
    let mut total = 0u32;
    for (src, rate) in [
        (1.0f32, 2.0f32),
        (3.5, 0.0),
        (-2.0, 4.0),
        (0.1, 0.1),
        (1e20, 1e-20),
    ] {
        assert_eq!(
            run_mul(&mut emu, src, rate).to_bits(),
            step_mul(src, rate).to_bits(),
            "step src={src} rate={rate}"
        );
        total += 1;
    }
    let mut s = 0x50db_c000u32;
    for _ in 0..2000 {
        let src = rng(&mut s, -1000.0, 1000.0);
        let rate = rng(&mut s, -10.0, 10.0);
        assert_eq!(
            run_mul(&mut emu, src, rate).to_bits(),
            step_mul(src, rate).to_bits(),
            "step fuzz"
        );
        total += 1;
    }

    // (2) |·|<eps probe: 0x50dcf8..0x50dd0a -> fld [ebp+8]; fabs; fcomp [0x8029d4]; fnstsw.
    let run_abs = |emu: &mut Emu, v: f32| -> u8 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, v.to_bits());
        emu.run(0x50dcf8, 0x50dd05); // through fcomp + fnstsw (test-ah not executed)
        ah_of(emu)
    };
    let mut af = 0u32;
    let mut as_ = 0u32;
    for (v, _l) in [
        (eps, "eq"),
        (eps * 0.5, "below"),
        (eps * 2.0, "above"),
        (-eps * 0.5, "neg-below"),
        (f32::NAN, "nan"),
    ] {
        let ah = run_abs(&mut emu, v);
        // test ah,5; jnp -> taken (settle) iff EVEN parity; falls to clamp iff odd (|v|<eps). 1⇒odd.
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin, abs_lt_eps(v), "abs_lt_eps v={v}");
        if bin {
            af += 1;
        } else {
            as_ += 1;
        }
    }
    let mut s = 0x50dc_f800u32;
    for _ in 0..1500 {
        let v = rng(&mut s, -1.0e-6, 1.0e-6);
        let ah = run_abs(&mut emu, v);
        assert_eq!(
            (ah & 0x5).count_ones() % 2 == 1,
            abs_lt_eps(v),
            "abs fuzz v={v}"
        );
        if (ah & 0x5).count_ones() % 2 == 1 {
            af += 1;
        } else {
            as_ += 1;
        }
    }
    assert!(af > 0 && as_ > 0, "abs_lt_eps: need both");

    // (3) clamp tail: 0x50dd2e..0x50dd64 -> the two bound-product fmuls + the live-st0/f32-reload
    // clamp. The two original exits each `fstp [esi+0x1ac]; ret 0xc`, so a single `run(begin,until)`
    // can't catch both. Rebuild the tail as an ISLAND with each op byte-verified
    // against the disasm, the three selection paths converging on one `fstp [out]` capture.
    //
    // Original tail (each line a verbatim op the island reproduces):
    //   0x50dd2e fld [eax+0x24]; 0x50dd37 fmul [ebp+8]; 0x50dd3a fld [ecx+0x24]; 0x50dd3d fmul [ebp+8];
    //   0x50dd40 fstp [ebp+0x10]; 0x50dd43 fld [esi+0x1ac]; 0x50dd49 fst [ebp+8]; 0x50dd4c fcomp st(1);
    //   0x50dd4e fnstsw; 0x50dd50 test ah,5; 0x50dd53 jnp ->0x50dd78 (hi: st0 already = hi);
    //   0x50dd55 fstp st(0); 0x50dd57 fld [ebp+8]; 0x50dd5a fcomp [ebp+0x10]; 0x50dd5d fnstsw;
    //   0x50dd5f test ah,5; 0x50dd62 jp ->0x50dd75 (lo: fld [ebp+0x10]); 0x50dd64 fld [ebp+8] (pass).
    // eax=[0xbe1050]=ghi (b_hi @ +0x24); ecx=[0xbe1054]=glo (b_lo @ +0x24); [ebp+8]=m;
    // [esi+0x1ac]=field. The island uses absolute `fld` for the three memory sources; every
    // ARITHMETIC/compare op (fmul/fst-no-pop/fcomp) is byte-identical and asserted below.
    let ghi = emu.alloc(0x40); // [0xbe1050] target; +0x24 = b_a (the `a` bound's table field)
    let glo = emu.alloc(0x40); // [0xbe1054] target; +0x24 = b_b (the `b` bound's table field)
    let out = emu.alloc(4);
    let field_cell = emu.alloc(4);
    let ebp = f.ebp; // [ebp+8]=m (then the fst-no-pop field), [ebp+0x10]=b store
    let isl = emu.alloc(0x80);

    // The island is the ORIGINAL bytes 0x50dd2e..0x50dd7f copied verbatim, with ONLY the
    // non-arithmetic operands rewritten: the three `fld` memory sources (the two table globals via
    // ECX/EAX → absolute scratch; the field via [esi+0x1ac] → absolute scratch) and the two
    // `fstp [esi+0x1ac]` stores → `fstp [out]`. The two `fmul`s, the `fst`-no-pop, the two `fcomp`s,
    // and BOTH jccs (`jnp 0x50dd78`/`jp 0x50dd75`) are byte-identical and keep the binary's exact
    // landings (both jccs branch to the SAME `fld [ebp+0x10]=b` store; only the double-fall-through
    // loads `[ebp+8]`). The internal jumps are RIP-relative — preserved verbatim, they self-adjust.
    let base = 0x50dd29u64; // include the two global-loads `mov eax,[0xbe1050]; mov ecx,[0xbe1054]`
                            // Copy 0x50dd29..0x50dd83 verbatim, then a trailing nop as the converged halt EIP.
    let copy_end = 0x50dd83u64;
    let mut c = emu.read(base, (copy_end - base) as usize);
    c.push(0x90); // END halt byte
    let at = |va: u64| (va - base) as usize;
    // Leave 0x50dd2e `fld [eax+0x24]` / 0x50dd3a `fld [ecx+0x24]` verbatim; EAX/ECX come from the
    // two in-island `mov`s reading [0xbe1050]/[0xbe1054] (set to ghi/glo at run time).
    // field load 0x50dd43 `fld [esi+0x1ac]` (d9 86 ac010000, 6B) → `fld [field_cell]` (d9 05 ..,6B).
    c[at(0x50dd43)] = 0xd9;
    c[at(0x50dd43) + 1] = 0x05;
    c[at(0x50dd43) + 2..at(0x50dd43) + 6].copy_from_slice(&(field_cell as u32).to_le_bytes());
    // store 0x50dd68 `fstp [esi+0x1ac]` → `fstp [out]` (6B).
    c[at(0x50dd68)] = 0xd9;
    c[at(0x50dd68) + 1] = 0x1d;
    c[at(0x50dd68) + 2..at(0x50dd68) + 6].copy_from_slice(&(out as u32).to_le_bytes());
    // store 0x50dd79 `fstp [esi+0x1ac]` → `fstp [out]` (6B).
    c[at(0x50dd79)] = 0xd9;
    c[at(0x50dd79) + 1] = 0x1d;
    c[at(0x50dd79) + 2..at(0x50dd79) + 6].copy_from_slice(&(out as u32).to_le_bytes());
    // Both store paths are followed by the fn epilogue `pop esi; mov esp,ebp; pop ebp; ret 0xc`
    // which restores ESP/EBP and returns — meaningless in the island. Replace each epilogue with a
    // `jmp END` so both converge on a single halt. The `pop edi` at 0x50dd67/0x50dd78 (BEFORE each
    // store) is kept verbatim — it just consumes 4 stack bytes (harmless; ESP has headroom).
    let end_off = (copy_end - base) as usize; // the trailing nop
                                              // hi/pass exit: epilogue at 0x50dd6e (7 bytes through 0x50dd74, before the 0x50dd75 lo-load).
    let j1 = at(0x50dd6e);
    c[j1] = 0xeb;
    c[j1 + 1] = (end_off as i64 - (j1 as i64 + 2)) as u8; // jmp END
    for b in &mut c[j1 + 2..at(0x50dd75)] {
        *b = 0x90; // pad the rest of the old epilogue up to the lo-load (untouched)
    }
    // lo exit: epilogue at 0x50dd7f (5 bytes through 0x50dd82, before END at 0x50dd83).
    let j2 = at(0x50dd7f);
    c[j2] = 0xeb;
    c[j2 + 1] = (end_off as i64 - (j2 as i64 + 2)) as u8; // jmp END
    for b in &mut c[j2 + 2..end_off] {
        *b = 0x90;
    }
    let isl_end = isl + end_off as u64;
    emu.write(isl, &c);

    // Byte-verify the load-bearing ARITHMETIC/compare ops are UNCHANGED from the binary (the island
    // copied them verbatim; assert the source bytes match what we rely on).
    assert_eq!(
        emu.read(0x50dd37, 3),
        vec![0xd8, 0x4d, 0x08],
        "fmul[ebp+8]@dd37"
    );
    assert_eq!(
        emu.read(0x50dd3d, 3),
        vec![0xd8, 0x4d, 0x08],
        "fmul[ebp+8]@dd3d"
    );
    assert_eq!(
        emu.read(0x50dd49, 3),
        vec![0xd9, 0x55, 0x08],
        "fst[ebp+8]@dd49 (no-pop)"
    );
    assert_eq!(emu.read(0x50dd4c, 2), vec![0xd8, 0xd9], "fcomp st1 @dd4c");
    assert_eq!(emu.read(0x50dd53, 2), vec![0x7b, 0x23], "jnp @dd53");
    assert_eq!(
        emu.read(0x50dd5a, 3),
        vec![0xd8, 0x5d, 0x10],
        "fcomp[ebp+0x10]@dd5a"
    );
    assert_eq!(emu.read(0x50dd62, 2), vec![0x7a, 0x11], "jp @dd62");

    // ghi=[0xbe1050] holds b_lo (the lo bound's table field); glo=[0xbe1054] holds b_hi.
    let run_clamp = |emu: &mut Emu, field: f32, b_lo: f32, b_hi: f32, m: f32| -> f32 {
        emu.write_u32(0xbe1050, ghi as u32); // 0x50dd29 mov eax,[0xbe1050] (lo bound source)
        emu.write_u32(0xbe1054, glo as u32); // 0x50dd31 mov ecx,[0xbe1054] (hi bound source)
        emu.write_u32(ghi + 0x24, b_lo.to_bits());
        emu.write_u32(glo + 0x24, b_hi.to_bits());
        emu.write_u32(field_cell, field.to_bits());
        emu.write_u32(ebp + 0x8, m.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(isl, isl_end);
        emu.read_f32(out)
    };

    // [lo, hi) clamp: field<lo(live) → lo; field_f32>=hi → hi; else field_f32. lo=b_lo·m, hi=b_hi·m.
    // Probe boundaries at field exactly == lo and == hi (the live-f64 vs f32-reload asymmetry).
    let mut lo_n = 0u32; // field < lo → lo
    let mut mid_n = 0u32; // lo <= field < hi → field
    let mut hi_n = 0u32; // field >= hi → hi
                         // b_lo=0.5,b_hi=1.0,m=2 ⇒ lo=1.0, hi=2.0.
    for (field, b_lo, b_hi, m, _l) in [
        (0.5f32, 0.5f32, 1.0f32, 2.0f32, "field<lo"), // lo=1 hi=2, field 0.5<lo → lo=1
        (1.0, 0.5, 1.0, 2.0, "field==lo"),            // field==lo (not <lo) & <hi → field=1
        (1.5, 0.5, 1.0, 2.0, "mid"),                  // → field=1.5
        (2.0, 0.5, 1.0, 2.0, "field==hi"),            // field==hi (>=hi) → hi=2
        (3.0, 0.5, 1.0, 2.0, "field>hi"),             // → hi=2
        (-5.0, 0.5, 1.0, 2.0, "field far<lo"),        // → lo=1
        (f32::NAN, 0.5, 1.0, 2.0, "nan-field"),       // lo cmp not-< ; hi cmp even ⇒ hi=2
    ] {
        let g = run_clamp(&mut emu, field, b_lo, b_hi, m);
        assert_eq!(
            g.to_bits(),
            clamp_tail(field, b_lo, b_hi, m).to_bits(),
            "clamp field={field} b_lo={b_lo} b_hi={b_hi} m={m}"
        );
        total += 1;
    }
    let mut s = 0x50dd_2e00u32;
    for _ in 0..3000 {
        let b_lo = rng(&mut s, -3.0, 3.0);
        let b_hi = rng(&mut s, -3.0, 3.0);
        let m = rng(&mut s, 0.1, 3.0);
        let field = rng(&mut s, -12.0, 12.0);
        let g = run_clamp(&mut emu, field, b_lo, b_hi, m);
        assert_eq!(
            g.to_bits(),
            clamp_tail(field, b_lo, b_hi, m).to_bits(),
            "clamp fuzz field={field} b_lo={b_lo} b_hi={b_hi} m={m}"
        );
        let lo = f64::from(b_lo) * f64::from(m);
        let hi = (f64::from(b_hi) * f64::from(m)) as f32;
        if f64::from(field) < lo {
            lo_n += 1;
        } else if f64::from(field) < f64::from(hi) {
            mid_n += 1;
        } else {
            hi_n += 1;
        }
        total += 1;
    }
    assert!(
        lo_n > 0 && mid_n > 0 && hi_n > 0,
        "clamp tail: need all three branches (lo={lo_n} mid={mid_n} hi={hi_n})"
    );
    eprintln!(
        "glue_50dbc0_step_clamp: {total} bit-exact (step fmul, |·|<eps fabs, clamp fmul×2 lo={lo_n} mid={mid_n} hi={hi_n})"
    );
}
