# terrain: render

How a visible chunk's texture layers are drawn: on the fixed-function path, one pass per layer
blended by its alpha texture and then the shadow; on the shader path, one pass whose pixel
shader is chosen by the layer count and reads the combined texture.

The code is the chunk draw object file, `0x6be960` through `0x6c07d0`, and the map render file,
`0x6b34c0` through `0x6b7070`, which also draws the map objects' groups. The layer textures
repeat once per cell: the texture matrix scales chunk-relative positions by 1/sy, sy being the
chunk's column step, the y of vertex 1 (+0x84c).

Roots: none.

## Chunk layer draw

`0x6bef30(batch)` draws a chunk's texture layers. A chunk with no layers, or whose view depth less its
radius has reached the fog end `0xc62964`, is drawn flat. A layer with flag 0x80 forces the multipass
path. Otherwise, with the terrain shader flag `0xc9a31c` the pixel program `0xc962cc[count]` draws all the layers in
one pass, and with `0xc89f44` the program `0xc96088[count]`, each when it is available and the device
has at least count + 1 stages; any other chunk takes the multipass path.

Addresses: `0x6bef30`, `0x6bef4c`, `0x6bef52`, `0x6bef5d`, `0x6bef73`, `0x6bef84`, `0x6bef8d`, `0x6befb1`, `0x6befc3`, `0x6befd1`, `0x6befda`, `0x6beffc`, `0x6bf00c`, `0x6bf01a`, `0x6bf672`, `0x6bfaf0`, `0x6bf690`, `0x58b360`, `0x58a230`, `0xc62964`, `0xc9a31c`, `0xc89f44`, `0xc962cc`, `0xc96088`
Confidence: verified
Proof: one
Test: none

`0x6bef52` compares the depth less the radius with the fog end (`0x6bef5a test ah,1; je`), so a chunk
at or beyond the fog end, or a NaN, goes to `0x6bf672`, which calls `0x6bfaf0`. The loop at `0x6bef71`
tests bit 7 of each layer's flag byte `+0x8`. The program is loaded from the table at the layer count
(`0x6bef8d`, `0x6befda`), tested by `0x58b360` and compared at `0x6befb1`; a failure falls to the
multipass path (`0x6bf01a`), not to the other program.

## Chunk layers in multipass

On the multipass path the chunk draws opaque with the first layer and alpha blended after it, one pass
a layer. Each layer's texture is stage 0; a later layer's alpha map is stage 1 with the combine preset 1.
Both stages generate their coordinates from the vertex position (mode 2) through a texture matrix that
scales by −1/sy and moves by the negated offset of the chunk from the camera; stage 1 scales by a
further −0.125. The shadow comes after the layers as one more pass.

Addresses: `0x6bef30`, `0x6bf01a`, `0x6bf035`, `0x6bf27f`, `0x6bf28f`, `0x6bf294`, `0x6bf2a3`, `0x6bf2b2`, `0x6bf2c1`, `0x6bf2d0`, `0x6bf2e4`, `0x6bf2f4`, `0x6bf307`, `0x6bf318`, `0x6bf338`, `0x6bf35b`, `0x6bf592`, `0x6bf5b9`, `0x6bf5be`, `0x6bf5d8`, `0x6bf5e7`, `0x6bf5f7`, `0x6bf604`, `0x6bf617`, `0x6bf61c`, `0x58b210`, `0x7bdd00`, `0x810e8c`, `0x810e88`, `0xc6328c`, `0xc6319c`, `0xc962cc`, `0xc96088`
Confidence: verified
Proof: one
Test: none

