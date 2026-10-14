# uicamera: the in-engine cinematic

How a cinematic sequence plays in the world: the sequence driver that steps through the shots of a
`CinematicSequences.dbc` row, the shot camera model the camera object loads and places, and what the shot
does and does not control.

This is `Camera.cpp` and the `GameUI.cpp` driver that calls it. The world camera maths are `uicamera/driver`
and `uicamera/solver`; the way the loading screen picks a first sequence is `glue/flow`; the opcode that starts a
cinematic is `objectlayer/playermessages`.

Roots: `0x50c870` (load a shot), `0x50c740` (arm the shot camera), `0x50f060` (per-frame step), `0x50ca50` (release), `0x50d5d0` (look-at)

## Starting a sequence

`0x48ecd0(id)` starts the sequence with the row `id` of `CinematicSequences.dbc`. It zeroes the six words from
0xb4e264: the row, the sequence sound, the shot index, the shot row, the narration sound and the music flag. An id
outside the table leaves the row null. The row's sound (`+0x04`) plays unless the CVar `EnableMusic` exists and is 0. The first shot is
the `CinematicCamera.dbc` row named at `+0x08`. The music is paused unless it already was. The in-cinematic word `0xb4e310` becomes 1, the hardware cursor
is hidden with `0x58b590(0)`, and when the active player resolves `0x48edd0` is scheduled after 0.25 seconds.

Addresses: `0x48ecd0`, `0x48ed70`, `0x48edd0`, `0x58b590`, `0x4c0d10`, `0x703e50`, `0x4603b0`, `0x460410`
Confidence: verified
Proof: one
Test: none

`0x48ecd0` loads the row from the store `0xc0deb8` (highest id `0xc0debc`) at `0x48ecee` and the CVar `EnableMusic` (0x835b6c) through `0x63de30` at `0x48ed04`; `0x48ed17` tests its value at `+0x28`. `0x48ed23` plays the sound and `0x48ed2e` stores its handle at `0xb4e268`. `0x48ed46` loads the shot row from `0xc0decc` (highest id `0xc0ded0`). `0x48ed70` calls `0x460410` and
stores `0xb4e278` as 1 when music was not suppressed, then `0x48ed83` calls `0x4603b0(1)`. `0x48eda5` resolves the player with the player mask 0x10 and `0x48edc5` schedules the callback. `0x48edd0` signals the event 370 (`CINEMATIC_START`) and calls `0x48edf0`; when that fails it ends the cinematic. The world-enter path `0x490a5b` schedules `0x48edd0` the same way when the word is already set.

## Playing a shot

`0x48edf0` starts the shot named by the shot row. It calls `0x50c870` with the model path (`+0x04`), the origin (`+0x0c` to `+0x14`), the facing (`+0x18`) and the
completion callback `0x48efc0`, and fails when the model does not load. When the old camera position lies farther than 30 yards from the shot's eye it asks the
world to prepare that area with `0x66fc80`. It then restarts the camera's scene clock with `0x50ca10` and plays the shot's own sound (`+0x08`) into `0xb4e274`.

Addresses: `0x48edf0`, `0x50c870`, `0x50ca10`, `0x66fc80`, `0x458a40`, `0x4c1280`
Confidence: verified
Proof: one
Test: `ui::dist2_48edf0`

`0x48edfe` loads the shot row from `0xb4e270` and `0x48ee55` calls `0x50c870`. `0x48ee82` and `0x48ee8e` read the eye (property 7) of camera 0 of the model; `0x48eeb5` to `0x48eec1` compare the squared distance with 30 squared (0x804554), and `0x48eed3` calls `0x66fc80` when it is larger. `0x48eee9` calls `0x50ca10`; `0x48ef2e` plays the shot sound with `0x458a40`. `0x48ef43` calls `0x4c1280` with a duration of 0.25 (0x804550).

`ui::dist2_48edf0` runs the slice `0x48ee93` to `0x48eec7`, the distance test.

## Stepping and ending

