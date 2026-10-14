# ui: unit presence, flags, classification and type

The small unit functions of `ScriptEvents.cpp` that answer from one object flag or one table: whether a unit is there, its charm,
tap and elite flags, its classification, its creature type, its relic slot and its ranged skill. The resolver and the argument
checks are `unittokens`; the roster-backed getters are `party`.

Roots: `0x515fb0`, `0x516030`, `0x516410`, `0x516cf0`, `0x516d40`, `0x516d90`, `0x519c90`, `0x519d00`, `0x519de0`, `0x519e50`, `0x51a280`, `0x518b90`

## UnitIsVisible

`UnitIsVisible(unit)` pushes the number 1 when `0x515940` returns an object, else nil. `0x515940` resolves the token with `0x515970` and, for a nonzero GUID, looks
the object up with type mask 8 (a unit); it adds no other test. No `Usage:` check: a nil or missing unit gives nil.

Addresses: `0x516030`, `0x516038`, `0x51603f`, `0x516044`, `0x515940`, `0x515957`, `0x515961`
Confidence: verified
Proof: one
Test: none

`0x515940` is `call 0x515970; or ecx,edx; jne; xor eax,eax` and then the lookup with `ecx = 8` (`0x515957`). A zero GUID returns null before the lookup. A unit that is out of range,
hidden or not yet created has no object, so the function is nil for it; a player's own party member is nil once out of range even though `UnitExists` is 1.

## UnitExists

`UnitExists(unit)` looks the GUID up with type mask 1 (any object). A found object answers through its virtual slot 22 (`[vtable+0x58]`); a nonzero result pushes 1. A missing object, or a refusal,
falls to `0x491900`, which is 1 for a party or raid member or pet and 0 for a zero GUID. Nil when both fail. There is no `Usage:` check.

Addresses: `0x515fb0`, `0x515fca`, `0x515fe1`, `0x515fea`, `0x515fee`, `0x515ff8`, `0x516012`, `0x491900`
Confidence: verified
Proof: one
Test: none

Slot 22 is the target code's selectability test (`objectlayer/units#selectability`): a unit with `UNIT_FIELD_FLAGS` bit 25 not created by the active player refuses, so `UnitExists` is nil for it unless
it is on the roster. The roster fallback receives the GUID that the token named, whether or not an object exists.

## Flag predicates

Four predicates read one word of the unit's fields and push 1 or nil. They take no `Usage:` check and answer nil for a unit that does not resolve to a held unit object.

| function | test |
|---|---|
| `UnitPlayerControlled` | `UNIT_FIELD_FLAGS` (`+0xa0`) bit 3 |
| `UnitIsPlusMob` | `UNIT_FIELD_FLAGS` bit 6 |
| `UnitIsCharmed` | `UNIT_FIELD_CHARMEDBY` (`+0x10`, `+0x14`) nonzero |
| `UnitIsCivilian` | `0x612550` (below) |

Addresses: `0x516410`, `0x516434`, `0x516d40`, `0x516d64`, `0x516cf0`, `0x516d0e`, `0x516d11`, `0x519de0`, `0x519e1c`
Confidence: verified
Proof: one
Test: none

`UnitIsCharmed` is true for the charmed unit and nil for its charmer: the field holds the charmer's GUID. No function reads the charmer's side, and no `UnitIsPossessed` exists in the table.
`UnitIsCivilian` has a `Usage:` check and calls `0x612550`: a PvP-flagged creature (`UNIT_FIELD_FLAGS` bit 12) whose record has the civilian byte, a reaction toward the active
player of 1 or less and `0x5f0700` (`unitlevels#trivial-units`) true (`objectlayer/units#creature-records`).

## Tapped and the dynamic flags

`UnitIsTapped(unit)` is 1 when `UNIT_DYNAMIC_FLAGS` (`+0x224`) has bit 0x4; `UnitIsTappedByPlayer(unit)` when it has bit 0x8. Both raise on a missing argument and answer nil for an unresolved unit.
The client reads the word in eight places by bit:

| bit | meaning | reader |
|---|---|---|
| 0x1 | lootable: sparkle on, tutorial 6 | `0x600440` |
| 0x2 | shown by tracking | `0x5ed244` |
| 0x4 | tapped; a change signals `UNIT_FACTION` (id 29) | `0x6005a1`, `0x519cd0` |
| 0x8 | tapped by the player | `0x519d40` only |
| 0x10 | has special tooltip information | `0x52a8ea` |
| 0x20 | dead, beside health | 22 load sites |

Addresses: `0x519c90`, `0x519cd0`, `0x519d00`, `0x519d40`, `0x600440`, `0x604b30`, `0x6005a1`, `0x6005b0`, `0x5ed210`, `0x5ed244`, `0x52a8ea`, `0x4b5390`, `0x515e50`
Confidence: verified
Proof: one
Test: none

`0x600440(old)` is the field watcher (`objectlayer/units#health-death-and-resurrection`): on bit 0x1 falling it removes the sparkle, rising it calls `0x4b5390(6, 0)` (the tutorial
trigger, `tutorials#the-tutorial-trigger`), and on bit 0x4 changing it signals unit event `0x1d` (29, `UNIT_FACTION`). Nothing in the client sets bit 0x8 or reads it except the Lua function; a change of
it signals no event. The unit's fields are written only by the update block, so no client store sets a bit. Bit 0x20 is tested as `shr 5; test 1` beside `health > 0` in the attack and assist
gates, the name-plate gate, the getters and the death animation code; bit 0x1 is also read at `0x6003c1`, `0x60057c`, `0x6005d9` and `0x624552`.

