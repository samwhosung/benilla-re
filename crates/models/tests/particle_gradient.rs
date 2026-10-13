//! The particle gradients (`systems/models/particles.md#gradients`): the sample and the record
//! setters called whole, the lifespan test read at its `fnstsw`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::particles::gradient::{
    cell_ramp_record, gradient_range, gradient_record, particle_gradient_sample, GradientRange,
    GradientRecord, GradientSpans, LIFESPAN_MIN,
};
use common::{assert_f32, frand, next, read_ah, SLICE_ESP};

struct GradRng(u32);

impl GradRng {
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

const GRADIENT_EPS_VA: u64 = 0x801360;

fn fuzz_range(s: &mut u32) -> f32 {
    match (next(s) >> 8) % 5 {
        0 => frand(s, -1.0, 1.0),
        1 => frand(s, -1000.0, 1000.0),
        2 => frand(s, -1.0e30, 1.0e30),
        3 => frand(s, -1.0e-20, 1.0e-20),
        _ => frand(s, -100.0, 100.0),
    }
}

/// `0x7b9b10` whole on random records and ages, both cell legs.
#[test]
fn particle_gradient_sample_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = emu.alloc(0x60);
    let out_color = emu.alloc(0x10);
    let out_p5 = emu.alloc(0x10);
    let out_p6 = emu.alloc(0x10);
    let out_p7 = emu.alloc(0x10);
    let mut lcg = GradRng(0x9e37_79b1);
    let (mut linear, mut nonlinear) = (0u32, 0u32);

    for iter in 0..600u32 {
        // Build a record. Bytes 0..3, signed int deltas (kept modest so quantized channels stay
        // sane), float scale/age fields, and the curve selector (mostly 1.0, some not).
        let mut r = GradientRecord::default();
        for k in 0..4 {
            r.start_bytes[k] = (lcg.next() >> 8) as u8;
            r.color_delta[k] = ((lcg.next() >> 8) % 4096) as i32 - 2048;
        }
        r.p5 = [
            ((lcg.next() >> 8) % 4096) as i32,
            ((lcg.next() >> 8) % 4096) as i32 - 2048,
        ];
        r.p6 = [
            ((lcg.next() >> 8) % 4096) as i32,
            ((lcg.next() >> 8) % 4096) as i32 - 2048,
        ];
        r.scale_base = lcg.f32_sane();
        r.scale_delta = lcg.f32_sane();
        r.age_origin = (lcg.next() % 100) as f32 * 0.01;
        r.age_slope = (lcg.next() % 200) as f32 * 0.02 + 0.01;
        // ~1/5 of cases nonlinear.
        let is_nl = iter % 5 == 4;
        r.curve = if is_nl {
            // a positive, non-1.0 exponent-bits value.
            f32::from_bits(0x3f00_0000 | ((lcg.next() >> 16) & 0x7fff)) // ~[0.5,1)
                .to_bits()
        } else {
            0x3f80_0000
        };
        let age = (lcg.next() % 300) as f32 * 0.005; // [0, 1.5)
        let p3 = (lcg.next() % 200) as f32 * 0.01; // [0, 2)

        // lay out the record at the binary's byte offsets.
        emu.write(rec, &r.start_bytes);
        emu.write_u32(rec + 0x4, r.color_delta[0] as u32);
        emu.write_u32(rec + 0x8, r.color_delta[1] as u32);
        emu.write_u32(rec + 0xc, r.color_delta[2] as u32);
        emu.write_u32(rec + 0x10, r.color_delta[3] as u32);
        emu.write_u32(rec + 0x14, r.p5[0] as u32);
        emu.write_u32(rec + 0x18, r.p5[1] as u32);
        emu.write_u32(rec + 0x1c, r.p6[0] as u32);
        emu.write_u32(rec + 0x20, r.p6[1] as u32);
        emu.write_u32(rec + 0x24, r.scale_base.to_bits());
        emu.write_u32(rec + 0x28, r.scale_delta.to_bits());
        emu.write_u32(rec + 0x2c, r.age_origin.to_bits());
        emu.write_u32(rec + 0x30, r.age_slope.to_bits());
        emu.write_u32(rec + 0x50, r.curve);
        // zero outputs.
        for o in [out_color, out_p5, out_p6, out_p7] {
            emu.write_u32(o, 0xdead_beef);
        }
        // __thiscall: ecx=rec; stack = age, p3, &color, &p5, &p6, &p7. ret 0x18.
        emu.call(
            0x7b9b10,
            rec as u32,
            0,
            &[
                age.to_bits(),
                p3.to_bits(),
                out_color as u32,
                out_p5 as u32,
                out_p6 as u32,
                out_p7 as u32,
            ],
        );
        let bcolor = emu.read_u32(out_color);
        let bp5 = emu.read_u32(out_p5);
        let bp6 = emu.read_u32(out_p6);
        let bp7 = emu.read_f32(out_p7);

        let ours = particle_gradient_sample(&r, age, p3);

        // Color BGRA + scale + age-derived outputs: bit-exact on BOTH branches.
        let our_color = u32::from(ours.color[0])
            | (u32::from(ours.color[1]) << 8)
            | (u32::from(ours.color[2]) << 16)
            | (u32::from(ours.color[3]) << 24);
        assert_eq!(
            bcolor & 0x00ff_ff00,
            our_color & 0x00ff_ffff_u32 & 0x00ff_ff00,
            "gradient color RGB iter={iter}"
        );
        // Compare all four color bytes individually (param_4 is a byte array [b0,b1,b2,b3]).
        for k in 0..4 {
            let bb = (bcolor >> (k * 8)) & 0xff;
            assert_eq!(
                bb,
                u32::from(ours.color[k]),
                "gradient color byte[{k}] iter={iter}"
            );
        }
        assert_f32(bp7, ours.scale, &format!("gradient scale iter={iter}"));
        // The cells on both legs; the repeat leg's `fmod` is exact.
        assert_eq!(
            bp5, ours.p5,
            "gradient p5 (head cell) iter={iter} nl={is_nl}"
        );
        assert_eq!(
            bp6, ours.p6,
            "gradient p6 (tail cell) iter={iter} nl={is_nl}"
        );
        if is_nl {
            nonlinear += 1;
        } else {
            linear += 1;
        }
    }
    assert!(linear > 0 && nonlinear > 0, "both curve branches fire");
}

