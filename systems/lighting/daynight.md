# lighting: day-night

The time-of-day light: the light state the world stamps every frame, the band evaluation that turns
a LightParams record into a light record at the time of day, the blend of the storm record and of
the area lights around the camera, and the table build that commits it all. Fog and the device
light take their values from the table (`fog`); the sky, the bodies and the clouds read it too.

The light state is a block of globals from `0xce9b60`; `0x6d48b0` returns its address, and other
systems write it through that pointer. The live light table is the 0x90-byte record at
`0xce9c0c`.

Roots: `0x6d48b0`, `0x6d2210`, `0x6d4530`, `0x6d45c0`, `0x6d4620`, `0x6d4630`, `0x6d4810`, `0x6d62e0`, `0x6d6ad0`

## Light state

The light state holds the clock, the camera, the weather and the outputs. These are the fields
other systems write or the table build reads; offsets are from `0xce9b60`.

| offset | holds | written by |
|---|---|---|
| `+0x00`, `+0x04`, `+0x08` | game clock word, time of day in [0, 1), day count as a float | the world frame |
| `+0x0c` | active player's position | the world frame |
| `+0x18`, `+0x24` | camera eye, unit camera forward | the world frame |
| `+0x30` | camera heading, radians in [0, 2π) | `0x6d4530` |
| `+0x34`, `+0x38`, `+0x3c` | far clip, clock in seconds, frame time | the world frame |
| `+0x40` | weather density | the world frame, the weather |
| `+0x44`, `+0x45` | screen tint level byte, tint colour | `0x6d45c0` |
| `+0x4c`, `+0x50` | override LightParams record, override light reference | `0x6d4620`, `0x6d4630` |
| `+0x6c` | storm weight | `0x6d4500` |
| `+0xac` | the light table, 0x90 bytes | `0x6d2260` |

Addresses: `0x6d48b0`, `0xce9b60`, `0x482ad0`, `0x482aed`, `0x482af0`, `0x482b0e`, `0x482b27`, `0x482b86`, `0x482bc9`, `0x482bdb`, `0x482be2`, `0x482bf6`, `0x482c02`, `0x6d1ba0`
Confidence: verified
Proof: one
Test: `ui::textglue_482_normalize3`, `ui::textglue_482_sq_len3_482200`, `ui::textglue_482_tick_scale`

`0x482ad0(dt)`, called once per rendered world frame, takes the state from `0x6d48b0` and stores
the frame time at `+0x3c` (`0x482aed`), the camera's `+0x3c` at `+0x34` (`0x482af0`), the
millisecond clock times 0.001 at `+0x38` (`0x482b0e`), the camera's `+0x8` at `+0x18`
(`0x482b27`), the camera's forward normalized at `+0x24` (`0x482b86`), the active player's
position at `+0xc` when there is one (`0x482bc9`), the game clock's word, time of day and day
count at `+0`, `+4` and `+8` (`0x482bdb`, `0x482be2`, `0x482bf6`), and the weather manager's
`+0x14` at `+0x40` (`0x482c02`). `0x6d1ba0` zeroes the block and sets `+0x50` to −1.

Bit-exact coverage: `ui::textglue_482_normalize3` covers `0x482b57` to `0x482b89` of `0x482ad0`; `ui::textglue_482_sq_len3_482200` covers `0x482b57` to `0x482b89` of `0x482ad0`; `ui::textglue_482_tick_scale` covers `0x482b08` to `0x482b36` of `0x482ad0`.

## Time of day to half-minutes

The light table build evaluates its bands at h = round(f32(t·2880) − 0.5), with t the time of day
at state `+0x4`, the product rounded to a float and the rounding to nearest, ties to even, so an
exact odd product comes out one low. For t in [0, 1) h runs from 0 to 2880. Other callers pass
their own h: glue evaluates LightParams 3, bands 0 and 1, at h = 0.

