# gxumath

The graphics maths layer: the matrix builders the interface and the world draw through, the
frustum and screen-projection code beside them, and picking, decal and image routines that
nothing in this build calls.

It is one object file, `0x5c3cc0` through `0x5c6640`, with the C runtime's initializer slots
`0x82aa38` to `0x82aa40`. A matrix is 16 floats, row-major: element (row r, column c) is float `4r + c`,
and a point is a row vector the matrix multiplies on the right, so `0x7bcb40` gives `x·m0 + y·m4 +
z·m8 + w·m12` first. The file calls `cmath` for the 4×4 determinant, inverse,
product, vector transform and translation, `spline` for the three-vector constructor `0x4549a0`,
and `gx` for the device getters. A failed argument check calls `0x64e850` with 0x57, which leaves
the output alone.

Roots: `0x5c3cc0`, `0x5c3d90`, `0x5c3e70`, `0x5c43b0`, `0x5c4f30`, `0x5c6540`, `0x5c5350`, and
the static initializers `0x5c4390`, `0x5c64f0` and `0x5c6510`.

## Perspective matrix

`0x5c3cc0` (`ecx` the output; `fov`, `aspect`, `near`, `far` on the stack; `ret 0x10`) writes the
perspective matrix. With `t = tan(0.5·fov/√(aspect² + 1))` and `d = far − near`: `m0 =
near/(aspect·(t·near))`, `m5 = near/(t·near)`, `m10 = (near + far)/d`, `m14 = near·far·(−2)/d`,
`m11 = 1`. The other floats, `m15` included, are 0. It needs `0 < fov < π`, `aspect > 0` and
`near < far`, NaN failing each; a failed test raises error 0x57 and writes nothing.

Addresses: `0x5c3cc0`, `0x8038dc`, `0x5c3cc6`, `0x5c3cd6`, `0x5c3ce6`, `0x5c3cf6`, `0x5c3d43`, `0x5c3d4c`
Confidence: verified
Proof: one
Test: `gxumath::perspective_difftest`

`0x5c3cc6` to `0x5c3cfe`: `fcomp 0.0; test ah,0x41; jne` (fails for `fov ≤ 0` or NaN), `fcomp π;
test ah,5; jp`, the aspect against 0.0, and `near` against `far` with `jnp`. `0x5c3d43` is `fdivr`
of `fov` by the root, `0x5c3d4c` runs `fptan` after `fmul 0.5`, and `fstp st(0)` drops its 1.0. The
float −2.0 at `0x8038dc` multiplies `near·far`. The products and quotients are 53-bit, `t·near`
one product, and each store rounds to float. The test runs the whole function.

## Orthographic matrix

`0x5c3d90` (`ecx` the output; `l`, `r`, `b`, `t`, `n`, `f` on the stack; `ret 0x18`) writes `m0 =
2/(r − l)`, `m5 = 2/(t − b)`, `m10 = 2/(f − n)`, `m12 = −(l + r)/(r − l)`, `m13 = −(b + t)/(t − b)`,
`m14 = −(n + f)/(f − n)`, `m15 = 1` and 0 elsewhere. It needs `l ≠ r`, `b ≠ t` and `n < f`, else error
0x57; NaN passes the first two. `0x5c3e50` loads six floats from the structure at `ecx` in that
order and calls it with the output in `edx`; nothing calls it.

Addresses: `0x5c3d90`, `0x5c3e50`, `0x5c3d9b`, `0x5c3da8`, `0x5c3db5`, `0x5c3dc5`
Confidence: verified
Proof: one
Test: `gxumath::ortho_difftest`

`0x5c3d9b` and `0x5c3da8` are `test ah,0x44; jnp` after `fcomp`: equal operands set C3 alone and
fail, an unordered pair sets C2 and C3 and passes. `0x5c3db5` is `test ah,5; jnp 0x5c3dc5`, which
proceeds only for `n < f`. Each quotient is the constant 2.0 over the 53-bit difference, rounded
at the store, and the negated sums divide by the same difference. The test runs the whole
function.

