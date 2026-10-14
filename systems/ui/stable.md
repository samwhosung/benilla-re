# ui: the stable master window

The pet stable (`StableInfo.cpp`, `0x4caa00` to `0x4cbd10`): the module state, the two server messages, the click and drag
rules that choose between stable, unstable and swap, the thirteen Lua functions, and the four events. The pet's food and
family read are shared with `ui/unitstats` and `ui/petbar`.

Roots: `0x4caa80`, `0x4cab10`, `0x4cacb0`, `0x4cadc0`, `0x4cae10`, `0x4cb420`, `0x4cb7a0`, `0x4cbbd0`

## Stable state

The state is a summoned-pet record, an array of stabled-pet records and a few cells. A record is 0x64 bytes: the pet
number (+0), the level (+4), the creature entry (+8), the loyalty row id (+0xc), the name (+0x10, 0x50 bytes) and the wire
slot byte (+0x60). The summoned pet is at `0xb72058` and the array of three at `0xb720c8`. The stable master's GUID is
`0xb72240`, the number of records in the array `0xb72248`, the number of slots owned `0xb7224c`, and the selected pet's
number `0xb72250` (-1 for the summoned pet, 0 for none). The array is never cleared, and nothing bounds its length.

Addresses: `0x4caab0`, `0x4caae0`, `0x4cab5f`, `0xb72058`, `0xb720c8`, `0xb72240`, `0xb72248`, `0xb7224c`, `0xb72250`, `0xb721fc`
Confidence: verified
Proof: one
Test: none

The array holds three because the next referenced address after it is 0x12c bytes on. A count of 4 or more written from
the wire writes past it, and 6 or more overwrites the GUID, both counts and the selection. The reset `0x4caab0` runs at each
world entry and zeroes the summoned record, the GUID, both counts and the selection; `0x4caae0` closes the window on leaving
the world. The range cell `0xb721fc` is the constant 30.864195 (5.5555553 squared), not state.

## The list message

`MSG_LIST_STABLED_PETS` (0x26f) carries a u64 stable master GUID, a u8 pet count, a u8 slot count, and per pet a u32
pet number, u32 creature entry, u32 level, a name of at most 0x50 bytes, a u32 loyalty and a u8 slot. A pet whose slot
byte has bit 1 clear is the summoned pet: it is copied to `0xb72058` and removed from the array count. The others are kept in
wire order. Every message makes the sender the interaction target when it differs, clears the selection and signals
`PET_STABLE_SHOW` (437), even for an empty or truncated list. The window opens only this way.

Addresses: `0x4cab10`, `0x4cab21`, `0x4cab39`, `0x4cab44`, `0x4caba3`, `0x4cabab`, `0x4cabb6`, `0x4cabc3`, `0x4cabce`, `0x4cabd9`, `0x4cabe3`, `0x4cabe5`, `0x4cac2e`, `0x4cac96`, `0x4cac9b`, `0x4cadc0`, `0x4cade2`, `0x4cadf8`, `0x4930d0`
Confidence: verified
Proof: one
Test: none

The kept-pet test is `test al,2; jne`, not a test for 1: a slot byte of 4 would be taken as the summoned pet. The target
and range are set only when the GUID changes (`0x4cadc0`); the selection store at `0x4cadf8` is unconditional. A message
with no pets skips the loop at `0x4cab6f`.

## The result message

`SMSG_STABLE_RESULT` (0x273) carries one byte. Code 1 shows `ERR_NOT_ENOUGH_MONEY`. Codes 8 and 9 ask for the list again.
Code 10 adds 1 to the slot count and asks again. Code 11 signals `PET_STABLE_UPDATE` (438). Every other code, including the
generic stable error 6, does nothing. The request is `MSG_LIST_STABLED_PETS` with the stable master GUID, sent only while a
master is stored.

Addresses: `0x4cacb0`, `0x4cacbd`, `0x4cacc7`, `0x4cacd0`, `0x4cacd7`, `0x4cacde`, `0x4cacf3`, `0x4cacf9`, `0x4cad05`, `0x4cad0e`, `0x4cad50`, `0x4cad83`, `0x4cad8d`, `0x496720`
Confidence: verified
Proof: one
Test: none

A byte map and a jump table that follow the function select the arm from the code minus 1. The count is incremented at
`0x4cacf3` before the master test, so a slot purchase confirmed with no master open still raises the local count. Code 1 pushes
the message id 0x25 (`0x4cacde`).

## Click and drag

