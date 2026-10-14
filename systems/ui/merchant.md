# ui: buy-back, shopping tooltips and repair

The merchant window's buy-back list, the compare tooltips it shows beside an item, and the repair service. This is
`MerchantFrame.cpp` (from `0x4facf0`) and the compare methods of `Tooltip.cpp`. The vendor payload and the
sell, buy and repair cursors are `ui/merchantcursor`; the vendor's list and its packets are
`objectlayer/inventory#vendors`; the item tooltip builder is `ui/tooltipitem`.

Roots: `0x4fafd0`, `0x4fc160`, `0x4fb310`, `0x4fb950`, `0x536080`, `0x535d70`, `0x4fadb0`, `0x4fbd60`, `0x4fbfe0`

## Buy-back list

The buy-back list is a cache rebuilt by `0x4fafd0`. It scans the player's inventory slots 69 to 80 and keeps each slot
whose GUID is set and whose entry in the 12 buy-back prices (descriptor bytes `0x1038` on) is nonzero. The kept
slots are sorted by the matching entry of the 12 timestamps (bytes `0x1068` on), the lowest first, and stored as
absolute slot numbers in `0xbddf24`, with their count in `0xbddfac`. The open vendor's GUID is `0xbddfa0`. The
prices and timestamps are player fields; the item itself is the inventory slot's.

Addresses: `0x4fafd0`, `0x4fb0c0`, `0x4fb01e`, `0x4fb02e`, `0x4fb047`, `0x4fb055`, `0x4fb083`, `0x4fb08f`, `0xbddf24`, `0xbddfac`, `0xbddfa0`, `0x73f727`
Confidence: verified
Proof: one
Test: none

`0x4fafdf` zeroes the 12 cells (`rep stos`) and `0x4fafed` the count. The loop runs byte offset `ecx` 0x228 to 0x288
(slots 69 to 80, step 8), `esi` from 0x1068 (`0x4fb01e`) step 4; `0x4fb031` bounds the slot against the table
count, `0x4fb03b` tests the GUID, `0x4fb055` the price at `esi - 0x30`. The pairs {timestamp, slot} go to a local
array sorted by `0x73f727` with the comparator `0x4fb0c0` (a first-dword compare, ascending), then the slots are
copied out (`0x4fb0a4`).

## Buy-back reads

`GetNumBuybackItems()` returns the cached count `0xbddfac`, or 0 with no vendor open. `GetBuybackItemInfo(index)`
(1-based) returns six values: the item name, the icon path, the price (the price field of the slot),
the stack count, 0 (available), and 1 when the player can use the item else nil. An index past the count, a missing
player, an empty slot or an uncached item record returns `nil, nil, 0, 1, 0, 1`; the last is the number, not a
boolean. After the window closes the cache still answers (below). A cache miss queues the item with `0x4fb570`, which signals `MERCHANT_UPDATE` (317).

Addresses: `0x4fc160`, `0x4fb310`, `0x4fb33e`, `0x4fb388`, `0x4fb393`, `0x4fb3de`, `0x4fb40c`, `0x4fb4aa`, `0x4fb4d1`, `0x4fb4f7`, `0x4fb515`, `0x4fb549`, `0x4fb570`, `0x6228a0`, `0x55ba30`, `0x5ea930`
Confidence: verified
Proof: one
Test: none

`GetBuybackItemInfo` never reads the vendor GUID `0xbddfa0` (`0x4fb310` to `0x4fb3d7`), and the close handler clears
that GUID (`0x4fae0e`) but not the count `0xbddfac`, so a stale cache still returns data after the close.
`GetNumBuybackItems` and `BuybackItem` do check the vendor.

`0x4fc166` tests the vendor GUID and `0x4fc185` loads the count. `0x4fb393` indexes the slot cache, `0x4fb3de` reads the
item with `0x6228a0`, and `0x4fb40c` asks the item cache with the callback `0x4fb570`. The price is `[values + eax*4 +
0x1038]` for `eax = slot - 69` below 12 (`0x4fb498`, `0x4fb4aa`); the count is the item's stack count
(`[item+0x114]+0x20`). The usage line of the getter is at `0x4fb33e`.

## Buying back

