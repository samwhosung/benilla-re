# lua: the libraries

The C functions of the libraries `luaopen_math`, `luaopen_table`, `luaopen_string` and
`luaopen_bit` register (`ui/luastate`), laid out between `0x7faa50` and `0x7fd6d0`. The math,
table and string libraries are the Lua 5.0 ones, `lmathlib.c`, `ltablib.c` and `lstrlib.c`, except
`string.format`, which is Lua 4.0's; the bit library is the client's own. A C function takes the state in `ecx` and answers its result count in
`eax`. The registration tables pair a name with each function: `bitlib` (`0x822c18`), `mathlib`
(`0x822c60`), `tab_funcs` (`0x822d40`) and `strlib` (`0x822d88`). The names are upstream's, except
the bit library's, which follow the same `<library>_<name>` rule, and the client's own helpers,
which are named for what they do. The client's `string.lower` and `string.upper` work on UTF-8.

Roots: `0x7faa50`, `0x7faa90`, `0x7fab10`, `0x7fab90`, `0x7fac10`, `0x7fac70`, `0x7facd0`, `0x7fad20`, `0x7fade0`, `0x7fae10`, `0x7fae40`, `0x7fae70`, `0x7faea0`, `0x7faed0`, `0x7faf00`, `0x7faf30`, `0x7fafa0`, `0x7fafd0`, `0x7fb020`, `0x7fb050`, `0x7fb0a0`, `0x7fb0d0`, `0x7fb100`, `0x7fb140`, `0x7fb170`, `0x7fb1a0`, `0x7fb1f0`, `0x7fb230`, `0x7fb2a0`, `0x7fb4e0`, `0x7fb590`, `0x7fb630`, `0x7fb670`, `0x7fb6b0`, `0x7fb770`, `0x7fb830`, `0x7fb920`, `0x7fbcf0`, `0x7fbd30`, `0x7fbdf0`, `0x7fc0c0`, `0x7fc1a0`, `0x7fc210`, `0x7fc280`, `0x7fc330`, `0x7fc3b0`, `0x7fcfa0`, `0x7fd0e0`, `0x7fd3d0`

## Bit library

The client's `bit` library works on the low 32 bits of `__ftol`, a truncation toward zero to a
64-bit integer (`eax` is read, `edx` ignored).
`bnot` inverts, `band`, `bor` and `bxor` combine every argument, `lshift`, `rshift` and `arshift`
shift argument 1 by the count in argument 2 (masked to five bits), `mod` takes the remainder of
argument 1 by argument 2. The results are pushed as unsigned numbers, except `arshift` and the
remainder, which are signed. `mod` tests only the low dword of the divisor, so 0, 0.5 and 2^32 all answer 1
divided by argument 2.

| function | name | decided by |
|---|---|---|
| `0x7faa50` | `bit_bnot` | `__ftol` (`0x7faa63`), `not eax` (`0x7faa68`), high word zeroed (`0x7faa6d`), `fild qword` |
| `0x7faa90` | `bit_band` | loop over `lua_gettop` arguments (`0x7faac0`), `and edx,eax` (`0x7faad1`), high word 0 (`0x7faae1`) |
| `0x7fab10` | `bit_bor` | the same loop with `or` |
| `0x7fab90` | `bit_bxor` | the same loop with `xor` |
| `0x7fac10` | `bit_lshift` | count from argument 2 (`0x7fac18`), value from argument 1 (`0x7fac29`), `shl eax,cl` (`0x7fac3e`), high word 0 (`0x7fac40`) |
| `0x7fac70` | `bit_rshift` | the same with `shr eax,cl` (`0x7fac9e`) and high word 0 (`0x7faca0`) |
| `0x7facd0` | `bit_arshift` | the same with `sar eax,cl` (`0x7facfc`), pushed as a signed integer by `fild dword` (`0x7fad06`) |
| `0x7fad20` | `bit_mod` | divisor from `__ftol` (`0x7fad2d`) tested (`0x7fad32`); zero divides `1.0` by `lua_tonumber` (`0x7fad42`) |
| `0x7fad70` | `bit_mod_remainder` | `cdq; idiv edi` (`0x7fad9a`, `0x7fad9b`), remainder `edx` pushed (`0x7fadab`) |

