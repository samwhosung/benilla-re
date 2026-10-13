//! The emitter's frame update (`systems/models/particles.md#update`): slices of
//! `CParticleEmitter2::Update` (`0x7b5230`) run to the stores they make.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::particles::update::{
    drift_block, eps_gate_early_exit, eps_gate_flag, prologue_length, trail_block, EARLY_EXIT_FLAG,
    EPS, ONE, TRAIL_PERIOD,
};
use common::{assert_f32, SLICE_ESP};

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

fn frand_mixed(s: &mut u32) -> f32 {
    let pick = (lcg(s) >> 8) % 5;
    let m = f64::from(lcg(s) >> 8) / f64::from(1u32 << 24);
    let v = match pick {
        0 => m * 2.0 - 1.0,            // [-1, 1)
        1 => m * 100.0,                // moderate
        2 => -(m * 100.0),             // negative
        3 => m * 1.0e6 - 5e5,          // large
        _ => (m * 2.0 - 1.0) * 1.0e-6, // tiny
    };
    v as f32
}

fn frand_dt(s: &mut u32) -> f32 {
    let pick = (lcg(s) >> 8) % 6;
    let m = f64::from(lcg(s) >> 8) / f64::from(1u32 << 24); // [0,1)
    let sign = if (lcg(s) >> 8) & 1 == 0 { 1.0 } else { -1.0 };
    let v = match pick {
        // tiny — straddles eps on both sides (continue + just-over)
        0 => (m * 2.0 - 1.0) * 1.0e-6,
        1 => (m * 2.0 - 1.0) * f64::from(EPS) * 4.0,
        2 => sign * m * 0.05,   // moderate frame dt — always early-exit
        3 => sign * m * 100.0,  // large — early-exit
        4 => sign * m * 1.0e-9, // far below eps — continue
        _ => m * 2.0 - 1.0,     // [-1,1)
    };
    v as f32
}

/// `0x7b523f..0x7b5277`: the distance to the camera.
#[test]
fn prologue_length_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let cur = e.alloc(0x40); // edi (cur fields at +0x30/0x34/0x38)
    let prev = e.alloc(0x40); // ebx (prev fields at +0/4/8)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, c: [f32; 3], p: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(cur + 0x30 + 4 * k as u64, c[k].to_bits());
            e.write_u32(prev + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::EDI, cur);
        e.set_reg(RegisterX86::EBX, prev);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b523f, 0x7b5277); // deltas + call 0x4549f0 (dot) + fsqrt + fstp [0xcf58e8]
        let bin_d = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let bin_len = e.read_f32(0xcf58e8);
        let (d, len) = prologue_length(c, p);
        for k in 0..3 {
            assert_f32(bin_d[k], d[k], &format!("{ctx} delta[{k}]"));
        }
        assert_f32(bin_len, len, &format!("{ctx} len"));
    };

    run_case(&mut e, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], "prologue 3-4-5");
    run_case(
        &mut e,
        [1.0, 2.0, 3.0],
        [1.0, 2.0, 3.0],
        "prologue zero-delta",
    );
    run_case(
        &mut e,
        [1e20, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "prologue huge (overflow sq)",
    );
    let mut s = 0x7b52_3f3fu32;
    for i in 0..3000u32 {
        let c = [
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
        ];
        let p = [
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
        ];
        run_case(&mut e, c, p, &format!("prologue fuzz[{i}]"));
    }
}

/// `0x7b530e..0x7b53c3`, flag 0x40000: the displacement and its clamp, NaN included.
#[test]
fn drift_block_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case =
        |e: &mut Emu, pos: [f32; 3], prev: [f32; 3], dt: f32, mul: f32, add: f32, ctx: &str| {
            for k in 0..3 {
                e.write_u32(emitter + 0x22c + 4 * k as u64, pos[k].to_bits());
                e.write_u32(emitter + 0x248 + 4 * k as u64, prev[k].to_bits());
            }
            e.write_u32(emitter + 0x1f8, mul.to_bits());
            e.write_u32(emitter + 0x1f4, add.to_bits());
            e.set_reg(RegisterX86::ESI, emitter);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.write_u32(ebp + 8, dt.to_bits()); // [ebp+8] = dt (param_2)
            e.run(0x7b530e, 0x7b53c3); // deltas + inlined normalize + ratio + clamp + scale
            let bin_out = [
                e.read_f32(emitter + 0x26c),
                e.read_f32(emitter + 0x270),
                e.read_f32(emitter + 0x274),
            ];
            let (_v, out) = drift_block(pos, prev, dt, mul, add);
            for k in 0..3 {
                assert_f32(bin_out[k], out[k], &format!("{ctx} out[{k}]"));
            }
        };

    // ratio extremes pin all three clamp sides: r<0 (→0), 0<=r<1 (→r), r>=1 (→1).
    run_case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        2.0,
        1.0,
        -10.0,
        "drift r<0 (clamp 0)",
    );
    run_case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        5.0,
        0.1,
        0.0,
        "drift mid (keep r)",
    );
    run_case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        1.0,
        10.0,
        0.0,
        "drift r>=1 (clamp 1)",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        1.0,
        1.0,
        0.5,
        "drift zero-len",
    );
    run_case(
        &mut e,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        1.0,
        f32::NAN,
        0.0,
        "drift NaN ratio (clamp 1)",
    );
    let mut s = 0x7b53_0e0eu32;
    for i in 0..4000u32 {
        let pos = [
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
        ];
        let prev = [
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
        ];
        // dt rarely zero (the fdiv [ebp+8] would be ±Inf/NaN — still diffed).
        let dt = if (lcg(&mut s) >> 8).is_multiple_of(13) {
            0.0
        } else {
            frand_mixed(&mut s)
        };
        let mul = frand_mixed(&mut s);
        let add = frand_mixed(&mut s);
        run_case(&mut e, pos, prev, dt, mul, add, &format!("drift fuzz[{i}]"));
    }
}

