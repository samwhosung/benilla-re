# ui: anchors and layout

How a frame's rectangle is computed from its anchor points, and when it is computed again.

Every laid-out object of the interface embeds a `CLayoutFrame` (`CLayoutFrame.cpp`, `0x7670a0` to
`0x769070`): nine anchor point slots, a size, a scale, the resolved rectangle and its flags. A
point holds an offset and, for a relative point, the frame and the point of that frame it is
pinned to. Each edge is resolved on demand from the points on that edge, else from the opposite
edge or the centre and the size. A frame whose rectangle cannot be resolved yet waits on one
queue, which the interface drains each frame and after a load. The relative point's two
coordinate getters and the "undefined" value they share, +∞ in `0xcf550c`, lie in a separate run,
`0x7a2f70` to `0x7a3070`. The Lua methods that reach this code (`SetPoint`, `GetLeft` and the
rest) are in `methods`.

Roots: `0x767800`, `0x767c70`, `0x767db0`, `0x7680e0`, `0x768360`, `0x7683d0`, `0x7683f0`, `0x768c70`, `0x768cc0`, `0x768cf0`, `0x768ed0`, `0x767680`, `0x767720`, `0x7a2f90`, `0x7a3070`

## Point slots and point names

A `CLayoutFrame` keeps its anchor points in nine slots at `+0x04` to `+0x24`, one per point, so a
frame has at most one anchor per point. The index of a point is fixed by the name table the XML
loader and the Lua methods share:

| index | point | index | point | index | point |
|---|---|---|---|---|---|
| 0 | `TOPLEFT` | 3 | `LEFT` | 6 | `BOTTOMLEFT` |
| 1 | `TOP` | 4 | `CENTER` | 7 | `BOTTOM` |
| 2 | `TOPRIGHT` | 5 | `RIGHT` | 8 | `BOTTOMRIGHT` |

Name lookup ignores case.

Addresses: `0x811a38`, `0x6f1840`, `0x6f1890`, `0x767620`
Confidence: verified
Proof: one
Test: none

`0x811a38` is nine pairs of an index and a name pointer: 7 `BOTTOM`, 6 `BOTTOMLEFT`, 8
`BOTTOMRIGHT`, 4 `CENTER`, 1 `TOP`, 2 `TOPRIGHT`, 0 `TOPLEFT`, 3 `LEFT`, 5 `RIGHT`. `0x6f1840`
compares the name with each pair's string through `SStrCmpI` (`0x6f185e`, length 0x7fffffff) and
stores the pair's index; `0x6f1890` maps an index back to its name, else `"UNKNOWN"`. Slot `i` is
`[this+i*4+4]`: `0x767620` walks `lea edi,[ecx+0x4]` nine times (`0x76762c`, `0x767668`).

## Absolute and relative points

`SetPoint` (`0x767c70`; point, relative frame, relative point, x, y, resize flag) refuses a null
relative frame or the frame itself with error 0x57, and changes nothing when the slot holds a point
to the same frame and point with both offsets within 2⁻²² of the new ones. Otherwise it frees the
old point, telling its relative frame (`Dependents`), allocates a 0x14-byte relative point {vtable
`0x81c44c`, x, y, frame, point}, clears flag 8, records itself as a dependent of the relative frame
with mask `1 << point`, and lays itself out when the flag is set. The unreached `0x767b50` stores a
12-byte absolute point {vtable `0x81c42c`, x, y}, that vtable's one store (`0x767bab`).

Addresses: `0x767c70`, `0x767c7f`, `0x767cb6`, `0x767ccb`, `0x767d29`, `0x767d49`, `0x767b50`, `0x767bab`, `0x81c42c`, `0x81c43c`, `0x81c44c`, `0x767be0`, `0x767bf0`, `0x767c00`, `0x767d70`, `0x767c10`, `0x767c40`, `0x767d80`, `0x8029d4`
Confidence: verified
Proof: one
Test: `ui::glue_767_767c70_anchor_change_detect`

