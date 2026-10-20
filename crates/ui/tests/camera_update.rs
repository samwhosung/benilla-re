//! Differential tests of the arithmetic of `CCamera::Update` (`0x50e8d0`: mood fade, look rates,
//! overshoot re-centre, distance fade, shake) and of three Lua and text functions:
//! `GetTradeSkillCooldown` (`0x4fda20`), `ExpandSpellToken` (`0x507710`) and `DrunkTransformText`
//! (`0x508680`), each run against `WoW.exe` in the emulator.
//!
//! Coverage of the arithmetic opcode set (load/store/compare/convert/`fild`/`fxch` excluded),
//! counted in each `[addr, true_ret)`:
//!
//! * `0x4fda20` GetTradeSkillCooldown [.,0x4fdadd ret]: `fmul dword [1/1000 f32]` ×1
//! * `0x507710` status-bar formatter  [.,0x50867b ret]:
//!     - `fdivr [1.0]` ×1 (the `/n` reciprocal scale)
//!     - `fmul st,st(1)` + `fmul [ebp+0x18]` ×2 (the M/O/S value pre-scale)
//!     - `fmul [ebp+0xc]` + `fdiv st,st(1)` ×2 + `fmul [ebp+0x18]` + `fdiv st,st(1)` (the o/O per-tick)
//!     - the round-half island: `fabs`×2, `fmul [local_c]`×2, `fadd st,st`×2, `fsub/fadd [0.5]`×2
//!     - the floor/ceil snap: `fsubr/fsub [slot]` ×6 (each after a CRT floor/ceil call)
//!     - the 'f' case: `fmul [field]` ×2, `fadd/fsub [0.5]` ×2
//! * `0x508680` garbled-chat filter  [.,0x508a53 ret]: `fsub [1.0]` ×3 + `fmul [0.25]` ×1
//! * `0x50e8d0` camera driver        [.,0x50f047 ret] (owned fade/rate islands):
//!     - `fmul [0.01]` (mood fade) ×1
//!     - `fmul [π/180]` (look rate) ×3
//!     - `fadd [local_8]` (pitch accum) ×1, `fadd [esi+0x104]` (local8 preadd) ×1
//!     - `fabs` (field-vs-ε) ×2
//!     - `fsub [cur]` + `fadd [1/9]` + `fadd [bias]` ×2 (overshoot recenter, 2 axes)
//!     - `fsub [d]` + `fsub [B]` + `fdivp` (ratio) + `fabs` + `fmul` + `fdivr [k]` (distance fade)
//!     - `fsub [field]` + `fdiv st,st(1)` (fade-alpha ratio)
//!     - `fsincos` (camera shake, transcendental → libm, structural diff)
//!
//! Diff strategy: every owned op is diffed via an **own-address slice** (`Emu::run(start,end)`) over
//! the exact source bytes halted at a basic-block boundary, feeding the inputs at the source stack
//! slots / fields and reading the stored results. The complex M/O/S round-half + floor/ceil snap runs
//! the binary's REAL bytes (incl. the CRT floor `0x73fdf5` / ceil `0x73ff3f` and `__ftol` `0x40a2b0`,
//! mapped `.text`) so it covers every embedded FP-arith op by construction. Each test carries a
//! deterministic `==threshold` boundary probe; fuzz reads the LCG HIGH bits. `.to_bits()`.
//!
//! Host double (`docs/METHOD.md#the-harness`): `glue4fd_50e8d0_shake_sincos` runs `fsincos` at
//! `0x50efe8`. The emulator computes each in host `double`, so where its result reaches a compared
//! value the host's math stands in for the x87's, and the test shows the code around it.
#![allow(clippy::float_cmp)]
#![allow(clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::camera_update::{
    ceil_frac, distance_fade, f_case_quantize, f_case_scale, fade_alpha_ratio, field_abs,
    garble_noise_minus_one, garble_threshold_quarter, local8_preadd, look_rate_rad, mood_fade,
    mos_prescale, mos_round_half, mos_snap, ot_per_tick, overshoot_recenter, pitch_accumulate,
    recip_scale, shake_sincos, tradeskill_cooldown_seconds, view_eye_offset, view_height_add,
    view_offset_accum,
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
    let scratch = emu.alloc(0x200);
    let ebp = scratch + 0x140; // room for [ebp-0x140 .. ebp+0x40]
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { ebp, esp }
}

fn f32_bits_eq(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

// ===========================================================================================
// 0x4fda20 GetTradeSkillCooldown — slice 0x4fdaad..0x4fdabe:
//   fild qword [ebp-0x14]; sub esp,8; fmul dword [0x801360]=1/1000(f32); fstp qword [esp]
//   (halt at the call). The cooldown remainder is written to [ebp-0x14] (lo dword) with [ebp-0x10]
//   (hi dword) forced 0 — the binary's `mov [ebp-0x10],0`. Read the pushed f64 at [esp-8].
// ===========================================================================================
#[test]
fn glue4fd_4fda20_tradeskill_cooldown() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, rem: i32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x14, rem as u32); // lo dword
        emu.write_u32(f.ebp - 0x10, 0); // hi dword forced 0 (the source `mov [ebp-0x10],0`)
        emu.run(0x4fdaad, 0x4fdabe);
        let b = emu.read(f.esp - 0x8, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    // boundary probes: 0, 1, the i32->u32 wrap pivot (-1 reads as 0xffffffff), large positives.
    let probes = [
        0i32,
        1,
        1000,
        60000,
        -1,
        -1000,
        i32::MIN,
        i32::MAX,
        0x7fff_ffff,
    ];
    for &r in &probes {
        assert_eq!(
            run(&mut emu, r).to_bits(),
            tradeskill_cooldown_seconds(r).to_bits(),
            "tradeskill_cooldown rem={r}"
        );
        total += 1;
    }
    let mut s = 0x4fda_2000u32;
    for _ in 0..3000 {
        let r = next(&mut s) as i32;
        assert_eq!(
            run(&mut emu, r).to_bits(),
            tradeskill_cooldown_seconds(r).to_bits(),
            "tradeskill_cooldown fuzz rem={r}"
        );
        total += 1;
    }
    eprintln!("glue4fd_4fda20_tradeskill_cooldown: {total} bit-exact (fild qword; fmul [1/1000 f32]; fstp qword)");
}

// ===========================================================================================
// 0x507710 `/n` reciprocal scale — slice 0x50777c..0x50778d:
//   fild dword [ebp-0xc]=n; cmp byte [ecx],'/'; (jne) fdivr dword [0x7ff9d8]=1.0; fstp dword [ebp-0x8]
// ecx must point at a byte == '/' so the fdivr runs. Read local_c at [ebp-0x8].
// ===========================================================================================
#[test]
fn glue4fd_507710_recip_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let slash = emu.alloc(8);
    emu.write(slash, &[b'/', 0, 0, 0]); // *ecx == '/' -> the fdivr fires

    let run = |emu: &mut Emu, n: i32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ECX, slash);
        emu.write_u32(f.ebp - 0xc, n as u32);
        emu.run(0x50777c, 0x50778d);
        emu.read_f32(f.ebp - 0x8)
    };

    let mut total = 0u32;
    let probes = [1i32, 2, 3, 5, 10, 100, 1000, -1, -2, 7, 255];
    for &n in &probes {
        assert!(
            f32_bits_eq(run(&mut emu, n), recip_scale(n)),
            "recip_scale n={n}"
        );
        total += 1;
    }
    let mut s = 0x5077_7c00u32;
    for _ in 0..3000 {
        let n = (next(&mut s) >> 8) as i32 % 100000 + 1; // parsed positive integer scale
        assert!(
            f32_bits_eq(run(&mut emu, n), recip_scale(n)),
            "recip_scale fuzz n={n}"
        );
        total += 1;
    }
    eprintln!(
        "glue4fd_507710_recip_scale: {total} bit-exact (fild dword; fdivr [1.0]; fstp dword)"
    );
}

