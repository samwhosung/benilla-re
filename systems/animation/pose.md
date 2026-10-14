# animation: bone matrices

How the bone loop turns a bone's clocks into its matrix. For each bone it picks the matrix of its
parent, edits that matrix for the bone's ignore-parent flags, builds the bone's own matrix from its
rotation, scale and translation tracks, multiplies the two into the matrix palette at instance `+0x94`,
and then hands the result to the billboard stage (`billboards`). The code is `0x71492d` through `0x7151f9`
of `CM2Model::Animate`. Matrices are row-major 4×4 floats and a point is a row vector, so
a product `a · b` applies `a` first. `models/animation#bone-transform` states the result in short; this
doc gives the order of the steps and the rounding.

Roots: none

## Parent matrix

A bone's flags are `[state+0xf4] | [bone+0x4]`. The parent word `[bone+0x8]` picks the matrix the bone
composes with: a root (0xffff) uses the instance's world matrix `+0xfc` and skips the ignore-parent
legs whatever its flags; any other bone uses the palette entry `[+0x94] + parent·0x40`. When flags bits
`0x7` are clear the entry is used in place. When any is set, the 16 dwords are copied to the working
matrix on the frame (`ebp-0x70`) and the legs below edit the copy, which then takes the entry's place.

Addresses: `0x71492d`, `0x714930`, `0x714936`, `0x714938`, `0x714940`, `0x714943`, `0x714945`, `0x71494e`, `0x714953`, `0x714961`, `0x714967`, `0x71496d`, `0x714980`, `0x714996`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The flags are stored at `ebp+0x14` (`0x714940`), the source pointer at `ebp+8`. `0x714943` sends a root to
`0x714d0f` with the world matrix as source. `0x714961 test cl,7` and `0x714967 je 0x714d0f` skip the
legs for the other bones. The copy loop at `0x714980` moves 8 qwords. The pivot of the next entry is
computed from the copy before any leg edits it.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Pivot in parent space

For a bone with flags `& 7`, the pass first finds where the bone's pivot `b` (the bone record's `+0x60`,
`+0x64`, `+0x68`) lies in the parent matrix `M`: `p[j] = f32(((M[0][j]·b0 + M[1][j]·b1) + M[2][j]·b2) + M[3][j])`
for j = 0, 1, 2, summed at 53 bits and narrowed once. After the legs have edited `M`, the translation row
is the world translation `+0x12c`, `+0x130`, `+0x134` when flag 1 is set. Otherwise it is
`M[3][j] = f32(p[j] − ((M[0][j]·b0 + M[1][j]·b1) + M[2][j]·b2))`, which keeps the pivot where it was.

Addresses: `0x714998`, `0x7149c0`, `0x7149df`, `0x7149fe`, `0x714c8c`, `0x714c90`, `0x714c92`, `0x714caf`, `0x714cc5`, `0x714ccb`, `0x714ce4`, `0x714cea`, `0x714d03`, `0x714d09`, `0x714d0c`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The x sum at `0x714998` is written `M[1][0]·b1 + M[0][0]·b0`, then `M[2][0]·b2`: the same grouping with
the first pair swapped, which is exact. `0x714c8c test byte [ebp+0x14],1` selects: set copies the three
dwords from `0x714c92`, clear computes the subtraction with `fsubr` against the stored `p`. The new
translation is stored at `ebp-0x40`, `ebp-0x3c` and `ebp-0x38`, the working matrix's row 3.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Ignore-parent legs

With flags `& 6` equal to 2, each of the three rows of the working matrix is replaced by the row divided
by its length when the length is not below 2^-22. With 4, row `k` becomes the world matrix row `k`
(`+0xfc`, `+0x10c`, `+0x11c`) times `sqrt(|row k|² / |world row k|²)`, or times 1.0 when
`|world row k|² ≤ 1e-5` or is not a number. With 6, the rows are the world matrix rows. With 0, the rows are
unchanged.

Addresses: `0x7149a7`, `0x7149ad`, `0x7149b0`, `0x714a04`, `0x714a0d`, `0x714a12`, `0x714a18`, `0x714a6e`, `0x714a96`, `0x714ab9`, `0x714abb`, `0x714abf`, `0x714bd6`, `0x714bdb`, `0x714be3`, `0x714bf4`, `0x714bf6`, `0x714c8c`, `0x4549f0`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The squared lengths of leg 4 are summed as `(x² + y²) + z²` for world rows 0 and 1, `(y² + z²) + x²` for
world row 2, `(x² + z²) + y²` for working row 0 and `(z² + y²) + x²` for working rows 1 and 2.
`and eax,6; sub eax,2; je` selects 2, a second `sub eax,2; je` selects 4, a third `jne` leaves 6 to
fall into `0x714a18`. The rows of 2 use `0x4549f0` (`(x² + y²) + z²`), `fsqrt`, `fabs` and
`fcomp 2^-22; test ah,5; jnp`, which skips only when the length is below 2^-22: a length equal to it or
not a number is normalized. The scale `1/len` is `fdivr 1.0` at 53 bits. Leg 4 compares `fcom 1e-5; test ah,0x41; jne`
and divides with `fdivrp` (`local / world`) before `fsqrt`; the ratio stays at 53 bits for the row's stores.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Local matrix route

The bone's flags `& 0x280` pick how the palette entry is made. When they are 0 the palette entry is a copy
of the parent matrix and nothing else is built. When either bit is set, the pass builds the bone's local
matrix from its tracks in the order rotation, scale, override matrix, translation, and sets the palette
entry to the local matrix times the parent matrix, `0x74a7c0(palette entry, local, parent)`. Both routes
then enter the billboard stage.

