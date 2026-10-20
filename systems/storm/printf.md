# storm: printf

Storm's string formatting and the options that steer it. `SStrPrintf` and `SStrVPrintf` format into
a buffer of a given size through `ISStrVPrintf`, which picks the C runtime's `_vsnprintf` or Storm's
own formatter by Storm option 10. The client sets that option at start, so from then on `SStrPrintf`
and `SStrVPrintf` go through `vsnoprintf`: it binds each conversion, `%N$` ones included, to an
argument by number and hands it, rewritten, to the C runtime's `_snprintf`. Storm's other formatting
calls the C runtime whatever the option: `0x667440` `__vsnprintf` (`0x667461`), as do `0x64d080`
(`0x64d0a3`) and `0x64db80` (`0x64dbaa`), and `SLogVWrite` `_vsprintf` (`0x65ac73`). The code is
Storm's, in two stretches: `SStrPrintf` (`0x64a7f0`) to `SStrVPrintf` (`0x64a8b0`) beside the option
calls `0x64ead0` and `0x64ec80`, and the formatter, `vsnoprintf` (`0x6697d0`) to `vsoprintf`
(`0x669d10`), with the variadic forms `0x669d30` (with a size) and `0x669d50` (2^20) after it,
which nothing reaches. The source file of neither stretch is named in the image.

Roots: `0x64a7f0`, `0x64a8b0`, `0x64ec80`, `0x64ead0`

## Storm options

`0x64ec80(id, data, size)` sets one of the options 1 to 13 and returns 1. It returns 0 for a null
`data`, an id outside the range or 8, a size other than the option's, or for option 2 a value that is
neither a power of two nor 2^31 + 2^k for k below 31: the test runs in 32 bits. It sets Storm's last error to 0x57 first and to 0 on success. `0x64ead0(id, data,
size)` reads an option into a buffer of at least its size and sets `*size`; option 8 it computes
through `0x650be0`. Every cell starts at 0, and `ClientMain` is the setter's only caller: it sets
options 10 and 11 to 1.

| option | size | cell | stored |
|---|---|---|---|
| 1 | 4 | `0xc53ef8` | the value |
| 2 | 4 | `0xc53efc` | the value, a power of two or 2^31 + 2^k |
| 3 | 4 | `0xc53ef4` | the value |
| 4 | 4 | `0xc53f00` | the value |
| 5 | 4 | `0xc53f04` | the value |
| 6 | 8 | `0xc53ef0`, `0xc53ee8` | the two dwords |
| 7 | 4 | `0xc53f08` | the value |
| 9 | 4 | `0xc53f0c` | `value != 0` |
| 10 | 4 | `0xc53f10` | `value != 0`: the ordered formatter |
| 11 | 4 | `0xc53f14` | `value != 0`: the wide-character file and registry calls |
| 12 | 4 | `0xc53f18` | `value != 0` |
| 13 | 4 | `0xc53f1c` | `value != 0` |

Addresses: `0x64ec80`, `0x64ec9a`, `0x64ece1`, `0x64edfe`, `0x64ee24`, `0x64ee84`, `0x64ead0`, `0x64eba3`, `0x64ec35`, `0x650be0`, `0x402141`, `0x402151`, `0x64be0c`, `0x64e850`, `0xc53ee8`, `0xc53ef0`, `0xc53ef4`, `0xc53ef8`, `0xc53efc`, `0xc53f00`, `0xc53f04`, `0xc53f08`, `0xc53f0c`, `0xc53f10`, `0xc53f14`, `0xc53f18`, `0xc53f1c`, `0x64a833`, `0x646d99`, `0x6674ff`
Confidence: verified
Proof: one
Test: `storm::set_option_difftest`

`0x64ec9a` bounds `id − 1` by 12 and jumps through `0x64ee8c`, whose entry for 8 is the failure
`0x64ee84`; each case needs the size exactly (`cmp [ebp+0x10],4; jne`). Option 2 needs `v` nonzero and `(v − 1) ^ v
== 2v − 1` in 32 bits (`0x64ece1` to `0x64ecec`). With `2^k` the lowest set bit the left side is
`2^(k+1) − 1`, so `2v` must be `2^(k+1)` modulo 2^32: `v` is `2^k` or `2^k + 2^31`, and
`0x80000001` passes; options 9 to 13 store `setne`
(`0x64edfe` for 10, `0x64ee24` for 11). The getter's table `0x64ec4c` needs `*size` at least the
option's, stores the cell and sets `*size` (`0x64ec35`). The census of `e8`/`e9` to `0x64ec80`
and of the address stored anywhere finds `0x402141` (id 10) and `0x402151` (id 11), each passing a
4-byte 1, and no stored copy; the control is the same scan finding the getter's call at `0x64be0c`.
The cells lie in the zero-filled part of `.data`. Option 10's one reader is `ISStrVPrintf`
(`0x64a833`). Option 11's twelve operands are the setter, the getter and ten readers, `SFileStat`
(`0x646d99`) and `SDirOpen` (`0x6674ff`) among them (`mount#loose-file-index`). The test runs
`0x64ec80` whole with `SErrSetLastError` a stub that records the codes.

