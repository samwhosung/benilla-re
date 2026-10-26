# crt: formatted output

The client links Microsoft's C runtime, and its `printf` family formats through one engine,
`__output`, which writes into a C `FILE`. The string front ends `sprintf`, `_vsprintf`, `_snprintf`
and `_vsnprintf` hand it a string `FILE` (flags 0x42, the buffer and a count) and end the text with
a NUL. The floating-point conversions go through a slot the start-up fills, to `_cfltcvt` and the
`e`, `f` and `g` forms, which take their digits from `_fltout2` (`crt/decimal`). The client's own
callers include `luaV_tostring` (`%.14g`), the Lua 4.0 `string.format`, Storm's printf, the
console variable handlers, and the saved-variables writer `FrameScriptSerializeValue`, whose
numbers go to disk as `%.16g` through Storm's `SStrPrintf` (`0x70452c`). That makes the `g` form
and the digits under it a fact on disk, which wants a second independent reading; its entries
here rest on one.

Everything here runs in the C locale: nothing writes the locale handle `0x883b08`
(`storm/format`), so the decimal point `0x831714` is `.`, no byte is a lead byte and `0x831710`
is 1. The wide conversions (`%C`, `%S`, `%wZ`, and `l` or `w` before `c` or `s`) convert through
`_wctomb` or read UTF-16, which is not read here; `%Z` without `w` reads a counted narrow string.

Roots: `0x73fec4`, `0x73f6ad`, `0x740107`, `0x7412c1`, `0x409a11`

## The format state machine

`__output` reads the format a byte at a time until its NUL. A byte's class and the state give
the next state: 0 normal, 1 percent, 2 flag, 3 width, 4 dot, 5 precision, 6 size, 7 type. Bytes
outside 0x20 to 0x78 are of class other. The normal state writes the byte, so `%%` writes `%`
and a byte of class other ends a conversion by being written. State 1 clears the flags and width
and sets the precision to -1. `*` reads an int: a negative width sets `-` and is negated, a
negative precision is -1. Digits accumulate in 32 bits. `I` is a size only before `64`, else
written. `F`, `L` and `N` do nothing.

| class | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| other | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| `%` | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| `.` | 0 | 4 | 4 | 4 | 0 | 0 | 0 | 0 |
| `*` | 0 | 3 | 3 | 0 | 5 | 0 | 0 | 0 |
| `0` | 0 | 2 | 2 | 3 | 5 | 5 | 0 | 0 |
| `1` to `9` | 0 | 3 | 3 | 3 | 5 | 5 | 0 | 0 |
| space `#` `+` `-` | 0 | 2 | 2 | 0 | 0 | 0 | 0 | 0 |
| `F I L N h l w` | 0 | 6 | 6 | 6 | 6 | 6 | 6 | 0 |
| `B C E G S X Z c d e f g i n o p s u x` | 0 | 7 | 7 | 7 | 7 | 7 | 7 | 0 |

Addresses: `0x742ba5`, `0x742be2`, `0x742bfc`, `0x742c13`, `0x7432c6`, `0x8161e0`, `0x742c1a`, `0x742c35`, `0x742c81`, `0x742cb8`, `0x742cc0`, `0x742cf5`, `0x742d28`, `0x742d41`, `0x742d44`, `0x7432ad`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_states_difftest`, `crt::crt_sprintf_difftest`, `crt::crt_sprintf_float_difftest`

`0x742be2` takes the class as the low nibble of the byte at `0x8161e0 + c - 0x20` for `c` from
0x20 to 0x78, else 0; `0x742bfc` reads the next state as the high nibble of the byte at
`0x8161e0 + class * 8 + state`, and `0x742c13` jumps through the eight cases at `0x7432c6`. The
flags (`0x742c35`) are `+` 1, space 2, `-` 4, `0` 8 and `#` 0x80. The size case (`0x742cf5`) sets
0x8000 for `I64` (`0x742d28`, two more bytes taken), 0x20 for `h`, 0x10 for `l` and 0x800 for `w`;
an `I` without `64` goes to `0x742d41`, which clears the state and falls into the normal case
`0x742d44`. `crt_sprintf_states_difftest` runs `sprintf` whole over every class in every state,
and the other tests add `%y`, `%I32d`, bytes 0x01, 0x7f and 0xff, and a width and precision that
wrap.

