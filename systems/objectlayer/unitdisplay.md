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
Test: `objectlayer::base_render_alpha_difftest`, `objectlayer::diff_transform_scale_copy`

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

## Creatures that animate out of sight

A unit the scene elects for pass 2 (out of the frustum) is neither drawn nor animated, except a
creature whose record flags `+0x14` have 0x20: `0x607da0` links its model into the animation worklist
anyway, so it plays its clips and fires their events and sounds.

Addresses: `0x607da0`, `0x607e1e`, `0x607e22`, `0x607e26`, `0x607e2f`, `0x607e43`, `0x623b70`, `0x710b90`, `0x683dd0`
Confidence: verified
Proof: one
Test: none

`0x607e1e` compares the pass argument with 2 and `0x607e26` calls `0x623b70`, which reads the creature
record `+0xb30` and tests `+0x14` for 0x20. The model is the mount at `+0xdc` when there is one, else the body
at `+0xd8` (`0x607e2f`, `0x607e39`), and `0x607e43` calls `0x710b90` with 1. The election is in
`terrain/scene#object-election`; the sound flag of `0x623b70` is in `sounds`.

## The shipped display tables

`CreatureDisplayInfo.dbc` has 12 fields in 48 bytes and `CreatureDisplayInfoExtra.dbc` 19 in 76. The
extra row holds the race, sex, skin, face, hair style, hair colour and facial hair (fields 1 to 7), ten
item display ids (8 to 17) and the baked texture name (18). Of the 10534 display rows, 6984 name an extra
row; 6939 extra rows exist and 6931 carry a baked name. Of the 6984, 147 also have a texture name in
fields 6 to 8, which `ApplyCreatureDisplay` reads and the character component does not.

Addresses: `0x60afb0`, `0x5fb200`, `0x60c690`, `0x60c6a0`, `0x60c6c0`, `0x60c6e0`, `0x4797b0`
Confidence: verified
Proof: one
Test: none

