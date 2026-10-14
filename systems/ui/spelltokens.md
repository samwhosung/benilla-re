# ui: Spell description tokens

How a description such as "Deals $s1 damage over $d" becomes a sentence: the expander that walks the text,
the syntax of a token, the field each token reads, the effect amounts with their rounding, the durations,
and the gender and plural choices. The spell and item tooltips and the buff tooltip all run their texts
through it; the quest text uses a sibling of the same file (`QuestTextParser.cpp`) with other tokens.

A token is a `$`, optional modifiers, a letter and optionally one digit that names an effect. The whole
expander works on one spell record and one output buffer, and keeps its place in the text in a global.

Roots: `0x5075f0`, `0x507710`, `0x508180`, `0x5082a0`

## The expander

`0x5075f0(spell record; buffer, size, pet, level, aura, stack, unmodified)` clears the buffer and copies the
text up to each `$`, expanding every token. The text is the aura description (`+0x24c`) when `aura` is set
and it is nonempty, else the description (`+0x228`). A level of 0 means the level the player has for the spell:
the player's value in the spell's skill line over five (`spell/records#caster-level`). A token the dispatcher
cannot expand leaves a literal `$` in the output and the text after the token is copied as it stands. The
return value is 1 when every token was expanded and 0 otherwise.

| caller | pet | level | aura | stack | unmodified |
|---|---|---|---|---|---|
| spell tooltip, item spell and set bonus lines | the pet flag, 0 for items | 0 | 0 | 1 | 0 |
| buff tooltip | 0 | the buff's | 1 | the stack, or 1 | 1 |

Addresses: `0x5075f0`, `0x5075ff`, `0x507614`, `0x507616`, `0x507622`, `0x50762d`, `0x507643`, `0x50764a`, `0x507662`, `0x507675`, `0x507680`, `0x507696`, `0x5076b4`, `0x5076b9`, `0x5076bd`, `0x5076c4`, `0x5076c9`, `0x5076da`, `0x5076ec`, `0x5076ef`, `0x5076fd`, `0x64a3c0`, `0x6e3130`, `0x84f2d8`, `0xbe0b80`, `0x52f955`, `0x52da24`
Confidence: verified
Proof: one
Test: none

The source is chosen at `0x507614` to `0x507629` and `SStrChr` (`0x64a3c0`) finds each `$` (`0x50762d`, `0x5076da`).
The default level is `0x6e3130(spell id, pet)` (`0x50764a`). The span before a token is appended
(`0x507680`) and the dispatcher `0x507710(spell, level, stack, pet, unmodified; buffer, size)` called
(`0x5076b4`); a zero return appends `$` (`0x84f2d8`, `0x5076c4`) and sets the failure flag (`0x5076c9`). The
return is `sete` of that flag (`0x5076fd`). The stack and unmodified arguments of the buff call are pushed at
`0x52f940` to `0x52f955`.

## The token syntax

Before the letter a token may carry a scale: `$/N;` divides the amounts it prints by `N`, `$*N;` multiplies them.
It may carry the id of another spell: `$1234s1` reads spell 1234, with the level cut to that spell's maximum
level when it has one. One digit after the letter, 1 to 9, picks the effect (1 to 3 give effects 0 to 2, 4 to 9
effect 0), and without a digit it is effect 0. A bad other-spell id and a letter that is not in the tables
below fail; so do `J K W Y j k w y [ \ ] ^ _` and the backquote.

Addresses: `0x507710`, `0x507720`, `0x507722`, `0x507734`, `0x50773a`, `0x507752`, `0x50776a`, `0x50777c`, `0x507784`, `0x50778a`, `0x5077a6`, `0x5077ae`, `0x5077d5`, `0x5077db`, `0x5077e2`, `0x5077e8`, `0x5077ff`, `0x507805`, `0x50780c`, `0x507817`, `0x50782f`, `0x50783a`, `0x507843`, `0x50784c`, `0x507850`, `0x507856`, `0x50785f`, `0x507866`, `0x5080ae`, `0x5080c1`, `0x5080d4`, `0x508138`, `0x5080e0`, `0x64ac60`
Confidence: verified
Proof: one
Test: `ui::glue4fd_507710_ceil_frac`, `ui::glue4fd_507710_f_case_quantize`, `ui::glue4fd_507710_f_case_scale`, `ui::glue4fd_507710_mos_prescale`, `ui::glue4fd_507710_mos_round_half`, `ui::glue4fd_507710_mos_snap`, `ui::glue4fd_507710_ot_per_tick`, `ui::glue4fd_507710_recip_scale`

