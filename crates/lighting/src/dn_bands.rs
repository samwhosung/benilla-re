//! The band kernels every day-night value flows through: `FindBandSegment` (`0x6d63e0`,
//! `systems/lighting/daynight.md#key-search`), `EvalLightFloatBand` (`0x6d6480`,
//! `daynight.md#scalar-band-value`), `EvalLightIntBand` (`0x6d62e0`,
//! `daynight.md#colour-band-value`) and `EvalSkyTrack` (`0x6cf6c0`,
//! `systems/lighting/celestial.md#tracks`), and the weighted accumulation `0x6dc470`, whose object
//! file is not identified. A band's time is in half-minutes of a day of 2880.

/// One day in half-minutes: `add eax,0xb40` in the search.
pub const DAY_HALF_MINUTES: i32 = 0xb40;

/// What `FindBandSegment` writes: the fraction, the segment's first key index and its next index.
pub struct BandFind {
    pub frac: f32,
    pub i: i32,
    pub next: i32,
}

/// `0x6d63e0` (`ecx` = the half-minute, `edx` = the key count; the keys, `&frac`, `&i` and `&next`
/// on the stack; `ret 0x10`): clears the three outputs, then for each `i` takes `next = (i + 1) %
/// count` (signed `idiv`). A segment with `keys[next] > keys[i]` matches when `keys[i] <= phase <=
/// keys[next]`. Any other segment wraps and matches unless `keys[next] < phase < keys[i]`; then
/// `keys[next]` gains 2880 and, when `phase < keys[i]`, so does the phase. The fraction is the
/// integer `(phase − keys[i]) / (keys[next] − keys[i])` divided at 53 bits and stored as `f32`.
/// With no match the loop ends with `i == count` and `frac == 0`; a count of 0 or less clears the
/// outputs only. All compares are signed.
pub fn dn_band_find_frac(keys: &[i32], count: i32, phase: i32) -> BandFind {
    let mut out = BandFind {
        frac: 0.0,
        i: 0,
        next: 0,
    };
    if count <= 0 {
        return out;
    }
    while out.i < count {
        let cur = keys[out.i as usize];
        out.next = (out.i + 1) % count;
        let nxt = keys[out.next as usize];
        let (matched, nxt_adj, phase_adj) = if nxt > cur {
            (phase >= cur && phase <= nxt, nxt, phase)
        } else {
            let m = !(phase > nxt && phase < cur);
            let p = if m && phase < cur {
                phase + DAY_HALF_MINUTES
            } else {
                phase
            };
            (m, nxt + DAY_HALF_MINUTES, p)
        };
        if matched {
            out.frac = (f64::from(phase_adj - cur) / f64::from(nxt_adj - cur)) as f32;
            return out;
        }
        out.i += 1;
    }
    out
}

/// `0x6d6480` (`edx` = the band, whose count is at `+0x4`, keys at `+0x8` and `f32` values at
/// `+0x48`; `ecx` = the half-minute): a count of exactly 0 gives 0; otherwise, with `i`, `next` and
/// `frac` from `dn_band_find_frac`, `(values[next] − values[i])·frac + values[i]` at 53 bits, left
/// in `st0`.
pub fn dn_band_lerp_scalar(keys: &[i32], values: &[f32], count: i32, phase: i32) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let f = dn_band_find_frac(keys, count, phase);
    let vi = f64::from(values[f.i as usize]);
    let vn = f64::from(values[f.next as usize]);
    (vn - vi) * f64::from(f.frac) + vi
}

