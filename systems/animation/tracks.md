# animation: tracks

How the pass reads an animated value. A track is a 0x1c-byte record in the shared model: an
interpolation word, a global sequence word, and count and pointer pairs for ranges, key times and
values. The key search turns a time into a pair of keys and a fraction, and each element family reads its
values from the pair by a fixed block of code. This doc gives the search, the shape of a sampling block
with its blend, the interpolations and the quaternion blend. `models/animation#key-lookup` states the
search in short. Matrix and quaternion routines that the client picks by processor are stated for the one
a current SSE processor runs (`models/animation#dispatched-matrix-math`).

Roots: none

## Key search

`FindKey(t, range, track, out)` takes the window `[lo, hi]` from the track's range `range`, or `[0, keys − 1]`
when the track has no ranges. When `lo ≥ hi` (unsigned) it stores `{lo, lo, 0.0}`. The time is the argument,
or `[+0x64][gs]` for a global sequence word `gs` below 0xffff. With `k` the index stored in `out[0]` by the
last search and `d = t − ts[k]` (32-bit): `d < 500` scans forward while `k < hi` and `ts[k+1] ≤ t`; `d` in
`[−500, 0)` scans back while `k > lo` and `ts[k] > t`; any other `d` rereads `d` from `ts[lo]`, scans forward
from `lo` for `d < 500`, and bisects otherwise.

Addresses: `0x713d50`, `0x713d5c`, `0x713d75`, `0x713d7b`, `0x713d96`, `0x713db5`, `0x713dc2`, `0x713dc9`, `0x713dd3`, `0x713de1`, `0x713de8`, `0x713df1`, `0x713dff`, `0x713e06`, `0x713e21`, `0x713e43`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`, `animation::m2_track_search_difftest`, `animation::m2_track_sample_byte_difftest`, `animation::m2_track_sample_float_difftest`, `animation::m2_track_sample_quat_difftest`

All compares are unsigned. The hint `k` is read without a bound check, and the forward and backward scans
do not clamp it to the window. On the forward leg a hint below `lo` scans forward from there and one at or above `hi` is
returned as it is (`0x713dcb`). On the backward leg a hint at or below `lo` is returned as it is (`0x713dec`) and one at or above `hi`
walks down from itself. The forward scan tests the bound
before the time (`0x713dcb`, `0x713dd3`: equal times advance); the backward scan exits at `lo` without
reading `ts[lo]` (`0x713dec`). The bisection takes `mid = (lo + hi) >> 1`; `t < ts[mid]` sets `hi = mid − 1`;
otherwise `lo = mid + 1` and `t < ts[mid+1]` ends it at `mid`. A window that collapses ends at the last
`lo` (`0x713e43`), and `hi = mid − 1` wraps when `mid` is 0.
`animation::m2_animate_difftest` runs `0x714260` whole. `animation::m2_track_search_difftest` runs
`0x713d50` whole. `animation::m2_track_sample_byte_difftest` runs `0x71ae90` whole, which searches with
`0x713d50`. `animation::m2_track_sample_float_difftest` runs `0x71af20` whole, which searches with
`0x713d50`. `animation::m2_track_sample_quat_difftest` runs `0x713ea0` whole, which searches with
`0x713d50` and blends with `0x74d11a`.

## Key fraction

After the search the pair is `{k, k+1}` when `k + 1` is below the track's key count `[+0xc]`, else `{k, k}`
with fraction 0.0; the count is the track's, not the window's, so the last key of a range pairs with the
first of the next. The fraction is `float(num / den)`, `num = t − ts[k]` read as an unsigned 32-bit value
in a 64-bit integer, `den = ts[k+1] − ts[k]` read as a signed 32-bit divisor. The division runs at 53 bits
and `fstp dword` rounds once more to a float. Two equal stamps divide by zero, which the control word masks.

Addresses: `0x713e45`, `0x713e48`, `0x713e4e`, `0x713e51`, `0x713e56`, `0x713e64`, `0x713e6e`, `0x713e79`, `0x713e7c`, `0x713e87`, `0x713e90`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`, `animation::m2_track_search_difftest`, `animation::m2_track_sample_byte_difftest`, `animation::m2_track_sample_float_difftest`, `animation::m2_track_sample_quat_difftest`

`0x713e4e cmp edx,edi; jae 0x713e87` stores `{k, k, 0}` at `0x713e8b` to `0x713e90`. `0x713e64 sub esi,eax` and
the zero stored at `ebp-4` (`0x713e6b`) make the numerator a qword read by `fild qword`; the denominator
is `fidiv dword` of `ecx − eax` at `0x713e79`. A numerator at or above 2^31 therefore divides as a large
positive value.
`animation::m2_animate_difftest` runs `0x714260` whole. `animation::m2_track_search_difftest` runs
`0x713d50` whole. `animation::m2_track_sample_byte_difftest` runs `0x71ae90` whole, which searches with
`0x713d50`. `animation::m2_track_sample_float_difftest` runs `0x71af20` whole, which searches with
`0x713d50`. `animation::m2_track_sample_quat_difftest` runs `0x713ea0` whole, which searches with
`0x713d50` and blends with `0x74d11a`.

