//! The add-on record lookups against the exe, each run whole
//! (`systems/ui/addoninfo.md#finding-a-record-by-name`): `0x51e050` (notes), `0x51e180` (URL)
//! and `0x51e3e0` (update byte), over an add-on hash and per-record metadata hashes the test
//! builds in memory, with `SStrHashHT` (`0x64b3f0`) and `SStrCmpI` (`0x64a4c0`) run from the image.
//!
//! A bucket is 12 bytes: the link offset into a record (`+0`), unused (`+4`) and the first record
//! or an odd end tag (`+8`). A record keeps its hash at `+0` and its next link at `+offset + 4`.
//! Records share buckets (the mask is 3), so the walk past a non-matching record runs.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_storm::str_hash_ht;
use benilla_re_ui::addon_lookup::{notes, update_byte, url, AddOnRecord};

const NOTES: u64 = 0x51e050;
const URL: u64 = 0x51e180;
const UPDATE: u64 = 0x51e3e0;
const BUCKETS: u64 = 0xbe1b7c;
const MASK: u64 = 0xbe1b84;
const LINK: u32 = 4;

fn cstr(e: &mut Emu, s: &[u8]) -> u32 {
    let va = e.alloc(s.len() as u64 + 1);
    e.write(va, s);
    e.write(va + s.len() as u64, &[0]);
    va as u32
}

/// Builds a hash of `entries` (key, node), each node's hash at `+0`, chained in each bucket in
/// the order given, and answers the bucket array. A mask of `u32::MAX` has no buckets.
fn build_hash(e: &mut Emu, mask: u32, entries: &[(&[u8], u64)]) -> u32 {
    if mask == u32::MAX {
        return 0;
    }
    let n = u64::from(mask) + 1;
    let buckets = e.alloc(12 * n);
    let mut tails: Vec<Option<u64>> = vec![None; n as usize];
    for b in 0..n {
        let at = buckets + 12 * b;
        e.write_u32(at, LINK);
        e.write_u32(at + 4, 0);
        e.write_u32(at + 8, (at as u32 + 4) | 1);
    }
    for &(key, node) in entries {
        let hash = str_hash_ht(key);
        e.write_u32(node, hash);
        let b = u64::from(hash & mask);
        let at = buckets + 12 * b;
        let end = (at as u32 + 4) | 1;
        e.write_u32(node + u64::from(LINK) + 4, end);
        match tails[b as usize] {
            None => e.write_u32(at + 8, node as u32),
            Some(prev) => e.write_u32(prev + u64::from(LINK) + 4, node as u32),
        }
        tails[b as usize] = Some(node);
    }
    buckets as u32
}

fn read_cstr(e: &Emu, va: u32) -> Option<Vec<u8>> {
    if va == 0 {
        return None;
    }
    let mut out = Vec::new();
    let mut p = u64::from(va);
    loop {
        let b = e.read(p, 1)[0];
        if b == 0 {
            return Some(out);
        }
        out.push(b);
        p += 1;
    }
}

/// Writes the records and the add-on hash; a `mask` of `u32::MAX` leaves the hash empty.
fn install(e: &mut Emu, records: &[AddOnRecord], mask: u32) {
    let mut entries = Vec::new();
    for r in records {
        let rec = e.alloc(0x200);
        let name = cstr(e, r.name);
        e.write_u32(rec + 0x14, name);
        e.write_u32(rec + 0x24, r.security);
        let url = match r.url {
            Some(u) => cstr(e, u),
            None => 0,
        };
        e.write_u32(rec + 0x30, url);
        e.write(rec + 0xc0, &[r.update]);
        let mut meta = Vec::new();
        for &(k, v) in r.metadata {
            let node = e.alloc(0x40);
            let key = cstr(e, k);
            let value = cstr(e, v);
            e.write_u32(node + 0x14, key);
            e.write_u32(node + 0x18, value);
            meta.push((k, node));
        }
        let meta_mask = if r.metadata.is_empty() { u32::MAX } else { 1 };
        let meta_buckets = build_hash(e, meta_mask, &meta);
        e.write_u32(rec + 0xb4, meta_buckets);
        e.write_u32(rec + 0xbc, meta_mask);
        entries.push((r.name, rec));
    }
    let buckets = build_hash(e, mask, &entries);
    e.write_u32(BUCKETS, buckets);
    e.write_u32(MASK, mask);
}

