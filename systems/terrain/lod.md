# terrain: lod

The terrain's level of detail: the chunk meshes of four levels, stitched to finer neighbours and
cut by holes, the shared mesh tables, and the WDL's low-detail mesh drawn for the tiles beyond the
loaded areas.

The code is the chunk draw object file, `0x6be960` through `0x6c07d0`, the chunk index file
`0x6c4ca0` through `0x6c51b0`, the chunk file's mesh tables, and the low-detail file `0x6bd010`
through `0x6be930`. A chunk's level is +0xb8 (`cull`), 3 for the full 145-vertex mesh; levels 0
to 3 walk the outer grid in steps of 8, 4, 2 and 1 vertices.

Roots: none.

## Chunk levels

A chunk takes the near level 3 (`0x867964`), or the far level 2 (`0x867968`) when the `lod` option
(world flag 4) is on and its view depth is above `lodDist` (`0x867960`); levels 0 and 1 are never
stored. The depth is that of the chunk's bounding sphere along the view axis, f·(centre − eye) − radius, taken
from the plane `0xc7bcb0` to `0xc7bcbc` and compared strictly (equal and NaN stay near). `lod` (default
1) sets or clears the flag; `lodDist` (default string 100.0) accepts 50.0 to 250.0 inclusive and a NaN, else it says the range
and uses 77.0; `maxLOD` stores 2 or 3, 2 for 0 and 1, and 3 for a value above 3 or negative.

Addresses: `0x6afad0`, `0x6afb06`, `0x6afb11`, `0x6afb1d`, `0x6afb23`, `0x6afb2a`, `0x688920`, `0x68893c`, `0x688953`, `0x688ab0`, `0x688ac8`, `0x688af4`, `0x672680`, `0x6726a6`, `0x673200`, `0x673219`, `0x67321e`, `0x673236`, `0x867960`, `0x867964`, `0x867968`, `0x80fed4`, `0x80fed0`
Confidence: verified
Proof: one
Test: none

`0x6afb1d` is `fcomp` with `test ah,0x41; jne`, so a depth at or below `lodDist` and a NaN keep the near
level. `0x688ab0` reads a float (`%f`, `0x835160`), compares it with 50.0 (`0x80fed4`, `test ah,5; jnp`
rejects below) and 250.0 (`0x80fed0`, `test ah,0x41; jne` accepts at or below, a NaN passing both) and
stores through `0x672680`, which checks the same range again. `0x688920` echoes `Terrain LOD
enabled.` or `Terrain LOD disabled.` and sets or clears bit 2 of `0xc7b2a4`. `0x673200` reads `%d`
into an unsigned compare with 3 (`0x673219`) and with 2 (`0x673236`). The levels are 3 and 2 at start-up
(`cull`).

## Level meshes

A level's static mesh is a vertex list and an index list. Level 2 has 41 vertices and 192 indices (64
triangles: 4 × 4 cells of four triangles around the centre) and level 3 has 145 vertices and 768
indices (256 triangles: 8 × 8 quads of four). A vertex list is pairs of 16-bit numbers: the chunk's
vertex and the slot it fills in the buffer. A buffer vertex is 24 bytes, the position `+0x83c + 12 i` and
the normal `+0x170 + 12 i`. The pool buffers hold 145 vertices and 768 indices.

Addresses: `0x86c290`, `0x86c2b0`, `0x6bed20`, `0x6bedf0`, `0x6bed40`, `0x6bed60`, `0x6bedc3`, `0x86b820`, `0x86b8c8`, `0x86ba48`, `0x86bc90`, `0x86b558`, `0x86b560`, `0x6ae8a0`
Confidence: verified
Proof: one
Test: none

`0x86c290` and `0x86c2b0` hold, at eight bytes a level, the count and the pointer: levels 0 and 1
are empty, level 2 is 0x29 vertices at `0x86b820` and 0xc0 indices at `0x86b8c8`, level 3 is 0x91 at
`0x86ba48` and 0x300 at `0x86bc90`. `0x6bed20` walks the pairs with the level from the chunk's `+0xb8`
(`0x6bed3d`), copying the position and the normal to the slot (`0x6bed86` to `0x6beda9`).

## Chunk mesh selection

