# gametime

The game clock: the server-synced time of day the world is lit and sounded by. One object, at
`0xce8538` (0x70 bytes), is built by a static initializer and driven by the server's time messages
and a per-frame update. Its methods are `0x643300` to `0x643c30`. The object holds a time (minute at
`+0`, hour at `+4`, then weekday, day, month and year), two offsets, a fractional-minute accumulator,
a time scale, the tick of the last minute and a table of callbacks per minute of the day. The
table's template code (`0x6433f0`, `0x643cb0` and after `0x643d10`) has no identified object file and
stays `unassigned`, as do the message handlers and the singleton's initializer in `0x6c5bc0` to
`0x6c6200`; their calls are stated here. The time helpers it calls, `0x6424c0` and the rest of
`0x642300` to `0x642c30`, belong to `base`.

Roots: `0x6435a0`, `0x6436c0`, `0x6434d0`, `0x643820`, `0x643a70`, `0x643300`, `0x643ad0`

## Clock state

`0x643300` builds the clock: `+0x20` (minute offset), `+0x24` (day offset), `+0x28` (a count of minutes
advanced), `+0x30` (the minute fraction) and `+0x34` (minutes still to skip) are 0; `+0x2c`, the scale
in game minutes per real second, is `0x3c888889`, 1/60; `+0x38` is the tick `0x42c010()`; `+0x3c`, the
minute offset, is 0; the time at `+0` is −1 in every field (`0x642300`). A callback table with vtable
`0x80e44c` starts at `+0x48` and `+0x6c` is −1. Static initializer `0x6c5bc0` jumps to the
constructor with `ecx = 0xce8538`. The table's first vtable slot, `0x643bb0`, frees one entry: its callback
records, then the entry through `0x643c30`.

Addresses: `0xce8538`, `0x643300`, `0x642300`, `0x6c5bc0`, `0x42c010`, `0x80e44c`, `0x643bb0`, `0x643c30`, `0x515ee0`, `0x60a0a4`, `0x457922`
Confidence: verified
Proof: one
Test: none

Stores `0x64330f` to `0x64335a`; the minute is `[0xce8538]` and the hour `[0xce853c]`. `Script::GetGameTime` (`0x515ee0`) returns the
hour, then the minute: −1 and −1 until the first time message. Other users of the clock: `IsDaytime` (`0x60a0a4`, a range test of the clock) and the sound frame (`0x457922`). `0x6424c0` returns `hour·60 + minute`, 0 when either is
negative; `0x6424e0` sets hour and minute from a number of minutes (`÷ 60`, remainder). `0x642320`
returns the day number, the seconds `mktime` gives for the date divided by 86,400.

## Day phase

`0x643a70` returns the time of day as a fraction. `x = (tick − [+0x38]) · 0.001f · [+0x2c] + [+0x3c]`,
with the tick difference an unsigned 32-bit value; `x` loses 1440.0 while it exceeds 1440.0; the result
is `x · (1/1440)` (`0x3a360b61`) left in `st0`, not rounded. At `x = 1440` that is `1440 · f32(1/1440)` = 1.0000000242, so the value is in [0, 1] only once stored: both callers store it with `fstp DWORD` (`0x482be2`, `0x6c5f77`), which rounds it to 1.0f.

Addresses: `0x643a70`, `0x42c010`, `0x801360`, `0x80e460`, `0x804480`
Confidence: verified
Proof: one
Test: `gametime::gametime_643a70_day_phase`

`sub eax,[esi+0x38]`, then `fild qword` of `{eax, 0}` (`0x643a81` to `0x643a8b`), `fmul [0x801360]`
(0.001f), `fmul [esi+0x2c]`, `fadd [esi+0x3c]`; the loop `fcom [0x80e460]`, `test ah,0x41; jne` leaves at
`x <= 1440` or NaN (`0x643aa6`), else `fsub` and repeat; `fmul [0x804480]` ends it. At 53 bits. The test
runs the whole function with the tick stubbed. Two callers: `StampLightState` (`0x482bdd`), each frame, and
the bridge `0x6c5f50` (`0x6c5f72`).

## One minute

