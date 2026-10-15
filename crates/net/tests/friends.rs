//! The friend and ignore lists (`systems/net/friends.md`), each function run whole beside the
//! model.

mod common;

use benilla_re_difftest::{Emu, RegisterX86};
use benilla_re_net::{
    compare_friends, compare_who, reverse_whois, strnicmp, DataStore, Fetch, FriendCall,
    FriendList, FriendSlot, NameTable, WhoRecord, WhoisLine,
};
use common::{Lcg, Wow};

const READ_FRIEND_LIST: u64 = 0x5ae350;
const READ_IGNORE_LIST: u64 = 0x5af530;
const ADD_IGNORE_SLOT: u64 = 0x5af5d0;
const HANDLE_STATUS: u64 = 0x5acab0;
const GAME_ERROR: u64 = 0x496720;
const DEC_FRIENDS: u64 = 0x5aeb70;
const DEC_IGNORES: u64 = 0x5aeb90;
const COMPARE_FRIENDS: u64 = 0x5ae870;
const COMPARE_WHO: u64 = 0x5ada00;
const REVERSE_WHOIS: u64 = 0x5adde0;
const CONSOLE_WRITE: u64 = 0x63cd00;
const CONSOLE_PRINTF: u64 = 0x63cd50;
const STRNICMP: u64 = 0x414310;
const SSTRCMPI: u64 = 0x64a4c0;
const SERR_SET_LAST_ERROR: u64 = 0x64e850;
const FRIENDS_DONE: u64 = 0x5ae850;
const IGNORES_DONE: u64 = 0x5ae910;
const NAME_LOOKUP: u64 = 0x55f080;
const SSTRDUPA: u64 = 0x64a620;
const SMEMFREE: u64 = 0x646430;
const DUP_BASE: u64 = 0x5000_0000;
const DUP_STRIDE: u64 = 0x40;
/// The name the cache returns for a guid whose low bit is set.
const NAME: u32 = 0x00c0_ffee;