/// `0x7b9da0` and `0x7b9de0` whole: rising, falling and equal ranges.
#[test]
fn cell_ramp_record_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x60);

    // (fn VA, raw-lo off, raw-hi off, base off, span off)
    const SITES: [(u64, u64, u64, u64, u64); 2] = [
        (0x7b9da0, 0x3c, 0x40, 0x14, 0x18), // head ramp (file u16 +0x168/+0x16a | +0x16e/+0x170)
        (0x7b9de0, 0x44, 0x48, 0x1c, 0x20), // tail ramp (file u16 +0x174/+0x176 | +0x178/+0x17a)
    ];

    let (mut asc, mut desc, mut eq) = (0u32, 0u32, 0u32);
    let run_case = |e: &mut Emu, start: i32, end: i32, ctx: &str| {
        for &(va, lo, hi, bo, so) in SITES.iter() {
            for off in [lo, hi, bo, so] {
                e.write_u32(rec + off, 0xdead_beef); // junk-seed every output field
            }
            // __thiscall: ecx = rec, stack = start, end. ret 8.
            e.call(va, rec as u32, 0, &[start as u32, end as u32]);
            let ours = cell_ramp_record(start, end);
            assert_eq!(
                e.read_u32(rec + lo) as i32,
                ours.raw_lo,
                "{ctx} @{va:#x} raw_lo"
            );
            assert_eq!(
                e.read_u32(rec + hi) as i32,
                ours.raw_hi,
                "{ctx} @{va:#x} raw_hi"
            );
            assert_eq!(
                e.read_u32(rec + bo) as i32,
                ours.base,
                "{ctx} @{va:#x} base"
            );
            assert_eq!(
                e.read_u32(rec + so) as i32,
                ours.span,
                "{ctx} @{va:#x} span"
            );
        }
    };

    // ==boundary probes the fuzz never reliably lands on: equal endpoints (the `jl`-not-taken
    // edge), and the adjacent ±1 pairs on either side of it.
    for &(s, t, kind) in &[
        (0i32, 0i32, "equal 0"),
        (7, 7, "equal 7"),
        (7, 8, "asc adjacent"),
        (8, 7, "desc adjacent"),
        (0, 15, "asc full 4x4"),
        (0, 63, "asc full 8x8"),
        (6, 5, "SHIPPED DwarvenBrazier01 seg-B (6,5)"),
        (31, 16, "SHIPPED ShadowWordSilence seg-A (31,16)"),
        (15, 0, "SHIPPED ShadowWordSilence seg-B (15,0)"),
        (63, 0, "desc full 8x8"),
        (0, 65535, "asc u16 max"),
        (65535, 0, "desc u16 max"),
    ] {
        run_case(&mut e, s, t, kind);
        match t.cmp(&s) {
            core::cmp::Ordering::Less => desc += 1,
            core::cmp::Ordering::Equal => eq += 1,
            core::cmp::Ordering::Greater => asc += 1,
        }
    }

    // Fuzz over the loader's real domain (zero-extended u16) plus a signed band, so the `jl` is
    // exercised in both directions with wide and narrow spans.
    let mut lcg = GradRng(0x7b9d_a000);
    for _ in 0..4000u32 {
        let (s, t) = match (lcg.next() >> 8) % 3 {
            0 => (
                ((lcg.next() >> 8) % 64) as i32,
                ((lcg.next() >> 8) % 64) as i32,
            ),
            1 => (
                ((lcg.next() >> 8) % 65536) as i32,
                ((lcg.next() >> 8) % 65536) as i32,
            ),
            _ => {
                let b = ((lcg.next() >> 8) % 64) as i32;
                (b, b + ((lcg.next() >> 8) % 3) as i32 - 1) // straddles the equal boundary
            }
        };
        run_case(&mut e, s, t, "fuzz");
        match t.cmp(&s) {
            core::cmp::Ordering::Less => desc += 1,
            core::cmp::Ordering::Equal => eq += 1,
            core::cmp::Ordering::Greater => asc += 1,
        }
    }
    assert!(asc > 0 && desc > 0 && eq > 0, "all three arms fire");
}

