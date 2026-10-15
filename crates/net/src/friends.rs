//! The friend and ignore lists (`FriendList.cpp`; `systems/net/friends.md`).
//!
//! The object is `0x728` bytes: 50 friend slots of 32 bytes from `+0`, the count of friend names
//! pending at `+0x640`, 25 ignore guids from `+0x650` and the count of ignore names pending at
//! `+0x718`. A name comes from the name cache (`0x55f080`), which either returns it or calls back
//! later; a later call is a pending name.

use crate::DataStore;

/// The message id a lookup passes when its callback only stores the name.
pub const NAME_ONLY: u32 = 0x1d1;

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
    /// fill, or `None` for a copy of the guid on the stack; whether the callback is the ignores'
    /// (`0x5acf40`) rather than the friends' (`0x5ace70`); and the message id the callback shows.
    Lookup {
        guid: u64,
        slot: Option<usize>,
        ignore: bool,
        id: u32,
    },
    /// `SStrDupA` of a cached name.
    Dup(u32),
    /// `0x5ae850`: sort the friends and fire `FRIENDLIST_UPDATE`.
    FriendsDone,
    /// `0x5ae910`: sort the ignores and fire their update.
    IgnoresDone,
    /// The game error `0x496720`, with the cached name passed twice, or alone.
    GameError { id: u32, name: Option<u32> },
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
    /// The guid `FindOrAddFriendSlot` writes as slot `-1`, at `-0x18` before the object, with a
    /// zero byte at `-0x20`, when it neither finds the guid nor an empty slot.
    pub before: Option<u64>,
}

fn guid(s: &mut DataStore) -> u64 {
    u64::from_le_bytes(s.get(8).unwrap_or_default().try_into().unwrap_or([0; 8]))
}

fn dword(s: &mut DataStore) -> Option<u32> {
    s.get(4).map(|v| u32::from_le_bytes(v.try_into().unwrap()))
}

