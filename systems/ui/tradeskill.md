# ui: the profession window list and its verbs

How the profession window (`TradeSkillFrame.cpp`, `0x4fc870` to `0x50031e`) builds its list of recipes from the player's
skill line, classes them by item type, orders and filters them, and what the Lua functions read from each row. The craft
window with a similar shape is `ui/craftverbs`; the recipe icon is `ui/spellbook` ("Recipe icons"); the item-type names come
from the same rows as `GetItemInfo` (`ui/serviceverbs`).

Roots: `0x4fc910`, `0x4fca20`, `0x4fd180`, `0x4fd380`, `0x4fd540`, `0x4fd820`, `0x4fdc50`, `0x4fdd40`, `0x4ff5c0`, `0x500280`

## Window state

The window keeps the skill line it shows (0xbde040, 0 for closed), the selected spell id (0xbde044), the row pointers (a growable
array at `0xbddfb8` with data at 0xbddfc0, 0x20 bytes per row, used count `0xbde04c`), the item-type headers (a growable array at
0xbddff8 with data at 0xbde000, 0x14 bytes each, used count `0xbde050`), the visible row count (0xbde054, what `GetNumTradeSkills`
returns), the mask of equipment slots present (0xbde058), the recast spell and count (0xbde05c, 0xbde060), the line last built
(0xbde064), the count of item records still missing (`0xbde048`) and three masks: the type filter (0x84dd60), the slot filter (0x84dd64) and the expanded types (0x84dd68).

Addresses: `0xbddfb8`, `0xbddff8`, `0xbde040`, `0xbde04c`, `0xbde050`, `0xbde054`, `0xbde058`, `0xbde048`, `0x84dd60`, `0x84dd64`, `0x84dd68`
Confidence: verified
Proof: one
Test: none

The row records are 0x20 bytes: the spell id (-1 for a header row), the difficulty bucket, the item class and subclass, the slot mask (`+0x10`), the item field that orders rows (`+0x14`), the makeable count (`+0x18`) and the visibility word (`+0x1c`). A header record holds the class, the subclass, the recipe count (`+0x08`), the type filter flag (`+0x0c`) and the collapsed flag (`+0x10`).

## Opening and closing

`0x4fc910(line)` closes the window when the line is the open one (TRADE_SKILL_CLOSE, 315). Otherwise it stores the line and,
when the line differs from the last built, resets the three masks to all ones and the slot set and the selection to 0. It then
builds the list and, when there are rows, signals TRADE_SKILL_SHOW (313). `CloseTradeSkill()` stores 0, clears the recast and signals CLOSE.

Addresses: `0x4fc870`, `0x4fc910`, `0x4fc918`, `0x4fc952`, `0x4fc95d`, `0x4fc985`, `0x4fc999`, `0x4fd7f0`, `0x4fd800`, `0x4fd9b0`, `0x4fd9f0`, `0x4fc9b0`, `0x4fc9e0`, `0x4fd790`, `0xbde064`
Confidence: verified
Proof: one
Test: none

`0x4fc870` stores 0, clears the recast and signals 0x13b. `0x4fc95d`, `0x4fc962` and `0x4fc967` store all ones in the three masks and `0x4fc96c` and `0x4fc976` clear the slot set and the selection. `0x4fc9b0` stores the id of a row as the selection (0 for a header); `0x4fc9e0` finds the selected spell among the rows, -1 for none.

## Building the list

`0x4fca20(refresh)` builds the rows. For each spell the player has in the skill line (`FindSkillLineSpells`) it makes a row of
the spell id, a difficulty bucket, the class and subclass of the item the spell creates, a slot mask and the number the player
can make. Rows wait until every created item record is cached, counting the missing ones; each arrival decrements the count and
the last rebuilds. Each distinct class and subclass pair gets a header, whose flags carry over from the earlier build. After
sorting, with `refresh` set, a header with a clear filter flag has its type-filter bit cleared and a collapsed one its expanded
bit, then TRADE_SKILL_UPDATE (314) is signalled.

Addresses: `0x4fca20`, `0x4fca40`, `0x4fcb2a`, `0x4fcbd2`, `0x4fcd4d`, `0x4fcd58`, `0x4fcd65`, `0x4fcd90`, `0x4fce44`, `0x4fcf2d`, `0x4fd08c`, `0x4fd0fb`, `0x4fd15f`, `0x4fc890`, `0x4fc8e0`, `0x4fdc30`, `0x5ea2c0`, `0xbde048`
Confidence: verified
Proof: one
Test: none

