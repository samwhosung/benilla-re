# crt: decimal digits

How the C runtime makes the decimal digits of a `double` for its `e`, `f` and `g` conversions
(`crt/printf`). `_fltout2` widens the double to the 80-bit real with `___dtold`, and
`I10_OUTPUT` scales that by a power of ten into [0.1, 1) and reads digits off it. The arithmetic
is the runtime's own, in a 12-byte real (`Ld12Real`): a 16-bit extension below the 64-bit mantissa
and the sign over a 15-bit exponent biased by 0x3fff, multiplied by `___ld12mul` and scaled by
`___multtenpow12` from two tables of powers of ten. No x87 instruction touches the value past
the load and store of the double in the conversions. So the digits are those of that
arithmetic, which truncates partial products and rounds twice: on the 80 bits of every product,
and on an 18th decimal digit.

`___multtenpow12` and `___ld12mul` also serve `___strgtold12`, the text-to-number scan of
`strtod`, which is not read here.

Roots: `0x40ff70`, `0x413111`

## Widening a double

`___dtold` gives the 80-bit real of a double exactly: the sign, the exponent plus 0x3c00, and
the 52 fraction bits shifted up 11 under an explicit leading 1. Exponent 0x7ff becomes 0x7fff,
the fraction kept, so a NaN keeps its quiet bit as the mantissa's second bit. A subnormal takes
exponent 0x3c01 with no leading 1 and is shifted up until its top bit is set, the exponent
falling one per shift. Both zeros become all zero bits: the sign of -0 is dropped.

Addresses: `0x40ffcc`, `0x40ffd6`, `0x410005`, `0x41000b`, `0x410013`, `0x41001a`, `0x410024`, `0x410032`, `0x41003b`, `0x410053`, `0x410074`
Confidence: verified
Proof: one
Test: `crt::crt_dtold_difftest`

`___dtold(out, in)` takes the top word's sign `& 0x8000` and exponent `>> 4 & 0x7ff`
(`0x40ffd6`). A nonzero exponent other than 0x7ff adds 0x3c00 (`0x41000b`); 0x7ff gives 0x7fff
(`0x410013`). Exponent 0 with a zero fraction stores ten zero bytes (`0x410024`); with a nonzero
fraction it takes 0x3c01 and clears the leading bit `[ebp-4]`, which `0x410005` set to 0x80000000
(`0x410032`). `0x41003b` builds `hi = fraction_hi << 11 | lo >> 21 | lead` and `lo << 11`. The loop at
`0x410053` shifts the 64 bits left and adds 0xffff to the exponent while bit 63 is clear, and
`0x410074` stores the low word of `sign | exponent`.

## The 12-byte real

Three helpers work on the 96 bits of a 12-byte real as one integer, the low word first:
`0x411070` adds two of them, the carry out of the top dword dropped; `0x4110ce` shifts left by one
and `0x4110fc` right by one, a zero coming in. `___addl` adds two dwords into a third and returns
the carry.

Addresses: `0x41104f`, `0x411070`, `0x4110ce`, `0x4110fc`
Confidence: verified
Proof: one
Test: `crt::crt_i10_output_difftest`, `crt::crt_ld12mul_difftest`

`___addl` (`0x41104f`) returns 1 when the sum is below either addend. `0x411070` adds the low
dwords, a carry adding 1 to the middle and, on its carry, to the top; then the middle dwords, a
carry adding 1 to the top; then the top dwords. `0x4110ce` and `0x4110fc` move bit 31 of each dword
into bit 0 of the next and back. The tests run them inside `I10_OUTPUT` and `___ld12mul`.

## Multiplying 12-byte reals

`___ld12mul(x, y)` sets `x` to `x*y`. Of the word products `x[p]*y[q]` of the five-word
mantissas, only those with `p+q` of 4 or more are added, at word `p+q-4` of a 96-bit sum. The exponent is the sum less 0x3ffe, one more per operand of exponent 0. The sum shifts left
until its top bit is set while the exponent is above 0; an exponent `e` of 0 or below shifts it
right `1-e` places, a sticky bit kept. It rounds to nearest even on the low word and keeps the top
80 bits. An exponent of 0x7fff in or out, or a sum above 0xbffd, is infinite; a sum of 0x3fbf or
less, or a zero operand, is +0.