/// `0x6cf6c0` (`ecx` = the track of `count` entries of key and value, `edx` = the count, the phase
/// on the stack; `ret 4`; the result in `st0`). The phase is clamped: below 0 it is 0, and anything
/// not below 1, NaN included, is 1 (`test ah,5; jp` then `jnp`). The scan takes the first key with
/// `phase <= key` (`test ah,0x41; jnp`; NaN does not stop it); with none, `i` is 0 and `prev` the
/// last key, and `i == 0` also takes the last as `prev`. Then:
/// - `|key[i] − key[prev]| < 0.001` (the `f32` `[0x801360]`) returns `value[prev]`;
/// - a negative `delta` gains 1, and `num = phase − key[prev]`, if negative, gains 1;
/// - with `frac = num/delta` at 53 bits, `value[i] < value[prev]` or an unordered pair (`test ah,1;
///   jne`) gives `vp − (vp − vi)·frac`, otherwise `vp + (vi − vp)·frac`.
pub fn dn_track_eval(track: &[[f32; 2]], count: i32, phase: f32) -> f64 {
    let phase = if phase < 0.0 {
        0.0f32
    } else if phase < 1.0 {
        phase
    } else {
        1.0f32
    };

    let count = count.max(0);
    let mut i: i32 = 0;
    while i < count {
        let k = track[i as usize][0];
        if phase <= k {
            break;
        }
        i += 1;
    }

    let prev: i32;
    if i == count {
        i = 0;
        prev = count - 1;
    } else if i == 0 {
        prev = count - 1;
    } else {
        prev = i - 1;
    }

    let ki = f64::from(track[i as usize][0]);
    let kp = f64::from(track[prev as usize][0]);
    let mut delta = ki - kp;

    if delta.abs() < f64::from(0.001f32) {
        return f64::from(track[prev as usize][1]);
    }

    if delta < 0.0 {
        delta += 1.0;
    }
    let mut num = f64::from(phase) - kp;
    if num < 0.0 {
        num += 1.0;
    }
    let frac = num / delta;

    let vi = f64::from(track[i as usize][1]);
    let vp = f64::from(track[prev as usize][1]);
    if vi >= vp {
        vp + (vi - vp) * frac
    } else {
        vp - (vp - vi) * frac
    }
}

/// `[0x8029cc]`, 512: the bias that puts a byte in the high bits of an `f32`.
const BGRA_REPACK_BIAS: f32 = 512.0;

/// `0x6d62e0` (`ecx` = out, `edx` = the half-minute, the band on the stack; `ret 4`): a count of
/// exactly 0 stores `0xff000000`. Otherwise, with `i`, `next` and `frac` from `dn_band_find_frac`,
/// each of the three low bytes of the values is `(hi − lo)·frac + lo + 512` at 53 bits, stored as
/// `f32`, and the result byte is `bits >> 14` of that store, low byte. The pixel is `0xff000000 |
/// byte2 << 16 | byte1 << 8 | byte0`.
pub fn dn_band_lerp_bgra(keys: &[i32], values: &[u32], count: i32, phase: i32) -> u32 {
    if count == 0 {
        return 0xff00_0000;
    }
    let f = dn_band_find_frac(keys, count, phase);
    let vi = values[f.i as usize];
    let vn = values[f.next as usize];
    let frac = f.frac;
    let chan = |k: u32| -> u32 {
        let lo = ((vi >> (8 * k)) & 0xff) as i32;
        let hi = ((vn >> (8 * k)) & 0xff) as i32;
        let v = (f64::from(hi - lo) * f64::from(frac) + f64::from(lo) + f64::from(BGRA_REPACK_BIAS))
            as f32;
        (v.to_bits() >> 14) & 0xff
    };
    0xff00_0000 | (chan(2) << 16) | (chan(1) << 8) | chan(0)
}

/// `0x6dc470` (`ecx`, `edx` and the first stack argument the three vectors, then the six weights
/// and the six sums; `ret 0xc`). For each lane and each source `j`: `a = src[j][lane]·w[j]` stays
/// in the register and `b = src[j][lane]·w[j + 3]` is stored as `f32`. When `a < b` (ordered; the
/// `jp` after `test ah,5` falls through only then) `sum[lane] += a` and the larger is `b`;
/// otherwise, NaN included, `sum[lane] += b` and the larger is `a`. Then `sum[lane + 3] +=` the
/// larger. Each sum is stored as `f32`.
pub fn dn_light_accumulate_3x3(src: &[[f32; 3]; 3], w: &[f32; 6], acc: &mut [f32; 6]) {
    for lane in 0..3 {
        for j in 0..3 {
            let s = f64::from(src[j][lane]);
            let a = s * f64::from(w[j]);
            let b = (s * f64::from(w[j + 3])) as f32;
            let larger: f64 = if a < f64::from(b) {
                acc[lane] = (a + f64::from(acc[lane])) as f32;
                f64::from(b)
            } else {
                acc[lane] = (f64::from(b) + f64::from(acc[lane])) as f32;
                a
            };
            acc[lane + 3] = (larger + f64::from(acc[lane + 3])) as f32;
        }
    }
}