`0x4fcb2a` fetches the spells of the line and `0x4fcbd2` finds each one's skill line ability. `0x4fcd4d` looks up the created item (`+0x19c` of the spell) with `0x4fc8e0` as the arrival callback and `0x4fcd58` counts a miss. `0x4fcd90` searches the headers for the item's class and subclass and `0x4fce44` fills the row from the record. `0x4fc8e0` decrements the count on arrival and calls the build with `refresh` 1 at zero. `0x4fd0fb` sorts.

## Difficulty and availability

A row's bucket is set from the skill line ability row (`+0x28` high, `+0x2c` low; a zero low is the high less 25, floored at 0) and the
player's rank in the line: below low is 0 (optimal), below the mean of the two 1 (medium), below high 2 (easy) and otherwise 3
(trivial); a spell without an ability row stays 0. The number the player can make is the least over its reagents of the
carried count divided by the needed count, 0 when it has no reagent; the carried count uses scope 0 of `CountInventoryItemsByEntry`.

Addresses: `0x4fcbd2`, `0x4fcbfc`, `0x4fcc09`, `0x4fcc28`, `0x4fcc41`, `0x4fcc49`, `0x4fcc5f`, `0x4fcc76`, `0x4fcc8c`, `0x4fccf6`, `0x4fcd02`, `0x4fcd04`, `0x4fcf0d`, `0x6de040`, `0x5ea3f0`, `0x622130`
Confidence: verified
Proof: one
Test: none

`0x4fcc09` computes the low bound from `+0x28`, `0x4fcc1f` the mean, and `0x4fcc49` to `0x4fcc8c` the four-way split. `0x4fccf6` counts reagent `i` and `0x4fcd02` divides by the needed count, keeping the least from -1 (`0x4fcd04`). `0x4fcf0d` clamps the result at 0. The bucket text for Lua is the table at 0x8080a0, 32 bytes apiece: `optimal`, `medium`, `easy`, `trivial`.

## Slot kinds

Each row also carries the equipment slots its item fits, from the table of slot masks by the item's inventory type; the finger
type keeps only the first finger slot, the trinket type only the first trinket slot, a bag its own bit (0x80000), and an item
with no slot gets bit 23 (not equippable). The union of the rows' masks is the set of slot kinds the window offers.

Addresses: `0x4fce7f`, `0x4fce8c`, `0x4fce9a`, `0x4fcea3`, `0x4fceb1`, `0x4fcebd`, `0x4fcec8`, `0x4fcedf`, `0x4fcefd`, `0x809200`
Confidence: verified
Proof: one
Test: none

`0x4fce7f` reads the mask for the inventory type at `+0x2c` of the item record. Types 0x12, 0xb and 0xc replace it with 0x80000, 0x400 and 0x1000, and `0x4fcedf` stores 0x800000 for a mask of 0. `0x4fcefd` ors the row's mask into the set.

## Order

The list sorts in two passes: the headers by `0x4fd540` and the rows by `0x4fd380`. A header comes before another when its item
class is smaller; within a class the headers compare the subclass names (the `+0x4c` text, else `+0x28`) case-insensitively. A
row that is hidden by a filter or a collapse sorts after the visible rows. Visible rows sort by their header's position, a header row before its recipes, then
bucket ascending, then the created item's field at `+0x38` descending, then the spell name case-insensitively.

Addresses: `0x4fd380`, `0x4fd3de`, `0x4fd432`, `0x4fd445`, `0x4fd45f`, `0x4fd4ad`, `0x4fd4b7`, `0x4fd4bb`, `0x4fd4d6`, `0x4fd4e2`, `0x4fd506`, `0x4fd528`, `0x4fd540`, `0x4fd567`, `0x4fd5f3`, `0x4fd624`, `0x4fd639`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

`0x4fd432` to `0x4fd445` compare the two visibility words; an invisible row is greater than a visible one and two invisible rows are equal. `0x4fd445` compares the header positions, `0x4fd45f` handles header rows (a header is smaller than a recipe; two headers of one group compare as 1), `0x4fd4ad` the buckets, `0x4fd4b7` the item field and `0x4fd4d6` the names.

## Filters and visibility

`0x4fd180(refresh)` marks what shows. A header's filter flag is its bit in the type filter (set means shown) and its collapse
flag is the clear bit in the expanded mask. A recipe passes the slot filter when its slot mask meets the slot-filter mask. A
recipe is hidden when it fails the slot filter, its header's filter flag is clear, its header has no recipe that passes, or its
header is collapsed; a header is hidden when its flag is clear or it has no passing recipe. Each hidden row lowers the visible count.

