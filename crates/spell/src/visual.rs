//! The timing and scaling of the spell visuals: the emitter rate, the phase accumulator, the chain
//! endpoint height, the screen tint, the fade clock and the chain lifetime
//! (`systems/spell/visuals.md`).

/// 0.33, the float `0x3ea8f5c3` at `0x808300`: the emitter rate factor at `spellEffectLevel` 0.
pub const QUALITY_LOW: f32 = f32::from_bits(0x3ea8_f5c3);
/// 0.66, the float `0x3f28f5c3` at `0x81199c`: the factor at `spellEffectLevel` 1.
pub const QUALITY_MED: f32 = f32::from_bits(0x3f28_f5c3);
/// 0.75, the float `0x3f400000` at `0x8012cc`: the share of the model's height at which a chain
/// attaches by default.
pub const ATTACH_HEIGHT_SCALE: f32 = f32::from_bits(0x3f40_0000);
/// 1.0, the float at `0x7ff9d8`.
pub const ONE: f32 = f32::from_bits(0x3f80_0000);
/// 0.001, the float `0x3a83126f` at `0x801360`: milliseconds to seconds.
pub const MS_TO_SEC: f32 = f32::from_bits(0x3a83_126f);

/// The rate `BlizzardObject::Initialize` (`0x6eb930`, the step `0x6eb95e` to `0x6eb995`) stores at
/// `+0x124`: `scale · 0.33` when the `spellEffectLevel` record's `+0x28` is 0, `scale · 0.66` when it
/// is 1, and `scale` for any other value or no record (`quality` is `None`)
/// (`systems/spell/visuals.md#emitter-initialization`). The product is at 53 bits, stored as `f32`.
pub fn spellvisual_quality_scale(quality: Option<i32>, scale: f32) -> f32 {
    match quality {
        Some(0) => (f64::from(scale) * f64::from(QUALITY_LOW)) as f32,
        Some(1) => (f64::from(scale) * f64::from(QUALITY_MED)) as f32,
        _ => scale,
    }
}

/// The accumulator of `BlizzardObject::Update` (`0x6ebad0`, `0x6ebad6` to `0x6ebaf4`): the value
/// stored at `+0x120`, `dt · rate + phase` at 53 bits rounded to `f32`. The compare with 1.0
/// that decides whether to spawn a shard uses the unrounded sum
/// (`systems/spell/visuals.md#shard-emission-rate`). This is the no-spawn case; the spawn loop
/// is not here.
pub fn spellvisual_phase(phase: f32, rate: f32, dt: f32) -> f32 {
    (f64::from(dt) * f64::from(rate) + f64::from(phase)) as f32
}

/// What `BlizzardObject::Update` (`0x6ebad0`) returns when it spawns nothing (`0x6ebdda` to
/// `0x6ebe05`): true while the rate is not 0.0 (NaN included); with a rate of 0.0, true only when the
/// list head at `+0x140` is non-null and its low bit is clear (`systems/spell/visuals.md#shard-emission-rate`).
pub fn spellvisual_emitter_active(rate: f32, list_head: u32) -> bool {
    if rate != 0.0 {
        true
    } else if list_head & 1 != 0 {
        false
    } else {
        list_head != 0
    }
}

/// `GetLightningCasterPoint` (`0x6ec6f0`) for a model with an attachment manager and no `$CSL`
/// event: the position `base` with z raised by `height · scale · 0.75`, the product at 53 bits
/// rounded to `f32` and the sum rounded again. x and y are copied
/// (`systems/spell/visuals.md#chain-endpoints`).
pub fn spellvisual_attach_height(model_height: f32, model_scale: f32, base: [f32; 3]) -> [f32; 3] {
    let h =
        (f64::from(model_height) * f64::from(model_scale) * f64::from(ATTACH_HEIGHT_SCALE)) as f32;
    let z = (f64::from(h) + f64::from(base[2])) as f32;
    [base[0], base[1], z]
}

/// The level `UpdateSpellScreenTint` (`0x6ec870`, `0x6ec879` to `0x6ec8f1`) passes on, from the
/// tint's times `start`, `mid1`, `mid2` and `end` (`+0x4` to `+0x10`) and the clock `now`
/// (`systems/spell/visuals.md#screen-tint`). The compares are unsigned.
///
/// - Before `mid1`: `(now - start) / (mid1 - start)`, the numerator loaded zero-extended as a
///   64-bit integer and the divisor a signed 32-bit integer.
/// - Before `mid2`: 1.0.
/// - Before `end`: `1.0 - (now - mid2) / (end - mid2)`.
/// - Otherwise 0.0 (the client also clears `start` and `end`).
pub fn spellvisual_intensity(start: u32, mid1: u32, mid2: u32, end: u32, now: u32) -> f32 {
    if now < mid1 {
        let num = f64::from(now.wrapping_sub(start));
        let den = f64::from(mid1.wrapping_sub(start) as i32);
        (num / den) as f32
    } else if now < mid2 {
        1.0
    } else if now < end {
        let num = f64::from(now.wrapping_sub(mid2));
        let den = f64::from(end.wrapping_sub(mid2) as i32);
        (f64::from(ONE) - num / den) as f32
    } else {
        0.0
    }
}

/// The five words `StartSpellScreenTint` (`0x6ec910`) stores at `0xcee8d4` to `0xcee8e4`: the
/// colour `spell_id_field`, `now`, `now + offset`, `now + cast_time` and `now + cast_time + 100`,
/// all wrapping, with `offset = __ftol(cast_time · fraction)`. `cast_time` is loaded zero-extended
/// as a 64-bit integer, multiplied by the float `fraction` at 53 bits, and truncated
/// (`systems/spell/visuals.md#screen-tint`).
pub fn spellvisual_timeline(
    spell_id_field: u32,
    cast_time: i32,
    now: u32,
    fraction: f32,
) -> [u32; 5] {
    let offset = (f64::from(cast_time as u32) * f64::from(fraction)) as i64 as u32;
    let ct = cast_time as u32;
    [
        spell_id_field,
        now,
        now.wrapping_add(offset),
        now.wrapping_add(ct),
        now.wrapping_add(ct).wrapping_add(0x64),
    ]
}

/// The seconds `SpellVisualsUpdate` (`0x6eca20`, `0x6eca8f` to `0x6ecab3`) passes to the renderer:
/// 0.0 when the last time `[0x870e20]` is `0xffffffff` (-1), else `(i32)(now - last) · 0.001` at 53
/// bits rounded to `f32` (`systems/spell/visuals.md#frame-update`).
pub fn spellvisual_fade_clock(now: u32, last: u32) -> f32 {
    if last == 0xffff_ffff {
        0.0
    } else {
        (f64::from(now.wrapping_sub(last) as i32) * f64::from(MS_TO_SEC)) as f32
    }
}

/// The life in seconds `CreateLightningObjects` (`0x6ecbd0`, `0x6ecd61` to `0x6ecd76`) stores at
/// `+0x3c` of a chain: `(i32)duration_ms · 0.001`, with the duration the record's `+0x14`, at 53
/// bits rounded to `f32` (`systems/spell/visuals.md#chain-effect-creation`).
pub fn spellvisual_chain_duration(effect_duration_ms: i32) -> f32 {
    (f64::from(effect_duration_ms) * f64::from(MS_TO_SEC)) as f32
}
