//! `IsUnitPlayerDriven` (`0x5fa550`) and `ClntObjMgrObjectPtr` (`0x468460`) against the exe, each
//! run whole (`systems/objectlayer/units.md#player-driven-units`,
//! `systems/objectlayer/manager.md#finding-an-object`).
//!
//! Objects are written into the emulator in the client's layout: `+8` points at the descriptors,
//! whose type word is at `+8`, and `+0x110` at the unit field block, which starts at descriptors
//! `+0x18`. The GUID lookup `GetObjectPtr` (`0x464870`) is stubbed with a table of the test's
//! objects, which is what the object manager's hash walk would return; everything else, including
//! the recursion through the possessor, is the exe's own code. The result of `0x5fa550` is the
//! byte `al`: its early return leaves the rest of `eax` as the flags.
//!
//! The first test crosses the six flags the function reads with the type words and the charm
//! fields, then fuzzes possessor chains of up to three units over the flags, the type words and
//! the GUIDs (live, dangling and 0). The second crosses masks with type words.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::player_driven::{is_unit_player_driven, object_ptr, Unit};
use common::next;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const IS_PLAYER_DRIVEN: u64 = 0x5fa550;
const OBJECT_PTR: u64 = 0x468460;
const GET_OBJECT_PTR: u64 = 0x464870;

/// A slot of the pool: an object, its descriptors and its field block, in the emulator.
struct Slot {
    obj: u64,
    desc: u64,
    guid: u64,
}

const SLOTS: usize = 4;
/// A GUID that no slot holds.
const DANGLING: u64 = 0x0000_0099_0000_7777;

fn guid_of(slot: usize) -> u64 {
    0x0000_0004_0000_0100 + slot as u64
}

fn pool(e: &mut Emu) -> Vec<Slot> {
    (0..SLOTS)
        .map(|i| Slot {
            obj: e.alloc(0x120),
            desc: e.alloc(0x100),
            guid: guid_of(i),
        })
        .collect()
}

fn put(e: &mut Emu, s: &Slot, u: &Unit) {
    e.write_u32(s.obj + 8, s.desc as u32);
    e.write_u32(s.obj + 0x110, (s.desc + 0x18) as u32);
    e.write_u32(s.desc + 8, u.type_word);
    let block = s.desc + 0x18;
    e.write_u32(block + 0xa0, u.flags);
    e.write(block + 0x10, &u.charmed_by.to_le_bytes());
    e.write(block + 0x20, &u.created_by.to_le_bytes());
}

/// Stubs `GetObjectPtr`: the object VA for a GUID in `live`, else 0 (which includes GUID 0).
fn install_lookup(e: &mut Emu) -> Rc<RefCell<HashMap<u64, u32>>> {
    let live: Rc<RefCell<HashMap<u64, u32>>> = Rc::default();
    let table = live.clone();
    e.stub_fn(GET_OBJECT_PTR, 2, move |io| {
        let guid = (u64::from(io.arg(1)) << 32) | u64::from(io.arg(0));
        table.borrow().get(&guid).copied().unwrap_or(0)
    });
    live
}

/// The flags the function reads: 0x1, 0x2, 0x4, confused, fleeing and possessed.
const FLAG_BITS: [u32; 6] = [0x1, 0x2, 0x4, 0x0040_0000, 0x0080_0000, 0x0100_0000];
/// Stunned and bit 11, which it does not read.
const NOISE: u32 = 0x0004_0000 | 0x0000_0800;
/// Type words: a creature, a player, an item, a bare player bit, a game object, none.
const TYPES: [u32; 6] = [0x9, 0x19, 0x3, 0x10, 0x21, 0x0];

/// Flags with each read bit set one time in four, the possessed bit one in three, and noise.
fn draw_flags(s: &mut u32) -> u32 {
    let mut f = 0;
    for (i, bit) in FLAG_BITS.iter().enumerate() {
        let every = if i == 5 { 3 } else { 4 };
        if (next(s) >> 16).is_multiple_of(every) {
            f |= bit;
        }
    }
    if (next(s) >> 16).is_multiple_of(2) {
        f |= NOISE;
    }
    f
}

type Tally = [u32; 2];

