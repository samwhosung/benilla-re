# objectlayer: player fields

The watchers the player code puts on player and unit fields, and what each change does: the
inventory, money, skills, rest, far sight, flags, drunkenness, guild stamps and the charm and
summon tokens. Type watchers run for every player from world entry to exit; object watchers run
for the active player alone. The watcher machinery itself is `fields`. A watcher callback takes
`ecx` the byte offset in its section, `edx` the length, then the GUID, the old bytes and its
argument, and returns with `ret 0x10`.

Roots: `0x5e2510`, `0x5e2e60`, `0x5dd8a0`, `0x5de5b0`

## Watchers on every player

`0x5e2510` registers type watchers (`ClntObjMgrSetTypeMirrorHandler`) at world entry, after
clearing the vendor GUID, the wrap item, the bind point's valid flag and the attack-error state,
and filling the weapon-skill spells (`playercombat`).

| section, offset | fields | callback |
|---|---|---|
| player `+0x118 + 0x30·i` | the 14 visible items of mask 0x7c3fd (slots 0, 2 to 9, 14 to 18), 48 bytes each | `0x5e2810` |
| unit `+0x00`, 16 bytes | `UNIT_FIELD_CHARM`, `UNIT_FIELD_SUMMON` | `0x5e2660` |
| player `+0x08` | `PLAYER_FLAGS` | `0x5e2850` |
| player `+0x0c`, `+0x10` | `PLAYER_GUILDID`, `PLAYER_GUILDRANK` | `0x5e2770`, `0x5e27b0` |
| player `+0x1d`, `+0x1e`, `+0x1f` | `PLAYER_BYTES_3` bytes 1, 2, 3 | `0x5e2a90`, `0x5e2a30`, `0x5e2a50` |
| player `+0x20`, `+0x24` | `PLAYER_DUEL_TEAM`, `PLAYER_GUILD_TIMESTAMP` | `0x5e27d0`, `0x5e2890` |

Addresses: `0x5e2510`, `0x5e2541`, `0x5e254f`, `0x5e2810`, `0x5e2660`, `0x5e2850`, `0x5e2770`, `0x5e27b0`, `0x5e2a90`, `0x5e2a30`, `0x5e2a50`, `0x5e27d0`, `0x5e2890`, `0x468070`
Confidence: verified
Proof: one
Test: none

The visible-item loop starts at `0x5e2541` (`mov esi,0x118`), tests the slot's bit at `0x5e254f`
and registers 0x30 bytes with kind 1. The guild id refreshes the guild tag (`0x5e08e0`) and calls
`0x49ea90`; the rank signals `PLAYER_GUILD_UPDATE`; the duel team and `PLAYER_BYTES_3` byte 3 run
`CGUnit_C::RefreshReaction`; bytes 2 and 3 call `0x491a00` with the GUID; a visible item calls
`0x5dee30(offset, old)`; the flags call `0x5ee990(old)`.

## Visible-item watcher removal

`0x5e2e60` removes the type watchers at world exit, but its visible-item loop starts at `+0x120`
instead of `+0x118`, so it names fields no watcher is on and removes none of the 14. The miss has
no effect: `ClientDestroyGame` calls `0x5e2e60` at `0x401fd5` and then `ClntObjMgrReleaseWatchers`
at `0x402043`, which frees every watcher on the 0x502 lists of all eight sections, the 14 among
them. In the world each is registered once, by `0x5e2510` from `ClientInitializeGame` at
`0x401648`, its one caller.

Addresses: `0x5e2e60`, `0x5e2f1d`, `0x5e2f3f`, `0x468240`, `0x468490`, `0x401fd5`, `0x402043`
Confidence: verified
Proof: one
Test: none

`0x5e2f1d` is `mov esi,0x120`; the loop adds 0x30 and stops at 0x570, calling `0x468240` with
section 4, offset `esi` and callback `0x5e2810` at `0x5e2f3f`. `0x468240` searches the list of
field `offset/4`, and the registrations sit on the lists of fields `0x118/4 + 12·i`, 70 and on,
never 72 and on. `0x468490` walks 8 sections of 0x502 lists of 12 bytes from `0xb23310`, reading each list's `+8`,
(`0x4684a8` to `0x46852a`), unlinking and freeing each node. `0x5e2e60` also frees the
guild-stamp, initial-spell, item-swap and weapon-spell arrays and clears the bind and tabard state.

