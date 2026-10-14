# ui: inventory queries and alerts

The Lua queries over the local player's inventory (`PaperDollInfoFrame.cpp`): the item count, the lock test, the
durability alerts, and how an inventory failure and the item lock reach the screen. Slots and the slot reader are
`ui/bags#slot-arguments`; the lock is `ui/cursor#the-item-lock`; the failure packet is
`objectlayer/playermessages#inventory-change-failures`.

Roots: `0x4c8680`, `0x4c8e60`, `0x4c94b0`, `0x4c9560`, `0x4c9570`, `0x4c6d30`, `0x4c6de0`, `0x4c7ee0`, `0x4c9790`

## Item counts

`GetInventoryItemCount(unit, slot)` is `0x4c8680` and always returns one number. A first argument that is neither
string nor number raises `Usage: GetInventoryItemCount(unit, slot)`; a slot the reader refuses raises
`Invalid inventory slot in GetInventoryItemCount`. The unit must resolve to a player the active player may read
(itself, or a player it can cooperate with, `0x606ba0`, within 10 yards, `0x4944a0`); else the result is 1. So is an empty slot. An item that
is not a container returns its stack count. Slot 0 (ammo) is read for the active player only, with
`PLAYER_AMMO_ID` set: the total stack count of that ammo over the equipment, equipped bags, backpack, their contents
and the keyring, not the bank; otherwise 1.

Addresses: `0x4c8680`, `0x4c86a9`, `0x4c86ca`, `0x4c86f8`, `0x4c8708`, `0x4c873f`, `0x4c8797`, `0x4c8894`, `0x4c88b6`, `0x4944a0`, `0x606ba0`, `0x48cc80`, `0x6228a0`, `0x622130`, `0x622160`
Confidence: verified
Proof: one
Test: none

The 1 is the `push 0x3ff00000` at `0x4c88b6`, which shares its tail with the 0 leg (`0x4c8813 push 0`); the unreadable
unit (`0x4c86ff`), the ammo leg for another player or no ammo (`0x4c8751`) and the empty slot (`0x4c8797`) all
jump there. `0x6228a0` returns null for a zero GUID, and `0x4c8894` pushes `fild [item+0x114]+0x20`. The range
`0x4944a0` is true when the squared distance is at most `[0xb4d918]`, which `0x48cc80` sets to 10 squared. `0x4c8763`
calls `0x622130`, which sums the stack counts of the items whose entry equals the argument, with the scope bits of
the flags applying to the player's table only.

## Container counts

A container in the slot counts differently. In slots above 22 (the bank bags) the result is 0 before any lookup. For
slots 19 to 22 the client scans `ItemSubClass.dbc` in file order for the first row whose class and subclass equal the
bag's. With no row, or with bit 0x4 of the row's `DisplayFlags` (+0x14) clear, the result is 0. Otherwise it is the
sum of the stack counts of the bag's contents, 0 when there is none. In the shipped file the bit is set for the soul
bag, quiver and ammo pouch subclasses and clear for the plain bag, so an ordinary bag shows no count.

Addresses: `0x4c87a6`, `0x4c87af`, `0x4c87b5`, `0x4c87e8`, `0x4c87ef`, `0x4c87fa`, `0x4c881a`, `0x4c8826`, `0x4c8830`, `0x4c8835`, `0x4c8813`, `0xc0db90`, `0x5d9f30`, `0x5d9f90`
Confidence: verified
Proof: one
Test: none

`0x4c87a6 test cl,1` after `shr ecx,2` is bit 2 of `OBJECT_FIELD_TYPE`. `0x4c87b3 jg` is signed on the 0-based slot
against 0x16. The rows are 0x74 bytes at `0xc0db90` (count at `0xc0db94`); `0x4c87ef` and `0x4c87fa` compare columns
0 and 1 with the class and subclass `0x5d9f30` and `0x5d9f90` return, and a class mismatch skips the subclass.
`0x4c8830` calls the container's table (vtable slot +0x10) with entry -1, the wildcard of `0x622160`.

## Item lock query

`IsInventoryItemLocked(slot)` is `0x4c8e60`, with the slot alone as argument 1. A refused slot raises
`Invalid inventory slot in IsInventoryItemLocked`. The ammo slot, no active player and an empty slot return nil;
otherwise it returns the number 1 when bit 0 of `[item+0x314]` is set and nil when it is clear.