Addresses: `0x7faa50`, `0x7faa63`, `0x7faa68`, `0x7faa6d`, `0x7faa90`, `0x7faac0`, `0x7faad1`, `0x7faae1`, `0x7fab10`, `0x7fab90`, `0x7fac10`, `0x7fac18`, `0x7fac29`, `0x7fac3e`, `0x7fac40`, `0x7fac70`, `0x7fac9e`, `0x7faca0`, `0x7facd0`, `0x7facfc`, `0x7fad06`, `0x7fad20`, `0x7fad2d`, `0x7fad32`, `0x7fad42`, `0x7fad70`, `0x7fad9a`, `0x7fad9b`, `0x7fadab`, `0x822c18`, `0x7fadc0`, `0x7fad5d`
Confidence: verified
Proof: one
Test: none

The names come from this client's own evidence: `bitlib` (`0x822c18`) pairs the strings `bnot`,
`band`, `bor`, `bxor`, `lshift`, `rshift`, `arshift` and `mod` with these addresses, and
`luaopen_bit` (`0x7fadc0`) registers the table as the library `bit`. They take the form upstream
gives `mathlib`'s functions, `math_abs` for `abs`: `bit_bnot` for `bnot`. Each function reads its arguments
with `luaL_checknumber`. A shift count goes through `cl`, which the processor masks to five bits.
One reference reaches `bit_mod_remainder`, the `jmp` at `0x7fad5d`. `bit.mod(7, 0.5)` pushes 2.
`bit.mod(-2^31, -1)` raises a divide error in `idiv` (`0x7fad9b`).

## Math functions

Twelve functions of the math library wrap one operation each: `abs` (`fabs`), `sin`, `cos` and
`tan` (`fsin`, `fcos`, `fptan`, in radians), `atan` and `atan2` (`fpatan`), `asin` and `acos` (the
C runtime), `floor`, `mod` (`fmod`), `sqrt` (`fsqrt`) and `pow`. Each reads its arguments with
`luaL_checknumber`, pushes one number and answers 1.

| function | name | decided by |
|---|---|---|
| `0x7fade0` | `math_abs` | `fabs` (`0x7faded`) |
| `0x7fae10` | `math_sin` | `fsin` (`0x7fae1d`) |
| `0x7fae40` | `math_cos` | `fcos` (`0x7fae4d`) |
| `0x7fae70` | `math_tan` | `fptan` (`0x7fae7d`), the extra 1.0 popped (`0x7fae84`) |
| `0x7faea0` | `math_asin` | the C runtime routine (`0x7faead`) |
| `0x7faed0` | `math_acos` | the C runtime routine (`0x7faedd`) |
| `0x7faf00` | `math_atan` | `fld1; fpatan` (`0x7faf0d`, `0x7faf0f`) |
| `0x7faf30` | `math_atan2` | two arguments, `fpatan` (`0x7faf5a`) |
| `0x7fafa0` | `math_floor` | C runtime `floor` (`0x7fafb3`) |
| `0x7fafd0` | `math_mod` | C runtime `fmod` (`0x7faff7`) |
| `0x7fb020` | `math_sqrt` | `fsqrt` (`0x7fb02d`) |
| `0x7fb050` | `math_pow` | C runtime `pow` (`0x7fb077`) |

Addresses: `0x7fade0`, `0x7faded`, `0x7fae10`, `0x7fae1d`, `0x7fae40`, `0x7fae4d`, `0x7fae70`, `0x7fae7d`, `0x7fae84`, `0x7faea0`, `0x7faead`, `0x7faed0`, `0x7faedd`, `0x7faf00`, `0x7faf0d`, `0x7faf0f`, `0x7faf30`, `0x7faf5a`, `0x7fafa0`, `0x7fafb3`, `0x7fafd0`, `0x7faff7`, `0x7fb020`, `0x7fb02d`, `0x7fb050`, `0x7fb077`, `0x822c60`
Confidence: verified
Proof: one
Test: none

