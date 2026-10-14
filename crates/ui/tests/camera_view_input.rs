//! Differential tests of the arithmetic of the camera view and input functions between
//! `CCamera::UpdateTransitions` (`0x50f160`) and `CCamera::ArmAutoReturn` (`0x510960`): view
//! store and reset, zoom, mouse turn, facing commit, and the bobbing and pivot-bias gates, each
//! run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice of the camera-input/transition glue TU `[0x50f160, 0x510960]` is diffed as
//! an **instruction slice** (`Emu::run`) — registers/memory/stack synthesized, the binary's own
//! bytes executed, the stored `fstp/fst [m32]` value or the `fnstsw` AH read back. Every in-body
//! FP-arith opcode (the arithmetic opcode set: fmul/fadd/fsub/fsubr/fdiv/fdivr/fabs/fchs/
//! fmulp/…) lands in a diffed slice. Fuzz reads the LCG's HIGH bits.
//! Every owned-slice test carries a deterministic `==threshold` boundary probe. The eps
//! change-detect branches are reconstructed from the EMITTED jcc, not the mask: `test ah,5` (C0|C2)
//! read by `jp`/`jnp` is a PARITY test — `jp` taken ⇔ odd parity ⇔ (ordered `<`), `jnp` taken ⇔
//! even ⇔ (`≥` or unordered).
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::camera_view_input::{
    abs_gt, abs_lt_dt, add2_f32, add_then_pitch_clamp, changed_lt_eps, deadband_eq_any,
    deg2rad_gated, deg2rad_settled, dist_le_sixth, mouselook_rate_pitch, mouselook_rate_yaw,
    mul2_f32, near_eq_eps, neg_mul2_f32, pitch_integrate, pitch_le_zero, rad2deg_gated, rate_frac,
    rate_frac_f32, rate_frac_interpolates, sub_field, sub_one, sum3_abs_lt_dt, throttle_frac,
    unit_convert_ftol, EPS_BITS, PITCH_CLAMP_BITS, SIXTH_BITS,
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
const DT_F32: f32 = f32::from_bits(0x3a83_126f);

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x300);
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
// 0x50f160 — rate fraction (8 channels: fild;fmul[dt];fdiv[rate];fst[ebp-4];fcomp 1.0;fnstsw)
// fmul×8 + fdiv×8 — diffed at EVERY channel address (byte-identical modulo the fdiv disp32).
// ===========================================================================================
const FRAC_BLOCKS: [(u64, u64, u64); 8] = [
    (0x50f18d, 0x50f1a7, 0x23c),
    (0x50f249, 0x50f263, 0x224),
    (0x50f2f5, 0x50f30f, 0x194),
    (0x50f3a8, 0x50f3c2, 0x1c4),
    (0x50f455, 0x50f46f, 0x1ac),
    (0x50f501, 0x50f51b, 0x1f4),
    (0x50f5b4, 0x50f5ce, 0x1dc),
    (0x50f66b, 0x50f685, 0x20c),
];

#[test]
fn glue_50f160_rate_frac() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run_frac =
        |emu: &mut Emu, start: u64, end: u64, off: u64, delta: i32, rate: f32| -> (u8, f32) {
            set_frame(emu, &f);
            emu.write_u32(f.ebp - 0x8, delta as u32); // low dword = Δ
            emu.write_u32(f.ebp - 0x4, 0); // high dword = 0
            emu.write_u32(f.this + off, rate.to_bits());
            emu.run(start, end);
            (ah_of(emu), emu.read_f32(f.ebp - 0x4))
        };

    let mut total = 0u32;
    let mut interp = 0u32;
    let mut snap = 0u32;
    let dt = f64::from(DT_F32);

    for &(start, end, off) in &FRAC_BLOCKS {
        // deterministic boundary: frac == 1.0 exactly. Δ=1_000_000, rate = (Δ·dt) so frac == 1.0.
        let rate_eq = (1_000_000.0_f64 * dt) as f32;
        for (delta, rate) in [
            (1_000_000i32, rate_eq),                          // ~boundary
            (1_000_000, (f64::from(rate_eq) * 1.001) as f32), // frac < 1.0 (interp)
            (1_000_000, (f64::from(rate_eq) * 0.999) as f32), // frac > 1.0 (snap)
        ] {
            let (ah, sf) = run_frac(&mut emu, start, end, off, delta, rate);
            let mine = rate_frac(delta, 0, rate);
            assert_eq!(
                rate_frac_f32(mine).to_bits(),
                sf.to_bits(),
                "frac f32 @ {start:#x}"
            );
            let bin = ah & 0x1 != 0; // test ah,1; jne taken iff C0 (live f64 frac < 1.0)
            assert_eq!(
                bin,
                rate_frac_interpolates(mine),
                "frac gate @ {start:#x} frac={mine}"
            );
            if bin {
                interp += 1;
            } else {
                snap += 1;
            }
            total += 1;
        }
    }

    let mut s = 0x50f1_6000u32;
    for &(start, end, off) in &FRAC_BLOCKS {
        for _ in 0..400 {
            let delta = (next(&mut s) >> 8) as i32 % 4_000_000 - 2_000_000;
            let rate = rng(&mut s, 0.5, 5000.0);
            let (ah, sf) = run_frac(&mut emu, start, end, off, delta, rate);
            let mine = rate_frac(delta, 0, rate);
            assert_eq!(
                rate_frac_f32(mine).to_bits(),
                sf.to_bits(),
                "frac f32 @ {start:#x}"
            );
            assert_eq!(
                ah & 0x1 != 0,
                rate_frac_interpolates(mine),
                "frac gate @ {start:#x}"
            );
            if ah & 0x1 != 0 {
                interp += 1;
            } else {
                snap += 1;
            }
            total += 1;
        }
    }
    eprintln!("glue_50f160_rate_frac: {total} bit-exact (interp={interp} snap={snap})");
    assert!(interp > 0 && snap > 0, "need both clamp outcomes");
}

// 0x50f160 change-detect — 7 channels: fld[tgt];fsub[field];fabs;fcomp[eps];fnstsw. fsub×7+fabs×7,
// each block self-contained, diffed at EVERY channel address (disp32-only variation).
const CHG_BLOCKS: [(u64, u64, u64); 7] = [
    // (fld_start, tgt_off, field_off); end = start + 0x16.
    (0x50f20b, 0x228, 0x10c),
    (0x50f2b8, 0x198, 0xec),
    (0x50f36a, 0x1c8, 0xfc),
    (0x50f417, 0x1b0, 0x108),
    (0x50f4c4, 0x1f8, 0x104),
    (0x50f576, 0x1e0, 0xf4),
    (0x50f632, 0x210, 0xf0),
];

#[test]
fn glue_50f160_change_detect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    let run_chg = |emu: &mut Emu, start: u64, ot: u64, of: u64, tgt: f32, field: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.this + ot, tgt.to_bits());
        emu.write_u32(f.this + of, field.to_bits());
        emu.run(start, start + 0x16);
        ah_of(emu)
    };

    let mut total = 0u32;
    let mut chg = 0u32;
    let mut settled = 0u32;
    for &(start, ot, of) in &CHG_BLOCKS {
        for (tgt, field, _l) in [
            (eps, 0.0f32, "eq"),
            (eps * 0.5, 0.0, "below"),
            (eps * 2.0, 0.0, "above"),
            (f32::NAN, 0.0, "nan"),
        ] {
            let ah = run_chg(&mut emu, start, ot, of, tgt, field);
            // test ah,5; jnp -> taken (skip/settled) iff EVEN parity; falls through (changes) iff odd.
            let bin = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(
                bin,
                changed_lt_eps(tgt, field),
                "chg @ {start:#x} tgt={tgt} field={field}"
            );
            if bin {
                chg += 1;
            } else {
                settled += 1;
            }
            total += 1;
        }
    }
    let mut s = 0x50f2_0b00u32;
    for &(start, ot, of) in &CHG_BLOCKS {
        for _ in 0..500 {
            let field = rng(&mut s, -10.0, 10.0);
            let tgt = field + rng(&mut s, -1.0e-6, 1.0e-6);
            let ah = run_chg(&mut emu, start, ot, of, tgt, field);
            let bin = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(bin, changed_lt_eps(tgt, field), "chg @ {start:#x}");
            if bin {
                chg += 1;
            } else {
                settled += 1;
            }
            total += 1;
        }
    }
    eprintln!("glue_50f160_change_detect: {total} bit-exact (changed={chg} settled={settled})");
    assert!(chg > 0 && settled > 0, "need both change-detect outcomes");
}

