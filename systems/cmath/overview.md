# cmath: overview

The Tempest math library: the routines the engine's other files share for vectors, matrices,
quaternions, intersections and floats. Its code is in three stretches of the image: two small
helpers (`0x455c70` and `0x456330`), the software floats (`0x760be0` through `0x762e20`, derived in
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

Roots: `0x7bf680`, `0x7c0120`, `0x7c1450`, `0x7c2040`, `0x7c22b0`, `0x7c23e0`, `0x7c29f0`, `0x7c2c40`, `0x7c46d0`
