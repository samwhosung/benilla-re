# ui: selecting a target

How the interface changes the player's selected target: the one function that sets it, the verbs that call it by token, by
name and by cycling, and the name match and the candidate list behind them. The code is `GameUI.cpp` (`0x489a40` to
`0x48a070`, `0x493540` to `0x494540`). The reaction and attack tests belong to `objectlayer`, the unit tokens to
`ui/unittokens`, and the range arithmetic to `ui/distancegates`.

Roots: `0x4899d0`, `0x489a80`, `0x489aa0`, `0x489ac0`, `0x489ae0`, `0x489b00`, `0x489b40`, `0x489b50`, `0x489b80`,
`0x489c40`, `0x489d60`, `0x489e00`, `0x489ec0`, `0x489ff0`, `0x493540`

## Setting the target

`0x493540(guid)` selects a unit, or clears the selection for GUID 0. It does nothing when the protected gate for kind 3
fails or when the active player is charmed (the descriptor dwords at `+0x10` and `+0x14` are not both 0). For a nonzero GUID
it needs the object: while a spell awaits a target the object is bound as the spell's target and nothing else happens;
otherwise its virtual slot `+0x58` must answer nonzero. Selecting the current target does nothing. Otherwise the old target
is kept for `TargetLastTarget`, the new one is stored and lit, and an attackable one is kept for `TargetLastEnemy`.
`CMSG_SET_SELECTION` (0x13d) is sent, `PLAYER_TARGET_CHANGED` is signalled, and an attacking player attacks the new target.

Addresses: `0x493540`, `0x493561`, `0x493590`, `0x4935ca`, `0x4935d5`, `0x4935ec`, `0x4935fb`, `0x49361d`, `0x493622`, `0x493639`, `0x4936a3`, `0x4936e2`, `0x493701`, `0x49372a`, `0x49377d`, `0x4937be`, `0x4937ff`, `0x493815`, `0x49383c`, `0x493843`, `0x493871`, `0x493892`, `0x49389c`, `0x4938c8`, `0xb4e2d8`, `0xb4e2e0`, `0xb4e2e8`, `0xb4e3b0`, `0x6e5b40`, `0x494a50`
Confidence: verified
Proof: one
Test: none

`0x493561` calls the gate `0x494a50(3)` and `0x493590` reads the charm words. `0x4935fb` zeroes the cycle cache time
`0xb4e3b0` for GUID 0. The old target is saved to `0xb4e2e0` at `0x493622`. `0x493639` tests `HasAttackTarget`, `0x4936a3` the
attackability of the current target (alive, not mounted, a living or flagged target, `CanAttack`), and the result decides
`0x4938c8`, which calls `StartMeleeAttack` with the new GUID. `0x493701` lights the object and `0x49372a` adds token flag
0x1000. A unit that `CanAttack` stores its GUID at `0xb4e2e8` (`0x49377d`). The tutorial triggers 18, 17, 4 and 34 are
fired at `0x4937be`, `0x4937ff`, `0x493815` and `0x49383c`. `0x493843` plays the interface sound `igCharacterSelect` for a
missing object. `0x493871` sends the message; `0x493892` refreshes usable states.

## Releasing the old target

`0x493910(sendSelection, guid)` runs when the target is replaced (`sendSelection` 0) or cleared (1), with the old target's
GUID. It returns at once with no target stored, or when the GUID differs from the stored one. Otherwise it ends the loot
window when that was the looted object, clears the old object's light and its token flag 0x1000, zeroes the stored target,
calls `StopMeleeAttack` on the active player and, when the player has a possessed far-sight unit, `SendPetStopAttack`. With
`sendSelection` set it sends `CMSG_SET_SELECTION` with GUID 0 and signals `PLAYER_TARGET_CHANGED` itself.

Addresses: `0x493910`, `0x493959`, `0x493974`, `0x4939a7`, `0x4939c5`, `0x4939d3`, `0x4939fb`, `0x493a08`, `0x493a0f`, `0x493a18`, `0x493a2a`, `0x493a73`, `0x4936cc`, `0x4938f8`, `0xb71b48`, `0x48f200`, `0x4bd650`, `0x5ecac0`, `0x5ee5a0`, `0x6144f0`, `0x60c4f0`
Confidence: verified
Proof: one
Test: none

