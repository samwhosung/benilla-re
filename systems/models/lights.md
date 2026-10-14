# models: lights

The model lights and the lighting record each instance draws with: a light's type, colours and
position, its link into the scene, how a record keeps the four nearest point lights and sums the
rest into ambient and spherical harmonic terms, the fog it carries, and how it reaches the device.

The code is the object file after M2Model.cpp, from `0x71b090` through `0x71ccf0`; it also holds
the bone and track state constructors the instance uses. A light is a 0x6c-byte record that model
lights, the world and the interface's model frames all use. A lighting record is 0x1b4 bytes; an
instance builds one each frame (`0x7192b0`) unless it borrows its parent's.

Roots: `0x71b4a0`, `0x71b610`, `0x71b620`, `0x71b650`, `0x71b6a0`, `0x71b780`, `0x71b7c0`, `0x71bc30`, `0x71bc70`, `0x71bce0`, `0x71bf90`, `0x71c110`, `0x71c730`

## Light object

`0x71b4a0` initializes a light: type 1 (point), zero position, direction and colours, attenuation
0.7 at `+0x58` and 0.03 at `+0x5c`, disabled and unlinked. A light belongs to a scene (`+0x0`,
with the scene's frame minus one at `+0x4`); setting its type or position relinks an enabled
point light, and enabling links it.

| offset | holds |
|---|---|
| `+0x008` | type: 1 point, other values directional |
| `+0x00c` | position |
| `+0x024` | direction, normalized unless shorter than 2.384186e-7 |
| `+0x030`, `+0x03c`, `+0x048` | ambient, diffuse and third colour |
| `+0x060` | enabled |
| `+0x064`, `+0x068` | link: address of the previous link, next light |

Addresses: `0x71b4a0`, `0x71b5f0`, `0x71b620`, `0x71b650`, `0x71b6a0`, `0x71b780`, `0x71b610`, `0x71bb30`, `0x71bb60`
Confidence: verified
Proof: one
Test: `models::grid_scale_product_difftest`

`0x71b4a0` stores 1 at `+0x8` and the bits 0x3f333333 and 0x3cf5c28f at `+0x58` and `+0x5c`.
`0x71b6a0` copies the direction, computes `sqrt((x·x + y·y) + z·z)` and multiplies each component
by its reciprocal unless the length's magnitude is under 2.384186e-7 (`test ah,5; jnp`, so a NaN
length scales). `0x71b780` links through
`0x71bb60` when enabling and unlinks through `0x71bb30` when disabling, only for a light with a scene.

Of `0x71bb60`, `grid_scale_product_difftest` runs `0x71bbb3`–`0x71bbdd`.

## Scene light grid

An enabled point light links into one of the scene's 64×64 grid cells, `floor(x·s) & 63` by
`floor(y·s) & 63` with the scene scale `s`; any other light links into the scene's global list.
The grid is allocated on the first point light. Adding a point's lights to a record takes every
global light and every light of the cells its radius covers, and disables a grid light whose
frame stamp is stale instead of adding it.

Addresses: `0x71bb60`, `0x70a0f0`, `0xceffec`, `0xcf04d0`, `0x71bf90`
Confidence: verified
Proof: one
Test: `models::attach_light_grid_window_difftest`, `models::m2_light_aggregate_dist_difftest`

`0x71bb60` allocates 0x4000 bytes at scene `+0x1c` with line 0xf0, multiplies x and y by
`0xcf04d0` (`0x71bbb3`, `0x71bbcc`), the same 0.5/10.0 that `0x71ccf0` stores and `0x70dc40`
stores in the `0xceffec` that `0x70a0f0` reads, floors them with `0x73ff3f`, truncates with `__ftol` and indexes `cell = 64·yi + xi`.
`0x70a0f0` adds the list at scene `+0x18`, then covers `floor((p − r)·s − 0.5) & 63` through
`floor((p + r)·s + 0.5) & 63` on both axes; a light whose `+0x4` differs from the scene frame
`+0x10` is passed to `0x71b780(light, 0)`.

