# objectlayer: unit animation

How a unit picks and plays its model's animations: the base animation a chain of resolvers picks
from the unit's state, the play call that splits it across the full body, the upper body and the
mount, the animations that follow one another when a sequence ends, the model events a sequence
fires, emotes, and drawing and sheathing weapons. Animation ids are rows of `AnimationData.dbc`,
named here by its `Name` column; 208, one past the last row, means none. The flag word `+0xd58`
holds the unit's animation state bits.

Roots: `0x5fd9e0`, `0x5fe2f0`, `0x5fc920`, `0x5ffbd0`, `0x611cf0`

## Choosing the base animation

`0x5fd8b0(mask)` starts from 208 and asks ten resolvers in order; the first to claim the unit
ends the chain, writing its animation only when its bit is in `mask`. A jumping unit that is also
falling, or one whose mover is mid-jump, gets none. With nothing claimed it returns a pending
animation, else the idle one. `0x5fd9e0` plays the result when the model is loaded.

| bit | resolver | claims |
|---|---|---|
| 0x1 | `0x5fcff0` | dead: 6 Dead, swimming 132 Drowned |
| 0x2 | `0x5fd050` | 0x8 of `+0xd58`; stand state 9 (201, 202) and leaving it (127) |
| 0x4 | `0x5fd100` | moving: swim, fly, back, stealth, sprint, run, walk |
| 0x8 | `0x5fd260` | looting: 50 Loot |
| 0x10 | `0x5fd2e0` | channelling or precasting: the cast kit's animation |
| 0x20 | `0x5fd360` | attacking: a pending swing or the weapon's ready animation |
| 0x40 | `0x5fd3f0` | turning in place: 11 or 12 |
| 0x80 | `0x5fd460` | ranged weapon drawn: a load or hold animation |
| 0x100 | `0x5fd550` | stand states 1 to 8 |
| 0x200 | `0x5fd770` | `UNIT_NPC_EMOTESTATE`'s emote |

Addresses: `0x5fd8b0`, `0x5fd9e0`, `0x5fcff0`, `0x5fd050`, `0x5fd100`, `0x5fd260`, `0x5fd2e0`, `0x5fd360`, `0x5fd3f0`, `0x5fd460`, `0x5fd550`, `0x5fd770`, `0x5fd830`, `0x5fd658`, `0x5fcdc0`, `0x5fce04`, `0x5fce14`, `0x5fce30`
Confidence: verified
Proof: one
Test: `objectlayer::uh_fall_timer_anim_difftest`

A resolver that does not claim returns `(mask & −2·bit) == 0`, so the chain also ends once no
later bit is asked; the last tests its own bit. Moving (`0x5fd100`): swimming gives 42 Swim, 45 backward, 41 turning, 43 or
44 strafing; a flying spline 135 Fly; backward 13; stealth (bit 1 of `UNIT_FIELD_BYTES_1` byte 3)
119; a speed of 11 (0x80c484) or more 143 Sprint; above twice the walk speed `+0xa30` 5 Run; else 4 Walk.
Stand states through `0x5fd658`: 1 gives 96 or, already sitting, 97; 3 gives 99 or 100; 4 to 6
give 102 to 104; 7 gives 6 or 132; 8 gives 114 or 115; 0 after 1, 3 or 8 gives 98, 101 or 116.
The ready animation `0x5fcdc0` maps the main-hand weapon subclass through `0x5fce14` into
`0x5fce04`: one-handers 26, two-handers 27, polearms, staves and spears 28, else 25. The idle
`0x5fd830` is 41 swimming, 120 stealthed, 193 Hover when flying, else 0 Stand. `0x5fce30` allows
a shuffle while turning, not flying, and not in an emote, cast or ranged animation.

## Playing an animation

`0x5fe2f0(anim)` plays an animation on the body, the upper body or the mount. An unloaded model
keeps it pending at `+0xd64`. A dead unit, or one with bit 0x2000, takes only death animations;
bits 0x40000 and 0x80000 (a lift-off, landing or knockdown) refuse it. Special1H and Special2H
become the unarmed or two-handed form. A swing over a swing restarts only the upper body. Moving
animations scale their speed to the mover's. The tracks then play through `0x5fdba0`, and the
sheathe state follows the new animation.

