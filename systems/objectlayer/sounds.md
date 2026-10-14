# objectlayer: unit sounds

How a unit makes its creature sounds: the vocal kinds of its `CreatureSoundData` row and the chance
and rate limits on them, footsteps, foley, emote voices, the death thud, the attack-reaction
sounds and the loop a creature hums while alive; and the player's own versions, which replace three
of the unit's virtual slots. The sound system, which plays, places and stops the sounds, is `sound`;
the greetings and the sound messages that call into this file are `unitnames`.

UnitSound_C.cpp runs from `0x623020` to `0x623dd0` and PlayerSound_C.cpp from `0x62f880` to
`0x62fb70`, initializers excepted (`initializers`). The unit's vtable slots 34 to 36 are
`0x623490`, `0x623610` and `0x6238f0`; the player's are `0x62f880`, `0x62fa30` and `0x62fb70`. Vocals,
foley, footsteps and the loop use the mount's sound data while the unit rides, through
`CGUnit_C::GetSoundData`; the reactions and the armour kind read the unit's own `+0xb40`.

Roots: `0x623110`, `0x6230c0`, `0x623490`, `0x623610`, `0x6238f0`, `0x6232c0`, `0x623390`, `0x623640`, `0x6236e0`, `0x6237c0`, `0x6237e0`, `0x623800`, `0x623870`, `0x623910`, `0x6239c0`, `0x6239f0`, `0x623a40`, `0x623b70`, `0x623c10`, `0x623c80`, `0x623d10`, `0x62f880`, `0x62fa30`, `0x62faa0`, `0x62fb70`

## Start-up and shutdown

`0x623110` builds the death thud tables, registers the `FootstepSounds` CVar (default 1), starts the
alert clock and finds the `SoundEntries` row named `SpiritWolf (DONOTRENAME)`. `0x6230c0` frees the
tables at shutdown.

Addresses: `0x623110`, `0x6231a0`, `0x623dd0`, `0x623d90`, `0x6230c0`, `0x6230d0`, `0xc4e0c8`, `0xc4e0e4`, `0xc4e0f4`, `0xc4e140`
Confidence: verified
Proof: one
Test: none

`0x6231a0` sizes five arrays of 8-byte pairs at `0xc4e0f4` (12 bytes each: count, capacity, data)
to the `TerrainTypeSounds` maximum id plus one through `0x623dd0`, zeroing new pairs, then walks the
`DeathThudLookups` rows from the last: a size class below 5 and a terrain id within the maximum store
the sound and the water sound at `[size][terrain]`. `0x623110` stores the CVar at `0xc4e140`, the
time at `0xc4e0e4` and the row id at `0xc4e0c8`; `0x6230d0` frees each array (`0x623d90` frees
one), and `0x6230c0` tail-jumps to 0x458c50, which zeroes 0xcf55fc.

## Vocal sounds

Slot 34, `0x623490(kind, force)`, plays a vocal of the unit's sound data at its position. Unless
forced a kind passes a chance: 71 in 101 for kind 0, 61 for 2, 41 for 5, always otherwise; kind 5
plays at most once every 10 seconds, forced or not. Kinds 0 to 3 and 8 stay silent while 0x4591f0
holds. The player's slot 34 uses 36, 31 and 41 in 101 and raises the sound two yards.

| kind | row | caller |
|---|---|---|
| 0, 1 | `+0x04`, `+0x08` | a swing's exertion, critical |
| 2, 3, 9 | `+0x0c`, `+0x10`, `+0x14` | a hit's pain, critical, crushing |
| 4, 5, 7, 8 | `+0x1c`, `+0x20`, `+0x2c`, `+0x34` | `$FDX`, 5; an alert reaction, 8 |
| 10, 11, 12 | `+0x30`, `+0x64`, `+0x68` | |

Addresses: `0x623490`, `0x623520`, `0x8626d4`, `0x623290`, `0x623020`, `0x623064`, `0x623b10`, `0x623b58`, `0x623b64`, `0x623b70`, `0x62f880`, `0x62f940`, `0x86424c`
Confidence: verified
Proof: one
Test: none