struct Rig<'a> {
    e: Emu<'a>,
    fl: u64,
    store: u64,
    buf: u64,
    /// The lookup island's log: a count, then five dwords a call in `0x20` bytes.
    log: u64,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let fl = e.alloc(0x748) + 0x20;
        let store = e.alloc(0x20);
        let buf = e.alloc(0x1000);
        let log = e.alloc(0x2000);
        let cnt = log as u32;
        let base = cnt + 0x20;
        let mut c = vec![0xa1];
        c.extend_from_slice(&cnt.to_le_bytes());
        c.extend_from_slice(&[0xc1, 0xe0, 0x05, 0x05]);
        c.extend_from_slice(&base.to_le_bytes());
        for (i, off) in [
            (0x04u8, 0x00u8),
            (0x08, 0x04),
            (0x0c, 0x08),
            (0x10, 0x0c),
            (0x14, 0x10),
        ] {
            c.extend_from_slice(&[0x8b, 0x4c, 0x24, i]);
            if off == 0 {
                c.extend_from_slice(&[0x89, 0x08]);
            } else {
                c.extend_from_slice(&[0x89, 0x48, off]);
            }
        }
        c.extend_from_slice(&[0xff, 0x05]);
        c.extend_from_slice(&cnt.to_le_bytes());
        c.extend_from_slice(&[0x8b, 0x44, 0x24, 0x04, 0xa8, 0x01, 0x74, 0x08, 0xb8]);
        c.extend_from_slice(&NAME.to_le_bytes());
        c.extend_from_slice(&[0xc2, 0x18, 0x00, 0x31, 0xc0, 0xc2, 0x18, 0x00]);
        let island = e.alloc(0x80);
        e.write(island, &c);
        let rel = (island as u32).wrapping_sub(NAME_LOOKUP as u32 + 5);
        let mut jmp = vec![0xe9];
        jmp.extend_from_slice(&rel.to_le_bytes());
        e.write(NAME_LOOKUP, &jmp);
        e.stub_alloc(SSTRDUPA, DUP_BASE, DUP_STRIDE, 3);
        e.stub(SMEMFREE, 1, 4);
        e.stub(FRIENDS_DONE, 0, 0);
        e.stub(IGNORES_DONE, 0, 0);
        e.stub(GAME_ERROR, 0, 0);
        Rig {
            e,
            fl,
            store,
            buf,
            log,
        }
    }

    /// Runs a member of the list with `EDI` and `ESI` zero, which a one-argument game error then
    /// shows as its missing name.
    fn call(&mut self, f: u64, args: &[u32]) {
        self.e.set_reg(RegisterX86::EDI, 0);
        self.e.set_reg(RegisterX86::ESI, 0);
        self.e.call(f, self.fl as u32, 0, args);
    }

    fn put(&mut self, f: &FriendList) {
        self.e.write(self.fl - 0x20, &[0xa5; 0x20]);
        for (k, s) in f.slots.iter().enumerate() {
            let a = self.fl + 0x20 * k as u64;
            self.e.write(a, &[s.online, s.status, 0xa5, 0xa5]);
            self.e.write_u32(a + 4, s.name);
            self.e.write(a + 8, &s.guid.to_le_bytes());
            self.e.write_u32(a + 0x10, s.level);
            self.e.write_u32(a + 0x14, s.class);
            self.e.write_u32(a + 0x18, s.area);
        }
        self.e.write_u32(self.fl + 0x640, f.friends_pending);
        for (k, g) in f.ignores.iter().enumerate() {
            self.e
                .write(self.fl + 0x650 + 8 * k as u64, &g.to_le_bytes());
        }
        self.e.write_u32(self.fl + 0x718, f.ignores_pending);
        self.e.write_u32(self.log, 0);
        self.e.reset_bump(DUP_BASE);
        self.e.clear_calls();
    }

    fn get(&self) -> FriendList {
        let mut f = FriendList {
            slots: [FriendSlot::default(); 50],
            friends_pending: self.e.read_u32(self.fl + 0x640),
            ignores: [0; 25],
            ignores_pending: self.e.read_u32(self.fl + 0x718),
            before: None,
        };
        let margin = self.e.read(self.fl - 0x20, 0x20);
        if margin != [0xa5; 0x20] {
            let g = u64::from_le_bytes(margin[8..16].try_into().unwrap());
            let mut want = [0xa5; 0x20];
            want[0] = 0;
            want[8..16].copy_from_slice(&g.to_le_bytes());
            assert_eq!(margin, want, "the bytes before the object");
            f.before = Some(g);
        }
        for (k, s) in f.slots.iter_mut().enumerate() {
            let a = self.fl + 0x20 * k as u64;
            let b = self.e.read(a, 2);
            let g = self.e.read(a + 8, 8);
            *s = FriendSlot {
                online: b[0],
                status: b[1],
                name: self.e.read_u32(a + 4),
                guid: u64::from_le_bytes(g.try_into().unwrap()),
                level: self.e.read_u32(a + 0x10),
                class: self.e.read_u32(a + 0x14),
                area: self.e.read_u32(a + 0x18),
            };
        }
        for (k, g) in f.ignores.iter_mut().enumerate() {
            let v = self.e.read(self.fl + 0x650 + 8 * k as u64, 8);
            *g = u64::from_le_bytes(v.try_into().unwrap());
        }
        f
    }

    fn load_store(&mut self, bytes: &[u8]) {
        self.e.write(self.buf, bytes);
        let len = bytes.len() as u32;
        for (i, v) in [0x7ff9e4u32, self.buf as u32, 0, len.max(1), len, 0]
            .into_iter()
            .enumerate()
        {
            self.e.write_u32(self.store + 4 * i as u64, v);
        }
    }

    /// The calls in the model's terms: the stubs in order, then the lookups the island logged
    /// placed where the model has them by comparing the two lists separately.
    fn calls(&self) -> (Vec<FriendCall>, Vec<FriendCall>) {
        let stubs = self
            .e
            .calls()
            .iter()
            .filter_map(|c| match c.addr {
                SMEMFREE => Some(FriendCall::Free(c.args[0])),
                SSTRDUPA => Some(FriendCall::Dup(c.args[0])),
                GAME_ERROR => {
                    let name = c.args[1];
                    assert_eq!(c.args[2], name, "the name twice");
                    Some(FriendCall::GameError {
                        id: c.args[0],
                        name: (name != 0).then_some(name),
                    })
                }
                FRIENDS_DONE => Some(FriendCall::FriendsDone),
                IGNORES_DONE => Some(FriendCall::IgnoresDone),
                _ => None,
            })
            .collect();
        let n = self.e.read_u32(self.log);
        let lookups = (0..u64::from(n))
            .map(|i| {
                let a = self.log + 0x20 + 0x20 * i;
                let lo = self.e.read_u32(a);
                let hi = self.e.read_u32(a + 4);
                let at = u64::from(self.e.read_u32(a + 8));
                let cb = self.e.read_u32(a + 12);
                let slot = (self.fl..self.fl + 0x640)
                    .contains(&at)
                    .then(|| ((at - self.fl - 8) / 0x20) as usize);
                FriendCall::Lookup {
                    guid: u64::from(lo) | (u64::from(hi) << 32),
                    slot,
                    ignore: cb == 0x5acf40,
                    id: self.e.read_u32(a + 16),
                }
            })
            .collect();
        (stubs, lookups)
    }
}

