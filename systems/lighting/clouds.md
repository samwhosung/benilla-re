# lighting: clouds

The cloud layer: a procedural cover texture regenerated a band at a time, lit by the sun or the
white moon, drawn on a shallow dome above the gradient. Its object is `0xce98e8`. The cover
threshold follows the light table's scalar band 3; the colours follow colour bands 9 to 11 and the
storm weight.

Roots: `0x6cfa90`, `0x6d1d60`

## Cloud texture

The texture is a square of cols × cols texels, cols = 128 at `SkyCloudLOD` 0 and 256 at 1 (the
setting clamped to [0, 1], default 0), with a row shift of 7 or 8. Every 0.1 s of frame time the
cloud update builds the next 32 rows: noise cover, then colour, then a device update of that band;
past the last row it wraps to row 0 and advances the noise phase. A full rebuild regenerates every
row at once. The texture starts white and opaque.

Addresses: `0x6d09b0`, `0x6d0a2f`, `0x6d0a58`, `0x6d0ab8`, `0x6d0bd8`, `0x6d0be3`, `0x6d1d60`, `0x6d1d8f`, `0x6cffc0`, `0x6cffcf`, `0x6cfff4`, `0x6d04e3`, `0x6d04fe`, `0x6cff90`, `0x6cffa8`, `0x6d0890`, `0x811548`, `0x81155c`, `0x8115b4`, `0xce9d18`
Confidence: verified
Proof: one
Test: none

`0x6d09b0(lod, rows)` stores cols `[0x811548 + 4·lod]` and the shift `[0x81155c + 4·lod]`, rows 32
when the argument is 0, fills the colour buffer with 0xffffffff and binds it to a device texture
with the callback `0x6d0890`. `0x6d1d60` clamps the parsed value and calls it. `0x6cffc0` returns
unless the texture exists (`0x6cffcf`), subtracts state `+0x3c` from `+0xb0`, and at or below 0
or NaN (`0x6cfff4`) resets it to `[0x8115b4]` (0.1) and builds the band, updates the texture rows
(`0x6d04e3`) and wraps (`0x6d04fe`). `0x6cff90` sets the band height to cols, starts at row 0
and calls `0x6cffc0` with the countdown at 0 (`0x6cffa8`).

## Cloud cover

Each texel's cover byte is curve[b − T], or 0 when b < T, with b the low byte of
bits(f32(n·64 + 128 + 512)) >> 14 for the noise value n and T = trunc((1 − C)·255) for the cover
C, scalar band 3 (table `+0x58`). The curve is built once at start-up from T = 102:
curve[i] = trunc(255 − 255·0.96^(i·d)), d = (255 − 102)/256 accumulated as a float.

Addresses: `0x6d0970`, `0x6d0984`, `0x6d0996`, `0x6d09a3`, `0x6d0900`, `0x6d0914`, `0x6d0928`, `0x6d0937`, `0x6d0942`, `0x6d0953`, `0x6d0010`, `0x6d0477`, `0x6d0489`, `0x6d0499`, `0x6d049b`, `0x808de4`, `0x80653c`, `0xce91d8`, `0xce9c64`, `0x6d1cc9`, `0x6d1cd8`
Confidence: verified
Proof: one
Test: none

`0x6d0970(c)` uses `+0xc` instead when it is not 0 (`0x6d0984`), and stores `__ftol((1 − c)·255)`
in byte `+0x8` (`0x6d09a3`). The band starts with `0x6d0970([0xce9c64])` (`0x6d0010`). `0x6d0900(γ)`
takes d = (255 − T)·(1/256) (`0x6d0928`) and stores `__ftol(255 − 255·pow(γ, x))` (`0x6d0942`),
x from 0 by d; `pow` is the C runtime's `_CIpow` with the base below the exponent on the stack, as
at `0x6cf6a2`. The initializer calls `0x6d0970(0.6)` and `0x6d0900(0.96)` (`0x6d1cc9`, `0x6d1cd8`).
The texel loop multiplies by 64.0, adds 128.0 and 512.0, `fstp`s and shifts (`0x6d0477`,
`0x6d0489`), subtracts T and indexes the curve when not negative (`0x6d0499`, `0x6d049b`).

## Cloud noise

The noise value n is four octaves of value noise, each half the amplitude of the one before:
lattice values from a table of 256 random floats 1 − 2·rand/32767 filled at start-up, a 256-entry
permutation, and a fade table 0.5·(1 − cos(i·π/256)) for the interpolation weights, stepped in
16-bit fixed point by a per-octave step from `0x86f3dc` and advanced by the noise phase. The third
octave's partial sum also gives each texel a surface slope, its difference to the previous texel
and the previous row times 2^(shift − 7).

Addresses: `0x6d0c90`, `0x6d0ca0`, `0x6d0cb4`, `0x6d0ce8`, `0x6d0cf6`, `0x6d0c00`, `0x86f2d0`, `0x86f3dc`, `0xce92d8`, `0xce8dd8`
Confidence: inferred
Proof: none
Test: none

`0x6d0c90` fills `0xce92d8` with `1 − 2·rand()·(1/32767)` through `_rand` (`0x6d0ca0`, `0x6d0cb4`)
and `0xce8dd8` with the fade (`0x6d0ce8`, `0x6d0cf6`); `0x6d0c00` sets four octaves at `+0x28`.
The octave and lattice addressing is read from the decompiled body of `0x6cffc0`, not settled at
the bytes. The lattice values depend on the C runtime's random state when the sky is built.

## Cloud colours

