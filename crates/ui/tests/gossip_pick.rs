//! Differential tests of the arithmetic of `0x490e60` (four rounded position legs), `0x4c0f50`
//! (tick-to-fraction band) and `PublishGossipGreeting` (`0x4e2010`, the weighted pick), each run
//! against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed as an **instruction slice** (`Emu::run`) over just the binary's own
//! FP-compute bytes — registers/memory/stack synthesized, the stored `fstp/fst [m32]` value or the
//! `__ftol` EAX / loop-index ECX read back. Every in-body FP-arith opcode (`fcom`/`fcomp`
//! included) lands in a diffed slice; the `fild`/`fld`/`fst`/`fstp` and integer
//! ops ride along inside the slices for free.
//!
//! * **0x490e60** — the four position legs run NATIVELY across their pure-x87 helpers (0x41ad70 =
//!   `fld 1.0`, 0x41ae40 = `fld a; fdiv 0.8`, __ftol = truncate), reading EAX. The mid-product's
//!   `fstp [esp]` f32 round and the `fcom 0.0`-gated ±0.5 sign bias are exercised by construction.
//! * **0x4c0f50** — leg A (`fild;fmul·0.001`), the `fcomp [D]` threshold (a `==`-boundary probe
//!   reading `fnstsw ah`), and leg B (`fdiv;fmul·255;fadd·512`) diffed as separate slices.
//! * **0x4e2010** — the accumulate sum, the `(2−r)·sum` threshold, and the weighted-pick search
//!   loop (`fadd`+`fcomp st1`, with a `==`-boundary case at exactly thr==acc) over a synthesized
//!   8-slot flag/value table at a caller-owned `esi`.
//!
//! Fuzz reads the LCG's HIGH bits.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::gossip_pick::{
    accumulate_sum, legA_seconds, legB_band, leg_round_plus, leg_round_signed, rand_unit,
    threshold, weighted_pick,
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
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    // a scratch ebp frame (cells at ebp±0x40) plus a generous stack below esp for the internal
    // helper `call`/`push`/`ret` of 0x490e60's legs.
    let scratch = emu.alloc(0x200);
    let ebp = scratch + 0x100;
    let stack = emu.alloc(0x4000);
    let esp = stack + 0x3000;
    Frame { ebp, esp }
}

