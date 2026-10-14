//! The per-record `T::Read` decoders whose strings are inline: `GetCString` writes straight into the
//! record struct with no `SStrDupA`, so the decoded `<Type>_C` struct compares byte for byte. Each reads
//! an `SMSG_*_QUERY_RESPONSE` record from a [`DataStore`] cursor (the fields after the key) in the exe's
//! read order and writes the struct at its field offsets: the page text (`0x7cabd0`), the name
//! (`0x6dc1f0`), the guild (`0x62f260`) and the quest (`0x6dbf60`). `out` is the zeroed record struct.

use crate::datastore::DataStore;

/// `GetInt32`, `GetUInt32` and `GetFloat`, all the same 4-byte copy, into the struct at `off`.
#[inline]
fn put32(ds: &mut DataStore, out: &mut [u8], off: usize) {
    let v = ds.u32();
    out[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

/// PageTextCache_C, `0x7cabd0` (record 0x404 bytes). Read order: `GetCString(+0, 0x400)`, the inline page
/// text, then `GetInt32(+0x400)`, the next page id.
pub fn page_text(ds: &mut DataStore, out: &mut [u8]) {
    ds.cstring(out, 0, 0x400);
    put32(ds, out, 0x400);
}

/// NameCache, keyed by guid, `0x6dc1f0` (record 0x144 bytes). Read order: `GetGuid(+0x130)`,
/// `GetCString(+0, 0x30)` the name, `GetCString(+0x30, 0x100)`, then `GetUInt32` at `+0x138`, `+0x13c` and
/// `+0x140`, the race, gender and class.
pub fn name(ds: &mut DataStore, out: &mut [u8]) {
    let g = ds.guid();
    out[0x130..0x138].copy_from_slice(&g);
    ds.cstring(out, 0, 0x30);
    ds.cstring(out, 0x30, 0x100);
    put32(ds, out, 0x138);
    put32(ds, out, 0x13c);
    put32(ds, out, 0x140);
}

/// GuildStats_C, `0x62f260` (record 0x2f8 bytes; `systems/objectlayer/party.md#the-guild-record`). Read
/// order: `GetUInt32(+0)`, `GetCString(+4, 0x60)` the guild name, ten `GetCString(+0x64 + i * 0x40, 0x40)`
/// rank names, then five `GetInt32` at `+0x2e4`, `+0x2e8`, `+0x2ec`, `+0x2f0` and `+0x2f4`.
pub fn guild(ds: &mut DataStore, out: &mut [u8]) {
    put32(ds, out, 0);
    ds.cstring(out, 4, 0x60);
    for i in 0..10 {
        ds.cstring(out, 0x64 + i * 0x40, 0x40);
    }
    for k in 0..5 {
        put32(ds, out, 0x2e4 + k * 4);
    }
}

/// QuestCache, `0x6dbf60` (record 0x18dc bytes). `GetCString` is called with the maximum `0x7fffffff`,
/// so the field sizes (Title 0x200, Objectives and Details 0x800, EndText 0x200, four ObjectiveText of
/// 0x100) are the struct layout and no copy bound. `GetFloat` (at `+0x90` and `+0x94`) is the same 4-byte
/// copy, read by its bits through `u32`.
pub fn quest(ds: &mut DataStore, out: &mut [u8]) {
    // 15 GetInt32 at +0x00 to +0x38.
    for k in 0..15 {
        put32(ds, out, k * 4);
    }
    // loop 1: 4 pairs { GetInt32(base - 0x10), GetInt32(base) }, base = 0x4c, 0x50, 0x54, 0x58.
    let mut iv = 0x4c;
    for _ in 0..4 {
        put32(ds, out, iv - 0x10);
        put32(ds, out, iv);
        iv += 4;
    }
    // loop 2: 6 pairs { GetInt32(base - 0x18), GetInt32(base) }, base = 0x74 to 0x88.
    let mut iv = 0x74;
    for _ in 0..6 {
        put32(ds, out, iv - 0x18);
        put32(ds, out, iv);
        iv += 4;
    }
    put32(ds, out, 0x8c);
    put32(ds, out, 0x90); // GetFloat (raw bits)
    put32(ds, out, 0x94); // GetFloat (raw bits)
    put32(ds, out, 0x98);
    ds.cstring(out, 0x9c, 0x7fff_ffff); // Title
    ds.cstring(out, 0x29c, 0x7fff_ffff); // Objectives
    ds.cstring(out, 0xa9c, 0x7fff_ffff); // Details
    ds.cstring(out, 0x129c, 0x7fff_ffff); // EndText

    // loop 3: 4 groups { GetInt32(base - 0x10), GetInt32(base), GetInt32(base + 0x10),
    // GetInt32(base + 0x20) }, base = 0x14ac, 0x14b0, 0x14b4, 0x14b8 (4 by 4 objective ints).
    let mut iv = 0x14ac;
    for _ in 0..4 {
        put32(ds, out, iv - 0x10);
        put32(ds, out, iv);
        put32(ds, out, iv + 0x10);
        put32(ds, out, iv + 0x20);
        iv += 4;
    }
    // 4 inline ObjectiveText at +0x14dc, +0x15dc, +0x16dc and +0x17dc (char[0x100] each).
    ds.cstring(out, 0x14dc, 0x7fff_ffff);
    ds.cstring(out, 0x15dc, 0x7fff_ffff);
    ds.cstring(out, 0x16dc, 0x7fff_ffff);
    ds.cstring(out, 0x17dc, 0x7fff_ffff);
}
