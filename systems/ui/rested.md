# ui: rested experience functions

The four Lua functions about rest: `GetRestState`, `GetXPExhaustion`, `IsResting` and `GetTimeToWellRested`, and the
`Exhaustion.dbc` table the first two read. The events and messages of a rest change are in
`objectlayer/playerfields#rest-state` and `objectlayer/playerfields#player-flag-changes`.

Roots: `0x48d350`, `0x48d3f0`, `0x516ea0`, `0x48d4b0`

## Exhaustion table

`Exhaustion.dbc` is loaded at startup. The loader accepts only 15 fields and 60 bytes a record; any other layout is a fatal
error. It builds an index by row id: the cell `index[row.id]` holds the row. The store keeps the index at `0xc0dd78` and the
highest id at `0xc0dd7c`, -1 until loaded. A row is the id (`+0`), a threshold (`+4`), a factor, a float (`+8`), two floats
that no Lua function reads (`+0xc`, `+0x10`) and the name, one pointer per locale from `+0x14`.

Addresses: `0x544da0`, `0x544e4e`, `0x544e86`, `0x544fb9`, `0x53a520`, `0x53a527`, `0x53a531`, `0x578bf0`, `0xc0dd70`, `0xc0dd78`, `0xc0dd7c`
Confidence: verified
Proof: one
Test: none

The field count test is `cmp eax,0xf` (`0x544e4e`) and the record size test `cmp eax,0x3c` (`0x544e86`), each ending in the
fatal error. The index store is `mov [edi+edx*4],eax` (`0x544fb9`) for each row, with the row id in `edx`. `0x53a520` sets the
index to null (`0x53a527`) and the highest id to -1 (`0x53a531`) at startup. The file name comes from `0x578bf0`.

## Rest state

`GetRestState()` returns three values for the rest state byte of the active player (byte 3 of `PLAYER_BYTES_2`, at `+0x1b`
of the player block): the row id, the row's name for the client locale and the row's factor. The byte indexes the table
directly, as a row id. A byte above the highest id, a byte with no row, or an absent player returns three nils. A null
name pointer returns nil in its place.

Addresses: `0x48d350`, `0x48d373`, `0x48d379`, `0x48d381`, `0x48d38f`, `0x48d396`, `0x48d3ab`, `0x48d3b6`, `0x48d3c7`, `0x48d3ce`
Confidence: verified
Proof: one
Test: none

The byte load is `movzx` (`0x48d379`), so the `jl` after it never branches. The bound is `cmp eax,[0xc0dd7c]; jg` (`0x48d381`).
The three pushes are `fild [esi]` (`0x48d396`), the name at `[esi + locale * 4 + 0x14]` through `lua_pushstring` (`0x48d3ab`), which
pushes nil for a null pointer, and `fld [esi+8]` (`0x48d3b6`). The locale index is the client's global one. The failure tail
pushes three nils and returns 3 (`0x48d3ce`).

## Experience bonus

`GetXPExhaustion()` returns the rested pool times the factor of the row with id 1, as a number. The pool is the
unsigned dword `PLAYER_REST_STATE_EXPERIENCE` at `+0xf6c` of the player block. With no row for id 1 the call faults: `fmul [edx+8]` (`0x48d484`) reads the row without a null test. The result is nil for an absent player and
for every rest state byte except 1; a rested player with an empty pool gets 0. The factor is the table's, not a constant,
and does not depend on the current state's row. When the unit field array's GUID differs from the active player's the
pool counts as 0.

Addresses: `0x48d3f0`, `0x48d41c`, `0x48d41e`, `0x48d427`, `0x48d42c`, `0x48d43b`, `0x48d441`, `0x48d446`, `0x48d448`, `0x48d451`, `0x48d462`, `0x48d468`, `0x48d474`, `0x48d47e`, `0x48d484`, `0x48d49b`
Confidence: verified
Proof: one
Test: `ui::luabind_48d3f0_xp_exhaustion`

The row is `index[1]` (`0x48d42c`), taken when the highest id is at least 1 and 0 otherwise; the multiply `fmul [edx+8]`
(`0x48d484`) reads it without a null test. The rest byte test is `dec eax; jne` (`0x48d445`, `0x48d446`), so only the value 1
reaches the arithmetic. The pool is widened to a quadword with a zero upper dword (`0x48d474` to `0x48d47e`), so it is unsigned, then
multiplied by the float factor. The GUID guard compares the object's GUID words with a fresh active-player GUID
(`0x48d451` to `0x48d45c`) and sets the pool to 0 on a mismatch.

Bit-exact coverage: `ui::luabind_48d3f0_xp_exhaustion` covers `0x48d47e` to `0x48d48a` of `0x48d3f0`.

## Resting flag

`IsResting()` returns the number 1 when bit 5 of `PLAYER_FLAGS` (the dword at `+8` of the player block) is set for the
active player, and nil otherwise, including for an absent player. It never returns a boolean.

Addresses: `0x516ea0`, `0x516ec2`, `0x516ec8`, `0x516ecb`, `0x516ece`, `0x516ed3`, `0x516ee8`
Confidence: verified
Proof: one
Test: none

The test is `shr ecx,5; test cl,1` (`0x516ecb`, `0x516ece`). The true leg pushes the double 1.0 (`push 0x3ff00000; push 0`,
`0x516ed3`) and the false leg `lua_pushnil` (`0x516ee8`); both return 1 value.

## Time to well rested

`GetTimeToWellRested()` pushes nil whatever the state and returns 1.

Addresses: `0x48d4b0`, `0x48d4b5`
Confidence: verified
Proof: one
Test: none

The whole body is `call lua_pushnil; mov eax,1; ret` (11 bytes), with no player lookup and no table read.
