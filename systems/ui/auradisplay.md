# ui: the buff display functions

The interface reads the player's own auras through a cache of 48 records that the client rebuilds from the unit fields, and every other unit's auras straight from its fields or from the group record. This doc is the cache, its rebuild, the
`GetPlayerBuff` family, `CancelPlayerBuff`, and `UnitBuff` and `UnitDebuff` with the gate that hides a creature's buffs. The aura fields themselves are `objectlayer/fields`; the tooltips of these auras are `tooltipbuff`; the cancel packet is `spell/cancel`.

Roots: `0x4e4100`, `0x4e4170`, `0x4e45d0`, `0x4e49a0`, `0x519500`, `0x5198f0`, `0x519860`

## The player's buff cache

`0xbc6040` holds 48 records of 16 bytes, one per displayed aura, packed from the front. A record is: `+0x0` the aura slot 0 to 47 (-1 for empty), `+0x4` the spell id, `+0x8` the caster level byte, `+0x9` the stack count (the applications byte plus one), `+0xa` the slot's flag nibble and `+0xc`
the until-cancelled flag. The aura fields it reads are, from the unit fields: spell ids at `+0xa4`, flag nibbles at `+0x164` (two to a byte, the low nibble for an even slot), levels at `+0x17c`, applications at `+0x1ac`. Slots 0 to 31 are buffs and 32 to 47 debuffs.

Addresses: `0xbc6040`, `0x4e44b0`, `0x4e44bc`, `0x4e44d6`, `0x4e44e6`, `0x4e44f6`, `0x4e4504`, `0x4e4528`, `0x4e4490`, `0x4e4070`, `0x4e4430`, `0x4e4100`, `0x4e4140`, `0x4e4150`, `0x490983`, `0x490aad`, `0x467e70`
Confidence: verified
Proof: one
Test: none

`0x4e44b0(record, slot, player)` fills a record, or stores slot -1 and spell 0 for a slot of -1. `0x4e4100` watches the active player's unit fields from `+0xa4` for 0x138 bytes, the four arrays above (`0x4e4100` is called from the interface setup `0x490983` and `0x4e4150` removes the watcher at `0x490aad`),
runs `0x4e4170` once and clears the tracking word `0xbc6378`. The watcher callback `0x4e4140` runs `0x4e4170` and returns 1. `0x4e4430(index)` returns the record, or null for an index of 48 or more or below 0.

## Rebuilding the cache

`0x4e4170` runs when the aura fields change. First it walks the records from the front and ends at the first one with no spell. A record whose slot holds another spell id, or whose flag nibble has no bit of 0xe, is removed: the later records shift down one place and the last record is emptied.
Then it walks slots 0 to 47 and, for each with a spell and a nibble bit of 0xe, appends a record at the first empty place, or refreshes the stack count of the record that already has that spell and slot. It then signals `PLAYER_AURAS_CHANGED` (event 192).

Addresses: `0x4e4170`, `0x4e41b0`, `0x4e41ee`, `0x4e4217`, `0x4e4223`, `0x4e4237`, `0x4e4260`, `0x4e4299`, `0x4e42a2`, `0x4e42b6`, `0x4e42c1`, `0x4e42d6`, `0x4e4302`, `0x4e430c`, `0x4e4332`, `0x4e433b`, `0x4e4356`, `0x4e437f`, `0xbc6378`, `0xc0d788`, `0xc0d78c`
Confidence: verified
Proof: one
Test: none

A spell is left out when its `Attributes` low byte has bit 7 (`js` after a byte load, `0x4e42b6`) or its `AttributesEx` has 0x10000000 (`0x4e42c1`). It is also left out, with its id stored in `0xbc6378`, when one of the three effect aura names at `+0x16c` is 0x2c, 0x2d or 0x97 (tracking), so `GetPlayerBuff`
never returns a tracking aura. The record positions are not the slots: a position is the insertion order and falls when an earlier record is removed. A full cache (48) drops the aura.

