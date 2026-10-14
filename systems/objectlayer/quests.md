# objectlayer: quests

The quest messages the player code reads: the quest giver's greeting, details, item request,
rewards and completion; the quest updates; the quest log's field watcher; quest progress for bought
items; and the quest-giver status markers the client keeps fresh. The quest window and log they
feed are `ui`'s. Several readers trust the server's counts against fixed arrays.

Roots: `0x5e59b0`, `0x5e5ad0`, `0x5e5eb0`, `0x5ddd80`, `0x5eb070`, `0x5eb3c0`

## Quest giver messages

`0x5e59b0` dispatches nine opcodes for the active player through the bytes `0x5e5ab8` and the
cases `0x5e5a90` (value − 0x183, `cmp 0x12`); any other opcode returns 0.

| opcode | reader | reads |
|---|---|---|
| 0x183 | `0x5dc9f0` | GUID, i32 status: sets it on a unit whose NPC flags have 0x2 |
| 0x185 | `0x5dbb10` | the greeting list |
| 0x188 | `0x5dbd50` | the details |
| 0x18b | `0x5dc050` | the item request |
| 0x18d | `0x5dc1e0` | the reward offer |
| 0x18f | `0x5dbca0` | i32 reason: the invalid-quest error, then `0x501130(0, 0)` |
| 0x191 | `0x5dc400` | the completion |
| 0x192 | `0x5dc840` | i32 quest, i32 reason: the failure |
| 0x195 | none | `ERR_QUEST_LOG_FULL` |

Addresses: `0x5e59b0`, `0x5e5ab8`, `0x5e5a90`, `0x5dc9f0`, `0x5dbb10`, `0x5dbd50`, `0x5dc050`, `0x5dc1e0`, `0x5dbca0`, `0x5dc400`, `0x5dc840`, `0x607440`
Confidence: verified
Proof: one
Test: none

The byte table has 19 entries into ten cases. `0x5dc9f0` reads `[+0x110]+0x234` (`UNIT_NPC_FLAGS`)
and calls `CGUnit_C::SetQuestGiverStatus` (`0x607440`). `0x5dbca0` maps reason − 1 through the bytes
`0x5dbd30` into `0x5dbd14`: 1 `ERR_QUEST_LOW_LEVEL`, 6 `_WRONG_RACE`, 12 `_ONLY_ONE_TIMED`, 13
`_ALREADY_ON`, 20 `_MISSING_ITEMS`, 22 `_NOT_ENOUGH_MONEY`, any other `ERR_QUEST_NEED_PREREQS`;
then `0x501130(0, 0)`.

## The greeting list

0x185 reads the giver's GUID, a greeting of at most 0x200, a delay and an emote, and a u8 count of
quests, each an id, an icon, a level and a title of at most 0x40. Ids, icons and levels go to three
32-dword arrays, `0xc4d2b0`, `0xc4d340` and `0xc4d6b0`, the titles to 32 stack slots. Icons 3 and 4
add the quest with its level through `0x500e90`, others through `0x500e40`. The giver queues the
emote with its delay. A count above 32 runs past all four: the arrays into the globals after
them, among them the weapon-skill table pointer `0xc4d730`; the titles over the greeting buffer, the
locals and the return address.

Addresses: `0x5dbb10`, `0x5dbbbb`, `0x5dbc42`, `0xc4d2b0`, `0xc4d340`, `0xc4d6b0`, `0xc4d740`
Confidence: verified
Proof: one
Test: none

The frame is 0xa14 bytes: titles from `ebp−0xa14` (`0x5dbbbb`) at 0x40 each (`0x5dbc42`), the
greeting at `ebp−0x214`, locals from `ebp−0x14`; title 40 starts at `ebp−0x14` and covers the
return address. The giver's GUID goes to `0xc4d740`. `0x500bd0(0, greeting, guid)` opens the
greeting before the loop and `0x500ee0` ends it; the emote is `SetEmoteQueue` with one entry.

## Details, request and reward

0x188 reads the giver, quest, title (0x200), details and objectives (0x800 each), then a count of
choice items into 6-entry stack arrays, a count of reward items into 4-entry arrays, money, a
value, and a count of emotes into a stack block of 8 bytes each. A player giver, while the GUID
at `0xbe0810` is set, gets 0x276 with result 5 instead. 0x18b reads a count of required
items into 6-entry arrays and four flags, all nonzero meaning completable. 0x18d reads emotes,
choices (6) and rewards (4). None bounds its counts: a count past an array's entries writes on up
the frame, and the emote block is sized from the count.

