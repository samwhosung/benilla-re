# cmath: boxes, cones and triangles

The routines of the Tempest math library that test a ray, a point or a box against a box, a cone,
a plane or a triangle. Vectors are three floats; a ray is an origin followed by a direction (`+0` and
`+0xc`); a box is its minimum then its maximum (`+0` and `+0xc`). A routine that answers yes or no
returns a byte. No routine here has a stored address, and none has a caller that code reaches (a relative-call scan and
the census of stored addresses): `0x6dc900` is called only from `0x6b99fa`, inside the unreached `0x6b9950`.

Roots: none

## Ray and box

`0x7c1920(outT, outPoint)` finds where the ray in `ecx` (any direction length) enters the box in `edx`. An
origin between the faces on all three axes gives `t = 0` and the origin. Otherwise each axis with the origin
outside has a candidate `T = (near face − origin) / dir`, and −1 for a zero direction component. The axis with the
largest `T` (the first on a tie) must have `T ≥ 0`. The point takes the near face on that axis and `origin + T·dir`,
rounded to a float, on the others, and each of those must lie between the box's minimum and maximum. Both
outputs are optional.

Addresses: `0x7c1920`, `0x7c196a`, `0x7c198b`, `0x7c19b5`, `0x7c19e6`, `0x7c1a00`, `0x7c1a22`, `0x7c1a4b`, `0x7c1a64`, `0x7c1a9b`, `0x7c1aaa`, `0x7c1abd`, `0x7ffd74`
Confidence: verified
Proof: one
Test: none

`0x7c196a fcomp [min]; test ah,5; jp` leaves an origin at or above the minimum, and below it the axis gets class 1 and
the minimum as its candidate. `0x7c198b fcomp [max]; test ah,0x41; jne` leaves an origin at or below the maximum
(class 2, no candidate), and above it the axis gets class 0 and the maximum. When no axis has an outside class
`0x7c19b5` copies the origin. A middle axis, and a zero direction (`0x7c1a00 test ah,0x44`), store −1.0 at `0x7c1a22`. The
winner scan `0x7c1a4b` keeps the later axis only when its `T` is strictly larger. `0x7c1a64 fcomp 0.0; test ah,5; jp`
continues for a non-negative or NaN `T`; `0x7c1a9b` forms `dir·T + origin`, and `0x7c1aaa` and `0x7c1abd` reject a value
below the minimum or above the maximum. A null `outPoint` is redirected to a local.

## Sphere and box

`0x6dc7a0(radius)` takes a box in `ecx` and a centre in `edx` and returns 1 when the sum over the three axes of the squared
distance from the centre to the box's slab (0 inside it) is at most the squared radius.

Addresses: `0x6dc7a0`, `0x6dc7c2`, `0x6dc7d3`, `0x6dc7f3`
Confidence: verified
Proof: one
Test: none

`0x6dc7c2 fcomp [min]; test ah,5; jp` takes the below-minimum leg (`c − min`), `0x6dc7d3 fcomp [max]; test ah,0x41; jne` skips a centre
at or below the maximum, and a NaN centre adds nothing. The final `0x6dc7f3 fcomp` with `jp` returns 1 for a sum below or equal to `r²`.

## Box and plane

`0x6dc810` takes a box in `ecx` and a plane in `edx` (normal at `+0`, constant at `+0xc`). Per axis it takes the box vertex
with the lowest signed distance (the minimum for a non-negative normal component, else the maximum) and the one with the highest,
and forms `n·v + d` for both as floats. It returns 0 when their sign bits differ or either magnitude is below 0.0194444 (a float at
`0x8101b8`), 1 when the lowest distance is above that value, else 2.

Addresses: `0x6dc810`, `0x6dc83c`, `0x6dc86a`, `0x6dc8a9`, `0x6dc8b6`, `0x6dc8d7`, `0x8101b8`
Confidence: verified
Proof: one
Test: none

`0x6dc83c fcomp 0.0; test ah,1` splits the normal's sign; `0x6dc8a9 xor ecx,edx; test ecx,ecx; js` is the sign-bit test, and
`0x6dc8b6` and `0x6dc8d7` compare the magnitudes and the lowest distance with the tolerance.

## Segments in the plane

