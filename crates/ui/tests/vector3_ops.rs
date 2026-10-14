//! Vector kernels at `0x4813b0`, `0x4841c0` and `0x4841f0` against `WoW.exe`.
//!
//! The two vec3 accessors (`0x4841c0` add, `0x4841f0` subtract) run whole (`Emu::call`) and the
//! stored vec3 is read back. `0x4813b0` runs as three slices: the two normalize divides
//! (`0x4813b6`..`0x4813ed`), the four range gates (`0x4813ed`..the predicted halt, whose EIP is the
//! branch assertion), and the two vec3 `+=` blocks (the `[edi]` and `[esi]` shapes). Every in-body
//! arithmetic opcode (fsub, fdivp, fmul, fadd) lands in a diffed slice. Fuzz reads the LCG's high
//! bits, and the gates carry `==` probes at exactly 0.0 and 1.0 (the parity pivots).
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::vector3_ops::{in_range, normalize_pair, vec3_add, vec3_add_inplace, vec3_sub};

// ---- shared LCG fuzz (high bits only) ------------------------------------------------------
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

// ===========================================================================================
// 0x4841c0 — vec3 dst += src. Whole-fn `Emu::call(ecx=dst, [ebp+8]=src)`; read dst back.
//   fadd ×3 (0x4841ca/d1/da) — all covered by the whole-fn call.
// ===========================================================================================
#[test]
fn glue_481_4841c0_vec_add() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let dst = emu.alloc(0x10);
    let src = emu.alloc(0x10);

    let run = |emu: &mut Emu, d: [f32; 3], s: [f32; 3]| -> [f32; 3] {
        for i in 0..3 {
            emu.write_u32(dst + 4 * i as u64, d[i].to_bits());
            emu.write_u32(src + 4 * i as u64, s[i].to_bits());
        }
        // __thiscall: ecx = dst (→ eax), one stack arg [ebp+8] = src. ret 4.
        emu.call(0x4841c0, dst as u32, 0, &[src as u32]);
        [
            emu.read_f32(dst),
            emu.read_f32(dst + 4),
            emu.read_f32(dst + 8),
        ]
    };

    let mut total = 0u32;
    // deterministic: zeros, exact cancellation, f32-rounding domain, large/small.
    let probes: [([f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([1.0, 2.0, 3.0], [-1.0, -2.0, -3.0]), // exact cancel → 0
        ([16_777_216.0, 1.0, -1.0], [1.0, 16_777_216.0, 0.0]), // f32 rounding
        ([1e30, -1e30, 0.5], [1e30, 1e30, 0.25]),
        ([0.1, 0.2, 0.3], [0.4, 0.5, 0.6]),
        ([f32::from_bits(0x4b00_0001), 7.5, -7.5], [1.0, 0.0, 7.5]),
    ];
    for (d, s) in probes {
        let got = run(&mut emu, d, s);
        let want = vec3_add(d, s);
        for i in 0..3 {
            assert_eq!(
                got[i].to_bits(),
                want[i].to_bits(),
                "vec_add d={d:?} s={s:?} i={i}"
            );
        }
        total += 3;
    }
    let mut st = 0x4841_c000u32;
    for _ in 0..3000 {
        let d = [
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
        ];
        let s = [
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
        ];
        let got = run(&mut emu, d, s);
        let want = vec3_add(d, s);
        for i in 0..3 {
            assert_eq!(
                got[i].to_bits(),
                want[i].to_bits(),
                "vec_add fuzz d={d:?} s={s:?} i={i}"
            );
        }
        total += 3;
    }
    eprintln!("glue_481_4841c0_vec_add: {total} bit-exact (whole-fn fadd×3 dst += src)");
}

// ===========================================================================================
// 0x4841f0 — vec3 dst = minuend − subtrahend. Whole-fn `Emu::call(ecx=dst, edx=minuend,
// [ebp+8]=subtrahend)`; read dst back. NOT a byte-twin of 0x4841c0 (loads all 3, then stores
// FILO). fsub ×3 (0x4841fb/201/206) — all covered by the whole-fn call.
// ===========================================================================================
#[test]
fn glue_481_4841f0_vec_sub() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let dst = emu.alloc(0x10);
    let minu = emu.alloc(0x10);
    let subt = emu.alloc(0x10);

    let run = |emu: &mut Emu, m: [f32; 3], s: [f32; 3]| -> [f32; 3] {
        for i in 0..3 {
            emu.write_u32(minu + 4 * i as u64, m[i].to_bits());
            emu.write_u32(subt + 4 * i as u64, s[i].to_bits());
            emu.write_u32(dst + 4 * i as u64, 0xdead_beef); // ensure stores actually write
        }
        // ecx = dst (→ eax), edx = minuend, one stack arg [ebp+8] = subtrahend. ret 4.
        emu.call(0x4841f0, dst as u32, minu as u32, &[subt as u32]);
        [
            emu.read_f32(dst),
            emu.read_f32(dst + 4),
            emu.read_f32(dst + 8),
        ]
    };

    let mut total = 0u32;
    let probes: [([f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([5.0, 5.0, 5.0], [5.0, 5.0, 5.0]), // exact → 0
        ([16_777_217.0, 1.0, -1.0], [1.0, 16_777_217.0, 0.0]),
        ([1e30, -1e30, 0.5], [-1e30, 1e30, 0.25]),
        ([0.1, 0.2, 0.3], [0.4, 0.5, 0.6]),
        ([f32::from_bits(0x4b00_0001), 7.5, -7.5], [1.0, 0.0, 7.5]),
    ];
    for (m, s) in probes {
        let got = run(&mut emu, m, s);
        let want = vec3_sub(m, s);
        for i in 0..3 {
            assert_eq!(
                got[i].to_bits(),
                want[i].to_bits(),
                "vec_sub m={m:?} s={s:?} i={i}"
            );
        }
        total += 3;
    }
    let mut st = 0x4841_f000u32;
    for _ in 0..3000 {
        let m = [
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
        ];
        let s = [
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
        ];
        let got = run(&mut emu, m, s);
        let want = vec3_sub(m, s);
        for i in 0..3 {
            assert_eq!(
                got[i].to_bits(),
                want[i].to_bits(),
                "vec_sub fuzz m={m:?} s={s:?} i={i}"
            );
        }
        total += 3;
    }
    eprintln!(
        "glue_481_4841f0_vec_sub: {total} bit-exact (whole-fn fsub×3 dst = minuend − subtrahend)"
    );
}

// object fields read by the normalize slice (lo/hi for x and y).
const F_LO_Y: u64 = 0x390; // [ebx+0x390]
const F_LO_X: u64 = 0x394; // [ebx+0x394]
const F_HI_Y: u64 = 0x398; // [ebx+0x398]
const F_HI_X: u64 = 0x39c; // [ebx+0x39c]

#[test]
fn glue_481_4813b0_normalize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    let obj = emu.alloc(0x1000); // ebx (this) — holds the four lo/hi fields.

    // --- slice A: the two normalize divides (fsub ×4, fdivp ×2) ---
    // start at the first fld (0x4813b6); the prologue's `push ebx; mov ebx,ecx` are inside the
    // slice — set ECX = obj (→ ebx) and ESP valid for the push.
    let run_norm = |emu: &mut Emu,
                    arg0: f32,
                    arg1: f32,
                    lo_x: f32,
                    hi_x: f32,
                    lo_y: f32,
                    hi_y: f32|
     -> (f32, f32) {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::ECX, obj);
        emu.write_u32(ebp + 0x8, arg0.to_bits());
        emu.write_u32(ebp + 0xc, arg1.to_bits());
        emu.write_u32(obj + F_LO_X, lo_x.to_bits());
        emu.write_u32(obj + F_HI_X, hi_x.to_bits());
        emu.write_u32(obj + F_LO_Y, lo_y.to_bits());
        emu.write_u32(obj + F_HI_Y, hi_y.to_bits());
        emu.run(0x4813b6, 0x4813ed); // through both `fstp` (coord_x → [ebp-4], coord_y → [ebp+8])
        (emu.read_f32(ebp - 0x4), emu.read_f32(ebp + 0x8))
    };

    let mut total = 0u32;
    // deterministic incl. exact-0 numerator, value at lo (→0.0) / at hi (→1.0).
    let probes: [(f32, f32, f32, f32, f32, f32); 6] = [
        (0.5, 0.5, 0.0, 1.0, 0.0, 1.0),   // → (0.5, 0.5)
        (0.0, 1.0, 0.0, 1.0, 0.0, 1.0),   // at lo / at hi → (0.0, 1.0)
        (5.0, 5.0, 0.0, 10.0, 0.0, 10.0), // → (0.5, 0.5)
        (3.0, 7.0, 2.0, 4.0, 5.0, 9.0),   // generic range
        (-1.0, 2.0, 0.0, 1.0, 0.0, 1.0),  // out-of-range numerators (still computes)
        (100.0, -50.0, 10.0, 20.0, -100.0, 0.0),
    ];
    for (arg0, arg1, lo_x, hi_x, lo_y, hi_y) in probes {
        let (gx, gy) = run_norm(&mut emu, arg0, arg1, lo_x, hi_x, lo_y, hi_y);
        let (wx, wy) = normalize_pair(arg0, arg1, lo_x, hi_x, lo_y, hi_y);
        assert_eq!(
            gx.to_bits(),
            wx.to_bits(),
            "norm x a0={arg0} lo={lo_x} hi={hi_x}"
        );
        assert_eq!(
            gy.to_bits(),
            wy.to_bits(),
            "norm y a1={arg1} lo={lo_y} hi={hi_y}"
        );
        total += 2;
    }
    let mut st = 0x4813_b000u32;
    for _ in 0..3000 {
        let lo_x = rng(&mut st, -1000.0, 1000.0);
        let hi_x = lo_x + rng(&mut st, 0.001, 2000.0); // hi != lo (avoid /0)
        let lo_y = rng(&mut st, -1000.0, 1000.0);
        let hi_y = lo_y + rng(&mut st, 0.001, 2000.0);
        let arg0 = rng(&mut st, -2000.0, 2000.0);
        let arg1 = rng(&mut st, -2000.0, 2000.0);
        let (gx, gy) = run_norm(&mut emu, arg0, arg1, lo_x, hi_x, lo_y, hi_y);
        let (wx, wy) = normalize_pair(arg0, arg1, lo_x, hi_x, lo_y, hi_y);
        assert_eq!(gx.to_bits(), wx.to_bits(), "norm x fuzz a0={arg0}");
        assert_eq!(gy.to_bits(), wy.to_bits(), "norm y fuzz a1={arg1}");
        total += 2;
    }
    eprintln!(
        "glue_481_4813b0_normalize: {total} bit-exact (fsub×4 + fdivp×2: 2× (arg−lo)/(hi−lo))"
    );
}

