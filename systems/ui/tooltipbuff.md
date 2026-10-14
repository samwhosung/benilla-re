# ui: Buff tooltips and the spell selectors

The tooltip of an aura, and the bindings that choose which spell or item a tooltip describes:
`SetPlayerBuff`, `SetUnitBuff`, `SetUnitDebuff`, `SetTrainerService` and `SetCraftSpell`. The aura builder is
a short function of its own; the selectors add no line themselves. The spell tooltip they hand over to
is in `tooltipspell`, the item tooltip in `tooltipitem`, the description tokens in `spelltokens`.

Roots: `0x52f880`, `0x532b00`, `0x534ac0`, `0x534e30`, `0x5338b0`, `0x533e90`

## The aura tooltip

`0x52f880(this = tooltip; spell id, level, stack, unused)` clears the tooltip and looks the spell up; a
missing record hides the tooltip. The first line is the spell's name on the left and its dispel type's name on
the right, both gold; the right cell is empty for a type of 0, for one not in the table and for one whose row
has no flag in `+0x28`. The second line is the aura description (`+0x24c`, else the description) with its
tokens expanded, white and wrapped. The stack given to the expander is the aura's stack count when the spell
stacks (`+0x9c` above 1), else 1, so the description shows the total. Nothing follows the description.

Addresses: `0x52f880`, `0x52f88d`, `0x52f8ae`, `0x52f8ca`, `0x52f8d0`, `0x52f8d9`, `0x52f8e5`, `0x52f8f1`, `0x52f8ff`, `0x52f906`, `0x52f90d`, `0x52f91e`, `0x52f929`, `0x52f930`, `0x52f935`, `0x52f955`, `0x52f96a`, `0x530380`, `0x5075f0`, `0xc0d83c`, `0xc0d840`
Confidence: verified
Proof: one
Test: none

`0x530380` pushes the gold pointer for both cells, so the name and the dispel type are both gold
(`0x52f91e`; `tooltipcontent#the-line-arrays`). The dispel type is `+0x10`; its table is indexed by `0xc0d83c`
and bounded by `0xc0d840` (`0x52f8f1`, `0x52f8ff`), and the name is at `[row + locale*4 + 4]` (`0x52f90d`)
when `[row+0x28]` is nonzero (`0x52f906`). The stack value is chosen at `0x52f8ca` to `0x52f8dc`. The
description call passes the level, the aura flag 1, the stack and the unmodified flag 1 (`0x52f940` to
`0x52f955`); the line is white with wrap 1 (`0x52f95c`, `0x52f96a`) and is skipped when the text is empty
(`0x52f930`, `0x52f935`).

## The time left on buffs

`GameTooltip:SetPlayerBuff(index)` finds the player's buff record by index and builds the aura tooltip
from its spell id, level and stack count. If the buff has an end, it adds a last line in gold with the
time left, `SPELL_TIME_REMAINING` with a unit suffix, counted up to the next whole unit for days, hours and
minutes and down for seconds. A buff that lasts until cancelled has no such line. The tooltips of other
units' auras never have it.

Addresses: `0x532b00`, `0x532b97`, `0x532bab`, `0x532bb4`, `0x532bbf`, `0x532bd5`, `0x532bda`, `0x532bdf`, `0x532be3`, `0x532bec`, `0x532bf7`, `0x532bfa`, `0x532c09`, `0x532c12`, `0x532c1c`, `0x4e4430`, `0x4e4450`, `0x52fa50`, `0x855278`
Confidence: verified
Proof: one
Test: none

`0x4e4430(index)` returns the record (`0x532bb4`); its spell id is at `+0x4`, level at `+0x8`, stack at `+0x9`
(`0x532bbf` to `0x532bd2`). `0x532bda` tests `[record+0xc]`, the flag for a buff without an end, and jumps over
the line when set (`0x532bdf`). `0x4e4450(index)` is the milliseconds left (`0x532be3`) and the text is
`0x52fa50(ms, SPELL_TIME_REMAINING, 0, 1)` into a 32-byte buffer (`0x532bec` to `0x532bfa`), added with
`0x530380`, wrap 0 (`0x532c09`). The tail shows the tooltip (`0x532c12`, `0x532c1c`).

## Auras of other units

`SetUnitBuff(unit, index [, filter])` and `SetUnitDebuff(unit, index [, filter])` show a unit's index-th aura.
Buffs are counted from the first 32 aura slots and debuffs from the 16 after them. A slot counts when it has a
spell, its flag nibble has a bit of `0xe`, the spell is not hidden (attribute bit 7, `+0x1c` bit `0x10000000`, or
an effect of 44, 45 or 151) and, with the third argument set, the filter function accepts it. A party member
out of sight is read from the party record's 32 aura words instead. A unit in sight gives the builder the
aura's level and stack count; a party record gives level 0 and stack 1.