Addresses: `0x5fe2f0`, `0x5fe3bc`, `0x5fe3e9`, `0x5fe43c`, `0x5fe48e`, `0x5fe526`, `0x5fe6c8`, `0x5feab5`, `0x5febe8`, `0x5fec0d`, `0x5fe1f0`, `0x5fdb50`, `0x5fdba0`, `0x5fdd7c`, `0x5fdd90`, `0x5fde80`, `0x5fdf80`, `0x60f5b0`
Confidence: verified
Proof: one
Test: none

`0x5fe3bc` turns 57 and 58 into 118 without weapons, else 58 for a two-hander (`0x5fc390`), else
57. `0x5fe1f0` reads the mount, body and key-bone (`+0xd5c`) track states; `0x5fdb50` returns the
key bone's animation, else the body's. With bit 0x20 and a swing over a swing (`0x5fe43c`) it
calls 0x712910 on the key bone and stores the swing at `+0xd60`. A locomotion animation's speed
is the mover's speed over the sequence's (`0x5fe526`). The split between upper body and full body
rests on the sets of the next entry. `0x5fde80` lets an effect on `+0xb4` with flag 0x1000 force
its animation, or one with flag 0x4 freeze the tracks. `0x5fdba0` plays a track and sets
`+0xd58` bits through `0x5fdd90`: 39 bit 0x4, 121 0x80000, 127 0x8, 192 and 200 0x40000.
`0x5fdf80` reads the animation's `AnimationData` row: 0x4 of its `+0x8` puts the weapons away,
0x20 or a swing while attacking draws them, and 112 nocks the ammunition. `0x60f5b0` tests 0x80 of
the row's `+0xc`.

## Animation sets

Byte-table switches and comparisons sort animations into the sets the player and the resolvers
test. Each table switch's dword table has two cases, the set's and the rest.

| function | set |
|---|---|
| `0x5fcbb0` | spell casts 2, 32, 33, 53, 54 |
| `0x5fcc10` | swings, parries, blocks, dodges: 22 ids from 10 to 118 |
| `0x5fced0` | emotes: 31 ids from 60 to 195 |
| `0x5fda90` | death: 1, 6, 131, 132 |
| `0x5fe180` | ranged attack and load: 46, 49, 105 to 107, 109 to 112 |
| `0x5fec80` | full-body base: 33 ids, stand to JumpLandRun |
| `0x5fed90` | upper-body: 95 ids from 2 to 195 |
| `0x5fee80` | moving: 15 ids, walk to JumpLandRun |
| `0x60b6e0` | unarmed: 16, 20, 25, 117, 118 |

Addresses: `0x5fcbb0`, `0x5fcbd0`, `0x5fcbd8`, `0x5fcc10`, `0x5fcc30`, `0x5fcc38`, `0x5fced0`, `0x5fcef4`, `0x5fcefc`, `0x5fda90`, `0x5fdabc`, `0x5fdac4`, `0x5fe180`, `0x5fe1a0`, `0x5fe1a8`, `0x5fec80`, `0x5feca0`, `0x5feca8`, `0x5fed90`, `0x5fedb4`, `0x5fedbc`, `0x5fee80`, `0x5feea4`, `0x5feeac`, `0x60b6e0`, `0x60b700`, `0x60b708`, `0x5fccb0`, `0x5fcf90`, `0x5fcfb0`, `0x5fcfd0`, `0x5fde40`, `0x5fde60`, `0x5fec40`, `0x5fec60`, `0x5fed70`, `0x5fc390`, `0x5fc3c8`, `0x5fc3d0`, `0x60a710`
Confidence: verified
Proof: one
Test: none

Each byte table maps the id minus its base (2, 10, 60, 1, 46, 0, 2, 4, 16) to the case that
returns 1 or 0; the membership above is every id whose byte names the returning case. The
comparison sets: `0x5fccb0` casts plus 46, 49, 107; `0x5fcf90` 107, 111, 112; `0x5fcfb0` 46,
105, 109; `0x5fcfd0` 49, 106, 110; `0x5fde40` 51, 52; `0x5fde60` 25 to 29; `0x5fec40` 39, 187;
`0x5fec60` 57, 58, 118; `0x5fed70` 37 to 39, 187. `0x5fc390` tests weapon subclasses 1, 5, 6, 8,
10, 12, 17, 20 through `0x5fc3d0`; `0x60a710` subclasses 2, 16, 18.