/// `0x7b9da0` then `0x7b9b10` whole across a span: the cells run from the first to the last.
#[test]
fn cell_ramp_flipbook_oracle_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x60);
    let (oc, o5, o6, o7) = (e.alloc(0x10), e.alloc(0x10), e.alloc(0x10), e.alloc(0x10));

    // Sample the binary's cell index for an authored pair at N evenly spaced segment fractions. The
    // record's age window is the identity segment [0,1] (`+0x2c`=0, `+0x30`=1.0), which is exactly
    // what `0x7b9cf0` produces for a whole-life single segment.
    let sweep = |e: &mut Emu, start: i32, end: i32, n: u32| -> Vec<u32> {
        for off in (0..0x60u64).step_by(4) {
            e.write_u32(rec + off, 0);
        }
        e.call(0x7b9da0, rec as u32, 0, &[start as u32, end as u32]);
        e.write_u32(rec + 0x2c, 0f32.to_bits()); // age origin
        e.write_u32(rec + 0x30, 1f32.to_bits()); // age slope
        e.write_u32(rec + 0x50, 0x3f80_0000); // repeat = 1.0 (fast leg)
        (0..=n)
            .map(|i| {
                let u = i as f32 / n as f32;
                e.call(
                    0x7b9b10,
                    rec as u32,
                    0,
                    &[
                        u.to_bits(),
                        1f32.to_bits(),
                        oc as u32,
                        o5 as u32,
                        o6 as u32,
                        o7 as u32,
                    ],
                );
                // The model agrees at every sample.
                let r = GradientRecord {
                    p5: [e.read_u32(rec + 0x14) as i32, e.read_u32(rec + 0x18) as i32],
                    age_slope: 1.0,
                    curve: 0x3f80_0000,
                    ..GradientRecord::default()
                };
                let ours = particle_gradient_sample(&r, u, 1.0);
                let bin = e.read_u32(o5);
                assert_eq!(bin, ours.p5, "flipbook ({start},{end}) u={u}");
                bin
            })
            .collect()
    };

    // (a) ascending (0,15) on a 4x4 atlas: 16 cells, forward, equal time each.
    let a = sweep(&mut e, 0, 15, 4);
    assert_eq!(a, vec![0, 4, 8, 11, 15], "ascending (0,15)");

    // (b) SHIPPED inverted pair — DwarvenBrazier01.m2 emitter[1] seg B, (6,5) on 4x4: two cells,
    //     played 6 -> 5 (BACKWARDS). No swap (a swap would give 5,5,5,6,6), no clamp, no panic.
    let b = sweep(&mut e, 6, 5, 4);
    assert_eq!(b, vec![6, 6, 6, 5, 5], "inverted (6,5) plays backwards");

    // (c) SHIPPED inverted pair — ShadowWordSilence_Breath.m2, (31,16) on 8x8: a 16-frame reverse
    // run.
    let c = sweep(&mut e, 31, 16, 4);
    assert_eq!(
        c,
        vec![31, 27, 24, 20, 16],
        "inverted (31,16) plays backwards"
    );

    // (d) SHIPPED inverted pair — the second segment (15,0): the reverse of (a), same 16 cells.
    let d = sweep(&mut e, 15, 0, 4);
    assert_eq!(d, vec![15, 11, 8, 4, 0], "inverted (15,0) plays backwards");

    // Endpoint law, both directions, every atlas size that ships: cell(u=0) == start and cell(u=1)
    // == end EXACTLY — the ±1 record bias and the 0.99/0.005 inset are co-designed for it.
    for &(s, t) in &[
        (0i32, 15i32),
        (15, 0),
        (0, 63),
        (63, 0),
        (6, 5),
        (5, 6),
        (31, 16),
        (16, 31),
        (7, 7),
        (0, 0),
    ] {
        let v = sweep(&mut e, s, t, 8);
        assert_eq!(v[0], s as u32, "cell(u=0) == start for ({s},{t})");
        assert_eq!(v[8], t as u32, "cell(u=1) == end for ({s},{t})");
    }
}

