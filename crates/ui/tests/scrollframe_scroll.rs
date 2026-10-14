//! Differential tests of the arithmetic of `CSimpleScrollFrame::SetHorizontalScroll`
//! (`0x786d30`), `SetVerticalScroll` (`0x786db0`) and `0x786e30`, `0x787140`, `0x787220`, each
//! run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed as an **instruction slice** (`Emu::run`) over the binary's own
//! bytes — registers/memory/stack synthesized, the stored `fstp/fst [m32]` value or the `fnstsw`
//! AH read back. Every in-body FP-arith opcode (the arithmetic opcode set) lands in a diffed slice:
//! - 0x786d30/db0 (clone pair): eps change-detect AH gate + the `g·1024·v` display product (4 ops
//!   each), diffed at BOTH addresses' own bytes (field offsets differ, same kernel).
//! - 0x786e30 (15 ops): the X/Y scale-normalize block (live-vs-reloaded reciprocal, 7 ops), the
//!   dual eps-detect tail (4 ops, two AH gates), and the 2× display product (4 ops).
//! - 0x787140 (2 ops): the rect delta (two subtractions).
//! - 0x787220 (6 ops): the per-axis UV clamp-to-[0,1] with carry (axis A + axis B, 3 ops each).
//!
//! Fuzz reads the LCG's HIGH bits. Every owned-slice test carries a
//! deterministic `==threshold` boundary probe: the eps gates probe the exact `==eps`
//! float; the clamps probe `==0.0`/`==1.0`; the live-vs-reload reciprocal is exercised at `s=3`
//! (1/3 not f32-exact). The eps/clamp branches are reconstructed from the EMITTED jcc, not the
//! mask: `test ah,5` (C0|C2) read by `jp`/`jnp` is a PARITY test (`jp` taken ⇔ even parity);
//! `test ah,0x41` (C0|C3) read by `je`/`jne` is a ZF test (`jne` taken ⇔ masked-AH != 0).
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::scrollframe_scroll::{
    axis_scale_x, axis_scale_y, axis_settled, clamp01_carry, display_product, eps_settled,
    rect_delta, EPS_BITS, ONE_BITS,
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
const EPS_F32: f32 = f32::from_bits(EPS_BITS);
const ONE_F32: f32 = f32::from_bits(ONE_BITS);
/// The display-product getter global `[0x832a4c]` (the `call 0x41ad70` leaf loads it into st0).
const G_SCALE_GLOBAL: u64 = 0x832a4c;

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x400);
    let scratch = emu.alloc(0x100);
    let ebp = scratch + 0x80;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1c00;
    Frame { this, ebp, esp }
}
fn set_frame(emu: &mut Emu, f: &Frame) {
    emu.set_reg(RegisterX86::ESI, f.this);
    emu.set_reg(RegisterX86::ECX, f.this);
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
}

// ===========================================================================================
// 0x786d30 / 0x786db0 — per-channel eps-gated display update (clone pair, diffed at own bytes).
//   eps gate:   fld [esi+field]; (push edi); fsubr [ebp+8]; fabs; fcomp eps; fnstsw  → AH.
//   product:    call 0x41ad70 (fld [0x832a4c]=g); fmul 1024.0; (push ecx); fmul [ebp+8]; fstp [esp].
// `test ah,5; jnp` → SKIP (settled) iff ODD parity = ordered |arg-field| < eps.
// ===========================================================================================
struct DispBlock {
    /// gate slice `[gate_start, gate_end)` — `fld [esi+field_off]` .. `fnstsw ax` (the eps detect).
    gate_start: u64,
    gate_end: u64,
    field_off: u64,
    /// product slice `[prod_start, prod_end)` — `call 0x41ad70` .. `fstp [esp]` (the `g·1024·v`).
    prod_start: u64,
    prod_end: u64,
}
// Literal slice ranges (for the orchestrator FP-op recount): gate `run(0x786d36, 0x786d4a)` /
// `run(0x786db6, 0x786dca)`; product `run(0x786d76, 0x786d88)` / `run(0x786df6, 0x786e08)`.
const DISP_BLOCKS: [(&str, DispBlock); 2] = [
    (
        "0x786d30",
        DispBlock {
            gate_start: 0x786d36,
            gate_end: 0x786d4a,
            field_off: 0x324,
            prod_start: 0x786d76,
            prod_end: 0x786d88,
        },
    ),
    (
        "0x786db0",
        DispBlock {
            gate_start: 0x786db6,
            gate_end: 0x786dca,
            field_off: 0x328,
            prod_start: 0x786df6,
            prod_end: 0x786e08,
        },
    ),
];

