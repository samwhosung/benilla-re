# tradeskill: recipes

The profession window's list: one row for each spell the player knows in the open skill line, grouped under headers made from the
created item's class and subclass. The list is built by the client from the player object, with no packet. This doc holds the state, the
records, the build, the difficulty and available count of a row, the filters and the order. The Lua functions that read the list are `bindings`.

Roots: none, the function rows of this file belong to `ui`

## List state

The window keeps its state in `TradeSkillFrame.cpp`'s globals. The two arrays hold a capacity, a count, a data pointer and a granularity; the list never shrinks its arrays.

| address | holds |
|---|---|
| `0xbddfb8` | the row array: capacity, count `0xbddfbc`, data `0xbddfc0`, granularity `0xbddfc4` |
| `0xbddff8` | the header array: capacity, count `0xbddffc`, data `0xbde000`, granularity `0xbde004` |
| `0xbde040` | the skill line of the open window, 0 when closed |
| `0xbde044` | the selected spell id, 0 for none |
| `0xbde048` | the count of pending item requests |
| `0xbde04c` | the full count of rows, header rows included |
| `0xbde050` | the count of headers |
| `0xbde054` | the displayed count |
| `0xbde058` | the inventory slot bits of the rows built for the open line |
| `0xbde05c`, `0xbde060` | the repeat spell and the repeat count |
| `0xbde064` | the skill line of the last build |
| `0x84dd60`, `0x84dd64`, `0x84dd68` | the masks: subclass filter, slot filter, collapse; all start as `0xffffffff` |

Addresses: `0xbddfb8`, `0xbddfbc`, `0xbddfc0`, `0xbddfc4`, `0xbddff8`, `0xbddffc`, `0xbde000`, `0xbde004`, `0xbde040`, `0xbde044`, `0xbde048`, `0xbde04c`, `0xbde050`, `0xbde054`, `0xbde058`, `0xbde05c`, `0xbde060`, `0xbde064`, `0x84dd60`, `0x84dd64`, `0x84dd68`, `0x4fc750`, `0x4909bf`, `0x4fc790`, `0x4911c8`
Confidence: verified
Proof: one
Test: none

`0x4fc750` zeroes the selection, the three counts, the last-built line and the repeat spell; the world entry calls it (`0x4909bf`). `0x4fc790` frees every row and header, the arrays and the open line, and zeroes the counts; the game shutdown calls it (`0x4911c8`).
A bit of a mask is set for a shown or expanded item: the subclass mask is keyed by header position, the slot mask by slot bit, the collapse mask by header position.

## Records

A row is `0x20` bytes under the file's path, and a header is `0x14` bytes. A header row of the list is a row whose spell id is -1 and whose class and subclass name its group.

| row offset | holds |
|---|---|
| `+0` | the spell id, or -1 for a header row |
| `+4` | the difficulty, 0 to 3 |
| `+8`, `+0xc` | the class and subclass of the created item; for a header row, of the group |
| `+0x10` | the inventory slot bit of the created item |
| `+0x14` | the item level of the created item |
| `+0x18` | the available count |
| `+0x1c` | the visible flag |

| header offset | holds |
|---|---|
| `+0`, `+4` | the item class and subclass |
| `+8` | the count of member rows that pass the slot filter |
| `+0xc` | 1 when the subclass filter shows the group |
| `+0x10` | 1 when the group is collapsed |

Addresses: `0x4fcb57`, `0x4fcb5c`, `0x4fcdc7`, `0x4fcdcc`, `0x4fce52`, `0x4fce61`, `0x4fce70`, `0x4fce86`, `0x4fcf19`, `0x4fcbe4`, `0x4fcbef`, `0x4fd0a3`, `0x4fd0c5`, `0x4fd0e5`, `0x4fd295`, `0x4fce0c`, `0x4fce19`, `0x4fce28`, `0x4fd1aa`, `0x4fd1cf`, `0x4fd1e6`, `0x500710`, `0x500800`, `0x5008f0`
Confidence: verified
Proof: one
Test: none

`0x4fcb5c` and `0x4fcdcc` push the sizes `0x20` and `0x14`. The builder stores the spell id at `0x4fcbe4`, the difficulty at `0x4fcbef`, the class and subclass at `0x4fce52` and `0x4fce61`, the item level at `0x4fce70`, the slot bit at `0x4fce86` and
the available count at `0x4fcf19`. A header takes its class and subclass at `0x4fce0c` and `0x4fce19` and starts shown (`0x4fce28`); `0x4fd1aa`, `0x4fd1cf` and `0x4fd1e6` write its count and flags. The header rows
are written at `0x4fd0a3`, `0x4fd0c5` and `0x4fd0e5`, and the visible flag at `0x4fd295`. The array grow functions are `0x500710` (rows), `0x500800` (headers) and `0x5008f0` (row capacity).

