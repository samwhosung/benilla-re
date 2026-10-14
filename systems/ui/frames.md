# ui: the frame object model

The classes every frame is built from, what a frame holds, how it is named and found, and what
its constructor, destructor and XML load do.

A frame is a `CSimpleFrame` (`CSimpleFrame.cpp`). It derives from `CScriptRegion`
(`CScriptRegion.cpp`, `0x76c4f0` to `0x76c8b0`), which joins two bases: a script object at `+0x0`,
the part Lua sees, from `FrameScript.cpp`, and a `CLayoutFrame` at `+0x24` (`anchors`). So every
frame has two vtables, its own at `+0x0` and its layout part's at `+0x24`, and code that receives
the layout part reaches the frame by subtracting 0x24. A `CScriptRegion` is 0xa0 bytes; the frame
fields follow to the strata link at `+0x30c`. The title region a frame may have is a bare
`CScriptRegion` with its own pair of vtables.

Roots: `0x769090`, `0x769380`, `0x7693b0`, `0x769770`, `0x769820`, `0x76a2f0`, `0x76a0d0`, `0x76c4f0`, `0x76c530`, `0x76c560`, `0x76c650`, `0x76c700`, `0x76c760`, `0x769d40`, `0x76c410`, `0x76c420`, `0x76c430`, `0x76c440`, `0x76c460`, `0x76c480`

## Vtables and the two parts

`CScriptRegion`'s vtables are `0x81c580` and, for its layout part, `0x81c5a0`; `CSimpleFrame`'s
are `0x81c498`, 36 slots, and `0x81c46c`; the title region's are `0x81c554` and `0x81c528`. The
first eight slots of each main vtable are those of the script object: the deleting destructor, the
name, the method table, the script slot by name, the type test by id, the parent setter, the type
test by name and the type name. A layout slot that must act on the frame is a thunk that moves
`ecx` back by 0x24.

Addresses: `0x76c4f0`, `0x76c513`, `0x76c51a`, `0x81c580`, `0x81c5a0`, `0x769090`, `0x7692e5`, `0x7692eb`, `0x81c498`, `0x81c46c`, `0x81c528`, `0x81c554`, `0x76c410`, `0x76c420`, `0x76c6e0`, `0x76c700`, `0x76c430`, `0x76c440`, `0x76c460`, `0x76c480`, `0x878870`
Confidence: verified
Proof: one
Test: none

`0x76c4f0` calls the script object's constructor `0x701ae0` and `0x767680`, zeroes `+0x98` and
`+0x9c`, and stores `0x81c5a0` at `+0x24` and `0x81c580` at `+0x0` (`0x76c513`, `0x76c51a`).
`0x769090` stores `0x81c498` and `0x81c46c` (`0x7692e5`, `0x7692eb`). Slot 4 of `0x81c580` is
`0x46ff70` (`+0x98`); slot 0x14 is `0x76c430`, `mov [ecx+0x9c],arg`; slot 0x10 is `0x76c440`,
`arg == [0xcf0c3c]`; slot 0x18 is `0x76c460`, `SStrCmpI(arg, "Region")`; slot 0x1c is `0x76c480`,
the string pointer `[0x878870]`. `0x76c410` and `0x76c420` are `sub ecx,0x24` thunks; `0x76c6e0`
returns the parent's layout part and `0x76c700` the layout part of a frame found by name.

## The region type id

Each script class answers its type test by comparing the argument with a type id it takes from a
shared counter the first time one is needed: `CScriptRegion`'s is `0xcf0c3c`, numbered from
`0xceef6c`. A lookup by name accepts an object only when its type test passes for the id asked.

Addresses: `0x76c700`, `0x76c440`, `0xcf0c3c`, `0xceef6c`, `0x7a11d0`
Confidence: verified
Proof: one
Test: none

`0x76c700` tests `[0xcf0c3c]` and, when 0, increments `[0xceef6c]` and stores it there; `0x7a11d0`,
the Lua `GetObjectType` of regions, opens with the same test (`mov eax,[0xcf0c3c]; test eax,eax;
jne`). A writer census of `0xcf0c3c` finds those stores only.

## Names and lookup by name

Naming a frame replaces `$parent` at the start of the name with the name of the nearest named
ancestor, or with `Top` when none is named, stores a copy at `+0x98` and makes the name a Lua global
for the frame's table unless that global is already set. Looking a frame up by name applies the same
substitution, reads the global, and accepts it only when it is a table whose element 0 is the object
and the object passes the type test.