## Watchers on the active player

`0x5dd8a0` registers object watchers on the active player (`ClntObjMgrSetObjectMirrorHandler`), and
`0x5de5b0` removes them. The removal also asks for a watcher on
`PLAYER_FIELD_LIFETIME_HONORABLE_KILLS` (`+0x10ac`) that nothing adds, which removes nothing.

| fields | callback |
|---|---|
| inventory slots 0 to 22, bank 39 to 68, keys 81 to 112 (8 bytes each) | `0x5ddcf0` |
| the 20 quest-log entries (12 bytes each) | `0x5ddd80` (`quests`) |
| `PLAYER_FIELD_COINAGE` | `0x5ddf30` |
| `PLAYER_FIELD_BYTES` byte 1 | `0x5ddff0` |
| `UNIT_FIELD_CHARMEDBY` | `0x5de090` (`CGUnit_C::RefreshReaction`) |
| `UNIT_FIELD_BYTES_1` byte 2 | `0x5de010` |
| `PLAYER_FARSIGHT` | `0x5de0d0` |
| each of 128 skills: value, max, permanent and temporary bonus | `0x5de180`, `0x5de450`, `0x5de470` |
| `PLAYER_BYTES_2` byte 3 | `0x5de4e0` |
| `PLAYER_FIELD_BYTES2` byte 1 | `0x5de560` |
| experience, rest, resistances, damage mods, kills, PvP rank, watched faction | `0x5de4b0`, each with its event |

Addresses: `0x5dd8a0`, `0x5de5b0`, `0x5de922`, `0x467e70`, `0x467fb0`, `0x5de4b0`
Confidence: verified
Proof: one
Test: none

The registrations pair an offset in `edx` with the callback and length pushed before `0x467e70`.
`0x5de4b0(event)` signals its argument for "player": `UPDATE_EXHAUSTION` (`+0xf6c`),
`PLAYER_XP_UPDATE` (`+0x840`, `+0x844`), `UNIT_RESISTANCES` (`+0xf9c`, 0x38 bytes),
`PLAYER_DAMAGE_DONE_MODS` (`+0xfd4`, `+0xff0`, `+0x100c`), `PLAYER_PVP_KILLS_CHANGED` (`+0x1098`),
`PLAYER_PVP_RANK_CHANGED` (`+0x1c`, `+0x10c0`), `UPDATE_FACTION` (`+0x10c4`). `0x5de922` loads
`edx` 0x10ac for a removal with no matching add in `0x5dd8a0`.

## Inventory and money changes

An inventory slot change passes the slot's new GUID to `0x495420` when it differs from the old
one, and signals `PLAYERBANKSLOTS_CHANGED` for slots 39 to 68. A money change plays
`LOOTWINDOWCOINSOUND`, signals `PLAYER_MONEY`, refreshes quest-giver statuses, and lowers the
player's trade offer to the money left when the offer is larger.

Addresses: `0x5ddcf0`, `0x5ddd4e`, `0x5ddd55`, `0x5ddd69`, `0x703e50`, `0x5ddf30`, `0x5d4760`
Confidence: verified
Proof: one
Test: none

`0x5ddcf0` takes the slot as `(offset − 0x4a8) >> 3`, the current GUID from the inventory list (0
past its count) and calls `0x495420` with it at `0x5ddd4e` when it differs from the old GUID the callback receives, so
the arriving item is the one unlocked. `0x5ddd55` tests the slot against 0x27 to 0x3e, the 24 bank slots, and
the last test against 0x3f to 0x44, the six bank bags; `0x5ddd69` signals event 0x15b with no arguments
through `FrameScript_SignalEvent`. `0x5ddf30` calls `0x4d7d40`, then `0x5eb070`, then
`TradeSubtractMoney(offer − money)` when `TradeGetPlayerMoney` (`0x5d4760`) exceeds the money.

## Combo points, form and far sight

