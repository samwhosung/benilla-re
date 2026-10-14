# ui: selection laws

What changing the selected target does to the player's swing and to the wire, what clears the selection, and the laws of the
name match, the nearest-unit cycle and the last-target pairs. The setter, the verbs and the candidate collector are in
`targeting`; the unit-side tests in `objectlayer/units` and `objectlayer/reaction`; the melee start in
`objectlayer/playercombat`. The selection is the pair `0xb4e2d8`, the last target `0xb4e2e0`, the last attackable target
`0xb4e2e8`.

Roots: `0x493540`, `0x493910`, `0x489a40`, `0x489b00`, `0x489b40`, `0x493f60`, `0x493e40`, `0x493aa0`

## Selecting stops the swing

Every selection change and clear first runs the teardown `0x493910` on the old selection, which stops the active player's
swing, so a unit-frame click and a world click cost the same packets. Re-selecting the current unit does nothing at all.
The table is for a player swinging at a valid unit; without a swing the stops are not sent.

| new selection | packets, in order |
|---|---|
| an attackable unit | `CMSG_ATTACKSTOP`, `CMSG_SET_SELECTION`, `CMSG_ATTACKSWING` |
| a unit the attack test refuses | `CMSG_ATTACKSTOP`, `CMSG_SET_SELECTION`, `CMSG_ATTACKSTOP` |
| the player, or a member with no object | `CMSG_ATTACKSTOP`, `CMSG_SET_SELECTION` |
| nothing (a clear) | `CMSG_ATTACKSTOP`, then `CMSG_SET_SELECTION` with GUID 0 |
| the same unit, or an unresolved token | none |

Addresses: `0x493540`, `0x493617`, `0x4936cc`, `0x4936e2`, `0x493857`, `0x4938a1`, `0x4938c8`, `0x4938f8`, `0x493910`, `0x493a08`, `0x493a2a`, `0x5ecac0`, `0x5ecb70`, `0x5ecc2e`, `0x5ecc48`, `0x624370`, `0x60ecb0`, `0x489a40`
Confidence: verified
Proof: one
Test: none

The teardown runs only when a selection exists; it closes the open interaction if that was the unit, clears the
highlight light bit (`0x6144f0`), dirties the name colour (`0x6c77f0`), clears token flag 0x1000, zeroes the pair (`0x4939d3`), calls `StopAttack` on the player (`0x493a08`), the pet stop
when a pet is out (`0x493a0f`, `0x4bd650`), and sends the guid-0 selection only for a clear (`0x493a1d`, flag `ecx`).
`StopAttack` returns unless the attack lock `+0xc48` or `+0xc50` is held (`0x5ecae4`); it sends `0x142` through `0x624370`,
which sets `+0xc54` to 1. Nothing in those functions clears `+0xc48`, so `0x60ecb0` is still true when `StartAttack` refuses
the new target at `0x5ecc2e`, and its refusal arm at `0x5ecc37` stops again. A same-GUID call returns at `0x493617`.

## The re-swing test

The swing restarts only when the old target was a valid one to be swinging at, judged before the new GUID is stored, and the
new GUID is not the player's own. The restart then lets `StartAttack` judge the new target, which can refuse it.

Addresses: `0x493622`, `0x49362d`, `0x493637`, `0x493639`, `0x49365f`, `0x49366c`, `0x49367d`, `0x49368d`, `0x4936a3`, `0x4936ac`, `0x4938a1`, `0x4938ae`, `0x4938b8`, `0x4938c3`, `0x606980`, `0x605f30`
Confidence: verified
Proof: one
Test: none

The latch `[ebp-1]`, zero at `0x49362d`, is set at `0x4936ac` when all hold, read from the old selection because
`0xb4e2d8` is stored later (`0x4936e2`): the player resolved (`0x493620`), `0x60ecb0` (already attacking, `0x493639`), the old
selection resolves as a unit (`0x49365f`), the player is not dead or a ghost (`0x49366c`), not mounted (`[+0x110]+0x1fc` at
most 0, `0x49367d`), the old target has health or dynamic flag 0x20 (`0x49368d`), and `0x606980` allows the attack
(`0x4936a3`). `0x4938a1` tests the latch and `0x4938ae`, `0x4938b8` compare the new GUID with the player's own to skip
`StartAttack` (`0x4938c8`).