## Look-at matrices

`0x5c3e70` (`ecx` the eye, `edx` the target; the up vector and the output on the stack; `ret 8`)
writes a view matrix. With `f = normalize(target − eye)`, `s = normalize(f × up)` and `u = s × f`:
`m0 m4 m8` is `s`, `m1 m5 m9` is `u`, `m2 m6 m10` is `f`, `m3 m7 m11` are 0 and `m15` is 1. `m12 m13
m14` is the eye negated through `0x7bdc40`; `m12 = (−e.z·m8 + −e.y·m4 + −e.x·m0) + 0`. It fails when
`|target − eye|²` or `|up|²` is under 0.01. `0x5c4100`, which no code calls, uses `s' = up × f` and
`u' = f × s'`, which negates `m0 m4 m8` and `m12`.

Addresses: `0x5c3e70`, `0x5c4100`, `0x7bdc40`, `0x5c3efd`, `0x5c3f25`, `0x5c3f53`, `0x5c3f71`, `0x5c3fe6`, `0x5c4059`, `0x5c40eb`
Confidence: verified
Proof: one
Test: `gxumath::lookat_v1_difftest`

`0x5c3ed4` builds `f` with each difference rounded to float; the squares are rounded too and
summed in the order y, z, x, and `fcomp [0x8029d0]` (the float 0.01) with `test ah,1` fails below
it. The up vector's squares are summed unrounded in the order x, y, z (`0x5c3f25`). `0x5c3f53`
takes the root and `1/` it and scales `f`. `0x5c3f71` stores `f × up` as `(fy·uz − fz·uy, fz·ux −
fx·uz, fx·uy − fy·ux)`; `s` is scaled by the root of its squares summed z, y, x unrounded, and
`0x5c3fe6` stores `s × f`. A `f` parallel to `up` divides by 0. `0x5c4100` swaps the operands of
each subtraction of `s` (`0x5c41f0` to `0x5c4237`) and of `u`; with `s'` negated, `f × s'` equals
`s × f`, and rounding is sign-symmetric. The test runs `0x5c3e70` whole.

## Frustum corners

`0x5c43b0` (`ecx` the view matrix, `edx` the projection, an array of eight three-vectors on the
stack, `ret 4`) writes the world-space corners of the frustum. With `inv(proj)·inv(view)` from
`0x7bcf90`, `0x7bd6c0` and `0x7bc6a0`, the clip corners are `(sx, sy, ∓1, 1)` for a projection
whose `|m15 − 1|` is under the float `2⁻²²`, and otherwise `(sx·n, sy·n, −n, n)` near and `(sx·f,
sy·f, f, f)` far, with `n = −m14/(m10 + 1)` and `f = −m14/(m10 − 1)`. `(sx, sy)` runs `(−,−)`,
`(−,+)`, `(+,+)`, `(+,−)`. Each corner goes through `0x7bcb40` and keeps three components: near
corners at offsets 0 to `0x24`, far at `0x30` to `0x54`.

Addresses: `0x5c43b0`, `0x7bcf90`, `0x7bd6c0`, `0x7bc6a0`, `0x7bcb40`, `0x5c4404`, `0x5c441d`, `0x5c46ba`, `0x5c46cf`, `0x5c46e0`
Confidence: verified
Proof: one
Test: `gxumath::frustum_corners_difftest`

`0x5c4404` to `0x5c441d`: `fld m15; fsub 1.0; fabs; fcomp [0x8029d4]; test ah,5; jp 0x5c46ba`
takes the perspective branch for a difference at or over the epsilon and for NaN. `0x5c442a` and
the stores after it write the cube corners. `0x5c46ba` to `0x5c46e0` compute `n` and `f` with `fdivr` and
`fchs`; `fst` keeps the 53-bit value in `st0` and stores a float copy, and each determinant is
stored as a float for `0x7bd6c0`. A projection from `0x5c3cc0` has `m10 = (n + f)/d` and `m14 =
−2nf/d`, so these recover its planes. The test runs the whole function. `camera#unprojection`
interpolates the corners.

