# terrain: footprints

The decals units leave on the ground as they walk: a ring of records, each a patch of the ground
triangles under a step with the footprint texture projected on them, faded out over six seconds.
The code is the footprint file, `0x699bd0` to `0x69a7d8`, called through the thunks at `0x670220`
(spawn, called only by the footstep handler `0x5fc093`) and `0x670240` (draw, called from the
frame at `0x483654`). The tables are created by the map initializer (`0x691b35`) and destroyed
with the map (`0x694920`).

Roots: `0x699bd0`, `0x699e70`, `0x69a3e0`

## Texture table

`0x699bd0` sizes an array of texture records to the largest FootprintTextures id plus 1 and loads
one texture for each row. The record is six words: the texture handle, then a record index list
(capacity, count, data, granule) and the count of vertices to draw this frame. It also sets the
two ring cursors: the unit ring starts at 0 and the general ring at 0x40.

Addresses: `0x699bd0`, `0x699bfb`, `0x699c14`, `0x699c1e`, `0x699cb0`, `0x699cd7`, `0x699cdf`, `0x699d11`, `0xc0dd20`, `0xc0dd24`, `0xc0dd2c`, `0xca0570`, `0xca0574`, `0xca0578`, `0xca057c`, `0xca0598`, `0x449d90`
Confidence: verified
Proof: one
Test: none

`0xc0dd20` is the loaded DBFilesClient\FootprintTextures.dbc store: rows of two words, the id
at `+0x0` and the texture name pointer at `+0x4`, the count at `0xc0dd24` and the largest id at
`0xc0dd2c`. `0x699bfb` takes that largest id plus 1 as the record count and `0x699cb0` stores it
at `0xca0578`; the array is grown (`0x69aad0`) or its surplus records destroyed (`0x69a890`) to fit.
`0x699cd7` indexes the record by the row's id (24 bytes each, `lea ecx,[eax+eax*2]` scaled by 8)
and `0x699d11` creates the texture of a record with no handle through `0x449d90`. Rows run from the
last to the first.

## Footprint spawn

`0x699e70` adds a footprint: texture record index (`ecx`), the size pair pointer (`edx`), then on
the stack the centre position pointer, the facing angle, the mirror flag, the terrain type and
the ring flag. It spawns nothing when the index is at or past the record count, the terrain type
is above the TerrainType id bound or its row is null or lacks bit 0 of the flags at `+0x14`, or
the ground has no triangle in the box. Otherwise the box holds the ground triangles under the step.

Addresses: `0x699e70`, `0x699e79`, `0x699e99`, `0x699eb6`, `0x699eca`, `0x699ee1`, `0x699ef5`, `0x699f06`, `0x699f1c`, `0x699f2d`, `0x699f87`, `0x699f8f`, `0x69a086`, `0x69a0a9`, `0x69a105`, `0x69a156`, `0x69a193`, `0x69a1af`, `0x69a200`, `0x69a2a0`, `0x6721b0`, `0x6d6fa0`, `0x7bca80`, `0x42c010`, `0xc0d67c`, `0xc0d684`, `0xc0d688`, `0xca05b0`, `0xca05f0`, `0x670220`
Confidence: verified
Proof: one
Test: `terrain::decal_query_box_difftest`, `terrain::liquid_tex_matrix_difftest`

The box is the centre plus and minus half the size pair's length in x and y and 0.3 in z; `0x6721b0` gathers into the list at `0xca05b0` with mask 0x200122 (`0x699ee1`). A
nonzero ring flag takes the next of the 64 slots of the first ring (cursor `0xca0570`, back to 0
at 0x40) and a zero flag the next of the 512 slots from 0x40 to 0x23f (cursor `0xca0598`, back
to 0x40); a footstep passes 1 when the unit is the active player (`0x5fc05c`). A record is 0x34 bytes at `0xca05f0`: `+0x0` position, `+0x10` texture
index, `+0x18` birth time (`0x42c010`), `+0x20` vertex count, `+0x24` vertex data. The matrix `0x69a2a0` mirrors x for the mirror flag, scales the longer axis of the size
pair by the ratio of the sizes and rotates by minus the angle about z. `0x6d6fa0` builds two
matrices from the box and that matrix with 0.5 and flag 1; each vertex of each gathered triangle
is stored (28 bytes: position, then a coordinate pair through each matrix with `0x7bca80`), so
a record holds 3 × the triangle count vertices. The TerrainType store is `0xc0d67c`
(rows of 0x1c bytes).

## Fade and draw

`0x69a3e0` runs only while `0xc7b2a4` has bit 0x400 (the `showfootprints` console command,
`0x6892e0`). Each live record's alpha is 1 − age/6000 with the age in ms; a record
below 0 is freed (its count zeroed) and one at 0 or above gets the byte min(round(alpha × 255 −
0.5), 127), so the footprint is at most half opaque. Live records are bucketed by texture and
drawn blended, alpha only, below the ground by a depth bias.

Addresses: `0x69a3e0`, `0x69a3e9`, `0x69a431`, `0x69a434`, `0x69a43a`, `0x69a440`, `0x69a474`, `0x69a483`, `0x69a494`, `0x69a4ad`, `0x69a4c7`, `0x69a4e5`, `0x69a50e`, `0x69a51f`, `0x69a530`, `0x69a53f`, `0x69a54a`, `0x69a5e5`, `0x69a6c2`, `0x69a748`, `0x69a765`, `0x69a798`, `0x69a7c8`, `0x6d6f90`, `0x8106dc`, `0x8106d8`, `0xca05a0`
Confidence: verified
Proof: one
Test: `terrain::footprint_fade_frac_difftest`, `terrain::footprint_gate_difftest`, `terrain::footprint_alpha_difftest`, `terrain::footprint_jitter_difftest`, `terrain::footprint_corner_ray_negate_difftest`

`0x69a3e9` tests `0xc7b2a4`. The fade uses the constant 1/6000 (`0x8106dc`) and the
age as a 64-bit integer (`fild qword`); `fcom` against 0 and `test ah,5; jp` (`0x69a448`) frees
below 0 only, so an alpha of exactly 0 is drawn. The byte is rounded to nearest, ties to even, by
`fistp`, and capped at 0x7f (`0x69a494`). Bucketing appends the record index to its texture's list
and adds its vertex count (`0x69a4c7`, `0x69a4e5`). The draw sets stage 1 to the ramp texture of
`0x6d6f90` (state 0x18), blend 2 (state 7), depth write off (state 0x12) and depth bias state 0 to
the `footstepBias` variable (record `0xc7f300`, value at `+0x24`, default 0.125) times 1/2048 (`0x8106d8`), and pushes matrix slot 8 as a
translation by minus the camera position (`0xc7cf20`), so the vertices are camera relative. For each
texture the vertices, 0x2c bytes each, hold the position, the normal (0, 0, 1) from `0xca05a0`,
the colour alpha << 24 | 0xffffff and the two coordinate pairs; the indices run 0 up in order.
The format is 6 (`0x69a748`), stage 0 is the texture (state 0x17), and `0x58a830` draws primitive 3
(triangles) over the bucket, whose list and count are then reset.