`0x4936cc` passes 0 for a replacement and `0x4938f8` passes 1 for a clear. `0x493949` compares the GUID argument with the
stored target (`0x493a95` returns on a mismatch). The loot end `0x48f200` is called at `0x493974` when `0xb71b48` equals
the target. `0x493a08` calls `StopMeleeAttack` and `0x493a0f` `GetPossessedFarSightUnit`, whose nonzero answer reaches
`0x493a18`. `0x493a2a` pushes 0x13d and `0x493a73` signals event 0xc3.

## Target by token

`TargetUnit("unit")` resolves the token to a GUID through `GetGUIDFromName` and calls `CGGameUI::Target`, which selects the
GUID when its unit object exists or it names a raid member or pet. `ClearTarget()` calls `0x493540(0)` and returns 1 if there
was a target, else nil. `TargetLastTarget()` selects the GUID kept at `0xb4e2e0`, or clears the target when it is 0,
`TargetLastEnemy()` the one kept at `0xb4e2e8`. `AttackTarget()` calls `StartAttack` on the active player. All raise
`Usage: TargetUnit("unit")` and the like for a non-string argument, except the argumentless ones.

Addresses: `0x4899d0`, `0x489a08`, `0x489a16`, `0x489a40`, `0x489a74`, `0x489ff0`, `0x48a00e`, `0x489b00`, `0x489b1e`, `0x489b2d`, `0x489b40`, `0x489b50`, `0x489b75`, `0x515970`, `0x491900`, `0x60ecb0`, `0x6131a0`
Confidence: verified
Proof: one
Test: none

`0x489a59` looks the GUID up with unit mask 8 and `0x489a64` falls back to `CGGameUI::IsRaidMemberOrPet`. `0x489ff0` ORs the
two words of `0xb4e2d8` for its return value before the call. `0x489b00` loads the pair `0xb4e2e0` and `0xb4e2e4`; both zero
takes `0x489b29`. `0x489b75` calls `CGUnit_C::StartAttack(0, 0)` on the player object (type mask 0x10) when it exists.

## Assist and follow

`AssistUnit("unit")` selects the unit's own target, the GUID in its descriptor at `+0x28`, and, when the `assistAttack`
option is nonzero, starts attacking it; a token that resolves to no unit shows `ERR_GENERIC_NO_TARGET`, and a unit with no
target does nothing. `FollowUnit("unit")` starts the follow auto-move toward the unit for the mover, with mode 3; an
unresolved token shows `ERR_GENERIC_NO_TARGET`. `ToggleSheath()` calls `ToggleSheathe` on the player.

Addresses: `0x489b80`, `0x489ba9`, `0x489bb2`, `0x489bcb`, `0x489bd0`, `0x489c02`, `0x489c0e`, `0x489e00`, `0x489e5c`, `0x489e74`, `0x489e7f`, `0x48a070`, `0xb4d8f8`, `0xc4da98`, `0x515940`, `0x60fed0`, `0x5ecb70`
Confidence: verified
Proof: one
Test: none

`0x515940` resolves the token. `0x489bd0` loads the `assistAttack` record `0xb4d8f8` and tests its integer at `+0x28`;
the error is message 0xb8 (`0x489c0e`). `0x489e00` finds the mover by the GUID `0xc4da98` and calls
`StartAutoMoveToObject(guid, 3)` (`0x489e74`, `0x489e7f`).

## Matching a name

`0x493aa0(name, typeMask, mode, exactOnly, maxDistance)` returns the GUID of the best unit for a typed name, or 0. Mode 0
and 2 try the four party slots, then the raid, then every object; mode 1 tries only objects; mode 3 tries party and objects.
Party and raid members take the same compare as objects, so they match by prefix too. An object must
pass the type mask and the filter, which receives the search's mode. An equal name nearer than the best distance so far ends the search. Otherwise, unless `exactOnly`, a unit counts by the prefix it shares with the text, at least one character: a longer prefix wins, and at equal length the nearer unit.

