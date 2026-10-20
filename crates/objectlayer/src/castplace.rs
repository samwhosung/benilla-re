//! The place rungs of the caster validator `CheckCasterStateForSpell` (`0x6094f0`): the
//! indoors-only, outdoors-only and no-mounts refusals (`systems/spell/castgates.md#indoors-and-outdoors-rungs`).
//!
//! The outdoors answer is an input: it is the `al` of `0x612cc0`, which tail-calls
//! `CWorld::IsEntityOutdoors` (`0x6706f0`) on the caster's node (`terrain` holds its
//! transcription).

/// `Attributes` bit 14 (`0x4000`), tested by `0x609f5c` and `0x609f6f`: the spell needs the
/// caster indoors.
pub const ATTR_INDOORS_ONLY: u32 = 0x4000;
/// `Attributes` bit 15 (`0x8000`), tested by `0x609f5c` and `0x609f90`: the spell needs the
/// caster outdoors.
pub const ATTR_OUTDOORS_ONLY: u32 = 0x8000;
/// `Effect` 6, apply aura (`0x609fa1`).
pub const EFFECT_APPLY_AURA: u32 = 6;
/// `EffectApplyAuraName` 78, mounted (`0x609fa6`).
pub const AURA_MOUNTED: u32 = 0x4e;
/// `SPELL_FAILED_ONLY_INDOORS`, reported at `0x609f7c`.
pub const ERR_ONLY_INDOORS: u8 = 0x52;
/// `SPELL_FAILED_ONLY_OUTDOORS`, reported at `0x609fb4`.
pub const ERR_ONLY_OUTDOORS: u8 = 0x55;
/// `SPELL_FAILED_NO_MOUNTS_ALLOWED`, loaded at `0x609fd0`.
pub const ERR_NO_MOUNTS_ALLOWED: u8 = 0x4b;

/// What the rungs do for one spell: whether they ask the outdoors question, and the reason they
/// report, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlaceRung {
    pub asked: bool,
    pub refusal: Option<u8>,
}

/// The slice of `0x6094f0` from `0x609f59` to `0x609fd7`, the rungs between the reputation rung
/// and the taming rung.
///
/// A spell with neither bit skips the question. Otherwise one answer decides both bits: an outdoors
/// answer refuses a spell with bit 14 (0x52), an indoors answer a spell with bit 15, with 0x4b when
/// one of the three effect slots applies the mounted aura and 0x55 when none does. A spell with
/// both bits is refused either way.
#[must_use]
pub fn cast_place_rung(
    attributes: u32,
    effects: [u32; 3],
    aura_names: [u32; 3],
    outdoors: bool,
) -> PlaceRung {
    if attributes & (ATTR_INDOORS_ONLY | ATTR_OUTDOORS_ONLY) == 0 {
        return PlaceRung {
            asked: false,
            refusal: None,
        };
    }
    let refusal = if outdoors {
        (attributes & ATTR_INDOORS_ONLY != 0).then_some(ERR_ONLY_INDOORS)
    } else if attributes & ATTR_OUTDOORS_ONLY != 0 {
        let mounts = effects
            .iter()
            .zip(aura_names)
            .any(|(&e, a)| e == EFFECT_APPLY_AURA && a == AURA_MOUNTED);
        Some(if mounts {
            ERR_NO_MOUNTS_ALLOWED
        } else {
            ERR_ONLY_OUTDOORS
        })
    } else {
        None
    };
    PlaceRung {
        asked: true,
        refusal,
    }
}
