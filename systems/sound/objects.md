# sound: object and combat sounds

The sound side of the unit, item and combat sounds: the tables `SoundInterface.cpp` and
`ISoundInterface.cpp` build from the DBCs, the functions the object code calls with a position,
and the server's object sounds with the volume duck they cause. Which events call them, and with
what, is the object code's (`../objectlayer/`).

Roots: `0x457040`, `0x458380`, `0x4584e0`, `0x458530`, `0x457dc0`, `0x457ec0`, `0x457f20`, `0x457f60`, `0x458610`, `0x459250`, `0x458a50`, `0x458fb0`, `0x4591f0`, `0x457960`, `0x457a50`, `0x45c490`, `0x45cb00`, `0x45cb60`

## Footstep table

`0x457040` builds, per CreatureFootstepID of FootstepTerrainLookup, two arrays indexed by
TerrainTypeSounds id: the dry sound (column 3) and the wet sound (column 4). The arrays hold
TerrainTypeSounds' largest id + 1 entries and the row's index column is written without a bound
check. `0x458450(footstep, terrain, wet)` reads the TerrainType row's sound id (`+0x10`) and returns
that entry, again unchecked, or 0 for an unknown terrain type or footstep id.

Addresses: `0x457040`, `0x457047`, `0x457053`, `0x45710d`, `0x457137`, `0x457181`, `0x457193`, `0x458450`, `0x45845a`, `0x458480`, `0x4584c4`, `0x4584d4`, `0xb05f50`
Confidence: verified
Proof: one
Test: none

`0x457047` empties the table, deleting its nodes. The row walk runs from the last row; `0x45710d`
adds a node keyed by row `+4` and grows `+0x18` and `+0x28` to the size; `0x457181` and `0x457193`
store row `+0xc` and `+0x10` at index row `+8` with no compare. `0x4584c4` picks `+0x28` when `wet`
is set and `0x4584d4` indexes it by the TerrainType sound id unchecked.

## Footstep sounds

`0x458380(footstep, pos, terrain, wet, far)` plays a footstep when MasterSoundEffects is on and the
position is within 20 yards of the listener, or 50 with `far`: the table's sound, on bus 9 (cap 6),
at the position. Equal distance plays. A NaN distance plays.

Addresses: `0x458380`, `0x458391`, `0x4583a9`, `0x4583b0`, `0x4583be`, `0x45840b`, `0x458414`, `0x458424`, `0x458437`, `0x803140`, `0x803144`
Confidence: verified
Proof: one
Test: `sound::env_sound_audible`

The radius is `[0x803144]` (50.0) or `[0x803140]` (20.0), squared in the register and stored as a
float; `fcomp` with `test ah,0x41; je` skips only `d² > r²`. The play is `0x458890(9, id, pos, −1,
1.0)`; the caller is `0x62342a`.
`env_sound_audible` runs `0x458380` whole.

## Material and sheathe sounds

`0x4584e0(material, pos)` plays a Material row's sound (column 2) 2.0 yards above the position, on
bus 0. `0x45c490` builds, per SheatheSoundLookups column 1, two arrays indexed by Material id from
columns 5 and 6; a row whose column 4 is 0 fills every material, else the one in column 3.
`0x458530(item, first, pos)` plays, when MasterSoundEffects is on, the entry for the item's first
and third bytes 2.0 yards above the position, on bus 0, from column 5's array when `first` is set.
Column 2 is not read.

Addresses: `0x4584e0`, `0x4584fe`, `0x45851d`, `0x45c490`, `0x45c4f6`, `0x45c5ec`, `0x45c5f1`, `0x45c619`, `0x458530`, `0x45856a`, `0x4585bd`, `0x4585c7`, `0x4585cb`, `0x801628`, `0xb061e0`, `0x4585d0`
Confidence: verified
Proof: one
Test: `sound::sound_emit_pos_z2`

`0x4584fe` reads Material `+8`; the height is `fadd [0x801628]` (2.0). `0x45c4f6` keys the table by
row `+4`; `0x45c5f1` tests `+0x10`, filling with `+0x14` and `+0x18`, else stores them at index
`+0xc` when below the size. `0x458530` keys by the item's first byte and indexes by its third,
bounded (`0x4585cb`); `first` nonzero takes the `+0x18` array.
`sound_emit_pos_z2` runs `0x4584e0` whole and `0x4585d0` to `0x4585f7`.

## Weapon impact sounds

`0x45cb60` sizes a table of 10 impact kinds per weapon subclass and fills it from WeaponImpactSounds:
for the subclass in column 1 and the type in column 2, kind `i` gets the normal and critical sounds
of columns 3 + i and 13 + i. `0x457ec0(weapon, kind, critical, pos)` plays the entry of the weapon's subclass and
type on bus 10 (cap 4). `0x457dc0(weapon, item, critical, pos)` takes the kind from the struck item:
5 or 6 for a weapon (class 2) and 3 or 4 for armor (class 4) by a test of its third byte, else 0,
and plays 2.0 yards higher.

Addresses: `0x45cb60`, `0x45cb68`, `0x45cba3`, `0x45cc40`, `0x45cc65`, `0x457ec0`, `0x457ef2`, `0x457f0b`, `0x457e80`, `0x457e8f`, `0x457ea5`, `0x457dc0`, `0x457de6`, `0x457dfd`, `0x457e29`, `0xb06224`
Confidence: verified
Proof: one
Test: `sound::sound_emit_pos_z2`

