//! Differential tests of the arithmetic of the camera shake and transition functions from
//! `CCamera::BeginBobDecay` (`0x511110`) to `CCamera::BeginHeightTransition` (`0x512790`): the
//! drunk field of view, shakes, distance and height transitions, each run against `WoW.exe` in
//! the emulator.
//!
//! Each owned FP slice of the widget-channel glue TU band `[0x511110, 0x512790]` is diffed as an
//! **instruction slice** (`Emu::run`) — registers/memory/stack synthesized, the binary's own bytes
//! executed, the stored `fstp/fst [m32]` value or the `fnstsw` AH read back. Every in-body FP-arith
//! opcode (fmul/fadd/fsub/fabs/fdiv/fchs) lands in a diffed slice. Fuzz reads the LCG's HIGH bits
//!. Every owned-slice test carries a deterministic `==threshold`
//! boundary probe. The change-detect branches are reconstructed from the EMITTED jcc,
//! not the mask: `test ah,5` (C0|C2) read by `jp`/`jnp` is a PARITY test — `jp` taken ⇔ even parity,
//! `jnp` taken ⇔ odd parity.
//!
//! 0x5126b0 is the masked-byte clone-twin of 0x512120 (verified in `glue_511_512120_body`); its
//! owned FP is the same shape, covered by the 0x512120 slices. 0x512200/0x512790 share the
//! `commit_if_moved` FP slice — both addresses diffed.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::camera_shake_transitions::{
    abs_div, axis_settled, body_frac, body_int_term, commit_if_moved, neg_f32, rate_accumulate,
    rate_accumulate_below_limit, rate_accumulate_f32, rate_accumulate_live, residual2_lt_2_9,
    residual_lt_thr_a, scale_const_minus_halfpi, scale_recip36, scale_settled, EPS_BITS,
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
    let this = emu.alloc(0x400);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { this, ebp, esp }
}

// ===========================================================================================
// 0x511110 — |field| / divisor  (fabs; fdiv)
// Slice 0x511132..0x511162: fld[esi+4*eax+0x110]; fabs; fdiv[div+0x24]; fstp[esi+0x23c].
// ===========================================================================================
#[test]
fn glue_511_511110_abs_div() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let div = emu.alloc(0x40);

    let run = |emu: &mut Emu, field: f32, divisor: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, f.this);
        emu.set_reg(RegisterX86::EDI, f.this + 0x110); // edi = lea [esi+0x110] (the 3-dword copy src)
                                                       // eax = channel index 0 (so fld reads [esi+0x110]); 0x511139 reloads eax=div for the fdiv.
        emu.set_reg(RegisterX86::EAX, 0);
        emu.write_u32(f.this + 0x110, field.to_bits());
        emu.write_u32(0xbe10bc, div as u32); // *([0xbe10bc]) = div struct
        emu.write_u32(div + 0x24, divisor.to_bits());
        // start at the fld (0x511132); halt right AFTER `fstp [esi+0x23c]` (0x511159) at 0x51115f,
        // before the `call 0x42c010` post-store. (The interleaved int moves copy [edi..edi+8]→
        // [esi+0x240], incidental — edi=this+0x110, mapped.)
        emu.run(0x511132, 0x51115f);
        emu.read_f32(f.this + 0x23c)
    };

    let mut total = 0u32;
    // deterministic boundaries: divide by 1.0 (identity-abs), and an exact power-of-two divisor.
    for (field, divisor) in [
        (1.0f32, 1.0f32),
        (-1.0, 1.0),
        (3.0, 2.0),
        (-7.5, 4.0),
        (0.0, 1.0),
        (-0.0, 8.0),
    ] {
        let got = run(&mut emu, field, divisor);
        assert_eq!(
            got.to_bits(),
            abs_div(field, divisor).to_bits(),
            "abs_div field={field} div={divisor}"
        );
        total += 1;
    }
    let mut s = 0x5111_1000u32;
    for _ in 0..3000 {
        let field = rng(&mut s, -1000.0, 1000.0);
        let divisor = rng(&mut s, 0.25, 100.0);
        let got = run(&mut emu, field, divisor);
        assert_eq!(
            got.to_bits(),
            abs_div(field, divisor).to_bits(),
            "abs_div fuzz field={field} div={divisor}"
        );
        total += 1;
    }
    eprintln!("glue_511_511110_abs_div: {total} bit-exact (fabs;fdiv)");
}

