//! The drift cloud kernels against the exe, each whole: the count `0x68e650`, the gust direction
//! `0x68e1c0`, the gust velocity `0x68e4f0`, the scatter `0x68e720` and the advance `0x68e930`.
//!
//! The count test poisons `this + 0xfa00`, runs the function and compares the stored integer; its
//! argument runs over zero, both signs, the `__ftol` overflow near 2^63/4000, NaN, the infinities
//! and a fuzz. The other four seed the generator state `[0x882664]` and `[0x882668]` to the same
//! value as the transcription's `Stream`, write the cloud header into the emulator, run the
//! function (its inlined draws and the callee run real) and compare every stored dword, the
//! generator state after the call included.
//!
//! Host double (`docs/METHOD.md#the-harness`): `weather_drift_reinit_difftest` runs `fsin` at
//! `0x68e243` and `0x68e247` and `fcos` at `0x68e253` and `0x68e262`.
//! `weather_gust_velocity_difftest`, `weather_toroidal_advect_difftest` run `fsin` at `0x68e243`,
//! `0x68e247` and `0x68e55f` and `fcos` at `0x68e253` and `0x68e262`. The emulator computes each in
//! host `double`, so where its result reaches a compared value the host's math stands in for the
//! x87's, and the test shows the code around it.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_random::Stream;
use benilla_re_terrain::driftcloud::{
    drift_cloud_count, weather_cloud_reseed, weather_drift_reinit, weather_gust_velocity,
    weather_toroidal_advect, DriftCloud,
};
use common::{assert_f32, next};

const FN_REINIT: u64 = 0x68e1c0;
const FN_GUST: u64 = 0x68e4f0;
const FN_RESEED: u64 = 0x68e720;
const FN_ADVECT: u64 = 0x68e930;

/// The generator's table.
const TABLE_VA: u64 = 0x802700;
/// The generator's accumulator.
const ACC_VA: u64 = 0x882664;
/// The generator's lanes.
const IDX_VA: u64 = 0x882668;
/// The camera position `0x68e930` reads.
const CAM_VA: u64 = 0xc7cf20;
/// The time step `0x68e930` passes to the gust.
const DT_VA: u64 = 0xc62510;

const OFF_COUNT: u64 = 0xfa00;
const OFF_LAST_CAM: u64 = 0xfa04;
const OFF_ENABLED: u64 = 0xfa14;
const OFF_RATE: u64 = 0xfa18;
const OFF_EXTENT: u64 = 0xfa1c;
const OFF_MODE: u64 = 0xfa24;
const OFF_DIR: u64 = 0xfa28;
const OFF_FREQ: u64 = 0xfa34;
const OFF_PHASE: u64 = 0xfa38;
const OFF_AMP: u64 = 0xfa3c;
/// The object: 4000 records of 16 bytes and the header to `+0xfa40`.
const OBJ_SIZE: u64 = 0xfa40;

/// The generator's table, as the exe holds it.
fn read_table(emu: &Emu) -> Vec<u8> {
    emu.read(TABLE_VA, 256)
}

/// Seed the exe's generator and return the transcription's, at the same state.
fn seed_rng<'t>(emu: &mut Emu, acc: u32, idx: u32, table: &'t [u8]) -> Stream<'t> {
    emu.write_u32(ACC_VA, acc);
    emu.write_u32(IDX_VA, idx);
    Stream { acc, idx, table }
}

/// Write the header fields of `c` into the object at `obj`.
fn write_cloud(emu: &mut Emu, obj: u64, c: &DriftCloud) {
    emu.write_u32(obj + OFF_COUNT, c.count);
    for (i, v) in c.last_cam.iter().enumerate() {
        emu.write_u32(obj + OFF_LAST_CAM + 4 * i as u64, v.to_bits());
    }
    emu.write(obj + OFF_ENABLED, &[u8::from(c.enabled)]);
    emu.write_u32(obj + OFF_RATE, c.spawn_rate.to_bits());
    emu.write_u32(obj + OFF_EXTENT, c.box_extent.to_bits());
    emu.write_u32(obj + OFF_MODE, c.mode);
    for (i, v) in c.gust_dir.iter().enumerate() {
        emu.write_u32(obj + OFF_DIR + 4 * i as u64, v.to_bits());
    }
    emu.write_u32(obj + OFF_FREQ, c.gust_freq.to_bits());
    emu.write_u32(obj + OFF_PHASE, c.gust_phase.to_bits());
    emu.write_u32(obj + OFF_AMP, c.gust_amp.to_bits());
}

/// A fuzzed finite f32: unit, box-sized, angle-sized and small values.
fn fz(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 4;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 40.0) as f32,
        2 => (m * 6.3 - 3.15) as f32,
        _ => (m * 0.2) as f32,
    }
}