`0x643ad0` advances the clock one minute. The new minute of the day is `(minute of day + 1) mod 1440`;
the hour and minute are set from it; at 0 it adds one calendar day; `+0x28` is incremented; the callbacks for
the new minute run (`0x643b30`); `+0x38` takes the tick; `+0x3c` becomes `(float)newMinute + [+0x30]`.

Addresses: `0x643ad0`, `0x643b30`, `0x6423c0`, `0x6424c0`, `0x6424e0`
Confidence: verified
Proof: one
Test: `gametime::gametime_643ad0_advance_offset`

`0x643ad8` to `0x643af3` (`idiv 0x5a0`); the day step is `0x6423c0(1, 0)` at `0x643afc` when `edi == 0`.
`0x643b11` to `0x643b1b`: `fild`, `fadd [esi+0x30]`, `fstp` to `[esi+0x3c]`. The test runs only that float
store (slice). `0x643b30(minute)` finds the entry for the minute in the table at `+0x48` and calls each of its
callbacks in list order as `callback(clock, argument)`, stdcall (`0x643ba1`).

## Per-frame advance

`0x6435a0(dt)` adds `dt · [+0x2c] + [+0x30]`, stored as a float in `+0x30`. When `+0x34` (minutes to skip)
is not 0 and the sum is at least 1.0, `k = min(+0x34, trunc(sum))` is taken from both. Then, while `+0x30 >= 1.0`, it
subtracts 1.0 and advances a minute. `0x6c5d50` calls it with the float at `[ecx]`; the client tick handler `0x401ec0` calls `0x6c5d50`.

Addresses: `0x6435a0`, `0x6c5d50`, `0x401ec0`, `0x40a2b0`, `0x401684`, `0x41fc90`
Confidence: verified
Proof: one
Test: `gametime::gametime_6435a0_minute_accum`

`fmul`, `fadd`, `fstp` (`0x6435ac` to `0x6435b8`); the guard is `fcomp [0x8015b8]` and `test ah,1; jne`
(`0x6435ce`), skipping a sum below 1.0 or NaN; `__ftol` then `cmp edi,eax; jae` picks the smaller, unsigned
(`0x6435db`); `fsubr` and `fstp` to `+0x30`; the loop compares with the float 1.0 at `0x7ff9d8`. The test
stubs `0x643ad0` and runs the whole function. `0x401ec0` is the category 5 handler `ClientInitializeGame` registers (`0x401684`); `core/loop#frame-time` dispatches it each tick with a pointer to the frame-time float. It
calls `0x6c5d50` with that pointer in `ecx`, which calls `0x6435a0` on `0xce8538` (`0x6c5d56`, `0x6c5d5b`) and returns 1.

## Setting the time

`0x6434d0(time, force)` copies the 28-byte time, adds the minute offset `+0x20` to its minute of day (plus 1440 when the sum is
negative, else modulo 1440) and the day offset `+0x24` in days, and stores it as the clock's time.
With `force` set it then steps the time back one minute (minute 0 becomes 59 of the previous hour, hour 0 of the previous day) and
advances one minute, so the callbacks for the new minute run and `+0x38` and `+0x3c` restart. A forced set shows the sent hour and
minute taken modulo a day, and 00:01 when either is unset.

Addresses: `0x6434d0`, `0x6423c0`, `0x643ad0`, `0x6424c0`, `0x6424e0`
Confidence: verified
Proof: two
Test: `gametime::gametime_6434d0_set_time`

Copies `0x6434eb`, `0x643537`; the offsets `0x6434f2` to `0x643523`; the step back `0x64353f` to `0x643575`, where `0x643553` stores
hour 23 before the day step `0x64355a` (`0x6423c0(-1, 0)`). Callers: `0x6c5ee9` and `0x6c617b` with `force = 1`. The advance (`0x643ad0`) sets hour and minute from `hour·60 + minute + 1` modulo 1440, with 0 for the sum when either is unset (`0x6424d3`). The test runs the
function whole, `0x6424c0` and `0x6424e0` as the exe has them, `0x643ad0` and `0x6423c0` recording their arguments and the time they find. A second reading from `0x6c5d70` agrees.