// ===========================================================================================
// 0x507710 M/O/S value pre-scale — slice 0x5078d1..0x5078e7:
//   fild [ebp+0x10]=p5; fld [ebp+0xc]=p4; mov eax,[0xbe0b80]; fmul st,st(1); fstp [ebp+0xc];
//   fmul [ebp+0x18]=p7; fstp [ebp+0x18]
// (the mov eax,[global] reads a mapped .data global, harmless). Read p4'/p7' at [ebp+0xc]/[ebp+0x18].
// ===========================================================================================
#[test]
fn glue4fd_507710_mos_prescale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, p4: f32, p7: f32, p5: i32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x10, p5 as u32);
        emu.write_u32(f.ebp + 0xc, p4.to_bits());
        emu.write_u32(f.ebp + 0x18, p7.to_bits());
        emu.run(0x5078d1, 0x5078e7);
        (emu.read_f32(f.ebp + 0xc), emu.read_f32(f.ebp + 0x18))
    };

    let mut total = 0u32;
    let p5s = [0i32, 1, 2, 3, 10, -1, 64, 1000];
    let fs = [0.0f32, 1.0, 0.5, -2.5, 12.34, 1.0 / 3.0, 1e6, -1e6];
    for &p5 in &p5s {
        for &p4 in &fs {
            for &p7 in &fs {
                let (g4, g7) = run(&mut emu, p4, p7, p5);
                let (w4, w7) = mos_prescale(p4, p7, p5);
                assert!(
                    f32_bits_eq(g4, w4) && f32_bits_eq(g7, w7),
                    "mos_prescale p4={p4} p7={p7} p5={p5} got=({g4},{g7}) want=({w4},{w7})"
                );
                total += 1;
            }
        }
    }
    let mut s = 0x5078_d100u32;
    for _ in 0..3000 {
        let p4 = rngf(&mut s, -1e4, 1e4) as f32;
        let p7 = rngf(&mut s, -1e4, 1e4) as f32;
        let p5 = (next(&mut s) >> 8) as i32 % 2000 - 1000;
        let (g4, g7) = run(&mut emu, p4, p7, p5);
        let (w4, w7) = mos_prescale(p4, p7, p5);
        assert!(
            f32_bits_eq(g4, w4) && f32_bits_eq(g7, w7),
            "mos_prescale fuzz p4={p4} p7={p7} p5={p5}"
        );
        total += 1;
    }
    eprintln!(
        "glue4fd_507710_mos_prescale: {total} bit-exact (fild; fmul st,st(1); fmul [ebp+0x18])"
    );
}

// ===========================================================================================
// 0x507710 o/O per-tick divide — slice 0x507926..0x507942:
//   fild [ebp+0x14]=ticks; fild [ebp+0x8]=denom; fld st(1); fmul [ebp+0xc]=p4; fdiv st,st(1);
//   fstp [ebp+0xc]; fxch st(1); fmul [ebp+0x18]=p7; fdiv st,st(1); fstp [ebp+0x18]; fstp st(0)
// Read p4'/p7' at [ebp+0xc]/[ebp+0x18]. denom>0 in the runtime (param_3 or 5000); ticks>0.
// ===========================================================================================
#[test]
fn glue4fd_507710_ot_per_tick() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run = |emu: &mut Emu, p4: f32, p7: f32, ticks: i32, denom: i32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x14, ticks as u32);
        emu.write_u32(f.ebp + 0x8, denom as u32);
        emu.write_u32(f.ebp + 0xc, p4.to_bits());
        emu.write_u32(f.ebp + 0x18, p7.to_bits());
        emu.run(0x507926, 0x507942);
        (emu.read_f32(f.ebp + 0xc), emu.read_f32(f.ebp + 0x18))
    };

    let mut total = 0u32;
    let fs = [0.0f32, 1.0, 0.5, -2.5, 12.34, 1e3];
    let ts = [1i32, 2, 60, 1000];
    let ds = [1i32, 5, 5000, 60000];
    for &p4 in &fs {
        for &p7 in &fs {
            for &t in &ts {
                for &d in &ds {
                    let (g4, g7) = run(&mut emu, p4, p7, t, d);
                    let (w4, w7) = ot_per_tick(p4, p7, t, d);
                    assert!(
                        f32_bits_eq(g4, w4) && f32_bits_eq(g7, w7),
                        "ot_per_tick p4={p4} p7={p7} t={t} d={d}"
                    );
                    total += 1;
                }
            }
        }
    }
    let mut s = 0x5079_2600u32;
    for _ in 0..3000 {
        let p4 = rngf(&mut s, -1e3, 1e3) as f32;
        let p7 = rngf(&mut s, -1e3, 1e3) as f32;
        let t = (next(&mut s) >> 8) as i32 % 100000 + 1;
        let d = (next(&mut s) >> 8) as i32 % 100000 + 1;
        let (g4, g7) = run(&mut emu, p4, p7, t, d);
        let (w4, w7) = ot_per_tick(p4, p7, t, d);
        assert!(
            f32_bits_eq(g4, w4) && f32_bits_eq(g7, w7),
            "ot_per_tick fuzz p4={p4} p7={p7} t={t} d={d}"
        );
        total += 1;
    }
    eprintln!("glue4fd_507710_ot_per_tick: {total} bit-exact (fild; fmul; fdiv st,st(1) x2)");
}

// ===========================================================================================
// 0x507710 M/O/S round-half — slice 0x507952..0x5079da (halt at the ceil call 0x73ff3f):
//   fld [ebp+0xc]=p4; fabs; fmul [ebp-0x8]=local_c; fst [ebp+0x8];
//   fld [ebp+0x18]=p7; fabs; fmul [ebp-0x8]; fstp [ebp+0x14]; fcomp 0; (round bias); fistp [ebp+0x10];
//   fld [ebp+0x14]; fcomp 0; sar esi; (round bias); fistp [ebp-0xc]; sar edi; store [ebp-0x8]
// Reads p4 [ebp+0xc], p7 [ebp+0x18], local_c [ebp-0x8]. Writes: m_f32 for p4 [ebp+0x8], half_p4
// [ebp+0x10], m_f32 for p7 [ebp+0x14], half_p7 [ebp-0x8]. Covers fabs x2, fmul[local_c] x2,
// fadd st,st x2, fsub/fadd [0.5] x2, and the two fistp.
// ===========================================================================================
#[test]
fn glue4fd_507710_mos_round_half() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // returns (m_f32_p4, half_p4, m_f32_p7, half_p7)
    let run = |emu: &mut Emu, p4: f32, p7: f32, local_c: f32| -> (f32, i32, f32, i32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0xc, p4.to_bits());
        emu.write_u32(f.ebp + 0x18, p7.to_bits());
        emu.write_u32(f.ebp - 0x8, local_c.to_bits());
        emu.run(0x507952, 0x5079da);
        (
            emu.read_f32(f.ebp + 0x8),
            emu.read(f.ebp + 0x10, 4)
                .try_into()
                .map(i32::from_le_bytes)
                .unwrap(),
            emu.read_f32(f.ebp + 0x14),
            emu.read(f.ebp - 0x8, 4)
                .try_into()
                .map(i32::from_le_bytes)
                .unwrap(),
        )
    };

    let check = |emu: &mut Emu, total: &mut u32, p4: f32, p7: f32, lc: f32| {
        let (gm4, gh4, gm7, gh7) = run(emu, p4, p7, lc);
        let (wm4, wh4) = mos_round_half(p4, lc);
        let (wm7, wh7) = mos_round_half(p7, lc);
        assert!(
            f32_bits_eq(gm4, wm4) && gh4 == wh4,
            "round_half p4={p4} lc={lc}: got=({gm4},{gh4}) want=({wm4},{wh4})"
        );
        assert!(
            f32_bits_eq(gm7, wm7) && gh7 == wh7,
            "round_half p7={p7} lc={lc}: got=({gm7},{gh7}) want=({wm7},{wh7})"
        );
        *total += 2;
    };

    let mut total = 0u32;
    // deterministic ==threshold probes: integer/half-integer pivots of 2*m ± 0.5 (where the round
    // and the >>1 flip). local_c is 1.0 or 1/n; n from atoi may be NEGATIVE, so `m = |v|·local_c`
    // can be < 0 — the negative lc values cover the `fadd [0.5]` (m<0) branch (0x50798a/0x5079c0),
    // the positive ones the `fsub [0.5]` (m>=0) branch (0x50797a/0x5079b0).
    let lc_vals = [1.0f32, 0.5, 0.25, 1.0 / 3.0, 2.0, -1.0, -0.5, -2.0];
    let v_vals = [
        0.0f32,
        0.25,
        0.5,
        0.75,
        1.0,
        1.5,
        2.0,
        2.5,
        3.5,
        10.0,
        59.0,
        60.0,
        0.249_999_99,
        0.250_000_03,
        -1.5,
        -60.0,
        // NaN, the infinities and values whose doubled magnitude passes `i32` and `i64`: the
        // `fistp` stores `0x80000000`.
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        2.0e9,
        -2.0e9,
        1.0e10,
        1.0e30,
        -1.0e30,
    ];
    for &lc in &lc_vals {
        for &v in &v_vals {
            check(&mut emu, &mut total, v, v, lc);
        }
    }
    let mut s = 0x5079_5200u32;
    for _ in 0..4000 {
        let p4 = rngf(&mut s, 0.0, 600.0) as f32;
        let p7 = rngf(&mut s, 0.0, 600.0) as f32;
        // span both signs of local_c so both the fsub[0.5] (m>=0) and fadd[0.5] (m<0) branches fire.
        let lc = rngf(&mut s, -4.0, 4.0) as f32;
        check(&mut emu, &mut total, p4, p7, lc);
    }
    eprintln!("glue4fd_507710_mos_round_half: {total} bit-exact (fabs x2; fmul[local_c] x2; fadd st,st x2; fsub/fadd[0.5] x2; fistp x2)");
}

