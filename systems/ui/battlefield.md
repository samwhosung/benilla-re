# ui: battlegrounds

`BattlefieldInfo.cpp` is the battleground client: the list of instances a battlemaster offers, three queue slots, the
score board, the leave and join requests, and the Lua functions and events a battleground frame uses. It registers
31 Lua functions (`tables/bindings.tsv`) and seven opcode handlers. The map and flag position functions share the file
and are in `worldmapmarkers`.

Roots: `0x4a9c40`, `0x4a9d30`, `0x4a9db0`, `0x4aa6c0`, `0x4aa850`, `0x4aab30`, `0x4aa200`

## Caches and lifecycle

Three caches live here: the instance list `0xb6e860`, a growable array of instance ids in wire order (count at `+4`,
data at `+8`), with the battlemaster GUID `0xb6ebb0` beside it; the queue `0xb6e9d0`, three slots of 0x20 bytes; and the
score board `0xb6e880`, 80 pointers to 0x40-byte blocks allocated once. `0x4a9c40` registers the handlers, sets the faction
filter `0x8457d0` and the active slot `0x8457cc` to -1, clears the queue and allocates the blocks; `0x4a9d30` unregisters
and frees them. `0x4a9db0`, run by the world-enter cascade, releases the pending name queries, clears the scalars, sets the
filter to -1 and sends opcode 0x2d3 with no body; it leaves the queue slots and `0x8457cc`.

Addresses: `0x4a9c40`, `0x4a9cf0`, `0x4a9d00`, `0x4a9d30`, `0x4a9db0`, `0x4a9e4d`, `0x4a9ef0`, `0x4a9f40`, `0x4a9f50`, `0x4aa670`, `0x4911fa`, `0x4909fb`, `0xb6e860`, `0xb6e880`, `0xb6e9d0`, `0xb6eba0`, `0xb6eba4`, `0xb6ebb0`, `0xb6ebc0`, `0xb6ebc4`, `0xb6ebcc`, `0xb6ebd0`, `0xb6ebd4`, `0x8457cc`, `0x8457d0`
Confidence: verified
Proof: one
Test: none

A queue slot is: `+0` Map row id, `+4` status 0 to 3, `+8` and `+0xc` bracket levels, `+0x10` instance id, `+0x14` the
confirm deadline, `+0x18` the estimated wait, `+0x1c` the queue stamp. Its two writers are `0x4a9cf0` and `0x4aa850`. A score block
is: `+0` GUID, `+8` kills, `+0xc` honorable kills, `+0x10` deaths, `+0x14` honor, `+0x18` the team, `+0x1c` the rank
and `+0x20` to `+0x3c` eight stat words. The wire count `0xb6ebc0` and the filtered count `0xb6ebc4` differ; `0xb6ebcc` is
the ended flag, `0xb6ebd0` the winner, `0xb6ebd4` the number of names still unknown. The blocks are 80 because the loop at
`0x4a9d00` allocates 0x140 / 4 of them with `SMemAlloc(0x40, ...)`.

## The Lua argument rules

Every function is `__fastcall(L)` with a bare return. A required number is gated by `lua_isnumber` (a numeric string
passes) and truncated toward zero by `__ftol`. An optional flag is read by `GetScriptFlagArg`, which never raises.
Every index is 1-based, tested as `dec eax; cmp eax,N; jae`, so index 0 fails with the others. Fourteen sites raise in the file, each a
`Usage:` string for an argument that is not a number. `GetBattlefieldInstanceInfo` raises with the text
`Usage: GetBattlefieldInfo(index)`; `GetBattlefieldInfo` itself takes no argument and cannot raise.

Addresses: `0x4ab20b`, `0x4ab2d8`, `0x4ab317`, `0x4ab3ce`, `0x4ab4be`, `0x4ab63d`, `0x4ab7ad`, `0x4ab83d`, `0x4ab8de`, `0x4aba04`, `0x4abd18`, `0x4abe52`, `0x4abfb5`, `0x4ac24d`, `0x4ab1f0`, `0x4ab0b0`, `0x6f1c10`, `0x6f34d0`, `0x6f3620`, `0x40a2b0`
Confidence: verified
Proof: one
Test: none

