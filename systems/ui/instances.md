# ui: instance messages

The seven server messages about instance saves and resets, and what each leaves for the interface:
the saved-instance list that `GetSavedInstanceInfo` reads, the lockout warnings and refusals the
client prints itself as system chat lines (type 10) through `0x49a870`, and the three words behind
`CanShowResetInstances` (`groupverbs`). `CGChat::InitializeGame` registers six of the handlers in
`ChatFrame.cpp` (`0x498680` to `0x4986d0`); `CGPartyInfo::InitializeGame` registers the save notice
of `PartyFrame.cpp`. No Lua handles the lines: each is a `GlobalStrings` template the handler fills.

Roots: `0x49e070`, `0x49e1c0`, `0x49e470`, `0x49e540`, `0x49e670`, `0x49e6c0`, `0x4e7e60`

## The handlers

| opcode | handler | body |
|---|---|---|
| 0x2cc | `0x49e070` | u32 count, then per entry i32 map, i32 seconds to reset, u32 instance id |
| 0x2fa | `0x49e1c0` | i32 type, i32 map, i32 seconds to reset |
| 0x31e | `0x49e470` | the map (`groupverbs`) |
| 0x31f | `0x49e540` | i32 reason, i32 map |
| 0x320 | `0x49e670` | i32 map |
| 0x32b | `0x49e6c0` | i32 ownership word |
| 0x2cb | `0x4e7e60` | i32 flag |

Every handler returns 1. A map's name is `Map.dbc`'s localized name (`+0x10` plus 4 × the locale),
or the id printed with `%d` when the id is out of range or has no record.

Addresses: `0x4982c0`, `0x498680`, `0x498690`, `0x4986a0`, `0x4986b0`, `0x4986c0`, `0x4986d0`, `0x4e7c70`, `0x4e7e48`, `0x835154`
Confidence: verified
Proof: one
Test: none

Each registration is `mov edx,handler; mov ecx,opcode` before `ClientServices::SetMessageHandler`:
0x2cc at `0x498680`, 0x2fa, 0x31e, 0x31f, 0x320 and 0x32b in the next five blocks, and 0x2cb at
`0x4e7e48`. The name rule is the same three tests in each handler: `jl` and `jg` against 0 and
`0xc0daac`, then the record from `0xc0daa8`; a miss formats the id with `0x835154`.

## The saved-instance list

`0x49e070` reads the count and, when it exceeds the list's count, reserves the difference
(`0x4a3370`) and raises the list's count to it; a smaller count leaves the list's count and storage
alone. It then reads each entry into the 12-byte `RaidInstanceInfo` records at `0xb4fdf4`: the map
at `+0`, the seconds at `+8`, the id at `+4`, in that wire order. It stores the new count in
`0xb6e5e8`, the count `GetSavedInstanceInfo` bounds by, and signals `UPDATE_INSTANCE_INFO` (539)
with no arguments, also for a count of 0. For each entry with a map record it formats
`RAID_INSTANCE_INFO_FMT` with the name, the id and the seconds as text (`0x49e9d0`), and discards
the result.

Addresses: `0x49e070`, `0x49e08e`, `0x49e0b1`, `0x49e0c1`, `0x49e0c9`, `0x49e0ec`, `0x49e0fe`, `0x49e110`, `0x49e147`, `0x49e179`, `0x49e19d`, `0x49e1a2`, `0x49e1a7`, `0x844870`, `0xb4fdec`, `0xb4fdf0`, `0xb4fdf4`, `0xb4fdf8`, `0xb6e5e8`, `0x4a1920`
Confidence: verified
Proof: one
Test: none

A zero count jumps from `0x49e08e` to the store and the signal (`0x49e19d` to `0x49e1a7`, event
0x21b). `0x49e0b1 jb` skips the reserve for a count below the list's, and `0x49e0c9` stores the count
otherwise. The three reads are `0x49e0ec`, `0x49e0fe` and `0x49e110`. The buffer `[ebp-0x908]` that
`0x49e179` fills is read by nothing before the return. The list is `{capacity, count, data,
granularity}` at `0xb4fdec`. `GetSavedInstanceInfo` (`0x4a1920`) pushes `+4` unsigned and `+8`
signed after the name.