fn split(calls: Vec<FriendCall>) -> (Vec<FriendCall>, Vec<FriendCall>) {
    calls
        .into_iter()
        .partition(|c| !matches!(c, FriendCall::Lookup { .. }))
}

fn guid(g: &mut Lcg) -> u64 {
    match g.next() >> 29 {
        0 => 0,
        1 => u64::from(g.next()),
        _ => u64::from(g.next()) | (u64::from(g.next()) << 32),
    }
}

fn list(g: &mut Lcg) -> FriendList {
    let mut f = FriendList {
        slots: [FriendSlot::default(); 50],
        friends_pending: [0u32, 0, 1, 3][(g.next() >> 16) as usize % 4],
        ignores: [0; 25],
        ignores_pending: [0u32, 0, 1, 2][(g.next() >> 16) as usize % 4],
        before: None,
    };
    for s in f.slots.iter_mut() {
        *s = FriendSlot {
            online: (g.next() >> 31) as u8,
            status: (g.next() >> 24) as u8,
            name: if g.next() >> 30 == 0 {
                0
            } else {
                0x6000_0000 | (g.next() >> 8)
            },
            guid: guid(g),
            level: g.next(),
            class: g.next(),
            area: g.next(),
        };
    }
    let full = (g.next() >> 16) as usize % 27;
    for (k, x) in f.ignores.iter_mut().enumerate() {
        *x = if k < full { guid(g) | 1 } else { 0 };
    }
    f
}

/// The name cache: a guid whose low bit is set has [`NAME`].
fn cached(guid: u64) -> Option<u32> {
    (guid & 1 == 1).then_some(NAME)
}

fn plain(bytes: &[u8]) -> DataStore {
    let mut m = DataStore::new(Fetch::Heap);
    m.data = bytes.to_vec();
    m.alloc = (bytes.len() as u32).max(1);
    m.size = bytes.len() as u32;
    m
}