The raise sites are every `push <string>; push L; call 0x6f4940` in `[0x4a9c00, 0x4ac600)`; the last two belong to
the position functions. No function in the file both raises and returns a value. The usage strings are
`Usage: JoinBattlefield(index)`, `Usage: SetSelectedBattlefield(index)`, `Usage: AcceptBattlefieldPort(index, accept)`,
`Usage: GetBattlefieldStatus(index)`, `Usage: GetBattlefieldPortExpiration(index)`,
`Usage: GetBattlefieldEstimatedWaitTime(index)`, `Usage: GetBattlefieldTimeWaited(index)`,
`Usage: ShowBattlefieldList(index)`, `Usage: GetBattlefieldScore(index)`, `Usage: GetBattlefieldStatInfo(index)` and
`Usage: GetBattlefieldStatData(playerIndex, statIndex)`.

## Queue status

`GetBattlefieldStatus(index)`, index 1 to 3, returns five values on every path: the status word (`none`, `queued`,
`confirm`, `active` for 0 to 3, `error` above), the map name (`Name_lang` of the Map row at `+0x10 + 4*locale`, or nil),
the instance id and the two bracket levels. An index outside 1 to 3 returns `nil, nil, 0, 0, 0`. Five time functions return one
number each, in milliseconds: the port deadline and the instance expiration give deadline minus now, 0 when unset or past;
the instance run time and the time waited give now minus the stamp, 0 when unset; the estimated wait gives the stored value.

Addresses: `0x4ab4a0`, `0x4ab4df`, `0x4ab501`, `0x4ab5c7`, `0x4ab604`, `0x4ab620`, `0x4ab6d0`, `0x4ab730`, `0x4ab790`, `0x4ab820`, `0x42c010`, `0xc0daa8`, `0xc0daac`, `0xb6ebb8`, `0xb6ebbc`
Confidence: verified
Proof: one
Test: none

The status strings come through a jump table `0x4ab604` of four addresses; the clock is `OsGetAsyncTimeMs`. The two time functions
without a sign guard zero-extend the difference, so a stamp in the future reads as a large positive number.
`GetBattlefieldInstanceExpiration` reads `0xb6ebb8` and `GetBattlefieldInstanceRunTime` `0xb6ebbc`, which only a status 3 message
sets. The `je` after `add eax,0xb6e9d0` in the slot getters tests the sum, which is never zero.

## The list and the selection

`GetNumBattlefields()` returns the list count and `GetBattlefieldInstanceInfo(n)` the n-th instance id, nothing when out
of range. `SetSelectedBattlefield(n)` stores the instance id at position n, or 0 for a position outside the list.
`GetSelectedBattlefield()` returns the 1-based position of the stored id by a scan, 0 when absent, so a selection follows the
instance id, not the position, and a new list can drop it. `GetBattlefieldInfo()` returns nine values for the battlemaster's map and none
without the player or a Map row: the map name, a description chosen by the player's faction group, three integer and two
float columns of the Map row (`+0x34`, `+0x38`, `+0x40`, `+0x44`, `+0x48`), then the two bracket levels.

Addresses: `0x4ab080`, `0x4ab1f0`, `0x4ab300`, `0x4ab360`, `0x4ab0b0`, `0x4ab129`, `0x4ab132`, `0x4ab135`, `0x4ab1dc`, `0x5efe00`, `0x6f2f30`, `0x4ab2f0`, `0x4ac380`, `0xb6eba0`
Confidence: verified
Proof: one
Test: none

The description is `Map + 4*locale + 0x50` for faction group 0 and `+ 0x74` for group 1 (`0x5efe00` gives 0 for a faction
template with flag 0x4, 1 for flag 0x2, else -1); for -1 nothing is pushed but the function still returns 9
(`mov eax,9` at `0x4ab1dc`), so the ninth value is the slot below the first. `CloseBattlefield()` does nothing.
`CanJoinBattlefieldAsGroup()` pushes 1 when the Map row's `+0xa0` is nonzero, else nil; the join check reads a different column.

## The list message

Opcode 0x23d, `0x4aa6c0`: `u64 battlemaster`, `u32 mapId`, `u8 bracket`, `u32 count`, `count` times `u32 instanceId`. The
bracket gives the levels: `min = bracket * Map[+0x9c] + Map[+0x34]` and `max = min(Map[+0x9c] + min - 1, 60)`, both 0
when the span is not positive. The handler stores the ids, signals `BATTLEFIELDS_SHOW` and only then stores the player's
position (three floats) as the anchor of the walk-away check, zeros without a player.

