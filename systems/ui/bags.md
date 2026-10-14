# ui: bags and bag slots

The Lua verbs that name an inventory slot or a container, the workers behind the bag drop and pickup verbs, and the
events the bag windows run on. This is `PaperDollInfoFrame.cpp` and `ContainerFrame.cpp`; the item cursor is `ui/cursor`,
the wire senders are `objectlayer/inventory#moving-items`, and the field watcher machinery is `objectlayer/fields`.

Inventory slots are numbers of the local player's flat table: count `[player+0x1d38]` (113 for the local player), GUID
pairs at `[player+0x1d3c]`, which is the descriptor array at `+0x4a8`. The client's own (0-based) bands are equipment
0 to 18, equipped bags 19 to 22, backpack 23 to 38, bank items 39 to 62, bank bags 63 to 68, buy-back 69 to 80 and
keyring 81 to 112. The Lua verbs count from 1, so each Lua band is one higher and the ammo slot is Lua 0.

Roots: `0x4c8f00`, `0x4c8f70`, `0x4c8fa0`, `0x4f94e0`, `0x4f8530`, `0x4c81b0`, `0x4f8cc0`, `0x4f8f90`, `0x4f9410`, `0x4f93d0`

## Slot arguments

`0x4c8520(L, out, index)` is the slot reader of this file. The argument must be a number (a numeric string passes). It
is truncated toward zero, reduced by 1, and accepted when the result is -1 (ammo), 0 to 22, 39 to 62, 63 to 68 or 81
to 112; in Lua terms 0, 1 to 23, 40 to 63, 64 to 69 and 82 to 113. The backpack (Lua 24 to 39) and the buy-back
slots (70 to 81) are refused. It stores the 0-based value and returns 1; on a refusal it returns 0 and the caller raises
its own error. Nothing is clamped, and the bound 112 is the last index of the 113-entry table.

Addresses: `0x4c8520`, `0x6f34d0`, `0x6f3620`, `0x40a2b0`
Confidence: verified
Proof: one
Test: none

`0x4c8546` is the `dec eax` after the truncation. The tests run in order `0x4c8547 js`, `0x4c8549 cmp eax,0x16`,
`0x4c854e cmp eax,0x27`, `0x4c8553 cmp eax,0x3e`, `0x4c8558 cmp eax,0x3f`, `0x4c855d cmp eax,0x44`,
`0x4c8562 cmp eax,-1`, `0x4c8567 cmp eax,0x51`, `0x4c856c cmp eax,0x70`; `0x4c8572` stores. The table count `0x71` is
`0x5dd456 and eax,0x71`, written only when the object is the active player (`0x5dd420`, `0x5dd425`); any other
player gets 0.

## Container ids

`ContainerIDToInventoryID(id)` is `0x4f94e0`. A non-number raises `Usage: ContainerIDToInventoryID(containerID)`.
Otherwise the id is truncated toward zero and reduced by 1; a result below 4 (signed) gets 20 added, any other 60.
So ids 1 to 4 give 20 to 23, ids 5 to 10 give 64 to 69, the backpack 0 gives 19 and the keyring -2 gives 17; any other
id n gives n + 19 (n at most 4) or n + 59 (n at least 5). Nothing is range checked. It returns one number.

Addresses: `0x4f94e0`, `0x4f951b`, `0x4f9522`, `0x4f9524`, `0x4f9526`, `0x4f9541`
Confidence: verified
Proof: one
Test: none

`dec eax` is `0x4f951b`, `cmp eax,4` with a signed `jl` is `0x4f9522`, and the two arms are `add eax,0x3c`
(`0x4f9526`) and `add eax,0x14` (`0x4f9541`). Both arms push the `fild` of the sum and return 1.

## Bank button ids

`BankButtonIDToInvSlotID(id [, isBag])` is `0x4f8530`. A non-number first argument raises
`Usage: BankButtonIDToInvSlotID(buttonID)`. The id is truncated toward zero, and the result is id + 59 when
`lua_isnumber` accepts the second argument and id + 39 otherwise. The test is on type: `true`, `false` and `nil` all
give id + 39, a numeric string gives id + 59. One number is returned, with no range check.

Addresses: `0x4f8530`, `0x4f856c`, `0x4f8575`, `0x4f8576`, `0x4f857f`, `0x4f8584`, `0x4f8587`
Confidence: verified
Proof: one
Test: none