/// `FriendList::ReadFriendList` whole on lists of 0 to 60 records, statuses 0 and not, names in
/// the cache or not: every slot, the pending count, the names freed, the lookups with the slot
/// each fills, and the sort when nothing is pending.
#[test]
fn friends_read_list() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0xf41e_0001);
    for case in 0..1500 {
        let mut f = list(&mut g);
        let count = [0u8, 1, 3, 49, 50, 51, 60][(g.next() >> 16) as usize % 7];
        let mut bytes = vec![count];
        for _ in 0..count {
            bytes.extend(guid(&mut g).to_le_bytes());
            let st = if g.next() >> 30 == 0 {
                0
            } else {
                (g.next() >> 24) as u8 | 1
            };
            bytes.push(st);
            if st != 0 {
                for _ in 0..3 {
                    bytes.extend(g.next().to_le_bytes());
                }
            }
        }
        r.put(&f);
        r.load_store(&bytes);
        r.call(READ_FRIEND_LIST, &[r.store as u32]);
        let mut n = 0;
        let calls = f.read_friend_list(&mut plain(&bytes), cached, |_| {
            n += 1;
            (DUP_BASE + DUP_STRIDE * (n - 1)) as u32
        });
        let ctx = format!("case {case} count {count}");
        assert_eq!(r.get(), f, "{ctx}: list");
        assert_eq!(r.calls(), split(calls), "{ctx}: calls");
    }
}

/// `FriendList::ReadIgnoreList` and `FriendList::AddIgnoreSlot` whole: lists of 0 to 30 guids,
/// past the 25 slots, guids of 0, names in the cache or not, pending counts that start above 0,
/// and a single add with and without its lookup into lists from empty to full.
#[test]
fn friends_ignore_list() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x1d40_0001);
    for case in 0..1500 {
        let mut f = list(&mut g);
        let ctx = format!("case {case}");
        if case % 2 == 0 {
            let count = [0u8, 1, 5, 24, 25, 26, 30][(g.next() >> 16) as usize % 7];
            let mut bytes = vec![count];
            for _ in 0..count {
                bytes.extend(guid(&mut g).to_le_bytes());
            }
            r.put(&f);
            r.load_store(&bytes);
            r.call(READ_IGNORE_LIST, &[r.store as u32]);
            let calls = f.read_ignore_list(&mut plain(&bytes), cached);
            assert_eq!(r.get(), f, "{ctx}: list");
            assert_eq!(r.calls(), split(calls), "{ctx}: calls");
        } else {
            let gd = guid(&mut g);
            let named = g.next() >> 31 == 0;
            r.put(&f);
            let flag = if named { g.next() | 1 } else { 0 };
            r.call(ADD_IGNORE_SLOT, &[gd as u32, (gd >> 32) as u32, flag]);
            let mut calls = Vec::new();
            f.add_ignore_slot(gd, named, cached, &mut calls);
            assert_eq!(r.get(), f, "{ctx}: add");
            assert_eq!(r.calls(), split(calls), "{ctx}: add calls");
        }
    }
}

/// `DecrementPendingFriendNames` and `DecrementPendingIgnoreNames` from 0, 1, 2 and the top: the
/// count and the sort at zero.
#[test]
fn friends_pending_counts() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0xdec0_0001);
    for &n in &[0u32, 1, 2, 3, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        for ignores in [false, true] {
            let mut f = list(&mut g);
            if ignores {
                f.ignores_pending = n;
            } else {
                f.friends_pending = n;
            }
            r.put(&f);
            r.call(if ignores { DEC_IGNORES } else { DEC_FRIENDS }, &[]);
            let done = if ignores {
                f.decrement_ignores_pending()
            } else {
                f.decrement_friends_pending()
            };
            let ctx = format!("{n:#x} ignores {ignores}");
            assert_eq!(r.get(), f, "{ctx}: counts");
            let want = match (done, ignores) {
                (false, _) => vec![],
                (true, false) => vec![FriendCall::FriendsDone],
                (true, true) => vec![FriendCall::IgnoresDone],
            };
            assert_eq!(r.calls().0, want, "{ctx}: sort");
        }
    }
}

