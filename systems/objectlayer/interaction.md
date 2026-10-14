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
Test: `objectlayer::p3_dist2_f32deltas_5f0130`, `objectlayer::p3_in_range_gate_5f0130`

Each component of player minus unit position is stored as f32 before the x87 sum; the limit
`0xc4c28c` holds 5.5555553² = 30.864195 (`InitPlayerSquare4`, a Player_C.cpp initializer). `StartAutoMoveToObject(&guid, 5)`
nonzero returns; a larger distance after that returns. The flags are `UNIT_NPC_FLAGS`
(`[+0x110]+0x234`); the quest test `0x5df490` reads the status `+0xcb8`. The tabard sender
`0x5e00e0` does nothing for the unit at `0xbdcee8` and shows error 0x130 while the display is not
native. The talk emote is `CGUnit_C::RequestTalkEmote(0)` (`0x60bb30`). Slices: p3_dist2_f32deltas_5f0130 `0x5f01b9`-`0x5f0200`; p3_in_range_gate_5f0130 `0x5f0202`-`0x5f020d` `0x5f0238`-`0x5f0243`.

## The vendor click

The vendor arm `0x5df5d0(&guid)` sends one of two messages, chosen by the cursor. With an item held on the cursor
(payload mode 1 at `0xb4d900`) it sends `CMSG_SELL_ITEM` (0x1a0) with the clicked unit as vendor, the held item
and a count of 0, the whole stack, then clears the cursor. Otherwise it sends `CMSG_LIST_INVENTORY` (0x19e) with
the unit's GUID. No merchant window is read, opened or closed on either path, and the talk emote plays after both.

Addresses: `0x5df5d0`, `0x5df5d7`, `0x5df5e0`, `0x5df5ee`, `0x5df5f0`, `0x5df600`, `0x5df60e`, `0x5df64d`, `0x494c60`, `0x5e1d50`, `0x494b60`, `0x494b87`, `0x494b8d`, `0x494ba3`, `0x494bb6`, `0x495190`, `0x4951c2`, `0x4951c4`, `0x4951bb`, `0xb4d900`, `0xb4b3e4`, `0x4fb7a7`, `0x4fa378`
Confidence: verified
Proof: one
Test: none

`0x494c60` returns the held item's GUID for mode 1 and 0 for every other payload, and `0x5df5e0` branches on it, never on
the argument. The sender `0x5e1d50` writes the opcode, the vendor GUID, the item GUID and one count byte; its
other two callers, `0x4fb7a7` and `0x4fa378`, pass the open merchant as the vendor, so here the vendor is the clicked
unit instead. `0x5df5ee` zeroes the count. The send is unconditional; the clear that follows goes through `0x494b60`,
which does nothing without the active player or when player control is lost (`0xb4b3e4` 0) with flag bit 20 clear
(`0x494ba3`). Its `ClearCursor(0, 1)` passes 0 as the first argument, so `0x4951c2` skips the unlock `0x495420` and
the sold item stays locked until the server answers. `0x5f0320` calls the talk emote after the call on both paths.

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
Test: `objectlayer::range_gate_5df8c0_difftest`, `objectlayer::range_gate_clone_family_difftest`, `objectlayer::range_gate_5df730_difftest`, `objectlayer::skill_rank_scale_5df730_difftest`, `objectlayer::range_gate_5df980_clone_family_difftest`

The experience is `__ftol(PLAYER_NEXT_LEVEL_XP · 0.05)` (`0x80ae90`), 0 for a player not active.
The accept paths test range inline in full x87 against `0xc4c28c`, where NaN passes; the range
tests `0x5df8c0`, `0x5dfb10`, `0x5dfd30` and `0x5dff50` that the interface calls use
`test ah,0x41; jp`, where NaN fails. The money is `PLAYER_FIELD_COINAGE`, compared unsigned. The
binder's area is `[0xb4e318]`, else `[0xb4e314]`, in the `AreaTable` store. Slices: range_gate_5df730_difftest `0x5df7a1`-`0x5df7d2`; skill_rank_scale_5df730_difftest `0x5df81e`-`0x5df82c`; range_gate_5df980_clone_family_difftest `0x5df7a1`-`0x5df7d2` `0x5df9dc`-`0x5dfa0d` `0x5dfbfc`-`0x5dfc2d`.

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
Test: `objectlayer::loot_range_5df130_difftest`, `objectlayer::interact_reach_5df2a0_difftest`, `objectlayer::p3_interact_reach_sq_5ec110`, `objectlayer::p3_interact_in_range_5ec110`

