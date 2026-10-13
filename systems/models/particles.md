# models: particles

The particle emitters a model carries: the emitter object and its three spawn shapes, the update
that advances it in substeps, the spawn count, the motion of one particle, the gradients that
colour and size it over its life, and the quads it draws. A legacy emitter class and the registries
that hold emitters outside a model are also here.

The emitter, `CParticleEmitter2`, is 0x290 bytes and its shapes 0x2a0 to 0x2d0; its code runs from `0x7b0dd0` through
`0x7b6990`, its plane, sphere and spline shapes from `0x7b8750` through `0x7b9ad0`, the gradient
records from `0x7b9af0` through `0x7b9f30`, and the legacy `CParticleEmitter` from `0x7b9f50`
through `0x7ba9f0`. The registries are the object file at `0x7adf90` through `0x7af530`. A model
creates one emitter per particle record of its header (`0x70ebd0`) and drives it each frame
(`0x718960`); the record and the spawn shapes are `emission`.

Roots: `0x7b1470`, `0x7b1dd0`, `0x7b22b0`, `0x7b2420`, `0x7b29c0`, `0x7b44e0`, `0x7b44f0`, `0x7b4500`, `0x7b4b90`, `0x7b4bf0`, `0x7b5dd0`, `0x7b6050`, `0x7b87b0`, `0x7b87f0`, `0x7b8890`, `0x7b8c10`, `0x7b8c20`, `0x7b8c30`, `0x7b8c40`, `0x7b8c90`, `0x7b8cd0`, `0x7b8d70`, `0x7b9180`, `0x7b91a0`, `0x7b91c0`, `0x7b91d0`, `0x7b9260`, `0x7b92a0`, `0x7b9500`, `0x7b9970`, `0x7b99c0`, `0x7b9a40`, `0x7b9a50`, `0x7b9a60`, `0x7adfb0`, `0x7ae000`

## Emitter classes

A model emitter is built by its shape's constructor: the base constructor, zeroed shape fields and
the shape's vtable. The three vtables share the base's first slots and differ in the spawn slot,
the clone and deleting destructor, four shape setters and the rate setter.

| slot | base `0x81d83c` | plane `0x81d888` | sphere `0x81d8b4` | spline `0x81d8e0` |
|---|---|---|---|---|
| 0 | `0x7b1dd0` | same | same | same |
| 1 | `0x7b2420` | same | same | same |
| 2 | `0x7b22b0` | `0x7b8890` | `0x7b8d70` | `0x7b9500` |
| 4 | pure | `0x7b87b0` | `0x7b8c90` | `0x7b9260` |
| 5 | `0x7b1470` | `0x7b87f0` | `0x7b8cd0` | `0x7b92a0` |
| 6 to 9 | pure | `0x7b8c10` to `0x7b8c40` | `0x7b9180` to `0x7b91d0` | `0x7b9970` to `0x7b9a50` |
| 10 | `0x7b4bf0` | same | same | `0x7b9a60` |

Addresses: `0x7b8770`, `0x7b8c50`, `0x7b91e0`, `0x7b11b0`, `0x81d83c`, `0x81d888`, `0x81d8b4`, `0x81d8e0`, `0x7b4bf0`, `0x7b9a60`, `0x70ebd0`, `0x7b1470`, `0x40a2d7`, `0x70f9b9`
Confidence: verified
Proof: one
Test: none

`0x7b8770` calls `0x7b11b0`, zeroes `+0x290` to `+0x29c`, stores `0x81d888` and sets `+0x24` to 1.
The table is read from the image at `0x81d83c` to `0x81d908`; slot 3 is `0x7b4b90` in every table.
The base's pure slots hold `0x40a2d7`; its slot 5 is the deleting destructor `0x7b1470`. The model's
initializer picks the shape with the `dec` chain at `0x70f9b9` on the record's 16-bit type `+0x2a`
(`emission#record-to-emitter`). `0x7b4bf0` stores the rate at `+0xa8`, or 0 when it is not positive;
the spline's `0x7b9a60` stores its argument at `+0x290` and `+0xa8 = arg · +0x298`.

## Update

`Update(dt, matrix, camera, frame)` stores the distance from the emitter's position to the camera
in a global for the spawn rate, keeps the last position, sets the frame of the emitter and its
children, and with `|dt| ≤ 2.384186e-7` only sets its flag 0x80. With flag 0x40000 it derives its
displacement this frame scaled by `clamp(|d|/dt · a + b, 0, 1)`. With flag 0x400 it accumulates
time and, past 1/30 s, derives a velocity from its motion. It then advances, and places model
particles.

