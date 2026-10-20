# ui: the pointer picture

The picture the mouse pointer shows: the 41 modes and their images, the shown and base mode, the Lua functions
that set them, the named custom cursor, and the classifier that picks a mode for the object under the pointer.

This is `Cursor.cpp` and the cursor half of `WorldFrame.cpp`. The payload picture that replaces it while
dragging is `ui/cursor`; the mouseover publish and the click legs are `ui/mouse`. The targeting cursor is
`spell/targeting#the-targeting-cursor`; a game object's mode comes from `objectlayer/gameobjects#cursor`.

Roots: `0x5236c0` (load images), `0x523c20` (set mode), `0x523c40` (custom), `0x489490` (SetCursor),
`0x481790` (world update), `0x4828d0` (classify), `0x482200` (unit), `0x482740` (corpse), `0x4826e0` (game object)

## Cursor modes

A mode is an index into the cursor name table `s_cursorNames` (`0x853b88`): 0 is blank, 1 Point, 2 Cast, 3 Buy,
4 Attack, 5 Interact, 6 Speak, 7 Inspect, 8 Pickup, 9 Taxi, 10 Trainer, 11 Mine, 12 Skin, 13 GatherHerbs, 14
PickLock, 15 Mail, 16 LootAll, 17 Repair, 18 Item, 19 SkinHorde, 20 SkinAlliance. Modes 21 to 40 are the same
names with `Unable` in front, so the unable form of a mode is the mode plus 20. Mode 41 is the named custom
cursor. The image of mode n is `Interface\Cursor\<name>.blp`, a 32 by 32 texture.

Addresses: `0x853b88`, `0x5236c0`, `0xbe2d80`, `0x449610`, `0x44ae00`, `0x58b5c0`
Confidence: verified
Proof: one
Test: none

`0x5236c0` runs once at start. For each of the 42 slots it reads the name at `0x853b88 + 4n`, formats the
path with `Interface\Cursor\%s.blp` (`0x523705`), loads it, and copies 0x400 dwords into `0xbe2d80 + 0x1000n`; a null
name gives a zeroed image. It sets the shown mode `0xbe2c2c` and the base mode `0xbe2c4c` to 1 and the
payload picture state to 0, and sends the image (`0x523790`) with the hotspot at 0, 0.

## Setting the mode

`0x523c20(mode)` stores the shown mode in `s_cursorMode` and sends its image, but does nothing when the mode is
unchanged. The base mode `s_cursorBaseMode` is the resting picture: `0x523d10` reads it, `0x523d20` writes it,
and `0x523d30` shows it, which is what `ResetCursor()` does. The base mode starts at 1; entering a spell target
sets it to 2, and the repair cursor sets it to 17. No Lua function reads the shown mode: the getter `0x523c10` has
no caller.

Addresses: `0x523c20`, `0x523d10`, `0x523d20`, `0x523d30`, `0x523c10`, `0x48ac70`, `0x48ac60`, `0xbe2c2c`, `0xbe2c4c`
Confidence: verified
Proof: one
Test: none

`0x523c20` compares `ecx` with `[0xbe2c2c]` at `0x523c20` and tail-jumps to the send at `0x523c2e`. `0x523d30` loads
`[0xbe2c4c]` and tail-jumps to `0x523c20`. `ResetCursor` (`0x48ac70`) calls `0x523d30`. `ShowInspectCursor`
(`0x48ac60`) calls `0x523c20` with 7. Neither function has a return value.

## Who sets the mode

`0x523c20` has 43 call sites and no indirect reference. 26 are in the world classifier (`0x481863` to `0x4829c8`),
three are Lua functions or the right-click leg (`0x489593` SetCursor, `0x48ac65` ShowInspectCursor, `0x492c72`), eight
belong to the sell and repair functions of `ui/merchantcursor`, three set or reset the base mode (`0x523d36`
and the gift wrap's `0x5edec9` and `0x5edefd`), and three are spell targeting starting and stopping (`0x6e49ff`,
`0x6e50ba`, `0x6e5556`). No code outside scripts computes a mode for a hovered item or a hovered interface
frame.

Addresses: `0x523c20`, `0x492c20`, `0x5edea0`, `0x5eded0`, `0x6e4940`, `0x6e4b60`, `0x6e54f0`
Confidence: verified
Proof: one
Test: none

The census is every `call` or `jmp` to `0x523c20` in the listing; no data word holds its address. `0x492c59` to `0x492c72` is
the right-click leg returning a repair base cursor to Point (`ui/mouse`).

## SetCursor

