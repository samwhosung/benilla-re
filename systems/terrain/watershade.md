# terrain: water shading

What decides how a liquid pixel is shaded: the video options and the pixel programs they enable,
what the water passes leave to the device (vertex colour, lighting, material, committed lights),
the equation of the water programs, how the depth ramp swatches follow the light state, and
the colour sources of the map object group water. The passes themselves are in `liquidpass`, the
vertex fills in `liquid`, the group draws in `groupdraw`.

Roots: `0x6941f0`, `0x6809c0`, `0x68de40`

## Pixel program gates

`CMap::Load` (`0x6941f0`) turns two video options into the flags every pixel program path tests.
The pixel shaders option is bit 28 of the world flags `0xc7b2a4` and the specular option bit 27;
both are set by their console variables (`pixelShaders`, `specular`, default 0) only on a device
whose capability word `+0x98` is not negative. The pixel flag `0xc9a340` is bit 28. With specular
and pixel shaders both on, `0xc9607c` is 1; with bit 27 or 28 clear it is 0. A program counts as
available when the handle is nonzero and its loaded word `+0x2c` is. Each gate below also needs
`0xc9a340`.

Addresses: `0x6941f0`, `0x6941fb`, `0x694205`, `0x69420d`, `0x69421f`, `0x694225`, `0x69423b`, `0x694271`, `0x694279`, `0x694282`, `0x6942ae`, `0x6942dc`, `0x69430a`, `0x694338`, `0x69436c`, `0x69437b`, `0x58b360`, `0x58b2b0`, `0x688e20`, `0x688f40`, `0x688e4f`, `0x688e7d`, `0x688f6f`, `0x688f9d`, `0x66f6c0`, `0xc9a340`, `0xc9607c`, `0xc9a31c`, `0xc9a324`, `0xc89f44`, `0xca8064`, `0xca80bd`, `0xca808c`, `0xca8065`, `0xc96304`, `0xca809c`, `0xc7b2a4`
Confidence: verified
Proof: one
Test: none

| flag | set when |
|---|---|
| `0xc89f44` | the `terrainp` and `terrainp_us` programs are available |
| `0xc9a31c` | specular and pixel shaders are on and `terrainp_s` is available |
| `0xc9a324` | `0xc9607c` is 1 and `ocean0_s` or the third-stage program `0xc800ec` is available |
| `0xca8064` | the `mapObjOverbright` variable `0xca809c` is on and `MapObjOverbright` is available |
| `0xca80bd` | `0xc9607c` is 1 and `MapObjSpecular` is available |
| `0xca808c` | `0xc9607c` is 1 and `MapObjMetal` is available |
| `0xca8065` | `MapObjTransDiffuse` and `MapObjTransSpecular` are both available |
| `0xc96304` | world flag 0x10000 |

`0x6941f9` shifts the flags right by 28 and `0x694205` tests bit 27. The program files are loaded
by `0x68c6f8` (`ocean0_s.bls`), `0x691b10` (the terrain programs `terrainp`, `terrainp_s`,
`terrainp_us`, `terrainp_u`, `terrain1` to `terrain4` and `terrain1_s` to `terrain4_s`, handles from
`0xc89ef8`, `0xc89f1c`, `0xc9a344`, `0xc962a0`, `0xc9608c` to `0xc96098` and `0xc962d0` to `0xc962dc`)
and `0x6a3040` (the eight map object programs, `groupdraw`). The callbacks `0x688e20`
(`Specular enabled`, `Specular disabled.`) and `0x688f40` (`Pixel shaders enabled.`) set bits 27 and 28
at `0x688e4f` and `0x688f6f` and clear them at `0x688e7d` and `0x688f9d`. The start flags
`0x7100b73` set neither bit (`0x66f6cb`).

## Device lighting

On the OpenGL arm the device is set up once with a zero global ambient, separate specular colour,
a local viewer, tracking of ambient and diffuse material by the vertex colour and normalization.
Lights are not part of the state journal that a push and pop save, so a light committed by an
earlier pass stays in force. The water passes commit none.

Addresses: `0x59cf5a`, `0x59cf6a`, `0x59cf80`, `0x59cf89`, `0x59cff1`, `0x59cffd`, `0x59d008`, `0x59d3dc`, `0x59d44c`, `0x59d4bc`, `0x59c8e0`, `0x59c1e9`, `0x593bf0`, `0x593c66`, `0x593d0c`, `0x593d18`, `0x593d24`, `0x593d30`, `0x593d3c`, `0x593d52`, `0x593d58`, `0x593d93`
Confidence: verified
Proof: one
Test: none

