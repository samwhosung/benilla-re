//! `IsUnitPlayerDriven` (`0x5fa550`) and the typed object lookup `ClntObjMgrObjectPtr`
//! (`0x468460`) it resolves a possessor with (`systems/objectlayer/units.md#player-driven-units`,
//! `systems/objectlayer/manager.md#finding-an-object`).
//!
//! An object is the three things the two functions read: the type word at `+8` of its
//! descriptors, and from the unit field block (`[unit+0x110]`, field 6 on) the unit flags at
//! `+0xa0`, `UNIT_FIELD_CHARMEDBY` at `+0x10` and `UNIT_FIELD_CREATEDBY` at `+0x20`.

/// Flag 0x2 of `UNIT_FIELD_FLAGS`: with it set, the 0xc00004 test is skipped.
pub const FLAG_SKIP_BLOCK_TEST: u32 = 0x2;
/// The flags that end the test with 0: bit 2, confused (0x400000) and fleeing (0x800000).
pub const FLAGS_BLOCKING: u32 = 0x00c0_0004;
/// Flag 0x1000000: the unit is possessed, so the answer is its possessor's.
pub const FLAG_POSSESSED: u32 = 0x0100_0000;
/// Flag 0x1: a player with it set is not player-driven.
pub const FLAG_BIT0: u32 = 0x1;
/// The type word's unit bit, the mask `0x5fa599` passes to `0x468460`.
pub const TYPE_UNIT: u32 = 0x8;
/// The type word's player bit, tested at `0x5fa5c7` as `shr ecx,4; test cl,1`.
pub const TYPE_PLAYER: u32 = 0x10;

/// One object as the two functions read it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unit {
    /// `OBJECT_FIELD_TYPE`, the dword at `+8` of the object's descriptors.
    pub type_word: u32,
    /// `UNIT_FIELD_FLAGS`.
    pub flags: u32,
    /// `UNIT_FIELD_CHARMEDBY`.
    pub charmed_by: u64,
    /// `UNIT_FIELD_CREATEDBY`.
    pub created_by: u64,
}

/// `ClntObjMgrObjectPtr` (`0x468460`): the object `table` finds for a GUID (`0x464870`, which
/// finds nothing for GUID 0) when its type word shares a bit with `mask`, else none. A mask of 0
/// admits nothing.
pub fn object_ptr<T: Copy>(
    table: &dyn Fn(u64) -> Option<T>,
    type_word: &dyn Fn(&T) -> u32,
    mask: u32,
    guid: u64,
) -> Option<T> {
    if guid == 0 {
        return None;
    }
    table(guid).filter(|o| type_word(o) & mask != 0)
}

/// `IsUnitPlayerDriven` (`0x5fa550`): whether the unit acts under a free player's control.
///
/// 1. With flag 0x2 clear and any of 0xc00004 set: false.
/// 2. With flag 0x1000000 set: the answer for the unit with a unit bit that the GUID
///    `UNIT_FIELD_CHARMEDBY` names, or `UNIT_FIELD_CREATEDBY` when that is 0; false when none.
/// 3. Otherwise true when the type word has the player bit, `UNIT_FIELD_CHARMEDBY` is 0 and
///    flag 0x1 is clear.
///
/// `table` stands for `0x464870`, the GUID lookup. A possession cycle recurses without bound, as
/// the client's does.
pub fn is_unit_player_driven(unit: &Unit, table: &dyn Fn(u64) -> Option<Unit>) -> bool {
    if unit.flags & FLAG_SKIP_BLOCK_TEST == 0 && unit.flags & FLAGS_BLOCKING != 0 {
        return false;
    }
    if unit.flags & FLAG_POSSESSED != 0 {
        let guid = if unit.charmed_by != 0 {
            unit.charmed_by
        } else {
            unit.created_by
        };
        return match object_ptr(table, &|u: &Unit| u.type_word, TYPE_UNIT, guid) {
            Some(possessor) => is_unit_player_driven(&possessor, table),
            None => false,
        };
    }
    unit.type_word & TYPE_PLAYER != 0 && unit.charmed_by == 0 && unit.flags & FLAG_BIT0 == 0
}
