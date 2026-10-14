# ui: Model frames, animation, light and fog

What a `Model` frame plays and when, how its two script hooks fire, and how its own light, fog and
alpha reach the scene. The sequence verbs are in `model`; the camera and the paint are in `modelview`.

The animation, the scene's draw lists and the batch states belong to the model system (`models`): a
model frame's scene is advanced and drawn by the same code the world uses, so its sequences, colour and
transparency tracks, texture transforms and particles behave as there.

Roots: `0x76cdc0`, `0x76d120`, `0x76d680`, `0x76e1e0`, `0x76e7d0`, `0x76eca0`, `0x76ed70`

## The idle sequence

A model frame that never calls `SetSequence` plays animation id 0 from the moment its model's data is
resident. If the model has no sequence for id 0 it plays the id of its first sequence. The sequence
starts at variation 0 (not a random one), offset 0, rate 1.0, without a blend, in the primary slot of
key bone 0. A character pane arms Stand again with a random variation (`playermodel`).

Addresses: `0x70ebd0`, `0x710153`, `0x710169`, `0x710174`, `0x71017b`, `0x710187`, `0x710189`, `0x71019b`, `0x711bf0`, `0x711960`, `0x7121a0`, `0x71d5d0`, `0x71d6a0`
Confidence: verified
Proof: one
Test: none

At the end of the load completion `0x70ebd0` marks the instance live (`+0x10 = 1`, `0x71015b`), resolves
animation 0 with `0x711bf0` and asks `0x711960` whether the model has that sequence. The id is 0 when it
does, else the first animation's id from `[[header+0x20]]` (`0x710181` to `0x710187`). The call
`0x7121a0(−1, id, 0, 0, 1.0, 0, 1)` follows (`0x710189` to `0x71019b`: the pushes are primary 1, blend 0,
rate 1.0, offset 0, variation 0). `0x70ebd0` has two callers: `0x71d5d0`, at once when the shared data is
ready, and `0x71d6a0`, from the load drain.

## Re-arming a sequence

`SetSequence(id)` and `SetSequenceTime(id, ms)` start a new arm every call, even for the id that is
playing. The new track starts at `clock − trunc(ms·|1/rate|)` (rate 1.0), ends one sequence length
later times the repeat count, and clears the completion latch; a finished sequence can be played
again. An id the model has no sequence for arms nothing, but it still ends the track that was playing.
Ending an unlatched track calls the completion callback with mode 1, which runs no script.

Addresses: `0x7121a0`, `0x7121b3`, `0x71225b`, `0x712280`, `0x7122a3`, `0x7122bc`, `0x7122d5`, `0x712484`, `0x712553`, `0x712635`, `0x712692`, `0x712750`, `0x71275b`, `0x712795`, `0x76cf50`, `0x76cf60`, `0x76cf80`, `0x76dec0`, `0x76dfc0`
Confidence: verified
Proof: one
Test: none

`0x7121a0` first queues the call when the model is not live (`0x7121b3`). Before it inspects the id it
tests: a done callback set (`+0x70`), the bone's track active (`+0xa4 ≠ −1`), its latch clear
(`+0xc0 = 0`) and a key bone id of −1 or bone 0 (`0x712280` to `0x712297`); then it calls the callback
with mode 1 (`0x7122bc`). An id `jae` past the sequence count returns (`0x712484`). The shortcut that
would keep a playing slot needs a blend and a set latch (`0x712553`); the Lua wrappers pass blend 0, so
every call takes `0x712635` and clears the secondary slot. The block at `0x712692` draws the repeat
count, not an offset jitter. The wrappers push `1, 0, 1.0, ms, −1, seq, −1` (`0x76cf60` to `0x76cf70`;
`0x76cf80`) when `+0x318` is set.

## Completion and OnAnimFinished

A model frame's `OnAnimFinished` handler runs once per arm, when the frame's clock reaches the track's end
(start plus length times repeats), for looping sequences too. Only natural completion runs the handler;
a re-arm that cuts a running track does not. After a clamping sequence ends, the model holds the last
frame as long as the slot stays armed; a looping one keeps looping.

