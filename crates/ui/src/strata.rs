//! The strata and levels of `CSimpleTop` (`systems/ui/strata.md`): raising a frame, compacting a
//! stratum's levels, the stratum and level setters, and the order of the update pass.
//!
//! The setters and the raise recurse over the frame list (`+0x300`) and change the top's strata
//! structure. All control flow is integer and pointer logic; the one float is the rectangle overlap
//! of the raise, which runs in `f64` (exact for `f32` edges): the value chosen is always one `f32`
//! operand copied through a `fcom` minimum or maximum, so nothing rounds, and a NaN compare is
//! false as `<` is in Rust.

/// A move between strata levels, in call order. `frame` is the caller's frame index. The binary's
/// remove and add are `__thiscall` on the top with the arguments `(frame, 1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BucketOp {
    /// `CSimpleTop::HideFrame` (`0x764920`): takes the frame out of its stratum level.
    Remove(usize),
    /// `CSimpleTop::ShowFrame` (`0x764780`): puts the frame into its (new) stratum level.
    Add(usize),
}

/// A frame of a [`set_frame_strata`] subtree.
#[derive(Clone, Debug)]
pub struct StrataFrame {
    /// `frame+0xc0`: the stratum.
    pub strata: u32,
    /// `frame+0xd4`: effective visibility; a visible frame is removed and added.
    pub visible: bool,
    /// The frame list at `+0x300`, head to next; the frame of a node is at `node+8`.
    pub children: Vec<usize>,
}

/// `CSimpleFrame::SetFrameStrata` (`0x76a470`, `__thiscall(this, new_strata)`, `ret 4`;
/// `strata#stratum-and-level-setters`): `if (new_strata != strata) { if visible remove; strata =
/// new_strata; if visible add; for each child: set_frame_strata(child, new_strata) }`. The whole
/// subtree takes the stratum; a child already at `new_strata` returns at once, so it stops the
/// recursion below it. The removes and adds are appended to `ops`, and each frame's stratum is
/// changed in place.
pub fn set_frame_strata(
    frames: &mut [StrataFrame],
    id: usize,
    new_strata: u32,
    ops: &mut Vec<BucketOp>,
) {
    if new_strata == frames[id].strata {
        return;
    }
    if frames[id].visible {
        ops.push(BucketOp::Remove(id));
    }
    frames[id].strata = new_strata;
    if frames[id].visible {
        ops.push(BucketOp::Add(id));
    }
    let kids = frames[id].children.clone();
    for c in kids {
        set_frame_strata(frames, c, new_strata, ops);
    }
}

/// A frame of a [`set_frame_level`] subtree.
#[derive(Clone, Debug)]
pub struct LevelFrame {
    /// `frame+0xc0`: the stratum, which gates the recursion and is not changed here.
    pub strata: u32,
    /// `frame+0xc4`: the level, compared as signed and kept at 0 or more.
    pub level: i64,
    /// `frame+0xd4`: effective visibility.
    pub visible: bool,
    /// The frame list at `+0x300`, head to next.
    pub children: Vec<usize>,
}

/// `CSimpleFrame::SetFrameLevel` (`0x76a4f0`, `__thiscall(this, new_level, propagate)`, `ret 8`):
/// `if (new_level < 0) new_level = 0; if (new_level != level) { delta = new_level - level; if
/// visible remove; level += delta; if visible add; if propagate, for each child with the same
/// stratum: set_frame_level(child, child.level + delta, 1) }`. Children in the same stratum move by
/// the same difference, so their offsets are kept; children in another stratum are skipped. The
/// recursion clamps again, so a child whose `level + delta` is negative goes to 0 and its own
/// difference differs. The removes and adds are appended to `ops`, and each level is changed in
/// place.
pub fn set_frame_level(
    frames: &mut [LevelFrame],
    id: usize,
    new_level: i64,
    propagate: bool,
    ops: &mut Vec<BucketOp>,
) {
    let nl = if new_level < 0 { 0 } else { new_level };
    if nl == frames[id].level {
        return;
    }
    let delta = nl - frames[id].level;
    if frames[id].visible {
        ops.push(BucketOp::Remove(id));
    }
    frames[id].level += delta;
    if frames[id].visible {
        ops.push(BucketOp::Add(id));
    }
    if propagate {
        let kids = frames[id].children.clone();
        let my_strata = frames[id].strata;
        for c in kids {
            if frames[c].strata == my_strata {
                let child_new = frames[c].level + delta;
                set_frame_level(frames, c, child_new, true, ops);
            }
        }
    }
}

