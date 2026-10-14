# ui: Spell tooltip requirements and description

The later lines of a spell tooltip: the totems, the required item, form and reputation, the reagents, the
remaining cooldown, the chance lines, the use-all line and the description. A requirement is white when met
and red when not. The builder, its arguments and the first lines are in `tooltipspell`; the `$` tokens of the
description in `spelltokens`.

An unmet item is marked inside a line with the inline escape `|cffff2020` and closed with `|r`; the builder
uses this escape for totems and reagents, where one line holds several items, and the colour pointers for
the other lines. The spell record's columns are four bytes wide and sit at four times their index.

Roots: `0x52e610`

## The record fields the builder reads

| offset | column | meaning |
|---|---|---|
| `+0x18`, `+0x1c`, `+0x20`, `+0x24` | 6 to 9 | attributes, attributes ex, ex2, ex3 |
| `+0x2c` | 11 | shapeshift forms that allow the spell |
| `+0x34` | 13 | target flags |
| `+0x4c`, `+0x50` | 19, 20 | recovery and category recovery |
| `+0x7c` | 31 | power type |
| `+0xa0`, `+0xa8`, `+0xc8` | 40, 42, 50 | two totem items; eight reagent items and their counts |
| `+0xe8`, `+0xec` | 58, 59 | required item class and subclass mask |
| `+0xf4` | 61 | the first effect |
| `+0x19c` | 103 | the first effect's item type |
| `+0x1e0`, `+0x204` | 120, 129 | name and rank |
| `+0x228`, `+0x24c` | 138, 147 | description and aura description |
| `+0x2a8`, `+0x2ac` | 170, 171 | required faction and standing rank |

Addresses: `0x52e610`, `0x52eb23`, `0x52ed56`, `0x52f2ae`, `0x52f3a1`, `0x52eeb4`, `0x52f10a`
Confidence: verified
Proof: one
Test: `ui::glue_512_52e610_timer_ratios`

Each offset is read at the line that uses it: `+0xe8` at `0x52eeb4`, `+0xec` at `0x52eec5`, `+0x2c` at `0x52f10a`,
`+0x2a8` at `0x52f2ae`, `+0xa0` at `0x52ed56`, `+0xa8` at `0x52f3a1`, and `+0xf4` at `0x52eb23`. The store
loader checks 173 fields and 692-byte records, so `+0xf4` is the first of three effect columns and
the spell icon column is at `+0x1d4`, not at `+0xf4`.

## Totems

A spell with totem items (`+0xa0`, two slots) adds `SPELL_TOTEMS` with the totems' names, separated by
commas, in white. A totem the player does not carry has its name inside the red escape. The line is
omitted for the pet's spells, and for a compact tooltip when every totem is carried. A totem whose
item record is not cached yet is left out until the cache answers.

Addresses: `0x52ed43`, `0x52ed50`, `0x52ed70`, `0x52ed77`, `0x52ed9c`, `0x52eda4`, `0x52edaf`, `0x52edba`, `0x52edc6`, `0x52edd7`, `0x52edeb`, `0x52ee12`, `0x52ee1d`, `0x52ee34`, `0x52ee47`, `0x52ee55`, `0x52ee7c`, `0x52ee83`, `0x52ee8a`, `0x52ee91`, `0x52eea2`, `0x622270`, `0x854120`, `0x844538`, `0x84480c`
Confidence: verified
Proof: one
Test: none

`0x52ed50` skips when p4 is set. The loop runs twice over `+0xa0` (`0x52ed65`, `0x52ee6f`). Each item is
asked of the item cache with the callback `0x52f820` (`0x52eda4`); a miss counts in `+0x3b0` (`0x52edaf`).
The first name starts the buffer with `SPELL_TOTEMS` (`0x52edc6`), later ones are joined with `", "`
(`0x52edeb`). `0x622270(player + 0x1d38, item, 0)` finds a carried item (`0x52ee12`); none puts the
escape pointer `0x854120` (the string `|cffff2020`) in front (`0x52ee1d`), clears the met flag `[ebp-0xc]`
(`0x52ee34`) and adds `|r` (`0x844538`) after the name (`0x52ee55`). `0x52ee83` to `0x52ee8f` skip a met,
compact line; `0x52eea2` adds it with white and wrap 0.

