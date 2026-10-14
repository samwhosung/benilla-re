# glue: the preview scenes

How the create and select screens light, fog and back their model scenes. The mechanisms of the model
renderer are `models` and the frame's script methods are `ui`; this is what the glue screens set up.

Each glue screen keeps a `ModelFFX` frame whose first model, `+0x318`, is the race's `UI_<Race>`
scene, and whose own model container `+0x314` holds the scene's lights. The create screen puts one
character model beside the scene (`0xb42078`); the select screen puts the selected body and pet.
Neither draws a world: the frame paints once, through `0x76d240`, over a depth-only clear, so the
scene's own geometry is the whole image.

Roots: `0x470b70`, `0x4707b0`, `0x470ce0`, `0x472110`

## Scene container

The background scene model and the character beside it register their lights in one container, the
frame's `+0x314`, built on first use by `0x76cfc0` (`0x706e10`). The background is created with
flags 5 and is the chain root; the character is created with flags 0 and chained to the background
by `0x712f70`. Because bit 0 of the parent's flags is set, `0x7192b0` does not alias the character to
the background's light collector: each model clears its own collector, gathers the container's
lights at its own position, runs its fill callback, then finalises.

Addresses: `0x76cfc0`, `0x706e10`, `0x76ccf5`, `0x70ea0d`, `0x70ea53`, `0x470bd5`, `0x470bdf`, `0x470c79`, `0x4707ed`, `0x712f70`, `0x7192b0`, `0x7192c4`, `0x7192c8`, `0x7192fd`, `0x719306`, `0x71930b`, `0x719324`, `0x70a0f0`
Confidence: verified
Proof: one
Test: none

`0x76cfc3` to `0x76cfd2` read `[esi+0x314]` and create it when null. `0x76ccf5 push 5` is the flags
argument of the scene load; `0x470bd5 push 0` is the character's. `0x70ea0d` stores the container at
`[model+0x2c]` and `0x70ea53` the flags at `[model+4]`. `0x7192c4 test [parent+4],1; je 0x719330`
skips the own gather only for a parent whose bit is clear.

## Create fill callback

The create screen's per-model fill callback `0x470ce0` adds no light. It stores 1 at `+0x1b0` of
the light collector and returns. The collector is cleared every frame by `0x71bc30`, so the create
character is lit by the scene's own lights, gathered at its own position, and nothing else.

Addresses: `0x470ce0`, `0x470c09`, `0x71bc30`, `0x707d24`
Confidence: verified
Proof: one
Test: none

`0x470ce0` is `mov [edx+0x1b0],1; ret 4`. `0x470c09` installs it at `+0x3bc` of the character
model. The collector word is read once, in the model render at `0x707d24`, as a texture-index
selector gated on a render bit of the model.

## Preview fog

The race scenes are fogged by the script, not by the exe. `SetBackgroundModel` calls the frame's
`SetFogColor`, `SetFogNear(0)` and `SetFogFar` from the table `CharModelFogInfo`; the select
screen's load sets the Orc scene and Orc fog the same way. The values are: Human (0.8, 0.65, 0.73) far 222; Orc (0.5, 0.5, 0.5) 270; Dwarf
(0.85, 0.88, 1.0) 500; NightElf (0.25, 0.22, 0.55) 611; Tauren (1.0, 0.61, 0.42) 153; Scourge (0, 0.22,
0.22) 26. The fog reaches the background only: the character's callback stages none and its
collector is cleared each frame. A material with render-flag bit 0x02 is exempt per batch. Once
`SetCharSelectBackground` has run, the select background has no fog either.

Addresses: `0x76d680`, `0x76d68a`, `0x76d7c1`, `0x71c110`, `0x70bb24`, `0x70bb2f`, `0x70bb3d`, `0x70bb46`, `0x71bc30`, `0x472144`, `0x76ee60`, `0x76f1e0`, `0x76f390`
Confidence: verified
Proof: one
Test: `ui::glue_76d680_color_pack`

