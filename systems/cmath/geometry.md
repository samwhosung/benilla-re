# cmath: intersections

The intersection and distance routines of the Tempest math library in the object files of `0x7bf680`
through `0x7c4800`. Boxes, cones and triangles against rays, points and boxes are in `cmath/solids`. Vectors
are three floats and a ray is an origin followed by a direction (`+0` and `+0xc`). A routine returns a byte
(0 or 1) and writes what it found through optional pointers. Eight of the routines here are called by live
code (`0x7bf680`, `0x7c1450`, `0x7c2040`, `0x7c22b0`, `0x7c23e0`, `0x7c29f0`, `0x7c2c40`, `0x7c46d0`); the
rest have no stored address and no caller that code reaches (a relative-call scan and the census of stored addresses). Their callers are other routines of the same kind:
`0x7c4150` is called from `0x7c3ebf` in the unreached `0x7c3e30`, `0x7c4510` from `0x7c442c` in the unreached `0x7c43d0`, and `0x7c42f0` from `0x68c17e` in the unreached `0x68c0f0`.

Roots: `0x7bf680`, `0x7c1450`, `0x7c2040`, `0x7c22b0`, `0x7c23e0`, `0x7c29f0`, `0x7c2c40`, `0x7c46d0`

## Dominant axis and bounds

`0x7bf680` returns the index of the largest component of the vector in `ecx` by magnitude: 0 only when
`|x|` is strictly greater than both `|y|` and `|z|`; otherwise 1 when `|y|` is strictly greater than
`|z|`, else 2, so a tie goes to the later axis. `0x7c1450(n, points)` writes the bounds of `n` points
(stride 12) to the six floats at `ecx`, minimum then maximum; for `n` of 0 all six are 0. It starts
from the first point and folds each next one with a per-axis compare, the maximum through `0x6992c0`. `0x7c16b0` does the same for the points an
index list names.

Addresses: `0x7bf680`, `0x7bf69a`, `0x7bf69f`, `0x7bf6a2`, `0x7bf6a4`, `0x7bf6ac`, `0x7bf6b4`, `0x7bf6b9`, `0x7bf6c1`, `0x7bf6c6`, `0x7bf6c8`, `0x7c1450`, `0x7c145e`, `0x7c1463`, `0x7c14b6`, `0x7c1513`, `0x7c1531`, `0x7c1560`, `0x7c16b0`, `0x7c173f`, `0x7c1759`, `0x7c1770`, `0x7c1839`, `0x6992c0`, `0x4549a0`
Confidence: verified
Proof: one
Test: `cmath::dominant_axis_difftest`

`0x7bf69a fcom [ebp-4]` compares `|x|` with `|y|`, `test ah,0x41; jne 0x7bf6b4` takes `|x|` at most `|y|` or NaN to
the second test; the first test's `fcomp [ebp-8]` with `jne 0x7bf6c8` returns 2 for `|x|` at most `|z|`.
At `0x7bf6b9` the compare of `|y|` with `|z|` and `mov eax,1; je 0x7bf6cd` return 1 only for `|y|` greater. The
bounds routine's loop runs `n − 1` times over the points from the second (`0x7c14b3 lea ebx,[eax-1]`). `0x7c16b0(vertices)` takes the output box in `ecx` and the
index list in `edx` (a pointer at `+0xc`, a count at `+0x1c`, 32-bit indices); it stores zeros for an empty list, otherwise seeds from the first index and folds the rest with
the same compares (`0x7c173f`, `0x7c1759`, `0x7c1770`), and copies six dwords out at `0x7c1839`.

## Ray and plane

`0x7c22b0(outT, outPoint, eps)` intersects the ray in `ecx` with the plane in `edx` (normal at `+0`,
constant `d` at `+0xc`). `D = (dir.y·n.y + dir.z·n.z) + n.x·dir.x`. For `|D| < 1e-4` (a double at `0x811658`)
the ray is parallel: it hits at `t = 0`, point = origin, only when `|origin·n + d| < eps`, else misses.
Otherwise `N = origin·n + d`, `t = −N/D` when `|N| ≥ eps` or NaN, else `t = 0`, and the point is
`origin + t·dir`, whose x and y terms are rounded to a float before the add and whose z term is not.
Both outputs are optional; with both null a non-parallel ray reports a hit.