## Server sync

`0x6436c0(time, force)` compares the sent time with the clock's. The sent time gets the same offsets as above.
`d = ((sentDay − day)·1440 + sentMinuteOfDay − minuteOfDay) mod 1440`, signed with the dividend, each day being `0x642320` of the time.
Without `force` and with `d <= 0` the clock takes the sent time, `+0x34` grows by `−d` and the clock advances `−d` minutes (back
to the minute it showed), so the next `−d` minutes of frame time are skipped. Otherwise the clock advances `d` minutes when `d > 0`, then
takes the sent time.

Addresses: `0x6436c0`, `0x6c606b`, `0x643ad0`, `0x642320`
Confidence: verified
Proof: two
Test: `gametime::gametime_6436c0_sync_time`

`0x64371d` to `0x643753` (`0x642320`, `imul 0x5a0`, `idiv`); the branch `0x64375a` to `0x643760`
(`jne`, then `jg` falling to `neg esi`); the forward loop `0x6437a8` to `0x6437b0`; the copy `0x6437be`; the
`+0x34` update and loop `0x6437c4` to `0x6437d9`. One live caller: the opcode 0x43 handler `0x6c6010` with `force = 0`. The test runs the function whole with
`0x642320` returning a function of the date alone and `0x643ad0`, `0x6423c0`, `0x642c80` and `0x6dd240` recording. A second reading from `0x6c5d70` agrees.

## Sync log and day numbers

Both log calls of `0x6436c0` go to `0x6dd240`, a bare `ret`, so the sync writes nothing. A `d` above 1 passes it `GameTimeSync: skipping forwards %d game minutes,
(current = %s, newtime = %s)`; a stall passes `GameTimeSync: delta=%d, differential=%d, HoursAndMinutes=%d` with 0, the new `+0x34` and the minute of day. The day
number the sync compares is `mktime` of local midnight divided by 86,400, so a host daylight-saving change can give two consecutive dates one number: the dividend
loses 1440, and without `force` a step forward of `d` minutes becomes a stall of `1440 − d`.

Addresses: `0x6dd240`, `0x642c80`, `0x642320`, `0x643779`, `0x64378a`, `0x64379c`, `0x6437f3`, `0x6436c0`
Confidence: verified
Proof: one
Test: `gametime::gametime_6436c0_sync_time`

The first log follows two `0x642c80` calls (`0x643779`, `0x64378a`) whose texts are its last two arguments (`0x64379c`); the second is `0x6437f3`. `0x6dd240` has 32 direct
calls, no `jmp` and no stored address. The loss of 1440 follows from the dividend's day term, the difference of two day numbers. The test records the log arguments and the formatter calls; the time-zone effect is derived, not run.

## Time scale

`0x643820(scale)` stores `scale` in `+0x2c`, in game minutes per real second, clamped to `[1/60, 60]`: below `0x3c888889` (negatives and zero of
either sign included) it stores that, above 60.0 (`0x42700000`) it stores 60.0, and a NaN is stored with its bits. It returns the scale the clock
held, in `st0`.

Addresses: `0x643820`, `0x80e45c`, `0x80e448`
Confidence: verified
Proof: two
Test: `gametime::gametime_643820_set_scale`

Compares `fcomp [ebp+8]` with the bounds (`0x64382c`, `0x643847`): `jne` on `test ah,0x41` keeps the argument
at or above the low bound, `jp` on `test ah,5` at or below the high one, and an unordered compare takes both. `fld [ecx+0x2c]` (`0x643823`) stays
on the x87 stack through the three exits (`0x64385c`, `0x643879`, `0x643883`). Callers: the handlers `0x6c5de0`
(opcode 0x47) and `0x6c5e80` (0x42), `0x6c5e31` and `0x6c5efc`. The test reads the old scale through
`call_f64` and the stored one for bounds, a step either side, zero, the infinities, denormals, quiet NaNs and random bits. A signalling NaN is left out: `fld` quiets
it on the processor and not in the emulator. A second reading from `0x6c5d70` agrees.

## Scale change effects