`0x6dc2c0(c, d, out)` takes the segment from `ecx` to `edx` and the segment from `c` to `d` (each point two floats). It
forms the determinant of the two directions and returns 0, writing nothing, when it is 0. Otherwise it computes the
parameters `s` along the first segment and `t` along the second, each rounded to a float, and returns 1 when both are in
[0, 1]. When not it writes `a + s·(b − a)`, where the two lines meet, to `out` and returns 0. `0x7c3d20` is the same
with `|det| < 0.001` (a float at `0x801360`) as the miss test, and it writes the point before it tests the parameters.

Addresses: `0x6dc2c0`, `0x6dc2f7`, `0x6dc302`, `0x6dc310`, `0x6dc353`, `0x6dc39e`, `0x7c3d20`, `0x7c3d57`, `0x7c3d5b`, `0x7c3dba`, `0x7c3dd4`, `0x801360`, `0x7ffd74`, `0x7ff9d8`
Confidence: verified
Proof: one
Test: none

The determinant at `0x6dc2f7` is `(a.y − b.y)·c.x + (c.y − d.y)·b.x + (b.y − a.y)·d.x + (d.y − c.y)·a.x`, and `0x6dc302 jp` is
not taken only for 0. The first numerator `(a.y − d.y)·c.x + (c.y − a.y)·d.x + (d.y − c.y)·a.x` over it equals `s` of the line crossing.
The range tests `0x6dc353` to `0x6dc386` are four `fcomp` pairs against 0.0 and 1.0 with `test ah,0x41; jp`, which let 0 and 1
through. `0x7c3d5b fcomp [0x801360]` and `jp 0x7c3d74` continue for `|det|` at least 0.001 or NaN; the point store is `0x7c3dba`.

## Point to triangle

`0x6dc900(e0, e1)` returns the squared distance from the point `P` in `ecx` to the triangle with vertex `B` in `edx` and edge
vectors `e0`, `e1` on the stack. With `r=B−P` it minimises `Q=a·s²+2b·s·t+c·t²+2d·s+2e·t+f`, which is `|r+s·e0+t·e1|²`, over
`s,t≥0` with `s+t≤1`. Here `a=e0·e0`, `b=e0·e1`, `c=e1·e1`, `d=r·e0`, `e=r·e1` and `f=r·r`. With `det=|ac−b²|`, `S=be−cd` and `T=bd−ae`,
the closest point is inside when `S,T≥0` and `S+T≤det`, at `s=S/det` and `t=T/det`. The result is `|Q|`.

Addresses: `0x6dc900`, `0x6dc906`, `0x6dc93d`, `0x6dc95e`, `0x6dc9d2`, `0x6dc9fb`, `0x6dca09`, `0x6dcb0d`, `0x6dcd73`
Confidence: verified
Proof: one
Test: none

`r` is stored at `0x6dc906` to `0x6dc922` as `edx − ecx`, `a` and `c` at `[ebp-8]` and `[ebp-4]` (`0x6dc93d`, `0x6dc95e`), `b` at
`[ebp-0xc]`, `d` and `e` over the argument slots `[ebp+0xc]` and `[ebp+8]`, and `f` stays on `st0`. `0x6dc9d2 fabs` takes the
determinant's magnitude. `0x6dc9fb fcomp [det]` and `0x6dca09 test ah,0x41; jp 0x6dcb59` send `S + T > det` away from the near side. The
interior leg `0x6dcb0d` takes `1/det` and stores `s` as a float (`0x6dcb1b`) while `t` stays at 53 bits. Corner legs add `2d` or `2e` to `f` before `a` or `c`. Every leg ends at `0x6dcd73 fabs`. Its one caller, `0x6b99fa`, is in the unreached `0x6b9950`, so nothing reaches it.

## Point to triangle edges

Within `S+T≤det`, `S<0` with `T≥0` minimises on the edge `s=0`, and `S≥0` with `T<0` on the edge `t=0`; `S<0` with `T<0` takes
`t=0` for `d<0` and `s=0` otherwise. Each edge clamps its parameter to [0, 1]: on `s=0` the value is `f−e²/c` with corners `f` and
`c+2e+f`, and on `t=0` it is `f−d²/a` with corners `f` and `a+2d+f`.

