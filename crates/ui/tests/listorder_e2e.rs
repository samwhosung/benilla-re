//! The visible row order of the two list windows whose sort a client renders directly:
//! `ClassTrainerFrame` at trainer type 2 (tradeskill) and `CraftFrame` at craft type 1 (beast
//! training) (`systems/ui/trainerlist.md`, `systems/ui/craft.md`).
//!
//! The comparator tests assert one result of -1, 0 or 1 at a time. These tests run the client's own
//! list finalizers over a synthesized list state and read the sorted pointer array back. Everything
//! on the path is the exe's code: the visibility and collapse pass, the C runtime's `qsort`
//! (`0x73f727`), the comparator the mode selects and the locale collator (`0x64a4c0`, `0x64a480`).
//! Nothing is stubbed.
//!
//! The order a player sees comes from the whole cascade, the group partition and the comparator
//! the mode selects, so a per-key test cannot catch a wrong comparator choice, a wrong group key or
//! a key that never separates real rows. Each case is driven by the spell ids, names and effects of
//! a real trainer's list, so a key that is constant across the real rows shows as constant.
#![allow(clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};

// ── ClassTrainerFrame globals (ClassTrainerFrame.cpp, __FILE__ 0x84a82c) ───────────────────────
const T_ROWS: u64 = 0x00b7_37b0; // display-row pointer array base
const T_HDRS: u64 = 0x00b7_39c0; // group-header pointer array base
const T_TYPE: u64 = 0x00b7_3a08; // the wire `trainerType`
const T_NROW: u64 = 0x00b7_3a10; // display-row count (rows + appended header rows)
const T_NHDR: u64 = 0x00b7_3a14; // header count
const T_NVIS: u64 = 0x00b7_3a18; // visible count — what GetNumTrainerServices returns
const T_STATEMASK: u64 = 0x00b7_3a1c;
const T_GROUPMASK: u64 = 0x00b7_3a20;
const T_EXPANDMASK: u64 = 0x00b7_3a24;
const TRAINER_FINALIZE: u64 = 0x004d_8410;
const TRAINER_GROUPKEY_SLICE_BEGIN: u64 = 0x004d_7781;
const TRAINER_GROUPKEY_SLICE_END: u64 = 0x004d_7807;

// ── CraftFrame globals (CraftInfo.cpp, __FILE__ 0x84d368) ──────────────────────────────────────
const C_ROWS: u64 = 0x00bd_cf78;
const C_HDRS: u64 = 0x00bd_cf88;
const C_TYPE: u64 = 0x00bd_cfb8; // craft type: 1=TRAIN 2=DISGUISE 3=ENSCRIBE
const C_NROW: u64 = 0x00bd_cfc0;
const C_NHDR: u64 = 0x00bd_cfc4;
const C_NVIS: u64 = 0x00bd_cfc8; // what GetNumCrafts returns
const C_EXPANDMASK: u64 = 0x0084_d0f0;
const CRAFT_RESORT: u64 = 0x004f_6630;

// ── shared stores ──────────────────────────────────────────────────────────────────────────────
const SPELL_ROWS: u64 = 0x00c0_d788; // ClientDB<SpellRec>::m_recordsById
const SPELL_MAXID: u64 = 0x00c0_d78c;
const SKILLLINE_ROWS: u64 = 0x00c0_d92c;
const SKILLLINE_MAXID: u64 = 0x00c0_d930;
const LOCALE: u64 = 0x00c0_e080;

const SPELLREC_SIZE: u64 = 692; // 173 fields × 4 — the shipped Spell.dbc record stride
const SPELLREC_NAME: u64 = 0x1e0; // field 120 SpellName_Lang[locale]
const SPELLREC_RANK: u64 = 0x204; // field 129 Rank_Lang[locale]
const SPELLREC_LEVEL: u64 = 0x74; // field 29 spellLevel
const SPELLREC_EFFECT: u64 = 0xf4; // field 61 Effect[0..2]

const TRAINER_ROW_SIZE: u64 = 0x38;
const TRAINER_HDR_SIZE: u64 = 0x28;
const CRAFT_ROW_SIZE: u64 = 0x10;
const CRAFT_HDR_SIZE: u64 = 0x0c;

fn cstr(emu: &mut Emu, s: &str) -> u64 {
    let a = emu.alloc(s.len() as u64 + 1);
    emu.write(a, s.as_bytes());
    a
}

/// Build the `ClientDB<SpellRec>` id→record table: `rows[id] = rec`, `maxid = max`. Only the ids
/// we name get a record; every other slot stays null, which is exactly the shipped store's shape
/// for a sparse id space.
fn spell_store(emu: &mut Emu, specs: &[SpellSpec]) {
    let maxid = specs.iter().map(|s| s.id).max().unwrap();
    let table = emu.alloc(u64::from(maxid + 1) * 4);
    for s in specs {
        let rec = emu.alloc(SPELLREC_SIZE);
        emu.write_u32(rec, s.id); // field 0 = Id
        emu.write_u32(rec + SPELLREC_LEVEL, s.spell_level);
        for (i, e) in s.effects.iter().enumerate() {
            emu.write_u32(rec + SPELLREC_EFFECT + (i as u64) * 4, *e);
        }
        let name = cstr(emu, s.name);
        emu.write_u32(rec + SPELLREC_NAME, name as u32);
        let rank = cstr(emu, s.rank);
        emu.write_u32(rec + SPELLREC_RANK, rank as u32);
        emu.write_u32(table + u64::from(s.id) * 4, rec as u32);
    }
    emu.write_u32(SPELL_ROWS, table as u32);
    emu.write_u32(SPELL_MAXID, maxid);
    emu.write_u32(LOCALE, 0);
}

struct SpellSpec {
    id: u32,
    name: &'static str,
    rank: &'static str,
    spell_level: u32,
    effects: [u32; 3],
}

// ═══ 1 · ClassTrainerFrame, trainerType 2 ══════════════════════════════════════════════════════

