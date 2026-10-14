# ui: unit tokens and the unit functions' arguments

Every unit function takes a token such as `player`, `target` or `party2target`, and every unit event passes one. The
resolver turns a token into a GUID, the wrapper into a unit object; the inverse walk lists the tokens of a GUID; a generic watcher
turns a change of a unit field into an event with those tokens as `arg1`. This doc also holds what the unit
functions check of their arguments (`tables/unit_gates.tsv`) and what they push (`tables/bindings.tsv`).

Roots: `0x515970`, `0x515940`, `0x515c50`, `0x515e50`, `0x51bbb0`, `0x51bdb0`, `0x51bd50`

## The unit token

`0x515970(token)` returns a GUID in `edx:eax`, 0 for none, and 0 for a null or empty token. `SStrCmpI` prefix tests, folding `A` to `Z` only, run in this order and the first match wins: `player` (6 characters), `pet` (3), `target` (6), `partypet` (8), `party` (5), `raidpet` (7), `raid` (4), `mouseover` (9), and `npc` (the whole string).
`player` is the GUID in the active player's descriptor; `target`, `mouseover` and `npc` are globals (`0xb4e2d8`, `0xb4e2c8`, `0xb4e2d0`). A token matching none raises
`Unknown unit name: %s`. Then any number of `target` hops: while the GUID is nonzero and text remains, the rest must be `target` (else GUID 0) and the GUID becomes
that unit's target. `npctarget` raises, since `npc` is compared whole.

Addresses: `0x515970`, `0x5159b4`, `0x5159d3`, `0x5159e9`, `0x5159f8`, `0x515a1c`, `0x515a3f`, `0x515a75`, `0x515a9a`, `0x515adb`, `0x515b1c`, `0x515b64`, `0x515ba5`, `0x515bec`, `0x515c14`, `0x515c1a`, `0x64a4c0`, `0x414310`, `0xb4e2c8`, `0xb4e2d0`, `0xb4e2d8`
Confidence: verified
Proof: one
Test: none

The ordering matters twice: `partypet` before `party` and `raidpet` before `raid`, since the compares are prefixes. Each compare is `SStrCmpI(token, literal, n)` with `n` the literal's length, except `npc`
(`0x7fffffff`). The case fold is `'A'..'Z'` plus 0x20 in `_strnicmp` (`0x414333` to `0x414352`); the locale path is not reachable because the locale handle word is 0 and no code stores to it, so a
byte of 0x80 or more is never folded. The target hop reads `[[obj+0x110]+0x28]` and `+0x2c` (the `UNIT_FIELD_TARGET` pair), using `0x468460` with the unit type mask (8). The failure string is raised by the `L`-less
`luaL_error` at `0x7040e0`. There is no `focus`, `boss` or `none` token.

## Indexed tokens

For `party`, `partypet`, `raid` and `raidpet` the digits after the prefix are summed in base 10 with no cap (`acc*10 + digit`, wrapping at 32 bits), stopping at the
first byte outside `0` to `9`. The 1-based number is decremented, so a missing suffix gives index `0xffffffff`. `party` takes indices below 4 from the GUID table `0xbc6f48` and 0 otherwise.
`raid` and `raidpet` take indices below the raid count `0xb713e0` from the roster. `partypet` has no bound: it reads the table with the index as given, then the
member record. A pet GUID is the owner's charm GUID when nonzero, else its summon GUID. `0x515940(token)` returns the unit object of the GUID, null for GUID 0.

Addresses: `0x515940`, `0x515949`, `0x51595c`, `0x515aab`, `0x515aec`, `0x515b2d`, `0x515b75`, `0x491920`, `0x491930`, `0x491940`, `0x491960`, `0x4e81a0`, `0x4e81d0`, `0x4e81dc`, `0x4e8221`, `0x515a64`, `0x515a6b`, `0x468460`
Confidence: verified
Proof: one
Test: none

The four handlers are reached through `jmp` stubs (`0x491920` to `0x4e81a0`, `0x491930` to `0x4e81d0`) or directly (`0x491940`, `0x491960`). `0x4e81a0` first tests the object manager
(`0x467d00`) and compares the index with 4 as unsigned. `0x4e81d0` calls the same manager test and then indexes `[idx*8+0xbc6f48]` and the member records `idx * 0x148 + 0xbc70b0` with no compare, so
`partypet`, `partypet0` and `partypetX` read the eight bytes below the table. The `pet` token and the raid pet handler both use the charm-else-summon rule; the raid pet handler falls back to the roster record's pet GUID (`+0x90`).