Addresses: `0x493aa0`, `0x493b24`, `0x493b60`, `0x493b80`, `0x493b93`, `0x493b9d`, `0x493c2f`, `0x493c34`, `0x493c60`, `0x493c7f`, `0x493c8b`, `0x493cc0`, `0x493cf8`, `0x493d30`, `0x493d7c`, `0x493dae`, `0x493e0f`, `0x4e81a0`, `0x468380`, `0xc0e228`, `0xb713e0`, `0xb712a8`
Confidence: verified
Proof: one
Test: none

`0x493aa0` stores the search record at `ebp-0x38`: mask, mode, name, exact flag, result GUID (`+0x18`), best prefix length
(`+0x20`, starting at 1) and best distance (`+0x24`, starting at the argument). Mode 1 jumps to the enumeration
(`0x493b1a`), mode 3 skips the raid (`0x493b93`). `0x493c60` is the compare: an equal name (`SStrCmpI`) under the best
distance (`fcomp` with `test ah,5`) stores the GUID and returns 0, which stops `0x468380`; else, with the exact flag clear
(`+0xc`), the loop at `0x493cc0` upper-cases both strings and counts equal leading characters, and `0x493cf8` keeps the longer
or, at equal length, the nearer. Party and raid members are compared by cached name at the fixed distance 2.5e9 (`push 0x4f1502f9`, `0x493b74`). `0x493c8b` makes an equal name stop the search only when it is nearer than the best distance; a farther one falls to the prefix test with the full text as its length, so it loses to an earlier prefix match of equal length that is nearer. `0x493d30` is the object callback: it needs the type mask, applies `0x493e40` to units with the search's mode `[edi+4]` (`0x493da6`) and gets the name from the object's slot `+0x70` with the squared distance.

## Targeting by name

`TargetByName("name" [, exact])` selects the best unit of mask 8 (any unit) in mode 0. `AssistByName("name")` does the
same with mask 0x10 (players), then assists as `AssistUnit` does; with an empty name it assists the current target.
`FollowByName("name" [, exact])` follows the best player, mode 2, or the current target for an empty name. A name that finds
nothing shows `ERR_UNIT_NOT_FOUND` when the text is nonempty, else `ERR_GENERIC_NO_TARGET`.

Addresses: `0x489d60`, `0x489d9d`, `0x489daa`, `0x489dbd`, `0x489dcf`, `0x489c40`, `0x489c82`, `0x489c89`, `0x489ccb`, `0x489d02`, `0x489d17`, `0x489ec0`, `0x489f23`, `0x489f2a`, `0x489f8c`, `0x489fa5`, `0x489fbd`, `0x496720`
Confidence: verified
Proof: one
Test: none

`0x489d9d`, `0x489c82` and `0x489f23` call `0x493aa0` with the masks 8, 0x10 and 0x10, the modes 0, 0 and 2, and the
exact flag from argument 2 for `TargetByName` and `FollowByName` and 0 for `AssistByName`; the distance is `0x7f7fffff`. The
error ids are 0x127 (`0x489dbd`) and 0xb8 (`0x489dcf`).

## Cycling through nearby units

`TargetNearestEnemy([reverse])`, `TargetNearestFriend`, `TargetNearestPartyMember` and `TargetNearestRaidMember` call
`0x493f60(reverse, kind)` with kind 1 to 4. The function builds, once per three seconds of use, a list of the units that
pass the kind's filter and a distance gate, sorts it, and selects the next entry after the current target (the previous
one for `reverse`), wrapping at the end; a list older than one second is rebuilt on wrapping. It does nothing without
the active player or when the protected gate for kind 4 fails.

Addresses: `0x489a80`, `0x489aa0`, `0x489ac0`, `0x489ae0`, `0x493f60`, `0x493f9b`, `0x493fc0`, `0x493fcb`, `0x4940b7`, `0x4940d4`, `0x4940e6`, `0x4940f4`, `0x4941e6`, `0xb4e3ac`, `0xb4e3b0`, `0xb4e3b4`, `0xb4e3b8`, `0xb4b428`, `0xb4b42c`, `0xb4b430`, `0xb4b434`, `0x6f1c10`
Confidence: verified
Proof: one
Test: none

