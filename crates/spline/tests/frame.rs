//! The curve object's segment functions run whole on a synthesized object (the scale at `+4`, the
//! count at `+0xc`, the control points at `+0x10`, the segment lengths at `+0x20`, the mode at
//! `+0x28`): the position `0x4541b0` and the length `0x454320` in both modes, the frame
//! `0x453b90`, and the frame `0x454580` in both modes. The basis matrices at `0xb05e58`, `0xb05e10`
//! and `0xb05dd8` are filled by running their initializers `0x453960`, `0x453fa0` and `0x454080`;
//! the derivative basis at `0xb05d80` is written from the constants `0x453cd0` stores, with its
//! guard byte `0xb05e08` set so the one-time path, which registers an `atexit` handler on CRT
//! state the emulator does not hold, is skipped.
//!
//! The two frame builders compare their tangent, normal and binormal with the transcription of
//! the arithmetic after the kernels; the kernels' results (`0x453cd0`'s derivative for `0x453b90`,
//! `seg_locate` and the evaluators for `0x454580`) are taken from the exe or from the functions
//! the evaluator tests run. A spline-mode derivative shorter than 0.01 leaves the tangent
//! unwritten and is not compared. A NaN result matches any NaN.
#![allow(clippy::float_cmp)]
#![allow(clippy::too_many_arguments)]

mod common;

use benilla_re_spline::eval;
use benilla_re_spline::frame::*;
use common::*;

// The constants the code reads from the image.
const C_ONE_F32: u64 = 0x007f_f9d8; // 1.0f
const C_EPS_FRENET: u64 = 0x0080_29d0; // 0.01f  (0x453b90 length-degeneracy eps)
const C_EPS_FRAME: u64 = 0x0080_29d4; // 2.384e-7f (0x454580 length-degeneracy eps)

fn rconst(emu: &Emu, va: u64) -> f32 {
    emu.read_f32(va)
}

// -------------------------------------------------------------------------------------------------
// The curve object in scratch memory, for the position and length functions and the frame builders.
// this+0x04 = scale (f32); this+0x0c = count (the control-point count);
// this+0x10 = control-point array ptr (C3Vector[], stride 0xc); this+0x20 = knot/length array ptr;
// this+0x28 = mode flag (0 = linear, !=0 = spline).
// -------------------------------------------------------------------------------------------------
struct Curve {
    this: u64,
    cps: u64,
    knots: u64,
    out: u64, // C3Vector out (and 0x40-byte frame scratch for the builders)
}

impl Curve {
    fn new(emu: &mut Emu) -> Curve {
        let this = emu.alloc(0x40);
        let cps = emu.alloc(0xc * 64); // up to 64 control points
        let knots = emu.alloc(0x100);
        let out = emu.alloc(0x80); // frame builders write T@0, N@0x10, B@0x20, scratch@0x30
        emu.write_u32(this + 0x10, cps as u32);
        emu.write_u32(this + 0x20, knots as u32);
        Curve {
            this,
            cps,
            knots,
            out,
        }
    }
    fn set_mode(&self, emu: &mut Emu, mode: u32) {
        emu.write_u32(self.this + 0x28, mode);
    }
    fn set_count(&self, emu: &mut Emu, count: u32) {
        emu.write_u32(self.this + 0xc, count);
    }
    fn set_scale(&self, emu: &mut Emu, scale: f32) {
        emu.write_u32(self.this + 0x4, scale.to_bits());
    }
    /// Write a control point at element index `i` (float-index 3*i in the array).
    fn set_cp(&self, emu: &mut Emu, i: u32, p: [f32; 3]) {
        let base = self.cps + i as u64 * 0xc;
        for (j, &c) in p.iter().enumerate() {
            emu.write_u32(base + j as u64 * 4, c.to_bits());
        }
    }
    fn set_knots(&self, emu: &mut Emu, knots: &[f32]) {
        for (i, &k) in knots.iter().enumerate() {
            emu.write_u32(self.knots + i as u64 * 4, k.to_bits());
        }
    }
    fn read_vec(&self, emu: &Emu, off: u64) -> [f32; 3] {
        [
            emu.read_f32(self.out + off),
            emu.read_f32(self.out + off + 4),
            emu.read_f32(self.out + off + 8),
        ]
    }
}

