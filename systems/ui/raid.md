# ui: the raid roster, ready check and raid targets

`RaidInfo.cpp` keeps the raid roster, answers the ready check opcode and the raid target opcode, and registers 16 Lua
functions (`tables/bindings.tsv`). The roster is rebuilt by the group list message (`objectlayer/party#the-group-list`); the leader
cell and the party side are `party`. This doc covers the roster record and its rebuild, the three query functions, the ready check
and the raid targets.

Roots: `0x4ba0b0`, `0x4ba140`, `0x4ba220`, `0x4ba360`, `0x4ba5f0`, `0x4bb1d0`, `0x4bb310`, `0x4bbec0`

## The roster record

The roster is 40 pointers at `0xb712a8` to records of 0x160 bytes, allocated at start-up, with the member count in `0xb713e0`. A record
holds: `+0` the GUID, `+8` the 0-based subgroup (the member's flag dword AND 0x7f), `+0xc` the rank (2 for the group leader's GUID, 1 when the
sign bit of the flag byte is set, else 0), `+0x10` a 0x148-byte member block of the party record kind, and `+0x158` the ready check word. In the block `+0x18` is
the status byte (bit 0 online, bit 2 dead), `+0x22` the level, `+0x24` the zone and `+0x90` the pet GUID. Bit 0 of `+0x18` comes from the group list's per-member
online byte.

Addresses: `0x4ba0b0`, `0x4ba0c0`, `0x4ba0d0`, `0x4ba108`, `0x4ba114`, `0x4ba125`, `0x4ba8d3`, `0x4ba8ea`, `0x4ba90c`, `0x4ba91c`, `0x4ba920`, `0x4ba945`, `0x4ba958`, `0x4ba960`, `0x4ba5f0`, `0xb712a8`, `0xb713e0`
Confidence: verified
Proof: two
Test: none

`0x4ba0b0` registers the target opcode 0x321 and the ready check opcode 0x322, zeroes the target table and allocates the records with
`SMemAlloc(0x160, ...)`: `cmp esi,0xa0` at 4 bytes a step gives 40. `0x4ba5f0` copies the 0x148-byte block into `+0x10` (`rep movs`, 0x52 dwords) and
rewrites bit 0 of `+0x18` after it. The dead bit and the level and zone offsets are those of `objectlayer/party#the-party-member-record`
shifted by 0x10. A member present before a list keeps its `+0x158` word; a new member starts at 0. An independent cold
read derived the same rows and fields.

## Rebuilding the roster

`0x4ba5f0(guids, flags, online, count)` replaces the roster with the group list's raid members, in list order. A GUID already on the
roster keeps its 0x148-byte block and ready check word; a new one gets a zeroed block whose u16 health, maximum, power and maximum power
are 1, and the word 0. Each row takes the GUID, subgroup, rank and online bit; rows past the new count keep what they held
(`#rows-past-the-count`). In order: tutorial 45 for a list with members; the notices (`#roster-change-notices`); `ERR_RAID_YOU_JOINED`
when the roster was empty; the rows and their name requests (`#pending-roster-names`); the token flags (`#unit-tokens-after-a-rebuild`);
`RAID_ROSTER_UPDATE` when no name is pending; and, when the roster was empty, opcode 0x321 with the byte 0xff.

Addresses: `0x4ba5f0`, `0x4ba60d`, `0x4ba612`, `0x4ba6c7`, `0x4ba6e4`, `0x4ba70f`, `0x4ba716`, `0x4ba721`, `0x4ba838`, `0x4ba83a`, `0x4ba870`, `0x4ba89e`, `0x4ba8a4`, `0x4ba8d3`, `0x4ba8ea`, `0x4ba90c`, `0x4ba920`, `0x4ba929`, `0x4ba945`, `0x4ba960`, `0x4ba978`, `0x4ba993`, `0x4baa91`, `0x4babef`, `0x4bac01`, `0x4bac33`, `0x5e6eb4`, `0xb713e0`, `0xbc75f8`
Confidence: verified
Proof: one
Test: `ui::raid_rebuild_difftest`

