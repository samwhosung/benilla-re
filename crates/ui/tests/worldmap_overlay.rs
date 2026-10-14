//! The overlay reveal test inside `SetMap` (`0x4a67a0`, `[0x4a6ad9, 0x4a6cdf)`,
//! `systems/ui/worldmap.md#the-exploration-test`, `#the-overlays`) run from the exe.
//!
//! The test builds the object manager, the `AreaTable` and `WorldMapOverlay` stores and the player's
//! explored bits in emulator memory and runs the exe's loop, reading the list of overlay rows it
//! builds. The Rust predicate is compared arm by arm: a negative exploration level, the guid pair,
//! the byte index at and past the end of the explored zones, negative area bits, and every bit of
//! the explored bytes.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::worldmap_overlay::{row_revealed, slot_reveals, AreaSlot, EXPLORED_ZONES_BYTES};

const OBJMGR_G: u64 = 0xb41414; // ClntObjMgr singleton ptr

const AT_IDX: u64 = 0xc0e048; // AreaTable id -> row

const AT_MAX: u64 = 0xc0e04c; // AreaTable max id

const OVL_ROWS: u64 = 0xc0d58c; // WorldMapOverlay rows base

const OVL_COUNT: u64 = 0xc0d590; // WorldMapOverlay row count

const CNT: u64 = 0xb6e6b4; // live overlay count

const DATA: u64 = 0xb6e630; // overlay id array

const CAP: u64 = 0xb6e62c; // container capacity

const CONT_TBL: u64 = 0xb6e668; // continent info table

const CONT_N: u64 = 0xb6e664;

const SEL_CONT: u64 = 0x84506c;

const SEL_ZONE: u64 = 0x845070;

const SEL_EXPLICIT: u64 = 0x845074;

const GUID_LO: u32 = 0xdead_beef;

const GUID_HI: u32 = 0x0000_0042;

struct Env {
    player: u64,
    desc_good: u64,
    desc_bad: u64,
    block: u64,
    out: u64,
    idx: u64,
    ebp: u64,
}

fn setup(emu: &mut Emu) -> Env {
    let objmgr = emu.alloc(0x200);
    let desc_good = emu.alloc(0x40);
    let desc_bad = emu.alloc(0x40);
    let block = emu.alloc(0x1100);
    let player = emu.alloc(0x1000);
    let out = emu.alloc(0x100);
    let idx = emu.alloc(0x4000);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x200;
    emu.write_u32(desc_good, GUID_LO);
    emu.write_u32(desc_good + 4, GUID_HI);
    emu.write_u32(desc_bad, 1);
    emu.write_u32(desc_bad + 4, 2);
    emu.write_u32(objmgr + 0xc0, GUID_LO);
    emu.write_u32(objmgr + 0xc4, GUID_HI);
    emu.write_u32(OBJMGR_G, objmgr as u32);
    emu.write_u32(player + 0x8, desc_good as u32);
    emu.write_u32(player + 0xe68, block as u32);
    emu.write_u32(AT_IDX, idx as u32);
    emu.write_u32(AT_MAX, 0xfff);
    emu.write_u32(DATA, out as u32);
    Env {
        player,
        desc_good,
        desc_bad,
        block,
        out,
        idx,
        ebp,
    }
}

fn set_explored(emu: &mut Emu, e: &Env, bits: &[i32]) -> [u8; EXPLORED_ZONES_BYTES] {
    let mut model = [0u8; EXPLORED_ZONES_BYTES];
    for &b in bits {
        model[(b / 8) as usize] |= 1u8 << (b % 8);
    }
    emu.write(e.block + 0xe6c, &model);
    model
}