Addresses: `0x6d2483`, `0x6d248f`, `0x6d2495`, `0x6d249b`, `0x6d24a1`, `0x811630`, `0x86f2c8`, `0x4721f8`, `0x47220f`
Confidence: verified
Proof: one
Test: `lighting::dn_color_table_build_difftest`, `lighting::dn_color_table_edges_difftest`

`fld [0xce9b64]`, `fmul [0x811630]` (2880.0) and `fstp` a float local (`0x6d2495`), then `fld`,
`fsub [0x86f2c8]` (0.5) and `fistp` (`0x6d24a1`) under the client's control word. Each gather site
repeats the same five instructions. Glue's `0x4721f8` and `0x47220f` call `0x6d62e0` with
edx = 0 on the bands `0x6d6ad0` returns for LightParams record 3.

## Key search

`0x6d63e0` finds the segment of a band's keys that holds h, and its fraction. Starting from key 0
it takes the next key, i + 1 modulo the count, and accepts the pair when the next key's time is
greater and keys[i] ≤ h ≤ keys[next]; when the next time is not greater the pair wraps midnight
and accepts h ≥ keys[i] or h ≤ keys[next], adding 2880 to the next time and, below keys[i], to h.
The fraction is (h − keys[i]) / (keys[next] − keys[i]) in integers, divided at 53 bits and rounded
to a float. With no keys it writes i = next = 0 and fraction 0; a single key is its own pair.

Addresses: `0x6d63e0`, `0x6d63ea`, `0x6d6407`, `0x6d6417`, `0x6d6424`, `0x6d642e`, `0x6d6437`, `0x6d644d`, `0x6d6450`, `0x6d646c`
Confidence: verified
Proof: one
Test: `lighting::dn_band_find_frac_difftest`

`0x6d63e0(keys, &frac, &i, &next)` with h in ecx and the count in edx, `ret 0x10`. It zeroes the
three outputs first (`0x6d63ea`) and returns at once for a count of 0 or less (`0x6d6407 jle`).
`0x6d6417 idiv` makes next. `0x6d6424 jg` takes the rising pair to `0x6d645b`: h < keys[i] or
h > keys[next] moves on. The wrap pair accepts h ≤ keys[next] directly and otherwise needs
h ≥ keys[i]; `0x6d642e` adds 0xb40 to keys[next] and `0x6d6437` to h when h < keys[i]. `fild`
then `fidiv` (`0x6d644d`), `fstp` to the fraction (`0x6d6450`). With no pair accepted the loop
leaves i equal to the count and the fraction 0 (`0x6d646c`); sorted keys always accept a pair.

## Colour band value

A colour band's value at h is opaque black, 0xff000000, when it has no keys. Otherwise each of
its three channels is the low byte of floor(f32(lo + (hi − lo)·f + 512)) − 512, with lo and hi the
channel's byte at keys i and next and f the fraction; the result is 0xff000000 with the channels
from the record's byte 2 in bits 16 to 23, byte 1 in 8 to 15 and byte 0 in 0 to 7.

Addresses: `0x6d62e0`, `0x6d62f4`, `0x6d62f6`, `0x6d6319`, `0x6d6355`, `0x6d6372`, `0x6d6379`, `0x6d6385`, `0x6d639d`, `0x6d63d4`, `0x8029cc`
Confidence: verified
Proof: one
Test: `lighting::dn_band_lerp_bgra_difftest`, `lighting::dn_band_repack_half_boundary_difftest`, `lighting::dn_band_single_key_constant_difftest`, `lighting::dn_band_zero_key_commit_difftest`

`0x6d62e0(out, h, row)`, `ret 4`, returns `out`. A count of 0 stores the immediate at `0x6d62f6`.
Per channel: `fild` (hi − lo), `fmul` the fraction, `fiadd` lo, `fadd [0x8029cc]` (512.0) and
`fstp` a float; the float's bits shifted right by 14 hold the channel in their low byte
(`0x6d6385`), so the add rounds at 2^-14 and the shift truncates. The pack starts
`or ecx,0xffffff00` (`0x6d639d`), forcing the top byte.