The calls are `glLightModelfv(0xb53)` for the ambient, `glLightModeli(0x81f8, 0x81fa)` (colour control,
separate specular, through a register at `0x59cf70`) and `glLightModeli(0xb51, 1)` (local viewer), then
`glEnable(0xb57)` colour material with `glColorMaterial(0x408, 0x1602)` and `glEnable(0xc11)`. The
two-sided light model (0xb52) is never set. State id 3 reaches `glMaterialfv(GL_FRONT, GL_SPECULAR)`
(`0x59d44c`), id 4 `glMaterialf(GL_FRONT, GL_SHININESS)` (`0x59d4bc`) and id 2 the emission; the
colour is 0xAARRGGBB scaled by 1/255. State id 1 is the current vertex colour, applied (`0x59c1e9`)
only when the bound vertex format has no colour element. `0x593bf0` writes the state defaults at
`cache + id × 0x18`: colour 0xffffffff, lighting (0xe) 1, fog (0xf) 1, depth test (0x10) 1, depth
write (0x12) 1, colour mask (0x13) 0xf, culling (0x14) 1, blend (0x7) 0, and 1 for the combine
preset of each of the eight stages (ids 0x1f to 0x26).

## Stage combine presets

State id 0x1f plus a stage index selects one of six texture environment presets; the passes use
preset 3, add, for the water sheet. The preset table is six records of 0x2c bytes at `0x85c250`.
When the card lacks the combine extension the OpenGL arm sets only the fallback mode in the first
dword, and the alpha of an add then is the product of the two alphas; with it the arm uploads the
whole record and the alpha of preset 3 is the sum.

Addresses: `0x85c250`, `0x59baa0`, `0x59bc79`, `0x59bc8c`, `0x59bd80`, `0x59d0a0`, `0x59d0b4`, `0x59d0be`, `0x59b2b9`, `0x59d866`, `0xc0f2b8`, `0xc0f3e4`
Confidence: verified
Proof: one
Test: none

| preset | colour | alpha | notes |
|---|---|---|---|
| 0 | previous × texture | previous | fallback mode replace |
| 1 | previous × texture | previous × texture | the default of every stage |
| 2 | texture over previous by the texture's alpha | previous | decal |
| 3 | previous + texture | previous + texture | |
| 4 | previous × texture × 2 | previous × texture × 2 | |
| 5 | interpolate | previous | |

`0x59d866` calls `0x59d0a0` with the public id less 0x1f; it selects the texture unit (internal
id 2, `0x59d0b4`) and sets internal id 0x2b plus the stage (`0x59d0be`) to the preset; `0x59bc79` drops a stage past the unit count `0xc0f3e4`. A record holds the fallback mode, then the scale, three sources and the operation for colour and
again for alpha, uploaded as the combine parameters (`0x59bc99` to `0x59bd7b`). The flag `0xc0f2b8` is written once, at `0x59b2b9`, from the probe of
the combine extension at `0x59b2ab`. The operand modes are never set. A pass that sets a preset between its push and pop leaves the
stage as it found it.

## Water draw inputs

The terrain water passes bind two textures, the ramp swatch on stage 0 and the animated sheet on
stage 1, and set no lighting, no colour and no fog. A liquid vertex has no colour element, so
the pass is lit through the current colour, 0xffffffff by default, and the normal (0, 0, 1) of its
fill. With the fancy leg the material specular is white and the shininess 6.0; without it neither
is set and the blend stays off, so the surface draws opaque. The map object group water sets
its own states (`groupdraw`).

Addresses: `0x6851b0`, `0x685010`, `0x6851da`, `0x68503a`, `0x6851ea`, `0x68504a`, `0x685206`, `0x685067`, `0x685452`, `0x685454`, `0x68546d`, `0x6850ad`, `0x68db90`, `0x68dab0`, `0x68dbc3`, `0x68dae3`, `0x58b200`, `0x8102e8`, `0xc9a324`
Confidence: verified
Proof: one
Test: none

Neither pass, nor `0x68db90` and `0x68dab0`, sets state 0xe, so lighting is the default 1; the
magma pass (`0x6855d6`) and the interior group water (`0x6b65bf`) turn it off. The two legs
arrive at one call at `0x6850be` in the ocean pass: the fancy leg carries (0x12, 0) and the other
(0x20, 3). The fancy leg sets no stage combine; the program replaces the texture stages.
The vertex format is 5 (position, normal, two texture coordinates, stride 0x28) for the terrain
liquids and 8 (position, colour, one coordinate, stride 0x18) for the interior group water. The interior
group water is fixed function whatever the options: lighting off and fog on (`0x6b65bf`, `0x6b65d5`),
the sheet on stage 0 with preset 3 (`0x6b63a9`), so its pixel is the vertex colour plus the sheet
in colour and in alpha (`0x6b6420` reads `0xc9607c` nowhere). No liquid draw sets state 8, the alpha test.

## River stage 2

The river pass carries a third texture stage that never runs: its branch at `0x685251` tests the
handle `0xc800ec`, which nothing sets, so the branch to `0x685440` is always taken. The skipped code
would bind the sky gradient texture on stage 2, texture generation mode 4, the stage-2 texture
matrix and a program in `0xc800ec`. The only ocean program name in the executable is
`Shaders\Pixel\ocean0_s.bls`.

