# terrain: queries

The point queries on the loaded map: which cell a world position falls in, the terrain height
there, the liquid height and type, the area id, and whether the point is in a chunk's shadow.
The segment traces and the box gathers are in `trace` and `collide`.

The code is the map query object file, `0x69b100` through `0x69bff0`, and the height object file,
`0x6b7280` through `0x6b8380`. A world position (x, y) maps to grid coordinates by
u = (17066.666 − x)·k and v = (17066.666 − y)·k, with k = 0.24 per cell of 4.1666665 yards or
0.03 per chunk; the cell index is round-to-nearest(u − 0.5), which is floor(u) for every u not an
integer. The area is the table entry ((ix >> 7) & 63)·64 + ((iy >> 7) & 63) for cells, the
chunk the area's entry ((ix >> 3) & 15)·16 + ((iy >> 3) & 15), and the cell (ix & 7, iy & 7).
Only the shadow test checks the range; elsewhere the masks wrap a point off the map to a tile of
the table, which is loaded or not.

Roots: `0x69b350`

## Chunk height

`0x6b72a0(cell column, cell row, out)`, on a chunk with the point in edx, returns 0 in a hole and
otherwise writes the height of the triangle of the cell under the point and returns 1. Each
cell is four triangles around its inner vertex C, chosen by two signed tests against the cell's
diagonals; the height solves the triangle's plane n·q + d = 0 for z at the point's (x, y) and
adds the chunk's z. With SSE, which the client detects at start-up, n is normalised with
`rsqrtss`; without it, with an exact square root.

| tests | triangle |
|---|---|
| neither | C, V+17, V |
| first | C, V, V+1 |
| second | C, V+18, V+17 |
| both | C, V+1, V+18 |

Addresses: `0x6b72a0`, `0x672130`, `0x6720f0`, `0x4841f0`, `0x602630`, `0x8103f8`, `0x81047c`, `0x810480`, `0xc9e388`, `0x6920a7`, `0x6b72cc`, `0x6b7326`, `0x6b7358`, `0x6b737c`, `0x6b73fd`, `0x6b74e9`
Confidence: verified
Proof: one
Test: `terrain::vec3_cross_world`, `terrain::liquid_hit_flush_difftest`, `terrain::vec3_normalize_world`, `terrain::chunk_get_height`, `terrain::chunk_get_height_quadrants`

V is outer vertex row·17 + column, C = V + 9; the hole bit is (column >> 1) + 4·(row >> 1) of
+0x3c (`0x6b72cc`, `0x8103f8`). With p the point less the position, stored, test one is (V18.x − V.x)(V18.y − p.y) − (V18.y − V.y)(V18.x − p.x) ≤ 0 (`0x6b7326`, a NaN
fails), test two the same with V17 and V+1 (`0x6b7358`); the triangle's other vertices come from
`0x81047c` and `0x810480`, (17, 0), (0, 1), (18, 17), (1, 18). `0x6920a7` sets `0xc9e388` when
the processor word has bit 2. With A and B the table's two vertices, the SSE path stores
n = (A − C)×(B − C), scales it by `rsqrtss` of |n|² (`0x6b73fd`), and computes z −
((n.y·p.y + n.x·p.x) + d)/n.z with d = −((n.z·C.z + n.y·C.y) + n.x·C.x) at 53 bits; the other
path stores A − C, takes B − C (`0x4841f0`) and n = (B − C)×(A − C) (`0x672130`), each
component one stored 53-bit expression; `0x6720f0` scales n by 1/√((x² + y²) + z²), kept at 53
bits, d = −n·C (`0x602630`), and `0x6b74e9` divides. n's flipped sign leaves z unchanged.

## Surface height

`0x6b7070(point, out, range)` gives the highest surface within range of a point: the chunk's
terrain height, raised to the liquid height when that is higher, then the hit of a vertical
segment from z + range to z − range against map objects, doodads and models, keeping the higher
of hit and terrain. It returns 0 when range is below 0.01, when no area covers the point, and
when nothing is hit and the chunk gave no height, a hole with liquid included; with no chunk
the terrain starts at −100000. Off the map the indices wrap. The weather is its one caller.

