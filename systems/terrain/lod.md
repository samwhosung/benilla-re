# terrain: lod

The terrain's level of detail: the chunk meshes of four levels, stitched to finer neighbours and
cut by holes, the shared mesh tables, and the WDL's low-detail mesh drawn for the tiles beyond the
loaded areas.

The code is the chunk draw object file, `0x6be960` through `0x6c07d0`, the chunk index file
`0x6c4ca0` through `0x6c51b0`, the chunk file's mesh tables, and the low-detail file `0x6bd010`
through `0x6be930`. A chunk's level is +0xb8 (`cull`), 3 for the full 145-vertex mesh; levels 0
to 3 walk the outer grid in steps of 8, 4, 2 and 1 vertices.

Roots: none.

## Chunk mesh selection

`0x6beb50(draw)` picks a chunk's mesh: its key is level << 16, and with world flag 4 each
neighbour of a higher level adds that level shifted by 0, 4, 8 and 12 for the four neighbours,
while the same levels shifted by 0, 8, 16 and 24 gather into a second word passed as caps. When the key
changes the chunk's buffers are refilled. A key with no neighbour bits and no holes takes the
level's static mesh; any other builds its own indices through `0x6c51b0`. With the argument set,
it then draws through `0x6bef30` with the level's or the chunk's batch.

Addresses: `0x6beb50`, `0x6c51b0`, `0x6bef30`, `0x86c2e0`, `0x86c2e4`, `0x86c2f4`, `0x86c2f8`, `0x6beb6d`, `0x6beb8f`, `0x6bebf2`, `0x6bec32`, `0x6bec86`, `0x6becdc`
Confidence: verified
Proof: one
Test: none

`0x6beb6d` shifts the level by `0x86c2f4` (0x10). The loop over +0xc4 to +0xd0 compares each
neighbour's +0xb8 (`0x6beb8f`, `jbe` skipping a level not above) and ors it shifted by
`0x86c2e4[i]` (0, 4, 8, 0xc) into the key and by `0x86c2f8[i]` (0, 8, 0x10, 0x18) into the caps. `0x6bebf2`
compares with the cached key +0xbc. `0x6bec32` tests the key with 0xfff0ffff (`0x86c2e0`) and
the holes word; `0x6bec86` passes the level, caps, holes and 4, 4. The static batch is
`0xca8308` + 32·level (`0x6becdc`), the built one +0x154.

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

`0x6c51b0` builds a chunk's triangle list for a level, its finer-neighbour caps and its holes: a
recursive walk over the quadtree of the outer grid that emits each cell as four triangles around
its centre, twelve indices, skipping holed quads and splitting the edges that face a finer
neighbour.

Addresses: `0x6c51b0`, `0x6bee20`, `0x6bee90`, `0xcac3ac`, `0xcaf160`
Confidence: inferred
Proof: none
Test: `terrain::quad_hole_strip_indices`

`0x6beb50` points `0xcac3ac` at `0xcaf160` before the call and hands the result to `0x6bee20`
and `0x6bee90`, which fill the chunk's vertex and index buffers. The recursion and the caps'
bit layout are not derived here.

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

## Low-detail pass

The scene draws the WDL tiles around the camera that have no loaded area: `0x683040` takes the
tiles within three of the camera's, culls them with their own frustum and the horizon, and
`0x6bd780` draws each, filling its buffers once through the two builders above.

Addresses: `0x683040`, `0x6bd780`, `0xc92078`
Confidence: inferred
Proof: none
Test: `terrain::scenewalk_arealow_far_project_difftest`, `terrain::scenewalk_camera_negate_difftest`

`0x683040` reads the area index from `0xc63274` shifted by 4 and steps ±3; the per-tile cull and
the projection it builds are not read here.

`terrain::scenewalk_arealow_far_project_difftest` runs `0x6830c4` to `0x6830d9`;
`terrain::scenewalk_camera_negate_difftest` runs `0x6831fc` to `0x683224`.