`mathlib` (`0x822c60`) pairs each of these with its name; `ceil` is `math_ceil` (`ui/luastate`). The
trigonometric functions take radians: no degree factor appears in `0x7fae10` to `0x7faf30`.

## More math functions

Nine more functions: `log` and `log10` multiply the base-2 logarithm by ln 2 and by
log10 2, `exp` computes 2 to the power x times log2 e with `f2xm1` and `fscale`, `deg` and `rad`
multiply by 180/pi and pi/180, `frexp` answers a mantissa and an exponent, `ldexp` scales by a power
of two, and `min` and `max` compare every argument with the running extreme.

| function | name | decided by |
|---|---|---|
| `0x7fb0a0` | `math_log` | `fldln2; fyl2x` (`0x7fb0ad`, `0x7fb0b6`) |
| `0x7fb0d0` | `math_log10` | `fldlg2; fyl2x` (`0x7fb0dd`, `0x7fb0e6`) |
| `0x7fb100` | `math_exp` | `fldl2e`, `frndint`, `f2xm1`, `fscale` (`0x7fb10d`, `0x7fb118`, `0x7fb11e`, `0x7fb124`) |
| `0x7fb140` | `math_deg` | `fmul` by 57.29577951308232 (`0x7fb14d`) |
| `0x7fb170` | `math_rad` | `fmul` by 0.017453292519943295 (`0x7fb17d`) |
| `0x7fb1a0` | `math_frexp` | C runtime `frexp` (`0x7fb1bb`), two `lua_pushnumber` (`0x7fb1c8`, `0x7fb1d8`) |
| `0x7fb1f0` | `math_ldexp` | exponent through `__ftol` (`0x7fb1fd`), C runtime `ldexp` (`0x7fb215`) |
| `0x7fb230` | `math_min` | `lua_gettop` (`0x7fb23b`), `luaL_checknumber` per argument, compare (`0x7fb26c`) |
| `0x7fb2a0` | `math_max` | the same loop with the opposite compare |

Addresses: `0x7fb0a0`, `0x7fb0ad`, `0x7fb0b6`, `0x7fb0d0`, `0x7fb0dd`, `0x7fb0e6`, `0x7fb100`, `0x7fb10d`, `0x7fb118`, `0x7fb11e`, `0x7fb124`, `0x7fb140`, `0x7fb14d`, `0x7fb170`, `0x7fb17d`, `0x7fb1a0`, `0x7fb1bb`, `0x7fb1c8`, `0x7fb1d8`, `0x7fb1f0`, `0x7fb1fd`, `0x7fb215`, `0x7fb230`, `0x7fb23b`, `0x7fb26c`, `0x7fb2a0`, `0x822d28`, `0x822d30`
Confidence: verified
Proof: one
Test: none

The factors for `deg` and `rad` are the doubles at `0x822d28` and `0x822d30`. `math_log` is the
natural logarithm because the multiplier loaded is ln 2, not 1. `math_exp` uses the fractional
split `x*log2e = n + f`, `2^f` from `f2xm1` plus 1, scaled by `2^n`. `math_random` and
`math_randomseed` are in `random`.

## Table library

Seven of the table functions use the length helpers `luaL_getn` and `luaL_setn`. `foreachi`
calls a function on each index up to the length and `foreach` on each key and value, both ending at
the first non-nil result. `getn` and `setn` read and set the length, `tinsert` shifts up and stores,
`tremove` shifts down and answers the removed value, and `concat` joins strings with a separator.
`sort` runs `auxsort`, a quicksort over `lua_rawgeti` and `lua_rawseti` that compares through
`sort_comp` and swaps with `set2`.

