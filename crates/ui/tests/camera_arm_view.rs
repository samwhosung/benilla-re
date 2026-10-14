//! Differential tests of the arithmetic of `CCamera::SetView` (`0x512e90`) and of four functions
//! in the same band: `BakePortrait` (`0x524f60`), `CGTooltip::BuildUnit` (`0x529fe0`),
//! `CGTooltip::BuildSpell` (`0x52e610`) and `GameTooltip::SetInventoryItem` (`0x532ee0`), each
//! run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed by executing the binary's OWN bytes (`Emu::run`) over just the FP
//! compute — registers/memory/stack synthesized, the `fst/fstp [m32]` value / pushed qword / EAX
//! read back. Every in-body arithmetic op lands in an executed slice (or, for the un-synthesizable
//! Lua marshaller, a byte-verified copied-byte island); the `fcom`/`fcomp`, `fild`, and
//! `fld`/`fst`/`fstp` ride along for free. The unit-scale leaves (`0x41ae60`/`0x41ae70`) and the
//! `__ftol` (`0x40a2b0`) run NATIVELY inside the slices (pure deterministic x87).
//!
//! Coverage of the arithmetic opcode set, counted in each `[addr, true_ret)`:
//! - 0x512e90 [.,0x5132e7]: fadd ×1 (recentre), fsub ×3 + fabs ×4 (four near-zero guards) = 8.
//! - 0x524f60 [.,0x525893]: fsub ×2 + fdiv + fmulp + fdiv + fmul + fsubr (region 1) + fsub ×2 +
//!   fdivp (region 2) = 10.
//! - 0x529fe0 [.,0x52aa15]: fmul + fadd (percent-quantize) = 2.
//! - 0x52e610 [.,0x52f7c3]: fdiv ×2 (two `st,st(1)` timer ratios) = 2.
//! - 0x532ee0 [.,0x533465]: fsub ×1 (slot − 1.0) = 1.
//!
//! Slice → op coverage:
//! - 0x512e90: recentre `[0x512ff0,0x513006)` = fadd; guard A `[0x513024,0x513043)` = fsub+fabs;
//!   guard B `[0x513051,0x513074)` = fsub+fabs; guard C `[0x513082,0x51309c)` = fsub+fabs;
//!   guard D `[0x5130ad,0x5130c4)` = fabs (reads fnstsw AX after each guard's compare).
//! - 0x524f60: region 1 `[0x5251b7,0x525227)` = fsub×2 fdiv fmulp fdiv fmul fsubr (leaves run
//!   native); region 2 `[0x525414,0x525433)` = fsub×2 fdivp.
//! - 0x529fe0: `[0x52a3d2,0x52a3e6)` = fild; fmul; fadd; __ftol (native) → EAX.
//! - 0x52e610: region 1 `[0x52eb8f,0x52ebb8)` = fdiv st,st(1) → qword; region 2
//!   `[0x52ecd5,0x52ecfe)` = fdiv st,st(1) → qword.
//! - 0x532ee0: copied-byte island `fld qword [in]; <source fsub qword [0x8015b8]>; call __ftol`,
//!   the `dc 25 b8 15 80 00` byte-verified == source @0x532fe2 → EAX.
//!
//! Threshold probes: the 0x512e90 guard `|x| == eps` boundary (the `test ah,5; jp`
//! disposition), the 0x529fe0 quantize `n·0.05 + 0.5` integer pivots over the full clamp domain
//! `0..=100`, and the 0x532ee0 `arg − 1.0` __ftol truncate pivots. Fuzz reads the LCG's HIGH bits
//!.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::camera_arm_view::{
    buf_delta, guard_abs, guard_abs_sub, guard_flags, inv_slot, percent_quantize, pitch_recenter,
    ratio_product1, ratio_product2, region2_ratio, timer_ratio, C1000_BITS, C60000_BITS,
    EPS_512_BITS, HALF_BITS, PCT_MUL_BITS,
};

const FTOL: u64 = 0x40a2b0;

