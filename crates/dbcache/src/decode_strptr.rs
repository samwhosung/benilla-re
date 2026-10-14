//! The per-record decoders with `char *` fields: `CreatureStats_C::Read` (`0x7c8120`),
//! `GameObjectStats_C::Read` (`0x7c87c0`), `ItemStats_C::Read` (`0x7c9640`) and `NPCText::Read`
//! (`0x7c9fa0`). Each reads an `SMSG_*_QUERY_RESPONSE` record from a [`DataStore`] cursor and writes the
//! `<Type>_C` struct. `out` is the zeroed record.
//!
//! The name fields of the creature and the game object always call `SStrDupA`. On an empty field (the
//! first wire byte NUL) at slot `i > 0` the exe passes `SStrDupA` the previous slot's pointer as the
//! source instead of the local buffer, but the return is still a fresh allocation and it is what is
//! stored. The tests stub `SStrDupA` to a counting bump allocator (`base + n * stride`), so the stored
//! pointer is the same either way, and the model is one `dup.alloc()` per slot. The ItemStats `desc` and
//! the NPCText `text` fields differ: they call `SStrDupA` only when non-empty, an empty field stores 0
//! and takes no allocation, so the stored pointer depends on emptiness and the model tests for it.

use crate::datastore::{DataStore, Dup};

/// Write a little-endian u32 at `out[off..off+4]`.
fn w32(out: &mut [u8], off: usize, v: u32) {
    out[off..off + 4].copy_from_slice(&v.to_le_bytes());
}

/// `GetCString(local, max)` into a scratch local, returning whether the string is empty (first byte NUL),
/// as the exe's `GetCString(local, ...); local[0] == '\0'` test does. The cursor advances by the full
/// source length regardless of `max` ([`DataStore::cstring`] semantics), so a 1-byte scratch reads just the
/// leading byte while consuming the whole wire string.
fn cstring_is_empty(ds: &mut DataStore) -> bool {
    let mut first = [0u8; 1];
    ds.cstring(&mut first, 0, 1);
    first[0] == 0
}

/// CreatureStats_C, `0x7c8120` (record 0x34). Read order: four name `char *` at `+0`, `+4`, `+8`, `+0xc`
/// (a loop; an empty one at `i > 0` takes the previous slot as `SStrDupA`'s source and still allocates); the
/// subname `char *` at `+0x10` (always allocates, no reuse); 7 `GetInt32` at `+0x14` to `+0x2c`; 2 `GetUInt8`
/// stored as bools (0 or 1, `setne`) at `+0x30` and `+0x31`.
pub fn creature(ds: &mut DataStore, out: &mut [u8], dup: &mut Dup) {
    for i in 0..4usize {
        ds.skip_cstring(0x100); // name[i]: an empty one changes SStrDupA's source, not the stored pointer
        w32(out, i * 4, dup.alloc());
    }
    ds.skip_cstring(0x100); // subname: always allocs
    w32(out, 0x10, dup.alloc());
    for off in [0x14usize, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c] {
        let v = ds.u32();
        w32(out, off, v);
    }
    out[0x30] = u8::from(ds.u8() != 0);
    out[0x31] = u8::from(ds.u8() != 0);
}

/// GameObjectStats_C, `0x7c87c0` (record 0x7c). Read order: `GetInt32` at `+0` and `+4`; five `char *` at
/// `+8`, `+0xc`, `+0x10`, `+0x14` and `+0x18` (the first four in a loop where an empty one takes the previous
/// source, the fifth always allocates; all allocate); 24 `GetInt32` at `+0x1c` to `+0x78`.
pub fn gameobject(ds: &mut DataStore, out: &mut [u8], dup: &mut Dup) {
    let a = ds.u32();
    w32(out, 0, a);
    let b = ds.u32();
    w32(out, 4, b);
    for i in 0..5usize {
        ds.skip_cstring(0x100);
        w32(out, 8 + i * 4, dup.alloc());
    }
    for k in 0..24usize {
        let v = ds.u32();
        w32(out, 0x1c + k * 4, v);
    }
}

