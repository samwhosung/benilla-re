# objectlayer: interaction

How the player starts things with the world: right-clicking a unit and the ladder of NPC services,
the confirmation dialogs and their range checks, looting, interacting through a spell, world clicks,
taxis, the bind point, stand states, text emotes, AFK, far sight, tracking, and the predicates the
interface asks about other units. The distances here are squared and compared in x87; where a
step rounds to f32 it is said.

Roots: `0x5f0130`, `0x5f05e0`, `0x5df2a0`, `0x5df130`, `0x5e0320`, `0x5ef560`, `0x5ee290`

## Talking to an NPC

`0x5f0130(unit, walk)` refuses with `ERR_CANT_INTERACT_SHAPESHIFTED` while the player is in a form
whose `SpellShapeshiftForm` row lacks flag 0x8, or has none. Beyond 5.5555553 yards it returns,
starting the walk up first when `walk` is set; a NaN distance passes. The unit whose GUID is at
`0xb4e2d0` does nothing, a quest giver of status 1 raises tutorial 0x2a, and then the first NPC
flag that matches picks the service. After any send the player plays the talk emote.

| flag | service |
|---|---|
| 0x1 | 0x17b gossip |
| 0x2, with status other than 0 and 1 | 0x184 quest giver |
| 0x4 | the vendor: sells the cursor's item (0x1a0), else lists (0x19e) |
| 0x8, 0x10 | 0x1ac taxi; 0x1b0 trainer, remembered in `0xc4d788` |
| 0x20, 0x40, 0x80 | the spirit healer dialog; the spirit guide (`0x4921c0`); the binder dialog |
| 0x100, 0x200, 0x400 | 0x1b7 banker; 0x1bb petitions; 0x1f2 tabard, refused while shapeshifted |
| 0x800, 0x1000, 0x2000 | 0x2d7 battlemaster; 0x255 auction; 0x26f stable |

Addresses: `0x5f0130`, `0xc4c28c`, `0xb4e2d0`, `0x5df490`, `0x5df4d0`, `0x5df550`, `0x5df5d0`, `0x5df680`, `0x5dffe0`, `0x5e0060`, `0x5e00e0`, `0x5e01a0`, `0x5e0220`, `0x5e02a0`, `0x5ed020`, `0x60bb30`, `0x5df950`
Confidence: verified
Proof: one
Test: none

Each component of player minus unit position is stored as f32 before the x87 sum; the limit
`0xc4c28c` holds 5.5555553² = 30.864195 (`InitGameObjSquare3`). `StartAutoMoveToObject(&guid, 5)`
nonzero returns; a larger distance after that returns. The flags are `UNIT_NPC_FLAGS`
(`[+0x110]+0x234`); the quest test `0x5df490` reads the status `+0xcb8`. The tabard sender
`0x5e00e0` does nothing for the unit at `0xbdcee8` and shows error 0x130 while the display is not
native. The talk emote is `CGUnit_C::RequestTalkEmote(0)` (`0x60bb30`).

## Confirmation dialogs

The spirit healer, talent wipe, pet unlearn and binder each run twice: with a GUID they remember
it and signal the confirmation; with 0 they act on the remembered one, if its unit is still in
range. The spirit healer needs a ghost and shows `CONFIRM_XP_LOSS` with 5% of the next level's
experience; the talent and pet dialogs remember the cost, show it, and on accept refuse with
`ERR_NOT_ENOUGH_MONEY` when it exceeds the money; the binder shows the inn's area name or
`HOME_INN`. The server's 0x222, 0x2aa, 0x2f1 and 0x2eb open them.

| dialog | GUID | event | accept |
|---|---|---|---|
| `0x5df730` | `0xc4d798` | `CONFIRM_XP_LOSS` | 0x21c |
| `0x5df980` | `0xc4d7a0`, cost `0xc4d7a8` | `CONFIRM_TALENT_WIPE` | 0x2aa |
| `0x5dfba0` | `0xc4d7b0`, cost `0xc4d7b8` | `CONFIRM_PET_UNLEARN` | 0x2f0 |
| `0x5dfdc0` | `0xc4d7c0` | `CONFIRM_BINDER` | 0x1b5 |

