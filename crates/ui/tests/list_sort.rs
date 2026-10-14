//! The list comparators that call a collator (`CompareStationery`, `CompareBookSpells`, the auction
//! list comparator, `CompareGuildMembers`, the comparator of `0x4d3070`, `CompareReputationHeaders`,
//! `CompareReputationRows`, `CompareTrainerRows`, `CompareTradeskillTrainerRows`) against the exe: each
//! runs in the emulator over records, objects and name strings the test builds in scratch memory and
//! the `.data` and `.bss` globals, and the returned `int` must match the transcription exactly, the
//! sign or the real collator's value.
//!
//! Two boundaries are pinned. The record resolvers `0x55ba30` and `0x55f080` are overwritten with
//! identity routines (`mov eax,[esp+4]; ret N`), so a record's field is the resolved pointer, `0` for
//! null, which lets a case hold two distinct objects. The local player GUID (`0x468550`) is a fixed
//! `edx:eax`, and the routines `0x4b24f0`, `0x4b3c30` and `0x4d09b0` read a field. The locale
//! collators `0x64a4c0` and `0x64a480` are not replaced; they run in the emulator, and the Rust side
//! gets the same value by calling them through the `collate` closure. The locale index `0xc0e080` is 0.

#![allow(clippy::too_many_arguments, clippy::float_cmp, clippy::type_complexity)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::list_sort::{self as sc, *};

// global VAs (mapped in `.data`'s zero-filled virtual tail)
const LOCALE: u64 = 0x00c0_e080;
// 4d0d50
const PREGATE: u64 = 0x00b7_3124;
const LIVE_TABLE: u64 = 0x00b7_2680;
const C0E048: u64 = 0x00c0_e048;
const C0E04C: u64 = 0x00c0_e04c;
const C0DEF4: u64 = 0x00c0_def4;
const C0DEF8: u64 = 0x00c0_def8;
// 4d3070
const B73170: u64 = 0x00b7_3170;
const B731BC: u64 = 0x00b7_31bc;
const C0D92C: u64 = 0x00c0_d92c;
const C0D930: u64 = 0x00c0_d930;
// 4d85c0
const B739C0: u64 = 0x00b7_39c0;
const B73A14: u64 = 0x00b7_3a14;
// shared player name-cache (4b30c0 / 4d85c0 / 4d8760)
const C0D788: u64 = 0x00c0_d788;
const C0D78C: u64 = 0x00c0_d78c;
// 4d5dc0 / 4d5e70
const C0DD50: u64 = 0x00c0_dd50;
const C0DD54: u64 = 0x00c0_dd54;
const MASK_84A0A4: u64 = 0x0084_a0a4;

// collator addresses
const COLL_A4C0: u64 = 0x0064_a4c0; // default
const COLL_A480: u64 = 0x0064_a480; // player-name-cache family

fn patch(emu: &mut Emu, addr: u64, code: &[u8]) {
    emu.write(addr, code);
}
/// `mov eax,[esp+4]; ret 0x14` — identity for `0x55ba30` (5 stack args, callee-cleans).
const GUID_ID: &[u8] = &[0x8b, 0x44, 0x24, 0x04, 0xc2, 0x14, 0x00];
/// `mov eax,[esp+4]; ret 0x18` — identity for `0x55f080` (6 stack args).
const NOTE_ID: &[u8] = &[0x8b, 0x44, 0x24, 0x04, 0xc2, 0x18, 0x00];

/// Allocate a NUL-terminated string in scratch; returns its VA.
fn cstr(emu: &mut Emu, s: &[u8]) -> u64 {
    let a = emu.alloc(s.len() as u64 + 1);
    emu.write(a, s);
    a
}
/// Build a single-deref name-cache table (`table[idx] = obj_va`); writes its base to `ptr_va` and
/// `max` to `max_va`. Returns the table VA.
fn build_cache(emu: &mut Emu, objs: &[u64], ptr_va: u64, max_va: u64) -> u64 {
    let arr = emu.alloc(4 * objs.len() as u64);
    for (i, &o) in objs.iter().enumerate() {
        emu.write_u32(arr + 4 * i as u64, o as u32);
    }
    emu.write_u32(ptr_va, arr as u32);
    emu.write_u32(max_va, (objs.len() as u32).wrapping_sub(1));
    arr
}
/// Build a double-deref id list (`*(*(base + i*4)) = ids[i]`); writes base to `ptr_va`, count to
/// `count_va`.
fn build_idlist(emu: &mut Emu, ids: &[u32], ptr_va: u64, count_va: u64) {
    let arr = emu.alloc(4 * ids.len().max(1) as u64);
    for (i, &id) in ids.iter().enumerate() {
        let node = emu.alloc(4);
        emu.write_u32(node, id);
        emu.write_u32(arr + 4 * i as u64, node as u32);
    }
    emu.write_u32(ptr_va, arr as u32);
    emu.write_u32(count_va, ids.len() as u32);
}

// `CompareStationery` (`0x4ada90`), the mail stationery list

