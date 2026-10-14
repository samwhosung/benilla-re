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

## The name colour

The overhead name has the colour of the unit's selection ring, and a player's name carries its AFK,
DND and GM tags.

Addresses: `0x6c6e90`, `0x6c6f63`, `0x6c6f75`, `0x605960`, `0x5c2750`, `0x5ec9e0`, `0x5eca40`, `0x5eca80`
Confidence: verified
Proof: one
Test: none

The name renderer `0x6c6e90` calls slot 11 (`+0x2c`) of the unit's vtable at `0x6c6f63` with the
descriptor's `+0xc`, and the unit and player vtables both hold `0x605960` there (`unitdisplay`);
`0x6c6f75` hands the colour to the string through `0x5c2750`. The prefix slots 31 to 33 hold the
player's AFK, DND and GM tags at `0x5ec9e0`, `0x5eca40` and `0x5eca80` (`players`); the unit's are
empty.

## The name anchor

The name hangs on a model attachment posed in the current frame: attachment 18 for an unmounted
unit and 29 for a mounted one. A model without it gets the point 1.25 heights above the feet.
`0x608640` rebuilds the point every call, caches nothing and smooths nothing; the name moves with
the head the model draws, never a frame behind it.

Addresses: `0x608640`, `0x608649`, `0x608652`, `0x60865a`, `0x60866d`, `0x608681`, `0x6086a7`, `0x6086b1`, `0x712cb0`, `0x712d50`, `0x714000`, `0x71400c`, `0x615185`, `0x60f7fd`, `0x6c6f00`, `0x6c7439`
Confidence: verified
Proof: one
Test: none

The id is `(−(field ≤ 0) & 0xb) + 0x12`: the mount display id at `[[+0x110]+0x1fc]` (`0x608652`)
tested by `setle` (`0x60865a`) gives 18 or 29. `0x712cb0` asks the model for the attachment and
`0x712d50` takes its bone's palette entry from the model after `0x714000` has run; `0x714000`
returns at once when the model's stamp `+0x40` equals its scene's `+0x10` (`0x71400c`), so the
palette is the one the mesh is skinned with. Without the attachment `0x6086a7` multiplies the unit scale
`+0x90` by the box height `+0xbc` and `0x6086b1` by 1.25 (`0x80c5d0`) and adds it to the base z. The box
height is `max.z − min.z` of the box copied at `0x615183` to `0x61518b`. `0x608640` has three callers:
the plate projection (`0x60f7fd`), the name update (`0x6c6f00`) and `0x6c7439`.

## The show test

A name shows by a fixed order of tests, the first that decides winning. `0x6070a0(mask)` returns 0
when the unit's selectable slot `+0x58` says no, when it has a nameplate (`+0xe60`) or a chat
bubble (`+0xe64`); for the active player it returns bit 0 of the mask; for the current target 1,
whatever the mask; 0 when the byte at field `+0x213` has bit 0 or 1; then bit 2 of the mask for a
player and bit 1 for anything else. The own-unit test comes before the target test, so
targeting oneself still obeys `UnitNameOwn`.

Addresses: `0x6070a0`, `0x6070a8`, `0x6070af`, `0x6070b9`, `0x6070c5`, `0x6070d1`, `0x6070dc`, `0x6070e6`, `0x607101`, `0x607113`, `0x5fa6d0`, `0xb4e2d8`
Confidence: verified
Proof: one
Test: none

The tests in address order: `[unit vtable+0x58]` (`0x6070a8`), `+0xe60` (`0x6070af`), `+0xe64`
(`0x6070b9`), `0x5fa6d0` for the active player's own unit (`0x6070c5`, return `mask & 1`), the GUID
against the target pair `0xb4e2d8` (`0x6070dc`, `0x6070e6`, return 1), `test [field+0x213],3`
(`0x607101`) and the unit type bit 4 of `[[+8]+8]` (`0x607113`) selecting `(mask >> 2) & 1` or
`(mask >> 1) & 1`. The mask is `0xce8720`, whose bits 0x1, 0x2, 0x4, 0x10 and 0x20 the five `UnitName*`
variables set (`playername`); only the first three reach this test.

## No distance cull

No name is dropped for its distance. The descriptor update `0x6c6d40` and the icon queue `0x6c6e00`
compare no float, and the name draw `0x6c6e90` has one float compare, of the unit's name height
against 4 (`0x6c6f1d`), which sets a line step and gates nothing. The name draw's gates are the
unit lookup and the show test.

Addresses: `0x6c6d40`, `0x6c6e00`, `0x6c6e90`, `0x6c6f1d`, `0x6c6f10`, `0x6070a0`
Confidence: verified
Proof: one
Test: none