Addresses: `0x7c22b0`, `0x7c22cb`, `0x7c22d0`, `0x7c22db`, `0x7c22f4`, `0x7c22f6`, `0x7c22fe`, `0x7c2338`, `0x7c2344`, `0x7c2360`, `0x7c2367`, `0x7c236f`, `0x7c2373`, `0x7c237b`, `0x7c2384`, `0x7c238d`, `0x7c23a1`, `0x7c23b5`, `0x811658`, `0x7ffd74`
Confidence: verified
Proof: one
Test: `cmath::ray_plane_difftest`, `cmath::raymesh_composed_difftest`

`0x7c22d0 fcomp qword [0x811658]; test ah,5; jp 0x7c2338` goes to the non-parallel leg for `|D|` at least
1e-4 (even parity covers greater, equal and NaN). In the parallel leg `0x7c22f6 fcomp [ebp+0x10]` with `jp
0x7c2330` returns 0 for `|N|` at least `eps` or NaN. `0x7c237b fdiv [ebp-4]; fchs` forms the root from the
stored `D`; `0x7c2384 fst` rounds the stored `t` while `st0` keeps the unrounded value for the point, whose x and y
terms are stored to `[ebp-0x1c]` and `[ebp-0x18]` as floats and whose z term is added at 53 bits (`0x7c23b5`).

## Point in polygon

`0x7c23e0(n, axis)` tests the point in `ecx` against `n` vertices (flat floats, stride 12, in `edx`)
by the crossing count after dropping coordinate `axis`: the two kept coordinates are `A = tab[axis].a`
and `B = tab[axis].b` from the table `0xcf5c7c`, set by the initializer `0x7c18d0` to (2, 1) for axis 0,
(0, 2) for axis 1 and (1, 0) for axis 2. For each edge from the previous vertex (the last for the first)
to the current, with `side(V) = V.A ≥ Q.A` (false for NaN): a change of side compares
`(Vprev.B − Vcur.B)·(Vcur.A − Q.A)` with `(Vcur.B − Q.B)·(Vprev.A − Vcur.A)`, and the accumulator toggles when
`term2 ≤ term1` equals the current side. `n` of 0 gives 0.

Addresses: `0x7c23e0`, `0x7c23e9`, `0x7c2409`, `0x7c241c`, `0x7c2443`, `0x7c2448`, `0x7c245e`, `0x7c2499`, `0x7c24a0`, `0x7c24ab`, `0x7c24ed`, `0xcf5c7c`, `0xcf5c80`, `0x7c18d0`, `0x7c18c0`, `0x7c24f0`, `0x7c2620`, `0x7c2740`, `0x7c2773`, `0x7c277e`, `0x7c278c`
Confidence: verified
Proof: one
Test: `cmath::point_in_poly_difftest`, `cmath::raymesh_composed_difftest`

`0x7c24f0` and `0x7c2620` are the same test over 16-bit and 32-bit index lists. `0x7c2740` intersects a
ray with a facet record (plane, then three vertices at `+0x10`): `0x7c22b0`, then `0x7bf680` of the
normal, then `0x7c23e0` with `n` 3.

`0x7c23e9 mov eax,[eax*8+0xcf5c80]` reads the first kept axis and `0x7c245e` the second, from the dwords at
`0xcf5c7c`; `0x7c18d0` stores 2, 2, 1, 0, 0, 1 in address order from `0xcf5c80`. `0x7c2499 fcompp` and `test ah,0x41;
jp` give `term2 ≤ term1` as a clear parity (NaN false). The 32-bit routine `0x7c2620` differs from `0x7c24f0` by index
width only (a normalized diff shows `movzx` against `mov` and the stride). The wrapper passes `edx+0x10` and 3
(`0x7c2784`, `0x7c2786`).

## Ray and triangle