/// `0x7b9b10` whole with a repeat: only the cells wrap.
#[test]
fn gradient_repeat_leg_scope_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rec = e.alloc(0x60);
    let (oc, o5, o6, o7) = (e.alloc(0x10), e.alloc(0x10), e.alloc(0x10), e.alloc(0x10));

    // A record with live colour, scale and BOTH cell ramps (the tail ramp decreasing, so the
    // negative span rides the repeat leg too).
    let base_rec = GradientRecord {
        start_bytes: [0x10, 0x40, 0x80, 0xc0],
        color_delta: [-90, 37, -140, 200],
        p5: [0, 16],   // head: ascending (0,15) -> base 0, span 16
        p6: [16, -16], // tail: descending (15,0) -> base 16, span -16
        scale_base: 0.75,
        scale_delta: -0.5,
        age_origin: 0.0,
        age_slope: 1.0,
        curve: 0x3f80_0000,
    };
    let lay = |e: &mut Emu, r: &GradientRecord| {
        e.write(rec, &r.start_bytes);
        for (i, d) in r.color_delta.iter().enumerate() {
            e.write_u32(rec + 0x4 + 4 * i as u64, *d as u32);
        }
        e.write_u32(rec + 0x14, r.p5[0] as u32);
        e.write_u32(rec + 0x18, r.p5[1] as u32);
        e.write_u32(rec + 0x1c, r.p6[0] as u32);
        e.write_u32(rec + 0x20, r.p6[1] as u32);
        e.write_u32(rec + 0x24, r.scale_base.to_bits());
        e.write_u32(rec + 0x28, r.scale_delta.to_bits());
        e.write_u32(rec + 0x2c, r.age_origin.to_bits());
        e.write_u32(rec + 0x30, r.age_slope.to_bits());
        e.write_u32(rec + 0x50, r.curve);
    };
    let sample = |e: &mut Emu, r: &GradientRecord, age: f32| -> (u32, f32, u32, u32) {
        lay(e, r);
        for o in [oc, o5, o6, o7] {
            e.write_u32(o, 0xdead_beef);
        }
        e.call(
            0x7b9b10,
            rec as u32,
            0,
            &[
                age.to_bits(),
                1f32.to_bits(),
                oc as u32,
                o5 as u32,
                o6 as u32,
                o7 as u32,
            ],
        );
        (
            e.read_u32(oc),
            e.read_f32(o7),
            e.read_u32(o5),
            e.read_u32(o6),
        )
    };

    // `a` for this record is monotone in age; bisect the f32 age whose `a` is EXACTLY 1.0, so that
    // `a · 2.0` is exactly 2.0 and `fmod` returns exactly 0.0 (the wrap boundary).
    let a_of = |age: f32| -> f32 { ((f64::from(age) * 0.99f32 as f64) + 0.005f32 as f64) as f32 };
    let (mut lo, mut hi) = (1.0f32, 1.02f32);
    for _ in 0..80 {
        let mid = f32::from_bits((lo.to_bits() + hi.to_bits()) / 2);
        if mid == lo || mid == hi {
            break;
        }
        if a_of(mid) < 1.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let age_a_is_one = if a_of(lo) == 1.0 { lo } else { hi };
    assert_eq!(a_of(age_a_is_one), 1.0, "bisection found an exact a == 1.0");

    let mut wrapped_cases = 0u32;
    for &age in &[0.0f32, 0.13, 0.5, 0.877, 1.0, age_a_is_one, 1.7, 3.25] {
        // Reference sample on the fast leg (+0x50 == 1.0).
        let (ref_color, ref_scale, _, _) = sample(&mut e, &base_rec, age);
        for &repeat in &[
            0.0f32,
            0.5,
            1.5,
            2.0,
            3.0,
            8.0,
            17.0,
            0.999_999_9,
            1.000_000_1,
        ] {
            let r = GradientRecord {
                curve: repeat.to_bits(),
                ..base_rec
            };
            let (color, scale, cell_head, cell_tail) = sample(&mut e, &r, age);

            // (i) the repeat value must NOT reach colour or scale — bit-identical to the fast leg.
            assert_eq!(
                color, ref_color,
                "colour must be independent of +0x50 (age={age} repeat={repeat})"
            );
            assert_eq!(
                scale.to_bits(),
                ref_scale.to_bits(),
                "scale must be independent of +0x50 (age={age} repeat={repeat})"
            );

            // (ii) the cells track the wrapped coefficient, and the model matches.
            let ours = particle_gradient_sample(&r, age, 1.0);
            assert_eq!(cell_head, ours.p5, "head cell (age={age} repeat={repeat})");
            assert_eq!(cell_tail, ours.p6, "tail cell (age={age} repeat={repeat})");

            // Count the cases where the wrap genuinely fired (product left [0,1)).
            let prod = f64::from(a_of(age)) * f64::from(repeat);
            if repeat != 1.0 && !(0.0..1.0).contains(&prod) {
                wrapped_cases += 1;
            }
        }
    }
    assert!(
        wrapped_cases > 0,
        "the fmod wrap must actually fire, not just pass values through"
    );
}

