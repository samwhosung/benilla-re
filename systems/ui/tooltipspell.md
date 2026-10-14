# ui: The spell tooltip

The builder behind `SetSpell`, `SetAction` for a spell, `SetPetAction`, `SetShapeshift`, `SetTalent`, `SetCraftSpell`,
`SetTrainerService`, the quest reward spells and spell hyperlinks: its seven arguments, how it names and ranks
a spell, the talent variants, the cost, range, cast time and recast lines. The requirement lines, the chance
lines and the description are in `tooltipspellreq`; the `$` tokens of the description in `spelltokens`; the
buff and aura tooltip in `tooltipbuff`.

The spell record has 173 columns of four bytes. A column's offset is four times its index; the offsets this
builder reads are listed in `tooltipspellreq`. Texts are global strings, resolved as in `tooltipcontent`.

Roots: `0x52e610`, `0x52f820`, `0x535170`, `0x52b390`, `0x52b0a0`

## The builder's arguments

`0x52e610(this = tooltip; …)` takes seven stack arguments (`ret 0x1c`). Without the active player it hides the
tooltip and returns 0; a spell id with no record does the same on a fresh build. It returns 1 when it shows
the cooldown line, else 0.

| argument | holds |
|---|---|
| p1 | spell id |
| p2 | compact: no cost, cast time, reagents or chance lines, and a trimmed requirement set |
| p3 | the spell's remaining cooldown in milliseconds, or 0 |
| p4 | the caster is the pet, so the pet's casting data and form are used |
| p5 | show the rank beside the name |
| p6 | a talent record, or 0 |
| p7 | incremental: add to the tooltip as the next-rank block |

Addresses: `0x52e610`, `0x52e62c`, `0x52e63a`, `0x52e655`, `0x52e661`, `0x52e66d`, `0x52e687`, `0x52e69f`, `0x52e6b4`, `0x52e6c1`, `0x52e6c6`, `0x52e6cc`, `0x52f7a1`, `0x52f7af`, `0x52f7b5`, `0x52f7c0`, `0x52f589`, `0x52f59e`, `0x52f5a4`
Confidence: verified
Proof: one
Test: none

`0x52e62c` reads p7; zero calls `0x530050` (`0x52e63a`). The record is `[0xc0d788][p1]` bounded by `0xc0d78c`
(`0x52e687`, `0x52e695`). A fresh build stores the spell id at `+0x39c` and zeroes `+0x3b0` (`0x52e6c6`,
`0x52e6cc`). The return value is `esi` of the cooldown line (`0x52f7b5`). A compact build ends after the
cooldown line (`0x52f589`), showing the tooltip; a full build ends with the learn hint and the same show
(`0x52f7a1`, `0x52f7af`).

## Who passes what

| method | p2 | p3 | p4 | p5 | p6 | p7 |
|---|---|---|---|---|---|---|
| `SetSpell` | 0 | cooldown of the book entry | 1 for the `pet` book | 0 | 0 | 0 |
| `SetAction`, `SetShapeshift` | the interface option `UberTooltips` is 0 | cooldown | 0 | 1 | 0 | 0 |
| `SetPetAction` | 0 | cooldown | 1 | 1 | 0 | 0 |
| `SetTalent` | 0 | 0 | 0 | 0 | the talent | 0 or 1 |
| `SetHyperlink`, `SetCraftSpell`, `SetTrainerService`, quest reward spells | 0 | 0 | 0 | 0 | 0 | 0 |

The option `UberTooltips` defaults to 1, so the compact form is on only when the player has turned it off.

Addresses: `0x532d10`, `0x532e01`, `0x532e03`, `0x532e13`, `0x532e1c`, `0x532e2a`, `0x532e50`, `0x532e6a`, `0x532e92`, `0x5322a0`, `0x532618`, `0x532622`, `0x532639`, `0x532672`, `0x53268b`, `0x532920`, `0x532abe`, `0x532730`, `0x5328dc`, `0x535170`, `0x535338`, `0x535354`, `0x535374`, `0x532259`, `0x6e2ea0`, `0x63de30`, `0x842fc8`
Confidence: verified
Proof: one
Test: none

