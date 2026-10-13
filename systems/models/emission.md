# models: emission

How a model's particle record becomes a `CParticleEmitter2`, and how that emitter places a new
particle: the record layout, the flags it maps, the parameters the model pushes each frame, and the
three spawn shapes with the speed roll, birth age and space they share. The update, motion and draw
of the particles are `particles`.

The code is the particle loop of the model initializer (`0x70f9a0` through `0x7100b5`), the
particle block of `UpdateAfterAnimate`, and the shape classes of the particle object file. A
particle is 0x20 bytes: position `+0x0`, gradient byte `+0xc`, velocity `+0x10`, age `+0x1c`.

Roots: none

## Particle record

A particle record is 0x1f8 bytes. The ten float tracks follow the element track layout and give
each parameter its first value at creation and its animated value each frame.

| offset | holds |
|---|---|
| `+0x004` | flags (`Record flags`) |
| `+0x008` | position relative to the bone |
| `+0x014`, `+0x016` | bone, texture index (16-bit) |
| `+0x018`, `+0x020` | model name and child emitter model name, count and pointer each |
| `+0x028`, `+0x02a`, `+0x02c` | blend; emitter type 1 plane, 2 sphere, 3 spline; 0 head, 1 tail, 2 both |
| `+0x030`, `+0x032` | texture grid rows, columns |
| `+0x034` to `+0x130` | speed, variation, vertical range, horizontal range, gravity, lifespan, rate, area length, area width, z source |
| `+0x14c` to `+0x164` | gradient midpoint, three colours, three scales |
| `+0x168` to `+0x17a` | head and tail cell ramps and repeats (`particles#gradients`) |
| `+0x17c` | tail length |
| `+0x180` to `+0x1d8` | twinkle, spin, drag, model ranges, wind, follow mapping, spline points |
| `+0x1dc` | enabled, a byte track |

Addresses: `0x70ebd0`, `0x70f9b9`, `0x70fa5e`, `0x70fc4f`, `0x70fd22`, `0x717db5`
Confidence: verified
Proof: one
Test: none

The loop steps records by 0x1f8 (`0x7100a0`) and states by 0x16c. `0x70f9b9` switches on the 16-bit
type with `dec`: 1 builds a plane (0x2a0 bytes), 2 a sphere (0x2a4), 3 a spline (0x2d0); any other
type, like a failed allocation, takes `0x70f92f`, which returns 0 from the initializer. Each track's
first value is read through its values pointer, track `+0x18` (`0x70fa5e` reads `+0xf4` for the rate
track at `+0xdc`). `0x70fd22` passes `+0x30` and `+0x32` to `0x7b4ed0`; `0x717db5` samples the byte
track `+0x1dc` into state `+0x14c`.

## Record to emitter

The record's scalars land in the emitter as follows; the twinkle, spin and follow terms are used
by the draw and the update.

| record | emitter |
|---|---|
| `+0x180`, `+0x184` | twinkle speed `+0x1b0`, visible fraction `+0x1b4` |
| `+0x188`, `+0x18c` | twinkle scale: minimum `+0x1b8`, range `+0x1c0` = max − min |
| `+0x190` | inherited velocity scale `+0x1c4` |
| `+0x194`, `+0x198` | drag `+0x1e0`, spin `+0x18c` |
| `+0x19c` to `+0x1b0` | model particle minimum and maximum vectors, `+0x1c8` to `+0x1dc` as minimum and range |
| `+0x1b4`, `+0x1c0` | wind `+0x1e4`, wind time `+0x1f0` |
| `+0x1c4` to `+0x1d0` | follow mapping `(s0, f0, s1, f1)`: slope `+0x1f8`, intercept `+0x1f4` |
| `+0x1d4` | spline points, through `0x7b9a80` for type 3 |

Addresses: `0x70ff27`, `0x70ff3f`, `0x70ff7d`, `0x70ffd7`, `0x70ffef`, `0x710029`, `0x7b5d30`, `0x710045`
Confidence: verified
Proof: one
Test: `models::part_span_deltas_difftest`, `models::remap_fit_difftest`, `models::remap_gate_difftest`

`0x70ff3f` stores `+0x188` at `+0x1b8` and `+0x18c − +0x188` at `+0x1c0`; the three model pairs
store the minimum and `max − min` the same way. `0x7b5d30(s0, f0, s1, f1)` stores the slope `(f1 −
f0)/(s1 − s0)` and `f0 − slope·s0`, or zero for both when `|s1 − s0| < 2.384186e-7`.

