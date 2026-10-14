# ui: the quest log list

How the quest log window's rows are built from the player's twenty quest slots: the slot layout, the
display list and its rebuild, the order, collapse and selection, and what a missing quest template does to
them. The row getters are `questtitle`, the timers `questtimers`, objectives `questobjectives`.

The code is `QuestLog.cpp`, `0x4de390` to `0x4e1a80`: the rebuild `0x4de510`, its template arrival
callback `0x4de8b0`, the sort `0x4dea50` with the row comparator `0x4deac0` and the group comparator
`0x4de8f0`, collapse `0x4ded30` and the Lua pair table of the quest log.

Roots: `0x4de510`, `0x4de8b0`, `0x4dea50`, `0x4de8f0`, `0x4deac0`, `0x4ded30`, `0x4de430`, `0x4de4d0`

## Quest slots

The player has 20 quest slots, 12 bytes each, in the player block at `[player+0xe68] + 0x28`
(`PLAYER_QUEST_LOG_1_1`): the quest id, then a dword of state, then the timer. In the state dword four 6-bit
counters fill bits 0 to 23 and the top byte holds the state: bit 0 complete, bit 1 failed. A slot with
id 0 is empty. Every reader checks the slot number against 0x14 with `ja`, which admits 20, but the
rebuild writes only 0 to 19.

Addresses: `0x4de620`, `0x4de63c`, `0x4de642`, `0x4de729`, `0x4de73e`, `0x4dfa50`, `0x4dfa61`, `0x4e08d8`, `0x4e147b`, `0x4e16a6`, `0x4e02a2`, `0x4e149f`
Confidence: verified
Proof: one
Test: none

Every reader computes `*(player+0xe68) + 0x28 + 12*slot` (`0x4dfa50`, `0x4e08d8`, `0x4e147b`, `0x4e16a6`). The
state byte is `[slot+7]`: `test byte [eax+7],1` is complete (`0x4e02a2`), `test byte [eax+7],2` failed
(`0x4dfa61`, `0x4e149f`). The rebuild's loop steps the byte offset by 12 and ends at `cmp eax,0xf0`
(`0x4de73e`), so it visits exactly 20 slots.

## The display list

The rows the window shows are the display list at `0xbb71c0`: 40 entries of 16 bytes, count `0xbb7478`.
An entry holds `+0x00` the quest id (or, for a header, the zone or sort id), `+0x04` the quest slot (0 for a
header), `+0x08` 1 for a header row and `+0x0c` the expiry latch. Headers are real rows and are counted. The
Lua getters take a 1-based index into this list, in display order, which is not slot order.

Addresses: `0xbb71c0`, `0xbb7478`, `0xbb7040`, `0xbb747c`, `0xbb748c`, `0xbb7490`, `0x4de5dd`, `0x4de5e9`, `0x4de5f8`, `0x4de5fd`, `0x4de608`, `0x4de68e`, `0x4de698`, `0x4de719`, `0x4df150`
Confidence: verified
Proof: one
Test: none

The rebuild clears 160 dwords at `0xbb71c0` (`rep stosd`, `0x4de5dd`) and 20 dwords at `0xbb7040`, the array of
header ids, and sets the row and header counts to 0 and the expanded mask `0xbb748c` to all ones (all headers
expanded). 40 rows suffice for 20 quests and 20 groups. `0x4df150(row)` returns the quest id of an in-range
row whose header word is 0, else 0. Only `0x4de719` writes `+0x08`.

## The rebuild