It is `__fastcall(ecx = guids, edx = flags, online, count)` with `ret 8`, over the 8-byte GUIDs, flag dwords and online bytes of
`objectlayer/party#the-group-list`. `0x4ba60d` is `FireTutorial(0x2d)` under `test ebx,ebx; jbe`, and `0x4ba612` keeps `count == 0` of the
old count `0xb713e0` at `ebp−0x28`. The first loop matches each new GUID against the old rows below the old count (`0x4ba6c7`): a match
copies the old block into a stack buffer (`0x4ba6e4`), a miss zeroes one (`0x4ba70f`) and stores 1 in the words at block `+0x0a` to `+0x10`
(`0x4ba716` to `0x4ba721`). `0x4ba838` passes message 404 (`0x4ba83a`) only for an old count of 0. The old GUIDs and ready words are saved
(`0x4ba870`), `0x4ba89e` zeroes the pending count and `0x4ba8a4` stores the new count. Per row: GUID `0x4ba8d3`, subgroup `0x4ba8ea`, rank
`0x4ba90c` to `0x4ba929`, block `0x4ba945`, online `0x4ba960`, and the word found by GUID in the saved list (`0x4ba978`, `0x4baa91`) or 0
(`0x4ba993`). The event is `0x4babef`; the request is built at `0x4bac01` and sent at `0x4bac33` under the flag at `ebp−0x28`, the full
target list request of `#setting-a-raid-target`. The test runs `0x4ba5f0` whole.

## What the rebuild sends

A rebuild sends two kinds of message. When the roster was empty on entry it sends `MSG_RAID_TARGET_UPDATE` (0x321) with the one byte
0xff, the full target list request, after the rows and the roster event; for each added member with no player object it sends
`CMSG_REQUEST_PARTY_MEMBER_STATS` (0x27f) with the member's GUID, before that event. A rebuild into a roster that had members sends no
0x321, and an added member whose object is held sends no 0x27f.

Addresses: `0x4ba612`, `0x4ba61c`, `0x4babf4`, `0x4bac01`, `0x4bac20`, `0x4bac33`, `0x4bab18`, `0x4bab21`, `0x4bab6e`, `0x4bab95`, `0x4baba1`, `0x4babac`, `0x5ab630`
Confidence: verified
Proof: two
Test: `ui::raid_rebuild_difftest`

`0x4ba61c` keeps `sete` of the old count at `ebp−0x28`, and `0x4babf4` tests it before `0x4bac01` puts the opcode, `0x4bac20` the byte
0xff and `0x4bac33` sends through `ClientServices::Send`. For an added member `0x4bab18` looks the GUID up with mask 0x10 and `0x4bab21`
takes the send path for a null object: `0x4bab6e` puts 0x27f, `0x4baba1` the GUID through `CDataStore::PutGuid`, and `0x4babac` sends. An
independent cold read derived the same. The test compares the bytes each send carries.

## Unit tokens after a rebuild

After the rows, each member gone and then each new member has its unit token flags changed (`objectlayer/units#reactions-tokens-and-groups`).
A member gone whose player object is held loses flag 0x100; its pet, `UNIT_FIELD_CHARM` or else `UNIT_FIELD_SUMMON`, first loses 0x200 when
its object is held, or gets `ReleaseTarget(1, pet)` when the GUID is nonzero and the object missing. A member gone with no object gets
`ReleaseTarget(1, guid)`. A new member whose object is held gains 0x100, then its pet 0x200 when the pet's object is held; one with no object
is asked for with opcode 0x27f and its GUID (`party#asking-for-stats`). Then the leader's running ready check ends when a member left and
no row below the count is unanswered and online (`#ending-a-ready-check`).

Addresses: `0x4baa00`, `0x4baa18`, `0x4baa2d`, `0x4baa3c`, `0x4baa64`, `0x4baa74`, `0x4baa80`, `0x4baab8`, `0x4baac4`, `0x4baad9`, `0x4bab18`, `0x4bab2a`, `0x4bab55`, `0x4bab65`, `0x4bab6e`, `0x4babac`, `0x4bac5a`, `0x4bac62`, `0x4bac6d`, `0x4bac90`, `0x4bacb4`, `0x4bacb9`, `0x468460`, `0x60c4f0`, `0x60c4a0`, `0x493910`, `0x5ab630`, `0xb713f4`
Confidence: verified
Proof: one
Test: `ui::raid_rebuild_difftest`

