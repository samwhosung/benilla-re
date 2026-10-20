# objectlayer: combat log messages

The combat log's lines, family by family: which message or event writes each, what it reads, the
global strings it chooses from and the `COMBAT_TEXT_UPDATE` type it raises. The machinery they share
(the relation classes, the range tests, the chat types, names, trailers and item waits) is
`combatlog`. Most formatters are called from other files: the melee swing from `unitcombat`, the
spell messages from `playercombat` and `spell`, deaths and XP from `units`.

UnitCombatLog_C.cpp registers six messages and UnitCombatLogXP_C.cpp one. Every spell formatter
first drops a spell that is missing, has Attributes 0x100 or 0x80, or has an empty name in the
client's locale.

Roots: `0x626dd0`, `0x628890`, `0x6289a0`, `0x628c20`, `0x628e60`, `0x628ea0`, `0x637e30`, `0x630170`, `0x629b10`, `0x62aac0`, `0x62ace0`, `0x62b1b0`, `0x62b480`, `0x62b800`, `0x62bab0`, `0x62c160`, `0x62c360`, `0x62c5f0`, `0x62c670`, `0x62c770`, `0x62ca00`, `0x62ca20`, `0x62cbe0`, `0x62cd80`, `0x62d240`, `0x62d440`, `0x62d480`, `0x62d630`, `0x62d800`, `0x62d8d0`, `0x62d9a0`, `0x62d9f0`, `0x62dbf0`, `0x62dc10`, `0x62de60`, `0x62e050`, `0x62e190`, `0x62e360`, `0x637a80`

## Log messages

| opcode | handler | body | writes |
|---|---|---|---|
| 0x24e | `0x626dd0` | packed target and caster, spell, count; per aura its type and values | below |
| 0x1d7 | `0x628ea0` | caster and target GUIDs, item entry, spell, byte | the enchantment line |
| 0x1f5 | `0x628890` | killer and victim GUIDs | `SELFKILLOTHER` or `PARTYKILLOTHER` |
| 0x260 | `0x6289a0` | caster and target, spell, a byte, then two floats when it is set | `PROCRESIST…` |
| 0x262 | `0x628c20` | caster and target, then spells to the end | `DISPELFAILED…` per spell |
| 0x2bd | `0x628e60` | none | `DURABILITYDAMAGE_DEATH` under 0x19 |
| 0x1d0 | `0x637e30` | victim GUID, XP, kind byte; kind 0 adds an i32 and a float | the XP lines |
| 0x1d6 | `0x630170` | read and dropped | nothing |

Addresses: `0x626d00`, `0x629a90`, `0x628890`, `0x628901`, `0x6289a0`, `0x628a4e`, `0x628c20`, `0x628cd5`, `0x628e60`, `0x628ea0`, `0x628f40`, `0x637e10`, `0x637fb0`, `0x630150`, `0x6301b0`
Confidence: verified
Proof: one
Test: none

`0x626d00` registers the first six and calls `0x630150` and `0x637e10`; `0x629a90` clears them all.
`0x628901`: killer class 0 writes `SELFKILLOTHER` with the victim, class 2 `PARTYKILLOTHER` with
victim and killer, under `0x628980`'s type, 0x2b for a victim class 0 to 5, else 0x2c. `0x628a4e`
passes the floats to the empty `0x630210`. `0x628cd5` loops while the store's vtable `+0x14` is 0.
Both raise their combat text, `PROC_RESISTED` and `DISPEL_FAILED`, when the second unit is the
player.
`0x628f40(caster, target, item, spell, byte)` waits for the item and writes
`ITEMENCHANTMENTADD…` with a caster (`0x6291d0`), else `ITEMENCHANTMENTREMOVE…` (`0x628ff0`),
under 0x44. `0x628bc0` and `0x628e00` pick the proc-resist and dispel-failed keys. Every handler
returns 1.

## Periodic auras

