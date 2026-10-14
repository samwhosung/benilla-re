# models: animation

How an instance animates: the animation id it resolves, the sequence each key bone plays with its
blend from the last, the key lookup and interpolation of a track, the bone transform with its pivot
and billboards, the animated colours, textures, lights and emitters, and the per-update events.

The code is M2Model.cpp's (`model`). A bone state is a 0x118-byte record per bone at instance
`+0x90`; its two sequence slots, the primary at `+0x98` and the blend source at `+0xc4`, hold a
time, a sequence, a start stamp, an end stamp, a speed, its inverse and an offset. A sequence is a
0x44-byte header record; a track is 0x1c bytes: interpolation word, global sequence word, then
count and pointer pairs of ranges, times in milliseconds and values.

Roots: `0x7121a0`

## Animation id resolution

`ResolveAnimationId(id)` returns an id the model has and two flags. An id below the header's
playable count reads its pair from the playable table. Otherwise it follows the client's animation
table: while the id is past the lookup or unmapped, it moves to the row's fallback; a row with flag
0x10 reverses playback and one with 0x20 holds the last frame. A loop, a missing row or a row that
falls back to itself ends at the default: animation 0 when mapped, else 0x93, else the first
sequence.

Addresses: `0x711bf0`, `0xc0e070`, `0xc0e074`
Confidence: verified
Proof: one
Test: none

The playable table is header `+0x30`, counted at `+0x2c`; the lookup is `+0x28`, counted at
`+0x24`. The animation rows come from `0xc0e070` with the highest id at `0xc0e074`; a row's flags
are `+0x14` and its fallback `+0x18`. Two counters start at 1 and 0; flag 0x10 adds the first to
the second and negates the first, flag 0x20 adds it and zeroes it. The result flag is 0 for a
positive first counter, 1 for a negative one, else 3 when the second is positive and 2 otherwise.
A visited array of 0xd0 dwords stops loops.

## Playing an animation

`PlayAnimation(keyBone, id, variation, offset, speed, blend, primary)` first calls the done
callback for an animation that ends unfinished. Id −1 stops the key bone, fading over 150 ms when
it blends. Otherwise it resolves the id, picks the variation (−1 draws one by probability), and
fills the primary slot, or the blend slot when not primary, with a repeat count drawn between the
sequence's minimum and maximum, the speed negated for a reversed id and zeroed for a held one. A
non-primary play fades out over the sequence's length `end − start`, at rate `1/(end − start)`
and scale 0.75.

Addresses: `0x7121a0`, `0x711bf0`, `0x710310`, `0x7126a8`, `0x7126f4`, `0x712701`, `0x71278f`, `0x712647`, `0x712682`, `0x7125c9`
Confidence: verified
Proof: one
Test: `models::anim_fade_difftest`, `models::keyframe_ratio_difftest`, `models::keyframe_timing_difftest`, `models::rate_reciprocal_difftest`

