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