`BuybackItem(index)` raises `BuybackItem(index)` usage text for a non-number, and otherwise does nothing without a
vendor, a player, a cached slot or an item in it. It sends `CMSG_BUYBACK_ITEM` (0x290): the vendor GUID (u64) and
the absolute inventory slot (u32, 69 to 80, with no re-basing). It returns nothing.

Addresses: `0x4fb950`, `0x4fb968`, `0x4fb9a8`, `0x4fb9b5`, `0x4fb9eb`, `0x4fba02`, `0x4fba34`, `0x4fba3d`, `0x4fba48`
Confidence: verified
Proof: one
Test: none

`0x4fb9b5` loads the slot from `0xbddf24`; `0x4fb9eb` and `0x4fb9f1` bound it against the player's table count, and
`0x4fb9fc` tests the slot's GUID. The packet is `push 0x290` (`0x4fba02`), the GUID by `0x418370` and the slot by
`0x4181f0`; `0x4fba48` sends. The usage message is `BuybackItem(index)`.

## Shopping tooltips

`GameTooltip:SetMerchantCompareItem(slot [, offset])` is `0x536080`. It resolves the receiver like every tooltip
method (three errors for a non-table, a non-framescript object and a wrong type), and `slot` must be a number or numeric
string (`Usage: SetMerchantCompareItem("slot" [, offset])`). It returns the number 1 when a tooltip was filled and nil
otherwise, never raising for data. It fills the tooltip with the equipped item chosen below, using the item builder
with `p4` 0 and `p5` 1: the full tooltip of that item, headed by the gray `CURRENTLY_EQUIPPED` line. No delta is
computed. The call does not clear the tooltip when it returns nil.

Addresses: `0x536080`, `0x536138`, `0x536141`, `0x536161`, `0x536166`, `0x536173`, `0x53617b`, `0x536189`, `0x5361c9`, `0x5361ee`, `0x5361f5`, `0x53620e`, `0x536287`, `0x5362d4`, `0x5362d9`, `0x52b650`, `0x52e530`
Confidence: verified
Proof: one
Test: none

`0x536166` subtracts 1.0 and `0x53616c` truncates: a 0-based merchant index, valid when 0 to the item
count `0xbddfa8` minus 1 (signed) and a vendor is set. The row is `0xbdd118 + 0x1c * index` and its entry (`+4`) must be
nonzero. `0x5361ee` asks the item cache with `0x52e530`, so an uncached item returns nil on the first hover and the
query goes out. The builder call at `0x5362d4` passes the equipped item's entry, the player and item GUIDs, 0, 1 and
zeros; `0x5362d9` pushes 1.0.

## Which equipped item

The third argument `offset` (1-based, default 1) picks among the equipped items that fit. Slots 0 to 18 are scanned in
order; a slot counts when its bit is set in the mask table `0x809200` for the vendor item's inventory type
(`objectlayer/items#equipment-slots`), it holds an item, and that item's class (not subclass) equals the vendor
item's. The offset-th such slot fills the tooltip; an offset of 0 or less, or past the count, gives nil. An empty slot
does not use up the offset. Bags (19 to 22) are never reached, so a bag, ammo pouch or quiver never compares. A wand
in the ranged slot compares with a bow; a shield does not compare with a one-hand weapon.

Addresses: `0x536228`, `0x536233`, `0x536236`, `0x536239`, `0x536245`, `0x53624f`, `0x536262`, `0x53626a`, `0x536271`, `0x536275`, `0x53627d`, `0x809200`, `0x6228a0`, `0x5d9f30`
Confidence: verified
Proof: one
Test: none

`0x536228 dec eax` re-bases the offset as an integer. The loop `0x536233` to `0x536283` runs `ecx` 0 to 0x12 (`cmp ecx,0x12;
jbe`): `0x536239` loads the mask, `0x536245 shl eax,cl; test` tests the bit, `0x53624f` calls `0x6228a0(player table,
slot)`, `0x536262` the class `0x5d9f30` and `0x53626a` compares it with `[record+0]`. A match with the counter at 0
branches to the fill (`0x536271`); otherwise the counter drops (`0x536275`). The mask has no relic gate and the index
into `0x809200` is not bounds checked.

## Auction compare tooltip

