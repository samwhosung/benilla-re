# objectlayer: unit combat

How a unit shows melee combat: the attack messages, the attacker state the server sends for each
swing, and how the client holds that swing until the attacker's animation strikes, then shows it
on the victim: the floating word or number, the wound, parry or block pose, the blood, the impact
sounds. Also the attack start and stop, environmental damage, a unit's death, and honor.

UnitCombat_C.cpp runs from `0x624370` to `0x625e40`, its initializers excepted (`initializers`).
Its message handler takes eight opcodes. A swing's record is kept on the attacker, 0xec bytes at
`+0xd70`, until the attacker's model fires `$CAH`, the next swing arrives, or the attack stops.
The combat log lines a swing writes are `combatlogmessages`.

Roots: `0x624370`, `0x6247d0`, `0x624f20`, `0x625190`, `0x6251e0`, `0x625270`, `0x625400`, `0x625450`, `0x625460`, `0x625500`, `0x625520`, `0x625bd0`

## Attack messages

`0x625520` registers `0x6255b0` for eight opcodes, then the combat log's; `0x625bd0` clears them.

| opcode | body | action |
|---|---|---|
| 0x143 | attacker GUID, victim GUID | the attacker's target `+0xc48` is the victim; the player raises `PLAYER_ENTER_COMBAT` |
| 0x144 | packed attacker, packed victim, u32 | `0x624e40` stops the attack; the player raises `PLAYER_LEAVE_COMBAT` |
| 0x145, 0x146 | none | the attack error state 1 or 2 (`players`, Attack error repeat) |
| 0x148, 0x149 | none | the active player's melee attack stops |
| 0x14a | the attacker state | below |
| 0x1fc | GUID, u8 type, i32 damage, absorbed, resisted | `0x624f30`, environmental damage |

Addresses: `0x625520`, `0x625bd0`, `0x6255b0`, `0x6255df`, `0x625aec`, `0x625b0c`, `0x62568d`, `0x625714`, `0x625a8a`, `0x625ab8`, `0x6255ed`, `0x624e40`
Confidence: verified
Proof: one
Test: none

The switch takes `opcode − 0x143`, bounded by `cmp 0xb9`, through the bytes `0x625b0c` into the
eight cases `0x625aec`: 0x143 case 0, 0x144 1, 0x145 2, 0x146 3, 0x148 and 0x149 4, 0x14a 5, 0x1fc
6, every other value 7, the exit. Each handler returns 1 except 0x14a, 0x145, 0x146, 0x148 and
0x149, which return 0. `0x624e40(victim, a, b)` clears `+0xc48` to `+0xc54`, shows the pending
swing on its victim through `0x6243e0`, zeroes the record, and when the u32 is nonzero and the
victim resolves sets bit 0x1 of `+0xc58` and turns `+0xcac` to the heading toward it.
`0x624370` sends 0x142 with no body and sets `+0xc54`.

## The attacker state

0x14a carries one swing. `0x625c60` reads it into a record: hit info, packed attacker, packed victim,
total damage and a byte count `n` of damage parts, each a school, a float, a damage, an absorb and a
resist; then the victim state, two u32 and a blocked amount. Hit info 0x1 adds a u32, eight floats,
five float pairs and a u32. A swing with no damage and a blocked amount becomes state 5. Nothing
bounds `n`: the parts are arrays of five, and a sixth part on writes into the fields after them.
The 24th part writes past the 0xec-byte record, and the 33rd reaches the handler's saved `ebp`.

Addresses: `0x625c60`, `0x625c95`, `0x625cb1`, `0x625cf7`, `0x625d2f`, `0x625e20`, `0x6263a0`
Confidence: verified
Proof: one
Test: none

| offset | field |
|---|---|
| `+0x00`, `+0x08` | attacker and victim GUIDs |
| `+0x10`, `+0x14` | hit info and total damage |
| `+0x18`, `+0x54`, `+0x68`, `+0x7c`, `+0x90` | five schools, floats, damages, absorbs, resists |
| `+0xa4`, `+0xa8`, `+0xac`, `+0xb0` | victim state, u32, u32, blocked |
| `+0xb4` to `+0xd8` | the hit info 0x1 extras |

`0x625c95` reads the byte; the loop `0x625cb1` stores part `i` at `+0x18+4i`, `+0x54+4i` (a float),
`+0x68+4i`, `+0x7c+4i` and `+0x90+4i`. The extras' pairs go to `+0x2c+4i` and `+0x40+4i`. `0x625e20`
sets `+0xa4` to 5 when the victim resolves, `+0x14` is 0 and `+0xb0` is not. `0x6263a0` clears the
record, setting the five schools to −1. The record sits at `[ebp-0x110]` in `0x6255b0`. Part `i` (from 0) writes as far as `+0x90+4i`: `i = 23` gives `+0xec`, the
record's end, and `i = 32` gives `+0x110`, the saved `ebp`.

