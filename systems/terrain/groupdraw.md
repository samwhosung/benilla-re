# terrain: map object draw

How the frame draws the groups of the map objects the visibility flood reached (`portals`): the pass
over the linked groups, the draw of one group for each of its frustum volumes, the batch draws
with their materials, the debug views, the liquid of a group, and the trace of a segment against the
map objects. The code is the map object file from `0x6b4d10` to `0x6b6b60` with the pass `0x684840`
and the trace `0x6be250`. A group's batches are 24 bytes (box as six 16-bit values, start index at
`+0xc`, index count `+0x10`, lowest and highest vertex `+0x12` and `+0x14`, flag byte `+0x16`,
material byte `+0x17`); a material is 64 bytes (flags `+0x0`, shader `+0x4`, blend `+0x8`, colour
`+0x10`, runtime colour `+0x14`, second colour `+0x1c`, textures `+0x38` and `+0x3c`).

Roots: `0x684840`, `0x6b3f90`, `0x6b4f10`, `0x6b5190`, `0x6b62e0`, `0x6b4060`, `0x6be250`

## Map object pass

`0x684840` draws the groups the frame linked in the list `0xc7caf8`. For each group def it unlinks
the def's node, and with world flag 0x100 set it sets matrix slot 8 to the translation by minus the
camera times the instance matrix `+0x94`, sets the group fog flag, builds the def's lighting
record, calls its two virtual slots, finishes the lighting at the camera and draws the group once
for each frustum volume the def holds. It releases the volumes in every case.

Addresses: `0x684840`, `0x684849`, `0x684853`, `0x684896`, `0x6849bf`, `0x6849e1`, `0x684ae3`, `0x684b1a`, `0x684c06`, `0x684c18`, `0x684c28`, `0x684c36`, `0x684c46`, `0x684c63`, `0x684c92`, `0x684ca3`, `0x684cab`, `0x684cbd`, `0x682380`, `0x682f90`, `0x58b240`, `0x71bc30`, `0x70a0f0`, `0x71c730`, `0x6b3f90`, `0xc7caf8`, `0xc7caf0`, `0xca7f00`, `0xc7b2a4`
Confidence: verified
Proof: one
Test: none

World flag bit 31 sets the device states 0x27 to 0x2a to 0.25 first (`0x684853`). The fog flag is 1
when the light state's `+0x7c` is nonzero, the def has an interior fog claim (`+0x98`) and the
instance is the state's claim `+0x98` (`0x684af3` to `0x684b26`), else 0. The draw is
`0x6b3f90(model +0x118, group +0x7c, matrix +0xd4, list +0x9c)`. `0x684c68` to `0x684cb0` release
each volume of the list with `0x513570` and `0x680920`.

## Group draw

`0x6b3f90(group, matrix, volumes)` draws a loaded group once for each stored volume. It runs the
portal vertex fade of the group once (the byte `+0xbc`), then for each volume loads it, moves it
into the instance's space with the matrix, and calls the group drawer: the interior drawer for a
group whose flags lack 0x48, the exterior drawer for the rest, with the group and the number of the
volume. World flag 0x40000000 then draws the vertex normals and 0x1000 the portals.

Addresses: `0x6b3f90`, `0x6b3fa1`, `0x6b3fa8`, `0x6b3fb5`, `0x6b3fdb`, `0x6b3fe3`, `0x6b3fe8`, `0x6b3ffc`, `0x6b401a`, `0x6b402d`, `0x6b403e`, `0x6b4048`, `0x6b4051`, `0x6a4a00`, `0x6c43d0`, `0x682d90`, `0x682ed0`, `0xcbe4b8`, `0xcbe584`, `0xcbe4b4`, `0xcbe580`, `0x6b5f90`, `0x6b5a30`
Confidence: verified
Proof: one
Test: none

The volume list is the def's `+0x9c`: the first node is at the list's `+0x8` and the next link at node
`+0x4` through the link offset in its first word (`0x6b3fbe`, `0x6b3fd0`). `0x682d90` copies the node's
volume to the frustum stack slot and `0x682ed0` forwards to `0x6868e0` with the matrix. Both
drawer slots are called with `ecx` the model plus a zero offset (`0xcbe4b8`, `0xcbe584`) and push the
group and the volume number (0 for the first), so a batch drawn for an earlier volume is not drawn
again.