`GameTooltip:SetAuctionCompareItem("type", index [, offset])` is `0x535d70`. It takes the vendor-item role from the auction
row (`owner`, `bidder` or `list`, matched without case) instead of a merchant row, and uses the same scan and builder
call. It raises its usage line unless argument 2 is a string and argument 3 a number, and any data failure returns nil.
The offset is argument 4. Its scan is a second copy of the code in `0x536080`, not a shared function. These two methods
are the only ones with "CompareItem" in their names; the engine has no knowledge of the shopping tooltip frames.

Addresses: `0x535d70`, `0x535e28`, `0x535e2f`, `0x535e3c`, `0x535e43`, `0x536065`, `0x535f98`, `0x535fef`
Confidence: verified
Proof: one
Test: none

`0x535e28` tests argument 2 with `lua_isstring` and `0x535e3c` argument 3 with `lua_isnumber`; either failure jumps to
`0x53605f`, which raises at `0x536065`. The scan is `0x535f98` to `0x535fed`, the same sequence as `0x536233` to
`0x536283` with other registers, and all data failures reach the nil push at `0x535fef`. The row arrays are the
browse, owner and bidder arrays of `ui/auction#session-state`.

## Repair capability

`CanMerchantRepair()` returns the number 1 when a vendor is open and its `UNIT_NPC_FLAGS` has bit 0x4000, else nil. It
reads the cached vendor GUID `0xbddfa0`, not the unit under the mouse. `InRepairMode()`, `ShowRepairCursor()` and
`HideRepairCursor()` are in `ui/merchantcursor#the-repair-cursor`.

Addresses: `0x4fadb0`, `0x4fba80`, `0xbddfa0`
Confidence: verified
Proof: one
Test: none

`0x4fadb0` resolves the vendor unit from the GUID with mask 8, loads `[[vendor+0x110]+0x234]` and returns
bit 14. The merchant close handler `0x4fadf0` signals `MERCHANT_CLOSED` and restores the Point cursor (`ui/merchantcursor#merchant-close`).

## Item repair cost

`0x4faf30(item)` is the price shown for one item: the base cost (`objectlayer/items#repair-cost`, `0x5da330`) times
`1 - discount`, both rounded as 32-bit floats and the product rounded to nearest even. The discount is `0x612b80(vendor,
player)`, or none without a vendor and a player. `0x4faf3f` starts the factor at 1.0 (`0x3f800000`); `0x4faf8f` applies
`fsubr` of 1.0; `0x4fafaf` stores the float product and `0x4fafb5` is the `fistp`. A base cost of 15 with a discount of 10
percent gives 14: 13.5 ties to even.

Addresses: `0x4faf30`, `0x4faf3f`, `0x4faf8a`, `0x4faf8f`, `0x4faf95`, `0x4faf9a`, `0x4fafa9`, `0x4fafaf`, `0x4fafb5`, `0x612b80`, `0x5da330`, `0x7ff9d8`
Confidence: verified
Proof: one
Test: none

`0x4faf70` loads the vendor with the unit mask 8 at the stored GUID, and `0x4faf81` and `0x4faf85` skip the discount
unless both the player and the vendor resolve. `0x7ff9d8` is the float 1.0. The discount's own reading belongs to
`objectlayer`: it is also used by the taxi node cost.

## Repair-all cost

`GetRepairAllCost()` returns the number 0 alone unless a repair vendor is open and a player exists; otherwise two
values: the total, and 1 when any counted item is damaged else nil. It sums `0x4faf30` over the equipped slots 0
to 18, the backpack slots 23 to 38 and every item inside the four equipped bags (slots 19 to 22); the bags themselves,
the bank, the keyring and the buy-back slots are not counted. An item counts when bit 1 of its object type word is
set, its flags lack 0x8 and its maximum durability is above 0. The damaged flag is any counted item with current
durability below maximum.

Addresses: `0x4fbd60`, `0x4fbd6d`, `0x4fbdd0`, `0x4fbdf1`, `0x4fbdfd`, `0x4fbe03`, `0x4fbe32`, `0x4fbe50`, `0x4fbed0`, `0x4fbef5`, `0x4fbf35`, `0x4fbf9d`, `0x4fbfb7`, `0x6228a0`
Confidence: verified
Proof: one
Test: none

