# objectlayer: players

`CGPlayer_C`, the player object: its members beyond the unit, its construction and teardown, the
virtual slots it adds or replaces, its arrival in the world and its death, the world-entry set-up
of the player code, and the area-trigger timer. The fields it watches are `playerfields`; the
server messages it registers are `playermessages`, `quests`, `party`, `inventory`, `interaction`
and `playercombat`.

Player_C.cpp runs from `0x5db900` to `0x5f1280`, its initializers excepted (`initializers`), and
Bag_C.cpp from `0x6220f0` to `0x6229d0`. The functions after `0x5f1290` that the player's tables
call (`0x5f17f0`, `0x5f18f0`, `0x5f2040`) belong to GameObject_C.cpp's template code.

Roots: `0x5db900`, `0x5dd2e0`, `0x5e2030`, `0x5e2500`, `0x5e2510`, `0x5e2e60`, `0x5debe0`, `0x5e99b0`

## The player object

`ObjectConstruct` calls `0x5dd2e0` with its two arguments, which it passes to
`CGUnit_C::CGUnit_C` (`0x5fad50`); then it sets the members below. The
active player also gets 796 empty watcher lists in the block `+0x1c68` points at.

| offset | member |
|---|---|
| `+0x0e70` | 298 object watcher lists, 12 bytes each, for the player fields |
| `+0x1c74` | hash table of `TRADESKILLLINE` nodes (vtable 0x80b044), mask `+0x1c98` |
| `+0x1c9c` | hash table of `PROFICIENCYSPELL` nodes (vtable 0x80b034), mask `+0x1cc0` |
| `+0x1cc4` | list of `PlayerPendingItemExpiration`, linked at `+0x1cc8` |
| `+0x1cd0` | four 16-byte spell arrays, indexed by Spell.dbc `+0x0c` |
| `+0x1d10` | four spell ids, indexed by an effect-47 spell's misc value |
| `+0x1d20` | count of equipment displays waiting on an item record |
| `+0x1d28` | the GUID being looted; `+0x1d30` the auto-loot byte |
| `+0x1d38` | the inventory: count 0x71 for the active player, else 0; `+0x1d3c` the GUID array, the fields `+0x4a8` |
| `+0x1d40` | the player's GUID, `+0x1d48` a byte 1 (the inventory's owner and own-flag) |
| `+0x1d68` | the active player's local stand state |

Addresses: `0x5dd2e0`, `0x5dd300`, `0x5dd456`, `0x5dd467`, `0x5dd473`, `0x5dd496`, `0x5dd4c0`, `0x5dd4df`, `0x5dd2a0`
Confidence: verified
Proof: one
Test: none

`0x5dd300` sets the count 0x12a for the list loop. `0x5dd456` (`neg; sbb; and 0x71`) writes the
inventory count and `0x5dd467` the GUID array, `[+0xe68] + 0x4a8`, which is field 486,
`PLAYER_FIELD_INV_SLOT_HEAD`. `0x5dd473` stores the vtable 0x80af78. `0x5dd496` loads the race's
`ChrRaces` row `+0x1c` into `+0x9c`. A race with no row leaves `eax` 0 and the `fld` reads address
0x1c and faults: an id above the maximum, and, since the loader zero-fills the id map (`0x542797`),
race 0 and every hole. The active-player loop runs `0x5dd4c0` to `0x5dd4df` over 0x2550 bytes.
`0x5dd2a0` points `+0xe68` at field 188 (`classes`).

## Destruction

`0x5dd500` (slot 0) runs `0x5dd600` and frees the block on flag bit 0. `0x5dd600` empties both hash
tables, frees every pending item expiration, releases the four spell arrays, stores a live party or
raid record and asks for the member's stats unless the object is inactive, cancels the guild-record
callbacks `0x5dd860` and `0x5e0650` when the record is still pending, and, for the active player,
unlinks the 796 lists through `0x5dd530`; then it unlinks the 298 lists and runs
`CGUnit_C::~CGUnit_C`.

