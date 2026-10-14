# terrain: cull

What the client draws of the map each frame: the frustum tests, the chunk visibility pass, the
horizon buffer that hides chunks and objects behind nearer terrain, the per-chunk level of detail
and the release of buffers a chunk has not drawn for five seconds.

The code is the scene object file, `0x67ef30` through `0x6882f0`, with the frame driver
`0x681070`, and the chunk's per-frame step in the chunk file. The frustum is six planes of four
floats (n, d), inside where n·p + d ≥ −0.019444443. The horizon is 320 floats at `0xc7b750`, one
per screen column, each the highest view-space y/z an occluder reached in that column. Both
horizon steps run only with world flag 0x20 and while the camera's forward z, `0xc7cff4`, lies
in [−0.9, 0.9], both ends included; a NaN passes.

Roots: none.

## Culling toggle

World flag 0x20 gates the horizon buffer, the horizon tests and the chunk occluder routing. It starts set and the
`showCull` command flips it, printing "Terrain culling disabled." or "Terrain culling enabled.". With it clear every
horizon test reports visible.

Addresses: `0x673250`, `0x673259`, `0x673265`, `0x67327c`, `0xc7b2a4`, `0x66f6cb`
Confidence: verified
Proof: one
Test: none

`0x673257` tests bit 0x20 of `0xc7b2a4`; set, `0x673265` clears it with `and 0xffffffdf`, else `0x67327c` ors it
in. Both legs print through `0x63cb50` and return 1. `0x66f6cb` ors 0x7100b73 into the word, which has the bit.

## Horizon view

Once a frame `0x680bc0` builds the matrix `0xc7b700` the horizon steps project through. The forward vector
f is the camera target less its position, normalised into `0xc7cfec` to `0xc7cff4`. When the squared length of f's x and y is above 0.0001
(`0x680e50`; a smaller, equal or NaN value takes identity), the matrix is the view looking from the origin along h = (f.x, f.y) normalised with up (0, 0, 1), then translated by
the negated camera position, then multiplied by the device projection `0xc7d280`. A point so projected is seen by a level camera at the
eye with the real lens, and its z is the distance along the heading.

Addresses: `0x680bc0`, `0x680e50`, `0x680e7a`, `0x680ede`, `0x680f0b`, `0x680f1e`, `0x680f48`, `0x680c4d`, `0x680cc5`, `0x680d19`, `0x5c3e70`, `0x7bdc40`, `0x7bc6a0`, `0xc7b700`, `0xc7d280`, `0xc7cfec`, `0xc7cff4`, `0xc7cf20`, `0x80c4f4`
Confidence: verified
Proof: one
Test: none

`0x680c4d` to `0x680c9e` divide f by its length. `0x680cc5` to `0x680d19` form h from f's x and y, normalising when
their squared length is above 0.0001 (`0x680cf3`). `0x680ede` calls the look-at builder `0x5c3e70` with the eye
(0, 0, 0), target h and up (0, 0, 1); `0x680f0b` translates by the negated eye `0xc7cf20`, and `0x680f1e`
multiplies by `0xc7d280`, copying 16 floats to `0xc7b700` at `0x680f30`. The band plane of `depth-buckets` is h
with its distance, and the view plane `0xc7bcb0` is f with distance −f·eye (`0x680cb1`, `0x680cb8`).

## Camera origin

The terrain, liquid, group and doodad passes subtract one stored camera position. `0x680bc0` copies the
three dwords at its first argument to `0xc7cf20`, `0xc7cf24` and `0xc7cf28`; no other instruction stores to
them directly but a zero at start-up. The world frame copies the camera's `+0x8` triple verbatim to a local and
passes it through `0x66fd50`, so these globals equal the position the model scene is rebased on.

Addresses: `0x680bc0`, `0x680bd0`, `0x680bd9`, `0x680be2`, `0x66fd50`, `0x66fe6a`, `0x482ea0`, `0x483099`, `0x48309f`, `0x4830b0`, `0x483294`, `0x483297`, `0xc7cf20`, `0xc7cf24`, `0xc7cf28`
Confidence: verified
Proof: one
Test: none

