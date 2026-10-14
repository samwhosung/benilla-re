//! The world map's selection (`systems/ui/worldmap.md#the-selection`, `#the-getters`): the three
//! cells `0x84506c`, `0x845070` and `0x845074` that `SetMap` (`0x4a67a0`) writes, and what
//! `GetMapInfo` reads from them. The head of `SetMap` holds no floating point.
//!
//! The cells are not a (continent, zone) pair. There are three states:
//!
//! | `[0x84506c]` | `[0x845070]` | `[0x845074]` | meaning |
//! |---|---|---|---|
//! | -1 | -1 | -1 | the world view |
//! | >= 0 | -1 or a child index | -1 | a continent, or a zone of it |
//! | -2 | -1 | a `WorldMapArea` row id | a direct area, an instance map |
//!
//! `[0x845074]` is the row id itself, not an index, and is -1 in the other two states.

/// What the head of `SetMap` (`[0x4a67a0, 0x4a67ef)`) stores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    /// `[0x84506c]`, stored unconditionally at `0x4a67ac` with no range check.
    pub continent: i32,
    /// `[0x845070]` (`0x4a67ba`, `0x4a67dd`, `0x4a67e4`).
    pub zone: i32,
    /// `[0x845074]`: `0x4a67c1` on the continent -2 leg, `0x4a67ea` on every other.
    pub direct_area: i32,
}

/// The head of `SetMap` (`0x4a67a0`, `ecx = continent`, `edx = zone`). `continent_count` is
/// `[0xb6e664]`; `child_count` is the selected continent's record `+0x0c`, read at `0x4a67d4`
/// only when the index is valid, so any value will do otherwise.
///
/// The store of the continent at `0x4a67ac` sits between the compare and its jump and does not
/// change the flags, so it is unconditional. The index test is unsigned (`jb 0x4a67c9`), so -1 and
/// -2 are not valid indexes. On a valid continent `[0x845070]` is written -1 first, which does not
/// change the flags either, and then kept as `edx` only when `edx < child_count` unsigned (`jae
/// 0x4a67ea`); the direct area is -1. Otherwise the zone is -1 and the direct area is `edx` when the
/// continent is exactly -2 (`0x4a67b7`, `0x4a67bf`), else -1.
pub fn set_map(continent: i32, edx: i32, continent_count: u32, child_count: u32) -> Selection {
    let stored_continent = continent;
    if (continent as u32) < continent_count {
        let zone = if (edx as u32) < child_count { edx } else { -1 };
        Selection {
            continent: stored_continent,
            zone,
            direct_area: -1,
        }
    } else {
        let direct_area = if continent == -2 { edx } else { -1 };
        Selection {
            continent: stored_continent,
            zone: -1,
            direct_area,
        }
    }
}

/// The `WorldMapArea` row id that the selection names: the common head of `GetSelectedAreaName`
/// (`0x4a6cf0`, `[0x4a6cf0, 0x4a6d29)`) and `GetSelectedTextureHeight` (`0x4a6d50`, `[0x4a6d54,
/// 0x4a6d87)`). A continent below 0 (`jl 0x4a6d24`, both -1 and -2) gives the direct area; a zone
/// below 0 gives the continent's own row, `continent_own_wma` (`record+0x04`); otherwise the zone's,
/// `zone_child_wma` (`((int*)record+0x10)[zone]`).
pub fn selected_wma_id(sel: Selection, continent_own_wma: i32, zone_child_wma: i32) -> i32 {
    if sel.continent < 0 {
        sel.direct_area
    } else if sel.zone < 0 {
        continent_own_wma
    } else {
        zone_child_wma
    }
}

/// The low byte of the second value `GetMapInfo` pushes, rounded up to a power of two: `eax` at the
/// join `0x4a7e9a`, a jump target from `0x4a7e77` and `0x4a7e90`. `test eax, ecx` with `ecx = low - 1`
/// (`0x4a7e75`) skips the round-up for 0 and for every power of two. Otherwise `cl` walks down from
/// 7 to the highest set bit and the result is `1 << (cl + 1)` (`0x4a7e79` to `0x4a7e98`). The
/// return for `cl == 0` (`0x4a7e8e`, `0x4a7e90`) is not reached for any value that passes the first
/// test.
pub fn pot_round_low(low: i32) -> i32 {
    if low & (low.wrapping_sub(1)) == 0 {
        return low;
    }
    let mut cl: u32 = 8;
    loop {
        cl -= 1;
        if low & (1i32 << cl) != 0 {
            return 1i32 << (cl + 1);
        }
        if cl == 0 {
            return low;
        }
    }
}

/// The third value of `GetMapInfo` (`0x4a7e69` to `0x4a7e9a`, applied to the result of `0x4a6d50`):
/// the value with its low byte cleared plus that low byte rounded up by [`pot_round_low`] (`add
/// eax, esi`, `0x4a7e9a`).
pub fn pot_round_low_byte(v: i32) -> i32 {
    let low = v & 0xff;
    let high = v - low;
    high + pot_round_low(low)
}
