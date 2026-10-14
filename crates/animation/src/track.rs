//! `CM2Model::FindKey` (`0x713d50`), the key search: the window from the track's range, the
//! clock (a global sequence's or the argument), the scan or bisection from the previous search's
//! key, and the fraction (`systems/animation/tracks.md#key-search`, `#key-fraction`).
//!
//! Every key-time comparison is unsigned (`ja`, `jae`, `jb`). The fraction is `fild qword` of the
//! zero-extended numerator divided by `fidiv dword` of the signed denominator, narrowed by one
//! `fstp dword`.

/// The header of an M2 track: `+0x0` the interpolation word, `+0x2` the global sequence word,
/// `+0x4` the range count, `+0x8` the ranges, `+0xc` the key count and `+0x10` the timestamps.
/// The search never reads the values at `+0x18`; each sampler takes them as its own argument.
pub struct M2Track<'a> {
    pub interp: u16,
    pub gseq: u16,
    pub range_count: u32,
    /// `[track+8]`: the `(lo, hi)` key indices of each range, 8 bytes each.
    pub ranges: &'a [[u32; 2]],
    pub key_count: u32,
    /// `[track+0x10]`: the key times.
    pub timestamps: &'a [u32],
}

/// The result `{k0, k1, t}` at `[ebp+0x14]`. `k0` is read on entry: the search starts from the
/// previous search's first key.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct TrackSearchResult {
    pub k0: u32,
    pub k1: u32,
    pub t: f32,
}

/// What the samplers read of a bone state (`systems/animation/clock.md#bone-state`): the primary
/// clock `(+0x98, +0x9c)`, the blend clock `(+0xc4, +0xc8)` and the blend weight at `+0x10c`.
#[derive(Clone, Copy, Default)]
pub struct TrackTimes {
    pub time: u32,
    pub gtime: u32,
    pub blend_time: u32,
    pub blend_gtime: u32,
    pub lambda: f32,
}

/// `0x713d50`. `gseq_times` is the clock table `[this+0x64]`, `time` the argument `[ebp+8]`, and
/// `range_index` selects the `(lo, hi)` pair when the track has ranges. `result.k0` carries the
/// previous search's first key in; `{k0, k1, t}` come out.
pub fn m2_track_search(
    gseq_times: &[u32],
    time: u32,
    range_index: u32,
    track: &M2Track,
    result: &mut TrackSearchResult,
) {
    // The window (`0x713d59` to `0x713d7b`): the range's `(lo, hi)`, else `[0, count - 1]`.
    let (lo, hi) = if track.range_count != 0 {
        let r = track.ranges[range_index as usize];
        (r[0], r[1])
    } else {
        (0, track.key_count.wrapping_sub(1))
    };

    // `lo >= hi`, unsigned (`0x713d7b`): the result is `{lo, lo, 0}`.
    if lo >= hi {
        *result = TrackSearchResult {
            k0: lo,
            k1: lo,
            t: 0.0,
        };
        return;
    }

    // The clock (`0x713d96` to `0x713dba`): a track with a global sequence uses its clock,
    // otherwise the argument.
    let time = if track.gseq != 0xffff {
        gseq_times[track.gseq as usize]
    } else {
        time
    };
    let ts = track.timestamps;
    let prev = result.k0;
    let prev_key = ts[prev as usize];

    // The search (`0x713dbd` to `0x713e43`): a scan from the previous key when the time is within
    // 500 above or below it, else a scan from `lo` or a bisection.
    let delta = time.wrapping_sub(prev_key);
    let found = if delta < 0x1f4 {
        scan_forward(ts, prev, hi, time)
    } else if delta >= 0xffff_fe0c {
        scan_backward(ts, prev, lo, time)
    } else {
        let d2 = time.wrapping_sub(ts[lo as usize]);
        if d2 < 0x1f4 {
            scan_forward(ts, lo, hi, time)
        } else {
            bisect(ts, lo, hi, time)
        }
    };

    // The fraction (`0x713e45` to `0x713e84`): `{k, k, 0}` at the end of the key array, else
    // `(time - t0) / (t1 - t0)` with a 64-bit numerator and a signed 32-bit denominator,
    // narrowed once at the store.
    let k1 = found.wrapping_add(1);
    if k1 >= track.key_count {
        *result = TrackSearchResult {
            k0: found,
            k1: found,
            t: 0.0,
        };
        return;
    }
    let t0 = ts[found as usize];
    let t1 = ts[k1 as usize];
    let num = f64::from(time.wrapping_sub(t0));
    let den = f64::from(t1.wrapping_sub(t0) as i32);
    *result = TrackSearchResult {
        k0: found,
        k1,
        t: (num / den) as f32,
    };
}

/// The forward scan (`0x713dc6` to `0x713ddd`): advances while `k < hi` and the next key is at or
/// before `time`.
fn scan_forward(ts: &[u32], from: u32, hi: u32, time: u32) -> u32 {
    let mut k = from;
    while k < hi && ts[(k + 1) as usize] <= time {
        k += 1;
    }
    k
}

/// The backward scan (`0x713de8` to `0x713dfd`): steps down while `k > lo` and the key is after
/// `time`. It reads `ts[k]` only for `k` in `(lo, seed]` and ends at `lo` without reading
/// `ts[lo]`.
fn scan_backward(ts: &[u32], from: u32, lo: u32, time: u32) -> u32 {
    let mut k = from;
    while k > lo && ts[k as usize] > time {
        k -= 1;
    }
    k
}

/// The bisection (`0x713e21` to `0x713e43`) over `[lo, hi]`: it ends at `mid` when
/// `ts[mid] <= time < ts[mid + 1]`, and at `lo` when the window collapses.
fn bisect(ts: &[u32], lo: u32, hi: u32, time: u32) -> u32 {
    let mut lo = lo;
    let mut hi = hi;
    while lo < hi {
        let mid = (hi + lo) >> 1;
        if time < ts[mid as usize] {
            hi = mid.wrapping_sub(1);
        } else if time < ts[(mid + 1) as usize] {
            return mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}