The equality test is `fld [edi+0x4]; fsub [ebp+0x14]; fabs; fcomp [0x8029d4]` with `test ah,0x5;
jp` at `0x767cb6` and again for y at `0x767ccb`; `0x8029d4` holds 0x34800000, 2⁻²². The vtables:
`0x81c43c` is the base with two `__purecall` getters; `0x81c42c` returns `+0x4` and `+0x8`
(`0x767bf0`, `0x767c00`) and no frame (`0x767be0`); `0x81c44c` uses the getters below and returns
`+0xc` as its frame (`0x767d70`). Slot 0 of each is a deleting destructor that restores
`0x81c43c` (`0x767c10`, `0x767c40`, `0x767d80`).

## Relative point coordinates

For the anchoring frame's scale `s`, a relative point's x is `s·x` plus the relative frame's left
edge for points 0, 3 and 6, its centre `(right + left)·0.5` for 1, 4 and 7 and its right edge for 2,
5 and 8; y is `s·y` plus the top for 0 to 2, the centre for 3 to 5 and the bottom for 6 to 8. A
deferred relative frame first has its deferral ended for good and is laid out if pending; with no
valid rectangle the result is +∞. When its slot 0x24 answers nonzero, its rectangle is first moved
so that its left (for x) or bottom (for y) is 0.

Addresses: `0x7a2f90`, `0x7a3070`, `0x7a2fb8`, `0x7a2fc0`, `0x768060`, `0x7680bf`, `0x7a2fcc`, `0x7a2fda`, `0x7a2ffc`, `0x7a3003`, `0x7a300a`, `0x7a301a`, `0x7a3035`, `0x7a3045`, `0x7a3054`, `0x7a3060`, `0x7a3134`, `0x7a3140`, `0xcf550c`
Confidence: verified
Proof: one
Test: `ui::anchor_resolve_x`, `ui::anchor_resolve_y`

`0x7a2f90` tests flag 2 of the relative frame's `+0x3c` and calls `0x768060(0)` (`0x7a2fb8`,
`0x7a2fc0`), which clears the flag (`0x7680bf`) and calls `0x7680e0(1)` only when flag 4 is set;
it reads its rectangle with `0x768320` (`0x7a2fcc`) and returns `[0xcf550c]` on failure
(`0x7a3045`). The shift is `fld [ebp-0xc]; fchs; fld [ebp-0xc]; fadd st,st(1)` after `call
[edx+0x24]` (`0x7a2fda`). The point at `+0x10` indexes the byte table `0x7a3060` (0,1,2 repeated)
into the jump table `0x7a3054`; the cases compute `fld [ebp+8]; fmul [esi+4]; fadd left`, `(right
+ left)·0.5 + s·x` and `s·x + right`. `0x7a3070` uses `0x7a3140` (0,0,0,1,1,1,2,2,2) and `+0x8`.
`s` is the argument the edge resolvers push, the anchoring frame's `+0x58` (`0x7671bf`).

`ui::anchor_resolve_x` and `ui::anchor_resolve_y` run `0x7a2f90` and `0x7a3070` whole and compare their `st0` results.

## Edge resolution from points

Each of the six edge values (left, right, top, bottom, horizontal and vertical centre) first asks
the points that lie on it, in a fixed order, and takes the first value that is not +∞: left asks
points 0, 3, 6; right 2, 5, 8; top 0, 1, 2; bottom 6, 7, 8; the horizontal centre 1, 4, 7; the
vertical centre 3, 4, 5. A NaN from a point counts as found. Each resolver sets its own bit in
`+0x28` while it runs and returns +∞ at once if the bit is already set, which breaks cycles.

Addresses: `0x7671a0`, `0x7671f0`, `0x7673d0`, `0x767540`, `0x7674d0`, `0x7675b0`, `0x767260`, `0x767360`, `0x81c3b8`, `0x81c3c4`, `0x81c3d0`, `0x81c3dc`, `0x81c3e8`, `0x81c3f4`, `0x7671ca`, `0x7671d5`
Confidence: verified
Proof: one
Test: `ui::layout_767_span`, `ui::layout_composed_resolve`

