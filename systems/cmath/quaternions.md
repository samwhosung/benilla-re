# cmath: quaternions

The quaternion functions of the Tempest math library, in `0x7c0120` through `0x7c1370`. A
quaternion is four floats `(x, y, z, w)`; a function that returns one writes it through a pointer.
Only the matrix conversion is called by live code; the rest are in the binary and reached by
nothing, and each entry says which.

Roots: `0x7c0120`, `0x7c0190`

## Matrix to quaternion

`0x7c0190(m)` converts a 3×3 row-major rotation matrix (nine packed floats, `m[r*3+c]`) to a
quaternion in `ecx`. With the trace
`t = (m11 + m00) + m22` above 0: `s = √(t + 1)`, `w = 0.5·s`, `k = 0.5/s` and
`x = (m21 − m12)·k`, `y = (m02 − m20)·k`, `z = (m10 − m01)·k`. Otherwise `i` is the index of the largest
diagonal entry, the later index winning only when strictly greater, `j = next[i]`, `k = next[j]` with
`next = {1, 2, 0}`; `s = √(1 + m_ii − m_jj − m_kk)`, `q[i] = 0.5·s`, `w = (m_kj − m_jk)·(0.5/s)`,
`q[j] = (m_ij + m_ji)·(0.5/s)`, `q[k] = (m_ik + m_ki)·(0.5/s)`.

Addresses: `0x7c0120`, `0x7c0190`, `0x7c019d`, `0x7c01a2`, `0x7c01ad`, `0x7c01b5`, `0x7c01f0`, `0x7c0210`, `0x7c021f`, `0x7c0226`, `0x7c024d`, `0x7c0253`, `0x7ffa24`, `0x7ffd74`, `0x7ff9d8`, `0x81d994`
Confidence: verified
Proof: one
Test: `cmath::matrix_quat_difftest`

`0x7c0197` to `0x7c019f` form the trace as `(m[4] + m[0]) + m[8]`; `0x7c01a2 fcom 0.0; test ah,0x41; jne
0x7c01f0` sends a trace of 0 or less, and NaN, to the diagonal leg. The diagonal leg compares `m11` with `m00`
(`0x7c01f7 fcomp [edx]`, `jne` keeps 0 unless strictly greater) then `m22` with the larger. The table
`0x81d994` is the dwords 1, 2, 0 and is read twice, `0x7c021f` for `j` and `0x7c0226` for `k`. Entry
`0x7c0120(m)`, for the transposed layout, loads the entries at `+0`, `+0xc`, `+0x18`, `+0x4`, `+0x10`, `+0x1c`, `+0x8`, `+0x14`, `+0x20`
into a nine-float local and passes its address (`0x7c0161`). `0x7c0120` is reached through three call
sites and `0x7c0190` through it.

## Axis angle and conjugate

`0x7c02c0(angle, axis)` builds `(sin(a/2)·axis, cos(a/2))` in `ecx`, the half angle rounded to a float
before `fsincos`. `0x7c0320(angleOut, axisOut)` reads `this` as a quaternion: with `x² + y² + z²`
above 0 it writes the angle `2·acos(w)` and the axis `v/|v|`, and otherwise the angle 0 and the axis
`(1, 0, 0)`; `w` is not clamped. `0x7c03b0(out)` writes the inverse `(−x, −y, −z, w)/(x² + y² + z² + w²)`,
and the identity `(0, 0, 0, 1)` when that sum is below 2⁻²². `0x7c1340(out)` writes the conjugate
`(−x, −y, −z, w)`. None is called.

Addresses: `0x7c02c0`, `0x7c02cc`, `0x7c02e7`, `0x7c02f3`, `0x7c0320`, `0x7c0348`, `0x7c0353`, `0x7c0358`, `0x741820`, `0x7c038d`, `0x7c03b0`, `0x7c03e3`, `0x7c03f1`, `0x7c0434`, `0x7c1340`, `0x8029d4`
Confidence: verified
Proof: one
Test: none

`0x7c02cc fmul 0.5` and `0x7c02db fstp` round the half angle. `fsincos` leaves the cosine on top: it goes to
`[ebp-4]` and then to `+0xc`, the sine over the angle slot, and multiplies the axis. `0x7c0320` forms the
sum with three products, `fcomp 0.0; test ah,0x41; jne 0x7c038d` sends 0, negative and NaN to the
default, and `0x741820` is the C runtime `acos`. `0x7c03b0 fabs; fcomp 2⁻²² (0x8029d4); test ah,5; jnp
0x7c0434` takes the identity below the bound; the inverse multiplies by `1/sum` and negates x, y and z.

## Exponential and logarithm

`0x7c0450(out)` maps a rotation vector `v = (x, y, z)` (the first three floats of `this`) to the
quaternion `(k·v, cos θ)` with `θ = |v|` and `k = sin θ/θ`; when `|sin θ|` is below 2⁻²¹ (`0x81d9a0`)
it writes `(v, cos θ)` unchanged. `0x7c04f0(out)` is the logarithm: for `|w|` strictly below 1 it
writes `((θ/sin θ)·v, 0)` with `θ = acos(w)`, and when `|sin θ|` is below 2⁻²¹, or `|w|` is 1 or more or NaN,
it writes `(v, 0)`. The multiplication in the exponential uses `k` unrounded for the z component and
rounded for x and y.