## Formatting into a buffer

`SStrPrintf(dest, size, fmt, ...)` and `SStrVPrintf(dest, size, fmt, va)` set error 0x57 and return
0 for a null `dest` or `fmt`, else return `ISStrVPrintf(dest, size, fmt, va)`. That returns 0 for a
size of 0 and writes nothing. For a size of `0x7fffffff` it returns the result of `vsoprintf`, which
is `vsnoprintf` with a size of 1,048,576 (2^20), or of the C runtime's `vsprintf`. Any other size runs
`vsnoprintf`, or with option 10 clear the C runtime's `_vsnprintf`; a result not below the size,
compared unsigned, ends the text with a NUL at `size − 1` and returns `size − 1`.

Addresses: `0x64a7f0`, `0x64a8b0`, `0x64a820`, `0x64a82d`, `0x64a83a`, `0x64a848`, `0x64a859`, `0x64a861`, `0x64a865`, `0x64a872`, `0x64a87f`, `0x64a896`, `0x669d10`, `0x73f6ad`, `0x7412c1`
Confidence: verified
Proof: one
Test: `storm::isstrvprintf_difftest`

`ISStrVPrintf` is `__fastcall(ecx = dest, edx = size, fmt, va)` with `ret 8`. `0x64a82d` compares
the size with 0x7fffffff, and `0x64a83a` and `0x64a872` test the option word. The calls are
`vsnoprintf` (`0x64a848`), `__vsnprintf` (`0x64a859`), `vsoprintf` (`0x64a87f`) and `_vsprintf`
(`0x64a896`). `0x64a861 cmp eax,esi; jb` keeps a smaller result; else `0x64a865` stores the NUL and
returns `size − 1`, which a result of −1 also meets. `vsoprintf` pushes 2^20 as the size. The
test runs `0x64a820` whole with option 10 set, `vsnoprintf` inside it and `_snprintf` a stub.

## Ordered conversions

`vsnoprintf(dest, size, fmt, va)` makes three passes. The first records each `%` spec and, for one
taking an argument, stores its kind at its index: `N − 1` after `%N$`, else one past the previous
spec's, so a plain spec after `%2$s` reads argument 3. The last spec naming an index decides its
kind. The second reads the arguments from index 0 to the highest named, by kind: 4 bytes for an
integer or a pointer, 8 for a long long or a double, 4 for an index no spec names. The third copies
the text and hands each spec to `_snprintf` with its argument, so a repeated index prints one value
twice.

Addresses: `0x6697d0`, `0x669867`, `0x669873`, `0x669888`, `0x669892`, `0x6698c1`, `0x669a80`, `0x6698c8`, `0x6698da`, `0x6698f6`, `0x669984`, `0x66999e`, `0x6699ae`, `0x669a90`, `0x6699cf`, `0x669a59`
Confidence: verified
Proof: two
Test: `storm::vsnoprintf_difftest`

