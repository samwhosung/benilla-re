# objectlayer: player spells and combat

The player's own spell book and skills, the weapon each hand holds, whether an item can be used,
the cast time, the melee attack and the sheathe toggle, and the spell messages of the combat log.
The spell system (`spell`) owns casting and the spell tables; the formatting of the combat log is
UnitCombatLog_C.cpp's, reached here as calls. Spell.dbc offsets are the record's: `+0x18`
Attributes, `+0x0c` the spell-book tab, `+0xe8` and `+0xec` the equipped class and subclass mask,
`+0xf4` the first of three effects, `+0x1a8` the first misc value.

Roots: `0x5e9c20`, `0x5e9fe0`, `0x5e6510`, `0x5ea930`, `0x5ecb70`, `0x5ecac0`, `0x5eb480`, `0x5e7f90`, `0x5e85e0`

## Learning a spell

`0x5e9c20(spell, x, a3, notify)` records a spell the player knows. An id above the store's bound
prints `NOSPELLIDFOUND|%d` and stops. With `notify` it first calls `0x4b25b0(spell, a3, 0)`. A
spell with Attributes 0x20 is added to its skill line's node in the table `+0x1c74`; one with a
tab above 0 to that tab's array at `+0x1cd0`, once. Otherwise effect 0x2f stores it by its misc
value at `+0x1d10`, effect 0x65 makes it the feed-pet spell, and any effect 0x3c records it under
its weapon proficiency in `+0x1c9c`. Effect 0x28 as the first also stores it in `0xc4d770`.

Addresses: `0x5ea2c0`, `0x5ea330`, `0x5ea360`, `0x5ea380`, `0x5eab70`, `0x5f0ee0`, `0x5f10e0`, `0x5e9c20`, `0x5e9c36`, `0x5e9c8e`, `0x5e9d83`, `0x5e9e1c`, `0x5e9e53`, `0x5e9fc1`, `0xc4d770`
Confidence: verified
Proof: one
Test: none

`0x5e9c36` compares with `[0xc0d78c]`. The skill line is `FindSkillLineAbility(race, class, spell)`
`+4` in `SkillLine`; its node (`0x5f18f0`) gets the spell appended to `+0x18`, then `0x4fca20(1)`.
The tab array is `+0x1cd0 + 16·tab`, then `0x4f60c0`. The proficiency is the first `ItemSubClass`
row (`0xc0db90`, 0x74 bytes) whose class equals `+0xe8` and `1 << subclass` equals `+0xec`; its
`+0x70` keys the node, whose `+0x18` takes the spell. The node byte `+0x14` takes byte 3 of the
spell argument, 0 for every caller. All paths end in `0x4d7d40` and `0x4f2f10`. The argument `x`
is unused. The getters: `0x5ea2c0` a skill line's array, `0x5ea330` a tab's (1 to 3), `0x5ea360`
an effect slot (0 to 3), `0x5ea380` a proficiency's spell, though nothing reaches it; `0x5eab70`
whether `0xc4d770` is set.
The nodes are `TRADESKILLLINE` (0x28 bytes, `0x5f0ee0`, `0x5f0e90`, `0x5f0fa0`) and
`PROFICIENCYSPELL` (0x1c, `0x5f10e0`, `0x5f1060`, `0x5f11a0`).

## The opener list

The client keeps a list of the spells it has learned whose first effect opens a lock, in the order the
server taught them. A lock that a spell opens uses the first spell in that order that is strong
enough.

Addresses: `0x4b25b0`, `0x4b265e`, `0x4b2665`, `0x4b2670`, `0x4b2689`, `0xb700b0`, `0xb700b4`, `0xb700b8`, `0x408d80`, `0x5f83d0`, `0x5f845d`, `0x5f8470`, `0x5f8476`, `0x5f84a1`, `0x5f84af`, `0x5f84f8`, `0x5f850f`, `0x5f8511`, `0x5f858c`
Confidence: verified
Proof: one
Test: none

`0x4b25b0` appends the spell id to the growable array at `0xb700b0` (count `0xb700b4`, data
`0xb700b8`) through `0x408d80` only when effect 0 is 0x21, `OPEN_LOCK` (`0x4b265e`, `0x4b2665`); the
learned spell is also marked in the known-spell bitmap, which holds every spell. `0x5f83d0` walks the
list from index 0 upward (`0x5f845d` to `0x5f8476`) and each spell's three effects (`0x5f84a1`): a match of effect
0x21 with the lock slot's index (`0x5f84af`) writes the spell id (`0x5f84f8`) and the effect's points and the
requirement before the comparison (`0x5f850f`); only a satisfied comparison leaves all the loops
(`0x5f8511`, `0x5f858c`), so a weaker match is overwritten by a later one and the first satisfying
spell wins.

