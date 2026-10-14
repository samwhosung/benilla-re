//! `CGUnit_C::UnitReaction` (`0x6061e0`) against the exe, run whole over a synthesized object
//! graph and the real `FactionTemplate.dbc` numbers for template 1577 (League of Arathor
//! Emissary) and 1 (Human player) and faction 509 (reputation index 53, base 0). The test has no
//! reimplementation: it pins the branch order, direction and gates the entry states, which a
//! function-level compare cannot express. For this pair the two branches differ, rank 3 against
//! comparator 4, so every case could fail.
//!
//! The environment, all read directly by the code under test:
//! - `[0xc0dd3c]`, `[0xc0dd40]`: the template table and its largest id (inclusive)
//! - `[0xc0dd50]`, `[0xc0dd54]`: the faction table and its largest id (inclusive)
//! - `0xb73290`: the 64 reputation slots of 16 bytes: faction id, wire flags, base, wire standing
//! - `[0xb73694]`, `[0xb73698]`: the forced reaction array and its count
//! - `[0xb41414]`: the object manager, whose `+0xc0` and `+0xc4` hold the active player's guid
//!
//! Nothing is stubbed: `0x468550`, `0x606170`, `0x605fc0`, `0x4d63a0`, `0x4d6370`, `0x4d5600`,
//! `0x4d61b0`, `0x4d6490` and `0x606640` run their real instructions.

mod common;

use benilla_re_difftest::{Emu, Wow};

const UNIT_REACTION: u64 = 0x6061e0;

// The globals the chain reads.
const FT_TABLE_PTR: u64 = 0x00c0_dd3c;
const FT_TABLE_MAX: u64 = 0x00c0_dd40;
const FACTION_TABLE_PTR: u64 = 0x00c0_dd50;
const FACTION_TABLE_MAX: u64 = 0x00c0_dd54;
const REP_SLOTS: u64 = 0x00b7_3290;
const FORCED_COUNT: u64 = 0x00b7_3694;
const FORCED_ARRAY: u64 = 0x00b7_3698;
const OBJ_MGR: u64 = 0x00b4_1414;

// The object field offsets.
const OBJ_DESC: u64 = 0x08; // -> descriptor array base ([0]/[1] GUID, [2] OBJECT_FIELD_TYPE)
const OBJ_UNIT_DESC: u64 = 0x110; // -> descriptor base + 0x18 (the CGUnit view)
const OBJ_PLAYER_DESC: u64 = 0xe68; // -> descriptor base + UNIT_END*4 (the CGPlayer view)
const UNIT_VIEW_BIAS: u64 = 0x18; // index*4 - 0x18 == the CGUnit-view byte offset
const PLAYER_VIEW_BIAS: u64 = 188 * 4; // UNIT_END (PLAYER_FLAGS is index 190 == +0x8 here)

// The unit view's byte offsets.
const U_CHARMEDBY: u64 = 0x10;
const U_CREATEDBY: u64 = 0x20;
const U_PERSUADED: u64 = 0x30;
const U_FACTIONTEMPLATE: u64 = 0x74;
const U_FLAGS: u64 = 0xa0;

const UNIT_FLAG_PLAYER_CONTROLLED: u32 = 0x8;
const TYPE_UNIT: u32 = 0x8; // OBJECT_FIELD_TYPE bit 3
const TYPE_PLAYER: u32 = 0x10; // bit 4 — 0x606170 requires it of the controller

/// The `FactionTemplate.dbc` rows 1577 and 1, in the client's offset order: id, faction, flags,
/// group, friend, enemy, enemies[4], friends[4].
const FT_1577: [u32; 14] = [1577, 509, 0x21, 0x2, 0x2, 0x4, 510, 0, 0, 0, 509, 0, 0, 0];
const FT_1: [u32; 14] = [1, 1, 0x48, 0x3, 0x2, 0xc, 0, 0, 0, 0, 0, 0, 0, 0];

const PLAYER_GUID_LO: u32 = 0x1234_5678;
const PLAYER_GUID_HI: u32 = 0x0000_0001;

const REP_INDEX_509: u32 = 53;

/// Two objects and every global the reaction chain reads.
struct World {
    emissary: u32,
    player: u32,
}

/// Lays out one object: the struct, its descriptor array and the three view pointers.
fn make_object(e: &mut Emu, guid_lo: u32, guid_hi: u32, type_mask: u32) -> u32 {
    let obj = e.alloc(0x1000);
    let desc = e.alloc(0x800);
    e.write_u32(desc, guid_lo);
    e.write_u32(desc + 4, guid_hi);
    e.write_u32(desc + 8, type_mask);
    e.write_u32(obj + OBJ_DESC, desc as u32);
    e.write_u32(obj + OBJ_UNIT_DESC, (desc + UNIT_VIEW_BIAS) as u32);
    e.write_u32(obj + OBJ_PLAYER_DESC, (desc + PLAYER_VIEW_BIAS) as u32);
    obj as u32
}

