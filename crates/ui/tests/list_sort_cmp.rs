//! The list comparators that return a verdict or a pair to collate (`CompareMountTrainerRows`,
//! `CompareQuestHeaderIds`, `CompareQuestLogRows`, `CompareLfgEntries`, `CompareLfgQuests`,
//! `CompareCraftRows`, `CompareCraftRowsTrain`, and the comparators of `0x4fd380` and `0x4fd540`)
//! against the exe. Each comparator's own bytes run in the emulator over record pairs the test builds
//! in scratch memory, differing in one cascade key at a time, plus the presence and null cases and a
//! tie on the locale name; the sign of the binary's `eax` must match the transcription's verdict. For
//! a verdict that defers to the collator the test runs the same collator on the same strings, so the
//! cascade is what is proven: which strings, which collator, the surrounding `+1` and `-1`. The
//! collators `0x64a4c0` and `0x64a480` and the string stage `0x64a5a0` execute for real; the id to
//! object resolver `0x562a40` is replaced by an id-to-object routine, as the basic stub cannot return
//! a value per argument.

#![allow(clippy::too_many_arguments, clippy::field_reassign_with_default)]

mod fpkit;

use benilla_re_difftest::{Emu, Wow};
// The comparators are aliased so the tests named after their addresses do not shadow them.
use benilla_re_ui::list_sort_cmp::{
    sortcmp_4d8850 as cmp_4d8850, sortcmp_4de8f0 as cmp_4de8f0, sortcmp_4deac0 as cmp_4deac0,
    sortcmp_4e8b20 as cmp_4e8b20, sortcmp_4e8c80 as cmp_4e8c80, sortcmp_4f67a0 as cmp_4f67a0,
    sortcmp_4f6920 as cmp_4f6920, sortcmp_4fd380 as cmp_4fd380, sortcmp_4fd540 as cmp_4fd540,
};
use benilla_re_ui::list_sort_cmp::{
    Cmp, CraftGroup, CraftRow, Entry4fd540, Group4fd380, LfgEntryRow, MountTrainerRow, Rec4fd380,
    Rec4fd540, RecQuestLogRow,
};
use fpkit::next;

// globals the comparators read; populated per test
const LOCALE: u64 = 0x00c0_e080; // 0xc0e080 — active locale (set 0)
const A4C0: u64 = 0x0064_a4c0; // 0x64a4c0 — default collator
const A480: u64 = 0x0064_a480; // 0x64a480 — the collator of the player name cache
const A5A0: u64 = 0x0064_a5a0; // 0x64a5a0 — string-stage into buffer
const D788: u64 = 0x00c0_d788; // cache base pointer (0xc0d788)
const D78C: u64 = 0x00c0_d78c; // cache maximum id

/// An integer in `[0, n)` from the LCG's high bits (its low bits repeat quickly).
fn rib(s: &mut u32, n: u32) -> u32 {
    (next(s) >> 8) % n
}

/// A NUL-terminated C string in scratch; returns its VA.
fn cstr(emu: &mut Emu, s: &str) -> u32 {
    let va = emu.alloc(s.len() as u64 + 1);
    emu.write(va, s.as_bytes());
    emu.write(va + s.len() as u64, &[0u8]);
    va as u32
}

/// Run a collator on two string pointers; returns its raw `eax` (signed).
fn collate(emu: &mut Emu, collator: u64, a: u32, b: u32) -> i32 {
    emu.call(collator, 0, 0, &[a, b, 0x7fff_ffff]) as i32
}

/// Reduce a [`Cmp`] verdict to its sign, running the engine collator (+ optional desc tail) for the
/// `Collate` case so the comparison is end-to-end against the binary.
fn verdict_sign(emu: &mut Emu, collator: u64, mine: Cmp) -> i32 {
    match mine {
        Cmp::Lt => -1,
        Cmp::Eq => 0,
        Cmp::Gt => 1,
        Cmp::Collate { a, b, tie } => {
            let c = collate(emu, collator, a, b);
            if c != 0 {
                c.signum()
            } else if let Some((ta, tb)) = tie {
                if tb < ta {
                    1
                } else {
                    -1
                }
            } else {
                0
            }
        }
    }
}

fn check(emu: &mut Emu, collator: u64, bin: i32, mine: Cmp, ctx: &str) {
    let want = verdict_sign(emu, collator, mine);
    assert_eq!(
        bin.signum(),
        want,
        "{ctx}: binary eax={bin} (sign {}) vs rust {mine:?} (sign {want})",
        bin.signum()
    );
}

/// Install an id→object island over `0x562a40` (thiscall, `ret 0x14`; arg0 = id at `[esp+4]`):
/// `eax = obj_table[id]`. `obj_table` is a VA holding object pointers indexed by id (ids ≥ 0).
fn install_562a40(emu: &mut Emu, obj_table: u64) {
    let mut code = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax,[esp+4]
    code.extend_from_slice(&[0x8b, 0x04, 0x85]); // mov eax,[eax*4 + disp32]
    code.extend_from_slice(&(obj_table as u32).to_le_bytes());
    code.extend_from_slice(&[0xc2, 0x14, 0x00]); // ret 0x14
    emu.write(0x0056_2a40, &code);
}

/// A record of the cache at `0xc0d788`: `+0x1e0` the locale 0 name pointer, `+0x74` an int (the last
/// key of `0x4f6920`).
fn name_obj(emu: &mut Emu, name: u32, plus74: i32) -> u32 {
    let o = emu.alloc(0x200);
    emu.write_u32(o + 0x1e0, name);
    emu.write_u32(o + 0x74, plus74 as u32);
    o as u32
}

// `CompareMountTrainerRows` (`0x4d8850`)

