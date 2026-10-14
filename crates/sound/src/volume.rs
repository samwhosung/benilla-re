//! Volume levels and fade rates: the values the client computes before it calls FMOD.
//!
//! The level curve reads the bits of a float, not a float-to-integer conversion: `v · 255 + 512.5`
//! is stored as an `f32` and `(bits >> 14) & 0xff` is the level
//! (`systems/sound/engine.md#volume-of-a-sound`).

/// The level curve: the low byte of `bits(v · 255.0 + 512.5) >> 14`. The sum is formed at 53 bits
/// and stored as an `f32` (`0x7ffe58` is 255.0, `0x81d72c` is 512.5).
fn volume_curve(v: f32) -> u32 {
    let bits = ((v as f64 * 255.0 + 512.5) as f32).to_bits();
    (bits >> 14) & 0xff
}

/// `Sound::GetVolumeLevel` (`0x7a5d70`): 0 for a sound with no channel (`slot == -1`, the field at
/// `+0x30`), else the level curve of the stored volume at `+0x74`
/// (`systems/sound/engine.md#volume-of-a-sound`).
pub fn channel_volume_curve(slot: i32, ch_volume: f32) -> u32 {
    if slot == -1 {
        0
    } else {
        volume_curve(ch_volume)
    }
}

/// The clamp of the category setters (`0x7a6500`): below 0 gives 0, and 1 or above, or NaN, gives 1,
/// since the `jp` after the compare against 1.0 takes the high branch when unordered
/// (`systems/sound/engine.md#category-volumes`).
fn clamp_unit(v: f32) -> f32 {
    // `!(v < 1.0)` and not `v >= 1.0`: NaN must reach the high branch.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    if v < 0.0 {
        0.0
    } else if !(v < 1.0) {
        1.0
    } else {
        v
    }
}

/// `Sound::SetMasterVolume` (`0x7a6500`): the level passed to `FSOUND_SetSFXMasterVolume`, the
/// level curve of the clamped argument (`systems/sound/engine.md#category-volumes`).
pub fn sfx_master_volume(v: f32) -> u32 {
    volume_curve(clamp_unit(v))
}

/// `__ftol` (`0x40a2b0`): truncation toward zero to a 64-bit integer, low 32 bits returned. An
/// unrepresentable value (overflow, infinity, NaN) gives the integer indefinite `0x8000000000000000`,
/// whose low 32 bits are 0.
fn ftol(x: f64) -> u32 {
    let t = x.trunc();
    let i = if (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&t) {
        t as i64
    } else {
        i64::MIN
    };
    i as u32
}

/// The fade rate `Sound::FadeIn` (`0x7a5730`) stores at `+0x48` on its audible path: `level` clamped
/// to 0 to 255, divided by `seconds · 1000.0` (`0x8022cc`) per millisecond. It is only reached for
/// `seconds` of 0.1 or more, or NaN; the clamped level goes to `+0x44`
/// (`systems/sound/engine.md#fades`).
pub fn channel_set_volume_rate(level: i32, seconds: f32) -> f32 {
    let level = level.clamp(0, 255);
    (level as f64 / (seconds as f64 * 1000.0)) as f32
}

/// The rate `Sound::FadeOut` (`0x7a5a50`) stores at `+0x48` on its audible path: the sound's level
/// ([`channel_volume_curve`], stored at `+0x44`) divided by `seconds · -1000.0` (`0x801448`)
/// (`systems/sound/engine.md#fades`).
pub fn channel_volume_rate_stream(slot: i32, ch_volume: f32, seconds: f32) -> f32 {
    let curve = channel_volume_curve(slot, ch_volume);
    (curve as f64 / (seconds as f64 * -1000.0)) as f32
}

/// `Sound::SetVolume2` (`0x7a5dc0`): the level passed to `FSOUND_SetVolume`,
/// `__ftol(c · v · gain · 255.0)` with `gain` the distance gain at `+0x78`. `c` is the music factor
/// (`0x87cef8`) for flag 0x2, else the ambience factor (`0x87cefc`) for 0x8, else 1.0 for 0x10, else
/// the sound factor (`0x87cef4`) (`systems/sound/engine.md#volume-of-a-sound`). The product is
/// formed at 53 bits.
pub fn channel_volume_mix(
    flags: u32,
    v: f32,
    atten: f32,
    music: f32,
    ambience: f32,
    sound: f32,
) -> u32 {
    let cat = if flags & 0x2 != 0 {
        music
    } else if flags & 0x8 != 0 {
        ambience
    } else if flags & 0x10 == 0 {
        sound
    } else {
        1.0
    };
    ftol(cat as f64 * v as f64 * atten as f64 * 255.0)
}

/// `Sound::SetVolume` (`0x7a5da0`): the volume `level · (1/255)` it hands to `Sound::SetVolume2`,
/// with `1/255` the float `0x3b808081` at `0x8026c8` (`systems/sound/engine.md#volume-of-a-sound`).
pub fn volume_ramp_scale(stepcount: i32) -> f32 {
    let k = f32::from_bits(0x3b80_8081) as f64;
    (stepcount as f64 * k) as f32
}

/// `StepSoundDuck` (`0x457960`): the value passed to the SoundVolume setter `0x7a6390`. It runs from
/// `start` to `target` over `dur` milliseconds as `start + (target - start) · (elapsed / dur)`, and
/// is `target` once `elapsed > dur`. With `transitioning` (the ramp down, `[0xb05f34]` set) the
/// start is `music_vol` and the target `music_vol · 0.5` over 500 ms; otherwise (the return) the
/// start is `music_vol · 0.5` and the target `config_volume` over 3000 ms
/// (`systems/sound/objects.md#the-duck`).
///
/// The down ramp's target is stored as an `f32` (`local8`) while its start stays at 53 bits; the
/// return ramp's start `music_vol · 0.5` is never spilled, and its target is a float field.
pub fn music_volume_fade(
    transitioning: bool,
    music_vol: f32,
    config_volume: f32,
    elapsed: i32,
) -> f32 {
    let mv = music_vol as f64;
    let (local8, fvar1, dur): (f64, f64, i32) = if transitioning {
        ((mv * 0.5) as f32 as f64, mv, 500)
    } else {
        (config_volume as f64, mv * 0.5, 3000)
    };
    if elapsed > dur {
        local8 as f32
    } else {
        let t = elapsed as f64 / dur as f64;
        ((local8 - fvar1) * t + fvar1) as f32
    }
}

/// `Sound::FadeInVolume` (`0x7a57b0`): the level `__ftol(v · 255.0)` it passes to `Sound::FadeIn`
/// unclamped (`systems/sound/engine.md#fades`).
pub fn volume_to_byte(v: f32) -> u32 {
    ftol(v as f64 * 255.0)
}

/// The step `Sound::ProcessFadeList` (`0x7a4c20`) computes for a fading sound,
/// `__ftol(elapsed · rate)`. `elapsed` is the 32-bit difference of the clock and the start at
/// `+0x40`, zero-extended into the `fild qword`, so it is never negative; `rate` is the float at
/// `+0x48` (`systems/sound/engine.md#the-fade-pump`).
pub fn ramp_step(now_ms: u32, start_ms: u32, rate: f32) -> u32 {
    let elapsed = now_ms.wrapping_sub(start_ms);
    ftol(elapsed as f64 * rate as f64)
}
