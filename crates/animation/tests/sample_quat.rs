//! `M2SampleRotation` (`0x713ea0`) and the quaternion blend `0x74d11a`
//! (`systems/animation/tracks.md#rotation-sample`, `#quaternion-blend`) run whole on a model, a
//! bone state, a track and a state block, and compared bit for bit with `m2_track_sample_quat`
//! and `quat_slerp`. The emulator evaluates `fpatan` and `fsin` with the host's maths library,
//! as `f64::atan2` and `f64::sin` do in the transcription.
//!
//! The sampler: the step copy, the interpolation, a zero, positive, negative and NaN weight, a
//! global sequence on the track, and the blend leg. The blend: a negative dot, the linear
//! fallback at `1 - c <= 2^-23`, and the sine path, with non-normalized inputs in the sweep.

mod common;

use benilla_re_animation::sample_quat::{
    m2_track_sample_quat, quat_slerp, QuatSampleState, QuatTrack,
};
use benilla_re_animation::track::{TrackSearchResult, TrackTimes};
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, next};

/// A unit-ish quaternion drawn from the LCG high bits (so the slerp `1−cosθ²` stays in domain).
fn norm_quat(s: &mut u32) -> [f32; 4] {
    let mut q = [0f64; 4];
    let mut n = 0f64;
    for c in q.iter_mut() {
        let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24); // [0,1)
        *c = m * 2.0 - 1.0; // [-1,1)
        n += *c * *c;
    }
    if n <= 0.0 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    let inv = 1.0 / n.sqrt();
    [
        (q[0] * inv) as f32,
        (q[1] * inv) as f32,
        (q[2] * inv) as f32,
        (q[3] * inv) as f32,
    ]
}

fn read_vec4(emu: &Emu, va: u64) -> [f32; 4] {
    [
        emu.read_f32(va),
        emu.read_f32(va + 4),
        emu.read_f32(va + 8),
        emu.read_f32(va + 0xc),
    ]
}

fn write_quat(emu: &mut Emu, va: u64, q: [f32; 4]) {
    for (i, &c) in q.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
}

// ===========================================================================
// 0x713ea0 m2_track_sample_quat
// ===========================================================================

/// One synthesized sampler case.
struct SampleCase {
    interp: u16,
    gseq: u16,
    /// the [+0x64] global-sequence time table.
    gseq_times: Vec<u32>,
    /// strictly-increasing key timestamps.
    timestamps: Vec<u32>,
    /// the 16-byte-stride quaternion payload (one per key).
    quats: Vec<[f32; 4]>,
    times: TrackTimes,
    /// out-block seeds: primary k0 (+0) and blend k0 (+0x1c).
    seed_primary: u32,
    seed_blend: u32,
}

