# models: the shared model

The shared model, `CM2Shared`: one per `.m2` file, holding the file's bytes with every offset turned
into a pointer, the view the device draws, its textures and its device buffers. Every instance of
the file points at it.

The code runs from `0x71cd10` through `0x71e0a0`, with the header fixups in the next object file,
`0x71e0c0` through `0x71f840`, and the buffer helpers and index sort in the last, `0x71f860` through
`0x71faa0`. A shared model is 0x164 bytes, created and cached by the cache (`cache`); its fixed-up
header is `md`, at `+0x130`.

Roots: `0x71d5e0`, `0x71d610`

## Shared model object

| offset | holds |
|---|---|
| `+0x000`, `+0x004` | reference count, cache |
| `+0x008` | flags: 1 loaded, 2 the open flag the load was given |
| `+0x00c` | read request while loading |
| `+0x010` | instances waiting for the load (link `+0x1c`/`+0x20` in the instance) |
| `+0x014`, `+0x018`, `+0x01c` | release list links, release time |
| `+0x020` | file name, 0x104 bytes |
| `+0x124`, `+0x128`, `+0x12c` | bucket links, name hash |
| `+0x130`, `+0x134` | file data, file size |
| `+0x138`, `+0x13c` | selected view, texture handles |
| `+0x140` to `+0x14c` | device index buffer handles, device vertex buffer handles |
| `+0x150`, `+0x154`, `+0x158` | per-batch vertex and pixel shader ids, copy of the view's submeshes |
| `+0x15c`, `+0x160` | instance copies the device buffers hold, most instances one batched draw may take |

Addresses: `0x71d2c0`, `0x71d330`, `0x71d430`, `0x71d480`
Confidence: verified
Proof: one
Test: none

`0x71d2c0` stores the cache at `+0x4`, clears flags 1 and 2 of `+0x8`, sets `+0x0` to 1 and zeroes
the pointers listed. The destructor unlinks `+0x124`/`+0x128`, closes `md+0x5c` texture handles from
`+0x13c`, frees the buffers and `+0x150`, `+0x154`, `+0x158` (count-prefixed), and frees the file
data through `0x71f9d0`.

## Loading

`Load(file, loose)` records `loose != 0` as flag 2, allocates a buffer of the file's size and queues
a read request whose completion closes the file and finishes the load, whatever the flag (`0x71d590`). Finishing fixes up the header, initializes the view, sets the loaded flag and initializes
every instance that was waiting. An instance added after the load initializes at once.

Addresses: `0x71d4e0`, `0x71d5e0`, `0x71d610`, `0x71d640`, `0x71d5a0`, `0x71f9a0`, `0x70ebd0`
Confidence: verified
Proof: one
Test: none

`0x71d4e0` stores the size at `+0x134`, the buffer from `0x71f9a0` at `+0x130`, and fills the
request with the file, buffer, size, the shared model, the completion `0x71d5e0` at `+0x10` and the
read function `0x71d610` at `+0x14`. `0x71d640` returns at once when flag 1 of `+0x8` is set, runs
`0x71cdf0` and `0x71d6c0`, sets flag 1, and unlinks each waiting instance before `0x70ebd0`.

## Header fixup

The file starts with an `MD20` header of version 0x100 or 0x101 and at least 0x154 bytes. Every
array is a count and an offset; the fixup checks that the array ends inside the file and replaces
the offset with a pointer, or with 0 for an empty array, and does the same inside the views,
colours, textures, transparencies, texture frames, texture transforms, attachments, events,
lights, cameras, ribbons and particles. Any check that fails makes the load fail.

| header | array | element |
|---|---|---|
| `+0x08` | name | 1 byte |
| `+0x14` to `+0x2c` | global sequences, sequences, sequence lookup, playable table | 4, 0x44, 2, 4 bytes |
| `+0x34`, `+0x3c`, `+0x44` | bones, key bone lookup, vertices | 0x6c, 2, 0x30 bytes |
| `+0x4c`, `+0x54`, `+0x5c` | views, colours, textures | 0x2c, 0x38, 0x10 bytes |
| `+0x64`, `+0x6c`, `+0x74` | transparency, texture frames, texture transforms | 0x1c, 0x1c, 0x54 bytes |
| `+0x7c` to `+0xac` | replaceable lookup, materials, then five 16-bit lookups | 2, 4, 2 bytes |
| `+0xec` to `+0xfc` | collision indices, vertices, normals | 2, 12, 12 bytes |
| `+0x104` to `+0x134` | attachments, attachment lookup, events, lights, cameras, camera lookup, ribbons | 0x30, 2, 0x2c, 0xd4, 0x7c, 2, 0xdc bytes |
| `+0x13c`; `+0x144`, `+0x14c` | particles; two 16-bit arrays in version 0x101 | 0x1f8; 2 bytes |