Combo points (`PLAYER_FIELD_BYTES` byte 1) fire the usable-action events and
`PLAYER_COMBO_POINTS`. A shapeshift form change fires the usable-action events, calls `0x4b3250`
and `0x4e4fc0`, and calls `0x4931c0(0xb4e2d0, 0)` when the active player takes a form whose
`SpellShapeshiftForm` row lacks flag 0x8. A far-sight change runs the far-sight update
(`interaction`) and signals `PLAYER_FARSIGHT_FOCUS_CHANGED`.

Addresses: `0x5ddff0`, `0x5de010`, `0x5de0d0`, `0x5ee270`, `0x5ee590`, `0xc0d76c`
Confidence: verified
Proof: one
Test: none

The form is `UNIT_FIELD_BYTES_1` byte 2 (`[+0x110]+0x212`) looked up in the store `0xc0d76c`
(rows `+8`, bound `+0xc`), flag at row `+0x2c`. `0x5de0d0` calls `0x5ee270` when the new far-sight
object exists and `0x5ee590` when it does not.

## Skill changes

A skill value change prints "Skill %d increased from %d to %d" to the console when the shown value
(the value plus the permanent bonus when above 0) differs from the old one. Then, when the skill's
`SkillRaceClassInfo` lacks flags 0x402, an old value of 0 shows `ERR_SKILL_GAINED_S`, and a rise
of the active player shows `ERR_SKILL_UP_SI` with the new value. A skill of category 11 refreshes
quest-giver statuses. Every value change ends in `0x4d7d40` and `0x4fca20(0)`.

Addresses: `0x5de180`, `0x5de450`, `0x5de470`, `0x5ed990`
Confidence: verified
Proof: one
Test: none

`0x5de180` reads the entry `k = (offset − 0x84c)/12` of the block at `+0x848` (field 718): id
`+0`, value `+4`, max `+6`, temporary bonus `+8`, permanent `+0xa`. It calls `0x5ed990`, prints,
calls `0x4d2cb0`, looks up `SkillLine` (`0xc0d924`) and `FindSkillRaceClassInfo`, and ends in
`0x4d7d40` and `0x4fca20(0)`; an unchanged value goes straight to the end. `0x5de450` (max and
permanent bonus) calls `0x4d7d40`, `0x4fca20(0)` and `0x4d2cb0`; `0x5de470` (temporary bonus) calls
`0x5ed990` and `0x4d2cb0`.

## Rest state

A rest-state change (`PLAYER_BYTES_2` byte 3) signals `UPDATE_EXHAUSTION`; for state 1 it shows
`ERR_EXHAUSTION_RESTED` and the rest tutorial, for state 2 `ERR_EXHAUSTION_NORMAL`. States 0 and
3 and above show nothing. `PLAYER_FIELD_BYTES2` byte 1 refreshes aura visibility with the bits
that changed.

Addresses: `0x5de4e0`, `0x80af50`, `0x5de560`, `0x5f0d90`
Confidence: verified
Proof: one
Test: none

`0x5de4e0` reads `[+0xe68]+0x1b` and indexes the 8-byte table `0x80af50` for states below 3, each
an error id and a tutorial with 0x1d1 and 0x32 as none. `0x5de560` passes `new ^ old` to
`0x5f0d90`, which calls `CGUnit_C::RefreshAuraVisibility` on every unit through `0x5f0db0`.

## Player flag changes

`0x5ee990(old)` reacts to the changed bits of `PLAYER_FLAGS`, in this order: AFK, DND or GM (0xe)
mark the name dirty and copy the active player's AFK bit to `0xb6e5cc`; bits 0x180 refresh the
reaction; 0xc00 call `0x524e10`; the unit event `PLAYER_FLAGS_CHANGED` always; 0x400 and 0x800 hide
or show the helm and cloak. For the active player: 0x10 runs the ghost state, 0x20 signals
`PLAYER_UPDATE_RESTING` (with the rest tutorial when set), 0x200 shows the PvP toggle error and
its verbose text, 0x3000 signal `PLAYTIME_CHANGED`.

Addresses: `0x5ee990`, `0x5e2850`, `0x5ee660`, `0x5ee740`, `0x5ee7f0`, `0x5ee8d0`, `0x5de9c0`, `0xb6e5cc`
Confidence: verified
Proof: one
Test: none

