# ui: time, console and the remaining unit functions

The Lua functions of `ScriptEvents.cpp` that read the clock, run a console line, deny file access, compare units and read a
unit's party, faction, reaction, experience, money and control. The token, the argument checks and the push kinds of the unit
functions are `ui/unittokens`; the flag predicates are `ui/unitinfo`; the stat readers are `ui/unitstats`.

Roots: `0x515ea0`, `0x515ee0`, `0x515f10`, `0x516070`, `0x516460`, `0x516630`, `0x516890`, `0x517350`, `0x518160`, `0x51a120`

## Time, console and files

`GetTime()` returns the asynchronous millisecond clock in seconds. `GetGameTime()` returns the hour and then the minute of the game clock
(`0xce8538`: the minute at `+0`, the hour at `+4`). `ConsoleExec(line)` runs a console line without recording it in the history
(`ConsoleCommandExecute` with a zero history flag, `0x63ce38`; `console/screen#writing-and-executing`) and raises `Usage: ConsoleExec("console_command")` for a non-string. `ReadFile`, `DeleteFile` and
`AppendToFile` take no argument and always raise `Access Denied`.

Addresses: `0x515ea0`, `0x515ea9`, `0x515ebb`, `0x515ee0`, `0x515ee7`, `0x515ef4`, `0x515f10`, `0x515f43`, `0x515f50`, `0x515f56`, `0x515f70`, `0x515f90`, `0x42c010`, `0x63ce00`, `0x63ce38`, `0xce8538`
Confidence: verified
Proof: one
Test: `ui::luabind_515ea0_get_time`

`0x515ebb` multiplies the milliseconds by the double 0.001. `0x515ee7` pushes the dword at `0xce853c` (the hour) and `0x515ef4` the one at `0xce8538`. `0x515f43` calls `ConsoleCommandExecute`. The three file functions are `push "Access Denied"; call luaL_error` and nothing else.

`ui::luabind_515ea0_get_time` runs the scaling of `GetTime`, the slice `0x515eb8` to `0x515ec4` (`fild qword`, `fmul` by 0.001, `fstp`).

## Same unit and membership

`UnitIsUnit(a, b)` returns 1 when the two strings are equal without regard to case, even if neither names a unit, and otherwise when both resolve
to the same nonzero GUID; else nil. A non-string raises `Usage: UnitIsUnit("unit", "otherUnit")`. `UnitInParty(unit)` returns 1 when the GUID is a
party member, `UnitPlayerOrPetInParty(unit)` 1 for a party member or pet and `UnitPlayerOrPetInRaid(unit)` 1 for a raid member or pet, else nil; none of the three checks its argument.

Addresses: `0x516070`, `0x5160c4`, `0x5160cd`, `0x5160e8`, `0x516100`, `0x516137`, `0x516290`, `0x5162b3`, `0x5162f0`, `0x516313`, `0x5163b0`, `0x5163d3`, `0x4e7f70`, `0x4e8fa0`, `0x4bafd0`
Confidence: verified
Proof: one
Test: none

`0x5160c4` compares the strings with a limit of 0x7fffffff and `0x5160cd` pushes 1.0 on a match. `0x5160e8` and `0x5160f4` resolve both tokens with `GetGUIDFromName` and `0x516100` refuses a zero GUID. `0x5162b3` calls `CGPartyInfo::IsMember`, `0x516313` the party member-or-pet test and `0x5163d3` `CGRaidInfo::IsMemberOrPet`.

## PVP flags

`UnitIsPVP(unit)` returns 1 for a unit that is flagged for PVP (`IsPVPUI`), for a corpse whose field at `+0x74` has bit 2, and, when the token names no object
but a remote party member, for a remote stats byte with bit 1. `UnitIsPVPFreeForAll(unit)` returns 1 for a unit that has the flag bit 3 of its unit flags and
whose controlling player has the free-for-all flag (bit 7 of the player flags), or for a remote party member with bit 4 of the stats byte. Both raise the usage text for a non-string and answer nil otherwise.

