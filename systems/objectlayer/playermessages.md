# objectlayer: player messages

The 130 server messages the player code registers and the small ones it handles itself: notices,
level-up, GM tickets, mirror timers, play time, the dice roll, resurrection and summoning. The
messages of quests, party and guild, inventory, interaction and combat have their own topics. A
handler takes `ecx` its context, `edx` the opcode, then the time and the `CDataStore`, and returns
1 when it read the message.

Roots: `0x5e3010`, `0x5e38c0`, `0x5e7830`, `0x5e78f0`, `0x5e7990`, `0x5e7bc0`, `0x5e7d90`

## Message handlers

`0x5e3010` registers 130 handlers at world entry and `0x5e8f20` clears them at exit. This topic
has `0x5e38c0` (43 notices, 0x1b to 0x2fd), `0x5e6140`, `0x5e7830` to `0x5e7dd0` and `0x5e8f10`
(0x252); the others:

| handler | opcodes | topic |
|---|---|---|
| `0x5e59b0` | 9 quest-giver messages, 0x183 to 0x195 | `quests` |
| `0x5e5ad0` | quest updates 0x196 to 0x19a | `quests` |
| `0x5e5eb0` | 0x19c, quest confirm | `quests` |
| `0x5e5910` | vendor 0x19f, 0x1a1, 0x1a4, 0x1a5 | `inventory` |
| `0x5e5f10` | trainer 0x1b1, 0x1b4 | `inventory` |
| `0x5e6010` | loot 0x160 to 0x165, rolls 0x29e to 0x2a4 | `inventory` |
| `0x5e4f30` | item and enchant timers 0x1ea, 0x1eb | `inventory` |
| `0x5e5050` | petitions, 6 opcodes | `party` |
| `0x5e5110` | party member stats 0x7e, 0x2f2 | `party` |
| `0x5e66b0` to `0x5e7520` | emote, group and guild, one each | `party` |
| `0x5e61c0` to `0x5e6680` | spells and action buttons | `playercombat` |
| `0x5e7e00` to `0x5e8d30` | the spell combat log, 11 opcodes | `playercombat` |

Addresses: `0x5e3010`, `0x5e8f20`, `0x5e942f`, `0x5ab650`
Confidence: verified
Proof: one
Test: none

`0x5e3010` pairs each `mov ecx,opcode` with `mov edx,handler` before `0x5ab650`; the counts per
handler add to 130. `0x5e8f20` passes each opcode to the clearing call, the last (0x328) through
the tail `jmp` at `0x5e942f`.

## The notice handler

`0x5e38c0` handles 43 opcodes in a tree of comparisons and six indexed switch tables. Each case
returns 1 and an unknown opcode returns 0. One case, for 0x32d, is dead: no code registers 0x32d.

| range | bytes | cases | bound |
|---|---|---|---|
| 0x1b to 0x113 | `0x5e4b10` | `0x5e4af8` | value − 0x1b, `cmp 0xf8` |
| 0x155 to 0x16e | `0x5e4c24` | `0x5e4c0c` | value − 0x155, `cmp 0x19` |
| 0x178 to 0x1c9 | `0x5e4c6c` | `0x5e4c40` | value − 0x178, `cmp 0x51` |
| 0x1d5 to 0x219 | `0x5e4ce8` | `0x5e4cc0` | value − 0x1d5, `cmp 0x44` |
| 0x21e to 0x2a9 | `0x5e4d5c` | `0x5e4d44` | value − 0x21e, `cmp 0x8b` |
| 0x2eb to 0x32d | `0x5e4e20` | `0x5e4e0c` | value − 0x2eb, `cmp 0x42` |

Addresses: `0x5e38c0`, `0x5e4b10`, `0x5e4af8`, `0x5e4c24`, `0x5e4c0c`, `0x5e4c6c`, `0x5e4c40`, `0x5e4ce8`, `0x5e4cc0`, `0x5e4d5c`, `0x5e4d44`, `0x5e4e20`, `0x5e4e0c`, `0x5e4ae8`, `0x5e49d1`
Confidence: verified
Proof: one
Test: none