Addresses: `0x5df730`, `0x5df980`, `0x5dfba0`, `0x5dfdc0`, `0x5df8c0`, `0x5dfb10`, `0x5dfd30`, `0x5dff50`, `0xc4d798`, `0xc4d7a0`, `0xc4d7b0`, `0xc4d7c0`, `0x80ae90`
Confidence: verified
Proof: one
Test: none

The experience is `__ftol(PLAYER_NEXT_LEVEL_XP · 0.05)` (`0x80ae90`), 0 for a player not active.
The accept paths test range inline in full x87 against `0xc4c28c`, where NaN passes; the range
tests `0x5df8c0`, `0x5dfb10`, `0x5dfd30` and `0x5dff50` that the interface calls use
`test ah,0x41; jp`, where NaN fails. The money is `PLAYER_FIELD_COINAGE`, compared unsigned. The
binder's area is `[0xb4e318]`, else `[0xb4e314]`, in the `AreaTable` store.

## Looting

`0x5ec110(object)` allows a loot when no loot is held, the player is alive with mover flag 0x2000
and unit flag 0x40000 clear, and in control; when the object cannot be reached by walking, it must
also be within the reach (for a unit, both combat reaches plus 4/3, at least 5; else 5).
`0x5df2a0(unit, walk)` loots a unit: lootable, the mover still, it walks up beyond reach, then sends
0x15d, holds the GUID and plays animation 50. `0x5df130` does the same for a corpse, walking up
beyond 5 yards (25 squared).

Addresses: `0x5ec110`, `0x5df2a0`, `0x5df130`, `0x5df460`, `0x80b058`, `0x80a1e8`, `0x80b054`
Confidence: verified
Proof: one
Test: none

`0x5ec110` takes `!CanAutoInteract`; the reach is stored f32, squared, and compared with the full
x87 distance by `test ah,0x41; jnp`, so equal passes and NaN fails. It then needs `+0x1d28` zero,
not `IsDeadOrGhost`, mover flags without 0x2000, `UNIT_FIELD_FLAGS` without 0x40000, and
`IsClientControlled`. `0x5df2a0` needs `IsLootable` and mover flags `& 0x20ff` zero; it walks with
mode 6 only when `r² < d²`. `0x5df460(byte)` sets the auto-loot byte `+0x1d30` and, with a held
GUID, calls `0x4c1fa0(0)`. `0x5ed5f0` holds a GUID as the loot target and plays animation 0x32.

## Interacting through a spell

`0x5f05e0(object, walk, set, value)` interacts with a corpse or player through the spell at
`0xb700e4 + 4` and with anything else through the one at `0xb700e4`: in range of the spell's
maximum it casts at once; out of range it walks up when `walk` is set, else casts anyway. With
`set` it then sets the auto-loot byte to `value`.

Addresses: `0x5f05e0`, `0xb700e4`
Confidence: verified
Proof: one
Test: none

The range is `Spell_C::GetMinMaxRange`; the components of object minus player go through
`SetVector3` as f32 and are squared in x87; `max² > d²` casts (`CastSpellAsActivePlayer`). The walk
is `StartAutoMoveToObject(&guid, 9)`.

## World clicks

`0x5e0320(record)` handles a click in the world: on terrain (kind 4) the active mover walks to the
point, moved into the transport's frame; on an object it calls `0x495300`; otherwise it clears
the target when a target is shown.

Addresses: `0x5e0320`
Confidence: verified
Proof: one
Test: none

The point goes through `CMovement::WorldToTransport` and `WalkToPoint`. Objects come from
`0x494c80`; the clear is `0x493540(0, 0)` while `[0xb4e2b8]` and its `+0x28` are set.

## Taxis

0x1a9 reads a mode; with mode set, a GUID, the current node and four 64-bit masks of known nodes.
Fewer than two known nodes show `ERR_TAXINOPATHS` and close the map; else the map opens. Mode 0
prints each known node's id and name to the console. 0x1ab marks a flight master with 4 when its
node is unknown and 0 when known. 0x1ae shows the activation result: 0 starts the flight, 1 to 12
show errors, 13 and above nothing.

