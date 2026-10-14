# minigame

The client side of a board minigame the server runs. `Minigame_C.cpp` holds a 0x70-byte object with
a nine-cell board; a second object file after it, `0x728b10` to `0x728ee0`, holds the move check
`0x728b80`. The object's three virtual methods apply a state packet, check a move and push the board
to Lua; the packet and Lua handlers that call them are in the interface's `0x4c4be0` to `0x4c4e00`.
The client decides nothing but whether a move is legal; turns and wins arrive from the server. Both
files also hold the shared header's initializers: +∞, 2π, 1/2π, 17 squared constants and a 5.0 each,
stored in floats from `0xcf050c` to `0xcf05b4` that nothing reads.

Roots: `0x728560`, `0x7285e0`, `0x728650`, `0x728700`, `0x728740`, `0x728770`, `0x7284a0`, `0x7284c0`

## The object

`0x728560(type, idLow, idHigh)` (`ecx` = type, `ret 8`) returns null unless `type == 1`; otherwise it
allocates 0x70 bytes (file tag `0x874020`, line 0x22) and returns it. The vtable `0x8126b4`
is at `+0`, the type 1 at `+8`, the 64-bit game id at `+0x10`; every other field is 0: `+0xc` a status
byte, `+0x18` and `+0x20` two 64-bit values, `+0x28` and `+0x29` bytes, and a 64-byte board from `+0x2a`.
The vtable's slots are `0x728700`, `0x728740` and `0x728770`. The singleton pointer is `0xb71e5c`.

Addresses: `0x728560`, `0x8126b4`, `0xb71e5c`, `0x874020`
Confidence: verified
Proof: one
Test: none

`dec ecx; jne` returns 0 (`0x728568`); `SMemAlloc(0x70, 0x874020, 0x22, 0)` (`0x72856b` to `0x728575`); the
stores `0x728586` to `0x7285c3`; `rep stos` of 0x10 dwords at `+0x2a` (`0x7285a2` to `0x7285c6`). A fresh board
holds 0, not 0xFF. The caller `0x4c4c70` stores the result in `0xb71e5c` without testing it, so a setup
packet of any other type calls `0x7285e0` with a null object and faults at `0x7285fc`.

## Setup packet

Opcode 0x2f6, handler `0x4c4c70`: a 64-bit game id, then a type byte; the old object is freed and replaced
by `0x728560`; `0x7285e0(packet)` fills it from the rest; the Lua events 0x216 and 0x217 follow. The fill reads
a status byte into `+0xc`, two 64-bit values into `+0x18` and `+0x20`, 64 bytes into the board at `+0x2a`, and
when the status is 2 a byte into `+0x29`.

Addresses: `0x7285e0`, `0x4c4c70`, `0x4c4be0`, `0x703e50`
Confidence: verified
Proof: one
Test: none

`CDataStore::GetGuid` (`0x4190b0`) reads the two 64-bit values, `0x419aa0` the 64 board bytes
(`0x728612` to `0x72861a`), and `cmp [esi+0xc],2` guards the last byte (`0x72861f`). `0x4c4be0` registers
`0x4c4c70` for 0x2f6 and `0x4c4cf0` for 0x2f7 with `0x5ab650`. The events are signalled through `0x703e50`
(`0x4c4cd5`, `0x4c4cda`).

## State packet

Opcode 0x2f7, handler `0x4c4cf0`: with an object present, vtable slot 0 (`0x728700`) reads nine bytes into the
board cells `+0x2a` to `+0x32`, then one byte into `+0x28`; event 0x217 follows, object or not.

Addresses: `0x728700`, `0x4c4cf0`
Confidence: verified
Proof: one
Test: none

The loop `0x728710` to `0x72871f` runs nine `CDataStore::GetUInt8`; the last read stores at `+0x28`
(`0x728722` to `0x728728`). The handler returns 1 and never passes the packet when the singleton is null
(`0x4c4cf9`).

## Move check

`0x728b80(idx)` (`ecx` = the object + 8; `ret 4`) returns true when
`0 <= idx <= 9` and the byte at `board[idx]` is 0xFF. `idx == 9` passes the bound and reads the byte after
the ninth cell. Vtable slot 1, `0x728740(byte, idx)`, calls it with `this + 8` and `idx`; the byte is not used.

Addresses: `0x728b80`, `0x728740`
Confidence: verified
Proof: one
Test: `minigame::minigame_move_legal_difftest`

`test; jl` and `cmp 9; jg` (`0x728b86` to `0x728b8d`); `idiv 3` then `lea edx,[edx+eax*2]; add eax,edx`
rebuild `idx`; `cmp byte [eax+ecx+0x22],0xff; sete` (`0x728b9e`). The test runs the whole function. A null
`this` makes `0x728740` pass a null base (`0x728760`) and read near address 0.

## Making a move

Lua `MakeMinigameMove(moveType, param)` (`0x4c4d80`) takes two numbers and calls
`0x728650(trunc(moveType − 1), trunc(param) − 1)` with the singleton. `0x728650` runs the move check on its second
argument; when it is true it sends CMSG 0x2f8: the 64-bit game id, the first argument as a byte and the
second as a 32-bit value. `0x728640` hands the packet to `ClientServices::Send`.

Addresses: `0x728650`, `0x728640`, `0x4c4d80`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

