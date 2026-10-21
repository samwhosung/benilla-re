//! The outbid list of the auction house against the exe (`systems/ui/auction.md#outbid-list`):
//! `AddOutbidAuction` (`0x4cde50`), `RemoveOutbidAuction` (`0x4cdf30`) and `RemoveOutbidHouse`
//! (`0x4cdf90`), each run whole from `ecx` and `edx`, and the slice of `CGAuctionHouse::
//! InitializeGame` that builds the list (`0x4cc1c9` to `0x4cc234`).
//!
//! `SMemAlloc` (`0x6462e0`) and `SMemReAlloc` (`0x646320`) are stubbed with a bump arena; the
//! array's quantum (`0x4d07d0`), rounding (`0x4d0810`) and capacity (`0x4d0830`) functions run for
//! real. After every call the test reads back the slot count, the count, the array's
//! allocated size and quantum cap, the pointer array and the pair each slot's block holds, and
//! compares them with `outbid_list::OutbidList`. Removals leave every block where it was and
//! doubling keeps the old blocks. The guard at `0x4cdea0` skips the array's growth when the
//! doubled slot count wraps, which a list that starts at eight slots never does. The guard at
//! `0x4cdea8` skips it when the array already holds the doubled count: not from a first
//! start-up, where the array holds exactly the slot count, but after the teardown and a later
//! start-up with the quantum cap set, where the array holds 64 from the start.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::outbid_list::{OutbidList, Pair};
use std::collections::HashSet;

const ADD: u64 = 0x4cde50;
const REMOVE_AUCTION: u64 = 0x4cdf30;
const REMOVE_HOUSE: u64 = 0x4cdf90;
const START_UP: (u64, u64) = (0x4cc1c9, 0x4cc236);
const TEARDOWN: (u64, u64) = (0x4cc3bb, 0x4cc419);
const SMEM_FREE: u64 = 0x646430;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const ALLOC: u64 = 0xb7258c;
const SLOTS: u64 = 0xb72590;
const POINTERS: u64 = 0xb72594;
const CHUNK: u64 = 0xb72598;
const COUNT: u64 = 0xb72634;
const ARENA: u64 = 0x1_0000;

/// An emulator whose list is what the exe's own start-up slice builds.
fn started() -> Option<Emu<'static>> {
    let wow = Wow::load()?;
    let mut e = Emu::new(&wow);
    let arena = e.alloc(ARENA);
    e.stub_alloc_sized(SMEM_ALLOC, arena, arena + ARENA, 0, 4);
    e.stub_realloc_sized(SMEM_REALLOC, 5);
    e.stub(SMEM_FREE, 1, 4);
    let stack = e.alloc(0x1000);
    e.set_reg(RegisterX86::ESP, stack + 0xf00);
    e.set_reg(RegisterX86::EBP, stack + 0xe00);
    // The start-up function keeps 0 in `edi` for its stores.
    e.set_reg(RegisterX86::EDI, 0);
    e.run(START_UP.0, START_UP.1);
    Some(e)
}

/// The exe's teardown slice, then its start-up slice again: the list a later world entry builds.
fn restart(e: &mut Emu) {
    e.set_reg(RegisterX86::EDI, 0);
    e.run(TEARDOWN.0, TEARDOWN.1);
    e.set_reg(RegisterX86::EDI, 0);
    e.run(START_UP.0, START_UP.1);
}

/// The pointer array: the block address of every slot.
fn pointers(e: &Emu) -> Vec<u32> {
    let data = u64::from(e.read_u32(POINTERS));
    (0..u64::from(e.read_u32(SLOTS)))
        .map(|i| e.read_u32(data + 4 * i))
        .collect()
}

/// The list as the globals and the blocks hold it.
fn state(e: &Emu) -> OutbidList {
    let slots = pointers(e)
        .iter()
        .map(|&p| (e.read_u32(u64::from(p)), e.read_u32(u64::from(p) + 4)))
        .collect();
    OutbidList {
        slots,
        count: e.read_u32(COUNT),
        alloc: e.read_u32(ALLOC),
        chunk: e.read_u32(CHUNK),
    }
}

/// The pairs in use.
fn used(e: &Emu) -> Vec<Pair> {
    let list = state(e);
    list.slots[..list.count as usize].to_vec()
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Add(u32, u32),
    RemoveAuction(u32),
    RemoveHouse(u32),
}

/// What one op did to the model: the slot a removal took out, or that it found none.
#[derive(Debug, PartialEq)]
enum Effect {
    Added { doubled: bool },
    Removed(usize),
    Missed,
}