The scale is read at `0x507720` to `0x507796`: `/` or `*`, a `;` found by `SStrChr`, at most eight characters
converted by `0x64ac60`, a result of 0 ignored, `/` storing the reciprocal against the float 1.0 at
`0x7ff9d8`. Leading digits are counted (`0x5077a6` to `0x5077b3`), converted (`0x5077db`) and the id checked
against `0xc0d78c` (`0x5077e8`); the other record's maximum level, at `+0x6c`, cuts the level (`0x507805` to
`0x507817`). The effect digit is read at `0x50782f` to `0x50784c`. The letter minus `'A'` is bounded by 0x39
(`0x507856`) and indexes the byte table `0x508138` (`0x50785f`) into the 22-entry jump table `0x5080e0`
(`0x507866`). After an arm the cursor moves past the letter, and past the digit if there was one (`0x5080ae`
to `0x5080c1`).

## Tokens that read a record field

Each letter reads one field of the record and prints it; both cases are the same arm except `f` and `F`.
The effect digit adds four bytes per effect to the offset.

| token | field | printed |
|---|---|---|
| `$a` | radius index at `+0x160` into the radius table, its `+4` float, modified (op 6) | integer |
| `$b` | float at `+0x1c0` | truncated integer |
| `$e` | float at `+0x184`, modified (op 27) | one decimal |
| `$f` | float at `+0x29c` times the scale | one decimal |
| `$F` | the same product | nearest integer, halves away from zero |
| `$h` | integer at `+0x64`, modified (op 18) | integer |
| `$i` | integer at `+0x28c` | integer |
| `$n` | integer at `+0x68`, modified (op 4) | integer |
| `$q` | integer at `+0x1a8` | integer |
| `$u` | integer at `+0x9c` | integer |
| `$v` | integer at `+0x27c` | integer |
| `$x` | integer at `+0x190`, modified (op 17) | integer |

Addresses: `0x507c74`, `0x507c86`, `0x507c8e`, `0x507c9b`, `0x507ca3`, `0x507cad`, `0x507cb6`, `0x507ed4`, `0x507edb`, `0x507ee0`, `0x507ee5`, `0x507f83`, `0x507f8a`, `0x507f94`, `0x507fa2`, `0x507ff7`, `0x507ffd`, `0x50800a`, `0x508025`, `0x50802f`, `0x50803c`, `0x508044`, `0x50804a`, `0x507e95`, `0x507e9b`, `0x507fc3`, `0x507eb5`, `0x507ebb`, `0x50806e`, `0x507ef1`, `0x507f4f`, `0x507dfc`, `0x507e03`, `0x6e6af0`, `0x6e6bf0`, `0x835154`, `0x84f398`, `0xc0d7b0`, `0xc0d7b4`
Confidence: verified
Proof: one
Test: none

The arms are at `0x507c74` (`$a`), `0x507ed4` (`$b`), `0x507f83` (`$e`), `0x507ff7` (`$f`), `0x508025` (`$F`),
`0x507e95` (`$h`), `0x507fc3` (`$i`), `0x507eb5` (`$n`), `0x50806e` (`$q`), `0x507ef1` (`$u`), `0x507f4f` (`$v`) and
`0x507dfc` (`$x`). The modifier calls take the op number in the push before them: `0x6e6af0` for the
integers (`0x507ca3`, `0x507e9b`, `0x507ebb`, `0x507e03`) and `0x6e6bf0` for `$e` (`0x507f8a`). `$b`
truncates with `__ftol` (`0x507edb`); `$F` adds 0.5 to a positive product and subtracts it otherwise
(`0x50803c`, `0x508044`) before `__ftol` (`0x50804a`). Integers print with `"%d"` (`0x835154`) and floats
with `"%.1f"` (`0x84f398`). The scale applies to `$f` and `$F` only. `$b`, `$h`, `$i`, `$n`, `$q`, `$u`,
`$v` and `$x` leave the plural key set to their value; `$a`, `$e`, `$f` and `$F` leave it alone.

## Tokens that compute a value

`$c` is the spell's power cost and `$p` its cost per second. `$d` is the duration: none reads
`SPELL_DURATION_UNTIL_CANCELLED`; a whole number of its largest unit (to 0.01) prints `INT_SPELL_DURATION`
with the integer count, and any other `SPELL_DURATION` with a one-decimal count. `$r` is the range
record's maximum, modified, with one decimal. `$t` is the effect's period in seconds, 5 when the word at
`+0x60` has bit 0, else the period at `+0x178`, modified (op 19). `$z` is the area name of the player's bind
point, or `HOME_INN`.