`0x4de510(full)` rebuilds the list. It is called with 1 from the world-enter set-up `0x4de430`, from the
watcher of the player's quest slots `0x4de4d0` and from the template callback; with 0 by the faction
standing message, which then only fires `QUEST_LOG_UPDATE` (id 308). A full call returns at once while the
in-flight counter `0xbb7498` is non-zero. Otherwise it saves the collapse state, clears the list and
visits each non-empty slot. A quest with a cached template gets a row, and a header row after it when its
group has none yet (the group is the template's zone-or-sort value). A quest without one gets no row and
adds 1 to the counter.

Addresses: `0x4de510`, `0x4de529`, `0x4de52d`, `0x4de534`, `0x4de545`, `0x4de603`, `0x4de64a`, `0x4de65f`, `0x4de66b`, `0x4de679`, `0x4de67e`, `0x4de6de`, `0x4de6e1`, `0x4de6f6`, `0x4de709`, `0x4de723`, `0x4de498`, `0x4de4d5`, `0x4d58f3`, `0x4de8e2`, `0xbb7498`, `0xc0e1b0`, `0x562a40`, `0x703e50`
Confidence: verified
Proof: one
Test: none

The miss branch `0x4de67e` jumps to the loop's own increment block (`0x4de729`), the same target as the
branch after a row is appended, so later slots are still visited. `0xbb7498` has seven references: zeroed at
`0x4de492` and `0x4de603`, read at `0x4de545`, `0x4de66d` and `0x4de8ca`, incremented at `0x4de679`, decremented at
`0x4de8d4`. A header's id is `[template+0xc]` (`0x4de6de`), found by linear search in `0xbb7040` (`0x4de6e1`) or
appended (`0x4de6f6`). The template is fetched with `0x562a40(0xc0e1b0, id, &out, 0x4de8b0, 0, 0)`, so a miss
sends one `CMSG_QUEST_QUERY` (opcode 0x5c) for that id and no more.

## When the templates arrive

The callback `0x4de8b0` runs for each queried template. On success it decrements the counter and, when the
counter reaches 0, runs the full rebuild; it also rebuilds when called with the counter already 0. On
failure (the server answered with an empty title) it only logs `Invalid quest log entry` to the console and
does not decrement, so the counter stays above 0 and every later rebuild returns at once until the world is
entered again. With a cold cache the log therefore stays empty and then fills in one step; with a partial
cache it holds the cached quests meanwhile.

Addresses: `0x4de8b0`, `0x4de8b3`, `0x4de8b8`, `0x4de8bc`, `0x4de8c7`, `0x4de8ca`, `0x4de8d1`, `0x4de8d4`, `0x4de8db`, `0x4de8dd`, `0x4de8e2`, `0x555300`, `0x555410`, `0x555417`, `0x555448`, `0x563150`, `0x562c80`, `0x555078`
Confidence: verified
Proof: one
Test: none

The query response handler `0x555300` (registered at `0x555078` for opcode 0x5d) tests the first byte of the title
(`0x555410`) and takes `DBCache::Fail` (`0x563150`, callbacks with 0) when it is empty, else `Set` (`0x562c80`).
The failure branch of `0x4de8b0` returns at `0x4de8c7` before the decrement; the success branch's `jle 0x4de8db` falls to the
rebuild when the counter was already 0. No timer or resend exists.

## The collapse state

Each header has a bit in the expanded mask `0xbb748c`, by its position in the header id array. Before a
rebuild the old ids are snapshotted, an id kept as it is when its bit is clear (collapsed) and negated when
set; after the new array is sorted, a new id equal to a snapshot entry has its bit cleared, so collapsed
state follows the id and not the position. A header whose id is 0 is always forced collapsed, and an
expanded header with id -N matches a collapsed one with id N.

Addresses: `0x4de578`, `0x4de58b`, `0x4de5b0`, `0x4de5bc`, `0x4de5c4`, `0x4de764`, `0x4de777`, `0x4de783`, `0x4de78d`, `0x4de751`, `0x4de7a0`, `0x4de7a2`, `0x4ded30`, `0x4ded8e`, `0x4dedb0`, `0x4dedcc`
Confidence: verified
Proof: one
Test: none

The snapshot loop stores `old[i]` or `-old[i]` by `test [0xbb748c],esi` (`0x4de5bc`, `neg` at `0x4de5c4`). The
restore at `0x4de764` to `0x4de7a0` compares each snapshot entry with the new id (`0x4de777`) and ANDs the new
ordinal's bit away (`0x4de78d`). Collapse and expand (`0x4ded30`) re-run only the row sort, so group
order changes only with a full rebuild.

## The order of the rows

The group ids are sorted first, by the case-insensitive collated localized name of the zone (AreaTable name
for an id above 0, QuestSort name for an id below 0), with id 0 first and unresolved names next. Rows are then
sorted by: quests of collapsed groups last; group position; a header before its quests; level ascending; title
collated without case. The comparator never sees the quest slot, and `qsort` is not stable, so full ties
land in an unspecified order.

Addresses: `0x4de8f0`, `0x4de904`, `0x4de913`, `0x4de91f`, `0x4dea31`, `0x4dea44`, `0x4dea50`, `0x4dea63`, `0x4deac0`, `0x4deb1a`, `0x4deb41`, `0x4dec77`, `0x4dec8a`, `0x4deca0`, `0x4decb3`, `0x4decd1`, `0x4decd9`, `0x4decdc`, `0x4decfb`, `0x64a4c0`, `0x73f727`, `0xc0d9b0`, `0xc0e048`
Confidence: verified
Proof: one
Test: none

The group comparator `0x4de8f0` returns -1 for a first id of 0 (`or eax,-1`) and 1 for a second id of 0, and copies each name
into 0x100 bytes from `QuestSort` (the id map of the store `0xc0d9b0`, name at `+4 + 4*locale`) or AreaTable (`+0x2c +
4*locale`), leaving the empty string for an unresolvable id, then compares them with `0x64a4c0` (a
case-insensitive `strnicmp`, `0x4dea44`). The row comparator reads the quest template (callback 0, so no
query): key 1 `0x4dec77` (collapsed quests +1), key 2 the header ordinal (`0x4dec8a`), key 3 headers first, key 4
level `setge` with `lea eax,[eax+eax-1]` (`levelA > levelB` gives +1, so ascending), key 5 the title at
`[rec+0x9c]`. The sort is `0x73f727(0xbb71c0, count, 0x10, 0x4deac0)`.

## Visible rows

After the sort `0xbb7490` is the row count minus one for each quest whose header is collapsed, and because
those quests sort last, the first `0xbb7490` rows are the visible ones. `GetNumQuestLogEntries()` returns
the visible count and the number of quests (the rows that are not headers), and the other getters still index
all rows, so a hidden quest stays addressable.

Addresses: `0x4dea50`, `0x4dea74`, `0x4dea88`, `0x4dea8d`, `0x4dea9b`, `0x4deaaa`, `0x4dee60`, `0x4df8b0`
Confidence: verified
Proof: one
Test: none

`0x4dee60` maps a row to its header ordinal, taking `[entry]` for a header and `[template+0xc]` for a quest,
or -1; `cmp eax,0x14; jae` skips the row.

## The selection

The selected quest is a quest id in `0xbb7480`, set only by `SelectQuestLogEntry(i)` (`0x4dfae0`) through
`0x4def30`, which refuses an index outside the list and a header row and then leaves the old
selection. `GetQuestLogSelection()` re-derives the row by searching for the id (0 when it has no row). No
rebuild changes the selection, so it survives re-sorts, and with the selected quest uncached it reads 0
until the completing rebuild.

Addresses: `0xbb7480`, `0x4def30`, `0x4def40`, `0x4def48`, `0x4def55`, `0x4def5d`, `0x4dfae0`, `0x4dfb04`, `0x4dfb20`, `0x4df010`, `0x4df049`, `0x4de3b0`
Confidence: verified
Proof: one
Test: none

An image scan finds 24 references to `0xbb7480` and two writers: the reset `0x4de3b0` and `0x4def5d`. `0x4df010`
returns the row index plus one, or -1 (`or eax,-1`, `0x4df049`), and `0x4dfb20` adds 1.

## Quest watches

The quest watch list `0xbb70a0` holds up to 5 quest ids, added by `AddQuestWatch` (`0x4e1930`, through
`0x4dede0`, a sixth being a silent no-op) and removed by `RemoveQuestWatch` (`0x4e1970`). Each rebuild
removes every watched id that has no non-header row in the new list, so a rebuild with uncached quests
drops their watches and the completing rebuild does not restore them.

Addresses: `0xbb70a0`, `0xbb7488`, `0x4de7a7`, `0x4de7c4`, `0x4de7fc`, `0x4de80f`, `0x4dede0`, `0x4dede5`, `0x4dedea`, `0x4dee00`, `0x4dee38`, `0x4dee46`, `0x4e1930`, `0x4e1968`, `0x4e1970`, `0x4e19a8`
Confidence: verified
Proof: one
Test: none

`0x4dedea` is the only append and `0x4dede5` is `cmp eax,5; jae`. The prune loop `0x4de7a7` to `0x4de80f` calls `0x4dee00`
(a move down, a count decrement and a zeroed tail, `0x4dee38`, `0x4dee46`) and re-examines the shifted index. The
rebuild ends by firing `QUEST_LOG_UPDATE` (`mov ecx,0x134`, `0x4de811`), whatever path reached the tail.
