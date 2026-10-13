# objectlayer: inventory

The player's inventory and what moves through it: the slot list and Bag_C.cpp's walks over it,
equipping and the bind-on-equip confirmation, opening, reading and wrapping items, the visible
equipment on the model, vendors and trainers, loot, and the item timers. The vendor list and the
loot table are fixed globals the server's counts can overrun.

Roots: `0x5e5910`, `0x5e5f10`, `0x5e6010`, `0x5e4f30`, `0x5e0c40`, `0x5e1480`, `0x622420`

## The inventory

The player's inventory at `+0x1d38` is a count, the GUID array (the player's
`PLAYER_FIELD_INV_SLOT_HEAD` fields), the owner's GUID and a byte marking the player's own list. A
container's list has the same shape. Bag_C.cpp walks and searches these lists.

| slots | holds | walk mask |
|---|---|---|
| 0 to 18 | equipment | 0x1 |
| 19 to 22 | bags | 0x2 |
| 23 to 38 | the backpack | 0x4 |
| 39 to 62, 63 to 68 | the bank and its bags | 0x8 |
| 69 to 80 | none of the walks or counts | |
| 81 to 112 | keys | 0x40 |

Addresses: `0x5dd2e0`, `0x622420`, `0x622910`
Confidence: verified
Proof: one
Test: none

The list's `+0x10` is the own-list byte the walk widens a zero mask by (`spell/checks`). The
masks are the walk's range tests from `0x62244f` to `0x6224a5`; `0x622910` skips 69 to 80 as it
skips the equipment and the bank.

## Searching the inventory

Bag_C.cpp's searches over an inventory list:

| function | does |
|---|---|
| `0x622420` | the walk (`spell/checks`): bit 0x10 stops it entering bags; with 0x20 only items whose `ITEM_FIELD_SPELL_CHARGES` is nonzero, and bags, reach the callback |
| `0x6223f0`, `0x6222b0` | the walk with a callback, or by entry with the owner and slot out |
| `0x622130`, `0x622180` | the stack count, or the summed absolute charges, of an entry (−1 any) |
| `0x622270`, `0x6222e0`, `0x6223a0` | the first item of an entry, of a class and subclass mask, of a bag family |
| `0x6221e0` | clears bit 0 of `+0x314` on items of an entry not in trade nor held at `0xb6ef90` or `0xb72608` |
| `0x6225d0` | the slot of an item object, or −1 |
| `0x6228a0` | the item in a slot, none for a bank slot while no banker is open |
| `0x622910` | free slots: empty backpack slots plus those of each bag (item class 1) in 19 to 22 |
| `0x6229d0` | equipment durability: the sum of current over the sum of max, times 100, truncated |

Addresses: `0x622420`, `0x622515`, `0x62252e`, `0x6223f0`, `0x6222b0`, `0x622130`, `0x622180`, `0x622270`, `0x6222e0`, `0x6223a0`, `0x6221e0`, `0x6225d0`, `0x6228a0`, `0x622910`, `0x6229d0`, `0xbdd038`
Confidence: verified
Proof: one
Test: none

`0x622515` tests mask 0x20 and `0x62251a` the charges `[+0x114]+0x28`; a zero-charge item that is no
bag (slot 4 null) or with mask 0x10 is skipped (`0x62252e`). The callbacks are `0x622160` (stack
counts), `0x6221b0` (charges), `0x622210` (the lock bit), `0x6222a0` (entry), `0x622320` (class) and
`0x6223d0` (bag family). `0x6228a0` tests the banker GUID `0xbdd038` for slots 39 to 68. `0x622910`
skips 0 to 18, 39 to 112 of the player's list and recurses into bags; a container's list counts
every empty slot. `0x6229d0` skips items with `ITEM_FIELD_FLAGS` bit 0x8, divides with `fidiv` and
multiplies by 100.0 before `__ftol`, and returns 0 for a zero max.

## Inventory results

`0x622630` maps a server inventory result to an error id through a 67-entry table; a result above
66 gives `ERR_BAG_FULL` and 0 gives none.