## When a sequence ends

The model calls `0x5fc920` through `0x6078d0` when a sequence ends or fires its end event. A death
stamps the time of death and adds the loot sparkle; a finished cast releases missiles; a lift-off,
landing or knockdown clears its lock. A finished full-body sequence plays its follow-on through
`0x5fc3f0`: the loop after its start, the stand-up after a sit or kneel, or a fresh base pick.

Addresses: `0x5fc920`, `0x5fcb40`, `0x5fcb50`, `0x5fc3f0`, `0x5fc7ec`, `0x5fc848`, `0x607800`, `0x6078d0`
Confidence: verified
Proof: one
Test: none

`0x5fc920` stamps `+0xcb4` for 1 and 131, calls `0x60c9b0` for casts and shots with missiles on
`+0xac`, clears 0x40000 for 192 and 200 and 0x80000 for 121. Its event branch switches through
`0x5fcb50`: 39 clears bit 4, 89 and 90 finish the sheathe, 127 clears bit 8. A bone track's end
continues a sheathe on bone 3 or 2 or stops the track. A body track's end clears 0x70, sets 0x10
(mount) or 0x40, restores the sheathe after an emote queue, calls `0x5fc3f0` through `0x5fc848`
into `0x5fc7ec`, and sets 0x70 again: 1 and 131 hold 6 or 132 while dead; 37, 38 give 38; 40 gives 40; 50
gives 188, which repeats while looting; 69 repeats out of combat; sits, sleeps and kneels loop
while the stand state holds and stand up or re-pick otherwise; loads give holds; 133 starts the fishing bobber and 134;
201 gives 202; the rest re-pick. `0x607800` installs the done and event callbacks with the GUID.

## Model events

A sequence's events reach `0x5ffbd0` through `0x607840`, keyed by four-character tags. Footstep
tags, two letters and a digit (`$FL0`, `$WR3`), leave footprints and splashes; `$FD1` to `$FD9`
and `$TRD` play foley; `$SHL` and `$SHR` move a weapon between hand and sheath; `$CSL`, `$CSR` and
`$CST` launch spell missiles; an unknown tag prints `UNHANDLEDANIMEVENT|%.4s`.

Addresses: `0x5ffbd0`, `0x607840`, `0x5fbf70`, `0x607920`, `0x5ffb10`, `0x5fc310`, `0x6002eb`
Confidence: verified
Proof: one
Test: `objectlayer::weapon_attach_offset_difftest`, `objectlayer::diff_transform_scale_copy`, `objectlayer::uh_placement_dist2_difftest`, `objectlayer::uh_placement_inner_range_difftest`, `objectlayer::uh_placement_ground_z_difftest`, `objectlayer::uh_placement_height_gate_difftest`

The tags are dwords compared in a binary search from `0x5ffbec`; the miss path is `0x6002eb`.
`0x5fbf70` skips a hovering, stealthed or ghost unit and one farther than 50 yards from the camera
(0x80c5b4, 2500 squared); it prints a footprint, adds the model's footstep shake (`+0x2c` of
the `CreatureModelData` row) and, within 25 yards (0x80c5b0) and with `showfootprintparticles` on,
sprays. The print's sizes come from `0x607920`: the mount's `+0xcc0` and scaled `+0xcc4`,
`+0xcc8` when mounted, else `+0xc5c`, `+0xc64`, `+0xc68`, which `0x5fc310` copies from the
`CreatureModelData` row. `0x5ffb10(hand, bit)` takes the hand's item and the sheathe state
before or after the change by `+0xd58`, re-attaches the weapon and plays the sheathe sound
0x458530. `0x607840` drops events whose position is not a number. Slices: uh_placement_dist2_difftest `0x5fbfdb`-`0x5fc015`; uh_placement_inner_range_difftest `0x5fc0b3`-`0x5fc0c1`; uh_placement_ground_z_difftest `0x5fc157`-`0x5fc197`; uh_placement_height_gate_difftest `0x5fc157`-`0x5fc172`.

