# lighting: shadows

The projected decals and the blob shadow: a texture laid onto the world geometry inside a box by
a texture transform. Units' blob shadows, selection circles, the auto-move target, footprints and
ripples all draw through it. The file follows the guild emblem file and ends before the minimap's.

Roots: `0x6d7330`, `0x6d6fa0`, `0x6d7480`, `0x6d78f0`, `0x6d8070`, `0x6d8420`, `0x6d6f90`

## Shadow settings

`shadowLOD`, 0 or 1 and 1 by default, turns the blob shadow on at 1; setting it to 1 also refills
the two ramp textures. `shadowBias`, 0 to 1 and 0.1 by default, is the depth bias a decal draws
with, at 2·bias/32768; at 0 the decal sets no bias. Values outside the range are refused with a
console message.

Addresses: `0x6d8070`, `0x6d8157`, `0x6d817d`, `0x6d8450`, `0x6d8476`, `0x6d847f`, `0x6d8492`, `0x6d84c4`, `0x6d8530`, `0x6d855f`, `0x6d856f`, `0x6d8589`, `0x86f5b4`, `0xcea620`, `0x811694`, `0x6d7506`, `0x6d7524`
Confidence: verified
Proof: one
Test: none

`0x6d8450` parses an integer, refuses it below 0 or above 1 (`0x6d8476`, `0x6d847f`), stores it
in `0x86f5b4` (`0x6d8492`) and at 1 refills `0xce9df8` and `0xce9df4` with their callbacks.
`0x6d8530` refuses a value below 0 (`0x6d855f`) or above 1 (`0x6d856f`) and stores `0xcea620`.
The decal draw doubles it and multiplies by `[0x811694]` (2⁻¹⁵) when it is not 0 (`0x6d7506`, `0x6d7524`).

## Shadow textures

The unit setup loads `Textures\ShadowBlob.blp` and builds two 64 × 8 procedural textures; the
unit shutdown releases all three. The first is white with a computed alpha; the second is a grey
ramp across the 64 columns: with f = 12·i/63, the darkness is f/2 below 2, 1 from 2 to 10 and
(12 − f)/2 above, and a texel's alpha is 0 where it is white.

Addresses: `0x6d8070`, `0x6d80c1`, `0x6d80fe`, `0x6d8136`, `0x6d8420`, `0x6d81a0`, `0x6d82d0`, `0x6038e9`, `0x605283`, `0xce9dfc`, `0xce9df8`, `0xce9df4`, `0x80c4e8`
Confidence: inferred
Proof: none
Test: none

`0x6038e9` and `0x605283` call `0x6d8070` and `0x6d8420`. `0x6d80c1` creates `0xce9dfc`; `0x6d80fe`
and `0x6d8136` create 64 × 8 textures with `0x6d81a0` and `0x6d82d0`. The ramp's breakpoints are
read from the decompiled body of `0x6d82d0`; the first texture's alpha source is not read.
`0x6d6f90` returns `0xce9df8` to the footprints.

## Projected decals

A decal is a box, a colour and a texture. The box gives two texture transforms: one maps the box's
horizontal extent onto [0, 1] after a quarter turn, the other its height onto a fade coordinate.
The draw tests the box against the view, sets the texture, the bias and the colour, and draws
every collected world batch with the transform, as white with the colour's alpha or with the
colour itself.

Addresses: `0x6d7330`, `0x6d6fa0`, `0x6d7480`, `0x6d7760`, `0x6d7810`, `0x6d78a0`, `0x608f28`, `0x611575`, `0x69014b`, `0x69a0a9`, `0x672b48`, `0x48395a`, `0x5d7105`, `0x811668`, `0x803068`
Confidence: inferred
Proof: none
Test: none

`0x6d7330` calls `0x6d6fa0` and `0x6d7480` between two transform pushes. `0x6d6fa0` turns by
`[0x811668]`·`[0x803068]` (−π/2). `0x6d7760` writes positions with the colour `0xce9dc8`,
`0x6d7810` positions with one packed colour, `0x6d78a0` a running index list. The callers are the
selection circle `0x608f28`, the auto-move target `0x611575`, ripples `0x69014b`, footprints
`0x69a0a9` and world events `0x672b48`. The batch list and the transform layout are read from the
decompiled bodies, not settled at the bytes.

## Blob shadow

A doodad model drawn with model flag 0x400 and without 0x8 casts a blob shadow when `shadowLOD`
is 1 and the model is loaded. The shadow is the blob texture projected over the model's first
sequence's bounds, each clamped to ±5, stretched down by 5/3 and up by 1 of a scale-derived
length, in white with the model's alpha.

Addresses: `0x683eb9`, `0x683ebe`, `0x683ec3`, `0x6d78f0`, `0x6d78fb`, `0x6d7904`, `0x6d790f`, `0x6d7920`, `0x6d6e20`, `0x6d6e50`, `0xcea610`, `0xcea60c`
Confidence: inferred
Proof: none
Test: none

`0x683ec3` calls `0x6d78f0` after testing the model's `+0x90` for 0x8 and 0x400. `0x6d78f0`
needs `0x710450` ready (`0x6d78fb`) and `[0x86f5b4]` equal to 1 (`0x6d7904`). The initializer
`0x6d6e20` stores 5·(1/3) in `0xcea610` and 3·(1/3) in `0xcea60c` through `0x6d6e50`. The box,
scale and colour steps of `0x6d7920` are read from its decompiled body.
