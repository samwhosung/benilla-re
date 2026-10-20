# terrain: map update

What the client does each frame to keep the map resident: the order of the update, which tiles it
reads and when it waits for them, which it unloads, and which map objects and their groups it
brings in. The windows it works from are in `world#area-of-interest`.

Roots: `0x698390`, `0x6985d0`, `0x6984f0`, `0x698720`, `0x6989e0`

## Map update order

`0x698390` runs once a frame from the world frame with 1, and from the map loaders with 0. With the
teleport flag set it first clears the pending work (`0x6a6710`). It then runs, in this order: the
map object frame start `0x6b3890`, the tile load pass `0x6985d0`, the progress hook with 0.5, the
pending doodads `0x698c50`, the area unload `0x6984f0`, the map object update `0x698720`, the node
reset `0x6989e0` and the progress hook with 1.0. Only with a nonzero argument does it go on to
refresh the sky gradient texture when 2.0 seconds have passed since the last refresh, step the
liquid animation and age the queued ripples.

Addresses: `0x698390`, `0xc6325c`, `0x698396`, `0x6983a6`, `0x6983bd`, `0x6983c2`, `0x6983c7`, `0x6983d6`, `0x6983dd`, `0x6983e4`, `0x6983eb`, `0x6983f2`, `0x698406`, `0x69840d`, `0x69842b`, `0x698463`, `0x698468`, `0x698495`, `0x6a6710`, `0x6b3890`, `0x6985d0`, `0x698c50`, `0x6984f0`, `0x698720`, `0x6989e0`, `0x68ac60`, `0x68a1a0`, `0x58ac70`
Confidence: verified
Proof: one
Test: `terrain::fade_time_gate_difftest`, `terrain::fade_count_index_difftest`

`0x698396` tests the teleport flag `0xc6325c`. The progress hook is the function pointer at `0xc962e8`
called with a fixed context word and 0x3f000000 (`0x6983d6`) or 0x3f800000 (`0x698406`) when set. `0x69840d`
tests the argument; `0x69842b` compares the clock less the last refresh with 2.0 (`test ah,0x41; jne` skips a
smaller or equal value); `0x698463` is the texture update `0x58ac70`; `0x698468` the liquid step
and `0x698495` the age of a queued ripple, whose survivors move to the free list.

## Forced reload

`0x66fc80(position)` reloads the map around a position and waits for it. It recomputes the two windows
(`0x672730`), copies the outer one to `0xc7b2e4` to `0xc7b2f0` and runs `0x694920`: the pending
work, the map object cache and the footprint rings are cleared, then, with the synchronous read flag
`0x86a2d0` on, one full update pass `0x698390(0)` loads what the windows hold, `0x443d30` waits for
every pending read, the flag goes off and the light snap flag `0xc62954` is set. Four callers
reach it: `0x48eed3`, `0x48f118`, `0x5f54d4` and `0x630bf3`, the last when a forced move of more than 30 yards
belongs to the unit the camera follows.

Addresses: `0x66fc80`, `0x66fc85`, `0x66fc96`, `0x66fcb1`, `0x694920`, `0x694925`, `0x694927`, `0x69492c`, `0x694931`, `0x694936`, `0x69493d`, `0x694947`, `0x69494c`, `0x694951`, `0x69495b`, `0x443d30`, `0x630bd0`, `0x630bf3`, `0x48eed3`, `0x48f118`, `0x5f54d4`, `0x66fe6f`, `0x66fe87`, `0x66ff60`, `0x6a6710`, `0x6a3160`, `0x699d50`, `0x449000`, `0x449010`, `0x86a2d0`, `0xc62954`, `0xc7b2e4`, `0xc7b2f0`
Confidence: verified
Proof: one
Test: none

The reads and the parse run inline while `0x86a2d0` is set (`files`). `0x694925` to `0x694936` call the
clears in order. The snap flag is or'ed with the teleport flag `0xc6325c` and passed to the light
update `0x66ff60` (`0x66fe6f` to `0x66fe7d`), so a reload does not fade the lighting across the jump, and
`0x66fe87` clears it each frame. No loading screen is raised on this path: the progress hook
`0xc962e8` is null here, and the frame takes as long as the reads. A unit moved less than 30 yards
cannot leave the inner window, which is resident (`world#area-of-interest`).

