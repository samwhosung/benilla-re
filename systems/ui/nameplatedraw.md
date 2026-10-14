# ui: how a nameplate draws

The seven parts of a nameplate and how each is laid out, blended and layered, and the type and
colour of its two strings. The frame, its pool, its scripts and its bar are `nameplates`; which units
get one and where its anchor projects are `objectlayer/unitnames`.

Roots: `0x7cb250`, `0x7cb6d0`, `0x7cb850`, `0x7cb8b0`, `0x76fb00`

## Parts and layout

A plate is built from a border, a health bar, a glow, a name, a level, a skull and a raid icon.
Every size and offset is a fraction of the plate's own layout frame, which the bind sets to 0.1 by
0.025 (`0x87d9cc`, `0x87d9d0`), times a constant.

| part | slot | texture or font | anchor | size | layer |
|---|---|---|---|---|---|
| border | local | `Interface\Tooltips\Nameplate-Border` | all points of the plate | the plate | ARTWORK 2 |
| health bar | `+0x504` | `Interface\TargetingFrame\UI-TargetingFrame-BarFill` | BOTTOMLEFT to the plate's, (0.0031, 0.003125) | 0.0804 by 0.007025 | frame level 0 |
| glow | `+0x4f0` | `Interface\Tooltips\Nameplate-Glow` | all points of the plate | the plate | HIGHLIGHT 4 |
| name | `+0x4fc` | `NAMEPLATE_FONT`, 0.01 | BOTTOM to the plate's CENTER, (0, 0) | the text | OVERLAY 3 |
| level | `+0x500` | `NAMEPLATE_FONT`, 0.009 | CENTER to the plate's BOTTOMRIGHT, (−0.0092, 0.0071) | the text | OVERLAY 3 |
| skull | `+0x4f4` | `Interface\TargetingFrame\UI-TargetingFrame-Skull` | CENTER to the level's CENTER | 0.01 by 0.01 | OVERLAY 3 |
| raid icon | `+0x4f8` | `Interface\TargetingFrame\UI-RaidTargetingIcons` | RIGHT to the border's LEFT | 0.02 by 0.02 | ARTWORK 2 |

Addresses: `0x7cb250`, `0x7cb2c2`, `0x7cb2d2`, `0x7cb33e`, `0x7cb35a`, `0x7cb39f`, `0x7cb456`, `0x7cb50b`, `0x7cb591`, `0x7cb5ad`, `0x7cb62b`, `0x7cb6d0`, `0x7cb747`, `0x7cb77d`, `0x7cb7f5`, `0x7cb806`, `0x767c70`, `0x767db0`
Confidence: verified
Proof: one
Test: none

The bind sets the bar position as `0.1·0.031` by `0.025·0.125`, the bar size
as `0.1·0.804` by `0.025·0.281` and the level offset as `0.1·−0.092` by
`0.025·0.284`. The constructor sets the border, glow and raid icon through
`0x767db0` or `0x767c70`; the skull and the raid icon get sizes from the immediates `0x3c23d70a` and
`0x3ca3d70a`. The glow is created hidden (`[+0xc4] = 0`, `0x77fc60`). The point numbers are those of
`anchors`: 0 TOPLEFT to 8 BOTTOMRIGHT. The raid icon hangs off the border's left edge, outside the plate.

## Blend and batch state

Each plate texture carries a blend mode at `+0xd0`, 2 (`BLEND`) by default; only the glow sets 3
(`ADD`), and the border, the skull and the raid icon set 2 again. The batch draw sets lighting, fog,
depth test and depth write off before it draws the plate's quads, and passes a record's mode to the
device as blend state 7 unchanged from 2 up, except that mode 11 leaves the state as it was; a mode
below 2 on a record with a texture draws as 2. Mode 3 is source alpha with a destination of one, so a
hovered plate's glow adds to what is behind it.

Addresses: `0x7703f0`, `0x76fc64`, `0x7cb2f2`, `0x7cb374`, `0x7cb525`, `0x7cb5c7`, `0x77302c`, `0x76fb00`, `0x76fb2c`, `0x76fb44`, `0x76fb50`, `0x76fb82`, `0x76fb9d`, `0x76fba2`, `0x589e60`, `0x878cf0`
Confidence: verified
Proof: one
Test: none

`0x7703f0` stores its argument at `+0xd0` and refreshes the texture when it changed. The names of
the modes are the table `0x811aa8`: `DISABLE` 0, `ALPHAKEY` 1, `BLEND` 2, `ADD` 3, `MOD` 4. Device state
2 is source alpha over inverse source alpha and 3 source alpha over one (`gx`, blend tables). The draw record copies `+0xd0` (`0x77302c`). `0x76fb00` writes
the ids 0x0e, 0x0f, 0x10 and 0x12 as 0, and per record the texture (id 0x17) and, from
`0x76fb82`, the blend: a texture slot `[rec+0x10]` that is non-zero with a mode below 2 sends 2
(`0x76fb9d`), mode 11 skips the store (`0x76fb98`). The five textures are created through `0x770200`
with the filter `[0x878cf0] & 7`; the dword is 1 in the image and has 39 references, all reads.

## Plate text

The name and the level are drawn with the font named by the global string `NAMEPLATE_FONT`, the
name at height 0.01 and the level at 0.009, with no outline or monochrome flag and a black shadow
displaced by 0.001 right and 0.001 down. The name is white and the level is white until the bind
colours it; hovering the plate turns the name yellow and shows the glow, leaving it turns the name
white and hides the glow, each while no flash is running.

Addresses: `0x7cb3e4`, `0x7cb405`, `0x7cb418`, `0x7cb42d`, `0x7cb486`, `0x7cb4a7`, `0x7cb4b2`, `0x7cb4df`, `0x7cb850`, `0x7cb869`, `0x7cb87e`, `0x7cb88b`, `0x7cb897`, `0x7cb8b0`, `0x7cb8bf`, `0x7cb8d4`, `0x7cb8e1`, `0x7cb8ed`, `0x703bf0`, `0x7714f0`, `0x7722a0`, `0x771563`, `0xcf6128`, `0x492890`
Confidence: verified
Proof: one
Test: none

The constructor calls `0x703bf0` on `NAMEPLATE_FONT` and `0x7714f0(font, 0.01, 0, 0)` (`0x3c23d70a`;
the level uses `0x3c1374bc`, 0.009); the flags argument is 0. `0x7722a0` is given the colour
`0xff000000` and the offsets `0x3a83126f` and `0xba83126f` (0.001 and −0.001). The level starts with
the colour at `0xcf6128` (white). `0x7cb850` calls `0x492890` with the plate's GUID `+0x4e8`,
which makes the unit the mouseover, shows the glow and, when the flash timer `+0x514` is not above 0,
colours the name `0xffffff00`; `0x7cb8b0` calls it with a zero GUID, hides the glow and
colours the name `0xffffffff`. The string's height, times its factor `+0x7c` (`0x771563`), reaches the font as
`size / 0x832a48` (the font bridge), so for a string whose factor is 1 the em is
`min(32, round(h·√(W² + H²)))` pixels, the name 13 and the level 12 at 1024 by 768.
