//! Bit-exact diff for the camera box-extent accumulator `0x50ca90` owned arithmetic
//! (`src/camera_pivot.rs`) vs `WoW.exe`. The full function is heavy orchestration over
//! key-binding / resource enumerators (`0x7103d0`/`0x711960`/`0x712cb0`/`0x712cf0`/`0x713680`/
//! `0x713700`/`0x711a20`) and CString builders (`0x4078e0`/`0x4b1df0`), un-synthesizable in
//! isolation — so we diff its owned FP via **owned-arithmetic slices** (`Emu::run`) + one verbatim
//! st-arith **island** (the scale scalar, whose live st0 input is a foreign vtable result). Each
//! slice sets `esi` (the camera object) and/or the `[ebp-…]` locals the binary's callees fill, runs
//! the binary's own bytes, and reads the stored result.
//!
//! The load-bearing slice is the final **clamp** `[0x50cfa7, 0x50d098)` (3 components → `[5/6, 15]`),
//! whose x87 NaN-aware `fcom;fnstsw;test ah,5;jp/jnp` disposition carries the per-component AH-branch
//! hazard; it gets deterministic `==threshold` probes at both bounds plus NaN/±Inf coverage.
//!
//! **Op coverage: every in-body FP-arith opcode lands in a diffed slice or island**
//! — clamp 9 fcom; A1 fadd+fmul; A2 fsub+fmul+fmul; fold 3 fadd; scale-scalar 2 fmul; parallax-combine
//! fsub+fsubr; **z-accum** fmul+fmul+fsub+fsubr (`0x50cce9/cef/cf4/cf6`); **scale clusters S1-S4**
//! 12 fmul (`0x50cd82/d92/d9b/dad/dc2/dd1` + `0x50cf11/f27/f36/f48/f54/f5d`); **x/y accum** 3 fsub
//! (`0x50cde5/df1/ce01`); **CE44** fmul (`0x50ce44`); **parallax tail** 3 fadd (`0x50cf7a/f89/f98`).
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::camera_pivot::{
    accum_a1, accum_a2, accum_fold, accum_xy, accum_z, clamp_box_extent, parallax_combine,
    parallax_tail, scale_ce44, scale_component, scale_scalar, CLAMP_HI, CLAMP_LO,
};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A fuzz draw, high bits only, in `[lo, hi)`.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

fn assert_bits(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}

struct Frame {
    ebp: u64,
    esp: u64,
    esi: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let scratch = emu.alloc(0x100); // [ebp-…] locals live below ebp
    let ebp = scratch + 0xc0;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    let esi = emu.alloc(0x200); // camera object `this` for [esi+0x11c/0x120/0x124]
    Frame { ebp, esp, esi }
}

