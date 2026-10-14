# ui: quest objectives

The lines under a quest in the log and the watch tracker: how many there are, which kind each is, where
each count comes from, the text and the finished flag, and the test that a quest can be handed in.
The inventory walk they use is `objectlayer/inventory`; the quest list is `questlog`.

The code is `QuestLog.cpp`: `GetNumQuestLeaderBoards` `0x4e0000`, `GetQuestLogLeaderBoard` `0x4e0110`
and the turn-in test `0x4df580`.

Roots: `0x4e0000`, `0x4e0110`, `0x4df580`

## The lines of a quest

A quest has up to one `event` line, then one line for each creature or game object it requires, then one for each
item, then one reputation line, in that order. The event line is present when the template's inline text at
`+0x129c` is non-empty. A creature or object line exists for each non-zero id in `ReqCreatureOrGOId[4]` (`+0x149c`;
a negative id is a game object). An item line exists for each id above 0 in `ReqItemId[4]` (`+0x14bc`) except the
quest's own source item (`+0x34`). The reputation line exists when `+0x14` is non-zero.
`GetNumQuestLeaderBoards()` returns the number of lines as one number, 0 with no player.

Addresses: `0x4e0000`, `0x4e0042`, `0x4e00a9`, `0x4e00c0`, `0x4e00c8`, `0x4e00ce`, `0x4e00d1`, `0x4e00da`, `0x4e0102`, `0x4e0280`, `0x4e0285`, `0x4e026c`, `0x4e02d7`, `0x4e02e1`, `0x4e02ea`, `0x4e04f4`, `0x4e04fa`, `0x4e0510`, `0x4e051b`, `0x4e0527`, `0x4e053b`
Confidence: verified
Proof: one
Test: none

The creature loop counts by array index, and `inc edi` runs for every element (`0x4e02e1`), so a gap keeps
later objectives at their own index. The count function tests the faction id only for non-zero, while the
line function also needs it non-negative, at most `[0xc0dd54]` and with a row in `[0xc0dd50]` (`0x4e051b`, `0x4e0527`,
`0x4e053b`), so a faction line the table cannot resolve is counted but returns three nils.

## Asking for a line

`GetQuestLogLeaderBoard(line [, row])` needs the line number (else it raises `Usage:
GetQuestLogLeaderBoard(index)`). The optional 1-based row is a row of the quest list, which must not be a header;
without it the selected quest is used. A quest id of 0, a missing template or a missing quest slot gives
three nils. Otherwise it returns three values: the text with the counts inside it, the line's type
(`event`, `monster`, `object`, `item` or `reputation`) and `finished`, which is the number 1 or nil.

Addresses: `0x4e0110`, `0x4e0141`, `0x4e01b8`, `0x4e01dc`, `0x4e01df`, `0x4e01e9`, `0x4e01f1`, `0x4e01f6`, `0x4e01fe`, `0x4e020c`, `0x4e0214`, `0x4e0231`, `0x4e023b`, `0x4e0240`, `0x4e025b`, `0x4e0266`, `0x4e078f`, `0x4e07bc`, `0x4df150`, `0x4df050`, `0x562a40`
Confidence: verified
Proof: one
Test: none

The row path is `0x4df150(row-1)` for the id and an inline read of the row's slot (`0x4e01df`, `0x4e01e9`); a header or
out-of-range row sets the slot to -1 (`0x4e01f1`). The slot pointer is `*(player+0xe68) + 0x28 + slot*0xc` and null above
0x14 (`0x4e023b`). The function never checks that the slot's quest id equals the id used for the template.
`finished` is `push 0x3ff00000` (`1.0`) at `0x4e078f` or `lua_pushnil` at `0x4e07bc`.

## Creature and object counts

The current count of a creature or object objective is bits `(i*6)` to `(i*6+5)` of the slot's state dword, `i`
being the objective's own array index. It is not clamped by the client, and it counts only what the server
wrote. The required count is `+0x14ac[i]`. The name is the objective text in the template (`+0x14dc + i*0x100`)
when it is non-empty, with the format `QUEST_OBJECTS_FOUND`, else the creature's name from the cache with
`QUEST_MONSTERS_KILLED` (a game object always `QUEST_OBJECTS_FOUND`); the line type stays `monster` or `object`.

Addresses: `0x4e02f7`, `0x4e02fa`, `0x4e02fd`, `0x4e0300`, `0x4e0302`, `0x4e0306`, `0x4e0353`, `0x4e035e`, `0x4e0390`, `0x4e03da`, `0x4e03df`, `0x4e03e1`, `0x4e0425`, `0x4e040b`, `0x4e0491`, `0x4e04ca`, `0x4e04dc`, `0xc0e354`
Confidence: verified
Proof: one
Test: none

