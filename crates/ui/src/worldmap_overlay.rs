//! The exploration test of the world map's overlays (`systems/ui/worldmap.md#the-exploration-test`,
//! `#the-overlays`): the integer predicate of `[0x4a6ad9, 0x4a6cdf)` inside `SetMap` (`0x4a67a0`)
//! that decides which `WorldMapOverlay` rows enter the list `GetNumMapOverlays` counts. It holds no
//! floating point.
//!
//! Three details separate a faithful version from a clean one:
//!
//! - `AreaBit / 8` truncates toward zero (`cdq; and edx, 7; add eax, edx; sar eax, 3`), and the
//!   bound `cmp eax, 0x100; jae` is unsigned, so the slot reveals for `AreaBit >= 2048` and for
//!   `AreaBit <= -8`, but not for `AreaBit` in `[-7, -1]`.
//! - `AreaBit % 8` is the signed remainder, and the shift count reaches `shl edx, cl` masked to 5
//!   bits, so `AreaBit` in `[-7, -1]` builds a mask in bits 25 to 31, which a zero-extended byte
//!   never meets. Such a slot never reveals.
//! - `ExplorationLevel < 0` is a strict sign test: 0 does not reveal.
//!
//! In the shipped data no `AreaTable` row has a negative `AreaBit`, and the one row with a negative
//! `ExplorationLevel` is in no overlay, so the last two do not occur in practice.

/// `PLAYER_EXPLORED_ZONES_1` is 64 dwords (the descriptor table at `0x83b594`, index `0x39b`, count
/// `0x40`): 256 bytes, 2048 bits. `cmp eax, 0x100` at `0x4a6bd4` is that bound.
pub const EXPLORED_ZONES_BYTES: usize = 0x100;

/// One `areaID` slot of a `WorldMapOverlay` row, resolved to its `AreaTable` row.
#[derive(Clone, Copy, Debug)]
pub struct AreaSlot {
    /// `AreaTable` field 3 (`row+0x0c`), read at `0x4a6bae`, signed.
    pub area_bit: i32,
    /// `AreaTable` field 10 (`row+0x28`), read at `0x4a6ba7`, signed.
    pub exploration_level: i32,
}

/// Whether the slot reveals its overlay (`0x4a6ba7` to `0x4a6c07`). `guid_matches` is the guid pair
/// test of `0x4a6bb1` to `0x4a6bc7`, true for the active player. A negative exploration level
/// reveals before the guid test (`0x4a6bac`, `jl 0x4a6c27`); a mismatched guid skips the slot
/// (`0x4a6c09`); a byte index past the end of the explored zones reveals (`0x4a6bd9`); otherwise
/// the slot reveals when its bit is set, by a zero-extended byte (`movzx`, `0x4a6bfa`).
pub fn slot_reveals(
    slot: AreaSlot,
    guid_matches: bool,
    explored: &[u8; EXPLORED_ZONES_BYTES],
) -> bool {
    if slot.exploration_level < 0 {
        return true;
    }
    if !guid_matches {
        return false;
    }
    let byte_idx = slot.area_bit / 8;
    if (byte_idx as u32) >= EXPLORED_ZONES_BYTES as u32 {
        return true;
    }
    let mask = 1u32 << ((slot.area_bit % 8) & 31);
    let byte = u32::from(explored[byte_idx as usize]);
    mask & byte != 0
}

/// The whole row test (`0x4a6b6b` to `0x4a6cc4`): the row is appended when its map area id equals
/// the displayed `WorldMapArea` id and some slot reveals. `slots` are the four `areaID` dwords at
/// `row+0x08` to `+0x14`; a slot is `None` when `areaID <= 0`, `areaID > AreaTable.maxId` or the
/// `AreaTable` row is null (`0x4a6b8e` to `0x4a6ba5`). Scanning stops at the first slot that
/// reveals, so a row is appended at most once (`0x4a6c27` falls into the next row's block).
pub fn row_revealed(
    row_map_area_id: u32,
    displayed_map_area_id: u32,
    slots: &[Option<AreaSlot>; 4],
    guid_matches: bool,
    explored: &[u8; EXPLORED_ZONES_BYTES],
) -> bool {
    if row_map_area_id != displayed_map_area_id {
        return false;
    }
    slots
        .iter()
        .flatten()
        .any(|&s| slot_reveals(s, guid_matches, explored))
}