`0x7c29f0(indices, outT, outUV, slack)` intersects the ray in `ecx` with the triangle whose 16-bit vertex indices
are at `[ebp+8]` into the vertex array in `edx`, both faces, by Möller–Trumbore. `e1 = v1 − v0`, `e2 = v2 − v0`,
`p = dir × e2`, `det = e1·p`; it misses for `−1e-6 < det < 1e-6`. With `s = origin − v0`: `u = (s·p)/det`,
`q = s × e1`, `v = (dir·q)/det`, and it hits when `u ≥ −slack`, `v ≥ −slack` and `u + v ≤ 1 + slack`, where the
caller gives `slack`. `t = (e2·q)/det` goes to `outT` and `(u, v)` to `outUV` when non-null; `t` is not
range-checked. `0x7c2c40` is the same for 32-bit indices.

Addresses: `0x7c29f0`, `0x7c2a48`, `0x7c2a81`, `0x7c2ac1`, `0x7c2ada`, `0x7c2adc`, `0x7c2ae7`, `0x7c2ae9`, `0x7c2af4`, `0x7c2b03`, `0x7c2b0c`, `0x7c2b55`, `0x7c2b58`, `0x7c2b5b`, `0x7c2b63`, `0x7c2b68`, `0x7c2b70`, `0x7c2bcb`, `0x7c2bce`, `0x7c2bd6`, `0x7c2be2`, `0x7c2bea`, `0x7c2bf0`, `0x7c2c10`, `0x7c2c2c`, `0x7c2c40`, `0x81d9bc`, `0x80e2e4`
Confidence: verified
Proof: one
Test: `cmath::intersect_difftest`

`0x7c2adc fcom [0x81d9bc]` (−1e-6) with `test ah,0x41; jne 0x7c2b03` continues for `det` at most −1e-6 or NaN,
then `0x7c2ae9 fcom [0x80e2e4]` (+1e-6) with `test ah,5; jp 0x7c2b03` continues for `det` at least 1e-6;
anything else returns 0. `u` is stored as a float at `0x7c2b58 fst`, with the unrounded value compared first
(`0x7c2b5b`, bound `[ebp-8] = −slack`) and the stored float second (`0x7c2b68`, `[ebp-0xc] = 1 + slack`). `v` is compared once, unrounded
(`0x7c2bcb`, `0x7c2bd6`), and `u + v` adds the stored `v` to the stored `u` (`0x7c2bdf`). The twin `0x7c2c40` differs by `mov` against
`movzx` and the index strides. Callers pass the slack: 0.002 (`0x6b8acf`, `0x6bc764`), 1e-6 (`0x709fc5`), 0.01 (`0x69cbca`, `0x69ccc0`, `0x6baf51`) and 0 (`0x6aa81d`).

## Other triangle rays

`0x7c27d0(outT, outUV, slack)` is `0x7c29f0` for a triangle of three stored vertices (nine floats at `edx`) with the
same two-sided band of `±1e-6`; it takes the reciprocal of `det` before the bounds. `0x7c2e90(outT, outUV)` takes the same
triangle and one face: it misses for `det < 1e-6`, tests the numerators `u·det` and `v·det` (`u ≥ 0`, `v ≥ 0`, `u + v ≤ det`) before it
divides, and divides once, only for an output. `0x7c30a0(indices, outT, outUV)` is `0x7c2e90` with the triangle given by
three 16-bit indices into the vertex array in `edx`. None has a caller.

Addresses: `0x7c27d0`, `0x7c2894`, `0x7c28a1`, `0x7c28ba`, `0x7c2909`, `0x7c2916`, `0x7c297c`, `0x7c2990`, `0x7c29d5`, `0x7c2e90`, `0x7c2f43`, `0x7c2f93`, `0x7c2fa7`, `0x7c3037`, `0x7c3041`, `0x7c30a0`, `0x7c317b`, `0x4549a0`, `0x80e2e4`, `0x81d9bc`, `0x7ffd74`
Confidence: verified
Proof: one
Test: none