// ===========================================================================================
// 0x511250 — scale (fsub const; fmul arg) + eps change-detect (fsub; fabs).
// ===========================================================================================
#[test]
fn glue_511_511250_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let eps = f32::from_bits(EPS_BITS);

    // scale slice 0x511253..0x51126d (until AFTER fstp [ebp+8] at 0x51126a): reads [ebp+8].
    let run_scale = |emu: &mut Emu, arg: f32| -> f32 {
        emu.set_reg(RegisterX86::ECX, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.write_u32(f.ebp + 0xc, 0); // keep below the change-detect-only region irrelevant here
        emu.run(0x511253, 0x51126d);
        emu.read_f32(f.ebp + 0x8)
    };
    // change-detect slice 0x51126f..0x511285 (until AFTER fnstsw at 0x511282): reads AH.
    let run_chg = |emu: &mut Emu, field: f32, scaled: f32| -> u8 {
        emu.set_reg(RegisterX86::ESI, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.this + 0x10c, field.to_bits());
        emu.write_u32(f.ebp + 0x8, scaled.to_bits());
        emu.run(0x51126f, 0x511282); // halt after fnstsw (0x511280), before test ah,5
        ah_of(emu)
    };

    let mut total = 0u32;
    for arg in [0.0f32, 1.0, -1.0, 2.5, 100.0, -50.0] {
        let got = run_scale(&mut emu, arg);
        assert_eq!(
            got.to_bits(),
            scale_const_minus_halfpi(arg).to_bits(),
            "scale arg={arg}"
        );
        total += 1;
    }
    let mut s = 0x5112_5000u32;
    for _ in 0..2500 {
        let arg = rng(&mut s, -1000.0, 1000.0);
        let got = run_scale(&mut emu, arg);
        assert_eq!(
            got.to_bits(),
            scale_const_minus_halfpi(arg).to_bits(),
            "scale fuzz arg={arg}"
        );
        total += 1;
    }

    // change-detect, with the deterministic ==eps boundary.
    let mut chg = 0u32;
    let mut settled = 0u32;
    for (field, scaled, _l) in [
        (eps, 0.0f32, "eq"),
        (eps * 0.5, 0.0, "below"),
        (eps * 2.0, 0.0, "above"),
        (f32::NAN, 0.0, "nan"),
    ] {
        let ah = run_chg(&mut emu, field, scaled);
        // jp -> taken (eax=0) iff EVEN parity; falls through (eax=1, settled) iff odd (|d|<eps).
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(
            bin,
            scale_settled(field, scaled),
            "scale_settled field={field} scaled={scaled}"
        );
        if bin {
            settled += 1;
        } else {
            chg += 1;
        }
        total += 1;
    }
    let mut s = 0x5112_5b00u32;
    for _ in 0..2000 {
        let scaled = rng(&mut s, -10.0, 10.0);
        let field = scaled + rng(&mut s, -1.0e-6, 1.0e-6);
        let ah = run_chg(&mut emu, field, scaled);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin, scale_settled(field, scaled), "scale_settled fuzz");
        if bin {
            settled += 1;
        } else {
            chg += 1;
        }
        total += 1;
    }
    assert!(
        chg > 0 && settled > 0,
        "scale change-detect: need both outcomes"
    );
    eprintln!("glue_511_511250_scale: {total} bit-exact (fsub;fmul + fsub;fabs settled={settled} chg={chg})");
}