The key bone maps through header `+0x3c`; −1 is bone 0. The variation chain follows sequence
`+0x40`; a random draw subtracts each probability `+0x14` from `rand()`. Repeats are `min +
((max − min)·rand()) / 32768`, a signed divide truncating toward zero (`cdq; and edx,0x7fff; add;
sar 15` at `0x7126a8`), 0 becoming 1; the length is `(end − start)·repeats`. With `inv = 1/speed`
(0 for `|speed| ≤ 1e-5`), the slot's start is `clock − ftol(offset·|inv|)`, its end `start +
ftol(length·|inv|)`, and its offset `+0x20` the length when the speed, after negation and before
zeroing, is negative and 0 otherwise (`0x7126f4` to `0x712701`, stored at `0x71278f`). A blend
copies the primary slot into the blend slot when the current weight is at most 0.5 (not when it
is above 0.5 or NaN), sets the fade end to `clock + blend time`, its rate to `1/blend time` and its
scale to 1.0; a non-primary play sets them to `clock + (end − start)`, `1/(end − start)` (1.0 for
an empty range) and 0.75 (`0x712647` to `0x712682`). The stop fade uses 150 ms and 1/150 (bits
0x3bda740e).

Of `0x7121a0`, `anim_fade_difftest` runs `0x712352`–`0x7125d4`, `rate_reciprocal_difftest`
`0x71260c`–`0x712633`, `keyframe_timing_difftest` `0x7126d2`–`0x71279c`, `keyframe_ratio_difftest`
`0x712718`–`0x712731`.

## Sequence time

Each frame a bone with a primary sequence computes its time. For a looping sequence it is
`start + (ftol((now − startStamp)·speed) + offset) mod (end − start)`, or `start` for an empty
range. A sequence with flag 1 plays once: at or past its end stamp the time is `start +
ftol((endStamp − startStamp)·speed) + offset`, or `start` when that sum is negative and `end` when
it exceeds the length; before its end stamp the looping formula runs with `now` raised to the start
stamp. A paused instance moves both stamps forward by the pause.

Addresses: `0x714260`, `0x714585`, `0x7145a1`, `0x7145e9`, `0x71462a`
Confidence: verified
Proof: one
Test: none

The stamps are bone `+0xa8` and `+0xac`, the speed `+0xb0`, the offset `+0xb8`; the products are
`fild` of the 32-bit difference times the float speed, truncated by `__ftol`, and the modulo is
unsigned. The result goes to `+0x98` with the sequence at `+0x9c`. A bone without a sequence copies
its parent's slot, or bone 0's; the blend slot `+0xc4` works the same and ends when the fade end
passes.

## Blend weight

The blend weight is 0 when the fade ended or both slots play the same key; otherwise `w = rem ·
rate`, with `rem` the milliseconds to the fade end, rounded to a float, and the weight is
`k·0` for `w < 0`, `k·1` for `w > 1`, and `k·(3 − 2w)·w·w` between, `k` being the slot's scale
(1.0, or 0.75 for a non-primary play). A NaN `w` fails both tests and gives NaN. A bone without
either slot inherits its parent's weight.

Addresses: `0x714260`, `0x714880`, `0x714909`, `0x71491b`
Confidence: verified
Proof: one
Test: none

At `0x7148b3`: `fild rem; fmul +0x104; fst w`; compare with 0.0 and 1.0; `fld w; fadd st,st;
fsubr 3.0; fmul w; fmul w; fmul +0x108; fstp +0x10c`. The rate `+0x104` and scale `+0x108` are
written by `0x7121a0`.

## Key lookup

`FindKey(time, range, track)` finds the key pair around a time. The range is the track's range
for the sequence, or all keys; a global-sequence track uses its global time instead. It starts
from the last key found: within 500 ms forward it steps forward, within 500 ms back it steps back;
otherwise it steps forward from the range's first key when `t` is within 500 ms of it, and else
binary-searches. The fraction is `(t − t0) / (t1 − t0)` from a 64-bit unsigned numerator over a
signed 32-bit denominator, rounded to a float. Only the track's last key gives fraction 0; at a
range's last key the pair runs into the next range's first key.

Addresses: `0x713d50`, `0x713dff`, `0x713e4e`, `0x713ea0`, `0x71af20`, `0x71ae90`
Confidence: verified
Proof: one
Test: none

A range with `lo ≥ hi` returns `{lo, lo, 0}`. The global time is instance `+0x64[gs]`. The windows
compare `t − times[k]` unsigned with 500 and 0xfffffe0c. The binary search halves with `(lo +
hi) >> 1` and stops when `t < times[m+1]`. The step from the first key is `0x713dff`; the end test
`0x713e4e` compares `k + 1` with the track's time count, not the range's end. `0x713ea0`
interpolates each quaternion component as `(v1 − v0)·s + v0`; its blend toward the blend slot's
sample goes through `0x74d114` (`Dispatched matrix math`). `0x71af20` lerps floats both ways.

## Bone transform

A bone with no parent uses the instance's world matrix as its parent. Flag 2 normalizes the
parent's rows; 4 makes each row `world row · sqrt(|parent row|² / |world row|²)`, factor 1.0 when
`|world row|² ≤ 1e-5` or NaN; 6 takes the world rows. Flag 1 takes the world's translation;
otherwise flags 2, 4 and 6 recompute it so the pivot keeps its parent-transformed position. An
animated bone builds `rotation · scale`, applies its override matrix, sets `row3 = (p + t) −
((m20·pz + m10·py) + m00·px)` per column for pivot `p` and translation `t`, with `p + t` rounded to
a float for x and y but not for z, and multiplies by its parent.

Addresses: `0x714260`, `0x714961`, `0x714a96`, `0x714ab9`, `0x714caf`, `0x7151b0`, `0x74b6b5`, `0x74a7c0`
Confidence: verified
Proof: one
Test: `models::m2_decal_projection_difftest`

The flags are bone record `+0x4` or'd with the runtime flags `+0xf4`. Flag 4's factor is `fdivrp`
(`de f1`) at `0x714ab9`, `|parent row|²` over `|world row|²`, then `fsqrt`; the 1e-5 test is
`0x714a96`. The pivot recompute is `0x714caf`. Rotation samples the track at record `+0x28` and
builds its matrix with `0x74b6b5`, and the product is `0x74a7c0(bone, local, parent)`; both are
dispatched routines (`Dispatched matrix math`), so the claim holds for the steps around them and
their results are not transcribed. Scale samples `+0x44` and scales the rows; translation samples
`+0xc` and adds the pivot `+0x60`; `fsubr st,st(1)` at `0x7151b0` uses the unrounded z sum. Each
track is sampled only when it has more keys than the first-update gate `+0x8c` allows.

Of `0x74a7c0`, `m2_decal_projection_difftest` runs `0x74a7c0`.

## Dispatched matrix math

The bone product, the rotation matrix of a quaternion, the blend-slot quaternion blend and the
scene matrix are linked math routines called through writable slots. The first call fills the
slots once by processor and registry: with `DisablePSGP` or `DisableD3DXPSGP` under
`HKLM\Software\Microsoft\Direct3D` equal to 1, only the defaults; otherwise the 3DNow set when
that feature is present and the value is not 2, else the SSE2 set when CPUID reports SSE2, else the
SSE set when SSE is present. A modern SSE2 processor without the registry values runs the SSE2
product `0x754a66` and inverse `0x754cd2`, and the default quaternion routines `0x74b6bb` and
`0x74d11a`, which no set replaces but 3DNow.

| slot | thunk | default | SSE2 | SSE | 3DNow |
|---|---|---|---|---|---|
| `0x876504` | `0x74a7c0` | `0x7507bb` | `0x754a66` | `0x751055` | `0x7580fb` |
| `0x876544` | `0x74ad38` | `0x74ad3e` | `0x754cd2` | `0x7512c1` | 3DNow set |
| `0x876588` | `0x74d114` | `0x74d11a` | default | default | 3DNow set |
| `0x876594` | `0x74b6b5` | `0x74b6bb` | default | default | 3DNow set |

Addresses: `0x74fb49`, `0x74fa8a`, `0x74f951`, `0x750930`, `0x7501e9`, `0x74ff9a`, `0x74fdf0`, `0x74a7ad`, `0x74b6a2`, `0x74d0ee`, `0x8764f8`, `0x876620`, `0x87674c`, `0x754a66`
Confidence: verified
Proof: one
Test: none

Each slot starts at a stub (`0x74a7ad`, `0x74b6a2`, `0x74d0ee`) that calls `0x74fb49(1)` and
jumps through the slot again. `0x74fb49` runs once while `0x87674c` is 0xffff: it copies 74 dwords
from `0x876620` to `0x8764f8`, applies `0x750930` (slot `+0xc` becomes `0x7507bb`), reads the two
`REG_DWORD` values with `0x74f951`, the second overriding the first, then tests
`IsProcessorFeaturePresent(7)`, `0x74fa8a`'s CPUID leaf 1 EDX bit 26 (returned as 8) and
`IsProcessorFeaturePresent(6)`, applying `0x7501e9`, `0x74ff9a` or `0x74fdf0`. Only `0x7501e9`
writes slots `+0x90` and `+0x9c`. The SSE2 product `0x754a66(out, a, b)` loads all of `b`, then
forms each output row from `a`'s row in single precision: rows 0 and 3 as `(a0·b0 + a1·b1) + (a2·b2
+ a3·b3)`, rows 1 and 2 as `((a0·b0 + a1·b1) + a2·b2) + a3·b3`. The other routines were not read. `0x74fa8a` runs
under an exception scope: its body runs from `0x74fa8a` through `0x74fac2`, `0x74fad0` through
`0x74fad4` and `0x74fadb` through `0x74fb2a`, and its scope-table handler `0x74fac4` through
`0x74facf` returns to `0x74fad6`.

## Billboards

A bone with flag 8, 0x10, 0x20 or 0x40 replaces its rotation with one facing the view: 8 faces
fully, 0x10, 0x20 and 0x40 turn about one axis. The rows are rescaled to the bone's original row
lengths and the translation recomputed from the pivot.

Addresses: `0x7152ea`, `0x71879c`, `0x718788`
Confidence: inferred
Proof: none
Test: none

`0x7152ea` reads the byte table `0x71879c` at `(flags & 0x78) − 8` and jumps through `0x718788`.
The per-axis cross products were read but their row order not settled.

## Element tracks

After the bones, every animated element samples with bone 0's times and blend: colours (an RGB track
and an alpha track of 16-bit values scaled by 1/32767), transparency weights, texture frame words
(stepped), texture transforms (`Texture transforms`), light colours and intensities, camera
positions, ribbon and particle parameters, and attachment visibility.

Addresses: `0x714260`, `0x811610`, `0xcf043c`, `0x71aff0`, `0x71b010`
Confidence: inferred
Proof: none
Test: none

The 16-bit scale is the float at `0x811610`. The centre is the static at `0xcf043c`, built once
under a guard bit. The order and the interpolation of each were read; their blend steps were not
transcribed one by one.

## Instance colours

Each frame the animate pass derives the instance's draw colours from what its caller passes, a
colour `A`, a colour `B` and an alpha `a`: the diffuse `+0x1a0 = +0x184 · A` and the emissive
`+0x1ac = +0x190 + B` per channel, and the alpha `+0x19c = a · +0x180`, each stored as a float. A
top-level instance gets `A` = (1, 1, 1), `B` = (0, 0, 0) and `a` = 1.0; an attached instance gets
its parent's `+0x1a0`, `+0x1ac` and `+0x19c`, so it multiplies its own diffuse and alpha onto its
parent's and adds its emissive to the parent's.

Addresses: `0x714260`, `0x7142cc`, `0x714309`, `0x71433b`, `0x71434c`, `0x714160`
Confidence: verified
Proof: one
Test: none

`0x7142cc` is `fld A.x; fmul +0x184`, then `+0x188 · A.y` and `+0x18c · A.z`, stored at `+0x1a0`;
`0x714309` adds `B` to `+0x190` to `+0x198` into `+0x1ac`; `0x71433b` multiplies the alpha argument
by `+0x180` into `+0x19c` (`0x71434c`). `0x714000` passes the parent's three fields (`0x714160`).
The scene passes the identities (`Animate pass`).

## Global sequence time

Each frame a global sequence's time is the scene clock less the clock when the instance was
initialized, modulo the sequence's length, unsigned; a length of 0 gives 0. Tracks with a global
sequence sample at that time (`Key lookup`).

Addresses: `0x714352`, `0x714370`, `0x714375`, `0x70eae1`
Confidence: verified
Proof: one
Test: none

`0x70eae1` stores scene `+0xc` at instance `+0x68` when it is initialized on a scene. `0x714360`
walks header `+0x14` lengths at `+0x18`: `clock − +0x68` (`0x714370`), `div` by the length
(`0x714375`), the remainder stored at `+0x64[i]`.

## Texture transforms

Each frame each texture transform builds its matrix from identity: when its rotation track has keys,
translate by the centre `p` = (0.5, 0.5, 0), rotate by the sampled quaternion, translate by `−p`;
then the same with the sampled scale; then translate by the sampled translation. Each step
multiplies on the left, so for the row vector `(u, v, 0, 1)` the result is `((uv + t − p) ⊙ s)·Q +
p`, `Q` the quaternion's matrix with rows `(1 − 2(y² + z²), 2xy + 2zw, 2xz − 2yw)`, `(2xy − 2zw, 1 −
2(x² + z²), 2yz + 2xw)` and `(2xz + 2yw, 2yz − 2xw, 1 − 2(x² + y²))`. The tracks sample at bone 0's
time.

Addresses: `0x715f45`, `0x715fdb`, `0x716005`, `0x716010`, `0x716048`, `0x716052`, `0x7161e0`, `0x716216`, `0x716384`, `0xcf043c`, `0x7bdc40`, `0x7bdca0`, `0x7bddb0`, `0x7bc6a0`
Confidence: verified
Proof: one
Test: `cmath::quat_mul4`

The centre is built once under bit 1 of `0xcf04c4` (`0x715f45`). The gates are the time counts of
the tracks at transform `+0x1c`, `+0x38` and `+0x0` (`0x715fdb`, `0x716052`, `0x716216`). `0x7bdc40`
adds `v·rows 0–2` to row 3 (`T(v)·M`), `0x7bdca0` scales rows 0 to 2 (`S·M`), and `0x7bddb0` builds
`Q` with `w = q[3]` and writes `0x7bc6a0(out, Q, M)`, which forms `out[r][c] = Σk Q[r][k]·M[k][c]`.
Interpolation word 0 copies the key's value (`0x71608d`); otherwise the scale and translation lerp,
and blend toward the blend slot when bone 0's weight is nonzero and the track has no global
sequence. The state is `+0xb0[i]` (0x98 bytes), the matrix `+0xb4[i]`.

`cmath::quat_mul4` runs `0x7bddb0` whole.

## Ensure animated

`EnsureAnimated` animates an instance whose last frame is not the scene's: an attached instance
animates its parent first and then animates with the parent's attachment bone matrix, moved to the
attachment offset, as its own parent matrix, and the parent's colours and alpha.

Addresses: `0x714000`, `0x714260`
Confidence: verified
Proof: one
Test: none

The test is `+0x40 != scene+0x10`. The offset moves the translation by `((m20·z + m10·y) +
m00·x) + m30` per column. Without a resolved attachment the parent's world matrix is used.

## Update and events

`Update(dt)` adds `dt` to the pending time and, with a done or event callback set, walks the key
bones that play. A bone's window runs from `max(start stamp, now − dt)` to `min(now, end stamp)`, or
to `now` for a looping sequence; reaching the end stamp queues the done callback once. In
sequence-local times, at most one length apart, it walks the event's keys from `start + (local mod
length)`, wrapping at the range end, and queues the event callback for each key with `local start ≤
key < local end`, or the reverse when playing backwards. An event belongs to the first bone up its
parent chain with a sequence. Attached children with a resolved, visible attachment update too.

Addresses: `0x719370`, `0x7193eb`, `0x719467`, `0x719484`, `0x7194bc`, `0x719503`, `0x71958c`, `0x7195fa`, `0x71965f`, `0x719685`, `0x7196a6`, `0x7196d9`, `0x719781`, `0x719909`, `0x70a280`, `0x70a400`, `0x714000`
Confidence: verified
Proof: one
Test: `models::abs_int_scale_difftest`, `models::bone_emit_frame_difftest`

A not-yet-loaded instance defers the call as kind 9; the pending time is `+0x88`. `0x7193eb` needs
`+0x1f4` or `+0x70`. The comparisons are unsigned (`0x719467`, `0x719484`); sequence flag 1 keeps
the end clamp (`0x719503`). The done callback `+0x70` (arguments `+0x78`, `+0x7c`) is latched by
bone `+0xc0` and skipped for a bone other than 0 whose key bone id is −1 (`0x7194bc`). Local time is
`ftol((t − start stamp)·speed) + offset`; `0x71958c` clamps the span to the sequence length. Events
are 0x2c-byte header records `+0x118`, ranges at `+0x20`, times at `+0x28`; `0x7195fa` walks the
event bone `+0x8` up past bones without a sequence. The cursor is `div` (`0x71965f`), the first key
is the first with `cursor ≤ time` (`0x719685`), `0x7196a6` wraps, and `0x7196d9` stops at the span.
The callback gets the event's id `+0x0`, its data `+0x4`, its offset `+0xc` through its bone and the
scene, and `ftol(|1/speed|·(local end − key))`. `0x719909` reads attachment state byte `+0xc`.

Of `0x719370`, `bone_emit_frame_difftest` runs `0x71952c`–`0x719567`, `abs_int_scale_difftest`
`0x719724`–`0x719861`.

## Pushing animated state

After animating, `UpdateAfterAnimate` positions the instance's lights, sets its cameras' eye,
target and roll, updates its ribbons and particle emitters with the elapsed seconds, puts the
instance on the scene's draw queue, and sets the first-update gate so later frames sample only
tracks with more than one key.

Addresses: `0x718960`, `0x71b650`, `0x71b6a0`, `0x71b780`, `0x7b7e60`, `0x7b5230`, `0x7ac250`, `0x7ac3a0`
Confidence: verified
Proof: one
Test: `models::camera_xform_difftest`, `models::count_scale_difftest`, `models::drift_block_difftest`, `models::eps_gate_difftest`, `models::field_scale_difftest`, `models::half_pi_difftest`, `models::light_translate_adds_difftest`, `models::m2_light_set_direction_difftest`, `models::particle_bone_xform_difftest`, `models::prologue_length_difftest`, `models::ribbon_bone_xform_difftest`, `models::trail_block_difftest`

The elapsed time is `fild qword +0x88` times 0.001, then `+0x88` is zeroed. A point light is placed
at its bone-transformed position, others along the negated third bone row. Cameras set property 7
(eye), 8 (target) and 5 (roll). Each particle emitter gets `0x7b5230(dt, matrix, scene camera,
reference frame)` with its bone matrix rotated by π/2 about z. The gate `+0x8c` becomes 1 at the
end.

Of `0x718960`, `camera_xform_difftest` runs `0x718a7e`–`0x718aed`, `light_translate_adds_difftest`
`0x718b66`–`0x718c03`, `count_scale_difftest` `0x718c78`–`0x718c95`, `field_scale_difftest`
`0x718cf0`–`0x718cfd`, `ribbon_bone_xform_difftest` `0x718d6d`–`0x718dd3`,
`particle_bone_xform_difftest` `0x7190a9`–`0x71910f`, `half_pi_difftest` `0x719114`–`0x719142`;
`m2_light_set_direction_difftest` runs `0x71b6a0` whole; of `0x7b5230`, `prologue_length_difftest`
runs `0x7b523f`–`0x7b528f`, `eps_gate_difftest` `0x7b52d7`–`0x7b5300`, `drift_block_difftest`
`0x7b530e`–`0x7b53cc`, `trail_block_difftest` `0x7b53ce`–`0x7b54cd`.