`0x7c27d0` forms `−slack` at `[ebp-0xc]` and `1 + slack` at `[ebp-0x10]`, and `0x7c2894 fcom [0x81d9bc]` with `0x7c28a1 fcom [0x80e2e4]` repeat the
band of `0x7c29f0`; `u` and `v` are scaled by the reciprocal (`0x7c28ba fdiv`) and compared at `0x7c2909`, `0x7c2916`, `0x7c297c` and `0x7c2990`, and `t` is `(e2·q)·inv`
without a range check. In `0x7c2e90`, `0x7c2f43 fcomp [0x80e2e4]; test ah,5; jnp 0x7c3093` misses below the bound, `0x7c2f93` and `0x7c2fa7` bound `u·det`, the
equivalent for `v` follows `0x7c3014`, and with both outputs null `0x7c3037` returns 1 with no divide. `0x7c30a0` differs in the index loads and
`ret 0xc`; its determinant test is `0x7c317b`.

## Ray and sphere

`0x7c1af0(outT, outPoint)` intersects the ray in `ecx` (direction taken as a unit vector) with the sphere in
`edx` (centre at `+0`, radius at `+0xc`). `L = centre − origin`, `tca = L·dir`, `L² = L·L`. It misses when
`tca < 0` and `L² > r²`, and when `L² − tca² > r²`. Otherwise `thc = √(r² − (L² − tca²))` and `t = tca − thc`
for an origin outside the sphere (`L² > r²`), `t = tca + thc` inside or on it. The point is `origin + t·dir`
with every term rounded to a float.

Addresses: `0x7c1af0`, `0x7c1b3d`, `0x7c1b56`, `0x7c1b6b`, `0x7c1b70`, `0x7c1b75`, `0x7c1b7d`, `0x7c1b93`, `0x7c1b96`, `0x7c1b9e`, `0x7c1bb8`, `0x7c1bbd`, `0x7c1bc5`, `0x7c1bd2`, `0x7c1c26`, `0x4549f0`
Confidence: verified
Proof: one
Test: none

`0x7c1b6b fcomp 0.0; test ah,5; jp` continues for `tca` at least 0 or NaN,
`0x7c1b7a test ah,0x41; jne` continues for `L² ≤ r²`, and `0x7c1bc5 jne 0x7c1bd2` takes `t = tca + thc` for
`L² ≤ r²`. The function is not called.

## Ray and sphere by the quadratic

`0x7c1c40(outT, outPoint)` takes the same arguments with the ray in `ecx` and the sphere in `edx`. It normalizes
the direction (`nD=dir/|dir|`) and forms `L=origin−centre`, `A=nD·nD`, `B=2(L·nD)`, `c=L·L−r²` and `disc=B²−4Ac`,
and returns 0 for `disc<0`. The root is `t=(−B−√disc)/(2A)`, the near root even for an origin inside the sphere, and
`−B/(2A)` for `disc=0`. `t` is multiplied by `1/|dir|` and written to `outT`, and the point is `origin+t·dir` with the
original direction. A null `outT` returns 1 after the test.

Addresses: `0x7c1c40`, `0x7c1c58`, `0x7c1c65`, `0x7c1c8e`, `0x7c1cb4`, `0x7c1ced`, `0x7c1cf0`, `0x7c1d06`, `0x7c1d12`, `0x7c1d2f`, `0x7c1d31`, `0x7c1d41`, `0x7c1d4d`, `0x7c1d66`, `0x7c1d7a`, `0x4549f0`, `0x4549a0`, `0x80306c`, `0x7ffd74`, `0x7ff9d8`
Confidence: verified
Proof: one
Test: none

`0x7c1c58 call 0x4549f0` is the self dot of the direction and `0x7c1c65` stores `1/|dir|` at `[ebp-8]`. `B` is stored as a float at
`0x7c1ced fst [ebp-4]` and `0x7c1cf0 fmul [ebp-4]` multiplies the unrounded `B` by that float for `B²`. `0x7c1d06 fmul [0x80306c]` is the 4.0. The tangent
test reads the stored float of `disc` (`0x7c1d31 fcomp 0.0; test ah,0x44; jp 0x7c1d4d`), so a positive `disc` that rounds to 0 takes the
tangent leg. `0x7c1d66` multiplies the stored `t` by `[ebp-8]`. The function is not called.

