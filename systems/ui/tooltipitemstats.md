# ui: Item damage, armor and stat lines

The lines of an item tooltip between the slot line and the enchantments: damage and speed, damage per
second, armor, block, stat bonuses and resistances. All of them are white, add one line each and stop at
`nameOnly`. The text of every template is a global string; the builder fixes only its arguments. The
builder is in `tooltipitem`, the lines before these in `tooltipitemhead`.

The record's fields: damage minimum `+0xb8`, maximum `+0xcc` and school `+0xe0`, each five long; armor
`+0xf4`; the six resistances `+0xf8` to `+0x10c`; delay in milliseconds `+0x110`; block `+0x1bc`; stat
types `+0x68` and values `+0x90`, ten each.

Roots: `0x52b650`

## Damage lines

A record has five damage slots. Each slot with a nonzero minimum or maximum adds one line. The minimum
is shown rounded down and the maximum rounded up, both with the constant 0.99998999: a minimum of 0 or
less has it subtracted, a maximum above 0 has it added, and the result is truncated. A slot with both
numbers 0 adds no line and is not counted as the first.

The template depends on the slot's school (nonzero or not), on whether the item class is 6 (ammunition),
and, for a plain slot, on whether the two numbers are equal. The first emitted line uses the plain key,
every later one the `PLUS_` key.

| school | class 6 | numbers equal | key | arguments |
|---|---|---|---|---|
| nonzero | yes | | `AMMO_SCHOOL_DAMAGE_TEMPLATE` | average, school name |
| nonzero | no | | `DAMAGE_TEMPLATE_WITH_SCHOOL` | minimum, maximum, school name |
| 0 | yes | | `AMMO_DAMAGE_TEMPLATE` | average |
| 0 | no | yes | `SINGLE_DAMAGE_TEMPLATE` | minimum |
| 0 | no | no | `DAMAGE_TEMPLATE` | minimum, maximum |

Addresses: `0x52c22b`, `0x52c234`, `0x52c245`, `0x52c253`, `0x52c261`, `0x52c263`, `0x52c269`, `0x52c27d`, `0x52c27f`, `0x52c285`, `0x52c290`, `0x52c298`, `0x52c2a0`, `0x52c2a6`, `0x52c2bd`, `0x52c2cb`, `0x52c2e5`, `0x52c2ed`, `0x52c2f7`, `0x52c305`, `0x52c35a`, `0x52c35f`, `0x52c3ae`, `0x52c3c0`, `0x52c3ce`, `0x52c3fc`, `0x52c402`, `0x52c407`, `0x52c44a`, `0x52c44f`, `0x52c473`, `0x52c517`, `0x52c521`, `0x52c537`, `0x52c547`, `0x808120`, `0x7ffd74`, `0x40a2b0`, `0x530270`
Confidence: verified
Proof: one
Test: none

The loop runs five times with a cursor from `record+0xcc` (`0x52c22b`, `0x52c245`); minimum, maximum and
school are read at the cursor minus 0x14, at the cursor and at the cursor plus 0x14 (`0x52c253`,
`0x52c26e`, `0x52c29b`). The conversion `0x40a2b0` truncates toward zero. `0x52c261 je` skips the
subtraction for a positive minimum and `0x52c27d jne` skips the addition for a maximum of 0 or less. The
constant at `0x808120` is the float `0x3f7fff58`. The school key is built with `SPELL_SCHOOL%d_CAP` and
the raw school number and resolved (`0x52c2a6` to `0x52c2cb`). The first-line flag `[ebp-0x10]` starts at
1 (`0x52c234`) and is cleared only after a line is added (`0x52c521`). Each pair of keys is chosen by a
`mov` of the plain key and a `jne` that skips the `mov` of the `PLUS_` key on the first line (`0x52c305`,
`0x52c35f`, `0x52c3ce`, `0x52c407`, `0x52c44f`). The average of the ammunition lines is
`(float)(minimum + maximum) * 0.5` of the rounded integers, with no division by the delay (`0x52c2ed`,
`0x52c3b6`). The numbers are pushed maximum first, so the minimum is the first `%d` (`0x52c472`,
`0x52c473`). The line is added by `0x530270` with white for both cells and wrap 0 (`0x52c517`).

## Speed and damage per second

The first damage line of a weapon (class 2) has a right cell, the word `SPEED` and the delay in seconds
as `"%s %.2f"`, a literal rather than a key; every other line has an empty right cell. After the loop, a
weapon with at least one damage line adds `DPS_TEMPLATE` with one number: the sum over the slots of
`(minimum + maximum) / 2` taken from the unrounded floats, divided by the delay in seconds. A slot without
a line adds nothing to the sum. Ammunition shows neither. There is no guard for a delay of 0.

Addresses: `0x52c48f`, `0x52c494`, `0x52c49c`, `0x52c4a3`, `0x52c4bf`, `0x52c4ce`, `0x52c4de`, `0x52c4f3`, `0x52c51c`, `0x52c528`, `0x52c52b`, `0x52c531`, `0x52c54d`, `0x52c552`, `0x52c55a`, `0x52c561`, `0x52c580`, `0x52c595`, `0x52c59b`, `0x52c5c1`, `0x854c5c`, `0x7ffa24`, `0x801360`
Confidence: verified
Proof: one
Test: none

`0x52c494` and `0x52c49c` jump to `0x52c4f3`, which stores an empty string in the right cell, unless
the first-line flag is set and the class is 2. The delay is `fild [rec+0x110]` times the float `0x801360`
(0.001, `0x52c4bf`, `0x52c4ce`), pushed as a double. The accumulator at `[ebp-0x24]` takes
`(max + min) * 0.5` of the floats after the line is added (`0x52c51c` to `0x52c534`). The DPS test needs a
cleared first-line flag (`0x52c552`) and class 2 (`0x52c55a`); the quotient is `fdivr` of the
accumulator by the delay (`0x52c59b`) and the line is added with white and wrap 0 (`0x52c5c1`).

