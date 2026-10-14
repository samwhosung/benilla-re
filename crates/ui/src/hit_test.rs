//! Hit testing of `CSimpleFrame` and `CSimpleTop` (`systems/ui/input.md#hit-test`,
//! `#input-lists`): whether a point is inside a frame, and where a frame registers in the input
//! lists.
//!
//! Integer and pointer logic is exact. The one float kernel is the point test: the rectangle edges
//! are `f32` compared with `fcomp; fnstsw; test ah, mask; j(n)p`. Each compare runs in `f64`, which
//! is exact for `f32` edges, and the value chosen is always one of the two `f32` operands copied
//! through, so nothing rounds. A NaN is unordered, which the binary treats as outside.

/// `CSimpleFrame::IsMouseOver` (`0x76b020`, `__thiscall`, `ecx` the frame, the argument a `float[2]`
/// point `{x at +0, y at +4}`, the result `al`): whether the frame's rectangle holds the point.
///
/// The rectangle is `own` = `[bottom, left, top, right]` at `+0xd8`, `+0xdc`, `+0xe0`, `+0xe4`.
/// `cache_valid` is bit 0 of the flags at `+0x60`; `clip_self` is bit `0x2000` of `+0xb4`.
///
/// - `cache_valid` clear: false (`test [ecx+0x60], 1; jz`).
/// - `clip_self` clear: the point is tested against `own`.
/// - `clip_self` set: the chain of parents (`+0x9c`) is walked to the nearest ancestor without the
///   `0x2000` bit, and `clip` is that ancestor's rectangle; with none (`clip == None`) the answer is
///   false. Otherwise the point is tested against the intersection of `own` and `clip`: the larger
///   of the low edges (bottom, left) and the smaller of the high edges (top, right). The picks
///   take the frame's own edge on a tie and on a NaN.
///
/// The point is inside when `left <= x <= right` and `bottom <= y <= top`; each compare fails on a
/// strict miss and on a NaN, as `>=` and `<=` do.
pub fn point_in_rect(
    cache_valid: bool,
    clip_self: bool,
    own: [f32; 4],
    clip: Option<[f32; 4]>,
    px: f32,
    py: f32,
) -> bool {
    if !cache_valid {
        return false;
    }
    let rect = if !clip_self {
        own
    } else {
        match clip {
            None => return false,
            Some(anc) => intersect_rect(own, anc),
        }
    };
    point_inside(rect, px, py)
}

/// The per-edge intersection: the low edges (`[0]` bottom, `[1]` left) take the larger value, the
/// high edges (`[2]` top, `[3]` right) the smaller. A tie or a NaN keeps `own`.
fn intersect_rect(own: [f32; 4], anc: [f32; 4]) -> [f32; 4] {
    [
        pick_low(own[0], anc[0]),
        pick_low(own[1], anc[1]),
        pick_high(own[2], anc[2]),
        pick_high(own[3], anc[3]),
    ]
}

/// A low edge (`fld anc; fcomp own` then `test ah, 0x41; jne use-own`): the ancestor's wins only
/// when it is strictly greater and the compare is ordered.
fn pick_low(own: f32, anc: f32) -> f32 {
    if f64::from(anc) > f64::from(own) {
        anc
    } else {
        own
    }
}

/// A high edge (`fld anc; fld own; fcompp` then `test ah, 0x41; jne use-own`): the ancestor's wins
/// only when it is strictly smaller and the compare is ordered.
fn pick_high(own: f32, anc: f32) -> f32 {
    if f64::from(anc) < f64::from(own) {
        anc
    } else {
        own
    }
}

/// `left <= x <= right && bottom <= y <= top` for `rect = [bottom, left, top, right]`; a NaN makes
/// a compare false, so the point is outside, as the binary's unordered `fcomp` fails.
fn point_inside(rect: [f32; 4], px: f32, py: f32) -> bool {
    let (bottom, left, top, right) = (
        f64::from(rect[0]),
        f64::from(rect[1]),
        f64::from(rect[2]),
        f64::from(rect[3]),
    );
    let (x, y) = (f64::from(px), f64::from(py));
    x >= left && x <= right && y >= bottom && y <= top
}

/// Where `CSimpleTop::RegisterForEvent` inserts a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchInsert {
    /// `frame_strata + category * 9`: the index into the 36 input lists at `CSimpleTop+0xd10` (each
    /// `0x1c` bytes, so the byte offset is `bucket_index * 0x1c`).
    pub bucket_index: u32,
    /// The position of the new node in the list, which stays sorted by level, highest first: the
    /// first index whose entry is strictly below `level`, else the end.
    pub insert_pos: usize,
}

/// `CSimpleTop::RegisterForEvent` (`0x764aa0`, `__thiscall`, the arguments `frame, category, flag,
/// level`, `ret 0x10`; `input#input-lists`). The lists are the topmost-first hit-test index that the
/// hover walk `0x7660d0` reads. The list is `frame_strata (frame+0xc0) + category * 9`. A `level_arg`
/// of `0xffff_ffff` means the frame's own level (`frame+0xc4`). The 8-byte node `{frame, level}` is
/// inserted at the first position whose existing level is strictly below this one (`cmp; ja`,
/// unsigned), which keeps the list in descending order, else appended.
///
/// The growth of the list, the allocation of the node, the high-water mark and the store
/// `[this+0x1100] = 1` when `flag == 0` and `category == 2` are not part of this value.
pub fn register_for_event_insert(
    frame_strata: u32,
    category: u32,
    level_arg: u32,
    frame_level: u32,
    existing_levels: &[u32],
) -> BatchInsert {
    let bucket_index = frame_strata + category * 9;
    let level = if level_arg == 0xffff_ffff {
        frame_level
    } else {
        level_arg
    };
    let mut insert_pos = existing_levels.len();
    for (i, &entry) in existing_levels.iter().enumerate() {
        if level > entry {
            insert_pos = i;
            break;
        }
    }
    BatchInsert {
        bucket_index,
        insert_pos,
    }
}
