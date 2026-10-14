//! The curve evaluators of `0x453300` to `0x453960` run whole on a synthesized curve object
//! written into scratch memory (the scale at `+4`, the control points at `+0x10`, the segment
//! lengths at `+0x20`): the basis weight `0x453620`, the cubic and quadratic evaluators `0x453580`
//! and `0x453640`, the arc length `0x453760`, the segment locator `0x453840`, the quantizer
//! `0x4538c0` and the knot sums `0x453300` and `0x453920`. The coefficients and control points are
//! random bit patterns and the Catmull-Rom rows, so the order of the arithmetic is what is
//! compared; the constants the code reads (`0x7ffd74`, `0x7ff9d8`, `0x8029c8`, `0x8029cc`) are read
//! from the exe and passed to the transcription. A NaN result matches any NaN.
#![allow(clippy::float_cmp)]

mod common;

use benilla_re_spline::eval::*;
use common::*;

// The constants the code reads from the image.
const C_ZERO: u64 = 0x007f_fd74; // 0.0
const C_ONE: u64 = 0x007f_f9d8; // 1.0
const C_STEP: u64 = 0x0080_29c8; // 0.05 (arc-length t-step)
const C_BIAS: u64 = 0x0080_29cc; // 512.0 (fixed-point rounding bias)

/// Reads an `f32` constant from the loaded image, to pass to the transcription.
fn rconst(emu: &Emu, va: u64) -> f32 {
    emu.read_f32(va)
}

// ---- shared scratch layout for a curve object ----
// this+0x04 = scale (f32, seg_locate); this+0x10 = control-point array ptr (C3Vector[], stride 0xc);
// this+0x20 = per-segment knot/length array ptr (f32[], stride 4).
struct Curve {
    this: u64,
    cps: u64,   // control-point array base
    coeff: u64, // basis-matrix base
    knots: u64, // segment-length array base
    out: u64,   // C3Vector out buffer
    idx: u64,   // u32 out
    frac: u64,  // f32 out
}

impl Curve {
    fn new(emu: &mut Emu) -> Curve {
        let this = emu.alloc(0x40);
        let cps = emu.alloc(0xc * 4 * 8); // up to 8 segments' worth of 4 control points
        let coeff = emu.alloc(0x80); // 4 rows, generous
        let knots = emu.alloc(0x100); // up to 64 segment lengths
        let out = emu.alloc(0x10);
        let idx = emu.alloc(0x10);
        let frac = emu.alloc(0x10);
        emu.write_u32(this + 0x10, cps as u32);
        emu.write_u32(this + 0x20, knots as u32);
        Curve {
            this,
            cps,
            coeff,
            knots,
            out,
            idx,
            frac,
        }
    }
    fn set_scale(&self, emu: &mut Emu, scale: f32) {
        emu.write_u32(self.this + 0x4, scale.to_bits());
    }
    /// Write a flat coeff matrix (caller decides the per-row stride in f32s).
    fn set_coeff(&self, emu: &mut Emu, coeff: &[f32]) {
        for (i, &c) in coeff.iter().enumerate() {
            emu.write_u32(self.coeff + i as u64 * 4, c.to_bits());
        }
    }
    /// Write the four control points the evaluator reads for `seg`: `P[seg + i]`, at byte offset
    /// `seg·0xc`, stride 0xc.
    fn set_cps(&self, emu: &mut Emu, seg: u32, cps: &[[f32; 3]; 4]) {
        let base = self.cps + seg as u64 * 0xc;
        for (i, p) in cps.iter().enumerate() {
            for (j, &c) in p.iter().enumerate() {
                emu.write_u32(base + i as u64 * 0xc + j as u64 * 4, c.to_bits());
            }
        }
    }
    fn set_knots(&self, emu: &mut Emu, knots: &[f32]) {
        for (i, &k) in knots.iter().enumerate() {
            emu.write_u32(self.knots + i as u64 * 4, k.to_bits());
        }
    }
    fn read_out(&self, emu: &Emu) -> [f32; 3] {
        [
            emu.read_f32(self.out),
            emu.read_f32(self.out + 4),
            emu.read_f32(self.out + 8),
        ]
    }
}