When a shot's animation finishes, `0x48efc0` schedules `0x48efe0` after 0.25 seconds. `0x48efe0` stops the narration, adds 1 to the shot index and, below 8, plays the shot whose camera id is at `+0x08 + 4 * index`; with none left it ends the cinematic. `0x48f050` skips: it stops the narration and
schedules the end. `0x48f080` ends it: it signals event 371 (`CINEMATIC_STOP`), prepares the area under the camera's target when the camera is
over 30 yards from it, releases the shot, stops the sounds, restores the music, shows the cursor, clears the word and rejoins the zone chat channels when the player is not a ghost.

Addresses: `0x48efc0`, `0x48efe0`, `0x48f050`, `0x48f080`, `0x50ca50`, `0x49a210`, `0x490a80`, `0x48c930`
Confidence: verified
Proof: one
Test: `ui::dist2_48f080`

`0x48efc0` acts only when its second argument is 0 (`0x48efc6`). `0x48f007` stores the new index at `0xb4e26c`, `0x48f004` stops at 8 and `0x48f00e` indexes `+0x08` by it. `0x48f08e` signals 0x173. `0x48f0b7` resolves the camera's target (`+0x88`) with the object mask 1; `0x48f0f6` compares with 30 squared (0x804558) and `0x48f118` calls `0x66fc80`. `0x48f11f` releases the shot. `0x48f183` clears the word. `0x48f1bd` to `0x48f1d0` call `0x49a210` unless bit 20 of the player's flags is set. `0x490b69` to `0x490bbb`, in the leave-world path `0x490a80`, do the same teardown when the word is set. `Script::InCinematic` (`0x48c930`) returns 1 while the word is set and nil otherwise.

`ui::dist2_48f080` runs the slice `0x48f0cf` to `0x48f102`, the distance test.

## What else reads the cinematic word

The word `0xb4e310` has ten references: the start (`0x48ed8a`), the end (`0x48f183`), the leave-world clear (`0x490bac`), the boot initialiser (`0x48f630`), the Lua test (`0x48c930`) and four readers. The world-enter path `0x490a5b` and the leave-world test `0x490b69` are the sequence's own. The zone text path `0x49491c` and the player-update path `0x5ff565` skip `0x49a210`, the zone chat channel rejoin, while it is set. The incoming trade request `0x4bf793` takes the decline branch, which sends `0x5d4150`, while it is set.

Addresses: `0x49491c`, `0x5ff565`, `0x4bf793`, `0x4bf720`, `0x5d4150`
Confidence: verified
Proof: one
Test: none

`0x49491c` to `0x494931` skip the call on the word; `0x5ff565` to `0x5ff56e` likewise. `0x4bf793` to `0x4bf7b2` choose `0x5d4150` over the begin-trade sender and skip the chat message at `0x4bf7e1`. The dispatcher and its other legs are `objectlayer/trade`.

## Loading a shot

`0x50c870(path, origin, facing, callback, a, b)` makes the shot model and places it. It releases any shot (`0x50ca50`), creates the model from the path in the camera's scene, stores it at `+0x48` and registers the done callback. The base matrix is a rotation by the facing about the vertical axis with the origin in its last row, kept at `+0x54`,
expanded to 4 by 4 and installed in the model with `0x710620`. A local point `v` goes to `origin + Rz(facing) * v`, where `Rz` turns x toward y. A model not yet loaded leaves the arm bit (`+0x90` bit 2) clear for the first frame to set.

Addresses: `0x50c870`, `0x50ca50`, `0x7bb990`, `0x7bbab0`, `0x7bb860`, `0x7bae60`, `0x7bca80`, `0x710620`, `0x711bb0`, `0x707350`
Confidence: verified
Proof: one
Test: none

`0x50c8f9` to `0x50c927` make the identity block at `+0x54`; `0x50c92a` calls `0x7bb990` with the origin, which adds it to the last row; `0x50c950` calls `0x7bbab0(facing, axis (0, 0, 1), 1)`, which builds the Rodrigues matrix with `0x7bb860` (`7bb926` gives `c`, `0x7bb931` gives `s`, `0x7bb944` gives `-s`) and composes it with `0x7bae60`, the rotation on the left. `0x50c955` to `0x50c9b8` copy the twelve floats into a 4 by 4 with a 0 after each three and a 1 last; `0x50c9bf` installs it. `0x7bca80` reads the matrix with a stride of 0x10 per term of one output component, so `x' = c x - s y`. `0x50c9ca` tests whether the model is loaded; `0x50c9f2` arms at once when it is.

