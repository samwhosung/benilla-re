# ui: master loot and the loot method

The master looter's dropdown functions, the packet that gives an item, and the group's loot method, master looter and
threshold: where they are stored, how they are set and read. This is the master loot half of `LootFrame.cpp`
(`0x4c2f10` to `0x4c3050`) and the loot method functions of `Party.cpp` (`0x4e86f0` to `0x4e95d0`). The loot window is
`ui/loot`; the candidate list the server sends is `objectlayer/lootroll#the-master-looters-list`; the group roster
packet that carries the method is `objectlayer/party`.

Roots: `0x4c2f10`, `0x4c2fd0`, `0x4e86f0`, `0x4e91b0`, `0x4e92a0`, `0x4e94e0`, `0x4e9500`

## The master loot events

The client has two master loot events, `OPEN_MASTER_LOOT_LIST` (503) and `UPDATE_MASTER_LOOT_LIST` (504), each fired
from one place. A click on a loot row whose wire loot type is 2 signals 503 with no arguments and sends nothing
(`ui/loot#taking-a-slot`); nothing else on the path compares a GUID, so the client opens the dropdown for any row the
server marked type 2, whoever the player is. Event 504 is raised only by the callback `0x4c2fb0`, which a candidate name
lookup registers: it fires when a name that was not cached has arrived. The candidate list packet itself fires nothing.

Addresses: `0x4c28d1`, `0x4c2fb0`, `0x4c2fb6`, `0x4c2fbf`, `0x4c2f65`, `0x4c2790`, `0x51b955`, `0x51b95f`
Confidence: verified
Proof: one
Test: none

The event names are stored by static initializers: `0x51b955` writes the string `OPEN_MASTER_LOOT_LIST` to the slot of
id 503 and `0x51b95f` `UPDATE_MASTER_LOOT_LIST` to 504, in the table of 549 names from `0xbe1198`. `0x4c28d1` is a tail jump
(`e9`) to `0x703e50`. `0x4c2fb0` is `mov al,[ebp+0xc]; test al,al; je`, then `mov ecx,0x1f8` and the call. It has no caller;
its one stored address is `push 0x4c2fb0` at `0x4c2f65`.

## Candidates

`GetMasterLootCandidate(index)` takes a number (else `Usage: GetMasterLootCandidate(index)`), subtracts 1 and asks
`0x61c660` for the GUID at that index of the 40-entry candidate array. The array is dense in wire order outside a raid
and keyed by subgroup (five slots per group) in a raid, so a walk to the first nil ends at the first gap. A zero GUID
returns nil. Otherwise it asks the name cache; a cached name is returned, and a missing name returns nil while
registering `0x4c2fb0` to raise 504 when the name arrives. It always returns one value, and checks no range itself.

Addresses: `0x4c2f10`, `0x4c2f27`, `0x4c2f4d`, `0x4c2f50`, `0x4c2f5f`, `0x4c2f70`, `0x4c2f75`, `0x4c2f7c`, `0x4c2f82`, `0x4c2f91`, `0x61c660`, `0x55f080`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

`0x4c2f4d dec eax` turns the index into the array index handed to `0x61c660`, which bounds it below 40 and answers 0
outside. `0x4c2f70` calls the name cache `0xc0e228` with the callback; `0x4c2f7c je 0x4c2f91` pushes nil for an uncached
name; both exits set `eax` to 1.

## Giving the item

`GiveMasterLoot(slot, index)` takes two numbers (`Usage: GiveMasterLoot(slot, index)`), both 1-based, finds the
candidate GUID of the index and does nothing if it is 0; otherwise it calls the worker `0x4c2940` and returns nothing.
The worker needs an open window, applies the coin-row shift, bounds the slot below 16, needs an occupied row and a
wire loot type of 2, and then sends `CMSG_LOOT_MASTER_GIVE` (0x2a3): the loot source GUID (u64), the wire slot byte
(u8) and the candidate GUID (u64). Every failed check returns without a message.

Addresses: `0x4c2fd0`, `0x4c3029`, `0x4c3033`, `0x4c2940`, `0x4c2959`, `0x4c2965`, `0x4c298b`, `0x4c29c3`, `0x4c29d0`, `0x4c29e8`, `0x4c29f7`, `0x4c2a0d`, `0x5ebce0`
Confidence: verified
Proof: one
Test: none

`0x4c3003` and `0x4c3017` subtract 1 from both arguments, `0x4c3021 or ecx,edx; je` skips a zero candidate, and the worker
has one caller (`0x4c3029`). `0x4c2965 cmp ecx,0x10; jae` is unsigned, so slot 1 with money, which becomes -1, is
refused. `0x4c298b cmp eax,2` repeats the type test. The packet is `push 0x2a3`, the stored GUID by `0x418370`, the byte
`[esi+0xb71980]` by `0x418070` and the candidate by `0x418370`; `0x4c2a0d` sends.

## The group's loot settings

