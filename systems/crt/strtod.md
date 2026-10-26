# crt: reading numbers

How the C runtime reads a decimal numeral into a `double` or a `float`. `___strgtold12` reads the
numeral into at most 24 decimal digits and a power of ten, `___mtold12` makes the digits a 12-byte
real (`Ld12Real`, `crt/decimal`), `___multtenpow12` scales it, and `__ld12cvt` rounds it to the
double's or the float's format. `strtod` fronts the scan through `__fltin2` and reads its flags.
The arithmetic is the runtime's own, so the answer is not always the double nearest the text:
the scaling truncates partial products, the rounding to the format skips one bit of its sticky
test, and `strtod` answers +0 for a result below the normal range, but a signed zero where the
12-byte product itself flushes to zero.

`luaO_str2d` (`0x6f5900`, `lua/object`) is the only caller of `strtod`, so every numeral in a
script and every `tonumber` of text is read here. `scanf`'s float conversions call `_fassign`
through the slot `0x82f400` (`crt/printf`), which rounds the same 12-byte real to a double or
straight to a float. Everything runs in the C locale: the decimal point `0x831714` is `.`.

Roots: `0x741e39`, `0x40b50e`

## The numeral scan

`___strgtold12(out, &end, s, mult12, scale, decpt, implicit_e)`, with its last four arguments 0,
skips space, tab, newline and return, then reads an optional sign, digits with at most one `.` (a
digit on either side), and an exponent after `e`, `E`, `d` or `D`: an optional sign and digits,
held at 5201 once past 5200. The first 25 significant digits are kept; a further integer digit adds
1 to the power of ten and a further fraction digit is dropped. A sign after the mantissa ends the
numeral. A numeral with no digit answers flags 4.

Addresses: `0x4111f0`, `0x41122b`, `0x411253`, `0x411691`, `0x4112d1`, `0x411358`, `0x41136e`, `0x411396`, `0x4113c9`, `0x4113db`, `0x41151e`, `0x411496`, `0x411650`
Confidence: verified
Proof: two
Test: `crt::crt_strgtold12_difftest`, `crt::crt_strtod_difftest`

The blank loop is `0x41122b` to `0x411242`. The state in `eax` is dispatched by
`jmp [eax*4+0x411691]` (`0x411253`) over twelve cases: 0 the start `0x41125a`, 1 after a leading
zero `0x4112a9`, 2 after a sign `0x411300`, 3 integer digits `0x41132a`, 4 fraction digits
`0x411385`, 5 a point before any digit `0x4113fc`, 6 after the mark `0x411432`, 7 after the
exponent's sign `0x41147c`, 8 its zeros `0x41145b`, 9 its digits `0x4114e0`, 10 the end
`0x4114ca`, and 11 a sign after the mantissa `0x411496`, which ends the numeral at `0x4114c6`. A digit is stored while the count `[ebp-4]` is below 0x19 (`0x411358`,
`0x4113c9`); an integer digit past them adds 1 to `[ebp-8]` (`0x41136e`), and a stored fraction
digit subtracts 1 (`0x4113db`), as each fraction zero before the first digit does (`0x411396`).
The mark test is `0x4112d1` to `0x4112eb`. The exponent is held by `cmp esi,0x1450; jg`
(`0x41151e`). With no digit `0x411650` answers 4.

## Where the numeral ends

`end` is after the numeral, at the mark when the exponent has no digit, and at `s` when the numeral
has none, which answers flags 4 and a zero of the sign read. `__fltin2` stores the bytes read, and
`strtod` sets `end` to its blank-skipped `s` plus them, or to the unskipped `s` for a numeral with
no digit. The calls from `__fltin2`, `_atodbl` and `_atoflt` pass 0 for the last four arguments.
Set, `implicit_e` takes a sign after the mantissa as the exponent's, `scale` is added to the power
when no exponent was read, `decpt` subtracted when no point was, and `mult12` goes to
`___multtenpow12`.

Addresses: `0x4112d1`, `0x411432`, `0x4114d8`, `0x411496`, `0x411650`, `0x4115d6`, `0x4115de`, `0x741e94`, `0x741e9d`, `0x741eaa`, `0x74756b`, `0x74757c`, `0x40fe79`, `0x40fe88`, `0x40fed4`, `0x40fee3`
Confidence: verified
Proof: one
Test: `crt::crt_strgtold12_difftest`, `crt::crt_strtod_difftest`