Addresses: `0x4c8e60`, `0x4c8e7c`, `0x4c8ec0`, `0x4c8ec9`, `0x4953e0`, `0x495420`, `0x495460`, `0x622210`, `0x6223f0`
Confidence: verified
Proof: one
Test: none

`0x4c8ec9 test byte [eax+0x314],1` is the test and `0x4c8ed2` pushes the 1.0. The bit is a member of the item
object, zeroed by its constructor `0x5d81f0`. It is set by `0x4953e0` (`or 1`, `0x495409`), cleared by
`0x495420` (`0x495449`), by the sweep `0x495460`, and by `0x622210`, which `0x6223f0` runs over the inventory.

## Inventory alerts

The client keeps 12 alert statuses, recomputed whole by `0x4c7ee0` and read by `GetInventoryAlertStatus(index)`
(`0x4c94b0`), which returns `status[index - 1]` as a number, and 0 for an index outside 1 to 12; a non-number raises
`Usage: GetInventoryAlertStatus(index)`. The regions are flat slots 0, 2, 4, 5, 6, 7, 8, 9, 15, 16 and 17 (head,
shoulders, chest, waist, legs, feet, wrists, hands, main hand, off hand, ranged) and, last, ammunition. Each pass sets all
12 to 0 and fires `UPDATE_INVENTORY_ALERTS` (501) unconditionally at its end, whether or not a status changed. With
no active player it does nothing.

Addresses: `0x4c7ee0`, `0x4c94b0`, `0x4c7f16`, `0x4c7f22`, `0x4c8080`, `0x806eb8`, `0xb71f60`, `0xb71f5c`
Confidence: verified
Proof: one
Test: none

`0x806eb8` is the 12-dword slot map, ending in -1 for the ammunition region (`0x4c7f28`). The array is `0xb71f60` with count
`0xb71f5c`; `0x4c94f2 cmp eax,ecx; jae` is unsigned, so index 0 is out of range too. The loop runs `edi` from 0 to 0x2c
(`0x4c8077 cmp edi,0x30`), and the event is `mov ecx,0x1f5` (`0x4c8080`).

## Equipment alert rules

For an equipped item the status is 4 when bit 0x10 of `ITEM_FIELD_FLAGS` (descriptor +0x3c) is set. Otherwise an item
with bit 0x8 clear and `MAXDURABILITY` (+0xa4) not zero is 4 at `DURABILITY` (+0xa0) of 0 and 3 at 1 to 5; the bound
is the absolute 5, not a fraction. Any other equipped item goes to the enchantment test: with bit 0x2000 of the
item record's virtual answer clear and the temporary enchantment id (+0x4c) set, the status is 1, and 2 when that
enchantment has 30000 ms or less left. Ammunition is 3 when the active player's `PLAYER_AMMO_ID` is set and its count
(as in the item count) is 20 or less, else 0.

Addresses: `0x4c7ee0`, `0x4c7faa`, `0x4c7fc8`, `0x4c7fd4`, `0x4c7fe4`, `0x4c7fee`, `0x4c8012`, `0x4c8035`, `0x4c803b`, `0x4c8043`, `0x4c8060`, `0x4c7f78`, `0x4b5390`, `0x5d9d00`, `0x622130`
Confidence: verified
Proof: one
Test: none

`0x4c7faa test [eax+0x3c],0x10` stores 4 at `0x4c7fb6`. `0x4c7fc8` (bit 8) and `0x4c7fd4` (`MAXDURABILITY`) divert to the
enchantment test at `0x4c8035`. `0x4c7fec` jumps on a nonzero durability to `0x4c8009`, where `cmp [eax+0xa0],5; ja` sends
a value above 5 to `0x4c8035` and else stores 3 (`0x4c8027`); `0x4c7ff4` stores 4. `0x4c8035 call [edx]` is the record's slot 0;
`0x4c8060 cmp eax,0x7530; ja` leaves 1. `0x4b5390` is called with `0x24` and `0x23` for 4 and 3. The ammunition arm
`0x4c7f78 cmp ecx,eax; sbb` yields 3 for a count of 20 or less. `ITEM_FIELD_FLAGS` bit 0x10 has no name in the image.

## Alert triggers