## Sampling block

Every sampled track runs the same block. It runs only when the track's key count exceeds the first-update
gate. It searches with the primary clock `(time = +0x98, slot = +0x9c)` of a bone state, writes the value,
and stops there for an interpolation word of 0. Otherwise, when that state's blend weight `+0x10c` is not equal to 0
(a negative or NaN weight runs the leg) and the track's global sequence word is 0xffff, it searches again with the blend clock `(+0xc4, +0xc8)` into the second slot and sets the
value to `value + (second − value) · weight`. Byte and word values are not blended: the second value is
fetched and stored, and the first is kept.

Addresses: `0x7159c6`, `0x715a39`, `0x715a3f`, `0x715a4a`, `0x715a50`, `0x715a74`, `0x715abf`, `0x715ad0`, `0x715ea3`, `0x715eb0`, `0x715eb4`, `0x715ef6`, `0x716454`, `0x71648f`, `0x7164f2`, `0x71ae90`, `0x71aec6`, `0x71af20`, `0x71af7b`, `0x71afca`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`, `animation::m2_track_sample_byte_difftest`, `animation::m2_track_sample_float_difftest`

The bone state is the bone's own for a bone track, bone 0's for colour, transparency, texture frame, texture
transform and camera tracks, and the state of the bone the record names for light, ribbon, particle and attachment tracks.
The weight test is `fld 0.0; fcomp [state+0x10c]; test ah,0x44; jnp skip`: `0x44` masks C3 and C2. An equal compare sets only C3, one bit, odd parity, so `jnp` skips. A less or greater compare sets neither and an unordered one sets both, even parity, so a positive, negative or NaN weight runs the leg. The blend leg reads `word [track+2] = 0xffff`
(`0x715a50`), so a track tied to a global sequence never blends. A state is `{k, k+1, fraction} @ +0x0`,
value `@ +0xc`, the second `{k, k+1, fraction} @ +0x10` for 4-byte and smaller values (`+0x18` for
vectors) and the second value after it (`+0x1c`, `+0x24`). Texture frames fetch a word at `k` even
when the interpolation word is not 0 (`0x715ea8`).
`animation::m2_animate_difftest` runs `0x714260` whole. `animation::m2_track_sample_byte_difftest` runs
`0x71ae90` whole, which searches with `0x713d50`. `animation::m2_track_sample_float_difftest` runs
`0x71af20` whole, which searches with `0x713d50`.

## Value codecs

A track's values come in five shapes. A vector is three floats (stride 12) and a scalar one float (stride 4);
both are `a + (b − a) · t` per component, `a` and `b` the values at `k` and `k+1`, summed at 53 bits and
narrowed once. A 16-bit fixed value is a signed word times the float 1/32767 (`0x38000100`); its step form is
`f32(float(a) · S)` and its linear form is `f32(a·S + (b·S − a·S) · t)` with the products kept at 53 bits.
A byte or a word is the value at `k`. The step form of a vector or scalar copies the value at `k`.

Addresses: `0x7159d1`, `0x7159f4`, `0x715a02`, `0x715b2f`, `0x715b4e`, `0x715b51`, `0x715ced`, `0x715d0a`, `0x715d3c`, `0x715d61`, `0x71aff0`, `0x71b010`, `0x717f26`, `0x717f34`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The interpolation word is read as `cmp word [track],0`: 0 selects the step form and any other value the linear form
at every vector, scalar and fixed-point site outside the camera loop. Bone tracks (rotation, scale, translation), the colour, light,
ribbon, particle and texture transform tracks have no other test, so a bone track with the word 2 or 3
still interpolates linearly. `0x71aff0(this, i)` returns `[this+4] + 2i`, and `0x71b010` copies one
word; the fixed-point sites call them for both keys.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Vector Hermite

The camera loop is the one place that switches on the interpolation word: 1 is linear, 2 a Bézier curve, 3 a
Hermite spline, and any other word writes nothing. A vector key is nine floats: value, in tangent (`+0xc`), out tangent (`+0x18`).
With `v0`, `out0` from the key at `k` and `v1`, `in1` from `k+1`, the Hermite result is `((h11·in1 + h00·v0) + h01·v1) + h10·out0` for x and
`((h11·in1 + h10·out0) + h01·v1) + h00·v0` for y and z. `h00 = f32((A − B) + 1)` with `A = f32(2t³)` and
`B = f32(3t²)`, `h10 = f32((t³ − 2t²) + t)`, `h01 = B − A` and `h11 = t³ − t²`, the last two at 53 bits.

Addresses: `0x716b4c`, `0x716b51`, `0x716b86`, `0x716b9e`, `0x716bbb`, `0x716bc7`, `0x716bd2`, `0x716bdb`, `0x716bfb`, `0x716c1d`, `0x716cf1`, `0x716d1d`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The dispatch is `movzx edx,word [esi]; dec; je 0x716cf1` (linear), `dec; je 0x716c41` (Bézier), `dec; jne
0x716d1d`. Hermite spills `A` at `ebp-4` and `B` at `ebp-0xc`; `h00` goes to `ebp+0x14` and `h10`, computed
from the 53-bit `t³`, `t²` and `t`, to `ebp+0x10`; `h01` and `h11` stay on the stack. The x accumulate is at
`0x716bdb` to `0x716bfb`, y from `0x716bfe`, z from `0x716c1d`. Linear at `0x716cf1` is `a + (b − a) · t` on
the values. Six blocks share this shape: two vector tracks and the scalar roll, each with a second block for the
blend search.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Vector Bézier

The Bézier result takes the control points `v0`, `out0`, `in1`, `v1` with the weights `B0 = (1 − t)³`,
`B1 = 3t(1 − t)²`, `B2 = 3t²(1 − t)` and `B3 = t³`. It is `((B0·v0 + B3·v1) + B1·out0) + B2·in1` for x and
`((B2·in1 + B1·out0) + B0·v0) + B3·v1` for y and z. The weights are formed as `B3 = C = f32(t³)`,
`D = f32(3t²)`, `E = f32(3C)`, `B0 = f32(((D − C) − 3t) + 1)`, `B1 = (E − 6t²) + 3t` and `B2 = D − E`, the last
two at 53 bits.

Addresses: `0x716c41`, `0x716c4c`, `0x716c57`, `0x716c70`, `0x716c7c`, `0x716c8e`, `0x716c96`, `0x716cb0`, `0x716cb3`, `0x716cd2`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

`0x716c4c` stores `C` at `ebp+0x10`, `0x716c57` stores `D` at `ebp-0xc`, `0x716c70` narrows `B0` to
`ebp+0x14`, and `0x716c7c` stores `E` at `ebp-4`. `0x716c81` multiplies `t²` by 6.0 and `0x716c87 fsubr` takes
`E` minus it; `0x716c8a` adds `3t` and `0x716c90` forms `B2` from the spills. The x accumulate is
`0x716c96` to `0x716cb0`: `B0·v0`, `B3·v1`, `B1·out0`, `B2·in1`.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Scalar cubics

The roll track of a camera has the same four forms on a key of three floats: value, in tangent, out tangent
(stride 12). The basis is kept at 53 bits, with one narrowing at the store, except the Bézier's `3t²`, which is
spilled to a float. Hermite is `((h10·out0 + h11·in1) + h00·v0) + h01·v1` with `h10 = t³ − 2t² + t`,
`h11 = t³ − t²`, `h00 = 2t³ − 3t² + 1` and `h01 = 3t² − 2t³`. Bézier is `((B1·out0 + B2·in1) + B0·v0) + B3·v1`
with `B1 = (3t³ − 6t²) + 3t`, `B2 = f32(3t²) − 3t³`, `B0 = ((f32(3t²) − t³) − 3t) + 1` and `B3 = t³`.

Addresses: `0x7173cc`, `0x7173d9`, `0x7173ec`, `0x717408`, `0x717419`, `0x717445`, `0x71745d`, `0x717466`, `0x71748c`, `0x7174bf`, `0x7174c6`, `0x7174d9`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The step form copies the float at `k·12` (`0x7173d9`). The Hermite block `0x717419` to `0x71745d` never stores
an intermediate. The Bézier block spills only `f32(3t²)` at `ebp+0x10` (`0x71748c`). The second sample for the
blend uses the same code with the state's `+0x10` triple (`0x71751e` onward) and `value2` at `+0x1c`.
`animation::m2_animate_difftest` runs `0x714260` whole.

## Rotation sample

The rotation sampler `0x713ea0(track, out)` searches with the state's primary clock. For an interpolation
word of 0 it copies the key at `k` to the value and returns. Otherwise it sets the value to the componentwise
`a + (b − a) · t` of the keys at `k` and `k+1`, narrowing each component, with no renormalization. Then,
when the weight is not equal to 0 (a negative or NaN weight runs the leg) and the track's global sequence word is 0xffff, it searches the
blend clock, lerps the blend keys the same way into a temporary quaternion at `out+0x28`, and calls the
quaternion blend `0x74d114(value, value, temporary, weight)`.

Addresses: `0x713ea0`, `0x713ec2`, `0x713ecc`, `0x713ed2`, `0x713ef7`, `0x713f0c`, `0x713f3f`, `0x713f41`, `0x713f58`, `0x713f5b`, `0x713f7d`, `0x713f9c`, `0x713fa1`, `0x713fd7`, `0x713fe3`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`, `animation::m2_track_sample_quat_difftest`

