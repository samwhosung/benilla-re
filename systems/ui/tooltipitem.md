# ui: The item tooltip builder

The one function behind every `Set…Item` method of a `GameTooltip`: its nine arguments, the fields it
keeps so it can build itself again when a cache answers, the flag that trims it to a name, and which
method fills which fields. What the lines say is in `tooltipitemhead`, `tooltipitemstats`,
`tooltipenchant`, `tooltipitemreq` and `tooltipitemfoot`; the line machinery is in `tooltipcontent`.

The builder reads the item record from the item cache by entry. Each method resolves its own item
(equipped, bagged, looted, offered, listed, linked), fills the tooltip's instance fields where it has
live data, and calls the builder once or twice.

Roots: `0x52b650`, `0x52e530`, `0x5326e0`, `0x532ee0`, `0x534620`

## The builder's arguments

`0x52b650` is a `thiscall` on the tooltip with nine stack arguments (`ret 0x24`). A fresh build clears
the tooltip first; without an active player or, on a fresh build, without a cached record it hides the
tooltip and returns 0. A complete build ends by showing the tooltip. The return value is 1 when a line
that depends on a running time was emitted, else 0, and the methods hand it to Lua as the first result.

| argument | holds |
|---|---|
| p1 | item entry |
| p2 | pointer to a GUID forwarded to the record query |
| p3 | pointer to the item's GUID, all zero when there is no object |
| p4 | `nameOnly` |
| p5 | prepend the gray `CURRENTLY_EQUIPPED` line |
| p6 | the instance fields of the tooltip are valid |
| p7 | incremental: do not clear, do not keep the arguments, separate the name with a newline |
| p8, p9 | low and high dword of the GUID of the unit whose gear counts for a set |

Addresses: `0x52b650`, `0x52b671`, `0x52b67e`, `0x52b683`, `0x52b69e`, `0x52b6a5`, `0x52b6af`, `0x52b722`, `0x52b72b`, `0x52b730`, `0x52b763`, `0x52b777`, `0x52b874`, `0x52bfa6`, `0x52ca82`, `0x52ce0d`, `0x52e140`, `0x52e4b5`, `0x52e4ba`, `0x52e4c4`, `0x468460`, `0x55ba30`, `0x530050`
Confidence: verified
Proof: one
Test: none