## Unlearning a spell

`0x5e9fe0(spell, silent)` removes a spell from its skill line's node and from its tab's array,
compacting each. A spell without Attributes 0x20 and with no tab above 0 shows
`ERR_SPELL_UNLEARNED_S` unless `silent` or its Attributes byte has 0x80. Then it calls
`0x4b2c50(spell, 0)` and `0x4e5e20(spell)`, without `silent` also `0x4d7d40` and `0x4f2f10`, clears
`0xc4d770` when it held the spell, and calls `0x494df0` when `0x494e10()` returns it.

Addresses: `0x5e9fe0`, `0x5ea170`, `0x5ea292`
Confidence: verified
Proof: one
Test: none

The skill-line path searches the node's array from its end and ends in `0x4fca20(1)`, then zeroes
the message flag (`0x5ea170`). A tab above 0 is removed with `0x4f60c0` and no message; the
message path is `0x5ea292`.

## Spell book messages

0x12b (`0x5e61c0`) reads a u16 spell and an i16 and learns it with notification; with no player
yet, the pair is appended to the initial list. 0x12c reads an old and a new u16 spell, calls
`0x4e5e60` and `0x4b2f50` with them, then unlearns the old silently and learns the new, or
replaces the id in the initial list. 0x12a reads a byte, the initial list (u16 spell and i16
each) and the cooldowns: u16 spell, u16 item, u16 category, i32 spell time and i32 category time,
whose bit 31 is a flag. 0x129 reads the 120 action buttons into `0xc4c2b4`.

Addresses: `0x5e61c0`, `0x5e6330`, `0x5e6510`, `0x5e6680`, `0xc4d2a0`, `0xc4c2b4`
Confidence: verified
Proof: one
Test: none

The list is capacity `0xc4d2a0`, count `+4`, data `+8` (`InitialSpellStruct`, 4 bytes), chunk
`+0xc`; 0x12a grows it only when the count passes both the old count and the capacity. Each
cooldown goes to `SetSpellCooldownRemaining(spell, item, category, time, category time & 0x7fffffff,
bit 31, 0, 0)`. The leading byte of 0x12a is read and dropped. The arrival `0x5dea50` learns the
list (`players`).

## Skills

The 128 skill entries at `[+0xe68]+0x848`, 12 bytes each, hold an id, the value and max, and a
temporary and a permanent bonus. `0x5ea3f0` finds an id's index; `0x5ea460` returns the base
(value plus permanent bonus when the value is above 0) and the temporary bonus; `0x5ea520`
their sum, 0 when negative. Slot 42, `0x5ea690`, gives a spell's skill: the total of the skill line
of the spell `0x5ea5d0` substitutes, capped at five times the spell's max level `+0x6c`.

Addresses: `0x5ea3f0`, `0x5ea460`, `0x5ea520`, `0x5ea690`, `0x5ea5d0`, `0x5ec720`, `0x5eaae0`
Confidence: verified
Proof: one
Test: none

Only the active player reads its fields; another player reads every id as 0. `0x5ea5d0` replaces a
spell needing a weapon (`+0xe8` 2, a nonzero mask, effect not 0x19, `+0x34` bit 0x10 and `+0x20`
bit 0x8000000 clear) with the weapon-skill spell of the first hand whose weapon's subclass the mask
holds. `0x5ec720(i)` gives the total for the spell at `[0xb700ac] + 4·i`; `0x5eaae0(record)`
compares the skill at `+0xac` with `+0xb0`.

## Weapon skills

At world entry `0x5e2ca0` builds, from Spell.dbc, a table from weapon subclass to a spell, and the
spell `0xc4d688`. It walks the spells from the last to the first; among those with Attributes 0x40,
the first with effect 0x1a becomes `0xc4d688`, and each with class 2 and a single subclass bit
writes its id for that subclass, so the lowest id wins. `0x5ed9d0(hand)` gives a hand's weapon
spell, the unarmed one (`0x53f8a0`) for no weapon. Slots 43 and 44 return the skill values of
`0xc4d688`'s line and of a hand's weapon spell; `0x5edae0` the ranged weapon's.

Addresses: `0x5e2ca0`, `0x5e2db0`, `0x5e2e10`, `0xc4d730`, `0xc4d688`, `0x5ed9d0`, `0x5eda20`, `0x5eda80`, `0x5edae0`
Confidence: verified
Proof: one
Test: none