Addresses: `0x5dbd50`, `0x5dbea5`, `0x5dc050`, `0x5dc1e0`, `0xbe0810`
Confidence: verified
Proof: one
Test: none

Each item is three i32 (id, count, display). `0x5dbd50` shows the text through `0x500bd0(1, …)`,
`0x500dc0` and `0x500ef0`; a creature giver plays the emotes (`0x60bb80`). `0x5dc050` uses
`0x500bd0(2, …)` and `0x501070`, `0x5dc1e0` `0x500bd0(3, …)` and `0x500ef0`. The emote block is
`alloca((8·count + 3) & ~3)` (`0x5dbea5`), pairs written u32 to `+0`, i32 to `+4`. The size wraps
modulo 2^32 for a count of 2^29 or more, so such a count can get a block too small for it; a size
above the 0x100000-byte stack reserve, from a count of about 131,000, faults in the probe.

## Quest completion

0x191 reads the quest, a value, the experience, the money and a count of reward items into
5-entry stack arrays, unbounded. It shows `ERR_QUEST_COMPLETE_S` with the title of a quest whose
record `+0x24` is 0, `ERR_QUEST_REWARD_EXP_I` for nonzero experience, `ERR_QUEST_REWARD_MONEY_S`
for money above 0, and `ERR_QUEST_REWARD_ITEM_S` with a link for each reward item and then the
chosen item `0xbe0828`, which it clears. A record not yet cached shows its line when it arrives.

Addresses: `0x5dc400`, `0x5dc710`, `0x5dc7b0`, `0xbe0828`, `0x6c6260`
Confidence: verified
Proof: one
Test: none

The money splits through `0x6c6260` into copper, silver and gold, each nonzero part "%d %s" with
the texts `COPPER`, `SILVER`, `GOLD`, joined as `%s%s%s%s%s` gold to copper with ", " only between
nonzero parts. The quest cache is `0xc0e1b0` with callback `0x5dc710`; items use `0x5dc7b0`, which
builds the link with `0x5d8b00` and `0x52adb0`. It ends with `0x501130`, and refreshes quest-giver
statuses when record `+0x4` is 0.

## Quest failure

0x192 stores the reason in `0xc4d794` and, with the quest's record, shows
`ERR_QUEST_FAILED_BAG_FULL_S` and `ERR_INV_FULL` for reasons 4 and 0x32,
`ERR_QUEST_FAILED_MAX_COUNT_S` for 0x11, and otherwise `ERR_QUEST_FAILED_S` when the quest is not
already done; then `0x501130(0, 0)`. A record still pending shows the same lines on arrival.

Addresses: `0x5dc840`, `0x5dc920`, `0xc4d794`
Confidence: verified
Proof: one
Test: none

The done test is `0x4df4b0(quest)`. The callback `0x5dc920` repeats the three cases without the
closing call.

## Quest updates

`0x5e5ad0` handles 0x196 to 0x19a. 0x19a reads an item and count and reports progress for each
log quest not done, as below. The others read a quest; a quest is done when
`0x4df4b0` says so or its log entry has failed bit 0x2. An unknown quest prints "Unkown questID
(%d) in QuestUpdate". 0x197 first calls `0x4df550`, then acts as 0x196: `ERR_QUEST_FAILED_S`.
0x198 shows `ERR_QUEST_OBJECTIVE_COMPLETE_S` with the objective text and `QUEST_WATCH_UPDATE`, or
`ERR_QUEST_UNKNOWN_COMPLETE` without text. 0x199 reads an id, a count, a need and a GUID: the
quest's own objective text gives `ERR_QUEST_ADD_FOUND_SII`, else an id with bit 31 names a game
object and shows the same error, else a creature `ERR_QUEST_ADD_KILL_SII`; then
`QUEST_WATCH_UPDATE`.

Addresses: `0x5e5ad0`, `0x5e5ea0`, `0x4df4b0`, `0x4df880`, `0x5e5db0`
Confidence: verified
Proof: one
Test: none

