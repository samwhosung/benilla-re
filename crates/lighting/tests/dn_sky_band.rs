//! `UpdateDisc` (`0x6d1960`) and `BuildSkyDome` (`0x6d0d10`) whole, and the arithmetic of
//! `ColourSkyDome` (`0x6d0f50`) as slices, against `fog_plane_clip`, `sky_band_orient_setup` and
//! the slice functions of `dn_sky_band`. The slices run with the leaves they call (`EvalSkyTrack`
//! `0x6cf6c0` and `LerpChannel` `0x6d15f0`) in the emulator, and the wide track value the
//! transcriptions take is read from the same leaf with `call_f64`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::dn_sky_band::{
    fog_plane_clip, lerp_result_to_byte, sky_alpha_byte, sky_alpha_index, sky_alpha_neg,
    sky_alpha_one_minus, sky_angle_accum, sky_angle_wrap, sky_band_orient_setup, sky_row_angle,
    sky_row_step, sky_scale_init, ClipParam1, ClipParam2,
};
use common::{assert_f32, next};

fn fuzz_depth(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 8;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 4.0 - 2.0) as f32,
        1 => (m * 0.8 - 0.4) as f32,
        2 => (m * 0.002) as f32,
        3 => (m * 1000.0) as f32,
        4 => (-(m * 1000.0)) as f32,
        5 => f32::from_bits(next(s)),
        6 => 0.0,
        _ => (m * 0.5) as f32,
    }
}

const G_CE9B80: u64 = 0x00ce_9b80;

