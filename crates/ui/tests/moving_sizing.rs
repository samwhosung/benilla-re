//! Differential tests of the arithmetic of `CSimpleTop::UpdateMovingOrSizing` (`0x7655b0`), the
//! pixel snaps `PixelAlignX` (`0x7666a0`) and `PixelAlignY` (`0x766750`), and
//! `CLayoutFrame::PinToParentOffset` (`0x768430`), each run against `WoW.exe` in the emulator.
//!
//! Each owned FP slice is diffed by executing the binary's OWN bytes (`Emu::run`) over just the FP
//! compute — registers/memory/stack synthesized, the `fst/fstp [m32]` value or `__ftol` EAX read
//! back. Every in-body arithmetic op lands in an executed slice; the `fcom`/`fcomp`, `fild`, and
//! `fld`/`fst`/`fstp` ride along for free. The leaf helpers `0x41ae40`/`0x41ae50` (÷0.8/÷0.6),
//! `0x41ae60`/`0x41ae70` (×), and the `__ftol` `0x40a2b0` run NATIVELY inside the slices (pure x87).
//! The object-graph fill `0x58a250`, the vtable/layout setters, and the field getters are NOT
//! entered — each owned slice starts past them with its inputs written directly to memory.
//!
//! Slice → op coverage:
//! - 0x7655b0: slice A `[0x7655b6,0x7655d6)` = fsub ×2; slice B `[0x765610,0x76562e)` = fadd ×2.
//! - 0x7666a0: prologue `[0x7666a6,0x7666b6)` = fabs#1; `[0x7666e5,0x7666f8)` = fsub + fabs#2;
//!   `[0x766706,0x766737)` = fmul + fadd + fdiv (the ÷0.8 leaf + __ftol run inside).
//! - 0x766750: prologue `[0x766756,0x766766)` = fabs#1; `[0x766795,0x7667a8)` = fsub + fabs#2;
//!   `[0x7667b6,0x7667e7)` = fmul + fadd + fdiv (the ÷0.6 leaf + __ftol run inside).
//! - 0x768430: block A `[0x7684a4,0x7684d5)` = fdiv#1(live) + fadd ×2 + fsubr ×2 + fchs ×2 + fmul
//!   ×2; block B `[0x7684f1,0x7684fd)` = fsub + fdiv; block C `[0x768518,0x768524)` = fsub + fdiv.
//!
//! Threshold probe: the 0x7666a0/0x766750 round-up bias pivot — the `fcom 0`
//! boundary where the product crosses `0` (bias added iff strictly positive). Fuzz reads the LCG's
//! HIGH bits.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::moving_sizing::{
    buf_delta, deltas_recenter, edge_products, final_div, local_convert, prologue_flags,
    ratio_round, recip_scale, BIAS_BITS, DIV06_BITS, DIV08_BITS, ONE_BITS,
};

const EPS_BITS: u32 = 0x3480_0000; // [0x8029d4] near-zero epsilon (2^-22)

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

const BIAS_VA: u64 = 0x808120;
const ONE_VA: u64 = 0x7ff9d8;
const DIV08_VA: u64 = 0x832a44;
const DIV06_VA: u64 = 0x832a48;

struct Frame {
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    Frame { ebp, esp }
}

