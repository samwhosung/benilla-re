# terrain: detail doodads

The ground clutter of the map: the tufts, flowers and rocks scattered on a chunk by the
ground-effect records of its texture layers. A chunk scatters them once when its models are
loaded and it lies within the detail distance, and the scene draws them with a distance fade
that comes from a ramp texture, not from the distance. The code is `DetailDoodad.cpp` and the
chunk's creation function.

Roots: `0x6bfc10`, `0x6b1a90`, `0x6b1d20`, `0x6b1d30`, `0x6b2b80`

## Detail doodad tables

`0x6b1a90` makes the model table after the DBCs load: one detail model for each
GroundEffectDoodad row, stored at the row's id, and the 64 × 8 ramp texture of the fade. It sets
the rebuild flag. `0x6b1d30` then, while the flag is set, sizes the buffers: the capacity is
the `frillDensity` setting times 64 placements, at most 0x2000. `0x6725a0` sets that setting
and `0x6b1d20` the flag.

Addresses: `0x6b1a90`, `0x6b1b2d`, `0x6b1b42`, `0x6b1b6e`, `0x6b1b9a`, `0x6b1bca`, `0x6b1d20`, `0x6b1d30`, `0x6b1d4b`, `0x6b1d53`, `0x6b1d67`, `0x6b1daa`, `0x6b1dca`, `0x6b1f4d`, `0x6b1f62`, `0xc7b494`, `0xcb0034`, `0x58a980`, `0x58aa30`
Confidence: verified
Proof: one
Test: none

`0x6b1a90` allocates a 0xc-byte model per row (`0x6b1b2d` to `0x6b1b42`, source path `DetailDoodad.cpp`
line 0x49). The ramp comes from `0x58aa30` with width 64, height 8 and the texture flags `0x58a980`
builds from (1, 0, 0, 0, 0, 0, 1, 0), which are 0x201: linear, clamped, no mip levels. In
`0x6b1d30`, `0xc7b494 << 6` is capped at 0x2000 (`0x6b1d53`, `0x6b1d67`); the pools are made at
`0x6b1daa` and `0x6b1dca`, 64 views of a vertex buffer and an index buffer follow in the loop to
0x80 (`0x6b1f4d`), and `0x6b1f62` clears the flag. With the density at or above 128 the capacity
saturates.

## Detail doodad scatter