The tree splits at 0x1d4, 0x16f, 0x137, 0x21b and 0x2aa, which it handles directly; the default
`0x5e4ae8` returns 0. The case `0x5e49d1` reads a string and shows `ERR_CHAT_PLAYER_AMBIGUOUS_S`
(0x1cb). Census: no `mov ecx,0x32d` anywhere in the listing; the control 0x2fd has two, its
registration and its clearing.

## Notices

| opcode | reads | does |
|---|---|---|
| 0x1b | u32, u32 | nothing |
| 0x23 | u8 | console "Godmode enabled" or "Godmode disabled" |
| 0xfa | i32 cinematic | plays it, or keeps it in `0xc4d75c` until arrival |
| 0x113 | GUID | opens the container window (`0x4f9410`) |
| 0x137 | u8 slot, i32 ms | sets the aura slot's duration (`0x4e4390`) |
| 0x157 | i32, i32 | nothing |
| 0x1d5 | GUID, f32 x, f32 y | the minimap ping (`0x4ee330`) |

Addresses: `0x5e38c0`, `0xc4d75c`
Confidence: verified
Proof: one
Test: none

Each row is one case of `0x5e38c0`, read from its reads and calls in order. The cinematic plays
through `0x48ecd0` when the connection's `+0x1b18` is set; `0x5efc90` returns the pending one.

## Error notices

| opcode | reads | does |
|---|---|---|
| 0x178, 0x1b6, 0x1c8, 0x1c9, 0x219, 0x2fd | none | errors 0xf7, 0x137, 0x13e, 0x13f, 0xf0, 0x1c3 |
| 0x1af | none | `ERR_NEWTAXIPATH` and `0x4e1fa0` |
| 0x203 | u16 spell | unlearns it for the active player |
| 0x20d | none | ends far sight (`0x5ee590`) |
| 0x21e | u32 seconds | rest start `GetUnixTime() − seconds` (`0x496240`) |
| 0x286 | i32 delay, u32 reason | with delay 0, reason 1 or 2: `ERR_RAID_GROUP_ONLY` or `_FULL`; then `0x496340(delay)` |
| 0x2a9 | string, at most 0x131 | `ERR_CHAT_PLAYER_NOT_FOUND_S` |
| 0x16e, 0x16f | u32 result | the mount error for results 0 to 9, the dismount error for 0 to 2 |

Addresses: `0x5e38c0`, `0x5e9fe0`, `0x5ee590`, `0x5ec6e0`, `0x5ec700`, `0x85fea0`, `0x85fecc`, `0xb4e33c`
Confidence: verified
Proof: one
Test: none

The errors are `ERR_INVALID_PETNAME`, `ERR_DEATHBINDALREADYBOUND`, `ERR_FISH_NOT_HOOKED`,
`ERR_FISH_ESCAPED`, `ERR_CHAT_WRONG_FACTION` and `ERR_CHAT_RESTRICTED`. The unlearn is
`0x5e9fe0(spell, 0)`. The mount results go through `0x5ec6e0` and the table `0x85fea0` (11
entries, the last none), the dismount results through `0x5ec700` and `0x85fecc` (4, the last none),
for the active player. Nothing reads the rest start: the listing's other reference to `0xb4e33c`
is a store of 0.

## Inventory change failures

0x112 reads a u8 result; 0 does nothing. Otherwise it maps the result to an error
(`0x622630`), reads `i32 level` for result 1, then `u64 item1, u64 item2, u8 bag`. Result 1 shows
the error with the level; result 0x10 lets `0x5ede00` show the bag-family error; others show the
error. A nonzero `item1` clears the loot target and its animation when it is the looted item,
cancels a cast that uses it, and is released in the interface; so is a nonzero `item2`.

Addresses: `0x5e38c0`, `0x622630`, `0x5ede00`, `0x5e3a4b`, `0x5e3acf`, `0x5e3ad8`, `0x5e3aff`, `0x5e3b07`, `0x5e3b0d`, `0x5e3b15`, `0x5e3b20`
Confidence: verified
Proof: one
Test: none