#[test]
fn glue_786d30_display() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // eps gate: run [gate_start, gate_end) — fld..fnstsw — read AH. The `push edi`
    // (one byte after fld) decrements ESP; harmless on our synthesized stack.
    let run_gate = |emu: &mut Emu, b: &DispBlock, arg: f32, field: f32| -> u8 {
        set_frame(emu, &f);
        emu.write_u32(f.this + b.field_off, field.to_bits());
        emu.write_u32(f.ebp + 0x8, arg.to_bits());
        emu.run(b.gate_start, b.gate_end); // fld..fnstsw (stop AFTER `fnstsw ax`)
        ah_of(emu)
    };
    // product: run [prod_start, prod_end) — call getter .. fstp [esp]. ESP path: call (push
    // ret/pop ret, net 0) → fmul 1024 → `push ecx` (esp-=4) → fmul [ebp+8] → `fstp [esp]` writes the
    // f32 product to [f.esp - 4]. `v` is read from [ebp+8]; `g` from [0x832a4c].
    let run_prod = |emu: &mut Emu, b: &DispBlock, g: f32, v: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(G_SCALE_GLOBAL, g.to_bits());
        emu.write_u32(f.ebp + 0x8, v.to_bits());
        emu.run(b.prod_start, b.prod_end); // call getter .. fstp [esp]
        emu.read_f32(f.esp - 0x4)
    };

    let mut total = 0u32;
    let mut chg = 0u32;
    let mut settled = 0u32;
    for (name, b) in &DISP_BLOCKS {
        // gate boundary probes: |arg-field| == eps (exact), below, above, NaN.
        for (arg, field, _l) in [
            (EPS_F32, 0.0f32, "==eps"),
            (EPS_F32 * 0.5, 0.0, "below"),
            (EPS_F32 * 2.0, 0.0, "above"),
            (f32::NAN, 0.0, "nan"),
            (5.0, 5.0, "equal"),
        ] {
            let ah = run_gate(&mut emu, b, arg, field);
            // jnp taken (skip/settled) iff EVEN parity of masked {C0,C2}.
            let bin_settled = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(
                bin_settled,
                eps_settled(arg, field),
                "{name} gate arg={arg} field={field}"
            );
            if bin_settled {
                settled += 1;
            } else {
                chg += 1;
            }
            total += 1;
        }
        // product probes incl. a non-f32-exact product chain (g=1/3, v large).
        for (g, v) in [
            (1.0f32, 1.0f32),
            (0.5, 2.0),
            (1.0 / 3.0, 16_777_217.0),
            (-2.5, 3.5),
            (0.0, 1234.5),
        ] {
            let got = run_prod(&mut emu, b, g, v);
            assert_eq!(
                got.to_bits(),
                display_product(g, v).to_bits(),
                "{name} product g={g} v={v}"
            );
            total += 1;
        }
    }

    let mut s = 0x786d_3000u32;
    for (name, b) in &DISP_BLOCKS {
        for _ in 0..2000 {
            let field = rng(&mut s, -10.0, 10.0);
            let arg = field + rng(&mut s, -1.0e-6, 1.0e-6);
            let ah = run_gate(&mut emu, b, arg, field);
            let bin_settled = (ah & 0x5).count_ones() % 2 == 1;
            assert_eq!(bin_settled, eps_settled(arg, field), "{name} gate fuzz");
            if bin_settled {
                settled += 1;
            } else {
                chg += 1;
            }
            let g = rng(&mut s, -1000.0, 1000.0);
            let v = rng(&mut s, -1000.0, 1000.0);
            let got = run_prod(&mut emu, b, g, v);
            assert_eq!(
                got.to_bits(),
                display_product(g, v).to_bits(),
                "{name} product fuzz"
            );
            total += 2;
        }
    }
    eprintln!(
        "glue_786d30_display: {total} bit-exact (eps gate chg={chg} settled={settled}, g·1024·v)"
    );
    assert!(chg > 0 && settled > 0, "need both gate outcomes");
}

