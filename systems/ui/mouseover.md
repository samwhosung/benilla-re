# ui: the world under the pointer

What the interface does with the pointer over the 3-D world: mouselook, the published mouseover object and
its tooltip, and what a click on the world does with the object, the ground or nothing, and with a payload on
the cursor.

This is the world half of `WorldFrame.cpp` and `GameUI.cpp`. The frame-side rules are `ui/mouse`; the pointer
picture the same hover sets is `ui/worldcursor`; the command bits and the click-versus-drag gate that start a click are
`input`. The GameTooltip itself is not documented here.

Roots: `0x482090` (publish gate), `0x492890` (publish), `0x483e80` (enter mouselook), `0x481f60` (click),
`0x492ce0`, `0x492c90`, `0x492d30` (the legs), `0x4925d0` (select), `0x492820` (interact)

## Mouselook

Entering mouselook, `0x50fe30` sets bit 0 of the camera's mode word at `+0x90`, hides the hardware cursor
(`0x58b590` with 0), switches the device to relative mouse mode (`0x420540` with 1, 0) and calls `0x493070`.
That sets bit 1 of the world frame's word at `+0x38c` (`0x483e80`), hides every nameplate (`0x60f830` with 1),
clears a published mouseover, and then makes the world frame the hovered frame. Leaving, `0x50fdd0` clears the mode bit,
shows the cursor and calls `0x493090`, which clears the world frame bit and shows the nameplates again
(`0x483e70`) and asks for a new hover pick. Bit 1 of `+0x38c` is read at one place, the classifier
(`ui/worldcursor#classifying-a-hovered-object`).

Addresses: `0x50fe30`, `0x50fdd0`, `0x493070`, `0x493090`, `0x483e80`, `0x483e70`, `0x60f830`, `0x58b590`, `0x420540`, `0x764dc0`
Confidence: verified
Proof: one
Test: none

`0x50fe30` is reached through the thunk `0x50f930`, called from the button-down handler (`0x51491f`) and the
`ToggleMouseMove` command (`input#mouse-commands`). `0x50fe5d` calls `0x493070`, `0x493079` calls `0x483e80` and `0x493085`
`0x764dc0(manager, world frame)`. `0x483e8f` ors 2 into `+0x38c`; `0x483e9a` calls `0x60f830` with `cl` 1. `0x483e70` ands the bit
out and tail-jumps to `0x60f830` with `cl` 0. `0x4930a0` stores 1 at `+0x1100` of the frame manager, the re-pick flag.

## Mouselook and the pointer feed

In relative mode the window procedure posts only mouse deltas, never absolute positions. The frame manager's
hover pick therefore never runs from the event pump, no frame gets `OnEnter` or `OnLeave`, and the hover frame cannot
change. The deltas go to the hook at `+0x112c` (`0x492c00`), which turns the camera. A hover pick still runs once when
the re-pick flag is set, at the point the pointer froze at; the nameplates are out of the lists by then, so it
cannot pick one. The world pick `0x481190` runs on every update but its result is not published.

Addresses: `0x42d31c`, `0x42d34a`, `0x42d394`, `0x7660d0`, `0x766250`, `0x492c00`, `0x765650`, `0x481190`, `0x4828da`
Confidence: verified
Proof: one
Test: none

`0x42d31c` reads the relative-mode word `0x884e5c`; the relative arm posts ring kind 0xc (`0x42d34a`), the
absolute arm kind 0xa (`0x42d394`) (`core/input#mouse-modes`). `0x766250` serves the delta category and calls the hook;
`0x492c00` answers 1 only for the delta tag 0x400500cb. `0x7657a1` runs the pick when `+0x1100` is set,
with the saved record, so the zero-motion filter of `0x7660d0` does not drop it.

## Publishing the mouseover