The table holds `0x53f880()` subclasses of 0xa0 bytes; entry `(sub, kind)` is four dwords, normal and
critical for type 0 then type 1 (`0x45cc65`). `0x457e80` returns subclass byte 1 and type
`0x5d9a50(byte 2) != 0`, or `0x53f8a0()` and type 0 without a weapon. The dword index is
`critical + 2·(type + 2·(kind + 10·sub))` (`0x457ef2`). Neither the build nor the plays bound the
subclass.
`sound_emit_pos_z2` runs `0x457e12` to `0x457e3d`.

## Swing, miss, deflect and absorb sounds

`0x45cb00` fills a 3 × 2 table from WeaponSwingSounds2 (type below 3, critical below 2).
`0x457f60(type, critical, pos, quiet)` plays it on bus 6 (cap 2) at volume 0.5 when `quiet`, else
1.0. `0x459250(oneHanded, pos)` plays the row named `(DONOTRENAME)Combat Miss 1H` or
`(DONOTRENAME)Combat Miss 2H`; `0x457f20(pos)` plays `(DONOTRENAME)ShieldWoodImpact` and
`0x458610(pos)` `(DONOTRENAME)AbsorbGetHit`, those two 2.0 yards higher. All three are on
bus 0.

Addresses: `0x45cb00`, `0x45cb36`, `0x45cb3d`, `0x45cb4f`, `0x457f60`, `0x457f63`, `0x457f74`, `0x457f91`, `0x459250`, `0x457f20`, `0x458610`, `0xb06bd4`
Confidence: verified
Proof: one
Test: `sound::sound_emit_pos_z2`

The index is `(critical != 0) + 2·type` (`0x45cb49`, `0x457f8d`). `0x459250` takes `0xb05fb0` (1H)
when `cl` is set, else `0xb05fac`. The named ids come from `interface#named-sounds`.
`sound_emit_pos_z2` runs `0x457f20` and `0x458610` whole.

## Spell school sounds

`PlaySpellSchoolSound(spell, unit)` plays the sound of the Resistances row of the spell's school
(Spell column 1) at the unit's position on bus 0, and nothing for a null unit or an unknown row.

Addresses: `0x458a50`, `0x458a73`, `0x458a88`, `0x458a9e`, `0x458aa6`
Confidence: verified
Proof: one
Test: none

`0x458a73` reads Spell `+4`, `0x458a88` the Resistances row, `+8` its sound; the position is the
unit's `vtable+0x14`.

## Server object sounds

The server's object-sound message (opcode 0x278) for a unit calls `0x458fb0(unit, id)`. A sound with
no definition, or with flag 0x2, plays once at the unit on bus 0. Otherwise a unit that already has
a live server sound gets nothing; a new one plays as a 3D sound with no category, follows the unit,
and fills a slot of the server sound registry with the unit's GUID. `0x4591f0(unit)` reports whether
a unit holds a slot; the unit sound code asks it (`0x6234cb`, `0x623a59`).

Addresses: `0x458fb0`, `0x458fd2`, `0x458ff3`, `0x459014`, `0x459048`, `0x459062`, `0x459070`, `0x45907a`, `0x4590c0`, `0x4590c9`, `0x4590e0`, `0x459120`, `0x4591f0`, `0xb05f30`, `0x48f8ac`
Confidence: verified
Proof: one
Test: none

`0x4590c0` returns true for a non-null definition (`0x45cda0`) with `+0x7c & 2` clear; it never
reads the registry. `0x459048` is `0x45cf00(culled,
keep 1, noCategory 1, bus 0, id, pos, −1, 1.0)`. `0x459120` reuses the last empty slot or grows the
16-byte slot array (granularity 16 or less). The other objects in a 0x278 message play once with
`0x458870` (`0x48f8d3`).

## The duck

The first server sound to fill an empty registry records the current SoundVolume and starts a ramp.
Each frame the registry drops ended or cut-off sounds; while any remains the Sound factor ramps
linearly from the recorded volume `V` to `V · 0.5` over 500 ms, and when the last one goes it ramps
back from `V · 0.5` to the SoundVolume CVar over 3000 ms. Each ramp's end applies its target: `V ·
0.5` for the ramp down, the CVar for the return. Changes to SoundVolume are ignored while a slot is
held or a ramp runs, so only the return ramp picks them up.

Addresses: `0x457960`, `0x457970`, `0x45798c`, `0x457999`, `0x4579cb`, `0x4579e6`, `0x4579ba`, `0x4579d6`, `0x4579f7`, `0x457a0d`, `0x457a22`, `0x457a37`, `0x457a50`, `0x457a80`, `0x457a92`, `0x457aa4`, `0x457ac4`, `0x45908c`, `0x45909f`, `0x457430`, `0xb05f34`, `0xb05f88`, `0xb05fc0`, `0x7ffa24`
Confidence: verified
Proof: one
Test: `sound::music_volume_fade`

The down ramp's start is the clock stored raw in `[0xb05fc0]` (`0x45909f`): a first fill at clock 0
stores 0, which reads as no ramp, so that fill gets no duck. `0x457960(now)` starts the
return ramp on the frame the registry empties, storing `now`, or 1 when `now` is 0 (`0x457999`);
past the duration (`jle`) it applies the target (`V · 0.5` at `0x4579cb`, the CVar at `0x4579e6`)
and clears the start. The value is `start + (T − start) · (e / d)` with `e / d` from `fild`/`fidiv`
in the register. `[0x7ffa24]` is 0.5. `0x457430` returns true for a non-null slot or a running ramp.
At world teardown `0x457cc0` destroys every slot's sound and frees the registry.
`music_volume_fade` runs `0x457960` whole.