`0x7671a0` loops over a list of point indices, calls slot 4 (x) of each present point with
`[edi+0x58]` and returns the first result for which `fcomp [0xcf550c]; test ah,0x44; jp` jumps
(not equal, or unordered); `0x7671f0` is the same with slot 8 (y). The six lists, three dwords
each, are `0x81c3d0` {0,3,6} for left (`0x7673d0`, bit 1), `0x81c3e8` {2,5,8} for right
(`0x767540`, bit 4), `0x81c3dc` {0,1,2} for top (`0x7674d0`, bit 2), `0x81c3f4` {6,7,8} for bottom
(`0x7675b0`, bit 8), `0x81c3b8` {1,4,7} for the horizontal centre (`0x767260`, bit 0x10) and
`0x81c3c4` {3,4,5} for the vertical centre (`0x767360`, bit 0x20). Each tests its bit first
(`test al,0x1` at `0x7673d6`), returns `[0xcf550c]` when set, and clears it on the way out
(`0x767430`).

`ui::layout_767_span` runs the span operations of the six edge resolvers, from the `fmul` to the `fstp`, as copies of the exe's own bytes asserted equal to the source. `ui::layout_composed_resolve` runs the six resolvers and the point scans for real, through `0x767a20`.
`0x7675b0` is the last of the six and ends at `0x767616`; ten padding bytes follow, and `0x76761f` is the last of them, before `0x767620`.

## Edge resolution from size

When no point fixes an edge it is derived from its size `d`, the frame's width (slot 0x1c, `+0x50`)
or height (slot 0x20, `+0x54`) times the scale `+0x58`, rounded to a float. A far edge (right,
top) is `near + d`, else `centre + d·0.5`, else `2·centre − near`; a near edge (left, bottom) is
the same with `−d`. The first two steps need their inputs to be finite (not +∞) and `d` nonzero;
the last is reached only when `d` is 0 and needs both the centre and the near edge finite. A centre
is `(near + far)·0.5`, else `near + d·0.5`, else `far − d·0.5`; the last two need `d` nonzero.
Otherwise the result is +∞.

Addresses: `0x767440`, `0x7672d0`, `0x767333`, `0x76748b`, `0x76749d`, `0x767409`, `0x76740c`, `0x767412`, `0x767417`, `0x767422`, `0x76742b`, `0x767299`, `0x7672b9`, `0x7ffa24`, `0x7ffd74`
Confidence: verified
Proof: one
Test: `ui::layout_combine_edge`, `ui::layout_combine_center`

Left (`0x7673d0`): `call [eax+0x1c]; fmul [esi+0x58]; fchs` pushes `−d`, then right and the centre,
then calls `0x767440(centre, right, −d)`. `0x767440(a, b, c)`: if `b ≠ +∞` and `c ≠ 0`, `b + c`
(`0x767463`); else if `a ≠ +∞`: `c·0.5 + a` when `c ≠ 0`, else `(a − b) + a` when `b ≠ +∞`
(`0x76748b`, `0x76749d`); else +∞. Right pushes `+d` and passes left; top passes bottom with `+d`;
bottom passes top with `−d`. `0x7672d0(near, far, d)`: `(near + far)·0.5`, `d·0.5 + near`, or
`far − d·0.5` by `fsubr` at `0x76734c`, after `far ≠ +∞` and `d ≠ 0` (`0x767333`). Equality with +∞
and with 0.0 (`0x7ffd74`) is `test ah,0x44; jnp`, which jumps on equal only, so NaN takes the
not-equal side. 0.5 is `0x7ffa24`. The arguments pass as floats; results return in `st0` at 53 bits.

`ui::layout_combine_edge` and `ui::layout_combine_center` run `0x767440` and `0x7672d0` whole and compare their `st0` results.

## Rectangle calculation and screen clamp

