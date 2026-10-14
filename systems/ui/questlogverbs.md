# ui: quest log verbs on the selected quest

The Lua functions that read the quest log's selected quest (its text, rewards, money, failure, abandon list) and that move
headers and watches by row number. The rows, order, selection and watch list are `ui/questlog`; objective lines are
`ui/questobjectives`; timers are `ui/questtimers`. Every function here takes a 1-based row of the display list and
subtracts 1, or reads the selected quest id (`0xbb7480`) and looks it up in the quest template cache (`0xc0e1b0`) with no callback, so an uncached
template answers as a missing quest and sends no new request of its own.

Roots: `0x4df8b0`, `0x4dff20`, `0x4e0960`, `0x4e09e0`, `0x4e0a70`, `0x4e0b00`, `0x4e0cf0`, `0x4e0ee0`, `0x4e1070`, `0x4e1220`

## Template fields read

The verbs below read these fields of a quest template: the level at `+0x08`, money at `+0x28` (negative means the quest asks
for money), bonus money at `+0x2c`, four reward item ids at `+0x3c` with counts at `+0x4c`, six choice item ids at `+0x5c` with
counts at `+0x74`, the title at `+0x9c`, the objectives text at `+0x29c`, the details text at `+0xa9c` and four required item
ids at `+0x14bc`.

Addresses: `0x562a40`, `0x4dff5e`, `0x4dfba0`, `0x4e0a26`, `0x4e0a31`, `0x4e0ac1`, `0x4e0b98`, `0x4e0c26`, `0x4e0d88`, `0x4e0e16`, `0x4e10f2`, `0x4e1102`, `0x4e1268`
Confidence: verified
Proof: one
Test: none

The offsets are the displacements the verbs add to the record: `0x4dfbab` loads the title at `+0x9c`, `0x4dff7a` the details at `+0xa9c`, `0x4dffa2` the objectives at
`+0x29c`, `0x4e0a31` the reward ids at `+0x3c`, `0x4e0b98` and `0x4e0c26` the id and count of reward `i`, `0x4e0ac1` the choices at `+0x5c`, `0x4e0d88` and
`0x4e0e16` the id and count of choice `i`, and `0x4e10f2`, `0x4e1102` and `0x4e1268` the money fields.

## Selected quest text

`GetQuestLogQuestText()` returns the details text and then the objectives text of the selected quest, each expanded for the
player (`$` macros) into a 0x400 byte buffer, or two nils when its template is not cached. `GetAbandonQuestName()` returns
the title of the quest set by `SetAbandonQuest()` (the selected id copied to `0xbb7484`), or nil. `IsCurrentQuestFailed()`
returns 1 when the selected quest's slot has the failed bit or its expiry latch is set, else nil, and nil with no player or no
selection.

Addresses: `0x4dff20`, `0x4dff89`, `0x4dffae`, `0x4dfb50`, `0x4dfb60`, `0x4dfbab`, `0x4e0960`, `0x4e0985`, `0x4e09a4`, `0x4e09aa`, `0x4df050`, `0x4df470`, `0x506f70`, `0xbb7484`
Confidence: verified
Proof: one
Test: none

`0x4dff89` and `0x4dffae` call `ExpandServerText` on the details and the objectives; the first is pushed first. `0x4dfb55` stores the selected id at `0xbb7484`. `0x4e0985` takes the selected quest's slot (`0x4df050`),
`0x4e09a4` tests bit 1 of the state byte, and `0x4e09aa` falls to `0x4df470`, the expiry latch of the selected row. `0x4e09b3` pushes 1.0.

## Reward lists

`GetNumQuestLogRewards()` counts the nonzero reward ids from the start of the four and `GetNumQuestLogChoices()` those of the
six, each 0 for an uncached template. A quest whose four reward ids are all nonzero reports 0 rewards, because the count is
stored only when the scan meets a zero; six choices report 6. `GetQuestLogRewardInfo(i)` and `GetQuestLogChoiceInfo(i)`
return five values from the cached item record: name, icon path, count, quality (the record's quality when its inventory type is
nonzero, else -1) and 1 when the player can use the item, else nil. An index outside the list, a zero id or an uncached item
returns nil, nil, 1, 0, nil. Both raise `Usage: GetQuestLogRewardInfo(index)`, the choice function too.

Addresses: `0x4e09e0`, `0x4e0a34`, `0x4e0a44`, `0x4e0a47`, `0x4e0a49`, `0x4e0a70`, `0x4e0ad2`, `0x4e0b00`, `0x4e0b2b`, `0x4e0b8f`, `0x4e0bc7`, `0x4e0be5`, `0x4e0c02`, `0x4e0c3c`, `0x4e0c83`, `0x4e0ca8`, `0x4e0cf0`, `0x4e0d1b`, `0x4e0d7f`, `0x55ba30`, `0x53f810`, `0x5d88b0`, `0x5ea930`
Confidence: verified
Proof: one
Test: none