/// One `SMSG_TRAINER_LIST` service record, in 1.12 wire field order.
struct TrainerWire {
    spell: u32,
    state: u8,
    req_level: u8,
    req_skill: u32,
    req_skill_value: u32,
    prof_cost: u32,
}

/// The real `npc_trainer` set of creature 957 "Dane Lindgren" (`trainer_type = 2`, tradeskill),
/// in wire order. Every recipe is red (state 1) for a fresh blacksmith; the profession-learn
/// service is green (state 0). `talentCost` and `reqSpell[3]` are 0 throughout.
const DANE_LINDGREN: &[TrainerWire] = &[
    TrainerWire {
        spell: 2020,
        state: 0,
        req_level: 5,
        req_skill: 0,
        req_skill_value: 0,
        prof_cost: 1,
    },
    TrainerWire {
        spell: 2743,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 1,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 2754,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 15,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 2755,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 20,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 3340,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 20,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 2756,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 25,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 3341,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 25,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 9984,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 30,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 8881,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 30,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 2744,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 35,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 3299,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 35,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 3342,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 40,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 3343,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 45,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 2746,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 65,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 7409,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 65,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 3118,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 65,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 3300,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 70,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 2747,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 70,
        prof_cost: 0,
    },
    TrainerWire {
        spell: 3344,
        state: 1,
        req_level: 0,
        req_skill: 164,
        req_skill_value: 75,
        prof_cost: 0,
    },
];

/// The shipped `Spell.dbc` rows for that set. `Effect[]` is quoted verbatim because the type-2
/// group key is decided by whether `Effect[0..2]` contains 44 (`SPELL_EFFECT_SKILL_STEP`) —
/// spell 2020 "Apprentice Blacksmith" carries `[36, 44, 0]`, every recipe wrapper `[36, 0, 0]`.
const DANE_SPELLS: &[SpellSpec] = &[
    SpellSpec {
        id: 2020,
        name: "Apprentice Blacksmith",
        rank: "",
        spell_level: 0,
        effects: [36, 44, 0],
    },
    SpellSpec {
        id: 2743,
        name: "Copper Chain Pants",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 2754,
        name: "Copper Mace",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 2755,
        name: "Copper Axe",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 3340,
        name: "Copper Chain Boots",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 2756,
        name: "Copper Shortsword",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 3341,
        name: "Rough Grinding Stone",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 9984,
        name: "Copper Claymore",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 8881,
        name: "Copper Dagger",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 2744,
        name: "Copper Chain Belt",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 3299,
        name: "Copper Battle Axe",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 3342,
        name: "Runed Copper Gauntlets",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 3343,
        name: "Runed Copper Pants",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 2746,
        name: "Coarse Sharpening Stone",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 7409,
        name: "Heavy Copper Maul",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 3118,
        name: "Coarse Weightstone",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 3300,
        name: "Thick War Axe",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 2747,
        name: "Runed Copper Belt",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 3344,
        name: "Coarse Grinding Stone",
        rank: "",
        spell_level: 0,
        effects: [36, 0, 0],
    },
];

/// Execute the builder's own type-2 group-key slice `[0x4d7781, 0x4d7807)` on one row. Returns the
/// key the binary computes (EDI at the halt). This is the byte-level proof that at trainerType 2
/// the group key is NOT the taught spell's skill line but a two-valued `SKILL_STEP` partition.
fn emu_group_key(emu: &mut Emu, rows_base: u64, index: u64) -> u32 {
    use benilla_re_difftest::RegisterX86 as R;
    let frame = emu.alloc(0x40) + 0x20; // a scratch EBP with room for [ebp-0x8]
    emu.set_reg(R::EBP, frame);
    emu.set_reg(R::ESI, index * 4);
    emu.set_reg(R::EDI, 0);
    let _ = rows_base;
    emu.run(TRAINER_GROUPKEY_SLICE_BEGIN, TRAINER_GROUPKEY_SLICE_END);
    emu.get_reg(R::EDI) as u32
}