Of `0x70a0f0`, `attach_light_grid_window_difftest` runs `0x70a122`–`0x70a1aa`; of `0x71bf90`,
`m2_light_aggregate_dist_difftest` runs `0x71bfb0`–`0x71bff5`.

## Lighting record

`0x71bc30(scene, position)` zeroes a record and stores its scene and position. The record holds
up to four point lights with their squared distances, the summed ambient and directional colours,
27 spherical harmonic terms (nine per channel), the fog and the clip plane.

| offset | holds |
|---|---|
| `+0x004` | position |
| `+0x014` | flags: 1 lights in view space, 2 dominant light derived, 4 constants packed |
| `+0x018`, `+0x024`, `+0x030` | directional sums `colour·direction` for red, green, blue |
| `+0x03c`, `+0x048` | luminance-weighted direction sum, directional colour sum |
| `+0x054` | ambient sum |
| `+0x060`, `+0x078` | dominant light colour and direction |
| `+0x06c` | third colour sum |
| `+0x084`, `+0x0a8`, `+0x0cc` | spherical harmonic terms for red, green, blue |
| `+0x0f0` | seven packed constant vectors |
| `+0x160`, `+0x170`, `+0x180` | nearest point lights, their squared distances, their count |
| `+0x184` to `+0x190` | fog start, fog end, `1/(end − start)`, fog colour |
| `+0x19c`, `+0x1a0` | clip plane flag, plane |

Addresses: `0x71bc30`, `0x71c110`
Confidence: verified
Proof: one
Test: `models::fade_set_range_difftest`

`0x71bc30` clears 0x6d dwords, stores the scene at `+0x0` and four dwords of the position from
`+0x4`. `0x71c110(colour, start, end)` stores the start, the end, `1.0/(end − start)` as a float
and the three colour floats.

`fade_set_range_difftest` runs `0x71c110` whole.

## Adding a light

`LightingAddLight` ignores a disabled light. A point light competes for the four nearest slots by
`(dz² + dy²) + dx²` from the record's position, kept as a max-heap: below four it is inserted, at
four it replaces the farthest when it is strictly nearer. The comparisons use the unrounded sum and
the slot stores it as a float. A NaN distance sifts up past every parent, is dropped when the heap
is full, and on a sift down moves below every child. Any other light adds its ambient colour, its
diffuse colour along its direction, and its third colour into the record's sums.

Addresses: `0x71bf90`, `0x71bfdf`, `0x71c009`, `0x71c04f`, `0x71c08e`, `0x71bc70`, `0x71bce0`, `0x71bf60`
Confidence: verified
Proof: one
Test: `models::m2_light_aggregate_dist_difftest`

`0x71bfdf` squares the z difference, adds the y square, then the x square, at the FPU's 53-bit
precision (control word 0x027F), and the sum stays on the stack until `fstp` into `+0x170`.
Insertion sifts up past the parent at `(i − 1) >> 1` unless `fcom` at `0x71c009` sets exactly one
of C0 and C3 (smaller or equal); replacement at `0x71c04f` needs C0 alone; the sift down takes the
first child when it is strictly larger, else the second, and moves it up while `fcom` at `0x71c08e`
sets C0 (smaller, or NaN). The other branch calls `0x71bc70(+0x30)`,
`0x71bce0(+0x3c, +0x24)` and `0x71bf60(+0x48)`.

Of `0x71bf90`, `m2_light_aggregate_dist_difftest` runs `0x71bfb0`–`0x71bff5`.

## Spherical harmonic terms