/// Fills the basis matrices the spline mode reads, by running their initializers, and the guarded
/// derivative matrix `0xb05d80` that `0x453cd0` fills on first use: it is written directly and its
/// guard byte `0xb05e08` set, so `0x453cd0` and the frame builders that call it skip the one-time
/// path, which registers an `atexit` handler through `0x409aef` on CRT state the emulator does not
/// hold. The values are the `mov [0xb05d80+..], imm` constants of `0x453cd0`'s initializer.
fn init_basis(emu: &mut Emu) {
    emu.call(0x0045_3960, 0, 0, &[]); // 0xb05e58 (uniform cubic)
    emu.call(0x0045_3fa0, 0, 0, &[]); // 0xb05e10 (Catmull-Rom)
    emu.call(0x0045_4080, 0, 0, &[]); // 0xb05dd8 (cubic derivative)
                                      // 0xb05d80 (the guarded derivative basis): 12 floats from 0x453cd0's init body.
    const D80: [u32; 12] = [
        0xc040_0000,
        0x40c0_0000,
        0xc040_0000,
        0x4110_0000,
        0xc140_0000,
        0x4040_0000,
        0xc110_0000,
        0x40c0_0000,
        0x0000_0000,
        0x4040_0000,
        0x0000_0000,
        0x0000_0000,
    ];
    for (i, &v) in D80.iter().enumerate() {
        emu.write_u32(0x000b_05d80 + i as u64 * 4, v);
    }
    emu.write_u32(0x000b_05e08, 1); // set the one-time-init guard → skip the atexit path
}

// =================================================================================================
// 0x4541b0 — the position. `__thiscall` ecx=this; [seg, t, out_ptr]. `ret 0xc`.
// Linear mode (0): out = lerp(P[seg+1], P[seg+2], t). Spline mode: `eval_point_cubic` with the
// basis at 0xb05e10, which eval.rs runs against the exe; here the whole function is compared with
// it over the curve object.
// =================================================================================================
#[test]
fn linear_pos_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    init_basis(&mut emu);
    let cv = Curve::new(&mut emu);
    cv.set_count(&mut emu, 64);

    // --- linear branch (mode==0): direct lerp of P[seg+1], P[seg+2]. ---
    cv.set_mode(&mut emu, 0);
    let run_lin = |emu: &mut Emu, seg: u32, pa: [f32; 3], pb: [f32; 3], t: f32| {
        cv.set_cp(emu, seg + 1, pa);
        cv.set_cp(emu, seg + 2, pb);
        emu.call(
            0x0045_41b0,
            cv.this as u32,
            0,
            &[seg, t.to_bits(), cv.out as u32],
        );
        let got = cv.read_vec(emu, 0);
        let want = linear_pos(pa, pb, t);
        for i in 0..3 {
            eqf32(
                got[i],
                want[i],
                &format!("linear_pos(seg={seg},t={t},pa={pa:?},pb={pb:?})[{i}]"),
            );
        }
    };
    // boundary probes: t==0 (→ Pa), t==1 (→ Pb), seg!=0, degenerate equal points.
    run_lin(&mut emu, 0, [1.0, 2.0, 3.0], [-4.0, 5.0, -6.0], 0.0);
    run_lin(&mut emu, 0, [1.0, 2.0, 3.0], [-4.0, 5.0, -6.0], 1.0);
    run_lin(&mut emu, 3, [7.0, -8.0, 9.0], [0.5, 0.5, 0.5], 0.5);
    run_lin(&mut emu, 1, [1.0, 1.0, 1.0], [1.0, 1.0, 1.0], 0.25);
    for &t in EDGES {
        run_lin(&mut emu, 2, [1.5, -2.5, 3.5], [-4.5, 5.5, -6.5], t);
    }
    let mut s = 0x4541_b000u32;
    for _ in 0..6000 {
        let pa = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let pb = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let seg = lcg(&mut s) % 10;
        run_lin(&mut emu, seg, pa, pb, fuzz_bounded(&mut s, 2.0));
    }

    // --- spline branch (mode!=0): delegates to eval_point_cubic with basis 0xb05e10. ---
    cv.set_mode(&mut emu, 1);
    let run_spl = |emu: &mut Emu, seg: u32, cps4: [[f32; 3]; 4], t: f32| {
        // eval_point_cubic reads P[seg + i] for i = 0..3.
        for (i, &p) in cps4.iter().enumerate() {
            cv.set_cp(emu, seg + i as u32, p);
        }
        emu.call(
            0x0045_41b0,
            cv.this as u32,
            0,
            &[seg, t.to_bits(), cv.out as u32],
        );
        let got = cv.read_vec(emu, 0);
        // basis matrix 0xb05e10 read from the loaded+initialized image.
        let mut coeff = [0.0f32; 16];
        for (i, c) in coeff.iter_mut().enumerate() {
            *c = emu.read_f32(0x000b_05e10 + i as u64 * 4);
        }
        let want = eval::eval_point_cubic(&coeff, &cps4, t);
        for i in 0..3 {
            eqf32(
                got[i],
                want[i],
                &format!("spline_pos(seg={seg},t={t})[{i}]"),
            );
        }
    };
    let hull = [
        [1.0, 2.0, 3.0],
        [-4.0, 5.0, -6.0],
        [7.0, -8.0, 9.0],
        [0.5, 0.5, 0.5],
    ];
    run_spl(&mut emu, 0, hull, 0.0);
    run_spl(&mut emu, 0, hull, 1.0);
    run_spl(&mut emu, 2, hull, 0.5);
    let mut s = 0x4541_b111u32;
    for _ in 0..3000 {
        let cps4 = [
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)],
        ];
        run_spl(&mut emu, lcg(&mut s) % 8, cps4, fuzz_bounded(&mut s, 1.5));
    }
}

