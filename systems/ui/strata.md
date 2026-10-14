# ui: strata, levels and the per-frame passes

How the top orders frames into strata and levels, raises them, and runs the update and draw
passes in that order.

`CSimpleTop` (`CSimpleTop.cpp`, `0x764160` to `0x767080`) is the root of the interface: a
`CLayoutFrame` covering the screen that owns nine strata records, the input lists (`input`) and the
list of every frame. The glue build and the game interface build construct one each (0x46a848,
0x48fe4b), and the constructor stores itself in `0xcf0bd8`, the top every frame joins. A shown
frame sits in one level node of its stratum; the nodes give the order of `OnUpdate` and of
drawing. The top's pass function is the callback its constructor hands to 0x442800.

Roots: `0x764180`, `0x764360`, `0x764330`, `0x764780`, `0x764920`, `0x764a20`, `0x764a60`, `0x7650f0`, `0x7652a0`, `0x764d50`, `0x766fc0`, `0x766fd0`

## Strata and their names

A frame's stratum is `+0xc0`, an index 0 to 8, and its level `+0xc4`, a count from 0. The
constructor of a frame sets the stratum to 3 and the level to 0. The names the XML attribute
`frameStrata` and the Lua methods accept are, ignoring case:

| index | name | index | name |
|---|---|---|---|
| 1 | `BACKGROUND` | 5 | `DIALOG` |
| 2 | `LOW` | 6 | `FULLSCREEN` |
| 3 | `MEDIUM` | 7 | `FULLSCREEN_DIALOG` |
| 4 | `HIGH` | 8 | `TOOLTIP` |

Index 0 has no name, so only a frame that keeps the top's own stratum record 0 uses it.

Addresses: `0x8119f8`, `0x6f17d0`, `0x6f1820`, `0x769090`, `0x7690d0`
Confidence: verified
Proof: one
Test: none

`0x8119f8` holds eight pairs of an index and a name pointer, read from the bytes. `0x6f17d0`
compares through `SStrCmpI` (`0x6f17ee`) over eight pairs and stores the index; `0x6f1820` maps an
index back to its name or `"UNKNOWN"`. The frame constructor `0x769090` stores 3 at `+0xc0` and 0
at `+0xc4` (`param[0x30] = 3`, `param[0x31] = 0`).

## Strata records and level nodes

The top holds nine pointers at `+0xcd4`, one 0x18-byte record per stratum: a draw-dirty flag, a
levels-dirty flag, the count of levels in use, and an array of level nodes. A level node,
`CFrameStrataNode`, is 0x11c bytes: two frame lists linked through each frame's `+0x30c`, a cursor
at `+0x18`, five draw batches of 0x30 bytes from `+0x1c`, a dirty-layer mask at `+0x10c` and the
list of batches to draw at `+0x110`. The second list holds the level's frames; the first holds
frames added while the update pass walks the level.

Addresses: `0x764180`, `0x7642b4`, `0x7642c1`, `0x764780`, `0x764801`, `0x764815`, `0x76482c`, `0x764849`, `0x764854`, `0x76485a`, `0x766850`, `0x766a90`, `0x766ae0`, `0x8783e4`
Confidence: verified
Proof: one
Test: none

The constructor allocates nine zeroed 0x18-byte records with `SMemAlloc(0x18)` into `+0xcd4`
(`0x7642b4` to `0x7642eb`). `0x764780` allocates a node with `SMemAlloc(0x11c)` (`0x764801`),
initializes two lists with link offset 0x30c (`0x764815`, `0x76482c`), zeroes `+0x18`, builds
five 0x30-byte batches with 0x772e80 through 0x404130 (`0x764849`), zeroes `+0x10c` and
makes `+0x110` an empty list with offset 0x28 (`0x766850`). The array grows through `0x766b30`,
tagged `.PAVCFrameStrataNode@@`.

## Joining and leaving a level

A frame joins its stratum when it becomes visible and leaves when it is hidden or its stratum or
level changes. Joining creates the level nodes up to its level, raises the in-use count to cover
it, and links the frame at the head of the level's frame list, or of the side list while the update
pass is on that level (`draworder`). Unless the frame draws itself (flag 0x2000 of `+0xb4`) every draw layer of the
level is marked dirty. The record's levels-dirty flag is set, draw-dirty when a layer is, and the
frame's input kinds join their lists.

