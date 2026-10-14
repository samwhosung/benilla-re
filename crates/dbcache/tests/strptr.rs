//! The `char *` field record decoders against the exe, each whole: Creature, GameObject, ItemStats and
//! NPCText `T::Read`.
//! Each synthesizes an `SMSG_*_QUERY_RESPONSE` record buffer in wire (= read) order, stubs `SStrDupA`
//! (0x64a620) to a deterministic counting bump allocator, drives the binary decoder over a `CDataStore`,
//! and asserts the decoded struct bytes equal our transcription on the same buffer. char\* fields compare
//! by the stored (stubbed) pointer; `GetFloat`/`GetInt32` fields by raw bits. Lengths include 0 (~25%) so
//! both the all-non-empty paths and the empty-source and empty-stores-0 paths are run.

mod common;

use benilla_re_dbcache::datastore::{DataStore, Dup};
use benilla_re_dbcache::decode_strptr::{creature, gameobject, itemstats, npctext};
use benilla_re_difftest::{Emu, Wow};
use common::{make_datastore, next, wire_cstring};

const CREATURE: u64 = 0x7c8120; // __thiscall(ecx=record, [esp+4]=CDataStore) ret 4
const CREATURE_SZ: usize = 0x34;
const GAMEOBJECT: u64 = 0x7c87c0;
const GAMEOBJECT_SZ: usize = 0x7c;
const ITEMSTATS: u64 = 0x7c9640;
const ITEMSTATS_SZ: usize = 0x1d4;
const NPCTEXT: u64 = 0x7c9fa0;
const NPCTEXT_SZ: usize = 0x140;

const SSTRDUPA: u64 = 0x64a620; // SStrDupA(src, FILE, LINE) ret 0xc — stubbed to a counting bump allocator
const DUP_BASE: u32 = 0x1000_0000;
const DUP_STRIDE: u32 = 0x100;

/// A wire C-string length, 0 (empty) ~25% of the time to exercise the empty/reuse paths.
fn slen(s: &mut u32) -> usize {
    let r = next(s);
    if r.is_multiple_of(4) {
        0
    } else {
        1 + ((r >> 8) % 12) as usize
    }
}

/// Append a fuzzed 4-byte field (GetInt32 / GetFloat — diffed by raw bits).
fn push4(buf: &mut Vec<u8>, s: &mut u32) {
    buf.extend_from_slice(&next(s).to_le_bytes());
}

/// Append one fuzzed NUL-terminated wire C-string (possibly empty).
fn push_str(buf: &mut Vec<u8>, s: &mut u32) {
    let len = slen(s);
    buf.extend_from_slice(&wire_cstring(s, len));
}

/// Run one case: build the binary's decoded struct in the emu, run our decoder on the same buffer, diff.
fn diff_case(
    wow: &Wow,
    addr: u64,
    sz: usize,
    buf: &[u8],
    ours_fn: impl Fn(&mut DataStore, &mut [u8], &mut Dup),
) -> (Vec<u8>, Vec<u8>) {
    let mut e = Emu::new(wow);
    e.stub_alloc(SSTRDUPA, DUP_BASE as u64, DUP_STRIDE as u64, 3);
    let ds = make_datastore(&mut e, buf);
    let rec = e.alloc(sz as u64);
    e.write(rec, &vec![0u8; sz]);
    e.call(addr, rec as u32, 0, &[ds as u32]);
    let bin = e.read(rec, sz);

    let mut ours = vec![0u8; sz];
    ours_fn(
        &mut DataStore::new(buf),
        &mut ours,
        &mut Dup::new(DUP_BASE, DUP_STRIDE),
    );
    (bin, ours)
}