// ===========================================================================================
// 0x507710 'f'-case scale — slice 0x507ff7..0x50800a (halt AFTER the fstp qword at 0x508007):
//   fld [ebp-0x8]=local_c; sub esp,8; fmul dword [esi+4*edi+0x29c]=field; fstp qword [esp]
// We point [esi+4*edi+0x29c] at a scratch cell holding the f32 field; read the pushed f64 at [esp-8].
// ===========================================================================================
#[test]
fn glue4fd_507710_f_case_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let field_base = emu.alloc(0x10);
    // esi + 4*edi + 0x29c == field_base  =>  set esi=field_base-0x29c, edi=0.
    let esi = field_base - 0x29c;

    let run = |emu: &mut Emu, local_c: f32, field: f32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.write_u32(f.ebp - 0x8, local_c.to_bits());
        emu.write_u32(field_base, field.to_bits());
        emu.run(0x507ff7, 0x50800a); // halt AFTER fstp qword [esp] at 0x508007
        let b = emu.read(f.esp - 0x8, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    let lcs = [0.0f32, 1.0, 0.5, 1.0 / 3.0, -2.0, 0.25, 7.5];
    let fields = [0.0f32, 1.0, -1.0, 3.1234, 100.0, 1e-7, -1e9, 0.5, -0.5];
    for &lc in &lcs {
        for &fld in &fields {
            assert_eq!(
                run(&mut emu, lc, fld).to_bits(),
                f_case_scale(lc, fld).to_bits(),
                "f_case_scale lc={lc} field={fld}"
            );
            total += 1;
        }
    }
    let mut s = 0x507f_f700u32;
    for _ in 0..3000 {
        let lc = rngf(&mut s, -10.0, 10.0) as f32;
        let fld = rngf(&mut s, -1e6, 1e6) as f32;
        assert_eq!(
            run(&mut emu, lc, fld).to_bits(),
            f_case_scale(lc, fld).to_bits(),
            "f_case_scale fuzz lc={lc} field={fld}"
        );
        total += 1;
    }
    eprintln!(
        "glue4fd_507710_f_case_scale: {total} bit-exact (fld; fmul dword [field]; fstp qword)"
    );
}

// ===========================================================================================
// 0x507710 'f'-alt quantize — slice 0x508025..0x50804f (halt after the __ftol call returns):
//   fld [ebp-0x8]=local_c; fmul [esi+4*edi+0x29c]=field; fcom [0x7ffd74]=0; fnstsw; test ah,0x41;
//   (jne) fadd/fsub [0x7ffa24]=0.5; call __ftol (0x40a2b0). Read EAX (the quantized i32).
// ===========================================================================================
#[test]
fn glue4fd_507710_f_case_quantize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let field_base = emu.alloc(0x10);
    let esi = field_base - 0x29c;

    let run = |emu: &mut Emu, local_c: f32, field: f32| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.write_u32(f.ebp - 0x8, local_c.to_bits());
        emu.write_u32(field_base, field.to_bits());
        emu.run(0x508025, 0x50804f);
        emu.get_reg(RegisterX86::EAX) as i32
    };

    let mut total = 0u32;
    // ==threshold probes around q==0 (the fcom-vs-0 branch pivot) and the trunc pivots.
    let lcs = [1.0f32, 0.5, -1.0, 2.0];
    let fields = [
        0.0f32,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.499_999_97,
        1.5,
        2.5,
        -2.5,
        100.4,
        100.6,
        1e-7,
        -1e-7,
        // NaN, the infinities and products past `i32` and `i64`: `__ftol` returns the low dword
        // of its qword, 0 for the indefinite.
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        4.0e9,
        -4.0e9,
        1.0e19,
        1.0e30,
        -1.0e30,
    ];
    for &lc in &lcs {
        for &fld in &fields {
            assert_eq!(
                run(&mut emu, lc, fld),
                f_case_quantize(lc, fld),
                "f_case_quantize lc={lc} field={fld}"
            );
            total += 1;
        }
    }
    let mut s = 0x5080_2500u32;
    for _ in 0..3000 {
        let lc = rngf(&mut s, -4.0, 4.0) as f32;
        let fld = rngf(&mut s, -1000.0, 1000.0) as f32;
        assert_eq!(
            run(&mut emu, lc, fld),
            f_case_quantize(lc, fld),
            "f_case_quantize fuzz lc={lc} field={fld}"
        );
        total += 1;
    }
    eprintln!(
        "glue4fd_507710_f_case_quantize: {total} bit-exact (fmul[field]; fadd/fsub[0.5]; __ftol)"
    );
}

// ===========================================================================================
// 0x508680 garbled-chat noise-bias subtract — slice 0x50882a..0x50883f (halt at the fcomp):
//   fld dword [ebp-0x4]=local_8; (mov..) fsub dword [0x7ff9d8]=1.0; (mov..) -> st0 = local_8 - 1.0
// The slice ends just before `fcomp [ebp+0xc]`. local_8 is read at [ebp-0x4]; st0 isn't stored, so
// we append a `fstp qword [out]` via a COPIED-BYTE ISLAND of the source `fld;fsub` bytes instead.
// ===========================================================================================
#[test]
fn glue4fd_508680_garble_noise_minus_one() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // copied-byte island: fld qword [in]; <source d8 25 d8 f9 7f 00 = fsub dword [0x7ff9d8]>;
    // fstp qword [out]. The fsub bytes are byte-verified == all 3 source sites.
    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xddu8, 0x05]; // fld qword [in]
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let fsub_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x25, 0xd8, 0xf9, 0x7f, 0x00]); // fsub dword [0x7ff9d8]
    code.push(0xdd); // fstp qword [out]  = DD 1D <disp32>
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    for src in [0x50882fu64, 0x508993, 0x5089ee] {
        assert_eq!(
            emu.read(isl + fsub_off, 6),
            emu.read(src, 6),
            "garble fsub island != source @ {src:#x}"
        );
    }

    let run = |emu: &mut Emu, local_8: f32| -> f64 {
        // fld qword reads an f64; the source feeds an f32 reloaded into a register then re-stored
        // and `fld dword` reads it — so feed the f32 widened to f64.
        emu.write(in_cell, &f64::from(local_8).to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    // local_8 = (rand & 0x7fffff | 0x3f800000): exactly the [1.0, 2.0) IEEE range. Probe the
    // endpoints and a dense sweep of representable mantissas.
    let probes = [
        1.0f32,
        2.0,
        1.5,
        f32::from_bits(0x3f800001), // 1.0 + 1 ulp
        f32::from_bits(0x3fffffff), // just below 2.0
    ];
    for &v in &probes {
        assert_eq!(
            run(&mut emu, v).to_bits(),
            garble_noise_minus_one(v).to_bits(),
            "garble_noise v={v}"
        );
        total += 1;
    }
    let mut s = 0x5088_2a00u32;
    for _ in 0..4000 {
        // build the actual runtime float: mantissa bits | 0x3f800000.
        let bits = (next(&mut s) & 0x7fffff) | 0x3f800000;
        let v = f32::from_bits(bits);
        assert_eq!(
            run(&mut emu, v).to_bits(),
            garble_noise_minus_one(v).to_bits(),
            "garble_noise fuzz v={v} bits={bits:#x}"
        );
        total += 1;
    }
    eprintln!("glue4fd_508680_garble_noise_minus_one: {total} bit-exact (fsub [1.0] island == all 3 sites)");
}

// ===========================================================================================
// 0x508680 garbled-chat threshold scale — slice 0x5089f4..0x5089fd (halt at fcompp):
//   fld dword [ebp+0xc]=param_4; fmul dword [0x8029b0]=0.25 -> st0 = param_4 * 0.25
// st0 isn't stored before the fcompp, so use a COPIED-BYTE ISLAND of the source `fld;fmul` bytes.
// ===========================================================================================
#[test]
fn glue4fd_508680_garble_threshold_quarter() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    // source @0x5089f4: d9 45 0c = fld dword [ebp+0xc]; @0x5089f7: d8 0d b0 29 80 00 = fmul [0x8029b0]
    // We can't reuse [ebp+0xc] addressing cleanly in the island; rebuild as fld dword [in]; fmul
    // dword [0x8029b0] (byte-verify only the fmul, which carries the owned constant), fstp qword.
    let mut code = vec![0xd9u8, 0x05]; // fld dword [in]
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x0d, 0xb0, 0x29, 0x80, 0x00]); // fmul dword [0x8029b0]
    code.push(0xdd); // fstp qword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        emu.read(isl + fmul_off, 6),
        emu.read(0x5089f7, 6),
        "garble fmul island != source @ 0x5089f7"
    );

    let run = |emu: &mut Emu, p4: f32| -> f64 {
        emu.write(in_cell, &p4.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    let probes = [0.0f32, 1.0, 0.5, 0.25, 0.123, 1e-7, 100.0, -3.0];
    for &v in &probes {
        assert_eq!(
            run(&mut emu, v).to_bits(),
            garble_threshold_quarter(v).to_bits(),
            "garble_threshold v={v}"
        );
        total += 1;
    }
    let mut s = 0x5089_f400u32;
    for _ in 0..3000 {
        let v = rngf(&mut s, -10.0, 10.0) as f32;
        assert_eq!(
            run(&mut emu, v).to_bits(),
            garble_threshold_quarter(v).to_bits(),
            "garble_threshold fuzz v={v}"
        );
        total += 1;
    }
    eprintln!("glue4fd_508680_garble_threshold_quarter: {total} bit-exact (fmul [0.25])");
}

// ===========================================================================================
// 0x50e8d0 mood-fade — slice 0x50eb93..0x50eb9c (halt at the fcomp):
//   fild [ebp-0x14]=byte; fmul dword [0x8029d0]=0.01 -> st0 = byte * 0.01
// st0 isn't stored, so COPIED-BYTE ISLAND of the source fmul: fild dword [in]; <fmul [0x8029d0]>; fstp.
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_mood_fade() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xdbu8, 0x05]; // fild dword [in]
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x0d, 0xd0, 0x29, 0x80, 0x00]); // fmul dword [0x8029d0]
    code.push(0xdd); // fstp qword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        emu.read(isl + fmul_off, 6),
        emu.read(0x50eb96, 6),
        "mood_fade fmul island != source @ 0x50eb96"
    );

    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.write_u32(in_cell, u32::from(byte));
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    for byte in 0u32..=100 {
        assert_eq!(
            run(&mut emu, byte as u8).to_bits(),
            mood_fade(byte as u8).to_bits(),
            "mood_fade byte={byte}"
        );
        total += 1;
    }
    eprintln!("glue4fd_50e8d0_mood_fade: {total} bit-exact (fild; fmul [0.01])");
}