## Padding, sign and text

Every conversion but `n` ends the same way. The pad is the width less the prefix and the text, in 32
bits. Without `-` or `0` the pad goes out as spaces; then the prefix; then, with `0` and without
`-`, the pad as zeros; then the text; then, with `-`, the pad as spaces. A signed conversion
(`d`, `i`, `e`, `f`, `g`) takes the prefix `-` for a negative value, else `+` with `+`, else a
space with the space flag. So `0` pads a string or a character with zeros too. A write that finds
no room sets the count to -1, and the engine returns -1 before the next format byte; otherwise
it returns the bytes written.

Addresses: `0x7431af`, `0x7431b9`, `0x7431e7`, `0x7431f0`, `0x743207`, `0x74321d`, `0x743280`, `0x743295`, `0x7432be`, `0x742bd9`, `0x7432e6`, `0x74331b`, `0x74334c`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_difftest`, `crt::crt_vsnprintf_difftest`

`0x7431af` skips the end when `[ebp-0x28]` is set, which `n` does. `0x7431b9` writes the sign
into the prefix only with flag 0x40, the signed conversions'.
`0x7431e7` computes `width - prefix - text`; `0x7431f0` tests `0xc`, `0x74321d` tests 8 then 4,
`0x743295` tests 4. `_write_char` (`0x7432e6`) decrements the `FILE`'s count and, when it goes
below zero, calls `__flsbuf`, which on a string `FILE` sets 0x20 and returns -1 without writing
(`0x7433c5`); the -1 makes the caller's count -1 (`0x743312`). `_write_multi_char` (`0x74331b`)
and `_write_string` (`0x74334c`) stop at a count of -1, and `0x742bd9` ends the loop on a
negative count.

## Integer conversions

`d` and `i` are signed decimal, `u` unsigned decimal, `o` octal, `x` and `X` hexadecimal in lower
and upper case, and `p` is `X` with the precision 8. The argument is eight bytes with `I64`, else
four, of which `h` keeps the low 16 bits, sign-extended for `d` and `i`. A precision clears the `0`
flag; without one it is 1. Digits are made until the value is 0 and the precision is met, so a
zero with precision 0 has none. With `#`, `x`, `X` and `p` take the prefix `0x` or `0X`, dropped
for a zero, and `o` gets a leading `0` unless it has one. Past the 512th digit the digits run
below the buffer.

Addresses: `0x742fe6`, `0x74301d`, `0x743024`, `0x74302b`, `0x74304d`, `0x74308a`, `0x74308e`, `0x743095`, `0x7430e8`, `0x74310d`, `0x743116`, `0x743129`, `0x743139`, `0x743157`, `0x743169`, `0x74318d`, `0x743384`, `0x743391`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_difftest`

The radix and the letter offset are set at `0x742fe6` (`x`, 0x27), `0x74301d` (`p`, precision 8)
into `0x743024` (`X`, 7), `0x74304d` (`o`, flag 0x200 with `#`) and `0x74308e` (10); `0x74308a`
adds the signed flag 0x40 for `d` and `i`. `0x74302b` stores `0` and `0x51` plus the offset as
the prefix. `0x743095` reads the argument by its size, `0x7430e8` negates a negative signed
value in 64 bits and sets 0x100, and `0x74310d` clears the high dword without `I64`. `0x743116`
defaults the precision or clears 8, `0x743129` drops the prefix for a zero, and `0x743139` loops
while the old precision is above 0 or the value is not, by `__aullrem` and `__aulldiv`
(`0x743157`, `0x743169`), adding the offset above `9`. The digits go down from `[ebp-0x49]`,
the top of the 512-byte buffer at `[ebp-0x248]`, so digits past the 512th overwrite the saved
`ebx`, `esi` and `edi` and then the stack below. `0x74318d` adds the octal `0`.

## Characters, strings and counts

`c` writes the low byte of an int argument. `s` writes the bytes of the string up to its NUL and
at most the precision of them; a null pointer writes `(null)`. `n` stores the number of bytes
written so far through its pointer argument, two bytes with `h`, else four, and writes nothing.
`h` makes `C` and `S` narrow. `B`, a type with no case, writes again the text the last conversion
left, with its own width and flags; before any conversion that text is empty. `S` and `C` without
a size, and `c` or `s` after `l` or `w`, are wide.