/// How many possessor hops the chain from the first unit follows before a unit that is not
/// possessed.
fn possessions(units: &[Unit]) -> usize {
    let mut depth = 0;
    let mut u = units[0];
    while u.flags & 0x0100_0000 != 0 {
        let g = if u.charmed_by != 0 {
            u.charmed_by
        } else {
            u.created_by
        };
        let Some(j) = (0..units.len()).find(|&j| guid_of(j) == g) else {
            break;
        };
        u = units[j];
        depth += 1;
    }
    depth
}

/// A chain of `n` units that answers true: each but the last is possessed by the next, named by
/// `UNIT_FIELD_CHARMEDBY` or, with that 0, by `UNIT_FIELD_CREATEDBY`, and the last is a free player.
fn ideal_chain(s: &mut u32, n: usize) -> Vec<Unit> {
    const RIDERS: [u32; 4] = [0, 0x2 | 0x4, 0x2 | 0x0040_0000 | 0x0080_0000, NOISE];
    (0..n)
        .map(|i| {
            let last = i + 1 == n;
            let mut u = Unit {
                type_word: if last {
                    0x19
                } else {
                    [0x9, 0x19][(next(s) >> 16) as usize % 2]
                },
                flags: RIDERS[(next(s) >> 16) as usize % RIDERS.len()],
                ..Unit::default()
            };
            if !last {
                u.flags |= 0x0100_0000;
                if (next(s) >> 16).is_multiple_of(2) {
                    u.charmed_by = guid_of(i + 1);
                    u.created_by = [0, DANGLING][(next(s) >> 16) as usize % 2];
                } else {
                    u.created_by = guid_of(i + 1);
                }
            }
            u
        })
        .collect()
}

#[test]
fn is_unit_player_driven_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let slots = pool(&mut e);
    let live = install_lookup(&mut e);

    let mut model: HashMap<u64, Unit> = HashMap::new();
    // Writes `units` into the first slots, makes only those slots objects, runs the exe on the
    // first and compares.
    let mut run = |e: &mut Emu, units: &[Unit], tally: &mut Tally| {
        model.clear();
        let mut l = live.borrow_mut();
        l.clear();
        for (s, u) in slots.iter().zip(units) {
            put(e, s, u);
            model.insert(s.guid, *u);
            l.insert(s.guid, s.obj as u32);
        }
        drop(l);
        let table = |g: u64| model.get(&g).copied();
        let ours = is_unit_player_driven(&units[0], &table);
        let bin = e.call(IS_PLAYER_DRIVEN, slots[0].obj as u32, 0, &[]) & 0xff;
        assert_eq!(
            bin,
            u32::from(ours),
            "binary {bin} ours {ours} for {units:x?}"
        );
        tally[usize::from(ours)] += 1;
    };

    // Phase 1: one unit, every combination of the six flags, with the type words, the charm
    // fields and the unread flags. Nothing it names is live.
    let mut single: Tally = [0; 2];
    for flags in 0u32..64 {
        let f =
            FLAG_BITS.iter().enumerate().fold(
                0u32,
                |a, (i, b)| if flags >> i & 1 != 0 { a | b } else { a },
            );
        for &type_word in &TYPES {
            for charmed_by in [0, DANGLING] {
                for created_by in [0, DANGLING] {
                    for noise in [0, NOISE] {
                        let u = Unit {
                            type_word,
                            flags: f | noise,
                            charmed_by,
                            created_by,
                        };
                        run(&mut e, &[u], &mut single);
                    }
                }
            }
        }
    }

    // Phase 2: a possessed root over up to three more units, each possessed in turn or not. A
    // unit points only at later slots, at 0 or at a GUID no slot holds, so there is no cycle.
    let mut chains: Tally = [0; 2];
    let mut s = 0x5fa5_5000u32;
    for _ in 0..8000 {
        let n = 2 + (next(&mut s) >> 16) as usize % (SLOTS - 1);
        let mut units = Vec::new();
        for i in 0..n {
            let mut flags = draw_flags(&mut s);
            if i == 0 {
                flags |= 0x0100_0000;
            }
            let refs: Vec<u64> = [0, DANGLING]
                .into_iter()
                .chain((i + 1..n).map(guid_of))
                .collect();
            let charmed_by = refs[(next(&mut s) >> 16) as usize % refs.len()];
            let created_by = refs[(next(&mut s) >> 16) as usize % refs.len()];
            let type_word = TYPES[(next(&mut s) >> 16) as usize % TYPES.len()];
            units.push(Unit {
                type_word,
                flags,
                charmed_by,
                created_by,
            });
        }
        run(&mut e, &units, &mut chains);
    }

    // Phase 3: chains built to be true, two to four units long, with at most one change that a
    // real client could meet: a blocking flag, a missing unit or player bit, a charm field, a
    // GUID no object holds, a cleared possession.
    let mut built: Tally = [0; 2];
    let mut deepest = 0usize;
    for _ in 0..6000 {
        let n = 2 + (next(&mut s) >> 16) as usize % (SLOTS - 1);
        let mut units = ideal_chain(&mut s, n);
        let k = (next(&mut s) >> 16) as usize % n;
        match (next(&mut s) >> 16) % 9 {
            0 => units[k].flags = (units[k].flags & !0x2) | 0x4,
            1 => units[k].flags = (units[k].flags & !0x2) | 0x0040_0000,
            2 => units[k].flags = (units[k].flags & !0x2) | 0x0080_0000,
            3 => units[n - 1].flags |= 0x1,
            4 => units[k].type_word = 0x1,
            5 => units[k].type_word = 0x10,
            6 => units[n - 1].type_word = 0x9,
            7 => {
                units[k].charmed_by = DANGLING;
                units[k].created_by = if k + 1 < n { guid_of(k + 1) } else { 0 };
            }
            _ => units[k].flags &= !0x0100_0000,
        }
        let before = built[1];
        run(&mut e, &units, &mut built);
        if built[1] > before {
            deepest = deepest.max(possessions(&units));
        }
    }
    eprintln!(
        "is_unit_player_driven: single {single:?} chains {chains:?} built {built:?} deepest true chain {deepest}"
    );
    assert!(
        built[0] > 100 && built[1] > 100,
        "phase 3 needs both answers"
    );
    assert!(
        single[0] > 100 && single[1] >= 20,
        "phase 1 needs both answers"
    );
    assert!(
        chains[0] > 100 && chains[1] >= 20,
        "phase 2 needs both answers"
    );
    assert!(
        deepest >= 3,
        "a true answer must come through three possessions"
    );
}