#[test]
fn sortcmp_4ada90() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    patch(&mut emu, 0x55ba30, GUID_ID);
    emu.write_u32(LOCALE, 0);
    let rec_a = emu.alloc(0x40);
    let rec_b = emu.alloc(0x40);
    let obj_a = emu.alloc(0x40);
    let obj_b = emu.alloc(0x40);
    let slot_a = emu.alloc(4);
    let slot_b = emu.alloc(4);
    emu.write_u32(slot_a, rec_a as u32);
    emu.write_u32(slot_b, rec_b as u32);
    let na = cstr(&mut emu, b"Anduin");
    let nb = cstr(&mut emu, b"Bolvar");

    // (f8,f10,f14)_a, _b, oa(f24,name), ob, label
    type Obj = Option<(i32, u64)>;
    let cases: &[((u32, u32, u32), (u32, u32, u32), Obj, Obj, &str)] = &[
        ((1, 0, 0), (0, 0, 0), None, None, "active-a"),
        ((0, 0, 0), (1, 0, 0), None, None, "active-b"),
        ((0, 0, 0), (0, 0, 0), None, None, "both-inactive-nullobj"),
        ((1, 1, 0), (1, 0, 0), None, None, "inprog-a"),
        ((1, 0, 0), (1, 0, 1), None, None, "inprog-b"),
        (
            (1, 1, 0),
            (1, 1, 0),
            Some((5, na)),
            Some((3, nb)),
            "obj+24 desc hi",
        ),
        (
            (1, 1, 0),
            (1, 1, 0),
            Some((3, na)),
            Some((5, nb)),
            "obj+24 desc lo",
        ),
        (
            (1, 1, 0),
            (1, 1, 0),
            Some((4, na)),
            Some((4, nb)),
            "name differ",
        ),
        (
            (1, 1, 0),
            (1, 1, 0),
            Some((4, na)),
            Some((4, na)),
            "name tie ->0",
        ),
        ((1, 1, 0), (1, 1, 0), Some((4, na)), None, "objA only"),
        ((1, 1, 0), (1, 1, 0), None, Some((4, nb)), "objB only"),
    ];
    let mut nonzero = 0;
    for (a, b, oa, ob, lab) in cases {
        let wr = |emu: &mut Emu, rec: u64, f: &(u32, u32, u32), o: &Obj, obj: u64| {
            emu.write_u32(rec + 8, f.0);
            emu.write_u32(rec + 0x10, f.1);
            emu.write_u32(rec + 0x14, f.2);
            match o {
                Some((f24, name)) => {
                    emu.write_u32(obj + 0x24, *f24 as u32);
                    emu.write_u32(obj + 8, *name as u32);
                    emu.write_u32(rec + 4, obj as u32);
                }
                None => emu.write_u32(rec + 4, 0),
            }
        };
        wr(&mut emu, rec_a, a, oa, obj_a);
        wr(&mut emu, rec_b, b, ob, obj_b);
        let bin = emu.call(0x4ada90, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
        let ra = StationeryRecFields {
            f8: a.0,
            f10: a.1,
            f14: a.2,
        };
        let rb = StationeryRecFields {
            f8: b.0,
            f10: b.1,
            f14: b.2,
        };
        let oar = oa.map(|(f24, name)| StationeryItemTemplate {
            f24,
            name: name as u32,
        });
        let obr = ob.map(|(f24, name)| StationeryItemTemplate {
            f24,
            name: name as u32,
        });
        let mine = sc::sortcmp_4ada90(&ra, &rb, oar, obr, &mut |ka, kb| {
            emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32
        });
        assert_eq!(bin, mine, "4ada90 [{lab}]: bin {bin} vs rust {mine}");
        if mine != 0 {
            nonzero += 1;
        }
    }
    assert!(nonzero >= 8, "4ada90: too few decisive cases ({nonzero})");
}

// `CompareBookSpells` (`0x4b30c0`)

#[test]
fn sortcmp_4b30c0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // 0x4b24f0 = `mov eax,ecx; ret` (ecx = *obj = obj[0], the tab position).
    patch(&mut emu, 0x4b24f0, &[0x8b, 0xc1, 0xc3]);
    // 0x4b3c30 = `mov eax,[ecx+4]; ret` (ecx = obj, the rank at obj+4).
    patch(&mut emu, 0x4b3c30, &[0x8b, 0x41, 0x04, 0xc3]);
    emu.write_u32(LOCALE, 0);
    let obj_a = emu.alloc(0x220);
    let obj_b = emu.alloc(0x220);
    // name-cache: idx 0 -> obj_a, 1 -> obj_b, 2 -> null.
    build_cache(&mut emu, &[obj_a, obj_b, 0], C0D788, C0D78C);
    // 4b30c0 is SINGLE-deref: `*param` IS the name-cache index directly (no record struct), so
    // each "record" is just a 4-byte int holding the index, passed by pointer.
    let rec_a = emu.alloc(0x10);
    let rec_b = emu.alloc(0x10);
    let na = cstr(&mut emu, b"Aaa");
    let nb = cstr(&mut emu, b"Bbb");

    // (idx, score, level, name1e0, name204), per side
    type S = (i32, u32, u32, u64, u64);
    let cases: &[(S, S, &str)] = &[
        ((2, 0, 0, na, na), (0, 0, 0, na, nb), "a missing -> 0"),
        ((0, 0, 0, na, na), (2, 0, 0, na, nb), "b missing -> 0"),
        ((0, 5, 0, na, na), (1, 9, 0, nb, nb), "score asc"),
        ((0, 9, 0, na, na), (1, 5, 0, nb, nb), "score desc"),
        ((0, 5, 0, na, na), (1, 5, 0, nb, nb), "name1e0 differ"),
        (
            (0, 5, 7, na, na),
            (1, 5, 9, na, nb),
            "level asc (name1e0 tie)",
        ),
        ((0, 5, 9, na, na), (1, 5, 7, na, nb), "level desc"),
        ((0, 5, 0, na, na), (1, 5, 9, na, nb), "level a=0 -> name204"),
        (
            (0, 5, 7, na, na),
            (1, 5, 7, na, nb),
            "level equal -> name204",
        ),
        ((0, 5, 7, na, na), (1, 5, 7, na, na), "name204 tie -> 0"),
    ];
    for (a, b, lab) in cases {
        let wr = |emu: &mut Emu, rec: u64, obj: u64, s: &S| {
            emu.write_u32(rec, s.0 as u32); // idx
            emu.write_u32(obj, s.1); // score (obj[0])
            emu.write_u32(obj + 4, s.2); // level (obj[4])
            emu.write_u32(obj + 0x1e0, s.3 as u32);
            emu.write_u32(obj + 0x204, s.4 as u32);
        };
        wr(&mut emu, rec_a, obj_a, a);
        wr(&mut emu, rec_b, obj_b, b);
        let bin = emu.call(0x4b30c0, 0, 0, &[rec_a as u32, rec_b as u32]) as i32;
        let mk = |s: &S| {
            (s.0 >= 0 && s.0 <= 1).then_some(BookSpell {
                tab_pos: s.1,
                rank: s.2,
                name: s.3 as u32,
                rank_text: s.4 as u32,
            })
        };
        let oa = mk(a);
        let ob = mk(b);
        let mine = sc::sortcmp_4b30c0(oa, ob, &mut |ka, kb| {
            emu.call(COLL_A480, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32
        });
        assert_eq!(bin, mine, "4b30c0 [{lab}]: bin {bin} vs rust {mine}");
    }
}

// 0x4cd940 — GUILD (mode switch 0..7)