Addresses: `0x6dc900`, `0x6dca14`, `0x6dca1d`, `0x6dca2a`, `0x6dca77`, `0x6dcac5`, `0x6dcae8`, `0x6dcaac`, `0x6dcbe3`, `0x6dccc9`, `0x8030a0`
Confidence: verified
Proof: one
Test: none

`0x6dca14 test ah,5; jp 0x6dcac5` separates `S≥0`. Then `0x6dcac5` tests `T` (`0x6dcb0d` is the interior) and `0x6dca1d` tests it for `S<0`;
the `T<0` pair meets at `0x6dcad2` and `0x6dca77`, and `0x6dca2a` tests `d` (`jnp 0x6dcae8` for `d<0`). The edge values divide by `a` or `c` through
the −1.0 at `0x8030a0` (`0x6dcaac`, `0x6dcbe3`, `0x6dccc9`), and the corner leaves add `2d` or `2e` to `f` and `a` or `c` (`0x6dcafb`, `0x6dca9a`).

## Point to triangle hypotenuse

Beyond `S+T>det`, `S≥0` with `T≥0` always takes the edge `s+t=1`, with `s=(c+e−b−d)/(a−2b+c)` clamped to [0, 1] and `t=1−s`. `S<0` takes
it when `c+e` exceeds `b+d`, and `T<0` when `a+d` exceeds `b+e`, with `t=(a+d−b−e)/(a−2b+c)` clamped and `s=1−t`; otherwise they take the edge `s=0` or `t=0`.

Addresses: `0x6dc900`, `0x6dcb59`, `0x6dcb62`, `0x6dcb7d`, `0x6dcb96`, `0x6dcbb3`, `0x6dcbfc`, `0x6dcc10`, `0x6dcc26`, `0x6dcc3f`, `0x6dcc57`, `0x6dcc99`, `0x6dcce2`, `0x6dccfb`, `0x6dcd0d`, `0x6dcd24`, `0x6dcd3b`
Confidence: verified
Proof: one
Test: none

`0x6dcb59 test ah,5; jp 0x6dcbfc` separates `S≥0` on the far side. `S<0` (`0x6dcb64`) forms `b+d` and `c+e` and `0x6dcb73 fcom; jne 0x6dcbb3` sends
`c+e≤b+d` to the edge leg; otherwise `numer=c+e−(b+d)` and `den=a−2b+c`, and `0x6dcb8f fcomp` with `jne 0x6dcbac` takes `numer<den` to
`s=numer/den` (tail `0x6dcd3b`) and else the corner `a+2d+f` (`0x6dcb96`). `T<0` (`0x6dcc10`) mirrors it with `a+d` and `b+e`, `numer/den` being `t` (tail `0x6dcc57`) and `numer≥den` the corner `c+2e+f` (`0x6dcc3f`). `S,T≥0`
(`0x6dcce2`) has `numer≤0` as the corner `c+2e+f` (`0x6dccfb`) and `numer≥den` as `a+2d+f` (`0x6dcd24`). Tails `0x6dcc57` and `0x6dcd3b` weight by `1−s`.

## Box against box

`0x7c3780` takes two oriented boxes and returns 1 when none of the fifteen axes separates them. A box is 15 floats: the
centre at `+0`, half-extents at `+0xc`, and three unit axis rows at `+0x18`, `+0x24` and `+0x30`. With `t` the centre of `edx`
minus the centre of `ecx` and `R[i][j]` the dot of axis `i` of `ecx` with axis `j` of `edx`, it tests the three axes of `ecx`,
the three of `edx`, then the nine cross axes `A_i × B_j` in row order. Axis `A_i × B_j`, with `(i, k, m)` and `(j, l, n)` cyclic, separates when
`|R[k][j]·(t·A_m) − R[m][j]·(t·A_k)|` is greater than `|R[k][j]|·hA_m + |R[m][j]|·hA_k + |R[i][l]|·hB_n + |R[i][n]|·hB_l`.
No epsilon is added to any `R`.

Addresses: `0x7c3780`, `0x7c37a8`, `0x7c3807`, `0x7c3820`, `0x7c3894`, `0x7c3904`, `0x7c39c9`, `0x7c39f7`, `0x7c3a08`, `0x7c3bc1`, `0x7c3bef`, `0x7c3c00`
Confidence: verified
Proof: one
Test: none