## The required item

A spell that requires an equipped item class and subclass mask (`+0xe8`, `+0xec`) adds `SPELL_EQUIPPED_ITEM`
with a name for the requirement, with wrap 1: white when the player has such an item equipped, red when not,
and for the pet always met. A compact tooltip leaves out a met line and uses `SPELL_EQUIPPED_ITEM_NOSPACE`
for an unmet one. A spell whose target flags (`+0x34`) have bit `0x10`, one with a negative class, and one
with an empty mask has no line. The name is the group name of the item-subclass-mask table when its row
matches the class and the whole mask; otherwise the long names of every matching item subclass, joined with
commas, each falling back to the display name.

Addresses: `0x52eea7`, `0x52eeaa`, `0x52eeb4`, `0x52eebf`, `0x52eec5`, `0x52eed0`, `0x52eed6`, `0x52eef0`, `0x52ef04`, `0x52ef0b`, `0x52ef23`, `0x52ef34`, `0x52ef39`, `0x52ef43`, `0x52ef56`, `0x52ef5d`, `0x52ef73`, `0x52ef87`, `0x52ef95`, `0x52ef9d`, `0x52efab`, `0x52efed`, `0x52f002`, `0x52f011`, `0x52f01d`, `0x52f03d`, `0x52f04a`, `0x52f05b`, `0x52f078`, `0x52f07f`, `0x52f084`, `0x52f09d`, `0x52f0a4`, `0x52f0e7`, `0x52f0f8`, `0x5f0c50`, `0xc0dba4`, `0xc0dba8`, `0xc0db90`, `0xc0db94`
Confidence: verified
Proof: one
Test: none

The gates are `0x52eeaa`, `0x52eebf` and `0x52eed0`, each jumping to `0x52f10a`. Stage one scans the table
`0xc0dba4` (count `0xc0dba8`, 0x30 bytes a row) for a row whose class and mask equal the spell's
(`0x52ef04`, `0x52ef0b`) and copies its name at `[row + locale*4 + 8]` (`0x52ef23`, `0x52ef34`), setting
`[ebp-0x14]` (`0x52ef39`). Stage two scans the item-subclass table (`0xc0db90`, 0x74 bytes a row) for the class
(`0x52efed`) and the bit `1 << subclass` in the mask (`0x52efff`, `0x52f002`), skipped when stage one matched
(`0x52f00a`), and appends the name at `+0x4c`, else `+0x28` (`0x52f03d`, `0x52f04a`). The met test is
`0x5f0c50(player, spell, mask)` with the slot mask `0x8000` for attribute-ex3 bit `0x400`, `0x10000` for
bit `0x1000000`, else all (`0x52ef43` to `0x52ef73`), and again with `0x10000` when both bits are set
(`0x52ef87` to `0x52efa6`). p4 forces met (`0x52f078`, `0x52f07f`). Key choice: `0x52f08e` to `0x52f0a9`;
colour and wrap 1: `0x52f0e7`, `0x52f0f8`.

## The required form

A spell with shapeshift forms in `+0x2c` adds `SPELL_REQUIRED_FORM` with the forms' names, separated by
commas, with wrap 0: white when the caster's current form is one of them and red when not, and a
caster with no form is always unmet. The line is left out when attributes-ex2 (`+0x20`) has bit `0x80000`,
when the form table is empty or no named form is in the mask, and, for a compact tooltip, when met; a
compact unmet line uses `SPELL_REQUIRED_FORM_NOSPACE`. The mask's bit `b` names the `b`th record of the form
table by position, not by id.

Addresses: `0x52f10a`, `0x52f10f`, `0x52f118`, `0x52f11d`, `0x52f127`, `0x52f12d`, `0x52f139`, `0x52f144`, `0x52f14b`, `0x52f150`, `0x52f153`, `0x52f161`, `0x52f173`, `0x52f177`, `0x52f17b`, `0x52f18a`, `0x52f194`, `0x52f1ae`, `0x52f1c9`, `0x52f1d5`, `0x52f1dd`, `0x52f1e3`, `0x52f1f2`, `0x52f1fc`, `0x52f20b`, `0x52f22b`, `0x52f230`, `0x52f23b`, `0x52f241`, `0x52f248`, `0x52f28b`, `0x52f29c`, `0x52f2a6`, `0xc0d76c`, `0xc0d770`, `0x52e747`, `0x52e72d`
Confidence: verified
Proof: one
Test: none