Addresses: `0x5ece60`, `0x5ecdd0`, `0x5ed1e0`, `0x85fedc`, `0x5ed020`, `0x5ed0a0`, `0x5ed130`, `0x5eb170`
Confidence: verified
Proof: one
Test: none

Node `64·k + b + 1` is bit b of mask k, named from `TaxiNodes` (`0xc0d6b8`) as "[%02d]: %s". The
map is `0x4db710`, the close `0x4e1fa0`, the flight `0x4dc0f0`; the marker is
`CGUnit_C::SetQuestMarker`. The senders: `0x5ed020` 0x1ac (GUID), `0x5ed0a0` 0x1ad (GUID, two u32),
`0x5ed130` 0x312 (GUID, a u32, the node count, then a u32 per node); `0x5eb170` sends 0x1aa with
a unit's GUID for `CGUnit_C::RefreshQuestGiverStatus`.

## The bind point

0x155 stores the bind point's x, y and z, its map and its zone and marks it valid. `0x5ed420`
returns the zone. `0x5ed330`, which nothing calls, tests a point within 15 yards on the same map.

Addresses: `0x5ed3c0`, `0x5ed420`, `0x5ed330`, `0x5ed410`, `0xc4c274`, `0xc4d6a8`, `0xc4c25c`, `0xc4d4e0`
Confidence: verified
Proof: one
Test: none

The floats go to `0xc4c274` to `0xc4c27c`, the map to `0xc4d6a8`, the zone to `0xc4c25c`, and
`0xc4d4e0` to 1. The radius is `10 · 1.5` from `0x80ae54` and `0x80308c`.

## Stand state

`0x5ed430(state)` sends 0x101 for the active player unless it is channelling or busy, mounted,
under `UNIT_FIELD_FLAGS` 0x100000, dead or a cast is in progress; any state but 0 needs the mover
without flags 0x20000f, and state 3 also without 0x30. After the send the unit runs
`CGUnit_C::OnStandStateChanged`, which keeps the local state `+0x1d68` through `0x5f0790`: state 0
calls `0x514640` on the input control, another state ends looting and the melee attack.

Addresses: `0x5ed430`, `0x5ed570`, `0x5f0790`, `0x6127b0`
Confidence: verified
Proof: one
Test: none

Busy is `CHANNEL_SPELL` (`[+0x110]+0x228`) or `+0xd58` bit 0x400; dead is `HEALTH` 0 or
`UNIT_DYNAMIC_FLAGS` 0x20. State 3 tests `[[+0x118]+0x40] & 0x30`, any nonzero state `& 0x20000f`.
`0x5f0790` calls `0x514640` on state 0, else `0x48f200(1,1,0)` while looting and `0x5ecac0` while
attacking.

## Text emotes

`0x5ef560(text, guid)` performs a text emote unless `UNIT_FIELD_FLAGS` has 0x1000000. The `Emotes`
row it names must pass `0x47db40`; one marked for standing still is refused while moving with
`ERR_NOEMOTEWHILERUNNING`; a sleeping player only takes stand-state emotes. A stand-state emote of a
state in 0x10b changes the stand state; another plays its animation while the player moves plainly.
It sends 0x104 with the text id, a number from the race and gender, and the target, 0 for the player
itself. 0x103 plays an emote on a unit that is not asleep or under mover flag 0x200000.

Addresses: `0x5ef560`, `0x5e66b0`, `0x85ff10`, `0xc0ddd4`
Confidence: verified
Proof: one
Test: none

The text row's `+8` indexes the `Emotes` store `0xc0ddd4` (loader `0x544290`); the emote's flags
`+0xc`, kind `+0x10`, state `+0x14` and animation `+8`. The number is `0x4592b0(id, race, gender)`
with the display race and gender while shapeshifted. The sheathe call is `SetSheatheState`; the
mask at `0x85ff10` is 0x10b.

## AFK

`0x5eb740(message)` marks the player AFK: an empty message takes `DEFAULT_AFK_MESSAGE`; the first
time it prints `MARKED_AFK_MESSAGE` and sets `0xb6e5cc`; every call sends 0x95 with type 0x14,
language 0 and the message. `0x5eb830(force)` clears it when set and forced or `autoClearAFK` is
on: it prints `CLEARED_AFK` and sends an empty message.

