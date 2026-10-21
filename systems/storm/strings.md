# storm: string conversions

Storm's string-to-number functions, which the client calls for the numbers in XML attributes, many
settings and console arguments; the C runtime's `_sscanf` (`0x73fb5d`, 32 direct calls, formats `%d`
and `%f` among them) reads others, as the `lodDist` setting does (`0x688ac0`). `SStrToFloat` reads a
decimal float, `SStrToInt` a signed decimal integer and `SStrToUnsigned` unsigned digits; the float's
fraction digits come from a table of powers of ten that `0x64a670` fills on first use
(`core/threads#string-conversion-table`), and its other powers from the C runtime's `pow`. All three
answer 0 for a null string and set error 0x57. `SStrTokenize` splits the text of console lines,
CVar commands and many files into tokens. `SStrLen` measures a string, `SStrPack` appends to one
within a buffer size and `SStrStr` finds one in another.

Roots: `0x64aaa0`, `0x64ac60`, `0x64ade0`, `0x64ae50`, `0x64a6f0`, `0x64a760`, `0x64b4f0`, `0x64b4a0`

## Float grammar

`SStrToFloat(s)` (`0x64aaa0`) reads one optional `-`, with no space skipped and no `+`; then
decimal digits; then a `.` and decimal digits; then an exponent; each part optional, and the read
stops at the first byte that fits none. A `.` with no digit after it is still passed, so `1.e2` reads
as `1e2`. The exponent is `e` or `E`, one optional `+`, and `SStrToInt` of the rest, so `1e+-2` reads
as `1e-2`. ` 12` and `+12` read 0, `.5` as `0.5`, `-` as −0.0, and a null `s` sets error 0x57 and
answers 0.0. A digit is a byte, sign-extended, minus `'0'` below 10 unsigned.

Addresses: `0x64aaa0`, `0x64aab1`, `0x64aab8`, `0x64aad7`, `0x64aae6`, `0x64ab05`, `0x64ab4b`, `0x64ab9b`, `0x64ab9f`, `0x64abaa`, `0x64abeb`, `0x64ac0a`, `0x64ac0f`, `0x64ac19`, `0x64ac2d`, `0x64ac48`, `0x64e850`, `0x7ffd74`
Confidence: verified
Proof: one
Test: `storm::sstrtofloat_whole`

`0x64aab1` passes 0x57 to `SErrSetLastError` and `0x64aab8` loads the float 0.0. `0x64aad7`
compares the first byte with `-` and keeps the flag for the `fchs` at `0x64ac48`. The digit tests
are `movsx`, `sub 0x30`, `cmp 0xa` with `jae` (`0x64aae6`, `0x64ab05`, `0x64ab4b`). `0x64ab9b`
compares with `.`, and `0x64ab9f` steps past it before `0x64abaa` tests the next byte, so a missed
fraction falls to the exponent test with the pointer after the dot. The fraction loop reads its next
byte at `0x64abeb`. `0x64ac0a` and `0x64ac0f` compare with `e` and `E`, `0x64ac19` skips one `+`,
and `0x64ac2d` calls `SStrToInt`. The test runs `0x64aaa0` whole over the cases above.

## Float value

The value is built at 53 bits; the steps hold while every one stays a normal double. Integer digits
gather in a 32-bit chunk, which at `0x19999999` folds into a double, `acc = pow(10, n)·acc + chunk`,
`n` its digit count, and restarts. The last chunk folds the same way, or is the value alone while
`acc` is 0. A fraction digit `d` at place `k`, from −1 down, adds `table[−k − 1][d]` while `k > −21`
and `pow(10, k)·d` past it. An exponent `e` makes the value `pow(10, e)·value`. `pow` is the C
runtime's `0x73f940`; a value is a correctly rounded decimal only as far as its powers are.

Addresses: `0x64aaa0`, `0x64aac6`, `0x64aace`, `0x64ab12`, `0x64ab19`, `0x64ab31`, `0x64ab46`, `0x64ab59`, `0x64ab64`, `0x64ab76`, `0x64abb6`, `0x64abc6`, `0x64abe4`, `0x64ac3b`, `0x64ac40`, `0x64a670`, `0x64a680`, `0x73f940`, `0x80e5a0`, `0x802970`, `0xc52560`, `0xc52ba0`
Confidence: verified
Proof: one
Test: `storm::sstrtofloat_whole`

