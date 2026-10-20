# objectlayer: party, guild and petitions

The group, guild and petition messages the player code reads, the party member record it keeps
for each member and the unit events a change raises, the guild tabard it shows and saves, and the
group and guild messages it sends. The party and raid lists themselves are `ui`'s. The group list
and the guild event trust the server's counts against stack arrays.

Roots: `0x5e5110`, `0x5e5050`, `0x5e6a40`, `0x5e7180`, `0x5e7520`, `0x5e03f0`, `0x5f0b80`

## Group notices

| opcode | handler | reads | does |
|---|---|---|---|
| 0x6f | `0x5e6730` | name (0x30) | `0x48f420(name)`, `ERR_INVITED_TO_GROUP_S` |
| 0x71 | `0x5e6770` | name | `0x48f440()` |
| 0x74 | `0x5e67a0` | name | `ERR_DECLINE_GROUP_S` |
| 0x79 | `0x5e67d0` | name | `ERR_NEW_LEADER_S`, or `ERR_NEW_LEADER_YOU` when it is the player's own (case-blind) |
| 0x77 | `0x5e6850` | none | `0x4e84a0(1)`, `0x4e8250(0,0)`, `ERR_UNINVITE_YOU` |
| 0x7c | `0x5e6880` | none | `ERR_GROUP_DISBANDED` when `0x4e86d0()` is nonzero; `0x4e84a0(1)`, `0x4e8250(0,0)` |
| 0x7f | `0x5e68b0` | i32 command, name, i32 result | as below |

0x7f with result 0 shows `ERR_INVITE_PLAYER_S` for command 0 with a name, and for command 2 leaves
the group as 0x77 does with `ERR_LEFT_GROUP_YOU`. Results 1 to 8 show `ERR_BAD_PLAYER_NAME_S`,
`ERR_TARGET_NOT_IN_GROUP_S`, `ERR_GROUP_FULL`, `ERR_ALREADY_IN_GROUP_S`, `ERR_NOT_IN_GROUP`,
`ERR_NOT_LEADER`, `ERR_PLAYER_WRONG_FACTION` and `ERR_IGNORING_YOU_S`; others nothing.

Addresses: `0x5e6730`, `0x5e6770`, `0x5e67a0`, `0x5e67d0`, `0x5e6850`, `0x5e6880`, `0x5e68b0`, `0x5e6a14`
Confidence: verified
Proof: one
Test: none

The result switch is `dec` and `cmp 7` through `0x5e6a14`. `0x5e67d0` compares with `SStrCmpI`
against the active player's name.

## Where the group notices appear

Every group notice (messages 0x3a to 0x4b and 0x13d) is a system chat line except `ERR_PLAYER_WRONG_FACTION` (0xff), which goes
to the red error line. `ERR_DECLINE_GROUP_S` plays `igPlayerInviteDecline`; `ERR_GROUP_FULL` and `ERR_ALREADY_IN_GROUP_S`
play an error voice, selectors 8 and 0x14; the rest are silent. A command result of 9 or more shows nothing and returns the same
1 as a shown one.

Addresses: `0x5e68b0`, `0x5e69dd`, `0x5e69e2`, `0x5e6a06`, `0x496720`, `0x484ca0`, `0x485497`, `0x4855f3`, `0x485603`, `0x485512`, `0x4856d9`, `0x486ece`, `0x486ed8`
Confidence: verified
Proof: one
Test: none

The rows of the message catalog `0xb4b498` are stored by `0x484ca0`: for these ids the route (+4) is the register that holds 0, chat,
except row 0xff, whose route is 2 (`0x486ed8`). `0x485603` stores the cue name `igPlayerInviteDecline` for 0x44; the rows 0x3d and 0x4a store 0x14
(`0x485512`) and 8 (`0x4856d9`) where the others store 0x44, the selector that plays the named cue unless it is `NONE`. `0x5e6a06` loads 1
and returns for results 9 and up without a call.

## The group list

