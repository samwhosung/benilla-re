//! Three small table lookups of the interface: the continent and zone names of the world map
//! lists (`0x4a65a0`, `0x4a65e0`, `systems/ui/worldmap.md`), the pet book's add gate
//! (`0x4b2f90`, `systems/ui/petbar.md`) and the trainer's collapsed-header test (`0x4d8c30`,
//! `systems/ui/trainer.md`).

/// A record table indexed by id: `[base + 4·id]` for `0 <= id <= max`, compared signed.
pub fn in_range(id: i32, max: i32) -> bool {
    (0..=max).contains(&id)
}

/// The byte offset of the localized string field at `field` for `locale`.
pub fn localized(field: u32, locale: u32) -> u32 {
    field.wrapping_add(locale.wrapping_mul(4))
}

/// `0x4a65a0`: the `Map.dbc` id a continent index names, when the index is below the count and
/// the id is in range; the name is then field `+0x10` of the map's record, if it has one.
pub fn continent_map(map_ids: &[i32], index: u32, map_max: i32) -> Option<i32> {
    let id = *map_ids.get(index as usize)?;
    in_range(id, map_max).then_some(id)
}

/// `0x4a65e0`: the `WorldMapArea` id a continent's zone names, when both indices are in bounds
/// (the zone against the record's count `+0xc`) and the id is in range.
pub fn zone_area(zones: &[Vec<i32>], continent: u32, zone: u32, wma_max: i32) -> Option<i32> {
    let id = *zones.get(continent as usize)?.get(zone as usize)?;
    in_range(id, wma_max).then_some(id)
}

/// `0x4b2f90`: whether a spell id goes into the pet book, from whether `Spell.dbc` has its record
/// and the record's byte `+0x18`, the low byte of `Attributes`, whose sign is `0x80`.
pub fn pet_book_takes(spell: u32, spell_max: i32, record_attr_byte: Option<u8>) -> bool {
    in_range(spell as i32, spell_max) && record_attr_byte.is_some_and(|b| b & 0x80 == 0)
}

/// `0x4d8c30`: 1 for a header row (first dword negative) whose group, found by key, has its bit
/// clear in the expanded mask; 0 for any other row, an index past the total, or no group. The
/// shift count is masked to five bits, as `shl eax,cl` does.
pub fn trainer_header_collapsed(
    rows: &[(i32, u32)],
    groups: &[u32],
    expanded: u32,
    index: u32,
) -> u32 {
    let Some(&(first, key)) = rows.get(index as usize) else {
        return 0;
    };
    if first >= 0 {
        return 0;
    }
    match groups.iter().position(|&g| g == key) {
        Some(i) => u32::from(expanded & (1u32 << (i as u32 & 31)) == 0),
        None => 0,
    }
}