## Batch buffers and cull

A batch is drawn only when the volume does not cull its box, and only once a frame. `0x6b4d10`
tests the box of 16-bit values against the current volume with `0x682f40`. `0x6b4da0(group,
format)` binds the group's vertex and index data: a block from the group vertex pool (age set to 30.0
at each bind, filled when new) and one from the index pool, or the group's own arrays when no block
is free. `0x6b5140` makes the draw record of a batch: primitive 3 (triangles), or 4 (strip) when
the strip switch `0xc96304` (world flag 0x10000 at map load) is set and bit 0 of the batch flag is.

Addresses: `0x6b4d10`, `0x6b4d8d`, `0x6b4da0`, `0x6b4dbf`, `0x6b4dc6`, `0x6b4ddc`, `0x6b4de4`, `0x6b4def`, `0x6b4dfb`, `0x6b4e55`, `0x6b4e8f`, `0x6b4e9a`, `0x6b4ead`, `0x6b4eb3`, `0x6b4ebd`, `0x6b4ee2`, `0x6b4efa`, `0x6b5140`, `0x6b514d`, `0x6b5154`, `0x682f40`, `0x6ba660`, `0x6ba770`, `0x6ba920`, `0x58a3d0`, `0x58a750`, `0x589a90`, `0xc96304`, `0x810cc8`
Confidence: verified
Proof: one
Test: none

The draw record of `0x6b5140` is the primitive at `+0x0`, the batch's start index at `+0x4`, its index
count at `+0x8`, lowest vertex at `+0xa` and highest at `+0xc` (the arguments of `0x58a830`). Format 3
is positions, normals and texture coordinates and format 4 adds the vertex colours (`0x6b4e0c`,
`0x6b4e2d`). Without a block the fallback sets the streams from `+0xd0`, `+0xd4`, `+0xd8` and `+0xf4`
with stride 0xc, 0xc, 8 and 4, and the indices from `+0xc8` (count `+0x128`) or, when `+0xcc`
exists and `0xc96304` is set, from `+0xcc` (count `+0x12c`). A block whose age reaches 0 returns
to the pool (`portals`, map object frame).

## Exterior group batches

`0x6b4f10(group, again)` draws the first `+0x40` batches of an exterior group: each batch that is
not marked drawn, not culled, with a loaded texture. Per material it sets lighting on unless flag
0x1, fog on unless flag 0x2, culling on unless flag 0x4, the material colour `+0x14` when the
flag 0x10 is set and else 0, the blend, and the texture with clamping per flags 0x40 and 0x80. A
specular material (shader 1) with the specular program loaded gets the program, a white specular colour
and shininess 14.0 for its draw. A zero `again` clears the drawn marks first.

Addresses: `0x6b4f10`, `0x6b4f25`, `0x6b4f32`, `0x6b4f37`, `0x6b4f57`, `0x6b4f5b`, `0x6b4f68`, `0x6b4f7c`, `0x6b4f95`, `0x6b4fab`, `0x6b4fbe`, `0x6b4fd2`, `0x6b4fdc`, `0x6b500f`, `0x6b5039`, `0x6b5066`, `0x6b5072`, `0x6b5077`, `0x6b507d`, `0x6b50a9`, `0x6b50be`, `0x6b50ca`, `0x6b5125`, `0x44acf0`, `0x589f40`, `0x589f50`, `0x589e60`, `0x589e80`, `0x58ade0`, `0x58ad30`, `0xca80bd`, `0xca80b8`, `0x810994`
Confidence: verified
Proof: one
Test: none

