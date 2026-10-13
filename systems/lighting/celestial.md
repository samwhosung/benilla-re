# lighting: sun, moons and stars

The day-night tracks and the bodies they drive: the lighting sun's direction, the sun disc, the
white moon and the second moon, their glares, and the star model. The bodies are camera-centred
objects in the day-night file's globals; the draws are in the sky draw file (`0x7e4f90` onward).
The sky pass draws the stars and discs (`sky#sky-pass`); the glares draw late in the world frame.

Roots: `0x6cf490`, `0x6cf7b0`, `0x6cf7d0`, `0x6cf800`, `0x7e5780`, `0x7e57e0`, `0x6d48c0`

## Tracks

A track is a list of keys {time, value}, time a fraction of the day. `0x6cf6c0` clamps the phase to
[0, 1] (NaN as 1), takes the first key i with phase ≤ time[i] and the key j before it, cyclically
(the last key when i is the first or when no key qualifies, i then the first). With d = time[i] −
time[j] it returns value[j] when |d| < 0.001; otherwise it adds 1 to a negative d and to a
negative e = phase − time[j], and returns value[j] + (value[i] − value[j])·e/d at 53 bits, written
as value[j] − (value[j] − value[i])·e/d when value[i] < value[j].

Addresses: `0x6cf6c0`, `0x6cf6d1`, `0x6cf6ea`, `0x6cf703`, `0x6cf70b`, `0x6cf714`, `0x6cf71d`, `0x6cf72f`, `0x6cf73f`, `0x6cf752`, `0x6cf76b`, `0x6cf773`, `0x6cf786`, `0x801360`
Confidence: verified
Proof: one
Test: none

`0x6cf6c0(keys, count, phase)` in ecx, edx and the stack, `ret 4`, value in st0. The search
`fcomp [key]`, `test ah,0x41`, `jnp` exits at phase ≤ time (`0x6cf70b`). `0x6cf714` handles no
match and `0x6cf71d` the first key. `fabs`, `fcomp [0x801360]` (0.001) returns value[j]
(`0x6cf73f`). `0x6cf773` is DE F1, ST(1) ← ST(0)/ST(1), e over d. `0x6cf786 jne` takes the
value[i] < value[j] arm, which subtracts. A track whose keys are out of order still searches by
this rule.

## Static tracks

The day-night file builds these tracks once, from static initializers or at first use. Times are
written as clock times; angles are π times a constant, shown in degrees.

| track | keys | read by |
|---|---|---|
| `0xce8d90` | 00:00, 06:00, 12:00, 18:00 all 225° | sun azimuth |
| `0xce8db0` | 00:00 127°, 06:00 110°, 12:00 127°, 18:00 110° | sun polar angle |
| `0xce8d4c` | 05:30, 12:00, 21:30 all 45° | sun disc azimuth |
| `0xce8d64` | 05:30 100°, 11:55 5°, 12:00 5°, 12:05 5°, 21:30 100° | sun disc polar angle |
| `0xce8cac` | 06:00 2, 06:45 1, 20:15 1, 21:00 2 | sun disc size |
| `0xce8d0c` | 00:00, 04:00, 22:00 all 45° | white moon azimuth |
| `0xce8d24` | 00:00 35°, 00:05 35°, 04:00 100°, 22:00 100°, 23:55 35° | white moon polar angle |
| `0xce8ccc` | 00:00 135°, 04:00 150°, 22:00 165° | moon02 azimuth |
| `0xce8ce4` | as `0xce8d24` | moon02 polar angle |
| `0xce8c8c` | 01:00 1, 04:00 1.5, 22:00 1.5, 23:59 1 | both moons' size |
| `0xce9a98` | 03:00 1, 04:30 0, 22:30 0, 24:00 1 | stars |
| `0xce9a58` | 03:00 2, 04:00 4, 05:30 4, 06:30 2, 19:30 2, 21:30 4, 22:00 4, 23:00 2 | nothing |

Addresses: `0x6d3a10`, `0x6d3a3b`, `0x6d3ab8`, `0x6d3b80`, `0x6ce4a0`, `0x6ce560`, `0x811508`, `0x81163c`, `0x811638`, `0x80c5d0`, `0x811650`, `0x811648`, `0x810018`, `0x811644`, `0xce8d90`, `0xce8db0`, `0xce8d4c`, `0xce8d64`, `0xce8cac`, `0xce8d0c`, `0xce8d24`, `0xce8ccc`, `0xce8ce4`, `0xce8c8c`, `0xce9a98`, `0xce9a58`
Confidence: verified
Proof: one
Test: none

