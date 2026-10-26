# crt: calendar time

How the C runtime turns a 32-bit `time_t` into calendar fields and back, and formats the fields.
`gmtime` breaks a time down in UTC into the calling thread's `tm` buffer; `localtime` takes off
the zone that `___tzset` reads once per process; `mktime` builds a time from fields and adds the
zone back; `strftime` formats fields with the C locale's names and Windows-style date patterns.
The client's callers are Lua's `date` and `time` (`ui/luabase`), the engine's time stamps
(`OsGetDateTimeStamp`, `OsFormatLocalTime`), `CopyLocalTime`, `GameTimeSetFromHost` and
`WowTime`'s day arithmetic.

`tm` is nine `int`s: seconds, minutes, hours, day of the month from 1, month from 0, years since
1900, weekday from Sunday 0, day of the year from 0, and the daylight flag. Nothing calls
`_setlocale` (`net/who`), so `__lc_time_curr` keeps its file value, the C locale's time record.

Roots: `0x7421da`, `0x73f125`, `0x74066b`, `0x4134e3`

## Breaking a time down in UTC

`gmtime(&t)` answers null for a negative time. Otherwise it fills the calling thread's 36-byte
buffer, allocated on first use: the year from blocks of 1461 days from 1970, whose third year is
the one of 366 days; `tm_yday` from 0; the month as the last whose entry in the days-before-month
table, `_lpdays` in that year and `_days` in the others, is below `tm_yday`; `tm_mday` as
`tm_yday` less that entry; `tm_wday` from day 0, a Thursday; then the hours, minutes and seconds;
`tm_isdst` 0. The tables hold the days before each month less one, so January's entry is -1.

Addresses: `0x7421da`, `0x7421ee`, `0x7421ff`, `0x74221c`, `0x742234`, `0x742250`, `0x742265`, `0x742270`, `0x742282`, `0x7422ac`, `0x7422d8`, `0x832800`, `0x832834`, `0x40acbb`
Confidence: verified
Proof: one
Test: `crt::crt_gmtime_difftest`

`jge` at `0x7421ee` passes `t ≥ 0`, else `eax` is 0. The buffer is `[ptd+0x40]` of `__getptd`
(`0x40acbb`), `_malloc(0x24)` when it is null (`0x7421ff`). `idiv 0x7861f80` (`0x74221c`) counts
blocks of 126,230,400 seconds, and `lea edx,[eax*4+0x46]` makes the year. `0x742234` to
`0x74224c` take off 31,536,000, 31,536,000 and then 31,622,400 seconds while the rest allows,
each adding a year; a rest short of the third sets the leap flag (`0x742250`). `idiv 0x15180`
gives `tm_yday` (`0x742265`), `0x742270` picks `_days` (`0x832834`) over `_lpdays` (`0x832800`)
without the flag, and `0x742282` walks from entry 1 while the entry is below `tm_yday`. `tm_wday`
is `(t / 86400 + 4) mod 7` (`0x7422ac`), the hours and minutes divide by 3600 and 60, and
`0x7422d8` clears `tm_isdst`.

## Local time

`localtime(&t)` answers null for a negative time and reads the zone first. For a time more than
three days into 1970 and three days short of 2^31, it answers `gmtime` of `t − _timezone`, and
when `_daylight` is set and `__isindst` holds for that, `gmtime` of it less `_dstbias` with
`tm_isdst` 1. Nearer either end it takes `gmtime(t)` and carries `−_timezone`, and `−_dstbias`
when `__isindst` holds, through the fields by hand, `tm_isdst` left 0: a day back across January 1
gives December 31 of the year before with `tm_yday` 364, and a day forward is added to `tm_mday`
and `tm_yday` with no month carried. `__isindst`'s rules are not read here; it answers 0 while
`_daylight` is 0.

Addresses: `0x73f125`, `0x73f12c`, `0x73f138`, `0x73f13f`, `0x73f146`, `0x73f14d`, `0x73f15f`, `0x73f170`, `0x73f183`, `0x73f192`, `0x73f19e`, `0x73f1b4`, `0x73f1ba`, `0x73f22c`, `0x73f24d`, `0x73f26c`, `0x41482c`, `0x414852`, `0x83274c`, `0x832750`, `0x832754`
Confidence: verified
Proof: one
Test: `crt::crt_localtime_difftest`