## World points to the screen

`0x5c4f30` (`ecx` a count, `edx` the points, the output on the stack, `ret 4`) maps world points
to window coordinates and returns 1, or 0 at the first point whose clip `|w'|` is under 0.001. `M`
is `0x7bc6a0` of the top of the device's matrix stack 8 and the matrix `0x58b0c0` gives; `(x', y', z', w')` is
`(x, y, z, 1)` through `0x7bcb40`; `iw = 1/w'`; `r` is the four floats of `0x58a240`. Then `out.x =
(x'·iw + 1)·0.5·(r3 − r1) + r1`, `out.y = (y'·iw + 1)·0.5·(r2 − r0) + r2` and `out.z = ((z'·iw +
1)·0.5·zs − zn) + zn`, with `zn` and `zs` the last two scalars of `0x58b060`. Earlier points keep
their values on a failure.

Addresses: `0x5c4f30`, `0x58a240`, `0x58b060`, `0x58b0c0`, `0x58b280`, `0x7bc6a0`, `0x7bcb40`, `0x5c5145`, `0x5c5152`, `0x5c5161`, `0x5c51ba`, `0x7e55ac`, `0x7e5220`
Confidence: verified
Proof: one
Test: `gxumath::unproject_difftest`

`0x58b280` with 8 copies the top of the device's matrix stack 8 (`0x5928e0`); `0x58b0c0` returns
the product of two device matrices, the stack top at `+0x1ac8` and the matrix at `+0xf50`. `0x5c5145`
to `0x5c5152`: `fabs; fcomp [0x801360]` (the float 0.001) `; test ah,5; jnp 0x5c51e2` returns 0
only for `|w'|` below it, NaN passing. The extents are `[ebp-4] − [ebp-0xc]` and `[ebp-8] − [ebp-0x10]`
(`0x5c517c`, `0x5c519b`), 53-bit; the z term subtracts and adds the same float
(`0x5c51bd`, `0x5c51c0`). The one caller `0x7e55ac` is in the glare query `0x7e5220`. The test
runs from world to window; the test is called `unproject_difftest`.

## Viewport projection for decals

`0x5c4d70` (`ecx` a count, `edx` points of three floats, the output on the stack, `ret 4`) maps
the x and y of each point through the matrix `0x58b0c0` gives: `x' = m0·x + m4·y + m12`, `y' =
m1·x + m5·y + m13`, then `sx = (x' + 1)·(a1 − a0)·0.5 + a0` and `sy = (y' + 1)·(b1 − b0)·0.5 + b0`,
with `a0 a1 b0 b1` the first four scalars of `0x58b060`. It writes `out.x = sx·r3`, `out.y = r2 −
sy·r2` and `out.z = 1`, `r` the four floats of `0x58a240`. Only `m0`, `m1`, `m4`, `m5`, `m12` and `m13`
are read. Its one caller is `0x5c5ee0`.

Addresses: `0x5c4d70`, `0x58a240`, `0x58b060`, `0x58b0c0`, `0x5c4e65`, `0x5c4e88`, `0x5c4ec3`, `0x5c4ef0`, `0x5c4eff`, `0x5c5ff4`
Confidence: verified
Proof: one
Test: none

`0x5c4e65` takes `a1 − a0` and `b1 − b0` first; the loop is at `0x5c4e88`. `0x5c4ec3` to `0x5c4ed1` add
1.0, multiply by the width and 0.5, add `a0`; the height path follows at `0x5c4eda`. `fst` stores
`sx` and `sy` as floats, then `0x5c4ef0` multiplies `sx` by `r3` and `0x5c4eff` is `fsubr` from
`r2`. A count of 0 returns without work.

