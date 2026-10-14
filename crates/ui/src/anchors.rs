//! Anchor and rectangle resolution of `CLayoutFrame` (`systems/ui/anchors.md`): the two leaf
//! combiners every edge resolver uses, the relative-point coordinates, the rectangle calculation
//! with its screen clamp, the resolve that stores a changed rectangle, the frame's screen-rectangle
//! callback, and the composition of the six edge resolvers over them.
//!
//! The x87 runs at 53-bit precision, so a chain is `f64` and each `fst`/`fstp dword` is an
//! `as f32`. A leaf that returns in `st0` hands its caller the unrounded value, so it returns
//! `f64`. The `+Inf` sentinel of an unset coordinate (the cell `0xcf550c`, set from `0x7f800000`
//! at run time) and the `0.0` of an unset size compare by IEEE equality, as the binary's `fcomp`
//! against the cells does: a NaN is unordered and so not equal to either.

/// An unset coordinate: `+Infinity`.
const UNSET: f64 = f64::INFINITY;

/// The column of each point 0 to 8 (the byte table at `0x7a3060`): 0 left, 1 centre, 2 right.
const X_COL: [u8; 9] = [0, 1, 2, 0, 1, 2, 0, 1, 2];
/// The row of each point 0 to 8 (the byte table at `0x7a3140`): 0 top, 1 centre, 2 bottom.
const Y_ROW: [u8; 9] = [0, 0, 0, 1, 1, 1, 2, 2, 2];

/// 2^-22, the `f32` at `0x8029d4`: how far the size of a frame must change to be reported.
const SIZE_EPS: f64 = 2.384_185_791_015_625e-7;
/// 10^-5, the `f32` at `0x80c5c8`: how far an edge must move to store a new rectangle.
const RESOLVE_EPS: f64 = f32::from_bits(0x3727_c5ac) as f64;

/// `CombineEdge` (`0x767440`, `anchors#edge-resolution-from-size`): resolves one edge from the
/// opposite edge `opp`, the centre `center` and the signed span `span` (size times scale), trying
/// in this order:
///
/// - the opposite edge set and a nonzero span: `opp + span`;
/// - the centre set: with a nonzero span `span * 0.5 + center`, else with the opposite edge set
///   `(center - opp) + center`;
/// - otherwise `+Inf`.
pub fn combine_edge(center: f32, opp: f32, span: f32) -> f64 {
    let (c, o, s) = (f64::from(center), f64::from(opp), f64::from(span));
    if o != UNSET && s != 0.0 {
        return o + s;
    }
    if c != UNSET {
        if s != 0.0 {
            return s * 0.5 + c;
        }
        if o != UNSET {
            return (c - o) + c;
        }
    }
    UNSET
}

/// `CombineCenter` (`0x7672d0`): resolves an axis centre from its low edge `lo`, high edge `hi`
/// and the signed span:
///
/// - the low edge set: with the high edge set `(lo + hi) * 0.5`, else with a nonzero span
///   `span * 0.5 + lo`;
/// - otherwise, the high edge set and a nonzero span: `hi - span * 0.5`;
/// - otherwise `+Inf`.
pub fn combine_center(lo: f32, hi: f32, span: f32) -> f64 {
    let (l, h, s) = (f64::from(lo), f64::from(hi), f64::from(span));
    if l != UNSET {
        if h != UNSET {
            return (l + h) * 0.5;
        }
        if s != 0.0 {
            return s * 0.5 + l;
        }
    }
    if h != UNSET && s != 0.0 {
        return h - s * 0.5;
    }
    UNSET
}

/// The span the no-negate edge resolvers pass to the combiners (`CLayoutFrame::GetRight`
/// `0x767540`, `GetTop` `0x7674d0`, and the centres `0x767260` and `0x767360`): `size * scale`
/// from `fmul dword [esi+0x58]` on the width or height the getter left in `st0`, stored as `f32`
/// by the `fstp dword [esp]` that pushes the argument.
pub fn span(size: f32, scale: f32) -> f32 {
    (f64::from(size) * f64::from(scale)) as f32
}