// =================================================================================================
// 0x454320 — the segment length. `__thiscall` ecx=this; [seg]. `ret 0x4`. Returns st0 (f64).
// Linear mode (0): ‖P[seg+2] − P[seg+1]‖. Spline mode: `arc_length`.
// =================================================================================================
#[test]
fn seg_length_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    init_basis(&mut emu);
    let cv = Curve::new(&mut emu);
    cv.set_count(&mut emu, 64);

    // --- linear branch ---
    cv.set_mode(&mut emu, 0);
    let run_lin = |emu: &mut Emu, seg: u32, pa: [f32; 3], pb: [f32; 3]| {
        cv.set_cp(emu, seg + 1, pa);
        cv.set_cp(emu, seg + 2, pb);
        let got = emu.call_f64(0x0045_4320, cv.this as u32, 0, &[seg]);
        let want = seg_length_linear(pa, pb);
        eqf64(
            got,
            want,
            &format!("seg_length(seg={seg},pa={pa:?},pb={pb:?})"),
        );
    };
    run_lin(&mut emu, 0, [0.0, 0.0, 0.0], [3.0, 4.0, 0.0]); // length 5
    run_lin(&mut emu, 0, [1.0, 1.0, 1.0], [1.0, 1.0, 1.0]); // length 0
    run_lin(&mut emu, 4, [7.0, -8.0, 9.0], [0.5, 0.5, 0.5]);
    let mut s = 0x4543_2000u32;
    for _ in 0..6000 {
        // Bounded so the sum of squares stays inside the f64 exponent range; the x87 register's
        // wider exponent would not overflow where an f64 does.
        let pa = [
            fuzz_bounded(&mut s, 1e3),
            fuzz_bounded(&mut s, 1e3),
            fuzz_bounded(&mut s, 1e3),
        ];
        let pb = [
            fuzz_bounded(&mut s, 1e3),
            fuzz_bounded(&mut s, 1e3),
            fuzz_bounded(&mut s, 1e3),
        ];
        run_lin(&mut emu, lcg(&mut s) % 10, pa, pb);
    }

    // --- spline branch (mode!=0): delegates to arc_length (Catmull-Rom, basis 0xb05e10). ---
    cv.set_mode(&mut emu, 1);
    let step = rconst(&emu, 0x0080_29c8); // 0.05 (arc-length t-step)
    let run_spl = |emu: &mut Emu, seg: u32, cps4: [[f32; 3]; 4]| {
        for (i, &p) in cps4.iter().enumerate() {
            cv.set_cp(emu, seg + i as u32, p);
        }
        let got = emu.call_f64(0x0045_4320, cv.this as u32, 0, &[seg]);
        let mut coeff = [0.0f32; 16];
        for (i, c) in coeff.iter_mut().enumerate() {
            *c = emu.read_f32(0x000b_05e10 + i as u64 * 4);
        }
        let want = eval::arc_length(step, |t| eval::eval_point_cubic(&coeff, &cps4, t));
        eqf64(got, want, &format!("spline_arclen(seg={seg})"));
    };
    run_spl(
        &mut emu,
        0,
        [
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
        ],
    );
    let mut s = 0x4543_2111u32;
    for _ in 0..2000 {
        let cps4 = [
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
        run_spl(&mut emu, lcg(&mut s) % 8, cps4);
    }
}