Addresses: `0x71cdf0`, `0x71ce27`, `0x71ce36`, `0x71d284`, `0x71cd10`, `0x71e0c0`, `0x71e110`, `0x71e160`, `0x71e1b0`, `0x71e210`, `0x71e270`, `0x71e2f0`, `0x71e440`, `0x71e4f0`, `0x71e5e0`, `0x71e6d0`, `0x71e880`, `0x71e8d0`, `0x71e9d0`, `0x71eaa0`, `0x71edb0`, `0x71ef40`, `0x71f210`
Confidence: verified
Proof: one
Test: none

`0x71ce27` compares the magic with 0x3032444d and `0x71ce36` the version with 0x100 and 0x101.
Each helper takes the base, the size and the pair: offset greater than size, or `offset + n·size`
greater than size, returns 0. The element sizes are the multipliers of the helpers: `0x71e0c0` 8,
`0x71e110` 4, `0x71e160` 2, `0x71e1b0` 12, the others their `imul` constants. The version-0x101
arrays at `0x71d284` are fixed up after the result is recorded, even on failure.

## View selection

The view the shared model draws is the one with the most bones that still fits the vertex shader
palette: up to 256 bones, or `(constants − 31) / 3` when vertex shaders are on and that is
smaller. Without a fitting view the load fails. A material with blend 5 or above is marked unlit.
With vertex or pixel shaders the view's batches get their shader ids, and every texture with a
name is created with its wrap flags.

Addresses: `0x71d6c0`, `0x71df30`, `0x449d90`
Confidence: verified
Proof: one
Test: none

`0x71d6c0` multiplies `constants − 0x1f` by 0xaaaaaaab and shifts right by 1. A view's bone count
is its `+0x28`; ties keep the later view. `+0x160` is `0x10000` divided by the view's `+0x8`,
lowered to the view's bone count divided by each submesh's `+0xc`, at least 1. Materials are header
`+0x88`; the test sets bit 0 of the flags when the blend word is 5 or more. A texture record whose
name length `+0x8` exceeds 1 calls `TextureCreate` (`0x449d90`) with its flag bits 1 and 2 moved to
bits 8 and 0x10 of the cache's texture settings and flag 2 of `+0x8` as the open flag, so the textures may come
from loose files exactly when the model file could (`0x71da5e`).

## Vertex shader id

A batch's vertex shader id is `5·(3·(3·Z + t0) + t1)` with `Z = G + 2·L + 4·B + 20·P`: `G` is 1 when
material flag 2 is clear, `L` 1 when flag 1 is clear, `B` the submesh's word `+0x10` and `P` 1 when
the cache grants pixel shaders (bit 0x10). The digit `t_k` of stage `k` is 0 from the batch's
texture count `+0xe` on; else 2 when the coordinate lookup entry (header `+0xa0`, unit `+0x12 + k`)
is above 2; else 1 when the transform lookup entry (header `+0xb0`, unit `+0x16 + k`) is below the
transform count `+0x74`; else 0. The scene adds the nearest-light count to a lit element's id and
may rewrite `B`.

Addresses: `0x71de50`, `0x71de93`, `0x71dea5`, `0x71deaa`, `0x71debe`, `0x71deeb`, `0x71df18`, `0x71d909`, `0x711695`, `0x707cf6`, `0x707cfe`, `0x70aff0`, `0x70b040`
Confidence: verified
Proof: two
Test: `models::vertex_shader_id_difftest`

`0x71de93` shifts bit 4 of the cache's features `[[this+4]+4]` into `P`. The loop `0x71dea5` runs `k` below 2: `jae` to digit 0 at the count, `cmp word,2; jbe` at `0x71deaa` to digit 2, `0x71debe` compares the transform entry with `+0x74` to digit 1. `0x71deeb` is `not ax` on the material's flags word; the `lea` chain to `0x71df18` builds `Z`, then `3·x + t` twice and `5·x`. `0x70aff0` splits an id into `id mod 5`, `mod 3` twice, `& 1` twice, `mod 5` and the quotient, so `t1`, `t0`, `G`, `L`, `B` and `P` are digits 5 to 0 and digit 6 is the light count. `0x71d909` and `0x711695` call it with the unit and its submesh; `0x707cf6` adds `[collector + 0x180]` (`lights`) to the stored id when the material's flag 1 is clear, before `0x70b040`. A word `B` above 4 selects the programs of the other `P`, and `B + 5·P` above 9 an id past the 1800 slots. `vertex_shader_id_difftest` runs `0x71de50` whole.