State 6 keeps `p − 2` as the end to fall back to (`0x411432`), and states 2, 5 and 7 end there or
at `s` (`0x4114d8`). State 11 (`0x411496`) ends the numeral at `0x4114c6` unless the last argument
is set. `0x4115d6` adds `[ebp+0x18]` when the exponent flag `[ebp-0x20]` is clear and `0x4115de`
subtracts `[ebp+0x1c]` when `[ebp-0x24]` is. `strtod` adds the stored count to the skipped `s`
(`0x741e94`); `test dx,0x240` (`0x741e9d`) stores the original `s` (`0x741eaa`). The three calls
push four zeros before the call (`0x74756b`, `0x40fe79`, `0x40fed4`).

## Digits to a 12-byte real

With 25 digits kept, the 24th is raised by 1 when it is 5 or more, whatever the 25th is, and the
25th is dropped with 1 added to the power. Trailing zero digits are dropped the same way. The
digits become a 96-bit integer by `x · 10 + d`, shifted up until bit 79 is set, with the exponent
0x404e less the shifts. With the scale and point arguments 0, the power is the exponent, negated
for `-`, plus the digit count's adjustment. Above 5200 the result is the infinity with flags 2,
below -5200 zero with flags 1; otherwise `___multtenpow12(out, power, mult12)` scales it. The sign
is set last.

Addresses: `0x41157a`, `0x41157f`, `0x4115a1`, `0x4115ba`, `0x411129`, `0x41119d`, `0x4111cf`, `0x4115c7`, `0x4115e1`, `0x411618`, `0x411630`, `0x411679`
Confidence: verified
Proof: two
Test: `crt::crt_strgtold12_difftest`, `crt::crt_strtod_difftest`

`cmp [ebp-4],0x18; jbe` (`0x41157a`) and `cmp byte [ebp-0x45],5; jl` (`0x41157f`) read the digit
at offset 23 of the buffer `[ebp-0x5c]`, the 24th. The loop at `0x4115a1` drops zero digits from
the end, adding 1 to `[ebp-8]` each. `___mtold12` (`0x411129`) multiplies by two `___shl_12`, an
`___add_12` of the copy and one more `___shl_12`, then adds the digit sign-extended; it shifts by
16 while the top dword is 0 (`0x41119d`), adding 0xfff0 to the exponent, then by 1 until bit 0x8000
of that dword is set (`0x4111cf`). `0x4115c7` negates the exponent for `[ebp-0x18]` below 0 and adds the adjustment `[ebp-8]`;
`cmp eax,0x1450; jle`
(`0x4115e1`) and `cmp eax,-5200; jge` (`0x411618`) bound it, and `0x411630` calls
`___multtenpow12`. `0x411601` writes mantissa `0x80000000` in the top dword and exponent 0x7fff;
`0x411679` ORs the sign `[ebp-0x28]` (0x8000 for `-`) into the exponent word.

## Rounding to a format

`__ld12cvt(x, out, format)` rounds the 80-bit mantissa to the format's 53 or 24 bits, adding 1 at
the last kept bit when the bit after it is set and so is any bit after the next one; those two
bits alone never round up. A carry raises the exponent. At or below the minimum exponent, -1023 or
-127, the unrounded mantissa is shifted into a subnormal, rounded and shifted one further,
answering 2; below the minimum less the precision, -1076 or -151, it is zero, answering 2. An
exponent of 1024 or 128 or more is the infinity, answering 1; a normal result answers 0. A zero
exponent field gives zero, answering 2 unless the mantissa is zero.

Addresses: `0x40fc5f`, `0x40fca4`, `0x40fce0`, `0x40fcf5`, `0x40fd05`, `0x40fd1f`, `0x40fd48`, `0x40fd74`, `0x40fd8d`, `0x40fb04`, `0x40fb43`, `0x40fb47`, `0x40fa65`, `0x40fa85`, `0x40faae`, `0x40fbd2`, `0x40fb90`, `0x40fbab`, `0x40fbb7`, `0x831928`, `0x831940`, `0x40fdcb`, `0x40fde1`
Confidence: verified
Proof: two
Test: `crt::crt_ld12cvt_difftest`, `crt::crt_strtod_difftest`