// ===========================================================================================
// 0x490e60 — 4× "round ((v−Y)/div·1024)/0.8 ± 0.5" legs. Each runs NATIVELY across the pure-x87
// helpers 0x41ad70/0x41ae40/__ftol(0x40a2b0); EAX is the truncated i32 result.
// fmul·1024 ×4, fdiv ×4, fmulp ×4, fcom ×2, fsub·0.5/fadd·0.5 (the ±bias) ×2+2.
//
// (start, end, has_y) — start = the leg's `call 0x41ad70`; end = the `push eax` after __ftol.
// Leg 1 has no `fsub Y`; legs 2,3,4 subtract a per-leg Y. Legs 1,2 carry the fcom sign-bias.
// ===========================================================================================
#[test]
fn glue_4c0_490e60_round_pos_legs() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // The v/Y cells per leg (ebp-relative). div is always [ebp-4]. y_off = None ⇒ leg has no fsub.
    // leg1: v=[ebp-0x14], no Y, signed bias. leg2: v=[ebp-0x10], Y=[ebp-0x20], signed bias.
    // leg3: v=[ebp-0xc],  Y=[ebp-0x14], +0.5.     leg4: v=[ebp-0x10], Y=[ebp-0x18], +0.5.
    struct Leg {
        start: u64,
        end: u64,
        v_off: i64,
        y_off: Option<i64>,
        signed: bool,
    }
    let legs = [
        Leg {
            start: 0x490fa8,
            end: 0x490fe4,
            v_off: -0x14,
            y_off: None,
            signed: true,
        },
        Leg {
            start: 0x491022,
            end: 0x491061,
            v_off: -0x10,
            y_off: Some(-0x20),
            signed: true,
        },
        Leg {
            start: 0x4910ae,
            end: 0x4910d8,
            v_off: -0xc,
            y_off: Some(-0x14),
            signed: false,
        },
        Leg {
            start: 0x491116,
            end: 0x491140,
            v_off: -0x10,
            y_off: Some(-0x18),
            signed: false,
        },
    ];

    let run = |emu: &mut Emu, leg: &Leg, v: f32, y: f32, div: f32| -> i32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32((f.ebp as i64 - 0x4) as u64, div.to_bits()); // [ebp-4] = div
        emu.write_u32((f.ebp as i64 + leg.v_off) as u64, v.to_bits());
        if let Some(yo) = leg.y_off {
            emu.write_u32((f.ebp as i64 + yo) as u64, y.to_bits());
        }
        emu.run(leg.start, leg.end);
        emu.get_reg(RegisterX86::EAX) as i32
    };
    let want = |leg: &Leg, v: f32, y: f32, div: f32| -> i32 {
        // leg 1 has no `fsub Y` — the binary never reads a Y cell, so its effective Y is 0.
        let y_eff = if leg.y_off.is_some() { y } else { 0.0 };
        if leg.signed {
            leg_round_signed(v, y_eff, div)
        } else {
            leg_round_plus(v, y_eff, div)
        }
    };

    let mut total = 0u32;
    // deterministic probes incl. the sign pivot (x just below/at 0, the half-away boundary), and
    // exact-half products that test round-half-away-from-zero.
    let probes: [(f32, f32, f32); 22] = [
        (0.0, 0.0, 1.0),
        (1.0, 0.0, 1.0),    // 1/1*1024/0.8 = 1280
        (-1.0, 0.0, 1.0),   // negative ⇒ −0.5 bias
        (0.8, 0.0, 1024.0), // 0.8/1024*1024/0.8 = 1.0
        (1.0, 0.0, 1600.0), // 1/1600*1024/0.8 = 0.8 → round to 1
        (1.0, 1.0, 1.0),    // (1-1)=0
        (1.5, 0.5, 800.0),
        (100.0, 0.0, 1.0),
        (-100.0, 0.0, 1.0),
        (0.5, 0.0, 1280.0),  // 0.5/1280*1024/0.8 = 0.5 exactly → half-away → 1
        (-0.5, 0.0, 1280.0), // −0.5 exactly → −1
        // NaN, the infinities and quotients past `i32` and `i64`: `__ftol` returns the low dword of
        // its qword, 0 for the indefinite.
        (f32::NAN, 0.0, 1.0),
        (1.0, f32::NAN, 1.0),
        (1.0, 0.0, f32::NAN),
        (f32::INFINITY, 0.0, 1.0),
        (f32::NEG_INFINITY, 0.0, 1.0),
        (1.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
        (1.0e10, 0.0, 1.0),
        (-1.0e10, 0.0, 1.0),
        (1.0e30, 0.0, 1.0),
        (1.0, 0.0, 1.0e-30),
    ];
    for leg in &legs {
        for &(v, y, div) in &probes {
            assert_eq!(
                run(&mut emu, leg, v, y, div),
                want(leg, v, y, div),
                "490e60 leg@{:#x} v={v} y={y} div={div}",
                leg.start
            );
            total += 1;
        }
        let mut s = (leg.start as u32).wrapping_mul(0x9e37);
        for _ in 0..3000 {
            let v = rng(&mut s, -5000.0, 5000.0);
            let y = if leg.y_off.is_some() {
                rng(&mut s, -5000.0, 5000.0)
            } else {
                0.0
            };
            // div spanning small & large, both signs; avoid exact 0 (binary would div-by-0 too, but
            // the transcription's f64 path matches inf/nan bit-for-bit — still, keep div nonzero).
            let mut div = rng(&mut s, -2000.0, 2000.0);
            if div == 0.0 {
                div = 1.0;
            }
            assert_eq!(
                run(&mut emu, leg, v, y, div),
                want(leg, v, y, div),
                "490e60 leg@{:#x} fuzz v={v} y={y} div={div}",
                leg.start
            );
            total += 1;
        }
    }
    eprintln!("glue_4c0_490e60_round_pos_legs: {total} bit-exact (4 legs · (v−Y)/div·1024/0.8 ± 0.5 → __ftol)");
}

