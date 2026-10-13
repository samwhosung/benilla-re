//! The lightning bolts (`systems/spell/visuals.md`): the points called whole with the global
//! `CRandom` seeded, the update and the render as slices.

#![allow(clippy::needless_range_loop)] // the loops index the client's memory layout
#![allow(clippy::too_many_arguments)] // a case takes every field it writes

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_gx::{
    bolt_scalar, lightning_age, lightning_blend, lightning_param_pos, lightning_step_fraction,
    lightning_subdivide, normalize_perp, strip_offsets, BlendVertex, LightningBolt, Point,
    LIGHTNING_PRNG_VA,
};
use benilla_re_random::TABLE_VA;
use common::{assert_f32, frand, next, read_ah, SLICE_ESP};

/// The `CRandom` table at `0x802700` and the three bytes past its last lane.
fn read_table(emu: &Emu) -> [u8; 0x103] {
    let mut t = [0u8; 0x103];
    for (i, b) in t.iter_mut().enumerate() {
        *b = emu
            .read_u32(u64::from(TABLE_VA) + (i as u64 & !3))
            .to_le_bytes()[i & 3];
    }
    t
}

struct BoltRng(u32);

impl BoltRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A "sane" mixed-magnitude finite f32 (emitter radii / pivots).
    fn f32_sane(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 4;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 50.0) as f32,
            2 => (-(m * 50.0)) as f32,
            _ => (m * 5.0) as f32,
        }
    }
}

struct StripRng(u32);

impl StripRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite f32 from the LCG's high bits, in a modest world-space range.
    fn f32_world(&mut self) -> f32 {
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        (m * 40.0 - 20.0) as f32
    }
}

fn transform(p: Point, eye: Point) -> Point {
    [
        (f64::from(p[0]) - f64::from(eye[0])) as f32,
        (f64::from(p[1]) - f64::from(eye[1])) as f32,
        (f64::from(p[2]) - f64::from(eye[2])) as f32,
    ]
}

fn expected_strip(points: &[Point], eye: Point, width: f32, n: usize) -> Vec<[f32; 3]> {
    let xf: Vec<Point> = points.iter().map(|&p| transform(p, eye)).collect();
    // 2n vertices, seeded to 0.0 (the binary ×0.0-zeroes record 0 and 1, then accumulates).
    let mut strip = vec![[0.0f32; 3]; 2 * n];

    // The interior loop: `prev` starts at transformed point 0, `cur` walks points 1..=n-1. The
    // binary increments its `uVar5` counter at the TOP (`uVar5 += 2`) then checks `uVar5 < 2n-2` at
    // the BOTTOM, so the loop runs exactly `n-1` iterations (cur = points 1,2,…,n-1). Each
    // iteration accumulates the prev-point outputs onto the previous record and WRITES the
    // cur-point outputs.
    let mut prev = xf[0];
    let mut idx = 1usize; // cur point index into xf
    let mut iter = 0u32;
    loop {
        iter += 2;
        let cur = xf[idx];
        let perp = normalize_perp(prev, cur);
        let (pp, pm, cp, cm) = strip_offsets(prev, cur, perp, width);
        // record (idx-1) = the *previous* point's record: verts 2(idx-1), 2(idx-1)+1 — ACCUMULATED.
        let rprev = 2 * (idx - 1);
        for k in 0..3 {
            strip[rprev][k] = (f64::from(strip[rprev][k]) + f64::from(pp[k])) as f32;
            strip[rprev + 1][k] = (f64::from(strip[rprev + 1][k]) + f64::from(pm[k])) as f32;
        }
        // record idx = the *current* point's record: verts 2·idx, 2·idx+1 — WRITTEN.
        let rcur = 2 * idx;
        strip[rcur] = cp;
        strip[rcur + 1] = cm;

        prev = cur;
        idx += 1;
        if iter >= (2 * n as u32).wrapping_sub(2) {
            break;
        }
    }

    // The post-loop writes endpoint 0 into record 0 (verts 0,1) and endpoint n-1 into the last
    // record (verts 2n-2, 2n-1), OVERWRITING whatever the interior loop accumulated there.
    strip[0] = xf[0];
    strip[1] = xf[0];
    strip[2 * n - 2] = xf[n - 1];
    strip[2 * n - 1] = xf[n - 1];
    strip
}