Addresses: `0x742dc6`, `0x742dd2`, `0x742ff8`, `0x742ffc`, `0x743005`, `0x742e60`, `0x742ea7`, `0x743060`, `0x743079`, `0x7431af`, `0x8761c0`, `0x8761c4`, `0x871708`, `0x81623c`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_difftest`

`0x742dc6` sets 0x800 for `S` and `C` when none of 0x830 is set. `0x742dd2` takes the precision,
-1 as 0x7fffffff, and with neither 0x10 nor 0x800 goes to `0x742ff8`, where a null pointer is
replaced by `[0x8761c0]`, the string `(null)` at `0x871708` (`0x742ffc`), and `0x743005` counts
bytes while the precision lasts and the byte is not 0. The wide path reads `[0x8761c4]`, the
UTF-16 `(null)` at `0x81623c`. `0x742e60` sends `c` with 0x810 to `_wctomb`, else `0x742ea7` takes
the low byte. `0x743060` stores a word with 0x20 (`0x743070`), else the dword (`0x743079`), and sets
the no-output flag. A type the switch at `0x742d88` has no case for reaches `0x7431af` with the
text pointer `[ebp-0x8]` and length `[ebp-0xc]` as they were, which the percent case does not
reset.

## Floating-point conversions

`e`, `f`, `g`, `E` and `G` read an eight-byte double. `E` and `G` set the capital exponent and are
then `e` and `g`. The precision is 6 when none is given, and 1 for `g` given 0. `_cfltcvt` writes
the text into the 512-byte buffer; with `#` and a precision of 0, `_forcdecpt` adds a decimal
point; for `g` without `#`, `_cropzeros` trims it. A leading `-` is taken off as the sign. The
size prefixes change nothing, so `%Lf` and `%lf` print a double. Nothing bounds the precision or
the text: a text past 512 bytes overruns the buffer up the stack frame.

Addresses: `0x742e29`, `0x742e33`, `0x742e3d`, `0x742f17`, `0x742f25`, `0x742f4f`, `0x742f5b`, `0x742f70`, `0x742f77`, `0x742f87`, `0x742f8e`, `0x742fa5`, `0x82f3f8`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_float_difftest`

`0x742e29` sets `[ebp-0x34]` to 1 and adds 0x20 to the byte for `E` and `G`; `0x742e33` sets the
signed flag. `0x742e3d` sets 6 for a negative precision, and `0x742f17` 1 for `g` at 0.
`0x742f25` copies the two argument dwords to `[ebp-0x48]` and `0x742f4f` calls `[0x82f3f8]` with
the double's address, the buffer `[ebp-0x248]`, the byte, the precision and the capital flag.
`0x742f5b` tests `#` and `0x742f70` calls `[0x82f404]` when the precision is 0; `0x742f87` calls
`[0x82f3fc]` for `g` without it. `0x742f8e` sets 0x100 and steps past a `-`. The buffer ends at the
double's copy, `[ebp-0x48]`, so a longer text overwrites it, the locals and the return address:
`%.600f` of any value, or `%.300f` of 1e300.

## The conversion slots

The start-up fills the slots the floating-point paths call through: `0x82f3f8` `_cfltcvt`
(`0x40b7fe`), `0x82f3fc` `_cropzeros` (`0x40b4a8`), `0x82f400` `_fassign` (`0x40b50e`), `0x82f404`
`_forcdecpt` (`0x40b44e`), `0x82f408` `0x40b4f6`, and `0x82f40c` `_cfltcvt` again. Their file
values are all `_fptrap` (`0x410425`), which ends the process with runtime error 2.
`__cinit` calls `__fpmath` through `0x82f274` before the initializer tables, and `__fpmath`
fills the slots first.

Addresses: `0x409a29`, `0x409a11`, `0x82f274`, `0x40a2e0`, `0x82f3f8`, `0x410425`
Confidence: verified
Proof: one
Test: none