#[test]
fn sortcmp_4d8850() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);

    // the cache: id 0->null, 1->alpha, 2->bravo, 3->alpha2 (name == id 1). max = 3.
    let s_alpha = cstr(&mut emu, "alpha");
    let s_bravo = cstr(&mut emu, "bravo");
    let o_alpha = name_obj(&mut emu, s_alpha, 0);
    let o_bravo = name_obj(&mut emu, s_bravo, 0);
    let o_alpha2 = name_obj(&mut emu, s_alpha, 0);
    let cache = emu.alloc(4 * 4);
    emu.write_u32(cache, 0);
    emu.write_u32(cache + 4, o_alpha);
    emu.write_u32(cache + 8, o_bravo);
    emu.write_u32(cache + 12, o_alpha2);
    emu.write_u32(D788, cache as u32);
    emu.write_u32(D78C, 3);
    let name_of = |emu: &Emu, id: i32| -> (u32, u32) {
        // (obj, name_str) per the cache resolution; obj 0 ⇒ absent.
        if !(0..=3).contains(&id) {
            return (0, 0);
        }
        let obj = emu.read_u32(cache + (id as u64) * 4);
        if obj == 0 {
            (0, 0)
        } else {
            (obj, emu.read_u32(obj as u64 + 0x1e0))
        }
    };

    // priority/order table B739C0 = array of ptrs-to-int (group ids); count B73A14.
    let g_ints = emu.alloc(4 * 4);
    for (i, g) in [10i32, 20, 30, 40].iter().enumerate() {
        emu.write_u32(g_ints + (i as u64) * 4, *g as u32);
    }
    let order_ptrs = emu.alloc(4 * 4);
    for i in 0..4u64 {
        emu.write_u32(order_ptrs + i * 4, (g_ints + i * 4) as u32);
    }
    emu.write_u32(0x00b7_39c0, order_ptrs as u32);
    emu.write_u32(0x00b7_3a14, 4);
    let order = [10i32, 20, 30, 40];

    // two records (size 0x38) behind handle slots (double-deref).
    let rec_a = emu.alloc(0x38);
    let rec_b = emu.alloc(0x38);
    let h_a = emu.alloc(4);
    let h_b = emu.alloc(4);
    emu.write_u32(h_a, rec_a as u32);
    emu.write_u32(h_b, rec_b as u32);

    struct C {
        va: i32,
        vb: i32,
        sa: i32,
        sb: i32,
        ga: i32,
        gb: i32,
        ba: u8,
        bb: u8,
        ida: i32,
        idb: i32,
    }
    let mut cases = vec![
        // presence
        C {
            va: 0,
            vb: 0,
            ..base()
        },
        C {
            va: 1,
            vb: 0,
            ..base()
        },
        C {
            va: 0,
            vb: 1,
            ..base()
        },
        // group index (asc)
        C {
            ga: 10,
            gb: 20,
            ..base()
        },
        C {
            ga: 30,
            gb: 10,
            ..base()
        },
        // sign neg-first
        C {
            sa: -1,
            sb: 5,
            ..base()
        },
        C {
            sa: 5,
            sb: -2,
            ..base()
        },
        C {
            sa: -1,
            sb: -1,
            ..base()
        },
        // byte30 desc
        C {
            ba: 3,
            bb: 7,
            ..base()
        },
        C {
            ba: 9,
            bb: 2,
            ..base()
        },
        // name: sign_id IS the cache id — distinct, tie (alpha==alpha2), null-object.
        C {
            sa: 1,
            sb: 2,
            ..base()
        },
        C {
            sa: 1,
            sb: 3,
            ..base()
        }, // alpha vs alpha2 -> collator tie
        C {
            sa: 0,
            sb: 2,
            ..base()
        }, // a null obj -> 0
    ];
    fn base() -> C {
        C {
            va: 1,
            vb: 1,
            sa: 5,
            sb: 5,
            ga: 10,
            gb: 10,
            ba: 0,
            bb: 0,
            ida: 1,
            idb: 1,
        }
    }
    let mut s = 0x4d88_5000u32;
    for _ in 0..1500 {
        cases.push(C {
            va: (rib(&mut s, 2)) as i32,
            vb: (rib(&mut s, 2)) as i32,
            sa: rib(&mut s, 8) as i32 - 2,
            sb: rib(&mut s, 8) as i32 - 2,
            ga: [10, 20, 30, 40, 99][rib(&mut s, 5) as usize],
            gb: [10, 20, 30, 40, 99][rib(&mut s, 5) as usize],
            ba: rib(&mut s, 6) as u8,
            bb: rib(&mut s, 6) as u8,
            ida: rib(&mut s, 4) as i32,
            idb: rib(&mut s, 4) as i32,
        });
    }

    let (mut presence, mut grp, mut sign, mut byte, mut collated, mut tie_hit) = (0, 0, 0, 0, 0, 0);
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(rec_a, c.sa as u32);
        emu.write_u32(rec_a + 4, c.ga as u32);
        emu.write(rec_a + 0x30, &[c.ba]);
        emu.write_u32(rec_a + 0x34, c.va as u32);
        emu.write_u32(rec_b, c.sb as u32);
        emu.write_u32(rec_b + 4, c.gb as u32);
        emu.write(rec_b + 0x30, &[c.bb]);
        emu.write_u32(rec_b + 0x34, c.vb as u32);

        let bin = emu.call(0x004d_8850, 0, 0, &[h_a as u32, h_b as u32]) as i32;
        let (oa, na) = name_of(&emu, c.sa);
        let (ob, nb) = name_of(&emu, c.sb);
        // sign_id is the cache id too; use sa/sb. For neg ids name_of returns (0,0).
        let _ = (c.ida, c.idb);
        let ra = MountTrainerRow {
            valid: c.va,
            sign_id: c.sa,
            group: c.ga,
            byte30: c.ba,
            name_obj: oa,
            name_str: na,
        };
        let rb = MountTrainerRow {
            valid: c.vb,
            sign_id: c.sb,
            group: c.gb,
            byte30: c.bb,
            name_obj: ob,
            name_str: nb,
        };
        let mine = cmp_4d8850(&ra, &rb, &order);
        if let Cmp::Collate { a, b, .. } = mine {
            collated += 1;
            if collate(&mut emu, A480, a, b) == 0 {
                tie_hit += 1;
            }
        }
        if (c.va == 0) != (c.vb == 0) || (c.va == 0 && c.vb == 0) {
            presence += 1;
        }
        if c.va != 0 && c.vb != 0 {
            grp += (c.ga != c.gb) as i32;
            sign += (c.sa < 0 || c.sb < 0) as i32;
            byte += (c.ba != c.bb) as i32;
        }
        check(&mut emu, A480, bin, mine, &format!("4d8850 case {i}"));
    }
    eprintln!(
        "4d8850: {} cases ok (presence={presence} grp={grp} sign={sign} byte={byte} collate-eligible={collated} ties={tie_hit})",
        cases.len()
    );
    assert!(presence > 0 && grp > 0 && sign > 0 && byte > 0 && collated > 0 && tie_hit > 0);
}

// 0x4de8f0 — the QUEST-LOG HEADER-ID comparator

