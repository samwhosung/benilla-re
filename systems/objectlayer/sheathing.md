# objectlayer: sheathing

How a unit's weapons go between hand and sheath: the sheathe state, the one function that
changes it, everything that calls it, the manual toggle, the rule that follows an animation's
weapon flags, and where a ranged weapon is held. The state is byte 0 of `UNIT_FIELD_BYTES_2`
(field 164), cached at `+0xd40` with the previous value at `+0xd3c`: 0 away, 1 melee, 2 ranged.
The sheathe animations and their pick are in `unitanim`.

Roots: `0x611cf0`, `0x5eb480`, `0x5fdf80`, `0x604c70`, `0x6127b0`

## The sheathe setter

`0x611cf0(state, instant, send)` refuses a change when the state is 2 and the unit is a player whose
class is out of range, has no class row or has a relic slot, when the state is the current one, and when the unit has no body model.
Otherwise it stores the state and the old one, sends `CMSG_SETSHEATHED` (0x1e0) with the new state for the
active player when `send` is set, and applies it: at once when `instant` is nonzero, else with
the sheathe animations.

Addresses: `0x611cf0`, `0x611cff`, `0x611d04`, `0x611d1e`, `0x611d2a`, `0x611d3b`, `0x611d41`, `0x611d4a`, `0x611d58`, `0x611d64`, `0x611d6c`, `0x611d72`, `0x611d7b`, `0x611de3`, `0x611770`, `0x611b60`, `0xc0def4`, `0xc0def8`, `0x539701`, `0x5fa6d0`
Confidence: verified
Proof: one
Test: none

`0x611cff` runs the class test for state 2 only, on a unit whose type has the player bit
(`0x611d04`): the class byte `+0x79` of the descriptor indexes the `ChrClasses` table at `0xc0def4`
(last valid index `0xc0def8`, set to −1 at `0x539701`); a class below 0 or above that index (`0x611d1e`, `0x611d2a`), a null row (`0x611d3b`) and a
nonzero `+0x40` of its row, the relic-slot column (Paladin, Shaman, Druid), each end the call at `0x611dfc` (`0x611d41`). The class test comes before the test of the
current state (`0x611d4a`) and the body model test (`0x611d58`). The stores are `0x611d64` and
`0x611d6c`; the send needs `IsActivePlayerUnit` (`0x611d72`) and the argument (`0x611d7b`). The
function writes neither the attack target nor any bit of `+0xd58`.

## Changes made by play

The sheathe state changes when the player starts a melee attack, toggles the weapons, casts a
ranged spell, fishes, emotes or sits, and when another unit attacks or casts. Only the manual toggle plays the
sheathe animations; every other change is at once. Nothing sheathes when combat ends, and no
vendor or gossip action changes it.

Addresses: `0x5ecd80`, `0x62583a`, `0x6e5930`, `0x6e78f3`, `0x60f34c`, `0x60d367`, `0x5ef630`, `0x6127fb`, `0x5df0fa`, `0x5eb65b`, `0x5eb673`, `0x5eb681`, `0x5eb6a8`, `0x5ecb70`, `0x6255b0`, `0x6e54f0`, `0x6e7700`, `0x60edf0`, `0x60d2f0`, `0x5ef560`, `0x6127b0`, `0x5dee30`
Confidence: verified
Proof: one
Test: none

Each site calls `0x611cf0` with state, instant, send. A melee attack start (`0x5ecd80`) passes (1, 1, 1)
and so does the attacker-state message (`0x62583a`) when the attacker is not already in melee
state. A ranged spell passes (2, 1, 1): the local send (`0x6e5930`) for a spell with Attributes bit 1 or
AttributesEx2 bit 5, the spell start of another caster (`0x6e78f3`), and the cast visual kit
(`0x60f34c`) for a ranged kit at stage 0 or 4. Channelling at a game object of type 17 (`0x60d367`) passes (1, 1, 1) when the unit is
not in melee state. A text emote (`0x5ef630`) and a stand state other than 0 and 2 (`0x6127fb`)
pass (0, 1, 1). A change of the ranged item while ranged state holds (`0x5df0fa`) passes (2, 0, 1)
and is refused as the current state. The four sites in the toggle (`0x5eb65b`, `0x5eb673`,
`0x5eb681`, `0x5eb6a8`) pass instant 0.

## The manual toggle

The toggle `0x5eb480` cycles the weapons and silently ignores the request while any of these
holds: the unit is not the active player or not client-controlled, mounted, stunned, dead, in
combat with an attack target, channelling, holding bit 0x400 of `+0xd58`, casting a pending
spell, in an animation that blocks it, not standing, or still playing a sheathe animation.

Addresses: `0x5eb480`, `0x5eb48e`, `0x5eb49f`, `0x5eb4d1`, `0x5eb4e4`, `0x5eb4f2`, `0x5eb502`, `0x5eb513`, `0x5eb520`, `0x5eb52b`, `0x5eb542`, `0x5eb553`, `0x5eb56f`, `0x5eb580`, `0x5eb58b`, `0x5eb5e8`, `0x5eb642`, `0x5eb68d`, `0x5eb6d0`, `0x5fa5f0`
Confidence: verified
Proof: one
Test: none