fn run_slice(
    e: &mut Emu,
    this: u64,
    pts: u64,
    vbuf: u64,
    ebp: u64,
    eye_va: u64,
    points: &[Point],
    eye: Point,
    width: f32,
    denom: f32,
    numer: f32,
    n: usize,
) -> (Vec<[f32; 3]>, f32) {
    // identity projection into 0xcf57c0 (0x7bc6a0 reads it; result 0xcf5800 = T).
    let mut ident = [0f32; 16];
    ident[0] = 1.0;
    ident[5] = 1.0;
    ident[10] = 1.0;
    ident[15] = 1.0;
    for (i, &f) in ident.iter().enumerate() {
        e.write_u32(0xcf57c0 + 4 * i as u64, f.to_bits());
        e.write_u32(0xcf5800 + 4 * i as u64, f.to_bits());
    }
    // skip the one-time init blocks (not in the slice, but harmless to pre-set).
    e.write_u32(0xcf57a8, 0xff);

    // bolt points (C3Vector stride 0xc).
    for (i, p) in points.iter().enumerate() {
        for (k, c) in p.iter().enumerate() {
            e.write_u32(pts + (i as u64 * 0xc) + k as u64 * 4, c.to_bits());
        }
    }
    // eye vector.
    for (k, c) in eye.iter().enumerate() {
        e.write_u32(eye_va + k as u64 * 4, c.to_bits());
    }
    // struct fields.
    e.write_u32(this + 0x1c, width.to_bits());
    e.write_u32(this + 0x28, denom.to_bits());
    e.write_u32(this + 0x34, n as u32); // INTEGER count
    e.write_u32(this + 0x38, pts as u32);
    e.write_u32(this + 0x44, vbuf as u32);
    e.write_u32(this + 0x60, numer.to_bits());

    // clear the strip buffer.
    for i in 0..(2 * n as u64 * 3 + 8) {
        e.write_u32(vbuf + i * 4, 0xdead_beef);
    }

    e.set_reg(RegisterX86::FPCW, 0x027F);
    e.set_reg(RegisterX86::ESI, this);
    e.set_reg(RegisterX86::EBP, ebp);
    e.set_reg(RegisterX86::ESP, SLICE_ESP);
    e.write_u32(ebp + 0xc, eye_va as u32);
    e.write_u32(ebp + 8, 0);

    e.run(0x7affb1, 0x7b057e);

    let mut strip = vec![[0.0f32; 3]; 2 * n];
    for (v, slot) in strip.iter_mut().enumerate() {
        for (k, c) in slot.iter_mut().enumerate() {
            *c = e.read_f32(vbuf + v as u64 * 0xc + k as u64 * 4);
        }
    }
    let scalar = e.read_f32(ebp - 0x30);
    (strip, scalar)
}