// ===========================================================================================
// Slice 1 — the box-extent clamp [0x50cfa7, 0x50d098): each of this+0x11c/+0x120/+0x124 clamped
// to [0.8333…, 15.0] via the x87 NaN-aware fcom/fnstsw/test ah,5/jp/jnp disposition. Reads esi +
// the three f32 components; writes them back. No stack/ebp use inside the slice.
// arithmetic covered: fcom @ cfad/d00d/d02e/d045/d066/d07d (+ the two upper-bound fcoms).
// ===========================================================================================
#[test]
fn glue_50c_clamp_box_extent() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let clamp_one = |emu: &mut Emu, c: [f32; 3]| -> [f32; 3] {
        emu.write_u32(f.esi + 0x11c, c[0].to_bits());
        emu.write_u32(f.esi + 0x120, c[1].to_bits());
        emu.write_u32(f.esi + 0x124, c[2].to_bits());
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.run(0x50cfa7, 0x50d098);
        [
            emu.read_f32(f.esi + 0x11c),
            emu.read_f32(f.esi + 0x120),
            emu.read_f32(f.esi + 0x124),
        ]
    };

    let lo = CLAMP_LO;
    let hi = CLAMP_HI;
    // Deterministic boundary probes: exactly each bound, ±1 ulp, NaN payloads, ±Inf, 0, negatives.
    let probes: [f32; 16] = [
        lo,
        f32::from_bits(lo.to_bits() - 1), // just below LO
        f32::from_bits(lo.to_bits() + 1), // just above LO
        hi,
        f32::from_bits(hi.to_bits() - 1), // just below HI
        f32::from_bits(hi.to_bits() + 1), // just above HI
        0.0,
        -0.0,
        -5.0,
        1.0,
        f32::NAN,
        f32::from_bits(0xffc0_0000), // a different NaN payload
        f32::INFINITY,
        f32::NEG_INFINITY,
        20.0,
        0.5,
    ];

    let mut asserts = 0usize;
    // Rotate each probe through all three component slots.
    for i in 0..probes.len() {
        let c = [
            probes[i],
            probes[(i + 5) % probes.len()],
            probes[(i + 11) % probes.len()],
        ];
        let got = clamp_one(&mut emu, c);
        for k in 0..3 {
            assert_bits(
                got[k],
                clamp_box_extent(c[k]),
                &format!("clamp probe[{i}].{k}"),
            );
            asserts += 1;
        }
    }
    let mut s = 0x50ca_9001u32;
    for _ in 0..3000 {
        let c = [
            rng(&mut s, -4.0, 20.0),
            rng(&mut s, -4.0, 20.0),
            rng(&mut s, -4.0, 20.0),
        ];
        let got = clamp_one(&mut emu, c);
        for k in 0..3 {
            assert_bits(got[k], clamp_box_extent(c[k]), &format!("clamp fuzz.{k}"));
            asserts += 1;
        }
    }
    eprintln!(
        "glue_50c_clamp_box_extent: {asserts} bit-exact asserts (3-component [5/6,15] clamp)"
    );
}

// ===========================================================================================
// Slice 2 — accum-branch A1 + fold [0x50cbe1, 0x50cc34): d = (local_14 + 0.0972…)·scalar, folded
// into all three components (cN = d + cN). local_14 @ [ebp-0x10], scalar @ [ebp-4]. The body's
// interleaved `push ebx`/`mov ecx,edi` need a valid esp (set, harmless to the FP).
// arithmetic covered: fadd @ cbe4, fmul @ cbea, fadd @ cc0c/cc1c/cc28.
// ===========================================================================================
#[test]
fn glue_50c_accum_a1_fold() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run_a1 = |emu: &mut Emu, v14: f32, scalar: f32, c: [f32; 3]| -> [f32; 3] {
        emu.write_u32(f.ebp - 0x10, v14.to_bits());
        emu.write_u32(f.ebp - 0x04, scalar.to_bits());
        emu.write_u32(f.esi + 0x11c, c[0].to_bits());
        emu.write_u32(f.esi + 0x120, c[1].to_bits());
        emu.write_u32(f.esi + 0x124, c[2].to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.run(0x50cbe1, 0x50cc34);
        [
            emu.read_f32(f.esi + 0x11c),
            emu.read_f32(f.esi + 0x120),
            emu.read_f32(f.esi + 0x124),
        ]
    };

    let mut asserts = 0usize;
    let mut s = 0xa1a1_0001u32;
    for _ in 0..1200 {
        let v14 = rng(&mut s, -10.0, 10.0);
        let scalar = rng(&mut s, -5.0, 5.0);
        let c = [
            rng(&mut s, -4.0, 20.0),
            rng(&mut s, -4.0, 20.0),
            rng(&mut s, -4.0, 20.0),
        ];
        let got = run_a1(&mut emu, v14, scalar, c);
        let want = accum_fold(accum_a1(v14, scalar), c);
        for k in 0..3 {
            assert_bits(got[k], want[k], &format!("accum_a1+fold.{k}"));
            asserts += 1;
        }
    }
    eprintln!("glue_50c_accum_a1_fold: {asserts} bit-exact asserts");
}

