# ui: the guild roster and its Lua API

The guild's member list, its ranks and rights, and the 32 Lua functions that read and change them, in
`GuildInfo.cpp`. The roster arrives whole in one message, is kept sorted by a chain of eight sort keys, and is
read by index. The rank names, the emblem and the guild's own record live in the guild cache; the guild
notices, events and the tabard are `objectlayer/party`. The function names and argument counts are
`tables/bindings.tsv`.

Roots: `0x4d0a00`, `0x4d0a70`, `0x4d0ad0`, `0x4d1160`, `0x4c9330`

## State and start-up

`0x4d0a00` registers the roster handler for opcode 0x8a (`0x4d0a0e`) and zeroes the counts, the selection and the
throttle; it sets the staged rank to -1 and seeds the eight-slot sort chain so that slot `i` holds key `i`,
direction 0. `0x4d0a70` clears the handler and frees every member record. `CGGameUI::InitializeGame` calls the first
(`0x48f5ea`), `CGGameUI::ShutdownGame` the second (`0x4911f0`). The state is a count and an online count, a rank
count, a show-offline flag, the selected member's GUID, ten rank rights, the message of the day (0x200 bytes), the
guild information text (0x7d0 bytes) and the array of member pointers.

Addresses: `0x4d0a00`, `0x4d0a0e`, `0x4d0a70`, `0x4d0a72`, `0x4d0950`, `0x4d0980`, `0x48f5ea`, `0x4911f0`, `0xb72680`, `0xb726d0`, `0xb726fc`, `0xb72720`, `0xb72f00`, `0xb73118`, `0xb7311c`, `0xb73120`, `0xb73124`, `0xb73128`, `0xb73130`, `0x84966c`, `0x849670`
Confidence: verified
Proof: one
Test: none

`0xb726fc` is the array's control block: capacity, the high-water count at `0xb72700`, the data pointer at
`0xb72704` and a growth hint; `0x4d0950` zeroes it and `0x4d0980` frees the array at exit. `0xb73128` and `0xb7312c`
hold the GUID of the selected member, so the selection follows a member through every sort. `0xb73130` is the
deadline of the next roster request. The rank flag table `0x849670` is thirteen pairs of mask and staged value
(`Rank control`); `0x84966c` is the staged rank, -1 for none. The member records are allocated once per slot and
reused: `0x4d0a70` frees the records and sets the high-water count to 0, so the next roster allocates every record anew.

## The roster message

Opcode 0x8a, `0x4d0ad0`: `u32 memberCount`, `cstr motd` (cap 0x200), `cstr guildInfo` (cap 0x7d0),
`u32 rankCount`, `u32 rankRights[rankCount]`, then per member `u64 guid`, `u8 presence`, `cstr name` (cap 0x30),
`u32 rankId`, `u8 level`, `u8 classId`, `u32 zoneId`, `f32 lastOnline` only when the presence byte is 0,
`cstr publicNote` (cap 0x80), `cstr officerNote` (cap 0x80). The handler masks the message and the information
text with the profanity masker at once, leaves the notes raw, sorts the array, signals `GUILD_ROSTER_UPDATE`
without an argument and returns 1.

Addresses: `0x4d0ad0`, `0x4d0af3`, `0x4d0b65`, `0x4d0b71`, `0x4d0b82`, `0x4d0b8e`, `0x4d0b9a`, `0x4d0bb3`, `0x4d0bfd`, `0x4d0c08`, `0x4d0c43`, `0x4d0c56`, `0x4d0c61`, `0x4d0c6c`, `0x4d0ca0`, `0x4d0cad`, `0x4d0cb0`, `0x4d0cc2`, `0x4d0ce7`, `0x4d0d02`, `0x4d0d32`, `0x4d0d3c`, `0x418cb0`, `0x4190b0`, `0x418e30`, `0x418eb0`, `0x419130`, `0x4191b0`
Confidence: verified
Proof: one
Test: none

