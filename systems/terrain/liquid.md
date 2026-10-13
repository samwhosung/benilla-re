# terrain: liquid

The chunk's liquid surface: the 9×9 grid an MCLQ block holds, built into chunk-relative vertices
and drawn as strips over the 8×8 tiles of one type, with depth-driven texture coordinates and 30
animated texture frames per liquid type.

The code is the liquid object file, `0x689e20` through `0x6913c0`, which also holds the water
effects (`water`). A `CChunkLiquid` is 1,036 bytes from the `WCHUNKLIQUID` pool, the name taken
from its buffer pools `CChunkLiquid_vtx` and `CChunkLiquid_idx`; a chunk has four liquid slots
(`chunk#liquid-blocks`). An MCLQ vertex is 8 bytes and ends in its height float; the tile bytes
hold the liquid type in their low nibble, 0xf for none.

Roots: `0x68c4c0`, `0x690ae0`

## Liquid vertices

`0x68d540` writes a liquid's 81 vertices at +0x34, nine rows of nine, relative to the chunk
position like the terrain's outer vertices: vertex (j, i) is (j·sx, i·sy, h − z), with sx and sy
the chunk's steps (`chunk#chunk-vertices`), h the MCLQ vertex's height and z the chunk position's
z, each a float.

Addresses: `0x68d540`, `0x68d5ee`, `0x68d636`, `0x68d654`, `0x68d666`
Confidence: verified
Proof: one
Test: `terrain::liquid_grid_verts`

The chunk is the liquid's +0x1c. The steps repeat the terrain's: four `fild; fstp; fmul
33.333332; fstp`, four negated sums with 17066.666 stored, the two differences (`0x68d5ee`)
negated and stored, each divided by 8 and stored. A row's x is the float j·sx (`0x68d636`),
copied to each of its vertices; y is one rounding of i·sy (`0x68d654`); z is `fld [vertex+4];
fsub [chunk+0x74]` stored (`0x68d666`). The vertex pointer is +0x0c, stepped 8 bytes to 0x288.

## Liquid strips

`0x68d9b0(type)` fills the liquid's index buffer with one triangle strip over the 8×8 tiles whose
type nibble equals the argument, row by row, and returns the index count. A run of matching tiles
in a row starts with v, v, v + 9 and adds v + 1, v + 10 for each tile, v = row·9 + column; a run
ends, at a non-matching tile or the row's end, by repeating its last index.

Addresses: `0x68d9b0`, `0x58a080`, `0x58a0a0`, `0x68da03`, `0x68da0e`, `0x68da41`, `0x68da66`
Confidence: verified
Proof: one
Test: `terrain::liquid_strip_indices`

`ret 4`. The buffer +0x404 is locked by `0x58a080` and unlocked by `0x58a0a0`. The tile byte is
`[[this+0x10] + 8·row + column] & 0xf` against the argument (`0x68da03`). The first match of a
run writes ax twice and the up-row index (`0x68da0e`) and sets the in-run byte; every match
writes v + 1 and v + 10 and keeps the latter. `0x68da41` and `0x68da66` repeat it to close. The
count is the bytes written over 2 (`sar eax,1`).

## Liquid render vertices

The liquid's vertex buffer holds 81 vertices of 40 bytes: the position, the normal (0, 0, 1), a
first texture coordinate and a second (i·0.25, j·0.25). For the ocean pass the first is
(0.5, B[d]) and for the river pass (0.5, A[d]), d being the MCLQ vertex's first byte; for the
magma pass it is the vertex's first two words times 3/256.

Addresses: `0x68d790`, `0x68d690`, `0x68d890`, `0xc81768`, `0xc7fcd8`, `0x810388`, `0x68d7d9`, `0x68d818`, `0x68d925`
Confidence: verified
Proof: one
Test: `terrain::mclq_verts_a`, `terrain::mclq_verts_b`, `terrain::liquid_render_verts_fill`

`0x68d790` takes `fild` of the row count times 0.25 once per row (`0x68d7d9`) and of the column
times 0.25 per vertex, each stored. `0x68d818` loads the float the byte indexes in `0xc81768`;
0.5 is the local 0x3f000000. `0x68d690` is the same function with `0xc7fcd8`. The river pass
`0x68db90` calls `0x68d790`, the ocean pass `0x68dab0` calls `0x68d690`, and the magma pass
`0x68dca0` calls `0x68d890`, which converts the two words with `fild` and `fmul` by 0.01171875
(0x3c400000, `0x68d925`); its second coordinate is not read here.

