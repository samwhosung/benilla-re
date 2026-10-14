# ui: trying items on the dressing room model

What the `DressUpModel` pane does beyond the copy of the unit's model: the extra setup it runs on the copy, `TryOn(item)`,
`Undress()` and `Dress()`. The class, vtable and the copy itself are `ui/playermodel`; the character component calls
(`0x478960`, `0x478aa0`, `0x47a0c0`, `0x47a310`) belong to the character model system.

Roots: `0x504350`, `0x504490`, `0x504540`, `0x504bc0`, `0x504c00`, `0x504cd0`, `0x504d90`

## Pane setup

When the pane receives its model (slot 36, `0x504350`), it drops its old character component, clones the unit's component
for the new model, and records what the unit holds: the visible main-hand item record at `+0x3fc` and the off-hand record at
`+0x404`, each eight bytes (item class, subclass, inventory type at byte 3, sheathe type at byte 4; a first byte of 0 means
the lane is empty), and a ready byte at `+0x40c`. The ranged slot is not recorded. The component pointer is `+0x3f8`.

Addresses: `0x504350`, `0x50435c`, `0x504361`, `0x504382`, `0x5043ab`, `0x5043b5`, `0x5043d5`, `0x5043e2`, `0x504414`, `0x50443e`, `0x504455`, `0x475fb0`, `0x476cb0`, `0x476000`, `0x468460`
Confidence: verified
Proof: one
Test: none

The clone `0x476cb0(component, model, source component)` runs only when the unit has a component (`[unit+0xd30]`).
The two item records come from the unit's virtual `+0x98` accessor with slot 0 then slot 1 (`0x5043e2`, `0x504414`); a
null result zeroes the lane. The ready byte is `[[unit+0x110]+0x1f8] == [[unit+0x110]+0x1f4]` (`0x504450`).

## Trying on an item

`TryOn(itemId)` looks the item up in the item cache and does nothing while the record is missing (the cache calls
`0x504ba0` back to try again), when the record has display id 0, or while the pane has no model. It then acts on the
record's inventory type:

| inventory type | action |
|---|---|
| 1 | clear and set component slot 0 |
| 3, 4, 5 and 20, 6, 7, 8, 9, 10 | component slots 1, 2, 3, 4, 5, 6, 7, 8 |
| 16 cloak | component slot 0xa |
| 19 tabard | component slot 9, with the guild emblem |
| 13, 14, 15, 17, 21, 22, 23, 25, 26 | the held-item lanes |
| 2, 11, 12, 28 | nothing |
| 18, 24, 27 and above 28 | `ERR_NOT_EQUIPPABLE`, nothing else |

Addresses: `0x504540`, `0x504574`, `0x50458b`, `0x504596`, `0x5045a4`, `0x5045b1`, `0x5045b8`, `0x5045bf`, `0x5045c5`, `0x5045cd`, `0x5045dd`, `0x5045ee`, `0x5045fe`, `0x504af6`, `0x504b2d`, `0x504b37`, `0x504ba0`, `0x478960`, `0x478aa0`, `0x504d90`, `0x496720`
Confidence: verified
Proof: one
Test: none

The dispatch is `dec eax; cmp eax,0x1b; ja` then a byte map and a jump table that follow the function (`0x5045b1`,
`0x5045b8`). Every armor and cloak arm needs the component (`[this+0x3f8]`) and the ready byte, else it ends in the
silent return (`0x504b37`); the clear is `0x478960(slot)` and the set `0x478aa0(slot, display id)` (`0x5045dd`,
`0x5045ee`). The held arms need neither. `0x504d90` is the Lua method; it passes the number argument to `0x504540`.

## Trying on a tabard

The tabard arm sets component slot 9 like the others, then, when the display record's flag bit 0 is set and the pane's
unit resolves to a player, reads that player's guild id (`[player+0xe68]+0xc`), asks the guild emblem cache for the
emblem (`0x6d6d20`) and, if it has the five values, installs them on the component (`0x47a610`). An unresolved player,
a record without the flag or a missing emblem leaves the plain tabard.

Addresses: `0x5047f6`, `0x504812`, `0x504825`, `0x50482a`, `0x504835`, `0x504852`, `0x504879`, `0x504886`, `0x504894`, `0x5048b1`, `0x5048d8`, `0x6d6d20`, `0x47a610`
Confidence: verified
Proof: one
Test: none

`0x50482a` loads the display id and `0x504841` indexes the display info table by it with the bounds at `0xc0dc14`;
`0x504852 test [edi+0x24],1` is the flag. The player is found by the pane's stored GUID with the player mask 0x10
(`0x504879`); the guild id is the player block's word at `+0xc` (`0x504894`). The emblem call has its five outputs in
`[ebp-0x14]` to `[ebp-0xc]` and passes them to `0x47a610` (`0x5048be` to `0x5048d8`).

## Held items

A held item goes to the main-hand lane (bodyslot 0xf) or the off-hand lane (0x10). A one-hand weapon takes the main
hand, or the off hand when the main hand holds a one-hand weapon and dual wield is allowed. Two-hand, main-hand, thrown
and right-hand ranged types take the main hand; shields, bows and other ranged weapons (type 15), holdables and off-hand
weapons the off hand. Before the install, an incompatible item in the other lane is detached, then the target lane's own
occupant, and the new record is stored in the lane. The weapon is always installed at the hand, never a sheath point.

Addresses: `0x5048e8`, `0x5048ef`, `0x504953`, `0x50495f`, `0x5049b1`, `0x5049e4`, `0x504a0c`, `0x504a2c`, `0x504a4f`, `0x504a85`, `0x504a8d`, `0x504ade`, `0x504ae6`, `0x5eab70`, `0x504bc0`, `0x47a310`, `0x47a0c0`
Confidence: verified
Proof: one
Test: none

The compatibility test `0x504bc0(mainType, offType, dualWield)` is true only when the main-hand side is type 13 or 21
and the off-hand side is type 14 or 23, or type 13 or 22 with dual wield allowed; types 15, 25 and 26 appear on neither
side, so a ranged weapon always evicts the other lane. The dual-wield flag is true when the pane shows the local player and `0x5eab70` (the dual-wield effect is known) is true.
The install pushes sheathe flag 0 (`0x504ade`), so the weapon is never placed at a sheath point. The detached
lane's record is not cleared (`0x504a0c`, `0x504a4f`).

## Undress and Dress

`Undress()` clears component slots 0 to 11, detaches both lane items and zeroes both lane records. `Dress()` runs the
`SetUnit` worker again with the pane's stored GUID (`+0x3e0`), so it restores the unit's own appearance.

Addresses: `0x504490`, `0x5044b0`, `0x5044b7`, `0x5044ec`, `0x5044f3`, `0x504529`, `0x504c00`, `0x504cd0`, `0x504d6f`, `0x505b50`
Confidence: verified
Proof: one
Test: none

The clear loop `0x5044b0` runs for slots 0 up to 11 (`cmp edi,0xc`). The detach calls are `0x47a310` with bodyslot 0xf and
0x10 and the lane's sheathe type; the stores after them zero the lane (`0x5044f3`, `0x504530`). `0x504cd0` loads the
GUID words from `+0x3e0` and `+0x3e4` (`0x504d6f`) and calls `0x505b50`.