Addresses: `0x5dd500`, `0x5dd600`, `0x5dd530`, `0x5dd860`, `0x5e0650`
Confidence: verified
Proof: one
Test: none

The record calls are `0x4e85d0(this, 0, 0)` and `0x4badc0(this, 0, 0)`, which store `0x5f0880`'s
record without the pet and send 0x27f, skipped when `0x614ea0` reports bit 0 of `+0xe4`. The guild
check reads `PLAYER_GUILDID` (`[+0xe68]+0xc`) and asks the cache `0xc0e1ec` through `0x560e30`;
`0x5614c0` drops each callback. `0x5dd530` calls `0x490a80` when the manager's `+0xc8` is not 1.

## Arrival in the world

`0x5debe0(time, a, created)`, from the create path, copies the stand state to `+0x1d68`, runs
`CGUnit_C::PostInit`, the name (`GetUnitName`), the guild tag (`0x5e08e0`) and `0x49ea90`, slot 15,
and the party and raid tokens. Any other player then gets its party record compared (`0x5f0b80`);
the active player runs `0x5dea50`, in this order: its watchers (`0x5dd8a0`), `0x4580f0(race,
gender)`, the item cache (`0x55bc60`), the initial spells learned in list order with notification,
the action bar's 120 slots, the ghost state (`0x5de9c0`), aura visibility for every unit, the
camera, client control and in-world state, a pending cinematic (`0xc4d75c`) or the loading
screen's end, and the class's spell family.

Addresses: `0x5debe0`, `0x5dea50`, `0x5f0b80`, `0xc4d75c`, `0xc4c2b4`
Confidence: verified
Proof: one
Test: none

`0x5dea50` learns each `{u16 id, i16}` of `0xc4d2a8` (count `0xc4d2a4`) through `0x5e9c20(id, x, 0,
1)`. With the manager's `+0xc8` zero it shows each action slot of `0xc4c2b4` whose value has a top
nibble or a known spell (`0x4e5d60`), signals `ACTIONBAR_SLOT_CHANGED` with 0, and later calls
`SetClientControl(1)`. A transport in slot 9's GUID (mask 0x20) calls `0x4083c0(1)`; otherwise
`EndLoadingScreen`. It ends with `SetClassSpellFamily` of the class byte.

## Release and reactivation

Slot 1, `0x5e9aa0`, releases a player: `0x4e85d0(this, 0, 1)` and `0x4badc0(this, 0, 1)` store a
live record (`0x5f0880`) into its party or raid slot and send 0x27f with its GUID; it drops the
guild-record callbacks, and, for the active player, removes its watchers, signals
`MIRROR_TIMER_STOP` for the three timers and calls `0x490a80`; after `CGUnit_C::Release` a party
member signals `PARTY_MEMBER_DISABLE` with the slot number `0x4e85d0` returned. Slot 2, `0x5e9bc0`,
copies the stand state, runs `CGUnit_C::Reactivate`, zeroes `+0x1d20`, and re-adds the active
player's watchers. Slot 3, `0x5dec90`, runs `CGUnit_C::PostUpdate`, refreshes the tokens and
compares the party record.

Addresses: `0x5e9aa0`, `0x5e9b90`, `0x5e9bc0`, `0x5dec90`, `0x5f0b80`, `0x5f0880`, `0x5e5510`, `0x5e5720`
Confidence: verified
Proof: one
Test: none

`0x5e9b90` loops type 0 to 2 through `0x5e7ae0`. `0x5f0b80` copies the party (`0x4e8820`) or raid
(`0x4bb0f0`) record, builds a live one with `0x5f0880(this, buffer, 1)`, and passes the change mask
`0x5e5510(old, live)` to `0x5e5720`; it writes neither record back (`party`).

## The player's virtual slots