`0x73f12c` tests `t < 0`; `0x73f138` calls `___tzset`. `cmp eax,0x3f480; jle` (`0x73f13f`) and
`cmp eax,0x7ffc0b7f; jge` (`0x73f146`) send the ends to `0x73f19e`. In the middle, `0x73f14d`
subtracts `_timezone` (`0x83274c`); `0x73f15f` tests `_daylight` (`0x832750`) and `0x73f170`
calls `__isindst` (`0x41482c`); `0x73f183` subtracts `_dstbias` (`0x832754`) and `0x73f192`
stores 1 in `tm_isdst`. At the ends `__isindst` is called whatever `_daylight` holds; `0x73f1b4`
and `0x73f1ba` take the biases off `tm_sec`, and each field is the signed remainder, raised by
60, 60 or 24 when negative with the carry lowered by the same. The days are added at `0x73f22c`
or, when negative, at `0x73f24d`, `tm_wday` as `(tm_wday + days + 7) mod 7`; `0x73f26c` adds 31 to
a day of 0 or less, takes a year off and stores 364 and 11. `__isindst` takes lock 0xb and
calls `0x41484d`, which answers 0 at `0x414852` while `_daylight` is 0.

## The zone, read once

`___tzset` runs `__tzset_lk` on its first call only, so the zone is read once per process, by the
first `localtime`, `mktime` or `strftime` `%Z`. With the environment variable `TZ` set and not
empty, the zone is its text: up to three bytes of standard name, then hours west of UTC with an
optional `-` for east, optional `:minutes` and `:seconds`, then up to three bytes of daylight
name. `_daylight` is the first byte of that name, 0 without one, and `_dstbias` keeps its value.
An empty `TZ` leaves the file's zone. `_tzset`, which would read the zone again, has no caller.

Addresses: `0x414561`, `0x414568`, `0x414580`, `0x41458f`, `0x4145a5`, `0x4145d6`, `0x4146e3`, `0x414744`, `0x414758`, `0x414766`, `0x414770`, `0x41478d`, `0x4147b8`, `0x4147e0`, `0x4147ee`, `0x414803`, `0x883c60`, `0x8327d8`
Confidence: verified
Proof: one
Test: none

`0x414568` returns while `0x883c60` is set; under lock 0xb `___tzset` tests again, calls
`__tzset_lk` and raises it (`0x414580`). Census of `0x883c60`: the listing's absolute operands and
the image's dwords find these three, the control `_timezone` twelve. `0x4145d6` reads `TZ`
through `__getenv_lk`; `0x4146e3` takes an empty value as no zone change. `0x414744` copies three
bytes into `_tzname[0]` (`0x8327d8`) and ends it; `0x414758` takes a `-`; `atol` (`0x414766`)
times 3600 (`0x414770`), and after a `:` (`0x41478d`, `0x4147b8`) `atol` times 60 and `atol` are
added, the digits skipped each time; `0x4147e0` negates for the `-`. `movsx` of the next byte is
`_daylight` (`0x4147ee`), and `0x414803` copies three bytes into `_tzname[1]`. A census of
`0x41458f`, `_tzset`, finds no `call`, `jmp` or dword of it beside the control `___tzset`'s four
calls.

## The system's zone

Without `TZ`, `__tzset_lk` asks `GetTimeZoneInformation`. `_timezone` is `Bias` in seconds, plus
`StandardBias` when the standard date's month is set. With the daylight date's month and
`DaylightBias` both set, `_daylight` is 1 and `_dstbias` `(DaylightBias − StandardBias) · 60`;
otherwise both are 0. The two names go through `WideCharToMultiByte` into 63 bytes of `_tzname`'s
buffers, each emptied when the conversion fails or needs a default character. A failed call
keeps the file's zone: 28800 seconds west, daylight time with `_dstbias` -3600, `PST` and `PDT`.

Addresses: `0x4145f3`, `0x4145fc`, `0x41460d`, `0x414610`, `0x414629`, `0x414633`, `0x414645`, `0x414657`, `0x414689`, `0x41468d`, `0x4146c2`, `0x883ba8`, `0x883bb0`, `0x8824c0`
Confidence: verified
Proof: one
Test: none