0x24e is ignored while `CombatLogPeriodicSpells` is off, and for a missing spell or target. Per
aura, types 3 and 89 carry damage, school, absorbed and resisted; 8 and 20 a heal; 21 and 24 a
power and an amount; 64 a power, an amount and a float. Any other type ends the message there.
Damage floats the number, or the absorb or resist word, and signals the target's wound unless
Attributes has 0x100; heals and energizes signal under the same rule, and drains signal nothing.

Addresses: `0x626dd0`, `0x626dee`, `0x626e41`, `0x626e73`, `0x626ec9`, `0x6271d0`, `0x6271e4`, `0x626f1b`, `0x626ffe`, `0x627068`, `0x6270f8`, `0x62719e`
Confidence: verified
Proof: one
Test: none

`0x626dee` reads the CVar's integer; a skip moves the cursor to the end. `0x626ec9`: the type minus
3, bounded by `cmp 0x56`, indexes the bytes `0x6271e4` into the five cases `0x6271d0`. `0x626f1b`:
damage calls `ShowDamageNumber`, or `ShowMissWord` 10 or 2, then `SignalUnitCombatWound` with 0x20
and 0x40 marks, then `0x628100`. `0x626ffe`: `SignalUnitCombatHeal` and `0x627240`. `0x627068`:
the amount over `GetPowerDisplayDivisor(power)`, `SignalUnitCombatEnergize` and `0x627500`.
`0x6270f8`: the same division and `0x627910` with the float. `0x62719e` moves the cursor to the end.

## Periodic lines

`0x628100` writes `PERIODICAURADAMAGE…` under the target's periodic damage type with the school name
and trailers, `0x627240` `PERIODICAURAHEAL…` under its periodic buff type; `0x627520` and `0x62ca00`
write `POWERGAIN…`, and `0x627930` `SPELLPOWERDRAIN…` or, for a leech, `SPELLPOWERLEECH…`. A
periodic heal on the player raises `PERIODIC_HEAL` only for a spell lasting at most 30 seconds;
periodic damage raises `RESIST`, `ABSORB` or `DAMAGE`; a direct power gain, a drain or a leech on
the player raises `MANA`, `RAGE`, `FOCUS` or `ENERGY` by the power's name.

Addresses: `0x628100`, `0x6283b0`, `0x628346`, `0x627240`, `0x627440`, `0x6273b2`, `0x627406`, `0x627520`, `0x627500`, `0x6277c0`, `0x6278f0`, `0x6276dc`, `0x627930`, `0x627910`, `0x627cd0`, `0x627c80`, `0x62dbf0`, `0x62ca00`
Confidence: verified
Proof: one
Test: `objectlayer::f3_powerleech_display_amount_difftest`

`0x628346`, for a target of class 0: a resisted amount above 0 raises `RESIST`, else an absorbed one
`ABSORB`, else `DAMAGE` with the amount. `0x6273b2`: the `SpellDuration` row of `+0x78` gives `min(base + perLevel·level, max)` by
`GetSpellLevel`, and `0x627406` skips above 30,000. `0x6278f0(power)` reads a name table for power
below 5. `0x6276dc` compares the name with `MANA_POINTS`, `RAGE_POINTS`, `FOCUS_POINTS` and
`ENERGY_POINTS`. `0x627500` and `0x62ca00` call `0x627520` with 1 and 0; `0x627910` and `0x62dbf0`
call `0x627930` with 1 and 0, the flag choosing the periodic chat types; `0x627520` raises its
combat text only with the flag clear. `0x627930` takes the leech keys of `0x627c80` when the
multiplier's absolute value (`fabs`, `0x6279fa`) is at least the epsilon `0x8029d4` or is NaN
(`test ah,5; jp`), else the drain keys of `0x627cd0`; `0x627c80` has no `SELFSELF` key,
so a self-leech writes nothing. Slices: f3_powerleech_display_amount_difftest `0x627a8f`-`0x627ace`.

## Melee lines