An ambient colour `c` adds `c` to the ambient sum and `3.5449078·c` (2√π) to each channel's
constant term. A directional colour `c` along `d` first takes `d` through the scene's view `+0x9c`
(row vector, 3×3 part) when the record has a scene, then adds `c_ch·d` to each channel's
directional sum, `(0.212671·r + 0.71516·g + 0.072169·b)·d` to the luminance-weighted sum, and `c`
to the directional colour sum. It then adds `2.956793·c_ch·Y_i` to the nine terms of each channel,
with `(x, y, z) = −d`:

| i | `Y_i` |
|---|---|
| 0 | 0.2820948 |
| 1, 2, 3 | −0.4886025·y, 0.4886025·z, −0.4886025·x |
| 4, 5 | 1.0925484·x·y, −1.0925484·y·z |
| 6 | 0.3153916·(3z² − 1) |
| 7, 8 | −1.0925484·x·z, 0.5462742·(x² − y²) |

A light's third colour adds to the third colour sum.

Addresses: `0x71bc70`, `0x71bce0`, `0x71bf60`, `0x81207c`, `0x812080`
Confidence: verified
Proof: one
Test: `models::m2_light_accumulate_difftest`, `models::m2_light_add_directional_difftest`, `models::m2light_accum_vec3_difftest`

`0x71bc70` multiplies by the float at `0x81207c`. `0x71bce0` builds `d·M` from `+0x9c`, `+0xac`,
`+0xbc` (x) and the next two columns, skipped when record `+0x0` is null; the weights are the floats
at `0x8120a0`, `0x8120a4`, `0x81209c`. It negates the three components (`0x71be79`), stores the
terms on the stack from `[ebp−0x3c]` = 0x3e906ebb, using the floats `0x812098`, `0x812094`,
`0x812090`, `0x81208c`, `0x812088`, `0x812084`, 3.0 and 1.0, and the
loop at `0x71bf20` adds `c·[0x812080]·Y_i` into `+0x84`, `+0xa8` and `+0xcc`. `0x71bf60` adds into
`+0x6c`.

`m2_light_accumulate_difftest` runs `0x71bc70` whole; `m2_light_add_directional_difftest` runs
`0x71bce0` whole; `m2light_accum_vec3_difftest` runs `0x71bf60` whole.

## Harmonic constants

The terms become seven vectors at `+0xf0`, built once per record (flag 4) and copied out. For each
channel with terms `L`:

| vector | x | y | z | w |
|---|---|---|---|---|
| A (0–2) | −0.325735·L3 | −0.325735·L1 | 0.325735·L2 | 0.2820948·L0 − 0.0788479·L6 |
| B (3–5) | 0.2731371·L4 | −0.2731371·L5 | 0.2365437·L6 | −0.2731371·L7 |
| C (6) | 0.1365686·L8 red | green | blue | 1.0 |

`M2DrawSetLighting` puts them in vertex constants c10–c16 or pixel constants c1–c7
(`render#lighting-lanes`).

Addresses: `0x71c4e0`, `0x8120a8`
Confidence: verified
Proof: one
Test: `models::sh_light_pack_difftest`

`0x71c4e0` skips the build when `+0x14` has bit 4 and copies 0x1c dwords from `+0xf0` with `rep
movs`. The weights are the floats at `0x8120c4` (−0.325735), `0x8120c0`, `0x8120bc`, `0x8120b8`,
`0x8120b4`, `0x8120b0` (−0.2731371), `0x8120ac` and `0x8120a8`; `+0x15c` gets 0x3f800000.

Of `0x71c4e0`, `sh_light_pack_difftest` runs `0x71c4ee`–`0x71c723`.

## Dominant light

Once per record (flag 2) the directional sums collapse into one light. Its direction `+0x78` is the
luminance-weighted sum normalized, or (0, 0, −1) when its squared length is at most 1e-5 or NaN.
With `c_ch` the dot product of that direction with each channel's directional sum and `S` the
directional colour sum, the light's colour `+0x60` is `1.25·c − 0.25·S` and `0.25·(S − c)` adds to
the ambient sum.