`PickupStablePet(index)` lifts a pet onto the cursor: an index of 0 or less is the summoned pet, 1 to N a stable slot. It
does nothing for an empty slot or a pet whose family has no icon, and it clears the cursor first. The payload is mode 10 with
the index. `ClickStablePet(index)` selects the pet and returns 1 unless the cursor holds the pet payload. With it, the click
sends one request and returns nil. A summoned pet dropped on an occupied slot sends a swap (0x275 with the occupant's number)
and on an empty owned slot stable (0x270). A stabled pet dropped on the summoned slot sends a swap when a pet is out and
unstable (0x271) otherwise.

Addresses: `0x4cb7a0`, `0x4cb7d5`, `0x4cb800`, `0x495010`, `0x495034`, `0x4950a8`, `0x4950ae`, `0x4950c0`, `0x4cb420`, `0x4cb469`, `0x4cb72b`, `0x4cb738`, `0x4cb763`, `0x4cb784`, `0x4cb4e1`, `0x4cb514`, `0x4cb537`, `0x4cb572`, `0x4cb59a`, `0x4cb5bc`, `0x4cb61b`, `0x4cb64f`, `0x4cb68b`, `0x4cb6bf`, `0x4cb70a`, `0xb4e300`, `0xb4d900`, `0xb714a0`
Confidence: verified
Proof: one
Test: none

`[0xb4d900] = 10` is written at one site, `0x4950ae`, with the index stored at `0xb4e300`. The pickup index test is signed
(`0x4cb7d5 jns`: any index of 0 or less selects the summoned pet); the click test is exact (`0x4cb72b cmp eax,-1`). The
swap and unstable requests carry the GUID and a pet number; stable carries the GUID only, so the destination slot is
the server's. A drop between stable slots, or on a slot not owned, sends nothing. The drop ends in `ClearCursor(1,1)` and `lua_pushnil` (`0x4cb70a`). Dropping on the world takes the cursor's
own clear path and sends nothing.

## Stable Lua functions

The thirteen functions are registered by `0x4cbbd0`. `GetNumStablePets()` and `GetNumStableSlots()` return the two counts.
`GetStablePetInfo(index)` returns the icon path of the pet's family, the name, the level, the family name and the loyalty
name; on a missing record or an unfetched creature it returns `nil, nil, 0, nil, nil`. `GetSelectedStablePet()` returns 0
for the summoned pet, the 1-based slot for a stabled one and -1 for none or a pet that is gone. `GetNextStableSlotCost()`
returns the next slot's cost row or 0. `GetStablePetFoodTypes(index)` returns the food names of the pet's family and nothing
on failure. `SetPetStablePaperdoll(frame)` puts the selected pet's model on a model frame.

Addresses: `0x4cbbd0`, `0x4cbbf0`, `0x4cb1d0`, `0x4cb200`, `0x4cb230`, `0x4cb286`, `0x4cb307`, `0x4cb360`, `0x4cb373`, `0x4cb3d0`, `0x4cb810`, `0x4cb84c`, `0x4cb870`, `0x4cba70`, `0x4cbace`, `0x4cbaa3`, `0x4cb3b0`, `0x4cba50`
Confidence: verified
Proof: one
Test: none

The usage texts are the function's own, except that `GetStablePetFoodTypes` raises `GetStablePetInfo`'s. The functions read
their arguments with the number tests only; none counts its arguments. The index conventions differ by
function: `GetStablePetInfo` and `PickupStablePet` use a signed test, `GetStablePetFoodTypes` and `ClickStablePet` an exact
-1, and `UnstablePet` an unsigned bound. The cost table is `StableSlotPrices.dbc`, read by `[0xb7224c] + 1`. The two arrival
callbacks `0x4cb3b0` and `0x4cba50` are passed to the creature cache, whose miss sends the creature query.

## Stable requests

`StablePet()` sends 0x270 with the GUID and nothing else, with no check but a stored master. `UnstablePet(index)` bails
silently while the player has a charmed unit or a summoned pet, and sends 0x271 with the GUID and the pet number of
the stabled pet at the 1-based index. `BuyStableSlot()` sends 0x272 with the GUID when a master is stored, the slot count
is not 2, a price row exists for the next slot and the player's coinage covers it; each failed test is silent.
`ClosePetStables()` closes the window and sends nothing.

Addresses: `0x4cae50`, `0x4cae56`, `0x4cae99`, `0x4cae10`, `0x4cae40`, `0x4caf20`, `0x4cafef`, `0x4cb022`, `0x4cb080`, `0x4cb0c4`, `0x4cb122`, `0x4cb15a`, `0x4cb182`, `0x5e02a0`, `0x5f05bc`
Confidence: verified
Proof: one
Test: none

The charm and summon words are read at `[player+0x110]` and `+8`; both must be 0 for an unstable. The slot cap is
`cmp eax,2` (`0x4cb0c4`). The client asks for the list itself when the player interacts with a unit that has the stable-master
NPC flag (bit 13): `0x5e02a0` sends the request from `0x5f05bc`.

## Closing and events

Closing the window (`0x4cae10`) clears the master GUID and signals `PET_STABLE_CLOSED` (440); the list and counts stay.
It does so through the interaction clear, which only acts when the stored target is the master, and puts nothing on the
wire. A script close while the master is the target signals the event twice. The other events are `PET_STABLE_UPDATE` (438),
signalled by result code 11 and by the creature arrival callback `0x4cb3b0`, and `PET_STABLE_UPDATE_PAPERDOLL` (439), signalled
by the callback `0x4cba50`; all four carry no arguments and are signalled nowhere else.

Addresses: `0x4cae10`, `0x4cae3a`, `0x4cac9b`, `0x4cad83`, `0x4cb3bf`, `0x4cba5f`, `0x4931c0`, `0x493310`, `0x4934de`, `0x4934f1`, `0x490a80`, `0x490b2a`
Confidence: verified
Proof: one
Test: none

The signal sites are `0x4cac9b` (437), `0x4cad83` and `0x4cb3bf` (438), `0x4cba5f` (439) and `0x4cae3a` (440, a tail jump). The two
callbacks signal only when their success byte is set. The twice-signalled close is `0x4cae10` calling `0x4931c0`, which
tail-jumps to the dispatcher `0x493310`, which calls `0x4cae10` again (`0x4934f1`) before the outer call clears the GUID. The
leave-world path calls the close before the dispatcher (`0x490b2a`) and signals once.
