# tradeskill: itemstats

`GetTradeSkillItemStats` (`0x4fdec0`), the one large function of the profession window's bindings. For a recipe row it pushes the tooltip lines of the item the recipe creates, one Lua string per line, and returns how many it pushed. The lines come from the item template and the global strings, in a fixed order, and a few of them are coloured by what the local player can use.
The other functions of the window are `bindings`.

Roots: none, the function rows of this file belong to `ui`

## Entry and gates

The function takes an index. It returns no value for an index out of range, a header row, a spell with no record, or an item whose template is not cached yet. The created item is `EffectItemType[0]` of the spell, as in `recipes`, and the template request carries the callback `0x4fdc30`, which rebuilds the list when the record arrives. The lines are, in order: quality, binding, uniqueness, quest start, container slots or the
usable-class lines, damage and speed, armor, resistances, stat bonuses, spell effects, race and class restrictions, item level, required level, and the skill, spell, honor rank and medal requirements.

Addresses: `0x4fdec0`, `0x4fdef1`, `0x4fdf1b`, `0x4fdf2c`, `0x4fdf3e`, `0x4fdf64`, `0x4fdf95`, `0x4fdfa1`, `0x4ff373`, `0x4fdc30`, `0x55ba30`, `0x4fe9b0`, `0x4fe9d2`, `0x6f3890`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

`0x4fdf1b` bounds the index against the full row count, `0x4fdf3e` loads the spell id and `0x4fdf64` the created item; `0x4fdf95` requests the template and `0x4fdfa1` returns nothing when it is missing. Every line is pushed with `0x6f3890` after the global string is fetched with `0x703bf0`. The sections that need the player begin at `0x4fe9b0`; when the player object is missing `0x4fe9d2` returns the count so far.
`0x4ff373` returns the count in `edi` at the end.

## Quality, binding and uniqueness

When the item's InventoryType (`+0x2c`) is not 0, the first line is the global string `ITEM_QUALITY%d_DESC` for the quality (`+0x1c`), if it is not empty. The bind line comes from the bonding value (`+0x194`): 1 gives `ITEM_BIND_ON_PICKUP`, 2 `ITEM_BIND_ON_EQUIP`, 3 `ITEM_BIND_ON_USE`, and 4 and 5 `ITEM_BIND_QUEST`; any other value gives none.
A uniqueness limit (`+0x5c`) above 0 on a non-container gives `ITEM_UNIQUE` for 1, else `ITEM_UNIQUE_MULTIPLE` with the limit; a nonzero quest-start field (`+0x1a8`) gives `ITEM_STARTS_QUEST`.

Addresses: `0x4fdfac`, `0x4fdfbe`, `0x4fe015`, `0x4fe01c`, `0x4fe034`, `0x4ff380`, `0x4fe03b`, `0x4fe042`, `0x4fe060`, `0x4fe07e`, `0x4fe0ba`, `0x4fe0c1`, `0x4fe0d0`, `0x4fe0ee`, `0x4fe13a`, `0x4fe149`
Confidence: verified
Proof: one
Test: none

`0x4fdfac` tests the InventoryType and `0x4fdfbe` formats the key. `0x4fe01c` loads the bonding value and `0x4fe034` jumps through the five-entry table `0x4ff380`, whose entries `0x4fe03b`, `0x4fe060`, `0x4fe07e` and `0x4fe042` (twice) select the keys. `0x4fe0ba` tests the limit and `0x4fe0c1` skips a container (class 1); `0x4fe0d0` and `0x4fe0ee` pick the one or the many string. `0x4fe13a` tests the quest-start field and `0x4fe149` selects its string.

## Containers and usable classes

For a bag (InventoryType 18) the one line is `CONTAINER_SLOTS` with the slot count (`+0x64`) and the name of the item's ItemSubClass row; with no row or an empty name it gives none. For every other item two lines follow: a class line and a subclass line. A projectile (class 6) shows the name of ItemClass row 6 as the class line; every other class shows the global string of its InventoryType.
The subclass line is the name (`+0x28`) of the ItemSubClass row. A line is red, between `|cffff2020` and `|r`, when the player lacks the proficiency.

Addresses: `0x4fe176`, `0x4fe1bc`, `0x4fe1ed`, `0x4fe216`, `0x4fe23b`, `0x4fe248`, `0x4fe25d`, `0x4fe25f`, `0x4fe264`, `0x4fe281`, `0x4fe288`, `0x4fe28d`, `0x4fe2f5`, `0x4fe2f8`, `0x4fe36f`, `0x4fe389`, `0x4fe39e`, `0x83ddb0`, `0x5e1e70`, `0x84e4e0`, `0x844538`, `0x882748`
Confidence: verified
Proof: one
Test: none