/// Runs `op` in the exe and in the model and requires the same state. The block addresses must
/// not change except for new blocks at the end, and every block must be its own.
fn apply(e: &mut Emu, model: &mut OutbidList, op: Op) -> Effect {
    let before = pointers(e);
    let slots = model.slots.len();
    let effect = match op {
        Op::Add(house, auction) => {
            e.call(ADD, house, auction, &[]);
            model.add(house, auction);
            Effect::Added {
                doubled: model.slots.len() != slots,
            }
        }
        Op::RemoveAuction(auction) => {
            e.call(REMOVE_AUCTION, auction, 0, &[]);
            model
                .remove_auction(auction)
                .map_or(Effect::Missed, Effect::Removed)
        }
        Op::RemoveHouse(house) => {
            e.call(REMOVE_HOUSE, house, 0, &[]);
            model
                .remove_house(house)
                .map_or(Effect::Missed, Effect::Removed)
        }
    };
    assert_eq!(&state(e), model, "after {op:?}");
    let after = pointers(e);
    assert_eq!(after[..before.len()], before[..], "blocks moved by {op:?}");
    assert_eq!(
        after.iter().collect::<HashSet<_>>().len(),
        after.len(),
        "two slots share a block after {op:?}"
    );
    assert!(after.iter().all(|&p| p != 0));
    effect
}

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

#[test]
fn outbid_list_start_up_difftest() {
    let Some(mut e) = started() else { return };
    assert_eq!(state(&e), OutbidList::new());
    assert_eq!(pointers(&e).len(), 8);
    // A list that has not doubled to 64 slots leaves the cap 0, and a later start-up is the first's.
    restart(&mut e);
    assert_eq!(state(&e), OutbidList::new());
    // Doubled to 64 slots, the cap is 64 and survives the teardown: the array is sized to 64.
    let mut model = OutbidList::new();
    let mut n = 0;
    while model.slots.len() < 64 {
        n += 1;
        apply(&mut e, &mut model, Op::Add(1, 100 + n));
    }
    assert_eq!(e.read_u32(CHUNK), 64);
    restart(&mut e);
    assert_eq!(state(&e), OutbidList::started_with_cap(64));
    assert_eq!(e.read_u32(ALLOC), 64);
    assert_eq!(e.read_u32(SLOTS), 8);
    assert_eq!(e.read_u32(COUNT), 0);
}

/// After a later start-up with the cap set the array holds 64 entries over 8 slots: the doublings
/// to 16, 32 and 64 find it holding the count and skip its growth (`0x4cdea8`), and the doubling
/// to 128 raises it to 128. The slots and the lost pair behave as at a first start-up.
#[test]
fn outbid_list_later_start_up_difftest() {
    let Some(mut e) = started() else { return };
    let mut model = OutbidList::new();
    let mut n = 0;
    while model.slots.len() < 64 {
        n += 1;
        apply(&mut e, &mut model, Op::Add(1, 100 + n));
    }
    restart(&mut e);
    let mut model = OutbidList::started_with_cap(64);
    assert_eq!(state(&e), model);
    let mut n = 0u32;
    for slots in [8u32, 16, 32, 64] {
        while n < slots {
            n += 1;
            apply(&mut e, &mut model, Op::Add(1, 1000 + n));
        }
        let effect = apply(&mut e, &mut model, Op::Add(7, 5000 + slots));
        assert_eq!(effect, Effect::Added { doubled: true });
        assert_eq!(e.read_u32(SLOTS), 2 * slots);
        // The array is untouched until the doubled count passes 64.
        assert_eq!(e.read_u32(ALLOC), (2 * slots).max(64));
        assert_eq!(e.read_u32(COUNT), slots);
        n = slots;
        apply(&mut e, &mut model, Op::Add(8, 6000 + slots));
        n += 1;
    }
    assert_eq!(e.read_u32(ALLOC), 128);
    assert_eq!(e.read_u32(CHUNK), 64);
}