// ===========================================================================================
// 0x50f710 — owned adds: [+0x100]+[ebx] (0x50f7de), fadd st,st(1) (0x50f7f2), [+0x108]+[edi]
// (0x50f816). Plus the 3-axis deadband (fcomp only — no FP-arith). fadd×3.
// ===========================================================================================
#[test]
fn glue_50f710_adds() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let out = emu.alloc(8);

    // The 3 `fadd`s are tiny mid-block sequences (a 3-instruction `Emu::run` slice hits a stale-TB
    // READ_UNMAPPED — mid-block hazard), so each is diffed via a COPIED-BYTE ISLAND: the EXACT
    // source bytes assembled in scratch with our own register pointers, byte-verified against the
    // disasm below. (copied-byte islands are credited only when the bytes match source.)
    //
    // add1+add2 island = 0x50f7d8..0x50f7f6 verbatim (`fst[esi+0x184]; fld[esi+0x100]; fadd[ebx];
    // fst[ebx]; fadd st,st(1); fstp[ebx]`). Before it we `fld [w_cell]` to put the live wrapped
    // value `W` on st0 — exactly what 0x50f7d8's `fst` expects (W comes from 0x50f7d8's predecessor).
    // So the island computes: st0=W; store W→[+0x184]; st0=[+0x100]; st0+=[ebx]; store→[ebx] (add1);
    // st0 += st(1)=W; store→[ebx] (add2). result = ([+0x100]+acc) + W.
    let w_cell = emu.alloc(4);
    let isl12 = emu.alloc(0x40);
    let mut c = vec![0xd9u8, 0x05]; // fld dword [w_cell]
    c.extend_from_slice(&(w_cell as u32).to_le_bytes());
    c.extend_from_slice(&[0xd9, 0x96, 0x84, 0x01, 0x00, 0x00]); // fst  [esi+0x184]   (0x50f7d8)
    c.extend_from_slice(&[0xd9, 0x86, 0x00, 0x01, 0x00, 0x00]); // fld  [esi+0x100]   (0x50f7de)
    c.extend_from_slice(&[0xd8, 0x03]); // fadd [ebx]            (0x50f7e4)
    c.extend_from_slice(&[0xd9, 0x13]); // fst  [ebx]            (0x50f7e6)  add1 store
    c.extend_from_slice(&[0xd8, 0xc1]); // fadd st,st(1)         (0x50f7f2)  add2
    c.extend_from_slice(&[0xd9, 0x1b]); // fstp [ebx]            (0x50f7f4)  add2 store
    let isl12_end = isl12 + c.len() as u64;
    emu.write(isl12, &c);
    // add3 island = 0x50f810..0x50f81a verbatim (`fld[esi+0x108]; fadd[edi]; fstp[edi]`).
    let isl3 = emu.alloc(0x20);
    let mut c3 = vec![0xd9u8, 0x86, 0x08, 0x01, 0x00, 0x00]; // fld  [esi+0x108]   (0x50f810)
    c3.extend_from_slice(&[0xd8, 0x07]); // fadd [edi]            (0x50f816)
    c3.extend_from_slice(&[0xd9, 0x1f]); // fstp [edi]            (0x50f818)
    let isl3_end = isl3 + c3.len() as u64;
    emu.write(isl3, &c3);

    // add1: read [ebx] right after the add1 store (the island leaves the add2 result there, so we
    // diff add1 separately by halting the island after the `fst [ebx]` add1 store).
    let isl1_end = isl12 + 6 + 6 + 6 + 2 + 2; // up to & including `fst [ebx]` (0x50f7e6 equivalent)
    let run_add1 = |emu: &mut Emu, base: f32, acc: f32, w: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(w_cell, w.to_bits());
        emu.write_u32(f.this + 0x100, base.to_bits());
        emu.write_u32(out, acc.to_bits());
        emu.set_reg(RegisterX86::EBX, out);
        emu.run(isl12, isl1_end);
        emu.read_f32(out)
    };
    let run_add2 = |emu: &mut Emu, base: f32, acc: f32, w: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(w_cell, w.to_bits());
        emu.write_u32(f.this + 0x100, base.to_bits());
        emu.write_u32(out, acc.to_bits());
        emu.set_reg(RegisterX86::EBX, out);
        emu.run(isl12, isl12_end);
        emu.read_f32(out)
    };
    let run_add3 = |emu: &mut Emu, base: f32, acc: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0x108, base.to_bits());
        emu.write_u32(out, acc.to_bits());
        emu.set_reg(RegisterX86::EDI, out);
        emu.run(isl3, isl3_end);
        emu.read_f32(out)
    };

    let mut total = 0u32;
    // add2: `fadd st,st(1)` adds the LIVE PC_53 st0 (`base+acc`, un-narrowed — the preceding `fst
    // [ebx]` was a no-POP store, live-st0 trap) to st1=W. So add2 = (f64 base+acc+W) → f32,
    // NOT (s1_f32 + W).
    for (b, a, w) in [
        (1.0f32, 2.0f32, 0.5f32),
        (1.0, 16_777_216.0, -3.0),
        (-3.5, 3.5, 1.0),
        (0.0, 0.0, 0.0),
    ] {
        let s1 = (f64::from(b) + f64::from(a)) as f32; // add1 store (fst → f32)
        let s2 = (f64::from(b) + f64::from(a) + f64::from(w)) as f32; // add2: live-st0 f64 sum + W
        assert_eq!(
            run_add1(&mut emu, b, a, w).to_bits(),
            s1.to_bits(),
            "add1 b={b} a={a}"
        );
        assert_eq!(
            run_add2(&mut emu, b, a, w).to_bits(),
            s2.to_bits(),
            "add2 b={b} a={a} w={w}"
        );
        assert_eq!(
            run_add3(&mut emu, b, a).to_bits(),
            s1.to_bits(),
            "add3 b={b} a={a}"
        );
        total += 3;
    }
    let mut s = 0x50f7_1000u32;
    for _ in 0..2000 {
        let b = rng(&mut s, -1000.0, 1000.0);
        let a = rng(&mut s, -1000.0, 1000.0);
        let w = rng(&mut s, -1000.0, 1000.0);
        let s1 = (f64::from(b) + f64::from(a)) as f32;
        let s2 = (f64::from(b) + f64::from(a) + f64::from(w)) as f32;
        assert_eq!(
            run_add1(&mut emu, b, a, w).to_bits(),
            s1.to_bits(),
            "add1 fuzz"
        );
        assert_eq!(
            run_add2(&mut emu, b, a, w).to_bits(),
            s2.to_bits(),
            "add2 fuzz"
        );
        assert_eq!(
            run_add3(&mut emu, b, a).to_bits(),
            s1.to_bits(),
            "add3 fuzz"
        );
        total += 3;
    }
    eprintln!("glue_50f710_adds: {total} bit-exact (add1/add2[+st1]/add3 copied-byte islands)");
}

// ===========================================================================================
// 0x50fae0 / 0x50fb80 / 0x50fbc0 — flag-gated deg→rad convert (fmul[π/180]); 0x50f990 — rad→deg
// (fmul[180/π], f64 store). Each fmul diffed at its own address; the getter 0x64aaa0 patched.
// ===========================================================================================
fn patch_getter_fld(emu: &mut Emu, cell: u64) {
    // 0x64aaa0 -> `fld dword [cell]; ret 4`
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc2);
    code.extend_from_slice(&4u16.to_le_bytes());
    emu.write(0x64aaa0, &code);
}

