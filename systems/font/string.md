# font: strings and their layout

A string (`CGxString`, 0xc4 bytes) keeps a text, a size, a box, a justification and, once drawn,
a layout: lines of glyph quads in per-line buffers. This doc covers the object, its size and
anchor rules, the line layout and the quad of one glyph. Creating and destroying strings is in
`api`.

Roots: `0x5cd6d0`, `0x5ccb80`, `0x5cdf70`, `0x5cdc20`, `0x5ccbe0`, `0x5cd310`, `0x5cd4d0`, `0x5cd6a0`

## String object

The fields the functions of this doc load and store:

| offset | holds |
|---|---|
| `+0x00`, `+0x10` | links: global or pool string list; the font's string list |
| `+0x18`, `+0x1c` | the size; the size clamped to two device pixels |
| `+0x20` to `+0x28` | the position, 3 floats |
| `+0x2c` to `+0x38` | main colour, shadow colour (alpha at `+0x2f`, `+0x33`), shadow offsets |
| `+0x3c`, `+0x40` | the box width and height |
| `+0x44` to `+0x4c` | the font; the text buffer; its capacity |
| `+0x50` to `+0x5c` | vertical and horizontal justification (0 to 2), line spacing, flags |
| `+0x68`, `+0x6c` | the selection range, −1 when none |
| `+0x70` to `+0x78` | the screen anchor |
| `+0x7c` to `+0x90` | the hyperlink region array and the layout's counts |
| `+0x9c`, `+0xa0` | laid-out line count (0 for no layout); eight per-line buffers |
| `+0xc0` | frames since the last layout |

Addresses: `0x5cd6d0`, `0x5cd736`, `0x5cd73c`, `0x5cd73f`, `0x5cd75e`, `0x5cd767`, `0x5cd764`, `0x5cd76c`, `0x5cd780`, `0x5cdf70`, `0x5cdef0`, `0x5cdf40`, `0x5cd9f0`
Confidence: verified
Proof: one
Test: none

The flag bits that the code reads: bit 0 shadow on, bit 2 take the font's natural height as the
size, bit 3 and bit 5 relayout rules (`0x5ccb40`, `0x5cd650`), bit 7 (a negative byte) a pre-snapped mode that
skips the anchor snap and keeps `+0x28`. The offsets are the ones stored
by `0x5cd6d0` at the addresses above and read by `0x5cdf70`, `0x5cdef0` and `0x5cdf40`.

## Setting the text and size

`0x5cd6d0` fills a new string: it copies the text, stores the box, spacing, justification and flags,
the position and colour, links the string into its font's list and sets the size. It returns 1.

Addresses: `0x5cd6d0`, `0x5cd6df`, `0x5cd707`, `0x5cd728`, `0x5cd774`, `0x5cd7cf`, `0x5cd7de`, `0x5cd800`, `0x5cd818`, `0x5ccb80`, `0x5ccb9a`, `0x5ccbc5`, `0x5c2540`, `0x801628`
Confidence: verified
Proof: one
Test: `font::cgxstring_size_clamp_difftest`, `font::resolve_clamp_size_difftest`

Its eleven stack arguments are the size, the position pointer, the box width and height, the
font, the text, the vertical and horizontal justification, the spacing, the flags and a pointer
to the colour. The buffer grows when `strlen + 1` exceeds `+0x4c` (a free and an allocation
of `GxuFontString.cpp` lines `0x1bd` and `0x1bf`) and the text goes through `SStrCopy`. When
the flags byte is not negative `+0x28` becomes 0. With flag bit 2 and a non-negative flags
byte the size is the font's natural height (`0x5c2540`), otherwise the argument. The clamped size `+0x1c` is
`size > 2.0 / H ? size : 2.0 / H` with `H` the viewport height (`fdivr [0x801628]`, `fcomp`, `test ah,0x41`: ties and NaN
keep the floor). `0x5ccb80` repeats the rule for an existing string (the same size pick, the same
clamp) and invalidates the layout; `0x5ca6f0` calls it for every string of a font after a resize.

## Anchor and justification

`0x5cdf70` turns the string's position into the screen anchor `+0x70` to `+0x78` by its justification.
With no layout (`+0x9c == 0`) or a pre-snapped string (negative flags byte) the anchor is the
position, unchanged.

Addresses: `0x5cdf70`, `0x5cdf83`, `0x5cdf9e`, `0x5cdfa9`, `0x5cdfb2`, `0x5cdfde`, `0x5cdfec`, `0x5ce000`, `0x5ce01d`, `0x5ce056`, `0x5ce05c`, `0x5ce07e`, `0x808120`
Confidence: verified
Proof: one
Test: `font::anchor_justify_snap_difftest`

