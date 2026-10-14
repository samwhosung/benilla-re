# terrain: map object visibility

Which groups of which map objects the frame draws: the frame start that ages the map objects, the
two seeds of the flood (camera inside a map object, camera outside), the walk through the portals,
and the projection and clipping of one portal. The code is the map object file from `0x6b3880` to
`0x6b4d00`, with the interior entry `0x685a90` and the guard planes of `0x680610`. A map object is
an instance (matrices at `+0x94` and `+0xd4`, the model at `+0x118`); the model (CMapObj) holds the
groups. A screen rectangle is four floats: low y, low x, high y, high x.

Roots: `0x6b3890`, `0x685a90`, `0x6b3b20`, `0x6b3dd0`, `0x6b4160`, `0x6b41c0`, `0x6b46f0`

## Map object frame

`0x6b3890` runs once a frame before anything is drawn. It points the group drawers at the exterior
batch draw and the interior batch draw (a world flag swaps in a debug draw), zeroes the portal
window count, the weather visibility byte and the sky box state, and ages every cached map object
model: the night colours of its materials, the buffer blocks of its groups, the keep-alive timers of
its groups and of the model, freeing what has expired.

Addresses: `0x6b3890`, `0x6b38bb`, `0x6b38c1`, `0x6b38c8`, `0x6b38dc`, `0x6b38f3`, `0x6b395c`, `0x6b398d`, `0x6b39af`, `0x6b39eb`, `0x6b3a14`, `0x6b3a72`, `0x6b3af8`, `0x6b4090`, `0x6b40d7`, `0x6b40e9`, `0x6b6b60`, `0x6983bd`, `0x6d48b0`, `0x6a0330`, `0x6b98c0`, `0x6ba410`, `0xca8044`, `0xca803c`, `0xcbe580`, `0xcbe4b4`, `0xc62510`, `0xc7b2a4`, `0xce26ac`, `0xce2660`
Confidence: verified
Proof: one
Test: none

`0x6b3896` and `0x6b38a1` store the exterior drawer `0x6b4f10` and the interior drawer `0x6b5190`.
World flag 0x80000 replaces both with `0x6b5ba0` (`0x6b38dc`); otherwise a clear 0x800 puts
`0x6b57a0` in the interior slot and, with 0x800 set, a clear 0x200 puts `0x6b58b0` there
(`0x6b38f3` to `0x6b3904`). The loop walks the cache list `0xca8044` by the link offset
`0xca803c`. `0x6b4090(model)` copies each material colour `+0x10` to `+0x14` for materials with
flag 0x10, each of the three channels becoming (c × k) >> 8 with k = round(night × 255 − 0.5) and
night the light state's `+0x1ac` (`0x6b40d7` to `0x6b40ef`); the alpha byte becomes 0. A group's two
buffer blocks lose the frame time `0xc62510` and return to their pools at age below 0
(`0x6b398d`, `0x6b39af`); a group whose timer `+0x15c` is 0 or less and which holds file data is
freed (`0x6b3a14`). A model with `+0x1c8` 0 whose timer `+0x1cc` is 0 or less is freed
(`0x6b3a72`, `0x6b3af8`).

## Camera inside a map object

`0x685a90` prepares the flood for the map object that holds the camera. It moves the camera
position and the camera target into the model's space with the instance matrix `+0xd4`, stores the
unit direction between them and the offset of the plane through the camera (the plane of the
view axis), loads matrix slot 8 with the translation by minus the camera times the instance matrix
`+0x94`, registers the group callback `0x685d70`, and runs `0x6b3b20` with the model, the
instance matrix and the groups the camera is in.

Addresses: `0x685a90`, `0x685c27`, `0x685c4e`, `0x685c61`, `0x685cad`, `0x685cba`, `0x685cfe`, `0x685d03`, `0x685d12`, `0x685d1f`, `0x685d25`, `0x6b3880`, `0x685d70`, `0x6b3b20`, `0x602630`, `0xca7edc`, `0xca7ef0`, `0xca7efc`, `0xca8018`, `0xca802c`, `0xcde5b0`, `0xcde5b4`, `0xc7cf20`, `0xc7d118`, `0x80c4f4`
Confidence: verified
Proof: one
Test: `terrain::corner_ray_negate_difftest`, `terrain::dir_len2_gate_difftest`, `terrain::dir_normalize_difftest`, `terrain::plane_offset_negate_difftest`