/// `FriendList::HandleStatus` whole on every result from 0 to 19 and large ones, for guids in a
/// friend slot, in the ignores, new and 0, cached or not, against lists packed or scattered, from
/// empty to full: the slots, the ignores, the pending counts, the bytes before the object, the
/// names freed and copied, the lookups with their message ids, the sorts and the game errors.
#[test]
fn friends_status() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut g = Lcg(0x57a7_0001);
    for case in 0..8000 {
        let mut f = list(&mut g);
        if g.next() >> 31 == 0 {
            let n = [0usize, 1, 10, 49, 50, 50][(g.next() >> 16) as usize % 6];
            for (k, s) in f.slots.iter_mut().enumerate() {
                s.guid = if k < n { guid(&mut g) | (1 << 33) } else { 0 };
            }
        }
        let result = if g.next() >> 29 == 0 {
            [18u32, 19, 0x100, 0x8000_0000, 0xffff_ffff][(g.next() >> 16) as usize % 5]
        } else {
            (g.next() >> 16) % 18
        };
        let gd = match g.next() >> 30 {
            0 => f.slots[(g.next() >> 16) as usize % 50].guid,
            1 => f.ignores[(g.next() >> 16) as usize % 25],
            2 => guid(&mut g),
            _ => guid(&mut g) & !1 | u64::from(g.next() >> 31),
        };
        let mut bytes = vec![(g.next() >> 24) as u8];
        for _ in 0..3 {
            bytes.extend(g.next().to_le_bytes());
        }
        r.put(&f);
        r.load_store(&bytes);
        r.call(
            HANDLE_STATUS,
            &[result, gd as u32, (gd >> 32) as u32, r.store as u32],
        );
        let mut n = 0;
        let calls = f.handle_status(result, gd, &mut plain(&bytes), cached, |_| {
            n += 1;
            (DUP_BASE + DUP_STRIDE * (n - 1)) as u32
        });
        let ctx = format!("case {case} result {result} guid {gd:#x}");
        assert_eq!(r.get(), f, "{ctx}: list");
        assert_eq!(r.calls(), split(calls), "{ctx}: calls");
    }
}

/// A name for the comparator: mixed case, the characters around `A`, `Z`, `a`, `z` and `@`, `[`,
/// bytes at and above `0x80`, often a prefix of another.
fn name(g: &mut Lcg) -> Vec<u8> {
    const PICK: &[u8] = b"aAbBzZ@[`{_0 \x7f\x80\xc8\xff";
    let n = (g.next() >> 16) as usize % 6;
    (0..n)
        .map(|_| PICK[(g.next() >> 16) as usize % PICK.len()])
        .collect()
}

/// `CompareFriends` whole on pairs of slots: empty and full, online and offline bytes that
/// differ in any way, and names present or missing that differ in case, length and high bytes;
/// `__strnicmp` on the same names with bounds 0, 1, 3 and none; and `SStrCmpI`, which calls it
/// for two strings and otherwise sets error `0x57` and returns the first pointer less the second.
#[test]
fn friends_compare() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let a = e.alloc(0x20);
    let b = e.alloc(0x20);
    let sa = e.alloc(0x20);
    let sb = e.alloc(0x20);
    e.stub(SERR_SET_LAST_ERROR, 0, 1);
    let mut g = Lcg(0xc0f4_0001);
    for case in 0..6000 {
        let na = name(&mut g);
        let nb = if g.next() >> 30 == 0 {
            na.to_ascii_uppercase()
        } else {
            name(&mut g)
        };
        let mut za = na.clone();
        za.push(0);
        let mut zb = nb.clone();
        zb.push(0);
        e.write(sa, &za);
        e.write(sb, &zb);
        let has_a = g.next() >> 29 != 0;
        let has_b = g.next() >> 29 != 0;
        let slot = |g: &mut Lcg, has: bool, at: u64| FriendSlot {
            online: [0u8, 1, 2, 0x80][(g.next() >> 16) as usize % 4],
            status: 0,
            name: if has { at as u32 } else { 0 },
            guid: if g.next() >> 29 == 0 {
                0
            } else {
                guid(g) | (1 << 40)
            },
            ..FriendSlot::default()
        };
        let x = slot(&mut g, has_a, sa);
        let y = slot(&mut g, has_b, sb);
        for (s, at) in [(&x, a), (&y, b)] {
            e.write(at, &[s.online, s.status, 0, 0]);
            e.write_u32(at + 4, s.name);
            e.write(at + 8, &s.guid.to_le_bytes());
        }
        let got = e.call(COMPARE_FRIENDS, 0, 0, &[a as u32, b as u32]) as i32;
        let want = compare_friends(&x, &y, has_a.then_some(&na[..]), has_b.then_some(&nb[..]));
        assert_eq!(got, want, "case {case} {x:?} {y:?} {na:02x?} {nb:02x?}");
        for n in [0u32, 1, 3, 0x7fff_ffff] {
            let got = e.call(STRNICMP, 0, 0, &[sa as u32, sb as u32, n]) as i32;
            assert_eq!(
                got,
                strnicmp(&na, &nb, n),
                "strnicmp {na:02x?} {nb:02x?} {n}"
            );
            e.clear_calls();
            let pa = if has_a { sa as u32 } else { 0 };
            let pb = if has_b { sb as u32 } else { 0 };
            let got = e.call(SSTRCMPI, 0, 0, &[pa, pb, n]);
            let errors: Vec<u32> = e.calls().iter().map(|c| c.args[0]).collect();
            if has_a && has_b {
                assert_eq!(
                    got as i32,
                    strnicmp(&na, &nb, n),
                    "SStrCmpI {na:02x?} {nb:02x?} {n}"
                );
                assert!(errors.is_empty());
            } else {
                assert_eq!(
                    (got, errors),
                    (pa.wrapping_sub(pb), vec![0x57]),
                    "SStrCmpI null"
                );
            }
        }
    }
}