`0x6bfc10` scatters the detail doodads of a chunk. It needs the chunk to have layers and every
model they name to be loaded (`0x6bfb70`); it makes the chunk's batch and seeds one random
stream with `(chunk +0x90 << 16) | chunk +0x8c`. Pass 1 draws the `frillDensity` cells, each two
draws (column, then row, each the stream's value & 7), repeats allowed, and builds the cell's four
ground planes once. Pass 2 takes the picks in order, skipping a pick whose cell has its no-effect
bit set, lies in a hole quad, or whose layer has no effect row, and places the row's Density
tufts (8 when it is 0).

Addresses: `0x6bfc10`, `0x6bfb70`, `0x6b1fb0`, `0x44ea80`, `0x4531e0`, `0x6bfc59`, `0x6bfc6c`, `0x6bfcfb`, `0x6bfe9a`, `0x6bfea1`, `0x6bfef0`, `0x6bff1c`, `0x6bff4d`, `0x6bff8f`, `0x6bffa6`, `0x6bffb7`, `0x6bffc4`, `0x6bfff8`, `0x6c0002`, `0x637480`, `0xce78f8`, `0xce78fc`, `0x8104e4`, `0x8104b4`, `0x8103f8`
Confidence: verified
Proof: one
Test: none

`0x6bfc59` seeds with the combined index; `0xc63194` nonzero ends the function after the batch is
made (`0x6bfc6c`). Pass 1's two steps are `0x4531e0` inlined; the pick goes to `0xce78f8` and
`0xce78fc` (`0x6bfe9a`, `0x6bfea1`), and a visited byte per row keeps the planes to once per cell:
plane k (`0x6bfef0`) is `0x637480` over the cell's centre vertex (`+9`) and corner pair k of the
tables `0x81047c` and `0x810480`, stored at `0xce68d0 + (col + row × 8) × 0x40 + 0x10k`. Pass 2:
the no-effect byte `+0xf10[row]` masked by `0x8104e4[col]` (`0x6bff4d`); the hole word
`[+0xf08] + 0x3c` against the bit table `0x8103f8` at `(col >> 1) + (row >> 1) × 4` (`0x6bff8f`);
the layer from `+0xf0c[row]` masked by `0x8104b4[col]` (`0x6bffa6`, `0x6bffb7`) selects
`+0x128 + 4 × layer`, whose effect id (`+0xa`) indexes the GroundEffectTexture table (`0x6bffc4`
to `0x6bfff8`); Density is `+0x14` (`0x6c0002`).

## Detail doodad tuft

Tuft j of pick i draws two values v₁, v₂ (`2 − f` or `f − 2` for f in [1, 2), by the draw's
sign) and takes the doodad `row[+4 + 4 × ((j + i) & 3)]`; −1 places nothing. In the cell x = v₁ ×
2.0833333 + 2.0833333, y likewise. The ground plane is the cell's quadrant of (x, y); the
chunk-relative position is (−(y + 4.1666665 row), −(x + 4.1666665 col), z from the plane). Scale
is v₃ × 0.1 + 1, yaw v₄ × π, drawn only for a placed tuft.

Addresses: `0x6bfc10`, `0x6c0043`, `0x6c0056`, `0x6c005b`, `0x6c00ab`, `0x6c00be`, `0x6c00c1`, `0x6c00f9`, `0x6c0114`, `0x6c013b`, `0x6c0158`, `0x6c0176`, `0x6c01ca`, `0x6c01fd`, `0x6c0206`, `0x6c023d`, `0x6c025b`, `0x6c026f`, `0x810e84`, `0x810e90`, `0x810e80`, `0x810678`
Confidence: verified
Proof: one
Test: none

`0x6c0043` draws; `0x6c005b` is `jge` on the draw's sign (non-negative gives f − 2). `0x6c00ab` and
`0x6c00c1` scale by `0x810e84` and add `0x810e90`. The texel is the sum less 0.5 (`0x6c00f3`) and
`fistp` (`0x6c00f9`, `0x6c0114`); `0x6c013b` tests the bit of the chunk's 64 × 64 shadow map `+0xf1c` at texel ((x + 4.1666665 col) × 1.92 − 0.5, (y + 4.1666665 row) × 1.92 − 0.5), the shadow flag. `0x6c0158` (`test ah,5; jp`)
sets the quadrant's first bit when y − x is negative; `0x6c0176` (`test ah,0x41; jne`) the second
when x + y exceeds the cell; the z is −(n·p + d)/n.z, negated at `0x6c01ca`. The scale and yaw draws are
`0x6c01fd` and `0x6c023d`; `0x6c025b` is `fsincos`; `0x6c026f` calls the batch add with the plane and the flag.

## Detail doodad batch

`0x6b2920` adds a placement to the chunk's batch. The batch has four slots of 0x24 bytes. It
takes the first slot that already holds the doodad's model or has room by the model's counts
and appends a 0x2c-byte placement: the shadow flag at `+0x0`, the doodad id `+0x4`, the position `+0x8`, sine,
cosine and scale `+0x14`, and the plane normal `+0x20`; the slot's vertex and index counts
grow by the model's.

Addresses: `0x6b2920`, `0x6b2931`, `0x6b2953`, `0x6b295b`, `0x6b296e`, `0x6b2987`, `0x6b2a14`, `0x6b2a24`, `0x6b2a55`, `0x6b2a5d`, `0x6b2a73`, `0x6b2a8a`, `0x6b2a91`, `0x6b2aaa`, `0x41af70`, `0x6b25c0`
Confidence: inferred
Proof: none
Test: none

`0x6b295b` compares the slot's model with the doodad's and `0x6b296e` asks `0x6b25c0` whether the
slot has room; the first free slot is taken when none matches (`0x6b2987`). The placement is
written at `0x6b2a55` onward and the slot's counts at `0x6b2aaa`. How a full batch behaves is
not read here.

## Detail doodad draw

`0x6b2b80` draws a chunk's batch. It sets stage 0 combine 1 (modulate), culling off, device colour
white, blend 2, the alpha reference to the byte `0xc7b2fc` (128 at start-up), depth write on and
the ramp on stage 1 with combine 1, camera-space texture coordinates (state 0x30 = 3) and its
texture matrix on (state 0x38 = 1). The stage 1 matrix makes u = (z − 52.5)/17.5 and v = y/17.5
of the eye-space position: scale 1/(70 × 0.25), rotate π/2 about y, translate (0, 0, 70 ×
−0.75). Each slot with data is uploaded and drawn with its model's texture on stage 0. Culling
and depth write are set to 1 at the end.

Addresses: `0x6b2b80`, `0x6b2b8f`, `0x6b2b9b`, `0x6b2bb3`, `0x6b2bc2`, `0x6b2bc7`, `0x6b2be2`, `0x6b2bf2`, `0x6b2c01`, `0x6b2c10`, `0x6b2c1f`, `0x6b2c24`, `0x6b2cad`, `0x6b2cc8`, `0x6b2ce0`, `0x6b2d09`, `0x6b2d16`, `0x6b2d25`, `0x6b2d37`, `0x6b2d46`, `0x6b2d55`, `0x6b2320`, `0x6b2d60`, `0x6b2e21`, `0x6c02d0`, `0x867958`, `0x810c10`, `0x810c68`, `0xc7b2fc`, `0xcaf79c`
Confidence: verified
Proof: one
Test: `terrain::detail_doodad_texgen_difftest`

The states are: `0x6b2b94` 0x1f = 1, `0x6b2ba0` 0x14 = 0, `0x6b2bb3` colour 1 = 0xffffffff, `0x6b2bc2`
blend 2, `0x6b2bd3` the alpha reference from `0xc7b2fc`, `0x6b2be2` 0x12 = 1, `0x6b2bf2` stage 1
(0x18) = the ramp `0xcaf79c`, `0x6b2c01` 0x20 = 1, `0x6b2c10` 0x30 = 3, `0x6b2c1f` 0x38 = 1. The
matrix: scale by `1 / (70.0 × 0.25)` (`0x6b2c24` to `0x6b2cad`), rotate by π × 0.5 about (0, 1, 0)
(`0x6b2ce0`), translate by (0, 0, 70.0 × −0.75) (`0x6b2d09`), set for stage 1 (`0x6b2d16`). The
ramp (`0x6b2320`) is 64 columns of eight rows, each pixel white with alpha 4 × (63 − column), so
the alpha plateau is 252/255 up to z = 52.64, falls 0.0574 a yard, and is 0 from 69.86. `0x6b2e21`
draws a slot as primitive 3 with its index count. `0x6c02d0` calls this draw when its argument is
nonzero and the chunk has a batch; the other leg is never taken.

## Detail doodad fade law

A detail doodad fades by its depth along the camera axis, z = dot(forward, p − eye), not by distance. The ramp
alpha is 252/255 for z up to 52.637, falls 0.057367 a yard to 0 at 69.863, and is 0 beyond. The alpha test
keeps a fragment when texture alpha × ramp ≥ 128/255, so nothing survives past z = 61.113 and a texel of alpha
k survives to z = 52.5 + (254 − 128/k)/14.628571, never when k < 0.50794. From 52.637 a tuft erodes from its
lowest-alpha texels while the survivors blend and still write depth. The colour is untouched: the ramp is white.

Addresses: `0x6b2b80`, `0x6b2bd3`, `0x6b2bc2`, `0x6b2be2`, `0x6b2ba0`, `0x6b2bf2`, `0x6b2c01`, `0x6b2c10`, `0x6b2c1f`, `0x6b2d16`, `0x6b23c0`, `0x6b2320`, `0x6b2600`, `0x5a24a0`, `0x5a24d9`, `0x6c02d0`, `0x6b2ae0`, `0x68165f`, `0x6813d0`, `0x6813ea`, `0x6813f4`, `0xcaf79c`, `0xc7b2fc`
Confidence: verified
Proof: one
Test: `terrain::detail_doodad_texgen_difftest`

The ramp's 8 rows are alike, so v does not matter; with clamp and linear sampling, u = (z − 52.5)/17.5 gives
the tap t = 64u − 0.5 and ramp = 4 × (63 − clamp(t, 0, 63))/255. Both stage combines are modulate (preset 1 colour
and alpha), so alpha is texture alpha × ramp × vertex alpha, and the two vertex colours `0x6b2600` writes,
`0xffffffff` and `0xffc0c0c0`, are opaque. The device compares with greater-or-equal, set once with the other
base states (`0x5a24d9`, `0x5a24a0` from device creation and each reset), never per draw. `0x6c02d0` is called
once, with 1 (`0x68165f`), so only the batch draw with blending runs and `0x6b2ae0` (the same states, blend off,
no ramp) is never reached. The pass sets the stage 0 texture LOD bias to 0.25 first (`0x6813ea`, state 0x27). The
M2 material's own blend mode never reaches the device: blend 2, alpha test, depth write and no culling
are set for every doodad. Texture coordinate mode 3 is the camera-space vertex position, and the state 0x38
that nothing applies is read back by the texture matrix applier, so it must be set for the ramp to move.

## Detail doodad range

A chunk makes its detail doodads once, when its nearest view depth first falls below 70.0, and the
batch is not rebuilt until it is freed. The depth is the chunk step's `+0x78`: the chunk sphere's
centre depth along the camera axis less its radius. At exactly 70.0, or for NaN, the chunk is skipped and
leaves the terrain draw list after its draw, so the doodad pass `0x6813d0`, which walks that list, never
sees it; a nearer chunk stays on the list. A tuft is never visible when its chunk is dropped: the ramp is
zero 8.9 yards before the 70 yard test, which a doodad's height above the sphere does not close.

Addresses: `0x684510`, `0x6847c9`, `0x6847d3`, `0x6847d8`, `0x6847e0`, `0x6847e3`, `0x6847e5`, `0x6847ed`, `0x6847fc`, `0x6bfc10`, `0x6afb03`, `0x867958`, `0xc7b2a4`, `0x673050`
Confidence: verified
Proof: one
Test: none

`0x6847c9` tests world flag 0x100000, set by default (`0x66f6cb`) and flipped by the `showDetailDoodads`
command `0x673050`, which prints "Detail doodads enabled." or "Detail doodads disabled.". `0x6847d8` compares
the depth with 70.0 (`0x867958`, never written): `test ah,5` with `jp` skips an equal, larger or NaN
value. `0x6847e5` builds only when `+0xc0` is empty. The far leg `0x6847fc` unlinks the chunk's node `+0xd4`
from that list and leaves `+0xc0`. `0x6afb03` stores the depth.

## Detail doodad alpha command

`detailDoodadAlpha` is a console command, not a setting: it takes a whole number from 0 to 255 for the alpha
reference of the detail doodad draw and has no saved value. The reference starts at 128. Outside the range it
prints "Alpha ref range 0 - 255.".

Addresses: `0x6739a0`, `0x6739d5`, `0x66f861`, `0xc7b2fc`, `0x6b2b21`, `0x6b2bc7`
Confidence: verified
Proof: one
Test: none

`0x66f861` stores 0x80 into `0xc7b2fc` at start-up; `0x6739d5` stores the parsed value. Both readers
(`0x6b2b21`, `0x6b2bc7`) load the byte unscaled, and a value above 0 turns the device's alpha test on at that
reference, 0 off.