`ecx` is the instance and `edx` the group list (`+0x4` count, `+0x8` indices). `0x685c27` and
`0x685c4e` transform `0xc7cf20` and `0xc7d118` by `+0xd4` into `0xca7edc` and `0xca8018`.
The difference is normalized only when its squared length exceeds 0.0001 (`0x685cad`, `fcom`
then `test ah,0x41; jne`); `0x685cfe` takes the dot product with the position and `0x685d03`
negates it into `0xca7efc`. `0x6b3880` stores the callback in `0xcde5b0` and the instance in
`0xcde5b4`. The matrix slot call is `0x58b240` with 8.

## Interior seed

`0x6b3b20` floods from the groups the camera is in. It advances the pass number `0xca8014` and
sets the inside flag `0xcde5ac`, the group fog flag `0xca7f00` and clears the window count. The
object-to-clip matrix `0xcbe2d8` is slot 8 times the device matrix. It walks from each start group
over the whole screen, (−1, −1, 1, 1), at depth 0. Then each recorded window is walked again from
every exterior group inside it, and last the callback runs for each group with flag 0x10000 whose
box the current volume shows.

Addresses: `0x6b3b20`, `0x6b3b2c`, `0x6b3b47`, `0x6b3b4c`, `0x6b3b51`, `0x6b3b57`, `0x6b3b7b`, `0x6b3ba3`, `0x6b3bab`, `0x6b3bb6`, `0x6b3bdf`, `0x6b3c05`, `0x6b3c12`, `0x6b3c1a`, `0x6b3c73`, `0x6b3c87`, `0x6b3c95`, `0x6b3ca4`, `0x6b3d11`, `0x6b3d22`, `0x6b3d39`, `0x6b3d4f`, `0x6b3d8b`, `0x6b3d9d`, `0x6b3da9`, `0x682380`, `0x682f90`, `0x682f60`, `0x682930`, `0x5f8d20`, `0x58b0c0`, `0x58b280`, `0x7bc6a0`, `0xca8014`, `0xcde5ac`, `0xca7f00`, `0xcbe320`, `0xcbe070`, `0xcbe468`, `0xcbe2d8`, `0xce66e8`, `0xc7bcd8`
Confidence: verified
Proof: one
Test: `terrain::camera_inside_bias_difftest`

`0x6b3b57` and `0x6b3b66` read the device matrix to `0xcbe070` and slot 8 to `0xcbe468`; the
product goes to `0xcbe2d8`. The start group is given to the walk as the camera's interior group
when its flags lack 0x48 (`0x6b3be8`); the walk receives the group, 0xffff, the rectangle, depth 0
and that group. `0x6b3c1a` clears the inside flag, so the second phase records no windows. A window
record (`0xcbe324`, 5 floats) maps to the volume rectangle through `0x682930` (`0x6b3ca4`) and
to (2r − 1) for the walk (`0x6b3ca9` to `0x6b3cf4`); a group with flag 0x8 and without 0x10000
whose oriented box `0x682f60` the volume does not cull (`0x6b3d22`) is walked at depth 0 with no
interior group (`0x6b3d39`). `0x6b3c87` skips the phase for an empty list. The last loop
(`0x6b3d8b` to `0x6b3da9`) calls `0x6b4160` for flag 0x10000 groups the current volume does not
cull.

## Exterior seed