The frame holds 256 kinds at `ebp−0xd14`, 256 eight-byte values at `ebp−0x914` and 256 specs
(start, length, index) at `ebp−0x1914`, all zeroed. `0x669867` parses from the byte after the `%`
with the index cell `ebp−8`, and `0x669873` stores the spec with the index the parse leaves. For a
positive return `0x669888` stores the kind at that index and `0x669892` stores the index plus one,
raising the count `ebp−4` to it. The second pass switches the kind through `0x669a80`: 0 and 1
copy a dword (`0x6698c8`), 2 two dwords (`0x6698da`), 3 a double through `fld` and `fstp qword`
(`0x6698f6`), which quiets a signalling NaN; an unnamed index keeps the zeroed kind. The third pass
copies a spec into a 256-byte buffer, cuts its `N$` (`0x669984`) and for kind 2 makes `ll` into
`I64` (`0x66999e`). The table `0x669a90` pushes one dword for kinds 0 and 1 and two for 2 and 3,
with the room `end − position + 1` (`0x6699cf`); the format cursor then skips the spec's length
less one (`0x669a59`). An independent cold read derived the same binding. The test runs `0x6697d0`
whole with `_snprintf` a stub and passes no signalling NaN, which the emulator's `fld` keeps.

## Parsing a conversion

`ParseFormatSpecifier` walks from the byte after the `%`. Digits build a number, and `$` sets the
index to it less one, or to 0 for none; any other clears the number and is classed. A digit is
what `_isdigit` says of the byte sign-extended (`#bytes-that-count-as-digits`). `%` and NUL end a spec taking no argument, `C D U X c d i u x` an integer, 8 bytes
after `ll` or `I64`, `E F G e f g` a double, `P S p s` a pointer. Every other byte goes on, so `%o`
runs on to the next ending byte. `_snprintf` gets the spec with its first `$` and the digits before
it cut, and an 8-byte integer's `ll` made `I64`.

Addresses: `0x669ab0`, `0x669ad7`, `0x669ae9`, `0x669af2`, `0x669b0f`, `0x669b1b`, `0x669bb8`, `0x669bd4`, `0x669b29`, `0x669b3a`, `0x669b56`, `0x669b69`, `0x669b81`, `0x669b9d`, `0x669c50`, `0x669cc0`, `0x8670e0`
Confidence: verified
Proof: one
Test: `storm::vsnoprintf_difftest`

It is `__fastcall(ecx = &cursor, edx = &kind, &index)` with `ret 4`, returning 1 when the spec takes
an argument. `0x669ad7` is `_isdigit` and `0x669ae9` builds `n = 10n + c − 0x30`; `0x669af2` is
`cmp bl,0x24`. `0x669b0f cmp edi,0x78; ja` sends higher bytes back to the loop, and `0x669b1b`
reads the class byte `0x669bd4` into the table `0x669bb8`: 0 at `0x669b9d` (kind 4, returns 0), 1
at `0x669b56` (the kind as set, 0 or 2), 2 at `0x669b81` (kind 3), 3 at `0x669b3a` (`64` after
`I`), 4 at `0x669b69` (kind 1), 5 at `0x669b29` (a second `l`), 6 back to the loop.
`RemoveOrderingFromFormatSpecifier` finds the `$` with `_strchr` and moves the rest of the spec over
it and the digits before it; `FixUpLongLongFormatSpecifier` finds `ll` with `_strstr`, moves the
rest one byte right and writes `I64` from `0x8670e0`. All three run whole inside the test.

## Bytes that count as digits

The parser and the walk back over the digits before a `$` hand `_isdigit` each byte sign-extended,
and `_isdigit` indexes its table with no bound. So besides `0` to `9` the bytes 0xf7, 0xfb and 0xfd
count as digits, worth −57, −53 and −51 in `n = 10n + c − 0x30`, and are cut with the digits
before a `$`; `%9\xf7$d` reads argument 33. Every other byte from 0x80 is no digit while a pointer
the small-block heap rewrites keeps its bits 2 and 18 clear, as it does at load; when one is set,
0xed or 0xee counts too.

Addresses: `0x669ad2`, `0x669ad7`, `0x669ae9`, `0x669c69`, `0x669c75`, `0x669c83`, `0x73fc0f`, `0x83171c`, `0x831710`, `0x831714`, `0x831720`, `0x40f057`
Confidence: verified
Proof: one
Test: `storm::vsnoprintf_difftest`

