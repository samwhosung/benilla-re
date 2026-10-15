# cmath: matrices

The matrix routines of the Tempest math library: sums, products and inverses of 4×4 matrices, 3×3
matrices and the 3×4 rotation-with-translation form, points and vectors through them, and the
rotations they are built from. A 4×4 is 16 floats, row-major: entry (row r, column c) is float
`4r + c`, a point is a row vector the matrix multiplies on the right, and the translation is `m12`
to `m14` (`0x7bcb40` gives `x·m0 + y·m4 + z·m8 + w·m12` first). A 3×3 is nine floats, `3r + c`. A
3×4 is the nine floats of a 3×3 followed by the translation `m9` to `m11`. Every result float is one
x87 expression at 53 bits stored once as a float, and each output sums its products in an order
of its own, which the transcriptions in `crates/cmath` keep.

Roots: `0x7bc6a0`, `0x7bca80`, `0x7bcb40`, `0x7bcc60`, `0x7bcf90`, `0x7bd6c0`, `0x7bd700`, `0x7bd820`,
`0x7bdb00`, `0x7bdc40`, `0x7bdd00`, `0x7bdd60`, `0x7bddb0`, `0x7bdfc0`, `0x7be490`, `0x7bf4b0`,
`0x7bf700`, `0x7bf860`

## Header initializers

Four object files hold the matrix routines, and the C runtime's run `0x82da08` to `0x82da34`
lists their header initializers, each a +∞, a 2π and a 1/2π. The 3×4 routines and the colour
conversions have no owner yet; the two files the `cmath` system owns follow.

| file | +∞ | 2π | 1/2π | holds |
|---|---|---|---|---|
| 3×4 matrices | `0x7baaa0` | `0x7bbc30` | `0x7bbc50` | `0x7bae60`, `0x7bb860` |
| colour | `0x7bbd40` | `0x7bbff0` | `0x7bc010` | `0x7bbd60`, `0x7bbec0` |
| 4×4 matrices | `0x7bc090` | `0x7bdf00` | `0x7bdf20` | `0x7bc6a0`, `0x7bcf90` |
| 3×3 matrices | `0x7be0c0` | `0x7bf630` | `0x7bf650` | `0x7bdfc0`, `0x7bf4b0` |

Addresses: `0x7baaa0`, `0x7bbc30`, `0x7bbc50`, `0x7bbd40`, `0x7bbff0`, `0x7bc010`, `0x7bc090`, `0x7bdf00`, `0x7bdf20`, `0x7be0c0`, `0x7bf630`, `0x7bf650`, `0x82da08`, `0x7baab0`, `0x7bbc40`, `0x7bbc60`, `0x7bbd50`, `0x7bc000`, `0x7bc020`
Confidence: inferred
Proof: none
Test: none

Each slot holds a `jmp` thunk to a body 16 bytes on: 11 bytes that copy +∞, 15 that store π + π,
19 that store `1/(π + π)`. The slots are consecutive, three to a file. A file's code lies among and
around its thunks, so a function's file is inferred from its position; `0x7bdf50` and `0x7bdfc0`
precede the 3×3 file's +∞ thunk and `0x7bc040` the 4×4 file's.

## Sum, scalar and product

`0x7bc290` (`ecx` out, `edx` a, `b` on the stack, `ret 4`) stores `a[i] + b[i]` for the 16 entries;
`0x7bc8e0` (`ecx` out, `edx` m, `s` on the stack) stores `s·m[i]`; `0x7bc1a0` (`ecx` a, `edx` b) returns
in `al` whether any pair differs under the float compare, so −0 equals 0 and NaN differs.
`0x7bc6a0` (`ecx` out, `edx` a, `b` on the stack) stores the product `a·b`, `out[4i + j]` row `i` of `a`
by column `j` of `b`, and returns `out`; `0x7bc8c0` gives entry 0 and `0x7bce30` (`ecx` A, `B` on the stack)
sets `A = A·B` in place and returns `A`. `0x7bdf50` stores sixteen floats from the stack.

Addresses: `0x7bc290`, `0x7bc8e0`, `0x7bc1a0`, `0x7bc6a0`, `0x7bc8c0`, `0x7bce30`, `0x7bdf50`, `0x7bce43`, `0x7bc8a7`, `0x7bc8b2`
Confidence: verified
Proof: one
Test: `cmath::mat4_add`, `cmath::mat4_mul_scalar`, `cmath::mat4_ne`, `cmath::multiply_matrix4_difftest`, `cmath::dot_leaf_difftest`