impl FriendList {
    /// `FriendList::ReadFriendList` (`0x5ae350`), `SMSG_FRIEND_LIST`: frees every slot's name and
    /// clears the pointers, then reads a count byte and up to 50 records of a guid, a status byte
    /// and, for a non-zero status, the area, level and class (zeros for status 0). Each record's
    /// name is looked up: `lookup(guid)` is the cache's name or `None`, and a cached name is
    /// copied by `dup`; a miss adds to `+0x640`. With nothing pending at the end the friends
    /// sort. The packet holds every record it counts: a short one leaves the reads on stale
    /// stack.
    pub fn read_friend_list(
        &mut self,
        s: &mut DataStore,
        lookup: impl Fn(u64) -> Option<u32>,
        mut dup: impl FnMut(u32) -> u32,
    ) -> Vec<FriendCall> {
        let mut calls = Vec::new();
        for slot in self.slots.iter_mut() {
            if slot.name != 0 {
                calls.push(FriendCall::Free(slot.name));
            }
            slot.name = 0;
        }
        let count = s.get(1).map_or(0, |v| v[0]);
        for k in 0..usize::from(count).min(50) {
            let guid = guid(s);
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
                id: NAME_ONLY,
            });
            match lookup(guid) {
                Some(name) => {
                    calls.push(FriendCall::Dup(name));
                    slot.name = dup(name);
                }
                None => self.friends_pending = self.friends_pending.wrapping_add(1),
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
        lookup: impl Fn(u64) -> Option<u32>,
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
            id: NAME_ONLY,
        });
        if lookup(guid).is_none() {
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
        lookup: impl Fn(u64) -> Option<u32>,
    ) -> Vec<FriendCall> {
        let mut calls = Vec::new();
        self.ignores = [0; 25];
        let count = s.get(1).map_or(0, |v| v[0]);
        for _ in 0..count {
            let guid = guid(s);
            self.add_ignore_slot(guid, false, &lookup, &mut calls);
            calls.push(FriendCall::Lookup {
                guid,
                slot: None,
                ignore: true,
                id: NAME_ONLY,
            });
            if lookup(guid).is_none() {
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

    /// The first of the 50 slots whose guid is `guid`, empty slots included.
    fn find(&self, guid: u64) -> Option<usize> {
        self.slots.iter().position(|s| s.guid == guid)
    }

    /// `FindOrAddFriendSlot` (`0x5aea60`): the slot holding `guid`, else the first empty one,
    /// which takes the guid and an online byte of 0. The scan stops at that empty slot, so a
    /// match after it is not seen. With neither it writes slot `-1`, before the object, and
    /// returns `-1`.
    fn find_or_add(&mut self, guid: u64) -> i32 {
        for k in 0..50 {
            if self.slots[k].guid == guid {
                return k as i32;
            }
            if self.slots[k].guid == 0 {
                self.slots[k].guid = guid;
                self.slots[k].online = 0;
                return k as i32;
            }
        }
        self.before = Some(guid);
        -1
    }

    /// `SetFriendStatus` (`0x5aeb30`): the online byte of the guid's slot.
    fn set_online(&mut self, guid: u64, online: u8) {
        if let Some(k) = self.find(guid) {
            self.slots[k].online = online;
        }
    }

    /// `FriendList::HandleStatus` (`0x5acab0`, `(result, guid, store)`), `SMSG_FRIEND_STATUS`
    /// after its result byte and guid. Results 0, 1, 4, 9 to 13, 17 and above show their game
    /// error alone. The others act on the lists, then look the name up with the error id as
    /// the callback's message: a cached name is stored for result 6 or 7, the list sorts and
    /// the error shows with the name twice; a miss adds to the list's pending count. Result 2
    /// and 6 read a status byte and the area, level and class; the store holds them, or the
    /// reads leave stale stack.
    pub fn handle_status(
        &mut self,
        result: u32,
        guid: u64,
        s: &mut DataStore,
        lookup: impl Fn(u64) -> Option<u32>,
        mut dup: impl FnMut(u32) -> u32,
    ) -> Vec<FriendCall> {
        let mut calls = Vec::new();
        let alone = match result {
            0 | 4 => Some(0x108),
            1 => Some(0x104),
            9 => Some(0x10d),
            10 => Some(0x109),
            11 => Some(0x10e),
            12 => Some(0x10f),
            13 => Some(0x110),
            17 => Some(0x114),
            18.. => Some(0x10b),
            _ => None,
        };
        if let Some(id) = alone {
            calls.push(FriendCall::GameError { id, name: None });
            return calls;
        }
        let fields = |s: &mut DataStore| {
            let status = s.get(1).map_or(0, |v| v[0]);
            let area = dword(s).unwrap_or(0);
            let level = dword(s).unwrap_or(0);
            let class = dword(s).unwrap_or(0);
            (status, area, level, class)
        };
        let fill = |slot: &mut FriendSlot, (status, area, level, class): (u8, u32, u32, u32)| {
            slot.status = status;
            slot.area = area;
            slot.level = level;
            slot.class = class;
        };
        let (id, ignore) = match result {
            2 => {
                let f = fields(s);
                if let Some(k) = self.find(guid) {
                    fill(&mut self.slots[k], f);
                }
                self.set_online(guid, 1);
                (0x106, false)
            }
            3 => {
                self.set_online(guid, 0);
                (0x107, false)
            }
            5 => {
                if let Some(k) = self.find(guid) {
                    let slot = &mut self.slots[k];
                    slot.guid = 0;
                    if slot.name != 0 {
                        calls.push(FriendCall::Free(slot.name));
                    }
                    slot.name = 0;
                }
                (0x10a, false)
            }
            6 => {
                let f = fields(s);
                let k = self.find_or_add(guid);
                if k >= 0 {
                    fill(&mut self.slots[k as usize], f);
                }
                self.set_online(guid, 1);
                (0x105, false)
            }
            7 => {
                self.find_or_add(guid);
                self.set_online(guid, 0);
                (0x105, false)
            }
            8 => {
                self.find_or_add(guid);
                (0x10c, false)
            }
            14 => (0x111, true),
            15 => {
                self.add_ignore_slot(guid, true, &lookup, &mut calls);
                (0x112, true)
            }
            _ => {
                if let Some(k) = self.ignores.iter().position(|&g| g == guid) {
                    self.ignores[k] = 0;
                    if self.ignores_pending == 0 {
                        calls.push(FriendCall::IgnoresDone);
                    }
                }
                (0x113, true)
            }
        };
        calls.push(FriendCall::Lookup {
            guid,
            slot: None,
            ignore,
            id,
        });
        match lookup(guid) {
            Some(name) => {
                if id == 0x105 {
                    if let Some(k) = self.find(guid) {
                        let slot = &mut self.slots[k];
                        if slot.name != 0 {
                            calls.push(FriendCall::Free(slot.name));
                        }
                        calls.push(FriendCall::Dup(name));
                        slot.name = dup(name);
                    }
                }
                calls.push(if ignore {
                    FriendCall::IgnoresDone
                } else {
                    FriendCall::FriendsDone
                });
                calls.push(FriendCall::GameError {
                    id,
                    name: Some(name),
                });
            }
            None if ignore => self.ignores_pending = self.ignores_pending.wrapping_add(1),
            None => self.friends_pending = self.friends_pending.wrapping_add(1),
        }
        calls
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

/// A who result as the comparator reads it: the name at `+0`, the guild at `+0x30`, and the
/// level, race, class and zone dwords at `+0x90` to `+0x9c`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WhoRecord {
    pub name: Vec<u8>,
    pub guild: Vec<u8>,
    pub level: u32,
    pub race: u32,
    pub class: u32,
    pub zone: u32,
}

/// A DBC table's localized names as the comparator reaches them: the largest id and the rows,
/// `None` for an id with no row.
pub struct NameTable<'a> {
    pub max_id: i32,
    pub rows: &'a [Option<&'a [u8]>],
}

impl NameTable<'_> {
    /// The row's name for an id from 0 to the largest, compared as signed; `None` otherwise.
    pub fn name(&self, id: u32) -> Option<&[u8]> {
        let i = id as i32;
        if i < 0 || i > self.max_id {
            return None;
        }
        self.rows[i as usize]
    }
}