The call is `call [eax+4]` (`0x728666`), then `PutUInt32(0x2f8)`, `PutGuid`, `PutInt8`, `PutUInt32` in that
order (`0x728674` to `0x7286bb`) and `0x728640` (`0x7286c9`); a failed check sends nothing and returns no
value. The Lua function reads the numbers with `lua_tonumber`; the first goes through `fsub 1.0` and `__ftol`, the second through `__ftol` and `dec eax` (`0x4c4dae` to `0x4c4dcc`); a
missing or non-numeric argument raises the usage error.

## Lua queries

`GetMinigameState` (`0x4c4df0`) calls vtable slot 2, `0x728770`, which pushes the nine board cells as numbers
and returns 9. `GetMinigameType` (`0x4c4d50`) reads the object's `+8` and, for 1, pushes the string at table slot
1 of `0x848384`, which is null, so the call returns nil; the name `TicTacToe` sits in slot 0, which no
object reaches.

Addresses: `0x728770`, `0x4c4df0`, `0x4c4d50`, `0x848384`
Confidence: verified
Proof: one
Test: none

`0x728770` checks the Lua stack for 9 slots (`0x6f2f30`), then `fild`, `fstp qword` and `lua_pushnumber` per cell
(`0x728790` to `0x7287a3`). `0x4c4d57` and `0x4c4d59` pass 1 only; the load is
`[eax*4+0x848384]` (`0x4c4d5e`) and the dword at `0x848388` is 0 in the image with no writer (the operand scan finds
`0x848384` once and `0x848388` not at all), so `lua_pushstring` pushes nil.

## Constants nothing reads

Each of the two files opens with the shared header's initializers: `0x7284a0` and `0x728b10` copy +∞, `0x728510`
and `0x728b30` store π + π, `0x728530` and `0x728b50` store 1 divided by it. 34 more store a constant's square, 17 in
each file, and two store 5.0 (`0x728a40`, `0x728e30`). The floats lie between `0xcf050c` and `0xcf05b4`.

Addresses: `0x7284b0`, `0x7284c0`, `0x7284d0`, `0x7284f0`, `0x728500`, `0x728510`, `0x728520`, `0x728530`, `0x728540`, `0x7287c0`, `0x7287d0`, `0x7287f0`, `0x728800`, `0x728820`, `0x728830`, `0x728850`, `0x728860`, `0x728880`, `0x728890`, `0x7288b0`, `0x7288c0`, `0x7288e0`, `0x7288f0`, `0x728910`, `0x728920`, `0x728940`, `0x728950`, `0x728970`, `0x728980`, `0x7289a0`, `0x7289b0`, `0x7289d0`, `0x7289e0`, `0x728a00`, `0x728a10`, `0x728a30`, `0x728a40`, `0x728a50`, `0x728a60`, `0x728a80`, `0x728a90`, `0x728ab0`, `0x728ac0`, `0x728ae0`, `0x728af0`, `0x728b10`, `0x728b20`, `0x728b30`, `0x728b40`, `0x728b50`, `0x728b60`, `0x728bb0`, `0x728bc0`, `0x728be0`, `0x728bf0`, `0x728c10`, `0x728c20`, `0x728c40`, `0x728c50`, `0x728c70`, `0x728c80`, `0x728ca0`, `0x728cb0`, `0x728cd0`, `0x728ce0`, `0x728d00`, `0x728d10`, `0x728d30`, `0x728d40`, `0x728d60`, `0x728d70`, `0x728d90`, `0x728da0`, `0x728dc0`, `0x728dd0`, `0x728df0`, `0x728e00`, `0x728e20`, `0x728e30`, `0x728e40`, `0x728e50`, `0x728e70`, `0x728e80`, `0x728ea0`, `0x728eb0`, `0x728ed0`, `0x728ee0`, `0xcf050c`, `0xcf0510`, `0xcf0514`, `0xcf0518`, `0xcf0520`, `0xcf0524`, `0xcf0528`, `0xcf052c`, `0xcf0530`, `0xcf0534`, `0xcf0538`, `0xcf053c`, `0xcf0540`, `0xcf0544`, `0xcf0548`, `0xcf054c`, `0xcf0550`, `0xcf0554`, `0xcf0558`, `0xcf055c`, `0xcf0560`, `0xcf0564`, `0xcf0568`, `0xcf056c`, `0xcf0570`, `0xcf0574`, `0xcf0578`, `0xcf057c`, `0xcf0580`, `0xcf0584`, `0xcf0588`, `0xcf058c`, `0xcf0590`, `0xcf0594`, `0xcf0598`, `0xcf059c`, `0xcf05a0`, `0xcf05a4`, `0xcf05a8`, `0xcf05ac`, `0xcf05b0`, `0xcf05b4`
Confidence: verified
Proof: one
Test: none

Census: every dword in the image equal to an address from `0xcf050c` to `0xcf05b4`, at any alignment. Each occurs
once, as the store operand of its initializer, except `0xcf054c` and `0xcf05a0` (twice: the π + π float and its
reciprocal's read). The control: those two. No method of the object touches the array. The initializer slots
`0x7284c0` call `0x7284d0` (increment of the shared unit count `0xc2a3c4`, and `0x5b71c0` on the first) and
register `0x728500` (decrement; `0x5b71d0` at 0) with `atexit`.
