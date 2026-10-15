//! The realm list fill (`systems/net/logonlink.md#realm-record-fill`): `HandleRealmList`,
//! `HandleRealmListResult` and `IsFileNameChar` run whole beside [`RealmHolder`].

mod common;

use benilla_re_difftest::Emu;
use benilla_re_net::{is_file_name_char, DataStore, Fetch, RealmHolder, REALM_RECORD};
use common::{Lcg, Wow};

const HANDLE_REALM_LIST: u64 = 0x5b2230;
const HANDLE_RESULT: u64 = 0x5b2560;
const IS_FILE_NAME_CHAR: u64 = 0x5b24b0;

struct Rig<'a> {
    e: Emu<'a>,
    login: u64,
    holder: u64,
    store: u64,
    buf: u64,
    arena: u64,
    slot5: u64,
}

impl<'a> Rig<'a> {
    /// A logon object whose `+0x1f0` is a holder with a stub in slot 5, the allocator calls on
    /// an arena, and a plain store.
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let login = e.alloc(0x200);
        let holder = e.alloc(0x20);
        let vt = e.alloc(0x40);
        let slot5 = e.alloc(0x10);
        let store = e.alloc(0x20);
        let buf = e.alloc(0x4000);
        let arena = e.alloc(0x200000);
        e.write_u32(login + 0x1f0, holder as u32);
        e.write_u32(holder, vt as u32);
        e.write_u32(vt + 0x14, slot5 as u32);
        e.stub(slot5, 0, 1);
        e.stub_alloc_sized(0x6462e0, arena, arena + 0x200000, 0, 4);
        e.stub_realloc_sized(0x646320, 5);
        e.stub(0x646430, 1, 4);
        Rig {
            e,
            login,
            holder,
            store,
            buf,
            arena,
            slot5,
        }
    }

    /// Sets the holder to `h`, its records in fresh arena memory, and the store to `bytes` with
    /// the cursor at `read`.
    fn load(&mut self, h: &RealmHolder, bytes: &[u8], read: u32) {
        self.e.write(self.arena, &vec![0u8; 0x200000]);
        let data = if h.records.is_empty() {
            0
        } else {
            self.arena as u32
        };
        let flat: Vec<u8> = h.records.iter().flatten().copied().collect();
        self.e.write(self.arena, &flat);
        self.e.reset_bump(self.arena + 0x100000);
        self.e.write_u32(self.holder + 4, h.alloc);
        self.e.write_u32(self.holder + 8, h.count);
        self.e.write_u32(self.holder + 0xc, data);
        self.e.write(self.holder + 0x10, &h.tail);
        self.e.write(self.buf, bytes);
        let len = bytes.len() as u32;
        for (i, v) in [0x7ff9e4, self.buf as u32, 0, len.max(1), len, read]
            .into_iter()
            .enumerate()
        {
            self.e.write_u32(self.store + 4 * i as u64, v);
        }
        self.e.clear_calls();
    }

    fn compare(&self, h: &RealmHolder, status: u32, read: u32, ctx: &str) {
        let f = |o: u64| self.e.read_u32(self.holder + o);
        assert_eq!((f(4), f(8)), (h.alloc, h.count), "{ctx}: alloc, count");
        assert_eq!(
            self.e.read(self.holder + 0x10, 2),
            h.tail.to_vec(),
            "{ctx}: tail"
        );
        if h.count != 0 {
            let data = u64::from(f(0xc));
            for (i, want) in h.records.iter().enumerate() {
                let got = self.e.read(data + (i * REALM_RECORD) as u64, REALM_RECORD);
                assert_eq!(got, want.to_vec(), "{ctx}: record {i}");
            }
        }
        let calls: Vec<u32> = self
            .e
            .calls()
            .iter()
            .filter(|c| c.addr == self.slot5)
            .map(|c| c.args[0])
            .collect();
        assert_eq!(calls, vec![status], "{ctx}: slot 5");
        assert_eq!(self.e.read_u32(self.store + 0x14), read, "{ctx}: cursor");
    }
}

fn fill(g: &mut Lcg, n: usize) -> Vec<u8> {
    (0..n).map(|_| (g.next() >> 16) as u8).collect()
}

/// A realm name: printable text with forbidden characters, spaces and dots, often at the end,
/// now and then a byte at or above `0x80` or below `0x20`, and sometimes 256 bytes or more.
fn name(g: &mut Lcg, cap: usize) -> Vec<u8> {
    const PICK: &[u8] = b"ab Z.9 \"*/:<>?\\|.  ..~}{!";
    let n = match g.next() >> 28 {
        0 => cap + (g.next() >> 28) as usize % 3,
        1 => 0,
        _ => (g.next() >> 24) as usize % 20,
    };
    let mut s: Vec<u8> = (0..n)
        .map(|_| match g.next() >> 28 {
            0 => 0x80 | (g.next() >> 24) as u8,
            1 => 1 + (g.next() >> 24) as u8 % 0x1f,
            _ => PICK[(g.next() >> 16) as usize % PICK.len()],
        })
        .collect();
    s.push(0);
    s
}