The census is every `fcom`, `fcomp`, `ucomis` and `comis` in `[0x6c6d40, 0x6c6e90)`: none, and in
`[0x6c6e90, 0x6c7100)`: `0x6c6f1d`, with the control that the same pattern finds `0x6c6f1d`. The operands
at `0x6c6f10` and `0x6c6f13` are the name anchor's z and the unit's base z, so the compared quantity is a
height. The callees of `0x6c6e90` include no camera accessor.

## Nameplates

`0x60f600` runs each frame. A unit gets a plate when it is alive, selectable, not stealthed, in a
visible node, not the camera's own unit, on screen and within 20 yards of it; when enemy or
friendly plates are switched on for its side; and when it is no friendly totem and no enemy
flagged dead. Plates come from a free list and stay sorted by distance from the screen point
(0.4, 0.3).

Addresses: `0x60f600`, `0x60f6f3`, `0x60f72f`, `0x60f750`, `0x60f757`, `0x60f75a`, `0x605570`, `0x60f7b8`, `0x60f7f0`, `0x6086e0`, `0x6087f0`, `0x608870`, `0x6089a0`, `0x608a10`, `0x608a90`, `0x608ce0`, `0x60f830`, `0x60f890`, `0x6054f0`, `0x605510`, `0x605530`, `0x605550`, `0x6122e0`, `0x604ef0`
Confidence: verified
Proof: one
Test: `objectlayer::dist2_to_anchor_difftest`, `objectlayer::follow_anchor_lift_difftest`, `objectlayer::nameplate_pos_changed_difftest`, `objectlayer::follow_dist_proceed_difftest`

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
`0xc4d908`. Slices: follow_dist_proceed_difftest `0x60f798`-`0x60f7c2`.

## Plates under far sight

Under far sight a plate is judged around the camera's unit, and its category by the active player.

Addresses: `0x60f600`, `0x4818f0`, `0x468550`, `0x60f68c`, `0x60f6c3`, `0x60f7c9`, `0x60706a`, `0x607ef9`
Confidence: verified
Proof: one
Test: none

`0x60f600` takes the camera from `0x4818f0` and resolves the unit its attached GUID names, the
camera's unit, and separately the active player from `0x468550`. `0x60f68c` drops the camera's own
unit, `0x60f7c9` measures the squared distance from the subject to the camera's unit, and `0x60f6c3`
makes the active player the observer of both category predicates. The gate runs completely every
tick, from `0x607ef9` in the frame update and from `0x60706a` after a reaction change.

## The plate projection

A plate whose anchor does not project inside the viewport is destroyed, not clamped, and a later
tick creates it afresh. The anchor is the name anchor raised by 2/3 in world z.

Addresses: `0x60f600`, `0x60f773`, `0x60f77a`, `0x60f740`, `0x60f7f0`, `0x60f808`, `0x608a10`, `0x483ee0`, `0x483f7a`, `0x484065`, `0x484075`, `0x484086`, `0x484093`, `0x4840a3`, `0x4840a8`, `0x80abfc`
Confidence: verified
Proof: one
Test: `ui::geo_483ee0`

`0x60f7f0` calls `0x608640`, adds the float at `0x80abfc` (0.6667) and returns the verdict of
`0x483ee0`, which `0x60f773` tests; a failure jumps to `0x60f740`, the pool return `0x608a10`,
which clears `+0xe60`. `0x483ee0` fails when the clip depth is below the camera's near plane
(`0x483f7a`) without writing the point, and when the mapped point leaves the viewport, edges
included (`0x484075` to `0x4840a3`), after writing it (`0x484065`). The projection test runs before
the 20-yard test.

`ui::geo_483ee0` runs `0x483ee0` whole, with the matrix transform `0x7bcb40` stubbed to return a chosen clip vector.

## Seating the plates

A plate is seated again only when one is created or one's screen point moves. The seat walk has no
visibility test of its own, because a plate off screen was dropped by `0x60f600`.

Addresses: `0x6086e0`, `0x6087a7`, `0x6087b6`, `0x6087c0`, `0x6087d5`, `0x6087dd`, `0x60874b`, `0x4840c0`, `0x483635`, `0x483640`, `0x483647`, `0x48364c`, `0x608ce0`, `0x608d37`, `0x509500`, `0x509ec0`
Confidence: verified
Proof: one
Test: none

`0x6086e0` stores the point at plate `+0x508` and `+0x50c` and sorts the plate (`0x6087d5`), then
`0x6087dd` sets bit 0 of the world frame's `+0x3f0` through `0x4840c0`; a new plate does the same at
`0x60874b`. The point counts as changed when the squared difference from the stored one is not
exactly 0 (`0x6087a7`, `0x6087b6`); bit 0x2 of `0xc4da34` skips that test. The paint pass tests
the bit (`0x483635`); when set it empties bucket 0 (`0x509500`), calls `0x608ce0` and clears the
bit (`0x48364c`), so one moved plate seats them all. `0x608ce0` hands each plate's stored point to
`0x509ec0` at `0x608d37`, nearest first.

## Chat bubbles

