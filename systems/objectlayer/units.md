# objectlayer: units

The unit object, `CGUnit_C`, 5120 bytes with the vtable 0x80c4f8, from its constructor to its
release: start-up and shutdown of the unit code, the field watchers that turn update fields into
client work, health and death, unit flags, stand states, reactions and unit tokens, a right
click, melee attack and the caster checks of a spell, creature records, known spells, and the
unit code nothing reaches. Movement, animation, auras, display, the body each frame and names
have topics of their own: [unitmovement](unitmovement.md), [unitanim](unitanim.md),
[auras](auras.md), [unitdisplay](unitdisplay.md), [unitbody](unitbody.md) and
[unitnames](unitnames.md).

Roots: `0x6033c0`, `0x5fad50`, `0x5fb880`, `0x6041f0`, `0x60bea0`, `0x6094f0`

## The unit object

`0x5fad50(a, block)` builds the object part, points `+0x118` at the `CMovement` it
builds at `+0x9a8`, empties the 182 watcher lists from `+0x11c`, sets the unit members and the
vtable, and reads the update's movement block (`0x5ff070`). `0x5fb5e0` tears it down in the reverse order.
Vtable slots 0 to 3 are the deleting destructor, release, reactivation and the post-update pass.

Addresses: `0x5fad50`, `0x5fb131`, `0x5fb14b`, `0x5fb1d0`, `0x5fb5e0`, `0x5ff070`, `0x5ff030`, `0x5fefc0`, `0x5fefe0`, `0x5ff000`
Confidence: verified
Proof: one
Test: `objectlayer::uh_low_hp_gate_difftest`

The constructor sets `+0xb08` and `+0xc5c`, `+0xc60` to −1, the footprint sizes to 10/36, `+0xc6c`
to 1, `+0xc70` to −10, `+0xc74` to `FLT_MAX`, the body facing to the mover's, `+0xccc` and `+0xcd0`
to 1, `+0xd40` to `UNIT_FIELD_BYTES_2` byte 0 and `+0xd58` to 0x70. At `0x5fb131` it calls
`0x5ff070(block, 0)`, which copies the block's attack target GUID `+0xe8` into `+0xc48` and hands the block to `0x5ff030`
(`CMovement::ApplyObjectUpdate` with the clock; a falling unit then plays Fall, `0x602c40`), with
bit 0x80 of `+0xc58` for a block whose byte `+0xd8` has bit 0. It zeroes the 48 aura slots twice
around the low-health test at `0x5fb14b` (health under 20% of `UNIT_FIELD_MAXHEALTH`, alive). `0x5fb5e0` stops the sounds,
releases marker, plate, bubble, name, component, trails, effects, missiles and models, clears the
active mover and its mode 12, cancels the two name queries, frees the chain array, destroys the
`CMovement` and unlinks the watcher lists. `0x5fb1d0` frees on bit 0. `0x5fefc0`, `0x5fefe0` and
`0x5ff000` read the running spline at `+0xa4c`: its points, `+0x20` and `+0x24` as floats. Slices: uh_low_hp_gate_difftest `0x5fb14b`-`0x5fb15c`.

## Lifecycle

`0x5fb880(time, a, spawned)` runs after creation: it resolves the display, gives the body its
callbacks, textures and sounds, sizes the mover, finds the race's sound race, sets up mount, death
time and loot sparkle, picks the first animation, and refreshes markers, tokens, name, breath,
auras and channel. Slot 1 tears a unit's client state down when it leaves; slot 2 rebuilds its
name and facings when it returns; slot 3 refreshes what an update may have changed.

Addresses: `0x5fb880`, `0x5fb93d`, `0x5fba5d`, `0x5fbb60`, `0x5fbd60`, `0x5fbe00`, `0x5fbe80`
Confidence: verified
Proof: one
Test: `objectlayer::diff_low_health_ratio`

