# objectlayer: combat log

How the client writes its combat log. The server sends numbers and GUIDs; the client builds each
sentence from a global string chosen by who did what to whom, checks that the event is near enough
to the player to report, picks the chat type the line goes out under, and raises
`COMBAT_TEXT_UPDATE` for what happens to the player. This topic is that machinery; the messages and
their lines are `combatlogmessages`.

UnitCombatLog_C.cpp runs from `0x6263a0` to `0x62e360` and UnitCombatLogXP_C.cpp from `0x637a80`
to `0x637fb0`, initializers excepted (`initializers`). Between them, `0x62e580` grows the item-cache
waits of the log. `0x630150` and `0x6301b0` register and clear opcode 0x1d6, and `0x6301c0` to
`0x630230` are seven empty hooks the formatters call.

Roots: `0x626d00`, `0x629a90`, `0x626850`, `0x626630`, `0x6264e0`, `0x626890`, `0x6268f0`, `0x626960`, `0x6269b0`, `0x6269d0`, `0x6269f0`, `0x630150`, `0x6301b0`

## A line

`0x626850(type, format, …)` formats a line of at most 2,047 characters and adds it to the chat
frames under a chat type, and to a file while flag 0xb4fdb8 is set; type 0x5e sends nothing.
Every formatter takes its sentence from a global string. In UnitCombatLog_C.cpp one that is missing
or empty writes `Warning, string %s not found in stringfile.` under type 0x19 instead, but for
two lines: the item durability line writes nothing, and the faction line warns only for a missing
string. In UnitCombatLogXP_C.cpp only the rested XP line warns; the others pass the string on
unchecked.

Addresses: `0x626850`, `0x62685d`, `0x626876`, `0x626886`, `0x6269f0`, `0x804710`
Confidence: verified
Proof: one
Test: none

`0x62685d` returns for 0x5e. `0x626876` calls 0x64a8b0 into a 0x800-byte buffer with the variable
arguments; `0x626886` passes the text and the type to 0x49b1b0, which adds it with six zero
arguments through 0x49b0b0 and, when 0xb4fdb8 is set, writes it with `%s` to the file 0xb50544.
`0x6269f0(name)` writes the warning for a nonempty name. `0x637cd0`, `0x637da0` and `0x637f70` hand
`FrameScript_GetText`'s result straight to `0x626850`; `0x637ac0` warns at `0x637bf9`. `0x6298a0` returns on either (`0x62995f`, `0x629968`);
`0x62c5f0` tests only for null (`0x62c62b`) and passes an empty string to `0x626850`. The chat types are rows of `0x804710`,
94 records of a 64-byte name and an RGB colour: 0x19 `COMBAT_MISC_INFO`, 0x1b to 0x2a the
`COMBAT_*_HITS` and `_MISSES`, 0x2b and 0x2c the friendly and hostile deaths, 0x2d `COMBAT_XP_GAIN`,
0x2e to 0x50 the `SPELL_*` families, 0x51 `COMBAT_HONOR_GAIN`, 0x55 `COMBAT_FACTION_CHANGE`.

## Who: the relation classes

Each line classifies its source and its target relative to the player with
`ClassifyGuidForPlayer`: 0 the player, 1 its pet, 2 a party member, 3 a party member's pet, 4 a
friendly player, 5 a friendly player's pet, 6 a hostile player, 7 a hostile player's pet, 8 a
creature, 9 an object the client cannot resolve. The two classes choose the global string and the
chat type; the event's school, amount and crits only fill the string.

Addresses: `0x5efea0`, `0x626630`, `0x626647`, `0x626651`
Confidence: verified
Proof: one
Test: none

`0x626647` and `0x626651` store both classes for the caller. The classes are the function's own
(`interaction`, Other units). A string key is the family's name followed by `SELF` or `OTHER` for
the source and again for the target, class 0 being `SELF`: `SPELLLOGSELFOTHER`, for one.

## Range

A line is written only for an event near the player: the player within the range of the source's
class of the source, or within the range of the target's class of the target. Classes 0 and 1 reach
100,000 yards and class 9 none; the others read the CVars `CombatLogRangeParty`, `…PartyPet`,
`…FriendlyPlayers`, `…FriendlyPlayersPets`, `…HostilePlayers` and `…HostilePlayersPets`, 50 yards by
default, and `CombatLogRangeCreature`, 30. Both comparisons are strict, and NaN fails.

Addresses: `0x626630`, `0x626698`, `0x626705`, `0x626716`, `0x62674d`, `0x6267c7`, `0x6267e5`, `0x626810`, `0x8629e0`, `0x626d00`
Confidence: verified
Proof: one
Test: `objectlayer::diff_two_endpoint_b`, `objectlayer::diff_two_endpoint_dist2`

`0x626630` returns false when the target or the active player does not resolve (`0x626698`,
`0x626705`); it also returns the two objects and the units among them. `0x62674d`: a missing source
measures from the target. `0x6267c7 test ah,5; jnp` passes on `d² < r²` against the source's range,
`0x6267e5` the same against the target's. `0x626810(class)` reads `0x8629e0`: eight-byte rows of a
CVar name and its default; a null name gives 100000.0, an empty one or a class outside 0 to 9 gives
0.0, a CVar its float `+0x24`. `0x626d00` registers the seven, then `CombatDeathLogRange` (60). Slices: diff_two_endpoint_b `0x62679c`-`0x6267e3`; diff_two_endpoint_dist2 `0x62679c`-`0x6267df`.

