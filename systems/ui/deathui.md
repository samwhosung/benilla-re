# ui: death, the corpse run and resurrection

What the interface functions of a dead player read: the release countdown, the reclaim delay, the corpse position and the range events, the soulstone, the resurrection offer and the sickness duration. The spirit healer dialog, the resurrect request packet and the corpse object are `objectlayer/interaction`, `objectlayer/playermessages` and `objectlayer/corpses`; the area spirit healer and the instance boot timer are `popups`.

Roots: `0x4962a0`, `0x496320`, `0x491f50`, `0x492130`, `0x48d260`, `0x48ac80`, `0x48ad70`, `0x495850`, `0x51a3a0`

## The release timer

`GetReleaseTimeRemaining()` pushes the seconds left until the dead player is released, -1 when the map has no timer and 0 when the timer is unset or past. The client arms it: when the active player dies, `0x5e99b0` tests bit 3 of byte 0 of `PLAYER_FIELD_BYTES` (player block `+0x1028`) and, if it is set, stores the current time plus 360000 milliseconds at `0xb4e340` (`0x496250`), else sets the no-timer byte `0xb4e344` (`0x496280`).
The countdown is not on the wire.

Addresses: `0x48b5a0`, `0x48b5a7`, `0x48b5ae`, `0x4962a0`, `0x4962a9`, `0x4962af`, `0x496250`, `0x496260`, `0x496269`, `0x496280`, `0x5e99b0`, `0x5e9a21`, `0x5e9a27`, `0x5e9a30`, `0x5e9a35`, `0x5e9a3c`, `0x48fbf0`, `0x490131`, `0x490137`, `0xb4e340`, `0xb4e344`
Confidence: verified
Proof: one
Test: none

`0x4962a0` returns -1000 while the no-timer byte is set and otherwise the time left, or 0; the binding divides by 1000 with a signed multiply by `0x10624dd3`. A sum of 0 is stored as 1 so that 0 stays the unset value. The constant 0x57e40 is the only use of that value in the image. The game-start block `0x48fbf0` zeroes the timer, its no-timer byte and the reclaim delay (`0x490131`, `0x490137`, `0x49013e`).

## The reclaim delay

`GetCorpseRecoveryDelay()` pushes the whole seconds left of `0xb4e348`, 0 when unset or past. The cell is set by the packet 0x269 (`u32 milliseconds`): `0x4962d0` stores the current time plus the delay (1 for a sum of 0), and when the in-range latch `0xb4e324` is set it signals `CORPSE_IN_RANGE` or `CORPSE_IN_INSTANCE` again.

Addresses: `0x48b5e0`, `0x48b5e9`, `0x48b5f0`, `0x496320`, `0x4962d0`, `0x4962d3`, `0x4962ec`, `0x496307`, `0x48f819`, `0x48f820`, `0x48f828`, `0xb4e348`, `0xb4e324`, `0x48f690`
Confidence: verified
Proof: one
Test: none

The binding divides unsigned (`mul`, `shr 6`). The packet is one of the handlers of `0x48f690`, registered by `CGGameUI::InitializeGame` (`0x48f480`, `0x48f4aa`). The re-signal lets the interface refresh its reclaim button when the delay arrives while the corpse is already in range.

## The corpse location

A ghost asks for its corpse with the empty packet 0x216, sent by `0x491f50` when the player has the ghost bit (`PLAYER_FLAGS` bit 4) set. The function first resets the cache (`0x492010(-1, 0, 0, 0, -1)`) and the range latch. The answer is `u8 found`, `i32 mapId`, `f32 x, y, z`, `i32 corpseMapId`; with found set and the player a ghost it is stored by `0x492010` at `0xb4e31c`, `0xb4e284` to `0xb4e28c` and `0xb4e320`,
otherwise the cache is reset.

Addresses: `0x491f50`, `0x491f74`, `0x491f7b`, `0x491fa1`, `0x491fb2`, `0x492010`, `0x492019`, `0x492042`, `0x492062`, `0x492072`, `0x48f734`, `0x48f764`, `0x48f776`, `0x48f7d1`, `0x48f7e3`, `0x48f800`, `0x4920f0`, `0x6dacd0`, `0x5ea700`, `0xb4e31c`, `0xb4e284`, `0xb4e288`, `0xb4e28c`, `0xb4e320`
Confidence: verified
Proof: one
Test: none