// ===========================================================================================
// 0x7655b0 — coordinate deltas (fsub ×2) + global re-centres (fadd ×2). [.,0x765632 ret 4]
//   slice A 0x7655b6..0x7655d6 : eax=[ebp+8] struct ([+0x24]=A,[+0x28]=B), esi=ecx=this
//            ([+0xd08]=g0,[+0xd0c]=g1) → [ebp-0x8]=dx, [ebp-0x4]=dy.
//   slice B 0x765610..0x76562e : [ebp-0x8]/[ebp-0x4] = dx/dy, esi=this → [+0xd08]/[+0xd0c] updated.
// ===========================================================================================
#[test]
fn glue_765_7655b0_deltas_recenter() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let this = emu.alloc(0x2000);
    let arg = emu.alloc(0x100);

    // Returns (dx, dy, g0', g1') from the binary's own bytes.
    let run = |emu: &mut Emu, a: f32, b: f32, g0: f32, g1: f32| -> (f32, f32, f32, f32) {
        emu.write_u32(arg + 0x24, a.to_bits());
        emu.write_u32(arg + 0x28, b.to_bits());
        emu.write_u32(this + 0xd08, g0.to_bits());
        emu.write_u32(this + 0xd0c, g1.to_bits());
        emu.write_u32(f.ebp + 0x8, arg as u32);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ECX, this);
        emu.run(0x7655b6, 0x7655d6); // fld;fld;fxch;fsub;fstp;fsub;fstp
        let dx = emu.read_f32(f.ebp - 0x8);
        let dy = emu.read_f32(f.ebp - 0x4);
        // slice B re-centres the globals from [ebp-0x8]/[ebp-0x4] (left by slice A).
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x765610, 0x76562e); // fld;fadd;fstp ×2
        (
            dx,
            dy,
            emu.read_f32(this + 0xd08),
            emu.read_f32(this + 0xd0c),
        )
    };

    let mut total = 0u32;
    let probes = [
        (0.0f32, 0.0f32, 0.0f32, 0.0f32),
        (10.0, -3.5, 2.0, 1.0),
        (-7.5, 1024.0, -7.5, 1024.0), // exact deltas of 0
        (1.0e6, -1.0e6, 0.5, -0.5),
        (f32::from_bits(0x4b00_0001), 1.5, 1.0, -2.0),
    ];
    for (a, b, g0, g1) in probes {
        let got = run(&mut emu, a, b, g0, g1);
        let want = deltas_recenter(a, b, g0, g1);
        assert_eq!(got.0.to_bits(), want.0.to_bits(), "dx a={a} g0={g0}");
        assert_eq!(got.1.to_bits(), want.1.to_bits(), "dy b={b} g1={g1}");
        assert_eq!(got.2.to_bits(), want.2.to_bits(), "g0' a={a} g0={g0}");
        assert_eq!(got.3.to_bits(), want.3.to_bits(), "g1' b={b} g1={g1}");
        total += 4;
    }
    let mut s = 0x7655_b000u32;
    for _ in 0..3000 {
        let a = rng(&mut s, -50000.0, 50000.0);
        let b = rng(&mut s, -50000.0, 50000.0);
        let g0 = rng(&mut s, -50000.0, 50000.0);
        let g1 = rng(&mut s, -50000.0, 50000.0);
        let got = run(&mut emu, a, b, g0, g1);
        let want = deltas_recenter(a, b, g0, g1);
        assert_eq!(got.0.to_bits(), want.0.to_bits(), "dx fuzz a={a} g0={g0}");
        assert_eq!(got.1.to_bits(), want.1.to_bits(), "dy fuzz b={b} g1={g1}");
        assert_eq!(got.2.to_bits(), want.2.to_bits(), "g0' fuzz a={a} g0={g0}");
        assert_eq!(got.3.to_bits(), want.3.to_bits(), "g1' fuzz b={b} g1={g1}");
        total += 4;
    }
    eprintln!("glue_765_7655b0_deltas_recenter: {total} bit-exact (fsub ×2 + fadd ×2)");
}