The reward count loop `0x4e0a34` to `0x4e0a42` leaves by `jmp 0x4e0a4c` at four nonzero ids without the store at `0x4e0a49`; the choice loop ends at `0x4e0ad2` with `ja`, which stores 6.
`0x4e0bc7` looks up a client string (the icon directory) and `0x4e0be5` the display icon; `0x4e0c02` formats `%s%s%s` (directory, a backslash only when the directory is nonempty, icon) into a 0x104 byte buffer.
`0x4e0c3c` tests the record's inventory type at `+0x2c` before using `+0x1c` as the quality. `0x4e0c83` calls `CanUseItem`; a zero answer reaches the nil at `0x4e0ccf`, no player or a nonzero answer pushes 1.0 at `0x4e0c8c`.
`0x4e0ca8` is the miss path.

## Reward links and money

`GetQuestLogItemLink(type, index)` takes `"choice"` or `"reward"` (compared without regard to case) and a 1-based index, and
returns the item link of that entry, built from the item's name with its suffix text and its quality colour, or nil for an
uncached template or item, a zero id, an unknown type or an index outside the list. Other arguments raise
`Usage: GetQuestLogItemLink("type", index)`. `GetQuestLogRewardMoney()` returns the template money when it is not negative, plus the bonus money
when the player's level is 60 or more, else 0. `GetQuestLogRequiredMoney()` returns the money negated when it is negative, else 0.

Addresses: `0x4e0ee0`, `0x4e0f05`, `0x4e0f19`, `0x4e0f95`, `0x4e0fba`, `0x4e1008`, `0x4e1022`, `0x4e1053`, `0x4e1070`, `0x4e10ac`, `0x4e10b2`, `0x4e10f7`, `0x4e1102`, `0x4e1220`, `0x4e126f`, `0x5d8b00`, `0x52adb0`
Confidence: verified
Proof: one
Test: none

`0x4e0fa4` bounds a choice index by 6 and `0x4e0fc9` a reward index by 4, both unsigned after the decrement. `0x4e1008` writes the suffix name and `0x4e1022` formats the link with the record quality at `+0x1c`.
`0x4e10ac` loads the unit block (`+0x110`) and `0x4e10b2` compares its level at `+0x70` with 0x3c; `0x4e10f7` skips a negative amount and `0x4e1102` adds the bonus.

## Abandoning a quest

`GetAbandonQuestItems()` lists the required items that abandoning would destroy: from the abandon quest's four required item
ids it keeps those the player carries whose item record has bonding 4 or 5 (quest items), drops every one that another
non-header row's quest also requires, and returns their names joined by the list separator in one string, or nothing when no
item remains, no player is present or the template is not cached. `AbandonQuest()` finds the non-header row whose id is the
abandon id and sends its quest slot (`SendQuestLogRemoveQuest`), then clears the id; with no matching row it keeps the id.

Addresses: `0x4dfbe0`, `0x4dfc8a`, `0x4dfcb1`, `0x4dfcb7`, `0x4dfd1c`, `0x4dfd38`, `0x4dfdb9`, `0x4dfde3`, `0x4dfe00`, `0x4df070`, `0x4df0ae`, `0x4df0ca`, `0x4df0cf`, `0x622130`, `0x5eaf40`, `0x4df150`
Confidence: verified
Proof: one
Test: none

`0x4dfc8a` counts the item with `CountInventoryItemsByEntry` and `0x4dfcb1` tests the record's `+0x194` against 4 and 5. `0x4dfcf0` to `0x4dfd67` walk every row, skipping the abandon quest itself (`0x4dfd1c`), and clear matching collected ids (`0x4dfd42`).
`0x4dfdb9` pushes the separator between names and `0x4dfde3` joins `2n-1` strings. `0x4df0a7` tests the row's header word, `0x4df0ae` its id, `0x4df0ca` sends and `0x4df0cf` clears.

## Headers and watches by row

`CollapseQuestHeader(row)` and `ExpandQuestHeader(row)` need a number, raise `Usage: CollapseQuestHeader(index)` or
`Usage: ExpandQuestHeader(index)` otherwise, and call the collapse routine for row - 1 with a collapse flag of 1 or 0.
`GetNumQuestWatches()` returns the count at `0xbb7488`. `IsQuestWatched(row)` returns 1 when the row is a quest whose id is in
the watch list, else nil. `GetQuestIndexForWatch(n)` returns the 1-based row of the quest in watch slot n, or nil for a slot
beyond 5, an empty slot, no player or a quest that has no row.

Addresses: `0x4e1730`, `0x4e176c`, `0x4e1780`, `0x4e17b9`, `0x4e1860`, `0x4e1890`, `0x4e18cb`, `0x4e18f0`, `0x4e19b0`, `0x4e19f2`, `0x4e1a32`, `0x4e1a53`, `0x4ded30`, `0xbb70a0`, `0xbb7488`
Confidence: verified
Proof: one
Test: none

`0x4e176c` and `0x4e17b9` pass the flag in `edx`. `0x4e18cb` resolves the row id, `0x4e18e0` bounds the watch scan by 5 and `0x4e18f0` compares each id. `0x4e19f2` bounds the slot with `cmp eax,5; jae`, and `0x4e1a32` scans the rows for the watched id.