Addresses: `0x7b5230`, `0xcf58e8`, `0x7b5160`, `0x7b5880`, `0x7b4840`, `0x81d82c`
Confidence: verified
Proof: one
Test: none

The distance is `sqrt` of `Vector3LengthSquared(row3 − camera)`, stored as a float at `0xcf58e8`.
The scale terms are `+0x1f8` and `+0x1f4`, the displacement `+0x26c`. The 1/30 threshold is the
float at `0x81d82c`; the velocity at `+0x258` is `(pos − last)·(1/(acc/(1/30)))·+0x1c4`, or zero
when `+0x64` is 0. `0x7b5880(dt, 0)` advances; flag 0x40 is set; `0x7b4840` runs when `+0x28` is 1.

## Substeps

An advance clamps a negative `dt` to 0 and rebuilds the gradients when the lifespan changed. A step
longer than 0.1 s runs `n` substeps of 0.1 s and one of the remainder `dt − 0.1·floor(dt/0.1)`,
with `n = floor(dt/0.1)` capped at `floor(lifespan/0.1)` and at 255; the remainder is taken before
the cap, so the substeps the cap drops are lost time. Each substep moves particles by `1/(n + 1)`
of the frame's displacement. Otherwise one step takes the whole displacement.

Addresses: `0x7b5880`, `0x7b590f`, `0x7b5935`, `0x7b5a10`, `0x7b9cf0`, `0x81d828`, `0x73ff3f`
Confidence: verified
Proof: one
Test: none

0.1 is the float at `0x81d828`; `0x73ff3f` floors the doubles. `0x7b590f` computes `dt − 0.1·n`
from the uncapped `n`, and the cap follows at `0x7b5935`. The count goes through `+ 512.0`
and `>> 14` of the float's bits to an integer byte. The displacement for a substep is
`+0x26c · (1/(n + 1))` into `+0x278`; `0x7b5a10(0.1, arg)` runs `n` times, then once with the rest.

## Spawn count

The rate is the density setting times the emission rate times `clamp(1 − (dist − 50)·0.02, 0.25,
1)`, a NaN factor counting as 1. A pending burst spawns `ftol(rate)` particles at once. An enabled
emitter adds `rate·dt` to its accumulator, which it stores as a float, spawns `ftol(rate·dt + acc +
0.5)` particles from the unrounded sum while free slots last, and subtracts the number spawned. With flag 0x1000 each spawn position is moved to a random point between the last and the
current position.

Addresses: `0x7b5550`, `0x7b566d`, `0x7b57d0`, `0x7ae000`, `0xcf58e8`, `0x4531e0`
Confidence: verified
Proof: one
Test: none

The distance factor uses 50.0 and 0.02, with `fcomp 0.25` and `fcomp 1.0` clamps. The burst flag is
0x20 and is cleared after; the enabled test needs flags 1 and 2. `fst` at `0x7b566d` stores the
accumulator and keeps the sum on the stack for `fadd 0.5` and `__ftol` (`0x7b57d0`). A spawn pops a free index from
`+0x70`, pushes it on the live list `+0x5c`, sets the record's `+0xd` to 1 and calls slot 2 (or
slot 1 for model particles). The random fraction is `asfloat((u & 0x7fffff) | 0x3f800000) − 1`
from `0x4531e0` on the state at `+0x2c`.

## Particle motion

One step of a particle, in order: while its age is under the emitter's wind time, wind times `dt`
is added to its velocity; with flag 0x40000 its position follows the emitter's displacement, except
on its first step; it moves by `dt·v` with gravity `g` giving `z += dt·vz − 0.5·g·dt²` and `vz −=
dt·g`; drag removes `min(dt·drag, 1)` of the velocity. A velocity component whose magnitude falls
under 1e-8 becomes 0. With flag 0x800 the step kills a particle moving away from the emitter's
origin.

Addresses: `0x7b2680`, `0x81d86c`
Confidence: verified
Proof: one
Test: none