The times are the floats 0.2291667 (05:30), 0.5, 0.8958333 (21:30), 0.9166667 (22:00), 0.1666667
(04:00) and 1.0 at `0x811510` to `0x811528`, with offsets 0.0034722 (5 min), 0.0208333, 0.0520833,
0.125 and 0.0826389 added or subtracted and each key rounded to a float. The angle constants are
0.7055556, 0.6111111, 1.25, 0.5555556, 1/36, 0.1944444, 0.25, 0.75, 0.8333333 and 0.9166667
times π (0x40490fdb). `0x6d3a10` and `0x6d3b80` build their tracks at first use behind guard bits;
`0x6ce4a0` and `0x6ce560` are static initializers. A census finds `0xce9a58` only in `0x6ce560`.

## Sun direction

The lighting sun is (cos a·sin p, sin a·sin p, cos p) at state `+0x168`, with a the sun azimuth
track and p the polar track at the time of day. Its bearing never changes and p stays between 110°
and 127°, so the direction always points down; it is the direction light travels.

Addresses: `0x6d3a10`, `0x6d3b12`, `0x6d3b17`, `0x6d3b25`, `0x6d3b2d`, `0x6d3b36`, `0x6d3b42`, `0x6d3b4f`, `0xce9cc8`
Confidence: verified
Proof: one
Test: none

The polar track's result is stored as a float (`0x6d3b17`), the azimuth's stays in st0. `fsin` of
the stored polar, `fcos` of the azimuth times it to `+0x168` (`0x6d3b36`), `fsin` of the azimuth
times it to `+0x16c` (`0x6d3b42`), `fcos` of the polar to `+0x170` (`0x6d3b4f`), each a float.
`fsin` and `fcos` approximate by CPU (`METHOD.md`).

## Celestial bodies

Each body sits 12 units from the camera eye along (cos a·sin p, sin a·sin p, cos p), normalized
after the x product is rounded to a float, with its own azimuth and polar tracks. The sun disc
and white moon use the time of day; moon02 uses the phase ((day + time) mod 1.7)/1.7, computed in
1/65536 steps. A body's size is its size track times its base scale: 1.0 for the sun disc, 1.75
for the white moon, 1.0 for moon02. The sun glare takes the sun disc's position, the moon glare the
white moon's position, and the moon glare's two scale ends the white moon's size.

Addresses: `0x6d3b80`, `0x6d3fda`, `0x6d3fed`, `0x6d4008`, `0x6d4033`, `0x6d403b`, `0x6d4067`, `0x6d408b`, `0x6d40a1`, `0x6d40b4`, `0x6d40c9`, `0x6d417a`, `0x6d4190`, `0x6d41a1`, `0x6d41ad`, `0x6d41b3`, `0x6d41bf`, `0x6d4207`, `0x6d4210`, `0x6d423a`, `0x6d4259`, `0x6d425f`, `0x6d426c`, `0x6d4322`, `0x80c4e8`, `0x80a000`, `0x80a8b0`, `0xce9858`, `0xce9878`, `0xce9898`, `0x73ff3f`
Confidence: verified
Proof: one
Test: none

The sun disc: polar `0xce8d64` (`0x6d3fda`), azimuth `0xce8d4c` (`0x6d3fed`), x stored at
`0x6d4008`, `fsqrt` of the squared sum (`0x6d4033`), `fdivr [0x80c4e8]` (12.0, `0x6d403b`), each
component times it plus the eye (`0x6d4067`), size `0xce8cac` times `[0xce9870]` to `0xce986c`
(`0x6d40b4`). The white moon repeats it with `0xce8d24`, `0xce8d0c` and `0xce8c8c`; `0x6d4190`
multiplies by `[0xce9890]` and three stores keep the product (`0x6d41a1` to `0x6d41b3`). moon02's
phase sums `fistp(day·65536 − 0.5)` and `fistp(time·65536 − 0.5)` (`0x6d41bf`), subtracts
`fistp(floor((day + time)/1.7)·1.7·65536 − 0.5)` clamped to the sum (`0x6d423a`), times 1/65536
(`0x6d4259`), divided by 1.7 (`0x6d425f`).

## Day ramp

`0x6d3b80` ends by storing a ramp of the time of day at state `+0x1a8`: rising 05:30 to 12:00,
falling 12:00 to 21:30, rising 22:00 to 24:00, falling 00:00 to 04:00, else 0. Nothing reads it.