/// A pool of strings in the emulator, refilled each case.
struct Pool {
    base: u64,
    at: u64,
}

impl Pool {
    fn put(&mut self, e: &mut Emu, bytes: &[u8]) -> u32 {
        let a = self.at;
        e.write(a, bytes);
        e.write(a + bytes.len() as u64, &[0]);
        self.at += (bytes.len() as u64 + 4) & !3;
        a as u32
    }
}

/// `CompareWhoResults` whole on pairs of records under random sort chains: every key 0 to 6, keys
/// above it, both directions and other non-zero ones; names and guilds that differ in case, length
/// and high bytes; levels across the sign boundary; and race, class and zone ids of `-1`, 0, the
/// table's largest, one past it and rows that are missing, with the locale picking one of eight
/// names a row holds.
#[test]
fn friends_who_compare() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let ra = e.alloc(0xa0);
    let rb = e.alloc(0xa0);
    let mut pool = Pool {
        base: e.alloc(0x2_0000),
        at: 0,
    };
    // The tables: index pointer, largest id; the name's offset in a row.
    let tables = [(0xc0e048u64, 0x2cu64), (0xc0def4, 0x14), (0xc0dee0, 0x44)];
    let mut g = Lcg(0x3e40_0001);
    for case in 0..6000 {
        pool.at = pool.base;
        let locale = (g.next() >> 16) % 8;
        e.write_u32(0xc0e080, locale);
        let mut names: Vec<Vec<Option<Vec<u8>>>> = Vec::new();
        let mut maxes = Vec::new();
        for &(table, off) in &tables {
            let max_id = [-1i32, 0, 3, 9][(g.next() >> 16) as usize % 4];
            let rows: Vec<Option<Vec<u8>>> = (0..=max_id)
                .map(|_| (g.next() >> 30 != 0).then(|| name(&mut g)))
                .collect();
            let index = pool.at;
            pool.at += 4 * (rows.len() as u64 + 1);
            for (i, row) in rows.iter().enumerate() {
                let at = match row {
                    None => 0,
                    Some(n) => {
                        let r = pool.at;
                        pool.at += 0x80;
                        for l in 0..8 {
                            let decoy = [b'q', b'0' + l as u8];
                            let p = if l == locale {
                                pool.put(&mut e, n)
                            } else {
                                pool.put(&mut e, &decoy)
                            };
                            e.write_u32(r + off + 4 * u64::from(l), p);
                        }
                        r as u32
                    }
                };
                e.write_u32(index + 4 * i as u64, at);
            }
            e.write_u32(table, index as u32);
            e.write_u32(table + 4, max_id as u32);
            names.push(rows);
            maxes.push(max_id);
        }
        let mut chain = [(0u32, 0u32); 7];
        for (k, c) in chain.iter_mut().enumerate() {
            let key = if g.next() >> 29 == 0 {
                [7u32, 8, 0xffff_ffff][(g.next() >> 16) as usize % 3]
            } else {
                (g.next() >> 16) % 7
            };
            let dir = [0u32, 0, 1, 0x100, 0x8000_0000][(g.next() >> 16) as usize % 5];
            *c = (key, dir);
            e.write_u32(0xc2817c + 8 * k as u64, key);
            e.write_u32(0xc28180 + 8 * k as u64, dir);
        }
        let id = |g: &mut Lcg, t: usize| -> u32 {
            match g.next() >> 29 {
                0 => 0xffff_ffff,
                1 => 0x8000_0000,
                2 => (maxes[t] + 1) as u32,
                3 => maxes[t].max(0) as u32,
                _ => (g.next() >> 16) % (maxes[t].max(0) as u32 + 1),
            }
        };
        let record = |g: &mut Lcg, e: &mut Emu, at: u64| {
            let r = WhoRecord {
                name: name(g),
                guild: name(g),
                level: [0u32, 1, 60, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff]
                    [(g.next() >> 16) as usize % 6],
                race: id(g, 2),
                class: id(g, 1),
                zone: id(g, 0),
            };
            e.write(at, &[0xa5; 0xa0]);
            e.write(at, &r.name);
            e.write(at + r.name.len() as u64, &[0]);
            e.write(at + 0x30, &r.guild);
            e.write(at + 0x30 + r.guild.len() as u64, &[0]);
            for (k, v) in [r.level, r.race, r.class, r.zone].into_iter().enumerate() {
                e.write_u32(at + 0x90 + 4 * k as u64, v);
            }
            r
        };
        let a = record(&mut g, &mut e, ra);
        let b = if g.next() >> 30 == 0 {
            let mut b = a.clone();
            b.name = b.name.to_ascii_uppercase();
            e.write(rb, &e.read(ra, 0xa0));
            e.write(rb, &b.name);
            b
        } else {
            record(&mut g, &mut e, rb)
        };
        let got = e.call(COMPARE_WHO, 0, 0, &[ra as u32, rb as u32]) as i32;
        let rows: Vec<Vec<Option<&[u8]>>> = names
            .iter()
            .map(|t| t.iter().map(|r| r.as_deref()).collect())
            .collect();
        let table = |t: usize| NameTable {
            max_id: maxes[t],
            rows: &rows[t],
        };
        let want = compare_who(&a, &b, &chain, &table(0), &table(1), &table(2));
        assert_eq!(got, want, "case {case} {chain:?} {a:?} {b:?}");
    }
}