## Model vertex programs

`Shaders\Vertex\Model2.bls` holds 1800 programs per level, one per vertex shader id: bytecode for
Direct3D vertex shader 2.0 (level 1), ARB text for OpenGL (level 2). A program reads the vertex
constants c6 to c9 only as stage matrices: c6 and c7 when `t0` is 1, c8 and c9 when `t1` is 1;
digits 0 and 2 read none. Digit 2 writes coordinate set `k` as `0.5 + 0.5·R.xy/|R|` with `R = P −
2(P·N)N`: `P` is the position and `N` the normalized normal, in the space `c2` to `c5` project from;
the 0.5 is a literal. Digit 0 passes coordinate set `k` through; digit 1 writes the dots of the two
constants with `(u, v, 0, 1)`.

Addresses: `0x706510`, `0x71de50`, `0x595130`, `0x5951e6`, `0x595211`, `0x598320`, `0x5990a7`, `0x5990ae`, `0x5990b6`, `0x58e6b1`, `0x58e6c7`, `0x70b8d0`, `0x70cd16`, `0x70cd34`, `0x58b330`
Confidence: verified
Proof: two
Test: none

`0x595211` calls slot 0 of the shaders in array order, so record `i` of the level's set is slot `i`
of the 1800 that `0x706510` loads. Level 1 is set at `0x5990a7` and level 2 at `0x58e6b1` (`gx`,
Shader files). Over the 1800 records of each level, by index digits: every program of a pair `(t0,
t1)` reads the constants above and no other of c6 to c9, 200 programs a pair; relative addressing
has only c31 to c33 as base, and `def` sets only c0. The rows c31 to c33 of the bone matrix take `P`
and the 3 by 3 part `N` when `B` is 0; a skinned program blends `c31[A0.x + i]` by the weights
first. The generated stage: Direct3D takes `dp3` of position and normal, `mad` with `-N`, `dp3` and
`rsq` of `R`, `mul` of `R.xy`, then `mad` by a literal 0.5 from `def c0`; ARB the same with a
`PARAM` literal, and writes (0, 1) to z and w. `0x70b8d0` also writes (0.5, 0, 0, 0.5) and (0, 0.5,
0, 0.5) to c(6 + 2k), c(7 + 2k), and `0x70cd16` to `0x70cd34` sends them. The census is made over
the install's file.

## Device buffers

The buffers hold `+0x15c` copies of the selected view, one per instance of a batched draw; a load
sets it to 1, and drawing a batched run of `n` raises it to `n` rounded up to 16, at most `+0x160`,
and frees the buffers to be rebuilt. Per copy the vertex buffer holds the vertices the view's lookup
names, in lookup order, 48 bytes each, their bone index bytes replaced by the view's palette bytes
plus `copy · bones`. Per submesh and copy the index buffer holds the submesh's triangle entries plus
`copy · lookup count` with shader ids, else the copy's vertex offset less the submesh's first
vertex; the submesh copy in `+0x158` gets its run's start.

Addresses: `0x71dab0`, `0x71dc80`, `0x71dfa0`, `0x71dff0`, `0x71d7c8`, `0x71db97`, `0x71dbc9`, `0x71dc25`, `0x71dd90`, `0x71ddbd`
Confidence: verified
Proof: one
Test: none

`0x71dfa0(n)`, called only at `0x70d3bf` in the batched run draw, grows `+0x15c` when it is below
both `n` and `+0x160`: `(n + 15) & ~15`, capped, then `0x71dff0` frees `+0x144`, `+0x140`, `+0x14c`
and `+0x148`. `0x71dab0` sizes `view+0x8 · +0x15c` words; per submesh it reads the start `+0x8` of
the view's record and copies `+0xa` entries of view `+0xc` from there (`0x71dbc9`), each plus the
base, which starts at 0 when `+0x150` is set and at `−copy+0x4` otherwise (`0x71db97`) and grows per
copy by view `+0x0` or the copy's `+0x6`; `0x71dc25` stores the run start in the copy's `+0x8`.
`0x71dc80` sizes `view+0x0 · +0x15c · 48` bytes, copies the vertex `view+0x4[p]` for each `p` in a
submesh's range (`0x71dd90`) and stores `view+0x14[p] + copy · +0xc · 0x01010101` at `+0x10`
(`0x71ddbd`). The buffers are named `CM2Shared_idx` and `CM2Shared_vtx`.