## Scalar band value

A scalar band's value at h is 0.0 with no keys, and otherwise v[i] + (v[next] − v[i])·f at 53
bits, left on the x87 stack; every caller stores it as a float.

Addresses: `0x6d6480`, `0x6d648e`, `0x6d6490`, `0x6d64b6`, `0x6d64ba`, `0x6d64be`, `0x6d64c1`, `0x7ffd74`
Confidence: verified
Proof: one
Test: `lighting::dn_band_lerp_scalar_difftest`, `lighting::dn_band_zero_key_commit_difftest`

`0x6d6480(h, row)` in ecx and edx. A count of 0 loads `[0x7ffd74]`, 0.0 (`0x6d6490`). Otherwise
`fld v[next]`, `fsub v[i]`, `fmul f`, `fadd v[i]` (`0x6d64b6` to `0x6d64c1`) and returns.

## Light record

`0x6d64d0` fills a 0x90-byte light record from a LightParams record at half-minute h: the 18 colour
bands and 6 scalar bands at h, and the record's own columns. Every slot below is written on every
call; `+0x80` and `+0x84` are not written. The table at `0xce9c0c` is such a record.

| offset | value | offset | value |
|---|---|---|---|
| `+0x00` to `+0x1c` | colour bands 0 to 7 | `+0x50` | LightParams column 1, int to float |
| `+0x20` to `+0x40` | colour bands 9 to 17 | `+0x54` to `+0x60` | scalar bands 2 to 5 |
| `+0x44`, `+0x48` | scalar bands 0 and 1 | `+0x64` to `+0x74` | LightParams columns 4 to 8 |
| `+0x4c` | colour band 8 | `+0x78`, `+0x7c` | LightParams column 2, 1.0 |
| `+0x88`, `+0x8c` | LightParams column 3, 1.0 | | |

Addresses: `0x6d64d0`, `0x6d64e4`, `0x6d65d5`, `0x6d66d1`, `0x6d66e6`, `0x6d675e`, `0x6d6761`, `0x6d676b`, `0x6d6792`, `0x6d6795`, `0x6d67a5`, `0x6d67ac`
Confidence: verified
Proof: one
Test: none

`0x6d64d0(h, out, row)`, h in ecx, out in edx, `ret 4`, has no conditional jump. For k = 0 to 17
it calls `0x6d6ad0(row, k)` and `0x6d62e0` and stores the colour; k = 8 goes to `+0x4c`
(`0x6d65d5`), k = 9 to 17 to `+0x20` to `+0x40` (to `0x6d66d1`). The scalars `fstp` to `+0x44`,
`+0x48`, `+0x54`, `+0x58`, `+0x5c`, `+0x60` (`0x6d66e6` to `0x6d675e`). `0x6d6761 fild [row+4]`
stores `+0x50`; the dwords `[row+0x10]` to `[row+0x20]`, `[row+8]` and `[row+0xc]` are copied, and
0x3f800000 to `+0x7c` and `+0x8c`.

## Record blend

`0x6d67c0` blends a source record into a destination by a weight t clamped to [0, 1] (NaN as 1).
With a = the low byte of the bits of f32(t·255 + 512) shifted right by 14, colours `+0x00` to
`+0x40` and `+0x4c` move each channel to dst + ((src − dst)·a >> 8), keep the destination's top
byte, and take the source's three channels whole when a is 255. The floats `+0x44`, `+0x48`,
`+0x58` and `+0x64` to `+0x74` become dst + (src − dst)·t; `+0x50`, `+0x54`, `+0x5c`, `+0x60` and
`+0x78` onward are kept.

