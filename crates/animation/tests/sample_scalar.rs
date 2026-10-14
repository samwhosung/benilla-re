//! `M2SampleByteTrack` (`0x71ae90`) and `M2SampleFloatTrack` (`0x71af20`) run whole
//! (`systems/animation/tracks.md#sampling-block`) on a model whose `+0x64` holds the clocks, a
//! bone state (`+0x98`, `+0x9c`, `+0xc4`, `+0xc8` the clocks, `+0x10c` the weight), a track, its
//! values and a state block; the values and the search results are compared bit for bit.
//!
//! Both samplers start their searches from `out+0` and `out+0x10`, so each case writes the same
//! starting keys to the emulator and to the state. The cases come from the bits of the case
//! number: a step or an interpolated track, a zero, positive, negative or NaN weight, no global
//! sequence or a real one (the clock and the blend gate), and the fraction at an exact key, mid,
//! near 1 and clamped.

mod common;

use benilla_re_animation::sample_scalar::{
    m2_track_sample_byte, m2_track_sample_float, ByteSampleState, FloatSampleState,
};
use benilla_re_animation::track::{M2Track, TrackSearchResult, TrackTimes};
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, fuzz_f32, next};

// arena layout (0x1000): this @ +0 (+0x64 gseq table ptr), bone-state @ +0x100, track @ +0x380,
// ranges @ +0x400 (unused — single-range fallback), gseq table @ +0x440, timestamps @ +0x500,
// payload @ +0x700, out-state @ +0x900.
const BONE: u64 = 0x100;
const TRACK: u64 = 0x380;
const RANGES: u64 = 0x400;
const GSEQ: u64 = 0x440;
const TIMES: u64 = 0x500;
const PAYLOAD: u64 = 0x700;
const OUT: u64 = 0x900;

/// The synthesized inputs shared by both samplers (payload supplied separately by type).
struct Inputs {
    interp: u16,
    gseq: u16,
    lambda: f32,
    gseq_times: Vec<u32>,
    key_count: u32,
    timestamps: Vec<u32>,
    time: u32,
    gtime: u32,
    blend_time: u32,
    blend_gtime: u32,
    seed_primary: u32,
    seed_secondary: u32,
}

/// A strictly-increasing timestamp array with small-ish gaps (the search-leg domain itself is the
/// `tests/track.rs` covers the search; here every search lands cleanly, so the sampler's own paths
/// are isolated).
fn gen_timestamps(s: &mut u32, n: u32) -> Vec<u32> {
    let mut v = Vec::with_capacity(n as usize);
    let mut t = 1 + (next(s) >> 24);
    for _ in 0..n {
        v.push(t);
        t += 5 + (next(s) >> 8) % 300;
    }
    v
}

/// Pick an effective search time for key index `ki` per the fraction mode (0 exact, 1 mid,
/// 2 near-1, 3 clamp past the last key).
fn eff_time(ts: &[u32], ki: u32, mode: u32) -> u32 {
    let k = ki as usize;
    match mode {
        0 => ts[k],
        1 => (ts[k] + ts[k + 1]) / 2,
        2 => ts[k + 1].saturating_sub(1),
        _ => ts[ts.len() - 1] + 100,
    }
}

/// Build a deterministic Inputs for case `case`, drawing values (never branch decisions) from the
/// LCG. `step_word_zero` selects whether the interp word is 0 (byte: step-only fetch path; float:
/// step-copy leg).
fn make_inputs(s: &mut u32, case: u32) -> Inputs {
    let interp_zero = case & 1 == 0;
    let gseq_real = case & 2 != 0;
    let lambda_pos = case & 4 != 0;
    let frac = (case >> 3) % 4;

    let n = 8 + (next(s) >> 8) % 16; // 8..=23 keys
    let timestamps = gen_timestamps(s, n);
    let ki = 1 + (next(s) >> 8) % (n - 3);
    let et = eff_time(&timestamps, ki, frac);

    // secondary (blend) clock: an independent key index, always mid-fraction, in-domain.
    let ki2 = 1 + (next(s) >> 8) % (n - 3);
    let et2 = eff_time(&timestamps, ki2, 1);

    // dual time base: a real global-sequence id routes the clock through [this+0x64][id]; the
    // pushed time becomes a decoy. (gseq != 0xffff also forces the blend leg off in both fns.)
    let mut gseq_times = vec![0u32; 8];
    let (gseq, time) = if gseq_real {
        let gi = 1 + (next(s) >> 8) % 6;
        gseq_times[gi as usize] = et;
        (gi as u16, 0xbad0_0bad)
    } else {
        (0xffffu16, et)
    };

    // The weight: positive, negative or NaN (each runs the blend leg when the interp word is not 0
    // and the track has no global sequence), or 0 (skips it).
    let lambda = if lambda_pos {
        0.001 + (next(s) >> 8) as f32 / (u32::MAX >> 8) as f32
    } else if frac == 1 {
        -0.5
    } else if frac == 2 {
        f32::NAN
    } else {
        0.0
    };

    Inputs {
        interp: if interp_zero { 0 } else { 1 },
        gseq,
        lambda,
        gseq_times,
        key_count: n,
        timestamps,
        time,
        gtime: 0,
        blend_time: et2,
        blend_gtime: 0,
        seed_primary: 0,
        seed_secondary: 0,
    }
}