| function | name | decided by |
|---|---|---|
| `0x7fb4e0` | `luaB_foreachi` | `luaL_checktype` twice, `luaL_getn` (`0x7fb4fc`), `lua_rawgeti`, `lua_call` (`0x7fb552`) |
| `0x7fb590` | `luaB_foreach` | `lua_pushnil`, `lua_next` (`0x7fb5bb`, `0x7fb617`), `lua_call` of three pushes (`0x7fb5f1`) |
| `0x7fb630` | `luaB_getn` | `luaL_getn` (`0x7fb64a`), `lua_pushnumber` (`0x7fb65d`) |
| `0x7fb670` | `luaB_setn` | `luaL_checknumber`, `__ftol`, `luaL_setn` (`0x7fb698`) |
| `0x7fb6b0` | `luaB_tinsert` | `luaL_getn` (`0x7fb6d8`), the argument count test (`0x7fb6e0`), `luaL_setn` (`0x7fb711`), shift loop (`0x7fb728`) |
| `0x7fb770` | `luaB_tremove` | `luaL_getn` (`0x7fb78c`), `luaL_optnumber` (`0x7fb7a6`), `luaL_setn` (`0x7fb7ca`), nil stored (`0x7fb80a`) |
| `0x7fb830` | `str_concat` | `luaL_optlstring` (`0x7fb84c`), `luaL_buffinit`, `table contains non-strings` (`0x7fb8d1`), `luaL_pushresult` (`0x7fb90e`) |
| `0x7fb920` | `luaB_sort` | `luaL_getn` (`0x7fb937`), `luaL_checkstack` (`0x7fb94a`), `auxsort` (`0x7fb981`) |
| `0x7fb990` | `auxsort` | three `sort_comp` and `set2` pairs (`0x7fb9d4`, `0x7fba30`, `0x7fba66`), the partition loop, `invalid order function for sorting` |
| `0x7fbc20` | `set2` | two `lua_rawseti` (`0x7fbc2c`, `0x7fbc3c`) |
| `0x7fbc50` | `sort_comp` | `lua_type` of the function (`0x7fbc5e`), `lua_call` (`0x7fbc93`), `lua_toboolean` (`0x7fbc9d`), `lua_lessthan` (`0x7fbcc0`) |

Addresses: `0x7fb4e0`, `0x7fb4fc`, `0x7fb552`, `0x7fb590`, `0x7fb5bb`, `0x7fb617`, `0x7fb5f1`, `0x7fb630`, `0x7fb64a`, `0x7fb65d`, `0x7fb670`, `0x7fb698`, `0x7fb6b0`, `0x7fb6d8`, `0x7fb6e0`, `0x7fb711`, `0x7fb728`, `0x7fb770`, `0x7fb78c`, `0x7fb7a6`, `0x7fb7ca`, `0x7fb80a`, `0x7fb830`, `0x7fb84c`, `0x7fb8d1`, `0x7fb90e`, `0x7fb920`, `0x7fb937`, `0x7fb94a`, `0x7fb981`, `0x7fb990`, `0x7fb9d4`, `0x7fba30`, `0x7fba66`, `0x7fbc20`, `0x7fbc2c`, `0x7fbc3c`, `0x7fbc50`, `0x7fbc5e`, `0x7fbc93`, `0x7fbc9d`, `0x7fbcc0`, `0x822d40`, `0x7fbafc`
Confidence: verified
Proof: one
Test: none

`tab_funcs` pairs `concat`, `foreach`, `foreachi`, `getn`, `setn`, `sort`, `insert` and `remove` with
these (`0x822d40`). The names are those of `ltablib.c`. `str_concat` carries the `str_` prefix there
too. `auxsort` raises `invalid order function for sorting` when a scan passes the range end
(`0x7fbafc`).

## String basics

Six string functions are plain: `len`, `sub` (indices from either end through `posrelat`, the
last index defaulting to -1), `rep` (a count of copies), `byte` (the code at one position,
default 1, one result), `char` (a string from codes, `invalid value` above 255) and `dump` (a Lua
function with no upvalues as a string through `lua_dump`). `posrelat` turns a negative position
into one counted from the end, and `writer` is the callback `lua_dump` calls.