`SetSpell` compares the book argument with `pet` (`0x532e08` to `0x532e13`): a match reads the pet's
spell list `0xb6f098` and sets p4 (`0x532e1c`, `0x532e23`), any other book reads the player's `0xb700f0` (`0x532e2a`). It asks `0x6e2ea0`
for the cooldown (`0x532e50`) and passes the remaining time (`0x532e76` to `0x532e7e`). `SetAction` and
`SetShapeshift` look the option up by name with `0x63de30` (`0x532622`) and set p2 to 1 when it is missing or its
integer is 0 (`0x532627` to `0x532632`); they call the builder at `0x532672` or `0x53268b`. The call sites of
the other methods push a literal 0 for p2.

## Rebuilding after a cache answers

The totem and reagent names come from the item cache, which may not have them. Each miss is counted in
`+0x3b0`, and the builder is called again from the stored spell id `+0x39c` when the last answer arrives,
with every other argument 0.

Addresses: `0x52f820`, `0x52f823`, `0x52f82e`, `0x52f838`, `0x52f83f`, `0x52f849`, `0x52f862`, `0x52f867`, `0x52edaf`, `0x52f3ff`, `0x55ba30`
Confidence: verified
Proof: one
Test: none

`0x52f820(cookie, found)` returns without `found` (`0x52f826`). It decrements `+0x3b0` when nonzero
(`0x52f838`), returns while it is above 0 (`0x52f847`), and with a nonzero `+0x39c` (`0x52f849`) calls
`0x52e610(spell, 0, 0, 0, 0, 0, 0)` (`0x52f862`) and then `0x530640` (`0x52f867`). The builder increments
`+0x3b0` on each item-cache miss of a totem or reagent (`0x52edaf`, `0x52f3ff`).

## Recipe spells

A spell whose attributes (`+0x18`) have bit `0x20` shows the item it creates instead: the item builder is
called with the entry in the spell's first effect item type (`+0x19c`), no GUIDs, `nameOnly`, the equipped
header and the instance flag 0 and p7 passed on as the item builder's incremental argument.

Addresses: `0x52e6d2`, `0x52e6d6`, `0x52e6d8`, `0x52e6de`, `0x52e6fe`, `0x52e703`, `0x52b650`
Confidence: verified
Proof: one
Test: none

`0x52e6d2` tests `[record+0x18]` bit `0x20`; `0x52e6de` loads `+0x19c` into p1; the pushes before `0x52e6fe`
are p9, p8 zero, p7 from the spell builder's p7 (`0x52e6d8`), p6 to p4 zero, and two zero GUID pointers.

## The name and rank line

The first line has the name in white on the left and the rank in gray on the right. The rank is shown
when the spell is compact or p5 is set, and is the empty string otherwise. An incremental build has no such
line; it adds `TOOLTIP_TALENT_NEXT_RANK`, behind a newline, in white. The name is column 120 and the rank
column 129 (`+0x1e0` and `+0x204`, plus the locale).

Addresses: `0x52e7ba`, `0x52e7bd`, `0x52e7c3`, `0x52e7ce`, `0x52e7f7`, `0x52e7fe`, `0x52e801`, `0x52e809`, `0x52e812`, `0x52e81a`, `0x52e821`, `0x52e826`, `0x52e82f`, `0x530270`, `0x854a10`
Confidence: verified
Proof: one
Test: none

