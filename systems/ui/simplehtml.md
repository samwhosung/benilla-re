# ui: SimpleHTML

How a `SimpleHTML` frame turns its text into blocks: the markup it accepts, the plain-text
fallback, how blocks stack and what font each takes. The book, letter and item text frames are
this type.

A `CSimpleHTML` is a hyperlinked frame of 0x374 bytes with the vtable 0x81d030. Its fields are a
list of content nodes `+0x340`, the last text block `+0x348`, the offset below it `+0x34c` (zero or
less), four element fonts `+0x350` to `+0x35c` (`P`, `H1`, `H2`, `H3`), the hyperlink format
`+0x360`, by default `|H%s|h%s|h`, and a flag `+0x370` that asks for the hyperlink regions to be
rebuilt. The markup is parsed as XML by expat through `XMLTree.cpp` (`framexml`, XML trees).

Roots: `0x789dd0`, `0x796f50`

## Setting the text

`SetText` first destroys the old blocks, then parses the text. The root must be `HTML` and the
first child named `BODY` is walked; later `BODY` elements are ignored. Any other root or child
reports `Unknown element type`. When the text does not parse, the root is not `HTML`, or no child
is `BODY`, the whole raw text becomes one `P` block justified `LEFT`, with its whitespace and
newlines kept. An empty `BODY` shows nothing. Names compare ignoring case.

Addresses: `0x78a3a0`, `0x78a40c`, `0x78a415`, `0x78a422`, `0x78a42c`, `0x78a449`, `0x78a46e`, `0x78a49d`, `0x78a4b6`, `0x78a4ba`, `0x78a4bf`, `0x78a4e1`, `0x78a4fc`, `0x78a501`, `0x78a508`, `0x78a50d`, `0x796a90`, `0x6f2a30`
Confidence: verified
Proof: one
Test: none

`0x78a3a0` clears through `0x78a580` (`0x78a40c`), zeroes its markup byte (`0x78a415`) and parses
with `0x6f2a30` (`0x78a422`); a failure jumps to the fallback (`0x78a42c`). It compares the root with
`HTML` (`0x78a449`) and each child with `BODY` (`0x78a46e`), reporting through the sink's slot 0xc
(`0x78a49d`, `0x78a4e1`); `BODY` sets the byte (`0x78a4b6`), is walked (`0x78a4ba`) and ends the
loop (`0x78a4bf`). A zero byte (`0x78a4fc`) calls `0x78adb0(text, 0, 1)` (`0x78a501`, `0x78a508`);
`0x78a50d` sets `+0x370`. `0x796a90` passes `lua_tostring` of argument 2.

## Elements

Inside `BODY`, `H1`, `H2`, `H3` and `P` make a text block, `BR` a block holding one newline, and
`IMG` an image; text directly inside `BODY` is dropped. A block's `align` is one of the six justify
names, default `LEFT`. Inside a block, `BR` adds a `|n` line break and `A` with a nonempty text and
`href` adds the hyperlink format filled with the `href` and the text. Any other element, here or in
a block, reports `Unknown element type` and its text is lost.

Addresses: `0x78a660`, `0x78a66f`, `0x78a68e`, `0x78a6b5`, `0x78a6dc`, `0x78a703`, `0x78a726`, `0x78a739`, `0x78a74e`, `0x78a790`, `0x78a7b0`, `0x78a7c8`, `0x78a7cf`, `0x78a7e2`, `0x78a825`, `0x78a865`, `0x78a8f0`, `0x78a90a`, `0x78aa29`, `0x87a838`
Confidence: verified
Proof: one
Test: none

`0x78a660` returns for a `BODY` with no child (`0x78a66f`) and compares each child's name with `H1`,
`H2`, `H3`, `P`, `BR`, `IMG` (`0x78a68e` to `0x78a74e`); `BR` calls `0x78adb0("\n", 0, 1)`
(`0x78a739`). `0x78a7b0` stages 1 as the align (`0x78a7c8`) and reads `align` with `0x6f1990`
(`0x78a7e2`); a child `BR` splices `|n` (`0x78a825`, `0x78a865`), an `A` reads `href` (`0x78a8f0`,
`0x78a90a`) and formats `+0x360`, whose default is the string at `0x87a838`.

## Whitespace in a block

Before a block is made, its text is collapsed: each run of spaces, tabs, carriage returns and
newlines becomes one space; whitespace at the start, right after a `|n`, and at the end is dropped,
and a space right before a `|n` is removed. Everything else, `|` escapes included, is kept. The
plain-text fallback is never collapsed.

Addresses: `0x78a8af`, `0x78a8b5`, `0x78a8c3`, `0x78a8d4`, `0x78aa3f`, `0x78aa41`, `0x78aa46`, `0x78aa50`, `0x78aa57`, `0x78aa5f`, `0x78aa7f`, `0x78aa95`
Confidence: verified
Proof: one
Test: none

The loop starts with the after-break byte set (`0x78a8b5`) and classes each byte through the table
0x78aac4 (`0x78a8d4`). Whitespace writes one space only when neither the after-break byte nor the
pending-space byte is set (`0x78aa3f`, `0x78aa41`). `|n` (`0x78aa46`) backs over a pending space
(`0x78aa50`), copies `|n` (`0x78aa57`) and sets the after-break byte (`0x78aa5f`); a trailing space
is backed over (`0x78aa7f`) before `0x78adb0` (`0x78aa95`).

## Blocks stack

