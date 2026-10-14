# ui: auction lists, sorting and deposits

What the auction Lua functions answer: the list reads, the selection, the sort, the filter menus, the deposit, the sell
slot and the three requests that check before they send. This is `AuctionHouse.cpp` (`0x4ce110` to `0x4d0260`). The
state and the records are `ui/auction`; the replies and messages are `ui/auctionmessages`.

Roots: `0x4cee40`, `0x4cf2f0`, `0x4cf470`, `0x4cfc00`, `0x4ce1c0`, `0x4ce310`, `0x4ce770`, `0x4cf610`, `0x4d0260`

## List reads

`GetNumAuctionItems("type")` is `0x4ced20`; the type is `owner`, `bidder` or `list`, matched without case in that
order. It returns two numbers, the list's count and its total. A type that is none of them returns a count of 2 with
nothing pushed, so Lua takes the two values already on the stack, the called function and its argument. A non-string
raises `Usage: GetNumAuctionItems("type")`. The functions that take an index (1-based, truncated) all take
`("type", index)`, raise their usage line for a bad argument, and treat an unknown type like an index outside the list.

Addresses: `0x4ced20`, `0x4ced6b`, `0x4cedb0`, `0x4cedf4`, `0x4cee30`, `0x4cee31`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

`0x4ced6b`, `0x4cedb0` and `0x4cedf4` are the three string tests; each success pushes the count by `fild` and the
total at the shared tail `0x4cee23`. The unmatched branch jumps to `0x4cee30`, then `mov eax,2` at `0x4cee31` with no push
in between. The invoker takes `L->top - 2*16` as the first result (`0x6f61aa shl eax,4`, `0x6f61b4 sub eax,edx`).

## Auction item info

`GetAuctionItemInfo("type", index)` is `0x4cee40` and returns 12 values: name, icon path, stack count, quality, usable
(1 or nil), required level (at least 1), minimum bid, minimum increment, buyout, current bid, high bidder and owner.
The name carries the random suffix; the icon path is built as in `ui/portraittextures#bag-portrait`. On the owner list, for a row
the player owns, the high bidder is the bidder's cached name (nil with no bidder or no name yet); elsewhere it is 1
when the bidder is the player, else nil. The owner is a name or nil. With no row, no cached item record or an unknown
type it returns `nil, nil, 1, -1, nil, 0, 0, 0, 0, 0, nil, nil`.

Addresses: `0x4cee40`, `0x4cef80`, `0x4cef96`, `0x4cefb9`, `0x4cefd0`, `0x4cf025`, `0x4cf035`, `0x4cf06e`, `0x4cf094`, `0x4cf0b6`, `0x4cf12a`, `0x4cf1ec`, `0x4cf270`, `0x5ea930`, `0x5d8b00`, `0x55f080`, `0x5d88b0`
Confidence: verified
Proof: one
Test: none

The two argument tests are `0x6f3510` and `0x6f34d0` (`0x4cf270` raises the usage line `Usage:
GetAuctionItemInfo("type", index)`). The record is found by `0x4ceed3`, `0x4cef1e` and `0x4cef5c`; a name lookup
waits on the callback `0x4cf2b0`, `0x4cf2d0` or `0x4cf290` for the list, which re-signals the list's event once the
name arrives. Quality is the template `+0x1c`, the level `+0x3c`, the count `rec+0x18`. The canned tuple is the
sequence from `0x4cf1ec` to `0x4cf262`, with the 1 and the -1 as numbers (`0x3ff00000`, `0xbff00000`).

## Item links and time left

`GetAuctionItemLink("type", index)` pushes one string, `<colour>|Hitem:<entry>:<enchant>:<random>:<suffix>|h[<name>]|h|r`,
the colour by quality from seven fixed codes (grey, white, green, blue, purple, orange, light gold; quality 7 or more
uses white), and pushes nothing on a miss. `GetAuctionItemTimeLeft("type", index)` pushes 1 to 4 and 0 on a miss: the
number 1 when the deadline has passed, otherwise 1, 2 or 3 as the time left is under 30 minutes, 2 hours or 8 hours,
else 4. A row more than 10000 ms past its deadline is also removed from the lists.

