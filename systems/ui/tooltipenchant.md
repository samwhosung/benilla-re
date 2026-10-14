# ui: Item enchantment, durability and duration lines

The lines between the resistances and the requirements: the item's enchantments, its durability and its
own expiry. Where the numbers come from depends on the method that built the tooltip (`tooltipitem`,
"Which method fills what"); this doc gives the rules the builder applies to them. The builder is in
`tooltipitem`, the stat lines before these in `tooltipitemstats`, the requirements after in
`tooltipitemreq`.

An enchantment line is one white, green or red line of the enchantment's own name. Seven slots exist: the
permanent enchantment, the temporary one and five that a random property fills. `nameOnly` drops the
enchantment lines but keeps the durability and duration lines.

Roots: `0x52b650`

## The enchantment fork

Three arms exist and never overlap. A wrapped gift shows only the random-enchantment line. An item with
a live object, or one without an object whose instance fields are valid, runs the seven-slot loop and
then the proposed-enchantment lines. Any other item shows only the random-enchantment line.

Addresses: `0x52c991`, `0x52c996`, `0x52c99c`, `0x52c9a1`, `0x52c9a3`, `0x52c9ab`, `0x52c9b1`, `0x52cb6c`, `0x52cb72`, `0x52cc31`, `0x52cc33`, `0x52cc5b`
Confidence: verified
Proof: one
Test: none

`0x52c996 jne` sends a wrapped gift (`[ebp-0x4c]`) to `0x52cc33`; `0x52c9a1 jne` takes a live object into
the loop (`0x52c9b1`), else `0x52c9ab je` sends a block that is not valid to `0x52cc33`. The loop ends at
`0x52cb6c` and falls into the proposed enchantment (`0x52cb72`), whose end jumps past `0x52cc33`
(`0x52cc31`).

## Enchantment ids

For each of the seven slots the id is read from the object's instance fields when there is an object,
and from the tooltip's `+0x3d0` array otherwise; the object always wins. The instance field of slot `i`
is at `flds + 0x40 + 0xc*i`; it is followed by a duration, which the tooltip never reads, and the
slot's charges. A record flagged signable (flags bit `0x2000`) gives id 0 in every slot. An id is used
only when its absolute value is a row of the enchantment store; the id's sign does not change the row.

Addresses: `0x52c9b1`, `0x52c9b6`, `0x52c9cb`, `0x52c9d4`, `0x52c9de`, `0x52c9e0`, `0x52c9e5`, `0x52c9e8`, `0x52c9f2`, `0x52c9f9`, `0x52ca01`, `0x52ca06`, `0x52ca0c`, `0x52ca18`, `0x52ca1e`, `0x52ca23`, `0x52cb50`, `0xc0d7d8`, `0xc0d7dc`
Confidence: verified
Proof: one
Test: none

The byte cursor `edi` starts at 0x48 and steps by 0xc to 0x9c (`0x52c9b1`, `0x52cb56`, `0x52cb5d`). The
object path calls the item's secondary-table slot 0 (`0x52c9de`), which returns the record flags, and
tests bit `0x2000` (`0x52c9e0`); then it reads `flds + edi − 8` (`0x52c9e8`). The block path reads the
array at `+0x3d0` (`0x52c9f2`). The id is converted to its absolute value (`0x52ca01`, `0x52ca04`) and
bounded by the store's last index at `0xc0d7dc` (`0x52ca0c`), and the row read from `0xc0d7d8`
(`0x52ca1e`). A zero id, a missing row and an `INT_MIN` id skip the slot (`0x52c9f9`, `0x52ca06`, `0x52ca23`).

## Enchantment lines

A slot's line is the enchantment row's localized name, added exactly as stored, without a format. The
permanent and temporary slots (0 and 1) are green for a positive id and pure red for a negative one; the
five random-property slots are always white. With an object and a running timer for the slot, the text
is `ITEM_ENCHANT_TIME_LEFT` of the time left and the name, and the builder's return value is 1. With an
object whose record is not signable and a nonzero charge count for the slot, the line gets
`" (%s)"` of `ITEM_SPELL_CHARGES` appended, looked up with the charge count as its plural index and
formatted with it. Each line wraps nowhere.

