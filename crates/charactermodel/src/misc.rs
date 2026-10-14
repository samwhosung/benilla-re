//! The hash-table bucket and rehash index arithmetic, the region-layer clear, the hand-grip
//! arguments, the helmet geoset flags, the equip-section filename indexing and the head-slot
//! precondition of the character component, with no floating point.
//!
//! The TSHashTable is a chained table whose buckets are a flat array of 12-byte cells, the sibling
//! of the TSGrowableArray in `container`. The bucket index is `key & mask`, with the mask
//! (capacity minus 1) at the global `[0xb4274c]`, and the cell address is `base + 12*(key & mask)`.
//! The create, insert and realloc calls it makes are not modelled.
#![allow(clippy::float_cmp)]

// ── TSHashTable bucket / rehash index math ────────────────────────────────────────────────────────

/// `0x47ae30`: TSHashTable get-or-create, the bucket-index arithmetic. With `mask = [0xb4274c]`
/// (capacity - 1) and `key` (the hash, `ebx`, the return of the call `0x64af90`): `idx = key &
/// mask`; `lea eax,[idx+2*idx]` is `3*idx`; `lea eax,[base + 4*eax + 4]` is the bucket-cell field
/// address `base + 12*idx + 4` (stride 12, `base = [0xb42744]`). On a hit the refcount
/// `[rec+0xbc]` is incremented; create and insert (`0x47aca0`, `0x47bab0`, `0x47b5c0`) are not
/// modelled. Returns `(idx, cell+4 address)`; the test reads `EAX = base + 12*idx + 4` from the
/// slice.
pub fn hashtable_bucket(key: u32, mask: u32, base: u32) -> (u32, u32) {
    let idx = key & mask;
    let cell4 = base.wrapping_add(12u32.wrapping_mul(idx)).wrapping_add(4);
    (idx, cell4)
}

/// `0x47b7d0`: TSHashTable rehash and grow, the new mask. The new bucket count is `n = [this+0x24]_old +
/// 1`, and at the tail the table's mask field is set to `n - 1`: `[ebp+8]` (the requested capacity)
/// goes to `ebx`, then `dec ebx; [this+0x24] = ebx`, so the stored mask is `arg - 1`. (`arg` is the
/// grown capacity.)
pub fn hashtable_new_mask(arg: u32) -> u32 {
    arg.wrapping_sub(1)
}

/// `0x47b7d0`: TSHashTable rehash, the re-bucket index. Each existing link node is placed at
/// `key & mask` with `mask = [this+0x24]` (the new `n-1`): `eax = [node]` (key); `and eax, mask`;
/// `lea eax,[eax+2*eax]` is `3*(key&mask)`; `lea ebx,[base + 4*eax]` is `base + 12*(key&mask)` (stride
/// 12, base = `[this+0x1c]`). Returns `(idx, cell address)`; the test reads `EBX = base + 12*idx`.
pub fn hashtable_rebucket(key: u32, mask: u32, base: u32) -> (u32, u32) {
    let idx = key & mask;
    let cell = base.wrapping_add(12u32.wrapping_mul(idx));
    (idx, cell)
}

// ── region-layer clear ────────────────────────────────────────────────────────────────────────────

/// `0x479180`: region-layer clear (`__thiscall`, `ret 8`; `ecx` is this, args `col` = `[ebp+8]`,
/// `row` = `[ebp+0xc]`). `eax = col; shl eax,4` gives `col<<4`; `edi = (col<<4) + row` is the flat
/// layer index. The parallel arrays are `[this + 4*edi + 0x228]` (the freed and zeroed slot) and
/// `[this + 4*edi + 0x248]` (zeroed unconditionally); the free `0x47b010` is not modelled. Returns the
/// flat index `(col<<4)+row`.
pub fn region_layer_clear_index(col: u32, row: u32) -> u32 {
    (col << 4).wrapping_add(row)
}

/// The two parallel-array slot offsets (relative to `this`) cleared for flat index `edi`:
/// `(+0x228 + 4*edi, +0x248 + 4*edi)`.
pub fn region_layer_clear_offsets(edi: u32) -> (u32, u32) {
    (
        0x228u32.wrapping_add(4u32.wrapping_mul(edi)),
        0x248u32.wrapping_add(4u32.wrapping_mul(edi)),
    )
}

// ── hand-grip arm and release ────────────────────────────────────────────────────────────────────
// These set the per-hand weapon grip, not a geoset: `0x7121a0` is the per-bone animation call, and the
// {8..=12} and {13..=17} values are finger key-bone ids. `0x479660` arms them with AnimationData id 15
// ("HandsClosed"); `0x4796b0` releases them (animId -1).