Addresses: `0x4fd180`, `0x4fd1af`, `0x4fd1cf`, `0x4fd1e6`, `0x4fd248`, `0x4fd250`, `0x4fd288`, `0x4fd28d`, `0x4fd2f3`, `0x4fd2fa`, `0x4fd30b`, `0x4fd32b`, `0x4fd342`, `0x4fd356`, `0x4fd36f`, `0x4fd710`, `0x4fd730`, `0x4fd750`, `0x73f727`
Confidence: verified
Proof: one
Test: none

`0x4fd180` first copies the masks into the headers, counts passing recipes per header (`0x4fd250`), then sets each row's visibility word at `+0x1c`. `0x4fd32b` decrements the count. The three setters `0x4fd710`, `0x4fd730` and `0x4fd750` store a mask, run the pass with `refresh` 1 and signal TRADE_SKILL_UPDATE.

## Reading a row

`GetTradeSkillInfo(row)` returns four values. For a recipe: the spell's name, its bucket text, the number makeable and nil. For a header: the
subclass name (the `+0x4c` text, else `+0x28`), `header`, 0 and 1 when the header is expanded, else nil. A row past the list, or a recipe whose spell is unknown, gives nil, nil, 0, nil.
`GetTradeSkillCooldown(row)` returns the seconds left of the recipe's cooldown, nil for a header, an unknown row or no cooldown.
`GetTradeSkillNumMade(row)` returns the two ends of the first effect's base points range, each at least 1, and 1, 1 for a missing spell.
`GetTradeSkillItemLink(row)` returns the link of the created item, and no value when it is unknown or uncached.

Addresses: `0x4fd820`, `0x4fd88a`, `0x4fd8fc`, `0x4fd91a`, `0x4fd934`, `0x4fd98a`, `0x4fd9a2`, `0x4fda20`, `0x4fda8b`, `0x4fdaa1`, `0x4fdc50`, `0x4fdcd3`, `0x4fdcdb`, `0x4ff410`, `0x4ff4d2`, `0x4ff4e9`, `0x4fd6a0`, `0x6e2ea0`, `0x6e3760`, `0x5d8b00`, `0x52adb0`
Confidence: verified
Proof: one
Test: none

`0x4fd8fc` reads the header name and `0x4fd934` asks `0x4fd6a0` whether the header is collapsed. `0x4fd992` indexes the bucket text. `0x4fda8b` calls `GetSpellCooldown` and `0x4fdaa1` subtracts the asynchronous time. `0x4fdcd3` reads the points range (`0x4fdcdb` and `0x4fdce0` raise each end to 1).

## The window's line

`GetTradeSkillLine()` returns the localized name of the open skill line, the player's rank in it (rank plus bonus when the rank is positive)
and its maximum; with no line or no skill line row, `UNKNOWN`, 0, 0. The player's skill slot is found with `FindSkillIndex` and
its result is not tested, so a line the player lacks reads the slot before the table. `GetFirstTradeSkill()` returns the
1-based row of the first recipe among the visible rows, 0 for none. `GetNumTradeSkills()` returns the visible count.

Addresses: `0x4fdd40`, `0x4fddb6`, `0x4fdde5`, `0x4fddec`, `0x4fde5b`, `0x4fde92`, `0x5001c0`, `0x5001ec`, `0x5001ef`, `0x4fd800`, `0x5ea3f0`
Confidence: verified
Proof: one
Test: none

`0x4fddb6` stores the index with no test against -1 and `0x4fdde5` and `0x4fde5b` use it. `0x4fde92` pushes the text `UNKNOWN`. `0x5001ec` tests the row id for above 0.

## Reagents, tools and filters

`GetTradeSkillNumReagents(row)` counts the nonzero reagent ids (eight at `+0xa8` of the spell). `GetTradeSkillReagentInfo(row, k)`
returns for the k-th nonzero reagent the item name, its icon path, the needed count and the carried count, four nils when none.
`GetTradeSkillReagentItemLink(row, k)` returns its link. `GetTradeSkillTools(row)` returns pairs of text and flag: the required
focus object name with 1, then each of the two tool items with 1 when the player has it, else nil. `GetTradeSkillSubClasses()` pushes the name of each header's
subclass and returns the header count. `GetTradeSkillInvSlots()` pushes the localized slot name of each slot kind present and returns their count.

Addresses: `0x4ff510`, `0x4ff583`, `0x4ff5c0`, `0x4ff66c`, `0x4ff6b5`, `0x4ff763`, `0x4ff7a2`, `0x4ff800`, `0x4ff980`, `0x4ffa51`, `0x4ffa6f`, `0x4ffaf5`, `0x4ffb10`, `0x4ffb60`, `0x4ffbd7`, `0x4ffc20`, `0x4ffc45`, `0x622130`, `0x622270`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

