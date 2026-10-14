//! Differential tests of the look-at frame (`0x50a9f0`), the orthonormal basis and the projection
//! aspect (`0x50ab70`) that feed the device view and projection matrices, each run against
//! `WoW.exe` in the emulator.
//!
//! Strategy: **whole-fn `call`** — `0x50a9f0` is pure inline x87 on the live path (the only callee,
//! `SErrSetLastError 0x64e850`, sits on the degenerate-`F` early-out we never feed, but is stubbed
//! defensively). One call covers every owned FP op (the two guard accumulations, `U×F`, the
//! normalize `fsqrt`/`fdivr`, and `F×out1`) by construction across BOTH branches (parallel ⇒
//! `out1=(1,0,0)`, non-parallel ⇒ `out1=normalize(U×F)`). We read all 9 output floats and diff bits.
//!
//! Cases exercise: non-parallel general frames, the up∥F parallel branch (a deterministic boundary —
//! `dot=1` ⇒ `s=0 < eps`), and wide fuzz on the non-parallel path. `F` magnitude is kept ≥ 1 so the
//! `|F|² ≥ eps` guard always passes (its degenerate branch writes no output, untestable by diff).
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::view_matrices::look_at_frame;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

#[test]
fn glue_509_50a9f0_lookat_frame() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x64e850, 0, 4); // SErrSetLastError(1 arg) — degenerate-F early-out, never reached here
    let fcell = emu.alloc(0x10);
    let ucell = emu.alloc(0x10);
    let o0 = emu.alloc(0x10);
    let o1 = emu.alloc(0x10);
    let o2 = emu.alloc(0x10);

    let run = |emu: &mut Emu, f: [f32; 3], up: [f32; 3]| -> ([f32; 3], [f32; 3], [f32; 3]) {
        for i in 0..3 {
            emu.write_u32(fcell + 4 * i as u64, f[i].to_bits());
            emu.write_u32(ucell + 4 * i as u64, up[i].to_bits());
        }
        // ecx = F ptr, edx = U ptr; stack args out0,out1,out2 (ret 0xc).
        emu.call(
            0x50a9f0,
            fcell as u32,
            ucell as u32,
            &[o0 as u32, o1 as u32, o2 as u32],
        );
        let rd = |emu: &Emu, base: u64| {
            [
                emu.read_f32(base),
                emu.read_f32(base + 4),
                emu.read_f32(base + 8),
            ]
        };
        (rd(emu, o0), rd(emu, o1), rd(emu, o2))
    };

    let cmp =
        |tag: &str, got: ([f32; 3], [f32; 3], [f32; 3]), want: ([f32; 3], [f32; 3], [f32; 3])| {
            for (k, (g, w)) in [(got.0, want.0), (got.1, want.1), (got.2, want.2)]
                .iter()
                .enumerate()
            {
                for j in 0..3 {
                    assert_eq!(
                        g[j].to_bits(),
                        w[j].to_bits(),
                        "{tag}: out{k}[{j}] got={} want={}",
                        g[j],
                        w[j]
                    );
                }
            }
        };

    let mut total = 0u32;
    // deterministic: axis frames (non-parallel), the up∥F parallel branch, oblique frames.
    let cases: &[([f32; 3], [f32; 3])] = &[
        ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),   // non-parallel, dot=0
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),   // non-parallel
        ([0.0, 0.0, 1.0], [0.0, 0.0, 1.0]),   // PARALLEL: dot=1 → s=0 < eps → out1=(1,0,0)
        ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),   // PARALLEL
        ([0.0, 0.0, -1.0], [0.0, 0.0, 1.0]),  // anti-parallel: dot=-1 → |−1|−1=0 → PARALLEL
        ([3.0, 4.0, 0.0], [0.0, 0.0, 1.0]),   // |F|=5, non-parallel
        ([1.0, 2.0, 2.0], [0.0, 1.0, 0.0]),   // |F|=3 oblique
        ([-2.0, 1.0, 3.0], [0.2, 0.9, -0.1]), // oblique up-hint
        ([10.0, -5.0, 2.0], [0.0, 1.0, 0.0]),
        ([0.577, 0.577, 0.577], [0.0, 1.0, 0.0]),
    ];
    for &(f, up) in cases {
        cmp("det", run(&mut emu, f, up), look_at_frame(f, up));
        total += 1;
    }

    // fuzz the non-parallel path: |F| ≥ ~1 (guard passes), up-hint a unit-ish vector.
    let mut s = 0x50a9_f000u32;
    for _ in 0..6000 {
        let f = [
            rng(&mut s, -50.0, 50.0),
            rng(&mut s, -50.0, 50.0),
            rng(&mut s, -50.0, 50.0),
        ];
        // keep |f| away from 0 (degenerate-F guard); components in ±50 give |f|²≫eps unless all tiny.
        if f[0].abs() < 1.0 && f[1].abs() < 1.0 && f[2].abs() < 1.0 {
            continue;
        }
        let up = [
            rng(&mut s, -1.0, 1.0),
            rng(&mut s, -1.0, 1.0),
            rng(&mut s, -1.0, 1.0),
        ];
        cmp("fuzz", run(&mut emu, f, up), look_at_frame(f, up));
        total += 1;
    }
    eprintln!("glue_509_50a9f0_lookat_frame: {total} bit-exact (whole-fn: U×F, normalize, F×out1, both branches)");
}