| function | name | decided by |
|---|---|---|
| `0x7fbcf0` | `str_len` | `luaL_checklstring` (`0x7fbd02`), the length pushed (`0x7fbd1f`) |
| `0x7fbd30` | `str_sub` | `posrelat` twice (`0x7fbd5f`, `0x7fbd83`), default -1 for the end, `lua_pushlstring` (`0x7fbda9`) |
| `0x7fbde0` | `posrelat` | a negative position gets `len + 1` added; the body is 11 bytes (`0x7fbde0`) |
| `0x7fc1a0` | `str_rep` | `luaL_buffinit` (`0x7fc1d9`), `luaL_addlstring` per copy (`0x7fc1ee`), `luaL_pushresult` (`0x7fc1fc`) |
| `0x7fc210` | `str_byte` | `luaL_optnumber` (`0x7fc238`), `posrelat` (`0x7fc247`), one `lua_pushnumber` (`0x7fc267`) |
| `0x7fc280` | `str_char` | `lua_gettop` loop, value tested against 255, `invalid value` (`0x7fc2c8`), `luaL_argerror` (`0x7fc2d1`) |
| `0x7fc330` | `str_dump` | `luaL_checktype` 6 (`0x7fc343`), `lua_dump` (`0x7fc363`), `unable to dump given function` (`0x7fc36c`) |
| `0x7fc390` | `writer` | `luaL_addlstring` (`0x7fc39a`) |

Addresses: `0x7fbcf0`, `0x7fbd02`, `0x7fbd1f`, `0x7fbd30`, `0x7fbd5f`, `0x7fbd83`, `0x7fbda9`, `0x7fbde0`, `0x7fc1a0`, `0x7fc1d9`, `0x7fc1ee`, `0x7fc1fc`, `0x7fc210`, `0x7fc238`, `0x7fc247`, `0x7fc267`, `0x7fc280`, `0x7fc2c8`, `0x7fc2d1`, `0x7fc330`, `0x7fc343`, `0x7fc363`, `0x7fc36c`, `0x7fc390`, `0x7fc39a`, `0x822d88`
Confidence: verified
Proof: one
Test: none

`strlib` pairs `len`, `sub`, `lower`, `upper`, `char`, `rep`, `byte`, `format`, `dump`, `find`,
`gfind` and `gsub` with these and the next entries (`0x822d88`). `posrelat` is
`pos >= 0 ? pos : len + pos + 1`, as in upstream, with no clamp: `str_sub` and `str_byte` apply
their own bounds.

## Case mapping

`str_lower` and `str_upper` walk a string by UTF-8 code points: `Utf8DecodeChar` reads one
and answers its length, `LowerCodepoint` or `UpperCodepoint` maps it, `Utf8EncodeChar` writes it
back. Lowering maps `A` to `Z` and `0xc0` to `0xde` up by 32, raising maps `a` to `z` and `0xe0` to
`0xfe` down by 32. A byte that is not valid UTF-8 decodes to `0x80000000`, which encodes to no
bytes, so it is dropped. A NUL byte decodes to -1 and consumes nothing, and the call never returns.

| function | name | decided by |
|---|---|---|
| `0x7fbdf0` | `str_lower` | `Utf8DecodeChar` (`0x7fbe2d`), `LowerCodepoint` (`0x7fbe34`), `Utf8EncodeChar` (`0x7fbe3e`), advance by the length (`0x7fbe8a`) |
| `0x7fc0c0` | `str_upper` | the same loop with `UpperCodepoint` (`0x7fc104`) |
| `0x7fbeb0` | `Utf8DecodeChar` | lead-byte masks `0xfc`, `0xf8`, `0xf0`, `0xe0`, `0xc0` (`0x7fbef2`..`0x7fbf4e`), continuation test `0xc0 == 0x80` (`0x7fbf7c`) |
| `0x7fbfb0` | `Utf8EncodeChar` | limits 128, 2048, 65536, 2097152, 4194304 and 2147483648 (`0x7fbfb6`..`0x7fc05a`) |
| `0x7fc0a0` | `LowerCodepoint` | ranges `0x41`..`0x5a` and `0xc0`..`0xde` (`0x7fc0a2`..`0x7fc0b3`), plus 32 |
| `0x7fc180` | `UpperCodepoint` | ranges `0x61`..`0x7a` and `0xe0`..`0xfe` (`0x7fc182`..`0x7fc193`), minus 32 |