The cast check compares `item1` with `GetCastItemGuidPtr` and calls `CancelCast(1, 0, 0x17)`; the
release is `0x495420`. Every nonzero result then reaches `0x5e3ad8`, which signals `LOOT_OPENED`
(`0x5e3b20`) only when the loot window's source `0xb71b48` is set (`0x5e3aff`), the active player
resolves (`0x5e3b07`) and its auto-loot byte is set (`0x5e3b0d`, `0x5e3b15`). The sweep has cleared
the byte by then (`ui/autoloot#auto-loot`), so the tail fires only while a window is open and the byte is
set: while its item records are still being fetched, or after a later interaction set the byte with the loot
target cleared (`0x5e3a84`, `0x5eb9d2`, `0x5ebad4`) and so no sweep run
(`objectlayer/interaction#the-auto-loot-byte`).

## Item push, bank and binding

0x166 reads a player GUID, three i32, a u8 bag, an i32 slot, the item, three more i32, and passes
them to `0x491a60`. 0x1b8 opens the bank for a GUID (`0x4f84d0`). 0x1ba shows
`ERR_BANKSLOT_FAILED_TOO_MANY`, `_INSUFFICIENT_FUNDS` or `_NOTBANKER` for results 0 to 2 and
nothing for 3 and above. 0x155 stores the bind point (`interaction`). 0x158 stores the bound GUID,
plays sound 0x475 and shows `ERR_DEATHBIND_SUCCESS_S` with the area's name.

Addresses: `0x5e38c0`, `0x80af14`, `0xc4d750`, `0x5ed3c0`
Confidence: verified
Proof: one
Test: none

The bank-slot errors are the table `0x80af14`. 0x158 stores the GUID in `0xc4d750` and reads a
u32 area for the `AreaTable` lookup.

## Level up and exploration

0x1d4 reads the level, health, five powers and five stats. For the active player it raises the
level tutorials and signals `PLAYER_LEVEL_UP` with nine values: level, health, the first power,
`level ≥ 10`, and the five stats; the other four powers are dropped. 0x1f8 reads an area and
experience, shows `ERR_ZONE_EXPLORED` with the area's name and `ERR_ZONE_EXPLORED_XP` when the
experience is above 0, and plays the race's discovery sound; an unknown area does nothing.

| level at least | tutorials |
|---|---|
| 3 | 0x16 |
| 4 | 0xd, 0x2 |
| 5 | 0x26 |
| 7 | 0x15, 0x25 |
| 8 | 0x20 |
| 10 | 0xc |

Addresses: `0x5e38c0`, `0x4b5390`
Confidence: verified
Proof: one
Test: none

Each tutorial goes through `0x4b5390(id, 0)`. The sound is the active player's `ChrRaces` row
`+0xc` through `SndInterfacePlaySound`.

## Dice rolls

0x1fb reads min, max, result and the roller's GUID, allocates a 12-byte block and asks the name
cache. On a hit it prints `RANDOM_ROLL_RESULT` with the name, result, min and max and frees the
block. On a miss it returns: the block stays unfilled and is never freed, and the callback it left
receives the address of the handler's own stack slot: once the name arrives it prints the three
dwords it then finds there and passes that stack address itself to `SMemFree`.

Addresses: `0x5e38c0`, `0x5e4e70`
Confidence: verified
Proof: one
Test: none

The request passes `lea [ebp-8]`, the slot holding the block pointer, as the callback's argument;
the callback `0x5e4e70` prints `[p+8]`, `[p]` and `[p+4]` and frees `p`, the stack address, after
the handler's frame is gone.

## GM tickets

| message | reads | does |
|---|---|---|
| 0x206, 0x208 | i32 r | 1 `ERR_TICKET_ALREADY_EXISTS`; 2, 4 send 0x211; 3 `ERR_TICKET_CREATE_ERROR`; 5 `ERR_TICKET_UPDATE_ERROR` |
| 0x212 | i32 s | 6: the ticket's text, category and three floats (a negative first float as 0) and two bytes, to `UPDATE_TICKET`; any other: `ERR_TICKET_DB_ERROR` for 0, then the no-ticket event |
| 0x218 | i32 | 9 `UPDATE_TICKET` with 0 and ""; else `ERR_TICKET_DB_ERROR` |
| 0x21b | i32 | `UPDATE_GM_STATUS` |
| 0x328 | i32 s | 1 sends 0x211; 2 the no-ticket event; 3 `GMSURVEY_DISPLAY` |