#[test]
fn sortcmp_4de8f0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);

    // positive-id cache E048 (obj +0x2c name), max E04C; negative-id cache D9B8 (obj +0x4), max D9BC.
    let s_alpha = cstr(&mut emu, "alpha");
    let s_bravo = cstr(&mut emu, "bravo");
    let s_charlie = cstr(&mut emu, "charlie");
    let pos_o1 = emu.alloc(0x40);
    emu.write_u32(pos_o1 + 0x2c, s_alpha);
    let pos_o2 = emu.alloc(0x40);
    emu.write_u32(pos_o2 + 0x2c, s_bravo);
    let pos_tab = emu.alloc(4 * 4);
    emu.write_u32(pos_tab, 0); // id 0 -> null (also id 0 short-circuits in the fn)
    emu.write_u32(pos_tab + 4, pos_o1 as u32);
    emu.write_u32(pos_tab + 8, pos_o2 as u32);
    emu.write_u32(pos_tab + 12, 0); // id 3 -> null object
    emu.write_u32(0x00c0_e048, pos_tab as u32);
    emu.write_u32(0x00c0_e04c, 3);
    let neg_o1 = emu.alloc(0x40);
    emu.write_u32(neg_o1 + 4, s_charlie);
    let neg_tab = emu.alloc(4 * 4);
    emu.write_u32(neg_tab, 0);
    emu.write_u32(neg_tab + 4, neg_o1 as u32); // index 1 (id -1)
    emu.write_u32(0x00c0_d9b8, neg_tab as u32);
    emu.write_u32(0x00c0_d9bc, 2); // 0xc0d9bc

    // resolve an id to its source name string (mirrors the fn's resolution).
    let resolve = |emu: &Emu, id: i32| -> u32 {
        if id < 0 {
            let k = -id;
            if k >= 0 && (k < 2 || -2 == id) {
                let obj = emu.read_u32(neg_tab + (k as u64) * 4);
                if obj != 0 {
                    return emu.read_u32(obj as u64 + 4);
                }
            }
            0
        } else if id <= 3 {
            let obj = emu.read_u32(pos_tab + (id as u64) * 4);
            if obj != 0 {
                emu.read_u32(obj as u64 + 0x2c)
            } else {
                0
            }
        } else {
            0
        }
    };

    // staged-buffer expected: init two 0x100 bufs (byte0 = 0x882748, rest 0), stage, collate.
    let init_byte = emu.read(0x0088_2748, 1)[0];
    let buf_a = emu.alloc(0x100);
    let buf_b = emu.alloc(0x100);
    let rec_a = emu.alloc(4);
    let rec_b = emu.alloc(4);

    let cases: Vec<(i32, i32)> = {
        let mut v = vec![
            (1, 1),   // equal id -> 0
            (0, 2),   // a id 0 -> -1
            (2, 0),   // b id 0 -> +1
            (1, 2),   // alpha vs bravo
            (2, 1),   // bravo vs alpha
            (1, -1),  // alpha vs charlie (neg source)
            (-1, 1),  // charlie vs alpha
            (1, 3),   // alpha vs null-object (empty staged)
            (3, 2),   // null-object vs bravo
            (-2, -1), // unresolved neg vs charlie
        ];
        let mut s = 0x4de8_f000u32;
        for _ in 0..400 {
            v.push((rib(&mut s, 6) as i32 - 2, rib(&mut s, 6) as i32 - 2));
        }
        v
    };

    let (mut eq, mut zero, mut collated) = (0, 0, 0);
    for (i, &(ida, idb)) in cases.iter().enumerate() {
        emu.write_u32(rec_a, ida as u32);
        emu.write_u32(rec_b, idb as u32);
        let bin = emu.call(0x004d_e8f0, 0, 0, &[rec_a as u32, rec_b as u32]) as i32;
        let na = resolve(&emu, ida);
        let nb = resolve(&emu, idb);
        let mine = cmp_4de8f0(ida, idb, na, nb);
        let want = match mine {
            Cmp::Eq => {
                eq += 1;
                0
            }
            Cmp::Lt => {
                zero += 1;
                -1
            }
            Cmp::Gt => {
                zero += 1;
                1
            }
            Cmp::Collate { a, b, .. } => {
                collated += 1;
                // stage both into re-initialised buffers, then collate.
                let mut buf = vec![0u8; 0x100];
                buf[0] = init_byte;
                emu.write(buf_a, &buf);
                emu.write(buf_b, &buf);
                if a != 0 {
                    emu.call(A5A0, 0, 0, &[buf_a as u32, a, 0x100]);
                }
                if b != 0 {
                    emu.call(A5A0, 0, 0, &[buf_b as u32, b, 0x100]);
                }
                collate(&mut emu, A4C0, buf_a as u32, buf_b as u32).signum()
            }
        };
        assert_eq!(
            bin.signum(),
            want,
            "4de8f0 case {i} ids=({ida},{idb}): binary {bin} vs {want} ({mine:?})"
        );
    }
    eprintln!(
        "4de8f0: {} cases ok (eq={eq} zero={zero} collated={collated})",
        cases.len()
    );
    assert!(eq > 0 && zero > 0 && collated > 0);
}

// 0x4deac0 — the QUEST-LOG DISPLAY-ROW comparator