The rectangle `{bottom, left, top, right}` is resolved in the order left, bottom, right, top; the
first +∞ fails the whole calculation. With flag 0x10 (clamped to screen) the rectangle is moved,
not resized: a left below 0 shifts both x edges right, a bottom below 0 shifts both y edges up,
then a right above the screen width (`0x41ae60(1.0)`) shifts both x edges left and a top above the
screen height (`0x41ae70(1.0)`) shifts both y edges down.

Addresses: `0x767a20`, `0x767a28`, `0x767a46`, `0x767a60`, `0x767a7b`, `0x767a99`, `0x767aa8`, `0x767ac8`, `0x767ae5`, `0x767aef`, `0x767b14`, `0x767b1e`, `0x41ae60`, `0x41ae70`
Confidence: verified
Proof: one
Test: `ui::layout_assemble_rect`, `ui::layout_composed_resolve`

`0x767a20` stores each edge with `fst` into the output (`+0x4`, `+0x0`, `+0xc`, `+0x8`) and
returns 0 when `fcomp [0xcf550c]; test ah,0x44; jnp` finds +∞. The clamp tests `[edi+0x3c]` bit
0x10 (`0x767a99`). `fcomp 0.0; test ah,0x5; jp` skips when the edge is at least 0 or NaN; else
`right − left` replaces right and left becomes 0 (`0x767ab8`). For the far side, `fcom
[esi+0xc]` with the screen width in `st0` skips when the width is not below right; else left
becomes `left − (right − width)` and right becomes the width (`0x767af9` to `0x767b04`).

`ui::layout_assemble_rect` runs `0x767a20` with the four edge resolvers and the two extent functions replaced by `fld` islands. `ui::layout_composed_resolve` runs it whole over a synthesized frame and anchor graph.

## Resolving and the changed callback

Resolving a frame calculates its rectangle. On failure it clears flag 1 (valid), sets flag 8
(failed) and answers 0. On success it clears flags 4 and 8; when the rectangle was valid and every
edge moved by less than 10⁻⁵ it answers 1 and stops. Otherwise it stores the new rectangle at
`+0x40`, sets flag 1, calls slot 0 with the old rectangle and answers flag 1, which slot 0 may
clear. `CLayoutFrame`'s own slot 0 lays out every dependent.

Addresses: `0x768d20`, `0x768d49`, `0x768d55`, `0x768d63`, `0x768d6e`, `0x768dc2`, `0x768df4`, `0x768e0e`, `0x768e14`, `0x80c5c8`, `0x768cf0`, `0x81c400`
Confidence: verified
Proof: one
Test: `ui::layout_recompute`

`0x768d20` calls `0x767a20` on a zeroed local (`0x768d49`); `and ecx,0xfffffffe; or ecx,0x8`
(`0x768d55`). On success `and ecx,0xfffffff3` (`0x768d63`); the four `fabs; fcomp [0x80c5c8]`
tests (0x3727c5ac, 10⁻⁵) compare left, right, bottom, top with `+0x44`, `+0x4c`, `+0x40`, `+0x48`.
The old rectangle is copied to a local, the new stored, `or ecx,0x1` (`0x768df4`), and `call
[eax]` (`0x768e0e`) is slot 0, `0x768cf0` in `0x81c400`, which calls `0x7680e0(0)` on the frame of
each node of `+0x34`.

`ui::layout_recompute` runs `0x768d20` whole, with `0x767a20` and slot 0 replaced by stubs.

## Dependents

A frame anchored to another is recorded in the other's dependents list at `+0x30`, one 0x10-byte
node {links, frame, point mask} per dependent frame; a second point to the same frame ORs its bit
into the node. Freeing a point clears its bit, and the node is freed when the mask reaches 0. A
frame being destroyed clears its own points, then every dependent's points that name it, and
leaves the layout queue. `0x768010` answers whether a frame depends on another through any chain
of relative points.

Addresses: `0x767ee0`, `0x767fa0`, `0x767e70`, `0x768e20`, `0x768010`, `0x767620`, `0x767db0`, `0x767f1e`, `0x767f90`, `0x767f93`, `0x767fd7`, `0x767e3f`, `0x768fc0`, `0x768ff0`, `0x878540`
Confidence: verified
Proof: one
Test: none