`0x669ad2` and `0x669c69`, `0x669c83` load the byte with `movsx`. With `__mb_cur_max` (`0x831710`)
at 1, `_isdigit` (`0x73fc0f`) tests bit 2 of the word at twice the value in the table `_pctype`
(`0x83171c`) points to. The 128 words below the table hold, at 0xf7, 0xfb and 0xfd,
`__decimal_point` (`0x831714`) and the low bytes of `_pctype` and `_pwctype` (`0x831720`), each
with bit 2 set and written only by the unreached locale initializers; at 0xed and 0xee, the low and
third bytes of a heap region pointer, written at `0x40f057`; and nothing with bit 2 set elsewhere.
The census of absolute operands in those words finds those cells alone. The test runs the three
bytes before a `$` and in a width.

## Truncation and overruns

The third pass stops once the position reaches `dest + size − 1` and writes a NUL there. `_snprintf`
gets the room left, the last byte included. A conversion that fills it exactly returns its length,
so the position passes the last byte and the NUL lands one byte past the buffer. One that does not
fit returns −1, so the position steps back a byte and the next text overwrites the cut conversion
from the byte before it; a cut conversion at `dest` leaves the position at `dest − 1`, so the next
byte or the NUL lands one byte before the buffer. A format ending in a lone `%` reads on past its NUL in both passes.

Addresses: `0x6697e6`, `0x669921`, `0x6699c9`, `0x669a51`, `0x669a69`, `0x669a76`, `0x669850`, `0x669ad6`, `0x669b9d`, `0x669baa`
Confidence: verified
Proof: two
Test: `storm::vsnoprintf_difftest`

`0x6697e6` sets the end `dest + size − 1`; `0x669921` and `0x669a69` compare the position with it
unsigned (`jae`, `jb`). The room is `end − position + 1` (`0x6699c9`), and `0x669a51` adds the
return to the position unchecked; `0x669a76` stores the NUL. Under the C runtime's `_snprintf`
contract (the length for an exact fit, −1 when cut) an exact fit leaves the position at `end + 1`;
`ISStrVPrintf` then also rewrites `size − 1`. At `dest − 1` the position is still below the end, so
the loop goes on: with a size of 4, `%s` and `longer`, the client writes 0 at `dest[−1]`. The first pass reads the format at `0x669850`; the parse steps past a NUL before
classing it (`0x669ad6`) and stores the cursor beyond it (`0x669b9d`, `0x669baa`). An independent
cold read derived the same. The test covers every room from 1 to 80 over one format, a conversion
at `dest` that does not fit, and the lone `%`.

## Unbounded spec tables

Nothing bounds the first pass's tables or the third pass's spec buffer: a spec longer than 255
bytes, a 257th spec, or an index outside 0 to 255 writes outside its table in the frame. An index
goes below 0 when the digits before a `$` wrap `n` past 2^31 or include bytes that count as
negative digits (`#bytes-that-count-as-digits`).

Addresses: `0x66996a`, `0x669873`, `0x669888`, `0x669892`, `0x669af2`
Confidence: verified
Proof: one
Test: none

`0x66996a` copies the spec's whole length into the 256 bytes at `ebp−0x114`, and the first pass
stores specs and kinds with no bound (`0x669873`, `0x669888`); `0x669892` raises the count to the
index plus one, signed. `0x669af2`'s `$` stores `n − 1` for any nonzero `n`. The test keeps every
index from 0 to 255.

## Unreached functions

Four functions of the two stretches have no caller and no stored address: `0x669d30(dest, size,
fmt, ...)` and `0x669d50(dest, fmt, ...)`, the variadic forms of `vsnoprintf` with a size and with
2^20, and `0x64eab0` and `0x64eac0`, which add 1 to option 6's first and second dword.

Addresses: `0x669d30`, `0x669d43`, `0x669d50`, `0x669d64`, `0x64eab0`, `0x64eac0`
Confidence: verified
Proof: one
Test: none

`0x669d30` passes its own size and the address of its variadic arguments to `vsnoprintf` at
`0x669d43`; `0x669d50` pushes 2^20 as the size (`0x669d64`). The census of `e8`/`e9` to each and of
its address stored anywhere finds none, with `vsoprintf`'s one call from `ISStrVPrintf` as the
control.
