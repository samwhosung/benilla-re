# gametime: overview

The game clock: the server-synced time of day the world is lit and sounded by. One object, at
`0xce8538` (0x70 bytes), is built by a static initializer and driven by the server's time messages
and a per-frame update. Its methods are `0x643300` to `0x643c30`. The object holds a time (minute at
`+0`, hour at `+4`, then weekday, day, month and year), two offsets, a fractional-minute accumulator,
a time scale, the tick of the last minute and a table of callbacks per minute of the day. The
table's template code (`0x6433f0`, `0x643cb0` and after `0x643d10`) has no identified object file and
stays `unassigned`, as do the message handlers and the singleton's initializer in `0x6c5bc0` to
`0x6c6200`; their calls are stated in `messages`. The time helpers it calls, `0x6424c0` and the rest of
`0x642300` to `0x642c30`, belong to `base`.

The docs split by topic. `overview` is the clock: its state, the day phase, the per-frame and per-minute steps, the set, the sync and the time scale. `messages` is the server's game-time
messages and what they do to it.

Roots: `0x6435a0`, `0x6436c0`, `0x6434d0`, `0x643820`, `0x643a70`, `0x643300`, `0x643ad0`

## Clock state

`0x643300` builds the clock: `+0x20` (minute offset), `+0x24` (day offset), `+0x28` (a count of minutes
advanced), `+0x30` (the minute fraction) and `+0x34` (minutes still to skip) are 0; `+0x2c`, the scale
in game minutes per real second, is `0x3c888889`, 1/60; `+0x38` is the tick `0x42c010()`; `+0x3c`, the
phase base (the minute of day after the last step plus `+0x30`, stored by `0x643ad0`), is 0; the time at `+0` is −1 in every field (`0x642300`). A callback table with vtable
`0x80e44c` starts at `+0x48` and `+0x6c` is −1. Static initializer `0x6c5bc0` jumps to the
constructor with `ecx = 0xce8538`. The table's first vtable slot, `0x643bb0`, frees one entry: its callback
records, then the entry through `0x643c30`.

Addresses: `0xce8538`, `0x643300`, `0x642300`, `0x6c5bc0`, `0x42c010`, `0x80e44c`, `0x643bb0`, `0x643c30`, `0x515ee0`, `0x6424d3`, `0x6424e0`, `0x643b1b`, `0x60a0a3`, `0x457811`, `0x457921`, `0x482bcc`, `0x483dd7`
Confidence: verified
Proof: one
Test: none

Stores `0x64330f` to `0x64335a`; the minute is `[0xce8538]` and the hour `[0xce853c]`. `Script::GetGameTime` (`0x515ee0`) returns the
hour, then the minute: −1 and −1 until the first time message or the first minute step of the frame advance (a minute of frame time at the constructor's scale), which makes them 0 and 1
(`0x6424d3`, `0x6424e0`). Other users of the clock: `IsDaytime` (`0x60a0a3`, a range test of the clock), the sound world initialization (`0x457811`) and frame handler (`0x457921`), which
make the same test, and `StampLightState` (`0x482bcc`); `0x483dd7` is in `0x483dd0`, which nothing reaches. `0x6424c0` returns `hour·60 + minute`, 0 when either is
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
hour 23 before the day step `0x64355a` (`0x6423c0(-1, 0)`). Live callers: `0x6c5ee9` and `0x6c617b` with `force = 1`; the unreached `0x643450` and `0x643640` call it too (`0x6434c4`, `0x6436a0`). The advance (`0x643ad0`) sets hour and minute from `hour·60 + minute + 1` modulo 1440, with 0 for the sum when either is unset (`0x6424d3`). The test runs the
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
`+0x34` update and loop `0x6437c4` to `0x6437d9`. One live caller: the opcode 0x43 handler `0x6c6010` with `force = 0`; the unreached `0x643640` also calls it (`0x6436b3`). The test runs the function whole with
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
either sign included) it stores that, above 60.0 (`0x42700000`) it stores 60.0, and a NaN is stored through `fld` (`0x64387c`): a quiet one with its bits, a signalling one with its quiet bit set. It returns the scale the clock
held, in `st0`.

Addresses: `0x643820`, `0x80e45c`, `0x80e448`, `0x64387c`, `0x64387f`
Confidence: verified
Proof: two
Test: `gametime::gametime_643820_set_scale`

