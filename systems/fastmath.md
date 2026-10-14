# fastmath

The client's software scalar maths file after the spline file, `0x454a90` to `0x455ba2`: a
base-2 logarithm and exponential built from a table and a polynomial, a quadratic solver, an
angle wrap, an arctangent polynomial, vector tests, batch reciprocal and reciprocal square root
routines and cubic table interpolation. The logarithm and exponential use no `f2xm1` or `fyl2x`,
so their results do not depend on the CPU. Of its functions only `0x454d40`, `0x454f40` and `0x455b30` are called. The file's three
static initializers store +∞, 2π and 1/2π in `0xb05e98`, `0xb05ea0` and `0xb05e9c`, and nothing
reads them. The next file opens at `0x455c00` with an initializer of its own; `0x455bb0` and
the code after it are not this file's. The x87 runs at 53 bits (`METHOD.md`).

Roots: `0x454d40`, `0x454f40`, `0x455b30`, and the initializers `0x454a90`, `0x455b60`, `0x455b80`

## Software exp2

`0x454d40(x)` (cdecl, `ret 8`, result in `st0`) returns 2^x for x from −1021 to 1026.
`S = x + 1023.0`, `n = low32(__ftol(S)) − 1`, `frac = S − (double)(uint64){n, 0}` in [1, 2).
`i` is the top six mantissa bits of `frac`; `rf` is `frac` with them cleared. The result is
`P(rf)·(2^n·T[i])`: `2^n` is the double with high dword `n << 20` and low dword 0, `T[i] = 2^(i/64)`
is at `0x8029e0`, and `P(r) = (c0 + r·c1) + ((c2 + r·c3) + (c4 + r·c5)·r²)·r²`, c0 to c5 the doubles
from `0x803028`. For unsigned `n > 0x7ff`: +∞ if `n` is positive as an int, else 0.0.

Addresses: `0x454d40`, `0x8029e0`, `0x803028`, `0x803058`, `0x40a2b0`, `0x875e90`, `0x7ffd74`
Confidence: verified
Proof: one
Test: `fastmath::exp2_diff`

`n` is stored in `[ebp+8]` with `[ebp+0xc]` zeroed, so `fild qword` reads it as unsigned
(`0x454d57` to `0x454d61`); the `fstp qword` to `[ebp+8]` (`0x454d6d`) then overwrites `[ebp+0xc]`
with the high dword of `frac`, which `0x454d70` reads for the index (`& 0xfc000`, `>> 0xb`) and
the cleared copy. `T` runs from 1.0 to 1.978456…; the constants are 0.99970524…, 0.69487494…,
0.23603244…, 0.06087614…, 0.00583003… and 0.00268119…; 1023.0 is at `0x803058`, +∞ at
`0x875e90`, `0.0f` at `0x7ffd74`. `n == 0x7ff` (x in [1025, 1026)) passes `jbe` with a scale of +∞;
`n == 0` (x in [−1022, −1021)) passes it with a scale of 0.0, so the result is 0. `__ftol` truncates with
`fistp qword`, so a NaN or ±∞ gives the integer indefinite, low dword 0, `n = −1` and the result 0.0, and so
does any x of 2^31 or more, through the sign of the low dword. From 2^32 to 2^63 in size the low dword wraps
and the result is arbitrary. Four calls reach it: `0x6783d1`, `0x6783fa`, `0x67841a` and `0x6cf138`. The test
runs `0x454d40` whole over the range, the table edges and the overflow and underflow limits.

## Software log2

`0x454c80(x)` returns `log2 x` for a double: −∞ when `x <= 1e-307` (`0x802ff0`) or NaN, 1024 for +∞.
`e` is the exponent field minus 0x3ff, `m` the mantissa as a double in [1, 2), `i` its top six
mantissa bits, `r = m·L[i] − 1.0`, `L[i] = 64/(64 + i)` at `0x802be0`. The result is
`r·d + (e + K[i])`, `K[i] = log2((64 + i)/64)` at `0x802de0`, `d = (a + (c3 − r·c4))·r² + (c1 − r·c2)`,
`a = (c5 − r·c6)·r²`, c1 to c6 the doubles from `0x802ff8`: 1.442695040888937, 0.7213475204127876, 0.48089833404997356, 0.36067132973951144, 0.2883070248990067 and 0.22949900023246153. The first four are close to `1/(k·ln 2)`; c5 and c6 are not (0.288539 and 0.240449 for k = 5, 6).

Addresses: `0x454c80`, `0x802be0`, `0x802de0`, `0x802ff0`, `0x802ff8`
Confidence: verified
Proof: one
Test: none

The compare of `x` with `1e-307` is `fcomp` then `test ah,0x41; je` (`0x454ccd`): equal, below and
unordered all take the −∞ path (`0x875e90` negated). The index is `(hi >> 11) & 0x1f8`, a byte offset.
`x` is the stack double at `[ebp+8]`, `ret 8`. Its 7 callers are the unreached random-number
generators and the wrapper `0x456660`, so it is unreached. No test runs it.

## Quadratic solver