/// A frame of a [`raise_frame`] scene.
#[derive(Clone, Debug)]
pub struct AutoFrame {
    /// `frame+0xb4 & 1`: the toplevel bit, which the walk up the parents looks for.
    pub toplevel: bool,
    /// `frame+0xb4 & 0x10`: the overlap bit, which gates the raise and which the scan overwrites.
    pub autoraise: bool,
    /// `frame+0x9c`: the parent.
    pub parent: Option<usize>,
    /// `frame+0xc0`: the stratum, which selects the list.
    pub strata: u32,
    /// `frame+0xc4`: the level, where the scan of the resolved toplevel starts.
    pub level: u32,
    /// `frame+0x64`: the rectangle `[v0, v1, v2, v3]` that `CLayoutFrame::GetRect` (`0x768320`)
    /// copies out.
    pub rect: [f32; 4],
}

/// What [`raise_frame`] does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoResult {
    /// `eax`: 1 when a toplevel frame was found and handled, 0 when there is none.
    pub ret: u32,
    /// Whether the raise ran: `CompactStrata`, `SetFrameLevel` and `root+0x1100 = 1`.
    pub raise: bool,
    /// The `new_level` given to `SetFrameLevel`, the list's level count, when the raise runs.
    pub set_level_arg: Option<u32>,
}

/// The rectangle overlap (`RectIntersect` `0x766c70`, then `RectIsNotEmpty` `0x766bc0`): the
/// intersection is `[max(a0, b0), max(a1, b1), min(a2, b2), min(a3, b3)]`, and the rectangles
/// overlap when `r0 < r2 && r1 < r3`, strictly. The max and min copy one operand through, with no
/// arithmetic.
pub fn rect_overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    let max = |x: f32, y: f32| if f64::from(y) > f64::from(x) { y } else { x };
    let min = |x: f32, y: f32| if f64::from(y) < f64::from(x) { y } else { x };
    let r0 = max(a[0], b[0]);
    let r1 = max(a[1], b[1]);
    let r2 = min(a[2], b[2]);
    let r3 = min(a[3], b[3]);
    f64::from(r0) < f64::from(r2) && f64::from(r1) < f64::from(r3)
}

/// Whether `candidate` is in the parent chain of the frame whose parents are `ancestors`
/// (`IsAncestorOf`, `0x767010`, `__thiscall(this = frame, candidate)`): the walk goes up `+0x9c`
/// comparing each parent with `candidate` and stops at a match (true) or at a null parent (false).
/// The starting frame is never compared, so a frame is not its own descendant, and the null at the
/// end is never compared, so a `candidate` of 0 is always false. `ancestors` is the chain of
/// non-null parents starting at `frame->parent`. The loop has no cycle guard; the frame tree has no
/// cycle.
pub fn is_descendant(ancestors: &[u32], candidate: u32) -> bool {
    for &a in ancestors {
        if a == candidate {
            return true;
        }
    }
    false
}

/// [`is_descendant`] over a scene: whether `target` is in `start`'s parent chain.
fn frame_is_descendant(frames: &[AutoFrame], start: usize, target: usize) -> bool {
    let mut p = frames[start].parent;
    while let Some(a) = p {
        if a == target {
            return true;
        }
        p = frames[a].parent;
    }
    false
}

/// `CSimpleTop::RaiseFrame` (`0x7650f0`, `__thiscall(this = top, frame, do_scan)`, `ret 8`;
/// `strata#raising-a-frame`). The toplevel is the frame itself when it has the toplevel bit, else
/// the first parent (`+0x9c`) that does. With none the function returns 0 and does nothing.
/// Otherwise it returns 1 and
///
/// - with `do_scan`, scans the toplevel's stratum from its own level up to the level count over
///   each level's frame list: a frame that is not the toplevel itself, is not a descendant of it
///   and has a rectangle that overlaps the toplevel's is an overlap. The toplevel's overlap bit
///   (`+0xb4 & 0x10`) is set to whether one was found;
/// - raises when the overlap bit is then set: `CompactStrata(strata)`, `SetFrameLevel(toplevel,
///   level_count, 1)` and `top+0x1100 = 1`.
///
/// `bucket_levels[lvl]` is the frames in that level's list, in list order, and `level_count` is the
/// record's `+0x8`. The toplevel's overlap bit is changed in `frames` when `do_scan`.
pub fn raise_frame(
    frames: &mut [AutoFrame],
    bucket_levels: &[Vec<usize>],
    level_count: u32,
    frame: usize,
    do_scan: bool,
) -> AutoResult {
    let tl = if frames[frame].toplevel {
        Some(frame)
    } else {
        let mut p = frames[frame].parent;
        let mut found = None;
        while let Some(a) = p {
            if frames[a].toplevel {
                found = Some(a);
                break;
            }
            p = frames[a].parent;
        }
        found
    };
    let Some(tl) = tl else {
        return AutoResult {
            ret: 0,
            raise: false,
            set_level_arg: None,
        };
    };

    if do_scan {
        let mut overlap = false;
        'outer: for lvl in frames[tl].level..level_count {
            if let Some(list) = bucket_levels.get(lvl as usize) {
                for &c in list {
                    if c == tl || frame_is_descendant(frames, c, tl) {
                        continue;
                    }
                    if rect_overlap(frames[tl].rect, frames[c].rect) {
                        overlap = true;
                        break 'outer;
                    }
                }
            }
        }
        frames[tl].autoraise = overlap;
    }

    let raise = frames[tl].autoraise;
    AutoResult {
        ret: 1,
        raise,
        set_level_arg: if raise { Some(level_count) } else { None },
    }
}

