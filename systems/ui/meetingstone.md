# ui: the meeting stone queue

`MeetingStoneInfo.cpp` keeps two words for the stone queue, the id of the area the player is queued for and a
status text, answers the server's queue messages and exposes three Lua functions. The stone object's own use
check and highlight are `objectlayer/gameobjecttypes`.

Roots: `0x4c9ee0`, `0x4c9f40`, `0x4c9f80`, `0x4ca230`, `0x4ca3c0`

## State and Lua functions

The queued area id is `0xb72038`, zero when not queued, and the status text is the heap string `0xb7203c`, null when
unset. `IsInMeetingStoneQueue()` pushes the number 1 when the id is nonzero, else nil. `GetMeetingStoneStatusText()` pushes
the string, or nil while it is null. `CancelMeetingStoneRequest()` returns no value: in a party whose leader is not the
player it shows `ERR_MEETING_STONE_NOT_LEADER` and sends nothing; otherwise it sends opcode 0x293 with no body and
changes no state. None of the three takes an argument or raises, and none pushes a boolean.

Addresses: `0x4ca570`, `0x4ca5b0`, `0x4ca5a0`, `0x4ca120`, `0x4ca126`, `0x4ca14e`, `0x4ca168`, `0x4ca5d0`, `0x4ca5f0`, `0xb72038`, `0xb7203c`, `0x6f3810`, `0x6f3890`, `0x6f37f0`
Confidence: verified
Proof: one
Test: none

The registration table holds three name and function pairs; its loop ends at `cmp esi,0x18` (`0x4ca5e7`). Each of
`0x4ca570` and `0x4ca5b0` sets `eax` to 1 on both legs, so nil is one returned value. `0x6f3890` jumps to
`lua_pushnil` for a null string. The cancel gate (`0x4ca126` to `0x4ca14e`) skips both checks when the first
party GUID slot `0xbc6f48` is zero, then compares the group leader GUID `0xbc75f8` with the player's.

## The status text

`0x4ca070` rebuilds the text: it takes the global `MEETINGSTONE_TOOLTIP` as a format and the queued area's name in
the client's locale as its one argument, and formats into a 256-byte buffer with `SStrPrintf`. A negative id, an id
past the AreaTable maximum or an id without a row uses the global `UNKNOWN` as the name. The old string is freed and
the new one is a duplicate of the buffer. A missing global gives the empty string, so the fallback format `%s`
at `0x4ca0ca` is not reached.

Addresses: `0x4ca070`, `0x4ca08b`, `0x4ca090`, `0x4ca094`, `0x4ca0a2`, `0x4ca0af`, `0x4ca0ca`, `0x4ca0dd`, `0x4ca0e2`, `0x4ca0fe`, `0x4ca111`, `0x703bf0`, `0x64a7f0`, `0x64a620`, `0xc0e048`, `0xc0e04c`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

`0x703bf0(key, -1, 0)` returns the value of the Lua global or the shared empty string `0x882748`, never null: the
empty string is the immediate operand `mov dword [ebp-4], 0x882748` in its prologue. The area name is
`[AreaTable row + 0x2c + 4*locale]`. The one call of `0x4ca070` is at `0x4ca385`, and no table holds its address.

## Entering and leaving the world

`0x4c9f40` frees the old text, stores a copy of the global `UNKNOWN` and jumps to `0x4ca1c0`, which sends opcode 0x296 with no body.
`0x4c9f80` frees the text and sets it null; it leaves the area id. `0x4c9ee0` zeroes the area id and registers the
handlers for opcodes 0x295, 0x297, 0x298, 0x299 and 0x2bb; `0x4c9fb0` clears them. The world-enter cascade calls
`0x4c9f40`, the world-leave cascade `0x4c9f80`, so the query is sent on every pass of the enter cascade, including a UI
reload, and between it and the server's 0x295 reply the status text is the bare `UNKNOWN`.

Addresses: `0x4c9f40`, `0x4c9f74`, `0x4c9f79`, `0x4ca1c0`, `0x4c9f80`, `0x4c9f98`, `0x4c9ee0`, `0x4c9eec`, `0x4c9fb0`, `0x490a14`, `0x490b2f`, `0x48f608`, `0x491213`, `0xb4b424`
Confidence: verified
Proof: one
Test: none

`0x4c9f40` has one caller, `0x490a14`, inside the enter cascade `0x4908c0`, which a run-once byte `0xb4b424` guards;
the leave cascade `0x490a80` clears that byte and calls `0x4c9f80` (`0x490b2f`). `0x4c9ee0` is called by the
handler registration cascade at `0x48f608`, `0x4c9fb0` from the teardown cascade at `0x491213`.

## The queue message

Opcode 0x295, `0x4ca230`: `u32 areaId`, `u8 status`. The handler remembers the old id, stores the new id
unconditionally, and for status 0 to 4 shows one message. Whatever the status, it then rebuilds the status text
and signals `MEETINGSTONE_CHANGED` with no argument, and returns 1.