// =================================================================================================
// 0x453b90 — the frame. `__thiscall` ecx=this; [global_t, frame_ptr]. `ret 0x8`.
// The function takes a derivative from 0x453cd0 (basis 0xb05d80) and builds T = normalize(der),
// N = (-T.y, T.x, 0) normalized and B = T×N. The test runs the whole function and compares T, N and
// B with `frenet_frame_from_derivative` of the derivative the exe's own 0x453cd0 returns.
// =================================================================================================
#[test]
fn frenet_frame_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    init_basis(&mut emu);
    let cv = Curve::new(&mut emu);
    let eps = rconst(&emu, C_EPS_FRENET);
    let one = rconst(&emu, C_ONE_F32);

    // Derivative-eval scratch: a 3-float buffer 0x453cd0 writes the tangent into.
    let der_out = emu.alloc(0x10);

    let run = |emu: &mut Emu, count: u32, scale: f32, knots: &[f32], global_t: f32| {
        cv.set_mode(emu, 1); // spline mode (the builder's normal mode)
        cv.set_count(emu, count);
        cv.set_scale(emu, scale);
        cv.set_knots(emu, knots);
        // 1. Locate the segment as 0x453a60 does: n_seg = count / 3.
        let n_seg = count / 3;
        let (seg, frac) = eval::seg_locate(scale, knots, global_t, n_seg);
        // 2. The exe's own derivative kernel at (seg, frac): 0x453cd0(this; seg, frac, &der_out).
        emu.write_u32(der_out, 0);
        emu.write_u32(der_out + 4, 0);
        emu.write_u32(der_out + 8, 0);
        emu.call(
            0x0045_3cd0,
            cv.this as u32,
            0,
            &[seg, frac.to_bits(), der_out as u32],
        );
        let der = [
            emu.read_f32(der_out),
            emu.read_f32(der_out + 4),
            emu.read_f32(der_out + 8),
        ];
        // 3. run the whole frame builder and read T/N/B.
        emu.call(
            0x0045_3b90,
            cv.this as u32,
            0,
            &[global_t.to_bits(), cv.out as u32],
        );
        let t = cv.read_vec(emu, 0);
        let n = cv.read_vec(emu, 0x10);
        let b = cv.read_vec(emu, 0x20);
        let want = frenet_frame_from_derivative(der, eps, one);
        let ctx = format!("frenet(count={count},t={global_t},seg={seg},frac={frac},der={der:?})");
        for i in 0..3 {
            eqf32(t[i], want[i], &format!("{ctx}.T[{i}]"));
        }
        for i in 0..3 {
            eqf32(n[i], want[4 + i], &format!("{ctx}.N[{i}]"));
        }
        for i in 0..3 {
            eqf32(b[i], want[8 + i], &format!("{ctx}.B[{i}]"));
        }
    };

    // A tame control-point hull (a gentle curve) so the derivative is non-degenerate; knots positive.
    let setup_hull = |emu: &mut Emu, s: &mut u32, n: u32| {
        for i in 0..n {
            cv.set_cp(
                emu,
                i,
                [
                    fuzz_bounded(s, 10.0),
                    fuzz_bounded(s, 10.0),
                    fuzz_bounded(s, 10.0),
                ],
            );
        }
    };
    let knots: Vec<f32> = (0..16).map(|i| 1.0 + (i as f32) * 0.3).collect();

    let mut s = 0x453b_9000u32;
    // boundary probes: t at the ends + a degenerate (all-equal) hull (zero derivative → eps path).
    for i in 0..12u32 {
        cv.set_cp(&mut emu, i, [1.0, 1.0, 1.0]); // degenerate → derivative ~0 → eps skip
    }
    run(&mut emu, 12, 10.0, &knots, 0.5);
    for &gt in &[0.0f32, 0.001, 0.25, 0.5, 0.75, 0.999, 1.0] {
        setup_hull(&mut emu, &mut s, 12);
        run(&mut emu, 12, 8.0, &knots, gt);
    }
    for _ in 0..3000 {
        let count = 6 + (lcg(&mut s) % 10) * 3; // multiple of 3, >=6
        setup_hull(&mut emu, &mut s, count);
        let scale = fuzz_bounded(&mut s, 20.0).abs() + 0.1;
        run(&mut emu, count, scale, &knots, fuzz_bounded(&mut s, 1.2));
    }
}

