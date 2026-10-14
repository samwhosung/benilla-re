# ui: the vendor cursors

What the merchant window does with the cursor: `PickupMerchantItem` and its two meanings, the vendor
payload (kind 5 of `ui/cursor`) that buys when dropped, and the pointer pictures the sell, buy-back and
repair cursors show.

This is `MerchantFrame.cpp` and the bag and character-sheet bindings that share its cursors. The vendor's list and
its messages are `objectlayer/inventory#vendors`; the pointer pictures are `ui/worldcursor`.

Roots: `0x4fb760` (PickupMerchantItem), `0x4fbab0` (ShowMerchantSellCursor), `0x4fbbb0` (ShowBuybackSellCursor),
`0x4fa460` (ShowContainerSellCursor), `0x4c90b0` (ShowInventorySellCursor), `0x4fbcc0` (ShowRepairCursor),
`0x4fadf0` (merchant close)

## Picking up a merchant item

`PickupMerchantItem(index)` is two verbs, chosen by the cursor. With no active player it does nothing. With an
item on the cursor (kind 1) it sells it, whatever the index: it sends the sell message (opcode 0x1a0) to
the open vendor with the vendor's GUID, the item's GUID and the split count as a byte, then clears the cursor
with `unlock` 0, so the item stays locked until the server answers. It does not test that a merchant is open, so
with none it sends a vendor GUID of 0. It returns no values.

Addresses: `0x4fb760`, `0x494c60`, `0x5e1d50`, `0x495190`, `0xbddfa0`
Confidence: verified
Proof: one
Test: none

`0x4fb787` takes the item GUID from `0x494c60` and `0x4fb790` forks on it. `0x4fb7a7` calls the sender `0x5e1d50` (opcode
0x1a0, vendor GUID, item GUID, count byte) and `0x4fb7ac` clears with `ecx` 0 and `edx` 1.

## Grabbing a merchant row

With anything but an item on the cursor, `PickupMerchantItem(index)` grabs a row. The argument must be a number (a
numeric string passes), truncated toward zero and reduced by 1. A bad argument, an index outside the item count
(compared signed), no open merchant, or a row whose slot word `+0` is 0 each clear the cursor and return.
Picking the row whose entry is already the preview entry clears the cursor too. Otherwise it puts a kind 5
preview on the cursor: entry `+4`, display id `+8`, the zero-based row index. It raises no error.

Addresses: `0x4fb760`, `0x495170`, `0x4950f0`, `0xbdd118`, `0xbddfa8`
Confidence: verified
Proof: one
Test: none

`0x4fb7e1 js` and `0x4fb7e9 jge` are the signed index tests, `0x4fb7f7` the merchant test, `0x4fb808` the slot word test,
`0x4fb818` the toggle test against `0x495170`. `0x4fb822` is the only place that makes a kind 5 payload. The rows
are 0x1c bytes at `0xbdd118`, counted in `0xbddfa8`.

## Buying by dropping

A kind 5 preview buys when it is dropped on a bag slot, a bank or key ring slot, a paper-doll slot, or a bag
button (`PutItemInBag`, `PutItemInBackpack`). Each of these three drop paths sends the buy-in-slot message
(opcode 0x1a3: vendor GUID, the row's entry, the target bag's GUID, the target slot, a count of 1) and then clears
the cursor. The bag and keyring path sends the flat slot; the paper doll sends the player's own GUID and the
equipment slot; the bag button sends slot 255, any free slot. With no open merchant a drop leaves the payload on
the bag and paper-doll paths and clears it on the bag button path.

Addresses: `0x4f9b30`, `0x4c7300`, `0x4c7c00`, `0x5e1f30`, `0x495180`, `0x4fadf0`
Confidence: verified
Proof: one
Test: none

`0x4f9efa`, `0x4c78fd` and `0x4c7e66` test the kind for 5; the three senders are `0x4f9f3c`, `0x4c7948` and `0x4c7eaa`, each
with `dl` 1 for the count and the entry from the row (`0x4f9f35`, `0x4c7943`). The row index comes from `0x495180`.
Any clear, a drop on the world and closing the merchant drop the payload with no packet. `0x4f9f0f` and `0x4c7916` return when no vendor GUID is held, and `0x4c7e7f` jumps to the clear at `0x4c7eaf`.

## Merchant close

`CloseMerchant()` runs `0x4fadf0`. With a vendor open it clears the interaction unit, zeroes the vendor GUID and the item
count, and signals event 0x13e (`MERCHANT_CLOSED`). Then, whether or not a vendor was open, it clears a held kind 5
preview, and returns the base cursor to Point (1) when it was the repair cursor (17).

Addresses: `0x4fadf0`, `0x4931c0`, `0x495170`, `0x523d10`, `0x523d20`, `0x523c20`
Confidence: verified
Proof: one
Test: none

`0x4fadf5` tests the vendor GUID and `0x4fadfb` jumps to the end when it is 0, so the clears below it run only with a
vendor open: `0x4fae31` tests the kind, `0x4fae3a` the entry, `0x4fae4f` the base cursor against 0x11.

## Lua view of the payload