#[test]
fn glue_deg2rad_gated() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let cell = emu.alloc(4);
    patch_getter_fld(&mut emu, cell);
    let src = emu.alloc(0x40);
    let out = emu.alloc(0x40);
    let edx_ptr = emu.alloc(4);
    emu.write_u32(edx_ptr, src as u32);

    // 0x50fb80: 0x50fb92..0x50fbab — xor eax; mov edi,[0x84f474]; test; mov edi,[edx]; fld[edi+0x24];
    // je; fmul[π/180]; fstp [ecx].  edx->edx_ptr->src; ecx=out; [0x84f474]=flag.
    let run_fb80 = |emu: &mut Emu, angle: f32, flag: bool| -> f32 {
        emu.write_u32(src + 0x24, angle.to_bits());
        emu.write_u32(0x84f474, u32::from(flag));
        emu.set_reg(RegisterX86::EDX, edx_ptr);
        emu.set_reg(RegisterX86::ECX, out);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50fb92, 0x50fbab);
        emu.read_f32(out)
    };
    // 0x50fae0: 0x50fb1e..0x50fb44 — xor esi; mov eax,[0x84f474]; test; (branch) getter; fmul[π/180]
    // or getter; fstp [edi].  ebx->src (the getter arg), edi=out, [0x84f474]=flag; getter patched
    // to return [cell]=angle.
    let run_fae0 = |emu: &mut Emu, angle: f32, flag: bool| -> f32 {
        emu.write_u32(cell, angle.to_bits());
        emu.write_u32(src, 0); // getter arg (unused by the patch)
        emu.write_u32(0x84f474, u32::from(flag));
        emu.set_reg(RegisterX86::EBX, src);
        emu.set_reg(RegisterX86::EDI, out);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50fb1e, 0x50fb44);
        emu.read_f32(out)
    };

    let mut total = 0u32;
    for flag in [true, false] {
        for angle in [0.0f32, 1.0, 90.0, 180.0, -45.0, 360.0] {
            let want = deg2rad_gated(angle, flag).to_bits();
            assert_eq!(
                run_fb80(&mut emu, angle, flag).to_bits(),
                want,
                "fb80 a={angle} f={flag}"
            );
            assert_eq!(
                run_fae0(&mut emu, angle, flag).to_bits(),
                want,
                "fae0 a={angle} f={flag}"
            );
            total += 2;
        }
    }
    let mut s = 0x50fa_e000u32;
    for _ in 0..1500 {
        let flag = (next(&mut s) >> 8) & 1 == 1;
        let angle = rng(&mut s, -720.0, 720.0);
        let want = deg2rad_gated(angle, flag).to_bits();
        assert_eq!(run_fb80(&mut emu, angle, flag).to_bits(), want, "fb80 fuzz");
        assert_eq!(run_fae0(&mut emu, angle, flag).to_bits(), want, "fae0 fuzz");
        total += 2;
    }
    eprintln!("glue_deg2rad_gated: {total} bit-exact (0x50fb80 + 0x50fae0 fmul[π/180])");
}

#[test]
fn glue_50f990_rad2deg() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let src = emu.alloc(4);
    // 0x50f9d0..0x50f9e8 — mov eax,[esi+0x84f474]; fld [ebx]; test eax; je; fmul[180/π]; sub esp,8;
    // fstp QWORD [esp].  esi=0 (so flag = [0x84f474]); ebx->src; read f64 at (esp-8).
    let run = |emu: &mut Emu, angle: f32, flag: bool| -> u64 {
        emu.write_u32(src, angle.to_bits());
        emu.write_u32(0x84f474, u32::from(flag));
        emu.set_reg(RegisterX86::ESI, 0);
        emu.set_reg(RegisterX86::EBX, src);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50f9d0, 0x50f9e8);
        u64::from(emu.read_u32(f.esp - 8)) | (u64::from(emu.read_u32(f.esp - 4)) << 32)
    };
    let mut total = 0u32;
    for flag in [true, false] {
        for angle in [
            0.0f32,
            1.0,
            std::f32::consts::PI,
            -1.5,
            std::f32::consts::TAU,
        ] {
            let want = rad2deg_gated(angle, flag).to_bits();
            assert_eq!(
                run(&mut emu, angle, flag),
                want,
                "rad2deg a={angle} f={flag}"
            );
            total += 1;
        }
    }
    let mut s = 0x50f9_9000u32;
    for _ in 0..2000 {
        let flag = (next(&mut s) >> 8) & 1 == 1;
        let angle = rng(&mut s, -12.0, 12.0);
        assert_eq!(
            run(&mut emu, angle, flag),
            rad2deg_gated(angle, flag).to_bits(),
            "rad2deg fuzz"
        );
        total += 1;
    }
    eprintln!("glue_50f990_rad2deg: {total} bit-exact (0x50f990 fmul[180/π], f64 store)");
}

// 0x50fbc0 — deg→rad gate + change-detect tail (fmul[π/180]; fsubr[field]; fabs; fcomp dt).
#[test]
fn glue_50fbc0_settled() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let cell = emu.alloc(4);
    patch_getter_fld(&mut emu, cell);
    let src = emu.alloc(4);
    let ebx = emu.alloc(4);
    // 0x50fbe0..0x50fc04 — mov eax,[edi]; push; call getter; mov eax,[esi+0x84f474]; test; je;
    // fmul[π/180]; fsubr [ebx]; fabs; fcomp dt; fnstsw.  esi=0; edi->src(getter arg); ebx->field.
    let run = |emu: &mut Emu, srcv: f32, field: f32, flag: bool| -> u8 {
        emu.write_u32(cell, srcv.to_bits());
        emu.write_u32(src, 0);
        emu.write_u32(ebx, field.to_bits());
        emu.write_u32(0x84f474, u32::from(flag));
        emu.set_reg(RegisterX86::ESI, 0);
        emu.set_reg(RegisterX86::EDI, src);
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50fbe0, 0x50fc04);
        ah_of(emu)
    };
    let mut total = 0u32;
    let mut early = 0u32;
    let mut cont = 0u32;
    for flag in [true, false] {
        for (srcv, field, _l) in [
            (0.0f32, DT_F32, "eq"),
            (0.0, DT_F32 * 0.5, "below"),
            (0.0, DT_F32 * 2.0, "above"),
        ] {
            let converted = deg2rad_gated(srcv, flag);
            let ah = run(&mut emu, srcv, field, flag);
            // test ah,5; jp -> taken (early-out) iff odd parity (|d|<dt ordered). settled()⇒odd.
            let bin = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(
                bin,
                deg2rad_settled(field, converted),
                "50fbc0 s={srcv} field={field} f={flag}"
            );
            if bin {
                early += 1;
            } else {
                cont += 1;
            }
            total += 1;
        }
    }
    let mut s = 0x50fb_c000u32;
    for _ in 0..2000 {
        let flag = (next(&mut s) >> 8) & 1 == 1;
        let srcv = rng(&mut s, -180.0, 180.0);
        let converted = deg2rad_gated(srcv, flag);
        let field = converted + rng(&mut s, -0.005, 0.005);
        let ah = run(&mut emu, srcv, field, flag);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin, deg2rad_settled(field, converted), "50fbc0 fuzz");
        if bin {
            early += 1;
        } else {
            cont += 1;
        }
        total += 1;
    }
    eprintln!("glue_50fbc0_settled: {total} bit-exact (fmul[π/180]+fsubr+fabs; early={early} cont={cont})");
    assert!(early > 0 && cont > 0, "need both early-out/continue");
}