// constant VAs the binary's bytes will read (re-asserted against the mapped image).
const EPS_VA: u64 = 0x801360;
const C64_VA: u64 = 0x808de4; // 0x524f60 region multiplier (64.0)
const DIV08_VA: u64 = 0x832a44; // leaf 0x41ae60 reads 0.8
const DIV06_VA: u64 = 0x832a48; // leaf 0x41ae70 reads 0.6
const PCT_VA: u64 = 0x8029c8;
const HALF_VA: u64 = 0x7ffa24;
const C60000_VA: u64 = 0x808ff0;
const C1000_VA: u64 = 0x8022cc;
const ONE_F64_VA: u64 = 0x8015b8;

const C64_BITS: u32 = 0x4280_0000; // 64.0
const DIV08_BITS: u32 = 0x3f4c_cccd; // 0.8 (f32) — leaf 0x41ae60's m0
const DIV06_BITS: u32 = 0x3f19_999a; // 0.6 (f32) — leaf 0x41ae70's m1/m2/m3

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}
fn rng32(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x400);
    let ebp = scratch + 0x300; // room for [ebp-0x300 .. ebp+0x100]
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    Frame { ebp, esp }
}

// ===========================================================================================
// 0x512e90 — camera-pitch re-centre (fadd) + four near-zero ABS guards (fsub/fabs). [.,0x5132e7]
// ===========================================================================================
#[test]
fn glue_512_512e90_pitch_and_guards() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let this = emu.alloc(0x400);
    let ebx_obj = emu.alloc(0x2000);
    assert_eq!(emu.read_u32(EPS_VA), EPS_512_BITS, "0x801360 eps");

    let mut total = 0u32;

    // ---- (A) the pitch re-centre fadd: fld [ebx+0xc98]; jmp; fadd [ebp+8]; push ecx; fstp [esp].
    let run_recenter = |emu: &mut Emu, pitch: f32, param2: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EBX, ebx_obj);
        emu.write_u32(ebx_obj + 0xc98, pitch.to_bits());
        emu.write_u32(f.ebp + 0x8, param2.to_bits());
        emu.run(0x512ff0, 0x513006); // fld [ebx+0xc98]; jmp; fadd [ebp+8]; push ecx; fstp [esp]
        emu.read_f32(emu.get_reg(RegisterX86::ESP)) // fstp dword [esp] after the push ecx
    };
    // the source st0 is `fld [ebx+0xc98]` = the f32 pitch widened to f64. param2 is the f32 arg.
    for (pitch, p2) in [
        (0.0f32, 0.0f32),
        (1.5, -0.25),
        (89.0, 1.0),
        (-3.0625, 2.71875),
        (1.0e6, -1.0e6),
        (f32::from_bits(0x4b00_0001), 0.5),
    ] {
        let got = run_recenter(&mut emu, pitch, p2);
        let want = pitch_recenter(f64::from(pitch), p2);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "pitch_recenter pitch={pitch} p2={p2}"
        );
        total += 1;
    }
    let mut s = 0x512e_9000u32;
    for _ in 0..2500 {
        let pitch = rng32(&mut s, -10000.0, 10000.0);
        let p2 = rng32(&mut s, -10000.0, 10000.0);
        let got = run_recenter(&mut emu, pitch, p2);
        let want = pitch_recenter(f64::from(pitch), p2);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "pitch_recenter fuzz pitch={pitch} p2={p2}"
        );
        total += 1;
    }

    // ---- (B) the four near-zero guards: each `fld [esi+off]; (fsub [coord]); fabs; fcomp [eps];
    // fnstsw ax`. eax/edi are lea-derived (eax = esi + 12*edi + 0xb0; edi from iVar9). Set edi=0 so
    // the guard pointers fall in `this`; write the field at [esi+off] and the coord at the pointer.
    // Read fnstsw AX, compare the masked guard disposition (`ah & 5`).
    //
    // Guard A 0x513024: fld [esi+0xec]; lea eax,[edi+2edi]; lea eax,[esi+4eax+0xb0]; fsub [eax];
    //   mov [ebp+0xc],eax; fabs; fcomp [eps]; fnstsw ax. (edi=0 ⇒ eax=esi+0xb0.)
    let run_guard_a = |emu: &mut Emu, field: f32, coord: f32| -> u16 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.write_u32(this + 0xec, field.to_bits());
        emu.write_u32(this + 0xb0, coord.to_bits()); // [esi+0xb0] = [eax]
        emu.run(0x513024, 0x513043); // ...; fsub [eax]; fabs; fcomp [eps]; fnstsw ax
        (emu.get_reg(RegisterX86::EAX) & 0xffff) as u16
    };
    // Guard B 0x513051: fld [esi+0xf4]; ...; lea eax,[edi+2edi+0x2d]; lea edi,[esi+4eax]; fsub [edi];
    //   ...; fabs; fcomp [eps]; fnstsw ax. (edi=0 ⇒ eax=0x2d ⇒ edi=esi+0xb4.)
    let run_guard_b = |emu: &mut Emu, field: f32, coord: f32| -> u16 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.write_u32(this + 0xf4, field.to_bits());
        emu.write_u32(this + 0xb4, coord.to_bits()); // [esi + 4*0x2d] = [esi+0xb4]
        emu.run(0x513051, 0x513074); // fld [esi+0xf4]; ...; fsub [edi]; fabs; fcomp; fnstsw ax
        (emu.get_reg(RegisterX86::EAX) & 0xffff) as u16
    };
    // Guard C 0x513082: fld [esi+0xf0]; fsub [ebp+8]; ...; fabs; fcomp [eps]; fnstsw ax.
    let run_guard_c = |emu: &mut Emu, field: f32, coord: f32| -> u16 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.write_u32(this + 0xf0, field.to_bits());
        emu.write_u32(f.ebp + 0x8, coord.to_bits()); // [ebp+8] = coord (param2)
        emu.run(0x513082, 0x51309c); // fld [esi+0xf0]; fsub [ebp+8]; fabs; fcomp; fnstsw ax
        (emu.get_reg(RegisterX86::EAX) & 0xffff) as u16
    };
    // Guard D 0x5130ad: fld [esi+0x104]; fabs; ...; fcomp [eps]; fnstsw ax (no fsub).
    let run_guard_d = |emu: &mut Emu, field: f32| -> u16 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.write_u32(this + 0x104, field.to_bits());
        emu.run(0x5130ad, 0x5130c4); // fld [esi+0x104]; fabs; fcomp [eps]; fnstsw ax
        (emu.get_reg(RegisterX86::EAX) & 0xffff) as u16
    };

    let eps = f32::from_bits(EPS_512_BITS);
    // deterministic incl. the |x| == eps boundary (C3) and |x| < eps (C0).
    let pairs = [
        (0.0f32, 0.0f32), // |0| < eps
        (eps, 0.0),       // |eps| == eps  (C3 boundary)
        (-eps, 0.0),      // |−eps| == eps
        (0.0, eps),       // |−eps| == eps
        (eps * 0.5, 0.0), // |eps/2| < eps
        (1.0, 0.0),       // |1| > eps
        (3.5, 3.4),       // small positive delta
        (3.4, 3.5),       // small negative delta
        (1000.0, -1000.0),
        (-50.0, 50.0),
    ];
    for (field, coord) in pairs {
        let wa = guard_flags(guard_abs_sub(field, coord), EPS_512_BITS);
        assert_eq!(
            run_guard_a(&mut emu, field, coord) & 5,
            wa & 5,
            "guardA f={field} c={coord}"
        );
        assert_eq!(
            run_guard_b(&mut emu, field, coord) & 5,
            wa & 5,
            "guardB f={field} c={coord}"
        );
        assert_eq!(
            run_guard_c(&mut emu, field, coord) & 5,
            wa & 5,
            "guardC f={field} c={coord}"
        );
        let wd = guard_flags(guard_abs(field), EPS_512_BITS);
        assert_eq!(run_guard_d(&mut emu, field) & 5, wd & 5, "guardD f={field}");
        total += 4;
    }
    let mut s = 0x5130_0000u32;
    for _ in 0..1500 {
        // mix magnitudes near eps and large so both branches fire.
        let field = rng32(
            &mut s,
            -2.0 * f64::from(eps) as f32,
            2.0 * f64::from(eps) as f32,
        );
        let coord = rng32(
            &mut s,
            -2.0 * f64::from(eps) as f32,
            2.0 * f64::from(eps) as f32,
        );
        let wa = guard_flags(guard_abs_sub(field, coord), EPS_512_BITS);
        assert_eq!(
            run_guard_a(&mut emu, field, coord) & 5,
            wa & 5,
            "guardA fuzz f={field} c={coord}"
        );
        assert_eq!(
            run_guard_c(&mut emu, field, coord) & 5,
            wa & 5,
            "guardC fuzz f={field} c={coord}"
        );
        let wd = guard_flags(guard_abs(field), EPS_512_BITS);
        assert_eq!(
            run_guard_d(&mut emu, field) & 5,
            wd & 5,
            "guardD fuzz f={field}"
        );
        total += 3;
    }
    eprintln!("glue_512_512e90_pitch_and_guards: {total} bit-exact (fadd + fsub×3 + fabs×4)");
}