`0x64aac6` builds the table through `0x64a670` while the guard `0xc52ba0` is 0. `0x64ab12` is
`chunk·10 + d` by two `lea`, and `0x64ab19` compares it with `0x19999999` (`jb`). The fold
`fld 10.0` (`0x80e5a0`), `fild n`, `pow` (`0x64ab31`), `fmul acc`, `fild qword chunk`, `faddp`
(`0x64ab46`) runs there and, unless `acc` equals 0.0 (`0x802970`, `0x64ab59`, `jnp` at `0x64ab64`),
again at the end (`0x64ab76`). `0x64abb6` compares the place with −21 (`jg`); the table path adds
`[0xc52560 + 8·(10·row + d)]` (`0x64abe4`), the other calls `pow` (`0x64abc6`), multiplies by the
digit and adds the stored value. The exponent calls `pow` at `0x64ac3b` and multiplies the stored
value at `0x64ac40`. `pow` reaches 10^n through `fyl2x` and `f2xm1`. The test takes `pow` as an
input: it runs `0x64aaa0` whole, its `pow` and table builder included, and the reimplementation reads
`pow(10, n)` from the same `pow` through a trampoline; inputs keep every step a normal double, and
the 200 table cells are compared.

## Float result

`SStrToFloat` leaves its value in `st0` as the 53-bit double the last step produced, never rounded
to a float; a caller sees it unrounded until it stores it. A step stored to a double and read back
overflows or rounds to a denormal past the double range, where `st0`'s wider exponent does not, so
the value past that range is not derived.

Addresses: `0x64aaa0`, `0x64ab48`, `0x64ab98`, `0x64abee`, `0x64ac48`, `0x64ac4f`, `0x688b7c`, `0x688b96`, `0x68900c`, `0x689026`
Confidence: verified
Proof: one
Test: `storm::sstrtofloat_whole`

The extent holds three stores of a float value, all `qword` and none `dword`: `fstp` at `0x64ab48`,
`fst` at `0x64ab98` and `0x64abee`; `st0` keeps its own copy through each `fst`. The `fchs`
(`0x64ac48`) and `ret 4` (`0x64ac4f`) leave `st0` unrounded. `HandleDistCullOption` (`0x688b70`) and
`HandleUnitDrawDistOption` (`0x689000`) call it and `fcom` `st0` against float bounds at `0x688b7c`
and `0x68900c`; their `fstp dword`, the pattern's control, comes after, at `0x688b96` and `0x689026`.
The test reads `st0` as a double and compares it bit for bit.

## String to integer

`SStrToInt(s)` (`0x64ac60`) reads one optional `-` and then decimal digits, as `SStrToFloat` tests
them, accumulating `v·10 + d` in wrapping 32-bit arithmetic, and negates after a `-`. No digit
gives 0; a null `s` sets error 0x57 and gives 0.

Addresses: `0x64ac60`, `0x64ac6a`, `0x64ac7b`, `0x64ac8f`, `0x64aca2`, `0x64aca5`, `0x64acbc`
Confidence: verified
Proof: one
Test: `storm::sstrtoint_whole`, `net::who_string_helpers`

`0x64ac6a` passes 0x57 to `SErrSetLastError` and returns 0. `0x64ac7b` compares with `-`. The digit
test is `0x64ac8f`; `0x64aca2` and `0x64aca5` are `lea eax,[eax+eax*4]; lea eax,[ecx+eax*2]`, and
`0x64acbc` is the `neg`. The test runs it whole over every length to 12 digits, the wrap and a fuzz.

## String to unsigned

`SStrToUnsigned(s)` (`0x64ade0`) reads decimal digits only, as `SStrToInt` tests them, accumulating
`v·10 + d` in wrapping 32-bit arithmetic. A first byte that is not a digit gives 0, so `-5`, `+5`,
` 5` and `abc` give 0: no sign or space is read. Digits stop at the first byte that is not one
(`12x3` gives 12) and `4294967296` wraps to 0. A null `s` sets error 0x57 and gives 0.

Addresses: `0x64ade0`, `0x64adea`, `0x64adf1`, `0x64adf7`, `0x64ae00`, `0x64ae0d`, `0x64ae11`, `0x64ae14`, `0x64ae1d`, `0x64ae20`, `0x64e850`
Confidence: verified
Proof: one
Test: `storm::sstrtounsigned_whole`

`0x64adea` passes 0x57 to `SErrSetLastError` (`0x64e850`) and returns 0. The first byte is
`movsx`, `sub 0x30`, `cmp 0xa` (`0x64adf7` to `0x64ae00`) and `jae` returns 0 (`0x64adf1`); a byte
of 0x80 or more sign-extends below zero and is no digit. The second byte is tested at `0x64ae0d`.
The loop is `lea eax,[eax+eax*4]; lea eax,[ecx+eax*2]` (`0x64ae11`, `0x64ae14`), the next byte
tested at `0x64ae1d` and `jb` at `0x64ae20`. The test runs `0x64ade0` whole over every length to 12
digits, the wrap, signs, other stops and a fuzz.

## Tokenizer