/// `0x453620` basis_weight (`__fastcall` ecx=degree, edx=coeff ptr, [ebp+8]=t → f64 in st0).
#[test]
fn basis_weight_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let coeff_va = emu.alloc(0x40);
    let run = |emu: &mut Emu, degree: u32, coeff: &[f32], t: f32| {
        for (i, &c) in coeff.iter().enumerate() {
            emu.write_u32(coeff_va + i as u64 * 4, c.to_bits());
        }
        let got = emu.call_f64(0x0045_3620, degree, coeff_va as u32, &[t.to_bits()]);
        let want = basis_weight(degree, coeff, t);
        eqf64(
            got,
            want,
            &format!("basis_weight(deg={degree},{coeff:?},{t})"),
        );
    };
    // boundary probes: t==0 (only coeff[deg] survives), t==1 (sum of coeffs), degree 0 (just coeff[0]).
    run(&mut emu, 3, &[-0.5, 1.5, -1.5, 0.5], 0.0);
    run(&mut emu, 3, &[-0.5, 1.5, -1.5, 0.5], 1.0);
    run(&mut emu, 0, &[2.5, 9.9, 9.9, 9.9], 0.5);
    // Catmull-Rom-shaped rows over the edge set.
    for &t in EDGES {
        run(&mut emu, 3, &[-0.5, 1.5, -1.5, 0.5], t);
        run(&mut emu, 2, &[1.5, -2.5, 1.0, 0.0], t);
        run(&mut emu, 1, &[-1.0, 1.0, 0.0, 0.0], t);
    }
    let mut s = 0x1357_2468u32;
    for _ in 0..6000 {
        let coeff = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let degree = lcg(&mut s) % 4; // 0..=3
        run(&mut emu, degree, &coeff, fuzz_bounded(&mut s, 1.5));
    }
}

/// Drive an evaluator (`0x453580`/`0x453640`): `__thiscall` ecx=this; stack
/// `[seg, t, coeff_base, out_ptr]`; result written to `out_ptr` (C3Vector).
fn eval_diff(
    addr: u64,
    degree_label: &str,
    row_stride: usize,
    rust: impl Fn(&[f32], &[[f32; 3]; 4], f32) -> [f32; 3],
) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cv = Curve::new(&mut emu);
    let n_coeff = row_stride * 4 + 4; // 4 rows + slack
    let run = |emu: &mut Emu, seg: u32, coeff: &[f32], cps: &[[f32; 3]; 4], t: f32| {
        cv.set_coeff(emu, coeff);
        cv.set_cps(emu, seg, cps);
        emu.call(
            addr,
            cv.this as u32,
            0,
            &[seg, t.to_bits(), cv.coeff as u32, cv.out as u32],
        );
        let got = cv.read_out(emu);
        let want = rust(coeff, cps, t);
        for i in 0..3 {
            eqf32(
                got[i],
                want[i],
                &format!("eval[{degree_label}](seg={seg},t={t})[{i}] coeff={coeff:?} cps={cps:?}"),
            );
        }
    };
    let mk_coeff = |vals: &[f32]| {
        let mut c = vec![0.0f32; n_coeff];
        let k = vals.len().min(n_coeff);
        c[..k].copy_from_slice(&vals[..k]);
        c
    };
    // boundary probes: t==0, t==1, a degenerate (all-zero) control hull, seg!=0.
    let cr = mk_coeff(&[
        -0.5, 1.5, -1.5, 0.5, 1.0, -2.5, 2.0, -0.5, -0.5, 1.5, -1.5, 0.5, 0.0, 1.0, 0.0, 0.0,
    ]);
    let hull = [
        [1.0, 2.0, 3.0],
        [-4.0, 5.0, -6.0],
        [7.0, -8.0, 9.0],
        [0.5, 0.5, 0.5],
    ];
    run(&mut emu, 0, &cr, &hull, 0.0);
    run(&mut emu, 0, &cr, &hull, 1.0);
    run(&mut emu, 2, &cr, &hull, 0.5);
    run(&mut emu, 0, &cr, &[[0.0; 3]; 4], 0.5);
    for &t in EDGES {
        run(&mut emu, 0, &cr, &hull, t);
    }
    let mut s = 0x2468_1357u32;
    for _ in 0..5000 {
        let mut coeff = vec![0.0f32; n_coeff];
        for c in coeff.iter_mut() {
            *c = fuzz(&mut s);
        }
        let cps = [
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
        ];
        let seg = lcg(&mut s) % 4;
        run(&mut emu, seg, &coeff, &cps, fuzz_bounded(&mut s, 1.5));
    }
}

