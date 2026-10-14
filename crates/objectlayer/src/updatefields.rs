//! The geometry of an object's update fields: `UpdateMaskTestBit` (`0x465550`), `GetDataBaseOffset`
//! (`0x465690`), `NextFieldSection` (`0x4656e0`), `ObjectWatcherList` (`0x465850`),
//! `ObjectFieldCount` (`0x465a80`) and `CGPlayer_C::SetActivePlayerExtra` (`0x466de0`)
//! (`systems/objectlayer/update.md#values-block` and `fields.md#field-counts-and-sections`).
//! A player's field count depends on whether its guid is the active player's: 1282 fields or 486.

/// The cumulative field count of each type id (the `*_END` constants), the table at `0xb4143c` the
/// manager constructor `0x464260` writes.
pub const FIELD_COUNTS: [u32; 8] = [6, 0x30, 0x7a, 0xbc, 0x502, 0x1a, 0x10, 0x26];

/// `0x465550`: `(1 << (idx & 0x1f)) & mask[idx >> 5]`, the masked dword, zero when the bit is
/// clear. There is no bounds check: the mask must cover `idx >> 5`.
pub fn updatemask_test_bit(mask: &[u32], idx: u32) -> u32 {
    (1u32 << (idx & 0x1f)) & mask[(idx >> 5) as usize]
}

/// `0x465690`: the element stride in bytes of a descriptor block id.
pub fn updateblock_stride(block_id: u32) -> u32 {
    match block_id {
        1 | 3 | 5..=7 => 0x18,
        2 => 0xc0,
        4 => 0x2f0,
        _ => 0,
    }
}

/// `0x4656e0`: the descriptor section (0 to 8) of a type mask and a sub-block ordinal; 8 means
/// there is no such sub-block. The client reads the mask from `*(*(obj+8)+8)`; here it is the
/// input.
pub fn descriptor_section(typemask: u32, sub: u32) -> u32 {
    match typemask {
        3 | 7 => match sub {
            0 => 1,
            1 => 2,
            _ => 8,
        },
        9 | 0x19 => match sub {
            0 => 3,
            3 => 4,
            _ => 8,
        },
        0x21 if sub == 0 => 5,
        0x41 if sub == 0 => 6,
        0x81 if sub == 0 => 7,
        _ => 8,
    }
}

/// `0x465850`: the byte address of a field from its descriptor section and index. `this` is the
/// object base, `guid` the object's guid and `self_guid` the active player's (from `**(obj+8)` and
/// `0x468550`). Section 4 is the player block: the active player at or past `UNIT_END` (`0x12a`)
/// indexes through the spill base `spill_base` (`*(this+0x1c68)`); every other case uses the inline
/// stride of 3 dwords. The arithmetic wraps in 32 bits.
pub fn updatefield_addr(
    this: u32,
    section: u32,
    index: u32,
    guid: u64,
    self_guid: u64,
    spill_base: u32,
) -> u32 {
    match section {
        0 => this.wrapping_add(index.wrapping_mul(3).wrapping_add(0x12).wrapping_mul(4)),
        1 | 3 => this
            .wrapping_add(0x11c)
            .wrapping_add(index.wrapping_mul(0xc)),
        2 => this
            .wrapping_add(0x350)
            .wrapping_add(index.wrapping_mul(0xc)),
        5..=7 => this
            .wrapping_add(0x118)
            .wrapping_add(index.wrapping_mul(0xc)),
        4 => {
            // The active player past UNIT_END goes through the spill base; everything else is inline.
            if guid != self_guid || index < 0x12a {
                this.wrapping_add(index.wrapping_mul(3).wrapping_add(0x39c).wrapping_mul(4))
            } else {
                spill_base.wrapping_add(index.wrapping_mul(3).wrapping_sub(0x37e).wrapping_mul(4))
            }
        }
        _ => 0,
    }
}

/// `0x465a80`: the field count of a type mask. A player (`0x19`) has 1282 fields when `guid` is
/// `self_guid`, the active player from `0x468550`, and 486 otherwise.
pub fn updatefield_count(typemask: u32, guid: u64, self_guid: u64) -> u32 {
    match typemask {
        1 => 6,
        3 => 0x30,
        7 => 0x7a,
        9 => 0xbc,
        0x19 => {
            if guid == self_guid {
                0x502
            } else {
                0x1e6
            }
        }
        0x21 => 0x1a,
        0x41 => 0x10,
        0x81 => 0x26,
        _ => 0,
    }
}

/// `0x466de0`: the spill base written to `this+0x1c68`, `this + 0x3178 + dynfield_index * 4`, where
/// `dynfield_index` is the count from `0x47d540`.
pub fn player_dynfield_base(this: u32, dynfield_index: u32) -> u32 {
    this.wrapping_add(0x3178)
        .wrapping_add(dynfield_index.wrapping_mul(4))
}