The no-ticket event passes seven dwords (0, "", −1, −1, −1, 0, 0) to the format `%d%s%f%f%f%d%d`,
which reads ten. The first float is `0xFFFFFFFFFFFFFFFF`, a NaN; the second `0x00000000FFFFFFFF`, a
denormal; the third takes its high half, and both `%d` their values, from past the arguments.
`0x5ef740(category, text)` sends 0x205 with the map, position, text and "Reserved for future use";
category 2 adds the 60 chat lines, their ages and the compressed text. `0x5efac0` sends 0x207;
`0x5efbb0` 0x211; `0x5efc20` 0x217.

Addresses: `0x5e38c0`, `0x5e78f0`, `0x5e7945`, `0x5e450c`, `0x704fe3`, `0x5ef740`, `0x5efac0`, `0x5efbb0`, `0x5efc20`, `0x8603e4`
Confidence: verified
Proof: one
Test: none

`0x5e7945` (0x328) and `0x5e450c` (0x212) push `0, 0, −1, −1, −1, ""`, then 0, before the format
`0x8603e4` and 0x18a. A `%f` takes 8 bytes (`fld qword` at `0x704fe3`). `0x5ef740` copies at most
0x7d0 characters, shows `ERR_TICKET_NO_TEXT` for none, and for category 2 gathers lines through
`0x49a810`, `0x49a830` and `0x49a850` into a 0x1e000-byte block, counts them, then compresses into
`n + n/1000 + 13` bytes of stack with `0x6605a0`.

## GM ticket requests

The ticket create message (0x205, `0x5ef740(category, text)`) carries a u8 category, the u32 map, three position floats, the text
and the string "Reserved for future use". For category 2 it adds a u32 line count, a u32 per line with the seconds since the line
was logged, a u32 size before compression and the compressed block as raw bytes with no length of its own; an empty log writes
size 0 and no block. The update message (0x207) is a u8 type and the text. 0x211, 0x217 and 0x21a have no body. Texts are cut to
1999 characters; an empty text shows `ERR_TICKET_NO_TEXT` and sends nothing.

Addresses: `0x5ef740`, `0x5ef83d`, `0x5ef851`, `0x5ef85a`, `0x5ef869`, `0x5ef896`, `0x5ef8a5`, `0x5ef8aa`, `0x5ef936`, `0x5ef956`, `0x5ef961`, `0x5ef97e`, `0x5ef9c4`, `0x5ef9d1`, `0x5efac0`, `0x5efb21`, `0x5efb2d`, `0x5efb3c`, `0x5efbb0`, `0x5efc20`, `0x48cfc0`, `0x4294c0`, `0x6605a0`, `0x64a5a0`
Confidence: verified
Proof: one
Test: none

`0x5ef740` ignores `ecx`. The widths come from the writers: `0x418070` one byte, `0x418190` and `0x4181f0` four, `0x4183d0` a
float, `0x418430` a string with its terminator, `0x418c10` raw bytes. `0x4294c0` is `time(NULL)` and `0x5ef956` subtracts the
line stamp from it. The log is 60 slots, the copy bound is 0x7d0 including the terminator (`0x64a5a0`), and `0x6605a0` is a zlib compress call with
the library version "1.1.3". `0x5ef978` skips the block when the text is empty. `0x5efbb0`, `0x5efc20` and `0x48cfc0` run only the
opcode write and the send. The reply 0x212 with status 6 reads the text, a u8 category, three floats and two u8 in that order.

## Mirror timers

0x1d9 starts a timer: type, value, max, scale, a paused byte and a spell; it signals
`MIRROR_TIMER_START` with the type's name, the four numbers and a label, the spell's name or the
text `<NAME>_LABEL`. Types 0 and 1 also raise tutorials 0x1a and 0x1c. 0x1da pauses (type and a
byte) and 0x1db stops (type). The names are EXHAUSTION, BREATH and FEIGNDEATH, any other type
UNKNOWN.

Addresses: `0x5e7990`, `0x5e7ae0`, `0x5e7b10`, `0x5e9b90`
Confidence: verified
Proof: one
Test: none