/// `0x6d1960` with the records and the fog depth written and every output poisoned. Both endpoints
/// in front, both behind, a straddle, the second plane crossed, `g1` and the ramp `r` at their
/// thresholds, a NaN depth and vertex, a fog offset, and a fuzz.
#[test]
fn fog_plane_clip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let strip = emu.alloc(0x80);
    let p1 = emu.alloc(0x100);
    let p2 = emu.alloc(0x80);
    let p3 = emu.alloc(0x100);
    let p4 = emu.alloc(0x20);
    let p5 = emu.alloc(0x10);
    let p6 = emu.alloc(0x10);

    let mut s = 0x66_6f67_u32;
    let flag6_poison = 0xc0de_6666u32;

    let run =
        |emu: &mut Emu, depth: f32, verts: [f32; 6], p04: f32, p14: f32, fog: f32, tag: &str| {
            let p1_in = ClipParam1 {
                v: [
                    verts[0],
                    verts[1],
                    verts[2],
                    f32::from_bits(0xdead_0003),
                    f32::from_bits(0xdead_0004),
                    f32::from_bits(0xdead_0005),
                ],
            };
            let p2_in = ClipParam2 {
                p04,
                p14,
                p1c: f32::from_bits(0xdead_0011),
                p24: f32::from_bits(0xdead_0012),
                p2c: f32::from_bits(0xdead_0013),
            };
            emu.write_u32(strip + 8, depth.to_bits());
            for (i, &v) in p1_in.v.iter().enumerate() {
                emu.write_u32(p1 + 8 + 0xc * i as u64, v.to_bits());
            }
            emu.write_u32(p2 + 4, p2_in.p04.to_bits());
            emu.write_u32(p2 + 0x14, p2_in.p14.to_bits());
            emu.write_u32(p2 + 0x1c, p2_in.p1c.to_bits());
            emu.write_u32(p2 + 0x24, p2_in.p24.to_bits());
            emu.write_u32(p2 + 0x2c, p2_in.p2c.to_bits());
            for i in 0..8u64 {
                emu.write_u32(p3 + 4 * i, 0xa5a5_a5a5);
            }
            for i in 0..3u64 {
                emu.write_u32(p4 + 4 * i, 0xbeef_0000 + i as u32);
            }
            emu.write_u32(p5, 0x1111_2222);
            emu.write_u32(p6, flag6_poison);
            emu.write_u32(G_CE9B80, fog.to_bits());

            emu.call(
                0x6d1960,
                strip as u32,
                0,
                &[
                    p1 as u32, p2 as u32, p3 as u32, p4 as u32, p5 as u32, p6 as u32,
                ],
            );

            let bin_flag = emu.read_u32(p5);
            let bin_flag6 = emu.read_u32(p6);
            let mut bin_v = [0f32; 6];
            for (i, vv) in bin_v.iter_mut().enumerate() {
                *vv = emu.read_f32(p1 + 8 + 0xc * i as u64);
            }
            let bin_p2 = ClipParam2 {
                p04: p2_in.p04,
                p14: emu.read_f32(p2 + 0x14),
                p1c: emu.read_f32(p2 + 0x1c),
                p24: emu.read_f32(p2 + 0x24),
                p2c: emu.read_f32(p2 + 0x2c),
            };

            let ours = fog_plane_clip(depth, p1_in, p2_in, fog, flag6_poison);

            assert_eq!(bin_flag, ours.flag, "{tag}: flag");
            for (i, &bv) in bin_v.iter().enumerate() {
                assert_f32(bv, ours.p1.v[i], &format!("{tag}: v[{i}]"));
            }
            assert_f32(bin_p2.p14, ours.p2.p14, &format!("{tag}: p2.p14"));
            assert_f32(bin_p2.p1c, ours.p2.p1c, &format!("{tag}: p2.p1c"));
            assert_f32(bin_p2.p24, ours.p2.p24, &format!("{tag}: p2.p24"));
            assert_f32(bin_p2.p2c, ours.p2.p2c, &format!("{tag}: p2.p2c"));

            match ours.flag6 {
                Some(v) => assert_eq!(bin_flag6, v, "{tag}: flag6 (crossing)"),
                None => assert_eq!(bin_flag6, flag6_poison, "{tag}: flag6 (unchanged)"),
            }
            let bin_edge = [emu.read_u32(p4), emu.read_u32(p4 + 4), emu.read_u32(p4 + 8)];
            match ours.edge_idx {
                Some(e) => assert_eq!(bin_edge, e, "{tag}: edge_idx (written)"),
                None => assert_eq!(
                    bin_edge,
                    [0xbeef_0000, 0xbeef_0001, 0xbeef_0002],
                    "{tag}: edge_idx (unchanged)"
                ),
            }
            for (i, slot) in ours.ramp.iter().enumerate() {
                let raw = emu.read(p3 + 3 + 4 * i as u64, 1)[0];
                match slot {
                    Some(b) => assert_eq!(raw, *b, "{tag}: ramp[{i}]"),
                    None => assert_eq!(raw, 0xa5, "{tag}: ramp[{i}] (skipped/untouched)"),
                }
            }
        };

    run(
        &mut emu,
        0.2,
        [0.1, 0.2, 0.5, 0.3, 0.15, 0.25],
        0.1,
        0.9,
        0.0,
        "both-front",
    );
    run(
        &mut emu,
        -2.0,
        [0.5, 0.2, 1.0, 0.3, 0.15, 0.25],
        0.1,
        0.9,
        0.0,
        "both-back",
    );
    run(
        &mut emu,
        0.2,
        [0.1, 0.2, -0.6, 0.3, 0.15, 0.25],
        0.1,
        0.9,
        0.0,
        "z0-straddle",
    );
    run(
        &mut emu,
        0.45,
        [0.1, 0.2, -0.05, 0.3, 0.15, 0.25],
        0.1,
        0.9,
        0.0,
        "plane-cross",
    );
    run(
        &mut emu,
        0.4,
        [0.2, 0.1, 0.0, 0.05, 0.3, 0.25],
        0.1,
        0.9,
        0.0,
        "plane-eq",
    );
    let eps_v = f32::from_bits(0x3a83_126f) + 0.4;
    run(
        &mut emu,
        0.0,
        [0.45, eps_v, 0.3, 0.4001, 0.39, 0.41],
        0.1,
        0.9,
        0.0,
        "ramp-eps",
    );
    run(
        &mut emu,
        f32::NAN,
        [0.3, 0.2, 0.1, 0.0, 0.05, 0.15],
        0.1,
        0.9,
        0.0,
        "nan-depth",
    );
    run(
        &mut emu,
        0.3,
        [f32::NAN, 0.2, 0.1, 0.0, 0.05, 0.15],
        0.1,
        0.9,
        0.0,
        "nan-v0",
    );
    run(
        &mut emu,
        0.9,
        [0.1, 0.2, -0.3, 0.3, 0.15, 0.25],
        0.2,
        0.8,
        0.5,
        "fog-shift",
    );

    for _ in 0..400 {
        let mut verts = [0f32; 6];
        for v in verts.iter_mut() {
            *v = fuzz_depth(&mut s);
        }
        run(
            &mut emu,
            fuzz_depth(&mut s),
            verts,
            fuzz_depth(&mut s),
            fuzz_depth(&mut s),
            fuzz_depth(&mut s),
            "fuzz",
        );
    }
}

