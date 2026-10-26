//! Three table lookups against `WoW.exe` in the emulator, each run whole on tables seeded in
//! scratch memory:
//!
//! * `worldmap_list_names`: `GetContinentName` (`0x4a65a0`) and `GetContinentZoneName`
//!   (`0x4a65e0`) over continent and zone indices at and past their bounds, ids out of range on
//!   both sides, missing records and two locales (`systems/ui/worldmap.md#the-getters`).
//! * `pet_book_add_gate`: `AddPetBookSpell` (`0x4b2f90`) over spell ids at and past the bound,
//!   negative ids, missing records and every value of the record's byte `+0x18`
//!   (`systems/ui/petbar.md#the-pet-spell-message`).
//! * `trainer_header_collapsed`: `IsTrainerHeaderCollapsed` (`0x4d8c30`) over header and service
//!   rows, an index past the total, a key with no group, and expanded masks with groups past bit
//!   31 (`systems/ui/trainer.md#index-law`).
use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::list_lookups::{
    continent_map, localized, pet_book_takes, trainer_header_collapsed as header_collapsed,
    zone_area,
};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*s).rotate_right(8)
}

const CONT_N: u64 = 0xb6_e664;
const CONT_TBL: u64 = 0xb6_e668;
const CONT_STRIDE: u64 = 0x1_0024;
const MAP_INDEX: u64 = 0xc0_daa8;
const MAP_MAX: u64 = 0xc0_daac;
const WMA_INDEX: u64 = 0xc0_d5bc;
const WMA_MAX: u64 = 0xc0_d5c0;
const AREA_INDEX: u64 = 0xc0_e048;
const AREA_MAX: u64 = 0xc0_e04c;
const LOCALE: u64 = 0xc0_e080;

#[test]
fn worldmap_list_names() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // Four continents: map ids 0, 1, a negative one and one past the bound; map 1 has no record.
    let map_ids = [0i32, 1, -5, 40];
    let map_max = 30i32;
    let zones: Vec<Vec<i32>> = vec![vec![10, 11, 12], vec![13, -1], vec![], vec![50, 14]];
    let wma_max = 20i32;
    let recs = emu.alloc(CONT_STRIDE * map_ids.len() as u64);
    let zone_arrays = emu.alloc(0x100);
    for (i, &id) in map_ids.iter().enumerate() {
        let rec = recs + CONT_STRIDE * i as u64;
        emu.write_u32(rec, id as u32);
        emu.write_u32(rec + 0xc, zones[i].len() as u32);
        let arr = zone_arrays + 0x20 * i as u64;
        for (k, &z) in zones[i].iter().enumerate() {
            emu.write_u32(arr + 4 * k as u64, z as u32);
        }
        emu.write_u32(rec + 0x10, arr as u32);
    }
    emu.write_u32(CONT_TBL, recs as u32);
    emu.write_u32(CONT_N, map_ids.len() as u32);
    // Map records: 0 present, 1 absent. Names are tagged pointers, one per locale.
    let map_index = emu.alloc(4 * (map_max as u64 + 1));
    let map0 = emu.alloc(0x60);
    for loc in 0..8u32 {
        emu.write_u32(map0 + u64::from(localized(0x10, loc)), 0xa000_0000 | loc);
    }
    emu.write_u32(map_index, map0 as u32);
    emu.write_u32(MAP_INDEX, map_index as u32);
    emu.write_u32(MAP_MAX, map_max as u32);
    // WorldMapArea rows 10 to 14: row 12 absent; their areas 100, 101, -, 500 (past the bound), 103.
    let wma_index = emu.alloc(4 * (wma_max as u64 + 1));
    let area_of = [(10, 100i32), (11, 101), (13, 500), (14, 103)];
    let area_max = 200i32;
    let area_index = emu.alloc(4 * (area_max as u64 + 1));
    for &(w, a) in &area_of {
        let row = emu.alloc(0x40);
        emu.write_u32(row + 8, a as u32);
        emu.write_u32(wma_index + 4 * w as u64, row as u32);
        if a != 101 && (0..=area_max).contains(&a) {
            let area = emu.alloc(0x80);
            for loc in 0..8u32 {
                emu.write_u32(
                    area + u64::from(localized(0x2c, loc)),
                    0xb000_0000 | (a as u32) << 4 | loc,
                );
            }
            emu.write_u32(area_index + 4 * a as u64, area as u32);
        }
    }
    emu.write_u32(WMA_INDEX, wma_index as u32);
    emu.write_u32(WMA_MAX, wma_max as u32);
    emu.write_u32(AREA_INDEX, area_index as u32);
    emu.write_u32(AREA_MAX, area_max as u32);

    let mut total = 0u32;
    for loc in [0u32, 3] {
        emu.write_u32(LOCALE, loc);
        for index in [0u32, 1, 2, 3, 4, 5, u32::MAX] {
            let got = emu.call(0x4a_65a0, index, 0, &[]);
            let want = continent_map(&map_ids, index, map_max)
                .filter(|&id| id == 0)
                .map_or(0, |_| 0xa000_0000 | loc);
            assert_eq!(got, want, "continent {index} locale {loc}");
            total += 1;
            for zone in [0u32, 1, 2, 3, u32::MAX] {
                let got = emu.call(0x4a_65e0, index, zone, &[]);
                let want = zone_area(&zones, index, zone, wma_max)
                    .and_then(|w| area_of.iter().find(|p| p.0 == w).map(|p| p.1))
                    .filter(|&a| a != 101 && (0..=area_max).contains(&a))
                    .map_or(0, |a| 0xb000_0000 | (a as u32) << 4 | loc);
                assert_eq!(got, want, "continent {index} zone {zone} locale {loc}");
                total += 1;
            }
        }
    }
    eprintln!("worldmap_list_names: {total} bit-exact (0x4a65a0, 0x4a65e0 whole)");
}