// ===========================================================================================
// 0x511760 — (i64)Δ·dt + field, live-st0 fst then fcomp (fmul; fadd).
// Slice 0x511816..0x51182f: fild qword[ebp-0x20]; fmul[dt]; fadd[ebx+0x1c]; fst[ebp-0xc];
// fcomp[ebx+0x18]; fnstsw. Reads stored f32 [ebp-0xc] AND the live-f64 gate AH.
// ===========================================================================================
#[test]
fn glue_511_511760_rate_accumulate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let ebx = emu.alloc(0x80);

    let run = |emu: &mut Emu, delta: u32, field: f32, limit: f32| -> (f32, u8) {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.write_u32(f.ebp - 0x20, delta); // low dword = Δ
        emu.write_u32(f.ebp - 0x1c, 0); // high dword = 0
        emu.write_u32(ebx + 0x1c, field.to_bits());
        emu.write_u32(ebx + 0x18, limit.to_bits());
        emu.run(0x511816, 0x51182d); // halt after fnstsw (0x51182b), before test ah,1
        (emu.read_f32(f.ebp - 0xc), ah_of(emu))
    };

    let mut total = 0u32;
    let mut below = 0u32;
    let mut above = 0u32;
    // deterministic boundary: pick field/limit so t == limit exactly.
    let dt = f64::from(DT_F32);
    for (delta, field, limit) in [
        (1000u32, 0.0f32, (1000.0 * dt) as f32), // t == limit (boundary)
        (1000, 0.0, ((1000.0 * dt) * 1.5) as f32), // t < limit
        (1000, 0.0, ((1000.0 * dt) * 0.5) as f32), // t > limit
        (0, 5.0, 6.0),                           // Δ=0: t = field
    ] {
        let (sf, ah) = run(&mut emu, delta, field, limit);
        let t = rate_accumulate(delta, field);
        assert_eq!(
            rate_accumulate_f32(sf).to_bits(),
            sf.to_bits(),
            "ra f32 store"
        );
        assert_eq!(
            sf.to_bits(),
            t.to_bits(),
            "ra store delta={delta} field={field}"
        );
        // test ah,1; jne taken iff C0 (live f64 t < limit, or NaN).
        let bin = ah & 0x1 != 0;
        assert_eq!(
            bin,
            rate_accumulate_below_limit(rate_accumulate_live(delta, field), limit),
            "ra gate delta={delta} field={field} limit={limit} t={t}"
        );
        if bin {
            below += 1;
        } else {
            above += 1;
        }
        total += 1;
    }
    let mut s = 0x5117_6000u32;
    for _ in 0..3000 {
        let delta = (next(&mut s) >> 8) % 5_000_000;
        let field = rng(&mut s, -1000.0, 1000.0);
        let limit = rng(&mut s, -1000.0, 6000.0);
        let (sf, ah) = run(&mut emu, delta, field, limit);
        let t = rate_accumulate(delta, field);
        assert_eq!(sf.to_bits(), t.to_bits(), "ra store fuzz");
        let bin = ah & 0x1 != 0;
        assert_eq!(
            bin,
            rate_accumulate_below_limit(rate_accumulate_live(delta, field), limit),
            "ra gate fuzz delta={delta} t={t} limit={limit}"
        );
        if bin {
            below += 1;
        } else {
            above += 1;
        }
        total += 1;
    }
    // fst-no-pop straddle probe: a Δ whose `Δ·dt` rounds UP to f32 (live `t` < `f32(t)`); set
    // `limit == f32(t)`. The binary's `fcomp` reads the LIVE f64 `t` → `t < limit` → BELOW (C0=1),
    // while an f32-reload gate would compute `f32(t) < f32(t)` → ABOVE. Proves the live-f64 gate is
    // load-bearing: revert `rate_accumulate_below_limit` to the f32 narrowing and this assert fails.
    let dt = f64::from(DT_F32);
    let probe = (1u32..2_000_000)
        .find(|&d| (((d as f64) * dt) as f32 as f64) > (d as f64) * dt)
        .expect("no round-up Δ found");
    let limit = ((probe as f64) * dt) as f32; // == f32(t), strictly above the live f64 t
    let (sf, ah) = run(&mut emu, probe, 0.0, limit);
    let bin = ah & 0x1 != 0;
    assert!(
        bin,
        "straddle Δ={probe}: binary live-f64 gate must read below (t_live < limit)"
    );
    assert_eq!(
        bin,
        rate_accumulate_below_limit(rate_accumulate_live(probe, 0.0), limit),
        "straddle Δ={probe}: live-f64 gate must match the binary",
    );
    // the f32-reload gate disagrees here — the probe bites iff someone reverts the fix.
    assert_ne!(
        bin,
        f64::from(sf) < f64::from(limit),
        "straddle Δ={probe}: f32-reload gate must DISAGREE with the binary (else probe is vacuous)",
    );
    assert!(below > 0 && above > 0, "rate gate: need both outcomes");
    eprintln!("glue_511_511760_rate_accumulate: {total} bit-exact + 1 fst-no-pop straddle (Δ={probe}, fmul;fadd; live-st0 gate below={below} above={above})");
}