Addresses: `0x76cdc0`, `0x76cdc6`, `0x76cdde`, `0x76cd8a`, `0x711bb0`, `0x719370`, `0x71945c`, `0x719482`, `0x71949a`, `0x71949e`, `0x7194af`, `0x7194bc`, `0x7194fe`, `0x719503`, `0x7074b0`, `0x7075af`, `0x714530`, `0x714585`, `0x7145a1`
Confidence: verified
Proof: one
Test: none

`SetModel` registers `0x76cdc0` as the done callback (`0x76cd8a` through `0x711bb0`). It returns unless
its mode argument is 0 and `+0x3d4` holds a handler (`0x76cdc6` to `0x76cdde`). The per-tick walk
`0x719370` computes the window and fires iff a callback is set, `hi == now` (the clock has reached the
end, `0x71949e`) and the latch is clear (`0x7194af`); it sets the latch (`0x7194bc`) before testing the
key bone and queues the record with mode 0 (`0x7194fe`), which `0x7074b0` drains in the same frame
(`0x7075af`). The sequence flag test at `0x719503` comes later and only widens the event window of a loop.
The time sampler `0x714530` tests flag 1 (`0x714585`): with it, once the clock is at the end
(`0x7145a1`), the time is clamped to the length and is independent of the clock.

## OnUpdateModel

A model frame's `OnUpdateModel` handler runs once per paint of a visible frame that has a model and a
rectangle wider and taller than 2^-22 of the screen, after the viewport and depth clear and before the
residency and camera checks. A model whose data is not yet resident still gets the handler on every
paint, and nothing is drawn. The handler takes no arguments.

Addresses: `0x76d1a0`, `0x76d1a9`, `0x76d1b7`, `0x76d1bc`, `0x76d5ea`, `0x76d603`, `0x702690`, `0x76cc99`, `0x76cc80`
Confidence: verified
Proof: one
Test: none

The root-transform step `0x76d1a0` loads the slot at `+0x3cc` and calls `0x702690` on it (`0x76d1a9` to
`0x76d1bc`) before the first scale computation; the paint reaches it through slot 38 (`0x76d5ea`), ahead
of `0x710450` (`0x76d603`). The only other call of slot 38 is the character pane's camera step
(`modelview#the-character-pane-camera`). `0x76cc80` returns the slot by name, `OnUpdateModel` at `+0x3cc`
and `OnAnimFinished` at `+0x3d4`.

## Frame alpha

A model's opacity follows its frame's own alpha, not the alpha inherited from its parents. The frame
pushes `alpha/255` of the frame's byte at `+0xc8` to the model when the model is set and on every
`SetAlpha`; all five model-frame vtables override slot 10 with the same function.

Addresses: `0x76d120`, `0x76d12a`, `0x76d139`, `0x76d147`, `0x76cd5e`, `0x76cd77`, `0x710cb0`, `0x8026c8`, `0x76a690`
Confidence: verified
Proof: one
Test: none

`0x76d120` calls the base `SetAlpha` (`0x76a690`) and, with a model, passes `movzx [frame+0xc8]` times
1/255 (`0x8026c8`) to `0x710cb0` (`0x76d139` to `0x76d150`). `SetModel` does the same at `0x76cd5e` to
`0x76cd77`. The model folds the value into its instance alpha, which `0x714260` multiplies by the
caller's factor, 1.0 on this path (`models#sequence-time`).

## The frame's light

A model frame carries a light of its own at `+0x324`. A `Model` starts with it disabled, so its model gets
no light and its lit materials draw dark; a `PlayerModel` starts with a directional light of ambient
(0.7, 0.7, 0.7), diffuse (0.8, 0.8, 0.64) and direction (0, 1, 0) that is enabled. The scene's lighting
pass takes the light through the frame's callback only while it is enabled. Materials flagged unlit
ignore it.