`0x6bf035` divides −1.0 by the chunk's `+0x84c` (sy); the stage 0 matrix is the scale of that
(`0x7bdd00`) translated by the negated chunk position `+0x6c` to `+0x74` less the camera
(`0xc7cf20`), pushed with `0x58b210` for stage 0 (`0x6bf27f`) and, scaled by `0x810e8c` (−0.125), for
stage 1 (`0x6bf28f`); states 0x2f and 0x30 are 2 and 0x37 and 0x38 are 1 (`0x6bf294` to `0x6bf2c1`).
With a terrain shader flag the combined texture `+0x13c` is stage 1 with the program `0xc89f1c` (white
specular and shininess 20.0 for `0xc9a31c`) or `0xc89ef8`. The shadow pass (`0x6bf592`) runs only
without a program, with a shadow texture `+0x140` and world flag 0x40: colour `0xc6328c`, blend 2, the
texture `0xc6319c` on stage 0, preset 1 and the shadow on stage 1, one draw; the colour returns to white
(`0x6bf617`). Every stage state is then cleared (`0x6bf61c` to `0x6bf664`).

## Chunk layer states

Each layer sets its own states between the passes. A null texture is skipped. Flag bit 7 turns lighting
off for the layer. Flag bit 6 marks an animated layer: the direction is the record of `0xc5a480` at 16
bytes times (flags & 7), the speed divisor is `0x86c318[(flags >> 3) & 7]` (64, 48, 32, 16, 8, 4, 2, 1),
and the stage 0 matrix is pushed again, translated by the record's three floats times −sy over the
divisor. A program takes the layer's constants and blends 0 for the first layer, 2 after; without one,
a layer with an alpha texture `+0x14` blends 2 with preset 1 and stage 1 set to it, else 0.

Addresses: `0x6bef30`, `0x6bf380`, `0x6bf3a9`, `0x6bf3b2`, `0x6bf424`, `0x6bf42e`, `0x6bf43a`, `0x6bf458`, `0x6bf464`, `0x6bf46e`, `0x6bf4a1`, `0x6bf4ce`, `0x6bf4fc`, `0x6bf511`, `0x58b4d0`, `0x58b250`, `0x58a830`, `0xc5a480`, `0x86c318`, `0xc89f4c`, `0xc962f0`, `0xce8128`
Confidence: verified
Proof: one
Test: none

The loop is at `0x6bf380`. `0x6bf3a9` and `0x6bf4fc` set lighting (state 0xe) around the draw. The animated
block (`0x6bf3b2` to `0x6bf42e`) loads the record's floats, divides 1.0 by −1/sy and by the divisor,
scales the three and pushes (`0x6bf424`) and translates (`0x6bf42e` through `0x58b250`); `0x6bf511` pops
it. The program constants are 16 bytes a layer from `0xce8128` (layer 0 zero, layer 1 (1, 0, 0, 0),
layer 2 (0, 1, 0, 0), layer 3 (0, 0, 1, 0)), sent with the register in `0xc89f4c` or `0xc962f0`
(`0x6bf458`).

## Chunk layers in one pass

On the shader path one program draws every layer: each layer's texture sits on its own stage i with
the coordinates generated from the position (state 0x2f + i = 2, matrix enable 0x37 + i) and the matrix of the
multipass stage 0, translated for an animated layer in the same way; the combined texture takes the
stage after the last layer with the matrix of stage 1. A layer whose texture is missing sends the
chunk to the flat draw.

Addresses: `0x6bf690`, `0x6bf6a5`, `0x6bf6aa`, `0x6bf750`, `0x6bf832`, `0x6bf876`, `0x6bf914`, `0x6bf921`, `0x6bf92e`, `0x6bf93d`, `0x6bf94f`, `0x6bf95e`, `0x6bf9a9`, `0x6bf9cc`, `0x6bf9e9`, `0x6bf9fc`, `0x6bfa04`, `0x6bfa11`, `0x6bfa23`, `0x6bfa5d`, `0x6bfa66`, `0x6bfa8e`, `0x6bfabf`, `0x6bface`, `0x6bfaf0`, `0x58b210`, `0x58b220`, `0xc5a480`, `0x86c318`
Confidence: verified
Proof: one
Test: none