#[test]
fn listorder_trainer_tradeskill_e2e() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    spell_store(&mut emu, DANE_SPELLS);

    // ── the builder's output: one display row per wire record, in wire order ────────────────────
    let n = DANE_LINDGREN.len() as u64;
    let row_recs: Vec<u64> = (0..n).map(|_| emu.alloc(TRAINER_ROW_SIZE)).collect();
    for (i, w) in DANE_LINDGREN.iter().enumerate() {
        let r = row_recs[i];
        emu.write_u32(r, w.spell); // +0x00 spell id
        emu.write_u32(r + 0x10, w.prof_cost); // +0x10 profCost
        emu.write(r + 0x14, &[w.req_level]); // +0x14 reqLevel (u8)
        emu.write_u32(r + 0x18, w.req_skill); // +0x18 reqSkill
        emu.write_u32(r + 0x1c, w.req_skill_value); // +0x1c reqSkillValue
        emu.write(r + 0x30, &[w.state]); // +0x30 state (u8)
    }
    // The pointer array the builder fills and the finalizer sorts.
    let rows_arr = emu.alloc((n + 2) * 4);
    for (i, r) in row_recs.iter().enumerate() {
        emu.write_u32(rows_arr + (i as u64) * 4, *r as u32);
    }
    emu.write_u32(T_ROWS, rows_arr as u32);
    emu.write_u32(T_TYPE, 2);

    // ── group keys, computed by the binary's own slice, then stored to [row+4] as it does ──────
    let mut keys = Vec::new();
    for i in 0..n {
        let k = emu_group_key(&mut emu, rows_arr, i);
        emu.write_u32(row_recs[i as usize] + 4, k);
        keys.push(k);
    }
    // Spell 2020 carries Effect[1] = 44 (SKILL_STEP); every recipe wrapper carries none.
    assert_eq!(
        keys[0], 1,
        "the profession-learn service must land in group 1"
    );
    assert!(
        keys[1..].iter().all(|&k| k == 2),
        "every recipe must land in group 2"
    );

    // ── the two headers the builder creates, in first-seen order, then qsorted by 0x4d7c30 ─────
    // 0x4d7c30 is a single signed-ascending compare on hdr[+0], so {1, 2} is already ordered.
    let hdr1 = emu.alloc(TRAINER_HDR_SIZE);
    emu.write_u32(hdr1, 1);
    emu.write_u32(hdr1 + 4, 1); // one row in state 0 (the green learn service)
    emu.write_u32(hdr1 + 0x24, 1); // profCost != 0 for its only member
    let hdr2 = emu.alloc(TRAINER_HDR_SIZE);
    emu.write_u32(hdr2, 2);
    emu.write_u32(hdr2 + 8, n as u32 - 1); // 18 rows in state 1 (red recipes)
    let hdrs_arr = emu.alloc(2 * 4);
    emu.write_u32(hdrs_arr, hdr1 as u32);
    emu.write_u32(hdrs_arr + 4, hdr2 as u32);
    emu.write_u32(T_HDRS, hdrs_arr as u32);
    emu.write_u32(T_NHDR, 2);

    // ── the appended header display rows (0x4d7af4: [+0] = -1, [+4] = the group key) ───────────
    let hrow1 = emu.alloc(TRAINER_ROW_SIZE);
    emu.write_u32(hrow1, u32::MAX);
    emu.write_u32(hrow1 + 4, 1);
    let hrow2 = emu.alloc(TRAINER_ROW_SIZE);
    emu.write_u32(hrow2, u32::MAX);
    emu.write_u32(hrow2 + 4, 2);
    emu.write_u32(rows_arr + n * 4, hrow1 as u32);
    emu.write_u32(rows_arr + (n + 1) * 4, hrow2 as u32);
    emu.write_u32(T_NROW, n as u32 + 2);

    // Every state bit on, every group shown, nothing collapsed (the dispatch's premise; the
    // client's own default state mask is 3, which selects the same rows for this set).
    emu.write_u32(T_STATEMASK, 0x3f);
    emu.write_u32(T_GROUPMASK, u32::MAX);
    emu.write_u32(T_EXPANDMASK, u32::MAX);

    // ── run the client's real finalizer: visibility pass + qsort 0x73f727 + cmp 0x4d8760 ───────
    emu.call(TRAINER_FINALIZE, 0, 0, &[]);

    let visible = emu.read_u32(T_NVIS);
    assert_eq!(
        visible,
        n as u32 + 2,
        "nothing is filtered out in this configuration"
    );

    let order: Vec<(i32, u32, u32)> = (0..visible as u64)
        .map(|i| {
            let r = u64::from(emu.read_u32(rows_arr + i * 4));
            (
                emu.read_u32(r) as i32,
                emu.read_u32(r + 4),
                emu.read_u32(r + 0x1c),
            )
        })
        .collect();

    // Assert the LAW rather than a hand-transcribed table: the two headers lead their groups,
    // group 1 precedes group 2, reqSkillValue ascends, and each tie is broken by the collator.
    assert_eq!(order[0].0, -1, "group-1 header first");
    assert_eq!(order[0].1, 1);
    assert_eq!(
        order[1].0, 2020,
        "the profession-learn service is row 2, not last"
    );
    assert_eq!(order[2].0, -1, "group-2 header next");
    assert_eq!(order[2].1, 2);
    let recipes = &order[3..];
    assert_eq!(recipes.len(), 18);
    for w in recipes.windows(2) {
        assert!(
            w[0].2 <= w[1].2,
            "reqSkillValue must ASCEND across the recipe block, got {} then {}",
            w[0].2,
            w[1].2
        );
    }
    assert_eq!(recipes[0].0, 2743, "reqSkillValue 1 leads the recipe block");
    assert_eq!(recipes[17].0, 3344, "reqSkillValue 75 ends it");

    // Print the full order so the note can quote it verbatim.
    let names: std::collections::HashMap<u32, &str> =
        DANE_SPELLS.iter().map(|s| (s.id, s.name)).collect();
    for (i, (sp, key, rsv)) in order.iter().enumerate() {
        let label = if *sp < 0 {
            if *key == 1 {
                "<header TRADESKILL_SERVICE_STEP>"
            } else {
                "<header TRADESKILL_SERVICE_LEARN>"
            }
        } else {
            names[&(*sp as u32)]
        };
        println!("trainer {:2}. grp={} rsv={:3} {}", i + 1, key, rsv, label);
    }
}

// ═══ 2 · CraftFrame, craft type 1 (Beast Training) ═════════════════════════════════════════════

/// A hunter's known pet-teach spells, the tier the player actually knows (`castUI == 1`,
/// `EffectImplicitTargetA == 5`), with the shipped `Spell.dbc` `spellLevel`. Deliberately fed in a
/// scrambled order so the sort has to do all the work.
const PET_SPELLS: &[SpellSpec] = &[
    SpellSpec {
        id: 24509,
        name: "Arcane Resistance",
        rank: "Rank 3",
        spell_level: 40,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 24441,
        name: "Fire Resistance",
        rank: "Rank 2",
        spell_level: 30,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 24495,
        name: "Arcane Resistance",
        rank: "Rank 1",
        spell_level: 20,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 24464,
        name: "Fire Resistance",
        rank: "Rank 4",
        spell_level: 50,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 24510,
        name: "Arcane Resistance",
        rank: "Rank 4",
        spell_level: 50,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 24440,
        name: "Fire Resistance",
        rank: "Rank 1",
        spell_level: 20,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 24463,
        name: "Fire Resistance",
        rank: "Rank 3",
        spell_level: 40,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 24508,
        name: "Arcane Resistance",
        rank: "Rank 2",
        spell_level: 30,
        effects: [36, 0, 0],
    },
];