A swing's line is written on receipt, unless the swing record's `+0xac` is set in a state other than
1. A miss (hit info 0x10) writes `MISSED…`; a block `VSBLOCK…`; no damage with hit info 0x20
`VSABSORB…` or with 0x40 `VSRESIST…`; damage in state 1 `COMBATHIT…`, `COMBATHITCRIT…` for 0x2, and their `SCHOOL`
forms for a school other than physical; states 2, 3, 6, 7 and 8 `VSDODGE…`, `VSPARRY…`, `VSEVADE…`,
`VSIMMUNE…` and `VSDEFLECT…`. Each raises its combat text, the hit `DAMAGE` or `DAMAGE_CRIT`.

Addresses: `0x629b10`, `0x629b27`, `0x629b60`, `0x629bb2`, `0x629bf0`, `0x629c32`, `0x629ca6`, `0x629d30`, `0x62a1a0`, `0x62a3b0`, `0x62a4d0`, `0x62a5f0`, `0x62a710`, `0x8628f8`
Confidence: verified
Proof: one
Test: none

`0x629b27`: a hit by the active player in state 1 sets flag 5 through 0x4b5390. `0x629b60` takes
both ends through the two-sided test; the hits use `0x62a0d0`'s types and the rest `0x62a2e0`'s.
`0x629d30` reads the first part's school (`0x629d7d`) and passes the trailers with the blocked
amount, the first part's absorb and resist and the hit info. `0x62a710` first needs `0x8628f8`
nonzero for the state (2, 3, 5 to 8), then switches `state − 2` through `0x62a8ec` and `0x62a908`.
The keys come from `0x629f90`, `0x629fe0`, `0x62a030`, `0x62a080`, `0x62a290`, `0x62a480`,
`0x62a5a0`, `0x62a6c0`, `0x62a930`, `0x62a980`, `0x62a9d0`, `0x62aa20` and `0x62aa70`, none with a
`SELFSELF` key.

## Spell damage and misses

`0x62cd80` writes a direct spell hit: `SPELLLOG…`, `SPELLLOGCRIT…` for hit flag 0x2, and the
`SCHOOL` forms for a school other than the spell's, with trailers, under the damage type; with no
damage it writes the absorb, the block miss or the resist instead. On the player it raises
`SPELL_RESISTED`, `SPELL_BLOCKED`, `SPELL_ABSORBED` or `SPELL_DAMAGE`, in that order of choice. `0x62bab0` writes a spell miss
by its reason: `SPELLMISS…`, `SPELLRESIST…`, `…DODGED…`, `…PARRIED…`, `…BLOCKED…`, `…EVADED…`,
`…IMMUNE…`, `…DEFLECTED…` or `SPELLREFLECT…`.

Addresses: `0x62cd80`, `0x62ce43`, `0x62ce65`, `0x62ceb7`, `0x62cf07`, `0x62bab0`, `0x62bb53`, `0x62bde8`, `0x62be10`, `0x6285b0`, `0x628720`, `0x62be40`, `0x62c0e0`
Confidence: verified
Proof: one
Test: none

`0x62ce43` flashes the target's nameplate for any damage. `0x62ce65` and `0x62cf0e` test the hit
flag 0x2; the school is compared with the one whose `Resistances` row has flag 0x1. With no damage, absorbed calls `0x6285b0` (`SPELL_ABSORBED`), blocked
`0x62bab0` with reason 5 (`0x62ceb7`), resisted `0x628720` (`SPELL_RESISTED`). `0x62bb53` takes the
reason minus 2, bounded by `cmp 9`, through `0x62bde8` to the keys and through `0x62be10` to the
combat text: 2 resist, 3 dodge, 4 parry, 5 block, 6 evade, 7 and 8 immune, 9 deflect, 11 reflect;
any other reason writes `SPELLMISS…`. The chat type is `0x627d30`'s, or with the last argument set
`0x62c140`'s, the damage shield's. `0x62d0c0`, `0x62d120`, `0x62d180` and
`0x62d1e0` pick the `SPELLLOG` keys, `0x6286c0` the `SPELLLOGABSORB` ones and `0x628830` the
`SPELLRESIST` ones; `0x62bea0`, `0x62bf00`, `0x62bf60`, `0x62bfc0`, `0x62c020` and `0x62c080` the
other misses'.