| result | error |
|---|---|
| 1 to 3 | `ERR_CANT_EQUIP_LEVEL_I`, `ERR_CANT_EQUIP_SKILL`, `ERR_WRONG_SLOT` |
| 4, 53, 56, 62 | `ERR_BAG_FULL` |
| 8 | `ERR_PROFICIENCY_NEEDED` |
| 10, 11 | `ERR_CANT_EQUIP_EVER` |
| 9, 12, 18 | `ERR_NO_SLOT_AVAILABLE` |
| 28 | `ERR_SPELL_FAILED_REAGENTS_GENERIC` |
| 50, 51 | `ERR_INV_FULL`, `ERR_BANK_FULL` |
| 63 to 66 | `ERR_CANT_EQUIP_RANK`, `_REPUTATION`, `ERR_TOO_MANY_SPECIAL_BAGS`, `ERR_LOOT_CANT_LOOT_THAT_NOW` |

Addresses: `0x622630`, `0x622794`
Confidence: verified
Proof: one
Test: none

`cmp ecx,0x42; ja` takes the default, which returns 9; each case is `mov eax,id; ret`. The other
results: 5 `ERR_BAG_IN_BAG`, 6 `ERR_TRADE_EQUIPPED_BAG`, 7 `ERR_AMMO_ONLY`, 13
`ERR_2HANDED_EQUIPPED`, 14 `ERR_2HSKILLNOTFOUND`, 15 and 16 `ERR_WRONG_BAG_TYPE`, 17
`ERR_ITEM_MAX_COUNT`, 19 and 55 `ERR_CANT_STACK`, 20 `ERR_NOT_EQUIPPABLE`, 21 `ERR_CANT_SWAP`, 22
`ERR_SLOT_EMPTY`, 23 and 54 `ERR_ITEM_NOT_FOUND`, 24 `ERR_DROP_BOUND_ITEM`, 25 `ERR_OUT_OF_RANGE`,
26 `ERR_TOO_FEW_TO_SPLIT`, 27 `ERR_SPLIT_FAILED`, 29 `ERR_NOT_ENOUGH_MONEY`, 30 `ERR_NOT_A_BAG`, 31
`ERR_DESTROY_NONEMPTY_BAG`, 32 `ERR_NOT_OWNER`, 33 `ERR_ONLY_ONE_QUIVER`, 34 `ERR_NO_BANK_SLOT`, 35
`ERR_NO_BANK_HERE`, 36 `ERR_ITEM_LOCKED`, 37 `ERR_GENERIC_STUNNED`, 38 `ERR_PLAYER_DEAD`, 39
`ERR_CLIENT_LOCKED_OUT`, 40 `ERR_INTERNAL_BAG_ERROR`, 41 `ERR_ONLY_ONE_BOLT`, 42
`ERR_ONLY_ONE_AMMO`, 43 to 48 the six `ERR_CANT_WRAP_` errors (stackable, equipped, wrapped, bound,
unique, bags), 49 `ERR_LOOT_GONE`, 52 and 57 `ERR_VENDOR_SOLD_OUT`, 58 `ERR_OBJECT_IS_BUSY`, 59
`ERR_CANT_BE_DISENCHANTED`, 60 `ERR_NOT_IN_COMBAT`, 61 `ERR_NOT_WHILE_DISARMED`.

## Moving items

The senders take bags as GUIDs and turn each into an index of the player's list with
`CGPlayer_C::FindSlotIndex` (0xff for the player itself).

| function | opcode | body |
|---|---|---|
| `0x5e0c40` | 0x10d, or 0x10c | src slot, dst slot within the player; else dst bag, dst slot, src bag, src slot |
| `0x5e1210` | 0x10e | src bag, src slot, dst bag, dst slot, count |
| `0x5e12e0` | 0x10b | src bag, src slot, dst bag |
| `0x5e18f0`, `0x5e19e0` | 0x282, 0x283 | bag, slot of an occupied slot |
| `0x5e1ad0`, `0x5e1b40` | 0x108, 0x109 | loot slot; bag, slot, loot slot |
| `0x5e1480` | 0x10a, or 0x268 | bag, slot; or the ammo's entry |