The float is read only on the `je` leg of `cmp [rec+0x48],edi` (`0x4d0cad`, `edi` is 0 throughout): the online word
is `presence != 0` (`0x4d0c12`, `0x4d0c1d`), so a presence byte of 0 reads the float and any other value
reads nothing and stores 0 (`0x4d0cb2`). A reader that always takes the float loses sync for every later
member. Its type is fixed by its readers, `fld` at `0x4d0f19` and `0x4d1508`. Level and class are single bytes
widened to dwords. The member count is stored as sent (`0xb73118`); the array grows to it (`0x4d0b01` to `0x4d0b57`,
0x158 bytes per new slot, never shrunk), so a large count allocates that many records. The rank rights loop
(`0x4d0bb0` to `0x4d0bc3`) runs `rankCount` times from the 10-entry array `0xb726d0`; the test for 10 at `0x4d0bc5`
only guards the zero-fill of the unused entries. A count of 11 overwrites the float after the array and a
count of 12 or more overwrites the array's control block. The online tally `0xb7311c` is zeroed at `0x4d0ae9`
and incremented per online member (`0x4d0d14`); `0xb73120` is zeroed at `0x4d0aee`.

## The member record

A `GuildMemberInfo` is 0x158 bytes: GUID at `+0`, name at `+8` (0x30), level `+0x38`, class `+0x3c`, zone `+0x40`,
rank `+0x44`, the online word `+0x48`, the raw presence byte `+0x4c`, a notes-unfiltered byte `+0x4d`, the public
note `+0x4e`, the officer note `+0xce` and the last-online time `+0x150`, a float in days. The presence bit 0x2
is away and 0x4 is do-not-disturb; the client tests the whole byte for online and names no bit 0x1.

Addresses: `0x4d0b3c`, `0x4d0c22`, `0x4d0c31`, `0x4d0c7e`, `0x4d0c8e`, `0x4d0cd0`, `0x4d09b0`, `0x4d09c0`, `0x4d09d0`, `0x4a1a60`, `0x4d13e1`, `0x4d13fb`
Confidence: verified
Proof: one
Test: none

The allocation passes the tag string `.PAUGuildMemberInfo@@` and the size 0x158 (`0x4d0b3c`). `0x4d0cd0` marks the
record's notes unfiltered. The accessors `0x4d09b0` (public) and `0x4d09c0` (officer) first run `0x4d09d0`, which
masks both notes in place and clears the mark, then return the address; the notes are filtered once on first read.
`GetGuildRosterInfo` tests the presence byte for the status text: bit 0x4 first (`0x4d13e4`), then 0x2
(`0x4d13fb`), so do-not-disturb wins when both are set.

## The roster order

The array is sorted after every roster message and every sort change with the C runtime `qsort`
(`0x73f727`, element width 4, comparator `0x4d0d50`), which is not stable. With show-offline off the comparator
puts online members before offline ones first. Then it walks the eight-slot chain `{key, direction}` and returns
the first nonzero result, negated when that slot's direction is 1. The keys, at direction 0: 0 rank, the higher
rank id first; 1 level, the lower first; 2 name, case-insensitive; 3 zone name and 4 class name in the client's
locale (a missing row ties); 5 group, which orders nothing; 6 online, then the smaller last-online
time first; 7 the public note. Equal members compare 0.

Addresses: `0x4d0d50`, `0x4d0d60`, `0x4d0d69`, `0x4d0d77`, `0x4d0db4`, `0x4d0f44`, `0x4d0e4e`, `0x4d0dcd`, `0x4d0dbb`, `0x4d0e67`, `0x4d0ded`, `0x4d0efd`, `0x4d0ebd`, `0x4d0edf`, `0x4d0f19`, `0x4d0f36`, `0x73f727`, `0x64a4c0`, `0xb72680`, `0xc0def4`, `0xc0e048`
Confidence: verified
Proof: one
Test: `ui::sortcmp_4d0d50`

`0x4d0d60` skips the online pre-gate when `0xb73124` is set. The jump table `0x4d0f44` holds the eight keys; key 5
points at `0x4d0efd`, the loop's own continue. The rank compare `cmp ecx,eax` / `sbb eax,eax` returns -1 when
the second member's rank id is lower, so rank 0 sorts last at direction 0. Key 6 for two offline members compares
the floats with `fcomp` and `test ah,5` (`0x4d0f19` to `0x4d0f31`): a smaller first value gives -1; larger, equal and
unordered values all give +1, so equal members never compare 0 there. The zone name is
`[AreaTable row + 0x2c + 4*locale]` (`0x4d0eac`) and the class name `[ChrClasses row + 0x14 + 4*locale]`
(`0x4d0e3a`); an id that is negative or past the store's maximum gives no row. The notes are read through
`0x4d09b0`.

Bit-exact coverage: `ui::sortcmp_4d0d50` covers 6 ranges from `0x4d0d50` to `0x4d0f43` of `0x4d0d50`.