// Shared driver for the 0x7666a0 / 0x766750 twin pair. `hi_off`/`lo_off` select the buffer fields,
// the three slice addresses the per-variant FP blocks. The ÷-divisor lives in the binary's own
// bytes the `ratio_slice` executes (the leaf `0x41ae40`/`0x41ae50`), so it needs no parameter here.
// Returns (prologue AX flags, d, __ftol result, fdiv result).
#[allow(clippy::too_many_arguments)]
fn run_ratio_pair(
    emu: &mut Emu,
    f: &Frame,
    prologue: (u64, u64),
    fsub_slice: (u64, u64),
    ratio_slice: (u64, u64),
    hi_off: i64,
    lo_off: i64,
    arg: f32,
    buf_hi: f32,
    buf_lo: f32,
) -> (u16, f32, i32, f32) {
    // prologue fabs#1 on |arg|: read fnstsw AX (the |arg| vs eps compare flags).
    emu.write_u32(f.ebp + 0x8, arg.to_bits());
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
    emu.run(prologue.0, prologue.1); // fld [ebp+8]; fabs; fcomp [eps]; fnstsw ax
    let ax = (emu.get_reg(RegisterX86::EAX) & 0xffff) as u16;

    // fsub + fabs#2: buffer fields → d = buf_hi - buf_lo at [ebp-0x4].
    emu.write_u32((f.ebp as i64 + hi_off) as u64, buf_hi.to_bits());
    emu.write_u32((f.ebp as i64 + lo_off) as u64, buf_lo.to_bits());
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
    emu.run(fsub_slice.0, fsub_slice.1); // fld;fsub;fst [ebp-0x4];fabs;fcomp;fnstsw
    let d = emu.read_f32(f.ebp - 0x4);

    // fmul + (bias) + fdiv: arg/divisor (leaf) · d → __ftol → /d. [ebp+8]=arg, [ebp-0x4]=d.
    emu.write_u32(f.ebp + 0x8, arg.to_bits());
    emu.write_u32(f.ebp - 0x4, d.to_bits());
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
    emu.run(ratio_slice.0, ratio_slice.1); // mov;push;call ÷leaf;fmul;fcom;(fadd);call __ftol;
                                           // mov;fild;push;fdiv;fstp [esp]
    let n = emu.get_reg(RegisterX86::EAX) as i32; // __ftol result
    let cur_esp = emu.get_reg(RegisterX86::ESP);
    let q = emu.read_f32(cur_esp); // the fdiv result stored to [esp]
    (ax, d, n, q)
}