At `0x5fb93d` a dead creature deeper than 0.75 of its collision height in liquid, with
`UNIT_FIELD_FLAGS` bit 3, 11 or 15, starts swimming. `+0xb18` takes `+0x34` of the `ChrRaces` row
(0xc0dee0). A dead unit stores `time` at `+0xcb4`; a mounted one mounts. At `0x5fba5d` a creature
made with `spawned` remembers stand state 9, so its first animation is Birth (127). The
manager's create path (0x465dd7) and the player's (0x5dec09) call it. `0x5fbb60` (slot 1) also
leaves the camera's transport, releases the mover, drops all token flags, ends the auto-move,
fades the sounds `+0xb1c` and `+0xb20` and hands the world node to `0x5fbe80`, which drops it at
once when the character component is not ready and otherwise places and fades it (0x672df0).
`0x5fbd60` (slot 2) resets both facings, rechecks low health and clears `+0xd58` bits 0x400 and
0x2000. `0x5fbe00` (slot 3) refreshes sounds, auras, channel, animation, display, marker, tokens
and breath, and gives a newly dead unit its death time and sparkle. Slices: diff_low_health_ratio `0x5fbd8f`-`0x5fbda6`.

## Start-up and shutdown

`0x6033c0` starts the unit code: 81 opcode handlers from 0xb5 to 0x325, the sound, spell, combat
and field watcher set-ups, the marker models, the talk emotes, the environmental damage kits,
nameplates off, auto-move mode 12 with no pending targets, and three CVars. `0x603280`
registers nine more CVars before it. `0x604f40` clears it all.

Addresses: `0x6033c0`, `0x6038fd`, `0x603280`, `0x604f40`, `0x605290`, `0x603e00`, `0x603e50`, `0x603ff0`
Confidence: verified
Proof: one
Test: none

The sub-inits are 0x623110, `Spell_C::SystemInitialize` 0x6e7150, 0x625520, `0x6041f0`,
0x6d8070, `0x603a00(1)` and 0x603a60; the tail is 0x61cbf0. At `0x6038fd` each
`EnvironmentalDamage` row stores its visual kit `+0x8` at `0xc4d8e4[type]` for types under 6. The
CVars are `showfootprintparticles` (`0xc4d948`), `automoveturnspeedwide` 1200 and
`automoveturnspeednarrow` 800. `0x603280` (called from 0x4015f8 before 0x401639) registers
`PetMeleeDamage`, `PetSpellDamage`, `CombatDamage`, `ChatBubbles` (all "1"),
`ChatBubblesParty`, `PlayerAnim` (`0xc4d974`), `TargetAnim` (`0xc4da30`) (all "0"), `AutoInteract` ("1" under locale
index 1, else "0") and `CombatLogPeriodicSpells` ("1"). `0x604f40` clears the 82 handlers, then
calls 0x6230c0, `Spell_C::Destroy`, 0x625bd0, `0x605290` (the watchers) and 0x6d8420. Handlers:
0x261 (`0x603e00`) replays the active player's stand state through `0x6127b0`; 0x29d
(`0x603e50`) passes its u8; 0x172 (`0x603ff0`) plays slot 18, MountSpecial, on a unit not deep
in liquid and not the active player.

## Field watchers

`0x6041f0` registers 25 watchers on unit fields; a field write the update applies calls them
with the old value. Each finds the unit and calls the code that owns the change.

Addresses: `0x6041f0`, `0x604cc0`, `0x6044b0`, `0x6044f0`, `0x604530`, `0x604570`, `0x6045b0`, `0x604600`, `0x6046f0`, `0x604860`, `0x604890`, `0x6048d0`, `0x604910`, `0x604950`, `0x604990`, `0x6049d0`, `0x604a20`, `0x604a60`, `0x604aa0`, `0x604b30`, `0x604bf0`, `0x604c30`
Confidence: verified
Proof: one
Test: none

