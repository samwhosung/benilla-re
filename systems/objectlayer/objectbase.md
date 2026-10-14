# objectlayer: the base object

What every object does for itself: the members `CGObject_C` adds to the field array, its model and
world node, the fade and scale ease that run each frame, the ground conform, the highlight light
bits, and the selection circle's textures. The code is Object_C.cpp, `0x613610` to `0x6151e0`, with its +∞ initializer `0x613ad0` inside the run and its block of static
initializers, `0x6151f0` to `0x615700`, after it.
The vtable slots are in [classes](classes.md); the manager that links and frees objects is
[manager](manager.md).

Roots: `0x6139a0`, `0x613b40`, `0x613af0`, `0x6145e0`, `0x614640`, `0x614a90`, `0x614cd0`, `0x6146d0`

## Constructor and members

`0x6139a0(fields, copy)` zeroes the base members, stores the base vtable `0x803640` and links the
object into the manager through `0x4643b0`. The members, by offset: the manager links at `+0x1c`,
`+0x24`, `+0x38` and `+0x40`; six empty watcher lists of 12 bytes from `+0x48`, one per base
field; the scale `+0x90` and its factors `+0x94`, `+0x98`, `+0x9c`, all 1.0; the scale ease start
`+0xa0` and from-value `+0xa4`; the missile and effect lists `+0xac`, `+0xb0`, `+0xb4`; the light
bits `+0xb8`; the height `+0xbc`, 1.0; the bounds `+0xc0` to `+0xd4`; the models `+0xd8`, `+0xdc`;
the world node `+0xe0`; the flags `+0xe4`; the fade `+0xec` to `+0xfc`; the master alpha `+0x100`,
1.0; and the smoothed up vector `+0x104` to `+0x10c`, (0, 0, 1).

Addresses: `0x6139a0`, `0x613ab1`, `0x4643b0`, `0x613610`, `0x613710`, `0x613810`, `0x6136e0`, `0x6137e0`, `0x613850`, `0x613750`
Confidence: verified
Proof: one
Test: none

The stores run `0x6139a6` to `0x613aa9`; `0x613a13` to `0x613a2b` store 1.0 in `+0x90` to `+0x9c`,
`+0xa4` and `+0xbc`; `0x613a97` stores 1.0 in `+0x100` and `0x613aa9` in `+0x10c`. The list node
helpers `0x613610`, `0x613710` and `0x613810` unlink a node whose link has bit 0 as the end
mark. `0x6136e0`, `0x6137e0` and `0x613850` return the node a link points at. `0x613750(n)`
resizes an array of 8-byte entries with `SMemReAlloc`, allocating anew and copying when it fails.

## Destruction

`0x613b40` runs when an object is freed. It restores the base vtable, releases the world node
(`0x6140a0`), clears the loaded, done and event callbacks of both models and closes them, frees
every missile on `+0xac` and `+0xb0` (`0x6140c0`, `0x6140f0`), unlinks and releases every effect
on `+0xb4` (`0x614120`), empties the six watcher lists, and unlinks the manager links `+0x40`,
`+0x38`, `+0x24` and `+0x1c`.

Addresses: `0x613b40`, `0x613b4f`, `0x6140a0`, `0x6140c0`, `0x6140f0`, `0x614120`, `0x61ccd0`, `0x6203a0`, `0x6210e0`
Confidence: verified
Proof: one
Test: none

`0x613b4f` calls `0x6140a0`, which calls `ReleaseWorldObject` and zeroes `+0xe0`. The model
blocks at `0x613b54` and `0x613b94` call `SetLoadedCallback`, `SetDoneCallback` and
`SetEventCallback` with 0 and then `HandleClose2`. `0x6140d0` and `0x614100` loop `FreeMissile`
while the list head is nonzero; `0x614130` calls `CEffect::Unlink` and `CEffect::Release` on
each effect.

## Models and the node

