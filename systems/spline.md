# spline

The movement spline file: curve objects that hold control points and give a point, a tangent or a
frame at a parameter, in two kinds, Bezier and Catmull-Rom, with the basis tables they read, the
vector leaves after them and an integer square root.

The object file runs from `0x4532c0` through `0x454a84`. Its initializers in the C runtime's run are the
header triple, +∞ `0x4532c0`, 2π `0x454930` and 1/2π `0x454950`, and four table initializers,
`0x453950`, `0x453f90`, `0x454070` and `0x454120` (slots `0x82753c` to `0x827554`). The fast-math
file starts at `0x454a90`. A Catmull-Rom curve is built in every creature movement record; Bezier
curves are built by two emitter objects. A vector is three floats. Every float result is one x87
expression at 53 bits stored once, unless an entry names a stored intermediate.

Roots: `0x4532c0`, `0x453300`, `0x453360`, `0x453390`, `0x453420`, `0x453480`, `0x453950`, `0x453f90`,
`0x454070`, `0x454120`, `0x4548a0`, `0x454930`, `0x454950`, `0x454980`, `0x4549a0`, `0x4549c0`,
`0x4549f0`, `0x454a10`

## Curve objects

A curve object is a vtable and a record. Three vtables of nine slots exist: the base `0x7ffdbc`, the
Catmull-Rom curve `0x7ffd84` and the Bezier curve `0x81d90c`. Slots 0 to 8 are the total length, the
rebuild of the segment lengths, the point at an arc-length and at a uniform parameter, the tangent at
each, the frame, the setting of the control points and the extraction of points. The base holds the
pure-call stub `0x40a2d7` in slots 0 to 6.

| offset | field |
|---|---|
| `+0` | vtable |
| `+4` | total length, a float |
| `+8`, `+0xc` | control point count, both stored by `0x4534f0` |
| `+0x10` | control point array, 12 bytes a point |
| `+0x18` to `+0x24` | segment length list: capacity, count, data, growth quantum |
| `+0x28` | mode: 0 straight chords, else cubic |

Addresses: `0x7ffd84`, `0x81d90c`, `0x7ffdbc`, `0x4543d0`, `0x4543e0`, `0x454420`, `0x454460`, `0x4544a0`, `0x454510`, `0x454580`, `0x454770`, `0x454820`, `0x453e70`, `0x453e90`, `0x453ad0`, `0x453a90`, `0x453b50`, `0x453b10`, `0x453b90`, `0x453ee0`, `0x4534f0`, `0x453570`, `0x469bc0`, `0x630c40`, `0x4077e5`, `0x5f9439`, `0x5f9b9f`, `0x7b9220`, `0x7b9320`, `0x4088dc`, `0x5f9cfe`, `0x7c6a50`, `0x7c6a8f`, `0x7c567a`, `0x7c55a0`
Confidence: verified
Proof: one
Test: none

The Catmull-Rom vtable is stored at `0x4077e5` (a local curve with mode 1), `0x469be3`, `0x5f9439`,
`0x5f9b9f` and `0x630c76`. The movement record constructors `0x469bc0` and `0x630c40` place the
curve at record `+0x2c` and store 1 into the mode at record `+0x54` (`0x469bea`, `0x630c7d`). The
Bezier vtable is stored at offset `0x2a8` of two emitter objects (`0x7b9220`, `0x7b9320`). Two base
constructors store the base vtable (`0x4088dc`, `0x5f9cfe`). Slot 8 of the Catmull-Rom vtable is
`0x454820`; the base and the Bezier curve hold `0x453570`, which returns 0. The spline start `0x7c6a50`
stores the mode again (`0x7c6a8f` to `0x7c6a9d`): 1 when the spline flags have 0x200, else 0. A
flying spline so takes the cubic and a ground one the chord. The creature follow asks the record's curve
for a point at `0x7c567a` and a frame at `0x7c55a0`, both with mode 1.

## Dispatch and clamps