`0x623520` advances the client random state in line, the steps of `CRandom::uint32` (`random`),
then takes `MulHigh32(r, 101)` and passes when it is at most `0x8626d4[kind]` (70, 100, 60, 100,
100, 40, then 100s). `0x623290` passes kind 5 when 10,000 ms have passed since `0xc4e0e4`, and
restarts it. `0x623020` reads the row by the 13 cases of `0x623064`; kind 6 and kinds above 12
give 0. The class passed to 0x458890 is `0x623b10(kind)`: kinds 0 and 1 give 5, or 11 when the
creature record's flags `+0x14` have 0x20 (`0x623b70`); 2, 3 and 9 give 7, or 12; the rest 0
(bytes `0x623b64` into `0x623b58`). The player's `0x62f940` uses `0x86424c` (35, 100, 30, …) and
gives the active player class 8 for kinds 2, 3 and 9.

## Foley and the alert

The model events `$FD1` to `$FD9` play the unit's foley: `$FD1` to `$FD4` play the four dwords
from the sound data's `+0x38`, two yards above the unit; `$FD5` to `$FD9` find nothing. `$FDX`
plays kind 5 on any unit but the active player.

Addresses: `0x6232c0`, `0x623440`, `0x6230a0`, `0x6236b0`
Confidence: verified
Proof: one
Test: none

`0x6232c0` compares the tag as a dword in a search tree: `$FD1` to `$FD9` pass 0 to 8 to
`0x623440`, whose `0x6230a0(row, i)` returns `+0x38 + 4i` for `i` below 4 and 0 otherwise; the
sound plays through `PlaySoundAtArgs` at the position plus 2.0 in z. `$FDX` calls `0x6236b0`, which
calls slot 34 with kind 5, not forced, unless the unit is the active player.

## Footsteps

`$FSD` plays a footstep: the sound data's `+0x24` at the event point, with the unit's terrain, a
flag for a point at a liquid surface and one for a size class of 3 or more. A unit with byte 3 flag
0x2 of `UNIT_FIELD_BYTES_1`, a mover with flag 0x40000000, a player ghost or `FootstepSounds` off
makes none.

Addresses: `0x623390`, `0x62339c`, `0x6233aa`, `0x6233b6`, `0x6233f3`, `0x623406`
Confidence: verified
Proof: one
Test: none

`0x62339c` tests `[+0x110]+0x213` bit 0x2; `0x6233aa` the mover flags `+0x9e8`; `0x6233b6` a player's
`PLAYER_FLAGS` bit 0x10. Vtable `+0x8c` runs first. `0x6233f3` reads the CVar's integer `+0x28`.
`0x623406`: `0x625500 ≥ 3` and `CGUnit_C::IsPointAtLiquidSurface(point, …)` go with `+0xc60`, the
ground's terrain type, to 0x458380 (sound, point).

## Creature reactions

`0x623a40(kind)` plays a creature's reaction at its mouth, attachment 17: 0 its aggro `+0x28`, 1 `+0x70`, 2 `+0x6c`,
4 its death `+0x18`; 3 plays nothing. A reaction still playing is replaced only by a higher kind.
`0x6239c0` stops the playing reaction when 0x458fb0 allows.

Addresses: `0x623a40`, `0x623a59`, `0x623a66`, `0x623afc`, `0x623b90`, `0x6239c0`, `0x862708`
Confidence: verified
Proof: one
Test: none

`0x623a40` needs its own sound data `+0xb40` and 0x4591f0 false (`0x623a59`). `0x623a66`: with `+0xb20` playing
(0x7a5810) and the kind not above `+0xb24` it returns; else it stores the kind, stops `+0xb20`, and
plays through 0x459230 into `+0xb20`, by the five cases of `0x623afc`. `0x623b90(out, attachment)`
returns the attachment's position, or for a model without it the unit's position raised by
`0x862708[attachment]` for ids 0 to 36.

## The loop sound

`0x623800` keeps a creature's loop sound, the sound data's `+0x5c`, on it while it is alive and its
dynamic flags lack 0x20, and stops it otherwise; death stops it through `0x6237e0`.

Addresses: `0x623800`, `0x6237e0`
Confidence: verified
Proof: one
Test: none