/// `0x7af6d0` whole after `0x44ea80` seeds the global `CRandom`.
#[test]
fn lightning_segment_subdivide_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    // The bolt record (`this`/param_1): endpoints +0x00/+0x0c, spacing +0x18, amplitude +0x24.
    let bolt = emu.alloc(0x40);
    // The point-array container (`param_2`): [0]=?, [1]=count, [2]=ptr. Pre-size so the grow
    // early-outs.
    let container = emu.alloc(0x10);
    let pointbuf = emu.alloc(0x4000); // (n+1)*0xc; n small here.
    let mut lcg = BoltRng(0x119b_017e);
    let mut nbig = 0u32;

    for _ in 0..120u32 {
        // a bolt whose length / spacing lands n in a small range. Keep endpoints modest.
        let p0 = [
            lcg.f32_sane() % 20.0,
            lcg.f32_sane() % 20.0,
            lcg.f32_sane() % 20.0,
        ];
        let p1 = [
            lcg.f32_sane() % 20.0,
            lcg.f32_sane() % 20.0,
            lcg.f32_sane() % 20.0,
        ];
        // spacing chosen so n = ftol(len/spacing + 2) ∈ ~[1, 12].
        let spacing = (lcg.next() % 8) as f32 + 1.0;
        let amplitude = lcg.f32_sane() % 2.0;
        let b = LightningBolt {
            p0,
            p1,
            spacing,
            amplitude,
        };
        let seed = lcg.next();

        emu.write_u32(bolt, p0[0].to_bits());
        emu.write_u32(bolt + 4, p0[1].to_bits());
        emu.write_u32(bolt + 8, p0[2].to_bits());
        emu.write_u32(bolt + 0xc, p1[0].to_bits());
        emu.write_u32(bolt + 0x10, p1[1].to_bits());
        emu.write_u32(bolt + 0x14, p1[2].to_bits());
        emu.write_u32(bolt + 0x18, spacing.to_bits());
        emu.write_u32(bolt + 0x24, amplitude.to_bits());

        // seed the GLOBAL CRandom at 0xcf57b0.
        emu.call(0x44ea80, LIGHTNING_PRNG_VA, 0, &[seed]);
        let mut state = (
            emu.read_u32(u64::from(LIGHTNING_PRNG_VA)),
            emu.read_u32(u64::from(LIGHTNING_PRNG_VA) + 4),
        );
        let ours = lightning_subdivide(&b, &mut state, &table);
        let count = ours.len();

        // pre-size the container so the binary's grow (uVar12 == param_2[1]) early-outs and reuses
        // our buf.
        emu.write_u32(container, 0);
        emu.write_u32(container + 4, count as u32); // param_2[1] = current count == target
        emu.write_u32(container + 8, pointbuf as u32); // param_2[2] = point buffer
        for i in 0..0x800u64 {
            emu.write_u32(pointbuf + i * 4, 0xdead_beef);
        }
        // __thiscall: ecx=bolt(param_1), stack = container(param_2).
        emu.call(0x7af6d0, bolt as u32, 0, &[container as u32]);

        // the binary's resulting point count.
        let bcount = emu.read_u32(container + 4) as usize;
        assert_eq!(bcount, count, "lightning point count");
        for (pi, op) in ours.iter().enumerate() {
            for k in 0..3 {
                assert_f32(
                    emu.read_f32(pointbuf + (pi as u64 * 0xc) + k as u64 * 4),
                    op[k],
                    &format!("lightning pt[{pi}][{k}]"),
                );
            }
        }
        // PRNG lockstep: the binary advanced 3 draws per interior point.
        assert_eq!(
            emu.read_u32(u64::from(LIGHTNING_PRNG_VA)),
            state.0,
            "lightning prng s0"
        );
        if count > 2 {
            nbig += 1;
        }
    }
    assert!(nbig > 0, "the interior-loop path (n>1) must fire");
}