`0x482ea0` reads the camera at `[world frame + 0x65b8]` (`0x483099`) and copies `+0x8` to `+0x10` as
dwords (`0x4830a2` to `0x4830b0`); `0x483294` passes the copy in `ecx` to `0x66fd50`, which keeps it in
`[ebp-8]` and hands it to `0x680bc0` at `0x66fe6a`. A pass subtracts the globals from an absolute
float with `fsub` and stores a float, as the chunk pass does at `0x684693`.

## Frustum box test

`0x686940(box)`, on six planes, returns 0 when the box is outside a plane and 3 otherwise. For
each plane it takes the box corner farthest along the normal, max on an axis whose normal
component has a clear sign bit and min otherwise, and rejects when (n.x·x + n.z·z) + n.y·y + d
is below −0.019444443.

Addresses: `0x686940`, `0x8101b4`, `0x68695a`, `0x686988`, `0x68698b`
Confidence: verified
Proof: one
Test: `terrain::aabb_frustum_pvertex`

`ret 4`. `0x68695a shr eax,0x1f` picks the corner by the component's sign bit, so −0.0 takes the
minimum. The sum is one 53-bit expression (`0x686988 fadd d`); `0x68698b` compares with
0xbc9f49f4 (`0x8101b4`) and `test ah,5; jnp` rejects below it, a NaN passing.

## Horizon buffer

At each frame the buffer is filled with −1,000,000. A chunk chosen as an occluder adds two of
its vertex rows: each vertex, plus the chunk position, goes through the view matrix to (x, y, z)
and becomes x/z and y/z; for each edge whose two ends both have z of at least 2.7777777, every
column from round(64·x/z − 0.5) + 160 to the other end's, clamped to 0 to 319, is raised to the
lower of the two ends' y/z.

Addresses: `0x685fe0`, `0x681b50`, `0x6c15d0`, `0x681b79`, `0x681b90`, `0xc7b750`, `0xc7b700`, `0xc7cff4`, `0x810180`, `0x810170`, `0x86861c`, `0x681c0d`, `0x681c44`, `0x681c69`, `0x681ce6`
Confidence: verified
Proof: one
Test: `terrain::horizon_insert`, `terrain::horizon_non_finite_probes`

`0x685fe0` stores 0xc9742400 into 0x140 dwords. `0x681b79` (`test ah,5; jnp`) leaves only below
−0.9 and `0x681b90` (`test ah,0x41; je`) only above 0.9, so equality and a NaN stay; the box test
repeats both at `0x6861a6` and `0x6861bd`. In `0x681b50`, `0x681c0d` divides 1.0 by z and
keeps the reciprocal on the stack for both stored products. `0x681c44` compares z with
2.7777777 (0x4031c71c), `test ah,5; jnp` skipping a nearer edge. `0x681c69` keeps the smaller
y/z, a tie taking the second. Columns are 64 (`0x810170`) times x/z, stored, less 0.5
(`0x86861c`), `fistp`, plus 0xa0. `0x681ce6` raises a column below the value. `0x6c15d0` picks
the rows by the chunk's level (`lod`).

## Horizon box test

`0x686180(box, flags)` returns 2 when the box is hidden behind the horizon and 0 otherwise. Its
eight corners go through the view matrix; unless flags bit 3 is set, a corner with z below 50
makes the box visible. The box spans the columns from round(64·min x/z − 0.5) + 160 to
round(64·max x/z − 0.5) + 161, one more than the occluders' rule, and is hidden when every
column in that span, clamped, is at least its highest y/z.

Addresses: `0x686180`, `0x6861a6`, `0x6861bd`, `0x868628`, `0x80fed4`, `0x68623f`, `0x686250`, `0x6862d4`, `0x6862f1`, `0x686316`
Confidence: verified
Proof: one
Test: `terrain::horizon_box_test`, `terrain::horizon_non_finite_probes`

The corner tables `0x868628`, `0x868648` and `0x868668` pick min or max per axis. `0x68623f`
compares z with 50 (0x42480000); `0x686250` divides 1.0 by z and stores x/z and y/z. The span
uses `lea eax,[ecx+0xa0]` (`0x6862d4`) and `lea ecx,[edx+0xa1]` (`0x6862f1`); a span starting
at 320 or ending below 0 is visible. `0x686316` makes the box visible at the first column below
its highest y/z; a NaN does not.