`0x491f50` runs at world entry (`0x490a28`) and from the `PLAYER_FLAGS` watcher at `0x5eeac1`; it is neither periodic nor sent on a zone change. `0x492010` also moves the map blip: to the corpse position when the map id equals the player's own map (`0x5ea700`), else to nothing. The two map ids differ for a corpse inside an instance, where the first is the map the arrow is drawn on.

## Corpse range events

Each frame the world frame update calls `0x492130` (`0x4818c5`, in the unconditional tail of `0x481790`). With a cached corpse and the active player it takes the squared 3D distance from the player to the cached position, summed as `(dz² + dx²) + dy²` (`0x492188` to `0x492194`), and compares it with `0xb4e2ac`; inside or equal is range 1. `0x4920f0(state)` signals only on a change: `CORPSE_OUT_OF_RANGE` for 0,
and for 1 `CORPSE_IN_RANGE` when the cached map ids are equal, else `CORPSE_IN_INSTANCE`.

Addresses: `0x492130`, `0x492136`, `0x492169`, `0x492188`, `0x492196`, `0x4921a2`, `0x4921b0`, `0x4920f0`, `0x4920f6`, `0x492100`, `0x49211a`, `0x492124`, `0x4818c5`, `0x481790`, `0x48d060`, `0x48d070`, `0x48d076`, `0x48d07c`, `0xb4e2ac`, `0xb4e324`, `0x8044cc`
Confidence: verified
Proof: one
Test: `ui::glue_492_492130_sqdist_radius`

`0xb4e2ac` is 1600.0: the static initialiser `0x48d070` squares the 40.0 at `0x8044cc`, so the radius is 40 yards and the compare is `fcomp` then `test ah,0x41; jp` (equal counts as in range). One threshold serves both directions, so there is no hysteresis. Without a cached corpse (`0xb4e31c` below 0) or a player the range is 0.

## RetrieveCorpse

`RetrieveCorpse()` sends opcode 0x1d2 with the corpse GUID at `0xb4e328`, the 8-byte GUID, low dword first (`0x418370` stores two raw dwords at `0x4183b4` and `0x4183ba`, then `add [esi+0x10],8`), and pushes nothing. It tests no distance, no delay, no ghost bit and not even a zero GUID; the gates are in the interface and in the server.

Addresses: `0x48d260`, `0x48d269`, `0x48d28b`, `0x48d290`, `0x48d2a0`, `0x48d2ab`, `0xb4e328`, `0xb4e32c`, `0x4920d0`
Confidence: verified
Proof: one
Test: none

The GUID cell is written only by the corpse object's own publishing (`objectlayer/corpses#the-players-own-corpse`) through `0x4920d0`.

## Soulstone and self-resurrection

`HasSoulstone()` answers only while `UNIT_FIELD_HEALTH` is 0 or less. It then reads `PLAYER_SELF_RES_SPELL` (player block `+0x1030`): a spell id in range pushes the spell's name (Spell.dbc `+0x1e0` plus 4 times the locale index), an id out of range or a missing row pushes `UNKNOWN`, and 0 asks for a carried self-resurrect item and pushes its name; no item gives nil.
`UseSoulstone()` has no health test, reads no argument and pushes nothing: with the field set it sends opcode 0x2b3 with no body (the frame bytes are `00 04 b3 02 00 00`); with 0 it finds the item (`0x5ed630`), cancels a pending cast and uses the item.

Addresses: `0x48ac80`, `0x48acb8`, `0x48acba`, `0x48acc6`, `0x48ace6`, `0x48acec`, `0x48ad06`, `0x48ad1e`, `0x48ad20`, `0x48ad37`, `0x48ad70`, `0x48adb6`, `0x48adf2`, `0x48adfa`, `0x48ae05`, `0x48ae58`, `0x48ae5a`, `0x48ae6b`, `0x48ae7d`, `0x5ed630`, `0x5d8bc0`, `0x5d8d00`, `0x6e4940`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