## Growing the saved list

`0x4a3370(extra, round)` returns when `count + extra` fits the capacity. Otherwise, with `round`
set, it rounds the need up to a multiple of the granularity `+0xc`. A granularity of 0 takes the
largest power of two not above the need, which makes the need the next power of two, or, for a need
of 21 or more, stores 21 as the granularity and uses it. It stores the need as the capacity and
calls `SMemReAlloc` for need × 12 bytes, in 32 bits; when that returns null it allocates anew,
copies the smaller of the need and the count, and frees the old storage. The count is not changed.

Addresses: `0x4a3370`, `0xb4fdf8`, `0x4a3384`, `0x4a338f`, `0x4a3398`, `0x4a33a2`, `0x4a33bb`, `0x4a33cb`, `0x4a33ee`, `0x4a33f0`, `0x4a3405`, `0x4a344f`
Confidence: verified
Proof: one
Test: `ui::instances_reserve_saved_list`

`0x4a3384 jbe` is the fit test (need against `[this]`), `0x4a338f` the `round` test. `0x4a3398`
compares the need with 0x15; below it the loop at `0x4a33a2` clears the lowest set bit until one
remains; `0x4a33bb` stores 0x15. `0x4a33cb div` and the add round up. `0x4a33ee` stores the capacity
before `SMemReAlloc` (`0x4a33f0`, flags 0x10); `0x4a3405` allocates, the loop copies 12-byte records,
and `0x4a344f` frees. The test runs it whole with the three allocator calls stubbed, over needs 1 to
70, granularities 0, 1, 7 and 21, both `round` values and a null return, and compares the capacity,
the granularity and the sizes passed.

## Duration text

`0x49e9d0(buf, size, seconds)` writes `%dd %dh %dm %ds` with days `s/86400`, hours
`(s − 86400d)/3600`, minutes `(s − 3600(24d + h))/60` and seconds `s mod 60`, every division signed
and truncating toward zero, so a negative input gives negative fields.

Addresses: `0x49e9d0`, `0x49e9de`, `0x49ea01`, `0x49ea1e`, `0x49ea2f`, `0x49ea52`, `0x8449f8`
Confidence: verified
Proof: one
Test: `ui::instances_format_duration`

The divisions are `imul` by `0xc22e4507` and `sar 16` (`0x49e9de`), by `0x91a2b3c5` and `sar 11`
(`0x49ea01`), `idiv` by 60 (`0x49ea1e`, its remainder the seconds) and by `0x88888889` and `sar 5`
(`0x49ea2f`), each `imul` form adding the quotient's sign bit. `0x49ea52` calls `SStrPrintf` with the
buffer, the size and the four numbers; the function returns with `ret 4`. The test runs it whole with
`SStrPrintf` stubbed, over every multiple of 60 up to two days, of 3600 up to 31 days and of 86400
up to 366 days, a step either side, both signs, the extremes of the word and 20,000 others, and
compares the four numbers.

## Lockout warnings

`0x49e1c0` picks a template by type: 1 `RAID_INSTANCE_WARNING_HOURS` with `t/3600`, 2
`RAID_INSTANCE_WARNING_MIN` and 3 `RAID_INSTANCE_WARNING_MIN_SOON` with `t/60`, and 4
`RAID_INSTANCE_WELCOME` with days, hours and minutes split as `0x49e9d0` splits them. Each division
is signed and truncates toward zero, so a seconds word of 2^31 or more prints negative numbers. Types
1 to 3 read the template through `GetText` with the number as its ordinal, so the plural form follows
it; type 4 reads the plain global. The line is the template filled with the map's name and the
numbers, shown as a system line. Any other type shows nothing.

Addresses: `0x49e1c0`, `0x49e1e9`, `0x49e228`, `0x49e246`, `0x49e24f`, `0x49e45c`, `0x49e259`, `0x49e27b`, `0x49e2c8`, `0x49e337`, `0x49e3a6`, `0x49e408`, `0x49e2b4`, `0x49e449`, `0x8448dc`, `0x8448c0`, `0x8448a0`, `0x844888`
Confidence: verified
Proof: one
Test: `ui::instances_lockout_warning`

