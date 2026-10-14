# cmath: scalars

The scalar file of the Tempest math library, `0x455c00` through `0x4567c2`: division by small
constants, the minimum, median, maximum, range and mean of 3, 5 and 9 integers, normalization of
vectors of two and three, bit-pattern estimates of reciprocals and roots, splitting a float into a
whole part and a fraction, and shaping functions. Three routines are live: the normalization of a
vector of two, the reciprocal square root estimate and the high multiply `0x455c70` in the file's
middle, derived in `cmath/overview#high-product`. Everything else is reached by nothing.

The file's initializers in the C runtime's run (`0x827564` to `0x82756c`) are the header triple, +∞
`0x455c00`, 2π `0x456780` and 1/2π `0x4567a0`. The next file starts at `0x4567d0`.

Roots: `0x455c00`, `0x456280`, `0x456330`, `0x456780`, `0x4567a0`

## Normalizing vectors

`0x456280` takes the addresses of `x` in `ecx` and `y` in `edx`, both floats, and replaces them with
`x·r` and `y·r`, where `r = 1/√(x² + y²)` at 53 bits and each product is stored as a float. There is no
zero test. `0x456250` does the same for two doubles; `0x4562f0` and `0x4562b0` take three addresses
(`ecx`, `edx` and one on the stack) for floats and doubles.

Addresses: `0x456250`, `0x456280`, `0x4562b0`, `0x4562f0`, `0x50a8eb`, `0x7e56fd`
Confidence: verified
Proof: one
Test: `spline::normalize2_f32_diff`

`0x456280` is `fld [edx]; fld [ecx]; …; fsqrt; fdivr 1.0` and then two `fmul; fstp`. The two callers are
`0x50a8eb` and `0x7e56fd`. The other three have no caller.

## Reciprocal and root estimates

`0x456330` takes a magic number in `ecx` and a float on the stack (`ret 4`) and returns in `st0` the
float whose bits are the magic less half of the argument's bits (`shr 1`, masked with `0x3fffffff`).
It is the seed of a reciprocal square root. `0x456350` does the same for a double, shifting its high
dword. Four siblings subtract the whole bit pattern instead, which seeds a reciprocal: `0x4563a0`
and `0x4563b0` take the value on the stack, `0x4563c0` and `0x4563d0` work in place through `ecx`
with the magic in `edx`, and `0x456370` and `0x456380` are the in-place forms of the root seed.

Addresses: `0x456330`, `0x456350`, `0x456370`, `0x456380`, `0x4563a0`, `0x4563b0`, `0x4563c0`, `0x4563d0`
Confidence: verified
Proof: one
Test: `cmath::fast_inv_sqrt_seed_difftest`

The magic numbers its callers pass are derived with the cloud colours in
`lighting/clouds#cloud-colours`; `0x456330` has two callers, `0x6cfe3e` and `0x6cfe63`. The siblings
have none. The test runs `0x456330` whole.

## Division by constants

`0x455c60(a, b)` returns in `eax` the high dword of the signed product of `ecx` and `edx`, and `ecx`
the sign of that dword. `0x455c80(n)` is `⌊n/3⌋` for unsigned `n` (`n·0xaaaaaaab >> 33`), exact for every
`n`. `0x455c90(n)` is the high dword of `n·0x55555556`, signed: `⌊n/3⌋`, one lower for a negative multiple
of 3. Four routines divide by 5 and 9 with shifts: `0x455ca0` is `⌊n/5⌋` for `n` below 84194565,
`0x455cc0` is `⌈n/5⌉` for `n` from −671088644 to 352630020, `0x455ce0` is `⌊n/9⌋` for `n` below
603979776, and `0x455d00` is `⌈n/9⌉` for `n` from −301989896 to 301989879. Outside those ranges the
results differ from the exact quotient.

Addresses: `0x455c60`, `0x455c80`, `0x455c90`, `0x455ca0`, `0x455cc0`, `0x455ce0`, `0x455d00`
Confidence: verified
Proof: one
Test: none

`0x455ca0` is `x = 3n; x += x >> 4; x += x >> 8; (x + (x >> 16) + 2) >> 4`; `0x455cc0` is the signed
form with `sar` and `+ 0xf`; `0x455ce0` is `x = 7n; x += x >> 6; x += x >> 12; (x + (x >> 24) + 2) >> 6`
and `0x455d00` the signed form with `+ 0x3e`. The ranges come from running a C transcription of each
listing over all 2³² inputs.

## Order statistics

Fifteen routines take 3, 5 or 9 signed integers, the first two in `ecx` and `edx` and the rest on the
stack, and return one: the minimum (`0x455d20`, `0x455e00`, `0x455f00`), the median (`0x455d40`,
`0x455e30`, `0x455f50`), the maximum (`0x455d70`, `0x455e70`, `0x4560e0`), the range, maximum less
minimum (`0x455d90`, `0x455ea0`, `0x456130`) and the mean (`0x455de0`, `0x455ee0`, `0x456210`). The mean of
three is the sum through `0x455c90`; the means of five and nine are the sum through `0x455cc0` and
`0x455d00`.

Addresses: `0x455d20`, `0x455d40`, `0x455d70`, `0x455d90`, `0x455de0`, `0x455e00`, `0x455e30`, `0x455e70`, `0x455ea0`, `0x455ee0`, `0x455f00`, `0x455f50`, `0x4560e0`, `0x456130`, `0x456210`
Confidence: verified
Proof: one
Test: none