## Shields, splits and immunities

`0x62ca20` writes a damage shield's hit, `DAMAGESHIELD…`, under `SPELL_DAMAGESHIELDS_ON_SELF` when
the shield's owner, the first unit, is the player or its pet, or `…ON_OTHERS`; `0x62de60` a split, `SPELLSPLITDAMAGE…`;
`0x62d240` an immunity, `IMMUNESPELL…`, while `CombatLogPeriodicSpells` is on; `0x62cbe0` an
instant kill, `INSTAKILL…`; `0x626a10` an interrupt, `SPELLINTERRUPT…`; `0x62e050` and `0x62e190`
durability losses on one item or all, `SPELLDURABILITYDAMAGE…` and `…ALL…`.

Addresses: `0x62ca20`, `0x62cb90`, `0x62c140`, `0x62de60`, `0x62e000`, `0x62d240`, `0x62d3e0`, `0x62cbe0`, `0x626a10`, `0x626b90`, `0x62e050`, `0x62e140`, `0x62e190`, `0x62e310`, `0x62d9a0`
Confidence: verified
Proof: one
Test: `objectlayer::diff_combat_gate_twins`, `objectlayer::diff_combat_gate_dist2`

`0x62c140(class)` gives 0x3f for 0 and 1 (the player or its pet), else 0x40. `0x62cb90`,
`0x62e000`, `0x62e140`, `0x62e310` and `0x626b90` have no `SELFSELF` key. `0x62e050` waits for the
item through `0x6298a0`. `0x62d9a0` passes a periodic hit to `0x628100` while the CVar is on.
Combat texts: `DAMAGE_SHIELD`, `SPLIT_DAMAGE`, `SPELL_IMMUNE`, `INTERRUPT`. Slices: diff_combat_gate_twins `0x62ccb7`-`0x62cce3`; diff_combat_gate_dist2 `0x62ccb7`-`0x62ccda`.

## Heals

`0x62c770` writes a heal, `HEALED…` or `HEALEDCRIT…` for its critical byte, under the buff type;
on the player it raises `HEAL` or `HEAL_CRIT` with the healer's name and the amount.

Addresses: `0x62c770`, `0x62c940`, `0x62c9a0`, `0x62c8ea`
Confidence: verified
Proof: one
Test: none

`0x62c8ea` tests the byte; the event is `0x626890` with 0x28 or 0x29.

## Casts

`0x62ace0` writes a spell's success: for a spell without effect 36, 33 or 59 and without `+0x28`
bit 0x1, `SIMPLECAST…` at a target or `SPELLTERSE_…` without one, their `PERFORM` forms for a spell
with Attributes 0x10. `0x62b1b0` writes the start of another unit's cast with a cast time,
`SPELLCASTOTHERSTART` or its `PERFORM` form, for a spell without effect 36 or `+0x28` bit 0x1. Its
`SELF` keys and its `SPELL_CAST_START` text sit behind class 0, which it has already dropped.
`0x62d9f0` writes extra attacks, `SPELLEXTRAATTACKS…` with `_SINGULAR` for one; `0x62dc10` an
opened lock, `OPEN_LOCK_…`; `0x62e360` a dismissed pet.

Addresses: `0x62ace0`, `0x62ad70`, `0x62ad90`, `0x62aff0`, `0x62adc0`, `0x62af30`, `0x62af90`, `0x62b000`, `0x62b1b0`, `0x62b2a6`, `0x62b460`, `0x62b32a`, `0x62d9f0`, `0x62dc10`, `0x62e360`
Confidence: verified
Proof: one
Test: `objectlayer::range_gate_clone_twins`, `objectlayer::diff_range_gate_dist2`, `objectlayer::diff_combat_gate_twins`, `objectlayer::diff_combat_gate_dist2`

