//! `CM2Model::FindKey` (`0x713d50`) run whole (`systems/animation/tracks.md#key-search`,
//! `#key-fraction`). The exe's code runs on a model with its clock table at `+0x64`, a track and
//! its key times; the result `{k0, k1, t}` is compared bit for bit with `m2_track_search`.
//!
//! Each branch has a deterministic case: a global sequence or the argument as the clock, a range
//! or the whole track, `lo >= hi`, the clamp at the last key, the fraction at 0, mid and near 1,
//! and the scan forward, the scan back, the scan from `lo` and the bisection.

mod common;

use benilla_re_animation::track::{m2_track_search, M2Track, TrackSearchResult};
use benilla_re_difftest::{Emu, Wow};
use common::{assert_f32, next};

/// One synthesized search case run through both the binary and the transcription.
struct Case {
    gseq: u16,
    gseq_times: Vec<u32>,
    range_count: u32,
    ranges: Vec<[u32; 2]>,
    key_count: u32,
    timestamps: Vec<u32>,
    time: u32,
    range_index: u32,
    seed_k0: u32,
}

fn run_case(emu: &mut Emu, arena: u64, c: &Case, ctx: &str) {
    // arena layout: this @ +0 (0x80 bytes), track @ +0x80, ranges @ +0x100, gseq table
    // @ +0x400, timestamps @ +0x800, result @ +0xf00.
    let this = arena;
    let track = arena + 0x80;
    let ranges = arena + 0x100;
    let gseqs = arena + 0x400;
    let times = arena + 0x800;
    let result = arena + 0xf00;

    emu.write_u32(this + 0x64, gseqs as u32);
    emu.write_u32(
        track,
        (u32::from(c.gseq) << 16) | 0x0001, /* interp word 1 (unread by the search) */
    );
    emu.write_u32(track + 4, c.range_count);
    emu.write_u32(track + 8, ranges as u32);
    emu.write_u32(track + 0xc, c.key_count);
    emu.write_u32(track + 0x10, times as u32);
    for (i, r) in c.ranges.iter().enumerate() {
        emu.write_u32(ranges + (i as u64) * 8, r[0]);
        emu.write_u32(ranges + (i as u64) * 8 + 4, r[1]);
    }
    for (i, &g) in c.gseq_times.iter().enumerate() {
        emu.write_u32(gseqs + (i as u64) * 4, g);
    }
    for (i, &t) in c.timestamps.iter().enumerate() {
        emu.write_u32(times + (i as u64) * 4, t);
    }
    // the result block: k0 carries the previous-frame seed IN; k1/t poisoned so a stale
    // value can't fake a match.
    emu.write_u32(result, c.seed_k0);
    emu.write_u32(result + 4, 0xdead_beef);
    emu.write_u32(result + 8, 0xdead_beef);

    emu.clear_calls();
    emu.call(
        0x713d50,
        this as u32,
        0,
        &[c.time, c.range_index, track as u32, result as u32],
    );
    let bin = TrackSearchResult {
        k0: emu.read_u32(result),
        k1: emu.read_u32(result + 4),
        t: emu.read_f32(result + 8),
    };

    let m2 = M2Track {
        interp: 1,
        gseq: c.gseq,
        range_count: c.range_count,
        ranges: &c.ranges,
        key_count: c.key_count,
        timestamps: &c.timestamps,
    };
    let mut ours = TrackSearchResult {
        k0: c.seed_k0,
        k1: 0,
        t: 0.0,
    };
    m2_track_search(&c.gseq_times, c.time, c.range_index, &m2, &mut ours);

    assert_eq!(bin.k0, ours.k0, "{ctx}: k0");
    assert_eq!(bin.k1, ours.k1, "{ctx}: k1");
    assert_f32(bin.t, ours.t, &format!("{ctx}: t"));
}

/// A strictly-increasing timestamp array with chosen gaps (some > 500 so the re-seed and
/// bisect legs are reachable), drawn from the LCG's high bits.
fn gen_timestamps(s: &mut u32, n: u32, big_gaps: bool) -> Vec<u32> {
    let mut v = Vec::with_capacity(n as usize);
    let mut t = (next(s) >> 24) + 1;
    for _ in 0..n {
        v.push(t);
        let gap = if big_gaps {
            600 + (next(s) >> 8) % 2000
        } else {
            1 + (next(s) >> 8) % 120
        };
        t += gap;
    }
    v
}