## Circle and point against a box

`0x7c2040(mode)` tests the circle or sphere in `edx` (centre x and y at `+0` and `+4`, radius at `+0xc`) against the box in `ecx` on the
first two axes and returns a byte, with `r²` the rounded squared radius. Per axis `dmin` and `dmax` are the squared distances to the two faces;
`near` sums `dmin` for a centre below the minimum and `dmax` above the maximum, and `far` sums the larger of the two. Mode 3 is true when
`near≤r²`, and mode 2 when also `r²≤far`. Mode 1 needs `near≤r²` and a hit flag, mode 0 the flag, `near≤r²` and `r²≤far`. A mode above 3 returns 0.

Addresses: `0x7c2040`, `0x7c2046`, `0x7c2057`, `0x7c2061`, `0x7c2067`, `0x7c206e`, `0x7c211e`, `0x7c21ab`, `0x7c21e1`, `0x7c2213`, `0x7c2229`, `0x7c223a`, `0x7c2250`, `0x7c2259`, `0x7c2280`, `0x7c2288`, `0x7c229c`, `0x68af10`
Confidence: verified
Proof: one
Test: `cmath::sphere_aabb_difftest`

The flag is set by an outside axis in both modes. For a centre between the faces, mode 0 sets it when the smaller of `dmin` and `dmax` is at most
`r²`, and mode 1 when the distance to a face is at most the radius itself (`0x7c216d fcomp [ebp-8]`, the unsquared `r`). Mode 0 stores `dmax` as a
float (`0x7c2099 fstp [ebp+8]`) and keeps `dmin` at 53 bits. The jump table `0x7c229c` holds `0x7c206e`, `0x7c211e`, `0x7c21ab` and `0x7c223a`, and all four
legs end in `fcomp` with `test ah,0x41; jp`, so equality passes and NaN fails (`0x7c2288 jp 0x7c21a0`). The caller `0x68af10` uses mode 3.
`0x7c1dd0` is a twin: a normalized diff differs only in the prologue and the jump table.

## Point to segment

`0x7c46d0(outT)` returns, in `st0`, the squared distance from the point in `edx` to the segment in `ecx`
(`a` at `+0`, `b` at `+0xc`), and writes the clamped parameter to `outT` when it is non-null. With `ab = b − a`
and `d = (q − a)·ab`: `t = 0` for `d ≤ 0`, `t = 1` for `d` above `|ab|²`, else `t = d/|ab|²`; the squared distance is
`|q − (a + t·ab)|²`. A zero-length segment takes `t = 0`. The closest point is not written.

Addresses: `0x7c46d0`, `0x7c4711`, `0x7c4735`, `0x7c4740`, `0x7c4742`, `0x7c4760`, `0x7c4763`, `0x7c476a`, `0x7c476e`, `0x7c4789`, `0x7c47b8`, `0x7c47be`, `0x7c47d7`, `0x6c4300`, `0x7ffd74`, `0x7ff9d8`
Confidence: verified
Proof: one
Test: `cmath::seg_proj_difftest`

`0x7c4735 fcomp 0.0; test ah,0x41; jp 0x7c474a` goes on for `d` above 0 and NaN; the fall-through loads 0.0 for `t`.
`0x7c4760 fld [ebp-4]; fcomp st(1)` with `jne 0x7c4789` takes the interior for `d ≤ |ab|²` and NaN, and the
fall-through loads 1.0 for the far end. The interior divides with `fdivr [ebp-4]`. The x component of the
difference stays at 53 bits while y and z subtract rounded products. The squared distance is left on the x87 stack. The caller `0x6c4300` takes its square root.

## Triangles in the plane

`0x7c3e30` takes two plane records with three vertices each (normal at `+0`, constant at `+0xc`, vertices at `+0x10`, `+0x1c` and `+0x28`) and
answers whether the triangles overlap. It classes the three vertices of the record in `edx` against the plane of the one in `ecx` (bounds at
`0x8029d0` and `0x810ccc`), and when all three lie in the plane calls `0x7c4150`. `0x7c4150` projects both triangles onto the two axes other than
the dominant axis of the first normal (a tie goes to the later axis) and returns 1 when an edge of one meets the other (`0x7c4030`) or a vertex of one lies in the other (`0x7c4090`).