/// The inclusive finger key-bone range selected by the hand flag `edx`: `edx == 0` (main hand, right)
/// gives `{8..=12}` (`esi = 8`, `edi = 0xc`), else (off hand, left) `{13..=17}` (`esi = 0xd`, `edi =
/// 0x11`). Shared by `0x479660` (arm) and `0x4796b0` (release); the loop walks `id` from `esi` to
/// `edi` inclusive, calling `0x7121a0` (the animation call, not modelled) for each key-bone.
pub fn finger_keybone_range(hand: u32) -> (u32, u32) {
    if hand == 0 {
        (0x8, 0xc)
    } else {
        (0xd, 0x11)
    }
}

/// `0x479660`: hand-grip arm. For each key-bone `id` in `finger_keybone_range(edx)` it calls
/// `0x7121a0`. The pushes, in code order, are `push 1; push 0; push 0x3f800000; push 0; push -1; push
/// 0xf; push id`. In stack order (`[esp+4]` upward at the call, the reverse of code order) the args
/// are `[keyBoneId=id, animId=0xf (15 HandsClosed), variationIdx=-1, 0, blendRate=0x3f800000, 0,
/// linkFlag=1]`. Slot 1 is `0xf` (HandsClosed), the constant that differs from the release function.
pub fn hand_grip_arm_args(id: u32) -> [u32; 7] {
    [id, 0xf, 0xffff_ffff, 0, 0x3f80_0000, 0, 1]
}

/// `0x4796b0`: hand-grip release. The same shape as the arm function, but the second pushed value
/// differs: arm pushes `0xf` (`6a 0f`, animId 15), release pushes `-1` (`6a ff`, the disarm form of
/// the call, which returns the finger to its inherited Stand rest). The pushes, in code order, are
/// `push 1; push 0; push 0x3f800000; push 0; push -1; push -1; push id`. In stack order the args
/// are `[keyBoneId=id, animId=-1, variationIdx=-1, 0, blendRate=0x3f800000, 0, linkFlag=1]`.
pub fn hand_grip_release_args(id: u32) -> [u32; 7] {
    [id, 0xffff_ffff, 0xffff_ffff, 0, 0x3f80_0000, 0, 1]
}

// ── helmet and region geoset-flag resolver ───────────────────────────────────────────────────────

/// `0x4799a0`: the helmet and region geoset-flag resolver's bit test and constant writes (the tail at
/// `0x479ade`, after the string compose and sprintf calls). `mask = 1 << ([this+0x18] & 0x1f)` (the
/// `mov ecx,[esi+0x18]; mov eax,1; shl eax,cl` idiom; the x86 `shl` masks the count to 5 bits). For
/// the looked-up record `rec` (`edx`), each region field is tested against `mask`; a set bit writes a
/// fixed region-base constant to a struct geoset field:
/// - `[rec+0x4] & mask` → `[this+0x144] = 0x1`
/// - `[rec+0x8] & mask` → `[this+0x148] = 0x65`
/// - `[rec+0xc] & mask` → `[this+0x14c] = 0xc9`
/// - `[rec+0x10] & mask` → `[this+0x150] = 0x12d`
/// - `[rec+0x14] & mask` → `[this+0x160] = 0x2bd`
///
/// The texture loader it also calls is not modelled; this transcribes the `1<<race` bit test and the
/// five constant writes. Returns, per region slot, `Some((struct_offset, const))` when the bit was set
/// (a write occurred) or `None`. `rec` is `(f4, f8, fc, f10, f14)`, the five record region-flag words.
pub fn helmet_geoset_flags(race: u32, rec: (u32, u32, u32, u32, u32)) -> [Option<(u32, u32)>; 5] {
    let mask = 1u32 << (race & 0x1f);
    let (f4, f8, fc, f10, f14) = rec;
    [
        ((f4 & mask) != 0).then_some((0x144, 0x1)),
        ((f8 & mask) != 0).then_some((0x148, 0x65)),
        ((fc & mask) != 0).then_some((0x14c, 0xc9)),
        ((f10 & mask) != 0).then_some((0x150, 0x12d)),
        ((f14 & mask) != 0).then_some((0x160, 0x2bd)),
    ]
}

// ── equip-section filename gender patch ──────────────────────────────────────────────────────────

/// `0x476e20`: equip-section filename table indexing (the args the exe builds for the sprintf `0x64a7f0`,
/// not modelled). `eax = [ebp+0xc]` is `displayId`; the two table reads are `tbl0 = [0x838a94 +
/// 4*displayId]` and `tbl1 = [arg0 + 4*displayId + 0x38]` where `arg0 = [ebp+8]` (a per-record base).
/// Returns the two byte addresses read, given the bases; the test reads `EAX = 0x838a94 +
/// 4*displayId` and `EDX = arg0 + 4*displayId + 0x38` from the slice.
pub fn equip_filename_table_addrs(arg0: u32, display_id: u32) -> (u32, u32) {
    (
        0x838a94u32.wrapping_add(4u32.wrapping_mul(display_id)),
        arg0.wrapping_add(4u32.wrapping_mul(display_id))
            .wrapping_add(0x38),
    )
}