Each text block is a `FontString` in the ARTWORK layer. The first is anchored `TOPLEFT` to the
frame's `TOPLEFT`; each later one `TOPLEFT` to the last text block's `BOTTOMLEFT`, offset down by
that block's line spacing snapped to pixels, so with the default spacing of 0 blocks touch. A
block's width is the frame's width when the text is set, and its height its own text's. It follows
its element's font, or `P`'s when the element's font has no file, and justifies by its `align`
alone, never by the font.

Addresses: `0x78adb0`, `0x78add6`, `0x78ade1`, `0x78adf0`, `0x78adfe`, `0x78ae0c`, `0x78ae19`, `0x78ae23`, `0x78ae29`, `0x78ae36`, `0x78ae3f`, `0x78ae54`, `0x78ae68`, `0x78ae75`, `0x78aea1`, `0x78aea9`, `0x78aebb`, `0x78aec0`, `0x78aec4`
Confidence: verified
Proof: one
Test: `ui::glue_788_78adb0_negate`

`0x78adb0(text, element, align)` builds the string with `0x770d30(this, 2, 1)` (`0x78add6`) and
calls `0x767c70` (`0x78ae0c`) with point 0 to the frame's point 0 when `+0x348` is 0 (`0x78adfe`),
else point 0 to `+0x348`'s point 6 at y `+0x34c` (`0x78adf0`). It copies the frame's width (slot
0x1c) into the block's (slot 0x14) (`0x78ae19`, `0x78ae23`), takes `+0x350` when the element's font
path is empty (`0x78ae36`, `0x78ae3f`), links it (`0x78ae54`), clears the justify inherit bits and
stores `align & 7` (`0x78ae68`, `0x78ae75`), sets the text (`0x78aea1`), stores the block
(`0x78aea9`) and `−0x766750(+0xf4)` (`0x78aebb` to `0x78aec4`).

## Images

`IMG` makes a texture of the `src` file in the ARTWORK layer, sized by its `width` and `height` in
script units when given. Aligned `LEFT`, `CENTER` or `RIGHT`, it hangs from the last text block's
bottom left, middle or right corner (the frame's top when there is none) at the current offset;
any other `align` leaves it unanchored. A `LEFT` or `RIGHT` image floats; any other moves the next
block down by its height. An image never becomes the block the next one hangs from.

Addresses: `0x78ab40`, `0x78ab60`, `0x78ab7b`, `0x78ab85`, `0x78abad`, `0x78abe5`, `0x78ac25`, `0x78acbb`, `0x78ace3`, `0x78acfa`, `0x78ad02`, `0x78ad0c`, `0x78ad12`, `0x78ad15`
Confidence: verified
Proof: one
Test: `ui::glue_geom_78ab40_unit_convert_and_subtract`

`0x78ab40` sets its float byte when `0x6f1990` gives 1 or 4 (`0x78ab7b`, `0x78ab85`), converts
`width` and `height` with `0x64aaa0` and `0x41ae60` (`0x78abad`, `0x78abe5`), and builds the
texture with `0x76fc40(this, 2, 1)` (`0x78ac25`). `LEFT` anchors point 0 to point 6 (`0x78acbb`),
`CENTER` 1 to 7 and `RIGHT` 2 to 8, through `0x767c70` (`0x78ace3`). `0x770200` loads `src`
(`0x78ad02`); unfloated (`0x78ad0c`), it subtracts the height (`0x78ad12`, `0x78ad15`) from `+0x34c`.

## Lua methods

`SimpleHTML` has 19 methods at `0x87ba80`, registered by `0x796f50`. The font methods take an
optional first argument naming the element, `P`, `H1`, `H2` or `H3` ignoring case, and act on
that element's font.

Addresses: `0x87ba80`, `0x796f50`, `0x795d80`, `0x795db3`, `0x795ddb`, `0x795e06`, `0x795e2e`, `0x795cb0`, `0x795e60`, `0x795f30`, `0x796000`, `0x7960d0`, `0x7961a0`, `0x796270`, `0x796340`, `0x796410`, `0x7964e0`, `0x7965b0`, `0x796680`, `0x796750`, `0x796820`, `0x7968f0`, `0x7969c0`, `0x796a90`, `0x796b40`, `0x796c30`
Confidence: verified
Proof: one
Test: none

The table holds pairs from `SetFontObject` (`0x795cb0`) to `GetHyperlinkFormat` (`0x796c30`);
`0x796f50` calls `0x701d80` with 0x13. `0x795d80` compares a string argument with `P`, `H1`, `H2`
and `H3` (`0x795db3` to `0x795e2e`) and removes it with `0x6f30d0` when it matches.

## Hyperlink format

The hyperlink format `+0x360` is a private copy of its text. The constructor sets `|H%s|h%s|h`,
the XML attribute `hyperlinkFormat` replaces it when present, and `SetHyperlinkFormat(format)`
replaces it when its argument is a string or a number; any other argument raises
`Usage: %s:SetHyperlinkFormat("format")` with the frame's name or `<unnamed>`. Each replacement frees
the old copy first.

Addresses: `0x78a540`, `0x78a54e`, `0x78a570`, `0x789eb4`, `0x78a2c0`, `0x78a2d4`, `0x796b40`, `0x796bcc`, `0x796be5`, `0x796c02`
Confidence: verified
Proof: one
Test: none

`0x78a540(format)` frees `+0x360` when it is set (`0x78a54e`, `SMemFree`) and stores `SStrDupA(format)`
(`0x78a570`). The constructor calls it at `0x789eb4` with `0x87a838`; `CSimpleHTML::LoadXML` reads the
attribute at `0x78a2c0` and calls it at `0x78a2d4`. The method tests `lua_isstring` (`0x796bcc`) and calls
it at `0x796be5`; the usage error is at `0x796c02`.