/// `0x7affb1..0x7b057e` of the render: the strip of a bolt.
#[test]
fn lightningrender_strip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // The instructions the model depends on, read from the exe: the stores and the branches.
    assert_eq!(
        e.read(0x7b01b3, 2),
        vec![0xd9u8, 0xfa],
        "0x7b01b3 fsqrt drifted"
    );
    assert_eq!(
        e.read(0x7b01bd, 3),
        vec![0xf6u8, 0xc4, 0x41],
        "0x7b01bd test ah,0x41 (normalize eps gate) drifted"
    );
    assert_eq!(
        e.read(0x7b01c0, 1),
        vec![0x75u8],
        "0x7b01c0 jne (normalize gate) drifted"
    );
    assert_eq!(
        e.read(0x7b01cb, 2),
        vec![0xd8u8, 0xc9],
        "0x7b01cb fmul st,st(1) (recip LIVE) drifted"
    );
    assert_eq!(
        e.read(0x7b054f, 3),
        vec![0xf6u8, 0xc4, 0x44],
        "0x7b054f test ah,0x44 (bolt-scalar denom gate) drifted"
    );
    assert_eq!(
        e.read(0x7b0552, 1),
        vec![0x7bu8],
        "0x7b0552 jnp (bolt-scalar gate) drifted"
    );

    let this = e.alloc(0x200);
    let vbuf = e.alloc(0x4000);
    let pts = e.alloc(0x1000);
    let frame = e.alloc(0x800);
    let ebp = frame + 0x400;
    let eye_va = e.alloc(0x20);

    let mut interior_seen = 0u32;

    // a closure asserting the whole strip + scalar for a given bolt.
    let mut run_case = |e: &mut Emu,
                        points: &[Point],
                        eye: Point,
                        width: f32,
                        denom: f32,
                        numer: f32,
                        ctx: &str| {
        let n = points.len();
        let (bin, bscalar) = run_slice(
            e, this, pts, vbuf, ebp, eye_va, points, eye, width, denom, numer, n,
        );
        let exp = expected_strip(points, eye, width, n);
        for (v, (b, x)) in bin.iter().zip(exp.iter()).enumerate() {
            for k in 0..3 {
                assert_f32(b[k], x[k], &format!("{ctx} vert[{v}][{k}]"));
            }
        }
        let exp_scalar = bolt_scalar(denom, numer);
        assert_f32(bscalar, exp_scalar, &format!("{ctx} bolt scalar"));
        if n > 2 {
            interior_seen += 1;
        }
    };

    // --- deterministic edge / ==threshold probes ---

    // 2-point bolt: no interior loop, only the two endpoints (each written twice).
    run_case(
        &mut e,
        &[[0.0, 0.0, 0.0], [3.0, 4.0, 5.0]],
        [1.0, 1.0, 1.0],
        0.5,
        2.0,
        1.0,
        "2pt endpoints-only",
    );

    // 3-point bolt: one interior point, the jitter loop fires once.
    run_case(
        &mut e,
        &[[0.0, 0.0, 0.0], [5.0, 1.0, 2.0], [10.0, 0.0, 4.0]],
        [1.0, 2.0, 3.0],
        0.5,
        0.8,
        1.5,
        "3pt one-interior",
    );

    // bolt-scalar denom == 0.0 boundary ⇒ divisor 1.0 (the jnp arm).
    run_case(
        &mut e,
        &[[0.0, 0.0, 0.0], [2.0, 3.0, 1.0], [4.0, 0.0, 2.0]],
        [0.0, 0.0, 0.0],
        1.0,
        0.0, // denom == 0 ⇒ scalar = -(numer/1.0)
        2.5,
        "denom==0 boundary",
    );
    // negative denom (uses denom, not 1.0).
    run_case(
        &mut e,
        &[[0.0, 0.0, 0.0], [2.0, 3.0, 1.0], [4.0, 0.0, 2.0]],
        [0.0, 0.0, 0.0],
        1.0,
        -4.0,
        2.0,
        "denom<0",
    );

    // normalize eps boundary: a segment whose screen-perpendicular length is EXACTLY 0.001.
    // perp = (-(dy), dx, 0); |perp| = sqrt(dx²+dy²) = sqrt of the XY screen delta. Make the delta
    // purely in x by 0.001 (dy=0): cur.x-prev.x = 0.001, cur.y=prev.y, cur.z arbitrary. Eye 0.
    // len == 0.001 EXACTLY ⇒ NOT normalized (the `<` is strict): perp stays (0, 0.001, 0).
    run_case(
        &mut e,
        &[[0.0, 0.0, 0.0], [0.001, 0.0, 1.0], [0.002, 0.0, 2.0]],
        [0.0, 0.0, 0.0],
        2.0,
        1.0,
        1.0,
        "norm eps==0.001 boundary",
    );
    // just above eps ⇒ normalized.
    run_case(
        &mut e,
        &[[0.0, 0.0, 0.0], [0.01, 0.0, 1.0], [0.02, 0.0, 2.0]],
        [0.0, 0.0, 0.0],
        2.0,
        1.0,
        1.0,
        "norm just-above-eps",
    );

    // degenerate: prev == cur in screen XY (zero perpendicular ⇒ len 0 ⇒ NOT normalized, perp=0).
    run_case(
        &mut e,
        &[[0.0, 0.0, 0.0], [0.0, 0.0, 3.0], [0.0, 0.0, 6.0]],
        [2.0, 2.0, 2.0],
        1.0,
        1.0,
        1.0,
        "zero-perp degenerate",
    );

    // --- fuzz: many bolts, varying point count, width, eye, denom/numer ---
    let mut lcg = StripRng(0x7afc_b00d);
    for f in 0..4000u32 {
        let n = 2 + (lcg.next() >> 8) as usize % 7; // 2..=8 points
        let points: Vec<Point> = (0..n)
            .map(|_| [lcg.f32_world(), lcg.f32_world(), lcg.f32_world()])
            .collect();
        let eye = [lcg.f32_world(), lcg.f32_world(), lcg.f32_world()];
        let width = lcg.f32_world().abs() % 5.0 + 0.1;
        // denom: occasionally exactly 0.0 to exercise the divisor-replacement arm.
        let denom = if (lcg.next() >> 8).is_multiple_of(11) {
            0.0
        } else {
            lcg.f32_world()
        };
        let numer = lcg.f32_world();
        run_case(
            &mut e,
            &points,
            eye,
            width,
            denom,
            numer,
            &format!("fuzz[{f}]"),
        );
    }

    assert!(
        interior_seen > 0,
        "the interior jitter loop (n>2) must fire in the suite"
    );
}