Addresses: `0x52ca29`, `0x52ca2c`, `0x52ca31`, `0x52ca34`, `0x52ca38`, `0x52ca44`, `0x52ca49`, `0x52ca56`, `0x52ca5b`, `0x52ca65`, `0x52ca7d`, `0x52ca82`, `0x52ca8b`, `0x52ca90`, `0x52caa1`, `0x52caa8`, `0x52cac9`, `0x52cae7`, `0x52caec`, `0x52cafb`, `0x52cb18`, `0x52cb33`, `0x52cb48`, `0x5d9d00`, `0x52fa50`
Confidence: verified
Proof: one
Test: none

The colour is a local value, white by default (`0x52ca2c`); `cmp edi,0x60` (`0x52ca29`) lets only slots
0 and 1 replace it, with the green at `0xc0d3ac` for `id > 0` and the pure red at `0xc0d398`
otherwise (`0x52ca38` to `0x52ca46`). The text is `[row + locale*4 + 0x34]`, copied with a 0x1000 bound
(`0x52ca8b` to `0x52caa1`). `0x5d9d00(object; slot)` returns the milliseconds left of a deadline kept
per slot on the object (`0x52ca56`); the timed text is `0x52fa50(ms, ITEM_ENCHANT_TIME_LEFT, name, 1)`
(`0x52ca7d`), and the return flag is set at `0x52ca82`. The charges are read at `flds + edi`, which is
`flds + 0x48 + 0xc*i` (`0x52cac9`), the key is looked up with them as the count (`0x52caec`) and the
result is appended (`0x52cb18` to `0x52cb33`). The line is added by `0x5303b0` with the local colour
(`0x52cb48`).

## Proposed enchantment

When the instance fields are valid and the spell id at `+0x40c` is positive and a row of the spell
store, two lines follow the seven slots: `ITEM_PROPOSED_ENCHANT` with the spell's name, in green, and
`ITEM_ENCHANT_DISCLAIMER`, in pure red. Only the trade window sets the spell id, and only for trade slot 6; every other method leaves it 0, so neither line shows elsewhere.

Addresses: `0x52cb72`, `0x52cb75`, `0x52cb83`, `0x52cb8b`, `0x52cb91`, `0x52cb97`, `0x52cba3`, `0x52cbb9`, `0x52cbdb`, `0x52cc0e`, `0x52cc1f`, `0x52cc2c`, `0x5343a8`, `0x5345a8`, `0x534397`, `0x53459a`, `0x5d3fa0`, `0xc0d788`, `0xc0d78c`
Confidence: verified
Proof: one
Test: none

The gates are `+0x440` (`0x52cb7d`), `+0x40c` nonzero and not negative (`0x52cb8b`, `0x52cb91`) and the spell
store bounds and row (`0x52cb97` to `0x52cbae`). The spell's name is read at `[row + locale*4 + 0x1e0]`
(`0x52cbdb`). `SetTradePlayerItem` stores `0x5d3fa0` of 1 (`0x534397`, `0x5343a8`) and `SetTradeTargetItem`
of 0 (`0x53459a`, `0x5345a8`) when the slot is 6, else 0; `0x5d3fa0(side)` returns the third dword of that side's
trade-status header.

## Random property enchantments

A random property gives an item up to five enchantments. With a live object they are in its enchantment
fields. Without one, the builder copies five dwords from the suffix row (the same row that names the
item, `+0x8` to `+0x18`) into the tooltip's slots 2 to 6 (`+0x3d8` to `+0x3e8`), so a looted, rolled
or linked item shows them too, in white. `ITEM_RANDOM_ENCHANT` is the placeholder for the other case:
a record with a random property (`+0x1b8` nonzero) and no source of ids at all, or a wrapped gift. It is
green and never shows together with the enchantment lines.

Addresses: `0x52b7e0`, `0x52b7e6`, `0x52b7f0`, `0x52b7fb`, `0x52cc33`, `0x52cc39`, `0x52cc49`, `0x52cc56`, `0x533470`, `0x5335b7`, `0x5335bf`, `0x5335df`, `0x5364a0`, `0x536602`, `0x536608`, `0x53662d`
Confidence: verified
Proof: one
Test: none

`0x52b7e0` points at `tooltip + 0x3d8` and `0x52b7e6` at the row plus 8; the loop copies five dwords
(`0x52b7e9`, `0x52b7f0` to `0x52b7fb`). `0x52cc33` tests the record's `+0x1b8` and `0x52cc49` adds the
line. A looted item and a roll item have no object: `SetLootItem` stores the loot record's random
property and fourth link number (`0x5335b7`, `0x5335bf`) and passes the zero GUID `{0, 0}`
(`0x5335d9`, `0x5335dc`); `SetLootRollItem` stores them from its roll (`0x536602`, `0x536608`). Neither
leg has a proposed enchantment, so neither reaches the placeholder.