`0x52e7ba` tests p7: set adds `"\n%s"` (`0x854de4`) of `TOOLTIP_TALENT_NEXT_RANK` through `0x5303b0`
(`0x52e7e4` to `0x52e7f7`). Otherwise `0x52e7fe` and `0x52e809` pick the rank text from `[record + locale*4 +
0x204]` (`0x52e812`) when p2 or p5 is nonzero, else 0. `0x530270` adds both cells with gray `0xc0d3c4` right and
white `0xc0cf60` left (`0x52e821`, `0x52e826`, `0x52e82f`).

## The talent lines

With a talent record (p6) and a fresh build, `TOOLTIP_TALENT_RANK` gives the current and maximum rank, white.
The record has nine rank spells from `+0x10`: the maximum is the last nonzero one, the current rank the
highest the player knows. A talent not yet learned then lists, in red, each prerequisite talent below its
rank, the tab points still needed for its tier and a spell it needs. A learnable talent, for the active
player with free talent points, ends with `TOOLTIP_TALENT_LEARN` in green.

Addresses: `0x52e834`, `0x52e83b`, `0x52e842`, `0x52e853`, `0x52e86a`, `0x52e887`, `0x52e891`, `0x52e764`, `0x52e782`, `0x52e794`, `0x52e7af`, `0x52b390`, `0x52b3f0`, `0x52b401`, `0x52b43f`, `0x52b450`, `0x52b49e`, `0x52b4a5`, `0x52b507`, `0x52b524`, `0x52b53a`, `0x52b561`, `0x52b5be`, `0x52b5ce`, `0x52b606`, `0x52b5af`, `0x52f731`, `0x52f735`, `0x52f758`, `0x52f761`, `0x52f76b`, `0x52f77a`, `0x52f783`, `0x52f78f`, `0x854a2c`, `0x854a5c`, `0x854a40`, `0x8549f8`, `0xc0d6e8`, `0xc0d6ec`, `0xc0d6d4`, `0xc0d6d8`, `0xbdcd24`, `0xbdcd28`
Confidence: verified
Proof: one
Test: none

`0x52e764` scans the nine rank spells from the last (`+0x30` downward): the first nonzero sets the maximum
`[ebp-0x3c]` (`0x52e776` to `0x52e77b`), and the first known one (`0x60c740`, then `0x60c8d0` for a higher rank,
`0x52e782`, `0x52e794`) sets the current rank `[ebp-0x40]` (`0x52e7af`). The rank line is
`"%d/%d"` of both plus one through `TOOLTIP_TALENT_RANK` (`0x52e842` to `0x52e882`). A current rank below 0 calls
`0x52b390(tooltip, talent)` (`0x52e891`), which checks the three prerequisite talents (ids at `+0x34`, ranks
at `+0x40`, store `0xc0d6e8`, `0x52b3f0` to `0x52b4d5`) and prints `TOOLTIP_TALENT_PREREQ` with the rank and the
talent's spell name (`0x52b49e`, `0x52b4a5`), then the tier points (`+0x4` tab and `+0x8` points against the tab
list `0xbdcd24`, `0x52b507` to `0x52b58e`, key `TOOLTIP_TALENT_TIER_POINTS`) and the required spell at `+0x50`
(`0x52b5af` to `0x52b636`, `ITEM_REQ_SKILL`), all red. Its result, 1 when nothing is missing, is `[ebp-0x44]`.
The learn hint needs a nonzero `[ebp-0x44]` (`0x52f735`), the active player (`0x52f758` to `0x52f761`), free
talent points at `[player fields + 0xe48]` (`0x52f761`), and a current rank other than the maximum (`0x52f76b`).

## Talent tooltips

`SetTalent(tab, index)` finds the talent at `tabs[tab − 1].talents[index − 1]` (0x54 bytes each). A talent
whose flags (`+0x4c`) have bit 0 uses the spell builder: for a talent the player knows, the current rank's
spell with p6 set, then, if it is not the last rank, the next rank's spell as an incremental block; for one
not known, the first rank's spell. A talent without that flag is built by `0x52b0a0`: the name in white, the
rank line, the prerequisites, then the current rank's description in gold and the next rank's below it behind
the next-rank header.