## Chunk visibility

`0x683bf0` walks the frame's candidate chunks: a chunk whose holes word is 0xffff, whose box fails
the frustum, or which the horizon hides, is dropped. With world flag 0x20, a visible chunk
without holes becomes an occluder candidate; the candidates whose view depth lies in
[`0xc7b66c`, `0xc7d278`) add their rows to the horizon, and all join the draw list. Without the
flag every visible chunk joins the draw list directly.

Addresses: `0x683bf0`, `0x682f40`, `0xc7cad8`, `0xc7cb50`, `0xc7b6f8`, `0x683c28`, `0x683c3c`, `0x683c4d`, `0x683cb1`, `0x683d53`, `0x683d67`
Confidence: verified
Proof: one
Test: none

`0x683c28` compares MCNK +0x3c with 0xffff. `0x683c3c` tests the chunk's box copy +0x94 through
`0x682f40`, nonzero when culled, and `0x683c4d` calls the horizon test with flags 0. A visible
chunk counts in `0xc7b6f8` and leaves its list link +0xd4. The candidate list is `0xc7cb50`; for
each, `0x683d53` needs +0x78 below `0xc7d278` and `0x683d67` not below `0xc7b66c` before
`0x6c15d0`, and the draw list is `0xc7cad8`. The occluders are added before their own draw.

## Chunk level and idle release

Each frame, for every chunk of a loaded area, `0x6afad0` stores the view depth of the chunk's
sphere, (p.y·c.y + p.z·c.z) + p.x·c.x + p.w − r with p the camera's view plane, sets the chunk's
level to `maxLOD` (3), or to 2 when world flag 4 is set and the depth is above `lodDist` (77),
and advances its idle time by the frame time. After 5 seconds it releases the chunk's vertex and
index buffers; its detail doodads get `0x6b28e0` after 1 second when `0x6b2900` allows and are
freed after 5; and each liquid slot releases its buffers after 5 seconds of its own.

Addresses: `0x6afad0`, `0xc7bcb0`, `0x867960`, `0x867964`, `0x867968`, `0x6afb03`, `0x6afb1d`, `0x6afb42`, `0x6afb6b`, `0x6afbb4`, `0x6afc0c`, `0x6b2900`, `0x6b28e0`, `0x6b2030`
Confidence: verified
Proof: one
Test: `terrain::plane_signed_dist_difftest`, `terrain::plane_lod_gate_ah_difftest`, `terrain::timer_accumulate_difftest`, `terrain::center_accumulate_difftest`, `terrain::center_half_difftest`

The plane is `0xc7bcb0` to `0xc7bcbc`, the sum stored at +0x78 (`0x6afb03`). The level +0xb8
loads `0x867964` and, when `0x6afb1d` finds the depth above `0x867960`, `0x867968`; the three
start at 3, 77.0 and 2 and are set by the console. The idle time +0xac adds `s_worldFrameSeconds`
with `fst` and is compared with 5.0 (`0x6afb42`) before `0x6afb6b` releases +0x14c and +0x150.
The detail doodads +0xc0 are released by `0x6afbb4`. Liquid +0x30 is the slot's own timer;
`0x6afc0c` releases it. The drawing pass resets the timers (`render`).

`terrain::plane_signed_dist_difftest` runs `0x6afad6` to `0x6afb06`;
`terrain::plane_lod_gate_ah_difftest` runs `0x6afad6` to `0x6afb28`;
`terrain::timer_accumulate_difftest` runs `0x6afb36` to `0x6afb48` and `0x6afbdd` to `0x6afbf7`;
`terrain::center_accumulate_difftest` runs `0x6afc7c` to `0x6afca6`; `terrain::center_half_difftest`
runs `0x6afd3d` to `0x6afd72`.

## Frame frustum box

Once a frame the scene captures the box around its frustum: the minimum and maximum corner of the eight
frustum corners, six floats at `0xc7cb5c` (min x, y, z, then max x, y, z). The chunk step, the map object
update and the liquid slots admit only what meets it.