`0x767ee0` (`ecx` the relative frame, arguments frame and mask) ORs into an existing node
(`0x767f0e`) or allocates one tagged `.?AUFRAMENODE@CLayoutFrame@@` (`0x767f1e`) and links it at
the head, storing frame and mask at `+0x8` and `+0xc`. `0x767fa0` ANDs with `~mask` and frees the
node at zero (`0x767fd7`). `SetAllPoints` (`0x767db0`) clears all points and sets point 0 to the
frame's point 0 and point 8 to its point 8 with zero offsets, mask 0x101 (`0x767e3f`). `0x767e70`
frees each point whose frame is the argument; `0x768e20` (from `~CLayoutFrame`) calls
`0x767620`, then `0x767e70` on each dependent, frees the nodes and unlinks the queue link.
`0x768010` recurses over all nine points.

## Invalidation and the layout queue

Laying a frame out (`0x7680e0`) with the force flag resolves at once unless flag 8 is set; success
removes it from the queue. Otherwise, if it is already pending (flag 4) and deferred (flag 2) or
queued, the retry count `+0x38` is reset to 6. Else it sets flag 4 and either, when deferred, lays
out each dependent, or joins the queue: just before the first queued frame with a point relative
to it, else at the tail, with the retry count 6. A frame already queued is not moved.

Addresses: `0x7680e0`, `0x7680f4`, `0x768142`, `0x76815f`, `0x76816c`, `0x768194`, `0x7681a0`, `0x7681b0`, `0x7681d1`, `0x768218`, `0x768220`, `0x76825f`, `0x768294`, `0x7682ed`, `0x7682a8`, `0x768310`, `0xcf0bec`, `0xcf0bf0`, `0xcf0bf4`
Confidence: verified
Proof: one
Test: none

`0x768310` is `mov eax,[ecx+0x3c]; and eax,0x4`. The queue is a list with its link at `+0x6c` of
each frame (`0xcf0bec`, the offset 0x6c), terminator `0xcf0bf0` and first link `0xcf0bf4`.
`0x7681b0` exits when the frame's next link is set (`0x7681d1`), walks the queue calling slot 0xc
of each queued frame's nine points (`0x768218`) and keeps the last match per frame, stopping at the
first frame that has one (`0x76822e`); it then links the frame before that frame's link
(`0x768294` to `0x7682a5`) or after the terminator's previous, the tail (`0x7682e5` to
`0x7682f9`), and stores 6 at `+0x38`.

## Deferred layout

A frame whose layout is deferred (flag 2) is taken off the queue and is not laid out by
invalidation; its dependents are. Ending the deferral lays the frame out at once when it was
pending. `CSimpleFrame` defers a frame while it is hidden (`visibility`).

Addresses: `0x768060`, `0x76806e`, `0x768074`, `0x7680bf`, `0x7680c4`, `0x7680cd`
Confidence: verified
Proof: one
Test: none

`0x768060(1)` ORs 2 into `+0x3c` (`0x76806e`) and unlinks the frame's queue link when it is linked
(`0x768074` to `0x7680b3`). `0x768060(0)` clears the bit (`0x7680bf`) and, when bit 4 is set,
calls `0x7680e0(1)` (`0x7680cd`).

## The queue drain

Draining the queue makes up to 32 passes while it is not empty. Each pass resolves every queued
frame in order: a resolved frame clears flag 4 and leaves the queue; an unresolved one loses one
retry, and when the count reaches 0 it too clears flag 4 and leaves the queue, keeping the flag 8
its failed resolve set.

Addresses: `0x768ed0`, `0x768eed`, `0x768ef7`, `0x768efc`, `0x768f12`, `0x768f17`, `0x768f55`
Confidence: verified
Proof: one
Test: none

