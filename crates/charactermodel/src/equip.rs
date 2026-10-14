//! The texture-apply index arithmetic and the equipment-slot cache and dispatch index arithmetic
//! of the character component, with no floating point.
//!
//! The CharSections addressing is `idx = sex + 2*race` with sex at `[this+0x1c]`, race at
//! `[this+0x18]`, the array base `[0xb4231c]` and a 0x28-byte row. The texture handlers (`0x478160`,
//! `0x478220`, `0x4782e0`) walk a nested dereference off that row to a texture id, clamp it to
//! `[0, [0xc0df70]]` (signed) and index the name table `[0xc0df6c][texid]`; an id that fails the
//! clamp yields 0 (`xor eax,eax`). The equipment handlers (`0x478900`, `0x478960`, `0x478ad0`)
//! compute flat slot and array indices (`(col<<4)+row`, `4*slot+0x4a8`) and a `5*slot` index into
//! the validity table at `0x803bf8`. The calls out (texture creation at `0x449d90`, `0x58a980`,
//! `0x710ec0`, the free at `0x47b010`, the dispatch table at `[0xb42424]`) are not modelled.
#![allow(clippy::float_cmp)]

// ── Texture-id clamp and name-table index (shared by 0x478160, 0x478220 and 0x4782e0) ────────────
//
// The clamp idiom is `test eax,eax; jl xor; cmp eax,[0xc0df70]; jg xor; ecx=[0xc0df6c];
// eax=[ecx+4*eax]; ... xor: eax=0`:
//   if (texid as i32) < 0                      -> 0
//   else if texid > [0xc0df70] (signed `jg`)   -> 0
//   else                                       -> [0xc0df6c][texid]   (the name-table entry dword)
// The upper bound and the table are globals; `name_tbl` is the dereferenced `[0xc0df6c]` array and
// `max` is `[0xc0df70]`. Returns the clamped name-table dword (0 when the clamp rejects).
pub fn texid_name(texid: u32, max: u32, name_tbl: &[u32]) -> u32 {
    if (texid as i32) < 0 || (texid as i32) > (max as i32) {
        0
    } else {
        name_tbl[texid as usize]
    }
}

/// `0x478160`: base-skin section apply. The arithmetic before the texture-create calls:
/// - `esi = [this+0x24]` (the section sub-index), gated by `0x477c20(0, 0, esi)`.
/// - `idx = [this+0x1c] + 2*[this+0x18]` (sex + 2*race); `rec4 = [[0xb4231c] + 0x28*idx + 4]`
///   (the CharSections row field +4, since `8*(5*idx)+4 = 0x28*idx+4`).
/// - `texid = [[rec4 + 0x8] + 4*esi]` (a nested dereference).
/// - then the shared clamp and name-table index ([`texid_name`]).
///
/// This returns the clamped name-table value the exe loads into EAX at `0x4781aa`, after the clamp
/// and before the byte copy and texture create. `texid` and the globals are read back from emulator
/// memory, so the index, the nested dereference and the clamp are all tested. The test reads the
/// clamped EAX from the slice that runs to `0x4781ad`, just past the clamp.
pub fn base_skin_texid_name(texid: u32, max: u32, name_tbl: &[u32]) -> u32 {
    texid_name(texid, max, name_tbl)
}

/// `0x478220`: face and facial-hair section apply (`ret 4`; arg0 = `[ebp+8]`). The nested DBC index and
/// the clamp before the texture-create calls:
/// - `idx = [this+0x1c] + 2*[this+0x18]` (sex + 2*race); `rec1c = [[0xb4231c] + 0x28*idx + 0x1c]`
///   (the CharSections row field +0x1c, since `8*(5*idx)+0x1c = 0x28*idx+0x1c`).
/// - inner stride: `sub = [rec1c + 4*(3*arg0) + 0x8]` (`lea eax,[esi+2*esi]` is 3*arg0, scaled by 4,
///   plus 8); then `texid = [sub + 4*[this+0x20]]`.
/// - then the shared clamp and name-table index ([`texid_name`]).
///
/// Returns the clamped name-table value at `0x478273`, after the clamp. The test reads `texid` and the
/// globals back from emulator memory and tests the clamp from the slice that runs to `0x478276`.
pub fn face_texid_name(texid: u32, max: u32, name_tbl: &[u32]) -> u32 {
    texid_name(texid, max, name_tbl)
}