## Another player's enchantments

`SetInventoryItem` on another player's slot reads that player's visible-item record (48 bytes): the
creator GUID at `+0x0`, the entry at `+0x8`, seven enchantment ids from `+0xc`, and the random property
id in the low word of the dword at `+0x28`. The builder copies all seven ids, the creator and the
random property into the tooltip fields (valid, p6 = 1). The leg shows what the record holds; which of
the seven a server fills is the server's.

Addresses: `0x533314`, `0x533344`, `0x533354`, `0x53339f`, `0x5333a3`, `0x5333a8`, `0x5333ab`, `0x5333b8`, `0x5333cd`, `0x5333cf`, `0x5333d5`, `0x53340a`, `0x5f0d60`
Confidence: verified
Proof: one
Test: none

`0x5f0d60(unit; slot)` returns the record (`0x533314`) and the leg stops without a record or an entry.
The block is cleared (`0x533354` to `0x53339c`), `+0x424` takes the zero-extended word at `+0x28`
(`0x53339f`, `0x5333a3`), the loop copies seven dwords from `+0xc` (`0x5333a8` to `0x5333c8`) and the
creator GUID is copied last (`0x5333cd` to `0x5333d5`).

## Durability

`DURABILITY_TEMPLATE` shows the current and maximum values. It shows for an object that is not a wrapped
gift and whose maximum durability field (`flds+0xa4`) is nonzero, and for a valid block whose maximum
(`+0x434`) is nonzero. The maximum is the object's, else the block's, else the record's (`+0x1c4`); the
current value is the object's (`flds+0xa0`), else the block's (`+0x438`), else the record's maximum. The
current value is cut to the maximum. The line is red when the current value is 0, white otherwise. A
block maximum of −1 is first replaced by the record's.

Addresses: `0x52cc5b`, `0x52cc60`, `0x52cc69`, `0x52cc73`, `0x52cc85`, `0x52cc8b`, `0x52cc98`, `0x52cc9e`, `0x52cca6`, `0x52ccb0`, `0x52ccb8`, `0x52ccc3`, `0x52ccf3`, `0x52cd0e`, `0x52cd22`, `0x52cd2d`, `0x52cd57`, `0x52cd69`, `0x52cd7c`, `0x52cd97`, `0x52cd9f`, `0x52cda6`, `0x52cdb9`
Confidence: verified
Proof: one
Test: none

A wrapped gift (`0x52cc60 jne`) skips to the duration line. `0x52cc73` compares `+0x434` with −1 and
`0x52cc85` stores the record's `+0x1c4`. The show test is `flds+0xa4` unsigned above 0 (`0x52cc9e`,
`0x52cca6`) or the block maximum (`0x52ccb0`, `0x52ccb8`). Current value: `0x52ccf3` (`flds+0xa0`),
`0x52cd22` (`+0x438`) or `0x52cd2d` (the record). Maximum: `0x52cd57`, `0x52cd69` or `0x52cd76`. `cmp
esi,eax; jb` keeps the smaller (`0x52cd7c`); the colour test is `0x52cd9f` with `0xc0d390` for 0 (`0x52cda6`).

## The item's own duration

An item with a limited lifetime shows `ITEM_DURATION` with the time left, in white. It needs a live
object and a nonzero time left; the time is read from the object twice, once for the test and once for
the value, so the second reading can be 0 and the line then reads zero seconds. The line also sets the
builder's return value to 1. It shows for a `nameOnly` tooltip and for the compare tooltips.

Addresses: `0x52cdc0`, `0x52cdc3`, `0x52cdc8`, `0x52cdce`, `0x52cdd5`, `0x52cdd9`, `0x52cded`, `0x52cdf3`, `0x52cdfa`, `0x52ce08`, `0x52ce0d`, `0x5d9c60`, `0x52fa50`
Confidence: verified
Proof: one
Test: none

`0x52cdc8 je` needs the object. `0x5d9c60` (`0x52cdce`, `0x52cdd9`) returns the time left from a deadline
on the object, and 0 when unset or past. The format call is `0x52fa50(ms, ITEM_DURATION, 0, 1)`
(`0x52cded` to `0x52cdf3`) and the add is white with wrap 0 (`0x52cdfa` to `0x52ce08`). The block is
entered from the durability line (`0x52cdbe`), from its skips (`0x52ccaa`, `0x52ccb8`) and from a wrapped
gift (`0x52cc60`).