## Decal grid rounding

`0x5c63d0` (`ecx` a capability record, `edx` an `(x, y)` pair, the output on the stack, `ret 4`)
writes `(g(x), g(y), 1)` for `g(v) = ceil(floor(16v + 0.5)/16)` when the dword at `+8` of the record
is nonzero, using `0x5c6470` if the dword at `+4` is 0 and `0x5c64b0` if not; the two are the same
code. With `+8` zero it writes nothing. The result is a whole number: the 53-bit product `16v`
plus the double 0.5 is floored, scaled by 1/16 and ceiled.

Addresses: `0x5c63d0`, `0x5c6470`, `0x5c64b0`, `0x5c63d6`, `0x5c63e0`, `0x5c647f`, `0x5c6488`, `0x5c648d`, `0x5c6496`, `0x73ff3f`, `0x73fdf5`
Confidence: verified
Proof: one
Test: none

`0x5c6479` `fmul 16.0`, `0x5c647f` `fadd qword` of the double 0.5, `0x5c6488` `call 0x73ff3f`, `0x5c648d` `fmul`
by the float 0.0625, `0x5c6496` `call 0x73fdf5`. `0x73ff3f` sets the x87 control word to `0x173f`
(rounding down) and `0x73fdf5` to `0x1b3f` (rounding up) through `0x40cf81`, pushed at
`0x73ff4c` and `0x73fe02`, run `frndint` and restore it. The capability record is the one `0x58a230`
returns; what its words mean is `gx`'s. `0x5c63d6` and `0x5c63e0` test the dwords. The function has
one caller, `0x5c5ee0`.

## Decal texture coordinates

`0x5c5ee0` (`ecx` four world points, `edx` four `(u, v)` pairs updated in place, `resX` and `resY`
on the stack, `ret 8`) replaces the pairs with decal coordinates. It projects the points by
`0x5c4d70`, rounds them by `0x5c63d0` and lowers a coordinate by 1 when it is above the centroid's.
It halves `(max u − min
u)·(max v − min v)·resX·resY` and shifts `resX` and `resY` right by the same count until the area is
under 3·|cross|, ending if a size reaches 0. A 3×3 fit through three corners then gives `u = m0·x +
m3·y + m6` and `v = m1·x + m4·y + m7` for each unrounded projected point, over the shifted sizes.

Addresses: `0x5c5ee0`, `0x5c63d0`, `0x5c4d70`, `0x58a230`, `0x7be0e0`, `0x7be2a0`, `0x7bdfc0`, `0x5c61c8`, `0x5c61ce`, `0x5c61f9`, `0x5c6306`, `0x5c6319`, `0x5c632e`
Confidence: verified
Proof: one
Test: none

It returns before the fit when the cross product of `p0 − p1` and `p2 − p1` has length² under 1 (`0x5c61b7`); `0x5c5fd1` copies the 0x38 dwords of the record; `0x5c60cc` to `0x5c60fc` take the area;
`0x5c610d` lowers the coordinates; `0x5c61c8` to `0x5c61ce` is `fsqrt; fadd st,st; fmul 1.5`; the loop
`0x5c61e6` halves the area; `0x5c61f9` is `shr esi,cl`; the texel pairs at `0x5c6253` are offset by 0.5 away
from the box centre; `0x5c6306`, `0x5c6319` and `0x5c632e` take the determinant, inverse and product of
the two 3×3 matrices; the last loop at `0x5c6352` multiplies by the reciprocal sizes. `0x7bdfc0` is `a·b`
with `out[3i + j]` row `i` of `a` by column `j` of `b`, so `[x y 1]` of a rounded corner through the product
gives its texel row: the map is exact at the three corners. The capability words are `gx`'s.

## Ray and sphere