The wind is `+0x1e4` to `+0x1ec`, its time `+0x1f0`, gravity `+0x180`, drag `+0x1e0`. The z update
is `fld dt; fmul g; fmul dt; fmul 0.5; fsubr z; fadd dt·vz`. The flush compares `fabs` with 1e-8
(`0x81d86c`) for a nonzero component. Flag 0x800 returns 0 when `dx·x + dy·y + dz·z > 0` with the
step and the new position; otherwise the step returns 1.

## Particle life

A step first lets an enabled emitter or one with a pending burst reserve storage, then spawns. Each
live particle then ages by `dt` and dies once its age reaches the lifespan, a NaN age included.
A survivor takes gradient 1 once its age is past `mid·lifespan`, else gradient 0, and moves
(`Particle motion`); a move that returns 0 kills it. A dead particle's slot returns to the free
list and the last live index takes its place in the live list.

Addresses: `0x7b5a10`, `0x7b5a2f`, `0x7b5a90`, `0x7b5aaa`, `0x7b5bd9`
Confidence: verified
Proof: one
Test: none

`0x7b5a2f` calls slot 0 when flags 1 and 2 are set or flag 0x20 is, then `0x7b5550(dt, +0x1fc)`
unless the second argument is set. The age is `fadd [+0x1c]; fst`, compared with `+0xac` under
`test ah,5; jp`. The gradient test compares with `+0xf0`, the second time bound of gradient 0, and
stores byte `+0xc` = 1 when neither C0 nor C3 is set. Death calls slot 3, pushes the index on
`+0x70` and moves the last of `+0x5c` into the hole.

## Child emitters

An emitter whose record names a child emitter model carries its emitters at `+0x80` (count
`+0x7c`). After each surviving particle moves, every child is moved to the particle: its position
becomes the particle's, its previous position the particle's before the move when the child has
flag 0x1000, its velocity the particle's under flag 0x400; it spawns for this step, and its
position is restored. After all
particles, each child advances without spawning.

Addresses: `0x7b5afb`, `0x7b5b4b`, `0x7b5b72`, `0x7b5b9f`, `0x7b5c2d`, `0x7b5da0`
Confidence: verified
Proof: one
Test: none

The loop at `0x7b5b21` saves child `+0x22c`, copies the particle's `+0x0` into it, under bit 10 its
`+0x10` into `+0x258`, under bit 12 the position saved before `0x7b2680` into `+0x248`, calls `0x7b5550(dt, child
+0x1fc)` and restores `+0x22c`. `0x7b5c2d` calls `0x7b5880(dt, 1)` per child; `Update` gives each
child the parent's frame (`0x7b52bb`).

## Gradients

Two gradients colour a particle, over ages 0 to `mid·L` and `mid·L` to `L` (`L` the lifespan, at
least 0.001; `mid` record `+0x14c`). Within a span `t0` to `t1`,
`f = (age − t0)/(t1 − t0)·0.99 + 0.005`, unclamped. A channel is `c0 + (c1 − c0)·f`, alpha times
the instance alpha; the size `s0 + (s1 − s0)·f`; a cell `a + n·f`, or `a + n·g` with
`g = fmod(f·repeat, 1)` when the repeat is not 1, and a range `p` to `q` gives `a = p`,
`n = q − p + 1` for `q ≥ p`, else `a = p + 1`, `n = q − p − 1`; bytes take `(x + 512) >> 14`.

| gradient | colours | sizes | head cells | tail cells | repeat |
|---|---|---|---|---|---|
| 0 | `+0x150` to `+0x154` | `+0x15c` to `+0x160` | `+0x168`, `+0x16a` | `+0x174`, `+0x176` | `+0x16c` |
| 1 | `+0x154` to `+0x158` | `+0x160` to `+0x164` | `+0x16e`, `+0x170` | `+0x178`, `+0x17a` | `+0x172` |

Addresses: `0x7b9b10`, `0x7b9cf0`, `0x7b9d50`, `0x7b9da0`, `0x7b9de0`, `0x7b9e20`, `0x7b9cd0`, `0x7b9d40`, `0x7b4d60`, `0x70fd4c`
Confidence: verified
Proof: one
Test: none

`0x7b9cf0` replaces a lifespan under 0.001 with 0x3a83126f and stores `t0 = L·m0`, `t1 = L·m1`
and `1/(t1 − t0)`; the record passes `(0, mid)` and `(mid, 1.0)`. `0x7b9b10` scales by 0.99
and adds 0.005 (floats 0x3f7d70a4 and 0x3ba3d70a), takes byte deltas from `0x7b9d50`, multiplies the alpha
byte by its second argument, and calls `__CIfmod` with 1.0 unless the repeat word is 0x3f800000.
The colour word holds alpha in its top byte; `>> 14` shifts the float's bits.

