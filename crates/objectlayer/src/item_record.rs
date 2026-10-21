//! `CGItem_C::ApplyRecord` (`0x5d8440`) as the calls it makes, in order, with their arguments
//! (`systems/objectlayer/items.md#the-item-record` and `#item-record-owner-signal`).
//!
//! The function caches five record bytes, looks the display row up for the sound, and then,
//! while the world latch `0xb4b424` is set, runs the lock and refresh calls for the active
//! player's item and signals the owner's `UNIT_INVENTORY_CHANGED` for every item. What it does
//! outside the bytes it stores is those calls, so that is what this models. The five byte getters
//! run twice, before the latch test and again at the tail, and the display id getter once between.

/// One call to a function the test stubs: its address and up to four values, whose meaning each
/// arm below names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Call {
    pub addr: u32,
    pub args: [u32; 4],
}

fn call(addr: u32, args: [u32; 4]) -> Call {
    Call { addr, args }
}

/// The five record-byte getters, `this` = the item.
pub const GETTERS: [u32; 5] = [0x5d9f30, 0x5d9f90, 0x5da110, 0x5d9ff0, 0x5da170];
/// `GetItemDisplayId`, whose negative answer skips the sound row.
pub const DISPLAY_ID: u32 = 0x5da0b0;
/// `DBCache_ItemStats_C::GetRecord`, `args` = (cache, entry).
pub const GET_RECORD: u32 = 0x55ba30;
/// `ClntObjMgrObjectPtr`, `args` = (mask, guid low, guid high, line).
pub const OBJECT_PTR: u32 = 0x468460;
/// `ClearItemFlagByEntry`, `args` = (inventory list, entry, 0).
pub const CLEAR_LOCKS: u32 = 0x6221e0;
/// `FrameScript_SignalEvent`, `args` = (event id).
pub const SIGNAL_EVENT: u32 = 0x703e50;
/// `UnlockItem`, `args` = (item guid low, high).
pub const UNLOCK_ITEM: u32 = 0x495420;
/// `ApplyPendingItemExpirations`, `args` = (player, item).
pub const APPLY_EXPIRATIONS: u32 = 0x5ebde0;
/// The refreshes, in order: `BuildTradeSkillList(0)`, `BuildCraftList()`,
/// `RecountItemActionSlots(entry)`, `SignalQuestLogForItem(entry)`.
pub const REFRESH: [u32; 4] = [0x4fca20, 0x4f60c0, 0x4e5c70, 0x4df7b0];
/// `0x4f9480`, `args` = (item guid low, high).
pub const BAG_REFRESH: u32 = 0x4f9480;
/// `SignalUnitEvent`, `args` = (guid low, guid high, event id).
pub const SIGNAL_UNIT_EVENT: u32 = 0x515e50;

/// The item cache object the record is asked of.
pub const ITEM_CACHE: u32 = 0xc0e2a0;
/// `UNIT_INVENTORY_CHANGED`.
pub const EVENT_UNIT_INVENTORY_CHANGED: u32 = 186;
/// `ITEM_LOCK_CHANGED`.
pub const EVENT_ITEM_LOCK_CHANGED: u32 = 188;

/// What `0x5d8440` reads.
#[derive(Clone, Copy, Debug)]
pub struct World {
    /// The byte `0xb4b424` is nonzero.
    pub latch: bool,
    /// The item's own GUID (`+0` of its descriptors).
    pub item_guid: u64,
    /// `ITEM_FIELD_OWNER`.
    pub owner: u64,
    /// The GUID `0x468550` returns: the active player's, 0 without a manager.
    pub active: u64,
    /// `OBJECT_FIELD_ENTRY`.
    pub entry: u32,
    /// The record `0x55ba30` returns, with its dword at `+0x60`, or none.
    pub record_stack: Option<i32>,
    /// The object `0x468460` returns for the active player, or none.
    pub player: Option<u32>,
    /// The item object's address, `this`.
    pub item: u32,
}

fn lo(g: u64) -> u32 {
    g as u32
}

fn hi(g: u64) -> u32 {
    (g >> 32) as u32
}

/// The calls `0x5d8440` makes for `w`, with `0x468550` (the active GUID, not stubbed) left out and
/// the display row read as absent (`0x5da0b0` answering −1).
pub fn apply_record_calls(w: &World) -> Vec<Call> {
    let getters = |v: &mut Vec<Call>| {
        for a in GETTERS {
            v.push(call(a, [w.item, 0, 0, 0]));
        }
    };
    let mut v = Vec::new();
    getters(&mut v);
    v.push(call(DISPLAY_ID, [w.item, 0, 0, 0]));
    if !w.latch {
        getters(&mut v);
        return v;
    }
    if w.owner == w.active {
        v.push(call(GET_RECORD, [ITEM_CACHE, w.entry, 0, 0]));
        v.push(call(OBJECT_PTR, [0x10, lo(w.active), hi(w.active), 0x90]));
        match w.record_stack {
            Some(stack) if stack > 1 => {
                if let Some(p) = w.player {
                    v.push(call(CLEAR_LOCKS, [p + 0x1d38, w.entry, 0, 0]));
                    v.push(call(SIGNAL_EVENT, [EVENT_ITEM_LOCK_CHANGED, 0, 0, 0]));
                    v.push(call(APPLY_EXPIRATIONS, [p, w.item, 0, 0]));
                }
            }
            _ => {
                v.push(call(UNLOCK_ITEM, [lo(w.item_guid), hi(w.item_guid), 0, 0]));
                if let Some(p) = w.player {
                    v.push(call(APPLY_EXPIRATIONS, [p, w.item, 0, 0]));
                }
            }
        }
        v.push(call(REFRESH[0], [0, 0, 0, 0]));
        v.push(call(REFRESH[1], [0, 0, 0, 0]));
        v.push(call(REFRESH[2], [w.entry, 0, 0, 0]));
        v.push(call(REFRESH[3], [w.entry, 0, 0, 0]));
        v.push(call(BAG_REFRESH, [lo(w.item_guid), hi(w.item_guid), 0, 0]));
    }
    v.push(call(
        SIGNAL_UNIT_EVENT,
        [lo(w.owner), hi(w.owner), EVENT_UNIT_INVENTORY_CHANGED, 0],
    ));
    getters(&mut v);
    v
}