Addresses: `0x5e0c40`, `0x5e1210`, `0x5e12e0`, `0x5e18f0`, `0x5e19e0`, `0x5e1ad0`, `0x5e1b40`, `0x5e1480`, `0x5e13b0`
Confidence: verified
Proof: one
Test: none

Every field is a u8. `0x5e0c40` and `0x5e12e0` send nothing when the destination bag is no index
and not the player. `0x5e1210` checks neither index. A zero source bag in `0x5e12e0` builds an
empty store, which `NetClient::Send` skips. `0x5e1480` sends 0x268 with
`0x495170()` while the cursor holds ammo (type 9 at `0xb4d900`) and for a projectile (class 6).

## Bind-on-equip confirmation

Equipping or swapping an item into an equipment slot asks first when the item is not yet bound,
binds on equip, and the player could use it. The move is parked in an `ITEMSWAP` table and
`EQUIP_BIND_CONFIRM` or `AUTOEQUIP_BIND_CONFIRM` is signalled with its index; an item record not
yet cached parks the move and replays it on arrival. `0x5e1be0(index, accept)` replays an
accepted move with the confirmation set, or releases both slots' items, and frees the entry.

Addresses: `0x5e0c40`, `0x5e1480`, `0x5e1110`, `0x5e1020`, `0x5e17a0`, `0x5e1be0`, `0x5e1840`, `0x5ea930`, `0xc4c290`
Confidence: verified
Proof: one
Test: none

An equipment side is the player with a slot in 0 to 22 or 63 to 68. The test is `0x5da2c0` (bound)
zero, the record's bonding `+0x194` equal to 2, and `0x5ea930` (`playercombat`). The table is
capacity `0xc4c290`, count `+4`, data `+8`, chunk `+0xc`, of 32-byte entries: source and
destination GUIDs, slots and entry; `0x5e1110` reuses an entry with no source or grows the table.
`0x5e1840` refuses an item in trade with `ERR_EQUIP_TRADE_ITEM`, else picks it up and auto-equips.

## Opening and wrapping items

`0x5edba0(item, &bag, &slot)` finds an item's bag index and slot. `0x5edc80` opens an item:
the auto-loot byte follows modifier key 0, the loot target becomes the item, and 0xac goes out
with the bag and slot; `0x5edd60` plays `UnwrapGift` and sends the same. `0x5edea0` starts
wrapping with an item, `0x5eded0` ends it, and `0x5edfc0(target)` sends 0x1d3 with the wrapper's
and the target's bag and slot, then ends it. `0x5ede00` shows `ERR_WRONG_BAG_TYPE_SUBCLASS` with
the bag family's name for result 0x10.

Addresses: `0x5edba0`, `0x5edc80`, `0x5edd60`, `0x5edea0`, `0x5eded0`, `0x5edf10`, `0x5edf30`, `0x5edfc0`, `0x5ede00`, `0xc4d778`
Confidence: verified
Proof: one
Test: none

The bag is 0xff for the player's own slots, else the container's index; the slot is the item's
index in its container. `0x5edf30` looks the container up with no null check. The wrap item is
`0xc4d778`; the cursor goes to 2 and back to 1 (`0x523c20`). The family is `0x5da050(item)` in the
`ItemBagFamily` store `0xc0dc30`.

## Visible equipment

A visible-item change (`playerfields`) calls `0x4fca20(0)`, `0x4f60c0` and `0x4c7ee0`; for the
active player it calls `0x4e5c70` on the old and new entries and signals `UNIT_ATTACK` for the
hands or `UNIT_RANGED_ATTACK_POWER` for the ranged slot. It then detaches the old display from the
model and applies the new. `0x5ed700(record, slot)` applies a display: the weapon slots 15 to 17
attach to the hands; the head and back are skipped while `PLAYER_FLAGS` hides them; the tabard
goes through `0x5e0720`; the rest go to the character component. The non-hand slots apply only
while the display is the native one. A record not yet cached counts in `+0x1d20` until it arrives.