const G_CE99D0: u64 = 0x00ce_99d0;
const G_CE9B90: u64 = 0x00ce_9b90;

/// `0x6d0fa0` to `0x6d0fcc` with `g` at 0 and 1 and across the range: the stored `f32` and its
/// byte.
#[test]
fn sky_alpha_index_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let mut s = 0x616c_7068_u32;

    let run = |emu: &mut Emu, g: f32| {
        emu.write_u32(G_CE99D0, g.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.run(0x6d0fa0, 0x6d0fcc);
        let binary = emu.read_f32(frame - 0x2c);
        let ours = sky_alpha_index(g);
        assert_f32(binary, ours, &format!("g={g} ({:#010x})", g.to_bits()));
        assert_eq!(
            sky_alpha_byte(binary),
            sky_alpha_byte(ours),
            "g={g}: alpha byte"
        );
    };

    let edges = [
        0.0f32,
        1.0,
        -0.0,
        0.5,
        0.25,
        0.999,
        1.001,
        -1.0,
        2.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x0000_0001),
    ];
    for &g in &edges {
        run(&mut emu, g);
    }
    for _ in 0..400 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(&mut emu, (m * 2.0 - 0.5) as f32);
    }
}

/// `0x6d10fa` to `0x6d110e` with counts of both signs, 0 (an infinite step), and large.
#[test]
fn sky_row_step_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let mut s = 0x7374_6570_u32;

    let run = |emu: &mut Emu, count: i32| {
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.write_u32(frame - 0x2c, count as u32);
        emu.run(0x6d10fa, 0x6d1111);
        let binary = emu.read_f32(frame - 0x5c);
        let ours = sky_row_step(count);
        assert_f32(binary, ours, &format!("count={count}"));
    };

    for c in [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        24,
        25,
        100,
        360,
        -1,
        -4,
        i32::MAX,
        i32::MIN,
    ] {
        run(&mut emu, c);
    }
    for _ in 0..200 {
        run(&mut emu, (next(&mut s) >> 16) as i32);
    }
}

/// `0x6d1111` to `0x6d113f` with the heading around `g = 1.5π`, where the wide `a` is just above
/// and below 1 while the stored `f32` is 1.
#[test]
fn sky_row_angle_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let mut s = 0x616e_676c_u32;

    let run = |emu: &mut Emu, g: f32| {
        emu.write_u32(G_CE9B90, g.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.run(0x6d1111, 0x6d113f);
        let binary = emu.read_f32(frame - 0x10);
        let ours = sky_row_angle(g);
        assert_f32(
            binary,
            ours.angle,
            &format!("g={g} ({:#010x})", g.to_bits()),
        );
    };

    let g_at_one = 1.5 * std::f32::consts::PI;
    let edges = [
        0.0f32,
        g_at_one,
        f32::from_bits(g_at_one.to_bits() + 1),
        f32::from_bits(g_at_one.to_bits() - 1),
        4.712,
        4.713,
        -10.0,
        100.0,
        -100.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        6.2831855,
    ];
    for &g in &edges {
        run(&mut emu, g);
    }
    for _ in 0..400 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(&mut emu, (m * 20.0 - 10.0) as f32);
    }
}

/// `0x6d1150` to `0x6d116c` with angles of both signs, both zeros, NaN and the infinities.
#[test]
fn sky_angle_wrap_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let mut s = 0x7772_6170_u32;

    let run = |emu: &mut Emu, a: f32| {
        emu.write_u32(frame - 0x10, a.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.run(0x6d1150, 0x6d116c);
        let binary = emu.read_f32(frame - 0x10);
        let ours = sky_angle_wrap(a);
        assert_f32(binary, ours, &format!("a={a} ({:#010x})", a.to_bits()));
    };

    let edges = [
        0.0f32,
        -0.0,
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0000_0001),
        -0.5,
        0.5,
        -1.0,
        1.0,
        -0.0001,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &a in &edges {
        run(&mut emu, a);
    }
    for _ in 0..300 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(&mut emu, (m * 4.0 - 2.0) as f32);
    }
}