Addresses: `0x76c650`, `0x76c5b0`, `0x76c5dd`, `0x76c629`, `0x76c760`, `0x701bd0`, `0x701c6c`, `0x701cb8`, `0x8788ac`, `0x8788b0`
Confidence: verified
Proof: one
Test: none

`0x76c5b0` compares the prefix `$parent` (`0x8788b0`) case-insensitively for its length, copies
`Top` (`0x8788ac`, `0x76c5dd`), walks `+0x9c` until slot 4 returns a non-empty name and copies that
over it, and appends the rest with `SStrPack` (`0x76c629`); with no prefix it copies the name.
`0x76c650` frees the old `+0x98`, builds the name into 1024 bytes, stores `SStrDupA` of it and calls
`0x701bd0` with it, which reads the global and sets it only when its type is nil (`0x701c6c` to
`0x701cb8`). `0x76c760` reads the global, checks the value's type is 5 (a table), reads element 0 as
light userdata and calls slot 0x10 with the id.

## Frame fields

A frame keeps, after the `CScriptRegion` fields `+0x98` (name) and `+0x9c` (parent): the top
(`+0xa0`), the frame its `OnEnter` shows (`+0xa4`), the title region (`+0xa8`), a state (`+0xac`, 3
while being destroyed), the id (`+0xb0`), flags (`+0xb4`), scale and parent scale (`+0xb8`,
`+0xbc`), stratum, level and alpha (`+0xc0`, `+0xc4`, `+0xc8`), input kinds (`+0xcc`), shown and
visible (`+0xd0`, `+0xd4`), the hit rectangle and insets (`+0xd8` to `+0xf4`), drag state
(`+0xf8` to `+0x110`), the loading flag (`+0x114`), sixteen script slots (`+0x118` to `+0x190`),
five layer switches (`+0x198`), the backdrop (`+0x1ac`), the regions (`+0x1b4`), the regions by
layer (`+0x1bc`), five own batches (`+0x1fc`) and the children (`+0x2fc`).

Addresses: `0x769090`, `0x7690d0`, `0x76a3c0`, `0x76a0d0`, `0x76a750`, `0x76a7f0`, `0x76a860`, `0x76a910`, `0x76aa20`, `0x76c2e0`, `0x7693b0`
Confidence: inferred
Proof: none
Test: none

The offsets are those the constructor initializes and the functions of this chunk read and write;
each field's meaning is taken from the writers named in the entry that uses it. `0x76a3c0` sets or
clears bits of `+0xb4`; the bits read here are 1 toplevel, 0x10 overlapped, 0x100 movable, 0x200
resizable, 0x1000 placed by the user (`layoutcache`), 0x2000 in a tree that draws itself and 0x4000 the root of one (`draworder`). `0x76a750` and
`0x76a7f0` add and remove a region node (`.?AUREGIONNODE@@`) of `+0x1b4`; `0x76a860` and `0x76a910`
those of a layer list. `0x76c2e0` stores `+0xf8` and switches layer 4 on or off with it.

## Child and region lists

A frame keeps its child frames and its regions in two lists of the same shape, headers at `+0x2fc` and `+0x1b4`,
and links each new member at the head, so a walk from the tail, by previous links, visits them in creation
order (head and tail as in `draworder`). A list node is 12 bytes: the next link, the previous link and the object. A link's low bit marks the end of the list, so an empty list holds a tagged
pointer to its own header, never 0. Attaching a frame to the parent it already has changes nothing and does not
move it to the end.

Addresses: `0x769216`, `0x76921c`, `0x769221`, `0x76922c`, `0x7692bc`, `0x7692c7`, `0x76a750`, `0x76a7c6`, `0x76a7d4`, `0x76a7d7`, `0x76aa20`, `0x76abdb`, `0x77fd6e`, `0x76ab10`, `0x76ab20`
Confidence: verified
Proof: one
Test: none

The constructor stores the header's own address in the head slot, the terminator's next pointer, and the same address
or 1 in the tail slot, its previous pointer (`0x769221`, `0x769223`, `0x76922c`; `0x7692c2`, `0x7692c4`, `0x7692c7`).
`0x76a750` loads the head (`0x76a7c6`), points the new node's next link at it and its previous link at the head's
old previous (`0x76a7cc`, `0x76a7d1`), links the head's previous to the new node (`0x76a7d4`) and becomes the head
(`0x76a7d7`); with an empty list the old head is the terminator. This is a head insert, as `0x7668c0` mode 2 and
`0x76a860` make, so the last member added is nearest the head and a walk that starts at the tail and steps by
previous links sees the first member added first. `0x76aa20` does the same for children. Each has one call site, `0x77fd6e` and `0x76abdb`.

