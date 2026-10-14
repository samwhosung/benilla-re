# ui: unit levels and the gray range

`UnitLevel` answers from the unit's level field with two overrides and a roster fallback. The same level arithmetic, the gray-range table and
the difficulty colour are used by the quest, trivial-unit and nameplate code. The unit token is `unittokens`; the plate that shows the level is `nameplates`.

Roots: `0x517fc0`, `0x4e17d0`, `0x7cbce0`, `0x7cbd50`

## UnitLevel

`UnitLevel(unit)` raises `Usage: UnitLevel("unit")` for a missing argument and pushes one number. With the object held (type mask 8) the answer is `UNIT_FIELD_LEVEL` (`+0x70`), unless the first of these applies:

1. The player exists, the unit's reaction toward it is 1 or less and the player's level is at most the unit's minus 10: -1.
2. The unit's creature rank (`0x605620`) is 3, a world boss: -1.
3. The level is above 0, the player's drunkenness is nonzero and the player can attack the unit (`0x606980`): the level less the drunk reduction, at least 1.

With no object held the answer is the level word at `+0x12` of the party or raid record `0x496400` finds, else the number 0.

Addresses: `0x517fc0`, `0x51801a`, `0x518045`, `0x51805a`, `0x51807b`, `0x51807f`, `0x518084`, `0x5180aa`, `0x5180dd`, `0x5180ea`, `0x518110`, `0x518113`, `0x51811c`, `0x51813e`, `0x496400`, `0x605620`, `0x606980`, `0x6061e0`, `0x808bf4`, `0x8029d0`, `0x7ffa24`, `0x40a2b0`
Confidence: verified
Proof: one
Test: `ui::luabind_517fc0_unit_level`

The reduction is `trunc(min(drunk, 100) * 5.0 * 0.01 + 0.5)` (`0x5180c8` to `0x5180dd`). The drunk byte is `PLAYER_BYTES_3` byte 1 (`[player+0xe68]+0x1d`). A level of 0 or less is returned as it is. Rule 1 runs before rule 3 and rule 2 on
its own is independent of the player, so a world boss reads -1 with no active player. No test reads the unit's health, and no branch handles a pet.

Bit-exact coverage: `ui::luabind_517fc0_unit_level` covers `0x5180c8` to `0x5180dd` of `0x517fc0`.

## The level on a plate

`0x7cbce0(player, unit)` is the plate's copy of the same reduction: it returns the unit's level, less `trunc(min(drunk, 100) * 0.05 + 0.5)` when the level is above 0, the player exists, its drunkenness is nonzero and it can attack the unit; never below 1.
It has no -1 leg, so the plate shows the real level where the Lua function pushes -1.

Addresses: `0x7cbce0`, `0x7cbd27`, `0x7cbd36`, `0x8029c8`, `0x7cbb7a`
Confidence: verified
Proof: one
Test: `ui::glue_7cbce0_percent_round`

The two functions form the product in a different order, `(drunk * 5.0) * 0.01` against `drunk * 0.05`. At the client's 53-bit precision the first gives 0.4999999888 for a drunkenness of 10 where the second gives 0.5000000074, so after adding 0.5 and truncating
the reduction is 0 against 1; the same holds at 30, 50, 70 and 90, and the two agree at every other value from 0 to 100 (0 to 5 levels). Under a 24-bit precision control both products round to the same single.

## The gray range

`GetQuestGreenRange()` pushes `gap[min(level / 5, 19)]` for the active player's level, or 0 with no player. `gap` is the table `0x8076c0` of 20 dwords: 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11, then 12 for the last eight. The index
saturates at 19, so a level of 100 and above reads 12.

Addresses: `0x4e17d0`, `0x4e17f4`, `0x4e1814`, `0x4e1824`, `0x4e182e`, `0x8076c0`, `0x80ae98`, `0x81dda8`, `0x5f0700`, `0x7cbd50`
Confidence: verified
Proof: one
Test: none

Two other readers carry their own copy of the same 20 dwords: `0x5f0700` (`0x80ae98`) and the difficulty colour `0x7cbd50` (`0x81dda8`). The division is the signed multiply by `0x66666667` and a shift. `0x4e1827` clamps the index to 19 with an unsigned compare; `0x5f075e` uses the constant 12 for an index of 20 or more.

## Trivial units

`0x5f0700(unit)` answers whether a unit is below the active player's level by more than the table entry: the unit is not a player object (type bit 0x10 clear), the active player exists and `playerLevel - unitLevel`
is above `gap[playerLevel / 5]` (12 from index 20), both raw levels. `UnitIsTrivial(unit)` pushes 1 for a true answer and nil otherwise, with a `Usage:` check; the civilian test (`unitinfo#flag-predicates`) and the plate's skull rule call the same function.

Addresses: `0x5f0700`, `0x5f0719`, `0x5f072f`, `0x5f073a`, `0x5f0743`, `0x5f0763`, `0x5f076c`, `0x5f0773`, `0x519d70`, `0x519dac`
Confidence: verified
Proof: one
Test: none

The compare is signed (`setg`), so a level difference equal to the entry is not trivial. A player unit is never trivial.

## The difficulty colour

`0x7cbd50(playerLevel, unitLevel)` returns the address of one of five ARGB words by the difference d = unit level less player level: d of 5 or more `0xffff1919`, 3 to 4 `0xffff7f3f`, -2 to 2 `0xffffff00`; below that, `0xff3fb23f` when -d is at
most `gap[playerLevel / 5]` and `0xff7f7f7f` above. The words sit in `0xcf60ec` to `0xcf60fc`, written once at first use.

Addresses: `0x7cbd50`, `0xcf6100`, `0x7cbd71`, `0x7cbd7b`, `0x7cbd85`, `0x7cbd8f`, `0x7cbd99`, `0x7cbdd0`, `0x7cbdd3`, `0x7cbddd`, `0x7cbdea`, `0x7cbdf7`, `0xcf60ec`
Confidence: verified
Proof: one
Test: none

The gap is 12 for an unsigned index of 20 or more (`0x7cbdc0`, `jae`). The comparisons are signed on d (`jl`).