/// `0x476e20`: equip-section filename gender-letter patch (the branch where the path canonicalization
/// `0x648a10` fails). The canonicalized path lives in the global buffer `[0xb42510]`; its length comes
/// from a `repne scasb` (`ecx = strlen`), and the byte at `[0xb42510 + strlen - 5]` is overwritten with
/// the gender suffix letter, `*([0x838ab4 + 4*[this+0x1c]])` (the first byte of the suffix string for
/// the index `[this+0x1c]`). The patch position, relative to the buffer base, is `strlen - 5`. Returns
/// the patch offset given the buffer's string length.
pub fn equip_gender_patch_offset(strlen: u32) -> u32 {
    strlen.wrapping_sub(5)
}

// ── texture-composite layer callback (case 1) ─────────────────────────────────────────────────────

/// `0x477a70`: texture-composite layer callback, case 1 (`ecx == 1`). The three-case dispatch is `sub
/// ecx,0; je case0; dec ecx; jne case2` (case 2 does nothing; case 0 forwards to `0x477b50`, not
/// modelled). Case 1 (`ret 0x18`, six stack dwords; `edx` is the fastcall region index):
/// - `*([ebp+0x18]) = 2 * edx`, the region-to-texel double (the out-param pointer at stack slot +0x18).
/// - `*([ebp+0x1c]) = [[0xb422d8] + 4*[ebp+0x10]]`, the model-handle table lookup (table base
///   `[0xb422d8]`, index `[ebp+0x10]`), written through the out-param pointer at stack slot +0x1c.
///
/// Returns `(doubled, handle)`, the two values written through the out-params, given the region
/// index `edx`, the handle table and its index `[ebp+0x10]`.
pub fn texture_layer_case1(edx: u32, handle_table: &[u32], index: u32) -> (u32, u32) {
    let doubled = edx.wrapping_mul(2);
    let handle = handle_table[index as usize];
    (doubled, handle)
}

// ── helm geoset-visibility precondition: the head-slot handler's early-out chain ─────────────────

/// Whether the head `ItemDisplayInfo` row at `[cc+0x4a8]` is present, and if so the first byte of its
/// `ModelName[0]` string (in memory `[rec+0x04]`, on disk ItemDisplayInfo column 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadDisplay {
    /// `[cc+0x4a8]` is null: no head row is equipped.
    Absent,
    /// A row is equipped; the payload is `*([rec+0x04])`, which is `0` for an empty model name.
    Present(u8),
}

/// The head of `0x4799a0`, `[0x4799ac, 0x4799ef)`: the precondition that gates the helm geoset-visibility
/// tail.
///
/// The `HelmetGeosetVisData` hide ([`helmet_geoset_flags`], the tail at `0x479ab8`) is not applied
/// unconditionally when the head slot names a display row: the head of `0x4799a0` is a chain of five
/// early-outs, every one of which jumps to the shared epilogue `0x479b33`, past the whole tail
/// `[0x479ab8, 0x479b33)`. The chain, in code order:
///
/// 1. `0x4799b2 test eax,eax; 0x4799b5 je 0x479b33`: `eax = [esi+0x4a8]`, the head row pointer, is null.
/// 2. `0x4799bb mov ecx,[eax+4]; 0x4799be cmp byte ptr [ecx],0; 0x4799c1 je 0x479b33`: a dereference of
///    `ModelName[0]` (`[rec+0x04]`, column 1) tests its first byte, so an empty model name exits.
///    This tests that the string is empty, not that the pointer is null.
/// 3. `0x4799ca test eax,eax; 0x4799cc jl 0x479b33`: `race = [esi+0x18]` is negative.
/// 4. `0x4799d2 cmp eax,[0xc0dee4]; 0x4799d8 jg 0x479b33`: race above the race-DBC maximum id.
/// 5. `0x4799e7 test edi,edi; 0x4799e9 je 0x479b33`: `[0xc0dee0][race]`, the race row, is null.
///
/// Past 5 the body is straight-line (builds the helm filename, `call 0x4798c0` at `0x479ab3`) and falls
/// into the tail unconditionally: the sub-model load's return value is never tested (`eax` is
/// overwritten by `mov eax,[edx+ecx*4+0x30]` at `0x479ac1`). So the gate is decided by the DBC row's
/// bytes, never by whether the model file exists or finished loading.
///
/// Returns `true` if control reaches the tail, that is, if the hide is applied.
pub fn helmet_vis_reaches_tail(
    head: HeadDisplay,
    race: i32,
    race_max_id: i32,
    race_row_present: bool,
) -> bool {
    match head {
        HeadDisplay::Absent => false,     // 1: 0x4799b5
        HeadDisplay::Present(0) => false, // 2: 0x4799c1, an empty ModelName[0]
        HeadDisplay::Present(_) => {
            race >= 0                                 // 3: 0x4799cc
                && race <= race_max_id                // 4: 0x4799d8
                && race_row_present // 5: 0x4799e9
        }
    }
}