| Field | Watcher | Calls |
|---|---|---|
| 37–39 virtual item display | `0x604cc0` | `0x605da0(slot)` |
| 46 unit flags | `0x6044b0` | `0x5ff580(old)` |
| 138 bytes 1: byte 3, byte 1, byte 0 | `0x6044f0`, `0x604530`, `0x6049d0` | `0x5ff800`, `0x5ff860`, `0x6127b0` |
| 133 mount display, 131 display | `0x604570`, `0x604990` | `0x5ffa50(old)`, `0x60abe0` |
| 34 level | `0x6045b0` | `0x60cdd0` when it differs |
| 22 health; 23–27 powers; 125 aura state | `0x6046f0`; `0x604860`; `0x604890` | below; usable events |
| 35 faction; 10–13; 18–19 | `0x6048d0`; `0x604910`; `0x604950` | `0x606e20` |
| 147 NPC flags; 148 emote state | `0x604a20`; `0x604a60` | `0x60b420`; `0x60b570` |
| 140 pet name stamp; 139 pet number | `0x604aa0`; `0x604bf0` | names; `UNIT_CLASSIFICATION_CHANGED`, `PET_UI_UPDATE` |
| 143 dynamic flags | `0x604b30` | `0x600440(old)` |
| object 3 entry; object 4 scale | `0x604600`; `0x604c30` | creature cache; `0x60b1d0(old)` |

The aura, channel and sheathe watchers are in their topics. `0x604860` and `0x604890` fire the
usable events for the active player, the second also for the target. `0x604910` also marks the
name dirty. `0x605290` removes the same 25.

## Health, death and resurrection

The health watcher redraws the name, keeps the low-health bit (under 20%), and flashes the
nameplate on a loss. A unit that dies stops the active mover's input, runs `0x605860` and clears
the player's precast on it; one that comes back resets input, plays its idle and rebuilds its
display. Dynamic flag 0x20 makes a unit look dead.

Addresses: `0x6046f0`, `0x604769`, `0x6047ee`, `0x605860`, `0x5fef70`, `0x6003e0`, `0x605f90`, `0x600440`, `0x6004a0`
Confidence: verified
Proof: one
Test: none

`0x605860` stops a player's cast and target (0x5e99b0), clears `+0xc48` to `+0xc54`, cancels the
pending cast with reason 0x65 when it targets the unit, clears the player's target and auto-repeat
when it is the target, releases the marker, fades, stops `+0xb1c` and the auto-move. `0x5fef70`
refreshes sounds, clears `+0xcb4`, resets the model speed (0x712910) and the animation, and for
the active player signals `PLAYER_ALIVE`. `0x6003e0` stops the active player's attack and clears
the target when it is this unit. `0x605f90` is no health, dynamic flag 0x20, or stand state 7.
`0x600440(old)` handles dynamic flags: losing 0x1 drops the sparkle, gaining it sets tutorial flag
6; at `0x6004a0` a change of 0x20 signals `UNIT_HEALTH` and the power event, and on gaining it
stops input, clears the target, refreshes sounds, plays death; a known death time with 0x1 adds the
sparkle; 0x4 signals `UNIT_FACTION`.

## Unit flags and bytes

`0x5ff580(old)` acts on the flags that changed: the active player's control bits, looting,
disarm, reaction, PvP, the pet's attack events, two more interface refreshes and `UNIT_AURA`.
`0x5ff800` follows byte 3 of `UNIT_FIELD_BYTES_1`, `0x5ff860` the pet loyalty in byte 1, and
`0x60cdd0` a level change.

Addresses: `0x5ff580`, `0x5ff5e5`, `0x5ff614`, `0x5ff711`, `0x5ff75e`, `0x5ff4e0`, `0x5ff800`, `0x5ff860`, `0x60cdd0`
Confidence: verified
Proof: one
Test: none

