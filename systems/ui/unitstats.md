# ui: unit stat functions

The Lua functions that report a unit's stats, resistances, damage, speed, power and skills to the character and pet
panes, plus the creature family read that sits beside them: `UnitStat`, `UnitResistance`, `UnitArmor`, `UnitDamage`,
`UnitRangedDamage`, `UnitAttackSpeed`, `UnitAttackPower`, `UnitRangedAttackPower`, `UnitDefense`,
`UnitAttackBothHands`, `UnitRangedAttack` and `UnitCreatureFamily`. They resolve a unit token with
`spell/targeting#lua-targeting-functions`. Field indices count from the start of the descriptor block; the unit block
is read at `[unit+0x110]` (index 6 on) and the player block at `[unit+0xe68]` (index 188 on).

Roots: `0x5184a0`, `0x518600`, `0x518810`, `0x518910`, `0x518b90`, `0x518c20`, `0x518e50`, `0x518f80`, `0x5190c0`, `0x519200`, `0x5192e0`, `0x51a310`

## The pet token

A unit token `pet` names the local player's charmed unit when `UNIT_FIELD_CHARM` (fields 6 and 7) is nonzero, and
otherwise its summoned unit (`UNIT_FIELD_SUMMON`, fields 8 and 9). With both zero the token names no unit. A charmed
unit therefore takes the place of the summoned pet for every function that reads `pet`, including the pet pane.

Addresses: `0x515970`, `0x515a50`, `0x515a5e`, `0x515a64`, `0x515a6b`, `0x515a6e`, `0x515940`
Confidence: verified
Proof: one
Test: none

The token resolver tries the prefix `player` at `0x5159b4` and the prefix `pet` at `0x515a47`.
`0x515a5e` loads the local player's field view and `0x515a64` tests the two charm words; `0x515a6b` steps to the summon
words. The resolver `0x515940` turns the GUID into an object with `0x468460` and the unit mask 8, so every function
below receives an object of the unit family or null.

## Who may be asked

Five gates decide what a function reads for the unit it resolved. NULL: no object, which gives the function's zero
answer. UNIT: the object has the unit bit of its type word (bit 3). PLAYER: bit 4. SELF: the object's GUID is the local
player's (`0x468550`). SELF-OR-MINE: SELF, or the object's `UNIT_FIELD_SUMMONEDBY` (fields 12 and 13) is the local
player. A pet fails SELF and PLAYER and passes UNIT and SELF-OR-MINE. The functions use them as the table says.

| function | gates |
|---|---|
| `UnitStat` | NULL, UNIT; SELF for the two buff values |
| `UnitResistance`, `UnitArmor` | SELF picks the decomposition |
| `UnitDamage`, `UnitRangedDamage` | NULL; PLAYER, then SELF per value |
| `UnitAttackSpeed` | NULL; PLAYER for the off hand |
| `UnitAttackPower`, `UnitRangedAttackPower` | NULL, UNIT |
| `UnitDefense`, `UnitAttackBothHands` | NULL; SELF-OR-MINE |
| `UnitRangedAttack` | NULL; PLAYER, then SELF |

Addresses: `0x518600`, `0x51866b`, `0x518677`, `0x5186d5`, `0x5184a0`, `0x518535`, `0x518c20`, `0x518cd4`, `0x518ce9`, `0x518e50`, `0x518ecf`, `0x518f80`, `0x518fd3`, `0x519200`, `0x51925a`, `0x519272`, `0x519279`, `0x518810`, `0x518868`, `0x518876`, `0x468550`, `0x468460`
Confidence: verified
Proof: one
Test: none

The type tests are `shr ecx,3; test cl,1` (UNIT) and `shr ecx,4; test cl,1` (PLAYER) on the object's type word; a
unit-mask lookup by `0x515940` already guarantees UNIT, so the repeated tests never fail. The SELF compare is the pair
of `cmp` against `0x468550`'s two words. `0x519272` and `0x519279` load the SUMMONEDBY words at `[unit+0x110]+0x18` and `+0x1c`.