The table is `0xc4d730` (capacity, count `+4`, data `+8`), sized one past the highest subclass of
class 2 in `ItemSubClass` (`0x5e2db0`). `0x5e2e10(mask)` returns the single bit's index. A skill
value is `0x5ea460` of the `SkillLineAbility` row of the spell whose spell field `+8` matches.

## Hand weapons

Slot 38 (`0x5ec240(hand, raw)`) returns a hand's weapon record: hand 0, 1 and 2 are slots 15, 16
and 17. Without `raw`, a disarmed player has no main-hand weapon, and no off-hand weapon when the
main hand holds none; an item whose visible flag bit 0 is set counts as none. The record is five
bytes at `+0x1d50 + 8·hand`: the class, subclass and three more from the item record. Slot 40
returns the display id, slot 39 its `ItemDisplayInfo` row.

Addresses: `0x5ec240`, `0x5ec380`, `0x5ec350`, `0x5f0d60`
Confidence: verified
Proof: one
Test: none

Disarmed is `UNIT_FIELD_FLAGS` 0x200000 and weapon is class 2, both tested on `raw` calls of the
other hand. The five bytes come from `0x62ea20`, `0x62ea80`, `0x62eba0`, `0x62eae0`, `0x62ec00`;
the display from `0x62eb40`, the row from the store `0xc0dc08`.

## Using an item

`0x5ea930(record, &error)` says whether the player can use an item record: no record is
`ERR_ITEM_NOT_FOUND`; a missing proficiency for its class and subclass `ERR_PROFICIENCY_NEEDED`;
too low a level, a class or race outside its masks, too low a required skill, an unknown required
spell (nor a higher rank), too low an honor rank, a missing medal bit, or too low a standing with
its faction, `ERR_CANT_USE_ITEM`.

Addresses: `0x5ea930`, `0x80928c`, `0x5e1e70`, `0xc4d4a0`, `0x5ea720`
Confidence: verified
Proof: one
Test: none

The record fields are `+0x3c` level, `+0x30` and `+0x34` the class and race masks, `+0x40` and
`+0x44` the skill, `+0x48` the spell, `+0x4c` the rank against `PLAYER_FIELD_BYTES` byte 3, `+0x50`
a bit of field 1225, and `+0x54` and `+0x58` the faction and rank, against `0x80928c`'s minimum
standings −42000, −6000, −3000, 0, 3000, 9000, 21000, 42000. The proficiency mask is
`0x5e1e70(class)`, the server's table `0xc4d4a0` for a class below 16; a mask of 0 passes.
`0x5ea720(slot, guid)` refuses a non-empty bag for a slot outside 19 to 22 and 63 to 68, then asks
the item's `0x5da1d0(slot, ranged)` with `ChrClasses` `+0x40` as the flag.

## Cast time

Slot 45 (`0x5ee150`) is the unit's cast time (`auras`) with two differences: the active player's
time goes through the casting-time spell modifier (10) before the speed, and Attributes 0x2 adds
500 ms where the unit's time becomes 0x7fffffff. The speed multiplies only a positive time, and
is skipped when it is less than 2^−22 from 1.

Addresses: `0x5ee150`, `0x5ee1fb`, `0x5ee221`, `0x5ee24c`
Confidence: verified
Proof: one
Test: `objectlayer::player_cast_time_difftest`, `objectlayer::player_cast_time_special_values_difftest`

`0x5ee1fb` calls `ApplySpellModifierInt(row, &t, 10)`. `0x5ee221` compares `|m − 1|` with
`s_layoutEpsilon` (2^−22) with `test ah,5; jnp` and skips on less only; the product is stored f32
and converted with `fistp` under the default rounding. `0x5ee24c` adds 0x1f4. The level is slot 42's
value over 5, unsigned.

## The melee attack

`0x5ecb70(guid)` starts the active player's attack on a unit it can attack, unless
`UNIT_FIELD_FLAGS` has 0x20000, the player lacks control, is dead or is mounted; a target that fails
stops a running attack. It cancels the cast when its precast spell has Attributes 0x2, stands up, selects the target, sends
0x141 with the GUID unless already attacking, draws the weapons, ends the precast and auto-repeat,
and refreshes the animation. `0x5ecac0` stops it: raises the low-health (0xa) and low-mana (0xb) tutorials below one half, sends
0x142, and cancels the next-swing spells. On the active player's own attacker state (`0x6259b6`), `0x5ea800` clears the attack
error and sends the stop with the victim at or beyond four times the reach.