The switch is value − 0x196 through `0x5e5ea0`. The texts are record `+0x129c` (objective),
`+0x149c` (four ids) with texts at `+0x14dc + 0x100·i`; names come from the game-object cache
`0xc0e318` or the creature cache `0xc0e354`, plural or singular by count, with no callback, so a
miss shows nothing. `QUEST_WATCH_UPDATE` takes `0x4df880(quest)`: the 1-based position of the quest
among the 40 16-byte entries at `0xbb71c0`, headers counted, skipping the entries whose `+8` header
flag is set (`0x4de719`, in `0x4de510`), and 0 when the quest is absent. 0x199 never tests the quest
record: for a quest whose record is not cached it reads `[0x149c]` at `0x5e5db0` and faults.

## Quest progress for items

A bought item (0x1a4) or an added item (0x19a) checks each of the 20 log quests not done: a quest
needing the item, among its four, shows `ERR_QUEST_ADD_ITEM_SII` with the item's name, the count
held plus the new count capped at the need, and the need, then `QUEST_WATCH_UPDATE`; the first
quest that needs it stops the walk, and so does an item whose record is not cached, silently. A
held count already at the need shows nothing. A quest record not yet cached queues the pair and
reports when it arrives.

Addresses: `0x5dd060`, `0x5dd180`, `0x5dced0`, `0x622130`
Confidence: verified
Proof: one
Test: none

`0x5dd060(record, item, count)` searches `+0x14bc` to `+0x14c8`, needs `+0x14cc + 4·i`, counts
with `0x622130(inventory, item, 8)` and uses the item name `+8 + 4·plural`, the plural chosen by
the need (`0x703cf0`). The log is
`[+0xe68]+0x28`, 12 bytes per entry. `0x5dd180` is the callback; it frees the 8-byte pair.

## The quest log watcher

A change to a quest-log entry refreshes quest-giver statuses, then, for a new quest id, or the
same id whose failed bit 0x2 cleared, shows `ERR_QUEST_ACCEPTED_S` with its title and raises
tutorial 0x28 when record `+0x10` is 1. An entry whose old quest left calls `0x4df0e0`. Every
change ends in `0x4e8be0`.

Addresses: `0x5ddd80`, `0x5ddeb0`, `0x4df0e0`, `0x4e8be0`
Confidence: verified
Proof: one
Test: none

The entry is `(offset − 0x28)/12` (`0xaaaaaaab`, `>> 3`); the title is record `+0x9c` from
`0xc0e1b0`, with `0x5ddeb0` as the callback for a pending record.

## Quest-giver status refresh

`0x5eb070` walks every object for the active player. A plain unit (type 9) with NPC flag 0x2 and a
reaction above 1 gets `CMSG_QUESTGIVER_STATUS_QUERY` (0x182); a unit with a reaction of 1 or less
loses its marker. A game object with flag 0x4 and an object reaction above 1 is queried too.
`0x5eb3c0` does the same for game objects, and calls `CGUnit_C::RefreshQuestGiverStatus` on
plain units.

Addresses: `0x5eb070`, `0x5eb0a0`, `0x5eb3c0`, `0x5eb3f0`, `0x5eafb0`, `0x5eb030`, `0x6073f0`, `0x5edb60`
Confidence: verified
Proof: one
Test: none

`0x5eb0a0` tests the type `[[+8]+8]` for 9 or bit 0x20, the game object's flags `[+0x110]+0xc`
bit 0x4 and `CGGameObject_C::ObjectReaction`, the unit's `+0x234` bit 0x2 and
`CGUnit_C::UnitReaction`; a unit without the flag and with a reaction of 1 or less also calls
`CGUnit_C::ReleaseQuestMarker`. `0x5eb030(object)` sends the query for one object. A level change
of the active player (`0x5edb60`) also refreshes them, after `0x4b2fd0(0,0)` and `0x4d7d40`, then
calls `0x4e89c0` and `SetNamePlateRefresh(1)`.

## Status query triggers

Eleven calls of the sweep `0x5eb070` ask the server for every quest giver's status again. They run
on the quest update message 0x197, a quest-giver dialog packet, a change of one of the
player's quest-log slots, coinage, skill info, the group list (opcode 0x7d), player flags, the
level, the item watch and the player's own death, and the player's reputation. A single unit is
queried again when its NPC flags change, its reaction may have changed, or it is created. The
status message answers units only: a game object is dropped.