/// `0x4782e0`: region-layer compositor (`ret 0x14`; ecx = this, args `[ebp+8..0x18]`). The layer-slot
/// pointer index and the dirty-bit mask (the texid clamp also lives here and is the same
/// [`texid_name`] arithmetic, tested through `0x478160` and `0x478220`).
///
/// The slot-pointer index is `lea ecx,[eax+8*ebx]; lea edi,[esi+4*ecx+0x188]`, with `ebx = [ebp+8]`
/// (slot) and `eax = [ebp+0xc]` (sub_idx):
///   `edi = this + 4*(sub_idx + 8*slot) + 0x188`
/// Returns the byte offset from `this`, `4*(sub_idx + 8*slot) + 0x188`. The test reads EDI from the
/// slice that runs to `0x4782f9`, just after `lea edi`, and subtracts `this`.
pub fn region_layer_slot_off(slot: u32, sub_idx: u32) -> u32 {
    4u32.wrapping_mul(sub_idx.wrapping_add(8u32.wrapping_mul(slot)))
        .wrapping_add(0x188)
}

/// The `0x4782e0` dirty-bit mask: at the tail (`mov ecx,[ebp+0x18]; mov eax,[esi+0x10]; mov edx,1;
/// shl edx,cl; or eax,edx; mov [esi+0x10],eax`) the compositor sets `[this+0x10] |= (1 << bit_no)`,
/// where `bit_no = [ebp+0x18]` (arg5). The x86 `shl` masks the count to 5 bits, so the shift is
/// `bit_no & 0x1f`. Returns the mask `1 << (bit_no & 0x1f)`. The test reads the OR result in
/// `[this+0x10]` from a whole run: it stubs the calls out, makes `0x477c20` pass, runs to `ret`
/// and reads `[this+0x10]`.
pub fn region_layer_dirty_mask(bit_no: u32) -> u32 {
    1u32 << (bit_no & 0x1f)
}

/// The flat index of `0x478900`, the equip attach-slot cache store (`ret 0xc`; ecx = this, col =
/// `[ebp+8]`, new_ptr = `[ebp+0xc]`, row = `[ebp+0x10]`): `esi = (col << 4) + row` (`shl eax,4` then
/// `lea esi,[eax+edx]`), indexing the parallel arrays `[this+4*esi+0x228]` (texture) and
/// `[this+4*esi+0x248]` (source). Returns the flat slot index `esi`.
pub fn attach_cache_index(col: u32, row: u32) -> u32 {
    (col << 4).wrapping_add(row)
}

/// The store outcome of `0x478900`:
/// - if `[this+0x3c] != 0`, nothing happens (early return).
/// - `cur = [this + 4*esi + 0x248]`. If `cur != 0` and `[cur+4*col+0x38] == [new_ptr+4*col+0x38]`,
///   it is a cache hit and nothing happens.
/// - otherwise the `+0x228` texture entry is freed if set, then `[this+4*esi+0x248] = new_ptr` is
///   stored.
///
/// `flag_3c` = `[this+0x3c]`; `cur` = `[this+4*esi+0x248]`; `cur_eq` = `[cur+4*col+0x38]` (read only
/// when `cur != 0`); `new_eq` = `[new_ptr+4*col+0x38]`. Returns whether the store fires.
pub fn attach_cache_stores(flag_3c: u32, cur: u32, cur_eq: u32, new_eq: u32) -> bool {
    if flag_3c != 0 {
        return false; // [this+0x3c] != 0: nothing happens
    }
    if cur != 0 && cur_eq == new_eq {
        return false; // cache hit: nothing happens
    }
    true // frees the old +0x228 entry if set, stores new_ptr into +0x248
}