Addresses: `0x7fbdf0`, `0x7fbe2d`, `0x7fbe34`, `0x7fbe3e`, `0x7fbe8a`, `0x7fc0c0`, `0x7fc104`, `0x7fbeb0`, `0x7fbef2`, `0x7fbf4e`, `0x7fbf7c`, `0x7fbfb0`, `0x7fbfb6`, `0x7fc05a`, `0x7fc0a0`, `0x7fc0a2`, `0x7fc0b3`, `0x7fc180`, `0x7fc182`, `0x7fc193`, `0x7fbebb`, `0x7fbeda`, `0x7fbee8`, `0x7fc15a`, `0x7fbfa4`, `0x7fbf75`
Confidence: verified
Proof: one
Test: none

Stock `str_lower` calls C `tolower` per byte. Here the decoder starts with the count at 0
(`0x7fbebb`) and a byte of 0 returns -1 at `0x7fbeda` before the count is raised
(`0x7fbee8`), so a NUL inside the length leaves the advance `ebx += count` of `str_lower`
(`0x7fbe8a`) and of `str_upper` (`0x7fc15a`) at 0, and the loop re-reads the NUL for ever: the call
never returns. A lone continuation byte `0x80` to `0xbf` takes the path to `0x7fbfa4`, returning
`0x80000000`. After a lead byte, each following byte is counted (`0x7fbf75`) before the continuation
test (`0x7fbf7c`), so an invalid one is consumed with the sequence: `"\xC9a"` loses both bytes.
`str_upper` is the same code with `UpperCodepoint` (the call at `0x7fc104`).

## Pattern classes

The pattern matcher's character tests. `luaI_classend` finds the end of a single-character
class in a pattern, raising on a trailing `%` or a missing `]`. `luaI_singlematch` tests one
character against a class: `.` matches any, `%x` goes to `match_class`, `[...]` to
`matchbracketclass`, anything else is compared. `match_class` handles `a c d l p s u w x z` and their
capital negations by C runtime tests, `matchbracketclass` ranges and negation, and `matchbalance`
the `%b` pair.

| function | name | decided by |
|---|---|---|
| `0x7fc820` | `luaI_classend` | `%` at `0x7fc827`, `[` at `0x7fc82f`, the errors ``malformed pattern (ends with `%')`` (`0x7fc87d`) and ``(missing `]')`` (`0x7fc84e`) |
| `0x7fc890` | `matchbracketclass` | leading `^` (`0x7fc898`), `%` calls `match_class` (`0x7fc8ce`), range `-` (`0x7fc8dc`) |
| `0x7fc930` | `match_class` | switch on the lowered letter, `_pctype` masks `0x103`, `0x20`, `0x4`, `0x2`, `0x10`, `0x8`, `0x1`, `0x107`, `0x80`; `z` tests 0 |
| `0x7fcb90` | `luaI_singlematch` | `%` (`0x7fcb95`) to `match_class` (`0x7fcbca`), `.` (`0x7fcb99`), `[` (`0x7fcb9d`) to `matchbracketclass` (`0x7fcbb4`) |
| `0x7fcbe0` | `matchbalance` | `unbalanced pattern` (`0x7fcbfc`) when the pattern lacks two characters, nesting count |

Addresses: `0x7fc820`, `0x7fc827`, `0x7fc82f`, `0x7fc87d`, `0x7fc84e`, `0x7fc890`, `0x7fc898`, `0x7fc8ce`, `0x7fc8dc`, `0x7fc930`, `0x7fcb90`, `0x7fcb95`, `0x7fcbca`, `0x7fcb99`, `0x7fcb9d`, `0x7fcbb4`, `0x7fcbe0`, `0x7fcbfc`, `0x7fc62e`, `0x7fc641`
Confidence: verified
Proof: one
Test: none

`match_class` tests the lowered class letter, then inverts the answer when the letter is a
capital (a final `_pctype` test for lower case). `0x7fc930` takes the character in `ecx` and the
class letter in `edx`. The `%f` frontier of `match` uses `matchbracketclass` twice (`0x7fc62e`, `0x7fc641`).

## Pattern matching

