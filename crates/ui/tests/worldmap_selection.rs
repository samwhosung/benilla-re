//! The selection of the world map (`systems/ui/worldmap.md#the-selection`, `#the-getters`) run from
//! the exe: the head of `SetMap` (`0x4a67a0`), `GetSelectedAreaName` (`0x4a6cf0`) and the
//! power-of-two round-up of `GetMapInfo`'s third value (`0x4a7e69` to `0x4a7e9a`).
//!
//! The test seeds the continent table and the `WorldMapArea` rows, runs the exe from the entry to
//! the end of the head and reads the three cells `0x84506c`, `0x845070` and `0x845074` back, for a
//! continent below, at and past the count and for a zone at both bounds. The round-up runs as an
//! instruction slice that halts at the join `0x4a7e9a`.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::worldmap_selection::{pot_round_low, selected_wma_id, set_map, Selection};

const SEL_CONT: u64 = 0x84506c;

const SEL_ZONE: u64 = 0x845070;

const SEL_DIRECT: u64 = 0x845074;

const CONT_N: u64 = 0xb6e664; // continent record count

const CONT_TBL: u64 = 0xb6e668; // continent record array base, stride 0x10024

const WMA_IDX: u64 = 0xc0d5bc; // WorldMapArea id -> row

const WMA_MAX: u64 = 0xc0d5c0; // WorldMapArea max id (SIGNED compare at 0x4a6d38)

const REC_STRIDE: u64 = 0x10024;

struct Env {
    recs: u64,
    child: u64,
    idx: u64,
    frame: u64,
}

/// One continent record with `child_count` children, plus a `WorldMapArea` id-index.
fn setup(emu: &mut Emu, child_count: u32) -> Env {
    let recs = emu.alloc(2 * REC_STRIDE);
    let child = emu.alloc(0x100);
    let idx = emu.alloc(0x4000);
    let frame = emu.alloc(0x400);
    emu.write_u32(recs, 0); // record+0x00 mapID
    emu.write_u32(recs + 0x04, 700); // record+0x04 the continent's own WorldMapArea id
    emu.write_u32(recs + 0x0c, child_count); // record+0x0c child count
    emu.write_u32(recs + 0x10, child as u32); // record+0x10 child id array
    for k in 0..8u64 {
        emu.write_u32(child + 4 * k, 800 + k as u32);
    }
    emu.write_u32(CONT_TBL, recs as u32);
    emu.write_u32(CONT_N, 1);
    emu.write_u32(WMA_IDX, idx as u32);
    emu.write_u32(WMA_MAX, 0xfff);
    Env {
        recs,
        child,
        idx,
        frame,
    }
}

/// Install a `WorldMapArea` row at `id` whose `+0x0c` (AreaName) is `name_ptr`.
fn put_wma(emu: &mut Emu, e: &Env, id: u32, name_ptr: u32) {
    let row = emu.alloc(0x20);
    emu.write_u32(row + 0x0c, name_ptr);
    emu.write_u32(e.idx + 4 * u64::from(id), row as u32);
}

fn binary_set_map(emu: &mut Emu, e: &Env, continent: i32, edx: i32) -> Selection {
    // Poison all three cells so a leg that fails to write is visible as the poison, not as a
    // leftover from the previous case.
    emu.write_u32(SEL_CONT, 0x5555_5555);
    emu.write_u32(SEL_ZONE, 0x5555_5555);
    emu.write_u32(SEL_DIRECT, 0x5555_5555);
    emu.set_reg(RegisterX86::ECX, continent as u32 as u64);
    emu.set_reg(RegisterX86::EDX, edx as u32 as u64);
    emu.set_reg(RegisterX86::ESP, e.frame + 0x300);
    emu.run(0x4a67a0, 0x4a67ef);
    Selection {
        continent: emu.read_u32(SEL_CONT) as i32,
        zone: emu.read_u32(SEL_ZONE) as i32,
        direct_area: emu.read_u32(SEL_DIRECT) as i32,
    }
}