A kind 5 payload is invisible to the cursor tests. `CursorHasItem` is true for kinds 1 and 9 only,
`CursorHasSpell` for 3, `CursorHasMoney` for 2, and `CursorCanGoInSlot(slot)` finds no item for it. Nothing in the
Lua API reports kind 5. A client that models it as an item makes every drop handler that tests `CursorHasItem` act on it.

Addresses: `0x4895d0`, `0x489600`, `0x489630`, `0x4c8ff0`
Confidence: verified
Proof: one
Test: none

`0x4895d0` compares the kind with 1 and 9 only (`ui/cursor#cursor-functions`). `0x4c8ff0` reads the item through `0x494c60`,
which is 0 for any kind but 1.

## Bag and sheet sell cursors

`ShowContainerSellCursor(bag, slot)` and `ShowInventorySellCursor(slot)` set the pointer picture to Buy (3) for
an item that exists and is not locked, and change nothing otherwise. Both do nothing when a spell is awaiting a
target or the base cursor is not Point (1). A non-number raises a usage error in the container form; the
inventory form raises `Invalid inventory slot in ShowInventorySellCursor` for a non-number or an out-of-range
slot, and does nothing for slot -1. Neither sends a packet or changes the payload.

Addresses: `0x4fa460`, `0x4c90b0`, `0x6e48a0`, `0x523d10`, `0x523c20`, `0x4f9820`, `0x4c8520`, `0x6228a0`
Confidence: verified
Proof: one
Test: none

`0x4fa469` and `0x4c90b7` test spell targeting (`0x6e48a0`); `0x4fa472` and `0x4c90c4` read the base cursor.
`0x4fa4a6` resolves the bag and slot (`0x4f9820`) and `0x4c90db` the slot (`0x4c8520`); `0x4fa4bf` and `0x4c9133`
test the item's lock bit before the call to `0x523c20` with 3.

## Merchant sell cursors

`ShowMerchantSellCursor(index)` and `ShowBuybackSellCursor(index)` have the same two opening gates and raise a
usage error for a non-number. The merchant form needs an open vendor and a row in range with a nonzero entry. It
shows Buy when the player's coinage is at least the row's price at `+0x10`, else UnableBuy (23), also when the
active player does not resolve. The buy-back form reads the inventory slot from the buy-back table and shows
UnableBuy when the coinage is below that slot's buy-back price, else Buy; a slot outside the buy-back range gets
Buy. Neither sends a packet.

Addresses: `0x4fbab0`, `0x4fbbb0`, `0xbddfac`, `0xbddf24`
Confidence: verified
Proof: one
Test: none

`0x4fbb06 js` and `0x4fbb0d jge` bound the merchant index, `0x4fbb87 cmp` and `0x4fbb90 jae` compare coinage
with the price. `0x4fbc08` bounds the buy-back index by `0xbddfac` (unsigned) and `0x4fbc13` reads the slot from the
table at `0xbddf24`; `0x4fbc85` subtracts 0x45 and `0x4fbc93` compares with `[fields+0x1038+4*n]`, valid for n
below 12.

## The repair cursor

`ShowRepairCursor()` does nothing while a spell awaits a target or when no vendor that repairs is open. Otherwise it
clears the cursor payload, sets both the base and the shown picture to Repair (17), and returns nothing.
`HideRepairCursor()` sets both back to Point (1) when the base is Repair. `InRepairMode()` returns the number 1 when
the base is Repair and nil otherwise; `CanMerchantRepair()` returns 1 when the open vendor's NPC flags have bit 14
and nil otherwise.

Addresses: `0x4fbcc0`, `0x4fbd00`, `0x4fbd30`, `0x4fba80`, `0x4fadb0`
Confidence: verified
Proof: one
Test: none

`0x4fadb0` looks up the vendor unit by the GUID `0xbddfa0` and tests `[fields+0x234]` shifted by 14. `0x4fbcd9` clears with
both arguments 1, `0x4fbce3` sets the base cursor and `0x4fbced` the shown one.

## Client faults

Dropping a kind 5 payload on a bag slot or a bag button looks the row up with the stored index bounded against the row count, but an index out of range gives a null row and the
lookup is used anyway. A fresh vendor list can shorten the table without clearing the cursor. The
bag and keyring path and the bag button path then read the entry from address 4. The toggle test of
`PickupMerchantItem` compares the preview entry without testing the kind, so a held action bar item or ammo with the
same entry as the row clears the cursor instead of picking up the row.

Addresses: `0x4f9f1c`, `0x4f9f24`, `0x4f9f30`, `0x4f9f35`, `0x4c7e9a`, `0x4c7ea5`, `0x4fb811`, `0x4facf0`
Confidence: inferred
Proof: none
Test: none

`0x4f9f1c` and `0x4f9f24` bound the index; `0x4f9f30 xor eax,eax` is the out-of-range branch of the row lookup and `0x4f9f35` reads `[eax+4]`; `0x4c7e9a` and `0x4c7ea5`
have the same pair. `0x4c78fd`'s paper-doll path bails to its exit instead (`0x4c791d`, `0x4c7925`). `0x495170` reads the entry
without the kind. `0x4facf0` replaces the rows and the count and calls no clear.