fn run_sample(emu: &mut Emu, regions: &Regions, c: &SampleCase, ctx: &str) {
    let Regions {
        model,
        bone,
        track,
        gseqs,
        times,
        quats,
        out,
        ..
    } = *regions;

    // The model (`ecx`): the clock table at `+0x64`.
    emu.write_u32(model + 0x64, gseqs as u32);
    for (i, &g) in c.gseq_times.iter().enumerate() {
        emu.write_u32(gseqs + (i as u64) * 4, g);
    }

    // The track: `+0` the interpolation word and `+2` the global sequence word, `+4` the range
    // count (0), `+8` the ranges (unused), `+0xc` the key count, `+0x10` the key times and
    // `+0x18` the quaternions.
    emu.write_u32(track, (u32::from(c.gseq) << 16) | u32::from(c.interp));
    emu.write_u32(track + 4, 0); // single-range fallback: [0, key_count-1]
    emu.write_u32(track + 8, 0);
    emu.write_u32(track + 0xc, c.timestamps.len() as u32);
    emu.write_u32(track + 0x10, times as u32);
    emu.write_u32(track + 0x18, quats as u32);
    for (i, &t) in c.timestamps.iter().enumerate() {
        emu.write_u32(times + (i as u64) * 4, t);
    }
    for (i, &q) in c.quats.iter().enumerate() {
        write_quat(emu, quats + (i as u64) * 16, q);
    }

    // The bone state (`edx`): the clocks `(+0x98, +0x9c)` and `(+0xc4, +0xc8)`, and the weight
    // at `+0x10c`.
    emu.write_u32(bone + 0x98, c.times.time);
    emu.write_u32(bone + 0x9c, c.times.gtime);
    emu.write_u32(bone + 0xc4, c.times.blend_time);
    emu.write_u32(bone + 0xc8, c.times.blend_gtime);
    emu.write_u32(bone + 0x10c, c.times.lambda.to_bits());

    // The state block: the starting keys at `+0` and `+0x1c`; the rest is poisoned.
    for o in 0..(0x40u64 / 4) {
        emu.write_u32(out + o * 4, 0xdead_beef);
    }
    emu.write_u32(out, c.seed_primary);
    emu.write_u32(out + 0x1c, c.seed_blend);

    // __fastcall: ecx = model, edx = bone state, stack = (track, out). ret 8.
    emu.call(
        0x713ea0,
        model as u32,
        bone as u32,
        &[track as u32, out as u32],
    );
    let bin = read_vec4(emu, out + 0xc);

    // The transcription.
    let qt = QuatTrack {
        interp: c.interp,
        gseq: c.gseq,
        range_count: 0,
        ranges: &[],
        key_count: c.timestamps.len() as u32,
        timestamps: &c.timestamps,
        quats: &c.quats,
    };
    let mut st = QuatSampleState {
        primary: TrackSearchResult {
            k0: c.seed_primary,
            ..Default::default()
        },
        blend: TrackSearchResult {
            k0: c.seed_blend,
            ..Default::default()
        },
        ..Default::default()
    };
    m2_track_sample_quat(&c.gseq_times, &c.times, &qt, &mut st);

    for (i, (&bv, &ov)) in bin.iter().zip(st.value.iter()).enumerate() {
        assert_f32(bv, ov, &format!("{ctx}: value[{i}]"));
    }
}

/// Region pointers for the sampler arena (one alloc each, kept disjoint).
#[derive(Clone, Copy)]
struct Regions {
    model: u64,
    bone: u64,
    track: u64,
    gseqs: u64,
    times: u64,
    quats: u64,
    out: u64,
}

