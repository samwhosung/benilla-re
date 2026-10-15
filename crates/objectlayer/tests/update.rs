//! The update packet's block readers (`systems/objectlayer/update.md`), each run whole with the
//! object manager's calls replaced: the values skip, the out-of-range and near-object lists and
//! the values block's dispatch.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_objectlayer::datastore::DataStore;
use benilla_re_objectlayer::update::{
    flagged_guid, near_objects, out_of_range, skip_values_block, values_block, ValuesBlock,
};
use common::next;

const SKIP_VALUES: u64 = 0x465bb0;
const OUT_OF_RANGE: u64 = 0x465ec0;
const NEAR_OBJECTS: u64 = 0x467230;
const VALUES_BLOCK: u64 = 0x467160;
const RELEASE: u64 = 0x464920;
const FIND_OR_REACTIVATE: u64 = 0x464530;
const REACTIVATE: u64 = 0x4644f0;
const APPLY_VALUES: u64 = 0x466590;
const LOG_LEVEL: u64 = 0x44cb20;
const MGR_PTR_GLOBAL: u64 = 0xb4_1414;

struct Rig<'a> {
    e: Emu<'a>,
    store: u64,
    buf: u64,
    mgr: u64,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let store = e.alloc(0x20);
        let buf = e.alloc(0x4000);
        let mgr = e.alloc(0x100);
        e.write_u32(MGR_PTR_GLOBAL, mgr as u32);
        Rig { e, store, buf, mgr }
    }

    fn load(&mut self, bytes: &[u8]) {
        self.e.write(self.buf, bytes);
        let len = bytes.len() as u32;
        for (i, v) in [0x7ff9e4u32, self.buf as u32, 0, len.max(1), len, 0]
            .into_iter()
            .enumerate()
        {
            self.e.write_u32(self.store + 4 * i as u64, v);
        }
        self.e.clear_calls();
    }

    fn cursor(&self) -> u32 {
        self.e.read_u32(self.store + 0x14)
    }

    fn set_active(&mut self, g: u64) {
        self.e.write_u32(self.mgr + 0xc0, g as u32);
        self.e.write_u32(self.mgr + 0xc4, (g >> 32) as u32);
    }
}

fn pack(g: u64) -> Vec<u8> {
    let mut mask = 0u8;
    let mut v = Vec::new();
    for k in 0..8 {
        let b = (g >> (8 * k)) as u8;
        if b != 0 {
            mask |= 1 << k;
            v.push(b);
        }
    }
    let mut out = vec![mask];
    out.extend(v);
    out
}

/// A GUID of each kind the passes split on: top nibbles 6 and 8, the server's 0xF and 1, 0, and
/// any.
fn guid(s: &mut u32) -> u64 {
    let low = u64::from(next(s) >> (next(s) >> 28)) | (u64::from(next(s) & 0xff) << 32);
    match next(s) >> 29 {
        0 => 0,
        1 => (6 << 60) | low,
        2 => (8 << 60) | low,
        3 => (0xf120u64 << 48) | low,
        4 => low,
        _ => (u64::from(next(s)) << 32) | u64::from(next(s)),
    }
}

/// A values block: a count byte up to 42, the mask words, and a dword for each set bit.
fn values(s: &mut u32) -> Vec<u8> {
    let n = match next(s) >> 29 {
        0 => 0,
        1 => 41,
        2 => 42,
        _ => (next(s) >> 16) % 8,
    } as usize;
    let mut mask = vec![0u32; n];
    for w in mask.iter_mut() {
        *w = match next(s) >> 30 {
            0 => 0,
            1 => next(s),
            _ => 1 << (next(s) >> 27),
        };
    }
    let mut v = vec![n as u8];
    for w in &mask {
        v.extend(w.to_le_bytes());
    }
    let mut last = if n == 42 { mask[41] } else { 0 };
    for i in 0..32 * n {
        let word = if i >> 5 == 41 { last } else { mask[i >> 5] };
        if word & (1 << (i & 31)) != 0 {
            // With 42 words the last word is the value slot: keep it sparse, so the walk ends.
            let val = if n == 42 {
                1 << (next(s) >> 27)
            } else {
                next(s)
            };
            v.extend(val.to_le_bytes());
            if n == 42 {
                last = val;
            }
        }
    }
    v
}