`0x643820` writes only `+0x2c`, so a new scale leaves the day phase base `+0x38` and `+0x3c` as the last minute step stamped them: `0x643a70` applies the new scale to the real
time since that step, and the phase moves by `(tick − [+0x38])·0.001·(stored − held)` game minutes at once. The scale also sets how long a game minute lasts in `0x6435a0`,
`1/scale` real seconds, which paces the minute `GetGameTime` returns. A NaN scale makes the minute fraction `+0x30` NaN on the next frame, and it stays NaN: the per-frame advance stops
and `0x643a70` returns NaN.

Addresses: `0x643820`, `0x643a70`, `0x6435a0`, `0x515ee0`, `0x64331f`, `0x6435be`, `0x6435f6`, `0x64361b`, `0x6435d1`, `0x643607`, `0x643a97`, `0x643b15`, `0x643b1b`
Confidence: verified
Proof: one
Test: `gametime::gametime_643a70_day_phase`, `gametime::gametime_6435a0_minute_accum`, `gametime::gametime_6435a0_nan_accumulator`

`0x643820` stores through `[ecx+0x2c]` alone. `0x643a70` adds `[+0x3c]` to the elapsed time times 0.001 and `[+0x2c]` (`0x643a97`); the stamps are written by the constructor and by `0x643ad0`
(`0x643b15`, `0x643b1b`). The stores to `+0x30` are the constructor's (`0x64331f`) and `0x6435a0`'s (`0x6435be`, `0x6435f6`, `0x64361b`). A NaN sum is unordered, sets C0 and leaves
`0x6435a0` at both of its `test ah,1` guards (`0x6435d1`, `0x643607`) with no minute, and `0x643a70` leaves its reduction loop for the NaN. The NaN test runs `0x6435a0` whole with
a NaN scale and then valid ones, and `0x643a70` whole.

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
`Gamespeed set from %.03f to %.03f` (class 0) prints the scale held and the rate as received, not as clamped, and it returns 1. It signals no event.

Addresses: `0x6c5de0`, `0x419130`, `0x401d10`, `0x643820`, `0x63cd00`, `0x64a7f0`, `0x63cb50`, `0x6c5df6`, `0x6c5dff`, `0x6c5e04`, `0x6c5e16`, `0x6c5e31`, `0x6c5e36`, `0x6c5e54`, `0x6c5e64`, `0x418ec4`, `0x418ec6`, `0xce8538`
Confidence: verified
Proof: two
Test: `gametime::gametime_6c5de0_game_speed_set`, `gametime::gametime_643820_set_scale`

`0x6c5df6` reads the float into the argument slot `[ebp+0xc]` once `esi` holds the store; `0x6c5dff` is `call [edx+0x14]`, slot 5 of the
store's vtable, `IsRead` (`0x401d10`), and `0x6c5e04` branches on it. The malformed path pushes the cursor `[esi+0x14]`, the size `[esi+0x10]`,
the opcode (`edx`, kept in `edi`), 0 and the format for `0x63cd00` (`0x6c5e16`), then returns 0. The accept path calls `0x643820` with `ecx = 0xce8538`
(`0x6c5e31`); `0x6c5e36` loads the received float over the old scale left in `st0`, two `fstp qword` make them the doubles of `0x64a7f0`
(256-byte buffer, `0x6c5e54`), and `0x6c5e64` writes it. A read past the size ends at `0x418ec4` (`inc ecx`, the size plus one) and `0x418ec6`
(the store to the cursor `[esi+0x14]`) without writing its output. The scale's effects on the phase and the minute pace are in `Scale change effects`. The test runs the handler whole beside the model, with the
store's reads and the clamp as the exe has them. A second reading from `0x6c5d70` agrees.

## Game time update

`SMSG_GAMETIME_UPDATE` (`0x43`), handler `0x6c6010`: the body is one little-endian dword, a packed date (`Packed date fields`). Read to its end,
it is decoded and passed to the clock's sync with `force = 0` (`Server sync`), and the handler returns 1: a date ahead of the clock steps
the clock forward minute by minute, then sets it; one behind sets it, steps forward again to the minute shown and skips as many minutes of
frame time; one equal sets it. The handler writes no console line of its own. A body not read to its end gets the malformed line (as `0x47`: Read = Len + 1 after a short
read), return 0 and no change. It signals no event.