Addresses: `0x76c8e0`, `0x76c98c`, `0x76c99c`, `0x76c9a5`, `0x71b4a0`, `0x71b620`, `0x71b6a0`, `0x71b780`, `0x505680`, `0x5056c9`, `0x5056e9`, `0x505761`, `0x50576a`, `0x76d680`, `0x76d7c6`, `0x76d7d9`, `0x71bf90`
Confidence: verified
Proof: one
Test: none

The `Model` constructor builds the light with `0x71b4a0` (`0x76c916`), disables it (`0x71b780(0)`,
`0x76c99c`), makes it a point light (`0x71b620(1)`, `0x76c9a5`) and stores ambient and diffuse (1, 1, 1)
at `+0x354` and `+0x360`. `0x505680` makes it directional (`0x71b620(0)`, `0x5056c9`), writes the direction
(0, −0.70710677, −0.70710677) with `0x71b6a0` (`0x5056e9`), diffuse (`0x3f4ccccd`, `0x3f4ccccd`,
`0x3f23d70a`), ambient (`0x3f333333` ×3) and then the direction (0, 1, 0) (`0x505761`), which stands;
`0x71b780(1)` enables it (`0x50576a`). The callback `0x76d680` calls `0x71bf90` on `+0x324` only when
`+0x384` is nonzero (`0x76d7c6`, `0x76d7d9`) and clears the collector's node hint at `+0x1b0`. A collector
with no light finishes with the direction (0, 0, −1) and zero ambient and diffuse.

## SetLight and GetLight

`SetLight(enabled[, omni, x, y, z, ambientIntensity[, r, g, b], diffuseIntensity[, r, g, b]])` builds a
light on the stack and copies it over the frame's light only when `enabled` is nonzero after truncation.
`SetLight(0, …)` writes nothing and cannot turn a light off, so a `PlayerModel`'s light stays. `omni`
(index 3) picks point (nonzero) or directional; x, y, z are a position or a direction, normalised.
Each intensity scales its colour; if an intensity is under 2^-22 in size or its three components are not
all numbers, its colour is white and the next intensity is read from the following index. Components are
clamped to [0, 1]. `GetLight` returns 7, 10 or 13 values.

Addresses: `0x76e1e0`, `0x76e290`, `0x76e2a3`, `0x76e2cb`, `0x76e2d2`, `0x76e2df`, `0x76e3b9`, `0x76e3fc`, `0x76e408`, `0x76e474`, `0x76e533`, `0x76e538`, `0x76e59f`, `0x76e777`, `0x76cf30`, `0x76f900`, `0x76e7d0`, `0x76e91f`, `0x76e953`, `0x878c00`
Confidence: verified
Proof: one
Test: none

Index 2 must be a number or the usage string `0x878c00` is raised through `luaL_error` (`0x76e295`).
`0x71b4a0` builds the local light (`0x76e2a3`); the enabled flag is `neg; sbb; neg` of the truncated
value (`0x76e2b9` to `0x76e2bd`) into `0x71b780`, and `0x76e2cb` tests the light's own enabled field: zero
destroys the local light (`0x71b610`) and returns 0 values (`0x76e2d2`, `0x76e2df`). The position
(`0x76e3b9`) and direction (`0x76e3fc`) setters are chosen by type. `0x76f900` packs a colour; `edi` is
11 after a full colour and 8 otherwise (`0x76e538`), and the second intensity is read at `edi` (`0x76e59f`).
`0x76cf30` copies 0x1b dwords to `+0x324` (`0x76e777`). `GetLight` pushes `1.0, r, g, b` for each colour
with a component above 0 and a single 0 otherwise (`0x76e91f` to `0x76e953`).

## Fog

A model frame can fog its model. `SetFogColor(r, g, b[, a])` clamps each component to [0, 1] (NaN to 1),
packs `0xAARRGGBB` with `trunc(c·255 + 0.5)` (alpha defaults to 1.0) and arms the fog; `SetFogNear` and
`SetFogFar` store a number as it is; `ClearFog` clears the arm bit only, leaving colour, near and far.
The XML attributes `fogNear` and `fogFar` are clamped at 0 and `FogColor` sets the colour and arms the
fog. The fog applies per material: a material marked unfogged ignores it, and the fog is on only for
`far > near`.

