//! Differential tests of the arithmetic of the rectangle-change tests and fade functions
//! `0x788310`, `0x788460`, `0x78a320` and `0x78adb0`, each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed as an **instruction slice** (`Emu::run`) — registers/memory/stack
//! synthesized, the binary's own bytes executed, the stored `fst/fstp [m32]` value, the `__ftol`
//! EAX byte, or the `fnstsw` AH branch read back. Every in-body FP-arith opcode lands in a diffed slice; the `fild`/loads/stores/compares ride along inside the
//! slices for free. The `__ftol` callee (`0x40a2b0`) runs NATIVELY inside the alpha slice (pure
//! x87, truncate toward zero). Where the binary only *compares* a computed magnitude (never stores
//! it), the magnitude itself is ALSO captured bit-exact via a COPIED-BYTE ISLAND (the exact source
//! `fld..fabs` bytes, byte-verified vs source, with an appended `fstp qword [cell]` to read the
//! full PC_53 `st0` as f64) — so both the arithmetic value and the branch outcome are proven.
//! Fuzz reads the LCG's HIGH bits. Each threshold compare carries
//! deterministic `==`-boundary probes straddling `eps`/`0.0`.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::frame_rect_change::{
    alpha_step, decrement_field_0c, dx_magnitude, dx_magnitude_78a320, dy_magnitude,
    dy_magnitude_78a320, negate_stp, parity_even, recompute_edges, rect_changed_788310,
    rect_changed_78a320, EPS_BITS, SCALE255_BITS,
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
    esi: u64,
    edi: u64,
    ebx: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    let esi = emu.alloc(0x600); // object `this` for the [esi+...] derefs
    let edi = emu.alloc(0x200); // rect block for 0x78a320's [edi+...] derefs
    let ebx = emu.alloc(0x600); // 0x788460 reads [ebx+0x368] (divisor)
    Frame {
        ebp,
        esp,
        esi,
        edi,
        ebx,
    }
}