/// The span the negate edge resolvers pass (`GetLeft` `0x7673d0` and `GetBottom` `0x7675b0`):
/// `-(size * scale)`. The `fchs` between the `fmul` and the store flips the sign of the
/// register value and never rounds, so the negation is applied before the one store to `f32`.
pub fn span_neg(size: f32, scale: f32) -> f32 {
    (-(f64::from(size) * f64::from(scale))) as f32
}

/// `RelativePointGetX` (`0x7a2f90`, `anchors#relative-point-coordinates`): the x of a relative
/// point for an anchoring frame of scale `scale`, against the relative frame's rectangle.
///
/// `rect` is the relative frame's rectangle `[bottom, left, top, right]` when reading it succeeded
/// and `None` when it did not, which gives `+Inf`. When `mirror` (the relative frame's slot `+0x24`
/// answers nonzero) the rectangle is first moved so that its left is 0 and its right is
/// `right - left`, stored as `f32`. Then `scale * x_off` plus the left edge for the points 0, 3
/// and 6, plus `(right + left) * 0.5` for 1, 4 and 7, and plus the right edge for 2, 5 and 8. A
/// point above 8 gives `+Inf`. The result stays in `st0`.
pub fn relative_point_get_x(
    scale: f32,
    x_off: f32,
    rel_point: u32,
    rect: Option<[f32; 4]>,
    mirror: bool,
) -> f64 {
    let Some([_bottom, mut left, _top, mut right]) = rect else {
        return UNSET;
    };
    if mirror {
        right = (-f64::from(left) + f64::from(right)) as f32;
        left = 0.0;
    }
    if rel_point > 8 {
        return UNSET;
    }
    let (s, xo) = (f64::from(scale), f64::from(x_off));
    let (l, r) = (f64::from(left), f64::from(right));
    match X_COL[rel_point as usize] {
        0 => s * xo + l,
        1 => (r + l) * 0.5 + s * xo,
        _ => s * xo + r,
    }
}

/// `RelativePointGetY` (`0x7a3070`): the y twin of [`relative_point_get_x`]. With `mirror` the
/// bottom becomes 0 and the top `top - bottom`; the points 0 to 2 take the top edge, 3 to 5 the
/// centre `(top + bottom) * 0.5` and 6 to 8 the bottom. `rect` is `[bottom, left, top, right]`.
pub fn relative_point_get_y(
    scale: f32,
    y_off: f32,
    rel_point: u32,
    rect: Option<[f32; 4]>,
    mirror: bool,
) -> f64 {
    let Some([mut bottom, _left, mut top, _right]) = rect else {
        return UNSET;
    };
    if mirror {
        top = (-f64::from(bottom) + f64::from(top)) as f32;
        bottom = 0.0;
    }
    if rel_point > 8 {
        return UNSET;
    }
    let (s, yo) = (f64::from(scale), f64::from(y_off));
    let (b, t) = (f64::from(bottom), f64::from(top));
    match Y_ROW[rel_point as usize] {
        0 => s * yo + t,
        1 => (t + b) * 0.5 + s * yo,
        _ => s * yo + b,
    }
}

/// What [`on_frame_size_changed`] stores and whether it reports a new size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApplyRect {
    /// The screen rectangle `[bottom (+0xd8), left (+0xdc), top (+0xe0), right (+0xe4)]`, each
    /// stored as `f32`.
    pub screen: [f32; 4],
    /// Whether the frame's `OnSizeChanged` (primary vtable `+0x48`) is called.
    pub fired: bool,
    /// The `(width, height)` passed to `OnSizeChanged` when `fired`.
    pub size: (f32, f32),
}