`0x453390(t, out, mode)` returns the first control point for `t ≤ 0` and the last, `P[count − 1]`,
for `t ≥ 1`. Otherwise mode 1 calls slot 2 and mode 0 calls slot 3 with `(t, out)`; any other mode
writes nothing. `0x453420` and `0x453480` clamp `t` into [0, 1], a NaN becoming 1, and call the
tangent (mode 1 slot 4, mode 0 slot 5) or the frame (mode 1 slot 6, otherwise nothing).
`0x453360(points, count)` calls slot 7, then `0x4532e0`, which rebuilds the lengths (slot 1) and
stores the total (slot 0) at `+4` when the count exceeds 3. The mode argument is the parameter kind,
not the curve's `+0x28`.

Addresses: `0x4532e0`, `0x453360`, `0x453390`, `0x453420`, `0x453480`, `0x453393`, `0x4533a1`, `0x4533c6`, `0x4533cb`, `0x4533d6`, `0x4533f4`, `0x453456`, `0x45345b`, `0x453468`, `0x453479`, `0x4534c3`
Confidence: verified
Proof: one
Test: none

`0x453393` to `0x4533a1`: `fcomp 0.0; test ah,0x41; jp` falls through for `t < 0` and `t = 0`
(one flag bit set) and branches for `t > 0` and NaN (none or both). `0x4533c6`: `fcomp 1.0; test ah,1;
jne` dispatches for `t < 1` and NaN, else `0x4533d6` copies the point at `ctrl + 12·count − 12`.
`0x4533f4` to `0x453417` select slot `+0xc` for mode 0 and `+8` for mode 1. `0x453420` and
`0x453480` use `test ah,5`: below 0 stores 0.0, and `jnp` stores 1.0 for `t ≥ 1` and NaN.
`0x453456` to `0x453468` pick slot `+0x14` or `+0x10`; `0x453480` calls `+0x18` only for mode 1. The
callers are `0x453390` at 4 sites, `0x453360` at 7, `0x453420` at 2 and `0x453480` at 2.

## Control point storage

`0x4534f0(points, n)` stores `n` at `+8`, reallocates the control array to `12n` bytes, copies `n`
vectors from `points` and stores `n` at `+0xc`. The Catmull-Rom slot 7, `0x454770`, grows the
segment length list to `n − 3` floats and calls `0x4534f0` with `(points, n)`. The Bezier slot 7,
`0x453ee0`, grows it to `⌊n/3⌋` and calls `0x4534f0` with `3·⌊n/3⌋ + 1` points, one more than `n`
unless `n = 3k + 1`. `0x4548a0(cap)` resizes a float list to `cap` entries. The segment count
`n − 3` wraps for `n` of 1 or 2.

Addresses: `0x4534f0`, `0x453330`, `0x454770`, `0x453ee0`, `0x4548a0`, `0x454808`, `0x454820`, `0x4534ff`, `0x453519`, `0x453efd`, `0x4547d5`, `0x453f55`, `0x82f230`, `0x82f250`
Confidence: verified
Proof: one
Test: none

`0x453519` calls `0x646320` with the old array, the size `12n` and the tag `0x82f230`; with `n` zero
and no array it skips to the `+0xc` store. `0x453535` skips a null destination. The list grows only
when the new count exceeds both the stored count (`+0x1c`) and the capacity. The quantum (`+0x24`) is
0x40 when the request is at least 0x40 (stored), else the largest power of 2 not above it (not
stored), and the request rounds up to a multiple of it (`0x4547d5`, `0x453f55`). `0x453efd` computes
`3s + 4·(s > 0) − 3` for `s = ⌊n/3⌋`. An `n` of 0 in `0x454770` clears `+0x1c` (`0x454808`).
`0x4548a0` allocates `4·cap` bytes under the tag `0x82f250` and, if the block moved, copies the
first `min(cap, count)` floats and frees the old block; four other callers use it, `0x58e278`,
`0x59a19e`, `0x6d0b15` and `0x6d0b44`. `0x453330(i, v)` stores a vector over point `i`, then calls
`0x4532e0`. `0x454820(t, out, max)` locates a segment `i` and copies up to `min(max, count − i − 3)`
points from `P[i + 2]`, none when `out` is null.

## Path control points