`0x482090(guid)` is the one gate: it compares the GUID with the pair cached at `+0x340`, `+0x344` of the world
frame, returns 0 when they are equal and otherwise calls `0x492e70` with the new and the previous GUID, stores
the new pair and returns 1. Every publish from the world goes through it. The publisher `0x492890(new, previous)`
is not gated: when entered, it rebuilds. So the tooltip is rebuilt only when the mouseover object changes, and
hiding or re-owning the GameTooltip from a script does not bring it back while the pointer rests on the same
object. Leaving the world frame and entering mouselook zero the cached pair, so the next pick publishes again.

Addresses: `0x482090`, `0x492e70`, `0x492890`, `0x483b10`, `0x483e80`, `0xb4e2c8`
Confidence: verified
Proof: one
Test: none

`0x4820ad` and `0x4820b1` compare the pair and `0x4820b5` is the no-op exit. `0x4820cf` calls `0x492e70`, which spreads its
record into `0x492890`'s four arguments; `0x4820d4` and `0x4820db` store the pair. The callers of `0x482090` are in the
world frame: `0x48186e`, `0x481880`, `0x4821ec`, `0x4828e7`, `0x4829e0`, `0x4829f3`. The GameTooltip's own hide, owner and
fade code never writes `0xb4e2c8` or `+0x340`.

## The publisher

`0x492890(newGuid, previousGuid)` needs an active player. It first tears down: if either GUID names an old
object it zeroes the published mouseover `0xb4e2c8`, fades the GameTooltip out (`0x530ae0`) and clears the old
object's highlight (`0x6144f0` with 1). It then stores the new GUID, finds the object, sets bit 1 of its highlight
mask (`0x4945e0` and `0x614550`) and picks by the object's type mask: a unit (9, 0x19), a game object (0x21) or a
corpse (0x81); any other type returns.

Addresses: `0x492890`, `0x530ae0`, `0x6144f0`, `0x4945e0`, `0x614550`, `0x4928ba`, `0x49291b`, `0x49295e`
Confidence: verified
Proof: one
Test: none

`0x4928b2` returns without an active player. `0x4928ba` to `0x4928d5` take the previous pair, or the global when it is zero, and
skip the teardown when both are zero. `0x49291b` to `0x492938` store the new pair. `0x492963` to `0x49297c` index a byte table by
the type mask minus 9 and jump through a three-entry table to `0x492983` (unit), `0x4929f6` (game object) and `0x492a7d`
(corpse); the default returns at `0x492ab2`.

## Tooltips by object type

For a unit, the publisher fires the tooltip's `OnTooltipSetDefaultAnchor` script, builds the unit tooltip
(`0x529fe0`), signals `UPDATE_MOUSEOVER_UNIT` and plays a highlight sound by the unit's reaction toward the player:
`GAMEHIGHLIGHTHOSTILEUNIT` for 1 or less, `GAMEHIGHLIGHTNEUTRALUNIT` for 2 and 3, `GAMEHIGHLIGHTFRIENDLYUNIT` for 4
and more. A corpse fires the script, builds its tooltip (`0x52aef0`) and plays the neutral sound. A game object
whose floating-tooltip test holds (only the generic type, with its first data word nonzero) sets the tooltip's
owner itself, to `UIParent` with the cursor mode 6, before building; every other game object fires the script,
builds and plays the neutral sound. The owner is always set before the lines are added, because setting it clears the tooltip.

Addresses: `0x492983`, `0x4929f6`, `0x492a7d`, `0x529fe0`, `0x52aa20`, `0x52aef0`, `0x52ffe0`, `0x6061e0`, `0x702690`, `0xb4b44c`, `0xb4b3c4`
Confidence: verified
Proof: one
Test: none