`0x768ed0` loops while `[0xcf0bf4]` is a frame and the pass count is below 0x20 (`0x768eed`). Per
frame it reads the next link `[esi+0x70]` before resolving (`0x768efc`), so a frame queued during
the pass at the tail is reached in the same pass. `dec byte [esi+0x38]; jne` (`0x768f12`) keeps an
unresolved frame; a resolved frame, and an unresolved one whose count reaches 0, fall into
`and [esi+0x3c],0xfffffffb` and the unlink (`0x768f17` to `0x768f4c`). Callers include the top's
update (`0x7656ba`, `0x765799`) and the glue and game interface builds (`0x46ac56`, 0x49457f).

## Size, scale and stored rectangle

Setting the width (`+0x50`) or height (`+0x54`) clears flag 8 and lays the frame out. Setting the
scale refuses 0 with error 0x57, ignores a change below 2⁻²² unless forced, and otherwise stores it,
zeroes the rectangle, clears flag 1 and lays the frame out. Reading the rectangle copies `+0x40` to
`+0x4c` and answers flag 1. A point is inside the rectangle when `left ≤ x ≤ right` and
`bottom ≤ y ≤ top`; NaN is outside. Flag 0x10 is set and cleared by `0x768cc0`, which lays the frame
out.

Addresses: `0x7683d0`, `0x7683f0`, `0x768360`, `0x768366`, `0x768373`, `0x76838f`, `0x7683a5`, `0x7683c3`, `0x768410`, `0x768420`, `0x768320`, `0x768c70`, `0x768c8f`, `0x768ca9`, `0x768cc0`
Confidence: verified
Proof: one
Test: `ui::glue_767_768360_rsub_change_detect`

`0x7683d0` and `0x7683f0` store the argument, `and edx,0xfffffff7` and call `0x7680e0(0)`.
`0x768360` returns 0 after `SErrSetLastError(0x57)` when `fcomp 0.0` is equal (`0x768366`); the
force flag is the byte argument; the store is `0x7683a5`. `0x768410` and `0x768420` return `+0x54`
and `+0x50` (slots 0x20 and 0x1c of `0x81c400`). `0x768c70` tests flag 1, then `+0x44 ≤ x`, `x ≤
+0x4c`, `+0x40 ≤ y` and `y ≤ +0x48`, the upper tests `test ah,0x41; jp` (`0x768c8f`, `0x768ca9`),
which an equal value passes.

## Construction and the XML anchors

The constructor zeroes the points, rectangle, size, bounds and links, empties the dependents list
and sets the scale to 1.0. The XML loader reads `<Size>`, `setAllPoints`, and each `<Anchor>` of
`<Anchors>`: `point`, `relativePoint` (defaulting to `point`), `relativeTo` (defaulting to the
parent), and `<Offset>`. An unknown point, a missing relative frame or a frame anchored to itself
is reported and skipped. The points are set without layout; the loader lays the frame out once at
the end.

Addresses: `0x767680`, `0x7676ae`, `0x7676ca`, `0x7676f0`, `0x767720`, `0x767800`, `0x767964`, `0x767970`, `0x7679f8`, `0x7670a0`, `0x7670b0`, `0x7670e0`, `0x7670f0`, `0x878440`
Confidence: verified
Proof: one
Test: none

`0x767680` stores 1.0 at `+0x58` (`0x7676ae`) and the vtable `0x81c400` (`0x7676ca`); the destructor
`0x767720` calls `0x768e20`. `0x767800` reads the strings `setAllPoints`, `Anchors`, `point`,
`relativePoint`, `relativeTo` and `Offset`, resolves `relativeTo` through slot 0x28 (`0x767964`),
reports a name it cannot find with `Couldn't find relative frame: %s` (`0x878440`, `0x767970`) and
calls `0x767c70` with the resize flag 0; a true `setAllPoints` without anchors calls `0x767db0` with
1, and the anchors path ends with `0x7680e0(0)` (`0x7679f8`). The queue's static initializer
`0x7670a0` calls `0x7670b0` (offset 0x6c, empty list) and registers `0x7670f0` with `_atexit`, which
unlinks every queued frame at exit.