The first two loops list the new members (`0x4ba75b`) and the members gone (`0x4ba805`). For one gone, `0x4baa18` looks the GUID up with
mask 0x10 and the pet is read from the descriptor `+0x110`: the CHARM pair, or the SUMMON pair at `+8` when CHARM is 0 (`0x4baa2d`).
`0x4baa3c` skips a pet GUID of 0 and `0x4baa64` looks the pet up with mask 8. `0x4baa74` and `0x4baa80` are
`RemoveUnitTokenFlags(0x200)` on the pet and `(0x100)` on the player; `0x4baab8` releases a missing pet before `0x4baac4`, and `0x4baad9`
releases a missing player. For one new, `0x4bab18` looks it up, `0x4bab2a` adds 0x100 before the pet is read, `0x4bab55` looks the pet up
with no zero test and `0x4bab65` adds 0x200; with no object `0x4bab6e` puts opcode 0x27f and the GUID and `0x4babac` sends them. The
ready check end tests the deadline `0xb713f4` (`0x4bac5a`), the count of members gone at `ebp−8` (`0x4bac62`), the active player against
the leader cell (`0x4bac6d`) and the rows (`0x4bac90`); with none open it stores `OsGetAsyncTimeMs() − 1` (`0x4bacb4`) and runs
`0x4bb310` (`0x4bacb9`). The test runs these inside `0x4ba5f0` with the callees stubbed.

## Pending roster names

The rebuild asks the name cache for each row with a nonzero GUID, with `0x4ba1b0` as the callback, and counts the misses in `0xb713f0`.
With none it signals `RAID_ROSTER_UPDATE` itself; otherwise each answer runs `0x4bad90`, which lowers the count and at 0, or when it
already was 0, signals `RAID_ROSTER_UPDATE` and then `PARTY_MEMBERS_CHANGED`. A rebuild while names are pending first cancels that
callback on every old row's GUID and restarts the count at 0. On entering the world `0x4ba140` gives the first row below the count with
GUID 0, the own row the group list appends while `ClntObjMgrGetActivePlayer` returns 0, the active player's GUID and asks for its name
the same way; a cached name signals `RAID_ROSTER_UPDATE` alone.

Addresses: `0x4ba9ab`, `0x4ba9c7`, `0x4ba9d0`, `0x4ba622`, `0x4ba631`, `0x4ba64b`, `0x4babdf`, `0x4ba1b0`, `0x4bad90`, `0x4bad97`, `0x4bad9f`, `0x4bada6`, `0x4badb0`, `0x4ba140`, `0x4ba165`, `0x4ba193`, `0x4ba1a2`, `0x4ba1a7`, `0x490a0f`, `0x55f870`, `0x55f080`, `0x5e6e73`, `0xb713f0`
Confidence: verified
Proof: one
Test: `ui::raid_pending_names_difftest`, `ui::raid_rebuild_difftest`

`0x4ba9ab` skips GUID 0; `0x4ba9c7` is `GetRecord(guid, row, 0x4ba1b0, 0, 0)` and a null return raises the count (`0x4ba9d0`). `0x4ba622`
tests the count, and `0x4ba631` to `0x4ba64b` call `0x55f870(guid, 0x4ba1b0, 0)` for each old row. `0x55f870` compares each callback
waiting on the GUID's request with the function and data (`0x55f934`, `0x55f93f`) and frees a match with `SMemFree` (`0x55f97f`); for a
record already loaded it prints `DBCache::CancelCallback ignored for id`. `0x4babdf` signals only for a count of 0. `0x4ba1b0` is `call
0x4bad90; ret 8`, so it ignores the answer's GUID and found byte; its address is pushed at `0x4ba187`, `0x4ba640` and `0x4ba9bb`, and no
instruction calls it. In `0x4bad90`, `0x4bad97` goes to the signal for a count of 0 and `0x4bad9f` returns while the decrement leaves it
nonzero; `0x4bada6` signals event 499 and `0x4badb0` tail-jumps with 218. `0x4ba140` has one caller, `EnterWorldUI` (`0x490a0f`);
`0x4ba165` reads the GUID, `0x4ba193` asks, `0x4ba1a2` tail-jumps to the signal and `0x4ba1a7` raises the count. The group list's own row
is `0x5e6e73`. The tests run `0x4ba1b0` and `0x4ba140` whole.

## Rows past the count