#[test]
fn sortcmp_4deac0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);

    // class table 0xbb7040 (20 entries) and special mask 0xbb748c.
    let classes: [i32; 20] = [
        100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
        118, 119,
    ];
    for (i, c) in classes.iter().enumerate() {
        emu.write_u32(0x00bb_7040 + (i as u64) * 4, *c as u32);
    }
    // special when index bit CLEAR: mask sets bits for NON-special indices. Make idx 0,1 special
    // (bits clear), idx 2,3 non-special (bits set).
    let special_mask: u32 = 0b1100;
    emu.write_u32(0x00bb_748c, special_mask);

    // spell objects (resolved by the 562a40 island): +0xc class, +8 rank, +0x9c inline name.
    let mk_spell = |emu: &mut Emu, class: i32, rank: i32, name: &str| -> u32 {
        let o = emu.alloc(0x9c + name.len() as u64 + 1);
        emu.write_u32(o + 0xc, class as u32);
        emu.write_u32(o + 8, rank as u32);
        emu.write(o + 0x9c, name.as_bytes());
        emu.write(o + 0x9c + name.len() as u64, &[0u8]);
        o as u32
    };
    // id 0 -> null; 1 -> class100 (idx0, SPECIAL) r5 "fireball"; the rest are class102 (idx2,
    // NON-special) so the rank/name tie-breaks have ≥2 members of one non-special class:
    // 2 r3 "frostbolt", 3 r7 "icebolt" (rank key), 4 r3 "frostbolt" (== id2 name+rank => collate tie),
    // 5 -> class103 (idx3, non-special) r1 "arcane".
    let sp = [
        0,
        mk_spell(&mut emu, 100, 5, "fireball"),
        mk_spell(&mut emu, 102, 3, "frostbolt"),
        mk_spell(&mut emu, 102, 7, "icebolt"),
        mk_spell(&mut emu, 102, 3, "frostbolt"),
        mk_spell(&mut emu, 103, 1, "arcane"),
    ];
    let obj_table = emu.alloc(sp.len() as u64 * 4);
    for (i, o) in sp.iter().enumerate() {
        emu.write_u32(obj_table + (i as u64) * 4, *o);
    }
    install_562a40(&mut emu, obj_table);

    // records (read directly): +0 id6, +8 field8.
    let rec_a = emu.alloc(0x10);
    let rec_b = emu.alloc(0x10);

    // build a Rec mirror from (id6, field8) the way the fn sees it.
    let mk_rec = |emu: &Emu, id6: i32, field8: i32| -> RecQuestLogRow {
        let (obj, class_id, rank, name_str) = if field8 == 0 && (0..sp.len() as i32).contains(&id6)
        {
            let o = emu.read_u32(obj_table + (id6 as u64) * 4);
            if o != 0 {
                (
                    o,
                    emu.read_u32(o as u64 + 0xc) as i32,
                    emu.read_u32(o as u64 + 8) as i32,
                    o + 0x9c,
                )
            } else {
                (0, 0, 0, 0)
            }
        } else {
            (0, 0, 0, 0)
        };
        RecQuestLogRow {
            id6,
            field8,
            obj,
            class_id,
            rank,
            name_str,
        }
    };

    struct C {
        ia: i32,
        fa: i32,
        ib: i32,
        fb: i32,
    }
    let mut cases = vec![
        // both special (class100=idx0 special, class101=idx1 special): both members -> tie 0
        C {
            ia: 1,
            fa: 0,
            ib: 1,
            fb: 0,
        },
        // a special (class100 idx0) vs b non-special (class102 idx2)
        C {
            ia: 1,
            fa: 0,
            ib: 2,
            fb: 0,
        },
        C {
            ia: 2,
            fa: 0,
            ib: 1,
            fb: 0,
        },
        // neither special: header (class102) vs member(class102) — header first
        C {
            ia: 102,
            fa: 7,
            ib: 2,
            fb: 0,
        },
        C {
            ia: 2,
            fa: 0,
            ib: 102,
            fb: 7,
        },
        // neither special, class index asc: class102(idx2) vs class103(idx3)
        C {
            ia: 102,
            fa: 1,
            ib: 103,
            fb: 1,
        },
        C {
            ia: 103,
            fa: 1,
            ib: 102,
            fb: 1,
        },
        // rank desc within same non-special class (id2/id3 both class102, ranks 3 vs 7)
        C {
            ia: 2,
            fa: 0,
            ib: 3,
            fb: 0,
        },
        C {
            ia: 3,
            fa: 0,
            ib: 2,
            fb: 0,
        },
        // name collate within same class+rank (id2/id4 both class102 r3, names "frostbolt")
        C {
            ia: 2,
            fa: 0,
            ib: 4,
            fb: 0,
        },
        // member resolution failure: field8==0 id->null
        C {
            ia: 0,
            fa: 0,
            ib: 2,
            fb: 0,
        },
        C {
            ia: 2,
            fa: 0,
            ib: 0,
            fb: 0,
        },
    ];
    // rank/name need two members of the SAME non-special class. Make objects 2 & a clone via id.
    // id2=class102 r3 "frostbolt"; add comparisons against itself's name tie using id with same class.
    // Use header/member combos in fuzz below for rank/name (class102 members).
    let mut s = 0x4dea_c000u32;
    for _ in 0..2000 {
        // ids: members 1..5 (field8=0) or headers (field8 != 0) with direct class id.
        let member = rib(&mut s, 2) == 0;
        let (ia, fa) = if member {
            (rib(&mut s, 6) as i32, 0)
        } else {
            (100 + rib(&mut s, 6) as i32, 1 + rib(&mut s, 3) as i32)
        };
        let member_b = rib(&mut s, 2) == 0;
        let (ib, fb) = if member_b {
            (rib(&mut s, 6) as i32, 0)
        } else {
            (100 + rib(&mut s, 6) as i32, 1 + rib(&mut s, 3) as i32)
        };
        cases.push(C { ia, fa, ib, fb });
    }

    let (
        mut both_special,
        mut one_special,
        mut idx_key,
        mut header,
        mut rank_key,
        mut collated,
        mut tie,
    ) = (0, 0, 0, 0, 0, 0, 0);
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(rec_a, c.ia as u32);
        emu.write_u32(rec_a + 8, c.fa as u32);
        emu.write_u32(rec_b, c.ib as u32);
        emu.write_u32(rec_b + 8, c.fb as u32);
        let bin = emu.call(0x004d_eac0, 0, 0, &[rec_a as u32, rec_b as u32]) as i32;
        let ra = mk_rec(&emu, c.ia, c.fa);
        let rb = mk_rec(&emu, c.ib, c.fb);
        let mine = cmp_4deac0(&ra, &rb, &classes, special_mask);
        // coverage bookkeeping
        let class_a = if ra.field8 == 0 { ra.class_id } else { ra.id6 };
        let class_b = if rb.field8 == 0 { rb.class_id } else { rb.id6 };
        let idxa = classes
            .iter()
            .take(0x14)
            .position(|&x| x == class_a)
            .unwrap_or(0);
        let idxb = classes
            .iter()
            .take(0x14)
            .position(|&x| x == class_b)
            .unwrap_or(0);
        let spa = ra.field8 == 0 && (ra.obj != 0) && (1u32 << (idxa & 0x1f) & special_mask) == 0;
        let spb = rb.field8 == 0 && (rb.obj != 0) && (1u32 << (idxb & 0x1f) & special_mask) == 0;
        let resolvable = (ra.field8 != 0 || ra.obj != 0) && (rb.field8 != 0 || rb.obj != 0);
        if resolvable {
            if spa && spb {
                both_special += 1;
            } else if spa != spb {
                one_special += 1;
            } else if idxa != idxb {
                idx_key += 1;
            } else if ra.field8 != 0 || rb.field8 != 0 {
                header += 1;
            } else if ra.rank != rb.rank {
                rank_key += 1;
            } else {
                collated += 1;
                if ra.name_str != 0 && rb.name_str != 0 {
                    let c0 = collate(&mut emu, A4C0, ra.name_str, rb.name_str);
                    if c0 == 0 {
                        tie += 1;
                    }
                }
            }
        }
        check(&mut emu, A4C0, bin, mine, &format!("4deac0 case {i}"));
    }
    eprintln!(
        "4deac0: {} cases ok (both_special={both_special} one_special={one_special} idx={idx_key} header={header} rank={rank_key} collate={collated} tie={tie})",
        cases.len()
    );
    assert!(
        both_special > 0
            && one_special > 0
            && idx_key > 0
            && header > 0
            && rank_key > 0
            && collated > 0
    );
}

// `CompareLfgEntries` (`0x4e8b20`)

#[test]
fn sortcmp_4e8b20() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);

    // The LfgDungeons id map 0xc0db34 (base), its maximum id 0xc0db38. Rows: +0x28 the least level,
    // +0x2c the greatest, +0x4 the locale name.
    let s_alpha = cstr(&mut emu, "alpha");
    let s_bravo = cstr(&mut emu, "bravo");
    let mk = |emu: &mut Emu, lo: i32, hi: i32, name: u32| -> u32 {
        let o = emu.alloc(0x30);
        emu.write_u32(o + 0x28, lo as u32);
        emu.write_u32(o + 0x2c, hi as u32);
        emu.write_u32(o + 4, name);
        o as u32
    };
    // id 0 -> null, 1 -> [10,20] alpha, 2 -> [30,40] bravo, 3 -> [10,20] alpha (tie w/ 1).
    let objs = [
        0,
        mk(&mut emu, 10, 20, s_alpha),
        mk(&mut emu, 30, 40, s_bravo),
        mk(&mut emu, 10, 20, s_alpha),
    ];
    let cache = emu.alloc(objs.len() as u64 * 4);
    for (i, o) in objs.iter().enumerate() {
        emu.write_u32(cache + (i as u64) * 4, *o);
    }
    emu.write_u32(0x00c0_db34, cache as u32);
    emu.write_u32(0x00c0_db38, 3); // max id

    let rec_a = emu.alloc(4);
    let rec_b = emu.alloc(4);
    let resolve = |emu: &Emu, id: i32| -> LfgEntryRow {
        let obj = if (0..=3).contains(&id) {
            emu.read_u32(cache + (id as u64) * 4)
        } else {
            0
        };
        if obj == 0 {
            LfgEntryRow {
                obj: 0,
                lvl_lo: 0,
                lvl_hi: 0,
                name_str: 0,
            }
        } else {
            LfgEntryRow {
                obj,
                lvl_lo: emu.read_u32(obj as u64 + 0x28) as i32,
                lvl_hi: emu.read_u32(obj as u64 + 0x2c) as i32,
                name_str: emu.read_u32(obj as u64 + 4),
            }
        }
    };

    struct C {
        ida: i32,
        idb: i32,
        level: i32,
    }
    let mut cases = vec![
        C {
            ida: 0,
            idb: 0,
            level: 15,
        }, // both absent
        C {
            ida: 1,
            idb: 0,
            level: 15,
        }, // a present -> -1
        C {
            ida: 0,
            idb: 2,
            level: 15,
        }, // b present -> +1
        C {
            ida: 1,
            idb: 2,
            level: 15,
        }, // a in [10,20], b out -> in-range first
        C {
            ida: 2,
            idb: 1,
            level: 35,
        }, // a in [30,40], b out
        C {
            ida: 1,
            idb: 2,
            level: 35,
        }, // a out, b in
        C {
            ida: 1,
            idb: 3,
            level: 15,
        }, // both in range, name tie (alpha==alpha)
        C {
            ida: 1,
            idb: 2,
            level: 99,
        }, // both out of range -> name decides
    ];
    let mut s = 0x4e8b_2000u32;
    for _ in 0..1200 {
        cases.push(C {
            ida: rib(&mut s, 4) as i32,
            idb: rib(&mut s, 4) as i32,
            level: [5, 15, 25, 35, 45][rib(&mut s, 5) as usize],
        });
    }

    let (mut presence, mut range, mut collated, mut tie) = (0, 0, 0, 0);
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(rec_a, c.ida as u32);
        emu.write_u32(rec_b, c.idb as u32);
        emu.write_u32(0x00bc_7624, c.level as u32); // 0xbc7624
        let bin = emu.call(0x004e_8b20, 0, 0, &[rec_a as u32, rec_b as u32]) as i32;
        let ra = resolve(&emu, c.ida);
        let rb = resolve(&emu, c.idb);
        let mine = cmp_4e8b20(&ra, &rb, c.level);
        if (ra.obj == 0) != (rb.obj == 0) || (ra.obj == 0 && rb.obj == 0) {
            presence += 1;
        }
        if ra.obj != 0 && rb.obj != 0 {
            let ina = !(c.level < ra.lvl_lo || ra.lvl_hi < c.level);
            let inb = !(c.level < rb.lvl_lo || rb.lvl_hi < c.level);
            if ina != inb {
                range += 1;
            } else {
                collated += 1;
                if ra.name_str == rb.name_str {
                    tie += 1;
                }
            }
        }
        check(&mut emu, A4C0, bin, mine, &format!("4e8b20 case {i}"));
    }
    eprintln!(
        "4e8b20: {} cases ok (presence={presence} range={range} collate={collated} tie={tie})",
        cases.len()
    );
    assert!(presence > 0 && range > 0 && collated > 0 && tie > 0);
}