## The one-sided range test

The formatters about one unit, deaths, casts, auras and dispels among them, test only that unit:
the player must lie strictly within its class's range. Each looks up `CombatDeathLogRange` first.
Only the death line uses it, as the range when the CVar exists; `0x628010` would with its flag set,
and both of its callers pass 0. The other thirteen lookups are dropped.

Addresses: `0x62b000`, `0x62b03f`, `0x62b0b0`, `0x62b0be`, `0x62c160`, `0x62c1a6`, `0x628010`, `0x628055`, `0x627de0`, `0x628ff0`, `0x62aac0`, `0x62b1b0`, `0x62b480`, `0x62b800`, `0x62c360`, `0x62cbe0`, `0x62d480`, `0x62d800`, `0x62d8d0`, `0x62d9f0`
Confidence: verified
Proof: one
Test: `objectlayer::diff_range_gate_visible`, `objectlayer::range_gate_clone_twins`, `objectlayer::diff_range_gate_dist2`, `objectlayer::diff_combat_gate_copied_twin`, `objectlayer::diff_combat_gate_dist2`

In `0x62b000`, `0x62b03f` calls `CVar::Lookup` and the next instruction overwrites `ecx`; `eax` is
not read. `0x62b0b0` loads `range·range` and `fcompp` against `d²`; `0x62b0be test ah,0x41; jne`
skips on `r² ≤ d²` and on NaN. The census is every dword equal to the name 0x862e00: 16, the
registration and fifteen lookups. `0x62c1a6` keeps the CVar's float `+0x24`; `0x628055` does only
for a nonzero byte argument, and its callers `0x627de0` and `0x62dc10` push 0. `0x627de0`,
`0x628ff0`, `0x62aac0`, `0x62b1b0`, `0x62b480`, `0x62b800`, `0x62c360`, `0x62cbe0`, `0x62d480`,
`0x62d800`, `0x62d8d0` and `0x62d9f0` follow `0x62b000`'s pattern. Slices: diff_range_gate_visible `0x627f0c`-`0x627f38`; range_gate_clone_twins `0x627f0c`-`0x627f38` `0x629083`-`0x6290af` `0x62b090`-`0x62b0bc`; diff_range_gate_dist2 `0x627f0c`-`0x627f2f` `0x629083`-`0x6290a6` `0x62b090`-`0x62b0b3`; diff_combat_gate_copied_twin `0x62c434`-`0x62c47a`; diff_combat_gate_dist2 `0x62c434`-`0x62c473`.

## The chat type

The chat type follows the source's class, and for some the target's, by family. Damage goes to
`SPELL_SELF_DAMAGE`, `_PET_`, `_PARTY_`, `_FRIENDLYPLAYER_` or `_HOSTILEPLAYER_DAMAGE`, or for a
creature or an unresolved source `SPELL_CREATURE_VS_SELF_`, `_VS_PARTY_` or `_VS_CREATURE_DAMAGE` by
the target; a party member's or friendly player's spell at the player, its pet or the party counts
as hostile. Buffs use the `_BUFF` twins; periodic lines go by the target alone; melee lines use the
`COMBAT_*_HITS` and `_MISSES` types the same way.

Addresses: `0x626be0`, `0x626c64`, `0x626c8c`, `0x626c98`, `0x627820`, `0x6278a4`, `0x6278cc`, `0x6278d8`, `0x6274a0`, `0x6274d0`, `0x627d80`, `0x627db0`, `0x62a0d0`, `0x62a154`, `0x62a17c`, `0x62a188`, `0x62a2e0`, `0x62a364`, `0x62a38c`, `0x62a398`, `0x627d30`, `0x627d60`, `0x62b190`
Confidence: verified
Proof: one
Test: none

`0x626be0(source, target)` switches the source through `0x626c64`: 0 gives 0x2e, 1 0x30; 2 and 4
give 0x36 for a target of 0 to 3, else 0x32 and 0x34; 3 gives 0x32, 5 0x34, 6 and 7 0x36; 8 and 9
map the target through the bytes `0x626c98` (0, 0, 1, 1, 2, …) into 0x38, 0x3a, 0x3c; anything else
0x5e. `0x627820` is its `+1` twin. `0x6274a0(target)` gives 0x47, 0x49, 0x4b, 0x4d, 0x4f for target
pairs (0, 1) to (8, 9), `0x627d80` the damage types 0x46 to 0x4e. `0x62a0d0` and `0x62a2e0` give
0x1b to 0x29 and 0x1c to 0x2a in the same shape. `0x627d30(source, target)` and `0x62b190` pick
damage when `GetSpellHostility` returns 2, else buff; `0x627d60` does so for the periodic pair.

## Names