The player replaces the unit's slots 0 to 4, 14, 16 and 31 to 46 (`classes`).

| slot | function | does |
|---|---|---|
| 14 | `0x5ec950` | `CGUnit_C::Update`, then the attack-error repeat |
| 16 | `0x5ec8f0` | the model update, held while displays wait or the own model is hidden |
| 31, 32, 33 | `0x5ec9e0`, `0x5eca40`, `0x5eca80` | append `CHAT_FLAG_AFK`, `_DND`, `_GM` |
| 37 | `0x5ec430` | the chest item's `+0x318` |
| 38, 39, 40 | `0x5ec240`, `0x5ec350`, `0x5ec380` | a hand's visible item, its display row, its display id |
| 41 | `0x5ed570` | the stand state, local for the active player |
| 42 | `0x5ea690` | a spell's skill value, at most the spell's max level times 5 |
| 43, 44 | `0x5eda20`, `0x5eda80` | the defense and a hand's weapon skill |
| 45 | `0x5ee150` | a spell's cast time |
| 4, 46 | `0x5f1f70`, `0x5f1f80` | the inventory at `+0x1d38` |

Slots 34 to 36 are PlayerSound_C.cpp's.

Addresses: `0x80af78`, `0x5ec950`, `0x5ec8f0`, `0x5ec9e0`, `0x5eca40`, `0x5eca80`, `0x5ec430`, `0x5ec240`, `0x5ec350`, `0x5ec380`, `0x5ed570`, `0x5ea690`, `0x5eda20`, `0x5eda80`, `0x5ee150`, `0x5f1f70`, `0x5f1f80`
Confidence: verified
Proof: one
Test: none

Read from the dwords of `0x80af78`. `0x5ec8f0` returns 0 while `+0xccc` and `+0x1d20` are both set;
otherwise it runs `CGUnit_C::OnModelUpdate` and, for the active player, returns its result only
while `0xc4d74c` is set (`0x5ec8c0` toggles it, `0x5ec8e0` sets it). The tags test `PLAYER_FLAGS`
(`[+0xe68]+8`) bits 0x2, 0x4 and 0x8; the AFK tag also shows for the active player while
`0xb6e5cc` is set.

## Attack error repeat

Every frame the active player with a melee target (`+0xc48`) shows `ERR_BADATTACKPOS` (0xd8) when
`0xc4d758` is 1 or `ERR_BADATTACKFACING` (0xd7) when it is 2, at most once per 4,000 ms.
`0x5ecdb0(state)` sets the state and restarts the delay.

Addresses: `0x5ec950`, `0x5ec999`, `0x5ec9b6`, `0x5ecdb0`, `0xc4d758`, `0xc4c270`
Confidence: verified
Proof: one
Test: none

`0x5ec999` takes `now − [0xc4c270]` and skips when it is negative (`js`). `0x5ec9b6` stores
`now + 0xfa0` before `DisplayError`. `0x5ecdb0` writes `ecx` to `0xc4d758` and the time to
`0xc4c270`. `0x5ea800` sets state 0 when an attack starts (`playercombat`).

## Death

`CGUnit_C::OnDied` calls `0x5e99b0`. Another player who is a party member shows
`ERR_PLAYER_DIED_S` with the name. The active player arms the release timer for 360,000 ms when
`PLAYER_FIELD_BYTES` bit 0x8 is set, else clears it; clears the target, calls `0x4931c0` and
`0x48f200`, refreshes quest-giver statuses, cancels the cast (`0x1c`), and requests a release
(`CMSG_REPOP_REQUEST`) at once when `UNIT_FIELD_FLAGS` has bit 0x100000.

Addresses: `0x5e99b0`, `0x5e9a27`, `0x5e9a30`, `0x5e9a7d`, `0x5e0ae0`
Confidence: verified
Proof: one
Test: none

