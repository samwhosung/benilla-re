# objectlayer: items

`CGItem_C` and `CGContainer_C`: the item object, its server record, the watchers on its fields,
using an item, its name, icon, timers, equipment slots and repair cost, and the container class
built on the item. The player's inventory, bags and the server messages about items are
`inventory`; locks are `gameobjects`.

Item_C.cpp's code passes its source path from `0x5d8300` to `0x5da7b0`; its file runs from the
storage setter `0x5d81b0` to the hash table code ending at `0x5db250`. The container class,
`0x5d76f0` to `0x5d79b0`, is a file of its own with its initializers from `0x5d79d0`, before the item file's from `0x5d7f40`, and passes no path. Item
fields are named by the client's own field table: `ITEM_FIELD_OWNER` is field 6 and
`ITEM_FIELD_MAXDURABILITY` field 47; `[item+0x114]` points at field 6, so `+0x3c` is
`ITEM_FIELD_FLAGS` (field 21).

Roots: `0x5d81f0`, `0x5d7730`, `0x5d8400`, `0x5d8d00`, `0x5d9360`, `0x5d9940`, `0x5d9290`, `0x5d88b0`, `0x5d8b00`, `0x5da330`, `0x5da5e0`, `0x5da1d0`

## The item object

The constructor builds the object part, 42 empty field watcher lists at `+0x11c`, and clears the
flags `+0x314`, the expiry `+0x320`, the seven enchantment expiries `+0x324` and the sound row
`+0x340`; it passes its GUID to 0x468270 and caches five record values as bytes. Bit 0 of
`+0x314` is the interface's item lock, bit 1 the record's arrival. The destructor cancels a
record request still pending, unlinks the lists and runs the object's.

| `+0x318` | `+0x319` | `+0x31a` | `+0x31b` | `+0x31c` |
|---|---|---|---|---|
| record `+0x0` | `+0x4` | `+0x1b0` | `+0x2c` | `+0x1b4` |

Addresses: `0x5d81f0`, `0x5d820d`, `0x5d8256`, `0x5d826e`, `0x5d82af`, `0x5d81b0`, `0x5d82d0`, `0x5d87d0`, `0x5d9f30`, `0x5d9f90`, `0x5da110`, `0x5d9ff0`, `0x5da170`, `0x6139a0`, `0x613b40`, `0x6221e0`
Confidence: verified
Proof: one
Test: none