The loops: `0x4fbdd0` to `0x4fbe45` (`cmp edi,0x12; jbe`), `0x4fbe50` to `0x4fbec4` (`edi` from 0x17 to 0x26) and
`0x4fbed0` to `0x4fbf87` over slots 0x13 to 0x16, each descending into the bag through its container's vtable slot
+0x10 (`0x4fbee6`, `0x4fbef5`). Eligibility is `shr ecx,1; test cl,1` on `[[item+8]+8]` (`0x4fbde4`), `test
[+0x3c],8` (`0x4fbdf1`) and the maximum (`0x4fbdfd`); `0x4fbe32 cmp eax,[esi+0xa4]; jae` leaves a full item uncounted for the flag.
The total is widened to 64 bits (`fild qword`, `0x4fbf9d`).

## Repair all

`RepairAllItems()` does nothing unless a repair vendor is open and a player exists. It sums the cost of the equipped
slots 0 to 18 only; if that exceeds the player's coin it shows `ERR_NOT_ENOUGH_MONEY` (error 0x25) and sends nothing.
Otherwise it sends `CMSG_REPAIR_ITEM` (0x2a8) with the vendor GUID and an item GUID of 0, which repairs everything.
It plays no sound; the interface plays `ITEM_REPAIR` after the call whether or not it sent.

Addresses: `0x4fbfe0`, `0x4fbfe9`, `0x4fc00c`, `0x4fc026`, `0x4fc08e`, `0x4fc091`, `0x4fc095`, `0x4fc0d5`, `0x4fc0f7`, `0x4fc107`
Confidence: verified
Proof: one
Test: none

`0x4fc08e` compares the cost with the money and `0x4fc091 jbe` passes. `0x4fc0d5 push 0x2a8` starts the packet and
`0x4fc107` sends it. The body calls no sound entry directly, as a walk over its direct calls shows.

## Repair clicks

In repair mode (locked cursor mode 0x11 and a repair vendor open), a click on an item in the four item-click Lua
functions sends `CMSG_REPAIR_ITEM` for that item. When `0x4faf30(item)` exceeds the player's coin it shows
`ERR_NOT_ENOUGH_MONEY` and plays the player's error speech line 0x28, and stops. Otherwise it plays `ITEM_REPAIR` and
sends the vendor GUID and the item's GUID; a cost of 0 sends too, and the cursor stays in repair mode.
`PickupInventoryItem` and `PickupContainerItem` reach the arm only after a held item, a held vendor row and an empty
slot are ruled out; money, spell, pet action, macro and stabled-pet payloads do not stop them. The two `Use` verbs
first clear the cursor (`ClearCursor(1, 1)`), then repair.

Addresses: `0x4c7300`, `0x4c7970`, `0x4f9b30`, `0x4fa0e0`, `0x4c7714`, `0x4c7722`, `0x4c775f`, `0x4c7764`, `0x4c7766`, `0x4c7780`, `0x4c77a8`, `0x4c79a9`, `0x4c79c4`, `0x4c7a2d`, `0x4c7a62`, `0x4f9c7b`, `0x4f9d11`, `0x4fa198`, `0x4fa1e8`, `0x4fa22e`, `0x4fa27a`, `0x458250`, `0x458030`
Confidence: verified
Proof: one
Test: none

The arm in `0x4c7300` is `0x4c7714 call 0x523d10; cmp eax,0x11`, `0x4c7722 call 0x4fadb0`, the coin `[[player+0xe68]+0xf70]`
(read only for the active player, else 0), the unsigned compare at `0x4c7764` and `jbe 0x4c777b`, the error `push 0x25`,
`ITEM_REPAIR` through `0x458030` (`0x4c7780`) and `push 0x2a8` (`0x4c77a8`). The other three arms are `0x4c79c4` to
`0x4c7a62`, `0x4f9c7b` to `0x4f9d11` and `0x4fa1da` to `0x4fa27a`. The error row 0x25 has voice id 0x28 (`0x4851c7`) and the
speech runs through `0x458250` (`ui/auctionmessages#message-routes`). `ClearCursor` is at `0x4c79a9` and `0x4fa198`; no arm
writes the base cursor mode.