// ===========================================================================================
// 0x7666a0 — X axis, ÷0.8. [.,0x766740 ret 4]. buf[3]-buf[1] = [ebp-0x8]-[ebp-0x10].
// fabs ×2 + fsub ×1 + fmul ×1 + fadd ×1 + fdiv ×1 = 6.
// ===========================================================================================
#[test]
fn glue_765_7666a0_ratio_round_x() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Sanity: the .data divisor + .rdata constants the binary's bytes will read.
    assert_eq!(emu.read_u32(DIV08_VA), DIV08_BITS, "0x832a44 != 0.8");
    assert_eq!(emu.read_u32(BIAS_VA), BIAS_BITS, "0x808120 bias");

    let prologue = (0x7666a6, 0x7666b6);
    let fsub_slice = (0x7666e5, 0x7666f8);
    let ratio_slice = (0x766706, 0x766737);
    let hi_off = -0x8; // buf[3]
    let lo_off = -0x10; // buf[1]

    let mut total = 0u32;
    let mut check = |emu: &mut Emu, arg: f32, bh: f32, bl: f32| {
        // buffer fields must keep |d| above eps so the fsub slice's `jp` falls through (else the
        // binary early-returns and our slice halt EIP would differ). All probe deltas are large.
        let (ax, d, n, q) = run_ratio_pair(
            emu,
            &f,
            prologue,
            fsub_slice,
            ratio_slice,
            hi_off,
            lo_off,
            arg,
            bh,
            bl,
        );
        assert_eq!(
            ax,
            prologue_flags(arg, EPS_BITS),
            "prologue fabs#1 flags arg={arg}"
        );
        let wd = buf_delta(bh, bl);
        assert_eq!(d.to_bits(), wd.to_bits(), "d bh={bh} bl={bl}");
        let wn = ratio_round(arg, DIV08_BITS, wd);
        assert_eq!(n, wn, "ftol arg={arg} d={wd}");
        let wq = final_div(wn, wd);
        assert_eq!(q.to_bits(), wq.to_bits(), "q arg={arg} d={wd}");
        total += 4;
    };

    // deterministic incl. the bias pivot (product crossing 0) and the |arg|==eps prologue boundary.
    let eps = f32::from_bits(EPS_BITS);
    for (arg, bh, bl) in [
        (50.0f32, 9.0f32, 2.0f32),
        (-50.0, 9.0, 2.0),  // product < 0 → no bias
        (0.0, 9.0, 2.0),    // product == 0 → no bias
        (0.0001, 9.0, 2.0), // product tiny + → bias fires (ftol=1)
        (50.0, 2.0, 9.0),   // delta < 0 → product < 0
        (12.3456, 100.5, -50.25),
        (1.0e5, 3.0, -3.0),
        (eps, 9.0, 2.0),       // |arg| == eps  (prologue C3 boundary)
        (-eps, 9.0, 2.0),      // |−eps| == eps
        (eps * 0.5, 9.0, 2.0), // |arg| < eps   (prologue C0)
        // The infinities and products past `i32` and `i64`: `__ftol` returns the low dword of its
        // qword, 0 for the indefinite. A NaN argument is not run: the prologue's flag word is not
        // modelled for it.
        (f32::INFINITY, 9.0, 2.0),
        (f32::NEG_INFINITY, 9.0, 2.0),
        (1.0e9, 9.0, 2.0),
        (-1.0e9, 9.0, 2.0),
        (1.0e30, 9.0, 2.0),
        (1.0e30, 9.0, -1.0e30),
    ] {
        check(&mut emu, arg, bh, bl);
    }
    let mut s = 0x7666_a000u32;
    for _ in 0..2500 {
        let arg = rng(&mut s, -5000.0, 5000.0);
        let bh = rng(&mut s, -2000.0, 2000.0);
        let mut bl = rng(&mut s, -2000.0, 2000.0);
        if (bh - bl).abs() < 0.01 {
            bl -= 5.0; // keep |d| comfortably above eps (the slice's fall-through path)
        }
        check(&mut emu, arg, bh, bl);
    }
    eprintln!(
        "glue_765_7666a0_ratio_round_x: {total} bit-exact (fabs×2 fsub fmul fadd fdiv; ÷0.8)"
    );
}

// ===========================================================================================
// 0x766750 — Y axis, ÷0.6, the non-clone twin. [.,0x7667f0 ret 4].
// buf[2]-buf[0] = [ebp-0xc]-[ebp-0x14]. fabs ×2 + fsub ×1 + fmul ×1 + fadd ×1 + fdiv ×1 = 6.
// ===========================================================================================
#[test]
fn glue_765_766750_ratio_round_y() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    assert_eq!(emu.read_u32(DIV06_VA), DIV06_BITS, "0x832a48 != 0.6");
    assert_eq!(emu.read_u32(BIAS_VA), BIAS_BITS, "0x808120 bias");

    let prologue = (0x766756, 0x766766);
    let fsub_slice = (0x766795, 0x7667a8);
    let ratio_slice = (0x7667b6, 0x7667e7);
    let hi_off = -0xc; // buf[2]
    let lo_off = -0x14; // buf[0]

    let mut total = 0u32;
    let mut check = |emu: &mut Emu, arg: f32, bh: f32, bl: f32| {
        let (ax, d, n, q) = run_ratio_pair(
            emu,
            &f,
            prologue,
            fsub_slice,
            ratio_slice,
            hi_off,
            lo_off,
            arg,
            bh,
            bl,
        );
        assert_eq!(
            ax,
            prologue_flags(arg, EPS_BITS),
            "prologue fabs#1 flags arg={arg}"
        );
        let wd = buf_delta(bh, bl);
        assert_eq!(d.to_bits(), wd.to_bits(), "d bh={bh} bl={bl}");
        let wn = ratio_round(arg, DIV06_BITS, wd);
        assert_eq!(n, wn, "ftol arg={arg} d={wd}");
        let wq = final_div(wn, wd);
        assert_eq!(q.to_bits(), wq.to_bits(), "q arg={arg} d={wd}");
        total += 4;
    };

    let eps = f32::from_bits(EPS_BITS);
    for (arg, bh, bl) in [
        (50.0f32, 9.0f32, 2.0f32),
        (-50.0, 9.0, 2.0),
        (0.0, 9.0, 2.0),
        (0.0001, 9.0, 2.0),
        (50.0, 2.0, 9.0),
        (12.3456, 100.5, -50.25),
        (1.0e5, 3.0, -3.0),
        (eps, 9.0, 2.0), // |arg| == eps boundary
        (-eps, 9.0, 2.0),
        (eps * 0.5, 9.0, 2.0), // |arg| < eps
    ] {
        check(&mut emu, arg, bh, bl);
    }
    let mut s = 0x7667_5000u32;
    for _ in 0..2500 {
        let arg = rng(&mut s, -5000.0, 5000.0);
        let bh = rng(&mut s, -2000.0, 2000.0);
        let mut bl = rng(&mut s, -2000.0, 2000.0);
        if (bh - bl).abs() < 0.01 {
            bl -= 5.0;
        }
        check(&mut emu, arg, bh, bl);
    }
    eprintln!(
        "glue_765_766750_ratio_round_y: {total} bit-exact (fabs×2 fsub fmul fadd fdiv; ÷0.6)"
    );
}