0x7d reads a raid byte, the player's own flags and a count, then per member a name, GUID,
online byte and flags byte, into stack arrays. The previous party's members and records
are copied first. A member whose flags match the player's own in the low 7 bits takes a
party slot; a new member shows `ERR_JOINED_GROUP_S` when the player already has a group. Then
`PARTY_MEMBERS_CHANGED`, the leader, the invite-accept sound, `ERR_LEFT_GROUP_S` for those gone,
the loot method, and the raid roster: rebuilt, with the player appended to a list with members, or
cleared without the raid flag (`ui/raid#rebuilding-the-roster`). The count is unbounded: past 40
entries run on through the frame, and a raid of 40 others appends a 41st.

Addresses: `0x5e6a40`, `0x5e6a9f`, `0x5e6e6b`, `0x5e6e6f`, `0x5e6e9c`, `0x5e6ea1`, `0x5e6ea8`, `0x5e6eb4`, `0x5e6ebb`, `0x4e82d0`, `0x4e86f0`, `0x4ba5f0`, `0x4ba550`
Confidence: verified
Proof: one
Test: none

The arrays are GUIDs at `ebp−0x2b0`, flags dwords at `ebp−0x170` and online bytes at `ebp−0xa0`,
40 apart. Each kept member calls `0x4e82d0(online, old record, guid)`; the leader goes to
`0x4e8250`; the sound is `igPlayerInviteAccept`; a gone member's units get `0x493910(1, guid)`. With
members, a u8 method, a GUID and a u8 threshold go to `0x4e86f0`. With the raid flag set
(`0x5e6e6b`) and a count above 0 (`0x5e6e6f`), `ClntObjMgrGetActivePlayer`'s GUID, the player's own
flags byte and an online byte of 1 are appended (`0x5e6e73` to `0x5e6e9c`); then `0x4ba5f0(guids,
flags, online, count)` runs with the count as it stands (`0x5e6eb4`). Without the flag `0x4ba550()`
runs (`0x5e6ebb`). A change between the two refreshes quest-giver statuses.

## The party member record

Each party or raid member has a 0x148-byte record: status, power type, health and max, power and
max, level, zone, map position, 48 auras, and the pet's GUID, name, display, health, power and 48
auras. `0x5f0880(unit, record, pet)` builds one from a live unit; `0x5e5510(old, new)` returns the
mask of what differs; `0x5e5720(guid, mask, old, new)` raises the unit events for it.

| offset | value | live source |
|---|---|---|
| `+0x08` | status: 1 online, 2 PvP, 4 dead, 8 ghost, 0x10 free-for-all, 0x20 from the wire | `UNIT_FIELD_FLAGS` 0x1000, health, `PLAYER_FLAGS` 0x10 and 0x80 |
| `+0x09`, `+0x0a` to `+0x12` | power type; u16 health, max, power, max, level | fields 36, 22, 28, 23 + type, 29 + type, 34 |
| `+0x14`, `+0x16` | zone; i16 x and y | `0xb4e314`; the position, truncated |
| `+0x1a`, `+0x5a` | auras 0 to 31 and 32 to 47 | fields 47 to 94 with a flag nibble of 0xe |
| `+0x80`, `+0x88` | pet GUID and name (0x50) | `UNIT_FIELD_CHARM`, else `UNIT_FIELD_SUMMON` |
| `+0xd8` to `+0x124` | pet power type, display, health, power and auras | the pet unit |

Addresses: `0x5f0880`, `0x5e5510`, `0x5e5720`, `0x5f0b80`
Confidence: verified
Proof: one
Test: none

`0x5f0880` reads the flag nibbles from `+0x164` (field 95), `i/2` bytes in, shift `4·(i & 1)`; it
zeroes 200 bytes from `+0x80` without a pet or with `pet` 0. The mask bits run 1 status, 2 health,
4 max, 8 power type, 0x10 power, 0x20 max power, 0x40 level, 0x80 zone, 0x100 position, 0x200 and
0x400 auras, 0x800 pet GUID, 0x1000 pet name, 0x2000 display, 0x4000 to 0x40000 the pet's health
to max power, 0x80000 and 0x100000 its auras.