// ===========================================================================================
// 0x788310 — "rect changed?" predicate. Slice (a) 0x788321..0x78839c recomputes the 4 edges into
// [esi+0x370/378/36c/374] and runs the X |Δ| compare (read the 4 stored f32 edges + the X AH);
// slice (b) 0x7883a1..0x7883ba runs the Y |Δ| compare on those edges + [ebp-4]=old_dy (read AH).
// arithmetic covered by (a): fsub 0x788327/333, fmul 0x788342/354/366/378, fadd 0x788345/369,
// fsubr 0x788357/37b, fsub 0x78838a, fsubp 0x788390, fabs 0x788392.  by (b): fsub 0x7883a7,
// fsubr 0x7883ad, fabs 0x7883b0.  Magnitudes ALSO captured via f64 islands (below).
// ===========================================================================================
#[test]
fn glue_788_788310_rect_changed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // object-field offsets (esi-relative).
    const O370: u64 = 0x370;
    const O378: u64 = 0x378;
    const O36C: u64 = 0x36c;
    const O374: u64 = 0x374;
    const F0: u64 = 0x380; // factor for e370
    const F1: u64 = 0x388; // factor for e378
    const F2: u64 = 0x37c; // factor for e36c
    const F3: u64 = 0x384; // factor for e374
    const S: u64 = 0x7c; // shared scale
    const OFF68: u64 = 0x68;
    const OFF70: u64 = 0x70;
    const OFF64: u64 = 0x64;
    const OFF6C: u64 = 0x6c;

    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    let setup = |emu: &mut Emu,
                 o370: f32,
                 o378: f32,
                 o36c: f32,
                 o374: f32,
                 f0: f32,
                 f1: f32,
                 f2: f32,
                 f3: f32,
                 s: f32,
                 a68: f32,
                 a70: f32,
                 a64: f32,
                 a6c: f32| {
        emu.write_u32(f.esi + O370, o370.to_bits());
        emu.write_u32(f.esi + O378, o378.to_bits());
        emu.write_u32(f.esi + O36C, o36c.to_bits());
        emu.write_u32(f.esi + O374, o374.to_bits());
        emu.write_u32(f.esi + F0, f0.to_bits());
        emu.write_u32(f.esi + F1, f1.to_bits());
        emu.write_u32(f.esi + F2, f2.to_bits());
        emu.write_u32(f.esi + F3, f3.to_bits());
        emu.write_u32(f.esi + S, s.to_bits());
        emu.write_u32(f.esi + OFF68, a68.to_bits());
        emu.write_u32(f.esi + OFF70, a70.to_bits());
        emu.write_u32(f.esi + OFF64, a64.to_bits());
        emu.write_u32(f.esi + OFF6C, a6c.to_bits());
    };

    // run slice (a): recompute + X compare. returns (e370,e378,e36c,e374, x_ah, old_dy@[ebp-4]).
    #[allow(clippy::too_many_arguments)]
    let run_x = |emu: &mut Emu| -> (f32, f32, f32, f32, u8) {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x788321, 0x78839c); // halt at `test ah,5` (X compare)
        let ah = ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8;
        (
            emu.read_f32(f.esi + O370),
            emu.read_f32(f.esi + O378),
            emu.read_f32(f.esi + O36C),
            emu.read_f32(f.esi + O374),
            ah,
        )
    };
    // run slice (b): Y compare. reads NEW [esi+0x374]/[esi+0x36c] and [ebp-4]=old_dy. returns AH.
    let run_y = |emu: &mut Emu| -> u8 {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x7883a1, 0x7883ba); // halt at `test ah,5` (Y compare)
        ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
    };

    // ---- f64-magnitude COPIED-BYTE ISLANDS (the binary never stores |Δ|, only compares it) ----
    // Island captures the FULL PC_53 st0 |Δ| as f64 via an appended `fstp qword [cell]`.
    // X-island: the whole chain 0x788321..0x788392 (old_dx live across recompute, fsubp, fabs)
    //   then `fstp qword [cellx]`. All derefs are [esi+...]/[ebp-4] (register-relative) + nothing
    //   absolute, so it runs on synthesized esi/ebp exactly like the source.
    let cellx = emu.alloc(8);
    let islx = emu.alloc(0x80);
    let xbytes = emu.read(0x788321, 0x788394 - 0x788321); // fld..(through fabs)
    let mut cx = xbytes.clone();
    cx.extend_from_slice(&[0xddu8, 0x1d]); // fstp qword [disp32]
    cx.extend_from_slice(&(cellx as u32).to_le_bytes());
    emu.write(islx, &cx);
    let islx_end = islx + cx.len() as u64;
    // byte-verify the island's FP body matches the live source span.
    assert_eq!(
        emu.read(islx, xbytes.len()),
        emu.read(0x788321, xbytes.len()),
        "X island body != source"
    );
    // Y-island: 0x7883a1..0x7883b0 (fld[374];fsub[36c];fsubr[ebp-4];fabs) then `fstp qword [celly]`.
    // [ebp-4] must hold old_dy first (we seed it from the X-slice run before invoking the island).
    let celly = emu.alloc(8);
    let isly = emu.alloc(0x40);
    let ybytes = emu.read(0x7883a1, 0x7883b2 - 0x7883a1); // fld..(through fabs)
    let mut cy = ybytes.clone();
    cy.extend_from_slice(&[0xddu8, 0x1d]);
    cy.extend_from_slice(&(celly as u32).to_le_bytes());
    emu.write(isly, &cy);
    let isly_end = isly + cy.len() as u64;
    assert_eq!(
        emu.read(isly, ybytes.len()),
        emu.read(0x7883a1, ybytes.len()),
        "Y island body != source"
    );

    let mut total = 0u32;
    let mut changed_n = 0u32;
    let mut clean_n = 0u32;

    #[allow(clippy::too_many_arguments)]
    let one = |emu: &mut Emu,
               o370: f32,
               o378: f32,
               o36c: f32,
               o374: f32,
               f0: f32,
               f1: f32,
               f2: f32,
               f3: f32,
               s: f32,
               a68: f32,
               a70: f32,
               a64: f32,
               a6c: f32,
               total: &mut u32,
               changed_n: &mut u32,
               clean_n: &mut u32| {
        let (we370, we378, we36c, we374) = recompute_edges(f0, f1, f2, f3, s, a68, a70, a64, a6c);
        // X magnitude via island FIRST (it reads the OLD [esi] edges for old_dx, then recomputes
        // and overwrites them — so it must run on a fresh `setup()` before run_x does the same).
        setup(
            emu, o370, o378, o36c, o374, f0, f1, f2, f3, s, a68, a70, a64, a6c,
        );
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(islx, islx_end);
        let bin_dx = f64::from_bits(u64::from_le_bytes(emu.read(cellx, 8).try_into().unwrap()));
        let my_dx = dx_magnitude(o378, o370, we378, we370);
        assert_eq!(bin_dx.to_bits(), my_dx.to_bits(), "dx magnitude");
        // re-setup the OLD edges, then run_x (recompute + X compare); read edges + X AH.
        setup(
            emu, o370, o378, o36c, o374, f0, f1, f2, f3, s, a68, a70, a64, a6c,
        );
        let (e370, e378, e36c, e374, x_ah) = run_x(emu);
        // edges bit-exact
        assert_eq!(e370.to_bits(), we370.to_bits(), "e370");
        assert_eq!(e378.to_bits(), we378.to_bits(), "e378");
        assert_eq!(e36c.to_bits(), we36c.to_bits(), "e36c");
        assert_eq!(e374.to_bits(), we374.to_bits(), "e374");
        // X branch parity.
        let x_changed = parity_even(x_ah, 0x5);
        // Y compare (binary reads the NEW edges left in [esi+...] + [ebp-4]=old_dy from run_x).
        let y_ah = run_y(emu);
        let y_changed = parity_even(y_ah, 0x5);
        // Y magnitude via island (uses the same [esi+0x374/36c] new edges and [ebp-4]=old_dy).
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(isly, isly_end);
        let bin_dy = f64::from_bits(u64::from_le_bytes(emu.read(celly, 8).try_into().unwrap()));
        let old_dy = (f64::from(o374) - f64::from(o36c)) as f32;
        let my_dy = dy_magnitude(old_dy, e374, e36c);
        assert_eq!(bin_dy.to_bits(), my_dy.to_bits(), "dy magnitude");
        // combined predicate: changed = X-jp OR Y-jp.
        let bin_changed = x_changed || y_changed;
        let (my_changed, my_edges) = rect_changed_788310(
            o370, o378, o36c, o374, f0, f1, f2, f3, s, a68, a70, a64, a6c,
        );
        assert_eq!(bin_changed, my_changed, "788310 changed");
        assert_eq!(my_edges.0.to_bits(), we370.to_bits());
        if bin_changed {
            *changed_n += 1;
        } else {
            *clean_n += 1;
        }
        *total += 6;
    };

    // deterministic clean (no move): factors=0, s=0 -> recompute = pure offsets; seed old edges to
    // those exact integer offsets so every |Δ|=0 (the f32-spilled old_dy is an integer = exact).
    {
        let (e370, e378, e36c, e374) =
            recompute_edges(0.0, 0.0, 0.0, 0.0, 0.0, 10.0, 110.0, 20.0, 220.0);
        one(
            &mut emu,
            e370,
            e378,
            e36c,
            e374,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            10.0,
            110.0,
            20.0,
            220.0,
            &mut total,
            &mut changed_n,
            &mut clean_n,
        );
    }
    one(
        &mut emu,
        0.0,
        100.0,
        0.0,
        200.0,
        0.5,
        -0.5,
        0.25,
        -0.25,
        2.0,
        5.0,
        95.0,
        7.0,
        193.0,
        &mut total,
        &mut changed_n,
        &mut clean_n,
    );
    // eps boundary on X: choose old edges so old_dx - new_dx == ±eps exactly.
    {
        let eps = f64::from(f32::from_bits(EPS_BITS));
        let s = 1.0f32;
        let (f0, f1) = (3.0f32, 1.0f32);
        let (a68, a70) = (0.0f32, 100.0f32);
        // new_dx = (a70 - f1*s) - (f0*s + a68) = (100-1) - (3+0) = 96
        let new_dx = (f64::from(a70) - f64::from(f1) * f64::from(s))
            - (f64::from(f0) * f64::from(s) + f64::from(a68));
        for d in [eps, eps * 0.5, eps * 2.0, -eps, -eps * 0.5] {
            // want old_dx = new_dx + d via o378 - o370.
            let o370 = 0.0f32;
            let o378 = (new_dx + d) as f32;
            one(
                &mut emu,
                o370,
                o378,
                0.0,
                50.0,
                f0,
                f1,
                1.0,
                1.0,
                s,
                a68,
                a70,
                0.0,
                50.0,
                &mut total,
                &mut changed_n,
                &mut clean_n,
            );
        }
    }

    let mut sd = 0x7883_1000u32;
    for _ in 0..1200 {
        let f0 = rng(&mut sd, -3.0, 3.0);
        let f1 = rng(&mut sd, -3.0, 3.0);
        let f2 = rng(&mut sd, -3.0, 3.0);
        let f3 = rng(&mut sd, -3.0, 3.0);
        let s = rng(&mut sd, -2.0, 2.0);
        let a68 = rng(&mut sd, -500.0, 500.0);
        let a70 = rng(&mut sd, -500.0, 500.0);
        let a64 = rng(&mut sd, -500.0, 500.0);
        let a6c = rng(&mut sd, -500.0, 500.0);
        // half the cases: seed the OLD edges to EXACTLY the recompute output (|Δ|=0 -> clean);
        // the other half: independent random old edges (almost always changed). This straddles
        // the predicate so both branches fire.
        let clean = (next(&mut sd) >> 8) & 1 == 0;
        let (o370, o378, o36c, o374) = if clean {
            recompute_edges(f0, f1, f2, f3, s, a68, a70, a64, a6c)
        } else {
            (
                rng(&mut sd, -500.0, 500.0),
                rng(&mut sd, -500.0, 500.0),
                rng(&mut sd, -500.0, 500.0),
                rng(&mut sd, -500.0, 500.0),
            )
        };
        one(
            &mut emu,
            o370,
            o378,
            o36c,
            o374,
            f0,
            f1,
            f2,
            f3,
            s,
            a68,
            a70,
            a64,
            a6c,
            &mut total,
            &mut changed_n,
            &mut clean_n,
        );
    }
    eprintln!(
        "glue_788_788310_rect_changed: {total} bit-exact (edges+|Δx|+|Δy|+predicate; changed={changed_n} clean={clean_n})"
    );
    assert!(changed_n > 0 && clean_n > 0, "need both predicate branches");
}