// ===========================================================================================
// Slice 3 — accum-branch A2 + fold [0x50cbfa, 0x50cc34): d = ((local_20 - local_2c)·scalar)·0.9,
// folded into all three components. local_20 @ [ebp-0x20], local_2c @ [ebp-0x2c], scalar @ [ebp-4].
// Note 0x50cbfa is preceded by the 0x713700 fill `call` (orchestration) — we enter past it.
// arithmetic covered: fsub @ cbfd, fmul @ cc00, fmul @ cc03, fadd @ cc0c/cc1c/cc28.
// ===========================================================================================
#[test]
fn glue_50c_accum_a2_fold() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run_a2 = |emu: &mut Emu, v20: f32, v2c: f32, scalar: f32, c: [f32; 3]| -> [f32; 3] {
        emu.write_u32(f.ebp - 0x20, v20.to_bits());
        emu.write_u32(f.ebp - 0x2c, v2c.to_bits());
        emu.write_u32(f.ebp - 0x04, scalar.to_bits());
        emu.write_u32(f.esi + 0x11c, c[0].to_bits());
        emu.write_u32(f.esi + 0x120, c[1].to_bits());
        emu.write_u32(f.esi + 0x124, c[2].to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.run(0x50cbfa, 0x50cc34);
        [
            emu.read_f32(f.esi + 0x11c),
            emu.read_f32(f.esi + 0x120),
            emu.read_f32(f.esi + 0x124),
        ]
    };

    let mut asserts = 0usize;
    let mut s = 0xa2a2_0001u32;
    for _ in 0..1200 {
        let v20 = rng(&mut s, -10.0, 10.0);
        let v2c = rng(&mut s, -10.0, 10.0);
        let scalar = rng(&mut s, -5.0, 5.0);
        let c = [
            rng(&mut s, -4.0, 20.0),
            rng(&mut s, -4.0, 20.0),
            rng(&mut s, -4.0, 20.0),
        ];
        let got = run_a2(&mut emu, v20, v2c, scalar, c);
        let want = accum_fold(accum_a2(v20, v2c, scalar), c);
        for k in 0..3 {
            assert_bits(got[k], want[k], &format!("accum_a2+fold.{k}"));
            asserts += 1;
        }
    }
    eprintln!("glue_50c_accum_a2_fold: {asserts} bit-exact asserts");
}

// ===========================================================================================
// Slice 4 — the parallax/ortho combine [0x50cf68, 0x50cf77): out = [ebp-8] - ([ebp-0x8c]-[ebp-0x50])
//   fld dword [ebp-0x8c]; fsub dword [ebp-0x50]; fsubr dword [ebp-8]; fstp dword [ebp-8].
// Reads/writes only ebp-locals. arithmetic covered: fsub @ cf6e, fsubr @ cf71.
// ===========================================================================================
#[test]
fn glue_50c_parallax_combine() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run_p = |emu: &mut Emu, a: f32, b: f32, inv: f32| -> f32 {
        emu.write_u32(f.ebp - 0x8c, a.to_bits());
        emu.write_u32(f.ebp - 0x50, b.to_bits());
        emu.write_u32(f.ebp - 0x08, inv.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(0x50cf68, 0x50cf77);
        emu.read_f32(f.ebp - 0x08)
    };

    let mut asserts = 0usize;
    // Boundary: a==b (Δ=0), inv exactly 0, equal-opposite, the clamp bounds.
    for &(a, b, inv) in &[
        (CLAMP_LO, CLAMP_LO, 0.0f32),
        (1.0, 1.0, CLAMP_HI),
        (3.0, -3.0, 0.5),
        (0.0, 0.0, 0.0),
        (-2.5, 2.5, 1.0),
    ] {
        assert_bits(
            run_p(&mut emu, a, b, inv),
            parallax_combine(a, b, inv),
            "parallax boundary",
        );
        asserts += 1;
    }
    let mut s = 0x9a9a_0001u32;
    for _ in 0..1200 {
        let a = rng(&mut s, -20.0, 20.0);
        let b = rng(&mut s, -20.0, 20.0);
        let inv = rng(&mut s, -20.0, 20.0);
        assert_bits(
            run_p(&mut emu, a, b, inv),
            parallax_combine(a, b, inv),
            "parallax fuzz",
        );
        asserts += 1;
    }
    eprintln!("glue_50c_parallax_combine: {asserts} bit-exact asserts");
}

