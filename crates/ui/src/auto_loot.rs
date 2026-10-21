//! The auto-loot sweep `0x4c1fa0` as the calls it makes, in order, and the cells it stores
//! (`systems/ui/loot.md#auto-loot` and `#window-state-after-the-sweep`).
//!
//! The sweep sends the money and each plain row, clears the player's auto-loot byte, and then
//! asks `LootHasContents` (`0x4c2a70`) whether anything is left. What it does outside the cells
//! it stores is those calls, so that is what this models. Three callees run for real in the test,
//! over memory it writes: the active player's GUID getter `0x468550`, the slot-type getter
//! `0x5ebce0` and `SetAutoLoot` `0x5df460` with 0, and so does `LootHasContents`; it is modelled
//! here by the one call it makes.

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

/// `ClntObjMgrObjectPtr`, `args` = (mask, guid low, guid high, line).
pub const OBJECT_PTR: u32 = 0x468460;
/// `FrameScript_SignalEvent`, `args` = (event id).
pub const SIGNAL_EVENT: u32 = 0x703e50;
/// `FrameScript_SignalEvent2`, `args` = (event id, format string, argument).
pub const SIGNAL_EVENT2: u32 = 0x703f50;
/// `CGPlayer_C::SendLootMoney`, `args` = (player).
pub const SEND_LOOT_MONEY: u32 = 0x5ea8c0;
/// `SendAutoStoreLootItem`, `args` = (player, wire slot).
pub const SEND_AUTOSTORE: u32 = 0x5e1ad0;
/// `DBCache_ItemStats_C::GetRecord`, `args` = (cache, entry, owner id address, callback).
pub const GET_RECORD: u32 = 0x55ba30;
/// `PlayItemDisplaySound`, `args` = (0, sound id).
pub const ITEM_SOUND: u32 = 0x457ff0;
/// `SndInterfacePlayInterfaceSound`, `args` = (name address).
pub const COIN_SOUND: u32 = 0x458030;
/// `EndLootWindow`, `args` = (release, move start, flag).
pub const END_LOOT: u32 = 0x48f200;
/// The money line's callees, in the order the line leg calls them: `SplitCopper`, then three
/// times `FrameScript_GetText`, `SStrCopy` and `SStrPrintf`, then `SStrPrintf`,
/// `FrameScript_GetText`, `SStrPrintf` and `AddChatLine`. Their arguments are not compared.
pub const SPLIT_COPPER: u32 = 0x6c6260;
pub const GET_TEXT: u32 = 0x703bf0;
pub const STR_COPY: u32 = 0x64a5a0;
pub const STR_PRINTF: u32 = 0x64a7f0;
pub const ADD_CHAT_LINE: u32 = 0x49a870;

/// The item cache object.
pub const ITEM_CACHE: u32 = 0xc0e2a0;
/// The loot source GUID cell, whose address is the owner id of every record request.
pub const SOURCE_GUID: u32 = 0xb71b48;
/// The coin sound's name string.
pub const COIN_SOUND_NAME: u32 = 0x843384;
/// `LOOT_OPENED`.
pub const EVENT_LOOT_OPENED: u32 = 267;
/// `LOOT_BIND_CONFIRM`.
pub const EVENT_LOOT_BIND_CONFIRM: u32 = 287;
/// The format string of the bind confirmation's argument.
pub const BIND_FORMAT: u32 = 0x847fbc;

/// A row of the window with the facts the sweep reads about it.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    /// The item entry, never 0 for a row that is there.
    pub entry: u32,
    /// The wire slot byte.
    pub wire_slot: u8,
    /// The slot type of the wire entry (0 for a plain row).
    pub slot_type: u32,
    /// The item record's bonding (`+0x194`) and quality (`+0x1c`).
    pub bonding: u32,
    pub quality: u32,
    /// The record's sound id (`+0x18`).
    pub sound: u32,
}

/// What `0x4c1fa0` reads.
#[derive(Clone, Copy, Debug)]
pub struct World {
    /// The pending fetch count `0xb71b44`.
    pub pending: u32,
    /// The active player's GUID, and whether `0x468460` finds the object (its address).
    pub active: u64,
    pub player: Option<u32>,
    /// The money cell `0xb71ba0`.
    pub money: i32,
    /// The cell `0xb71bc4`.
    pub shown: u32,
    /// The first party member's GUID `0xbc6f48` is nonzero.
    pub in_party: bool,
    /// The raid member count `0xb713e0` is nonzero.
    pub in_raid: bool,
    /// The source GUID `0xb71b48` and whether `0x468460` finds it.
    pub source_guid: u64,
    pub source: bool,
    /// The 16 rows; `None` is an empty row (entry 0).
    pub rows: [Option<Row>; 16],
    /// The argument in `ecx`.
    pub flag: u32,
}