/// Write the parts of the arena common to both samplers (everything but the typed payload).
fn write_common(emu: &mut Emu, arena: u64, inp: &Inputs) {
    emu.write_u32(arena + 0x64, (arena + GSEQ) as u32);
    emu.write_u32(arena + BONE + 0x98, inp.time);
    emu.write_u32(arena + BONE + 0x9c, inp.gtime);
    emu.write_u32(arena + BONE + 0xc4, inp.blend_time);
    emu.write_u32(arena + BONE + 0xc8, inp.blend_gtime);
    emu.write_u32(arena + BONE + 0x10c, inp.lambda.to_bits());

    emu.write_u32(
        arena + TRACK,
        (u32::from(inp.gseq) << 16) | u32::from(inp.interp),
    );
    emu.write_u32(arena + TRACK + 4, 0); // range_count = 0 ⇒ single-range fallback [0, n-1]
    emu.write_u32(arena + TRACK + 8, (arena + RANGES) as u32);
    emu.write_u32(arena + TRACK + 0xc, inp.key_count);
    emu.write_u32(arena + TRACK + 0x10, (arena + TIMES) as u32);
    emu.write_u32(arena + TRACK + 0x18, (arena + PAYLOAD) as u32);

    for (i, &g) in inp.gseq_times.iter().enumerate() {
        emu.write_u32(arena + GSEQ + (i as u64) * 4, g);
    }
    for (i, &t) in inp.timestamps.iter().enumerate() {
        emu.write_u32(arena + TIMES + (i as u64) * 4, t);
    }

    // out block: k0 / k0' carry the search seeds IN; the rest poisoned so stale state can't fake
    // a match.
    emu.write_u32(arena + OUT, inp.seed_primary);
    emu.write_u32(arena + OUT + 4, 0xdead_beef);
    emu.write_u32(arena + OUT + 8, 0xdead_beef);
    emu.write_u32(arena + OUT + 0xc, 0xa5a5_a5a5);
    emu.write_u32(arena + OUT + 0x10, inp.seed_secondary);
    emu.write_u32(arena + OUT + 0x14, 0xdead_beef);
    emu.write_u32(arena + OUT + 0x18, 0xdead_beef);
    emu.write_u32(arena + OUT + 0x1c, 0x5a5a_5a5a);
}

fn read_result(emu: &Emu, base: u64) -> TrackSearchResult {
    TrackSearchResult {
        k0: emu.read_u32(base),
        k1: emu.read_u32(base + 4),
        t: emu.read_f32(base + 8),
    }
}

fn rust_times(inp: &Inputs) -> TrackTimes {
    TrackTimes {
        time: inp.time,
        gtime: inp.gtime,
        blend_time: inp.blend_time,
        blend_gtime: inp.blend_gtime,
        lambda: inp.lambda,
    }
}