Compares `fcomp [ebp+8]` with the bounds (`0x64382c`, `0x643847`): `jne` on `test ah,0x41` keeps the argument
at or above the low bound, `jp` on `test ah,5` at or below the high one, and an unordered compare takes both. `fld [ecx+0x2c]` (`0x643823`) stays
on the x87 stack through the three exits (`0x64385c`, `0x643879`, `0x643883`). Callers: the handlers `0x6c5de0`
(opcode 0x47) and `0x6c5e80` (0x42), `0x6c5e31` and `0x6c5efc`. The test reads the old scale through
`call_f64` and the stored one for bounds, a step either side, zero, the infinities, denormals, quiet NaNs and random bits. A signalling NaN is left out: `fld` quiets
it on the processor and not in the emulator, so its quieting rests on reading. A second reading from `0x6c5d70` agrees.

## Scale change effects

`0x643820` writes only `+0x2c`, so a new scale leaves the day phase base `+0x38`, `+0x3c` as the last minute step stamped them: `0x643a70` applies the new scale to the real time since
that step, and the phase moves by `(tick − [+0x38])·0.001·(stored − held)` minutes at once. The scale also sets how long a game minute lasts in `0x6435a0`, `1/scale` real seconds, which paces
the minute `GetGameTime` returns. A NaN scale makes the fraction `+0x30` NaN on the next frame, for good: the per-frame advance stops. `0x643a70` returns NaN for a NaN scale; with a valid
scale it is finite until a set or sync steps a minute and stamps `+0x3c` from the fraction (`0x643b18`), after which it is NaN.

Addresses: `0x643820`, `0x643a70`, `0x6435a0`, `0x515ee0`, `0x64331f`, `0x6435be`, `0x6435f6`, `0x64361b`, `0x6435d1`, `0x643607`, `0x643a97`, `0x643b15`, `0x643b18`, `0x643b1b`
Confidence: verified
Proof: one
Test: `gametime::gametime_643a70_day_phase`, `gametime::gametime_6435a0_minute_accum`, `gametime::gametime_6435a0_nan_accumulator`

`0x643820` stores through `[ecx+0x2c]` alone. `0x643a70` adds `[+0x3c]` to the elapsed time times 0.001 and `[+0x2c]` (`0x643a97`); the stamps are written by the constructor and by `0x643ad0`
(`0x643b15`, `0x643b1b`). The stores to `+0x30` are the constructor's (`0x64331f`) and `0x6435a0`'s (`0x6435be`, `0x6435f6`, `0x64361b`). A NaN sum is unordered, sets C0 and leaves
`0x6435a0` at both of its `test ah,1` guards (`0x6435d1`, `0x643607`) with no minute, and `0x643a70` leaves its reduction loop for the NaN. The NaN test runs `0x6435a0` whole with
a NaN scale and then valid ones, `0x643a70` whole with each scale, and `0x643ad0` whole on the NaN fraction.

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

## Time range test

`IsDaytime` (`0x60a050`) asks `WowTime::InRange` (`0x642710`) whether the clock lies from 5:30 up to, not
including, 21:00: 5:30 and 20:59 are day, 21:00 and 5:29 are night. Only the hour and the minute count,
since the bounds leave weekday, day, month and year unset. `InRange(this, from, to)` gives `this >= from`
and `this < to` when `from <= to`, else `this >= from` or `this < to`. An ordering walks year, month, day,
weekday, hour, minute and skips a field either time leaves negative; for equality an unset field matches
anything. A clock with no hour and minute set, as built, is not daytime: `>= 5:30` holds through the
wildcard equality, and `< 21:00` fails.

Addresses: `0x60a050`, `0x60a067`, `0x60a08b`, `0x60a0a8`, `0x642ae0`, `0x642c30`, `0x642710`, `0x642880`, `0x6429d0`, `0x642770`, `0x6428c0`, `0x642a10`, `0x6425f0`, `0x642620`, `0x642650`, `0x642680`, `0x6426b0`, `0x6426e0`, `0xce8538`
Confidence: verified
Proof: one
Test: `gametime::time_order_difftest`, `gametime::time_in_range_difftest`, `gametime::time_encode_difftest`, `gametime::is_daytime_difftest`

`0x60a067` and `0x60a08b` call `0x642ae0` (minute in `edx`, hour the first stack argument, weekday to year
-1, the two-bit field 0); its stores mask each field, so -1 packs as all ones, and `0x642c30` returns those as
-1. `0x60a0a8` calls `0x642710` on `0xce8538`. `0x642710` calls `0x642880(from, to)`; if true, `0x6429d0(this,
from)` and `0x642770(this, to)` must both pass, else either. `0x642880` is `0x642a10` or `0x642770`;
`0x6429d0` is `0x642a10` or `0x6428c0`. `0x642770` and `0x6428c0` read the fields at `+0x14` down to `+0`,
skip one that either record has negative, stop at the first unequal pair (`0x6425f0` to `0x6426e0` return 1,
0 or -1, signed) and answer `setl` or `setg`, 0 for the same record. `0x642a10` returns 0 at the first
unequal pair both set, else 1. The tests run each function whole.

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
