# ui: quest objectives

The lines under a quest in the log and the watch tracker: how many there are, which kind each is, where
each count comes from, the text and the finished flag, and the test that a quest can be handed in.
The inventory walk they use is `objectlayer/inventory`; the quest list is `questlog`.

The code is `QuestLog.cpp`: `GetNumQuestLeaderBoards` `0x4e0000`, `GetQuestLogLeaderBoard` `0x4e0110`,
the cache callbacks `0x4e07d0`, `0x4e0800` and `0x4e0830`, the turn-in test `0x4df580` and the item trigger
`0x4df7b0`.

Roots: `0x4e0000`, `0x4e0110`, `0x4df580`, `0x4df7b0`, `0x4e07d0`, `0x4e0800`, `0x4e0830`

## The lines of a quest

A quest has up to one `event` line, then one line for each creature or game object it requires, then one for each
item, then one reputation line, in that order. The event line is present when the template's inline text at
`+0x129c` is non-empty. A creature or object line exists for each non-zero id in `ReqCreatureOrGOId[4]` (`+0x149c`;
a negative id is a game object). An item line exists for each id above 0 in `ReqItemId[4]` (`+0x14bc`) except the
quest's own source item (`+0x34`). The reputation line exists when `+0x14` is non-zero.
`GetNumQuestLeaderBoards([row])` returns the number of lines of the quest in the optional row, else of the selected quest, as one number, 0 with no player.

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

## Cache query callbacks

`GetQuestLogLeaderBoard` passes `0x4e07d0` with a creature lookup, `0x4e0800` with a game object
lookup and `0x4e0830` with an item lookup; `GetQuestLogRewardInfo` and `GetQuestLogChoiceInfo` pass
`0x4e0830` too. A cache calls each with its status: on 1 (the record arrived) it signals event 308,
`QUEST_LOG_UPDATE`; on 0 (refused) the creature and object callbacks write `Invalid creature in quest`
or `Invalid object in quest` to the console in class 0, and the item callback does nothing. Only the
status's low byte is read.

Addresses: `0x4e07d0`, `0x4e0800`, `0x4e0830`, `0x4e0836`, `0x4e0838`, `0x4e083a`, `0x4e044c`, `0x4e0461`, `0x4e0387`, `0x4e039c`, `0x4e05d1`, `0x4e05e0`, `0x4e0b9e`, `0x4e0d8e`, `0x556aa0`, `0x558560`, `0x55ba30`, `0x556e20`, `0x556f91`, `0x556f05`, `0x5588e0`, `0x558a63`, `0x5589cf`, `0x55bdb0`, `0x55bf39`, `0x55bea8`
Confidence: verified
Proof: one
Test: none

The pushes `0x4e044c` and `0x4e0387` pass the creature and object callbacks to `0x556aa0` on `s_creatureCache`
(`0x4e0461`) and to `0x558560` on `s_gameObjectCache` (`0x4e039c`); `0x4e05d1` passes the item callback to
`0x55ba30` on `s_itemCache` (`0x4e05e0`), and `0x4e0b9e` and `0x4e0d8e` do the same. A scan of the image for
every little-endian copy of `0x4e0830` finds those three pushes alone; the same scan finds `0x4e07d0` once,
at `0x4e044c`. Each cache's receive pushes the status, then the data: the creature cache's `0x556e20` at
`0x556f91` (1) and `0x556f05` (0), the game object cache's `0x5588e0` at `0x558a63` (1) and `0x5589cf` (0),
the item cache's `0x55bdb0` at `0x55bf39` (1) and `0x55bea8` (0). So the status is `[ebp+0xc]`; each callback
tests its low byte (`0x4e0836` in the item callback, whose `je 0x4e0844` at `0x4e0838` skips the signal at
`0x4e083a`) and returns with `ret 8`. Event 308 is `mov ecx,0x134` before `FrameScript_SignalEvent`.

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

`0x4df580(row)` returns 1 when a row's quest can be handed in. A bad or header row,
no player, an uncached template, no slot, a failed slot or a set expiry latch fail. When the template's
flags `+0x38 & 6` are non-zero, the slot's complete bit must be set; with them zero and no reputation id at `+0x14` or
`+0x1c`, a quest whose `ReqCreatureOrGOId` and `ReqItemId` are all 0 is never complete. Then standing with
`+0x14` must be at least `+0x18`, with `+0x1c` at most `+0x20`; a negative `+0x28` needs
`-[+0x28]` coinage; each creature or object count and the live count of each non-zero `ReqItemId`, the source item
included, must reach its required count. A pass fires a tutorial (below).

Addresses: `0x4df580`, `0x4df5a4`, `0x4df5b7`, `0x4df628`, `0x4df642`, `0x4df64e`, `0x4df65b`, `0x4df661`, `0x4df668`, `0x4df66a`, `0x4df671`, `0x4df690`, `0x4df6a3`, `0x4df6b0`, `0x4df6b6`, `0x4df6bd`, `0x4df6c7`, `0x4df6cc`, `0x4df6d6`, `0x4df6e1`, `0x4df6eb`, `0x4df70b`, `0x4df72e`, `0x4df74a`, `0x4df778`, `0x4df780`, `0x4df792`, `0x4df270`, `0x4df480`, `0x4b5390`, `0x4d6370`, `0x622130`
Confidence: verified
Proof: one
Test: none