// `CompareLfgQuests` (`0x4e8c80`)

#[test]
fn sortcmp_4e8c80() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);

    // spell objects via 562a40 island: +0x9c inline name.
    let mk = |emu: &mut Emu, name: &str| -> u32 {
        let o = emu.alloc(0x9c + name.len() as u64 + 1);
        emu.write(o + 0x9c, name.as_bytes());
        emu.write(o + 0x9c + name.len() as u64, &[0u8]);
        o as u32
    };
    let sp = [
        0,
        mk(&mut emu, "alpha"),
        mk(&mut emu, "bravo"),
        mk(&mut emu, "alpha"),
    ];
    let obj_table = emu.alloc(sp.len() as u64 * 4);
    for (i, o) in sp.iter().enumerate() {
        emu.write_u32(obj_table + (i as u64) * 4, *o);
    }
    install_562a40(&mut emu, obj_table);

    let rec_a = emu.alloc(4);
    let rec_b = emu.alloc(4);
    let resolve = |emu: &Emu, id: i32| -> (u32, u32) {
        if (0..sp.len() as i32).contains(&id) {
            let o = emu.read_u32(obj_table + (id as u64) * 4);
            if o != 0 {
                return (o, o + 0x9c);
            }
        }
        (0, 0)
    };

    let cases: Vec<(i32, i32)> = {
        let mut v = vec![(0, 0), (1, 0), (0, 2), (1, 2), (2, 1), (1, 3)];
        let mut s = 0x4e8c_8000u32;
        for _ in 0..600 {
            v.push((rib(&mut s, 4) as i32, rib(&mut s, 4) as i32));
        }
        v
    };
    let (mut presence, mut collated, mut tie) = (0, 0, 0);
    for (i, &(ia, ib)) in cases.iter().enumerate() {
        emu.write_u32(rec_a, ia as u32);
        emu.write_u32(rec_b, ib as u32);
        let bin = emu.call(0x004e_8c80, 0, 0, &[rec_a as u32, rec_b as u32]) as i32;
        let (oa, na) = resolve(&emu, ia);
        let (ob, nb) = resolve(&emu, ib);
        let mine = cmp_4e8c80(oa, na, ob, nb);
        if (oa == 0) != (ob == 0) || (oa == 0 && ob == 0) {
            presence += 1;
        }
        if oa != 0 && ob != 0 {
            collated += 1;
            if na != 0 && nb != 0 && collate(&mut emu, A4C0, na, nb) == 0 {
                tie += 1;
            }
        }
        check(&mut emu, A4C0, bin, mine, &format!("4e8c80 case {i}"));
    }
    eprintln!(
        "4e8c80: {} cases ok (presence={presence} collate={collated} tie={tie})",
        cases.len()
    );
    assert!(presence > 0 && collated > 0 && tie > 0);
}

// `CompareCraftRows` (`0x4f67a0`) and `CompareCraftRowsTrain` (`0x4f6920`)

/// The shared fixture of the two craft comparators: the cache at `0xc0d788` and the group table at
/// `0xbdcf88`, with a mirror of the group entries.
struct CraftFix {
    cache: u64,
    rec_a: u64,
    rec_b: u64,
    h_a: u64,
    h_b: u64,
    groups: Vec<CraftGroup>,
}

fn craft_group_setup(emu: &mut Emu) -> CraftFix {
    emu.write_u32(LOCALE, 0);
    let s_alpha = cstr(emu, "alpha");
    let s_bravo = cstr(emu, "bravo");
    // id 0 null, 1 alpha(+74=5), 2 bravo(+74=9), 3 alpha2(+74=2, name==1 for tie)
    let o1 = name_obj(emu, s_alpha, 5);
    let o2 = name_obj(emu, s_bravo, 9);
    let o3 = name_obj(emu, s_alpha, 2);
    let cache = emu.alloc(4 * 4);
    emu.write_u32(cache, 0);
    emu.write_u32(cache + 4, o1);
    emu.write_u32(cache + 8, o2);
    emu.write_u32(cache + 12, o3);
    emu.write_u32(D788, cache as u32);
    emu.write_u32(D78C, 3);
    // The group table 0xbdcf88 is an array of pointers to entries; entry[0] is the key and
    // entry[2] the flag. The count is 0xbdcfc4.
    let entries = [(7i32, 0i32), (8, 0), (9, 1)]; // keys 7, 8, 9; key 9 has entry2 != 0
    let groups_base = emu.alloc(entries.len() as u64 * 4);
    let mut groups = Vec::new();
    for (i, &(k, f)) in entries.iter().enumerate() {
        let e = emu.alloc(0x10);
        emu.write_u32(e, k as u32);
        emu.write_u32(e + 8, f as u32);
        emu.write_u32(groups_base + (i as u64) * 4, e as u32);
        groups.push(CraftGroup { key: k, entry2: f });
    }
    emu.write_u32(0x00bd_cf88, groups_base as u32);
    emu.write_u32(0x00bd_cfc4, entries.len() as u32);
    let rec_a = emu.alloc(0x10);
    let rec_b = emu.alloc(0x10);
    let h_a = emu.alloc(4);
    let h_b = emu.alloc(4);
    emu.write_u32(h_a, rec_a as u32);
    emu.write_u32(h_b, rec_b as u32);
    CraftFix {
        cache,
        rec_a,
        rec_b,
        h_a,
        h_b,
        groups,
    }
}