Addresses: `0x7c3e30`, `0x7c3e55`, `0x7c3e60`, `0x7c3ebf`, `0x7c3f52`, `0x7c3ffd`, `0x7c401d`, `0x7c4150`, `0x7c4156`, `0x7c420d`, `0x7c426d`, `0x7c428a`, `0x7c4030`, `0x7c4040`, `0x7c3c10`, `0x7c4090`, `0x7c40fa`, `0x602630`, `0x8029d0`, `0x810ccc`, `0x802970`
Confidence: inferred
Proof: none
Test: none

`0x7c3e55 call 0x602630` forms the signed distance with `0x7c3e5d fadd [eax+0xc]`, and the compares at `0x7c3e60` and the class counts at `[ebp-0x44]` to
`[ebp-0x3c]` decide the route. The edge intersections between the triangle edges and the plane (`0x7c3ee8` to `0x7c3f4a`) and the choice of kept axes (`0x7c3f52`)
feed the calls at `0x7c3ffd` and `0x7c401d`. `0x7c4030` tests a segment against the three edges of a triangle with `0x7c3c10` (read as four cross products and sign compares, not traced to its algebra), and
`0x7c4090` is the point in triangle test with a tolerance in a double at `0x802970`.

## Lines

`0x7c4630(flag)` returns the distance from the point in `edx` to the line through the origin at `+0` and the direction at `+0xc` of the record in
`ecx`, as the length of `(q − origin) × dir`, divided by `|dir|` when the flag byte is 0. `0x7c4510` finds the closest points of two lines given
by an origin and a direction each: it writes the parameter on each line to two output pointers, and 0 for both when the squared cross product
of the directions is below 0.001 (the float at `0x801360`). `0x7c43d0` uses it for the distance between a segment (endpoints at `+0` and `+0xc`) and a line.

Addresses: `0x7c4630`, `0x7c463b`, `0x7c4687`, `0x7c4689`, `0x7c4510`, `0x7c455a`, `0x7c460c`, `0x7c43d0`, `0x7c442c`, `0x7c44f6`, `0x7c44fb`, `0x801360`, `0x4841f0`, `0x621160`
Confidence: inferred
Proof: none
Test: none

`0x7c4689 jne 0x7c46bb` skips the division by `|dir|` for a non-zero flag, and `0x7c4687 fsqrt` is the length of the cross product. `0x7c455a fcomp [0x801360]` with
`jnp 0x7c460c` sends a squared cross product below the bound to the zero outputs. `0x7c43d0` passes the segment's difference and the second record's
origin and direction to `0x7c4510`, forms both closest points (`0x4841f0`, `0x621160`) and returns the length of their difference (`0x7c44fb fsqrt`). The
algebra of the parameters in `0x7c4510` is not traced.

## Rotation of an angle

`0x7c42f0(angle)` writes the 2×2 rotation `[[cos, sin], [−sin, cos]]` of an angle to the four floats at `ecx`, each of cosine and sine rounded to a
float and the negated sine an exact negation. Its one caller, `0x68c17e`, is in the unreached `0x68c0f0`, so nothing reaches it.

Addresses: `0x7c42f0`, `0x68c17e`, `0x68c0f0`, `0x7c430b`, `0x7c430d`, `0x7c430f`
Confidence: verified
Proof: one
Test: none

`0x7c430b fsincos` leaves the cosine on top; `0x7c430d fstp [eax]` stores the cosine at `[ebp-8]` and `0x7c430f fstp [edx]` the sine at `[ebp-4]`. The four
output stores are the cosine, the sine, `fchs` of the sine and the cosine again.

## Static constants