// ===========================================================================================
// 0x524f60 — two scale-ratio reconstructions over the 0x58a250 buffer. [.,0x525893 ret]
//   region 1 [0x5251b7,0x525227): fsub×2 fdiv fmulp fdiv fmul fsubr (leaves 0x41ae60/0x41ae70
//     run native; their arg is the fixed `push 0x3f800000`=1.0, so m0=0.8, m1=m2=m3=0.6).
//   region 2 [0x525414,0x525433): fsub×2 fdivp.
// ===========================================================================================
#[test]
fn glue_512_524f60_scale_ratios() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    assert_eq!(emu.read_u32(C64_VA), C64_BITS, "0x808de4 != 64.0");
    assert_eq!(emu.read_u32(DIV08_VA), DIV08_BITS, "0x832a44 != 0.8");
    assert_eq!(emu.read_u32(DIV06_VA), DIV06_BITS, "0x832a48 != 0.6");

    // region 1: feed d0 = [ebp-0x38]-[ebp-0x40], d1 = [ebp-0x3c]-[ebp-0x44]; read [ebp-0xc]=product1
    // and [ebp-0x24]=product2. The leaves run native (m0=0.8·1.0, m3=0.6·1.0). d0/d1 must be
    // non-zero (the fdiv divisors). [ebp-0x4] is reused as both d0 and m2 — handled by the slice.
    let run_r1 = |emu: &mut Emu, a0: f32, b0: f32, a1: f32, b1: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x38, a0.to_bits());
        emu.write_u32(f.ebp - 0x40, b0.to_bits());
        emu.write_u32(f.ebp - 0x3c, a1.to_bits());
        emu.write_u32(f.ebp - 0x44, b1.to_bits());
        emu.run(0x5251b7, 0x525227);
        (emu.read_f32(f.ebp - 0xc), emu.read_f32(f.ebp - 0x24))
    };
    // m0 / m3 / m2 are the leaf constants (leaf arg fixed 1.0). product1 = m0·(64/d0);
    // product2 = m2 − ((m3/d1)·64) with m2 = m3 = 0.6 (the prior leaf result stored to [ebp-0x4]).
    let m0 = f64::from(f32::from_bits(DIV08_BITS)); // 0.8 · 1.0
    let m3 = f64::from(f32::from_bits(DIV06_BITS)); // 0.6 · 1.0
    let m2 = f32::from_bits(DIV06_BITS); // 0.6, stored to [ebp-0x4]
    let check_r1 = |emu: &mut Emu, total: &mut u32, a0: f32, b0: f32, a1: f32, b1: f32| {
        let (p1, p2) = run_r1(emu, a0, b0, a1, b1);
        let d0 = buf_delta(a0, b0);
        let d1 = buf_delta(a1, b1);
        let w1 = ratio_product1(m0, d0, C64_BITS);
        let w2 = ratio_product2(m3, d1, m2, C64_BITS);
        assert_eq!(p1.to_bits(), w1.to_bits(), "r1.product1 d0={d0}");
        assert_eq!(p2.to_bits(), w2.to_bits(), "r1.product2 d1={d1}");
        *total += 2;
    };

    // region 2: ((a-b)/(c-d)) as f32. inputs [ebp-0x18],[ebp-0x20],[ebp-0x1c],[ebp-0x24].
    let run_r2 = |emu: &mut Emu, a: f32, b: f32, c: f32, d: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x18, a.to_bits());
        emu.write_u32(f.ebp - 0x20, b.to_bits());
        emu.write_u32(f.ebp - 0x1c, c.to_bits());
        emu.write_u32(f.ebp - 0x24, d.to_bits());
        emu.run(0x525414, 0x525433); // fld;fsub;fld;fsub;fdivp st(1),st;fstp [ebp-0xc]
        emu.read_f32(f.ebp - 0xc)
    };
    let check_r2 = |emu: &mut Emu, total: &mut u32, a: f32, b: f32, c: f32, d: f32| {
        let got = run_r2(emu, a, b, c, d);
        let want = region2_ratio(a, b, c, d);
        assert_eq!(got.to_bits(), want.to_bits(), "r2 a={a} b={b} c={c} d={d}");
        *total += 1;
    };

    let mut total = 0u32;
    // deterministic: positive/negative deltas, exact small integers, the divisor never 0.
    for (a0, b0, a1, b1) in [
        (10.0f32, 2.0f32, 8.0f32, 1.0f32),
        (2.0, 10.0, 1.0, 8.0), // negative deltas
        (1024.0, 0.0, 768.0, 0.0),
        (1.5, 0.5, 3.5, 0.25),
        (100.0, -100.0, 50.0, -50.0),
    ] {
        check_r1(&mut emu, &mut total, a0, b0, a1, b1);
    }
    for (a, b, c, d) in [
        (10.0f32, 2.0f32, 8.0f32, 1.0f32),
        (2.0, 10.0, 8.0, 1.0), // negative numerator
        (10.0, 2.0, 1.0, 8.0), // negative denominator
        (0.0, 0.0, 5.0, 1.0),  // zero numerator
        (640.0, 0.0, 480.0, 0.0),
    ] {
        check_r2(&mut emu, &mut total, a, b, c, d);
    }
    let mut s = 0x524f_6000u32;
    for _ in 0..2500 {
        let a0 = rng32(&mut s, -5000.0, 5000.0);
        let mut b0 = rng32(&mut s, -5000.0, 5000.0);
        if (a0 - b0).abs() < 0.5 {
            b0 -= 10.0;
        } // keep d0 (a divisor) away from 0
        let a1 = rng32(&mut s, -5000.0, 5000.0);
        let mut b1 = rng32(&mut s, -5000.0, 5000.0);
        if (a1 - b1).abs() < 0.5 {
            b1 -= 10.0;
        } // keep d1 (a divisor) away from 0
        check_r1(&mut emu, &mut total, a0, b0, a1, b1);

        let a = rng32(&mut s, -5000.0, 5000.0);
        let b = rng32(&mut s, -5000.0, 5000.0);
        let c = rng32(&mut s, -5000.0, 5000.0);
        let mut d = rng32(&mut s, -5000.0, 5000.0);
        if (c - d).abs() < 0.5 {
            d -= 10.0;
        } // keep the denominator away from 0
        check_r2(&mut emu, &mut total, a, b, c, d);
    }
    eprintln!(
        "glue_512_524f60_scale_ratios: {total} bit-exact (r1 fsub×2 fdiv fmulp fdiv fmul fsubr; r2 fsub×2 fdivp)"
    );
}