`match` tries a pattern at a position and answers the end of the match or null. It dispatches
on the pattern character: `(` and `)` to `start_capture` and `end_capture`, `%b` to
`matchbalance`, `%f` to a frontier test, `%1` to `%9` to `match_capture`, `$` at the end, and a
single-character class followed by `*`, `+`, `-` or `?` to `max_expand`, `min_expand` or a trial of
the rest. A pattern holds at most 32 captures. `check_capture` validates a `%n` index and
`capture_to_close` finds the open capture a `)` closes.

| function | name | decided by |
|---|---|---|
| `0x7fc570` | `match` | dispatch on the pattern character (`0x7fc584`), the `%b` and `%f` cases (`0x7fc59e`, `0x7fc5a6`) |
| `0x7fcc50` | `max_expand` | `luaI_singlematch` loop (`0x7fcc71`), `match` on the rest (`0x7fcc9a`) |
| `0x7fccc0` | `min_expand` | `match` first (`0x7fccd1`), then `luaI_singlematch` (`0x7fccef`) and `match` again (`0x7fccfe`) |
| `0x7fcd20` | `start_capture` | limit 32 (`0x7fcd2b`), `too many captures` (`0x7fcd35`), `match` (`0x7fcd5a`) |
| `0x7fcd70` | `end_capture` | `capture_to_close` (`0x7fcd7a`), `match` (`0x7fcd95`) |
| `0x7fcdb0` | `capture_to_close` | scans for a length of -1 (`0x7fcdc0`), `invalid pattern capture` (`0x7fcdd0`) |
| `0x7fcde0` | `match_capture` | `check_capture` (`0x7fcdec`), length compare and byte compare |
| `0x7fce20` | `check_capture` | index `n - 0x31` below the level (`0x7fce2c`), `invalid capture index` (`0x7fce36`) |

Addresses: `0x7fc570`, `0x7fc584`, `0x7fc59e`, `0x7fc5a6`, `0x7fcc50`, `0x7fcc71`, `0x7fcc9a`, `0x7fccc0`, `0x7fccd1`, `0x7fccef`, `0x7fccfe`, `0x7fcd20`, `0x7fcd2b`, `0x7fcd35`, `0x7fcd5a`, `0x7fcd70`, `0x7fcd7a`, `0x7fcd95`, `0x7fcdb0`, `0x7fcdc0`, `0x7fcdd0`, `0x7fcde0`, `0x7fcdec`, `0x7fce20`, `0x7fce2c`, `0x7fce36`, `0x7fcf4f`, `0x7fcf79`, `0x7fc6d1`
Confidence: verified
Proof: one
Test: none

A match state is `src_init` at `+0`, `src_end` at `+4`, the state at `+8`, the level at `+0xc`
and 32 pairs of start and length from `+0x10`; a length of -1 marks an open capture and -2 a
position capture (`0x7fcf4f`, `0x7fcf79`). `match` is recursive through its own address
(`0x7fc6d1`) and through the expanders.

## Search and replace

`str_find` searches for a pattern, or for a plain string when the fourth argument is true or the
pattern has none of the characters `^$*+?.([%-`, and answers the start, the end and the captures.
`gfind` returns an iterator closure over `gfind_aux`, which carries the subject, pattern and
next position in three upvalues. `str_gsub` replaces matches with a string or a function result
through `add_s` and answers the new string and the count. `push_captures` and `push_onecapture`
push the captures, `lmemfind` is the plain search.