#[test]
fn sortcmp_4cd940() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    patch(&mut emu, 0x55ba30, GUID_ID);
    patch(&mut emu, 0x55f080, NOTE_ID);
    // 0x468550 -> fixed local-player GUID edx:eax = (HI<<32)|LO.
    let (loc_lo, loc_hi) = (0xAAAA_0001u32, 0xBBBB_0002u32);
    let mut isle = vec![0xb8u8]; // mov eax, imm32
    isle.extend_from_slice(&loc_lo.to_le_bytes());
    isle.push(0xba); // mov edx, imm32
    isle.extend_from_slice(&loc_hi.to_le_bytes());
    isle.push(0xc3); // ret
    patch(&mut emu, 0x468550, &isle);
    emu.write_u32(LOCALE, 0);
    let rec_a = emu.alloc(0x60);
    let rec_b = emu.alloc(0x60);
    let obj_a = emu.alloc(0x40);
    let obj_b = emu.alloc(0x40);
    let na = cstr(&mut emu, b"Aaa");
    let nb = cstr(&mut emu, b"Bbb");
    let nota = cstr(&mut emu, b"NoteA");
    let notb = cstr(&mut emu, b"NoteZ");

    // helper to set obj (recX+8 -> obj or 0) + obj fields
    let set_obj =
        |emu: &mut Emu, rec: u64, obj: u64, present: bool, f3c: i32, f1c: i32, name: u64| {
            if present {
                emu.write_u32(obj + 0x3c, f3c as u32);
                emu.write_u32(obj + 0x1c, f1c as u32);
                emu.write_u32(obj + 8, name as u32);
                emu.write_u32(rec + 8, obj as u32);
            } else {
                emu.write_u32(rec + 8, 0);
            }
        };

    let mut modes_hit = [false; 8];
    // Each case: (mode, setup closure-ish via tuple). We build records inline per case.
    struct Case {
        mode: u32,
        // record fields
        a: AuctionRec,
        b: AuctionRec,
        oa: Option<AuctionItem>,
        ob: Option<AuctionItem>,
        note_a: Option<u64>,
        note_b: Option<u64>,
        lab: &'static str,
    }
    let g = |f40, f28, f34, f30, f38, f3c| AuctionRec {
        f40,
        f28,
        f34,
        f30,
        f38,
        f3c,
    };
    let z = g(0, 0, 0, 0, 0, 0);
    let cases = vec![
        // mode 0: obj+0x3c desc (max>=1)
        Case {
            mode: 0,
            a: z,
            b: z,
            oa: Some(AuctionItem {
                f3c: 5,
                f1c: 0,
                name8: na as u32,
            }),
            ob: Some(AuctionItem {
                f3c: 2,
                f1c: 0,
                name8: nb as u32,
            }),
            note_a: None,
            note_b: None,
            lab: "m0 hi",
        },
        Case {
            mode: 0,
            a: z,
            b: z,
            oa: Some(AuctionItem {
                f3c: 0,
                f1c: 0,
                name8: na as u32,
            }),
            ob: Some(AuctionItem {
                f3c: -3,
                f1c: 0,
                name8: nb as u32,
            }),
            note_a: None,
            note_b: None,
            lab: "m0 clamp eq->0",
        },
        Case {
            mode: 0,
            a: z,
            b: z,
            oa: None,
            ob: Some(AuctionItem {
                f3c: 2,
                f1c: 0,
                name8: nb as u32,
            }),
            note_a: None,
            note_b: None,
            lab: "m0 null->0",
        },
        // mode 1: obj+0x1c asc
        Case {
            mode: 1,
            a: z,
            b: z,
            oa: Some(AuctionItem {
                f3c: 0,
                f1c: 3,
                name8: na as u32,
            }),
            ob: Some(AuctionItem {
                f3c: 0,
                f1c: 9,
                name8: nb as u32,
            }),
            note_a: None,
            note_b: None,
            lab: "m1 asc",
        },
        Case {
            mode: 1,
            a: z,
            b: z,
            oa: Some(AuctionItem {
                f3c: 0,
                f1c: 9,
                name8: na as u32,
            }),
            ob: Some(AuctionItem {
                f3c: 0,
                f1c: 9,
                name8: nb as u32,
            }),
            note_a: None,
            note_b: None,
            lab: "m1 eq->0",
        },
        // mode 2: rec+0x40 | +0x28 desc
        Case {
            mode: 2,
            a: g(10, 0, 0, 0, 0, 0),
            b: g(4, 0, 0, 0, 0, 0),
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m2 f40",
        },
        Case {
            mode: 2,
            a: g(0, 7, 0, 0, 0, 0),
            b: g(0, 2, 0, 0, 0, 0),
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m2 fallback f28",
        },
        // mode 3: rec+0x34 asc
        Case {
            mode: 3,
            a: g(0, 0, 2, 0, 0, 0),
            b: g(0, 0, 9, 0, 0, 0),
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m3 asc",
        },
        // mode 4: rec+0x30 desc
        Case {
            mode: 4,
            a: g(0, 0, 0, 2, 0, 0),
            b: g(0, 0, 0, 9, 0, 0),
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m4 desc",
        },
        // mode 5: online rank desc
        Case {
            mode: 5,
            a: g(0, 0, 0, 0, loc_lo, loc_hi),
            b: g(0, 0, 0, 0, 7, 0),
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m5 self vs other",
        },
        Case {
            mode: 5,
            a: g(0, 0, 0, 0, 7, 0),
            b: g(0, 0, 0, 0, 0, 0),
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m5 other vs empty",
        },
        Case {
            mode: 5,
            a: g(0, 0, 0, 0, 7, 0),
            b: g(0, 0, 0, 0, 7, 0),
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m5 eq->0",
        },
        // mode 6: name
        Case {
            mode: 6,
            a: z,
            b: z,
            oa: Some(AuctionItem {
                f3c: 0,
                f1c: 0,
                name8: na as u32,
            }),
            ob: Some(AuctionItem {
                f3c: 0,
                f1c: 0,
                name8: nb as u32,
            }),
            note_a: None,
            note_b: None,
            lab: "m6 name",
        },
        Case {
            mode: 6,
            a: z,
            b: z,
            oa: Some(AuctionItem {
                f3c: 0,
                f1c: 0,
                name8: na as u32,
            }),
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m6 null->0",
        },
        // mode 7: note
        Case {
            mode: 7,
            a: z,
            b: z,
            oa: None,
            ob: None,
            note_a: Some(nota),
            note_b: Some(notb),
            lab: "m7 note",
        },
        Case {
            mode: 7,
            a: z,
            b: z,
            oa: None,
            ob: None,
            note_a: None,
            note_b: Some(notb),
            lab: "m7 nullA -> (B!=0)",
        },
        Case {
            mode: 7,
            a: z,
            b: z,
            oa: None,
            ob: None,
            note_a: Some(nota),
            note_b: None,
            lab: "m7 nullB -> -1",
        },
        Case {
            mode: 7,
            a: z,
            b: z,
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m7 both null -> 0",
        },
        // unknown column
        Case {
            mode: 9,
            a: z,
            b: z,
            oa: None,
            ob: None,
            note_a: None,
            note_b: None,
            lab: "m9 default 0",
        },
    ];

    for c in &cases {
        // records
        let wr = |emu: &mut Emu, rec: u64, r: &AuctionRec| {
            emu.write_u32(rec + 0x40, r.f40 as u32);
            emu.write_u32(rec + 0x28, r.f28 as u32);
            emu.write_u32(rec + 0x34, r.f34);
            emu.write_u32(rec + 0x30, r.f30);
            emu.write_u32(rec + 0x38, r.f38);
            emu.write_u32(rec + 0x3c, r.f3c);
        };
        wr(&mut emu, rec_a, &c.a);
        wr(&mut emu, rec_b, &c.b);
        set_obj(
            &mut emu,
            rec_a,
            obj_a,
            c.oa.is_some(),
            c.oa.map_or(0, |o| o.f3c),
            c.oa.map_or(0, |o| o.f1c),
            c.oa.map_or(0, |o| o.name8 as u64),
        );
        set_obj(
            &mut emu,
            rec_b,
            obj_b,
            c.ob.is_some(),
            c.ob.map_or(0, |o| o.f3c),
            c.ob.map_or(0, |o| o.f1c),
            c.ob.map_or(0, |o| o.name8 as u64),
        );
        match c.note_a {
            Some(n) => emu.write_u32(rec_a + 0x20, n as u32),
            None => emu.write_u32(rec_a + 0x20, 0),
        }
        match c.note_b {
            Some(n) => emu.write_u32(rec_b + 0x20, n as u32),
            None => emu.write_u32(rec_b + 0x20, 0),
        }
        let bin = emu.call(0x4cd940, rec_a as u32, rec_b as u32, &[c.mode]) as i32;
        let mine = sc::sortcmp_4cd940(
            c.mode,
            &c.a,
            &c.b,
            c.oa,
            c.ob,
            c.note_a.map(|n| n as u32),
            c.note_b.map(|n| n as u32),
            loc_lo,
            loc_hi,
            &mut |ka, kb| emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
        );
        assert_eq!(bin, mine, "4cd940 [{}]: bin {bin} vs rust {mine}", c.lab);
        if c.mode < 8 {
            modes_hit[c.mode as usize] = true;
        }
    }
    assert!(
        modes_hit.iter().all(|&h| h),
        "4cd940: not all modes 0..7 hit: {modes_hit:?}"
    );
}