`0x4ff583` loads the first reagent id and counts nonzero ones. `0x4ff6b5` takes the id of the matched slot and `0x4ff763` its needed count; `0x4ff7a2` counts the player's. `0x4ffa51` loads the focus id (`+0x3c`) and `0x4ffb10` looks for a tool in the inventory. `0x4ffc45` reads the slot name keys (`HEADSLOT` to `NONEQUIPSLOT`), the table at 0x84dd70.

## Setting the filters

`SetTradeSkillSubClassFilter(n [, on, exclusive])` and `SetTradeSkillInvSlotFilter(n [, on, exclusive])` take a string or a number as the first argument (a
non-string, non-number raises the usage text). A number below 1 sets the mask to all ones. Otherwise the subclass filter needs
n - 1 below the header count (`Bad sub class in ...`) and the slot filter takes the n-th slot kind present (`Bad inv slot in ...`); `on` must be a number
(`Missing on//off parameter in ...`): 0 clears the bit, nonzero with a nonzero `exclusive` leaves only it, nonzero otherwise adds it. The getters return 1 when the bit
is set and nil otherwise, and for n below 1 return 1 only when all are set.

Addresses: `0x4ffc70`, `0x4ffc7a`, `0x4ffcab`, `0x4ffcbb`, `0x4ffcd7`, `0x4ffd0d`, `0x4ffd90`, `0x4ffe06`, `0x4ffe60`, `0x4ffe6e`, `0x4ffec0`, `0x4ffed9`, `0x4fff1c`, `0x4fffd0`, `0x500047`, `0x500069`, `0x4fd710`, `0x4fd730`
Confidence: verified
Proof: one
Test: none

`0x4ffc7a` and `0x4ffe6e` test `lua_isstring`, which also passes numbers. `0x4ffec0` walks the 24 slot kinds counting those present to find the n-th. The slot filter stores the bit of the slot kind, not of the index (`0x4ffec7` shifts by the kind). `0x4ffd0d` reads `on` and `0x4ffd30` the exclusive flag.

## Collapsing, repeating and casting

`CollapseTradeSkillSubClass(n)` and `ExpandTradeSkillSubClass(n)` clear or set the bit of the header at row n - 1 in the expanded mask,
or the whole mask for n below 1; a row that is not a header raises `Bad sub class in CollapseTradeSkillSubClass` or `Bad skill line in ExpandTradeSkillSubClass`.
`DoTradeSkill(row [, repeat])` limits the repeat to the row's makeable count, sets the recast spell and count when above 1 (signalling UPDATE_TRADESKILL_RECAST, 502) and casts the
spell for the active player. `GetTradeskillRepeatCount()` returns the recast count while the recast spell is the selected one and above 1, else 1.

Addresses: `0x5000c0`, `0x500104`, `0x50010d`, `0x500133`, `0x500140`, `0x500185`, `0x5001b2`, `0x500230`, `0x500256`, `0x500280`, `0x5002f5`, `0x500303`, `0x500314`, `0x4fd650`, `0x4fd6a0`, `0x4fd750`, `0x4fd770`, `0x6e5a90`
Confidence: verified
Proof: one
Test: none

`0x4fd650` maps a row to its header ordinal, -1 for a recipe. `0x5002f5` loads the makeable count and `0x5002f8` clamps the repeat with it. `0x4fd770` stores the recast spell and count and signals 0x1f6. `0x500314` calls `CastSpellAsActivePlayer` with the spell and no target.

## Item statistics

`GetTradeSkillItemStats(row)` pushes one string per statistics line of the item the recipe creates and returns their number: the quality
name, the binding kind, uniqueness, quest start, container size, then damage, speed, armour, resistances, stat bonuses and spell effects with their charges, and last
the proficiency, skill and rank requirement lines, coloured red when unmet. The text comes from the global strings of the stock item tooltip.

Addresses: `0x4fdec0`, `0x4fdf95`, `0x4fdfbe`, `0x4fe03b`, `0x4fe248`, `0x4fe405`, `0x4fe4d8`, `0x4fe775`, `0x4fe8c2`, `0x4ff242`, `0x4ff2d6`, `0x4ff372`, `0x5e1e70`, `0x703bf0`
Confidence: inferred
Proof: none
Test: none

`0x4fdf95` looks up the created item and `0x4fdfbe` formats `ITEM_QUALITY%d_DESC`. `0x4fe03b` selects the binding string, `0x4fe248` reads the proficiency mask, `0x4fe4d8` formats `%d - %d %s%s`, `0x4fe775` picks the `ITEM_MOD_*` key, `0x4ff242` and `0x4ff2d6` build the requirement lines (`ITEM_REQ_SKILL`, `PVP_MEDAL%d`) and `0x4ff372` returns the count. The call sites follow the function's order.