/// `CSimpleFrame::OnFrameSizeChanged` (`0x76b580`, `anchors#resolving-and-the-changed-callback`,
/// `input#hit-test`): the frame's rectangle callback. `cached` and `old` are the new and previous
/// layout rectangles `[bottom, left, top, right]` and `inset` the hit insets `[bottom (+0xe8),
/// left (+0xec), top (+0xf0), right (+0xf4)]`; `scale` is `+0x7c`.
///
/// Each screen edge is stored as `f32`: the low edges add `inset * scale` and the high edges
/// subtract it (`right = cached right - inset right * scale`). `OnSizeChanged` is called with the
/// new `width = right - left` and `height = top - bottom` (each stored as `f32`) when the width or
/// the height differs from the old one by 2^-22 or more. The calls the function also makes, the
/// dependents' notification `0x77f0c0`, the ancestors' resolve `0x768cf0` and the strata mark
/// `0x764a20`, are not part of this value.
pub fn on_frame_size_changed(
    cached: [f32; 4],
    inset: [f32; 4],
    scale: f32,
    old: [f32; 4],
) -> ApplyRect {
    let s = f64::from(scale);
    let [cb, cl, ct, cr] = cached;
    let screen_bottom = (f64::from(inset[0]) * s + f64::from(cb)) as f32;
    let screen_left = (f64::from(inset[1]) * s + f64::from(cl)) as f32;
    let screen_top = (f64::from(ct) - f64::from(inset[2]) * s) as f32;
    let screen_right = (f64::from(cr) - f64::from(inset[3]) * s) as f32;
    let new_w = f64::from(cr) - f64::from(cl);
    let old_w = f64::from(old[3]) - f64::from(old[1]);
    let new_h = f64::from(ct) - f64::from(cb);
    let old_h = f64::from(old[2]) - f64::from(old[0]);
    let fired = (old_w - new_w).abs() >= SIZE_EPS || (old_h - new_h).abs() >= SIZE_EPS;
    let width = (f64::from(cr) - f64::from(cl)) as f32;
    let height = (f64::from(ct) - f64::from(cb)) as f32;
    ApplyRect {
        screen: [screen_bottom, screen_left, screen_top, screen_right],
        fired,
        size: (width, height),
    }
}

/// What [`assemble_rect`] returns: whether every edge resolved, and the rectangle
/// `[bottom, left, top, right]`, clamped when asked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssembleRect {
    pub ok: bool,
    pub rect: [f32; 4],
}

/// The end of `CLayoutFrame::CalculateRect` (`0x767a20`, `anchors#rectangle-calculation-and-screen-clamp`),
/// after the four edge resolvers have run (left `0x7673d0`, bottom `0x7675b0`, right `0x767540`,
/// top `0x7674d0`): `edges` is `[bottom, left, top, right]`, each `+Inf` when its resolver found
/// nothing.
///
/// Any `+Inf` edge fails (`ok` false, nothing stored). Otherwise, with `clamp` (flag `0x10` at
/// `+0x3c`), the rectangle is moved and not resized, each store rounded to `f32`: a left below 0
/// moves `right` to `right - left` and `left` to 0; a bottom below 0 does the same to the top; a
/// right above `extent_x` (`0x41ae60(1.0)`) moves `left` to `left - (right - extent_x)` and
/// `right` to `extent_x`; a top above `extent_y` (`0x41ae70(1.0)`) does the same to the bottom.
pub fn assemble_rect(edges: [f32; 4], clamp: bool, extent_x: f32, extent_y: f32) -> AssembleRect {
    let [mut bottom, mut left, mut top, mut right] = edges;
    let inf = f32::INFINITY;
    if bottom == inf || left == inf || top == inf || right == inf {
        return AssembleRect {
            ok: false,
            rect: edges,
        };
    }
    if clamp {
        if (left as f64) < 0.0 {
            right = (f64::from(right) - f64::from(left)) as f32;
            left = 0.0;
        }
        if (bottom as f64) < 0.0 {
            top = (f64::from(top) - f64::from(bottom)) as f32;
            bottom = 0.0;
        }
        if (right as f64) > (extent_x as f64) {
            let over = f64::from(right) - f64::from(extent_x);
            left = (f64::from(left) - over) as f32;
            right = extent_x;
        }
        if (top as f64) > (extent_y as f64) {
            let over = f64::from(top) - f64::from(extent_y);
            bottom = (f64::from(bottom) - over) as f32;
            top = extent_y;
        }
    }
    AssembleRect {
        ok: true,
        rect: [bottom, left, top, right],
    }
}