The state ids are 0xe lighting, 0xf fog, 0x14 culling, 0x2 colour, 0x7 blend, 0x17 stage 0 texture,
0x3f pixel program, 0x3 specular colour and 0x4 shininess. The batch's flag byte `+0x16` keeps the
drawn marks in its high nibble; `0x6b4f57` clears them for a zero `again` and `0x6b4f5b` skips a
marked batch. `0x6b4f7c` marks it. The texture flags start from `0x58a980(1, 0, 0, 0, 0, 0, 1, 0)`
and take bit 3 from the clear of material flag 0x40 and bit 4 from the clear of 0x80
(`0x6b5040` to `0x6b5063`). The loop bound `+0x40` is the third batch count, so batches before it are
drawn whatever their kind. The colour is 0 when flag 0x10 is clear (`0x6b4fdc` to `0x6b4ff4`).

## Interior group batches

`0x6b5190(group, again)` draws all of an interior group's batches (`+0x13c`) in three kinds by
position: the first `+0x3c` are transparent, the next `+0x3e` interior, the rest exterior.
Transparent batches draw twice, lit with blend 9 and unlit with blend 7. Interior batches draw unlit
with the material blend and a pixel program by shader. Exterior batches draw lit with the material
blend and no program. The interior fog colours replace the scene's when the group fog flag is set,
and a window material (flag 0x20) lights with the window colours.

Addresses: `0x6b5190`, `0x6b51a8`, `0x6b51b5`, `0x6b51cd`, `0x6b51de`, `0x6b51ef`, `0x6b5204`, `0x6b5224`, `0x6b5227`, `0x6b523f`, `0x6b52a6`, `0x6b52ba`, `0x6b52f7`, `0x6b5300`, `0x6b53bc`, `0x6b541f`, `0x6b5426`, `0x6b5431`, `0x6b543e`, `0x6b5458`, `0x6b5462`, `0x6b5471`, `0x6b547d`, `0x6b5484`, `0x6b548e`, `0x6b54bf`, `0x6b54c6`, `0x6b54d3`, `0x6b54e1`, `0x6b5524`, `0x6b5547`, `0x6b5560`, `0x6b5576`, `0x6b557b`, `0x6b55bd`, `0x6b562a`, `0x6b5673`, `0x6b568f`, `0x6b56a2`, `0x6b5765`, `0x71b7c0`, `0x589de0`, `0xcbe2d8`, `0xca7ecc`, `0xca8064`, `0xca8065`, `0xca8034`, `0xca8030`, `0xca8088`, `0xca7ee8`, `0xca8010`, `0xca7ed8`, `0xca80b8`, `0xca80bd`
Confidence: verified
Proof: one
Test: none

Transparent batches (`0x6b5428`): program TransSpecular for shader 1, else TransDiffuse, only when
`0xca8065` is set; lighting on and blend 9 for the first draw, lighting off and blend 7 for
the second. Interior batches (`0x6b54cc`): lighting off; with `0xca80bd` set, shader 3 gets Env and
shader 5 EnvMetal (stage 1 takes the second texture and state 0x30 is 4), others the Overbright
program when `0xca8064` is set; without `0xca80bd` only that choice. With the light LOD `0xca7ecc`
equal to 1, a batch with a nonzero shader is then drawn again lit with light 0 off, additive blend
10 and depth write off; shader 1 adds the Specular program, shader 2 the Metal program, each with a
white specular colour and shininess 14.0 (`0x6b557b` to `0x6b5671`). Exterior batches: lighting on, program none (`0x6b5673`).

## Group debug draws

World flags replace or add draws of a group. 0x80000 draws the faces by their flag, 0x800 clear draws
the vertex colours alone, 0x200 clear draws the textures alone, 0x1000 draws the portals and
0x40000000 the vertex normals. `0x6b5ba0` draws the faces with flag 0x20 and not 0x4 in
0xff00ff00, with 0x4 in 0xff8080ff and with 0x8 in 0x80ff0000 (blend 2).

Addresses: `0x6b57a0`, `0x6b58b0`, `0x6b5a30`, `0x6b5ba0`, `0x6b5f90`, `0x6b5bfa`, `0x6b5c05`, `0x6b5d1c`, `0x6b5e1f`, `0x6b5ada`, `0x6b5adc`, `0x6b5b4d`, `0x6b5f33`, `0x6b6098`, `0x6b6090`, `0x6b57ca`, `0x6b5a60`, `0x8012cc`
Confidence: verified
Proof: one
Test: `terrain::cmapobjgroup_normal_endpoint_difftest`