#[test]
fn glue_481_4813b0_gates() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;

    // confirm the gate constants at their source VAs (0.0 / 1.0).
    assert_eq!(
        emu.read_f32(0x7ffd74).to_bits(),
        0x0000_0000,
        "lo 0.0 const"
    );
    assert_eq!(
        emu.read_f32(0x7ff9d8).to_bits(),
        0x3f80_0000,
        "hi 1.0 const"
    );

    // run the 4-gate block on pre-computed coords [ebp-4]=cx, [ebp+8]=cy; the PREDICTED halt EIP is
    // the assertion: all-pass ⇒ flow reaches 0x481439; any-fail ⇒ branches to the 0x4814ac exit.
    // (`Emu::run` asserts it ends EXACTLY at `until`; a misprediction = wrong-EIP/runaway failure.)
    const PASS: u64 = 0x481439;
    const FAIL: u64 = 0x4814ac;
    let run_gate = |emu: &mut Emu, cx: f32, cy: f32| {
        let want_pass = in_range(cx, cy);
        let until = if want_pass { PASS } else { FAIL };
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(ebp - 0x4, cx.to_bits());
        emu.write_u32(ebp + 0x8, cy.to_bits());
        emu.run(0x4813ed, until); // halt at predicted target — the bit-exact branch verdict
        want_pass
    };

    let mut total = 0u32;
    let mut npass = 0u32;
    let mut nfail = 0u32;
    // deterministic `==`-boundary probes at exactly 0.0 and 1.0 (the parity pivots), plus a
    // grid that exercises both branches per gate, and a NaN coord (survives all gates in the
    // binary — its `fcomp` is unordered, never ordered-`<` nor `>`).
    let edge: &[(f32, f32)] = &[
        (0.0, 0.0),               // both at lo boundary → PASS (>= 0.0, <= 1.0)
        (1.0, 1.0),               // both at hi boundary → PASS
        (0.0, 1.0),               // PASS
        (1.0, 0.0),               // PASS
        (0.5, 0.5),               // PASS
        (-0.0, 0.5),              // -0.0 is not < 0.0 → PASS
        (-1e-30, 0.5),            // just below 0 → gate1 FAIL
        (0.5, -1e-30),            // gate2 FAIL
        (1.0001, 0.5),            // gate3 FAIL
        (0.5, 1.0001),            // gate4 FAIL
        (-5.0, -5.0),             // FAIL
        (2.0, 2.0),               // FAIL
        (f32::NAN, 0.5),          // NaN x — survives all gates → PASS
        (0.5, f32::NAN),          // NaN y — survives → PASS
        (f32::INFINITY, 0.5),     // +inf > 1.0 → gate3 FAIL
        (f32::NEG_INFINITY, 0.5), // -inf < 0.0 → gate1 FAIL
    ];
    for &(cx, cy) in edge {
        let pass = run_gate(&mut emu, cx, cy);
        if pass {
            npass += 1;
        } else {
            nfail += 1;
        }
        total += 1;
    }
    let mut st = 0x4813_ed00u32;
    for _ in 0..4000 {
        // spread around [0,1] so both pass/fail are hit frequently.
        let cx = rng(&mut st, -0.5, 1.5);
        let cy = rng(&mut st, -0.5, 1.5);
        let pass = run_gate(&mut emu, cx, cy);
        if pass {
            npass += 1;
        } else {
            nfail += 1;
        }
        total += 1;
    }
    assert!(
        npass > 0 && nfail > 0,
        "gates: need both branches (pass={npass} fail={nfail})"
    );
    eprintln!("glue_481_4813b0_gates: {total} bit-exact branch verdicts (pass={npass} fail={nfail}; halt-EIP assertion)");
}

