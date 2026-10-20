# gametime: messages

The server's game-time messages and what they do to the clock of `overview`: the packed date, the handlers of opcodes `0x47`, `0x43`, `0x45` and `0x49` (opcode `0x42` is `net/messages`' Time
speed packet), the bridge that stamps the light state after a new time, their registration, and the census of events. The handlers and the bridge are owned by no system yet, and the clock's
methods they call are `overview`'s.

Roots: none

## Packed date fields

`WowTime::WowDecodeTime` (`0x642c30`, the packed dword in `ecx`, a 28-byte record in `edx`) calls `0x642b30`, which stores seven `int32` fields:
minute (bits 0 to 5) at `+0`, hour (6 to 10) at `+4`, weekday (11 to 13) at `+8`, day (14 to 19) at `+0xc`, month (20 to 23) at `+0x10`, year (24 to 28)
at `+0x14` and a two-bit field (29 and 30) at `+0x18`. A field whose bits are all ones is stored as −1, and bit 31 is ignored. The day field holds the day − 1, the month field the month − 1 and the year field the year − 2000. No field is range-checked: a decoded hour reaches 30 and a minute 62.

Addresses: `0x642c30`, `0x642b30`, `0x642b39`, `0x642b3c`, `0x642be8`, `0x642c80`, `0x642cd5`, `0x642d0a`, `0x642d3b`
Confidence: verified
Proof: two
Test: `gametime::gametime_642c30_decode_time`

`0x642c30` pushes the six field addresses `edx+0x18` down to `edx+0x4` and enters `0x642b30` with `ecx` and `edx` as they are; `0x642b30` ends
`ret 0x18`. Each field is a shift and an `and` with its mask (`0x642b39` is `and eax,0x3f`, `0x642be8` `shr ecx,0x1d` and `and ecx,3`), a `cmp` with the
mask (`0x642b3c`) and a store of −1 on equality. A null field pointer skips the field. The test runs both functions whole on each field
at 0, 1, its mask less one and its mask, alone and among ones, on bit 31 and on random dwords. `0x642cd5` adds 2000 to the year and `0x642d0a` and `0x642d3b` add 1 to the month and the day. A second reading from `0x6c5d70` agrees.

## Game speed set

`SMSG_GAMESPEED_SET` (`0x47`), handler `0x6c5de0`: the body is one little-endian `float`, the rate. If the store is not at its end after the
read, it writes `Malformed message recieved: Id = %d, Len = %d, Read = %d` (class 0; the opcode, the store's size with the opcode's two bytes, its cursor),
returns 0 and changes nothing. A longer body leaves Read at 6; a short read sets the cursor to size + 1, leaving its output unwritten, so Read = Len + 1. Otherwise `0x643820` stores the rate clamped, the line
`Gamespeed set from %.03f to %.03f` (class 0) prints the scale held and the rate as received, not as clamped, and it returns 1.

Addresses: `0x6c5de0`, `0x419130`, `0x401d10`, `0x643820`, `0x63cd00`, `0x64a7f0`, `0x63cb50`, `0x6c5df6`, `0x6c5dff`, `0x6c5e04`, `0x6c5e16`, `0x6c5e31`, `0x6c5e36`, `0x6c5e54`, `0x6c5e64`, `0x419144`, `0x419146`, `0xce8538`
Confidence: verified
Proof: two
Test: `gametime::gametime_6c5de0_game_speed_set`, `gametime::gametime_643820_set_scale`

`0x6c5df6` reads the float into the argument slot `[ebp+0xc]` once `esi` holds the store; `0x6c5dff` is `call [edx+0x14]`, slot 5 of the
store's vtable, `IsRead` (`0x401d10`), and `0x6c5e04` branches on it. The malformed path pushes the cursor `[esi+0x14]`, the size `[esi+0x10]`,
the opcode (`edx`, kept in `edi`), 0 and the format for `0x63cd00` (`0x6c5e16`), then returns 0. The accept path calls `0x643820` with `ecx = 0xce8538`
(`0x6c5e31`); `0x6c5e36` loads the received float over the old scale left in `st0`, two `fstp qword` make them the doubles of `0x64a7f0`
(256-byte buffer, `0x6c5e54`), and `0x6c5e64` writes it. The float reader's read past the size ends at `0x419144` (`inc ecx`, the size plus one) and `0x419146`
(the store to the cursor `[esi+0x14]`) without writing its output. The scale's effects on the phase and the minute pace are in `Scale change effects`. The test runs the handler whole beside the model, with the
store's reads and the clamp as the exe has them. A second reading from `0x6c5d70` agrees.

