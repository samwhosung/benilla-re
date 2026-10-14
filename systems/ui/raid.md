# ui: the raid roster, ready check and raid targets

`RaidInfo.cpp` keeps the raid roster, answers the ready check opcode and the raid target opcode, and registers 16 Lua
functions (`tables/bindings.tsv`). The roster is rebuilt by the group list message (`objectlayer/party#the-group-list`); the leader
cell and the party side are `party`. This doc covers the roster record, the three query functions, the ready check
and the raid targets.

Roots: `0x4ba0b0`, `0x4ba220`, `0x4ba360`, `0x4bb1d0`, `0x4bb310`, `0x4bbec0`

## The roster record

The roster is 40 pointers at `0xb712a8` to records of 0x160 bytes, allocated at start-up, with the member count in `0xb713e0`. A record
holds: `+0` the GUID, `+8` the 0-based subgroup (the member's flag dword AND 0x7f), `+0xc` the rank (2 for the group leader's GUID, 1 when the
sign bit of the flag byte is set, else 0), `+0x10` a 0x148-byte member block of the party record kind, and `+0x158` the ready check word. In the block `+0x18` is
the status byte (bit 0 online, bit 2 dead), `+0x22` the level, `+0x24` the zone and `+0x90` the pet GUID. Bit 0 of `+0x18` comes from the group list's per-member
online byte.

Addresses: `0x4ba0b0`, `0x4ba0c0`, `0x4ba0d0`, `0x4ba108`, `0x4ba114`, `0x4ba125`, `0x4ba8d3`, `0x4ba8ea`, `0x4ba90c`, `0x4ba91c`, `0x4ba920`, `0x4ba945`, `0x4ba958`, `0x4ba960`, `0x4ba5f0`, `0xb712a8`, `0xb713e0`
Confidence: verified
Proof: one
Test: none

`0x4ba0b0` registers the target opcode 0x321 and the ready check opcode 0x322, zeroes the target table and allocates the records with
`SMemAlloc(0x160, ...)`: `cmp esi,0xa0` at 4 bytes a step gives 40. `0x4ba5f0` copies the 0x148-byte block into `+0x10` (`rep movs`, 0x52 dwords) and
rewrites bit 0 of `+0x18` after it. The dead bit and the level and zone offsets are those of `objectlayer/party#the-party-member-record`
shifted by 0x10. A member present before a list keeps its `+0x158` word; a new member starts at 0.

## Group leader and membership

`IsRaidLeader()` takes no argument and pushes 1 when the group leader cell `0xbc75f8` is the active player's GUID, else nil. The cell is the
same one `IsPartyLeader` reads, so a party leader is a raid leader to this function. `UnitInRaid(unit)` pushes the number 1, not an index,
when the unit's GUID is a roster member's, else nil; a recognised token that names nothing, an empty
argument and a non-string give nil, and a token no prefix matches raises (`unittokens#the-unit-token`). `GetNumRaidMembers()` pushes `0xb713e0`.

Addresses: `0x4bb8c0`, `0x4bb8c2`, `0x4bb8c9`, `0x4bb8d1`, `0x4bb8de`, `0x4bb8f5`, `0x516350`, `0x516365`, `0x516373`, `0x4baee0`, `0x4baee9`, `0x4bafd0`, `0x4bb530`, `0xbc75f8`
Confidence: verified
Proof: one
Test: none

`0x4baee0(&guid)` returns 1 or 0 over the roster GUIDs and returns 0 at once for a zero GUID; `UnitInRaid` pushes the literal double 1.0
(`push 0x3ff00000; push 0`) for 1 and nil for 0, one value either way. `IsRaidLeader` has no zero guard, unlike `IsPartyLeader`.

## GetRaidRosterInfo

`GetRaidRosterInfo(index)` is 1-based, raises `Usage: GetRaidRosterInfo(index)` for a non-number and always returns nine values: name, rank,
1-based subgroup, level, class name, class token, zone, online and dead. The level is the held object's, else the block's word for an online
member, else 0. The class comes from the name cache record (`+0x140`) through ChrClasses (`+0x14 + 4*locale`; token at `+0x38`). The zone of a held
object is the local player's `0xb4e314`; else the block's zone, or the text `PLAYER_OFFLINE` for an offline member, whose online value is nil. Dead is 1 when a held
object's health is 0 or less, and for a member without one when status bits 0 and 2 are both set, else nil.

Addresses: `0x4bb560`, `0x4bb582`, `0x4bb5bb`, `0x4bb5be`, `0x4bb5c4`, `0x4bb5ee`, `0x4bb5f8`, `0x4bb607`, `0x4bb617`, `0x4bb65d`, `0x4bb66c`, `0x4bb68e`, `0x4bb6b6`, `0x4bb6c4`, `0x4bb6e5`, `0x4bb6f0`, `0x4bb739`, `0x4bb740`, `0x4bb777`, `0x4bb796`, `0x4bb7b9`, `0xb4e314`, `0x55f080`
Confidence: verified
Proof: one
Test: none