`0x5ec110` takes `!CanAutoInteract`; the reach is stored f32, squared, and compared with the full
x87 distance by `test ah,0x41; jnp`, so equal passes and NaN fails. It then needs `+0x1d28` zero,
not `IsDeadOrGhost`, mover flags without 0x2000, `UNIT_FIELD_FLAGS` without 0x40000, and
`IsClientControlled`. `0x5df2a0` needs `IsLootable` and mover flags `& 0x20ff` zero; it walks with
mode 6 only when `r² < d²`. `0x5df460(byte)` sets the auto-loot byte `+0x1d30` and, with a held
GUID, calls `0x4c1fa0(0)`. `0x5ed5f0` holds a GUID as the loot target and plays animation 0x32. Slices: loot_range_5df130_difftest `0x5df1a0`-`0x5df1d4`; interact_reach_5df2a0_difftest `0x5df301`-`0x5df389`; p3_interact_reach_sq_5ec110 `0x5ec142`-`0x5ec17e`; p3_interact_in_range_5ec110 `0x5ec196`-`0x5ec1c1`.

## The loot target

The active player's loot target is a GUID at `+0x1d28`. Five sites set it: `LootUnit`
(`0x5df253`), `LootCorpse` (`0x5df40d`), a loot response that accepts its source (`0x5ebb60`),
`SetLootTarget` (`0x5ed5fb`) and `OpenItem` (`0x5edcc0`). Six clear it: the player constructor
(`0x5dd403`), the loot window's close (`0x48f2c9`), the player notice that names the target
(`0x5e3a84`), two legs of the loot response (`0x5eb9d2`, `0x5ebad4`) and the loot release
response (`0x5ec0d4`). The base animation kneels the active player while it is set; another unit
kneels on unit flag 0x400.

Addresses: `0x5df253`, `0x5df40d`, `0x5ebb60`, `0x5ed5fb`, `0x5edcc0`, `0x5dd403`, `0x48f2c9`, `0x5e3a84`, `0x5eb9d2`, `0x5ebad4`, `0x5ec0d4`, `0x6126b0`, `0x612710`, `0x5fd260`, `0x5fd266`, `0x5fd270`
Confidence: verified
Proof: one
Test: none

Each site stores both dwords. `0x5eb9d2` stores the zero in `esi` and `0x5ebad4` the zero of `xor eax,eax`
(after error 0x83). `0x5fd260` declines the kneel (anim 50) with a mount model (`0x5fd266`) or bit
0x40 of `+0xd58` clear (`0x5fd270`), then needs `0x6126b0` (`IsLooting`: the latch for the active
player, flag 0x400 for others) and `0x612710` (`ShowsLootAnimation`).

## Clicking while mounted

A mounted player's right click does the following. A lootable corpse is not looted and the click
falls to the skinning test, so a corpse that cannot be skinned does nothing and shows nothing.
Skinning, a player corpse's insignia and a lock opened by a spell cast, and the cast validator
refuses the mounted caster with failure 0x39. A live NPC with services opens its dialog. A
hostile creature is not attacked and shows no error. A pet order does nothing when the pet is
mounted.

Addresses: `0x60bea0`, `0x60bf36`, `0x60bf98`, `0x60c01f`, `0x60c1b4`, `0x60c257`, `0x60c25f`, `0x60c270`, `0x5d6c22`, `0x6094f0`, `0x609c6f`, `0x609c8c`, `0x612df0`, `0x4bd40d`, `0x6131aa`, `0x6e4efb`, `0x5ecb70`
Confidence: verified
Proof: one
Test: none

`0x60bea0` reads the mount display (field 133, `+0x1fc`) three times: `0x60bf36` on the pet the
player controls, `0x60bf98` on the player for the loot leg, whose `jg` goes to the skin leg
(`0x60c01f`), and `0x60c1b4` on the player for the attack leg. A mount ends the pet leg at
`0x60c25f` and the attack leg at `0x60c257`; the latter calls `0x60c270`, which speaks a greeting
only when the unit's reaction toward the player and the player's toward it are both 3 or more. The
attack leg shows no error: the refusal ladder `0x612df0` is called from `0x4bd40d`, `0x6131aa` and
`0x6e4efb`, never from a click. `0x6094f0` fails a mounted caster at `0x609c8c` unless the spell's
attribute word `+0x18` holds bit 0x1000000 (`0x609c6f`).

## Mounted cursor and selection

The hover cursor knows about the mount only for the attack cursor and a controlled pet's, so a
mounted player sees the loot cursor over a corpse the click then ignores. Selecting a target
sends `CMSG_SET_SELECTION` mounted or not. The client never ends a mount itself.

Addresses: `0x482200`, `0x482278`, `0x482659`, `0x493540`, `0x49367d`, `0x493748`, `0x493857`, `0x6e7040`, `0x4e49a0`
Confidence: verified
Proof: one
Test: none