Addresses: `0x71c2f0`, `0x4549c0`, `0x80c5c8`, `0x8029b0`, `0x71c35e`
Confidence: verified
Proof: one
Test: `models::camera_facing_basis_difftest`, `spline::vec_div_scalar_diff`

`0x71c334` compares the squared length with 1e-5 and takes the fallback on C0 or C3 (`test ah,
0x41`); otherwise `0x4549c0` multiplies by `1/length` when the length is at least 2.384186e-7 (`jnp`
at `0x71c35e` skips on C0 alone); past the 1e-5 gate the length is above 0.003, so the skip is
unreachable. `c` reads `+0x18`, `+0x24`, `+0x30`. The scales are 1.25 and 0.25 at `0x8029b0`.

`camera_facing_basis_difftest` runs `0x4549c0` whole; `camera_facing_basis_difftest` runs `0x71c2f0`
whole.

`spline::vec_div_scalar_diff` runs `0x4549c0` whole.

## View-space lights

Once per record (flag 1), and only with a scene, each nearest point light's position `+0xc` goes
through the view `+0x9c` into the light's `+0x18`. A clip plane `(n, w)` becomes view-space: the
point `−w·n` goes through the view, `n` through its 3×3 part and is normalized unless shorter than
2.384186e-7, and `w` becomes `−(n′·p′)`.

Addresses: `0x71c160`, `0x7bca80`
Confidence: verified
Proof: one
Test: `models::m2_light_coord_finalize_difftest`

`0x71c160` returns at once when `+0x14` has bit 1 and sets the bit on every other path, also when
`+0x0` is null. The plane is at `+0x1a0` (normal) and `+0x1ac`, gated by `+0x19c`.

Of `0x71c160`, `m2_light_coord_finalize_difftest` runs `0x71c1d7`–`0x71c2e2`.

## Fixed-function lights

Without vertex shaders a record reaches the device as four lights. Slot 0 is the dominant light: a
directional light along `+0x78` with ambient `+0x54`, diffuse `+0x60` and third colour `+0x6c`.
Slots 1 to 3 take the nearest point lights from the end of the heap backward, so with four the
heap root, the farthest, is left out: diffuse from the light's `+0x3c`, no ambient or third
colour, attenuation `+0x54` to `+0x5c`, at `+0x18` (view space) or at `+0xc` minus the caller's
origin. Unused slots are disabled. A colour reaches the device as an ARGB word divided by its
largest component, alpha included, with that largest value as the light's scale.

Addresses: `0x71c730`, `0x71ca80`, `0x591260`, `0x589d80`, `0x589de0`, `0x71b7c0`
Confidence: verified
Proof: one
Test: `models::driver_clamp_pack_difftest`, `models::driver_mag_gate_difftest`, `models::driver_vec_diff_difftest`, `models::light_clamp_pack_difftest`, `models::m2_light_color_pack_difftest`

`0x71c730(origin, fog)` sets device light bit 1 for point lights; the colour calls pass alpha 0.0
(ambient, third) and 1.0 (diffuse). `0x71ca80` takes the maximum through `fcomp` with `test ah,5`,
writes colour 0 when it is under 1e-5, else clamps each scaled channel to 0 when at or under 0 and
to 255 when at or over 1, else `__ftol(v·255 + 0.5)`. With `fog` non-zero and
`|+0x18c| ≥ 2.384186e-7` it sets render states 0xd (colour `0xff000000` | the packed fog colour),
0xb (`+0x188`), 0xa (`+0x184`) and 0xf on; with `|+0x18c|` smaller, 0xf off. Models call it
with `(0, 0)`; the world draws pass the origin at `0xc7cf20` and fog on. `0x71b7c0`, which the map
object group draw calls, fills a device light from one light with its colours clamped and scaled by
255 without that division; it is read for its type-0 branch only.

