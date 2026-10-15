//! The friend and ignore lists (`FriendList.cpp`; `systems/net/friends.md`).
//!
//! The object is `0x728` bytes: 50 friend slots of 32 bytes from `+0`, the count of friend names
//! pending at `+0x640`, 25 ignore guids from `+0x650` and the count of ignore names pending at
//! `+0x718`. A name comes from the name cache (`0x55f080`), which either returns it or calls back
//! later; a later call is a pending name.

use crate::DataStore;

/// A friend slot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FriendSlot {
    /// `+0`: 1 when online.
    pub online: u8,
    /// `+1`: the status flags.
    pub status: u8,
    /// `+4`: the name, a copy the slot owns, as its address.
    pub name: u32,
    /// `+8`.
    pub guid: u64,
    /// `+0x10`, `+0x14`, `+0x18`.
    pub level: u32,
    pub class: u32,
    pub area: u32,
}

/// A call out of the friend list code, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FriendCall {
    /// `SMemFree` of a slot's name.
    Free(u32),
    /// The name cache lookup of a guid: the friend slot whose guid it passes as the place to
    /// fill, or `None` for a copy of the guid on the stack; and whether the callback is the
    /// ignores' (`0x5acf40`) rather than the friends' (`0x5ace70`).
    Lookup {
        guid: u64,
        slot: Option<usize>,
        ignore: bool,
    },
    /// `0x5ae850`: sort the friends and fire `FRIENDLIST_UPDATE`.
    FriendsDone,
    /// `0x5ae910`: sort the ignores and fire their update.
    IgnoresDone,
}

/// The friend and ignore lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FriendList {
    pub slots: [FriendSlot; 50],
    /// `+0x640`.
    pub friends_pending: u32,
    /// `+0x650`.
    pub ignores: [u64; 25],
    /// `+0x718`.
    pub ignores_pending: u32,
}

impl FriendList {
    /// `FriendList::ReadFriendList` (`0x5ae350`), `SMSG_FRIEND_LIST`: frees every slot's name and
    /// clears the pointers, then reads a count byte and up to 50 records of a guid, a status byte
    /// and, for a non-zero status, the area, level and class (zeros for status 0). Each record's
    /// name is looked up: `lookup(guid)` is whether the cache has it, and a cached name is copied
    /// by `dup`; a miss adds to `+0x640`. With nothing pending at the end the friends sort. The
    /// packet holds every record it counts: a short one leaves the reads on stale stack.
    pub fn read_friend_list(
        &mut self,
        s: &mut DataStore,
        lookup: impl Fn(u64) -> bool,
        mut dup: impl FnMut() -> u32,
    ) -> Vec<FriendCall> {
        let mut calls = Vec::new();
        for slot in self.slots.iter_mut() {
            if slot.name != 0 {
                calls.push(FriendCall::Free(slot.name));
            }
            slot.name = 0;
        }
        let count = s.get(1).map_or(0, |v| v[0]);
        let dword = |s: &mut DataStore| s.get(4).map(|v| u32::from_le_bytes(v.try_into().unwrap()));
        for k in 0..usize::from(count).min(50) {
            let guid =
                u64::from_le_bytes(s.get(8).unwrap_or_default().try_into().unwrap_or([0; 8]));
            let status = s.get(1).map_or(0, |v| v[0]);
            let slot = &mut self.slots[k];
            slot.guid = guid;
            slot.online = u8::from(status != 0);
            slot.status = status;
            if status != 0 {
                if let Some(v) = dword(s) {
                    slot.area = v;
                }
                if let Some(v) = dword(s) {
                    slot.level = v;
                }
                if let Some(v) = dword(s) {
                    slot.class = v;
                }
            } else {
                slot.area = 0;
                slot.level = 0;
                slot.class = 0;
            }
            calls.push(FriendCall::Lookup {
                guid,
                slot: Some(k),
                ignore: false,
            });
            if lookup(guid) {
                slot.name = dup();
            } else {
                self.friends_pending = self.friends_pending.wrapping_add(1);
            }
        }
        if self.friends_pending == 0 {
            calls.push(FriendCall::FriendsDone);
        }
        calls
    }