`0x5e7ae0(type)` returns the name and `0x5e7b10(type, spell)` the label from the spell's locale
name `+0x1e0`. Pause signals `MIRROR_TIMER_PAUSE` (`%s%d`), stop `MIRROR_TIMER_STOP` (`%s`);
`0x5e9b90` stops types 0 to 2 when the active player is released.

## Play time

0x2f5 reads u32 flags and i32 seconds and formats the minutes as `%2d:%02d` hours and minutes,
signed and truncating. Flag 0x1000 shows `ERR_APPROACHING_PARTIAL_PLAY_TIME` with the time, else
0x2000 `ERR_APPROACHING_NO_PLAY_TIME`, else bit 31 `ERR_UNHEALTHY_TIME`.

Addresses: `0x5e7830`
Confidence: verified
Proof: one
Test: none

The minutes come from `imul 0x88888889`, `sar 5` and the sign fix, then `idiv 60`; the tests are
`test ah,0x10`, `test ah,0x20` and `jns`.

## Resurrection and summoning

0x15b reads the resurrecter's GUID, a name (length and bytes in place) and two bytes, keeps the GUID
and bytes in `0xc4d760` to `0xc4d76c`, and shows the request (`0x48f3e0`) to a dead or ghost active
player, with the name or, when it is empty, the cached one; a cache miss forgets the request.
`0x5ded80(accept)` answers with 0x15c and clears the GUID. 0x252 (`0x5e8f10`) calls `0x495850`,
which signals `PLAYER_DEAD` for the active player. 0x2ab reads the summoner, a zone and a delay and
shows them (`0x4963a0`) unless the active player is dead or a ghost.

Addresses: `0x5e7bc0`, `0x5e7ca0`, `0x5ded50`, `0x5ded80`, `0x5e8f10`, `0x5e6140`, `0xc4d760`
Confidence: verified
Proof: one
Test: none

`0x5e7bc0` stores before testing for the player. `0x5ded50(name)` needs a non-empty name and
`IsDeadOrGhost`. The cache is `0xc0e228` (`DBCache::NameCache::GetRecord`) with callback
`0x5e7ca0`, which clears `0xc4d760` to `0xc4d76c` on a miss.

## Reading items

`0x5e7d90` handles 0xae and 0xaf for the active player and reads a GUID. For 0xae it calls
`0x5d9a80` on the item, then `0x4e35b0(&guid, 1)`; a missing item does neither. For 0xaf it reads
a reason byte: 0 calls `0x4e35b0(&guid, 1)`; 1 calls `0x4e35b0(&guid, 0)`, reads an i32 time in ms
and signals `ITEM_TEXT_TRANSLATION` with it in seconds; any other value signals `ITEM_TEXT_CLOSED`
when the GUID is the open item's. `0x5ec500(bag, slot)` sends 0xad with the two bytes.

Addresses: `0x5e7d90`, `0x5ec600`, `0x5ec500`, `0x5ec580`, `0xbc3fa0`
Confidence: verified
Proof: one
Test: none

`0x5ec600` compares the opcode with 0xae at `0x5ec615`; the seconds are the ms times
`s_millisecondsToSeconds`, passed as a double for `%f`. `0x5ec580` sends bag 0xff for the player
itself and the inventory index for a bag in slots 19 to 22.

## Other small handlers

| opcode | handler | does |
|---|---|---|
| 0x127 | `0x5e7b70` | stores a u32 mask at `0xc4d4a0 + 4·class` for a u8 class, unchecked against its 16 slots, and prints "Proficiency in item class %d set to %08x" to the console |
| 0x2bc | `0x5e7d40` | `0x48f400` with the byte's truth |
| 0x115 | `0x5e7d70` | reads a GUID and drops it |
| 0x14e | `0x5e7dd0` | stops the melee attack (`0x5ecac0`) |

Addresses: `0x5e7b70`, `0x5e7d40`, `0x5e7d70`, `0x5e7dd0`, `0xc4d4a0`, `0xc4d4e0`
Confidence: verified
Proof: one
Test: none

The opcodes are the registrations' in `0x5e3010`. `0x5e7b70` indexes with the byte as read;
`0x5e2030` zeroes the 16 dwords at world entry. Class 16 writes `0xc4d4e0`, the bind point's
valid flag, and higher classes the globals after it, up to class 255.