Addresses: `0x6b7070`, `0x6b72a0`, `0x6b7500`, `0x6b7790`, `0x6b7b10`, `0x7089b0`, `0x7099e0`, `0x810d2c`, `0x86b6c0`, `0x6b707a`, `0x6b7124`, `0x6b7159`, `0x6b7183`, `0x6b722b`, `0x6b7258`, `0x67c801`
Confidence: inferred
Proof: none
Test: `terrain::sphere_cell_indices_difftest`, `terrain::sphere_seg_prep_difftest`, `terrain::sphere_compose_z_difftest`

`ret 4`. `0x6b707a` compares range with 0.01; a NaN passes. The cell coordinates use k = 0.24
(`0x810d2c`, 0x3e75c28f), each product stored, and `0x86b6c0`, 0.5. `0x6b7124` returns the empty
area entry, 0. `0x6b7159` passes the y cell, then the x cell, and keeps the result; the liquid
gets the two stored coordinates. `0x6b7183` raises the height when the liquid's is greater. On a
one-object map (`0xc9e384`) the terrain is skipped. The segment goes to `0x6b7790`, `0x6b7b10`
and the model manager's `0x7089b0` and `0x7099e0`, whose contracts are not read here, so the
entry is inferred. A hit fraction t below 1.0 gives top − (range + range)·t (`0x6b722b`),
stored, then replaced by the terrain height when that is greater; otherwise `0x6b7258` returns 0
when the chunk height returned 0, and writes the terrain height and returns 1 when it did not.

`terrain::sphere_cell_indices_difftest` runs `0x6b70c1` to `0x6b7100`;
`terrain::sphere_seg_prep_difftest` runs `0x6b71a9` to `0x6b71c6`;
`terrain::sphere_compose_z_difftest` runs `0x6b7225` to `0x6b7237`.

## Liquid height

`0x6b7500(u, v, out)`, on a chunk with the point in edx, takes u = (17066.666 − y)·0.24, v from
x. With i = round(u − 0.5), j = round(v − 0.5), fu = u − i and fv = v − j, stored,
it takes the first slot whose tile (j & 7, i & 7) is typed, else returns 0. Ocean (type & 3 ==
1) is height 0; otherwise, h01 being the next column's MCLQ height and h10 the next row's, a = (h01 − h00)·fu + h00, b = (h11 − h10)·fu + h10 and h = (b − a)·fv + a, at 53
bits, stored once. The masks wrap any u and v into the chunk.

Addresses: `0x6b7500`, `0x6b7177`, `0x69be70`, `0x6b750f`, `0x6b752d`, `0x6b756c`, `0x6b758c`, `0x6b759e`, `0x6b75c4`, `0x6b75cb`
Confidence: verified
Proof: one
Test: `terrain::liquid_height_sample`, `terrain::vec3_dist2_difftest`

`ret 0xc`. `0x6b7177` pushes the y coordinate last, so it is the first argument. The cells are
`fistp` of the arguments less 0.5 (`0x6b750f`) and the fractions each argument less its unmasked
cell (`0x6b752d`), stored. The tile byte is `[slot+0x10] + 8·j + i`, low nibble, 0xf for none
(`0x6b756c`). `0x6b758c and ecx,3; dec ecx` takes the ocean case. The heights are the floats at
+4 of vertices 9·j + i, +1, +9 and +10; `0x6b759e` loads h01, subtracts h00, multiplies by fu and
adds h00, and `0x6b75c4` multiplies b − a by fv before adding a. After the store (`0x6b75cb`), a
height above the point runs the liquid's flow records through `0x69be70` into locals that
nothing reads.

## Liquid status