// ===========================================================================================
// 0x511ad0 — two-stage residual subtract (fsub; fsub), live-st0 fcom chain.
// Diffed via the two compare AHs (each fsub feeds an fcom on the LIVE f64 value). Slices start at
// the fld (0x511b5b), halt at the fnstsw BEFORE the conditional jcc.
// ===========================================================================================
#[test]
fn glue_511_511ad0_residual() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let getter_struct = emu.alloc(0x40); // eax = [ebp-0x20] -> field8 at +8

    // stage 1: 0x511b5b..0x511b69 — fld[ebp+8]; fsub[eax+8]; fcom[esi+0x1c8]; fnstsw.
    let run_s1 = |emu: &mut Emu, arg: f32, field8: f32, thr_a: f32| -> u8 {
        emu.set_reg(RegisterX86::ESI, f.this);
        emu.set_reg(RegisterX86::EAX, getter_struct);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.write_u32(getter_struct + 0x8, field8.to_bits());
        emu.write_u32(f.this + 0x1c8, thr_a.to_bits());
        emu.run(0x511b5b, 0x511b69);
        ah_of(emu)
    };
    // stage 2: 0x511b5b..0x511b7e — run through fld st(0); fsub[esi+0x1c8]; fcom[0x8089d0]; fnstsw.
    // (We must pass the 0x511b6c jnp — so use d >= thrA inputs. The jnp is the last insn BEFORE the
    // 0x511b6e block; halting at 0x511b7e (after the 2nd fnstsw) executes it. For d>=thrA the jnp is
    // NOT taken so control reaches the 2nd stage straight-line.)
    let run_s2 = |emu: &mut Emu, arg: f32, field8: f32, thr_a: f32| -> u8 {
        emu.set_reg(RegisterX86::ESI, f.this);
        emu.set_reg(RegisterX86::EAX, getter_struct);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.write_u32(getter_struct + 0x8, field8.to_bits());
        emu.write_u32(f.this + 0x1c8, thr_a.to_bits());
        emu.run(0x511b5b, 0x511b7e);
        ah_of(emu)
    };

    let mut total = 0u32;
    let mut lt = 0u32;
    let mut ge = 0u32;
    // stage-1 deterministic boundary: d == thrA exactly (arg-field8 == thrA).
    for (arg, field8, thr_a, _l) in [
        (5.0f32, 4.0f32, 1.0f32, "d==thrA"),
        (5.0, 4.0, 2.0, "d<thrA"),
        (5.0, 4.0, 0.5, "d>thrA"),
        (f32::NAN, 0.0, 1.0, "nan"),
    ] {
        let ah = run_s1(&mut emu, arg, field8, thr_a);
        // jnp taken (early flag) iff odd parity (d<thrA ordered). residual_lt_thr_a => odd.
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(
            bin,
            residual_lt_thr_a(arg, field8, thr_a),
            "s1 arg={arg} f8={field8} thrA={thr_a}"
        );
        if bin {
            lt += 1;
        } else {
            ge += 1;
        }
        total += 1;
    }
    let mut s = 0x511a_d000u32;
    for _ in 0..2500 {
        let field8 = rng(&mut s, -10.0, 10.0);
        let thr_a = rng(&mut s, -2.0, 2.0);
        let arg = field8 + thr_a + rng(&mut s, -1.0e-5, 1.0e-5);
        let ah = run_s1(&mut emu, arg, field8, thr_a);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin, residual_lt_thr_a(arg, field8, thr_a), "s1 fuzz");
        if bin {
            lt += 1;
        } else {
            ge += 1;
        }
        total += 1;
    }
    assert!(lt > 0 && ge > 0, "stage1: need both d<thrA and d>=thrA");

    // stage-2: only valid when d >= thrA (else the binary jnp's out before reaching stage 2). Build
    // inputs with d = arg-field8 >= thrA, then probe d2 == 2/9 boundary.
    let two_ninths = f32::from_bits(0x3e63_8e39);
    let mut s2lt = 0u32;
    let mut s2ge = 0u32;
    for (arg, field8, thr_a, _l) in [
        // d = 2.0, thrA = 1.0 -> d2 = 1.0 (> 2/9); d=thrA -> d2=0 (<2/9); d2 == 2/9 boundary:
        (3.0f32, 1.0f32, 1.0f32, "d2>2/9"),
        (2.0, 1.0, 1.0, "d2=0<2/9"),
        (1.0 + 1.0 + f32::from_bits(0x3e63_8e39), 1.0, 1.0, "d2==2/9"),
    ] {
        // ensure d >= thrA for stage 2 reachability.
        let d = f64::from(arg) - f64::from(field8);
        assert!(
            d >= f64::from(thr_a) - 1e-9,
            "stage2 input must have d>=thrA"
        );
        let ah = run_s2(&mut emu, arg, field8, thr_a);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(
            bin,
            residual2_lt_2_9(arg, field8, thr_a),
            "s2 arg={arg} f8={field8} thrA={thr_a} two_ninths={two_ninths}"
        );
        if bin {
            s2lt += 1;
        } else {
            s2ge += 1;
        }
        total += 1;
    }
    let mut s = 0x511a_d200u32;
    for _ in 0..2500 {
        let field8 = rng(&mut s, -10.0, 10.0);
        let thr_a = rng(&mut s, -1.0, 1.0);
        // d = thrA + extra (extra >= 0 so d >= thrA), d2 = extra; probe extra near 2/9.
        let extra = rng(&mut s, 0.0, 0.5);
        let arg = field8 + thr_a + extra;
        let d = f64::from(arg) - f64::from(field8);
        if d < f64::from(thr_a) {
            continue; // skip rare rounding where d slipped below thrA (binary jnp's out)
        }
        let ah = run_s2(&mut emu, arg, field8, thr_a);
        let bin = (ah & 0x5).count_ones() % 2 == 1;
        assert_eq!(bin, residual2_lt_2_9(arg, field8, thr_a), "s2 fuzz");
        if bin {
            s2lt += 1;
        } else {
            s2ge += 1;
        }
        total += 1;
    }
    assert!(s2lt > 0 && s2ge > 0, "stage2: need both d2<2/9 and d2>=2/9");
    eprintln!("glue_511_511ad0_residual: {total} bit-exact (fsub;fsub live-st0 fcom chain)");
}