// ===========================================================================================
// 0x50a840 — orthonormal basis. Whole-fn `call`: ecx=in ptr, edx=out0 ptr, stack [out1,out2]
// (ret 8). The delegated normalize `0x456280` runs native; SErrSetLastError stubbed (degenerate-in
// early-out never fed). Cases include the |in.xy|²<eps degenerate-xy branch (tiny x,y + large z ⇒
// out1=(1,0,0)) and the general perp-normalize branch.
// ===========================================================================================
#[test]
fn glue_509_50a840_orthonormal_basis() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x64e850, 0, 4); // SErrSetLastError — degenerate-in early-out, never reached here
    let icell = emu.alloc(0x10);
    let o0 = emu.alloc(0x10);
    let o1 = emu.alloc(0x10);
    let o2 = emu.alloc(0x10);

    let run = |emu: &mut Emu, inp: [f32; 3]| -> ([f32; 3], [f32; 3], [f32; 3]) {
        for (i, v) in inp.iter().enumerate() {
            emu.write_u32(icell + 4 * i as u64, v.to_bits());
        }
        emu.call(0x50a840, icell as u32, o0 as u32, &[o1 as u32, o2 as u32]);
        let rd = |emu: &Emu, base: u64| {
            [
                emu.read_f32(base),
                emu.read_f32(base + 4),
                emu.read_f32(base + 8),
            ]
        };
        (rd(emu, o0), rd(emu, o1), rd(emu, o2))
    };
    let cmp =
        |tag: &str, got: ([f32; 3], [f32; 3], [f32; 3]), want: ([f32; 3], [f32; 3], [f32; 3])| {
            for (k, (g, w)) in [(got.0, want.0), (got.1, want.1), (got.2, want.2)]
                .iter()
                .enumerate()
            {
                for j in 0..3 {
                    assert_eq!(g[j].to_bits(), w[j].to_bits(), "{tag}: out{k}[{j}]");
                }
            }
        };

    let mut total = 0u32;
    let cases: &[[f32; 3]] = &[
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],        // |xy|²=0 < eps → out1=(1,0,0)
        [1.0e-4, 1.0e-4, 10.0], // tiny xy, big z → degenerate-xy branch
        [3.0, 4.0, 0.0],
        [1.0, 2.0, 2.0],
        [-5.0, 3.0, -2.0],
        [0.577, 0.577, 0.577],
        [10.0, -7.0, 1.0],
    ];
    for &inp in cases {
        cmp("det", run(&mut emu, inp), orthonormal_basis_ref(inp));
        total += 1;
    }
    let mut s = 0x50a8_4000u32;
    for _ in 0..6000 {
        let inp = [
            rng(&mut s, -50.0, 50.0),
            rng(&mut s, -50.0, 50.0),
            rng(&mut s, -50.0, 50.0),
        ];
        if inp[0].abs() < 1.0 && inp[1].abs() < 1.0 && inp[2].abs() < 1.0 {
            continue; // keep |in|² ≥ eps (guard1)
        }
        cmp("fuzz", run(&mut emu, inp), orthonormal_basis_ref(inp));
        total += 1;
    }
    eprintln!("glue_509_50a840_orthonormal_basis: {total} bit-exact (whole-fn: guards, perp-normalize, cross)");
}