`SetCursor(name)` takes a string, a number or nil to reset. A number converts to its text (`0x6f3690` through
`0x6f7c80`) and is then a name like any other, so `SetCursor(5)` loads a custom cursor named `5`. Called with no argument,
or with a value of another type, it raises `Usage: SetCursor("cursor" or nil)`. Eight names, compared without case, give a mode:
`POINT_CURSOR` 1, `CAST_CURSOR` 2, `BUY_CURSOR` 3, `ATTACK_CURSOR` 4, and the same names with `_ERROR_CURSOR`
for 21, 22, 23 and 24. Any other name is a custom cursor. A mode, or a custom cursor that loads, returns the
number 1; a custom cursor that fails to load returns nil. Nil sets the base mode with `0x523d30` and returns
nothing.

Addresses: `0x489490`, `0x6f3400`, `0x6f3690`, `0x523c40`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

`0x4894a6` builds the eight-pair table on the stack. `0x48950f` takes the type of argument 1 and sends a type
tag of 0 to the reset at `0x489518`; an absent argument has type -1, so it reaches `0x48952c`, which takes the string (`0x6f3690` converts a number at `0x6f36b4`),
finds none and raises the usage error (`0x489537`). `0x489550` walks the pairs, `0x489593` sets the mode, `0x48956c` calls the
custom loader and `0x48959c` pushes nil on failure.

## Custom cursors

`0x523c40(name)` loads the texture at the path `name`, as given, as the mode 41 cursor: `SetCursor` passes the
script's string, and a game object its formatted `Interface\Cursor\<name>.blp`, or `Interface\Cursor\Unable<name>.blp`
when unusable. It returns 1 at once when the shown
mode is 41 and the name equals the last one, without case. Otherwise it loads the texture, which must
be 32 by 32, copies it to image slot 41 (`0xc0bd80`), stores mode 41 and sends it, remembers the name in `0xbe2c70`,
and returns 1. A texture that cannot be loaded or has another size returns 0 and changes nothing. `0x523d40(name)` maps a name back to a mode: the first mode
from 1 up whose table name matches without case, or 0.

Addresses: `0x523c40`, `0x523c54`, `0x523c86`, `0x449610`, `0x523d40`, `0x5f8790`, `0x5f87dd`, `0x8609a4`, `0xbe2c70`, `0xc0bd80`
Confidence: verified
Proof: one
Test: none

`0x523c54` keeps the name in `ebx`, and `0x523c86` passes it unchanged as the path of `0x449610` (format 2, loose 1; `gx/textures#texture-file-creation`); no path is formatted here. A game object's path is formatted by `0x5f8790` (`objectlayer/gameobjects#cursor`), which pushes the format `Interface\Cursor\Unable%s.blp` (`0x8609a4`) at `0x5f87dd` when `0x5f8810` answers 0, else `Interface\Cursor\%s.blp`. `0x523c52` compares the mode with 0x29 and `0x523c66` the name. `0x523cb9` and `0x523cbd` test both sizes against 0x20.
`0x523cc9` stores the mode and `0x523cdb` the image slot. The only caller of `0x523d40` is the game object
cursor-name path (`objectlayer/gameobjects#cursor`); `0x523d68` ends the scan below 0x2a.

## The world update