/// The ninth append finds the list full: the slots double to 16, the pair lands in slot 8 and
/// the count stays 8, so the next append takes the same slot. The same at 16, 32 and 64 slots,
/// the last of which is the first to find the array's quantum cap already set.
#[test]
fn outbid_list_doubling_difftest() {
    let Some(mut e) = started() else { return };
    let mut model = OutbidList::new();
    let mut n = 0u32;
    for slots in [8u32, 16, 32, 64] {
        while n < slots {
            n += 1;
            let effect = apply(&mut e, &mut model, Op::Add(1, 1000 + n));
            assert_eq!(effect, Effect::Added { doubled: false });
        }
        assert_eq!(e.read_u32(COUNT), slots);
        assert_eq!(e.read_u32(SLOTS), slots);
        // The append that finds the list full.
        let lost = (7, 5000 + slots);
        let effect = apply(&mut e, &mut model, Op::Add(lost.0, lost.1));
        assert_eq!(effect, Effect::Added { doubled: true });
        assert_eq!(e.read_u32(SLOTS), 2 * slots);
        assert_eq!(e.read_u32(ALLOC), 2 * slots);
        assert_eq!(e.read_u32(COUNT), slots, "the doubling counts the pair");
        let list = state(&e);
        assert_eq!(
            list.slots[slots as usize], lost,
            "the pair is in slot {slots}"
        );
        assert_eq!(used(&e).len(), slots as usize);
        assert!(!used(&e).contains(&lost), "the pair is not in the list");
        // The next append takes slot `slots` and the pair is gone.
        let next = (8, 6000 + slots);
        apply(&mut e, &mut model, Op::Add(next.0, next.1));
        assert_eq!(e.read_u32(COUNT), slots + 1);
        let list = state(&e);
        assert_eq!(list.slots[slots as usize], next);
        assert!(!list.slots.contains(&lost), "the pair is overwritten");
        n = slots + 1;
    }
    // The quantum cap is set by the doubling to 64 slots, and the array holds the doubled count.
    assert_eq!(e.read_u32(CHUNK), 64);
    assert_eq!(e.read_u32(ALLOC), 128);
    // A removal between the doubling and the next append: the lost pair stays past the count and
    // the next append takes the slot the removal freed.
    let mut e = started().unwrap();
    let mut model = OutbidList::new();
    for n in 0..8 {
        apply(&mut e, &mut model, Op::Add(1, 10 + n));
    }
    apply(&mut e, &mut model, Op::Add(2, 99));
    apply(&mut e, &mut model, Op::RemoveAuction(10));
    assert_eq!(e.read_u32(COUNT), 7);
    apply(&mut e, &mut model, Op::Add(2, 100));
    assert_eq!(e.read_u32(COUNT), 8);
    assert_eq!(
        used(&e),
        [
            (1, 11),
            (1, 12),
            (1, 13),
            (1, 14),
            (1, 15),
            (1, 16),
            (1, 17),
            (2, 100)
        ]
    );
}

/// `RemoveOutbidHouse` removes one pair per call: the first whose house matches, wherever it is,
/// and the later pairs move down a slot.
#[test]
fn outbid_list_remove_house_difftest() {
    let Some(mut e) = started() else { return };
    let mut model = OutbidList::new();
    for (house, auction) in [(1, 10), (2, 11), (1, 12), (2, 13), (1, 14)] {
        apply(&mut e, &mut model, Op::Add(house, auction));
    }
    let effect = apply(&mut e, &mut model, Op::RemoveHouse(1));
    assert_eq!(effect, Effect::Removed(0));
    assert_eq!(used(&e), [(2, 11), (1, 12), (2, 13), (1, 14)]);
    // The block of the last slot keeps the pair that moved down.
    assert_eq!(state(&e).slots[4], (1, 14));
    let effect = apply(&mut e, &mut model, Op::RemoveHouse(1));
    assert_eq!(effect, Effect::Removed(1));
    assert_eq!(used(&e), [(2, 11), (2, 13), (1, 14)]);
    let effect = apply(&mut e, &mut model, Op::RemoveHouse(1));
    assert_eq!(effect, Effect::Removed(2));
    assert_eq!(used(&e), [(2, 11), (2, 13)]);
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveHouse(1)),
        Effect::Missed
    );
    assert_eq!(used(&e), [(2, 11), (2, 13)]);
    apply(&mut e, &mut model, Op::RemoveHouse(2));
    assert_eq!(used(&e), [(2, 13)]);
    apply(&mut e, &mut model, Op::RemoveHouse(2));
    assert_eq!(e.read_u32(COUNT), 0);
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveHouse(2)),
        Effect::Missed
    );
    assert_eq!(e.read_u32(COUNT), 0);

    // A full list, a match in slot 0, in a middle slot and in the last slot; a pair removed
    // earlier, left in a slot past the count, is not matched.
    for at in [0usize, 3, 7] {
        let mut e = started().unwrap();
        let mut model = OutbidList::new();
        for i in 0..8usize {
            let house = if i == at { 9 } else { 1 };
            apply(&mut e, &mut model, Op::Add(house, 20 + i as u32));
        }
        apply(&mut e, &mut model, Op::RemoveHouse(9));
        assert_eq!(e.read_u32(COUNT), 7);
        let want: Vec<Pair> = (0..8usize)
            .filter(|&i| i != at)
            .map(|i| (1, 20 + i as u32))
            .collect();
        assert_eq!(used(&e), want, "match in slot {at}");
        assert_eq!(
            apply(&mut e, &mut model, Op::RemoveHouse(9)),
            Effect::Missed
        );
        assert_eq!(used(&e), want);
    }
}