Addresses: `0x764780`, `0x764882`, `0x764893`, `0x7648b3`, `0x7648c9`, `0x7648d1`, `0x7648e4`, `0x764901`, `0x76490c`, `0x764920`, `0x76493a`, `0x764942`, `0x76498f`, `0x7649a4`, `0x7649f4`, `0x764a0d`, `0x7668c0`, `0x766aa0`, `0x766880`
Confidence: verified
Proof: one
Test: none

`0x764780` compares the level with `+0x8` and stores `level + 1` (`0x764882`). With the cursor
`+0x18` set it links the frame before the first list's terminator (`0x7648b3` to `0x7648c4`),
else `0x7668c0(frame, 2, 0)` appends to the second (`0x7648d1`). `test ah,0x20` on `+0xb4` guards
the store of 0xffffffff into `+0x10c` (`0x7648e4`); `+0x4` gets 1 and `+0x0` ORs `+0x10c ≠ 0`.
`0x76afa0` registers the input kinds. `0x764920` (frame, keep-drag) first ends a move of this frame
unless kept (`0x76493a`), removes its input kinds, and, when linked, advances the cursor past it
(`0x76498f` to `0x7649a4`) and unlinks it.

## Dirty marks

A laid-out frame whose rectangle changes marks its stratum's levels dirty, raises the frame being
moved or sized without the overlap check, and asks for a new hover pick. A frame whose region
changes in one draw layer marks that layer of its level node and its stratum's draw-dirty flag.

Addresses: `0x764a20`, `0x764a36`, `0x764a4c`, `0x7650f0`, `0x7651ac`, `0x764a51`, `0x764a60`, `0x764a8a`, `0x764a90`, `0x76b648`, `0x76a990`, `0x76a9d0`
Confidence: verified
Proof: one
Test: none

`0x764a20` stores 1 in the record's `+0x4` (`0x764a36`), calls the raise `0x7650f0(+0xcfc, 0)` when
a frame is being moved or sized (`0x764a4c`), which without the check moves that frame only when its
bit 0x10 is already set (`0x7651ac`), and stores 1 in `+0x1100` (`0x764a51`); its caller is the
frame's rectangle callback (`0x76b648`). `0x764a60(frame, layer)` ORs `1 << layer` into the node's
`+0x10c` (`0x764a8a`) and stores 1 in the record's `+0x0` (`0x764a90`); `0x76a990` and `0x76a9d0`
call it for a visible frame of a top, one layer or all five.

## Raising a frame

Raising a frame acts on it or its nearest ancestor with the toplevel flag (bit 1 of `+0xb4`);
without one nothing happens. With the overlap check, the frame is first laid out if needed and
bit 0x10 is set when any frame at the same or a higher level of its stratum, other than itself and
its descendants, has a rectangle that intersects its own with positive area. When bit 0x10 is
set, the stratum's levels are compacted and the frame moves to the level just above the highest
in use, its children following. `Lower` does nothing.

Addresses: `0x7650f0`, `0x7650fb`, `0x765113`, `0x76513e`, `0x765150`, `0x765191`, `0x765221`, `0x76524d`, `0x765254`, `0x7651a3`, `0x7651ac`, `0x7651be`, `0x7651d8`, `0x7651dd`, `0x7652a0`, `0x76a5b0`, `0x76a5c0`, `0x767010`, `0x766c70`, `0x766bc0`
Confidence: verified
Proof: one
Test: `ui::overlap_autoraise`, `ui::is_descendant`

`0x7650f0` walks `+0x9c` until `test [edi+0xb4],0x1` (`0x765113`). With the check (`0x76513e`) it
calls `0x7680e0(1)` when not valid or pending (`0x765150`), scans levels from `+0xc4` to the
record's `+0x8` (`0x765191`), skips frames whose parent chain holds it (`0x767010`, `0x765221`),
and intersects the two rectangles with `0x766c70`; `0x766bc0` answers `bottom < top && left <
right`. `0x76a3c0(0x10, found)` stores the bit (`0x7651a3`). Then `test [edi+0xb4],0x10`
(`0x7651ac`) gates `0x764eb0` and `0x76a4f0(record+0x8, 1)` (`0x7651d8`) and `+0x1100 = 1`. The
Lua `Raise` path is `0x76a5b0`, `push 1; push ecx`; `Lower` is `0x76a5c0` to `0x7652a0`, `xor
eax,eax; ret 4`.