## Depth ramps

At start-up `0x68c4c0` builds the tables the vertex fills read. With c = 0.58333331, for i from
0 to 255, B[i] = min(1, (c·i)·(1/(c·255))) and A[i] = (i·(1/9))·0.21428572, or 1.0 when
i·(1/9) is above 4.6666665, B at `0xc7fcd8` and A at `0xc81768`; every value is a float of a
53-bit computation. It also fills 64 alpha
bytes, floor of 255·min(1, 1.6·(i/63)⁸), into the top byte of `0xc7fbc0`'s dwords, makes three
liquid textures and loads the shader `ocean0_s`.

Addresses: `0x68c4c0`, `0xc81184`, `0xc7fcd8`, `0xc81768`, `0xc7fbc0`, `0x81028c`, `0x810384`, `0x810380`, `0x81037c`, `0x810358`, `0x68c535`, `0x68c567`, `0x68c589`, `0x68c5c9`, `0x68c5d9`, `0x68c615`
Confidence: verified
Proof: one
Test: `terrain::depth_ramps_difftest`, `terrain::glare_table_difftest`, `terrain::geom_rate_difftest`

c is `0xc81184`, set at start-up to −21 × (−1/36). `0x68c535 fdivr` leaves 1/(c·255) on the stack
for the whole loop. `0x68c567 fcom 1.0` keeps the product only below 1, a NaN taking 1.0. The
second value is st0 = i times 1/9 (`0x81028c`); `0x68c589 fcom` against 4.6666665 multiplies by
0.21428572 when not above and otherwise stores 1.0. The alpha table takes t = i·(1/63)
(0x3c820821), then t² = t·t, t³ = t²·t, t⁴ = t·t³ and t⁸ = t⁴·t⁴ (`0x68c5c9` to `0x68c5d9`), all
on the stack, scales by 1.6, clamps to [0, 1], and `0x68c615` adds 512 to 255
times it and stores a float, whose bits >> 14 give the byte.

`terrain::depth_ramps_difftest` runs `0x68c51b` to `0x68c5b9`; `terrain::glare_table_difftest` runs
`0x68c5c0` to `0x68c636`; `terrain::geom_rate_difftest` runs a copy of `0x68c65c`.

## Animated textures

`0x68aac0(type)` returns the current frame of liquid type 0 to 7: with a period of 1.25 s,
truncated to 1,250 ms, frame = nearest(float(30·(t mod 1250)/1250) − 0.5) for the clock t in
milliseconds. The 30 frames load on first use from `XTextures\river\lake_a.%d.blp`,
`ocean\ocean_h`, `lava\lava`, `slime\slime`, then `lake_a`, none, `lava` and `slime` for types
4 to 7, numbered 1 to 30.

Addresses: `0x68aac0`, `0x8102b8`, `0x86a000`, `0xc811c0`, `0xc89da4`, `0x869ffc`, `0x68aadb`, `0x68aafc`, `0x68ab11`, `0x68ab3d`
Confidence: verified
Proof: one
Test: `terrain::liquid_anim_frame_difftest`

The period is `__ftol` (`0x68aadb`) of `0x8102b8[type]` (1.25 for all eight) times 1000; the
clock's remainder is a 64-bit `fild`, `fidiv` the period (`0x68aafc`), times 30, stored, less 0.5
(`0x869ffc`), and `fistp` (`0x68ab11`) under the round-to-nearest control word. Frame k's name
takes k + 1 (`0x68ab3d`). `0xc89da4[type]` is set once every frame has loaded; the handles are
`0xc811c0[30·type + frame]`. Type 5's name pointer is null.

`terrain::liquid_anim_frame_difftest` runs `0x68aacc` to `0x68ab14`.

## Liquid render passes

`0x68de40(pass)` draws a liquid: pass 0 is the river, pass 1 the ocean and pass 2 magma, after
making the liquid's buffers when it has none and creating the pass's ramp texture once. Each pass
binds the current frame of type 4, 1 or 6, refills the vertices and the strips of tile type 4, 1
or 6 when the buffers are fresh, and draws the strip with the 40-byte vertex format. Tiles of any
other type are never drawn by these passes.

Addresses: `0x68de40`, `0x68dab0`, `0x68db90`, `0x68dca0`, `0x68cc60`, `0x68dac0`, `0x68db2a`, `0x68db7e`, `0x68dba2`, `0x68dc0a`, `0x68dcb0`, `0x68dd1a`
Confidence: verified
Proof: one
Test: `terrain::liquid_pass_anim_frame_difftest`, `terrain::liquid_lazy_init_scalar_difftest`

