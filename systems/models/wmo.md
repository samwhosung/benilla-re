# models: map objects

The world model objects, `CMapObj`: the root file of a `.wmo` with its materials, groups, portals,
lights and doodad sets, and its groups, `CMapObjGroup`, each a file of its own with geometry, a
BSP tree and liquid. This doc covers how the files are read and what the rest of the client asks
of them.

Four object files hold the code: MapObj.cpp from `0x6a2c90` through `0x6a6710`, the group file
from `0x6b83c0` through `0x6bcff0`, the reader from `0x6c3840` through `0x6c4c20`, and AaBsp.cpp
from `0x692ba0` through `0x693e80`. The group file has no source path and is named after the
`VertArray@CMapObjGroup` type it allocates; the reader is named for what it does. The world's
map code (`terrain`) creates, places and draws map objects; the collision code queries them.

Roots: `0x693ca0`, `0x693ce0`, `0x693d20`, `0x6a3040`, `0x6a3100`, `0x6a3160`, `0x6a3230`, `0x6a33f0`, `0x6a3410`, `0x6a3420`, `0x6a3430`, `0x6a34b0`, `0x6a35b0`, `0x6a36d0`, `0x6a37b0`, `0x6a3a40`, `0x6a3b10`, `0x6a3c20`, `0x6a3cf0`, `0x6a3dc0`, `0x6a3ee0`, `0x6a3f10`, `0x6a3f80`, `0x6a4200`, `0x6a4270`, `0x6a42b0`, `0x6a4310`, `0x6a4420`, `0x6a44b0`, `0x6a45b0`, `0x6a4640`, `0x6a4670`, `0x6a47b0`, `0x6a47e0`, `0x6a4830`, `0x6a4990`, `0x6a4a00`, `0x6a4a30`, `0x6a4a50`, `0x6a4a90`, `0x6a4ad0`, `0x6a4b10`, `0x6a4cc0`, `0x6a4dc0`, `0x6a4e00`, `0x6a4fb0`, `0x6a5200`, `0x6a5620`, `0x6a5650`, `0x6a5710`, `0x6a57d0`, `0x6a6710`, `0x6b87c0`, `0x6b8810`, `0x6b8880`, `0x6b92b0`, `0x6b98c0`, `0x6b9d50`, `0x6b9f10`, `0x6ba410`, `0x6ba660`, `0x6ba770`, `0x6ba920`, `0x6ba970`, `0x6c3840`, `0x6c3f50`, `0x6c43d0`, `0x6c4c20`

## Root file chunks

A root file is parsed in a fixed chunk order without reading the tags: after the version chunk,
`MOHD`, `MOTX`, `MOMT`, `MOGN`, `MOGI`, `MOSB`, `MOPV`, `MOPT`, `MOPR`, `MOVV`, `MOVB`, `MOLT`,
`MODS`, `MODN`, `MODD` and `MFOG` follow one another, each pointer stored and each record count
derived from the chunk size. Only the last, `MCVP`, is tested: it is taken when it lies inside the
file and carries its tag.

| chunk | record | chunk | record |
|---|---|---|---|
| `MOMT` | 64 bytes | `MOVV` | 12 bytes |
| `MOGI` | 32 bytes | `MOVB` | 4 bytes |
| `MOPV` | 12 bytes | `MOLT` | 48 bytes |
| `MOPT` | 20 bytes | `MODS` | 32 bytes |
| `MOPR` | 8 bytes | `MODD` | 40 bytes |
| `MFOG` | 48 bytes | `MCVP` | 16 bytes |

Addresses: `0x6c3a60`, `0x6c3a00`, `0x6c3a20`, `0x6c3990`, `0x6c3b2d`, `0x6c39d0`, `0x6a3496`, `0x6c39e0`, `0x6a5650`
Confidence: verified
Proof: one
Test: none