Addresses: `0x412ef1`, `0x412f29`, `0x412f39`, `0x412f44`, `0x412f4f`, `0x412f5a`, `0x412f7e`, `0x412f94`, `0x412fb1`, `0x412fe6`, `0x412ff5`, `0x413015`, `0x413023`, `0x413048`, `0x413061`, `0x413081`, `0x413098`, `0x4130cd`, `0x4130d6`, `0x4130f1`
Confidence: verified
Proof: one
Test: `crt::crt_ld12mul_difftest`

The sign is the XOR of the sign bits. `0x412f29`, `0x412f39` and `0x412f44` send an exponent of
0x7fff or a sum above 0xbffd to `0x4130f1`, which stores mantissa `0x8000 << 48` and exponent
0x7fff with the sign. `0x412f4f` zeroes `x` for a sum of 0x3fbf or less. An exponent of 0 adds one
to the sum (`0x412f5a`, `0x412f7e`), and a zero operand stores +0 (`0x412f94`). The loops at
`0x412fb1` add `x[i + k] * y[4 - k]` by `___addl` into the dword at word `i` (`0x412fe6`), a carry
incrementing word `i + 2` (`0x412ff5`); the kept products sum below 2^96, so no word wraps.
`0x413015` adds 0xc002; `0x413023` normalizes, `0x413048` subtracts one more and shifts right,
counting set bits shifted out (`0x413061`). `0x413081` rounds when the low word is above 0x8000
or the low 17 bits are 0x18000, carrying through the 80 bits (`0x413098`); a carry out sets the
top word to 0x8000 and adds one to the exponent. `0x4130cd` tests 0x7fff and `0x4130d6` stores
words 1 to 5.

## Powers of ten

`___multtenpow12(x, pow, mult12)` multiplies `x` by 10^pow, a factor per nonzero octal digit of
`|pow|`: digit `d` of group `g`, from the low one, is row `7g+d-1` of the table for the sign. The
rows are 10^1 to 10^7, then by 8 to 10^56, by 64 to 10^448, by 512 to 10^3584, and 10^4096. A row
holds the power's mantissa rounded to nearest at 64 bits and the remainder rounded to a signed
16-bit extension; an extension of 0x8000 or more takes the low dword one lower, so each factor
is the power rounded to nearest at 80 bits. `mult12` 0 clears the extension of `x`. A power of 0
does nothing; 8192 or more reads past the table.

Addresses: `0x413111`, `0x413118`, `0x41312c`, `0x413139`, `0x41314b`, `0x413151`, `0x41315f`, `0x413170`, `0x41317a`, `0x832490`, `0x8325f0`
Confidence: verified
Proof: one
Test: `crt::crt_multtenpow12_difftest`

`0x413118` takes the positive table less 0x60, `0x41312c` the negative one for a negative `pow`,
negated; `0x413139` clears word 0 of `x` when `mult12` is 0. The loop (`0x41314b`) adds 0x54, seven
rows, to the base per group and takes `pow & 7`, shifting `pow` right by 3 (`0x413151`); a digit
picks the row at base plus 12 times the digit. `0x41315f` compares the row's word 0 with 0x8000,
and `0x413170` copies the row and decrements its dword at offset 2. `0x41317a` calls
`___ld12mul`. Each of the 58 rows at `0x832490` and `0x8325f0` is its power by this rule exactly,
and no row with an extension of 0x8000 or more has a low dword of 0, so the decrement never
borrows. Census: the only instructions or image dwords that name an address in either table are
the two `mov ebx` of `0x413118` and `0x41312c`, so nothing writes them.

## Zeros and non-finite values

`I10_OUTPUT(ld, ndigits, flags, fos)` writes the digits of an 80-bit real into `fos` with the sign
`-` for a set sign bit, else a space. Exponent and mantissa both zero give the digit `0`, exponent
0 and the sign a space, and return 1. Exponent 0x7fff gives exponent 1 and returns 0, with the
digits `1#INF` for the mantissa `0x8000 << 48`, `1#IND` for a set sign and the mantissa
`0xc000 << 48`, `1#SNAN` for any other mantissa with bit 62 clear, and `1#QNAN` for the rest.

Addresses: `0x4116fd`, `0x411759`, `0x41176b`, `0x411774`, `0x411789`, `0x4117a2`, `0x4117b6`, `0x4117c4`, `0x4117cb`, `0x4117d5`, `0x4117f2`, `0x8003dc`, `0x8003e4`, `0x8003ec`, `0x8003f4`
Confidence: verified
Proof: one
Test: `crt::crt_i10_output_difftest`