`0x6beb50(draw)` keys a chunk's mesh. The key is the level shifted by 16. With world flag 4 each of
the four neighbours (`+0xc4` to `+0xd0`: y − 1, x + 1, y + 1 and x − 1 in the tile's chunk grid) of a strictly
higher level adds that level to the key shifted by 0, 4, 8 and 12, and to the caps word shifted by 0, 8, 16
and 24; a lower or equal neighbour adds nothing. A changed key invalidates both buffers and is stored at
`+0xbc`.

Addresses: `0x6beb50`, `0x86c2e4`, `0x86c2f4`, `0x86c2f8`, `0x6beb6d`, `0x6beb8f`, `0x6bebf2`, `0x6c29c0`
Confidence: verified
Proof: one
Test: none

`0x6beb6d` shifts the level by `0x86c2f4` (0x10). The loop over `+0xc4` to `+0xd0` compares each
neighbour's `+0xb8` (`0x6beb8f`, `jbe` skipping a level not above) and ors it shifted by `0x86c2e4[i]` (0, 4, 8,
0xc) into the key and by `0x86c2f8[i]` (0, 8, 0x10, 0x18) into the caps. `0x6c29c0` sets the four
neighbours from the chunk's grid position `+0x7c`, `+0x80`, across the tile edge through the global tile
table, with the reciprocal links. `0x6bebf2` compares the cached key.

## Chunk mesh rebuild

A chunk whose buffers are not filled gets its mesh when it is drawn. With no neighbour bit in the key and a
zero hole word it copies the static lists of its level. Any other chunk builds its own indices with
`0x6c51b0` over all 145 vertices, a triangle list. With the draw argument set the pass then draws the
level's static batch or the chunk's built one.

Addresses: `0x6beb50`, `0x6c51b0`, `0x6bef30`, `0x6bee20`, `0x6bee90`, `0x86c2e0`, `0x6bec14`, `0x6bec32`, `0x6bec3a`, `0x6bec64`, `0x6bec86`, `0x6becdc`, `0xcac3ac`, `0xcaf160`, `0xca8308`
Confidence: verified
Proof: one
Test: none

`0x6bec14` runs the two `0x58a010` tests. The static path needs the key to have no bit in `0xfff0ffff`
(`0x86c2e0`, `0x6bec32`) and the hole word, the WORD at header `+0x3c` of the chunk's MCNK record
(`+0xf08`), to be zero (`0x6bec3a`); then `0x6bed20` and `0x6bedf0` copy the level's lists. Otherwise
the cursor `0xcac3ac` is set to `0xcaf160` (`0x6bec64`), `0x6c51b0` is called with the level, the caps,
the holes and 4, 4 (`0x6bec86`), `0x6bee20` copies all 145 vertices (the stitch needs the outer
vertices a level 2 list lacks) and `0x6bee90` uploads the list, the count the bytes written over 2.
The static batch is `0xca8308` + 32 × level (`0x6becdc`), the built batch is at `+0x154`.

## Chunk mesh tables

At start-up `0x6aef50` builds the 145-entry texture coordinate tables in the vertices' order: at
`0xcac860` an outer vertex (j, i) gets (i, j) and an inner one (i + 0.5, j + 0.5); at `0xcac3c8`
the same over 8, in [0, 1]. It then fills, for each of the four levels, remapped copies of both
tables and the level's batch descriptor from the static index lists at `0x86b540`.

Addresses: `0x6aef50`, `0xcac860`, `0xcac3c8`, `0x86b540`, `0xca8308`, `0x802978`, `0x802998`, `0x6aef89`, `0x6af020`, `0x6af0af`
Confidence: verified
Proof: one
Test: `terrain::chunk_mesh_tables`

The coordinates accumulate exact steps of 0.125 (`0x802978`) from 0 and, for inner rows and
columns, from 0.0625 (`0x802998`); `0x6aef89` multiplies by 8 before each store into
`0xcac860`, and `0x6af020` stores the unscaled value into `0xcac3c8`. Each level's list holds
pairs of words, source and destination vertex; `0x6af0af` copies the source's two coordinates
to the destination of the level's block, 145 entries apart. The lists are then sorted by their
second word.

## Chunk index building

`0x6c51b0(depth, level, caps, holes, column, row)` builds a chunk's triangle list by a walk down a
quadtree of the 8 × 8 quads, from depth 0 at the centre (4, 4) to the leaves at the chunk's level, and
writes 16-bit indices through the cursor `0xcac3ac`. A leaf is a cell of four triangles around its
centre, twelve indices; a hole bit drops the cell. Level 3 leaves are single quads, level 2 leaves cells
of 2 × 2 quads.

Addresses: `0x6c51b0`, `0x6c51bd`, `0x6c5214`, `0x6c5246`, `0x6c5274`, `0x6bee20`, `0x6bee90`, `0xcac3ac`, `0xcaf160`, `0x8111cc`, `0x8111fc`, `0x8103f8`
Confidence: verified
Proof: one
Test: `terrain::quad_hole_strip_indices`