// ===========================================================================================
// 0x788460 — per-item fade/alpha loop step. Slice (a) 0x7884d1..0x7884da: fld[esi+0xc];
// fsub[ebp+8];fst[esi+0xc] (read [esi+0xc]). Slice (b) 0x788510..0x788558: the alpha leg
// fld[esi+0x10];fsub[ebp+8];fcom;jp;fst[esi+0x10];fdiv[ebx+0x368];fmul[255];call __ftol (native
// truncate); read AL + stored [esi+0x10]. Slice (c) 0x788510..0x788521: the fcom-0 branch (read AH).
// arithmetic: fsub 0x7884d4 (a); fsub 0x788513, fdiv 0x788547, fmul 0x78854d (b).
// ===========================================================================================
#[test]
fn glue_788_788460_alpha_step() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (a) decrement field 0xc.
    let run_dec = |emu: &mut Emu, field_0c: f32, step: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.esi + 0xc, field_0c.to_bits());
        emu.write_u32(f.ebp + 0x8, step.to_bits());
        emu.run(0x7884d1, 0x7884da); // fld;fsub;fst — halt at the fcomp
        emu.read_f32(f.esi + 0xc)
    };
    // (b) alpha leg (requires field0x10 - step >= 0 so the `jp 0x788544` is taken).
    let run_alpha = |emu: &mut Emu, field_10: f32, step: f32, divisor: f32| -> (f32, u8) {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::EBX, f.ebx);
        emu.write_u32(f.esi + 0x10, field_10.to_bits());
        emu.write_u32(f.ebp + 0x8, step.to_bits());
        emu.write_u32(f.ebx + 0x368, divisor.to_bits());
        emu.run(0x788510, 0x788558); // through fdiv,fmul,__ftol — halt at `mov [esi+7],al`
        (
            emu.read_f32(f.esi + 0x10),
            (emu.get_reg(RegisterX86::EAX) & 0xff) as u8,
        )
    };
    // (c) the `(field0x10 - step) cmp 0` branch (fld;fsub;fcom;fnstsw;test ah,5 — halt at jp).
    let run_cmp = |emu: &mut Emu, field_10: f32, step: f32| -> u8 {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.write_u32(f.esi + 0x10, field_10.to_bits());
        emu.write_u32(f.ebp + 0x8, step.to_bits());
        emu.run(0x788510, 0x78851e); // fld;fsub;fcom;fnstsw — halt at `test ah,5`
        ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
    };

    let mut total = 0u32;

    // ---- (a) decrement field 0xc ----
    for (fc, st) in [
        (1.0f32, 0.1),
        (0.0, 0.0),
        (255.0, 1.0),
        (-3.5, 2.25),
        (0.05, 0.05),
        (1.0e-9, 0.0),
    ] {
        assert_eq!(
            run_dec(&mut emu, fc, st).to_bits(),
            decrement_field_0c(fc, st).to_bits(),
            "dec field0xc fc={fc} st={st}"
        );
        total += 1;
    }
    let mut sa = 0x7884_d100u32;
    for _ in 0..3000 {
        let fc = rng(&mut sa, -1000.0, 1000.0);
        let st = rng(&mut sa, -50.0, 50.0);
        assert_eq!(
            run_dec(&mut emu, fc, st).to_bits(),
            decrement_field_0c(fc, st).to_bits(),
            "dec field0xc fuzz fc={fc} st={st}"
        );
        total += 1;
    }

    // ---- (b) THRESHOLD PROBE: (field0x10 - step) vs 0.0 ; `test ah,5 ; jp 0x788544` ----
    // jp taken (PF=1) ⇔ diff >= 0 (alpha path); not taken ⇔ diff < 0 (zero-out path).
    // NOTE: this short slice shares its start (0x788510) with the alpha-leg slice but must run
    // BEFORE it — once a slice containing `call __ftol` (0x788553) executes, the emulator's
    // translation of the 0x788510 block desyncs and a later *mid-block* halt at 0x78851e spuriously
    // faults (a Unicorn TB-cache artifact, not a fidelity divergence). Running the probe first
    // (and the c fuzz also first) keeps every 0x788510-start halt clean. (`run` halts after
    // `fnstsw` so we read the raw status word; the `jp` tests parity of `ah & 5` either way.)
    let mut hi = 0u32;
    let mut lo = 0u32;
    let probe = |emu: &mut Emu, f10: f32, st: f32, hi: &mut u32, lo: &mut u32, total: &mut u32| {
        let ah = run_cmp(emu, f10, st);
        let bin_taken = parity_even(ah, 0x5); // jp ⇔ PF=1
        let diff = f64::from(f10) - f64::from(st);
        // the binary's jp at 0x788521 is taken on (>= 0 or unordered); strict-< is the zero
        // path. `diff` is finite here (finite probes), so `>= 0.0` matches the not-strict-< sense.
        let my_taken = diff >= 0.0;
        assert_eq!(bin_taken, my_taken, "alpha branch probe f10={f10} st={st}");
        if bin_taken {
            *hi += 1;
        } else {
            *lo += 1;
        }
        *total += 1;
    };
    for (f10, st) in [
        (1.0f32, 1.0), // diff == 0 (the pivot) -> jp taken
        (1.0, 0.9999999),
        (1.0, 1.0000001),
        (5.0, 5.0),
        (0.0, 1.0e-7), // tiny negative -> not taken
        (1.0e-7, 0.0), // tiny positive -> taken
        (-3.0, -3.0),  // diff 0
        (100.0, 50.0),
        (50.0, 100.0),
    ] {
        probe(&mut emu, f10, st, &mut hi, &mut lo, &mut total);
    }
    // fuzz the branch boundary too (still before any __ftol slice).
    let mut sp = 0x7885_2100u32;
    for _ in 0..2000 {
        let f10 = rng(&mut sp, -200.0, 200.0);
        // straddle: half the time near-equal to f10, half a wide spread.
        let st = if (next(&mut sp) >> 8) & 1 == 0 {
            f10 + rng(&mut sp, -0.001, 0.001)
        } else {
            rng(&mut sp, -200.0, 200.0)
        };
        probe(&mut emu, f10, st, &mut hi, &mut lo, &mut total);
    }
    assert!(hi > 0 && lo > 0, "alpha branch needs both sides");

    // ---- (c) alpha leg: feed inputs with (field0x10 - step) >= 0 (the jp-taken path) ----
    // deterministic incl. boundary where the ·255/div lands exactly on an integer.
    for (f10, st, div) in [
        (1.0f32, 0.0, 1.0),    // 1/1*255 = 255 -> 255
        (1.0, 0.0, 255.0),     // 1/255*255 = 1 -> 1
        (0.5, 0.0, 1.0),       // 0.5*255 = 127.5 -> 127 (trunc)
        (0.0, 0.0, 1.0),       // 0 -> 0
        (2.0, 1.0, 4.0),       // (2-1)/4*255 = 63.75 -> 63
        (300.0, 45.0, 1000.0), // (255)/1000*255 = 65.025 -> 65
        (1.0, 0.0, 2.0),       // 0.5*255 = 127.5 -> 127
        // A zero or NaN divisor, and quotients past `i32` and `i64`: `__ftol` returns the low
        // dword of its qword, 0 for the indefinite, and the byte is its low byte.
        (1.0, 0.0, 0.0),
        (0.0, 0.0, 0.0),
        (1.0, 0.0, f32::NAN),
        (1.0, 0.0, 1.0e-9),
        (1.0, 0.0, 1.0e-30),
        (1.0e10, 0.0, 1.0),
        (1.0e30, 0.0, 1.0),
        (f32::INFINITY, 0.0, 1.0),
    ] {
        let (bs, ba) = run_alpha(&mut emu, f10, st, div);
        let (ms, ma) = alpha_step(f10, st, div);
        assert_eq!(
            bs.to_bits(),
            ms.to_bits(),
            "alpha stored f10={f10} st={st} div={div}"
        );
        assert_eq!(ba, ma, "alpha byte f10={f10} st={st} div={div}");
        total += 2;
    }
    let mut sb = 0x7885_1000u32;
    for _ in 0..3000 {
        // ensure (f10 - st) >= 0 and div != 0.
        let st = rng(&mut sb, 0.0, 50.0);
        let f10 = st + rng(&mut sb, 0.0, 500.0); // f10 >= st
        let mut div = rng(&mut sb, 1.0, 4000.0);
        if div == 0.0 {
            div = 1.0;
        }
        let (bs, ba) = run_alpha(&mut emu, f10, st, div);
        let (ms, ma) = alpha_step(f10, st, div);
        assert_eq!(
            bs.to_bits(),
            ms.to_bits(),
            "alpha stored fuzz f10={f10} st={st} div={div}"
        );
        assert_eq!(ba, ma, "alpha byte fuzz f10={f10} st={st} div={div}");
        total += 2;
    }
    eprintln!(
        "glue_788_788460_alpha_step: {total} bit-exact (decrement + alpha div/mul/__ftol + branch probe; jp+={hi} jp-={lo})"
    );
}