Addresses: `0x714d0f`, `0x714d11`, `0x714d16`, `0x714d19`, `0x7151ba`, `0x7151bf`, `0x7151c4`, `0x7151d8`, `0x7151e0`, `0x7151f9`, `0x74a7c0`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x714d19 je 0x7151c4` takes a clear `& 0x280`. `0x7151c4` copies the source (`ebp+8`) into the entry with
8 qword moves. The stored `& 0x280` at `ebp-0x18` is read again by the first billboard case
(`billboards`). Bit `0x80` is also the override gate below. The local matrix is built at `ebp-0xe4`
and its translation row at `ebp-0xb4`.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Rotation leg

With a rotation key count `[bone+0x34]` of 0 the local matrix is the identity. Otherwise, when the count
exceeds the first-update gate, the leg samples the rotation track `[bone+0x28]` into the state at `+0x30`
(quaternion at `+0x3c`, `tracks`). Then, with the sample just taken or the one stored, it builds the
matrix from the quaternion `(x, y, z, w)` by `0x74b6b5(out, q)`: row 0 is `(1 − 2yy − 2zz, 2xy + 2zw,
2xz − 2yw, 0)`, row 1 `(2xy − 2zw, 1 − 2xx − 2zz, 2yz + 2xw, 0)`, row 2 `(2xz + 2yw, 2yz − 2xw,
1 − 2xx − 2yy, 0)` and row 3 `(0, 0, 0, 1)`.

Addresses: `0x714d1f`, `0x714d24`, `0x714d26`, `0x714d2c`, `0x714d3a`, `0x714d3f`, `0x714d4a`, `0x714d57`, `0x714ded`, `0x74b6b5`, `0x74b6bb`, `0x74b6c6`, `0x74b726`, `0x74b74a`, `0x74b77f`, `0x74b796`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`, `animation::m2_quat_to_mat4_difftest`

`0x74b6bb` loads 2.0 from the image and forms `2x` and `2y` as floats, `2xw`, `2yw`, `2zw` and `2zz` as
exact 53-bit products, and `2xx`, `2xy`, `2xz`, `2yy`, `2yz` narrowed to floats. Each off-diagonal element
is one float plus one 53-bit product, narrowed on store. Elements 0 and 5 subtract the 53-bit `2zz`
from `1 − f32(2yy)` and `1 − f32(2xx)`. Element 10 is `f32(f32(1 − f32(2xx)) − f32(2yy))`
(`0x74b74a` spills the first difference). This is the default routine; the table at `0x876594` holds
it unless a 3DNow set replaces it (`models/animation#dispatched-matrix-math`).
`animation::m2_animate_difftest` runs `0x714260` whole. `animation::m2_quat_to_mat4_difftest` runs
`0x74b6bb` whole.

## Scale and override legs

With a scale key count `[bone+0x50]` of 0 the scale leg does nothing. Otherwise, when the count exceeds
the gate, it samples the scale track `[bone+0x44]` into the state at `+0x68` (vector at `+0x74`), and in
both cases multiplies the local matrix rows 0, 1, 2 by the stored vector's x, y, z with `ScaleMatrixRows`.
Then, when flags bit `0x80` is set and the state's override pointer `+0xf0` is not null, the local matrix
becomes `local · override` through `0x74a7c0(local, local, override)`.

Addresses: `0x714df7`, `0x714dfc`, `0x714e02`, `0x714e08`, `0x714e0e`, `0x714e29`, `0x714f6a`, `0x714f74`, `0x714f7f`, `0x714f81`, `0x714f83`, `0x714f8b`, `0x714f9c`, `0x7bdca0`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x714dfc je 0x714f7f` skips the scale for a count of 0; `0x714e08 jbe 0x714f6a` skips only the sampling
at the gate. `0x714f6a` always runs for a nonzero count. The override test is `0x714f7f test cl,cl` with
`jns`, the sign bit of the flags' low byte, then `cmp [esi+0xf0],0`. A step track copies the value and
jumps to `0x714f6a` without the blend leg (`0x714e5c`).
`animation::m2_animate_difftest` runs `0x714260` whole.

## Translation leg

The translation row of the local matrix is built from a base `b` that starts as the pivot. With a
translation key count `[bone+0x18]` of 0 the base is the pivot. Otherwise, when the count exceeds the
gate, the leg samples the translation track `[bone+0xc]` into the state at `+0x00` (vector at `+0xc`), and
the base is the pivot plus the stored vector: x and y are narrowed to floats, z stays at 53 bits.
The local row 3 is `f32(b[j] − ((L[2][j]·p2 + L[1][j]·p1) + L[0][j]·p0))` for j = 0, 1, 2, `L` the
local matrix after scale and override and `p` the pivot.

Addresses: `0x714fa1`, `0x714faf`, `0x714fc0`, `0x714fe1`, `0x71500b`, `0x715107`, `0x71510d`, `0x715116`, `0x715119`, `0x715121`, `0x715124`, `0x715160`, `0x715163`, `0x715188`, `0x71518b`, `0x7151b0`, `0x7151b2`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x714fa1` loads the pivot into `ebp-0x30` through `ebp-0x28`. A step track copies the value and jumps
to `0x715107` without the blend leg. `0x715107` and `0x715110` add and `fstp` x and y; `0x715119` adds z
and falls into `0x715124` with the sum still on the x87 stack, where `0x7151b0 fsubr st,st(1)` uses it.
The no-key path (`0x715121`) loads z from memory. The sums at `0x715124`, `0x715169` and `0x715191` are
`m2·p2`, `m1·p1`, `m0·p0` in that order, added in that order.
`animation::m2_animate_difftest` runs `0x714260` whole.