`0x4fe176` to `0x4fe1b7` find the ItemSubClass row by class and subclass. `0x4fe1bc` tests for a bag and `0x4fe216` loads the slot count. `0x4fe248` fetches the proficiency mask of the class (`0x5e1e70`, 0 for a class from 16 up). With a nonzero mask lacking the subclass bit (`0x4fe25d`) the subclass line is red; a weapon (`0x4fe25f`) whose row has a proficiency bit in its field at `+8` (`0x4fe264`)
gets a red class line instead, and a red subclass line only when that bit is missing too (`0x4fe281` marks the subclass red). `0x4fe28d` handles the projectile class and `0x4fe2f5` indexes the string table `0x83ddb0` by InventoryType. A weapon with no ItemSubClass row reads that field through a null pointer at `0x4fe264`, reached only when the class mask from `0x5e1e70` is nonzero and lacks the subclass bit (`0x4fe24d` to `0x4fe262`). The colour is chosen at `0x4fe39e`: a failed check loads the code `|cffff2020` (`0x84e4e0`) and `|r` (`0x844538`), and a passed one the empty string `0x882748`.

## Damage, armor and resistances

When the first damage slot has a nonzero minimum (`+0xb8`) or maximum (`+0xcc`), one line `%d - %d %s%s` shows the minimum rounded down, the maximum rounded up, the school name with a space (when `+0xe0` is nonzero, from `SPELL_SCHOOL%d_CAP`) and the global `DAMAGE`; a weapon then gets a `SPEED` line with the delay (`+0x110`) divided by 100 and truncated.
Only the first damage slot shows. An armor value above 0 (`+0xf4`) gives `%d` and `ARMOR`. The six resistances from `+0xf8`: when all six are equal and nonzero, one `ITEM_RESIST_ALL` line; otherwise one `ITEM_RESIST_SINGLE` line for each nonzero school. The sign character is `+` for a positive value and `-` otherwise.

Addresses: `0x4fe3d0`, `0x4fe405`, `0x4fe4a0`, `0x4fe4c0`, `0x4fe4d8`, `0x4fe507`, `0x4fe530`, `0x4fe547`, `0x4fe54f`, `0x4fe57d`, `0x4fe587`, `0x4fe5b6`, `0x4fe5df`, `0x4fe5f6`, `0x4fe610`, `0x4fe641`, `0x4fe674`, `0x4fe689`, `0x4fe6c7`, `0x808120`, `0x82ee00`
Confidence: verified
Proof: one
Test: `ui::luabind_4fde_get_trade_skill_item_stats_quantize`

`0x4fe3d0` compares both bounds with zero. `0x4fe4a0` adds 0.99999 to a positive maximum before the truncation and `0x4fe4c0` subtracts it from a minimum that is not above zero, using the float at `0x808120`. `0x4fe4d8` formats the line. `0x4fe507` tests the weapon class; `0x4fe530` divides the delay by 100 with the reciprocal multiplication `0x51eb851f` and `0x4fe54f` formats `%s %d`. `0x4fe57d` tests the armor and `0x4fe5b6` formats `%d %s`.
`0x4fe5f6` compares the six resistances and `0x4fe641` picks the sign (`0x2b` plus twice the flag); the per-school loop starts at `0x4fe689` and its format key is `ITEM_RESIST_SINGLE` (`0x4fe6c7`). The school text is `0x82ee00` appended to the name.

## Stat and spell lines

The item has ten stat pairs, a type at `+0x68` and a value at `+0x90`. A pair with a nonzero value and a type of 0, 1, 3, 4, 5, 6 or 7 gives one line, the global `ITEM_MOD_MANA`, `HEALTH`, `AGILITY`, `STRENGTH`, `INTELLECT`, `SPIRIT` or `STAMINA` used as the format with the sign character and the absolute value. The five spell slots, ids at `+0x11c` and charges at `+0x144`, each give an
`ITEM_SPELL_EFFECT` line with the spell's name when the id is above 0 and has a record, plus `ITEM_SPELL_CHARGE_SINGLE` for a charge count of -1 or `ITEM_SPELL_CHARGES` with the absolute count for any other nonzero one. The line does not depend on the trigger of the spell.

Addresses: `0x4fe73a`, `0x4fe748`, `0x4fe751`, `0x4fe76e`, `0x4ff394`, `0x4fe775`, `0x4fe77f`, `0x4fe79d`, `0x4fe7bb`, `0x4fe7c2`, `0x4fe7e0`, `0x4fe7fe`, `0x4fe829`, `0x4fe853`, `0x4fe876`, `0x4fe890`, `0x4fe8c2`, `0x4fe907`, `0x4fe91a`, `0x4fe935`, `0x4fe99a`
Confidence: verified
Proof: one
Test: none