/// `CompareWhoResults` (`0x5ada00`): walks the seven `(key, descending)` pairs of the sort chain
/// (`0xc2817c`) and returns the first non-zero comparison, negated when its pair's direction is
/// not 0, or 0. Keys: 0 the zone's name, 1 the level as signed dwords (1 or `-1`), 2 the class's
/// name, 4 the name, 5 the race's name, 6 the guild; 3 and any key above 6 tie. The names
/// compare with [`strnicmp`] and no bound; an id with no row on either side ties.
pub fn compare_who(
    a: &WhoRecord,
    b: &WhoRecord,
    chain: &[(u32, u32); 7],
    zones: &NameTable,
    classes: &NameTable,
    races: &NameTable,
) -> i32 {
    let rows = |t: &NameTable, x: u32, y: u32| match (t.name(x), t.name(y)) {
        (Some(p), Some(q)) => strnicmp(p, q, 0x7fff_ffff),
        _ => 0,
    };
    for &(key, descending) in chain {
        let c = match key {
            0 => rows(zones, a.zone, b.zone),
            1 => match (a.level as i32).cmp(&(b.level as i32)) {
                std::cmp::Ordering::Equal => 0,
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Greater => 1,
            },
            2 => rows(classes, a.class, b.class),
            4 => strnicmp(&a.name, &b.name, 0x7fff_ffff),
            5 => rows(races, a.race, b.race),
            6 => strnicmp(&a.guild, &b.guild, 0x7fff_ffff),
            _ => 0,
        };
        if c != 0 {
            return if descending != 0 { c.wrapping_neg() } else { c };
        }
    }
    0
}

/// A console line `ReverseWhoisResponseHandler` writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WhoisLine {
    /// `RWhoIs failed` (`0x85dc58`) in colour 3.
    Failed,
    /// `Not found` (`0x85dc4c`) in colour 4.
    NotFound,
    /// `Account: %s` (`0x85dc3c`), through `ConsolePrintf`.
    Account(Vec<u8>),
    /// A character's name (`%s`, `0x85dc34`) in colour 4 at the index, else 0.
    Character { name: Vec<u8>, colour: u32 },
}

/// The C string in a buffer the store filled.
fn text(buf: &[u8]) -> Vec<u8> {
    buf.iter().take_while(|&&c| c != 0).copied().collect()
}

/// `ReverseWhoisResponseHandler` (`0x5adde0`), `SMSG_RWHOIS`: a signed count, `-1` for a failure
/// and 0 for none; for a positive count, each account is a C string of up to 64 bytes, a
/// character count and an index, then the characters' C strings of up to 48 bytes, the one at
/// the index in colour 4. Each dword is zeroed before its read, so a short packet reads zeros,
/// and a name the store cannot give, or one that fills its buffer, is empty.
pub fn reverse_whois(s: &mut DataStore) -> Vec<WhoisLine> {
    let mut lines = Vec::new();
    let dword = |s: &mut DataStore| dword(s).unwrap_or(0) as i32;
    let count = dword(s);
    if count == -1 {
        lines.push(WhoisLine::Failed);
        return lines;
    }
    if count == 0 {
        lines.push(WhoisLine::NotFound);
        return lines;
    }
    for _ in 0..count.max(0) {
        let mut account = [0u8; 0x40];
        s.get_cstring(&mut account, false);
        lines.push(WhoisLine::Account(text(&account)));
        let n = dword(s);
        let index = dword(s);
        for i in 0..n.max(0) {
            let mut name = [0u8; 0x30];
            s.get_cstring(&mut name, false);
            lines.push(WhoisLine::Character {
                name: text(&name),
                colour: if i == index { 4 } else { 0 },
            });
        }
    }
    lines
}
