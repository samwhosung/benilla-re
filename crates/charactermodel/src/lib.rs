//! The character component's selection and indexing arithmetic: the attach-slot mapping, the
//! CharSections record indexing, geoset and region selection, the InventoryType-to-body-slot
//! dispatchers, the texture blit kernels, and the container policy of the hash table and growable
//! array it uses.
//!
//! The component's math is integer, index, bitfield and pointer arithmetic, with no floating
//! point. The `neg al; sbb eax,eax; add eax,K` idiom is `K - (al != 0)`, so `attach_slot` case 1
//! returns `0x1b - (dl != 0)`, which is `0x1b` or `0x1a`.
#![allow(clippy::float_cmp)]

/// `0x47a070`: equipment attach-slot id by `case` (ecx in 0..4) and a left/right flag `dl`.
/// `cmp ecx,4; ja default; jmp [0x47a0a8+4*ecx]`; the cases use `mov al,dl; neg al; sbb eax,eax;
/// add eax,K`, which is `K - (dl != 0)`. `case 0` and `case > 4` return `-1`; `case 4` is the constant
/// `0x1c`.
pub fn attach_slot(case: u32, dl: u32) -> u32 {
    let b = u32::from(dl & 0xff != 0);
    match case {
        1 => 0x1b - b,
        2 => 0x1f - b,
        3 => 0x21 - b,
        4 => 0x1c,
        _ => 0xffff_ffff, // case 0 and the `ja` default
    }
}

/// `0x47a4c0`: race-id table indexer, `[[0xb4244c] + 4*(edx + 2*ecx)]` (`lea eax,[edx+2*ecx]; mov
/// ecx,[0xb4244c]; mov eax,[ecx+4*eax]`). `table` is the dereferenced `[0xb4244c]` array; the
/// arithmetic is the flat index `edx + 2*ecx`.
pub fn raceid_index(table: &[u32], ecx: u32, edx: u32) -> u32 {
    table[(edx + 2 * ecx) as usize]
}

/// `0x47b6f0`: TSLink offset resolver, `arg ? (*this + arg) : (this + 4)`
/// (`mov edx,[ebp+8]; test edx,edx; je L; mov eax,[ecx]; add eax,edx; ret; L: lea eax,[ecx+4]`).
/// `this_link0` is the value at `[this]` (read only when `arg != 0`).
pub fn tslink_offset(this: u32, this_link0: u32, arg: u32) -> u32 {
    if arg != 0 {
        this_link0.wrapping_add(arg)
    } else {
        this.wrapping_add(4)
    }
}

/// `0x4774f0`: section-slot occupancy predicate, `al = (arg0 != 0) && (*(arg0 + 4*idx + 0x18) != 0)`
/// (`ret 8`). `slot_val` is the dword read at `arg0 + 4*idx + 0x18`.
pub fn occupancy(arg0: u32, slot_val: u32) -> bool {
    arg0 != 0 && slot_val != 0
}

// ── CharSections-DBC fetch family ───────────────────────────────────────────────────────────────
// All four address the same record `rec = [0xb4231c] + 0x28*(sex + 2*race)` (`lea eax,[edx+2*ecx];
// lea eax,[eax+4*eax]; lea eax,[ecx+8*eax]` is base + 0x28*idx, the 40-byte CharSections row; ecx is
// race, edx is sex). They differ only in the guarded field or sub-dereference returned. The counts
// compare signed (`jle`, `jge`), so the guards are `> 0` and `<` on `i32`. The transcription is the
// guard and select given the fields the exe reads; the test places the record at `base + 0x28*idx`,
// so the index is tested too.

/// `0x47a3e0`: `[rec+0x1c] != 0 ? [rec+0x18] : 0`.
pub fn charsec_a(rec_1c: u32, rec_18: u32) -> u32 {
    if rec_1c != 0 {
        rec_18
    } else {
        0
    }
}

/// `0x47a400` (`ret 4`): `(rec[+0x1c]!=0 && rec[+0x18]>0 && arg < rec[+0x18]) ? [[rec+0x1c] + 12*arg] : 0`.
/// `sub` is the dword at `[rec+0x1c] + 12*arg`.
pub fn charsec_b(rec_1c: u32, rec_18: u32, arg: u32, sub: u32) -> u32 {
    if rec_1c != 0 && (rec_18 as i32) > 0 && (arg as i32) < (rec_18 as i32) {
        sub
    } else {
        0
    }
}

/// `0x47a440`: `(rec[+0x4]!=0 && rec[+0]>0) ? [[rec+0x4] + 4] : 0`. `sub4` is the dword at `[rec+0x4] + 4`.
pub fn charsec_c(rec_0: u32, rec_4: u32, sub4: u32) -> u32 {
    if rec_4 != 0 && (rec_0 as i32) > 0 {
        sub4
    } else {
        0
    }
}

/// `0x47a4a0`: `(rec[+0xc]!=0 && rec[+0x8]>0) ? rec[+0x8] : 0`.
pub fn charsec_e(rec_c: u32, rec_8: u32) -> u32 {
    if rec_c != 0 && (rec_8 as i32) > 0 {
        rec_8
    } else {
        0
    }
}

pub mod attach;
pub mod blit;
pub mod container;
pub mod dispatch;
pub mod equip;
pub mod misc;
pub mod selection;