The row checks are `0x4df5a4` and `0x4df5b7`; the template comes from `0x562a40`, which on a miss queries it with the callback `0x4df270`
(`0x4df628`); failed is `test byte [eax+7],2` (`0x4df642`) and the latch `0x4df480` (`0x4df64e`), both ahead of
the flags. `0x4df65b` loads `+0x38` and `and edi,6`; non-zero jumps over the scan to `0x4df6b6`, where
`test edi,edi; je 0x4df6c7` skips the complete-bit test `test byte [eax+7],1` (`0x4df6bd`) for zero. After the reputation ids (`0x4df668`, `0x4df66a`), the scan
(`0x4df671` to `0x4df6b0`) sets `edx` to 1 for a non-zero creature id (`0x4df690`), leaves for `0x4df6b6` on a
non-zero item id (`0x4df6a3`) and with `edx` 0 returns 0. The standings, each read only for a non-zero id (`0x4df6cc`, `0x4df6e1`), are `0x4d6370` with signed `jl` (`0x4df6d6`)
and `jg` (`0x4df6eb`). Coinage is `PLAYER_FIELD_COINAGE` at `*(player+0xe68)+0xf70` (0 for an object that is
not the active player, `0x4df70b`), compared unsigned (`0x4df72e`). Creatures use `shr edi,cl; and edi,0x3f;
cmp edi,[eax]` with `jb` (`0x4df74a`); items `0x622130(player+0x1d38, id, 8)` (`0x4df778`) against `[esi+0x10]`
with `jl` (`0x4df780`), with no source-item test. The pass leaves through `0x4df78b` (below).

## The tutorial on a pass

On a pass, before it returns 1, the turn-in test fires tutorial 0x21, which the interface sees as 34:
`TUTORIAL_TITLE34`, "Quest Completion" in the stock strings. So each caller fires it on a quest that can be handed
in, once per fire-once set: `GetQuestLogTitle`'s isComplete (`questtitle#complete-failed-and-neither`) and the
latch `0x4df4f0`, which the expired branch of `GetQuestTimers`, the log rebuild and the 0x197 handler reach
(`questtimers#where-the-two-lists-split`).

Addresses: `0x4df78b`, `0x4df78d`, `0x4df792`, `0x4df797`, `0x4b5390`, `0x4dfa8d`, `0x4df4fc`, `0x4e14f1`, `0x4de6ca`, `0x4df575`
Confidence: verified
Proof: one
Test: none

`xor edx,edx` (`0x4df78b`) and `mov ecx,0x21` (`0x4df78d`) set up `0x4b5390(0x21, delay 0)` (`0x4df792`), which does
nothing when tutorials are off or 0x21 already fired (`tutorials#the-tutorial-trigger`); then `mov eax,1`
(`0x4df797`). The event carries 34, and the stock `TutorialFrame` reads `TUTORIAL_TITLE34` for the alert's tooltip and
the frame's title, and `TUTORIAL34` for its text. The image holds two `call 0x4df580`, at `0x4dfa8d` and in `0x4df4f0` (`0x4df4fc`), no
`jmp` to it and no copy of its address; `0x4df4f0` is called at `0x4e14f1` and `0x4de6ca` and reached by the
`jmp` at `0x4df575`.

## Items a quest needs

`0x4df7b0(entry)` signals one `QUEST_LOG_UPDATE` when a quest in the display list needs an item entry. It
walks every row in order, headers included, looks the row's id up in the quest cache without a callback, so an
uncached quest is skipped and no query is sent, and compares the entry with the four `ReqItemId` (`+0x14bc`).
At the first match it signals event 308, calls `0x5eb070` when there is a player, and returns. Three item paths
call it with the item's `OBJECT_FIELD_ENTRY`, each for an item the active player owns: the record's arrival
`0x5d8440`, the stack count watcher `0x5d9400` and deactivation `0x5d8660`.

Addresses: `0x4df7b0`, `0x4df7da`, `0x4df7f0`, `0x4df7f4`, `0x4df805`, `0x4df818`, `0x4df830`, `0x4df83b`, `0x4df840`, `0x4df864`, `0x5d85d9`, `0x5d9519`, `0x5d87b2`, `0x5d8506`, `0x5d8521`, `0x5d944b`, `0x5d877f`, `0x5eb070`, `0x562a40`
Confidence: verified
Proof: one
Test: none

The walk is `0x4df7f0` to `0x4df830` over `0xbb71c0`, stride 0x10, against the count `0xbb7478`; it reads no
header flag, so a header's zone or sort id is looked up as a quest id. The lookup `0x562a40` (`0x4df805`) gets a
null callback (`0x4df7f4`), and a miss without one returns null and sends nothing (`dbcache#asking-for-a-record`).
`cmp [eax],edx` at `0x4df818` compares `ReqItemId[i]` with the entry, so an entry of 0 matches any cached quest
with an empty item slot. A match leaves the walk for `0x4df83b`: `mov ecx,0x134` and `FrameScript_SignalEvent`
(`0x4df840`), then `0x468460` with mask 0x10 and `0x5eb070` (`0x4df864`). The callers `0x5d85d9`, `0x5d9519` and
`0x5d87b2` load `ecx` from `[descriptor+0xc]`, the entry, behind the owner compares with the active player's
GUID (`0x5d8521`, `0x5d944b`, `0x5d877f`); the arrival also needs the byte at 0xb4b424 (`0x5d8506`). The image
holds three `call 0x4df7b0`, no `jmp` to it and no copy of its address; the same scan finds `0x4e07d0` once.