`0x6b3dd0` seeds the flood from outside, for a map object seen through a rectangle of the screen
in [0, 1] (the object's instance matrix and the rectangle are the arguments). It resets the pass
state like the interior seed with the inside flag 0, maps the rectangle to [−1, 1], and builds the
volume for it. For each group with flag 0x10000 whose box is not culled it runs the group callback;
for each other group with flag 0x8 whose box is not culled it walks the portals from that group at
depth 0.

Addresses: `0x6b3dd0`, `0x6b3df0`, `0x6b3dff`, `0x6b3e04`, `0x6b3e13`, `0x6b3e28`, `0x6b3e7c`, `0x6b3e9b`, `0x6b3eea`, `0x6b3ef6`, `0x6b3f1b`, `0x6b3f2d`, `0x6b3f39`, `0x6b3f40`, `0x6b3f4f`, `0x6b3f64`, `0x6b3f79`, `0x6856c0`, `0x685a3c`, `0x682930`, `0x682f60`, `0x6b4160`
Confidence: verified
Proof: one
Test: `terrain::camera_outside_bias_difftest`

The window is the second argument; `0x6b3e9b` to `0x6b3ee7` form 2r − 1 of its four floats, and
`0x6b3e7c` builds the three-by-three rotation of the instance matrix with the position from `+0x30`.
`0x6b3f1b` tests flag 0x10000 and calls `0x6b4160` for an unculled box (`0x6b3f39`); `0x6b3f40`
tests 0x8 and `0x6b3f64` walks. It is called by the exterior map object pass `0x6856c0` (`0x685a3c`).

## Group callback

`0x6b4160(group)` runs the registered callback `0xcde5b0` with the group index, the registered
instance and a flag, for a loaded group. A group with flag 0x1000 (liquid) that comes first sets
the liquid group switch `0xce66e8` and the parity `0xce269c` to 0, and the flag passed is 1 when
that parity is 0.

Addresses: `0x6b4160`, `0x6b416a`, `0x6b4176`, `0x6b417b`, `0x6b4184`, `0x6b418e`, `0x6b4198`, `0x6b41b4`, `0x6a4a00`, `0xcde5b0`, `0xcde5b4`, `0xce66e8`, `0xce269c`
Confidence: verified
Proof: one
Test: none

`0x6b4176` tests bit 12 of the group flags, `0x6b417b` the switch. The call at `0x6b41b7` is
thiscall with the group index in `ecx` and the instance in `edx`, and pushes `sete cl` of the
parity.

## Portal walk

`0x6b41c0(group, from, rectangle, depth, interior group)` visits a group and recurses through its
portals. It stops at depth above the cap `0xcb004c` (10), for an unloaded group and for flag
0x10000. A flag 0x1000 group, when none was seen, stores the depth parity. It sets the weather
byte for flags 0x148 and the sky box for 0x40000 with a set byte at `+0x12c`, calls the callback,
and for each portal reference not leading back to `from` projects the portal once a pass; a portal
the eye is behind or whose screen rectangle misses the rectangle or is narrower than 0.001 is
skipped, the rest recurse with the rectangle clipped to the window.

Addresses: `0x6b41c0`, `0x6b41d0`, `0x6b4200`, `0x6b4215`, `0x6b4220`, `0x6b4225`, `0x6b4241`, `0x6b425f`, `0x6b4277`, `0x6b42b8`, `0x6b42c6`, `0x6b42d0`, `0x6b42d9`, `0x6b42e0`, `0x6b42f4`, `0x6b4306`, `0x6b432c`, `0x6b4385`, `0x6b4391`, `0x6b43a0`, `0x6b43ad`, `0x6b43c1`, `0x6b43cd`, `0x6b43f1`, `0x6b43f8`, `0x6b4409`, `0x6b444f`, `0x6b44b1`, `0x6b44ca`, `0x6b4502`, `0x6b4521`, `0x6b4541`, `0x6b45a6`, `0x6b4610`, `0x6b4637`, `0x6b4643`, `0x6b466d`, `0x6b4680`, `0x6b46f0`, `0x682380`, `0x682930`, `0x682f90`, `0x6d48b0`, `0xcb004c`, `0xcb005c`, `0xca8014`, `0xca7f00`, `0xca80c4`, `0xca8080`, `0xcde5ac`, `0x801360`
Confidence: verified
Proof: one
Test: `terrain::portal_plane_dot_difftest`, `terrain::portal_rect_extent_difftest`, `terrain::portal_ndc_to_screen_difftest`

A portal reference is 8 bytes: portal index, other group, a signed side (`0x13c` table from the
group's `+0x2c`, count `+0x30`); the portal record is 20 bytes: first vertex, vertex count, normal
and offset. The state of a portal is 0x1c bytes at `0xcb005c` (flags word, the rectangle at `+0x4`
to `+0x10`, the pass of its projection at `+0x14` and of its window at `+0x18`). `0x6b4277` to
`0x6b42b8` set the fog flag `0xca7f00` to 1 when the light state's claim (`+0x98` a model, `+0xa0`
group indices) names the interior group, else 0. The eye test (`0x6b43cd` to `0x6b4403`) is
dot(eye, n) + d, negated for a negative side; below 0 the portal is skipped. The window clamps the
portal rectangle by `0x6b4409` to `0x6b44b1`. A portal whose other group has flag 0x8, met in the inside leg, records a window of (v + 1) × 0.5 and the reach
`0x6b4680` (`0x6b4521` to `0x6b45ab`) once a pass: the portal state's `+0x18` is compared with the pass stamp
(`0x6b4502`) and stored after (`0x6b45b7`), once per portal, not per group, with no bound on 16 records. The recursion pushes a volume from `0x682930`, walks with depth + 1 and pops (`0x6b4610` to `0x6b4643`).

## Portal reach

`0x6b4680(portal)` returns the largest distance of the portal's vertices to the plane through the
camera, dot(v, direction) + offset, never below 0. It is the fifth word of a window and sets the
horizon feed of the scene walk through the window.

Addresses: `0x6b4680`, `0x6b468c`, `0x6b46a8`, `0x6b46c6`, `0x6b46cf`, `0xca7ef0`, `0xca7efc`
Confidence: verified
Proof: one
Test: `terrain::portal_plane_maxdist_difftest`

The result starts at 0 (`fld 0.0` at `0x6b4686`); `0x6b46cf` is `fcomp st(1); test ah,0x41; jne`,
which replaces the maximum only for a strictly larger distance.

## Portal projection

`0x6b46f0(portal, state)` stores in the state the portal's screen rectangle and flags. Each vertex
goes through the object-to-clip matrix `0xcbe2d8`, the polygon (x, y, w) is clipped, and nothing
left sets flag 1 (the portal is dropped). When the camera lies within 0.01 of the portal's plane
inside its polygon, the rectangle is the whole screen (−1, −1, 1, 1). Otherwise it is the bounds of
x/w and y/w over the clipped vertices, with |w| below 0.001 replaced by 1e-5.

Addresses: `0x6b46f0`, `0x6b46f6`, `0x6b4733`, `0x6b47a7`, `0x6b47f1`, `0x6b47fd`, `0x6b4883`, `0x6b4923`, `0x6b4925`, `0x6b4935`, `0x6b494a`, `0x6b4953`, `0x6b4992`, `0x6b49c6`, `0x6b49d3`, `0x6b49e0`, `0x6b4a80`, `0x7bcb40`, `0x7bf680`, `0x7c23e0`, `0xcbe4c0`, `0xcbe2d8`, `0x8029d0`, `0x801360`
Confidence: verified
Proof: one
Test: `terrain::portal_screen_rect_difftest`

The polygon buffer `0xcbe4c0` holds 12 vertices of 16 bytes; a portal with more overruns it. The
state word is cleared at `0x6b473d`. The plane is the normalized cross product (v1 − v0) × (v2 − v0)
with `rsqrtss` when `0xc9e388` is set (`0x6b4883`) and `fsqrt` otherwise (`0x6b48aa`); the distance
|n·eye + d| is compared at `0x6b4925` (`fcomp 0.01; test ah,0x41; jp`), so the eye is on the plane
at or below 0.01 and a NaN is not. After the divide by w (`0x6b49e0`) the state keeps the
minimum and maximum of the second component, y, at `+0x4` and `+0xc` and of the first, x, at `+0x8`
and `+0x10` (`0x6b49fd` to `0x6b4a3c`). The third argument is not read. The walk tests flag bit 1
(`0x6b43c9`), which nothing sets.

## Portal polygon clip

`0x6b4a80(polygon, count, out pointer, out count)` clips a polygon of (x, y, w) triples against the
four guard planes in turn and returns 0 vertices when one removes everything. The planes
(1, 0, 1), (−1, 0, 1), (0, 1, 1) and (0, −1, 1) over √2 hold the points |x| and |y| at most w.
A vertex is inside when its distance exceeds 0.01, on the plane within ±0.01 and outside below
−0.01; an inside vertex followed by an outside one, or the reverse, adds the intersection.

Addresses: `0x6b4a80`, `0x6b4aa0`, `0x6b4b0b`, `0x6b4b65`, `0x6b4b72`, `0x6b4b95`, `0x6b4be5`, `0x6b4c0c`, `0x6b4c4e`, `0x6b4c60`, `0x6b4cb3`, `0x6b4ce1`, `0x680610`, `0x680720`, `0xc7d0d8`, `0xcbe0b0`, `0xcbe0f0`, `0xcbe138`, `0xcbe1f8`, `0x8029d0`, `0x810ccc`, `0x802940`, `0x8030a0`
Confidence: verified
Proof: one
Test: none

`0x680610` writes the planes once: each is its normal scaled by 1/√2 (`0x680720`, with `0x8030a0`
= −1). The loop at `0x6b4b0b` takes the planes from `0xc7d0dc` in steps of 0x10. Two buffers of 16
vertices (`0xcbe138`, `0xcbe1f8`) alternate. The class of a vertex is 1 above `0x8029d0` (0.01), 2
below `0x810ccc` (−0.01) and 0 between (`0x6b4b72`, `0x6b4b95`). A class 0 vertex is copied
(`0x6b4bf4`); a class 1 vertex is copied; an edge to the next vertex of the other nonzero class adds
the point at d[i] / (d[i] − d[i+1]) along it, after the inside vertex or in place of the outside one
(`0x6b4c4e` to `0x6b4c8a`). The vertex after the last is the first.