## Tile load pass

`0x6985d0` walks the outer window in chunk indices and keeps the map resident. A chunk in a tile the
WDT lists (state bit 0) is handled by where it lies. Inside the inner window the tile's ADT is
requested if it was not, the frame then waits for that read to finish, and the chunk is built with
no limit. In the outer ring the tile is only requested, and the chunk is built only while less than 5
ms have passed since the pass began.

Addresses: `0x6985d0`, `0x6985d6`, `0x69861d`, `0x698630`, `0x69863c`, `0x698644`, `0x698653`, `0x69865e`, `0x698666`, `0x698672`, `0x698679`, `0x698685`, `0x698689`, `0x6986a3`, `0x6986af`, `0x6986b9`, `0x6986c8`, `0x6986cb`, `0x6986e6`, `0x6c2720`, `0x6c2880`, `0x443bd0`, `0x42c010`, `0xc7b2a8`
Confidence: verified
Proof: one
Test: none

The tile index is `(a >> 4) × 64 + (b >> 4)` (`0x69862d`); its state word is at
`0xc89f50 + 8 × index` (`0x698630`), bit 0 the WDT's flag and bit 1 a read already issued. The
inner test compares b with `0xc7b2ac` and `0xc7b2b4` and a with `0xc7b2a8` and `0xc7b2b0`
(`0x698644` to `0x698666`). `0x698679` issues the read; `0x698689` calls the wait `0x443bd0` on the
in-flight handle at `0xc89f54 + 8 × index`, which sleeps 1 ms in a loop until the read completes.
`0x6986c8` and `0x6986cb` take the elapsed time against 5 (`jae` skips). A tile whose chunks
are all built is not touched again.

## Area unload

`0x6984f0` unloads areas that lie outside the outer window and gives the rest their chunk update.
For each loaded area it compares the tile indices with the window's bounds shifted right 4; an area
outside it clears its slot in the area table, its state bit 1 and its read handle, unlinks and
destroys the area. An area inside goes to `0x6c1f70` with the pass argument. Only the window decides: no age, no
memory test.

Addresses: `0x6984f0`, `0x6984f6`, `0x698513`, `0x698556`, `0x69855c`, `0x698560`, `0x698564`, `0x69856a`, `0x69856e`, `0x69857b`, `0x698588`, `0x698596`, `0x69859d`, `0x6985a4`, `0x6985b7`, `0x6a0510`, `0x6a69f0`, `0x6c1f70`, `0xc63274`
Confidence: verified
Proof: one
Test: none

The bounds are the outer window `0xc63274` to `0xc63280` shifted by 4 (`0x698519` to `0x698522`); the
area's `+0x88` is tested against the second pair and `+0x8c` against the first (`0x69855c`,
`0x69856a`). `0x69857b` clears the area table entry, `0x698588` the state bit 1 (`and ..., 0xfffffffd`),
`0x698596` the handle; `0x69859d` unlinks and `0x6a69f0` destroys. `0x6985b7` runs the chunk update.

## Chunk window

Inside a kept area, `0x6c1f70` keeps the chunks whose grid indices lie in the outer window and destroys the
others. For each chunk of the area's list, `+0x8c` must lie in `0xc63278` to `0xc63280` and `+0x90` in
`0xc63274` to `0xc6327c`, both ends included. A chunk outside clears its slot in the area's chunk table
(`+0x278 + index × 4`) and is destroyed; with a nonzero argument a chunk inside is updated (`0x6afad0`, in `cull#chunk-level-and-idle-release`
and `cull#chunk-points`). Only the window decides.

Addresses: `0x6c1f70`, `0x6c1fa6`, `0x6c1fb8`, `0x6c1fc6`, `0x6c1fce`, `0x6c1fd4`, `0x6c1fdf`, `0x6c1fee`, `0x6c1ff9`, `0x6c2000`, `0x6a0510`, `0x6a6a00`, `0xc63274`
Confidence: verified
Proof: one
Test: none

`0x6c1fb4` and `0x6c1fb8` compare `+0x8c` with `0xc63278` and `0xc63280` (`jl`, `jg`), `0x6c1fc6` and `0x6c1fce`
compare `+0x90` with `0xc63274` and `0xc6327c`. `0x6c1fd6` tests the argument before `0x6c1fdf`. The outside leg
clears the area table slot the chunk's `+0xb4` names (`0x6c1fee`) and calls `0x6a0510` and `0x6a6a00`.

