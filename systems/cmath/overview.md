# cmath: overview

The Tempest math library: the routines the engine's other files share for vectors, matrices,
quaternions, intersections and floats. Its code is in four stretches of the image: two small
helpers (`0x455c70` and `0x456330`), the software floats (`0x760be0` through `0x762e20`, derived in
`spell/records#software-floats`), and the geometry file (`0x7bc090` through `0x7c4810`), whose
quaternion functions are in `cmath/quaternions` and whose intersection and distance routines are in
`cmath/geometry`. The geometry file's matrix routines (`0x7bc6a0` and the rest of `0x7bc090` to `0x7bf630`)
are not read in a doc yet. The file begins with initializers that store 2π, 1/2π and +∞, and its
`Tempest\cmemblock.cpp` code (`0x7bfc30` through `0x7c00f0`) lies between the matrices and the
quaternions; the geometry routines below `0x7c0120` belong to the same run of initializers
(`0x82da18` to `0x82dab0`), which is the evidence for their ownership. A second small group of
intersection routines at `0x6dc2c0` belongs to a file whose ownership is not settled, and its
rows stay unassigned. Most of the library is not called: the live routines are the matrix
conversion, the ray, plane, triangle and circle tests and the point to segment distance.

Roots: `0x7bf680`, `0x7c0120`, `0x7c1450`, `0x7c2040`, `0x7c22b0`, `0x7c23e0`, `0x7c29f0`, `0x7c2c40`, `0x7c46d0`