/// `0x6d1484` to `0x6d1497` with edge pairs and a fuzz.
#[test]
fn sky_angle_accum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let mut s = 0x6163_636d_u32;

    let run = |emu: &mut Emu, a: f32, step: f32| {
        emu.write_u32(frame - 0x10, a.to_bits());
        emu.write_u32(frame - 0x5c, step.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.write_u32(frame - 0x28, frame as u32);
        emu.run(0x6d1484, 0x6d149a);
        let binary = emu.read_f32(frame - 0x10);
        let ours = sky_angle_accum(a, step);
        assert_f32(binary, ours, &format!("a={a} step={step}"));
    };

    let edges = [
        (0.0f32, 0.0f32),
        (1.0, -1.0),
        (0.25, -0.16666667),
        (-0.0, 0.0),
        (1.0e30, 1.0e30),
        (f32::NAN, 1.0),
        (1.0, f32::INFINITY),
    ];
    for &(a, st) in &edges {
        run(&mut emu, a, st);
    }
    for _ in 0..300 {
        let ma = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let mb = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(&mut emu, (ma * 2.0 - 1.0) as f32, (mb * 0.4 - 0.2) as f32);
    }
}

/// `0x6d0d10` whole, with the three vectors sized so the grow calls are skipped: the seven band
/// records, the coordinate ring and the two trailing words are compared.
#[test]
fn sky_band_orient_setup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let this = emu.alloc(0x40);
    let v1 = emu.alloc(0xa8 * 0xc + 0x40);
    let v2 = emu.alloc(0xa8 * 4 + 0x40);
    let v3 = emu.alloc(0x12c * 8 + 0x80);

    let mut s = 0x6261_6e64_u32;

    let run = |emu: &mut Emu, arg: f32| {
        emu.write_u32(this + 4, 0xa8);
        emu.write_u32(this + 8, v1 as u32);
        emu.write_u32(this + 0x10, 0xa8);
        emu.write_u32(this + 0x14, v2 as u32);
        emu.write_u32(this + 0x1c, 0x12c);
        emu.write_u32(this + 0x20, v3 as u32);
        for i in 0..(0xa8 * 0xc / 4) as u64 {
            emu.write_u32(v1 + 4 * i, 0);
        }
        for i in 0..(0x12c * 8 / 4) as u64 {
            emu.write_u32(v3 + 4 * i, 0);
        }
        emu.write_u32(this + 0x28, 0xdead_beef);

        emu.call(0x6d0d10, this as u32, 0, &[arg.to_bits()]);

        let ours = sky_band_orient_setup(arg, v1 as u32);

        for (b, rec) in ours.bands.iter().enumerate() {
            for (k, &val) in rec.iter().enumerate() {
                let off = v1 + (b as u64) * 0xc + (k as u64) * 4;
                let binv = emu.read_f32(off);
                assert_f32(binv, val, &format!("arg={arg}: band[{b}][{k}]"));
            }
        }
        for (c, &(x, y)) in ours.coords.iter().enumerate() {
            let bx = u16::from_le_bytes([
                emu.read(v3 + (c as u64) * 4, 1)[0],
                emu.read(v3 + (c as u64) * 4 + 1, 1)[0],
            ]);
            let by = u16::from_le_bytes([
                emu.read(v3 + (c as u64) * 4 + 2, 1)[0],
                emu.read(v3 + (c as u64) * 4 + 3, 1)[0],
            ]);
            assert_eq!(bx, x, "arg={arg}: coord[{c}].x");
            assert_eq!(by, y, "arg={arg}: coord[{c}].y");
        }
        let bin_cw =
            u16::from_le_bytes([emu.read(this + 0x28, 1)[0], emu.read(this + 0x28 + 1, 1)[0]]);
        let bin_tw =
            u16::from_le_bytes([emu.read(this + 0x2a, 1)[0], emu.read(this + 0x2a + 1, 1)[0]]);
        assert_eq!(bin_cw, ours.count_word, "arg={arg}: count_word");
        assert_eq!(bin_tw, ours.tail_word, "arg={arg}: tail_word");
    };

    let edges = [1.0f32, 0.5, 2.0, 100.0, 0.0, -1.0, 256.0, 0.001];
    for &a in &edges {
        run(&mut emu, a);
    }
    for _ in 0..50 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(&mut emu, (m * 512.0) as f32);
    }
}