`0x4fe748` tests the value and `0x4fe751` the type for -1. `0x4fe76e` jumps through the table `0x4ff394`; its entries select `ITEM_MOD_MANA` (`0x4fe775`), `HEALTH` (`0x4fe77f`), `AGILITY` (`0x4fe79d`), `STRENGTH` (`0x4fe7bb`), `INTELLECT` (`0x4fe7c2`), `SPIRIT` (`0x4fe7e0`) and `STAMINA` (`0x4fe7fe`), and type 2 and values above 7 skip the pair. `0x4fe829` takes the absolute value and the sign and `0x4fe853` formats.
`0x4fe890` loops the spell slots, `0x4fe8c2` loads the effect key and `0x4fe907` tests the charges; `0x4fe91a` and `0x4fe935` pick the charge key.

## Race and class lines

When the allowed-race mask (`+0x34`) and the allowed-class mask (`+0x30`) each hold at most one bit, one line `RACE_CLASS_ONLY` names the race and the class and is red unless the player is that race and class. Otherwise each mask gives its own line when it leaves out a playable race or any class: `ITEM_RACES_ALLOWED` or `ITEM_CLASSES_ALLOWED` with the names joined by a comma and a space,
red unless the player's race or class is among them. A mask that covers every playable race, or every class, gives no line.

Addresses: `0x4fe9e8`, `0x4fe9fb`, `0x4fea09`, `0x4fea0f`, `0x4fea2e`, `0x4fea71`, `0x4fea8a`, `0x4fea93`, `0x4feadf`, `0x4feb26`, `0x4feb6b`, `0x4feb7e`, `0x4feb94`, `0x4febc2`, `0x4febe6`, `0x4fec38`, `0x4feca1`, `0x4fecb2`, `0x4fed14`, `0x4fed6c`, `0x4fed9f`, `0x4fedeb`, `0x4fee79`, `0x4feed1`, `0x4fef04`, `0x84480c`
Confidence: verified
Proof: one
Test: none

`0x4fe9fb` and `0x4fea09` test each mask for at most one bit (`x & (x - 1)`). The race rows have `0x74` bytes and the class rows `0x44`. `0x4fea2e` skips a race row whose flag bit 0 is set; `0x4fea71` formats the race name with a trailing space; `0x4fea8a` clears the match flag when the player's race differs. `0x4feadf` appends the class name. `0x4feb26` loads the line key; `0x4feb6b` and `0x4feb7e` choose the plain or red form.
`0x4feb94` begins the separate lists: `0x4febc2` tests each race row against the mask and `0x4febe6` records a missing one, `0x4feca1` pushes the separator `0x84480c` of the race list, and `0x4fed14` to `0x4fed6c` show it. `0x4fedeb` and `0x4fee79` do the same for classes, which have no flag test.

## Level and requirement lines

When the required level (`+0x3c`) is above 0, an `ITEM_LEVEL` line shows the item level (`+0x38`), and above 1 an `ITEM_MIN_LEVEL` line shows the required level, red while the player's level is lower. A required skill (`+0x40`) gives `ITEM_MIN_SKILL` with its name and the rank (`+0x44`), or `ITEM_REQ_SKILL` when the rank is 0. That line is red when the player's skill total (`0x5ea520`) is at least the rank
and plain when it is below, the opposite of every other line. A required spell (`+0x48`) gives `ITEM_REQ_SKILL` with its name, red unless the player knows it or a higher rank. A required honor rank (`+0x4c`) and a required medal (`+0x50`) give the same key, red unless the player has them.

Addresses: `0x4fef07`, `0x4fef1a`, `0x4fef75`, `0x4fefbb`, `0x4ff013`, `0x4ff023`, `0x4ff02a`, `0x4ff0dd`, `0x4ff0e9`, `0x4ff0f2`, `0x4ff0fb`, `0x4ff13c`, `0x4ff16e`, `0x4ff181`, `0x4ff215`, `0x4ff230`, `0x4ff237`, `0x4ff2af`, `0x4ff2d4`, `0x4ff2d6`, `0x5ea520`, `0x60c740`, `0x60c8d0`, `0x5efe60`, `0x838044`
Confidence: verified
Proof: one
Test: none

`0x4fef07` loads the required level and `0x4fef1a` the item level key; `0x4fef75` formats the minimum level and `0x4fefbb` compares the player's level (`+0x70` of the unit fields) with it. `0x4ff023` and `0x4ff02a` pick the minimum-skill or required-skill key. `0x4ff0dd` fetches the skill total and `0x4ff0e9` (`setg`) sets the flag when the rank is above it; a set flag takes the plain form at `0x4ff0f2` and a clear one the red form at `0x4ff0fb`.
`0x4ff16e` and `0x4ff181` ask whether the player knows the spell or a higher rank. `0x4ff230` compares the honor rank byte and `0x4ff237` fetches the rank's name for the line. `0x4ff2d4` tests the medal bit and `0x4ff2d6` formats `PVP_MEDAL%d`, whose text fills the line. A skill row that is missing shows `UNKNOWN` (`0x838044`).
