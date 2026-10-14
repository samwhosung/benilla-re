# lighting: overview

The world's light, sky and weather: the time-of-day lights read from the Light DBC tables, the
blend of the area lights around the camera, the fog and the device light the scene draws with,
the sky dome, its skyboxes, the sun, the moons, the stars and the clouds, the weather and its
precipitation, and the projected decals and blob shadows.

The system owns four object files, listed below; `0x827000`, the C runtime's initializer table,
lists each one's static initializers in one run. The docs split by topic: `tables` (the DBC stores
and records), `daynight` (the light state, the band blend and the area lights), `fog` (the colours
and fog the scene takes), `celestial` (the tracks, the bodies and their glare), `sky` (the dome,
the sky pass and the skyboxes), `clouds`, `weather` (the manager), `precipitation` (rain, snow,
sand and mist) and `shadows`. This doc holds the object files, their static initializers and the
code nothing reaches. Other systems enter through the roots each topic doc lists, and through the
static initializers below.

Roots: `0x674560`, `0x674580`, `0x6745a0`, `0x67d070`, `0x67d640`, `0x67d670`, `0x6cdf50`, `0x6ce110`, `0x6ce200`, `0x6ce2c0`, `0x6ce310`, `0x6ce380`, `0x6ce490`, `0x6ce550`, `0x6ce660`, `0x6ce6f0`, `0x6ce750`, `0x6ce9a0`, `0x6ce9e0`, `0x6cea80`, `0x6cebb0`, `0x6cec10`, `0x6cec70`, `0x6ced20`, `0x6cedd0`, `0x6cf330`, `0x6cf3f0`, `0x6d5c00`, `0x6d5cb0`, `0x6d5d70`, `0x6d5e30`, `0x6d5ef0`, `0x6d6df0`, `0x6d6e10`, `0x6d6e60`, `0x6d6eb0`, `0x6d6f40`, `0x7e4f90`, `0x7e4fb0`, `0x7e4fd0`, `0x7e61f0`, `0x7e6660`, `0x6d5050`

## Object files

The weather file, `MapWeather.cpp`, runs from `0x674560` through `0x67e350`. The day-night file runs
from `0x6cdf50` through `0x6d6c20`: the light state and tables, the tracks, the sky dome, clouds and
bodies, the sky pass, the area lights and the DBC readers. The shadow file runs from `0x6d6df0`
through `0x6d8530`, after the guild emblem file. The sky draw file, the body, gradient, cloud and star
draws, runs from `0x7e4f90` through `0x7e6670`.

Addresses: `0x674560`, `0x67e350`, `0x6cdf50`, `0x6d6c20`, `0x6d6df0`, `0x6d8530`, `0x7e4f90`, `0x7e6670`, `0x678218`, `0x82bc44`, `0x82bcb0`, `0x82cde4`, `0x82ce48`, `0x82ce4c`, `0x82ce50`, `0x82ce60`, `0x82dfb4`, `0x82e020`, `0x67e390`, `0x6cde70`, `0x6cded0`, `0x6cdf20`
Confidence: verified
Proof: one
Test: `lighting::snow_exp2_arg_difftest`, `lighting::snow_exp2_blend_difftest`, `lighting::snow_field_lerp_difftest`, `lighting::snow_final_recip_difftest`, `lighting::snow_min_select_scale_difftest`, `lighting::snow_normalize_scale_difftest`, `lighting::snow_record_distance_difftest`, `lighting::snow_wind_deflect_difftest`

`0x678218` pushes the path `MapWeather.cpp`. The runs are `0x82bc44` to `0x82bcb0` (weather,
opening with its +∞), `0x82cde4` to `0x82ce48` (day-night), `0x82ce50` to `0x82ce60` (shadow,
opening with its +∞) and `0x82dfb4` to `0x82e020` (sky draw, opening with its +∞). The slot
`0x82ce4c` between them holds the guild emblem file's initializer; the run after the weather's opens at
`0x67e390` (terrain), and the minimap's after the shadow's. The day-night file's code starts after FFXEffects'
`0x6cde70`, `0x6cded0` and `0x6cdf20`, which only FFXEffects' code calls.
The tests run `0x677ff0` in 8 slices between `0x6780d8` and `0x678572`.

## Shared header constants

The weather and sky draw files open with the shared header's +∞, 2π and 1/2π and run the same
header constants as the map object files, the squares and products of
`models/overview#shared-header-constants`, with 10² and 0.5/10 added. The shadow file runs only
its +∞.