Addresses: `0x6d432f`, `0x6d4463`, `0xce9d08`
Confidence: verified
Proof: one
Test: none

A census of `0xce9d08` in the listing finds four `fstp` and one `mov` of 0 in `0x6d3b80`, ending at
`0x6d4463`, and no other instruction; the control `0xce9d0c` finds its two stores in `0x6d2260`.

## Body objects

The sky initializer builds the bodies: the sun glare (`0xce97a8`), the moon glare (`0xce96f8`), the
sun disc (`0xce9858`), the white moon (`0xce9878`), moon02 (`0xce9898`) and the stars (`0xce96d8`).
A glare holds an enabled flag, its position, a colour, a base scale, its scale, a rise and a fall
rate, an envelope, a texture, a 4-key visibility track, its scale ends, a look threshold, its
alpha ends and a look factor. A disc holds its position, colour, texture, size and base scale.

| glare field | sun glare | moon glare |
|---|---|---|
| `+0x20` base scale | 1.0 | 2.0 |
| `+0x28` rise, `+0x2c` fall per second | 4.0, 1.5151515 | 3.030303, 1.5151515 |
| `+0x70` visibility | 06:30 0, 07:30 1, 19:30 1, 21:00 0 | 02:00 1, 03:15 0, 22:45 0, 23:59 1 |
| `+0x90`, `+0x94` scale ends | 3.0, 20.0 | the white moon's size |
| `+0x98` look threshold | 0.7 | 0.7 |
| `+0x9c`, `+0xa0` alpha ends | 0.5, 1.0 | 0.1, 1.0 |
| texture | `Textures\sunGlare.blp` | `Textures\moonGlare.blp` |

Addresses: `0x6d1ba0`, `0x6d1cf8`, `0x6d1e30`, `0x6d1f30`, `0x6d2030`, `0x6d20a0`, `0x7e5000`, `0x7e5b10`, `0x7e60f0`, `0x6cec80`, `0x6ced30`, `0x6cec20`, `0x6cede0`, `0x8115d4`, `0x8115e8`, `0x80e004`, `0x811624`, `0x86f4f4`, `0x86f50c`, `0x86f54c`, `0x86f538`, `0x86f524`, `0x881c70`
Confidence: verified
Proof: one
Test: none

`0x6d1e30` and `0x6d1f30` load the texture through `0x7e5000`, which also creates the occlusion
query object at `+0x4` when the device supports one, and store the fields above; the times are
05:30 + 1:00 and + 2:00 (`[0x80e004]` = 1/12), 21:30 − 2:00 and − 0:30, 04:00 − 2:00 and − 0:45
(`[0x811624]`), 22:00 + 0:45 and + 1:59. `0x6d2030` loads `sunCenter.blp`, `moon.blp` and
`moon02.blp` through `0x7e5b10` with base scales 1.0, 1.75 (0x3fe00000) and 1.0, and moon02's
period 1.7. `0x6d20a0` creates the star model `0x881c70` in its own scene through `0x7e60f0`.
`0x6cec80` and `0x6ced30` zero the glares and store their method tables `0x8115d4` and `0x8115e8`.

## Glare update

Each frame `0x6cf490` sets a glare's target to the product of its cloud and submersion
occlusion, 1 − the largest active skybox weight, its visibility track at the time of day and, when
that is nonzero, its draw occlusion. The envelope `+0x30` moves toward the target by at most rise
or fall times the frame time. With c = camera forward · the unit direction to the glare, raised to
the look threshold, and f = (c − threshold)/(1 − threshold), the scale `+0x24` is lerp(scale ends,
f) times the base scale, the alpha byte becomes bits(f32(old/255 · lerp(alpha ends, f) · envelope ·
255 + 512)) >> 14, and the look factor `+0xa4` is c^10 · envelope.

Addresses: `0x6cf490`, `0x6cf4cf`, `0x6cf4f1`, `0x6cf4fe`, `0x6cf50e`, `0x6cf533`, `0x6cf557`, `0x6cf576`, `0x6cf58c`, `0x6cf592`, `0x6cf5a6`, `0x6cf5bd`, `0x6cf5e3`, `0x6cf61a`, `0x6cf641`, `0x6cf65a`, `0x6cf68b`, `0x6cf699`, `0x6cf6a2`, `0x6cf6aa`, `0x86f3d8`, `0x73f940`
Confidence: verified
Proof: one
Test: none