fn craft_row(emu: &Emu, cache: u64, sign: i32, gk: i32, kc: i32) -> CraftRow {
    let obj = if (0..=3).contains(&sign) {
        emu.read_u32(cache + (sign as u64) * 4)
    } else {
        0
    };
    let (name_str, plus74) = if obj != 0 {
        (
            emu.read_u32(obj as u64 + 0x1e0),
            emu.read_u32(obj as u64 + 0x74) as i32,
        )
    } else {
        (0, 0)
    };
    CraftRow {
        sign_id: sign,
        group_key: gk,
        key_c: kc,
        name_obj: obj,
        name_str,
        plus74,
    }
}

struct FC {
    sa: i32,
    sb: i32,
    gka: i32,
    gkb: i32,
    kca: i32,
    kcb: i32,
    groups_on: bool,
}

fn craft_cases() -> Vec<FC> {
    let mut v = vec![
        // the group prefix decides (both in groups, different index)
        FC {
            sa: 1,
            sb: 2,
            gka: 7,
            gkb: 8,
            kca: 0,
            kcb: 0,
            groups_on: true,
        },
        FC {
            sa: 1,
            sb: 2,
            gka: 8,
            gkb: 7,
            kca: 0,
            kcb: 0,
            groups_on: true,
        },
        // one grouped, one not (group 9 is offline-flag, treated as not-grouped)
        FC {
            sa: 1,
            sb: 2,
            gka: 7,
            gkb: 99,
            kca: 0,
            kcb: 0,
            groups_on: true,
        },
        FC {
            sa: 1,
            sb: 2,
            gka: 99,
            gkb: 7,
            kca: 0,
            kcb: 0,
            groups_on: true,
        },
        // neither grouped, both with offline-flag group 9 -> 0
        FC {
            sa: 1,
            sb: 2,
            gka: 9,
            gkb: 9,
            kca: 0,
            kcb: 0,
            groups_on: true,
        },
        // both grouped same index -> name path
        FC {
            sa: 1,
            sb: 2,
            gka: 7,
            gkb: 7,
            kca: 0,
            kcb: 0,
            groups_on: true,
        },
        // the name path (groups off): -1 sentinel
        FC {
            sa: -1,
            sb: 2,
            gka: 0,
            gkb: 0,
            kca: 0,
            kcb: 0,
            groups_on: false,
        },
        FC {
            sa: 1,
            sb: -1,
            gka: 0,
            gkb: 0,
            kca: 0,
            kcb: 0,
            groups_on: false,
        },
        // key_c desc
        FC {
            sa: 1,
            sb: 2,
            gka: 0,
            gkb: 0,
            kca: 3,
            kcb: 7,
            groups_on: false,
        },
        FC {
            sa: 1,
            sb: 2,
            gka: 0,
            gkb: 0,
            kca: 9,
            kcb: 1,
            groups_on: false,
        },
        // name distinct & tie (alpha vs bravo / alpha vs alpha2)
        FC {
            sa: 1,
            sb: 2,
            gka: 0,
            gkb: 0,
            kca: 0,
            kcb: 0,
            groups_on: false,
        },
        FC {
            sa: 1,
            sb: 3,
            gka: 0,
            gkb: 0,
            kca: 0,
            kcb: 0,
            groups_on: false,
        },
        // null object in name path -> 0
        FC {
            sa: 0,
            sb: 2,
            gka: 0,
            gkb: 0,
            kca: 0,
            kcb: 0,
            groups_on: false,
        },
    ];
    let mut s = 0x4f60_0000u32;
    for _ in 0..1500 {
        let on = rib(&mut s, 2) == 0;
        v.push(FC {
            sa: rib(&mut s, 6) as i32 - 2,
            sb: rib(&mut s, 6) as i32 - 2,
            gka: [7, 8, 9, 99][rib(&mut s, 4) as usize],
            gkb: [7, 8, 9, 99][rib(&mut s, 4) as usize],
            kca: rib(&mut s, 4) as i32,
            kcb: rib(&mut s, 4) as i32,
            groups_on: on,
        });
    }
    v
}

#[test]
fn sortcmp_4f67a0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let fx = craft_group_setup(&mut emu);
    let cases = craft_cases();
    let (mut group, mut sentinel, mut kc, mut collated, mut tie) = (0, 0, 0, 0, 0);
    for (i, c) in cases.iter().enumerate() {
        // the group count, toggling the prefix on or off
        emu.write_u32(
            0x00bd_cfc4,
            if c.groups_on {
                fx.groups.len() as u32
            } else {
                0
            },
        );
        let groups: &[CraftGroup] = if c.groups_on { &fx.groups } else { &[] };
        emu.write_u32(fx.rec_a, c.sa as u32);
        emu.write_u32(fx.rec_a + 4, c.gka as u32);
        emu.write_u32(fx.rec_a + 0xc, c.kca as u32);
        emu.write_u32(fx.rec_b, c.sb as u32);
        emu.write_u32(fx.rec_b + 4, c.gkb as u32);
        emu.write_u32(fx.rec_b + 0xc, c.kcb as u32);
        let bin = emu.call(0x004f_67a0, 0, 0, &[fx.h_a as u32, fx.h_b as u32]) as i32;
        let ra = craft_row(&emu, fx.cache, c.sa, c.gka, c.kca);
        let rb = craft_row(&emu, fx.cache, c.sb, c.gkb, c.kcb);
        let mine = cmp_4f67a0(&ra, &rb, groups);
        // coverage
        if let Cmp::Collate { a, b, .. } = mine {
            collated += 1;
            if collate(&mut emu, A4C0, a, b) == 0 {
                tie += 1;
            }
        }
        if c.groups_on {
            group += 1;
        }
        if c.sa == -1 || c.sb == -1 {
            sentinel += 1;
        }
        if !c.groups_on && ra.name_obj != 0 && rb.name_obj != 0 && c.kca != c.kcb {
            kc += 1;
        }
        check(&mut emu, A4C0, bin, mine, &format!("4f67a0 case {i}"));
    }
    eprintln!(
        "4f67a0: {} cases ok (group={group} sentinel={sentinel} kc={kc} collate={collated} tie={tie})",
        cases.len()
    );
    assert!(group > 0 && sentinel > 0 && kc > 0 && collated > 0 && tie > 0);
}

#[test]
fn sortcmp_4f6920() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let fx = craft_group_setup(&mut emu);
    let mut cases = craft_cases();
    // ensure a name-tie-with-different-+0x74 case (id1 alpha +74=5 vs id3 alpha +74=2): tail decides.
    cases.insert(
        0,
        FC {
            sa: 1,
            sb: 3,
            gka: 0,
            gkb: 0,
            kca: 0,
            kcb: 0,
            groups_on: false,
        },
    );
    let (mut group, mut sentinel, mut kc, mut collated, mut tail) = (0, 0, 0, 0, 0);
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(
            0x00bd_cfc4,
            if c.groups_on {
                fx.groups.len() as u32
            } else {
                0
            },
        );
        let groups: &[CraftGroup] = if c.groups_on { &fx.groups } else { &[] };
        emu.write_u32(fx.rec_a, c.sa as u32);
        emu.write_u32(fx.rec_a + 4, c.gka as u32);
        emu.write_u32(fx.rec_a + 0xc, c.kca as u32);
        emu.write_u32(fx.rec_b, c.sb as u32);
        emu.write_u32(fx.rec_b + 4, c.gkb as u32);
        emu.write_u32(fx.rec_b + 0xc, c.kcb as u32);
        let bin = emu.call(0x004f_6920, 0, 0, &[fx.h_a as u32, fx.h_b as u32]) as i32;
        let ra = craft_row(&emu, fx.cache, c.sa, c.gka, c.kca);
        let rb = craft_row(&emu, fx.cache, c.sb, c.gkb, c.kcb);
        let mine = cmp_4f6920(&ra, &rb, groups);
        if let Cmp::Collate { a, b, tie } = mine {
            collated += 1;
            if collate(&mut emu, A4C0, a, b) == 0
                && tie.is_some()
                && tie.unwrap().0 != tie.unwrap().1
            {
                tail += 1;
            }
        }
        if c.groups_on {
            group += 1;
        }
        if c.sa == -1 || c.sb == -1 {
            sentinel += 1;
        }
        if !c.groups_on && ra.name_obj != 0 && rb.name_obj != 0 && c.kca != c.kcb {
            kc += 1;
        }
        check(&mut emu, A4C0, bin, mine, &format!("4f6920 case {i}"));
    }
    eprintln!(
        "4f6920: {} cases ok (group={group} sentinel={sentinel} kc={kc} collate={collated} tail={tail})",
        cases.len()
    );
    assert!(group > 0 && sentinel > 0 && kc > 0 && collated > 0 && tail > 0);
}