The script slot is `[tooltip+0x444]`, fired through `0x702690` (`0x49299a`, `0x492a59`, `0x492a94`). The unit leg signals event 0x150 at
`0x4929ae`, reads the reaction twice (`0x4929bb`, `0x4929d8`) and plays the sound at `0x4929ca`, `0x4929e6` or `0x492aa8`. The game
object leg calls the object's virtual `+0x5c` (`0x4929fa`) and, when it answers true, `0x52ffe0(UIParent+0x24, 6, 0, 0)`
(`0x492a1e`) and the builder `0x52aa20` (`0x492a2d`). `0xb4b44c` is the frame found by the name `UIParent`
(`0x490065`) and `0xb4b3c4` the one named `GameTooltip` (`0x490091`).

## The world input hook

The frame manager's hook for button events (`+0x1128`, set to `0x492b50`) runs before a button reaches any frame,
and only when no frame holds the press on release. A right-button press (event field `+0x14` equal to 4) while any
payload is held or a gift wrap is pending clears the cursor with `ClearCursor(1, 1)` and consumes the event, so
the click does nothing else. In relative mouse mode (the event's mode word `+0x10` equal to 1) the hook sends the
press to the world frame's slot 0x68 and the release to its slot 0x6c directly, and consumes it.

Addresses: `0x492b50`, `0x495190`, `0x5edf10`, `0x766420`, `0x7662c0`
Confidence: verified
Proof: one
Test: none

`0x492b53` tests the press tag 0x400500c8 and `0x492b5c` the button; `0x492b62` to `0x492ba3` test the payload words in turn
(`ui/cursor#payload-kinds`); `0x492ba5` tests the wrap; `0x492bb5` clears. `0x492bc1` tests the mode word; `0x492bd9` calls slot 0x68 and
`0x492be5` slot 0x6c on the world frame `0xb4b2bc`. The release handler runs the hook only with no pressed frame
(`ui/mouse#mouse-release`).

## The tooltip stays put

Nothing in the GameTooltip's hide, owner or fade code writes the mouseover state, and nothing polls for a missing
plate. The hide `0x530a60` is `SetOwner(0, 0, 0, 0)` then the base hide. `0x52ffe0` clears the tooltip first
(`0x530050`), which zeroes the remembered unit and game object subjects and the line count and fires
`OnTooltipCleared`; it then stores the owner and the anchor mode, 7 (none) unless an owner is given. The fade
`0x530ae0` hides at once when the anchor mode is 6 (cursor) and otherwise arms a fade of 2.0 seconds. So a script
that hides the tooltip leaves the plate gone until the mouseover object changes or a reset of the cached pair
happens.

Addresses: `0x530a60`, `0x52ffe0`, `0x530050`, `0x530ae0`, `0x4919d0`, `0x529fe0`, `0x76ad50`
Confidence: verified
Proof: one
Test: none

`0x530a6b` calls `0x52ffe0` and `0x530a73` jumps to the base hide. `0x52ffe6` clears, `0x53000c` stores the owner at `+0x314`, `0x530012` mode 7
at `+0x318` and `0x530031` the given mode. Clear stores zero at `+0x368`, `+0x36c`, `+0x370`, `+0x374` (`0x5300ad` to `0x5300bf`)
and `+0x31c` (`0x530244`), then fires `+0x44c` (`0x530263`). `0x530ae3` tests mode 6; the fade timer is 2.0 (`0x808e7c`). `0x4919dc` to
`0x4919ec` compare the remembered subject at `+0x368`, `+0x36c` and `0x4919f2` calls `0x529fe0`: the refresh after a
unit's flags change rebuilds only for that subject, which a hide or owner call has just zeroed. The only code that writes
`0xb4e2c8` is `0x492890`; the other references are the crash dump and the `mouseover` unit token.

## Leaving the world frame

The world frame's leave runs the base leave, publishes a null mouseover when one is cached, zeroes the cache,
shows the base cursor and resets the reticle state. A nameplate that is hovered when mouselook begins publishes
the null mouseover from its own leave. So the cache is empty after a click on the world, and the next pick rebuilds
the tooltip.

Addresses: `0x483b10`, `0x7cb8b0`, `0x764ba0`
Confidence: verified
Proof: one
Test: none

`0x483b1d` calls the base leave, `0x483b46` the publish and `0x483b4b`, `0x483b51` clear the pair. The removal tail
`0x764ba0` calls the plate's leave with 1 only for the plate that was hovered (`ui/input#input-lists`).

## Clicks on the world

After the click-versus-drag gate accepts a release, `0x481f60(action)` builds a record for one of three legs by the
pick state `+0x350`, found on the press: an object (state 2, the GUID at `+0x358`), the ground (state 1, the
point at `+0x360`) or nothing (state 0, the press and release points at `+0x370` and `+0x37c`). Action 1 is the
left button and 4 the right. A charmed player takes the nothing leg with zeroed points. The press-time pick is
used, not the live mouseover.

Addresses: `0x481f60`, `0x481f00`, `0x492ce0`, `0x492c90`, `0x492d30`, `0x514ae0`
Confidence: verified
Proof: one
Test: none

`0x481f00` stores the pick state at the press (`0x481f4c`). `0x481f92` and `0x481f98` test the charm pair; `0x481fa4` to
`0x481fad` branch on the state to `0x48200b` (nothing), `0x481fdf` (ground) and `0x481fb8` (object). The object leg
reads only the three words of the record `0x481f60` built, none of the live mouseover globals.

## Payloads on the world

The object leg clears a preview payload (kinds 5, 7, 9) with `ClearCursor(1, 1)`, leaves any other payload
alone, then selects (action 1) or interacts (action 4) with the object. The ground leg clears any payload on
action 4 and only a preview payload on action 1. It then tries a ground-targeted cast (action 1) and otherwise runs
the click handler `0x5e0320`: for action 4 it walks the active mover to the point; for action 1 it asks to confirm
destroying an item payload that has a source container, leaving it held, and otherwise deselects the target.

Addresses: `0x492ce0`, `0x492c90`, `0x492580`, `0x5e0320`, `0x495300`, `0x6e60f0`, `0x6102b0`, `0x493540`
Confidence: verified
Proof: one
Test: none

`0x492ce0` tests the preview entry, `0x492cf8` the action, `0x492d05` calls select and `0x492d18` interact. `0x492c93` and
`0x492c99` are the ground leg's clears, `0x492cb0` the call of `0x492580`, which tries `0x6e60f0` for action 1
(`0x4925b0`) and `0x5e0320` otherwise. `0x5e0360` takes the walk for action 4; `0x5e0394` reads the item through `0x494c80`,
`0x5e03a9` calls `0x495300` and `0x5e03cf` deselects.

## The nothing leg

The nothing leg notes whether any payload is held. Action 4 clears the payload. Action 1 asks to confirm destroying
an item payload that has a source container, and otherwise clears the payload. The target is deselected only for the left button (action 1), when
no payload was held on entry and the CVar `deselectOnClick` (default 1) is nonzero. With no payload held, a click whose press and release points are not identical, by
the active mover being able to auto-interact (`objectlayer/unitmovement`), starts a walk along the heading for
the right button (`0x610100`) and clears nothing. The identity test compares the squared distance with the constant
at `0x7ffd74`, which is 0.0.

Addresses: `0x492d30`, `0x60f900`, `0x610100`, `0x7ffd74`, `0xc4da98`, `0xb4e2b8`
Confidence: verified
Proof: one
Test: none

`0x492d3f` notes the payload, `0x492ddf` and `0x492e04` clear, `0x492e31` asks to confirm and `0x492e5d` deselects, guarded by the
left-button test at `0x492e1b`, the note at `0x492e45` and the CVar record `0xb4e2b8` with its value at `+0x28` (`0x492e49` to `0x492e55`). `0x48fd00` to `0x48fd17` register `deselectOnClick` and store its record there. `0x492da2` is the compare and `0x492daa jnp` skips the walk for identical points; `0x492db5` calls `0x60f900` on
the mover, the player found for the GUID at `0xc4da98`, and `0x492dca` calls `0x610100` with the difference vector.

## Selecting by click

Selecting (`0x4925d0`, action 1) first offers a held payload to the clicked object, then makes it the target
(`0x493540`). For a player, while no spell awaits a target, an item payload with a source container or a money payload
is offered. A bag with items in it shows error 0x2b, and an empty bag equipped in a bag slot (0x13 to 0x16) error 0xe. A
dead active player shows 0x7e. With a trade open, the trade partner gets the payload in the trade window and anyone
else shows 0xc4; with none open the click asks to trade (opcode 0x116). For a unit that is the player's pet an item
payload feeds it (`ui/cursor#dropping-an-item-on-a-unit`). Any unit also plays its greeting
(`objectlayer/unitnames#greetings-and-unit-sounds`).

Addresses: `0x4925d0`, `0x4bfc50`, `0x5d3fb0`, `0x6ea1e0`, `0x60c270`, `0x493540`, `0x496720`, `0x5da2c0`, `0xb71728`
Confidence: verified
Proof: one
Test: none

`0x4925d0` requires an active player (`0x4925f4`) and the object (`0x492619`). `0x492635` tests the player type; `0x492668`
tests spell targeting; `0x4926f4` shows 0x2b, `0x49271f` shows 0xe, `0x492744` shows 0x7e and `0x492777` shows 0xc4. `0x49276b` calls
the trade window and `0x49278e` the trade request. `0x4927a6` to `0x4927f8` is the pet feed, `0x4927ff` the greeting and `0x49280c`
the select.

## Interacting by click

Interacting (`0x492820`, action 4) makes any object the target and calls the object's interact virtual with the
Shift state as its argument. `0x4949f0(action, guid)` runs select for 1 and interact for 4 when an active player
exists.

Addresses: `0x492820`, `0x4949f0`, `0x493540`, `0x41f8f0`
Confidence: verified
Proof: one
Test: none

`0x492870` selects and `0x492881` calls slot 0x60 of the object with the result of `0x41f8f0(0)`, the Shift state
(`core/input#key-codes`). `0x494a15` and `0x494a2c` test the action for 1 and 4.

## Clicking a nameplate

A press whose hovered frame is a nameplate goes to the plate alone and never starts mouselook, since the command
that enters mouselook runs only inside the world frame's own button handlers. The plate takes the press, becomes
the pressed frame, and the release reaches its click slot, which calls `0x4949f0` with the plate's unit and then
the button's own click. So a click on a plate selects or interacts with its unit exactly as a click on the model does.

Addresses: `0x7cb910`, `0x4949f0`, `0x779540`, `0x7792d0`
Confidence: inferred
Proof: none
Test: none

`0x7cb91c` and `0x7cb916` load the plate's GUID from `+0x4e8` and `+0x4ec`; `0x7cb92a` calls `0x4949f0` and `0x7cb936` `0x779540`.
That the world frame's handlers alone start mouselook was read in the caller census of the binding dispatch, not here.

## The right button

On a right-button world click, `0x492c20` first rotates a rotatable placement while a spell awaits a position, or
stops targeting. It clears the cursor when a gift wrap is pending, and returns a repair cursor to Point.

Addresses: `0x492c20`, `0x6e69d0`, `0x6e69e0`, `0x6e4900`, `0x5edf10`, `0x523d10`, `0x523d20`
Confidence: verified
Proof: one
Test: none

`0x492c20` tests the action word for 4 first. `0x492c26` to `0x492c3f` are the targeting arm, `0x492c44` the wrap test with the clear at
`0x492c54`, and `0x492c59` to `0x492c72` the repair reset. The targeting side is `spell/targeting#placement-model-and-rotation`.