// ===========================================================================================
// 0x529fe0 — percent-quantize (i32)(n·0.05 + 0.5), n clamped 0..=100. [.,0x52aa15 ret 4]
//   slice [0x52a3d2,0x52a3e6): fild [ebp-0x18]; fmul [0x8029c8]; fadd [0x7ffa24]; call __ftol → EAX.
// ===========================================================================================
#[test]
fn glue_512_529fe0_percent_quantize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    assert_eq!(emu.read_u32(PCT_VA), PCT_MUL_BITS, "0x8029c8 != 0.05");
    assert_eq!(emu.read_u32(HALF_VA), HALF_BITS, "0x7ffa24 != 0.5");

    let run = |emu: &mut Emu, n: u8| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x18, u32::from(n)); // movzx-style zero-extended clamped byte
        emu.run(0x52a3d2, 0x52a3e6); // fild; fmul[0.05]; fadd[0.5]; call __ftol
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // full clamp domain 0..=100 — every n·0.05+0.5 integer pivot is hit exactly.
    for n in 0u8..=100 {
        assert_eq!(
            run(&mut emu, n),
            percent_quantize(n, PCT_MUL_BITS, HALF_BITS),
            "n={n}"
        );
        total += 1;
    }
    eprintln!(
        "glue_512_529fe0_percent_quantize: {total} bit-exact (fmul[0.05]; fadd[0.5]; __ftol)"
    );
}