## Index and argument errors

`UnitStat(unit, i)` and `UnitResistance(unit, i)` truncate `i` toward zero (`__ftol`). `UnitStat` accepts 1 to 5 and
`UnitResistance` 0 to 6; any other index raises `Invalid stat index in UnitStat` or `Invalid resistance index in
UnitResistance`, so the call does not return. A first argument that is not a string or number, or a second that is not
a number or numeric string, raises `Usage: UnitStat("unit", statIndex)` or `Usage: UnitResistance("unit",
resistanceIndex)`. An unrecognised token raises `Unknown unit name: %s`. `UnitRangedAttackPower` shows the usage text of
`UnitAttackPower`.

Addresses: `0x518600`, `0x518610`, `0x518624`, `0x518652`, `0x51865a`, `0x518663`, `0x5187d6`, `0x5187ed`, `0x5184a0`, `0x5184fd`, `0x518506`, `0x5185cb`, `0x5185e2`, `0x5190d7`, `0x515c1a`, `0x40a2b0`
Confidence: verified
Proof: one
Test: none

The messages carry no index and no format directive; each is passed to `luaL_error` as its only argument
(`push message; push L; call 0x6f4940`). The stat index is tested after the decrement, so `0.5` and `-3` fail and `1.9`
selects stat 1. `0x5190d7` pushes `UnitAttackPower`'s string a second time for the ranged function. The unknown-token
error is raised inside the resolver at `0x515c1a`.

## Primary stats

`UnitStat(unit, i)` returns four numbers: the stat (`UNIT_FIELD_STAT0` plus `i-1`, fields 150 to 154, as a signed 32-bit
integer), the same value with a negative one raised to 0, and the positive and negative buffs of the player block
(`PLAYER_FIELD_POSSTAT0` and `NEGSTAT0`, fields 1177 and 1182, signed 32-bit). The buffs are read only for the local
player; any other unit gets 0 for both. An absent or non-unit object gives four zeros. The first value is the raw
field, not the second minus the buffs.

Addresses: `0x518600`, `0x518689`, `0x5186a3`, `0x5186ac`, `0x5186ba`, `0x518718`, `0x518722`, `0x518778`, `0x518782`, `0x51879e`, `0x5187b4`
Confidence: verified
Proof: one
Test: none

Each value is an integer loaded with `fild` and pushed with `lua_pushnumber` (`0x6f3810`): the stat at `0x518689`, the
clamped copy (`test eax,eax; setl cl; dec ecx; and ecx,eax`) at `0x5186ba`, and the buffs from a plain `mov` of
`[player+0xe68]+0xf74` and `+0xf88` (plus `i*4`) into a stack slot at `0x518718` and `0x518778`. No instruction of the
function loads a float.

## Resistances and armor

`UnitResistance(unit, i)` returns the base, the resistance, the positive buff and the negative buff, all integers.
For the local player the base is the field less the buffs, the resistance is the field (0 when it is negative), and the
buffs come from `PLAYER_FIELD_RESISTANCEBUFFMODSPOSITIVE` and `NEGATIVE` (fields 1187 and 1194). Any other unit gets
the field raised to 0 as both base and resistance, and 0 for both buffs. `UnitArmor(unit)` does the same for the armor
school and returns five values: the base, the effective armor, the armor, and the two buffs. A field below zero makes the
resistance and both armor copies 0 while the base keeps the signed difference.

Addresses: `0x5184a0`, `0x51852a`, `0x518544`, `0x51855b`, `0x518562`, `0x518568`, `0x51857f`, `0x5192e0`, `0x51934e`, `0x51937e`, `0x5efcd0`, `0x5efce0`, `0x5efcf5`, `0x5efd02`, `0x5efd11`, `0x5efd1a`, `0x5efd40`, `0x5efd7b`, `0x5efda0`, `0x5efddb`, `0x53f830`
Confidence: verified
Proof: one
Test: none

