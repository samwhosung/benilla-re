# cmath: value types

The value types the Tempest math library's routines take: vectors, matrices, a quaternion, boxes,
spheres, planes, facets, rays, segments, cones and colours. Each is a packed run of floats (a
colour is four bytes) with no vtable, and each layout is read from the routines that build and use
it. A matrix is stored row by row for a row vector on the left, v·M, so its last row holds the
translation. Five types are named by the image itself, through the type names its growable arrays
pass as allocation tags; the others take the names a later build's public symbols give the same
layouts, in the `NTempest` namespace the image gives the library's own types, or descriptive
names (`../../symbols/README.md`, Names).

Roots: none

## Type names

The image holds the RTTI names `.?AVC2Vector@NTempest@@` (`0x85ab48`), `.?AVC3Vector@NTempest@@`
(`0x82f230`), `.?AVC4Plane@NTempest@@` (`0x860944`), `.?AUCFacet@NTempest@@` (`0x85005c`) and
`.?AVCImVector@NTempest@@` (`0x84f09c`), each passed as the tag of array allocations and frees. The
arrays tie three names to sizes: `NTempest::C3Vector` elements are 12 bytes, `NTempest::C2Vector`
elements 8 bytes, and `NTempest::CFacet` elements 52 bytes with three 12-byte vectors from `+0x10`.
`NTempest::C4Plane` is tagged only at a free. `NTempest::CImVector` is also the class of the
community database's `0x4a2c10`, which stores one dword. The other types are not named in the image.

Addresses: `0x85ab48`, `0x82f230`, `0x860944`, `0x85005c`, `0x84f09c`, `0x453519`, `0x690933`, `0x690986`, `0x674388`, `0x67436d`, `0x5f5cd5`, `0x77f6f0`, `0x4a2c10`, `0x4a2c2d`
Confidence: verified
Proof: one
Test: none

`0x453519` reallocates the spline's control points with the size `12n` and the tag `0x82f230`.
`0x690933` sizes the array of `0x6908c0` as `lea eax,[esi*8]` under the tag `0x85ab48`, and its
copy loop moves two dwords per element (`0x690986`). `0x674388 imul edi,edi,0x34` sizes the array
of `0x6742d0` under the tag `0x85005c`, and for each dropped element `0x67436d` passes the address
`+0x10`, the size 12 and the count 3 to `__ehvec_dtor`. `0x5f5cd5` frees the array at `+0x50` of the
transport handler under `0x860944`; `0x77f6f0` frees a region's colour array under `0x84f09c`.
`0x4a2c10` packs its four byte arguments into one dword and stores it at `ecx` (`0x4a2c2d`). Each
name is preceded by the eight bytes of a type descriptor. A scan of the image for `@NTempest@@`
finds four more names, `CRect`, `CiRect`, `C2iVector` and `C3iVector`, which no math routine takes.
`NTempest::CRect` and `NTempest::CiRect`, the rects, have no struct row yet.

## Vectors

`NTempest::C2Vector` is two floats, x then y. `NTempest::C3Vector` is three floats, x, y, z.
`NTempest::C4Vector` is four floats, x, y, z, w, where w multiplies row 3 of a matrix. Vertex lists are arrays
of `NTempest::C3Vector` with a stride of 12.

Addresses: `0x6dc2c0`, `0x7c3d20`, `0x7bf680`, `0x7bf700`, `0x7c1450`, `0x7bcb40`, `0x7bcb4c`
Confidence: verified
Proof: one
Test: none

`0x6dc2c0` and `0x7c3d20` read `+0` and `+4` of each of their four points and write two floats. `0x7bf680` and `0x7bf700`
return 0, 1 or 2 for the components at `+0`, `+4` and `+8`. `0x7c1450` steps through its points by 12. `0x7bcb4c` multiplies
the float at `+0xc` of the vector by the row at `+0x30` of the matrix. The wowdev wiki documents `C4Vector` as the floats
x, y, z, w; the name is from a later build.

## Matrices