For the active player `0x5ff4e0` stops input on 0x40000, fires usable events on 0x1c20000,
signals `PLAYER_REGEN_DISABLED` or `ENABLED` on 0x80000 and ends an auto-move on gaining 0x100000.
At `0x5ff5e5` a looting change re-picks the animation or plays 189 after 188. At `0x5ff614` bit
21 (disarm) clears or re-hangs the hands (options 15, 16). At `0x5ff711` 0x4080 and 0x1000 refresh
reactions; 0x1000 on the active player sets tutorial flag 0x1f. At `0x5ff75e` 0x800 signals
`PET_ATTACK_START` or `STOP` when the charmer, else the creator, is the active player; 0x4000000
calls 0x4919d0; 0x8000000 signals `UNIT_AURA`. `0x5ff800`: bit 0x2 re-picks the animation, dirties
the name and fires usable events for the active player; bit 0x1 dirties the name. `0x5ff860`: a
rise plays effects 8 and 9, a fall 10 and 11; when the charmer, else the creator, is the active
player it logs through 0x62d440(1 or 0). `0x60cdd0` plays effect 5 on any level change, up or
down, calls 0x5edb60 for a player and refreshes the plate.

## The stunned flag

Bit 0x40000 of `UNIT_FIELD_FLAGS`, stunned, is acted on in thirteen places. The input code stops turning and pitching, a relayed turn or pitch start is
refused, and the unit cannot toggle its weapons, loot, cast, auto-move to an object or start an attack.

Addresses: `0x5145d2`, `0x5151fe`, `0x51529b`, `0x602b29`, `0x602b89`, `0x600dd7`, `0x5eb4f2`, `0x5ec219`, `0x6099bc`, `0x60ff95`, `0x612ef2`, `0x5ff4ef`, `0x4bd075`, `0x5145b0`
Confidence: verified
Proof: one
Test: none

The tests are `0x5145b0` (not stunned, `not`, `shr 0x12`, `and 1` at `0x5145d2`), the input
functions `0x5151b0` and `0x515250` (`0x5151fe`, `0x51529b`), the relayed turn and pitch starts
`0x602b20` and `0x602b80` (`0x602b29`, `0x602b89`), which return 0, the display-facing steer
`0x600cd0` (`0x600dd7`), the weapon toggle (`0x5eb4f2`), `CanLootNow` (`0x5ec219`), the cast
validator (`0x6099bc`), `0x60fed0` (`0x60ff95`, message 0x191), `0x612df0` (`0x612ef2`, message 0xa2), the
flags delta `0x5ff4e0` (`0x5ff4ef`), which re-runs the input tick, and the pet bar's predicate
`0x4bcf70` (`0x4bd075`). Of the twelve relayed `MSG_MOVE_*` wrappers `0x602a80` to `0x602c20`,
only the turn and pitch starts test it.

## Reactions, tokens and groups

`0x606e20` redraws a unit's name and signals `UNIT_FACTION` for it and the units it summons and
charms; for the active player it then does so for every unit not the player's own. `0x60c520`
keeps the unit's token flags `+0xc90`, which register it for unit events while nonzero.
`0x606c20` and `0x606d20` tell whether two player-controlled units share the active player's
party or raid.

Addresses: `0x606e20`, `0x606f20`, `0x60c520`, `0x60c4a0`, `0x60c4f0`, `0x606c20`, `0x606d20`, `0x6116e0`, `0x60be60`
Confidence: verified
Proof: one
Test: none

