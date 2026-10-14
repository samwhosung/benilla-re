# ui: which frames take the mouse

The frame-side mouse rules that `ui/input` does not hold: which input kinds a frame starts with and how they
are switched, what a hidden frame keeps, which frame the pointer feeds, who gets a release and a wheel turn,
and what the enter and leave notifies do.

`ui/input` has the lists, the hover pick and the button handlers; `ui/frames` the frame record; `ui/framexml` the script
elements that enable input. The world side of the pointer (mouselook, the published mouseover, clicks on the
world) is `ui/mouseover`.

Roots: `0x76af00`, `0x76af50`, `0x777070`, `0x769820`, `0x7660d0`, `0x766420`, `0x7664f0`, `0x76c180`, `0x76b6a0`, `0x76b6f0`

## Input kinds at construction

A frame's input mask `+0xcc` has bit 0 for characters, 1 for keys, 2 for the mouse and 3 for the wheel. A plain
frame starts with 0. Button and check button start with the mouse bit (4), edit box with characters, keys
and the mouse (7), slider and color select with the mouse. Message frame, model, scroll frame, scrolling message
frame, simple HTML, status bar and movie frame start with 0, as does a tooltip. The world frame starts with keys,
mouse and wheel (0xe) and the minimap with the mouse (4).

Addresses: `0x769090`, `0x7786a0`, `0x779ced`, `0x779cf8`, `0x779d03`, `0x789467`, `0x78b2b5`, `0x481960`, `0x4edbc0`, `0x76af00`
Confidence: verified
Proof: one
Test: none

`0x7690d9` stores 0 in the frame constructor. The enables are `0x778771` (button, kind 2), `0x779ced`, `0x779cf8` and
`0x779d03` (edit box, kinds 0, 1 and 2), `0x789467` (slider), `0x78b2b5` (color select), `0x481b09`, `0x481b14` and `0x481b1f`
(world frame, kinds 1 to 3, `edi` being 1) and `0x4edc38` (minimap). Only these three instructions write `+0xcc`:
the constructor, the enable `0x76af38` and the disable `0x76af8e`.

## Switching input kinds

The attribute `enableMouse` enables the mouse when it parses true and does nothing when it is false;
`enableKeyboard` likewise enables keys and characters. Neither can disable. The Lua `EnableMouse(flag)` can:
it reads its flag with a default of true, so `EnableMouse()` enables and `EnableMouse(nil)` disables. The wheel and
keyboard calls work the same. A disable runs the removal tail of `ui/input`, an enable does not ask for a new hover
pick. `IsMouseEnabled()` reads bit 2 and nothing else and returns the number 1 or nil, never a boolean;
`IsMouseWheelEnabled()` reads bit 3.

Addresses: `0x769820`, `0x777070`, `0x777130`, `0x7772b0`, `0x76af00`, `0x76af50`, `0x6f1c10`
Confidence: verified
Proof: one
Test: none

`0x769ab1` skips the enable for a false attribute and no `0x76af50` call exists on that path. `0x7770f5` passes
default 1 to `0x6f1c10`; `0x77710d` enables with kind 2, `0x77711a` disables. `0x7771bb` and `0x77733b` shift the mask
by 2 and 3. There is no `enableMouseWheel` attribute.

## A hidden frame's bit

`0x76af00(kind, flag)` sets the bit even when the frame is not visible, and enters the frame in the kind's list
only when it is. A frame that becomes visible later enters every list its mask names, so the order of
`EnableMouse` and `Show` does not matter. A parent never gates its child by its own mouse bit: the bit does not
propagate, and a mouse-disabled parent leaves an enabled child hit. Only visibility gates, through the
child's own effective-visibility word `+0xd4`.

Addresses: `0x76af00`, `0x76ae10`, `0x764780`, `0x76afa0`
Confidence: verified
Proof: one
Test: none

`0x76af1b` tests `+0xd4` before `0x764aa0`; `0x76af38` ors the bit in either way. `0x76ae10` requires the frame
shown and its parent effectively visible before it sets `+0xd4` (`0x76ae7b`) and calls `0x764780`, which enters each enabled
kind through `0x76afa0`.

## What the pointer is tested against

Every frame is probed through its slot 0x3c, which is the drag-threshold check followed by the hit test
`0x76b020`; the world frame, minimap, edit box, slider, color select and nameplate override it and each calls
the base. The nameplate's refuses before testing while spell targeting wants a position. The hit test ignores
alpha, so a fully transparent frame with the mouse bit still takes the pointer. A click has no hit test of its own: the press
goes to the pressed frame, else the hovered one, so a frame the hover pick never chose cannot get `OnMouseDown`
or `OnMouseUp`.

Addresses: `0x7660d0`, `0x76b320`, `0x76b020`, `0x482aa0`, `0x4eb0c0`, `0x77a860`, `0x789ba0`, `0x78bd80`, `0x7cba30`
Confidence: verified
Proof: one
Test: none