## GetPlayerBuff

`GetPlayerBuff(index [, filter])` raises `Usage: GetPlayerBuff(index [, "filter"])` for a non-number index and pushes two numbers: the record position of the index-th record that passes the filter (0-based), or -1, and the record's until-cancelled flag, or 0. The filter is a string of tokens split at spaces and pipes:
`HELPFUL` (1), `HARMFUL` (2), `CANCELABLE` (0x10) and `NOT_CANCELABLE` (0x20), case-insensitive. With no filter argument the mask is 3. A filter string with no known token gives 0 and nothing passes.

Addresses: `0x4e45d0`, `0x4e460a`, `0x4e4618`, `0x4e4640`, `0x4e4651`, `0x4e46e2`, `0x4e46f1`, `0x4e46f6`, `0x4e470e`, `0x4e43b0`, `0x4e43c9`, `0x4e43dd`, `0x4e43f9`, `0x4e4415`
Confidence: verified
Proof: one
Test: none

`0x4e43b0(index, mask, &position)` walks all 48 records in order, skips empty ones, tests slot below 0x20 against bit 1 and below 0x30 against bit 2, then the cancelable bit (`+0xa` bit 0) against 0x10 and 0x20, and counts the passing ones until the count equals the index. Bit 0 of the flag nibble is the cancelable bit; the mask has no `PASSIVE` token. The other accessors take the position it returns.

## Texture, dispel type, stacks and time

`GetPlayerBuffTexture(position)` pushes the icon path of the record's spell (Spell.dbc icon id at `+0x1d4`, then the `SpellIcon` row's path at `+4`), or nil. `GetPlayerBuffApplications(position)` pushes the stack count of `+0x9`, or the number 1 for no record.
`GetPlayerBuffDispelType(position)` pushes the string in column 11 of the `SpellDispelType` row of the spell's dispel id (`+0x10`), or nil for an id of 0, a missing row or a row whose word `+0x28` is 0. `GetPlayerBuffTimeLeft(position)` pushes the time left in seconds (`0x4e4450` times 0.001), 0 for no record.

Addresses: `0x4e4740`, `0x4e47b7`, `0x4e47e4`, `0x4e48b0`, `0x4e48f8`, `0x4e4917`, `0x4e4800`, `0x4e485b`, `0x4e4879`, `0x4e4880`, `0x4e4930`, `0x4e496f`, `0x4e4986`, `0x4e4450`, `0xc0d7e4`, `0xc0d7ec`, `0xc0d7f0`, `0xc0d83c`, `0xc0d840`, `0x801608`
Confidence: verified
Proof: one
Test: none

`GetPlayerBuffApplications` raises `Usage: GetPlayerBuffTimeLeft(buffIndex)`, the usage text of its neighbour (`0x4e48c5`). Column 11 is the one string column of `SpellDispelType` outside the eight locale names (`tables/dbc_schemas.tsv`), so the string does not change with the locale.
`0x4e4450(position)` is the expiry in `0xbc5f68` less the current time in milliseconds, 0 once past or for an empty record.

## The until-cancelled flag

A record's flag at `+0xc` is 1 when the spell shows no finite duration. `0x4e44b0` sets it to 1 when the spell's duration row (Spell.dbc `+0x78` into `SpellDuration`) is missing, or when the row's first word is negative and its second is 0 or less; and, for an aura whose nibble has no cancelable bit, also when
one of the spell's three effect ids at `+0xf4` is 0x23, 0x77, 0x80 or 0x81. The stock buff frame hides the duration text for a record with the flag set. The flag does not read the expiry array.

Addresses: `0x4e44b0`, `0x4e4552`, `0x4e4559`, `0x4e4567`, `0x4e456e`, `0x4e4585`, `0x4e4588`, `0x4e4590`, `0x4e45be`, `0xc0d828`, `0xc0d82c`
Confidence: verified
Proof: one
Test: none

