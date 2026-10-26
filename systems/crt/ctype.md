# crt: characters

The C locale's character classes and case: the type table `_ctype`, the class tests that read it,
`toupper`'s fold, and the query for a wide character's type. Nothing calls `_setlocale`
(`net/who`), so the table, `_pctype` and `__mb_cur_max` keep their file values and the locale
word `0x883b08` stays 0 (`storm/format`); each function below runs its C locale path.

Roots: `0x73fb91`, `0x41089b`

## Character classes

`_ctype` holds 16 bits for EOF and each byte, `_pctype` pointing at the entry for 0: controls
0x20, 9 to 13 0x28, the space 0x48, the digits 0x84, `A` to `F` 0x81, `G` to `Z` 0x01,
`a` to `f` 0x82, `g` to `z` 0x02, other printable bytes 0x10, EOF and 0x80 to 0xff 0.
`isalpha(c)` answers `_pctype[c] & 0x103`, nonzero for `A` to `Z` and `a` to `z`. Nothing bounds
`c`: a negative one reads the words before the table. `IsAllowedLaunchUrl` passes each byte
sign-extended, so from their file values the bytes 0xee, 0xef, 0xf5, 0xf7, 0xf9 and 0xfb to 0xfe
pass as letters, 0xed to 0xf0 read two words code writes, and no byte 0x80 to 0xbf passes.

Addresses: `0x831724`, `0x83171c`, `0x831710`, `0x73fb91`, `0x73fbab`, `0x73fbb5`, `0x73fbb9`, `0x5abc10`, `0x5abc60`
Confidence: verified
Proof: one
Test: `crt::crt_isalpha_difftest`

`isalpha` (`0x73fb91`) takes `_isctype` only while `__mb_cur_max` (`0x831710`) is above 1;
`0x73fbab` loads `_pctype` (`0x83171c`), `mov ax,[ecx+eax*2]` (`0x73fbb5`) and `and eax,0x103`
(`0x73fbb9`). The table's runs are its 514 bytes at `0x831724`. `0x5abc10` pushes
`movsx eax,al` (`0x5abc60`). A byte `b` from 0x80 reads the word at `0x831726 + 2·(b − 0x100)`,
from 0x831626 to 0x831724. Of those words the listing's absolute operands write the dwords at
0x831700 and 0x831704, read by bytes 0xed to 0xf0, and the ones the category initializers
`___init_numeric` (`0x411eff`: `0x411fff`, `0x412005`, `0x4120bf`, `0x4120c9`) and `0x41237d`
(`0x412454`, `0x41251d`, `0x412526`, `0x412595`, `0x41259a`) write, both unreached, so
`__mb_cur_max`, `__decimal_point`, `_pctype`, `_pwctype` and the rest keep their file values: 0xfb reads `_pctype`'s low word
0x1726 and 0xfd `_pwctype`'s.

## Upper case

`toupper(c)` answers `c − 0x20` for `a` to `z` and `c` for every other `int`, while the locale word
is 0. With it set it would take the locale lock and call `0x41090a`, which is not read here.

Addresses: `0x41089b`, `0x41089e`, `0x4108aa`, `0x4108b4`, `0x4108b9`, `0x883b08`
Confidence: verified
Proof: one
Test: `crt::crt_toupper_difftest`

`cmp [0x883b08],ebx; jne` (`0x41089e`) sends a set word to `0x4108b9`; `0x4108aa` to `0x4108b4`
compare `c` with 0x61 and 0x7a, signed, and subtract 0x20. `storm/format` names the word's one
writer, reached only from `_setlocale`.

## Wide character types

`___crtGetStringTypeW(type, src, n, out, code_page, lcid)` decides on its first call whether the
system has `GetStringTypeW`, by calling it on a one-unit empty string. When it has, each call is
`GetStringTypeW(type, src, n, out)`. When it has not and `GetStringTypeA` answers, it converts
the units to bytes in the code page, the runtime's by default, asks `GetStringTypeA` for the
locale, the locale word by default, and copies the `n` answers out only when the call wrote
exactly `n`; otherwise, or when neither call works, it answers 0. `_iswctype` asks it for one
unit's `CT_CTYPE1` bits.

Addresses: `0x413ed3`, `0x413ef9`, `0x413f12`, `0x413f1c`, `0x413f2c`, `0x413f3a`, `0x413f53`, `0x413f86`, `0x413fef`, `0x414044`, `0x41405a`, `0x414063`, `0x414077`, `0x414084`, `0x883ba4`, `0x8824a0`, `0x7458e4`, `0x745918`
Confidence: verified
Proof: one
Test: none

`0x413ef9` reads the mode at `0x883ba4`, 0 in the file. The probe `0x413f12` calls through
`0x8824a0` with `(1, "", 1, &type)`; success stores mode 1 (`0x413f1c`), else `GetStringTypeA`
is probed (`0x413f2c`) and stores mode 2 (`0x413f3a`) or answers 0. Mode 1 forwards the four
arguments (`0x413f53`). Mode 2 sizes the bytes with `WideCharToMultiByte` and flags 0x220
(`0x413f86`), converts into a zeroed stack buffer (`0x413fef`), marks the words at `n − 1` and `n`
of a second buffer 0xffff (`0x414044`), calls `GetStringTypeA` (`0x41405a`), and copies `2n` bytes
(`0x414077`) only when the first mark was overwritten and the second was not (`0x414063`).
`_iswctype` (`0x7458e4`) calls it at `0x745918` with type 1 and a count of 1.