`ui::overlap_autoraise` runs `0x7650f0` whole, with `0x764eb0` and `0x76a4f0` stubbed, and `ui::is_descendant` runs `0x767010` whole.

## What raises a frame

A frame is raised by the script call `Raise`, by being shown when it is toplevel, by a mouse press
on a frame (the pressed frame, else the hovered one), by the start of a move or size, and, without the
overlap check, when the rectangle of the frame being moved or sized changes. Each acts on the
frame's nearest toplevel self or ancestor. Setting the toplevel flag raises nothing, a raise never
changes the stratum, and `Lower` does nothing.

Addresses: `0x76a5b0`, `0x76a5c0`, `0x7652a0`, `0x775a50`, `0x775a91`, `0x775ad6`, `0x775afd`, `0x775b0a`, `0x775b20`, `0x775b61`, `0x775b3d`, `0x76aeec`, `0x766392`, `0x7652d7`, `0x764a4c`, `0x775440`, `0x76a3c0`, `0x765010`, `0x7650f0`
Confidence: verified
Proof: one
Test: none

A census of `call` and `jmp` to `0x76a5b0` finds `0x7652d7` (the start of a move or size), `0x766392`
(the mouse press, before `OnMouseDown` and the capture), `0x76aeec` (a show, after the children and
before `OnShow`, when bit 1 of `+0xb4` is set) and the binder `0x775a50`, which calls it at four sites: `0x775b0a`, the live raise, and
`0x775a91`, `0x775ad6` and `0x775afd`, each after a `luaL_error` (`0x775a87`, `0x775acc`, `0x775af3`) with `xor ecx,ecx`, so never reached because
`luaL_error` does not return. That is seven callers in all. To `0x7650f0` itself,
`0x764a4c` (without the check), `0x76a5b9` and the unreached `0x7650d6`. `0x76a5b0` is `push 1; push ecx; mov
ecx,[ecx+0xa0]; call 0x7650f0`, `0x76a5c0` ends in `0x7652a0`, which is `xor eax,eax; ret 4`. `Frame:SetToplevel`
passes mask 1 to `0x76a3c0` (`0x7754d8`), which only sets or clears bits. `0x7650f0` never stores `+0xc0`. Mouse release
holds no raise.

## Unknown strata names

An XML `frameStrata` whose value is not one of the eight names is reported and skipped: the frame
keeps the stratum it had, loads on, and the status gets a severity 1 message `Frame %s: Unknown
frame strata: %s`. An empty or missing attribute is skipped silently. The script call
`SetFrameStrata` with such a name raises `%s:SetFrameStrata(): Unknown frame strata: %s`, and with
no string argument `Usage: %s:SetFrameStrata(level)`.

Addresses: `0x6f17d0`, `0x769820`, `0x769945`, `0x76994c`, `0x769953`, `0x769957`, `0x769961`, `0x769971`, `0x769978`, `0x7699a4`, `0x7699ad`, `0x774360`, `0x7743f0`, `0x77440b`, `0x774412`, `0x77441a`, `0x77444f`, `0x774456`, `0x76a470`, `0x6f4940`
Confidence: verified
Proof: one
Test: none

`0x6f17d0` is called only from `0x769961` and `0x77440b` and stores the index only on a hit. In the
loader a miss at `0x769968` pushes the frame's name (or `<unnamed>`), the value and the format
and calls slot 3 of the status object `[ebp+0xc]` (`0x7699a4`) with severity 1, then falls into the
`frameLevel` read at `0x7699ad`, where the hit path also lands; `0x76a470` is called only at
`0x769971`. The binder reaches `luaL_error` (`0x6f4940`), which does not return (`luastate`).

## Level compaction

Compacting a stratum walks its levels up from 0 to the in-use count. A level is occupied when its
node lists a frame or any frame the top knows, shown or not, has that stratum and level. At the
first occupied level after a run of empty ones, every frame of that level and the levels above is
lowered by the run's length, children not moved separately, and the count drops by it. The walk
goes on at the same index, so levels that slid into the run are not examined again: O, E, E, O, E,
O compacts to O, O, E, O with the count 4. A trailing empty run is dropped from the count.