Addresses: `0x6d67c0`, `0x6d67d6`, `0x6d67ef`, `0x6d6807`, `0x6d6813`, `0x6d6820`, `0x6d6913`, `0x6d691d`, `0x6d69d8`, `0x6d6a11`, `0x6d6c20`, `0x6d6c35`, `0x690ae0`, `0x69eb70`
Confidence: verified
Proof: one
Test: `lighting::sky_weighted_blend_difftest`

`0x6d67c0(dst, src, t)`, `ret 4`. `fcomp 0.0`, `test ah,5`, `jp` keeps t unless below 0
(`0x6d67d6`); `fcomp 1.0`, `jnp` keeps it below 1 (`0x6d67ef`); a NaN takes 1.0. Colours `+0x00`
to `+0x24` go through `0x6d6c20(dst, a, src)` (`0x6d6820`), which returns at a = 0, and `+0x28` to
`+0x40` and `+0x4c` through `0x690ae0` only when a ≠ 0 (`0x6d6913`); the two compute the same
bytes: a = 255 replaces the low 24 bits (`0x6d6c35`), otherwise `imul` and `shr 8` per channel,
the low byte added to the destination's. `0x69eb70` is the 24-bit replace alone.

## Storm weight

The storm weight at state `+0x6c` is min(4·density, 1.0), with the density at state `+0x40`; it
has no lower clamp, and a NaN density gives NaN.

Addresses: `0x6d4500`, `0x6d4506`, `0x6d4512`, `0x6d4519`, `0x6d451d`, `0x6d4528`, `0x80306c`, `0xce9bcc`
Confidence: verified
Proof: one
Test: `lighting::cloud_density_clamp_difftest`

`fld [0xce9ba0]`, `fmul [0x80306c]` (4.0); `fld 1.0`, `fcomp st(1)`; `test ah,5`, `jp` stores the
product (`0x6d4528`) unless 1.0 < product, which stores 0x3f800000 (`0x6d451d`).

## Area lights

The Light records around the camera blend into the base record. Each list slot from 1 whose
distance d from the camera eye, √(dx² + dy² + dz²) rounded to a float, is below its outer radius
joins a max-heap by d. The heap drains farthest first: each light's record is gathered as for the
base, with the storm blend, and blended in by w = 1 when d ≤ inner, else 1 − (d − inner) /
(outer − inner). The nearest light is blended last.

Addresses: `0x6d2d00`, `0x6d2d2f`, `0x6d2d64`, `0x6d2d95`, `0x6d2d9d`, `0x6d2da3`, `0x6d2dab`, `0x6d2dda`, `0x6d2e60`, `0x6d2e91`, `0x6d2ea2`, `0x6d2f75`, `0x6d2f8b`, `0x6d2fe8`, `0x6d2ffd`, `0x6d301a`, `0x6d3032`, `0x6d6a90`, `0x6d5a70`, `0x6d5ad0`, `0x6d5b70`
Confidence: verified
Proof: one
Test: `lighting::cloud_layer_distance_difftest`, `lighting::cloud_layer_factor_difftest`, `lighting::cloud_phase_index_difftest`

`0x6d2d00(base, k)` starts at slot 1 (`0x6d2d2f`), reads the position through `0x6d6a90`, and
`fstp`s the root (`0x6d2d9d`). `fcomp [row+0x18]`, `jp` (`0x6d2dab`) skips d ≥ outer and NaN. The
sift-up stops when d is below the parent (`0x6d2dda`, `test ah,1`), so an equal distance rises.
The drain pops index 1 (`0x6d2e60`), sifts down choosing the right child unless it is below the
left (`0x6d2e91`), gathers with reference k (`0x6d2f75`), blends reference k + 2 when the storm
weight is above 0 (`0x6d2f8b`, `0x6d2fe8`), and computes w with `fcomp [row+0x14]` and
`test ah,0x41` (`0x6d2ffd`), `fdivp` (DE F9) and `fsubr 1.0` (`0x6d301a`) before `0x6d30e0`.
The tests run `0x6d2d00` from `0x6d2d69` to `0x6d2da0` and from `0x6d300e` to `0x6d3025` and at `0x6d2f40` and `0x6d2f98`.