// ===========================================================================================
// 0x50e8d0 look-rate deg->rad — slice 0x50ecf2..0x50ecfe (the clean third site, no interleaved mov):
//   fld dword [eax+0x24]=rate; fmul dword [0x7ffaac]=π/180; fstp dword [ebp+0xc]
// The three sites (0x50ec18 / 0x50ec24 / 0x50ecf5) carry the byte-identical `fmul dword [0x7ffaac]`
// (clone inheritance, masked-eq trivially) so this one stands for all three. eax points at the rate
// field; read the radians at [ebp+0xc].
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_look_rate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let rate_obj = emu.alloc(0x40);
    let eax = rate_obj; // [eax+0x24] = rate field

    // byte-verify the three source fmul sites are identical (clone inheritance, masked-eq trivially).
    for src in [0x50ec18u64, 0x50ec24, 0x50ecf5] {
        assert_eq!(
            emu.read(0x50ec18, 6),
            emu.read(src, 6),
            "look_rate fmul source mismatch @ {src:#x}"
        );
    }

    let run = |emu: &mut Emu, rate: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EAX, eax);
        emu.write_u32(rate_obj + 0x24, rate.to_bits());
        emu.run(0x50ecf2, 0x50ecfe);
        emu.read_f32(f.ebp + 0xc)
    };

    let mut total = 0u32;
    let probes = [
        0.0f32, 1.0, -1.0, 90.0, 180.0, 360.0, 45.0, 0.5, -90.0, 1e-7, 1e6,
    ];
    for &r in &probes {
        assert!(
            f32_bits_eq(run(&mut emu, r), look_rate_rad(r)),
            "look_rate r={r}"
        );
        total += 1;
    }
    let mut s = 0x50ec_1800u32;
    for _ in 0..3000 {
        let r = rngf(&mut s, -720.0, 720.0) as f32;
        assert!(
            f32_bits_eq(run(&mut emu, r), look_rate_rad(r)),
            "look_rate fuzz r={r}"
        );
        total += 1;
    }
    eprintln!("glue4fd_50e8d0_look_rate: {total} bit-exact (fmul [π/180] x3 via clone)");
}

// ===========================================================================================
// 0x50e8d0 local8 preadd — slice 0x50eb52..0x50eb62:
//   fld [ebp-0x10]=local_14; (test) fadd dword [esi+0x104]=field; (mov) fstp dword [ebp-0x4]
// Read [ebp-0x4]. (the `test ebx,ebx` is harmless; we set ebx=0 to make the following je not matter
// — we halt before it.)
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_local8_preadd() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x200);
    let esi = obj;

    let run = |emu: &mut Emu, local_14: f32, field: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(f.ebp - 0x10, local_14.to_bits());
        emu.write_u32(obj + 0x104, field.to_bits());
        emu.run(0x50eb52, 0x50eb62);
        emu.read_f32(f.ebp - 0x4)
    };

    let mut total = 0u32;
    let fs = [0.0f32, 1.0, -1.0, 0.5, 12.34, -56.78, 1e-7, 1e6, -1e6];
    for &a in &fs {
        for &b in &fs {
            assert!(
                f32_bits_eq(run(&mut emu, a, b), local8_preadd(a, b)),
                "local8_preadd a={a} b={b}"
            );
            total += 1;
        }
    }
    let mut s = 0x50eb_5200u32;
    for _ in 0..3000 {
        let a = rngf(&mut s, -1e4, 1e4) as f32;
        let b = rngf(&mut s, -1e4, 1e4) as f32;
        assert!(
            f32_bits_eq(run(&mut emu, a, b), local8_preadd(a, b)),
            "local8_preadd fuzz a={a} b={b}"
        );
        total += 1;
    }
    eprintln!("glue4fd_50e8d0_local8_preadd: {total} bit-exact (fadd [esi+0x104])");
}

// ===========================================================================================
// 0x50e8d0 pitch accumulate — slice 0x50ed5e..0x50ed6e:
//   fld [ebp-0x30]=local_34; (mov) fadd dword [ebp-0x4]=local_8; (mov) fstp dword [ebp-0x30]
// Read [ebp-0x30].
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_pitch_accumulate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x200);

    let run = |emu: &mut Emu, local_34: f32, local_8: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.write_u32(f.ebp - 0x30, local_34.to_bits());
        emu.write_u32(f.ebp - 0x4, local_8.to_bits());
        emu.run(0x50ed5e, 0x50ed71); // halt AFTER the fstp [ebp-0x30] at 0x50ed6e
        emu.read_f32(f.ebp - 0x30)
    };

    let mut total = 0u32;
    let fs = [0.0f32, 1.0, -1.0, 0.5, 12.34, -56.78, 1e-7, 1e6];
    for &a in &fs {
        for &b in &fs {
            assert!(
                f32_bits_eq(run(&mut emu, a, b), pitch_accumulate(a, b)),
                "pitch_accumulate a={a} b={b}"
            );
            total += 1;
        }
    }
    let mut s = 0x50ed_5e00u32;
    for _ in 0..3000 {
        let a = rngf(&mut s, -1e4, 1e4) as f32;
        let b = rngf(&mut s, -1e4, 1e4) as f32;
        assert!(
            f32_bits_eq(run(&mut emu, a, b), pitch_accumulate(a, b)),
            "pitch_accumulate fuzz a={a} b={b}"
        );
        total += 1;
    }
    eprintln!("glue4fd_50e8d0_pitch_accumulate: {total} bit-exact (fadd [ebp-0x4])");
}