The blend is off (`0x6bf6a5`). `0x6bf921` makes the stage `0x17 + i` texture of layer i, `0x6bf92e` sets
its coordinate mode 2 and `0x6bf93d` the matrix enable; `0x6bf94f` pushes the stage matrix. The combined
texture goes to stage count (`0x6bf9e9`) with the second matrix (`0x6bf9fc`), the program is bound
(`0x6bfa23`) and the chunk is drawn once (`0x6bfa5d`); `0x6bfa66` to `0x6bfa8e` clear the stages from the
last to the first. A missing layer texture (`0x6bf91b`) pops the matrices pushed so far (`0x6bfabf`) and
calls `0x6bfaf0`.

## Chunk flat draw

`0x6bfaf0` draws a chunk untextured: white colour, no blend, no textures on stages 0 and 1, no program,
the chunk's own vertex and index buffers in format 1 and one draw.

Addresses: `0x6bfaf0`, `0x6bfafe`, `0x6bfb10`, `0x6bfb1c`, `0x6bfb28`, `0x6bfb34`, `0x6bfb44`, `0x6bfb5c`, `0x58a7c0`, `0x58a800`, `0x58a830`
Confidence: verified
Proof: one
Test: none

The sequence is colour 0xffffffff (`0x6bfafe`), state 7 = 0 (`0x6bfb10`), states 0x17 and 0x18 cleared
(`0x6bfb1c`, `0x6bfb28`), the program cleared (`0x6bfb34`), the buffers `+0x14c` and `+0x150` bound
(`0x6bfb44`) and the draw record the argument.

## Chunk draw pass

`0x684510` draws the frame's chunk list `0xc7cad8` in order. For each chunk it sets the transform to the chunk
position less the camera, picks the lights of the chunk as a whole, draws the mesh through `0x6beb50` and
resets the chunk's idle time. Lights are chosen once per chunk, at the centre of its box, with its bounding
radius as the search radius; nothing below the chunk looks at a light again.

Addresses: `0x684510`, `0x684564`, `0x68477f`, `0x68478a`, `0x68479c`, `0x6847a9`, `0x6847bb`, `0x6847c4`, `0x68460a`, `0x71bc30`, `0x70a0f0`, `0x6c0cc0`, `0x71c730`, `0x6beb50`, `0x6b0e50`, `0x6b10ec`, `0x6b113e`, `0xc7b298`, `0xc7cf20`
Confidence: verified
Proof: one
Test: none

`0x68477f` passes chunk +0x5c to the query builder `0x71bc30`, which copies four floats, so the search
radius is +0x68; `0x68479c` gathers the scene's lights around that point (`0x70a0f0` on `0xc7b298`), `0x6847a9`
runs the chunk's two light slots and `0x6847bb` sets the device lights relative to the camera `0xc7cf20`.
`0x6b0e50` stores the centre, half the sum of the box's corners, at +0x5c to +0x64 (`0x6b10ec`) and
the distance from it to the box's maximum, at +0x68 (`0x6b113e`): 23.570166 on flat ground, more
with height relief. The idle time +0xac is cleared at `0x68460a`.

## Chunk light slots

`0x6c0cc0` runs two slots of the chunk's class. Slot 1 (`0x6c31e0`) always sets the fog. Slot 2 (`0x6af9f0`)
adds lights, only when the chunk's view depth less twice its radius is under the fog end `0xc62964`
(`0x6c0cce`, `0x6c0cd4`). It clears bit 0 of the chunk's word +0xc, adds the global exterior light block
`0xc89ef0` + 0x7c, then each map light on the chunk's list (+0x100 stride, +0x108 head) whose object +0xdc is
not zero, adding the light embedded at +0x7c. So placed map lights and model lights compete for the
four nearest.

Addresses: `0x6c0cc0`, `0x6c0cce`, `0x6c0cd4`, `0x6c0cdf`, `0x6af9f0`, `0x6af9f7`, `0x6afa0a`, `0x6afa2c`, `0x6afa3c`, `0x6c31e0`, `0x71bf90`, `0x6c0920`, `0xc62964`, `0xc89ef0`
Confidence: verified
Proof: one
Test: none