#[test]
fn listorder_craft_beast_training_e2e() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    spell_store(&mut emu, PET_SPELLS);

    // Every one of these sits on SkillLineAbility skill line 261 ("Beast Training") with
    // `max_value = 0`, so the builder's difficulty ladder never fires and every row's +0x0c
    // stays 0 — and `req_skill_value` is a constant 1 across all 111 rows of that line.
    let n = PET_SPELLS.len() as u64;
    let recs: Vec<u64> = (0..n).map(|_| emu.alloc(CRAFT_ROW_SIZE)).collect();
    for (i, s) in PET_SPELLS.iter().enumerate() {
        emu.write_u32(recs[i], s.id); // +0x00 spell id
        emu.write_u32(recs[i] + 4, 261); // +0x04 group key = SkillLineAbility.skillId
        emu.write_u32(recs[i] + 8, 0); // +0x08 numAvailable
        emu.write_u32(recs[i] + 0xc, 0); // +0x0c difficulty bucket ("none")
    }
    let rows_arr = emu.alloc(n * 4);
    for (i, r) in recs.iter().enumerate() {
        emu.write_u32(rows_arr + (i as u64) * 4, *r as u32);
    }
    emu.write_u32(C_ROWS, rows_arr as u32);
    emu.write_u32(C_NROW, n as u32);

    // One skill line ⇒ one header. `0x4f65fb` forces the header count to 0 in that case, so the
    // header array is present but empty and the row comparator's group phase never runs.
    let hdr = emu.alloc(CRAFT_HDR_SIZE);
    emu.write_u32(hdr, 261);
    let hdrs_arr = emu.alloc(4);
    emu.write_u32(hdrs_arr, hdr as u32);
    emu.write_u32(C_HDRS, hdrs_arr as u32);
    emu.write_u32(C_NHDR, 0);
    emu.write_u32(C_EXPANDMASK, u32::MAX);
    emu.write_u32(SKILLLINE_ROWS, 0);
    emu.write_u32(SKILLLINE_MAXID, 0);

    emu.write_u32(C_TYPE, 1); // TRAIN — selects comparator 0x4f6920 at 0x4f6765
    emu.call(CRAFT_RESORT, 0, 0, &[]);

    let visible = emu.read_u32(C_NVIS);
    assert_eq!(visible, n as u32);
    let order: Vec<u32> = (0..n)
        .map(|i| {
            let r = u64::from(emu.read_u32(rows_arr + i * 4));
            emu.read_u32(r)
        })
        .collect();

    let by_id: std::collections::HashMap<u32, &SpellSpec> =
        PET_SPELLS.iter().map(|s| (s.id, s)).collect();
    for (i, id) in order.iter().enumerate() {
        let s = by_id[id];
        println!(
            "craft(TRAIN) {}. {} ({}) spellLevel={} [{}]",
            i + 1,
            s.name,
            s.rank,
            s.spell_level,
            id
        );
    }
    assert_eq!(
        order,
        vec![24495, 24508, 24509, 24510, 24440, 24441, 24463, 24464],
        "name ascending, then Spell.dbc spellLevel ascending — ranks ascend inside each name group"
    );

    // ── the control that makes the finding falsifiable ─────────────────────────────────────────
    // At any craft type OTHER than 1 the client selects `0x4f67a0`, which has NO spellLevel key.
    // Every same-name pair then ties on the whole cascade and the unstable qsort leaves them in
    // an arbitrary order. Re-run the identical state at type 3 (ENSCRIBE) and show the ranks are
    // no longer ascending — this is exactly the failure a re-implementation reproduces if it
    // omits the third key.
    let mut emu2 = Emu::new(&wow);
    spell_store(&mut emu2, PET_SPELLS);
    let recs2: Vec<u64> = (0..n).map(|_| emu2.alloc(CRAFT_ROW_SIZE)).collect();
    for (i, s) in PET_SPELLS.iter().enumerate() {
        emu2.write_u32(recs2[i], s.id);
        emu2.write_u32(recs2[i] + 4, 261);
        emu2.write_u32(recs2[i] + 0xc, 0);
    }
    let rows2 = emu2.alloc(n * 4);
    for (i, r) in recs2.iter().enumerate() {
        emu2.write_u32(rows2 + (i as u64) * 4, *r as u32);
    }
    emu2.write_u32(C_ROWS, rows2 as u32);
    emu2.write_u32(C_NROW, n as u32);
    let hdr2 = emu2.alloc(CRAFT_HDR_SIZE);
    emu2.write_u32(hdr2, 261);
    let hdrs2 = emu2.alloc(4);
    emu2.write_u32(hdrs2, hdr2 as u32);
    emu2.write_u32(C_HDRS, hdrs2 as u32);
    emu2.write_u32(C_NHDR, 0);
    emu2.write_u32(C_EXPANDMASK, u32::MAX);
    emu2.write_u32(SKILLLINE_ROWS, 0);
    emu2.write_u32(SKILLLINE_MAXID, 0);
    emu2.write_u32(C_TYPE, 3); // ENSCRIBE — selects 0x4f67a0, no spellLevel key
    emu2.call(CRAFT_RESORT, 0, 0, &[]);
    let order3: Vec<u32> = (0..n)
        .map(|i| {
            let r = u64::from(emu2.read_u32(rows2 + i * 4));
            emu2.read_u32(r)
        })
        .collect();
    for (i, id) in order3.iter().enumerate() {
        let s = by_id[id];
        println!(
            "craft(ENSCRIBE control) {}. {} ({}) [{}]",
            i + 1,
            s.name,
            s.rank,
            id
        );
    }
    assert_ne!(
        order3, order,
        "the spellLevel key is the ONLY thing ordering ranks — without it the order differs"
    );
}

// ═══ 3 · ClassTrainerFrame, trainerType 1 (vmangos TRAINER_TYPE_MOUNTS) ════════════════════════