Addresses: `0x674560`, `0x674580`, `0x6745a0`, `0x67d070`, `0x67d0a0`, `0x67d2e0`, `0x67d300`, `0x67d480`, `0x67d640`, `0x67d670`, `0x7e4f90`, `0x7e4fb0`, `0x7e4fd0`, `0x7e61f0`, `0x7e6460`, `0x7e6630`, `0x7e6660`, `0x6d6df0`, `0x674570`, `0x674590`, `0x6745b0`, `0x67d080`, `0x67d0b0`, `0x67d0d0`, `0x67d0e0`, `0x67d100`, `0x67d110`, `0x67d130`, `0x67d140`, `0x67d160`, `0x67d170`, `0x67d190`, `0x67d1a0`, `0x67d1c0`, `0x67d1d0`, `0x67d1f0`, `0x67d200`, `0x67d220`, `0x67d230`, `0x67d250`, `0x67d260`, `0x67d280`, `0x67d290`, `0x67d2b0`, `0x67d2c0`, `0x67d2f0`, `0x67d310`, `0x67d330`, `0x67d340`, `0x67d360`, `0x67d370`, `0x67d390`, `0x67d3a0`, `0x67d3c0`, `0x67d3d0`, `0x67d3f0`, `0x67d400`, `0x67d420`, `0x67d430`, `0x67d450`, `0x67d460`, `0x67d490`, `0x67d650`, `0x67d680`, `0x6d6e00`, `0x7e4fa0`, `0x7e4fc0`, `0x7e4fe0`, `0x7e6200`, `0x7e6220`, `0x7e6230`, `0x7e6250`, `0x7e6260`, `0x7e6280`, `0x7e6290`, `0x7e62b0`, `0x7e62c0`, `0x7e62e0`, `0x7e62f0`, `0x7e6310`, `0x7e6320`, `0x7e6340`, `0x7e6350`, `0x7e6370`, `0x7e6380`, `0x7e63a0`, `0x7e63b0`, `0x7e63d0`, `0x7e63e0`, `0x7e6400`, `0x7e6410`, `0x7e6430`, `0x7e6440`, `0x7e6470`, `0x7e6480`, `0x7e6490`, `0x7e64b0`, `0x7e64c0`, `0x7e64e0`, `0x7e64f0`, `0x7e6510`, `0x7e6520`, `0x7e6540`, `0x7e6550`, `0x7e6570`, `0x7e6580`, `0x7e65a0`, `0x7e65b0`, `0x7e65d0`, `0x7e65e0`, `0x7e6600`, `0x7e6610`, `0x7e6640`, `0x7e6670`
Confidence: verified
Proof: one
Test: none

Each is a 5-byte `jmp` to its body. `0x674560` copies +∞ to `0xc7b4e0`, `0x674580` stores π + π,
`0x6745a0` its reciprocal; `0x67d070` to `0x67d2b0` square 10, 11.111111, 5.5555553, 10, 5, 10,
30, 120, 20, 10, 0.5, 3 and 100; `0x67d2e0` stores 5.0; `0x67d640` squares 10 and `0x67d670`
divides 0.5 by 10. The sky draw file's `0x7e4f90` to `0x7e6660` are the same in its own globals
from `0xcf61dc`. `0x6d6df0` copies +∞ to `0xcea614`.

## Static objects

The day-night file's initializers build its statics: the light state, the sky colour, cloud,
star, night and unused tracks (`celestial#static-tracks`), the skybox table, the sky dome, the
cloud object's octaves, the light list, the four disc records, the two glares, the star object,
and the five DBC stores. The shadow file's clear the ramp texture buffer, its decal geometry
lists and the decal colour, and store its two box ratios. Each pair is `call body; jmp registrar`,
the registrar passing an exit hook to `_atexit`.