/// `SkipValuesBlock` whole on counts from 0 to 42, sparse and full masks, and stores cut inside
/// the values: the cursor after it.
#[test]
fn update_skip_values() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let mut s = 0x5c1b_0001u32;
    for case in 0..4000 {
        let mut bytes = values(&mut s);
        let n = usize::from(bytes[0]);
        let head = 1 + 4 * n;
        if next(&mut s) >> 30 == 0 && bytes.len() > head {
            let cut = head + (next(&mut s) as usize) % (bytes.len() - head);
            bytes.truncate(cut);
        }
        bytes.extend([0x5a; 3]);
        r.load(&bytes);
        r.e.call(SKIP_VALUES, r.store as u32, 0, &[]);
        let mut m = DataStore::new(&bytes);
        skip_values_block(&mut m);
        assert_eq!(r.cursor(), m.pos, "case {case} n {n}");
    }
}

/// `ProcessOutOfRangeObjects` whole on lists of 0 to 8 GUIDs of every kind the split knows,
/// with the active player among them or not: the GUIDs released in order, the cursor, and the
/// return of 0.
#[test]
fn update_out_of_range() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    r.e.stub(RELEASE, 0, 2);
    let mut s = 0x0a7e_0001u32;
    for case in 0..4000 {
        let n = (next(&mut s) >> 16) % 9;
        let guids: Vec<u64> = (0..n).map(|_| guid(&mut s)).collect();
        let active = match (next(&mut s) >> 30, guids.is_empty()) {
            (0 | 1, false) => guids[(next(&mut s) >> 16) as usize % guids.len()],
            _ => guid(&mut s),
        };
        let mut bytes = n.to_le_bytes().to_vec();
        for &g in &guids {
            bytes.extend(pack(g));
        }
        let tail = (next(&mut s) >> 30) as usize;
        bytes.extend(vec![0x5a; tail]);
        r.set_active(active);
        r.load(&bytes);
        let ret = r.e.call(OUT_OF_RANGE, r.store as u32, 0, &[]);
        let got: Vec<u64> =
            r.e.calls()
                .iter()
                .map(|c| u64::from(c.args[0]) | (u64::from(c.args[1]) << 32))
                .collect();
        let mut m = DataStore::new(&bytes);
        let want = out_of_range(&mut m, active);
        let ctx = format!("case {case} {guids:x?} active {active:#x}");
        assert_eq!(got, want, "{ctx}: released");
        assert_eq!((ret, r.cursor()), (0, m.pos), "{ctx}: return and cursor");
    }
}

/// `ProcessNearObjects` whole in both passes on lists of 0 to 8 GUIDs of every kind, the active
/// player among them or not: the GUIDs passed on, the count returned and the cursor.
#[test]
fn update_near_objects() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    r.e.stub(REACTIVATE, 0, 2);
    let mut s = 0x4ea2_0001u32;
    for case in 0..4000 {
        let n = (next(&mut s) >> 16) % 9;
        let guids: Vec<u64> = (0..n).map(|_| guid(&mut s)).collect();
        let active = match (next(&mut s) >> 30, guids.is_empty()) {
            (0 | 1, false) => guids[(next(&mut s) >> 16) as usize % guids.len()],
            _ => guid(&mut s),
        };
        let flag = next(&mut s) >> 31 == 0;
        let mut bytes = n.to_le_bytes().to_vec();
        for &g in &guids {
            bytes.extend(pack(g));
        }
        r.set_active(active);
        r.load(&bytes);
        let ret = r.e.call(NEAR_OBJECTS, r.store as u32, u32::from(flag), &[]);
        let got: Vec<u64> =
            r.e.calls()
                .iter()
                .map(|c| u64::from(c.args[0]) | (u64::from(c.args[1]) << 32))
                .collect();
        let mut m = DataStore::new(&bytes);
        let want = near_objects(&mut m, active, flag);
        let ctx = format!("case {case} {guids:x?} active {active:#x} flag {flag}");
        assert_eq!(got, want, "{ctx}: passed on");
        assert_eq!(
            (ret, r.cursor()),
            (want.len() as u32, m.pos),
            "{ctx}: count and cursor"
        );
    }
}