`NTempest::C44Matrix` is 16 floats, row by row: entry (r, c) at float `4r + c`. A point is a row vector on the
left, v·M, `x' = x·m0 + y·m4 + z·m8 + m12`, so row 3 holds the translation. `NTempest::C33Matrix`
is nine floats, entry (r, c) at `3r + c`. `NTempest::C34Matrix` is a `NTempest::C33Matrix` followed by a translation row
at `+0x24`. `Matrix2x2` is four floats, row by row; its one builder `0x7c42f0` writes the rotation
`[[cos, sin], [−sin, cos]]`.

Addresses: `0x7bca80`, `0x7bca9e`, `0x7bc6a0`, `0x7bdfc0`, `0x7bb0e0`, `0x7bae60`, `0x7bcae0`, `0x674c50`, `0x674f45`, `0x674f54`, `0x7c42f0`
Confidence: verified
Proof: one
Test: none

`0x7bca80` adds `[m+0x38]` to the z output (`0x7bca9e`) after the three products of column 2, and `[m+0x30]` and
`[m+0x34]` to x and y. `0x7bc6a0` stores `out[4i + j]` as row `i` of `a` by column `j` of `b`, and `0x7bdfc0` does
the same for `3i + j`. `0x7bb0e0` and `0x7bae60` add the translation from `+0x24`. `0x7bcae0` takes a `NTempest::C44Matrix`
and reads its first three rows as rows of four, the product with the transpose: `0x674c50` builds an identity
4×4, rotates it with `0x7bdd60` (`0x674f45`) and passes it (`0x674f54`). The wowdev wiki documents `C44Matrix` as
four `C4Vector` columns, column-major, which is the same 16 floats with the translation at floats 12 to 14, and
`NTempest::C33Matrix` and `NTempest::C34Matrix` as three and four `C3Vector`s; those names are from a later build. No public name
for the 2×2 is known, so its name is descriptive.

## Quaternions

`NTempest::C4Quaternion` is four floats: the vector part x, y, z, then the scalar w. `0x7c0190`
forms `x = (m21 − m12)·k`, the formula for a matrix acting on column vectors; its one caller,
`0x7c0120`, transposes the stored `NTempest::C33Matrix` first, so the pair gives
`x = (m12 − m21)·k` of the row-vector matrix, and y and z alike.

Addresses: `0x7c0120`, `0x7c0129`, `0x7c0158`, `0x7c0162`, `0x7c0190`, `0x7c01bf`, `0x7c01d0`, `0x7c1340`, `0x7c135e`, `0x7c03b0`
Confidence: verified
Proof: one
Test: none

In the trace leg of `0x7c0190` the scalar `0.5·√(t + 1)` is stored at `+0xc` (`0x7c01bf`) and `(m21 − m12)·k` at
`+0` (`0x7c01d0`). `0x7c1340` negates `+0`, `+4` and `+8` and copies `+0xc` (`0x7c135e`), and `0x7c03b0` writes the
identity as 1.0 at `+0xc`. `0x7c0120` copies `+0`, `+0xc`, `+0x18`, `+4`, `+0x10`, `+0x1c`, `+8`, `+0x14`
and `+0x20` into a local in that order (`0x7c0129` to `0x7c0158`) and passes it (`0x7c0162`); the census of
`0x7c0190` finds no other call and no stored address. The wowdev wiki documents `C4Quaternion` with the scalar last; the name is from a later
build.

## Boxes, spheres and planes

`NTempest::CAaBox` is the minimum corner then the maximum. `NTempest::CAaSphere` is the centre then the radius.
`NTempest::C4Plane` is the unit normal `n` then `d`, with `n·p + d = 0` for a point `p` on the plane.
`NTempest::CFacet` is a plane followed by the three vertices it was built from, at `+0x10`, `+0x1c`
and `+0x28`.

Addresses: `0x7c1450`, `0x7c1560`, `0x6992c0`, `0x7c1920`, `0x7c1af0`, `0x7c1af9`, `0x7c2040`, `0x7c22b0`, `0x7c22f1`, `0x7bf860`, `0x7bf870`, `0x7bf888`, `0x7bf89e`, `0x7bf96d`, `0x7c2740`, `0x7c2786`
Confidence: verified
Proof: one
Test: none