// ===========================================================================================
// Slice 5 — the scale scalar [0x50cb27, 0x50cb3b): scalar = (r · [ebx+0x94]) · local_10, where `r`
// is the foreign vtable result on st0. We can't preload st0 for a plain `run`, so this is a verbatim
// st-arith ISLAND: a `fld dword [r]` preamble loads `r`, then the VERBATIM source bytes
// `d8 4d fc d8 4d f4 d9 5d fc` (fmul [ebp-4]; fmul [ebp-0xc]; fstp [ebp-4]) execute with ebp set so
// [ebp-4]=f94, [ebp-0xc]=mul, and `fstp [ebp-4]` writes the result back (read from [ebp-4]).
// arithmetic covered: fmul @ cb27, fmul @ cb30.
// ===========================================================================================
#[test]
fn glue_50c_scale_scalar() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Byte-verify the three FP-arith source ops at their exact addresses (the integer ops
    // 0x50cb2a/0x50cb33 interleaved between them are the branch-selector orchestration, excluded).
    let op_fmul1 = emu.read(0x50cb27, 3); // fmul dword [ebp-4]
    let op_fmul2 = emu.read(0x50cb30, 3); // fmul dword [ebp-0xc]
    let op_fstp = emu.read(0x50cb3b, 3); // fstp dword [ebp-4]
    assert_eq!(op_fmul1, vec![0xd8, 0x4d, 0xfc], "0x50cb27 fmul [ebp-4]");
    assert_eq!(op_fmul2, vec![0xd8, 0x4d, 0xf4], "0x50cb30 fmul [ebp-0xc]");
    assert_eq!(op_fstp, vec![0xd9, 0x5d, 0xfc], "0x50cb3b fstp [ebp-4]");
    // The island = fld [r_cell] + the three verbatim FP ops (ebp-relative, run with ebp set).
    let r_cell = emu.alloc(4);
    let region = emu.alloc(0x40);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [r_cell]
    code.extend_from_slice(&(r_cell as u32).to_le_bytes());
    code.extend_from_slice(&op_fmul1); // fmul dword [ebp-4]  (= f94)
    code.extend_from_slice(&op_fmul2); // fmul dword [ebp-0xc] (= mul)
    code.extend_from_slice(&op_fstp); // fstp dword [ebp-4]  (store scalar)
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run_scale = |emu: &mut Emu, r: f32, f94: f32, mul: f32| -> f32 {
        emu.write_u32(r_cell, r.to_bits());
        emu.write_u32(f.ebp - 0x04, f94.to_bits()); // [ebp-4] = f94, also the result slot
        emu.write_u32(f.ebp - 0x0c, mul.to_bits()); // [ebp-0xc] = local_10
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(region, end);
        emu.read_f32(f.ebp - 0x04)
    };

    let mut asserts = 0usize;
    for &(r, f94, mul) in &[
        (1.0f32, 1.0f32, 1.0f32),
        (0.0, 5.0, 3.0),
        (-2.0, 0.5, 4.0),
        (16_777_217.0, 1.0, 1.0), // f32 rounding boundary
        (CLAMP_HI, CLAMP_LO, 1.0),
    ] {
        assert_bits(
            run_scale(&mut emu, r, f94, mul),
            scale_scalar(r, f94, mul),
            "scale boundary",
        );
        asserts += 1;
    }
    let mut s = 0x5ca1_e001u32;
    for _ in 0..2000 {
        let r = rng(&mut s, -10.0, 10.0);
        let f94 = rng(&mut s, -10.0, 10.0);
        let mul = rng(&mut s, -10.0, 10.0);
        assert_bits(
            run_scale(&mut emu, r, f94, mul),
            scale_scalar(r, f94, mul),
            "scale fuzz",
        );
        asserts += 1;
    }
    eprintln!("glue_50c_scale_scalar: {asserts} bit-exact asserts (verbatim st-arith island)");
}

