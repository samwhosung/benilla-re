# ui: Frame resize bounds

The four numbers a frame keeps as its smallest and largest size, how `SetMinResize`, `SetMaxResize` and the
`<ResizeBounds>` element set them, and the one place they are enforced: the drag that `StartSizing` begins.
The drag itself (which button, which frame) is in `input`; the layout core they belong to is in `anchors`.

A bound of exactly 0 is no bound. Nothing else reads the four numbers, so a bound never limits a size set
by `SetWidth`, `SetHeight`, `SetPoint` or the layout.

Roots: `0x776020`, `0x7762a0`, `0x775f20`, `0x7761a0`

## The four bounds

The layout part of a frame holds, after its scale, the minimum height, minimum width, maximum height and
maximum width, in that order, as floats in the interface's own length unit. All four start at 0. The
Lua methods convert from script units with the same transform as `SetWidth`: `internal = G1 · value /
(G3 · 1024)`, with the screen globals `G1` and `G3` of `screencoord`, and `Get…` runs the inverse. Both axes
use the same scale.

Addresses: `0x767680`, `0x7676b5`, `0x7676b8`, `0x7676bb`, `0x7676be`, `0x776020`, `0x7760fd`, `0x776102`, `0x776109`, `0x77610f`, `0x776138`, `0x776149`, `0x77614e`, `0x776151`, `0x776154`, `0x7763d1`, `0x7763d4`, `0x775f20`, `0x41ad70`, `0x41ae60`, `0x7ffd68`
Confidence: verified
Proof: one
Test: none

The layout constructor zeroes the four words (`0x7676b5` to `0x7676be`, at layout `+0x5c` to `+0x68`). `0x776020` converts
argument 2 (`0x7760fd` to `0x77610f`, `0x41ae60`) and argument 3 (`0x776126` to `0x776138`); it stores the height of the
pair at the first word of the pair (`0x776149`) and the width at the second (`0x77614e`), and rewrites the other
pair with its old values (`0x776151`, `0x776154`). `0x7762a0` stores the maximum pair the same way (`0x7763d1`,
`0x7763d4`). The multiplier `0x7ffd68` is 1024.

## The Lua methods

`SetMinResize(width, height)` and `SetMaxResize(width, height)` take two numbers and return nothing;
`GetMinResize()` and `GetMaxResize()` return the width and the height, 0 and 0 when none was set. A
number given as a numeric string counts. A missing or non-numeric argument raises "Usage:
`<name>:SetMinResize(minWidth, minHeight)`" (maximum accordingly), with `<unnamed>` for a frame without a
name. The setters store the numbers as they are: they do not check that the minimum is below the maximum,
and do not change the frame's size, so a frame already outside a new bound stays so until it is next
sized by a drag.

Addresses: `0x776020`, `0x7762a0`, `0x775f20`, `0x7761a0`, `0x7760d1`, `0x7760e5`, `0x776351`, `0x776365`, `0x776160`, `0x776012`, `0x776292`, `0x6f34d0`, `0x8797b8`, `0x8797e4`, `0x84c7f0`, `0x879000`, `0x879008`, `0x879010`, `0x879018`
Confidence: verified
Proof: one
Test: none

The four entries are the pairs at `0x879000` to `0x879018` of the frame method table. Each setter tests
arguments 2 and 3 with `lua_isnumber` (`0x6f34d0`; `0x7760d1`, `0x7760e5`, `0x776351`, `0x776365`), which accepts a
string that converts; failure jumps to the cold block (`0x776160`) that names the frame with slot 1 of its
table or `<unnamed>` (`0x84c7f0`) and raises the usage string (`0x8797b8`, `0x8797e4`). The getters push two
values and return 2 (`0x776012`, `0x776292`). After the stores each setter does nothing else (`0x776157`
to `0x77615f`).

## The XML element

`<ResizeBounds>` inside a frame holds `<minResize>` and `<maxResize>`, each with a size given as
`x` and `y` attributes or as a child `<AbsDimension>` (or `<RelDimension>`). They go through the same
transform. The element sets all four bounds at once, so one with only `<minResize>` clears the maximum. It is
handled where it stands among the frame's other child elements, after `<Size>` and `<Anchors>`, which the base
class reads first.

Addresses: `0x769820`, `0x769baa`, `0x769bb2`, `0x769bb8`, `0x769bc1`, `0x769bc8`, `0x769bd4`, `0x769bea`, `0x769bef`, `0x769c0c`, `0x769c11`, `0x769c1a`, `0x769c1f`, `0x769c25`, `0x769c28`, `0x769c2b`, `0x6f2c50`, `0x6f1eb0`, `0x87859c`, `0x878590`, `0x8785a8`
Confidence: verified
Proof: one
Test: none

The child loop compares each tag with `ResizeBounds` (`0x769bb2`, `0x769bb8`). The four locals are zeroed
(`0x769bc8` to `0x769bd1`); `0x6f2c50` fetches `minResize` (`0x87859c`, `0x769bd4`) and `maxResize` (`0x878590`,
`0x769bef`), and `0x6f1eb0` reads the pair into the locals (`0x769bea`, `0x769c0c`). All four words are written
(`0x769c1a` to `0x769c28`), height first in each pair, even when a child element is missing.