// =================================================================================================
// 0x454580 — the frame along the curve. `__thiscall` ecx=this; [global_t, frame_ptr]. `ret 0x8`.
// Linear mode (this+0x28 == 0): T = normalize(P[seg+2] − P[seg+1]), N the perpendicular in x and y,
// B = T×N, both normalizations guarded by eps2. The whole function is compared with
// frame_tangent_linear and frame_finish, taking seg from `seg_locate`.
// =================================================================================================
#[test]
fn frame_builder_linear_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    init_basis(&mut emu);
    let cv = Curve::new(&mut emu);
    let eps2 = rconst(&emu, C_EPS_FRAME);
    let one = rconst(&emu, C_ONE_F32);

    let run = |emu: &mut Emu, count: u32, scale: f32, knots: &[f32], cps: &[[f32; 3]], gt: f32| {
        cv.set_mode(emu, 0); // linear mode
        cv.set_count(emu, count);
        cv.set_scale(emu, scale);
        cv.set_knots(emu, knots);
        for (i, &p) in cps.iter().enumerate() {
            cv.set_cp(emu, i as u32, p);
        }
        // 0x454580 uses n_seg = count - 3 (the 0x454380 wrapper subtracts 3).
        let n_seg = count.wrapping_sub(3);
        let (seg, _frac) = eval::seg_locate(scale, knots, gt, n_seg);
        // run the whole builder.
        emu.call(
            0x0045_4580,
            cv.this as u32,
            0,
            &[gt.to_bits(), cv.out as u32],
        );
        let t = cv.read_vec(emu, 0);
        let n = cv.read_vec(emu, 0x10);
        let b = cv.read_vec(emu, 0x20);
        // expected: tangent = normalize(P[seg+1], P[seg+2]); then N/B finisher.
        let pa = cps[(seg + 1) as usize];
        let pb = cps[(seg + 2) as usize];
        let tan = frame_tangent_linear(pa, pb, eps2, one);
        let want = frame_finish(tan, eps2, one);
        let ctx = format!("frame_lin(count={count},t={gt},seg={seg},pa={pa:?},pb={pb:?})");
        for i in 0..3 {
            eqf32(t[i], want[i], &format!("{ctx}.T[{i}]"));
            eqf32(n[i], want[4 + i], &format!("{ctx}.N[{i}]"));
            eqf32(b[i], want[8 + i], &format!("{ctx}.B[{i}]"));
        }
    };

    let knots: Vec<f32> = (0..32).map(|i| 1.0 + (i as f32) * 0.2).collect();
    let mut s = 0x4545_8000u32;
    let mk_cps = |s: &mut u32, n: usize| -> Vec<[f32; 3]> {
        (0..n)
            .map(|_| {
                [
                    fuzz_bounded(s, 20.0),
                    fuzz_bounded(s, 20.0),
                    fuzz_bounded(s, 20.0),
                ]
            })
            .collect()
    };

    // boundary probes: t at ends; a degenerate adjacent pair (zero tangent → eps2 skip).
    {
        let mut cps = mk_cps(&mut s, 20);
        cps[2] = cps[1]; // P[seg+1]==P[seg+2] for some seg → zero diff
        for &gt in &[0.0f32, 0.001, 0.25, 0.5, 0.75, 0.999, 1.0] {
            run(&mut emu, 16, 10.0, &knots, &cps, gt);
        }
    }
    for _ in 0..4000 {
        let count = 8 + lcg(&mut s) % 16; // >= 8
        let cps = mk_cps(&mut s, (count + 4) as usize);
        let scale = fuzz_bounded(&mut s, 20.0).abs() + 0.1;
        run(
            &mut emu,
            count,
            scale,
            &knots,
            &cps,
            fuzz_bounded(&mut s, 1.2),
        );
    }
}