A creature path carries a guard point at each end. The monster move parser `0x6018f0` takes the
anchored points from `0x6021c0`, copies the last point once more at the end (`0x601e66` to
`0x601e80`) and writes the first control point as the reflection of the third through the second,
`P[0] = P[1] − (P[2] − P[1])` (`0x601e83` to `0x601eca`). `K` path points make `K + 2` control
points and `K − 1` Catmull-Rom segments. A parameter of 0 or less returns `P[0]`, the guard.

Addresses: `0x6018f0`, `0x6021c0`, `0x601e4d`, `0x601e52`, `0x601e66`, `0x601e83`, `0x601eca`, `0x601ed1`
Confidence: verified
Proof: one
Test: `objectlayer::monster_move_reflect_through_difftest`

`0x601e4d` calls `0x6021c0`; `0x601e52` subtracts 12 from the returned buffer, which reserves slot
0, and `0x601e55` to `0x601e60` skip everything for a null buffer or a zero count. `0x601e83` to
`0x601ec7` subtract `P[1]` from `P[2]` per axis (z stored as a float first) and store `P[1]` less
the difference over slot 0, each axis stored as a float. `0x601ed1` (`cmp edx,3`) gates only the
length loop that follows. The test runs the reflection slice.

## Catmull-Rom segments

The Catmull-Rom curve has `count − 3` segments, segment `i` using `P[i]` to `P[i + 3]` and running
from `P[i + 1]` to `P[i + 2]`. Slot 0 sums the segment lengths (`0x4543d0`) and slot 1 stores each
as a float (`0x4543e0`). The point and tangent slots locate the segment and local parameter by arc
length (`0x454380`) or uniformly (`0x4543b0`), then evaluate the point (`0x4541b0`) or the tangent
(`0x454250`). The tangent slots clamp `t` into [0, 1]; the point slots do not. In mode 0 a segment
is the chord and its length the chord length; otherwise it is the cubic and its arc length.

Addresses: `0x454380`, `0x4543b0`, `0x4541b0`, `0x454320`, `0x454250`, `0x4544a0`, `0x454510`, `0x454420`, `0x454460`, `0x4543d0`, `0x4543e0`, `0xb05e10`, `0xb05dd8`, `0x453920`
Confidence: verified
Proof: one
Test: `spline::linear_pos_diff`, `spline::seg_length_diff`

`0x454380` calls `0x453840(t, 0xb05e10, count − 3, …)` and `0x4543b0` calls `0x4538c0(t, count − 3,
…)`. `0x4541b0` reads `+0x28`: zero gives `P[i + 1] + (P[i + 2] − P[i + 1])·t`, where the x
product `dx·t` and the z difference `dz` are stored as floats before use and the y product stays at
53 bits; otherwise it calls `0x453580(i, t, 0xb05e10, out)`. `0x454320` is `√((dz² + dy²) + dx²)`
of that difference in mode 0, else `0x453760(i, 0xb05e10)`. `0x454250` copies the twelve floats of
`0xb05dd8` into rows of four (the fourth float of rows 0 to 2 is 0, of row 3 is 1.0; degree 2 never
reads it) and calls `0x453640`.

## Bezier segments

The Bezier curve has `⌊count/3⌋` segments of four points sharing their ends, segment `i` using `P[3i]`
to `P[3i + 3]`. Slot 0 sums the segment lengths (`0x453e70`), slot 1 stores each as the arc length of
`0x453e50`. The point slots locate by arc length (`0x453a60`) or uniformly (`0x453a30`) and
evaluate with `0x453e30`; the tangent slots do the same with `0x453cd0`. No point or tangent slot
clamps `t`.

Addresses: `0x453a30`, `0x453a60`, `0x453e30`, `0x453cd0`, `0x453e50`, `0x453e70`, `0x453e90`, `0x453a90`, `0x453ad0`, `0x453b10`, `0x453b50`, `0xb05e58`, `0xb05d80`, `0xb05e08`, `0x453e20`
Confidence: verified
Proof: one
Test: none