## Area overblend

`0x6d30e0` moves the base record toward an area light's record by w. Each channel of the colours
`+0x04`, `+0x00`, `+0x4c`, `+0x08` to `+0x1c`, `+0x20` to `+0x30` and `+0x34` to `+0x40` becomes
round(f32(lerp) − 0.5), ties to even, with alpha 0xff; the floats `+0x44`, `+0x48`, `+0x50`,
`+0x58` and `+0x64` to `+0x74` become the lerp. The lerp is a − (a − b)·w when b < a, else
a + (b − a)·w. An area light's skybox adds its weight to a matching pair or takes a free one.

Addresses: `0x6d30e0`, `0x6d310f`, `0x6d3114`, `0x6d311a`, `0x6d3120`, `0x6d319d`, `0x6d375b`, `0x6d3770`, `0x6d378c`, `0x6d37a4`, `0x6d37ae`, `0x6d37c2`, `0x6d37c8`, `0x6d15f0`, `0x6d15fe`, `0x6d1619`, `0x4a2c10`
Confidence: verified
Proof: one
Test: `lighting::cloud_record_overblend_difftest`, `lighting::sky_scalar_lerp_difftest`

`0x6d15f0(a, b, w)`, `ret 0xc`, is the lerp: `fcomp` b against a, `test ah,1`, then `fsub`,
`fmul`, `fadd` or `fsubr` (`0x6d1619`). Each channel `fstp`s the lerp, subtracts 0.5 and `fistp`s
(`0x6d3120`); an exact odd integer drops by one. `0x4a2c10(0xff, c2, c1, c0)` packs them. `+0x54`,
`+0x5c` and `+0x60` are not touched. The skybox pass runs when the source's `+0x78` is nonzero and
w > 0 (`0x6d3770`): a matching id adds w, capped at 1.0 (`0x6d37a4`); an empty pair takes the id
and w (`0x6d37ae`); two other ids drop it. `+0x88` is copied and w stored at `+0x8c`.

## Light table build

`0x6d2260` builds the light table from the camera's liquid. Dry or in water or ocean, the table is
the light list's slot 0 record gathered with reference k (1 underwater, 0 otherwise), blended with
reference k + 2 by the storm weight when it is above 0, then the area lights. In magma it is
LightParams 7 and in slime LightParams 6, gathered straight into the table; any other liquid kind
leaves the table as it was. Before any map loads it is the default table.

Addresses: `0x6d2260`, `0x6d226e`, `0x6d2277`, `0x6d227c`, `0x6d228b`, `0x6d22a6`, `0x6d22c2`, `0x6d22db`, `0x6d22fb`, `0x6d2315`, `0x6d2371`, `0x6d237d`, `0x6d2386`, `0x6d2389`, `0x6d239e`, `0x6d23e1`, `0x6d23c9`, `0x6d240c`, `0x6d24ad`, `0x6d24bf`, `0x6d24d5`, `0x6d2572`, `0x6d259c`, `0x6d25d4`, `0x6d26df`, `0x672470`
Confidence: verified
Proof: one
Test: `lighting::dn_color_table_build_difftest`, `lighting::dn_color_table_edges_difftest`

`0x672470(0)` returns the liquid word, 0xf when dry; k = 1 when its low nibble is 0 or 1
(`0x6d227c`, `0x6d228b`). With an empty list and no override (`0x6d22a6`, `0x6d22c2`) it fills the
table with 0xffffffff, fog end 1e10 (`0x6d22fb`), colour 1 0xff404040 (`0x6d2315`) and fixed
scalars. A full word of 0xf or k = 1 takes the full path (`0x6d2371`, `0x6d237d`); otherwise nibble
2 reads `[map+0x1c]` when the largest id is at least 7 (`0x6d23e1`) and nibble 3 `[map+0x18]` at
least 6 (`0x6d239e`), each gathered into `0xce9c0c`. The full path gathers on the stack,
`setne dl` (`0x6d24ad`), `jne` on a storm weight not above 0 (`0x6d24d5`), and `0x6d2d00`
(`0x6d25d4`); with no override it copies the stack record into the table (`0x6d26df`).