## The drag

Dragging a frame by an edge or corner (`StartSizing`) runs the one code that applies the bounds: each frame,
when the cursor has moved, the screen's top frame passes the change to the frame's layout, which computes the
new width and height per grip, clamps each to the bounds and resizes the frame. The grip is a point name:
corner points change both sizes, edge points one, and the centre point is the move drag, which is never
bounded.

Addresses: `0x7655b0`, `0x7655d9`, `0x7655e9`, `0x76560b`, `0x76a660`, `0x76a679`, `0x768710`, `0x76898a`, `0x768430`, `0x7660d0`, `0x76616c`, `0x766193`, `0x776830`, `0x7768bb`, `0x7652b0`, `0x768c44`
Confidence: verified
Proof: one
Test: none

`0x7655b0` returns without a nonzero cursor difference (`0x7655d9`, `0x7655e9`) and with no frame in `+0xcfc`,
else calls `0x76a660` (`0x76560b`), which calls `0x768710(screen root, dx, dy, grip)` (`0x76a679`). The
grip is the anchor point id 0 to 8, handled through the table at `0x768c44`; an id above 8 skips to the
tail (`0x768974`). Every arm but the centre ends in `0x768430(…)` with the new size and the delta (`0x76898a`).
`StartSizing` refuses a frame whose resizable flag (`+0xb4` bit 9) is clear (`0x7768bb`).

## The clamp

For each axis the new size is the old size plus or minus the cursor difference over the layout scale. If the
minimum is not 0 and the size is below it, the size becomes the minimum; then, if the maximum is not 0 and the
size is above it, the size becomes the maximum, so a minimum above the maximum loses. A negative bound clamps;
a bound that is not a number does nothing. A grip on the left or top edge takes the amount cut off from the
cursor difference, so the anchor stops at the bound; a grip on the right or bottom edge zeroes the difference.

Addresses: `0x768753`, `0x768756`, `0x76875e`, `0x768761`, `0x768766`, `0x76876b`, `0x76876e`, `0x768770`, `0x76877c`, `0x768785`, `0x768793`, `0x768798`, `0x7687a0`, `0x7687a2`, `0x7687ae`, `0x7688df`, `0x768902`, `0x768908`, `0x7ffd74`
Confidence: verified
Proof: one
Test: none

The sentinel test is `fcomp` with 0.0 and `test ah,0x44` read by `jnp` (`0x768756` to `0x768761`): it skips only when
the bound equals 0. The minimum test is `fcomp` of the size against the bound and `test ah,5` read by `jp`
(`0x768766` to `0x76876e`), skipping unless the size is below; the maximum test is `test ah,0x41` and `jne`
(`0x768798` to `0x7687a0`), skipping unless the size is above. A NaN skips both. On a moving edge the cut
amount, times the scale, is taken from (`0x76877c`) or added to (`0x7687ae`) the difference; on the other edges the
size is stored plainly (`0x7688df`, `0x768902`) and the difference zeroed (`0x768908`).

## The grips

Which size changes for each of the nine anchor points:

| point | width | height |
|---|---|---|
| top left | smaller by the difference, clamped, anchor follows | taller by the difference, clamped, anchor follows |
| top | unchanged | taller by the difference, clamped, anchor follows |
| top right | wider by the difference, clamped | taller by the difference, clamped, anchor follows |
| left | smaller by the difference, clamped, anchor follows | unchanged |
| centre | none: moves the frame | none: moves the frame |
| right | wider by the difference, clamped | unchanged |
| bottom left | smaller by the difference, clamped, anchor follows | shorter by the difference, clamped |
| bottom | unchanged | shorter by the difference, clamped |
| bottom right | wider by the difference, clamped | shorter by the difference, clamped |

Addresses: `0x768c44`, `0x76873f`, `0x768831`, `0x7688ae`, `0x768b7f`, `0x768bfb`, `0x768914`, `0x768aac`, `0x768a4d`, `0x768997`, `0x768974`, `0x768c0a`, `0x768c15`, `0x768c25`, `0x768c31`, `0x768c37`, `0x7680e0`
Confidence: verified
Proof: one
Test: none

The table `0x768c44` holds the nine arm addresses in point order (`0x76873f`, `0x768831`, `0x7688ae`, `0x768b7f`,
`0x768bfb`, `0x768914`, `0x768aac`, `0x768a4d`, `0x768997`). The centre arm (`0x768bfb`) takes the frame's own
first anchor and, if it is valid (`0x768c0a`), adds the difference over the scale to the anchor's two offsets
(`0x768c15`, `0x768c25`), invalidates the layout (`0x768c31`) and returns 1 (`0x768c37`) without calling
`0x768430` or clamping.

## The unreached clamp

A second function with the same clamp exists (`0x768550`, scaling a size about a point). Nothing reaches it:
its only caller, `0x76a630`, is in no call and no table. It differs from the drag in the operand order
of its compares, so a bound that is not a number would overwrite the size there.

Addresses: `0x768550`, `0x76a630`, `0x76a649`
Confidence: verified
Proof: one
Test: none

`0x76a649` is the one call of `0x768550`; `0x76a630` has no call, jump or table reference in the image.