Addresses: `0x507dde`, `0x507de3`, `0x507ded`, `0x507df2`, `0x507cbb`, `0x507ccb`, `0x507cce`, `0x507ceb`, `0x507d00`, `0x507d06`, `0x507d19`, `0x507d1f`, `0x507d32`, `0x507d37`, `0x507d5b`, `0x507d78`, `0x507d86`, `0x507d9a`, `0x507da8`, `0x507db8`, `0x507e3c`, `0x507e42`, `0x507e4b`, `0x507e5c`, `0x507e61`, `0x507f07`, `0x507f1e`, `0x507f2b`, `0x507f3a`, `0x6e31b0`, `0x6e3300`, `0x6ea000`, `0x52f980`, `0x52fa50`, `0x52fbd0`, `0x5ed420`, `0xc0d79c`, `0xc0d7a0`, `0xc0e048`, `0xc0e04c`, `0x842cf4`, `0x84f310`, `0x84f330`, `0x84f340`
Confidence: verified
Proof: one
Test: `ui::time_band_remainder`, `ui::time_band_select`

`$c` calls `0x6e31b0` (`0x507de3`) and `$p` `0x6e3300` (`0x507df2`). `$d` asks `0x6ea000(spell, pet, unmodified)`
(`0x507ccb`); a result of 0 or less takes the until-cancelled text (`0x507d32`). Otherwise the milliseconds go
to `0x52f980(ms, 0.01)` (`0x507ceb`), which divides by the largest of 86400000, 3600000, 60000 or 1000 that the value
reaches and tests the fraction against the second argument: whole takes `0x52fa50(…, INT_SPELL_DURATION, 0,
0)` (`0x507d06`), else `0x52fbd0` with `SPELL_DURATION` (`0x507d1f`). `$r` takes the range index at `+0x90`
(minimum 1), reads `0xc0d79c` and its row's `+8` (`0x507d78` to `0x507d90`), applies op 5 (`0x507d9a`) and sets the
plural key to the ceiling (`0x507dad`, `0x507dbd`). `$t` tests `[record+0x60]` bit 0 (`0x507e3c`) for the 5000
default (`0x507e42`), else applies op 19 (`0x507e5c`) to `+0x178` and divides by 1000 with a multiply
(`0x507e61`). `$z` asks `0x5ed420` (`0x507f07`) and reads the area name at `+0x2c` (`0x507f2b`), else
`HOME_INN` (`0x507f3a`).

## Effect amounts

`$s`, `$S`, `$m`, `$M`, `$o` and `$O` print one effect's amount. The minimum and maximum of the effect
formula (`spell/records#effect-amount-formula`) at the expander's level are each multiplied by the stack
count and, for `$o` and `$O`, by the duration over the effect's period (0 counts as 5000 ms, a negative
one gives 0), made positive and scaled. A value within 0.001 of a whole number is whole. `$m` and `$M` print
the minimum and maximum, an integer when whole, else one decimal. The others print one number when the
unscaled bounds are equal, else a range: `SPELL_POINTS_SPREAD_TEMPLATE` for `$S`, `INT_SPELL_POINTS_SPREAD_TEMPLATE`
for the rest.

Addresses: `0x5078b4`, `0x5078ca`, `0x5078cc`, `0x5078d1`, `0x5078dc`, `0x5078e1`, `0x5078e9`, `0x5078f1`, `0x5078fd`, `0x507906`, `0x50790a`, `0x50791a`, `0x507931`, `0x50793d`, `0x507944`, `0x507952`, `0x507955`, `0x507978`, `0x507980`, `0x507990`, `0x5079d2`, `0x5079da`, `0x5079e5`, `0x5079fb`, `0x507a17`, `0x507a2d`, `0x507a57`, `0x507aac`, `0x507ab1`, `0x507abc`, `0x507b06`, `0x507b10`, `0x507b28`, `0x507b50`, `0x507b58`, `0x507b7d`, `0x507b96`, `0x507b99`, `0x507ba1`, `0x507baf`, `0x507bc2`, `0x507bd4`, `0x507be7`, `0x507bfb`, `0x507c2a`, `0x507c3a`, `0x507c4a`, `0x6e3800`, `0x73ff3f`, `0x73fdf5`, `0x801360`, `0x84f2d4`, `0x84f354`, `0x84f374`, `0xbe0b84`
Confidence: verified
Proof: one
Test: none