Addresses: `0x680bc0`, `0x680e3c`, `0x680e4b`, `0x680e60`, `0x7c1450`, `0x5c43b0`, `0xc7bcd8`, `0xc7cb5c`, `0x6992c0`
Confidence: verified
Proof: one
Test: none

`0x680e3c` fills the corners at `0xc7bcd8`; `0x680e4b` calls `0x7c1450` with them and the count 8
pushed at `0x680e41`, and `0x680e60` copies the result with `rep movsd`. `0x7c1450` returns six zeros for a
count of 0, else starts from the first point and folds the rest, the minimum per axis kept with
`fcompp` and the maximum through `0x6992c0`.

## Chunk points

After the level and idle step, `0x6afad0` files what a chunk holds, if the chunk's box meets the
frame box (`0xc7cb5c`). The chunk goes to its band by one of its corner vertices: the index 0, 8, 0x88 or
0x90 of the outer grid, chosen by two bits that tell which way the camera target lies from the eye. A
detail doodad def and each liquid slot whose box meets the frame box are filed by their own sphere or
box centre.

Addresses: `0x6afad0`, `0x6afc1b`, `0x6afc2f`, `0x6afc43`, `0x6afc5f`, `0x6afc6c`, `0x6afca6`, `0x6afccc`, `0x6afce7`, `0x6afd38`, `0x6afd72`, `0x6afdda`, `0x699330`, `0x68df40`, `0x86b580`, `0xc7f294`, `0xc7d118`, `0xc7cf20`, `0x681026`, `0x68102c`, `0x681040`, `0x681059`
Confidence: verified
Proof: one
Test: none

`0x6afc1b` to `0x6afc51` skip the chunk when its min exceeds the frame box's max on an axis (a NaN skips
too), and `0x699330` skips it when its max (+0x50) is below the frame box's min. `0xc7f294` is 2 when the
target's x is below the eye's and plus 1 when its y is (`0x681026` to `0x681059`); `0x86b580[q]` picks the
vertex, which `0x6afc7c` to `0x6afca3` offsets by the chunk origin (+0x6c to +0x74) before `0x6afca6`.
`0x6afccc` files a detail doodad def whose byte +0xc has bit 7 and whose +0x88 or +0x174 is not zero
(`0x6afce7`). For each of the four liquid slots, `0x68df40` fills the box (the chunk's with the slot's heights as z); the centre is half the sum, the same
box test applies (`0x6afd72` to `0x6afdc2`), and `0x6afdda` files it with the slot index.

## Depth buckets

The scene files what it draws into 32 depth bands of 33.33 yards, nearest first. The band is
round-to-nearest of (depth · 0.03 − 0.5), the depth the signed distance along the camera's horizontal
facing, less the radius for a sphere. A negative value is band 0, and a band of 32 or more is not filed at
all. Each band holds a list per kind, and a node leaves its old list before it joins the tail of the new one.

Addresses: `0x6816f0`, `0x6817c0`, `0x6818b0`, `0x681970`, `0x681a40`, `0x6afca6`, `0x6afdda`, `0xc7bd40`, `0xc7cfb8`, `0x810174`, `0x86861c`, `0x67f100`, `0x67f560`, `0x6818f6`, `0x681901`
Confidence: verified
Proof: one
Test: `terrain::depth_bucket_key_difftest`, `terrain::depth_bucket_v1_difftest`, `terrain::depth_bucket_v3_difftest`, `terrain::depth_bucket_v4_difftest`, `terrain::view_plane_bucket_difftest`

`0x67f100` makes 32 records of 0x6c bytes at `0xc7bd40`, each by `0x67f560`. `0x6818b0` files a chunk from
a point, `0x6816f0` a doodad def from its sphere, `0x6817c0` a map object def, `0x681970` a liquid slot from
its centre and `0x681a40` an entity; `0x6afca6` and `0x6afdda` are the chunk step's calls. The last
three lists sit at +0x24, +0x30 + 12 × slot and +0x60. The band plane `0xc7cfb8` is the forward
direction with z zeroed, renormalised when its squared length exceeds 0.0001; d is −n·eye. The scale
`0x810174` is 0.03 and `0x86861c` 0.5. `0x6818f6` and `0x681901` clamp and skip. `0x6817c0` and
`0x681a40` also store the depth less the radius on the view plane `0xc7bcb0` at +0x78.