const SPELL_INDEX: u64 = 0xc0_d788;
const SPELL_MAX: u64 = 0xc0_d78c;
const BOOK: u64 = 0xb6_f098;
const BOOK_COUNT: u64 = 0xb7_1174;

#[test]
fn pet_book_add_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let spell_max = 300i32;
    let index = emu.alloc(4 * (spell_max as u64 + 1));
    let rec = emu.alloc(0x40);
    emu.write_u32(SPELL_INDEX, index as u32);
    emu.write_u32(SPELL_MAX, spell_max as u32);
    let mut total = 0u32;
    let mut s = 0x4b2f_9000u32;
    for attr in 0..=255u8 {
        for spell in [
            0u32,
            1,
            150,
            299,
            300,
            301,
            u32::MAX,
            0x8000_0000,
            next(&mut s) % 300,
        ] {
            let present = spell != 150;
            emu.write(rec + 0x18, &[attr]);
            if (0..=spell_max).contains(&(spell as i32)) {
                emu.write_u32(
                    index + 4 * u64::from(spell),
                    if present { rec as u32 } else { 0 },
                );
            }
            let count = next(&mut s) % 1000;
            emu.write_u32(BOOK_COUNT, count);
            emu.write_u32(BOOK + 4 * u64::from(count), 0xdead_beef);
            emu.call(0x4b_2f90, spell, 0, &[]);
            let record = present.then_some(attr);
            let takes = pet_book_takes(spell, spell_max, record);
            let got_count = emu.read_u32(BOOK_COUNT);
            assert_eq!(
                got_count,
                count + u32::from(takes),
                "spell {spell} attr {attr:#x}"
            );
            let slot = emu.read_u32(BOOK + 4 * u64::from(count));
            assert_eq!(
                slot,
                if takes { spell } else { 0xdead_beef },
                "spell {spell} attr {attr:#x}"
            );
            if (0..=spell_max).contains(&(spell as i32)) {
                emu.write_u32(index + 4 * u64::from(spell), 0);
            }
            total += 1;
        }
    }
    eprintln!("pet_book_add_gate: {total} bit-exact (0x4b2f90 whole)");
}

const SERVICES: u64 = 0xb7_37b0;
const SERVICE_COUNT: u64 = 0xb7_3a10;
const GROUPS: u64 = 0xb7_39c0;
const GROUP_COUNT: u64 = 0xb7_3a14;
const EXPANDED: u64 = 0xb7_3a24;

#[test]
fn trainer_header_collapsed() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // 40 groups with keys 1000 + g; rows alternate headers (first dword -1 or i32::MIN) and
    // services (0, 1, 0x7fffffff), keyed to groups, to a missing key, or to group 32 and up.
    let groups: Vec<u32> = (0..40).map(|g| 1000 + g).collect();
    let group_ptrs = emu.alloc(4 * 64);
    for (g, &key) in groups.iter().enumerate() {
        let p = emu.alloc(0x10);
        emu.write_u32(p, key);
        emu.write_u32(group_ptrs + 4 * g as u64, p as u32);
    }
    let mut rows: Vec<(i32, u32)> = Vec::new();
    for g in 0..40u32 {
        rows.push((if g % 2 == 0 { -1 } else { i32::MIN }, 1000 + g));
        rows.push(([0, 1, i32::MAX][(g % 3) as usize], 1000 + g));
    }
    rows.push((-1, 5));
    let row_ptrs = emu.alloc(4 * rows.len() as u64);
    for (i, &(first, key)) in rows.iter().enumerate() {
        let p = emu.alloc(0x40);
        emu.write_u32(p, first as u32);
        emu.write_u32(p + 4, key);
        emu.write_u32(row_ptrs + 4 * i as u64, p as u32);
    }
    emu.write_u32(SERVICES, row_ptrs as u32);
    emu.write_u32(SERVICE_COUNT, rows.len() as u32);
    emu.write_u32(GROUPS, group_ptrs as u32);
    emu.write_u32(GROUP_COUNT, groups.len() as u32);
    let mut s = 0x4d8c_3000u32;
    let mut masks = vec![0u32, u32::MAX, 1, 0x8000_0000, 0x5555_5555];
    for _ in 0..30 {
        masks.push(next(&mut s));
    }
    let mut total = 0u32;
    for &mask in &masks {
        emu.write_u32(EXPANDED, mask);
        for index in (0..=rows.len() as u32 + 1).chain([u32::MAX]) {
            let got = emu.call(0x4d_8c30, index, 0, &[]);
            let want = header_collapsed(&rows, &groups, mask, index);
            assert_eq!(got, want, "index {index} mask {mask:#x}");
            total += 1;
        }
    }
    eprintln!("trainer_header_collapsed: {total} bit-exact (0x4d8c30 whole)");
}