/// `0x453580` cubic (degree-3) point evaluator.
#[test]
fn eval_point_cubic_diff() {
    eval_diff(0x0045_3580, "cubic", 4, eval_point_cubic);
}

/// `0x453640` degree-2 evaluator.
#[test]
fn eval_point_quad_diff() {
    eval_diff(0x0045_3640, "quad", 4, eval_point_quad);
}

/// `0x453760` arc-length integrator (`__thiscall` ecx=this; stack `[seg, coeff_base]`; f64 in st0).
/// It calls `0x453580`; compared with `arc_length` sampling `eval_point_cubic`.
#[test]
fn arc_length_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cv = Curve::new(&mut emu);
    let step = rconst(&emu, C_STEP);
    let n_coeff = 4 * 4 + 4;
    let run = |emu: &mut Emu, seg: u32, coeff: &[f32], cps: &[[f32; 3]; 4]| {
        cv.set_coeff(emu, coeff);
        cv.set_cps(emu, seg, cps);
        let got = emu.call_f64(0x0045_3760, cv.this as u32, 0, &[seg, cv.coeff as u32]);
        let want = arc_length(step, |t| eval_point_cubic(coeff, cps, t));
        eqf64(
            got,
            want,
            &format!("arc_length(seg={seg}) coeff={coeff:?} cps={cps:?}"),
        );
    };
    let mut cr = vec![0.0f32; n_coeff];
    cr[..16].copy_from_slice(&[
        -0.5, 1.5, -1.5, 0.5, 1.0, -2.5, 2.0, -0.5, -0.5, 1.5, -1.5, 0.5, 0.0, 1.0, 0.0, 0.0,
    ]);
    // boundary: degenerate hull (zero length), a straight line, seg != 0.
    run(&mut emu, 0, &cr, &[[0.0; 3]; 4]);
    run(
        &mut emu,
        0,
        &cr,
        &[
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
        ],
    );
    run(
        &mut emu,
        1,
        &cr,
        &[
            [1.0, 2.0, 3.0],
            [-4.0, 5.0, -6.0],
            [7.0, -8.0, 9.0],
            [0.5, 0.5, 0.5],
        ],
    );
    let mut s = 0x9bdf_0246u32;
    for _ in 0..3000 {
        // bounded control points so the chord sum stays finite (real curves are tame; inf-inf NaN is still
        // accepted as any-NaN by eqf64, but a tame hull exercises the f32-narrowing accumulator meaningfully).
        let cps = [
            [
                fuzz_bounded(&mut s, 50.0),
                fuzz_bounded(&mut s, 50.0),
                fuzz_bounded(&mut s, 50.0),
            ],
            [
                fuzz_bounded(&mut s, 50.0),
                fuzz_bounded(&mut s, 50.0),
                fuzz_bounded(&mut s, 50.0),
            ],
            [
                fuzz_bounded(&mut s, 50.0),
                fuzz_bounded(&mut s, 50.0),
                fuzz_bounded(&mut s, 50.0),
            ],
            [
                fuzz_bounded(&mut s, 50.0),
                fuzz_bounded(&mut s, 50.0),
                fuzz_bounded(&mut s, 50.0),
            ],
        ];
        let mut coeff = vec![0.0f32; n_coeff];
        for c in coeff.iter_mut() {
            *c = fuzz_bounded(&mut s, 2.0);
        }
        run(&mut emu, lcg(&mut s) % 4, &coeff, &cps);
    }
}

