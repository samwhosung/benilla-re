//! `FactionTemplateReaction` (`0x606640`) against the exe, run whole on synthesized rows. The
//! result is 1, 3 or 4.
//!
//! The six conditions are made independent by construction, so all 2^6 combinations run and fix
//! the precedence. A second test probes the list mechanics: a match at each of the four slots,
//! the 0 terminator ending a scan, the index-4 bound and the side each list matches on. A fuzz
//! over a tiny id and mask universe covers their interactions.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::faction::{
    faction_template_reaction, FactionTemplate, FACTION_TEMPLATE_SIZE,
};
use common::next;

const FACTION: u64 = 0x606640;

fn ft_bytes(ft: &FactionTemplate) -> Vec<u8> {
    unsafe {
        std::slice::from_raw_parts(
            (ft as *const FactionTemplate).cast::<u8>(),
            FACTION_TEMPLATE_SIZE,
        )
    }
    .to_vec()
}

/// Runs one pair of rows through the exe and the transcription and compares the reaction.
fn check(e: &mut Emu, a_va: u64, b_va: u64, a: &FactionTemplate, b: &FactionTemplate, ctx: &str) {
    e.write(a_va, &ft_bytes(a));
    e.write(b_va, &ft_bytes(b));
    let bin = e.call(FACTION, a_va as u32, b_va as u32, &[]);
    let ours = faction_template_reaction(a, b);
    assert_eq!(bin, ours, "{ctx}: reaction (bin {bin} vs ours {ours})");
}

// Distinct nonzero faction ids for the structured cases; 0 would end a list scan.
const FA: u32 = 0xAAAA;
const FB: u32 = 0xBBBB;

#[test]
fn faction_template_reaction_truth_table() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let a_va = e.alloc(FACTION_TEMPLATE_SIZE as u64);
    let b_va = e.alloc(FACTION_TEMPLATE_SIZE as u64);

    // All 2^6 combinations of the six conditions, made independent by disjoint mask bits:
    //   c1 = A.enemy_mask & B.our_mask     (B.our_mask bit0)
    //   c2 = B.faction ∈ A.enemies
    //   c3 = A.friend_mask & B.our_mask    (B.our_mask bit0)
    //   c4 = B.faction ∈ A.friends
    //   c5 = B.friend_mask & A.our_mask    (A.our_mask bit1)
    //   c6 = A.faction ∈ B.friends
    for bits in 0u32..64 {
        let c = |i: u32| bits & (1 << i) != 0;
        let mut a = FactionTemplate {
            faction: FA,
            our_mask: 0x2,
            enemy_mask: if c(0) { 0x1 } else { 0 },
            friend_mask: if c(2) { 0x1 } else { 0 },
            ..Default::default()
        };
        a.enemies[0] = if c(1) { FB } else { 0 };
        a.friends[0] = if c(3) { FB } else { 0 };
        let mut b = FactionTemplate {
            faction: FB,
            our_mask: 0x1,
            friend_mask: if c(4) { 0x2 } else { 0 },
            ..Default::default()
        };
        b.friends[0] = if c(5) { FA } else { 0 };
        check(
            &mut e,
            a_va,
            b_va,
            &a,
            &b,
            &format!("truth bits {bits:#04x}"),
        );
    }
}

#[test]
fn faction_template_reaction_list_mechanics() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let a_va = e.alloc(FACTION_TEMPLATE_SIZE as u64);
    let b_va = e.alloc(FACTION_TEMPLATE_SIZE as u64);
    let b = FactionTemplate {
        faction: FB,
        ..Default::default()
    };

    // An enemy match at each of the four slots, with earlier slots holding other nonzero ids so
    // the scan reaches slot i: hostile.
    for i in 0..4 {
        let mut a = FactionTemplate {
            faction: FA,
            ..Default::default()
        };
        for (j, slot) in a.enemies.iter_mut().enumerate() {
            *slot = if j < i {
                0x10 + j as u32 // nonzero and not FB
            } else if j == i {
                FB
            } else {
                0
            };
        }
        check(&mut e, a_va, b_va, &a, &b, &format!("enemy slot {i}"));
    }

    // A 0 entry ends the scan: an id after it is never seen.
    for &(e0, e1, e2) in &[(0u32, FB, 0u32), (0x11, 0, FB)] {
        let mut a = FactionTemplate {
            faction: FA,
            ..Default::default()
        };
        a.enemies = [e0, e1, e2, 0];
        check(&mut e, a_va, b_va, &a, &b, "enemy 0-terminator"); // neutral
    }

    // The index-4 bound: four nonmatching enemies and friends[0] = FB. The enemy scan stops at
    // index 3, so friends[0] is not read as an enemy, and it makes the pair friendly.
    let mut a = FactionTemplate {
        faction: FA,
        enemies: [0x11, 0x12, 0x13, 0x14],
        ..Default::default()
    };
    a.friends[0] = FB;
    check(&mut e, a_va, b_va, &a, &b, "enemy bound vs friends[0]"); // friendly
    a.friends[0] = 0;
    // With no friend either, four nonmatching enemies are neutral.
    check(&mut e, a_va, b_va, &a, &b, "enemy full non-match"); // neutral

    // The key side: A's lists match on B's faction and B's friend list on A's faction. A list
    // holding the other side's faction does not match.
    let mut a = FactionTemplate {
        faction: FA,
        ..Default::default()
    };
    a.friends[0] = FA; // A's own faction, not B's
    check(&mut e, a_va, b_va, &a, &b, "A.friends wrong key"); // neutral
    let a = FactionTemplate {
        faction: FA,
        ..Default::default()
    };
    let mut b2 = FactionTemplate {
        faction: FB,
        ..Default::default()
    };
    b2.friends[0] = FB; // B's own faction, not A's
    check(&mut e, a_va, b_va, &a, &b2, "B.friends wrong key"); // neutral
    b2.friends[0] = FA; // A's faction
    check(&mut e, a_va, b_va, &a, &b2, "B.friends right key"); // friendly
}

#[test]
fn faction_template_reaction_fuzz() {
    let Some(wow) = Wow::load() else { return };
    let mut e = Emu::new(&wow);
    let a_va = e.alloc(FACTION_TEMPLATE_SIZE as u64);
    let b_va = e.alloc(FACTION_TEMPLATE_SIZE as u64);

    // A tiny universe of 4 faction ids (0 never matches, since it ends a scan) and 4 mask values,
    // so list matches and mask overlaps are dense.
    let mut s = 0xfac0_0001u32;
    let pick = |s: &mut u32, m: u32| (next(s) >> 8) % m;
    let rand_ft = |s: &mut u32| {
        let mut ft = FactionTemplate {
            faction: pick(s, 4),
            our_mask: pick(s, 4),
            friend_mask: pick(s, 4),
            enemy_mask: pick(s, 4),
            ..Default::default()
        };
        for slot in ft.enemies.iter_mut() {
            *slot = pick(s, 4);
        }
        for slot in ft.friends.iter_mut() {
            *slot = pick(s, 4);
        }
        ft
    };
    for i in 0..4000 {
        let a = rand_ft(&mut s);
        let b = rand_ft(&mut s);
        check(&mut e, a_va, b_va, &a, &b, &format!("fuzz {i}"));
    }
}