## Party member events

`0x5e5720` raises unit events through `0x515e50` for the member's GUID: status gives
`UNIT_HEALTH` and the power event and drops bits 0x12; then `UNIT_LEVEL`, `UNIT_HEALTH`,
`UNIT_MAXHEALTH`, `UNIT_DISPLAYPOWER`, the power and max power events for the type, `UNIT_FACTION`
when status bits 0x12 changed, and `UNIT_AURA` for either aura bit. A new pet GUID refreshes both
pets' tokens and raises `UNIT_PET`; the pet's own bits raise the same events for the pet's GUID.

Addresses: `0x5e5720`, `0x515e50`, `0x60c520`
Confidence: verified
Proof: one
Test: none

The power events are `0x11 + type` and `0x17 + type`. The pet name and display call `0x491a00` and
`0x491a50` with the pet's GUID; the tokens are `CGUnit_C::UpdateUnitTokenFlags`.

## Party member stats

0x7e and 0x2f2 (`0x5e5110`) read a packed GUID and a u32 mask, then each field the mask names, in
the record's order; bit 31 sets status bit 0x20 and its absence clears it. The old record is the
party or raid copy; the new starts from it for 0x7e and from zero for 0x2f2. The new record is
written to both lists where the member is found, and the events go out for `0x5e5510(old, new)`.

Addresses: `0x5e5110`, `0x4e8820`, `0x4bb0f0`
Confidence: verified
Proof: one
Test: none

Bits 0x200 and 0x80000 carry a u32 mask and a u16 per set bit; 0x400 and 0x100000 a u16 mask and
a u16 per set bit of its low 16. The position is two i16 (`0x418d30`), the pet name at most 0x50.

## Guild notices

| opcode | handler | does |
|---|---|---|
| 0x83 | `0x5e6f20` | inviter (0x30) and guild (0x60): `0x48f450`, `ERR_INVITED_TO_GUILD_SS` |
| 0x86 | `0x5e6f80` | `ERR_GUILD_DECLINE_S` |
| 0x88 | `0x5e6fb0` | name (0x60) and five u32: `GUILD_NAME_TEMPLATE` and `GUILD_INFO_TEMPLATE` to the chat frame |
| 0x93 | `0x5e7520` | the command result |
| 0x1f1 | `0x5e70f0` | the emblem result |
| 0x1f2 | `0x5e70c0` | opens the tabard vendor (`0x4f5840`) |

0x93 reads a command, a name (0x60) and a result. Result 0 shows `ERR_GUILD_CREATE_S`,
`_INVITE_S`, `_QUIT_S` or `_FOUNDER_S` for commands 0, 1, 3 and 14, and calls `0x4d1160(1)` for 19
and 20. Results 1 to 20 show, in order: `ERR_GUILD_INTERNAL`, `ERR_ALREADY_IN_GUILD`, its `_S`,
`ERR_INVITED_TO_GUILD`, `ERR_ALREADY_INVITED_TO_GUILD_S`, `ERR_GUILD_NAME_INVALID`,
`_NAME_EXISTS_S`, `_LEADER_LEAVE` for command 3 else `_PERMISSIONS`, `_PLAYER_NOT_IN_GUILD`, its
`_S`, `_PLAYER_NOT_FOUND_S`, `_NOT_ALLIED`, `_RANK_TOO_HIGH_S`, `_RANK_TOO_LOW_S`, nothing twice,
`_RANKS_LOCKED`, `_RANK_IN_USE`, `ERR_IGNORING_YOU_S`, and `0x4d1160(1)` for command 5.

Addresses: `0x5e6f20`, `0x5e6f80`, `0x5e6fb0`, `0x5e7520`, `0x5e77bc`, `0x5e77a4`, `0x5e77d4`, `0x5e70c0`
Confidence: verified
Proof: one
Test: none

`0x5e7520` switches result 0 by command through the bytes `0x5e77bc` (`cmp 0x14`) into
`0x5e77a4`, and other results by `result − 1` (`cmp 0x13`) through `0x5e77d4`.

