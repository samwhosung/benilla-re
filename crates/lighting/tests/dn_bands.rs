//! The band kernels `FindBandSegment` (`0x6d63e0`), `EvalLightFloatBand` (`0x6d6480`),
//! `EvalSkyTrack` (`0x6cf6c0`) and `EvalLightIntBand` (`0x6d62e0`), and the accumulation
//! `0x6dc470`, each whole, against `dn_band_find_frac`, `dn_band_lerp_scalar`, `dn_track_eval`,
//! `dn_band_lerp_bgra` and `dn_light_accumulate_3x3`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::dn_bands::{
    dn_band_find_frac, dn_band_lerp_bgra, dn_band_lerp_scalar, dn_light_accumulate_3x3,
    dn_track_eval,
};
use common::{assert_f32, next};

/// `count` keys in [0, 2880): ascending and distinct, or, when `malformed`, as drawn.
fn synth_keys(s: &mut u32, count: usize, malformed: bool) -> Vec<i32> {
    let mut keys: Vec<i32> = (0..count).map(|_| ((next(s) >> 8) % 2880) as i32).collect();
    if !malformed {
        keys.sort_unstable();
        keys.dedup();
        while keys.len() < count {
            keys.push((keys.last().copied().unwrap_or(0) + 7) % 2880);
            keys.sort_unstable();
            keys.dedup();
        }
    }
    keys.truncate(count);
    keys
}

/// `0x6d63e0`, with `count` up to 8 keys. Phases on the keys and in the wrap gap, tables that are
/// not ascending (a segment that never matches) and the rest of the fuzz run the match ladder; the
/// three outputs are compared.
#[test]
fn dn_band_find_frac_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let keys_p = emu.alloc(0x80);
    let frac_p = emu.alloc(4);
    let i_p = emu.alloc(4);
    let next_p = emu.alloc(4);
    let mut s = 0x6261_6e64u32;
    for case in 0..500 {
        let count = 1 + ((next(&mut s) >> 8) % 8) as usize;
        let malformed = case % 7 == 6;
        let keys = synth_keys(&mut s, count, malformed);
        let phase = ((next(&mut s) >> 8) % 2880) as i32;
        for (k, v) in keys.iter().enumerate() {
            emu.write_u32(keys_p + 4 * k as u64, *v as u32);
        }
        emu.call(
            0x6d63e0,
            phase as u32,
            keys.len() as u32,
            &[keys_p as u32, frac_p as u32, i_p as u32, next_p as u32],
        );
        let ours = dn_band_find_frac(&keys, keys.len() as i32, phase);
        assert_eq!(emu.read_u32(i_p) as i32, ours.i, "i (case {case})");
        assert_eq!(emu.read_u32(next_p) as i32, ours.next, "next (case {case})");
        assert_f32(
            emu.read_f32(frac_p),
            ours.frac,
            &format!("frac (case {case} keys={keys:?} phase={phase})"),
        );
    }
}

/// `0x6d6480`: the band struct holds the count at `+0x4`, 16 keys at `+0x8` and 16 values at
/// `+0x48`; a count of 0 is among the cases. The result in `st0` is compared as an `f32`.
#[test]
fn dn_band_lerp_scalar_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let track = emu.alloc(0x90);
    let mut s = 0x736b_7972u32;
    for case in 0..500 {
        let count = ((next(&mut s) >> 8) % 9) as usize;
        let keys = synth_keys(&mut s, count.max(1), case % 11 == 10);
        for k in 0..16usize {
            emu.write_u32(track + 8 + 4 * k as u64, *keys.get(k).unwrap_or(&0) as u32);
            let v = f32::from_bits(next(&mut s) & 0x7fbf_ffff);
            emu.write_u32(track + 0x48 + 4 * k as u64, v.to_bits());
        }
        emu.write_u32(track + 4, count as u32);
        let phase = ((next(&mut s) >> 8) % 2880) as i32;
        let binary = emu.call_f32(0x6d6480, phase as u32, track as u32, &[]);
        let values: Vec<f32> = (0..16)
            .map(|k| f32::from_bits(emu.read_u32(track + 0x48 + 4 * k)))
            .collect();
        let ours = dn_band_lerp_scalar(&keys[..count.min(keys.len())], &values, count as i32, phase)
            as f32;
        assert_f32(binary, ours, &format!("lerp (case {case})"));
    }
}