`0x7660d0` calls slot 0x3c at `0x766207`. The base slot is `0x76b320`; the six overrides are the world frame's
`0x482aa0`, the minimap's `0x4eb0c0`, `0x77a860`, `0x789ba0`, `0x78bd80` and the nameplate's `0x7cba30`, each of which
calls `0x76b320`. `0x76b020` reads `+0x60` bit 0 and the rectangle at `+0xd8` to `+0xe4` only.

## Mouse release

The release goes to the pressed frame `+0x80` and to nothing else. It never uses the hover. The handler copies the
event, stamps the idle clock, and ends a title-region or modifier drag without any dispatch. Otherwise,
with no pressed frame and a hook (`+0x1128`), the hook may take it; with a pressed frame it runs that frame's button-up slot. The
pressed frame is cleared after the dispatch, and only when the event's held-buttons word `+0x18` is 0, so a release
with another button still down keeps it. The clear is a plain store, so a capture the script sets inside
`OnMouseUp` is lost. With nothing pressed the handler returns 1 and the event continues.

Addresses: `0x766420`, `0x766f70`, `0xcf0bc8`, `0x765640`, `0x764dc0`
Confidence: verified
Proof: one
Test: none

`0x76642b` snapshots `+0x80` into `ebx`. `0x766467` loads the drag mode and `0x766477` and `0x76647c` send modes 1 and 2 to
the cancel `0x765640`. `0x766485` runs the hook only if `+0x80` is null. `0x766498` is the only target test, `0x7664a4`
the slot 0x6c call and `0x7664bb` the blind clear after the test of `+0x18` at `0x7664a7`. Entering mouselook also
sends a release to a pressed frame, through `0x764dc0`.

## The wheel

A wheel turn goes to the first frame, from stratum 8 down, whose wheel mask bit is set, whose hit test holds at the
saved cursor point, and whose `OnMouseWheel` script exists and is shown. It needs no hover, no capture and no
mouse bit. The script gets the sign of the turn only, 1 for zero or more and -1 below zero, as `arg1`. A
wheel-enabled frame with no script, or hidden, does not take the turn and the walk goes on.

Addresses: `0x7664f0`, `0x76c180`, `0x7026f0`, `0x41ad80`
Confidence: verified
Proof: one
Test: none

`0x766524` to `0x76654b` convert the manager's saved cursor `+0x1118`, `+0x111c`, not the event's. `0x766586` calls
`0x76b020` directly, `0x766597` slot 0x70. `0x76c183` and `0x76c18d` test `+0xd4` and the script at `+0x160`;
`0x76c1a9` makes the sign and `0x76c1b8` fires the script with the format `%d`.

## Enter and leave

The base enter `0x76b6a0` shows the frame's highlight texture, sets a flag when the frame has none, and fires
the `OnEnter` script (`+0x140`). The base leave `0x76b6f0(arg)` hides the highlight, clears the flag, clears the
drag arm words `+0x100` and `+0x104` when `arg` is nonzero, and fires `OnLeave` (`+0x148`). The scripts get no
arguments. A hover pick runs the old frame's leave with 0 and then the new frame's enter; the removal tail runs
the leave with 1. The world frame's leave publishes a null mouseover, shows the base cursor and resets the
reticle state; a nameplate's enter and leave publish and clear its unit as the mouseover; the minimap's leave
hides the game tooltip first.

Addresses: `0x76b6a0`, `0x76b6f0`, `0x483b10`, `0x7cb850`, `0x7cb8b0`, `0x4eb880`, `0x492890`
Confidence: verified
Proof: one
Test: none

`0x76b6af` shows and `0x76b70c` hides the highlight through slot 0x88 and 0x84; `0x76b6c9` and `0x76b71c` set and clear flag
4 in `+0xf8`'s absence; `0x76b72c` clears the arm words; `0x76b6e5` and `0x76b751` fire the scripts. `0x483b1d` calls the
base leave first, `0x483b46` the publish, `0x483b58` the base cursor and `0x483b5d` stores 3 in `0x83dc2c`.
`0x7cb869` publishes the plate's GUID (`+0x4e8`) with a null previous and `0x7cb8bf` publishes null. `0x4eb88e`
hides the tooltip (`0xb4b3c4`).

## Hit rectangle insets

`SetHitRectInsets(left, right, top, bottom)` needs four numbers (usage error otherwise) and `GetHitRectInsets()`
returns the four in that order. The insets are stored in screen units converted from interface units, and take
effect when the frame's rectangle is next resolved: positive values shrink the hit area, negative values grow
it beyond the frame.

Addresses: `0x76b1b0`, `0x76b580`, `0x775bf0`, `0x775d50`, `0x41ae40`, `0x41ae60`
Confidence: inferred
Proof: none
Test: none

`0x76b1b0` stores left `+0xec`, right `+0xf4`, top `+0xf0`, bottom `+0xe8`; `0x76b580` adds each inset times the effective scale
(`ui/input#hit-test`). `0x775bf0` converts with `0x41ae40` and `0x775d50` with `0x41ae60`; the argument checks and the
order were not read here beyond those calls.
