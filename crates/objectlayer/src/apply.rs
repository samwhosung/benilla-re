//! `ObjectApplyValues` (`0x466590`) and `ObjectClearFields` (`0x466c70`), the kernels that write
//! an update's values into an object (`systems/objectlayer/update.md#writing-the-values` and
//! `#creating-an-object`).

use crate::updatefields::{player_dynfield_base, updatefield_count, updatemask_test_bit};

/// `ObjectApplyValues` (`0x466590`): applies a values block to the object's live field array
/// (`*(obj+8)`). It reads a byte count of mask dwords (the client's buffer holds 41), then for each
/// field index below `updatefield_count(typemask, guid, self_guid)`: a set mask bit reads the next
/// wire dword into `live[idx*4]`, a clear bit stores 0 when `clear_unset` is set, and otherwise the
/// field is left. The change-record path through `*(obj+0xc)` (`0x4657e0`, `0x465970`, `0x4667a0`)
/// is inert without registered watchers, and its skip flag does not change the live array.
///
/// `live` is the object's `*(obj+8)` image, whose head holds the type mask; `wire` is the block;
/// `self_guid` is the active player's guid, which splits a player's field count.
pub fn apply_update_fields(
    live: &mut [u8],
    wire: &[u8],
    typemask: u32,
    guid: u64,
    self_guid: u64,
    clear_unset: bool,
) {
    let mut c = 1usize; // cursor past the u8 maskDwordCount
    let mask_dwords = wire[0] as usize;
    let mut mask = [0u32; 41];
    for slot in mask.iter_mut().take(mask_dwords) {
        *slot = u32::from_le_bytes(wire[c..c + 4].try_into().unwrap());
        c += 4;
    }
    let n = updatefield_count(typemask, guid, self_guid);
    for idx in 0..n {
        let off = idx as usize * 4;
        if updatemask_test_bit(&mask, idx) != 0 {
            let v = u32::from_le_bytes(wire[c..c + 4].try_into().unwrap());
            c += 4;
            live[off..off + 4].copy_from_slice(&v.to_le_bytes());
        } else if clear_unset {
            live[off..off + 4].copy_from_slice(&0u32.to_le_bytes());
        }
    }
}

/// `ObjectClearFields` (`0x466c70`): resets an object's field block for construction. It first
/// calls a cleanup constructor for the embedded containers (stubbed in the test), then zero-fills
/// `size` bytes at `obj + offset` from the table at `0x466db8`. A player (type id 4) takes size
/// `0x1408` when it is the active player and `0x798` otherwise, and stores the dynamic-field spill
/// pointer at `+0x1c68`: `player_dynfield_base` for the active player, else 0. A type id above 7
/// does nothing.
///
/// `obj_va` is the object's base address, which the spill pointer is computed against, and
/// `dynfield_count` is the count from `0x47d540`.
pub fn zero_descriptor_block(
    obj: &mut [u8],
    obj_va: u32,
    type_id: u32,
    self_player: bool,
    dynfield_count: u32,
) {
    let (offset, size): (usize, usize) = match type_id {
        0 => (0x110, 0x18),
        1 => (0x348, 0xc0),
        2 => (0x6e0, 0x1e8),
        3 => (0xe68, 0x2f0),
        4 => {
            let size = if self_player { 0x1408 } else { 0x798 };
            let spill = if self_player {
                player_dynfield_base(obj_va, dynfield_count)
            } else {
                0
            };
            obj[0x1c68..0x1c6c].copy_from_slice(&spill.to_le_bytes());
            (0x1d70, size)
        }
        5 => (0x288, 0x68),
        6 => (0x1a0, 0x40),
        7 => (0x2b8, 0x98),
        _ => return,
    };
    for b in &mut obj[offset..offset + size] {
        *b = 0;
    }
}