## Override light

Setting state `+0x50` to a reference k switches the table to the light list's slot 0 reference k
the next frame, with no blend; −1 is the normal light. `0x6d4620(4)` sets it for a ghost and
`0x6d4630` clears it. With an override the table is that record's gather, keeping the location's
`+0x64` to `+0x8c`, and the override's skybox column becomes the override skybox at weight 1.0. A
nonzero `+0x4c` names the LightParams record to use instead; nothing live sets it.

Addresses: `0x6d4620`, `0x6d4630`, `0x6d45f0`, `0x6d25d9`, `0x6d25e2`, `0x6d25ea`, `0x6d25f8`, `0x6d25fc`, `0x6d262f`, `0x6d2634`, `0x6d268e`, `0x6d2694`, `0x6d26a5`, `0x6d26c6`, `0x6d26cb`, `0x5de9c0`
Confidence: verified
Proof: one
Test: `lighting::dn_color_table_build_difftest`, `lighting::dn_color_table_edges_difftest`

`0x6d25d9` reads `[0xce9bb0]`; −1 takes the copy (`0x6d25e2`). Otherwise an empty list skips
(`0x6d25ea`), `[0xce9bac]` nonzero is the record (`0x6d25f8`), else `0x6d6ab0([slot0], edx)` with
edx still `[0xce9bb0]` (`0x6d25fc`); a null record skips. It gathers into the table (`0x6d262f`)
and copies stack `+0x64` to `+0x8c` over it (`0x6d2634` to `0x6d268e`). A skybox id in
`[record+8]` that is nonzero and below the LightSkybox count (`0x6d26a5`) is resolved and loaded
(`0x6d26c6`), its entry stored at `0xce9bb4` with 1.0. The player field code `0x5de9c0` is the only
caller of both setters. `0x6d45f0`, which sets `+0x4c`, has no caller (`overview#unreached-code`).

## Underwater tint

Underwater in water or ocean, the table's fog colour, ambient and diffuse are tinted after the fog
derive: f = 1 in water, and in ocean, when the whole liquid word is 1, f = 1 + clamp(z, −30, 0)/30
with z the camera eye's height (NaN as −30). Fog colour and ambient have their HSV value scaled
by (f + 1)·0.5 and diffuse by f·0.25 + 0.75. The fog colour written here is replaced later the
same frame by the fog update.

Addresses: `0x6d2809`, `0x6d280e`, `0x6d2819`, `0x6d2834`, `0x6d2847`, `0x6d285a`, `0x6d2872`, `0x6d2878`, `0x6d288a`, `0x6d28a7`, `0x6d28c5`, `0x6d28da`, `0x6d28f7`, `0x6d1620`, `0x6d1688`, `0x7bbc80`, `0x7bbd60`, `0x81162c`, `0x811628`
Confidence: verified
Proof: one
Test: `cmath::hsv_to_rgb_difftest`, `cmath::rgb_to_hsv_difftest`, `lighting::dn_color_table_build_difftest`, `lighting::dn_color_table_edges_difftest`, `lighting::dn_color_to_bgra_difftest`

