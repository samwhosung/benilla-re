# font: measuring and wrapping text

The kernels that decide how wide a text is, how much of it fits on a line and how tall a block
of lines is. They all walk the text with the markup decoder, add up glyph advances and kerning
steps, and convert to the caller's unit, a fraction of the screen. The x87 conventions of
`metrics` apply.

Roots: `0x5c6940`, `0x5c6c50`, `0x5c7260`, `0x5c7300`, `0x5c7470`, `0x5c70b0`

## Line seed

Every measure starts a line with a pen offset: the extra spacing converted to whole pixels,
plus a pad for an outline or a shadow. The pad is 4.0 when font flag bit 3 is set, else 2.0 for bit 0, else 0.

Addresses: `0x5c6940`, `0x5c698d`, `0x5c69e2`, `0x73fdf5`, `0x80306c`, `0x801628`
Confidence: verified
Proof: one
Test: `font::text_extent_measure_difftest`, `font::wrap_max_chars_difftest`, `font::measure_run_proportional_difftest`

`seed = extra > 0 ? ceil(ScreenToPixelWidth(extra)) : 0` stored as a float, then `+ 4.0` or `+ 2.0` by
the flag (`fadd [0x80306c]`, `fadd [0x801628]`, each stored as a float). `ScreenToPixelWidth` is called with
its raw flag clear. The seed is the run's start for `0x5c6940` (`0x5c698d` to `0x5c69e2`), `0x5c6c50`
and `0x5c7470`.

## Text extent

`0x5c6940(text, bytes, size, flags, extra)` returns the width of the widest line of the first `bytes`
bytes of a text, as a fraction of the screen width (in pixels when flag `0x80` is set).

Addresses: `0x5c6940`, `0x5c2050`, `0x5c6a1a`, `0x5c6a3b`, `0x5c6a40`, `0x5c6a8c`, `0x5c6a95`, `0x5c6aa0`, `0x5c6ac5`, `0x5c6ace`, `0x5c6fa0`, `0x5c2540`
Confidence: verified
Proof: one
Test: `font::text_extent_measure_difftest`

A size of 0, or flag bit 2, takes the font's natural height (`0x5c2540`). The loop decodes a token
(`0x5c2810`) and subtracts its length from the byte count. Class 2, or a glyph `0x0a`, ends the line; classes 3 and 6 are glyphs;
classes 0, 1, 4 and 5 are skipped. For a glyph with a descriptor (`0x5cabd0`): `kern = prev ? step(prev, cur) : 0`,
`lastAdv = advance(cur)` (`0x5c6b70`, a float store), `run += kern` (a float store) and `prev = cur`. At a line
end `lineWidth = lastAdv + run` replaces the line maximum when strictly greater, and `run` and
`lastAdv` reset (`prev` does not). The last line is closed the same way. The result is
`(ScreenToPixelHeight(size) / pixelSize) · width`, divided by the viewport width unless flag `0x80`.
`0x5c2050` is the C API entry that calls it.

## Fitting characters to a width

`0x5c6c50` counts how many characters of a text fit one line of a given box width and can fill an
array with the running width after each character. `0x5c2320` uses it with an effectively
infinite box to find how many characters from the end fit a width.

Addresses: `0x5c6c50`, `0x5c2190`, `0x5c2320`, `0x808120`, `0x73fdf5`
Confidence: verified
Proof: one
Test: `font::wrap_max_chars_difftest`, `font::font_width_fit_difftest`

`0x5c6c50` takes the size default, the line seed and the threshold `ceil(pixelSize · W · box / max(ScreenToPixelHeight(size), 1))`
where `W` is the viewport width and the ceiling is `__ftol(q + 0.99999)` (`0x808120`, a bias added only for `q > 0`). The
scale is `ScreenToPixelHeight(size) / pixelSize`. A glyph is accepted unless `kern + advance + run > threshold`; the
test stops without counting it and rewinds the cursor. An accepted glyph does `run += kern`,
`lastAdv = advance`, `count++` and, with an array, `cum[i] = run · scale` (divided by `W` unless flag `0x80`). A
newline ends the line and a class 3 token is a glyph. `0x5c2190` wraps it. `0x5c2320` measures the text at a
box of 10000.0, returns all characters when the total fits, else walks the array for the first
`i` with `total − box < cum[i]`, returning `count − i` and `total − cum[i]`; no such `i` gives 0.

## Steps along a line

`0x5c7260` measures one line step for a text. It chooses the fixed-width run `0x5c7300` for flag
`0x80` and the proportional run `0x5c7470` otherwise, and zeroes its outputs for a negative size
or box or an empty text. Each returns the bytes consumed, the width and the start of the next line.

Addresses: `0x5c7260`, `0x5c7300`, `0x5c7470`, `0x5c7710`, `0x5c741f`, `0x5c7780`, `0x73fdf5`
Confidence: verified
Proof: one
Test: `font::measure_run_fixedwidth_difftest`, `font::measure_run_proportional_difftest`

The fixed run adds `step(prev, cur)` for each glyph and ends at a newline; the width is
`(sum + lastAdv) · (size / pixelSize)` with the raw `size`, the bytes are the cursor at the end and
the next position follows a skip over trailing white space (`0x5c741f`). The proportional run
tests each glyph with `kern + advance + run > threshold`, where `threshold = ceil((pixelSize /
ScreenToPixelHeight(size)) · box · W)`, and tracks the latest break opportunity (`0x5c7780`) and its width.
It ends in one of three ways: at a newline or the end of the text; at the last break opportunity; or
in the middle of a word, which sets the caller's broke flag to `(flags >> 6) & 1`. Widths are scaled by
`ScreenToPixelHeight(size) / pixelSize` over `W`. The proportional run skips the white space after
the break with `0x5c7710`.

## Block height and fit

`0x5c2070` returns the height of a wrapped text: the number of lines `N` times the size plus the
line gap between them. `0x5c21c0` counts how many characters fit a given height.

Addresses: `0x5c2070`, `0x5c21c0`, `0x8026bc`, `0x5c7260`
Confidence: verified
Proof: one
Test: `font::font_textblock_height_difftest`, `font::font_height_fit_difftest`

`0x5c2070` counts lines by calling the step `0x5c7260`: a newline token advances by its length, any
other measures a line and jumps to its next position; no progress, flag bit 1 or a null
continuation ends the count. The height is `N · size + (N − 1) · ceil(gap · H) / H` with `H` the viewport height in
pixels, and 0 for `N = 0`. `0x5c21c0` returns 0 for any zero argument or an empty text; it
rounds the gap to whole pixels the same way and lays lines until one takes no characters or
`box + 2⁻²⁰ < used + lineHeight` (`0x8026bc` is 2⁻²⁰); each line adds its characters and
`lineHeight + gap` to the height; a line that ends hard or the end of the text stops it.

## Viewport rectangle

`0x5c70b0` rounds a screen-fraction viewport rectangle to whole pixels before the device sets it.

Addresses: `0x5c70b0`, `0x58b060`, `0x58a230`, `0x5c3d90`, `0x73ff3f`
Confidence: verified
Proof: one
Test: none

The rectangle is queried from the device (`0x58b060`); each edge becomes `floor(W · x)` or `floor(H · y)` and a
half-pixel offset, (−0.5, +0.5) when the device caps field `+4` is 0, is added. An empty
rectangle (`x1 <= x0` or `y1 <= y0`) is refused. The orthographic matrix `0x5c3d90(x0, x1, y0, y1, −1, 1)` is then installed.
The device query cannot be run alone, so no test covers it.