/// `0x68e650` whole: the count for arguments around zero, the `__ftol` overflow, NaN, infinities
/// and a fuzz.
#[test]
fn drift_cloud_set_fa00_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x1_0000);

    let mut run = |arg: f32| {
        emu.write_u32(this + 0xfa00, 0xdead_beef);
        emu.call(0x68e650, this as u32, 0, &[arg.to_bits()]);
        let binary = emu.read_u32(this + 0xfa00) as i32;
        assert_eq!(
            binary,
            drift_cloud_count(arg),
            "arg={arg} ({:#010x})",
            arg.to_bits()
        );
    };

    for a in [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.25,
        -0.25,
        0.99999994,
        1000.0,
        -1000.0,
        2.305_843e15,
        2.4e15,
        f32::from_bits(0x5a0f_ffff),
        f32::from_bits(0x5a10_0000),
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
    ] {
        run(a);
    }
    let mut s = 0x6472_6966u32;
    for _ in 0..500 {
        run(f32::from_bits(next(&mut s)));
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        run(((m * 2.0 - 1.0) * 10_000.0) as f32);
    }
}

/// `0x68e1c0` whole, 600 seeded states: both signs of each draw, the direction, the frequency, the
/// amplitude, the zeroed phase and the generator state after the four draws.
#[test]
fn weather_drift_reinit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let obj = emu.alloc(OBJ_SIZE);
    let mut s = 0x6472_6966u32;
    for case in 0..600 {
        let acc = next(&mut s);
        let idx = next(&mut s);
        let mut rng = seed_rng(&mut emu, acc, idx, &table);
        let mut c = DriftCloud::default();
        write_cloud(&mut emu, obj, &c);
        emu.write_u32(obj + OFF_PHASE, 0x3f80_0000);

        emu.call(FN_REINIT, obj as u32, 0, &[]);
        weather_drift_reinit(&mut c, &mut rng);

        for (k, v) in c.gust_dir.iter().enumerate() {
            assert_f32(
                emu.read_f32(obj + OFF_DIR + 4 * k as u64),
                *v,
                &format!("dir[{k}] (case {case} acc={acc:#x} idx={idx:#x})"),
            );
        }
        assert_f32(emu.read_f32(obj + OFF_FREQ), c.gust_freq, "freq");
        assert_f32(emu.read_f32(obj + OFF_PHASE), c.gust_phase, "phase");
        assert_f32(emu.read_f32(obj + OFF_AMP), c.gust_amp, "amp");
        assert_eq!(emu.read_u32(ACC_VA), rng.acc, "rng.acc (case {case})");
        assert_eq!(emu.read_u32(IDX_VA), rng.idx, "rng.idx (case {case})");
    }
}

/// `0x68e4f0` whole, 800 states over modes 0 to 4 and 0xffffffff (above 1 is unsigned), plus the
/// reseed boundary: a phase times frequency of exactly 0.5 keeps the direction, just above it
/// reseeds, and a NaN step or frequency never reseeds.
#[test]
fn weather_gust_velocity_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let obj = emu.alloc(OBJ_SIZE);
    let out = emu.alloc(0x10);
    let mut s = 0x6775_7374u32;

    let run = |emu: &mut Emu, mut c: DriftCloud, dt: f32, acc: u32, idx: u32, ctx: &str| {
        let mut rng = seed_rng(emu, acc, idx, &table);
        write_cloud(emu, obj, &c);
        emu.call(FN_GUST, obj as u32, 0, &[out as u32, dt.to_bits()]);
        let ours = weather_gust_velocity(&mut c, dt, &mut rng);
        for (k, v) in ours.iter().enumerate() {
            assert_f32(
                emu.read_f32(out + 4 * k as u64),
                *v,
                &format!("vel[{k}] ({ctx} mode={})", c.mode),
            );
        }
        if c.mode <= 1 {
            for (k, v) in c.gust_dir.iter().enumerate() {
                assert_f32(
                    emu.read_f32(obj + OFF_DIR + 4 * k as u64),
                    *v,
                    &format!("post dir[{k}] ({ctx})"),
                );
            }
            assert_f32(emu.read_f32(obj + OFF_PHASE), c.gust_phase, ctx);
            assert_eq!(emu.read_u32(ACC_VA), rng.acc, "rng.acc ({ctx})");
            assert_eq!(emu.read_u32(IDX_VA), rng.idx, "rng.idx ({ctx})");
        }
    };

    for case in 0..800 {
        let acc = next(&mut s);
        let idx = next(&mut s);
        let mode = match case % 6 {
            5 => u32::MAX,
            m => m as u32,
        };
        let c = DriftCloud {
            mode,
            gust_freq: fz(&mut s).abs() + 0.01,
            gust_phase: fz(&mut s),
            gust_amp: fz(&mut s),
            gust_dir: [fz(&mut s), fz(&mut s), fz(&mut s)],
            ..Default::default()
        };
        let dt = fz(&mut s);
        run(&mut emu, c, dt, acc, idx, &format!("case {case}"));
    }

    let base = DriftCloud {
        gust_freq: 0.5,
        gust_phase: 0.0,
        gust_amp: 0.25,
        gust_dir: [0.6, 0.0, 0.8],
        ..Default::default()
    };
    run(&mut emu, base, 1.0, 1, 2, "term exactly 0.5");
    run(
        &mut emu,
        base,
        f32::from_bits(0x3f80_0001),
        1,
        2,
        "term just above 0.5",
    );
    run(
        &mut emu,
        base,
        f32::from_bits(0x3f7f_ffff),
        1,
        2,
        "term just below 0.5",
    );
    run(&mut emu, base, f32::NAN, 1, 2, "NaN step");
    run(&mut emu, base, -0.0, 1, 2, "negative zero step");
    let nan_freq = DriftCloud {
        gust_freq: f32::NAN,
        ..base
    };
    run(&mut emu, nan_freq, 1.0, 1, 2, "NaN frequency");
}

