//! What an entity's map object group says about it: the group claim of `SetEntityLightFlags`
//! (`0x6a87f0`, `systems/terrain/mapentities.md#group-claim-flags`) and the outdoors test of
//! `CWorld::IsEntityOutdoors` (`0x6706f0`, `systems/terrain/world.md#entity-outdoors-test`), which
//! reads the bit the claim sets and the flags of the group `FindMapObjectGroupAt` (`0x69d4f0`)
//! finds.

/// `[entity + 0x90]` bit 0: the entity stands in a map object group without the exterior flag.
pub const ENTITY_INDOORS: u32 = 0x1;
/// Group flag `0x8`: the exterior group (`0x6a880d`).
pub const GROUP_EXTERIOR: u32 = 0x8;
/// Group flag `0x40`: the exterior-lit group (`0x6a8823`).
pub const GROUP_EXTERIOR_LIT: u32 = 0x40;
/// Group flag `0x8000`, bit 15: what `0x670735` returns for an entity with the indoors bit.
pub const GROUP_OUTDOORS: u32 = 0x8000;

/// What `0x69d4f0` finds behind one record of the entity's attachment list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attachment {
    /// The record's target has tag bit `0x10` clear (`0x69d52c`): the walk goes on.
    NotGroupNode,
    /// A group node whose group `0x6a4a00` answers null (the map object's loaded byte `+0x1d4`
    /// or the group's parsed byte `+0x164` is zero): the walk goes on.
    GroupNotLoaded,
    /// A group node whose group is parsed: its flags, the dword at `+0x10`.
    Group { flags: u32 },
}

/// `CWorld::IsEntityOutdoors` (`0x6706f0`) whole, as its `al`: `entity` is `None` for a null
/// entity, else its `+0x90` word and its attachment records in list order.
///
/// Null answers 1 (`0x6706fa`), as does a clear bit 0 of `+0x90` (`0x670709`). Otherwise the answer
/// is bit 15 of the flags of the first record `0x69d4f0` returns a group for, and 0 when it
/// returns none (`0x67073e`).
#[must_use]
pub fn is_entity_outdoors(entity: Option<(u32, &[Attachment])>) -> bool {
    let Some((flags, attachments)) = entity else {
        return true;
    };
    if flags & ENTITY_INDOORS == 0 {
        return true;
    }
    attachments
        .iter()
        .find_map(|a| match *a {
            Attachment::Group { flags } => Some(flags & GROUP_OUTDOORS != 0),
            Attachment::NotGroupNode | Attachment::GroupNotLoaded => None,
        })
        .unwrap_or(false)
}

/// `SetEntityLightFlags` (`0x6a87f0`) whole: the entity's `+0x90` word and the byte at `+0xc`
/// after the claim of a group with these flags.
///
/// An exterior group (`0x8`) sets `+0xc` bit 2 (`0x4`) and leaves `+0x90` alone. Any other group
/// sets `+0x90` bit 0, then `+0xc` bit 2 (`0x4`) for an exterior-lit group (`0x40`), else bit 1
/// (`0x2`).
#[must_use]
pub fn claim_group_light(entity_flags: u32, light_byte: u8, group_flags: u32) -> (u32, u8) {
    if group_flags & GROUP_EXTERIOR != 0 {
        return (entity_flags, light_byte | 0x4);
    }
    let flags = entity_flags | ENTITY_INDOORS;
    if group_flags & GROUP_EXTERIOR_LIT != 0 {
        (flags, light_byte | 0x4)
    } else {
        (flags, light_byte | 0x2)
    }
}
