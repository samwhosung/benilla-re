//! The small light-state functions `GetGlareDepthFactor` (`0x6cf800`), `SampleCloudCover`
//! (`0x6cfa90`), `BuildCloudCurve` (`0x6d0900`), `UpdateStars` (`0x6d1b50`), `SetSpellScreenTint`
//! (`0x6d45c0`), `GetNightFraction` (`0x6d4860`) and `ScaleFogDistance` (`0x6d6090`), each whole,
//! against `dn_fade_factor`, `dn_sky_tex_sample`, `dn_tone_curve`, `dn_cam_sky_setter`,
//! `dn_override_sky_phase`, `dn_track_phase_wrap` and `dn_array_scale_36`. `UpdateStars` and
//! `GetNightFraction` call `EvalSkyTrack` (`0x6cf6c0`), which runs in the emulator too.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_setters::{
    dn_array_scale_36, dn_cam_sky_setter, dn_fade_factor, dn_override_sky_phase, dn_sky_tex_sample,
    dn_tone_curve, dn_track_phase_wrap,
};
use common::{assert_f32, fuzz_f32, next};

// The globals the functions read or write.
/// The scalar `0x672470` copies for `0x6cf800`.
const G_C7BC60: u64 = 0x00c7_bc60;
/// The mode word.
const G_C7F288: u64 = 0x00c7_f288;
/// The time of day.
const G_PHASE: u64 = 0x00ce_9b64;
/// The camera.
const G_CAM_X: u64 = 0x00ce_9b78;
const G_CAM_Y: u64 = 0x00ce_9b7c;
const G_CAM_Z: u64 = 0x00ce_9b80;
/// The tint byte and colour `0x6d45c0` stores.
const G_BA4: u64 = 0x00ce_9ba4;
const G_BA5: u64 = 0x00ce_9ba5;
/// The 256 bytes `0x6d0900` builds.
const G_TONE_TABLE: u64 = 0x00ce_91d8;
/// The tracks of four entries `0x6d1b50` and `0x6d4860` read.
const G_TRACK_1B50: u64 = 0x00ce_9a98;
const G_TRACK_4860: u64 = 0x00ce_9a34;

/// Writes a track of four `{key, value}` pairs at `base` and returns it.
fn write_track4(emu: &mut Emu, base: u64, keys: [f32; 4], vals: [f32; 4]) -> [[f32; 2]; 4] {
    let mut t = [[0.0f32; 2]; 4];
    for i in 0..4 {
        emu.write_u32(base + 8 * i as u64, keys[i].to_bits());
        emu.write_u32(base + 8 * i as u64 + 4, vals[i].to_bits());
        t[i] = [keys[i], vals[i]];
    }
    t
}

/// `0x6cf800` with the mode word 0xf and 0, and parameters that put `v = param·0.1` on and either
/// side of 0 and 1: both zeros, the float either side of 10, negatives, NaN and the infinities.
#[test]
fn dn_fade_factor_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x6661_6465_u32;

    let run = |emu: &mut Emu, param: f32, mode: u32| {
        emu.write_u32(G_C7BC60, param.to_bits());
        emu.write_u32(G_C7F288, mode);
        let binary = emu.call_f32(0x6cf800, 0, 0, &[]);
        assert_f32(
            binary,
            dn_fade_factor(param, mode),
            &format!("param={param} mode={mode:#x}"),
        );
    };

    let param_edges = [
        0.0f32,
        -0.0,
        10.0,
        9.99,
        10.01,
        f32::from_bits(10.0f32.to_bits() - 1),
        f32::from_bits(10.0f32.to_bits() + 1),
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        -1.0,
        5.0,
        100.0,
        -100.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &p in &param_edges {
        run(&mut emu, p, 0);
        run(&mut emu, p, 0xf);
    }
    for _ in 0..500 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let param = match (next(&mut s) >> 8) % 3 {
            0 => (m * 12.0 - 1.0) as f32,
            1 => (m * 2.0 - 1.0) as f32,
            _ => fuzz_f32(&mut s),
        };
        let mode = match (next(&mut s) >> 8) % 3 {
            0 => 0xf,
            1 => 0,
            _ => next(&mut s),
        };
        run(&mut emu, param, mode);
    }
}