/// ItemStats_C, `0x7c9640` (record 0x1d4). Read order: `GetInt32` at `+0` and `+4`; four `char *` at `+8`
/// to `+0x14` (a loop; always allocates); 20 `GetInt32` at `+0x18` to `+0x64`; two int[10] at `+0x68` and
/// `+0x90` (A[i], B[i] interleaved); 5 groups of `GetFloat` at `+0xb8`, `GetFloat` at `+0xcc`, `GetInt32` at
/// `+0xe0`; 7 `GetInt32` at `+0xf4` to `+0x10c`; `+0x110` and `+0x114`; `GetFloat` at `+0x118`; six int[5] at
/// `+0x11c`, `+0x130`, `+0x144`, `+0x158`, `+0x16c` and `+0x180` (read arr0[i] to arr5[i]); `+0x194`; the
/// desc `char *` at `+0x198` (allocates only if non-empty, else 0); 14 `GetInt32` at `+0x19c` to `+0x1d0`.
pub fn itemstats(ds: &mut DataStore, out: &mut [u8], dup: &mut Dup) {
    let a = ds.u32();
    w32(out, 0, a);
    let b = ds.u32();
    w32(out, 4, b);
    for i in 0..4usize {
        ds.skip_cstring(0x100);
        w32(out, 8 + i * 4, dup.alloc());
    }
    for k in 0..20usize {
        let v = ds.u32();
        w32(out, 0x18 + k * 4, v);
    }
    for i in 0..10usize {
        let aa = ds.u32();
        w32(out, 0x68 + i * 4, aa);
        let bb = ds.u32();
        w32(out, 0x90 + i * 4, bb);
    }
    for i in 0..5usize {
        let f0 = ds.u32();
        w32(out, 0xb8 + i * 4, f0);
        let f1 = ds.u32();
        w32(out, 0xcc + i * 4, f1);
        let n = ds.u32();
        w32(out, 0xe0 + i * 4, n);
    }
    for k in 0..7usize {
        let v = ds.u32();
        w32(out, 0xf4 + k * 4, v);
    }
    let v110 = ds.u32();
    w32(out, 0x110, v110);
    let v114 = ds.u32();
    w32(out, 0x114, v114);
    let v118 = ds.u32();
    w32(out, 0x118, v118);
    for i in 0..5usize {
        for base in [0x11cusize, 0x130, 0x144, 0x158, 0x16c, 0x180] {
            let v = ds.u32();
            w32(out, base + i * 4, v);
        }
    }
    let v194 = ds.u32();
    w32(out, 0x194, v194);
    if !cstring_is_empty(ds) {
        w32(out, 0x198, dup.alloc()); // desc: allocs only when non-empty
    }
    for k in 0..14usize {
        let v = ds.u32();
        w32(out, 0x19c + k * 4, v);
    }
}

/// NPCText, `0x7c9fa0` (record 0x140). An 8-iteration loop; per iteration `i`: `GetFloat` prob at
/// `+i*4`; two `char *`, text0 at `+0x20+i*8` and text1 at `+0x24+i*8` (each allocates only if non-empty,
/// else 0, with no reuse); `GetUInt32` lang at `+0x60+i*4`; an inner loop of 3 (`j`): `GetInt32` emoteId at
/// `+0x80+i*0xc+j*4`, `GetInt32` emoteDelay at `+0xe0+i*0xc+j*4`. The exe calls `SErr` if the existing
/// pointer is non-zero before the assignment, which a zeroed record never reaches.
pub fn npctext(ds: &mut DataStore, out: &mut [u8], dup: &mut Dup) {
    for i in 0..8usize {
        let prob = ds.u32();
        w32(out, i * 4, prob);
        if !cstring_is_empty(ds) {
            w32(out, 0x20 + i * 8, dup.alloc()); // text0
        }
        if !cstring_is_empty(ds) {
            w32(out, 0x24 + i * 8, dup.alloc()); // text1
        }
        let lang = ds.u32();
        w32(out, 0x60 + i * 4, lang);
        for j in 0..3usize {
            let emote_id = ds.u32();
            w32(out, 0x80 + i * 0xc + j * 4, emote_id);
            let emote_delay = ds.u32();
            w32(out, 0xe0 + i * 0xc + j * 4, emote_delay);
        }
    }
}
