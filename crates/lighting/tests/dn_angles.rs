//! The glare setups `BuildSunGlare` (`0x6d1e30`) and `BuildMoonGlare` (`0x6d1f30`), the heading
//! `UpdateCameraHeading` (`0x6d4530`), the tables of `BuildCloudNoiseTables` (`0x6d0c90`) and
//! `ScaleLightPosition` (`0x6d5fb0`), against `sun_glare_setup`, `moon_glare_setup`,
//! `wind_angle_update`, `sky_noise_value`, `sky_bump_value` and `vec_scale_remap`. The glare setups
//! and the heading run as the slice that holds their arithmetic; the others run whole.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_angles::{
    moon_glare_setup, sky_bump_value, sky_noise_value, sun_glare_setup, vec_scale_remap,
    wind_angle_update, VecRemap,
};
use common::{assert_f32, next};

/// A float across signs and scales (tiny, to straddle the wind epsilon, up to huge), with an
/// occasional exact 0 and NaN, drawn from the high bits of the LCG.
fn fuzz_coord(s: &mut u32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    let sign = if (next(s) >> 8) & 1 == 0 { 1.0 } else { -1.0 };
    match (next(s) >> 8) % 7 {
        0 => 0.0,
        1 => f32::NAN,
        2 => (sign * m * 1.0e-3) as f32,
        3 => (sign * m * 0.05) as f32,
        4 => (sign * m * 5.0) as f32,
        5 => (sign * m * 200.0) as f32,
        _ => (sign * m * 1.0e18) as f32,
    }
}

/// `0x6d1e30` from `0x6d1e3f`, after its call to `0x7e5000`, to the `ret` at `0x6d1f22`: the four
/// sums are poisoned and read back.
#[test]
fn sun_glare_setup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let expected = sun_glare_setup();
    for &(va, _) in &expected {
        emu.write_u32(va, 0xdead_beef);
    }
    emu.run(0x6d1e3f, 0x6d1f22);
    for &(va, bits) in &expected {
        assert_f32(
            emu.read_f32(va),
            f32::from_bits(bits),
            &format!("sun glare [{va:#x}]"),
        );
    }
}

/// `0x6d1f30` from `0x6d1f3f`, after its call to `0x7e5000`, to the `ret` at `0x6d2022`.
#[test]
fn moon_glare_setup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let expected = moon_glare_setup();
    for &(va, _) in &expected {
        emu.write_u32(va, 0xdead_beef);
    }
    emu.run(0x6d1f3f, 0x6d2022);
    for &(va, bits) in &expected {
        assert_f32(
            emu.read_f32(va),
            f32::from_bits(bits),
            &format!("moon glare [{va:#x}]"),
        );
    }
}

/// The wind globals `0x6d4530` reads and writes.
const G_WIND_X: u64 = 0xce9b84;
const G_WIND_Y: u64 = 0xce9b88;
const G_WIND_Z: u64 = 0xce9b8c;
const G_WIND_OUT: u64 = 0xce9b90;

/// `0x6d4530` from its start to the first call at `0x6d4596`, which reads `gx`, `gy` and `gz` and
/// writes the heading. The squared length is probed on and either side of its threshold, along each
/// axis, with both zeros and NaN in each position.
#[test]
fn wind_angle_update_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let run = |emu: &mut Emu, gx: f32, gy: f32, gz: f32| {
        emu.write_u32(G_WIND_X, gx.to_bits());
        emu.write_u32(G_WIND_Y, gy.to_bits());
        emu.write_u32(G_WIND_Z, gz.to_bits());
        emu.write_u32(G_WIND_OUT, 0xdead_beef);
        emu.run(0x6d4530, 0x6d4596);
        let binary = emu.read_f32(G_WIND_OUT);
        assert_f32(
            binary,
            wind_angle_update(gx, gy, gz),
            &format!("wind angle gx={gx} gy={gy} gz={gz}"),
        );
    };

    let edges: [(f32, f32, f32); 17] = [
        (1.0, 1.0, 0.5),
        (-1.0, 1.0, 0.5),
        (-1.0, -1.0, 0.5),
        (1.0, -1.0, 0.5),
        (1.0, 0.0, 0.5),
        (0.0, 1.0, 0.5),
        (0.0, -1.0, 0.5),
        (0.0, 0.0, 0.0),
        (-0.0, -0.0, -0.0),
        (1.0e-4, 1.0e-4, -1.0),
        (0.005, 0.005, 0.0),
        (f32::NAN, 1.0, 0.5),
        (0.01, 0.01, f32::NAN),
        (0.01, f32::NAN, 0.5),
        (f32::INFINITY, 0.0, 0.5),
        (0.0, f32::INFINITY, 0.5),
        (-1.0, -0.0, 0.5),
    ];
    for &(gx, gy, gz) in &edges {
        run(&mut emu, gx, gy, gz);
    }

    // The squared length at the threshold: one component steps through the neighbours of the
    // square root of 0.0001 on each axis, with the other at 0 or -0.
    let root = (1.0e-4f64.sqrt() as f32).to_bits();
    for step in 0..=8u32 {
        let c = f32::from_bits(root - 4 + step);
        for other in [0.0f32, -0.0] {
            run(&mut emu, c, other, 0.7);
            run(&mut emu, other, c, 0.7);
            run(&mut emu, -c, other, -0.7);
            run(&mut emu, other, -c, -0.7);
        }
    }

    let mut s = 0x77_69_6e_64_u32;
    for _ in 0..900 {
        let gx = fuzz_coord(&mut s);
        let gy = fuzz_coord(&mut s);
        let gz = fuzz_coord(&mut s);
        run(&mut emu, gx, gy, gz);
    }
}