An object holds up to two models, the body at `+0xd8` and an overriding one at `+0xdc`; every
user takes `+0xdc` when it is set. `0x613cf0(model)` swaps the body: it takes a reference,
releases the old one, and sets `0x613d70` as the loaded callback. `0x613d80(model)` does the
same for `+0xdc`, and clearing it refreshes the bounds from the body. A node, when one exists,
is told the new model. `0x613e10` creates the model from the slot 10 path when there is none, then
the node: kind 11 for a game object (type bit 5), 2 for a dynamic object (bit 6), else 0.

Addresses: `0x613cf0`, `0x613d70`, `0x613d80`, `0x613e10`, `0x613e59`, `0x613eb1`, `0x613ec6`, `0x613ed2`, `0x670db0`, `0x6716f0`, `0x6151b0`, `0x615150`, `0x614eb0`, `0x614fe0`
Confidence: verified
Proof: one
Test: none

`0x613d70` is the loaded callback: it calls the object's slot 13 with the model, and `0x6151b0`
(the base slot 13) calls `0x615150` when that model is the current one. `0x615150` copies the
model's bounding box to `+0xc0`, stores its height, `max z − min z`, at `+0xbc` and calls
`0x613ef0(1)`. `0x613e59` skips node creation while `0x468570` is nonzero; an object that already
has a node prints `OBJECTALREADYACTIVE|0x%016I64X`. `0x613ed2` calls `CreateWorldCallbackObject`
with the model, callback `0x481540`, the GUID and the kind; `0x6716f0` swaps a node's model.
`0x614eb0` is true when the current model has loaded; `0x614fe0` (slot 30) when it is ready.

## The node transform

`0x613ef0(flag)` rebuilds the node's matrix: a translation to slot 5's position, a rotation about
Z by slot 6's facing, and a uniform scale of `+0x90 · +0x94 ·` (`+0x9c` with a second model, else
`+0x98`). With a loaded body it takes the collision box's centre as the node's pivot. It passes the
matrix, the pivot, the bounds `+0xc0` and the flag to `SetWorldObjectBounds`. `0x615030` calls it
with 1 for one object, and `0x615020` enumerates every object into it.

Addresses: `0x613ef0`, `0x613f82`, `0x613fb0`, `0x6717d0`, `0x615020`, `0x615030`, `0x468380`
Confidence: verified
Proof: one
Test: none

`0x613f82` builds the translation with `TranslateMatrix` and `0x613fb0` the rotation with
`RotateMatrix` about (0, 0, 1). The centre is `(c4 + c1, c3 + c0, c2 + c5)` of the collision box
halved by `ScaleVector3(0.5)`; without a loaded body it is (0, 0, 0). `0x615030` resolves the GUID
with mask 1 and returns 1 so the enumeration goes on.

## Showing an object

`0x613af0` sets bit 0x4 of `+0xe4`, then fades to slot 27's alpha over 2000 ms when slot 26 is
true, else at once, and sets `+0x90` to slot 7's scale with the ease off (`+0xa0` = 0). The base
slots return true and 1.0, so every object appears over 2 s unless its class says otherwise.
`0x614640` (reactivate) clears bit 0x1, sets bit 0x2, does the same and recreates a missing node.
`0x6145e0` (deactivate) conforms and hands the node to `FadeOutModel` at the current alpha, then
zeroes the light bits and the fade and sets bit 0x1.

Addresses: `0x613af0`, `0x613b0d`, `0x614640`, `0x6145e0`, `0x614e90`, `0x614ea0`, `0x614ef0`, `0x614f40`, `0x614e80`, `0x6146ab`, `0x672df0`
Confidence: verified
Proof: one
Test: none

`0x613b0d` is `neg eax; sbb eax,eax; and eax,0x7d0`. `0x614e90`, `0x614ea0`, `0x614ef0` and
`0x614f40` return bits 0x4, 0x1, 0x2 and 0x10 of `+0xe4`; `0x614e80` clears bit 0x2. The update
reads bit 0x4 to tell a created object from one it has seen (`update`).

## The alpha fade