The row is `[row+4] < 0 and [row+8] <= 0` for the infinite row (`0x4e456e`, `0x4e4575`). The four effect ids are the area-aura effects; they only matter for an aura without the cancelable bit.

## The duration packet

The server sends a duration for an aura slot in the packet `0x137` (`u8 slot, i32 milliseconds`). `0x4e4390` stores the current time plus the milliseconds at `0xbc5f68` indexed by the slot, for a slot below 48 and nothing for a larger one. The array is keyed by the slot and not by the record position;
it is zeroed only at interface setup (`0x4e40c0`), so a slot keeps its last expiry when its aura ends or changes, and an aura that never gets a duration reads the previous one's.

Addresses: `0x4e4390`, `0x4e4393`, `0x4e439b`, `0x4e43a6`, `0x4e40c0`, `0x48f5a9`, `0xbc5f68`, `0x5e3bc8`
Confidence: verified
Proof: one
Test: none

`0x4e43a6` is the only store to the array besides the clearing loop. The packet handler is `objectlayer/playermessages`. The setter can run before the field update that gives the slot its spell, since the two stores are independent.

## CancelPlayerBuff and tracking

`CancelPlayerBuff(position)` raises its usage for a non-number and does nothing for no record. It cancels when the record's slot is below 32 and its nibble has the cancelable bit, or when the spell's `AttributesEx` (`+0x1c`) has bit 0x4. Cancelling is `0x6e7040(spellId)`, which sends the spell id.
`GetTrackingTexture()` pushes the icon path of the spell stored in `0xbc6378`, or nil, and `CancelTrackingBuff()` cancels that spell when it is nonzero.

Addresses: `0x4e49a0`, `0x4e49df`, `0x4e49fd`, `0x4e4a01`, `0x4e4a06`, `0x4e4a10`, `0x4e4a16`, `0x6e7040`, `0x4e4a20`, `0x4e4a3e`, `0x4e4a5d`, `0x4e4a80`, `0x4e4a86`, `0x4e4a8a`
Confidence: verified
Proof: one
Test: none

The second test is the only one for a debuff slot. `0xbc6378` holds the last tracking spell met in slot order by the rebuild and is cleared by the setup call.

## UnitBuff and UnitDebuff

`UnitBuff(unit, index [, filter])` and `UnitDebuff(unit, index [, filter])` raise `Usage: UnitBuff("unit", index [,raidFilter])` (and the matching text) for a non-string unit or non-number index. The index is 1-based within the half and counts only the slots that pass: 1 to 32 for buffs from slots 0 to 31, 1 to 16 for debuffs from slots 32 to 47.
`UnitBuff` pushes the icon path and the stack count (applications byte plus one), `UnitDebuff` also the dispel string; a missing icon pushes nil and 0 (and nil), and no match or an index out of range pushes a single nil. The third argument is read as a script flag (`actionuse#reading-a-script-flag`).

Addresses: `0x519500`, `0x519558`, `0x51957a`, `0x51958b`, `0x51960f`, `0x519621`, `0x51962c`, `0x51964e`, `0x519656`, `0x51966d`, `0x51967b`, `0x5196b1`, `0x5196e8`, `0x5196ac`, `0x5198f0`, `0x51996d`, `0x519979`, `0x5199ab`, `0x5199c6`, `0x5199ed`, `0x519a0c`, `0x519a1a`, `0x519a62`, `0x519a82`, `0x519ac4`, `0x6f1c10`
Confidence: verified
Proof: one
Test: none

The walk reads the unit's own fields, never the cache: for each slot it requires a spell id above 0, a nibble bit of 0xe, a spell row and `0x519860`, then the third-argument test. `UnitBuff`'s walk covers the byte offsets `0xa4` to `0x124` and `UnitDebuff`'s `0x124` to `0x164`. The unit is looked up with type mask 8.

## Who sees a unit's buffs