## Raise and quiet

A token that names nothing is not an error: a null or empty string, a prefix with a bad suffix (`playerfoo`), an index out of range, an unset `target` and a unit without a target each give GUID 0
and a quiet nil, 0 or empty answer. Only a token whose start matches no prefix raises, and a number argument is stringified first, so `UnitName(5)` raises too. Every
lookup guards against GUID 0: the object manager (`0x46487b`), the name cache (`0x55f0a5`) and the party and raid scans (`0x4e882a`, `0x4bb0fb`) return nothing for it.

Addresses: `0x515992`, `0x51599d`, `0x515c2d`, `0x515c31`, `0x515c3c`, `0x46487b`, `0x55f0a5`, `0x4e882a`, `0x4bb0fb`, `0x6f3690`
Confidence: verified
Proof: one
Test: none

`lua_tostring` returns null for a nil, boolean or table argument and a string for a number, so `0x515992` and `0x51599d` give 0 for those and the number raises later in the chain. The prefix chain
ends at `0x515c14` with the push of the format and `0x515c1a`, the raise.

## Argument checks

`tables/unit_gates.tsv` lists, for the 83 unit functions, the arguments each checks and its `Usage:` string. 53 check: a failed check is a raise before any lookup, so a nil or missing
unit raises, with one exception: `GetPVPRankInfo`'s argument 2 is a probe, not a check (`0x51a97c`, `0x51a998`): its result is tested and it never raises. `lua_isstring` accepts a number, so a numeric unit passes the check and raises in the resolver. The other 30 take no check: 17 take no unit token at all,
and 13 take a token and answer quietly for a missing one (`UnitExists`, `UnitIsVisible`, `UnitIsPlayer`, `UnitIsCorpse`, `UnitIsPartyLeader`, `UnitInParty`,
`UnitPlayerOrPetInParty`, `UnitInRaid`, `UnitPlayerOrPetInRaid`, `UnitPlayerControlled`, `UnitIsCharmed`, `UnitIsPlusMob`, `UnitClassification`).

Addresses: `0x6f3510`, `0x6f34d0`, `0x6f4940`, `0x6f4440`, `0x6fc780`, `0x6f5d80`, `0x742014`
Confidence: verified
Proof: one
Test: none

`luaL_error` does not return: `lua_error` calls `luaG_errormsg`, which has no `ret` and ends in a jump to `luaD_throw`, whose live leg is the C runtime `longjmp`. The `xor eax,eax; ret`
after each such call is dead code. A raise from a widget script is caught by the script runner's `lua_pcall` and shown by the interface error handler.

## What the functions push

The predicates push the number 1 (the double `0x3ff0000000000000`, from `push 0x3ff00000; push 0`) for true and nil for false, one value each. No unit function pushes a Lua boolean:
`lua_pushboolean` is `0x6f39f0` and has 7 callers, none inside the 83 unit functions or anything they call; `IsPetAttackActive` is the one game function that pushes a boolean, and `rawequal`,
`pcall` and `xpcall` the library ones. The numeric getters push the number 0 for a unit that resolves to nothing, and the string getters nil. `UnitReaction` pushes the engine's rating plus 1, nil when
either token fails to resolve, never 0.

Addresses: `0x6f39f0`, `0x4be15c`, `0x4be16c`, `0x70311b`, `0x70331b`, `0x70337b`, `0x703580`, `0x7046d6`, `0x4be0e0`, `0x5167e0`, `0x516836`, `0x516839`, `0x51683e`, `0x51685f`, `0x6061e0`
Confidence: verified
Proof: one
Test: none

The seven call sites are the whole `e8` census; the other three are the date table field helper, the saved variable serializer and the base library. `tables/bindings.tsv` has the push kinds of every function, and the two rows
with kind `(boolean)` are `IsPetAttackActive` and `rawequal`. `0x6061e0(a, b)` returns 4 for the same unit twice, so `UnitReaction("player", "player")` is 5 (`inc eax; fild`, `0x51683e`).

## The tokens of a GUID

