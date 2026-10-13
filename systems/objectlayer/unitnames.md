# objectlayer: unit names and markers

What a unit shows over its head and how it is named: the name lookup for players, pets and
creatures, the text block a name descriptor draws, the nameplates and their pool, chat bubbles,
the quest markers, the damage and miss numbers, and the greeting and farewell sounds.
A unit's name descriptor sits at `+0xc7c`, its nameplate at `+0xe60`, its chat bubble at `+0xe64`
and its marker model at `+0xb2c`.

Roots: `0x609210`, `0x608f50`, `0x60f600`, `0x608ac0`, `0x607440`, `0x6128b0`, `0x60c270`

## Unit names

`0x609210(realm)` names a unit. A player's name comes from the name cache, which queries the
server on a miss; a pet's from the pet name cache, refreshed when `UNIT_FIELD_PET_NAME_TIMESTAMP`
moves; a creature's from its creature record. A miss reads `UNKNOWNOBJECT`, or `Unknown Being`
(0x860fa4) when that text is empty. Name and creature replies release the nameplate so it is rebuilt.

Addresses: `0x609210`, `0x60926b`, `0x6092d0`, `0x6094c0`, `0x5fb840`, `0x604600`, `0x604660`, `0x604aa0`, `0x612bf0`
Confidence: verified
Proof: one
Test: none

`0x609210` asks 0xc0e228 (0x55f080, callback `0x5fb840`) for a record whose `+0x30` string
`0x60926b` stores in `*realm` when not empty. With
`UNIT_FIELD_PETNUMBER` set it asks 0xc0e138; at `0x6092d0` a record whose `+0x54` differs from the
timestamp is dropped (0x5670d0) and asked for again, and the call returns the unknown text. Others
return the first field of `+0xb30`. `0x6094c0` returns the creature record's subname `+0x10` for a
creature with no pet number. `0x5fb840` and `0x604660` release the plate and refresh the unit's
frames (0x491a00); `0x604660` also stores the record at `+0xb30` and signals
`UNIT_CLASSIFICATION_CHANGED`. `0x604600`, the `OBJECT_FIELD_ENTRY` watcher, asks the creature
cache 0xc0e354. `0x604aa0` drops the pet's cache entry, asks again, and signals
`LOCALPLAYER_PET_RENAMED` when the charmer, else the summoner, is the active player.
`0x612bf0(key)` returns the script text `key` in variant 3 for a female unit (the active player's
gender, else `UNIT_FIELD_BYTES_0` byte 2) or 2, falling back to variant 0.

## The name block

`0x608f50(flags, out, size)` builds what a name descriptor draws, as up to eight parts: three
colour prefixes (vtable slots 31 to 33), the name, a player's guild or a creature's subname, a
foreign server label, and a pet's owner title. It returns the count of extra lines. `0x609370`
gives the name, with a PvP rank or the civilian title when asked.

Addresses: `0x608f50`, `0x609085`, `0x609122`, `0x6091e6`, `0x609370`, `0x6070a0`, `0x608640`, `0x606fe0`, `0x607030`, `0x607050`, `0x607080`, `0x608f40`
Confidence: verified
Proof: one
Test: none

Buffers: three of 32 bytes, the name and two lines of 1024, the guild of 100, the label of 8, the
title of 128, each filled within its size. Flag 0x20 asks
`0x609370` for the rank; flag 0x10 adds the guild (`0x609085`) as `\n<%s>` (0x860f9c). A
creature (`0x609122`) adds its subname and 0x52fd30's owner title. A player with a server name
adds `FOREIGN_SERVER_LABEL` when 0x5efea0 allows. `0x6091e6` joins the parts with
`%s%s%s%s%s%s%s%s`. `0x609370(buf, size, rank, kind)`: a player with rank byte `+0x1f` gets
`UNIT_PVP_NAME` with 0x5efe60's rank name, then a new line and `PVP_MEDAL%d` for byte `+0x1e`
(kind 2); a civilian (`0x612550`) gets `PVP_RANK_CIVILIAN` and a space first. The unit's slots
31 to 33 return nothing. `0x6070a0(mask)` lets the descriptor show a selectable unit with no
plate and no bubble: itself by mask bit 0, the target always, a player by bit 2, others by bit 1,
none with `UNIT_FIELD_BYTES_1` byte 3 bits 0x3. `0x608640` places the name at attachment 29
when mounted, else 18, else 1.25 heights above the feet (0x80c5d0). `0x606fe0` makes the
descriptor, `0x607030` frees it, `0x607050` dirties it and re-checks the plate, `0x607080`
redraws both; `0x608f40`, which nothing reaches, dirties it.