Addresses: `0x4cf2f0`, `0x52adb0`, `0x52ad90`, `0x854124`, `0xc0cf68`, `0x8549c8`, `0x4cf470`, `0x4cf548`, `0x4cf554`, `0x4cf569`, `0x4cf574`, `0x4cf590`, `0x4cf5a1`, `0x8072a8`
Confidence: verified
Proof: one
Test: none

`0x52adb0` formats `%s|Hitem:%d:%d:%d:%d|h[%s]|h%s` into the static buffer `0xc0cf68` (limit 0x400); the colour is
`0x854124[quality]`, index 1 from quality 7 (`0x52ad90`). The time left uses `0x42c010`; `0x4cf558` pushes 1.0 for a
past deadline and `0x4cf569 cmp edi,0x2710; jbe` guards the removal `0x4cdfe0`. The thresholds are `1800000`, `7200000` and
`28800000` at `0x8072a8`; `0x4cf5a1` pushes 4.0.

## Selection

`SetSelectedAuctionItem("type", index)` stores the id of the indexed record in a cell per list (`0x849058` browse,
`0x84905c` owner, `0x849060` bidder), or -1 when the index is outside the list, and returns nothing.
`GetSelectedAuctionItem("type")` scans the list for that id and returns its 1-based position, or 0 when it is gone, so
sorting never moves the selection. The cells start at -1 and the reset sets them to 0.

Addresses: `0x4cfda0`, `0x4cfec0`, `0x849058`, `0x84905c`, `0x849060`, `0x4cfe13`, `0x4cfe53`, `0x4cfe92`, `0x4cff35`, `0x4cc2d5`
Confidence: verified
Proof: one
Test: none

The stores are `0x4cfe13`, `0x4cfe53` and `0x4cfe92`, their -1 forms at `0x4cfe1e`, `0x4cfe5d` and `0x4cfe9c`. The getter's scan
is `0x4cff21` to `0x4cff30`, and `0x4cff32 or eax,-1` then `inc eax` gives 0 when not found. The reset stores 0 at
`0x4cc2d5`. The usage lines are `Usage: SetSelectedAuctionItem("type", index)` and `Usage: GetSelectedAuctionItem("type")`.

## Sorting

`SortAuctionItems("type", "sort")` matches the sort name without case: `level` 0, `quality` 1, `bid` 2, `duration` 3,
`buyout` 4, `status` 5, `name` 6, `seller` 7. An unmatched sort name or type sorts nothing. The column is searched in
the list's 8-slot key stack: at slot 0 its reversed flag toggles; at a lower slot it moves to slot 0 and keeps its flag;
absent, the stack is unchanged. The list is then sorted by `qsort` with all 8 keys in turn, each result negated when
its flag is set, and the list's update event is signalled. A list that has just arrived is sorted the same way once its
lookups are done.

Addresses: `0x4cfc00`, `0x4cd850`, `0x4cdc70`, `0x4cdd60`, `0x4cd8b0`, `0x4cdcd0`, `0x4cddc0`, `0x4cd8e0`, `0x4cdd00`, `0x4cddf0`, `0x4cd940`, `0x4cd926`, `0x4cd92f`, `0x4cd866`, `0x4cd87f`, `0x4cd8c2`, `0x73f727`, `0x4cfd90`
Confidence: verified
Proof: one
Test: none

`0x4cfc4b` to `0x4cfd19` hold the eight string tests and `0x4cfd22` onward the type tests that call the three drivers.
Each driver searches the stack (`0x4cd852`), toggles with `sete` (`0x4cd876`) at slot 0 or shifts with the loop `0x4cd887`
to `0x4cd895`, stores the column and flag (`0x4cd899`, `0x4cd89f`) and falls into the sort tail. The adapters
compare the pointers' records slot by slot (`0x4cd8f0` to `0x4cd91d`) with `0x4cd940(a, b, mode)` and negate when
`[slot+4]` is nonzero. Ties on all eight keys return 0, and `qsort` does not promise to keep their order.

