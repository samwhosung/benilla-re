//! The TSLink relocation, the TSGrowableArray capacity policy and the render-slot region-base
//! table.
//!
//! The arithmetic is integer, pointer and container arithmetic, with no floating point.

/// `0x47b6c0`: TSLink relocation resolver (`__thiscall`, `ret 4`). `a = [this+4]`:
/// - if `(a & 1) != 0` or `a == 0` → `a & ~1` (`and eax, -0x2`).
/// - else if `arg >= 0` (SIGNED, `jge`) → `a + arg` (`add eax, edx`).
/// - else → `a + (this - [[this]+4])`  (`edx=[this]; ecx = this - [edx+4]; add eax, ecx`).
///
/// `link_a = [this+4]`, `this_0 = [this]` (read only on the negative-arg branch),
/// `inner_4 = [[this]+4]` (the dword at `[this]+4`, read only on the negative-arg branch).
pub fn tslink_reloc(this: u32, link_a: u32, arg: u32, this_0: u32, inner_4: u32) -> u32 {
    if (link_a & 1) != 0 || link_a == 0 {
        link_a & !1
    } else if (arg as i32) >= 0 {
        link_a.wrapping_add(arg)
    } else {
        let _ = this_0; // [this] is dereferenced for inner_4; named for clarity
        link_a.wrapping_add(this.wrapping_sub(inner_4))
    }
}

/// The TSGrowableArray capacity policy shared by SetCount (`0x47bab0`) and Add-by-N (`0x47bbf0`) when the
/// stored capacity field `[this+0xc]` is zero (a fresh array): for `n >= 0x15` the capacity is 0x15 (21);
/// otherwise it is the largest power of two `<= n`, floored to 1 (the `lea eax,[ebx-1]; and eax,ebx`
/// clear-lowest-set-bit loop, then `if cap < 1 { cap = 1 }`). `n == 0` yields capacity 1.
pub fn ts_capacity(n: u32) -> u32 {
    if n >= 0x15 {
        0x15
    } else {
        // Largest power of two <= n: repeatedly clear the lowest set bit until one bit (or zero) remains.
        let mut cap = n;
        let mut t = n.wrapping_sub(1) & n;
        while t != 0 {
            cap = t;
            t = cap.wrapping_sub(1) & cap;
        }
        if cap < 1 {
            1
        } else {
            cap
        }
    }
}

/// Rounds `n` up to the nearest multiple of `cap` (the `div ecx; if rem { ecx -= rem; ecx += n }` idiom).
/// `cap` is never zero here (the policy floors it to 1). When `n % cap == 0` the size stays `n`.
fn round_up_to_multiple(n: u32, cap: u32) -> u32 {
    let rem = n % cap;
    if rem == 0 {
        n
    } else {
        n.wrapping_add(cap - rem)
    }
}

/// `0x47bab0`: TSGrowableArray::SetCount (`__thiscall`, `ret 4`), the size and capacity of the grow
/// path. On the grow path (new `count` above the current capacity `[this]`, stored capacity
/// `[this+0xc] == 0`) `cap = ts_capacity(count)` (also written back to `[this+0xc]`), and the realloc
/// size passed to `0x47bd80` is `round_up_to_multiple(count, cap)`. Returns `(capacity, size)`. The
/// realloc itself (`0x47bd80`) is not modelled; the test slices the function up to the `push esi`
/// that passes `size`.
pub fn ts_setcount_grow(count: u32) -> (u32, u32) {
    let cap = ts_capacity(count);
    let size = round_up_to_multiple(count, cap);
    (cap, size)
}

/// `0x47bbf0`: TSGrowableArray::Add-by-N (`__thiscall`, `ret 8`), the total and size of the grow
/// path. `total = old_count[this+4] + extra[ebp+8]`. On the grow path (`total` above the current
/// capacity `[this]`) with `arg1[ebp+0xc] != 0` and stored capacity `[this+0xc] == 0`, `cap =
/// ts_capacity(total)`, and the new size (stored to `[this]` and fed to the realloc) is
/// `round_up_to_multiple(total, cap)`. When `arg1 == 0` the size is the unrounded `total`. Returns
/// the final size `esi`. The realloc (`0x646320`) is not modelled; the test slices up to `cmp
/// esi,[ebx+4]`, where `esi` is final.
pub fn ts_add_n_size(old_count: u32, extra: u32, arg1_nonzero: bool) -> u32 {
    let total = old_count.wrapping_add(extra);
    if !arg1_nonzero {
        total
    } else {
        let cap = ts_capacity(total);
        round_up_to_multiple(total, cap)
    }
}

/// `0x47bd80`: TSGrowableArray::Grow (`__thiscall`, `ret 4`), the realloc byte size and the stored
/// count. The realloc byte size is `12 * newcount` (`lea eax,[edi+2*edi]; shl 2`, which is
/// `(3*newcount) << 2`); the element count `newcount` is stored at `[this]`. Returns the byte size.
/// The realloc (`0x646320`) is not modelled; the test slices up to the `call`.
pub fn ts_grow_bytesize(newcount: u32) -> u32 {
    newcount.wrapping_mul(12)
}

/// The 16 render-slot region-base constants that `0x476810` writes to struct offsets
/// `+0x144..+0x180` (step 4), in offset order. (`+0x160 = 0x2be` is stored out of code order; the
/// table below is sorted by offset.) Index `i` is offset `0x144 + 4*i`.
pub const REGION_BASES: [u32; 16] = [
    0x1,   // +0x144
    0x65,  // +0x148
    0xc9,  // +0x14c
    0x12d, // +0x150
    0x191, // +0x154
    0x1f5, // +0x158
    0x259, // +0x15c
    0x2be, // +0x160
    0x321, // +0x164
    0x385, // +0x168
    0x3e9, // +0x16c
    0x44d, // +0x170
    0x4b1, // +0x174
    0x515, // +0x178
    0x579, // +0x17c
    0x5dd, // +0x180
];

/// `0x476810` (constructor): the render-slot region-base table. Returns the 16 dwords it stores to
/// `[this+0x144 .. this+0x180]` (step 4), in offset order.
pub fn region_base_table() -> [u32; 16] {
    REGION_BASES
}