`0x608ac0(kind, text)` hangs a chat bubble over a unit that has no plate, when the
`ChatBubblesParty` CVar (kind 1) or `ChatBubbles` (others) is on and the active player stands
within 20 yards. A new bubble replaces the old one.

Addresses: `0x608ac0`, `0x608b94`, `0x608c00`, `0x608c20`
Confidence: verified
Proof: one
Test: `objectlayer::bubble_in_range_difftest`

`0x608b94` compares the squared distance with 400 (0x806798). The colour comes from 0x49e990 by
kind; the bubble is made by 0x4b1870 on the world frame 0xb4b2bc and filled by 0x4b1600 with the
GUID, text and colour into `+0xe64`. `0x608c00` releases it. `0x608c20`, which nothing reaches,
returns the position raised by animation 0's height plus 0.7. Slices: bubble_in_range_difftest `0x608b74`-`0x608ba5`.

## Quest markers

`0x607380` clears the marker and, for a unit whose reaction toward the active player is above
hostile, asks the player side about it: a quest giver (NPC flag 0x2) and a flight master (0x8).
A quest giver's status reaches `0x607440(status)`, which maps it to a marker model hung over the
head, playing animation 190 while the name shows. Losing an NPC flag closes the window that NPC
holds open.

Addresses: `0x607380`, `0x6073f0`, `0x607440`, `0x607480`, `0x6074c0`, `0x607570`, `0x6076c0`, `0x603a00`, `0x5fa430`, `0x60b420`
Confidence: verified
Proof: one
Test: `objectlayer::glow_inv_len_difftest`

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

## The marker scale

The marker is scaled once per attach, by the reciprocal of the length of its attachment's first row,
which cancels the unit's scale. A bake that is skipped leaves the marker's base matrix as the
constructor made it, the identity.

Addresses: `0x607570`, `0x607584`, `0x607591`, `0x6075ac`, `0x6075ba`, `0x6075cb`, `0x60761f`, `0x607633`, `0x6076ac`, `0x607553`, `0x60b256`, `0x6074c0`, `0x6074ea`, `0x60ce70`, `0x70dd7f`, `0x712de0`, `0x710620`, `0x604c30`
Confidence: verified
Proof: one
Test: none

`0x607570` returns at `0x607584` without a marker, at `0x607591` before it has a parent, at
`0x6075ac` while the mount model is not loaded, at `0x6075ba` and `0x6075cb` when the body is
absent or not loaded. It takes the attachment matrix of the body (`0x712de0`), forms `L` as the
square root of the sum of the squares of its first three floats, returns when `L` is exactly 1.0
(`0x60761f`) or not above 0 (`0x607633`), and writes a diagonal of 1/L through `0x710620`
(`0x6076ac`). It runs from `0x6074c0` (`0x607553`), whose callers are the marker swap, the
model-loaded handler `0x60ce70` and the mount display watcher, and from the scale-change handler
`0x60b1d0` (`0x60b256`). `0x6074c0` returns at `0x6074ea` while the body is not loaded, and
`0x70dd7f` is where the constructor sets 1.0.

## The cached quest status

A unit caches its last quest status at `+0xcb8`; the status also feeds the minimap and the
interaction tests.

Addresses: `0x5faee5`, `0x607426`, `0x60746a`, `0x607446`, `0x4eac31`, `0x5df4a9`, `0x5f0268`
Confidence: verified
Proof: one
Test: none

The constructor zeroes it (`0x5faee5`), the marker release clears it (`0x607426`) and `0x607440`
stores it (`0x60746a`) after reading the old value to compare mapped markers (`0x607446`). The
other readers are the minimap's object icon (`0x4eac31`), the quest-giver interaction test
(`0x5df4a9`) and the compare with 1 at `0x5f0268`.

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
Test: `objectlayer::pet_dismiss_pos_lift_difftest`

Both skip players. Reactions must reach 3 both ways (0x6061e0). At `0x60c32b` a GUID other than
`0xc4d908` zeroes the count `0xc4d970`; 0x623910 picks a sound from the count, 0x458f90 plays it
into `+0xb1c`, and the count takes the returned value. `0x60c3b0(first)` clears `0xc4d908` and
plays `+0x4` of the `NPCSounds` row (0xc0da6c, by display `+0x2c`) when `first` is set, else
`+0x8`. Opcode 0x13c (`0x604060`: GUID, u32) passes the u32 to `0x6056e0`: 0 calls slot 34 with
8, 2 calls 0x623a40(0). Opcode 0x324 (`0x6040c0`: GUID, u32 0 or 1) calls 0x623a40(1 or 2).
Opcode 0x325 (`0x604140`: u32 model data id, x, y, z) plays the `+0x74` sound of that model's
`CreatureSoundData` row one yard above the point. Slices: pet_dismiss_pos_lift_difftest `0x6041cb`-`0x6041dc`.