`0x6b57a0` draws every unculled batch unlit with the vertex colours, without a texture. `0x6b58b0`
draws every unculled batch unlit with its first texture and the texture coordinates alone. `0x6b5a30`
draws, for each portal reference of the group, a triangle fan around the portal's last vertex in
0x60ff00ff with blend 2 and no lighting, depth test, depth write or culling. `0x6b5ba0` runs for the
first volume only and reads the face flags from `+0xc4`. `0x6b5f90` draws a segment from each vertex
to the vertex plus 0.75 of its normal in 0xffffff00, 0x4000 vertices at a time.

## Group liquid

`0x6b4060(group, flag)` draws a group's liquid. The type is that of the first tile with liquid
(`0x6ba970`); types 0, 4 and 8 draw water, 2, 3, 6 and 7 draw magma or slime, and the rest nothing.
Water is drawn with blend 2 and no depth write, by the interior draw for a group
without flags 0x48 and the exterior draw for the rest. The flag is the parity flag the flood
stored; it is kept in `0xce269c`.

Addresses: `0x6b4060`, `0x6b406c`, `0x6b4074`, `0x6b407d`, `0x6b62e0`, `0x6b62ef`, `0x6b6307`, `0x6b6352`, `0x6b635b`, `0x6b6374`, `0x6b63a9`, `0x6b63b8`, `0x6b63c4`, `0x6b63c9`, `0x6b63d5`, `0x6b63e8`, `0x6b6387`, `0x6b6408`, `0x6b63fc`, `0x6ba970`, `0x68aac0`, `0x6b6420`, `0x6b6630`, `0x6b68f0`, `0x6a4a00`, `0xce269c`, `0xca7f00`
Confidence: verified
Proof: one
Test: none

`0x6b62e0` clears culling, sets the interior fog when the group fog flag is set, takes the frame texture
of the liquid type (`0x68aac0`) as stage 0 (nothing is drawn without one), and dispatches on the byte
table at `0x6b6408` (types 0 to 8: 0, 2, 1, 1, 0, 2, 1, 1, 0) through the three arms at `0x6b639f`
(water), `0x6b6387` (magma) and `0x6b63ed` (none); types above 8 draw nothing. The water arm sets
state 0x1f to 3, blend 2 and depth write off (`0x6b639f` to `0x6b63bf`).

## Group liquid mesh

A liquid is a grid of (columns × rows) vertices (`+0xf8`, `+0xfc`) with cells of 4.1666665 yards from the
origin `+0x108`, `+0x10c`. The vertex data (`+0x118`, 8 bytes each) holds a depth byte and the height at
`+0x4` (water) or two signed 16-bit coordinates and the height (magma). The tile bytes (`+0x11c`) hold
the type nibble, 0xf for no liquid. The strip skips a tile of type 0xf and, when bit 7 of its byte
is set, a tile unless the parity flag is set.

Addresses: `0x6b61a0`, `0x6b61b4`, `0x6b6209`, `0x6b6212`, `0x6b6220`, `0x6b6229`, `0x6b6263`, `0x6b62c9`, `0x6b6420`, `0x6b6446`, `0x6b64c6`, `0x6b64e9`, `0x6b651a`, `0x6b6587`, `0x6b65b1`, `0x6b6601`, `0x6b6630`, `0x6b6645`, `0x6b6672`, `0x6b6677`, `0x6b66be`, `0x6b6749`, `0x6b6833`, `0x6b6867`, `0x6b68c5`, `0x6b68f0`, `0x6b6928`, `0x6b69b3`, `0x6b6a3e`, `0x6b6a52`, `0x6b6ac2`, `0x6b6aed`, `0x6b6b2a`, `0x4a2c10`, `0x58a230`, `0x73f90a`, `0x58b210`, `0x58b220`, `0x6d48b0`, `0xc9607c`, `0xca7eb8`, `0xca7f10`, `0xc7b2a0`, `0x810cc0`, `0x8102e8`, `0x80a8ac`, `0x80e5a0`, `0x801620`
Confidence: verified
Proof: one
Test: none