Within one world session nothing clears a row at or past the member count: `0x4ba0b0` zeroes all 40 when the session starts and
`ShutdownRaid` frees them when it ends, while the rebuild writes the rows below its new count, the other writers stop at the count, and
`ClearRaidRoster` (`0x4ba550`) only zeroes the count. Membership (`0x4baee0`, and so `UnitInRaid`) stops at the count, but `0x4bb0f0`
returns the block of the first of all 40 rows holding a GUID, and `0x4bb130` the pet block of the first online row whose pet GUID
matches. So a GUID that left the raid finds its old row: the record getters fall back to it
(`party#reading-a-member-without-its-object`), and a stats message for it writes into it (`objectlayer/party#party-member-stats`).

Addresses: `0x4ba0b0`, `0x4ba10d`, `0x4ba12a`, `0x48f603`, `0x4ba1d0`, `0x4ba1fb`, `0x49120e`, `0x4bb0f0`, `0x4bb0fb`, `0x4bb115`, `0x4bb127`, `0x4bb130`, `0x4bb149`, `0x4bb14e`, `0x4bb169`, `0x4bb17b`, `0x4baee0`, `0x4baeee`, `0x4ba550`, `0x4ba571`, `0x4ba173`, `0x4bb24c`, `0x4bb264`, `0x4ba40e`, `0x4ba3e8`, `0x4bae1d`, `0x4bae03`, `0x49640f`, `0x49642f`, `0x5e5166`, `0x5f0bcc`, `0x4ba0a5`
Confidence: verified
Proof: one
Test: none

`0x4bb0f0` returns 0 for GUID 0 (`0x4bb0fb`), compares the GUIDs of rows 0 to 39 (`cmp ecx,0x28`, `0x4bb115`) with no status test, and
returns `row + 0x10` (`0x4bb127`). `0x4bb130` skips a row with GUID 0 (`0x4bb149`) or with bit 0 of `+0x18` clear (`0x4bb14e`), stops at
0x28 (`0x4bb169`) and returns `row + 0x90` (`0x4bb17b`). `0x4baee0` stops at `0xb713e0` (`0x4baeee`). Of the 37 functions that name the
table or the count, those that store through a row pointer are `0x4ba0b0`, which zeroes each new record (`0x4ba10d`, `rep stos` at
`0x4ba12a`) once, from `CGGameUI::InitializeGame` (`0x48f603`), the rebuild, `0x4ba140` (`0x4ba173`), the ready check's start (`0x4bb24c`,
bound `0x4bb264`) and answer (`0x4ba40e`, bound `0x4ba3e8`), and `0x4badc0`, which rebuilds a found member's block through `0x5f0880`
(`0x4bae1d`, bound `0x4bae03`); the rebuild's stores are the control. `0x4ba550` stores the count (`0x4ba571`) and the target table only.
The scans' callers are `GPartyInfo::GetRemoteStats` (`0x49640f`) after the party record, `GetPartyPetRecord` (`0x49642f`),
`ReadPartyMemberStats` (`0x5e5166`), which writes the block it gets, and `CGPlayer_C::ComparePartyRecord` (`0x5f0bcc`), which copies it.
`ShutdownRaid` frees the records (`0x4ba1fb`), from `CGGameUI::ShutdownGame` (`0x49120e`).
A 41st list entry (`objectlayer/party#the-group-list`) takes its row pointer from the dword after the table, which `0x4ba0a5` sets to 0x7f800000.

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

`ConfirmReadyCheck([ready])` sends opcode 0x322 with one byte, the low byte of `GetScriptFlagArg(1, 0)`, only while a check is running. A number
sends its truncated low byte (2 sends 2, 1.9 sends 1, 256 sends 0), a boolean its value, nil 0 and no argument the default 0; a string follows the
first-letter table of `ui/actionuse#reading-a-script-flag`. The leader's own client handles an incoming 0x322 (`u64 guid`, `u8 status`) at `0x4ba3b0`: for the roster record of that GUID it
stores 0 in `+0x158` whatever the status was, and only for a status of 0 it shows `RAID_MEMBER_NOT_READY` with the member's name. A body-less message reads as
GUID 0 and status 0 and matches nothing.

Addresses: `0x4bc090`, `0x4bc099`, `0x4bc0ad`, `0x4bc0bf`, `0x4bc0d8`, `0x4bc0e4`, `0x6f1c10`, `0x6f1c46`, `0x4ba360`, `0x4ba3a2`, `0x4ba3b7`, `0x4ba3c4`, `0x4ba3d4`, `0x4ba3f9`, `0x4ba405`, `0x4ba40e`, `0x4ba414`, `0x4ba41d`, `0x4ba44c`, `0x4ba484`, `0x4ba48f`, `0x49a870`
Confidence: verified
Proof: one
Test: none