// ===========================================================================================
// 0x50e8d0 field-abs — COPIED-BYTE ISLAND of the source `fabs` (d9 e1) @ 0x50ebdb / 0x50ee38:
//   fld qword [in]; <source d9 e1>; fstp qword [out]
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_field_abs() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xddu8, 0x05]; // fld qword [in]
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    let fabs_off = code.len() as u64;
    code.extend_from_slice(&[0xd9, 0xe1]); // fabs
    code.push(0xdd); // fstp qword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    for src in [0x50ebdbu64, 0x50ee38] {
        assert_eq!(
            emu.read(isl + fabs_off, 2),
            emu.read(src, 2),
            "field_abs fabs island != source @ {src:#x}"
        );
    }

    let run = |emu: &mut Emu, v: f32| -> f64 {
        // the source operand is an f32 field widened to f64 by `fld dword`; feed f32 widened.
        emu.write(in_cell, &f64::from(v).to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    let probes = [0.0f32, -0.0, 1.0, -1.0, 1e-7, -1e-7, 1e9, -1e9, 0.5, -0.5];
    for &v in &probes {
        assert_eq!(
            run(&mut emu, v).to_bits(),
            field_abs(v).to_bits(),
            "field_abs v={v}"
        );
        total += 1;
    }
    let mut s = 0x50eb_db00u32;
    for _ in 0..3000 {
        let v = rngf(&mut s, -1e6, 1e6) as f32;
        assert_eq!(
            run(&mut emu, v).to_bits(),
            field_abs(v).to_bits(),
            "field_abs fuzz v={v}"
        );
        total += 1;
    }
    eprintln!("glue4fd_50e8d0_field_abs: {total} bit-exact (fabs island == both sites)");
}

// ===========================================================================================
// 0x50e8d0 overshoot-recenter (yaw axis) — slice 0x50ee5d..0x50eeb0:
//   fld [esi+0xec]=target; fsub [ebp+0xc]=cur; fcomp [0x808a00]=1/9; test ah,0x41; jne 0x50eeb0;
//   fld [ebp+0xc]; fadd [0x808a00]; fadd [0x8026bc]; fst [esi+0xec]; fstp [esi+0x19c]
// We halt at 0x50eeb0 (the join). If `over`, [esi+0xec] is rewritten to the recentered value; else
// [esi+0xec] is unchanged (== target). So read [esi+0xec] to get the recenter result. We also assert
// the `over` flag by comparing the rewritten-or-not value. NB the branch sets ecx=0x40000000 first
// (a velocity field write at [esi+0x194]); irrelevant to the FP read.
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_overshoot_recenter() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x300);
    let esi = obj;

    // returns the value at [esi+0xec] after the slice (recentered if over, else target unchanged).
    let run = |emu: &mut Emu, target: f32, cur: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(obj + 0xec, target.to_bits());
        emu.write_u32(f.ebp + 0xc, cur.to_bits());
        emu.run(0x50ee5d, 0x50eeb0);
        emu.read_f32(obj + 0xec)
    };

    let check = |emu: &mut Emu, total: &mut u32, target: f32, cur: f32| {
        let got = run(emu, target, cur);
        let (over, rc) = overshoot_recenter(target, cur);
        let want = if over { rc } else { target };
        assert!(
            f32_bits_eq(got, want),
            "overshoot_recenter target={target} cur={cur} over={over}: got={got} want={want}"
        );
        *total += 1;
    };

    let mut total = 0u32;
    // ==threshold probe at target-cur == 1/9 exactly (the strict-> boundary).
    let ninth = f32::from_bits(0x3de3_8e39);
    check(&mut emu, &mut total, ninth, 0.0); // diff == 1/9 -> NOT over (strict)
    check(&mut emu, &mut total, ninth * 2.0, ninth); // diff == 1/9 -> NOT over
    check(&mut emu, &mut total, 0.5, 0.0); // diff > 1/9 -> over
    check(&mut emu, &mut total, 0.0, 0.5); // diff < 0 -> not over
    let pairs = [
        (0.0f32, 0.0),
        (1.0, 0.0),
        (10.0, 0.0),
        (0.1, 0.0),
        (0.12, 0.0),
        (-1.0, 0.0),
        (5.0, 4.5),
        (3.1234, 1.0),
    ];
    for &(t, c) in &pairs {
        check(&mut emu, &mut total, t, c);
    }
    let mut s = 0x50ee_5d00u32;
    for _ in 0..4000 {
        let t = rngf(&mut s, -20.0, 20.0) as f32;
        let c = rngf(&mut s, -20.0, 20.0) as f32;
        check(&mut emu, &mut total, t, c);
    }
    eprintln!(
        "glue4fd_50e8d0_overshoot_recenter: {total} bit-exact (fsub[cur]; fadd[1/9]; fadd[bias])"
    );
}

// ===========================================================================================
// 0x50e8d0 overshoot-recenter (PITCH axis) — slice 0x50eeb0..0x50eefe:
//   fld [esi+0xfc]=target; fsub [ebp-0x4]=cur; fcomp [0x808a04]=1/9; test ah,0x41; jne 0x50eefe;
//   fld [ebp-0x4]; fadd [0x808a04]; fadd [0x8026bc]; fst [esi+0xfc]; fstp [esi+0x1cc]
// Arithmetically identical to the yaw axis above — `0x808a04` holds the SAME 1/9 (`0x3de38e39`) as
// `0x808a00` (objdump-verified) and the same bias `0x8026bc` — so it reuses `overshoot_recenter`;
// only the object field ([esi+0xfc] vs +0xec) and the cur slot ([ebp-0x4] vs +0xc) differ. Halt at
// the join 0x50eefe; read [esi+0xfc] (recentered if over, else target unchanged). Covers the pitch
// fsub @0x50eeb6 and the two fadds @0x50eecf (1/9) / @0x50eee6 (bias).
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_overshoot_recenter_pitch() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x300);
    let esi = obj;

    // byte-verify the slice's owned FP ops are the operations we diff (the same fsub/fadd-dword ops
    // the yaw axis uses; here we self-gate the three pitch sites explicitly).
    assert_eq!(emu.read(0x50eeb6, 1), [0xd8], "pitch fsub @0x50eeb6 opcode");
    assert_eq!(
        emu.read(0x50eeb6, 2)[1] & 0x38,
        0x20,
        "pitch fsub @0x50eeb6 reg /4"
    );
    assert_eq!(emu.read(0x50eecf, 1), [0xd8], "pitch fadd @0x50eecf opcode");
    assert_eq!(
        emu.read(0x50eecf, 2)[1] & 0x38,
        0x00,
        "pitch fadd @0x50eecf reg /0"
    );
    assert_eq!(emu.read(0x50eee6, 1), [0xd8], "pitch fadd @0x50eee6 opcode");
    assert_eq!(
        emu.read(0x50eee6, 2)[1] & 0x38,
        0x00,
        "pitch fadd @0x50eee6 reg /0"
    );

    let run = |emu: &mut Emu, target: f32, cur: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(obj + 0xfc, target.to_bits());
        emu.write_u32(f.ebp - 0x4, cur.to_bits());
        emu.run(0x50eeb0, 0x50eefe);
        emu.read_f32(obj + 0xfc)
    };

    let check = |emu: &mut Emu, total: &mut u32, target: f32, cur: f32| {
        let got = run(emu, target, cur);
        let (over, rc) = overshoot_recenter(target, cur);
        let want = if over { rc } else { target };
        assert!(
            f32_bits_eq(got, want),
            "overshoot_recenter_pitch target={target} cur={cur} over={over}: got={got} want={want}"
        );
        *total += 1;
    };

    let mut total = 0u32;
    // ==threshold probe at target-cur == 1/9 exactly (the strict-> boundary), mirroring yaw.
    let ninth = f32::from_bits(0x3de3_8e39);
    check(&mut emu, &mut total, ninth, 0.0); // diff == 1/9 -> NOT over (strict)
    check(&mut emu, &mut total, ninth * 2.0, ninth); // diff == 1/9 -> NOT over
    check(&mut emu, &mut total, 0.5, 0.0); // diff > 1/9 -> over
    check(&mut emu, &mut total, 0.0, 0.5); // diff < 0 -> not over
    let pairs = [
        (0.0f32, 0.0),
        (1.0, 0.0),
        (10.0, 0.0),
        (0.1, 0.0),
        (0.12, 0.0),
        (-1.0, 0.0),
        (5.0, 4.5),
        (3.1234, 1.0),
    ];
    for &(t, c) in &pairs {
        check(&mut emu, &mut total, t, c);
    }
    let mut s = 0x50ee_b000u32;
    for _ in 0..4000 {
        let t = rngf(&mut s, -20.0, 20.0) as f32;
        let c = rngf(&mut s, -20.0, 20.0) as f32;
        check(&mut emu, &mut total, t, c);
    }
    eprintln!(
        "glue4fd_50e8d0_overshoot_recenter_pitch: {total} bit-exact (pitch fsub[cur]; fadd[1/9]; fadd[bias])"
    );
}

