//! The rotation track sampler `M2SampleRotation` (`0x713ea0`) and the quaternion blend
//! `BlendQuaternionDefault` (`0x74d11a`, reached through the slot `0x876588`)
//! (`systems/animation/tracks.md#rotation-sample`, `#quaternion-blend`).
//!
//! The sampler searches the key range ([`m2_track_search`]), then copies the key at `k` for an
//! interpolation word of 0, or takes the componentwise `a + (b - a) · t` of the two keys. When the
//! weight is not 0 and the track has no global sequence, it searches the blend clock, interpolates
//! the blend keys the same way and blends the first result toward them by the weight, in place.
//!
//! The x87 runs at 53 bits, so a sum is an `f64` chain with `as f32` at each `fstp dword`. The
//! blend's `fsqrt` is exact; `fpatan` and `fsin` differ by CPU, and the emulator evaluates them
//! with the host's maths library, as `f64::atan2` and `f64::sin` do here.

use crate::track::{m2_track_search, M2Track, TrackSearchResult, TrackTimes};

/// `[0x818868]`, 2^-23: the blend falls back to a linear blend when `1 - c` is at or below it.
const FLT_EPS: f64 = f32::from_bits(0x3400_0000) as f64;

/// A rotation track: the fields of [`M2Track`], and at `+0x18` the quaternions, 16 bytes per
/// key.
pub struct QuatTrack<'a> {
    pub interp: u16,
    pub gseq: u16,
    pub range_count: u32,
    pub ranges: &'a [[u32; 2]],
    pub key_count: u32,
    pub timestamps: &'a [u32],
    /// `[track+0x18]`: the quaternions.
    pub quats: &'a [[f32; 4]],
}

impl QuatTrack<'_> {
    /// The track as the key search reads it.
    fn search_view(&self) -> M2Track<'_> {
        M2Track {
            interp: self.interp,
            gseq: self.gseq,
            range_count: self.range_count,
            ranges: self.ranges,
            key_count: self.key_count,
            timestamps: self.timestamps,
        }
    }
}

/// The track state (`systems/animation/tracks.md#sampling-block`). `primary.k0` and `blend.k0`
/// carry the previous search's first key in, where the next search starts.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct QuatSampleState {
    /// `+0x0`: the primary search result.
    pub primary: TrackSearchResult,
    /// `+0xc`: the sampled quaternion, replaced by the blend result.
    pub value: [f32; 4],
    /// `+0x1c`: the blend search result.
    pub blend: TrackSearchResult,
    /// `+0x28`: the interpolated blend quaternion.
    pub blend_value: [f32; 4],
}

/// `0x713ea0`, a `fastcall` with `ecx` the model (`gseq_times` is its `+0x64` clocks) and `edx` the
/// bone state (`times`); the stack arguments are the track and the state.
pub fn m2_track_sample_quat(
    gseq_times: &[u32],
    times: &TrackTimes,
    track: &QuatTrack,
    state: &mut QuatSampleState,
) {
    let view = track.search_view();

    // The primary search, with the clock `(+0x98, +0x9c)`.
    m2_track_search(
        gseq_times,
        times.time,
        times.gtime,
        &view,
        &mut state.primary,
    );
    let k0 = state.primary.k0 as usize;

    // The interpolation word at `[track+0]`.
    if track.interp == 0 {
        // The key at `k` is copied, and there is no blend leg.
        state.value = track.quats[k0];
        return;
    }

    // `out[i] = a[i] + (b[i] - a[i]) · t`, with no renormalization.
    let a = track.quats[k0];
    let b = track.quats[state.primary.k1 as usize];
    state.value = lerp4(a, b, state.primary.t);

    // The blend leg (`0x713f47`): `fld 0.0; fcomp λ; fnstsw ax; test ah,0x44; jnp`. An equal
    // compare sets C3 alone, an odd parity, so the leg is skipped only when the weight is 0; a
    // negative or NaN weight runs it. The track must have no global sequence (`0x713f5b`).
    if times.lambda != 0.0 && track.gseq == 0xffff {
        // The blend search, with the clock `(+0xc4, +0xc8)`.
        m2_track_search(
            gseq_times,
            times.blend_time,
            times.blend_gtime,
            &view,
            &mut state.blend,
        );
        let ab = track.quats[state.blend.k0 as usize];
        let bb = track.quats[state.blend.k1 as usize];
        state.blend_value = lerp4(ab, bb, state.blend.t);
        // The value is blended toward the blend quaternion by the weight, in place.
        state.value = quat_slerp(state.value, state.blend_value, times.lambda);
    }
}

