# ui: the pet bar and the form bar

The pet action bar and the shapeshift form bar: the state each keeps, the Lua functions the stock
interface reads and drives them with, and the messages they send and take.

The pet bar is `PetInfo.cpp`, `0x4bc080` to `0x4bf200`, with a table of 28 Lua functions at 0x8475a0
that `0x4beb90` registers. Its state is a pet guid, ten packed slot words, a mode word and an attack
latch, all filled by `SMSG_PET_SPELLS`. The form bar is a list of spell ids kept by the spell book
(`ui/spellbook`) and four Lua functions at `0x4b4590` to `0x4b49a0`. The generic cast entry is in
`spell/cast`, the cursor payloads in `ui/cursor`.

Roots: `0x4bc690`, `0x4bc7b0`, `0x4beb90`, `0x4bc7e0`, `0x4bc930`, `0x4bc940`, `0x4bc960`, `0x4bc9a0`, `0x4bcbb0`, `0x4bcea0`, `0x4bcf70`, `0x4bd1d0`, `0x4bd630`, `0x4bd6e0`, `0x4bd990`, `0x4bdb10`, `0x4bdb70`, `0x4bdc00`, `0x4b2bb0`, `0x4b2c50`

## Pet state

The client keeps the pet's guid at `0xb714a0` (a zero guid is no pet), the tick at which a possession
ends at `0xb714a8`, an attack latch at `0xb714b0`, ten slot words at `0xb71438` and a mode word at
`0xb71468`: byte 0 is the reaction (0 passive, 1 defensive, 2 aggressive), byte 1 the command (0 wait,
1 follow), and bit 27 comes only from the server. The slot word is `type = byte 3 & 0x3f`, `action =
low 16 bits`, bit 31 autocast allowed and bit 30 autocast enabled. Types 1 to 5 are spells, 6 a reaction
and 7 a command; a zero word is an empty slot.

Addresses: `0xb714a0`, `0xb714a8`, `0xb714b0`, `0xb71438`, `0xb71468`, `0x4bc7e0`, `0x4bc930`, `0x4bc940`, `0x4bc960`, `0x4bc980`, `0x4bc822`, `0x4bc836`, `0x4bc84f`, `0x4bc872`, `0x4bc8ce`, `0x4bc8f8`, `0x4bc90c`, `0x5df680`
Confidence: verified
Proof: one
Test: none

`0x4bc7e0(duration, guid)` is the only writer of the guid (`0x4bc822`). It records whether it
changed, sets `0xb714a8` to now plus the duration or 0 (`0x4bc836`), and on a change signals `UNIT_PET` (id 2) with
the argument `player` (`0x4bc84f`), calls `0x5df680` with the guid of the open trainer when that
trainer's type is 3 (`0x4bc872`), clears the attack latch (`0x4bc8ce`) and, when the new guid resolves
to a unit that passes `0x6116e0`, signals `PET_UI_CLOSE` (428). It always signals `PET_BAR_UPDATE` (353) at `0x4bc90c`. `0x4bc930`
stores the whole mode word; `0x4bc940(action)` replaces byte 0, `0x4bc960(action)` replaces byte 1
and keeps bit 27 and byte 0, each signalling `PET_BAR_UPDATE`. `0x4bc980` zeroes the ten slots and
the raw spell count (0xb71428).

## The pet gate

`0x4bcf70` answers whether the player may use the pet bar, and is the gate of every click, drop and
autocast toggle. It needs the active player, who is not charmed (`[player.fields+0x10]` and `+0x14`
both 0); a far-sight guid, if the player has one, equal to the pet; a pet unit resolving from the cached
guid, whose charmer (else creator) guid is the player; none of the pet's `UNIT_FIELD_FLAGS` bits
0x40000, 0x400000 and 0x800000; and bit 27 of the mode word clear. `GetPetActionsUsable` pushes 1 or nil
from it. `PetHasActionBar` pushes 1 for a nonzero pet guid, else nil.