// ===========================================================================================
// 0x786e30 scale block 0x786e7e..0x786ed7: X = max(width_x-arg0,0)·(1/s)_LIVE, Y = max(width_y-
// arg1,0)·(1/s)_F32. width = max-min; recip stored f32 to [ebp+8], used live for X (fmulp) and
// reloaded f32 for Y (fmul [ebp+8]). rx → [esi+0x31c], ry → [esi+0x320], recip_f32 → [ebp+8].
// fsub×4 + fdivr + fmulp + fmul = 7 ops.
// ===========================================================================================
#[test]
fn glue_786e30_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // The box cells: max_x=[ebp-0xc], min_x=[ebp-0x14], max_y=[ebp-0x10], min_y=[ebp-0x18];
    // arg0=[ebp+8], arg1=[ebp+0xc]; scale s=[esi+0x7c]. Read rx=[esi+0x31c], ry=[esi+0x320],
    // recip_f32=[ebp+8] (overwritten by `fst [ebp+8]`).
    let run = |emu: &mut Emu,
               max_x: f32,
               min_x: f32,
               max_y: f32,
               min_y: f32,
               arg0: f32,
               arg1: f32,
               s: f32|
     -> (f32, f32, f32) {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0xc, max_x.to_bits());
        emu.write_u32(f.ebp - 0x14, min_x.to_bits());
        emu.write_u32(f.ebp - 0x10, max_y.to_bits());
        emu.write_u32(f.ebp - 0x18, min_y.to_bits());
        emu.write_u32(f.ebp + 0x8, arg0.to_bits());
        emu.write_u32(f.ebp + 0xc, arg1.to_bits());
        emu.write_u32(f.this + 0x7c, s.to_bits());
        emu.run(0x786e7e, 0x786ed7); // X+Y scale block, stop before the eps-detect tail
        (
            emu.read_f32(f.this + 0x31c), // rx
            emu.read_f32(f.this + 0x320), // ry
            emu.read_f32(f.ebp + 0x8),    // recip_f32 (the f32 slot [ebp+8])
        )
    };
    let predict = |max_x, min_x, max_y, min_y, arg0, arg1, s: f32| -> (f32, f32, f32) {
        let (rx, recip_f32) = axis_scale_x(max_x, min_x, arg0, s);
        let ry = axis_scale_y(max_y, min_y, arg1, recip_f32);
        (rx, ry, recip_f32)
    };

    let mut total = 0u32;
    // deterministic probes: s=3 (1/3 not f32-exact → live-vs-reload recip diverges), the clamp
    // boundary width-arg == 0 (c == 0 → keep 0.0, NOT the >0 path), and a negative c → 0.
    for (max_x, min_x, max_y, min_y, arg0, arg1, s) in [
        (10.0f32, 2.0f32, 8.0f32, 1.0f32, 1.0f32, 1.0f32, 3.0f32), // 1/3 live-vs-reload
        (5.0, 5.0, 5.0, 5.0, 0.0, 0.0, 2.0),                       // width=0 ⇒ c=0 (clamp boundary)
        (3.0, 1.0, 3.0, 1.0, 2.0, 2.0, 4.0),                       // c == 0 exactly (keep 0.0)
        (1.0, 5.0, 1.0, 5.0, 0.0, 0.0, 2.0),                       // negative width ⇒ clamp 0
        (100.0, 0.0, 50.0, 0.0, 10.0, 5.0, 7.0),                   // general
    ] {
        let got = run(&mut emu, max_x, min_x, max_y, min_y, arg0, arg1, s);
        let want = predict(max_x, min_x, max_y, min_y, arg0, arg1, s);
        assert_eq!(got.0.to_bits(), want.0.to_bits(), "rx s={s}");
        assert_eq!(got.1.to_bits(), want.1.to_bits(), "ry s={s}");
        assert_eq!(got.2.to_bits(), want.2.to_bits(), "recip s={s}");
        total += 3;
    }
    let mut sd = 0x786e_3000u32;
    for _ in 0..4000 {
        let max_x = rng(&mut sd, -100.0, 100.0);
        let min_x = rng(&mut sd, -100.0, 100.0);
        let max_y = rng(&mut sd, -100.0, 100.0);
        let min_y = rng(&mut sd, -100.0, 100.0);
        let arg0 = rng(&mut sd, -50.0, 50.0);
        let arg1 = rng(&mut sd, -50.0, 50.0);
        // s away from 0 (1/s finite); both signs.
        let s = {
            let v = rng(&mut sd, -20.0, 20.0);
            if v.abs() < 0.25 {
                v.signum() * 0.25 + v
            } else {
                v
            }
        };
        let got = run(&mut emu, max_x, min_x, max_y, min_y, arg0, arg1, s);
        let want = predict(max_x, min_x, max_y, min_y, arg0, arg1, s);
        assert_eq!(got.0.to_bits(), want.0.to_bits(), "rx fuzz");
        assert_eq!(got.1.to_bits(), want.1.to_bits(), "ry fuzz");
        assert_eq!(got.2.to_bits(), want.2.to_bits(), "recip fuzz");
        total += 3;
    }
    eprintln!("glue_786e30_scale: {total} bit-exact (X live-recip / Y f32-recip, max(w-arg,0))");
}