The world frame's update `0x481790` picks a cursor mode only while the frame under the mouse is the world frame
(the frame manager's mouse focus equals the world frame). Over any other frame it writes no mode, and the frame's
own script decides the picture. For the world it picks (`0x481190`) with the pointer position: a terrain hit goes to the
reticle (`0x4820f0`), an object to the classifier `0x4828d0`, and nothing, while a spell awaits a target, shows
UnableCast (22) and clears the mouseover; with no spell it shows the base mode and clears the mouseover.

Addresses: `0x481790`, `0x481190`, `0x4820f0`, `0x4828d0`, `0x6e48a0`, `0x482090`
Confidence: verified
Proof: one
Test: none

`0x4817ae cmp [eax+0x7c],esi` and `0x4817b1 jne` are the focus test. `0x48184a` calls the pick; `0x48184f` and `0x481852`
branch on its result 1 and 2; `0x481855` tests spell targeting; `0x48185e` shows 0x16 and `0x481875` the base.
A pick result other than 1 and 2 reaches the same two branches.

## Classifying a hovered object

`0x4828d0` runs for a picked object. While mouselook is on (bit 1 of the frame's word at `+0x38c`) it clears the
mouseover and shows nothing. While a spell awaits a target it asks `0x6e6460` and shows Cast (2) or UnableCast
(22). Otherwise it needs an active player and an object whose mouseover virtual (`+0x54`) answers nonzero, else
it clears the mouseover and shows the base mode. It then picks by the object's type mask: exactly an item (3)
shows Pickup (8); a unit (9) or player (0x19) goes to the unit classifier; a game object (0x21) and a corpse (0x81)
to theirs; any other type, containers included, shows the base mode. It ends by publishing the mouseover.

Addresses: `0x4828d0`, `0x6e6460`, `0x482090`
Confidence: verified
Proof: one
Test: none

`0x4828da test [edi+0x38c],2` is the mouselook test. `0x482982 call [eax+0x54]` is the mouseover virtual. `0x482992`
bounds the type mask minus 3 by 0x7e, `0x482997` maps it through a byte table and `0x48299e` jumps through a
four-entry table: the targets are `0x4829c3` (item), `0x4829a5` (unit), `0x4829af` (game object) and `0x4829b9`
(corpse); the default is `0x4829cf`. The publish `0x482090` stores the pair at
`+0x340`, `+0x344` and calls `0x492e70` only when it changes.

## Units: charm and interaction

With the player's charm latch (bit 0 of `+0x1c70`) set, a hovered unit that is alive gets Attack (4) when the
possessed unit (`objectlayer/interaction#far-sight`) exists, is neither dead nor mounted, and may attack it;
anything else gets the base mode. There is no range test. Otherwise the unit's squared centre distance to the
player is compared with 30.864 (5.5556 squared; equal counts as in range). If the unit can be interacted with
(`objectlayer/reaction#interacting-now`), its NPC flags pick the mode, lowest bit first, and the unable form
(+20) is used beyond that distance.

Addresses: `0x482200`, `0x5ee5a0`, `0x606980`, `0x606880`, `0x5df490`, `0x605f30`, `0xb4b32c`, `0x482231`, `0x4822c8`
Confidence: verified
Proof: one
Test: `ui::textglue_482_normalize3`, `ui::textglue_482_sq_len3_482200`, `ui::textglue_482_square_cmp`

`0x482231` tests the latch. `0x482253` refuses a unit with health 0 or less, `0x48225d` a missing possessed unit, `0x48226a`
a dead one and `0x482280` a mounted one (its field at `+0x1fc` above 0); `0x4822a8` is the attack test and `0x4822b5` the
mode. `0x4822e7` forms the difference of the two positions and `0x482307` stores the sum of squares. `0x482320`
compares it with `0xb4b32c` (the square of 5.5556, set at `0x48439c`), and `0x482334` makes the unable flag
`ebx`. Each mode is `ebx` turned into 0 or 20 (`neg`, `sbb`, `and 0x14`) plus the base.

Bit-exact coverage: `ui::textglue_482_normalize3` covers `0x4822f7` to `0x482310` of `0x482200`; `ui::textglue_482_sq_len3_482200` covers `0x4822f7` to `0x482310` of `0x482200`; `ui::textglue_482_square_cmp` covers `0x4825d5` to `0x4825e6` of `0x482200`.

## NPC flag cursors

The first set bit picks: gossip (0x1) Speak (6); quest giver (0x2, by `0x5df490`) Speak; vendor (0x4) Pickup (8);
flight master (0x8) Taxi (9); trainer (0x10) Trainer (10); spirit healer (0x20) and spirit guide (0x40) Speak;
innkeeper (0x80) Interact (5); banker (0x100) Buy (3); petitioner (0x200), tabard designer (0x400) and
battlemaster (0x800) Speak; auctioneer (0x1000) Buy; stable master (0x2000) Speak. The repair flag (0x4000) is
never tested, so a unit with no other flag gets the base mode. In range the mode is the one listed; beyond 5.5556 it
is that mode plus 20.

Addresses: `0x482336`, `0x482385`, `0x4823b5`, `0x4823d9`, `0x4823fd`, `0x482419`, `0x48243d`, `0x482483`, `0x4824a7`, `0x4824cc`, `0x4826cb`
Confidence: verified
Proof: one
Test: none

The flag word is `[unit+0x110]+0x234`. The tests and the modes they add to are: bit 0 `0x48233c` (6), the quest giver
call `0x482362` (6), bit 2 `0x482396` (8), bit 3 `0x4823ba` (9), bit 4 `0x4823de` (10), bits 5 and 6 `0x482405` and
`0x482410` to `0x4824cc` (6), bit 7 `0x48241e` (5), bit 8 `0x482442` to `0x4824a7` (3), bits 9, 10 and 11
`0x48244c`, `0x482456` and `0x48247e` to `0x482483` (6), bit 12 `0x4824a2` to `0x4824a7` (3), bit 13 `0x4824c4` to `0x4824cc`
(6). A unit whose bit 13 is clear jumps to `0x4826cb`, the base mode. The re-test of bits 9 and 10 at `0x48245b` cannot
add a result.

## Units: loot, skin and attack

A unit that cannot be interacted with is tested in order. If it is lootable (`objectlayer/units#right-click-and-loot`),
it shows Pickup (8) when `CanLootNow` allows or a loot window already holds it, else UnablePickup (28); both become
LootAll (16) and UnableLootAll (36) while Shift is held. Else, if it is skinnable and the player knows the skinning
latch for its kind, it shows Skin (12) within the interact reach squared or UnableSkin (32) beyond it. A player target
the player cannot assist shows the faction skin cursor instead, SkinAlliance (20) or SkinHorde (19) in reach and
(40, 39) beyond it, by the player's side.

Addresses: `0x4824e6`, `0x6003a0`, `0x5ec110`, `0x41f8f0`, `0x6e3480`, `0x6066f0`, `0x5efe00`, `0xb700e4`
Confidence: verified
Proof: one
Test: none

`0x4824f2` is the lootable test, `0x4824fe` the `CanLootNow` call and `0x48250a` to `0x48251d` the already-looting test of the
player's `+0x1d28`. `0x482521` reads Shift (key code 0, `core/input#key-codes`). `0x48256d` tests the skinnable flag
(bit 26 of unit flags), `0x482589` the latch `0xb700e4` indexed by whether the target is a player,
`0x48259f` the assist test and `0x4825ca` the reach (`0x6e3480`). `0x4825db` compares the reach squared with the
distance squared; `0x4825ef` and `0x48260b` index the tables at 0x8043a4 and 0x80439c by `0x5efe00`.

## Units: attack

The last test is the attack cursor. It shows Attack (4) within 10.45 yards squared (109.2025, equal counts) and
UnableAttack (24) beyond, when the player is alive, not mounted and has control, may attack the unit, and has no
unit flag 0x20000, and the unit is alive or is dead with its dynamic flag bit 5. Otherwise the base mode.

Addresses: `0x482648`, `0x605f30`, `0x606980`, `0x80447c`, `0xb4b3e4`
Confidence: verified
Proof: one
Test: none

`0x48264a` tests the player dead, `0x482653` reads the player's mount field, `0x48266c` the unit's health and `0x482676` its
dynamic flags. `0x482680` is the attack test, `0x48268f` the flag, `0x48269b` the control word and `0x4826a7` the 109.2025
compare; `0x4826b8` is UnableAttack and `0x4826cb` the base mode.

## Corpses and game objects

A corpse that `0x5d6e20` says the player may loot shows Pickup or LootAll (and the unable forms) by the same loot
rule as a unit, from the corpse's distance. Otherwise a corpse with flag bit 5 set, the player-corpse skin latch
`0xb700e8` set and a corpse the player cannot assist gets the faction skin cursor by reach, as a player
target does. Anything else shows the base mode. A game object shows the mode `0x5f8760` returns, the base mode
for 0, and a custom cursor for 41 (`objectlayer/gameobjects#cursor`).

Addresses: `0x482740`, `0x5d6e20`, `0x6067d0`, `0x4826e0`, `0x5f8760`, `0x5f8790`, `0xb700e8`
Confidence: verified
Proof: one
Test: `ui::textglue_482_sq_len3_482740`, `ui::textglue_482_square_cmp`, `ui::textglue_482_sub3`

`0x4827ca` calls the lootable test, `0x4827d7` `CanLootNow`, `0x482838` the skin branch with the corpse flag test (`0x482844`,
bit 5 of `[corpse+0x110]+0x74`) and `0x482855` the assist test. `0x4826f0` calls `0x5f8760`; `0x482702` tests for 0x29
and `0x482715` builds the name with `0x5f8790` before `0x482720` loads it.

Bit-exact coverage: `ui::textglue_482_sq_len3_482740` covers `0x4827af` to `0x4827ca` of `0x482740`; `ui::textglue_482_square_cmp` covers `0x482878` to `0x482885` of `0x482740`; `ui::textglue_482_sub3` covers `0x482789` to `0x4827aa` of `0x482740`.

## Mounted and lootable

The unit and corpse classifiers read a mount field twice: the player's in the attack test and the possessed unit's in
the charm leg. A mounted player still sees the loot and skin cursors over a lootable unit or corpse. A game
object's usable test refuses a mounted player (`objectlayer/gameobjects#usable`), which makes its cursor the unable form.

Addresses: `0x482659`, `0x482278`
Confidence: verified
Proof: one
Test: none

`0x482659` is the player's mount field, tested in the attack branch; `0x482278` is the possessed unit's in the charm branch.
`0x482740` reads neither, nor does the loot branch of `0x482200`.