## Clearing the selection

Two instruction pairs write the selection: the setter's `0x4936e2`, `0x4936e8` and the teardown's `0x4939d3`, `0x4939d9`.
Callers of the setter replace or clear it when the target dies (`0x605901`), when the player dies (`0x5e9a45`), for
`ClearTarget` (`0x48a00e`), for a ground click that no action takes with `deselectOnClick` on (`0x492cce`), when an interaction opens on a
unit (`0x48f3be`, `SetInteract`), and when a struck player with no selection takes its attacker (`0x6259fe`). The teardown clears it when the
object is deactivated, whether destroyed or out of range, when the roster does not hold it, and when an interaction
closes on a dead unit.

Addresses: `0x4936e2`, `0x4939d3`, `0x605901`, `0x5e9a45`, `0x48a00e`, `0x492cce`, `0x48f3be`, `0x6259fe`, `0x48f369`, `0x4baab8`, `0x4baad9`, `0x4e845a`, `0x4e8558`, `0x4e8587`, `0x5e2719`, `0x5e6df7`, `0x5fbbe5`, `0x5fbb60`, `0x4674a0`, `0x6046f0`
Confidence: verified
Proof: one
Test: none

The census of `0x493540` is 20 calls (`e8`, no jump, no stored address), of `0x493910` 11. The health watcher `0x6046f0` calls
the death handler `0x605860` on the edge from health above 0 to 0 or less, and that handler compares the dying unit's GUID
with the selection (`0x6058ea`) and clears it through `0x493540(0, 0)`, which reaches `0x493910` with the send flag. The
deactivation `0x5fbb60` (unit slot 1) calls the teardown at `0x5fbbe5`; `0x4674a0` reaches it through the remove. `deselectOnClick`
defaults to "1".

## Name match order

`TargetByName`, `AssistByName` and `FollowByName` take the first whole-string match the walk reaches, never the nearest, and
otherwise the longest shared prefix. The walk visits the party slots, then the raid slots, then every object the client
holds in the order the manager linked it, the longest linked first. A whole-string match farther than a prefix match
already kept does not win; an equal prefix goes to the nearer unit, and a tie keeps the first. The player is a candidate.

Addresses: `0x493aa0`, `0x493b20`, `0x493b46`, `0x493b6b`, `0x493b9d`, `0x493c2c`, `0x493c60`, `0x493c7f`, `0x493c88`, `0x493cab`, `0x493cf8`, `0x493d0b`, `0x493d30`, `0x468380`, `0x4650c2`, `0x55f080`
Confidence: verified
Proof: one
Test: none

`0x493c60` compares the whole strings case-insensitively (`0x64a4c0`); on a match with a score strictly under the best
(`test ah,5; jp`, `0x493c88`) it stores the GUID and returns 0, which stops the walk (`0x468380` tests the result). A match
that is not under the best falls to the prefix leg: a longer common prefix takes the slot, an equal one needs a strictly
smaller squared distance. Party and raid members without an object are scored by name from the name cache at
2 500 000 000 (`0x4f1502f9`). The walk order is the manager's list (`objectlayer/manager`), where an object streamed out and in is linked again at the tail. With `/target Bob`,
a nearer "Bobby" met first beats a farther "Bob". The by-name verbs never apply a range, a cone or a scene test.

## The by-name modes

The verb's mode decides the per-candidate filter. `TargetByName` and `AssistByName` pass mode 0, which accepts every unit,
the dead, the hostile, the player itself. `FollowByName` passes mode 2, which accepts a unit the player may assist and
that has health. The masks differ: `TargetByName` takes any unit (8), the other two players (0x10), so `/assist` and
`/follow` cannot find a creature by name.

Addresses: `0x489d60`, `0x489d9d`, `0x489c40`, `0x489c82`, `0x489ec0`, `0x489f23`, `0x493d30`, `0x493da6`, `0x493dae`, `0x493e40`, `0x493e59`, `0x493e66`, `0x493eca`, `0x493f3f`
Confidence: verified
Proof: one
Test: none