`0x7c1450` folds the points into two local triples, the second through the maximum `0x6992c0`, and copies the
six dwords, minimum first, to the box (`0x7c1560`); `0x7c1920` tests a coordinate against `+0` before `+0xc`.
`0x7c1af0` loads the radius from `+0xc` (`0x7c1af9`), as `0x7c2040` does. `0x7c22b0` adds `+0xc` of the plane to
`origin·n` (`0x7c22f1`). `0x7bf860` copies its three points to `+0x10`, `+0x1c` and `+0x28` (`0x7bf870`, `0x7bf888`,
`0x7bf89e`) and stores `d = −(n·p0)` at `+0xc` (`0x7bf96d`); `0x7c2740` passes the facet to `0x7c22b0` as a plane and
`+0x10` to `0x7c23e0` as three vertices (`0x7c2786`). The 52-byte facet arrays (Type names) match. The wowdev wiki
documents `CAaBox`, `CAaSphere` and `C4Plane` with these layouts; those names are from a later build. The image names
`C4Plane` but ties no layout to it, so its match with this layout rests on that documentation.

## Rays and segments

`NTempest::C3Ray` is an origin then a direction. `NTempest::C3Segment` is a start point then an end point. The line
routines take a record of the ray's layout, an origin then a direction.

Addresses: `0x7c22b0`, `0x7c22c6`, `0x7c29f0`, `0x7c46d0`, `0x7c46f2`, `0x7c4630`
Confidence: verified
Proof: one
Test: none

`0x7c22b0` multiplies the plane normal by `+0xc` of the ray (`0x7c22c6`), and `0x7c29f0` takes `+0xc` as the
direction it crosses with an edge. `0x7c46d0` subtracts `+0` of the segment from the point and reads the end from
`+0xc` (`0x7c46f2` loads its z). The wowdev wiki documents `C3Ray` and `C3Segment` with these layouts; the names are
from a later build.

## Cones and oriented boxes

`Cone` is the apex at `+0`, the unit axis at `+0x10`, the cap length at `+0x1c` and the cosine of
the half-angle at `+0x20`; nothing reads `+0xc`. `OrientedBox` is the centre at `+0`, the
half-extents at `+0xc` and three unit axes at `+0x18`, `+0x24` and `+0x30`. Both are taken only by
unreached routines, no allocation or constructor gives their sizes, and the sizes 36 and 60 are the
extents the routines read. No public names for them are known, so their names are descriptive.

Addresses: `0x7c32f0`, `0x7c3390`, `0x7c33ca`, `0x7c33ff`, `0x7c3780`
Confidence: verified
Proof: one
Test: none

`0x7c3390` dots the direction with `+0x10` to `+0x18` (from `0x7c33ca`) and squares `+0x20` (`0x7c33ff`); it and
`0x7c32f0` read `+0` to `+8`, `+0x10` to `+0x20` and nothing else. `0x7c3780` reads `+0` to `+0x38` of both boxes.
The meanings are in `cmath/solids#ray-and-cone` and `cmath/solids#box-against-box`.

## Colours

`NTempest::CImVector` is four bytes: blue, green, red, alpha. A float colour is a
`NTempest::C3Vector` of red, green and blue.

Addresses: `0x7bbec0`, `0x7bbec8`, `0x7bbee3`, `0x7bbf19`, `0x7bbf20`, `0x78ed10`, `0x78edeb`, `0x78ee08`, `0x78ee25`, `0x7bbc80`
Confidence: verified
Proof: one
Test: none

`0x7bbec0` stores 0xff at `+3` (`0x7bbec8`) and channels 0, 1 and 2 of the float colour at `+2`, `+1` and `+0`
(`0x7bbee3`, `0x7bbf19`); `0x7bbf20` reverses it. `ColorSelect:GetColorRGB` (`0x78ed10`) returns the bytes at `+2`
(`0x78edeb`), `+1` (`0x78ee08`) and `+0` (`0x78ee25`) in that order as red, green and blue. `0x7bbc80` passes the
float colour to `0x7bf680` and `0x7bf700` as a `NTempest::C3Vector`.