/// `ObjectValuesBlock` whole in both passes on GUIDs of every kind, for no object, a live one
/// and a released one the lookup reactivates: the lookup's GUID, the values written with the
/// object's own GUID, slot 2, and the skip's cursor for a block it does not handle.
#[test]
fn update_values_block() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = Rig::new(&wow);
    let obj = r.e.alloc(0x40);
    let vt = r.e.alloc(0x20);
    let fields = r.e.alloc(0x20);
    let slot2 = r.e.alloc(0x10);
    let log = r.e.alloc(0x10);
    r.e.write_u32(obj, vt as u32);
    r.e.write_u32(obj + 8, fields as u32);
    r.e.write_u32(vt + 8, slot2 as u32);
    r.e.stub(slot2, 0, 0);
    r.e.stub(APPLY_VALUES, 0, 4);
    r.e.stub(LOG_LEVEL, 1, 1);
    // `ClntObjMgrFindOrReactivate(ecx = &reactivated, guid)`: logs the guid; an object for an
    // odd low dword, reactivated when bit 1 is set too.
    let l = log as u32;
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xa3];
    c.extend(l.to_le_bytes());
    c.extend([0x8b, 0x54, 0x24, 0x08, 0x89, 0x15]);
    c.extend((l + 4).to_le_bytes());
    c.extend([
        0xa8, 0x01, 0x74, 0x11, 0x89, 0xc2, 0xd1, 0xea, 0x83, 0xe2, 0x01, 0x89, 0x11, 0xb8,
    ]);
    c.extend((obj as u32).to_le_bytes());
    c.extend([0xc2, 0x08, 0x00, 0x31, 0xc0, 0xc2, 0x08, 0x00]);
    let island = r.e.alloc(0x60);
    r.e.write(island, &c);
    let mut j = vec![0xe9];
    j.extend(
        (island as u32)
            .wrapping_sub(FIND_OR_REACTIVATE as u32 + 5)
            .to_le_bytes(),
    );
    r.e.write(FIND_OR_REACTIVATE, &j);
    let mut s = 0x7a1b_0001u32;
    for case in 0..4000 {
        let g = guid(&mut s);
        let flag = if next(&mut s) >> 30 == 0 {
            flagged_guid(g)
        } else {
            next(&mut s) >> 31 == 0
        };
        let own = (u64::from(next(&mut s)) << 32) | u64::from(next(&mut s));
        r.e.write(fields, &own.to_le_bytes());
        let mut bytes = pack(g);
        bytes.extend(values(&mut s));
        bytes.extend([0x5a; 2]);
        r.load(&bytes);
        r.e.write(log, &[0; 8]);
        r.e.call(VALUES_BLOCK, r.store as u32, u32::from(flag), &[]);
        let lookup = |x: u64| (x & 1 == 1).then_some(x & 2 == 2);
        let mut m = DataStore::new(&bytes);
        let (got_guid, want) = values_block(&mut m, flag, lookup);
        let ctx = format!("case {case} guid {g:#x} flag {flag}");
        assert_eq!(got_guid, g, "{ctx}: the packed guid");
        let calls: Vec<(u64, [u32; 4])> =
            r.e.calls()
                .iter()
                .map(|c| (c.addr, [c.args[0], c.args[1], c.args[2], c.args[3]]))
                .collect();
        let logged = u64::from(r.e.read_u32(log)) | (u64::from(r.e.read_u32(log + 4)) << 32);
        match want {
            ValuesBlock::OtherPass => {
                assert!(calls.is_empty(), "{ctx}: {calls:x?}");
                assert_eq!(logged, 0, "{ctx}: no lookup");
                assert_eq!(r.cursor(), m.pos, "{ctx}: the skip");
            }
            ValuesBlock::Unknown => {
                assert_eq!(calls.len(), 1, "{ctx}: {calls:x?}");
                assert_eq!(logged, g, "{ctx}: the lookup");
                assert_eq!(r.cursor(), m.pos, "{ctx}: the skip");
            }
            ValuesBlock::Applied { reactivated } => {
                assert_eq!(logged, g, "{ctx}: the lookup");
                let mut want_calls = vec![
                    (LOG_LEVEL, calls[0].1),
                    (APPLY_VALUES, [own as u32, (own >> 32) as u32, 0, 0]),
                ];
                if reactivated {
                    want_calls.push((slot2, calls.last().unwrap().1));
                }
                assert_eq!(calls, want_calls, "{ctx}: calls");
            }
        }
    }
}