    /// `FriendList::AddIgnoreSlot` (`0x5af5d0`, `(guid, lookup_name)`): stores the guid in the
    /// first ignore slot whose guid is 0, or does nothing when all 25 are taken. With
    /// `lookup_name` set it looks the name up, a miss adding to `+0x718`, and sorts the ignores
    /// when nothing is pending.
    pub fn add_ignore_slot(
        &mut self,
        guid: u64,
        lookup_name: bool,
        lookup: impl Fn(u64) -> bool,
        calls: &mut Vec<FriendCall>,
    ) {
        let Some(k) = self.ignores.iter().position(|&g| g == 0) else {
            return;
        };
        self.ignores[k] = guid;
        if !lookup_name {
            return;
        }
        calls.push(FriendCall::Lookup {
            guid,
            slot: None,
            ignore: true,
        });
        if !lookup(guid) {
            self.ignores_pending = self.ignores_pending.wrapping_add(1);
        }
        if self.ignores_pending == 0 {
            calls.push(FriendCall::IgnoresDone);
        }
    }

    /// `FriendList::ReadIgnoreList` (`0x5af530`), `SMSG_IGNORE_LIST`: clears the 25 ignores,
    /// reads a count byte and that many guids, with no cap: each goes through
    /// [`FriendList::add_ignore_slot`] without a lookup, then is looked up, a miss adding to
    /// `+0x718`, which is not reset first. With nothing pending at the end the ignores sort.
    pub fn read_ignore_list(
        &mut self,
        s: &mut DataStore,
        lookup: impl Fn(u64) -> bool,
    ) -> Vec<FriendCall> {
        let mut calls = Vec::new();
        self.ignores = [0; 25];
        let count = s.get(1).map_or(0, |v| v[0]);
        for _ in 0..count {
            let guid =
                u64::from_le_bytes(s.get(8).unwrap_or_default().try_into().unwrap_or([0; 8]));
            self.add_ignore_slot(guid, false, &lookup, &mut calls);
            calls.push(FriendCall::Lookup {
                guid,
                slot: None,
                ignore: true,
            });
            if !lookup(guid) {
                self.ignores_pending = self.ignores_pending.wrapping_add(1);
            }
        }
        if self.ignores_pending == 0 {
            calls.push(FriendCall::IgnoresDone);
        }
        calls
    }

    /// `FriendList::DecrementPendingFriendNames` (`0x5aeb70`): a non-zero `+0x640` drops by one,
    /// and the friends sort when it reaches 0. Returns whether they sort.
    pub fn decrement_friends_pending(&mut self) -> bool {
        if self.friends_pending == 0 {
            return false;
        }
        self.friends_pending -= 1;
        self.friends_pending == 0
    }

    /// `FriendList::DecrementPendingIgnoreNames` (`0x5aeb90`): the same on `+0x718`.
    pub fn decrement_ignores_pending(&mut self) -> bool {
        if self.ignores_pending == 0 {
            return false;
        }
        self.ignores_pending -= 1;
        self.ignores_pending == 0
    }
}

/// `__strnicmp` (`0x414310`) as the client runs it, in the C locale (`_setlocale` is unreached):
/// `A` to `Z` fold to lower case, the first differing pair of bytes, unsigned, decides `-1` or 1,
/// and a NUL in either string ends the compare, or `n` bytes. `SStrCmpI` (`0x64a4c0`) calls it
/// for two non-null strings.
pub fn strnicmp(a: &[u8], b: &[u8], n: u32) -> i32 {
    let fold = |c: u8| if c.is_ascii_uppercase() { c + 0x20 } else { c };
    let at = |s: &[u8], i: usize| s.get(i).copied().unwrap_or(0);
    let mut i = 0usize;
    let mut left = n;
    if left == 0 {
        return 0;
    }
    loop {
        let (x, y) = (at(a, i), at(b, i));
        if x == 0 || y == 0 {
            return match x.cmp(&y) {
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Greater => 1,
            };
        }
        let (x, y) = (fold(x), fold(y));
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        i += 1;
        left -= 1;
        if left == 0 {
            return 0;
        }
    }
}

/// The friend order of `CompareFriends` (`0x5ae870`, the `qsort` comparator of the slots): an
/// empty slot (guid 0) after a full one, two empty ones equal; an online slot before an offline
/// one; then the names by [`strnicmp`] with no bound, a missing name after a present one, two
/// missing ones equal. `a_name` and `b_name` are the names, `None` for a null pointer.
pub fn compare_friends(
    a: &FriendSlot,
    b: &FriendSlot,
    a_name: Option<&[u8]>,
    b_name: Option<&[u8]>,
) -> i32 {
    match (a.guid == 0, b.guid == 0) {
        (true, true) => return 0,
        (true, false) => return 1,
        (false, true) => return -1,
        _ => {}
    }
    if a.online != b.online {
        return if a.online == 0 { 1 } else { -1 };
    }
    match (a_name, b_name) {
        (Some(x), Some(y)) => strnicmp(x, y, 0x7fff_ffff),
        (None, None) => 0,
        (None, Some(_)) => 1,
        (Some(_), None) => -1,
    }
}