/// Build the row + its AreaTable rows, then run ONE row iteration of the binary's own loop.
fn binary_row(
    emu: &mut Emu,
    e: &Env,
    displayed: u32,
    ovl_id: u32,
    row_map_area: u32,
    slots: &[(u32, i32, i32)],
) -> (u32, u32) {
    let row = emu.alloc(0x44);
    emu.write_u32(row, ovl_id);
    emu.write_u32(row + 4, row_map_area);
    for s in 0..4u64 {
        emu.write_u32(row + 8 + 4 * s, 0);
    }
    for (s, &(aid, bit, lvl)) in slots.iter().enumerate() {
        emu.write_u32(row + 8 + 4 * s as u64, aid);
        if aid != 0 {
            let ar = emu.alloc(100);
            emu.write_u32(ar + 0x0c, bit as u32);
            emu.write_u32(ar + 0x28, lvl as u32);
            emu.write_u32(e.idx + 4 * u64::from(aid), ar as u32);
        }
    }
    emu.write_u32(CNT, 0);
    emu.write_u32(CAP, 16);
    emu.write_u32(e.out, 0);
    emu.write_u32(e.ebp - 0x14, displayed);
    emu.write_u32(e.ebp - 0x4, e.player as u32);
    emu.set_reg(RegisterX86::EBP, e.ebp);
    emu.set_reg(RegisterX86::ESP, e.ebp - 0x80);
    emu.set_reg(RegisterX86::EAX, row);
    emu.run(0x4a6b6b, 0x4a6cc4);
    (emu.read_u32(CNT), emu.read_u32(e.out))
}

#[test]
fn worldmap_overlay_reveal_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let env = setup(&mut emu);
    let mut total = 0u32;

    // (displayed, ovlId, rowMapArea, slots, exploredBits, guidOk) — every arm of the gate, with the
    // deterministic boundary probes the fuzz can never land on.
    #[allow(clippy::type_complexity)]
    let cases: &[(&str, u32, u32, u32, &[(u32, i32, i32)], &[i32], bool)] = &[
        (
            "mapAreaID mismatch",
            121,
            984,
            999,
            &[(1119, 369, 48)],
            &[369],
            true,
        ),
        ("bit set", 121, 984, 121, &[(1119, 369, 48)], &[369], true),
        ("bit clear", 121, 984, 121, &[(1119, 369, 48)], &[], true),
        (
            "ExplorationLevel -1",
            121,
            984,
            121,
            &[(1119, 369, -1)],
            &[],
            true,
        ),
        (
            "ExplorationLevel 0 (boundary)",
            121,
            984,
            121,
            &[(1119, 369, 0)],
            &[],
            true,
        ),
        (
            "AreaBit 2047 (byteIdx 255)",
            121,
            984,
            121,
            &[(1119, 2047, 48)],
            &[],
            true,
        ),
        (
            "AreaBit 2048 (byteIdx 256)",
            121,
            984,
            121,
            &[(1119, 2048, 48)],
            &[],
            true,
        ),
        ("AreaBit -1", 121, 984, 121, &[(1119, -1, 48)], &[], true),
        ("AreaBit -7", 121, 984, 121, &[(1119, -7, 48)], &[], true),
        ("AreaBit -8", 121, 984, 121, &[(1119, -8, 48)], &[], true),
        ("AreaBit -9", 121, 984, 121, &[(1119, -9, 48)], &[], true),
        (
            "slot1 set, slot0 clear",
            121,
            990,
            121,
            &[(1101, 396, 44), (2519, 937, 41)],
            &[937],
            true,
        ),
        (
            "four slots clear",
            121,
            990,
            121,
            &[
                (1101, 396, 44),
                (2519, 937, 41),
                (1102, 300, 44),
                (1104, 301, 44),
            ],
            &[],
            true,
        ),
        (
            "slot0 areaID 0, slot1 set",
            121,
            990,
            121,
            &[(0, 0, 0), (2519, 937, 41)],
            &[937],
            true,
        ),
        (
            "guid mismatch, bit set",
            121,
            984,
            121,
            &[(1119, 369, 48)],
            &[369],
            false,
        ),
        (
            "guid mismatch, ExpLvl -1",
            121,
            984,
            121,
            &[(1119, 369, -1)],
            &[],
            false,
        ),
    ];

    for &(label, displayed, ovl, row_area, slots, bits, guid_ok) in cases {
        emu.write_u32(
            env.player + 0x8,
            if guid_ok {
                env.desc_good as u32
            } else {
                env.desc_bad as u32
            },
        );
        let model = set_explored(&mut emu, &env, bits);
        let (count, appended) = binary_row(&mut emu, &env, displayed, ovl, row_area, slots);
        let mut rust_slots = [None; 4];
        for (i, &(aid, bit, lvl)) in slots.iter().enumerate() {
            if aid != 0 {
                rust_slots[i] = Some(AreaSlot {
                    area_bit: bit,
                    exploration_level: lvl,
                });
            }
        }
        let ours = row_revealed(row_area, displayed, &rust_slots, guid_ok, &model);
        assert_eq!(
            count == 1,
            ours,
            "{label}: binary count={count}, ours={ours}"
        );
        if ours {
            assert_eq!(appended, ovl, "{label}: appended id");
        }
        total += 1;
    }

    // `AreaBit == -1` must stay hidden even with EVERY explored bit set — the 5-bit shift mask.
    emu.write_u32(env.player + 0x8, env.desc_good as u32);
    let all = [0xffu8; EXPLORED_ZONES_BYTES];
    emu.write(env.block + 0xe6c, &all);
    let (count, _) = binary_row(&mut emu, &env, 121, 984, 121, &[(1119, -1, 48)]);
    assert_eq!(
        count == 1,
        slot_reveals(
            AreaSlot {
                area_bit: -1,
                exploration_level: 48
            },
            true,
            &all
        ),
        "AreaBit -1 with all 2048 bits set"
    );
    assert_eq!(count, 0, "AreaBit -1 must stay hidden with all bits set");
    total += 1;

    eprintln!("worldmap_overlay_reveal_gate: {total} arms bit-exact vs WoW.exe");
}

