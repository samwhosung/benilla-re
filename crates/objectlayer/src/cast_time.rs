//! `CGPlayer_C::GetSpellCastTime` (`0x5ee150`), vtable slot 45: a spell's cast time for a player
//! (`systems/objectlayer/playercombat.md#cast-time`).
//!
//! The record lookups (`Spell.dbc` through `0xc0d788`, `SpellCastTimes.dbc` through `0xc0d878`)
//! and the level call (`vtable+0xa8`) are inputs here. The active player's time also goes through
//! `ApplySpellModifierInt(spell, &time, 10)` (`0x5ee1f4` to `0x5ee1fb`), a separate function that
//! writes the time back; this transcription is the time of a unit that is not the active player.
//!
//! The speed multiply is `fld m32; fimul m32int; fstp m32; fld m32; fistp m32int`: a product at
//! 53 bits, rounded to `f32` by the first store, then rounded to nearest-even into a dword.

use crate::x87::fistp;

/// The unity the speed's distance is measured from, `1.0` at `0x7ff9d8`.
pub const SPEED_ONE: f32 = 1.0;
/// The speed dead band, `2^-22` (bits `0x34800000`) at `0x8029d4`.
pub const SPEED_EPS: f32 = f32::from_bits(0x3480_0000);

/// The `Spell.dbc` fields the function reads.
pub struct SpellRec {
    /// `+0x18`, a byte: `& 0x30` skips the speed, `& 0x2` adds 500.
    pub flags: u8,
    /// `+0x48`: the index into `SpellCastTimes.dbc`; out of range gives 0.
    pub cast_time_idx: i32,
    /// `+0x70`: the spell level subtracted from the caster's level term.
    pub spell_level: i32,
}

/// The `SpellCastTimes.dbc` fields the function reads.
pub struct CastTimeRec {
    /// `+0x04`: the base time.
    pub base: i32,
    /// `+0x08`: the time added per level.
    pub per_level: i32,
    /// `+0x0c`: the floor, compared signed.
    pub min: u32,
}

/// The cast time of a spell. A `None` spell stands for the early-outs at `0x5ee150` (id out of
/// range, or a null record) and a `None` cast time for those at `0x5ee182` (index out of range, or
/// a null record); both return 0 before the level call. `level` is the `vtable+0xa8` result and
/// `speed` is `*(this+0x110)+0x22c`.
pub fn player_cast_time(
    spell: Option<&SpellRec>,
    cast: Option<&CastTimeRec>,
    level: u32,
    speed: f32,
) -> u32 {
    let Some(spell) = spell else { return 0 };
    let Some(cast) = cast else { return 0 };

    // `(level / 5 - spell_level) * per_level + base`, unsigned `/ 5` then 32-bit wrapping.
    let mut time = (level / 5)
        .wrapping_sub(spell.spell_level as u32)
        .wrapping_mul(cast.per_level as u32)
        .wrapping_add(cast.base as u32);

    // `cmp edx,esi; jge`: a signed compare raises the time to the floor.
    if (time as i32) < (cast.min as i32) {
        time = cast.min;
    }

    // The speed applies to a positive time unless Attributes has 0x10 or 0x20. `fcomp eps;
    // test ah,5; jnp` skips only when `|speed - 1| < eps` is ordered and less; equal, greater and
    // NaN all multiply.
    let near_one = (f64::from(speed) - f64::from(SPEED_ONE)).abs() < f64::from(SPEED_EPS);
    if spell.flags & 0x30 == 0 && (time as i32) > 0 && !near_one {
        let scaled = (f64::from(speed) * f64::from(time as i32)) as f32;
        time = fistp(f64::from(scaled)) as u32;
    }

    if spell.flags & 0x2 != 0 {
        time = time.wrapping_add(500);
    }

    if (time as i32) < 1 {
        0
    } else {
        time
    }
}