Addresses: `0x5eb070`, `0x4df531`, `0x4df864`, `0x5dc6f9`, `0x5dc792`, `0x5ddddd`, `0x5ddf71`, `0x5de428`, `0x5e6f00`, `0x5e9a61`, `0x5edb85`, `0x5eeac6`, `0x607380`, `0x60b490`, `0x60b4c5`, `0x606f0a`, `0x5fba72`, `0x5fbe2f`, `0x60acff`, `0x5eb3c0`, `0x5fefa9`, `0x606eef`, `0x5dca22`, `0x5ddd80`, `0x5ddf30`, `0x5e5ea0`, `0x5e5cd6`, `0x5e5cd9`, `0x4df550`, `0x4df575`, `0x4df4f0`, `0x5e5afa`, `0x5e5cde`, `0x5e5d12`, `0x5e5d61`, `0x5e5e6b`, `0x4df880`, `0x4df4b0`, `0x562a40`, `0x5dd060`
Confidence: verified
Proof: one
Test: none

`0x5e5ad0` is registered for opcodes 0x196 to 0x19a. 0x19a is handled before the jump table
(`0x5e5afa`) and reaches `0x4df4b0`, `0x562a40` and `0x5dd060`, not the sweep. The jump table `0x5e5ea0`
sends 0x196 to `0x5e5cde`, which skips the call, and 0x197 to `0x5e5cd6`, which calls `0x4df550`
(`0x5e5cd9`) before `0x5e5cde`. 0x198 goes to `0x5e5d12`, which has no such call, and 0x199 to
`0x5e5d61`, which calls `0x4df880`, the quest log index lookup (`0x5e5e6b`). `0x4df550` looks its
argument up in the 16-byte entries at 0xbb71c0 (count 0xbb7478) and tail-jumps `0x4df4f0` (`0x4df575`),
which reaches `0x4df531`. `0x5e59b0` calls `0x5dc400` (`0x5e5a4c`), which calls the sweep at `0x5dc6f9` and
leaves `0x5dc710` to the second at `0x5dc792`. The quest log slots (`0x5dd990`, offsets 0x28 to
0x10c step 0xc) watch `0x5ddd80`; coinage (`0x5dd9bf`) watches `0x5ddf30`; skills `0x5de180`; the
group list opcode (`0x5e31d6`) `0x5e6a40`; the level (`0x60426d`) leads to `0x5edb60`; the player
flags (`0x5e25e8`) to `0x5ee990`. `0x605860` calls `0x5e99b0` for a player (`0x60588d`). The single
unit `0x607380` has seven callers, two of them the NPC-flag handler `0x60b420`; the full re-query
`0x5eb3c0` has two, the revival edge `0x5fef70` and the reaction refresh `0x606e20`. `0x5dca22`
resolves the status packet's GUID with mask 8, which a game object fails.

## Quest messages sent

| sender | opcode | body |
|---|---|---|
| `0x5eab80` | 0x186 | GUID, u32 quest |
| `0x5eac10` | 0x189 | GUID, u32 quest |
| `0x5eaca0` | 0x18a | GUID, u32 quest |
| `0x5ead30` | 0x18c | GUID, u32 quest |
| `0x5eadc0` | 0x18e | GUID, u32 quest, u32 choice |
| `0x5eaed0` | 0x19d | u32 quest |
| `0x5eaf40` | 0x194 | u8 log slot |
| `0x5eafb0` | 0x182 | GUID |

Addresses: `0x5eab80`, `0x5eac10`, `0x5eaca0`, `0x5ead30`, `0x5eadc0`, `0x5eaed0`, `0x5eaf40`, `0x5eafb0`
Confidence: verified
Proof: one
Test: none

Each builds a store with `CDataStore::PutUInt32` of the opcode, the body with `PutGuid`,
`PutUInt32` or `PutInt8`, and sends it through `ClientServices::Send`.

## Shared quests

0x19c reads a quest, a title of at most 0x400 and a GUID and offers the shared quest
(`0x5019a0`). 0x276 reads a GUID and a result and, with the player's cached name, shows the result
0 to 8 as `ERR_QUEST_PUSH_SUCCESS_S` through `ERR_QUEST_PUSH_ALREADY_DONE_S`; a name not cached
shows nothing.

Addresses: `0x5e5eb0`, `0x5e38c0`
Confidence: verified
Proof: one
Test: none

The results are errors 0x181 + r. The name lookup passes no callback.