fn unit_field(e: &mut Emu, obj: u32, off: u64, v: u32) {
    let view = e.read_u32(obj as u64 + OBJ_UNIT_DESC) as u64;
    e.write_u32(view + off, v);
}

fn player_field(e: &mut Emu, obj: u32, off: u64, v: u32) {
    let view = e.read_u32(obj as u64 + OBJ_PLAYER_DESC) as u64;
    e.write_u32(view + off, v);
}

fn build(e: &mut Emu) -> World {
    // Template id to row; the largest id is inclusive (`cmp eax,[0xc0dd40]; jg`).
    let ft_table = e.alloc(0x2000);
    e.write_u32(FT_TABLE_PTR, ft_table as u32);
    e.write_u32(FT_TABLE_MAX, 1677);
    for row in [FT_1577, FT_1] {
        let va = e.alloc(56);
        for (i, w) in row.iter().enumerate() {
            e.write_u32(va + (i * 4) as u64, *w);
        }
        e.write_u32(ft_table + (row[0] as u64) * 4, va as u32);
    }

    // Faction id to row; only the id at +0 and the reputation index at +4 are read here.
    let f_table = e.alloc(0x1000);
    e.write_u32(FACTION_TABLE_PTR, f_table as u32);
    e.write_u32(FACTION_TABLE_MAX, 954);
    for (id, rep_index) in [(1u32, u32::MAX /* -1 */), (509, REP_INDEX_509)] {
        let va = e.alloc(148);
        e.write_u32(va, id);
        e.write_u32(va + 4, rep_index);
        e.write_u32(f_table + (id as u64) * 4, va as u32);
    }

    // The reputation slots: faction 509 at slot 53, base 0, standing 0, flags 0.
    for i in 0..64u64 {
        for d in 0..4u64 {
            e.write_u32(REP_SLOTS + i * 16 + d * 4, 0);
        }
    }
    e.write_u32(REP_SLOTS + (REP_INDEX_509 as u64) * 16, 509);

    // No forced reactions.
    e.write_u32(FORCED_COUNT, 0);
    e.write_u32(FORCED_ARRAY, 0);

    let emissary = make_object(e, 0xdead_0001, 0, TYPE_UNIT);
    unit_field(e, emissary, U_FACTIONTEMPLATE, 1577);
    unit_field(e, emissary, U_FLAGS, 0);

    let player = make_object(e, PLAYER_GUID_LO, PLAYER_GUID_HI, TYPE_UNIT | TYPE_PLAYER);
    unit_field(e, player, U_FACTIONTEMPLATE, 1);
    unit_field(e, player, U_FLAGS, UNIT_FLAG_PLAYER_CONTROLLED);
    player_field(e, player, 0x8, 0); // PLAYER_FLAGS

    // The object manager, whose guid at +0xc0 and +0xc4 is the active player's (`0x468550`).
    let mgr = e.alloc(0x200);
    e.write_u32(OBJ_MGR, mgr as u32);
    e.write_u32(mgr + 0xc0, PLAYER_GUID_LO);
    e.write_u32(mgr + 0xc4, PLAYER_GUID_HI);

    World { emissary, player }
}

fn set_slot(e: &mut Emu, slot: u32, base: i32, standing: i32, flags: u32) {
    let s = REP_SLOTS + (slot as u64) * 16;
    e.write_u32(s + 4, flags);
    e.write_u32(s + 8, base as u32);
    e.write_u32(s + 0xc, standing as u32);
}

fn reaction(e: &mut Emu, this: u32, other: u32) -> u32 {
    e.call(UNIT_REACTION, this, 0, &[other])
}

/// The two directions differ. The emissary toward the player takes the reputation branch and
/// returns rank 3, neutral, although the template comparator says 4 (the emissary's friend mask
/// 0x2 meets the player's group 0x3). The player toward the emissary returns `AtWar ? 1 : 4`, so 4.
#[test]
fn unit_reaction_league_of_arathor_emissary() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let w = build(&mut e);

    // Control: the comparator on the same two rows says friendly.
    let ft_table = e.read_u32(FT_TABLE_PTR) as u64;
    let row_a = e.read_u32(ft_table + 1577 * 4);
    let row_b = e.read_u32(ft_table + 4);
    assert_eq!(
        e.call(0x606640, row_a, row_b, &[]),
        4,
        "the comparator on 1577 and 1 must say friendly, or the pair tells the branches apart"
    );

    // The colour direction, which `0x7cbaa0` and `0x605960` ask.
    assert_eq!(
        reaction(&mut e, w.emissary, w.player),
        3,
        "emissary toward player at standing 0 is the reputation rank, neutral, not the comparator's"
    );

    // The gate direction, which the attack, assist and interact checks ask.
    assert_eq!(
        reaction(&mut e, w.player, w.emissary),
        4,
        "player toward emissary is friendly through the at-war leg, not the rank"
    );
}