#[test]
fn glue_481_4813b0_vec_add() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let edi = emu.alloc(0x10); // accumulator vec3 for the [edi] block
    let esi = emu.alloc(0x10); // accumulator vec3 for the [esi] block

    // [edi] block 0x481466..0x48148a: the source x is already at [ebp-0x10] (the 0x481463
    // `mov [ebp-0x10],eax` precedes the slice), and y/z come from ECX/EDX (the slice's interleaved
    // `mov [ebp-0xc],ecx; mov [ebp-0x8],edx` write them before the fld). fadd ×3 (0x481469/47b/484).
    let run_edi = |emu: &mut Emu, d: [f32; 3], s: [f32; 3]| -> [f32; 3] {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::EDI, edi);
        emu.write_u32(ebp - 0x10, s[0].to_bits()); // src.x (loaded at 0x481466)
        emu.set_reg(RegisterX86::ECX, u64::from(s[1].to_bits())); // → [ebp-0xc] = src.y
        emu.set_reg(RegisterX86::EDX, u64::from(s[2].to_bits())); // → [ebp-0x8] = src.z
        for (i, &di) in d.iter().enumerate() {
            emu.write_u32(edi + 4 * i as u64, di.to_bits());
        }
        emu.run(0x481466, 0x48148a); // halt at `pop edi` (fstp [edi+8] ends at 0x48148a)
        [
            emu.read_f32(edi),
            emu.read_f32(edi + 4),
            emu.read_f32(edi + 8),
        ]
    };

    // [esi] block 0x48148b..0x4814a4: reads src.{x,y,z} from [ebp-0x10]/[ebp-0xc]/[ebp-0x8] (no
    // interleaved register movs — clean). fadd ×3 (0x48148e/495/49e).
    let run_esi = |emu: &mut Emu, d: [f32; 3], s: [f32; 3]| -> [f32; 3] {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESI, esi);
        emu.write_u32(ebp - 0x10, s[0].to_bits());
        emu.write_u32(ebp - 0xc, s[1].to_bits());
        emu.write_u32(ebp - 0x8, s[2].to_bits());
        for (i, &di) in d.iter().enumerate() {
            emu.write_u32(esi + 4 * i as u64, di.to_bits());
        }
        emu.run(0x48148b, 0x4814a4);
        [
            emu.read_f32(esi),
            emu.read_f32(esi + 4),
            emu.read_f32(esi + 8),
        ]
    };

    let mut total = 0u32;
    let probes: [([f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([1.0, 2.0, 3.0], [-1.0, -2.0, -3.0]),
        ([16_777_216.0, 1.0, -1.0], [1.0, 16_777_216.0, 0.0]),
        ([1e30, -1e30, 0.5], [1e30, 1e30, 0.25]),
        ([0.1, 0.2, 0.3], [0.4, 0.5, 0.6]),
        ([f32::from_bits(0x4b00_0001), 7.5, -7.5], [1.0, 0.0, 7.5]),
    ];
    for (d, s) in probes {
        let we = run_edi(&mut emu, d, s);
        let wf = run_esi(&mut emu, d, s);
        let want = vec3_add_inplace(d, s);
        for i in 0..3 {
            assert_eq!(
                we[i].to_bits(),
                want[i].to_bits(),
                "edi add d={d:?} s={s:?} i={i}"
            );
            assert_eq!(
                wf[i].to_bits(),
                want[i].to_bits(),
                "esi add d={d:?} s={s:?} i={i}"
            );
        }
        total += 6;
    }
    let mut st = 0x4814_6600u32;
    for _ in 0..2000 {
        let d = [
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
        ];
        let s = [
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
            rng(&mut st, -10000.0, 10000.0),
        ];
        let we = run_edi(&mut emu, d, s);
        let wf = run_esi(&mut emu, d, s);
        let want = vec3_add_inplace(d, s);
        for i in 0..3 {
            assert_eq!(we[i].to_bits(), want[i].to_bits(), "edi add fuzz i={i}");
            assert_eq!(wf[i].to_bits(), want[i].to_bits(), "esi add fuzz i={i}");
        }
        total += 6;
    }
    eprintln!(
        "glue_481_4813b0_vec_add: {total} bit-exact (fadd×6: [edi] block ×3 + [esi] block ×3)"
    );
}
