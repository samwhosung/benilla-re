//! `CM2Model::ResolveAnimationId` (`0x711bf0`), the animation id resolution
//! (`systems/models/animation.md#animation-id-resolution`): an id below the playable count reads
//! its pair from the playable table, and any other id walks the animation table's fallbacks to an
//! id the model has, with a class code. A 289-byte `thiscall` (`ret 8`) of integer code.
//!
//! - `0x711bf9`: `[this+0x30]` leads to the header at `+0x130`; the playable count is `[hdr+0x2c]`.
//! - `0x711c09`: an id below the count returns the dword of the playable table, with no class.
//! - `0x711c1f`: the default, used only when the walk ends in a loop: 0 when the lookup's entry 0
//!   is not 0xffff, else 0x93 when its entry at byte `0x126` is not 0xffff, else the dword
//!   `[hdr+0x20][0]`.
//! - `0x711c51` to `0x711cd5`: the cursor starts at the id, the direction at +1 and the
//!   accumulator at 0, with 208 visited flags clear. A step ends the walk with the cursor when
//!   `cursor < [hdr+0x24]` (unsigned) and the lookup's word at the cursor is not 0xffff. Otherwise
//!   the row is `[0xc0e070][cursor]` when `0 <= cursor <= [0xc0e074]` (signed, both ends
//!   included), else null; the walk ends in a loop when the cursor was visited, the row is null
//!   or the row's fallback is the cursor. Otherwise the cursor is marked visited and becomes the
//!   fallback `[row+0x18]`, then flag `0x10` of `[row+0x14]` adds the direction to the
//!   accumulator and negates the direction, and flag `0x20` adds it and zeroes it (`0x10` first).
//! - `0x711cdd`: the class is 0 for a positive direction, 1 for a negative one, else 2 plus
//!   whether the accumulator is positive; a walk that ends in a loop returns the default with
//!   class 0.
//! - `0x711cf8`: the result is `(class << 16) | (value & 0xffff)`.
//!
//! The exe reads the visited flag of the cursor before testing the row, even for a cursor outside
//! the table; the three loop exits meet at `0x711cd6`, so the bounded test here gives the same
//! result. The highest id must be below 208, or the visited flags overrun the
//! frame.

/// The model header as `0x711bf0` reads it (`[this+0x30]`, then `+0x130`).
pub struct AnimHeader<'a> {
    /// `[hdr+0x2c]`: the playable count.
    pub playable_count: u32,
    /// `[hdr+0x30]`: the playable table, one dword per id.
    pub playable: &'a [u32],
    /// `[hdr+0x28]`: the lookup, words probed at entries 0 and 0x93 for the default and at the
    /// cursor on each step.
    pub lookup: &'a [u16],
    /// `[hdr+0x20]`: the dword at `[0]` is the default when both probed words are 0xffff.
    pub default_ids: &'a [u32],
    /// `[hdr+0x24]`: the lookup length, the bound on the cursor (unsigned, exclusive).
    pub lookup_len: u32,
}

/// One row of the animation table (`[0xc0e070][i]` points at it; null is absent). The walk reads
/// `+0x14`, the flags, and `+0x18`, the fallback.
#[derive(Clone, Copy, Debug)]
pub struct AnimationRow {
    /// `[entry+0x14]`: flags `0x10` and `0x20` change the direction and the accumulator.
    pub flags: u32,
    /// `[entry+0x18]`: the fallback, the fallback cursor.
    pub fallback: u32,
}

/// The default (`0x711c1f` to `0x711c50`), the result of a walk that ends in a loop.
fn seed_for(hdr: &AnimHeader) -> u32 {
    if hdr.lookup[0] == 0xffff {
        if hdr.lookup[0x93] != 0xffff {
            0x93
        } else {
            hdr.default_ids[0]
        }
    } else {
        0
    }
}

/// The class from the final direction and accumulator (`0x711cdd` to `0x711cf5`): 0 for
/// `dir > 0`, 1 for `dir < 0`, else `2 + (acc > 0)` (`setg`).
fn classify(dir: i32, acc: i32) -> u32 {
    if dir > 0 {
        0
    } else if dir < 0 {
        1
    } else {
        2 + u32::from(acc > 0)
    }
}

/// `0x711bf0`. `hdr` is the header; `rows` is the animation table (`[0xc0e070]`, one slot
/// per id, `None` for a null pointer) with `max_id` = `[0xc0e074]`, the highest id, so the
/// slice must cover `0..=max_id`; `id` is the id (`[ebp+0xc]`). Returns the result
/// dword (`[ebp+8]`).
pub fn m2_bone_classify(
    hdr: &AnimHeader,
    rows: &[Option<AnimationRow>],
    max_id: u32,
    id: u32,
) -> u32 {
    // An id below the playable count returns its table dword (`0x711bf9` to `0x711c1c`).
    if id < hdr.playable_count {
        return hdr.playable[id as usize];
    }

    // The default.
    let seed = seed_for(hdr);

    // The walk.
    let mut visited = [false; 0xd0];
    let mut cursor = id;
    let mut dir: i32 = 1;
    let mut acc: i32 = 0;
    let (value, class) = loop {
        // The cursor is in the lookup and its word is not 0xffff (`0x711c71` to `0x711c7f`).
        if cursor < hdr.lookup_len && hdr.lookup[cursor as usize] != 0xffff {
            break (cursor, classify(dir, acc));
        }
        // The row (`0x711c81` to `0x711c9a`): `0 <= cursor <= count`, signed, both included.
        let entry = if (cursor as i32) >= 0 && (cursor as i32) <= max_id as i32 {
            rows[cursor as usize]
        } else {
            None
        };
        // The three loop exits (`0x711caa` visited, `0x711cae` null, `0x711cb5` fallback to
        // itself) meet at `0x711cd6`, so their order does not change the result.
        let Some(e) = entry else { break (seed, 0) };
        if visited[cursor as usize] {
            break (seed, 0);
        }
        if e.fallback == cursor {
            break (seed, 0);
        }
        // Mark, advance and apply the flags (`0x711cb7` to `0x711cd4`); both flags can apply.
        visited[cursor as usize] = true;
        cursor = e.fallback;
        if e.flags & 0x10 != 0 {
            acc += dir;
            dir = -dir;
        }
        if e.flags & 0x20 != 0 {
            acc += dir;
            dir = 0;
        }
    };

    // The result (`0x711cf8` to `0x711d0e`).
    (class << 16) | (value & 0xffff)
}