## Arming and the per-frame step

`0x50c740` arms the shot camera: it sets the arm bit, stamps the clock `+0x4c`, sets the scene time to 0, starts animation 0 of the model, links it for animation, runs one animate pass, takes camera 0 of the model into `+0x50` and applies the first view. `0x50f060` runs each frame while a model is held: if the arm bit is clear it waits for the model to load and arms; then it advances the scene by the milliseconds since `+0x4c`, animates it, reads the camera's eye (property 7), look-at (property 8) and roll (property 5), stores the clock and applies the view with `0x50d5d0`. `+0x50` is non-null exactly while a shot is armed.

Addresses: `0x50c740`, `0x50f060`, `0x50c860`, `0x713590`, `0x7abf80`, `0x7ac0b0`, `0x7074b0`, `0x707680`, `0x7121a0`
Confidence: verified
Proof: one
Test: none

`0x50c749` sets bit 2 and `0x50c759` stores the clock. `0x50c77c` calls `0x7121a0` with the animation arguments; `0x50c7b7` takes the camera with `0x713590` and `0x50c7e8` stores it. `0x50c815`, `0x50c826` and `0x50c833` read properties 7, 8 and 5. `0x50f0b1` to `0x50f0b8` advance the scene and `0x50f0d8` to `0x50f0df` animate it; `0x50f11d`, `0x50f12e` and `0x50f13b` read the properties; `0x50f151` calls `0x50d5d0`. `0x511bf0` is the only caller of `0x50f060`. `0x50c860` answers whether `+0x50` is set; its one call site, `0x48310b`, switches the sound listener to the camera (`sound`).

## Applying the view

`0x50d5d0(eye, target, roll)` copies the eye to `+0x08`, forms the unit vector from eye to target, keeping the vector as it is when its length is under 2^-22, and builds the basis from that vector and the roll angle through `0x50a930`. The camera's view therefore comes from the shot's eye, look-at and roll alone.

Addresses: `0x50d5d0`, `0x50a930`
Confidence: verified
Proof: one
Test: `ui::cam_50d5d0`

`0x50d5df` to `0x50d5f0` subtract the eye from the target, `0x50d609` takes the length and `0x50d60f` compares it with 2^-22 (0x8029d4). `0x50d651` takes the sine and cosine of the roll and `0x50d65c` to `0x50d66a` copy the eye. `0x50d685` calls `0x50a930`. The twin `0x50d540`, which has no roll, has no caller.

`ui::cam_50d5d0` runs `0x50d5d0` whole, with the basis builder `0x50a930` stubbed; the test reads the direction and the roll from its arguments.

## What the shot leaves alone

A shot's camera record carries a field of view, a near clip and a far clip, and the loader stores them in properties 4, 3 and 2. Nothing reads them back: the eight sites that read a property by id take 7, 8 and 5 only. While a cinematic plays the picture uses the world camera's field of view (`uicamera/transitions#field-of-view`), which is 90 degrees plus the drunk bias unless set otherwise, and the near and far clips the CVars `nearclip` and `farclip` give.

Addresses: `0x70f336`, `0x70f346`, `0x70f356`, `0x7ac3a0`, `0x7ac640`, `0x7ada40`, `0x511bc0`
Confidence: verified
Proof: one
Test: none

The sites that pass a property id are, for `0x7abf80`: `0x46c4fa`, `0x48ee8e`, `0x50c815`, `0x50c826`, `0x50f11d`, `0x50f12e`, `0x52558b`, `0x52559f`, `0x76d411`, `0x76d421`; for `0x7ac0b0`: `0x50c833` and `0x50f13b`. The ids 2, 3 and 4 appear only at the setters `0x70f336`, `0x70f346`, `0x70f356`. `0x7ac640` reads the field of view and clips directly, and its only path is `0x7ada40`, called from `0x5255d6` (the portrait bake) and `0x76d42d` (the model widget). `0x511bdc` and `0x511be8` stamp the clips from the CVars each frame.