## Building the list

`0x4fca20` rebuilds the list and takes one flag. A set flag means the recipe set changed: the headers are saved, dropped and made again, and each new header takes the shown and collapsed flags of the saved header with the same class and subclass. A clear flag keeps the headers and
refreshes the rows and counts. The build reads the spells of the open line from the local player, makes a row for each and a header row for each header, filters and sorts with `0x4fd180`, and raises `TRADE_SKILL_UPDATE`. It makes nothing when no window is open, the player is missing or the line has no list.

Addresses: `0x4fca20`, `0x4fca40`, `0x4fca4b`, `0x4fcad8`, `0x4fcae2`, `0x4fcae8`, `0x4fcaee`, `0x4fcb1b`, `0x4fcb2a`, `0x4fcb36`, `0x4fcba4`, `0x4fcd70`, `0x4fcf3a`, `0x4fcf7c`, `0x4fd08c`, `0x4fd0f3`, `0x4fd0fb`, `0x4fd100`, `0x4fd15f`, `0x4fd164`, `0x4fc985`, `0x5e9d79`, `0x5ea165`, `0x4f8e65`, `0x4f8f6d`, `0x4f917c`, `0x4fdc3c`, `0x5d85be`, `0x5d8788`, `0x5d94d1`, `0x5de434`, `0x5de457`, `0x5dee70`, `0x5ea2c0`, `0x468460`
Confidence: verified
Proof: one
Test: none

The window open (`0x4fc985`), a learned spell (`0x5e9d79`) and an unlearned spell (`0x5ea165`) pass flag 1. Flag 0 comes from a bag or backpack slot change (`0x4f8e65`, `0x4f8f6d`, `0x4f917c`), the item-record callback of the Lua functions (`0x4fdc3c`), an arriving item record (`0x5d85be`), a deactivated item (`0x5d8788`), a stack change (`0x5d94d1`), a skill value change (`0x5de434`, `0x5de457`) and a visible item change (`0x5dee70`).
`0x4fca40` cancels the pending requests. `0x4fcad8` zeroes the header count and `0x4fcae2` and `0x4fcae8` reset the subclass and collapse masks, for a set flag only. `0x4fcaee`, `0x4fcb1b` and `0x4fcb36` leave when the line is 0, the player is missing or the line's list is. The player is the object of type 0x10 for the local guid (`0x468460`);
`0x4fcb2a` fetches the spells and `0x4fcba4` stores their count. `0x4fcf3a` to `0x4fcf7c` restore the saved flags. `0x4fd08c` to `0x4fd0f3` append the header rows; a clear flag skips the header creation at `0x4fcd70`. From `0x4fd100` a set flag clears the subclass-mask bit of every header whose shown flag is 0 and the collapse-mask bit of every collapsed header.

## Difficulty

A row's difficulty comes from the SkillLineAbility row that `0x6de040` returns for the local player's race, class and the spell. With high = `+0x28` and low = `+0x2c`, a zero low becomes high - 25 (zero when that is zero, negative when high is below 25), and mid = (high + low) / 2 truncated toward zero.
The rank is the player's raw rank in the line of the row (`+4`), 0 when the player has no slot for it. A rank below low gives 0, below mid 1, below high 2, and 3 otherwise; a spell with no row stays 0.

Addresses: `0x4fcbd2`, `0x4fcbfc`, `0x4fcc01`, `0x4fcc09`, `0x4fcc10`, `0x4fcc1d`, `0x4fcc28`, `0x4fcc41`, `0x4fcc49`, `0x4fcc5f`, `0x4fcc76`, `0x4fcc8c`, `0x6de040`, `0x5ea3f0`
Confidence: verified
Proof: one
Test: none

`0x4fcbd2` calls the ability finder; `0x4fcbfc` and `0x4fcc01` load low and high. `0x4fcc09` to `0x4fcc12` compute high - 25 and keep it when it is not zero (`sbb edi,edi` after `cmp 0,eax`). `0x4fcc1d` halves the sum toward zero. `0x4fcc28` finds the skill slot and `0x4fcc41` reads the rank word at
`+0x84c` of the slot, with no bonus. The compares at `0x4fcc49`, `0x4fcc5f`, `0x4fcc76` and `0x4fcc8c` are signed.