/// What a sweep leaves besides its calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stores {
    /// The player's auto-loot byte is written 0 (`0x4c2219`).
    pub byte_cleared: bool,
    /// `0xb71bc4` is written 1 (`0x4c221e`).
    pub shown_set: bool,
    /// `0x847cec` is written with this array position (`0x4c21dc`).
    pub bind_row: Option<u32>,
}

fn lo(g: u64) -> u32 {
    g as u32
}

fn hi(g: u64) -> u32 {
    (g >> 32) as u32
}

/// `0x4c2a70`: the source resolves and has money above 0 or an occupied row.
pub fn has_contents(w: &World) -> bool {
    w.source && (w.money > 0 || w.rows.iter().any(Option::is_some))
}

/// The calls `0x4c1fa0` makes for `w` (with `0x468550`, `0x5ebce0`, `0x5df460` and the body of
/// `0x4c2a70` run for real, so not among them) and its stores.
pub fn sweep(w: &World) -> (Vec<Call>, Stores) {
    let mut v = Vec::new();
    let mut stores = Stores {
        byte_cleared: false,
        shown_set: false,
        bind_row: None,
    };
    // Returns while records are being fetched (`0x4c1fb3`).
    if w.pending != 0 {
        return (v, stores);
    }
    v.push(call(OBJECT_PTR, [0x10, lo(w.active), hi(w.active), 0x90]));
    // Without the player: the event, and nothing else (`0x4c1fd9` to `0x4c1fe8`).
    let Some(player) = w.player else {
        v.push(call(SIGNAL_EVENT, [EVENT_LOOT_OPENED, 0, 0, 0]));
        return (v, stores);
    };
    // The money cell, -1 included, sends the request; the sound and the line are the first
    // sweep's and need money above 0 (`0x4c1ff4` to `0x4c2175`).
    if w.money != 0 {
        v.push(call(SEND_LOOT_MONEY, [player, 0, 0, 0]));
        if w.shown == 0 && w.money > 0 {
            v.push(call(COIN_SOUND, [COIN_SOUND_NAME, 0, 0, 0]));
            if !w.in_party && !w.in_raid {
                v.push(call(SPLIT_COPPER, [0; 4]));
                for _ in 0..3 {
                    v.push(call(GET_TEXT, [0; 4]));
                    v.push(call(STR_COPY, [0; 4]));
                    v.push(call(STR_PRINTF, [0; 4]));
                }
                v.push(call(STR_PRINTF, [0; 4]));
                v.push(call(GET_TEXT, [0; 4]));
                v.push(call(STR_PRINTF, [0; 4]));
                v.push(call(ADD_CHAT_LINE, [0; 4]));
            }
        }
    }
    // The rows (`0x4c2175` to `0x4c2208`): a plain row is sent, the first bind-on-pickup row of
    // quality 2 or more is confirmed instead, any other type is left.
    let mut confirmed = false;
    for (i, row) in w.rows.iter().enumerate() {
        let Some(row) = row else { continue };
        if row.slot_type != 0 {
            continue;
        }
        v.push(call(GET_RECORD, [ITEM_CACHE, row.entry, SOURCE_GUID, 0]));
        if row.bonding == 1 && row.quality >= 2 {
            if confirmed {
                continue;
            }
            v.push(call(
                SIGNAL_EVENT2,
                [EVENT_LOOT_BIND_CONFIRM, BIND_FORMAT, i as u32 + 1, 0],
            ));
            stores.bind_row = Some(i as u32);
            confirmed = true;
        } else {
            v.push(call(ITEM_SOUND, [0, row.sound, 0, 0]));
            v.push(call(
                SEND_AUTOSTORE,
                [player, u32::from(row.wire_slot), 0, 0],
            ));
        }
    }
    // The byte is cleared, the flag set, and the contents asked (`0x4c2214` to `0x4c2257`).
    stores.byte_cleared = true;
    stores.shown_set = true;
    v.push(call(
        OBJECT_PTR,
        [1, lo(w.source_guid), hi(w.source_guid), 0x2a6],
    ));
    if !has_contents(w) {
        v.push(call(END_LOOT, [1, 0, 0, 0]));
    } else if w.flag == 0 {
        v.push(call(SIGNAL_EVENT, [EVENT_LOOT_OPENED, 0, 0, 0]));
    }
    (v, stores)
}