// ===========================================================================================
// Slice 6 — the z-axis box accum [0x50cce3, 0x50cd04): this+0x124 -= (L8c·s - L50·s), where the
// two products and their difference ride LIVE on the x87 stack (st-vs-st fsub, no f32 spill). Reads
// [ebp-0x8c], [ebp-0x50], [ebp-4], [esi+0x124]; writes [esi+0x124]. The trailing `fstp st(0)`
// (0x50cd02) drops the leftover L50·s — covered by halting at 0x50cd04. Needs valid esp (no pushes,
// but reset_fp expects a clean entry).
// arithmetic covered: fmul @ cce9, fmul @ ccef, fsub @ ccf4 (st,st(1)), fsubr @ ccf6.
// ===========================================================================================
#[test]
fn glue_50c_accum_z() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run_z = |emu: &mut Emu, a: f32, b: f32, scalar: f32, z: f32| -> f32 {
        emu.write_u32(f.ebp - 0x8c, a.to_bits());
        emu.write_u32(f.ebp - 0x50, b.to_bits());
        emu.write_u32(f.ebp - 0x04, scalar.to_bits());
        emu.write_u32(f.esi + 0x124, z.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.run(0x50cce3, 0x50cd04);
        emu.read_f32(f.esi + 0x124)
    };

    let mut asserts = 0usize;
    // Boundary: a==b (product diff exactly 0), scalar 0/1, z at clamp bounds, f32-rounding scalar.
    for &(a, b, scalar, z) in &[
        (1.0f32, 1.0f32, 1.0f32, CLAMP_HI),
        (3.0, 3.0, 2.0, 0.0),
        (5.0, -5.0, 0.0, CLAMP_LO),
        (16_777_217.0, 1.0, 1.0, 1.0),
        (-2.5, 2.5, 4.0, 10.0),
    ] {
        assert_bits(
            run_z(&mut emu, a, b, scalar, z),
            accum_z(a, b, scalar, z),
            "accum_z boundary",
        );
        asserts += 1;
    }
    let mut s = 0x2222_5a01u32;
    for _ in 0..2000 {
        let a = rng(&mut s, -10.0, 10.0);
        let b = rng(&mut s, -10.0, 10.0);
        let scalar = rng(&mut s, -5.0, 5.0);
        let z = rng(&mut s, -4.0, 20.0);
        assert_bits(
            run_z(&mut emu, a, b, scalar, z),
            accum_z(a, b, scalar, z),
            "accum_z fuzz",
        );
        asserts += 1;
    }
    eprintln!("glue_50c_accum_z: {asserts} bit-exact asserts (z-axis live-st0 product diff)");
}