const G_CE9B2C: u64 = 0x00ce_9b2c;
const G_CE9B64: u64 = 0x00ce_9b64;
const G_CE9C5C: u64 = 0x00ce_9c5c;
const G_CE9AF8: u64 = 0x00ce_9af8;
const G_CE9B90B: u64 = 0x00ce_9b90;
const G_CE9BA4: u64 = 0x00ce_9ba4;

fn write_track(emu: &mut Emu, base: u64, pairs: &[(f32, f32)]) {
    for (i, &(t, v)) in pairs.iter().enumerate() {
        emu.write_u32(base + 8 * i as u64, t.to_bits());
        emu.write_u32(base + 8 * i as u64 + 4, v.to_bits());
    }
}

fn track6(value_scale: f32) -> [(f32, f32); 6] {
    [
        (0.0, 0.0 * value_scale),
        (0.2, 0.3 * value_scale),
        (0.4, 0.55 * value_scale),
        (0.6, 0.8 * value_scale),
        (0.8, 0.95 * value_scale),
        (1.0, 1.0 * value_scale),
    ]
}

/// `0x6d0f6b` to `0x6d0f90`: the wide return of `EvalSkyTrack` on the track at `0xce9b2c` times
/// `[0xce9c5c]`, over times below, at and above the keys.
#[test]
fn sky_scale_init_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let mut s = 0x7363_6c69_u32;

    let run = |emu: &mut Emu, value_scale: f32, time: f32, k: f32| {
        write_track(emu, G_CE9B2C, &track6(value_scale));
        emu.write_u32(G_CE9B64, time.to_bits());
        emu.write_u32(G_CE9C5C, k.to_bits());
        let t0 = emu.call_f64(0x6cf6c0, 0x00ce_9b2c, 6, &[time.to_bits()]);
        write_track(emu, G_CE9B2C, &track6(value_scale));
        emu.write_u32(G_CE9B64, time.to_bits());
        emu.write_u32(G_CE9C5C, k.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.run(0x6d0f6b, 0x6d0f90);
        let binary = emu.read_f32(frame - 0x20);
        let ours = sky_scale_init(t0, k);
        assert_f32(binary, ours, &format!("vs={value_scale} t={time} k={k}"));
    };

    let edges = [
        (1.0f32, 0.5f32, 1.0f32),
        (1.0, 0.0, 2.0),
        (1.0, 1.0, 0.5),
        (-1.0, 0.5, 1.0),
        (1.0, -0.5, 1.0),
        (1.0, 1.5, 1.0),
        (3.0, 0.37, -1.0),
        (1.0, 0.5, 0.0),
        (0.0, 0.5, 1.0),
    ];
    for &(vs, t, k) in &edges {
        run(&mut emu, vs, t, k);
    }
    for _ in 0..200 {
        let m1 = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let m2 = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let m3 = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(
            &mut emu,
            (m1 * 4.0 - 2.0) as f32,
            (m2 * 1.4 - 0.2) as f32,
            (m3 * 4.0 - 2.0) as f32,
        );
    }
}