`0x6c3a60` starts at file `+0xc` (the data at map object `+0x1c0`) and calls `0x6c3a00` with each
tag; `0x6c3a00` returns the cursor and adds 8, ignoring the tag, and the caller adds the chunk size.
The counts are `size >> 6`, `size >> 5`, and multiplications by 0xaaaaaaab or 0xcccccccd followed by
shifts, which divide by 12, 48, 20 and 40. `0x6c3a20` compares the cursor with `+0x1c0 + +0x1c4` and
the tag with `MCVP`. The `MOGI` count `size >> 5` goes to `+0x168` (`0x6c3b2d`); `0x6c3990` then
stores it at `+0x1f0`, which the reset `0x6a3430` seeds to 384 (`0x6a3496`), and allocates a group
object for each group into the pointers from `+0x1f4`, 384 of which end the 0x7f4 fixed bytes;
`0x6a5650` allocates 0x7f4 plus the caller's extra bytes. The loop at `0x6c39e0` runs to the `MOGI`
count with no cap at 384, so a file of more than 384 groups writes its pointers past the fixed part,
into the extra bytes and beyond.

## Group file chunks

A group file's required chunks also follow in a fixed order without tag checks: `MOPY` (2-byte
face records), `MOVI` (2-byte indices), `MOVT` and `MONR` (12-byte vectors), `MOTV` (8-byte
coordinates) and `MOBA` (24-byte batches). The vertex stride is 32 bytes when the group's flags
have 8 or 0x40 and 36 otherwise; the optional chunks follow by the group's flags.

Addresses: `0x6c40c0`, `0x6c4530`, `0x6c3f80`
Confidence: verified
Proof: one
Test: none

`0x6c40c0` takes the first chunk at its argument and stores each data pointer from `+0xc4` and
each count from `+0x124`: `size >> 1` twice, `size·0xaaaaaaab >> 35` twice (12 bytes), `size >> 3`,
and `size·0xaaaaaaab >> 36` (24 bytes). It tests group `+0x10` with 0x48 and stores 32 or 36 times
the vertex count at `+0xec`, then calls `0x6c4530`. The index byte count `+0xf0` is twice `+0x12c`
when `+0xcc` and a global flag are set, else twice `+0x128`.

## Group header

A group file is the version chunk and one `MOGP` chunk whose data opens with a 0x44-byte header;
the geometry chunks start right after it. The header fields land in the group:

| header | group | holds |
|---|---|---|
| `+0x00` | `+0xc0` | name, an offset into the root's `MOGN` turned into a pointer |
| `+0x08` | `+0x10` | flags |
| `+0x0c` | `+0x14` | bounding box, six floats |
| `+0x24`, `+0x26` | `+0x2c`, `+0x30` | first portal reference and count, 16-bit |
| `+0x28` to `+0x2c` | `+0x3c` to `+0x40` | three batch counts, 16-bit |
| `+0x30`, `+0x34`, `+0x38` | `+0x34`, `+0x38`, `+0x14c` | fog indices, liquid word, group id |

After the chunks, every batch resolves the textures of its material, the batch's byte `+0x17`.

Addresses: `0x6c3f80`, `0x6c3fa6`, `0x6c3ff5`, `0x6c4053`, `0x6c4080`, `0x6c3db0`
Confidence: verified
Proof: one
Test: none