Addresses: `0x6c6010`, `0x418eb0`, `0x401d10`, `0x642c30`, `0x6436c0`, `0x63cd00`, `0x6c6023`, `0x6c602c`, `0x6c6043`, `0x6c605b`, `0x6c606b`, `0x418ec6`, `0xce8538`
Confidence: verified
Proof: two
Test: `gametime::gametime_6c6010_game_time_update`, `gametime::gametime_6436c0_sync_time`, `gametime::gametime_642c30_decode_time`

`0x6c6023` reads the dword into `[ebp+0xc]`, `0x6c602c` is the `IsRead` call and `0x6c6033` to `0x6c6052` the malformed path. `0x6c605b` decodes
into the local record `[ebp-0x1c]`, `0x6c6060` to `0x6c6065` push 0 and the record, and `0x6c606b` calls `0x6436c0` on the clock; `eax = 1` follows (`0x6c6071`). `GetGameTime` returns the clock's fields: the sent hour and minute for a date at or ahead of the clock, the clock's own for one behind. `0x418eb0` ends a short read at `0x418ec6` as `0x419130` does. The test runs the handler whole with the sync recording its record and `force`. A second reading from `0x6c5d70` agrees.

## Game time set

`SMSG_GAMETIME_SET` (`0x45`), handler `0x6c6120`: the body is one little-endian dword, a packed date. Read to its end, it is decoded
and passed to the clock's set with `force = 1` (`Setting the time`), then `0x6c5f50` stamps the light state and prints `Time set to <time>`
(`Time changed bridge`), and the handler returns 1. `GetGameTime` returns the sent time from then on. A body not read to its end gets the
malformed line (Read = Len + 1 after a short read), return 0 and no change. It signals no event.

Addresses: `0x6c6120`, `0x418eb0`, `0x401d10`, `0x642c30`, `0x6434d0`, `0x6c5f50`, `0x63cd00`, `0x6c6133`, `0x6c613c`, `0x6c6153`, `0x6c616b`, `0x6c617b`, `0x6c6180`, `0xce8538`
Confidence: verified
Proof: two
Test: `gametime::gametime_6c6120_game_time_set`, `gametime::gametime_6434d0_set_time`, `gametime::gametime_6c5f50_time_changed`, `gametime::gametime_642c30_decode_time`

The shape is that of `0x6c6010`: the read `0x6c6133`, the `IsRead` call `0x6c613c`, the malformed path `0x6c6143` to `0x6c6162`, the decode
`0x6c616b` into `[ebp-0x1c]`, `0x6c6170` to `0x6c6175` pushing 1 and the record for `0x6434d0` (`0x6c617b`), the bridge (`0x6c6180`) and `eax = 1`. The sent
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
constructor `0x643300` has one caller, the `jmp` of `0x6c5bc0` (`0x6c5bc5`), which the initializer thunk `0x6c5bb0` calls. `0x42` repeats the decode, set and bridge of `0x45` and the clamp and speed line of `0x47` in its own body (`Time speed packet`); `0x49` is its own entry (`Server time message`). The five handlers share no body. Events: `tables/event_sites.tsv` holds
no signal site from `0x6c5bc0` to `0x6c61e0`, in the clock's methods `0x643300` to `0x643c30` or in `0x66ff60` (control: `0x4c4d0a`), and a walk of direct calls from
the five handlers reaches neither `FrameScript_SignalEvent` (`0x703e50`) nor `FrameScript_SignalEvent2` (`0x703f50`), as the same walk from `0x4c4cf0` reaches
`0x703e50`. The indirect calls on the way are the store's slots, the four callbacks and the minute callbacks of `0x643b30`, whose only adder `0x643890` is unreached.

## Server time message

`SMSG_SERVERTIME` (`0x49`), handler `0x6c6080`, registered by the same function: the body is one little-endian dword, a packed date (`Packed date fields`). Read to its end, it is
decoded, formatted by `WowTime::FormatTime` into a 128-byte buffer and written as `Server game time is %s` in class 0, and the handler returns 1; it touches neither the clock nor
the light state. A body not read to its end gets the malformed line (Read = Len + 1 after a short read), return 0.