#[test]
fn m2_track_sample_quat_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // The thunk `0x74d114` is `jmp [0x876588]`, and the slot starts at the stub `0x74d0ee`, which
    // selects the routines on the first call. The exe's own selection fills it with `0x74d11a`
    // (`systems/models/animation.md#dispatched-matrix-math`).
    benilla_re_difftest::select_math_slots(&mut emu);
    assert_eq!(emu.read_u32(0x876588), 0x74d11a, "blend slot");
    let regions = Regions {
        model: emu.alloc(0x80),
        bone: emu.alloc(0x140),
        track: emu.alloc(0x40),
        gseqs: emu.alloc(0x40),
        times: emu.alloc(0x80),
        quats: emu.alloc(0x200),
        out: emu.alloc(0x40),
    };

    let n: u32 = 8;
    let timestamps: Vec<u32> = (0..n).map(|i| i * 100).collect(); // gaps of 100 (< 500)
    let t_first = timestamps[0];
    let t_last = timestamps[(n - 1) as usize];

    let mut s = 0x713e_a000u32;
    let mut checked = 0u32;
    let (mut copy, mut lerp, mut gseqtb, mut slerp_leg) = (0u32, 0u32, 0u32, 0u32);

    for case in 0..600u32 {
        let quats: Vec<[f32; 4]> = (0..n).map(|_| norm_quat(&mut s)).collect();
        // a time landing strictly inside [first, last) so the search brackets two keys (k1=k0+1).
        let span = t_last - t_first;
        let time = t_first + 1 + (next(&mut s) >> 8) % (span - 1);
        let blend_time = t_first + 1 + (next(&mut s) >> 8) % (span - 1);

        // 6 modes, cycled deterministically (never LCG low bits).
        let mode = case % 6;
        let (interp, gseq, lambda, gseq_times) = match mode {
            // copy leg: interp == 0, gseq direct, λ ignored.
            0 => (0u16, 0xffffu16, 0.0f32, vec![0u32; 4]),
            // lerp leg, no blend (λ == 0).
            1 => (1, 0xffff, 0.0, vec![0; 4]),
            // lerp leg, global-sequence time base (the search keys off the table).
            2 => {
                let gi = (next(&mut s) >> 8) % 4;
                let mut gt = vec![0u32; 4];
                gt[gi as usize] = time; // the table supplies the search key
                (1, gi as u16, 0.5, gt) // a global sequence on the track skips the blend leg
            }
            // a negative weight runs the blend leg.
            3 => (1, 0xffff, -0.5, vec![0; 4]),
            // a NaN weight runs the blend leg (`0x713f4f test ah,0x44`).
            5 => (1, 0xffff, f32::NAN, vec![0; 4]),
            // slerp blend leg: interp==1, gseq==0xffff, λ ∈ (0,1).
            _ => {
                let lam =
                    0.05 + (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32 * 0.9;
                (1, 0xffff, lam, vec![0; 4])
            }
        };

        let c = SampleCase {
            interp,
            gseq,
            gseq_times,
            timestamps: timestamps.clone(),
            quats,
            times: TrackTimes {
                time,
                gtime: 0, // range-index (unused: range_count == 0)
                blend_time,
                blend_gtime: 0,
                lambda,
            },
            seed_primary: 0,
            seed_blend: 0,
        };
        run_sample(
            &mut emu,
            &regions,
            &c,
            &format!("sample case {case} mode {mode}"),
        );
        match mode {
            0 => copy += 1,
            2 => gseqtb += 1,
            4 => slerp_leg += 1,
            _ => lerp += 1,
        }
        checked += 1;
    }

    eprintln!(
        "m2_track_sample_quat: {checked} cases bit-exact vs WoW.exe @ 0x713ea0 \
         (copy={copy} lerp={lerp} gseq-timebase={gseqtb} slerp-blend={slerp_leg})"
    );
}

// ===========================================================================
// 0x74d11a quat_slerp  (standalone)
// ===========================================================================

