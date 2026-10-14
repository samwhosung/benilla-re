# objectlayer: loot rolls

The group-loot rolls: the five server messages that start, announce and end a roll, the list of
open rolls the loot-roll frames show, the client's own vote, the chat lines for each message, and
the master looter's candidate list. The code is LootRoll.cpp, `0x61b130` to `0x61c700`. The
dispatcher `0x5e6010` (`inventory`) hands it the messages; the `GetLootRollItemInfo`,
`RollOnLoot` and the other Lua functions that read the list are `ui`'s.

Roots: `0x61b310`, `0x61b500`, `0x61b870`, `0x61bfa0`, `0x61c550`, `0x61bdf0`, `0x61b290`

## The open rolls

Each roll is a 0x48-byte record on a list at `0xc4dc00`, its first record at `+4` (tag `PendingLootRoll`): the looted
object's GUID `+0x10`, its item slot `+0x18`, the item id `+0x1c`, two random-property words
`+0x20` and `+0x24`, the roller or winner GUID `+0x30`, the roll number `+0x38`, the type `+0x40`
and the countdown `+0x3c`. A roll also holds its id `+0x8`, its expiry time `+0x28`, and three
flags: voted `+0x2c`, queued `+0x39` and closed `+0x3a`. At most four rolls are open at once,
counted at `0xc4dda8`.

Addresses: `0x61b2e0`, `0x61b5f0`, `0x61c6d0`, `0x61b240`, `0x61b290`, `0xc4dc00`, `0xc4dda4`, `0xc4dda8`, `0x42b790`
Confidence: verified
Proof: one
Test: none

The roll id is a client-side serial: `0xc4dda4` is stored in the record and incremented
(`0x61b38c` to `0x61b394`); the server never sees it. `0x61b2e0(id)` finds a record by id and
`0x61b5f0(guid, slot)` by GUID and slot. `0x61b290` signals `CANCEL_LOOT_ROLL` for every record
that is neither queued nor closed, and `0x61b240` frees them all and zeroes the open count; the
world-entry code calls the pair when the world changes (`0x401c2c`).

## Starting a roll

`SMSG_LOOT_START_ROLL` is read in `0x61b310`: the GUID, the slot, the item id, the two property
words and the countdown. The expiry is the time plus the countdown. The roll waits for the item's
cache record, requested with the callback `0x61b460`; once the record is there `0x61b430` signals
`START_LOOT_ROLL(id, countdown)` if fewer than four rolls are open, else marks the roll queued.
When a roll closes, `0x61b760` signals the first queued roll that fits.

Addresses: `0x61b310`, `0x61b38c`, `0x61b394`, `0x61b430`, `0x61b460`, `0x61b45b`, `0x61b760`, `0x61b7be`, `0x55ba30`, `0xc0e2a0`, `0x703f50`
Confidence: verified
Proof: one
Test: none

`0x61b310` allocates the record (`SMemAlloc` 0x48), links it at the head of the list, clears
`+0x2c`, `+0x39` and `+0x3a`, and asks `0x55ba30` on the item cache `0xc0e2a0` for the item
`+0x1c` with the callback `0x61b460`; a record in hand goes straight to `0x61b430`. `0x61b430`
tests the count against 4, increments it, and signals event 0x1f9 with the format `%d%d`
(`0x61b452`); `0x61b45b` marks the roll queued. `0x61b7be` walks the list for queued rolls.

## Voting

`0x61bdf0(id, type, confirm)` is the vote, called by `RollOnLoot` and `ConfirmLootRoll`. A closed
or unknown roll does nothing. A vote other than pass with `confirm` false and an item that binds
when picked up (record `+0x194` equals 1) signals `CONFIRM_LOOT_ROLL(id, type)` and stops. Otherwise it
sends `CMSG_LOOT_ROLL`, 0x2a0: the roll's GUID, slot and the type byte. It sets the voted flag,
signals `CANCEL_LOOT_ROLL(id)`, closes the roll, lowers the open count and promotes a queued one.

Addresses: `0x61bdf0`, `0x61be48`, `0x61be67`, `0x61be70`, `0x61be8b`, `0x61bece`, `0x61bf10`, `0x61bf18`, `0x61bf2a`, `0x61bf32`, `0x61bf40`, `0x61bf46`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`0x61be48` and `0x61be4c` enter the bind test for a nonzero type with `confirm` zero; `0x61be70`
compares the record's `+0x194` with 1 and `0x61be8b` signals event 0x1fb with `%d%d`. The packet
is built from `0x61bece`: the opcode as a dword, the GUID from `+0x10`, the slot from `+0x18` and one
type byte, sent at `0x61bf10`. `0x61bf2a` signals event 0x1fa.

## The roll announcement

`SMSG_LOOT_ROLL` is parsed by `0x61bfa0`: the looted object's GUID, the slot, the roller's GUID,
the item id, the two property words, the roll number byte and the type byte. It waits for the item
record and the roller's name (callbacks `0x61c4b0` and `0x61c3e0`), then `0x61c0b0` writes one line
to chat as type 24 and frees the record, unless `showLootSpam` is 0.

Addresses: `0x61bfa0`, `0x61bfb7`, `0x61c070`, `0x61c3e0`, `0x61c4b0`, `0x61c0b0`, `0x61c0b9`, `0x61c3d2`, `0x55f080`, `0xb4e2bc`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