Addresses: `0x6c6080`, `0x418eb0`, `0x401d10`, `0x642c30`, `0x642c80`, `0x64a7f0`, `0x63cb50`, `0x63cd00`, `0x6c6096`, `0x6c609f`, `0x6c60b6`, `0x6c60ce`, `0x6c60e1`, `0x6c60f8`, `0x6c6108`
Confidence: verified
Proof: one
Test: `gametime::gametime_6c6080_server_time`

The shape is that of `0x6c6010`: the read `0x6c6096`, the `IsRead` call `0x6c609f`, the malformed path `0x6c60a6` to `0x6c60c5` and the decode `0x6c60ce` into `[ebp-0x1c]`. `0x6c60d3` to `0x6c60e1` push 128 and call
`0x642c80` on the record with a buffer at `[ebp-0x9c]`; `0x6c60e6` to `0x6c60f8` pass the text it returns to `0x64a7f0` with the format and a 256-byte buffer; `0x6c6108` writes that buffer with
`edx = 0`, and `eax = 1` follows. The body names no address of the clock or the light state. The test runs the handler whole with the formatter and the sinks recording.

## Day number and time text

`WowTime::GetDayNumber` (`0x642320`) returns 0 when the year, month or day field is negative.
Otherwise it builds a `tm` with day `day + 1`, month `month`, year `year mod 100 + 100`, daylight
saving −1 and every other field 0, and returns `(mktime(tm) + 0x6423b0()) / 86400`, truncated
toward zero; `0x6423b0` returns 0. `WowTime::FormatTime(time, buffer, size)` (`0x642c80`)
writes `Not Set` when `0x642c10` gives 0 for the time, else `%s/%s/%s (%s) %s:%s` of month + 1,
day + 1, year + 2000, the weekday's `Sun` to `Sat` (`Any` when negative), hour and the minute as
two digits; any other negative field prints `A`. It returns `buffer`.

Addresses: `0x642320`, `0x64232d`, `0x64234c`, `0x642372`, `0x64237b`, `0x6423b0`, `0x642c80`, `0x642c91`, `0x642cd5`, `0x642d64`, `0x642dc6`, `0x642df4`, `0x642c10`
Confidence: verified
Proof: one
Test: none

The record is the clock's: minute `+0`, hour `+4`, weekday `+8`, day `+0xc`, month `+0x10`, year
`+0x14`. `0x64232d` to `0x642339` test the year, month and day with `jl`; `0x64234c` divides the
year by 100 (`idiv`) and adds 100 to the remainder; `0x642372` calls `_mktime`. `0x64237b` multiplies
by 0xc22e4507, adds the dividend, shifts right 16 and adds the sign bit: a signed division by
86,400. In `0x642c80`, `0x642cd5` adds 2000; the weekday names are the seven string pointers from
0x865864 (`0x642d64`); `0x642dc6` formats the minute with `%2.2i`.

## Unreached functions

`0x643450` applies new minute and day offsets and sets the clock again. `0x643640` sets the clock from the host's
local time, forced or by sync. `0x643810` returns the scale. `0x643890` adds a callback for a minute of the day;
`0x6439e0` removes one. `0x6c61e0` stores a callback in the first empty of the four slots at `0xce85a8`, or clears the slot that
holds it. None has a caller or a stored address. The minute offset `+0x20` and the day offset `+0x24` stay 0 in the live client: the other stores to them are the constructor's.

Addresses: `0x643450`, `0x643640`, `0x643810`, `0x643890`, `0x6439e0`, `0x6c61e0`, `0x6434a6`, `0x6434c1`, `0x64330f`, `0x643312`
Confidence: inferred
Proof: none
Test: none

Census: the two scans of `docs/METHOD.md` over the image for each address: no `e8`/`e9` target and no dword; the
control is `0x643ad0` with six calls. The meanings are read from the first instructions only, but for `0x6c61e0`, whose body is read in full:
`ecx` selects add or remove, `edx` is the callback, and an add takes the first empty slot without looking for the callback in a later one.
