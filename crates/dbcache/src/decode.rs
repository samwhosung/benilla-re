//! Two per-record `T::Read` decoders. Each reads an `SMSG_*_QUERY_RESPONSE` record from a [`DataStore`]
//! cursor (the fields after the key) and writes the `<Type>_C` struct, in the exe's read order and at its
//! field offsets. `out` is the zeroed record struct.

use crate::datastore::{DataStore, Dup};

/// PetNameCache_C, `0x6dc290` (record 0x58 bytes). Read order: u32 at `+0x50`, a C string at `+0` (at
/// most 0x50), u32 at `+0x54`. The string is inline.
pub fn pet_name(ds: &mut DataStore, out: &mut [u8]) {
    let a = ds.u32();
    out[0x50..0x54].copy_from_slice(&a.to_le_bytes());
    ds.cstring(out, 0, 0x50);
    let b = ds.u32();
    out[0x54..0x58].copy_from_slice(&b.to_le_bytes());
}

/// ItemName_C, `0x7c8dc0` (record 4 bytes): `GetCString(local, 0x100)`, then `SStrDupA(local)` stored as a
/// `char *` at `+0`. This is the single name; the item id is the cache key, read by the receive worker.
pub fn item_name(ds: &mut DataStore, out: &mut [u8], dup: &mut Dup) {
    ds.skip_cstring(0x100);
    let p = dup.alloc();
    out[0..4].copy_from_slice(&p.to_le_bytes());
}