// ===========================================================================================
// 0x786e30 eps-detect tail 0x786ed7..0x786efd: with rx,ry live on the stack and old0,old1 in
// [ebp-0x8]/[ebp-0x4], two gates: |rx-old0|<eps (fsubr st,st(2); fabs; fcomp eps; test ah,5; jp),
// then |ry-old1|<eps (fsubr st,st(1); fabs; fcomp eps; test ah,5; jnp). fsubr×2+fabs×2 = 4 ops.
// Settled (binary SKIPS) iff BOTH within eps. The tail needs rx (st2), ry (st1) live — we re-run
// the scale block 0x786e7e..0x786efd in one go so the stack is the binary's own.
// ===========================================================================================
/// One settled-probe: a scale config (box extents + args + scale) plus the prior values
/// `old0`/`old1` and the predicted `rx`/`ry`.
#[derive(Clone, Copy)]
struct SettledProbe {
    max_x: f32,
    min_x: f32,
    max_y: f32,
    min_y: f32,
    arg0: f32,
    arg1: f32,
    s: f32,
    old0: f32,
    old1: f32,
    rx: f32,
    ry: f32,
}

#[test]
fn glue_786e30_settled() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Run the scale block THROUGH each eps gate's `fnstsw`+`test` to its `jcc` and read AH. Both
    // gates' AH must be captured; the first gate `jp 0x786eff` (taken ⇒ NOT settled on X, skips the
    // Y gate). We run the full block twice with split `until`, each halt at the gate's `jcc`
    //   stop1 = 0x786ee9 (the first `jp` — a basic-block boundary; `fnstsw`+`test ah,5` at
    //           0x786ee4/0x786ee6 have run, so AH is final). Halting at the BRANCH (not mid-block at
    //           0x786ee6) avoids stale-TB FETCH_UNMAPPED on re-entry of 0x786e7e across cases.
    //   stop2 = 0x786efd (the second `jnp` — bb boundary; second `fnstsw`+`test` have run) → AH_y
    //           (reached only when the first gate fell through, i.e. |rx-old0|<eps).
    let run = |emu: &mut Emu, p: &SettledProbe, until: u64| -> u8 {
        let (max_x, min_x, max_y, min_y, arg0, arg1, s, old0, old1) = (
            p.max_x, p.min_x, p.max_y, p.min_y, p.arg0, p.arg1, p.s, p.old0, p.old1,
        );
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0xc, max_x.to_bits());
        emu.write_u32(f.ebp - 0x14, min_x.to_bits());
        emu.write_u32(f.ebp - 0x10, max_y.to_bits());
        emu.write_u32(f.ebp - 0x18, min_y.to_bits());
        emu.write_u32(f.ebp + 0x8, arg0.to_bits());
        emu.write_u32(f.ebp + 0xc, arg1.to_bits());
        emu.write_u32(f.this + 0x7c, s.to_bits());
        emu.write_u32(f.ebp - 0x8, old0.to_bits());
        emu.write_u32(f.ebp - 0x4, old1.to_bits());
        // The two literal slice ranges (recount anchor): run(0x786e7e, 0x786ee9) [first eps gate],
        // run(0x786e7e, 0x786efd) [scale block + both eps gates — covers fsubr/fabs ×2 at
        // 0x786eda/0x786edc/0x786eee/0x786ef0].
        emu.run(0x786e7e, until);
        ah_of(emu)
    };

    let mut total = 0u32;
    let mut settled = 0u32;
    let mut moved = 0u32;
    // Choose configs spanning: both within eps, X out, Y out, exact ==eps boundaries.
    // rx = max(max_x-min_x-arg0,0)·(1/s); pick old0/old1 to straddle eps around rx/ry.
    let cases: [(f32, f32, f32, f32, f32, f32, f32); 6] = [
        (10.0, 2.0, 8.0, 1.0, 1.0, 1.0, 1.0), // s=1 ⇒ rx=7, ry=6
        (10.0, 2.0, 8.0, 1.0, 1.0, 1.0, 2.0), // s=2
        (4.0, 0.0, 4.0, 0.0, 0.0, 0.0, 4.0),
        (4.0, 0.0, 4.0, 0.0, 0.0, 0.0, 1.0),
        (5.0, 5.0, 5.0, 5.0, 0.0, 0.0, 1.0), // rx=ry=0 (clamp boundary)
        (20.0, 0.0, 10.0, 0.0, 0.0, 0.0, 3.0),
    ];
    // Build the full probe list (config + old0/old1) once. stale-TB: re-entering 0x786e7e with a
    // SHORTER `until` after a LONGER one fetches a stale TB → FETCH_UNMAPPED. far→near is unsafe;
    // near→far and same-`until` repeats are safe. So we run ALL X-gate reads (near `until=0x786ee9`)
    // in one pass, THEN all Y-gate reads (far `until=0x786efd`) in a second pass — never far-then-
    // near from the same start. x_settled[k] gates whether probe k's Y-gate decides settledness.
    let mut probes: Vec<SettledProbe> = Vec::new();
    for (max_x, min_x, max_y, min_y, arg0, arg1, s) in cases {
        let (rx, recip) = axis_scale_x(max_x, min_x, arg0, s);
        let ry = axis_scale_y(max_y, min_y, arg1, recip);
        for (old0, old1) in [
            (rx, ry),                 // both within eps → settled
            (rx + EPS_F32, ry),       // |dx|≈eps boundary
            (rx + EPS_F32 * 2.0, ry), // X moved
            (rx, ry + EPS_F32 * 2.0), // Y moved (X settled)
            (rx + 1.0, ry + 1.0),     // both moved
            (f32::NAN, ry),           // unordered X
        ] {
            probes.push(SettledProbe {
                max_x,
                min_x,
                max_y,
                min_y,
                arg0,
                arg1,
                s,
                old0,
                old1,
                rx,
                ry,
            });
        }
    }
    // pass 1: X-gate (near). `jp 0x786eff` taken (→ work, X not settled) iff EVEN parity.
    let mut x_settled: Vec<bool> = Vec::with_capacity(probes.len());
    for p in &probes {
        let ah_x = run(&mut emu, p, 0x786ee9);
        x_settled.push((ah_x & 0x5).count_ones() % 2 == 1); // odd = |dx|<eps = fall through
    }
    // pass 2: Y-gate (far) — only meaningful when X settled (else the binary jumped past it).
    for (k, p) in probes.iter().enumerate() {
        let bin_settled = if !x_settled[k] {
            false
        } else {
            let ah_y = run(&mut emu, p, 0x786efd);
            (ah_y & 0x5).count_ones() % 2 == 1 // `jnp 0x786f66` taken (SKIP/settled) iff ODD
        };
        assert_eq!(
            bin_settled,
            axis_settled(p.rx, p.ry, p.old0, p.old1),
            "settled rx={} ry={} old0={} old1={}",
            p.rx,
            p.ry,
            p.old0,
            p.old1
        );
        if bin_settled {
            settled += 1;
        } else {
            moved += 1;
        }
        total += 1;
    }
    eprintln!("glue_786e30_settled: {total} bit-exact (dual-eps settled={settled} moved={moved})");
    assert!(settled > 0 && moved > 0, "need both settled outcomes");
}