/// `0x453840` segment locator (`__thiscall` ecx=this; stack `[t, arg2(unused), n_seg, outIdx, outFrac]`).
#[test]
fn seg_locate_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cv = Curve::new(&mut emu);
    let run = |emu: &mut Emu, scale: f32, knots: &[f32], t: f32, n_seg: u32| {
        cv.set_scale(emu, scale);
        cv.set_knots(emu, knots);
        emu.call(
            0x0045_3840,
            cv.this as u32,
            0,
            &[t.to_bits(), 0, n_seg, cv.idx as u32, cv.frac as u32],
        );
        let got_idx = emu.read_u32(cv.idx);
        let got_frac = emu.read_f32(cv.frac);
        let (want_idx, want_frac) = seg_locate(scale, knots, t, n_seg);
        eqi(
            got_idx,
            want_idx,
            &format!("seg_locate.idx(scale={scale},t={t},n={n_seg}) knots={knots:?}"),
        );
        eqf32(
            got_frac,
            want_frac,
            &format!("seg_locate.frac(scale={scale},t={t},n={n_seg}) knots={knots:?}"),
        );
    };
    let knots = [2.0, 3.0, 1.5, 4.0, 0.5, 2.5, 1.0, 3.5];
    // boundary probes: n_seg==1 (idx=0,frac=t), t==0 (s=0 → seg 0), t==1, a segment-boundary t.
    run(&mut emu, 10.0, &knots, 0.5, 1); // n_seg==1 short path
    run(&mut emu, 10.0, &knots, 0.0, 4); // t=0
    run(&mut emu, 10.0, &knots, 1.0, 4); // t=1
    run(&mut emu, 10.0, &knots, 0.2, 4); // s = 2.0 exactly = first knot boundary
    for &t in &[-0.5f32, 0.0, 0.1, 0.25, 0.5, 0.75, 0.99, 1.0, 1.5] {
        for &n in &[1u32, 2, 4, 8] {
            run(&mut emu, 10.0, &knots, t, n);
        }
    }
    let mut s = 0x0f1e_2d3cu32;
    for _ in 0..4000 {
        // positive knot lengths (segment lengths are |Δ|; a real knot table is positive) + a tame scale/t.
        let knots: Vec<f32> = (0..8)
            .map(|_| fuzz_bounded(&mut s, 5.0).abs() + 0.01)
            .collect();
        let scale = fuzz_bounded(&mut s, 20.0);
        let n_seg = 1 + lcg(&mut s) % 8; // 1..=8
        run(&mut emu, scale, &knots, fuzz_bounded(&mut s, 1.5), n_seg);
    }
}