`0x62ad70` finds effect 36 and `0x62ad90` 33 or 59 among `+0xf4`; `0x62aff0` returns bit 4 of
Attributes. `0x62b2a6` drops the active player's casts; `0x62b460` drops a cast time of 0 from the
unit's `GetSpellCastTime`; `0x62b32a`: a caster of class 0 to 3 needs a spell with an aura effect
(`SpellHasAuraEffect`). Combat texts: `SPELL_CAST` for a cast at the player or its own untargeted cast, `EXTRA_ATTACKS`
for the player's. `0x62dc10` names a game object from `s_gameObjectCache` or an item from the item
cache. `0x62e360` writes `SPELLDISMISSPET…` under 0x19. Slices: range_gate_clone_twins `0x62b090`-`0x62b0bc`; diff_range_gate_dist2 `0x62b090`-`0x62b0b3`; diff_combat_gate_twins `0x62dada`-`0x62db06`; diff_combat_gate_dist2 `0x62dada`-`0x62dafd`.

## Auras

`0x62b480` writes an aura's arrival, `AURAADDED…HARMFUL` or `…HELPFUL`, and its loss,
`AURAREMOVED…`, raising `AURA_START`, `AURA_START_HARMFUL`, `AURA_END` or `AURA_END_HARMFUL` on
the player, except for a passive spell (Attributes 0x40); `0x62b800` a stack of a spell that stacks,
`AURAAPPLICATIONADDED…`; `0x62d480` a dispel, `AURADISPEL…`, and
`0x62d630` a theft, `AURASTOLEN…`, both raising `SPELL_DISPELLED` under 0x45. The aura chat types
are the periodic ones; a loss goes to `SPELL_AURA_GONE_SELF`, `_PARTY` or `_OTHER`.

Addresses: `0x62b480`, `0x62b7d0`, `0x62b7f0`, `0x62b800`, `0x62b96d`, `0x62d480`, `0x62d630`, `0x62d7a0`
Confidence: verified
Proof: one
Test: `objectlayer::f3_combat_log_in_range_difftest`, `objectlayer::diff_combat_gate_twins`, `objectlayer::diff_combat_gate_dist2`

`0x62b7d0(class)` gives 0x41 for 0 and 1 and 0x42 for 2 and 3 through `0x62b7f0`, else 0x43.
`0x62b480` takes harmful or helpful from its caller's byte; `0x62b800` needs the spell's `+0x9c`
above 1 and calls aura slots 32 to 47 harmful (`0x62b96d`). Slices: f3_combat_log_in_range_difftest `0x62b5ad`-`0x62b5d9`; diff_combat_gate_twins `0x62d55b`-`0x62d587`; diff_combat_gate_dist2 `0x62d55b`-`0x62d57e`.

## Items, pets and reputation

`0x629690` writes a trade skill's product, `TRADESKILL_LOG_FIRSTPERSON` or `…THIRDPERSON`, and
`0x629760` a pet's meal, `FEEDPET_LOG_…`, both under 0x3e once the item is known. `0x627de0` writes
a pet's happiness drain, `SPELLHAPPINESSDRAIN…`, under 0x19; `0x62d440` a loyalty change,
`PET_LOYALTY_GAIN` or `_LOSS`. `0x62c5f0` writes `FACTION_STANDING_INCREASED` or `…DECREASED` under
0x55 and raises `FACTION`; `0x62c670` writes `FACTION_STANDING_CHANGED` with the standing's label.

Addresses: `0x629690`, `0x629760`, `0x62d800`, `0x62d8d0`, `0x627de0`, `0x62d440`, `0x62c5f0`, `0x62c670`, `0x62c733`
Confidence: verified
Proof: one
Test: `objectlayer::diff_combat_defer_visible`

`0x62d800` and `0x62d8d0` run the one-sided test and call `0x629760` and `0x629610`, which wait for
the item. `0x627de0` divides the drain by `GetPowerDisplayDivisor(4)`. `0x62c670` reads the
faction's row in the client's locale and the standing's `FACTION_STANDING_LABEL%d` through
`CGUnit_C::GetGenderedText`, and adds the line under type 10 through 0x49a870 (`0x62c733`). Slices: diff_combat_defer_visible `0x62d957`-`0x62d983` `0x62d887`-`0x62d8b3`.

## Deaths and the environment