`0x409a29` stores the six values; `__fpmath` (`0x409a11`) calls it at its first instruction, then
`__ms_p5_mp_test_fdiv` and `__setdefaultprecision`. `__cinit` (`0x40a2e0`) calls `[0x82f274]`
when it is not 0, and its file value is `0x409a11`. Census of `0x82f3f8` to `0x82f40f`: the
listing's instructions that name an address in it, and every little-endian dword of the image in
that range, which agree. The writes are the six stores of `0x409a29`; the rest are the three calls
of `__output`, the call of `0x82f400` at `0x745251` and two compares, at `0x40b26c` and `0x40b3b7`,
so `0x82f408` and `0x82f40c` are never read. `0x82f274` is named only by `0x40a2e0`. `0x410425`
pushes 2 and calls `__amsg_exit`.

## Rounding the digit text

`_fptostr` writes `0` and then `n` digits of the mantissa text, `0` once the text runs out. For
`n` of 0 or more, when the next mantissa byte is `5` or above as a signed byte, it adds one at the
last digit, nines becoming zeros as the carry moves left. A carry into the leading `0` makes it `1`
and adds one to the decimal exponent; otherwise the leading `0` is removed. The mantissa of a
non-finite value is its text, so `%.2f` of +∞ rounds `I` up: `1.#J`. For `n` below 0 the result is
empty. `__shift` moves a string up by `n` bytes, its NUL included, and does nothing for 0.

Addresses: `0x40fef9`, `0x40ff0e`, `0x40ff11`, `0x40ff1c`, `0x40ff39`, `0x40ff3d`, `0x40ff42`, `0x40ff4f`, `0x40ff54`, `0x40ff63`, `0x40b84f`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_float_difftest`

`0x40ff0e` stores the `0`; the loop at `0x40ff1c` copies a byte and advances the mantissa only when
it is not 0. `ebx` is zeroed in the loop's preamble, so `0x40ff39` skips the rounding only for a
negative `n`. `0x40ff3d` compares the next byte with 0x35 by `jl`; `0x40ff42` steps back over
`9`s, writing `0`. `0x40ff4f` tests the first byte for `1` and `0x40ff54` increments the decimal
exponent `[flt+4]`, else `0x40ff63` moves the rest down one with the copy `0x4100f0`. `__shift`
(`0x40b84f`) copies `strlen + 1` bytes to `s + n` when `n` is not 0.

## The e and f forms

`_cfltcvt` sends `e` and `E` to `_cftoe`, `f` to `_cftof` and anything else to `_cftog`.
`_cftoe` rounds to `ndec + 1` digits written after room for the sign and the point, then
`_cftoe2` writes `-`, the first digit, the point when `ndec` is above 0, the rest, and the exponent
from the template `e+000`: `E` when capital, the decimal exponent less one with its sign and three
digits, and `+000` for a zero. `_cftof` rounds to `decpt + ndec` digits, then `_cftof2` writes `-`,
the integer digits or `0`, and for `ndec` above 0 the point, then the zeros a negative exponent
puts before the first digit, at most `ndec` of them, then the digits.

Addresses: `0x40b7fe`, `0x40b54c`, `0x40b565`, `0x40b58d`, `0x40b59f`, `0x40b5ad`, `0x40b5db`, `0x40b5e8`, `0x40b5ff`, `0x40b620`, `0x40b627`, `0x40b62f`, `0x40b638`, `0x40b665`, `0x40b66f`, `0x40b6a7`, `0x40b6c4`, `0x40b701`, `0x40b71a`, `0x40b730`, `0x40b745`, `0x40b75c`, `0x8001e0`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_float_difftest`

`0x40b7fe` compares the format with 0x65, 0x45 and 0x66. `_cftoe` (`0x40b54c`) passes `_fptostr`
the buffer plus the sign and `ndec > 0` (`0x40b58d`). `_cftoe2` (`0x40b5ad`) moves the first digit
down and stores the point after it (`0x40b5e8`), copies the template `0x8001e0` to the point plus
`ndec` plus 1 (`0x40b5ff`), writes `E` (`0x40b620`), and, unless the mantissa starts with `0`
(`0x40b627`), adds the hundreds, tens and units of `|decpt - 1|` into the template's digits
(`0x40b638` to `0x40b665`). `_cftof` (`0x40b66f`) asks `decpt + ndec` digits (`0x40b6a7`).
`_cftof2` (`0x40b6c4`) shifts in a `0` for a `decpt` of 0 or less (`0x40b701`), a point
(`0x40b71a`), and for a negative `decpt` (`0x40b730`) `min(ndec, -decpt)` zeros (`0x40b745`,
`0x40b75c`).