Each sum and product is one `fld; fadd|fmul; fstp dword`, 16 times. `0x7bc8a7` is the call to
`0x7bc8c0` in `0x7bc6a0`, which sums `((b12·a3 + b8·a2) + a1·b4) + a0·b0` with `a` the first row, and
`0x7bc8b2` the call to the copy constructor `0x7bdf50`. `0x7bce43` is the call from `0x7bce30`, which
copies the 16 entries back (the loop at `0x7bce50`) and returns `A`. `0x7bc6a0` has 47
call sites. The tests run each whole.

## Points and vectors through a matrix

`0x7bca80` (`ecx` out, `edx` point of three, matrix on the stack) gives the point times the upper 3×3 plus
row 3: `x = ((m8·z + m4·y) + x·m0) + m12`, `y = ((m9·z + m1·x) + m5·y) + m13`, `z = ((m10·z + m2·x) + m6·y)
+ m14`, each stored as a float; `0x7bcc60` is the same in place. `0x7bcb40` does it for four components
(`w` first): `x' = ((m12·w + m8·z) + m4·y) + x·m0`, `y'`, `z'` and `w'` from columns 1 to 3. `0x7bcae0` (`ecx` out, `edx`
a 4×4, vector on the stack) gives `row·v + row[3]` for each of the 4×4's first three rows, the product with its
transpose (`cmath/types#matrices`).

Addresses: `0x7bca80`, `0x7bcc60`, `0x7bcb40`, `0x7bcae0`, `0x7bcb4c`
Confidence: verified
Proof: one
Test: `cmath::transform_point4`, `cmath::transform_vec4`, `cmath::transform_mat43_vec3_difftest`

`0x7bca80` has 140 call sites, `0x7bcc60` 31, `0x7bcb40` 25, `0x7bcae0` 6. The order of the additions is the order of
the `fld; fmul; faddp` groups; `0x7bcb4c` is the first, the `w` row of `0x7bcb40`. The tests run the
instructions whole and compare every output.

## Translate, scale and rotate

`0x7bdc40` (`ecx` the matrix, `t` of three on the stack) adds `t0·row0 + t1·row1 + t2·row2` of the 3×3 to
row 3: `m12 = ((m8·t2 + m4·t1) + t0·m0) + m12`, likewise `m13`, `m14`. `0x7bdca0` multiplies the first three
entries of rows 0 to 2 by the three scalars `s[r]` at the pointer; `0x7bdd00` by one scalar; entry 3 of each row and row 3 stay.
`0x7bdd60` (`ecx` M, angle, axis pointer and a unit flag on the stack, `ret 0xc`) builds `0x7bdb00`'s rotation
and sets `M = R·M`; `0x7bddb0` does the same with the rotation matrix of a quaternion.

Addresses: `0x7bdc40`, `0x7bdca0`, `0x7bdd00`, `0x7bdd60`, `0x7bddb0`, `0x7bdd7b`, `0x7bdd85`
Confidence: verified
Proof: one
Test: `cmath::translate_matrix_difftest`, `cmath::mat34_scale3_rows`, `cmath::mat34_scale3`, `cmath::quat_mul4`

`0x7bdd7b` calls `0x7bdb00`, `0x7bdd85` calls `0x7bc6a0` with the rotation first, then 16 entries are
copied back. The scale functions are `fld s; fmul [m]; fstp` per entry, nine times. The quaternion form
`0x7bddb0` is documented at `models/animation#texture-transforms`. `0x7bdc40` has 52 call sites,
`0x7bdd60` 31, `0x7bdd00` 15, `0x7bdca0` 13.

## Determinants and inverses

`0x7bc040` (nine floats on the stack, `ret 0x24`) leaves the 3×3 determinant in `st0` at 53 bits,
`m1·m5·m6 + m2·m3·m7 + m0·m4·m8 − m2·m4·m6 − m1·m3·m8 − m0·m5·m7`. `0x7bcf90` expands the 4×4 along row 0,
`m0·D0 − m1·D1 + m2·D2 − m3·D3` with `Dj` that determinant of the minor without row 0 and column `j`, each
partial sum stored as a float. `0x7bd390` stores the adjugate: entry `4i + j` is `(−1)^(i+j)` times the
minor without row `j` and column `i`. `0x7bd6c0(m, out, s)` stores `RN24(1/s)` times each adjugate entry;
nothing tests `s` for 0.