/// `out[i] = a[i] + (b[i] - a[i]) · t` (`fld b; fsub a; fmul t; fadd a; fstp`): the difference,
/// product and sum stay at 53 bits and the store narrows once.
fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let tf = f64::from(t);
    let mut out = [0f32; 4];
    for i in 0..4 {
        let diff = f64::from(b[i]) - f64::from(a[i]);
        out[i] = (diff * tf + f64::from(a[i])) as f32;
    }
    out
}

/// `0x74d11a` (`cdecl`, `out, a, b, t`; `ret 0x10`), the routine the slot `0x876588` holds
/// (`systems/animation/tracks.md#quaternion-blend`):
/// - `d = ((a0·b0 + a2·b2) + a1·b1) + a3·b3`, and the sign is -1 when `d < 0` or unordered
///   (`fcom 0.0; test ah,1`), else 1; `c = d · sign` stays a 53-bit register, and `fst dword
///   [ebp+0x10]` stores the copy `fpatan` reads.
/// - When `1 - c > 2^-23` is false (`test ah,0x41; jne`, so at or below it, or NaN), the
///   coefficients are `1 - t` and `t · sign`.
/// - Otherwise `s = sqrt(1 - c²)`, `θ = f32(atan2(s, f32(c)))` (stored at `[ebp+0x10]` and
///   reloaded for both products), `inv = f32(1 / s)` (stored at `[ebp-4]`), and the
///   coefficients are `sin(θ · (1 - t)) · inv` and `sin(θ · t) · inv · sign`, with `1 - t` the
///   53-bit register of entry (`fld1; fsub dword [ebp+0x14]`). Neither is stored as a float.
/// - Each output is `f32(coeffA · a[i] + coeffB · b[i])`.
///
/// The sign multiplies the coefficient of `b` (`fmul [ebp+0xc]` at `0x74d1bc`).
pub fn quat_slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    // `d = ((p0 + p2) + p1) + p3`.
    let p = |i: usize| f64::from(a[i]) * f64::from(b[i]);
    let d = ((p(0) + p(2)) + p(1)) + p(3);

    // `fcom 0.0; test ah,1`: C0 is set for `d < 0` or unordered, giving -1.0.
    let sign = if d >= 0.0 { 1.0_f64 } else { -1.0_f64 };

    // The 53-bit `c` serves the epsilon test and `c²`; the stored float is what `atan2` reads.
    let cos_reg = d * sign;
    let cos_f32 = f64::from(cos_reg as f32);

    let tf = f64::from(t);
    let one_minus_t = 1.0 - tf; // `fld1; fsub dword [ebp+0x14]`, a 53-bit register

    // `fcomp eps; test ah,0x41; jne` takes the linear blend when `1 - c` is at or below the
    // epsilon or unordered, so the test is the negation of `>`; `<=` would send a NaN to the
    // sine path.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !((1.0 - cos_reg) > FLT_EPS) {
        // The linear blend: `1 - t` and `t · sign`.
        return blend4(a, b, one_minus_t, tf * sign);
    }

    // `s = sqrt(1 - c²)` at 53 bits; `θ` and `inv` are narrowed to floats by their stores, and both
    // products reload the floats.
    let s = (1.0 - cos_reg * cos_reg).sqrt();
    let theta = f64::from(s.atan2(cos_f32) as f32);
    let inv = f64::from((1.0 / s) as f32);
    let coeff_a = (theta * one_minus_t).sin() * inv;
    let coeff_b = (theta * tf).sin() * inv * sign;
    blend4(a, b, coeff_a, coeff_b)
}

/// `out[i] = f32(coeffA · a[i] + coeffB · b[i])`, the sum at 53 bits.
fn blend4(a: [f32; 4], b: [f32; 4], coeff_a: f64, coeff_b: f64) -> [f32; 4] {
    let mut out = [0f32; 4];
    for i in 0..4 {
        out[i] = (coeff_a * f64::from(a[i]) + coeff_b * f64::from(b[i])) as f32;
    }
    out
}