Addresses: `0x6851b0`, `0x685244`, `0x685251`, `0x685257`, `0x685440`, `0x68543e`, `0x685285`, `0x685546`, `0x68c7e9`, `0x68c7f8`, `0x68dc39`, `0x694262`, `0x68c6f8`, `0x68c707`, `0x58b310`, `0xc800ec`, `0xc7fbb8`
Confidence: verified
Proof: one
Test: none

Every reference to `0xc800ec` is a read (`0x685244`, `0x685285`, `0x685546`, `0x68dc39`, `0x694262`), a release (`0x68c7e9`) or
the zeroing store at `0x68c7f8`; the load of `0xc81b7c` at `0x68c6f8` and `0x68c707` is the control.
The skipped block binds `0xc7fbb8` to state 0x19, sets state 0x31 to 4 and state 0x39 to 1.

## Water pixel

The water program of the fancy leg adds three terms to the fragment: the lit vertex colour times
the ramp swatch, the sheet colour, and the sheet's alpha times the sum of the specular colour and
0.25. Its alpha is the swatch's. The program of the group water is the vertex colour plus the sheet
colour plus the specular colour times the sheet's alpha, with the vertex colour's alpha.

Addresses: `0x685067`, `0x68c707`, `0x6a30c7`, `0x6b665a`, `0x6b689c`, `0xc81b7c`, `0xca7eb8`, `0x86a214`
Confidence: inferred
Proof: none
Test: none

The executable only names the programs; the arithmetic is read off the program files the data
holds, so it is not derived from the executable's bytes. ADT water: rgb = primary × ramp.rgb +
sheet.rgb + (secondary + 0.25) × sheet.a, a = ramp.a. Group water (`MapObjExtWater0`): rgb = primary +
sheet.rgb + secondary × sheet.a, a = primary.a. The constant 0.25 is a program constant: no
texture environment colour is ever set, and the float 0.25 at `0x684527` to `0x684554` and
`0x684857` to `0x684884` is the mip bias of states 0x27 to 0x2a.

## Swatch refresh

The two swatch textures follow the light state every frame. The dirty flags `0xc8117c` (river) and
`0xc81b70` (ocean) are cleared by `0x6809c0` once a frame, and the first liquid slot drawn after
that finds its flag clear, refills the swatch through the texture update `0x58acd0` (the fill
callback `0x68a830` runs again) and sets the flag.

Addresses: `0x6809c0`, `0x680b90`, `0x680b97`, `0x681064`, `0x68de40`, `0x68de9e`, `0x68ded1`, `0x68deed`, `0x68df20`, `0x58acd0`, `0xc8117c`, `0xc81b70`, `0x81023c`, `0x808de4`
Confidence: verified
Proof: one
Test: none

`0x681064` calls `0x6809c0` once each world frame before the scene driver. `0x68de40(layer)` takes
layer 1 (ocean) to `0x68de9e` and layer 0 (river) to `0x68deed`; each tests its flag, builds the
rectangle (0, 0, 0x40, width) with width 0.125 × 64.0 = 8 and calls `0x58acd0` (`0x68ded1`,
`0x68df20`), and stores 1. The fill callback itself is in `liquidpass`.

## Group water colour

The two water arms of the group liquid colour their vertices differently. The interior arm takes
the second colour of the material the liquid names; the exterior arm takes the light state's
bytes at `+0xec`, colour band 17 of the light record, the deep river colour. Both take the vertex alpha from the depth ramp `0xca7f10`
by the vertex's depth byte, rebuilt each frame the group liquid pass draws.

Addresses: `0x6b6420`, `0x6b6630`, `0x6b64c9`, `0x6b64cf`, `0x6b64e4`, `0x6b674c`, `0x6b6752`, `0x6b6766`, `0x6b6672`, `0x6b6682`, `0x6b665a`, `0x6b689c`, `0x6b6b60`, `0x684dd7`, `0x6c3ab8`, `0x6c3ace`, `0x6c3ad7`, `0xca7f10`, `0xc9607c`, `0xca7eb8`
Confidence: verified
Proof: one
Test: `terrain::mliq_depth_alpha_lut_difftest`

The material array of the model is at `+0x1d8` (64 bytes each, from the root loader at `0x6c3ab8`
to `0x6c3ad7`) and the liquid's material index is the group's `u16 +0x114`; the colour bytes
are `+0x1c`, `+0x1d` and `+0x1e`. The vertex data holds only the depth byte and the height, so a
group water surface carries no depth beyond the alpha. The exterior arm binds the `MapObjExtWater0`
program (`0x6b665a`), sets the specular colour white and the shininess 6.0 (`0x6b6672`, `0x6b6682`)
only when `0xc9607c` is 1 and the program is available, and unbinds them after the draw (`0x6b689c`).
The ramp runs from the byte of the light state's river alpha `+0x114` to that of `+0x118`: 127 to 255
with the start-up values 0.5 and 1.0.
