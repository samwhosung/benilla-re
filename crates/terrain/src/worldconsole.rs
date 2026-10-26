//! The world's console handlers and its frame rate (`systems/terrain/world.md#world-console-commands`,
//! `systems/terrain/world.md#smoothed-frame-rate`): the flag toggles (`0x673050`, `0x673250`,
//! `0x673770`, `0x673a30`, `0x673a70`), the simple doodad switch (`0x673ac0`), the `waterMaxLOD`
//! bound (`0x6736c0`) and `GetSmoothedFrameRate` (`0x6724c0`).

/// 1/30 as the client stores it (`0x807334`, `0x3d088889`).
const THIRTIETH: f32 = f32::from_bits(0x3d08_8889);
/// 0.001 (`0x801360`), the least mean frame time a rate is computed from.
const FLOOR: f32 = f32::from_bits(0x3a83_126f);

/// A world flag toggle: a set `bit` is cleared and the handler prints its "disabled" line, a clear
/// one is set with the "enabled" line. Returns the new flags and whether the bit is now set.
pub fn toggle_world_flag(flags: u32, bit: u32) -> (u32, bool) {
    if flags & bit != 0 {
        (flags & !bit, false)
    } else {
        (flags | bit, true)
    }
}

/// `0x673ac0`: a nonzero word becomes 0 ("disabled"), a zero one 1 ("enabled").
pub fn toggle_simple_doodads(word: u32) -> u32 {
    u32::from(word == 0)
}

/// `0x6736c0` after its `%d`: above 4 is 4 and below 0 is 0, signed.
pub fn clamp_water_max_lod(v: i32) -> i32 {
    v.clamp(0, 4)
}

/// `0x6724c0`: the ring's 30 times summed from `start` on, wrapping, from 0.0 at 53 bits, times
/// 1/30; a mean below 0.001 answers 1000.0, any other (a NaN included) `1 / mean`, unrounded.
pub fn smoothed_frame_rate(times: &[f32; 30], start: usize) -> f64 {
    let mut sum = 0.0f64;
    let mut i = start;
    for _ in 0..30 {
        sum += f64::from(times[i]);
        i += 1;
        if i == 30 {
            i = 0;
        }
    }
    let mean = sum * f64::from(THIRTIETH);
    if mean < f64::from(FLOOR) {
        1000.0
    } else {
        1.0 / mean
    }
}