`0x62c160` writes a unit's death: `UNITDIESSELF` for the player, `UNITDESTROYEDOTHER` for a unit a
summoning or totem spell created, else `UNITDIESOTHER`, under 0x2b or 0x2c. A unit marked when its
kill's XP line was written writes none; its XP line names it instead. `0x62aac0` writes environmental damage,
`VSENVIRONMENTALDAMAGE_<type>_SELF` or `_OTHER`, flashes the victim's nameplate, and raises
`DAMAGE`, `ABSORB` or `RESIST`.

Addresses: `0x62c160`, `0x62c232`, `0x62c2d0`, `0x62c318`, `0x62c320`, `0x628980`, `0x62aac0`, `0x62abac`, `0x62abcb`
Confidence: verified
Proof: one
Test: none

`0x62c232` tests bit 0x8 of `+0xc58`. `0x62c2d0` reads `UNIT_CREATED_BY_SPELL`'s first effect: 50,
74, 87 to 90 and 104 to 107 map through the bytes `0x62c320` to the true case of `0x62c318`.
`0x62abcb` formats the key with `VSENVIRONMENTALDAMAGE_%s_%s` from the type's name and `SELF` for
class 0. Both use the one-sided test.

## Experience

0x1d0 shows XP the player earns, written and floated over the player when the message arrives.
For a visible victim and kind 0 the gain goes through the victim, which marks it so that its death
writes no line, and a gain of 0 writes nothing. That line names the victim and adds a group bonus
or raid penalty: `COMBATLOG_XPGAIN_FIRSTPERSON`, `_GROUP`, `_RAID`, or with a rest modifier
`COMBATLOG_XPGAIN_EXHAUSTIONn` and its forms. Otherwise the line is
`COMBATLOG_XPGAIN_FIRSTPERSON_UNNAMED`. The unnamed group and raid forms, which would print the group
string's address where the XP belongs, are unreachable: they need no unit and a multiplier other
than 1, and the one caller without a unit passes 1.0.

Addresses: `0x637e30`, `0x637ec0`, `0x637ac0`, `0x637cd0`, `0x637da0`, `0x637f70`, `0x637f90`, `0x637a80`, `0x864330`, `0x60a0c0`, `0x60a0f7`, `0x6251e0`, `0x625202`, `0x625259`, `0x6251a4`
Confidence: verified
Proof: one
Test: `objectlayer::xpgain_rest_gate_difftest`

`0x637ec0`: the player's own GUID writes `COMBATLOG_XPLOSS_FIRSTPERSON_UNNAMED` with the negated XP
(`0x637f70`); a victim with kind 0 calls `CGUnit_C::SetPendingXP`, which stores the gain at `+0xc80`,
sets bit 0x8 of `+0xc58` and calls `0x6251e0` at `0x60a0f7`. Its one gate is `+0xc80` nonzero
(`0x625202`); it writes through `0x637a80`, floats and zeroes `+0xc80` (`0x625259`), so the death's
call at `0x6251a4` finds nothing. Else `0x637f90` writes with no unit and multiplier 1.0, and
`ShowXPGain` floats. `0x637ac0`: the `Exhaustion` row of the player's rest byte (`+0x1b` of its
fields) with a multiplier other than 0 and 1 selects `0x864330`'s key; the base `ftol(xp/mult)`
equal, below or above the received picks the plain, `_GROUP` or `_RAID` form. `0x637ac0`'s callers
are `0x637ab1`, in `0x637a80` with the unit `ClntObjMgrObjectPtr` found at `0x637aa8`, and
`0x637fa3`, in `0x637f90` with no unit and 1.0 (`0x637f98`); `0x637cd0` with a multiplier of 1.0
computes `ftol(xp / 1.0)`, the XP itself, so the unnamed line is the plain one. `0x637da0`: after
`ebx` holds the unnamed format, `GetText(…UNNAMED_GROUP)` is pushed as the first argument of
`You gain %d experience. (+%d group bonus)`, ahead of the XP and the difference. Slices: xpgain_rest_gate_difftest `0x637b3e`-`0x637b7b`.