The hover classifier `0x482200` reads `+0x1fc` twice: `0x482278` on the controlled pet and
`0x482659` on the player for the attack cursor; the loot, skin, corpse and object branches read
nothing of it. `0x493540`, the target setter, reads it at `0x49367d` and `0x493748` only in tests
whose pass records attackability and the enemy GUID at `0xb4e2e8`; the `0x13d` send at `0x493857`
does not test it. The aura cancel `0x6e7040`, which a dismount is, has nine call sites, among them
`CancelPlayerBuff`'s (`0x4e49a0`), and none is on a click path. No `0x375` immediate exists in the
image.

## Interacting through a spell

`0x5f05e0(object, walk, set, value)` interacts with a corpse or player through the spell at
`0xb700e4 + 4` and with anything else through the one at `0xb700e4`: in range of the spell's
maximum it casts at once; out of range it walks up when `walk` is set, else casts anyway. With
`set` it then sets the auto-loot byte to `value`.

Addresses: `0x5f05e0`, `0xb700e4`
Confidence: verified
Proof: one
Test: `objectlayer::p3_attack_range_gate_5f05e0`

The range is `Spell_C::GetMinMaxRange`; the components of object minus player go through
`SetVector3` as f32 and are squared in x87; `max² > d²` casts (`CastSpellAsActivePlayer`). The walk
is `StartAutoMoveToObject(&guid, 9)`. Slices: p3_attack_range_gate_5f05e0 `0x5f0656`-`0x5f069c`.

## World clicks

`0x5e0320(record)` handles a click in the world: on terrain (kind 4) the active mover walks to the
point, moved into the transport's frame; on an object it calls `0x495300`; otherwise it clears
the target when a target is shown.

Addresses: `0x5e0320`
Confidence: verified
Proof: one
Test: none

The point goes through `CMovement::WorldToTransport` and `WalkToPoint`. Objects come from
`0x494c80`; the clear is `0x493540(0, 0)` only when the CVar `deselectOnClick` is nonzero (its record is `[0xb4e2b8]`, the value at `+0x28`).

## Taxis

0x1a9 reads a mode; with mode set, a GUID, the current node and four 64-bit masks of known nodes.
Fewer than two known nodes show `ERR_TAXINOPATHS` and clear the gossip target; else the map opens (`ui/taxi`). Mode 0
prints each known node's id and name to the console. 0x1ab marks a flight master with 4 when its
node is unknown and 0 when known. 0x1ae shows the activation result: 0 closes the map, 1 to 12
show errors, 13 and above nothing (`ui/taxiroute#activation-replies`).

Addresses: `0x5ece60`, `0x5ecdd0`, `0x5ed1e0`, `0x85fedc`, `0x5ed020`, `0x5ed0a0`, `0x5ed130`, `0x5eb170`
Confidence: verified
Proof: one
Test: none

Node `64·k + b + 1` is bit b of mask k, named from `TaxiNodes` (`0xc0d6b8`) as "[%02d]: %s". The
map is `0x4db710`, the gossip clear `0x4e1fa0`, the map close `0x4dc0f0`; the marker is
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

## Emote eligibility

`0x47db40(flags, mode, out)` decides whether an `Emotes` row, by its flags `+0xc`, may be performed
by the unit. It refuses, in order: with `mode` set and flag 0x400; flag 0x1 unless the stand state
is 0; flag 0x80 while the mover swims (0x200000); a sleeping or dead stand state (3 or 7) unless
flag 0x200 allows it. Flag 0x4000 with any of the movement bits 0x20ff sets `out` to 1, the
mark that the emote needs standing still. Anything else is eligible.

Addresses: `0x47db40`, `0x47db56`, `0x47db58`, `0x47db65`, `0x47db6c`, `0x47db76`, `0x47db7d`, `0x47db8e`, `0x47db95`, `0x47dbab`, `0x47dbb3`, `0x47dbbc`, `0x5ef5c3`, `0x5ef5ca`
Confidence: verified
Proof: one
Test: none

`0x47db58` tests `dh` for 0x400 (when `mode` is nonzero); `0x47db65` tests 0x1 and `0x47db6c` reads
the stand state, byte `+0x210` of the unit's fields; `0x47db76` tests the sign bit of the flag
byte (0x80) and `0x47db7d` the mover's flags `+0x40`; `0x47db8e` tests 0x200 and `0x47db95` the
stand state against 3 and 7; `0x47dbab` to `0x47dbbc` handle 0x4000. `0x5ef5c3` is the only
caller, in `SendTextEmote`, which skips the whole emote and its packet when the result is 0
(`0x5ef5ca`).

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
| `0x5efcd0` | a resistance's base, effective, positive and negative values |
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
`0x5efcd0` takes the base as the resistance field less the positive and negative resistance buff mods
(`0x5efd40`, `0x5efda0`: the player-block words at `+0xf9c` and `+0xfb8`) and sets a negative field to 0 in its two
raw copies, leaving the base as computed (`ui/unitstats#resistances-and-armor`). `0x5ed650` tests the item record's five spells
(`+0x11c`) for trigger 0 (`+0x130`) and an effect 0x5e among the three.
