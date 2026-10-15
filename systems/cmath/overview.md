# cmath: overview

The Tempest math library: the routines the engine's other files share for vectors, matrices,
quaternions, intersections and floats. Its code is in three stretches of the image: the scalar
file (`0x455c00` through `0x4567c2`, `cmath/scalars`), the software floats (`0x760be0` through `0x762e20`, derived in
`spell/records#software-floats`), and twelve object files from `0x7bc090` through `0x7c4810`, whose
initializers the C runtime's run `0x82da20` to `0x82dab0` lists in groups of +∞, 2π and 1/2π
(`cmath/geometry#static-constants`). They hold the matrix routines (not yet in a doc), the
quaternions (`cmath/quaternions`), the rays, points and segments (`cmath/geometry`) and the tests of
boxes, cones and triangles (`cmath/solids`). The run gives a file's initializers, not its extent, so
no routine is placed in a file here. Four routines of a second small group, `0x6dc2c0` through
`0x6dc900`, have the same shape and are in `cmath/solids`; their rows are not assigned to a system.
Most of the library is not called. The live routines are the dominant axis, the bounds, the matrix to
quaternion conversion, the ray and plane and ray and triangle tests, the polygon test, the circle and
box test and the point to segment distance.

Roots: `0x455c70`, `0x7bf680`, `0x7c0120`, `0x7c1450`, `0x7c2040`, `0x7c22b0`, `0x7c23e0`, `0x7c29f0`, `0x7c2c40`, `0x7c46d0`

## High product

`0x455c70` returns in `eax` the high 32 bits of the unsigned 64-bit product of `ecx` and `edx`; it
takes no stack arguments and returns with a plain `ret`. It is the plain high-half multiply
`(a·b) >> 32`, called from the random library, the sound code and other systems.

Addresses: `0x455c70`, `0x455c72`, `0x455c74`
Confidence: verified
Proof: one
Test: `cmath::mul_high32_difftest`

`0x455c70` is `mov eax,ecx; mul edx; mov eax,edx; ret`, so the product is unsigned (`mul`) and the
low dword is dropped. It has 17 call sites: `0x4501fb`, `0x450339`, `0x450478`, `0x4505a8`, `0x4506d8`
in the random library, `0x458d45`, `0x458e95`, `0x45bcc8` and `0x4601de` in the sound code, and
eight more in other systems, one of them in the unreached `0x455c80`.
The test runs the whole function over the zeros, the extremes, each power of two and a step
either side, and a fuzz of pairs.

