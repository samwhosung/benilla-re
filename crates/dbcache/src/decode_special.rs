//! The two per-record decoders that are not a plain fixed-layout `T::Read`: the variable-length,
//! count-prefixed petition decoder, and the item text decode, which sits inside its receive worker.
//! `out` is the zeroed record struct; the wire reads are modelled by [`DataStore`].

use crate::datastore::DataStore;

/// CGPetition::Read, `0x7c7a30`, record 0x13c4 bytes: variable length, count prefixed, inline strings.
/// The read order is below; the count is read mid-stream and bounds the trailing signer-name loop:
/// - `GetInt32`  → +0x0
/// - `GetGuid`   → +0x8         (8 bytes; +0x4..+0x8 is an unread struct hole)
/// - `GetCString`→ +0x10        title  (max `0x7fffffff`, an unbounded cap; a char[0x100] field)
/// - `GetCString`→ +0x110       body   (max `0x7fffffff`; a char[0x1000] field)
/// - `GetInt32` ×8 → +0x1110, +0x1114 … +0x112c
/// - `GetInt16`  → +0x1130      (+0x1132..+0x1134 is a 2-byte hole)
/// - `GetInt32`  → +0x1134
/// - `GetInt32`  → +0x1138
/// - `GetInt32`  → +0x13bc      = **signer count** (signed; ≤ 0 ⇒ the loop runs zero times)
/// - `for i in 0..count`: `GetCString` → +0x113c + i*0x40   (inline char[0x40] signer names, array of 10)
/// - `GetUInt32` → +0x13c0
///
/// The signer-name array (+0x113c to +0x13bc) is char[10][0x40]; the count bounds how many of the 10
/// slots are filled from the wire. Each `GetCString` passes the unbounded cap `0x7fffffff`, so the full
/// inline string is written.
pub fn petition(ds: &mut DataStore, out: &mut [u8]) {
    let v = ds.u32(); // +0x0 GetInt32
    out[0x0..0x4].copy_from_slice(&v.to_le_bytes());

    let g = ds.guid(); // +0x8
    out[0x8..0x10].copy_from_slice(&g);

    ds.cstring(out, 0x10, 0x7fffffff); // +0x10 title
    ds.cstring(out, 0x110, 0x7fffffff); // +0x110 body

    for k in 0..8usize {
        // +0x1110 .. +0x112c (8 × GetInt32)
        let off = 0x1110 + k * 4;
        let v = ds.u32();
        out[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    let h = ds.u16(); // +0x1130 GetInt16
    out[0x1130..0x1132].copy_from_slice(&h.to_le_bytes());

    let v = ds.u32(); // +0x1134
    out[0x1134..0x1138].copy_from_slice(&v.to_le_bytes());
    let v = ds.u32(); // +0x1138
    out[0x1138..0x113c].copy_from_slice(&v.to_le_bytes());

    let count = ds.u32() as i32; // +0x13bc signer count (signed)
    out[0x13bc..0x13c0].copy_from_slice(&count.to_le_bytes());

    let mut i = 0i32;
    while i < count {
        // +0x113c + i*0x40 (inline char[0x40] signer name)
        let off = 0x113c + (i as usize) * 0x40;
        ds.cstring(out, off, 0x7fffffff);
        i += 1;
    }

    let v = ds.u32(); // +0x13c0 GetUInt32
    out[0x13c0..0x13c4].copy_from_slice(&v.to_le_bytes());
}

/// The ItemTextCache_C decode, inside the receive worker `0x569d00`; there is no separate `T::Read`.
/// This is the per-record decode only; the record payload sits at the cache node `+0x18`, which is
/// `out`'s offset 0:
/// - `GetInt32` is the id. Its high bit is an evict flag: when set, `0x569d00` takes the eviction branch
///   and does not decode (no `GetCString`). Only the high-bit-clear branch decodes, and it stores the
///   id as read; the `& 0x7fffffff` strip is the eviction branch's lookup-key fix-up.
/// - `GetCString(payload+0, 8000)` is the inline text body.
/// - Stores: the id at payload+0x1f40 and a valid byte (1) at payload+0x1f44.
///
/// `out` is the ItemTextCache_C payload (at least 0x1f48 bytes), zeroed.
pub fn item_text(ds: &mut DataStore, out: &mut [u8]) {
    let id = ds.u32(); // GetInt32 (decode path: high bit clear, stored verbatim)
    ds.cstring(out, 0, 8000); // GetCString text body @ payload+0
    out[0x1f40..0x1f44].copy_from_slice(&id.to_le_bytes()); // id @ payload+0x1f40
    out[0x1f44] = 1; // valid flag @ payload+0x1f44
}