// ===========================================================================================
// 0x50fc60 / 0x50fcd0 — unit-convert via __ftol: fld[G+0x24]; fdivr arg; fmul[1000]; __ftol.
// fdivr×1 + fmul×1, byte-identical across the two functions (one slice covers both).
// ===========================================================================================
#[test]
fn glue_unit_convert_ftol() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let g = emu.alloc(0x40);
    // Both functions' slices are byte-identical (verified: 0x50fc63..0x50fc79 == 0x50fcd3..0x50fce9).
    let run = |emu: &mut Emu, start: u64, end: u64, arg: f32, gval: f32| -> i32 {
        // [0xbe0fbc] holds the G struct address directly; fld [eax+0x24] reads G.
        emu.write_u32(0xbe0fbc, g as u32);
        emu.write_u32(g + 0x24, gval.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.set_reg(RegisterX86::ECX, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(start, end);
        emu.get_reg(RegisterX86::EAX) as i32
    };
    let mut total = 0u32;
    for &(start, end) in &[(0x50fc63u64, 0x50fc7eu64), (0x50fcd3, 0x50fcee)] {
        for (arg, gv) in [
            (1000.0f32, 1.0f32),
            (1500.0, 1.0),
            (-1500.0, 1.0),
            (2.0, 4.0),
            (0.0, 1.0),
            (1999.999, 1.0),
            // NaN, the infinities and quotients past `i32` and `i64`: `__ftol` returns the low
            // dword of its qword, 0 for the indefinite.
            (f32::NAN, 1.0),
            (f32::INFINITY, 1.0),
            (f32::NEG_INFINITY, 1.0),
            (4.0e6, 1.0),
            (-4.0e6, 1.0),
            (1.0e30, 1.0),
            (1.0, 0.0),
            (0.0, 0.0),
            (1.0, 1.0e-30),
            (1.0, f32::NAN),
        ] {
            assert_eq!(
                run(&mut emu, start, end, arg, gv),
                unit_convert_ftol(arg, gv),
                "ftol @ {start:#x} arg={arg} g={gv}"
            );
            total += 1;
        }
        let mut s = 0x50fc_6000u32;
        for _ in 0..3000 {
            let arg = rng(&mut s, -5000.0, 5000.0);
            let gv = rng(&mut s, 0.5, 50.0);
            assert_eq!(
                run(&mut emu, start, end, arg, gv),
                unit_convert_ftol(arg, gv),
                "ftol @ {start:#x} fuzz arg={arg} g={gv}"
            );
            total += 1;
        }
    }
    eprintln!(
        "glue_unit_convert_ftol: {total} bit-exact (0x50fc60/0x50fcd0 shared fdivr+fmul+__ftol)"
    );
}

// ===========================================================================================
// 0x5106f0 / 0x5107f0 / 0x510850 — eps predicates.
// ===========================================================================================
#[test]
fn glue_eps_predicates() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    // 0x5106f0: fld[+0x118];fadd[+0x114];fadd[+0x110];fabs;fcomp dt;fnstsw — fadd×2+fabs×1.
    let run_sum3 = |emu: &mut Emu, a: f32, b: f32, c: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0x118, a.to_bits());
        emu.write_u32(f.this + 0x114, b.to_bits());
        emu.write_u32(f.this + 0x110, c.to_bits());
        emu.run(0x5106f3, 0x51070f);
        ah_of(emu)
    };
    let mut sf = 0u32;
    let mut ss = 0u32;
    for (a, b, c, _l) in [
        (DT_F32, 0.0f32, 0.0f32, "eq"),
        (DT_F32 * 0.5, 0.0, 0.0, "below"),
        (DT_F32 * 2.0, 0.0, 0.0, "above"),
        (f32::NAN, 0.0, 0.0, "nan"),
    ] {
        let ah = run_sum3(&mut emu, a, b, c);
        let bin = (ah & 0x5).count_ones() % 2 == 1; // jp taken (body) iff odd parity
        assert_eq!(bin, sum3_abs_lt_dt(a, b, c), "sum3 a={a} b={b} c={c}");
        if bin {
            sf += 1;
        } else {
            ss += 1;
        }
    }
    let mut s = 0x5106_f000u32;
    for _ in 0..1500 {
        let a = rng(&mut s, -0.005, 0.005);
        let b = rng(&mut s, -0.005, 0.005);
        let c = rng(&mut s, -0.005, 0.005);
        let ah = run_sum3(&mut emu, a, b, c);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin, sum3_abs_lt_dt(a, b, c), "sum3 fuzz");
        if bin {
            sf += 1;
        } else {
            ss += 1;
        }
    }
    assert!(sf > 0 && ss > 0, "sum3: need both");

    // 0x5107f0: fld[+0x104];fabs;fcomp dt;fnstsw — fabs×1.
    let run_abs = |emu: &mut Emu, v: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0x104, v.to_bits());
        emu.run(0x5107f3, 0x510803);
        ah_of(emu)
    };
    let mut af = 0u32;
    let mut as_ = 0u32;
    for (v, _l) in [
        (DT_F32, "eq"),
        (DT_F32 * 0.5, "below"),
        (DT_F32 * 2.0, "above"),
        (f32::NAN, "nan"),
    ] {
        let ah = run_abs(&mut emu, v);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin, abs_lt_dt(v), "abs v={v}");
        if bin {
            af += 1;
        } else {
            as_ += 1;
        }
    }
    let mut s = 0x5107_f000u32;
    for _ in 0..1500 {
        let v = rng(&mut s, -0.005, 0.005);
        let ah = run_abs(&mut emu, v);
        assert_eq!((ah & 0x5).count_ones() % 2 == 1, abs_lt_dt(v), "abs fuzz");
        if (ah & 0x5).count_ones() % 2 == 1 {
            af += 1;
        } else {
            as_ += 1;
        }
    }
    assert!(af > 0 && as_ > 0, "abs: need both");

    // 0x510850 near-eq: fsubr[+0xf0];fabs;fcomp eps;fnstsw — fsubr×1+fabs×1. st0 entry = delegated
    // value; an island prologue `fld [cell]` precedes the copied 0x510910.. bytes (byte-verified).
    let cell = emu.alloc(4);
    let region = emu.alloc(0x40);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [cell]
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd8, 0xae, 0xf0, 0x00, 0x00, 0x00]); // fsubr [esi+0xf0]  (0x510910)
    code.extend_from_slice(&[0xd9, 0xe1]); // fabs                                       (0x510916)
    code.extend_from_slice(&[0xd8, 0x1d, 0xd4, 0x29, 0x80, 0x00]); // fcomp [0x8029d4]   (0x510918)
    code.extend_from_slice(&[0xdf, 0xe0]); // fnstsw ax                                  (0x51091e)
    let end = region + code.len() as u64;
    emu.write(region, &code);
    let run_neq = |emu: &mut Emu, field: f32, deleg: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(cell, deleg.to_bits());
        emu.write_u32(f.this + 0xf0, field.to_bits());
        emu.run(region, end);
        ah_of(emu)
    };
    let mut nf = 0u32;
    let mut ns = 0u32;
    for (field, deleg, _l) in [
        (eps, 0.0f32, "eq"),
        (eps * 0.5, 0.0, "below"),
        (eps * 2.0, 0.0, "above"),
        (0.0, f32::NAN, "nan"),
    ] {
        let ah = run_neq(&mut emu, field, deleg);
        // test ah,5; jnp -> taken (return 0) iff EVEN parity; falls to ret 1 iff odd (|d|<eps). 1⇒odd.
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(
            bin,
            near_eq_eps(field, deleg),
            "neareq field={field} deleg={deleg}"
        );
        if bin {
            nf += 1;
        } else {
            ns += 1;
        }
    }
    let mut s = 0x5108_5000u32;
    for _ in 0..1500 {
        let field = rng(&mut s, -10.0, 10.0);
        let deleg = field + rng(&mut s, -1.0e-6, 1.0e-6);
        let ah = run_neq(&mut emu, field, deleg);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin, near_eq_eps(field, deleg), "neareq fuzz");
        if bin {
            nf += 1;
        } else {
            ns += 1;
        }
    }
    assert!(nf > 0 && ns > 0, "neareq: need both");
    eprintln!("glue_eps_predicates: sum3({sf}/{ss}) abs({af}/{as_}) neareq({nf}/{ns}) bit-exact");
}

// ===========================================================================================
// 0x50fee0 — mouse-look yaw rate (fmul×2+fmulp) + integrate (fmul+fadd). (pitch rate is the
// byte-twin of yaw modulo the cvar/arg disp; the pitch-clamp ±89° is shared with the family.)
// ===========================================================================================
#[test]
fn glue_50fee0_rates() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // yaw rate slice 0x50ff5b..0x50ff6f: fld[-0x10]; fmul[π/180]; fld[ebp+0xc]; fmul[cvar]; fmulp;
    // fstp[-0x8].
    let cvar0 = f32::from_bits(0x3ada_740e); // 1/600
    let run_yaw = |emu: &mut Emu, g: f32, arg: f32, cvar: f32| -> f32 {
        emu.write_u32(f.ebp - 0x10, g.to_bits());
        emu.write_u32(f.ebp + 0xc, arg.to_bits());
        emu.write_u32(0x808ac0, cvar.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50ff5b, 0x50ff72); // until AFTER the fstp [ebp-8] at 0x50ff6f
        emu.read_f32(f.ebp - 0x8)
    };
    // integrate slice 0x510020..0x51002c: fld[-0x4]=sign; fmul[-0x8]=rate; fadd[+0x104]; fst[+0x104].
    let run_int = |emu: &mut Emu, sign: f32, rate: f32, field: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0x4, sign.to_bits());
        emu.write_u32(f.ebp - 0x8, rate.to_bits());
        emu.write_u32(f.this + 0x104, field.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x510020, 0x510032); // until AFTER the fst [esi+0x104] at 0x51002c
        emu.read_f32(f.this + 0x104)
    };

    let mut total = 0u32;
    for (g, arg) in [(1.0f32, 1.0f32), (0.0, 5.0), (-2.5, 3.0), (90.0, -1.0)] {
        let want = mouselook_rate_yaw(g, arg, cvar0).to_bits();
        assert_eq!(
            run_yaw(&mut emu, g, arg, cvar0).to_bits(),
            want,
            "yaw g={g} arg={arg}"
        );
        total += 1;
    }
    for (sign, rate, field) in [
        (1.0f32, 0.5f32, 10.0f32),
        (-1.0, 0.5, 10.0),
        (1.0, 1.0, 16_777_216.0),
    ] {
        let want = pitch_integrate(sign, rate, field).to_bits();
        assert_eq!(
            run_int(&mut emu, sign, rate, field).to_bits(),
            want,
            "int sign={sign}"
        );
        total += 1;
    }
    let mut s = 0x50fe_e000u32;
    for _ in 0..2000 {
        let g = rng(&mut s, -10.0, 10.0);
        let arg = rng(&mut s, -50.0, 50.0);
        let cvar = rng(&mut s, 0.0005, 0.01);
        assert_eq!(
            run_yaw(&mut emu, g, arg, cvar).to_bits(),
            mouselook_rate_yaw(g, arg, cvar).to_bits(),
            "yaw fuzz"
        );
        let sign = if (next(&mut s) >> 8) & 1 == 1 {
            1.0
        } else {
            -1.0
        };
        let rate = rng(&mut s, -5.0, 5.0);
        let field = rng(&mut s, -360.0, 360.0);
        assert_eq!(
            run_int(&mut emu, sign, rate, field).to_bits(),
            pitch_integrate(sign, rate, field).to_bits(),
            "int fuzz"
        );
        total += 2;
    }
    eprintln!("glue_50fee0_rates: {total} bit-exact (yaw fmul×2+fmulp, integrate fmul+fadd)");
}