Addresses: `0x764eb0`, `0x764eed`, `0x764efb`, `0x764f18`, `0x764f20`, `0x764f41`, `0x764f71`, `0x764fb6`, `0x764fd3`, `0x764fd8`, `0x764fe3`, `0x764f62`, `0x764ff1`
Confidence: verified
Proof: one
Test: `ui::level_compact`, `ui::level_compact_ignores_visibility`, `ui::level_compact_leaves_interior_gaps`

`0x764eb0` loops over levels below the record's `+0x8`; a node's frame list is non-empty when its
first link `[node+0x14]` is a pointer (`0x764eed`). Otherwise it walks the top's frame list
`+0xcc0`/`+0xcc4` with link offset `+0xcbc` (`0x764efb`) matching `+0xc0` and `+0xc4`
(`0x764f18`, `0x764f20`). An empty level opens a run (`0x764f41`); at the next occupied level the
gap is `level − start` (`0x764f71`) and `0x76a4f0(level − gap, 0)` is called for each matching
frame of every higher level (`0x764fb6`); the count drops by the gap (`0x764fd8`), and the walk
restores the index from `ebp−0x8` (`0x764fd3`) and jumps past its `inc edi` to the bound test
(`0x764fe3`). The count ends
as the start of a trailing run (`0x764f62`) or as reduced.

The three tests run `0x764eb0` whole with the real `0x76a4f0`, whose changes to the levels the walk then reads.

## Stratum and level setters

Setting a frame's stratum or level takes a visible frame out of its level, stores the value and
puts it back. A new stratum is passed to every child. A level below 0 becomes 0; with the
propagate flag, each child in the same stratum moves by the same difference. A frame given a
parent takes the parent's stratum and the parent's level plus one.

Addresses: `0x76a470`, `0x76a481`, `0x76a498`, `0x76a4a5`, `0x76a4b6`, `0x76a4dd`, `0x76a4f0`, `0x76a4fd`, `0x76a50f`, `0x76a53b`, `0x76a582`, `0x76a595`, `0x76ab10`
Confidence: verified
Proof: one
Test: `ui::set_frame_strata`, `ui::set_frame_level`

`0x76a470` returns when equal (`0x76a481`); with `+0xd4` set it calls `0x764920(frame, 1)` and
`0x764780(frame, 1)` on the top `+0xa0` around the store at `0x76a4a5`, then calls itself on each
child of `+0x300` (`0x76a4dd`). `0x76a4f0` clamps with `jge` (`0x76a4fd`), keeps the difference
(`0x76a50f`), stores `level + difference` (`0x76a53b`) and, with its second argument set, calls
`0x76a4f0(child level + difference, 1)` for each child whose `+0xc0` equals its own (`0x76a582`).
`0x76ab10` calls `0x76a470(parent+0xc0)` and `0x76a4f0(parent+0xc4 + 1, 0)`, or 3 and 0 for no
parent.

`ui::set_frame_strata` and `ui::set_frame_level` run `0x76a470` and `0x76a4f0` whole on a synthesized frame tree, with `0x764920` and `0x764780` stubbed to log the frame.

## The update pass

The pass function `0x764330` takes the elapsed time and runs the update, then the draw. The update
first deletes the objects on the top's deletion list and drains the layout queue. Then for each
stratum from 0 to 8, each level from 0 up and each frame of the level from the oldest link to the newest, it calls the
frame's update slot 0x38 with the elapsed time, moving the cursor ahead so the frame may leave.
Frames that joined the level during the walk are then appended to its list. The layout queue is
drained again, and a pending hover pick is made from the last mouse position.

Addresses: `0x764330`, `0x764340`, `0x765650`, `0x765660`, `0x7656b6`, `0x7656ba`, `0x765707`, `0x765710`, `0x765713`, `0x76572d`, `0x765757`, `0x765799`, `0x7657a1`, `0x7657b4`, `0x7657be`
Confidence: verified
Proof: one
Test: `ui::render_traverse_order`