Addresses: `0x535170`, `0x535253`, `0x535281`, `0x535298`, `0x5352a4`, `0x5352ab`, `0x5352af`, `0x53530c`, `0x535315`, `0x53531e`, `0x535323`, `0x535338`, `0x53533d`, `0x535342`, `0x535354`, `0x535362`, `0x535374`, `0x535382`, `0x535386`, `0x52b0a0`, `0x52b0d8`, `0x52b128`, `0x52b1ed`, `0x52b212`, `0x52b24d`, `0x52b285`, `0x52b2a8`, `0x52b2cc`, `0x52b2f5`, `0x52b307`, `0x82ee00`
Confidence: verified
Proof: one
Test: none

The tab list is `0xbdcd28` of count `0xbdcd24`; a tab's talent array is at `+0xc` with the count at `+0x8`
(`0x535281` to `0x5352a7`) and a row is 0x54 bytes (`0x5352a4`). `0x5352ab` tests `+0x4c` bit 0; clear goes to
`0x52b0a0` (`0x535386`). `0x535323` takes the spell `[row + rank*4 + 0x10]` (`0x535323`) and calls with p6 the
row (`0x535338`); `0x535342` takes the next spell `[row + rank*4 + 0x14]` for the second call with p7 = 1
(`0x535354`). An unknown talent, or one with no spells, calls with `[row+0x10]` (`0x535374`). The variant
`0x52b0a0` loops twice (`0x52b26a` to `0x52b315`): the name in white (`0x52b1fe`), the rank line (`0x52b241`),
`0x52b390` for the prerequisites (`0x52b24d`), the description through `0x5075f0` in gold with wrap 1
(`0x52b2f5`, `0x52b307`), and the next-rank header `TOOLTIP_TALENT_NEXT_RANK` for the second pass (`0x52b2cc`).

## The cost and range line

A full build adds one two-cell line: the power cost on the left and the range on the right. The cost is the
spell's power cost over the power type's display divisor, with a cost per second as well when it has one;
types 0 to 4 use `MANA_COST`, `RAGE_COST`, `FOCUS_COST`, `ENERGY_COST` and `HAPPINESS_COST`, any other
`HEALTH_COST`. The range is `SPELL_RANGE` with `"%d-%d"` when the minimum is above 0, else the maximum alone,
and moves to the left cell when there is no cost. It is omitted for next-swing spells (`+0x18` bits `0x404`),
for attribute bit `0x40000000` of `+0x24`, and for a maximum of 0 or less.

Addresses: `0x52e8a7`, `0x52e8ad`, `0x52e8b9`, `0x52e8c5`, `0x52e8d2`, `0x52e8dd`, `0x52e8e5`, `0x52e8f9`, `0x52e905`, `0x52e90e`, `0x52e913`, `0x52e925`, `0x52e926`, `0x52e946`, `0x52e968`, `0x52e9a5`, `0x52e9b2`, `0x52e9dd`, `0x52e9e2`, `0x52e9f0`, `0x52e9f9`, `0x52ea04`, `0x52ea06`, `0x52ea0c`, `0x52ea1a`, `0x52ea2f`, `0x52ea39`, `0x52ea51`, `0x52ea78`, `0x52ea8c`, `0x52ea94`, `0x52eab5`, `0x52ead2`, `0x6e31b0`, `0x6e3300`, `0x6e3480`, `0x6e7130`, `0x85416c`, `0x854180`
Confidence: verified
Proof: one
Test: none

