//! Per-record decoder difftests. Synthesize an SMSG record buffer in wire order, drive the binary `T::Read` over
//! a `CDataStore`, read the decoded struct, assert byte-equal vs our transcription on the same buffer.

mod common;

use benilla_re_dbcache::datastore::{DataStore, Dup};
use benilla_re_dbcache::decode::{item_name, pet_name};
use benilla_re_difftest::{Emu, Wow};
use common::{make_datastore, next, wire_cstring};

const PET_NAME: u64 = 0x6dc290; // __thiscall(ecx=record, [esp+4]=CDataStore) ret 4
const PET_NAME_SZ: usize = 0x58;

const ITEM_NAME: u64 = 0x7c8dc0; // __thiscall(ecx=record, [esp+4]=CDataStore) ret 4
const ITEM_NAME_SZ: usize = 4;
const SSTRDUPA: u64 = 0x64a620; // SStrDupA(src, FILE, LINE) ret 0xc — stubbed to a deterministic bump allocator
const DUP_BASE: u32 = 0x1000_0000;
const DUP_STRIDE: u32 = 0x100;

#[test]
fn dbcache_pet_name() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x9e37_0001u32;
    // wire order = read order: u32 @+0x50, cstring (max 0x50) @+0, u32 @+0x54.
    // probe name lengths incl. empty, short, and near-max (< max so the common copy path).
    let lens = [0usize, 1, 5, 16, 0x4e];
    for case in 0..400u32 {
        let nlen = lens[(case as usize) % lens.len()];
        let a = next(&mut s);
        let name = wire_cstring(&mut s, nlen);
        let b = next(&mut s);
        let mut buf = Vec::new();
        buf.extend_from_slice(&a.to_le_bytes());
        buf.extend_from_slice(&name);
        buf.extend_from_slice(&b.to_le_bytes());

        let mut e = Emu::new(&wow);
        let ds = make_datastore(&mut e, &buf);
        let rec = e.alloc(PET_NAME_SZ as u64);
        e.write(rec, &[0u8; PET_NAME_SZ]);
        e.call(PET_NAME, rec as u32, 0, &[ds as u32]);
        let bin = e.read(rec, PET_NAME_SZ);

        let mut ours = vec![0u8; PET_NAME_SZ];
        pet_name(&mut DataStore::new(&buf), &mut ours);
        assert_eq!(bin, ours, "pet_name case={case} nlen={nlen}");
    }
}

#[test]
fn dbcache_item_name() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x55aa_0001u32;
    let lens = [0usize, 1, 8, 0x40];
    for case in 0..200u32 {
        let buf = wire_cstring(&mut s, lens[(case as usize) % lens.len()]); // GetCString then SStrDupA
        let mut e = Emu::new(&wow);
        e.stub_alloc(SSTRDUPA, DUP_BASE as u64, DUP_STRIDE as u64, 3);
        let ds = make_datastore(&mut e, &buf);
        let rec = e.alloc(ITEM_NAME_SZ as u64);
        e.write(rec, &[0u8; ITEM_NAME_SZ]);
        e.call(ITEM_NAME, rec as u32, 0, &[ds as u32]);
        let bin = e.read(rec, ITEM_NAME_SZ);

        let mut ours = vec![0u8; ITEM_NAME_SZ];
        item_name(
            &mut DataStore::new(&buf),
            &mut ours,
            &mut Dup::new(DUP_BASE, DUP_STRIDE),
        );
        assert_eq!(bin, ours, "item_name case={case}");
    }
}