The keys are 16 bytes (`shl eax,4`), the track values at `[track+0x18]`. `0x713ecc cmp word [edi],0` and
`jne 0x713ef7` separate the copy at `0x713ed2` from the lerp. The blend leg's gate is `0x713f47 fcomp
[ebx+0x10c]; test ah,0x44; jnp` and `0x713f5b cmp word [ecx+2],0xffff`. The call pushes the weight,
the temporary's address, the value address twice (`0x713fd7` to `0x713fe3`).
`animation::m2_animate_difftest` runs `0x714260` whole. `animation::m2_track_sample_quat_difftest` runs
`0x713ea0` whole, which searches with `0x713d50` and blends with `0x74d11a`.

## Quaternion blend

`0x74d11a(out, A, B, t)` blends two quaternions. With `d = ((Ax·Bx + Az·Bz) + Ay·By) + Aw·Bw`, `s = −1` when
`d < 0` or unordered and else 1, and `c = s·d`: when `1 − c ≤ 2^-23` or is not a number, `cA = 1 − t` and
`cB = t·s`. Otherwise `θ = f32(atan2(sqrt(1 − c²), f32(c)))`, `inv = f32(1 / sqrt(1 − c²))`,
`cA = sin(θ · (1 − t)) · inv` and `cB = sin(θ · t) · inv · s`. The result is `f32(cA·A_i + cB·B_i)` for each
component, the sum at 53 bits.

Addresses: `0x74d114`, `0x74d11a`, `0x74d123`, `0x74d147`, `0x74d152`, `0x74d15b`, `0x74d165`, `0x74d170`, `0x74d17d`, `0x74d190`, `0x74d192`, `0x74d199`, `0x74d1a3`, `0x74d1b2`, `0x74d1b7`, `0x74d1bc`, `0x74d1f6`, `0x876588`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`, `animation::m2_track_sample_quat_difftest`, `animation::sample_quat_slerp_difftest`