## Game time update

`SMSG_GAMETIME_UPDATE` (`0x43`), handler `0x6c6010`: the body is one little-endian dword, a packed date (`Packed date fields`). Read to its end,
it is decoded and passed to the clock's sync with `force = 0` (`Server sync`), and the handler returns 1: a date ahead of the clock steps
the clock forward minute by minute, then sets it; one behind sets it, steps forward again to the minute shown and skips as many minutes of
frame time; one equal sets it. The handler writes no console line of its own. A body not read to its end gets the malformed line (as `0x47`: Read = Len + 1 after a short
read), return 0 and no change.

Addresses: `0x6c6010`, `0x418eb0`, `0x401d10`, `0x642c30`, `0x6436c0`, `0x63cd00`, `0x6c6023`, `0x6c602c`, `0x6c6043`, `0x6c605b`, `0x6c606b`, `0x418ec4`, `0x418ec6`, `0xce8538`
Confidence: verified
Proof: two
Test: `gametime::gametime_6c6010_game_time_update`, `gametime::gametime_6436c0_sync_time`, `gametime::gametime_642c30_decode_time`

`0x6c6023` reads the dword into `[ebp+0xc]`, `0x6c602c` is the `IsRead` call and `0x6c6033` to `0x6c6052` the malformed path. `0x6c605b` decodes
into the local record `[ebp-0x1c]`, `0x6c6060` to `0x6c6065` push 0 and the record, and `0x6c606b` calls `0x6436c0` on the clock; `eax = 1` follows (`0x6c6071`). `GetGameTime` returns the clock's fields: the sent hour and minute, as sent, for a date at or ahead of the clock, the clock's own for one behind. The dword reader `0x418eb0` ends a short read the same way, at `0x418ec4` and `0x418ec6`. The test runs the handler whole with the sync recording its record and `force`. A second reading from `0x6c5d70` agrees.

## Game time set

`SMSG_GAMETIME_SET` (`0x45`), handler `0x6c6120`: the body is one little-endian dword, a packed date. Read to its end, it is decoded
and passed to the clock's set with `force = 1` (`Setting the time`), then `0x6c5f50` stamps the light state and prints `Time set to <time>`
(`Time changed bridge`), and the handler returns 1. `GetGameTime` returns the sent hour and minute until the next minute step for in-range fields; an unset field gives 00:01 and an out-of-range one is taken modulo a day. A body not read to its end gets the
malformed line (Read = Len + 1 after a short read), return 0 and no change.

Addresses: `0x6c6120`, `0x418eb0`, `0x401d10`, `0x642c30`, `0x6434d0`, `0x6c5f50`, `0x63cd00`, `0x6c6133`, `0x6c613c`, `0x6c6153`, `0x6c616b`, `0x6c617b`, `0x6c6180`, `0x418ec4`, `0x418ec6`, `0xce8538`
Confidence: verified
Proof: two
Test: `gametime::gametime_6c6120_game_time_set`, `gametime::gametime_6434d0_set_time`, `gametime::gametime_6c5f50_time_changed`, `gametime::gametime_642c30_decode_time`

The shape is that of `0x6c6010`: the read `0x6c6133`, the `IsRead` call `0x6c613c`, the malformed path `0x6c6143` to `0x6c6162`, the decode
`0x6c616b` into `[ebp-0x1c]`, `0x6c6170` to `0x6c6175` pushing 1 and the record for `0x6434d0` (`0x6c617b`), the bridge (`0x6c6180`) and `eax = 1`. A short read ends at `0x418ec4` and `0x418ec6`. The sent
hour and minute come back unchanged for in-range fields (`Setting the time`). The test runs the handler whole with the set and the bridge recording. A second reading from `0x6c5d70` agrees.