Addresses: `0x4aa6c0`, `0x4aa6d2`, `0x4aa6de`, `0x4aa6e9`, `0x4aa70e`, `0x4aa71c`, `0x4aa72d`, `0x4aa731`, `0x4aa75c`, `0x4aa7bf`, `0x4aa7d1`, `0x4aa7ff`, `0x4aa804`, `0xb6e870`, `0xb6e874`, `0xb6e878`, `0xb6eba4`, `0xb6eba8`, `0xb6ebac`, `0x4aa090`, `0x4818d5`, `0xb6e854`, `0x4ac54e`
Confidence: verified
Proof: one
Test: `ui::dist2_4aa090`

`BATTLEFIELDS_CLOSED` is fired by `0x4aa090`, called once from a world tick (`0x4818d5`): it proceeds when the squared
distance from the stored anchor exceeds `0xb6e854`, which the static initialiser `0x4ac54e` sets to the square of the
float 5.5555553; it zeroes the anchor before the signal. A list with no battlemaster stores GUID 0 and later joins use the other opcode
(`Joining and leaving`).

`ui::dist2_4aa090` runs the slices `0x4aa096` to `0x4aa0c8` (the guard) and `0x4aa0fb` to `0x4aa12f` (the distance), after running the initializer `0x4ac540` that sets the threshold.

## The status message

Opcode 0x2d4, `0x4aa850`: `u32 slot` (0 to 2; a larger value ends the handler), `u32 mapId`. A zero map id clears the slot
and the stat count, reads nothing more and clears the instance timers when the slot is the active one. Otherwise:
`u8 bracket` (levels as above), `u32` instance id, `u32 status`. Status 2 arms tutorial 0x30 and reads `u32 d`,
setting the deadline `now + d`. Status 3 reads `u32 d1`, `u32 d2` (`0xb6ebb8 = now + d1`, `0xb6ebbc = now - d2`, each 0
when 0), sets `0x8457cc` to the slot, rebuilds the stat columns and runs `0x4aa5a0(-1)`. Status 1 arms tutorial 0x2f,
reads `u32` into `+0x18` and `u32 d`, setting `+0x1c = now - d`.

Addresses: `0x4aa850`, `0x4aa86e`, `0x4aa88b`, `0x4aa8bd`, `0x4aa8f8`, `0x4aa90e`, `0x4aa91d`, `0x4aa92d`, `0x4aa93a`, `0x4aa95e`, `0x4aa96b`, `0x4aa98d`, `0x4aa99d`, `0x4aa9ac`, `0x4aa9dd`, `0x4aaa1f`, `0x4aaa44`, `0x4aaa5a`, `0x4aaa66`, `0x4aaa7c`, `0x4aaa86`, `0x4aaa93`, `0x4aaab3`, `0x4aaac3`, `0x4aab05`, `0x4aab12`, `0x4b5390`, `0xc0d564`, `0xc0d568`, `0xb6ebd8`
Confidence: verified
Proof: one
Test: none

A zero map id also clears the position count `0xb6ebdc` (`0x4aaae6`), and status 2 with d = 0 stores a deadline of 0. Any status but 3 zeroes `0xb6ebb8` and `0xb6ebbc` whatever the slot, and resets `0x8457cc` to -1 when it equals the slot.
Any status but 2 zeroes `+0x14`, any but 1 zeroes `+0x18` and `+0x1c`. The stat columns are a run of consecutive `WorldStateUI` rows
(0x9c bytes each, store `0xc0d564`) starting at the first row whose map word (`+4`) equals the slot's map or -1 and whose type
(`+0x60`) is 2, collected into `0xb6e6ec` with the count `0xb6ebd8`. All paths but the abort signal
`UPDATE_BATTLEFIELD_STATUS` and then restore the reader's cursor and call `0x72b710(reader)`, which parses the same bytes
again. On status 3 `UPDATE_BATTLEFIELD_SCORE` fires, from `0x4aa200`, before the status event.

## Names and the score order

Opcode 0x2e0, `0x4aab30`: `u8 ended`, `u8 winner` only when ended is nonzero, `u32 count`, then per row `u64 guid`, `u32 rank`,
`u32 kills`, `u32 honorableKills`, `u32 deaths`, `u32 honor`, `u32 statCount` and that many words, of which eight are kept.
The count is not clamped to 80. Each row asks the name cache; a miss adds one to `0xb6ebd4` and the callback `0x4a9ef0`
rebuilds when the count reaches zero. `0x4aa200` returns while names are pending; else it sets each row's team (0 for a race
whose faction template has flag 0x4, 1 for flag 0x2, else -1), counts the rows of the filter's team, or all rows for -1,
sorts the array and signals `UPDATE_BATTLEFIELD_SCORE`.