/// `0x7b53ce..0x7b542f`, flag 0x400: the accumulator, both sides of 1/30, NaN, and both counts.
#[test]
fn trail_block_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu,
                    dt: f32,
                    acc: f32,
                    pos: [f32; 3],
                    prev: [f32; 3],
                    count: u32,
                    burst: f32,
                    ctx: &str| {
        e.write_u32(emitter + 0x254, acc.to_bits());
        e.write_u32(emitter + 0x64, count);
        e.write_u32(emitter + 0x1c4, burst.to_bits());
        for k in 0..3 {
            e.write_u32(emitter + 0x22c + 4 * k as u64, pos[k].to_bits());
            e.write_u32(emitter + 0x248 + 4 * k as u64, prev[k].to_bits());
        }
        // pre-poison the slot so a no-write path is visible.
        for k in 0..3u64 {
            e.write_u32(emitter + 0x258 + 4 * k, 0xdead_0000 + k as u32);
        }
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, dt.to_bits());
        e.run(0x7b53ce, 0x7b542f); // accumulator + ==period gate + fired div + normalize/scale
        let bin_acc = e.read_f32(emitter + 0x254);
        let bin_slot = [
            e.read_f32(emitter + 0x258),
            e.read_f32(emitter + 0x25c),
            e.read_f32(emitter + 0x260),
        ];
        let (acc_out, fired, slot) = trail_block(dt, acc, pos, prev, count, burst);
        assert_f32(bin_acc, acc_out, &format!("{ctx} acc"));
        for k in 0..3 {
            if fired {
                assert_f32(bin_slot[k], slot[k], &format!("{ctx} slot[{k}]"));
            } else {
                assert_eq!(
                    bin_slot[k].to_bits(),
                    0xdead_0000 + k as u32,
                    "{ctx} slot[{k}] kept"
                );
            }
        }
    };

    let period = TRAIL_PERIOD;
    // gate: acc' <= period (no fire); acc' > period with count!=0 (normalize) and count==0 (zero).
    run_case(
        &mut e,
        0.01,
        0.0,
        [0.0; 3],
        [0.0; 3],
        4,
        2.0,
        "trail under-period (no fire)",
    );
    run_case(
        &mut e,
        period,
        0.0,
        [0.0; 3],
        [0.0; 3],
        4,
        2.0,
        "trail ==period (<=, no fire)",
    );
    run_case(
        &mut e,
        1.0,
        0.0,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        4,
        2.0,
        "trail fire count!=0",
    );
    run_case(
        &mut e,
        1.0,
        0.0,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        0,
        2.0,
        "trail fire count==0 (zero slot)",
    );
    run_case(
        &mut e,
        f32::NAN,
        0.0,
        [3.0, 4.0, 0.0],
        [0.0; 3],
        4,
        2.0,
        "trail NaN (no fire)",
    );
    let mut s = 0x7b53_cec0u32;
    let (mut fired_n, mut nofire_n) = (0u32, 0u32);
    for i in 0..4000u32 {
        // bias dt/acc so both gate sides fire frequently.
        let dt = if (lcg(&mut s) >> 8).is_multiple_of(2) {
            frand_mixed(&mut s).abs() * 0.05
        } else {
            frand_mixed(&mut s)
        };
        let acc = if (lcg(&mut s) >> 8).is_multiple_of(2) {
            frand_mixed(&mut s).abs() * 0.05
        } else {
            frand_mixed(&mut s)
        };
        let pos = [
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
        ];
        let prev = [
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
        ];
        let count = lcg(&mut s) & 1; // 0 or 1 — both count paths
        let burst = frand_mixed(&mut s);
        if f64::from(dt) + f64::from(acc) > f64::from(period) {
            fired_n += 1;
        } else {
            nofire_n += 1;
        }
        run_case(
            &mut e,
            dt,
            acc,
            pos,
            prev,
            count,
            burst,
            &format!("trail fuzz[{i}]"),
        );
    }
    assert!(
        fired_n > 0 && nofire_n > 0,
        "both trail gate sides must fire (fire={fired_n} nofire={nofire_n})"
    );
}