`dec eax; cmp eax,ecx; jae` bounds the index by the unsigned count, so 0 and negative indices fall to `0x4bb7b9`, which also serves a
roster slot whose name is not cached. That arm pushes `nil, 0, 1, 1, nil, nil, nil, nil, nil`: nine values. Every other return
site sets `eax` to 9. The block's status byte `+0x18` is tested for the dead bit 0x4 and then the online bit 0x1 (`test al,4`, `test al,1`), the bits of the party record; a held object
gives its health at `[[obj+0x110]+0x40]`.

## Starting a ready check

`DoReadyCheck()` runs `0x4bb1d0`. For the group leader only, and only when no check is running, it marks every roster record with a
nonzero GUID other than the leader's as unanswered (`+0x158 = 1`), the leader's own as 0, sends opcode 0x322 with no body, sets the deadline
`0xb713f4` to `now + 0x7530` (30 s) and shows `READY_CHECK_START` as a `CHAT_MSG_SYSTEM` line. While a check runs it shows
`ERR_READY_CHECK_IN_PROGRESS` and does nothing else. `CheckReadyCheckTime()` runs `0x4bb310`. A member who receives opcode 0x322 with
no body (`0x4ba4ee`) shows `ERR_RAID_LEADER_READY_CHECK_START_S` with the leader's name when it is cached, sets the same 30 s deadline and signals `READY_CHECK`.

Addresses: `0x4bb1d0`, `0x4bb1eb`, `0x4bb203`, `0x4bb20c`, `0x4bb230`, `0x4bb24c`, `0x4bb258`, `0x4bb270`, `0x4bb2b6`, `0x4bb2c7`, `0x4bb2f0`, `0x4bc080`, `0x4bc120`, `0x4ba4ee`, `0x4ba518`, `0x4ba526`, `0x4ba530`, `0x4ba53a`, `0xb713f4`, `0xbc75f8`
Confidence: verified
Proof: one
Test: none

`0x4bb2f0` returns `deadline - now` while the signed difference is negative, else 0; it is the test for a check in progress. The roster loop skips a
record whose GUID is 0 (`0x4bb240`). Message ids are those of `tables/messages.tsv` (454 and 455) and `READY_CHECK` is event 536. The 30 seconds are the
immediate `0x7530`.

## Answering a ready check

`ConfirmReadyCheck([ready])` sends opcode 0x322 with one byte, the low byte of `GetScriptFlagArg(1, 0)`, only while a check is running; the
byte is 1 for true and 0 for false or nothing. The leader's own client handles an incoming 0x322 (`u64 guid`, `u8 status`) at `0x4ba3b0`: for the roster record of that GUID it
stores 0 in `+0x158` whatever the status was, and only for a status of 0 it shows `RAID_MEMBER_NOT_READY` with the member's name. A body-less message reads as
GUID 0 and status 0 and matches nothing.

Addresses: `0x4bc090`, `0x4bc099`, `0x4bc0ad`, `0x4bc0d8`, `0x4bc0e4`, `0x4ba360`, `0x4ba3a2`, `0x4ba3b7`, `0x4ba3c4`, `0x4ba3d4`, `0x4ba3f9`, `0x4ba405`, `0x4ba40e`, `0x4ba414`, `0x4ba41d`, `0x4ba44c`, `0x4ba484`, `0x4ba48f`, `0x49a870`
Confidence: verified
Proof: one
Test: none

Both reads are guarded by the data store's at-end test (vtable `+0x14`), so a short message leaves the locals 0. `ecx` holds the constant 0 from `0x4ba3e4`
to the store at `0x4ba40e` (re-zeroed at `0x4ba48f` after the calls), and the status byte is read only by the compare at `0x4ba414`. The client sends the plain byte; the
store ignores it. The line is `AddChatLine(text, 10, ten zeros)`, type 10 being `CHAT_MSG_SYSTEM` (`chatevents`).

## Ending a ready check

The check ends when the leader's client has no unanswered online member, when a roster change leaves none, or at the deadline. `0x4ba491` tests every record
for `+0x158 == 1` and bit 0 of `+0x18`; reaching the end of the roster, or an empty roster, without one sets the deadline to `now - 1` and runs the summary `0x4bb310`
(`0x4ba4ce`). `0x4bb310` runs for the leader once the deadline has passed (`CheckReadyCheckTime` calls it too): it clears the deadline and shows `RAID_MEMBERS_AFK` with the names of
all records whose `+0x158` is nonzero, joined by a comma, or `READY_CHECK_NO_AFK` when there are none.