/// The bytes `0x7af9d7..0x7af9dd`, copied: the age sum.
#[test]
fn lightning_age_fadd_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    // byte-verify the island source bytes EXACTLY match the binary (else the island is circular).
    let fld_ebp8 = e.read(0x7af9d7, 3); // d9 45 08
    let fadd_esi60 = e.read(0x7af9da, 3); // d8 46 60
    assert_eq!(
        fld_ebp8,
        vec![0xd9u8, 0x45, 0x08],
        "0x7af9d7 fld [ebp+8] drifted"
    );
    assert_eq!(
        fadd_esi60,
        vec![0xd8u8, 0x46, 0x60],
        "0x7af9da fadd [esi+0x60] drifted"
    );

    // assemble the island: <fld [ebp+8]> <fadd [esi+0x60]> <fstp qword [slot]>.
    let slot = e.alloc(0x10);
    let island = e.alloc(0x20);
    let mut code = Vec::new();
    code.extend_from_slice(&fld_ebp8);
    code.extend_from_slice(&fadd_esi60);
    code.push(0xdd); // fstp qword [disp32]
    code.push(0x1d);
    code.extend_from_slice(&(slot as u32).to_le_bytes());
    let island_end = island + code.len() as u64;
    e.write(island, &code);

    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x100);

    let read_f64 = |e: &Emu, va: u64| -> f64 {
        let b = e.read(va, 8);
        let mut a = [0u8; 8];
        a.copy_from_slice(&b);
        f64::from_bits(u64::from_le_bytes(a))
    };

    let run_case = |e: &mut Emu, elapsed: f32, age_prev: f32, ctx: &str| {
        e.write_u32(ebp + 8, elapsed.to_bits());
        e.write_u32(this + 0x60, age_prev.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, this);
        e.run(island, island_end);
        let bin = read_f64(e, slot); // the LIVE f64 sum the binary feeds to fmod
        let ours = f64::from(elapsed) + f64::from(age_prev);
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: binary={:#018x} ours={:#018x} (elapsed={elapsed} age_prev={age_prev})",
            bin.to_bits(),
            ours.to_bits()
        );
        // and confirm the model fn's fmod wrap matches f64 `%` on the same sum (period nonzero).
        let period = 1.5f32;
        let m = lightning_age(elapsed, age_prev, period);
        assert_f32(
            m,
            (ours % f64::from(period)) as f32,
            &format!("{ctx} fmod-wrap"),
        );
    };

    // edge probes: accumulation, sign mixes, cancellation, large magnitudes.
    run_case(&mut e, 0.5, 0.0, "age 0.5+0");
    run_case(&mut e, 0.3, 0.4, "age 0.3+0.4");
    run_case(&mut e, -0.3, 1.0, "age -0.3+1.0");
    run_case(&mut e, 1.0e7, 1.0e-7, "age big+tiny (rounding)");
    run_case(&mut e, -5.0, 5.0, "age cancel to 0");

    let mut s = 0x7af9_d7d7u32;
    for i in 0..6000u32 {
        let elapsed = frand(&mut s, -50.0, 50.0);
        let age_prev = frand(&mut s, -50.0, 50.0);
        run_case(&mut e, elapsed, age_prev, &format!("age fuzz[{i}]"));
    }
}