`UnitBuff` lists a held unit's buffs only when the unit has `UNIT_FIELD_FLAGS` bit 27 (0x08000000), or the active player can assist it and is not charmed. Otherwise it pushes a single nil for every index, so a hostile or neutral creature, and a friendly one not flagged for PvP, shows no buffs. `UnitDebuff` has no such test and lists debuffs for any unit.

Addresses: `0x5195aa`, `0x5195b6`, `0x5195bc`, `0x5195d4`, `0x5195dd`, `0x5195e2`, `0x5195e9`, `0x5195eb`, `0x5195f7`, `0x5195f9`, `0x6066f0`, `0x5199a5`
Confidence: verified
Proof: one
Test: none

`0x6066f0` (`objectlayer/reaction#assisting-cooperating-and-interacting`) refuses an unselectable unit and a reaction below 4; for a unit that is not player-controlled it passes when the player is not player-controlled and otherwise answers the PvP flag of the unit's charmer or creator, else its own. The charm test is the active player's own `UNIT_FIELD_CHARMEDBY`
(`0x5195eb` to `0x5195f7`): while the player is charmed, only the flagged unit shows buffs. The hidden and the empty answers are the same single nil (`0x5195fe`).

## The per-slot display test

`0x519860(spell)` hides an aura of a spell with `Attributes` low byte bit 7 or `AttributesEx` bit 0x10000000, the same two tests as the cache rebuild, and one with a tracking aura name (0x2c, 0x2d or 0x97). A spell whose Spell.dbc word at `+0x2b0` is nonzero is shown only when the active player exists and bit (word minus 1) of the player block byte `+0x10c1`
is set. A non-zero third argument adds one test: `UnitBuff` needs the spell to be a buff the player could cast (`0x4b3870`), `UnitDebuff` needs its dispel id to be in the player's dispel list (`0x4b3920`).

Addresses: `0x519860`, `0x519863`, `0x51986a`, `0x519873`, `0x519893`, `0x5198a2`, `0x5198b0`, `0x5198b8`, `0x5198c2`, `0x4b3870`, `0x4b3893`, `0x4b38a0`, `0x4b38cc`, `0x4b38d4`, `0x4b38e5`, `0x4b3920`, `0x4b3933`, `0xb710fc`, `0xb6f098`, `0xb71110`, `0xb71114`, `0xb71118`
Confidence: verified
Proof: one
Test: none

The byte test (`test cl,dl`) sees only the low 8 bits of `1 << (n - 1)`, so a nonzero word above 8 always hides the aura. `0x4b3870` is true for a spell the player knows (the known-spell bits `0xb710fc`, else the 1024-entry pet list `0xb6f098`) whose row has `+0x24` bit 0x4000 clear, `+0x148` not 1 and no effect id of 0x23, 0x77, 0x80, 0x81 or 0x1b.
`0x4b3920(id)` scans `0xb71118` for `0xb71114` entries.

## Group members out of range

For a token with no unit object, or an object that is released (`0x614ea0` bit 0), both functions read a spell id array from the group record instead: the member record `0x496400` finds, its aura words at `+0x1a`, or the pet record `0x496420` finds, its words at `+0x64`. A member record whose status byte `+0x8` has bit 0 clear gives nil without trying the pet record.
The words are 16 bits, indexed 0 to 31 for buffs and 32 to 47 for debuffs; the same tests apply, with no flag nibble. This path has no visibility test, and the stack count it pushes is the number 1.

Addresses: `0x519735`, `0x519741`, `0x51974b`, `0x519750`, `0x519760`, `0x519780`, `0x5197cc`, `0x5197e7`, `0x519b25`, `0x519b31`, `0x519b70`, `0x519bbf`, `0x519bda`, `0x496400`, `0x496420`, `0x614ea0`
Confidence: verified
Proof: one
Test: none

A record that exists but whose status is not online gives nil (`0x519745`, `0x519b35`) and does not try the pet table. The record layout is `objectlayer/party#the-party-member-record`.