/// The eight spells the shipped 1.12.1 `SkillLineAbility.dbc` puts on a *Riding* skill line — the
/// whole population a mount trainer can offer. `Effect = [3, 118, 0]` carries neither `LEARN_SPELL`
/// (36) nor `LEARN_PET_SPELL` (57), so `0x4d7c60` leaves its `effIdx` sentinel at `-1` and queries
/// `0x60c920` with the spell's **own** id (`0x4d7d25`), which resolves to its riding skill line.
const RIDING_SPELLS: &[SpellSpec] = &[
    SpellSpec {
        id: 824,
        name: "Horse Riding",
        rank: "Passive",
        spell_level: 1,
        effects: [3, 118, 0],
    },
    SpellSpec {
        id: 826,
        name: "Ram Riding",
        rank: "Passive",
        spell_level: 1,
        effects: [3, 118, 0],
    },
    SpellSpec {
        id: 828,
        name: "Tiger Riding",
        rank: "Passive",
        spell_level: 1,
        effects: [3, 118, 0],
    },
    SpellSpec {
        id: 33388,
        name: "Riding",
        rank: "Apprentice",
        spell_level: 1,
        effects: [3, 118, 0],
    },
    SpellSpec {
        id: 33391,
        name: "Riding",
        rank: "Journeyman",
        spell_level: 1,
        effects: [3, 118, 0],
    },
];

/// `trainerType == 1` is the one mode where the **state byte `[+0x30]`** is a sort key. The wire
/// roster and the per-row states here are representative (a live mount trainer's exact offering is a
/// world-DB fact, not a client one); the ORDER is the binary's, produced by the real finalizer.
///
/// Group keys follow the builder's type-1 branch (`0x4d77da`–`0x4d77e8`): a row whose state is `2`
/// ("used" / already known) goes to the pre-seeded **`-1`** group — the one labelled by the
/// `KNOWN_TALENTS_HEADER` global string (`ds:0x84a3d8` → `0x84a7cc`) — and every other row falls to
/// `0x4d7c60`, i.e. its own SkillLine id.
#[test]
fn listorder_trainer_mounts_e2e() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    spell_store(&mut emu, RIDING_SPELLS);

    // (spell, state, group key). state: 0 = available, 1 = unavailable, 2 = used/known.
    // Fed deliberately out of order so the sort has to do all the work.
    let wire: &[(u32, u8, i32)] = &[
        (33391, 1, 762), // Riding (Journeyman) — unavailable
        (828, 2, -1),    // Tiger Riding        — already known ⇒ the -1 group
        (826, 0, 152),   // Ram Riding          — available
        (33388, 0, 762), // Riding (Apprentice) — available
        (824, 1, 148),   // Horse Riding        — unavailable
    ];
    let n = wire.len() as u64;

    let recs: Vec<u64> = (0..n).map(|_| emu.alloc(TRAINER_ROW_SIZE)).collect();
    for (i, (sp, st, key)) in wire.iter().enumerate() {
        emu.write_u32(recs[i], *sp);
        emu.write_u32(recs[i] + 4, *key as u32);
        emu.write(recs[i] + 0x30, &[*st]);
    }
    // Header array in the order `0x4d7b90` leaves it: `-1` first (its own leg at `0x4d7c1a`), then
    // ascending by `SkillLine.displayName` — "Horse Riding" < "Ram Riding" < "Riding" < "Tiger
    // Riding". `hdr[+0x24]` is 0 for all of them (no talentCost/profCost on a riding row).
    let hdr_keys: &[i32] = &[-1, 148, 152, 762];
    let hdrs: Vec<u64> = hdr_keys
        .iter()
        .map(|k| {
            let h = emu.alloc(TRAINER_HDR_SIZE);
            emu.write_u32(h, *k as u32);
            h
        })
        .collect();
    // Per-state member counters at hdr[+4 + state*4], so the finalizer marks each header visible.
    for (i, (_, st, key)) in wire.iter().enumerate() {
        let _ = i;
        let hi = hdr_keys.iter().position(|k| k == key).unwrap();
        let slot = hdrs[hi] + 4 + u64::from(*st) * 4;
        let prev = emu.read_u32(slot);
        emu.write_u32(slot, prev + 1);
    }
    let hdrs_arr = emu.alloc(hdrs.len() as u64 * 4);
    for (i, h) in hdrs.iter().enumerate() {
        emu.write_u32(hdrs_arr + (i as u64) * 4, *h as u32);
    }
    emu.write_u32(T_HDRS, hdrs_arr as u32);
    emu.write_u32(T_NHDR, hdrs.len() as u32);

    // Rows + one appended header display row per group (`0x4d7af4`: [+0] = -1, [+4] = the key).
    let total = n + hdrs.len() as u64;
    let rows_arr = emu.alloc(total * 4);
    for (i, r) in recs.iter().enumerate() {
        emu.write_u32(rows_arr + (i as u64) * 4, *r as u32);
    }
    for (i, k) in hdr_keys.iter().enumerate() {
        let hr = emu.alloc(TRAINER_ROW_SIZE);
        emu.write_u32(hr, u32::MAX);
        emu.write_u32(hr + 4, *k as u32);
        emu.write_u32(rows_arr + (n + i as u64) * 4, hr as u32);
    }
    emu.write_u32(T_ROWS, rows_arr as u32);
    emu.write_u32(T_NROW, total as u32);
    emu.write_u32(T_TYPE, 1);
    emu.write_u32(T_STATEMASK, 0x3f);
    emu.write_u32(T_GROUPMASK, u32::MAX);
    emu.write_u32(T_EXPANDMASK, u32::MAX);

    emu.call(TRAINER_FINALIZE, 0, 0, &[]);

    let visible = emu.read_u32(T_NVIS);
    assert_eq!(visible, total as u32);
    let names: std::collections::HashMap<u32, (&str, &str)> = RIDING_SPELLS
        .iter()
        .map(|s| (s.id, (s.name, s.rank)))
        .collect();
    let mut got = Vec::new();
    for i in 0..visible as u64 {
        let r = u64::from(emu.read_u32(rows_arr + i * 4));
        let sp = emu.read_u32(r) as i32;
        let key = emu.read_u32(r + 4) as i32;
        let st = emu.read(r + 0x30, 1)[0];
        got.push((sp, key));
        let label = if sp < 0 {
            format!("<header group {key}>")
        } else {
            let (n, rk) = names[&(sp as u32)];
            format!("{n} ({rk}) state={st}")
        };
        println!("trainer(type1) {:2}. grp={:4} {}", i + 1, key, label);
    }

    // The law: groups follow the header array's order (-1 first, then name-sorted), each group's
    // synthesized header row leads it, and WITHIN a group the state byte ascends (available before
    // unavailable before used) — the key that exists at no other trainer type.
    assert_eq!(
        got[0],
        (-1, -1),
        "the -1 (KNOWN_TALENTS_HEADER) group leads"
    );
    assert_eq!(got[1].0, 828, "the already-known row sits under it");
    let keys_in_order: Vec<i32> = got
        .iter()
        .filter(|(sp, _)| *sp < 0)
        .map(|(_, k)| *k)
        .collect();
    assert_eq!(
        keys_in_order,
        vec![-1, 148, 152, 762],
        "group order is the header array's order (name-sorted), not numeric"
    );
    // Group 762 holds both Riding rows: available (33388) must precede unavailable (33391).
    let g762: Vec<i32> = got
        .iter()
        .filter(|(sp, k)| *k == 762 && *sp > 0)
        .map(|(sp, _)| *sp)
        .collect();
    assert_eq!(
        g762,
        vec![33388, 33391],
        "state [+0x30] is ASCENDING at trainerType 1 — available (0) before unavailable (1)"
    );
}