Addresses: `0x7bc040`, `0x7bcf90`, `0x7bd390`, `0x7bd6c0`, `0x7bcfb8`, `0x7bd6d8`, `0x7bd6e4`
Confidence: verified
Proof: one
Test: `cmath::det3`, `cmath::det4`, `cmath::adj4t`, `cmath::mat4_inverse_from_det`

`0x7bc040` has 36 call sites: 4 in `0x7bcf90` (the first at `0x7bcfb8`), 16 in `0x7bd390` and 16 in
the unreached `0x7bd060`. The adjugate keeps each 3×3 determinant in the register and rounds
once. `0x7bd6c0` calls `0x7bd390` (`0x7bd6d8`) and `0x7bc8e0` (`0x7bd6e4`).

## Rotation and affine inverses

`0x7bd700(m, out)` stores the inverse of a rotation with a translation: the transposed 3×3 with the
negated translation carried through it by `0x7bdc40`. `0x7bd820(m, out, s)` does the same, but when `|s −
1| ≥ 2⁻²⁰` it first scales every entry of the transposed 3×3 by `RN24(1/(s·s))` through `0x7bdd00`. Row 3
of the output starts as `(0, 0, 0, 1)`, and the fourth column as 0.

Addresses: `0x7bd700`, `0x7bd820`, `0x8026bc`, `0x7bd838`, `0x7bd843`, `0x7bd849`, `0x7bdc40`, `0x7bdd00`
Confidence: verified
Proof: one
Test: `cmath::mat4_affine_invert`, `cmath::mat4_affine_invert_uscale`

`0x7bd838` compares `|s − 1|` with the float `0x8026bc` (2⁻²⁰) and `jp` (`0x7bd843`) takes the scaling
path for a larger value or NaN; `0x7bd849` calls `0x7bd700` otherwise. `0x7bd700` has 4 call sites
and `0x7bd820` 12. The tests run both whole.

## Axis and angle rotations

`0x7be490(out, axis, angle, unit)`, `0x7bdb00` and `0x7bb860` build the rotation about an axis for a row
vector. Unless the flag says the axis is a unit vector, it is scaled by `1/√((z² + y²) + x²)`, each
component stored as a float. With `c`, `s` and `t = 1 − c` stored as floats, the 3×3 is, by row:

| row | column 0 | column 1 | column 2 |
|---|---|---|---|
| 0 | `x·x·t + c` | `t·x·y + z·s` | `t·x·z − y·s` |
| 1 | `t·x·y − z·s` | `y·y·t + c` | `x·s + t·y·z` |
| 2 | `t·x·z + y·s` | `t·y·z − x·s` | `z·z·t + c` |

`0x7bdb00` puts it in a 4×4 with `(0, 0, 0, 1)` last, `0x7bb860` in a 3×4 with zero translation;
`0x7be730` and `0x7bbab0` multiply a 3×3 or a 3×4 on the left by one.

Addresses: `0x7be490`, `0x7bdb00`, `0x7bb860`, `0x7be730`, `0x7bbab0`, `0x7bdfc0`, `0x7bae60`
Confidence: verified
Proof: one
Test: `cmath::axis_angle_mat3`, `cmath::axis_angle_mat4_difftest`, `cmath::axis_angle_mat34`

All three are `fsincos` of the angle at 53 bits stored as floats; the products with `t` stay in the
register except `t·x·z`, which is stored once as a float for entry (2,0). `0x7be730` calls `0x7be490` then
`0x7bdfc0` with the rotation first and copies nine entries back; `0x7bbab0` calls `0x7bb860` then `0x7bae60`.
`0x7bdb00` has 4 call sites. `fsincos` is the x87
transcendental, so the tests compare it with the host's `sin` and `cos` structurally.

## Three by three matrices

`0x7bdfc0` (`ecx` out, `edx` a, `b` on the stack) stores the product `a·b`, `out[3i + j]` row `i` of `a`
by column `j` of `b`. `0x7be0e0` leaves the determinant by the rule of Sarrus in `st0` at 53 bits. `0x7be1e0` stores the
transposed cofactor matrix. `0x7be2a0(m, out, s)` stores `RN24(1/s)` times each of its entries; nothing tests `s`. `0x7be6d0` (`ecx` m, `v`
of three on the stack) multiplies the entries of row `r` by `v[r]` in place; `0x7be670` does so with three scalars on the
stack.

