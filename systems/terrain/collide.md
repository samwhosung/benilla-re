# terrain: collide

The world facets the movement code sweeps against: the query that fills a facet list from the
map's objects and terrain inside a box, the terrain cells' triangles, the walls an impassable
chunk raises, and the other volume and box queries over the same data.

The code is the collision gather object file, `0x6a9af0` through `0x6ad2c0`, the box query file
`0x6ad310` through `0x6ae0d0`, and the world entry `0x6721b0`. A facet is a 52-byte record: its
plane (n, d) at +0x00 and three vertices at +0x10, +0x1c and +0x28 (`collision/contact`). A list
holds its count at +4 and records at +8, and beside them, at +0x10, a count and an array of
8-byte owner ids, one per facet, which terrain leaves zero. The query mask selects classes:
0xf00 terrain, bit 16 + i liquid slot i, 0xf and 0xf00000 the chunk's doodads and entities.

Roots: `0x6721b0`

## World triangle query

`QueryWorldTriangles` (`0x6721b0`) empties the list and gathers the facets in the box through
`0x6aa8b0`. With mask bit 0x4000 it gathers into a private list instead and keeps only facets
whose normal's z is at least 0.64278764, cos 50°, copying each with its owner id. It returns the
gather's result, 1 for a completed query.

Addresses: `0x6721b0`, `0x6aa8b0`, `0x80dffc`, `0xc7b358`, `0x6721f0`, `0x6722c2`, `0x672307`
Confidence: verified
Proof: one
Test: none

`ret 8`, the box in ecx, the list in edx. `0x6721f0 test ah,0x40` picks the filtered path; the
private list `0xc7b358` is made once and registered for exit. `0x6722c2` compares record +0x08,
n.z, with 0.64278764 (0x3f248dbb, `0x80dffc`): `test ah,5; jnp` skips one below it, and a NaN is
kept. Each kept record is copied with `rep movs` of 13 dwords, and `0x672307` appends its owner
id from `0xc7b370`.

## Terrain gather

`0x6aa8b0(box, list, mask, owner)` advances the two query counters, empties the list, gathers the
map objects' facets (`0x6aaab0`), and, unless the map is one object, the terrain's: the box's
corners map to cells by u = (17066.666 − x)·0.24, each a float, taken − 0.5 and rounded, and
every chunk whose cells the box touches is gathered with the cell rectangle. The box must lie
inside [0, 34133.332) on both axes. It returns 0 when a step fails.

Addresses: `0x6aa8b0`, `0x6aaab0`, `0x6aadc0`, `0x810ae4`, `0x86aa2c`, `0xc9a348`, `0xc89f20`, `0x6aa967`, `0x6aa98c`, `0x6aa9b4`, `0x6aaa66`
Confidence: verified
Proof: one
Test: `terrain::aabb_query_negates_difftest`, `terrain::aabb_query_cell_indices_difftest`

`ret 8`. The four differences 17066.666 − max.x, − max.y, − min.x and − min.y are stored; the
first two must not be negative (`0x6aa967`, `test ah,5; jnp`, a NaN passing) and the others
below 34133.332 (`0x6aa98c`). From `0x6aa9b4` each is multiplied by 0.24 (`0x810ae4`), stored,
less 0.5 (`0x86aa2c`) and `fistp`. Chunk rows run from the max x cell >> 3 to the min x cell >> 3,
columns likewise from y; `0x6aaa66` passes column, row, the rectangle, the box, the list and the
mask, and the result is the AND of the chunks'.

`terrain::aabb_query_negates_difftest` runs `0x6aa930` to `0x6aa967`;
`terrain::aabb_query_cell_indices_difftest` runs `0x6aa9b4` to `0x6aaa14`.

## Chunk facets

`0x6aadc0` gathers a chunk's facets: four triangles per cell of the rectangle clamped to the
chunk, skipping holes, from its inner vertex C and corners V, V+1, V+17 and V+18 as (V+17, C, V),
(C, V+1, V), (C, V+17, V+18) and (C, V+18, V+1). A triangle is kept unless one face of the box,
grown by 0.019444443 and taken relative to the chunk, has all three vertices outside. The
vertices are stored as chunk vertex plus position. It also adds an impassable chunk's walls, the
liquid slots' facets and the doodads' and entities' by the mask.

Addresses: `0x6aadc0`, `0x6ab530`, `0x6ab4e0`, `0x6abc40`, `0x6abe60`, `0x810468`, `0x810438`, `0x8101b8`, `0x6aae2a`, `0x6aaed7`, `0x6aaf7c`, `0x6ab036`, `0x6ab1e0`, `0x6ab3f7`
Confidence: verified
Proof: one
Test: `terrain::chunk_gather_aabb_difftest`, `terrain::chunk_gather_aabb_nonzero_origin_difftest`

