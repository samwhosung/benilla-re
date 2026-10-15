//! The cache record writer `0x6ca070` and reader `0x6ca0c0` run whole, over real data stores (the
//! exe's `PutUInt32`, `PutInt8`, `GetUInt32` and `GetUInt8` run too), against
//! `benilla_re_warden::cache`. The allocator is a bump stub.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_warden::cache::{decode_record, encode_record};
use common::Rng;

const WRITER: u64 = 0x6ca070;
const READER: u64 = 0x6ca0c0;
const S_MEM_ALLOC: u64 = 0x6462e0;
const PATH: u32 = 0x86f074;
/// A data store: the vtable pointer, the buffer, the base, the capacity, the size, the cursor.
const STORE_BUFFER: u64 = 4;
const STORE_CAPACITY: u64 = 0xc;
const STORE_SIZE: u64 = 0x10;
const STORE_CURSOR: u64 = 0x14;

struct Rig {
    emu: Emu<'static>,
    arena: u64,
}

fn rig(wow: &Wow) -> Rig {
    let mut emu = Emu::new(wow);
    emu.map(0x10000, 0x2000);
    let arena = emu.alloc(0x20_0000);
    emu.stub_alloc_sized(S_MEM_ALLOC, arena, arena + 0x20_0000, 0, 4);
    Rig { emu, arena }
}

impl Rig {
    /// A store at `at` with `bytes` to read and room to write.
    fn store(&mut self, at: u64, bytes: &[u8], capacity: u64) {
        let buf = self.emu.alloc(capacity.max(bytes.len() as u64) + 16);
        self.emu.write(buf, bytes);
        self.emu.write(at, &[0u8; 0x20]);
        self.emu.write_u32(at + STORE_BUFFER, buf as u32);
        self.emu.write_u32(at + STORE_CAPACITY, capacity as u32);
        self.emu.write_u32(at + STORE_SIZE, bytes.len() as u32);
        self.emu.write_u32(at + STORE_CURSOR, 0);
    }
    fn contents(&self, at: u64) -> Vec<u8> {
        let buf = u64::from(self.emu.read_u32(at + STORE_BUFFER));
        self.emu
            .read(buf, self.emu.read_u32(at + STORE_SIZE) as usize)
    }
}

#[test]
fn record_writer() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = rig(&wow);
    let mut rng = Rng(0x6ca0_7000);
    for case in 0..60 {
        let n = if case == 0 {
            0
        } else {
            rng.below(700) as usize
        };
        let data: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        let src = r.emu.alloc(n as u64 + 16);
        r.emu.write(src, &data);
        let obj = r.emu.alloc(8);
        r.emu.write_u32(obj, n as u32);
        r.emu
            .write_u32(obj + 4, if n == 0 { 0 } else { src as u32 });
        let store = r.emu.alloc(0x20);
        r.store(store, &[], 0x2000);
        r.emu.call(WRITER, obj as u32, 0, &[store as u32]);
        assert_eq!(r.contents(store), encode_record(&data), "case {case}");
    }
}

#[test]
fn record_reader() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = rig(&wow);
    let mut rng = Rng(0x6ca0_c000);
    for case in 0..60 {
        let n = if case == 0 {
            0
        } else {
            rng.below(700) as usize
        };
        let data: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        let mut bytes = encode_record(&data);
        // Bytes after the record are not read.
        bytes.extend((0..rng.below(5)).map(|_| rng.next() as u8));
        let store = r.emu.alloc(0x20);
        r.store(store, &bytes, bytes.len() as u64);
        let obj = r.emu.alloc(8);
        r.emu.write(obj, &[0xaa; 8]);
        r.emu.clear_calls();
        r.emu.call(READER, obj as u32, 0, &[store as u32]);
        assert_eq!(r.emu.read_u32(obj), n as u32, "case {case}: size");
        let want = decode_record(&bytes).unwrap();
        let ptr = r.emu.read_u32(obj + 4);
        if n == 0 {
            assert_eq!(ptr, 0, "case {case}: empty record has no buffer");
            assert!(r.emu.calls().is_empty(), "case {case}: no allocation");
        } else {
            assert_eq!(r.emu.read(u64::from(ptr), n), want, "case {case}");
            let call = r.emu.calls()[0];
            assert_eq!(
                (call.args[0], call.args[1], call.args[3]),
                (n as u32, PATH, 0)
            );
        }
    }
}

#[test]
fn record_reader_short_stores() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut r = rig(&wow);
    // A record that claims more bytes than the store has: the bytes past the end are not written,
    // and the buffer holds what the allocator gave.
    let mut bytes = 10u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
    let store = r.emu.alloc(0x20);
    r.store(store, &bytes, bytes.len() as u64);
    r.emu.write(r.arena, &[0u8; 0x100]);
    let obj = r.emu.alloc(8);
    r.emu.call(READER, obj as u32, 0, &[store as u32]);
    assert_eq!(r.emu.read_u32(obj), 10);
    let ptr = u64::from(r.emu.read_u32(obj + 4));
    assert_eq!(r.emu.read(ptr, 10), [1, 2, 3, 4, 5, 6, 0, 0, 0, 0]);
    assert!(decode_record(&bytes).is_none());

    // A store shorter than four bytes writes no length, and the length is the store's own
    // address: the stack slot that held the argument.
    let store_at = 0x10040u64;
    r.store(store_at, &[7, 8], 2);
    r.emu.clear_calls();
    let obj = r.emu.alloc(8);
    r.emu.call(READER, obj as u32, 0, &[store_at as u32]);
    assert_eq!(r.emu.read_u32(obj), store_at as u32);
    assert_eq!(r.emu.calls()[0].args[0], store_at as u32);
}