`0x5efcd0(unit, i, &base, &raw, &raw2, &pos, &neg)` reads `UNIT_FIELD_RESISTANCES` plus `i` (field 155 on, `[unit+0x110]+0x254`
as `mov`), copies it to the two raw slots, asks `0x5efd40` and `0x5efda0` for the buffs, subtracts both from the field
as integers (`0x5efd11`, `0x5efd13`) and zeroes the two raw slots when the field is negative (`0x5efd1a`). The two
helpers carry their own SELF test (`0x5efd50`, `0x5efdb0`) and return the buff word in `eax`, 0 when the unit is not the
player. The armor school index is the cell behind `0x53f830` (the Resistances row flagged physical; ordinarily row 0).

## Damage

`UnitDamage(unit)` returns seven values: the main-hand and off-hand minimum and maximum damage (`UNIT_FIELD_MINDAMAGE`
to `MAXOFFHANDDAMAGE`, fields 134 to 137, floats), then the physical bonus damage positive and negative (signed
integers, fields 1201 and 1208), then the damage percent (a float, field 1215). `UnitRangedDamage(unit)` returns the
ranged speed in seconds, the ranged minimum and maximum (fields 171 and 172) and the same three bonus values read for
the damage school of the ranged weapon. The bonus values are read only for the local player; for another player they
are 0, 0 and 0.0, and for a non-player they are 0, 0 and 1.0.

Addresses: `0x518c20`, `0x518c72`, `0x518cce`, `0x518d06`, `0x518d0f`, `0x518d47`, `0x518d50`, `0x518d7a`, `0x518d9e`, `0x518dbc`, `0x518dd2`, `0x518deb`, `0x518910`, `0x518977`, `0x5189a3`, `0x5189e1`, `0x5189ec`, `0x518a21`, `0x518a3b`, `0x518a77`, `0x518ac0`, `0x518af7`, `0x518b0e`, `0x518b1d`, `0x518b33`, `0x518b3a`, `0x5f0d60`, `0x7ffd74`
Confidence: verified
Proof: one
Test: none

An absent unit gives seven zeros (six for the ranged function). The positive and negative bonuses are `fild` of a
`mov`ed word and the percent is `fld dword` straight off the descriptor, so one function types two adjacent arrays as
integers and the third as a float. The ranged school: `0x5f0d60(unit, 0x11)` returns the ranged-slot item, its record
is fetched from the item cache and the first of five damage lines whose float at `+0xb8` is above 0 gives the school
at `+0xe0` (else school 0); the three values are read at `base + school*4`. A non-player answers the percent with the
literal 1.0 (`0x518dd2`, `0x518b33`) and a player that is not the local one with the zero constant.

## Attack speed

`UnitAttackSpeed(unit)` returns two values. The first is the main-hand speed in seconds (`UNIT_FIELD_BASEATTACKTIME`,
field 126, as an unsigned 64-bit read scaled by 0.001), or 0 for no unit. The second is the off-hand speed (field 127,
scaled the same) when the unit is a player with more than 16 items listed whose item in slot 16 is a weapon (item
class 2), and nil otherwise.

Addresses: `0x518e50`, `0x518ea0`, `0x518eb0`, `0x518ed7`, `0x518f09`, `0x518f14`, `0x518f19`, `0x518f24`, `0x518f34`, `0x518f4b`, `0x518f55`, `0x518f60`, `0x5d9f30`
Confidence: verified
Proof: one
Test: none

The scale constant is the float 0.001 at `0x801360`; `fild qword` reads the 32-bit field zero-extended. The slot test
is `cmp [esi+0x1d38],0x10; ja`, then the item GUID at `[[unit+0x1d3c]+0x80]` resolves through `0x468460` with the item
mask; `0x5d9f30` returns the item class from the item cache.

## Attack power