## Reading the lists from Lua

`GetNumChildren` and `GetNumRegions` return one number, 0 for an empty list. `GetChildren` and `GetRegions`
return one value per entry, not a table, so a frame with none returns no value. The result is the whole list in
list order: hidden children and regions are counted and returned, and nothing in the four methods tests
visibility, alpha, layer, strata, level, type or name. `GetRegions` reads the one flat list, not the lists by draw
layer, so its order is creation order and not BACKGROUND to HIGHLIGHT.

Addresses: `0x773e60`, `0x773f05`, `0x773f20`, `0x773f43`, `0x773f60`, `0x774080`, `0x774125`, `0x774180`, `0x774018`, `0x77402d`, `0x774032`, `0x77404f`, `0x774056`, `0x77405a`, `0x77406a`, `0x701bd0`, `0x6f3810`, `0x6f2f30`, `0x6f3bc0`, `0x878ec0`
Confidence: verified
Proof: one
Test: none

`0x773e60` and `0x774080` count the list in a loop with two tests, the tag bit and null (`0x773f20` to `0x773f2c`),
and push the count with `lua_pushnumber` (`0x773f43`), returning 1. `0x773f60` and `0x774180` count first,
reserve that many stack slots with `lua_checkstack` (`0x77402d`), then walk again, and for each object make its Lua
table if it has none (`0x701bd0`, `0x77405a`) and push it with `lua_rawgeti` from the registry (`0x77406a`); the return
value is the count (`0x774078`). The four are entries of the method table `0x878ec0`. The `xor eax,eax` after each
argument error call is not reached, as the error unwinds.

## What the region list holds

A region is in its parent's list from the moment it is attached and leaves it when it is detached. The title
region of a frame is never in the list: it is a bare object whose parent setter stores the parent and nothing else,
so `GetRegions` does not return it and `GetNumRegions` does not count it. A button's label and state textures are in
the list. A region detached with `SetParent(nil)` is removed. Hiding a frame or region changes no list.

Addresses: `0x77fd10`, `0x77fd2d`, `0x77fd47`, `0x77fd55`, `0x77fd69`, `0x77fd6e`, `0x76a7f0`, `0x76c430`, `0x81c554`, `0x7739df`, `0x769b79`, `0x778d20`, `0x778d49`, `0x778fd0`, `0x779017`, `0x779110`, `0x779139`, `0x76ad50`, `0x76ada2`, `0x76adc9`, `0x7733e5`, `0x769e74`
Confidence: verified
Proof: one
Test: none

`0x77fd10(parent, layer, show)` is the one gateway: it detaches the region from its old parent through slot 0x0b
(`0x77fd55`, which runs `0x76a7f0`), stores the new parent (`0x77fd5d`) and links through `0x76a750` only when the new
parent is not null (`0x77fd69`, `0x77fd6e`). The title region is built by the Lua method (`0x7739df`) and the XML
element (`0x769b79`) as an object of vtable `0x81c554`, whose slot 5 is `0x76c430` (`0x76c436` stores `+0x9c`), and
neither path reaches `0x77fd10`. The label setter `0x778d20` and the state texture setters `0x778fd0` and `0x779110` call
`0x77fd10` (`0x778d49`, `0x779017`, `0x779139`). `0x76ad50` walks both lists read-only, and a hidden region keeps its node.

## Construction

The constructor builds the `CScriptRegion`, zeroes the frame fields, sets shown to 1, the state to
1, the stratum to 3, alpha to 255 and both scales to 1.0, builds the lists and the five batches,
counts the frame in `0xcf0c1c`, takes the top from `0xcf0bd8` and joins its frame list, sets the
parent passed to it, switches the first four layers on, and makes itself visible through slot
0x88.

Addresses: `0x769090`, `0x769099`, `0x769272`, `0x7692f2`, `0x7692f8`, `0x76930d`, `0x769318`, `0x76934a`, `0xcf0c1c`, `0x7646a0`
Confidence: verified
Proof: one
Test: none