The format is six dwords: the maximum exponent, the minimum, the precision, the exponent width,
the width and the bias, `(1024, -1023, 53, 11, 64, 1023)` at `0x831928` for `_ld12tod`
(`0x40fdcb`) and `(128, -127, 24, 8, 32, 127)` at `0x831940` for `_ld12tof` (`0x40fde1`). The
mantissa is three dwords, the highest first. `__RoundMan` (`0x40fb04`) tests bit `precision`
(`0x40fb43`), then calls `__ZeroTail` with `precision + 1` (`0x40fb47`), whose mask
`not (-1 << (31 − n mod 32))` (`0x40fa85`) covers only the bits after bit `n`; `__IncMan`
(`0x40faae`) adds at bit `precision − 1` through `___addl`. `0x40fca4` takes exponent field 0;
`0x40fce0` rounds; `0x40fcf5` is the zero, `0x40fd05` the subnormal path, which copies the saved
mantissa back (`__CopyMan`, `0x40fb90`) and shifts by `min − e`, `e` the exponent after the
first rounding, and then by the exponent width
plus 1 (`__ShrMan`, `0x40fbd2`, at `0x40fd1f` and `0x40fd39`); `0x40fd48` the infinity;
`0x40fd74` clears the leading bit and adds the bias. `0x40fd8d` assembles
`exponent << (31 − exponent width) | sign | top`, the width `[fmt+0xc]` (`0x40fd90`).

## strtod

`strtod(s, &end)` skips blanks, 9 to 13 and space, and reads on with `__fltin2`. No digit
answers +0. An overflow, the scan's exponent above 5200 or the rounding's
infinity, answers the infinity of the sign of the first byte after the blanks and sets `errno` to 34 (`ERANGE`). An
underflow, the scan's exponent below -5200 or a result rounding below 2^-1022 (`__ld12cvt`'s 2),
answers +0 for either sign with `ERANGE`. A scale `___ld12mul` flushes to zero, from about
1e-4951 to the -5200 bound, leaves a zero real with the numeral's sign, answered as that zero,
`errno` untouched: `-1e-5000` is -0. Otherwise it answers the rounded double: `6e23` reads as the
double below the nearest.

Addresses: `0x741e39`, `0x741e46`, `0x741e85`, `0x741e9d`, `0x741eae`, `0x741ed3`, `0x747560`, `0x74757c`, `0x747586`, `0x7475a0`, `0x7475a6`, `0x7475b6`, `0x875e90`, `0x4104e6`, `0x412f4f`, `0x411679`, `0x40fcb4`
Confidence: verified
Proof: two
Test: `crt::crt_strtod_difftest`

`0x741e46` loops while `_pctype[c] & 8`. `__fltin2` (`0x747560`) calls `___strgtold12` with the
last four arguments 0 (`0x74757c`); flags 4 give `0x200` (`0x747586`); otherwise `_ld12tod`
(`0x7475a0`), and scan flag 2 or answer 1 gives `0x80` (`0x7475a6`), scan flag 1 or answer 2 adds
`0x100` (`0x7475b6`); it stores the flags and the double. `test dx,0x240` (`0x741e9d`) gives `fldz`;
`test dl,0x81` (`0x741eae`) loads `0x875e90`, negated when the first byte is `-`, and stores 0x22
through `__errno` (`0x4104e6`); `test dh,1` (`0x741ed3`) gives `fldz` with the same `errno`.
`cmp dx,0x3fbf; ja` (`0x412f4f`) makes a flushed product zero, `0x411679` ORs the sign into it,
and `__ld12cvt` answers 0 for the all-zero mantissa (`0x40fcb4`).

## The scanf float assignment

`_fassign(is_double, out, text)` reads the text a `scanf` float conversion collected with
`___strgtold12`, no scale, and rounds it with `_ld12tod` into a double, or with `_ld12tof`
straight into a float, one rounding from the 12-byte real. Nothing tests the answer: an
overflow stores the infinity and a subnormal stores `__ld12cvt`'s subnormal bits.

Addresses: `0x40b50e`, `0x40b513`, `0x40b520`, `0x40b53b`, `0x40fe71`, `0x40fe88`, `0x40fe94`, `0x40fecc`, `0x40fee3`, `0x40feef`
Confidence: verified
Proof: one
Test: none

`cmp [ebp+8],0` (`0x40b513`) picks `_atodbl` (`0x40fe71`, called at `0x40b520`), which stores eight
bytes, or `_atoflt` (`0x40fecc`, at `0x40b53b`), which stores four. Each calls `___strgtold12`
with four zero arguments (`0x40fe88`, `0x40fee3`) and then `_ld12tod` (`0x40fe94`) or `_ld12tof`
(`0x40feef`) on its stack real; the answer in `eax` goes back unread.
