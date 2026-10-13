# core: time

The client's clocks. `OsGetAsyncTimeMs` (`0x42c010`), called from over 300 sites, is the millisecond
clock every timestamp, frame time and timer uses. It reads `GetTickCount` at first. A calibration
thread can move it to the processor's time-stamp counter once its rate estimates agree, at the
earliest a quarter of a second in, and back again, keeping it continuous; on a processor whose CPUID
reports more than one core in the way `Multi-core gate` reads, it stays `GetTickCount` exactly.

The clock lives in three OS object files: the time calls from `0x4293d0` through `0x429790`
(calendar time, file times and the manager's creation), `TimeManager.cpp` from `0x42b530` through
`0x42ba90` (the counter, its scale and offset, the calibration thread), and the clock file from
`0x42bff0` (the millisecond clock and the processor frequency), whose end is not settled. The state is the
counter mode `0x884c80`, the scale `0x8332c8`, the offset `0x884c88`, the counter rate
`0x8332c0`, and one time manager object at `0x884c78`.

Roots: `0x42c010`, `0x42c020`, `0x42c060`, `0x4293d0`, `0x429450`, `0x4294c0`, `0x429580`, `0x429780`, `0x429790`, and the static initializers `0x42b530`, `0x42b670`, `0x42b690`, `0x42bff0`, `0x42c150`, `0x42c170`

## The millisecond clock

`0x42c010` returns `trunc(c · scale + offset)` as a 32-bit integer, where `c` is the 64-bit
counter: `GetTickCount` (zero-extended) in mode 0, `rdtsc` in mode 1. The product and sum are
doubles at 53-bit precision, and the truncation keeps the low dword of the 64-bit result. At
startup the mode is 0, the scale 1.0 and the offset 0, so the clock is `GetTickCount`.

Addresses: `0x42c010`, `0x42b790`, `0x42b750`, `0x42b770`, `0x4293d0`, `0x8332c8`, `0x884c88`, `0x884c80`, `0x8332c0`
Confidence: verified
Proof: two
Test: none

`0x42c010` is `jmp 0x42b790`. `0x42b790` calls `0x42b750` and does `fild QWORD; fmul QWORD
[0x8332c8]; fadd QWORD [0x884c88]`, then `__ftol` (`0x42b7a1` to `0x42b7b0`). `0x42b750` tail-jumps
to `0x4293d0`, `rdtsc; ret`, when `[0x884c80]` is 1, else returns `GetTickCount` with `edx` 0.
`0x42b770` returns the counter rate: `[0x8332c0]` in mode 1, else 1000. The time manager's
constructor stores mode 0 on both outcomes of its configuration read (`0x42b822` to `0x42b837`),
scale `0x3ff00000:00000000` and offset 0 (`0x42b858` to `0x42b876`).

## Counter switch

`0x42b8b0` changes the counter mode keeping the clock continuous: with `c0` the old counter and `c1`
the new one read at once, the new scale is `(1.0 / rate) · 1000`, each step rounded at 53 bits, and
the new offset `(c0 · old scale + old offset) − c1 · new scale`, with `c0` and `c1` rounded to
doubles first. It does nothing when the processor flag at `+0x20` of the time manager is set: that
flag is the multi-core bit (`Multi-core gate`).

Addresses: `0x42b8b0`, `0x42b8be`, `0x42b90b`, `0x42b933`, `0x42b94f`, `0x42b955`, `0x8015b8`, `0x42b913`
Confidence: verified
Proof: two
Test: none

`0x42b8be` returns for `+0x20` nonzero. The rate is `fild QWORD` of `0x42b770()` and `fdivr` of the
double 1.0 at `0x8015b8` gives `1/rate` (`0x42b90b`); `fmul 1000.0` gives the new scale
(`0x42b933`), stored at `0x42b955`. `c0` and `c1` go to doubles through memory (`0x42b913` to
`0x42b91c`); the offset is `c0·S + O − c1·S'` (`0x42b939` to `0x42b94f`), in registers until the
stores. The same arithmetic in seconds updates the manager's own `+0x00` and `+0x08`. With no mode
change, `c1 = c0`.

## Multi-core gate

The time manager's flag is bit 6 of the processor capability word: set only when CPUID's maximum
extended leaf is above 0x80000000, and then for a `GenuineIntel` processor whose maximum basic leaf
is at least 4 and whose leaf 4 `EAX >> 24` is nonzero, or an `AuthenticAMD` one whose leaf
0x80000008 `ECX` low byte is nonzero: in practice more than one core. On such a processor the clock
never leaves `GetTickCount`, with scale exactly 1.0 and offset 0.

Addresses: `0x42b7c0`, `0x42b7f6`, `0x42b801`, `0x4298e0`, `0x429b90`
Confidence: verified
Proof: two
Test: none

`0x42b7f6` calls `0x429d10`, which returns `0x429b90`'s word; `shr eax,6; and eax,1` stores the bit
at `+0x20` (`0x42b7fb` to `0x42b801`). `0x4298e0` gives up when the maximum extended leaf is not
above 0x80000000 (`0x42993f`). `0x429b90` ORs `0x40` at `0x429c4f` after `cmp [maximum basic
leaf],4` and `EAX >> 24` nonzero for the Intel vendor string, and at `0x429c89` for a nonzero core
count byte of leaf 0x80000008 for the AMD one. Both are the processor code's, owned by its system.

## Calibration thread

The time manager starts a thread, `OsTime`, which samples the counter and `GetTickCount` every 250
ms into a ring of 121 samples and estimates the counter's rate over 1, 4, 20 and 120 samples. It
picks the shortest span whose estimate strays from the stored rate beyond its tolerance, capped
where neighbouring estimates disagree. In mode 0, a pick of the 20- or 120-sample span, or none,
stores the 120-sample estimate and switches to mode 1; a 1- or 4-sample pick stores that estimate
and stays. In mode 1 a pick stores its estimate and rescales, and returns to mode 0 when the
short-span estimates disagree. The switch is not one-way.

Addresses: `0x42ba90`, `0x42b9a0`, `0x42ba20`, `0x42b9c0`, `0x42be3f`, `0x42be64`, `0x42bf67`, `0x8015c0`, `0x8015c8`, `0x8015d0`, `0x8015d8`, `0x8015e0`, `0x8015e8`, `0x8015f0`, `0x8015f8`, `0x801600`, `0x42bf19`, `0x42bf36`, `0x42bf42`
Confidence: verified
Proof: one
Test: none

The wait is on the stop event with timeout `+0x18`, 250 (`0x42b7e7`, `0x42bfda`). `0x42b9a0` stores
`rdtsc` and `GetTickCount`; the first sample is copied into all 120 others (`0x42bacb`). `0x42ba20`
gives `Δcounter · 1000 / Δms` for a span, skipping a zero `Δms`. Each estimate is compared with the
rate by `|a − b| / max(a, b)` against 0.000333, 0.002, 0.01 and 0.04, and with its neighbour against
0.03, 0.008 and 0.001666; the index that results picks the switch (`0x42be3f` to `0x42be64`) and the
rate, then `0x42b8b0` runs (`0x42bf67`). In mode 1 the two differences are compared with 0.06 and
0.016 (`0x42bf19` to `0x42bf36`), and a negative pick skips the store and the switch (`0x42bf42`). The first round has two samples, and the prefill makes all four spans equal.

## Time manager lifetime

`WinMain`'s initialization creates the time manager and its shutdown destroys it. The
constructor reads the rate from the configuration value `Internal\CpuTicksPerSecond` (10⁹ when
absent), sets up the clock, and starts the calibration thread; at shutdown the thread frees the
manager and writes the rate back.

Addresses: `0x429780`, `0x42b6c0`, `0x42b7c0`, `0x429790`, `0x42b6f0`, `0x42b970`, `0x42b990`, `0x884c78`
Confidence: verified
Proof: one
Test: none

`0x429780` jumps to `0x42b6c0`, called at `0x402160`; it allocates 0x7e0 bytes, runs `0x42b7c0`
and calls `Sleep(0)`. `0x42b7c0` stores the object in `0x884c78`, creates the stop event at
`+0x14` and the thread `0x42b9c0` (`0x42b899`). `0x429790` jumps to `0x42b6f0`, called at
`0x403bce`: it signals the event through `0x42b990` and waits up to 20 ms for the thread to clear
`0x884c78`. The thread exchanges it out, closes both handles (`0x42b970`), frees it and writes
the rate (`0x42ba0b`).

## Counter rate

`0x42c020` returns `trunc(1000 / scale)`: the counter's ticks per second while the clock runs on
the time-stamp counter, and 1000 while it runs on `GetTickCount`. `debugprofilestop` divides a
time-stamp counter difference by a thousandth of it, so it answers milliseconds only after the
switch, and raw counter ticks on a multi-core processor.

Addresses: `0x42c020`, `0x7028a6`, `0x4293d0`
Confidence: verified
Proof: one
Test: none

`0x42c020` is `fld QWORD 1000.0; fdiv QWORD [0x8332c8]` and a tail jump to `__ftol`. The Lua
function at `0x702860` takes `rdtsc` (`0x702869`) less the start `0x702840` latched, divides
`0x42c020()` by 1000 with `__alldiv` (`0x7028b4`) and divides the difference by that
(`0x7028c4`).

## Processor frequency

`0x42c060` returns the processor's frequency in hertz: the maximum MHz from
`CallNtPowerInformation`, once, times 10⁶, or else the calibrated rate, waiting in 500 ms sleeps
until the calibration has stored one.

Addresses: `0x42c060`, `0x42c0b0`, `0x42c0e0`, `0x884c70`, `0x884060`
Confidence: verified
Proof: one
Test: none

`0x42c0e0` loads `powrprof.dll`, calls `CallNtPowerInformation(11, …)` and multiplies the second
dword by 1000000 (`0x42c11b` to `0x42c125`); `0x42c0b0` caches the answer. The fallback reads
`0x884060`, which the calibration raises to the largest rate it has stored.

## Calendar time

`GetUnixTime` (`0x4294c0`) returns `time(p)` for the pointer `p` in `ecx`. `0x429580` returns a
cached `time(NULL)`, refreshed when 500 ms of `GetTickCount` have passed since it was taken.
`0x429450` writes the local date and time as `%m%d%y_%H%M%S`.

Addresses: `0x4294c0`, `0x429580`, `0x429450`, `0x884048`
Confidence: verified
Proof: one
Test: none

`0x4294c0` is `push ecx; call _time`. `0x429580` reads the pair at `0x884048` atomically
(`0x65ae90`), returns the cached seconds when the tick difference is below `0x1f4` (`0x4295af`),
else calls `_time` and swaps the pair in with `0x65aec0`. `0x429450` formats through `_localtime`
and the C runtime's `strftime`-style `0x4134e3`.

## Static initializers

`TimeManager.cpp` and the clock file each run three static initializers: +∞ first, then 2π and
1/2π. The time manager's are `0x42b530`, `0x42b670` and `0x42b690`; the clock file's are
`0x42bff0`, `0x42c150` and `0x42c170`.

Addresses: `0x8273f4`, `0x8273f8`, `0x42b530`, `0x42b540`, `0x42b670`, `0x42b680`, `0x42b690`, `0x42b6a0`, `0x42bff0`, `0x42c000`, `0x42c150`, `0x42c160`, `0x42c170`, `0x42c180`
Confidence: verified
Proof: one
Test: none

They are the slots `0x8273f4` through `0x827408`, matched by the census of
`core/loop#static-initializers`. A file that includes the shared header opens its run with its +∞
(`systems/README`, Object files): `0x42b530` copies the +∞ at 0x8015a0 and `0x42b670` doubles the π
4 bytes on; `0x42bff0` copies 0x8015a8 and `0x42c150` doubles 0x8015ac. So the code from
`0x42b530` to `0x42b690`, and from `0x42bff0` to `0x42c170`, is one file each. Past `0x42b690`,
`TimeManager.cpp` is pushed at 0x42b6c7, in `0x42b6c0`, and the functions to `0x42ba90` are the
time manager's own; the clock file's run follows at once. Past `0x42c180` the code from 0x42c1a0
to 0x42c828 calls IMM32 and passes `OsIME.cpp` (`os`, `core/input`); nothing settles whether the
clock file ends at `0x42c180` or runs on through it.

## Unreached functions

The file-time and system-time conversions, the seconds clock, the date variants, the tick count
alone, and two accessors for the scale and the mode have no caller and no stored address.

Addresses: `0x4293e0`, `0x429410`, `0x429490`, `0x4294d0`, `0x429510`, `0x429540`, `0x429570`, `0x429600`, `0x429610`, `0x429620`, `0x429680`, `0x4296b0`, `0x4296e0`, `0x429710`, `0x429740`, `0x42c040`, `0x42c050`, `0x801608`
Confidence: verified
Proof: one
Test: none

The census of `core/loop#unreached-functions`; `0x429600`'s two dword matches are unaligned bytes
of a float table. `0x4296b0`, `0x4296e0` and `0x429680` are called
only by `0x429740`. `0x42c040` multiplies the scale by the double 0.001 at `0x801608`.
