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

## Map object gather

`0x6aaab0(box, list, mask, flags)` gathers the facets of the map objects and their groups in a
box, 1, or 0 when one there is not ready. Each instance of the map object list whose box `+0x44`
overlaps the box, but one with bit 0 of byte `+0xd` or no map object `+0x118`, needs its map
object loaded (`+0x1d4`) and a def group, or the gather fails. The box, moved about its centre
into the instance's space, is tested against the map object (`0x6a4670`): a pass gathers its
group facets under 0xf0, ORing `s_batchOverflow` into `*flags`, its liquid under 0xf0000.
Each overlapping def group must be loaded and its doodads (0xf0000f) ready (Doodad gather leg),
then its game objects under 0xf00000.

Addresses: `0x6aaab0`, `0x6aaae6`, `0x6aaaf0`, `0x6aab1f`, `0x6aab2b`, `0x6aab3c`, `0x6aab61`, `0x6aab6b`, `0x6aab87`, `0x6aab9a`, `0x6aabaa`, `0x6aabb8`, `0x6aabdb`, `0x6aac06`, `0x6aac28`, `0x6aac34`, `0x6aac4e`, `0x6aac5c`, `0x6aac6c`, `0x6aac8a`, `0x6aaca3`, `0x6aacb5`, `0x6aacba`, `0x6aaccd`, `0x6aad0c`, `0x6aad1b`, `0x6aad2b`, `0x6aad37`, `0x6aad4c`, `0x6aad58`, `0x6aad6d`, `0x6aad89`, `0x6aada7`, `0x6aadb2`, `0x6acb40`, `0x699330`, `0x6a42b0`, `0x6a4670`, `0x6a3b10`, `0x6a3c20`, `0x671cc0`, `0x6abc40`, `0x6abe60`, `0x6dc3f0`, `0xca7d9c`, `0xc63258`, `0xc5a474`, `0xc7b378`, `0xc63190`
Confidence: verified
Proof: one
Test: `terrain::collectinbox_negate_difftest`

`ret 8`. The centre is `(min + max) · 0.5` (`0x6aaae6`, `0x6aaaf0`); the box less the centre is
kept (`0x6aab1f` to `0x6aab37`). Per instance, `0x6acb40` (every component of its min at most the
box's max) and `0x699330` (its max at least the box's min) are the overlap, a NaN failing it. The
failures are `0x6aabaa` and `0x6aabb8` to `0x6aabc8`, jumping to `0x6aada7`. The centre moves by
`+0xd4` (`0x6aabdb`); `0x6aac06` builds the 3×3 of that matrix's upper rows and `0x6aac28` passes
it and the centred box to `0x6dc3f0`, the centre added back after (`0x6aac34`). `0x6aac6c` zeroes
`s_batchOverflow`, `0xc5a474`, `0xc7b378`, `0xc63190` and one more batch global before
`0x6a3b10`, and `0x6aaca3` emits through `0x671cc0` with `+0x158`, `+0x15c`. Groups are tested by
`+0x44` and `+0x50` (`0x6aad0c`, `0x6aad1b`), then `0x6a42b0(group index +0x7c)`, which is the
group's parsed byte `+0x164`, 0 for an unloaded map object (`0x6aad2b`). The group walk runs
whether or not `0x6a4670` passed. `terrain::collectinbox_negate_difftest` runs `0x6aaaf5` to
`0x6aab1c`.

## Doodad gather leg

`0x6abc40(doodads, box, list, mask)` returns 1 at once without any of 0xf0000f in the mask, else
walks a doodad def list and returns 0 at the first def it would take that is not loaded. A def is
passed over when its flag word `+0xc` has bit 0x100, its stamp `+0x8c` is the query id or it has
no model `+0x88`; it is taken by the mask under 0xf00000 when it has a game object's owner
pair `+0x180`, `+0x184`, else under 0xf. A taken def needs bit 0x80 (loaded); its box `+0x14c`
overlapping the query box adds its facets (`0x6abd90`); either way it is stamped. It returns 1 at
the list's end.

Addresses: `0x6abc40`, `0x6abc4c`, `0x6abc60`, `0x6abc89`, `0x6abc90`, `0x6abc9f`, `0x6abcb1`, `0x6abcc5`, `0x6abcc7`, `0x6abccf`, `0x6abcda`, `0x6abcee`, `0x6abcf6`, `0x6abd1d`, `0x6abd46`, `0x6abd54`, `0x6abd70`, `0x6abd7b`, `0x6abd90`, `0xc89f20`, `0x6aad4c`, `0x6ab4a1`
Confidence: verified
Proof: one
Test: none