Addresses: `0x76ee60`, `0x76ef28`, `0x76effc`, `0x76f059`, `0x76f062`, `0x76f1e0`, `0x76f282`, `0x76f390`, `0x76f432`, `0x76f540`, `0x76f5c5`, `0x76f2d0`, `0x76f480`, `0x76cbbb`, `0x76cbd2`, `0x76cc0a`, `0x76cc4b`, `0x76cc51`, `0x76d680`, `0x76d6a7`, `0x76d7c1`, `0x71c110`, `0x70baf0`, `0x70bb24`
Confidence: verified
Proof: one
Test: none

`0x76ee60` reads the components at indices 2 to 4 and alpha at 5, each through `fcom 0.0` and `fcom 1.0`
with `test ah,5; jp` (`0x76ef28` to `0x76eff3`), packs them (`0x76effc`) and ors bit 0 into `+0x3a4`
(`0x76f059`) before storing the colour (`0x76f062`). `SetFogNear` and `SetFogFar` store at `+0x3ac`
(`0x76f282`) and `+0x3b0` (`0x76f432`); `ClearFog` is `and [+0x3a4], −2` (`0x76f5c5`). `LoadXML`
(geometry base `edi = frame + 0x24`) stores `fogNear` at `+0x388` and `fogFar` at `+0x38c` with
`fcomp 0.0` forcing a negative value to 0 (`0x76cbbb`, `0x76cbd2`, `0x76cc0a`), and the `FogColor` child
through `0x6f23d0` at `+0x384` with bit 0 of `+0x380` (`0x76cc4b`, `0x76cc51`). The callback `0x76d680`
quantises each colour byte as `trunc(v·255 + 0.5)` clamped, and calls `0x71c110(collector, colour,
near, far)`; the batch step turns fog on for a lit material only if the collector's range reciprocal
`+0x18c` is positive (`0x70bb24`, `models#lighting-and-fog`).

## Texture transform sense

A texture transform turns an image counter-clockwise in a frame with u to the right and v up when the
rotation key's z and w are positive, and clockwise when z is negative. The rotation key is lerped
component by component and never renormalised, so between two keys the matrix is that of a
quaternion shorter than 1, a rotation with a slight shrink. For a z-only key, with `c = 1 − 2z²`,
`s = 2zw`, centre p, translation t and scale k, `u' = p.u + k.u·(u + t.u − p.u)·c − k.v·(v + t.v − p.v)·s`
and `v'` swaps the roles of `s` and `c`.

Addresses: `0x7bddb0`, `0x7bc6a0`, `0x713ea0`, `0x713ec7`, `0x713f05`, `0x713f3c`, `0x715fdb`, `0x716010`, `0x716048`, `0x7161e0`, `0x716384`
Confidence: verified
Proof: one
Test: none

`0x7bddb0` forms `Q` (`models#texture-transforms`) and multiplies it on the left of the matrix; p is
(0.5, 0.5). In `v'` the term in `u` carries `s` and the term in `v` carries `c`. The
element `Q[0][1] = 2xy + 2zw` and `Q[1][0] = 2xy − 2zw` give, for the row vector `(u, v)`,
`u' = c·u − s·v` and `v' = s·u + c·v`: with z = w = √½ the point (1, 0.5) about (0.5, 0.5) goes to
(0.5, 1.0). The key sampler `0x713ea0` copies an interpolation-0 key (`0x713ed2`) and otherwise computes
`(v1 − v0)·f + v0` per component (`0x713f05` to `0x713f3c`) with no normalisation after it.

## Texture wrap flags

A model texture's flag bits 0 and 1 become bits 3 and 4 of the flag word the texture is created with:
bit 3 for the U direction and bit 4 for V. The texture cache keys on the name and that word, so one file
used with different flags is two textures. The remaining bits of the word come from the global default
the model cache holds.

Addresses: `0x71da1a`, `0x71da26`, `0x71da44`, `0x71da4d`, `0x71da54`, `0x71da6c`, `0x71da72`, `0x449d90`
Confidence: verified
Proof: one
Test: none