`0x4145f3` calls the import with the 172-byte record at `0x883bb0`; `cmp eax,-1; je` (`0x4145fc`)
returns. `imul eax,eax,0x3c` (`0x41460d`) stores `_timezone` and sets `0x883ba8`; the word at
`+0x46`, the standard date's month, is tested at `0x414610` and `+0x54` added (`0x414629`). The
word at `+0x9a` and the dword at `+0xa8` are tested at `0x414633`; `0x414645` subtracts `+0x54`
and stores `_daylight` 1 and the product by 60, else `0x414657` stores 0 in both. `0x414689`
and `0x4146c2` call `WideCharToMultiByte` through `0x8824c0` with flags 0x220, the names at
`+0x4` and `+0x58`, length -1 and 63 bytes; `0x41468d` stores the NUL at byte 63, or at byte 0
when the answer is 0 or the default was used. The file holds 0x7080, 1 and -3600 at `0x83274c`,
`0x832750` and `0x832754`.

## Fields to a time

`mktime(tm)` is `__make_time_t(tm, 1)`: -1 for a `tm_year` outside 69 to 139, a `tm_mon`
outside 0 to 11 folded into the year, stored back, and the year tested again. With `days`
before the month, one more past February when 4 divides the year, it sums
`year · 365 + ((year − 1) >> 2) + days − 25567 + tm_mday`, times 24 plus `tm_hour`, times 60 plus
`tm_min`, times 60 plus `tm_sec`, -1 on any signed 32-bit overflow. It adds `_timezone` untested,
-1 when `localtime` of that is null. A positive `tm_isdst`, or a negative one with that answer in
daylight time, adds `_dstbias` for a second `localtime`. The last answer is copied over the
fields and the sum answered.

Addresses: `0x74066b`, `0x740687`, `0x740692`, `0x7406c8`, `0x7406d4`, `0x7406ec`, `0x740701`, `0x74070c`, `0x740716`, `0x740737`, `0x740744`, `0x740811`, `0x74081d`, `0x740825`, `0x74082a`, `0x740836`, `0x740842`, `0x740860`, `0x740864`, `0x740679`
Confidence: verified
Proof: one
Test: `crt::crt_mktime_difftest`

`0x740692` bounds the year by 0x45 and 0x8b; `0x7406c8` stores the folded month and `0x7406d4`
bounds again. `0x7406ec` tests `year & 3` and the month above 1; `imul ecx,ecx,0x16d`
(`0x740701`) and `lea eax,[ecx+eax-0x63df]` (`0x74070c`) make the days. Each sum is tested by
sign (`0x740716`): operands of one sign and a sum of the other answer -1; each product
(`0x740737`) is divided back (`0x740744`) and must give its factor. `0x740811` adds `_timezone`
and `0x74081d` calls `localtime`, null answering -1 (`0x740825`). `0x74082a` takes a positive
`tm_isdst` to `0x740836`, a zero past it, and a negative one there when the answer's flag is
positive; `0x740842` calls `localtime` again and its answer is not tested before
`rep movs` copies nine dwords (`0x740860`). `0x740679`, the same with the flag 0, has no caller.

The second answer is null when `_dstbias` takes a sum from 0 or more to below 0, and the copy then
reads address 0: in a zone whose `_timezone` is 0 and whose `_dstbias` is -3600, `time{year=1970,
month=1, day=1, hour=0, isdst=true}` faults.

## The strftime format loop

`strftime(out, max, format, tm)` calls `_Strftime` with no time record, which takes
`__lc_time_curr`, the C locale's record. It copies each byte of the format that is not `%` while
room is left. A `%` clears the `#` mark, takes a `#` after it as the mark, and expands the next
byte, whatever it is: a format that ends in `%` expands its NUL, which writes nothing, and reads on
past it. The loop ends when the room or the format runs out; with room left it writes a NUL and
answers the bytes before it, else it answers 0 with no NUL written.

Addresses: `0x4134e3`, `0x4134fe`, `0x413512`, `0x413520`, `0x413546`, `0x413556`, `0x413569`, `0x413592`, `0x41359f`, `0x4135bb`, `0x4135c3`, `0x4135c4`, `0x4135dd`, `0x4135e9`, `0x832398`, `0x8323a0`, `0xcf7284`, `0xcf728c`, `0xcf7290`
Confidence: verified
Proof: one
Test: `crt::crt_strftime_difftest`, `crt::crt_strftime_patterns_difftest`