`0x68de40` switches on its argument to `0x68db90`, `0x68dab0` and `0x68dca0`. The frame calls
pass ecx = 4 (`0x68dba2`), 1 (`0x68dac0`) and 6 (`0x68dcb0`); the strip calls push 4
(`0x68dc0a`), 1 (`0x68db2a`) and 6 (`0x68dd1a`), and store the count at +0x408. `0x68db7e` draws
4 with the count and 0x50 as the last vertex. The three passes' callers are the scene passes
(`cull`); what each pass sets in the device beyond the frame is not read here.

`terrain::liquid_pass_anim_frame_difftest` runs `0x68dd26` to `0x68dd41`;
`terrain::liquid_lazy_init_scalar_difftest` runs `0x68dea6` to `0x68deb7`.

## Liquid buffers

A liquid's vertex and index buffers (+0x400, +0x404) come from a pool of buffer pairs, and the
chunk's per-frame step returns them after 5 seconds without a draw (`cull`).

Addresses: `0x68ca50`, `0x68cc60`, `0x68cd50`, `0x68cea0`, `0xc81b80`
Confidence: inferred
Proof: none
Test: none

`0x68cd50` puts the pair back into the arrays at `0xc81b80`; `0x68cc60` takes one;
`0x68ca50` makes the pool's buffers named `CChunkLiquid_vtx` and `CChunkLiquid_idx`. The
release's caller and its 5.0 test are `0x6afc06`.

## Drift cloud

The world builds one drift cloud, the motes in liquid, at start-up: 4000 records of 16 bytes, a camera-relative position and, at `+0xc`, its quad size. The scatter
places every record uniformly in a cube of edge E around the camera, z, y and x from the first,
second and third draws of the client random generator (`random`), and sets the size to s/2 + u·s with u from a fourth
step of the same generator, s the size base. At creation E is 30 and s 1/36. The draw sizes each quad by `+0xc`; `drift-cloud-scatters` gives
when the cloud scatters again.

Addresses: `0x66f971`, `0x66f9a3`, `0x66f9a8`, `0x66f9af`, `0x68e5a0`, `0x68e650`, `0x68e657`, `0x68e670`, `0x68e720`, `0x68e729`, `0x68e75c`, `0x68e77d`, `0x68e7c6`, `0x68e8ea`, `0x68e8f8`, `0x68e902`, `0x68e91e`, `0x68efe0`, `0x68f249`, `0xc63180`
Confidence: verified
Proof: one
Test: none

`0x66f971` allocates 0xfa40 bytes and `0x66f9af` constructs them with the size base 0x3ce38e39,
the edge 30.0 and a texture path, storing the cloud in `0xc63180`. `0x68e5a0` zeroes the
positions, sets the count through `0x68e650`, 1.0 times `[0x81038c]` (`0x68e657`), and scatters.
`0x68e720` takes s/2 and 1.5·s (`0x68e729`) and E/2 (`0x68e75c`), calls `0x4531e0` three times
(`0x68e77d`) for (u − 1)·E − E/2 per axis (`0x68e7c6`), advances the generator state inline and
stores (u − 1)·s + s/2 at `+0xc` (`0x68e8ea` to `0x68e902`), then the mode, its argument & 3
(`0x68e91e`). The draw multiplies each corner offset by the record's
`+0xc` (`0x68f249`).

## Drift cloud scatters

A change of the camera's liquid to water or ocean scatters the cloud, then sets s to 1/36; to
magma it scatters, then sets 1/9; slime clears the enable byte `+0xfa14`. So the previous base
holds until the advect re-scatters with the current base and mode, when the camera moves more
than E, or until the liquid changes again. Both scatters need `[0xc7b2a4]` bit 0x2000000.

Addresses: `0x680afa`, `0x680b0d`, `0x680b1d`, `0x680b39`, `0x680b4c`, `0x680b5c`, `0x680b75`, `0x68e930`, `0x68e9da`
Confidence: verified
Proof: one
Test: none

The liquid probe tests the bit (`0x680afa`, `0x680b39`), calls `0x68e720` at `0x680b0d` and
`0x680b4c` before `0x68e670` with 1/36 (`0x680b1d`) or 1/9 (`0x680b5c`), and for slime stores 0 in
`+0xfa14` (`0x680b75`). `0x68e930` calls `0x68e720` with the stored mode `+0xfa24` at `0x68e9da`.