`0x6e31b0(spell, pet)` is the cost (`0x52e8ad`) and `0x6e3300` the cost per second (`0x52e8c5`), each divided by
`0x6e7130` of the power type (`+0x7c`, `0x52e8d2`, `0x52e8df`, `0x52e8f4`). The key array is `0x85416c` for types 0 to 4
(`0x52e905`) and `HEALTH_COST` at `0x854fc8` otherwise (`0x52e90e`). A pair of tests picks both amounts, one,
or none (`0x52e913` to `0x52e921`); the per-time arm formats `"%s_PER_TIME"` of the name (`0x52e925`, `0x52e926`).
`0x6e3480(caster, spell, &min, &max, 0)` (`0x52e9dd`) and its rules are in `spell/records#range`; the two
values are converted with `fistp` (round to nearest even, `0x52ea09`, `0x52ea0f`, `0x52ea32`), and the string
is `SPELL_RANGE` of `"%d-%d"` (`0x52ea1a`) or `"%d"` (`0x52ea39`). With an empty cost cell the range is
copied to the left (`0x52ea94` to `0x52eaae`). The line is added by `0x530270` with white on both sides
(`0x52ead2`).

## The cast time and recast line

A full build adds one line: the cast time on the left, the recast time on the right. A spell whose first
effect is trade skill (47) or attack (78), or that is passive (`+0x18` bit `0x40`), has none; effect 47 and
passive also mark the spell passive for the chance lines. A positive cast time is `SPELL_CAST_TIME_MIN` at
60000 ms or more, else `_SEC`, with the count a double over 60000 or 1000. A time of 0 is, by the first
match, next-swing, ranged (bit 2), channeled (`+0x1c` bits `0x44`), then instant, or instant without mana
when the spell costs nothing. The recast cell is the larger recovery (`+0x4c`, `+0x50`), modified:
`SPELL_RECAST_TIME_MIN` at 60000 or more, else `_SEC`.

Addresses: `0x52eada`, `0x52eaea`, `0x52eaf4`, `0x52eb00`, `0x52eb15`, `0x52eb1a`, `0x52eb23`, `0x52eb2c`, `0x52eb32`, `0x52eb3c`, `0x52eb4d`, `0x52eb54`, `0x52eb59`, `0x52eb67`, `0x52eb99`, `0x52eba1`, `0x52ebb5`, `0x52ebce`, `0x52ebd5`, `0x52ebf4`, `0x52ec03`, `0x52ec20`, `0x52ec27`, `0x52ec32`, `0x52ec4b`, `0x52ec5b`, `0x52ec74`, `0x52ec95`, `0x52eca0`, `0x52eca9`, `0x52ecb4`, `0x52ecfb`, `0x52ed11`, `0x52ed35`, `0x52ed3c`, `0x6e3340`, `0x6e6af0`, `0x6e2b60`, `0x808ff0`, `0x8022cc`
Confidence: verified
Proof: one
Test: none

`0x52eada` to `0x52eb12` compute the recovery: `+0x4c` and `+0x50` modified by `0x6e6af0` (op 11) and
`0x6e2b60`, the larger kept in `[ebp-0x38]` (`0x52eb0d`). `0x52eaea` zeroes the passive flag; the four exits are at
`0x52eb1a` (compact), `0x52eb2c` (effect 47, sets the flag at `0x52ed3c`), `0x52eb36` (passive, sets it) and
`0x52eb3f` (effect 78). `0x6e3340(spell, pet, 1)` is the cast time (`0x52eb4d`); `cmp eax,0xea60` / `setge`
(`0x52eb54`, `0x52eb59`) picks `_MIN`. The count is the milliseconds over the float 60000 (`0x808ff0`) or 1000
(`0x8022cc`), pushed as a double (`0x52ebb5`) into the raw template. The next-swing, ranged and channeled
tests are at `0x52ebf4`, `0x52ec1c` and `0x52ec27`; the instant arm at `0x52ec4b` needs the power type 0 and a
cost above 0. The recast cell is built the same way from `[ebp-0x38]` (`0x52eca0`) and is empty for 0 or less
(`0x52ed11`). The line is added by `0x530270` with white on both sides (`0x52ed35`).