// ===========================================================================================
// 0x511da0 — negate a delegated getter return (fchs).
// Slice 0x511e3a..0x511e45 calls the vtable getter then fchs;fstp[esp]. We diff the OWNED fchs via
// a copied-byte island (the getter return is foreign): fld[cell]; fchs; fstp[out]. Island bytes
// d9 e0 (fchs) + d9 1c 24 (fstp [esp]) are byte-verified against 0x511e3d/0x511e42.
// ===========================================================================================
#[test]
fn glue_511_511da0_neg() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // byte-verify the OWNED `fchs` op-site inside 0x511da0's extent via
    // `emu.read` (records the .text address — the diffed op lives at its own site, not a rider).
    assert_eq!(
        emu.read(0x511e3d, 2),
        vec![0xd9, 0xe0],
        "fchs @0x511e3d (0x511da0)"
    );
    let cell = emu.alloc(4);
    let out = emu.alloc(4);
    // island: fld dword [cell]; fchs; fstp dword [out].  (0x511e3d = d9 e0 fchs; 0x511e42 = d9 1c 24
    // fstp [esp] -> we redirect to fstp [out] with the same opcode family.)
    let region = emu.alloc(0x20);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [cell]
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0xe0]); // fchs            (byte-verified vs 0x511e3d)
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run = |emu: &mut Emu, x: f32| -> f32 {
        emu.write_u32(cell, x.to_bits());
        emu.run(region, end);
        emu.read_f32(out)
    };

    let mut total = 0u32;
    for x in [0.0f32, -0.0, 1.0, -1.0, 3.5, -7.25, f32::INFINITY] {
        assert_eq!(
            run(&mut emu, x).to_bits(),
            neg_f32(x).to_bits(),
            "neg x={x}"
        );
        total += 1;
    }
    let mut s = 0x511d_a000u32;
    for _ in 0..2000 {
        let x = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(run(&mut emu, x).to_bits(), neg_f32(x).to_bits(), "neg fuzz");
        total += 1;
    }
    eprintln!("glue_511_511da0_neg: {total} bit-exact (fchs copied-byte island, d9 e0 verified)");
}