Addresses: `0x5eb740`, `0x5eb830`, `0xb6e5cc`, `0xc4d68c`
Confidence: verified
Proof: one
Test: none

The message is formatted into a 0x800-byte stack buffer and printed through `0x49a870` as type
0xa. The CVar is `[0xc4d68c]+0x28`.

## Far sight

`0x5ee290` follows `PLAYER_FARSIGHT`. With no far-sight object, or with the camera already on it,
it ends far sight: sends 0x27a with 0 if it is on, stops spell targeting and makes the player the
active mover (none when not client-controlled). Otherwise it sends 0x27a with 1 once, stops the
melee attack, and makes the object the active mover when it is a unit the client controls, else
calls `0x514640` on the input control. A set GUID with no object
returns 0 after restoring control.

Addresses: `0x5ee290`, `0x5ee270`, `0x5ee590`, `0x5ee5a0`
Confidence: verified
Proof: one
Test: none

The on bit is `+0x1c70` bit 0; `0x5ee270` runs the update when it is clear, `0x5ee590` when set.
Control goes through `SetActiveMover`; the camera's GUID is its `+0x88`. `0x5ee5a0` returns the
far-sight unit while it has `UNIT_FIELD_FLAGS` 0x1000000 and its charmer, or creator when none, is
the player.

## Tracking

`0x5ed210(unit)` is true for a stealthed unit while the player tracks stealth, for a unit with
dynamic flag 0x2, or for a creature type set in `PLAYER_TRACK_CREATURES`. `0x5ed2b0(object)` is true
for a game object whose lock has a skill entry (type 2) set in `PLAYER_TRACK_RESOURCES`.

Addresses: `0x5ed210`, `0x5ed2b0`
Confidence: verified
Proof: one
Test: none

Stealth is `UNIT_FIELD_BYTES_1` byte 3 bit 0x2 with `PLAYER_FIELD_BYTES` byte 0 bit 0x2; the masks
are fields 1104 and 1105, bit `type − 1`, 0 for a player not active. The lock is
`0x5f8180(object)`, eight entries of type `+4` and index `+0x24`.

## Other units

| function | returns |
|---|---|
| `0x5f0700` | a unit is trivial: not a player, below the player's level by more than 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11, then 12, by `level / 5` |
| `0x5efea0` | the relation of a GUID's unit, or a game object's creator, to the player, 0 to 9 |
| `0x5efe00` | the player's faction side: 0 Horde, 1 Alliance, −1 neither |
| `0x5efe60` | the gendered `PVP_RANK_%d_%d` text of a rank for that side |
| `0x5efcd0` | a stat's base, effective, positive and negative values |
| `0x5ec4c0`, `0x5ec890` | a race's male or female display id, which `0x5ec480` resolves to a model record; the race's `ChrRaces` row `+0x20` |
| `0x5ed5b0` | the gender: `PLAYER_BYTES_3` byte 0 for the active player, else `UNIT_FIELD_BYTES_0` byte 2 |
| `0x5ed630` | the first carried item with an on-use spell of effect 0x5e, for `Script::UseSoulstone` |
| `0x5decc0` | sends 0x114 with a nonzero GUID and calls `0x4c6f70` |

Addresses: `0x5f0700`, `0x80ae98`, `0x5efea0`, `0x5efe00`, `0x5efe60`, `0x5efcd0`, `0x5ec4c0`, `0x5ec890`, `0x5ed5b0`, `0x5ed630`, `0x5ed650`, `0x5decc0`, `0x5ec480`
Confidence: verified
Proof: one
Test: none

`0x5efea0`: self 0 (1 for its game object); another player 6 when both can attack, 2 a party
member, 4 else; a unit whose charmer or creator is the player 1, a party member 3, another
player 7 or 5; no owner 8; nothing 9. The side tests `FactionTemplate` `+0xc` bits 0x4 then 0x2.
`0x5efcd0` takes base as effective less `PLAYER_FIELD_POSSTAT0` (`0x5efd40`) and `NEGSTAT0`
(`0x5efda0`) and clamps a negative effective to 0. `0x5ed650` tests the item record's five spells
(`+0x11c`) for trigger 0 (`+0x130`) and an effect 0x5e among the three.
