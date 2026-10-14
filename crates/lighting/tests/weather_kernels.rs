//! The weather leaves against the exe: the particle clock `0x6745d0` and `0x674600`, the clamped
//! lerp `0x674ba0`, the fade ramp `0x67ad60`, `0x67ad90` and `0x67add0`, the intensity ramp
//! `0x67bc70` and the rain tilt `0x674a70`. Each runs the function's own instructions in the
//! emulator and compares bit for bit. The tilt runs the real `0x7be730` and its cmath callees; the
//! intensity ramp stubs the timer, the three effect setters and the recompute, and checks the
//! argument each live setter receives.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::weather_kernels::{
    weather_clamped_lerp, weather_fixed_quantize, weather_intensity_ramp, weather_ramp_eval,
    weather_ramp_eval_inv, weather_ramp_setup, weather_tick_scale, weather_wind_tilt,
    IntensityRampResult, WeatherManagerIntensity, WeatherWind,
};
use common::{assert_f32, fuzz_f32, next};

/// `0x6745d0` whole: edge values, NaN, an exact odd number of ticks (the tie that rounds down),
/// infinities and values beyond `i32` (which store `0x80000000`), and a fuzz.
#[test]
fn weather_fixed_quantize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x7177_6e64u32;
    let edges: [f32; 30] = [
        1.0 / 1024.0,
        3.0 / 1024.0,
        5.0 / 1024.0,
        1001.0 / 1024.0,
        1_048_577.0 / 1024.0,
        -1.0 / 1024.0,
        -3.0 / 1024.0,
        2_097_152.0,
        2_097_151.0,
        -2_097_152.0,
        -2_097_153.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN_POSITIVE,
        1.5 / 1024.0,
        2.5 / 1024.0,
        0.25 / 1024.0,
        0.0,
        -0.0,
        0.5,
        -0.5,
        1.0,
        -1.0,
        0.001,
        1000.0,
        -1000.0,
        2_200_000.0,
        -2_200_000.0,
        f32::NAN,
    ];
    for (i, &x) in edges.iter().enumerate() {
        let binary = emu.call(0x6745d0, 0, 0, &[x.to_bits()]) as i32;
        let ours = weather_fixed_quantize(x);
        assert_eq!(binary, ours, "edge {i} x={x}");
    }
    for case in 0..600 {
        let x = fuzz_f32(&mut s);
        let binary = emu.call(0x6745d0, 0, 0, &[x.to_bits()]) as i32;
        let ours = weather_fixed_quantize(x);
        assert_eq!(binary, ours, "case {case} x={x} bits={:#010x}", x.to_bits());
    }
}

/// `0x674600` whole: tick counts around the unsigned range and a fuzz.
#[test]
fn weather_tick_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edges: [u32; 6] = [0, 1, 1024, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff];
    let mut s = 0x7469_636bu32;
    for (i, &t) in edges.iter().enumerate() {
        let binary = emu.call_f32(0x674600, t, 0, &[]);
        let ours = weather_tick_scale(t);
        assert_f32(binary, ours, &format!("edge {i} t={t:#x}"));
    }
    for case in 0..400 {
        let t = next(&mut s);
        let binary = emu.call_f32(0x674600, t, 0, &[]);
        let ours = weather_tick_scale(t);
        assert_f32(binary, ours, &format!("case {case} t={t:#x}"));
    }
}

/// `0x674ba0` whole: `t` below 0, -0, 0, inside, exactly 1, above 1, NaN and infinities, and a
/// fuzz.
#[test]
fn weather_clamped_lerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x6c65_7270u32;
    let edges: [(f32, f32, f32); 15] = [
        (1.0, 5.0, -0.0),
        (1.0, 5.0, f32::INFINITY),
        (1.0, 5.0, f32::NEG_INFINITY),
        (f32::NAN, 5.0, 0.5),
        (1.0, f32::NAN, 0.5),
        (f32::INFINITY, 5.0, 0.0),
        (1.0, 5.0, f32::from_bits(0x3f7f_ffff)),
        (1.0, 5.0, f32::from_bits(0x3f80_0001)),
        (1.0, 5.0, -0.5),
        (1.0, 5.0, 0.0),
        (1.0, 5.0, 0.5),
        (1.0, 5.0, 1.0),
        (1.0, 5.0, 2.0),
        (1.0, 5.0, f32::NAN),
        (-3.0, 7.0, 0.25),
    ];
    for (i, &(a, b, t)) in edges.iter().enumerate() {
        let binary = emu.call_f32(0x674ba0, 0, 0, &[a.to_bits(), b.to_bits(), t.to_bits()]);
        let ours = weather_clamped_lerp(a, b, t);
        assert_f32(binary, ours, &format!("edge {i} a={a} b={b} t={t}"));
    }
    for case in 0..500 {
        let a = fuzz_f32(&mut s);
        let b = fuzz_f32(&mut s);
        let t = match (next(&mut s) >> 8) % 4 {
            0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
            1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 0.5) as f32,
            2 => fuzz_f32(&mut s),
            _ => f32::from_bits(next(&mut s)),
        };
        let binary = emu.call_f32(0x674ba0, 0, 0, &[a.to_bits(), b.to_bits(), t.to_bits()]);
        let ours = weather_clamped_lerp(a, b, t);
        assert_f32(binary, ours, &format!("case {case} a={a} b={b} t={t}"));
    }
}