#[test]
fn m2_track_search_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let arena = emu.alloc(0x1000);

    let mut s = 0x713d_5000u32;
    let mut checked = 0u32;

    // The empty windows.
    for (lo, hi, rc) in [(5u32, 5u32, 1u32), (7, 3, 1), (0, 0, 0)] {
        let c = Case {
            gseq: 0xffff,
            gseq_times: vec![0; 4],
            range_count: rc,
            ranges: vec![[lo, hi]],
            key_count: if rc == 0 { 1 } else { 16 },
            timestamps: gen_timestamps(&mut s, 16, false),
            time: 50,
            range_index: 0,
            seed_k0: lo,
        };
        run_case(
            &mut emu,
            arena,
            &c,
            &format!("degenerate lo={lo} hi={hi} rc={rc}"),
        );
        checked += 1;
    }

    // The backward scan ends at `lo`: every key in `(lo, seed]` is after the time and so is
    // `ts[lo]`. The exe returns `lo` without reading `ts[lo]`; a scan that did not stop there
    // would step below the window and return key 1.
    {
        let c = Case {
            gseq: 0xffff,
            gseq_times: vec![0; 4],
            range_count: 1,
            ranges: vec![[2, 6]],
            key_count: 8,
            timestamps: vec![100, 200, 315, 320, 340, 360, 380, 400],
            time: 310, // delta vs the seed key ts[5]=360 is −50 → the backward leg
            range_index: 0,
            seed_k0: 5,
        };
        run_case(&mut emu, arena, &c, "backward lo-bound exit");
        checked += 1;
    }

    // The scan forward, the scan back, the scan from `lo`, the bisection, the clamp, the global
    // sequence clock and the fraction edges, with ranges and with the whole track.
    for case in 0..400u32 {
        let use_ranges = case % 2 == 0;
        let big_gaps = case % 4 >= 2; // big key spacing makes the far legs reachable
        let n: u32 = 8 + (next(&mut s) >> 8) % 24;
        let timestamps = gen_timestamps(&mut s, n, big_gaps);
        let (range_count, ranges, range_index, lo, hi) = if use_ranges {
            // three ranges partitioning [0, n-1]; pick one.
            let a = n / 3;
            let b = 2 * n / 3;
            let ranges = vec![[0, a], [a, b], [b, n - 1]];
            let idx = (next(&mut s) >> 8) % 3;
            let r = ranges[idx as usize];
            (3u32, ranges, idx, r[0], r[1])
        } else {
            (0u32, vec![[0, 0]], 0, 0, n - 1)
        };

        // the driven leg, cycled deterministically (never LCG low bits).
        let leg = case % 5;
        let t_lo = timestamps[lo as usize];
        let t_hi = timestamps[hi as usize];
        let span = t_hi - t_lo;
        let (time, seed_k0) = match leg {
            // forward: seed at lo, time a little above the seed key.
            0 => (t_lo + (next(&mut s) >> 8) % 499, lo),
            // backward: seed at hi, time within 500 BELOW the seed key (>= t_lo).
            1 => {
                let back = 1 + (next(&mut s) >> 8) % 499;
                let time = timestamps[hi as usize].saturating_sub(back).max(t_lo);
                (time, hi)
            }
            // re-seed forward: seed key far above time, time near t_lo.
            2 => (t_lo + (next(&mut s) >> 8) % 400, hi),
            // bisect: time far above t_lo (needs big gaps), seed far below.
            3 => (t_lo + span / 2 + (next(&mut s) >> 8) % (span / 4 + 1), lo),
            // clamp / exact-key / near-1 fractions: time on or past keys.
            _ => match case % 3 {
                0 => (t_hi + 1 + (next(&mut s) >> 8) % 1000, lo), // above last
                1 => (timestamps[(lo + (hi - lo) / 2) as usize], lo), // exactly on a key
                _ => {
                    let k = (lo + (hi - lo) / 2) as usize;
                    (timestamps[k + 1].wrapping_sub(1), lo) // near-1 fraction
                }
            },
        };

        // The dual time base: half the cases key off a global sequence — the pushed
        // time is then a decoy the binary must ignore.
        let use_gseq = case % 8 >= 4;
        let (gseq, gseq_times, pushed_time) = if use_gseq {
            let gi = (next(&mut s) >> 8) % 4;
            let mut gt = vec![0u32; 4];
            gt[gi as usize] = time;
            (gi as u16, gt, 0xbad0_0bad)
        } else {
            (0xffffu16, vec![0; 4], time)
        };

        let c = Case {
            gseq,
            gseq_times,
            range_count,
            ranges,
            key_count: n,
            timestamps,
            time: pushed_time,
            range_index,
            seed_k0,
        };
        run_case(
            &mut emu,
            arena,
            &c,
            &format!("case {case} leg {leg} gseq {use_gseq}"),
        );
        checked += 1;
    }

    eprintln!("m2_track_search: {checked} searches bit-exact vs WoW.exe @ 0x713d50");
}