`0x769090` calls `0x76c4f0` (`0x769099`), stores 1 at `+0xac`, 3 at `+0xc0`, 0xff at the byte
`+0xc8`, 1.0 at `+0xb8` and `+0xbc`, builds five batches with 0x772e80 (`0x769272`), increments
`0xcf0c1c` (`0x7692f2`), stores `[0xcf0bd8]` at `+0xa0` (`0x7692f8`), calls `0x7646a0`
(`0x76930d`), which links the frame into the top's list at `+0xcc0` through `+0x304`, and `0x76ab10`
with the parent (`0x769318`); it stores 1 at `+0x198` to `+0x1a4`, 0 at `+0x1a8` and 1 at `+0xd0`,
then calls slot 0x88 (`0x76934a`).

## Destruction

The destructor marks the state 3, removes the frame from its level and the top's list,
releases its Lua table, deletes its title region, its regions and its children, unparents itself,
frees its backdrop, releases its sixteen script slots, uncounts itself, and tears down its lists,
batches and `CScriptRegion`.

Addresses: `0x7693b0`, `0x769380`, `0x764710`, `0x764690`, `0x701cd0`, `0x702670`, `0x76c560`, `0x76c530`, `0x769d40`
Confidence: verified
Proof: one
Test: none

`0x7693b0` stores 3 at `+0xac`, calls `0x764710` (the top's `0x764920` with 0, then the unlink from
`+0xcc0`) and `0x764690`, which is `ret 4`, zeroes `+0xa0`, calls `0x701cd0(0)`, slot 0 of `+0xa8`
with 1, deletes each region and child through slot 0, calls `0x76aab0` on the parent, frees
`+0x1ac`, calls `0x702670` on the sixteen slots from `+0x118` and decrements `0xcf0c1c`; it ends
with `0x76c560`, which releases the name `+0x98` and calls `~CLayoutFrame` and the script object's
destructor. `0x769380`, `0x76c530` and `0x769d40` are the deleting destructors.

## Script slots by name

A frame's script slots are found by handler name, ignoring case. `OnEvent` belongs to the script
object at `+0xc`; the frame adds sixteen at `+0x118 + 8·i` in the order `OnLoad`, `OnSizeChanged`,
`OnUpdate`, `OnShow`, `OnHide`, `OnEnter`, `OnLeave`, `OnMouseDown`, `OnMouseUp`, `OnMouseWheel`,
`OnDragStart`, `OnDragStop`, `OnReceiveDrag`, `OnChar`, `OnKeyDown`, `OnKeyUp`. An unknown name
has no slot.

Addresses: `0x76a0d0`, `0x702590`
Confidence: verified
Proof: one
Test: none

`0x76a0d0` (slot 0xc of `0x81c498`) asks `0x702590` first, which compares with `OnEvent` and
returns `this + 0xc`, then compares with each name in turn and returns `this + 0x118` to `this +
0x190` in steps of 8.

## Loading from XML

Loading a frame from its XML node runs three steps. The first sets the name and the `id` and
marks the frame loading. The second applies an inherited node first, then the layout, then the
attributes `hidden`, `toplevel`, `movable`, `resizable`, `frameStrata`, `frameLevel`, `alpha`,
`enableMouse`, `enableKeyboard` and `clampedToScreen`, then the elements `TitleRegion`,
`ResizeBounds`, `Backdrop`, `HitRectInsets`, `Layers` and `Scripts`. A script element enables the
input kinds it needs: `OnChar` characters; `OnKeyDown` or `OnKeyUp` keys; `OnEnter`, `OnLeave`,
`OnMouseDown`, `OnMouseUp` or `OnDragStart` the mouse; `OnMouseWheel` the wheel. The last step
loads the child `Frames` and ends the load (`visibility`).

Addresses: `0x769770`, `0x769820`, `0x769d70`, `0x769ef0`, `0x76a060`, `0x76a2f0`, `0x76af00`
Confidence: verified
Proof: one
Test: none

`0x769820` (slot 8 of the layout vtable) reads `inherits` and calls slot 8 with the inherited
node, calls `0x767800`, then each attribute by its string; a strata name goes through `0x6f17d0`
and a level must be at least 1. `hidden` true calls slot 0x84 and false slot 0x88. `0x769ef0`
compiles each script element with `0x7025c0` and calls `0x76af00(0)` for `OnChar`, `(1)` for the
key handlers, `(2)` for the five mouse handlers and `(3)` for `OnMouseWheel`. `0x769d70` adds each
`Texture` and `FontString` of a `Layer` to the layer its `level` names. `0x76a060` recurses into
`inherits` before creating the `Frames` children.