The unit direction is (pos − eye) times 1/√(sum of squares), each a float (`0x6cf4cf`). Slots 3
and 4 multiply in (`0x6cf4f1`, `0x6cf4fe`). The skybox weight is the override's when its handle is
set and its weight above 0, else the largest pair weight with a handle (`0x6cf533`). `0x6cf58c`
skips slot 2 for a target of exactly 0. The envelope rises to min(env + rise·dt, target)
(`0x6cf5bd`) and falls to max(env − fall·dt, target) (`0x6cf5e3`). `0x6cf61a jp` keeps c at or
above the threshold. `0x6cf641` is DE F9, (c − t)/(1 − t). `0x6cf6a2` calls the C runtime's
`pow(c, [0x86f3d8])`, 10.0. The alpha byte is reset by the table build each frame
(`daynight#body-colours`).

## Glare occlusion

A glare's three occlusion terms are its method table's slots 2 to 4. Slot 3 samples the cloud
cover at the glare: 1 − cover for the sun, 1 − |2·cover − 1| for the moon. Slot 4 is 1 dry and
1 − clamp(depth·0.1, 0, 1) underwater. Slot 2 returns 0 unless the glare is enabled and its alpha
byte is nonzero; then it runs the device occlusion test when the glare has a query object and the
`occlusion` setting is on, and the world trace otherwise.

Addresses: `0x6cf7b0`, `0x6cf7be`, `0x6cf7c3`, `0x6cf7d0`, `0x6cf7e3`, `0x6cf800`, `0x6cf810`, `0x6cf81d`, `0x6cf82e`, `0x6cf84d`, `0x7e5780`, `0x7e5787`, `0x7e579c`, `0x7e57aa`, `0x7e57af`, `0x801620`, `0xc7f31c`
Confidence: verified
Proof: one
Test: none

`0x6cf7b0` passes `this + 0xc` and 1.0 to the cloud sampler `0x6cfa90` (`clouds#cover-projection`) on
`0xce98e8` and returns `fsubr 1.0`; `0x6cf7d0` subtracts 0.5, doubles, takes `fabs` and subtracts
from 1 (`0x6cf7e3`). `0x6cf800` calls `0x672470` with a depth out-pointer (`0x6cf810`), returns 1.0
for 0xf, and otherwise clamps depth times `[0x801620]` (0.1). `0x7e5780` tests `[0xce8c34]`, `+0x8`
and `+0x1b` (`0x7e5787`), then `+0x4` and `[0xc7f31c]+0x28` (`0x7e579c`) before jumping to the test
(`0x7e57aa`) or the trace (`0x7e57af`). `0xc7f31c` is the `occlusion` CVar.

## Draw occlusion

The device test draws the glare's quad in the depth range [0.995, 1.0] with render states 0x13,
0x12, 0xf and 0xe at 0 and returns a count read back from the device divided by the quad's
projected area, at most 1, and 0 when the area is at most 1 or the glare's `+0x38` is set. The world
trace returns 0 when a trace from the eye along the glare's direction to the far clip hits, else
1 when a second test along the same line finds nothing.

Addresses: `0x7e5220`, `0x7e5259`, `0x7e527d`, `0x7e50a0`
Confidence: inferred
Proof: none
Test: none

`0x7e5220` sets the four states at `0x7e5259` to `0x7e527d`; the readback and area are read from
the decompiled body, not settled at the bytes. `0x7e50a0` traces with flags 0x200112 through the
world and then calls `0x480a50`; the second test's meaning is not read.

## Glare draw

`0x6d48c0` updates and draws the sun glare, then the moon glare, when the sky is on; the world
frame calls it late, after the world scene and the liquid particulates (`0x483740`). A glare draws when enabled with a nonzero alpha byte: a
camera-facing quad of half-size 0.5 times its scale at its direction, flat colour `+0x18`, its
texture, the device's blend 3 (`models/render#material-state`), lighting, fog and depth writes off,
in the depth range [0.995, 1.0].

Addresses: `0x6d48c0`, `0x6d48d4`, `0x6d48de`, `0x6d48f9`, `0x7e57e0`, `0x7e57f3`, `0x7e57fe`, `0x7e5809`, `0x7e5a20`, `0x7e5a2c`, `0x7e5a38`, `0x7e5a44`, `0x7e5a50`, `0x6cf340`, `0x6cf400`, `0x7e5670`, `0x80fec4`, `0x483740`
Confidence: verified
Proof: one
Test: none