Addresses: `0x4aab30`, `0x4aab69`, `0x4aab7f`, `0x4aab9f`, `0x4aabbb`, `0x4aabda`, `0x4aac0e`, `0x4aac26`, `0x4aac43`, `0x4aac79`, `0x4aac82`, `0x4aaca1`, `0x4aacac`, `0x4aa580`, `0x4aa200`, `0x4aa216`, `0x4aa2ac`, `0x4aa2b7`, `0x4aa2d7`, `0x4aa2ee`, `0x4aa308`, `0x4aa316`, `0x4aa341`, `0x4aa5a0`, `0x4aa5a5`, `0x55f870`, `0x73f727`, `0xb6ebc0`, `0xb6ebc4`, `0xb6ebd0`
Confidence: verified
Proof: one
Test: none

`0x4aa5a0(f)` accepts -1, 0 and 1 and ignores any other value; it stores the filter and runs `0x4aa200`. The rebuild uses the
inline team lookup without the null-row guards of `0x5efe00`: a missing race or faction template row dereferences zero.
A row count of more than 80 walks past the 80-pointer array. The array is sorted, never compacted, so
the filtered count is the number of rows at the front only when the sort puts them there.

## The comparator

`0x4aa350` returns 0 when either name record is missing. With a filter other than -1 it computes both teams; if they
differ the row whose team equals the filter comes first. Then the keys are: kills, the higher first; deaths, the lower first;
honor, the higher first; the name, case-insensitive. Honorable kills and rank order nothing.

Addresses: `0x4aa350`, `0x4aa3d7`, `0x4aa4e9`, `0x4aa4f5`, `0x4aa509`, `0x4aa518`, `0x4aa535`, `0x4aa54e`, `0x4aa567`, `0x4aa575`
Confidence: verified
Proof: one
Test: none

With `A` the first and `B` the second row: `cmp edx,ecx` with `edx = B` and `sbb` give -1 when `B < A` for kills (`and 0xfffffffe; inc`)
and honor, and +1 for deaths (`and 2; dec`), so the larger kills and honor and the smaller deaths come first. Teams
equal go to the keys; teams different return `setne dl; lea edx,[edx+edx-1]` on the filter compare, -1 for the row with the filter's team.

## Scores and stats

`GetNumBattlefieldScores()` returns the filtered count, but `GetBattlefieldScore(index)` is bounded by the wire count, so an
index past the filtered count returns a real row of another team. It returns nine values on every path: the name
(`Name-Realm` with a realm), kills, honorable kills, deaths, honor, team, rank, race and class names (nil when unresolved),
and `nil, 0, 0, 0, 0, 0, 0, nil, nil` for a missing row or name. `GetBattlefieldWinner()` is the winner, nil until ended.
`GetBattlefieldStatInfo(index)` gives a column's text, icon and tooltip, three nils for a negative id and for an index past the
count. `GetBattlefieldStatData(row, stat)` returns the stat word, 0 outside the row, and reads one word past the
block for stat 9.

Addresses: `0x4ab9a0`, `0x4ab9d0`, `0x4aba04`, `0x4abbd9`, `0x4abc40`, `0x4abc90`, `0x4abcd0`, `0x4abd00`, `0x4abd41`, `0x4abdc0`, `0x4abe52`, `0xb6ebc0`, `0xb6e6ec`, `0xc0d56c`
Confidence: verified
Proof: one
Test: none

An index past the count makes `GetBattlefieldStatInfo` look up id 0, which the id index holds as null (`0x4abd41` sets `eax` to 0): three nils, as in `worldstate`. `SetBattlefieldScoreFaction([f])` reads an optional number (-1 otherwise, never an error) and runs `0x4aa5a0`. The stat
getter admits `stat - 1` up to 8 (`cmp eax,8; ja`) while the block holds eight words at `+0x20` to `+0x3c`.
The stat trio's store is `WorldStateUI.dbc` (39 fields, 0x9c-byte rows).

## Joining and leaving