## Nameplates

`0x60f600` runs each frame. A unit gets a plate when it is alive, selectable, not stealthed, in a
visible node, not the camera's own unit, on screen and within 20 yards of it; when enemy or
friendly plates are switched on for its side; and when it is no friendly totem and no enemy
flagged dead. Plates come from a free list and stay sorted by distance from the screen point
(0.4, 0.3).

Addresses: `0x60f600`, `0x60f6f3`, `0x60f72f`, `0x60f750`, `0x60f757`, `0x60f75a`, `0x605570`, `0x60f7b8`, `0x60f7f0`, `0x6086e0`, `0x6087f0`, `0x608870`, `0x6089a0`, `0x608a10`, `0x608a90`, `0x608ce0`, `0x60f830`, `0x60f890`, `0x6054f0`, `0x605510`, `0x605530`, `0x605550`, `0x6122e0`, `0x604ef0`
Confidence: verified
Proof: one
Test: none

Friendly is cooperative and not attackable for a player, not attackable for others. At `0x60f6f3`
bit 0x1 of `0xc4da34` admits enemies and bit 0x8 friends (`0x6054f0`, `0x605510`); a created,
unsummoned unit with `UNIT_FIELD_FLAGS` bit 9 is skipped. A friend (`0x60f72f`, `0x60f731`) goes
to `0x60f750`, which takes `0x605570`'s creature type; `0x60f757` compares it with 11, a totem,
and `0x60f75a` skips it; an enemy with dynamic flag 0x20 is skipped too. `0x60f7b8` compares the squared distance with `0xc4d988` (20², from 0x5fa130).
`0x60f7f0` projects the name position plus 2/3. `0x6086e0` takes a plate (`0x6087f0`: the free
list `0xc4d918`, else a new 0x518-byte frame), drops the chat bubble, and re-sorts it on the
list `0xc4d924` (`0x608870`, keyed by `0x6089a0` from `0xc4da50`) when it moves or bit 0x2 asks. `0x608a10` puts it
back. `0x608ce0` lays them out, `0x60f830` shows or hides them, `0x60f890` releases all.
`0x605530` and `0x605550` set bits 0x2 and 0x4. `0x6122e0` flashes a plate red for 5 seconds
(0x7cba70); `0x608a90` sets its icon from 0x4bb190. `0x604ef0` frees the pool and clears
`0xc4d908`.

## Chat bubbles

`0x608ac0(kind, text)` hangs a chat bubble over a unit that has no plate, when the
`ChatBubblesParty` CVar (kind 1) or `ChatBubbles` (others) is on and the active player stands
within 20 yards. A new bubble replaces the old one.

Addresses: `0x608ac0`, `0x608b94`, `0x608c00`, `0x608c20`
Confidence: verified
Proof: one
Test: none

`0x608b94` compares the squared distance with 400 (0x806798). The colour comes from 0x49e990 by
kind; the bubble is made by 0x4b1870 on the world frame 0xb4b2bc and filled by 0x4b1600 with the
GUID, text and colour into `+0xe64`. `0x608c00` releases it. `0x608c20`, which nothing reaches,
returns the position raised by animation 0's height plus 0.7.

## Quest markers

`0x607380` clears the marker and, for a unit whose reaction toward the active player is above
hostile, asks the player side about it: a quest giver (NPC flag 0x2) and a flight master (0x8).
A quest giver's status reaches `0x607440(status)`, which maps it to a marker model hung over the
head, playing animation 190 while the name shows. Losing an NPC flag closes the window that NPC
holds open.

Addresses: `0x607380`, `0x6073f0`, `0x607440`, `0x607480`, `0x6074c0`, `0x607570`, `0x6076c0`, `0x603a00`, `0x5fa430`, `0x60b420`
Confidence: verified
Proof: one
Test: none

