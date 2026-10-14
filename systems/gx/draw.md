# gx: drawing

How primitives reach the device: the draw call and its descriptor record, the vertex format table, the helper that builds an interleaved vertex buffer from separate arrays, and what each backend does between the last state change and the draw (lights, fill mode, attribute arrays). Render state ids are in `renderstate.md`.

Roots: `0x58a830`, `0x58a810`, `0x58a7c0`, `0x58a3d0`, `0x58a2a0`, `0x589d80`, `0x589de0`, `0x5a0000`

## Draw call

`0x58a810(descs, count, indexed)` loads the device and calls slot `+0x78` with the same arguments; `0x58a830(descs, indexed)` draws one record. Slot `+0x78` is `CGxDeviceOpenGl::PrimRender` (`0x59fc90`) or `CGxDeviceD3d::PrimRender` (`0x5a0f50`). Each record is 16 bytes: `+0` topology (0 points, 1 lines, 2 line strip, 3 triangles, 4 triangle strip, 5 triangle fan), `+4` first vertex or start index, `+8` u16 count, `+0xa` u16 minimum index, `+0xc` u16 maximum index, `+0xe` unused. Both refuse to draw and log `invalid buf!` unless every stream the attribute mask names, and for an indexed draw the index buffer at `[dev+0x27ec]`, is a created and valid buffer (bytes `+0x1c` and `+0x1d` non-zero).

Addresses: `0x58a810`, `0x58a830`, `0x59fc90`, `0x5a0f50`, `0x592b00`, `0x592c10`, `0x85c5c0`, `0x80a14c`, `0x85acf0`, `0x85ad08`
Confidence: verified
Proof: one
Test: none

The topology tables are `0x85c5c0` (OpenGL mode: 0, 1, 3, 4, 5, 6) and `0x80a14c` (`D3DPT`: 1 to 6). `0x592c10(topology, n)` is the primitive count `n / div - sub` with `div = 1, 2, 1, 3, 1, 1` and `sub = 0, 0, 1, 0, 2, 2`. OpenGL draws `glDrawArrays(mode, first, n)`, `glDrawRangeElements` when its entry point exists (`[0xc0f284]`) or `glDrawElements` with 16-bit indices at `[dev+0x3a4c] + first * 2`. Direct3D calls `DrawIndexedPrimitive` or `DrawPrimitive`; it never reads `+4` for a non-indexed draw: `StartVertex` and `BaseVertexIndex` are the stream 0 buffer's byte offset divided by its stride (`[dev+0x24c]` clear). `0x592b00` adds each record's primitive count, vertex bytes and index bytes to the counters at `[dev+0x7c]`, `+0x9c` and `+0xa0`, and the record count to `+0x80`.

## Vertex formats

Thirteen formats are 16-byte records at `0x809c00`: descriptor pointer, element count, stride, and a mask of components. Mask bit 0 is position (12 bytes), bit 3 normal (12), bit 4 colour (4, a packed dword), bit 5 first texture coordinate (8), bit 6 second (8); `terrain/render#vertex-formats` lists which formats the terrain code binds. `0x58a7c0(buffer, id)` binds a buffer with format `id`: it sets the attribute mask (`0x592a60`, which also sets the dirty mask `[dev+0x27dc] |= old ^ new`), the format id and the stride (`0x592aa0`).

| id | stride | components |
|---|---|---|
| 0 | `0xc` | position |
| 1, 2 | `0x18`, `0x1c` | position, normal; and colour |
| 3, 4 | `0x20`, `0x24` | position, normal, texcoord 0; and colour |
| 5, 6 | `0x28`, `0x2c` | position, normal, texcoords 0 and 1; and colour |
| 7 | `0x10` | position, colour |
| 8, 9 | `0x18`, `0x20` | position, colour, texcoord 0; and texcoord 1 |
| 10, 11 | `0x14`, `0x1c` | position, texcoord 0; and texcoord 1 |
| 12 | `0x30` | position, 4 bytes, 4 bytes, normal, texcoords 0 and 1 |

Addresses: `0x809c00`, `0x58a7c0`, `0x592a60`, `0x592aa0`, `0x592990`, `0x8097a8`
Confidence: verified
Proof: one
Test: none

`0x8097a8` holds each format's component byte offsets (13 columns, `-1` for absent): format 4 has position at 0, normal at 12, colour at 24, texcoord 0 at 28; format 12 has the two 4-byte components at 12 and 16, normal at 20, texcoords at 32 and 40. The descriptor arrays the first word points at are written by static initializers (`0x590640` through `0x590fc0`).

## Interleaving helper