// ===========================================================================================
// Slice 7 — the per-component scale transform. The SAME arithmetic (loc = (loc·scalar) as f32,
// fld;fmul;fstp with a full f32 store between) appears as four clusters:
//   S1 [0x50cd7f, 0x50cda1): [ebp-0x64], [ebp-0x60], [ebp-0x5c]
//   S2 [0x50cda6, 0x50cdda): [ebp-0xa0], [ebp-0x9c], [ebp-0x98]
//   S3 [0x50cf0b, 0x50cf3f): [ebp-0xa0], [ebp-0x9c], [ebp-0x98]
//   S4 [0x50cf44, 0x50cf63): [ebp-0x64], [ebp-0x60], [ebp-0x5c]
// Each cluster does three independent scales; we diff every one of the four slices (driving all 12
// fmuls). The interleaved `push edi`/`lea ecx` integer ops need a valid esp (set).
// arithmetic covered: fmul @ cd82/cd92/cd9b/cdad/cdc2/cdd1 (S1,S2) + cf11/cf27/cf36/cf48/cf54/cf5d
// (S3,S4).
// ===========================================================================================
#[test]
fn glue_50c_scale_component() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Each cluster: (begin, end, [the three [ebp-off] slots it scales]).
    let clusters: [(u64, u64, [u64; 3]); 4] = [
        (0x50cd7f, 0x50cda1, [0x64, 0x60, 0x5c]),
        (0x50cda6, 0x50cdda, [0xa0, 0x9c, 0x98]),
        (0x50cf0b, 0x50cf3f, [0xa0, 0x9c, 0x98]),
        (0x50cf44, 0x50cf63, [0x64, 0x60, 0x5c]),
    ];

    let mut asserts = 0usize;
    let mut s = 0x5ca1_c001u32;
    for (ci, &(begin, end, slots)) in clusters.iter().enumerate() {
        // Deterministic boundary cases interleaved with fuzz.
        let cases: [(f32, [f32; 3]); 3] = [
            (1.0, [CLAMP_LO, CLAMP_HI, 0.0]),
            (0.0, [3.0, -3.0, 7.5]),
            (16_777_217.0, [1.0, -1.0, 2.0]), // f32-rounding magnitude
        ];
        for (v, vals) in cases {
            // Load each slot with a distinct value from `vals`; `v` is the scalar @ [ebp-4].
            emu.write_u32(f.ebp - slots[0], vals[0].to_bits());
            emu.write_u32(f.ebp - slots[1], vals[1].to_bits());
            emu.write_u32(f.ebp - slots[2], vals[2].to_bits());
            emu.write_u32(f.ebp - 0x04, v.to_bits()); // scalar @ [ebp-4]
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.run(begin, end);
            for (k, off) in slots.iter().enumerate() {
                assert_bits(
                    emu.read_f32(f.ebp - off),
                    scale_component(vals[k], v),
                    &format!("scale_component boundary cluster{ci}.{k}"),
                );
                asserts += 1;
            }
        }
        for _ in 0..1000 {
            let v = rng(&mut s, -5.0, 5.0);
            let vals = [
                rng(&mut s, -20.0, 20.0),
                rng(&mut s, -20.0, 20.0),
                rng(&mut s, -20.0, 20.0),
            ];
            emu.write_u32(f.ebp - slots[0], vals[0].to_bits());
            emu.write_u32(f.ebp - slots[1], vals[1].to_bits());
            emu.write_u32(f.ebp - slots[2], vals[2].to_bits());
            emu.write_u32(f.ebp - 0x04, v.to_bits());
            emu.set_reg(RegisterX86::EBP, f.ebp);
            emu.set_reg(RegisterX86::ESP, f.esp);
            emu.run(begin, end);
            for (k, off) in slots.iter().enumerate() {
                assert_bits(
                    emu.read_f32(f.ebp - off),
                    scale_component(vals[k], v),
                    &format!("scale_component fuzz cluster{ci}.{k}"),
                );
                asserts += 1;
            }
        }
    }
    eprintln!("glue_50c_scale_component: {asserts} bit-exact asserts (4 clusters × 3 components)");
}

// ===========================================================================================
// Slice 8 — the CE44 single-fmul spill: [ebp-0x8] = (local_20·scalar) as f32.
//   0x50ce3e: fld  dword [ebp-0x20]    (d9 45 e0)
//   0x50ce41: mov  edx,[ebp+8]         (integer orchestration — interleaved)
//   0x50ce44: fmul dword [ebp-4]       (d8 4d fc)
//   0x50ce47: mov  eax,[edx+0x110] ; mov ecx,[eax+0xa0] ; shr ecx,0x14  (a [ebp+8]→+0x110→+0xa0
//             mode-flag deref — pure orchestration, needs a fully-built param_2 object)
//   0x50ce56: fstp dword [ebp-0x8]     (d9 5d f8)
// The `fld` and the `fstp` are split by an un-synthesizable foreign-object deref, so we diff this as
// a verbatim st-arith ISLAND (, like scale_scalar): byte-verify the three FP source ops at
// their exact addresses, assemble JUST them in scratch (skipping the integer deref), run with ebp
// set so [ebp-0x20]=v20, [ebp-4]=scalar, and read [ebp-0x8] back.
// arithmetic covered: fmul @ ce44.
// ===========================================================================================
#[test]
fn glue_50c_scale_ce44() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // Byte-verify the three FP source ops at their exact addresses (the integer deref between fmul
    // and fstp is the excluded orchestration).
    let op_fld = emu.read(0x50ce3e, 3); // fld  dword [ebp-0x20]
    let op_fmul = emu.read(0x50ce44, 3); // fmul dword [ebp-4]
    let op_fstp = emu.read(0x50ce56, 3); // fstp dword [ebp-0x8]
    assert_eq!(op_fld, vec![0xd9, 0x45, 0xe0], "0x50ce3e fld [ebp-0x20]");
    assert_eq!(op_fmul, vec![0xd8, 0x4d, 0xfc], "0x50ce44 fmul [ebp-4]");
    assert_eq!(op_fstp, vec![0xd9, 0x5d, 0xf8], "0x50ce56 fstp [ebp-0x8]");
    let region = emu.alloc(0x40);
    let mut code = Vec::new();
    code.extend_from_slice(&op_fld); // fld  dword [ebp-0x20]
    code.extend_from_slice(&op_fmul); // fmul dword [ebp-4]
    code.extend_from_slice(&op_fstp); // fstp dword [ebp-0x8]
    let end = region + code.len() as u64;
    emu.write(region, &code);

    let run_ce44 = |emu: &mut Emu, v20: f32, scalar: f32| -> f32 {
        emu.write_u32(f.ebp - 0x20, v20.to_bits());
        emu.write_u32(f.ebp - 0x04, scalar.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.run(region, end);
        emu.read_f32(f.ebp - 0x08)
    };

    let mut asserts = 0usize;
    for &(v20, scalar) in &[
        (1.0f32, 1.0f32),
        (0.0, 5.0),
        (-2.0, 4.0),
        (16_777_217.0, 1.0),
        (CLAMP_HI, CLAMP_LO),
    ] {
        assert_bits(
            run_ce44(&mut emu, v20, scalar),
            scale_ce44(v20, scalar),
            "scale_ce44 boundary",
        );
        asserts += 1;
    }
    let mut s = 0xce44_0001u32;
    for _ in 0..1500 {
        let v20 = rng(&mut s, -20.0, 20.0);
        let scalar = rng(&mut s, -5.0, 5.0);
        assert_bits(
            run_ce44(&mut emu, v20, scalar),
            scale_ce44(v20, scalar),
            "scale_ce44 fuzz",
        );
        asserts += 1;
    }
    eprintln!("glue_50c_scale_ce44: {asserts} bit-exact asserts");
}