`0x52f10a` skips an empty `+0x2c`; `0x52f118` masks bit `0x80000` of `+0x20` and `0x52f127` jumps over the name
loop when set, leaving the first-name flag `[ebp-0x8]` at 1, which `0x52f22b` and `0x52f230` read as no line. The
loop runs `i` over the form table's count `0xc0d770` (`0x52f12d`, `0x52f1dd`); the form table is `0xc0d76c`
and a row is 0x38 bytes (`0x52f1d5`). A row is used when bit `i` of the mask is set (`0x52f14b`, `0x52f150`) and
its name `[row + locale*4 + 8]` is nonempty (`0x52f173` to `0x52f17b`); the names are joined with `", "`
(`0x52f1ae`, `0x52f1c9`). The caster is the player, or the pet unit for p4 (`0x52e72d`, `0x52e747`); the
current form is the byte at `fields+0x212` (`0x52f1f2`), and the test is `mask(1 << (form − 1)) & Stances`
(`0x52f1fc` to `0x52f20b`). The branch at `0x52f210` runs only on the path that already left the line out.
The text is `SStrPrintf` limited to 0x80 bytes (`0x52f2a6`).

## The required reputation

A spell with a required faction (`+0x2a8`) adds `ITEM_REQ_REPUTATION` with the faction name and standing
label: white when the player's standing is at least the minimum of the rank, red when not. A met line is
left out of a compact tooltip.

Addresses: `0x52f2ae`, `0x52f2b4`, `0x52f2c5`, `0x52f2ca`, `0x52f2d5`, `0x52f2e1`, `0x52f2e6`, `0x52f2fc`, `0x52f307`, `0x52f30d`, `0x52f33b`, `0x52f345`, `0x52f351`, `0x52f375`, `0x52f38e`, `0x4d6370`, `0x612bf0`, `0x80928c`, `0xc0dd50`, `0xc0dd54`, `0x84b5dc`
Confidence: verified
Proof: one
Test: none

`0x4d6370` returns the standing (`0x52f2c5`) and the compare against `0x80928c[rank]` is `setge` (`0x52f2ca`,
`0x52f2d5`). The line has the same text as the item's reputation requirement (`tooltipitemreq`).

## The reagents

A spell with reagents (`+0xa8`, eight slots with counts at `+0xc8`) adds `SPELL_REAGENTS` with their names,
separated by commas, a count above 1 in `"%s (%d)"` form, in white and wrapped. A reagent the player does
not carry has its name inside the red escape. The line is omitted for the pet's spells and, for a compact
tooltip, when every reagent is carried.

Addresses: `0x52f393`, `0x52f3a1`, `0x52f3c0`, `0x52f3d5`, `0x52f3f4`, `0x52f3ff`, `0x52f40a`, `0x52f416`, `0x52f434`, `0x52f43b`, `0x52f451`, `0x52f45c`, `0x52f472`, `0x52f49f`, `0x52f4aa`, `0x52f4c1`, `0x52f4d7`, `0x52f4e0`, `0x52f50c`, `0x52f513`, `0x52f51a`, `0x52f521`, `0x52f532`, `0x854b30`, `0x854e54`
Confidence: verified
Proof: one
Test: none

The structure matches the totem loop with eight iterations (`0x52f3b5`, `0x52f506`). The count is read from
`+0x20` of the item cursor, which sits 0x20 bytes past the reagent id (`0x52f454`), and a count above 1 uses
`"%s (%d)"` (`0x52f461`, `0x52f472`). The carried test is `0x622270(player + 0x1d38, item, 0)`
(`0x52f49f`). The line is added with white and wrap 1 (`0x52f524`, `0x52f532`).

## The remaining cooldown line

A spell with a remaining cooldown (p3) and a recovery of at least a minute adds `ITEM_COOLDOWN_TIME` with
the time left, white, and the builder returns 1. The recovery tested is the larger modified one; a short
cooldown shows only in the recast cell.