// ═══ 4 · ClassTrainerFrame — does a STATE FILTER that empties a group also hide its HEADER? ═════

/// Real warrior-trainer spells on two class skill lines (`SkillLineAbility.dbc`): 26 "Arms" and
/// 256 "Fury".
const WARRIOR_SPELLS: &[SpellSpec] = &[
    SpellSpec {
        id: 78,
        name: "Heroic Strike",
        rank: "Rank 1",
        spell_level: 1,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 284,
        name: "Heroic Strike",
        rank: "Rank 2",
        spell_level: 8,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 1464,
        name: "Slam",
        rank: "Rank 1",
        spell_level: 30,
        effects: [36, 0, 0],
    },
    SpellSpec {
        id: 8820,
        name: "Slam",
        rank: "Rank 2",
        spell_level: 38,
        effects: [36, 0, 0],
    },
];

/// Build a two-group class trainer (`trainerType = 0`) and run the real finalizer once per filter
/// configuration. Group **Arms (26)** holds two `state = 1` ("unavailable") rows; group
/// **Fury (256)** holds two `state = 0` ("available") rows — so a state mask of `0b01` empties Arms
/// and leaves Fury untouched. Returns `(visible_count, [(spell_id, group_key), …])` in sorted order.
fn run_class_trainer(wow: &Wow, state_mask: u32, expand_mask: u32) -> (u32, Vec<(i32, i32)>) {
    let mut emu = Emu::new(wow);
    spell_store(&mut emu, WARRIOR_SPELLS);

    // (spell, state, group key)
    let wire: &[(u32, u8, i32)] = &[
        (78, 1, 26),    // Heroic Strike (Rank 1) — unavailable
        (284, 1, 26),   // Heroic Strike (Rank 2) — unavailable
        (1464, 0, 256), // Slam (Rank 1)          — available
        (8820, 0, 256), // Slam (Rank 2)          — available
    ];
    let n = wire.len() as u64;
    let recs: Vec<u64> = (0..n).map(|_| emu.alloc(TRAINER_ROW_SIZE)).collect();
    for (i, (sp, st, key)) in wire.iter().enumerate() {
        emu.write_u32(recs[i], *sp);
        emu.write_u32(recs[i] + 4, *key as u32);
        emu.write(recs[i] + 0x30, &[*st]);
    }

    // Headers in the order `0x4d7b90` leaves them (localized SkillLine name ASC: "Arms" < "Fury"),
    // each carrying the builder's BUILD-TIME per-state member counters at hdr[+4 + state*4].
    let hdr_keys: &[i32] = &[26, 256];
    let hdrs: Vec<u64> = hdr_keys
        .iter()
        .map(|k| {
            let h = emu.alloc(TRAINER_HDR_SIZE);
            emu.write_u32(h, *k as u32);
            h
        })
        .collect();
    for (_, st, key) in wire.iter() {
        let hi = hdr_keys.iter().position(|k| k == key).unwrap();
        let slot = hdrs[hi] + 4 + u64::from(*st) * 4;
        let prev = emu.read_u32(slot);
        emu.write_u32(slot, prev + 1);
    }
    let hdrs_arr = emu.alloc(hdrs.len() as u64 * 4);
    for (i, h) in hdrs.iter().enumerate() {
        emu.write_u32(hdrs_arr + (i as u64) * 4, *h as u32);
    }
    emu.write_u32(T_HDRS, hdrs_arr as u32);
    emu.write_u32(T_NHDR, hdrs.len() as u32);

    let total = n + hdrs.len() as u64;
    let rows_arr = emu.alloc(total * 4);
    for (i, r) in recs.iter().enumerate() {
        emu.write_u32(rows_arr + (i as u64) * 4, *r as u32);
    }
    for (i, k) in hdr_keys.iter().enumerate() {
        let hr = emu.alloc(TRAINER_ROW_SIZE);
        emu.write_u32(hr, u32::MAX);
        emu.write_u32(hr + 4, *k as u32);
        emu.write_u32(rows_arr + (n + i as u64) * 4, hr as u32);
    }
    emu.write_u32(T_ROWS, rows_arr as u32);
    emu.write_u32(T_NROW, total as u32);
    emu.write_u32(T_TYPE, 0); // class trainer ⇒ comparator 0x4d85c0
    emu.write_u32(T_STATEMASK, state_mask);
    emu.write_u32(T_GROUPMASK, u32::MAX);
    emu.write_u32(T_EXPANDMASK, expand_mask);

    emu.call(TRAINER_FINALIZE, 0, 0, &[]);

    let visible = emu.read_u32(T_NVIS);
    // `GetTrainerServiceInfo` indexes the sorted array directly, so the visible list is the prefix
    // `[0, ds:0xb73a18)` — the hidden rows are parked past it by the comparator's top key.
    let list = (0..visible as u64)
        .map(|i| {
            let r = u64::from(emu.read_u32(rows_arr + i * 4));
            (emu.read_u32(r) as i32, emu.read_u32(r + 4) as i32)
        })
        .collect();
    (visible, list)
}