`0x58a3d0(count, pos, posStride, normal, normalStride, colour, colourStride, tex0, tex0Stride, tex1, tex1Stride)` creates a vertex buffer of `count` vertices and fills it from strided arrays. It picks the format from which of normal, colour, texcoord 0 and texcoord 1 are non-null: none 0; normal 1; normal and colour 2; normal and tex0 3; normal, colour, tex0 4; normal, tex0, tex1 5; all four 6; colour 7; colour and tex0 8; colour, tex0, tex1 9; tex0 10; tex0 and tex1 11; texcoord 1 without texcoord 0 gives 1. Position and normal copy 12 bytes, texture coordinates 8. A colour copies as the dword, except that when the capability block's `+0x1c` equals 1 its bytes are reordered from `AARRGGBB` to `AABBGGRR`.

Addresses: `0x58a3d0`, `0x58a2a0`, `0x58a2e0`, `0x58a380`, `0x58a140`, `0x58a080`, `0x58a0a0`, `0x589a90`, `0x58a230`, `0x58a6a8`, `0x58a830`
Confidence: verified
Proof: one
Test: none

The buffer comes from `0x58a140(0, stride, count)`, is locked with `0x58a080`, unlocked with `0x58a0a0` and bound with `0x58a7c0`. `0x589a90(id)` returns the format's stride. `0x58a2a0` stores its count at `0xc0ed2c` before calling `0x58a3d0`; `0x58a2e0` and `0x58a380` build one descriptor record from values they keep and draw it through `0x58a830`.

## Colour byte order

The capability block's `+0x1c` (`[dev+0x258]`) is 1 on OpenGl and 0 on Direct3D, written by the two constructors. Eighteen sites call `0x58a230` and compare `+0x1c` with 1 (`0x58a6a8`, `0x5c887d`, `0x5c8b28`, `0x612124`, `0x675326`, `0x675412`, `0x67a103`, `0x67b576`, `0x690177`, `0x6b64e9`, `0x6b676b`, `0x6ba898`, `0x6c6648`, `0x7b2b46`) or load it (`0x61f90e`, `0x6b261c`, `0x6b264a`, `0x6d7594`). With 1, the packed colours they emit are in memory order red, green, blue, alpha; otherwise `0xAARRGGBB` passes through, which is `D3DCOLOR`.

Addresses: `0x58a230`, `0x598d31`, `0x58df39`, `0x58a6a8`, `0x61f90e`, `0x6b261c`, `0x6d7594`
Confidence: verified
Proof: one
Test: none

The census is every `call 0x58a230` followed within eight instructions by a read of `[reg+0x1c]`, 18 hits; its control is the known positive `0x58a6a8`. The flag written by `0x598d31` is 0 because `0x598ce0` clears it after the base constructor.

## Fixed lights

The device holds four light records (`+0x24fc`, 0x54 bytes each). `0x593330(index, light, offset)` copies a light into a record through `0x593040`, which sets one dirty bit per changed group (bit 1 enable, 2 position or direction, 4 ambient, 8 diffuse, 0x10 specular, 0x20, 0x40, 0x80 the three attenuation terms) and stores each colour as three floats `byte * (scale / 255)`, the product of the light's scalar and `1/255` formed at 53 bits before it multiplies the byte. When the light is positional (`flags & 2`) the offset is subtracted from the position. `0x593570` sets the enable word. `0x589d80` does both and fails for an index of 4 or more.

Addresses: `0x593330`, `0x593040`, `0x593570`, `0x589d80`, `0x589de0`, `0x589dc0`, `0x5933c0`
Confidence: verified
Proof: one
Test: none

`0x589dc0` has no caller, and its target `0x5933c0` (which rounds each float colour to a byte with `+0.5` and `__ftol`) is called only from `0x589dc0`. Light state is applied before each draw: Direct3D by `0x5a1b60` (`SetLight` slot `+0xcc`, `LightEnable` `+0xd4`), OpenGL by `0x59c820` (`glLightfv`).

## Lighting at draw time

Before each draw Direct3D runs `0x5a1b20`: it flushes render state (`0x594210`), then `0x5a1b60`, which decides lighting from id 14. Lighting on: `LIGHTING` is capability bit 0, `AMBIENT` id 6, and each dirty light is set and enabled. Lighting off with a vertex colour or id 1 equal to `0xffffffff`: `LIGHTING` 0 and `AMBIENT` id 6. Lighting off with no vertex colour and id 1 not white: all four lights disabled, `LIGHTING` 1, `AMBIENT` `0xffffffff`. Then `0x5a1dd0` sets `FILLMODE` to solid (3) or wireframe (2) from capability bit 9, `0x5a1e30` the diffuse and ambient material source to the vertex colour when the format has one and to the material otherwise, and `0x5a1ee0` the changed clip planes.

