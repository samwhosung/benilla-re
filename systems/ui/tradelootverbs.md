# ui: trade window and loot roll verbs

The Lua functions of `TradeFrame.cpp` and `LootFrame.cpp` that describe the two sides of the trade window and drive its
slots, and the three that describe and answer a loot roll. The trade state they read is `objectlayer/trade`, the status
messages `ui/trade`, the roll state and the vote `objectlayer/lootroll`, and the loot rows `ui/loot`.

Roots: `0x4bfdf0`, `0x4c00f0`, `0x4c0360`, `0x4c0450`, `0x4c0650`, `0x4c0700`, `0x4c0750`, `0x4c3050`, `0x4c31f0`,
`0x4c32d0`, `0x4c3370`, `0x4c33e0`

## Describing a trade slot

A trade has seven slots, the seventh one the trade will not carry out. `GetTradeTargetItemInfo(index)` describes the
partner's item in slot index (1 to 7) and `GetTradePlayerItemInfo(index)` the player's own; both raise
`Usage: Get<Side>ItemInfo(index)` for a non-number. They return the item name, the icon path, the count, the quality (the
record's quality when its field `+0x2c` is nonzero, else −1) and, for the partner only, 1 or nil for whether the active
player can use the item. The last value is nil, or for slot 7 the enchant's name. An empty slot or an
unknown item gives nil, nil, 0, 0, nil for the player's side and nil, nil, 0, 0, nil, nil for the partner's.

Addresses: `0x4c00f0`, `0xb7150c`, `0x4c0113`, `0x4c0148`, `0x4c014b`, `0x4c015a`, `0x4c01a1`, `0x4c0224`, `0x4c0241`, `0x4c0290`, `0x4c02b0`, `0x4c02d7`, `0x4c02f6`, `0x4c0335`, `0x4c0450`, `0x4c04a8`, `0x4c04b3`, `0x4c0529`, `0x4c055d`, `0x4c05a0`, `0x4c05c6`, `0x4c0605`, `0x4c0613`, `0xb714b8`, `0xb714f4`, `0xb71708`, `0xb716d0`, `0x5d3fa0`, `0x5d8b00`, `0x5d8bc0`, `0x5ea930`
Confidence: verified
Proof: one
Test: none

The partner's offer is held in arrays of seven dwords from index 0: item entries (`0xb714f4`, its seventh entry being `0xb7150c`), suffix
ids (`0xb714b8`) and counts (`0xb71708`). The own offer is seven GUIDs (`0xb716d0`, 8 bytes each) resolved to item objects with type mask 2. `0x4c0148` to `0x4c0151`
bound the index; `0x4c01a1` writes the name with the suffix through `0x5d8b00` (the own side uses `0x5d8bc0`), `0x4c0224` pushes the count
(the own side reads the item's field at `+0x20` of its `+0x114` block, `0x4c055d`), `0x4c0241` the quality or −1 and `0x4c0290` the use
test `CanUseItem`. `0x4c02b0` selects slot 7 (index 6) and `0x4c02d7` reads the enchant name at `+0x1e0 + 4·locale` of its spell row.
`0x4c0335` returns 6; the own side returns 5 (`0x4c0605`, `0x4c0613` for an empty slot).

## Trade links and money

`GetTradeTargetItemLink(index)` and `GetTradePlayerItemLink(index)` return the item link text of a slot, or no value for an
invalid index or an empty slot. `GetPlayerTradeMoney()` returns the own money offer, 0 with no trade window, and
`GetTargetTradeMoney()` the partner's. The partner's link is formatted from the arrays above and two more of seven dwords: the permanent
enchant (`0xb715c0`) and the unique id (`0xb715a0`).

Addresses: `0x4c0360`, `0x4c03c5`, `0x4c03f1`, `0x4c0431`, `0x4c0650`, `0x4c068c`, `0x4c06bc`, `0x4c0700`, `0x4c0716`, `0x4c0750`, `0x4c0759`, `0xb715a0`, `0xb715c0`, `0xb71728`, `0x5d4760`, `0x5d4770`, `0x52adb0`, `0x52ae00`
Confidence: verified
Proof: one
Test: none

`0x4c0431` calls `0x52adb0(entry, quality, enchant, suffix, unique, name)`, which formats `|Hitem:id:enchant:suffix:unique|h[name]|h` with the quality
colour escape (the arguments after the entry are pushed in that order at `0x4c0425` to `0x4c042e`). The own side's link is built by `0x52ae00` from the
item object (`0x4c06bc`). `0x4c0716` calls `0x5d4760` only when the partner GUID `0xb71728` is set; `0x4c0759` calls `0x5d4770` unconditionally.

## Clicking a trade slot

`ClickTradeButton(index)` raises `Usage: ClickTradeButton(index)` for a non-number. When the cursor carries money it adds
that money to the offer and clears the cursor on success. Otherwise it works on the item the cursor carries and the slot index−1:
for slots 1 to 6 an item that is bound refuses with `ERR_TRADE_QUEST_ITEM` when its record's field `+0x194` is 4 or 5 and
`ERR_TRADE_BOUND_ITEM` otherwise; an unbound container that holds items refuses with `ERR_TRADE_BAG`; an item in an equipped
bag slot (inventory index 0x13 to 0x16) refuses with `ERR_TRADE_EQUIPPED_BAG`.

Addresses: `0x4bfdf0`, `0x4bfe34`, `0x4bfe3e`, `0x4bfe52`, `0x4bfe80`, `0x4bfea1`, `0x4bfeb7`, `0x4bfee3`, `0x4bfef7`, `0x4bfefe`, `0x4bff0c`, `0x4bff62`, `0x4bffa4`, `0x4bffae`, `0x4bffc3`, `0xb4e2f0`, `0x5d4500`, `0x495190`, `0x494c80`, `0x5da2c0`, `0x6225d0`
Confidence: inferred
Proof: none
Test: none

`0x4bfe34` tests the cursor money `0xb4e2f0`. `0x4bfeb7` calls the bound test; `0x4bfee7` reads `+0x194` of the record and
`0x4bfef7` shows message 0x29, `0x4bff0c` message 0x28. `0x4bff62` shows 0x2b after counting the nonzero GUIDs in the
container's slot list, and `0x4bffae` shows 0xe. Past `0x4bffc3` the function reads the slot's offer state and places the item; that tail is open: its conditions and the packets it sends are not decoded.

## Loot rolls

`GetLootRollItemInfo(id)` returns five values for the open roll with that id: the icon path, the item name with its
random suffix, the number 1, the quality, and 1 when the item binds when picked up (record field `+0x194` equal to 1), else nil. An unknown roll or
uncached item gives nil, nil, 1, 1, nil. `GetLootRollItemLink(id)` returns the roll's item link or no value. `GetLootRollTimeLeft(id)` returns
the milliseconds until the roll's deadline (`+0x28`) minus the clock, or 0 for an unknown roll. `RollOnLoot(id, type)` calls the vote
with confirm 0 and `ConfirmLootRoll(id, type)` with confirm 1. Each raises `Usage: <Verb>(id)` or `(id, rollType)` for non-numbers.

Addresses: `0x4c3050`, `0x4c30aa`, `0x4c3101`, `0x4c3138`, `0x4c3160`, `0x4c316e`, `0x4c317e`, `0x4c3189`, `0x4c31a3`, `0x4c31f0`, `0x4c324a`, `0x4c32ab`, `0x4c32d0`, `0x4c3311`, `0x4c3321`, `0x4c334b`, `0x4c3370`, `0x4c33bc`, `0x4c33e0`, `0x4c3426`, `0x4c342c`, `0x61b2e0`, `0x61bdf0`, `0x4c1880`
Confidence: verified
Proof: one
Test: none

`0x4c30aa` finds the roll with `FindLootRollById`. `0x4c3138` to `0x4c3160` push the path, the suffix name and the constant 1.0 (`push 0x3ff00000`); `0x4c316e`
pushes the quality; `0x4c317e` compares `+0x194` with 1. `0x4c3321` subtracts the clock from `+0x28`. `RollOnLoot` pushes 0 before
`VoteOnLootRoll` (`0x4c33b6`), `ConfirmLootRoll` pushes 1 (`0x4c3426`). `LootButton:SetSlot` is in `ui/loot`.