/// `0x68e720` whole, 300 states with 0 to 24 particles (0 writes nothing): every record and the
/// stored mode `new_mode & 3`, for a full 32-bit `new_mode`.
#[test]
fn weather_cloud_reseed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let obj = emu.alloc(OBJ_SIZE);
    let mut s = 0x7273_6564u32;
    for case in 0..300 {
        let acc = next(&mut s);
        let idx = next(&mut s);
        let count = if case == 0 {
            0
        } else {
            1 + (next(&mut s) >> 8) % 24
        };
        let mut c = DriftCloud {
            count,
            spawn_rate: fz(&mut s).abs() + 0.05,
            box_extent: fz(&mut s).abs() + 1.0,
            mode: (next(&mut s) >> 8) % 4,
            ..Default::default()
        };
        let new_mode = next(&mut s);
        let mut rng = seed_rng(&mut emu, acc, idx, &table);
        write_cloud(&mut emu, obj, &c);
        for k in 0..count as u64 * 4 {
            emu.write_u32(obj + 4 * k, 0);
        }

        emu.call(FN_RESEED, obj as u32, 0, &[new_mode]);
        let ours = weather_cloud_reseed(&mut c, new_mode, &mut rng);

        assert_eq!(ours.len(), count as usize, "len (case {case})");
        for (p, rec) in ours.iter().enumerate() {
            for (k, v) in rec.iter().enumerate() {
                assert_f32(
                    emu.read_f32(obj + (p as u64 * 16) + 4 * k as u64),
                    *v,
                    &format!("particle[{p}][{k}] (case {case} count={count})"),
                );
            }
        }
        assert_eq!(emu.read_u32(obj + OFF_MODE), c.mode, "mode (case {case})");
        assert_eq!(c.mode, new_mode & 3, "mode (case {case})");
        assert_eq!(emu.read_u32(ACC_VA), rng.acc, "rng.acc (case {case})");
        assert_eq!(emu.read_u32(IDX_VA), rng.idx, "rng.idx (case {case})");
    }
}