#[test]
fn m2_track_sample_byte_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let arena = emu.alloc(0x1000);
    let mut s = 0x71ae_9000u32;
    let mut checked = 0u32;

    for case in 0..96u32 {
        let inp = make_inputs(&mut s, case);
        let payload: Vec<u8> = (0..inp.key_count)
            .map(|_| (next(&mut s) >> 24) as u8)
            .collect();

        write_common(&mut emu, arena, &inp);
        emu.write(arena + PAYLOAD, &payload);

        emu.clear_calls();
        emu.call(
            0x71ae90,
            arena as u32,
            (arena + BONE) as u32,
            &[(arena + TRACK) as u32, (arena + OUT) as u32],
        );
        let bin_primary = read_result(&emu, arena + OUT);
        let bin_value = (emu.read_u32(arena + OUT + 0xc) & 0xff) as u8;
        let bin_secondary = read_result(&emu, arena + OUT + 0x10);
        let bin_secondary_value = (emu.read_u32(arena + OUT + 0x1c) & 0xff) as u8;

        let m2 = M2Track {
            interp: inp.interp,
            gseq: inp.gseq,
            range_count: 0,
            ranges: &[],
            key_count: inp.key_count,
            timestamps: &inp.timestamps,
        };
        let mut ours = ByteSampleState {
            primary: TrackSearchResult {
                k0: inp.seed_primary,
                ..Default::default()
            },
            secondary: TrackSearchResult {
                k0: inp.seed_secondary,
                ..Default::default()
            },
            ..Default::default()
        };
        m2_track_sample_byte(&inp.gseq_times, &rust_times(&inp), &m2, &payload, &mut ours);

        let blend = inp.interp != 0 && inp.lambda != 0.0 && inp.gseq == 0xffff;
        let ctx = format!("byte case {case} blend={blend}");
        assert_eq!(bin_primary, ours.primary, "{ctx}: primary triple");
        assert_eq!(bin_value, ours.value, "{ctx}: value");
        if blend {
            assert_eq!(bin_secondary, ours.secondary, "{ctx}: secondary triple");
            assert_eq!(
                bin_secondary_value, ours.secondary_value,
                "{ctx}: secondary value"
            );
        }
        checked += 1;
    }

    eprintln!("m2_track_sample_byte: {checked} samples bit-exact vs WoW.exe @ 0x71ae90");
}

#[test]
fn m2_track_sample_float_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let arena = emu.alloc(0x1000);
    let mut s = 0x71af_2000u32;
    let mut checked = 0u32;

    for case in 0..96u32 {
        let inp = make_inputs(&mut s, case);
        let payload: Vec<f32> = (0..inp.key_count).map(|_| fuzz_f32(&mut s)).collect();

        write_common(&mut emu, arena, &inp);
        for (i, &f) in payload.iter().enumerate() {
            emu.write_u32(arena + PAYLOAD + (i as u64) * 4, f.to_bits());
        }

        emu.clear_calls();
        emu.call(
            0x71af20,
            arena as u32,
            (arena + BONE) as u32,
            &[(arena + TRACK) as u32, (arena + OUT) as u32],
        );
        let bin_primary = read_result(&emu, arena + OUT);
        let bin_value = emu.read_f32(arena + OUT + 0xc);
        let bin_secondary = read_result(&emu, arena + OUT + 0x10);
        let bin_secondary_value = emu.read_f32(arena + OUT + 0x1c);

        let m2 = M2Track {
            interp: inp.interp,
            gseq: inp.gseq,
            range_count: 0,
            ranges: &[],
            key_count: inp.key_count,
            timestamps: &inp.timestamps,
        };
        let mut ours = FloatSampleState {
            primary: TrackSearchResult {
                k0: inp.seed_primary,
                ..Default::default()
            },
            secondary: TrackSearchResult {
                k0: inp.seed_secondary,
                ..Default::default()
            },
            ..Default::default()
        };
        m2_track_sample_float(&inp.gseq_times, &rust_times(&inp), &m2, &payload, &mut ours);

        let blend = inp.interp != 0 && inp.lambda != 0.0 && inp.gseq == 0xffff;
        let ctx = format!("float case {case} interp={} blend={blend}", inp.interp);
        assert_eq!(bin_primary, ours.primary, "{ctx}: primary triple");
        assert_f32(bin_value, ours.value, &format!("{ctx}: value"));
        if blend {
            assert_eq!(bin_secondary, ours.secondary, "{ctx}: secondary triple");
            assert_f32(
                bin_secondary_value,
                ours.secondary_value,
                &format!("{ctx}: secondary value"),
            );
        }
        checked += 1;
    }

    eprintln!("m2_track_sample_float: {checked} samples bit-exact vs WoW.exe @ 0x71af20");
}