The channel field `+0x228` (`0x5eb48e`) and bit 0x400 (`0x5eb49f`) set a flag that `0x5eb520`
tests. The tests then run in order: the active player, `IsClientControlled` (`0x5fa5f0`, at
`0x5eb4d1`), a mount display `+0x1fc` above 0 (`0x5eb4e4`), `UNIT_FIELD_FLAGS` bit 0x40000
(`0x5eb4f2`), health at or below 0 (`0x5eb502`), a nonzero attack target `+0xc48` (`0x5eb513`),
the flag, a nonzero `+0xc8c` (`0x5eb52b`), `0x5eb6d0` on the current animation (`0x5eb542`), and
a stand state above 0 (`0x5eb553`). A body model must be loaded (`0x5eb56f`) with animations 90 and
89 (`0x5eb580`, `0x5eb58b`), and key bones 3 and 2 must play neither. From state 2 the toggle
goes to 0; from 1 to 2 when a ranged weapon exists in a class without a relic slot
(`0x5eb5e8`), else to 0; from 0 to 1 when either hand holds a weapon, else to 2 when ranged is
available (`0x5eb68d`).

## The animation reconcile

Whenever an animation is requested, `0x5fdf80` forces the sheathe state from the weapon-flag word
of the requested animation's `AnimationData` row. Flag 0x4 stows, flag 0x10 stows except in
the ranged state for nine fire and load animations, and flag 0x20 draws melee. A mounted unit is
stowed, and a unit with an attack target is drawn. The thrown wind-up, animation 108, is stowed
too; the weapon stays in hand because the local cast plays its kit (`0x6e5898`) before it draws (`0x6e5930`).

Addresses: `0x5fdf80`, `0x5fdf89`, `0x5fdfab`, `0x5fdfb9`, `0x5fdfc7`, `0x5fdfd9`, `0x5fdfe5`, `0x5fe005`, `0x5fe02d`, `0x5fe04c`, `0x5fe061`, `0x5fe073`, `0x5fe07f`, `0x5fe0d7`, `0x5fe0f9`, `0x5fe10c`, `0x5fe118`, `0x5fe124`, `0x5fe13b`, `0x5fe147`, `0x5fe16e`, `0x5fe180`, `0x6e5898`, `0x6e5930`, `0x5fdb50`, `0xc0e070`, `0xc0e074`
Confidence: verified
Proof: one
Test: none

`0x5fdf80` indexes `AnimationData` (`0xc0e070`, largest id `0xc0e074`) with the id armed on the key
bone, not the one played (`0x5fdb50`); column 2 at `+0x8` holds the flags. In order: flag 0x4
forces (0, 1, 1) (`0x5fdfb9`); a mount model `+0xdc` forces (0, 1, 1) (`0x5fdfd9`), every time.
In ranged state, a unit that is not the active player asks for state 2 again, which the setter
refuses (`0x5fe005`); animations 105 and 106 (`0x69`, `0x6a`) and the others of the nine of
`0x5fe180` leave the state alone; 112 hangs a free ammo model at attachment 1 unless the weapon is a
gun or crossbow (`0x5fe07f`); any other animation with flag 0x10 stows (`0x5fe073`). In the other states, an attack target with a swing animation
(`0x5fe0f9`), flag 0x20 or an attack target (`0x5fe13b`) draw melee, flag 0x10 stows (`0x5fe118`),
and a unit that is not the active player takes the server byte `+0x278` (`0x5fe16e`). The
local player is never reconciled to the server byte.

## Stand state and the server byte

A unit with weapons drawn is stowed at once when its stand state becomes anything but 0 and 2.
The server sets the state through the `UNIT_FIELD_BYTES_2` field for any unit, the active
player included, and no `SMSG` for it exists.

Addresses: `0x6127b0`, `0x6127cc`, `0x6127fb`, `0x604c70`, `0x604ca7`, `0x5fc920`, `0x5fca2a`, `0x5fcb14`, `0x5fcaff`
Confidence: verified
Proof: one
Test: none

`0x6127cc` tests that the sheathe state is nonzero, calls the stand state (slot `+0xa4`) twice
for 0 and 2, and stows through `0x611cf0(0, 1, 1)`. `0x604c70` is the watcher of byte `+0x278`:
it resolves the GUID with mask 8 and calls `0x611cf0(byte, 1, 0)`, so it applies to any unit
and never sends. The animation-done handler `0x5fc920` sets the current state again at `0x5fca2a`
for the sheathe animations, and returns to the previous state at `0x5fcb14` when bit 0x8000 of
`+0xd58` is set (`0x5fcaff`).

## The ranged weapon's hands

A drawn ranged weapon is held in the right hand when it is a gun, crossbow, wand or thrown weapon
and in the left hand when it is a bow. A ranged weapon is never attached to a sheath point: out
of state 2 it is not drawn.

Addresses: `0x611770`, `0x6117da`, `0x6117ee`, `0x611842`, `0x61184a`, `0x611865`, `0x61186b`, `0x4796b0`, `0x611e10`, `0x611e31`, `0x7130a0`, `0x47a0c0`, `0x47a070`
Confidence: verified
Proof: one
Test: none

`0x611e10` passes the weapon's inventory type to `0x47a0c0` as a flag that is set for 0x1a and
0x19 (`0x611e31`): set picks attachment 1, clear picks 2. `0x611770` leaving state 2 for 0 detaches
attachment 0x23 (`0x6117ee`) and the weapon from attachment 1 for those types (`0x61184a`) or from 2 for
others (`0x611865`), then disarms animation slots 8 to 12 or 13 to 17 (`0x4796b0`) and attaches nothing. Leaving 2
for 1 calls `0x611e10(1)`, whose sheath point comes from `0x47a070` with the weapon's sheathe type
0, which gives no attachment.