## Available count

The available count of a row is the number of crafts the bags support: for each of the eight reagent slots of the spell (items at `+0xa8`, counts at `+0xc8`) that has both an item and a count, the inventory count of the item
(`0x622130` on the player's inventory at `+0x1d38`) divided by the count with truncation; the least quotient wins. The count is 0 when no slot qualifies or the least quotient is not above 0. The scan stops at the first quotient of 0.

Addresses: `0x4fcce0`, `0x4fccfc`, `0x4fcd02`, `0x4fcd04`, `0x4fcd09`, `0x4fcf03`, `0x4fcf0d`, `0x4fcf19`, `0x622130`
Confidence: verified
Proof: one
Test: none

`0x4fcce0` leaves the loop when the running minimum is 0. `0x4fccfc` counts the item, `0x4fcd02` divides, and `0x4fcd04` to `0x4fcd09` take the minimum with -1 as the first value. `0x4fcf0d` clamps the minimum to 0 when it is not above 0, and `0x4fcf19` stores it. A spell with no reagent slot keeps -1 and gets 0.

## Created item

The created item of a row is `EffectItemType[0]` (`+0x19c` of the spell), whatever the effect. Its template comes from the item cache (`0x55ba30` with the requester spell id and the callback `0x4fc8e0`). A miss counts a pending request in `0xbde048`; the
rows from it on keep only their spell id, difficulty and available count, and make no header. When the pass ends with requests pending, the build returns without a sort or an event. The callback rebuilds with a set flag when the pending count reaches 0 or is already 0. The cancel `0x4fc890` that
starts a build passes the row's spell id as its key, while the requests carry the item id, so it cancels nothing (below).

Addresses: `0x4fcd1e`, `0x4fcd3d`, `0x4fcd4d`, `0x4fcd58`, `0x4fcd5e`, `0x4fcd65`, `0x4fcf2d`, `0x4fcf34`, `0x4fc8e0`, `0x4fc890`, `0x4fc8b1`, `0x4fc8bc`, `0x4fc8cb`, `0x4fc8f1`, `0x4fc8fb`, `0x55c0d6`, `0x55c0f8`, `0x4fc780`, `0x490ae9`, `0x4fc913`, `0x55ba30`, `0x55c0c0`, `0x4fc8aa`, `0x4fc8ad`
Confidence: verified
Proof: one
Test: none

`0x4fcd3d` loads the item id and `0x4fcd4d` calls the cache. A null result increments the pending count at `0x4fcd58`; `0x4fcd65` leaves the row's work when the count is not 0, and `0x4fcf34` leaves the build. `0x4fc8e0` returns when its ready byte is 0 (`0x4fc8e3`); otherwise it decrements a non-zero count and
rebuilds with flag 1 when the count is 0. `0x4fc890` loops over the rows (`0x4fc8aa`), loads each spell id (`0x4fc8ad`), calls `0x55c0c0` with that id, the callback `0x4fc8e0` and 0 (`0x4fc8bc`), and zeroes the count (`0x4fc8cb`). It runs at the start of a build, at the start of a toggle (`0x4fc913`) and
on leaving the world (`0x4fc780` from `0x490ae9`).

## Cancelling the item requests

The cancel `0x4fc890` removes nothing the build asked for. The cache's cancel `0x55c0c0` finds a record by the id it is given and drops the callback entry with the same callback and argument from that record. The requests are keyed by item id and `0x4fc890` passes spell ids, so the item callbacks stay registered. `0x4fc8cb` then zeroes the pending count, a record that arrives later finds the count 0 and rebuilds with flag 1, and a spell id equal to a pending item id would drop that entry's callback.

Addresses: `0x4fc890`, `0x55c0c0`, `0x55c0d6`, `0x55c0f8`, `0x55c117`, `0x4fc8cb`, `0x4fc8f1`, `0x4fc8fb`, `0xc0e2a0`
Confidence: verified
Proof: one
Test: none

`0x55c0c0` is a thiscall on the cache `0xc0e2a0` with the id, the callback and the argument pushed (`ret 0xc`). It hashes the id (`and eax,edi`, `0x55c0d6`) and walks the bucket for the record whose id equals it (`cmp [eax],edi`, `0x55c0f8`). A loaded record (byte `+0x201`, `0x55c117`) only gets `DBCache::CancelCallback ignored for id %016I64X` written; otherwise the matching entry of the list at `+0x1fc` is unlinked and freed. `0x4fc8f1` takes the count-0 branch to `0x4fc8fb`.


## Inventory slots

Each row carries the slot bit of its created item: the entry of the table `0x809200` for the item's InventoryType (`+0x2c`), replaced by 0x80000 for a bag, 0x400 for a finger item and 0x1000 for a trinket, and by 0x800000 when the result is 0. The builds of a line OR the bits into `0xbde058`. The 24 bit positions name the global strings at
`0x84dd70`, from HEADSLOT through four BAGSLOT entries to NONEQUIPSLOT.

Addresses: `0x809200`, `0x4fce7f`, `0x4fce8c`, `0x4fce9a`, `0x4fcea3`, `0x4fceb1`, `0x4fcebf`, `0x4fcec8`, `0x4fcedf`, `0x4fcefd`, `0x84dd70`, `0x4ffc2b`
Confidence: verified
Proof: one
Test: none

`0x4fce7f` loads the table entry; `0x4fce8c`, `0x4fcea3` and `0x4fceba` compare the type with 0x12, 0xb and 0xc and store the replacement bits at `0x4fce9a`, `0x4fceb1` and `0x4fcec8`. `0x4fcedf` stores 0x800000 for a zero bit and `0x4fcefd` ORs the bit into the mask. The table gives head 0x1, neck 0x2, shoulder 0x4, body 0x8, chest and robe 0x10, waist 0x20, legs 0x40,
feet 0x80, wrist 0x100, hands 0x200, finger 0xc00, trinket 0x3000, weapon 0x18000, shield, off hand and holdable 0x10000, ranged, thrown, ranged right and relic 0x20000, cloak 0x4000, two-hand and main hand 0x8000, bag 0x780000, tabard 0x40000, and 0 for non-equippable, ammo and quiver. `0x4ffc2b` walks the 24 pointers in `bindings`.

## Filter and visibility

`0x4fd180` recomputes the displayed count and the visible flags and takes one flag. A set flag first derives each header's shown flag from the subclass mask and its collapsed flag from the collapse mask. It counts the members of each header whose slot bit passes the slot mask `0x84dd64`. A recipe row is hidden when its slot bit fails, its group is not shown, its group has no members or its group is collapsed;
a header row is hidden when its group is not shown or empty. Each hidden row reduces the displayed count, which starts as the full count. Last it sorts the headers with `0x4fd540` and the rows with `0x4fd380`.

Addresses: `0x4fd180`, `0x4fd18b`, `0x4fd1af`, `0x4fd1cf`, `0x4fd1e6`, `0x4fd242`, `0x4fd250`, `0x4fd282`, `0x4fd28d`, `0x4fd295`, `0x4fd2e7`, `0x4fd2f0`, `0x4fd2f7`, `0x4fd31b`, `0x4fd32b`, `0x4fd306`, `0x4fd30b`, `0x4fd312`, `0x4fd342`, `0x4fd356`, `0x4fd36f`, `0x4fd540`, `0x4fd380`, `0x73f727`
Confidence: verified
Proof: one
Test: none

`0x4fd18b` copies the full count to the displayed count. `0x4fd1af` to `0x4fd1e6` derive the two flags from the masks. `0x4fd242` and `0x4fd250` count a member whose slot bit passes. `0x4fd28d` hides a recipe that fails the slot mask. `0x4fd2f0` to `0x4fd31b` hide a row of a group that is not shown or has no members, `0x4fd306` leaves a header row visible, and `0x4fd30b` to `0x4fd312` hide a recipe of
a collapsed group, so a collapsed group keeps its header. `0x4fd32b` decrements the displayed count. A row whose group is not found uses the first header (`0x4fd2e7`). `0x4fd356` and `0x4fd36f` call the sort `0x73f727` with the comparators.

## Row order

The row comparator `0x4fd380` puts the rows that are visible and in a shown group first; two hidden rows compare equal. Among the others it orders by the position of the group in the header array, the header row before its recipes, difficulty ascending, item level descending, and the localized spell name (`+0x1e0` plus 4 for the locale) case-blind. Two recipes
of equal difficulty, level and name compare equal. A pair with a missing spell record compares equal.

Addresses: `0x4fd380`, `0x4fd3de`, `0x4fd3f6`, `0x4fd417`, `0x4fd44f`, `0x4fd464`, `0x4fd4a1`, `0x4fd4ad`, `0x4fd4b9`, `0x4fd4e6`, `0x4fd4bb`, `0x4fd4d6`, `0x4fd51b`, `0x4fd528`, `0x4fd506`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

`0x4fd3de` to `0x4fd3f6` and `0x4fd40d` to `0x4fd425` set the shown flag of each side; `0x4fd432` to `0x4fd43f` and `0x4fd51b` to `0x4fd528` order the sides by it. `0x4fd44f` orders by group position; `0x4fd464` and `0x4fd506` put the header row first; `0x4fd4ad` compares the difficulty ascending and `0x4fd4e6` (`setl`) the item
level so the higher level sorts first; `0x4fd4d6` calls the case-blind compare `0x64a4c0` with the spell names. `0x4fd4a1` returns 0 when a spell record is missing.

## Header order

The header comparator `0x4fd540` orders by item class ascending and, within a class, by the localized name of the ItemSubClass row of the group, case-blind. The name is `+0x4c` plus 4 for the locale, or `+0x28` plus 4 for the locale when the first is empty. It returns 0 for two headers
of the same class when either subclass row is missing. Rows of the table at `0xc0db90` are `0x74` bytes.

Addresses: `0x4fd540`, `0x4fd567`, `0x4fd581`, `0x4fd5a1`, `0x4fd5f3`, `0x4fd604`, `0x4fd624`, `0x4fd639`, `0x4fd630`, `0xc0db90`, `0xc0db94`
Confidence: verified
Proof: one
Test: none

`0x4fd567` branches on equal classes and `0x4fd639` (`setg`) orders unequal classes ascending. The scan from `0x4fd581` matches the class and subclass of each side against the rows (`0x4fd5a1`). `0x4fd5f3` and `0x4fd604` select the names and `0x4fd624` compares them; `0x4fd630` returns 0 for a missing row.

## Filter masks

Three small functions set a mask and refresh. `0x4fd710` stores the subclass mask, `0x4fd730` the slot mask and `0x4fd750` the collapse mask; each calls `0x4fd180` with flag 1 and raises `TRADE_SKILL_UPDATE`. `0x4fd650` returns the header position of a header row, or -1, and
`0x4fd6a0` returns 1 when the group of a header row is collapsed (its collapse bit is clear), else 0.

Addresses: `0x4fd710`, `0x4fd730`, `0x4fd750`, `0x4fd650`, `0x4fd6a0`, `0x4fd6ea`, `0x4fd6fa`, `0x4fd725`, `0x4fd745`, `0x4fd765`
Confidence: verified
Proof: one
Test: none

`0x4fd710`, `0x4fd730` and `0x4fd750` store `ecx` in `0x84dd60`, `0x84dd64` and `0x84dd68`, load flag 1, call `0x4fd180` and jump to the signal at `0x4fd725`, `0x4fd745` and `0x4fd765`. `0x4fd6ea` tests the bit of the header position in `0x84dd68` and `0x4fd6fa` turns a clear bit into 1.

## Open, close and selection

`0x4fc910` opens a window for a skill line. It cancels the pending requests first. The line that is already open closes the window with `TRADE_SKILL_CLOSE`. A line with a SkillLine row is stored; when it differs from the last built line, the three masks, the slot bits and the selection reset. The build runs with flag 1 and `TRADE_SKILL_SHOW` follows when the full count is above 0.
`0x4fc870` closes: it stores 0 in the line, clears the repeat latch and raises `TRADE_SKILL_CLOSE`. Selecting a row (`0x4fc9b0`) stores its spell id, or 0 for a header row or an index out of range; `0x4fc9e0` returns the index of the selected spell among all rows, or -1.

Addresses: `0x4fc910`, `0xbde064`, `0x4fc913`, `0x4fc918`, `0x4fc920`, `0x4fc92a`, `0x4fc94c`, `0x4fc95d`, `0x4fc96c`, `0x4fc976`, `0x4fc980`, `0x4fc991`, `0x4fc999`, `0x4fc870`, `0x4fc87a`, `0x4fc884`, `0x4fc9b0`, `0x4fc9e0`, `0x4fd7f0`, `0x4fc930`, `0x4fc99f`
Confidence: verified
Proof: one
Test: none

`0x4fc918` compares the argument with the open line and `0x4fc920` to `0x4fc930` close. `0x4fc94c` compares it with the last built line; `0x4fc95d` to `0x4fc976` reset the masks, the slot bits and the selection. `0x4fc980` calls the build with flag 1; `0x4fc991` stores the last built line and `0x4fc999` raises the event.
`0x4fc87a` calls the repeat clear `0x4fd790` and `0x4fc884` raises the close event; `0x4fd7f0` is the Lua function that calls `0x4fc870` and returns 0. `0x4fc9b0` and `0x4fc9e0` read the row array.