#[test]
fn worldmap_set_map_head() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let env = setup(&mut emu, 5);

    // (name, continent, edx)
    let cases: &[(&str, i32, i32)] = &[
        ("world view", -1, -1),
        // -1 must NOT capture edx into [0x845074] even when edx looks like a real id.
        ("world view, edx = a valid id", -1, 443),
        ("DIRECT AREA, WarsongGulch's id", -2, 443),
        ("DIRECT AREA, edx = 0", -2, 0),
        ("DIRECT AREA, edx = -1", -2, -1),
        ("DIRECT AREA, edx above the id bound", -2, 0x7fff),
        // An out-of-range continent that is neither -1 nor -2 still lands in [0x84506c].
        ("continent -3 (neither sentinel)", -3, 443),
        ("continent == count (out of range)", 1, 2),
        ("continent 0, zone -1", 0, -1),
        ("continent 0, zone 0", 0, 0),
        ("continent 0, zone childCount-1 (boundary)", 0, 4),
        ("continent 0, zone == childCount (boundary)", 0, 5),
        ("continent 0, zone -2", 0, -2),
    ];

    for &(what, continent, edx) in cases {
        let got = binary_set_map(&mut emu, &env, continent, edx);
        let ours = set_map(continent, edx, 1, 5);
        assert_eq!(got, ours, "{what}: continent={continent} edx={edx}");
    }
}

#[test]
fn worldmap_selected_area_name() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let env = setup(&mut emu, 5);

    // 443 is WarsongGulch's WorldMapArea id in the shipped 5875 data; 700 is the fixture's
    // continent-level id and 802 one of its children. 900 resolves to a NULL id-index slot.
    put_wma(&mut emu, &env, 443, 0xaaaa_0443);
    put_wma(&mut emu, &env, 700, 0xaaaa_0700);
    put_wma(&mut emu, &env, 802, 0xaaaa_0802);

    // (name, selection, expected model inputs)
    let cases: &[(&str, i32, i32, i32)] = &[
        ("world view", -1, -1, -1),
        ("DIRECT AREA, WarsongGulch", -2, -1, 443),
        ("DIRECT AREA, id -1", -2, -1, -1),
        ("DIRECT AREA, id 0", -2, -1, 0),
        ("DIRECT AREA, id with a NULL id-index slot", -2, -1, 900),
        ("DIRECT AREA, id above the bound", -2, -1, 0x1000),
        ("continent, no zone", 0, -1, -1),
        ("continent, zone 2", 0, 2, -1),
    ];

    for &(what, continent, zone, direct) in cases {
        emu.write_u32(SEL_CONT, continent as u32);
        emu.write_u32(SEL_ZONE, zone as u32);
        emu.write_u32(SEL_DIRECT, direct as u32);
        let got = emu.call(0x4a6cf0, 0, 0, &[]);

        // The model's own resolve, fed the two record-derived ids the binary would read.
        let sel = Selection {
            continent,
            zone,
            direct_area: direct,
        };
        let cont_own = emu.read_u32(env.recs + 0x04) as i32;
        let child_wma = if zone >= 0 {
            emu.read_u32(env.child + 4 * zone as u64) as i32
        } else {
            0
        };
        let id = selected_wma_id(sel, cont_own, child_wma);
        let want = if id == -1 || id < 0 || id > emu.read_u32(WMA_MAX) as i32 {
            0
        } else {
            let row = emu.read_u32(env.idx + 4 * id as u64);
            if row == 0 {
                0
            } else {
                emu.read_u32(u64::from(row) + 0x0c)
            }
        };
        assert_eq!(got, want, "{what}: selection=({continent},{zone},{direct})");
    }
}

#[test]
fn worldmap_getmapinfo_pot_round() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);

    // `esi` enters the block as 0x4a6d50's return value; `eax`/`ecx`/`edx`/`edi` are scratch.
    // Every power-of-two boundary of the low byte, plus a high part the round must preserve.
    for v in [
        0i32, 1, 2, 3, 4, 5, 7, 8, 9, 63, 64, 65, 127, 128, 129, 200, 254, 255, 256, 257, 333, 512,
        668, 700, 1002, 0x1234,
    ] {
        emu.set_reg(RegisterX86::ESI, v as u32 as u64);
        emu.set_reg(RegisterX86::EBP, frame + 0x200);
        emu.set_reg(RegisterX86::ESP, frame + 0x100);
        emu.run(0x4a7e69, 0x4a7e9a);
        let eax = emu.get_reg(RegisterX86::EAX) as u32 as i32;
        let esi = emu.get_reg(RegisterX86::ESI) as u32 as i32;
        let low = v & 0xff;
        assert_eq!(eax, pot_round_low(low), "rounded low byte of {v}");
        assert_eq!(esi, v - low, "high part of {v}");
    }
}