`0x5d81f0(time, block)` calls `0x6139a0`, stores `0x80ae24` at `+0x110`, loops `edx = 0x2a` over
12-byte heads from `+0x11c`, zeros `+0x314`, `+0x320`, `+0x340`, stores the vtables, calls
0x468270 with the GUID, then `0x5d826e` on stores the five getters' results; `0x5d82af` is `rep
stos` of 7 dwords at `+0x324`. `0x5d81b0` points `+0x114` at field 6 and `+0x118` at the watch
copy past 3. The interface sets bit 0 at 0x495409 and clears it at 0x495449; Bag_C's `0x6221e0`
clears it for an entry. `0x5d87d0` calls the cache's cancel 0x55c0c0 unless bit 1 is set, unlinks
the 42 lists from the last and calls `0x613b40`; `0x5d82d0` frees on bit 0.

## Item slots

Deactivation lets go of what still names the item: a match with the GUID at 0xb71b48 calls
0x48f200, a pending cast from it is cancelled, a match with 0x494c60's GUID calls 0x495190 and
one with the item text's GUID clears that text. Without the record it cancels the request and
lowers the owner's count at `+0x1d20`. After the object's own deactivation, an item the active
player owns passes its entry and GUID to six interface calls. Slot 11 writes −1 and returns 1.
Slot 24 sends opcode 0x107 with the item's GUID while the active player is alive. The item text
handler sets `ITEM_FIELD_FLAGS` bit 0x2.

Addresses: `0x5d8660`, `0x5d9d40`, `0x5d9d60`, `0x5d9a80`, `0x5ec600`, `0x6145e0`, `0x605f30`
Confidence: verified
Proof: one
Test: none

`0x5d8660` compares the GUID with 0xb71b48 before 0x48f200(1, 0, 0), with `GetPendingCastBlock`
before `CancelCast(1, 0, 0x1c)`, with 0x494c60 before 0x495190(1, 1) and with 0xbc3fa0 before
0x4e32e0 of a zero GUID. With `+0x314` bit 1 clear it calls 0x55c0c0(entry, `0x5d8390`, item) on
0xc0e2a0 and decrements `[owner+0x1d20]`, the owner found by mask 0x10. Then `0x6145e0`; for the
active player's item 0x4fca20(0), 0x4f60c0, 0x4e5c70(entry), 0x4bfb10(guid), 0x4df7b0(entry),
0x4c7180(guid). `0x5d9d60` checks `IsDeadOrGhost` and sends 0x107 and the GUID through
`ClientServices::Send`. `0x5d9a80` is `or [fields+0x3c],2`, called from `0x5ec600`.

## The item record

An item reads its record from the item record cache by its entry. A request that misses counts
one more pending display on the owner, the player whose GUID is `ITEM_FIELD_OWNER`; the arrival
callback counts it back. Arrival sets bit 1, recaches the five bytes and the sound row: the
`ItemGroupSounds` row whose id is the `ItemDisplayInfo` row's `+0x2c`. Then, while the byte at
0xb4b424 is set and the item is the active player's, it clears the lock on the entry's items when
the record's `+0x60` exceeds 1 and signals `ITEM_LOCK_CHANGED`, else clears this item's lock
through the interface; passes the item to `0x5ebde0`; refreshes the interface; and signals
`UNIT_INVENTORY_CHANGED` for the owner.

Addresses: `0x5d8400`, `0x5d8300`, `0x5d8390`, `0x5d8440`, `0x5d84cb`, `0x5d8577`, `0x5d8590`, `0x5d85fe`, `0x55ba30`, `0x5ebde0`, `0x613af0`, `0xc0dc08`, `0xc0dbf4`
Confidence: verified
Proof: one
Test: none

The create hook `0x5d8400` calls `0x613af0` with its three arguments, then `0x5d8300`, which calls
`0x55ba30` on the cache at 0xc0e2a0 with the entry (field 3), the GUID and the callback `0x5d8390`;
a record goes to `0x5d8440`, a miss to `inc [player+0x1d20]`. `0x5d8390` finds the item (mask 2)
and, when its found byte is set, calls `0x5d8440` and decrements. `0x5d8440` sets `+0x314` bit 1,
takes the display row from `0xc0dc08` and its `+0x2c` (`0x5d84cb`) into `0xc0dbf4`. Then
`cmp [rec+0x60],1; jle` (`0x5d8577`); above, `0x6221e0(entry, 0)` (`0x5d8590`) and event 188;
else 0x495420(guid), which clears bit 0 and signals 188. Then `0x5ebde0(item)`, the refreshes
0x4fca20(0), 0x4f60c0, 0x4e5c70(entry), 0x4df7b0(entry), 0x4f9480(guid), and 0x515e50(&owner, 186)
(`0x5d85fe`).

## Record getters

Small getters read one dword of the item's record, or return 0 without one. Each asks the cache
with the item's entry and no callback.

| function | record field | consumer |
|---|---|---|
| `0x5d9f30` | `+0x0` | the item class |
| `0x5d9f90` | `+0x4` | the subclass |
| `0x5da0b0` | `+0x18` | the `ItemDisplayInfo` id |
| `0x5d9ff0` | `+0x2c` | the inventory type `0x5da1d0` indexes |
| `0x5da110` | `+0x1b0` | the `Material.dbc` id `0x5d9a50` reads |
| `0x5da170` | `+0x1b4` | cached at `+0x31c` |
| `0x5da050` | `+0x1d0` | the bag family |
| `0x5da500` | `+0x1c8` | 1 when nonzero |
| `0x5da570` | `+0x1cc` | 1 when nonzero |
| `0x5d9e70` | `+0x8` | the name, the item's vtable slot 28 |
| `0x5d9e10` | `+0x19c` | vtable slot 29, its argument the callback |
| `0x5da2a0` | `+0x20` | the flags, the `+0x110` vtable's slot |

Addresses: `0x5d9f30`, `0x5d9f90`, `0x5da0b0`, `0x5d9ff0`, `0x5da110`, `0x5da170`, `0x5da050`, `0x5da500`, `0x5da570`, `0x5d9e70`, `0x5d9e10`, `0x5da2a0`, `0x5da260`, `0x5d9a50`, `0x55ba30`
Confidence: verified
Proof: one
Test: none

Each pushes three zeros, a zero GUID and the entry `[[item+8]+0xc]`, calls `0x55ba30` on the cache
and returns the field. `0x5da260` returns the record itself; `0x5da2a0` subtracts 0x110 from `this`
and calls it. `0x5d9e10` passes the item's GUID and its argument as the callback. `0x5d9a50(id)`
returns 1 when the `Material.dbc` row's `+0x4` has bit 0.

## Item field watchers

At start-up the item code registers six type watchers on item fields; shutdown removes them and
empties the icon cache. Each watcher finds the item by GUID and acts for the active player's item.

| field | watcher | action |
|---|---|---|
| `ITEM_FIELD_STACK_COUNT` | `0x5d9400` | the record arrival's lock step, refreshes, the action-bar recount `0x4e5c70` |
| `ITEM_FIELD_SPELL_CHARGES`, 5 | `0x5d9540` | `0x4e5c70`, `BAG_UPDATE_COOLDOWN` |
| `ITEM_FIELD_ENCHANTMENT`, 21 | `0x5d95a0` | `0x5d9a90`, which calls `0x4e5c70` |
| `OBJECT_FIELD_ENTRY` | `0x5d95e0` | `0x4e5c70` for the old entry, asks for the new record, `0x4e5c70` for the new |
| `ITEM_FIELD_FLAGS`, 2 bytes | `0x5d96e0` | bit 0x10 changed on an equipped item |
| `ITEM_FIELD_DURABILITY` | `0x5d97c0` | the model's weapon and shield display |

Addresses: `0x5d9360`, `0x5d9940`, `0x5d9400`, `0x5d9540`, `0x5d95a0`, `0x5d95e0`, `0x5d96b0`, `0x5d96e0`, `0x5d97c0`, `0x5d9a90`, `0x4e5c70`, `0x5d94e1`, `0x5d9582`, `0x5d963e`, `0x5d9676`, `0x5d96bd`, `0x5d9afd`, `0x468070`, `0x468240`, `0x4f93d0`, `0x5ed8d0`, `0x47a310`, `0x5e13b0`, `0x5f0d60`, `0x5eedc0`
Confidence: verified
Proof: one
Test: none

`0x5d9360` calls `0x468070(section, offset, length, callback, 0, 0)` with (1, 0x20, 4), (1, 0x28,
0x14), (1, 0x40, 0x54), (0, 0xc, 4), (1, 0x3c, 2), (1, 0xa0, 4); `0x5d9940` removes them with
`0x468240`. These recounts run only when the owner GUID that `[item+0x114]` points to is the active player's. `0x5d9400` is the arrival's step with flag 8 to `0x6221e0`, more refreshes,
`0x4e5c70` of the entry (`0x5d94e1`) and 0x4c7ee0 for inventory type 0x18. `0x5d9540` calls `0x4e5c70` (`0x5d9582`) and `0x4f93d0`.
`0x5d95e0` recounts the old entry from the copy (`0x5d963e`), asks with the callback `0x5d96b0`, and on a cached record recounts the new
one (`0x5d9676`); `0x5d96b0` recounts on arrival (`0x5d96bd`). `0x5d9a90` recounts at `0x5d9afd`. `0x5d96e0` compares `[old] & 0x10` with the new
flags and, for slots 0 to 18 of the owner's list, calls 0x4c7ee0. `0x5d97c0`: a broken or wrapped
item (durability 0 or flags bit 0x8) maps list slot 15 or 16 to itself, 17 to 15 when the inventory
type is 0x1a, else 16, and calls `0x47a310(model, slot, +0x31c, type == 0xe)`; otherwise
`0x5ed8d0`. `0x5d9a90` does nothing for record flags bit 0x2000, else refreshes and, for slots 0 to
18, calls `0x5eedc0(0x5f0d60(slot, slot))`.

## Using an item

`0x5d8d00(target, confirmed)` uses an item through a ladder of tests; each refusal shows its error
and returns 0.

| test, in order | result |
|---|---|
| a container, lock bit 0, no record or player | 0 |
| record flags 0x200 | unwrap (`0x5edd60`) when wrapped, else start wrapping (`0x5edea0`) |
| record `+0x1a8` | `0x5eab80(&guid, value)` |
| `0x5ea930` refuses | its error; error 2 with the record's `+0x3c` |
| record `+0x19c` or `ITEM_FIELD_ITEM_TEXT_ID` | 0x4e32e0(&guid, 0) |
| client locked out, no fear, confusion or charm spell | `ERR_CLIENT_LOCKED_OUT` |
| record flags 0x4 / 0x2000 | open (`0x5edc80`) / `0x5eef40(guid)` |
| no use spell | 0 |
| an equippable item | the equip tests (next entry) |
| shapeshifted, the form not allowing items | `ERR_NO_ITEMS_WHILE_SHAPESHIFTED` |

Addresses: `0x5d8d00`, `0x5d8d26`, `0x5d8d32`, `0x5d8d8c`, `0x5d8dcc`, `0x5d8e0e`, `0x5d8e43`, `0x5d8e75`, `0x5d8eaa`, `0x5d8f76`, `0x5d8fb8`, `0x5d9105`, `0x5edd60`, `0x5edea0`, `0x5eab80`, `0x5ea930`, `0x5edc80`, `0x5eef40`, `0x6e9f20`, `0x6e9f40`, `0x6e9f60`, `0x60e9f0`, `0x496720`, `0xb4b3e4`, `0xc0d76c`
Confidence: verified
Proof: one
Test: none

`0x5d8d26` tests bit 2 of field 2 (a container's type mask). The locked-out test reads
`0xb4b3e4`; with it clear, the use spell must pass `0x6e9f20` with unit flags bit 0x800000,
`0x6e9f40` with 0x400000, or `0x6e9f60` with `UNIT_FIELD_CHARMEDBY` set. The error code starts at
0x1d1 before `0x5ea930`. The form test (`0x5d9105`) reads byte `+0x212` of the unit fields into
`SpellShapeshiftForm` (`0xc0d76c`) and refuses unless the row's `+0x2c` has bit 0x8, or bit 0x40 with
an inventory type.

## Item use gates

An equippable item with a use spell must be held by the active player in one of the first 23
slots of its list, not broken, and not a disarmed hand's. Then a use spell whose record's `+0x1d8`
is set and whose aura is on the player with bit 0 of its aura flags is cancelled instead of
cast. A bind-on-use item (`+0x194` = 3) not yet bound
asks first: unless `confirmed`, it keeps the item and target GUIDs and signals
`USE_BIND_CONFIRM`; the confirmation `0x5d9290` uses the kept item with `confirmed` 1. Otherwise
`CastSpellAsActivePlayer` casts the use spell with the item at the target.

Addresses: `0x5d8fc7`, `0x5d9027`, `0x5d907d`, `0x5d90fc`, `0x5d9157`, `0x5d91d3`, `0x5d91f2`, `0x5d9249`, `0x5d9290`, `0x5d8c00`, `0x5d8c80`, `0x5da2c0`, `0x6e7040`, `0x6e5a90`, `0xc4c240`, `0xc4c1d0`
Confidence: verified
Proof: one
Test: none

`ITEM_FIELD_CONTAINED` must be the active player, else `ERR_MUST_EQUIP_ITEM` (0x9d); an unwrapped
item with a positive `ITEM_FIELD_MAXDURABILITY` and durability 0 gives `ERR_MUST_REPAIR_DURABILITY`
(`0x5d9027`). With unit flags bit 0x200000, `ERR_CANT_USE_DISARMED` refuses list slot 15 when the
player's vtable slot 38 with (0, 1) returns a block starting with 2, and slot 16 when that is not so
and the call with (1, 1) does; slots from 23 give 0x9d (`0x5d90fc`); an item missing from the list
passes. The aura scan walks 48 `UNIT_FIELD_AURA` slots for the spell with nibble bit 0 in
`UNIT_FIELD_AURAFLAGS` and calls `0x6e7040`. `0x5da2c0` is the bound test; `0x5d91f2` stores the
GUIDs and signals 290. `0x5d8c00` and `0x5d8c80` find the first of five record spells (`+0x11c`)
whose trigger (`+0x130`) is 0.

## Bound items

An item is bound when bit 0 of its flags is set, or when, for a record without flag 0x2000, one of
its seven enchantments names a `SpellItemEnchantment` row whose `+0x5c` has bit 0.

Addresses: `0x5da2c0`, `0x5da2d5`, `0x5da300`, `0xc0d7d0`
Confidence: verified
Proof: one
Test: none

`0x5da2c0` tests `[fields+0x3c]` bit 0, calls the `+0x110` vtable's slot for the record flags and
`test ah,0x20`, then walks `ITEM_FIELD_ENCHANTMENT` in 12-byte steps, the id first, into the store
`0xc0d7d0`. Its callers include the equip, autoequip and use confirmations and the interface.

## Item names

An item's name is the record's name, or, with a random property id whose
`ItemRandomProperties` row has a suffix in the client's locale, the global string
`ITEM_SUFFIX_TEMPLATE` formatted with the name and the suffix. A wrapped item names no property.

Addresses: `0x5d8b00`, `0x5d8bc0`, `0x5d8b78`, `0x703bf0`, `0xc0dbcc`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

`0x5d8b00(buffer, size, entry, property)` clears the buffer, gets the record and, for a property in
`0xc0dbcc`'s range with a non-empty string at `+0x1c + 4·locale`, calls `SStrPrintf(buffer, size,
FrameScript_GetText("ITEM_SUFFIX_TEMPLATE"), name, suffix)` (`0x5d8b78`); otherwise `SStrCopy`. No
record leaves it empty. `0x5d8bc0` passes `ITEM_FIELD_RANDOM_PROPERTIES_ID`, or 0 when
`ITEM_FIELD_FLAGS` has bit 0x8.

## Inventory icons

An icon is the `ItemDisplayInfo` row's `+0x14` name, cached by display id in a hash table of
`INVENTORYART` nodes. A display without a row or a name logs `NOINVENTORYICON` and gives
`INV_Misc_QuestionMark`.

Addresses: `0x5d88b0`, `0x5d8af0`, `0x5d8a90`, `0x5d8950`, `0x5d8927`, `0xc4c1f4`, `0x5d7f70`, `0x5d8040`, `0x5da7b0`, `0x5da820`, `0x5da8e0`, `0x5da9a0`, `0x5dad50`, `0x5daa60`, `0x5daaa0`, `0x5dab30`, `0x5daba0`, `0x5dad20`, `0x5daf40`, `0x5daf60`, `0x5daf90`, `0x5db050`, `0x5db100`, `0x5db110`, `0x5db250`
Confidence: verified
Proof: one
Test: none

`0x5d88b0(display)` returns `0x5d8a90`'s cached string, else reads the row, passes the name through
0x44b2c0 and, when that returns 1, 0x44b320 into a 260-byte buffer, inserts it with `0x5d8950` and
returns the cached copy. `0x5d8927` prints `NOINVENTORYICON|%d` at level 2. `0x5d8af0` is the item's
icon. The table at `0xc4c1f4` starts with mask −1; the first insert makes four buckets (mask 3).
Each node holds the id, a byte from 0xc4c234 at `+0x14` and an `SStrDup` copy at `+0x18`.
`0x5dad50` doubles the buckets, up to mask 0x1fff, when an insert walks more than 13 nodes. The
node and bucket bodies from `0x5daa60` to `0x5db250` match the core's list and bucket helpers
instruction for instruction.

## Item and enchantment timers

An item keeps one expiry and seven enchantment expiries as clock times in milliseconds. Setting
one stores now plus 1000 times the given count, or 0 for a count of 0 or less. Reading one gives
the expiry minus now while that difference is positive, else 0.

Addresses: `0x5d9c00`, `0x5d9c60`, `0x5d9cc0`, `0x5d9d00`, `0x42c010`
Confidence: verified
Proof: one
Test: none

`0x5d9c00(n)` stores `OsGetAsyncTimeMs() + n·1000` (`imul esi,esi,0x3e8`) at `+0x320`;
`0x5d9cc0(slot, n)` at `+0x324 + 4·slot`. `0x5d9c60` and `0x5d9d00(slot)` return 0 for an unset
expiry, else `sub edx,eax; jns` returns 0 once now reaches the expiry and `expiry − now` before it.

## Equipment slots

`0x5da1d0(slot, flag)` says whether an item fits an inventory slot: slot −1 takes item class 6;
slots 23 and up take anything; other slots test bit `slot` of a mask per inventory type, and slot
17 first requires `flag` to equal whether the inventory type is 0x1c.

Addresses: `0x5da1d0`, `0x809200`
Confidence: verified
Proof: one
Test: none

The mask is `test [eax*4+0x809200], 1 << slot`. The table holds, for inventory types 0 to 28: 0,
0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0xc00, 0x3000, 0x18000, 0x10000,
0x20000, 0x4000, 0x8000, 0x780000, 0x40000, 0x10, 0x8000, 0x10000, 0x10000, 0, 0x20000, 0x20000, 0,
0x20000. Four interface functions read the table too.

## Repair cost

The repair cost of a damaged item is the missing durability times a quality multiplier times a
per-subclass cost, rounded half away from zero, at least 1. Only weapons (class 2, subclasses 0 to
21) and armour (class 4, subclasses 0 to 8) cost anything; a wrapped item, one without maximum
durability, or one at full durability costs 0. A `DurabilityCosts` row is 30 dwords: the id, 21
weapon costs and 8 armour costs. So weapon subclass 21 reads armour column 0, and armour subclass 8
reads past the row, the next row's id.

Addresses: `0x5da330`, `0x5da3c5`, `0x5da42f`, `0x5da449`, `0x5da450`, `0x5da4ab`, `0x5da4d4`, `0xc0ddfc`, `0xc0dde8`
Confidence: verified
Proof: one
Test: `objectlayer::item_randprop_delta_difftest`

The cost row is `DurabilityCosts` at the record's `+0x38` (`0x5da3c5`); armour reads `+0x58 +
4·subclass`, weapons `+0x4 + 4·subclass`. The multiplier is the float at `+0x4` of
`DurabilityQuality` row number 2·quality + 1 (quality `+0x1c`, `0x5da450`), its records indexed
directly. `0x5da4ab`: `fild` of maximum minus current durability, `fmul` the multiplier, `fimul`
the cost; then ±0.5 by the sign test `test ah,0x41` (zero and NaN subtract) and `__ftol`.
`0x5da4d4` replaces an unsigned result below 1 with 1. The loader 0x543e20 requires 30 columns
(0x543ece) and 0x78-byte records (0x543f06); `cmp eax,0x15; ja` at `0x5da440` lets subclass 21 read
`[row+0x58]`, and `cmp eax,8; ja` at `0x5da426` lets subclass 8 read `[row+0x78]`. Slices: item_randprop_delta_difftest `0x5da4ab`-`0x5da4d4`.

## The container

`CGContainer_C` is an item with 74 more watcher lists at `+0x350` and a slot list at `+0x6c8` in
the inventory's shape: `CONTAINER_FIELD_NUM_SLOTS`, the address of the slot GUIDs, the
container's GUID and a byte 0. Its reactivation reactivates each slot's item; its slot 4 returns
the list.

Addresses: `0x5d7730`, `0x5d76f0`, `0x5d77d0`, `0x5d7890`, `0x5d7f30`, `0x5d78e0`, `0x4644f0`
Confidence: verified
Proof: one
Test: none

`0x5d76f0` runs the item's storage setter and points `+0x348` at field 48 and `+0x34c` 35 past the
watch copy. `0x5d7730` loops `0x4a` heads, then stores `[+0x348]` at `+0x6c8`, `+0x348 + 8` at
`+0x6cc`, the GUID at `+0x6d0` and 0 at `+0x6d8`. `0x5d77d0` unlinks the 74 lists, runs the item's
destructor body and frees on bit 0. `0x5d7890` calls `0x614640` and `0x4644f0(guid)` for each
non-zero slot GUID. `0x5d78e0` is an empty slot body.

## Unreached item code

The container and item functions the [census](overview.md#unreached-functions) finds unreached
are small field copies and level-to-offset maps, a material test, a visual lookup and constant
returns.

Addresses: `0x5d78f0`, `0x5d7900`, `0x5d7910`, `0x5d7920`, `0x5d7930`, `0x5d7940`, `0x5d7960`, `0x5d7970`, `0x5d7980`, `0x5d79b0`, `0x5d92e0`, `0x5d9310`, `0x5d9340`, `0x5d9a40`, `0x5d9b70`, `0x5d9ed0`, `0x5da250`
Confidence: verified
Proof: one
Test: none

`0x5d7980`, `0x5d92e0` and `0x5d9310` copy into the sections; `0x5d92e0` passes an index below 6 to
`CGObject_C::SetUpdateField`. `0x5d79b0` maps 0, 1, 2 to 0, 0x18, 0xc0 and `0x5d9340` maps 0, 1 to 0, 0x18, other values
to −1; `0x5d9b70` returns −1 for a display with an `ItemVisuals` row, else the first
enchantment's `+0x58`.