`0x614f80(alpha, ms)` starts a fade. Unless `ms` is nonzero and `alpha` already is the target
`+0xfc`, it stamps the time at `+0xec`, `ms` at `+0xf0`, the current alpha `+0xf4` into `+0xf8`
and `alpha` into `+0xfc`; with `ms` 0 it also sets `+0xf4` to the target. Each frame the fade
moves `+0xf4` from `+0xf8` to `+0xfc` by `t³`, `t` = age over `ms` between 0 and 1, and the
model's alpha becomes `+0x100 · +0xf4`. At the end it sets `+0xf4` to `+0xfc` and `+0xf0` to 0.

Addresses: `0x614f80`, `0x614f91`, `0x614f9c`, `0x614b34`, `0x614b79`, `0x614b82`, `0x614baa`, `0x614bba`, `0x710cb0`, `0x6151e0`
Confidence: verified
Proof: one
Test: none

`0x614b1c` is the completion test, `now − start − ms ≥ 0`. `0x614b4c` to `0x614b79` clamp
`t` to 0 below zero and 1 above one, else cube it; `0x614b82` computes
`(to − from)·e + from`, and `0x614baa` the model's alpha. `0x6151e0(a)` stores `a` in `+0x100`. An
object with no model skips the fade and the alpha store, but not the later steps. The equality test of `0x614f80` is
`fcomp` with `test ah, 0x44` and `jnp` (`0x614f91` to `0x614f9c`): an unordered compare, a NaN on either side, is not equal and restarts the fade.

## The frame update

Slot 14, `0x614a90`, is the base per-frame step. If the object's GUID is the target GUID
`0xb4e2d8` and not the active player, it draws the selection circle through slot 12 while the
`ObjectSelectionCircle` variable is nonzero. It then runs the alpha fade, the scale ease and the
missile fade.

Addresses: `0x614a90`, `0x614aa5`, `0x614ac8`, `0x614ada`, `0x614bbf`, `0x614c1a`, `0x614c2c`, `0x614c4e`, `0xb4e2d8`, `0xc4daf8`
Confidence: verified
Proof: one
Test: none

The scale ease, from `0x614bbf` while `+0xa0` is set and past: `e = (now − +0xa0)·0.0005·π`;
under 2000 ms, `+0x90 = (slot 7 − +0xa4)·(1 − cos e)/2 + +0xa4`, else `+0x90` is the field
scale and `+0xa0` is 0. The head of the list `+0xb0` is freed while its time `+0x50` has
passed (`0x614c36`); each remaining missile gets alpha `u³`, `u` = `(+0x50 − now)·0.0005`
clamped to 0 to 1, through `SetAlpha`.

## The scale change

The watcher `0x614a00`, on base field 4 (`OBJECT_FIELD_SCALE_X`, offset 0x10, any object), calls
`0x615100(old)` with the previous value. It stamps `+0xa0` with the time and `+0xa4` with the old
scale, which starts the ease above, and if the object is the camera's follow target, tells the
interface (`0x48ec90`).

Addresses: `0x614a00`, `0x615100`, `0x6146d0`, `0x614a2b`, `0x48ec90`
Confidence: verified
Proof: one
Test: none

`0x6146d0` registers the watcher (`ClntObjMgrSetTypeMirrorHandler(0, 0x10, 4, 0x614a00)`) and
`0x614a40` removes it. `0x615100` compares the object's GUID with the camera's follow GUID at
`+0x88` of `GetActiveCamera`.

## Ground conform

Slot 15, `0x614cd0(dt)`, tilts the model to the ground. When slot 8's normal has `z ≥ 0.3572124`,
the smoothed vector `+0x104` becomes `normal + (+0x104 − normal)·0.0018^dt`; a steeper normal
leaves it as it was. The model's base transform is then set through `SetBaseTransformConformed`
with the position (slot 5), the facing (slot 17), the model scale and `+0x104`.

Addresses: `0x614cd0`, `0x614ce2`, `0x6146c0`, `0x6151b0`, `0x80c6a0`, `0x80c6a8`, `0x7106c0`, `0x615070`
Confidence: verified
Proof: one
Test: none