`0x493d30` loads the mode from the search record (`[edi+4]`, `0x493da6`) and passes it to `0x493e40`, which subtracts 1 and
sends a result above 3 to `0x493f3f`, returning 1 (`0x493e5c` to `0x493e66`); mode 0 gives `0xffffffff`. The record's
mode comes from the verb: push 0 for target and assist, push 2 for follow. Mode 2 is `0x493eca`: `0x6066f0`, then health
above 0. A name that matches nothing leaves the target alone and shows message 0x127 (named) or 0xb8 (empty).

## Filters of the cycle

The cycle keys of one function share the filter `0x493e40` by kind: 1 the enemies, 2 the friends, 3 the party, 4 the party
or raid. Kinds 3 and 4 refuse the player's own object; kinds 1 and 2 do not test for it.

Addresses: `0x493e40`, `0x493e73`, `0x493eca`, `0x493eed`, `0x493f15`, `0x4918d0`, `0x4918e0`, `0x4e7f70`, `0x4baee0`, `0x605f30`, `0x606980`, `0x6066f0`, `0x60be50`
Confidence: verified
Proof: one
Test: none

Kind 1 (`0x493e73`) refuses a dead or ghost unit (`0x605f30`), health at most 0, a unit the player cannot attack
(`0x606980`), dynamic flag 0x20 and a stand state of 7 (slot `+0xa4`). Kind 2 (`0x493eca`) is `0x6066f0` and health above 0
and nothing else. Kind 3 (`0x493eed`) refuses the candidate that is the ctx object, then needs `0x4e7f70` (the player's GUID or
a slot of the party array `0xbc6f48`, four slots of 8, bound 0x20). Kind 4 (`0x493f15`) is the same with `0x4918e0`, which adds
the raid array `0xb712a8` bounded by `0xb713e0` (`0x4baee0`).

## The player in the cycle

`TargetNearestFriend` selects the player itself when its facing lies within π/6 of π/2, and otherwise when no other friend
stands in the forward cone. Plain `TargetNearestEnemy` never does, because the player cannot attack itself.

Addresses: `0x494200`, `0x49424b`, `0x4942a4`, `0x4942cc`, `0x4942db`, `0x4942ef`, `0x49431b`, `0x494350`, `0x494459`, `0x47f220`, `0x47f23e`, `0x47f245`, `0x47f262`, `0x6061e0`, `0x606207`, `0x606980`, `0x606b89`, `0x468380`, `0x468460`, `0x493540`, `0x6066f0`
Confidence: verified
Proof: one
Test: none

The walk yields every object the client holds, the player's included, and `0x468460` with mask 8 admits it (the player's type
word is 0x19). `0x494200` has no compare against the player and no `0x468550` call. The friend filter accepts it:
`0x6061e0` returns 4 for a unit toward itself (`0x606207`), so `0x6066f0` is true, and its squared distance is 0. For equal
positions `0x47f220` takes the axis arm: dx under 2^-22 and dy equal to 0.0 give π/2 (`0x47f262`). The cone is
`wrapped < π/6` or above 2π − π/6 of `bearing − facing + 4π` taken modulo 2π, so the self entry is in the cone for facings in
(π/3, 2π/3). The creature-type reject (`0x494391`, bit 0 of record `+0x28`) is set in the shipped records for Critter alone, and
every player race is humanoid, so it does not fire. In-cone sorts first (`0x494459`), then distance. `0x606980` refuses it for kind 1 through the reaction test
`< 4` (`0x606b89`).

## The last attackable target

`TargetLastEnemy` (key G in the shipped default bindings; TAB, SHIFT-TAB, CTRL-TAB and CTRL-SHIFT-TAB are the cycle) reselects
the last unit the player selected while it could have attacked it; the memory outlives a deselect, a death and a zone. An empty or unresolvable memory does nothing, unlike `TargetLastTarget`, which with an empty
memory deselects.

Addresses: `0x49372f`, `0x493732`, `0x493736`, `0x49374e`, `0x49375b`, `0x493771`, `0x49377d`, `0x493783`, `0x489b40`, `0x489b00`, `0x489b2d`, `0x489a40`, `0x489a59`, `0x489a64`, `0x491900`, `0xb4e2e8`
Confidence: verified
Proof: one
Test: none