// ===========================================================================================
// 0x5103e0 — add-then-clamp (fadd + ±89° select), throttle fmul[0.01], deg→rad fmul[π/180]×2.
// fadd×1 + fmul×3.
// ===========================================================================================
#[test]
fn glue_5103e0_arith() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let hi = f32::from_bits(0x3fc6_d3f2); // +89°

    // (1) add-then-clamp: 0x5103ef..0x510449 — fld[ebx+0x104]; fadd[ebx+0xf4]; ...; clamp; fstp[ebp-4].
    let run_clamp = |emu: &mut Emu, a: f32, b: f32| -> f32 {
        emu.set_reg(RegisterX86::EBX, f.this);
        emu.write_u32(f.this + 0x104, a.to_bits());
        emu.write_u32(f.this + 0xf4, b.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x5103ef, 0x510449);
        emu.read_f32(f.ebp - 0x4)
    };
    let mut total = 0u32;
    let mut lo_n = 0u32;
    let mut hi_n = 0u32;
    let mut mid_n = 0u32;
    // deterministic boundaries at exactly ±89°.
    for (a, b, _l) in [
        (hi, 0.0f32, "==hi"),
        (-hi, 0.0, "==lo"),
        (2.0, 0.0, ">hi"),
        (-2.0, 0.0, "<lo"),
        (0.5, 0.0, "mid"),
        (hi, 1.0e-6, ">hi-by-eps"),
    ] {
        let g = run_clamp(&mut emu, a, b);
        assert_eq!(
            g.to_bits(),
            add_then_pitch_clamp(a, b).to_bits(),
            "clamp a={a} b={b}"
        );
        total += 1;
    }
    let mut s = 0x5103_e000u32;
    for _ in 0..3000 {
        let a = rng(&mut s, -3.5, 3.5);
        let b = rng(&mut s, -0.5, 0.5);
        let g = run_clamp(&mut emu, a, b);
        assert_eq!(
            g.to_bits(),
            add_then_pitch_clamp(a, b).to_bits(),
            "clamp fuzz a={a} b={b}"
        );
        let sum = f64::from(a) + f64::from(b);
        if sum < f64::from(-hi) {
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
        "clamp: need all three select branches"
    );

    // (2) throttle fmul[0.01]: 0x5104ba..0x5104c9 — fild[ebp-0xc]; fmul[0.01]; fcomp 0.0; fnstsw.
    // The product is left live; we diff its SIGN via the fcomp (sign(n·0.01)=sign(n), exact).
    let run_thr = |emu: &mut Emu, n: i32| -> u8 {
        emu.write_u32(f.ebp - 0xc, n as u32);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x5104ba, 0x5104cb);
        ah_of(emu)
    };
    for n in [0i32, 1, 50, 100, -1] {
        let ah = run_thr(&mut emu, n);
        // fcomp(product, 0.0): odd parity {C0,C2} ⟺ product < 0 (ordered). throttle_frac<0 ⟺ n<0.
        let bin_lt0 = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin_lt0, throttle_frac(n) < 0.0, "throttle n={n}");
        total += 1;
    }

    // (3) deg→rad ×2: 0x5104da..0x5104fc — src1=[0xbe0f4c]+0x24, src2=[0xbe0fd8]+0x24, each ·π/180.
    let s1 = emu.alloc(0x40);
    let s2 = emu.alloc(0x40);
    let p1 = emu.alloc(4);
    let p2 = emu.alloc(4);
    let run_d2r = |emu: &mut Emu, a1: f32, a2: f32| -> (f32, f32) {
        emu.write_u32(p1, s1 as u32);
        emu.write_u32(p2, s2 as u32);
        emu.write_u32(0xbe0f4c, p1 as u32);
        emu.write_u32(0xbe0fd8, p2 as u32);
        emu.write_u32(s1 + 0x24, a1.to_bits());
        emu.write_u32(s2 + 0x24, a2.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(p1, s1 as u32);
        emu.write_u32(p2, s2 as u32);
        // [0xbe0f4c] indirection: mov eax,[0xbe0f4c] gives p1; fld[p1+0x24]? No — fld[eax+0x24] with
        // eax=[0xbe0f4c]=ptr-to-src. So [0xbe0f4c] must hold the src struct address directly.
        emu.write_u32(0xbe0f4c, s1 as u32);
        emu.write_u32(0xbe0fd8, s2 as u32);
        emu.run(0x5104da, 0x5104fc);
        (emu.read_f32(f.ebp - 0x8), emu.read_f32(f.ebp - 0xc))
    };
    for (a1, a2) in [(0.0f32, 90.0f32), (180.0, -45.0), (1.0, 360.0)] {
        let (g1, g2) = run_d2r(&mut emu, a1, a2);
        assert_eq!(
            g1.to_bits(),
            deg2rad_gated(a1, true).to_bits(),
            "d2r1 a={a1}"
        );
        assert_eq!(
            g2.to_bits(),
            deg2rad_gated(a2, true).to_bits(),
            "d2r2 a={a2}"
        );
        total += 2;
    }
    let mut s = 0x5104_da00u32;
    for _ in 0..1500 {
        let a1 = rng(&mut s, -720.0, 720.0);
        let a2 = rng(&mut s, -720.0, 720.0);
        let (g1, g2) = run_d2r(&mut emu, a1, a2);
        assert_eq!(g1.to_bits(), deg2rad_gated(a1, true).to_bits(), "d2r1 fuzz");
        assert_eq!(g2.to_bits(), deg2rad_gated(a2, true).to_bits(), "d2r2 fuzz");
        total += 2;
    }
    eprintln!("glue_5103e0_arith: {total} bit-exact (fadd+clamp lo={lo_n} hi={hi_n} mid={mid_n}, fmul[0.01], fmul[π/180]×2)");
}

// ===========================================================================================
// 0x50f710 — 3-axis deadband (fcomp-only; eax=1 iff any axis differs). Diffs the documented
// branch reconstruction against the binary (no FP-arith, but exercises the family's parity logic).
// ===========================================================================================
#[test]
fn glue_50f710_deadband() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    // slice 0x50f752..0x50f78b: three fld[ebp-?]; fcomp[esi+thr]; test ah,0x44; jp 0x50f786. The
    // candidate vector is [ebp-0xc]/[ebp-8]/[ebp-4]; thresholds [esi+0x174/0x178/0x17c]. Result eax.
    let run = |emu: &mut Emu, v: [f32; 3], thr: [f32; 3]| -> u32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0xc, v[0].to_bits());
        emu.write_u32(f.ebp - 0x8, v[1].to_bits());
        emu.write_u32(f.ebp - 0x4, v[2].to_bits());
        emu.write_u32(f.this + 0x174, thr[0].to_bits());
        emu.write_u32(f.this + 0x178, thr[1].to_bits());
        emu.write_u32(f.this + 0x17c, thr[2].to_bits());
        emu.run(0x50f752, 0x50f78b);
        emu.get_reg(RegisterX86::EAX) as u32
    };
    let mut total = 0u32;
    let mut diff_n = 0u32;
    let mut same_n = 0u32;
    for (v, thr, _l) in [
        ([1.0f32, 2.0, 3.0], [1.0f32, 2.0, 3.0], "all-eq"),
        ([1.0, 2.0, 3.0], [9.0, 2.0, 3.0], "axis0-diff"),
        ([f32::NAN, 2.0, 3.0], [1.0, 2.0, 3.0], "nan"),
    ] {
        let eax = run(&mut emu, v, thr);
        assert_eq!(
            eax == 1,
            deadband_eq_any(v, thr),
            "deadband {v:?} vs {thr:?}"
        );
        if eax == 1 {
            diff_n += 1;
        } else {
            same_n += 1;
        }
        total += 1;
    }
    let mut s = 0x50f7_1100u32;
    for _ in 0..1500 {
        // half the time make all three exactly equal (the eax=0 case).
        let thr = [
            rng(&mut s, -5.0, 5.0),
            rng(&mut s, -5.0, 5.0),
            rng(&mut s, -5.0, 5.0),
        ];
        let v = if (next(&mut s) >> 8) & 1 == 0 {
            thr // exactly equal
        } else {
            [thr[0] + rng(&mut s, -1.0, 1.0), thr[1], thr[2]]
        };
        let eax = run(&mut emu, v, thr);
        assert_eq!(eax == 1, deadband_eq_any(v, thr), "deadband fuzz");
        if eax == 1 {
            diff_n += 1;
        } else {
            same_n += 1;
        }
        total += 1;
    }
    assert!(diff_n > 0 && same_n > 0, "deadband: need both outcomes");
    eprintln!("glue_50f710_deadband: {total} bit-exact (differs={diff_n} all-equal={same_n})");
}