## Armor and block

A record with an armor value above 0 adds `ARMOR_TEMPLATE` with the value, and one with a block value
above 0 adds `SHIELD_BLOCK_TEMPLATE` with the value, each in white. They follow the damage lines.

Addresses: `0x52c5c6`, `0x52c5c9`, `0x52c5cf`, `0x52c5d1`, `0x52c5d8`, `0x52c62a`, `0x52c632`, `0x52c63a`, `0x52c641`, `0x52c693`
Confidence: verified
Proof: one
Test: none

`0x52c5c9` reads `+0xf4` and `0x52c5d1 jle` skips a value of 0 or less; the format and the value are
printed by `0x64a7f0` and added by `0x5303b0` (`0x52c62a`). `0x52c632` reads `+0x1bc` with the same
test (`0x52c63a`).

## Stat lines

Each of the ten stat slots with a nonzero value adds a line of its type's key, with a sign character and
the absolute value. The lines come out in a fixed order of types, not slot order: strength, agility,
stamina, intellect, spirit, health, mana. Types 2 and above 7 add nothing.

A client fault: a mana stat (type 0) adds its line 27 times, once in its place and 26 more after the
last stat line, because the order table has 37 entries, the 26 after the last real one are 0, and 0 is
mana's type.

| type | key |
|---|---|
| 0 | `ITEM_MOD_MANA` |
| 1 | `ITEM_MOD_HEALTH` |
| 3 | `ITEM_MOD_AGILITY` |
| 4 | `ITEM_MOD_STRENGTH` |
| 5 | `ITEM_MOD_INTELLECT` |
| 6 | `ITEM_MOD_SPIRIT` |
| 7 | `ITEM_MOD_STAMINA` |

Addresses: `0x52c698`, `0x52c69b`, `0x52c6a1`, `0x52c6b0`, `0x52c6b8`, `0x52c6c4`, `0x52c6ca`, `0x52c6d3`, `0x52c6dd`, `0x52c6e3`, `0x52c6ea`, `0x52c6f4`, `0x52c715`, `0x52c733`, `0x52c73a`, `0x52c758`, `0x52c776`, `0x52c7a1`, `0x52c7a5`, `0x52c7ac`, `0x52c7b6`, `0x52c7cb`, `0x52c7e4`, `0x52c7e9`, `0x52c7ec`, `0x52c7f9`, `0x808e88`, `0x52e510`
Confidence: verified
Proof: one
Test: none

The outer loop advances a byte offset `[ebp-0x24]` from 0 to `0x94` in steps of 4 (`0x52c7f3` to
`0x52c801`); each pass loads the wanted type from `0x808e88 + offset` (`0x52c6c4`) and scans all ten
slots: the cursor `[ebp-0x28]` starts at `record+0x90` and is not advanced between passes (`0x52c69b`,
`0x52c6b0`). A slot shows when its value is nonzero (`0x52c6b8`), its type (at the value minus 0x28)
equals the wanted type (`0x52c6ca`) and the type is at most 7 (`0x52c6d3`); the switch is the eight-entry
table at `0x52e510` (`0x52c6e3`). The table at `0x808e88` reads 4, 3, 7, 5, 6, 1, 0, 8, 9, 2, 10 and then
26 zeros. The sign is `+` for a positive value and `-` otherwise (`0x52c7ac`), and the arguments are the
sign and the absolute value (`0x52c7a5`, `0x52c7cb`).

## Resistance lines

Six resistance fields, Holy to Arcane. When all six are equal and nonzero, one `ITEM_RESIST_ALL` line
shows the common value. When they differ, one `ITEM_RESIST_SINGLE` line is added for each nonzero field
other than Holy, in the order Arcane, Fire, Nature, Frost, Shadow, with the sign, the absolute value and
the school's name. Holy is only ever part of the all-equal test: a record with Holy 0 and the five others
equal shows five lines, and a record with only Holy set shows nothing. All six equal and 0 adds nothing.

Addresses: `0x52c807`, `0x52c80a`, `0x52c820`, `0x52c822`, `0x52c82c`, `0x52c831`, `0x52c833`, `0x52c85a`, `0x52c8a3`, `0x52c8a8`, `0x52c8ad`, `0x52c8b2`, `0x52c8b5`, `0x52c8bc`, `0x52c8c1`, `0x52c8ca`, `0x52c8d1`, `0x52c8fa`, `0x52c92d`, `0x52c93a`, `0x52c97c`, `0x52c981`, `0x52c985`, `0x84e4cc`
Confidence: verified
Proof: one
Test: none

The test loads `+0xf8` into `edx` and compares `+0xfc` to `+0x10c` against it (`0x52c80a` to `0x52c82f`);
a difference jumps to the single lines (`0x52c822`). The singles loop runs `edi` from 1 to 5 and sets
`esi` to 6 when `edi` is 1, else to `edi` (`0x52c8b2` to `0x52c8bc`); the value is `record + 0xf4 + 4*esi`
(`0x52c8c1`), which makes the order Arcane (6), then 2 to 5; a zero skips (`0x52c8ca`). The school name is
`SPELL_SCHOOL%d_CAP` with `esi`, copied into a 0x20-byte buffer (`0x52c8d1` to `0x52c904`), and is the
last argument of `ITEM_RESIST_SINGLE` (`0x52c93a`).