Addresses: `0x4ba491`, `0x4ba498`, `0x4ba4a7`, `0x4ba4b6`, `0x4ba4c9`, `0x4ba4ce`, `0x4ba4d4`, `0x4ba4d9`, `0x4bb310`, `0x4bb340`, `0x4bb377`, `0x4bb3a8`, `0x4bb3f2`, `0x4bb46c`, `0x4bb489`, `0x4bac5a`, `0x4bacb9`
Confidence: verified
Proof: one
Test: none

An offline member who never answered does not hold the check open but is listed as away: the summary tests only `+0x158`. The scan also runs at the end of the group list
handler (`0x4bacb9`), so a pending member who leaves or goes offline ends the check at the next roster message. The list buffer is 0x200 bytes joined with `SStrPack`.
`0x4ba4b6` leaves the scan early only when a pending online member and the answering member's record (the second flag, `[ebp-2]`, set at `0x4ba417`) have both been seen, to the plain return `0x4ba53f`.

## Setting a raid target

`SetRaidTarget(unit, index)` raises `Usage: SetRaidTarget(unit, index)` for a bad argument, returns no value and
sends opcode 0x321: `u8 slot`, `u64 guid`, 13 bytes. The slot is the low byte of `index` minus 1 when that is 0 to 7. Any other index clears: the slot is the one the unit holds and the GUID is 0;
a unit with no icon sends nothing, and a unit with GUID 0 returns silently. For a slot 0 to 7 the unit must be a raid or party member or pet, an NPC, a player the
local player can cooperate with, or the local player; else `ERR_INVALID_RAID_TARGET` is shown and nothing is sent. No leader check.

Addresses: `0x4bbec0`, `0x4bbed0`, `0x4bbee4`, `0x4bbeff`, `0x4bbf0c`, `0x4bbf23`, `0x4bbf29`, `0x4bbf36`, `0x4bbf43`, `0x4bbf56`, `0x4bbf66`, `0x4bbfa4`, `0x4bbfc1`, `0x4bbfc4`, `0x4bbfd0`, `0x4bbfd2`, `0x4bbfea`, `0x4bbff8`, `0x4bc01b`, `0x4bc02f`, `0x4bc03a`, `0x4bafd0`, `0x4e8fa0`, `0x606ba0`, `0x4bb190`
Confidence: verified
Proof: one
Test: none

The tests run raid roster `0x4bafd0` first, then party `0x4e8fa0`, then the two objects (`0x468460` with mask 0x10 for the player and 8 for the target); a missing object or a player
that cannot be cooperated with fails. `0x606ba0` compares the faction groups of the two objects. The clear path jumps over all of these to `0x4bbfea`, where a slot of 8 or more returns.
Nothing is written locally: the table changes only when the server echoes opcode 0x321. The roster rebuild `0x4ba5f0` also sends the opcode with the byte 0xff and no GUID (`0x4bac01`, under a condition on a local flag), the full-list request.

## The raid target table

`GetRaidTargetIndex(unit)` pushes one value, the unit's 1-based icon number 1 to 8, or nil. The table `0xb71368` holds eight GUIDs; `0x4bb190(guid)` returns the slot of a
GUID or 8. The wire is 0-based, the Lua side 1-based. Opcode 0x321 arrives at `0x4ba220`: `u8 mode`; a nonzero mode clears all eight and refreshes the nameplate icon of each former holder;
then `u8 slot, u64 guid` pairs are read to the end, each refreshing the old and new holder, and `RAID_TARGET_UPDATE` is signalled once. A group list without the raid flag also
clears the table (`0x4ba550`): when the roster was not empty it shows `ERR_RAID_YOU_LEFT` and signals `RAID_ROSTER_UPDATE`; it always zeroes the table and signals `RAID_TARGET_UPDATE`.

Addresses: `0x4bb4b0`, `0x4bb4be`, `0x4bb4f1`, `0x4bb4f6`, `0x4bb50c`, `0x4bb190`, `0x4ba220`, `0x4ba235`, `0x4ba24b`, `0x4ba259`, `0x4ba28a`, `0x4ba2f3`, `0x4ba343`, `0x4ba550`, `0x4ba556`, `0x4ba55f`, `0x4ba56c`, `0x4ba587`, `0x4ba59d`, `0x4ba5d6`, `0x4ba5f0`, `0x5e6e6b`, `0x5e6ebb`, `0x608a90`, `0xb71368`
Confidence: verified
Proof: one
Test: none

`GetRaidTargetIndex` raises `Usage: GetRaidTargetIndex(unit)` for a non-string and returns nil rather than 0, so the stock frame hides the icon. `0x608a90` refreshes a unit's nameplate icon; its atlas lookup uses the 0-based
index (`nameplates`). The clear sits in the group list handler (`0x5e6a40`): when the raid flag byte is clear (`0x5e6e6b`) it calls `0x4ba550` (`0x5e6ebb`), else `0x4ba5f0`. The writers of the table are the start-up fill `0x4ba0dc`,
the mode clear and the entry stores of `0x4ba220`, and the clear of `0x4ba550`; `SetRaidTargetIconTexture` is not an engine function.