Addresses: `0x5a1b20`, `0x5a1b60`, `0x5a1dd0`, `0x5a1e30`, `0x5a1ee0`, `0x5a2e20`, `0x5a2eb0`, `0x5a24a0`
Confidence: verified
Proof: one
Test: none

`0x5a2e20` builds the `D3DMATERIAL9` from the id 1 to 4 colours: diffuse and ambient are id 1, emissive id 2, specular id 3, power id 4, each colour as `byte * (1/255)` with red at `+2`, and id 6, the global ambient, is only ever set to 0: the five setter facades have no call with the id 6 (a census of literal ids finds id 14 at 57 sites as control; 21 sites pass computed stage ids), and `0x593bf0` and `0x593e60` write 0. `COLORVERTEX`, `SPECULARMATERIALSOURCE` and `EMISSIVEMATERIALSOURCE` are written at no site: the set render state calls of the module push no `0x8d`, `0x92` or `0x94`.

## OpenGL draw preparation

Before each draw OpenGL runs `0x59c6d0`: it merges the dirty masks, flushes render state, then applies lights (`0x59c820`), the fill mode (`0x59c9f0`, `glPolygonMode` fill `0x1b02` or line `0x1b01` from capability bit 9), texture matrices, the viewport (`0x59c730`) and the attribute arrays (`0x59c100`). `0x59c100` sets the current colour with `glColor4fv` from id 1 when the format has no colour, then per component enables or disables the client array and passes the pointer (`glVertexPointer`, `glNormalPointer`, `glColorPointer`, `glTexCoordPointer`) at the buffer offset.

Addresses: `0x59c6d0`, `0x59c820`, `0x59c9f0`, `0x59c100`, `0x59c730`, `0x59ca60`, `0x59cdb0`, `0x59c610`
Confidence: verified
Proof: one
Test: none

`0x59c820` writes `GL_POSITION` (`0x1203`) as the point position or as the negated direction with w negated too, the attenuation terms (`0x1207` to `0x1209`) and the three colours (`0x1200` to `0x1202`) for `GL_LIGHT0` (`0x4000`) plus the index; the first change of a position loads the top view matrix through `0x5a0090`. The colour in `0x59c100` is the bytes of id 1 times `1/255` (`0x809ffc`) in the order red, green, blue, alpha of `0xAARRGGBB`.

## Projection matrix

`0x5a11d0` (Direct3D) and `0x59fe20` (OpenGL) take a pointer to a 4 by 4 float matrix, copy it, change it for the device and load it as the projection. Direct3D first stores the caller's matrix at `[dev+0xf50]` (`0x5925e0`), divides all sixteen elements by element 11 when that is neither 1 nor near 0, and rewrites elements 10 and 14 to the device depth range, in one form when element 15 is 0 and another when it is not. OpenGL negates elements 8 to 11, the third row. Both then multiply the matrix by a diagonal of `0.2, 0.2, 0.2, 1` when capability bit 8 of `[dev+0x269c]` is clear and element 15 is not 1.

Addresses: `0x5a11d0`, `0x59fe20`, `0x5925e0`, `0x7bc6a0`, `0x5a1450`, `0x592610`, `0x59baa0`, `0x58a230`
Confidence: verified
Proof: one
Test: none

Direct3D finishes with `SetTransform` (slot `+0xb0`) for projection (3). OpenGL sets matrix mode `0x1701` through `0x59baa0` and loads the matrix with `glLoadMatrixf`; when a render target is bound (`[dev+0x2848]` or `[dev+0x2854]` non-zero) and the capability block's `+0x90` is 0 it first multiplies elements 1, 5, 9 and 13 by -1. Each stores the final matrix at `[dev+0xf90]`. The view setter `0x5a1450` records its matrix with `0x592610` and calls `SetTransform` with 2 (view).

## Handedness flip

`0x5a0000(src, dst)` copies a 4 by 4 matrix and negates elements 2, 6, 10 and 14, the Z column of the row-major layout. OpenGL loads the view matrix through `0x5a0090`, which flips a copy with it and calls `glLoadMatrixf` in `GL_MODELVIEW`, and builds the texture matrix of coordinate mode 2 with it.

Addresses: `0x5a0000`, `0x5a0090`, `0x59d2de`
Confidence: verified
Proof: one
Test: none

The negations are `fchs` at `0x5a001c`, `0x5a003e`, `0x5a0058` and `0x5a0072`; every other element is copied unchanged.