/// `0x4538c0` fixed-point quantize (`__cdecl`; stack `[t, n, outIdx, outFrac]`).
#[test]
fn fixed_quantize_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let bias = rconst(&emu, C_BIAS);
    let one = rconst(&emu, C_ONE);
    let p_idx = emu.alloc(0x10);
    let p_frac = emu.alloc(0x10);
    let run = |emu: &mut Emu, t: f32, n: u32| {
        emu.call(
            0x0045_38c0,
            0,
            0,
            &[t.to_bits(), n, p_idx as u32, p_frac as u32],
        );
        let got_idx = emu.read_u32(p_idx);
        let got_frac = emu.read_f32(p_frac);
        let (want_idx, want_frac) = fixed_quantize(bias, one, t, n);
        eqi(
            got_idx,
            want_idx,
            &format!("fixed_quantize.idx(t={t},n={n})"),
        );
        eqf32(
            got_frac,
            want_frac,
            &format!("fixed_quantize.frac(t={t},n={n})"),
        );
    };
    // boundary probes: t==0, t==1, exact 1/N grid points (frac→0), n at the 0..255 idx ceiling.
    run(&mut emu, 0.0, 8);
    run(&mut emu, 1.0, 8);
    run(&mut emu, 0.125, 8); // t*8 = 1 exactly → idx=1, frac=0
    run(&mut emu, 0.5, 4);
    for &t in &[0.0f32, 0.05, 0.1, 0.25, 0.333_333, 0.5, 0.75, 0.999, 1.0] {
        for &n in &[1u32, 2, 3, 4, 8, 16, 100, 255] {
            run(&mut emu, t, n);
        }
    }
    let mut s = 0x55aa_33ccu32;
    for _ in 0..4000 {
        let n = 1 + lcg(&mut s) % 300;
        run(&mut emu, fuzz_bounded(&mut s, 1.5), n);
    }
}

/// `0x453300` knot-sum over `n-1` entries (`__thiscall` ecx=this; stack `[n]`; f64 in st0).
#[test]
fn knot_sum_nm1_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cv = Curve::new(&mut emu);
    let zero = rconst(&emu, C_ZERO);
    let run = |emu: &mut Emu, knots: &[f32], n: u32| {
        cv.set_knots(emu, knots);
        let got = emu.call_f64(0x0045_3300, cv.this as u32, 0, &[n]);
        let want = knot_sum_nm1(zero, knots, n);
        eqf64(got, want, &format!("knot_sum_nm1(n={n}) knots={knots:?}"));
    };
    let knots = [2.0, 3.0, 1.5, 4.0, 0.5, 2.5, 1.0, 3.5, 9.0, -1.0, 0.25, 6.0];
    // boundary probes: n==0 (returns 0.0), n==1 (sums 0 entries → 0.0), n==len.
    run(&mut emu, &knots, 0);
    run(&mut emu, &knots, 1);
    run(&mut emu, &knots, knots.len() as u32);
    for n in 0..=knots.len() as u32 {
        run(&mut emu, &knots, n);
    }
    let mut s = 0x7e1d_4c0bu32;
    for _ in 0..3000 {
        let len = 1 + (lcg(&mut s) % 12) as usize;
        let knots: Vec<f32> = (0..len + 1).map(|_| fuzz(&mut s)).collect();
        run(&mut emu, &knots, lcg(&mut s) % (len as u32 + 1));
    }
}

/// `0x453920` knot-sum over `n` entries (`__thiscall` ecx=this; stack `[n]`; f64 in st0).
#[test]
fn knot_sum_n_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cv = Curve::new(&mut emu);
    let zero = rconst(&emu, C_ZERO);
    let run = |emu: &mut Emu, knots: &[f32], n: u32| {
        cv.set_knots(emu, knots);
        let got = emu.call_f64(0x0045_3920, cv.this as u32, 0, &[n]);
        let want = knot_sum_n(zero, knots, n);
        eqf64(got, want, &format!("knot_sum_n(n={n}) knots={knots:?}"));
    };
    let knots = [2.0, 3.0, 1.5, 4.0, 0.5, 2.5, 1.0, 3.5, 9.0, -1.0, 0.25, 6.0];
    // boundary probes: n==0 (returns 0.0), n==len.
    run(&mut emu, &knots, 0);
    run(&mut emu, &knots, 1);
    run(&mut emu, &knots, knots.len() as u32);
    for n in 0..=knots.len() as u32 {
        run(&mut emu, &knots, n);
    }
    let mut s = 0x3b2a_1908u32;
    for _ in 0..3000 {
        let len = 1 + (lcg(&mut s) % 12) as usize;
        let knots: Vec<f32> = (0..len).map(|_| fuzz(&mut s)).collect();
        run(&mut emu, &knots, lcg(&mut s) % (len as u32 + 1));
    }
}