## Classification

`UnitClassification(unit)` pushes one word from the table at `0x850424`: `normal`, `elite`, `rareelite`, `worldboss`, `rare`, indexed by `0x605620`, the creature rank. An unresolved unit pushes `normal`. No `Usage:` check.

Addresses: `0x516d90`, `0x516daa`, `0x516daf`, `0x516dc4`, `0x850424`, `0x605620`
Confidence: verified
Proof: one
Test: none

`0x605620` returns the `+0x20` dword of the unit's creature record (`+0xb30`), or 0 when the unit has no record or has a pet number (`UNIT_FIELD_PETNUMBER`, `+0x214`, nonzero). The index is not bounded: a rank of 5
or more reads the next words of the table, which are the name pointers of the function registrar (`0x850438` holds the `UnitExists` string). A player has no record, so it is `normal`.

## The creature type

`UnitCreatureType(unit)` raises `Usage: UnitCreatureType("unit")` for a missing argument and pushes the localised type name, or nil. `0x605570` finds the type id in three stages: the shapeshift form of
the unit (`UNIT_FIELD_BYTES_1` byte 2, `+0x212`), when its row has a type above 0; else the creature record's type (`+0x18`) when the unit has a record; else the race row's type, when above 0. The
name is `CreatureType.dbc` row word `4 + 4*locale`.

Addresses: `0x51a280`, `0x51a2bc`, `0x51a2c7`, `0x51a2cf`, `0x51a2e2`, `0x605570`, `0x605576`, `0x60559e`, `0x6055a8`, `0x6055ca`, `0xc0d76c`, `0xc0d774`, `0xc0d778`, `0xc0de2c`, `0xc0de30`, `0xc0ded8`, `0xc0dee0`, `0xc0dee4`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

The form row is read at `+0x30` and the race row at `+0x24`. The record leg returns its dword unchecked, including 0 and a negative id: it ends the search, so a record with type 0 gives nil and
never falls to the race. The final lookup accepts an id from 1 to the highest id and a nonnull row (`0xc0de30`, the id map `0xc0de2c`). The record at `+0xb30` is set only for creature objects: `0x60b160` stores
it (a creature type), `0x5fae1a` zeroes it at construction and `0x60464b` and `0x6046c9` store a null when the entry is 0. A player answers by its race row.

## UnitHasRelicSlot

`UnitHasRelicSlot(unit)` raises `Usage: UnitHasRelicSlot("unit")` for a missing argument and pushes 1 for a unit that is a player (type bit 0x10) whose class row has a nonzero dword at `+0x40`, else nil.

Addresses: `0x519e50`, `0x519e8a`, `0x519e98`, `0x519eae`, `0x519ebb`, `0xc0def4`, `0xc0def8`
Confidence: verified
Proof: one
Test: none

The class byte is `UNIT_FIELD_BYTES_0` byte 1 (`+0x79`), bounded by `0xc0def8` and looked up in the `ChrClasses` row array `0xc0def4`; a missing row gives nil. The type test is bit 4 of the object's type word
(`[[obj+8]+8]`).

## UnitRangedAttack

`UnitRangedAttack(unit)` raises `Usage: UnitRangedAttack("unit")` for a missing argument and always pushes two numbers: the ranged skill with its permanent bonus, and the temporary bonus. Both are 0 for
every unit but the active player, for an empty ranged slot and for a ranged item that is not a weapon.

Addresses: `0x518b90`, `0x518bca`, `0x518be5`, `0x518bea`, `0x518bfa`, `0x612b40`, `0x5edae0`, `0x5ea460`, `0x5ea4a1`, `0x5ea4f6`
Confidence: verified
Proof: one
Test: none

`0x612b40` runs `0x5edae0` for a player object and zeroes both outputs otherwise. `0x5edae0` takes hand 2 from the weapon slot function (`[vtable+0x98]`), requires class byte 2, indexes the weapon-skill
spell table with the subclass byte (`objectlayer/playercombat#weapon-skills`) and finds the skill line through `0x6de040`. `0x5ea460` answers 0 and 0 unless the object is the active player. Its first
value is the skill block's value word (`+0x84c + 0xc*i` of the player block `+0xe68`) plus the permanent bonus word (`+0x852`) when the value is above 0; its second the signed temporary word (`+0x850`). Nothing
clamps. `fild` pushes both as doubles.

## Player, corpse and attack tests

`UnitIsPlayer(unit)` pushes 1 when the token's GUID has an object of type mask 0x10 (a player) or is a party or raid member by GUID (`0x4918e0`, members only, not pets), else nil; a party member out of range is a player to it.
`UnitIsCorpse(unit)` pushes 1 when the GUID has a corpse object (type mask 0x80), which a dead unit does not have. `UnitCanAttack(unit, other)` pushes 1 when both tokens resolve to unit objects (mask 8) and `0x606980(unit, other)` holds.

Addresses: `0x516150`, `0x516181`, `0x51618d`, `0x4918e0`, `0x5161c0`, `0x5161e5`, `0x516c50`, `0x516ca5`, `0x606980`
Confidence: verified
Proof: one
Test: none

The first two take no `Usage:` check; `UnitCanAttack` raises `Usage: UnitCanAttack("unit", "otherUnit")` unless both arguments are strings. A missing or unresolved unit on either side gives nil.