/// `0x7af9c7..0x7af9d2`: the period test read at its `fnstsw`.
#[test]
fn lightning_age_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);
    let this = e.alloc(0x100);

    let run_case = |e: &mut Emu, period: f32, ctx: &str| {
        e.write_u32(this + 0x28, period.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::ESI, this);
        e.run(0x7af9c7, 0x7af9d2); // fld [esi+0x28]; fcomp [0.0]; fnstsw ax
        let ah = read_ah(e);
        // `test ah,0x44; jnp` takes ZERO path ⇔ parity of (ah & 0x44) ODD.
        let bin_zero = !(ah & 0x44).count_ones().is_multiple_of(2);
        // value model: zero ⇔ period == 0.0 (NaN != 0 → fmod path).
        let ours_zero = period == 0.0;
        assert_eq!(
            bin_zero, ours_zero,
            "{ctx}: binary zero-path={bin_zero} ours={ours_zero} (period={period} ah={ah:#04x})"
        );
    };

    // ==0.0 boundary: 0.0 and -0.0 → zero path; tiny → fmod; NaN → fmod.
    run_case(&mut e, 0.0, "gate period==0 (zero)");
    run_case(&mut e, -0.0, "gate period==-0 (zero)");
    run_case(
        &mut e,
        f32::from_bits(1),
        "gate period smallest-denormal (fmod)",
    );
    run_case(&mut e, 1.0, "gate period=1 (fmod)");
    run_case(&mut e, -1.0, "gate period<0 (fmod)");
    run_case(&mut e, f32::NAN, "gate period=NaN (fmod)");

    let mut s = 0x7af9_c7c7u32;
    for i in 0..4000u32 {
        let period = match (next(&mut s) >> 8) % 4 {
            0 => 0.0,
            1 => frand(&mut s, -2.0, 2.0),
            2 => frand(&mut s, 0.0, 100.0),
            _ => frand(&mut s, -100.0, 0.0),
        };
        run_case(&mut e, period, &format!("gate fuzz[{i}] period={period}"));
    }
}