// ===========================================================================================
// 0x768430 — edge-rect builder. [.,0x76854c ret 0x14].
//   block A 0x7684a4..0x7684d5 : recip=1/scale (live), pa→[esp+4], pb→[esp].
//   block B 0x7684f1..0x7684fd : ([ebp-0x4]-[ebp-0xc])/scale → [ebp+0xc].
//   block C 0x768518..0x768524 : ([ebp-0x8]-[ebp-0x10])/scale → [ebp+0x10].
// fdiv ×3 + fadd ×2 + fsubr ×2 + fchs ×2 + fmul ×2 + fsub ×2 = 13.
// ===========================================================================================
#[test]
fn glue_765_768430_edge_rect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    let this = emu.alloc(0x2000);
    assert_eq!(emu.read_u32(ONE_VA), ONE_BITS, "0x7ff9d8 != 1.0");

    // Block A: read the two products from the stack (esp+4, esp) after the two fstp stores.
    // At halt 0x7684d5: esp = esp0 - 4 (push 0 @0x7684ad) - 8 (sub esp,8 @0x7684af) = esp0 - 12;
    // pa was fstp [esp+4], pb was fstp [esp].
    #[allow(clippy::too_many_arguments)]
    let run_a = |emu: &mut Emu,
                 u: f32,
                 p18: f32,
                 e18: f32,
                 v: f32,
                 p14: f32,
                 e1c: f32,
                 scale: f32|
     -> (f32, f32) {
        emu.write_u32(this + 0x58, scale.to_bits());
        emu.write_u32(f.ebp - 0x8, u.to_bits());
        emu.write_u32(f.ebp + 0x18, p18.to_bits());
        emu.write_u32(f.ebp - 0x18, e18.to_bits());
        emu.write_u32(f.ebp - 0xc, v.to_bits());
        emu.write_u32(f.ebp + 0x14, p14.to_bits());
        emu.write_u32(f.ebp - 0x1c, e1c.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.run(0x7684a4, 0x7684d5);
        let cur = emu.get_reg(RegisterX86::ESP);
        (emu.read_f32(cur + 4), emu.read_f32(cur)) // (pa, pb)
    };

    // Block B / Block C: one (hi-lo)/scale convert each.
    let run_bc = |emu: &mut Emu,
                  slice: (u64, u64),
                  hi_off: i64,
                  lo_off: i64,
                  dest_off: i64,
                  hi: f32,
                  lo: f32,
                  scale: f32|
     -> f32 {
        emu.write_u32(this + 0x58, scale.to_bits());
        emu.write_u32((f.ebp as i64 + hi_off) as u64, hi.to_bits());
        emu.write_u32((f.ebp as i64 + lo_off) as u64, lo.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.run(slice.0, slice.1);
        emu.read_f32((f.ebp as i64 + dest_off) as u64)
    };

    let mut total = 0u32;
    let probesa = [
        (5.0f32, 1.0f32, 20.0f32, 7.0f32, 2.0f32, 30.0f32, 4.0f32),
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0),
        (100.0, -50.0, 25.5, -3.5, 7.0, 0.0, 0.5),
        (-1.0e4, 1.0e4, 1.0e4, 1.0e4, -1.0e4, 1.0e4, 2.0),
    ];
    for (u, p18, e18, v, p14, e1c, scale) in probesa {
        let (pa, pb) = run_a(&mut emu, u, p18, e18, v, p14, e1c, scale);
        let (wpa, wpb) = edge_products(u, p18, e18, v, p14, e1c, scale);
        assert_eq!(pa.to_bits(), wpa.to_bits(), "pa u={u} scale={scale}");
        assert_eq!(pb.to_bits(), wpb.to_bits(), "pb v={v} scale={scale}");
        total += 2;
    }
    let mut s = 0x7684_3000u32;
    for _ in 0..2000 {
        let u = rng(&mut s, -20000.0, 20000.0);
        let p18 = rng(&mut s, -5000.0, 5000.0);
        let e18 = rng(&mut s, -20000.0, 20000.0);
        let v = rng(&mut s, -20000.0, 20000.0);
        let p14 = rng(&mut s, -5000.0, 5000.0);
        let e1c = rng(&mut s, -20000.0, 20000.0);
        let scale = rng(&mut s, 0.25, 8.0);
        let (pa, pb) = run_a(&mut emu, u, p18, e18, v, p14, e1c, scale);
        let (wpa, wpb) = edge_products(u, p18, e18, v, p14, e1c, scale);
        assert_eq!(pa.to_bits(), wpa.to_bits(), "pa fuzz u={u} scale={scale}");
        assert_eq!(pb.to_bits(), wpb.to_bits(), "pb fuzz v={v} scale={scale}");
        total += 2;

        // Block B: ([ebp-0x4]-[ebp-0xc])/scale → [ebp+0xc].
        let hi = rng(&mut s, -20000.0, 20000.0);
        let lo = rng(&mut s, -20000.0, 20000.0);
        let b = run_bc(
            &mut emu,
            (0x7684f1, 0x7684fd),
            -0x4,
            -0xc,
            0xc,
            hi,
            lo,
            scale,
        );
        assert_eq!(
            b.to_bits(),
            local_convert(hi, lo, scale).to_bits(),
            "blockB hi={hi} lo={lo} scale={scale}"
        );
        // Block C: ([ebp-0x8]-[ebp-0x10])/scale → [ebp+0x10].
        let hi2 = rng(&mut s, -20000.0, 20000.0);
        let lo2 = rng(&mut s, -20000.0, 20000.0);
        let c = run_bc(
            &mut emu,
            (0x768518, 0x768524),
            -0x8,
            -0x10,
            0x10,
            hi2,
            lo2,
            scale,
        );
        assert_eq!(
            c.to_bits(),
            local_convert(hi2, lo2, scale).to_bits(),
            "blockC hi={hi2} lo={lo2} scale={scale}"
        );
        total += 2;
    }

    // recip_scale spot-check (the live-st0 reciprocal feeding both products).
    for sc in [0.5f32, 1.0, 4.0, 100.0] {
        let r = recip_scale(sc);
        assert_eq!(r, f64::from(1.0f32) / f64::from(sc));
    }
    eprintln!(
        "glue_765_768430_edge_rect: {total} bit-exact (fdiv×3 fadd×2 fsubr×2 fchs×2 fmul×2 fsub×2)"
    );
}