// ===========================================================================================
// 0x78a320 — "rect changed?" predicate (no recompute). Slice (a) 0x78a330..0x78a348 (dy |Δ| compare,
// read AH); slice (b) 0x78a34d..0x78a364 (dx |Δ| compare, read AH). Magnitudes ALSO via f64 islands.
// arithmetic: fsub 0x78a333/339, fsubp 0x78a33c, fabs 0x78a33e (a); fsub 0x78a350/355, fsubp 0x78a358,
// fabs 0x78a35e (b).
// ===========================================================================================
#[test]
fn glue_788_78a320_rect_changed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // dy uses [edi+0xc]/[edi+0x4] and [esi+0x4c]/[esi+0x44]; dx uses [edi+0x8]/[edi] and
    // [esi+0x48]/[esi+0x40].
    let setup = |emu: &mut Emu,
                 ay_hi: f32,
                 ay_lo: f32,
                 by_hi: f32,
                 by_lo: f32,
                 ax_hi: f32,
                 ax_lo: f32,
                 bx_hi: f32,
                 bx_lo: f32| {
        emu.write_u32(f.edi + 0xc, ay_hi.to_bits());
        emu.write_u32(f.edi + 0x4, ay_lo.to_bits());
        emu.write_u32(f.esi + 0x4c, by_hi.to_bits());
        emu.write_u32(f.esi + 0x44, by_lo.to_bits());
        emu.write_u32(f.edi + 0x8, ax_hi.to_bits());
        emu.write_u32(f.edi, ax_lo.to_bits());
        emu.write_u32(f.esi + 0x48, bx_hi.to_bits());
        emu.write_u32(f.esi + 0x40, bx_lo.to_bits());
    };
    let run_dy = |emu: &mut Emu| -> u8 {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EDI, f.edi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x78a330, 0x78a348); // halt at `test ah,5` (dy compare)
        ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
    };
    let run_dx = |emu: &mut Emu| -> u8 {
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EDI, f.edi);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x78a34d, 0x78a364); // halt at `test ah,5` (dx compare)
        ((emu.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
    };

    // f64-magnitude islands: copy fld..fabs source bytes, append `fstp qword [cell]`.
    let celly = emu.alloc(8);
    let isly = emu.alloc(0x40);
    let yb = emu.read(0x78a330, 0x78a340 - 0x78a330); // fld..fabs (16 bytes)
    let mut cy = yb.clone();
    cy.extend_from_slice(&[0xddu8, 0x1d]);
    cy.extend_from_slice(&(celly as u32).to_le_bytes());
    emu.write(isly, &cy);
    let isly_end = isly + cy.len() as u64;
    assert_eq!(
        emu.read(isly, yb.len()),
        emu.read(0x78a330, yb.len()),
        "dy island != src"
    );

    let cellx = emu.alloc(8);
    let islx = emu.alloc(0x40);
    let xb = emu.read(0x78a34d, 0x78a35c - 0x78a34d); // fld..fabs (15 bytes; fsub[edi] is 2B)
    let mut cx = xb.clone();
    cx.extend_from_slice(&[0xddu8, 0x1d]);
    cx.extend_from_slice(&(cellx as u32).to_le_bytes());
    emu.write(islx, &cx);
    let islx_end = islx + cx.len() as u64;
    assert_eq!(
        emu.read(islx, xb.len()),
        emu.read(0x78a34d, xb.len()),
        "dx island != src"
    );

    let mut total = 0u32;
    let mut dirty_n = 0u32;
    let mut clean_n = 0u32;

    #[allow(clippy::too_many_arguments)]
    let one = |emu: &mut Emu,
               ay_hi: f32,
               ay_lo: f32,
               by_hi: f32,
               by_lo: f32,
               ax_hi: f32,
               ax_lo: f32,
               bx_hi: f32,
               bx_lo: f32,
               total: &mut u32,
               dirty_n: &mut u32,
               clean_n: &mut u32| {
        setup(emu, ay_hi, ay_lo, by_hi, by_lo, ax_hi, ax_lo, bx_hi, bx_lo);
        let y_ah = run_dy(emu);
        let x_ah = run_dx(emu);
        // magnitudes (f64) via islands.
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EDI, f.edi);
        emu.run(isly, isly_end);
        let bin_dy = f64::from_bits(u64::from_le_bytes(emu.read(celly, 8).try_into().unwrap()));
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.set_reg(RegisterX86::EDI, f.edi);
        emu.run(islx, islx_end);
        let bin_dx = f64::from_bits(u64::from_le_bytes(emu.read(cellx, 8).try_into().unwrap()));
        assert_eq!(
            bin_dy.to_bits(),
            dy_magnitude_78a320(ay_hi, ay_lo, by_hi, by_lo).to_bits(),
            "dy mag"
        );
        assert_eq!(
            bin_dx.to_bits(),
            dx_magnitude_78a320(ax_hi, ax_lo, bx_hi, bx_lo).to_bits(),
            "dx mag"
        );
        // dirty = Y-jp OR X-jp.
        let bin_dirty = parity_even(y_ah, 0x5) || parity_even(x_ah, 0x5);
        let my_dirty = rect_changed_78a320(ay_hi, ay_lo, by_hi, by_lo, ax_hi, ax_lo, bx_hi, bx_lo);
        assert_eq!(bin_dirty, my_dirty, "78a320 dirty");
        if bin_dirty {
            *dirty_n += 1;
        } else {
            *clean_n += 1;
        }
        *total += 3;
    };

    // deterministic: identical rects -> clean; moved -> dirty; eps straddle on each axis.
    one(
        &mut emu,
        100.0,
        0.0,
        100.0,
        0.0,
        200.0,
        0.0,
        200.0,
        0.0,
        &mut total,
        &mut dirty_n,
        &mut clean_n,
    );
    one(
        &mut emu,
        100.0,
        0.0,
        100.0,
        5.0,
        200.0,
        0.0,
        200.0,
        0.0,
        &mut total,
        &mut dirty_n,
        &mut clean_n,
    );
    {
        let eps = f64::from(f32::from_bits(EPS_BITS));
        // Y delta: da = ay_hi-ay_lo, db = by_hi-by_lo; |da-db| straddles eps.
        for d in [eps, eps * 0.5, eps * 2.0, -eps] {
            let ay_hi = 100.0f32;
            let ay_lo = 0.0f32;
            let by_lo = 0.0f32;
            let by_hi = ((f64::from(ay_hi) - f64::from(ay_lo)) - f64::from(by_lo) - d) as f32;
            // X axis identical -> clean, so dirty is decided by Y alone.
            one(
                &mut emu,
                ay_hi,
                ay_lo,
                by_hi,
                by_lo,
                50.0,
                0.0,
                50.0,
                0.0,
                &mut total,
                &mut dirty_n,
                &mut clean_n,
            );
        }
    }

    let mut sd = 0x78a3_2000u32;
    for _ in 0..1500 {
        let ay_hi = rng(&mut sd, -500.0, 500.0);
        let ay_lo = rng(&mut sd, -500.0, 500.0);
        // bias by toward equality so both branches fire.
        let eq = (next(&mut sd) >> 8) & 1 == 0;
        let by_hi = if eq {
            ay_hi
        } else {
            rng(&mut sd, -500.0, 500.0)
        };
        let by_lo = if eq {
            ay_lo
        } else {
            rng(&mut sd, -500.0, 500.0)
        };
        let ax_hi = rng(&mut sd, -500.0, 500.0);
        let ax_lo = rng(&mut sd, -500.0, 500.0);
        let bx_hi = if eq {
            ax_hi
        } else {
            rng(&mut sd, -500.0, 500.0)
        };
        let bx_lo = if eq {
            ax_lo
        } else {
            rng(&mut sd, -500.0, 500.0)
        };
        one(
            &mut emu,
            ay_hi,
            ay_lo,
            by_hi,
            by_lo,
            ax_hi,
            ax_lo,
            bx_hi,
            bx_lo,
            &mut total,
            &mut dirty_n,
            &mut clean_n,
        );
    }
    eprintln!(
        "glue_788_78a320_rect_changed: {total} bit-exact (|Δy|+|Δx|+predicate; dirty={dirty_n} clean={clean_n})"
    );
    assert!(dirty_n > 0 && clean_n > 0, "need both predicate branches");
}