/// `0x67ad60` whole: edges, an empty span, NaN and a fuzz.
#[test]
fn weather_ramp_setup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out = emu.alloc(8);
    let mut s = 0x7261_6d70u32;
    let edges: [(f32, f32); 8] = [
        (0.0, 1.0),
        (10.0, 20.0),
        (5.0, 5.0),
        (-2.0, 3.0),
        (f32::NAN, 1.0),
        (0.0, f32::NAN),
        (0.0, f32::INFINITY),
        (f32::INFINITY, f32::INFINITY),
    ];
    let run = |emu: &mut Emu, lo: f32, hi: f32, ctx: &str| {
        emu.write_u32(out, 0);
        emu.write_u32(out + 4, 0);
        emu.call(0x67ad60, out as u32, 0, &[lo.to_bits(), hi.to_bits()]);
        let ours = weather_ramp_setup(lo, hi);
        assert_f32(emu.read_f32(out), ours.slope, &format!("{ctx} slope"));
        assert_f32(
            emu.read_f32(out + 4),
            ours.intercept,
            &format!("{ctx} intercept"),
        );
    };
    for (i, &(lo, hi)) in edges.iter().enumerate() {
        run(&mut emu, lo, hi, &format!("edge {i}"));
    }
    for case in 0..400 {
        let lo = fuzz_f32(&mut s);
        let hi = fuzz_f32(&mut s);
        run(&mut emu, lo, hi, &format!("case {case} lo={lo} hi={hi}"));
    }
}

/// `0x67ad90` whole, over ramps from a fuzz and `x` inside and outside them, NaN included.
#[test]
fn weather_ramp_eval_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let coeffs = emu.alloc(8);
    let mut s = 0x6576_616cu32;
    for case in 0..500 {
        let lo = fuzz_f32(&mut s);
        let hi = fuzz_f32(&mut s);
        let c = weather_ramp_setup(lo, hi);
        emu.write_u32(coeffs, c.slope.to_bits());
        emu.write_u32(coeffs + 4, c.intercept.to_bits());
        let x = match (next(&mut s) >> 8) % 3 {
            0 => {
                let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                (f64::from(lo) + m * (f64::from(hi) - f64::from(lo)) * 1.6 - 0.3) as f32
            }
            1 => fuzz_f32(&mut s),
            _ => f32::from_bits(next(&mut s)),
        };
        let b_fwd = emu.call_f32(0x67ad90, coeffs as u32, 0, &[x.to_bits()]);
        assert_f32(
            b_fwd,
            weather_ramp_eval(c, x),
            &format!("fwd case {case} x={x}"),
        );
    }
}

/// `0x67add0` whole, over the same ramps as the forward evaluator.
#[test]
fn weather_ramp_eval_inv_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let coeffs = emu.alloc(8);
    let mut s = 0x6576_616cu32;
    for case in 0..500 {
        let lo = fuzz_f32(&mut s);
        let hi = fuzz_f32(&mut s);
        let c = weather_ramp_setup(lo, hi);
        emu.write_u32(coeffs, c.slope.to_bits());
        emu.write_u32(coeffs + 4, c.intercept.to_bits());
        let x = match (next(&mut s) >> 8) % 3 {
            0 => {
                let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                (f64::from(lo) + m * (f64::from(hi) - f64::from(lo)) * 1.6 - 0.3) as f32
            }
            1 => fuzz_f32(&mut s),
            _ => f32::from_bits(next(&mut s)),
        };
        let b_inv = emu.call_f32(0x67add0, coeffs as u32, 0, &[x.to_bits()]);
        assert_f32(
            b_inv,
            weather_ramp_eval_inv(c, x),
            &format!("inv case {case} x={x}"),
        );
    }
}