// ===========================================================================================
// 0x511d40 — field · 1/36 scale (fmul). Slice 0x511d62..0x511d82: fld[eax+0xc]; (4× push edx from
// [eax+0x18/14/10/8]); fmul[0x7ff9d0=1/36]; push ecx; fstp[esp]. Reads the scaled f32 at [esp].
// ===========================================================================================
#[test]
fn glue_511_511d40_scale_recip36() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let row = emu.alloc(0x40); // eax -> table row; field at +0xc, plus +0x4/8/10/14/18 read as int.

    let run = |emu: &mut Emu, field: f32| -> f32 {
        emu.set_reg(RegisterX86::EAX, row);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(row + 0xc, field.to_bits());
        // halt at 0x511d82 (after fstp [esp] @0x511d7f, before the final `push edx`): the 4× push edx
        // + push ecx left esp = entry_esp - 0x14, and `fstp [esp]` wrote the scaled value there.
        emu.run(0x511d62, 0x511d82);
        emu.read_f32(emu.get_reg(RegisterX86::ESP))
    };

    let mut total = 0u32;
    for field in [0.0f32, -0.0, 1.0, -1.0, 36.0, -72.0, 0.5] {
        assert_eq!(
            run(&mut emu, field).to_bits(),
            scale_recip36(field).to_bits(),
            "scale_recip36 field={field}"
        );
        total += 1;
    }
    let mut s = 0x511d_4000u32;
    for _ in 0..3000 {
        let field = rng(&mut s, -10000.0, 10000.0);
        assert_eq!(
            run(&mut emu, field).to_bits(),
            scale_recip36(field).to_bits(),
            "scale_recip36 fuzz field={field}"
        );
        total += 1;
    }
    eprintln!("glue_511_511d40_scale_recip36: {total} bit-exact (fmul[1/36])");
}