The median of five trims the extremes: `lo = max(min(a, b), min(d, e))`, `hi = min(max(a, b),
max(d, e))` and the median of `lo`, `c` and `hi` through `0x455d40`; the range of five trims the same
way into `0x455d90`. The median of nine is a network of compare-exchanges
(`t = b − a; a += min(t, 0); b −= min(t, 0)`, the minimum by `setg` and a mask). The routines with
branches use signed compares (`jl`, `jg`). A transcription of the minimum, median, maximum and range
of three and nine, and of the minimum, maximum and trimmed median and range of five, agrees with
sorting over random inputs.

## Splitting a float

Four routines split `x` into `⌊x⌋` and a fraction. `0x4563e0` (a double on the stack) and `0x456420`
(a float) truncate with `__ftol` and subtract 1 when `x` is negative: `edx` holds the address of the integer and
`ecx` the address of the fraction `x − i`. `0x456470` and `0x4564c0` keep the whole part as a double
or a float. A negative whole number gives a part one too low and a fraction of 1.0.

Addresses: `0x4563e0`, `0x456420`, `0x456470`, `0x4564c0`, `0x40a2b0`, `0x4563f3`, `0x456442`
Confidence: verified
Proof: one
Test: none

`__ftol` is `0x40a2b0`. `0x456442` compares `x` with 0.0 and `jp` skips the `dec ecx` for `x ≥ 0` and NaN.
For −2.0 the parts are −3 and 1.0. `0x455c20` and `0x455bb0` use `0x456420`.

## Shaping functions

`0x456510(a, b)` returns 1.0 when `a ≥ b` and 0.0 otherwise. `0x456540(x, lo, hi)` is the difference of two
of them, 1.0 inside `[lo, hi)`. `0x456570(x, lo, hi)` is `(x − lo)/(hi − lo)` clamped into [0, 1], a NaN
giving 1.0. `0x4565c0` is the smooth step: 0.0 below `lo`, 1.0 at or above `hi`, else `t²·(3 − 2t)` for
that `t`. `0x456620(a, b)` is the C runtime's `pow(a, b)` (`__CIpow`, `0x73f940`), `0x456640` is
`pow(a, 1/b)`, `0x456660(x, b)` is `pow(x, −log2(b))` through the fast logarithm `0x454c80`, and `0x4566a0(x, g)` is
the gain built from it: `0.5·bias(2x, 1 − g)` for `x < 0.5` and `1 − 0.5·bias(2 − 2x, 1 − g)` otherwise.

Addresses: `0x456510`, `0x456540`, `0x456570`, `0x4565c0`, `0x456620`, `0x456640`, `0x456660`, `0x4566a0`, `0x455c20`, `0x456700`, `0x456720`, `0x456740`, `0x456760`, `0x455bb0`, `0x73f940`, `0x80297c`, `0x8030a0`, `0x8030a8`, `0x8030b0`, `0x801628`
Confidence: verified
Proof: one
Test: none

`0x455c20(x, y)` is a smooth zigzag of the phase `p = x·y`: with `f = p − ⌊p⌋` it is `1 − (6 − 4f)·f²`,
negated for odd `⌊p⌋`; `0x455bb0` is the same with `p = x·y − 0.5`. It belongs to no object file here: it lies after fastmath's last
initializer (`0x455ba2`) and before this file's +∞ initializer (`0x455c00`), and a file can have code on both sides of its
initializers, so position decides nothing; it calls `0x456420` as `0x455c20` does, which favours this file, and nothing ties it
to fastmath. `0x456700` and `0x456740` return `sin(πxy)/(πxy)`, and `0x456720` and `0x456760`
`sin(πx)/(πx)`, with no zero guard. `0x456510` is `fcomp; test ah,5; jp`: below gives 0.0 (`0x7ffd74`), else 1.0. `0x4565c0` loads 3.0 from
`0x80297c` and `0x4566da` loads 2.0 from `0x801628`. `0x456660` multiplies the logarithm by −1.0 (`0x8030a0`) before `__CIpow`. The sine
routines multiply by π as a double (`0x8030a8`) or a float (`0x8030b0`) and divide `fsin` into the
product.

## Header initializers

Three globals hold the file's header constants: +∞ at `0xb05ea4` from `0x803098`, 2π at `0xb05eac` (the
float π at `0x80309c` doubled) and 1/2π at `0xb05ea8` (1.0 over 2π at 53 bits). Nothing but the 1/2π
initializer reads any of them.

Addresses: `0x455c00`, `0x455c10`, `0x456780`, `0x456790`, `0x4567a0`, `0x4567b0`, `0xb05ea4`, `0xb05eac`, `0xb05ea8`, `0x803098`, `0x80309c`
Confidence: verified
Proof: one
Test: none

`0x455c00`, `0x456780` and `0x4567a0` are each one `jmp` to the next function. A scan of the listing and
of every dword of the image finds `0xb05eac` read only at `0x4567b6`.

## Unreached code

Nothing live reaches the file's functions other than `0x455c70` (`cmath/overview#high-product`), `0x456280`
and `0x456330`. The census is a scan of every `call`, `jmp` and `jcc` target in the listing and of every
dword of the image for each address; its control is `0x456330`, with two call sites. The routines that
call others in the file (`0x455de0`, `0x455ee0`, `0x456210`, `0x455c20`, `0x456640`, `0x456660`, `0x4566a0`,
`0x456540`) are themselves unreached. `0x455bb0` is unreached too (placement in `cmath/scalars#shaping-functions`).

Addresses: `0x456330`, `0x455c70`, `0x456280`, `0x455c20`, `0x455c60`, `0x455de0`, `0x455ee0`, `0x456210`, `0x456540`, `0x456640`, `0x456660`, `0x4566a0`, `0x455bb0`
Confidence: verified
Proof: one
Test: none

`0x455c80` is called by the unreached batch routines `0x454fd0`, `0x455260` and `0x4555f0` of the fast-math file.
No address in the file is stored in a table.