The caps are four bytes, byte k the level of neighbour k when finer, else 0. A child keeps the
parent's byte only for the two sides that are outer sides of the child; the two inner sides get the
own level (`0x6c51bd` to `0x6c5207`). The children's centres are offset by the tables `0x8111cc`
(column) and `0x8111fc` (row): ±2, ±1, then (−1, −1), (0, −1), (0, 0), (−1, 0) at the last step. With a
nonzero hole word the nodes of depth 1 visit their four depth-2 children and skip one whose bit
`1 << ((column − 1)/2 + 4 × (row − 1)/2)` of the table `0x8103f8` is set (`0x6c5214` to
`0x6c5263`): a hole is one cell of 2 × 2 quads. The recursion stops when the depth reaches the level
(`0x6c5274`).

## Chunk index leaves

A leaf numbers the vertices flat in rows of 17. At level 3 the quad's corner is `row × 17 + column`, the
centre is the inner vertex 9 above it and the other corners are one and 17 on; at level 2 the cell centre is an
outer vertex and the corners are the centre ± 17 ± 1. It emits four fans (centre, from, to) in the
order column − 1 edge (caps byte 3), row + 1 (byte 2), column + 1 (byte 1) and row − 1 (byte 0); an edge whose
caps byte is above the level is split at its middle into (centre, from, middle) and (centre, middle, to).

Addresses: `0x6c51b0`, `0x6c52c0`, `0x6c52e5`, `0x6c5304`, `0x6c5317`, `0x6c53af`, `0x6c53bd`, `0x6c5453`, `0x6c5458`, `0x6c54e7`, `0x6c54f8`, `0x81122c`, `0x811230`
Confidence: verified
Proof: one
Test: `terrain::quad_hole_strip_indices`

The spans from `0x81122c` and `0x811230` are the row step 17 and column step 1 at level 3 and 34 and 2 at
level 2; the code halves them for the corners (`0x6c52ed`, `0x6c52f9`). Each fan tests its caps byte
against the level (`0x6c5304`, `0x6c53af`, `0x6c5453`, `0x6c54e7`) and writes the split form or the plain
one. Only level 2 can split: a level 3 chunk has no finer neighbour, so a level 2 chunk beside a level 3
chunk gains one triangle per boundary cell on that side, and a level 3 chunk never stitches.

## Low-detail mesh

A `CMapAreaLow` fills a 545-vertex buffer of 16-byte vertices (x, y, z, 0xffffffff): 17 rows of
17 outer vertices, then 16 rows of 16 inner ones. An outer vertex's x is the tile's X less 33.333332
for each row and its y the tile's Y less 33.333332 for each column, each by repeated subtraction
at 53 bits; inner ones start 16.666666 in. The height is the WDL value. Each of the 16×16 quads is
four triangles around its inner vertex C: (C, v+1, v), (C, v+18, v+1), (C, v+17, v+18) and
(C, v, v+17), v the quad's first outer vertex.

Addresses: `0x6bd8c0`, `0x6bd9b0`, `0x810e20`, `0x6bd8fb`, `0x6bd901`, `0x6bd918`, `0x6bd936`, `0x6bd9c1`, `0x6bd9e4`
Confidence: verified
Proof: one
Test: `terrain::wdl_strip_indices`, `terrain::wdl_mesh_fill`

`0x6bd8c0` loads X (+0x28) and Y (+0x2c) and keeps both on the stack; `0x6bd8fb` stores y and
`0x6bd901` subtracts 33.333332 from it, `0x6bd918` from x after each row, so the values carry
53-bit accumulations and round only at the stores. The heights are copied from +0x3c and, for
the inner grid starting 16.666666 (`0x810e20`) in, from +0x4c0. In `0x6bd9b0`, C starts at 0x121
(`0x6bd9c1`) and advances one a quad; v + 1 is `lea edi,[ecx-0x11]` with ecx = v + 18
(`0x6bd9e4`), and v skips the last column at each row's end.

## Low-detail tile walk

The scene lists the WDL tiles around the camera for each portal window or the outdoor rectangle.
`0x683040` takes the tile rectangle of the loaded window (`0xc63274` to `0xc63280` shifted right 4), grows it by
3 tiles each way and clamps to 0 to 63. It builds a frustum of its own and narrows it with the walk's
rectangle. Each tile pointer in the 64 by 64 table `0xc92078` that is not null, not already linked, and not
outside the frustum or the horizon becomes a draw candidate.