## Time changed bridge

`0x6c5f50()` runs after the clock takes a new time. It stamps the first three words of the light state as each frame does: the minute of
day (`0x6424c0`), the day phase (`0x643a70`, rounded once to a float) and the day number (`0x642320`, as a float); calls `0x66ff60(1, 0)`,
the light commit with a full recompute and no eye stamp (`lighting/fog#world-light-commit`); calls each non-zero callback of the four at `0xce85a8` with the
hour in `ecx` and the minute in `edx`; and writes `Time set to %s` for the clock, class 0. The four slots stay 0: the one function that sets
one, `0x6c61e0`, is unreached. Callers: `0x6c5eee` (opcode 0x42) and `0x6c6180` (0x45).

Addresses: `0x6c5f50`, `0x6d48b0`, `0x6424c0`, `0x643a70`, `0x642320`, `0x66ff60`, `0xce85a8`, `0xce9b60`, `0x6c5f70`, `0x6c5f77`, `0x6c5f91`, `0x6c5f94`, `0x6c5fb6`, `0x6c5fd0`, `0x6c5fe7`, `0x6c5ff7`
Confidence: verified
Proof: two
Test: `gametime::gametime_6c5f50_time_changed`

`0x6d48b0` gives the light state `0xce9b60`; the stores are `0x6c5f70`, `0x6c5f77` (`fstp dword` of the unrounded day phase) and `0x6c5f91` (`fild` of the day number,
`fstp dword`); `StampLightState` stores the same three words each frame (`0x482bcc` to `0x482be5`). `0x6c5f8a` and `0x6c5f8c` load
`edx = 0` and `ecx = 1` for `0x66ff60`, where a zero `edx` skips the eye stamp. The loop `0x6c5fa0` to `0x6c5fbe` calls a non-zero slot (`0x6c5fb6`) with
`ecx = [0xce853c]` and `edx = [0xce8538]`. A dword census of `0xce85a8` finds the zeroing (`0x6c5dc7`), the reads and the stores of `0x6c61e0`; the control
is `0xce85ac` in the store at `0x6c5dcc`. The text is `FormatTime` of the clock into 128 bytes (`0x6c5fd0`), `Time set to %s` into 256 (`0x6c5fe7`), `ConsoleWrite` (`0x6c5ff7`).
The test runs the function whole on the
real clock and light state, with the tick, the day number, the sky update and the text sinks replaced. A second reading from `0x6c5d70` agrees.

## Server messages

`ClientInitializeGame` (`0x401570`) calls `ClientInitializeGameTime` (`0x6c5d70`), which registers five handlers with `0x5ab650` (the opcode in `ecx`, the handler in `edx`, a zero context):
`0x47` → `0x6c5de0`, `0x42` → `0x6c5e80`, `0x43` → `0x6c6010`, `0x49` → `0x6c6080`, `0x45` → `0x6c6120`, then zeroes the four time-changed callbacks.
`ClientDestroyGame` (`0x401ee0`) calls `0x6c61a0`, which clears the five and jumps to `0x6433f0` on the clock. The clock is built once, at start-up, and neither call resets it.
Each handler reads its body from the store, tests `IsRead`, and on a miss writes the malformed line (Read = Len + 1 after a short read) and returns 0; it ignores its context and time.

Addresses: `0x6c5d70`, `0x401570`, `0x401689`, `0x402020`, `0x6c61a0`, `0x6433f0`, `0x6c5bb0`, `0x6c5bc0`, `0x5ab650`, `0x6c5e80`, `0x6c6080`, `0x6c5de0`, `0x6c6010`, `0x6c6120`, `0x6c5f50`, `0xce8538`
Confidence: verified
Proof: one
Test: none