Addresses: `0x7c0450`, `0x7c048e`, `0x7c0499`, `0x7c04a1`, `0x7c04a7`, `0x7c04ac`, `0x7c04af`, `0x7c04b2`, `0x7c04f0`, `0x7c04fc`, `0x7c0504`, `0x7c0507`, `0x7c050c`, `0x7c0513`, `0x7c0526`, `0x7c0528`, `0x81d9a0`
Confidence: verified
Proof: one
Test: none

`0x7c048e fsincos` stores the cosine at `[ebp-8]` and the sine at `[ebp-4]`; `0x7c0499 fabs; fcomp
2⁻²¹; test ah,1; jne 0x7c04d4` takes the copy leg for a sine below the bound. `0x7c04af fst [ebp-4]`
keeps `st0` unrounded for the z product (`0x7c04b2`) while the y and x products reload the rounded
store. In the logarithm `0x7c04fc fcomp 1.0; test ah,5; jp 0x7c0546` copies for `|w|` of 1 or more and NaN, and
`0x7c0513 fsin` takes the sine of the `acos` result; the quotient is `θ/sin θ` (`0x7c0526 fxch`, `fdiv`).
Neither is called.

## Slerp

`0x7c0570(a, t, b)` interpolates spherically between `a` (`edx`) and `b` (`[ebp+0xc]`) by `t` (`[ebp+8]`),
writing the result through `ecx`. The cosine `d = a·b` (accumulated z, y, x, w); when it is negative the
sign is −1 and `d` is negated. `s = √|1 − d²|`. When `s` is below 2⁻²¹ the result is `a`. Otherwise
`θ = atan2(s, d)` (`fpatan`), `w0 = sin((1 − t)·θ)/s`, `w1 = sign·sin(t·θ)/s`, and the result is
`w0·a + w1·b`, each component rounded to a float.

Addresses: `0x7c0570`, `0x7c0576`, `0x7c0597`, `0x7c059f`, `0x7c05aa`, `0x7c05ac`, `0x7c05b5`, `0x7c05bf`, `0x7c05c1`, `0x7c05c8`, `0x7c05d3`, `0x7c05fe`, `0x7c0606`, `0x7c060f`, `0x7c0614`, `0x7c0620`, `0x7c066f`, `0x5c6540`, `0x81d9a0`
Confidence: verified
Proof: one
Test: none

`0x7c059f fcom 0.0; test ah,5; jp 0x7c05b5` keeps the sign for a dot of 0 or more and NaN, else `0x7c05ac`
stores −1.0 and `0x7c05b3 fchs` negates. `0x7c05c8 fcomp 2⁻²¹; test ah,5; jp 0x7c05f6` continues for `s` at the
bound or above, and the copy leg `0x7c05d5` stores the four dwords of `a`. The weights multiply the
components of `a` and `b` into locals and the four-float builder `0x5c6540`; the sums are stored as
floats one at a time. The function is not called.

## Squad

The squad family builds spline control quaternions from keys and blends them; none of it is
called. `0x7c06c0` takes three keys, `q1` in `ecx`, `q2` in `edx` and `q3` on the stack, and two
output pointers. It forms `d = log(q1⁻¹⊗q2) − log(q2⁻¹⊗q3)`, scales it by 0.25, and writes
`q2 ⊗ exp(0.25·d)` to the first output and `q2 ⊗ exp(−0.25·d)` to the second. `0x7c0940` is the same setup
with one product and one output. `0x7c1270` is three slerps (`0x7c0570`) with the weight
`2·(1 − t)·t`. `0x7c0b60` evaluates a segment from keys and control points with clamps on two time
arguments, logs, exponentials and the constants 0.5 and −0.5.

Addresses: `0x7c06c0`, `0x7c06ce`, `0x7c075c`, `0x7c0767`, `0x7c07f8`, `0x7c0804`, `0x7c0812`, `0x7c0842`, `0x7c084d`, `0x7c08d2`, `0x7c08dd`, `0x7c0902`, `0x7c090d`, `0x7c0940`, `0x7c1270`, `0x7c128b`, `0x7c129b`, `0x7c12b6`, `0x7c0b60`, `0x7c1320`, `0x7b29c0`, `0x8029b0`, `0x803068`
Confidence: inferred
Proof: none
Test: none

`0x7c06c0` conjugates `q1` and `q2` with `0x7c1340`, multiplies with inline Hamilton products
into `0x7c1320` (a four-float constructor like `0x5c6540`), takes `0x7c04f0` of both, subtracts the
four components, scales them by the constant at `0x8029b0` (`0x7c084d` to `0x7c087d`, read as 0.25) and
exponentiates with `0x7c0450`; `0x7b29c0` multiplies `q2` by each (`0x7c08dd`, `0x7c090d`). The
function returns with `ret 0xc`. The product orders and the signs are read from the calls and are not
checked against a reference. `0x7c0b60` (`ret 0x24`) is read only at its call and constant skeleton.