`0x6264e0(guid)` names a line's object: a unit's name, a game object's name, an item's
name from the item cache, a player's from the name cache, or a creature's from the creature
cache; else the `UNKNOWNOBJECT` string, and failing that the literal `UKNOWNOBJECT`.
`0x6264b0(school)` returns a school's `Resistances` name in the client's locale.

Addresses: `0x6264e0`, `0x626526`, `0x62653b`, `0x626551`, `0x626581`, `0x6265b2`, `0x6265f4`, `0x6264b0`
Confidence: verified
Proof: one
Test: none

`0x626526` tests type bit 3 (unit), `0x62653b` bit 5 (game object), `0x626551` bit 1 (item), whose
entry `[+8]+0xc` goes to `DBCache_ItemStats_C_int_HASHKEY_INT_::GetRecord` and `+0x8` of the
record. Unresolved, `0x626581` asks `s_nameCache` for a GUID whose high dword's top 4 bits are 0
and `0x6265b2` `s_creatureCache` for the entry `+4` of one whose top nibble is 0xf, both with the
empty callback `0x626620`; a record's name is its `+0x0`. `0x6264b0` reads the row `+0xc` plus
4·locale.

## Combat text

Each line about the player also raises `COMBAT_TEXT_UPDATE` with a type name from `0x862920`
(`INTERRUPT` to `HONOR_GAINED`, 47 names) and up to two further arguments: a number, a name, or
both. Most formatters raise it when the player, class 0, is the line's target or its one unit; the
faction line always does. Which lines raise which is `combatlogmessages`. One type comes from no log
line: `SPELL_ACTIVE` (0x26), raised by the usable refresh `0x4b31c0` (`ui/reactivespells#usable-refresh`).

Addresses: `0x626890`, `0x6268f0`, `0x626960`, `0x6269b0`, `0x6269d0`, `0x862920`, `0x4b31c0`, `0x4b3221`
Confidence: verified
Proof: one
Test: none

All five call `FrameScript_SignalEvent2(0x21e, …)` with the type's name first: `0x6269d0(type)`
with `%s`; `0x6269b0(type, text)` with `%s%s`; `0x626960(type, n)` with `%s%s` and `%d` of `n`;
`0x626890(type, text, n)` with `%s%s%s`, the text then `%d` of `n`; `0x6268f0(type, n, m)` with
`%s%s%s` and both as `%d`. 0x21e is `COMBAT_TEXT_UPDATE` (542). Index 0x26 of `0x862920` is
`SPELL_ACTIVE`; of the 68 call sites of the five, only the one at `0x4b3226` has 0x26, loaded at
`0x4b3221`.

## Trailers

A damage line ends with trailers, in this order: `GLANCING_TRAILER` for hit info 0x4000,
`CRUSHING_TRAILER` for 0x8000, `RESIST_TRAILER` with a positive resisted amount,
`VULNERABLE_TRAILER` with the negated amount when it is negative, `BLOCK_TRAILER` with a nonzero
blocked amount, `ABSORB_TRAILER` with a nonzero absorbed amount. A trailer whose global string is
missing or empty is left out.

Addresses: `0x628410`, `0x62841e`, `0x62844f`, `0x62847b`, `0x6284c2`, `0x628508`, `0x62854f`, `0x628596`
Confidence: verified
Proof: one
Test: none

`0x628410(type, line, unused, absorbed, resisted, blocked, hit info)` appends each with `SStrPack`
into the 0x800-byte line, formatting the amount into a second 0x800-byte buffer with `SStrPrintf`,
and writes the result with `%s` under the type. `0x62847b test esi,esi; jle` skips 0 and negative
resists; `0x6284c2 jge` skips 0 and positive ones.

## Item waits

A line that names an item not yet in the item cache waits for the cache. `0x628f40`, `0x629610`,
`0x629760` and `0x6298a0` ask the cache with the callback `0x6294b0`; on a miss they keep the
line's arguments in a list of 48-byte entries keyed by item, and when the record arrives the
callback runs each waiting entry's formatter again, range test included.

Addresses: `0x628f40`, `0x6293a0`, `0x6293f0`, `0x62947c`, `0x6294b0`, `0x6294cc`, `0x6295b0`, `0x629730`, `0x629870`, `0x629a30`, `0x62e580`
Confidence: verified
Proof: one
Test: none

`0x6293f0` finds a free entry (`[entry] == 0`) or grows the list by 8 through `0x62e580`, which
reallocates 48-byte `GENERICITEMCACHEDESC` entries keeping the old ones. `0x6293a0`, `0x629730`,
`0x629870` and `0x629a30` fill an entry with the item and the kind 1 to 4 at `+0x4`; `0x6294b0`
bounds the kind below 5 at `0x6294cc`, finds the entries `0x6295b0` matches, frees each, and calls
`0x628f40`, `0x629760`, `0x629610` or `0x6298a0` by kind. `0x629a90` frees the list at shutdown.

## Unreached log code

None: every function of the two files is reached. The seven hooks `0x6301c0` to `0x630230`
return at once (`overview`, Small bodies).

Addresses: `0x6301c0`, `0x630230`
Confidence: verified
Proof: one
Test: none

The census of `overview`, Unreached functions, lists no function of either file.