`JoinBattlefield(index [, asGroup])` runs `0x4a9f60`. With asGroup set it shows `ERR_GROUP_JOIN_BATTLEGROUND_TOO_MANY`
(442) and sends nothing unless the Map row's `+0x3c` is at least the party count and the raid count. It then sends opcode 0x2ee
(`u64 battlemaster`, `u32 mapId`, `u32 instanceId`, `u8 asGroup`) when a battlemaster GUID is stored, else opcode 0x23e without
the GUID; the instance id is the list entry or 0. `AcceptBattlefieldPort(index, accept)` sends 0x2d5 (`u32 mapId`, `u8 accept`),
`ShowBattlefieldList(index)` 0x23c with the map id. `LeaveBattlefield()` sends 0x2e1 with the active slot's map id (0 for a
negative slot), only once a score message reported the end. `RequestBattlefieldScoreData` sends 0x2e0 and
`RequestBattlefieldPositions` 0x2e9, each at most once per 5000 ms.

Addresses: `0x4a9f60`, `0x4a9f94`, `0x4a9f9c`, `0x4a9fa0`, `0x4a9fa8`, `0x4a9fbf`, `0x4a9fea`, `0x4aa00b`, `0x4aa01f`, `0x4aa03b`, `0x4aa04c`, `0x4ab290`, `0x4ab3b0`, `0x4ab429`, `0x4ab8c0`, `0x4ab91f`, `0x4abe60`, `0x4abe66`, `0x4abe72`, `0x4abe99`, `0x4ab990`, `0x4aa170`, `0x4aa190`, `0x4aa5c0`, `0x4aa600`, `0x4abef0`, `0xb6ebc8`, `0xb6ebe0`
Confidence: verified
Proof: one
Test: none

`0x4aa170` reads the clock, returns when `0xb6ebc8` is nonzero and `now - stamp` is negative, else sends and stores
`now + 0x1388`; `0x4aa5c0` does the same with `0xb6ebe0`, and `RequestBattlefieldPositions()` (`0x4abef0`) is a call to it. The ended flag `0xb6ebcc` has two readers (`LeaveBattlefield` and
`GetBattlefieldWinner`) and one writer, so a leave before the first log message sends nothing. The slot test is signed with no upper bound; its writers keep the value
in -1 to 2.

## Other messages

Opcode 0x2e8, `0x4aacc0`: `u32 result` shows message 439 (`ERR_GROUP_JOIN_BATTLEGROUND_DESERTERS`) for -2, 440 with the map name
for a valid Map index, else 441. Opcodes 0x2ec and 0x2ed share `0x4aae10` and are told apart by the dispatcher's opcode
register: one GUID, resolved through the name cache with a callback of its own, then message 444 (`ERR_BG_PLAYER_JOINED_SS`,
three arguments) or 445 (`ERR_BG_PLAYER_LEFT_S`, two). Opcode 0x2e9, `0x4aad40`, fills the position arrays and fires no event.

Addresses: `0x4aacc0`, `0x4aae10`, `0x4aae21`, `0x4aae28`, `0x4aaef0`, `0x4aafe0`, `0x4aad40`, `0x4ac190`, `0x4ac108`
Confidence: verified
Proof: one
Test: none

Events by site: `BATTLEFIELDS_SHOW` (`0x4aa7d1`), `BATTLEFIELDS_CLOSED` (`0x4aa15b`), `UPDATE_BATTLEFIELD_STATUS` (`0x4aab05`),
`UPDATE_BATTLEFIELD_SCORE` (`0x4aa308`) and `WORLD_MAP_NAME_UPDATE` (`0x4ac195`, the callback `0x4ac190` registered at
`0x4ac108`); none carries an argument. The messages are the ids of `tables/messages.tsv`.

## Player notices

The handlers of the battleground join and leave packets ask the name cache for the player and pass `0x4aaef0` (joined) or `0x4aafe0` (left) as the callback. A joined notice shows `ERR_BG_PLAYER_JOINED_SS` (message 444) and a left notice `ERR_BG_PLAYER_LEFT_S` (445) with the player's name; when the cache record has a realm name (the byte at `+0x30`), the left notice reads `name-realm` and the joined notice appends `-` and the realm with its spaces and tabs removed.

Addresses: `0x4aaef0`, `0x4aafe0`, `0x55f080`, `0x496720`, `0x4aae2f`, `0x4aae36`
Confidence: verified
Proof: one
Test: none

`0x4aae2f` and `0x4aae36` push the callbacks. `0x4aaf35` tests the realm byte; `0x4aaf5f` packs the `-` (0x844808) and `0x4aaf8a` to `0x4aafa8` copy the realm without 0x20 and 0x9; `0x4aafbb` pushes id 0x1bc and the buffer for both `%s`. `0x4ab040` formats `%s-%s` for the left notice and `0x4ab067` pushes 0x1bd.