// ===========================================================================================
// 0x50e8d0 distance-fade — slice 0x50ef27..0x50ef61 (halt at the join):
//   fld [0x808a94]=A; fsub [esi+0xf4]=d; fld [0x808a94]; fsub [0x808a98]=B; fdivp st(1),st;
//   fst [ebp-0x4]; fabs; fcomp [0x8029d4]=ε; test ah,5; jnp 0x50ef61;
//   fstp st(0); fld [ebp-0x4]; fmul [ebp-0x4]; fdivr [0x8089b0]=k; (st0=fade left live -> consumed
//   downstream). The slice ends at 0x50ef61. We read r_f32 at [ebp-0x4] always; and to capture `fade`
//   (left live on st0 when over), we cannot read it from memory — so we diff the ratio r_f32 here and
//   diff the `k/(r*r)` window via a separate COPIED-BYTE ISLAND below.
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_distance_fade_ratio() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x200);
    let esi = obj;

    // The fabs @0x50ef44 (on the LIVE f64 ratio, for the `|r| >= ε` test) is the byte-identical
    // `d9 e1` of the field_abs sites — clone inheritance (the magnitude semantics are diffed by
    // `glue4fd_50e8d0_field_abs`; here we byte-verify the op is the same `fabs`).
    assert_eq!(
        emu.read(0x50ef44, 2),
        emu.read(0x50ebdb, 2),
        "distance_fade fabs @0x50ef44 != field_abs fabs @0x50ebdb"
    );

    // slice 0x50ef27..0x50ef44 (the ratio + fst, halt at the fabs): read r_f32 at [ebp-0x4].
    let run_ratio = |emu: &mut Emu, d: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(obj + 0xf4, d.to_bits());
        emu.run(0x50ef27, 0x50ef44); // up to (not incl) the fabs
        emu.read_f32(f.ebp - 0x4)
    };

    let mut total = 0u32;
    let probes = [
        0.0f32,
        1.0,
        -1.0,
        -std::f32::consts::FRAC_PI_2, // -π/2 = the FADE_A anchor (d makes the ratio numerator 0)
        -1.1868,                      // ≈ the FADE_B anchor (ratio denominator 0 region)
        0.5,
        100.0,
        -100.0,
        1e-7,
    ];
    for &d in &probes {
        let g = run_ratio(&mut emu, d);
        let (w, _) = distance_fade(d);
        assert!(
            f32_bits_eq(g, w),
            "distance_fade ratio d={d}: got={g} want={w}"
        );
        total += 1;
    }
    let mut s = 0x50ef_2700u32;
    for _ in 0..3000 {
        let d = rngf(&mut s, -10.0, 10.0) as f32;
        let g = run_ratio(&mut emu, d);
        let (w, _) = distance_fade(d);
        assert!(f32_bits_eq(g, w), "distance_fade ratio fuzz d={d}");
        total += 1;
    }

    // The window `k / (r_f32 * r_f32)` via a COPIED-BYTE ISLAND of the source square+fdivr bytes
    // (0x50ef55: d9 45 fc = fld [ebp-0x4]; 0x50ef58: d8 4d fc = fmul [ebp-0x4];
    //  0x50ef5b: d8 3d b0 89 80 00 = fdivr [0x8089b0]). Seed [ebp-0x4] = r_f32, read st0 via fstp.
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    // fld dword [ebp-0x4]; fmul dword [ebp-0x4]; fdivr dword [0x8089b0]; fstp qword [out].
    // To make [ebp-0x4] addressable in the island we keep the same EBP-relative form and set EBP.
    let mut code = Vec::new();
    let fld_off = code.len() as u64;
    code.extend_from_slice(&[0xd9, 0x45, 0xfc]); // fld dword [ebp-0x4]
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x4d, 0xfc]); // fmul dword [ebp-0x4]
    let fdivr_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x3d, 0xb0, 0x89, 0x80, 0x00]); // fdivr dword [0x8089b0]
    code.push(0xdd); // fstp qword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        emu.read(isl + fld_off, 3),
        emu.read(0x50ef55, 3),
        "window fld"
    );
    assert_eq!(
        emu.read(isl + fmul_off, 3),
        emu.read(0x50ef58, 3),
        "window fmul"
    );
    assert_eq!(
        emu.read(isl + fdivr_off, 6),
        emu.read(0x50ef5b, 6),
        "window fdivr"
    );

    let run_window = |emu: &mut Emu, r_f32: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.write_u32(f.ebp - 0x4, r_f32.to_bits());
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap())) as f32
    };

    // diff distance_fade end-to-end where the window runs (|r| >= ε).
    for &d in &probes {
        let (r_f32, fade_opt) = distance_fade(d);
        if let Some(fade) = fade_opt {
            let g = run_window(&mut emu, r_f32);
            assert!(
                f32_bits_eq(g, fade),
                "distance_fade window d={d} r={r_f32}: got={g} want={fade}"
            );
            total += 1;
        }
    }
    let mut s = 0x50ef_5500u32;
    for _ in 0..3000 {
        let d = rngf(&mut s, -10.0, 10.0) as f32;
        let (r_f32, fade_opt) = distance_fade(d);
        if let Some(fade) = fade_opt {
            let g = run_window(&mut emu, r_f32);
            assert!(f32_bits_eq(g, fade), "distance_fade window fuzz d={d}");
            total += 1;
        }
    }
    eprintln!("glue4fd_50e8d0_distance_fade: {total} bit-exact (ratio fsub/fsub/fdivp + window fmul/fdivr[k])");
}

// ===========================================================================================
// 0x50e8d0 fade-alpha ratio — slice 0x50ef61..0x50ef6a (the subtract + fst, halt at fcomp st(1)):
//   fld [ebp+0xc]=param_3; fsub [esi+0x38]=field; fst [ebp+0xc] -> param_3' stored, st0 live.
// Read param_3' at [ebp+0xc]. Then the divide `param_3' / fade` via a COPIED-BYTE ISLAND of
// 0x50ef96 (fld [ebp+0xc]) + 0x50ef9a (fdiv st,st(1)) with fade pre-loaded on st(1).
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_fade_alpha_ratio() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x100);
    let esi = obj;

    let run_sub = |emu: &mut Emu, param_3: f32, field: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(f.ebp + 0xc, param_3.to_bits());
        emu.write_u32(obj + 0x38, field.to_bits());
        emu.run(0x50ef61, 0x50ef6a);
        emu.read_f32(f.ebp + 0xc)
    };

    // divide island: fld dword [fade]; fld dword [p3]; fdiv st,st(1); fstp qword [out].
    // (mirrors the source: fade is on st(1), p3 reloaded on st0, fdiv st,st(1) = p3/fade.)
    let fade_cell = emu.alloc(8);
    let p3_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x20);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [fade]  (st0=fade)
    code.extend_from_slice(&(fade_cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0x05]); // fld dword [p3]  (st0=p3, st1=fade)
    code.extend_from_slice(&(p3_cell as u32).to_le_bytes());
    let fdiv_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0xf1]); // fdiv st,st(1)  => p3/fade
    code.push(0xdd); // fstp qword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    assert_eq!(
        emu.read(isl + fdiv_off, 2),
        emu.read(0x50ef9a, 2),
        "fade fdiv island != source @ 0x50ef9a"
    );

    let run_div = |emu: &mut Emu, p3: f32, fade: f32| -> f64 {
        emu.write_u32(fade_cell, fade.to_bits());
        emu.write_u32(p3_cell, p3.to_bits());
        emu.run(isl, isl_end);
        let b = emu.read(out_cell, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    let p3s = [0.0f32, 1.0, -1.0, 5.0, 7.0, 0.5, 100.0];
    let fields = [0.0f32, 1.0, -1.0, 0.5, 2.0];
    let fades = [1.0f32, 1.8315, 0.5, 2.0, 7.0, 0.1];
    for &p3 in &p3s {
        for &fld in &fields {
            let g = run_sub(&mut emu, p3, fld);
            let (w, _) = fade_alpha_ratio(p3, fld, 1.0);
            assert!(f32_bits_eq(g, w), "fade subtract p3={p3} field={fld}");
            total += 1;
        }
    }
    for &p3 in &p3s {
        for &fld in &fields {
            for &fade in &fades {
                let p3p = fade_alpha_ratio(p3, fld, fade).0;
                let g = run_div(&mut emu, p3p, fade);
                let (_, w) = fade_alpha_ratio(p3, fld, fade);
                assert_eq!(
                    g.to_bits(),
                    w.to_bits(),
                    "fade divide p3={p3} field={fld} fade={fade}"
                );
                total += 1;
            }
        }
    }
    let mut s = 0x50ef_6100u32;
    for _ in 0..3000 {
        let p3 = rngf(&mut s, -50.0, 50.0) as f32;
        let fld = rngf(&mut s, -10.0, 10.0) as f32;
        let fade = rngf(&mut s, 0.01, 10.0) as f32;
        let g = run_sub(&mut emu, p3, fld);
        let (w, _) = fade_alpha_ratio(p3, fld, fade);
        assert!(f32_bits_eq(g, w), "fade subtract fuzz p3={p3} field={fld}");
        total += 1;
        let p3p = fade_alpha_ratio(p3, fld, fade).0;
        let gd = run_div(&mut emu, p3p, fade);
        let (_, wd) = fade_alpha_ratio(p3, fld, fade);
        assert_eq!(
            gd.to_bits(),
            wd.to_bits(),
            "fade divide fuzz p3={p3} field={fld} fade={fade}"
        );
        total += 1;
    }
    eprintln!("glue4fd_50e8d0_fade_alpha_ratio: {total} bit-exact (fsub[field]; fdiv st,st(1))");
}

// ===========================================================================================
// 0x50e8d0 camera-shake fsincos — slice 0x50efe5..0x50efee:
//   fld dword [ebp+0xc]=angle; fsincos; fstp dword [eax]=cos; fstp dword [edx]=sin
// eax/edx point at two scratch cells. Transcendental -> libm, diffed STRUCTURALLY.
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_shake_sincos() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let cos_cell = emu.alloc(8);
    let sin_cell = emu.alloc(8);

    let run = |emu: &mut Emu, angle: f32| -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EAX, cos_cell);
        emu.set_reg(RegisterX86::EDX, sin_cell);
        emu.write_u32(f.ebp + 0xc, angle.to_bits());
        emu.run(0x50efe5, 0x50efee);
        (emu.read_f32(cos_cell), emu.read_f32(sin_cell))
    };

    let mut total = 0u32;
    let probes = [
        0.0f32,
        1.0,
        -1.0,
        std::f32::consts::PI,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::FRAC_PI_4,
        0.5,
        -0.5,
        3.0,
    ];
    for &a in &probes {
        let (gc, gs) = run(&mut emu, a);
        let (wc, ws) = shake_sincos(a);
        assert!(
            f32_bits_eq(gc, wc) && f32_bits_eq(gs, ws),
            "shake_sincos a={a}: got=({gc},{gs}) want=({wc},{ws})"
        );
        total += 1;
    }
    let mut s = 0x50ef_e500u32;
    for _ in 0..3000 {
        let a = rngf(&mut s, -6.3, 6.3) as f32;
        let (gc, gs) = run(&mut emu, a);
        let (wc, ws) = shake_sincos(a);
        assert!(
            f32_bits_eq(gc, wc) && f32_bits_eq(gs, ws),
            "shake_sincos fuzz a={a}"
        );
        total += 1;
    }
    eprintln!("glue4fd_50e8d0_shake_sincos: {total} bit-exact STRUCTURAL (fsincos -> libm)");
}

