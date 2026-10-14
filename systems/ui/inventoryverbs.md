# ui: inventory, container, bank and vendor getters

The Lua functions that read what a unit wears, what a bag holds, what the bank costs and what a vendor sells, and the few that
act on them. The slot argument, the container ids and the bank button ids are `ui/bags`; counts, locks and alerts are
`ui/inventory`; the buy-back list, repair and the sell cursors are `ui/merchant` and `ui/merchantcursor`; the tooltips of
these items are `ui/tooltipitem`. Slot numbers here are Lua numbers; the reader subtracts 1.

Roots: `0x4c82a0`, `0x4c8590`, `0x4c88d0`, `0x4c8a60`, `0x4c8c10`, `0x4c8de0`, `0x4f9560`, `0x4f9670`, `0x4f9f70`, `0x4f86b0`, `0x4fb150`

## Reading a worn item

`GetInventoryItemTexture`, `GetInventoryItemBroken`, `GetInventoryItemQuality`, `GetInventoryItemCooldown` and
`GetInventoryItemLink` take a unit token and a slot. A first argument that is not a string raises `Usage: <name>(unit, slot)`; a slot the slot reader refuses raises `Invalid inventory slot in <name>`.
The unit must pass `CanReadUnitInventory`: the active player, or a cooperative player within 10 yards. Slot -1 (the ammo slot)
reads the item entry the player carries in the ammo field (`+0x102c`). A unit that fails the test, a missing item or an uncached record gives nil
(the texture, quality, broken and link getters) or 0, 0, 0 (the cooldown).

Addresses: `0x4c82a0`, `0x4c82cf`, `0x4c8301`, `0x4c8328`, `0x4c8331`, `0x4c8382`, `0x4c8590`, `0x4c88d0`, `0x4c8a13`, `0x4c8a60`, `0x4c8c10`, `0x4c8c8e`, `0x4c8ca7`, `0x4c8520`, `0x4944a0`, `0x515940`
Confidence: verified
Proof: one
Test: none

`0x4c82f8`, `0x4c85ca`, `0x4c8925`, `0x4c8ab0` and `0x4c8c68` call the slot reader `0x4c8520` with argument 2. `0x4c8328` resolves the token with `GetUnitFromToken` and `0x4c8331` tests the unit. `0x4c8382` loads the ammo item entry at `+0x102c` of the player's field block, only for the active player. `0x4c8c8e` makes the link getter answer nil for slot -1.

## Texture, quality and link

`GetInventoryItemTexture` returns the item's inventory icon path (directory, a backslash when it is nonempty, icon), from the
item in the slot of the player's own inventory or, for another player, from the visible item block of the slot (the display id
of its entry). `GetInventoryItemQuality` returns the record's quality when its inventory type is nonzero, else -1. `GetInventoryItemLink`
returns the item link, from the item object for the player and from the visible item block (entry, random property) for another
player.

Addresses: `0x4c82a0`, `0x4c88d0`, `0x4c8c10`, `0x4c8420`, `0x4c8423`, `0x4c8435`, `0x4c8449`, `0x4c8461`, `0x4c8a13`, `0x4c8a1a`, `0x4c8ce2`, `0x4c8d05`, `0x4c8d57`, `0x4c8d74`, `0x5f0d60`, `0x62eb40`, `0x5da0b0`, `0x52ae00`, `0x5d8b00`, `0x52adb0`
Confidence: verified
Proof: one
Test: none

`0x4c8423` reads the visible item block of a unit; `0x4c8435` takes its display id, and `0x4c8461` the display id of the item object's record when no block exists. `0x4c8a13` tests the record's inventory type at `+0x2c` before using `+0x1c`. `0x4c8ce2` builds the link of the player's own item and `0x4c8d57` and `0x4c8d74` that of the visible item from its entry and suffix.

## Broken and cooldown

`GetInventoryItemBroken` returns 1 for an item of the unit's own inventory with a maximum durability above 0, a durability of 0 and
no wrapped flag (bit 8 of the item flags), else nil; other players' items give nil. `GetInventoryItemCooldown` returns the start
and the duration in seconds and the enable value of the item's cooldown. It requires a use spell in the record (`+0x11c`),
and, for an item with charges (`+0x144`), a charge left (`+0x28` of the item field block). Anything else, and slot -1,
returns 0, 0, 0.

Addresses: `0x4c8590`, `0x4c8a60`, `0x4c8621`, `0x4c8628`, `0x4c8632`, `0x4c863e`, `0x4c8643`, `0x4c8b47`, `0x4c8b55`, `0x4c8b67`, `0x4c8b86`, `0x4c8b9e`, `0x4c8bbd`, `0x6e2ed0`, `0x6228a0`
Confidence: verified
Proof: one
Test: `ui::luabind_4c8a60_get_inventory_item_cooldown`