| status | message (ids in `tables/messages.tsv`) | argument |
|---|---|---|
| 0 | `ERR_MEETING_STONE_LEFT_QUEUE_S` (423) | the old area's name; nothing is shown when the old id has no row |
| 1 | `ERR_MEETING_STONE_IN_QUEUE_S` (422) | the new area's name, `UNKNOWN` when it has no row; skipped when the id did not change |
| 2 | `ERR_MEETING_STONE_OTHER_MEMBER_LEFT` (424) | none |
| 3 | `ERR_MEETING_STONE_PARTY_KICKED_FROM_QUEUE` (425) | none |
| 4 | `ERR_MEETING_STONE_MEMBER_STILL_IN_QUEUE` (426) | none |

Addresses: `0x4ca230`, `0x4ca24a`, `0x4ca25a`, `0x4ca265`, `0x4ca274`, `0x4ca279`, `0x4ca27f`, `0x4ca286`, `0x4ca2c7`, `0x4ca2c9`, `0x4ca2f4`, `0x4ca311`, `0x4ca350`, `0x4ca363`, `0x4ca36a`, `0x4ca385`, `0x4ca38f`, `0x4ca3a4`
Confidence: verified
Proof: one
Test: none

The status switch is the table `0x4ca3a4` of five addresses; a status above 4 goes straight to `0x4ca385`. Status 1
also runs the visual effect `Meeting Stone Join` (kind 0xc of the 14 named effects) on the player: a 0x90-byte
effect is allocated, `0x61fae0(0xc, &playerGuid, 0x5fbf50)` starts it and `0x6210e0` releases it. Then it calls the tutorial
helper `0x4b5390(0x2e, 0)` (`tutorials#the-tutorial-trigger`), which signals `TUTORIAL_TRIGGER` with 47 once per character.
`0x496720` is called with the message id first and the name second. Event 519 (`MEETINGSTONE_CHANGED`) has the one
fire site `0x4ca38f`. A short packet leaves the locals unset and the handler stores them.

## The status messages

One handler, `0x4ca3c0`, serves opcodes 0x297, 0x298, 0x299 and 0x2bb and switches on the opcode (`edx`). 0x297
shows `ERR_MEETING_STONE_SUCCESS` (427) and 0x298 `ERR_MEETING_STONE_IN_PROGRESS` (428), neither reading a body. 0x299 reads a
GUID and shows `ERR_MEETING_STONE_MEMBER_ADDED_S` (429) with the member's name, at once from the name cache or from the cache
callback `0x4ca510` once the name arrives; a callback with a zero success byte shows nothing. 0x2bb reads a byte: 1
shows `ERR_MEETING_STONE_MUST_BE_LEADER` (433), 2 `ERR_MEETING_STONE_GROUP_FULL` (430), 3 `ERR_MEETING_STONE_NO_RAID_GROUP` (434), anything else nothing. None of the four
touches the area id or the text or signals an event.

Addresses: `0x4ca3c0`, `0x4ca3c6`, `0x4ca3cf`, `0x4ca3d5`, `0x4ca3dc`, `0x4ca3e3`, `0x4ca3ef`, `0x4ca448`, `0x4ca460`, `0x4ca478`, `0x4ca4b2`, `0x4ca4bc`, `0x4ca4c9`, `0x4ca4d4`, `0x4ca4e8`, `0x4ca510`, `0x4ca526`, `0x4ca554`
Confidence: verified
Proof: one
Test: none

`lea eax,[edx-0x297]` bounds the opcode to 0 to 0x24, a byte table `0x4ca4e8` maps it to one of five table entries
`0x4ca4d4` (opcodes 0x29a to 0x2ba and anything else go to the plain return `0x4ca4c9`). In the 0x2bb arm the
byte is decremented step by step (`dec eax; je`), so 1, 2 and 3 select the three arms. The messages of kind 0 in
`tables/messages.tsv` are chat lines (`CHAT_MSG_SYSTEM`) and those of kind 2 are error-frame text.

## Joining a stone

`0x4c9ff0(guid)` sends opcode 0x292 with the stone's GUID. Its one caller, `0x5f6af6`, is the end of the stone's use
check `0x5f69d0` (`objectlayer/gameobjecttypes`), which refuses with `ERR_MEETING_STONE_MUST_BE_LEADER`,
`ERR_MEETING_STONE_GROUP_FULL`, `ERR_MEETING_STONE_INVALID_LEVEL` or `ERR_MEETING_STONE_NO_RAID_GROUP` and sends
nothing in those cases.

Addresses: `0x4c9ff0`, `0x4c9ff9`, `0x4ca02b`, `0x5f6af6`, `0x5f69d0`
Confidence: verified
Proof: one
Test: none

The body is the opcode and the 64-bit GUID (`0x418190`, then `0x418370`).