// ===========================================================================================
// 0x512120 (and clone-twin 0x5126b0) — change-detect quartet + body.
//   quartet: 4× fld[field]; fsub[arg]; fabs; fcomp[dt] (axis_settled).
//   body: fsub; (fmul -> __ftol); fabs; fdiv st,st(1); fmul -> fstp[esp] (body_frac + body_int).
// ===========================================================================================
#[test]
fn glue_511_512120_quartet() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // The 4 guard tests, each a self-contained fld;fsub;fabs;fcomp;fnstsw slice.
    // (fld_start, field_off, arg_ebp_off, fnstsw_end)
    let guards: [(u64, u64, u64, u64); 4] = [
        (0x512132, 0x198, 0x8, 0x512145),
        (0x51214a, 0x1a4, 0xc, 0x51215d),
        (0x512162, 0x1a0, 0x10, 0x512175),
        (0x512184, 0xec, 0x8, 0x512197),
    ];
    let run_guard =
        |emu: &mut Emu, start: u64, foff: u64, aoff: u64, end: u64, field: f32, arg: f32| -> u8 {
            emu.set_reg(RegisterX86::ESI, f.this);
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.write_u32(f.this + foff, field.to_bits());
            emu.write_u32(f.ebp + aoff, arg.to_bits());
            emu.run(start, end);
            ah_of(emu)
        };

    let mut total = 0u32;
    let mut settled = 0u32;
    let mut moved = 0u32;
    for &(start, foff, aoff, end) in &guards {
        for (field, arg, _l) in [
            (DT_F32, 0.0f32, "==dt"),
            (DT_F32 * 0.5, 0.0, "below"),
            (DT_F32 * 2.0, 0.0, "above"),
            (f32::NAN, 0.0, "nan"),
        ] {
            let ah = run_guard(&mut emu, start, foff, aoff, end, field, arg);
            // jp taken (out/moved) iff EVEN parity; falls through (settled) iff odd (|d|<dt).
            let bin = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(
                bin,
                axis_settled(field, arg),
                "guard @ {start:#x} field={field} arg={arg}"
            );
            if bin {
                settled += 1;
            } else {
                moved += 1;
            }
            total += 1;
        }
        let mut s = 0x5121_2000u32 ^ start as u32;
        for _ in 0..600 {
            let arg = rng(&mut s, -10.0, 10.0);
            let field = arg + rng(&mut s, -0.005, 0.005);
            let ah = run_guard(&mut emu, start, foff, aoff, end, field, arg);
            let bin = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(bin, axis_settled(field, arg), "guard fuzz @ {start:#x}");
            if bin {
                settled += 1;
            } else {
                moved += 1;
            }
            total += 1;
        }
    }
    assert!(settled > 0 && moved > 0, "quartet: need both outcomes");
    eprintln!("glue_511_512120_quartet: {total} bit-exact (4× fsub;fabs guard, settled={settled} moved={moved})");
}

#[test]
fn glue_511_512120_body() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let g_struct = emu.alloc(0x40);

    // body slice 0x5121a3..0x5121ee (halt BEFORE fstp st(0)). Runs __ftol (0x40a2b0) inline. Reads
    // the f32 result at [esp] (pushed by fstp [esp] @0x5121ea; esp = entry_esp - 8 from 2× push ecx).
    let run = |emu: &mut Emu,
               arg0: f32,
               arg1: f32,
               arg2: f32,
               arg4: i32,
               gv: f32,
               field_ec: f32|
     -> (f32, i32) {
        emu.set_reg(RegisterX86::ESI, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp + 0x8, arg0.to_bits());
        emu.write_u32(f.ebp + 0xc, arg1.to_bits());
        emu.write_u32(f.ebp + 0x10, arg2.to_bits());
        emu.write_u32(f.ebp + 0x14, arg4 as u32);
        emu.write_u32(f.this + 0xec, field_ec.to_bits());
        emu.write_u32(0xbe106c, g_struct as u32); // *([0xbe106c]) = g struct
        emu.write_u32(g_struct + 0x24, gv.to_bits());
        // halt at 0x5121ed (after fstp [esp] @0x5121ea, before `push edx`@0x5121ed): the two
        // `push ecx` (0x5121e3/e4) left esp = entry_esp-8, and `fstp [esp]` wrote the frac there.
        emu.run(0x5121a3, 0x5121ed);
        let frac = emu.read_f32(f.esp - 8);
        // the int term `arg4 - __ftol(arg1*-1000)` was computed in ecx (0x5121e1) then `push ecx`
        // (0x5121e3) to [esp-4]; the 2nd `push ecx` + `fstp [esp]` overwrote [esp-8] with the frac,
        // so the int survives at [esp-4]. (ecx itself is clobbered by `mov ecx,esi` @0x5121e5.)
        let int_term = emu.read_u32(f.esp - 4) as i32;
        (frac, int_term)
    };

    let mut total = 0u32;
    for (arg0, arg1, arg2, arg4, gv, field_ec) in [
        (10.0f32, 1.0f32, 2.0f32, 0i32, 4.0f32, 0.0f32),
        (-5.0, 0.5, 3.0, 100, 2.0, 1.0),
        (0.0, -1.0, 1.0, -50, 1.0, 0.0),
        (7.5, 0.0, -2.0, 7, 0.5, -3.0),
    ] {
        let (frac, ecx) = run(&mut emu, arg0, arg1, arg2, arg4, gv, field_ec);
        assert_eq!(
            frac.to_bits(),
            body_frac(arg0, field_ec, arg2, gv).to_bits(),
            "body_frac arg0={arg0} arg2={arg2} g={gv} fec={field_ec}"
        );
        assert_eq!(
            ecx,
            body_int_term(arg1, arg4),
            "body_int arg1={arg1} arg4={arg4}"
        );
        total += 2;
    }
    let mut s = 0x5121_2000u32;
    for _ in 0..3000 {
        let arg0 = rng(&mut s, -1000.0, 1000.0);
        let arg1 = rng(&mut s, -50.0, 50.0);
        let arg2 = rng(&mut s, -100.0, 100.0);
        let arg4 = (next(&mut s) >> 8) as i32 % 200_000 - 100_000;
        let gv = rng(&mut s, 0.25, 100.0);
        let field_ec = rng(&mut s, -1000.0, 1000.0);
        let (frac, ecx) = run(&mut emu, arg0, arg1, arg2, arg4, gv, field_ec);
        assert_eq!(
            frac.to_bits(),
            body_frac(arg0, field_ec, arg2, gv).to_bits(),
            "body_frac fuzz arg0={arg0} g={gv}"
        );
        assert_eq!(ecx, body_int_term(arg1, arg4), "body_int fuzz arg1={arg1}");
        total += 2;
    }
    eprintln!("glue_511_512120_body: {total} bit-exact (fsub;fmul[__ftol];fabs;fdiv;fmul) — twin 0x5126b0 covered by masked-byte equality");
}