`0x4f856c` keeps only the low dword of the truncation. `0x4f8575 dec esi` runs before the test, `0x4f857f` adds 0x3b or
`0x4f8584` adds 0x27, and `0x4f8587 inc esi` is the join, so the constants are 59 and 39. The sum is pushed by `fild`
and `0x6f3810`, and the function returns 1. The same 59 is the second arm of `0x4f94e0`, and the stock bank window's
bag buttons (ids 5 to 10) are the callers that pass a number.

## Slot names

`GetInventorySlotInfo(name)` is `0x4c81b0`. The argument is a string or a number (turned into a string). It walks the
`PaperDollItemFrame.dbc` record store in file order and takes the first row whose `ItemSlotName` equals the argument
without regard to ASCII case, over the whole string. A match returns three values: the `SlotNumber`, the `SlotIcon`
string as stored, and the number 1 when `SlotNumber` is 18 (the ranged slot), else nil. A miss, a missing argument or
an empty store raises `Invalid inventory slot in GetInventorySlotInfo`; the name is not in the message.

Addresses: `0x4c81b0`, `0xc0da00`, `0x64a4c0`, `0x414310`, `0x54b870`
Confidence: verified
Proof: one
Test: none

The store `0xc0da00` (records at +0, count at +4) is loaded by `0x54b870` from `DBFilesClient\PaperDollItemFrame.dbc`: the header must be `WDBC` (`0x54b8c5`), 3 fields (`0x54b91e`) and a record size of 12 (`0x54b956`); each
entry is 16 bytes (`0x54b995 shl eax,4`), the three columns at +0, +4 and +8 and the row index at +0xc
(`0x54b9de`). The loop (`0x4c81f0` to `0x4c8227`) compares `[entry]` with `0x64a4c0(entry, arg, 0x7fffffff)`, a
null-guarded `_strnicmp` (`0x414310` folds both operands from `A`-`Z`), and `0x4c8264` tests `SlotNumber - 1 == 17`.
In the shipped file the equipment rows are numbered 1 to 19, the four bag slots `Bag0Slot` to `Bag3Slot` 20 to 23,
`Bag1` to `Bag12` 64 to 75 and `AmmoSlot` 0, the last row; every bag row shares one icon string.

## Using a container item

`UseContainerItem(index, slot [, onSelf])` (`0x4fa0e0`) uses the item in a bag slot. A non-number first or second
argument raises the usage text. With no player, an unresolved pair or a busy item (bit 0 of `+0x314`) it does nothing; else
it clears the cursor, always, and takes the first arm that applies. The repair cursor on a repairing vendor repairs
(0x2a8; `ERR_NOT_ENOUGH_MONEY` if the cost exceeds the money). A bank-flagged item sends 0x282, an open bank window 0x283
and an open vendor window 0x1a0, each then locking the item. Otherwise an item with an inventory type and no quest start
is auto-equipped, and anything else goes to the use ladder.

Addresses: `0x4fa0e0`, `0x4fa102`, `0x4fa116`, `0x4fa13e`, `0x4fa156`, `0x4fa17d`, `0x4fa198`, `0x4fa1a6`, `0x4fa1cd`, `0x4fa1da`, `0x4fa1e8`, `0x4fa22e`, `0x4fa23a`, `0x4fa27a`, `0x4fa2b2`, `0x4fa2f2`, `0x4fa305`, `0x4fa322`, `0x4fa33e`, `0x4fa35b`, `0x4fa378`, `0x4fa38f`, `0x4fa3b4`, `0x4fa3bd`, `0x4fa3ce`, `0x4fa3ee`, `0x4fa3ff`, `0x4fa40b`, `0x4fa428`, `0x4fa430`, `0x4fa440`, `0x4f9820`, `0x523d10`, `0x4fadb0`, `0x4faf30`, `0x5e18f0`, `0x5e19e0`, `0x5e1d50`, `0x5e1840`, `0x5d9ff0`, `0x6f1c10`, `0x5d8d00`, `0x495190`, `0x6228a0`, `0xb4e2d8`, `0xb4b3e4`
Confidence: verified
Proof: one
Test: none