The span `0x7bc090` to `0x7c4810` holds twelve object files. The C runtime's initializer run `0x82da20` to `0x82dab0` opens each with its +∞, followed by its 2π and 1/2π, at `0x7bc090`, `0x7be0c0`,
`0x7bf6e0`, `0x7bf840`, `0x7bfa30`, `0x7bfb20`, `0x7c0170`, `0x7c13a0`, `0x7c1570`, `0x7c18a0`, `0x7c4340` and `0x7c43b0`. The first stores at `0xcf5c04`, and each next file's three
constants follow from `0xcf5c10`, `0xcf5c1c`, `0xcf5c28`, `0xcf5c34`, `0xcf5c40`, `0xcf5c4c`, `0xcf5c58`, `0xcf5c64`, `0xcf5c70`, `0xcf5c94` and `0xcf5ca0`; the twelfth file's 2π and 1/2π
are in `collision/overview#object-files`. Nothing else in the listing refers to any of the copies.

The image gives no source path for these files, so the initializer names number them in this order: `InitCmathFile01` for the file at `0x7bc090` to `InitCmathFile12` for the file at `0x7c43b0`.

Addresses: `0x7bc090`, `0x7bc0a0`, `0x7bdf00`, `0x7bdf10`, `0x7bdf20`, `0x7bdf30`, `0x7be0c0`, `0x7be0d0`, `0x7bf630`, `0x7bf640`, `0x7bf650`, `0x7bf660`, `0x7bf6e0`, `0x7bf6f0`, `0x7bf760`, `0x7bf770`, `0x7bf780`, `0x7bf790`, `0x7bf840`, `0x7bf850`, `0x7bf980`, `0x7bf990`, `0x7bf9a0`, `0x7bf9b0`, `0x7bfa30`, `0x7bfa40`, `0x7bfa50`, `0x7bfa60`, `0x7bfa70`, `0x7bfa80`, `0x7bfb20`, `0x7bfb30`, `0x7c00a0`, `0x7c00b0`, `0x7c00c0`, `0x7c00d0`, `0x7c0170`, `0x7c0180`, `0x7c12d0`, `0x7c12e0`, `0x7c12f0`, `0x7c1300`, `0x7c13a0`, `0x7c13b0`, `0x7c1400`, `0x7c1410`, `0x7c1420`, `0x7c1430`, `0x7c1570`, `0x7c1580`, `0x7c1850`, `0x7c1860`, `0x7c1870`, `0x7c1880`, `0x7c18a0`, `0x7c18b0`, `0x7c42a0`, `0x7c42b0`, `0x7c42c0`, `0x7c42d0`, `0x7c4340`, `0x7c4350`, `0x7c4360`, `0x7c4370`, `0x7c4380`, `0x7c4390`, `0x7c43b0`, `0x7c43c0`, `0xcf5c04`, `0xcf5c08`, `0xcf5c0c`, `0xcf5c10`, `0xcf5c14`, `0xcf5c18`, `0xcf5c1c`, `0xcf5c20`, `0xcf5c24`, `0xcf5c28`, `0xcf5c2c`, `0xcf5c30`, `0xcf5c34`, `0xcf5c38`, `0xcf5c3c`, `0xcf5c40`, `0xcf5c44`, `0xcf5c48`, `0xcf5c4c`, `0xcf5c50`, `0xcf5c54`, `0xcf5c58`, `0xcf5c5c`, `0xcf5c60`, `0xcf5c64`, `0xcf5c68`, `0xcf5c6c`, `0xcf5c70`, `0xcf5c74`, `0xcf5c78`, `0xcf5c94`, `0xcf5c98`, `0xcf5c9c`, `0xcf5ca0`
Confidence: verified
Proof: one
Test: none

Every run entry is a five-byte jump to a body ten bytes on. The +∞ bodies are `mov eax,[+∞]; mov [dest],eax`; the 2π bodies are `fld [π]; fadd st,st; fstp [dest]`, and the 1/2π bodies `fld 1.0; fdiv [2π]; fstp [dest]`.
The slots before the run, `0x82da18` and `0x82da1c`, are the 2π and 1/2π of the file below it. The tenth file's run also holds `0x7c18c0`, the polygon axis initializer. The +∞ at `0x7c43c0` opens the twelfth file, not the
eleventh, whose 2π and 1/2π are `0x7c4360` and `0x7c4380`. A raw search of the image for each of the 36 copies' addresses finds the same references as the listing.