`0x623800` reads health `[+0x110]+0x40` and `[+0x110]+0x224` bit 0x20 and starts 0x461d80(sound,
position, 1) into `+0xcbc` when it holds none. `0x6237e0` calls 0x461f80 on `+0xcbc` and clears it.

## Death thud and emote sounds

`$DTH` plays a death thud for a unit below size class 5: the `DeathThudLookups` sound for its size
and ground, the water one when it stands in liquid, and none when the surface lies more than two
yards above its feet. `$ESD` plays the sound of the unit's emote state when its `Emotes`
row has kind 2. A text emote's voice plays only for the player or a party member's emote, and only
with `EmoteSounds` on.

Addresses: `0x6236e0`, `0x62372a`, `0x62373d`, `0x6239f0`, `0x623c80`, `0x623cc2`, `0x623c10`, `0x623d10`
Confidence: verified
Proof: one
Test: none

`0x6236e0` asks `CWorld::GetEntityLight` for the liquid at the unit; `0x62372a` returns when the
surface minus z is above 2.0. `0x62373d` needs `0x625500` below 5 and `+0xc60`, the
ground's `TerrainType` id, a row whose `+0x10` is below the size's count; liquid takes the pair's `+4`. `0x6239f0(point)` reads `Emotes` by `UNIT_NPC_EMOTESTATE` and
plays `+0x18` when `+0x10` is 2. `0x623c80(emote, x)` (0x49b4e9) needs the CVar and
`ClassifyGuidForPlayer` 0 or 2, gets the voice from 0x459270 by display race and gender, and calls
`0x623c10(voice, 0, x)`, which plays at attachment 17, attaches the sound to the unit and replaces
`+0xb28`; with its second argument set it also needs `EmoteSounds`. `0x623d10` plays `0xc4e0c8`.

## Weapon and armour sounds

Slot 35 plays the unit's material sound, the model data's `+0x28`; the player's uses the material
of its chest item. Slot 36 gives the armour kind an impact sounds against: the sound data's `+0x60`
through `0x80db94` (0, 8, 7, 9), or for a player the material flags of its chest armour. `0x623640`
plays a parry or block impact from the attacker's weapon and the victim's, and `0x623870` gives a
hand's weapon swing size.

Addresses: `0x623610`, `0x62fa30`, `0x6238f0`, `0x80db94`, `0x62fb70`, `0x623640`, `0x623870`
Confidence: verified
Proof: one
Test: none

`0x623610` calls 0x4584e0(`[+0xb3c]+0x28`, position). `0x62fa30` takes the GUID of inventory slot
4 from vtable `+0xb8` when the inventory holds more than 4, and 0x5da110 of that item. `0x6238f0`
returns 0 for `+0x60` of 4 or more. `0x62fb70` reads that item's `+0x318` info: class 4, and the
`Material` row of its byte `+0x2`, whose flags `+0x4` give 2 for 0x2, else 1 for 0x4, else 0. `0x623640(block, record, point)`
calls 0x457dc0 with `0x625460(attacker, off hand, 0)`, `0x625400(block)` and the record's 0x80 bit.
`0x623870(mainHand, out)` stores the `ItemSubClass` row's `+0x24` from 0x588d10 for the hand's
weapon, or 0 when the hand shows nothing.

## Splashes and greetings

`0x6237c0(point)` plays the unit's race splash where it starts to swim. `0x623910` picks the next
greeting of a creature clicked again and again: the `NPCSounds` greeting for the first five clicks,
then the row's third sound file by file, then the greeting again from the start.

Addresses: `0x6237c0`, `0x623910`, `0x623947`, `0x623967`
Confidence: verified
Proof: one
Test: none

`0x6237c0` calls 0x4588d0 with `+0xb18`, the `ChrRaces` `+0x34` (`units`), from the swim start
of 0x6030c0. `0x623910(count, sound, index)` reads the `NPCSounds` row of the display `+0xb34`
`+0x2c`; without one it returns −1. `0x623947`: a count below 5 gives `+0x4` with index −1 and
returns `count + 1` (`0x62395f`). `0x623967`: past that, `count − 5` (`0x62396a`) below 0x458f70's number for `+0xc` gives
`+0xc` at that index and `count + 1`; at or above it gives `+0x4` and 0. Its caller is the greeting
(`unitnames`, Greetings and unit sounds).