/// `0x6cfa90` with the byte pinned: the lookup table is filled with one value B, so any in-bounds
/// index reads B, and the result is B/255 for every B. With `[ecx+0x2c]` 0 the result is 0.
/// `0x6cf870`, which projects the vector, runs in the emulator.
#[test]
fn dn_sky_tex_sample_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // A finite direction; the camera is at the origin.
    let vec = emu.alloc(0x10);
    emu.write_u32(vec, 0.0f32.to_bits());
    emu.write_u32(vec + 4, 0.0f32.to_bits());
    emu.write_u32(vec + 8, 1.0f32.to_bits());
    emu.write_u32(G_CAM_X, 0.0f32.to_bits());
    emu.write_u32(G_CAM_Y, 0.0f32.to_bits());
    emu.write_u32(G_CAM_Z, 0.0f32.to_bits());

    // A large table, based in the middle so a small index in either direction reads it.
    let lut = emu.alloc(0x1_0000);
    let lut_base = lut + 0x8000;

    let this = emu.alloc(0x60);
    emu.write_u32(this + 0x1c, 1);
    emu.write_u32(this + 0x20, 0);
    emu.write_u32(this + 0x44, lut_base as u32);

    emu.write_u32(this + 0x2c, 0);
    let off = emu.call_f32(0x6cfa90, this as u32, 0, &[vec as u32, 0]);
    assert_f32(off, dn_sky_tex_sample(false, 0), "gate off");

    emu.write_u32(this + 0x2c, 1);
    for b in 0u16..=255 {
        let b = b as u8;
        emu.write(lut, &vec![b; 0x1_0000]);
        let binary = emu.call_f32(0x6cfa90, this as u32, 0, &[vec as u32, 0]);
        assert_f32(binary, dn_sky_tex_sample(true, b), &format!("byte={b}"));
    }
}

/// `0x6d0900` builds the 256 bytes; every entry is compared. The step is `(255 − b)/256` and `acc`
/// iterates in `f32`. Gamma runs over (0, 1] and the edges 0, -0, 1 and 2, `b` over 0 to 255; a
/// NaN, negative or infinite gamma sends `pow` into the C runtime's error path, which this does not
/// run. The table is poisoned first, so a missed entry fails.
#[test]
fn dn_tone_curve_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x10);
    let mut s = 0x746f_6e65_u32;

    let run = |emu: &mut Emu, gamma: f32, b: u8| {
        emu.write(G_TONE_TABLE, &[0xAA; 256]);
        emu.write(this + 8, &[b]);
        emu.call(0x6d0900, this as u32, 0, &[gamma.to_bits()]);
        let binary = emu.read(G_TONE_TABLE, 256);
        let ours = dn_tone_curve(gamma, b);
        for (i, (&bb, &oo)) in binary.iter().zip(ours.iter()).enumerate() {
            assert_eq!(bb, oo, "gamma={gamma} b={b} entry {i}");
        }
    };

    let gamma_edges = [
        0.0f32, -0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0, 0.99, 0.01, 2.0,
    ];
    let b_edges = [0u8, 1, 64, 128, 200, 254, 255];
    for &g in &gamma_edges {
        for &b in &b_edges {
            run(&mut emu, g, b);
        }
    }
    for _ in 0..150 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let gamma = (m * 0.99 + 0.005) as f32;
        let b = ((next(&mut s) >> 8) & 0xff) as u8;
        run(&mut emu, gamma, b);
    }
}

/// `0x6d1b50` over three tracks (one with a coincident pair of keys): the camera words are copied
/// as arbitrary bits, and the byte is `__ftol(s·254 + 1)` of the track sample `s` at phases across
/// the clamp, NaN and the infinities.
#[test]
fn dn_cam_sky_setter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x20);
    let mut s = 0x6361_6d31_u32;

    let tracks: [([f32; 4], [f32; 4]); 3] = [
        ([0.1, 0.4, 0.6, 0.9], [0.0, 0.3, 0.8, 1.0]),
        ([0.0, 0.25, 0.5, 0.75], [10.0, -5.0, 20.0, 0.5]),
        ([0.2, 0.2, 0.7, 0.95], [0.4, 0.4, 0.9, 0.1]),
    ];

    for (keys, vals) in tracks {
        let track = write_track4(&mut emu, G_TRACK_1B50, keys, vals);
        for _ in 0..120 {
            let cam = [next(&mut s), next(&mut s), next(&mut s)];
            emu.write_u32(G_CAM_X, cam[0]);
            emu.write_u32(G_CAM_Y, cam[1]);
            emu.write_u32(G_CAM_Z, cam[2]);
            let phase = match (next(&mut s) >> 8) % 3 {
                0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
                1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1.4 - 0.2) as f32,
                _ => fuzz_f32(&mut s),
            };
            emu.write_u32(G_PHASE, phase.to_bits());
            emu.write(this + 0xb, &[0xAA]);
            for off in [0xcu64, 0x10, 0x14] {
                emu.write_u32(this + off, 0xdead_beef);
            }
            emu.call(0x6d1b50, this as u32, 0, &[]);
            let ours = dn_cam_sky_setter(&track, cam, phase);
            assert_eq!(
                emu.read_u32(this + 0xc),
                ours.field_c,
                "field_c phase={phase}"
            );
            assert_eq!(
                emu.read_u32(this + 0x10),
                ours.field_10,
                "field_10 phase={phase}"
            );
            assert_eq!(
                emu.read_u32(this + 0x14),
                ours.field_14,
                "field_14 phase={phase}"
            );
            assert_eq!(
                emu.read(this + 0xb, 1)[0],
                ours.byte_b,
                "byte_b phase={phase} keys={keys:?}"
            );
        }
    }
}