/// `0x7afa17..0x7afa26`: the step.
#[test]
fn lightning_step_fraction_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    // byte-verify it's `fdivr [0x7ff9d8]` (REVERSE, d8 3d ..) not `fdiv` (d8 35 ..).
    assert_eq!(
        e.read(0x7afa1d, 6),
        vec![0xd8u8, 0x3d, 0xd8, 0xf9, 0x7f, 0x00],
        "0x7afa1d fdivr [0x7ff9d8] drifted"
    );

    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x100); // esi: only the harmless `cmp edi,[esi+0x4c]` reads it.

    let run_case = |e: &mut Emu, segs: i32, ctx: &str| {
        let denom = i64::from(segs).wrapping_mul(2).wrapping_sub(2);
        e.write_u32(ebp.wrapping_sub(0x18), denom as u32);
        e.write_u32(ebp.wrapping_sub(0x14), (denom >> 32) as u32);
        e.write_u32(ebp.wrapping_sub(4), 0xdead_beef);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, this); // [esi+0x4c] for the `cmp` (FP-irrelevant)
        e.run(0x7afa17, 0x7afa26);
        let bin = e.read_f32(ebp.wrapping_sub(4));
        let ours = lightning_step_fraction(segs);
        assert_f32(bin, ours, &format!("{ctx} (segs={segs} denom={denom})"));
    };

    // edge probes: segs=2 (denom=2), segs=1 (denom=0 → fdivr 1/0 = +Inf), segs=0 (denom=-2),
    // large segs.
    run_case(&mut e, 2, "step segs=2");
    run_case(&mut e, 1, "step segs=1 (denom=0 → Inf)");
    run_case(&mut e, 0, "step segs=0 (denom=-2)");
    run_case(&mut e, 3, "step segs=3");
    run_case(&mut e, 16, "step segs=16");
    run_case(&mut e, 1000, "step segs=1000");

    let mut s = 0x7afa_1717u32;
    for i in 0..4000u32 {
        let segs = ((next(&mut s) >> 8) % 4096) as i32;
        run_case(&mut e, segs, &format!("step fuzz[{i}]"));
    }
}

/// `0x7afb1d..0x7afb2f`: a vertex's parameter.
#[test]
fn lightning_param_pos_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let scratch = e.alloc(0x100); // edx: the in-slice `mov [edx+8*eax]` target

    let run_case = |e: &mut Emu, i: i64, step: f32, ctx: &str| {
        e.write_u32(ebp.wrapping_sub(0x24), i as u32);
        e.write_u32(ebp.wrapping_sub(0x20), (i >> 32) as u32);
        e.write_u32(ebp.wrapping_sub(4), step.to_bits());
        e.write_u32(ebp.wrapping_sub(0xc), 0xdead_beef);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::EDX, scratch); // [edx+8*eax]
        e.set_reg(RegisterX86::EAX, 0); // eax index = 0
        e.run(0x7afb1d, 0x7afb2f);
        let bin = e.read_f32(ebp.wrapping_sub(0xc));
        let ours = lightning_param_pos(i, step);
        assert_f32(bin, ours, &format!("{ctx} (i={i} step={step})"));
    };

    // edge probes: i=0, i=1, even indices, step from a real fdivr, negative step.
    run_case(&mut e, 0, 0.1, "pos i=0");
    run_case(&mut e, 1, 0.1, "pos i=1");
    run_case(&mut e, 2, 0.25, "pos i=2");
    run_case(&mut e, 100, 0.01, "pos i=100");
    run_case(&mut e, 7, -0.3, "pos negative step");

    let mut s = 0x7afb_1d1du32;
    for i in 0..4000u32 {
        let idx = i64::from((next(&mut s) >> 8) % 4096);
        let step = frand(&mut s, -1.0, 1.0);
        run_case(&mut e, idx, step, &format!("pos fuzz[{i}]"));
    }
}