/// **Does a state filter that empties a group also hide that group's header row?**
///
/// The two candidate readings are a false dichotomy. `hdr[+4 + state*4]` IS a build-time member
/// count (written by the builder at `0x4d785b`/`0x4d7979`, never recomputed) — but the finalizer
/// reads it *through the live state mask*: `0x4d8431` loads `ds:0xb73a1c`, `0x4d8447 test esi,edi`
/// skips any disabled state, and only `0x4d844e cmp [edi+ecx*4+0x4],ebx` / `0x4d8452 ja` on an
/// ENABLED state sets `hdr[+0x1c] = 1`. So the vanish test is effectively post-filter.
///
/// And the row loop's `hdr[+0x1c]` test at `0x4d8528` carries **no header-row exemption** — the hide
/// at `0x4d8535` runs for whatever row is current. The COLLAPSE test three instructions later does
/// have one (`0x4d853d cmp [eax],ebx` / `0x4d853f jl 0x4d854f`), and it reads a different field
/// (`hdr[+0x20]`, written at `0x4d8483` from `ds:0xb73a24`). That asymmetry is the whole answer.
#[test]
fn listorder_trainer_filter_hides_header_e2e() {
    let Some(wow) = Wow::load() else { return };

    // (a) baseline — every state shown, nothing collapsed.
    let (vis_all, list_all) = run_class_trainer(&wow, 0x3f, u32::MAX);
    for (sp, k) in &list_all {
        println!(
            "filter(all)      grp={k:3} {}",
            if *sp < 0 {
                "<header>".into()
            } else {
                sp.to_string()
            }
        );
    }
    assert_eq!(vis_all, 6, "2 headers + 4 services");
    assert_eq!(
        list_all,
        vec![
            (-1, 26),
            (78, 26),
            (284, 26),
            (-1, 256),
            (1464, 256),
            (8820, 256)
        ]
    );

    // (b) THE QUESTION — state mask 0b01 keeps state 0 only, so every Arms row (state 1) is hidden.
    let (vis_f, list_f) = run_class_trainer(&wow, 0x1, u32::MAX);
    for (sp, k) in &list_f {
        println!(
            "filter(state0)   grp={k:3} {}",
            if *sp < 0 {
                "<header>".into()
            } else {
                sp.to_string()
            }
        );
    }
    assert_eq!(
        list_f,
        vec![(-1, 256), (1464, 256), (8820, 256)],
        "a state filter that empties a group hides that group's HEADER too — no bare header survives"
    );
    assert_eq!(vis_f, 3);
    assert!(
        !list_f.iter().any(|(sp, k)| *sp < 0 && *k == 26),
        "the Arms header must NOT survive its group being emptied by the filter"
    );

    // (c) the control — COLLAPSE the Arms group instead (clear its bit in ds:0xb73a24), every state
    // still shown. The header must survive and only its services disappear.
    let (vis_c, list_c) = run_class_trainer(&wow, 0x3f, !1u32);
    for (sp, k) in &list_c {
        println!(
            "collapse(Arms)   grp={k:3} {}",
            if *sp < 0 {
                "<header>".into()
            } else {
                sp.to_string()
            }
        );
    }
    assert_eq!(
        list_c,
        vec![(-1, 26), (-1, 256), (1464, 256), (8820, 256)],
        "a COLLAPSED group keeps its header and loses only its services"
    );
    assert_eq!(vis_c, 4);

    // (d) every state box unchecked — the whole list, headers included, goes empty.
    let (vis_none, list_none) = run_class_trainer(&wow, 0x0, u32::MAX);
    println!("filter(none)     visible={vis_none} rows={list_none:?}");
    assert_eq!(
        vis_none, 0,
        "no state enabled ⇒ NOTHING renders, not even bare headers"
    );
    assert!(list_none.is_empty());
}