/// What [`on_frame_resize`] returns and stores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recompute {
    /// The return value: 1 when the rectangle was calculated, else 0.
    pub eax: u32,
    /// The layout flags (`+0x3c`) afterwards.
    pub flags: u32,
    /// The stored rectangle `[bottom, left, top, right]` (`+0x40`) afterwards.
    pub cached: [f32; 4],
    /// Whether slot 0, the frame's rectangle callback, is called.
    pub applied: bool,
}

/// `CLayoutFrame::OnFrameResize` (`0x768d20`, `anchors#resolving-and-the-changed-callback`): calculates the
/// rectangle and stores it when it changed. `flags_in` and `cached_in` are `+0x3c` and the stored
/// rectangle; `assemble_ok` and `new_rect` the outcome of [`assemble_rect`].
///
/// On failure: bit 0 (valid) is cleared, bit 3 (failed) set, the rectangle is left alone, and the
/// result is 0. On success bits 2 and 3 are cleared; if bit 0 was clear (the first calculation) or
/// any edge moved by 10^-5 or more, the new rectangle is stored, bit 0 set and the callback called;
/// otherwise the function stops with the rectangle unchanged. The result is 1 either way.
pub fn on_frame_resize(
    flags_in: u32,
    cached_in: [f32; 4],
    assemble_ok: bool,
    new_rect: [f32; 4],
) -> Recompute {
    if !assemble_ok {
        return Recompute {
            eax: 0,
            flags: (flags_in & !1) | 8,
            cached: cached_in,
            applied: false,
        };
    }
    let flags = flags_in & !0xC;
    let changed = flags & 1 == 0 || {
        let d = |a: f32, b: f32| (f64::from(a) - f64::from(b)).abs() >= RESOLVE_EPS;
        d(new_rect[1], cached_in[1]) // left
            || d(new_rect[3], cached_in[3]) // right
            || d(new_rect[0], cached_in[0]) // bottom
            || d(new_rect[2], cached_in[2]) // top
    };
    if !changed {
        return Recompute {
            eax: 1,
            flags,
            cached: cached_in,
            applied: false,
        };
    }
    Recompute {
        eax: 1,
        flags: flags | 1,
        cached: new_rect,
        applied: true,
    }
}

// ---- the six edge resolvers composed ----
//
// `CalculateRect` runs the six resolvers (left, bottom, right, top and the two centres) over the
// anchor scans and the leaves above. The composition adds no arithmetic of its own: the control
// flow is the recursion guard (the bitfield at `+0x28`), the choice of the opposite edge and centre
// to ask, and the narrowing to `f32` at each store: a sub-resolver's result is stored to a local
// `f32` before it goes to a combiner, and each final edge is stored to the `f32` rectangle.

/// One anchor on a frame (a `CAnchor`: x and y offset, the point on the relative frame) whose
/// relative frame already has a valid rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    /// The x offset, scaled by the anchoring frame's scale (`+4`).
    pub x_off: f32,
    /// The y offset (`+8`).
    pub y_off: f32,
    /// The point 0 to 8 on the relative frame (`+0x10`).
    pub rel_point: u32,
    /// The relative frame's rectangle `[bottom, left, top, right]`.
    pub rel_rect: [f32; 4],
    /// The relative frame's slot `+0x24` answer. The ordinary frame's slot answers 0.
    pub mirror: bool,
}

/// What `CalculateRect` reads of a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameLayout {
    /// The anchors by point 0 (top left) to 8 (bottom right).
    pub anchors: [Option<Anchor>; 9],
    /// The width (`+0x50`); 0 means derive it from the anchors.
    pub width: f32,
    /// The height (`+0x54`).
    pub height: f32,
    /// The scale (`+0x58`).
    pub scale: f32,
    /// Flag `0x10`: clamp the rectangle to the screen.
    pub clamp: bool,
    /// The screen width, `0x41ae60(1.0)`; read only with `clamp`.
    pub extent_x: f32,
    /// The screen height, `0x41ae70(1.0)`; read only with `clamp`.
    pub extent_y: f32,
}

