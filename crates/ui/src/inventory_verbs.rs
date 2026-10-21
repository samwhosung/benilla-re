//! The record callback of the ammo legs of `GetInventoryItemTexture` and
//! `GetInventoryItemQuality` (`systems/ui/inventoryverbs.md#ammo-record-callback`).

/// `UNIT_INVENTORY_CHANGED`, the event id `0x4c84f0` signals.
pub const EVENT_UNIT_INVENTORY_CHANGED: u32 = 186;

/// `0x4c84f0`: the event it signals through `0x703f50` for the status byte the item cache passes
/// (`[ebp+0xc]`, one byte of the dword): the event id, the format `%s` and the token `player`, or
/// nothing for a status of 0.
pub fn ammo_record_signal(status: u32) -> Option<(u32, &'static str, &'static str)> {
    if status & 0xff != 0 {
        Some((EVENT_UNIT_INVENTORY_CHANGED, "%s", "player"))
    } else {
        None
    }
}