## Holding a swing

On 0x14a the attacker draws its weapons; a swing still pending on it floats its word or number
on its victim at once, and the new record replaces it. The attacker plays its swing animation, and
the active player targets an attacker that hits it while it has no target. The victim is shown
the swing when the attacker's model fires `$CAH` or `$AH0` to `$AH3`, or at once when the attacker
is not in sight. The combat log line is written on receipt.

Addresses: `0x62578d`, `0x625829`, `0x62583f`, `0x625898`, `0x6259b6`, `0x6259c4`, `0x6259e6`, `0x625a03`, `0x625a3e`, `0x625a78`, `0x6246a0`
Confidence: verified
Proof: one
Test: none

`0x625829` calls `CGUnit_C::SetSheatheState(1, 1, 1)` unless `+0xd40` is 1. `0x62583f` flushes a
pending record with both GUIDs through `0x6243e0` on its victim, then copies 0xec bytes to
`+0xd70`. `0x6259b6` runs `CGPlayer_C::OnAttackStart` for the active player with a victim.
`0x6259e6` selects the attacker when the victim is the active player and `s_targetGuid` is empty.
`0x625a03`: an auto-move in no mode 4 stops through `CGUnit_C::StopAutoMove(1, 1)`. With no attacker
`0x625a3e` runs `0x624530` on the victim. `0x625a78` calls `0x629b10`. `0x6246a0` skips a dead
attacker and hit info 0x10000; a swing record `+0xac` of 0 plays the animation, and a victim
state other than 0 plays the attacker's exertion, slot 34 with kind `(hit info >> 7) & 1`.

## The swing animation

The attacker's swing plays by its main-hand weapon: 17 Attack1H for one-handed axes, maces, swords,
exotics and miscellany; 18 Attack2H for two-handed axes, maces, swords and exotics; 19 Attack2HL for
polearms, staves, spears and fishing poles; 85 Attack1HPierce for daggers; 16 AttackUnarmed for fist
weapons, ranged weapons, no weapon or anything else. An off-hand swing (hit info 0x4) plays 88
AttackOffPierce with a dagger, 87 AttackOff with another weapon, 117 AttackUnarmedOff without.

Addresses: `0x6246a0`, `0x6246d8`, `0x62472c`, `0x624794`, `0x6247ac`
Confidence: verified
Proof: one
Test: none

The main hand's item info comes from vtable `+0x98(0, 0)` and must be class 2. The subclass, bounded
by `cmp 0x14`, indexes the bytes `0x6247ac` (`00 01 05 05 00 01 02 00 01 05 02 00 01 03 00 04 05 02
05 05 02`) into the six cases `0x624794`: 17, 18, 19, 16, 85, and 16 left as set. `0x6246d8` tests
0x4 and takes the off hand, `+0x98(1, 0)`; subclass 15 gives `0x57 + 1`.

## Showing a swing on the victim

`0x624530` shows a swing on its victim unless the victim's dynamic flags have 0x1. It floats the word
or number, then for hit info 0x2 plays the wound (critical 0x80) and, with damage in state 1 or
4, blood; then the parry or block impact sound. An absorbed or resisted swing (0x60) or an immune
one plays an impact sound; otherwise a 0x2 swing plays the victim's pain, kind 9 for a crushing
blow (0x8000), 3 for a critical, else 2 unless 0x10.

Addresses: `0x624530`, `0x624552`, `0x62456b`, `0x62457f`, `0x6245b5`, `0x62460c`, `0x624620`, `0x6243e0`, `0x625e40`
Confidence: verified
Proof: one
Test: none

`0x624552` tests `UNIT_DYNAMIC_FLAGS` bit 0x1. `0x62457f`: damage and state 1 or 4 call
`0x625010(attacker, hit info & 0x2000)`. `0x6245b5`: state 3 calls `0x623640(0, record, position)`,
5 `0x623640(1, …)`, 8 calls 0x457f20 at the position. `0x62460c`: hit info 0x60 or state 7 calls
0x458610. `0x6243e0` floats only when `0x625e40` allows (`+0xac` 0 or state 1): states 8, 3, 6, 2, 5
and 7 give words 9, 4, 6, 3, 5 and 7; else damage floats `ShowDamageNumber(attacker, damage, 0,
critical)`; else, unless the victim is the active player or hit info has 0x1000, word 10 for 0x20,
2 for 0x40, else 1. Each path ends in 0x4946d0 with the record.

