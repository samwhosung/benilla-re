//! Difftests for the two "special" decoders. **Petition** (0x7c7a30) is variable-length, count-prefixed with
//! inline strings: we synthesize the SMSG record in wire order, fuzz the signer count (0 / 1 / mid / max 10) to
//! exercise the trailing loop's bounds, drive the binary `T::Read` over a `CDataStore`, and assert byte-equal on
//! the whole record. **ItemText** (0x569d00) is decoded inline in its Receive worker (no separate `T::Read`); we
//! call the worker with the "process exactly one record" flag, point the node allocator at a zeroed node via
//! a stub, and compare the decoded payload (text body, id and valid byte).

mod common;

use benilla_re_dbcache::datastore::DataStore;
use benilla_re_dbcache::decode_special::{item_text, petition};
use benilla_re_difftest::{Emu, Wow};
use common::{make_datastore, next, wire_cstring};

const PETITION: u64 = 0x7c7a30; // __thiscall(ecx=record, [esp+4]=CDataStore) ret 4
const PETITION_SZ: usize = 0x13c4;

const ITEM_TEXT: u64 = 0x569d00; // __thiscall(ecx=DBCache singleton, [esp+4]=ds, [esp+8]=process-flag) ret 8
const ITEM_TEXT_ALLOC: u64 = 0x56e210; // GetRecord/alloc-node: __thiscall(id,0,0) ret 0xc — stubbed to our node
const ITEM_TEXT_NODE_SZ: usize = 0x2000;
const ITEM_TEXT_PAYLOAD_OFF: u64 = 0x18; // the record payload sits at node+0x18
const ITEM_TEXT_REC_SZ: usize = 0x1f48; // text[8000] @+0, id @+0x1f40, valid @+0x1f44 (+pad)

/// Build a Petition SMSG record buffer in wire (= read) order: u32, guid(8), title cstr, body cstr, 8×u32,
/// u16, u32, u32, **u32 signer count**, `count` signer cstrs, u32. String lengths stay within their inline
/// fields (title ≤0xff, body ≤0xfff, names ≤0x3f) — the real records do, and it keeps the synthesized record
/// self-consistent.
fn petition_buf(s: &mut u32, count: usize, tlen: usize, blen: usize, nlen: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&next(s).to_le_bytes()); // +0x0  GetInt32
    for _ in 0..8 {
        buf.push((next(s) >> 8) as u8); // +0x8  GetGuid (8 bytes)
    }
    buf.extend_from_slice(&wire_cstring(s, tlen)); // +0x10  title
    buf.extend_from_slice(&wire_cstring(s, blen)); // +0x110 body
    for _ in 0..8 {
        buf.extend_from_slice(&next(s).to_le_bytes()); // +0x1110..+0x112c (8 × GetInt32)
    }
    buf.extend_from_slice(&((next(s) >> 8) as u16).to_le_bytes()); // +0x1130 GetInt16
    buf.extend_from_slice(&next(s).to_le_bytes()); // +0x1134
    buf.extend_from_slice(&next(s).to_le_bytes()); // +0x1138
    buf.extend_from_slice(&(count as u32).to_le_bytes()); // +0x13bc signer count
    for _ in 0..count {
        buf.extend_from_slice(&wire_cstring(s, nlen)); // +0x113c + i*0x40 signer names
    }
    buf.extend_from_slice(&next(s).to_le_bytes()); // +0x13c0 GetUInt32
    buf
}

#[test]
fn dbcache_petition() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x1234_0001u32;
    let counts = [0usize, 1, 2, 5, 10]; // fuzz the loop bound: empty, single, mid, full (10-slot array)
    let tlens = [0usize, 1, 16, 0xff];
    let blens = [0usize, 32, 0x100, 0xfff];
    let nlens = [0usize, 1, 16, 0x3f];
    let mut case = 0u32;
    for &count in &counts {
        for i in 0..16usize {
            let tlen = tlens[i % tlens.len()];
            let blen = blens[(i / 4) % blens.len()];
            let nlen = nlens[(i / 2) % nlens.len()];
            let buf = petition_buf(&mut s, count, tlen, blen, nlen);

            let mut e = Emu::new(&wow);
            let ds = make_datastore(&mut e, &buf);
            let rec = e.alloc(PETITION_SZ as u64);
            e.write(rec, &vec![0u8; PETITION_SZ]);
            e.call(PETITION, rec as u32, 0, &[ds as u32]);
            let bin = e.read(rec, PETITION_SZ);

            let mut ours = vec![0u8; PETITION_SZ];
            petition(&mut DataStore::new(&buf), &mut ours);
            assert_eq!(
                bin, ours,
                "petition case={case} count={count} tlen={tlen} blen={blen} nlen={nlen}"
            );
            case += 1;
        }
    }
}

#[test]
fn dbcache_item_text() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0xabcd_0001u32;
    let lens = [0usize, 1, 16, 100, 1000, 4000]; // text-body lengths (< the 8000 field bound)
    for case in 0..120u32 {
        let tlen = lens[(case as usize) % lens.len()];
        let id = next(&mut s) & 0x7fffffff; // high bit clear ⇒ the decode (not evict) branch
        let text = wire_cstring(&mut s, tlen);
        let mut buf = Vec::new();
        buf.extend_from_slice(&id.to_le_bytes()); // GetInt32 id
        buf.extend_from_slice(&text); // GetCString text body

        let mut e = Emu::new(&wow);
        let ds = make_datastore(&mut e, &buf);
        let node = e.alloc(ITEM_TEXT_NODE_SZ as u64);
        e.write(node, &vec![0u8; ITEM_TEXT_NODE_SZ]);
        // DBCache<ItemTextCache_C> singleton: on the decode path only the hash mask (+0x24) is read; set it
        // uninitialised (0xffffffff) so the bucket lookup is skipped and the alloc/insert path is taken.
        let singleton = e.alloc(0x40);
        e.write(singleton, &[0u8; 0x40]);
        e.write_u32(singleton + 0x24, 0xffff_ffff);
        // The node allocator and inserter are the cache's own: stub it to hand back our zeroed node, so
        // the decode (GetInt32, GetCString and the stores) runs into it.
        e.stub(ITEM_TEXT_ALLOC, node as u32, 3);
        // [esp+8] = 1 ⇒ process exactly one record (skips the leading count byte).
        e.call(ITEM_TEXT, singleton as u32, 0, &[ds as u32, 1]);
        let bin = e.read(node + ITEM_TEXT_PAYLOAD_OFF, ITEM_TEXT_REC_SZ);

        let mut ours = vec![0u8; ITEM_TEXT_REC_SZ];
        item_text(&mut DataStore::new(&buf), &mut ours);
        assert_eq!(bin, ours, "item_text case={case} tlen={tlen} id={id:#x}");
    }
}