## Twinkle

An emitter with a visible fraction under 1, or a twinkle scale range, picks a noise value per
particle: index `(byte(clamp(speed·age, 0, 255)) + (particle address >> 5)) & 127` into the 128
noise floats. Under a visible fraction below 1, a particle whose value exceeds it is not
drawn. With a scale range its size is multiplied by `min + value·range`.

Addresses: `0x7b2a5e`, `0x7b2a86`, `0x7b2aef`, `0x7b2b71`, `0xcf58f0`
Confidence: verified
Proof: one
Test: none

`0x7b2a86` multiplies `+0x1b0` by the age, clamps to 0 and 255 with `fcom`, and converts through
`+ 512.0` and `>> 14`; it adds the particle pointer shifted right by 5 and masks with 0x7f. The
draw test at `0x7b2aef` returns 0 when the value compares above `+0x1b4` (neither C0 nor C3). The
scale is `value·+0x1c0 + +0x1b8` (`0x7b2b84`). The table is `cache#noise-table`.

## Particle view space

An emitter draws in view space with the device view set to identity. Positions go through the
particle matrix: for an emitter in its own space, the emitter matrix, then a translation by minus
the camera position, then the view; otherwise the reference frame, when given, then the translation
and the view. Every vertex takes as normal the view image of world +z.

Addresses: `0x7b3d20`, `0x7b3e41`, `0x7b3ef9`, `0x7b3f48`, `0x7b3fc7`, `0xcf5b68`, `0x7b5160`
Confidence: verified
Proof: one
Test: none

`0x7b3e41` saves the view and sets identity; it is restored at `0x7b44cc`. The translation row
holds `−+0x23c` to `−+0x244`, the camera position `SetFrame` copies. Each product uses
`0x7bc6a0`, and the result is copied to `0xcf5b68`. The normal is row 2 of the saved view
(`0x7b3fc7` into `0xcf5878`). `SetFrame` stores `matrix·inverse(frame)` at `+0x1fc` for a world
space emitter with a frame, else the matrix, and the length of its first row at `+0x264`.

## Head quads

A head is a square of half-size `s` around the view position, corners (−1, 1), (−1, −1), (1, 1),
(1, −1) with texture corners (0, 0), (0, 1), (1, 0), (1, 1) inside its cell; a cell `c` of a grid
with `C` columns and `R` rows starts at `((c & (C − 1))/C, (c >> log2 C)/R)`. With a spin `w` the
square turns by `w·age` in the view plane, negated for alternate particle slots under flag 0x8000.
With flag 0x2000 the square lies in the emitter's xy plane instead, turned about the emitter's z
axis. Flag 0x200 scales `s` by the emitter's scale.

Addresses: `0x7b2a50`, `0x7b2bc3`, `0x7b2c1c`, `0x7b2d0c`, `0x7b2dda`, `0x7b2f5e`, `0x7b41a5`, `0x7b4249`, `0x7b4ed0`, `0x87d714`
Confidence: verified
Proof: one
Test: none

The corners are the float pairs from `0x87d714` and the texture corners from `0x87d734`. `0x7b4ed0`
needs both grid sizes powers of two. The spin is `+0x18c·age`; the sign flip tests bit 5 of the
particle pointer (`0x7b2dea`); the plain turn adds `cx·s·cos − cy·s·sin` and `cx·s·sin + cy·s·cos`.
`0x7b41a5` takes the flat corners (∓1, ±1, 0) through the 3×3 part of `+0x1fc` times the particle
matrix (the particle matrix alone in emitter space); `0x7b4249` keeps its normalized z row as the
axis for `0x7be490`.

## Tail quads

A tail runs from the particle back along its velocity: `T = length·(−v)` in view space, `length`
the record's tail length, capped at the particle's age under flag 0x10000. When `T` is shorter
than 1/36 in the view plane, the tail is drawn as a head square with the tail cell. Otherwise its
four corners are the particle and the tip `p + T`, each offset by `±s·(−T.y, T.x)/|T.xy|`, with
texture corners (0, 0) and (0, 1) at the particle, (1, 0) and (1, 1) at the tip.