`0x49e246` bounds `type − 1` by 3 (`ja` to the return) and `0x49e24f` jumps through the four dwords at
`0x49e45c`. The divisions are `imul` by `0x91a2b3c5` with `sar 11` (3600), by `0x88888889` with
`sar 5` (60) and by `0xc22e4507` with `sar 16` (86400), each adding the sign bit. Arms 1 to 3 leave the
quotient in `edx` for `0x703bf0`'s ordinal (`0x49e27b`); arm 4 sets `or edx,-1` (`0x49e408`).
`SStrPrintf` gets the 0x800-byte line, the template, the name and the numbers; `0x49a870` gets type 10
and ten zero arguments. The test runs `0x49e1c0` whole with the reads, `GetText`, `SStrPrintf` and `0x49a870`
stubbed, over types −1 to 6, a named map, one without a record and two out of range, and 417 reset
words, and compares the key, the ordinal, the line and its type.

## Reset refusals

`0x49e540` shows `INSTANCE_RESET_FAILED` for reason 0, `INSTANCE_RESET_FAILED_OFFLINE` for 1 and
`INSTANCE_RESET_FAILED_ZONING` for 2, each filled with the map's name. Any other reason, negative
included, skips the format and still shows the line: the 0x800-byte stack buffer, never written, goes
to `0x49a870` as it is.

Addresses: `0x49e540`, `0x49e5ba`, `0x49e5bf`, `0x49e5c2`, `0x49e60f`, `0x49e5ea`, `0x49e5c5`, `0x49e63a`, `0x49e659`, `0x844910`, `0x844928`, `0x844948`
Confidence: verified
Proof: one
Test: none

`0x49e5ba sub eax,0; je` takes reason 0 to `0x49e60f`, `dec; je` reason 1 to `0x49e5ea`, and `dec; jne`
everything else but 2 to `0x49e63a`, past the format and straight to the call at `0x49e659`. The
templates are read with ordinal -1.

## Last instance and ownership

`0x49e670` reads a map and, when the map has a record, runs `RecordMapChange(time(0), map,
current)` with the manager's current map: the packet's map counts as the one left, so it is
recorded as the last instance when its type is 1 and it is not the current map, and the current map
is stored in `0xb4e378`. `0x49e6c0` stores
its word in `0xb4e37c` through `0x495d50`, whole; `CanShowResetInstances` tests it for nonzero.
Neither shows anything.

Addresses: `0x49e670`, `0x49e684`, `0x49e698`, `0x49e69a`, `0x49e6a2`, `0x49e6ac`, `0x49e6c0`, `0x49e6d2`, `0x495d10`, `0x495d50`, `0x495c90`, `0xb4e37c`
Confidence: verified
Proof: one
Test: none

`0x49e684` to `0x49e698` are the range and record tests, which skip to the return. `0x49e69a` pushes
`ClntObjMgrGetMapID()`, `0x49e6a2` calls `GetUnixTime(null)`, and `0x49e6ac` passes the time in `ecx`
and the map in `edx`. `0x495d10` returns when the two maps are equal, records the left map and the time
for a type-1 record, and stores the current map. `0x49e6d2` passes the word in `ecx`.

## The save notice

`0x4e7e60` shows `INSTANCE_SAVED` for flag 0 and `(Debug-Only Lock Notice) %s` filled with it for
flag 1. Any other flag shows the 0x800-byte stack buffer unwritten, as the refusals do.

Addresses: `0x4e7e60`, `0x4e7e7a`, `0x4e7e97`, `0x4e7e9e`, `0x4e7eb3`, `0x4e7eeb`, `0x84c388`, `0x84c36c`
Confidence: verified
Proof: one
Test: none

`0x4e7e7a jne` leaves flag 0, which copies the template with `SStrCopy` (`0x4e7e97`); `0x4e7e9e cmp
eax,1; jne` sends every other flag to the call at `0x4e7eeb` unformatted. Both arms read the template
with ordinal -1.