// ===========================================================================================
// 0x78adb0 — re-attach widget; the ONE owned arith op is `fchs` at 0x78aec0 (negate the float
// returned by 0x766750, fstp to [edi+0x34c] as f32). Diffed via a COPIED-BYTE ISLAND:
// `fld [cell_in]; <source fchs bytes>; fstp dword [cell_out]` — the `fchs` (d9 e0) byte-verified
// against the source. arithmetic: fchs 0x78aec0.
// ===========================================================================================
#[test]
fn glue_788_78adb0_negate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let _ = frame(&mut emu);

    let cell_in = emu.alloc(4);
    let cell_out = emu.alloc(4);
    let isl = emu.alloc(0x20);
    // fld dword [cell_in]  (D9 05 + disp32)
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(cell_in as u32).to_le_bytes());
    // the source `fchs` bytes (d9 e0), copied verbatim.
    let fchs = emu.read(0x78aec0, 2);
    code.extend_from_slice(&fchs);
    // fstp dword [cell_out]  (D9 1D + disp32)
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(cell_out as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);
    // byte-verify the island's fchs op against the live source.
    assert_eq!(
        emu.read(isl + 6, 2),
        emu.read(0x78aec0, 2),
        "fchs island != source"
    );

    let run = |emu: &mut Emu, q: f32| -> f32 {
        emu.write_u32(cell_in, q.to_bits());
        emu.run(isl, isl_end);
        emu.read_f32(cell_out)
    };

    // NB: a queried widget-geometry float (from 0x766750) is finite — NaN is out of this slice's
    // domain (and `fchs` is a pure sign-bit flip whose NaN-payload handling differs from arithmetic
    // negation through the x87, so NaN is deliberately not probed).
    let mut total = 0u32;
    for q in [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        12345.678,
        -98765.43,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE,
        f32::MAX,
    ] {
        assert_eq!(
            run(&mut emu, q).to_bits(),
            negate_stp(q).to_bits(),
            "negate q={q}"
        );
        total += 1;
    }
    let mut s = 0x78ae_c000u32;
    for _ in 0..3000 {
        let q = rng(&mut s, -100000.0, 100000.0);
        assert_eq!(
            run(&mut emu, q).to_bits(),
            negate_stp(q).to_bits(),
            "negate fuzz q={q}"
        );
        total += 1;
    }
    eprintln!("glue_788_78adb0_negate: {total} bit-exact (fchs sign-negate, copied-byte island)");
    // touch the imported scale constant so an unused-import lint can't fire if refactored.
    let _ = SCALE255_BITS;
}