Addresses: `0x5ecb70`, `0x5ecac0`, `0x5ea800`, `0x624370`, `0x80b058`, `0x80a1e8`, `0x80306c`
Confidence: verified
Proof: one
Test: `objectlayer::reach_gate_5ea800_difftest`, `objectlayer::lowhp_warn_5ecac0_difftest`

The start needs `UNIT_FIELD_FLAGS` without 0x20000, `s_playerHasControl`, and a target with health
or dynamic flag 0x20. It sends when `+0xc54` is set or neither `+0xc48` nor `+0xc50` is, storing the
GUID in `+0xc48`; a cast with `+0x54` bit 0x8 returns before. The reach is `max(r_player + r_target
+ 4/3, 5) · 4` from `UNIT_FIELD_COMBATREACH`; `test ah,0x41; jp` stops on `r² ≤ d²` and keeps NaN. Slices: lowhp_warn_5ecac0_difftest `0x5ecac0`-`0x5ecb63`.

## Sheathing

`0x5eb480` toggles the sheathe state for the active player unless busy, mounted,
under `UNIT_FIELD_FLAGS` 0x40000, dead,
attacking, casting, in a blocking animation, or with a model lacking or playing the sheathe
animations. From 0 it draws melee weapons (1) or else the ranged one (2), and stops the melee
attack; from 1 it goes to 2 with a ranged weapon, else 0; from 2 to 0. A class whose `ChrClasses`
`+0x40` is set ignores the ranged weapon.

Addresses: `0x5eb480`, `0x5eb6d0`, `0x5eb6f8`, `0x611cf0`
Confidence: verified
Proof: one
Test: none

`0x5eb6d0` blocks animations 46, 49, 105 to 107 and 109 to 112 through the bytes `0x5eb6f8`. The
model must hold animations 0x5a and 0x59 and play neither on key bones 2 and 3. The states go
through `CGUnit_C::SetSheatheState(state, 0, 1)`.

## Spell combat log messages

| opcode | handler | body | calls |
|---|---|---|---|
| 0x24b | `0x5e7e00` | spell, caster, debug byte, count; per target GUID and reason | `0x494720`, `0x62bab0`, `CGUnit_C::ShowMissWord` |
| 0x24c | `0x5e7f90` | packed caster, spell, effect count; per effect an id and a target count | by effect, below |
| 0x24f | `0x5e84e0` | GUID, victim, damage, school | `0x62ca20`, `ShowDamageNumber`, `0x4946b0` |
| 0x32f | `0x5e85a0` | GUID, spell | `0x62cbe0` |
| 0x150 | `0x5e89c0` | packed target, caster, spell, amount, critical byte | `0x494740`, `0x62c770` |
| 0x151 | `0x5e8a90` | packed target, caster, spell, power, amount | `0x62ca00`, with the amount over the power's divisor |
| 0x27b, 0x333 | `0x5e8b60`, `0x5e8c00` | packed GUIDs, a count of spells | `0x62d480`, `0x62d630` |
| 0x263 | `0x5e8ca0` | two GUIDs, spell, byte | `0x62d240` |
| 0x27c | `0x5e8d30` | the damage calculation, to the console | |

A spell missing, in damage or in a miss, with Attributes bit 0x100 raises no `UNIT_COMBAT`.
0x24c reads per target by effect: a GUID for effects 1, 18, 38, 63, 69, 79, 91, 108, 113, 114,
116, 125 and 126 (1 skips the caster itself); GUID and three values for 8; GUID and a count for
19; an item for 24; a GUID for 33 and 59; GUID and spell for 68; a value for 101; a GUID for 102;
GUID and two values for 111. Any other effect reads nothing per target.

Addresses: `0x5e7e00`, `0x5e7f90`, `0x5e845c`, `0x5e8430`, `0x5e84e0`, `0x5e85a0`, `0x5e89c0`, `0x5e8a90`, `0x5e8b60`, `0x5e8c00`, `0x5e8ca0`, `0x5e8d30`, `0x494600`
Confidence: verified
Proof: one
Test: none

0x24c switches `effect − 1` (`cmp 0x7d`) through the bytes `0x5e845c` into the 11 cases `0x5e8430`;
the default calls `0x62ace0` with a zero target, and an effect with no targets calls it only when
it is the sole effect and of kind 3. The per-target calls are `0x62ace0`, `0x62dbf0`, `0x62d9f0`,
`0x62d8d0`, `0x62dc10`, `0x626a10`, `0x62d800`, `0x62e360`, and `0x62e190` or `0x62e050` for 111 by
whether both values are −1. `0x494600` signals `UNIT_COMBAT` per unit token; `0x494770`, the
energize feedback, returns at once. Counts are trusted: 0x27b and 0x333 test theirs only for 0.