At `0x71da1a` the word starts as `[[cache + 0x1034]]` in `edi`. `movzx` of the record's flags (`0x71da44`)
shifted left by 3 replaces bit 3 of `edi` through `(x ^ edi) & 8` (`0x71da4d`), then bit 4 through
`(x ^ edi) & 0x10` (`0x71da54`); the word is passed to `0x449d90` with the texture name (`0x71da72`). A
record with a name length of 1 or less is skipped.

## Passes and device state

A model frame draws pass 0 and pass 1 of its scene once each, which together hold every batch of its
model exactly once and clip none. The frame creates its model with flags 5, which turns off the model's
depth-prime twin. The scene draws into the current target, the back buffer, between the interface's own
quads. The interface's 2-D layers neither write nor test depth, so the model starts from a depth buffer
cleared inside its rectangle and cannot hide, or be hidden by, a later 2-D layer.

Addresses: `0x76ccf5`, `0x707350`, `0x70ea53`, `0x707a48`, `0x76d629`, `0x76d639`, `0x708900`, `0x70b360`, `0x707971`, `0x70c0b1`, `0x76fb00`, `0x76fc14`, `0x70b3dd`, `0x70b3fd`
Confidence: verified
Proof: one
Test: none

`0x76ccf5` pushes 5 as the flags of `0x707350`, which `0x70ea53` stores at `+0x4`; the depth-prime gate
`0x707a48` tests bit 0 and takes the branch without the twin. `0x708900(scene, 0)` and `(scene, 1)` are the
two calls (`0x76d629`, `0x76d639`); pass 2 is not drawn. The widget's clip-plane vector is zero (the
collector is zeroed each frame), so a command straddling it is listed in both lists but the plane
`0x70c0b1` arms rejects no vertex. `0x70b360` installs an identity view (`0x70b3dd`) and `LEQUAL`
(`0x70b3fd`) for the pass. The batch drain `0x76fb00` draws a layer's quads and strings before it runs the
callbacks (`0x76fc14`), so a model frame's scene draws over its BACKGROUND, BORDER and ARTWORK regions
(`model#where-the-scene-draws`).

## AdvanceTime and ReplaceIconTexture

`Model:AdvanceTime()` validates its receiver and does nothing; it reads no argument and returns no value.
`Model:ReplaceIconTexture("texture")` takes a texture file name, or a number rendered as its decimal
text, and swaps that texture for the model's replaceable texture, type 14, by reference. Without a model
the call is dropped, not stored, and a later model does not get it. For a model whose data is not
resident the pair is queued and applied when it loads. Calling it with anything else raises
`Usage: %s:ReplaceIconTexture("texture")`. `SetModel` or `ClearModel` destroys the swap with the model.

Addresses: `0x76eca0`, `0x76ed58`, `0x76cfb0`, `0x76ed70`, `0x76edf5`, `0x76ee13`, `0x76ee21`, `0x76cfe0`, `0x76cfe9`, `0x710ec0`, `0x710ecd`, `0x71104f`, `0x7101d6`, `0x710ef0`, `0x6f3510`, `0x6f3690`, `0x84c7f0`
Confidence: verified
Proof: one
Test: none

`0x76eca0` validates the receiver (`0x76ed58`) and calls `0x76cfb0`, which is `mov eax,1; ret`; every
`ret` follows `xor eax,eax`. `0x76ed70` tests `lua_isstring` on index 2 (`0x76edf5`), which accepts a
string or a number, and pushes type 14 (`0x76ee13`, the only producer of type 14) to the forwarder
`0x76cfe0`, which returns at once when `+0x318` is null (`0x76cfe9`). `0x710ec0` queues the pair on
`+0x3c` when the model is not resident (`0x710ecd`, `0x71104f`), replayed at `0x7101d6`, and otherwise
replaces the handle in `[model+0xa4][i]` for every texture of the type, and the ribbon and particle
slots that use it (`0x710ef0`). A slot not yet replaced is 0, so the batch binds no texture. The usage
error names the frame, or `<unnamed>` (`0x84c7f0`).