// ===========================================================================================
// 0x50fee0 — the remaining owned FP beyond the yaw rate + integrate (covered above):
//   pitch-rate triple (fmul×2+fmulp @0x50ff35/49/56), the fabs trio (@0x50ffd8/fff3/510002),
//   the upper-limit fsub (@0x510050), and the two integrator-arg pushes (sign·rate fmul @0x5100b3;
//   −(yaw·g) fmul+fchs @0x5100f3/f8). Every 0x50fee0 FP-arith op is in a diffed slice or island.
// ===========================================================================================
#[test]
fn glue_50fee0_rest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let cvar1 = f32::from_bits(0x3aa3_d70a); // 0x808ac4 = 1/800
    let mut total = 0u32;

    // pitch-rate triple slice 0x50ff32..0x50ff5b: fld[-0x14]; fmul[π/180]; fld[ebp+8]; fmul[0x808ac4];
    // fmulp; fstp[-0x14]. (Same shape as yaw; covers fmul@0x50ff35, fmul@0x50ff49, fmulp@0x50ff56.)
    // [0xbe1040]+0x28 is a sign-flag deref that does NOT affect the magnitude [ebp-0x14]; point it
    // at a valid scratch struct so the deref is mapped.
    let flag_struct = emu.alloc(0x40);
    let run_pitch = |emu: &mut Emu, g: f32, arg: f32, cvar: f32| -> f32 {
        emu.write_u32(0xbe1040, flag_struct as u32);
        emu.write_u32(f.ebp - 0x14, g.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.write_u32(0x808ac4, cvar.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50ff32, 0x50ff5b); // until AFTER fstp[-0x14] at 0x50ff58
        emu.read_f32(f.ebp - 0x14)
    };
    for (g, arg) in [(1.0f32, 1.0f32), (0.0, 5.0), (-2.5, 3.0), (90.0, -1.0)] {
        assert_eq!(
            run_pitch(&mut emu, g, arg, cvar1).to_bits(),
            mouselook_rate_pitch(g, arg, cvar1).to_bits(),
            "pitch g={g} arg={arg}"
        );
        total += 1;
    }
    let mut s = 0x50ff_3200u32;
    for _ in 0..1500 {
        let g = rng(&mut s, -10.0, 10.0);
        let arg = rng(&mut s, -50.0, 50.0);
        let cvar = rng(&mut s, 0.0005, 0.01);
        assert_eq!(
            run_pitch(&mut emu, g, arg, cvar).to_bits(),
            mouselook_rate_pitch(g, arg, cvar).to_bits(),
            "pitch fuzz"
        );
        total += 1;
    }

    // fabs trio. (1) 0x50ffd2..0x50ffe5: fld[+0x104];fabs;fcomp dt;fnstsw;test ah,5;jnp. body
    // (mov ecx,1, NOT taken) iff |field|<dt ordered → odd parity.
    let run_fabs1 = |emu: &mut Emu, v: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0x104, v.to_bits());
        emu.run(0x50ffd2, 0x50ffe2);
        ah_of(emu)
    };
    // (2) 0x50fff0..0x50fffa: fld[-0x8];fabs;fcomp[-0xc];fnstsw;test ah,0x41;jne. fall-through
    // (mov ecx,1) iff |a|>b strictly (ZF=1 ⇒ masked AH==0).
    let run_fabs2 = |emu: &mut Emu, a: f32, b: f32| -> u8 {
        emu.write_u32(f.ebp - 0x8, a.to_bits());
        emu.write_u32(f.ebp - 0xc, b.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50fff0, 0x50fffa);
        ah_of(emu)
    };
    // (3) 0x50ffff..0x510009: fld[-0x14];fabs;fcomp[-0x18];fnstsw;test ah,5;jp. fall-through
    // (mov ecx,1) iff |a|<b ordered → odd parity.
    let run_fabs3 = |emu: &mut Emu, a: f32, b: f32| -> u8 {
        emu.write_u32(f.ebp - 0x14, a.to_bits());
        emu.write_u32(f.ebp - 0x18, b.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50ffff, 0x510009);
        ah_of(emu)
    };
    let dt = DT_F32;
    for (v, _l) in [
        (dt, "eq"),
        (dt * 0.5, "below"),
        (dt * 2.0, "above"),
        (f32::NAN, "nan"),
    ] {
        let ah = run_fabs1(&mut emu, v);
        assert_eq!(
            (ah & 0x5).count_ones() % 2 == 1,
            abs_lt_dt(v),
            "fabs1 v={v}"
        );
        total += 1;
    }
    for (a, b, _l) in [
        (2.0f32, 1.0f32, "gt"),
        (1.0, 2.0, "lt"),
        (1.0, 1.0, "eq"),
        (f32::NAN, 1.0, "nan"),
    ] {
        let ah = run_fabs2(&mut emu, a, b);
        // jne taken (skip) iff masked AH != 0; fall-through (|a|>b) iff masked AH == 0.
        assert_eq!((ah & 0x41) == 0, abs_gt(a, b), "fabs2 a={a} b={b}");
        total += 1;
    }
    for (a, b, _l) in [
        (1.0f32, 2.0f32, "lt"),
        (2.0, 1.0, "gt"),
        (1.0, 1.0, "eq"),
        (f32::NAN, 1.0, "nan"),
    ] {
        let ah = run_fabs3(&mut emu, a, b);
        let bin = (ah & 0x5).count_ones() % 2 == 1; // |a|<b ordered
        let mine = !f64::from(a).abs().is_nan() && f64::from(a).abs() < f64::from(b);
        assert_eq!(bin, mine, "fabs3 a={a} b={b}");
        total += 1;
    }
    let mut s = 0x50ff_d200u32;
    for _ in 0..1000 {
        let v = rng(&mut s, -0.005, 0.005);
        assert_eq!(
            (run_fabs1(&mut emu, v) & 0x5).count_ones() % 2 == 1,
            abs_lt_dt(v),
            "fabs1 fuzz"
        );
        let a = rng(&mut s, -5.0, 5.0);
        let b = rng(&mut s, 0.0, 5.0);
        assert_eq!(
            (run_fabs2(&mut emu, a, b) & 0x41) == 0,
            abs_gt(a, b),
            "fabs2 fuzz"
        );
        let ah3 = run_fabs3(&mut emu, a, b);
        let mine3 = !f64::from(a).abs().is_nan() && f64::from(a).abs() < f64::from(b);
        assert_eq!((ah3 & 0x5).count_ones() % 2 == 1, mine3, "fabs3 fuzz");
        total += 3;
    }

    // upper-limit fsub slice 0x51004a..0x510059: fld[0x8089dc=−89°]; fsub[+0xf4]; fstp[-0xc].
    let run_sub = |emu: &mut Emu, field: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0xf4, field.to_bits());
        emu.run(0x51004a, 0x510059); // until AFTER fstp[-0xc] at 0x510056
        emu.read_f32(f.ebp - 0xc)
    };
    let neg89 = f32::from_bits(0xbfc6_d3f2); // -89°
    for field in [0.0f32, 1.0, -1.0, 100.0] {
        assert_eq!(
            run_sub(&mut emu, field).to_bits(),
            sub_field(neg89, field).to_bits(),
            "sub field={field}"
        );
        total += 1;
    }
    let mut s = 0x5100_4a00u32;
    for _ in 0..1000 {
        let field = rng(&mut s, -200.0, 200.0);
        assert_eq!(
            run_sub(&mut emu, field).to_bits(),
            sub_field(neg89, field).to_bits(),
            "sub fuzz"
        );
        total += 1;
    }

    // sign·rate push (0x5100b0 fld[-0x4]; 0x5100b3 fmul[-0x8]) and neg-yaw push (0x5100ef fld[-0x10];
    // 0x5100f3 fmul[-0x14]; 0x5100f8 fchs) store to [esp] across intervening push/mov, so each is
    // diffed via a COPIED-OPCODE ISLAND: the exact fmul (`d8 /1`) / fchs (`d9 e0`) opcodes with our
    // own operand cells + a trailing fstp[out] capture (clone-inheritance: operand disp masked).
    let a_cell = emu.alloc(4);
    let b_cell = emu.alloc(4);
    let out = emu.alloc(4);
    let mul_isl = emu.alloc(0x20);
    let mut mc = vec![0xd9u8, 0x05];
    mc.extend_from_slice(&(a_cell as u32).to_le_bytes()); // fld [a]
    mc.extend_from_slice(&[0xd8, 0x0d]);
    mc.extend_from_slice(&(b_cell as u32).to_le_bytes()); // fmul [b]
    mc.extend_from_slice(&[0xd9, 0x1d]);
    mc.extend_from_slice(&(out as u32).to_le_bytes()); // fstp [out]
    let mul_end = mul_isl + mc.len() as u64;
    emu.write(mul_isl, &mc);
    let nmul_isl = emu.alloc(0x20);
    let mut nc = vec![0xd9u8, 0x05];
    nc.extend_from_slice(&(a_cell as u32).to_le_bytes());
    nc.extend_from_slice(&[0xd8, 0x0d]);
    nc.extend_from_slice(&(b_cell as u32).to_le_bytes());
    nc.extend_from_slice(&[0xd9, 0xe0]); // fchs
    nc.extend_from_slice(&[0xd9, 0x1d]);
    nc.extend_from_slice(&(out as u32).to_le_bytes());
    let nmul_end = nmul_isl + nc.len() as u64;
    emu.write(nmul_isl, &nc);
    let run_mul = |emu: &mut Emu, a: f32, b: f32, isl: u64, end: u64| -> f32 {
        emu.write_u32(a_cell, a.to_bits());
        emu.write_u32(b_cell, b.to_bits());
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(isl, end);
        emu.read_f32(out)
    };
    for (a, b) in [
        (1.0f32, 0.5f32),
        (-1.0, 0.5),
        (1.0, 16_777_216.0),
        (-1.0, 16_777_216.0),
    ] {
        assert_eq!(
            run_mul(&mut emu, a, b, mul_isl, mul_end).to_bits(),
            mul2_f32(a, b).to_bits(),
            "signrate a={a} b={b}"
        );
        assert_eq!(
            run_mul(&mut emu, a, b, nmul_isl, nmul_end).to_bits(),
            neg_mul2_f32(a, b).to_bits(),
            "negyaw a={a} b={b}"
        );
        total += 2;
    }
    let mut s = 0x5100_b000u32;
    for _ in 0..1000 {
        let a = rng(&mut s, -10.0, 10.0);
        let b = rng(&mut s, -10.0, 10.0);
        assert_eq!(
            run_mul(&mut emu, a, b, mul_isl, mul_end).to_bits(),
            mul2_f32(a, b).to_bits(),
            "signrate fuzz"
        );
        assert_eq!(
            run_mul(&mut emu, a, b, nmul_isl, nmul_end).to_bits(),
            neg_mul2_f32(a, b).to_bits(),
            "negyaw fuzz"
        );
        total += 2;
    }

    eprintln!("glue_50fee0_rest: {total} bit-exact (pitch-rate, fabs trio, fsub, sign·rate, −yaw)");
}