`0x6c3f80` advances the file pointer `+0x150` by 0x14 (version chunk and the `MOGP` header), reads
the fields above with `rep movs` for the box, then `add eax, 0x44` and calls `0x6c40c0`. It first
unlinks the group's node and links it again into the root's group list (`+0x1e4`, which holds the
node's offset in the group), and last sets `+0x164` to 1 and `+0xbc` to 0. The loop at `0x6c4080` steps the batches by 0x18.

## Material textures

A material record is the file's 64-byte `MOMT` record, used in place. Its first texture is the
`MOTX` string at offset `+0xc`, or `createcrappygreentexture.blp` when that string is empty; its
second, at `+0x18`, is loaded only when its string is not empty. The handles go to `+0x38` and
`+0x3c`, once: a material whose first handle is set is skipped.

Addresses: `0x6c3db0`, `0x6c3d70`, `0x6c4c20`, `0x86c3c0`
Confidence: verified
Proof: one
Test: none

`0x6c3db0(index)` takes record `+0x1d8 + 64·index`, returns at once when `+0x38` is non-zero, and
calls `0x6c4c20` with `MOTX` (`+0x124`) plus the offset, or with the string at `0x86c3c0`; an empty
second string stores 0 at `+0x3c`. `0x6c3d70` zeroes both handles of every material.

## Loading

A map object is looked up by name in a cache and created on a miss; its root file is read,
asynchronously when asked, and parsed when the read completes. Each group file, named after the
root with `_%03d` and `.wmo`, loads the same way when first needed.

Addresses: `0x6a3230`, `0x6c3890`, `0x6c3840`, `0x6c3990`, `0x6a4a30`, `0x6c3e20`, `0x6c3f50`, `0x6c3f80`
Confidence: inferred
Proof: none
Test: none

`0x6a3230` hashes the name into the cache and calls `0x6c3890` for a new entry; `0x6c3e20` formats
the group name with `_%03d`. The completion paths `0x6c3840` and `0x6c3f50` close the read request
and the file before parsing.

## Queries

The collision and world code ask a map object for segment and box hits, facets in a box or a
volume, liquid heights, and the vertex colour under a point. A group answers through
its axis-aligned BSP, whose leaves are welded into cached meshes on first use, and through its
liquid grid of 4.1666665-yard tiles.

Addresses: `0x6a37b0`, `0x6a3b10`, `0x6a3c20`, `0x6a3dc0`, `0x6b92b0`, `0x6bc370`, `0x6bc1c0`, `0x693d60`, `0x6b9f10`, `0x6b9a50`
Confidence: inferred
Proof: none
Test: `models::aabsp_box_query_difftest`, `models::bspdispatch_normalize_difftest`, `models::mliq_liquid_height_difftest`, `models::seg_face_cross_difftest`, `models::seg_face_dot_difftest`, `models::seg_normal_scale_difftest`, `models::wmo_mocv_barycentric_color_difftest`

Each map object query guards on `+0x1d4` and each group through `0x6a4a00`. The segment walk is
`BSP segment descent`; the liquid grid scales by 1/4.1666665.

Of `0x6a37b0`, `seg_face_cross_difftest` runs `0x6a3920`–`0x6a39b8`, `seg_face_dot_difftest`
`0x6a39bd`–`0x6a39f4`, `seg_normal_scale_difftest` `0x6a3a11`–`0x6a3a36`; of `0x6b92b0`,
`bspdispatch_normalize_difftest` runs `0x6b9354`–`0x6b9388`; `wmo_mocv_barycentric_color_difftest`
runs `0x6b9a50` whole; `mliq_liquid_height_difftest` runs `0x6b9f10` whole;
`aabsp_box_query_difftest` runs `0x6bc1c0` whole.

## BSP segment descent

A group's BSP node is 16 bytes: a 16-bit flags word (0x4 leaf, low two bits the split axis), the
negative and positive children (`+0x2`, `+0x4`, 0xffff for none), the face count `+0x6`, the first
face reference `+0x8`, and the split distance `+0xc`. A segment descends with the node's box: it
stops when both ends lie more than 0.01 beyond the box on the split axis; with `f0` and `f1` the
ends' distances from the split, it visits both children whole when either is within 0.01, one
child when both are beyond 0.01 on one side, and otherwise splits at `t = f0/(f0 − f1)`, the
start's side first. A leaf tests its cached mesh, else each face.

Addresses: `0x6bc370`, `0x6bc394`, `0x6bc3e9`, `0x6bc4c5`, `0x6bc4fc`, `0x6bc535`, `0x6bc56e`, `0x6b88e0`, `0x6bc700`, `0x810ccc`
Confidence: verified
Proof: one
Test: `models::aabsp_leaf_tscale_difftest`, `models::aabsp_segment_clip_difftest`, `models::aabsp_segment_split_difftest`, `models::wmobsp_leaf_tscale_difftest`

`0x6bc3e9` masks the axis with 3 and compares `p − box.min` with −0.01 (`0x810ccc`) and
`box.max − p` with 0.01. The child boxes copy the parent's with `min[axis]` or `max[axis]` set to
the split. `0x6bc56e` computes `f0/(f0 − f1)` with `fdivr` and the midpoint `p0 + (p1 − p0)·t`;
`f0 > 0` takes the positive child first. A leaf calls `0x6b88e0`, and on 0 `0x6bc700` for each of
its `+0x6` face indices.

Of `0x6b88e0`, `aabsp_segment_clip_difftest` runs `0x6b88e0`–`0x6b88f4`,
`wmobsp_leaf_tscale_difftest` `0x6b8af8`–`0x6b8b2a`; `aabsp_segment_clip_difftest` runs `0x6bc370`
whole; of `0x6bc370`, `aabsp_segment_split_difftest` runs `0x6bc56e`–`0x6bc5bc`; of `0x6bc700`,
`aabsp_segment_clip_difftest` runs `0x6bc700`–`0x6bc719`, `aabsp_leaf_tscale_difftest`
`0x6bc78d`–`0x6bc7bb`.

## Bounding spheres and volume

A loaded map object's bounding sphere is centred at `(min + max)·0.5` of its box, the sum rounded
to a float before the halving, with radius `|max − centre|`; a group's comes the same way from its
group record's box. An unloaded map object gives a zero sphere. A point is inside the map object's
convex volume when every plane gives `n·p + d ≤ 0`, a NaN counting as inside, and with no planes
it is inside; an unloaded map object contains nothing.

Addresses: `0x6a4310`, `0x6a44b0`, `0x6a4990`, `0x6a49d7`
Confidence: verified
Proof: one
Test: `models::wmo_bounding_sphere_difftest`, `models::wmo_point_in_convex_volume_difftest`

`0x6a4310` adds `+0x1ac` to `+0x1a0` and so on into a stored vector, multiplies by 0.5, and takes
`sqrt` of the squared distance from the centre to `+0x1ac`. `0x6a44b0(out, index)` reads the
32-byte record at `+0x130`, box at `+0x4` and `+0x10`. `0x6a4990` walks `+0x194` planes of 16
bytes at `+0x15c` (the `MCVP` chunk), computes `((ny·py + nz·pz) + nx·px) + d` and returns 0 at the
first plane where `test ah,0x41` finds neither C0 nor C3 (`0x6a49d7`).

`wmo_bounding_sphere_difftest` runs `0x6a4310` whole; `wmo_bounding_sphere_difftest` runs `0x6a44b0`
whole; `wmo_point_in_convex_volume_difftest` runs `0x6a4990` whole.

## Portal vertex fade

When a group's `MOCV` colours are read, every vertex of a face whose `MOPY` flags lack bit 0 gets
alpha 0xff, unless a global switch is set. On the group's first draw, each vertex up to the
highest vertex of the batches its first batch count covers is faded toward the outside: `d` is its least distance to a portal
of the group that leads to a group with flags 0x8 or 0x40. At `d = 0` the colour becomes
0xffffffff; for `0 < d < 6.666667` with alpha 0, with `t = 1 − 0.15·d`, each of r, g, b becomes
`c + (255 − c)·t` and alpha `255·t`, each byte taken as `(x + 512) >> 14`.

Addresses: `0x6c41e0`, `0x6c4605`, `0x6c43d0`, `0x6c4340`, `0x6c4240`, `0x6c443e`, `0x6c4449`, `0x6c4468`, `0x6b3fb5`
Confidence: verified
Proof: one
Test: `models::aabsp_nearest_dist_difftest`, `models::wmo_vertex_fog_blend_difftest`

`0x6c4605` calls `0x6c41e0` unless the byte at `0xca8064` is set. `0x6b3fb5` calls `0x6c43d0` while
group `+0xbc` is 0; it sets `+0xbc` and loops vertices 0 through batch `+0x14` of batch
`+0x3c − 1`. `0x6c4340` walks the group's `+0x30` portal references from `+0x2c`, testing the other
group's record flags with 0x48. `0x6c4240` returns 0 when the squared plane distance is under
1/36 and the point lies in the portal polygon, else the least edge distance. The constants are
6.666667, 0.15 and 255.0.

`aabsp_nearest_dist_difftest` runs `0x6c4240` whole; of `0x6c4340`, `wmo_vertex_fog_blend_difftest`
runs `0x6c4340`–`0x6c435a`; `wmo_vertex_fog_blend_difftest` runs `0x6c43d0` whole.

## Vertex buffer pools

The group file also holds the vertex buffer pools the world draws terrain and map objects from:
blocks of a device vertex and index buffer, split, merged and evicted by size.

Addresses: `0x6ba1e0`, `0x6ba340`, `0x6ba410`, `0x6ba5a0`, `0x6ba660`, `0x6ba770`, `0x6b87c0`
Confidence: inferred
Proof: none
Test: none

`0x6ba1e0` creates the buffers named `VBBList_vtx` and `VBBList_idx`; `0x6b87c0`, called from the
map code, creates two pools.