// ===========================================================================================
// Slice 9 — the x/y box accum [0x50cde2, 0x50ce0b): d = (L50 - L8c) LIVE on st0;
//   this+0x11c -= d; then `cmp ebx,0x5b; jne 0x50ce09` — if NOT-taken (ZF=1) also this+0x120 -= d.
// `d` rides st1 across both subtractions un-spilled. We enter past the `cmp` (0x50cddf), so `run`'s
// reset_fp does NOT touch EFLAGS — we set EFLAGS directly to drive both legs:
//   0x246 (ZF=1) -> jne not-taken -> y-leg runs;  0x202 (ZF=0) -> jne taken -> y-leg skipped.
// Halt at 0x50ce0b (covers the y-leg + the trailing `fstp st(0)` @ 0x50ce09). Reads [ebp-0x50],
// [ebp-0x8c], [esi+0x11c], [esi+0x120]; writes [esi+0x11c] (+ [esi+0x120] iff y-leg).
// arithmetic covered: fsub @ cde5, fsub @ cdf1 (st,st(1)), fsub @ ce01 (st,st(1)).
// ===========================================================================================
#[test]
fn glue_50c_accum_xy() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run_xy = |emu: &mut Emu, a: f32, b: f32, cx: f32, cy: f32, do_y: bool| -> (f32, f32) {
        emu.write_u32(f.ebp - 0x50, a.to_bits());
        emu.write_u32(f.ebp - 0x8c, b.to_bits());
        emu.write_u32(f.esi + 0x11c, cx.to_bits());
        emu.write_u32(f.esi + 0x120, cy.to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        // ZF=1 (0x246) -> jne not-taken -> y-leg; ZF=0 (0x202) -> jne taken -> skip y-leg.
        emu.set_reg(RegisterX86::EFLAGS, if do_y { 0x246 } else { 0x202 });
        emu.run(0x50cde2, 0x50ce0b);
        (emu.read_f32(f.esi + 0x11c), emu.read_f32(f.esi + 0x120))
    };

    let mut asserts = 0usize;
    // Both branch legs, deterministic boundaries: a==b (d=0), cN at clamp bounds, f32-round magnitudes.
    for do_y in [true, false] {
        for &(a, b, cx, cy) in &[
            (1.0f32, 1.0f32, CLAMP_HI, CLAMP_LO),
            (3.0, -3.0, 0.0, 10.0),
            (16_777_217.0, 1.0, 2.0, -2.0),
            (-5.0, 5.0, 7.5, 1.25),
        ] {
            let (gx, gy) = run_xy(&mut emu, a, b, cx, cy, do_y);
            let (wx, wy) = accum_xy(a, b, cx, cy, do_y);
            assert_bits(gx, wx, &format!("accum_xy boundary x (do_y={do_y})"));
            assert_bits(gy, wy, &format!("accum_xy boundary y (do_y={do_y})"));
            asserts += 2;
        }
    }
    let mut s = 0x7878_0001u32;
    for i in 0..2000 {
        let do_y = (next(&mut s) >> 8) & 1 == 0; // decide from high bits
        let a = rng(&mut s, -10.0, 10.0);
        let b = rng(&mut s, -10.0, 10.0);
        let cx = rng(&mut s, -4.0, 20.0);
        let cy = rng(&mut s, -4.0, 20.0);
        let (gx, gy) = run_xy(&mut emu, a, b, cx, cy, do_y);
        let (wx, wy) = accum_xy(a, b, cx, cy, do_y);
        assert_bits(gx, wx, &format!("accum_xy fuzz x [{i}]"));
        assert_bits(gy, wy, &format!("accum_xy fuzz y [{i}]"));
        asserts += 2;
    }
    eprintln!("glue_50c_accum_xy: {asserts} bit-exact asserts (both jne legs, live-st1 d)");
}

