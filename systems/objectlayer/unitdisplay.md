# objectlayer: unit display

How a unit looks: the display records behind `UNIT_FIELD_DISPLAYID` (field 131), the body model
and its character component, the mount behind `UNIT_FIELD_MOUNTDISPLAYID` (field 133), the
selection circle and its colour, the items in a creature's hands and their trails, and the
shapeshift test. The records are `CreatureDisplayInfo`, `CreatureDisplayInfoExtra`,
`CreatureModelData`, `CreatureSoundData`, `UnitBloodLevels` and `UnitBlood` rows, kept as
pointers on the unit from `+0xb34` to `+0xb48`.

Roots: `0x60abe0`, `0x5ffa50`, `0x605da0`, `0x608e00`, `0x605960`

## Display records

`0x60afb0` resolves the display: the `CreatureDisplayInfo` row at `+0xb34`, its extra row at
`+0xb38`, its model data at `+0xb3c`, the sound data at `+0xb40` (the display's own, else the
model's) and the blood levels at `+0xb48`. An unknown display id prints `NOUNITDISPLAYID` and
falls back to the table's first row. The model path (vtable slot 10) falls back to
`Spells\ErrorCube.mdx` instead.

Addresses: `0x60afb0`, `0x60b131`, `0x60ae10`, `0x600320`, `0x60d2d0`, `0x5fc310`, `0x60c690`, `0x60c6a0`, `0x60c6c0`, `0x60c6e0`, `0x60a390`
Confidence: verified
Proof: one
Test: none

The rows come from the index tables 0xc0de90 (display), 0xc0dea4 (extra, by display `+0xc`),
0xc0de68 (model, `+0x4`), 0xc0de54 (sound, display `+0x8` else model `+0x34`) and 0xc0d634
(blood levels, display `+0x28` else model `+0x14` else the first row). At `0x60b131` a creature
(type word 9) asks the creature cache 0xc0e354 for `+0xb30`. Bit 0x100 of `+0xc58` marks the native display (field 131 equals field 132); model
flag 0x8 sets bit 0x20000 of `+0xd58`. `0x60ae10` returns 1 when the row, the extra id, the model
row or the sound row differs from those held. `0x600320` prints `NOCREATUREDISPLAYIDFOUND` or
`INVALIDDISPLAYMODELRECORD` before returning 0x860c9c. `0x60d2d0` (slot 27) is the display's
`+0x14` over 255. `0x60c6a0` and
`0x60c6c0` give race and gender from the extra row, else `UNIT_FIELD_BYTES_0` bytes 0 and 2;
`0x60c6e0` gives its baked texture name `+0x48`. `0x60a390` picks the
`UnitBlood` row (0xc0d620) at `+0x4 + 4·level` of the blood levels row, by the violence level.

## Changing the display

`0x60abe0` rebuilds the body when `0x60ae10` sees a new record set. It dismounts, stops the unit's
sounds and effects, resolves the records, loads the slot 10 model and hands it to the object,
reapplies textures, name and quest marker, replays the auras, the channel and the current
animation, plays the transform spell's impact kit once, and remounts. The character component
waits until the model has loaded.

Addresses: `0x60abe0`, `0x60ad36`, `0x607da0`, `0x60ce70`, `0x5fb200`
Confidence: verified
Proof: one
Test: none

`0x60abe0` sets key bone `+0xd5c` to 6, clears bit 0x80 of `+0xd58`, sets the model's callbacks
`0x6078d0` and `0x607840`, calls `0x613cf0` and `ApplyCreatureDisplay`, resets a camera that follows the unit, deletes the component `+0xd30` and
marks `+0xccc` and `+0xcd0` pending. At `0x60ad36` the transform spell `+0xd54` plays its
`SpellVisual` `+0xc` kit as kind 1 and is cleared. The native display of a live unit with a stand
state sends it again (`0x60be30`); model flag 0x8 sets bit 0x20000 of `+0xd58`. `0x607da0` (slot
16) builds the component once the model is loaded and refreshes the portrait (0x491a50) for
`+0xcd0`. `0x60ce70` (slot 13) sets a rider's transport frame; on the body, bones 4 and 6 set bits 0x80
and 0x100 and pick the key bone 4, 6 or −1; a mount
model without attachment 0 prints `MOUNTDISPLAYIDNOMOUNTATTACHMENT`. `0x5fb200` builds the
component from the extra row (race, gender, skin, face, hair, facial hair; texture
`Textures\BakedNpcTextures\%s` into a 260-byte buffer, bounded) or, for a player whose model data
has flag 0x4, from its bytes. Equipment: 19 slots for a native player display, else the extra's
ten item displays `+0x20` to `+0x44`, then both hands (`0x60b770`).

## Mounts

The `UNIT_FIELD_MOUNTDISPLAYID` watcher passes the old id to `0x5ffa50`, which dismounts from the
old mount and mounts the new one with effect kind 6. `0x607a00` loads the mount model, seats the
body at the mount's attachment 0 playing animation 91, scales the rider by its scale over the
mount's, and takes the mount's footprints and sound data. `0x607ce0` reverses it.

Addresses: `0x5ffa50`, `0x607a00`, `0x607ce0`, `0x607c60`, `0x60c480`, `0x6079b0`
Confidence: verified
Proof: one
Test: none

`0x607a00` needs both the display row (0xc0de90) and its model data (0xc0de68), stores the
display's `+0x10` scale at `+0x9c`, hands the model to `0x613d80`, copies the model data's
footprint into `+0xcc0` to `+0xcc8` (sizes over 36) and `0x607c60`'s sound data into `+0xb44`.
The rider's transform scale is `+0x98` over `+0x9c`, with alpha 1 and a white tint. Already
mounted (`+0xdc` set), it only re-picks the animation and resets the camera. `0x607c60` takes the
display's `+0x8` sound row, else the model data's `+0x34`; `0x60c480` returns `+0xb44` before
`+0xb40`. `0x607ce0` detaches, clears the mount model, plays animation 0 on the body, sizes the
selection from the body, clears `+0xb44` and resets the camera. `0x5ffa50` also refreshes the unit
sounds, the quest marker and the name. `0x6079b0`, which nothing reaches, returns the mount's model
path and reads through a null row when the display or model is missing.

## Selection circle and colour

`0x608e00` (slot 12) draws the selection circle at the unit's feet: radius `+0xcf0` times the
object scale, the colour from slot 11, half alpha. `0x60aee0` sets the radius from the model's
bounds. `0x605960` (slot 11) picks the colour: the flashing target colour; for player-controlled
units by attack, PvP and party; for others grey when dead, else by reaction toward the player.

Addresses: `0x608e00`, `0x60aee0`, `0x605960`, `0x607f60`, `0x60b1d0`
Confidence: verified
Proof: one
Test: none

Colours are built once (guard bits in `0xc4d9b8`) as ARGB: reactions 0 to 7 at `0xc4d8a4` red,
red, orange 0xffff8000, yellow, then green; `0xc4da4c` blue (0x60, 0x60, 0xff), `0xc4d990` light
blue, `0xc4d934` light green, `0xc4d978` light red, which nothing reads. A unit with bit 3 of
`UNIT_FIELD_FLAGS` takes red when it and the player can attack each other, blue when only it can,
yellow when only the player can; else, flagged for PvP (0x605ff0), light green in the party or
green, unflagged light blue in the party or blue. Others take `0xc4da80` (0xff7f7f7f) at no
health, else the reaction (0x6061e0) colour. At `0x607f60` slot 14 sets bit 0x10 of `+0xc58` on
the player's target while the player attacks something and may attack it; `0x605960` then
returns `0xc4d8c8`, red whose green byte slot 14 sweeps 0 to 128 and back every 500 ms
(`0xc4daa0`, `0xc4daa4`). The radius is √(½·√(dx² + dy²)) of animation 0's bounds or the model's,
1.2 when flat. `0x60b1d0(old)` resizes the mover (0x60b270); a 0 result sends opcode 0x29b, no body.

## Hand items and weapons

A creature's hands come from `UNIT_VIRTUAL_ITEM_SLOT_DISPLAY` (fields 37 to 39) and
`UNIT_VIRTUAL_ITEM_INFO` (40 to 45). `0x605da0` hangs a slot's item model or clears it.
`0x60b770` places a hand's item in the hand or its sheath by the sheathe state; `0x60b590` moves
it between them; `0x611e10` hangs the ranged weapon and a bow's string. A disarmed unit hides its
main-hand weapon. Each hand keeps a weapon trail.

Addresses: `0x605da0`, `0x605ddb`, `0x605e0f`, `0x605e30`, `0x605ee0`, `0x605f10`, `0x60b770`, `0x60b590`, `0x611e10`, `0x611ff0`, `0x608d60`, `0x60ba00`, `0x60d5d0`, `0x60e550`, `0x608dd0`
Confidence: verified
Proof: one
Test: none

Info bytes: 0 class, 1 subclass, 3 inventory type, 4 sheath type. `0x605e30(slot, raw)` is null
without a display (`0x605f10`); unless raw, with `UNIT_FIELD_FLAGS` bit 21, slot 0 is null when it holds a weapon, and slot 1 when it holds one and slot 0 does not.
`0x605da0` uses model option
0x860a50[slot] with `AttachItemModel` (`0x605ddb`, `0x605e0f`); the table holds two dwords, so
slot 2 reads past it and takes 0 from the next table, 0x860a58. `0x60b770(hand)`
uses options 15 and 16 and sends hand 2 to `0x611e10`; a non-player's off hand stays empty behind
a two-handed main hand (subclass 1, 5, 6, 8, 10, 12, 17 or 20). `0x60b590(hand, sheathed)` moves the
model between the hand attachment (1; 2, or 0 for a shield; 1 or 2 for ranged) and the sheath
one (0x47a070), opening or closing the hand.
`0x611e10` uses option 17, keeps the model at `+0xd24`, gives a bow `0x611ff0` (a line from `$WTT`
to `$WTB`) and lets the active player face its target. `0x608d60` rebuilds a hand's trail at `+0xd34` (0x6c6920, a
`WTOBJECT`); `0x60ba00` and `0x60d5d0` release them, `0x60e550` passes them arguments. `0x608dd0`, which nothing reaches, rebuilds both.

## Shapeshift forms

`0x60e9f0` returns whether a unit is shapeshifted: `UNIT_FIELD_BYTES_1` byte 2 names a
`SpellShapeshiftForm` row whose flags lack bit 0, which the 1.12 table sets on the warrior
stances, Shadowform, Stealth and Moonkin Form. A form id with no row crashes the client.

Addresses: `0x60e9f0`, `0x60ea19`, `0x60ea24`
Confidence: verified
Proof: one
Test: none

The byte indexes 0xc0d774 up to 0xc0d778. With a row, `0x60ea19` reads `+0x2c`; past the bound,
`0x60ea24` reads `[0 + 0x2c]` with no row, and an in-bound empty slot reads the same null. The byte
is the server's, and the 1.12 table holds ids 1 to 32, so a form from 33 to 255 faults rather
than returning 0.