Addresses: `0x7b3041`, `0x7b30ae`, `0x7b30ce`, `0x7b310f`, `0x7b313c`, `0x7b33fa`, `0x7b4f60`, `0x7bcb40`
Confidence: verified
Proof: one
Test: none

The tail uses the tail cell from the gradient and `+0xb4`; `0x7b30ae` replaces it by the age when
it compares above it. `0x7bcb40` takes `−v` through `0xcf5b68` as a direction. The squared length
of its x and y is compared with 0.0007716 (1/1296) and `test ah,1` takes the square path. The
offsets are `a = T.x·s/L` and `b = T.y·s/L`: the corners are `(p.x − b, p.y + a)`,
`(p.x + b, p.y − a)`, then the same about the tip.

## Draw order and batches

An emitter with flag 0x10 draws its particles in descending view z, the third column of the
particle matrix, through a binary heap; others draw in live-list order. Each quad is two triangles
(0, 1, 2) and (3, 2, 1); with both heads and tails a particle writes eight vertices. A frame draws
at most `0x4000/vertices` particles, the first popped or the first in the live list, and nothing
while the texture has no device object. The colour bytes are swapped to the device's order when its caps ask.

Addresses: `0x7b3a10`, `0x7b3a5b`, `0x7b3aa3`, `0x7b3c50`, `0x7b42a8`, `0x7b42cd`, `0x7b2b46`, `0x7b63a0`, `0x7b64a0`
Confidence: verified
Proof: one
Test: none

`0x7b3a5b` computes `x·m02 + y·m12 + z·m22 + m32` from `0xcf5b70`, `0xcf5b80`, `0xcf5b90` and
`0xcf5ba0`, pushes it, and sifts up while the key is not below the parent; the pops take the
root first. `0x7b3c50` fills 0x1000 six-index quads or 0xaaa twelve-index pairs. `0x7b42cd`
divides 0x4000 by the vertices per particle and caps it at the live count. `0x7b2b46` swaps bytes
0 and 2 when caps `+0x1c` is 1.

## Model particles

An emitter whose record names a model emits 0x40-byte particles that each carry a model instance.
At birth the particle takes its orientation from the spawn matrix, scaled by `1/|row 0|` when row 0
is not of unit length, and a spin vector of `min + U·range` on x and `(1 + U)·range` on y
and z, each component's sign flipped at random under flag 0x8000; its instance is created on
first need. Each step turns the orientation by `|ω|·dt` about `ω` when `|ω|` exceeds 1e-4, then
moves the particle as any other. Each frame every particle's instance animates with the owner's
view, colours and alpha and draws with the owner's lighting record.

Addresses: `0x7b2420`, `0x7b24ca`, `0x7b2582`, `0x7b25dd`, `0x7b2648`, `0x7b28e0`, `0x7b29c0`, `0x719180`, `0x7191b2`, `0x7191be`
Confidence: verified
Proof: one
Test: none

`0x7b2420` calls slot 2 for the position, then builds the quaternion at `+0x20` with `0x7c0120`;
`0x7b2582` stores `U·+0x1cc + +0x1c8`, `(U + 1)·+0x1d4` and `(U + 1)·+0x1dc` at `+0x30` (the
`fsub 1.0; fadd 1.0` pair at `0x7b25a2`); the sign draws test bit 0 of three random words.
`0x7b28e0` compares `|+0x30|` with 1e-4 and composes `(ω/|ω|·sin(h), cos(h))`, `h = |ω|·dt/2`,
with `+0x20`. `0x7191b2` calls `Animate(scene +0x9c, +0x1a0, +0x1ac, +0x19c)` and `0x7191be`
copies `+0x3b8`.

## Registries

The particle registry, created on first use at `0xcf575c`, holds four arrays: live and free legacy
emitters, live and free model emitters. The ribbon registry at `0xcf5760` holds two. Destroying
the cache releases everything in both. Most of their functions have no caller.

Addresses: `0x7ae010`, `0x7ae190`, `0x7ae1d0`, `0x7ae240`, `0x7ae780`, `0x7aecd0`, `0xcf575c`, `0xcf5760`
Confidence: verified
Proof: one
Test: none

`0x7ae010` allocates 0x40 bytes and fills the noise table on creation. `0x7ae780`, called by the
copy constructor `0x7b14a0`, clones through slot 4 of the prototype and appends the clone. The
releases call `0x7ba810` for legacy emitters and `0x7b5e50` for the others.