/// A track of `count` entries of key and value, keys in [0, 1]: `mode` 0 ascending, 1 ascending
/// with a coincident pair (the 0.001 guard), 2 ascending with 0 and 1 as the ends, otherwise as
/// drawn.
fn synth_track(s: &mut u32, count: usize, mode: u32) -> Vec<[f32; 2]> {
    let mut keys: Vec<f32> = (0..count)
        .map(|_| (f64::from(next(s) >> 8) / f64::from(1u32 << 24)) as f32)
        .collect();
    match mode {
        0 => {
            keys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        }
        1 => {
            keys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if count >= 2 {
                keys[1] = keys[0];
            }
        }
        2 => {
            keys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if count >= 1 {
                keys[0] = 0.0;
            }
            if count >= 2 {
                let last = keys.len() - 1;
                keys[last] = 1.0;
            }
        }
        _ => {}
    }
    keys.into_iter()
        .map(|k| {
            let v = f32::from_bits(next(s) & 0x7fbf_ffff);
            [k, v]
        })
        .collect()
}

/// `0x6cf6c0`: 1 to 10 keys, in order and not, with a coincident pair and with 0 and 1 as keys;
/// phases below 0, above 1, on a key and between. The result in `st0` is compared as an `f32`.
#[test]
fn dn_track_eval_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let track_p = emu.alloc(0x100);
    let mut s = 0x7472_6b65u32;
    for case in 0..600 {
        let count = 1 + ((next(&mut s) >> 8) % 10) as usize;
        let mode = (next(&mut s) >> 8) % 4;
        let track = synth_track(&mut s, count, mode);
        for (k, e) in track.iter().enumerate() {
            emu.write_u32(track_p + 8 * k as u64, e[0].to_bits());
            emu.write_u32(track_p + 8 * k as u64 + 4, e[1].to_bits());
        }
        let phase = match case % 5 {
            0 => -0.3_f32,
            1 => 1.7_f32,
            2 => track[(next(&mut s) >> 8) as usize % count][0],
            _ => (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32,
        };
        let binary = emu.call_f32(0x6cf6c0, track_p as u32, count as u32, &[phase.to_bits()]);
        let ours = dn_track_eval(&track, count as i32, phase) as f32;
        assert_f32(
            binary,
            ours,
            &format!("track (case {case} mode={mode} count={count} phase={phase})"),
        );
    }

    // The clamp at 0 and 1 and the scan's compare, exactly: NaN of either sign (clamped to 1),
    // both zeros, 1 and the float below it, the infinities and a huge value, on tracks whose
    // keys include 0, 1 and a pair 0.001 apart; and a NaN value, which takes the first arm of
    // the direction compare.
    let phases = [
        f32::NAN,
        -f32::NAN,
        -0.0,
        0.0,
        1.0,
        f32::from_bits(0x3f7f_ffff),
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN_POSITIVE,
        0.5,
    ];
    let tracks: [&[[f32; 2]]; 6] = [
        &[[0.0, 1.0], [0.5, 3.0], [1.0, 2.0]],
        &[[0.25, 1.0], [0.75, -4.0]],
        &[[0.5, 7.0]],
        &[[0.1, 2.0], [0.1009, 5.0], [0.9, 1.0]],
        &[[0.25, f32::NAN], [0.75, 1.0]],
        &[[0.25, 1.0], [0.75, f32::NAN]],
    ];
    for (t, track) in tracks.iter().enumerate() {
        for (k, e) in track.iter().enumerate() {
            emu.write_u32(track_p + 8 * k as u64, e[0].to_bits());
            emu.write_u32(track_p + 8 * k as u64 + 4, e[1].to_bits());
        }
        for &phase in &phases {
            let binary = emu.call_f32(
                0x6cf6c0,
                track_p as u32,
                track.len() as u32,
                &[phase.to_bits()],
            );
            let ours = dn_track_eval(track, track.len() as i32, phase) as f32;
            assert_f32(binary, ours, &format!("track {t} phase={phase}"));
        }
    }
}

/// `0x6d62e0`: the band struct holds the count at `+0x4`, 16 keys at `+0x8` and 16 packed colours
/// at `+0x48`; a count of 0 is among the cases.
#[test]
fn dn_band_lerp_bgra_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let band = emu.alloc(0x90);
    let out_p = emu.alloc(4);
    let mut s = 0x6267_7261u32;
    for case in 0..500 {
        let count = ((next(&mut s) >> 8) % 9) as usize;
        let keys = synth_keys(&mut s, count.max(1), case % 11 == 10);
        let mut values = [0u32; 16];
        for (k, slot) in values.iter_mut().enumerate() {
            emu.write_u32(band + 8 + 4 * k as u64, *keys.get(k).unwrap_or(&0) as u32);
            *slot = next(&mut s);
            emu.write_u32(band + 0x48 + 4 * k as u64, *slot);
        }
        emu.write_u32(band + 4, count as u32);
        emu.write_u32(out_p, 0xdead_beef);
        let phase = ((next(&mut s) >> 8) % 2880) as i32;
        emu.call(0x6d62e0, out_p as u32, phase as u32, &[band as u32]);
        let binary = emu.read_u32(out_p);
        let ours = dn_band_lerp_bgra(&keys[..count.min(keys.len())], &values, count as i32, phase);
        assert_eq!(
            binary, ours,
            "bgra (case {case} count={count} phase={phase}): binary={binary:#010x} ours={ours:#010x}"
        );
    }
}