`0x5e9a27` tests `[+0xe68]+0x1028` (field 1222) bit 0x8; `0x5e9a30` passes 0x57e40 to `0x496250`,
which stores `now + ms` in `0xb4e340`; `0x496280` zeroes it. Then `0x493540(0,0)`,
`0x4931c0(0xb4e2d0, 0)`, `0x48f200(1,0,0)`, `0x5eb070`, `CancelCast(1,1,0x1c)`. `0x5e9a7d`
tests bit 20 of `[+0x110]+0xa0`; `0x5e0ae0` sends 0x15a with no body while the player is dead or
a ghost.

## Area list polling

`0x5db900` counts its calls in `0xc4d790` and on every tenth reports the active mover's, or else the
active player's, `+0xe0` with the map to `AreaListRegisterLocation`; it returns 1.

Addresses: `0x5db900`, `0xc4d790`, `0x67e510`
Confidence: verified
Proof: one
Test: none

The counter wraps by subtracting 10 at `0x5db917`. The mover is `s_activeMoverGuid`, looked up with
mask 8; the player with mask 0x10. Its caller is `0x401ec0`, which `ClientInitializeGame` registers
on event 5 with `EventRegister` and `ClientDestroyGame` removes with `EventUnregister`.

## World entry and exit

`ClientInitializeGame` calls `0x5e2030`, then `0x5e2510`; `ClientDestroyGame` calls `0x5e2e60`,
then `0x5e2500`. `0x5e2030` registers the 130 handlers (`0x5e3010`), clears the proficiency table,
the resurrect request and `0xc4d74c`'s state (set to 1), registers the CVars
`CombatModeMaxDistance` ("30.0f"), `guildMemberNotify` ("0") and `autoClearAFK` ("1"), and starts
the area-trigger timer for the current map. `0x5e2500` clears every handler (`0x5e8f20`) and stops
the timer.

Addresses: `0x5e2030`, `0x5e3010`, `0x5e8f20`, `0x5e942f`, `0x5e2490`, `0x5e2500`, `0x5e22a0`, `0xc4d3c4`, `0xc4d68c`
Confidence: verified
Proof: one
Test: none

`0x5e3010` pairs `mov ecx,opcode` with `mov edx,handler` before `0x5ab650`; `0x5e8f20` clears the
same 130, the last (0x328) by the tail `jmp` at `0x5e942f`. `0x5e2490` discards the first CVar
pointer and stores the second, `guildMemberNotify`, in `0xc4d3c4` (`0x5e24e5`) and the third in
`0xc4d68c`. `0x5e22a0` cancels the timer through `0x401550`.

## Area triggers

A 100 ms timer checks the active player's position against the current map's `AreaTrigger` rows.
Entering a trigger sends a movement heartbeat (0xee) and `CMSG_AREATRIGGER` (0xb4)
with the row id; staying inside sends nothing. A row with radius 0 is a box: half its length,
width and height around its centre, rotated by its orientation about z, strictly inside on each
axis. Any other row is a sphere, inside unless `r² < d²`.

Addresses: `0x5e2080`, `0x5e2110`, `0x5e22d0`, `0xc4d6ac`, `0xc4c264`, `0xc4c498`, `0xc4d73c`, `0xc4d3cc`
Confidence: verified
Proof: one
Test: none

`0x5e2080(map)` finds the map's first and end rows in the store `0xc0e02c` (0x28-byte rows sorted by
`+4`), keeps them in `0xc4d6ac` and `0xc4c264`, the map in `0xc4c498`, clears the current trigger
`0xc4d73c`, and arms `0x5e2110` through `0x401520(100)`. `0x5e2110` re-arms itself, needs health
above 0, reloads on a map change, keeps the current trigger while `0x5e22d0` holds, else takes the
first row that holds. `0x5e22d0` builds the box transform with `0x7bdc40`, `RotateMatrix` and the
inverse `0x7bd700`; a NaN fails the box and passes the sphere.
