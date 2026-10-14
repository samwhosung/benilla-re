# ui: a quest log row

What `GetQuestLogTitle(i)` returns for a row of the quest log list (`questlog`): the title, level, tag
and the three state flags, and where each comes from. The quest template it reads is the cached
`SMSG_QUEST_QUERY_RESPONSE` record.

The code is `QuestLog.cpp`: the binding `0x4df930` and its getters `0x4df180` (title), `0x4df340` (level) and
`0x4df2a0` (tag), and the completion test `0x4df580`.

Roots: `0x4df930`, `0x4df180`, `0x4df340`, `0x4df2a0`, `0x4df270`

## The six returns

`GetQuestLogTitle(index)` takes a number, or a numeric string (anything else, a missing argument too, raises
`Usage: GetQuestLogTitle(index)`), as a 1-based index into the list, and always returns six values: the
title, the level, the tag, `isHeader`, `isCollapsed` and `isComplete`. An index outside the list gives a nil
title, level 0 and nil for the rest.

Addresses: `0x4df930`, `0x4df935`, `0x4df943`, `0x4df951`, `0x4df956`, `0x4df95d`, `0x4df960`, `0x4df970`, `0x4df98a`, `0x4df998`, `0x4df99c`, `0x4df9a9`, `0x4df9b3`, `0x4df9c3`, `0x4df9d4`, `0x4df9e5`, `0x4dfa81`, `0x4dfac1`
Confidence: verified
Proof: one
Test: none

The index is `lua_tonumber` truncated by `__ftol` and decremented (`0x4df95d`). The five pushes are the title
(`0x4df969`), the level (`fild`, `0x4df983`), the tag (`0x4df993`), `isHeader` as `1.0` (`0x4df9b3`) or nil, and
`isCollapsed` as `1.0` or nil. `isCollapsed` is 1 only for a header whose bit in `0xbb748c` is clear. The
function returns 6 on all three exits (`mov eax,6` at `0x4dfa81` and the two like it). A null string pushes nil
because `lua_pushstring` jumps to `lua_pushnil` for it.

## Title and level

For a quest row the title is the template's inline title at `+0x9c` and the level the template's `+0x08`; a
quest whose template is not cached has a nil title and level 0, and the read sends the query. For a header
the title is the AreaTable name when its id is above 0 and the QuestSort name when below 0, and for id
0 the text `Missing header! (quest designers)`; its level is 0. Only a template with a zone-or-sort value of 0 can make that text,
since a header id is always a cached template's value.

Addresses: `0x4df180`, `0x4df1c0`, `0x4df1f2`, `0x4df21e`, `0x4df25a`, `0x4df340`, `0x4df235`, `0x4df270`, `0x4df28a`, `0x4de8bc`, `0x562a40`, `0xc0e1b0`, `0xc0d9b0`, `0xc0e048`
Confidence: verified
Proof: one
Test: none

The string `Missing header! (quest designers)` has one reference, `0x4df21e`, reached at id 0 exactly (`0x4df1c0
test eax,eax; jle`, then `jge`). Reads of an uncached row pass the callback `0x4df270` to `0x562a40`; it fires
`QUEST_LOG_UPDATE` when the response arrives (`mov ecx,0x134`, `0x4df28a`), which is how a row fills in on
the second pass. The header names come from the group sort (`questlog#the-order-of-the-rows`).

## The tag

The third return is the localized name from `QuestInfo.dbc` of the row whose id is the template's fifth
field (`+0x10`, the quest type), or nil: for a header, a cache miss, a type of 0 or below, a type past the
table's maximum id, or an id with no row. The table is a sparse array indexed by id. The stock
quest log shows it in parentheses. The type is also compared with 1 by the quest accept line, which fires
tutorial flag 0x28 once.

Addresses: `0x4df2a0`, `0x4df2bb`, `0x4df2c3`, `0x4df2c8`, `0x4df2e6`, `0x4df2f9`, `0x4df302`, `0x4df309`, `0x4df30f`, `0x4df311`, `0x4df317`, `0x4df31a`, `0x4df324`, `0x4df98a`, `0x5dde6b`, `0x5dde7d`, `0x5ddf02`, `0x5ddf14`, `0x4b5390`, `0xc0d9c4`, `0x54bf20`, `0x57f700`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

`mov eax,[eax+0x10]` (`0x4df302`) is bounded by `[0xc0d9c4 + 0xc]` (`0x4df309`) and indexes the id map at
`[0xc0d9c4 + 8]` (`0x4df311`, `0x4df317`); `0x4df31a` rejects a hole; the name is `[row + 4 + 4*locale]`
(`0x4df324`). The loader `0x54bf20` asserts 10 fields and size 40 and builds the map by field 0. The record read
`0x57f700` stores the id, eight name pointers and a locale mask nothing reads. `0x4df2a0` has one caller,
`0x4df98a`. The shipped file has seven rows (Elite 1, Life 21, PvP 41, Raid 62, Dungeon 81, World Event 82,
Legendary 83), so its id map is 84 slots with 77 empty.

## Complete, failed and neither

The sixth return is -1 for a failed quest, 1 for a quest that can be turned in, and nil otherwise. A header
is always nil. A quest is failed when its slot's state byte has bit 1 set or its expiry latch is set; it
is complete when the turn-in test `0x4df580` passes.

Addresses: `0x4dfa03`, `0x4dfa08`, `0x4dfa19`, `0x4dfa20`, `0x4dfa37`, `0x4dfa41`, `0x4dfa49`, `0x4dfa50`, `0x4dfa61`, `0x4dfa69`, `0x4dfa72`, `0x4dfa8d`, `0x4dfa96`, `0x4dfaaf`, `0x4df480`, `0x4df580`
Confidence: verified
Proof: one
Test: none

The code resolves the active player (`0x468550`, `0x468460` with mask 0x10) and reads the row's slot
(`0x4dfa41`) against the 0x14 bound; `test byte [eax+7],2` (`0x4dfa61`) gives failed, `0x4df480(row)` tests the
expiry latch (`questtimers`) and `0x4df580` the turn-in predicate (`questobjectives#the-turn-in-test`). No player
object leaves the value nil.
