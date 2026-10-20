//! The client's Lua 5.0 (`lobject.c`, `ltable.c`): the helpers whose exact output decides how a
//! table is laid out, and so the order `next` visits its keys in (`systems/lua/object.md`).

/// `luaO_int2fb` (`0x6f5810`), `__fastcall(ecx = x)` returning `eax`: a size as a byte whose low
/// three bits are a mantissa and whose high bits are a power of two, rounded up.
///
/// The loop is `while x >= 8 { x = (x + 1) >> 1; e += 1 }` (`0x6f5812` to `0x6f581e`, unsigned
/// compares), then `(e << 3) | x` (`0x6f5820`). `inc ecx` wraps, so `0xffffffff` becomes 0 after
/// one step and encodes as 8. `OP_NEWTABLE` decodes `(B & 7) << (B >> 3)` (`0x6f8b31`, `0x6f8b41`).
#[must_use]
pub fn luao_int2fb(mut x: u32) -> u32 {
    let mut e = 0u32;
    while x >= 8 {
        x = x.wrapping_add(1) >> 1;
        e += 1;
    }
    (e << 3) | x
}

/// `computesizes` (`0x6fab00`), `__fastcall(ecx = nums, edx = ntotal, narray, nhash)`, `ret 8`:
/// the array size `rehash` gives a table, and the count left to its hash part.
///
/// `nums[0]` counts key 1 and `nums[i]` the keys in `(2^(i-1), 2^i]`; `narray` is every key that
/// can go to the array. A size `2^i` is kept when the keys up to it reach half of it, `a >= 2^(i-1)`
/// (`0x6fab43`, `jl` at `0x6fab45`), and the walk stops once `narray < 2^(i-1)` (`0x6fab39`) or
/// every such key is counted (`0x6fab56`). A count is taken only when positive (`jle`, `0x6fab3f`).
/// The shifts are x86's, by the count modulo 32. Returns the new `narray`, `0` when no size is
/// kept (`0x6fab69`), and `nhash = ntotal - na` (`0x6fab64`), which the code stores first.
#[must_use]
pub fn computesizes(nums: &[i32], ntotal: i32, narray: i32) -> (i32, i32) {
    let mut a = nums[0];
    let mut na = a;
    let mut n: i32 = if a != 0 { 0 } else { -1 };
    if a < narray {
        // `ecx` is i - 1; `edx` walks `nums[i]`.
        let mut shift = 0u32;
        loop {
            let half = 1i32.wrapping_shl(shift);
            if narray < half {
                break;
            }
            let count = nums[shift as usize + 1];
            if count > 0 {
                a = a.wrapping_add(count);
                if a >= half {
                    n = shift as i32 + 1;
                    na = a;
                }
            }
            shift += 1;
            if a >= narray {
                break;
            }
        }
    }
    let nhash = ntotal.wrapping_sub(na);
    let size = if n == -1 {
        0
    } else {
        1i32.wrapping_shl(n as u32)
    };
    (size, nhash)
}