`0x4c7ee0` runs from the Lua binding `UpdateInventoryAlerts` (`0x4c9560`), from five item and player field handlers
(`0x5d952a`, `0x5d97aa`, `0x5d9920`, `0x5d9b2f`, `0x5dee7a`), and from `0x4c6d30`, which the run-once player setup
calls once. `0x4c6d30` registers the character-points and ammunition watchers (`0x4c6fb0`, `ui/talents`), then makes
the 12-dword array of 48 bytes if the count is not 12, copying an older one over, and ends in the recompute.
`0x4c6de0` removes the watchers and frees the array.

Addresses: `0x4c6d30`, `0x4c6de0`, `0x4c6ce0`, `0x4c6cb0`, `0x4c6cd0`, `0x4c6fb0`, `0x4c7130`, `0x4c9560`, `0x49097e`, `0xb71f58`, `0x5d952a`, `0x5d97aa`, `0x5d9920`, `0x5d9b2f`, `0x5dee7a`
Confidence: verified
Proof: one
Test: none

`0x4c6cb0` zeroes the three array cells (capacity `0xb71f58`, count, pointer) at start-up and `0x4c6cd0` registers the exit freeing. `0x4c6d37` calls
`0x4c6fb0` with the active player's GUID, `0x4c6d3c` tests the count against 12, `0x4c6d59` allocates 0x30 bytes
tagged with the enum name, and `0x4c6dd6` jumps to `0x4c7ee0`. `0x4c9560` is a `call` and `xor eax,eax`. The only
caller of `0x4c6d30` is `0x49097e`, of `0x4c6de0` `0x490aa8`.

## Weapon enchant info

`GetWeaponEnchantInfo()` takes no argument and ignores any it is given. It always returns six values, three for the
main-hand item (equipment slot 15) and then three for the off-hand item (slot 16): whether the item has a temporary
enchantment (the number 1, else nil), the milliseconds left (a number, nil when the duration field is 0) and the charges
(a number). A hand with no item, an item whose record flags carry 0x2000, or an enchantment id of 0 gives three nils for
that hand, and the off hand is read either way. With no player object all six are nil.

Addresses: `0x4c9790`, `0x4c97ca`, `0x4c97e7`, `0x4c97f5`, `0x4c97fc`, `0x4c9818`, `0x4c982c`, `0x4c9852`, `0x4c9873`, `0x4c9888`, `0x4c98f3`, `0x4c993a`, `0x4c9944`, `0x4c995b`, `0x4c9965`, `0x4c998f`, `0x6228a0`, `0x5d9d00`, `0x5da2a0`, `0x6f3810`, `0x6f37f0`
Confidence: verified
Proof: one
Test: none

The item comes from `0x6228a0(0xf)` and `0x6228a0(0x10)` over the player's item list (`player+0x1d38`). The three fields
are read at `[item+0x114]` plus `0x4c`, `0x50` and `0x54`, the id, duration and charges of the temporary enchantment slot
(field block `ITEM_FIELD_ENCHANTMENT`, slot 1). Before each read the code calls the flags getter `0x5da2a0` through the
`+0x110` table and tests bit 0x2000 of the result (`test ah,0x20`); a missing item record gives flags 0, which passes.
The expiry is `0x5d9d00(1)` (`objectlayer/items#item-and-enchantment-timers`), set by the enchantment time update
(`objectlayer/inventory`). The 1 is pushed as a double (`push 0x3ff00000; push 0`). The duration field only gates; the milliseconds come from the expiry clock, so a field set without the time update
gives `1, 0, charges`. Every exit is `mov eax,6`; the main
hand's three nils fall through into the off-hand block.

## Off-hand weapon

`OffhandHasWeapon()` is `0x4c9570`. It returns the number 1 when the active player has an item in flat slot 16 whose
item class is 2 (weapon), and nil otherwise, including with no player or an empty slot.

Addresses: `0x4c9570`, `0x4c959a`, `0x4c95a5`, `0x4c95aa`, `0x5d9f30`
Confidence: verified
Proof: one
Test: none

`0x4c9592 push 0x10` is the slot, `0x6228a0` the lookup, `0x4c95aa cmp eax,2` the class test and `0x4c95af` the push of 1.0.

## Failure messages

An inventory failure shows its text through `CGGameUI::DisplayError(id, args)`, `0x496720`. An id of 465 or more does
nothing. The id's 0x14-byte entry at `0xb4b498` holds a key name (+0), a route 0 to 3 (+4), an interface sound name
(+8) and a vocal sound id (+0xc). The key is looked up as a global string with `FrameScript_GetText`, formatted
with the arguments into a 0x800 buffer, copied to `0xb4da40` and shown by the route: 0 through `0x49a870`, 1 as
`UI_INFO_MESSAGE` and 2 as `UI_ERROR_MESSAGE` through `0x4945b0`, 3 to the console. A key with no string in the interface
gives an empty text, which `0x4945b0` drops without a sound or a frame. Reason 0 and reason 59
(`ERR_CANT_BE_DISENCHANTED`) display nothing.