## Blood

A melee hit spills the victim's blood: the `UnitBlood` row of its blood levels and the violence
level picks an effect, the front one at attachment 15 when the attacker is ahead of the victim or
out of sight, the back one at 16 when it is behind; hit info 0x2000 takes each side's second effect.

Addresses: `0x625010`, `0x62502e`, `0x6250b6`, `0x6250d9`, `0x6250e3`, `0x625177`
Confidence: verified
Proof: one
Test: `objectlayer::diff_blood_spurt_front_back`, `objectlayer::diff_blood_spurt_front_back_diff_rounding`, `objectlayer::diff_blood_spurt_dot`

`0x62502e` is `CGUnit_C::GetBloodRecord`. With `f` the victim's `CGUnit_C::GetWorldBodyFacing` and
`d` the attacker minus the victim in x and y, `0x6250b6` takes `cos f·dx + sin f·dy`; `0x6250d9`
keeps 16 below 0 (`test ah,5; jnp`) and takes 15 otherwise, NaN included. Attachment 15 reads the
row's `+0x4` or `+0x8`, 16 its `+0xc` or `+0x10`, by whether the flag is set. `0x625177` calls
`CEffect::AddEffect(attachment, 0, 0, record, attacker, victim, 0, EffectDoneCallback)` and releases
its reference (`effects`). `0x60a390` (`unitdisplay`) has two callers, this and `0x60a260`, which
only tests for a row; this one reads `+0x4` to `+0x10` (`0x6250fc`, `0x625105`), and the store's
other readers are its load, set-up and teardown `0x53eb20`. The row's five strings `+0x14` to
`+0x24`, the ground splat textures, are never read, and no creature or display flag gates the spurt. Slices: diff_blood_spurt_front_back, diff_blood_spurt_front_back_diff_rounding `0x6250b6`-`0x6250d7`; diff_blood_spurt_dot `0x6250b6`-`0x6250d2`.

## Combat model events

The attacker's model events present its swing. `$CAH` plays the weapon impact sound of a damaging
swing, slows the attacker's animation to half speed for a missed, dodged or evaded swing, and shows
the swing on the victim; `$AH0` to `$AH3` do the same with one of the attacker's four attack
sounds in place of the impact. With no swing pending a player's `$CAH` plays its precast sound.
`$CPP` plays the victim's parry, dodge or block pose. `$DTH` thuds, shakes the camera and adds the
loot sparkle. `$BWP` draws the bow. `$CSS` plays the whoosh of a missed swing or the weapon's
swing sound.

Addresses: `0x6247d0`, `0x624857`, `0x624898`, `0x624930`, `0x624986`, `0x6249b0`, `0x6249c0`, `0x6249e5`, `0x624a0c`, `0x624a94`, `0x624ab4`, `0x624b85`, `0x624ca0`
Confidence: verified
Proof: one
Test: none

`0x6247d0` resolves the record's attacker and victim. `0x624898`: with damage, `$AH0` to `$AH3`
play the attacker's own sound data `+0xb40`, its `+0x48` to `+0x54` through 0x458890 at the event and set 0x80000 in
`+0xd80`, then run the `$CAH` body. With neither unit, `0x624909` calls `0x62faa0` for a player.
`0x624930`: without 0x80000 and with damage, 0x457ec0 takes the attacker's weapon `0x625460` and
the victim's slot 36. `0x624986`: `0x624ca0` (state 0, 2 or 6) sets key bone −1 to 0.5. `0x6249b0`
runs `0x624530`; `0x6249c0` clears both GUIDs. `0x624a0c`: `$CPP` skips dynamic flag 0x20 and
stand state 7, then plays `CGUnit_C::PlayParryAnimation` for state 3, 30 for 2 or 8, 24 for 5,
clearing hit info 0x2. `0x624ab4`: `$BWP` needs a precast or auto-repeat spell with Attributes 0x2
and no movement flags 0x20200f. `0x624b85`: `$CSS` with state 0, 2 or 6 plays 0x459250 at
attachment 1, else 0x457f60 for the hand's swing sound `0x623870`.

## The ranged reload pose

On `$BWP` the unit's ranged weapon model plays 160 BowPull, stretched to end when the body's current
animation does; the ammo model is attached at 35 unless the weapon is a gun or crossbow, and
`+0xd58` gains 0x4000.

Addresses: `0x624cc0`, `0x624d1f`, `0x624d5d`, `0x624d75`, `0x624e12`, `0x624b27`
Confidence: verified
Proof: one
Test: `objectlayer::castbar_anim_speed_difftest`