`0x6b61a0(group, indices, base, count)` writes a triangle strip of 16-bit indices: for each row and
tile it opens a run with the base vertex twice and the vertex a row down, then adds the next
vertex and the one below for each tile, and closes the run with the last index repeated;
it adds the index count to `count`. The interior draw (`0x6b6420`, unlit with fog, vertices of 24 bytes) gives each vertex
the position, the colour whose alpha is the ramp `0xca7f10` of the depth byte and whose other bytes
are the second colour (`+0x1c` to `+0x1e`) of the material `+0x114` names, and the texture
coordinate (column, row); the colour goes through `0x4a2c10` when the device's capability word `+0x1c` is 1. The exterior draw
(`0x6b6630`, 36 bytes) adds the normal (0, 0, 1) and takes the colour bytes from the light state
`+0xec`; with `0xc9607c` set and the ExtWater0 program loaded it uses the program, white specular
and shininess 6.0. Magma (`0x6b68f0`) has white vertices, the coordinates of the data times 1/256,
and for types 6 and 7 a stage-0 texture matrix, an identity but for the v translation fmod(clock, 10) × 0.1 (state
0x37 on).

## Group liquid ramp

`0x6b6b60` fills `0xca7f10` with 256 bytes: the alpha of a liquid vertex by depth byte, a linear ramp from
the byte of the light state's `+0x114` to that of `+0x118`.

Addresses: `0x6b6b60`, `0x6b6b6a`, `0x6b6b76`, `0x6b6b8e`, `0x6b6ba2`, `0x6b6bbc`, `0x6b6bc3`, `0x6d48b0`, `0xca7f10`, `0x7ffe58`, `0x8029cc`
Confidence: verified
Proof: one
Test: none

Each end is `f32(v × 255 + 512)` shifted right 14 and truncated to a byte (`0x6b6b76`, `0x6b6b8e`); entry i is
`(a × 256 + i × (b − a)) >> 8` (`0x6b6bb5` to `0x6b6bbc`, with b − a as a signed 24-bit step).
The frame's group liquid pass builds it before drawing (`liquidpass`).

## Floor under a segment

`0x6be250(start, end, fraction, object out, ids out)` finds the nearest map object floor under a
segment. It clears the outputs, then for each instance whose flag byte `+0xd` has bit 0 clear and whose
box takes the segment it works in the model's space: it traces the groups of the instance's list
`+0x138` whose boxes the segment crosses, and asks the model which group the segment passes
through. A floor hit counts when it is less than 0.0001 farther than the best so far. The result is
the fraction, the instance, the group and, for an interior group, the face; if the best is an
exterior group the instance is cleared. It returns whether an instance remains.

Addresses: `0x6be250`, `0x6be25f`, `0x6be265`, `0x6be2a1`, `0x6be2b5`, `0x6be2e0`, `0x6be332`, `0x6be349`, `0x6be388`, `0x6be3a6`, `0x6be411`, `0x6be418`, `0x6be429`, `0x6be44c`, `0x6be451`, `0x6be472`, `0x6be477`, `0x6be47f`, `0x6be4ae`, `0x6c3330`, `0x6a47e0`, `0x6a4b10`, `0x6b92b0`, `0x6a3f80`, `0xca7da4`, `0xca7d9c`, `0x80c4f4`
Confidence: verified
Proof: one
Test: `terrain::nearest_param_delta_difftest`

The outputs start as no object and the ids 0xffff (`0x6be25f` to `0x6be26c`). The instance list is
`0xca7da4` by the link offset `0xca7d9c`. `0x6c3330` is the box test against the instance. The group
trace (`0x6b92b0`, mode 4) follows `0x6a47e0` and the group load check and zeroes the five batch
counters first (`0x6be388` to `0x6be3a0`); a hit stores the instance and the group with the face
id 0xffff. After the list, `0x6a3f80` runs with the fraction 1.0666666 (`0x6be411`). Its hit is
compared at `0x6be421` to `0x6be434` as `fcomp 0.0001; test ah,5; jp`: it is taken when the
difference to the fraction is below 0.0001 and refused when equal or NaN. `0x6be451` and
`0x6be477` test bit 3 of the group flags.