#[test]
fn sample_quat_slerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out = emu.alloc(0x20);
    let qa = emu.alloc(0x20);
    let qb = emu.alloc(0x20);

    let run = |emu: &mut Emu,
               a: [f32; 4],
               b: [f32; 4],
               t: f32,
               ctx: &str,
               counts: &mut (u32, u32, u32)| {
        write_quat(emu, qa, a);
        write_quat(emu, qb, b);
        for o in 0..4u64 {
            emu.write_u32(out + o * 4, 0xdead_beef);
        }
        // cdecl: ecx/edx ignored; stack = (out, a, b, t-bits). ret 0x10.
        emu.call(
            0x74d11a,
            0,
            0,
            &[out as u32, qa as u32, qb as u32, t.to_bits()],
        );
        let bin = read_vec4(emu, out);
        let ours = quat_slerp(a, b, t);

        for (i, (&bv, &ov)) in bin.iter().zip(ours.iter()).enumerate() {
            assert_f32(bv, ov, &format!("{ctx}: out[{i}]"));
        }

        // leg accounting (the binary's own dot order, so the counters name the leg that ran).
        let p = |i: usize| f64::from(a[i]) * f64::from(b[i]);
        let d = ((p(0) + p(2)) + p(1)) + p(3);
        let cos_reg = d * if d >= 0.0 { 1.0 } else { -1.0 };
        if d < 0.0 {
            counts.0 += 1; // sign flip exercised
        }
        if (1.0 - cos_reg) > f64::from(f32::from_bits(0x3400_0000)) {
            counts.2 += 1; // general slerp path
        } else {
            counts.1 += 1; // lerp fallback (≤ eps, or unordered)
        }
    };

    let mut counts = (0u32, 0u32, 0u32);

    // A negative dot and the sine path: orthogonal and opposed unit pairs.
    let structured: &[([f32; 4], [f32; 4], f32)] = &[
        // 90° apart (general path), positive dot small.
        ([1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], 0.5),
        // opposed → dot<0 sign flip.
        ([1.0, 0.0, 0.0, 0.0], [-0.6, -0.8, 0.0, 0.0], 0.25),
        ([0.0, 0.0, 1.0, 0.0], [0.0, 0.0, -1.0, 0.0], 0.5), // antiparallel (dot=-1)
        // 45°-ish, t at the ends and middle.
        (
            [1.0, 0.0, 0.0, 0.0],
            [0.707_106_77, 0.707_106_77, 0.0, 0.0],
            0.0,
        ),
        (
            [1.0, 0.0, 0.0, 0.0],
            [0.707_106_77, 0.707_106_77, 0.0, 0.0],
            1.0,
        ),
        (
            [1.0, 0.0, 0.0, 0.0],
            [0.707_106_77, 0.707_106_77, 0.0, 0.0],
            0.5,
        ),
        // Near-parallel, the linear fallback (`1 - c <= 2^-23`).
        ([1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], 0.3), // identical (cos=1)
        ([0.0, 1.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], 0.7),
        ([1.0, 0.0, 0.0, 0.0], [-1.0, 0.0, 0.0, 0.0], 0.4), // sign-flip → near-parallel
        ([0.5, 0.5, 0.5, 0.5], [0.5, 0.5, 0.5, 0.5], 0.5),  // unit, identical
    ];
    for &(a, b, t) in structured {
        run(&mut emu, a, b, t, "slerp structured", &mut counts);
    }

    // Fuzz: mostly normalized pairs (the sine path, with the linear fallback and the sign flip
    // now and then), some near-parallel, and some non-normalized (NaN results compare equal).
    let mut s = 0x74d1_1a00u32;
    for case in 0..600u32 {
        let a = norm_quat(&mut s);
        let kind = case % 8;
        let b = match kind {
            // near-parallel: b = a (+ optional sign flip) → the lerp fallback.
            0 => a,
            1 => [-a[0], -a[1], -a[2], -a[3]],
            // Non-normalized and small, so `1 / sin θ` does not blow up. A dot above 1 gives a
            // NaN.
            6 | 7 => {
                let mut q = [0f32; 4];
                for c in q.iter_mut() {
                    let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
                    *c = ((m * 2.0 - 1.0) * 0.5) as f32;
                }
                q
            }
            // An independent unit quaternion with `|cos θ| <= 0.9`, away from the linear fallback
            // and from the near-antiparallel blow-up of `1 / sin θ`.
            _ => {
                let mut q;
                loop {
                    q = norm_quat(&mut s);
                    let dot: f64 = (0..4).map(|i| f64::from(a[i]) * f64::from(q[i])).sum();
                    if dot.abs() <= 0.9 {
                        break;
                    }
                }
                q
            }
        };
        // t across [-0.25, 1.25] to exercise extrapolation (the body never clamps t).
        let t = ((f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) * 1.5 - 0.25) as f32;
        run(
            &mut emu,
            a,
            b,
            t,
            &format!("slerp fuzz {case} kind {kind}"),
            &mut counts,
        );
    }

    let _ = next(&mut s);
    eprintln!(
        "quat_slerp: structured+fuzz cases bit-exact vs WoW.exe @ 0x74d11a \
         (sign-flip={} lerp-fallback={} general-slerp={})",
        counts.0, counts.1, counts.2
    );
}