`0x4f9820` turns the pair into a container list and a slot: the index less 1 is below 0 for the backpack, which uses the
player's item list (`player+0x1d38`) with 0x17 added to the slot; the bank (-2) adds 0x27 and sets the bank flag; the
keyring (-3) adds 0x51; indices 0 to 9 resolve the equipped bag object (`0x4f93e0`, `0x17b`) and those from 4 set the
bank flag. The repair arm needs cursor mode 0x11 (`0x523d10`) and a vendor that repairs (`0x4fadb0`); its packet is
the opcode, the vendor GUID and the item GUID. The use target is the current target (`0xb4e2d8`), or the player when the
third argument is true (`0x6f1c10`). The auto-equip fork tests `0x5d9ff0` (inventory type) and the quest-start word
`+0x1a8` of the record; it needs control (`0xb4b3e4`) or player flag bit 20, and locks the item after the equip unless
its class is 6.

## Putting an item in a bag

`PutItemInBag(slot)` is `0x4c8f00` and `PutItemInBackpack()` is `0x4c8f70`; both return the number 1 or nil, from the
worker `0x4c7c00(slot)` (the backpack passes 0xff through the thunk `0x4c7ed0`). The Lua slot goes through `0x4c8520`
and must be at least 19 (0-based): a bad argument raises `Invalid bag slot in PutItemInBag`. There is no upper bound,
so bank item and keyring slots pass. The worker returns 1 when the click was consumed against an existing container
and 0 otherwise.

| leg | when | does | returns |
|---|---|---|---|
| gates | no player, the control flag `0xb4b3e4` zero, or a bank bag slot with the bank closed | nothing | 0 |
| equip | the bag slot is empty or past the table | `0x4c7300(slot)` | 0 |
| same | the held item is the destination bag | clear the cursor | 1 |
| empty | nothing held and no vendor preview | clear the cursor | 0 |
| back | held from this container, and the container is not the player or the source is backpack | clear the cursor | 1 |
| swap | held from the player's own bag slot (19 to 22), or the held item is a container | lock the target, send swap | 1 |
| split | a split count `[0xb4b40c]` is pending | send 0x10e with slot 0xff | 1 |
| store | otherwise | send 0x10b | 1 |
| buy | nothing held, a vendor preview (kind 5), a vendor open, the index in range | send 0x1a3 with slot 0xff, clear | 0 |

Addresses: `0x4c8f00`, `0x4c8f70`, `0x4c7c00`, `0x4c7ed0`, `0x4c7300`, `0x4c7ca7`, `0x4c7d62`, `0x4c7d8d`, `0x4c7df5`, `0x4c7e2e`, `0x4c7e49`, `0x4c7eaa`, `0x5e0c40`, `0x5e1210`, `0x5e12e0`, `0x5e1f30`
Confidence: verified
Proof: one
Test: none

`0x4c7c75` skips the table for 0xff and takes the player's own GUID (`[[esi+8]]`), so the equip leg never runs for the
backpack, and `0x4c7d3e` jumps from there past the swap legs: a held bag is stored, never swapped. The swap sender
`0x5e0c40` sends 0x10d for two slots of the player and 0x10c otherwise. The cursor is read with `0x494c80` and
`0x495180`; every send clears with `ClearCursor(0, 1)` (`0x4c7e4e`), which leaves the item locked until the server
answers. The container test of the swap leg is bit 2 of `OBJECT_FIELD_TYPE` (`0x4c7dbe` to `0x4c7dc4`). The
destination slot of the store is never chosen here.

## Picking up a bag

`PickupBagFromSlot(slot)` is `0x4c8fa0`, which returns nothing; its argument rules are those of `PutItemInBag`
(`Invalid bag slot in PickupBagFromSlot`). The worker `0x4c7b00` has the same three gates, then clears a held item
(`ClearCursor(1, 1)`, releasing its lock). An empty slot goes to `0x4c7300(slot)`. An occupant that is not a
container (mask 4) or is locked (`[item+0x314]` bit 0) is left alone; otherwise the bag goes on the cursor
(`SetCursorItem(slot, 1, item, player, 0)`) and is locked.

Addresses: `0x4c8fa0`, `0x4c7b00`, `0x4c7b79`, `0x4c7b9b`, `0x4c7bb3`, `0x4c7bc1`, `0x4c7be2`, `0x4c7be9`
Confidence: verified
Proof: one
Test: none

`0x4c7b6a` tests the held GUID and `0x4c7b79` clears. `0x4c7bb3 mov ecx,4` selects the container mask, `0x4c7bc1` tests
the lock, and `0x4c7be2` and `0x4c7be9` set the cursor and the lock. The bank-bag gate (`0x4c7b40` to `0x4c7b55`) is the
one of `PutItemInBag`. The delegate's return value is dropped (`0x4c8fc9 xor eax,eax`).