## The g form and its trims

`_cftog` rounds to `ndec` significant digits, then takes the decimal exponent after the rounding,
less one: below -4 or at least `ndec`, the `e` form with `ndec - 1` digits after the point;
otherwise the `f` form, with every zero a negative exponent puts before the first digit, and with
the last digit dropped when the rounding carried into a new first digit. `_cropzeros`, for `g`
without `#`, removes the zeros before the exponent or the end of the fraction, and the point
when no digit is left after it. `_forcdecpt`, for `#` with precision 0, puts a point after the
leading digits, or before an `e` that comes first.

Addresses: `0x40b76b`, `0x40b791`, `0x40b7a8`, `0x40b7b3`, `0x40b7b9`, `0x40b7be`, `0x40b7c2`, `0x40b7cd`, `0x40b7da`, `0x40b7f1`, `0x40b6d4`, `0x40b739`, `0x40b4a8`, `0x40b4c4`, `0x40b4cb`, `0x40b4e0`, `0x40b4e6`, `0x40b4eb`, `0x40b44e`, `0x40b462`, `0x40b48e`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_float_difftest`

`_cftog` keeps `decpt - 1` before the rounding (`0x40b791`), calls `_fptostr` for `ndec` digits
(`0x40b7a8`), and compares `decpt - 1` after it (`0x40b7b3`) with -4 (`0x40b7b9`) and `ndec`
(`0x40b7be`). The `f` path clears the last byte when the exponent grew (`0x40b7c2`, `0x40b7cd`)
and calls `_cftof2` with `g_fmt` 1 (`0x40b7da`), where `0x40b739` takes all `-decpt` zeros; the
`e` path calls `_cftoe2` with `g_fmt` 1 (`0x40b7f1`), which first shifts the digits up for the
point and puts the template at the point plus `ndec`. `0x40b6d4`, for `g_fmt` and `decpt - 1`
equal to `ndec`, writes a `0` and a NUL there; `_cftog` sends that case to the `e` form, so it does
not run. `_cropzeros` (`0x40b4a8`) finds the point, then the `e`, `E` or NUL (`0x40b4cb`), backs
over `0`s (`0x40b4e0`) and the point (`0x40b4e6`), and copies the tail down (`0x40b4eb`).
`_forcdecpt` (`0x40b44e`) skips digits from the second byte (`0x40b462`) and shifts in the point
(`0x40b48e`).

## The string front ends

`sprintf` and `_vsprintf` give `__output` the count 0x7fffffff; `_snprintf` and `_vsnprintf` give
it the caller's count, as a signed int. One more decrement of the count then decides the end: at
zero or above, a NUL at the cursor; below zero, nothing. So a text shorter than the count ends in a
NUL and returns its length; a text of exactly the count has no NUL and returns the count; a longer
one stops at the count with no NUL and returns -1. A count of 0 or from 0x80000001 up leaves no
room: nothing is written, and the return is -1, or 0 for an empty text. 0x80000000 wraps to
0x7fffffff on its first decrement and has no bound.

Addresses: `0x73fec4`, `0x73fee5`, `0x73fef5`, `0x73f6ad`, `0x73f6c6`, `0x73f6dc`, `0x740107`, `0x740121`, `0x7412c1`, `0x7412e1`, `0x7433af`, `0x7434bb`
Confidence: verified
Proof: one
Test: `crt::crt_vsnprintf_difftest`, `crt::crt_snprintf_vsprintf_difftest`, `crt::crt_sprintf_difftest`

Each builds the `FILE` on its stack: the cursor and base at the buffer, flags 0x42, the count
0x7fffffff (`0x73fee5`, `0x7412e1`) or the caller's (`0x73f6c6`, `0x740121`). After `__output`,
`dec` of the count and `js` (`0x73fef5`, `0x73f6dc`) choose between `and byte [cursor],0` and
`__flsbuf(0)`, which for flags 0x42 sets 0x20 and returns -1 at `0x7434bb`. The return is
`__output`'s. `_write_char`'s `dec` and `js` see the count as signed, so 0x80000000 becomes
0x7fffffff.