## The precast sound

On `$TRD` a player plays the sound of the spell it is preparing: the visual `+0x38` sound one yard
above it. A missing spell prints `NOSPELLIDFOUND|%d`, a missing visual `SPELLVISUALIDNOTFOUND|%d`.

Addresses: `0x62faa0`, `0x62fab9`, `0x62faf8`, `0x62fb2a`
Confidence: verified
Proof: one
Test: none

`0x62faa0` reads `+0xc8c`; `0x62fab9` looks it up and `0x62faf8` calls `UnitGetSpellVisual`. A
`$CAH` with no pending swing also calls it (`unitcombat`). Both messages go through `SysMsgPrintf`.

## The event sound

`$CSD` plays the event's own payload dword as a `SoundEntries` id, through the emote voice
`0x623c10` with its second argument 1: at attachment 17, attached to the unit, replacing `+0xb28`,
and only while `EmoteSounds` is on. No table is read and any unit plays it.

Addresses: `0x5ffde4`, `0x5ffeed`, `0x623c10`, `0x623c21`, `0x623c3a`, `0x623c4c`, `0x623c63`, `0x623c70`
Confidence: verified
Proof: one
Test: none

`0x5ffde4` matches `0x44534324`; `0x5ffeed` passes `[event+0xc]` with `push 1` and `push −1` to
`0x623c10`, whose set second argument makes it look up the `EmoteSounds` CVar (`0x623c21`) and
return while its integer is 0. The play is `0x459230` at the attachment point (`0x623c3a`), then
`Sound::Set3DUpdateHandle` with the unit's GUID and `Sound::Stop` of the old `+0xb28`.

## Loot and auto-equip item sounds

Taking an item from a loot window plays kind 0 of its display's `ItemGroupSounds` row before the
client asks the server for it with opcode 0x108; the auto-loot walk does the same for each item it
takes, but of the items whose record has `+0x194` 1 and quality 2 or more it signals event 0x11f for
the first and skips the rest, with no sound and no request. Taking the money plays `LOOTWINDOWCOINSOUND`, and the
money change then plays it again (`playerfields`). An auto-equip from a bag puts the item on the
cursor, which plays kind 0, then equips it, whose cursor drop plays kind 1; the kinds are named
only by this use.

Addresses: `0x4c2790`, `0x4c27f9`, `0x4c2803`, `0x4c2926`, `0x4c2840`, `0x4c1fa0`, `0x4c1ffc`, `0x4c2020`, `0x4c21b3`, `0x4c21c4`, `0x4c21e2`, `0x4c2204`, `0x4c21ee`, `0x5e1ad0`, `0x5ea8c0`, `0x5e1840`, `0x5e18c9`, `0x5e18d5`, `0x494b60`, `0x494c4a`, `0x5e1480`, `0x5e1567`, `0x5e1763`, `0x495190`, `0x49520a`
Confidence: verified
Proof: one
Test: none

`0x4c2790` reads the slot's item record through `0x55ba30` (`0x4c27f9`) and calls
`0x457ff0(0, [record+0x18])` at `0x4c2803` or `0x4c2926`, then `0x5e1ad0`; for the money slot it
plays at `0x4c2840` and calls `0x5ea8c0`. `0x4c1fa0` sends the money first (`0x4c1ffc`) and plays the
coin at `0x4c2020` while `[0xb71bc4]` is 0; it plays each item at `0x4c21ee` unless `+0x194` is 1
and the item's `+0x1c` is 2 or more (`0x4c21b3`): the first such item signals event 0x11f and sets
`ebx` (`0x4c21e2`), after which `0x4c21c4` jumps every later such item to the next slot
(`0x4c2204`). `0x5e1840` calls `SetCursorItem` (`0x5e18c9`), which plays `0x457fb0(0, item)` at
`0x494c4a`, then `0x5e1480` (`0x5e18d5`), which calls `0x495190` with `edx` 1 at `0x5e1567` or `0x5e1763` (`ecx` 1 at the first, `ebx` at
the second);
its held-item arm plays `0x457fb0(1, item)` at `0x49520a` when the held GUID resolves.