After `0x6d37e0` (`0x6d2809`) the tint runs when k = 1 (`0x6d280e`) and ramps only when
`esi − 1 == 0` (`0x6d2819`). The ladder clamps `[0xce9b80]` to [−30, 0] (`0x6d2834`, `0x6d2847`,
`0x6d285a`), `fmul` −1/30 (0xbd088889) and `fsubr 1.0` (`0x6d2878`). `0x6d1620(out, colour, k)`
converts the bytes times 1/255 to HSV with `0x7bbc80`, multiplies the value (`0x6d1688`),
converts back with `0x7bbd60`, and packs each channel as low byte of f32(c·255 + 512) bits >> 14
with alpha 0xff.

## Screen tint

A spell visual sets a tint colour and a level in [0, 1]; the level becomes the byte
a = bits(f32(level·255 + 512)) >> 14. When a ≠ 0 the table build moves the fog colour, ambient,
diffuse, sun disc and white moon colours toward the tint by the 8-bit blend, after the
underwater tint and the body broadcast.

Addresses: `0x6d45c0`, `0x6d45cf`, `0x6d45e3`, `0x6d290d`, `0x6d2943`, `0x6d295f`, `0x6d2992`, `0x6d2a21`, `0x6d2a58`, `0x6d2ad5`, `0x6d2b7e`, `0x6d2c24`, `0xce9ba4`, `0xce9ba5`
Confidence: verified
Proof: one
Test: `lighting::dn_color_table_build_difftest`, `lighting::dn_color_table_edges_difftest`, `lighting::dn_override_sky_phase_difftest`

`0x6d45c0(colour, level)`, `ret 8`, stores the colour dword at `0xce9ba5` (`0x6d45cf`) and the
byte (`0x6d45e3`). `0x6d2943 jnp` skips when a = 0. The fog colour goes through `0x690ae0`
(`0x6d295f`); ambient, diffuse, sun disc and white moon are the same blend inline, a = 255 taking
the 24-bit replace (`0x6d2992`, `0x6d2a58`), each keeping its own top byte (`0x6d2a21`, `0x6d2ad5`,
`0x6d2b7e`, `0x6d2c24`). The spell side is `spell/visuals#screen-tint`.

## Body colours

Each build copies colour band 9 (table `+0x20`) into the sun disc, sun glare, white moon and moon
glare colours. When the storm weight is not 0 it then writes alpha = bits(f32((1 − w)·255 +
512)) >> 14 into the top byte of those four and of moon02's colour.

Addresses: `0x6d2914`, `0x6d291d`, `0x6d2926`, `0x6d292c`, `0x6d2932`, `0x6d2c3d`, `0x6d2c57`, `0x6d2c60`, `0x6d2c74`, `0xce9864`, `0xce97c0`, `0xce9884`, `0xce9710`, `0xce98a4`
Confidence: verified
Proof: one
Test: `lighting::dn_color_table_build_difftest`, `lighting::dn_color_table_edges_difftest`

`0x6d2914` loads `[0xce9c2c]` and stores it at the four colours (`0x6d291d` to `0x6d2932`).
`fcomp 0.0`, `test ah,0x44`, `jnp` skips only an exact 0 (`0x6d2c3d`), so a NaN or negative
weight stamps. The byte goes to `0xce9867`, `0xce97c3`, `0xce9887`, `0xce9713` and `0xce98a7`
(`0x6d2c60` to `0x6d2c74`). Nothing else writes moon02's colour bytes `0xce98a4` to `0xce98a6`.

## Skybox slots

The table's two skybox pairs at `+0x78` and `+0x80` become the slot handles `0xce9bbc` and
`0xce9bc0` with weights `0xce9bc4` and `0xce9bc8`: a pair with a nonzero id below the LightSkybox
count and a weight above 0 loads its model path. The build clears all three slots first; the
override slot `0xce9bb4` is the override light's. `0x6d4810` sets a slot from outside.

Addresses: `0x6d2292`, `0x6d22ac`, `0x6d2c79`, `0x6d2c97`, `0x6d2c9f`, `0x6d2cbd`, `0x6d2cc2`, `0x6d4810`, `0xce9bb4`, `0xce9bbc`, `0xce9bc0`, `0xce9bc4`, `0xce9bc8`
Confidence: verified
Proof: one
Test: `lighting::dn_color_table_build_difftest`, `lighting::dn_color_table_edges_difftest`