Addresses: `0x534ac0`, `0x534b7d`, `0x534b8b`, `0x534bd1`, `0x534bda`, `0x534bee`, `0x534bfb`, `0x534c10`, `0x534c16`, `0x534c33`, `0x534c3f`, `0x534c5e`, `0x534c6e`, `0x534c70`, `0x534ca5`, `0x534cd1`, `0x534cf5`, `0x534cfd`, `0x534d40`, `0x534d5f`, `0x534da6`, `0x534db0`, `0x534df0`, `0x534e30`, `0x535000`, `0x53500f`, `0x53507e`, `0x5350e3`, `0x5350f6`, `0x535135`, `0x4b3870`, `0x4b3920`
Confidence: verified
Proof: one
Test: none

The loop of `SetUnitBuff` runs the slot byte offset from `0xa4` to `0x124` (`0x534bfb`, `0x534c8e`); that of
`SetUnitDebuff` runs to `0x164` (`0x53500f`). The aura spell is `fields + offset` (`0x534c16`); the nibble of
flags is at `fields + 0x164 + slot/2` (`0x534c33` to `0x534c42`). `0x534df0` is the hidden-aura test: attribute
bit 7 set (`0x534df5`), `+0x1c` bit `0x10000000` (`0x534df7`) or an effect of 44, 45 or 151 (`0x534e0a` to
`0x534e14`) refuses. `0x4b3870` is the buff filter (`0x534c70`) and `0x4b3920` the debuff one (`0x5350f6`). The
level is the byte at `fields + 0x17c + slot` (`0x534ceb`), the stack the byte at `fields + 0x1ac + slot`
plus one (`0x534cd1`, `0x534cdd`). The party path calls `0x496400` and `0x496420` (`0x534cfd`) and reads
words from the record (`0x534d40`).

## Trainer services

`GameTooltip:SetTrainerService(index)` shows what a service teaches. The service names a spell. If one of
its first three effects is learn spell (36) or learn pet spell (57) and the trigger spell has a record, that
taught spell is shown: as the item it creates (by the item builder, with a pseudo GUID holding the spell id)
when it is a recipe (attribute `0x20`, first effect 24), else by the spell builder, with the pet flag set for
learn pet spell. A spell with no learning effect is described as itself. An index with no service raises
"Invalid trainer service in SetTrainerService(index)".

Addresses: `0x5338b0`, `0x533986`, `0x533989`, `0x533996`, `0x5339ac`, `0x5339c5`, `0x5339c7`, `0x5339cc`, `0x5339d1`, `0x5339ee`, `0x5339f9`, `0x533a0b`, `0x533a1b`, `0x533a21`, `0x533a2c`, `0x533a3b`, `0x533a5a`, `0x533a6a`, `0x533a75`, `0x533a7f`, `0x533a8a`, `0x533a9a`, `0x4d89b0`, `0x52e610`, `0x52b650`, `0x8553a4`
Confidence: verified
Proof: one
Test: none

`0x4d89b0(index − 1)` returns the service row (`0x533989`), whose first word is the spell (`0x533996`). The loop
over the three effect slots (`0x5339c5`) accepts effect 36 (`0x5339c7`) or 57 (`0x5339cc`) and takes the trigger
spell at `+0xc0` from the effect (`0x5339d1`). The recipe test is attribute `0x20` (`0x533a1b`) and effect 24
(`0x533a21`); the item is the effect's item type at `+0x19c` (`0x533a2c`). The pseudo GUID has the spell id
and the high word `0xb0000000` (`0x533a3b`). The spell call has p4 from `sete` of effect 57 (`0x533a75`,
`0x533a7f`) and the fallback passes the service spell (`0x5339f9`, `0x533a0b`).

## Craft spells

`GameTooltip:SetCraftSpell(index)` shows what a craft-list entry teaches. The entry names a spell; the first
of its three effects that is learn spell (36) shows the trigger spell by the spell builder; the first that is
create item (24) shows the item, as for the trainer; any other, or none, shows the entry's spell itself. It
tests no attribute bit and sets no pet flag.

Addresses: `0x533e90`, `0x533f61`, `0x533f6d`, `0x533f72`, `0x533f87`, `0x533f92`, `0x533fb8`, `0x533fba`, `0x533fbf`, `0x533fef`, `0x533fdf`, `0x534005`, `0x534015`, `0x534039`, `0x53404a`, `0x53405a`, `0x52e610`, `0x52b650`, `0x85544c`
Confidence: verified
Proof: one
Test: none

The entry list is the table at `0xbdcf78` of count `0xbdcfc0` (`0x533f72`, `0x533f81`). The loop at `0x533fb8` tests
effects 36 (`0x533fba`) and 24 (`0x533fbf`). Learn spell loads the trigger spell at `+0x1b4` (`0x533fef`) for the
spell builder (`0x534005`); create item loads `+0x19c` (`0x534039`) for the item builder with the pseudo GUID
(`0x53404a`). No match calls the spell builder with the entry's spell (`0x533fdf`). A bad index raises the error
at `0x85544c` (`0x53405a`).
