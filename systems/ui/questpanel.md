# ui: quest panel state and the quest dialog verbs

What the quest dialog keeps when a quest giver offers, asks for or rewards a quest, and what the Lua functions that read or
act on it return. The pools of greeting rows are `ui/questgiver`, accepting and declining are `ui/questshare`, the texts
are `ui/gossip` ("Quest greetings"), and the messages that fill the panel are `objectlayer/quests` ("Details, request and
reward").

The code is `QuestFrame.cpp`, `0x500af0` to `0x5021b0`: the text setter `0x500bd0`, the reward copy `0x500ef0`, the request
copy `0x501070`, the item cache callback `0x5018c0` and the Lua functions `0x501a10` to `0x502090`.

Roots: `0x500bd0`, `0x500ef0`, `0x501070`, `0x501230`, `0x501560`, `0x5015b0`, `0x501700`, `0x5018c0`, `0x501920`

## Panel state

The dialog keeps the giver GUID (`0xbe0810`), the kind of panel (0xbe0818), the quest id (0xbe081c), the completable flag of
a request (0xbe0820), the item the player chose (`0xbe0828`), the reward money (`0xbe082c`), the money to hand over
(0xbe083c) and the acted latch (`0xbe0844`). The kind is the text kind of `0x500bd0`: 0 greeting, 1 quest details, 2 progress
and 3 reward. Setting a text records the kind, the quest id and the giver, clears the chosen item and the latch, and sets the
interaction target; the giver must resolve to a unit, a game object or an item.

Addresses: `0xbe0818`, `0xbe081c`, `0x500bd0`, `0x500c07`, `0x500c3b`, `0x500d36`, `0x500d68`, `0x500d6e`, `0x500d74`, `0x500d86`, `0x500d8c`, `0x500d91`, `0x500af0`, `0xbe0810`, `0xbe0828`, `0xbe0844`
Confidence: verified
Proof: one
Test: none

`0x500d68` stores the kind and `0x500d6e` the quest id; `0x500d74` calls `SetInteractionTarget`; `0x500d86` stores the giver GUID and `0x500d8c` and `0x500d91` clear the chosen item and the latch. `0x500c3b` tests kind 0 and clears both
pools, the counts and the six text buffers first. `0x500af0` is the full reset that also zeroes the item table.

## Item records

The panel holds six records of 0x24 bytes from 0xbdf020. A record has three triples of item id, display id and count:
the reward at `+0x00`, the choice at `+0x0c` and the required item at `+0x18`. The reward copy `0x500ef0` fills the reward
and choice triples from its arrays, sets unused entries to id 0, display 0 and count 1, stores the title (a single space for an
empty one), the money split into reward and wanted, and the reward spell, then signals QUEST_DETAIL (301) for a details panel
and QUEST_COMPLETE (303) otherwise. The request copy `0x501070` fills the required triples the same way and signals QUEST_PROGRESS (302).

Addresses: `0xbdf020`, `0xbe0820`, `0xbe082c`, `0xbe083c`, `0x500ef0`, `0x500f04`, `0x500f44`, `0x500f4e`, `0x500f71`, `0x500f76`, `0x500fa8`, `0x501003`, `0x50101a`, `0x501026`, `0x501035`, `0x50104a`, `0x50105d`, `0x501070`, `0x501095`, `0x5010d0`, `0x5010fc`, `0x50110a`, `0x501120`
Confidence: verified
Proof: one
Test: none

`0x500f04` clears the six records; `0x500f44` to `0x500f4e` copy one choice; `0x500f71` to `0x500f76` fill the rest. `0x50104a` stores the reward spell at 0xbe0830. `0x501026` stores a non-negative money in the reward cell and `0x501035` stores a negative one, negated, in the wanted cell.
`0x50110a` stores the completable flag. The third value of each triple is the count: `0x501700` returns it for all three kinds.

## Counting and describing items

`GetNumQuestRewards()` and `GetNumQuestChoices()` count the leading records whose item id is nonzero, and `GetNumQuestItems()`
counts the leading records whose required display id (not the item id) is nonzero, so a required item with display id 0 ends the
count. `GetQuestItemInfo(type, index)` takes `"reward"`, `"choice"` or `"required"` and a 1-based index and returns the name,
the icon path, the count, the quality and 1 when the player can use the item, else nil; a missing or wrongly typed argument, an unknown type and an index of 8 or more raise
`Invalid quest item in GetQuestItemInfo("type", index)` (`0x501f98`, `0x501fac`, `0x502000`). The index test is `ja 6` after the decrement (`0x501719`), so index 7 is accepted and
reads past the sixth record.

Addresses: `0x5016a0`, `0x5016c0`, `0x5016e0`, `0x501700`, `0x501719`, `0x501734`, `0x50173f`, `0x50176d`, `0x50179d`, `0x5017de`, `0x5017fa`, `0x501844`, `0x50188d`, `0x501896`, `0x501ec0`, `0x501f00`, `0x501f40`, `0x501f80`, `0x501ffb`, `0x50207a`
Confidence: verified
Proof: one
Test: none

`0x5016e4` reads `0xbdf03c`, the display cell; the other two counters read the id cells at `+0` and `+0xc`. `0x501708` to `0x50172e` preset an empty name and icon, count 1, quality 0 and usable 1 before the lookup. `0x5017f5` copies the name,
`0x5017fa` to `0x501809` give the quality as `+0x1c` when `+0x2c` is nonzero, else -1, and `0x501896` clears usable when `CanUseItem` answers 0. The icon path is built as in `ui/questlogverbs`. A missing item record keeps the preset values.

## Item arrival and links

When an item record arrives for the panel, `0x5018c0` signals QUEST_ITEM_UPDATE (305) provided its arrival flag is set and the
record is cached. `GetQuestItemLink(type, index)` returns the item link of the same entries, built as the quest log link, or nil
for an unknown type, an index of 8 or more, a zero id or an uncached item (the test is `ja 6` after the decrement at `0x501924`, so index 7 passes and reads past the sixth record); other arguments raise
`Usage: GetQuestItemLink("type", index)`.

Addresses: `0x5018c0`, `0x5018d6`, `0x5018de`, `0x5018f5`, `0x501903`, `0x501920`, `0x502090`, `0x5020fa`, `0x502114`, `0x502134`, `0x50214e`, `0x50217f`
Confidence: verified
Proof: one
Test: none

`0x5018d6` tests the arrival flag, `0x5018f5` looks the record up and `0x501903` signals. `0x501920` returns the item id of the named kind and index (0 for an unknown kind or `ja 6`), and `0x5020fa` calls it. `0x502134` writes the suffix name and `0x50214e` formats the link with the record quality.

## Money and the title

`GetRewardMoney()` returns the reward money cell and `GetQuestMoneyToGet()` the wanted money cell. `GetTitleText()` and
`GetObjectiveText()` return the title and the objective buffers. `GetAvailableTitle(i)` and `GetActiveTitle(i)` return nil
beyond the pool; `GetAvailableLevel(i)` and `GetActiveLevel(i)` return 0 there. The level functions raise
`Usage: GetGetAvailableLevel(index)` and `Usage: GetGetActiveLevel(index)` for a non-number. `CloseQuest()` runs `ReportQuestDeclined` with no sharer flag.

Addresses: `0x501a10`, `0x501a17`, `0x501a20`, `0x501a50`, `0x501ac0`, `0x501b05`, `0x501b30`, `0x501ba0`, `0x501bdf`, `0x501c00`, `0x501c20`, `0x501dd0`, `0x501ea0`, `0x501130`
Confidence: verified
Proof: one
Test: none

`0x501ae9` bounds the title index with `jae` after the decrement and `0x501b05` pushes a null string (nil). `0x501bdf` stores the level 0. `0x501c00` pushes the usage text. `0x501dd0` and `0x501ea0` push the two money cells with `fild`.

## Completing and choosing

`IsQuestCompletable()` returns 1 when the request is flagged completable, a player exists and the player carries each of the
six required items in the required count, else nil. `CompleteQuest()` acts only in a progress panel (kind 2) with the latch clear
and a player: it sends `CMSG_QUESTGIVER_REQUEST_REWARD` for the giver and quest, then sets the latch. `GetQuestReward([choice])`
acts only in a reward panel (kind 3): with the latch clear and a player it checks a 1-based choice against the number of
choices, stores the chosen item id, sends `CMSG_QUESTGIVER_CHOOSE_REWARD` with the 0-based choice (0 for none) and sets the latch.

Addresses: `0x501230`, `0x50127b`, `0x501280`, `0x501560`, `0x501567`, `0x50156e`, `0x50159f`, `0x5015a4`, `0x5015b0`, `0x5015ff`, `0x501608`, `0x50161a`, `0x501638`, `0x501643`, `0x501d40`, `0x501d70`, `0x501d80`, `0x501db4`, `0x5ead30`, `0x5eadc0`
Confidence: verified
Proof: one
Test: none

`0x50127b` counts each required item with scope 8 and `0x501280` compares with the required count. `0x501560` tests kind 2 (`cmp [0xbe0818],2`) and the latch. `0x5015b0` tests kind 3; with at least one choice (`0x5015ff`) a choice outside 0 to count-1 returns 0
(`0x501608`, `0x50160c`) and the verb raises `Invalid reward choice in GetQuestReward([choice])` (`0x501db4`); `0x50161a` stores the item id. A panel without choices ignores the argument. `0x501638` sends the choice, clamped at 0 from below.

## Gossip rows

`GetGossipText()` returns the gossip text buffer. `SelectGossipAvailableQuest(n)` and `SelectGossipActiveQuest(n)` take a
1-based number among the rows that are not (available) or are (active) icon 3 or 4. The active one sends
`CMSG_QUESTGIVER_COMPLETE_QUEST` for the row; the available one sends it for a row of icon 0 and
`CMSG_QUESTGIVER_QUERY_QUEST` otherwise. Neither uses the acted latch, and a number past the rows sends nothing.
`CloseGossip()` clears the gossip target.

Addresses: `0x4e28c0`, `0x4e2aa0`, `0x4e2ad7`, `0x4e2ae0`, `0x4e2b17`, `0x4e2b20`, `0x4e2b44`, `0x4e24b0`, `0x4e2516`, `0x4e253c`, `0x4e2554`, `0x4e2600`, `0x4e267f`, `0x4e1fa0`, `0x5eab80`, `0x5eaca0`
Confidence: verified
Proof: one
Test: none

`0x4e2ad7` and `0x4e2b17` decrement the number and call the scanners. `0x4e24e6` and `0x4e24ee` skip icons 3 and 4 in the available scan; `0x4e2522` tests the icon, and zero reaches `0x4e253c` (complete) while any other reaches `0x4e2554` (query). `0x4e2b44` calls `SetGossipTarget` with a zero GUID.