`0x4c8628` loads the flags, `0x4c8632` the maximum and `0x4c863e` and `0x4c8643` the current durability. `0x4c8b47` tests the use spell, `0x4c8b55` the charges and `0x4c8b86` calls `GetItemCooldown`. `0x4c8b9e` and `0x4c8bbd` multiply the first two by 0.001; the third is pushed as it is.

`ui::luabind_4c8a60_get_inventory_item_cooldown` runs the two cooldown scalings of `GetInventoryItemCooldown` (`fild qword`, `fmul` by 0.001, `fstp`) at `0x4c8b96` to `0x4c8ba7` and `0x4c8bb5` to `0x4c8bc6`.

## Using and cursor tests

`UseInventoryItem(slot [, onSelf])` raises `Invalid inventory slot in UseInventoryItem` for a refused slot. For slot -1 it
does nothing. Otherwise it uses the slot on the current target, or on the player when the second argument is true.
`CursorCanGoInSlot(slot)` returns 1 when the cursor item can be placed in the slot (the item check of `CheckItemForSlot`) or when the slot is -1
and the cursor kind is 9, else nil. `HasWandEquipped()` returns 1 when slot 17 holds an item whose subclass is 19, else nil.
`KeyRingButtonIDToInvSlotID(id)` returns the id plus 81.

Addresses: `0x4c8de0`, `0x4c8e18`, `0x4c8e23`, `0x4c8e37`, `0x4c8e50`, `0x4c8ff0`, `0x4c9047`, `0x4c9051`, `0x4c9079`, `0x4c99a0`, `0x4c99c2`, `0x4c99ff`, `0x4c9a04`, `0x4c8150`, `0x4c818b`, `0x4c7970`, `0x5ea720`, `0x5d9f90`
Confidence: verified
Proof: one
Test: none

`0x4c8e37` reads the second argument with `GetScriptFlagArg` and `0x4c8e50` calls `UseInventorySlot`. `0x4c9051` calls `CheckItemForSlot` with the cursor item and `0x4c9079` compares the cursor kind with 9. `0x4c99c2` requires an item table longer than 17 entries, `0x4c99ff` fetches the item of slot 17 and `0x4c9a04` compares the subclass with 0x13. `0x4c818b` adds 0x51.

## Bag contents

`GetContainerNumSlots(bag)` returns 16 for the backpack (0), 24 for the bank (-1), the slot count of the bag object for 1 to
10, and 0 for anything else, the keyring included; with no active player it returns 0 for every bag (`0x4f95ab`). `GetContainerItemInfo(bag, slot)` resolves the pair as `ui/bags` describes and,
for an item in it, returns the icon path, the stack count, 1 when it is locked (bit 0 of `+0x314`) else nil, the quality (the
record's, or -1 for an inventory type of 0) and 1 when the item can be read (a page text from its virtual `+0x74` or an item text id
at `+0x9c`) else nil. It returns nothing for an empty slot, as `GetContainerItemLink` does.

Addresses: `0x4f9560`, `0x4f95c4`, `0x4f95de`, `0x4f9625`, `0x4f9670`, `0x4f96c3`, `0x4f96d7`, `0x4f9742`, `0x4f9752`, `0x4f9794`, `0x4f97ba`, `0x4f97c7`, `0x4f9930`, `0x4f997f`, `0x4f9820`, `0x4f93e0`, `0x6228a0`
Confidence: verified
Proof: one
Test: none

`0x4f95c4` and `0x4f95de` push 16.0 and 24.0. `0x4f9625` calls the bag object's virtual `+0x10` and reads the count from the array it returns. `0x4f9742` pushes the stack count at `+0x20` of the field block, `0x4f9752` the lock and `0x4f9794` the quality. `0x4f97ba` calls the virtual at `+0x74` and `0x4f97c7` loads the text id.

## Bag names

`GetBagName(bag)` returns `BACKPACK_TOOLTIP` for 0, the name of the bag item for 1 to 10, and nil for anything else or an uncached
bag item.

Addresses: `0x4fa670`, `0x4fa6f0`, `0x4fa767`, `0x4f93e0`
Confidence: verified
Proof: one
Test: none

`0x4fa6f0` loads the global string key. `0x4fa767` reads the bag item's entry from the field block and looks its record up.

## Container cooldown and splitting

`GetContainerItemCooldown(bag, slot)` is the worn version without the use spell test: it returns the start, duration and enable value for
an item with a cooldown (an item with charges needs one left), else 0, 0, 0. `SplitContainerItem(bag, slot, amount)` needs the three numbers (else
`Usage: SplitContainerItem(index, slot, amount)`), a player who has control or the control-flag bit (bit 20 of the unit flags), an
unlocked item and an amount from 1 to the stack count. It then clears the cursor and spell targeting, puts the amount on the cursor (the whole stack when it equals the count) and locks the item.

