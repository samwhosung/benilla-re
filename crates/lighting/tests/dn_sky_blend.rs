//! `BlendLightRecord` (`0x6d67c0`) and `OverblendAreaLight` (`0x6d30e0`) whole, against
//! `sky_weighted_blend` and `cloud_record_overblend`: a random record pair and a weight are laid
//! out, the function runs, and all 0x24 dwords of the destination are compared. The leaves
//! `0x6d6c20`, `0x690ae0`, `0x6d15f0` and `0x4a2c10` run in the emulator too.
//!
//! The weights include both zeros, 1 and the float either side, negatives, values above 1, NaN and
//! any bits, which put the clamp, both directions of every lerp, the colour weights 0 and 0xff and
//! the `w > 0` gate of the cloud accumulator on and either side of their thresholds.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lighting::dn_sky_blend::{cloud_record_overblend, sky_weighted_blend, GatherRecord};
use common::next;

/// Write a 0x24-dword record into emulator memory at `base`.
fn write_record(emu: &mut Emu, base: u64, rec: &GatherRecord) {
    for (k, &w) in rec.iter().enumerate() {
        emu.write_u32(base + 4 * k as u64, w);
    }
}

/// Read a 0x24-dword record back from emulator memory at `base`.
fn read_record(emu: &Emu, base: u64) -> GatherRecord {
    let mut r: GatherRecord = [0; 0x24];
    for (k, slot) in r.iter_mut().enumerate() {
        *slot = emu.read_u32(base + 4 * k as u64);
    }
    r
}

/// A record of random dwords: full random bits, or a small positive or negative `f32`. Both
/// functions read each slot by their own offset tables, and the mix gives both lerp directions and
/// a spread of byte values.
fn fuzz_record(s: &mut u32) -> GatherRecord {
    let mut r: GatherRecord = [0; 0x24];
    for slot in r.iter_mut() {
        let pick = (next(s) >> 8) % 4;
        *slot = match pick {
            0 | 1 => next(s),
            2 => ((next(s) >> 8) as f32 / 4096.0).to_bits(),
            _ => (-((next(s) >> 8) as f32) / 4096.0).to_bits(),
        };
    }
    r
}

/// The weights that sit on a threshold of either function.
fn weight_probes() -> Vec<f32> {
    vec![
        0.0,
        1.0,
        -0.0,
        f32::from_bits(0x3380_0000),
        0.5,
        0.25,
        0.75,
        f32::from_bits(0x3f7f_ffff),
        f32::from_bits(0x3f80_0001),
        -0.5,
        2.0,
        100.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1e30,
        f32::from_bits(0x3b80_8081),
    ]
}

/// `0x6d67c0`: the probes first, then weights in [0, 2), [-1, 1), any bits, 0 and 1.
#[test]
fn sky_weighted_blend_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let dst_p = emu.alloc(0x90);
    let src_p = emu.alloc(0x90);
    let mut s = 0x736b_795fu32;
    let probes = weight_probes();
    for case in 0..1200u32 {
        let src = fuzz_record(&mut s);
        let dst0 = fuzz_record(&mut s);
        let t = if (case as usize) < probes.len() {
            probes[case as usize]
        } else {
            match case % 5 {
                0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 23)) as f32,
                1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
                2 => f32::from_bits(next(&mut s)),
                3 => 0.0,
                _ => 1.0,
            }
        };

        write_record(&mut emu, dst_p, &dst0);
        write_record(&mut emu, src_p, &src);
        emu.call(0x6d67c0, dst_p as u32, src_p as u32, &[t.to_bits()]);
        let binary = read_record(&emu, dst_p);

        let mut ours = dst0;
        sky_weighted_blend(&mut ours, &src, t);

        for (k, (&b, &o)) in binary.iter().zip(ours.iter()).enumerate() {
            assert_eq!(
                b,
                o,
                "sky_weighted_blend slot {k:#x} (case {case} t={t} bits={:#010x}): \
                 binary={b:#010x} ours={o:#010x}",
                t.to_bits()
            );
        }
    }
}

/// `0x6d30e0`: the same weights, with the cloud accumulator driven in four ways: a match in the
/// first pair with a weight of 0.9 (so a positive `w` clamps to 1), an insert into an empty first
/// pair, both pairs held by other ids (a drop), and a match in the second pair.
#[test]
fn cloud_record_overblend_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let dst_p = emu.alloc(0x90);
    let src_p = emu.alloc(0x90);
    let mut s = 0x636c_6f75u32;
    let probes = weight_probes();
    // The ids and weights of the two pairs: `+0x78`, `+0x7c`, `+0x80`, `+0x84`.
    let (id0, w0, id1, w1) = (0x78 / 4, 0x7c / 4, 0x80 / 4, 0x84 / 4);

    for case in 0..1600u32 {
        let src = fuzz_record(&mut s);
        let mut dst0 = fuzz_record(&mut s);

        let src_id = src[id0];
        match case % 4 {
            0 => {
                dst0[id0] = src_id;
                dst0[w0] = 0.9f32.to_bits();
                dst0[id1] = 0;
            }
            1 => {
                dst0[id0] = 0;
                dst0[id1] = 0;
            }
            2 => {
                dst0[id0] = src_id.wrapping_add(1) | 1;
                dst0[id1] = src_id.wrapping_add(2) | 2;
            }
            _ => {
                dst0[id0] = src_id.wrapping_add(7) | 4;
                dst0[id1] = src_id;
                dst0[w1] = 0.3f32.to_bits();
            }
        }

        let w = if (case as usize) < probes.len() {
            probes[case as usize]
        } else {
            match case % 6 {
                0 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 23)) as f32,
                1 => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0 - 1.0) as f32,
                2 => f32::from_bits(next(&mut s)),
                3 => 0.0,
                4 => 1.0,
                _ => 0.5,
            }
        };

        write_record(&mut emu, dst_p, &dst0);
        write_record(&mut emu, src_p, &src);
        emu.call(0x6d30e0, dst_p as u32, src_p as u32, &[w.to_bits()]);
        let binary = read_record(&emu, dst_p);

        let mut ours = dst0;
        cloud_record_overblend(&mut ours, &src, w);

        for (k, (&b, &o)) in binary.iter().zip(ours.iter()).enumerate() {
            assert_eq!(
                b,
                o,
                "cloud_record_overblend slot {k:#x} (case {case} w={w} bits={:#010x} \
                 src_id={src_id:#010x}): binary={b:#010x} ours={o:#010x}",
                w.to_bits()
            );
        }
    }
}