Addresses: `0x496720`, `0x49672c`, `0x49673d`, `0x4967b6`, `0x4967c5`, `0x4967d7`, `0x4967e9`, `0x496810`, `0x4945b0`, `0x4945b4`, `0x703bf0`, `0x704350`, `0x5e3991`, `0x5e39a9`, `0x5e39cb`, `0x5e3a27`, `0x5e3a35`, `0x622630`
Confidence: verified
Proof: one
Test: none

`0x49672c cmp eax,0x1d1; jge` is the bound, `0x4967b6` loads the key, `0x4967bd` and `0x4967c5` drop a null or empty key.
`0x703bf0` is pre-seeded with the empty string `0x882748` and its fast path `0x704350` writes the text only when the
global is a string. The route jump table is `0x496888`. The handler `0x5e3991` skips everything for reason 0
(`0x5e39a9 je`), reads a u32 only for reason 1 (`0x5e39cb`), and shows it as the `%d` of the level message
(`0x5e3a35`). Reason 59 maps to id 362, whose key `ERR_CANT_BE_DISENCHANTED` has no string in the shipped
interface, which makes the text empty. The reason table is `objectlayer/inventory#inventory-results`.

## Wrong bag type

Reason 16 shows a specific line when the named bag resolves. `0x5ede00` takes the trailing wire byte as a bag slot;
`0xff` or a slot past the table gives 0, and the generic wrong-bag line shows. Otherwise it resolves the bag as an
item, takes the localized name of its bag family (the `ItemBagFamily` store), displays error 0x118
(`ERR_WRONG_BAG_TYPE_SUBCLASS`, `Only %s can be placed in that.`) with it and returns 1, so the generic line does not
show. With no active player the generic line shows.

Addresses: `0x5e39f9`, `0x5e3a0f`, `0x5e3a18`, `0x5e3a25`, `0x5ede00`, `0x5ede8a`, `0x5da050`
Confidence: verified
Proof: one
Test: none

`0x5e3a25 jne 0x5e3a45` skips the generic `0x5e3a27` when the helper returned nonzero. `0x5ede8a` is the helper's own
`DisplayError` call.

## What locks an item

The lock is set by 16 call sites of `0x4953e0` and cleared by 20 of `0x495420`; there are no tail jumps and no
stored addresses of either. The setters are the cursor pickups (`0x4c7300`, `0x4c7b00`, `0x4f9b30`), the
move, equip and delete senders (`0x4c7c00`, `0x4897b0`, `0x489660`, `0x4f9f70`), the container uses that sell or
move an item (`0x4fa0e0`, four sites), right-click open (`0x5edc80`) and begin-wrap (`0x5edea0`). An ordinary
item use, an inventory item use and the use-item send chain set nothing. The unwrap send `0x5edd60` does not lock.

Addresses: `0x4953e0`, `0x495420`, `0x4c7300`, `0x4c7b00`, `0x4c7c00`, `0x4897b0`, `0x489660`, `0x4f9b30`, `0x4f9f70`, `0x4fa0e0`, `0x5edc80`, `0x5edea0`, `0x5edd60`, `0x5d8d00`
Confidence: verified
Proof: one
Test: none

The setter sites are `0x489761`, `0x4898bc`, `0x4c783f`, `0x4c78c8`, `0x4c7be9`, `0x4c7d62`, `0x4c7dca`, `0x4f9df0`,
`0x4f9e85`, `0x4fa0b1`, `0x4fa312`, `0x4fa34b`, `0x4fa37f`, `0x4fa418`, `0x5edcd9` and `0x5edeb5`, found as `call`s
and as dwords over the whole image. `0x4fa312`, `0x4fa34b` and `0x4fa37f` follow the sends 0x282, 0x283 and 0x1a0, and
`0x4fa418` the auto-equip `0x5e1840`; the plain use `0x4fa430` calls `0x5d8d00` with no lock. The setter bails on a
zero GUID and on an object that does not resolve; the clearer bails only on a zero GUID, and signals
`ITEM_LOCK_CHANGED` even when the object is gone.