Addresses: `0x4f99b0`, `0x4f9a48`, `0x4f9a59`, `0x4f9a7f`, `0x4f9f70`, `0x4f9fdb`, `0x4f9ff1`, `0x4fa03f`, `0x4fa05e`, `0x4fa06c`, `0x4fa078`, `0x4fa0aa`, `0x4fa0b1`, `0x4f9820`, `0x495190`, `0x494b60`, `0x4953e0`
Confidence: verified
Proof: one
Test: none

`0x4f9a48` and `0x4f9a59` test the charges. `0x4f9fdb` and `0x4f9ff1` test control and the unit flag. `0x4fa03f` refuses a locked item, `0x4fa05e` an amount below 1 and `0x4fa06c` one above the stack. `0x4fa0aa` calls `SetCursorItem` and `0x4fa0b1` `LockItem`.

## The bank

`GetNumBankSlots()` returns the number of bank bag slots the player bought (the byte at `+0x1a` of the field block) and 1 when it is 6 or
more, else nil. `GetBankSlotCost()` returns the price of the next slot from the bank slot price store, 0 without a player. `PurchaseSlot()` does
nothing without a player, control or an open banker, or with six slots bought. With less money than the price it shows
`ERR_BANKSLOT_INSUFFICIENT_FUNDS` (257); otherwise it sends `CMSG_BUY_BANK_SLOT` (0x1b9) with the interaction target. `CloseBankFrame()` closes the banker and signals BANKFRAME_CLOSED.

Addresses: `0x4f85b0`, `0x4f85dd`, `0x4f85f8`, `0x4f8630`, `0x4f8663`, `0x4f8690`, `0x4f86b0`, `0x4f86dc`, `0x4f86e9`, `0x4f8700`, `0x4f8703`, `0x4f8743`, `0x4f8748`, `0x4f8766`, `0x4f878b`, `0x4f87d0`, `0x4f87f4`, `0x4f84d0`, `0x496720`
Confidence: verified
Proof: one
Test: none

`0x4f85dd` loads the bought count and `0x4f85f8` compares it with 6. `0x4f8663` and `0x4f8712` ask `0x4f8690` for the price of slot count + 1 (`+0x4` of the store entry). `0x4f86dc`, `0x4f86e9` and `0x4f8703` are the control, banker and count tests. `0x4f8743` compares the money with the price and `0x4f8748` pushes error 0x101. `0x4f8766` pushes the opcode 0x1b9 and `0x4f878b` the interaction target.

## Vendor stock

`GetMerchantNumItems()` returns the vendor's item count and `CloseMerchant()` closes the window. `GetMerchantItemInfo(index)` returns
the item name (nil while uncached), the icon path, the price, the bundle quantity, the number available (-1 for unlimited) and 1 when the player can use the item; for an
unusable item the last is nil, and an index past the list gives nil, nil, 0, 1, 0, 1. `GetMerchantItemLink(index)` returns the link or no
value. `GetMerchantItemMaxStack(index)` returns the stack size of the item when the row's bundle quantity is 1 or less, else 1. `BuyMerchantItem(index [, quantity])` clears the cursor and
buys with the quantity from 1 to 255 (1 for a missing or below-1 number).

Addresses: `0x4fb120`, `0x4fb130`, `0x4fb150`, `0x4fb199`, `0x4fb1ba`, `0x4fb1d4`, `0x4fb245`, `0x4fb255`, `0x4fb265`, `0x4fb2a3`, `0x4fb2be`, `0x4fb580`, `0x4fb5f7`, `0x4fb670`, `0x4fb6e8`, `0x4fb71c`, `0x4fb850`, `0x4fb89f`, `0x4fb8f7`, `0x4fb93c`, `0x4faed0`, `0x5e1e90`, `0xbdd118`
Confidence: verified
Proof: one
Test: none

A row is 0x1c bytes at `0xbdd118`: the item id at `+4`, the display id at `+8`, the number available at `+0xc`, the price at `+0x10` and the bundle quantity at `+0x18`. `0x4fb245` to `0x4fb265` push the price, the quantity and the number available in that order. `0x4fb2a3` calls `CanUseItem`. `0x4fb6e8` tests the quantity and `0x4fb71c` loads the stack size at `+0x60`. `0x4fb8f7` caps the quantity at 255 and `0x4fb93c` calls `SendBuyItem`.

## Pending equips

`EquipPendingItem(index)` and `CancelPendingEquip(index)` answer the bind-on-equip confirmation parked at that index (`objectlayer/inventory`,
"Bind-on-equip confirmation"): the first accepts it and the second releases it. Each raises `Usage: <name>(index)` for a non-number and
does nothing without a player; the index is truncated, not shifted.

Addresses: `0x4898f0`, `0x489919`, `0x48994b`, `0x489960`, `0x4899bb`, `0x5e1be0`
Confidence: verified
Proof: one
Test: none

`0x489935` pushes 1 and `0x4899a5` pushes 0 as the second argument of `AnswerBindConfirm` (`0x48994b` and `0x4899bb`); the truncated index is the first.