`0x4134e3` pushes 0 and forwards its four arguments. `_Strftime` raises `0xcf7290` with
`InterlockedIncrement` (`0x413512`), and while `0xcf728c` is set takes lock 0x13 instead
(`0x413520`); a null record is `[0x832398]` (`0x413546`), `0x8323a0` in the file, written only by
`___init_time` (`0x411990`), whose address is held only in `__lc_category` (`0x831bd8`). `0x413556` ends at a NUL;
`0x413569` copies a lead byte with the next, and the C table marks none. `0x413592` clears
`0xcf7284`, `0x41359f` sets it after `#`, `0x4135bb` calls `__expandtime(byte, tm, &out, &left,
record)` and `0x4135c3` steps past the byte expanded. `0x4135c4` loops while room is left;
`0x4135dd` tests it, `0x4135e9` stores the NUL and the answer is `max − left`.

## Name and pattern conversions

The record holds pointers: seven short and seven long day names, twelve short and twelve long
month names, `AM`, `PM`, and the patterns `M/d/yy`, `dddd, MMMM dd, yyyy` and `H:mm:ss`. Each
conversion writes no more than the room left.

| conversion | writes |
|---|---|
| `a`, `A` | the short or long day name of `tm_wday` |
| `b`, `B` | the short or long month name of `tm_mon` |
| `p` | `AM` for a `tm_hour` of 11 or less, else `PM` |
| `x` | the pattern `M/d/yy`; with `#` the pattern `dddd, MMMM dd, yyyy` |
| `X` | the pattern `H:mm:ss` |
| `c` | the `x` pattern, a space when room is left, and the `X` pattern |
| `Z`, `z` | `_tzname[tm_isdst != 0]`, after `___tzset` |
| `%` | `%` |
| any other | nothing |

Addresses: `0x4135f8`, `0x4135fb`, `0x41369c`, `0x413684`, `0x4138c3`, `0x4138ab`, `0x4139ba`, `0x413741`, `0x413844`, `0x413888`, `0x41395b`, `0x413916`, `0x4136b4`, `0x4139e2`
Confidence: verified
Proof: one
Test: `crt::crt_strftime_difftest`, `crt::crt_strftime_patterns_difftest`

`movsx eax,[ebp+8]` (`0x4135fb`) is switched by compares and subtractions. The names are read at
`[record + wday·4]` (`0x4138c3`), `+0x1c` (`0x41369c`), `+0x38 + mon·4` (`0x4138ab`) and `+0x68`
(`0x413684`), the designators at `+0x98` and `+0x9c` by `cmp [tm+8],0xb; jg` (`0x4139ba`). `X`
clears `0xcf7284` and patterns `+0xa8` (`0x413741`); `c` (`0x413844`) and `x` (`0x41395b`) pattern
`+0xa4` after clearing the mark when it is set, else `+0xa0`, and `c` writes its space only when
the room is not 0 (`0x413888`). `0x413916` calls `___tzset` and reads `[0x8327d8 + (isdst != 0)·4]`.
`%` stores its byte without a room test (`0x4136b4`); the loop only expands with room left. Any
other byte reaches `0x4139e2`.

## Number conversions

Each number conversion copies the `#` mark into `__no_lead_zeros` and writes through
`__store_num` with a digit count. The arithmetic is signed. The week is 0 while `tm_yday` is below
the first day's index, `tm_wday` for `U` and for `W` `tm_wday − 1` with Sunday 6, else
`tm_yday / 7`, plus 1 when `tm_yday mod 7` is not below the index.

| conversion | number | digits |
|---|---|---|
| `d` | `tm_mday` | 2 |
| `H` | `tm_hour` | 2 |
| `I` | `tm_hour mod 12`, 12 for 0 | 2 |
| `j` | `tm_yday + 1` | 3 |
| `m` | `tm_mon + 1` | 2 |
| `M` | `tm_min` | 2 |
| `S` | `tm_sec` | 2 |
| `U`, `W` | the week | 2 |
| `w` | `tm_wday` | 1 |
| `y` | `tm_year mod 100` | 2 |
| `Y` | `(tm_year / 100 + 19) · 100 + tm_year mod 100` | 4 |

Addresses: `0x413827`, `0x413667`, `0x41363d`, `0x413808`, `0x4138da`, `0x4136c8`, `0x4137be`, `0x41377e`, `0x413760`, `0x41378e`, `0x41399d`, `0x413938`, `0x413701`, `0x413734`, `0xcf7288`
Confidence: verified
Proof: one
Test: `crt::crt_strftime_difftest`