`0x69b6d0(point, type, height, flow, flag)` reports the liquid at a point: a map object's liquid
first, then the chunk's slots. For each slot it writes bit 7 of the tile byte to the flag and,
for a tile with a type, accepts when the point is below the surface: an ocean tile when the
point's z is below 0.01, with height 0 and no flow, and another tile when the bilinear height
plus 0.01 is above z, after computing the flow when the height is above z. It writes type & 3
and returns 1, or returns 0.

Addresses: `0x69b6d0`, `0x69b520`, `0x81073c`, `0x86a750`, `0x8029d0`, `0x69b6ec`, `0x69b7e6`, `0x69b80d`, `0x69b819`, `0x69ba23`, `0x69ba3e`, `0x69ba72`
Confidence: verified
Proof: one
Test: `terrain::liquid_status_world`, `terrain::liquid_status_sound_zone`

`ret 0xc`. `0x69b6ec` calls the map object query, which on a hit zeroes the flag and returns 1.
The cells use k = 0.24 (`0x81073c`) and 0.5 (`0x86a750`). `0x69b7e6 shr eax,7` stores the flag
for every present slot. The blend repeats the liquid height's (`0x69b80d`). `0x69ba23` adds 0.01
to the stored height and compares with z; `0x69ba3e` compares z with 0.01 for the ocean, a NaN
failing both. The flow is the liquid's flow records scaled through `0x69be70` distances, written
to the flow vector; `0x69ba72` writes the type. A slot that fails continues the loop. As in the
liquid height, the column and fu come from y and the row and fv from x (`0x69b819` multiplies
by y's fraction).

## Area id

`0x69b120(point)` returns the MCNK area id (+0x16c) of the chunk under the point, or 0 where
no chunk is loaded; for a point off the map, that of the chunk its coordinates wrap to. Its chunk coordinates use k = 0.03; y's is one rounding of the product, x's
two, the difference being stored as a float before it is multiplied.

Addresses: `0x69b120`, `0x810740`, `0x69b131`, `0x69b13f`, `0x69b15a`, `0x69b172`, `0x69b182`, `0x69b1b3`
Confidence: verified
Proof: one
Test: `terrain::area_id_lookup_difftest`

`0x69b131` stores 17066.666 − x as a float; `0x69b13f` multiplies 17066.666 − y by 0.03
(0x3cf5c28f, `0x810740`) before its store, and `0x69b15a` multiplies the stored x. Both take
0.5 off and `fistp`. The area index uses bits 4 to 9 of the chunk coordinates, the chunk bits 0
to 3. `0x69b1b3` loads +0x16c. Nothing checks the range: `0x69b172` to `0x69b182` mask the chunk
coordinates, so a point off the map reads the tile they wrap to.

## Shadow test

`0x69b350(point)` returns whether the chunk's shadow map has a set bit under the point: the bit
at row round(1.92·(17066.666 − x) − 0.5) & 63 and column the same from y, the products stored.
It returns 0 outside [0, 34133.332) on either axis and where no chunk is loaded. It reads the
MCNK shadow pointer whether or not the chunk has a shadow map.

Addresses: `0x69b350`, `0x810744`, `0x7ffab0`, `0x69b37c`, `0x69b39c`, `0x69b42f`, `0x69b466`, `0x69b481`
Confidence: verified
Proof: one
Test: `terrain::tile_index_conversion_difftest`

Both differences are stored. `0x69b37c test ah,5; jnp` rejects a negative difference; a NaN
passes, and `0x69b39c` rejects one not below 34133.332 (`0x7ffab0`), a NaN passing again. The
chunk comes from k = 0.03. `0x69b42f` multiplies by 1.92 (0x3ff5c28f, `0x810744`); the pointer
is +0xf1c (`0x69b466`), not the +0x144 a loaded shadow map sets, and `0x69b481` tests bit
column & 7 of byte 8·row + column >> 3. The callers are doodad and light code (`entities`).

`terrain::tile_index_conversion_difftest` runs `0x69b356` to `0x69b371`, `0x69b3c1` to `0x69b3f2`
and `0x69b42f` to `0x69b466`.