// ===========================================================================================
// 0x4c0f50 — tick→seconds→0..255 band. Leg A: fild;fmul·0.001. The fcomp [D] threshold (== probe
// via fnstsw ah). Leg B: fdiv[D];fmul·255;fadd·512. fmul ×2, fdiv ×1, fadd ×1, fcomp ×1.
// ===========================================================================================
#[test]
fn glue_4c0_4c0f50_tick_band() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // leg A: set [ebp-0xc]=elapsed low, [ebp-8]=0 (high), run fild;fmul·0.001;fstp [ebp-8].
    let run_a = |emu: &mut Emu, elapsed: u32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0xc, elapsed); // i64 low dword
        emu.write_u32(f.ebp - 0x8, 0); // i64 high dword (binary forces 0)
        emu.run(0x4c10df, 0x4c10eb); // fild qword; fmul [0.001]; fstp [ebp-8]
        emu.read_f32(f.ebp - 0x8)
    };
    // leg B: set [ebp-8]=t, [0xb717bc]=d, run fld;fdiv;fmul·255;fadd·512;fstp [ebp-8].
    let run_b = |emu: &mut Emu, t: f32, d: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x8, t.to_bits());
        emu.write_u32(0xb717bc, d.to_bits());
        emu.run(0x4c1123, 0x4c1141); // fld [ebp-8]; fdiv [D]; fmul [255]; fadd [512]; fstp [ebp-8]
        emu.read_f32(f.ebp - 0x8)
    };
    // the fcomp threshold: fld [ebp-8]=t; fcomp [0xb717bc]=d; fnstsw ax. Read AX, test C0 (ah&1) =
    // (t < d). Halt right after fnstsw (a bb boundary: the next insn is `test ah,1`).
    let run_cmp = |emu: &mut Emu, t: f32, d: f32| -> u16 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.ebp - 0x8, t.to_bits());
        emu.write_u32(0xb717bc, d.to_bits());
        emu.run(0x4c10f7, 0x4c1102); // fld [ebp-8]; fcomp [D]; fnstsw ax
        (emu.get_reg(RegisterX86::EAX) & 0xffff) as u16
    };

    let mut total = 0u32;

    // --- leg A: elapsed·0.001 (i64 with high=0, so a non-negative u32 magnitude). ---
    for elapsed in [
        0u32,
        1,
        2,
        999,
        1000,
        1001,
        60_000,
        1_000_000,
        0x7fff_ffff,
        0xffff_ffff,
    ] {
        assert_eq!(
            run_a(&mut emu, elapsed).to_bits(),
            legA_seconds(i64::from(elapsed)).to_bits(),
            "legA elapsed={elapsed}"
        );
        total += 1;
    }
    let mut s = 0x4c0f_5001u32;
    for _ in 0..3000 {
        let elapsed = next(&mut s) >> 8; // high bits, [0, 2^24)
        assert_eq!(
            run_a(&mut emu, elapsed).to_bits(),
            legA_seconds(i64::from(elapsed)).to_bits(),
            "legA fuzz elapsed={elapsed}"
        );
        total += 1;
    }

    // --- leg B: (t/d)·255 + 512. ---
    for &(t, d) in &[
        (0.0f32, 1.0f32),
        (1.0, 2.0),
        (0.5, 1.0),
        (1.0, 1.0),
        (3.7, 10.0),
        (60.0, 30.0),
        (1.0, 0.001),
        (123.456, 7.89),
    ] {
        assert_eq!(
            run_b(&mut emu, t, d).to_bits(),
            legB_band(t, d).to_bits(),
            "legB t={t} d={d}"
        );
        total += 1;
    }
    let mut s = 0x4c0f_50b0u32;
    for _ in 0..3000 {
        let t = rng(&mut s, 0.0, 1000.0);
        let mut d = rng(&mut s, 0.01, 1000.0);
        if d == 0.0 {
            d = 1.0;
        }
        assert_eq!(
            run_b(&mut emu, t, d).to_bits(),
            legB_band(t, d).to_bits(),
            "legB fuzz t={t} d={d}"
        );
        total += 1;
    }

    // --- fcomp threshold probe: ah&1 (C0) must equal (t < d), incl. the == boundary t==d. ---
    let cmp_cases: [(f32, f32); 6] = [
        (1.0, 1.0), // equal — C0 must be 0 (not less-than)
        (0.999_999, 1.0),
        (1.000_001, 1.0),
        (0.0, 0.0),  // equal
        (-1.0, 1.0), // less
        (5.0, 2.0),  // greater
    ];
    for &(t, d) in &cmp_cases {
        let ax = run_cmp(&mut emu, t, d);
        let c0 = (ax >> 8) & 1; // ah bit0 = C0 (st0 < src)
        let expect = u16::from(t < d);
        assert_eq!(c0, expect, "fcomp C0 t={t} d={d} ax={ax:#06x}");
        total += 1;
    }
    eprintln!(
        "glue_4c0_4c0f50_tick_band: {total} bit-exact (legA ·0.001 ; fcomp [D] ; legB /d·255+512)"
    );
}