`0x606f20` skips the active player and units its charmer, else summoner, is the player; others
call `0x607380`. Token bits: 0x1 the active player, 0x2 its charm (or summon), 0x10 a party
member (0x4e7f70), 0x20 raid (0x4e7fc0), 0x100 0x4baee0, 0x200 0x4baf30, 0x1000 the target;
0x51bbb0 registers on the first bit, 0x51bdb0 unregisters at none (`0x60c4a0`, `0x60c4f0`).
`0x606c20` is true for the same unit, else needs `UNIT_FIELD_FLAGS` bit 3 on both and one
controlling player (0x606170) to be the active player and the other in its party (0x4918d0);
`0x606d20` asks 0x4918e0. `0x6116e0` is true for a player, or for the active player's charm or
summon with a pet number when the player's class (byte 1 of `UNIT_FIELD_BYTES_0`) is 3.
`0x60be60` (slot 21) refuses `UNIT_FIELD_FLAGS` bit 25 unless the active player created the unit.

## Stand states

`0x6127b0(state)` applies a stand state: the active player's own handling, weapons sheathed for
any state but standing or sitting in a chair, and the animation for the change: Submerge (201)
into 9, death into 7, Birth (127) from 9 to standing, else a re-pick.

Addresses: `0x6127b0`, `0x6127f3`, `0x612849`, `0x60be30`, `0x60be50`
Confidence: verified
Proof: one
Test: none

The active player first runs 0x5f0790. `0x6127f3` calls `0x611cf0(0, 1, 1)` when `+0xd40` is set
and slot 41 is neither 0 nor 2. The old state is `+0xc1c`; a state 7 clears the target first
(`0x6003e0`); the re-pick sets `+0xd58` bit 0x40. A followed camera resets. `0x60be30` sends a
player's own stand state through 0x5ed430; `0x60be50` (slot 41) is byte 0 of `UNIT_FIELD_BYTES_1`.

## Right click and loot

`0x60bea0` (slot 24) answers a right click by a live player: a pet order,
loot, skinning or a player corpse's spell, talk, attack or a greeting. Loot needs a lootable
corpse, a standing player and no mount; skinning compares the skill with the level.

Addresses: `0x60bea0`, `0x60bef6`, `0x60bfaa`, `0x60c01f`, `0x60c0ef`, `0x60c162`, `0x6003a0`, `0x6005d0`, `0x600640`, `0x600680`, `0x6126b0`, `0x612710`
Confidence: verified
Proof: one
Test: none