`0x515c50(&guid, &count)` builds the list of tokens that name a GUID, in this order: `player` (the active player's GUID), `pet` (its charm else summon), `target`, `party%d` or `partypet%d` (the first of
slots 1 to 4 that matches), `raid%d` or `raidpet%d` (the first of 1 to 40), `npc`, `mouseover`. It returns an empty list for GUID 0 or with no active player object. Nothing removes duplicates, so one
unit can have several tokens, at most 7. The strings are the seven 0x20-byte buffers of a global table (`0x8503d8`), overwritten by the next call. `0x515e50(&guid, id)` signals event `id` once
per token with the format `%s` and the token as `arg1`.

Addresses: `0x515c50`, `0x515c6a`, `0x515ca0`, `0x515cd7`, `0x515d05`, `0x515d50`, `0x515d63`, `0x515db3`, `0x515dc6`, `0x515df9`, `0x515e27`, `0x515e50`, `0x515e5c`, `0x515e7a`, `0x703f50`, `0x8503d8`
Confidence: verified
Proof: one
Test: none

The party and raid loops stop at their first match (`0x515d61`, `0x515dc4` jump past the index increment into the formatting). The local player is in the raid roster but not in the party table, so `player` and a
`raid%d` can both appear and `player` and a `party%d` cannot. The inverse of the token walk is `0x515970`, whose literals are the same nine.

## Unit field events

A change of a named unit field signals an event of the same name with the tokens of the unit. `0x51bbb0(object)` registers, for each field index 0 to 0xb5 whose name in the event table `0xbe1198` is nonzero, one
watcher on the unit with the callback `0x51bd50`. The watch length is 4 bytes, except index 0, 2, 4 and 10 (8 bytes), 41 (0xd8, the aura array), 107 (0x30) and 167 and 174 (0x1c). A player also gets two
watchers with fixed ids, callback `0x51bd90` for the player block at `+0x28` (id `0x20a`) and `0x51bd70` for `+0x118` (id `0xba`). `0x51bd50` signals event `offset / 4` through `0x515e50`. The fields and ids are `tables/events.tsv`
ids 0 to 0xb5.

Addresses: `0x51bbb0`, `0x51bbb6`, `0x51bbbd`, `0x51bbc7`, `0x51bbcf`, `0x51bbd6`, `0x51bc05`, `0x51bc17`, `0x51bc25`, `0x51bc52`, `0x51bc79`, `0x51bd50`, `0x51bd5b`, `0x51bd70`, `0x51bd90`, `0x51bdb0`, `0x51bc84`, `0x51bc98`, `0x467e70`, `0x467fb0`, `0xbe1198`
Confidence: verified
Proof: one
Test: none

The selector table `0x51bc98` has a byte per field index; the byte picks the length through the five entries at `0x51bc84` (8, 0xd8, 0x30, 0x1c, 4), and an index past 0xae uses 4. `0x51bdb0` is the mirror, removing the same watchers with the same callback and argument,
which is how they match. The watchers fire through `objectlayer/fields#when-a-watcher-runs`: the whole watched span is compared with its copy, so any change of the dword signals `UNIT_FLAGS` and an equal rewrite signals nothing. A create block for an object that does not exist yet runs no
watchers, so `UNIT_FLAGS` does not signal for a unit's first appearance; a create block for an existing object compares every field.

## When a unit has watchers

The registration is gated by a mask of references kept at `+0xc90` of the unit: `AddUnitTokenFlags` registers the watchers when the mask goes from 0, `RemoveUnitTokenFlags` removes them when it
returns to 0, and `UpdateUnitTokenFlags` recomputes it (`objectlayer/units`). A unit nobody references has no watchers and signals nothing. `UNIT_FLAGS` has one producer: the reflex `0x6044b0` on the same field signals
`PET_ATTACK_START` and `PET_ATTACK_STOP` and `UNIT_AURA`, never `UNIT_FLAGS`.

Addresses: `0x60c4a0`, `0x60c4f0`, `0x60c520`, `0x60c65c`, `0x60c679`, `0x6044b0`, `0x5ff580`, `0x5ff793`, `0x5ff79a`, `0x5ff7d7`, `0x5ff7e2`, `0x6042ec`
Confidence: verified
Proof: one
Test: none

The aura event also has explicit producers (`mov edx,0x29` at `0x5e57dd`, `0x5e58f9`, `0x612dcf`) and the bit 27 arm of `0x5ff580` (`test dword [ebp-4],0x8000000`). The power events add the unit's power type to a base (`add edx,0x11`
and `add edx,0x17`) with no range check, so a power type of 5 aliases `UNIT_MAXHEALTH`.