`0x5c53e0` (`ecx` origin, `edx` direction; centre, radius and the output on the stack; `ret 0xc`)
stores the float +∞ at `0xc2b850` to the output, and requires `|dir·dir − 1| < 2⁻¹⁰`, else error 0x57
and 0. With `t = (c − o)·d` it returns 1 and stores `t` when `t ≥ −radius` and the distance² from
`o + t·d` to `c` is at most `radius²`; otherwise 0. `t` is the distance to the closest approach, not
an entry point, and may be negative. A NaN in the origin or centre passes the first test and fails
the second.

Addresses: `0x5c53e0`, `0xc2b850`, `0x5c5424`, `0x5c5463`, `0x5c548a`, `0x5c54a9`, `0x5c54c3`
Confidence: verified
Proof: one
Test: none

`0x5c5424` `fcomp [0x8090d8]` (the float 2⁻¹⁰) `; test ah,5; jnp` passes only a defect below it. The sum for `t`
runs x, z, y at 53 bits (`0x5c5457` to `0x5c5463`). The foot point's x is stored as a float
(`0x5c548a`), its z term `t·dz` too, its y not; the differences from the centre are stored as
floats for x and y (`0x5c549b`, `0x5c54a3`) and not for z, and the squares sum z, y, x. `0x5c54c3`
`fcompp; test ah,1; jne` rejects `dist² > r²`. The function has no caller.

## Ray and triangle

`0x5c5560` (`ecx` origin, `edx` direction; `v0`, `v1`, `v2` and the output on the stack; `ret
0x10`) stores +∞ to the output and applies the same direction test. With `e1 = v1 − v0`, `e2 = v2 −
v0`, `p = d × e2`, `det = e1·p`: it misses for `|det| < 2⁻²²`, and with `iv = RN24(1/det)`, `u = ((o −
v0)·p)·iv` and `q = (o − v0) × e1`, `v = (d·q)·iv`, it hits for `0 ≤ u ≤ 1`, `0 ≤ v` and `u + v ≤ 1`,
storing `t = (e2·q)·iv`. NaN passes every miss test. `t` may be negative.

Addresses: `0x5c5560`, `0x4549f0`, `0x5c5593`, `0x5c564d`, `0x5c5667`, `0x5c5686`, `0x5c56c7`, `0x5c56db`, `0x5c5744`, `0x5c5756`
Confidence: verified
Proof: one
Test: none

`0x4549f0` gives the direction's squared length. `0x5c5667` `fcomp [0x8029d4]` is `2⁻²²` and `jnp`
misses below it, NaN passing. `0x5c5686` stores `iv` as a float. The dot products add z, y, x terms for `u`, y,
x, z for `v` and y, z, x for `t`. `0x5c56db` `fcomp 1.0; test ah,0x41; je` misses `u > 1`; `0x5c56c7`
`fcomp 0.0; test ah,5; jnp` misses `u < 0`; `0x5c5744` is the same for `v`; `0x5c5756` misses `u + v >
1`. Only the two mesh picks call it.

## Mesh ray picks

`0x5c5bf0` and `0x5c57a0` return 1 when a ray hits a triangle mesh and store the lowest hit `t`
and its triangle index. The static pick `0x5c5bf0` (`ecx` origin, `edx` direction; a count, the
vertices, the mode, the index count, the `u16` indices and two outputs; `ret 0x1c`) walks the
triangles of mode 3 (list), 4 (strip) or 5 (fan around `idx[0]`) through `0x5c5560`; the skinned pick
`0x5c57a0` (`ret 0x34`) first blends each vertex from up to four bones, then keeps only `t ≥ 0`. A list
of `n` indices walks `(n − 1)/3 + 1` triangles, so a count that is not a multiple of 3 reads past
the indices. Indices are not checked against the vertex count.

Addresses: `0x5c5bf0`, `0x5c57a0`, `0x5c5560`, `0x7bca80`, `0x596ff0`, `0x409aef`, `0x5c5bd0`, `0xc2b854`, `0xc2b968`, `0xc2b96c`, `0xc2b970`, `0x5c58eb`, `0x5c5902`, `0x5c59a5`, `0x5c5a64`, `0x5c5d98`, `0x5c5bb0`, `0x5c5e0c`
Confidence: verified
Proof: one
Test: none