/// `RemoveOutbidAuction` removes the first pair with the auction id.
#[test]
fn outbid_list_remove_auction_difftest() {
    let Some(mut e) = started() else { return };
    let mut model = OutbidList::new();
    for (house, auction) in [(1, 10), (2, 11), (1, 12), (2, 11), (3, 13)] {
        apply(&mut e, &mut model, Op::Add(house, auction));
    }
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveAuction(11)),
        Effect::Removed(1)
    );
    assert_eq!(used(&e), [(1, 10), (1, 12), (2, 11), (3, 13)]);
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveAuction(11)),
        Effect::Removed(2)
    );
    assert_eq!(used(&e), [(1, 10), (1, 12), (3, 13)]);
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveAuction(11)),
        Effect::Missed
    );
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveAuction(13)),
        Effect::Removed(2)
    );
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveAuction(10)),
        Effect::Removed(0)
    );
    assert_eq!(used(&e), [(1, 12)]);
    // The house id is not matched.
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveAuction(1)),
        Effect::Missed
    );
    apply(&mut e, &mut model, Op::RemoveAuction(12));
    assert_eq!(e.read_u32(COUNT), 0);
    assert_eq!(
        apply(&mut e, &mut model, Op::RemoveAuction(12)),
        Effect::Missed
    );
}

/// Random lists: grown to as many as 140 pairs, then added to and removed from, with house and
/// auction ids drawn so that removals hit, miss and meet equal pairs and the edge values 0 and
/// 0xffffffff; the state is compared after every call.
#[test]
fn outbid_list_sequence_difftest() {
    let Some(mut e) = started() else { return };
    let houses = [0u32, 1, 2, 3, 0xffff_ffff];
    let mut doublings = [0u32; 5];
    let (mut first, mut middle, mut last, mut missed, mut calls) = (0u32, 0u32, 0u32, 0u32, 0u32);
    let mut stale_hits = 0u32;
    let snapshot = e.snapshot();
    for seed in 0..40u32 {
        let mut s = 0x0b1d_0000 + seed * 7919;
        let mut model = OutbidList::new();
        // Every fourth list is filled past 128 pairs, so that it doubles to 256 slots.
        let fill = match seed % 4 {
            0 => 129 + lcg(&mut s) % 12,
            _ => 1 + lcg(&mut s) % 140,
        };
        let rest = 200 + lcg(&mut s) % 200;
        for step in 0..fill + rest {
            let house = houses[(lcg(&mut s) % 5) as usize];
            let auction = match lcg(&mut s) % 8 {
                0 => 0,
                1 => 0xffff_ffff,
                _ => 1 + lcg(&mut s) % 50,
            };
            let op = if step < fill {
                Op::Add(house, auction)
            } else {
                match lcg(&mut s) % 20 {
                    0..=6 => Op::Add(house, auction),
                    7..=13 => Op::RemoveHouse(house),
                    _ => Op::RemoveAuction(auction),
                }
            };
            let used_before = model.count as usize;
            let effect = apply(&mut e, &mut model, op);
            calls += 1;
            match effect {
                Effect::Added { doubled: true } => {
                    doublings[(model.slots.len() / 16).trailing_zeros() as usize] += 1;
                }
                Effect::Added { .. } => {}
                Effect::Removed(0) => first += 1,
                Effect::Removed(at) if at + 1 == used_before => last += 1,
                Effect::Removed(_) => middle += 1,
                Effect::Missed => {
                    missed += 1;
                    // A pair past the count that the removal's key would match.
                    let key = match op {
                        Op::RemoveHouse(h) => (h, true),
                        Op::RemoveAuction(a) => (a, false),
                        Op::Add(..) => unreachable!(),
                    };
                    if model.slots[model.count as usize..].iter().any(|p| {
                        if key.1 {
                            p.0 == key.0
                        } else {
                            p.1 == key.0
                        }
                    }) {
                        stale_hits += 1;
                    }
                }
            }
        }
        e.restore(&snapshot);
    }
    eprintln!(
        "outbid_list_sequence_difftest: {calls} calls bit-exact; doublings to 16, 32, 64, 128, 256 \
         slots: {doublings:?}; removals at the first, a middle and the last slot: {first}, {middle}, \
         {last}; misses {missed}, of which {stale_hits} had a matching stale pair"
    );
    assert!(doublings.iter().all(|&n| n > 0));
    assert!(first > 0 && middle > 0 && last > 0 && missed > 0 && stale_hits > 0);
}