/// `0x478960`: equip slot dispatch by item class (`ret 4`; ecx = this, slot = `[ebp+8]`). The slot
/// index is `4*slot + 0x4a8` (`mov eax,[esi+4*ebx+0x4a8]`). When `[this+4*slot+0x4a8] == 0` the
/// function returns at once with no writes; otherwise `[this+0xa] = 1` and a per-slot branch runs.
/// The per-slot writes (the region-offset adds and the literal display constants):
/// - slot 0: `[this+0x144] = 0x478540(...)`, region adds `[this+0x148]=[rec+0x18]+0x64`,
///   `[this+0x150]=[rec+0x1c]+0x12c`, `[this+0x14c]=[rec+0x20]+0xc8` (only if the `0x478740` rec is
///   non-zero), `[this+0x160]=0x2be`, then `[this+0x4a8]=0`.
/// - slot 1: `[this+0x4ac]=0` (after two `0x47a380` calls).
/// - slot 0xb: `[this+0x4d4]=0`.
/// - slot 0xa: `[this+0x180]=0x5dd`, `[this+0x4d0]=0`.
/// - slot 9: `[this+0x180]=0x5dd`, then falls into the default dispatch-table walk.
/// - default: `[this+4*slot+0x4a8]=0`, then the `[0xb42424..0xb42444)` dispatch walk.
///
/// The tests cover the slot index `4*slot+0x4a8`, the three region-offset adds (slot 0) and the
/// constant 0x2be. Returns the byte offset of the slot table entry.
pub fn equip_slot_index(slot: u32) -> u32 {
    4u32.wrapping_mul(slot).wrapping_add(0x4a8)
}

/// The slot-0 region adds of `0x478960` (the same mapping as the geoset base of `0x478660`): `rec` is
/// the `0x478740` region record (`None` means rec == 0 and no adds), as `(rec_18, rec_1c, rec_20)`.
pub struct EquipSlot0 {
    pub dest_148: Option<u32>,
    pub dest_150: Option<u32>,
    pub dest_14c: Option<u32>,
    /// `[this+0x160] = 0x2be` is written only when `rec != 0`.
    pub dest_160: Option<u32>,
}

/// The slot-0 region-offset adds of `0x478960`: `[this+0x148]=rec_18+0x64`, `[this+0x150]=rec_1c+0x12c`,
/// `[this+0x14c]=rec_20+0xc8` and `[this+0x160]=0x2be`, all guarded on `rec != 0`.
pub fn equip_slot0_region(rec: Option<(u32, u32, u32)>) -> EquipSlot0 {
    match rec {
        Some((r18, r1c, r20)) => EquipSlot0 {
            dest_148: Some(r18.wrapping_add(0x64)),
            dest_150: Some(r1c.wrapping_add(0x12c)),
            dest_14c: Some(r20.wrapping_add(0xc8)),
            dest_160: Some(0x2be),
        },
        None => EquipSlot0 {
            dest_148: None,
            dest_150: None,
            dest_14c: None,
            dest_160: None,
        },
    }
}

/// `0x478ad0`: equip item into slot (`ret 8`; ecx = this, slot = `[ebp+8]`, new_ptr = `[ebp+0xc]`). The
/// slot index is `4*slot + 0x4a8` (the same as [`equip_slot_index`]); `[this+4*slot+0x4a8]=new_ptr`
/// is stored unconditionally near the entry (`0x478ae1`). Returns that byte offset.
pub fn equip_item_slot_index(slot: u32) -> u32 {
    4u32.wrapping_mul(slot).wrapping_add(0x4a8)
}

/// The `0x478ad0` default-branch validity-table index (`lea edx,[edi+4*edi]` is 5*slot; `lea
/// ecx,[esi+2*edx]` is esi + 2*(5*slot); `cmp [4*ecx+0x803bf8], -1`): the probe for (slot, i) is
/// `[0x803bf8 + 4*(i + 10*slot)] != -1`, with `i` the loop counter 0 to 8. Returns the element index
/// `i + 10*slot` (the dword index relative to `0x803bf8`, before the scale of 4).
pub fn equip_validity_index(slot: u32, i: u32) -> u32 {
    i.wrapping_add(10u32.wrapping_mul(slot))
}