#[test]
fn dbcache_creature() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0xc0de_0001u32;
    for case in 0..300u32 {
        let mut buf = Vec::new();
        for _ in 0..4 {
            push_str(&mut buf, &mut s); // name[0..3]
        }
        push_str(&mut buf, &mut s); // subname
        for _ in 0..7 {
            push4(&mut buf, &mut s); // GetInt32 +0x14..+0x2c
        }
        buf.push((next(&mut s) & 0xff) as u8); // GetUInt8 -> bool +0x30
        buf.push((next(&mut s) & 0xff) as u8); // GetUInt8 -> bool +0x31

        let (bin, ours) = diff_case(&wow, CREATURE, CREATURE_SZ, &buf, creature);
        assert_eq!(bin, ours, "creature case={case}");
    }
}

#[test]
fn dbcache_gameobject() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0xfeed_0001u32;
    for case in 0..300u32 {
        let mut buf = Vec::new();
        push4(&mut buf, &mut s); // +0
        push4(&mut buf, &mut s); // +4
        for _ in 0..5 {
            push_str(&mut buf, &mut s); // 5 char* @+8..+0x18
        }
        for _ in 0..24 {
            push4(&mut buf, &mut s); // GetInt32 +0x1c..+0x78
        }

        let (bin, ours) = diff_case(&wow, GAMEOBJECT, GAMEOBJECT_SZ, &buf, gameobject);
        assert_eq!(bin, ours, "gameobject case={case}");
    }
}

#[test]
fn dbcache_itemstats() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0xabba_0001u32;
    for case in 0..300u32 {
        let mut buf = Vec::new();
        push4(&mut buf, &mut s); // +0
        push4(&mut buf, &mut s); // +4
        for _ in 0..4 {
            push_str(&mut buf, &mut s); // 4 char* @+8..+0x14
        }
        for _ in 0..20 {
            push4(&mut buf, &mut s); // GetInt32 +0x18..+0x64
        }
        for _ in 0..10 {
            push4(&mut buf, &mut s); // int[10] A[i]
            push4(&mut buf, &mut s); // int[10] B[i]
        }
        for _ in 0..5 {
            push4(&mut buf, &mut s); // float[5] @+0xb8
            push4(&mut buf, &mut s); // float[5] @+0xcc
            push4(&mut buf, &mut s); // int[5]   @+0xe0
        }
        for _ in 0..7 {
            push4(&mut buf, &mut s); // GetInt32 +0xf4..+0x10c
        }
        push4(&mut buf, &mut s); // +0x110
        push4(&mut buf, &mut s); // +0x114
        push4(&mut buf, &mut s); // GetFloat +0x118
        for _ in 0..5 {
            for _ in 0..6 {
                push4(&mut buf, &mut s); // 6×int[5] arr0[i]..arr5[i]
            }
        }
        push4(&mut buf, &mut s); // +0x194
        push_str(&mut buf, &mut s); // desc char* @+0x198 (conditional)
        for _ in 0..14 {
            push4(&mut buf, &mut s); // GetInt32 +0x19c..+0x1d0
        }

        let (bin, ours) = diff_case(&wow, ITEMSTATS, ITEMSTATS_SZ, &buf, itemstats);
        assert_eq!(bin, ours, "itemstats case={case}");
    }
}

#[test]
fn dbcache_npctext() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x1234_0001u32;
    for case in 0..300u32 {
        let mut buf = Vec::new();
        for _ in 0..8 {
            push4(&mut buf, &mut s); // prob (GetFloat) @+0+i*4
            push_str(&mut buf, &mut s); // text0 @+0x20+i*8
            push_str(&mut buf, &mut s); // text1 @+0x24+i*8
            push4(&mut buf, &mut s); // lang @+0x60+i*4
            for _ in 0..3 {
                push4(&mut buf, &mut s); // emoteId @+0x80
                push4(&mut buf, &mut s); // emoteDelay @+0xe0
            }
        }

        let (bin, ours) = diff_case(&wow, NPCTEXT, NPCTEXT_SZ, &buf, npctext);
        assert_eq!(bin, ours, "npctext case={case}");
    }
}