`SStrTokenize(&cursor, out, size, delims, quoted)` (`0x64ae50`) copies the next token of the string
`*cursor` into `out` and advances `*cursor`. It skips leading delimiters, then takes bytes to the
first delimiter, which it consumes (a `"` aside; below), or to the end of the string. A byte is
stored while the stored count plus 1 is below `size`: a token is cut at `size − 1` bytes, the rest
skipped, and a NUL follows if `size` is above 0. `*quoted`, when given, is 1 if a `"` opened or
closed a quote in the call, else 0. A null cursor, `*cursor` or delimiters, or a null `out` with a
nonzero `size`, sets error 0x57 and writes nothing. `eax` is no result.

Addresses: `0x64ae50`, `0x64ae5b`, `0x64ae5f`, `0x64ae77`, `0x64ae79`, `0x64ae8d`, `0x64aeb0`, `0x64aeda`, `0x64af00`, `0x64af46`, `0x64af4b`, `0x64af4e`, `0x64af66`, `0x64af6a`, `0x64af73`, `0x64af77`, `0x64af7f`, `0x64af86`, `0x64a3c0`, `0x64e850`
Confidence: verified
Proof: one
Test: `storm::sstrtokenize_whole`

`0x64ae5b` to `0x64ae77` test the cursor, `*cursor`, `out` with `size`, and the delimiters;
`0x64ae79` passes 0x57 to `SErrSetLastError` and returns with `ret 0x14`. `0x64ae8d` asks
`SStrChr` (`0x64a3c0`) whether `"` is a delimiter. The skip loop is `0x64aeb0` to `0x64aeda`. The
main loop `0x64af00` ends on a delimiter at `0x64af66`, which steps past it. The store test is
`cmp edx,[ebp+0x10]; jae` (`0x64af4b`, `0x64af4e`) with `edx` the stored count plus 1; `0x64af6a`
and `0x64af73` store the NUL when the count is below `size`; `0x64af7f` stores the cursor and
`0x64af86` the quote flag. `eax` leaves holding the quote pointer (`0x64af77`). The test runs
`0x64ae50` whole, with `SStrChr` run as its callee, over every string of five symbols to length
five against seven delimiter strings and five sizes, longer fuzz strings, lines tokenized call
after call, a null quote pointer and the argument errors, and compares every byte of the output
buffer, the cursor and the flag.

## Tokenizer quotes

A `"` is special only when it is one of the delimiters. A `"` among the leading delimiters opens a
quote and ends the skip. Inside a quote every byte, delimiters included, is token text until a
`"`, which closes the quote, is consumed and ends the token. Outside a quote a `"` after at least
one stored byte ends the token and is left in the string. The test is the stored count, not the
token's length, so with `size` 0 or 1 every `"` toggles the quote. An unclosed quote runs to the end of
the string, and `""` gives an empty token with `*quoted` set.

Addresses: `0x64ae50`, `0x64aebf`, `0x64aece`, `0x64af04`, `0x64af09`, `0x64af10`, `0x64af12`, `0x64af14`, `0x64af2a`, `0x64af30`
Confidence: verified
Proof: one
Test: `storm::sstrtokenize_whole`

`0x64aebf` compares the byte with `"` when the delimiter-quote flag `edi` is set; `0x64aece` stores
1 in the quote flag `[ebp-0xc]` and the open flag `[ebp-4]` and steps past the quote. In the main
loop `0x64af04` compares with `"`. `0x64af09` tests the stored count `[ebp-8]` and `0x64af10` the
open flag; a count with no open quote jumps to the end at `0x64af12` without consuming the quote.
`0x64af14` to `0x64af2a` otherwise toggle the open flag, step past the quote, set the quote flag and
end the token when the toggled flag is clear. `0x64af30` sends every byte in an open quote to the store,
past the delimiter test.

## String length and append

`SStrLen(s)` (`0x64a6f0`) returns the count of bytes before the first NUL. `SStrPack(dest, src, size)` (`0x64a760`)
appends `src` to the string in `dest` and returns the new length of `dest`. A `size` of 0 returns 0 and writes
nothing. Otherwise bytes of `src` are copied to the end of `dest` while the write address is below
`dest + size - 1` (unsigned 32-bit address arithmetic), and a NUL follows the last byte copied; a `dest` already at the
limit gets no byte and its NUL is rewritten at its old end. `size` 0x7fffffff has no limit. A null `s`, `dest` or
`src` sets error 0x57 and gives 0, `SStrPack` writing nothing even for a `size` of 0.