Addresses: `0x5dee30`, `0x5ed700`, `0x5ed880`, `0x5ed8d0`, `0x5f0d60`, `0x62eb40`, `0x5ed970`, `0x5eed00`, `0x5eed50`, `0x5eedc0`
Confidence: verified
Proof: one
Test: none

`0x5f0d60(slot)` is `[+0xe68] + 0x118 + 48·slot` for slots below 19. The display id is
`0x62eb40(record)`, `ItemStats +0x18`. Hands call `CGUnit_C::AttachHandItem` and
`CGUnit_C::RebuildWeaponTrail`; the rest `SetCharacterComponentItem`. The shapeshift test compares
`+0x1f8` and `+0x1f4` (display and native display). `0x5ed8d0(item)`, from the item's own update,
reapplies each slot in 0x7c3fd holding it. `0x5ed970` signals `UNIT_ATTACK` for "player". A
weapon's enchant visual goes on through `0x5eed00(attach, hand)`, from `CGUnit_C::AttachHandItem`,
and `0x5eedc0(record, slot)`, which picks the attachment by the sheathe state and a shield;
`0x5eed50` attaches `0x62ec70(record)` at the model's child for the point.

## Helm and cloak

`0x5ee660` shows the helm on the model and `0x5ee740` hides it; `0x5ee7f0` and `0x5ee8d0` do the
same for the cloak. A show sends 0x2b9 (0x2ba for the cloak) when the flag still says hidden, a
hide when it still says shown, so the server's toggle answers with the flag the client already
shows.

Addresses: `0x5ee660`, `0x5ee740`, `0x5ee7f0`, `0x5ee8d0`
Confidence: verified
Proof: one
Test: none

The flags are `PLAYER_FLAGS` 0x400 and 0x800; the sends need the active player. The model change
is skipped while shapeshifted or without the component `+0xd30`; a hide calls `0x478bd0(slot)`.

## Vendors

`0x5e5910` dispatches 0x19f, 0x1a1, 0x1a4 and 0x1a5 for the active player. 0x19f reads the vendor
and a u8 count of 28-byte items into the 128-entry table at `0xc4c4a0`. A count above 128 writes
on past it through the globals that follow, to 255 entries, the initial-spell list header and
the vendor GUID among them. A count of 0 reads a reason and prints one of five console lines;
with reason 0 it also opens an empty window. 0x1a4 stores an item's new stock and
reports quest progress; 0x1a5 zeroes a sold-out item's stock and shows the purchase error; 0x1a1
shows the sale error and passes the item to `0x495420`.

Addresses: `0x5e5910`, `0x5db980`, `0x5db9c7`, `0x5dbabd`, `0x5dced0`, `0x5dcd60`, `0x5dd1f0`, `0xc4c4a0`, `0xc4d2a0`, `0xc4d498`
Confidence: verified
Proof: one
Test: none

The table runs from `0xc4c4a0` to 0xc4d2a0. Entry 128 lands on the initial-spell list header
`0xc4d2a0`, and entry 146 (0xc4c4a0 + 28·146) is the first to reach the vendor GUID `0xc4d498`,
which is stored before the loop (`0x5db9c7`). The loop at
`0x5dbabd` compares with the count byte alone. The window opens through `0x4facf0(0xc4c4a0, count,
guid)` with the GUID read off the message, for a count above 0 or a reason of 0. The reasons 0 to 4
switch through `0x5dbaf4` to five console lines: no inventory, a vendor that dislikes the player,
too far away, a dead vendor, a dead shopper. 0x1a5's errors: 1 and 7 `ERR_VENDOR_SOLD_OUT`, 2
`ERR_NOT_ENOUGH_MONEY`, 4 `ERR_VENDOR_HATES_YOU`, 5 `ERR_VENDOR_TOO_FAR`, 6 none, 8
`ERR_ITEM_MAX_COUNT`, 11 and 12 the rank and reputation errors, others `ERR_ITEM_NOT_FOUND`. The
senders are `0x5e1d50` (0x1a0), `0x5e1e90` (0x1a2) and `0x5e1f30` (0x1a3).