/// `0x7b9cf0` whole, about the 0.001 threshold.
#[test]
fn gradient_record_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    // The constants are the exe's: the threshold, the value it stores, the 1.0 numerator.
    assert_eq!(
        e.read_u32(GRADIENT_EPS_VA),
        LIFESPAN_MIN.to_bits(),
        "0x801360 gradient eps drifted"
    );
    assert_eq!(LIFESPAN_MIN.to_bits(), 0x3a83_126f, "gradient clamp imm");
    assert_eq!(
        e.read_u32(0x7ff9d8),
        1.0f32.to_bits(),
        "0x7ff9d8 1.0 drifted"
    );

    let this = e.alloc(0x200); // gradient record (m54 @ +0x54, m58 @ +0x58; spans @ +0x2c..+0x5c)

    let run_case = |e: &mut Emu, span: f32, m54: f32, m58: f32, ctx: &str| {
        e.write_u32(this + 0x54, m54.to_bits());
        e.write_u32(this + 0x58, m58.to_bits());
        for off in [0x2cu64, 0x30, 0x34, 0x5c] {
            e.write_u32(this + off, 0xdead_beef); // junk-seed the four output fields
        }
        // __thiscall: ecx = this, stack arg = span. ret 4.
        e.call(0x7b9cf0, this as u32, 0, &[span.to_bits()]);
        let bin = GradientSpans {
            at_2c: e.read_f32(this + 0x2c),
            at_34: e.read_f32(this + 0x34),
            at_30: e.read_f32(this + 0x30),
            at_5c: e.read_f32(this + 0x5c),
        };
        let ours = gradient_record(span, m54, m58);
        assert_f32(bin.at_2c, ours.at_2c, &format!("{ctx} +0x2c (span·m54)"));
        assert_f32(bin.at_34, ours.at_34, &format!("{ctx} +0x34 (span·m58)"));
        assert_f32(bin.at_30, ours.at_30, &format!("{ctx} +0x30 (slope)"));
        assert_f32(bin.at_5c, ours.at_5c, &format!("{ctx} +0x5c (span raw)"));
    };

    // ==threshold / edge probes: span below eps (clamp to 0.001), span == eps (KEEP — strict <),
    // span above eps, span 0 (→ clamp), m54==m58 (B−A=0 → slope=±Inf), NaN span (kept).
    run_case(&mut e, 0.0, 1.0, 2.0, "grad span=0 (→clamp)");
    run_case(&mut e, LIFESPAN_MIN, 1.0, 2.0, "grad span==eps (keep)");
    run_case(
        &mut e,
        f32::from_bits(LIFESPAN_MIN.to_bits() - 1),
        1.0,
        2.0,
        "grad span just-below-eps (→clamp)",
    );
    run_case(&mut e, 0.01, 3.0, 3.0, "grad m54==m58 (slope=+Inf)");
    run_case(&mut e, 1.0, 2.0, 5.0, "grad span=1");
    run_case(&mut e, 100.0, -1.0, 1.0, "grad span=100 spread");
    run_case(&mut e, f32::NAN, 1.0, 2.0, "grad span=NaN (kept)");

    let mut s = 0x7b9c_f0f0u32;
    for i in 0..6000u32 {
        let span = match (next(&mut s) >> 8) % 4 {
            0 => frand(&mut s, 0.0, 0.002), // straddles eps (0.001)
            1 => frand(&mut s, 0.0, 10.0),  // typical positive spans
            2 => frand(&mut s, -5.0, 5.0),  // includes negatives (< eps → clamp)
            _ => frand(&mut s, 0.0, 1.0e3), // large
        };
        let m54 = frand(&mut s, -100.0, 100.0);
        let m58 = frand(&mut s, -100.0, 100.0);
        run_case(&mut e, span, m54, m58, &format!("grad fuzz[{i}]"));
    }
}

