# cmath: intersections

The intersection and distance routines of the Tempest math library in the geometry file, `0x7bf680`
through `0x7c4800`, and a second small file at `0x6dc2c0`. Vectors are three floats and a ray is an
origin followed by a direction (`+0` and `+0xc`). A routine returns a byte (0 or 1) and writes what it
found through optional pointers. Eight of the routines here are called by live code
(`0x7bf680`, `0x7c1450`, `0x7c2040`, `0x7c22b0`, `0x7c23e0`, `0x7c29f0`, `0x7c2c40`, `0x7c46d0`); the rest have
no caller and no stored address (a relative-call scan and the census of stored addresses). None has a
bit-exact test yet.

Roots: `0x7bf680`, `0x7c1450`, `0x7c2040`, `0x7c22b0`, `0x7c23e0`, `0x7c29f0`, `0x7c2c40`, `0x7c46d0`

## Dominant axis and bounds

`0x7bf680` returns the index of the largest component of the vector in `ecx` by magnitude: 0 only when
`|x|` is strictly greater than both `|y|` and `|z|`; otherwise 1 when `|y|` is strictly greater than
`|z|`, else 2, so a tie goes to the later axis. `0x7c1450(n, points)` writes the bounds of `n` points
(stride 12) to the six floats at `ecx`, minimum then maximum; for `n` of 0 all six are 0. It starts
from the first point and folds each next one with a per-axis compare, the maximum through `0x6992c0`.

Addresses: `0x7bf680`, `0x7bf69a`, `0x7bf69f`, `0x7bf6a2`, `0x7bf6a4`, `0x7bf6ac`, `0x7bf6b4`, `0x7bf6b9`, `0x7bf6c1`, `0x7bf6c6`, `0x7bf6c8`, `0x7c1450`, `0x7c145e`, `0x7c1463`, `0x7c14b6`, `0x7c1513`, `0x7c1531`, `0x7c1560`, `0x6992c0`, `0x4549a0`
Confidence: verified
Proof: one
Test: none

`0x7bf69a fcom [ebp-4]` compares `|x|` with `|y|`, `test ah,0x41; jne 0x7bf6b4` takes `|x|` at most `|y|` or NaN to
the second test; the first test's `fcomp [ebp-8]` with `jne 0x7bf6c8` returns 2 for `|x|` at most `|z|`.
At `0x7bf6b9` the compare of `|y|` with `|z|` and `mov eax,1; je 0x7bf6cd` return 1 only for `|y|` greater. The
bounds routine's loop runs `n − 1` times over the points from the second (`0x7c14b3 lea ebx,[eax-1]`).

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
Test: none

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
Test: none

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
Test: none

`0x7c2adc fcom [0x81d9bc]` (−1e-6) with `test ah,0x41; jne 0x7c2b03` continues for `det` at most −1e-6 or NaN,
then `0x7c2ae9 fcom [0x80e2e4]` (+1e-6) with `test ah,5; jp 0x7c2b03` continues for `det` at least 1e-6;
anything else returns 0. `u` is stored as a float at `0x7c2b58 fst`, with the unrounded value compared first
(`0x7c2b5b`, bound `[ebp-8] = −slack`) and the stored float second (`0x7c2b68`, `[ebp-0xc] = 1 + slack`). `v` is compared once, unrounded
(`0x7c2bcb`, `0x7c2bd6`), and `u + v` adds the stored `v` to the stored `u` (`0x7c2bdf`). The twin `0x7c2c40` differs by `mov` against
`movzx` and the index strides.

## Other triangle and polygon rays

`0x7c27d0`, `0x7c2e90` and `0x7c30a0` are further ray and triangle routines. `0x7c27d0` takes three stored vertices in
the argument and a slack, with the same `±1e-6` determinant band; `0x7c2e90` takes one-sided
vertices and divides last, and `0x7c30a0` is its 16-bit indexed form. `0x7c2e90` has the determinant constant
`+1e-6` and 0.0. None is called.