| function | name | decided by |
|---|---|---|
| `0x7fc3b0` | `str_find` | `luaL_optnumber` init (`0x7fc3f2`), `lua_toboolean` plain (`0x7fc41c`), `strpbrk` of the special set (`0x7fc432`), `lmemfind` (`0x7fc515`) |
| `0x7fce50` | `lmemfind` | `memchr` of the first byte (`0x7fce99`), an empty needle returns the start |
| `0x7fcee0` | `push_captures` | `luaL_checkstack` `too many captures` (`0x7fcef4`), whole match by `lua_pushlstring` (`0x7fcf0f`), `push_onecapture` (`0x7fcf29`) |
| `0x7fcf40` | `push_onecapture` | length -1 raises `unfinished capture` (`0x7fcf57`), -2 pushes a position (`0x7fcf79`, `0x7fcf94`) |
| `0x7fcfa0` | `gfind` | two `luaL_checklstring`, `lua_settop` 2 (`0x7fcfc4`), 0 pushed, `lua_pushcclosure` with 3 upvalues (`0x7fcfdd`) |
| `0x7fcff0` | `gfind_aux` | upvalues read by `lua_tostring` and `lua_tonumber` (`0x7fd003`..`0x7fd045`), `match` (`0x7fd076`), `lua_replace` (`0x7fd0be`) |
| `0x7fd0e0` | `str_gsub` | maximum count `luaL_optnumber` (`0x7fd12f`), anchor `^` (`0x7fd13c`), `string or function expected` (`0x7fd189`), `add_s` (`0x7fd203`) |
| `0x7fd2a0` | `add_s` | `lua_isstring` (`0x7fd2b8`), `%` handling (`0x7fd2f0`), `check_capture`, `push_onecapture`, `lua_call` (`0x7fd38d`) |

Addresses: `0x7fc3b0`, `0x7fc3f2`, `0x7fc41c`, `0x7fc432`, `0x7fc515`, `0x7fce50`, `0x7fce99`, `0x7fcee0`, `0x7fcef4`, `0x7fcf0f`, `0x7fcf29`, `0x7fcf40`, `0x7fcf57`, `0x7fcf79`, `0x7fcf94`, `0x7fcfa0`, `0x7fcfc4`, `0x7fcfdd`, `0x7fcff0`, `0x7fd003`, `0x7fd045`, `0x7fd076`, `0x7fd0be`, `0x7fd0e0`, `0x7fd12f`, `0x7fd13c`, `0x7fd189`, `0x7fd203`, `0x7fd2a0`, `0x7fd2b8`, `0x7fd2f0`, `0x7fd38d`, `0x7fc42c`
Confidence: verified
Proof: one
Test: none

`str_gsub` reads its optional fourth argument as the maximum count, with the default one
past the subject's length (`srcl + 1`). `str_find` tests the pattern against the special characters
`^$*+?.([%-` (`0x7fc42c`). `add_s` serves both replacement kinds, as upstream's does.

## Formatting

`str_format` walks its format string, copies text and `%%`, and for each `%` conversion scans the
specification with the matcher and the pattern `[-+ #0]*(%d*)%.?(%d*)`, raising when a width or
precision has more than two digits. The conversion letter selects `sprintf` with a double
(`e E f g G`), an integer (`c d i o u x X`), the quoted form (`q`, `luaI_addquoted`), or a string
(`s`; long strings without precision go in whole). An argument may be numbered (`%n$`).

| function | name | decided by |
|---|---|---|
| `0x7fd3d0` | `str_format` | the pattern (`0x7fd4dd`), `match` (`0x7fd4fa`), width and precision tests against 2 (`0x7fd50c`, `0x7fd510`), spec length limit 18 (`0x7fd51c`), switch on the letter (`0x7fd54f`), `sprintf` (`0x7fd586`) |
| `0x7fd6d0` | `luaI_addquoted` | quote (`0x7fd716`), the escapes, the text `\000` through `luaL_addlstring` (`0x7fd75c`) |

Addresses: `0x7fd3d0`, `0x7fd4dd`, `0x7fd4fa`, `0x7fd50c`, `0x7fd510`, `0x7fd51c`, `0x7fd54f`, `0x7fd586`, `0x7fd6d0`, `0x7fd716`, `0x7fd75c`, `0x7fc570`
Confidence: verified
Proof: one
Test: none

The client's `str_format` is Lua 4.0's, not 5.0's hand scanner: `lstrlib.c` of 4.0.1 (lines 549 to
559) has the `n$` argument number, `src_end` at the format length plus 1, the same pattern, and the
limit `MAX_FORMAT-2`, 18. `0x7fd4fa` calls `0x7fc570` with the pattern string, and the two capture
lengths are compared with 2 (`0x7fd50c`, `0x7fd510`). A NUL in a `%q` argument is written as the four bytes
`\000`. Upstream's `scanformat` has no function of its own here.