// ===========================================================================================
// 0x786e30 display product 0x786f0f..0x786f44: fstp [ebp+8]=ry; fstp [ebp+0xc]=rx; then 2×
// (call 0x41ad70=g; fmul 1024.0; push; fmul [ebp+8]/[ebp+0xc]; fstp [esp]; call 0x41ae40 ...).
// The owned fmuls are the SAME `g·1024·v` product as 0x786d30. We diff each product instance
// (prod0 = g·1024·ry over [ebp+8], prod1 = g·1024·rx over [ebp+0xc]) at its own bytes, ending
// each slice at its `fstp [esp]` (before the `call 0x41ae40` unit-convert boundary).
// ===========================================================================================
#[test]
fn glue_786e30_product() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // prod0: call 0x41ad70 (0x786f15) .. fstp [esp] — slice `run(0x786f15, 0x786f27)`, reads [ebp+8];
    // prod1: `run(0x786f32, 0x786f44)`, reads [ebp+0xc]. ESP path = 0x786d30 (call net-0, push, fstp).
    let run_prod = |emu: &mut Emu, start: u64, end: u64, vslot: u64, g: f32, v: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(G_SCALE_GLOBAL, g.to_bits());
        emu.write_u32(f.ebp + vslot, v.to_bits());
        emu.run(start, end); // call getter .. fstp [esp] inclusive
        emu.read_f32(f.esp - 0x4)
    };

    let mut total = 0u32;
    for (start, end, vslot) in [
        (0x786f15u64, 0x786f27u64, 0x8u64),
        (0x786f32, 0x786f44, 0xc),
    ] {
        for (g, v) in [
            (1.0f32, 1.0f32),
            (0.5, 2.0),
            (1.0 / 3.0, 16_777_217.0),
            (-2.5, 3.5),
            (0.0, 1234.5),
        ] {
            let got = run_prod(&mut emu, start, end, vslot, g, v);
            assert_eq!(
                got.to_bits(),
                display_product(g, v).to_bits(),
                "product @ {start:#x} g={g} v={v}"
            );
            total += 1;
        }
        let mut s = 0x786f_0000u32 ^ (start as u32);
        for _ in 0..2000 {
            let g = rng(&mut s, -1000.0, 1000.0);
            let v = rng(&mut s, -1000.0, 1000.0);
            let got = run_prod(&mut emu, start, end, vslot, g, v);
            assert_eq!(
                got.to_bits(),
                display_product(g, v).to_bits(),
                "product @ {start:#x} fuzz"
            );
            total += 1;
        }
    }
    eprintln!("glue_786e30_product: {total} bit-exact (2× g·1024·v over [ebp+8]/[ebp+0xc])");
}