/// Replaces a console writer with code that logs each call in `0x80` bytes from `log + 0x20`,
/// counted at `log`: the format, the colour (`-1` for `ConsolePrintf`, which has none) and 64
/// bytes of the string argument, read while the caller's buffer holds it.
fn console_island(e: &mut Emu, at: u64, log: u64, printf: bool) {
    let cnt = log as u32;
    let mut c = vec![0x56, 0x57, 0xa1];
    c.extend(cnt.to_le_bytes());
    c.extend([0xc1, 0xe0, 0x07, 0x05]);
    c.extend((cnt + 0x20).to_le_bytes());
    c.extend([0x89, 0xc7, 0x8b, 0x4c, 0x24, 0x0c, 0x89, 0x0f]);
    let mut skip = None;
    if printf {
        c.extend([
            0xc7, 0x47, 0x04, 0xff, 0xff, 0xff, 0xff, 0x8b, 0x74, 0x24, 0x10,
        ]);
    } else {
        c.extend([0x8b, 0x4c, 0x24, 0x10, 0x89, 0x4f, 0x04, 0x81, 0x3f]);
        c.extend(0x85dc34u32.to_le_bytes());
        c.extend([0x75, 0x00]);
        skip = Some(c.len());
        c.extend([0x8b, 0x74, 0x24, 0x14]);
    }
    c.extend([0x8d, 0x7f, 0x08, 0xb9, 0x40, 0x00, 0x00, 0x00, 0xf3, 0xa4]);
    if let Some(k) = skip {
        c[k - 1] = (c.len() - k) as u8;
    }
    c.extend([0xff, 0x05]);
    c.extend(cnt.to_le_bytes());
    c.extend([0x5f, 0x5e, 0xc3]);
    let island = e.alloc(0x80);
    e.write(island, &c);
    let rel = (island as u32).wrapping_sub(at as u32 + 5);
    let mut jmp = vec![0xe9];
    jmp.extend(rel.to_le_bytes());
    e.write(at, &jmp);
}