/// A realm list body after the size field: the count, the records and the tail, then a cut or a
/// padding now and then.
fn body(g: &mut Lcg) -> Vec<u8> {
    let count = [0u8, 1, 2, 3, 6][(g.next() >> 16) as usize % 5];
    let mut v = vec![count];
    for _ in 0..count {
        v.extend(fill(g, 5));
        v.extend(name(g, 256));
        v.extend(name(g, 32));
        let pop: u32 = match g.next() >> 29 {
            0 => 0x4416_0000,
            1 => 0x4348_0000,
            2 => 0x43c8_0000,
            _ => g.next(),
        };
        v.extend(pop.to_le_bytes());
        v.extend(fill(g, 3));
    }
    v.extend(fill(g, 2));
    match g.next() >> 28 {
        // Short by the tail and one to three bytes of the last record, so its reads run past the
        // end after its population and name are read.
        15 if count > 0 => {
            let n = v.len() - 3 - (g.next() >> 30) as usize % 3;
            v.truncate(n);
        }
        0..=2 => {
            let n = 1 + (g.next() as usize >> 8) % v.len();
            v.truncate(n);
        }
        3 => {
            let n = 1 + (g.next() >> 24) as usize % 4;
            v.extend(fill(g, n));
        }
        _ => {}
    }
    v
}

/// A holder as the last list left it: empty, or with records.
fn holder(g: &mut Lcg) -> RealmHolder {
    let n = if g.next() >> 31 == 0 {
        0
    } else {
        1 + (g.next() >> 30)
    };
    RealmHolder {
        alloc: n,
        count: n,
        records: (0..n)
            .map(|_| {
                let mut r = [0u8; REALM_RECORD];
                for b in r.iter_mut() {
                    *b = (g.next() >> 16) as u8;
                }
                r
            })
            .collect(),
        tail: [(g.next() >> 8) as u8, (g.next() >> 16) as u8],
    }
}

fn plain(bytes: &[u8], read: u32) -> DataStore {
    let mut m = DataStore::new(Fetch::Heap);
    m.data = bytes.to_vec();
    m.alloc = (bytes.len() as u32).max(1);
    m.size = bytes.len() as u32;
    m.read = read;
    m
}

/// `HandleRealmList` whole: lists of 0 to 6 records with names that need cleaning, the three
/// populations it rewrites, names past their capacity, cut and padded lists, holders that held
/// records, and the null store and the unfinished cursor it hands to `HandleRealmListResult`.
#[test]
fn realm_list_fill() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut g = Lcg(0x4ea1_3501);
    let mut r = Rig::new(&wow);
    for case in 0..3000 {
        let mut h = holder(&mut g);
        let pre = (g.next() >> 29) as usize;
        let mut bytes = fill(&mut g, pre);
        bytes.extend(body(&mut g));
        let mode = g.next() >> 29;
        let read = if mode == 0 { 0xffff_ffff } else { pre as u32 };
        r.load(&h, &bytes, read);
        let null = mode == 1;
        let arg = if null { 0 } else { r.store as u32 };
        r.e.call(HANDLE_REALM_LIST, r.login as u32, 0, &[arg]);
        let mut m = plain(&bytes, read);
        let status = h.handle_realm_list(if null { None } else { Some(&mut m) });
        let ctx = format!("case {case} mode {mode} {bytes:02x?}");
        r.compare(&h, status, m.read, &ctx);
    }
}

/// `HandleRealmListResult` whole, with the results its callers pass (2 and 4) and with 0 on
/// generated lists, which no caller passes but which runs its own copy of the fill.
#[test]
fn realm_list_result() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut g = Lcg(0x4e50_0001);
    let mut r = Rig::new(&wow);
    for case in 0..3000 {
        let mut h = holder(&mut g);
        let result = [0u32, 0, 0, 2, 4, 7][(g.next() >> 16) as usize % 6];
        let bytes = body(&mut g);
        r.load(&h, &bytes, 0);
        r.e.call(HANDLE_RESULT, r.holder as u32, 0, &[result, r.store as u32]);
        let mut m = plain(&bytes, 0);
        let status = h.handle_result(result, Some(&mut m));
        let ctx = format!("case {case} result {result} {bytes:02x?}");
        r.compare(&h, status, m.read, &ctx);
    }
}

/// `IsFileNameChar` over every byte and both values of its flag, and flag values with only upper
/// bits set, which it compares as a byte.
#[test]
fn realm_name_char() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for c in 0..=255u8 {
        for keep in [0u32, 1, 2, 0xff, 0x100, 0xffff_ff00] {
            let got = e.call(IS_FILE_NAME_CHAR, 0, 0, &[u32::from(c) | 0x1234_5600, keep]) & 0xff;
            let want = is_file_name_char(c, keep as u8);
            assert_eq!(got, u32::from(want), "char {c:#04x} flag {keep:#x}");
        }
    }
}