Addresses: `0x7bdfc0`, `0x7be0e0`, `0x7be1e0`, `0x7be2a0`, `0x7be6d0`, `0x7be670`
Confidence: verified
Proof: one
Test: `cmath::multiply_matrix3_difftest`, `cmath::det3b`, `cmath::adj3t`, `cmath::inv3`, `cmath::mat3_scale_rows`, `cmath::mat3_scale_rows3`

The nine outputs of `0x7bdfc0` sum their three products in nine different orders. `0x7be0e0` (`0x7be0e0` to `0x7be11c`) sums
`m6·m1·m5 + m2·m3·m7 + m8·m0·m4 − m6·m2·m4 − m8·m1·m3 − m0·m7·m5` in that order. The adjugate entries are
differences of two products, negated at odd positions. `0x7bdfc0` has 18 call sites, `0x7be2a0` 2 and `0x7be6d0` 6.

## Yaw, pitch and roll

`0x7bf4b0(out, yaw, pitch, roll)` builds a 3×3 basis as the product `Rz(yaw)·(Ry(pitch)·Rx(roll))`, each factor from the
`fsincos` of its angle, and stores it transposed through the 3×3 constructor `0x5f8d20`. Its first row is
`(cos yaw·cos pitch, sin yaw·cos pitch, −sin pitch)` whatever the roll.

Addresses: `0x7bf4b0`, `0x7bdfc0`, `0x5f8d20`
Confidence: verified
Proof: one
Test: `cmath::euler_to_mat3_f`

The two products go through `0x7bdfc0`. `0x7bf4b0` has two callers, `0x50ab62` and `0x60a184`. Its
view-side meaning is at `uicamera/view#pitch-units-and-sign`.

## Axis index and planes

`0x7bf700` (`ecx` a vector) returns the index of the component of least magnitude: 0 when `|x| < |y|` and `|x| <
|z|`, 2 when `|x| < |y|` and `|x| ≥ |z|`, else 2 when `|z| < |y|` and 1 otherwise; a NaN falls to the later index.
`0x7bf860(plane, p0, p1, p2)` copies the three points to `+0x10` to `+0x30` and writes the unit normal of `(p1 − p0)
× (p2 − p0)` and `d = −(n·p0)` first; there is no test for a zero length. `0x7bf7b0(plane, s)` stores the record
`(0, 0, 1, −s)` with every point coordinate `s`; its fourth float is `(s + s)·0 − s`, NaN for infinite `s`.

Addresses: `0x7bf700`, `0x7bf860`, `0x7bf7b0`, `0x7bf71d`, `0x7bf744`, `0x7bf680`
Confidence: verified
Proof: one
Test: `cmath::minor_axis_difftest`, `cmath::plane_from_3pts`, `cmath::scalar_bcast_struct13`

`0x7bf71d` `fcomp [|y|]; test ah,5; jp 0x7bf744` takes the `|x| ≥ |y|` leg, where `0x7bf744`
compares `|z|` with `|y|`: `eax = 2` when `|z| < |y|`, else 1. In the other leg `0x7bf72a fcomp st(1)`
compares `|x|` with `|z|` and `jp` returns 2. `0x7bf700` has one caller, `0x7bbc90`. The dominant-axis
partner `0x7bf680` is in `cmath/geometry#dominant-axis-and-bounds`; the 13-float record is the plane
the collision facets use.

## Three by four matrices

A 3×4 is the form of a rotation with a translation. `0x7bae60(out, a, b)` stores `a·b`: the 3×3 product, and
the translation `t_a·B + t_b` with `t_b` added last. `0x7bb0e0(out, point, matrix)` stores the point times the 3×3 plus the
translation. `0x7bb990(m, v)` adds the 3×3 applied to `v` into the translation `m9` to `m11` of `m`. Their file
is not identified; the rows stay unassigned.

Addresses: `0x7bae60`, `0x7bb0e0`, `0x7bb990`
Confidence: verified
Proof: one
Test: `cmath::affine_mul34`, `cmath::transform_point34`, `cmath::apply_translation3`