// `CompareGuildMembers` (`0x4d0d50`), driven by the live column table

#[test]
fn sortcmp_4d0d50() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // 0x4d09b0 = `lea eax,[ecx+0x4e]; ret` (returns this+0x4e, the inline col-7 name).
    patch(&mut emu, 0x4d09b0, &[0x8d, 0x41, 0x4e, 0xc3]);
    emu.write_u32(LOCALE, 0);

    let rec_a = emu.alloc(0x180);
    let rec_b = emu.alloc(0x180);
    let slot_a = emu.alloc(4);
    let slot_b = emu.alloc(4);
    emu.write_u32(slot_a, rec_a as u32);
    emu.write_u32(slot_b, rec_b as u32);
    // col3 objects (c0e048) name @+0x2c ; col4 objects (c0def4) name @+0x14
    let o3a = emu.alloc(0x40);
    let o3b = emu.alloc(0x40);
    let o4a = emu.alloc(0x40);
    let o4b = emu.alloc(0x40);
    let na = cstr(&mut emu, b"Aaa");
    let nb = cstr(&mut emu, b"Bbb");
    emu.write_u32(o3a + 0x2c, na as u32);
    emu.write_u32(o3b + 0x2c, nb as u32);
    emu.write_u32(o4a + 0x14, na as u32);
    emu.write_u32(o4b + 0x14, nb as u32);
    // c0e048 table: idx 0->o3a, 1->o3b ; c0def4: 0->o4a, 1->o4b
    build_cache(&mut emu, &[o3a, o3b], C0E048, C0E04C);
    build_cache(&mut emu, &[o4a, o4b], C0DEF4, C0DEF8);

    fn write_table(emu: &mut Emu, table: &[(u32, u32); 8]) {
        for (i, &(col, dir)) in table.iter().enumerate() {
            emu.write_u32(LIVE_TABLE + 8 * i as u64, col);
            emu.write_u32(LIVE_TABLE + 8 * i as u64 + 4, dir);
        }
    }
    let noop_rest = |col0: (u32, u32)| -> [(u32, u32); 8] {
        let mut t = [(5u32, 0u32); 8];
        t[0] = col0;
        t
    };

    // a GuildMemberRec builder
    let mk = |f38, f40, f3c, f44, f48, f150: f32| GuildMemberRec {
        f38,
        f40,
        f3c,
        f44,
        f48,
        f150,
        name8: 0, // set per write
        name4e: 0,
    };

    // per-column decisive pairs: differ in exactly that column's key.
    // name8/name4e are inline (rec+8 / rec+0x4e). We write distinct ASCII for the decisive col.
    let mut col_hit = [false; 8];
    for col in 0u32..=7 {
        for dir in [0u32, 1] {
            // base equal records
            let mut a = mk(0, 99, 99, 0, 1, 0.0); // f40/f3c=99 (OOB ⇒ col3/col4 skip by default)
            let mut b = mk(0, 99, 99, 0, 1, 0.0);
            let (mut a8, mut b8) = (b"Aaa".as_ref(), b"Aaa".as_ref());
            let (mut a4e, mut b4e) = (b"Aaa".as_ref(), b"Aaa".as_ref());
            let (mut o3a_idx, mut o3b_idx) = (99i32, 99i32);
            let (mut o4a_idx, mut o4b_idx) = (99i32, 99i32);
            match col {
                0 => {
                    a.f44 = 5;
                    b.f44 = 3;
                }
                1 => {
                    a.f38 = 7;
                    b.f38 = 3;
                }
                2 => {
                    a8 = b"Aaa";
                    b8 = b"Bbb";
                }
                3 => {
                    o3a_idx = 0;
                    o3b_idx = 1;
                    a.f40 = 0;
                    b.f40 = 1;
                }
                4 => {
                    o4a_idx = 0;
                    o4b_idx = 1;
                    a.f3c = 0;
                    b.f3c = 1;
                }
                5 => { /* no-op column: stays all-equal -> result 0 */ }
                6 => {
                    a.f48 = 0;
                    b.f48 = 5;
                } // one zero -> +1 (a sorts last)
                7 => {
                    a4e = b"Aaa";
                    b4e = b"Bbb";
                }
                _ => unreachable!(),
            }
            a.f40 = if col == 3 { 0 } else { a.f40 };
            // write records
            let wr = |emu: &mut Emu, rec: u64, r: &GuildMemberRec, s8: &[u8], s4e: &[u8]| {
                emu.write_u32(rec + 0x38, r.f38 as u32);
                emu.write_u32(rec + 0x40, r.f40 as u32);
                emu.write_u32(rec + 0x3c, r.f3c as u32);
                emu.write_u32(rec + 0x44, r.f44);
                emu.write_u32(rec + 0x48, r.f48);
                emu.write_u32(rec + 0x150, r.f150.to_bits());
                emu.write(rec + 8, s8);
                emu.write(rec + 8 + s8.len() as u64, &[0]);
                emu.write(rec + 0x4e, s4e);
                emu.write(rec + 0x4e + s4e.len() as u64, &[0]);
            };
            wr(&mut emu, rec_a, &a, a8, a4e);
            wr(&mut emu, rec_b, &b, b8, b4e);
            let table = noop_rest((col, dir));
            write_table(&mut emu, &table);
            emu.write_u32(PREGATE, 1); // pregate disabled
            let bin = emu.call(0x4d0d50, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
            // Rust inputs
            let ra = GuildMemberRec {
                name8: (rec_a + 8) as u32,
                name4e: (rec_a + 0x4e) as u32,
                ..a
            };
            let rb = GuildMemberRec {
                name8: (rec_b + 8) as u32,
                name4e: (rec_b + 0x4e) as u32,
                ..b
            };
            let res3 = |idx: i32, key: u64| (idx == 0 || idx == 1).then_some(key as u32);
            let obj3_a = res3(o3a_idx, na);
            let obj3_b = res3(o3b_idx, nb);
            let obj4_a = res3(o4a_idx, na);
            let obj4_b = res3(o4b_idx, nb);
            let mine = sc::sortcmp_4d0d50(
                &ra,
                &rb,
                &table,
                false,
                obj3_a,
                obj3_b,
                obj4_a,
                obj4_b,
                &mut |ka, kb| emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
            );
            assert_eq!(
                bin, mine,
                "4d0d50 col {col} dir {dir}: bin {bin} vs rust {mine}"
            );
            if mine != 0 {
                col_hit[col as usize] = true;
            }
        }
    }
    // every value-bearing column produced a nonzero (col 5 is a designed no-op).
    for c in [0, 1, 2, 3, 4, 6, 7] {
        assert!(col_hit[c], "4d0d50: column {c} never decided");
    }

    // pre-gate (0xb73124 == 0): exactly one +0x48 zero -> that side last, before the loop.
    {
        let table = noop_rest((5, 0)); // all no-op so only the pregate can decide
        write_table(&mut emu, &table);
        emu.write_u32(PREGATE, 0); // pregate ACTIVE
        for (av, bv, _) in [(0u32, 5u32, 1i32), (5, 0, -1), (0, 0, 0), (5, 5, 0)] {
            let a = mk(0, 99, 99, 0, av, 0.0);
            let b = mk(0, 99, 99, 0, bv, 0.0);
            for (rec, r) in [(rec_a, &a), (rec_b, &b)] {
                emu.write_u32(rec + 0x38, r.f38 as u32);
                emu.write_u32(rec + 0x40, r.f40 as u32);
                emu.write_u32(rec + 0x3c, r.f3c as u32);
                emu.write_u32(rec + 0x44, r.f44);
                emu.write_u32(rec + 0x48, r.f48);
                emu.write_u32(rec + 0x150, r.f150.to_bits());
                emu.write(rec + 8, b"Aaa\0");
                emu.write(rec + 0x4e, b"Aaa\0");
            }
            let bin = emu.call(0x4d0d50, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
            let ra = GuildMemberRec {
                name8: (rec_a + 8) as u32,
                name4e: (rec_a + 0x4e) as u32,
                ..a
            };
            let rb = GuildMemberRec {
                name8: (rec_b + 8) as u32,
                name4e: (rec_b + 0x4e) as u32,
                ..b
            };
            let mine = sc::sortcmp_4d0d50(
                &ra,
                &rb,
                &table,
                true,
                None,
                None,
                None,
                None,
                &mut |ka, kb| emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
            );
            assert_eq!(
                bin, mine,
                "4d0d50 pregate a={av} b={bv}: bin {bin} vs rust {mine}"
            );
        }
    }

    // col-6 both-zero float compare + a multi-column tie-then-decide chain.
    {
        emu.write_u32(PREGATE, 1);
        let table = noop_rest((6, 0));
        write_table(&mut emu, &table);
        for (fa, fb) in [(1.0f32, 2.0f32), (2.0, 1.0), (3.0, 3.0)] {
            let a = mk(0, 99, 99, 0, 0, fa);
            let b = mk(0, 99, 99, 0, 0, fb);
            for (rec, r) in [(rec_a, &a), (rec_b, &b)] {
                emu.write_u32(rec + 0x48, r.f48);
                emu.write_u32(rec + 0x150, r.f150.to_bits());
                emu.write(rec + 8, b"Aaa\0");
                emu.write(rec + 0x4e, b"Aaa\0");
            }
            let bin = emu.call(0x4d0d50, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
            let ra = GuildMemberRec {
                name8: (rec_a + 8) as u32,
                name4e: (rec_a + 0x4e) as u32,
                ..a
            };
            let rb = GuildMemberRec {
                name8: (rec_b + 8) as u32,
                name4e: (rec_b + 0x4e) as u32,
                ..b
            };
            let mine = sc::sortcmp_4d0d50(
                &ra,
                &rb,
                &table,
                false,
                None,
                None,
                None,
                None,
                &mut |ka, kb| emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
            );
            assert_eq!(
                bin, mine,
                "4d0d50 col6 float {fa} vs {fb}: bin {bin} vs rust {mine}"
            );
        }
        // multi-column: col0 ties (equal f44) then col2 names decide.
        let mut table = [(5u32, 0u32); 8];
        table[0] = (0, 0);
        table[1] = (2, 0);
        write_table(&mut emu, &table);
        let a = mk(0, 99, 99, 7, 1, 0.0);
        let b = mk(0, 99, 99, 7, 1, 0.0);
        for (rec, r) in [(rec_a, &a), (rec_b, &b)] {
            emu.write_u32(rec + 0x44, r.f44);
            emu.write_u32(rec + 0x48, r.f48);
            emu.write_u32(rec + 0x150, 0);
        }
        emu.write(rec_a + 8, b"Aaa\0");
        emu.write(rec_b + 8, b"Bbb\0");
        emu.write(rec_a + 0x4e, b"Aaa\0");
        emu.write(rec_b + 0x4e, b"Aaa\0");
        let bin = emu.call(0x4d0d50, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
        let ra = GuildMemberRec {
            name8: (rec_a + 8) as u32,
            name4e: (rec_a + 0x4e) as u32,
            ..a
        };
        let rb = GuildMemberRec {
            name8: (rec_b + 8) as u32,
            name4e: (rec_b + 0x4e) as u32,
            ..b
        };
        let mine = sc::sortcmp_4d0d50(
            &ra,
            &rb,
            &table,
            false,
            None,
            None,
            None,
            None,
            &mut |ka, kb| emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
        );
        assert_eq!(bin, mine, "4d0d50 multi-col: bin {bin} vs rust {mine}");
        assert!(mine != 0, "4d0d50 multi-col should decide via col2");
    }
}

// `0x4d3070`

#[test]
fn sortcmp_4d3070() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);
    // priority list (0xb73170 double-deref): ids [10, 20, 30].
    build_idlist(&mut emu, &[10, 20, 30], B73170, B731BC);
    // name-cache c0d92c: idx 1->obj_a, 2->obj_b, 3->null. (idx 0 reserved: f0==0 sentinel.)
    let obj_a = emu.alloc(0x20);
    let obj_b = emu.alloc(0x20);
    build_cache(&mut emu, &[0, obj_a, obj_b, 0], C0D92C, C0D930);
    let na = cstr(&mut emu, b"Aaa");
    let nb = cstr(&mut emu, b"Bbb");
    emu.write_u32(obj_a + 0xc, na as u32);
    emu.write_u32(obj_b + 0xc, nb as u32);
    let rec_a = emu.alloc(0x20);
    let rec_b = emu.alloc(0x20);
    let slot_a = emu.alloc(4);
    let slot_b = emu.alloc(4);
    emu.write_u32(slot_a, rec_a as u32);
    emu.write_u32(slot_b, rec_b as u32);

    // (f0, f4, f8, fc) per side
    type R = (i32, u32, u32, u32);
    let cases: &[(R, R, &str)] = &[
        ((1, 10, 0, 0), (1, 10, 0, 0), "both absent -> 0"),
        ((1, 10, 1, 0), (1, 10, 0, 0), "present a only -> -1"),
        ((1, 10, 0, 0), (1, 10, 1, 0), "present b only -> +1"),
        ((1, 10, 1, 0), (1, 20, 1, 0), "priority asc"),
        ((1, 30, 1, 0), (1, 10, 1, 0), "priority desc"),
        ((0, 10, 1, 0), (1, 10, 1, 0), "f0==0 a -> -1"),
        ((1, 10, 1, 0), (0, 10, 1, 0), "f0==0 b -> +1"),
        ((1, 10, 1, 5), (2, 10, 1, 0), "fc flag differ"),
        ((1, 10, 1, 0), (2, 10, 1, 0), "names differ"),
        ((1, 10, 1, 0), (1, 10, 1, 0), "names tie -> collate(=0)"),
        ((3, 10, 1, 0), (1, 10, 1, 0), "name-cache miss a"),
    ];
    for (a, b, lab) in cases {
        let wr = |emu: &mut Emu, rec: u64, r: &R| {
            emu.write_u32(rec, r.0 as u32);
            emu.write_u32(rec + 4, r.1);
            emu.write_u32(rec + 8, r.2);
            emu.write_u32(rec + 0xc, r.3);
        };
        wr(&mut emu, rec_a, a);
        wr(&mut emu, rec_b, b);
        let bin = emu.call(0x4d3070, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
        let resolve = |f0: i32| match f0 {
            1 => Some(na as u32),
            2 => Some(nb as u32),
            _ => None, // 0 (sentinel obj=0) or OOB
        };
        let ra = Rec4d3070 {
            f0: a.0,
            f4: a.1,
            f8: a.2,
            fc: a.3,
        };
        let rb = Rec4d3070 {
            f0: b.0,
            f4: b.1,
            f8: b.2,
            fc: b.3,
        };
        let mine = sc::sortcmp_4d3070(
            &ra,
            &rb,
            &[10, 20, 30],
            resolve(a.0),
            resolve(b.0),
            &mut |ka, kb| emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
        );
        assert_eq!(bin, mine, "4d3070 [{lab}]: bin {bin} vs rust {mine}");
    }
}

// 0x4d5dc0 — cache list B

#[test]
fn sortcmp_4d5dc0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);
    let obj_a = emu.alloc(0x60);
    let obj_b = emu.alloc(0x60);
    // c0dd50: idx 0->null, 1->obj_a, 2->obj_b, 3->null.
    build_cache(&mut emu, &[0, obj_a, obj_b, 0], C0DD50, C0DD54);
    let na = cstr(&mut emu, b"Aaa");
    let nb = cstr(&mut emu, b"Bbb");
    emu.write_u32(obj_a + 0x4c, na as u32);
    emu.write_u32(obj_b + 0x4c, nb as u32);
    let rec_a = emu.alloc(0x10);
    let rec_b = emu.alloc(0x10);
    let slot_a = emu.alloc(4);
    let slot_b = emu.alloc(4);
    emu.write_u32(slot_a, rec_a as u32);
    emu.write_u32(slot_b, rec_b as u32);

    // (f0, f4)
    type R = (i32, u32);
    let cases: &[(R, R, &str)] = &[
        ((0, 0), (0, 0), "both absent -> 0"),
        ((0, 1), (0, 0), "present a only -> -1"),
        ((0, 0), (0, 1), "present b only -> +1"),
        ((0, 1), (3, 1), "both unmapped: raw id desc (a<b ->+1)"),
        ((5, 1), (3, 1), "both unmapped: a>b -> -1"),
        ((1, 1), (2, 1), "both mapped: names differ"),
        ((1, 1), (2, 1), "both mapped: (same setup) "),
        ((1, 1), (3, 1), "a mapped, b unmapped -> -1"),
        ((3, 1), (1, 1), "a unmapped, b mapped -> +1"),
    ];
    for (a, b, lab) in cases {
        emu.write_u32(rec_a, a.0 as u32);
        emu.write_u32(rec_a + 4, a.1);
        emu.write_u32(rec_b, b.0 as u32);
        emu.write_u32(rec_b + 4, b.1);
        let bin = emu.call(0x4d5dc0, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
        let resolve = |f0: i32| match f0 {
            1 => Some(na as u32),
            2 => Some(nb as u32),
            _ => None,
        };
        let mine = sc::sortcmp_4d5dc0(
            &ReputationHeader { f0: a.0, f4: a.1 },
            &ReputationHeader { f0: b.0, f4: b.1 },
            resolve(a.0),
            resolve(b.0),
            &mut |ka, kb| emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
        );
        assert_eq!(bin, mine, "4d5dc0 [{lab}]: bin {bin} vs rust {mine}");
    }
}

// 0x4d5e70 — cache list B (grouped)

#[test]
fn sortcmp_4d5e70() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);
    // special mask: bit set => NOT special. Set bit 7 so category 7 is non-special.
    emu.write_u32(MASK_84A0A4, 1 << 7);
    let obj_a = emu.alloc(0x60);
    let obj_b = emu.alloc(0x60);
    build_cache(&mut emu, &[0, obj_a, obj_b, 0], C0DD50, C0DD54);
    let na = cstr(&mut emu, b"Aaa");
    let nb = cstr(&mut emu, b"Bbb");
    emu.write_u32(obj_a + 0x4c, na as u32);
    emu.write_u32(obj_b + 0x4c, nb as u32);
    let rec_a = emu.alloc(0x20);
    let rec_b = emu.alloc(0x20);
    let slot_a = emu.alloc(4);
    let slot_b = emu.alloc(4);
    emu.write_u32(slot_a, rec_a as u32);
    emu.write_u32(slot_b, rec_b as u32);
    let mask = 1u32 << 7;

    // (f4 idx, f8, fc cat)
    type R = (i32, u32, u32);
    let cases: &[(R, R, &str)] = &[
        // categories differ, both non-special (f8 != 0): desc by cat
        (
            (1, 1, 3),
            (2, 1, 9),
            "cat differ nonspecial (catB>catA->-1)",
        ),
        (
            (1, 1, 9),
            (2, 1, 3),
            "cat differ nonspecial (catB<catA->+1)",
        ),
        // A special (f8==0,cat<0x20,bit clear), B non-special (f8!=0)
        ((1, 0, 4), (2, 1, 9), "A special, B not -> A first"),
        ((1, 1, 9), (2, 0, 4), "B special, A not -> -1"),
        // both special, different cats -> A special: !specialB = 0
        ((1, 0, 4), (2, 0, 5), "both special -> 0"),
        // category 7 is non-special (mask bit set) even with f8==0
        ((1, 0, 7), (2, 1, 9), "cat7 masked nonspecial vs nonspecial"),
        // equal category -> f8 flag
        ((1, 1, 5), (2, 0, 5), "equal cat, f8 flag"),
        ((1, 0, 5), (2, 0, 5), "equal cat, both f8==0 -> names"),
        ((1, 0, 5), (1, 0, 5), "equal cat, name tie -> 0"),
        ((3, 0, 5), (2, 0, 5), "equal cat, name-cache miss a -> 0"),
    ];
    for (a, b, lab) in cases {
        let wr = |emu: &mut Emu, rec: u64, r: &R| {
            emu.write_u32(rec + 4, r.0 as u32);
            emu.write_u32(rec + 8, r.1);
            emu.write_u32(rec + 0xc, r.2);
        };
        wr(&mut emu, rec_a, a);
        wr(&mut emu, rec_b, b);
        let bin = emu.call(0x4d5e70, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
        let resolve = |idx: i32| match idx {
            1 => Some(na as u32),
            2 => Some(nb as u32),
            _ => None,
        };
        let mine = sc::sortcmp_4d5e70(
            &ReputationRow {
                f4: a.0,
                f8: a.1,
                fc: a.2,
            },
            &ReputationRow {
                f4: b.0,
                f8: b.1,
                fc: b.2,
            },
            mask,
            resolve(a.0),
            resolve(b.0),
            &mut |ka, kb| emu.call(COLL_A4C0, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
        );
        assert_eq!(bin, mine, "4d5e70 [{lab}]: bin {bin} vs rust {mine}");
    }
}

// `CompareTrainerRows` (`0x4d85c0`)

#[test]
fn sortcmp_4d85c0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);
    // order list (0xb739c0): ids [10, 20, 30].
    build_idlist(&mut emu, &[10, 20, 30], B739C0, B73A14);
    let obj_a = emu.alloc(0x220);
    let obj_b = emu.alloc(0x220);
    // player name-cache c0d788: idx 0->obj_a, 1->obj_b, 2->null.
    build_cache(&mut emu, &[obj_a, obj_b, 0], C0D788, C0D78C);
    let na = cstr(&mut emu, b"Aaa");
    let nb = cstr(&mut emu, b"Bbb");
    let n2a = cstr(&mut emu, b"Sec0");
    let n2b = cstr(&mut emu, b"Sec9");
    let rec_a = emu.alloc(0x40);
    let rec_b = emu.alloc(0x40);
    let slot_a = emu.alloc(4);
    let slot_b = emu.alloc(4);
    emu.write_u32(slot_a, rec_a as u32);
    emu.write_u32(slot_b, rec_b as u32);

    // (f34, f4, f0 idx, byte14, f1c, name1e0, name204)
    type R = (u32, u32, i32, u8, u32, u64, u64);
    let cases: &[(R, R, &str)] = &[
        (
            (0, 10, 0, 0, 0, na, na),
            (0, 10, 0, 0, 0, na, na),
            "both invalid -> 0",
        ),
        (
            (1, 10, 0, 0, 0, na, na),
            (0, 10, 0, 0, 0, na, na),
            "valid a only -> -1",
        ),
        (
            (0, 10, 0, 0, 0, na, na),
            (1, 10, 0, 0, 0, na, na),
            "valid b only -> +1",
        ),
        (
            (1, 10, 0, 0, 0, na, na),
            (1, 20, 0, 0, 0, nb, nb),
            "order asc",
        ),
        (
            (1, 30, 0, 0, 0, na, na),
            (1, 10, 0, 0, 0, nb, nb),
            "order desc",
        ),
        (
            (1, 10, -1, 0, 0, na, na),
            (1, 10, 0, 0, 0, nb, nb),
            "sign neg a -> -1",
        ),
        (
            (1, 10, 0, 0, 0, na, na),
            (1, 10, -1, 0, 0, nb, nb),
            "sign neg b -> +1",
        ),
        (
            (1, 10, 2, 0, 0, na, na),
            (1, 10, 0, 0, 0, nb, nb),
            "objA miss -> 0",
        ),
        (
            (1, 10, 0, 0, 0, na, na),
            (1, 10, 2, 0, 0, nb, nb),
            "objB miss -> 0",
        ),
        (
            (1, 10, 0, 3, 0, na, na),
            (1, 10, 1, 7, 0, nb, nb),
            "byte14 differ",
        ),
        (
            (1, 10, 0, 5, 9, na, na),
            (1, 10, 1, 5, 3, nb, nb),
            "f1c differ (byte tie)",
        ),
        (
            (1, 10, 0, 5, 7, na, n2a),
            (1, 10, 1, 5, 7, nb, n2b),
            "name1e0 differ",
        ),
        (
            (1, 10, 0, 5, 7, na, n2a),
            (1, 10, 1, 5, 7, na, n2b),
            "name1e0 tie -> name204",
        ),
        (
            (1, 10, 0, 5, 7, na, n2a),
            (1, 10, 1, 5, 7, na, n2a),
            "name204 tie -> 0",
        ),
    ];
    for (a, b, lab) in cases {
        let wr = |emu: &mut Emu, rec: u64, obj: u64, r: &R| {
            emu.write_u32(rec + 0x34, r.0);
            emu.write_u32(rec + 4, r.1);
            emu.write_u32(rec, r.2 as u32);
            emu.write_u32(rec + 0x14, r.3 as u32);
            emu.write_u32(rec + 0x1c, r.4);
            emu.write_u32(obj + 0x1e0, r.5 as u32);
            emu.write_u32(obj + 0x204, r.6 as u32);
        };
        wr(&mut emu, rec_a, obj_a, a);
        wr(&mut emu, rec_b, obj_b, b);
        let bin = emu.call(0x4d85c0, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
        let resolve = |idx: i32, n1: u64, n2: u64| {
            (idx == 0 || idx == 1).then_some(TrainerSpellRec {
                name1e0: n1 as u32,
                name204: n2 as u32,
            })
        };
        let ra = TrainerRow {
            f34: a.0,
            f4: a.1,
            f0: a.2,
            byte14: a.3,
            f1c: a.4,
        };
        let rb = TrainerRow {
            f34: b.0,
            f4: b.1,
            f0: b.2,
            byte14: b.3,
            f1c: b.4,
        };
        let mine = sc::sortcmp_4d85c0(
            &ra,
            &rb,
            &[10, 20, 30],
            resolve(a.2, a.5, a.6),
            resolve(b.2, b.5, b.6),
            &mut |ka, kb| emu.call(COLL_A480, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32,
        );
        assert_eq!(bin, mine, "4d85c0 [{lab}]: bin {bin} vs rust {mine}");
    }
}

// `CompareTradeskillTrainerRows` (`0x4d8760`)

#[test]
fn sortcmp_4d8760() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);
    let obj_a = emu.alloc(0x220);
    let obj_b = emu.alloc(0x220);
    build_cache(&mut emu, &[obj_a, obj_b, 0], C0D788, C0D78C);
    let na = cstr(&mut emu, b"Aaa");
    let nb = cstr(&mut emu, b"Bbb");
    emu.write_u32(obj_a + 0x1e0, na as u32);
    emu.write_u32(obj_b + 0x1e0, nb as u32);
    let rec_a = emu.alloc(0x40);
    let rec_b = emu.alloc(0x40);
    let slot_a = emu.alloc(4);
    let slot_b = emu.alloc(4);
    emu.write_u32(slot_a, rec_a as u32);
    emu.write_u32(slot_b, rec_b as u32);

    // (f34, f4, f0 idx, f1c)
    type R = (u32, u32, i32, u32);
    let cases: &[(R, R, &str)] = &[
        ((0, 10, 0, 0), (0, 10, 0, 0), "both invalid -> 0"),
        ((1, 10, 0, 0), (0, 10, 0, 0), "valid a only -> -1"),
        ((0, 10, 0, 0), (1, 10, 0, 0), "valid b only -> +1"),
        ((1, 5, 0, 0), (1, 9, 0, 0), "f4: recB<recA? (5 vs 9) -> -1"),
        ((1, 9, 0, 0), (1, 5, 0, 0), "f4: recB<recA -> +1"),
        ((1, 10, -1, 0), (1, 10, 0, 0), "sign neg a -> -1"),
        ((1, 10, 0, 0), (1, 10, -1, 0), "sign neg b -> +1"),
        ((1, 10, 2, 0), (1, 10, 0, 0), "objA miss -> 0"),
        (
            (1, 10, 0, 5),
            (1, 10, 1, 9),
            "f1c: recB<recA? (5 vs 9) -> -1",
        ),
        ((1, 10, 0, 9), (1, 10, 1, 5), "f1c desc -> +1"),
        ((1, 10, 0, 7), (1, 10, 1, 7), "f1c tie -> name1e0"),
        ((1, 10, 0, 7), (1, 10, 0, 7), "name tie -> 0"),
    ];
    for (a, b, lab) in cases {
        let wr = |emu: &mut Emu, rec: u64, r: &R| {
            emu.write_u32(rec + 0x34, r.0);
            emu.write_u32(rec + 4, r.1);
            emu.write_u32(rec, r.2 as u32);
            emu.write_u32(rec + 0x1c, r.3);
        };
        wr(&mut emu, rec_a, a);
        wr(&mut emu, rec_b, b);
        let bin = emu.call(0x4d8760, 0, 0, &[slot_a as u32, slot_b as u32]) as i32;
        // idx maps to a fixed object: 0 -> obj_a (na), 1 -> obj_b (nb); both share-idx ties.
        let resolve = |idx: i32| match idx {
            0 => Some(na as u32),
            1 => Some(nb as u32),
            _ => None,
        };
        let ra = TradeskillTrainerRow {
            f34: a.0,
            f4: a.1,
            f0: a.2,
            f1c: a.3,
        };
        let rb = TradeskillTrainerRow {
            f34: b.0,
            f4: b.1,
            f0: b.2,
            f1c: b.3,
        };
        let mine = sc::sortcmp_4d8760(&ra, &rb, resolve(a.2), resolve(b.2), &mut |ka, kb| {
            emu.call(COLL_A480, 0, 0, &[ka, kb, 0x7fff_ffff]) as i32
        });
        assert_eq!(bin, mine, "4d8760 [{lab}]: bin {bin} vs rust {mine}");
    }
}