`0x483740` calls `0x6d48c0`, which runs `0x6cf490(dt)` and `0x7e57e0` on `0xce97a8`, then on
`0xce96f8`. `0x7e57e0` builds the basis of the direction with `0x7e5670`, scales it by `+0x24`, sets
the depth range from 1 − `[0x80fec4]` (0.005) to 1, render state 7 to 3 (`0x7e5a20`) and states
0xe, 0xf, 0x12 and 0x14 to 0, and draws the four corners `0x6cf340` with the UVs `0x6cf400` and the
colour at stride 0.

## Disc draw

A disc draws as a camera-facing quad of half-size 0.5 times its size, flat colour `+0xc`, using
the quad's local z as height: with t = disc z − eye z, a disc whose top and bottom corners are
both below the eye is skipped, and one crossing it is clipped there. The quad is split again where
it crosses 0.4 units above the eye, and every vertex lower than 0.4 + 0.001 takes alpha
bits(f32(clamp(2.5·h, 0, 1)·255 + 512)) >> 14, h its height t + z; higher vertices keep the
colour's alpha. The disc uses the device's blend 2, with lighting, fog and depth writes off.

Addresses: `0x7e5b90`, `0x7e5c0b`, `0x7e5c36`, `0x7e5d82`, `0x6d1710`, `0x6d1960`, `0x6d1972`, `0x6d19a3`, `0x6d19db`, `0x6d19ed`, `0x6d1a3b`, `0x6d1a4b`, `0x6d1a56`, `0x6d1ad6`, `0x6d1ae0`, `0x6d1b27`, `0x803088`, `0x80c5c0`, `0x86f410`, `0x86f418`
Confidence: verified
Proof: one
Test: none

`0x6d1710` scales six template corners (y and z ±0.5, x 0; the last two at z 99.0 as spares) by
`[disc+0x14]` and copies `[disc+0xc]` to each vertex, strip `0x86f410`. `0x6d1960` stores t
(`0x6d1972`), h0 = t + top z and h2 = t + bottom z as floats; both above 0 keep four vertices
(`0x6d19a3`), both below 0 draw nothing (`0x6d19db`), else the bottom moves to h0/(h0 − h2) along
the edge (`0x6d19ed`) with that UV v. With t + top z − 0.4 above 0.001 (`0x6d1a3b`) and the bottom's
below (`0x6d1a4b`) it adds two vertices at the crossing, strip `0x86f418` (`0x6d1a56`). `0x6d1ad6 jp`
skips the alpha write at or above 0.001 and for NaN; `0x6d1ae0` multiplies by `[0x803088]` (2.5).
`0x7e5b90` calls both (`0x7e5c0b`, `0x7e5c36`) and sets state 7 to 2 (`0x7e5d82`).

## Stars

The stars are the model `0x881c70` at the camera eye with alpha a/255, a = trunc(track·254 + 1)
from the star track at the time of day, drawn only when a > 1 and advanced by the milliseconds
since its last draw.

Addresses: `0x6d1b50`, `0x6d1b81`, `0x6d1b86`, `0x6d1b92`, `0x7e6120`, `0x7e6129`, `0x7e61c0`, `0x40a2b0`, `0x811618`
Confidence: verified
Proof: one
Test: none

`0x6d1b50` copies the eye to `+0xc` and stores `__ftol(track·[0x811618] + 1.0)` (254.0) in byte
`+0xb` (`0x6d1b92`). `0x7e6120` returns when the byte is below 2 (`0x7e6129`), sets the model
animated and visible with that alpha, advances the scene, collects with a zero offset and draws
lists 0 and 1. `0x7e61c0` releases the model and the scene.

## Sun glare toggle

The console command `SkySunGlare` turns both glares on or off, printing a line; any argument that
does not parse as a nonzero number turns them off.

Addresses: `0x6d1d43`, `0x6d1d4d`, `0x6d1dc0`, `0x86f478`, `0x86f4b0`, `0x86f4c4`, `0xce97b0`, `0xce9700`, `0x63f9e0`
Confidence: verified
Proof: one
Test: none

`0x6d1ba0` registers `0x86f478` with the handler `0x6d1dc0` and the help `0, 1` through
`0x63f9e0`. The handler parses the argument with `%d`; a nonzero value prints `0x86f4c4` and
stores 1 in both glares' `+0x8`, else it prints `0x86f4b0` and stores 0.
