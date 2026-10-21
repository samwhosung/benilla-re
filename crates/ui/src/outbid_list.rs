//! The auction house's outbid list: `AddOutbidAuction` (`0x4cde50`), `RemoveOutbidAuction`
//! (`0x4cdf30`) and `RemoveOutbidHouse` (`0x4cdf90`), over the state start-up leaves
//! (`systems/ui/auction.md#outbid-list`).
//!
//! The list is a pointer array `0xb72594` with `0xb72590` slots, each pointing at an 8-byte block
//! holding a {house id, auction id} pair, and a count `0xb72634` of the slots in use. The array
//! has its own allocated size `0xb7258c` and grow quantum cap `0xb72598`. A removal copies the
//! later pairs down a slot, so the blocks never move; the model holds the pairs by slot.

/// A {house id, auction id} pair, as a slot's 8-byte block holds it.
pub type Pair = (u32, u32);

/// The grow quantum `0x4d07d0` returns for a count: 64, and the cap `chunk` set to 64, from 64 up;
/// below it the largest power of two not above the count, and at least 1.
fn quantum(count: u32, chunk: &mut u32) -> u32 {
    if count >= 0x40 {
        *chunk = 0x40;
        return 0x40;
    }
    let mut n = count;
    while n & n.wrapping_sub(1) != 0 {
        n &= n.wrapping_sub(1);
    }
    n.max(1)
}

/// `0x4d0810`: `size` rounded up to a multiple of `quantum`, in 32 bits.
fn round_up(size: u32, quantum: u32) -> u32 {
    match size % quantum {
        0 => size,
        rem => size.wrapping_add(quantum - rem),
    }
}

/// The outbid list as the globals hold it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutbidList {
    /// The pair in every slot of the pointer array: `slots.len()` is `0xb72590`. A slot from
    /// `count` up holds a pair already removed, or one an append wrote without counting. A fresh
    /// slot holds what the allocator returned; the exe reads only slots below `count`, and the
    /// model holds zeros, as the test's allocator does.
    pub slots: Vec<Pair>,
    /// The pairs in use, `0xb72634`: always at most `slots.len()`.
    pub count: u32,
    /// The pointer array's allocated size, `0xb7258c`: at least the slot count.
    pub alloc: u32,
    /// The array's grow quantum cap, `0xb72598`: 0 until a quantum is asked for a count of 64 or
    /// more.
    pub chunk: u32,
}

impl Default for OutbidList {
    fn default() -> OutbidList {
        OutbidList::new()
    }
}

impl OutbidList {
    /// What `0x4cc1c9` to `0x4cc234` build at the first start-up: eight zeroed slots, the array
    /// sized to eight, none in use, the quantum cap 0.
    pub fn new() -> OutbidList {
        OutbidList::started_with_cap(0)
    }

    /// What the same slice builds when the quantum cap `0xb72598` is already `chunk`: the
    /// teardown (`0x4cc3bb` to `0x4cc419`) zeroes the array, its slot count and the count but
    /// not the cap, so a later start-up sizes the array to eight rounded up to the cap, 64 once a
    /// list has doubled to 64 slots. The slots are eight either way.
    pub fn started_with_cap(chunk: u32) -> OutbidList {
        OutbidList {
            slots: vec![(0, 0); 8],
            count: 0,
            alloc: round_up(8, if chunk == 0 { 8 } else { chunk }),
            chunk,
        }
    }

    /// `0x4cde50(house, auction)`. With a slot free it fills the slot at `count` and counts it.
    /// Full, it doubles the slot count (growing the array to the doubled count, rounded up to a
    /// quantum, when the array holds fewer) and fills the slot at the old count without counting
    /// it, so the pair sits past `count` and the next append overwrites it.
    pub fn add(&mut self, house: u32, auction: u32) {
        let slots = self.slots.len() as u32;
        if self.count < slots {
            self.slots[self.count as usize] = (house, auction);
            self.count += 1;
            return;
        }
        let doubled = slots
            .checked_mul(2)
            .expect("a slot count doubled from 8 stays below 2^32");
        if doubled > self.alloc {
            let quantum = if self.chunk == 0 {
                quantum(doubled, &mut self.chunk)
            } else {
                self.chunk
            };
            self.alloc = round_up(doubled, quantum);
        }
        self.slots.resize(doubled as usize, (0, 0));
        self.slots[slots as usize] = (house, auction);
    }

    /// `0x4cdf30(auction)`: removes the first pair with the auction id, copying each later pair
    /// down a slot, and returns its slot; none when no pair has it.
    pub fn remove_auction(&mut self, auction: u32) -> Option<usize> {
        let at = self.slots[..self.count as usize]
            .iter()
            .position(|pair| pair.1 == auction)?;
        self.remove_at(at);
        Some(at)
    }

    /// `0x4cdf90(house)`: removes the first pair with the house id, as `remove_auction` does, and
    /// stops: the pairs after it with the same house stay. Returns its slot; none when no pair has
    /// the house.
    pub fn remove_house(&mut self, house: u32) -> Option<usize> {
        let at = self.slots[..self.count as usize]
            .iter()
            .position(|pair| pair.0 == house)?;
        self.remove_at(at);
        Some(at)
    }

    fn remove_at(&mut self, at: usize) {
        self.count -= 1;
        for slot in at..self.count as usize {
            self.slots[slot] = self.slots[slot + 1];
        }
    }
}