The verbs pass kind 1 to 4 in `edx` and `GetScriptFlagArg(1, 0)` in `ecx`. `0x493fcb` compares the last use `0xb4e3b4` plus
0xbb8 milliseconds with the clock and `0x493fea` the cached kind `0xb4e3b8`. The rebuild at `0x4940b7` zeroes the cursor
`0xb4e3ac`, the build time and the use time and the count, enumerates with `0x494200` (`0x4940d4`), sorts the 16-byte
records with `0x494450` (`0x4940f4`) and stamps the build time. `0x49404c` advances the cursor and `0x494055` wraps it to 0;
`0x494074` rebuilds when the list is a second old. The entry is re-checked with `0x493e40` before `0x493540` runs
(`0x4941e6`); an entry that fails moves on, and a full circle empties the list.

## The candidate filter

A candidate is a unit that the kind accepts. Kind 1 accepts a living unit with health, that the player can attack, whose
dynamic flags `+0x224` lack bit 5, and whose slot `+0xa4` does not answer 7. Kind 2 accepts a unit the player can assist
that has health. Kind 3 accepts a party member other than the player, and kind 4 a party or raid member other than the
player. Any other kind accepts everything.

Addresses: `0x493e40`, `0x493e6c`, `0x493e75`, `0x493e8f`, `0x493ea4`, `0x493eaf`, `0x493eca`, `0x493eed`, `0x493f02`, `0x493f15`, `0x493f2a`, `0x493f50`, `0x605f30`, `0x606980`, `0x6066f0`, `0x4918d0`, `0x4918e0`
Confidence: verified
Proof: one
Test: none

`0x493e59` subtracts 1 from the kind and `0x493e6c` jumps through the four-entry table `0x493f50`; a kind above 4 takes
`0x493f3f`, which answers 1. Kind 1 is `0x493e73` to `0x493ebe`: `IsDeadOrGhost`, health above 0 (`[+0x40]`), `CanAttack`,
bit 5 of `[+0x224]`, and `[slot +0xa4] == 7` which refuses (`0x493eb5`).

## Collecting the candidates

The collector `0x494200` runs for every object. It keeps a unit that passes the cached kind's filter, is drawn, and lies
within a distance of the player: `targetNearestDistance` (default 41) when it is in front, `targetNearestDistanceRadius`
(default 10) when it is not. In front is a heading from the player to the unit within 30 degrees of the player's facing,
either side. The record stored is the GUID, the squared distance as a float, and the in-front flag. Units of a
creature type whose record has bit 0 of `+0x28` set are skipped. The sort puts in-front units first and then ascending
distance. Both options must lie between 1 and 50.

Addresses: `0x494200`, `0x494246`, `0x4942a4`, `0x4942cc`, `0x4942db`, `0x4942ef`, `0x49430a`, `0x494321`, `0x49434e`, `0x494350`, `0x494373`, `0x494391`, `0x4943a0`, `0x494406`, `0x494436`, `0x494450`, `0x494459`, `0x494470`, `0x490720`, `0x490739`, `0x4942ad`, `0x804488`, `0x803568`, `0xb4d8f4`, `0xb4d8fc`, `0xb4e2a8`, `0xb4e27c`, `0x48fd60`, `0x48fda4`, `0x6704c0`, `0x605570`, `0x47f220`
Confidence: verified
Proof: one
Test: none

`0x4942a4` stores the squared distance as a float. `0x4942cc` takes the heading from the player to the unit; the
facing is subtracted, 4 times the constant π (`0x804488`) is added and `0x73f90a` takes the remainder modulo 2π. The cone
is π/6 (`0x803568` times π, stored once in `0xb4d8f4`, guarded by `0xb4e27c`): the angle below it (`0x494321`) or above 2π minus it
(`0x494338`) is in front. The limit is the record's float at `+0x24`, squared (`0x49435d`), and a squared distance above it
rejects (`0x494368`). `0x494391` skips the creature types and `0x4943a0` requires `IsEntityDrawn`. The record is stored at
`0x494433`. The comparator `0x494450` returns −1 when only the first record is in front, 1 when only the second is, then
orders by the float at `+8`, equal giving 0. `0x490720` is both options' change callback: it rejects below 1 or above 50
with the message `TargetNearestDistance must be between 1 and %f`. The defaults are registered at `0x48fd60` and `0x48fda4`.