The cases are `d` `0x413827`, `H` `0x413667`, `I` `0x41363d` (`idiv` by 12, a zero remainder
made 12), `j` `0x413808`, `m` `0x4138da`, `M` `0x4136c8`, `S` `0x4137be`, `U` `0x41377e`, `W`
`0x413760` (`tm_wday` 0 gives 6, else `tm_wday − 1`), `w` `0x41399d`, `y` `0x413938` (`idiv` by
100) and `Y` `0x413701`; each stores `[0xcf7284]` into `0xcf7288` and calls `__store_num` at
`0x413734` with the count it pushed. The week is decided at `0x41378e`: `cmp ecx,esi; jge`, then
`idiv` by 7 and `cmp edx,esi; jl` before the `inc`.

## Writing numbers and strings

`__store_str` copies a string's bytes while room is left. Without `__no_lead_zeros`, `__store_num`
writes exactly the count of the number's last digits, zero-padded, when the count is below the
room, and otherwise sets the room to 0, which ends the format. With it, `__store_number` writes
the digits low first while more than one byte of room is left, stopping after a quotient of 0 or
less, then reverses them by a loop that swaps before it tests: with room for no digit it swaps
the last byte written with the one after it, so the output ends in whatever the buffer held there.
A digit is `'0'` plus the signed remainder: a negative number gives bytes below `'0'`.

Addresses: `0x4139e7`, `0x4139ed`, `0x413a07`, `0x413a0e`, `0x413a15`, `0x413a39`, `0x413a51`, `0x413a78`, `0x413a7d`, `0x413a8b`, `0x413aa7`, `0x413aae`, `0x413ab3`, `0x413abf`
Confidence: verified
Proof: one
Test: `crt::crt_strftime_difftest`, `crt::crt_strftime_patterns_difftest`

`__store_str` (`0x4139e7`) returns for no room (`0x4139ed`) and loops until the NUL or `dec`
of the room reaches 0 (`0x413a07`). `__store_num` (`0x413a0e`) tests `0xcf7288` (`0x413a15`);
`cmp ecx,[edi]; jae` (`0x413a39`) stores 0 in the room (`0x413a78`), else `idiv` by 10
(`0x413a51`) writes from the last place down. `__store_number` (`0x413a7d`) tests
`cmp [esi],1; jbe` before each digit (`0x413a8b`) and loops while the quotient is above 0
(`0x413aa7`); `0x413aae` sets the end and swaps from both ends at `0x413ab3` before
`cmp eax,ecx; jb` (`0x413abf`).

## Locale date patterns

`__store_winword` reads a pattern as a Windows date or time picture while room is left. A run of
one letter is a conversion by its length:

| run | conversion |
|---|---|
| `M`, `MM`, `MMM`, `MMMM` | `m`, `m`, `b`, `B` |
| `d`, `dd`, `ddd`, `dddd` | `d`, `d`, `a`, `A` |
| `yy`, `yyyy` | `y`, `Y` |
| `h` or `hh`, `H` or `HH`, `m` or `mm`, `s` or `ss` | `I`, `H`, `M`, `S` |
| `t` of any length | that many bytes of the designator, its NUL and what follows included |
| `a` or `A` of any length | `p`; a rest of the pattern equal to `am/pm` or `a/p` in any case goes with it |

An odd run of quotes copies up to the next quote; an even run writes nothing. Any other byte is
written alone. The single letters set `__no_lead_zeros`, but the conversion's own copy of the `#`
mark, always clear here, overwrites it: `M/d/yy` pads as `MM/dd/yy`.

Addresses: `0x413ac6`, `0x413ae1`, `0x413ae9`, `0x413b03`, `0x413b13`, `0x413b4a`, `0x413b9c`, `0x413bca`, `0x413be0`, `0x413c00`, `0x413cd2`, `0x413d80`, `0x414240`
Confidence: verified
Proof: one
Test: `crt::crt_strftime_patterns_difftest`

`0x413ae1` returns for no room and `0x413ae9` clears `0xcf7288`; `0x413b03` counts the run and
`0x413b13` switches on the letter. Each case decides by `dec ecx` on the count, and the single
letters store 1 first (`0x413b9c`). `0x413bca` and `0x413be0` call `__strcmpi` (`0x414240`),
which folds `A` to `Z` while the locale word is 0, on the rest of the pattern against `am/pm`
and `a/p`, adding 5 or 3. `0x413c00` tests `cl & 1` for the quote. `0x413cd2` picks the designator
by `tm_hour > 11` and copies the count of bytes with no NUL test. `0x413d80` calls `__expandtime`
with the letter. The default `0x413b4a` writes one byte and steps one.