`0x6c0cdf` is `jp` after `test ah,5`: it skips slot 2 for a depth at or beyond the fog end and for NaN.
`0x6af9f7` is `and byte [esi+0xc],0xfe`. `0x6afa0a` and `0x6afa3c` call the light inserter `0x71bf90`
for each; the map light's constructor `0x6c0920` embeds its light at +0x7c.

## Vertex formats

The device takes a vertex format number: `0x58a7c0` binds a vertex buffer under one of the 13
formats of the table at `0x809c00`. A record of 16 bytes holds the pointer to the element
descriptors, the element count, the stride and the usage mask. Bit 0 of the mask is the position (12
bytes), bit 3 the normal (12), bit 4 the colour (4), bit 5 the first texture coordinate (8) and bit 6
the second (8); bits 1 and 2 are 4 bytes each and occur only in format 12. The table lists the
formats the terrain code binds.

| format | mask | stride | elements | used by |
|---|---|---|---|---|
| 3 | 0x29 | 0x20 | position, normal, coordinate | exterior group batches |
| 4 | 0x39 | 0x24 | position, normal, colour, coordinate | interior group batches, exterior group water |
| 5 | 0x69 | 0x28 | position, normal, two coordinates | terrain liquid |
| 6 | 0x79 | 0x2c | position, normal, colour, two coordinates | footprints |
| 8 | 0x31 | 0x18 | position, colour, coordinate | interior group water, magma |

Addresses: `0x809c00`, `0x58a7c0`, `0x68d7a7`, `0x6b65e2`, `0x6b6850`, `0x69a748`, `0x6b4e0c`, `0x6b4e2d`
Confidence: verified
Proof: one
Test: none

The fills write the formats as they say: `0x6b6850` sets format 4 for the exterior group water,
`0x6b65e2` format 8 for the interior, `0x69a748` format 6 for the footprints and `0x6b4e0c` and
`0x6b4e2d` the group batch formats. The stride of a format is the sum of its element sizes. The other records are 0 (mask 0x01), 1 (0x09),
2 (0x19), 7 (0x11), 9 (0x71), 10 (0x21), 11 (0x61) and 12 (0x6f, six elements, stride 0x30).

## Depth bias

Device state 0 is the depth bias of the draws that lay a decal on the ground. The Direct3D arm,
which runs unless the `gxApi` variable is set to OpenGl, sets the render state `D3DRS_DEPTHBIAS`
(195) to minus the value, a constant in normalized depth with no slope term. The OpenGL arm sets a
polygon offset of factor −4.0 and units −65536 × value, enabled for a nonzero value only. The
footprints and the foam draw the value `footstepBias` (default 0.125) times 1/2048 and 1/1024.

Addresses: `0x5a2f00`, `0x5a2f30`, `0x5a2f38`, `0x5a2f48`, `0x5a2f51`, `0x59d383`, `0x59d38c`, `0x59d39f`, `0x59bef7`, `0x59befb`, `0x59bf0a`, `0x59bf0f`, `0x69a54a`, `0x68fd0f`, `0x63a3c1`, `0x80a000`, `0x8106d8`, `0x810390`
Confidence: verified
Proof: one
Test: none

The Direct3D arm gates on the device capability word `+0x2ec` (`0x5a2f30`), negates with `fchs`
(`0x5a2f48`) and calls `SetRenderState` with 0xc3 (`0x5a2f51`). The OpenGL arm tests the value
against 0 (`0x59d38c`, `test ah,0x44; jnp`) and enables the offset fill and calls the capability 0x41
(`0x59d39f`), whose arm multiplies by 65536.0 (`0x80a000`) and −1.0 and pushes 0xc0800000, the factor
(`0x59bf0a`, `glPolygonOffset` at `0x59bf0f`); the 65536 is a 16-bit depth step. At the default the Direct3D
bias of the foam is −0.00012207 and the footprints' half that. `0x63a3c1` makes the OpenGL arm the
choice only for the value OpenGl; its default string is `direct3d` (`0x864f7c`).

## Face culling

