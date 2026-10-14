//! The byte and float track samplers `M2SampleByteTrack` (`0x71ae90`) and `M2SampleFloatTrack`
//! (`0x71af20`) (`systems/animation/tracks.md#sampling-block`). Both are `fastcall` with `ecx` the
//! model (whose `+0x64` holds the global-sequence clocks) and `edx` the bone state, and both
//! search the key range with the primary clock and, on the blend leg, with the blend clock
//! ([`m2_track_search`]); each search starts from the previous search's first key (`out+0`,
//! `out+0x10`).
//!
//! The byte sampler takes the byte at `k`; on the blend leg it also fetches the byte at the blend
//! search's `k` into `out+0x1c`, and never blends. The float sampler copies the float at `k` for an
//! interpolation word of 0, else takes `a + (b - a) · t`, and on the blend leg blends toward the
//! second value by the weight. The sums run at 53 bits and narrow at each `fstp dword`; the
//! blend's `fst` stores the second value as a float while the 53-bit value stays on the x87 stack
//! and feeds the blend.

use crate::track::{m2_track_search, M2Track, TrackSearchResult, TrackTimes};

/// The byte track state (`[ebp+0xc]`): the primary search result, the byte at `+0xc`, the
/// blend search result at `+0x10` and the blend byte at `+0x1c`. `primary.k0` and `secondary.k0`
/// carry the previous search's first key in.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct ByteSampleState {
    pub primary: TrackSearchResult,
    pub value: u8,
    pub secondary: TrackSearchResult,
    pub secondary_value: u8,
}

/// `0x71ae90`. `payload` is `[track+0x18]`, one byte per key.
pub fn m2_track_sample_byte(
    gseq_times: &[u32],
    times: &TrackTimes,
    track: &M2Track,
    payload: &[u8],
    state: &mut ByteSampleState,
) {
    // The primary search (`0x71aea0`), with the clock `(+0x98, +0x9c)`.
    m2_track_search(
        gseq_times,
        times.time,
        times.gtime,
        track,
        &mut state.primary,
    );

    // The byte at `k` (`0x71aeb7`).
    state.value = payload[state.primary.k0 as usize];

    // The blend leg (`0x71aec6`): the interpolation word is not 0, the weight is not 0 and the
    // track has no global sequence. `fld 0.0; fcomp λ; test ah,0x44; jnp` skips only an equal
    // compare, so a negative or NaN weight runs the leg. The second byte is fetched, not blended.
    if track.interp != 0 && times.lambda != 0.0 && track.gseq == 0xffff {
        m2_track_search(
            gseq_times,
            times.blend_time,
            times.blend_gtime,
            track,
            &mut state.secondary,
        );
        state.secondary_value = payload[state.secondary.k0 as usize];
    }
}

/// The float track state (`[ebp+0xc]`): the primary search result, the float at `+0xc`, the
/// blend search result at `+0x10` and the second float at `+0x1c`. `primary.k0` and
/// `secondary.k0` carry the previous search's first key in.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct FloatSampleState {
    pub primary: TrackSearchResult,
    pub value: f32,
    pub secondary: TrackSearchResult,
    pub secondary_value: f32,
}

/// `0x71af20`. `payload` is `[track+0x18]`, one float per key.
pub fn m2_track_sample_float(
    gseq_times: &[u32],
    times: &TrackTimes,
    track: &M2Track,
    payload: &[f32],
    state: &mut FloatSampleState,
) {
    // The primary search (`0x71af30`).
    m2_track_search(
        gseq_times,
        times.time,
        times.gtime,
        track,
        &mut state.primary,
    );

    // The interpolation word (`0x71af47`): 0 is a step, any other value is linear.
    if track.interp == 0 {
        // The step leg (`0x71af4d`) copies the float at `k`, a dword move, and returns.
        state.value = payload[state.primary.k0 as usize];
        return;
    }

    // The linear leg (`0x71af61`): `a + (b - a) · t` at 53 bits, narrowed once.
    let a = f64::from(payload[state.primary.k0 as usize]);
    let b = f64::from(payload[state.primary.k1 as usize]);
    let t = f64::from(state.primary.t);
    let primary_value = ((b - a) * t + a) as f32;
    state.value = primary_value;

    // The blend leg (`0x71af7b`): the weight is not 0 and the track has no global sequence.
    // `fld 0.0; fcomp λ; test ah,0x44; jnp` skips only an equal compare, so a negative or NaN
    // weight runs the leg.
    if times.lambda != 0.0 && track.gseq == 0xffff {
        m2_track_search(
            gseq_times,
            times.blend_time,
            times.blend_gtime,
            track,
            &mut state.secondary,
        );
        let a2 = f64::from(payload[state.secondary.k0 as usize]);
        let b2 = f64::from(payload[state.secondary.k1 as usize]);
        let t2 = f64::from(state.secondary.t);
        // `fst dword [esi+0x1c]` stores the second value as a float, and the 53-bit value stays
        // on `st0` for the blend.
        let second_value = (b2 - a2) * t2 + a2;
        state.secondary_value = second_value as f32;

        // `0x71afca`: `fld [esi+0xc]` reloads the first value as a float, and
        // `value = prev + (second - prev) · λ` uses the 53-bit second value.
        let prev = f64::from(primary_value);
        state.value = ((second_value - prev) * f64::from(times.lambda) + prev) as f32;
    }
}