/// The map-level gate at `0x4a6af6`/`0x4a6afa`: the zone map is the only level with overlays.
#[test]
fn worldmap_overlay_map_level_gate() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let env = setup(&mut emu);
    let cont = emu.alloc(0x10100);
    emu.write_u32(cont + 4, 13); // the continent's own WorldMapArea id
    let zones = emu.alloc(0x40);
    emu.write_u32(zones, 121); // zone 0 -> Feralas' WorldMapArea id
    emu.write_u32(cont + 0x10, zones as u32);
    emu.write_u32(CONT_TBL, cont as u32);
    emu.write_u32(CONT_N, 1);
    let row = emu.alloc(0x44);
    emu.write_u32(row, 984);
    emu.write_u32(row + 4, 121);
    emu.write_u32(row + 8, 1119);
    let ar = emu.alloc(100);
    emu.write_u32(ar + 0x0c, 369);
    emu.write_u32(ar + 0x28, 48);
    emu.write_u32(env.idx + 4 * 1119, ar as u32);
    emu.write_u32(OVL_ROWS, row as u32);
    emu.write_u32(OVL_COUNT, 1);
    set_explored(&mut emu, &env, &[369]);

    // (continentIndex, zoneIndex, explicitWmaId, expected count)
    for &(c, z, x, want) in &[
        (0i32, 0i32, -1i32, 1u32), // zone map, bit set
        (0, -1, -1, 0),            // continent map
        (-1, -1, -1, 0),           // world map
        (-1, -1, 121, 1),          // the continentIndex == -2 explicit-WorldMapArea path
    ] {
        emu.write_u32(SEL_CONT, c as u32);
        emu.write_u32(SEL_ZONE, z as u32);
        emu.write_u32(SEL_EXPLICIT, x as u32);
        emu.write_u32(CNT, 0);
        emu.write_u32(CAP, 16);
        emu.write_u32(env.out, 0);
        emu.write_u32(env.ebp - 0x4, env.player as u32);
        emu.set_reg(RegisterX86::EBP, env.ebp);
        emu.set_reg(RegisterX86::ESP, env.ebp - 0x80);
        emu.set_reg(RegisterX86::EDI, 0); // the region's literal zero (0x4a69ce xor edi,edi)
        emu.run(0x4a6ad9, 0x4a6cdf);
        assert_eq!(
            emu.read_u32(CNT),
            want,
            "map level (cont {c}, zone {z}, explicit {x})"
        );
    }

    // No local player object -> empty list, and the count is zeroed on that leg too.
    emu.write_u32(CNT, 7);
    emu.write_u32(env.ebp - 0x4, 0);
    emu.set_reg(RegisterX86::EBP, env.ebp);
    emu.set_reg(RegisterX86::ESP, env.ebp - 0x80);
    emu.set_reg(RegisterX86::EDI, 0);
    emu.run(0x4a6ad9, 0x4a6cdf);
    assert_eq!(
        emu.read_u32(CNT),
        0,
        "null player must still zero the count (0x4a6adc)"
    );

    eprintln!("worldmap_overlay_map_level_gate: 5 arms bit-exact vs WoW.exe");
}