`0x614ce2` is the 0.3572124 test (`fcomp`, `test ah,1`: below it jumps over the update). The
base of the power is the double at `0x80c6a0`, 0.0018, and `dt` the stack argument (0 from
`0x6145e0`, which sets the vector to the normal outright). `0x6146c0` (slot 16) returns its argument's
bit 0. `0x615070(matrix, point)` fills an identity matrix and rotates it about Z by
`−HeadingBetweenPoints(camera, point)`, the yaw a projected decal needs.

## Selection circle textures

`0x6146d0` registers the variable `ObjectSelectionCircle` (default "1"), makes a 64 by 8 texture
filled by the generator `0x6147f0`, and loads `Textures\UnitSelectTexture.blp`. `0x614e00` sets
the circle's render state: blend mode 3, cull, fog and depth write off, the blp on texture stage
0 and the ramp on stage 1. The ramp's alpha rises from 0 to 1 over the first sixth of a row, holds at 1 to five sixths, and
falls to 0 over the last sixth.

Addresses: `0x6146d0`, `0x6147f0`, `0x614a40`, `0x6149e0`, `0x614e00`, `0xc4dac4`, `0xc4dafc`, `0xc4daf8`, `0xc4dae4`
Confidence: verified
Proof: one
Test: none

`0x6147f0` is a three-mode texture callback: mode 0 grows the dword buffer `0xc4dae4` to `rows · width`, mode 1
returns the pitch `4·width` and the buffer after filling it, mode 2 frees it. Mode 1 fills each
row, for x in 0 to width−1, with `v = 12·x/(width−1)`: `v/2` below 2, 1 below 10, `(12 − v)/2`
after, clamped at 0, times 255 in the alpha byte over white (`0x614932`). `0x614e00` returns unless
both textures exist.

## Highlight light bits

`0x614550(bit)` sets the bit in `+0xb8` and gives the model an emissive colour from the light
state's byte triple at `+0x178`. `0x6144f0(bit)` clears the bit and, when no bit is left,
resets the model's emissive colour to black. The interface uses bit 0 for the locked target and
bit 1 for the object under the cursor.

Addresses: `0x614550`, `0x6144f0`, `0x6d48b0`, `0x710d40`, `0x493701`, `0x4945ec`, `0x492916`, `0x4939a7`
Confidence: verified
Proof: one
Test: none

The colour is the three bytes at `+0x178`, `+0x179` and `+0x17a` of `GetLightState()`, each times
1/255, passed to `SetEmissiveColor` on the `+0xdc` model, else `+0xd8`. The callers pass 0 at
`0x493701` (select) and `0x4939a7` (clear), and 1 at `0x4945ec` and `0x492916`.

## Stopping effects by kit

`0x614210(visual, kind)` stops every effect on `+0xb4` that was made from one visual kit of a
`SpellVisual` row. The kind picks the row's kit: kind 0 `+0x8`, 1 `+0xc`, 2 and 3 `+0x10`, 4 `+0x4`;
any other kind, or a missing row or kit, does nothing. An effect stops when its kit record has
the chosen kit's id. `0x6141f0` stops the effect passed when its second argument is 0.

Addresses: `0x614210`, `0x6142a1`, `0x6142b9`, `0x6141f0`, `0x6203e0`, `0x6ec1e0`
Confidence: verified
Proof: one
Test: none

`0x614210` indexes the `SpellVisual` store, calls `GetSpellVisualRecord` (`0x6ec1e0`) and jumps
through the table at 0x6142c8 (cases `0x61425c`, `0x614261`, `0x61427b`, `0x61427b`,
`0x614257`) to load the kit index, which indexes the kit store. `0x6142a1` walks `+0xb4` by the
link at `+0x8c` and calls `CEffect::Stop` (`0x6203e0`) on an effect whose kit record `+0x1c`
holds the same first dword as the chosen kit. Its callers are `0x60d200` and `0x60d20a`.