Addresses: `0x7c27d0`, `0x7c2e90`, `0x7c30a0`, `0x4549a0`, `0x80e2e4`, `0x81d9bc`, `0x7ff9d8`
Confidence: inferred
Proof: none
Test: none

The three are read only at their skeleton: the stack arguments (`ret 0xc`, `ret 8`, `ret 0xc`), the calls to
`0x4549a0` and the constants above. Their algorithms are the same family as `0x7c29f0` by the shape of the
calls, which is not verified instruction by instruction.

## Ray and sphere

`0x7c1af0(outT, outPoint)` intersects the ray in `ecx` (direction taken as a unit vector) with the sphere in
`edx` (centre at `+0`, radius at `+0xc`). `L = centre − origin`, `tca = L·dir`, `L² = L·L`. It misses when
`tca < 0` and `L² > r²`, and when `L² − tca² > r²`. Otherwise `thc = √(r² − (L² − tca²))` and `t = tca − thc`
for an origin outside the sphere (`L² > r²`), `t = tca + thc` inside or on it. The point is `origin + t·dir`
with every term rounded to a float.

Addresses: `0x7c1af0`, `0x7c1b3d`, `0x7c1b56`, `0x7c1b6b`, `0x7c1b70`, `0x7c1b75`, `0x7c1b7d`, `0x7c1b93`, `0x7c1b96`, `0x7c1b9e`, `0x7c1bb8`, `0x7c1bbd`, `0x7c1bc5`, `0x7c1bd2`, `0x7c1c26`, `0x7c1c40`, `0x4549f0`, `0x80306c`
Confidence: inferred
Proof: none
Test: none

`0x7c1c40` takes the same arguments; it normalizes the direction itself (`√` of its square and a
reciprocal), solves the quadratic with the literal 4.0, and rescales `t`. The first routine's instructions are read: `0x7c1b6b fcomp 0.0; test ah,5; jp` continues for `tca` at least 0 or NaN,
`0x7c1b7a test ah,0x41; jne` continues for `L² ≤ r²`, and `0x7c1bc5 jne 0x7c1bd2` takes `t = tca + thc` for
`L² ≤ r²`. The second is read at its skeleton (calls `0x4549f0` and `0x4549a0`, the constants 4.0, 1.0 and 0.0,
`ret 8`); the description of its scale and quadratic is the family's, not traced.

## Circle and point against a box

`0x7c2040(mode)` tests the circle or sphere in `edx` (centre x and y at `+0` and `+4`, radius at `+0xc`) against the
box in `ecx` (minimum at `+0`, maximum at `+0xc`) on the first two axes, returning a byte. `r²` is the
radius squared and rounded. With the per-axis squared distance `near` to the box (0 inside the slab, the
squared distance to the nearer face outside), mode 3 is true when `near ≤ r²`; mode 2 is true when `near ≤ r²`
and `r² ≤ far`, where `far` sums the larger of the two squared face distances. Modes 0 and 1 add a hit
flag. A mode above 3 returns 0. The routine is called with mode 3.

Addresses: `0x7c2040`, `0x7c2046`, `0x7c2057`, `0x7c2061`, `0x7c2067`, `0x7c206e`, `0x7c211e`, `0x7c21ab`, `0x7c21e1`, `0x7c2213`, `0x7c2229`, `0x7c223a`, `0x7c2250`, `0x7c2259`, `0x7c2280`, `0x7c2288`, `0x7c229c`, `0x7c1dd0`, `0x68af10`
Confidence: inferred
Proof: none
Test: none

`0x7c1dd0` is a twin: a normalized diff differs only in the prologue and the jump table. Modes 2 and 3 are read: `0x7c2250 fld [edx]; fcomp [edx+ecx]` against the minimum, `jp` past it for a centre at least the minimum,
then the maximum test at `0x7c2263`, two passes of `add edx,4`, and the final `fcomp [ebp-4]` at `0x7c2280` with
`jp 0x7c21a0` returning 0 for `near > r²` and NaN. The jump table `0x7c229c` holds `0x7c206e`, `0x7c211e`, `0x7c21ab`
and `0x7c223a`; modes 0 and 1 are read only for their frame. `0x68af10` is the caller with mode 3.