At `0x60bef6` the player's `+0x1c70` bit 0 sends the unit 0x5ee5a0 returns to attack (0x4bd630).
A corpse without dynamic flag 0x20 (`0x60bfaa`) loots through 0x5df2a0 when `0x6003a0` allows,
else message 0x85 `ERR_LOOT_NOTSTANDING`. At `0x60c01f` `UNIT_FIELD_FLAGS` bit 26 and the spell
0xb700e4[player] lead to 0x5f05e0: a player corpse the player cannot assist, or a creature
whose skinning value (effect 95) meets 1 to level 10, 10·level − 100 below 20, else 5·level
(`0x60c0ef`), else message 0xe0. At `0x60c162` 0x6067f0 allows talk (0x5f0130); else an attack
on a live target the player may attack, walking first for the mover (`0x60fcc0`), through
0x5ecb70; else `0x60c270`. `0x6003a0(time)` is no health, the death time passed, dynamic flag 0x1.
`0x6005d0` hangs the sparkle effect once; `0x600640` plays its animation; `0x600680` removes it.
`0x6126b0` is looting (the player's loot GUID, else `UNIT_FIELD_FLAGS` 0x400); `0x612710` refuses
the loot animation for flag bit 28 or, for the player, a fishing node, a live unit or an item.

## Selectability

A unit whose `UNIT_FIELD_FLAGS` has bit 25 cannot be moused over, targeted or given a name plate,
unless the active player created it. The mouse code asks the object's slot 21, `IsSelectable`; the
target code asks slot 22, which for units and players jumps to slot 21.

Addresses: `0x60be60`, `0x60be6f`, `0x5f1ec0`, `0x80afcc`, `0x80c54c`, `0x80afd0`, `0x80c550`, `0x482982`, `0x4935ee`, `0x4935f3`, `0x4938fd`, `0x606706`, `0x606835`, `0x6069fa`, `0x60f622`, `0x6066f0`, `0x6067f0`, `0x606980`, `0x60f600`
Confidence: verified
Proof: one
Test: none

`0x60be60` shifts the flags right by 25 (`0x60be6f`) and, with the bit set, compares
`UNIT_FIELD_CREATEDBY` (`+0x20`, `+0x24`) with the active player's GUID. The unit vtable
`0x80c4f8` and the player vtable `0x80af78` hold it at `+0x54` (`0x80c54c`, `0x80afcc`) and
`0x5f1ec0`, a jump through slot 21, at `+0x58` (`0x80c550`, `0x80afd0`). The hover handler calls
slot 21 at `0x482982`; `SetSelection` calls slot 22 at `0x4935ee` and, on a refusal, jumps to its
epilogue (`0x4938fd`) without storing the selection or sending a message. Five functions test the bit by
a shift of 25: `0x6066f0` (assist, `0x606706`), `0x6067f0` (interact, `0x606835`), `0x606980` (attack,
`0x6069fa`), `0x60be60` and the plate gate `0x60f600` (`0x60f622`).

## A refusal that arrives

When the server sets bit 25 on a unit that is already the target, the target stays: its ring, frame and target of target
remain, and the selection pair is not written. The mouseover goes on the next hover pass, the nameplate at the next update
of the unit and the overhead name when it is next asked, so the unit stays selected while it cannot be re-selected,
hovered, given a plate or named.

Addresses: `0x6042ec`, `0x6044b0`, `0x5ff580`, `0x4828d0`, `0x482982`, `0x60f600`, `0x607ef9`, `0x60706a`, `0x6070a0`, `0x6070a8`, `0x492890`, `0x7cb869`, `0x7cb8bf`
Confidence: verified
Proof: one
Test: none

The only watch on `UNIT_FIELD_FLAGS` for units is `0x6042ec` (offset 0xa0, handler `0x6044b0`), which calls `0x5ff580`; that
function holds no test of bit 25 and no call of the setter or the teardown. `0x60f600` has two callers, the unit update
`0x607ef9` and the reaction refresh `0x60706a`, so it runs every tick. `0x6070a0` has one caller, `0x6c6ee2`. The plate's own
enter and leave handlers publish the mouseover through `0x492890` (`0x7cb869`, `0x7cb8bf`) without the slot 21 test, so the
plate gate is what keeps the tooltip away. The census of `shr r32,0x19` is 12 instructions: the five readers above and seven that are
four MD5 constants and three reads of another record's flags; no byte test of `+0xa3` by 2 exists.

## Player-driven units

`0x5fa550` says whether a unit acts under its own control: a player that no one has charmed, or a
possessed unit whose possessor is such a unit.

Addresses: `0x5fa550`, `0x5fa566`, `0x5fa571`, `0x5fa582`, `0x5fa599`, `0x5fa5c7`, `0x5fa5e2`, `0x468460`
Confidence: verified
Proof: one
Test: none

With bit 1 of `UNIT_FIELD_FLAGS` clear and any of 0xc00004 set (`0x5fa566`: confused, fleeing and bit 4) it
returns 0. With bit 24 (possessed) set (`0x5fa582`) it takes the charmer GUID at `+0x10`, else the one at
`+0x20`, resolves it with mask 8 (`0x5fa599`), returns 0 for no unit and otherwise asks `0x5fa550` of
that unit. Without bit 24 (`0x5fa5c7`) the unit must be a player with no charmer GUID and bit 0 of
the flags clear; the result is then 1 (`0x5fa5e2`), else 0. Stunned is not among the bits it tests.
A creature possessed by a free player therefore returns 1, one possessed by a feared player 0, and
a creature the server lets the player drive without flag 24 returns 0.

## Melee attack

`0x612df0(guid)` checks a melee start and settles its target: no attack while dead, charmed,
stunned, pacified, fleeing, confused or mounted; an empty GUID takes the target, a friendly one is
replaced; the target must be attackable. `0x6131a0` toggles the player's attack or orders the
pet's.

Addresses: `0x612df0`, `0x612e6a`, `0x61305d`, `0x6131a0`, `0x60ecb0`, `0x60ecd0`
Confidence: verified
Proof: one
Test: none

Messages 0xa0 to 0xa9 are `ERR_NO_ATTACK_TARGET` to `ERR_ATTACK_PREVENTED_BY_MECHANIC_S`. Dead
gives 0xa8. Charmed by another (`0x612e6a`) names the aura's spell with 0xa9, or 0xa7. Flags
0x40000, 0x20000, 0x800000 and 0x400000 scan their auras and give 0xa9 with the spell, or 0xa2,
0xa3, 0xa5, 0xa6. Mounted gives 0xa4. At `0x61305d` a friendly target (reaction 4 or more) is
replaced through 0x493f60; none gives 0xa0. A dead attacker, a mount, a dead target without
dynamic flag 0x20, or one 0x606980 refuses gives 0xa1. `0x6131a0` then stops the player's attack
when one runs and the player has control (0x5ecac0), else starts it (0x5ecb70); the pet bar's
unit attacks through 0x4bd630. `0x60ecb0` tests `+0xc48`; `0x60ecd0` adds `UNIT_FIELD_FLAGS` 0x800.

## Caster checks for a spell

`0x6094f0` refuses a cast for the caster's state, in a fixed order: the record, cooldowns, power,
effects that need a free slot or a pet, life, crowd control, weapons, mount, stance, time of day,
water, movement, form, stealth, combat, aura state, reputation, indoors and outdoors, and taming.

Addresses: `0x6094f0`, `0x609517`, `0x609884`, `0x609cd6`, `0x609cdb`, `0x609ce2`, `0x609d06`, `0x609d12`, `0x609e50`, `0x609f63`, `0x60a01e`, `0x612480`, `0x60a050`, `0x612cc0`, `0x60d020`
Confidence: verified
Proof: one
Test: none

Results are `SPELL_FAILED` codes; 0x8d carries a second code. No record sets error 0x57
(`0x609517`). Item cooldown 0x28; spell cooldown 0x3c, cancelling auto-repeat for AttributesEx3
0x400000; power 0x4d (a negative type is health). A known learn-spell effect shows 0x32 or 0x33
(`0x609884`); a charm or summon with one present 0x4 or 0x5; a pet target without a pet 0x4c;
a living player with bit 0x4 of `PLAYER_FIELD_BYTES` byte 0 (`+0x1028`) 0x61; charmed, stunned, silenced (prevention 1, flag 0x2000), pacified (2, 0x20000), fleeing,
confused give 0x8d with 0x14, 0x64, 0x60, 0x5a, 0x1e, 0x16. Interrupt flag 0x200 needs drawn
weapons (0x41), 0x40 a mount (0x53); Attributes 0x1000000 clear refuses a mount (0x39), 0x8000000
clear a seated caster (0x3e); 0x1000 needs day (`0x60a050` is nonzero from 5:30 to 21:00; 0x51 otherwise) and
0x2000 night (0x54; the name switch `0x6e23e0` keys them at `0x6e25dc` and `0x6e25ee`); 0x100 water (0x58), 0x80 none (0x50); interrupt flag 1 refuses a moving
caster (0x2e); `0x612480` checks forms (0x3d, 0x56); 0x20000 needs stealth (0x57); 0x10000000
refuses combat; caster aura state 0x12; reputation 0x8f; `0x612cc0` tests outdoors for 0x4000
(0x52) and 0x8000 (mount aura 0x4b, else 0x55); taming 0x6e6a20(2), 0x17. `0x60d020` sets the
precast spell `+0xc8c` and bit 0x400.

## Creature records

A creature's `CreatureCache` record at `+0xb30` gives its subname `+0x10`, type flags `+0x14`,
type `+0x18`, family `+0x1c`, rank `+0x20`, the `+0x24` its pet info reads, civilian `+0x30` and
racial leader `+0x31`. `0x605570` gives a unit's creature type: its form's, else its record's,
else its race's.

Addresses: `0x605570`, `0x6055e0`, `0x605600`, `0x605620`, `0x612550`, `0x6125c0`, `0x6125f0`, `0x612610`, `0x613230`, `0x612530`
Confidence: verified
Proof: one
Test: none

`0x605570` reads the form row's `+0x30` (0xc0d774), then `+0x18`, then the `ChrRaces` row's
`+0x24` (0xc0dee0), each when above 0. `0x6055e0` is the family, `0x605600` `+0x24`, `0x605620`
the rank without a pet number. `0x612550` is a civilian: PvP flag 0x1000, the civilian byte, a
reaction of at most 1 toward the active player and 0x5f0700. `0x6125c0` is a racial leader: 0x1000
and `+0x31`. Type flags: `0x6125f0` bit 0x8 (no wound or parry animation), `0x612610` not 0x10
(faction in the tooltip, 1 without a record), `0x613230` 0x80 (interact while dead), `0x612530`
0x4, which nothing reaches.

## Known spells, skills and prices

`0x60c740` tells whether a unit knows a spell: the active player by its known-spell bits, the pet
bar's unit by its spell list. `0x60c7c0` finds a spell's skill line ability for a creature family
or a race and class; `0x60c840` raises a spell's level to its skill's, `0x60c8d0` finds a known
higher rank. `0x612b80` gives a vendor's discount.

Addresses: `0x60c740`, `0x60c7c0`, `0x60c840`, `0x60c8d0`, `0x60c920`, `0x612b40`, `0x612b80`
Confidence: verified
Proof: one
Test: `objectlayer::vendor_price_discount_difftest`

`0x60c740` tests the bit at 0xb710fc up to the highest id, or the 1024 entries at 0xb6f098.
`0x60c7c0` takes the `CreatureFamily` row's two skill lines (`+0x14`, `+0x18`) for a creature,
else race and class from `UNIT_FIELD_BYTES_0`. `0x60c840(spell, out)` raises `+0x74` to the
`SkillRaceClassInfo` row's `+0x14`. `0x60c8d0` walks the ability's `+0x20` chain; `0x60c920`
returns its `+0x4`. `0x612b40` asks 0x5edae0 for a player, else zeroes both outputs.
`0x612b80(player)` is 0.1 at reaction 5 or more, plus 0.05 for a PvP vendor when the player's
rank byte is 6 or more and 0.05 more from 8.

## Unreached unit code

Nothing live reaches these ten functions; a whole-image scan for calls, jumps and stored
addresses finds no site, or only a site in another of them.

Addresses: `0x5fccd0`, `0x603220`, `0x603250`, `0x606940`, `0x60b580`, `0x60bba0`, `0x60d600`, `0x611670`, `0x6116a0`, `0x612530`
Confidence: verified
Proof: one
Test: none

`0x5fccd0` tests `AnimationData` flag 0x100 of the current animation. `0x603220(src, n)` copies n
bytes into the unit fields. `0x603250` maps 0 to 0, 1 and 3 to 0x18, 2 to 0xc0 and others to −1
through 0x603270. `0x606940` refuses a ghost player, else needs a game object's reaction of 3 or
more. `0x60b580` re-picks the animation. `0x60bba0` tests set D (`0x5fced0`). `0x60d600` tests the
spell cast animations 2, 32, 33, 53 and 54 through 0x60d624 and 0x60d62c. `0x611670` returns the
creator when the unit has no charmer or summoner, and its only caller `0x6116a0` tests it against
the active player and the party. `0x612530` tests type flag 0x4.