`0x52b671` reads p7; zero calls `0x530050` (`0x52b67e`). `0x468460(0x10, …)` of the active player
(`0x52b699`) failing stores 0 at `+0xd0` and calls slot 0x84 (`0x52b6a5`, `0x52b6af`); a missing record
does the same only when p7 is zero (`0x52b730`). The return value lives in `[ebp-0x38]`, zeroed at
`0x52b763` and set to 1 at `0x52bfa6` (a lock line), `0x52ca82` (a timed enchant), `0x52ce0d`
(the item's duration) and `0x52e140` (a cooldown). The tail stores 1 at `+0xd0` and calls slot 0x88
(`0x52e4ba`, `0x52e4c4`). `0x52b874` adds the p7 newline.

## Fields kept for a rebuild

A fresh build copies p1, p3, p4, p5, p6, p8 and p9 into the tooltip, and `0x52e530` rebuilds from them
when a cache answers. The instance fields at `+0x3d0` hold what the method knows about the item that the
record does not: enchantments, a timer, the creator, a random property, charges, a lock and durability.
Methods that have no such data leave `+0x440` zero, and the builder ignores the block.

| field | holds |
|---|---|
| `+0x380`, `+0x398` | item GUID, entry |
| `+0x3a0`, `+0x3a4` | p4, p5 |
| `+0x3a8`, `+0x3ac` | p8, p9 |
| `+0x3d0` to `+0x3e8` | seven enchantment ids |
| `+0x408`, `+0x40c` | remaining cooldown in milliseconds, spell id of the proposed enchantment |
| `+0x410`, `+0x418` | creator GUID, gift creator GUID |
| `+0x420`, `+0x424` | fourth item-link number (read by no function), random property id |
| `+0x428`, `+0x42c` | charges (−1 when unset), wrapped flag |
| `+0x430` | lock id |
| `+0x434`, `+0x438` | maximum and current durability |
| `+0x440` | p6 |

Addresses: `0x52b6cc`, `0x52b6ce`, `0x52b6da`, `0x52b6e0`, `0x52b6e3`, `0x52b6ec`, `0x52b6f5`, `0x52b6fe`, `0x52b704`, `0x52b70a`, `0x5326e0`, `0x5326e2`, `0x532710`, `0x52e530`, `0x52e556`, `0x52e5a7`, `0x52e5ac`, `0x530640`, `0x529240`, `0x5292f4`, `0x808f60`
Confidence: verified
Proof: one
Test: none

`0x52b6c5` to `0x52b6ca` skip the copy block `0x52b6cc` to `0x52b70a` when p7 is set. `0x5326e0` takes
`tooltip+0x3d0` in `ecx`, zeroes two seven-dword arrays (`+0x000` and `+0x01c`) and the dwords to `+0x68`,
and sets `+0x58` to −1 (`0x532710`). `0x52e530(cookie, found)` does nothing without `found` or an entry
(`0x52e556`), else calls the builder with p1 from `+0x398`, p2 a zero GUID, p3 a copy of `+0x380`, p4 from
`+0x3a0`, p5 from `+0x3a4`, p6 from `+0x440`, p7 zero and p8, p9 from `+0x3a8` (`0x52e5a7`), then sizes
the tooltip (`0x52e5ac`). The record cache, the name cache, the petition cache and the item-name cache
all take `0x52e530` as their callback.
The tooltip object is 1120 bytes; its constructor `0x529240` stores the 36-slot virtual table `0x808f60` first (`0x5292f4`).

## Wrapped gifts

The builder decides once whether the item is a wrapped gift: its object's `ITEM_FIELD_FLAGS` has the
wrapped bit (bit 3), or the tooltip's fields are valid and `+0x42c` is set. A wrapped gift shows the name
without a random-property suffix, none of its enchantments, no durability line and `ITEM_WRAPPED_BY`
in place of the maker line; the other lines are as for any item.

Addresses: `0x52b777`, `0x52b783`, `0x52b789`, `0x52b78d`, `0x52b78f`, `0x52b799`, `0x52b7a1`, `0x52b7aa`, `0x52b7b0`, `0x52b7b3`, `0x52c996`, `0x52cc60`, `0x52e1b9`, `0x52e25e`
Confidence: verified
Proof: one
Test: none

`0x52b777` finds the object by the GUID at p3 (type mask 2) and stores it at `[ebp-0x20]`. The flag is
`test [flds+0x3c],8` (`0x52b789`) or `+0x440` and `+0x42c` (`0x52b78f` to `0x52b7a1`), stored at `[ebp-0x4c]`
(`0x52b7b0`). `0x52b7b3` skips the suffix lookup, `0x52c996` the enchantment loop, `0x52cc60` the
durability line and `0x52e1b9` and `0x52e25e` pick the gift arm of the maker line.

## The nameOnly flag

`GameTooltip:SetInventoryItem(unit, slot, nameOnly)` passes a third argument that is a number above 0
as p4; every other method passes 0. A set p4 shows the name in white, keeps the slot and type line and
the container line, and drops the bind, lock and area lines, the whole stat body and the enchantments,
and ends before the description. It is kept in `+0x3a0`, so a rebuild stays trimmed.

Addresses: `0x532ee0`, `0x532f94`, `0x532fa9`, `0x532faf`, `0x533020`, `0x533027`, `0x53302a`, `0x53303a`, `0x53303f`, `0x533047`, `0x53304a`, `0x53304c`, `0x533106`, `0x533267`, `0x5332ad`, `0x53340a`, `0x52b8b3`, `0x52bac3`, `0x52c225`, `0x52e14c`, `0x6f4940`, `0x802970`
Confidence: verified
Proof: one
Test: none

Of the 31 calls of the builder, 27 push a literal or register zero for p4, `0x52e5a7` forwards `+0x3a0`, and
`0x533267`, `0x5332ad` and `0x53340a` push the local at `[ebp-0x10]`. That local is seeded 0 at
`0x533027`, and set to 1 at `0x53304c` only when argument 4 is a number (`0x53302a`) with `fcomp`
against 0.0 (`0x802970`) leaving the compare bits clear, that is, strictly above 0; zero, negatives,
NaN, strings and nil keep 0. The usage string at `0x8552dc` names it `nameOnly`. The leg at `0x533106`
(slot −1 of the player) pushes 0. Four reads of p4 consume it: the name colour (`0x52b8b3`), the jump
over the bind region (`0x52bac3`), the jump over the stat body (`0x52c225`) and the early return
(`0x52e14c`). The stock interface passes two arguments.

## Which method fills what

Every `Set…Item` method calls the builder; they differ in the item they resolve and in what they write
into the instance fields. A method with a live item object copies the fields from the object; the trade
partner's items come from the replicated trade slots; another player's items from the visible-item
records. A call with p6 = 0 leaves the fields untouched.

| method and leg | p6 | fields from |
|---|---|---|
| `SetBagItem`, item with a running cooldown | 1 | the object |
| `SetBagItem`, any other item | 0 | none |
| `SetInventoryItem`, item with a running cooldown | 1 | the object |
| `SetInventoryItem`, other item of the player, charged item with 0 charges, ammo slot | 0 | none |
| `SetInventoryItem`, slot with no item object, such as another player's | 1 | the visible-item record |
| `SetTradePlayerItem` | 1 | the object |
| `SetTradeTargetItem` | 1 | the replicated trade slot |
| `SetLootItem`, `SetLootRollItem`, `SetInboxItem`, `SetAuctionItem`, `SetHyperlink` | 1 | the loot, mail, auction or link record |
| `SetAction` item, found in the bags | 1 | the object |
| `SetAction` item, not found | 0 | none |
| every other method | 0 | none |

Addresses: `0x534620`, `0x53483a`, `0x53483f`, `0x53484a`, `0x534855`, `0x534863`, `0x534900`, `0x53493e`, `0x6e2ed0`, `0x532ee0`, `0x5331ec`, `0x533209`, `0x533267`, `0x5332ad`, `0x533106`, `0x533354`, `0x53340a`, `0x5341e0`, `0x534302`, `0x5343da`, `0x534410`, `0x534514`, `0x5345e9`, `0x533470`, `0x533564`, `0x5335df`, `0x5354c0`, `0x5355fa`, `0x5356bb`, `0x535810`, `0x5359d9`, `0x535a79`, `0x5364a0`, `0x5365b6`, `0x53662d`, `0x531fd0`, `0x532181`, `0x5321fa`, `0x5322a0`, `0x5324a0`, `0x532592`, `0x5325ff`
Confidence: verified
Proof: one
Test: none

`SetBagItem` calls `0x6e2ed0` (`0x53483a`) and takes the cooldown leg only when all three outputs are
nonzero (`0x53483f`, `0x53484a`, `0x534855`). That leg stores the cooldown at `+0x408` (`start + duration −
now`), the creator at `+0x410` and the wrapped bit at `+0x42c`, and calls the builder with p6 = 1
(`0x534900`); the plain leg at `0x53493e` has p6 = 0. `SetInventoryItem` queries the cooldown the same
way (`0x5331ec`): a running one reaches the call at `0x533267` with p6 = 1; the call at `0x5332ad` has
p6 = 0 and is reached for an item without one and for an item whose template has charges and whose
instance has none. The ammo slot (−1, `0x533106`) passes the player's ammo entry with p6 = 0. A slot with
no item object, such as another player's, copies the visible-item record into the fields (`0x533354`) and calls with p6 = 1
(`0x53340a`). The trade methods and the loot, mail, auction and link methods zero the fields with
`0x5326e0` or the same stores and write their own values.