// ===========================================================================================
// 0x510960 — owned FP: 2× table-clamp fsub (table−1.0 @0x510ad7/0x510ba8), table-interp fmul
// (slope·frac @0x510b61) + fadd (base+offset @0x510b6a), 4× deg→rad (@0x510bd9/be5/bf1/bff), the
// sound-arg fadd (getter+arg @0x510f04). All 9 FP-arith ops in a diffed slice / island.
// ===========================================================================================
#[test]
fn glue_510960_arith() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let mut total = 0u32;

    // (a) table-clamp fsub: 0x510ad1..0x510ae1 — fld[ecx+0x808a1c]; fsub[1.0]; fstp[ebp+ecx-0x28].
    // ecx=0 → reads [0x808a1c], stores [ebp-0x28]. Covers fsub@0x510ad7 (the @0x510ba8 twin is
    // byte-identical modulo the `4*ecx` SIB — same `fsub [0x7ff9d8]`, inherited by clone).
    let run_subone = |emu: &mut Emu, tv: f32| -> f32 {
        emu.set_reg(RegisterX86::ECX, 0);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(0x808a1c, tv.to_bits());
        emu.run(0x510ad1, 0x510ae1); // until AFTER fstp[ebp-0x28] at 0x510add
        emu.read_f32(f.ebp - 0x28)
    };
    for tv in [0.0f32, 1.0, 45.0, -5.0] {
        assert_eq!(
            run_subone(&mut emu, tv).to_bits(),
            sub_one(tv).to_bits(),
            "subone tv={tv}"
        );
        total += 1;
    }
    let mut s = 0x510a_d100u32;
    for _ in 0..800 {
        let tv = rng(&mut s, -90.0, 90.0);
        assert_eq!(
            run_subone(&mut emu, tv).to_bits(),
            sub_one(tv).to_bits(),
            "subone fuzz"
        );
        total += 1;
    }

    // (b) interp fmul (slope·frac @0x510b61) + fadd (base+offset @0x510b6a) — copied-opcode islands.
    let a_cell = emu.alloc(4);
    let b_cell = emu.alloc(4);
    let out = emu.alloc(4);
    let mul_isl = emu.alloc(0x20);
    let mut mc = vec![0xd9u8, 0x05];
    mc.extend_from_slice(&(a_cell as u32).to_le_bytes());
    mc.extend_from_slice(&[0xd8, 0x0d]); // fmul [b]
    mc.extend_from_slice(&(b_cell as u32).to_le_bytes());
    mc.extend_from_slice(&[0xd9, 0x1d]);
    mc.extend_from_slice(&(out as u32).to_le_bytes());
    let mul_end = mul_isl + mc.len() as u64;
    emu.write(mul_isl, &mc);
    let add_isl = emu.alloc(0x20);
    let mut ac = vec![0xd9u8, 0x05];
    ac.extend_from_slice(&(a_cell as u32).to_le_bytes());
    ac.extend_from_slice(&[0xd8, 0x05]); // fadd [b]
    ac.extend_from_slice(&(b_cell as u32).to_le_bytes());
    ac.extend_from_slice(&[0xd9, 0x1d]);
    ac.extend_from_slice(&(out as u32).to_le_bytes());
    let add_end = add_isl + ac.len() as u64;
    emu.write(add_isl, &ac);
    let run_isl = |emu: &mut Emu, a: f32, b: f32, isl: u64, end: u64| -> f32 {
        emu.write_u32(a_cell, a.to_bits());
        emu.write_u32(b_cell, b.to_bits());
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(isl, end);
        emu.read_f32(out)
    };
    for (a, b) in [
        (2.0f32, 3.0f32),
        (1.0, 16_777_216.0),
        (-3.5, 0.25),
        (0.0, 0.0),
    ] {
        assert_eq!(
            run_isl(&mut emu, a, b, mul_isl, mul_end).to_bits(),
            mul2_f32(a, b).to_bits(),
            "interp-mul"
        );
        assert_eq!(
            run_isl(&mut emu, a, b, add_isl, add_end).to_bits(),
            add2_f32(a, b).to_bits(),
            "interp-add"
        );
        total += 2;
    }
    let mut s = 0x510b_6100u32;
    for _ in 0..800 {
        let a = rng(&mut s, -100.0, 100.0);
        let b = rng(&mut s, -100.0, 100.0);
        assert_eq!(
            run_isl(&mut emu, a, b, mul_isl, mul_end).to_bits(),
            mul2_f32(a, b).to_bits(),
            "interp-mul fuzz"
        );
        assert_eq!(
            run_isl(&mut emu, a, b, add_isl, add_end).to_bits(),
            add2_f32(a, b).to_bits(),
            "interp-add fuzz"
        );
        total += 2;
    }

    // (c) 4× deg→rad (@0x510bd9/be5/bf1/bff): each is `fld[g+0x24]; fmul[0x7ffaac=π/180]; fstp`.
    // Diffed via a copied-opcode island reproducing the exact `fmul [0x7ffaac]` (`d8 0d ac fa 7f
    // 00`) — the SAME constant operand the four source fmuls use (clone family, all four identical
    // modulo the source-global load). One island, run per channel value, covers all four addresses.
    let src = emu.alloc(0x40);
    let d2r_isl = emu.alloc(0x20);
    let mut dc = vec![0xd9u8, 0x40, 0x24]; // fld dword [eax+0x24]
    dc.extend_from_slice(&[0xd8, 0x0d, 0xac, 0xfa, 0x7f, 0x00]); // fmul [0x7ffaac]  (verbatim)
    dc.extend_from_slice(&[0xd9, 0x1d]); // fstp [out]
    dc.extend_from_slice(&(out as u32).to_le_bytes());
    let d2r_end = d2r_isl + dc.len() as u64;
    emu.write(d2r_isl, &dc);
    let run_d2r = |emu: &mut Emu, angle: f32| -> f32 {
        emu.write_u32(src + 0x24, angle.to_bits());
        emu.set_reg(RegisterX86::EAX, src);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(d2r_isl, d2r_end);
        emu.read_f32(out)
    };
    for angle in [0.0f32, 1.0, 90.0, 180.0, -45.0, 360.0] {
        assert_eq!(
            run_d2r(&mut emu, angle).to_bits(),
            deg2rad_gated(angle, true).to_bits(),
            "d2r a={angle}"
        );
        total += 1;
    }
    let mut s = 0x510b_bf00u32;
    for _ in 0..2000 {
        let angle = rng(&mut s, -720.0, 720.0);
        assert_eq!(
            run_d2r(&mut emu, angle).to_bits(),
            deg2rad_gated(angle, true).to_bits(),
            "d2r fuzz"
        );
        total += 1;
    }

    // (d) sound-arg fadd (@0x510f04): getter-st0 + arg → f32. Copied-opcode island.
    let arg_cell = emu.alloc(4);
    let add2_isl = emu.alloc(0x20);
    let mut a2 = vec![0xd9u8, 0x05];
    a2.extend_from_slice(&(a_cell as u32).to_le_bytes());
    a2.extend_from_slice(&[0xd8, 0x05]);
    a2.extend_from_slice(&(arg_cell as u32).to_le_bytes()); // fadd [arg_cell]
    a2.extend_from_slice(&[0xd9, 0x1d]);
    a2.extend_from_slice(&(out as u32).to_le_bytes());
    let a2_end = add2_isl + a2.len() as u64;
    emu.write(add2_isl, &a2);
    let run_soundadd = |emu: &mut Emu, getter: f32, arg: f32| -> f32 {
        emu.write_u32(a_cell, getter.to_bits());
        emu.write_u32(arg_cell, arg.to_bits());
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(add2_isl, a2_end);
        emu.read_f32(out)
    };
    for (g, a) in [(1.0f32, 2.0f32), (1.0, 16_777_216.0), (-3.5, 3.5)] {
        assert_eq!(
            run_soundadd(&mut emu, g, a).to_bits(),
            add2_f32(g, a).to_bits(),
            "soundadd"
        );
        total += 1;
    }
    let mut s = 0x510f_0400u32;
    for _ in 0..800 {
        let g = rng(&mut s, -100.0, 100.0);
        let a = rng(&mut s, -100.0, 100.0);
        assert_eq!(
            run_soundadd(&mut emu, g, a).to_bits(),
            add2_f32(g, a).to_bits(),
            "soundadd fuzz"
        );
        total += 1;
    }

    eprintln!(
        "glue_510960_arith: {total} bit-exact (table-sub, interp mul/add, deg→rad ×4, sound fadd)"
    );
}