`terrain::depth_bucket_key_difftest` runs `0x6818b0` to `0x6818f3`;
`terrain::depth_bucket_v1_difftest` runs `0x6816f0` to `0x681754`;
`terrain::depth_bucket_v3_difftest` runs `0x681970` to `0x6819b4`;
`terrain::depth_bucket_v4_difftest` runs `0x681a40` to `0x681ab2`;
`terrain::view_plane_bucket_difftest` runs `0x6817c0` to `0x68183e`.

## Scene frame

`0x681070` draws the world each frame: it sets the fog distances from the far clip, captures the
camera planes and frustum (`0x680bc0`), walks the scene (`0x682fa0`), and runs the passes:
far-band objects, terrain chunks, map objects, liquids, weather and the effects.

Addresses: `0x681070`, `0x680bc0`, `0x682fa0`, `0x6841a0`, `0x684510`, `0x684840`, `0x6816d0`, `0x6813d0`
Confidence: inferred
Proof: none
Test: `terrain::viewport_gate_difftest`, `terrain::chunk_cam_relative_delta_difftest`, `terrain::wmo_pass_negate_difftest`, `terrain::far_fsub1_difftest`, `terrain::far_fsub2_difftest`, `terrain::far_pass_negate_difftest`, `terrain::scene_dir_subtract_difftest`, `terrain::scene_dir_normalize_difftest`, `terrain::scene_dot_negate_difftest`, `terrain::scene_plane_gate_difftest`, `terrain::scene_plane_normalize_difftest`, `terrain::scene_corner_negate_difftest`, `terrain::render_near_bias_difftest`, `terrain::render_fog_dist_difftest`, `terrain::render_leg_fog_offset_difftest`, `terrain::render_pos_accumulate_difftest`, `terrain::cam_relative_delta_difftest`

The order is read from the driver's calls; `0x684510` is the terrain chunk pass, `0x6816d0` the
liquid phase (ocean, river, map object liquid, then the ripples) and `0x6813d0` the stage set-up
that biases texture level of detail by +0.25. The passes' state is not read here.

`terrain::viewport_gate_difftest` runs `0x682fa0` to `0x682fb0` and `0x682fb5` to `0x682fc3`;
`terrain::chunk_cam_relative_delta_difftest` runs `0x684690` to `0x6846bb`;
`terrain::wmo_pass_negate_difftest` runs `0x6849ff` to `0x684a4e`; `terrain::far_fsub1_difftest`
runs `0x684253` to `0x684287`; `terrain::far_fsub2_difftest` runs `0x68434d` to `0x684367`;
`terrain::far_pass_negate_difftest` runs `0x684373` to `0x6843d1`;
`terrain::scene_dir_subtract_difftest` runs `0x680c01` to `0x680c22`;
`terrain::scene_dir_normalize_difftest` runs `0x680c27` to `0x680cb1`;
`terrain::scene_dot_negate_difftest` runs `0x680cb1` to `0x680cbe` and `0x680d2e` to `0x680d4a`;
`terrain::scene_plane_gate_difftest` runs `0x680cc5` to `0x680cf3`;
`terrain::scene_plane_normalize_difftest` runs `0x680cc5` to `0x680d19`;
`terrain::scene_corner_negate_difftest` runs `0x680d7c` to `0x680da5` and `0x680ee3` to `0x680f0b`;
`terrain::render_near_bias_difftest` runs `0x6810d8` to `0x6810ea`;
`terrain::render_fog_dist_difftest` runs a copy of `0x6810f5` to `0x68111a`;
`terrain::render_leg_fog_offset_difftest` runs `0x6811a0` to `0x6811af`;
`terrain::render_pos_accumulate_difftest` runs `0x681310` to `0x68131c`;
`terrain::cam_relative_delta_difftest` runs `0x68151b` to `0x681546`.