## Emotes

`0x5fcd20` plays an `Emotes.dbc` row's animation unless it is already playing, the unit
channels or precasts, or, except for 192 and 200, it fights. Talk emotes come from five rows
chosen at start-up by flag. An emote queue at `+0xc20` holds up to four timed emotes, each
started 500 ms after the previous one ends.

Addresses: `0x5fcd20`, `0x603a60`, `0x60bb30`, `0x60bb80`, `0x60bbc0`, `0x60bcb0`, `0x60bce0`, `0x60b570`, `0x5fd680`
Confidence: verified
Proof: one
Test: none

`0x603a60` fills `0xc4d95c` with the rows flagged 0x8, 0x10, 0x20, 0x40 and 0x100: talk,
question, exclamation, shout and laugh. `0x60bb30(kind)` plays one unless the unit swims or
sleeps. `0x60bbc0(pairs, count)` keeps the rows `0x60bcb0` allows (no special procedure), adds
the current time to the first delay and, when an emote animates, keeps the sheathe state to
restore. `0x60bce0` plays a due slot and delays the next by the sequence length plus 500; the
last sets bit 0x8000 and `+0xc24` −1. `0x60bb80` passes a list's two words. `0x60b570`
re-picks when `UNIT_NPC_EMOTESTATE` changes. `0x5fd680` is true for a moving unit with no
overriding state.

## Drawing and sheathing

The sheathe state, 0 put away, 1 melee, 2 ranged, is `+0xd40`, the previous one `+0xd3c`.
`0x611cf0(state, instant, send)` changes it: three classes cannot draw a ranged weapon; the
active player sends 0x1e0 with the state. An instant change moves the models at once; otherwise sheathe
animations play on the arm bones and the model events move the models.

Addresses: `0x611cf0`, `0x611770`, `0x611b60`, `0x6118a0`, `0x611960`, `0x611a20`, `0x604c70`, `0x5fdf80`
Confidence: verified
Proof: one
Test: none

`0x611cf0` refuses 2 to a player whose class (`UNIT_FIELD_BYTES_0` byte 1) has no `ChrClasses`
row or a nonzero `+0x40`: Paladin, Shaman, Druid.
`0x611770` re-attaches through `0x60b590` and `0x611e10`, and on putting a ranged weapon away
detaches attachment 35, removes the outfit slot 11 and stops an auto-move. `0x611b60` starts from
the old state. `0x6118a0` (main hand, bone 3, bit 0x100000), `0x611960` (off hand, bone 2, bit
0x200000) and `0x611a20` (both, 0x300000) play 90 HipSheath for sheath types 3 and 7
(`1 << type & 0x88`), else 89 Sheath. `0x604c70` applies `UNIT_FIELD_BYTES_2` byte 0, the
server's sheathe state, through `0x611cf0(state,1,0)`.

## Reactions and move animations

Hits, parries, deaths and move messages play one-shot animations. A wound plays 10 for a
critical hit, 9 while attacking, else 8, upper body only when moving. A parry plays by weapon
type. A death plays 1, or 131 swimming. Move messages start the jump, the fall, or a re-pick.

Addresses: `0x60ea70`, `0x60ec00`, `0x60ec84`, `0x60ec98`, `0x60ea30`, `0x60e480`, `0x60e520`, `0x60e534`, `0x602c40`, `0x608d50`
Confidence: verified
Proof: one
Test: none

`0x60ea70(critical)` skips a dead or feigning unit, an unloaded model, a creature whose type flags
have 0x8 (`0x6125f0`) and one with a flag-4 effect, and plays only when the model has the
sequence. `0x60ec00` maps the main-hand subclass through `0x60ec98` into `0x60ec84`: one-handers
21, two-handers 22, polearms and staves 23, fist weapons 20, bows, guns and thrown none.
`0x60ea30` skips a unit already in a death animation. `0x60e480(opcode)` switches 0xb5 to 0xcb
through `0x60e534`: starts end the ranged pose and re-pick, 0xbb plays 37 JumpStart, stops and
turns re-pick; 0xf0 plays 40 Fall. `0x602c40` plays 40 for a falling unit with health. Slot 18
`0x608d50` plays 94 MountSpecial.
