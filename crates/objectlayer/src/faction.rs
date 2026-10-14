//! `FactionTemplateReaction` (`0x606640`), the comparator that gives one `FactionTemplate.dbc` row's
//! reaction toward another (`systems/objectlayer/reaction.md#templates-compared`). It takes the
//! two rows in `ecx` and `edx` and returns 1 hostile, 3 neutral or 4 friendly, with the first
//! match of these rules deciding:
//!
//! 1. A's enemy mask meets B's group mask: hostile.
//! 2. B's faction is in A's enemy list: hostile.
//! 3. A's friend mask meets B's group mask: friendly.
//! 4. B's faction is in A's friend list: friendly.
//! 5. B's friend mask meets A's group mask: friendly.
//! 6. A's faction is in B's friend list: friendly.
//! 7. Otherwise neutral.
//!
//! Hostility reads only A's side; friendliness reads both.

/// One `FactionTemplate.dbc` row, a `#[repr(C)]` mirror of the client's `0x38` bytes. The
/// comparator does not read `id` or `flags`.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FactionTemplate {
    pub id: u32,           // +0x00 record id
    pub faction: u32,      // +0x04 Faction.dbc id, matched against the lists
    pub flags: u32,        // +0x08 faction flags
    pub our_mask: u32,     // +0x0c group mask, this template's own groups
    pub friend_mask: u32,  // +0x10 friend group mask
    pub enemy_mask: u32,   // +0x14 enemy group mask
    pub enemies: [u32; 4], // +0x18..+0x24 enemy faction ids, 0-terminated
    pub friends: [u32; 4], // +0x28..+0x34 friend faction ids, 0-terminated
}

/// The size of the row image.
pub const FACTION_TEMPLATE_SIZE: usize = 0x38;

const _: () = assert!(std::mem::size_of::<FactionTemplate>() == FACTION_TEMPLATE_SIZE);

pub const REACTION_HOSTILE: u32 = 1;
pub const REACTION_NEUTRAL: u32 = 3;
pub const REACTION_FRIENDLY: u32 = 4;

/// Looks for `key` in a 4-entry id list. The scan stops at the first 0, so an id after it is never
/// seen, and at index 4, so the dword after `enemies[3]` is never read as an enemy.
fn list_contains(list: &[u32; 4], key: u32) -> bool {
    for &id in list {
        if id == 0 {
            return false;
        }
        if id == key {
            return true;
        }
    }
    false
}

/// `0x606640`: the reaction of `a` toward `b`.
pub fn faction_template_reaction(a: &FactionTemplate, b: &FactionTemplate) -> u32 {
    // Hostile: A's enemy mask meets B's groups, or B's faction is among A's enemies.
    if a.enemy_mask & b.our_mask != 0 {
        return REACTION_HOSTILE;
    }
    if list_contains(&a.enemies, b.faction) {
        return REACTION_HOSTILE;
    }
    // Friendly: A's friend mask or list toward B, then B's toward A.
    if a.friend_mask & b.our_mask != 0 {
        return REACTION_FRIENDLY;
    }
    if list_contains(&a.friends, b.faction) {
        return REACTION_FRIENDLY;
    }
    if b.friend_mask & a.our_mask != 0 {
        return REACTION_FRIENDLY;
    }
    if list_contains(&b.friends, a.faction) {
        return REACTION_FRIENDLY;
    }
    REACTION_NEUTRAL
}