// ===========================================================================================
// 0x52e610 — two timer ratios n / C stored as f64 (qword). [.,0x52f7c3 ret 0x1c]
//   region 1 [0x52eb8f,0x52ebb8): (esi?60000:1000); fild [ebp-0x14]; sub esp,8; fdiv st,st(1);
//     fstp qword [esp]. region 2 [0x52ecd5,0x52ecfe): same with [ebp-0x38].
// ===========================================================================================
#[test]
fn glue_512_52e610_timer_ratios() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    assert_eq!(emu.read_u32(C60000_VA), C60000_BITS, "0x808ff0 != 60000");
    assert_eq!(emu.read_u32(C1000_VA), C1000_BITS, "0x8022cc != 1000");

    // region 1: n at [ebp-0x14], esi selects the constant. After `sub esp,8; fstp qword [esp]`,
    // the f64 lands at the halt-time esp.
    let run_r1 = |emu: &mut Emu, n: i32, esi: u32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, u64::from(esi));
        emu.write_u32(f.ebp - 0x14, n as u32);
        emu.run(0x52eb8f, 0x52ebb8);
        let bytes = emu.read(emu.get_reg(RegisterX86::ESP), 8);
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };
    // region 2: n at [ebp-0x38].
    let run_r2 = |emu: &mut Emu, n: i32, esi: u32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, u64::from(esi));
        emu.write_u32(f.ebp - 0x38, n as u32);
        emu.run(0x52ecd5, 0x52ecfe);
        let bytes = emu.read(emu.get_reg(RegisterX86::ESP), 8);
        f64::from_bits(u64::from_le_bytes(bytes.try_into().unwrap()))
    };

    let mut total = 0u32;
    let check = |emu: &mut Emu, total: &mut u32, n: i32, esi: u32| {
        let c_bits = if esi != 0 { C60000_BITS } else { C1000_BITS };
        let r1 = run_r1(emu, n, esi);
        assert_eq!(
            r1.to_bits(),
            timer_ratio(n, c_bits).to_bits(),
            "r1 n={n} esi={esi}"
        );
        let r2 = run_r2(emu, n, esi);
        assert_eq!(
            r2.to_bits(),
            timer_ratio(n, c_bits).to_bits(),
            "r2 n={n} esi={esi}"
        );
        *total += 2;
    };

    // deterministic: 0, exact divisors, both constant branches, negatives.
    for n in [0i32, 1, 1000, 60000, 59999, 30000, -1, -60000, 123456, 7] {
        check(&mut emu, &mut total, n, 1); // /60000
        check(&mut emu, &mut total, n, 0); // /1000
    }
    let mut s = 0x52e6_1000u32;
    for _ in 0..3000 {
        let n = (next(&mut s) >> 8) as i32 - 0x80_0000; // signed-ish range
        let esi = (next(&mut s) >> 8) & 1;
        check(&mut emu, &mut total, n, esi);
    }
    eprintln!("glue_512_52e610_timer_ratios: {total} bit-exact (2× fdiv st,st(1) → qword)");
}