`0x454f40(a, b, c)` takes three floats, with `ecx = &t1` and `edx = &t2`, and returns in `al` whether `a·t² + b·t + c`
has two roots: 0, leaving `t1` and `t2` untouched, when `b·b <= (a·c)·4` or either is NaN. Otherwise `q = √(b² − 4ac)`, `s = (b > 0 ? b + q : b − q)·(−0.5)`, `inv = 1/(a·s)`, the roots are
`(inv·s)·s`, kept wide, and `f32(f32(inv)·a·c)`, and the smaller goes to `t1`, the larger to `t2`, both as floats.
A tie or a NaN wide root puts the wide root in `t2`.

Addresses: `0x454f40`, `0x80306c`, `0x803068`, `0x6cf9c0`, `0x6cfa21`
Confidence: verified
Proof: one
Test: `fastmath::quadratic_roots_difftest`

The products and sum are at 53 bits: `fld a; fmul c; fmul [0x80306c]` (4.0), `fld b; fmul b`, `fcom`
and `test ah,0x41; jne` (`0x454f55` to `0x454f5c`); `fsubrp` and `fsqrt`; the sign test `fcomp` against `0.0f`
(`0x7ffd74`) with `jne` to the subtract (`0x454f77`), so a `b` of 0 or NaN subtracts; `fmul [0x803068]` (−0.5).
`fdivr [0x7ff9d8]` gives `inv`, `fst` stores its float copy to `[ebp+0xc]` while `st0` keeps it wide. The roots are
compared with `fcom`, `test ah,5; jp` (`0x454fa6`): `jp` is taken, the wide root to `t2`, unless it is strictly below the float root.
One caller, `ProjectCloudLight` (`0x6cf9c0`, call `0x6cfa21`). The test runs the whole function.

## Angle wrap

`0x455b30(a)` takes a float and returns in `st0` `r = fmod(a, 2π)`, plus 2π when `r < 0`. The modulus
is the double at `0x803090`, the float 2π widened (6.2831854820251465), and the added value is the float at
`0x80295c`, the same number; the sum stays at 53 bits. A NaN or −0 remainder is returned as it is.

Addresses: `0x455b30`, `0x803090`, `0x80295c`, `0x73f90a`
Confidence: verified
Proof: one
Test: `fastmath::angle_wrap_diff`

`fld a; fld [0x803090]; call __CIfmod (0x73f90a)`, then `fcom` against `0.0f` and `test ah,5; jp` (`0x455b4c`):
a remainder not below 0, and an unordered one, skips the `fadd`. `__CIfmod` returns the `fprem` remainder with
the sign of `a`. A NaN or infinite `a` takes its error handlers. 15 call sites, in the camera (`0x50d762` to
`0x513006`), the unit's facing (`0x60102a` to `0x610a56`) and `0x47f1af`. The test runs the whole function, with `__CIfmod` replaced
by chosen remainders for NaN and the infinities.

## Static initializers

The file's three initializers run from slots `0x827558`, `0x82755c` and `0x827560` of the C runtime's table.
`0x454aa0` copies +∞ (`0x7f800000`, `0x8029d8`) to `0xb05e98`; `0x455b70` stores `π + π` of the float at
`0x8029dc` in `0xb05ea0` (2π, `0x40c90fdb`); `0x455b90` stores `1.0 / 0xb05ea0` in `0xb05e9c`. Each is entered
through a `jmp` thunk, `0x454a90`, `0x455b60` and `0x455b80`.

Addresses: `0x454a90`, `0x454aa0`, `0x455b60`, `0x455b70`, `0x455b80`, `0x455b90`, `0x827000`, `0x8029d8`, `0x8029dc`, `0xb05e98`, `0xb05e9c`, `0xb05ea0`
Confidence: verified
Proof: one
Test: none

The three results have no reader except the third initializer reading the second. Census: the operand `0xb05e98`
occurs once, `0xb05e9c` once (a store) and `0xb05ea0` twice, all in this file.

## Unreached functions

`0x454ab0` is an arctangent: a polynomial in `t = |x|`, or in `1/|x|` when `|x| > 1`, with the second float
argument minus the result in that case and the sign of `x` restored. `0x454b30`, `0x454ba0` and `0x454c10`
are log2 polynomials taking the table value and coefficients on the stack. `0x454e10` compares a double
3-vector with two others and a limit; `0x454ea0` is the quadratic solver in doubles. `0x454fd0` refines a
reciprocal per double in blocks of three from the seed `0x7fde61ee`; `0x455260` and `0x4555f0` do the same
for the reciprocal square root, seed `0x5fe732f7`. `0x455970` and `0x455a50` interpolate a double or float
table by a Catmull-Rom cubic.

Addresses: `0x454ab0`, `0x454b30`, `0x454ba0`, `0x454c10`, `0x454e10`, `0x454ea0`, `0x454fd0`, `0x455260`, `0x4555f0`, `0x455970`, `0x455a50`, `0x454c80`
Confidence: inferred
Proof: none
Test: none

The census is two scans over the whole image: every `e8`/`e9` whose target is the function and every occurrence of its
address as a dword in any section. For each of the functions but `0x454c80` both find nothing; the control is `0x454d40`,
which the first finds four times. Each function's meaning is read from its first instructions and constants, not from a
full walk.