`ret 0x10`. `0x6aae2a` tests chunk +0xc bit 0x40 for the walls. The box's min adds the negated
position inline and its max through `0x4841c0` (`0x6aaed7`). Terrain needs mask & 0xf00. Each of
the five vertices `0x810468` lists (0, 9, 17, 1, 18) gets six bits, the sign bits of the stored
floats v − min + ε and max − v + ε per axis (`0x6aaf7c`, ε = 0x3c9f49f4). `0x6ab036` keeps a
triangle of `0x810438` when the AND of its codes is 0. `0x6ab1e0` adds the position to each
vertex and stores. Liquid slot i needs bit 16 + i (`0x6ab3f7`); the owner array grows to the
count with the new ids zeroed.

## Facet plane

A terrain facet's plane is n = (v1 − v0)×(v2 − v0), normalised, and d = −n·v0. With SSE the
components of v1 − v0 in x and y are stored and the rest kept at 53 bits, n is stored, scaled by
`rsqrtss` of |n|² and stored, and d uses the unrounded scaled n; without SSE the collision code's
`0x637480` builds the plane. With world flag 0x200000 the gather draws kept facets green and
rejected ones red.

Addresses: `0x637480`, `0xc9e388`, `0x6ab2ad`, `0x6ab2f0`, `0x6ab340`, `0x6ab34f`, `0x6ab373`, `0x6ab38a`
Confidence: verified
Proof: one
Test: none

`0x6ab2ad` tests `0xc9e388`. The x and y differences of v1 and v0 are stored at −0x5c and −0x58,
the rest are on the stack; `0x6ab2f0` stores n.x = e1.y·e2.z − e1.z·e2.y, with e1 = v1 − v0 and
e2 = v2 − v0, and the next two store n.y and n.z. |n|² is (x² + y²) + z², stored. `0x6ab340`
takes `rsqrtss`; each `fst` from `0x6ab34f` stores r·n while keeping it, and `0x6ab373` negates
(n.z·v0.z + n.y·v0.y) + n.x·v0.x. `0x6ab38a` tests 0x200000.

## Impassable walls

For an impassable chunk (MCNK flag bit 1), `0x6ab530` adds, for each side of the chunk's box the
query box crosses, two facets of a vertical wall 32,000 yards high along that side, facing out:
y-min when the box's min y is below the chunk's, y-max when its max y is above, then x-min and
x-max. A unit cannot walk out of the chunk across those sides.

Addresses: `0x6ab530`, `0x6acde0`, `0x6aca50`, `0x6ab53d`, `0x6ab599`, `0x6ab5df`, `0x6ab70d`, `0x6ab86c`, `0x6ab9ba`
Confidence: inferred
Proof: none
Test: `terrain::plane_from_np_difftest`, `terrain::impassable_chunk_fence_difftest`

`0x6ab53d` compares the box's min y with chunk +0x48 and builds the first wall when it is below,
with normal (0, −1, 0) (`0x6ab5df`), height 32000 (0x46fa0000, `0x6ab599`) and the chunk's
x extent; `0x6ab70d`, `0x6ab86c` and `0x6ab9ba` test the other three sides, whose normals are
(0, 1, 0), (−1, 0, 0) and (1, 0, 0). `0x6acde0` sets d = −n·p. The wall's corner and the
rounding of its far vertices are not read here.

## Other world queries

The map answers four more queries over the same data: the convex volume gather, the segment
trace (`trace`), the facet-plane query and the box hit test, each through a `World.cpp` entry.

Addresses: `0x6ac060`, `0x6ac1e0`, `0x6ab4e0`, `0x6aa160`, `0x6ad2c0`, `0x6ad4e0`, `0x6ad610`, `0x6ad7e0`, `0x6adbf0`
Confidence: inferred
Proof: none
Test: `terrain::aabb8_cell_indices_difftest`, `terrain::box_cell_indices_difftest`, `terrain::aabb_intersect_i`, `terrain::chunk_collect_aabb_difftest`, `terrain::chunk_collect_aabb_overflow_probe`, `terrain::chunk_gather_planes_difftest`, `terrain::facet_seg_gate_proceeds_difftest`, `terrain::chunk_box_translate_difftest`

`0x6ac060` gathers the facets inside eight corners, per chunk through `0x6ac1e0` against six
planes; `0x6aa160` returns the planes a segment hits, dispatching to the map objects and the
terrain; `0x6ad2c0` dispatches the box test to the map objects and to `0x6ad4e0`, which tests
each chunk through `0x6ad610` and `0x6ad7e0` with the rectangle clipped by `0x6adbf0`.
`0x6ab4e0` gathers a liquid slot's facets through `0x68cec0`. These entries were not read through.

`terrain::aabb8_cell_indices_difftest` runs `0x6ac0b2` to `0x6ac148`;
`terrain::box_cell_indices_difftest` runs `0x6ad4e0` to `0x6ad586`;
`terrain::facet_seg_gate_proceeds_difftest` runs `0x6aa160` to `0x6aa1a1`;
`terrain::chunk_box_translate_difftest` runs `0x6ad676` to `0x6ad6b0`.