`0x624cc0` needs `+0xd24` loaded with animation 0xa0 and a loaded body. `0x624d1f` reads key bone
4's state when `+0xd5c` is 4 and it plays, else key bone −1. `0x624d5d` returns for a nonzero
`+0x18`. `0x624d75`: the time left is `end − ftol(t/speed) − start` from the state; when positive,
`0x624e12` plays 0xa0 at speed `length/left`, `fild` of the length over `fidiv` of the time.
`0x624b27` reads the ranged item info, `+0x98(2, 0)`: subclass 3 or 18 keeps the ammo `+0xd28` off. Slices: castbar_anim_speed_difftest `0x624d75`-`0x624e26`.

## Environmental damage and death

0x1fc writes the environmental line, signals the active player's wound in the type's school
with absorb and resist marks, and plays the type's kit, the kit even for a hit of all zeros. A unit's death writes the death line,
shows a pending XP gain, plays the death animation and the death sound, stops its loop sound and,
for the active player, signals `PLAYER_DEAD`.

Addresses: `0x624f30`, `0x602d94`, `0x624f7c`, `0x624fb3`, `0x624fcc`, `0x625190`, `0x6251e0`, `0x809274`, `0xc4d8e4`
Confidence: verified
Proof: one
Test: none

`0x624f30(type, damage, absorbed, resisted)` writes no line and raises no event when all three
values are 0; else it calls `0x62aac0(guid, type, damage, absorbed, resisted)`. `0x624fb3` reads the school from `0x809274`:
0, 0, 0, 2, 3, 2 for types 0 to 5, marks 0x20 for absorbed and 0x40 for resisted. `0x624fcc`
plays `s_environmentalDamageKits[type]` as kind 0. `0x625190` calls `0x62c160`, `0x6251e0`,
`CGUnit_C::PlayDeathAnimation`, `0x623a40(4)` and `0x6237e0`. `0x6251e0` logs a pending gain
`+0xc80` through `0x637a80`, floats it over the active player and clears it. The landing handler
0x602d00 is `0x624f30`'s other caller (`0x602d94`): a player-driven unit, without the movement flags
0x20200800 or a mover `+0xa4` record with 0x200 and not 0x4, that falls more than 13 yards while alive, or 70 or more, plays its forced pain vocal,
slot 34 kind 2, then `0x624f30(2, 0, 0, 0)`: the fall kit with no line.

## Honor

`0x625270(guid, rank, honor)` writes the honor line: with a victim, `COMBATLOG_HONORGAIN` with its
name and `PVP_RANK_n_s` rank title for positive honor, `COMBATLOG_DISHONORGAIN` with
`PVP_RANK_CIVILIAN` otherwise, then floats `HK` or `DK`; without one, `COMBATLOG_HONORAWARD`. Each
raises `COMBAT_TEXT_UPDATE` `HONOR_GAINED`.

Addresses: `0x625270`, `0x6252de`, `0x62530c`, `0x625370`, `0x6253bb`
Confidence: verified
Proof: one
Test: none

`0x6252de` reads the victim's `ChrRaces` row by its race byte; `0x62530c`: flag 0x4 gives side 0,
else 0x2 gives 1, otherwise none, and a race with neither writes nothing. The rank text goes through
`CGUnit_C::GetGenderedText`. The line is chat type 0x51 through `0x626850`; `0x626960(0x2e, honor)`
raises the event; `CGUnit_C::ShowHonorKill(honor, sign, title)` floats it.

## Weapon and sound helpers

`0x625400(block)` returns the victim's parrying item: the main-hand weapon unless blocking, else an
off-hand weapon or shield. `0x625460(offHand, loaded)` returns a hand's weapon while weapons are
drawn, with `loaded` only when its model is loaded. `0x625500` returns the unit's sound size: the
display's `+0x24`, or the model data's `+0xc` when that is −1. `0x625c30` shakes the camera with
the model data's `+0x30`.

Addresses: `0x625400`, `0x625460`, `0x625500`, `0x625c30`, `0x624f20`, `0x625450`
Confidence: verified
Proof: one
Test: none

`0x625400` takes `+0x98(0, 0)` unless `block`, keeping class 2, then `+0x98(1, 0)` keeping class 2
or 4. `0x625460` needs `+0xd40` and class 2; with `loaded`, the body and the child at attachment
`1 + offHand`. `0x625c30` calls `CameraAddShake(id, position)` on `GetActiveCamera()`.
`0x624f20` returns its argument and `0x625450` returns 0.