/// `0x6d45c0`: the colour is stored as given, and the byte is `bits(f32(level·255 + 512)) >> 14` at
/// levels across and beyond [0, 1], both zeros, NaN and the infinities.
#[test]
fn dn_override_sky_phase_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x6f76_7264_u32;

    let run = |emu: &mut Emu, flag: u32, phase: f32| {
        emu.write(G_BA4, &[0x55]);
        emu.write_u32(G_BA5, 0xdead_beef);
        emu.call(0x6d45c0, 0, 0, &[flag, phase.to_bits()]);
        let ours = dn_override_sky_phase(flag, phase);
        assert_eq!(
            emu.read_u32(G_BA5),
            ours.flag,
            "flag={flag:#x} phase={phase}"
        );
        assert_eq!(
            emu.read(G_BA4, 1)[0],
            ours.phase_b,
            "phase_b flag={flag:#x} phase={phase}"
        );
    };

    let phase_edges = [
        0.0f32,
        -0.0,
        0.25,
        0.5,
        0.75,
        1.0,
        -0.1,
        1.5,
        -1.0,
        1e30,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ];
    for &p in &phase_edges {
        run(&mut emu, 0x1234_5678, p);
        run(&mut emu, 0, p);
    }
    for _ in 0..400 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let phase = match (next(&mut s) >> 8) % 2 {
            0 => (m * 1.4 - 0.2) as f32,
            _ => fuzz_f32(&mut s),
        };
        let flag = next(&mut s);
        run(&mut emu, flag, phase);
    }
}

/// `0x6d4860` over three tracks: `f32([0xce9b64] + offset)` feeds `EvalSkyTrack`, and the result in
/// `st0` is compared at full width.
#[test]
fn dn_track_phase_wrap_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x7472_6b77_u32;

    let tracks: [([f32; 4], [f32; 4]); 3] = [
        ([0.1, 0.4, 0.6, 0.9], [0.0, 0.3, 0.8, 1.0]),
        ([0.0, 0.3, 0.55, 0.8], [-2.0, 7.5, -1.0, 3.0]),
        ([0.15, 0.5, 0.5, 0.85], [0.2, 0.6, 0.6, 0.95]),
    ];

    for (keys, vals) in tracks {
        let track = write_track4(&mut emu, G_TRACK_4860, keys, vals);
        for _ in 0..150 {
            let base = match (next(&mut s) >> 8) % 3 {
                0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
                1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1.4 - 0.2) as f32,
                _ => fuzz_f32(&mut s),
            };
            let offset = match (next(&mut s) >> 8) % 3 {
                0 => 0.0,
                1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 0.6 - 0.3) as f32,
                _ => fuzz_f32(&mut s),
            };
            emu.write_u32(G_PHASE, base.to_bits());
            let binary = emu.call_f64(0x6d4860, 0, 0, &[offset.to_bits()]);
            let ours = dn_track_phase_wrap(&track, base, offset);
            assert_eq!(
                binary.to_bits(),
                ours.to_bits(),
                "base={base} offset={offset} keys={keys:?}: binary={binary} ours={ours}"
            );
        }
    }
}

/// `0x6d6090`: the flag in `dl` off (no change), a count of 0, and counts of 1 to 8 values of mixed
/// sign and scale.
#[test]
fn dn_array_scale_36_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x48 + 0x40);
    let mut s = 0x6172_7363_u32;

    let run = |emu: &mut Emu, vals: &[f32], enable: bool| {
        emu.write_u32(this + 4, vals.len() as u32);
        for (k, &v) in vals.iter().enumerate() {
            emu.write_u32(this + 0x48 + 4 * k as u64, v.to_bits());
        }
        let edx = u32::from(enable);
        emu.call(0x6d6090, this as u32, edx, &[]);
        let ours = dn_array_scale_36(vals, enable);
        for (k, &o) in ours.iter().enumerate() {
            let got = emu.read_f32(this + 0x48 + 4 * k as u64);
            assert_f32(
                got,
                o,
                &format!("enable={enable} len={} elem {k}", vals.len()),
            );
        }
    };

    run(&mut emu, &[], true);
    run(&mut emu, &[1.0, 36.0, -72.0], false);
    run(&mut emu, &[36.0], true);
    run(&mut emu, &[f32::NAN, f32::INFINITY, -0.0, 0.0], true);

    for _ in 0..300 {
        let len = ((next(&mut s) >> 8) % 8) as usize + 1;
        let vals: Vec<f32> = (0..len).map(|_| fuzz_f32(&mut s)).collect();
        let enable = (next(&mut s) >> 8) & 1 == 0;
        run(&mut emu, &vals, enable);
    }
}