Addresses: `0x516460`, `0x5164d1`, `0x5164e6`, `0x5164f9`, `0x516502`, `0x516540`, `0x5165b3`, `0x5165bc`, `0x5165c3`, `0x5165d5`, `0x5165e5`, `0x5165f1`, `0x605ff0`, `0x606170`, `0x496400`
Confidence: verified
Proof: one
Test: none

`0x5164d1` calls `IsPVPUI` for an object of unit type; `0x5164e6` reads the corpse's `+0x74` and `0x5164e9` shifts it by 2. `0x5164f9` and `0x5165e5` call `GetRemoteStats` for a token with no object; `0x516502` tests bit 1 and `0x5165f1` bit 4. `0x5165b3` tests bit 3 of the unit flags (`+0xa0`) and `0x5165c3` fetches the controlling player.

## Faction and reaction

`UnitFactionGroup(unit)` returns the group token and its localized name for the first faction group whose mask the unit's faction template contains,
and nil, nil when none. The template comes from the unit's `+0x74` field, a corpse's race template, or, for a player token with no object, the race
in the name cache. `UnitReaction(a, b)` is `ui/unittokens`. `UnitIsEnemy(a, b)` returns 1 when both resolve and `UnitReaction` is 1 or less, else nil;
`UnitCanAssist(a, b)` returns 1 when both resolve and `CanAssist_Unit` allows it.

Addresses: `0x516630`, `0x5166b8`, `0x5166be`, `0x5166cd`, `0x5166f4`, `0x516701`, `0x516744`, `0x51674a`, `0x51675e`, `0x516764`, `0x516771`, `0x5167b1`, `0x516792`, `0x516890`, `0x5168e5`, `0x5168ea`, `0x516bb0`, `0x516c05`, `0x6061e0`, `0x6066f0`, `0x5d7120`, `0x55f080`
Confidence: verified
Proof: one
Test: none

`0x5166b8` loads the unit block and `0x5166be` its faction template id, `0x5166cd` calls the corpse's race template and `0x5166f4` and `0x516701` the name cache race. `0x51674a` loads the template's group mask (`+0xc`), `0x516764` tests the mask against `1 << [group+4]` and `0x5167b1` pushes the token at `+8` and the name at `+0xc` by locale. `0x5168ea` compares the reaction with 1 (`jg` gives nil).

## Player numbers

`GetDodgeChance()`, `GetBlockChance()` and `GetParryChance()` return the float of the player field block at `+0xe5c`, `+0xe58` and `+0xe60`, 0 without a
player. `UnitXP(unit)` and `UnitXPMax(unit)` return the active player's experience (`+0x840`) and the experience needed (`+0x844`); a unit that is not a player, or not
the active one, gives 0. `UnitCharacterPoints(unit)` returns the two counters at `+0xe48` and `+0xe4c` for the active player and 0, 0 otherwise. `GetMoney()` returns the coinage (`+0xf70`)
and `UnitOnTaxi(unit)` 1 when bit 20 of the unit flags is set. `HasFullControl()` returns 1 for a player with bit 0 of the dword at `+0x1c70` clear and health (the unit field at `+0x40`) above 0, else nil.

Addresses: `0x516f00`, `0x516f2a`, `0x516f60`, `0x516f8a`, `0x516fc0`, `0x516fea`, `0x517350`, `0x5173c9`, `0x517410`, `0x517489`, `0x519420`, `0x519477`, `0x5194a7`, `0x5194c4`, `0x518160`, `0x5181ae`, `0x517a40`, `0x517a86`, `0x51a120`, `0x51a142`, `0x51a151`
Confidence: verified
Proof: one
Test: none

`0x5173c9` and `0x517489` load the experience and its limit. `0x519477` starts the loop at offset 0xe48 and `0x5194c4` ends it at 0xe50. `0x517a86` shifts the unit flags by 20. `0x51a142` tests the dword at `+0x1c70` and `0x51a151` loads the health at `+0x40` of the unit block.