## Guild events

0x92 (`0x5e7180`) reads a type, a u8 count and that many strings of at most 0x100 into three
0x100-byte stack slots, unbounded, so a fourth string overruns the frame. Types 0, 1 and 3 to 8 show
the promote, demote, join, leave, remove, leader-is, leader-changed and disbanded messages; 2, the
MOTD, signals `GUILD_MOTD`; 10 renames the rank `atoi(first)` to the second string, unchecked; 12
and 13 show a member signing on or off; 11 does nothing; 9 and any type above 13 show
`ERR_GUILD_INTERNAL`. Every type ends in `0x4d1160(1)` except 12, 13 and a failed gate of 10.

Addresses: `0x5e7180`, `0x5e74dc`, `0x5e74bf`, `0x5e74c9`, `0x5e73a9`, `0x5e744a`, `0x64ac60`, `0x64a480`, `0x40de80`
Confidence: verified
Proof: one
Test: none

The type is read into `ebp−1`, which then serves as the string index; the count is `ebp−2`.
String 4 starts at `ebp−0x18` and reaches `ebp+0xe7`, over the count and index bytes, the saved
registers and the return address, and later strings go further. The messages take the first one,
two or three strings for a count of 1 to 3 and none for any other count, then reach `0x4d1160(1)`
at `0x5e74bf`; 12 and 13 return at `0x5e73a9` and `0x5e744a`, and the failed gates of 10, 12 and
13 jump to `0x5e74c9`, past it. The
switch is `cmp 0xd` through `0x5e74dc`. The MOTD runs the profanity filter `0x4a1a60` unless its
CVar is registered and off, then signals `GUILD_MOTD` with the first string. The rank write needs
the active player and its cached guild record, and is `SStrCopy(record + 0x64 + 64·atoi(s1), s2,
0x40)`. Sign-on (`ERR_FRIEND_ONLINE_SS`, the name twice) and sign-off (`ERR_FRIEND_OFFLINE_S`) each
read a GUID and show only with the active player present, the CVar `guildMemberNotify` on, the
first string different from the active player's name by `SStrCmp` (`0x64a480`, through `strncmp`
`0x40de80`, so case-sensitive), and the GUID not a friend's (`0x5ae810`).

## Guild tabards

`0x5e03f0` saves an emblem: values out of range (style 170, colour 17, border 6, border colour
17, background 51) show `ERR_GUILDEMBLEM_INVALID_TABARD_COLORS`; no guild record `_NOGUILD`; the
emblem the record holds `_SAME`; a rank other than 0 `_NOTGUILDMASTER`; too little money
`ERR_NOT_ENOUGH_MONEY`. Else it sends 0x1f1 with the vendor and five u32, sets `0xc4d780` and
signals `TABARD_SAVE_PENDING`. The reply 0x1f1 shows results 0 to 4, clears `0xc4d780`, and on
success drops the cached guild record.

Addresses: `0x5e03f0`, `0x5e70f0`, `0x85fe88`, `0xc4d780`, `0x5f0800`, `0x5e0720`, `0x5e0650`, `0x5e08e0`
Confidence: verified
Proof: one
Test: none

The record is `0xc0e1ec` by `PLAYER_GUILDID`, emblem at `+0x2e4` to `+0x2f4`; the price is
`0x6d6de0()`. `0x5e70f0` indexes `0x85fe88` for results 0 to 5 and does nothing outside them.
`0x5f0800` returns 1 while the record exists and nothing is pending. `0x5e0720(display)` puts the
tabard on the model and, for a display with `+0x24` bit 0, the emblem through `GetGuildEmblem` with
callback `0x5e0650`. `0x5e08e0` refreshes the name, signals `TABARD_CANSAVE_CHANGED` and the unit
event `PLAYER_GUILD_UPDATE`, and redraws the tabard. `0x5e0830` reapplies the tabard in slot 18,
`0x5e07d0` rebuilds the character component, both then calling `0x491a50`, which jumps to
`0x524e10`; `0x5e09f0` returns the guild's name for the name text.

