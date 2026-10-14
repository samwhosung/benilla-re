//! Helpers shared by the dbcache tests: the LCG (a choice reads its high bits, since the low bits
//! repeat quickly) and the `CDataStore` builder, a record buffer mapped into emulator memory through
//! `CDataStore`'s own-buffer path (no vtable refill), so the exe's `Get*` functions read the
//! synthesized SMSG record. The layout (from `GetInt32`, `0x418e30`): +0 vtable (0, unused on the
//! own-buffer path), +4 buffer base, +8 0 (a cursor at or above it selects the own-buffer path), +0xc
//! size (the own-buffer bound), +0x10 size (the entry bound), +0x14 cursor (0). Reads are at buffer
//! base plus cursor.

#![allow(dead_code)]

use benilla_re_difftest::Emu;

pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// Builds a `CDataStore` in emulator memory reading `buf`; returns its address (the decoder's `ds` argument).
pub fn make_datastore(e: &mut Emu, buf: &[u8]) -> u64 {
    let buf_va = e.alloc(buf.len().max(4) as u64);
    e.write(buf_va, buf);
    let ds = e.alloc(0x18);
    e.write_u32(ds, 0);
    e.write_u32(ds + 4, buf_va as u32);
    e.write_u32(ds + 8, 0);
    e.write_u32(ds + 0xc, buf.len() as u32);
    e.write_u32(ds + 0x10, buf.len() as u32);
    e.write_u32(ds + 0x14, 0);
    ds
}

/// A NUL-terminated wire C-string of the given byte length (excl. NUL), filled with fuzzed printable bytes.
pub fn wire_cstring(s: &mut u32, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len + 1);
    for _ in 0..len {
        v.push((0x20 + (next(s) >> 8) % 0x5e) as u8); // printable ASCII
    }
    v.push(0);
    v
}