/// The order of the update pass (`CSimpleTop::OnLayerUpdate`, `0x765650`, `__thiscall(this = top,
/// elapsed)`, `ret 4`; `strata#the-update-pass`): strata 0 to 8, within each its levels from 0, and
/// within each level the frame list in insertion order (the list head at `levelNode+0x14`, linked
/// through `frame+0x310`), calling each frame's update slot `0x38`.
///
/// `strata_levels[s][lvl]` is the frame ids at stratum `s`, level `lvl`, in insertion order.
/// Returns the sequence in which the frames are visited.
pub fn layer_update_order(strata_levels: &[Vec<Vec<usize>>]) -> Vec<usize> {
    let mut order = Vec::new();
    for strata in strata_levels {
        for level in strata {
            order.extend_from_slice(level);
        }
    }
    order
}

/// What `CompactStrata` leaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LevelCompact {
    /// The record's final level count (`+0x8`).
    pub new_level_count: u32,
    /// The final level (`+0xc4`) of each frame in the input chain, in the chain's order.
    pub final_levels: Vec<u32>,
    /// The `new_level` argument of each `SetFrameLevel` call, in call order.
    pub renumbers: Vec<u32>,
}

/// `CSimpleTop::CompactStrata` (`0x764eb0`, `__thiscall(this = top, strata)`, `ret 4`;
/// `strata#level-compaction`), as the binary runs it: the renumbering callee, the real
/// `SetFrameLevel`, changes the chain's `+0xc4` words while the walk is in progress, and the walk
/// reads the changed occupancy. A level `i` is occupied when `array_occupied[i]` or when a chain
/// frame of `target_strata` is at level `i` now. At an occupied level after a gap, every chain
/// frame at the levels `[i, count)` moves down by `delta = i - gap_start` (each through
/// `SetFrameLevel(frame, level - delta, 0)`, which lowers `+0xc4` by `delta`), the count drops by
/// `delta`, the cursor goes back to the value saved before the gap (`[ebp-8]`) and the walk goes on
/// from there, so a second separate gap is found among the already moved levels. A trailing gap cuts
/// the count to the gap's index.
///
/// This is the instruction flow of the `edi`, `edx`, `ebx` and `[ebp-8]` registers: cursor, gap,
/// count and saved cursor.
pub fn compact_strata(
    level_count: u32,
    array_occupied: &[bool],
    chain: &[(u32, u32)],
    target_strata: u32,
) -> LevelCompact {
    let mut lvl: Vec<u32> = chain.iter().map(|&(_, l)| l).collect();
    let strata: Vec<u32> = chain.iter().map(|&(s, _)| s).collect();
    let mut renumbers = Vec::new();

    let occupied = |i: u32, lvl: &[u32]| -> bool {
        array_occupied.get(i as usize).copied().unwrap_or(false)
            || (0..lvl.len()).any(|k| strata[k] == target_strata && lvl[k] == i)
    };

    let mut count = level_count;
    if count == 0 {
        return LevelCompact {
            new_level_count: 0,
            final_levels: lvl,
            renumbers,
        };
    }

    let mut i: u32 = 0;
    let mut gap: i64 = -1; // edx, -1 when no gap is open
    let mut saved_i: u32 = 0; // [ebp-8]

    loop {
        if occupied(i, &lvl) {
            if gap == -1 {
                // Occupied with no gap open: advance (0x764f48).
                i += 1;
                saved_i = i;
                if i >= count {
                    break;
                }
            } else {
                // Occupied after a gap: move [i, count) down by the gap's length (0x764f6c to
                // 0x764fd8).
                let delta = i - gap as u32;
                let mut e = i;
                while e < count {
                    for k in 0..lvl.len() {
                        if strata[k] == target_strata && lvl[k] == e {
                            let nl = e - delta;
                            renumbers.push(nl);
                            lvl[k] = nl;
                        }
                    }
                    e += 1;
                }
                i = saved_i;
                count -= delta;
                gap = -1;
                if i >= count {
                    break;
                }
            }
        } else {
            // Empty (0x764f41): open a gap at the first, advance.
            if gap == -1 {
                gap = i as i64;
            }
            i += 1;
            saved_i = i;
            if i >= count {
                break;
            }
        }
    }

    let new_level_count = if gap == -1 { count } else { gap as u32 };
    LevelCompact {
        new_level_count,
        final_levels: lvl,
        renumbers,
    }
}