/// `0x6dc470`: three source vectors, six weights and six running sums in `[-2, 2)`, so the products
/// fall on both sides of each other, and the cases below.
#[test]
fn dn_light_accumulate_3x3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let src_p = [emu.alloc(0x10), emu.alloc(0x10), emu.alloc(0x10)];
    let w_p = emu.alloc(0x18);
    let acc_p = emu.alloc(0x18);
    let mut s = 0x6163_6c33u32;
    let fz = |s: &mut u32| -> f32 {
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
        (m * 4.0 - 2.0) as f32
    };
    for case in 0..400 {
        let mut src = [[0.0f32; 3]; 3];
        for (j, sp) in src_p.iter().enumerate() {
            for (lane, slot) in src[j].iter_mut().enumerate() {
                let v = fz(&mut s);
                *slot = v;
                emu.write_u32(sp + 4 * lane as u64, v.to_bits());
            }
        }
        let mut w = [0.0f32; 6];
        for (k, wv) in w.iter_mut().enumerate() {
            *wv = fz(&mut s);
            emu.write_u32(w_p + 4 * k as u64, wv.to_bits());
        }
        let mut acc = [0.0f32; 6];
        for (k, av) in acc.iter_mut().enumerate() {
            *av = fz(&mut s);
            emu.write_u32(acc_p + 4 * k as u64, av.to_bits());
        }
        emu.call(
            0x6dc470,
            src_p[0] as u32,
            src_p[1] as u32,
            &[src_p[2] as u32, w_p as u32, acc_p as u32],
        );
        dn_light_accumulate_3x3(&src, &w, &mut acc);
        for (k, &a) in acc.iter().enumerate() {
            assert_f32(
                emu.read_f32(acc_p + 4 * k as u64),
                a,
                &format!("acc[{k}] (case {case})"),
            );
        }
    }

    // Equal products (the compare's tie), a NaN in each source and weight position, an infinity
    // times a zero, and signed zeros.
    let mut cases: Vec<([[f32; 3]; 3], [f32; 6])> = vec![
        ([[1.5; 3]; 3], [0.5; 6]),
        ([[0.0; 3]; 3], [0.0; 6]),
        ([[-0.0; 3]; 3], [0.0; 6]),
        ([[f32::INFINITY; 3]; 3], [0.0; 6]),
    ];
    for j in 0..3 {
        for lane in 0..3 {
            let mut src = [[1.0f32; 3]; 3];
            src[j][lane] = f32::NAN;
            cases.push((src, [2.0, 3.0, 4.0, 5.0, 6.0, 7.0]));
        }
    }
    for k in 0..6 {
        let mut w = [2.0f32, 3.0, 4.0, 5.0, 6.0, 7.0];
        w[k] = f32::NAN;
        cases.push(([[1.0; 3]; 3], w));
    }
    for (src, w) in cases {
        for (j, sp) in src_p.iter().enumerate() {
            for (lane, v) in src[j].iter().enumerate() {
                emu.write_u32(sp + 4 * lane as u64, v.to_bits());
            }
        }
        for (k, wv) in w.iter().enumerate() {
            emu.write_u32(w_p + 4 * k as u64, wv.to_bits());
        }
        let mut acc = [0.25f32, -1.0, 8.0, 0.5, 3.0, -2.0];
        for (k, av) in acc.iter().enumerate() {
            emu.write_u32(acc_p + 4 * k as u64, av.to_bits());
        }
        emu.call(
            0x6dc470,
            src_p[0] as u32,
            src_p[1] as u32,
            &[src_p[2] as u32, w_p as u32, acc_p as u32],
        );
        dn_light_accumulate_3x3(&src, &w, &mut acc);
        for (k, &a) in acc.iter().enumerate() {
            assert_f32(
                emu.read_f32(acc_p + 4 * k as u64),
                a,
                &format!("acc[{k}] src={src:?} w={w:?}"),
            );
        }
    }
}