`0x61c0b9` loads the `showLootSpam` variable at `0xb4e2bc`: a null variable or a nonzero value goes
on, a zero value frees the record unspoken (`0x61c0cc`). The line is chosen by `rollNumber`
as a signed byte and `type`. A negative number (128 or more) is a vote announcement: type 2 reads
`LOOT_ROLL_GREED_SELF` for the active player and `LOOT_ROLL_GREED` for another, any other type
`LOOT_ROLL_PASSED_SELF` or `LOOT_ROLL_PASSED`. A non-negative number with type 0 is a vote for
need, `LOOT_ROLL_NEED_SELF` or `LOOT_ROLL_NEED`. Any other non-negative number is a dice result,
`LOOT_ROLL_ROLLED_GREED` for type 2, else `LOOT_ROLL_ROLLED_NEED`, with no self form.

## Formatting the lines

Every line is the global string for its key, filled by `SStrPrintf` into an 0x800 buffer and added
to chat as type 24 (`CHAT_MSG_LOOT`). The arguments are a colour prefix of the item's quality, the
item id, 0, the two property words, the item link text and a colour suffix; a third-person line
adds the roller's name first, a dice line adds the roll number before the rest.

Addresses: `0x61c116`, `0x61c13f`, `0x61c18f`, `0x61c1a1`, `0x61c33d`, `0x61c365`, `0x61c381`, `0x61c38c`, `0x52ad90`, `0x5d8b00`, `0x703bf0`, `0x64a7f0`, `0x49a870`
Confidence: verified
Proof: one
Test: none

`0x61c116` calls `0x52ad90(quality)` for the prefix; `0x61c13f` `0x5d8b00` for the link text into a
stack buffer. The keys are looked up with `FrameScript_GetText(key, -1)` at `0x61c18f` and the
others; `0x61c33d` sign-extends the roll number for `ROLLED_*`. `0x61c38c` calls the chat sink
with type `0x18`. The dice lines take the same arguments as the others plus the number.

## The result

`SMSG_LOOT_ROLL_WON` (`0x61b870`) reads the GUID, slot, item id, property words, the winner's GUID
and two bytes (the roll number and the type). `0x61b9e0` then writes the winner's line, closes an
open roll, and promotes a queued one. With `showLootSpam` nonzero (`0x61ba43`, `0x61bb07`) the line is `LOOT_ROLL_YOU_WON` or
`LOOT_ROLL_WON`, without the roll number. With the variable 0 or missing, the line is the
`_NO_SPAM_NEED` form when the type is 1, else `_NO_SPAM_GREED`, and carries the number. A null variable is
shown by `0x61c0b0` (`0x61c0c3`) but takes the `_NO_SPAM` line here (`0x61ba41`, `0x61ba8a`).

Addresses: `0x61b870`, `0x61b9a0`, `0x61b9e0`, `0x61bc80`, `0x61bd50`, `0x61ba3a`, `0x61ba94`, `0x61bafe`, `0x61bb52`, `0x61bc12`, `0x61bc6b`, `0x5ebcf0`, `0x61c0c3`, `0x61ba41`, `0x61ba8a`
Confidence: verified
Proof: one
Test: none

`0x61b97c` calls `ClearLootRollState` (`0x5ebcf0`) with the roll's slot and GUID. `0x61ba3a` reads
the variable for the active player's line and `0x61bafe` for another's; `0x61ba94` and `0x61bb52`
compare the type with 1. A roll not yet closed signals `CANCEL_LOOT_ROLL` (`0x61bc12`), lowers the
open count, and `0x61bc6b` promotes. A roll the client has no record of is made closed first.

## Everyone passed

`SMSG_LOOT_ALL_PASSED` (`0x61b500`) reads the GUID, slot, item id and the two property words. It
finds the roll by GUID and slot, or makes a closed one, clears the slot's roll state in the active
player's loot, and `0x61b640` writes `LOOT_ROLL_ALL_PASSED` as chat type 24 with the item's link
and no names, whatever `showLootSpam` says, then closes and promotes like the result does.

Addresses: `0x61b500`, `0x61b5ab`, `0x61b5d2`, `0x61b640`, `0x61b7d0`, `0x61b6f3`, `0x61b73d`
Confidence: verified
Proof: one
Test: none

`0x61b5ab` asks the item cache with the callback `0x61b7d0`, which ends in `0x61b640`; `0x61b5d2`
calls `ClearLootRollState`. `0x61b640` reads no variable. `0x61b6f3` signals `CANCEL_LOOT_ROLL`
only if the roll is not closed.

## The master looter's list

`SMSG_LOOT_MASTER_LIST` (`0x61c550`) clears 40 GUID slots at `0xc4dc38` and reads a count byte and
that many GUIDs. With no party list loaded, the GUIDs fill the slots in order. With a party list,
each GUID is looked up among the members and put into the first empty slot of its subgroup's five,
slots `5·g` to `5·g + 4` for the member's group `g` (the member record `+0x8`); a GUID that is not
a member, or a full group, is dropped. `0x61c660(i)` returns slot `i`, or 0 for 40 or more.

Addresses: `0x61c550`, `0x61c567`, `0x61c598`, `0x61c5c9`, `0x61c5e0`, `0x61c614`, `0x61c630`, `0x61c660`, `0x61c5b9`, `0xc4dc38`, `0xc4dda4`, `0xc4dda8`
Confidence: verified
Proof: one
Test: none

`0x61c567` zeroes the 0x50 dwords with `rep stos`. The party list is the pointer array at
0xb712a8, counted at 0xb713e0; `0x61c5e0` compares each member's GUID, `0x61c614` scans for the
free slot below `0x28`. Slots are indexed by subgroup, so candidates come out grouped by raid group.
With no party list `0x61c5b9` stores each GUID at the count-byte index with no bound, so a count above 40 writes
up to 255 GUIDs past the table: the high dword of index 45 lands on `0xc4dda4` and the low dword of index 46 on `0xc4dda8`.