/// The rank drives the unit-to-player direction: moving the standing across every band boundary
/// walks the ladder, while the comparator stays at 4.
///
/// Two values are read apart: `0x4d63a0`, the raw 0 to 7 ladder, and `0x6061e0`, the ladder after
/// the `min(rank, 6)` at `0x606439`, so the clamp shows.
#[test]
fn unit_reaction_tracks_the_reputation_ladder() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let w = build(&mut e);

    // (standing, base, raw rank from `0x4d63a0`), both sides of every threshold.
    let cases: &[(i32, i32, u32)] = &[
        (-42_000, 0, 0),
        (-6_001, 0, 0),
        (-6_000, 0, 1),
        (-3_001, 0, 1),
        (-3_000, 0, 2),
        (-1, 0, 2),
        (0, 0, 3),
        (2_999, 0, 3),
        (3_000, 0, 4),
        (8_999, 0, 4),
        (9_000, 0, 5),
        (20_999, 0, 5),
        (21_000, 0, 6),
        (41_999, 0, 6),
        (42_000, 0, 7),
        // The base and the standing add (`0x4d6383`, `0x4d6389`): equal totals split differently.
        (0, 3_000, 4),
        (1_500, 1_500, 4),
        (-1_500, 4_500, 4),
        (2_999, 1, 4),
        (-100_000, 58_000, 0),
    ];
    for &(standing, base, rank) in cases {
        set_slot(&mut e, REP_INDEX_509, base, standing, 0);
        assert_eq!(
            e.call(0x4d63a0, 509, 0, &[]),
            rank,
            "0x4d63a0(509) at standing {standing} + base {base}"
        );
        assert_eq!(
            reaction(&mut e, w.emissary, w.player),
            rank.min(6),
            "emissary->UnitReaction(player) at standing {standing} + base {base}"
        );
    }

    // The clamp is a ceiling: Exalted and Revered read alike, and 7 is not reached on this path,
    // since the `+1` bump at `0x6064aa` needs a rank under 5 and cannot lift a 6.
    for standing in [21_000, 42_000, 1_000_000] {
        set_slot(&mut e, REP_INDEX_509, 0, standing, 0);
        assert_eq!(reaction(&mut e, w.emissary, w.player), 6, "ceiling");
    }
}

/// The gate of the reputation branch. Each case removes one precondition and the answer falls
/// through to the comparator's 4, with the standing at Revered (rank 6) so the two paths differ.
#[test]
fn unit_reaction_reputation_branch_is_gated() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let w = build(&mut e);
    set_slot(&mut e, REP_INDEX_509, 0, 21_000, 0);

    // With every gate satisfied the answer is the rank, not 4.
    assert_eq!(
        reaction(&mut e, w.emissary, w.player),
        6,
        "baseline revered"
    );

    // The target is not player controlled: the comparator.
    unit_field(&mut e, w.player, U_FLAGS, 0);
    assert_eq!(
        reaction(&mut e, w.emissary, w.player),
        4,
        "a target without the player controlled flag takes the comparator"
    );
    unit_field(&mut e, w.player, U_FLAGS, UNIT_FLAG_PLAYER_CONTROLLED);
    assert_eq!(reaction(&mut e, w.emissary, w.player), 6, "restored");

    // The target is player controlled by another player: the comparator. The controlling
    // player's guid must be the active player's (`0x6065ab`).
    let other = make_object(&mut e, 0x0bad_0bad, 7, TYPE_UNIT | TYPE_PLAYER);
    unit_field(&mut e, other, U_FACTIONTEMPLATE, 1);
    unit_field(&mut e, other, U_FLAGS, UNIT_FLAG_PLAYER_CONTROLLED);
    player_field(&mut e, other, 0x8, 0);
    assert_eq!(
        reaction(&mut e, w.emissary, other),
        4,
        "another player is not the active player's reputation and takes the comparator"
    );

    // The faction has no reputation slot (index below 0): the comparator.
    let f_table = e.read_u32(FACTION_TABLE_PTR) as u64;
    let row509 = e.read_u32(f_table + 509 * 4) as u64;
    e.write_u32(row509 + 4, u32::MAX); // -1
    assert_eq!(
        reaction(&mut e, w.emissary, w.player),
        4,
        "a reputation index of -1 takes the comparator"
    );
    e.write_u32(row509 + 4, REP_INDEX_509);
    assert_eq!(reaction(&mut e, w.emissary, w.player), 6, "restored");

    // A forced reaction replaces the rank unchanged (`0x4d6490`, `0x6065ef`).
    let forced = e.alloc(0x40);
    e.write_u32(forced, 509);
    e.write_u32(forced + 4, 2);
    e.write_u32(FORCED_ARRAY, forced as u32);
    e.write_u32(FORCED_COUNT, 1);
    assert_eq!(
        reaction(&mut e, w.emissary, w.player),
        2,
        "a forced reaction overrides the rank"
    );
    e.write_u32(FORCED_COUNT, 0);

    // The contested guard: template flag 0x1000 with player flag 0x100. The flags of row 1577 are
    // 0x21, so both halves are set, and only both fire it.
    let ft_table = e.read_u32(FT_TABLE_PTR) as u64;
    let row1577 = e.read_u32(ft_table + 1577 * 4) as u64;
    e.write_u32(row1577 + 8, 0x21 | 0x1000);
    assert_eq!(
        reaction(&mut e, w.emissary, w.player),
        6,
        "the template flag alone does not fire"
    );
    player_field(&mut e, w.player, 0x8, 0x100);
    assert_eq!(
        reaction(&mut e, w.emissary, w.player),
        1,
        "the template flag with the player flag is hostile"
    );
    e.write_u32(row1577 + 8, 0x21);
    player_field(&mut e, w.player, 0x8, 0);
    assert_eq!(reaction(&mut e, w.emissary, w.player), 6, "restored");
}