// ===========================================================================================
// 0x507710 M/O/S floor/ceil integer-snap — covers the 6 `fsubr/fsub [slot]` ops the round-half
// slice stops before. Two binary slices run the REAL bytes (incl. the CRT floor 0x73ff3f / ceil
// 0x73fdf5 / __ftol 0x40a2b0):
//   p4 axis: slice 0x5079cc..0x507a5b — set [ebp+0x8]=m_f32, esi=half, [ebp+0x10]=half; read
//            ebx (bVar1) and esi (corrected half).
//   p7 axis: slice 0x507a5b..0x507aa2 — set [ebp+0x14]=m_f32, edi=half, [ebp-0x8]=half; read
//            esi (bVar2) and edi (corrected half). (Halt before the global [0xbe0b80] read at
//            0x507aa2; ebx must be preset since the p7 path doesn't touch it.)
// fsubr @0x5079df/0x507a69; fsub @0x507a00/0x507a44/0x507a8a; (the p7 fsub mirror @0x507a8a covers
// the p7 `ceil-m`, and the p4 has fsub @0x507a00 AND @0x507a44 — all 6 reached across the cases).
// ===========================================================================================
#[test]
fn glue4fd_507710_mos_snap() {
    let Some(wow) = Wow::load() else {
        return;
    };
    // The snap region embeds the CRT floor/ceil/__ftol `call`s and reconverges via several `jmp`s;
    // re-entering it at 0x5079cc across cases on a REUSED emu trips Unicorn's stale-TB FETCH/READ-
    // UNMAPPED (multi-exit-slice hazard). So this region uses a FRESH `Emu` per call (≈1ms),
    // the sanctioned fix — each axis runs its slice on a clean engine.

    // p4 axis: returns (bVar1, corrected_half_p4). slice 0x5079cc..0x507a5b.
    let run_p4 = |m_f32: f32, half: i32| -> (bool, i32) {
        let mut emu = Emu::new(&wow);
        let f = frame(&mut emu);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, half as u32 as u64); // esi = half_p4 entering the snap
        emu.set_reg(RegisterX86::EDI, 0); // edi (half_p7 housekeeping) — irrelevant to p4 out
        emu.write_u32(f.ebp + 0x8, m_f32.to_bits()); // [ebp+0x8] = m_f32 (fVar10 slot)
        emu.write_u32(f.ebp + 0x10, half as u32); // [ebp+0x10] = half_p4 slot
        emu.write_u32(f.ebp - 0xc, 0); // [ebp-0xc] (read at 0x5079c9 into edi); irrelevant
        emu.run(0x5079cc, 0x507a5b);
        let ebx = emu.get_reg(RegisterX86::EBX) as u32;
        let esi = emu.get_reg(RegisterX86::ESI) as u32 as i32;
        (ebx != 0, esi)
    };

    // p7 axis: returns (bVar2, corrected_half_p7). slice 0x507a5b..0x507aa2.
    let run_p7 = |m_f32: f32, half: i32| -> (bool, i32) {
        let mut emu = Emu::new(&wow);
        let f = frame(&mut emu);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EBX, 0); // bVar1 preset (p7 path leaves it untouched)
        emu.set_reg(RegisterX86::EDI, half as u32 as u64); // edi = half_p7 entering the snap
        emu.write_u32(f.ebp + 0x14, m_f32.to_bits()); // [ebp+0x14] = m_f32 (param_6 slot)
        emu.write_u32(f.ebp - 0x8, half as u32); // [ebp-0x8] = half_p7 slot
        emu.run(0x507a5b, 0x507aa2);
        let esi = emu.get_reg(RegisterX86::ESI) as u32; // bVar2 (xor esi,esi=false; set=true)
        let edi = emu.get_reg(RegisterX86::EDI) as u32 as i32;
        (esi != 0, edi)
    };

    let check = |total: &mut u32, m: f32, half: i32| {
        let (gb4, gh4) = run_p4(m, half);
        let (wb4, wh4) = mos_snap(m, half);
        assert!(
            gb4 == wb4 && gh4 == wh4,
            "mos_snap(p4) m={m} half={half}: got=({gb4},{gh4}) want=({wb4},{wh4})"
        );
        let (gb7, gh7) = run_p7(m, half);
        let (wb7, wh7) = mos_snap(m, half);
        assert!(
            gb7 == wb7 && gh7 == wh7,
            "mos_snap(p7) m={m} half={half}: got=({gb7},{gh7}) want=({wb7},{wh7})"
        );
        *total += 2;
    };

    let mut total = 0u32;
    // ==threshold probes: m exactly integral (frac_gap==0), m at 1e-3 below/above an integer (the
    // is_int boundary), and half mismatching ftol(ceil(m)) to fire the +1 bump.
    let eps = f32::from_bits(0x3a83_126f); // 1e-3
    let probes: &[(f32, i32)] = &[
        (0.0, 0),
        (1.0, 0),
        (1.0, 1),
        (2.0, 1),
        (2.0, 2),
        (5.0, 2),
        (5.0, 4),
        (5.0, 5),
        (3.0 - eps, 1),
        (3.0 - eps, 2),
        (3.0 + eps, 1),
        (10.0, 4),
        (10.0, 5),
        (59.0, 29),
        (60.0, 30),
        (0.5, 0),
        (1.5, 0),
        (1.5, 1),
        (2.5, 1),
        // Integral magnitudes past `i32` and `i64`: `__ftol(ceil(m))` keeps the low dword of its
        // qword, 0 past `i64`. NaN and the infinities do not reach it: the C runtime's `floor`
        // and `ceil` take an error path the emulator does not map.
        (3.0e9, 7),
        (4_294_967_296.0, 0),
        (4_294_967_296.0, 7),
        (1.0e10, 7),
        (1.0e19, 0),
        (1.0e30, 0),
        (1.0e30, 7),
    ];
    for &(m, half) in probes {
        check(&mut total, m, half);
    }
    let mut s = 0x5079_cc00u32;
    for _ in 0..1500 {
        // m is a magnitude (|v|*local_c >= 0); half is roughly m/2 but probe the whole nearby range.
        let m = rngf(&mut s, 0.0, 600.0) as f32;
        let half = (next(&mut s) >> 8) as i32 % 320;
        check(&mut total, m, half);
    }
    eprintln!(
        "glue4fd_507710_mos_snap: {total} bit-exact (fsubr x2 + fsub x4 across floor/ceil snap)"
    );
}