// ===========================================================================================
// 0x787140 rect delta 0x787188..0x78719d: dy = buf2-buf0 (fld [ebp-0x8]; fsub [ebp-0x10]; fstp
// [esp+4]); dx = buf3-buf1 (fld [ebp-0x4]; fsub [ebp-0xc]; fstp [esp]). fsub×2. The `mov ecx,esi`
// and `sub esp,8` between are stack/reg setup; the two fstp land at [esp+4] and [esp].
// ===========================================================================================
#[test]
fn glue_787140_rect_delta() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // buf = [ebp-0x10]/[ebp-0xc]/[ebp-0x8]/[ebp-0x4]. Slice 0x787188..0x78719d ends right after
    // the second `fstp [esp]` (0x78719d is the fstp; stop at 0x7871a0, the following `call`).
    // ESP path: entry esp=E. `sub esp,8` (0x78718b) → E-8. `fstp [esp+4]` → [E-4]; `fstp [esp]` →
    // [E-8]. So dy=[E-4], dx=[E-8].
    let run = |emu: &mut Emu, buf0: f32, buf1: f32, buf2: f32, buf3: f32| -> (f32, f32) {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0x10, buf0.to_bits());
        emu.write_u32(f.ebp - 0xc, buf1.to_bits());
        emu.write_u32(f.ebp - 0x8, buf2.to_bits());
        emu.write_u32(f.ebp - 0x4, buf3.to_bits());
        emu.run(0x787188, 0x7871a0); // fld dy .. fstp dx, stop at the `call 0x786e30`
        let esp = emu.get_reg(RegisterX86::ESP);
        (emu.read_f32(esp + 4), emu.read_f32(esp)) // (dy, dx)
    };

    let mut total = 0u32;
    for (b0, b1, b2, b3) in [
        (0.0f32, 0.0f32, 1.0f32, 1.0f32),
        (2.0, 3.0, 10.0, 7.0),
        (-5.0, -2.0, 5.0, 2.0),
        (1.0, 1.0, 1.0, 1.0),          // dy=dx=0
        (16_777_217.0, 0.0, 0.0, 1.0), // f32-precision edge
    ] {
        let (dy, dx) = run(&mut emu, b0, b1, b2, b3);
        let (wdy, wdx) = rect_delta(b0, b1, b2, b3);
        assert_eq!(dy.to_bits(), wdy.to_bits(), "dy b2={b2} b0={b0}");
        assert_eq!(dx.to_bits(), wdx.to_bits(), "dx b3={b3} b1={b1}");
        total += 2;
    }
    let mut s = 0x7871_4000u32;
    for _ in 0..3000 {
        let b0 = rng(&mut s, -1000.0, 1000.0);
        let b1 = rng(&mut s, -1000.0, 1000.0);
        let b2 = rng(&mut s, -1000.0, 1000.0);
        let b3 = rng(&mut s, -1000.0, 1000.0);
        let (dy, dx) = run(&mut emu, b0, b1, b2, b3);
        let (wdy, wdx) = rect_delta(b0, b1, b2, b3);
        assert_eq!(dy.to_bits(), wdy.to_bits(), "dy fuzz");
        assert_eq!(dx.to_bits(), wdx.to_bits(), "dx fuzz");
        total += 2;
    }
    eprintln!("glue_787140_rect_delta: {total} bit-exact (buf2-buf0, buf3-buf1)");
}