`0x453a30` and `0x453a60` take the segment count as `⌊count/3⌋` by `mul 0xaaaaaaab; shr edx,1` and
call `0x4538c0` or `0x453840` with `0xb05e58` as the unread table. `0x453e30(i, f, out)` is
`0x453580(3i, f, 0xb05e58, out)`; `0x453e50(i)` is `0x453760(3i, 0xb05e58)`. `0x453cd0(i, f, out)` is
`0x453640` over the derivative table `0xb05d80` at point `3i`, expanded into rows of four as
`0x454250` does. Its first call fills `0xb05d80`, sets the guard byte `0xb05e08` and registers the
empty exit stub `0x453e20`.

## Basis tables

The tables hold the coefficients of each control point's weight as a polynomial in `t`, highest power
first, one row per point. `0xb05e58` is the Bezier basis and `0xb05e10` the Catmull-Rom basis;
`0xb05d80` and `0xb05dd8` are their first derivatives, rows of three; `0xb05db4` is the Catmull-Rom
second derivative, rows of two.

| table | row 0 | row 1 | row 2 | row 3 |
|---|---|---|---|---|
| `0xb05e58` | −1, 3, −3, 1 | 3, −6, 3, 0 | −3, 3, 0, 0 | 1, 0, 0, 0 |
| `0xb05e10` | −0.5, 1, −0.5, 0 | 1.5, −2.5, 0, 1 | −1.5, 2, 0.5, 0 | 0.5, −0.5, 0, 0 |
| `0xb05d80` | −3, 6, −3 | 9, −12, 3 | −9, 6, 0 | 3, 0, 0 |
| `0xb05dd8` | −1.5, 2, −0.5 | 4.5, −5, 0 | −4.5, 4, 0.5 | 1.5, −1, 0 |
| `0xb05db4` | −3, 2 | 9, −5 | −9, 4 | 3, −1 |

Addresses: `0xb05e58`, `0xb05e10`, `0xb05d80`, `0xb05dd8`, `0xb05db4`, `0x453960`, `0x453fa0`, `0x454080`, `0x454130`, `0x453cd0`, `0x453950`, `0x453f90`, `0x454070`, `0x454120`, `0x453a10`, `0x454050`, `0x454100`, `0x454190`, `0x453a20`, `0x454060`, `0x454110`, `0x4541a0`
Confidence: verified
Proof: one
Test: `spline::basis_weight_diff`

Static initializers write `0xb05e58` (`0x453960`), `0xb05e10` (`0x453fa0`), `0xb05dd8` (`0x454080`) and
`0xb05db4` (`0x454130`) as runs of `mov dword` stores. The thunks `0x453950`, `0x453f90`,
`0x454070` and `0x454120` call them, then the registrars `0x453a10`, `0x454050`, `0x454100` and
`0x454190`, each of which registers an empty exit stub with `_atexit`. `0xb05d80` is written only by
`0x453cd0`. Only the unreached `0x454300` reads the second derivative.

## Point and derivative kernels

`0x453620(t)` with `ecx` the degree and `edx` a row evaluates the polynomial by Horner's rule at 53
bits: `w = c0; for i in 1..degree { w = w·t + c[i] }`. `0x453580(seg, t, table, out)` sums the four
weights times the control points `P[seg]` to `P[seg + 3]`; `0x453640` does the same with degree 2.
Rows lie 16 bytes apart. Per point the x product stays at 53 bits, the y and z products are stored
as floats, and each running sum is stored as a float.

Addresses: `0x453620`, `0x453580`, `0x453640`, `0x4535c4`, `0x4535cb`, `0x4535d5`, `0x4535df`, `0x4535ed`, `0x4535f0`, `0x453606`, `0x453679`
Confidence: verified
Proof: one
Test: `spline::basis_weight_diff`, `spline::eval_point_cubic_diff`, `spline::eval_point_quad_diff`

`0x4535c4` calls `0x453620` with `ecx` 3; `0x4535cb` multiplies the weight by x, `0x4535d5` by y
(stored at `0x4535df`), and z is stored at `0x4535ed`. `0x4535f0` adds x to the output and stores
it, then y and z. `0x453606` loops four times. `0x453679` is the degree-2 call.