## Pickup of an inventory slot

`PickupInventoryItem(slot)` is `0x4c8da0` and calls `0x4c7300` directly. It differs from the bag pickup in six ways:
no floor on the slot, so equipment slots and the ammo slot (-1) pass; a held item is placed or swapped, not dropped;
the occupant mask is item (2), not container (4); a bank item slot is gated on the bank as well as a bank bag slot;
its control gate also passes on bit 20 of the player's flags; and it has the item-targeting leg (a spell waiting for
an item target takes the click) and the repair-cursor leg. The ammo leg (slot -1) sends 0x268 with the held ammo's entry.

Addresses: `0x4c8da0`, `0x4c7300`, `0x4c734e`, `0x4c7366`, `0x4c73a6`, `0x4c7414`, `0x4c76c1`
Confidence: verified
Proof: one
Test: none

`0x4c7345` and `0x4c734e` to `0x4c7360` form the gate: the control flag, else bit 20 of `[[player+0x110]+0xa0]`.
`0x4c7366` to `0x4c7386` test slots `0x27` to `0x3e` and `0x3f` to `0x44` against the open bank. `0x4c73a6 cmp edi,-1`
opens the ammo leg, which needs a held item of class 6 (`0x4c73e6`) and sends 0x268 (`0x4c741a`). `0x4c76c1 mov
ecx,2` is the item mask. The item-targeting and repair legs lie between `0x4c76df` and `0x4c77a8`; the repair arm is
`ui/merchant#repair-clicks`.

## Bag slot watchers

At construction of the local player `0x4f8cc0` registers 58 watchers of 8 bytes on the player's inventory GUIDs, in
four runs, each offset `n` meaning flat slot (n - 0x4a8) / 8: bytes 0x540 to 0x558 (slots 19 to 22) and 0x6a0 to
0x6c8 (63 to 68) call `0x4f8ec0`; 0x560 to 0x5d8 (23 to 38) and 0x730 to 0x828 (81 to 112) call `0x4f8db0`. The
bank items (39 to 62) and the buy-back slots (69 to 80) get none from this file. `0x4f8f90` removes them, and the content watchers of every cached bag. The
registrations end with the zeroing of the ten-entry bag GUID cache at `0xbdd060` and a call to `0x4f91a0`.

Addresses: `0x4f8cc0`, `0x4f8ce7`, `0x4f8d17`, `0x4f8d47`, `0x4f8d77`, `0x4f8d87`, `0x4f8da6`, `0x4f8f90`, `0x467e70`, `0xbdd060`
Confidence: verified
Proof: one
Test: none

Each loop pushes the callback, a length of 8 and the active player's GUID (`0x468550`) to `0x467e70` with type 4:
`0x4f8cc2` to `0x4f8cef`, `0x4f8cf7` to `0x4f8d1f`, `0x4f8d27` to `0x4f8d4f` and `0x4f8d57` to `0x4f8d7f`, all `jle`, so
the bound itself is registered. A player's watchers fire in registration order: `0x5dd8a0` (the inventory watcher
`0x5ddcf0` of `objectlayer/playerfields`) runs before these, on every construction path.

## Bag events

A change of a bag slot runs `0x4f8ec0(offset, length, guid, previous)`. A GUID it finds in the slot is resolved as an
item; when that is a container, each of its items refreshes the action slots that hold the item's entry (`0x4e5c70`).
Then `0x4f91a0` compares bags 1 to 10 (slots n + 18 for 1 to 4, n + 58 for 5 to 10) with the cache `0xbdd060 + 8(n-1)`. A bag
whose GUID differs fires `BAG_CLOSED(n)` first when the old GUID was not zero, and, after the cache is rewritten,
`BAG_UPDATE(n)` when the new one is not; an unchanged bag fires nothing. The ending calls refresh `TRADE_SKILL_UPDATE`
and `CRAFT_UPDATE` while those windows are open. `BAG_CLOSED` has no other raiser.

Addresses: `0x4f8ec0`, `0x4f91a0`, `0x4f9249`, `0x4f9299`, `0x4f92b5`, `0x4f92c7`, `0x4f932a`, `0x4f9343`, `0x4fca20`, `0x4f60c0`, `0x4e5c70`, `0x703f50`, `0x51b293`
Confidence: verified
Proof: one
Test: none