The counts are read from the shipped files. A sample of 40 baked names each opened as a `BLP2` file
under `Textures\BakedNpcTextures\`. `0x5fb200` reads the extra row at `+0x4` to `+0x1c` and loops over
`+0x20` to `+0x44`; `0x60c6e0` reads `+0x48`. Neither `0x60afb0` nor `0x5fb200` reads display fields
6 to 8. The unit hands its display row (`+0xb34`) and model row (`+0xb3c`) to `0x4797b0` (`0x5fb986`).

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

## Player appearance bytes

A player's character component takes its look from three fields: race and gender from
`UNIT_FIELD_BYTES_0`, skin, face, hair style and hair color from `PLAYER_BYTES` (field 193), and
facial hair from `PLAYER_BYTES_2` (field 194).

Addresses: `0x5fb200`, `0x5fb4a2`, `0x5fb4ac`, `0x5fb4b6`, `0x5fb4bc`, `0x5fb4c6`, `0x5fb4d0`, `0x5fb4da`, `0x5fb4e4`, `0x476b90`
Confidence: verified
Proof: one
Test: none

`0x5fb200` reads bytes 0 and 2 of the unit descriptor at `+0x78` and `+0x7a` (`0x5fb4a2`,
`0x5fb4ac`) as race and gender. The player block pointer `+0xe68` (`0x5fb4b6`), the descriptor from
offset 0x2f0, gives bytes 0 to 3 at `+0x14` to `+0x17` as skin (`0x5fb4bc`), face (`0x5fb4c6`), hair
style (`0x5fb4d0`) and hair color (`0x5fb4da`), and byte 0 at `+0x18` as facial hair (`0x5fb4e4`). The
record is handed to the component by `0x476b90`.

## Model data flags

The flags word `+0x4` of a unit's `CreatureModelData` row is tested at four places, none of
which decides whether the model is drawn.

Addresses: `0x5fc0dd`, `0x5fc0e1`, `0x600003`, `0x600007`, `0x5fb3d3`, `0x5fb3d7`, `0x60adc7`, `0x60add2`, `0x60b198`, `0x60b1a2`
Confidence: verified
Proof: one
Test: none

Bit 0x1 skips the footstep ground effect (`0x5fc0dd`, `0x5fc0e1`), bit 0x2 the breath puff of the `$BTH`
event (`0x600003`, `0x600007`), bit 0x4 allows the character bake (`0x5fb3d3`, `0x5fb3d7`) and bit 0x8
sets bit 0x20000 of `+0xd58`, clearing it otherwise (`0x60adc7`, `0x60b198`). All four go through the row
cached at `+0xb3c`.

## A model with nothing to draw

A unit whose model has no geometry is not hidden by any flag: it submits no draw commands because the
scene's batch loop runs once per texture unit of the selected model view. `InvisibleStalker`, the model
of display rows 11686 and 13069, has 0 vertices, 0 textures and four views with no submeshes and no
texture units. No `UNIT_FIELD_FLAGS` bit and no `CreatureModelData` flag reaches the draw decision, and
the display alpha only hides a unit at 0.

Addresses: `0x707680`, `0x707a2b`, `0x707a33`, `0x707a72`, `0x707a7f`, `0x70808d`, `0x466a20`, `0x613e10`, `0x6044b0`, `0x5ff580`, `0x60d2d0`
Confidence: verified
Proof: one
Test: none

`0x707a72` loads the view's texture-unit count `+0x20` and `0x707a7f jbe 0x70808d` leaves the loop for a count of 0.
The create dispatcher `0x466a20` runs `0x613e10` for units, players, game objects, dynamic objects and corpses
alike, on the one world scene. The change handler of `UNIT_FIELD_FLAGS`, `0x6044b0`, reaches `0x5ff580`, which
only attaches and detaches equipment models. The shipped file has 430 `CreatureModelData` rows and
`InvisibleStalker.m2` (row 1731, flags 3, collision box 0.7778 by 2.438) has a zero render box and a collision box of
that size. Rows 11686 and 13069 have scale 1.0 and 0.01 and alpha 255.

## The display alpha

A unit's alpha comes from its display row. The rebuild of a display ends by fading the unit over
1000 ms to the row's alpha byte over 255 (slot 27, 1.0 without a row), times the alpha of the
first effect on the list `+0xb50` when there is one. The same fade restores the alpha when such an
effect ends. A player is a unit, so a player in a creature display takes the row's alpha.

Addresses: `0x60abe0`, `0x60ad9f`, `0x60d180`, `0x60d2d0`, `0x614f80`, `0x60d219`, `0x5ff2b2`, `0x60d9de`
Confidence: verified
Proof: one
Test: none

`0x60ad9f` is the call to `RestoreAlpha` (`0x60d180`) in the rebuild; `0x60d180` calls slot 27
(`0x60d2d0`: `fild [row+0x14]`, `fmul` 1/255), multiplies by `[+0xb50]+0x78` when the list is not
empty and calls `FadeTo(alpha, 0x3e8)`. The other callers are `0x5ff2b2`, `0x60d219` and `0x60d9de`
(`auras`). Slot 27 is the same function in the unit and player tables; no code tests the object
type.

## Mounts

The `UNIT_FIELD_MOUNTDISPLAYID` watcher passes the old id to `0x5ffa50`, which dismounts from the
old mount and mounts the new one with effect kind 6. `0x607a00` loads the mount model, seats the
body at the mount's attachment 0 playing animation 91, scales the rider by its scale over the
mount's, and takes the mount's footprints and sound data. `0x607ce0` reverses it.

Addresses: `0x5ffa50`, `0x607a00`, `0x607ce0`, `0x607c60`, `0x60c480`, `0x6079b0`
Confidence: verified
Proof: one
Test: `objectlayer::unit_mount_scale_difftest`

`0x607a00` needs both the display row (0xc0de90) and its model data (0xc0de68), stores the
display's `+0x10` scale at `+0x9c`, hands the model to `0x613d80`, copies the model data's
footprint into `+0xcc0` to `+0xcc8` (sizes over 36) and `0x607c60`'s sound data into `+0xb44`.
The rider's transform scale is `+0x98` over `+0x9c`, with alpha 1 and a white tint. Already
mounted (`+0xdc` set), it only re-picks the animation and resets the camera. `0x607c60` takes the
display's `+0x8` sound row, else the model data's `+0x34`; `0x60c480` returns `+0xb44` before
`+0xb40`. `0x607ce0` detaches, clears the mount model, plays animation 0 on the body, sizes the
selection from the body, clears `+0xb44` and resets the camera. `0x5ffa50` also refreshes the unit
sounds, the quest marker and the name. `0x6079b0`, which nothing reaches, returns the mount's model
path and reads through a null row when the display or model is missing. Slices: unit_mount_scale_difftest `0x607ae0`-`0x607b8c`.

## The mount poof

A unit that gets a nonzero mount display plays the hard-coded effect "Mount Poof" (kind 6) once, from
any distance, after its mount model is built. It hangs on the rider's body model at attachment 0x13 and
ends with its model's clip. A dismount to 0 plays none. A running poof is replaced by the next one on
the unit and destroyed by a model rebuild, which builds the mount again without a poof.

Addresses: `0x5ffa50`, `0x5ffa6c`, `0x5ffa7f`, `0x5ffa87`, `0x5ffa8b`, `0x5ffa9c`, `0x5ffacb`, `0x5ffad2`, `0x5ffad9`, `0x5fbf50`, `0x6203e0`, `0x6203f1`, `0x620580`, `0x62058f`, `0x620be0`, `0x620c78`, `0x620e16`, `0x620e2d`, `0x61fae0`, `0x6208e0`, `0x60abe0`, `0x60ac10`, `0x60adfa`, `0x60ae06`, `0x607a00`, `0x608046`
Confidence: verified
Proof: one
Test: none

`0x5ffa50` runs `0x607ce0` when the old display is nonzero (`0x5ffa6c`), then skips the build and the
effect when the new one is 0 (`0x5ffa87`); it tests no unit identity and no distance. The effect is made
with callback `0x5fbf50` (`0x5ffac2`), which destroys the effect (`0x6203e0`) when its second argument is
0, the clip-finished notice, and ignores the replaced notice, 1. `0x6203e0` sets flags 0x900 on the
effect and the per-tick `0x620580` skips an effect with 0x900 (`0x62058f`). With flags 0 the tick
takes the attachment leg: `0x620c78` loads the body model and `0x620e2d` finds attachment 0x13 on it
every tick. `0x60abe0` destroys every effect on the unit (`0x60ac10` loop) and, when the mount display is
still above 0, ends in `0x607a00` (`0x60ae06`). The effect model `DruidMorph_Impact_Base` has one clip
of 100 to 2900 ms, so the poof lasts 2.8 s. `0x607a00` and `0x607ce0` do not touch the unit's effects.

## Selection circle and colour

`0x608e00` (slot 12) draws the selection circle at the unit's feet: radius `+0xcf0` times the
object scale, the colour from slot 11, half alpha. `0x60aee0` sets the radius from the model's
bounds. `0x605960` (slot 11) picks the colour: the flashing target colour; for player-controlled
units by attack, PvP and party; for others grey when dead, else by reaction toward the player.

Addresses: `0x608e00`, `0x60aee0`, `0x605960`, `0x607f60`, `0x60b1d0`
Confidence: verified
Proof: one
Test: `objectlayer::shadow_quad_difftest`, `objectlayer::camera_model_scale_difftest`

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
1.2 when flat. `0x60b1d0(old)` resizes the mover (0x60b270); a 0 result sends opcode 0x29b, no body. Slices: shadow_quad_difftest `0x608e14`-`0x608e6a`; camera_model_scale_difftest `0x60af4f`-`0x60afac`.

## Hand items and weapons

A creature's hands come from `UNIT_VIRTUAL_ITEM_SLOT_DISPLAY` (fields 37 to 39) and
`UNIT_VIRTUAL_ITEM_INFO` (40 to 45). `0x605da0` hangs a slot's item model or clears it.
`0x60b770` places a hand's item in the hand or its sheath by the sheathe state; `0x60b590` moves
it between them; `0x611e10` hangs the ranged weapon and a bow's string. A disarmed unit hides its
main-hand weapon. Each hand keeps a weapon trail.

Addresses: `0x605da0`, `0x605ddb`, `0x605e0f`, `0x605e30`, `0x605ee0`, `0x605f10`, `0x60b770`, `0x60b590`, `0x611e10`, `0x611ff0`, `0x608d60`, `0x60ba00`, `0x60d5d0`, `0x60e550`, `0x608dd0`
Confidence: verified
Proof: one
Test: `objectlayer::u5_trail_color_byte_difftest`, `objectlayer::u5_trail_midpoint_difftest`

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
`WTOBJECT`); `0x60ba00` and `0x60d5d0` release them, `0x60e550` passes them arguments. `0x608dd0`, which nothing reaches, rebuilds both. Slices run `0x612044`-`0x612116` and `0x612221`-`0x612269`.

## The ammo model

A shooter holds one ammo model, built from an item display id. Two callers set it: a spell start that
carries an ammo id, for any caster, and a change of the active player's own ammo field. An id of 0
removes it. Firing does not remove it.

Addresses: `0x60ba30`, `0x60ba5f`, `0x60ba7e`, `0x60bab3`, `0x60bacb`, `0x60baf6`, `0x60bb19`, `0x60bb24`, `0x6e78b6`, `0x4c70b0`, `0x4c70fc`, `0x479f40`, `0x838d70`, `0x838d50`, `0x60f530`, `0x624b70`, `0x60f322`, `0x60f533`, `0x600177`
Confidence: verified
Proof: one
Test: none

`0x60ba30(id)` does nothing when `+0xd2c` already holds the id and a model hangs at `+0xd28`;
else it detaches and closes the old model (`0x60ba5f` to `0x60ba7e`), stores the id, and builds the new
one with `0x479f40(body, record, selector)`. The selector is 0x18, or 0x19 for a ranged item of inventory
type 0x19 (`0x60bab3`), and picks the `Item\ObjectComponents\Ammo\` or `Weapon\` directory
(`0x838d70`, `0x838d50`); the record is the item display row for the id. The model attaches to the body at
35 only for selector 0x18, a ranged item that is not a gun or crossbow (class 2, subclass 3 or 18,
`0x60bacb`), and bit 0x4000 of `+0xd58` (`0x60bb19`). The spell start calls it at `0x6e78b6` with the packet's id; the field watcher below calls it too.
`0x60f530` detaches the model without clearing `+0xd28` or `+0xd2c`; its six callers are the cancel, the
cast-result and movement paths. Bit 0x4000 is set at `0x624b70` and cleared at `0x60f322`, `0x60f533` and `0x600177`.

## The ammo field watcher

The active player's ammo field sets the ammo model before any shot.

Addresses: `0x4c70b0`, `0x4c6fea`, `0x4c70b1`, `0x4c70c7`, `0x4c70dc`, `0x4c70e7`, `0x4c70e9`, `0x4c70fc`, `0x4c7110`, `0x4c711a`, `0x60ba30`
Confidence: verified
Proof: one
Test: none

`0x4c70b0` is registered at `0x4c6fea` for byte offset 0x102c of the player block and calls `0x60ba30`
at `0x4c70fc` with that field of the active player. It ignores its arguments: it resolves the active player's own GUID (`0x4c70b1` to `0x4c70c7`, mask 0x10) and compares it with itself
(`0x4c70dc` to `0x4c70e7`), so the zero branch (`0x4c70e9`) is never taken. It then signals event 0xba (`0x4c7110`) and returns 1 (`0x4c711a`).

## The quiver

A bow shows its quiver on the back while the bow is drawn.

Addresses: `0x611e10`, `0x611e31`, `0x611f08`, `0x611f17`, `0x6117bd`, `0x611813`, `0x611f2d`, `0x611f33`, `0x611f4c`, `0x6222e0`, `0x5da0b0`, `0x478aa0`, `0x478960`, `0x479c50`, `0x479cfe`, `0x4798c0`, `0x838d10`
Confidence: verified
Proof: one
Test: none

`0x611e10(leaving)` hangs the ranged weapon. For a player with a character component
(`+0xd30`), a nonzero argument, passed when the ranged state ends (`0x6117bd`), clears slot 11
(`0x611f17`), and so does the change out of state 2 to state 0 (`0x611813`). With the argument 0 and a weapon whose inventory type
is neither 0x1a nor 0x19 (`0x611e31`), so a bow, it finds an item of class 11 among the unit's items at
`+0x1d38` (`0x6222e0`), takes its display id (`0x5da0b0`) and sets slot 11 (`0x478aa0`). The component
builds `Item\ObjectComponents\Quiver\` plus the record's model name (`0x479c50`) and attaches the model at
attachment 26 (`0x479cfe`).

## The bowstring

A bow's string is drawn by a callback, not by the model.

Addresses: `0x611ff0`, `0x611edf`, `0x611eee`, `0x710db0`, `0x7131b0`, `0x6121b8`, `0x6121c9`
Confidence: verified
Proof: one
Test: none

`0x611e10` registers `0x611ff0` with `SetDrawCallback` (`0x611eee`) only when the ranged weapon is
class 2, subclass 2 (`0x611edf`). The callback reads the bow's event positions `$WTT` and `$WTB`
through `0x7131b0` and joins them to a middle point by two lines: the unit's attachment 35 while
bit 0x4000 of `+0xd58` is set (`0x6121b8`), else the midpoint of the two ends.

## The shield and unarmed animations

A shield is not held while the unit plays an unarmed ready or attack animation.

Addresses: `0x60b770`, `0x60b8c9`, `0x60b8d9`, `0x60b6e0`, `0x60b590`, `0x60b632`, `0x5fdb50`
Confidence: verified
Proof: one
Test: none

`0x60b770` calls `0x60b6e0` with the current animation (`0x5fdb50`) at `0x60b8c9`; for the ids 16, 20,
25, 117 and 118 it clears the in-hand flag of a shield (inventory type 14) at `0x60b8d9`. `0x60b590`
asks the same question at `0x60b632`.

## Weapon trails

A visual kit's character procedure of type 8 arms the swing trails of the unit's weapons. It
stores the colour, parameter 0 with parameter 3 in the top byte, at `+0xd1c` and the duration,
parameter 2, at `+0xd20`. The next `PlayAnimation` passes both to each of the two trail objects at
`+0xd34` and clears `+0xd20`, so one animation fires one trail. The trails draw their own geometry.

Addresses: `0x60d80a`, `0x60d828`, `0x60d835`, `0x5fe48e`, `0x5fe4a3`, `0x5fe4ac`, `0x5fe4b1`, `0x60e550`, `0x6c69c0`
Confidence: verified
Proof: one
Test: none

`0x60d80a` truncates the three parameters with `_ftol`; `0x60d828` and `0x60d835` store them. The
read in `0x5fe48e` skips the pass when the duration is 0; `0x5fe4ac` calls `0x60e550(&colour,
duration)`, which calls `0x6c69c0` on each trail object that exists.

## Disarm and the hands

Hands attach through `0x60b770(hand)`, which asks for the hand's weapon without `raw` and returns,
attaching nothing, when the weapon is hidden by disarm. The flags watcher redoes the hands when the
disarm bit 0x200000 changes. A unit that holds a class 2 item in the main hand, is now disarmed and
has a weapon drawn (`+0xd40` not 0) loses the main-hand attachment (`0x47a310`, equipment slot
0xf); every other change re-hangs the hand through `0x60b770(0)`. The ranged weapon is never hidden.

Addresses: `0x5ff580`, `0x5ff614`, `0x5ff623`, `0x5ff63e`, `0x5ff65d`, `0x5ff676`, `0x5ff67d`, `0x5ff690`, `0x5ff695`, `0x5ff699`, `0x5ff6a5`, `0x60b770`, `0x60b791`, `0x60b79e`, `0x47a310`
Confidence: verified
Proof: one
Test: none

`0x5ff614` and `0x5ff619` run the block only when 0x200000 is in the changed bits. `0x5ff629` calls
slot 38 (`GetWeapon(0, 1)`, the true main hand); `0x5ff631` and `0x5ff636` pass on to the off hand
unless it is a class 2 item; `0x5ff63e` tests the flag, `0x5ff65d` the drawn state, and `0x5ff676`
detaches. `0x5ff695` repeats the block for the off hand but pushes slot 0x10 at `0x5ff699`, an
equipment index that slot 38 does not know: a player returns none, so `0x5ff6a5` leaves the block
dead, and a creature reads past its virtual item block. `0x60b791` calls slot 38 and `0x60b79e`
returns on none.

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
