# ui: Item description, creator and sell price lines

The last lines of an item tooltip: the quoted description, the maker, the open and read hints and the sell
price, then what the item methods return to Lua. A `nameOnly` tooltip ends before the description. The
builder is in `tooltipitem`; the lines before these are in `tooltipitemreq`.

Roots: `0x52b650`, `0x534620`, `0x532ee0`

## The description

A record with a description (`+0x198`, nonempty) adds it in quotation marks, in gold, wrapped. The
quotes are literal, not part of a global string.

Addresses: `0x52e147`, `0x52e14c`, `0x52e170`, `0x52e173`, `0x52e17d`, `0x52e183`, `0x52e19c`, `0x52e1aa`, `0x854ad4`
Confidence: verified
Proof: one
Test: none

`0x52e14c je 0x52e170` is the only way to the description: with `nameOnly` the fall-through at `0x52e14e`
shows the tooltip and returns. The text is formatted with `"%s"` between quotes (`0x854ad4`, `0x52e183`)
and added with `0xc0d3e8` and wrap 1 (`0x52e19c`, `0x52e1aa`).

## The maker line

The line names who made the item, in white, from a player name the name cache holds. A wrapped gift shows
`ITEM_WRAPPED_BY` with the gift giver; any other item shows `ITEM_CREATED_BY`, or `ITEM_WRITTEN_BY` when
the object has text (`ITEM_FIELD_ITEM_TEXT_ID` nonzero). The GUID is the object's creator field, else
the tooltip's `+0x410` or, for a gift, `+0x418`. A zero GUID or a name not yet cached (the cache calls
the builder again) adds no line. A tooltip without an object or valid fields skips to the sell price.

Addresses: `0x52e1b1`, `0x52e1b9`, `0x52e1bf`, `0x52e1c7`, `0x52e1d5`, `0x52e1eb`, `0x52e204`, `0x52e209`, `0x52e211`, `0x52e21b`, `0x52e230`, `0x52e247`, `0x52e255`, `0x52e25c`, `0x52e272`, `0x52e28a`, `0x52e29d`, `0x52e2a6`, `0x52e2ab`, `0x52e2cb`, `0x52e2db`, `0x52e32a`, `0x55f080`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

`[ebp-0x4c]` selects the arm (`0x52e1b1`). The creator arm takes the object's `flds+0x10` (`0x52e247`),
else the block's `+0x410` (`0x52e1d5`) when valid (`0x52e1c7`); the gift arm takes `flds+0x18`
(`0x52e32a`) or `+0x418` (`0x52e272`). The name is requested with `0x55f080(0xc0e228; GUID, tooltip,
0x52e530)` (`0x52e204`, `0x52e29d`) and a miss skips to the open test (`0x52e209`). The key is chosen at
`0x52e211` to `0x52e255` (`flds+0x9c`, `0x52e21b`); the text is added white, wrap 0 (`0x52e2cb`, `0x52e2db`).

## The open and read hints

An item that can be opened shows `ITEM_OPENABLE`, and one that can be read `ITEM_READABLE`, in green. They
need a live object, and the open hint only an object whose fields are not valid. A template with flags bit 2
is openable when it has no lock or the object is unlocked; a template with flags bit `0x200` is openable
when the object is wrapped. Otherwise the item is readable when it has page text or its object has a
text id. A bag item with a running cooldown has valid fields, so it shows no open hint.

Addresses: `0x52e2e0`, `0x52e2e2`, `0x52e2e8`, `0x52e2f8`, `0x52e2fc`, `0x52e30c`, `0x52e312`, `0x52e31d`, `0x52e323`, `0x52e33b`, `0x52e341`, `0x52e34e`, `0x52e358`, `0x52e35d`, `0x52e371`, `0x5d9e10`
Confidence: verified
Proof: one
Test: none

`0x52e2e0 je 0x52e376` leaves for the sell price without an object, and `0x52e2e8 jne 0x52e33b` skips the
open test when the fields are valid. The open test is the record flags `0x4` with the lock id (`0x52e2fc`)
and the object's unlocked bit (`0x52e30c`), or flags `0x200` (`0x52e312`) and the object's wrapped bit
(`0x52e31d`); `0x52e323` loads `ITEM_OPENABLE`. The read test is the object's slot `+0x74` (`0x5d9e10`, the
page text id from the record, `0x52e341`) or `flds+0x9c` (`0x52e34e`); `0x52e35d` adds the line with green,
wrap 0 (`0x52e371`). `ITEM_OPENABLE` has no other emitter.

## The sell price

While a merchant window is open, an item the player owns shows what the merchant pays, or `ITEM_UNSELLABLE`
when the record's sell price (`+0x28`) is 0. The builder does not draw coins: it runs the script handler
stored at the tooltip's `+0x454` with the amount in copper, and does nothing without a handler. The item
needs a real GUID. One held directly by the player must be in the backpack or a later slot, not equipped
nor in a bag slot; one in a bag qualifies. The base cursor mode must not be 0x11.