`0x76d680` tests the frame's fog bit (`+0x3a4`) and calls `0x71c110` with the colour and `+0x3ac`,
`+0x3b0`, which stores near, far and `1/(far − near)` at collector `+0x184` to `+0x18c`. At draw
`0x70bb24` skips fog for a material with bit 0x02, and `0x70bb2f` to `0x70bb40` for a collector
whose `+0x18c` is not above 0 (a NaN too). The fog mode is the dword at `0x811fc4` indexed by the
material's blend mode (`0x70bb46`). `0x472144` replaces `0x76d680` on the select background.

## Preview clear

The frame's draw clears only the depth buffer. `0x76d240` sets the aspect-fit viewport (`0x76d5d7`)
and calls `GxClear(2)` (`0x76d5e1`); the OpenGL clear maps flag bit 0 to the colour buffer and bit 1
to the depth buffer. The OpenGL device's clear colour is opaque black, stored by its constructor; the Direct3D clear `0x5a17d0` swaps in a sine-cycled colour when `[dev+0x269c]&0x100` is clear (`0x5a17f6` to `0x5a189e`), and maps its flags the same way (`0x5a18a6` to `0x5a18b3`), so the depth-only clear stands. The only colour clear
in the frame's draw path is the effect bracket's, `0x6cd978`, which clears the offscreen target the
bracket has just bound when `gxMultisample` is 1.

Addresses: `0x76d240`, `0x76d5d7`, `0x76d5dc`, `0x76d5e1`, `0x58a970`, `0x59b910`, `0x59b9bd`, `0x59b9c6`, `0x591b34`, `0x5a17d0`, `0x6cd944`, `0x6cd958`, `0x6cd973`, `0x6cd978`
Confidence: verified
Proof: one
Test: `ui::glue_76d240_change_detect_and_scale`

`0x76d5dc mov ecx,2; call 0x58a970` is the one clear in the draw; `0x58a970` calls device slot
`+0x6c`. In `0x59b910` `test al,1` sets `0x4000` and `test al,2` sets `0x100`. The colour is
`[device+0x324]`, stored `0xff000000` at `0x591b34`. `0x58a970` has 14 call sites, flags 1 or 3 at
eleven of them; `0x6cd944 cmp [..+0x28],1` gates `0x6cd958`, which binds the target, before
`0x6cd973 mov ecx,3`.

## Unused model light

The model frame can hold one light of its own, `Model:SetLight`, which the fill callback `0x76d680`
adds to the collector when armed. No glue screen arms it: the only two `SetLight` calls in the
shipped GlueXML, in `CharacterCreate.lua` and `CharacterSelect.lua`, are commented out.

Addresses: `0x76e1e0`, `0x76d7c6`, `0x76d7d9`, `0x71bf90`
Confidence: verified
Proof: one
Test: none

`0x76d7c6 mov eax,[esi+0x384]; test; je 0x76d7de` gates `0x76d7d9`, which adds `+0x324`. The census is
over the 20 files of `GlueXML.toc`; its control finds the live `SetFogColor`, `SetFogNear` and
`SetFogFar` calls of `GlueParent.lua`.

## Scene material flags

In the six `UI_<Race>` scenes, the materials with render-flag bit 0x01, drawn fullbright by
`0x70c190`, are Human 3, 5, 13, 14, 21, 25; Orc 1, 3, 10; Dwarf 25, 27; NightElf 9, 10, 11, 13, 14,
17, 19, 20, 21; Tauren 3, 13, 14, 15, 16; Scourge 16, 17, 18, 20, 21. Tauren's 3 (a sky) and 16
(an additive glow) also carry bit 0x02, so they are exempt from fog. A zero ambient in a scene is
therefore not a black scene: an unlit material multiplies its lighting term by zero and adds the
texture unmodulated.

Addresses: `0x70c190`, `0x70bb00`, `0x70bb0a`, `0x70bb24`
Confidence: verified
Proof: one
Test: none

The material arrays are read from the six `.m2` files (header `+0x84` count, `+0x88` offset, four
bytes each: flags, blend mode). `0x70bb00 test [ecx],1` clears the lit flag `[batch+0x3328]` and
`0x70bb0a` also clears it for the blend modes whose entry at `0x811fa8` is 0. The law that the
lit flag selects is in `models`. Which batches draw each material is not read here.