/// `0x6d101d` to `0x6d1029`, the `fld; fsub 0.5; fistp` the 13 sites share (each site is checked to
/// hold the same `fsub` opcode): floats exactly on `n + 0.5` and `n`, so the tie goes to even, and
/// a fuzz.
#[test]
fn lerp_result_to_byte_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let mut s = 0x6c72_7062_u32;

    const FSUB_HALF: [u8; 6] = [0xd8, 0x25, 0xc8, 0xf2, 0x86, 0x00];
    for site in [
        0x6d1020u64,
        0x6d1059,
        0x6d1092,
        0x6d11d1,
        0x6d1208,
        0x6d123f,
        0x6d12b1,
        0x6d1303,
        0x6d133a,
        0x6d1371,
        0x6d13c7,
        0x6d13fb,
        0x6d142f,
    ] {
        let bytes = emu.read(site, 6);
        assert_eq!(
            bytes, FSUB_HALF,
            "clone-family fsub byte mismatch at {site:#x}"
        );
    }

    let run = |emu: &mut Emu, lerp_f32: f32| {
        emu.write_u32(frame - 0x2c, lerp_f32.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.run(0x6d101d, 0x6d1029);
        let binary = emu.read_u32(frame - 0x24) as i32;
        let ours = lerp_result_to_byte(lerp_f32);
        assert_eq!(
            binary,
            ours,
            "lerp_f32={lerp_f32} ({:#010x})",
            lerp_f32.to_bits()
        );
    };

    let edges = [
        0.0f32,
        0.5,
        1.0,
        1.5,
        2.0,
        2.5,
        3.5,
        4.5,
        255.5,
        255.0,
        0.4999999,
        0.5000001,
        -0.5,
        -1.5,
        127.5,
        128.5,
        2_147_483_648.0,
        -2_147_483_648.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &v in &edges {
        run(&mut emu, v);
    }
    for _ in 0..400 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(&mut emu, (m * 300.0 - 20.0) as f32);
    }
}

/// The two heads after the compare of the track value `T` with 0: `T` below 0 or NaN takes
/// `sky_alpha_neg`, `T` of 0 and above `sky_alpha_one_minus`. `T` is read from `EvalSkyTrack` with
/// the same state; its sign is driven by the value scale of the track, and NaN and exactly 0 are
/// included.
#[test]
fn sky_alpha_branch_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400) + 0x200;
    let outbuf = emu.alloc(0x400);
    let mut s = 0x616c_6272_u32;

    let run = |emu: &mut Emu, value_scale: f32, angle: f32, scale: f32| {
        write_track(emu, G_CE9AF8, &track6(value_scale));
        emu.write_u32(G_CE9B90B, 0);
        emu.write_u32(G_CE9BA4, 0);
        let t = emu.call_f64(0x6cf6c0, 0x00ce_9af8, 6, &[angle.to_bits()]);

        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x100);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::EBX, outbuf);
        emu.write_u32(frame - 0x74, 0);
        emu.write_u32(frame - 0x10, angle.to_bits());
        emu.write_u32(frame - 0x20, scale.to_bits());

        if t < 0.0 || t.is_nan() {
            emu.run(0x6d116c, 0x6d12de);
            let neg_weight = emu.read_f32(frame - 0x24);
            let alpha255_idx = emu.read_u32(frame - 0x40) as i32;
            let glow07 = emu.read_f32(frame - 0x3c);
            let ours = sky_alpha_neg(t, scale);
            assert_f32(
                neg_weight,
                ours.neg_weight,
                &format!("neg: vs={value_scale} a={angle} sc={scale} neg_weight"),
            );
            assert_eq!(
                alpha255_idx, ours.alpha255_idx,
                "neg: vs={value_scale} a={angle} sc={scale} alpha255_idx"
            );
            assert_f32(
                glow07,
                ours.glow07,
                &format!("neg: vs={value_scale} a={angle} sc={scale} glow07"),
            );
        } else {
            emu.run(0x6d116c, 0x6d11aa);
            let binary = emu.read_f32(frame - 0x24);
            let ours = sky_alpha_one_minus(t, scale);
            assert_f32(
                binary,
                ours,
                &format!("pos: vs={value_scale} a={angle} sc={scale} (1-T)*scale"),
            );
        }
    };

    let edges = [
        (1.0f32, 0.5f32, 1.0f32),
        (-1.0, 0.5, 1.0),
        (0.0, 0.5, 1.0),
        (1.0, -1.0, 1.0),
        (-1.0, 1.5, 1.0),
        (1.0, 0.5, 255.0),
        (-1.0, 0.5, 255.0),
        (-1.0, 0.5, -1.0),
        (-1.0, 0.5, 1.0e30),
        (-2.0, 0.5, 0.003_921_569),
        (1.0, 0.5, 0.0),
    ];
    for &(vs, a, sc) in &edges {
        run(&mut emu, vs, a, sc);
    }
    for _ in 0..300 {
        let m1 = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let m2 = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let m3 = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(
            &mut emu,
            (m1 * 4.0 - 2.0) as f32,
            (m2 * 1.4 - 0.2) as f32,
            (m3 * 2.0 - 0.5) as f32,
        );
    }
}