/// The 1.0 the update uses.
#[test]
fn one_constant_is_unit() {
    assert_eq!(ONE.to_bits(), 0x3f80_0000);
}

/// `0x7b52d7` to the return or the body: the short-update test about 2.3841858e-7 and NaN.
#[test]
fn eps_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi (flag word @ +0x1ac)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    // A sentinel incoming flag word with bit 0x80 CLEAR (so an early-exit `|= 0x80` is observable)
    // and other bits set (so the continue arm leaving it untouched is observable too).
    const FLAG_IN: u32 = 0x0004_0017;

    let mut early_n = 0u32;
    let mut cont_n = 0u32;

    let run_case = |e: &mut Emu, arg: f32, ctx: &str| {
        e.write_u32(emitter + 0x1ac, FLAG_IN);
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, arg.to_bits()); // [ebp+8] = arg (param_2, the dt)
        let early = eps_gate_early_exit(arg);
        // The PREDICTED halt: 0x7b5300 (ret 0x10, early-exit) or 0x7b5303 (body, continue). A wrong
        // prediction overruns the halt and the harness panics — the branch direction is the assert.
        let until = if early { 0x7b5300 } else { 0x7b5303 };
        e.run(0x7b52d7, until);
        let bin_flag = e.read_u32(emitter + 0x1ac);
        let ours_flag = eps_gate_flag(arg, FLAG_IN);
        assert_eq!(
            bin_flag, ours_flag,
            "{ctx}: flag binary={bin_flag:#010x} ours={ours_flag:#010x} (arg={arg:?})"
        );
        if early {
            assert_eq!(
                bin_flag & EARLY_EXIT_FLAG,
                EARLY_EXIT_FLAG,
                "{ctx}: early-exit must set 0x80"
            );
        } else {
            assert_eq!(
                bin_flag, FLAG_IN,
                "{ctx}: continue arm must leave flag untouched"
            );
        }
    };

    // ---- deterministic ==threshold probes (all four orderings + NaN) ----
    // early-exit iff |arg| < eps STRICTLY; continue iff |arg| >= eps or NaN.
    let eps = EPS;
    let eps_below = f32::from_bits(eps.to_bits() - 1); // one ULP below eps -> EARLY-EXIT
    let eps_above = f32::from_bits(eps.to_bits() + 1); // one ULP above eps -> CONTINUE
    let tally = |e: &mut Emu, arg: f32, ctx: &str, early: &mut u32, cont: &mut u32| {
        if eps_gate_early_exit(arg) {
            *early += 1;
        } else {
            *cont += 1;
        }
        run_case(e, arg, ctx);
    };
    // early-exit side (|arg| < eps strictly):
    tally(
        &mut e,
        0.0,
        "arg==0 (early-exit)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        -0.0,
        "arg==-0 (early-exit)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        eps_below,
        "arg==eps-1ulp (early-exit)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        -eps_below,
        "arg==-(eps-1ulp) (early-exit)",
        &mut early_n,
        &mut cont_n,
    );
    // continue side (|arg| >= eps, inclusive of the exact == boundary, + NaN):
    tally(
        &mut e,
        eps,
        "arg==+eps (continue, ==boundary)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        -eps,
        "arg==-eps (continue, ==boundary)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        eps_above,
        "arg==eps+1ulp (continue)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        0.05,
        "arg==0.05 (continue, typical dt)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        -3.0,
        "arg==-3.0 (continue)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        f32::NAN,
        "arg==NaN (continue, unordered)",
        &mut early_n,
        &mut cont_n,
    );
    tally(
        &mut e,
        f32::INFINITY,
        "arg==+Inf (continue)",
        &mut early_n,
        &mut cont_n,
    );

    // ---- fuzz both sides ----
    let mut s = 0x7b52_d7d7u32;
    for i in 0..6000u32 {
        let arg = frand_dt(&mut s);
        if eps_gate_early_exit(arg) {
            early_n += 1;
        } else {
            cont_n += 1;
        }
        run_case(&mut e, arg, &format!("eps_gate fuzz[{i}]"));
    }
    assert!(
        early_n > 0 && cont_n > 0,
        "both gate exits must fire (early={early_n} continue={cont_n})"
    );
}

/// The short-update threshold and flag.
#[test]
fn eps_constant_is_2pow_neg22() {
    assert_eq!(EPS.to_bits(), 0x3480_0000); // 2^-22
    let v = f64::from(EPS);
    assert_f32((v as f32).abs(), EPS, "eps fabs identity");
    assert_eq!(EARLY_EXIT_FLAG, 0x80);
}