## Non-melee damage

0x250 (`0x5e85e0`) reads packed target and caster, spell, damage, school, absorbed, resisted, a
periodic byte, a byte, blocked, hit flags and a debug byte. Hit flag 0x8 logs through `0x62de60`
and raises a plain `WOUND`. Otherwise it logs periodic or direct damage and raises `WOUND` with
critical (hit flag 0x2), absorb, block and resist marks; the target floats the damage, or for none
the absorb (10), block (5) or resist (2) word.

Addresses: `0x5e85e0`, `0x5e8700`, `0x62d9a0`, `0x62cd80`, `0x4946b0`, `0x6128b0`, `0x607140`
Confidence: verified
Proof: one
Test: none

`0x5e8700` tests hit flag 0x8. The marks are 0x80, 0x20, 0x800 and 0x40 into `0x4946b0(target,
damage, school, marks)`, which passes `WOUND` to `0x494600`. Periodic calls `0x62d9a0(target,
caster, spell, damage, school, absorbed, resisted)`, direct `0x62cd80` with blocked and the hit
flags too. The debug byte reads two floats for hit flag 0x1 and two for 0x4 and passes them to
`0x630200`, which returns at once.

## The unit combat event

One function raises `UNIT_COMBAT` (event 0xb6): `0x494600(unit, action, amount, type, hit flags)` signals
it once per unit token of the unit (`ui/unittokens`) with the token, the action, a descriptor chosen from the
amount and the flags, the amount and the type. With no token nothing is signalled. Four wrappers feed it, and only
these four: `0x4946b0` the action `WOUND`, `0x4946d0` the melee record's victim state, `0x494720` the spell miss
type with amount, type and flags 0, and `0x494740` `HEAL` with type 0 and flag 0x80 for a critical heal.

Addresses: `0x494600`, `0x494604`, `0x494617`, `0x49461d`, `0x494629`, `0x494634`, `0x49463f`, `0x49464b`, `0x494656`, `0x49465e`, `0x494681`, `0x494686`, `0x49468b`, `0x4946b0`, `0x4946d0`, `0x494706`, `0x494720`, `0x494729`, `0x494740`, `0x494746`, `0x49474a`, `0x84333c`, `0x83de28`, `0x86582c`
Confidence: verified
Proof: one
Test: none

With a positive amount the flags choose `CRITICAL` (0x80), `GLANCING` (0x4000) or `CRUSHING` (0x8000), else an
empty string; with an amount of 0 or less they choose `ABSORB` (0x20), `BLOCK` (0x800) or `RESIST` (0x40), else
empty. `0x49465e` hands the token lister the address of the flags slot as its count. The format is `%s%s%s%d%d`
(`0x84333c`). The melee wrapper reads the record's GUID at `+0x8`, damage `+0x14`, school `+0x18`, hit info `+0x10` and
victim state `+0xa4`, indexing nine actions from `0x83de28`: MISS, WOUND, DODGE, PARRY, INTERRUPT, BLOCK, EVADE, IMMUNE,
DEFLECT. The miss wrapper accepts types 0 to 11 from `0x86582c`: NONE, MISS, RESIST, DODGE, PARRY, BLOCK, EVADE, IMMUNE,
IMMUNE, DEFLECT, ABSORB, REFLECT, and returns for any other. The melee and damage callers pass the school as type; the miss and heal
wrappers pass 0. No `ENERGIZE` string exists; `0x494770` returns at once.

## Damage calculation log

0x27c prints "DMG: '%s' attacks '%s' sourceType=%d, spell='%s'" with the two units' names
("Non-unit" and "Unknown Unit" when absent) and the spell's first-locale name or "None", then per
entry a name of at most 30, an index and two floats: "val" prints "DMG: NOT VALIDATED: Old=%f,
New=%f", an index below 36 prints its name from `0x85fc54` as "DMG: (%s) '%s'=%8.2f, total=%f", any
other the number.

Addresses: `0x5e8d30`, `0x85fc54`
Confidence: verified
Proof: one
Test: none

The names come from `CGUnit_C::GetUnitName`; the spell name is `+0x1e0`, not the locale's. The
comparison is `SStrCmpI` with "val"; the index names are 16 bytes apart. A rule of '=' ends it.