/// What [`calculate_rect`] returns: the rectangle `[bottom, left, top, right]`, or that some edge
/// stayed `+Inf`, where the binary returns 0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResolveOutcome {
    Resolved([f32; 4]),
    Unresolvable,
}

// The recursion guard's bits (`+0x28`).
const G_LEFT: u32 = 0x01;
const G_TOP: u32 = 0x02;
const G_RIGHT: u32 = 0x04;
const G_BOTTOM: u32 = 0x08;
const G_XC: u32 = 0x10;
const G_YC: u32 = 0x20;

// The points each resolver asks, the lists at `0x81c3b8` to `0x81c3f4`.
const IDS_LEFT: [usize; 3] = [0, 3, 6];
const IDS_RIGHT: [usize; 3] = [2, 5, 8];
const IDS_TOP: [usize; 3] = [0, 1, 2];
const IDS_BOTTOM: [usize; 3] = [6, 7, 8];
const IDS_XC: [usize; 3] = [1, 4, 7];
const IDS_YC: [usize; 3] = [3, 4, 5];

/// `CLayoutFrame::GetPointX` (`0x7671a0`): the first present anchor of `ids` whose x is not
/// `+Inf`, else `+Inf`.
fn get_point_x(ids: &[usize; 3], f: &FrameLayout) -> f64 {
    for &id in ids {
        if let Some(a) = f.anchors[id] {
            let v = relative_point_get_x(f.scale, a.x_off, a.rel_point, Some(a.rel_rect), a.mirror);
            if v != f64::INFINITY {
                return v;
            }
        }
    }
    f64::INFINITY
}

/// `CLayoutFrame::GetPointY` (`0x7671f0`): the y twin.
fn get_point_y(ids: &[usize; 3], f: &FrameLayout) -> f64 {
    for &id in ids {
        if let Some(a) = f.anchors[id] {
            let v = relative_point_get_y(f.scale, a.y_off, a.rel_point, Some(a.rel_rect), a.mirror);
            if v != f64::INFINITY {
                return v;
            }
        }
    }
    f64::INFINITY
}

/// `CLayoutFrame::GetLeft` (`0x7673d0`): the anchors on points 0, 3 and 6, else
/// `CombineEdge(centre, right, -width * scale)`.
fn resolve_left(f: &FrameLayout, guard: &mut u32) -> f64 {
    if *guard & G_LEFT != 0 {
        return f64::INFINITY;
    }
    *guard |= G_LEFT;
    let v = get_point_x(&IDS_LEFT, f);
    let r = if v != f64::INFINITY {
        v
    } else {
        let s = span_neg(f.width, f.scale);
        let opp = resolve_right(f, guard) as f32;
        let center = resolve_xcenter(f, guard) as f32;
        combine_edge(center, opp, s)
    };
    *guard &= !G_LEFT;
    r
}

/// `CLayoutFrame::GetRight` (`0x767540`): points 2, 5 and 8, else `CombineEdge(centre, left,
/// width * scale)`.
fn resolve_right(f: &FrameLayout, guard: &mut u32) -> f64 {
    if *guard & G_RIGHT != 0 {
        return f64::INFINITY;
    }
    *guard |= G_RIGHT;
    let v = get_point_x(&IDS_RIGHT, f);
    let r = if v != f64::INFINITY {
        v
    } else {
        let s = span(f.width, f.scale);
        let opp = resolve_left(f, guard) as f32;
        let center = resolve_xcenter(f, guard) as f32;
        combine_edge(center, opp, s)
    };
    *guard &= !G_RIGHT;
    r
}

