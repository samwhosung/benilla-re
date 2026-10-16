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

Addresses: `0xce8538`, `0x643300`, `0x642300`, `0x6c5bc0`, `0x42c010`, `0x80e44c`, `0x643bb0`, `0x643c30`
Confidence: verified
Proof: one
Test: none

Stores `0x64330f` to `0x64335a`; the minute is `[0xce8538]` and the hour `[0xce853c]`, which
`Script::GetGameTime` returns in that order. `0x6424c0` returns `hour·60 + minute`, 0 when either is
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

Addresses: `0x6435a0`, `0x6c5d50`, `0x401ec0`, `0x40a2b0`
Confidence: verified
Proof: one
Test: `gametime::gametime_6435a0_minute_accum`

`fmul`, `fadd`, `fstp` (`0x6435ac` to `0x6435b8`); the guard is `fcomp [0x8015b8]` and `test ah,1; jne`
(`0x6435ce`), skipping a sum below 1.0 or NaN; `__ftol` then `cmp edi,eax; jae` picks the smaller, unsigned
(`0x6435db`); `fsubr` and `fstp` to `+0x30`; the loop compares with the float 1.0 at `0x7ff9d8`. The test
stubs `0x643ad0` and runs the whole function.

## Setting the time

`0x6434d0(time, force)` copies the 28-byte time, adds the minute offset `+0x20` to its minute of day (wrapped at
1440) and the day offset `+0x24` in days, and stores it as the clock's time. With `force` set it then steps the
time back one minute (minute 0 becomes 59 of the previous hour, hour 0 of the previous day) and advances one
minute, so the callbacks for the new minute run and `+0x38` and `+0x3c` restart.

Addresses: `0x6434d0`, `0x6423c0`, `0x643ad0`
Confidence: verified
Proof: one
Test: none

Copies `0x6434eb`, `0x643537`; the offsets `0x6434f2` to `0x643523`; the step back `0x64353f` to `0x643575`.
Callers: `0x6c5ee9` and `0x6c617b` with `force = 1`.

## Server sync

`0x6436c0(time, force)` compares the sent time with the clock's. The sent time gets the same offsets as above.
`d = ((sentDay − day)·1440 + sentMinuteOfDay − minuteOfDay) mod 1440`, signed with the dividend. Without `force`
and with `d <= 0` the clock takes the sent time, `+0x34` grows by `−d` and the clock advances `−d` minutes (back
to the minute it showed), so the next `−d` minutes of frame time are skipped. Otherwise a `d > 0` advances the clock `d`
minutes, then the clock takes the sent time.

Addresses: `0x6436c0`, `0x6c606b`, `0x643ad0`
Confidence: verified
Proof: one
Test: none

`0x64371d` to `0x643753` (`0x642320`, `imul 0x5a0`, `idiv`); the branch `0x64375a` to `0x643760`
(`jne`, then `jg` falling to `neg esi`); the forward loop `0x6437a8` to `0x6437b0`; the copy `0x6437be`; the
`+0x34` update and loop `0x6437c4` to `0x6437d9`. A `d` above 1 logs `GameTimeSync: skipping forwards`;
the second path logs `GameTimeSync: delta=`. One live caller: the opcode 0x43 handler `0x6c6010` with
`force = 0`.

## Time scale

`0x643820(speed)` stores `speed` in `+0x2c` clamped to `[1/60, 60]`; a NaN is stored as it is. Anything
below `0x3c888889` becomes that, anything above 60.0 (`0x42700000`) becomes 60.0.

Addresses: `0x643820`, `0x80e45c`, `0x80e448`
Confidence: verified
Proof: one
Test: none

Compares `fcomp [ebp+8]` with the bounds (`0x64382c`, `0x643847`): `jne` on `test ah,0x41` keeps the argument
at or above the low bound, `jp` on `test ah,5` at or below the high one. Callers: the handlers `0x6c5de0`
(opcode 0x47) and `0x6c5e80` (0x42), `0x6c5e31` and `0x6c5efc`.

## Server messages

`ClientInitializeGameTime` (`0x6c5d70`) registers five handlers with `0x5ab650`: opcode 0x47 `0x6c5de0` sets
the scale; 0x42 `0x6c5e80` decodes a packed time with `0x642c30`, sets it with `force = 1`, runs the bridge
`0x6c5f50` and sets the scale; 0x43 `0x6c6010` syncs without force; 0x45 `0x6c6120` sets it with `force = 1` and
runs the bridge; 0x49 `0x6c6080` only prints. The bridge stores the minute of day, `0x643a70()` and the day
number in the light state, calls `0x66ff60(1, 0)`, then four callbacks at `0xce85a8` with the hour in `ecx` and the minute in `edx`.

Addresses: `0x6c5d70`, `0x6c5de0`, `0x6c5e80`, `0x6c6010`, `0x6c6080`, `0x6c6120`, `0x6c5f50`, `0xce85a8`
Confidence: verified
Proof: one
Test: none

The stores `0x6c5f70`, `0x6c5f77`, `0x6c5f91`, `0x6c5f94`. Nothing in this file reads the callbacks' table but the bridge
(`0x6c5fa0`). Other users of the clock: `IsDaytime` (`0x60a0a4`, a range test of the clock), the sound frame
(`0x457922`) and `StampLightState`, which stores the day phase each frame beside the minute of day and day number.

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
`0x6439e0` removes one. None has a caller or a stored address.

Addresses: `0x643450`, `0x643640`, `0x643810`, `0x643890`, `0x6439e0`
Confidence: inferred
Proof: none
Test: none

Census: the two scans of `docs/METHOD.md` over the image for each address: no `e8`/`e9` target and no dword; the
control is `0x643ad0` with six calls. The meanings are read from the first instructions only.