// ===========================================================================================
// 0x512200 / 0x512790 — |field − arg| < dt change-detect (fsub; fabs). Both addresses diffed.
// ===========================================================================================
#[test]
fn glue_511_commit_if_moved() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // 0x512200: fld[ecx+0xec]; fsub[ebp+8]; fabs; fcomp[dt]; fnstsw  (end after fnstsw @0x512214).
    // 0x512790: fld[ecx+0xfc]; fsub[ebp+8]; fabs; fcomp[dt]; fnstsw  (end after fnstsw @0x5127a4).
    let sites: [(u64, u64, u64); 2] = [(0x512203, 0xec, 0x512216), (0x512793, 0xfc, 0x5127a6)];
    let run = |emu: &mut Emu, start: u64, off: u64, end: u64, field: f32, arg: f32| -> u8 {
        emu.set_reg(RegisterX86::ECX, f.this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.this + off, field.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(start, end);
        ah_of(emu)
    };

    let mut total = 0u32;
    let mut commit = 0u32;
    let mut hold = 0u32;
    for &(start, off, end) in &sites {
        for (field, arg, _l) in [
            (DT_F32, 0.0f32, "==dt"),
            (DT_F32 * 0.5, 0.0, "below"),
            (DT_F32 * 2.0, 0.0, "above"),
            (f32::NAN, 0.0, "nan"),
        ] {
            let ah = run(&mut emu, start, off, end, field, arg);
            // jp taken (commit/body) iff EVEN parity (|d|>=dt or unordered). commit_if_moved => even.
            let bin = (ah & 0x5).count_ones() % 2 == 0;
            assert_eq!(
                bin,
                commit_if_moved(field, arg),
                "commit @ {start:#x} field={field} arg={arg}"
            );
            if bin {
                commit += 1;
            } else {
                hold += 1;
            }
            total += 1;
        }
        let mut s = 0x5122_0000u32 ^ start as u32;
        for _ in 0..1500 {
            let arg = rng(&mut s, -10.0, 10.0);
            let field = arg + rng(&mut s, -0.005, 0.005);
            let ah = run(&mut emu, start, off, end, field, arg);
            let bin = (ah & 0x5).count_ones() % 2 == 0;
            assert_eq!(bin, commit_if_moved(field, arg), "commit fuzz @ {start:#x}");
            if bin {
                commit += 1;
            } else {
                hold += 1;
            }
            total += 1;
        }
    }
    assert!(
        commit > 0 && hold > 0,
        "commit_if_moved: need both outcomes"
    );
    eprintln!("glue_511_commit_if_moved: {total} bit-exact (0x512200 + 0x512790 fsub;fabs commit={commit} hold={hold})");
}
