//! The vendor window's wait for its row records (`systems/ui/merchant.md#vendor-row-records-and-stock`):
//! `0x4faed0` asks the item cache for a row's record and counts a miss, and `0x4fb0f0`, run by
//! the cache's callback `0x4fada0` for each answer, counts it down and signals `MERCHANT_UPDATE`.

/// The event id of `MERCHANT_UPDATE`, the argument `FrameScript_SignalEvent` gets.
pub const MERCHANT_UPDATE: u32 = 0x13d;

/// What `0x4faed0(entry)` does with the cache's answer `record` (0 for a miss): whether it asks
/// the cache at all, the wait count after it, and what it returns. Entry 0 asks nothing.
pub fn ask_row_record(entry: u32, record: u32, pending: u32) -> (bool, u32, u32) {
    if entry == 0 {
        return (false, pending, 0);
    }
    let pending = if record == 0 {
        pending.wrapping_add(1)
    } else {
        pending
    };
    (true, pending, record)
}

/// What `0x4fb0f0` does with the wait count and the open vendor's GUID: the count after it, and
/// whether it signals `MERCHANT_UPDATE`. A count above 0 drops by one; the signal goes out when
/// the count is then 0, or was 0 already, and a vendor is open.
pub fn count_record_answer(pending: u32, vendor_guid: u64) -> (u32, bool) {
    let pending = pending.saturating_sub(1);
    (pending, pending == 0 && vendor_guid != 0)
}