/// `CLayoutFrame::GetTop` (`0x7674d0`): points 0, 1 and 2, else `CombineEdge(centre, bottom,
/// height * scale)`.
fn resolve_top(f: &FrameLayout, guard: &mut u32) -> f64 {
    if *guard & G_TOP != 0 {
        return f64::INFINITY;
    }
    *guard |= G_TOP;
    let v = get_point_y(&IDS_TOP, f);
    let r = if v != f64::INFINITY {
        v
    } else {
        let s = span(f.height, f.scale);
        let opp = resolve_bottom(f, guard) as f32;
        let center = resolve_ycenter(f, guard) as f32;
        combine_edge(center, opp, s)
    };
    *guard &= !G_TOP;
    r
}

/// `CLayoutFrame::GetBottom` (`0x7675b0`): points 6, 7 and 8, else `CombineEdge(centre, top,
/// -height * scale)`.
fn resolve_bottom(f: &FrameLayout, guard: &mut u32) -> f64 {
    if *guard & G_BOTTOM != 0 {
        return f64::INFINITY;
    }
    *guard |= G_BOTTOM;
    let v = get_point_y(&IDS_BOTTOM, f);
    let r = if v != f64::INFINITY {
        v
    } else {
        let s = span_neg(f.height, f.scale);
        let opp = resolve_top(f, guard) as f32;
        let center = resolve_ycenter(f, guard) as f32;
        combine_edge(center, opp, s)
    };
    *guard &= !G_BOTTOM;
    r
}

/// `CLayoutFrame::GetCenterX` (`0x767260`): points 1, 4 and 7, else `CombineCenter(left, right,
/// width * scale)`.
fn resolve_xcenter(f: &FrameLayout, guard: &mut u32) -> f64 {
    if *guard & G_XC != 0 {
        return f64::INFINITY;
    }
    *guard |= G_XC;
    let v = get_point_x(&IDS_XC, f);
    let r = if v != f64::INFINITY {
        v
    } else {
        let s = span(f.width, f.scale);
        let lo = resolve_left(f, guard) as f32;
        let hi = resolve_right(f, guard) as f32;
        combine_center(lo, hi, s)
    };
    *guard &= !G_XC;
    r
}

/// `CLayoutFrame::GetCenterY` (`0x767360`): points 3, 4 and 5, else `CombineCenter(bottom, top,
/// height * scale)`.
fn resolve_ycenter(f: &FrameLayout, guard: &mut u32) -> f64 {
    if *guard & G_YC != 0 {
        return f64::INFINITY;
    }
    *guard |= G_YC;
    let v = get_point_y(&IDS_YC, f);
    let r = if v != f64::INFINITY {
        v
    } else {
        let s = span(f.height, f.scale);
        let lo = resolve_bottom(f, guard) as f32;
        let hi = resolve_top(f, guard) as f32;
        combine_center(lo, hi, s)
    };
    *guard &= !G_YC;
    r
}

/// `CLayoutFrame::CalculateRect` (`0x767a20`): resolves the four edges in the order left, bottom,
/// right, top, each stored as `f32` into the rectangle, and returns
/// [`ResolveOutcome::Unresolvable`] if any stayed `+Inf` (tested on the register value, before the
/// store), else the rectangle `[bottom, left, top, right]`, clamped by [`assemble_rect`] when
/// `f.clamp`. Each edge's guard bit is cleared on the way out, so the order does not change the
/// values.
pub fn calculate_rect(f: &FrameLayout) -> ResolveOutcome {
    let mut guard = 0u32;
    let left = resolve_left(f, &mut guard);
    let bottom = resolve_bottom(f, &mut guard);
    let right = resolve_right(f, &mut guard);
    let top = resolve_top(f, &mut guard);
    if left == f64::INFINITY
        || bottom == f64::INFINITY
        || right == f64::INFINITY
        || top == f64::INFINITY
    {
        return ResolveOutcome::Unresolvable;
    }
    let edges = [bottom as f32, left as f32, top as f32, right as f32];
    let a = assemble_rect(edges, f.clamp, f.extent_x, f.extent_y);
    ResolveOutcome::Resolved(a.rect)
}