An item whose cached record has no entry gives an empty name, which is a string and not nil. The cancel `0x6e4940(1, 1, 0x1c)` signals `SPELLCAST_FAILED` and sends the cast cancel when a cast is active. The field has no watcher, so a change of it raises no event.
The item is the first carried item (equipment, bags, backpack, keyring) with an on-use spell of effect 0x5e (`objectlayer/interaction`).

## Dead and alive

`0x495850` signals `PLAYER_DEAD` and clears the cursor (`0x495190(1, 1)`) when `UNIT_FIELD_HEALTH` of the active player is 0 or less, else signals `PLAYER_ALIVE`. It runs from the health watcher chain of the death handler, once at world entry and on the packet 0x252.

Addresses: `0x495850`, `0x495871`, `0x495876`, `0x495878`, `0x49587d`, `0x495889`, `0x49588e`, `0x495893`, `0x495190`, `0x6251cc`, `0x625190`, `0x6058ae`, `0x5deb65`, `0x5e8f10`, `0x6046f0`
Confidence: verified
Proof: one
Test: none

The three callers are the tail jump at `0x6251cc` (from `0x625190`, from the death handler `0x605860` at `0x6058ae`), the call at `0x5deb65` after the world-entry batch `0x4908c0` and the handler `0x5e8f10` of packet 0x252. The health field's watcher is `0x6046f0`, registered on the unit window at offset 0x40.

## The resurrection offer

`ResurrectHasSickness()` pushes 1 when the first byte of the offer packet (`0xc4d768`) is nonzero, else nil; `ResurrectHasTimer()` does the same for the second byte (`0xc4d76c`). Neither reads an argument. The offer itself (GUID, name and the event `RESURRECT_REQUEST`) is `objectlayer/playermessages#resurrection-and-summoning`.

Addresses: `0x48aa00`, `0x48aa05`, `0x48aa30`, `0x48aa35`, `0xc4d768`, `0xc4d76c`
Confidence: verified
Proof: one
Test: none

The values are left from the last offer and are not cleared by the functions.

## Resurrection sickness

`GetResSicknessDuration()` pushes the length of the sickness as text, or nil. The spell is the active player's race row (`ChrRaces`, `+0x30`), or 15007 with no row; with no player the spell is 15007 at level 0 (`je` at `0x51a3d1`). The length is the spell's `SpellDuration` base (`+4`) plus the per-level word (`+8`) times `UNIT_FIELD_LEVEL`, at most the maximum (`+0xc`), passed through the spell modifiers (`0x6e6af0`). Below 1000 milliseconds, or with a missing row, the answer is nil;
otherwise it is formatted by `0x52fa50` with the `GENERIC` key.

Addresses: `0x51a3a0`, `0x51a3cc`, `0x51a3d9`, `0x51a3fd`, `0x51a408`, `0x51a420`, `0x51a43c`, `0x51a459`, `0x51a461`, `0x51a46c`, `0x51a47d`, `0x6e6af0`, `0x52fa50`, `0xc0dee0`, `0xc0dee4`, `0xc0d828`, `0xc0d82c`
Confidence: verified
Proof: one
Test: none

A race row with a zero in column 12 looks up spell 0, whose row is read like any other; a negative value gives nil. No branch tests the player's level: a level whose product is below 1000 ms gives nil through the duration data.

## NotWhileDeadError

`NotWhileDeadError()` shows the error `ERR_PLAYER_DEAD` (message id 0x7e) through `0x496720` and pushes nothing. It reads no argument and tests nothing: the caller decides when to call it. Nothing in the client calls it; it is reached only through the Lua global.

Addresses: `0x48d340`, `0x48d342`, `0x496720`, `0x83e398`
Confidence: verified
Proof: one
Test: none

The function is five instructions (`push 0x7e; call 0x496720; add esp,4; xor eax,eax; ret`) and the registry pair at `0x83e398` is the one word that holds its address. Row 126 of `tables/messages.tsv` is `ERR_PLAYER_DEAD`, kind 2, which signals `UI_ERROR_MESSAGE`, with no sound.
