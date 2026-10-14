# terrain: water effects

The two effects drawn with the liquids: the drift cloud of motes the camera sees in liquid, and
the foam rings and wakes of units wading through shallow water. Both live in the liquid object
file; the passes that draw the liquids themselves are in `liquidpass`.

Roots: `0x68e4f0`, `0x68e930`, `0x68efe0`, `0x68bda0`, `0x68f8b0`, `0x68f9f0`, `0x68fae0`

## Drift gust

The cloud holds one gust: a unit direction, a time, a rate and an amplitude. `0x68e1c0` rolls a
new one from the client random stream: two angles a and b, each ±(2 − f)·π with f in [1, 2) the
mantissa-filled draw and the sign from the draw's top bit; the direction is (sin a·cos b, sin
a·sin b, |cos a|·0.25) normalized; the time is 0; the rate is a fresh f × 0.0125 and the
amplitude a fresh f × 0.005. `0x68e4f0` gives the frame's displacement from the gust.

Addresses: `0x68e1c0`, `0x68e4f0`, `0x4531e0`, `0x68e1d0`, `0x68e24b`, `0x68e257`, `0x68e270`, `0x68e27d`, `0x68e2e5`, `0x68e3e3`, `0x68e4db`, `0x68e503`, `0x68e53a`, `0x68e54b`, `0x68e55f`, `0x68e567`, `0x68e583`, `0x86a098`
Confidence: verified
Proof: one
Test: none

`0x4531e0` is called with ecx `0x882664` (`0x68e1d0`); its result's low 23 bits become a float in [1,
2) (`or 0x3f800000`); the second angle uses a second call, and the rate and amplitude two more
steps inlined from `0x68e2fd` and `0x68e3e9`, both on the same state. The sign test (`jge` at
`0x68e1e3`) picks f − 2 or 2 − f. The three components go to the cloud's `+0xfa28`, `+0xfa2c` and
`+0xfa30`; `0x68e270` stores |cos a|·0.25 by negating a negative value (`0x68e27d`); `+0xfa38` is the
time (`0x68e2e5`); `+0xfa34` is the rate (`0x68e3e3`, ×0x807a4c) and `+0xfa3c` the amplitude
(`0x68e4db`, ×0x807a3c). `0x68e4f0` reads the mode `+0xfa24`. Modes 0 and 1: the time gains dt
(`+0xfa38`), and x = time × rate; when x is above 0.5 a new gust is rolled and x is 0
(`0x68e53a` is `fcom 0.5; test ah,0x41; jne`: a smaller, equal or NaN x keeps the gust); the result is
sin(x·2π) × amplitude times the direction. Mode 2: (0, 0, −0.02·dt) (`0x86a098`). Any other mode:
(0, 0, 0).

## Drift advect

`0x68e930` moves the motes with the frame. With the cloud's enable byte `+0xfa14` set, it takes
the camera's step since the last frame (`+0xfa04`: the previous position less the current), and
when its squared length exceeds the squared box edge `+0xfa1c` it scatters the cloud again in the
current mode and clears the step. It adds the gust displacement to the step, adds that to every
mote (`+0xfa00` of them, 16 bytes each) and wraps each axis into [−edge/2, edge/2]: above the half
edge it subtracts the edge, below minus the half edge it adds it.

Addresses: `0x68e930`, `0x68e939`, `0x68e95b`, `0x68e985`, `0x68e9c2`, `0x68e9cf`, `0x68e9da`, `0x68ea13`, `0x68ea45`, `0x68ea6a`, `0x68ea72`, `0x68ea84`, `0x68ea8c`, `0x68ea8e`, `0x68e720`, `0x68e650`, `0x68e670`, `0x68e680`
Confidence: verified
Proof: one
Test: none

`0x68e9c2` to `0x68e9cf` compare edge² with the step's squared length (`fcompp`, `test ah,5; jp`):
the scatter runs when the step is longer than the edge. `0x68e99a` loads the edge, not half.
The wrap compares the stored f32 sum with the half edge `[ebp-0x4]`: `test ah,0x41; jne` leaves a value
at or below it (and NaN) alone (`0x68ea72`); the lower test (`0x68ea84`) adds the edge only for a
value strictly below. `0x68e650` stores the count, its argument × 4000.0, `0x68e670` the size base
at `+0xfa18` and `0x68e680` the edge at `+0xfa1c`.

## Drift draw