Addresses: `0x6cdf50`, `0x6ce110`, `0x6ce200`, `0x6ce2c0`, `0x6ce310`, `0x6ce380`, `0x6ce490`, `0x6ce550`, `0x6ce660`, `0x6ce6f0`, `0x6ce750`, `0x6ce9a0`, `0x6ce9e0`, `0x6cea80`, `0x6cebb0`, `0x6cec10`, `0x6cec70`, `0x6ced20`, `0x6cedd0`, `0x6cf330`, `0x6cf3f0`, `0x6d5c00`, `0x6d5cb0`, `0x6d5d70`, `0x6d5e30`, `0x6d5ef0`, `0x6d6e10`, `0x6d6e60`, `0x6d6eb0`, `0x6d6f40`, `0x6d0c00`, `0x6790a0`, `0x679870`, `0x67ad50`, `0x67c6c0`, `0x67c6d0`, `0x67d5f0`, `0x6cdf60`, `0x6ce080`, `0x6ce090`, `0x6ce1d0`, `0x6ce1e0`, `0x6ce290`, `0x6ce2a0`, `0x6ce2d0`, `0x6ce320`, `0x6ce460`, `0x6ce470`, `0x6ce520`, `0x6ce530`, `0x6ce630`, `0x6ce640`, `0x6ce6d0`, `0x6ce6e0`, `0x6ce730`, `0x6ce740`, `0x6ce820`, `0x6ce9b0`, `0x6ce9c0`, `0x6ce9d0`, `0x6ce9f0`, `0x6cea20`, `0x6cea30`, `0x6cea90`, `0x6ceaa0`, `0x6ceab0`, `0x6cebc0`, `0x6cebe0`, `0x6cebf0`, `0x6cec50`, `0x6cec60`, `0x6ced00`, `0x6ced10`, `0x6cedb0`, `0x6cedc0`, `0x6cee10`, `0x6cee20`, `0x6cf3c0`, `0x6cf3d0`, `0x6cf460`, `0x6cf470`, `0x6d1940`, `0x6d1950`, `0x6d3070`, `0x6d3b60`, `0x6d3b70`, `0x6d4480`, `0x6d4490`, `0x6d44a0`, `0x6d44b0`, `0x6d44c0`, `0x6d44d0`, `0x6d44e0`, `0x6d44f0`, `0x6d52a0`, `0x6d5c40`, `0x6d5cf0`, `0x6d5d00`, `0x6d5db0`, `0x6d5dc0`, `0x6d5e70`, `0x6d5e80`, `0x6d5f30`, `0x6d5f40`, `0x6d6e70`, `0x6d6e90`, `0x6d6ea0`, `0x6d6ec0`, `0x6d6ef0`, `0x6d6f00`, `0x6d6f50`, `0x6d6f70`, `0x6d6f80`
Confidence: verified
Proof: one
Test: none

`0x6cdf50` zeroes the state's clock and camera fields and builds its `0xce9c14` array;
`0x6ce110`, `0x6ce200`, `0x6ce380`, `0x6ce490`, `0x6ce550`, `0x6ce660` and `0x6ce6f0` fill the
tracks at `0xce9b2c`, `0xce9af8`, `0xce9ab8`, `0xce9a98`, `0xce9a58`, `0xce9a34` and `0xce9a24`;
`0x6ce2c0` and `0x6ce310` are plain `jmp`s to stores from `0xce9d20`. `0x6ce750` builds the skybox
table, `0x6ce9e0` the dome `0xce99a0`, `0x6cea80` calls `0x6d0c00` on `0xce98e8`, `0x6cebb0`
clears `0xce98d8`, `0x6cec10` four records from `0xce9858`, `0x6cec70` and `0x6ced20` the glares.
`0x6d5c00` to `0x6d5ef0` clear the stores at `0xce9db0` to `0xce9d60`. Functions' local statics
register `ret` exit hooks the same way, as `0x6790a0` from `0x678960`. `0x6d6e10` stores 5/3 and
1; `0x6d6e60`, `0x6d6eb0` and `0x6d6f40` clear `0xce9e0c`, `0xce9dd4` and set `0xce9dc8`.

## Unreached code

Seven functions of the system have no live caller: the weather grid bounds query `0x67bd90`, the cloud
cover setter `0x6d08e0`, the override LightParams record setter `0x6d45f0` (`0xce9bac`), the disc record accessor
`0x6d4600`, the override skybox setter `0x6d47f0`, and the light list reload `0x6d4c80` with the
light-at-a-point query `0x6d4c90`. No `call` or `jmp` reaches them, no byte sequence `e8` or `e9`
encodes a branch to one, and their addresses appear nowhere in the image as a dword.

Addresses: `0x67bd90`, `0x6d08e0`, `0x6d45f0`, `0x6d4600`, `0x6d47f0`, `0x6d4c80`, `0x6d4c90`
Confidence: verified
Proof: one
Test: none

The census walks from the live entries: references from other systems' code, initializer slots
and stored addresses, across every listing operand, every `e8`/`e9` rel32 in `.text` and every
dword of the image. It reaches 515 of the system's 522 functions; the rel32 scan and the dword
search find nothing for these seven. As a control, the scan finds the calls to `0x6d4940` at
`0x6812ac` and `0x6bddcf`, and the dword `0x67a990` at `0x81002c`.