The skinned pick takes the bone indices, their stride, the weights, their stride, the 64-byte
bone matrices, the vertex count, the vertices and their stride, then the mode, index count, indices
and outputs. Each vertex is `0x7bca80` by bone `idx[0]` times its weight byte, plus the next bones
while the next weight byte is not 0 (four at most), each term and sum stored as float, then
multiplied by the float `0x8026c8`, 1/255. The work buffer `0xc2b970` grows through `0x596ff0` when
the vertex count exceeds `0xc2b96c`; the first call registers `0x5c5bd0`. `0x5c5d98` is the list count
(`mul 0xaaaaaaab; shr 1; inc`). `0x5c5a64` rejects `t < 0`; the static pick has no such test. Each
returns 1 only when the output is not +∞ (`0x5c5bb0`, `0x5c5e0c`).

## Clip outcode

`0x5c5e30` (`ecx` a matrix, `edx` a point) transforms `(x, y, z, 1)` by `0x7bcb40` to `(x', y', z', w')`
and returns six bits, each set when the float's sign bit is: bit 0 `w' − x'`, bit 1 `x' + w'`, bit 2
`w' − y'`, bit 3 `y' + w'`, bit 4 `w' − z'`, bit 5 `z' + w'`, each rounded to float first. A point
inside the clip volume gives 0, apart from −0.0 and NaN with their sign bits. It has no caller.

Addresses: `0x5c5e30`, `0x7bcb40`, `0x5c5e5a`, `0x5c5e77`, `0x5c5e8c`, `0x5c5ea2`, `0x5c5eb8`, `0x5c5ecb`, `0x5c5ed3`
Confidence: verified
Proof: one
Test: none

The sign bits move down through `shr 1` between the masks `0xbfffffff`, `0x9fffffff`, `0x8fffffff`,
`0x87ffffff` and `0x83ffffff`, then `shr 0x1a`. `w' − x'` is shifted five times and `z' + w'` not at all
before that, so the sign of `z' + w'` is bit 5 and that of `w' − x'` bit 0. Each mask keeps the sign
bit and the bits below the position it will land under, so the other bits drop below bit 26.

## Image resample

`0x5c51f0` (`ecx` the source, `edx` its format; source width, height and pitch, the destination,
its format, width, height and pitch on the stack; `ret 0x20`) copies an image to another size.
Both formats must be below 9 as signed integers and both pointers nonzero, else error 0x57.
`stepX = (srcW << 16)/dstW` and `stepY = (srcH << 16)/dstH`, unsigned divides, so a zero
destination size faults. Destination row `r`, column `c` takes the source element at row `(r·stepY)
>> 16` and column `(c·stepX) >> 16`. The element is 4 bytes when both formats are 1, otherwise 2.

Addresses: `0x5c51f0`, `0x5c5201`, `0x5c5222`, `0x5c522b`, `0x5c5239`, `0x5c5251`, `0x5c526b`, `0x5c5285`, `0x5c5309`
Confidence: verified
Proof: one
Test: none

`0x5c5222` and `0x5c522b` test the formats against 1; the 4-byte loop is at `0x5c5280`
(`mov ecx,[edx+ecx*4]`) and the 2-byte one at `0x5c5304` (`mov cx,[edx+ecx*2]`). `0x5c5239`:
`shl eax,0x10; xor edx,edx; div ebx`. Each row restarts the column accumulator at 0. Nothing
calls it.

## Texel buffer callback