## Sort columns

The comparator `0x4cd940` returns 0 for equal keys and for a row whose item record is not cached, and otherwise: level
(item level requirement, at least 1) lowest first; quality highest first; bid (current bid, or the minimum bid when
no bid) lowest first; duration (deadline) soonest first; buyout (unsigned) highest first; status (2 for your own
bid, 1 for another's, 0 for none) your bid first; name (the item template name, not the displayed name) A to
Z without case; seller (the owner's cached name) A to Z, a seller whose name is unknown last. Each is the order at
reversed = 0.

Addresses: `0x4cd940`, `0x4cd96e`, `0x4cdc48`, `0x4cda54`, `0x4cd9e1`, `0x4cdb81`, `0x4cdad4`, `0x4cdaf2`, `0x4cdb10`, `0x4cd975`, `0x4cdbc0`, `0x4cdc23`, `0x4cdc31`
Confidence: verified
Proof: one
Test: none

The jump table `0x4cdc48` maps the modes 0 to 7 to `0x4cda54`, `0x4cd9e1`, `0x4cdb81`, `0x4cdad4`, `0x4cdaf2`, `0x4cdb10`,
`0x4cd975` and `0x4cdbc0`. The idioms: level `setge` (+1 when a > b), quality `setl` (+1 when a < b), bid `setge`,
duration `sbb; and -2; inc` (-1 when a < b), buyout `sbb; and 2; dec` (+1 when a < b), status `setl`. The name arm calls
`0x64a4c0` on the template `+8` strings; the seller arm resolves both GUIDs in the name cache (`0xc0e228`) and
returns -1 when only b is unknown (`0x4cdc23`) and 1 when only a is (`0x4cdc31`).

## Reversed flag

`IsAuctionSortReversed("type", "sort")` returns the reversed flag of the column's slot in that list's key stack, as a
number; the string tests match `SortAuctionItems`. The sort state lives only in the three key stacks.

Addresses: `0x4d0030`, `0xb725b0`, `0xb72420`, `0xb72548`
Confidence: inferred
Proof: none
Test: none

`0x4d0030` repeats the column and type tests of `0x4cfc00` and reads the key stack's second dword; its body past those tests
is not derived. The stacks are set only by the initialization (`0x4cc238` to `0x4cc274`) and by the three drivers.

## Filter menus

`GetAuctionItemClasses()` pushes the localized name (locale column at `+0xc` of the `ItemClass.dbc` row) of each class
of the table `0x807060` in order: weapon (2), armor (4), container (1), consumable (0), trade goods (7), projectile (6),
quiver (11), recipe (9), reagent (5), miscellaneous (15). `GetAuctionItemSubClasses(index)` takes a 1-based index into
that table; for 10 or more, or a class without subclasses, it returns nothing, else it pushes the name (`+0x4c` of the
`ItemSubClass.dbc` row, or `+0x28` when empty) of each row of the class whose `DisplayFlags` bit 0x2 is clear.
`GetAuctionInvTypes(classIndex, subClassIndex)` pushes up to 14 inventory type keys, not localized text.

Addresses: `0x4cf970`, `0x4cf9c0`, `0x4cfab0`, `0x807060`, `0x8070b0`, `0xc0dc24`, `0xc0db90`, `0x4cf9ff`, `0x4cfa54`, `0x4cfb01`, `0x4cfb61`, `0x4cfb63`
Confidence: verified
Proof: one
Test: none

`0x807060` holds ten pairs {class id, has subclasses}: (2,1) (4,1) (1,1) (0,0) (7,0) (6,1) (11,1) (9,1) (5,0) (15,0).
`0x4cf9ff dec eax; cmp eax,0xa; jae` bounds the index; `0x4cfa54 test [esi+0x14],2` is the exclusion. `0x8070b0` is 14
records of 0x24 bytes: an id and a name, `INVTYPE_HEAD`, `NECK`, `SHOULDER`, `BODY`, `CHEST`, `WAIST`, `LEGS`, `FEET`,
`WRIST`, `HAND`, `FINGER`, `TRINKET`, `CLOAK`, `HOLDABLE` (ids 1 to 12, 16 and 23). Only the names are pushed. A
class index of 10 or more raises nothing and pushes nothing (`0x4cfb01`); a subclass index past the end skips the row
test at `0x4cfb63` and pushes all 14 names.

## Query arguments

`QueryAuctionItems(text, minLevel, maxLevel, invTypeIndex, classIndex, subClassIndex, page, usable, quality)` sends
0x258. Every argument is optional. The text is the empty string when absent; the levels and `usable` are truncated to a
byte; the inventory type is the id from the 14-entry table (index 1-based) else -1; the class is the id from the
class table (1-based) else -1; the subclass is the id of the Nth non-excluded row of that class else -1; the page
becomes the offset page * 50, else 0; the quality is the number or -1.

Addresses: `0x4ce980`, `0x4cea67`, `0x4ceaa0`, `0x4ceafe`, `0x4ceb44`, `0x4ceb9a`, `0x4cebb5`, `0x4cec3d`, `0x8070b0`, `0x807060`
Confidence: verified
Proof: one
Test: none

The arguments are read at `0x4ce9e2` (text), `0x4cea17` and `0x4cea3b` (levels), `0x4cea70` (inventory type),
`0x4ceaa6` (class), `0x4ceb1c` (subclass), `0x4ceb44` (page), `0x4ceb6f` (usable) and `0x4ceb96` (quality). The subclass is
chosen by the scan from `0x4ceaf0` to `0x4ceb17`, counting rows whose class matches and whose `DisplayFlags` bit 0x2 is clear.

## Deposit

`GetAuctionHouseDepositRate()` returns the house's deposit percentage (column 2 of its `AuctionHouse.dbc` row), or 0.
`CalculateAuctionDeposit(runTime)` needs a number (`Usage: CalculateAuctionDeposit(runTime)`), and returns
floor(rate * sellPrice * count / 100) * (runTime / 120) when a house, the sell slot item and its record are known and
`runTime` is 120, 480 or 1440; the division is exact and happens before the multiply. Anything unknown gives 0. The
consignment column of the row is never read.

Addresses: `0x4ce170`, `0x4ce1c0`, `0x4ce29c`, `0x4ce292`, `0x4ce295`, `0x4ce29f`, `0x644bc0`, `0x644bd0`, `0x644be9`, `0x644bf9`, `0x80e4b0`, `0xc0dff0`, `0x4ce2f0`
Confidence: verified
Proof: one
Test: none

`0x4ce181` indexes the house id (`0xb72600`) in the `AuctionHouse.dbc` store (`0xc0dff0`) and
`0x4ce18e` pushes `fild [row+8]`. `0x644bc0(rate, price * count, runTime)` accepts only the three durations (`0x80e4b0`),
computes `rate * price * count` in 32 bits, divides by 100 (`0x51eb851f`, `shr 5`) and runTime by 120 (`0x88888889`,
`shr 6`), multiplies in the x87 and truncates with `__ftol`. A rate of 5, price 9, count 1 and 1440 gives 0.

## Sell slot

`ClickAuctionSellItemButton()` swaps the cursor item with the sell slot, the item whose GUID is held in
`0xb72608`, using the ordinary cursor item calls and no cursor mode of its own. The new item must pass in this order,
each refusal showing an error and leaving the slot alone: bound items show `ERR_AUCTION_BOUND_ITEM`, or
`ERR_AUCTION_QUEST_ITEM` when the template's bonding is 4 or 5; items with the record flags 0x2 or 0x10000, a nonzero
`ITEM_FIELD_DURATION` or the template's two further flags show `ERR_AUCTION_CONJURED_ITEM`; a container with contents
shows `ERR_AUCTION_BAG`; a wrapped item shows `ERR_AUCTION_WRAPPED_ITEM`; a lootable item shows `ERR_AUCTION_LOOT_ITEM`.
Placing the item already in the slot clears the cursor; a change signals `NEW_AUCTION_UPDATE` (423).

Addresses: `0x4ce310`, `0x4ce362`, `0x4ce38f`, `0x4ce3e2`, `0x4ce3ec`, `0x4ce411`, `0x4ce432`, `0x4ce43d`, `0x4ce493`, `0x4ce4b1`, `0x4ce4d4`, `0x4ce523`, `0x4ce56c`, `0x4ce576`, `0x4cd640`, `0x494c80`, `0x494b60`
Confidence: verified
Proof: one
Test: none

`0x4ce362` calls `0x5da2c0` (bound), `0x4ce38f` loads the template bonding at `+0x194` and tests 4 and 5 (`0x4ce395`,
`0x4ce39a`) for id 0x16c, else 0x16d (`0x4ce3b7`). `0x4ce411 mov eax,[ecx+0x24]` is the only read of `ITEM_FIELD_DURATION` in the
image; `0x4ce3e2` and `0x4ce3ec` test the record's flags (`0x5da2a0`), `0x4ce432` and `0x4ce43d` call `0x5da500` and `0x5da570`.
`0x4ce4b1 test [eax+0x3c],8` is the wrapped bit of `ITEM_FIELD_FLAGS`. The slot is cleared by `0x4cd640`, which
unlocks the item, zeroes the GUID and the three words after it, and signals 423 (`0x4cd673`).

## Sell slot info

`GetAuctionSellItemInfo()` returns six values: name, icon, count, quality, whether the player can use it, and the
price (sell price times count). An empty slot, an unresolved item or an uncached record returns `nil, nil, 1, -1, nil,
0`. Quality is the template `+0x1c` when its `+0x2c` is nonzero, else -1. `0x4ce2f0` is the callback registered with the
item cache by this function and by `CalculateAuctionDeposit`; it signals 423 when its flag is set.

Addresses: `0x4ce590`, `0x4ce5f2`, `0x4ce601`, `0x4ce68e`, `0x4ce69d`, `0x4ce6da`, `0x4ce703`, `0x4ce726`, `0x4ce2f0`, `0x4ce2ff`, `0x5d8bc0`
Confidence: verified
Proof: one
Test: none

`0x4ce726` is the shared canned tuple (`push 0x3ff00000`, `push 0xbff00000`). `0x4ce703 imul ecx,[edi+0x28]` is the price:
the stack count `[item+0x114]+0x20` times the template sell price `+0x28`. `0x4ce2ff` is the fire in the standalone
callback; the other two producers of event 423 are `0x4ce576` and `0x4cd678`.

## Starting an auction

`StartAuction(minBid, buyout, runTime)` needs three numbers (`Usage: StartAuction(minBid, buyoutPrice, runTime)`) and
returns nothing. It does nothing without an auctioneer or with a zero sell slot GUID; a set GUID whose item object is missing still sends (`0x4ce802 je 0x4ce855`). It refuses a damaged item (not
flagged without durability, with a maximum above 0 and a current value below it) with `ERR_AUCTION_REPAIR_ITEM`
(0x177). A buyout of at least 1 below the minimum bid is raised to it. A `runTime` other than 120, 480 or 1440
does nothing. Otherwise it sends 0x256 and clears the sell slot.

Addresses: `0x4ce770`, `0x4ce95a`, `0x4ce7b5`, `0x4ce813`, `0x4ce837`, `0x4ce83f`, `0x4ce87b`, `0x4ce883`, `0x4ce8a0`, `0x4ce8c5`, `0x4ce925`, `0x4ce92a`, `0x807028`
Confidence: verified
Proof: one
Test: none

`0x4ce80f test dl,bl` tests flag 0x8 of the item flags and `0x4ce813` loads `+0xa4`; `0x4ce837 cmp ecx,[eax+0xa4]; jae`
passes a full item. `0x4ce87b` to `0x4ce883` is `test esi,esi; je; cmp esi,ebx; jae; mov esi,ebx`. The table
`0x807028` holds 120, 480 and 1440 (`0x4ce8a0`). The packet order is `0x4ce8c5` opcode, auctioneer, item GUID, minimum,
buyout, duration.

## Placing a bid

`PlaceAuctionBid("type", index, bid)` needs a string and two numbers (`Usage: PlaceAuctionBid("type", index, bid)`). It
does nothing with no row, player or auctioneer. A bid above a nonzero buyout becomes the buyout. A bid under the
minimum shows `ERR_AUCTION_MIN_BID`; one not above the current bid `ERR_AUCTION_HIGHER_BID`; with a current bid, one
under it plus the increment shows `ERR_AUCTION_BID_INCREMENT` unless it reaches the buyout. A bid over 2,000,000,000 and
over current plus increment is dropped silently. The player's money must cover the bid, or only the rise when the
player already leads (`ERR_NOT_ENOUGH_MONEY`); a unique item already carried at its limit shows error 0x12. Then it
sends 0x25a with the full bid and remembers it.

Addresses: `0x4cf610`, `0x4cf762`, `0x4cf775`, `0x4cf77a`, `0x4cf792`, `0x4cf799`, `0x4cf7b1`, `0x4cf7c6`, `0x4cf7e1`, `0x4cf812`, `0x4cf838`, `0x4cf84c`, `0x4cf88c`, `0x4cf8a8`, `0x4cf8cb`, `0x4cf915`, `0x4cf91e`, `0xb72658`
Confidence: verified
Proof: one
Test: none

`0x4cf762` clamps to the buyout, `0x4cf775 cmp edi,[esi+0x28]; jae` tests the minimum (`0x4cf77a` pushes 0x176),
`0x4cf795 cmp edi,ecx; ja` the current bid (`0x4cf799` pushes 0x175), `0x4cf7ba` and `0x4cf7c2` the increment and buyout
(`0x4cf7c6` pushes 0x174), `0x4cf7e1 cmp edi,0x77359400` the cap. The delta is `0x4cf817 sub ecx,edx`. `0x4cf8a3 cmp
eax,[ecx+0x5c]; jl` compares the carried count (`0x622130` with scope 0x4f) with the template's `+0x5c` and pushes 0x12. The
packet is the opcode, the auctioneer, the id (`rec+4`) and the bid, and `0x4cf91e` stores the bid.

## Cancelling

`CancelAuction(index)` takes the 1-based index into the owner list and sends 0x257 with the auctioneer GUID and that auction's id; an
out-of-range index or no auctioneer does nothing. When the auction has a current bid, the player must hold
at least that bid times 0.05 (the float `0x3d4ccccd`, truncated), else `ERR_NOT_ENOUGH_MONEY` is shown and nothing
is sent. With no player the error shows at once (`0x4d0301`); with a player that is not the active one the coinage reads as 0 (`0x4d031c`), so any fee of 1 or more gives the error and a bid under 20 copper, whose truncated fee is 0, passes (`0x4d0341`, `0x4d0346`). A non-number raises `Usage: SetSelectedAuctionItem(index)`, the message of another function.

Addresses: `0x4d0260`, `0x4d0279`, `0x4d02c5`, `0x4d02dc`, `0x4d0338`, `0x4d033b`, `0x4d0346`, `0x4d034a`, `0x4d0360`, `0x4d03a6`, `0x807024`
Confidence: verified
Proof: one
Test: none

`0x4d0279 push 0x84963c` is the only use of that text, which reads `Usage: SetSelectedAuctionItem(index)`. `0x4d02c5 jae` bounds
the index against `0xb72624`; `0x4d02dc cmp [edi+0x40],esi` tests for a bid; `0x4d0338 fild qword` and `0x4d033b fmul dword
[0x807024]` (0.05) feed `__ftol`, and `0x4d0346 cmp ebx,eax; jae` compares the money (`[[player+0xe68]+0xf70]`). The bid
increment is a different quantity, `rec+0x2c`, and is not used here.