// ===========================================================================================
// Slice 10 — the parallax tail [0x50cf77, 0x50cfa4): this+0x11c/+0x120/+0x124 += [ebp-0x8]. Each
// axis is an independent fld [ebp-8]; fadd [esi+N]; fstp [esi+N] (inv reloaded f32, every product
// narrowed). Reads [ebp-0x8] + the three components; writes the three components.
// arithmetic covered: fadd @ cf7a, fadd @ cf89, fadd @ cf98.
// ===========================================================================================
#[test]
fn glue_50c_parallax_tail() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let run_pt = |emu: &mut Emu, inv: f32, c: [f32; 3]| -> [f32; 3] {
        emu.write_u32(f.ebp - 0x08, inv.to_bits());
        emu.write_u32(f.esi + 0x11c, c[0].to_bits());
        emu.write_u32(f.esi + 0x120, c[1].to_bits());
        emu.write_u32(f.esi + 0x124, c[2].to_bits());
        emu.set_reg(RegisterX86::EBP, f.ebp);
        emu.set_reg(RegisterX86::ESP, f.esp);
        emu.set_reg(RegisterX86::ESI, f.esi);
        emu.run(0x50cf77, 0x50cfa4);
        [
            emu.read_f32(f.esi + 0x11c),
            emu.read_f32(f.esi + 0x120),
            emu.read_f32(f.esi + 0x124),
        ]
    };

    let mut asserts = 0usize;
    for &(inv, c) in &[
        (0.0f32, [CLAMP_LO, CLAMP_HI, 0.0]),
        (CLAMP_HI, [1.0, -1.0, 2.0]),
        (16_777_217.0, [1.0, 1.0, 1.0]),
        (-2.5, [10.0, -10.0, 5.0]),
    ] {
        let got = run_pt(&mut emu, inv, c);
        let want = parallax_tail(inv, c);
        for k in 0..3 {
            assert_bits(got[k], want[k], &format!("parallax_tail boundary.{k}"));
            asserts += 1;
        }
    }
    let mut s = 0x7a11_0001u32;
    for _ in 0..1500 {
        let inv = rng(&mut s, -20.0, 20.0);
        let c = [
            rng(&mut s, -4.0, 20.0),
            rng(&mut s, -4.0, 20.0),
            rng(&mut s, -4.0, 20.0),
        ];
        let got = run_pt(&mut emu, inv, c);
        let want = parallax_tail(inv, c);
        for k in 0..3 {
            assert_bits(got[k], want[k], &format!("parallax_tail fuzz.{k}"));
            asserts += 1;
        }
    }
    eprintln!("glue_50c_parallax_tail: {asserts} bit-exact asserts");
}