Three cells hold the group's loot setup: the method `0xbc7600` (0 free for all, 1 round robin, 2 master, 3 group, 4
need before greed), the master looter's GUID `0xbc7608` (u64) and the threshold `0xbc7610`. `0x4e86f0(method, threshold,
looter)` stores them, called from the group list packet whenever it carries members. For each value that differs from
the stored one it first shows a message: the method, one of `ERR_SET_LOOT_FREEFORALL`, `_ROUNDROBIN`, `_MASTER`,
`_GROUP` and `_NBG` (ids 0xe6 to 0xea, in method order); a new nonzero looter, when the method is 2 and the name is
cached, `ERR_NEW_LOOT_MASTER_S` (0xec); the threshold, `ERR_SET_LOOT_THRESHOLD_S` (0xeb) with the text of the global
`ITEM_QUALITY<n>_DESC`. Then, on every call, it signals `PARTY_LOOT_METHOD_CHANGED` (222) with no arguments.

Addresses: `0x4e86f0`, `0x4e8709`, `0x4e871c`, `0x4e8724`, `0x4e880c`, `0x4e8773`, `0x4e878f`, `0x4e87a6`, `0x4e87ac`, `0x4e87b5`, `0x4e87c9`, `0x4e87ef`, `0x4e87f5`, `0x4e87fa`, `0xbc7600`, `0xbc7608`, `0xbc7610`
Confidence: verified
Proof: one
Test: none

`0x4e8709` compares the method with the stored one and `0x4e8724` jumps through the five-entry table `0x4e880c` to
`push 0xe6` to `push 0xea`; a method above 4 is stored with no message. `0x4e8773 cmp [ebp-0xc],2` gates the looter message.
`0x4e87c9` formats `ITEM_QUALITY%d_DESC` and `0x4e87d9` reads that global with `0x703bf0`. The stored words are zero
extended wire bytes. The roster packet's tail is a u8 method, a u64 looter, a u8 threshold and a u8 dungeon difficulty,
which is read and dropped (`objectlayer/party`).

## Reading the loot settings

`GetLootMethod()` returns two values: the method name as a string (`freeforall`, `roundrobin`, `master`, `group`,
`needbeforegreed`, or `ERROR!` above 4) and the master looter's party index, nil with no looter. The index is 0 when the
looter is the player and otherwise 1 to 4 for the matching slot of the four party GUIDs; a looter outside the player's
party, including a raid member, gives nil. There is no third return. `GetLootThreshold()` returns the stored threshold
as a number and ignores its arguments.

Addresses: `0x4e91b0`, `0x4e91c5`, `0x4e9280`, `0x4e9209`, `0x4e9210`, `0x4e9215`, `0x4e9223`, `0x4e9236`, `0x4e9247`, `0x4e925f`, `0x4e94e0`, `0x4e81a0`
Confidence: verified
Proof: one
Test: none

Every exit sets `eax` to 2 (`0x4e922a`, `0x4e9255`, `0x4e927a`). The table `0x4e9280` selects the string; `0x4e9215`
and `0x4e921b` compare the looter GUID with the player's `0x468550`; `0x4e9238` calls the party GUID getter `0x4e81a0`
for slots 0 to 3. `0x4e94e0` is `fild [0xbc7610]` and the number push, with no prologue.

## Setting the loot method

`SetLootMethod("method" [, master] [, threshold])` and `SetLootThreshold(threshold)` need the player to be in a group
(`ERR_NOT_IN_GROUP`) and its leader (`ERR_NOT_LEADER`); each failure returns nothing. The method string, matched without
case, is `freeforall`, `roundrobin`, `master`, `group` or `needbeforegreed`, else `Invalid loot method`. Only for
`master` is the next argument read: a player name, resolved by closest match, who must be in the player's party or raid
(`ERR_TARGET_NOT_IN_GROUP_S`); an empty or missing name gives `ERR_SPECIFY_MASTER_LOOTER`. The threshold is the argument
after that and defaults to 2. Both send `CMSG_LOOT_METHOD` (0x7a): method (u32), master looter GUID (u64), threshold
(u32); `SetLootThreshold` sends the current method and looter with the new threshold. A non-number threshold raises
`Usage: SetLooThreshold(threshold)`.

Addresses: `0x4e92a0`, `0x4e9500`, `0x4e92c1`, `0x4e92ed`, `0x4e9340`, `0x4e9370`, `0x4e93cf`, `0x4e93e3`, `0x4e9418`, `0x4e942a`, `0x4e9436`, `0x4e9440`, `0x4e9453`, `0x4e946d`, `0x4e94b9`, `0x4e94c7`, `0x4e95c2`, `0x5e97e0`, `0x493aa0`, `0x4e7f70`, `0x4baee0`
Confidence: verified
Proof: one
Test: none

The group test reads the first party GUID (`0x4e92a6`) and the raid count (`0x4e92b8`); the leader test compares the group
leader GUID with `0x468550` (`0x4e92d4` to `0x4e92eb`). The strings are compared in a chain with `0x64a4c0`
(`0x4e9340`, `0x4e9359`, `0x4e9376`, `0x4e9393`, `0x4e93b0`) storing 0 to 4. `0x4e93cf cmp eax,edi` with `edi` 2 selects
`master`, which moves the threshold's argument index to 3 (`0x4e93e3`). `0x5e97e0` writes `push 0x7a`, the method by `0x4181f0`,
the GUID by `0x418370` and the threshold by `0x418190`. 

## No master looter check

No loot window code reads the three loot setting cells: every reference to them lies between `0x4e86f0` and `0x4e95c7`.
The window opens the dropdown and gives an item on the wire loot type alone, and the only test of whether the player
is the master looter is the index `GetLootMethod` returns.

Addresses: `0x4e86f0`, `0x4e95c7`, `0x4e8709`, `0x4e91b6`, `0x4e95b8`, `0x4c2790`
Confidence: verified
Proof: one
Test: none

The references to `0xbc7600` are at `0x4e8709`, `0x4e871c`, `0x4e91b6` and `0x4e95b8`; to `0xbc7608` at `0x4e8754`, `0x4e87a6`,
`0x4e91fb` and `0x4e95b0`; to `0xbc7610` at `0x4e87b5`, `0x4e87ef` and `0x4e94e0`. The same scan finds the loot source GUID
`0xb71b48` 25 times inside the loot window code.