Horizontal justification 2 adds the box width `+0x3c` to the anchor x and 1 adds half of it;
any other value adds nothing. The block height is `h = (linesPx / H) · (n − 1) + n · clamped`, with `n = [+0x9c]` and
`linesPx = __ftol(H · spacing + 0.99999)`. The anchor y is then: justification 0, `boxH + y`;
1, `((boxH − h) · 0.5 + y) + h`; otherwise `h + y`, with `boxH = +0x40` (`0x5ce06b` to `0x5ce07b`). Both
coordinates snap to pixels, `x = floor(W · x)` and `y = floor(H · y)`, and the anchor z is 0. `W` is the
viewport width.

## Laying out lines

`0x5cdc20` lays out a string: for each line it measures the line (`0x5c7260`), computes the line's
horizontal offset and emits its glyph quads. It stops at the box bottom, at the end of the text, on
no progress, or after one line when flag bit 1 is set.

Addresses: `0x5cdc20`, `0x5cdc55`, `0x5cdcc9`, `0x5cdcdc`, `0x5cdd01`, `0x5cdd08`, `0x5cdd37`, `0x5cddd3`, `0x5cde28`, `0x5cdeb6`, `0x5c7260`, `0x5ccbe0`, `0x5c6fa0`, `0x5c7010`
Confidence: verified
Proof: one
Test: `font::string_layout_lines_difftest`

`lineMetric = ScreenToPixelHeight(raw, −clamped)`. Not raw (flag `0x80` clear): `wrapPx = __ftol(H · spacing + 0.99999)`,
`lineStepFrac = wrapPx / H`, `lineStep = ScreenToPixelHeight(false, clamped) + (float)wrapPx`, `originZ = anchor.z`. Raw:
`lineMetric = 0`, `lineStepFrac = spacing`, `lineStep = spacing + clamped`, `originZ = 0`. Each line: the loop ends
when the pen y reaches the box bottom (`penY >= bottom` or NaN); else `penY += clamped + lineStepFrac`; it ends on an empty cursor.
A newline token is a line of its own (`count++`, `baseline −= lineStep`). Otherwise it measures the line and ends when `next ==
cursor` or `bytes == 0 && *next == 0`; the horizontal offset is `ScreenToPixelWidth(raw, −offset)` with
`offset` the line width for justification 2, half of it for 1 and 0 otherwise; it emits the line at origin
`(offset, baseline, originZ)`, increments the count, lowers the baseline by `lineStep`, and stops after one
line for flag bit 1. `wrapPx` has a zero high dword (`0x5cdcf2`) for `fild qword`, so a negative count reads as unsigned. The test runs
the layout through the real measure and quad build.

## One glyph quad

`0x5ccbe0` writes the four vertices of one glyph: positions from the pen, the glyph's width and the
line height, the atlas UVs and the colour.

Addresses: `0x5ccbe0`, `0x5ccc74`, `0x5ccc8a`, `0x5ccca0`, `0x5ccccd`, `0x5ccec3`, `0x5ccf62`, `0x5cd099`, `0x5cd0b6`, `0x5cd0d8`, `0x5cd1aa`, `0x5cd1cf`, `0x5cd1fd`, `0x5cd21b`, `0x5cd248`, `0x5cd274`, `0x5cb080`, `0x80306c`, `0xc2b9f8`, `0xc2ba00`
Confidence: verified
Proof: one
Test: `font::glyph_quad_build_difftest`

`sizeMetric = ScreenToPixelHeight(raw, clamped)`, `scale = sizeMetric / pixelSize` and
`lineHeight = sizeMetric` plus 4.0 (font flag bit 3) or 2.0 (bit 0). For a literal glyph (the class above 5
at `0x5ccec3`): `proto.x = GlyphWidth(raw, width, clamped, pixelSize) + penX` (`0x5cb080`), `cellW = (u32)advance · scale`,
floored unless raw, and `proto.y = top · scale + (originY − pad)` with `pad` 2.0 for font flag bit 3, 1.0 for
bit 0 or 0. The height is `lineHeight`, or `sizeMetric` when raw. The corners are `v0 = (x, y)`, `v1 = (x, y + H)`, `v2 = (x + cellW, y)` and `v3 = (x + cellW, y +
H)` with z `originZ`; the UVs are copied per column and row, with `+1/512` added to bottom and left and
`−1/512` to top and right when raw (`0xc2b9f8`, `0xc2ba00`); the colour word is stored in all four vertices.