`0x49377d`, `0x493783` write the pair from the incoming GUID, under the block `0x49372f` to `0x493778`: the player resolved,
not dead or a ghost (`0x493736`), not mounted (`0x49374e`), the new target with health or dynamic flag 0x20 (`0x49375b`) and
`0x606980` true (`0x493771`), inside the unit-type guard of `0x493712`. The pair has these two writers and a read-only
overlay; nothing clears it. `0x489b40` passes the pair's address to `0x489a40` and nothing else; with a zero GUID all three
arms of `0x489a40` fall to `0x489a79`, and `0x489b00` tests its own pair and calls `0x493540(0, 0)` when it is zero
(`0x489b2d`).

## The cycle state

The four cycle keys share one candidate list, one cursor and one cached kind, so alternating them restarts each cycle; a
press after more than three seconds of rest rebuilds the list and selects slot 0 even when that is the current target,
which changes nothing.

Addresses: `0x493f60`, `0x493f96`, `0x493fa0`, `0x493fc0`, `0x493fe7`, `0x493ff6`, `0x49400d`, `0x49404c`, `0x4940ac`, `0x4940bc`, `0x4940f4`, `0x494126`, `0x494170`, `0x4941e6`, `0x494a50`
Confidence: verified
Proof: one
Test: none

The rebuild test (`0x493fa8` to `0x493ff0`) is any of: the build time or the use time 0, the use time plus 3000 ms
not after now, an empty list, a different kind (`0x493fea`). It zeroes the cursor, both times and the count (`0x4940bc`),
walks with `0x494200` and sorts with `0x494450`. Only the reuse path steps the cursor past the current selection
(`0x49400d` compares, `0x49404c` advances); the scan commits the first entry from the cursor that passes `0x493e40`
(`0x494170`) and never compares it with the current selection, so after a rebuild the commit repeats the selection and
`0x493617` ends it. `0x494a50` is called with 4 here (`0x493f96`) and with 1 or 3 in the setter: arguments 1, 3, 5 to 9
return 1, and 2, 4, 10 test three UI singletons.

## Acquiring a target to attack

The attack resolver `0x612df0` picks the nearest enemy when it has no usable target: with no target, or a selected one
toward which the player's reaction is 4 or more, it calls the cycle core with kind 1, and with none found shows message
0xa0 and stops. A struck player with no selection selects the attacker and does not swing back.

Addresses: `0x612df0`, `0x613069`, `0x61307f`, `0x6130a3`, `0x6130b5`, `0x6130bc`, `0x6130d9`, `0x6130f3`, `0x613100`, `0x6259fe`, `0x6259e6`, `0x493f60`
Confidence: verified
Proof: one
Test: none

`0x613069` tests the argument GUID, `0x61307f` the selection; a unit whose reaction `0x6061e0` is at least 4 (`0x6130a3`)
zeroes the GUID, and `0x6130bc` calls `0x493f60` with `ecx` 0 and `edx` 1. A named target that is not friendly is
selected by `0x613100`. `0x6259fe` selects the attacker when the packet's victim is the active player and the selection is
empty (`0x6259e6`); `unitcombat` holds the packet.

## Assist and follow

`AssistUnit` and `AssistByName` select the target of the named unit (`UNIT_FIELD_TARGET` at `+0x28`) and, with `assistAttack`
nonzero, start the swing on it; `assistAttack` is "0" by default. The assisted unit with no target does nothing. Neither verb
tests `CanAssist`. `FollowUnit` and `FollowByName` send no packet; they start the follow auto-move (mode 3).

Addresses: `0x489b80`, `0x489bb2`, `0x489bc6`, `0x489bcb`, `0x489bd0`, `0x489c02`, `0x489c40`, `0x489cc2`, `0x489d02`, `0x489e00`, `0x489ec0`, `0x48fc50`, `0x48fc67`, `0xb4d8f8`, `0x5ecb70`, `0x60fed0`
Confidence: verified
Proof: one
Test: none

`0x48fc50` registers `assistAttack` with the default string `0x82e570`, "0", the pushes before the call; the store at
`0x48fc67` belongs to that call, since each store is scheduled into the next call's arguments. `0xb4d8f8` is read at `0x489bd0`
and `0x489cd0` only. `0x489bb2` loads the field from the basis unit, `0x489bc6` returns on 0, `0x489bcb` selects through
`0x489a40`, and `0x489c02`, `0x489d02` call `StartAttack`. The census of `call 0x6066f0` (25 sites) has none in the verbs. The
follow verbs and `0x60fed0`, `0x611130`, `0x60fb60` hold no packet send.