/// `0x6d62e0` and `0x6d6480` with a count of 0: the colour is committed as `0xff000000` and the
/// scalar is 0, without reading the keys or values, which hold the 0xcccccccc filler the shipped
/// tables use. The output cell is poisoned first, so a skipped store would read back.
#[test]
fn dn_band_zero_key_commit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let band = emu.alloc(0x90);
    let out_p = emu.alloc(4);
    for k in 0..16u64 {
        emu.write_u32(band + 8 + 4 * k, 0xcccc_cccc);
        emu.write_u32(band + 0x48 + 4 * k, 0xcccc_cccc);
    }
    emu.write_u32(band + 4, 0);
    for &phase in &[0i32, 1, 719, 1440, 2159, 2879] {
        emu.write_u32(out_p, 0xdead_beef);
        emu.call(0x6d62e0, out_p as u32, phase as u32, &[band as u32]);
        let binary = emu.read_u32(out_p);
        assert_eq!(
            binary, 0xff00_0000,
            "zero-key colour @phase {phase}: binary={binary:#010x} (0xdeadbeef = store skipped)"
        );
        assert_eq!(
            binary,
            dn_band_lerp_bgra(&[], &[], 0, phase),
            "colour vs ours"
        );
        let f = emu.call_f32(0x6d6480, phase as u32, band as u32, &[]);
        assert_eq!(
            f.to_bits(),
            0,
            "zero-key scalar @phase {phase}: binary={f} bits={:#010x}",
            f.to_bits()
        );
        assert_f32(
            f,
            dn_band_lerp_scalar(&[], &[], 0, phase) as f32,
            "scalar vs ours",
        );
    }
}

/// A band of one key is constant in time: `i` and `next` are both 0 for every phase, so the colour
/// and the scalar are the key's value, without extrapolation. The shipped tables hold many such
/// bands.
#[test]
fn dn_band_single_key_constant_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let band = emu.alloc(0x90);
    let out_p = emu.alloc(4);
    for &key_t in &[0i32, 1, 1440, 2879] {
        for k in 0..16u64 {
            emu.write_u32(band + 8 + 4 * k, 0xcccc_cccc);
            emu.write_u32(band + 0x48 + 4 * k, 0xcccc_cccc);
        }
        emu.write_u32(band + 8, key_t as u32);
        emu.write_u32(band + 0x48, 0x0012_3456);
        emu.write_u32(band + 4, 1);
        for phase in (0i32..2880).step_by(97) {
            emu.write_u32(out_p, 0xdead_beef);
            emu.call(0x6d62e0, out_p as u32, phase as u32, &[band as u32]);
            let binary = emu.read_u32(out_p);
            assert_eq!(
                binary, 0xff12_3456,
                "1-key colour key_t={key_t} phase={phase}: binary={binary:#010x}"
            );
            assert_eq!(
                binary,
                dn_band_lerp_bgra(&[key_t], &[0x0012_3456], 1, phase),
                "1-key colour vs ours"
            );
            let f = emu.call_f32(0x6d6480, phase as u32, band as u32, &[]);
            let want = f32::from_bits(0x0012_3456);
            assert_eq!(
                f.to_bits(),
                want.to_bits(),
                "1-key scalar key_t={key_t} phase={phase}"
            );
        }
    }
}

/// The `+512` and `>> 14` repack floors. Two keys 1440 apart sampled at 720 give `frac` exactly
/// 0.5, so a channel pair `(lo, lo + 1)` lands on `lo + 0.5`: flooring gives `lo`, rounding half up
/// gives `lo + 1` for either parity, and rounding half to even gives 2 for both 1 and 2. Both
/// parities run.
#[test]
fn dn_band_repack_half_boundary_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let band = emu.alloc(0x90);
    let out_p = emu.alloc(4);
    for &lo in &[1u32, 2] {
        for k in 0..16u64 {
            emu.write_u32(band + 8 + 4 * k, 0);
            emu.write_u32(band + 0x48 + 4 * k, 0);
        }
        emu.write_u32(band + 8, 0);
        emu.write_u32(band + 12, 1440);
        let v0 = lo | (lo << 8) | (lo << 16);
        let v1 = (lo + 1) | ((lo + 1) << 8) | ((lo + 1) << 16);
        emu.write_u32(band + 0x48, v0);
        emu.write_u32(band + 0x4c, v1);
        emu.write_u32(band + 4, 2);
        emu.write_u32(out_p, 0xdead_beef);
        emu.call(0x6d62e0, out_p as u32, 720, &[band as u32]);
        let binary = emu.read_u32(out_p);
        let want = 0xff00_0000 | v0;
        assert_eq!(
            binary,
            want,
            "repack at exactly lo+0.5 (lo={lo}): binary={binary:#010x} want(floor)={want:#010x}, \
             a rounding repack gives {:#010x}",
            0xff00_0000u32 | v1
        );
        assert_eq!(
            binary,
            dn_band_lerp_bgra(&[0, 1440], &[v0, v1], 2, 720),
            "repack vs ours (lo={lo})"
        );
    }
}