/// `0x7afc0d..0x7afc81`: the blend.
#[test]
fn lightning_blend_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    // byte-verify the live `fadd st,st(2)` (x) and the reloaded `fld [ebp-0x18]; fadd st,st(1)`
    // (y).
    assert_eq!(
        e.read(0x7afc56, 2),
        vec![0xd8u8, 0xc2],
        "0x7afc56 fadd st,st(2) (x LIVE) drifted"
    );
    assert_eq!(
        e.read(0x7afc5b, 3),
        vec![0xd9u8, 0x45, 0xe8],
        "0x7afc5b fld [ebp-0x18] (y reload) drifted"
    );
    assert_eq!(
        e.read(0x7afc61, 2),
        vec![0xd8u8, 0xc1],
        "0x7afc61 fadd st,st(1) (y) drifted"
    );
    assert_eq!(
        e.read(0x7afc72, 3),
        vec![0xd9u8, 0x45, 0xec],
        "0x7afc72 fld [ebp-0x14] (z reload) drifted"
    );
    assert_eq!(
        e.read(0x7afc75, 3),
        vec![0xd8u8, 0x45, 0xf8],
        "0x7afc75 fadd [ebp-0x8] (z) drifted"
    );

    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x100); // esi: +0x38 = vertex base
    let ref_base = e.alloc(0x80); // [0xcf585c] points here (reference buffer)
    let vert_base = e.alloc(0x80); // [esi+0x38] points here (the strip vertex buffer)

    // [0xcf585c] is a runtime global the slice reads as `eax`; we set eax directly (the slice does
    // not re-read it inside the diffed range — eax is the base, ecx the offset).
    e.write_u32(this + 0x38, vert_base as u32);

    let run_case = |e: &mut Emu, vert: [f32; 3], reference: [f32; 3], ctx: &str| {
        // offset 0 (ecx=0): ref at [ref_base], vert at [vert_base].
        for (k, v) in reference.iter().enumerate() {
            e.write_u32(ref_base + (k as u64) * 4, v.to_bits());
        }
        for (k, v) in vert.iter().enumerate() {
            e.write_u32(vert_base + (k as u64) * 4, v.to_bits());
        }
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ECX, 0); // loop offset = 0
        e.set_reg(RegisterX86::EAX, ref_base); // [0xcf585c] base, used as [ecx+eax]
        e.set_reg(RegisterX86::EDX, 2); // edx = loop counter (dec edx; not 0 → harmless)
        e.run(0x7afc0d, 0x7afc81);
        let bin = BlendVertex {
            x: e.read_f32(vert_base),
            y: e.read_f32(vert_base + 4),
            z: e.read_f32(vert_base + 8),
        };
        let ours = lightning_blend(vert, reference);
        assert_f32(bin.x, ours.x, &format!("{ctx} x (LIVE+LIVE)"));
        assert_f32(bin.y, ours.y, &format!("{ctx} y (reload+LIVE)"));
        assert_f32(bin.z, ours.z, &format!("{ctx} z (reload+reload)"));
    };

    // edge probes: zeros, unit, mixed signs, values that force the live-vs-reload divergence (odd
    // mantissas whose ·0.75 / ·0.25 product is not f32-representable).
    run_case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "blend zeros");
    run_case(&mut e, [1.0, 1.0, 1.0], [1.0, 1.0, 1.0], "blend ones");
    run_case(
        &mut e,
        [
            f32::from_bits(0x3f800001),
            f32::from_bits(0x3f800001),
            f32::from_bits(0x3f800001),
        ],
        [
            f32::from_bits(0x3f800003),
            f32::from_bits(0x3f800003),
            f32::from_bits(0x3f800003),
        ],
        "blend odd-mantissa (live-vs-reload divergence)",
    );
    run_case(&mut e, [-3.0, 2.0, -1.0], [4.0, -5.0, 6.0], "blend signed");
    run_case(
        &mut e,
        [100.0, 0.001, -7.5],
        [0.25, 33.0, -0.125],
        "blend spread",
    );

    let mut s = 0x7afc_0d0du32;
    for i in 0..6000u32 {
        let vert = [
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
        ];
        let reference = [
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
        ];
        run_case(&mut e, vert, reference, &format!("blend fuzz[{i}]"));
    }
}