The slot `0x876588` holds this body unless a 3DNow set replaces it (`models/animation#dispatched-matrix-math`). The dot
sums in the order given (`0x74d12b` to `0x74d145`). `fcom 0.0; test ah,1; jne` picks `s = −1.0` (`0xbf800000`).
`1 − c` is compared with `2^-23` (`0x34000000`) by `fcomp; test ah,0x41; jne 0x74d1b7`. `fpatan` takes `(sqrt(1 − c²),
f32(c))` and the spills of `θ` (`0x74d192`) and `inv` (`0x74d199`) are floats. `fsin` runs at 53 bits on `θ·(1 − t)`
with `1 − t` the 53-bit register of `0x74d123`. `fsin` and `fpatan` give the full-precision value the instruction
approximates, which differs by processor.
`animation::m2_animate_difftest` runs `0x714260` whole. `animation::m2_track_sample_quat_difftest` runs
`0x713ea0` whole, which searches with `0x713d50` and blends with `0x74d11a`.
`animation::sample_quat_slerp_difftest` runs `0x74d11a` whole.

## No pose filter

The bone loop reads from a bone's state only the clocks, the records and the speeds, the flags, the override
pointer, the fade fields, the key hints and, for a track the gate skips, its stored value. It never mixes the
new sample with the last frame's pose: the one carried datum is `out[0]`, the index that starts the next
search, and the stored fraction is rewritten by every search before it is read. The blend slots and the
smoothstep weight are the only mixing, and they mix two clocks of the same pass.

Addresses: `0x713d50`, `0x713db5`, `0x714d26`, `0x714e02`, `0x714fc0`, `0x714f6a`, `0x714d3f`, `0x71491b`, `0x714923`
Confidence: verified
Proof: one
Test: `animation::m2_animate_difftest`

The rotation block `0x714d26 cmp eax,[ebx+0x8c]; jbe 0x714d3f` skips the sample but still builds the matrix from
the stored quaternion at `+0x3c`; the scale (`0x714e02`, `0x714f6a`) and translation (`0x714fc0`) blocks do the
same with `+0x74` and `+0xc`. `0x713db5 mov ebx,[eax]` reads `out[0]` before anything else in the search reads `out`.
The weight is 0 at `0x714923` once the fade has ended, so no sample leaves a trace after that.
`animation::m2_animate_difftest` runs `0x714260` whole.