The arm calls `0x6e3800(spell, effect, &min, &max, level, pet, unmodified)` (`0x5078cc`) and multiplies both
by the stack count (`0x5078d1` to `0x5078e4`). The over-time letters (`0x5078e9`, `0x5078ed`) take the period
at `+0x178` (`0x5078f1`), 5000 for 0 (`0x5078ff`) and zeroes for a negative one (`0x507944`), the duration
from `0x6ea000` (`0x50791a`) and scale both amounts by `duration / period` (`0x507926` to `0x50793d`). The
magnitudes are `fabs` times the scale (`0x507952` to `0x507965`). Integers come from `2x − 0.5` converted
with `fistp` and shifted right one (`0x507978` to `0x5079a1`), the same for the maximum (`0x5079ab` to
`0x5079d2`). The whole test is `x − floor(x)` below the 0.001 at `0x801360` (`0x5079e5`), else
`ceil(x) − x` below it (`0x5079fb`); a whole value near the integer above takes it (`0x507a2d`,
`0x507a57`). `$m` is the arm at `0x507ab1` and `$M` at `0x507b50`. The equality test is `0x507b96` and
`0x507b99`; equal values print `"%d"` of the minimum when it is whole or the letter is not `S`
(`0x507b28`, `0x507ba1`), else one decimal (`0x507baf`). Unequal values take the integer spread when both are
whole (`0x507bd4`), `SPELL_POINTS_SPREAD_TEMPLATE` with two doubles for `$S` (`0x507bfb`), and otherwise
the integer spread with the minimum truncated and the maximum truncated plus one (`0x507c2a` to `0x507c4a`).

## The plural key

One global, `0xbe0b84`, holds the number the last counted token printed; `$l` reads it. It is written by `$b`,
`$h`, `$i`, `$n`, `$q`, `$r`, `$u`, `$v`, `$x` and the amount letters (for `$m` the minimum when whole, for the
others the maximum when whole), and set to 2 by an amount that is not whole. Nothing resets it, so a `$l`
before any such token reads what the previous expansion left, 0 at the start.

Addresses: `0xbe0b84`, `0x507abc`, `0x507b08`, `0x507b10`, `0x507dbd`, `0x507e21`, `0x507eaa`, `0x507eca`, `0x507ee0`, `0x507ef7`, `0x507f55`, `0x507fc9`, `0x508075`, `0x507897`, `0x5078a3`
Confidence: verified
Proof: one
Test: none

The stores are the instructions listed; `0x507897` pushes the key for `0x5082a0`. The expander never writes
it. `$a`, `$e`, `$f`, `$F`, `$c`, `$d`, `$p`, `$t` and `$z` do not.

## Gendered text

`$gMale:Female;` picks the first form for a male and the second otherwise, using the active player's
gender (`fields+0x7a`), not the described unit's. With no active player the token fails and the output
holds a literal `$` followed by the text. Each form loses its leading and trailing spaces and keeps interior
ones. A token without a colon, or without a `;` after it, prints nothing and the text after the letter is copied
as it stands.

Addresses: `0x50786d`, `0x507877`, `0x50787c`, `0x508180`, `0x508193`, `0x5081a4`, `0x5081b1`, `0x5081c2`, `0x5081c8`, `0x5081ed`, `0x5081f6`, `0x508200`, `0x50820c`, `0x50820e`, `0x508214`, `0x508219`, `0x508222`, `0x50823d`, `0x508257`, `0x508263`, `0x508267`, `0x508285`, `0x50828c`, `0x64a760`, `0x64a6f0`
Confidence: verified
Proof: one
Test: none

`0x508193` and `0x5081a4` find the active player with mask 0x10; none returns 0 (`0x5081b1`), and the dispatcher
returns it (`0x507883`). The colon and semicolon are searched for with `SStrChr` to the end of the text
(`0x5081ed`, `0x508200`). The byte at `fields+0x7a` (`0x508214`) of 0 keeps the first form (`0x50821b`); anything else,
2 included, takes the second (`0x508222`). The form is appended whole (`0x508257`) and the trailing spaces trimmed
(`0x508267` to `0x508280`); the cursor ends after the `;` (`0x508285`).

## Plural text

`$lSingular:Plural;` has the same shape and no player. It asks the interface's plural function for the
plural index of the key and takes the first form when the index is 1, the second otherwise, so the first form is
used only for a key of 1: a key of 0 reads plural. It cannot fail.

Addresses: `0x50788d`, `0x507897`, `0x5082a0`, `0x5082b8`, `0x5082c0`, `0x5082e5`, `0x5082f6`, `0x50830c`, `0x50830f`, `0x508314`, `0x508316`, `0x50831f`, `0x508343`, `0x508357`, `0x508367`, `0x508385`, `0x50838c`, `0x703cf0`
Confidence: verified
Proof: one
Test: none

`0x50830f` calls `0x703cf0(key)`, which runs the Lua function `GetPluralIndex` and returns its result minus
one, or 0 when the call fails (`ui/script#text-counts-and-running-code`). A zero (`0x508314`) takes the first form
(`0x508318`), a nonzero one the second (`0x50831f`). Every exit returns 1 (`0x5082d3`, `0x50838e`).