The delta is `old ^ [[+0xe68]+8]`. The name mark is `0x6c78f0(+0xc7c)`; the event goes through
`0x515e50` with 0x197. Bit 0x400 set calls `0x5ee740` (hide), clear `0x5ee660` (show); 0x800 the
same with `0x5ee8d0` and `0x5ee7f0` (`inventory`). Bit 0x10 also calls `0x491f50`, `0x5eb070`,
`0x6c7800`. The PvP errors are 0x1b5 and 0x1b6, the texts `PVP_TOGGLE_ON_VERBOSE` and
`PVP_TOGGLE_OFF_VERBOSE` printed through `0x49a870` as type 0xa.

## Ghost state

`0x5de9c0` applies `PLAYER_FLAGS` bit 0x10: set, it switches to the ghost fog `[0xb4b39c]`, calls
`0x6d4620(4)` and the ghost tutorial; clear, it restores `[0xb4b350]`, calls `0x6d4630`,
`0x6d4900` and `0x4921c0(0,0)` and signals `PLAYER_UNGHOST`. Both end in `0x458680` with the bit.

Addresses: `0x5de9c0`, `0xb4b39c`, `0xb4b350`
Confidence: verified
Proof: one
Test: none

The two branches call `SetCurrentFfx` with the stored pointer; the tutorial is
`0x4b5390(0x18, 0)`; `0x6d4900` takes `[0x80af74]` and −1.

## Drunk messages

`PLAYER_BYTES_3` byte 1 is the drunk level, `min(b,100)·0.01` as f32. For the active player it also
updates the target, name plates and camera. It tests the thresholds 0.01, 0.5 and 0.9 in that order,
as k = 0, 1 and 2: a rise across threshold k (`old < t ≤ new`) prints `DRUNK_MESSAGE_SELF%d`
(`_OTHER%d` with the name) for k + 2 and skips threshold k + 1; a fall across it (`new < t ≤ old`)
prints k + 1. A rise across two thresholds prints only the lower one's message, and a rise across
all three prints messages 2 and 4; a fall across two prints both, the lower first. A NaN level
crosses nothing.

Addresses: `0x5e2a90`, `0x80af68`, `0x5e2b6f`, `0x5e2b8b`, `0x5e2b9b`, `0x5e2bab`, `0x5e2bbf`, `0x5e2bd0`, `0x5e2c70`
Confidence: verified
Proof: one
Test: none

The active player's extra steps signal `UNIT_LEVEL` for a target the player can attack, refresh the
name plates and pass the level to the camera. The thresholds are the floats at `0x80af68`, the index
starts at 1 (`0x5e2b6f`). `0x5e2b8b` (`test ah,5; jp`) passes only C0 alone, `old < t`, then
`0x5e2b9b` needs `new` not below; the falling test at `0x5e2bab` and `0x5e2bbf` is the mirror. A
crossing with `new > old` (`0x5e2bd0`) adds one to the index and 4 to the table offset before
printing; the loop end at `0x5e2c70` adds them again. The message goes through `0x49a870` as type
0xa; the camera call is `0x511250`.

## Guild timestamps

A changed `PLAYER_GUILD_TIMESTAMP` updates the guild's entry in a growable `{guild, stamp}` table,
adding one for a new guild; an unchanged stamp does nothing. A new stamp drops the cached guild
record and queries it again; its arrival refreshes the guild tag of every player in that guild.

Addresses: `0x5e2890`, `0x5e29c0`, `0x5e29e0`, `0xc4d668`, `0x561d20`
Confidence: verified
Proof: one
Test: none

The table is capacity `0xc4d668`, count `+4`, data `+8`, chunk `+0xc` (32). `0x561d20` drops the
record from the cache `0xc0e1ec`; the query's callback `0x5e29c0` walks every object with
`0x5e29e0`, which calls `0x5e08e0` on players of that guild.

## Charm and summon tokens

A change to a player's `UNIT_FIELD_CHARM` or `UNIT_FIELD_SUMMON` matters for the active player and
party members: for each of the two, a live old unit and the live new unit get
`CGUnit_C::UpdateUnitTokenFlags`, and an old GUID with no unit that differs from the new one goes
to `0x493910(1, old)`.

Addresses: `0x5e2660`, `0x4918e0`
Confidence: verified
Proof: one
Test: none

`0x5e2660` tests the active player or `0x4918e0`, then for each of the two GUIDs compares old and
current, and calls `0x60c520` on the live units.