Addresses: `0x683040`, `0x683056`, `0x68307b`, `0x683095`, `0x6830a1`, `0x6830c4`, `0x6830d9`, `0x683254`, `0x683259`, `0x683267`, `0x6832ae`, `0x6832b8`, `0x6832c8`, `0x6832d3`, `0x683325`, `0xc92078`, `0xc7cacc`, `0x682f40`, `0x686180`, `0x682380`, `0x682930`, `0x682f90`, `0x5c3cc0`, `0x8101b0`, `0xc7b49c`
Confidence: verified
Proof: one
Test: `terrain::scenewalk_arealow_far_project_difftest`, `terrain::scenewalk_camera_negate_difftest`

The perspective matrix of `0x6830c4` to `0x68317f` has the active camera's field of view and aspect (its +0x44),
a near plane of the camera's far clip (+0x3c) less 33.0 (`0x8101b0`), and `0xc7b49c` as far. The view is
the identity translated by the negated camera position (`0x6831fc`). `0x683254` takes the eight corners,
`0x683259` pushes a frustum and `0x683267` narrows it with the rectangle the walk passed in `ecx`;
`0x683325` pops it. A tile whose list node is already linked (`0x6832ae`) is skipped; the frustum test
takes the tile pointer itself as the box (`0x6832b8`, 0 is inside) and the horizon test flags 8 (`0x6832c8`,
so a corner nearer than 50 does not make it visible). A passing tile is appended to the list at `0xc7cacc`.

`terrain::scenewalk_arealow_far_project_difftest` runs `0x6830c4` to `0x6830d9`;
`terrain::scenewalk_camera_negate_difftest` runs `0x6831fc` to `0x683224`.

## Low-detail pass

`0x6841a0` draws the far band: the linked tiles, with a projection and depth range of their own. It takes the
perspective matrix of the camera's field of view and aspect with near the far clip less 33.0 and far `0xc7b49c`,
sets the depth range to 0.955 to 0.96 (`0x80febc` and `0x80fec4`), translates matrix 8 by the negated camera, and
for each tile unlinks it and, with world flag 0x4000000, draws it with `0x6bd780`. It then restores the
viewport and the projection.

Addresses: `0x6841a0`, `0x684240`, `0x684253`, `0x6842f2`, `0x68434d`, `0x684373`, `0x684456`, `0x6844bc`, `0x6844ca`, `0x6844d8`, `0x6844f5`, `0x6844fd`, `0x6bd780`, `0x80febc`, `0x80fec4`, `0xc7cacc`, `0xc7b2a4`, `0x58af60`
Confidence: verified
Proof: one
Test: none

`0x684253` takes the far clip less 33.0 as near (`fsub 0x8101b0`) and `0x684240` the far plane. The viewport
call at `0x68436e` passes the saved rectangle and the two depths, 0.96 less 0.005 first (`0x68434d`
to `0x684367`). `0x6844bc` tests `0xc7b2a4` against 0x4000000 before `0x6bd780`. The list `0xc7cacc` is drained from its head.

## Low-detail tile draw

`0x6bd780` draws a tile with culling and lighting off, the fog start 0, the fog end 1.0 and the fog colour
from the light state's `+0x70`, depth test mode 3, and a 545-vertex buffer it fills once through
`0x6bd8c0`. It draws 3072 indices (16 by 16 quads of four triangles) as a triangle list with the highest
vertex 0x220. With culling off the tile shows from below (`render#face-culling`).

Addresses: `0x6bd780`, `0x6bd791`, `0x6bd79d`, `0x6bd7a4`, `0x6bd7ae`, `0x6bd7ba`, `0x6bd7ca`, `0x6bd7e5`, `0x6bd835`, `0x6bd882`, `0x6bd8a9`, `0x6bd8af`, `0x6bd8b4`, `0x8103f4`, `0x6bd8c0`, `0x6bd9b0`, `0x6d48b0`
Confidence: verified
Proof: one
Test: none

State 0x14 and 0x0e are set to 0 (`0x6bd79d`, `0x6bd7a4`), states 0xa and 0xb to 0 and 1.0
(`0x6bd7ae`, `0x6bd7ba`), state 0xd to the light state's `+0x70` (`0x6bd7ca`) and state 0x10 to 3
(`0x6bd7e5`). A tile fills its buffer when it has none (`0x6bd835`: 0x221 vertices of 16 bytes) and
shares the index buffer `0xc9a318`, filled on first use by `0x6bd9b0`. The draw packet holds primitive 3,
index count `0x8103f4` (3072) and highest index 0x220 (`0x6bd882` to `0x6bd8a9`). The draw sits between
the saved state `0x589f40` and `0x589f50` and returns once.