/// `0x67bc70` whole, 600 managers: the grade and density stored, the light state's `+0x40`, the
/// cleared dirty byte, and the argument of each live effect setter (and none for a dead slot);
/// grades in and out of [0, 1], and NaN grade endpoints, which the exe passes on as NaN.
#[test]
fn weather_intensity_ramp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let dnstate: u64 = 0xce9b60;
    let intensity_off: u64 = 0x40;

    let mgr = emu.alloc(0x90);
    const NOW: u32 = 0x0010_0000;
    emu.stub(0x42c010, NOW, 0);
    emu.stub(0x6749e0, 0, 1);
    emu.stub(0x6776c0, 0, 1);
    emu.stub(0x679380, 0, 1);
    emu.stub(0x6d2210, 0, 0);

    let mut s = 0x696e_7472u32;

    #[allow(clippy::too_many_arguments)]
    fn write_mgr(emu: &mut Emu, mgr: u64, m: &WeatherManagerIntensity) {
        emu.write_u32(mgr + 0x08, m.chan_a_out.to_bits());
        emu.write_u32(mgr + 0x0c, m.chan_b_from.to_bits());
        emu.write_u32(mgr + 0x10, m.chan_b_to.to_bits());
        emu.write_u32(mgr + 0x14, m.chan_b_out.to_bits());
        emu.write_u32(mgr + 0x18, m.start_a);
        emu.write_u32(mgr + 0x1c, m.start_b);
        emu.write_u32(mgr + 0x24, 0x0101_0101);
        emu.write_u32(mgr + 0x28, m.effect_rain);
        emu.write_u32(mgr + 0x2c, m.effect_snow);
        emu.write_u32(mgr + 0x30, m.effect_sand);
    }

    for case in 0..600 {
        let bias_unit = |s: &mut u32| (f64::from(next(s) >> 8) / f64::from(1u32 << 24)) as f32;
        let arg_from = if case % 5 == 0 {
            fuzz_f32(&mut s)
        } else {
            bias_unit(&mut s)
        };
        let arg_to = if case % 5 == 0 {
            fuzz_f32(&mut s)
        } else {
            bias_unit(&mut s)
        };
        let special: [(f32, f32); 8] = [
            (f32::NAN, 1.0),
            (0.5, f32::NAN),
            (f32::INFINITY, 0.0),
            (0.0, f32::INFINITY),
            (0.25, 0.25),
            (0.25, 1.0),
            (-0.0, 0.0),
            (1.0, 1.0),
        ];
        let (arg_from, arg_to) = if case < 8 {
            special[case]
        } else {
            (arg_from, arg_to)
        };
        let live = (next(&mut s) >> 8) % 8;
        let m = WeatherManagerIntensity {
            chan_a_out: 0.0,
            chan_b_from: bias_unit(&mut s),
            chan_b_to: bias_unit(&mut s),
            chan_b_out: 0.0,
            start_a: NOW.wrapping_sub((next(&mut s) >> 8) % 4_000_000),
            start_b: NOW.wrapping_sub((next(&mut s) >> 8) % 4_000_000),
            effect_rain: if live & 1 != 0 { 0x4000_0000 } else { 0 },
            effect_snow: if live & 2 != 0 { 0x4000_0010 } else { 0 },
            effect_sand: if live & 4 != 0 { 0x4000_0020 } else { 0 },
        };
        write_mgr(&mut emu, mgr, &m);
        emu.write_u32(dnstate + intensity_off, 0xdead_beef);
        emu.clear_calls();

        emu.call(
            0x67bc70,
            mgr as u32,
            0,
            &[arg_from.to_bits(), arg_to.to_bits()],
        );

        let ours: IntensityRampResult = weather_intensity_ramp(&m, NOW, arg_from, arg_to);

        let bin_a = emu.read_f32(mgr + 0x08);
        let bin_b = emu.read_f32(mgr + 0x14);
        let bin_dn = emu.read_f32(dnstate + intensity_off);
        assert_f32(bin_a, ours.chan_a_out, &format!("chan_a_out case {case}"));
        assert_f32(bin_b, ours.chan_b_out, &format!("chan_b_out case {case}"));
        assert_f32(
            bin_dn,
            ours.dnstate_intensity,
            &format!("DNState+0x40 case {case}"),
        );
        assert_eq!(
            bin_dn.to_bits(),
            bin_b.to_bits(),
            "DNState+0x40 must equal manager+0x14 (case {case})"
        );

        assert_eq!(
            emu.read_u32(mgr + 0x24) & 0xff,
            0,
            "dirty byte +0x24 cleared (case {case})"
        );

        for (slot_val, addr) in [
            (m.effect_rain, 0x6749e0u64),
            (m.effect_snow, 0x6776c0),
            (m.effect_sand, 0x679380),
        ] {
            let rec = emu.calls().iter().find(|c| c.addr == addr);
            if slot_val != 0 {
                let rec =
                    rec.unwrap_or_else(|| panic!("setter {addr:#x} not called (case {case})"));
                let got = f32::from_bits(rec.args[0]);
                assert_f32(
                    got,
                    ours.effect_arg,
                    &format!("effect_arg @ {addr:#x} case {case}"),
                );
            } else {
                assert!(
                    rec.is_none(),
                    "setter {addr:#x} called on dead slot (case {case})"
                );
            }
        }
    }
}