`0x68efe0` draws the motes as camera-facing quads in view space. A mote counts when its view-space
position has z above 0 and |x|, |y| below z; it fills a vertex buffer of 2,664 vertices (666
quads) and stops when full. A quad is the mote's position plus the corner offsets (−0.5, 0.5),
(−0.5, −0.5), (0.5, 0.5), (0.5, −0.5) times its size, in view space with identity matrices,
colour 0xffffffff, and the texture cell of its index. The state is the cloud's texture on stage 0,
blend 2, fog on for mode 2 only, depth write and lighting off.

Addresses: `0x68efe0`, `0x68f002`, `0x68f0b3`, `0x68f15b`, `0x68f1c9`, `0x68f1d4`, `0x68f1e5`, `0x68f1fe`, `0x68f20f`, `0x68f220`, `0x68f240`, `0x68f249`, `0x68f27b`, `0x68f2a2`, `0x68f2cd`, `0x68f2e8`, `0x68f2ee`, `0x68f338`, `0x68f349`, `0x68f36e`, `0x68f37a`, `0x68f386`, `0x68f392`, `0x68f3c9`, `0x86a0a0`, `0xc800f4`, `0xc81b98`, `0x68ebf0`
Confidence: verified
Proof: one
Test: `terrain::fx_uv_table`

The vertex format is 8 (position, colour, one texture coordinate; stride 0x18) and the buffers are
requested with 0xa68 vertices and 0xf9c indices (`0x68f002`). The tests are
`0x68f1c9` (z > 0, `jne` skips for z at or below 0 or NaN), `0x68f1d4` (x < z), `0x68f1fe`
(x > −z), `0x68f20f` and `0x68f220` likewise for y. The corners are the 12-byte records at
`0xc800f4` times the size at record `+0xc` (`0x68f249`), added to the view position. The mote's
cell: the first mote uses cell 8; the mote after mote i uses `0x86a0a0[(i & 7) + 8·mode]`
(`0x68f2e8`, `0x68f2ee`) whether or not mote i was kept: modes 0 and 1 cells 0 to 7, mode 2
cells 9, 10, 11, 12, 9, 10, 11, 12, mode 3 cell 0. A cell is two 16-byte rows at `0xc81b98` (four
(u, v) pairs), in multiples of 51/256 built by `0x68ebf0`. The index list of a quad is (0, 1, 2),
(3, 2, 1) added to its first vertex (`0x68f2a2`); the draw has primitive type 3.
`0x68f338` binds the texture `+0xfa10`, `0x68f349` sets blend 2, `0x68f36e` fog to (mode == 2).

## Ripple pool

`0x68f8b0` loads `XTextures\splash\splash.blp` (category 0, the ring) and
`XTextures\splash\wake.blp` (category 1, the wake) and makes a pool of 128 records of 0x50
bytes. Records 0 to 31 serve the active mover and 32 to 127 every other unit; `0x68f9f0` takes the
next of the right range, wrapping to its first when it passes the last, so a busy pool overwrites
the oldest record. Emission is allowed when the waterRipples setting is nonzero.

Addresses: `0x68f8b0`, `0x68f8c7`, `0x68f8cd`, `0x68f903`, `0x68f914`, `0x68f92c`, `0x68f9f0`, `0x68f9fc`, `0x68fa05`, `0x68fa10`, `0x68fa1c`, `0x68fa25`, `0x68fa33`, `0x68fa40`, `0x68bda0`, `0x68bda3`, `0x86a120`, `0x86a124`, `0x86a03c`
Confidence: verified
Proof: one
Test: none

`0x68f8c7` and `0x68f8cd` start the counters at 0 and 0x20; `0x68f92c` is `0x80`, the pool size.
`0x68f9fc` takes the counter for the active mover (the last argument nonzero), wraps at 0x20
(`cmp esi,0x20`, back to 0 at `0x68fa10`); `0x68fa1c` takes the other counter, wraps at 0x80 (back to
0x20 at `0x68fa33`). The record is `0x50 × index` from the pool (`0x68fa40`). `0x68bda3` tests the
waterRipples setting and returns for 0.

## Ripple emission

`0x68bda0` gates and relays an emission. In the immediate mode (`0xc89db8` is 0) it makes the peak
alpha t = 6x when x < 1/6, else 1.0, and passes t, the heading, the size and the lifetime and
growth to `0x68f9f0`. `0x68f9f0` negates the heading and `0x68f5c0` writes the record: position,
heading, size, growth (final size less size, over lifetime), peak t, alpha 0, a rise rate t /
(lifetime × 0.4), a decay rate −t / (lifetime × 0.6), the death time (the clock plus the lifetime)
and the category. It copies in the transformed vertices of every liquid triangle that the box of
half-size the final size and half-height 1.0 around the position overlaps.