#[test]
fn object_ptr_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let slots = pool(&mut e);
    let live = install_lookup(&mut e);
    let masks = [
        0u32,
        1,
        2,
        4,
        8,
        0x10,
        0x18,
        0x19,
        0x20,
        0x80,
        0x8000_0000,
        0xffff_fff7,
        0xffff_ffff,
    ];
    let types = [
        0u32,
        1,
        3,
        7,
        9,
        0x19,
        0x21,
        0x41,
        0x81,
        0x8000_0000,
        0xffff_ffff,
    ];
    let mut found = 0u32;
    let mut cases = 0u32;
    for &mask in &masks {
        for &type_word in &types {
            let u = Unit {
                type_word,
                ..Unit::default()
            };
            put(&mut e, &slots[0], &u);
            for (label, guid, present) in [
                ("live", slots[0].guid, true),
                ("absent", DANGLING, false),
                ("zero", 0, false),
            ] {
                {
                    let mut l = live.borrow_mut();
                    l.clear();
                    if present {
                        l.insert(slots[0].guid, slots[0].obj as u32);
                    }
                }
                let table = |g: u64| (present && g == slots[0].guid).then_some(u);
                let ours = object_ptr(&table, &|o: &Unit| o.type_word, mask, guid);
                let want = if ours.is_some() {
                    slots[0].obj as u32
                } else {
                    0
                };
                // ecx = mask; edx and the third stack dword are a source path and a line number
                // that the function never reads, so other values must give the same answer.
                for (edx, line) in [
                    (0x860c0c, 0x1df),
                    (0, 0),
                    (0xdead_beef, 0xffff_ffff),
                    (1, 7),
                ] {
                    let bin = e.call(
                        OBJECT_PTR,
                        mask,
                        edx,
                        &[guid as u32, (guid >> 32) as u32, line],
                    );
                    assert_eq!(
                        bin, want,
                        "{label} mask {mask:#x} type {type_word:#x} edx {edx:#x} line {line:#x}: \
                         binary {bin:#x} ours {want:#x}"
                    );
                }
                found += u32::from(ours.is_some());
                cases += 1;
            }
        }
    }
    eprintln!("object_ptr: {cases} cases, {found} found");
    assert!(found > 10 && found < cases, "both outcomes must occur");
}