/// `0x7b9cf3..0x7b9cfe`: the lifespan test read at its `fnstsw`, NaN included.
#[test]
fn gradient_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    // our model: the binary clamps iff `span < eps` (ordered, strict). The `jp` (parity-even) is
    // the SKIP-clamp path; clamp happens on parity-odd. We predict clamp from the value model and
    // assert the binary's AH parity matches.
    let run_case = |e: &mut Emu, span: f32, ctx: &str| {
        e.write_u32(ebp + 8, span.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b9cf3, 0x7b9cfe); // fld [ebp+8]; fcomp [0x801360]; fnstsw ax; halt at `test ah,0x5`
        let ah = read_ah(e);
        // `test ah,0x5; jp` taken (SKIP clamp) ⇔ parity of (ah & 5) even.
        let bin_skips_clamp = (ah & 0x5).count_ones().is_multiple_of(2);
        // value model: skip clamp iff NOT (ordered AND span < eps). i.e. clamp iff span < eps.
        let ours_clamps = span < LIFESPAN_MIN; // Rust `<` is false for NaN → skip clamp (matches)
        assert_eq!(
            bin_skips_clamp, !ours_clamps,
            "{ctx}: binary skips-clamp={bin_skips_clamp} ours-clamps={ours_clamps} (span={span} ah={ah:#04x})"
        );
    };

    // ==eps boundary: span==eps must KEEP span (skip clamp); just-below must clamp.
    run_case(&mut e, LIFESPAN_MIN, "gate span==eps (keep)");
    run_case(
        &mut e,
        f32::from_bits(LIFESPAN_MIN.to_bits() - 1),
        "gate span just-below-eps (clamp)",
    );
    run_case(
        &mut e,
        f32::from_bits(LIFESPAN_MIN.to_bits() + 1),
        "gate span just-above-eps (keep)",
    );
    run_case(&mut e, 0.0, "gate span=0 (clamp)");
    run_case(&mut e, -1.0, "gate span<0 (clamp)");
    run_case(&mut e, 1.0, "gate span=1 (keep)");
    run_case(&mut e, f32::NAN, "gate span=NaN (keep)");

    let mut s = 0x7b9c_f3f3u32;
    for i in 0..4000u32 {
        let span = match (next(&mut s) >> 8) % 3 {
            0 => frand(&mut s, 0.0, 0.002), // straddles eps
            1 => frand(&mut s, -5.0, 5.0),
            _ => frand(&mut s, 0.0, 1.0e3),
        };
        run_case(&mut e, span, &format!("gate fuzz[{i}] span={span}"));
    }
}