// ===========================================================================================
// 0x787220 UV clamp-to-[0,1] with carry. Per-axis: lower clip (lo<0 ⇒ hi-=lo, lo=0) then upper
// clip (hi>1 ⇒ lo-=(hi-1), hi=1). axis A: lo=[ebp-0xc], hi=[ebp-0x4]; axis B: lo=[ebp-0x10],
// hi=[ebp-0x8]. The binary runs A then B sequentially — axis A's tail (0x7874c8) IS axis B's first
// instruction — so we diff BOTH axes in ONE slice `0x787482..0x78750e` (the whole clamp block),
// reading all four endpoints. a single fixed (start,until) per `run` avoids the far→near
// stale-TB FETCH/READ_UNMAPPED that splitting the two overlapping axis blocks would trigger.
// fsub+fsub+fsubr per axis = 6 ops total.
// ===========================================================================================
#[test]
fn glue_787220_clamp01() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let slot = |off: i64| -> u64 { (f.ebp as i64 + off) as u64 };
    // Run the whole clamp block over both axes: A=(lo[ebp-0xc], hi[ebp-0x4]),
    // B=(lo[ebp-0x10], hi[ebp-0x8]). Returns ((lo_a,hi_a),(lo_b,hi_b)) after clamping.
    let run =
        |emu: &mut Emu, lo_a: f32, hi_a: f32, lo_b: f32, hi_b: f32| -> ((f32, f32), (f32, f32)) {
            set_frame(emu, &f);
            emu.write_u32(slot(-0xc), lo_a.to_bits());
            emu.write_u32(slot(-0x4), hi_a.to_bits());
            emu.write_u32(slot(-0x10), lo_b.to_bits());
            emu.write_u32(slot(-0x8), hi_b.to_bits());
            emu.run(0x787482, 0x78750e); // both axes (A then B) in one slice
            (
                (emu.read_f32(slot(-0xc)), emu.read_f32(slot(-0x4))),
                (emu.read_f32(slot(-0x10)), emu.read_f32(slot(-0x8))),
            )
        };

    let mut total = 0u32;
    let mut lower = 0u32;
    let mut upper = 0u32;
    let mut both = 0u32;
    // boundary probes: lo==0 / hi==1 exactly (no clip), lo just below 0, hi just above 1, both,
    // NaN on each endpoint. Each probe drives axis A; axis B is given an independent in-range pair
    // so it exits cleanly (its own clip outcomes are exercised by the fuzz below).
    let probes: [(f32, f32, &str); 10] = [
        (0.0, 1.0, "exact [0,1]"),
        (0.0, 0.5, "in range"),
        (-0.25, 0.5, "lower only"),
        (0.25, 1.5, "upper only"),
        (-0.25, 1.5, "both"),
        (-1e-7, 1.0 + 1e-7, "tiny over"),
        (-0.5, 2.0, "wide"),
        (f32::NAN, 0.5, "lo nan"),
        (0.5, f32::NAN, "hi nan"),
        (0.0, 1.0 + f32::EPSILON, "hi just over 1"),
    ];
    // pair probes for both axes: axis A gets probe[i], axis B gets probe[len-1-i] (so B also sees
    // every boundary case across the probe set).
    for i in 0..probes.len() {
        let (lo_a, hi_a, _la) = probes[i];
        let (lo_b, hi_b, _lb) = probes[probes.len() - 1 - i];
        let (ga, gb) = run(&mut emu, lo_a, hi_a, lo_b, hi_b);
        let wa = clamp01_carry(lo_a, hi_a);
        let wb = clamp01_carry(lo_b, hi_b);
        assert_eq!(ga.0.to_bits(), wa.0.to_bits(), "A lo lo={lo_a} hi={hi_a}");
        assert_eq!(ga.1.to_bits(), wa.1.to_bits(), "A hi lo={lo_a} hi={hi_a}");
        assert_eq!(gb.0.to_bits(), wb.0.to_bits(), "B lo lo={lo_b} hi={hi_b}");
        assert_eq!(gb.1.to_bits(), wb.1.to_bits(), "B hi lo={lo_b} hi={hi_b}");
        for (lo, hi) in [(lo_a, hi_a), (lo_b, hi_b)] {
            if lo < 0.0 {
                lower += 1;
            }
            if hi > 1.0 {
                upper += 1;
            }
            if lo < 0.0 && hi > 1.0 {
                both += 1;
            }
        }
        total += 4;
    }
    let mut s = 0x7872_2000u32;
    for _ in 0..4000 {
        let lo_a = rng(&mut s, -1.5, 1.5);
        let hi_a = rng(&mut s, -0.5, 2.5);
        let lo_b = rng(&mut s, -1.5, 1.5);
        let hi_b = rng(&mut s, -0.5, 2.5);
        let (ga, gb) = run(&mut emu, lo_a, hi_a, lo_b, hi_b);
        let wa = clamp01_carry(lo_a, hi_a);
        let wb = clamp01_carry(lo_b, hi_b);
        assert_eq!(ga.0.to_bits(), wa.0.to_bits(), "A lo fuzz");
        assert_eq!(ga.1.to_bits(), wa.1.to_bits(), "A hi fuzz");
        assert_eq!(gb.0.to_bits(), wb.0.to_bits(), "B lo fuzz");
        assert_eq!(gb.1.to_bits(), wb.1.to_bits(), "B hi fuzz");
        for (lo, hi) in [(lo_a, hi_a), (lo_b, hi_b)] {
            if lo < 0.0 {
                lower += 1;
            }
            if hi > 1.0 {
                upper += 1;
            }
            if lo < 0.0 && hi > 1.0 {
                both += 1;
            }
        }
        total += 4;
    }
    // ensure ONE_F32 (the clamp ceiling) is referenced so the const stays load-bearing in evidence.
    assert_eq!(ONE_F32, 1.0);
    eprintln!(
        "glue_787220_clamp01: {total} bit-exact (clamp-to-[0,1] carry; lower={lower} upper={upper} both={both})"
    );
    assert!(lower > 0 && upper > 0 && both > 0, "need all clip outcomes");
}