## The guild record

A guild's cache record holds its id, its name, ten rank names and five emblem values, read in
that order from a stream.

Addresses: `0x62f260`, `0x62f26e`, `0x62f27b`, `0x62f295`, `0x62f2af`, `0x62f2e7`, `0x5552a3`, `0x5612f6`, `0x561963`
Confidence: verified
Proof: one
Test: `dbcache::dbcache_guild`

`0x62f260(stream)` fills the record at `this`: a u32 at `+0` (`0x62f26e`), a string of at most 0x60
bytes at `+4` (`0x62f27b`), ten strings of at most 0x40 bytes from `+0x64`, 0x40 apart (`0x62f295`),
and five int32 at `+0x2e4`, `+0x2e8`, `+0x2ec`, `+0x2f0` and `+0x2f4` (`0x62f2af` to `0x62f2e7`), the
emblem of the tabard section. The callers are the guild info reply (`0x5552a3`) and the two
cache routines (`0x5612f6`, `0x561963`). The test runs `0x62f260` whole.

## Petitions

`0x5e5050` handles six petition opcodes for the active player. 0x1bc lists a vendor's charters: a u8
count of 20-byte entries into a 10-entry table, unbounded, so entry 10 on overwrites the globals
after it, the vendor GUID `0xc4d498` among them. 0x1bf passes the signatures to the window unless
the owner is ignored. 0x1c1 reports a signature: another signer goes to `0x4f41c0`, the player's own
shows `ERR_PETITION_SIGNED` and signals `PETITION_CLOSED`, or the reasons 1, 2, 3, 5. 0x1c2 shows
`ERR_PETITION_DECLINED_S`; 0x1c5 the turn-in result; 0x2c1 renames a charter item's cached petition.
`0x5ef2b0` turns a charter in.

Addresses: `0x5e5050`, `0x5eeb80`, `0x5eee60`, `0x5eefc0`, `0x5ef0e0`, `0x5ef150`, `0x5ef1d0`, `0x5ef2b0`, `0xc4d3d0`
Confidence: verified
Proof: one
Test: none

`0x5eee60` calls `0x4f3f60` and `0x4f40b0`. `0x5eeb80` zeroes 50 dwords at `0xc4d3d0` and opens the
list (`0x4f4fb0`) for a unit with NPC flags 0x200 and 0x400, and only with bit 0 of `0xc4d3e0`,
the first entry's `+0x10`, set (`0x5eec42`). `0x5eee60` allocates `8n` and `4n` on
the stack for the u8 count. The turn-in result: 0 `GUILD_REGISTRAR_CLOSED`, 2
`ERR_PETITION_IN_GUILD`, 4 `ERR_PETITION_NOT_ENOUGH_SIGNATURES`, else "Petition error". `0x5ef2b0`
searches the backpack and bags for an item with flag 0x2000 whose petition (cache `0xc0e0fc`) has
`+0x1110` bit 0, sends 0x1c4 with its GUID, or shows `ERR_NO_GUILD_CHARTER`; a record still pending
retries through the callback `0x5ef520`.

## Reading a petition record

`0x7c7a30` reads a petition record (the record in `ecx`, the store on the stack) in the order `0x7c78f0` writes it: a u32 at `+0`, a GUID at `+8`,
the title into `+0x10` and the text into `+0x110`, eleven values from `+0x1110` to `+0x1138` (the one at `+0x1130` 16-bit),
the choice count at `+0x13bc`, then that many choices from `+0x113c`, 0x40 bytes apart. Every string is copied with
the capacity `0x7fffffff`, so a title, text or choice longer than its field runs into the next, and the loop has no cap
at ten: an eleventh choice lands on the count, which the loop reads again each pass, a client fault. Three readers call it
(`0x555598`, `0x568306`, `0x568973`).