## Changing the order

`SortGuildRoster(field)` takes one of the strings `rank`, `level`, `name`, `zone`, `class`, `group`, `online` or
`note`, compared without case, and an unknown string sorts by name. `0x4d0fb0(key)` finds the key in the chain.
At slot 0 its direction flips; elsewhere it keeps its remembered direction. It moves the key to slot 0, shifting
the earlier slots down, sorts, and signals `GUILD_ROSTER_UPDATE` without an argument. An argument that is neither
a string nor a number raises `Usgae: SortGuildRoster("type")`. `SetGuildRosterShowOffline([flag])` defaults to on; the worker
`0x4d0f70` does nothing for an unchanged value and otherwise stores the flag, sorts a nonempty roster and signals.

Addresses: `0x4d1cb0`, `0x4d1cd3`, `0x4d1cdb`, `0x4d1cf2`, `0x4d1dfb`, `0x4d0fb0`, `0x4d0fc3`, `0x4d0fda`, `0x4d1018`, `0x4d1022`, `0x4d0f70`, `0x4d0f9f`, `0x4d1e10`, `0x4d1e17`, `0x4d1e30`, `0xb73124`
Confidence: verified
Proof: one
Test: none

`0x4d1cdb` seeds the key register with 2 and the chain of string compares overwrites it on a match, so a string that
matches none reaches the call with 2. `0x4d0fc3` tests the found slot; the direction of a slot above 0 is read at
`0x4d0fc6` and kept. `SetGuildRosterShowOffline` reads its argument with `GetScriptFlagArg` (default 1) and
`GetGuildRosterShowOffline` returns 1 or nil.

## Reading the roster

`GetNumGuildMembers([all])` returns the full count when show-offline is on or the argument is true, else the online
count. `GetGuildRosterInfo(index)` is 1-based and returns ten values: name, rank name, rank id, level, class name,
zone name, public note, officer note, 1 or nil for online, and the status text for do-not-disturb or away or the empty
string. Any index below 1 or past the full count, a missing member, an unknown player or guild, or a guild record
not yet cached returns `nil, nil, 0, 0, nil, nil, nil, nil, nil, ""`. A missing argument raises
`Usage: GetGuildRosterInfo(index)`. The rank name is copied with no size limit (`0x7fffffff`) into the stack buffer at `ebp-0x48` (`0x4d12f3`).

Addresses: `0x4d1190`, `0x4d11a5`, `0x4d1200`, `0x4d1222`, `0x4d1291`, `0x4d12bb`, `0x4d12e2`, `0x4d1339`, `0x4d1361`, `0x4d1397`, `0x4d13c5`, `0x4d1412`, `0x4d146c`, `0x4d1480`, `0xc0e1ec`
Confidence: verified
Proof: one
Test: none

The rank name is `guildRecord + 0x64 + 0x40*rankId`, copied and masked on every call; the rank id is not
checked against 10, so a rank id past 9 reads the emblem words and what follows. The class and zone names come from the
locale columns of `ChrClasses` and `AreaTable` by the same checks as the sort, nil for a missing row. The guild
record is `0x560e30(guildId, &playerGuid, 0x4d1480)` on the cache `0xc0e1ec`: a miss sends the guild query and
returns the nil tuple, and the callback `0x4d1480` signals `GUILD_ROSTER_UPDATE` without an argument when the
record lands. The online value is the number 1 or nil. The index bound is the full count whatever the show-offline
flag: with it off the online members are the first `GetNumGuildMembers()` entries and the offline ones follow, filled in.

## Last online and selection

`GetGuildRosterLastOnline(index)` returns four numbers, years, months, days and hours, and four nils for an index
out of range, a missing member or an online member. With `t` the stored float in days: years = trunc(t * f32(1/365));
rem = t - years * 365; months = trunc(rem * f32(1/30)); r = f32(rem - months * 30); days = trunc(r);
hours = trunc((r - days) * 24). `GetGuildRosterSelection()` returns the selected member's 1-based index, or 0 when
nothing is selected or the member left; `SetGuildRosterSelection(index)` stores the member's GUID, or 0 for an
index out of range, and raises `Usage: SetGuildRosterSelection(index)` without a number.