/// `0x7b9e20` whole.
#[test]
fn gradient_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);
    let this = e.alloc(0x200); // CRibbonEmitter (lo @ +0x24, delta @ +0x28, hi @ +0x4c)

    let run_case = |e: &mut Emu, lo: f32, hi: f32, ctx: &str| {
        for off in [0x24u64, 0x28, 0x4c] {
            e.write_u32(this + off, 0xdead_beef); // junk-seed the three output fields
        }
        // __thiscall: ecx = this, stack args = (lo, hi). ret 8.
        e.call(0x7b9e20, this as u32, 0, &[lo.to_bits(), hi.to_bits()]);
        let bin = GradientRange {
            at_24: e.read_f32(this + 0x24),
            at_4c: e.read_f32(this + 0x4c),
            at_28: e.read_f32(this + 0x28),
        };
        let ours = gradient_range(lo, hi);
        assert_f32(bin.at_24, ours.at_24, &format!("{ctx} +0x24 (lo)"));
        assert_f32(bin.at_4c, ours.at_4c, &format!("{ctx} +0x4c (hi)"));
        assert_f32(bin.at_28, ours.at_28, &format!("{ctx} +0x28 (hi−lo)"));
    };

    // ==/edge probes: equal (delta 0), reversed (negative delta), zero, signed-zero, huge.
    run_case(&mut e, 0.0, 0.0, "grad zero");
    run_case(&mut e, 2.0, 5.0, "grad ordinary"); // delta 3
    run_case(&mut e, 5.0, 2.0, "grad reversed"); // delta -3
    run_case(&mut e, 3.0, 3.0, "grad equal"); // delta 0
    run_case(&mut e, -0.0, 0.0, "grad signed-zero"); // delta +0.0
    run_case(&mut e, -1e30, 1e30, "grad huge"); // overflow → +inf
    run_case(&mut e, f32::NAN, 1.0, "grad NaN lo");
    let mut s = 0x7b9e_2020u32;
    for i in 0..6000u32 {
        let lo = fuzz_range(&mut s);
        let hi = fuzz_range(&mut s);
        run_case(&mut e, lo, hi, &format!("grad fuzz_range[{i}]"));
    }
}