// ===========================================================================================
// 0x50e8d0 camera eye-offset vector composition (owned scalar·delegated-vector math).
//
// (a) height pre-add — slice 0x50ea8d..0x50eaa7 (halt after fstp [ebp-0x54] at 0x50eaa4):
//     fld [ebp-0x30]=local_34; fadd dword [esi+0xfc]=field; fstp [ebp-0x54]. Read [ebp-0x54].
// (b) eye-offset — slice 0x50eabc..0x50eafc (halt at the FUN_00511760 call): fVar1·v then base−scaled.
//     eax->view vec; esi+0xec=fVar1; [ebp-0x50/-0x4c/-0x48]=base xyz. Read p1[ebp-0x58], p2[ebp-0x54],
//     out_x[ebp-0x50], out_y[ebp-0x4c], out_z[ebp-0x48].
// (c) offset accumulate — slice 0x50eb0f..0x50eb3e (after the 3rd fstp at 0x50eb3b): edi->p1[0x110]=
//     the +0x44 vector; [ebp-0x44/-0x40/-0x3c]=acc xyz. Read the three back.
// Covers: 50ea93 fadd; 50eac7/50eacf/50eada fmul; 50eae3/50eaed/50eaf6 fsub; 50eb12/50eb22/50eb2b fadd.
// ===========================================================================================
#[test]
fn glue4fd_50e8d0_view_compose() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let obj = emu.alloc(0x200); // esi base (p1[0x3b]=+0xec, p1[0x3f]=+0xfc)
    let vec = emu.alloc(0x20); // eax -> view vector v0,v1,v2
    let acc_obj = emu.alloc(0x200); // edi base for the accumulate (+0..+8)

    // (a) height pre-add
    let run_a = |emu: &mut Emu, local_34: f32, field_fc: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.write_u32(f.ebp - 0x30, local_34.to_bits());
        emu.write_u32(obj + 0xfc, field_fc.to_bits());
        emu.run(0x50ea8d, 0x50eaa7);
        emu.read_f32(f.ebp - 0x54)
    };
    // (b) eye-offset compose
    let run_b =
        |emu: &mut Emu, fvar1: f32, v: [f32; 3], base: [f32; 3]| -> (f32, f32, f32, f32, f32) {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.set_reg(RegisterX86::ESI, obj);
            emu.set_reg(RegisterX86::EAX, vec);
            emu.write_u32(obj + 0xec, fvar1.to_bits());
            for (i, &vi) in v.iter().enumerate() {
                emu.write_u32(vec + 4 * i as u64, vi.to_bits());
            }
            emu.write_u32(f.ebp - 0x50, base[0].to_bits());
            emu.write_u32(f.ebp - 0x4c, base[1].to_bits());
            emu.write_u32(f.ebp - 0x48, base[2].to_bits());
            emu.run(0x50eabc, 0x50eafc);
            (
                emu.read_f32(f.ebp - 0x58),
                emu.read_f32(f.ebp - 0x54),
                emu.read_f32(f.ebp - 0x50),
                emu.read_f32(f.ebp - 0x4c),
                emu.read_f32(f.ebp - 0x48),
            )
        };
    // (c) offset accumulate
    let run_c = |emu: &mut Emu, acc: [f32; 3], field: [f32; 3]| -> (f32, f32, f32) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, obj); // [esi+0x90] is read into eax (harmless)
        emu.set_reg(RegisterX86::EDI, acc_obj);
        emu.write_u32(f.ebp - 0x44, acc[0].to_bits());
        emu.write_u32(f.ebp - 0x40, acc[1].to_bits());
        emu.write_u32(f.ebp - 0x3c, acc[2].to_bits());
        for (i, &fi) in field.iter().enumerate() {
            emu.write_u32(acc_obj + 4 * i as u64, fi.to_bits());
        }
        emu.run(0x50eb0f, 0x50eb3e);
        (
            emu.read_f32(f.ebp - 0x44),
            emu.read_f32(f.ebp - 0x40),
            emu.read_f32(f.ebp - 0x3c),
        )
    };

    let mut total = 0u32;
    let fs = [0.0f32, 1.0, -1.0, 0.5, 12.34, -56.78, 1e-7, 1e3];
    // (a)
    for &a in &fs {
        for &b in &fs {
            assert!(
                f32_bits_eq(run_a(&mut emu, a, b), view_height_add(a, b)),
                "view_height_add a={a} b={b}"
            );
            total += 1;
        }
    }
    // (b) — exercise the LIVE-f64 x-axis vs reloaded-f32 y/z distinction with a dense fuzz.
    let mut s = 0x50ea_bc00u32;
    for _ in 0..2500 {
        let fvar1 = rngf(&mut s, -100.0, 100.0) as f32;
        let v = [
            rngf(&mut s, -2.0, 2.0) as f32,
            rngf(&mut s, -2.0, 2.0) as f32,
            rngf(&mut s, -2.0, 2.0) as f32,
        ];
        let base = [
            rngf(&mut s, -500.0, 500.0) as f32,
            rngf(&mut s, -500.0, 500.0) as f32,
            rngf(&mut s, -500.0, 500.0) as f32,
        ];
        let g = run_b(&mut emu, fvar1, v, base);
        let w = view_eye_offset(fvar1, v, base);
        assert!(
            f32_bits_eq(g.0, w.0)
                && f32_bits_eq(g.1, w.1)
                && f32_bits_eq(g.2, w.2)
                && f32_bits_eq(g.3, w.3)
                && f32_bits_eq(g.4, w.4),
            "view_eye_offset fvar1={fvar1} v={v:?} base={base:?}\n got={g:?}\nwant={w:?}"
        );
        total += 1;
    }
    // (c)
    let mut s = 0x50eb_0f00u32;
    for _ in 0..2000 {
        let acc = [
            rngf(&mut s, -500.0, 500.0) as f32,
            rngf(&mut s, -500.0, 500.0) as f32,
            rngf(&mut s, -500.0, 500.0) as f32,
        ];
        let field = [
            rngf(&mut s, -100.0, 100.0) as f32,
            rngf(&mut s, -100.0, 100.0) as f32,
            rngf(&mut s, -100.0, 100.0) as f32,
        ];
        let g = run_c(&mut emu, acc, field);
        let w = view_offset_accum(acc, field);
        assert!(
            f32_bits_eq(g.0, w.0) && f32_bits_eq(g.1, w.1) && f32_bits_eq(g.2, w.2),
            "view_offset_accum acc={acc:?} field={field:?}"
        );
        total += 1;
    }
    eprintln!("glue4fd_50e8d0_view_compose: {total} bit-exact (fadd; 3 fmul + 3 fsub; 3 fadd)");
}

// ===========================================================================================
// 0x507710 mos_snap ceil-fraction (0x507af0) — the one snap FP-arith op past the p4/p7 slices'
// halt (0x507aa2), in the ceil-double-check block (reached when edi != ftol(ceil(m))). VERBATIM-
// byte island reproducing the source 0x507ae2..0x507af3:
//   d9 45 14   fld dword [ebp+0x14]   (== src 0x507ae2)
//   83 ec 08   sub esp,8              (== src 0x507ae5)
//   dd 1c 24   fstp qword [esp]       (== src 0x507ae8)  -> push x as f64, st0 popped
//   e8 <rel32> call ceil 0x73fdf5     (rel32 recomputed for the island position)  -> st0=ceil(x)
//   d8 65 14   fsub dword [ebp+0x14]  (== src 0x507af0, byte-IDENTICAL — the diffed op)
//   83 c4 08   add esp,8              (== src 0x507af3)  cdecl arg cleanup
//   dd 1d <o> fstp qword [out]        (appended: capture ceil(x)-x as f64)
// The fld+sub+fstp prelude and the fsub are byte-verified == the source verbatim; only the call's
// rel32 is recomputed (position-dependent). Runs the binary's REAL ceil, so the stub settles it.
// ===========================================================================================
#[test]
fn glue4fd_507710_ceil_frac() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let out = emu.alloc(8);
    const CEIL: u64 = 0x73fdf5;

    let isl = emu.alloc(0x40);
    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&[0xd9, 0x45, 0x14]); // fld dword [ebp+0x14]   == src 0x507ae2
    code.extend_from_slice(&[0x83, 0xec, 0x08]); // sub esp,8              == src 0x507ae5
    code.extend_from_slice(&[0xdd, 0x1c, 0x24]); // fstp qword [esp]       == src 0x507ae8
    let call_va = isl + code.len() as u64;
    let rel = (CEIL as i64 - (call_va as i64 + 5)) as i32;
    code.push(0xe8);
    code.extend_from_slice(&rel.to_le_bytes()); // call ceil 0x73fdf5
    let fsub_off = code.len() as u64;
    code.extend_from_slice(&[0xd8, 0x65, 0x14]); // fsub dword [ebp+0x14]  == src 0x507af0 (verbatim)
    code.extend_from_slice(&[0x83, 0xc4, 0x08]); // add esp,8              == src 0x507af3
    code.push(0xdd); // fstp qword [out] = DD 1D <disp32>
    code.push(0x1d);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // byte-verify the verbatim prelude + the diffed fsub == the source bytes (non-circular).
    assert_eq!(
        emu.read(isl, 9),
        emu.read(0x507ae2, 9),
        "ceil_frac island prelude != src 0x507ae2 (fld;sub esp;fstp qword [esp])"
    );
    assert_eq!(
        emu.read(isl + fsub_off, 3),
        emu.read(0x507af0, 3),
        "ceil_frac island fsub != src 0x507af0"
    );

    let run = |emu: &mut Emu, x: f32| -> f64 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x14, x.to_bits());
        emu.run(isl, isl_end);
        let b = emu.read(out, 8);
        f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
    };

    let mut total = 0u32;
    // ==threshold probes: integers (ceil(x)-x == 0 exactly), just-below an integer (frac→1⁻), the ε
    // pivot (0x801360=1e-3), and sign cases. The runtime m is a magnitude (≥0) but the op is domain-
    // agnostic — probe negatives + the f32-mantissa edge too.
    let eps = f32::from_bits(0x3a83_126f); // 1e-3
    let probes = [
        0.0f32,
        1.0,
        2.0,
        60.0,
        3.0 - eps,
        3.0 + eps,
        0.5,
        1.5,
        f32::from_bits(0x4b00_0000), // 2^23 (integral, frac=0)
        -1.0,
        -1.5,
        -0.5,
        0.999,
        0.001,
    ];
    for &x in &probes {
        assert_eq!(
            run(&mut emu, x).to_bits(),
            ceil_frac(x).to_bits(),
            "ceil_frac x={x}"
        );
        total += 1;
    }
    // dense sweep near every integer in the snap domain (m ≥ 0, up to ~600).
    for k in 0u32..=600 {
        for d in [-1e-4f32, 0.0, 1e-4] {
            let x = k as f32 + d;
            assert_eq!(
                run(&mut emu, x).to_bits(),
                ceil_frac(x).to_bits(),
                "ceil_frac x={x} (k={k} d={d})"
            );
            total += 1;
        }
    }
    let mut s = 0x5079_af00u32;
    for _ in 0..4000 {
        let x = rngf(&mut s, 0.0, 600.0) as f32;
        assert_eq!(
            run(&mut emu, x).to_bits(),
            ceil_frac(x).to_bits(),
            "ceil_frac fuzz x={x}"
        );
        total += 1;
    }
    eprintln!("glue4fd_507710_ceil_frac: {total} bit-exact (verbatim island: fld;push;call ceil;fsub == src 0x507af0)");
}