/// `ReverseWhoisResponseHandler` whole on counts of `-1`, 0, negative and 1 to 3 accounts, each
/// with `-1` to 4 characters and an index inside, outside and negative; names empty, long and
/// past their 64 and 48 byte buffers; and packets cut anywhere: every console line in order.
#[test]
fn friends_reverse_whois() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let log = r.e.alloc(0x8000);
    console_island(&mut r.e, CONSOLE_WRITE, log, false);
    console_island(&mut r.e, CONSOLE_PRINTF, log, true);
    let mut g = Lcg(0x4e01_0001);
    let text = |g: &mut Lcg, cap: usize| -> Vec<u8> {
        let n = [0usize, 1, 5, cap - 1, cap, cap + 3][(g.next() >> 16) as usize % 6];
        (0..n)
            .map(|_| b'a' + ((g.next() >> 16) % 26) as u8)
            .collect()
    };
    for case in 0..3000 {
        let count = [-1i32, 0, -5, 1, 2, 3][(g.next() >> 16) as usize % 6];
        let mut bytes = count.to_le_bytes().to_vec();
        for _ in 0..count.max(0) {
            bytes.extend(text(&mut g, 0x40));
            bytes.push(0);
            let n = [-1i32, 0, 1, 2, 4][(g.next() >> 16) as usize % 5];
            let index = [-1i32, 0, 1, 3, 9][(g.next() >> 16) as usize % 5];
            bytes.extend(n.to_le_bytes());
            bytes.extend(index.to_le_bytes());
            for _ in 0..n.max(0) {
                bytes.extend(text(&mut g, 0x30));
                bytes.push(0);
            }
        }
        if g.next() >> 30 == 0 {
            let cut = (g.next() >> 8) as usize % (bytes.len() + 1);
            bytes.truncate(cut);
        }
        r.load_store(&bytes);
        r.e.write_u32(log, 0);
        let ret = r.e.call(REVERSE_WHOIS, 0, 0, &[0x1fe, r.store as u32]);
        let got: Vec<WhoisLine> = (0..u64::from(r.e.read_u32(log)))
            .map(|i| {
                let a = log + 0x20 + 0x80 * i;
                let s: Vec<u8> =
                    r.e.read(a + 8, 0x40)
                        .into_iter()
                        .take_while(|&c| c != 0)
                        .collect();
                match (r.e.read_u32(a), r.e.read_u32(a + 4)) {
                    (0x85dc58, 3) => WhoisLine::Failed,
                    (0x85dc4c, 4) => WhoisLine::NotFound,
                    (0x85dc3c, 0xffff_ffff) => WhoisLine::Account(s),
                    (0x85dc34, colour) => WhoisLine::Character { name: s, colour },
                    other => panic!("console call {other:x?}"),
                }
            })
            .collect();
        let want = reverse_whois(&mut plain(&bytes));
        assert_eq!(ret, 1);
        assert_eq!(got, want, "case {case} {bytes:02x?}");
    }
}