// ===========================================================================================
// 0x5105e0 / 0x510690 — the cameraBobbing and cameraPivot GATE seams.
//
// Both gates are orchestration (no FP-arith) and both are
// STUBBED by the kernel difftests, which transcribe the body reached once the gate passes. That
// leaves the polarity of their two `fcomp` seams as ungated prose — and a
// downstream client has to reproduce it exactly. Each is `fld [this+off]; fcomp [.rdata k];
// fnstsw ax; test ah,0x41; je <fail>`, where mask 0x41 = {C0,C3} is read by `je` on **ZF**, not
// parity: `je` is taken (the gate FAILS) only on an ordered strictly-greater. So the conjunct is
// `<=`, INCLUSIVE at the threshold, and unordered (NaN) also holds. Both probes carry the
// deterministic `== threshold` case the fuzz never lands on. // ===========================================================================================
#[test]
fn glue_cvar_gate_seams() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let sixth = f32::from_bits(SIXTH_BITS);

    // --- 0x5105e0: fld [ecx+0xec]; fcomp [0x8089ac]; fnstsw ax; test ah,0x41 ---
    let run_dist = |emu: &mut Emu, v: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0xec, v.to_bits());
        emu.run(0x5105e3, 0x5105f2);
        ah_of(emu)
    };
    let (mut hold, mut fail) = (0u32, 0u32);
    let mut check_dist = |emu: &mut Emu, v: f32, label: &str| {
        let ah = run_dist(emu, v);
        // `je` taken (gate fails) iff the masked AH is zero.
        let binary_holds = (ah & 0x41) != 0;
        assert_eq!(binary_holds, dist_le_sixth(v), "dist seam v={v} ({label})");
        if binary_holds {
            hold += 1;
        } else {
            fail += 1;
        }
    };
    // The == boundary is the whole point: 1/6 exactly must HOLD (C3 set ⇒ masked AH != 0).
    check_dist(&mut emu, sixth, "==1/6");
    check_dist(&mut emu, f32::from_bits(sixth.to_bits() - 1), "1ulp below");
    check_dist(&mut emu, f32::from_bits(sixth.to_bits() + 1), "1ulp above");
    check_dist(&mut emu, 0.0, "zero");
    check_dist(&mut emu, -1.0, "negative");
    check_dist(&mut emu, 15.0, "max orbit");
    check_dist(&mut emu, f32::NAN, "nan");
    let mut s = 0x5105_e000u32;
    for _ in 0..1500 {
        let v = rng(&mut s, -0.5, 2.0);
        check_dist(&mut emu, v, "fuzz");
    }
    assert!(hold > 0 && fail > 0, "dist seam: need both outcomes");

    // --- 0x510690: fld [ecx+0xf4]; fcomp [0x7ffd74]; fnstsw ax; test ah,0x41 ---
    let run_pitch = |emu: &mut Emu, v: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.this + 0xf4, v.to_bits());
        emu.run(0x5106c3, 0x5106d1);
        ah_of(emu)
    };
    let (mut phold, mut pfail) = (0u32, 0u32);
    let mut check_pitch = |emu: &mut Emu, v: f32, label: &str| {
        let ah = run_pitch(emu, v);
        let binary_holds = (ah & 0x41) != 0;
        assert_eq!(binary_holds, pitch_le_zero(v), "pitch seam v={v} ({label})");
        if binary_holds {
            phold += 1;
        } else {
            pfail += 1;
        }
    };
    // Exactly 0.0 is a value the pitch channel genuinely lands on, and it must HOLD.
    check_pitch(&mut emu, 0.0, "==0.0");
    check_pitch(&mut emu, -0.0, "-0.0");
    check_pitch(&mut emu, f32::from_bits(1), "smallest subnormal above 0");
    check_pitch(&mut emu, -f32::from_bits(1), "smallest subnormal below 0");
    check_pitch(&mut emu, f32::from_bits(PITCH_CLAMP_BITS), "+89deg");
    check_pitch(&mut emu, -f32::from_bits(PITCH_CLAMP_BITS), "-89deg");
    check_pitch(&mut emu, f32::NAN, "nan");
    let mut s = 0x5106_9000u32;
    for _ in 0..1500 {
        let v = rng(&mut s, -1.6, 1.6);
        check_pitch(&mut emu, v, "fuzz");
    }
    assert!(phold > 0 && pfail > 0, "pitch seam: need both outcomes");

    eprintln!(
        "glue_cvar_gate_seams: dist {hold} hold / {fail} fail, pitch {phold} hold / {pfail} fail \
         (0x5105e0 <=1/6 inclusive, 0x510690 <=0.0 inclusive)"
    );
}