`BAG_CLOSED` is event 329 (its name is stored at `0x51b293`), `BAG_UPDATE` 328 and `BAG_OPEN` 327, each with
the format `%d` (`0x835154`). `0x4f9245 or esi,ecx; je` skips `BAG_CLOSED` for an empty old slot, and
`0x4f92cc` skips `BAG_UPDATE` for an empty new one. Before the close, `0x4f9299` removes the departing container's
content watchers (callback `0x4f90d0`), and `0x4f932a` adds the arriving one's. A bag moved out of the slot with a
new bag in it therefore fires both events with the same number. Equipped bags fire numbers 1 to 4 and bank bags 5 to 10;
the backpack has no number in this loop.

## Container content events

`0x4f90d0` watches the item slots of one container (offset (n - 8) / 8). When the slot's GUID differs from the
previous value it unlocks the item, refreshes the action slots for the old and new entries, and refreshes the two
windows; whether or not it changed, it then raises `BAG_UPDATE` through `0x4f9370(container)`. That argument is 0 when
the container is the player and its index 1 to 10 in the cache otherwise; a container not in the cache raises nothing.
A bag sitting directly in a bank bag slot has the player as its container, so its update carries 0.

Addresses: `0x4f90d0`, `0x4f9370`, `0x4f912c`, `0x4f9186`, `0x4f918e`, `0x4f93bf`, `0x495420`
Confidence: verified
Proof: one
Test: none

`0x4f9106` bounds the slot against the container's count; `0x4f9126` is the equality skip, which lands on `0x4f9186`
and the `BAG_UPDATE` call. `0x4f9386` and `0x4f938a` compare the container with the active player and push 0
(`0x4f938c`); `0x4f9392` to `0x4f93ac` scan the ten cache entries and `0x4f93b3` pushes the index plus 1.

## Backpack and keyring events

`0x4f8db0` is the watcher of the backpack and keyring. On a change it unlocks the item, refreshes the action slots for
the old and the new entry, and refreshes the two windows. Changed or not, it then raises `BAG_UPDATE(-2)` for slots
81 to 112 and calls `0x4f9370(player)` for 23 to 38, which raises `BAG_UPDATE(0)`. `0x4f9410(container)` raises
`BAG_OPEN` (327) with 0 for the player or the cache index, once for each match, and `0x4f93d0` raises
`BAG_UPDATE_COOLDOWN` (330) with no argument.

Addresses: `0x4f8db0`, `0x4f9410`, `0x4f93d0`, `0x4f8e15`, `0x4f8e72`, `0x4f8e7c`, `0x4f8e8a`, `0x4f8ea6`, `0x4f9434`, `0x4f9469`
Confidence: verified
Proof: one
Test: none

`0x4f8e11` is the equality skip and lands on `0x4f8e72`, past the unlock and refresh. `0x4f8e72 cmp ecx,0x51; jb` and
`0x4f8e7c cmp ecx,0x70; ja` choose the keyring arm, which pushes `0xfffffffe` (`0x4f8e7e`) and signals at `0x4f8e8a`.
`0x4f9445` to `0x4f9475` scan the cache in `0x4f9410`, which keeps going after a match.

## Slot change events

`0x4c7180(guid)` is called from the item field watchers with an item GUID. It finds the item's flat slot by scanning
the table for the GUID, else for its container's GUID, so an item in a bank bag reports the bag's slot. Slots 0 to 22
raise `UNIT_INVENTORY_CHANGED("player")`; slots 39 to 68 raise `PLAYERBANKSLOTS_CHANGED("player")` (347). Then, whether
or not a slot was found, an item whose entry equals `PLAYER_AMMO_ID` of the active player raises
`UNIT_INVENTORY_CHANGED("player")` as well.

Addresses: `0x4c7180`, `0x4c7265`, `0x4c726f`, `0x4c7279`, `0x4c728d`, `0x4c72cd`, `0x4c72e0`, `0x5d87c1`, `0x5d950e`, `0x5d9694`, `0x5d96d5`, `0x5d9b2a`
Confidence: verified
Proof: one
Test: none

All four range tests (`0x4c7261` to `0x4c727c`) are signed because a miss is -1. The ammo test reads
`[[player+0xe68]+0x102c]` for the active player and 0 for any other (`0x4c72b2` to `0x4c72c7`) and compares it with
`[item descriptor+0xc]` at `0x4c72cd`. The five callers are in the item watcher and refresh chains. An item found in
slots 0 to 22 fires at `0x4c72d1` once, and falls out at the end without taking the ammo test.