Terrain is one-sided. The device default for face culling (state 0x14) is on, the chunk pass never sets it,
and every chunk triangle faces up, so the ground is drawn from above and gone from below. The distant
low-detail tiles turn culling off and show from below. Culling removes the clockwise face: Direct3D
mode 2, and `glCullFace(GL_BACK)` with the default counter-clockwise front for OpenGL.

Addresses: `0x593bf0`, `0x593d58`, `0x591b76`, `0x59d7d8`, `0x5a320c`, `0x80a2a8`, `0x684510`, `0x6beb50`, `0x6b0e50`, `0x86bc90`, `0x86ba48`, `0x86b8c8`, `0x86b820`, `0x6bd9b0`, `0x6bd780`, `0x6bd798`, `0x6bd79d`, `0x6b0f35`, `0x6b0f43`
Confidence: verified
Proof: one
Test: none

`0x593d58` stores 1 for state 0x14 (`ecx` is loaded with 1 at `0x593d07`), and `0x591b76` gives the device
capability word 0x3ff, which includes the culling bit 0x20 both arms test (`0x59d7d8`, `0x5a320c`). The
OpenGL arm calls `glEnable` or `glDisable` on 0xb44; the Direct3D arm indexes `0x80a2a8` (1, 2) by the
value. The default `gxApi` is `direct3d`. No instruction of `0x684510` or the draw subtree from `0x6beb50`
writes state 0x14; the only literal 0x14 loads in the map render object file are `0x6bd3a3` and `0x6bd798`.
Vertex `r*17 + c` is the outer grid, `9 + r*17 + c` the centre of cell (r, c); `0x6b0e50` takes x from the row
and y from the column, each step the negated 33.333332/8 (`0x6b0f35`, `0x6b0f43`). With the slot-to-vertex
pairs of `0x86ba48` and `0x86b820`, all 256 triangles of `0x86bc90` and all 64 of `0x86b8c8` have a positive
upward normal; the first cell is (9, 0, 17), (9, 17, 18), (9, 18, 1), (9, 1, 0). `0x6bd9b0` emits
(C, v+1, v), (C, v+18, v+1), (C, v+17, v+18), (C, v, v+17), also upward. `0x6bd798` sets state 0x14 to 0 in
`0x6bd780`.

## Texture detail bias

The map textures (chunk layers and liquid frames) are made with linear filtering and the nearest mip level (filter index 3 of the Direct3D table `0x85c7d8`), repeat wrapping and the
anisotropy of the `anisotropic` setting. A setting above 1 also sets world flag 0x80000000, and the chunk pass
and the map object pass then set the sampler bias of stages 0 to 3 to 0.25; the scene driver resets all four
to 0. The detail doodad pass sets stage 0 to 0.25 whatever the setting.

Addresses: `0x689110`, `0x672650`, `0x6c4c20`, `0x85c7d8`, `0x68ab7b`, `0x684510`, `0x68451e`, `0x684525`, `0x684840`, `0x681070`, `0x681080`, `0xc6297c`, `0xc62978`, `0xc7b2a4`
Confidence: verified
Proof: one
Test: none

The `anisotropic` callback `0x689110` takes a whole number from 1 to 16 and refuses one above the device's
limit (the caps record +0xac) with a message; for a value above 1 it ors 0x80000000 into `0xc7b2a4`
(`0x68918b`) and says it takes effect on restart, else clears the bit (`0x6891ab`), and calls `0x672650`. That
stores the value and its floor log2 in `0xc6297c` and `0xc62978`. `0x6c4c20` passes `0xc6297c` as the anisotropy
of the flags `0x58a980` builds from (3, 1, 1, 0, 0, 0, anisotropy, 0); the liquid frame loader `0x68ab7b` does the same. `0x684510` begins with a
sign test of `0xc7b2a4` (`0x68451e`, `0x684525` jns) and sets states 0x27 to 0x2a to 0.25 only when the bit is
set. State 0x27 is the stage's sampler mip bias on the Direct3D arm.