The reaction must top 1 (0x6061e0); `UNIT_NPC_FLAGS` bit 1 calls 0x5eb030 and bit 3 0x5eb170,
which sends a message. The status handler 0x5dc9f0 reads a GUID and a u32 and calls `0x607440`
only for a unit with flag 0x2; the handler 0x5ecdd0 reads a GUID and a u8 and, for flag 0x8, calls
`0x607480` with marker 4 (green) when the u8 is 0, else 0.
`0x80c454` maps statuses 0 to 7 to markers 0, 3, 0, 2, 7, 1, 6, 6; `0x607440` reloads only when
the marker changes. `0xc4d9d8` holds eight model and path pairs that `0x5fa430` fills
(`TalkToMe.mdx` for 1, `TalkToMeQuestion_Grey` 2, `TalkToMeGrey` 3, `Green` 4, `Blue` 5,
`QuestionMark` 6, `Question_LTBlue` 7) and `0x603a00` loads or closes. `0x6074c0` hangs it at
attachment 29 when mounted and present, else 18; `0x607570` undoes the parent's scale;
`0x6076c0` plays 190 when 0x6c7950 says the name shows, else 0. `0x60b420(old)` closes, for a
lost flag: 0x1 the gossip (GUID 0xbc3f58, 0x4e1fa0), 0x2 the quest (0xbe0810, 0x501130), 0x8 the
taxi (0xbb4ab8, 0x4dc0f0), 0x10 the trainer (0xb73a00, 0x4d7440), 0x100 the one at 0xbdd038
(0x4f84d0); bits 0x2 and 0x8 also refresh the marker.

## Damage and miss numbers

`0x6128b0(source, amount, spell, crit)` floats a damage number over the unit, never over the
active mover, for the player's or the pet's hits while the `CombatDamage` CVar is on. Spells show
gold; the player's melee shows the default colour and the pet's orange, each pet kind behind its
own CVar. `0x607140` floats a miss word the same way. `0x60a0c0` and `0x6072e0` show XP and honor.

Addresses: `0x6128b0`, `0x612928`, `0x607140`, `0x6071e7`, `0x607230`, `0x607260`, `0x6072e0`, `0x60a0c0`
Confidence: verified
Proof: one
Test: none

A spell counts as melee when absent or with `AttributesEx3` bit 15. `0x607230` maps 0x5efea0's 0
(the player) to 1 and 1 (the pet) to 0, else shows nothing. At `0x612928` a spell takes
`0xc4d8a0` (0xffffde00), a pet's spell needing `PetSpellDamage` (`0xc4d99c`); the player's melee
takes no colour; a pet's melee needs `PetMeleeDamage` (`0xc4d9cc`) and takes `0xc4d8cc`
(0xffff8400). The text is `%d` in 32 bytes, kind 2 for a crit else 0, through 0x6c7840 on `+0xc7c`;
`CombatDamage` is `0xc4d944`. At `0x6071e7` the miss word is the script text named by `0x86582c`
(`NONE`, `MISS`, `RESIST`, `DODGE`, `PARRY`, `BLOCK`, `EVADE`, `IMMUNE` twice, `DEFLECT`,
`ABSORB`, `REFLECT`) and its kind `0x80c48c` (6, then 3, `ABSORB` 1). `0x607260(xp)` shows
`%s: %d` of `XP` as kind 4; `0x6072e0(sign, a, b)` shows `HK` or `DK` with `b` as kind 5.
`0x60a0c0(xp, a, b)` stores `+0xc80` to `+0xc88`, sets bit 0x8 of `+0xc58` and runs 0x6251e0
before and after.

## Greetings and unit sounds

`0x60c270` plays a creature's greeting when the player clicks it: alive, not already speaking,
neutral or better both ways with the player's mover, the player alive. Clicking the same unit
again steps through its greetings; another unit starts over. `0x60c3b0` plays a farewell from the
display's `NPCSounds` row. Three server messages play unit sounds.

Addresses: `0x60c270`, `0x60c32b`, `0x60c3b0`, `0x6056e0`, `0x604060`, `0x6040c0`, `0x604140`
Confidence: verified
Proof: one
Test: none

Both skip players. Reactions must reach 3 both ways (0x6061e0). At `0x60c32b` a GUID other than
`0xc4d908` zeroes the count `0xc4d970`; 0x623910 picks a sound from the count, 0x458f90 plays it
into `+0xb1c`, and the count takes the returned value. `0x60c3b0(first)` clears `0xc4d908` and
plays `+0x4` of the `NPCSounds` row (0xc0da6c, by display `+0x2c`) when `first` is set, else
`+0x8`. Opcode 0x13c (`0x604060`: GUID, u32) passes the u32 to `0x6056e0`: 0 calls slot 34 with
8, 2 calls 0x623a40(0). Opcode 0x324 (`0x6040c0`: GUID, u32 0 or 1) calls 0x623a40(1 or 2).
Opcode 0x325 (`0x604140`: u32 model data id, x, y, z) plays the `+0x74` sound of that model's
`CreatureSoundData` row one yard above the point.