The constructor calls 0x442800 with `ecx` 0, `edx` 4, then 1.0, 0, `0x764330` and the address of
`+0x74` pushed (`0x764220` to `0x764264`); `0x764330` calls `0x765650(elapsed)` and `0x7657d0`.
`0x765650` deletes the nodes of `+0xcd0` through slot 0 (`0x7656b6`), calls `0x768ed0`
(`0x7656ba`), and per node stores the next frame in `+0x18` (`0x765707`) before `call
[edx+0x38]` (`0x765710`). The splice moves the first list onto the second when they differ
(`0x765727`), with error 0x57 on a malformed list (`0x76572d`). `0x7657b4` clears `+0x1100` and
calls `0x7660d0` with `+0x1104` (`0x7657be`).

`ui::render_traverse_order` runs `0x765650` whole with each frame's update slot stubbed to log the frame, and reads the order.

## The draw pass

The draw pass sets an orthographic projection over the screen, then for each stratum from 0 to 8:
when its levels are dirty, it recomputes the overlap bit 0x10 of every toplevel frame in it; when
its draw is dirty, it rebuilds the batches of each level whose layer mask is set; and it draws the
batches of each level from the lowest. A rebuilt batch gathers, for its draw layer, the regions of
every frame in the level that does not draw itself, from the oldest link to the newest (`draworder`).

Addresses: `0x7657d0`, `0x7657f1`, `0x765847`, `0x765851`, `0x765896`, `0x7658e2`, `0x765920`, `0x765a90`
Confidence: verified
Proof: one
Test: none

`0x7657d0` calls `0x7ad7f0(0)` (`0x7657f1`); for a record with `+0x4` set, each frame with bit 1
of `+0xb4` gets `0x76a3c0(0x10, 0x765a90(frame))` (`0x765847`, `0x765851`) and `+0x4` is cleared.
With `+0x0` set, `0x765920` runs per node (`0x765896`) and `+0x0` becomes whether any node is
still dirty. `0x765920` loops five layers of 0x30 bytes from `+0x1c`, unlinks each from `+0x110`,
and for a set bit calls 0x7731d0, slot 0x44 of each frame without bit 0x2000 with the batch and
layer, and 0x7731a0, relinking a batch that holds something; then clears `+0x10c`. Drawing is
0x76fb00 per batch of `+0x110` (`0x7658e2`). `0x765a90` is the overlap test of `Raising a
frame`.

## Construction and teardown

The top's constructor builds its `CLayoutFrame`, its frame list (link offset 0x304), the queue of
objects to delete (0x304), the strata records and 36 input lists, a 782-entry table of the frame
that took each key, the cursor position, and the pass and input registrations; it sets the
hover-pick flag and zeroes the four host callbacks `+0x1128` to `+0x1134`. Of this file's code only
`0x764ce0`, which nothing calls, links an object onto the deletion list. The destructor deletes
the queued objects and every frame, frees the records and nodes, removes the cursor and the input
registrations, closes the pass registration and clears `0xcf0bd8`.

Addresses: `0x764180`, `0x76418b`, `0x7641b8`, `0x7641df`, `0x764206`, `0x76422a`, `0x76424c`, `0x76425e`, `0x76426b`, `0x764282`, `0x764302`, `0x764360`, `0x764390`, `0x7643ea`, `0x764436`, `0x76446c`, `0x7644c0`, `0x7644c7`, `0x7644cf`, `0x7644d4`, `0x764592`, `0x7645a0`, `0x766980`, `0x766a10`, `0x766b10`, `0x766d10`, `0x766d50`, `0xcf0bd8`, `0x81c380`
Confidence: verified
Proof: one
Test: none

`0x764180` calls `0x767680` (`0x76418b`), stores 0x304 as the link offset of `+0xcc0` and
`+0xccc` (`0x7641b8`, `0x7641df`), zeroes 36 lists of 0x1c bytes (`0x764206`), stores 1 at
`+0x1100` (`0x76422a`), the vtable `0x81c380` (`0x76424c`) and `0xcf0bd8` (`0x76425e`), calls
`0x765bf0` (`0x76426b`) and clears 0x30e dwords from `+0x84` (`0x764282`). `0x764390` deletes
through slot 0 (`0x7643ea`, `0x764436`), frees each node with `0x7645a0`, then `SetCursor(0)`
(`0x7644c0`), `0x765d10`, `HandleClose(+0x74)` and the store of 0 (`0x7644d4`); `0x764360` is the
deleting destructor. `0x766fc0` answers no layout parent and `0x766fd0` no frame by name.