## Map object update

`0x698720` walks the map object list. An object whose box meets the far box, of half-size the far
clip around the position, waits for its root's loaded byte and, unless bit 7 of its byte `+0xc` is
set, goes to `BindDoodadSet`. Then, met or not, each of the object's groups that meets the far box
sets its timer to 30.0 and, when neither loaded nor reading, goes to `CMapObj::LoadGroup`; one that also meets the box of half-size 150 is then waited on while its read
handle is set. A loaded group goes to `CreateGroupLights` and `CreateGroupDoodads`. With a nonzero
argument, an object with bit 7 set whose box meets the frame's frustum box goes to `0x6817c0`.

Addresses: `0x698720`, `0x698727`, `0x698751`, `0x69875d`, `0x698795`, `0x69879e`, `0x6987aa`, `0x6987b9`, `0x6987c4`, `0x6987ff`, `0x698804`, `0x69885d`, `0x698865`, `0x698882`, `0x69888b`, `0x6988ea`, `0x6988f1`, `0x698900`, `0x69890f`, `0x698924`, `0x69892b`, `0x698930`, `0x69895c`, `0x6989bd`, `0x699330`, `0x6a4a50`, `0x6a4a00`, `0x6a4a30`, `0x6a4a90`, `0x6a4ab6`, `0x6a4ac3`, `0x443bd0`, `0x695c00`, `0x695aa0`, `0x6a8730`, `0x698a30`, `0x6817c0`, `0x672730`, `0xc63240`, `0xc62520`
Confidence: verified
Proof: one
Test: none

The list starts at `0xca7da4`, ending at null or an odd link. Every box test counts touching faces
and fails on NaN. `CWorld::PrepareAreaOfInterest` builds both boxes (`world#area-of-interest`).
The far box is `0xc63240` (min x, y, z, then max) against the object's box `+0x44` (min) and `+0x50` (max): three `fcomp` with `test
ah,0x41; jp` need `far.min <= box.max`, and `0x699330` needs `far.max >= box.min`. `0x6987a8` to
`0x6987b7` loops `0x6a4a50` until the root's `+0x1d4` is set; `0x6a4a50` calls `0x443bd0` while the
root's read handle `+0x1d0` is set. Each group node gives its index `+0x7c` to `0x6a4a00(root,
index, 1)` (`0x6987ff`), which returns null for a root without `+0x1d4`, else the group
`[root+0x1f4+4·index]`, and its box `+0x44` to the same tests. `0x698865` stores 0x41f00000 at
`+0x15c`; with `+0x164` and `+0x160` clear, `0x698882` calls `0x6a4a30`, which calls `0x6c3e20(index, 0)`. The 150 box `0xc62520` is
tested at `0x69888b` to `0x6988e8`, and `0x6988f1` calls `0x6a4a90`, which loops `0x443bd0` on the
group's `+0x160` (`0x6a4ab6` to `0x6a4ac3`). With `+0x164` set, flag 0x10 of the node's `+0xc`
clear calls `0x695c00` (`0x69890f`) and 0x8 clear calls `0x695aa0` and `0x6a8730` (`0x698924`,
`0x69892b`). `0x6989bd` calls `0x6817c0`.

## Scene node reset

`0x6989e0` runs for a nonzero argument. For every node in the scene list it sets the cull state
`+0x94` to 1, and a node whose flags `+0x90` have bit 1 or bit 9 goes into the depth buckets at once
(`0x681a40`); the others are bucketed by the chunk step.

Addresses: `0x6989e0`, `0x6989e5`, `0x6989ff`, `0x698a07`, `0x698a05`, `0x698a13`, `0x698a1a`, `0x681a40`
Confidence: verified
Proof: one
Test: none

`0x6989e5` loads the scene list head `0xca7d70`; `0x698a07` stores 1 in `+0x94`, `0x698a05` tests `al,2`
(bit 1) and `0x698a13` `ah,2` (bit 9); the call is `0x681a40`. The argument 0 skips the whole walk (`0x6989e0` tests `ecx`).