Each output is one 53-bit expression stored as a float, its additions in the order the exe loads
them. `0x7bae60` has 3 call sites, `0x7bb0e0` 1 (`0x7a4f8f`) and `0x7bb990` 5.

## Unreached matrix code

Nothing live reaches these routines, which exist for the same shapes as the live ones: the 3×4
compare `0x7baac0`, sum `0x7bab70`, scalar sums `0x7bac10`, `0x7baca0`, difference `0x7bad30`, scalar difference `0x7badd0`, scalar
products `0x7bafc0`, `0x7bb050`, `0x7bb9f0`, `0x7bba50`, vector and matrix products `0x7bb140`, `0x7bb1a0`, `0x7bb220`,
`0x7bb290`, `0x7bb300`, builders `0x7bb4e0`, `0x7bb5d0`, `0x7bb710`, by-value wrappers `0x7bb3a0` to `0x7bb4a0`,
the quaternion form `0x7bbb00`; the 4×4 compare `0x7bc0b0`, scalar sums `0x7bc360`, `0x7bc430`, difference `0x7bc500`,
scalar difference `0x7bc5d0`, scalar products `0x7bc9b0`, `0x7bccd0`, vector product `0x7bcbd0`, wrappers `0x7bcdb0`, `0x7bcdf0`, `0x7bce70`, `0x7bceb0`,
transpose `0x7bcef0`, adjugate `0x7bd060`, builder `0x7bd980`; the 3×3 forms `0x7be120`, `0x7be320`, `0x7be3f0`, `0x7be5b0`,
`0x7be610`, `0x7be770`, `0x7be860`, `0x7be890`, `0x7be8c0`, `0x7be8f0`; the Euler extractors `0x7be930`, `0x7be9e0`, `0x7bea80`,
`0x7beb20`, `0x7bebd0`, `0x7bec80` and builders `0x7bed30`, `0x7beeb0`, `0x7bf030`, `0x7bf1b0`, `0x7bf330`.

Addresses: `0x7baac0`, `0x7bab70`, `0x7bac10`, `0x7baca0`, `0x7bad30`, `0x7badd0`, `0x7bafc0`, `0x7bb050`, `0x7bb9f0`, `0x7bba50`, `0x7bb140`, `0x7bb1a0`, `0x7bb220`, `0x7bb290`, `0x7bb300`, `0x7bb4e0`, `0x7bb5d0`, `0x7bb710`, `0x7bb3a0`, `0x7bb3e0`, `0x7bb420`, `0x7bb460`, `0x7bb4a0`, `0x7bbb00`, `0x7bc0b0`, `0x7bc360`, `0x7bc430`, `0x7bc500`, `0x7bc5d0`, `0x7bc9b0`, `0x7bccd0`, `0x7bcbd0`, `0x7bcdb0`, `0x7bcdf0`, `0x7bce70`, `0x7bceb0`, `0x7bcef0`, `0x7bd060`, `0x7bd980`, `0x7be120`, `0x7be320`, `0x7be3f0`, `0x7be5b0`, `0x7be610`, `0x7be770`, `0x7be860`, `0x7be890`, `0x7be8c0`, `0x7be8f0`, `0x7be930`, `0x7be9e0`, `0x7bea80`, `0x7beb20`, `0x7bebd0`, `0x7bec80`, `0x7bed30`, `0x7beeb0`, `0x7bf030`, `0x7bf1b0`, `0x7bf330`, `0x7bc040`
Confidence: inferred
Proof: none
Test: none

The census is a scan of every `call`, `jmp` and `jcc` target in the listing and of every dword of
the image for each address; the control is `0x7bc040`, found at 36 call sites. The only callers of
these routines are other routines of the list (the wrappers call the sum, difference and product; the
builders call `0x7bb990`, `0x7be6d0` and `0x7bdca0`). Their names come from the operations in
their bodies: the compares are 12 or 16 `fcomp`, the sums 12 or 16 `fadd`, the scalar sums and products one `fld [ebp+0x8]`
per entry with `fadd` or `fmul` of the entry of `edx`, the scalar differences the entry then `fsub [ebp+0x8]`
(`0x7badd6`, `0x7bc5d6`), each stored to `ecx`, the Euler builders three
`fsincos` and two `0x7bdfc0` products, the extractors `fpatan` pairs and the C runtime's `asin`; the
semantics are not read through.