Addresses: `0x7c7a30`, `0x7c7a4e`, `0x7c7a5e`, `0x7c7aea`, `0x7c7b19`, `0x7c7b33`, `0x7c7b43`, `0x7c7b4d`, `0x7c78f0`, `0x555598`, `0x568306`, `0x568973`
Confidence: verified
Proof: one
Test: none

`0x7c7a4e` and `0x7c7a5e` push `0x7fffffff` for the title and the text, and `0x7c7b33` for each choice; `0x7c7aea`
calls `0x418d30` for the 16-bit value. The loop tests `[record+0x13bc]` before the first pass (`0x7c7b19`) and after
each (`0x7c7b43`, `0x7c7b4d`), stepping the destination by 0x40.

## Petition answers

Another player's decline (0x1c2, `0x5ef0e0`) shows `ERR_PETITION_DECLINED_S` only when the decliner's name is already
cached; an uncached name shows nothing. A rename (0x2c1, `0x5ef1d0`) reads the item GUID and a name of up to 0x60 bytes,
returns for an empty name, an item without the signable flag (0x2000) or without a petition id in `ITEM_FIELD_ENCHANTMENT`
slot 0, and otherwise overwrites the cached petition's title and signals `PETITION_SHOW` (390). The offered, signed and
declined lines go to the chat window as system lines; the five refusals go to the red error line. The two signed lines play
`igPlayerInviteAccept` and the declined line and the refusals `igPlayerInviteDecline`.

Addresses: `0x5ef0e0`, `0x5ef100`, `0x5ef12a`, `0x5ef139`, `0x5ef1d0`, `0x5ef200`, `0x5ef240`, `0x5ef248`, `0x5ef27c`, `0x5ef292`, `0x5ef29c`, `0x55f080`, `0x567d60`, `0x496720`, `0x484ca0`, `0x487794`, `0x4877cc`, `0x4877e8`, `0x487804`, `0x487826`, `0x487847`, `0x487868`, `0x487889`, `0x4878aa`
Confidence: verified
Proof: one
Test: none

`0x5ef12a` looks the GUID up in the name cache with no callback; `0x5ef131` skips the line for a miss. The rename's cache read
`0x5ef27c` also passes no callback and sends nothing. Event 0x186 is 390. The catalog rows 0x141 to 0x149 of `0xb4b498` (20
bytes each) are stored by the static initializer `0x484ca0`, which holds 2, 0xa, 0x44 and 0 in registers: the row's route (+4)
is 0 for the four lines (chat) and 2 for the five refusals (the red line), +0x10 is 0xa, the chat type of system lines, and
+8 names the sound, `igPlayerInviteAccept` for 0x142 and 0x143, `igPlayerInviteDecline` for 0x144 to 0x149 and none for
0x141. The route table at `0x496888` is in `ui/inventory#failure-messages`.

## Group and guild messages sent

| sender | opcode | body |
|---|---|---|
| `0x5e9440`, `0x5e9490` | 0x6e | a name (by GUID: the unit's name) |
| `0x5e9510`, `0x5e95d0` | 0x76, 0x75 | a GUID when no unit has it, else its name |
| `0x5e9640` | 0x78 | GUID |
| `0x5e96c0`, `0x5e9720`, `0x5e9780` | 0x72, 0x73, 0x7b | none |
| `0x5e97e0` | 0x7a | u32 method, GUID, u32 threshold |
| `0x5e9870`, `0x5e98e0` | 0x84, 0x85 | none |
| `0x5eec70` | 0x1bd | GUID, then a petition record through `0x7c78f0` |
| `0x5eef40` | 0x1be | GUID |

Addresses: `0x5e9440`, `0x5e9490`, `0x5e9510`, `0x5e95d0`, `0x5e9640`, `0x5e96c0`, `0x5e9720`, `0x5e9780`, `0x5e97e0`, `0x5e9870`, `0x5e98e0`, `0x5eec70`, `0x5eef40`
Confidence: verified
Proof: one
Test: none

Each builds a store with the opcode and sends it through `ClientServices::Send`. `0x5e9490`
first calls `0x4b54c0(0x11)`.