Of `0x70ebd0`, `part_span_deltas_difftest` runs `0x70ff3f`–`0x710029`; `remap_fit_difftest` runs
`0x7b5d30` whole; of `0x7b5d30`, `remap_gate_difftest` runs `0x7b5d33`–`0x7b5d48`.

## Record flags

| record flag | emitter effect |
|---|---|
| 0x2 | flag 0x10: particles are drawn in depth order |
| 0x10 | flag 0x100: particles stay in the emitter's space |
| 0x20 | flag 0x200: size scales by the emitter matrix's scale |
| 0x40 | flag 0x400: particles inherit the emitter's velocity |
| 0x80, 0x100 | sphere only: flags 0x800 (kill when moving away from the origin), 0x4000 (velocity along +z) |
| 0x200 | flag 0x8000: random spin and model particle signs |
| 0x400 | flag 0x10000: the tail length is capped at the particle's age |
| 0x800, 0x1000 | flags 0x1000 (spawn between the last and current position), 0x2000 (flat quads) |
| 0x2000, 0x4000 | flags 0x20000 (snap to the ground), 0x40000 (follow the emitter's motion) |
| 0x8000 | burst mode |

Material flag 1 is set when record flag 1 is clear, flag 2 when record flag 8 is clear; the blend
`+0x28` maps 1 to 6 to modes 1, 2, 0, 3, 4, 5, any other value to mode 0, and flag 4 stays set only
for blend 1, 3 or another value.

Addresses: `0x70faf8`, `0x70fb8e`, `0x70fc32`, `0x70fc44`, `0x70fca4`, `0x70fcf2`, `0x70fd13`, `0x7b4f60`, `0x7b5d00`, `0x70fcae`, `0x7102d0`
Confidence: verified
Proof: one
Test: none

Each test sets its bit with `and ~bit; xor bit`. The sphere bits sit under `cmp word [+0x2a], 2`.
`0x70fc44` shifts flag 0x400 into the fourth argument of `0x7b4f60`, which also takes head and tail
from `+0x2c` and the tail length `+0x17c` into `+0xb4`. The blend switch `0x70fcae` jumps through
`0x7102d0` on `+0x28` minus 1; `0x70fcf2` and `0x70fd01` fold record bits 0 and 3, inverted, into
the material.

## Per-frame parameters

Each frame the model pushes every track whose key count exceeds the first-update gate into the
emitter's setters and stores the instance alpha in `+0x1a8`. The emitter emits while its enabled
byte is set and the instance is visible: flag 1 follows that, and the rate is the rate track's
value, or 0. A burst-mode emitter keeps flag 1 clear and requests one burst, flag 0x20, on the
frame its enabled byte and a positive rate first hold, if the instance is visible. With the
enabled byte clear and the gate set, the other parameters stay. The emitter matrix is the bone
matrix moved to the record position, times the scene matrix, turned by π/2 about z.

Addresses: `0x718ea0`, `0x718ed0`, `0x718f12`, `0x718f2d`, `0x718f56`, `0x719073`, `0x717eb0`, `0x7b4ba0`, `0x7b5c50`, `0x719142`
Confidence: verified
Proof: one
Test: `models::half_pi_difftest`

`0x717eb0` stores state `+0x160` = byte `+0x14c` and instance `+0x50`; `0x718f2d` passes it to
`0x7b4ba0`, which copies bit 0 into flag 1, and `0x718f56` calls slot 10 with state `+0xcc` when
`+0x160` is set, else 0. In burst mode (`+0x4` bit 15) the request needs `+0x14c`, `+0xcc > 0`,
`+0x50` and state `+0x168` clear, and `+0x168` latches; creation calls `0x7b4ba0(0, 1)`. Speed and
variation use `0x7b4c50` and `0x7b4c70`, the ranges slots 8 and 9, gravity, lifespan, area length
and width slots 6 and 7, z source `0x7b5cd0`, which stores 0 when `|z| < 0.001`. `0x719142` calls
`0x7bdd60` with half of 2π about (0, 0, 1).

Of `0x718960`, `half_pi_difftest` runs `0x719114`–`0x719142`.

## Speed and birth

Every shape starts a particle the same way: gradient byte 0 and age `U·dt`, where `U` is uniform in
[0, 1) and `dt` the substep (0 for a burst). The speed is `speed·(1 + S·variation)`. With `m` the
low 23 bits of a random word `w`, `U = m/2^23` and `S = ±(1 − m/2^23)`, positive when `w` is
negative: `S` takes the values `±j/2^23`, `j` from 1 to 2^23, so it is never 0 and can be ±1. `U` is
`asfloat((w & 0x7fffff) | 0x3f800000) − 1`; `S` takes the same float `x` in [1, 2) and gives `2 − x`
in (0, 1] when `w` is negative, else `x − 2` in [−1, 0).

Addresses: `0x7b1cc0`, `0x7b88c4`, `0x7b8da4`, `0x7b9532`, `0x4531e0`, `0x801628`
Confidence: verified
Proof: one
Test: `models::burst_velocity_difftest`, `models::emit_shape_plane_difftest`, `models::emit_shape_sphere_difftest`, `models::emit_shape_spline_difftest`, `models::emitter_speed_difftest`, `models::emitter_speed_states_difftest`

`0x7b1cc0` inlines the state advance of `0x4531e0` on `+0x2c` and returns
`(S·+0x184 + 1.0)·+0x17c`. The age store is `fmul [ebp+0xc]; fstp [edi+0x1c]` in each kernel; the
burst call passes 0 (`particles#spawn-count`). `S` uses `fld 2.0; fsub x` on `jge` not taken.

`burst_velocity_difftest`, `emit_shape_plane_difftest`, `emit_shape_sphere_difftest`,
`emit_shape_spline_difftest`, `emitter_speed_difftest` and `emitter_speed_states_difftest` run
`0x7b1cc0` whole; `burst_velocity_difftest` and `emit_shape_plane_difftest` run `0x7b8890` whole;
`emit_shape_sphere_difftest` runs `0x7b8d70` whole; `emit_shape_spline_difftest` runs `0x7b9500`
whole.

## Plane spawn

A plane particle starts at `(S·length/2, S·width/2, 0)`, the x draw taken after the y draw. With a
z source `z` its velocity is `speed·normalize(p − (0, 0, z))`; otherwise, with `θ = S·vertical` and
`φ = S·horizontal`, it is `speed·(cos φ·sin θ, sin φ·sin θ, cos θ)`.

Addresses: `0x7b8890`, `0x7b892b`, `0x7b8970`, `0x7b8a73`, `0x7b8c10`, `0x7b8c20`, `0x7b8c30`, `0x7b8c40`
Confidence: verified
Proof: one
Test: `models::burst_velocity_difftest`, `models::emit_shape_plane_difftest`

Slots 6 to 9 store length `+0x290`, width `+0x294`, vertical `+0x298`, horizontal `+0x29c`. The
second draw scales `+0x294`, the third `+0x290` (`0x7b892b`), each times 0.5. The z source is
`+0x188`; the radial branch divides the speed by `sqrt` of the squared length with no zero test.
`0x7b8a73` takes `fsin` and `fcos` of θ times the speed, then of φ.

`burst_velocity_difftest` and `emit_shape_plane_difftest` run `0x7b8890` whole.

## Sphere spawn

A sphere particle starts at radius `R = min + U·(max − min)` along `(cos A·cos B, cos A·sin B, sin A)`
with `A = S·vertical` and `B = S·horizontal`. Its velocity is the speed times, in order of
precedence: `normalize(p − (0, 0, z))` with a z source; (0, 0, 1) with flag 0x4000; otherwise the
unit direction itself.

Addresses: `0x7b8d70`, `0x7b8dc7`, `0x7b8e5b`, `0x7b8ecc`, `0x7b8f7f`, `0x7b9180`, `0x7b91a0`, `0x7b91c0`, `0x7b91d0`
Confidence: verified
Proof: one
Test: `models::emit_shape_sphere_difftest`, `models::sphere_set_max_difftest`, `models::sphere_set_mid_difftest`

Slots 6 and 7 store the minimum `+0x290` and maximum `+0x294` and keep `+0x298 = max − min`; slots 8
and 9 store `+0x29c` and `+0x2a0`. `0x7b8e5b` and `0x7b8e76` take `fsincos` of A and B. The
direction is scaled by `R` into the position, and the chosen velocity by the speed roll at
`0x7b8fe6`, unrounded: `0x7b8fee` multiplies by the roll still on the FPU stack, where the plane and
the spline store it as a float first (`0x7b896d`, `0x7b95a4`).

`emit_shape_sphere_difftest` runs `0x7b8d70` whole; `sphere_set_mid_difftest` runs `0x7b9180` whole;
`sphere_set_max_difftest` runs `0x7b91a0` whole.

## Spline spawn

A spline particle sits at parameter `t = start + U·(end − start)` of the emitter's spline; after the
end changes, the next one sits at `t = end`. With a z source its velocity is radial as for a plane.
With a vertical spread `v`, the direction is +z rotated by `S·v` about the spline's unit tangent at
`t`, the velocity is the speed along it, and with a horizontal spread the position moves
`U·horizontal` along it. With neither, the velocity is `(0, 0, speed)`. The three tests read their
words as integers, so −0.0 counts as set.

Addresses: `0x7b9500`, `0x7b9538`, `0x7b9598`, `0x7b963b`, `0x7b96f0`, `0x7b9737`, `0x7b9970`, `0x7b99c0`, `0x7b9a60`, `0x453390`, `0x453420`, `0x7be490`, `0x7b973f`, `0x7b9747`, `0x7b9a09`
Confidence: verified
Proof: one
Test: `models::emit_shape_spline_difftest`, `models::particle_quad_expand_tiled_rotated_difftest`, `models::spline_set_progress_difftest`, `models::spline_set_scale_difftest`

Slot 6 clamps the start into [0, 1] at `+0x294`. Slot 7 clamps the end into [0, 1] (NaN to 1) and,
when `|end − +0x298|` is at least 2.384186e-7, stores it at `+0x298`, sets `+0x2a4` and the rate
`+0xa8 = end·+0x290`: `jnp` at `0x7b9a09` returns on C0 alone, so an equal move stores; slot 10
stores the rate at `+0x290`. `0x7b9538` takes `+0x298` and clears `+0x2a4`. The rotated +z is row 2
of `0x7be490(out, tangent, angle, 1)` (rows 0 and 1 enter times 0.0); `0x7be490` normalizes the axis
when its byte is 0, takes `fsincos` of the angle and returns `out` (`ret 8`). The z source `+0x188`,
the spread `+0x29c` and the horizontal `+0x2a0` are tested with `test eax, eax`; the horizontal draw
at `0x7b9747` runs only when `+0x2a0` is set (`0x7b973f`), so it shifts the random stream only then.
The zero-spread branch leaves the speed in the z slot (`0x7b95a4`).

`emit_shape_spline_difftest` runs `0x7b9500` whole; `spline_set_progress_difftest` runs `0x7b99c0`
whole; of `0x7b9a60`, `spline_set_scale_difftest` runs `0x7b9a63`–`0x7b9a7c`;
`particle_quad_expand_tiled_rotated_difftest` runs `0x7be490` whole.

## Spawn space

Without flag 0x100 the new position goes through the emitter matrix and the velocity through it as
a direction, so the particle lives in world space. With flag 0x20000 it then asks the scene's
ground callback for the height below it and, when answered, sits at that height plus its current
size. With flag 0x100 both stay in the emitter's space. Last, with flag 0x400, the emitter's
velocity times `1 + S·variation` adds to the particle's velocity.

Addresses: `0x7b8a9a`, `0x7b8b26`, `0x7b8b3d`, `0x7b9805`, `0x7bcb40`, `0x7bca80`, `0x7b2140`, `0x70a250`
Confidence: verified
Proof: one
Test: `terrain::portal_screen_rect_difftest`, `models::burst_velocity_difftest`, `models::color_scale_apply_sum_difftest`, `models::emit_shape_plane_difftest`, `models::emit_shape_spline_difftest`, `models::particle_quad_expand_stretched_difftest`

`0x7b8aa0` tests `+0x1ac` bit 8; the clear branch calls `0x7bcb40` with w = 0 for the velocity and
`0x7bca80` for the position, then `0x7b2140` under bit 17: it calls the callback `0xcf5758` with
`0xcf5750`, which `0x70a250` stores and the world sets to its ground probe (`0x66fa0b`), and adds
the size from `0x7b9b10` to the height. `0x7b8b3d` tests bit 10 and adds
`(S·+0x184 + 1)·+0x258` to the velocity. The sphere (`0x7b9016`) and spline (`0x7b9805`) share
the block.

Of `0x7b2140`, `color_scale_apply_sum_difftest` runs `0x7b219c`–`0x7b21aa`;
`burst_velocity_difftest` and `emit_shape_plane_difftest` run `0x7b8890` whole;
`emit_shape_spline_difftest` runs `0x7b9500` whole; `particle_quad_expand_stretched_difftest` runs
`0x7bcb40` whole.