## Arc length and parameter

`0x453760(seg, table)` returns the length of a segment as 20 chords: points at `t = 0.05·k`, each
chord `√((dz² + dy²) + dx²)`, the running sum stored as a float after each. `0x453840(t, _, n, &i, &f)`
finds the segment by arc length: `n` of 1 gives `(0, t)`; else `s = total·t` and the stored lengths are
added until `s < running + length[i]` or the last segment, `f = (s − running)/length[i]`.
`0x4538c0(t, n, &i, &f)` splits [0, 1] into `n` equal parts: `i` is byte 1 of the float bits of
`t·n + 512`, `f = (t − (1/n)·i)·n`.

Addresses: `0x453760`, `0x453840`, `0x4538c0`, `0x453300`, `0x453920`, `0x8029c8`, `0x8029cc`, `0x4537b3`, `0x45377b`, `0x453814`, `0x45381a`, `0x453899`, `0x4538d1`, `0x4538dc`, `0x4538f0`
Confidence: verified
Proof: one
Test: `spline::arc_length_diff`, `spline::seg_locate_diff`, `spline::fixed_quantize_diff`, `spline::knot_sum_nm1_diff`, `spline::knot_sum_n_diff`

`0x453300(k)` and `0x453920(k)` sum the first `k − 1` and the first `k` stored lengths. `0x45377b`
stores the start step `0x3d4ccccd`; `0x453814` stores each running sum and `0x45381a` adds the
float 0.05, `0x8029c8`. In `0x453840` the running sum is held at 53 bits for the comparison and stored as a
float at `0x453899` when the loop moves on; the loop ends at `n − 1`. `0x4538d1` loads `n` as a
64-bit integer; `0x4538dc` adds `0x8029cc` (512.0), which puts `⌊t·n⌋` in the mantissa bits 14 to 21,
read by `shr edx,0xe; mov al,dl` (`0x4538f0`). `0x453300(0)` and `0x453920(0)` return 0.0.

## Frames

A frame is the tangent `T` at `+0`, the normal `N` at `+0x10`, the binormal `B` at `+0x20` and the
position at `+0x30`; `+0xc`, `+0x1c` and `+0x2c` are untouched. `N = (−T.y, T.x, 0)` divided by its
length, `B = T × N`. The Bezier frame `0x453b90` takes the tangent from `0x453cd0` at the arc-length
parameter and writes it when its length is at least 0.01. The Catmull-Rom frame `0x454580` forms the
unit chord and in mode 0 uses it; otherwise it takes the derivative (`0x454250`) and uses it when it
is at least 0.01 long and its dot product with the chord is at least 0.5, else the chord.

Addresses: `0x454580`, `0x453b90`, `0x4545af`, `0x4545e7`, `0x4545f6`, `0x4545fb`, `0x454606`, `0x454666`, `0x4546a9`, `0x4546e0`, `0x45470b`, `0x453c05`, `0x453c10`, `0x453c74`, `0x8029d0`, `0x8029d4`, `0x7ffa24`
Confidence: verified
Proof: one
Test: `spline::frenet_frame_diff`, `spline::frame_builder_linear_diff`, `spline::frame_builder_spline_diff`

`0x453c05` compares `|len|` with 0.01 (`0x8029d0`) and `jnp 0x453c43` skips the write for a shorter
tangent, which leaves the output's tangent as it was; equal and NaN write. The Bezier length is
`√((y² + z²) + x²)`; `T` is each component times `1/len` stored as a float, and `N` and `B` come
from the 53-bit products without a zero test, so a vertical tangent gives NaN. The Catmull-Rom frame
stores the chord length as a float at `0x4545f6`, skips the division when it is under 2⁻²²
(`0x8029d4`, `0x4545fb`) and else calls `0x4549c0`; `0x454666` is the 0.01 test and `0x4546a9` the
0.5 test (`0x7ffa24`; a shorter derivative leaves `T` unwritten at `0x4546e0`). It rounds `N` to
floats before building `B` and skips the division of `N` under 2⁻²² (`0x45470b`). `B.z =
T.x·N.y − T.y·N.x`, `B.x = −(N.y·T.z)`, `B.y = N.x·T.z`.