const RECORDS: [AddOnRecord<'static>; 6] = [
    AddOnRecord {
        name: b"Atlas",
        metadata: &[(b"Title", b"Atlas"), (b"Notes", b"Maps of instances")],
        url: Some(b"http://atlas.example/"),
        security: 1,
        update: 0,
    },
    AddOnRecord {
        name: b"Bongos",
        metadata: &[(b"NOTES", b"Bars"), (b"Author", b"Tuller")],
        url: Some(b""),
        security: 0,
        update: 1,
    },
    AddOnRecord {
        name: b"CT_RaidAssist",
        metadata: &[(b"Title", b"CTRA")],
        url: Some(b"http://ctmod.example/"),
        security: 2,
        update: 1,
    },
    AddOnRecord {
        name: b"Decursive",
        metadata: &[],
        url: None,
        security: 1,
        update: 0x80,
    },
    AddOnRecord {
        name: b"eCastingBar",
        metadata: &[(b"Notes", b"")],
        url: Some(b"x"),
        security: 0,
        update: 0,
    },
    AddOnRecord {
        name: b"FuBar_Clock",
        metadata: &[(b"notes", b"A clock"), (b"Title", b"Clock")],
        url: Some(b"http://fubar.example/clock"),
        security: 1,
        update: 0xff,
    },
];

/// Every record's name in its own case, upper and lower case, and names no record holds.
fn probes() -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    for r in &RECORDS {
        out.push(r.name.to_vec());
        out.push(r.name.to_ascii_uppercase());
        out.push(r.name.to_ascii_lowercase());
    }
    for n in [
        &b"Atla"[..],
        b"Atlass",
        b"",
        b"Bongos ",
        b"Unknown",
        b"CT/RaidAssist",
    ] {
        out.push(n.to_vec());
    }
    out
}

fn run(f: u64, check: impl Fn(&mut Emu, &[u8], u32)) {
    let Some(wow) = Wow::load() else {
        return;
    };
    for mask in [3u32, 0, u32::MAX] {
        let mut e = Emu::new(&wow);
        install(&mut e, &RECORDS, mask);
        for name in probes() {
            let arg = cstr(&mut e, &name);
            let got = e.call(f, arg, 0, &[]);
            check(&mut e, &name, got);
        }
    }
}

/// The records a lookup can find: none while the hash has no buckets.
fn records_for(e: &Emu) -> &'static [AddOnRecord<'static>] {
    if e.read_u32(MASK) == u32::MAX {
        &[]
    } else {
        &RECORDS
    }
}

#[test]
fn addon_notes_lookup_difftest() {
    run(NOTES, |e, name, got| {
        let want = notes(records_for(e), name).map(<[u8]>::to_vec);
        assert_eq!(
            read_cstr(e, got),
            want,
            "{:?}",
            String::from_utf8_lossy(name)
        );
    });
}

#[test]
fn addon_url_lookup_difftest() {
    run(URL, |e, name, got| {
        let want = url(records_for(e), name).map(<[u8]>::to_vec);
        assert_eq!(
            read_cstr(e, got),
            want,
            "{:?}",
            String::from_utf8_lossy(name)
        );
    });
}

#[test]
fn addon_update_flag_lookup_difftest() {
    run(UPDATE, |e, name, got| {
        let want = update_byte(records_for(e), name);
        assert_eq!(got as u8, want, "{:?}", String::from_utf8_lossy(name));
    });
}
