# models: particles

The particle emitters a model carries: the emitter object and its three spawn shapes, the update
that advances it in substeps, the spawn count, the motion of one particle, the gradients that
colour and size it over its life, and the quads it draws. A legacy emitter class and the registries
that hold emitters outside a model are also here.

The emitter, `CParticleEmitter2`, is about 0x2a0 bytes; its code runs from `0x7b0dd0` through
`0x7b6990`, its plane, sphere and spline shapes from `0x7b8750` through `0x7b9ad0`, the gradient
records from `0x7b9af0` through `0x7b9f30`, and the legacy `CParticleEmitter` from `0x7b9f50`
through `0x7ba9f0`. The registries are the object file at `0x7adf90` through `0x7af530`. A model
creates one emitter per particle record of its header (`0x70ebd0`) and drives it each frame
(`0x718960`).

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
| 4, 5 | pure | `0x7b87b0`, `0x7b87f0` | `0x7b8c90`, `0x7b8cd0` | `0x7b9260`, `0x7b92a0` |
| 6 to 9 | pure | `0x7b8c10` to `0x7b8c40` | `0x7b9180` to `0x7b91d0` | `0x7b9970` to `0x7b9a50` |
| 10 | `0x7b4bf0` | same | same | `0x7b9a60` |

Addresses: `0x7b8770`, `0x7b8c50`, `0x7b91e0`, `0x7b11b0`, `0x81d83c`, `0x81d888`, `0x81d8b4`, `0x81d8e0`, `0x7b4bf0`, `0x7b9a60`, `0x70ebd0`
Confidence: verified
Proof: one
Test: none

`0x7b8770` calls `0x7b11b0`, zeroes `+0x290` to `+0x29c`, stores `0x81d888` and sets `+0x24` to 1.
The table is read from the image at `0x81d83c` to `0x81d908`; slot 3 is `0x7b4b90` in every table.
The model's initializer picks the shape through the jump table `0x7102d0` on the record's emitter
type. `0x7b4bf0` stores the rate at `+0xa8`, or 0 when it is not positive; the spline's
`0x7b9a60` stores its argument at `+0x290` and `+0xa8 = arg · +0x298`.

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

## Gradients and quads

A gradient record holds three colour keys, three sizes and texture cell ramps for the head and the
tail, with a middle point; the sampler gives a particle's colour, size and cell from its age over
the lifespan. The quad writer expands each particle into a billboard and an optional tail quad
with those values.

Addresses: `0x7b9b10`, `0x7b9cf0`, `0x7b9d50`, `0x7b9e20`, `0x7b2a50`, `0x7b3a10`, `0x7b3d20`
Confidence: inferred
Proof: none
Test: none

`0x7b9cf0` clamps each span to 0.001 before taking its reciprocal. `0x7b9b10` wraps with `fmod`
and reads 0.99 and 0.005; `0x7b2a50` uses 512.0 and 1/1296. Neither was transcribed.

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