/// The C runtime `rand` (`0x7400e5`) that `0x6d0c90` calls, and the two tables it fills.
const RAND_FN: u64 = 0x7400e5;
const NOISE_TABLE: u64 = 0xce92d8;
const BUMP_TABLE: u64 = 0xce8dd4;

/// The noise table: with `rand` stubbed to return K every entry is `sky_noise_value(K)`. A fresh
/// emulator for each K, since a stub is fixed per emulator.
#[test]
fn sky_noise_lut_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    for &k in &[0u32, 1, 0x1234, 0x4000, 0x7fff, 0x3abc, 0x5d21] {
        let mut emu = Emu::new(&wow);
        emu.stub(RAND_FN, k, 0);
        for i in 0..256u64 {
            emu.write_u32(NOISE_TABLE + 4 * i, 0xdead_beef);
        }
        emu.call(0x6d0c90, 0, 0, &[]);
        let ours = sky_noise_value(k);
        for i in 0..256u64 {
            assert_f32(
                emu.read_f32(NOISE_TABLE + 4 * i),
                ours,
                &format!("noise[{i}] K={k:#x}"),
            );
        }
    }
}

/// The cosine table: entries 1 to 256 at `0xce8dd4`, independent of `rand`. `fcos` is the host's.
#[test]
fn sky_bump_lut_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(RAND_FN, 0, 0);
    for i in 0..258u64 {
        emu.write_u32(BUMP_TABLE + 4 * i, 0xdead_beef);
    }
    emu.call(0x6d0c90, 0, 0, &[]);
    for v in 0..256u64 {
        assert_f32(
            emu.read_f32(BUMP_TABLE + 4 * (v + 1)),
            sky_bump_value(v as u32),
            &format!("bump[{}] (v={v})", v + 1),
        );
    }
}

/// The global `0x66fc30` returns the truth of, which picks the offset.
const G_C66: u64 = 0xc9e384;

/// `0x6d5fb0` whole. The no-op gates (all components zero, the gate off), both offsets, partial
/// zeros, NaN and infinite components, and components whose scaled value is exactly the offset
/// (614400 and the float below it), where `fsub` then `fchs` leaves a negative zero.

#[test]
fn vec_scale_remap_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec = emu.alloc(0x20);

    let run = |emu: &mut Emu, v: VecRemap, gate: bool, c66: bool| {
        emu.write_u32(rec + 0x08, v.f8.to_bits());
        emu.write_u32(rec + 0x0c, v.fc.to_bits());
        emu.write_u32(rec + 0x10, v.f10.to_bits());
        emu.write_u32(rec + 0x14, v.f14.to_bits());
        emu.write_u32(rec + 0x18, v.f18.to_bits());
        emu.write_u32(G_C66, u32::from(c66));
        emu.call(0x6d5fb0, rec as u32, u32::from(gate), &[]);
        let ours = vec_scale_remap(v, gate, c66);
        let ctx = format!("{v:?} gate={gate} c66={c66}");
        assert_f32(emu.read_f32(rec + 0x08), ours.f8, &format!("f8 {ctx}"));
        assert_f32(emu.read_f32(rec + 0x0c), ours.fc, &format!("fc {ctx}"));
        assert_f32(emu.read_f32(rec + 0x10), ours.f10, &format!("f10 {ctx}"));
        assert_f32(emu.read_f32(rec + 0x14), ours.f14, &format!("f14 {ctx}"));
        assert_f32(emu.read_f32(rec + 0x18), ours.f18, &format!("f18 {ctx}"));
    };

    let z = VecRemap {
        f8: 0.0,
        fc: 0.0,
        f10: 0.0,
        f14: 0.0,
        f18: 0.0,
    };
    let v = VecRemap {
        f8: 3.0,
        fc: -6.0,
        f10: 9.0,
        f14: 12.0,
        f18: -15.0,
    };
    let p = VecRemap {
        f8: 0.0,
        fc: 0.0,
        f10: 2.5,
        f14: -1.0,
        f18: 0.0,
    };
    let mz = VecRemap {
        f8: -0.0,
        fc: 0.0,
        f10: -0.0,
        f14: 1.0,
        f18: -0.0,
    };
    let at_off = VecRemap {
        f8: 614400.0,
        fc: 2.0,
        f10: 614400.0,
        f14: 0.0,
        f18: 1.0,
    };
    let at_off_below = VecRemap {
        f8: 614_399.94,
        fc: -2.0,
        f10: 614_399.94,
        f14: 5.0,
        f18: 6.0,
    };
    for &gate in &[true, false] {
        for &c66 in &[true, false] {
            run(&mut emu, z, gate, c66);
            run(&mut emu, v, gate, c66);
            run(&mut emu, p, gate, c66);
            run(&mut emu, mz, gate, c66);
            run(&mut emu, at_off, gate, c66);
            run(&mut emu, at_off_below, gate, c66);
        }
    }
    let nan = VecRemap {
        f8: f32::NAN,
        fc: 1.0,
        f10: f32::INFINITY,
        f14: f32::NAN,
        f18: 2.0,
    };
    run(&mut emu, nan, true, true);
    run(&mut emu, nan, true, false);

    let mut s = 0x76_65_63_72_u32;
    for _ in 0..600 {
        let v = VecRemap {
            f8: fuzz_coord(&mut s),
            fc: fuzz_coord(&mut s),
            f10: fuzz_coord(&mut s),
            f14: fuzz_coord(&mut s),
            f18: fuzz_coord(&mut s),
        };
        let gate = (next(&mut s) >> 8) & 1 == 0;
        let c66 = (next(&mut s) >> 8) & 1 == 0;
        run(&mut emu, v, gate, c66);
    }
}