/// `0x454580` in spline mode (`this+0x28 != 0`), the branch the linear test does not run. The
/// function locates the segment (`0x454380`), forms the linear tangent, takes the derivative tangent
/// (`0x454250`, the quadratic evaluator over the derivative basis `0xb05dd8`), normalizes it and
/// picks between the two by the dot product, then finishes the frame. The expected frame composes
/// `seg_locate`, `frame_tangent_linear`, `eval_point_quad` over the derivative basis,
/// `frame_spline_tangent` and `frame_finish`. A derivative shorter than 0.01 leaves the tangent
/// unwritten (`frame_spline_tangent` gives `None`) and the case is skipped.
#[test]
fn frame_builder_spline_diff() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    init_basis(&mut emu);
    let cv = Curve::new(&mut emu);
    let eps2_frame = rconst(&emu, C_EPS_FRAME); // 0x8029d4 (frame + linear-tangent eps, 2.384e-7)
    let eps2_deriv = rconst(&emu, C_EPS_FRENET); // 0x8029d0 = 0.01f (deriv-normalize eps)
    let one = rconst(&emu, C_ONE_F32);
    // 0x454250 lays the 12 values of the derivative basis 0xb05dd8 out on the stack as four rows of
    // [c0, c1, c2, pad]; the pad (0, 0, 0, then 1.0) is not read by the degree-2 weight. The
    // same layout is built here, since a plain read of 16 words has the wrong stride.
    let mut src = [0.0f32; 12];
    for (i, c) in src.iter_mut().enumerate() {
        *c = emu.read_f32(0x000b_05dd8 + i as u64 * 4);
    }
    let deriv_basis = [
        src[0], src[1], src[2], 0.0, src[3], src[4], src[5], 0.0, src[6], src[7], src[8], 0.0,
        src[9], src[10], src[11], 1.0,
    ];
    let run = |emu: &mut Emu, count: u32, scale: f32, knots: &[f32], cps: &[[f32; 3]], gt: f32| {
        let n_seg = count.wrapping_sub(3); // the 0x454380 wrapper subtracts 3
        let (seg, frac) = eval::seg_locate(scale, knots, gt, n_seg);
        let su = seg as usize;
        if su + 3 >= cps.len() {
            return; // out of the provided hull
        }
        // derivative tangent: eval_point_quad with the deriv basis, over P[seg+0..3] at the local frac.
        let cps4 = [cps[su], cps[su + 1], cps[su + 2], cps[su + 3]];
        let deriv_raw = eval::eval_point_quad(&deriv_basis, &cps4, frac);
        let lin = frame_tangent_linear(cps[su + 1], cps[su + 2], eps2_frame, one);
        let Some(tan) = frame_spline_tangent(deriv_raw, lin, eps2_deriv) else {
            return; // degenerate derivative → T unwritten by the binary; outside the diff domain
        };
        let want = frame_finish(tan, eps2_frame, one);
        cv.set_mode(emu, 1); // spline mode
        cv.set_count(emu, count);
        cv.set_scale(emu, scale);
        cv.set_knots(emu, knots);
        for (i, &p) in cps.iter().enumerate() {
            cv.set_cp(emu, i as u32, p);
        }
        emu.call(
            0x0045_4580,
            cv.this as u32,
            0,
            &[gt.to_bits(), cv.out as u32],
        );
        let (t, n, b) = (
            cv.read_vec(emu, 0),
            cv.read_vec(emu, 0x10),
            cv.read_vec(emu, 0x20),
        );
        let ctx = format!("frame_spline(count={count},t={gt},seg={seg},frac={frac})");
        for i in 0..3 {
            eqf32(t[i], want[i], &format!("{ctx}.T[{i}]"));
            eqf32(n[i], want[4 + i], &format!("{ctx}.N[{i}]"));
            eqf32(b[i], want[8 + i], &format!("{ctx}.B[{i}]"));
        }
    };
    let knots: Vec<f32> = (0..40).map(|i| 1.0 + (i as f32) * 0.2).collect();
    let mut s = 0x4545_8222u32;
    for _ in 0..6000 {
        let count = 8 + lcg(&mut s) % 16;
        let cps: Vec<[f32; 3]> = (0..(count + 4))
            .map(|_| {
                [
                    fuzz_bounded(&mut s, 20.0),
                    fuzz_bounded(&mut s, 20.0),
                    fuzz_bounded(&mut s, 20.0),
                ]
            })
            .collect();
        let scale = fuzz_bounded(&mut s, 20.0).abs() + 0.1;
        run(
            &mut emu,
            count,
            scale,
            &knots,
            &cps,
            fuzz_bounded(&mut s, 1.0),
        );
    }
}