/// `0x674a70` whole, with the real `0x7be730`: winds under and over the gates, large and negative
/// winds, a non-identity matrix, NaN and infinite winds, and a fuzz over the whole bit space.
#[test]
fn weather_wind_tilt_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    const MGR_GLOBAL: u64 = 0xc6326c;
    let mgr = emu.alloc(0x90);
    emu.write_u32(MGR_GLOBAL, mgr as u32);
    let mat = emu.alloc(0x24);

    let mut s = 0x7769_6e64_u32;

    let id = [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    let edges: [([f32; 9], WeatherWind); 10] = [
        (
            id,
            WeatherWind {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        ),
        (
            id,
            WeatherWind {
                x: 0.01,
                y: 0.0,
                z: 0.0,
            },
        ),
        (
            id,
            WeatherWind {
                x: 0.032,
                y: 0.0,
                z: 0.0,
            },
        ),
        (
            id,
            WeatherWind {
                x: 5.0,
                y: 3.0,
                z: 9.0,
            },
        ),
        (
            id,
            WeatherWind {
                x: 30.0,
                y: 40.0,
                z: 0.0,
            },
        ),
        (
            id,
            WeatherWind {
                x: -7.0,
                y: 2.5,
                z: -1.0,
            },
        ),
        (
            id,
            WeatherWind {
                x: 100.0,
                y: -200.0,
                z: 0.0,
            },
        ),
        (
            [0.5, -0.3, 0.8, 1.2, 0.1, -0.6, 0.0, 0.9, 0.4],
            WeatherWind {
                x: 12.0,
                y: -5.0,
                z: 2.0,
            },
        ),
        (
            id,
            WeatherWind {
                x: f32::NAN,
                y: 1.0,
                z: 0.0,
            },
        ),
        (
            id,
            WeatherWind {
                x: f32::INFINITY,
                y: 0.0,
                z: 0.0,
            },
        ),
    ];

    let run = |emu: &mut Emu, m_in: &[f32; 9], wind: WeatherWind, ctx: &str| {
        emu.write_u32(mgr + 0x50, wind.x.to_bits());
        emu.write_u32(mgr + 0x54, wind.y.to_bits());
        emu.write_u32(mgr + 0x58, wind.z.to_bits());
        for (i, v) in m_in.iter().enumerate() {
            emu.write_u32(mat + 4 * i as u64, v.to_bits());
        }
        emu.call(0x674a70, 0, 0, &[mat as u32]);
        let ours = weather_wind_tilt(wind, m_in);
        for i in 0..9 {
            assert_f32(
                emu.read_f32(mat + 4 * i as u64),
                ours.matrix[i],
                &format!("{ctx} M[{i}]"),
            );
        }
    };

    for (i, (m_in, wind)) in edges.iter().enumerate() {
        run(&mut emu, m_in, *wind, &format!("edge {i}"));
    }

    for case in 0..700 {
        let m_in: [f32; 9] = match (next(&mut s) >> 8) % 3 {
            0 | 1 => id,
            _ => {
                let mut m = [0.0f32; 9];
                for v in m.iter_mut() {
                    *v = fuzz_f32(&mut s);
                }
                m
            }
        };
        let wind = match (next(&mut s) >> 8) % 5 {
            0 => {
                let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                WeatherWind {
                    x: (m * 0.2 - 0.1) as f32,
                    y: (m * 0.05) as f32,
                    z: fuzz_f32(&mut s),
                }
            }
            1 => WeatherWind {
                x: (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 30.0) as f32,
                y: (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 30.0) as f32,
                z: fuzz_f32(&mut s),
            },
            2 => WeatherWind {
                x: fuzz_f32(&mut s) * 50.0,
                y: fuzz_f32(&mut s) * 50.0,
                z: fuzz_f32(&mut s),
            },
            3 => WeatherWind {
                x: fuzz_f32(&mut s),
                y: fuzz_f32(&mut s),
                z: fuzz_f32(&mut s),
            },
            _ => WeatherWind {
                x: f32::from_bits(next(&mut s)),
                y: f32::from_bits(next(&mut s)),
                z: f32::from_bits(next(&mut s)),
            },
        };
        run(
            &mut emu,
            &m_in,
            wind,
            &format!("case {case} wind=({},{},{})", wind.x, wind.y, wind.z),
        );
    }
}