## Vector leaves

`0x4549a0(x, y, z)` stores three floats at `ecx` and is the constructor of the file's vector type,
with 401 call sites. `0x454980` leaves `x² + y²` of a vector in `st0`, and `0x4549f0` leaves
`(x² + y²) + z²`, both at 53 bits. `0x4549c0(s)` divides a vector in place: it computes `1/s` at
53 bits once and stores each component times it as a float.

Addresses: `0x4549a0`, `0x454980`, `0x4549f0`, `0x4549c0`
Confidence: verified
Proof: one
Test: `spline::vec2_len2_diff`, `spline::vec3_len2_diff`, `spline::vec_div_scalar_diff`

`0x4549a0` is three `mov`s and `ret 0xc`. `0x4549c0` is `fld 1.0; fdiv [s]` and three
`fmul; fstp`. `0x454980` has 5 call sites (`0x4546fa`, `0x6350fa`, `0x635de1`, `0x7c56c3`,
`0x7c56f6`), `0x4549f0` 60 and `0x4549c0` 5.

## Integer square root

`0x454a10(n)`, with `n` in `ecx`, returns `⌊√n⌋` in `eax` for every unsigned 32-bit `n`. The first
estimate is `⌊n·0xaaaaaaab/2³⁵⌋ + 1` for `n ≤ 0xff`, `⌊n·0x51eb851f/2³⁸⌋ + 21` for `n ≤ 0xffff`, and
otherwise `⌊(⌊(n − t)/2⌋ + t)/2¹⁴⌋ + 0x1bc` with `t = ⌊n·0x39acc69d/2³²⌋`. Each step is `q = n / e`,
`d = e − q`, `e = (q + e)/2`, both halves truncating toward zero, and the loop ends when `d/2` is 0.

Addresses: `0x454a10`, `0x4ea255`, `0x454a60`
Confidence: verified
Proof: one
Test: `spline::isqrt_diff`

`0x454a60` is the loop: `div esi`, `cdq; sub eax,edx; sar 1` for each half. The result equals the
exact floor root for all 2³² inputs (a C transcription of the listing, run over the whole range).
The only caller is `0x4ea255` in the interface code.

## Object file and initializers

The file's header globals are three floats, +∞ at `0xb05db0`, 2π at `0xb05e50` and 1/2π at
`0xb05dd4`, set by `0x4532c0` through `0x4532d0`, `0x454930` through `0x454940` and `0x454950`
through `0x454960`. +∞ is the float at `0x8029c0`; 2π is the float π at `0x8029c4` doubled; 1/2π is
1.0 divided by 2π at 53 bits. Nothing reads +∞ or 1/2π.

Addresses: `0x4532c0`, `0x4532d0`, `0x454930`, `0x454940`, `0x454950`, `0x454960`, `0xb05db0`, `0xb05e50`, `0xb05dd4`, `0x8029c0`, `0x8029c4`
Confidence: verified
Proof: one
Test: none

A scan of the listing and of every dword of the image finds no reader of the three globals except
`0x454966`, the 1/2π initializer reading 2π. The C runtime's run `0x82753c` to `0x827554` lists the
file's seven initializers.

## Unreached code

Nothing live reaches `0x453330`, `0x4534d0`, `0x4536d0` and `0x454300`. The census is a scan of every
`call`, `jmp` and `jcc` target in the listing and of every dword of the image for each address; its
control is `0x453390`, with four call sites. `0x4536d0` has one caller, the unreached `0x454300`;
the other three have none and no stored address.

Addresses: `0x453330`, `0x4534d0`, `0x4536d0`, `0x454300`, `0x454314`, `0x453390`
Confidence: verified
Proof: one
Test: none

`0x4534d0` forwards three stack arguments to slot 8 (`call [eax+0x20]`). `0x4536d0` is the body of
`0x453640` with degree 1 and rows 8 bytes apart (`add edx,8` at `0x453723`). `0x454300` calls it at
`0x454314` with `0xb05db4`.