Addresses: `0x64a6f0`, `0x64a6fa`, `0x64a707`, `0x64a718`, `0x64a760`, `0x64a773`, `0x64a785`, `0x64a789`, `0x64a791`, `0x64a7a0`, `0x64a7a9`, `0x64a7ac`, `0x64a7b4`, `0x64a7c0`, `0x64a7c5`, `0x64a7c9`, `0x64a7d0`, `0x64e850`
Confidence: verified
Proof: one
Test: `storm::sstrlen_whole`, `storm::sstrpack_whole`

`0x64a6fa` and `0x64a773` pass 0x57 to `SErrSetLastError` (`0x64e850`). `SStrLen` tests the first byte at `0x64a707`
and subtracts the start at `0x64a718`. In `SStrPack`, `0x64a789` returns 0 when `size`, tested at `0x64a785`, is 0, after
the null tests; `0x64a791` to `0x64a79e` find the end of `dest`; `0x64a7a0` compares `size` with 0x7fffffff and
`0x64a7ac` forms the limit `lea edi,[ebx+ecx-1]`. The copy loop tests `src` for NUL (`0x64a7a9`, `0x64a7c0`) and the
write address with `jae` (`0x64a7b4`); `0x64a7c5` stores the NUL and `0x64a7c9` returns the write address less `dest`.
The unlimited loop is `0x64a7d0`. `sstrlen_whole` runs `0x64a6f0` over every string of four bytes (`a`, 0x80,
0xff, `%`) to length six, 2000 random strings, bytes after the NUL and the null string. `sstrpack_whole` runs `0x64a760`
over every destination and source of two letters to length five against 14 sizes (0 to 7, 9, 0x40, 0x7fffffff,
0x80000000 and the two that wrap the limit address), a fuzz and the null arguments, comparing the sentinel-filled
destination region whole. It cannot tell the unlimited loop from a limit above every source: for 0x7fffffff and
0x80000000 each source copies whole.

## String find

`SStrStr(haystack, needle)` (`0x64b4f0`; `0x64b4a0`, `SStrStr2`, has the same instructions and call targets, with other call displacements) returns the address of the first position
of `haystack` where `needle` matches, case-sensitively, else null. It tries each position from the start while
the byte there is not NUL, comparing the needle's length in bytes through `SStrCmp` (`0x64a480`). An empty haystack
gives null, even for an empty needle; an empty needle in a non-empty haystack matches at the start. A null
argument sets error 0x57 and gives null. `SStrStrI` (`0x64b540`) is the same loop over `SStrCmpI`.

Addresses: `0x64b4f0`, `0x64b4a0`, `0x64b4f4`, `0x64b4fb`, `0x64b501`, `0x64b50d`, `0x64b514`, `0x64b520`, `0x64b523`, `0x64b52a`, `0x64b52c`, `0x64b534`, `0x64b53a`, `0x64a480`, `0x64a4aa`, `0x40de80`, `0x64b540`, `0x64a6f0`
Confidence: verified
Proof: one
Test: `storm::sstrstr_whole`

`0x64b4f4` and `0x64b4fb` test the two arguments and `0x64b501` passes 0x57 to `SErrSetLastError`. `0x64b50d` keeps the
needle's length (`SStrLen`); `0x64b514` returns null for an empty haystack. The loop pushes the length, the needle and the
cursor and calls `SStrCmp` (`0x64b523`), which tests for null arguments and calls the C runtime's `strncmp`
(`0x64a4aa`), a `repz cmpsb` that folds no case (`0x40de80`). A zero result returns the cursor (`0x64b53a`); else
`0x64b52c` loads the next byte, steps and loops while it is not NUL, and the end returns null (`0x64b534`). The image
holds nine direct calls of `0x64b4f0` (`0x49ae3c`, `0x49f68b`, `0x49f9eb`, `0x4efee0`, `0x6ede6a`, `0x6eded2`, `0x6f12e0`,
`0x6f12f4`, `0x6f1359`) and five of `0x64b4a0`, and no dword holds either address. The controls: the call scan finds `0x64b540`'s four calls, and
the dword scan finds `0x4ebe70` in the vtable slot `0x807ac4`. The two copies differ in four bytes, all in the
displacements of their three `call`s (`0x64b4b1`, `0x64b4bd`, `0x64b4d3`). `sstrstr_whole` runs both whole over every
haystack of `a`, `A` and `b` to length five against needles to length three, 6000 random pairs over seven bytes, the
escape check's `|r`, and the null arguments.

## Unreached functions

`0x64a8e0` is `SStrToFloat`'s body with one difference: a null string loads the double 0.0
(`0x802970`) instead of the float. Nothing reaches it: no `call` or `jmp` in the listing targets it
and no dword of the image, at any alignment, holds its address. The controls: the same scans find
107 direct calls of `0x64aaa0` and the vtable slot `0x807ac4` holding `0x4ebe70`.

Addresses: `0x64a8e0`
Confidence: verified
Proof: one
Test: none