`0x4bc0ad` calls `GetScriptFlagArg`, whose number arm returns the `_ftol` result whole (`0x6f1c46`); `0x4bc0bf` keeps `al` and `0x4bc0e4` writes it with `PutInt8`. Both reads are guarded by the data store's at-end test (vtable `+0x14`), so a short message leaves the locals 0. `ecx` holds the constant 0 from `0x4ba3e4`
to the store at `0x4ba40e` (re-zeroed at `0x4ba48f` after the calls), and the status byte is read only by the compare at `0x4ba414`. The client sends the plain byte; the
store ignores it. The line is `AddChatLine(text, 10, ten zeros)`, type 10 being `CHAT_MSG_SYSTEM` (`chatevents`).

## Ending a ready check

The check ends when an answer leaves the leader's client no unanswered online member, when a roster change that removes a member leaves none, or at the deadline. `0x4ba491` tests every record
for `+0x158 == 1` and bit 0 of `+0x18`; reaching the end of the roster, or an empty roster, without one sets the deadline to `now - 1` and runs the summary `0x4bb310`
(`0x4ba4ce`). `0x4bb310` runs for the leader once the deadline has passed (`CheckReadyCheckTime` calls it too): it clears the deadline and shows `RAID_MEMBERS_AFK` with the names of
all records whose `+0x158` is nonzero, joined by a comma, or `READY_CHECK_NO_AFK` when there are none.

Addresses: `0x4ba491`, `0x4ba498`, `0x4ba4a7`, `0x4ba4b6`, `0x4ba4c9`, `0x4ba4ce`, `0x4ba4d4`, `0x4ba4d9`, `0x4bb310`, `0x4bb340`, `0x4bb377`, `0x4bb3a8`, `0x4bb3f2`, `0x4bb46c`, `0x4bb489`, `0x4bac5a`, `0x4bacb9`
Confidence: verified
Proof: one
Test: none

An offline member who never answered does not hold the check open but is listed as away: the summary tests only `+0x158`. The scan also runs at the end of the roster rebuild
when a member left (`0x4bacb9`, `#unit-tokens-after-a-rebuild`), so a pending member who leaves ends the check at that roster message, and one who only goes offline
holds it until the next answer, a departure or the deadline. The list buffer is 0x200 bytes joined with `SStrPack`.
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
Nothing is written locally: the table changes only when the server echoes opcode 0x321. The roster rebuild `0x4ba5f0` also sends the opcode with the byte 0xff and no GUID (`0x4bac01`) when the roster was empty on entry (`0x4ba612`), the full-list request.

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

## Roster change notices

When a rebuild adds a raid member to a roster that was not empty, or removes one, it asks the name cache for the member's name with
`0x4bacd0` (added) or `0x4bad30` (removed) as the completion callback. A cached name shows the notice at once: `ERR_RAID_MEMBER_ADDED_S`
(message 406), or `ERR_RAID_MEMBER_REMOVED_S` (407) when `ClntObjMgrGetActivePlayer` returns a GUID. Otherwise the callback shows it when
the cache answers: each takes the GUID and a found byte, and with the byte set and the name record found it shows its message with the
record's name through `CGGameUI::DisplayError`, else nothing. Members added to an empty roster get no notice and no request.

Addresses: `0x4bacd0`, `0x4bad30`, `0x55f080`, `0x496720`, `0x4ba725`, `0x4ba738`, `0x4ba744`, `0x4ba74e`, `0x4ba7d7`, `0x4ba7e3`, `0x4ba7ee`, `0x4ba7f8`
Confidence: verified
Proof: one
Test: `ui::raid_rebuild_difftest`

`0x4ba725` skips the request for an old count of 0. `0x4ba738` and `0x4ba7d7` push the callbacks, and the requests are `0x4ba744` and
`0x4ba7e3`; a record goes straight to `DisplayError` (`0x4ba74e`, `0x4ba7f8`), for a member gone only after `0x4ba7ee` finds an active
player GUID. The test runs that path inside `0x4ba5f0`; the callbacks run in no test. Each callback looks the GUID up with `GetRecord` on the name cache `0xc0e228` and pushes message id 0x196 or 0x197 (`0x4bad15`, `0x4bad75`) and the record, whose first bytes are the name.