#[inline]
fn orthonormal_basis_ref(inp: [f32; 3]) -> ([f32; 3], [f32; 3], [f32; 3]) {
    benilla_re_ui::view_matrices::orthonormal_basis(inp)
}

// ===========================================================================================
// 0x50ab70 — proj/view setup. Owned-arith SLICE 0x50ab79..0x50ac25: aspect = (b−t)/(r−l) stays
// live on st0 through the integer identity-matrix fill, then `fst[esi+0x44]`/`fstp[ebp-0x8]`.
// We run the slice (the gxumath/gx/vtable calls are all PAST the halt) and read [ebp-0x8].
// ==threshold probe: exact integer ratios (the divide pivot); denominators kept nonzero.
// ===========================================================================================
#[test]
fn glue_50a_50ab70_proj_aspect() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let fr = emu.alloc(0x200);
    let ebp = fr + 0x130; // [ebp-0x94] writes below, [ebp+8] arg above
    let stack = emu.alloc(0x400);
    let esp = stack + 0x300;
    let dev = emu.alloc(0x80); // esi (this): [esi+0x44] store target + field reads
    let rect = emu.alloc(0x10); // eax: rect (l,t,r,b)

    let run = |emu: &mut Emu, l: f32, t: f32, r: f32, b: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::ECX, dev); // mov esi,ecx
        emu.write_u32(rect, l.to_bits());
        emu.write_u32(rect + 4, t.to_bits());
        emu.write_u32(rect + 8, r.to_bits());
        emu.write_u32(rect + 0xc, b.to_bits());
        emu.write_u32(ebp + 8, rect as u32); // mov eax,[ebp+8]
        emu.run(0x50ab79, 0x50ac25); // aspect compute + identity-fill + fst[esi+0x44]/fstp[ebp-0x8]
        emu.read_f32(ebp - 0x8)
    };

    let mut total = 0u32;
    // (l, t, r, b) — aspect = (b−t)/(r−l). Denominator r−l kept nonzero.
    let cases: &[(f32, f32, f32, f32)] = &[
        (0.0, 0.0, 1.0, 1.0),      // 1/1 = 1
        (0.0, 0.0, 2.0, 1.0),      // 1/2
        (0.0, 0.0, 1.0, 2.0),      // 2/1 = 2 (exact pivot)
        (0.0, 0.0, 4.0, 3.0),      // 3/4
        (-1.0, -1.0, 1.0, 1.0),    // 2/2 = 1
        (10.0, 20.0, 110.0, 95.0), // 75/100
        (0.0, 0.0, 1024.0, 768.0), // 0.75
        (5.0, -3.0, 5.5, 7.0),     // 10/0.5 = 20
        (0.0, 0.0, -2.0, 1.0),     // 1/-2 (negative denom)
        (0.0, 0.0, 3.0, -6.0),     // -6/3 = -2
    ];
    for &(l, t, r, b) in cases {
        assert_eq!(
            run(&mut emu, l, t, r, b).to_bits(),
            proj_aspect(l, t, r, b).to_bits(),
            "aspect l={l} t={t} r={r} b={b}"
        );
        total += 1;
    }
    let mut s = 0x50ab_7000u32;
    for _ in 0..5000 {
        let l = rng(&mut s, -1000.0, 1000.0);
        let t = rng(&mut s, -1000.0, 1000.0);
        let r = l + rng(&mut s, 1.0, 2000.0); // keep r−l ≥ 1 (nonzero denom)
        let b = rng(&mut s, -1000.0, 1000.0);
        assert_eq!(
            run(&mut emu, l, t, r, b).to_bits(),
            proj_aspect(l, t, r, b).to_bits(),
            "aspect fuzz l={l} t={t} r={r} b={b}"
        );
        total += 1;
    }
    eprintln!("glue_50a_50ab70_proj_aspect: {total} bit-exact (aspect (b−t)/(r−l) owned slice)");
}

fn proj_aspect(l: f32, t: f32, r: f32, b: f32) -> f32 {
    benilla_re_ui::view_matrices::proj_aspect(l, t, r, b)
}