`ret 8`; the list in `ecx` holds its link offset at `+0` and its head at `+8`, each node's def at
`+4`. `0x6abc90 test ah,1` is bit 0x100; `0x6abcc5` the owner pair's `or`. `0x6abcda` is `test al,al;
jns` to `0x6abd70`, `xor eax,eax`. The box test copies the def's box `+0x14c` to the stack (`0x6abcee`) and
compares each min with the box's max (`test ah,0x41; jp`) and each max with its min (`test ah,1;
jne`), a NaN failing it. `0x6abd54` stores `s_mapQueryId`. The callers are the map object gather
(`0x6aad4c`, a group's `+0xa8`) and the chunk gather (`0x6ab4a1`).

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

A chunk whose header flag bit 1 is set (`0x40` in the runtime word +0xc, written once at `0x6af677`)
fences itself. For each side the query box crosses, `0x6ab530` adds a vertical quad of two facets
along the chunk's own box. Min and max are chunk +0x44 to +0x4c and +0x50 to +0x58 (x, y, z); the
quad rises 32000.0 from the chunk's lowest height `C.minZ`. The walls ignore the query's class mask, the
flag is read once per chunk, and the chunk's own triangles are gathered as for any chunk.

| side | test (strict) | start p | edge e | normal |
|---|---|---|---|---|
| y-min | Q.min1 < C.min1 | (C.min0, C.min1, C.minZ) | (+W, 0, 0) | (0, −1, 0) |
| y-max | Q.max1 > C.max1 | (C.max0, C.max1, C.minZ) | (−W, 0, 0) | (0, 1, 0) |
| x-min | Q.min0 < C.min0 | (C.min0, C.max1, C.minZ) | (0, −D, 0) | (−1, 0, 0) |
| x-max | Q.max0 > C.max0 | (C.max0, C.min1, C.minZ) | (0, +D, 0) | (1, 0, 0) |

Addresses: `0x6ab530`, `0x6aba30`, `0x6acde0`, `0x6aca50`, `0x6ab53d`, `0x6ab599`, `0x6ab5df`, `0x6ab70d`, `0x6ab86c`, `0x6ab9ba`, `0x6aae2a`, `0x6af677`, `0x6aadc0`
Confidence: verified
Proof: one
Test: `terrain::plane_from_np_difftest`, `terrain::impassable_chunk_fence_difftest`

W and D are the chunk's x and y extents. The tests are `fcomp` then `test ah,5` with `jp` (min sides, strictly
less) and `test ah,0x41` with `jne` (max sides, strictly greater); the query's z is never read. With
u = (0, 0, 32000.0) (`0x46fa0000`, stored at `0x6ab599`) the facets are (p, p+e, p+e+u) and (p, p+e+u, p+u):
vertices at +0x10, +0x1c and +0x28 of the 52-byte record, normal at +0, `d = −n·p` (`0x6acde0`). The
x-max side goes through `0x6aba30`; each facet is appended with `0x6aca50`. `0x6aae2a` runs only for a chunk found in the tile table at
`0xc96318`, before the cell loop, so a flag covers the whole chunk. `0x6ab530` reads one stack argument
(the collector, `ret 8`); the mask in the other is unread. The walls' normals point away from the chunk.
`0x6aadc0` is called only by the world box query `0x6aa8b0`, which only the box gather `0x6721b0` calls (from `0x5d6ab0`, `0x60fe25`,
`0x63172f`, `0x6320b1`, `0x6320e0`, `0x633b46`, `0x633e90`, `0x63451f` and `0x699ee1`), so segment queries never see a wall.

## Triangle batches

The liquid and map object queries record the triangles they hit as batches in one shared table, and
`0x671cc0` turns them into facets. A batch header is 0x20 bytes at
`0xc62550` (at most 31 per query, the count at `0xc5a474`): `+0x0` the matrix, `+0x4` the vertex
array (12-byte vertices), `+0xc` the index triples (16 bits), `+0x14` the index count, `+0x16`
the triangle count, `+0x18` and `+0x1a` the lowest and highest index. The map object
trace keeps the face words of its hit at the first header's `+0x10`. Matrices are 0x40 bytes at
`0xc62980` (31), indices come from a pool of 0xc000 at `0xc63298`; a full table sets bit 0 of
`0xc63258` and the batch is dropped.

Addresses: `0x68cec0`, `0x68d029`, `0x68d053`, `0x68d06f`, `0x68d0c7`, `0x68d107`, `0x68d10f`, `0x68d127`, `0x68d153`, `0x68d168`, `0x68d194`, `0x68cf45`, `0x68d015`, `0x6aac6c`, `0x810304`, `0x8101b8`, `0x68d032`, `0xc62550`, `0xc62560`, `0xc62980`, `0xc63298`, `0xc63258`, `0xc63190`, `0xc5a474`, `0xc7b378`
Confidence: verified
Proof: one
Test: none

`0x68cec0`, the liquid slot's gather, is one producer. For each cell of a rectangle of the slot's
9 × 9 grid whose type nibble is not 0xf it takes the triangles (v, v+9, v+10) and (v, v+10, v+1)
of the table at `0x810304`, v being 9 × column + row, and drops a triangle whose three vertices are
all below or all above the box in z, with the margin `0x8101b8` (`0x68cf45`, `0x68d015`). The first
kept triangle takes the next header (`0x68d029`) unless the count plus 1 reaches 0x20, zeroed
with the lowest index 0xffff (`0x68d053`), and the next matrix (`0xc63190`): an identity with the
translation `[liquid + 0x1c] + 0x6c` (`0x68d0c7`). The vertex array is the slot's, at `liquid + 0x34`.
The pool takes 6 indices per cell of the rectangle at cursor `0xc7b378`, only below 0xc000
(`0x68d10f`). Each index is stored with the cell's base added (`0x68d153`), the bounds widen
(`0x68d168`) and `0x68d194` counts the triangle. The map object gather zeroes the counters, flag
included, before it runs (`0x6aac6c`).

## Batch flush

`0x671cc0(list, owner)` (`ecx` unused, owner two words on the stack) appends one facet per triangle
of every batch to the list, in batch order. The vertices are the batch's, moved by its matrix. The
normal is (v1 − v0) × (v2 − v0) normalized, d = −n·v0. Every new facet gets the owner pair, and the
list's owner array is grown and counted to match.

Addresses: `0x671cc0`, `0x671cd0`, `0x671d16`, `0x671d53`, `0x671dd6`, `0x671e5a`, `0x671e94`, `0x671fd7`, `0x672000`, `0x672012`, `0x67202b`, `0x672034`, `0x672076`, `0x6720d0`, `0x6aaca3`, `0x6ab521`, `0x6ac851`, `0x6ac9af`, `0x6bbcf8`, `0x6bbd58`, `0x6742d0`, `0x7bca80`, `0x4841f0`, `0x672130`, `0x6720f0`, `0x602630`, `0xc9e388`
Confidence: verified
Proof: one
Test: `terrain::liquid_hit_flush_difftest`

Callers are the map object facet gather, the liquid slot gathers, the volume gathers and the group
liquid segment query. `0x671cd0` tests `0xc9e388` (the SSE flag): with it, `0x671e5a` scales the
cross product by `rsqrtss` of its squared length, the 12-bit estimate of the reciprocal root,
so the unit normal is only that accurate; without it `0x672012` crosses (`0x672130`), normalizes
exactly with `fsqrt` and a divide (`0x6720f0`) and takes the dot (`0x602630`). `0x671d16` makes room
for a facet (`0x6742d0`) and the vertex positions are written at `+0x10`, `+0x1c` and `+0x28`.
The plane is stored as n at `+0x0` and d at `+0xc`. After the loops `0x672076` compares the list's new
count with the old: when it grew past the owner array's count and capacity, the array is resized
to a multiple of its granule, its count is set, and `0x6720d0` stores the two owner words in each new
slot, 8 bytes each. The cross product is the first vector `v1 − v0` crossed with `v2 − v0`
(`0x4841f0` subtracts; its second pushed argument is the second vector).

## Other world queries

The map answers four more queries over the same data: the convex volume gather, the segment
trace (`trace`), the facet-plane query and the box hit test, each through a `World.cpp` entry.

Addresses: `0x6ac060`, `0x6ac1e0`, `0x6ab4e0`, `0x6aa160`, `0x6ad2c0`, `0x6ad4e0`, `0x6ad610`, `0x6ad7e0`, `0x6adbf0`
Confidence: inferred
Proof: none
Test: `terrain::aabb8_cell_indices_difftest`, `terrain::box_cell_indices_difftest`, `terrain::aabb_intersect_i`, `terrain::chunk_collect_aabb_difftest`, `terrain::chunk_collect_aabb_overflow_probe`, `terrain::chunk_gather_planes_difftest`, `terrain::facet_seg_gate_proceeds_difftest`, `terrain::chunk_box_translate_difftest`, `terrain::aabb_intersect_i_difftest`

`0x6ac060` gathers the facets inside eight corners, per chunk through `0x6ac1e0` against six
planes; `0x6aa160` returns the planes a segment hits, dispatching to the map objects and the
terrain; `0x6ad2c0` dispatches the box test to the map objects and to `0x6ad4e0`, which tests
each chunk through `0x6ad610` and `0x6ad7e0` with the rectangle clipped by `0x6adbf0`.
`0x6ab4e0` gathers a liquid slot's facets through `0x68cec0`. These entries were not read through.

`terrain::aabb8_cell_indices_difftest` runs `0x6ac0b2` to `0x6ac148`;
`terrain::box_cell_indices_difftest` runs `0x6ad4e0` to `0x6ad586`;
`terrain::facet_seg_gate_proceeds_difftest` runs `0x6aa160` to `0x6aa1a1`;
`terrain::chunk_box_translate_difftest` runs `0x6ad676` to `0x6ad6b0`.

`aabb_intersect_i_difftest` runs `0x6adbf0` whole.