/// The player-to-NPC direction is `AtWar ? 1 : 4`, not the rank: Exalted still reads 4, and
/// at war reads 1.
#[test]
fn unit_reaction_player_to_npc_is_atwar_not_rank() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let w = build(&mut e);

    for &(standing, flags, want) in &[
        (0i32, 0u32, 4u32),
        (42_000, 0, 4),   // Exalted still reads 4
        (-42_000, 0, 4),  // Hated still reads 4
        (42_000, 0x2, 1), // the at-war bit, flag bit 1 (`0x4d61b0`)
        (0, 0x2, 1),
        (0, 0x1, 4), // the visible bit is not the at-war bit
    ] {
        set_slot(&mut e, REP_INDEX_509, 0, standing, flags);
        assert_eq!(
            reaction(&mut e, w.player, w.emissary),
            want,
            "player->emissary at standing {standing}, slot flags {flags:#x}"
        );
    }
}

/// The `+1` bump for a persuaded target and the `min(rank, 6)` clamp before it.
#[test]
fn unit_reaction_persuaded_bump_and_clamp() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let w = build(&mut e);

    // The emissary's faction has a reputation slot (509 does). A persuaded target is one whose
    // persuaded-by field holds the other unit's guid.
    let bump = |e: &mut Emu, on: bool| {
        unit_field(
            e,
            w.emissary,
            U_PERSUADED,
            if on { PLAYER_GUID_LO } else { 0 },
        );
        unit_field(
            e,
            w.emissary,
            U_PERSUADED + 4,
            if on { PLAYER_GUID_HI } else { 0 },
        );
    };

    for &(standing, want_plain, want_bumped) in &[
        (-42_000i32, 0u32, 0), // rank 0: a rank at or under 1 blocks the bump
        (-6_000, 1, 1),        // rank 1: blocked
        (-3_000, 2, 3),        // rank 2 -> 3
        (0, 3, 4),             // rank 3 -> 4
        (3_000, 4, 5),         // rank 4 -> 5
        (9_000, 5, 5),         // rank 5: a rank of 5 or more blocks the bump
        (42_000, 6, 6),        // rank 7 is clamped to 6 first, then blocked
    ] {
        set_slot(&mut e, REP_INDEX_509, 0, standing, 0);
        bump(&mut e, false);
        assert_eq!(
            reaction(&mut e, w.emissary, w.player),
            want_plain,
            "standing {standing} unbumped"
        );
        bump(&mut e, true);
        assert_eq!(
            reaction(&mut e, w.emissary, w.player),
            want_bumped,
            "standing {standing} bumped"
        );
    }
    bump(&mut e, false);

    // The clamp applies without a bump: Exalted (7) reads 6.
    set_slot(&mut e, REP_INDEX_509, 0, 42_000, 0);
    unit_field(&mut e, w.emissary, U_CHARMEDBY, 0);
    unit_field(&mut e, w.emissary, U_CREATEDBY, 0);
    assert_eq!(
        reaction(&mut e, w.emissary, w.player),
        6,
        "`0x606439` clamps the rank to 6 before the bump"
    );
}

/// A unit toward itself returns 4 before anything else (`0x606207`), needing no faction data.
#[test]
fn unit_reaction_self_is_friendly() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let w = build(&mut e);
    assert_eq!(reaction(&mut e, w.emissary, w.emissary), 4);
    assert_eq!(reaction(&mut e, w.player, w.player), 4);
}