Addresses: `0x4d14a0`, `0x4d14fd`, `0x4d1508`, `0x4d1514`, `0x4d1521`, `0x4d1529`, `0x4d153c`, `0x4d1544`, `0x4d1584`, `0x4d158a`, `0x4d1590`, `0x4d1820`, `0x4d1890`, `0x4d1030`, `0xb73128`, `0x40a2b0`, `0x80733c`, `0x807338`, `0x807334`, `0x807330`, `0x80732c`
Confidence: verified
Proof: one
Test: `ui::luabind_4d14a0_guild_last_online`

The constants are the floats 1/365 (`0x3b338cfa`), 365.0, 1/30 (`0x3d088889`), 30.0 and 24.0. Every truncation
is `__ftol`, toward zero. The remainder after the months is stored to a 4-byte slot (`0x4d1544`), which narrows it to
a float, and both the days and the hours are taken from that narrowed value; the other steps stay on the x87 stack.
`0x4d1030` searches the array for the stored GUID and returns the 0-based index or -1; the binding adds 1.

Bit-exact coverage: `ui::luabind_4d14a0_guild_last_online` covers `0x4d1508` to `0x4d15a3` of `0x4d14a0`.

## Notes and text

`GetGuildRosterMOTD()` and `GetGuildInfoText()` each return one string, empty when unset. `SetGuildInfoText(text)`
sends opcode 0x2fc with the text and changes nothing locally. `GuildRosterSetPublicNote(index, note)` and
`GuildRosterSetOfficerNote` send opcode 0x234 or 0x235 with the member's name and the new note, and only when the
new text differs from the stored (masked) note by a case-sensitive compare; the stored note is not changed.
`GuildRoster()` sends opcode 0x89 with no body at most once per 10 000 ms. `CloseGuildRoster()` does nothing.

Addresses: `0x4d11f0`, `0x4d2370`, `0x4d2380`, `0x4d23af`, `0x4d15e0`, `0x4d1668`, `0x4d1671`, `0x4d1700`, `0x4d1782`, `0x4d1791`, `0x4d2360`, `0x4d10d0`, `0x4d10df`, `0x4d2350`, `0x64a480`, `0xb72720`, `0xb72f00`, `0xb73130`
Confidence: verified
Proof: one
Test: none

`0x4d10d0` reads the millisecond clock, skips the send when the deadline word is nonzero and later than the
clock (a negative signed difference), else sends and stores clock + 0x2710. The signal with argument 1
(`Roster events`) clears the deadline. Each note setter compares the stored text from `0x4d09b0` or `0x4d09c0`
with `SStrCmp` and sends on a nonzero result; a missing record or text sends nothing. The usage errors are
`Usage: SetGuildInfoText(text)`, `Usage: GuildRosterSetPublicNote(index, note)` and
`Usage: GuildRosterSetOfficerNote(index, note)`.

## Rank rights

Nine functions take no argument and return 1 or nil for one bit of the player's rank rights. The rights are the
entry of the rank array `0xb726d0` for the player's `PLAYER_GUILDRANK`, 0 for a rank past 9. Not in a guild, or the
player missing, returns nil. There is no guild-leader shortcut: rank 0 is tested like any other. `IsInGuild()` is
1 when `PLAYER_GUILDID` is nonzero; `IsGuildLeader()` is 1 when `PLAYER_GUILDRANK` is 0.

| function | address | bit |
|---|---|---|
| `CanGuildInvite` | `0x4d19a0` | 0x10 |
| `CanGuildRemove` | `0x4d1a10` | 0x20 |
| `CanGuildPromote` | `0x4d18c0` | 0x80 |
| `CanGuildDemote` | `0x4d1930` | 0x100 |
| `CanEditMOTD` | `0x4d1a80` | 0x1000 |
| `CanEditPublicNote` | `0x4d1af0` | 0x2000 |
| `CanViewOfficerNote` | `0x4d1bd0` | 0x4000 |
| `CanEditOfficerNote` | `0x4d1b60` | 0x8000 |
| `CanEditGuildInfo` | `0x4d1c40` | 0x10000 |

Addresses: `0x4d18c0`, `0x4d1930`, `0x4d19a0`, `0x4d1a10`, `0x4d1a80`, `0x4d1af0`, `0x4d1b60`, `0x4d1bd0`, `0x4d1c40`, `0x516de0`, `0x516e40`, `0x4d18f2`, `0x4d1902`, `0x4d1972`, `0x4d19e2`, `0x4d1a52`, `0x4d1ac2`, `0x4d1b32`, `0x4d1ba2`, `0x4d1c12`, `0x4d1c82`
Confidence: verified
Proof: one
Test: none