/// `0x68e930` whole, 300 states: the disabled no-op, a camera jump past the box (rescatter), a
/// small move (wrap), modes above 3 (the rescatter stores `mode & 3`), and the jump boundary: a
/// camera delta of exactly the box edge does not rescatter, NaN camera deltas never do, and the
/// squared length sums in the exe's order.
#[test]
fn weather_toroidal_advect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let table = read_table(&emu);
    let obj = emu.alloc(OBJ_SIZE);
    let mut s = 0x6164_7663u32;

    let run = |emu: &mut Emu,
               mut c: DriftCloud,
               mut parts: Vec<[f32; 4]>,
               cam: [f32; 3],
               dt: f32,
               acc: u32,
               idx: u32,
               ctx: &str| {
        let mut rng = seed_rng(emu, acc, idx, &table);
        write_cloud(emu, obj, &c);
        for (k, v) in cam.iter().enumerate() {
            emu.write_u32(CAM_VA + 4 * k as u64, v.to_bits());
        }
        emu.write_u32(DT_VA, dt.to_bits());
        for (p, rec) in parts.iter().enumerate() {
            for (k, v) in rec.iter().enumerate() {
                emu.write_u32(obj + (p as u64 * 16) + 4 * k as u64, v.to_bits());
            }
        }

        emu.call(FN_ADVECT, obj as u32, 0, &[]);
        weather_toroidal_advect(&mut c, &mut parts, cam, dt, &mut rng);

        for (p, rec) in parts.iter().enumerate() {
            for (k, v) in rec.iter().enumerate() {
                assert_f32(
                    emu.read_f32(obj + (p as u64 * 16) + 4 * k as u64),
                    *v,
                    &format!("particle[{p}][{k}] ({ctx})"),
                );
            }
        }
        for (k, v) in c.last_cam.iter().enumerate() {
            assert_f32(
                emu.read_f32(obj + OFF_LAST_CAM + 4 * k as u64),
                *v,
                &format!("last_cam[{k}] ({ctx})"),
            );
        }
        assert_eq!(emu.read_u32(obj + OFF_MODE), c.mode, "mode ({ctx})");
        assert_eq!(emu.read_u32(ACC_VA), rng.acc, "rng.acc ({ctx})");
        assert_eq!(emu.read_u32(IDX_VA), rng.idx, "rng.idx ({ctx})");
    };

    for case in 0..300 {
        let acc = next(&mut s);
        let idx = next(&mut s);
        let enabled = case != 1;
        let count = 1 + (next(&mut s) >> 8) % 20;
        let extent = fz(&mut s).abs() + 2.0;
        let cam = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let big_jump = case % 3 == 0;
        let last_cam = if big_jump {
            [cam[0] + extent * 4.0, cam[1], cam[2]]
        } else {
            [
                cam[0] + fz(&mut s) * 0.1,
                cam[1] + fz(&mut s) * 0.1,
                cam[2] + fz(&mut s) * 0.1,
            ]
        };
        let c = DriftCloud {
            count,
            last_cam,
            enabled,
            spawn_rate: fz(&mut s).abs() + 0.05,
            box_extent: extent,
            mode: if case % 7 == 0 {
                next(&mut s)
            } else {
                (next(&mut s) >> 8) % 4
            },
            gust_freq: fz(&mut s).abs() + 0.01,
            gust_phase: fz(&mut s),
            gust_amp: fz(&mut s),
            gust_dir: [fz(&mut s), fz(&mut s), fz(&mut s)],
        };
        let dt = fz(&mut s);
        let parts: Vec<[f32; 4]> = (0..count)
            .map(|_| {
                [
                    fz(&mut s) * extent,
                    fz(&mut s) * extent,
                    fz(&mut s) * extent,
                    fz(&mut s),
                ]
            })
            .collect();
        run(
            &mut emu,
            c,
            parts,
            cam,
            dt,
            acc,
            idx,
            &format!("case {case} jump={big_jump} enabled={enabled}"),
        );
    }

    let cam = [1.0, 2.0, 3.0];
    let base = DriftCloud {
        count: 3,
        enabled: true,
        spawn_rate: 1.0,
        box_extent: 8.0,
        mode: 1,
        gust_freq: 0.01,
        gust_phase: 0.0,
        gust_amp: 0.1,
        gust_dir: [0.0, 0.0, 1.0],
        ..Default::default()
    };
    let parts = vec![
        [3.9, -3.9, 0.0, 1.0],
        [4.5, -4.5, 4.0, 1.0],
        [-4.0, 4.0, -4.0, 1.0],
    ];
    let step = |d: [f32; 3]| DriftCloud {
        last_cam: [cam[0] + d[0], cam[1] + d[1], cam[2] + d[2]],
        ..base
    };
    run(
        &mut emu,
        step([8.0, 0.0, 0.0]),
        parts.clone(),
        cam,
        0.016,
        5,
        6,
        "delta equals the edge",
    );
    run(
        &mut emu,
        step([8.0, 0.5, 0.0]),
        parts.clone(),
        cam,
        0.016,
        5,
        6,
        "delta over the edge",
    );
    run(
        &mut emu,
        step([0.0, 0.0, 8.0]),
        parts.clone(),
        cam,
        0.016,
        5,
        6,
        "z delta equals the edge",
    );
    run(
        &mut emu,
        step([f32::NAN, 0.0, 0.0]),
        parts.clone(),
        cam,
        0.016,
        5,
        6,
        "NaN delta",
    );
    run(
        &mut emu,
        step([0.1, 0.0, 0.0]),
        parts.clone(),
        [f32::NAN, 2.0, 3.0],
        0.016,
        5,
        6,
        "NaN camera",
    );
    let nan_part = vec![
        [f32::NAN, 5.0, -5.0, 1.0],
        [4.0, -4.0, 4.0, 1.0],
        [-0.0, 0.0, 0.0, 0.0],
    ];
    run(
        &mut emu,
        step([0.0, 0.0, 0.0]),
        nan_part,
        cam,
        0.016,
        5,
        6,
        "NaN particle and the half edges",
    );
}