`UnitAttackPower(unit)` and `UnitRangedAttackPower(unit)` return three integers: the power, the positive mod and the
negative mod, each already multiplied by `1 + multiplier` and rounded on its own. The power is the signed field
(`UNIT_FIELD_ATTACK_POWER`, field 165), the mods are the low and high signed 16-bit halves of the next field (166),
and the multiplier is the float of field 167. The ranged forms use fields 168, 169 and 170. Each value is
`round_even(float((1 + m) * x))`, so the three do not sum to the rounded sum. Any unit of
the unit family gets the real answer; a missing unit or non-unit gets zeros.

Addresses: `0x518f80`, `0x518fe2`, `0x518fe8`, `0x518fee`, `0x518ff4`, `0x518ffa`, `0x519019`, `0x519051`, `0x519088`, `0x5190c0`, `0x7ff9d8`
Confidence: verified
Proof: one
Test: none

The sequence is `fld m; fadd 1.0; fimul x` at 53-bit precision, `fstp` to a single, then `fistp` (ties to even)
(`0x518fe2` to `0x518ffa`).

Each block is `fld [m]; fadd 1.0; fimul x; fstp dword; fld dword; fistp dword; fild dword` (`0x518fe2` to `0x518ffd`), so the
product is stored as a single float before the rounding to an integer by `fistp` under round-to-nearest. The mods are
loaded with `movsx` from `+0x280` and `+0x282` (`0x519019`, `0x519051`). The ranged function repeats this over the
three ranged fields.

## Defense and weapon skills

`UnitDefense(unit)` returns the defense skill and its modifier; `UnitAttackBothHands(unit)` returns base and modifier for
the main hand and then the off hand; `UnitRangedAttack(unit)` returns the ranged pair. The first two ask the unit through
a virtual call: for a creature or pet the answer is the unit's level times 5 with modifier 0, for the local player the
skill line read, the value plus the permanent bonus and the temporary bonus. `UnitDefense` and
`UnitAttackBothHands` raise a negative sum to zero by setting the modifier to the negative base. `UnitRangedAttack`
answers 0, 0 for a pet, for any unit that is not the local player, and for a ranged slot that is not a weapon.

Addresses: `0x519200`, `0x51928c`, `0x519298`, `0x518810`, `0x5188a1`, `0x5188a7`, `0x518b90`, `0x518be5`, `0x518bea`, `0x518bfa`, `0x613680`, `0x6136b0`, `0x5eda20`, `0x5eda80`, `0x612b40`, `0x5edae0`, `0x5ea460`
Confidence: verified
Proof: one
Test: none

The call is `[vtable+0xac]` (defense) or `[vtable+0xb0]` (attack, once per hand): `0x613680` and `0x6136b0` load the level
(`[unit+0x110]+0x70`), store `level * 5` through `lea ecx,[eax+eax*4]` and 0; the second ignores its hand argument. The
`0x5ea460` pair is `(value + permBonus, tempBonus)` where the bonuses come from the skill-info line of the player block
and only the temporary one is sign-extended. `UnitRangedAttack` has no virtual call and no clamp: `0x518be5` calls
`0x612b40` directly and `0x518bea` pushes the integer.

## Creature family

`UnitCreatureFamily(unit)` returns the localized family name of a unit, or nil when the family id is 0, negative, above
the table's highest id or has no row, or while the creature record has not arrived. The family id is not a descriptor
field: it is the word at `+0x1c` of the cached creature record (`[unit+0xb30]`). A non-string argument raises `Usage:
UnitCreatureFamily("unit")`. The food types of the pet's family are `ui/petbar#other-pet-functions`.

Addresses: `0x51a310`, `0x51a34c`, `0x51a357`, `0x51a35f`, `0x51a372`, `0x6055e0`, `0xc0de7c`, `0xc0de80`
Confidence: verified
Proof: one
Test: none

Every path returns one value; the failure legs push nil (`0x51a384`). The family row's name is at
`+0x20 + locale*4`, read through the locale index cell. `0x51a353 je` and `0x51a355 jl` reject 0 and negative ids and
`0x51a357 cmp eax,[0xc0de80]; jg` the ids above the highest.