Each function holds its own copy of the code: the active player by `ClntObjMgrGetActivePlayer` and
`ClntObjMgrObjectPtr` with the player type mask, the guild id at `[[player+0xe68]+0xc]`, the rank at `+0x10`, then
`cmp eax,0xa; jb` before the table read (`0x4d18f2`). The tests are `test al,0x10`, `test al,0x20`, `test al,al` with
`jns` for bit 0x80, `test ah,1`, `test ah,0x10`, `test ah,0x20`, `test ah,0x40`, `test ah,ah` with `jns` for bit
0x8000 and `test eax,0x10000`. Census of the 13 reads of `0xb726d0`: the nine above, `0x4d1098`, `0x4d2019`,
`0x4d215c`, the loop at `0x4d0baa` and the fill at `0x4d0bca`. The bits 0x1, 0x2, 0x4, 0x8, 0x40, 0x200, 0x400 and
0x800 are tested nowhere in the guild code.

## Rank control

Thirteen flags are indexed 1 to 13 by the table `0x849670`, in the order of the masks 0x1, 0x2, 0x4, 0x8, 0x80,
0x100, 0x10, 0x20, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000. `GuildControlSetRank(rank)` takes a 1-based rank,
stores `rank - 1` in `0x84966c` and stages each flag as its mask AND that rank's rights; when `rank - 1` is below
0 or at least 13 it stores -1 instead. It returns no value. `GuildControlGetRankFlags()` returns thirteen values,
1 or nil, from the live rights of the staged rank and not from the staged flags. `GuildControlSetRankFlag(index,
enabled)` stores the boolean in the staged slot; an index outside 1 to 13 is ignored.

Addresses: `0x4d1fa0`, `0x4d1080`, `0x4d1089`, `0x4d109f`, `0x4d10b8`, `0x4d1fe0`, `0x4d200b`, `0x4d2039`, `0x4d2070`, `0x4d20a9`, `0x4d20bc`, `0x849670`, `0x84966c`
Confidence: verified
Proof: one
Test: none

`0x4d1080` rejects an index below 0 or at 13 or more, not at the rank count. `0x4d1fe0` tests
`[0x849670 + 8*i]` against the rights and never reads the staged word at `+4`, so a `SetRankFlag` is invisible to
`GetRankFlags` until a roster message changes the rights. The roster functions' rank id is 0-based; this family takes
rank - 1, so a roster rank id of 0 passed unchanged stages -1 and clears the staging.

## Rank edits

`GuildControlGetRankName(index)` is 1-based and returns the masked rank name, or nil past the rank count.
`GuildControlGetNumRanks()` returns the rank count. `GuildControlSaveRank(name)` sends opcode 0x231 with the staged
rank, the rights and the name. The rights start from the live rights of the staged rank; each of the thirteen
flags sets its mask when staged on and clears it when off, and other bits stay. It does nothing for a staged
rank past the rank count, an empty name or a name outside 1 to 16 characters.
`GuildControlAddRank(name)` sends 0x232 with the name under the same length rule while fewer than 10 ranks exist.
`GuildControlDelRank()` reads no argument and sends 0x233 without a body while over 5 ranks exist.

Addresses: `0x4d1e90`, `0x4d1f0c`, `0x4d1f39`, `0x4d1e60`, `0x4d20d0`, `0x4d2121`, `0x4d2127`, `0x4d213f`, `0x4d2144`, `0x4d2163`, `0x4d2194`, `0x4d2210`, `0x4d2271`, `0x4d227a`, `0x4d22e0`, `0x4d22e6`, `0x4d22f2`, `0x65b250`
Confidence: verified
Proof: one
Test: none

The name length is `0x65b250(name, 0x7fffffff, 0)` minus 1, which counts the terminator. The usage errors are
`Usage: GuildControlGetRankName(index)`, `Usage: GuildControlSetRank(rank)`,
`Usage: GuildControlSetRankFlag(index, enabled)`, `Usage: GuildControlSaveRank(name)` and
`Usage: GuildControlAddRank(name)`; `GuildControlDelRank` has none. The packets are the opcode, then for 0x231 a
rank dword, a rights dword and the string.

## The unit's guild