`light_clamp_pack_difftest` runs `0x591260` whole; of `0x71b7c0`, `light_clamp_pack_difftest` runs
`0x71b7c0`–`0x71bb16`; of `0x71c730`, `driver_vec_diff_difftest` runs `0x71c820`–`0x71c846`,
`driver_mag_gate_difftest` `0x71c902`–`0x71c915`, `driver_clamp_pack_difftest`
`0x71c91b`–`0x71ca2a`; `m2_light_color_pack_difftest` runs `0x71ca80` whole.

## Model light tracks

Each model light has a 0x170-byte state at instance `+0x200`, its light object at `+0x104` and its
enabled word at `+0x100` (1 from the constructor, set by `SetLightEnabled`). Animation samples the
record's visibility byte track (`+0xb8`) into `+0xec`; when the light is disabled or invisible,
later updates skip it. Each colour product is recomputed only when its colour track is sampled:
ambient = ambient colour (`+0x10`) × ambient intensity (`+0x2c`) × instance alpha `+0x19c`, into
the light's `+0x30`; diffuse likewise from `+0x48` and `+0x64` into `+0x3c`. The attenuation
tracks `+0x80` and `+0x9c` are never sampled, so model lights keep 0, 0.7 and 0.03.

Addresses: `0x7163bf`, `0x71b370`, `0x713430`, `0x71b4a0`
Confidence: verified
Proof: one
Test: none

The loop at `0x7163e0` steps records by 0xd4 and states by 0x170. Each track is sampled when its
key count exceeds the first-update gate `+0x8c` (`0x71651a`, `0x716610`, `0x7167d7`, `0x7168f1`),
with the light's bone `+0x2` selecting the bone state for `0x713d50`, and blends toward the second
sequence by bone state `+0x10c` when the track has no global sequence. The ambient store at
`0x71678a` sits inside the ambient colour track's branch, the diffuse store at `0x716a67` inside
the diffuse colour track's; no instruction in the loop reads record `+0x80` or `+0x9c`.

## Model lights per frame

After animating, each model light is placed and enabled only when its state is enabled and
visible. A point light (type 1) sits at its record position `+0x4` through its bone's matrix and
the scene matrix `+0xdc`; any other light points along the negated third row of its bone matrix
taken through the 3×3 part of `+0xdc`. The light's frame stamp becomes the scene frame, so the
grid keeps it this frame.

Addresses: `0x718960`, `0x71b650`, `0x71b6a0`, `0x71b780`
Confidence: verified
Proof: one
Test: none

The loop at `0x7189e6` tests state `+0x100` and byte `+0xec`, reads the bone matrix at instance
`+0x94 + 64·bone`, calls `0x71b650` or `0x71b6a0`, then `0x71b780(enabled)` with 0 when either
gate fails, and stores scene `+0x10` into state `+0x108` (the light's `+0x4`).

## Instance lighting

An instance draws with its parent's record when it has a parent created without flag 1, else with
its own at `+0x204`. Building its own: the record starts at the world translation `+0x12c` through
the scene matrix, radius 0; it takes the scene's lights for that point, then the owner's callback
`+0x3bc(instance, record, +0x3c0)` when set, then derives the dominant light and moves the lights
to view space. When the instance has attachment states, its children with a resolved attachment
index update after it.

Addresses: `0x7192b0`, `0x7105d0`, `0x718960`, `0x71bc30`, `0x70a0f0`
Confidence: verified
Proof: one
Test: none

`0x718982` sets `+0x3b8` to the parent's `+0x3b8` when `+0x1cc` is set and its `+0x4` lacks bit 0,
else to `+0x204`. `0x7192b0` makes the same test, calls `0x71bc30(scene, position)`, `0x70a0f0`,
the callback with `ecx` the instance and `edx` the record, `0x71c2f0` and `0x71c160`, then, when
`+0x1c8` is set, walks `+0x1dc` through `+0x1e4` for children whose `+0x1d4` is not 0xffff.