Each texel with cover a > 0 gets alpha a and colour base + glow, each channel clamped to 1 and
packed as bits(f32(c·255 + 512)) >> 14. base = band 10·p + band 11 with p = (((255 − a) >> 1) +
64)/255. glow = band 9·k when the angle term k is positive: k = cos θ · (1 − 0.75·w) · E(time),
θ between (s − texel, 192·w + 64) and (slope, 1), s the light's texel position, w the storm weight
and E the track at `0xce9ab8`. The light is the sun disc from 04:50 to 22:10, else the white
moon. A texel with no cover copies the previous texel's colour with alpha 0.

Addresses: `0x6cfb00`, `0x6cfb43`, `0x6cfb5c`, `0x6cfd2a`, `0x6cfd36`, `0x6cfd3f`, `0x6cfd4d`, `0x6cfd57`, `0x6cf9c0`, `0x456330`, `0x811608`, `0x8012cc`, `0x6ce390`, `0xce9ab8`, `0xce9d00`, `0xce9d04`
Confidence: verified
Proof: one
Test: none

`0x6cfb43` and `0x6cfb5c` pick the sun disc for 0.2291667 − 0.0277778 ≤ t ≤ 0.8958333 +
0.0277778. Its direction from the eye meets the unit sphere centred cos 45° below the eye
(`0x6cf9c0`) and is projected (`cover-projection`); state `+0x1a0` and `+0x1a4` keep it over cols.
`0x6cfd2a` to `0x6cfd36` give 192·w + 64; `0x6cfd3f` evaluates `0xce9ab8` and `0x6cfd4d` to
`0x6cfd57` multiply by 1 − 0.75·w. `0x456330` gives both inverse lengths. Bands 9, 10 and 11 are
read at `0xce9c30`, `0xce9c34` and `0xce9c38`, bytes 2, 1 and 0 as red, green and blue.

## Cloud envelope

The envelope E has eight keys from `0x6ce390`, the last four out of order: 04:00 1, 04:40 0,
04:50 0, 05:30 1, 21:30 1, 22:10 0, 21:20 0, 22:00 1. By the track rule E falls from 1 to 0 over
21:30 to 22:10 and is 1 from 22:10 to 04:00, dips to 0 over 04:00 to 04:50 and rises back by 05:30.

Addresses: `0x6ce390`, `0x6ce3da`, `0x6ce434`, `0x6ce446`, `0x8115ac`, `0x81151c`
Confidence: verified
Proof: one
Test: none

`0x6ce3da` stores 04:00 + 0:40 for the second key; the sixth is 21:30 + 0:40 (`0x6ce434`) and the
seventh 22:00 − 0:40 (`0x6ce446`), with `[0x8115ac]` = 1/36 day. After 22:10 no key time is at or
above the phase, so the evaluator interpolates from the last key (22:00, 1) to the first.

## Cover projection

A point maps to the cloud texture by its direction from the eye with the vertical raised by cos 45°:
r = min(acos(dz'/|d'|), 0.25π)/(0.25π)·0.5, and with h the horizontal length, u = (dx/h·r + 0.5)·cols
and v = (dy/h·r + 0.5)·cols, or 0.5·cols each when h ≤ 1e-5. The cover sampler reads byte
(trunc v << shift) + trunc u and divides by 255, or returns 0 before the texture exists.

Addresses: `0x6cf870`, `0x6cf8bd`, `0x6cf8fd`, `0x6cf91f`, `0x6cf92f`, `0x6cf947`, `0x6cf995`, `0x6cfa90`, `0x6cfa9e`, `0x6cface`, `0x6cfae5`, `0x741820`, `0x811600`
Confidence: verified
Proof: one
Test: none

`0x6cf8bd` subtracts −cos(0.25π) from dz. `0x6cf8fd` calls `_CIacos` on dz' over the length;
`0x6cf91f jne` keeps an angle at or below 0.25π, a NaN included. `0x6cf947` compares h with the
double 1e-5. The products are `fimul` by cols (`0x6cf995`). `0x6cfa90` truncates with `__ftol`
(`0x6cface`); u = cols reads the next row's first byte, and v = cols, at dx = 0 with the angle at
or above 0.25π, reads a row past the buffer.

## Cloud dome

The clouds draw on a dome of one apex and eleven rings of 16 columns at polar angles 0, 0.025π,
0.05π, … 0.205π, 0.23π, 0.245π, 0.25π, built at radius 1.0 and dropped by cos 45° like the
gradient. Ring k's texture coordinate is (sin φ, cos φ)·k/22 + 0.5 and its colour white with alpha
255 for rings 0 to 8, 128 for ring 9 and 0 for rings 10 and 11. The draw uses the texture, blend 2
and render states 0xe, 0xf, 0x12 and 0x14 at 0; it skips a cloud object without a texture.

Addresses: `0x6d0530`, `0x811570`, `0x8115a0`, `0x81160c`, `0x6d1cba`, `0x7e5f80`, `0x7e5f8e`, `0x7e5fd9`, `0x7e601e`
Confidence: verified
Proof: one
Test: none

`0x6d0530(r)` walks the twelve floats at `0x811570` times π with 16 columns of 2π/16, stores
vertices as the sky dome does, UVs `sin·(k·[0x81160c]·0.5) + 0.5`, and colours `byte[0x8115a0 +
k] << 24 | 0xffffff`, 374 indices (0x176). The initializer calls it with 1.0 (`0x6d1cba`).
`0x7e5f80` sets state 7 to 2 (`0x7e5fd9`); its other colour path, a single colour from
`0xce99d0`, sits behind the never-set flag `0xce99f8` (`0x7e601e`, `fog#dead-fog-modifiers`).