## Point to segment

`0x7c46d0(outT)` returns, in `st0`, the squared distance from the point in `edx` to the segment in `ecx`
(`a` at `+0`, `b` at `+0xc`), and writes the clamped parameter to `outT` when it is non-null. With `ab = b − a`
and `d = (q − a)·ab`: `t = 0` for `d ≤ 0`, `t = 1` for `d` above `|ab|²`, else `t = d/|ab|²`; the squared distance is
`|q − (a + t·ab)|²`. A zero-length segment takes `t = 0`. The closest point is not written.

Addresses: `0x7c46d0`, `0x7c4711`, `0x7c4735`, `0x7c4740`, `0x7c4742`, `0x7c4760`, `0x7c4763`, `0x7c476a`, `0x7c476e`, `0x7c4789`, `0x7c47b8`, `0x7c47be`, `0x7c47d7`, `0x6c4300`, `0x7ffd74`, `0x7ff9d8`
Confidence: verified
Proof: one
Test: none

`0x7c4735 fcomp 0.0; test ah,0x41; jp 0x7c474a` goes on for `d` above 0 and NaN; the fall-through loads 0.0 for `t`.
`0x7c4760 fld [ebp-4]; fcomp st(1)` with `jne 0x7c4789` takes the interior for `d ≤ |ab|²` and NaN, and the
fall-through loads 1.0 for the far end. The interior divides with `fdivr [ebp-4]`. The x component of the
difference stays at 53 bits while y and z subtract rounded products. The squared distance is left on the x87 stack. The caller `0x6c4300` takes its square root.

## Distances, sweeps and overlaps

`0x7c1920` clips a segment against a box (`ret 8`, `0.0`).
`0x7c32f0` tests a point against a cone: the offset from an apex on the axis at `+0x10`, the squared projection
against the squared offset scaled by a squared factor at `+0x20`, and a cap at `+0x1c` (0 means none).
`0x7c3390` sweeps a sphere against a capsule axis through `0x5f8cf0` (scale) and `0x621160` (add), with the
`1e-6` bands and 0.5; `0x7c3780` is the box-box overlap on fifteen axes with no epsilon; `0x7c3c10`, `0x7c3d20`
`0x7c3e30`, `0x7c4030`, `0x7c4090` and `0x7c4150` are 2D segment and triangle tests; `0x7c42f0` writes the
2×2 rotation `[[cos, sin], [−sin, cos]]` of an angle. `0x7c43d0`, `0x7c4510` and `0x7c4630` are line and
line gaps. 
Addresses: `0x7c1920`, `0x7c32f0`, `0x7c3390`, `0x7c3780`, `0x7c3c10`, `0x7c3d20`, `0x7c3e30`, `0x7c4030`, `0x7c4090`, `0x7c4150`, `0x7c42f0`, `0x7c43d0`, `0x7c4510`, `0x7c4630`, `0x6dc2c0`, `0x6dc7a0`, `0x6dc810`, `0x6dc900`, `0x5f8cf0`, `0x621160`, `0x8101b8`
Confidence: inferred
Proof: none
Test: none

The second file has `0x6dc7a0` (sphere against a box, true for squared distance at most `r²`),
`0x6dc810` (a box against a plane, 0 straddling or within 0.01944 of it, 1 in front, 2 behind) and
`0x6dc900` (squared distance between two segments given as a point and a direction each), and `0x6dc2c0`.

These are read at their skeletons: frames, stack arguments, calls, constants and a few compares. `0x7c32f0` is read
through its first compare. `0x6dc810` selects per axis the near and far vertex by the plane normal's sign
(`0x6dc83c fcomp 0.0; test ah,1`) and compares the two signed distances by sign bit and against `0x8101b8`. `0x6dc7a0`
clamps the centre to the box on three axes (`0x6dc7c2` to `0x6dc7ee`). The remaining algorithms are named by the
rows and are not traced here.
