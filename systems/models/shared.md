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
| `+0x008` | flags: 1 loaded, 2 asynchronous |
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

`Load(file, async)` records the asynchronous flag, allocates a buffer of the file's size and starts
a read request whose completion closes the file and finishes the load; a synchronous load reads at
once. Finishing fixes up the header, initializes the view, sets the loaded flag and initializes
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

Addresses: `0x71d6c0`, `0x71de50`, `0x71df30`, `0x449d90`
Confidence: verified
Proof: one
Test: none

`0x71d6c0` multiplies `constants − 0x1f` by 0xaaaaaaab and shifts right by 1. A view's bone count
is its `+0x28`; ties keep the later view. `+0x160` is `0x10000` divided by the view's `+0x8`,
lowered to the view's bone count divided by each submesh's `+0xc`, at least 1. Materials are header
`+0x88`; the test sets bit 0 of the flags when the blend word is 5 or more. A texture record whose
name length `+0x8` exceeds 1 calls `TextureCreate` (`0x449d90`) with its flag bits 1 and 2 moved to
bits 8 and 0x10 of the cache's texture settings, asynchronous when the load is.

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