// ===========================================================================================
// 0x4e2010 — weighted pick over an 8-slot flag/value table at a caller-owned esi.
//   slice 1 (sum)  0x4e20f5..0x4e2129 : fld 0; for i { if flag[i] { fadd value[i] } }; fstp [ebp-8]
//   slice 2 (thr)  0x4e212e..0x4e214e : fld 2.0; fsub r; fmul [sum]; fstp [ebp-4]   (r built from EAX)
//   slice 3 (pick) 0x4e214e..break    : acc=0; for i { if flag[i] { acc+=value[i]; if thr<=acc break } }
// fadd ×2, fsub ×1, fmul ×1, fcomp ×1.
//
// esi layout: values (f32, stride 4) at esi[0x00..0x20); flags (dword, stride 8) at esi[0x20..0x60)
// (edx=[ebp-0x10]=0 ⇒ edi=esi+0x20, disjoint from the value array).
// ===========================================================================================
#[test]
fn glue_4c0_4e2010_weighted_pick() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let esi = emu.alloc(0x100);

    let setup_table = |emu: &mut Emu, flags: &[bool; 8], values: &[f32; 8]| {
        for i in 0..8 {
            emu.write_u32(esi + 4 * i as u64, values[i].to_bits()); // value[i]
            emu.write_u32(esi + 0x20 + 8 * i as u64, u32::from(flags[i])); // flag[i] (stride 8)
        }
    };

    // slice 1: returns the accumulated sum [ebp-8] (edx=[ebp-0x10]=0 ⇒ edi=esi+0x20).
    let run_sum = |emu: &mut Emu, flags: &[bool; 8], values: &[f32; 8]| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, esi);
        // `mov edx,[ebp-0x10]` is at 0x4e20f2, before our 0x4e20f5 start — seed EDX (= the selector,
        // 0) so `lea edi,[esi+4*edx+0x20]` lands on the flags base esi+0x20.
        emu.set_reg(RegisterX86::EDX, 0);
        emu.write_u32(f.ebp - 0x10, 0); // edx selector ⇒ edi = esi+0x20
        setup_table(emu, flags, values);
        emu.run(0x4e20f5, 0x4e2129); // accumulate loop, halt at the `call 0x4531e0`
        emu.read_f32(f.ebp - 0x8)
    };

    // slice 2: returns the threshold [ebp-4] = (2.0 - rand_unit(raw)) * sum. EAX=raw PRNG dword.
    let run_thr = |emu: &mut Emu, raw: u32, sum: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EAX, u64::from(raw));
        emu.set_reg(RegisterX86::EDI, 0); // `mov edx,edi` is harmless
        emu.write_u32(f.ebp - 0x8, sum.to_bits());
        emu.run(0x4e212e, 0x4e214e); // fld 2.0; fsub r; fmul [sum]; fstp [ebp-4]
        emu.read_f32(f.ebp - 0x4)
    };

    // slice 3: the search loop. edi = esi+0x20 (edx selector 0). Returns the picked index via ECX:
    // expected<8 ⇒ break at 0x4e2172 (read ECX); expected==8 ⇒ fall-through to 0x4e216e.
    let run_pick =
        |emu: &mut Emu, flags: &[bool; 8], values: &[f32; 8], thr: f32, expect: usize| {
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.set_reg(RegisterX86::ESI, esi);
            // slice 3 walks the flags through EDX and indexes values by ECX (the `mov edx,edi` /
            // `xor ecx,ecx` are in slice 2's tail, before our start) — seed both: EDX = flags base
            // (esi+0x20, the selector-0 edi), ECX = 0 (the value index).
            emu.set_reg(RegisterX86::EDX, esi + 0x20);
            emu.set_reg(RegisterX86::ECX, 0);
            emu.write_u32(f.ebp - 0x14, 0); // acc seed = 0.0 (fld [ebp-0x14])
            emu.write_u32(f.ebp - 0x4, thr.to_bits()); // threshold (fld [ebp-4])
            setup_table(emu, flags, values);
            if expect < 8 {
                emu.run(0x4e214e, 0x4e2172); // break: halt at `test ecx,ecx`
                assert_eq!(
                    emu.get_reg(RegisterX86::ECX) as usize,
                    expect,
                    "4e2010 pick index thr={thr}"
                );
            } else {
                emu.run(0x4e214e, 0x4e216e); // no break: fall-through to `fstp st(0)`
            }
        };

    let mut total = 0u32;

    // --- slice 1 + slice 3 over deterministic tables. ---
    let tables: [([bool; 8], [f32; 8]); 6] = [
        (
            [true, true, true, true, true, true, true, true],
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        ),
        (
            [true, false, true, false, true, false, true, false],
            [10.0, 99.0, 20.0, 99.0, 30.0, 99.0, 40.0, 99.0],
        ),
        (
            [false, false, false, false, false, false, false, false],
            [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
        ),
        (
            [true, true, false, true, false, false, true, false],
            [0.25, 0.5, 9.0, 0.125, 9.0, 9.0, 4.0, 9.0],
        ),
        (
            [true, true, true, true, true, true, true, true],
            [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
        ),
        (
            [false, true, false, false, false, false, false, true],
            [9.0, 100.0, 9.0, 9.0, 9.0, 9.0, 9.0, 50.0],
        ),
    ];
    for (flags, values) in &tables {
        // sum
        let sum = run_sum(&mut emu, flags, values);
        let wsum = accumulate_sum(flags, values);
        assert_eq!(sum.to_bits(), wsum.to_bits(), "4e2010 sum");
        total += 1;

        // pick across a sweep of thresholds (incl. just-below / exactly-at each cumulative sum, and
        // a > total threshold so no entry is picked).
        let mut cum = 0.0f32;
        let mut thrs = vec![-1.0f32, 0.0, wsum + 1.0];
        for i in 0..8 {
            if flags[i] {
                // build the exact cumulative via the transcription (matches the binary's PC_53 acc).
                let mut acc = 0.0f64;
                for j in 0..=i {
                    if flags[j] {
                        acc += f64::from(values[j]);
                    }
                }
                cum = acc as f32;
                thrs.push(cum); // exactly-at boundary (thr == acc ⇒ break here)
                thrs.push(f32::from_bits(cum.to_bits().wrapping_sub(1))); // just below
                thrs.push(f32::from_bits(cum.to_bits().wrapping_add(1))); // just above
            }
        }
        let _ = cum;
        for &thr in &thrs {
            let expect = weighted_pick(flags, values, thr);
            run_pick(&mut emu, flags, values, thr, expect);
            total += 1;
        }
    }

    // --- slice 2: (2 - rand_unit(raw)) * sum over deterministic raw/sum pairs + fuzz. ---
    for &(raw, sum) in &[
        (0x0000_0000u32, 10.0f32), // rand_unit = 1.0 ⇒ (2-1)*sum = sum
        (0x007f_ffff, 10.0),       // rand_unit ≈ 2.0 (just under) ⇒ ≈0
        (0x0040_0000, 10.0),       // rand_unit = 1.5 ⇒ 0.5*sum
        (0xdead_beef, 36.0),
        (0x1234_5678, 0.0),
        (0xffff_ffff, 100.0),
    ] {
        assert_eq!(
            run_thr(&mut emu, raw, sum).to_bits(),
            threshold(rand_unit(raw), sum).to_bits(),
            "4e2010 thr raw={raw:#x} sum={sum}"
        );
        total += 1;
    }
    let mut s = 0x4e20_1000u32;
    for _ in 0..3000 {
        let raw = next(&mut s);
        let sum = rng(&mut s, 0.0, 1000.0);
        assert_eq!(
            run_thr(&mut emu, raw, sum).to_bits(),
            threshold(rand_unit(raw), sum).to_bits(),
            "4e2010 thr fuzz raw={raw:#x} sum={sum}"
        );
        total += 1;
    }

    // --- fuzz slice 1 (sum) + slice 3 (pick) over random tables. ---
    let mut s = 0x4e20_10aau32;
    for _ in 0..1500 {
        let mut flags = [false; 8];
        let mut values = [0.0f32; 8];
        for i in 0..8 {
            flags[i] = (next(&mut s) >> 8) & 1 == 1;
            values[i] = rng(&mut s, 0.0, 50.0);
        }
        let sum = run_sum(&mut emu, &flags, &values);
        let wsum = accumulate_sum(&flags, &values);
        assert_eq!(sum.to_bits(), wsum.to_bits(), "4e2010 sum fuzz");
        total += 1;

        let thr = rng(&mut s, -1.0, wsum + 1.0);
        let expect = weighted_pick(&flags, &values, thr);
        run_pick(&mut emu, &flags, &values, thr, expect);
        total += 1;
    }

    eprintln!("glue_4c0_4e2010_weighted_pick: {total} bit-exact (sum ; (2−r)·sum ; weighted-pick fadd+fcomp)");
}