The registrations are `0x6c5d7c`, `0x6c5d8d`, `0x6c5d9e`, `0x6c5daf`, `0x6c5dc0`; the zeroing `0x6c5dc7` to `0x6c5dd6`; the clears `0x6c61a5` to `0x6c61cd` and the jump `0x6c61d7`. The
constructor `0x643300` has one caller, the `jmp` of `0x6c5bc0` (`0x6c5bc5`), which the initializer thunk `0x6c5bb0` calls. `0x42` repeats the decode, set and bridge of `0x45` and the clamp and speed line of `0x47` in its own body (`Time speed packet`); `0x49` is its own entry (`Server time message`). The five handlers share no body. Events are in `No event on direct paths`.

## Server time message

`SMSG_SERVERTIME` (`0x49`), handler `0x6c6080`, registered by the same function: the body is one little-endian dword, a packed date (`Packed date fields`). Read to its end, it is
decoded, formatted by `WowTime::FormatTime` into a 128-byte buffer and written as `Server game time is %s` in class 0, and the handler returns 1; it touches neither the clock nor
the light state. A body not read to its end gets the malformed line (Read = Len + 1 after a short read), return 0.

Addresses: `0x6c6080`, `0x418eb0`, `0x401d10`, `0x642c30`, `0x642c80`, `0x64a7f0`, `0x63cb50`, `0x63cd00`, `0x6c6096`, `0x6c609f`, `0x6c60b6`, `0x6c60ce`, `0x6c60e1`, `0x6c60f8`, `0x6c6108`, `0x418ec4`, `0x418ec6`
Confidence: verified
Proof: one
Test: `gametime::gametime_6c6080_server_time`

The shape is that of `0x6c6010`: the read `0x6c6096`, the `IsRead` call `0x6c609f`, the malformed path `0x6c60a6` to `0x6c60c5` and the decode `0x6c60ce` into `[ebp-0x1c]`. `0x6c60d3` to `0x6c60e1` push 128 and call
`0x642c80` on the record with a buffer at `[ebp-0x9c]`; `0x6c60e6` to `0x6c60f8` pass the text it returns to `0x64a7f0` with the format and a 256-byte buffer; `0x6c6108` writes that buffer with
`edx = 0`, and `eax = 1` follows. A short read ends at `0x418ec4` and `0x418ec6`. The body names no address of the clock or the light state. The test runs the handler whole with the formatter and the sinks recording.

## No event on direct paths

No walk by direct `call` and `jmp` from the five handlers (`0x6c5de0`, `0x6c5e80`, `0x6c6010`, `0x6c6080`, `0x6c6120`) reaches `FrameScript_SignalEvent` (`0x703e50`) or
`FrameScript_SignalEvent2` (`0x703f50`), and `tables/event_sites.tsv` has no signal site in the handlers' file, the clock's methods or `0x66ff60`. The walk leaves 571 indirect sites unresolved:
562 lie under the console write `0x63cb50` and the light commit `0x66ff60` (through `0x6d2210`), and nine are the data store's window slot (2), the handlers' `IsRead` calls (5), the
four time-changed callbacks (all 0) and the minute callbacks of `0x643b30` (none added: `0x643890` is unreached). Whether code under the first two signals an event is not settled.

Addresses: `0x703e50`, `0x703f50`, `0x6c5de0`, `0x6c5e80`, `0x6c6010`, `0x6c6080`, `0x6c6120`, `0x63cb50`, `0x66ff60`, `0x6d2210`, `0x643b30`, `0x643890`, `0x6c5f50`, `0x4c4cf0`, `0x4c4d0a`
Confidence: verified
Proof: one
Test: none

The walk follows every `call` and `jmp` whose target is a function row, in the rows' extents, from the five handlers: 829 functions. Its control is the same walk from `0x4c4cf0`, which reaches `0x703e50`; the table's
control is `0x4c4d0a`, the `MINIGAME_UPDATE` site in `0x4c4cf0`. The table holds no signal site from `0x6c5bc0` to `0x6c61e0`, from `0x643300` to `0x643c30` or in `0x66ff60`. The unresolved sites outside the two
closures are the `call [edx+0x14]` of each handler, the `call [edx+8]` of `0x418eb0` and `0x419130`, the `call eax` of the bridge and the `call [eax+0xc]` of `0x643b30`.