Three loops of three iterations (`0x7c37a8`, `0x7c3820`, `0x7c3894`) store each `R` entry and its absolute value, signed in
`[ebp-0x60]` to `[ebp-0x40]` and absolute in `[ebp-0x3c]` to `[ebp-0x1c]`. The first three tests use `t·A_i` (stored at `[ebp-0xc]`, `[ebp-8]`
and `[ebp-4]`) against the sum of `|R|` times the other box's extents plus the box's own extent; the next three use `t·B_j` against the mirrored sum.
Each test ends `fcompp; fnstsw; test ah,5; jp next`: the next test runs unless the radius is strictly below the projection, so touching boxes
overlap and a NaN does not separate. A separation returns 0 at once; passing all fifteen reaches `0x7c3c00` (`mov al,1`).

## Ray and cone

`0x7c3390(outT, outPoint)` intersects the ray in `ecx` (origin `+0`, direction `D` `+0xc`) with the cone in `edx`: apex `C` `+0`, unit axis `U` `+0x10`, cap length `H` `+0x1c`, `k` `+0x20` (half-angle cosine). With `W = L + t·D` and `L = origin − C`, the surface is
`(W·U)² = k²·|W|²`, a quadratic `A·t² + 2B·t + Cc = 0` with `A = (D·U)² − k²|D|²`, `B = (D·U)(L·U) − k²(L·D)` and
`Cc = (L·U)² − k²|L|²`. The hit point `P` must have `(P − C)·U` above 0 and, for a non-zero `H`, below `H`, except in the degenerate leg, which returns 1 at the origin untested.

Addresses: `0x7c3390`, `0x7c3444`, `0x7c3455`, `0x7c3477`, `0x7c3496`, `0x7c34ac`, `0x7c34e8`, `0x7c3505`, `0x7c3526`, `0x7c35b5`, `0x7c365d`, `0x7c3673`, `0x7c36e6`, `0x7c36f8`, `0x7c3737`, `0x7c3751`, `0x5f8cf0`, `0x621160`, `0x80e2e4`, `0x7ffa24`
Confidence: verified
Proof: one
Test: none

`|A| < 1e-6` (`0x80e2e4`) takes the linear leg, otherwise `disc = B² − A·Cc` below 0 returns 0, 0 takes the single root and above 0 the two roots. The two-root leg
computes `t1 = (−B − √disc)/A` and `t2 = (√disc − A)/A`; the second subtracts `A`, not `B` (`0x7c3507 fsub [ebp-4]`). It reports `t1` when `t1 ≥ 0`
else FLT_MAX, replaced by `t2` when `t2` is non-negative and smaller, and the point is `origin + t·D`. The single root reports `t = B/A` and the linear leg
`t = 0.5·Cc/B` (`0x7ffa24`) for `|B| ≥ 1e-6`; both place the point at `origin − t·D`. With `|B|` below the bound the leg returns 0, or for `|Cc|` below it
reports `t = 0` at the origin and returns 1 at `0x7c3751` without the `(P − C)·U` test. `0x7c36e6` forms `(P − C)·U` and requires it above 0 and, for a non-zero `H`, below `H`.
The helpers `0x5f8cf0` and `0x621160` scale and add vectors.

## Point in a cone

`0x7c32f0` tests the point in `ecx` against the cone in `edx` (same layout as the ray and cone). With `d = point − apex` and `p = d·U` it
returns 1 when `p² ≥ k²·|d|²` and, for a non-zero cap `H`, `p ≤ H`. The sign of `p` is not tested, so the cone is a double cone and a point
behind the apex inside the mirror cone passes.

Addresses: `0x7c32f0`, `0x7c3304`, `0x7c3317`, `0x7c3339`, `0x7c3343`, `0x7c3350`, `0x7c3353`, `0x7c3363`, `0x7c336d`, `0x7c337d`, `0x7ffd74`
Confidence: verified
Proof: one
Test: none

`0x7c3317 fstp [ebp-4]` stores `p`. `0x7c3339 fcompp` compares `k²·|d|²` with `p²` and `0x7c3343 test ah,0x41; jp` clears the flag when `k²·|d|²`
is larger or NaN. `0x7c3353 fcomp 0.0; test ah,0x44; jnp 0x7c337d` returns the flag unchanged for a cap of 0; otherwise `0x7c3363 fcomp [cap]`
and `jp 0x7c3377` fail `p` above the cap. The function is not called.