// `0x4fd380`, the comparator of `0x4fd180`

#[test]
fn sortcmp_4fd380() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);

    let s_alpha = cstr(&mut emu, "alpha");
    let s_bravo = cstr(&mut emu, "bravo");
    let o1 = name_obj(&mut emu, s_alpha, 0);
    let o2 = name_obj(&mut emu, s_bravo, 0);
    let o3 = name_obj(&mut emu, s_alpha, 0);
    let cache = emu.alloc(4 * 4);
    emu.write_u32(cache, 0);
    emu.write_u32(cache + 4, o1);
    emu.write_u32(cache + 8, o2);
    emu.write_u32(cache + 12, o3);
    emu.write_u32(D788, cache as u32);
    emu.write_u32(D78C, 3);

    // The table 0xbde000 is an array of pointers to entries; entry[0] and [1] are keys and entry[3]
    // a flag. The count is 0xbde050.
    let gdefs = [(100i32, 1i32, 1i32), (100, 2, 1), (200, 1, 0)]; // (e0,e1,e3); last has e3=0
    let gbase = emu.alloc(gdefs.len() as u64 * 4);
    let mut groups = Vec::new();
    for (i, &(e0, e1, e3)) in gdefs.iter().enumerate() {
        let e = emu.alloc(0x10);
        emu.write_u32(e, e0 as u32);
        emu.write_u32(e + 4, e1 as u32);
        emu.write_u32(e + 0xc, e3 as u32);
        emu.write_u32(gbase + (i as u64) * 4, e as u32);
        groups.push(Group4fd380 { e0, e1, e3 });
    }
    emu.write_u32(0x00bd_e000, gbase as u32);

    let rec_a = emu.alloc(0x20);
    let rec_b = emu.alloc(0x20);
    let h_a = emu.alloc(4);
    let h_b = emu.alloc(4);
    emu.write_u32(h_a, rec_a as u32);
    emu.write_u32(h_b, rec_b as u32);

    struct C {
        sign: i32,
        p4: i32,
        p8: i32,
        pc: i32,
        p14: i32,
        p1c: i32,
    }
    let mk = |emu: &Emu, c: &C| -> Rec4fd380 {
        let obj = if (0..=3).contains(&c.sign) {
            emu.read_u32(cache + (c.sign as u64) * 4)
        } else {
            0
        };
        Rec4fd380 {
            sign_id: c.sign,
            plus4: c.p4,
            plus8: c.p8,
            plus_c: c.pc,
            plus14: c.p14,
            plus1c: c.p1c,
            name_obj: obj,
            name_str: if obj != 0 {
                emu.read_u32(obj as u64 + 0x1e0)
            } else {
                0
            },
        }
    };
    let write_rec = |emu: &mut Emu, base: u64, c: &C| {
        emu.write_u32(base, c.sign as u32);
        emu.write_u32(base + 4, c.p4 as u32);
        emu.write_u32(base + 8, c.p8 as u32);
        emu.write_u32(base + 0xc, c.pc as u32);
        emu.write_u32(base + 0x14, c.p14 as u32);
        emu.write_u32(base + 0x1c, c.p1c as u32);
    };

    struct Pair {
        a: C,
        b: C,
        groups_on: bool,
    }
    let mut cases: Vec<Pair> = vec![
        // flag-group index desc (both grouped, p1c!=0 & e3!=0): entry0 (idx0) vs entry1 (idx1)
        Pair {
            a: C {
                sign: 1,
                p4: 0,
                p8: 100,
                pc: 1,
                p14: 0,
                p1c: 1,
            },
            b: C {
                sign: 2,
                p4: 0,
                p8: 100,
                pc: 2,
                p14: 0,
                p1c: 1,
            },
            groups_on: true,
        },
        // a grouped, b not (b matches the e3=0 entry -> grp false)
        Pair {
            a: C {
                sign: 1,
                p4: 0,
                p8: 100,
                pc: 1,
                p14: 0,
                p1c: 1,
            },
            b: C {
                sign: 2,
                p4: 0,
                p8: 200,
                pc: 1,
                p14: 0,
                p1c: 1,
            },
            groups_on: true,
        },
        // neither grouped (both p1c=0) -> 0
        Pair {
            a: C {
                sign: 1,
                p4: 0,
                p8: 100,
                pc: 1,
                p14: 0,
                p1c: 0,
            },
            b: C {
                sign: 2,
                p4: 0,
                p8: 100,
                pc: 2,
                p14: 0,
                p1c: 0,
            },
            groups_on: true,
        },
        // name path (groups off): sentinel
        Pair {
            a: C {
                sign: -1,
                p4: 0,
                p8: 0,
                pc: 0,
                p14: 0,
                p1c: 0,
            },
            b: C {
                sign: 2,
                p4: 0,
                p8: 0,
                pc: 0,
                p14: 0,
                p1c: 0,
            },
            groups_on: false,
        },
        // +0x4 desc
        Pair {
            a: C {
                sign: 1,
                p4: 3,
                p8: 0,
                pc: 0,
                p14: 0,
                p1c: 0,
            },
            b: C {
                sign: 2,
                p4: 7,
                p8: 0,
                pc: 0,
                p14: 0,
                p1c: 0,
            },
            groups_on: false,
        },
        // +0x14 asc
        Pair {
            a: C {
                sign: 1,
                p4: 5,
                p8: 0,
                pc: 0,
                p14: 2,
                p1c: 0,
            },
            b: C {
                sign: 2,
                p4: 5,
                p8: 0,
                pc: 0,
                p14: 8,
                p1c: 0,
            },
            groups_on: false,
        },
        // name tie (alpha==alpha2): sign 1 vs 3, equal p4/p14
        Pair {
            a: C {
                sign: 1,
                p4: 5,
                p8: 0,
                pc: 0,
                p14: 4,
                p1c: 0,
            },
            b: C {
                sign: 3,
                p4: 5,
                p8: 0,
                pc: 0,
                p14: 4,
                p1c: 0,
            },
            groups_on: false,
        },
    ];
    let mut s = 0x4fd3_8000u32;
    for _ in 0..1500 {
        let on = rib(&mut s, 2) == 0;
        let mkc = |s: &mut u32| C {
            sign: rib(s, 6) as i32 - 2,
            p4: rib(s, 4) as i32,
            p8: [100, 200, 300][rib(s, 3) as usize],
            pc: rib(s, 3) as i32,
            p14: rib(s, 4) as i32,
            p1c: rib(s, 2) as i32,
        };
        cases.push(Pair {
            a: mkc(&mut s),
            b: mkc(&mut s),
            groups_on: on,
        });
    }

    let (mut grp_idx, mut grp_flag, mut sentinel, mut p4k, mut p14k, mut collated, mut tie) =
        (0, 0, 0, 0, 0, 0, 0);
    for (i, p) in cases.iter().enumerate() {
        emu.write_u32(
            0x00bd_e050,
            if p.groups_on { groups.len() as u32 } else { 0 },
        );
        let g: &[Group4fd380] = if p.groups_on { &groups } else { &[] };
        write_rec(&mut emu, rec_a, &p.a);
        write_rec(&mut emu, rec_b, &p.b);
        let bin = emu.call(0x004f_d380, 0, 0, &[h_a as u32, h_b as u32]) as i32;
        let ra = mk(&emu, &p.a);
        let rb = mk(&emu, &p.b);
        let mine = cmp_4fd380(&ra, &rb, g);
        if let Cmp::Collate { a, b, .. } = mine {
            collated += 1;
            if collate(&mut emu, A4C0, a, b) == 0 {
                tie += 1;
            }
        }
        if p.groups_on {
            grp_idx += 1;
            grp_flag += 1;
        }
        if p.a.sign == -1 || p.b.sign == -1 {
            sentinel += 1;
        }
        if !p.groups_on && ra.name_obj != 0 && rb.name_obj != 0 {
            if p.a.p4 != p.b.p4 {
                p4k += 1;
            } else if p.a.p14 != p.b.p14 {
                p14k += 1;
            }
        }
        check(&mut emu, A4C0, bin, mine, &format!("4fd380 case {i}"));
    }
    eprintln!(
        "4fd380: {} cases ok (grp={grp_idx}/{grp_flag} sentinel={sentinel} p4={p4k} p14={p14k} collate={collated} tie={tie})",
        cases.len()
    );
    assert!(grp_idx > 0 && sentinel > 0 && p4k > 0 && p14k > 0 && collated > 0 && tie > 0);
}

