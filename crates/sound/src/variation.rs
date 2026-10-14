//! The per-play volume and pitch variation. Both take a draw from the client's shared generator
//! (`0x455c70`); the maths here is the transform of that draw
//! (`systems/sound/entries.md#volume-and-pitch-of-a-play`).

/// The per-step volume scale 0.01, the float `0x3c23d70a` at `0x8029d0`.
const VAR_VOL_K: f64 = f32::from_bits(0x3c23_d70a) as f64;

/// The clamp of `GetSoundDefPlayVolume` (`0x458c60`): 0 for a value at or below 0 or NaN (the
/// `jne` after the compare against 0 takes the unordered case), 1 for a value at or above 1.
fn clamp01_lo(v: f64) -> f64 {
    // `!(v > 0.0)` and not `v <= 0.0`: NaN must reach the low branch.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if !(v > 0.0) {
        0.0
    } else if v >= 1.0 {
        1.0
    } else {
        v
    }
}

/// `GetSoundDefPlayVolume` (`0x458c60`): the play volume, `base` plus `(draw - 15) · 0.01` when
/// the definition's flag 0x800 gives a draw (`Some`) and `base` alone otherwise, times `param2`,
/// clamped to 0 to 1. Computed at 53 bits and returned in `st0`, which the caller rounds to `f32`.
pub fn variation_volume(draw: Option<i32>, base: f32, param2: f32) -> f32 {
    let f = match draw {
        None => base as f64,
        Some(rng) => (rng - 0xf) as f64 * VAR_VOL_K + base as f64,
    };
    clamp01_lo(f * param2 as f64) as f32
}

/// The frequency `ApplySoundDefVolumePitch` (`0x458da0`) passes to `FSOUND_SetFrequency` through
/// `0x7a5e40` when flag 0x400 is set: `22050 · (draw + 85) / 100`. The product is a 32-bit `imul`
/// (22050 is the integer at `0x835a70`) and the division truncates (`imul 0x51eb851f` and a sign
/// correction).
pub fn variation_pitch_freq(draw: i32) -> i32 {
    22050i32.wrapping_mul(draw.wrapping_add(0x55)) / 100
}