`current = (state >> (arrayIndex*6)) & 0x3f` is `lea ecx,[edi+edi*2]; shl ecx,1; shr eax,cl; and eax,0x3f`. A game
object id is the id masked with `0x7fffffff` (`0x4e0390`). The format key is chosen by `test cl,cl` on the first
byte of the objective text (`0x4e03da`). A name missing from the cache is the single space `" "` (`0x82ee00`, at
`0x4e0481` and `0x4e0601`), and the lookup sends the query with a callback that fires `QUEST_LOG_UPDATE` when it
arrives; it logs `Invalid creature in quest` or `Invalid object in quest` on failure.

## Item counts

The current count of an item objective is a live count of the player's items of that id, never above the
required count, and the slot's counters are not read. The count sums stack sizes over the equipped items, the
four bag slots and their contents, the backpack, the bank items and bank bags with their contents and the
keyring; it leaves out the buyback slots. So an item objective can fall when items leave.

Addresses: `0x4e0559`, `0x4e055c`, `0x4e0563`, `0x4e0569`, `0x4e0570`, `0x4e0579`, `0x4e0583`, `0x4e0592`, `0x4e059c`, `0x4e064c`, `0x622130`, `0x622420`, `0x622434`, `0x622437`, `0x622439`, `0x5dd44d`, `0x5dd459`, `0x5dd467`
Confidence: verified
Proof: one
Test: none

The call is `0x622130(player+0x1d38, ReqItemId[i], 8)` (`0x4e0579`) with the clamp `cmp eax,ecx; jge`
(`0x4e0583`). The walk widens a mask that has none of 0x47 to include them (`0x622437`, `0x622439`), so 8 becomes 0x4f:
gear, bags, backpack and keyring plus the bank (bit 8); the player constructor sets the flag that allows it
(`0x5dd44d`), and the slot count 0x71 and base `+0x4a8` (`0x5dd459`, `0x5dd467`) are the inventory fields. The
walk enters bags, adds `ITEM_FIELD_STACK_COUNT` for a matching entry, and skips broken items and items with
flag 0x10 (`objectlayer/inventory#searching-the-inventory`).

## The reputation line and finished

A reputation line is composed with `QUEST_FACTION_NEEDED` from the faction's name (`+0x4c` of its row), the
required standing label and the current one. The required value `+0x18` is bucketed on 42000, 21000, 9000, 3000,
0, -3000 and -6000; the current label is the player's standing rank. A line is finished when the current
count is at least the required count, and for the reputation line when the player's raw standing is at least
`+0x18`. The slot's complete bit decides only the event line.

Addresses: `0x4e0541`, `0x4e0544`, `0x4e069d`, `0x4e06e2`, `0x4e0722`, `0x4e0743`, `0x4e0787`, `0x4e078a`, `0x4e02a2`, `0x4e02ba`, `0x4d63a0`, `0x4d6370`, `0xc0dd50`, `0xc0dd54`
Confidence: verified
Proof: one
Test: none

The shared tail `jl 0x4e07bc` (`0x4e078d`) takes the compares `cmp [ebp-0x14],[esi+edi*4+0x14ac]` (`0x4e04dc`), the item
compare (`0x4e064c`) and `cmp edi,[esi+0x18]` (`0x4e0787`). `0x4d63a0(faction)` gives the rank and `0x4d6370(faction)` the raw value.
`test byte [eax+7],1` at `0x4e02a2` is the one read of the complete bit on this path; set gives `1.0` and a return of three values.

## The turn-in test

`0x4df580` tells whether a quest can be handed in: its slot's complete bit is set; every objective is met by the
rules above; the two reputation conditions (`+0x14` and `+0x1c`, with `+0x18` and `+0x20`) hold; and for a quest
that requires money (`+0x28` negative) the player's coinage covers it. The log shows such a quest as complete.

Addresses: `0x4df580`, `0x4df6bd`, `0x4df6c7`, `0x4df6dc`, `0x4df6e8`, `0x4df6f1`, `0x4df72e`, `0x4df74a`, `0x4df74f`, `0x4df77d`, `0x4df65b`, `0x4df778`
Confidence: verified
Proof: one
Test: none

`test byte [eax+7],1; je` is `0x4df6bd`; creature counts use `shr edi,cl; and edi,0x3f; cmp edi,[eax]`
(`0x4df74a`); items use the live count (`0x4df778`, mask 8) against `[esi+0x10]`; the money test reads
`PLAYER_FIELD_COINAGE` at `*(player+0xe68)+0xf70`. `0x4df65b` reads the template's flags `+0x38` and 0x6.