// `0x4fd540`, the other comparator of `0x4fd180`

#[test]
fn sortcmp_4fd540() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(LOCALE, 0);

    // The table 0xc0db90, stride 0x1d dwords; the count is 0xc0db94.
    // entry: [0]=e0, [1]=e1, [0x13]=primary name (loc0), [10]=fallback name.
    let s_alpha = cstr(&mut emu, "alpha");
    let s_bravo = cstr(&mut emu, "bravo");
    let s_empty = cstr(&mut emu, "");
    let s_fb = cstr(&mut emu, "fallbackname");
    let stride = 0x1du64 * 4;
    // 4 entries: (e0,e1, primary, fallback)
    let defs = [
        (100i32, 1i32, s_alpha, 0u32), // primary alpha
        (100, 2, s_bravo, 0),          // primary bravo
        (100, 3, s_empty, s_fb),       // primary empty -> fallback
        (100, 4, 0, s_fb),             // primary null -> fallback
    ];
    let base = emu.alloc(defs.len() as u64 * stride);
    let mut craft = Vec::new();
    for (i, &(e0, e1, prim, fb)) in defs.iter().enumerate() {
        let e = base + (i as u64) * stride;
        emu.write_u32(e, e0 as u32);
        emu.write_u32(e + 4, e1 as u32);
        emu.write_u32(e + 0x13 * 4, prim);
        emu.write_u32(e + 10 * 4, fb);
        let primary_empty = prim == 0 || emu.read(prim as u64, 1)[0] == 0;
        craft.push(Entry4fd540 {
            e0,
            e1,
            name_primary: prim,
            primary_empty,
            name_fallback: fb,
        });
    }
    emu.write_u32(0x00c0_db90, base as u32);
    emu.write_u32(0x00c0_db94, defs.len() as u32);

    let rec_a = emu.alloc(8);
    let rec_b = emu.alloc(8);
    let h_a = emu.alloc(4);
    let h_b = emu.alloc(4);
    emu.write_u32(h_a, rec_a as u32);
    emu.write_u32(h_b, rec_b as u32);

    struct C {
        p0a: i32,
        p4a: i32,
        p0b: i32,
        p4b: i32,
    }
    let mut cases = vec![
        // +0 desc
        C {
            p0a: 100,
            p4a: 1,
            p0b: 200,
            p4b: 1,
        },
        C {
            p0a: 300,
            p4a: 1,
            p0b: 100,
            p4b: 1,
        },
        // same +0, distinct names (entry1 alpha vs entry2 bravo)
        C {
            p0a: 100,
            p4a: 1,
            p0b: 100,
            p4b: 2,
        },
        C {
            p0a: 100,
            p4a: 2,
            p0b: 100,
            p4b: 1,
        },
        // fallback selection (entry3 empty->fb, entry4 null->fb): both fallbackname -> tie
        C {
            p0a: 100,
            p4a: 3,
            p0b: 100,
            p4b: 4,
        },
        // alpha vs fallback
        C {
            p0a: 100,
            p4a: 1,
            p0b: 100,
            p4b: 3,
        },
        // unmatched (+0 same, +4 not in table) -> 0
        C {
            p0a: 100,
            p4a: 99,
            p0b: 100,
            p4b: 1,
        },
    ];
    let mut s = 0x4fd5_4000u32;
    for _ in 0..1500 {
        cases.push(C {
            p0a: [100, 200, 300][rib(&mut s, 3) as usize],
            p4a: rib(&mut s, 6) as i32,
            p0b: [100, 200, 300][rib(&mut s, 3) as usize],
            p4b: rib(&mut s, 6) as i32,
        });
    }

    let (mut desc, mut collated, mut tie, mut fb_used, mut zero) = (0, 0, 0, 0, 0);
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(rec_a, c.p0a as u32);
        emu.write_u32(rec_a + 4, c.p4a as u32);
        emu.write_u32(rec_b, c.p0b as u32);
        emu.write_u32(rec_b + 4, c.p4b as u32);
        let bin = emu.call(0x004f_d540, 0, 0, &[h_a as u32, h_b as u32]) as i32;
        let ra = Rec4fd540 {
            plus0: c.p0a,
            plus4: c.p4a,
        };
        let rb = Rec4fd540 {
            plus0: c.p0b,
            plus4: c.p4b,
        };
        let mine = cmp_4fd540(&ra, &rb, &craft);
        if c.p0a != c.p0b {
            desc += 1;
        }
        match mine {
            Cmp::Collate { a, b, .. } => {
                collated += 1;
                if collate(&mut emu, A4C0, a, b) == 0 {
                    tie += 1;
                }
                if a == s_fb || b == s_fb {
                    fb_used += 1;
                }
            }
            Cmp::Eq if c.p0a == c.p0b => zero += 1,
            _ => {}
        }
        check(&mut emu, A4C0, bin, mine, &format!("4fd540 case {i}"));
    }
    eprintln!(
        "4fd540: {} cases ok (desc={desc} collate={collated} tie={tie} fallback={fb_used} zero={zero})",
        cases.len()
    );
    assert!(desc > 0 && collated > 0 && tie > 0 && fb_used > 0 && zero > 0);
}