Addresses: `0x4bcf70`, `0x4bcf9e`, `0x4bcfc5`, `0x4bcfd1`, `0x4bd000`, `0x4bd034`, `0x4bd045`, `0x4bd075`, `0x4bd08d`, `0x4be0b0`, `0x4be0c5`, `0x4bdc20`, `0x4bdc2d`
Confidence: verified
Proof: one
Test: none

`0x4bcf9e` to `0x4bcfa4` test the player's pair; `0x4bcfc5` loads the far-sight guid from
`[player+0xe68]+0x830` and `0x4bcfd1 je` skips its compare when it is 0 (`0x4bd000` compares it
with the pet guid). `0x4bd034` resolves the pet with the unit mask; `0x4bd045` falls back to `+0x18`
when `+0x10` is 0; `0x4bd075` tests the flags; `0x4bd08d` the mode bit. No health field is read.
`0x4be0b0` calls `0x4bcf70`; `0x4bdc20` is `or eax,[0xb714a4]` and a push of 1.0 or nil.

## GetPetActionInfo

`GetPetActionInfo(index)` returns seven values, all nil for no pet, an index outside 1 to 10 or an
empty slot. A spell slot (types 1 to 5): name, rank, icon path (the active icon when `0x4bcea0` finds
the spell's aura on the pet, else the spell icon), nil, nil, then autocast allowed and enabled as 1 or
nil, nil when the spell is unknown. A reaction (type 6): `PET_MODE_` plus the token, nil,
`PET_<token>_TEXTURE`, 1, 1 when it is the current reaction, nil, nil. A command (type 7):
`PET_ACTION_` plus the token, nil, the texture key, 1, 1 when it is the current command or attack
with the latch set, nil, nil. The names are global-string keys.

Addresses: `0x4bdc50`, `0x4bdc9d`, `0x4bdca6`, `0x4bdcbf`, `0x4bdcd0`, `0x4bdcdc`, `0x4bdce3`, `0x4bdd2f`, `0x4bddd3`, `0x4bde3c`, `0x4bde98`, `0x4bdf03`, `0x4bdf3e`, `0x4bdf70`, `0x4bce50`, `0x4bce60`, `0x4bcea0`
Confidence: verified
Proof: one
Test: none

`0x4bdc9d` to `0x4bdcbf` are the three early outs, each reaching `0x4bdf3e`, which pushes seven nils.
`0x4bdcd0 lea eax,[ecx-1]; cmp 6; ja` and the table at `0x4bdcdc` send types 1 to 5 to `0x4bdce3`, 6
to `0x4bddd3` and 7 to `0x4bde98`. A type of 0 or above 7 skips the five leading values and pushes
only two nils but returns 7 (`0x4bde6f`, `0x4bdf70`). `0x4bde3c` to `0x4bde5d`: the reaction is current
when `(state & 0x08000000 ? 0 : state & 0xff)` equals the action; `0x4bdf03` to `0x4bdf22`: the command
when `state >> 8` (unmasked) equals the action, or the action is 2 and `0xb714b0` is nonzero. The tokens
are 32-byte strings (`0x4bce50` indexes reactions, `0x4bce60` commands from the fourth) with no bound:
PASSIVE, DEFENSIVE, AGGRESSIVE then WAIT, FOLLOW, ATTACK, DISMISS. The pet's aura test `0x4bcea0` is
the pet twin of `ui/actionbar#active-icon-test`.

## Pet predicates and cooldowns

`IsPetAttackActive(index)` pushes a boolean, true for a pet guid, an index below 10, a command slot of
action 2 and the latch set. `HasPetUI` pushes two values: 1 when the pet resolves, its object type lacks the
player bit (0x10) and `UNIT_FIELD_PETNUMBER` is nonzero, else nil; and 1 when `0x6116e0` holds, else nil. `GetPetActionCooldown(index)` returns start and
duration times 0.001 and the enable flag for a spell slot (types 1 to 5) from the pet's bank
(`0x6e2ea0` with bank 1), and zeros for anything else.

Addresses: `0x4be0e0`, `0x4be121`, `0x4be125`, `0x4be135`, `0x4be14c`, `0x4be670`, `0x4be697`, `0x6116e0`, `0x4bdfa0`, `0x4bdff4`, `0x4be013`, `0x4be01e`, `0x4be03c`, `0x4be04a`, `0x4be088`, `0x6e2ea0`
Confidence: verified
Proof: one
Test: none

`0x4be121` to `0x4be14c` are the five conditions, the result pushed as a boolean with `0x6f39f0`.
`0x6116e0` is true at once for a pet with the player type bit; otherwise the pet number must be nonzero,
the player's charm guid (its summon guid when none) must be the pet, and the player's class byte 3.
`0x4bdff4`, `0x4be013`, `0x4be01e` gate the cooldown on index, nonempty slot and type 1 to 5;
`0x4be03c` calls `0x6e2ea0(spell, 1, ...)`; the two scaled pushes multiply by the double 0.001 and the
third is unscaled. The three outputs start at 0, so every skipped case returns zeros.

## The pet spell message

`SMSG_PET_SPELLS` (377), handled by `0x4bd990`, clears the bar and reads a guid; a zero guid clears
the pet spell list and goes to the tail. Otherwise it reads a u32 duration, a u32 mode word (stored
whole), ten u32 slot words (stored without sending), then a u8 count of u32 spells, each kept raw and,
when its record exists without `Attributes & 0x80`, appended to the pet book. Last come a u8 count of
cooldowns, each `u16 spell, u16 category, u32, u32`, the high bit of the last the enable flag, added to the pet
cooldown bank. The tail sorts the pet book and sets the guid with `0x4bc7e0`.

Addresses: `0x4bd990`, `0x4bd999`, `0x4bd9b1`, `0x4bd9bc`, `0x4bd9c8`, `0x4bd9d3`, `0x4bd9db`, `0x4bd9eb`, `0x4bd9f6`, `0x4bda07`, `0x4bda2c`, `0x4bda44`, `0x4bda58`, `0x4bda6c`, `0x4bda77`, `0x4bda82`, `0x4bda8d`, `0x4bdabf`, `0x4bdacf`, `0x4bdad4`, `0x4bdaf0`, `0x4b2f70`, `0x4b2f90`, `0x4b2fd0`, `0x6e2c60`, `0x4bc690`
Confidence: verified
Proof: one
Test: none

`0x4bc690` registers the handlers for opcodes 0x179, 0x17a, 0x2c6 and 0x2af and `0x4bc7b0` removes them.
In `0x4bd990`: `0x4bd999 call 0x4bc980`, the 64-bit read at `0x4bd9b1`, the zero test `0x4bd9bc`, duration
(`0x418fb0`), mode (`0x418eb0` then `0x4bc930`), ten slots through `0x4bc9a0` with send 0, count (`0x418cb0`),
spells through `0x4bd150` and `0x4b2f90`, cooldown count, the two u16 reads (`0x418db0`) and two u32
reads (`0x418e30`), `0x4bda97` splitting the high bit, `0x4bdabf` calling `0x6e2c60` with bank 1, and
`0x4bdad4` the sort `0x4b2fd0(0, 1)`, then `0x4bdaf0 call 0x4bc7e0`.

## Pet mode and feedback messages

`SMSG_PET_MODE` (378) reads a guid and a u32; for the cached pet guid only it stores the word with
`0x4bc930` and signals `PET_BAR_UPDATE`, otherwise it is dropped. Opcode 0x2c6 reads one byte and shows
the error message 336, 160, 161 or 337 for 1 to 4 and nothing else. Opcode 0x2af reads nothing and shows
the error message 419. Neither changes state.

Addresses: `0x4bdb10`, `0x4bdb33`, `0x4bdb3f`, `0x4bdb4a`, `0x4bdb4f`, `0x4bdb70`, `0x4bdb89`, `0x4bdb90`, `0x4bdc00`, `0x496720`
Confidence: verified
Proof: one
Test: none

`0x4bdb33` and `0x4bdb3f` compare the guid with `0xb714a0` and `0xb714a4`. `0x4bdb70` jumps
through a table at `0x4bdb89` over the byte minus 1 (up to 3) to pushes of 0x150, 0xa0, 0xa1 and 0x151
before `0x496720`. `0x4bdc00` is `push 0x1a3; call 0x496720`.

## Pet events

| id | event | signalled by |
|---|---|---|
| 353 | `PET_BAR_UPDATE` | pet state changes: `0x4bc90c`, `0x4bc951`, `0x4bc974`, `0x4bdb4f`, `0x4bcb68`, `0x4bcc7c`, `0x4bd429`, `0x4bd6a9` |
| 354 | `PET_BAR_UPDATE_COOLDOWN` | `0x4bce90`, from the cooldown store when its bank is the pet's |
| 355, 356 | `PET_BAR_SHOWGRID`, `PET_BAR_HIDEGRID` | `0x4bce70`, `0x4bce80` |
| 385 | `PET_DISMISS_START` | `0x4bd6e0` |

Addresses: `0x4bce70`, `0x4bce80`, `0x4bce90`, `0x4bc951`, `0x4bc974`, `0x4bcb68`, `0x4bcc7c`, `0x4bd429`, `0x4bd6a9`
Confidence: verified
Proof: one
Test: none

`0x4bce70`, `0x4bce80` and `0x4bce90` are `mov ecx,id` and a jump to the signal with 355, 356 and 354.
The show grid event is signalled when a pet action goes on the cursor (`0x494f28`), the hide grid
event by the cursor clear when the payload is a pet action (`0x4952ac`). `PET_ATTACK_START` and
`PET_ATTACK_STOP` come from the unit flag watcher, not from here
(`objectlayer/units#unit-flags-and-bytes`).

## Pressing a pet action

`0x4bd1d0(slotWord, target)` carries out a pet bar click, using the current target when none is passed.
It does nothing unless the action gate `0x494a50` for kind 2 and the pet gate allow. A reaction or a
wait or follow command sets its mode byte first. A spell whose aura is up sends `CMSG_PET_CANCEL_AURA`
(619: pet guid, spell) and returns. Another spell goes through the generic cast entry `0x6e4b60`, with
the pet as caster, when the record's word at `+0x28` has bit 5 or the pet is the player's far-sight
unit. An attack command sets the latch, signals `PET_BAR_UPDATE`, and sends only if `0x612df0` accepts
the target. All else sends `CMSG_PET_ACTION` (373): `u64 pet guid, u32 slot word, u64 target`.

Addresses: `0x4bd1d0`, `0x4bd1e5`, `0x4bd1f2`, `0x4bd1ff`, `0x4bd212`, `0x4bd239`, `0x4bd248`, `0x4bd25f`, `0x4bd355`, `0x4bd35e`, `0x4bd36e`, `0x4bd385`, `0x4bd391`, `0x4bd399`, `0x4bd3a3`, `0x4bd3b1`, `0x4bd3c3`, `0x4bd40d`, `0x4bd414`, `0x4bd420`, `0x4bd42e`, `0x4bd43f`, `0x4bd444`, `0x4bd44c`, `0x4bd4c6`, `0x612df0`, `0x5ee5a0`, `0x6e4b60`, `0x6e2de0`, `0x494a50`
Confidence: verified
Proof: one
Test: none

`0x4bd1e5` and `0x4bd1f2` are the two gates, both leaving through `0x4bd4c6` without a packet. The type
is `byte 3 & 0x3f` (`0x4bd22a`), dispatched through the table at `0x4bd239`. `0x4bd248` calls
`0x4bcea0`, the cancel opens at `0x4bd25f`. `0x4bd355 test [rec+0x28],0x20` and `0x4bd35e` call
`0x5ee5a0`, which returns the player's possessed far-sight unit (`objectlayer/interaction#far-sight`);
a match goes to `0x4bd378` and the cast `0x4bd385`; otherwise `0x4bd36e`. Types 6 and 7 enter
at `0x4bd391` and `0x4bd3a3`; `0x4bd3b1` sends actions 0 and 1 to `0x4bc960` (`0x4bd43f`), `0x4bd3c3` runs
the attack chain, whose veto is `0x4bd414`, and the latch store is `0x4bd42e`; it is set only when
the player's possessed unit is the pet (`0x4bd420`). Actions above 2 (dismiss) fall to the send. The send
at `0x4bd444` opens opcode 0x175 (`0x4bd44c`).

## Pet commands from Lua

`CastPetAction(index)` drops a held pet action onto the slot; with an empty cursor it plays a sound and
presses the slot with a zero target override, so the click carries the current target. `PetPassiveMode`,
`PetDefensiveMode`, `PetAggressiveMode`, `PetWait`, `PetFollow` and `PetAttack` take no arguments and
press the slot words 0x06000000, 0x06000001, 0x06000002, 0x07000000, 0x07000001 and 0x07000002 as
`0x4bd1d0` does, so the mode or command byte is set and signalled before the packet. `PetStopAttack`
does nothing unless the latch is set, then sends opcode 0x2ea with the pet guid, clears the latch and
signals `PET_BAR_UPDATE`, without the pet gate. `PetDismiss` presses 0x07000003 with a zero target,
then signals `PET_DISMISS_START` with 10000 whether or not a packet went out.

Addresses: `0x4be330`, `0x4be3d1`, `0x4be42b`, `0x4be450`, `0x4be460`, `0x4be470`, `0x4be480`, `0x4be490`, `0x4be4a0`, `0x4be4b0`, `0x4be4d0`, `0x4bd4f0`, `0x4bd530`, `0x4bd570`, `0x4bd5b0`, `0x4bd5f0`, `0x4bd630`, `0x4bd650`, `0x4bd662`, `0x4bd6a9`, `0x4bd6ae`, `0x4bd6e0`, `0x4bd701`, `0x4bd70e`, `0x4bd722`
Confidence: verified
Proof: one
Test: none

Each one-shot binding is `call <builder>; xor eax,eax; ret` onto a 58-byte builder that stores its
word and calls `0x4bd1d0(&word, &{0,0})`; `0x4be4a0` loads `ecx` with `0xb4e2d8` so `0x4bd630`
forwards the selection pair. `0x4bd630` is also called from `0x60bf65` and `0x613217`. `0x4be3d1` tests
bit 0 of the pet's `[+0x110]+0xa3`; `0x4be42b` passes the zero pair. `0x4bd650` returns at `0x4bd65e` for
a zero latch, sends at `0x4bd662` and clears `0xb714b0` at `0x4bd6ae`. `0x4bd6e0` stores 0x07000003
(`0x4bd701`), presses it (`0x4bd70e`) and signals 0x181 at `0x4bd722` with no test between.

## Pet autocast

`TogglePetAutocast(index)` with a pet action on the cursor drops it; otherwise `0x4bcbb0(slot)` runs the
action and pet gates and does nothing unless bit 31 of the slot word is set. It flips bit 30 in the
word, stores it, copies it into the raw pet spell array (`0x4bd190`), sends opcode 0x174
(`u64 pet guid, u32 slot, u32 word`) and signals `PET_BAR_UPDATE`. The bar repaints without waiting for the
server. `ToggleSpellAutocast` (`ui/spellbook`) is a different message, 0x2f3.

Addresses: `0x4be290`, `0x4bcbb0`, `0x4bcbc2`, `0x4bcbcf`, `0x4bcbf4`, `0x4bcc04`, `0x4bcc17`, `0x4bcc19`, `0x4bcc1e`, `0x4bcc7c`, `0x4bd190`, `0x4bccb0`, `0x4bcd5f`
Confidence: verified
Proof: one
Test: none

`0x4bcbf4 test cl,1` is the bit 31 requirement and `0x4bcc04 test dl,1` the bit 30 toggle
(`or 0x40000000` or `and 0xbfffffff`); `0x4bcc17` stores before `0x4bcc19` copies and `0x4bcc1e push
0x174` sends. `0x4bd190` scans the raw array backwards for an entry equal under `& 0x3fffffff` and writes
the full word. `0x4bccb0` sends 0x2f3 (`0x4bcd5f`) and is called only from the book function.

## Picking up and dropping pet actions

`PickupPetAction(index)` drops a held pet action onto the slot, or with an empty cursor puts the slot's
action on the cursor (`0x494e20`) and, for a spell slot only, blanks the slot's spell id and sends it.
A drop runs `0x4bc9a0(target, &word, send)`. It ignores a word equal to the slot's, refuses a passive
spell without a message, and empties another slot holding the same type and action, giving it the
displaced word. A displaced reaction or command with no duplicate goes to the first slot that is not
one and has no spell id, and with none nothing happens. The result is sent as `CMSG_PET_SET_ACTION`
(372): pet guid and one or two `u32 slot, u32 word` pairs, then `PET_BAR_UPDATE`.

Addresses: `0x4be180`, `0x4be25d`, `0x4be268`, `0x4be27f`, `0x4bc9a0`, `0x4bc9ad`, `0x4bc9cc`, `0x4bc9e4`, `0x4bc9f8`, `0x4bca20`, `0x4bca31`, `0x4bca40`, `0x4bca6f`, `0x4bca7a`, `0x4bca8b`, `0x4bcab8`, `0x4bcac3`, `0x4bcad9`, `0x4bcaf7`, `0x4bcb92`, `0x4bce00`, `0x4bce33`, `0x4bce3d`, `0x494e20`
Confidence: verified
Proof: one
Test: none

`0x4bc9ad` calls `0x494a50` for kind 9, which the gate always passes. `0x4bc9cc` runs the pet gate only
when the send flag is set. `0x4bc9e2` to `0x4bc9ef` return when the words are equal. `0x4bca20 test
[rec+0x18],0x40` refuses a passive spell. The duplicate scan `0x4bca40` compares under `0x3fffffff` for
every type except a type 1 with no spell id; the relocation scan `0x4bca8b` looks for a slot whose
type is not 6 or 7 and whose low 16 bits are 0; `0x4bcab8` is the abort. The slot index is never
compared with 10 inside the core. `0x4bcb92` returns 1 when no relocation happened, an abort included. The caller clears the held
payload before the drop, so a refused drop loses it, and index 10 passes the callers' bound and writes one
word past the bar.
`0x4bce00(payload, slot)` forwards to the core and puts the displaced word on the cursor again
(`0x4bce3d`) when the core returns 1.

## Pet stats

`GetPetLoyalty` returns the loyalty level string of the pet (from `PetLoyalty` by the byte at
`[pet+0x110]+0x211`) or nil. `GetPetTrainingPoints` returns the two words of the dword at `+0x23c`,
high word first, and `GetPetExperience` the dwords at `+0x21c` and `+0x220`; both return zeros when the
pet fails the test `0x6116e0`. `GetPetHappiness` returns a bucket 0 to 3, a damage percent
and a loyalty rate: the happiness at `+0x54` is compared with the three thresholds of the pet's
`PetPersonality` row (row 1 when the pet has none), the damage entry for the bucket is times 100.0,
and bucket 0, a failed gate or no row gives `100.0, 0.0` (nil as the bucket when the gate fails).

Addresses: `0x4be700`, `0x4be790`, `0x4be840`, `0x4be900`, `0x4be947`, `0x4be97c`, `0x4be998`, `0x4be9ae`, `0x4be9e0`, `0x6116e0`, `0x605600`
Confidence: verified
Proof: one
Test: none

All four resolve the pet from the cached guid and gate on `0x6116e0`. `GetPetHappiness`:
`0x4be947 call 0x605600` gives the personality id, `0x4be96c` the fallback row, `0x4be97c` the threshold loop
over `rec+0x28`, `rec+0x2c`, `rec+0x30`; the pushes `0x4be998` (bucket), `0x4be9ae` (damage entry at
`rec+4i+0x30`, times the constant 100.0) and the rate at `rec+4i+0x3c`; `0x4be9e0` is the failure
return. The failure of `GetPetLoyalty` is one nil; of the other two, numbers.

## Abandon and rename

`PetCanBeRenamed` and `PetCanBeAbandoned` push 1 or nil: the pet resolves, was summoned by the player
(`UNIT_FIELD_SUMMONEDBY`, `+0x18`) and has `UNIT_FIELD_FLAGS` bit 0x10 (rename) or 0x20 (abandon). `PetAbandon`
sends opcode 0x176 with the pet guid after the kind 2 gate, and message 0x26f with the stable master guid
when the stable window is open. `PetRename(name)` shows an error for no pet (243), a pet that is not the
player's (244), one that cannot be renamed (245) or an empty name (246), then sends opcode 0x177:
`u64 pet guid, cstring name`, cut to 79 bytes. Nothing is echoed; the new name arrives through the name
cache, whose row is dropped and queried again when the pet's name timestamp field changes.

Addresses: `0x4be500`, `0x4be544`, `0x4be580`, `0x4be5c4`, `0x4be4c0`, `0x4bd740`, `0x4bd765`, `0x4bd7b2`, `0x4be4e0`, `0x4bd840`, `0x4bd872`, `0x4bd8b5`, `0x4bd8d2`, `0x4bd8fe`, `0x4bd935`, `0x4bd947`, `0x4bd8ee`, `0x6f3690`
Confidence: verified
Proof: one
Test: none

`0x4be544 test byte [ecx+0xa0],0x20` and `0x4be5c4` with 0x10, each after the owner compare
against `0x468550`; the answer 1.0 or nil. `0x4bd740` runs the kind 2 gate, sends 0x176 at `0x4bd765` and,
when the stable master guid at 0xb72240 is nonzero, 0x26f at `0x4bd7b2`. `0x4bd840` raises the four
errors at `0x4bd872`, `0x4bd89f`, `0x4bd8b5` and `0x4bd8d2`, formats the name with `%s` into 80
bytes (`0x4bd8ee`) and sends at `0x4bd8fe` and `0x4bd947`. The 12-letter limit is the interface's.

## Form list

The form bar is a sorted list of spell ids at `0xb71108` (count `0xb71104`). `0x4b25b0` adds a learned
spell unless its `AttributesEx2` has bit 1 (0x2); the spell needs a shapeshift aura effect (aura 36) or
`AttributesEx2` bit 4 (0x10). The sort `0x4b2bb0` orders by the word at `+0x298`, ascending, entries of -1
last and entries below 0 by spell id. Unlearning removes the entry (`0x4b2c50`). Both signal
`UPDATE_SHAPESHIFT_FORMS` (387). `GetNumShapeshiftForms` pushes the count.

Addresses: `0x4b4590`, `0xb71104`, `0xb71108`, `0x4b27d1`, `0x4b2810`, `0x4b2815`, `0x4b28f7`, `0x4b28ff`, `0x4b2bb0`, `0x4b2c16`, `0x4b2c50`, `0x4b2e43`
Confidence: verified
Proof: one
Test: none

`0x4b27d1 test dl,2; jne` excludes; `0x4b2810 cmp [ecx],0x24` admits and `0x4b2815 test dl,0x10`
force-admits; `0x4b28f7` sorts with `0x4b2bb0`; `0x4b28ff` loads 0x183 for the event and `0x4b2e43`
does so on removal. In `0x4b2bb0`, `0x4b2bf0` loads the key; two negative keys order by id
(`0x4b2c04`); `0x4b2c16` sends a -1 key last and `0x4b2c2e` ahead of it.

## Form info

`GetShapeshiftFormInfo(index)` returns icon path, name, active and castable. Without a number it
raises; an index outside the list, no player or a missing record gives four nils. The spell's form
number is its shapeshift aura effect's misc value, 0 when none. With a form number the row is active
when the player's form byte equals it; with none, when the spell has an active icon and its own aura sits
in the unit's aura slots with the flag bit set. An active row shows the active icon when it has one;
any other row shows the spell icon. Active is 1 or nil; castable is 1 for an active row, else the
usability test `0x6e3d60`.

Addresses: `0x4b45c0`, `0x4b4633`, `0x4b4639`, `0x4b4680`, `0x4b4699`, `0x4b46af`, `0x4b46f2`, `0x4b4713`, `0x4b4754`, `0x4b475c`, `0x4b4763`, `0x4b4769`, `0x4b479c`, `0x4b47c9`, `0x4b47d9`, `0x6e3d60`
Confidence: verified
Proof: one
Test: none

The five exits to the nil block `0x4b47d9` are `0x4b4633`, `0x4b4639`, `0x4b464b`, `0x4b4657` and
`0x4b466b`. `0x4b4680` scans for aura 0x24. `0x4b4699` selects between the form-byte compare (`0x4b46a0`
to `0x4b46af`) and the aura scan (`0x4b46f2` onward, gated on a nonzero `+0x1d8`); a hit and a form
match both reach `0x4b4754`, which loads the active icon, stores 1 as active (`0x4b475c`) before
testing it (`0x4b4763`), and takes the active icon (`0x4b4769`). A form-backed row with no active
icon is active and shows the spell icon.

## Form cooldown and cast

`GetShapeshiftFormCooldown(index)` returns start and duration times 0.001 and the enable flag for the
form's spell from the player's bank, and `0, 0, 1` for an index outside the list. `CastShapeshiftForm(index)`
takes the form's spell. With a shapeshift aura whose form number is in the form table: if the player is
already in that form it cancels the form (`0x6e7040`), or does nothing when the form record's flag word
(`+0x2c`) has bit 1 set; in any other form state it casts. A spell without a form number toggles by its
aura instead: the cancel happens when the spell has an active icon and its aura is up, else it casts.
The cast is `0x6e5a90` on the player.

Addresses: `0x4b49a0`, `0x4b4810`, `0x4b48c6`, `0x4b48d4`, `0x4b4912`, `0x4b4930`, `0x4b4939`, `0x4b495f`, `0x4b4963`, `0x4b4969`, `0x4b497c`, `0x4b498a`, `0x6e5a90`, `0x6e7040`, `0xc0d774`, `0xc0d778`
Confidence: verified
Proof: one
Test: none

`0x4b4930` loads the form number; `0x4b4939 je 0x4b48d4` sends a zero number to the aura toggle,
`0x4b495f` compares the form byte, `0x4b4963 test [form+0x2c],2` is the no-op guard and `0x4b4969` the
cancel. The aura toggle: `0x4b48d4` requires an active icon, the scan with the nibble test at
`0x4b4912`, and the cancel at `0x4b497c`; the guard is unreachable from it. `0x4b498a` is the cast.

## Other pet functions

`GetPetTimeRemaining` (`0x4be600`) pushes nil when the expiry tick `0xb714a8` is 0, else the milliseconds
left, 0 once it has passed (no scaling to seconds). `GetPetFoodTypes` (`0x4bea10`) returns, as
strings, the localized name of each food type whose bit (the type minus 1) is set in the word at
`+0x1c` of the pet's creature family record, in table order; it returns no value for a pet that fails
`0x6116e0`. `GetPetIcon` (`0x4beb10`) pushes the string at `+0x44` of that family record, or nil.

Addresses: `0x4be600`, `0x4be607`, `0x4be614`, `0x4be628`, `0x4be634`, `0x4bea10`, `0x4bea4e`, `0x4bea5d`, `0x4bea86`, `0x4beaad`, `0x4beaba`, `0x4bead2`, `0x4beb10`, `0x4beb66`, `0x6055e0`, `0x4beb90`
Confidence: verified
Proof: one
Test: none

`0x4be607` loads the tick, `0x4be614` and `0x4be61f` read the clock, `0x4be628` makes a passed tick 0, and
`0x4be634` pushes the integer. `0x4bea5d call 0x6055e0` gives the family id, which indexes the
`CreatureFamily` store (at 0xc0de7c); `0x4bea86` starts the loop over the 40-byte food records at 0xc0dbe0,
`0x4beaad` loads a record's type, `0x4beaba` tests the family's mask word and `0x4bead2` pushes the name of
the active locale. `0x4beb66` loads the family record's string at `+0x44`. `0x4beb90` registers the 28
names with `0x704120`.