/// Build the real Blacksmithing trainer (`trainerType = 2`, the `DANE_LINDGREN` wire set) and run
/// the client's real finalizer once per state-filter configuration. The player here has **no rank
/// in Blacksmithing at all** — every recipe carries `reqSkill = 164` with `reqSkillValue` up to 75
/// and arrives from the server as `state = 1` ("unavailable"), exactly as `SMSG_TRAINER_LIST`
/// delivers it. Returns `(visible_count, [(spell_id, group_key), …])` in sorted order.
fn run_trainer_tradeskill(wow: &Wow, state_mask: u32) -> (u32, Vec<(i32, i32)>) {
    let mut emu = Emu::new(wow);
    spell_store(&mut emu, DANE_SPELLS);

    let n = DANE_LINDGREN.len() as u64;
    let recs: Vec<u64> = (0..n).map(|_| emu.alloc(TRAINER_ROW_SIZE)).collect();
    for (i, w) in DANE_LINDGREN.iter().enumerate() {
        let r = recs[i];
        emu.write_u32(r, w.spell);
        emu.write_u32(r + 0x10, w.prof_cost);
        emu.write(r + 0x14, &[w.req_level]);
        emu.write_u32(r + 0x18, w.req_skill);
        emu.write_u32(r + 0x1c, w.req_skill_value);
        emu.write(r + 0x30, &[w.state]);
    }
    let total = n + 2;
    let rows_arr = emu.alloc(total * 4);
    for (i, r) in recs.iter().enumerate() {
        emu.write_u32(rows_arr + (i as u64) * 4, *r as u32);
    }
    emu.write_u32(T_ROWS, rows_arr as u32);
    emu.write_u32(T_TYPE, 2);

    // Group keys from the builder's own slice, never hand-typed: 1 = TRADESKILL_SERVICE_STEP,
    // 2 = TRADESKILL_SERVICE_LEARN.
    let keys: Vec<u32> = (0..n)
        .map(|i| {
            let k = emu_group_key(&mut emu, rows_arr, i);
            emu.write_u32(recs[i as usize] + 4, k);
            k
        })
        .collect();

    // The two headers, each carrying the builder's build-time per-state member counters at
    // `hdr[+4 + state*4]` (`0x4d785b` / `0x4d7979`).
    let hdr_keys: &[u32] = &[1, 2];
    let hdrs: Vec<u64> = hdr_keys
        .iter()
        .map(|k| {
            let h = emu.alloc(TRAINER_HDR_SIZE);
            emu.write_u32(h, *k);
            h
        })
        .collect();
    for (i, w) in DANE_LINDGREN.iter().enumerate() {
        let hi = hdr_keys.iter().position(|k| *k == keys[i]).unwrap();
        let slot = hdrs[hi] + 4 + u64::from(w.state) * 4;
        let prev = emu.read_u32(slot);
        emu.write_u32(slot, prev + 1);
    }
    let hdrs_arr = emu.alloc(hdrs.len() as u64 * 4);
    for (i, h) in hdrs.iter().enumerate() {
        emu.write_u32(hdrs_arr + (i as u64) * 4, *h as u32);
    }
    emu.write_u32(T_HDRS, hdrs_arr as u32);
    emu.write_u32(T_NHDR, hdrs.len() as u32);

    for (i, k) in hdr_keys.iter().enumerate() {
        let hr = emu.alloc(TRAINER_ROW_SIZE);
        emu.write_u32(hr, u32::MAX);
        emu.write_u32(hr + 4, *k);
        emu.write_u32(rows_arr + (n + i as u64) * 4, hr as u32);
    }
    emu.write_u32(T_NROW, total as u32);

    emu.write_u32(T_STATEMASK, state_mask);
    emu.write_u32(T_GROUPMASK, u32::MAX);
    emu.write_u32(T_EXPANDMASK, u32::MAX);

    emu.call(TRAINER_FINALIZE, 0, 0, &[]);

    let visible = emu.read_u32(T_NVIS);
    let list = (0..visible as u64)
        .map(|i| {
            let r = u64::from(emu.read_u32(rows_arr + i * 4));
            (emu.read_u32(r) as i32, emu.read_u32(r + 4) as i32)
        })
        .collect();
    (visible, list)
}

/// **Is a profession recipe the player cannot learn suppressed at `trainerType == 2`?**
///
/// No — and this pins which mechanism *does* suppress it. The builder `0x4d7560` and the finalizer
/// `0x4d8410` never read the player's skill value; the only thing that removes a red recipe row is the
/// **state filter** `ds:0xb73a1c`, whose build-time default the builder writes itself at
/// `0x4d75d9` (`c7 05 1c 3a b7 00 03 00 00 00` ⇒ **3** = available|unavailable, at every
/// trainerType except 1).
///
/// So the three configurations below separate the two candidate explanations for a Blacksmithing
/// window that shows only "Development Skills / Apprentice Blacksmith":
///   * at the client's own default mask **3** the 18 red recipes and their "Recipes" header are all
///     present — a client that drops them is wrong;
///   * only with bit 1 cleared (mask **5** = available|used, i.e. the user unchecking
///     "Unavailable" in the Filter dropdown) do all 18 vanish **together with their header**.
#[test]
fn listorder_trainer_tradeskill_filter_e2e() {
    let Some(wow) = Wow::load() else { return };
    let n = DANE_LINDGREN.len() as u32;

    // (a) the builder's own default (0x4d75d9) — nothing is suppressed, red recipes included.
    let (vis_default, list_default) = run_trainer_tradeskill(&wow, 3);
    println!("trainer(type2) mask=3   visible={vis_default} of {}", n + 2);
    assert_eq!(
        vis_default,
        n + 2,
        "at the client's DEFAULT state mask 3 every red recipe is visible — there is no \
         skill-keyed suppression at trainerType 2"
    );
    assert_eq!(list_default[0], (-1, 1), "TRADESKILL_SERVICE_STEP header");
    assert_eq!(list_default[1], (2020, 1), "Apprentice Blacksmith");
    assert_eq!(list_default[2], (-1, 2), "TRADESKILL_SERVICE_LEARN header");
    assert_eq!(
        list_default[3..].len(),
        n as usize - 1,
        "all 18 recipes render under the Recipes header"
    );

    // (b) "Unavailable" unchecked ⇒ mask 5 (available|used). Every recipe is state 1, so group 2's
    // only nonzero counter is counter[1] — disabled — and `hdr[+0x1c]` goes 0, taking the header.
    let (vis_filtered, list_filtered) = run_trainer_tradeskill(&wow, 5);
    println!("trainer(type2) mask=5   visible={vis_filtered} rows={list_filtered:?}");
    assert_eq!(
        list_filtered,
        vec![(-1, 1), (2020, 1)],
        "with `unavailable` filtered out, the 18 red recipes AND the Recipes header both go"
    );
    assert_eq!(vis_filtered, 2);
    assert!(
        !list_filtered.iter().any(|(sp, k)| *sp < 0 && *k == 2),
        "the Recipes header must not survive its group being emptied by the state filter"
    );

    // (c) `SetTrainerServiceTypeFilter("all", …)` ⇒ mask 7 (0x4da2a9 `mov ecx,0x7`). Same as (a):
    // "all" never sets bit 3, so it is not a superset of every state the client can produce.
    let (vis_all, _) = run_trainer_tradeskill(&wow, 7);
    assert_eq!(vis_all, n + 2, "mask 7 shows the same rows as mask 3 here");
}