`0x411759` stores `-` or space. `0x41176b` sends exponent, high and low dwords all zero to
`0x411774`, which stores exponent 0, a space, count 1 and `0`. Exponent 0x7fff (`0x411789`) stores
exponent 1; `0x4117a2` sends a mantissa other than `0x8000 << 48` with bit 62 clear to copy
`0x8003f4`; `0x4117b6` with the sign set and a high dword of 0xc0000000 copies `0x8003ec` for a low
dword of 0 (`0x4117c4`), else `0x8003dc`; `0x4117cb` copies `0x8003e4` for `0x8000 << 48`
(`0x4117d5`), else `0x8003dc` (`0x4117f2`). The count is 5 or 6 and the return 0.

## Making the digits

For a finite nonzero value, `I10_OUTPUT` estimates the decimal exponent `r`, scales the
magnitude by 10^-r, and when that is 1 or more adds one to `r` and multiplies by 0.1 truncated to
80 bits. The digit count is `ndigits`, plus `r` with bit 0 of `flags` (then 0 or less is the zero),
at most 21. It makes one digit more than the count, each the integer part of ten times the
remaining fraction. A last digit of 5 or more adds one to the digit before it, nines carrying; a
carry out of the first makes it `1` and raises the exponent. Otherwise trailing zeros go, and
none left is the zero.

Addresses: `0x4116fd`, `0x41171e`, `0x411808`, `0x411849`, `0x411851`, `0x411862`, `0x411869`, `0x411886`, `0x41188e`, `0x41189d`, `0x4118b3`, `0x4118de`, `0x41190f`, `0x41192b`, `0x411930`, `0x41194b`, `0x411968`, `0x411978`
Confidence: verified
Proof: one
Test: `crt::crt_i10_output_difftest`

`0x41171e` builds 0.1 as ten `0xcc` bytes under exponent 0x3ffb. The estimate (`0x411808`) is the
top 16 bits, signed, of `e * 0x4d10 + ((e >> 8) + (mantissa >> 56) * 2) * 0x4d - 0x134312f4` for
exponent `e`, a 16.16 log10. `0x411849` calls `___multtenpow12` with `-r` and `mult12` 1;
`0x411851` tests exponent 0x3fff before the multiply at `0x411862`. `0x411869` adds `r` for the
flag, and `0x411886` caps at 21. `0x41188e` clears the exponent word after taking its distance
below 0x3ffe; `0x41189d` shifts the 96 bits left by 8 and `0x4118b3` right by that distance. Each
digit (`0x4118de`) is `(4v + v) * 2`, its top byte (`0x41190f`) taken and cleared. `0x411930`
compares the last digit with 0x35; `0x41194b` handles a carry out of the first; `0x411968` backs
over zeros, to `0x411978` when none is left. The count is the kept digits, a NUL after them.

## The digits a conversion reads

`_fltout2(value, flt, buf)` asks `I10_OUTPUT` for 17 digits with no flags of the double's 80-bit
real, copies the digits into `buf`, and fills the `_strflt` at `flt`: the sign as `-` or a space,
the decimal exponent (the value is `0.d1d2... * 10^decpt`), the return of `I10_OUTPUT`, and `buf`.
So `-0` is `0` with sign space, and a NaN's sign is kept. A signalling NaN never reaches it as
one: `_cftoe`, `_cftof` and `_cftog` copy the double with `fld` and `fstp`, and the x87 quiets a
signalling NaN on the load, so `1#SNAN` is never printed for a double.

Addresses: `0x40ff70`, `0x40ff80`, `0x40ff9b`, `0x40ffa6`, `0x40ffa9`, `0x40ffaf`, `0x40ffbb`, `0x40ffc3`, `0x40b560`, `0x40b683`, `0x40b781`
Confidence: verified
Proof: one
Test: `crt::crt_sprintf_float_difftest`

`0x40ff80` calls `___dtold` into `[ebp-0xc]`, and `0x40ff9b` passes those ten bytes, 0x11, 0 and
the `FOS` at `[ebp-0x28]`. `0x40ffa6` stores the return at `+8`, `0x40ffa9` the sign byte
`[ebp-0x26]` at `+0`, `0x40ffaf` the exponent word `[ebp-0x28]` at `+4`; `0x40ffbb` copies the
digits from `[ebp-0x24]` and `0x40ffc3` stores `buf` at `+0xc`. `0x40b560`, `0x40b683` and
`0x40b781` are each an `fld qword` of the argument and an `fstp qword`; a signalling NaN loaded
from memory raises the masked invalid exception and loads its quiet form (Intel SDM, `FLD`). The
emulator keeps it signalling (`METHOD#the-harness`), so the test passes none.