## Baseline row of a line

The row of the screen where the baseline of a line falls follows from the anchor, the line ladder, the quad corners and the cell's top padding. With the quad scale 1 and no gap term it is the same row for plain text and for a normal outline, and one row higher for a thick outline.

Addresses: `0x5cdf70`, `0x73ff3f`, `0x5cdc55`, `0x5cdd08`, `0x5c6fa0`, `0x5ccbe0`, `0x5ce481`, `0x5cea7e`, `0x5cf237`
Confidence: verified
Proof: one
Test: none

The anchor y is `floor(H · y)` (`0x73ff3f`); the main pass adds it to every quad. Line `k` (from 0) has the origin y `−r − k · (r + wrapPx)` with `r = ScreenToPixelHeight(raw, −clamped)`, which is `−round(H · clamped)` (`0x5cdc55`; `0x5c6fa0` rounds half away from zero and has no cap) and `wrapPx` as in the line layout above; the step is the `lineStep` of `0x5cdd08`. A quad's top is `origin + gap · scale − pad + lineHeight` with `lineHeight = sizeMetric + 2 · pad`, and `pad` is 0 for a plain font, 1 for a normal outline (flag bit 0) and 2 for a thick one (flag bit 3). The bake puts the first ink row at cell row `top_pad` for a plain glyph (`0x5cf237`) and at `top_pad + 1` for an outlined one (the grid offset `0x202` bytes, one row and one column, at `0x5ce481` and `0x5cea7e`). The ink top is therefore `origin + sizeMetric − top_pad` for plain and normal, and one row higher for thick: its quad top rises by 2 over the plain one, the ink row stays.

## String defaults

`0x5cda00` clears a new string and sets its defaults: the size `+0x18` and the clamped size `+0x1c` to `0x3ca3d70a` (0.02), the main colour `+0x2c` to `0xffffffff`, the shadow colour `+0x30` to `0xff000000`, the shadow offsets `+0x34` and `+0x38` to `0x3aa3d70a` and `0xbaa3d70a` (±0.00125) and the flags `+0x5c` to 0. The shadow flag is clear, so the default offset draws nothing until `0x5cd650` sets a shadow.

Addresses: `0x5cda00`, `0x5cd8b2`, `0x5cda16`, `0x5cda30`, `0x5cda37`, `0x5cda3e`, `0x5cda57`, `0x5cd650`, `0x5cd66f`, `0x5cd67b`
Confidence: verified
Proof: one
Test: none

`0x5cd8b2` calls `0x5cda00`. The offsets are fractions of the viewport: the x (`+0x34`) goes through `0x5c7010` (width, `0x5c878b`) and the y (`+0x38`) through `0x5c6fa0` (height, `0x5c8783`) when the shadow is drawn. `0x5cd66f` and `0x5cd67b` store the two offsets from the argument of `0x5cd650`, and `0x5cd678` sets flag bit 0.

## Hyperlink regions and selection

A hyperlink region is a rectangle the layout records for a `|H` link; its coordinates are stored
as fractions of the screen. A selection paints a range of cells of the line buffers with a
fading alpha.

Addresses: `0x5cd310`, `0x5cd316`, `0x5cd369`, `0x5cd4d0`, `0x5cd55c`, `0x5cd581`, `0x5cd615`, `0x5cd460`, `0x5cd490`, `0x5cd420`, `0x5cd3f0`
Confidence: verified
Proof: one
Test: `font::hyperlink_region_rect_difftest`, `font::selection_highlight_fade_difftest`

`0x5cd310` forms `invH = 1.0 / H` and `invW = 1.0 / W`, multiplies the rectangle's y fields (`+0x08`, `+0x10`) by `invH` and its x
fields (`+0x0c`, `+0x14`) by `invW`, and appends the 32-byte record to the array at `+0x7c`. `0x5cd4d0`
fills a selection (columns `start` to `start + end`): `base = [+0x2f]`, `step = __ftol(base / end)`; for cell `i` of `cellCount`:
below `4 · start` or past the range, the cell gets `base`; inside the range the cell gets `base` and, on odd `i`, `base` drops by
`step` (to 0 at the floor). It returns 2 when the range starts past the cells, otherwise whether the range
reaches beyond them. `end` must be at least 1: 0 divides by zero. Per-cell alpha bytes
are written and read by `0x5cd460` and `0x5cd490`; `0x5cd420` lays the string out and returns the count at `+0x84` when `+0x80` is set and the viewport is non-empty, and
`0x5cd3f0` drops the layout when a glyph of its text is not cached (`0x5c9fa0`).