`0x5c5350` is a data callback that three places pass to a texture creation function with an 8 by 8
size: `0x44a929` and `0x5996d6` to `0x448450`, `0x4c0d49` to `0x58aa30` (`call` at `0x4c0d5d`). With `ecx` 0 it fills the 64 dwords of the buffer `0xc2b860` with the dword at
`[ebp+0x14]`; with `ecx` 1 it stores `4·edx` to the pointer at `[ebp+0x18]` and the buffer's address
to the pointer at `[ebp+0x1c]`; other values do nothing (`ret 0x18`). The first call clears the
buffer and registers `0x5c53d0`, a bare `ret`, with `0x409aef`.

Addresses: `0x5c5350`, `0x5c53d0`, `0xc2b860`, `0xc2b960`, `0x448450`, `0x58aa30`, `0x44a929`, `0x4c0d49`, `0x5996d6`, `0x409aef`
Confidence: verified
Proof: one
Test: none

`0x5c5357` tests bit 0 of the byte `0xc2b960`, sets it, stores 64 zero dwords (`rep stos`, `ecx =
0x40`) and calls `0x409aef`. `0x5c538d` `sub eax,0; je 0x5c53b1` selects the fill, `dec eax; jne` the
size case. The callers push `0x5c5350` after the sizes 8 and 8 (`0x44a929`, `0x4c0d49`,
`0x5996d6`). What the texture code passes in the other arguments belongs to `gx`.

## Vector helpers

`0x5c6540` stores four floats at `ecx` (`ret 0x10`). `0x5c6560` multiplies the four floats at `ecx`
by a float. `0x5c6590` stores them negated through the pointer on the stack. `0x5c65c0` stores `a − b`
of two vectors of four, `a` in `edx` and `b` on the stack. `0x5c65f0` and `0x5c6620` copy a matrix's
column 2 (`+0x8`, `+0x18`, `+0x28`, `+0x38`) and column 3 (`+0xc`, `+0x1c`, `+0x2c`, `+0x3c`) to the
stack pointer. The plane extraction uses all of them; `0x5c6540` is also called from the world and
scene code.

Addresses: `0x5c6540`, `0x5c6560`, `0x5c6590`, `0x5c65c0`, `0x5c65f0`, `0x5c6620`, `0x680e05`, `0x5c4930`
Confidence: verified
Proof: one
Test: `gxumath::vec_helpers_difftest`

`0x5c6540` has 27 call sites, 16 in `0x5c4930` and 11 elsewhere, the first at `0x680e05`. Each
vector function is `fld`, `fstp` or `fchs`, `fsub` and `fmul` through `st0` with float stores. The
test runs `0x5c6540`.

## Frustum planes and box

`0x5c4930` (`ecx` a matrix, `edx` six planes of four floats) writes `C0 − C3`, `−C0 − C3`, `C1 −
C3`, `−C1 − C3`, `C2 − C3`, `−C2 − C3` for the matrix's columns, each a float, then scales each plane's four
floats by `RN24(1/√(a² + b² + c²))`. `0x5c54f0` (`ecx` a point, `edx` the planes, a margin on
the stack) returns 1 when `py·ply + px·plx + pz·plz + plw − margin` is at most 0 for all six, NaN
passing. `0x5c4c70` (`ecx` view, `edx` projection; minimum and maximum outputs on the stack, `ret 8`)
writes the componentwise bounds of the eight corners `0x5c43b0` gives.

Addresses: `0x5c4930`, `0x5c54f0`, `0x5c4c70`, `0x5c43b0`, `0x5c4c22`, `0x5c4c42`, `0x5c4c50`, `0x5c552b`, `0x5c5539`, `0x5c4cc9`, `0x5c4d10`
Confidence: verified
Proof: one
Test: none

`0x5c494f` to `0x5c4c1f` build the columns with `0x5c6540`, `0x5c6620` and `0x5c65f0` and the planes with
`0x5c65c0` and `0x5c6590`. The loop at `0x5c4c27` runs six times, sums `x² + y² + z²` unrounded
(`0x5c4c42` is `fsqrt`), takes `fdivr 1.0`, stores a float (`0x5c4c50`) and calls `0x5c6560`.
`0x5c5539` `test ah,0x41; je` leaves for `(d − margin) > 0` only. The box tests `c <= min` with
`test ah,0x41; jne` and `c >= max` with `test ah,5; jp` (`0x5c4cc9`, `0x5c4d10`); the sentinel
box starts at corner 0 and a NaN corner replaces the bound. None of the three has a caller.