Addresses: `0x52f537`, `0x52f53c`, `0x52f543`, `0x52f54a`, `0x52f54c`, `0x52f560`, `0x52f568`, `0x52f577`, `0x52f57c`, `0x52f581`, `0x52fa50`
Confidence: verified
Proof: one
Test: none

`0x52f537` loads p3 and `0x52f543` compares `[ebp-0x38]`, the larger recovery, with 60000 (`0x52f54a`). The text
is `0x52fa50(p3, ITEM_COOLDOWN_TIME, 0, 1)` (`0x52f560`) added white with wrap 0 (`0x52f568`, `0x52f577`); the
return flag is `esi` set to 1 (`0x52f57c`).

## The chance lines

A passive spell whose first effect is dodge (20), parry (22) or block (23), and a spell whose first effect is
attack (78), whether passive or not, adds one line of the player's current chance: `CHANCE_TO_DODGE`,
`CHANCE_TO_PARRY`, `CHANCE_TO_BLOCK` or `CHANCE_TO_CRIT`, in white. The percentage is a float of the
player's field block, passed as a double to the template. A compact tooltip has no such line.

| first effect | key | field of the player block |
|---|---|---|
| 20 | `CHANCE_TO_DODGE` | `+0xe5c` |
| 22 | `CHANCE_TO_PARRY` | `+0xe60` |
| 23 | `CHANCE_TO_BLOCK` | `+0xe58` |
| 78 | `CHANCE_TO_CRIT` | `+0xe64` |

Addresses: `0x52f5b1`, `0x52f5b4`, `0x52f5ba`, `0x52f5bd`, `0x52f5bf`, `0x52f5c4`, `0x52f5ca`, `0x52f5cf`, `0x52f5d4`, `0x52f5dd`, `0x52f5e0`, `0x52f5e9`, `0x52f5f0`, `0x52f5f7`, `0x52f60d`, `0x52f623`, `0x52f639`, `0x52f64d`, `0x52f652`, `0x52f687`, `0x52f7c4`, `0x52f7d8`
Confidence: verified
Proof: one
Test: none

`0x52f5ba` compares the first effect with 78 and jumps over the passive test to the dispatch (`0x52f5bd`);
`0x52f5bf` reads the passive flag `[ebp-0x34]`, set by the cast-time line, and `0x52f5ca`, `0x52f5cf` and
`0x52f5d4` admit 20, 23 and 22. The effect minus 20 indexes the byte table at `0x52f7d8` and then the
five-entry jump table at `0x52f7c4` (`0x52f5e0` to `0x52f5f0`). Each arm loads the float field (`0x52f5f7`,
`0x52f60d`, `0x52f623`, `0x52f639`), widens it to a double (`0x52f652`) and formats the global string
with `SStrPrintf` limited to 0x80 bytes; `0x52f687` adds it white with wrap 0.

## The use-all line

A spell whose attributes-ex (`+0x1c`) have bit 2 adds `SPELL_USE_ALL_` followed by the power's name, white;
a power type outside 0 to 4 uses `SPELL_USE_ALL_HEALTH`.

Addresses: `0x52f697`, `0x52f69b`, `0x52f69d`, `0x52f6a9`, `0x52f6b2`, `0x52f6bc`, `0x52f6e3`, `0x854180`, `0x854dfc`
Confidence: verified
Proof: one
Test: none

`0x52f697` tests bit 2 of `+0x1c`. The power type (`+0x7c`) below 5 indexes the key array at `0x854180`
(`0x52f6a9`); else the key at `0x854dfc` is used (`0x52f6b2`). The line is added with white and wrap 0
(`0x52f6e3`).

## The description

A spell with a description (`+0x228`) adds it in gold, wrapped, with its tokens expanded for the pet's
level data when p4 is set. A compact tooltip stops before it.

Addresses: `0x52f6e8`, `0x52f6ee`, `0x52f6f5`, `0x52f6fe`, `0x52f715`, `0x52f717`, `0x52f729`, `0x5075f0`, `0x530380`
Confidence: verified
Proof: one
Test: none

`0x52f6ee` reads `[record + locale*4 + 0x228]`; an empty text skips (`0x52f6f9`). `0x5075f0(spell; buffer, 0x400,
p4, 0, 0, 1, 0)` expands it (`0x52f717`) and `0x530380(text, 0, 1)` adds it as a gold line with wrap 1
(`0x52f729`).