`GetGuildInfo(unit)` returns the guild name, the rank name and the rank number of a unit that is a player with a
guild, and `nil, nil, 0` for any other unit, an unresolved token or a guild not yet cached. The unit comes from the
unit token grammar (`unittokens#the-unit-token`); a token no prefix matches raises
`Unknown unit name: %s`. A missing argument raises `Usage: GetGuildInfo("unit")`.

Addresses: `0x4c9330`, `0x4c9382`, `0x4c9394`, `0x4c93a3`, `0x4c93ca`, `0x4c93e4`, `0x4c9402`, `0x4c943c`, `0x4c9470`, `0x515940`
Confidence: verified
Proof: one
Test: none

The unit's descriptor type word is tested for the player bit (`shr ecx,4; test cl,1`), then its guild id
`[[unit+0xe68]+0xc]`; the cache is asked with that id and the unit's GUID, callback `0x4c9470`. The rank name
is `record + 0x64 + 0x40*rank`, copied and masked; the rank is not bounded by 10. The guild name is the record's
`+4`, not masked. The first call for an uncached guild answers `nil, nil, 0`. When the record arrives the callback
broadcasts `PLAYER_GUILD_UPDATE` for the unit through `0x515e50`.

## The guild cache

The guild cache `0xc0e1ec` holds one record per guild id: id, name, ten rank names and the emblem
(`objectlayer/party#the-guild-record`). It is created at start-up with tag `WGLD`, the file `guildcache.wdb` and
query opcode 0x54, and nothing asks for a record at world entry: the Lua functions above, the rank-rename event and the
name and emblem readers ask when called. Opcode 0x55 (`0x555290`) decodes a
record into a 0x2f8-byte local and stores it, or removes the cache entry when the decoded guild name is empty,
which is how the server answers an unknown guild.

Addresses: `0x554d20`, `0x554d28`, `0x554d2d`, `0x555290`, `0x5552a3`, `0x5552a8`, `0x5552c5`, `0x5552e1`, `0x62f260`, `0x560e30`, `0x561070`, `0x561390`
Confidence: verified
Proof: one
Test: none

`0x554d20` pushes `0x57474c44` (the characters `WGLD`), the file name and 0x54 into the cache constructor
`0x560460`. `0x5552a8` tests the first byte of the name at `local + 4`. The decoder reads a fixed ten rank names;
there is no count on the wire. Census: 26 instructions load `0xc0e1ec`; they belong to the three Lua functions
above, the emblem and name readers, the guild event and tabard handlers, the player teardown and field watchers, and the
cache's own routines.

## Roster events

`GUILD_ROSTER_UPDATE` is signalled by `0x4d1160(flag)`. With flag 0 it fires without an argument and means the
roster or the cache changed. With flag 1 it clears the request deadline `0xb73130` and fires with the number 1,
which means the roster is stale. The callers with 0: a roster message (`0x4d0d3c`), a show-offline change (`0x4d0f9f`),
a sort (`0x4d1022`) and a guild record arrival (`0x4d148c`). The callers with 1: every guild event of types 0 to 11
(`0x5e74c4`) and a command result of 19, 20 or result 20 on command 5 (`0x5e7792`).

Addresses: `0x4d1160`, `0x4d1164`, `0x4d1170`, `0x4d117a`, `0x4d1188`, `0x4d0d3c`, `0x4d0f9f`, `0x4d1022`, `0x4d148c`, `0x4d1480`, `0x5e74c4`, `0x5e7792`, `0x5e27b0`, `0x5e2770`, `0x5e08e0`, `0x515e50`, `0x48f450`, `0x48f470`
Confidence: verified
Proof: one
Test: none

The argument is a `%d` of 1 (`0x835154` holds the format). `PLAYER_GUILD_UPDATE` fires on three paths: a change of
a player's guild rank, with no argument (`0x5e27b0`); a change of a player's guild id, through `0x5e08e0`, which
signals `TABARD_CANSAVE_CHANGED` and broadcasts the event with one unit token per token that names the player
(`0x515e50`); and the cache callback of `GetGuildInfo`. `GUILD_INVITE_REQUEST` is signalled by `0x48f450` with
the inviter and the guild name. The signal wrapper `0x48f470` for `GUILD_INVITE_CANCEL` (`mov ecx,0x11b; jmp 0x703e50`, like `0x48f440`) has no caller and no stored
address: a census of the image finds 0 `call`, 0 `jmp` and 0 address hits for it (the control `0x48f450` has 1 `call`), so the event never fires.