## Object file and initializers

The initializer run is three slots, `0x82aa38` to `0x82aa40`. `0x5c43a0` copies +∞ into `0xc2b850`,
which the ray code reads; `0x5c6500` stores π + π in `0xc2b964`; `0x5c6520` stores `1/0xc2b964`
in `0xc2b858`. Nothing reads `0xc2b858`, and `0xc2b964` is read only by `0x5c6520`. The first function of the file is `0x5c3cc0`;
the code below it (`0x5c3c60`, `0x5c3c90`, `0x5c3ca0`) is list code of a file that is not identified.
`0x5c5350` is part of the file: it touches only `0xc2b860` to `0xc2b960`, which lie between the
file's own `0xc2b858` and `0xc2b964`, and no font data.

Addresses: `0x5c4390`, `0x5c43a0`, `0x5c64f0`, `0x5c6500`, `0x5c6510`, `0x5c6520`, `0xc2b850`, `0xc2b858`, `0xc2b964`, `0x82aa38`
Confidence: verified
Proof: one
Test: none

Each slot holds a `jmp` thunk to its body 16 bytes on. A scan of the listing and of every dword of
the image for each global finds `0xc2b850` only at `0x5c43a5`, `0x5c53e6`, `0x5c5566`, `0x5c57a6`,
`0x5c5bf6` and `0x5c5e0c`, the others only where named above and in the pick buffer's setup. Slot
`0x82aa44` holds `0x5c6650`, the first initializer of the next file.

## Unreached code

Nothing live reaches `0x5c3e50`, `0x5c4100`, `0x5c4930`, `0x5c4c70`, `0x5c51f0`, `0x5c53e0`,
`0x5c54f0`, `0x5c57a0`, `0x5c5bf0`, `0x5c5e30` and `0x5c5ee0`, or what only they call: `0x5c4d70`,
`0x5c5560`, `0x5c63d0`, `0x5c6470`, `0x5c64b0`, `0x5c5bd0`, `0x5c6560`, `0x5c6590`, `0x5c65c0`, `0x5c65f0` and
`0x5c6620`. Their entries above keep what they do.

Addresses: `0x5c3e50`, `0x5c4100`, `0x5c4930`, `0x5c4c70`, `0x5c51f0`, `0x5c53e0`, `0x5c54f0`, `0x5c57a0`, `0x5c5bf0`, `0x5c5e30`, `0x5c5ee0`, `0x5c4d70`, `0x5c5560`, `0x5c63d0`, `0x5c6470`, `0x5c64b0`, `0x5c5bd0`, `0x5c6560`, `0x5c6590`, `0x5c65c0`, `0x5c65f0`, `0x5c6620`, `0x5c3cc0`
Confidence: verified
Proof: one
Test: none

The census is a scan of every `call`, `jmp` and `jcc` target in the listing and of every dword of
the image for each address. Its control is `0x5c3cc0`, found at six call sites. `0x5c4d70` has one
call, at `0x5c5ff4` in `0x5c5ee0`; `0x5c5560` six, at `0x5c5a58`, `0x5c5adf`, `0x5c5b72`, `0x5c5cdb`,
`0x5c5d49` and `0x5c5dce`, in the two picks; `0x5c63d0` one at `0x5c603a`; `0x5c6470` two at `0x5c63eb`
and `0x5c63f6`; `0x5c64b0` two at `0x5c6432` and `0x5c643d`; `0x5c5bd0` is stored at `0x5c5855` in the
skinned pick; `0x5c6560`, `0x5c6590`, `0x5c65c0`, `0x5c65f0` and `0x5c6620` are called only from
`0x5c4930`.