## Trainers

0x1b1 (`0x5dca60`) reads the trainer, a type and a u32 count, and sizes twelve stack arrays, ten of
four times the count and two of the count, 42 bytes a spell, all through the probe `0x40a280`. The
image reserves 0x100000 bytes of stack, so a count above about 25,000 faults in the probe, long
before four times the count wraps at 2^30. With the greeting it
opens the trainer when the GUID is the one asked (`0xc4d788`) or `0xbc3f58`. 0x1b4 prints one of
three console lines for results 0 to 2.

Addresses: `0x5e5f10`, `0x5dca60`, `0x40a280`, `0xc4d788`, `0xbc3f58`
Confidence: verified
Proof: one
Test: none

Each spell is an id, a state, a cost, two i32, a level, a skill and value, and three i32. The arrays
come from `0x40a280` with `(4·count + 3) & ~3` or `(count + 3) & ~3`. At a count of 2^30 or more the
four-times sizes wrap small, but the count-sized arrays still fault in the probe. The opening is
`0x4d7440(&guid, type)` then `0x4d7560`. The results print "Trainer service %d unavailable", "Not
enough money for trainer service %d", "Not enough skill points for trainer service %d".

## Loot

`0x5e6010` dispatches 0x160 to 0x165 for the active player and 0x29e to 0x2a4 to LootRoll.cpp. 0x160
reads the source and a type; type 0 shows the failure, others read the money and a count, capped at
16, of items, each with a slot byte placing it in the 16-entry table `0xc4d4e8`, unchecked: a slot
above 15 writes past the table, up to slot 255. A response for another source, or a failure,
releases it. 0x162 clears a slot, again unchecked; 0x161 ends the loot; 0x163 prints the money
split.

Addresses: `0x5e6010`, `0x5ea8c0`, `0x5eb900`, `0x5ec090`, `0x5ebeb0`, `0x5ebf20`, `0x5ec040`, `0x5ec030`, `0x5ebcf0`, `0x5ebc64`, `0xc4d4e8`
Confidence: verified
Proof: one
Test: none

`0x5eb900` keeps `+0x1d28`, accepts types 2 to 4 with none held, and otherwise sends 0x15f. Failures
4 to 14 map through `0x5ebc64`; 10 and 12 to 14 keep the target. An item is a u8 slot, then u32 to
`+0`, `+8`, `+4`, an i32, a u32 and a u8 into a 24-byte entry at `0xc4d4e8 + 24·slot`. `0x48f3a0`
opens the window. 0x164 reads and drops its fields; 0x165 calls `0x4c2750`. `0x5ebcf0` clears a
slot's roll byte. `0x5ebc90`, `0x5ebca0`, `0x5ebcb0`, `0x5ebcc0`, `0x5ebcd0` and `0x5ebce0` read the
entries' six dwords. `0x5ea8c0` sends 0x15e while a loot is held.

## Item timers

0x1ea gives an item's remaining time (`0x5d9c00`); 0x1eb an enchantment slot's, for an item not
yet known kept on the player's `PlayerPendingItemExpiration` list until the item arrives and
`0x5ebde0` applies it.

Addresses: `0x5e4f30`, `0x5ebd40`, `0x5ebde0`, `0x5f0e50`
Confidence: verified
Proof: one
Test: none

0x1eb reads the item, slot, time and the owner GUID; a missing item goes to the named player or
the active one through `0x5ebd40`, which links a 24-byte node to `+0x1cc8`. `0x5ebde0`, from the
item's creation, calls `0x5d9cc0(slot, seconds)` for each node of that GUID, unlinks it with `0x5f0e50`
and frees it.