Addresses: `0x68bda0`, `0x68bdb0`, `0x68be62`, `0x68be6c`, `0x68be77`, `0x68be7c`, `0x68be9d`, `0x68f9f0`, `0x68fa48`, `0x68fa6a`, `0x68f5c0`, `0x68f5cc`, `0x68f5f7`, `0x68f60d`, `0x68f621`, `0x68f633`, `0x68f64f`, `0x68f65b`, `0x68f6a7`, `0x68f6d3`, `0x68f6fc`, `0x68f7f6`, `0x6723b0`, `0x810348`, `0x803568`, `0x802990`
Confidence: verified
Proof: one
Test: `terrain::ripple_params_difftest`

`0x68bdb0` is `cmp [0xc89db8]`: 0 goes to `0x68be62`, 1 queues through `0x68a1f0`, any other value
emits nothing. `0x68be6c` compares x with 1/6 (`0x803568`; `test ah,5; jp` skips the multiply by
6.0, `0x802990`, for x at or above 1/6 or NaN, leaving 1.0). The record fields: `+0x0` position,
`+0xc` the heading (negated at `0x68fa48`), `+0x10` the size, `+0x14` the growth (`0x68f621`), `+0x18`
alpha 0, `+0x1c` t, `+0x20` t / (lifetime × `0x810348[category]`) (`0x68f633`), `+0x24` the
negated t / (lifetime × (1 − rate)) (`0x68f64f`), `+0x28` the clock `0xc7b2a0` plus the lifetime
(`0x68f65b`), `+0x2c` the category (nonzero → 1, `0x68f5f7`). Both rates are 0.4, so the rise
takes the first 40% of the life and the decay the rest. The box is the position ± (size, size, 1.0)
(`0x68f6a7`) with size = lifetime × growth + size0; the liquid triangles come from the box query
`0x6723b0` with mask 0xf0000 (`0x68f6fc`), and each batch's triangles are copied
to the record's polygon (`+0x44` points, `+0x48` the array) through `0x7bca80` (`0x68f7f6`).

## Ripple update

`0x68fae0` ages and draws the ripples each frame. A record whose death time has come, or whose
alpha has reached 0 or less, goes back to the pool. Otherwise its size grows by dt × growth and
its alpha rises by dt × rise to the peak; once the rise ends, alpha moves by step × decay, where step is
dt, or on the frame the peak is passed dt − (sum − peak)/rise. Live records are drawn per category
as triangle lists over their polygons.

Addresses: `0x68fae0`, `0x68fb45`, `0x68fb50`, `0x68fb8e`, `0x68fb9a`, `0x68fba6`, `0x68fbbb`, `0x68fbc3`, `0x68fbdd`, `0x68fbfd`, `0x68fc08`
Confidence: verified
Proof: one
Test: none

`0x68fb50` compares the clock with the death time (`test ah,1`: still alive when it is less); a
dead record is unlinked. `0x68fb9a` adds dt × growth to the size. `0x68fba6` tests the rise rate for
zero: with a rise rate the alpha gains dt × rise (`0x68fbb8`) and a result above the peak
(`0x68fbc3`) is clamped, the rise rate cleared and (excess / rise) taken from the step before the
decay is applied (`0x68fbed`): the decay runs for the time the rise took, not for the time past the
peak, which is the client's own quirk. A record whose alpha is 0 or less (`0x68fc08`) is
unlinked.

## Ripple draw

The ripples draw additively (blend 3) with fog, depth write, culling and lighting off and a depth
bias. The depth test and function are not written, so the foam is tested against the scene (test
on, LEQUAL) with depth write off. It runs once a frame as the tail jump of `0x6816d0`, after the
ocean, river and group liquid passes.

Addresses: `0x68fae0`, `0x68fcbc`, `0x68fcc6`, `0x68fcd7`, `0x68fce3`, `0x68fcef`, `0x68fcfb`, `0x68fd03`, `0x68fd0f`, `0x68ff4c`, `0x690150`, `0x690177`, `0x6901f0`, `0x69026c`, `0x69029d`, `0x6d6fa0`, `0x7bdd60`, `0x690095`, `0x810390`, `0x6816d0`, `0x6816df`
Confidence: verified
Proof: one
Test: none

The depth bias is state 0 set to the `footstepBias` variable (record `0xc7f300`, value at `+0x24`,
default 0.125) times 1/1024 (`0x810390`, `0x68fd03`). The texture is the category's (`0x68ff4c`), the
colour is alpha·255 in the top byte over white (`0x690150`), and each vertex gets a normal (0, 0, 1)
and the texture coordinate from the decal projection `0x6d6fa0` (`0x6901f0`) of the box position ± (size,
size, 1.0) turned by the record's heading about z (`0x7bdd60` at `0x690095`); `0x69029d` draws primitive 3.