Addresses: `0x52e376`, `0x52e37e`, `0x52e384`, `0x52e390`, `0x52e396`, `0x52e39b`, `0x52e3a4`, `0x52e3b4`, `0x52e3ba`, `0x52e3c4`, `0x52e3f3`, `0x52e40f`, `0x52e418`, `0x52e47c`, `0x52e48c`, `0x52e496`, `0x52e4a7`, `0x523d10`, `0x7026f0`, `0xbddfa0`, `0x835154`
Confidence: verified
Proof: one
Test: none

The gates are a nonzero item GUID at p3 (`0x52e37e`), a nonzero vendor GUID at `0xbddfa0` (`0x52e384`,
`0x52e390`) and `0x523d10() != 0x11` (`0x52e396`, `0x52e39b`). An object whose container is the player (the
GUID at `flds+0x8` against the player's, `0x52e3ba`, `0x52e3c4`) is looked up in the player's slot list
at `+0x1d3c` of count `+0x1d38` (`0x52e3f3`); a slot below 0x17 (`0x52e40f`) or none leaves without a price.
An item in another container, and a call with no object, goes on. A sell price of 0 is `ITEM_UNSELLABLE`
(`0x52e418`, `0x52e496`), white, wrap 0. The call `0x7026f0(tooltip, +0x454, "%d", amount)` (`0x52e48c`)
runs only when the handler slot is nonzero (`0x52e47c`). Every path then stores 1 at `+0xd0` and calls slot
0x88.

## The sell price amount

For an item with an object the amount starts at the record's sell price. A template whose first spell has
charges below −1 scales it by the object's charges over the template's. The item's repair cost then comes
off: the amount becomes 1 if the cost is at least the amount, else the amount minus the cost. A stack
multiplies it by the count (`flds+0x20`). Without an object the amount is the sell price.

Addresses: `0x52e418`, `0x52e41b`, `0x52e426`, `0x52e42d`, `0x52e432`, `0x52e43d`, `0x52e446`, `0x52e44a`, `0x52e451`, `0x52e45e`, `0x52e466`, `0x52e46e`, `0x52e473`, `0x5da330`, `0x52d982`
Confidence: verified
Proof: one
Test: none

The charge scale reads the template's first charges through `[ebp-0x64]`, a pointer to `record+0x144`
saved at `0x52d982`, and applies only below −1 (`0x52e432`, `0x52e435`): the object's charges at `flds+0x28`
are multiplied by the sell price and divided by the template's charges (`0x52e43d` to `0x52e446`). `0x5da330` is the repair
cost (`0x52e44a`); `cmp eax,esi; jl` (`0x52e451`) picks 1 (`0x52e453`) or subtracts (`0x52e45e`); a zero cost
subtracts nothing. The stack count at `flds+0x20` multiplies when above 1 (`0x52e466`, `0x52e46e`).

## Results of the item methods

`SetBagItem(bag, slot)` returns two values: 1 or nil by the builder's return value (a lock line, a timed
enchantment, a duration or a cooldown), and the item's repair cost. `SetInventoryItem(unit, slot)` returns
three: 1 when the slot has an item, else nil; 1 or nil by the builder's return value; and the repair cost.
A slot with no item object, such as another player's, returns nil and 0 for the last two. The ammo slot
returns two values: 1 or nil, and nil. The repair cost is the item's repair cost scaled by one minus the
open merchant's discount, a number even when 0.

Addresses: `0x534620`, `0x534945`, `0x534948`, `0x534952`, `0x534958`, `0x534966`, `0x534975`, `0x534985`, `0x5349a1`, `0x532ee0`, `0x5332b5`, `0x5332c3`, `0x5332e1`, `0x533300`, `0x53330c`, `0x53340f`, `0x533427`, `0x53342e`, `0x53343e`, `0x4faf30`, `0x6f3810`, `0x6f37f0`
Confidence: verified
Proof: one
Test: none

`SetBagItem` stores the builder's result at `[ebp-0xc]` (`0x534945`), pushes the repair cost of
`0x4faf30(item)` (`0x534948`) after a `1.0` or nil chosen by the result (`0x534958`, `0x534966`) and returns
2 (`0x534985`); an empty slot returns 1 value, nil (`0x5349a1`). `SetInventoryItem` pushes `1.0`
(`0x5332b5`), the cooldown value (`0x5332c3`), the repair cost (`0x5332e1`) and returns 3 (`0x533300`); the
objectless leg pushes `1.0` (`0x53340f`) or nil (`0x533427`), then nil (`0x53342e`) and `0` (`0x53343e`).