The loop at `0x6d2292` zeroes `0xce9bbc` to `0xce9bc8`, and `0x6d22ac` the override pair. The pass
from `0x6d2c79` skips a zero id, a weight not above 0 (`0x6d2c97`) and an id not below
`[0xce9db4]` (`0x6d2c9f`), resolves through the id map, calls `0x6d4640` on the record's path
(`0x6d2cbd`) and stores the entry and the weight. `0x6d4810(slot, path, weight)` loads a path into
slot 0 to 2 counted from `0xce9bbc`, or clears it for a null path; the world's map object pass
uses it (`sky#skyboxes`).

## Recompute

The table is rebuilt every frame by the world's sky update. `0x6d2210` clears the interior fog
blend, then runs the storm weight, the table, the sun direction and the celestial bodies, and
rebuilds the cloud texture whole when its flag is set, else one band of it. The per-frame path
first sets the camera heading.

Addresses: `0x6d2210`, `0x6d2215`, `0x6d2229`, `0x6d222e`, `0x6d2233`, `0x6d2238`, `0x6d2247`, `0x6d224c`, `0x6d4530`, `0x6d4551`, `0x6d4555`, `0x6d4569`, `0x6d4582`, `0x6d4596`, `0x6d45b9`, `0x66ff60`, `0x66ff98`, `0x66ffe3`, `0x66ffe8`, `0x811658`, `0x80295c`
Confidence: verified
Proof: one
Test: `lighting::wind_angle_update_difftest`

`0x6d2210` zeroes `0xce9bdc`, `0xce9bf8` and `0xce9c00`, calls `0x6d4500`, `0x6d2260`, `0x6d3a10`
and `0x6d3b80`, and jumps to `0x6cff90` when cl is set, else `0x6cffc0`. `0x66ff60` calls it with
cl = 1 when its flag argument is set (`0x66ff98`), else `0x6d4530` (`0x66ffe3`); both then call
the fog update (`0x66ffe8`). `0x6d4530` stores atan2(fy, fx) of the camera forward when
fx² + fy² is not at most 0.0001 (`0x6d4555`), else atan2(fz, fx) (`0x6d4569`), adds 2π below 0
(`0x6d4582`), runs the same four steps, the cloud band and the stars (`0x6d45b9`).
The tests run `0x6d4530` from `0x6d4530` to `0x6d4596`.

## Night fraction

Each build stores two curves of the time of day: `+0x1ac` is 1 at night and 0 by day, from 1.0
at 21:30 and 06:00 to 0 at 20:30 and 07:00; `+0x1b0` rises from 0.25 at 02:00 to 1.0 at 12:00.
The map object group fade `0x6b4090` reads the first.

Addresses: `0x6d2cd9`, `0x6d2cde`, `0x6d2ce4`, `0x6d2ce9`, `0x6d4860`, `0x6d4890`, `0x6ce670`, `0x6ce700`, `0xce9a24`, `0xce9a34`, `0xce9d0c`, `0xce9d10`, `0x6b4090`
Confidence: verified
Proof: one
Test: `lighting::dn_color_table_build_difftest`, `lighting::dn_color_table_edges_difftest`, `lighting::dn_init_6ce670_difftest`, `lighting::dn_track_phase_wrap_difftest`

`0x6d4860(0)` evaluates the four keys at `0xce9a34` at the time of day plus its argument, and
`0x6d4890` the two at `0xce9a24`, through `celestial#tracks`. The static initializers store the keys: `0x6ce670` (0.25, 1),
(0.2916667, 0), (0.8541667, 0), (0.8958333, 1), and `0x6ce700` (0.0833333, 0.25), (0.5, 1.0).
`0x6b4090` loads the state's `+0x1ac`.