// ===========================================================================================
// 0x532ee0 SetInventoryItem — slot = (i32)(lua_arg − 1.0). [.,0x533465 ret]
//   The Lua marshaller (`0x6f3620` lua_tonumber → double on st0) is un-synthesizable in isolation,
//   so the single owned `fsub qword [0x8015b8]=1.0` is diffed via a COPIED-BYTE ISLAND:
//   `fld qword [in]` seeds st0 with the lua double, the EXACT source `fsub qword` bytes follow
//   (byte-verified == source @0x532fe2), then a real `call __ftol` truncates → EAX.
// ===========================================================================================
#[test]
fn glue_512_532ee0_setinventoryitem_slot() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(
        emu.read(ONE_F64_VA, 8),
        1.0f64.to_bits().to_le_bytes(),
        "0x8015b8 != 1.0 (f64)"
    );

    let in_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);
    // fld qword [in_cell]  =  DD 05 <disp32>
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    // copied source fsub qword [0x8015b8]  =  DC 25 B8 15 80 00
    let fsub_off = code.len() as u64;
    code.extend_from_slice(&[0xdc, 0x25, 0xb8, 0x15, 0x80, 0x00]);
    // call __ftol  =  E8 <rel32>
    let call_va = isl + code.len() as u64;
    let rel = (FTOL as i64 - (call_va as i64 + 5)) as i32;
    code.push(0xe8);
    code.extend_from_slice(&rel.to_le_bytes());
    let isl_end = isl + code.len() as u64; // __ftol's `ret` lands here
    emu.write(isl, &code);
    // byte-verify the island's fsub == source @0x532fe2 (else the island proves nothing).
    assert_eq!(
        emu.read(isl + fsub_off, 6),
        emu.read(0x532fe2, 6),
        "slot island fsub != source @0x532fe2"
    );

    let f = frame(&mut emu);
    let run = |emu: &mut Emu, arg: f64| -> i32 {
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write(in_cell, &arg.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // deterministic: the 1-based Lua slots (1..=N), the __ftol truncate pivots around integers,
    // and the negative / fractional edges.
    let probes = [
        1.0f64,
        2.0,
        16.0,
        23.0,
        1.5,
        1.999_999,
        2.000_001,
        0.999_999,
        1.0 - 1e-12,
        0.0,
        -1.0,
        100.5,
        1000.0,
        1.000_000_000_1,
    ];
    for &v in &probes {
        assert_eq!(run(&mut emu, v), inv_slot(v), "inv_slot v={v}");
        total += 1;
    }
    // dense integer slots 1..=255 (the real Lua inventory domain) and a touch above/below.
    for k in 1u32..=255 {
        for d in [-1e-9, 0.0, 1e-9] {
            let v = f64::from(k) + d;
            assert_eq!(
                run(&mut emu, v),
                inv_slot(v),
                "inv_slot v={v} (k={k} d={d})"
            );
            total += 1;
        }
    }
    let mut s = 0x532e_e000u32;
    for _ in 0..3000 {
        let v = rngf(&mut s, -10.0, 300.0);
        assert_eq!(run(&mut emu, v), inv_slot(v), "inv_slot fuzz v={v}");
        total += 1;
    }
    eprintln!("glue_512_532ee0_setinventoryitem_slot: {total} bit-exact (island: fsub qword[1.0]; __ftol)");
}
