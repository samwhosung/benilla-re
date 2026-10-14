# font: markup and line breaks

The in-text codes the layout reads (`|c` colours, `|H` links and the rest), the function that
strips them, the line-break counter and the classifier that says where a line may break.

Roots: `0x5c7780`

## Markup tokens

`0x5c2810` reads one token at the cursor (text in `ecx`, `&length` in `edx`, an unused stack argument, the
flags and `&value`) and returns its class:

| class | token |
|---|---|
| 0 | `\|cAARRGGBB` colour |
| 1 | `\|r` restore |
| 2 | newline: CR, LF, CRLF or `\|n` |
| 3 | `\|\|`, an escaped pipe |
| 4 | `\|H` to the closing `\|h`, a link |
| 5 | `\|h` alone, a link end |
| 6 | a literal character, or a code that is malformed or gated off |

Four flag bits switch codes off: `0x100` the colour and restore codes, `0x200` the `|n` newline,
`0x400` the link codes, `0x800` every pipe code (a `|` then is a literal). The token's byte length is
written to `*length` and its character to `*value`. A colour has length 10 and its value is the
colour with `0xff` forced into the top byte (the first hex pair is discarded).

Addresses: `0x5c2810`, `0x5c284b`, `0x5c2871`, `0x5c2896`, `0x5c28af`, `0x5c28b8`, `0x5c28d4`, `0x5c28dd`, `0x5c28e4`, `0x5c28eb`, `0x5c29c8`, `0x5c29e7`, `0x5c2a06`, `0x5c2a22`, `0x5c2a36`, `0x5c2b10`, `0x5c2af4`, `0x41aab0`
Confidence: verified
Proof: one
Test: `font::decode_quoted_code_difftest`

`0x5c2810` decodes UTF-8 with `0x41aab0`, which returns −1 for a NUL lead, 0x80000000 for a
malformed sequence and counts the bytes it read. CR then LF is one token of length 2
(`0x5c2871` to `0x5c288b`); a lone CR or LF has length 1 (`0x5c289b`). For `|`, `0x5c28b8` returns a
literal when `0x800` is set; otherwise the next byte minus `0x43`, below `0x3a`, indexes the byte table
`0x5c2b10` and the case table `0x5c2af4`. `|C`/`|c` (`0x5c2a36`) needs eight hex digits, read as
four pairs with `strtoul` base 16; a NUL or a non-hex pair gives a literal. `|R`/`|r` is class 1,
`|N`/`|n` class 2, `||` class 3 and `|h` class 5, each of length 2. `|H` (`0x5c28eb`) scans to the next
`|`: `|h` there with a byte after it gives class 4 with the whole link as its length; an empty
link `|H|h` or a doubled close is a literal.

## Stripping

`0x5c2590(text, length, mask, out, capacity)` copies the text with the codes removed or
normalised and returns the number of bytes written. `mask` takes the four gate bits.

Addresses: `0x5c2590`, `0x5c25e3`, `0x5c25f4`, `0x5c26dc`, `0x5c25fb`, `0x5c263c`, `0x5c267b`, `0x85f4d8`
Confidence: verified
Proof: one
Test: `font::strip_control_codes_difftest`

An empty text or a zero length gives an empty result by `SStrPrintf`. Each token is read with
gates 0 (every code active). Classes 0, 1 and 4 are dropped when the mask has the class's bit
and otherwise copied as written. Classes 2, 3 and 5 are dropped when the mask has the bit and
otherwise replaced: class 2 by one `\n`, class 3 by a single `|`, class 5 by `|h`. Class 6 is copied. The
table `0x85f4d8` holds seven triples (mask, replacement character, prefix flag): (`0x100`, `C`, 1),
(`0x100`, `R`, 1), (`0x200`, `\n`, 0), (`0x800`, `|`, 0), (`0x400`, `H`, 1), (`0x400`, `h`, 1),
(0, `-`, 1); a prefix flag of 1 writes a `|` before the character. Output stops short of the capacity and is
NUL-terminated.

## Line starts

`0x5c2430(text, flags, size, width, offsets, count, ...)` fills an array with the byte offset
at which each line of a text starts and returns the number written.

Addresses: `0x5c2430`, `0x5c7260`, `0x5c2540`
Confidence: verified
Proof: one
Test: `font::line_break_enumerate_difftest`

The loop records the cursor, decodes a token and either (class 2) moves past it or asks the
line measure `0x5c7260` (see `measure`) how far one line goes. A step of 0 ends it. A text
that ends in a LF adds one more offset equal to its length. A zero size or box returns 0; with
flag bit 2 the font's default size (`0x5c2540`) replaces the size. The test covers the newline,
count and offset paths; the wrapped path calls the measure, which `measure` tests.

## Break opportunities

`0x5c7780(cur, next)` says whether a line may break between two characters (1 yes). It tests, in order:
`cur == 0` gives 0; `cur` of `-`, `:` or `/` gives 1; `next == 0xffffffff` (end) gives 1; a space in `cur`
gives 0; a space in `next` gives 1; `cur` in the opener set gives 0; `next` in the closer set gives 0;
`next` in the East Asian ranges gives 1; everything else 0.

Addresses: `0x5c7780`, `0x5c7785`, `0x5c778e`, `0x5c77b5`, `0x5c77c3`, `0x5c77d4`, `0x5c77e8`, `0x5c7814`, `0x5c7974`, `0x5c785f`, `0x74006f`
Confidence: verified
Proof: one
Test: `font::line_break_class_ascii_difftest`, `font::line_break_class_cjk_difftest`

A space is the C locale set `0x09` to `0x0d` and `0x20` for values below 0x100 (`0x74006f` masks the
value to 16 bits and reads the CRT's ctype table); above that it asks the locale. The openers (no
break after) are `$ ( [ \ {`, `0x2018`, `0x201c`, `0x2035`, `0x3008`, `0x300a`, `0x300c`, `0x300e`, `0x3010`,
`0x3014`, `0x301d`, `0xfe59`, `0xfe5b`, `0xfe5d`, `0xff04`, `0xff08`, `0xff3b`, `0xff5b`, `0xffe1`, `0xffe5`,
`0xffe6`. The closers (no break before) are `! % ) , . : ; ? ] }`, `0xb0`, `0xb7`, `0x2013`, `0x2014`,
`0x2019`, `0x201d`, `0x2022`, `0x2026`, `0x2027`, `0x2032`, `0x2033`, `0x2103`, `0x3001`, `0x3002`, `0x3009`,
`0x300b`, `0x300d`, `0x300f`, `0x3011`, `0x3015`, `0x301e`, `0x30fc`, `0xfe30`, `0xfe50` to `0xfe52`, `0xfe54` to
`0xfe57`, `0xfe5a`, `0xfe5c`, `0xfe5e`, `0xff01`, `0xff05`, `0xff09`, `0xff0c`, `0xff0e`, `0xff1a`, `0xff1b`,
`0xff1f`, `0xff3d`, `0xff5d`, `0xff70`, `0xff9e`, `0xff9f`, `0xffe0`. The East Asian ranges are `0x1100`
to `0x11ff`, `0x3000` to `0xd7af`, `0xf900` to `0xfaff`, `0xff00` to `0xff9f` and `0xffa0` to `0xffdc`.
The sets come from the function's jump tables; the tests run both groups against the exe.
