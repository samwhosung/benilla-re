# font: scaling, advances and metrics

The arithmetic under every text measure: the conversion between a fraction of the screen and
pixels, the width and advance of a glyph, the step between two glyphs with their kerning, and
the per-font metrics derived from the face. The x87 runs at 53-bit precision, so each `f*` result
is a double and each `fstp dword` rounds to a float. The rounding calls are the C runtime's
`ceil` (`0x73fdf5`, control word `0x1b3f`) and `floor` (`0x73ff3f`, `0x173f`).

Roots: `0x5c6fa0`, `0x5c7010`, `0x5c7080`, `0x5ca030`, `0x5ca2b0`, `0x5ca2d0`, `0x5ca4b0`, `0x5cb080`, `0x5c6b70`, `0x5caea0`, `0x5ca6f0`

## Screen and pixel units

The layout measures in fractions of the screen height. `0xc2b9a0` holds the viewport height in
pixels and `0xc2b9a4` its width. `0x5c6fa0(raw, v)` converts a height fraction to pixels (rounded half away
from zero), `0x5c7010` does the same against the width, `0x5c7080(pixels)` converts back and `0x5c2540(font)`
returns a font's natural height `pixelSize / screenHeight`.

Addresses: `0x5c6fa0`, `0x5c7010`, `0x5c7080`, `0x5c2540`, `0xc2b9a0`, `0xc2b9a4`, `0x5c2563`, `0x5c257c`
Confidence: verified
Proof: one
Test: `font::screen_to_pixel_height_difftest`, `font::screen_to_pixel_width_difftest`, `font::pixel_to_screen_height_difftest`, `font::one_to_one_height_difftest`

`0x5c6fa0` multiplies the fraction by the viewport height (`fild qword`, `fmul`), adds 0.5 for a
positive product and −0.5 otherwise, truncates (`__ftol`) and converts back to a float; when
its flag is set it returns the argument unchanged. `0x5c7010` is the same against `0xc2b9a4`.
`0x5c7080` is `fild pixels; fidiv screenHeight` (an integer divide). `0x5c2540` is
`fild [font + 0x24c]; fidiv [0xc2b9a0]` (`0x5c2563` to `0x5c257c`); for a null font it sets error `0x57`
and returns 0.0.

## Viewport size

`0x5c2b50` recomputes the two pixel sizes from the device's viewport rectangle and, when the
rectangle changed, re-lays every font.

Addresses: `0x5c2b50`, `0x5c2bbe`, `0x5c2be9`, `0x5c2bfd`, `0x5c2c07`, `0x5c2c1d`, `0x5c2c87`, `0x5c2c92`, `0x5c2c9c`, `0x5c2cbd`, `0xc2b828`, `0xc2b82c`, `0xc2b9a0`, `0xc2b9a4`, `0x5ca6f0`
Confidence: verified
Proof: one
Test: `font::viewport_pixel_dims_difftest`

The device rectangle (`0x58a240`, four floats) is `{r0, r1, r2, r3}`. If `r3 − r1` or `r2 − r0` is
0 the rectangle `{0, 0, 480.0, 640.0}` replaces it (`0x5c2bfd`, `0x5c2c07`). A rectangle equal to the cached one at
`0xc2b82c` to `0xc2b838` ends the call. Otherwise it is cached, `0xc2b9a4 = __ftol(r3 − r1)` and
`0xc2b9a0 = __ftol(r2 − r0)` (the width is stored first), and `0x5ca6f0` runs for every font of the
registry, from `0xc2b97c` by the previous links. The first call registers an empty exit
function with `_atexit` (`0x5c2b64`).

## Per-font metrics

`0x5ca030(font)` derives the raster size, the cell size and the advance ratio of a font from its
requested size and the face.

Addresses: `0x5ca030`, `0x5caea0`, `0x5cae90`, `0x5ca160`, `0x5ca6f0`
Confidence: verified
Proof: one
Test: `font::font_raster_metrics_difftest`, `font::min_metric_screen_height_difftest`

With `m = 0x5caea0(face, requested)` and `s = max(m, 2.0 / screenHeight)` (a float): the rendered
size `+0x24c = min(32, __ftol(ScreenToPixelHeight(0, s)))` (an unsigned compare against 32);
the baseline ascender `+0x17c = __ftol(renderSize · (asc / (|desc| + asc)) + 0.5)` with `asc` and `desc` the
face's signed 16-bit ascent and descent (the denominator is stored as a float); the cell size
`+0x178 = renderSize + 4` for flag bit 3, else `+ 2` for bit 0, else `+ 0`; and
`+0x188 = (float)(u16 [face[0x58] + 0xc] / u16 face[0x44])`. `0x5caea0(face, v)` returns
`max(v, minMetric / screenHeight)`, `minMetric` the smallest signed 16-bit value among `count` values
(seeded `0x7fffffff`; an empty array returns `v`). The function returns whether the face's size
call (`0x7ce760`) succeeded. `0x5ca6f0` is the relayout after a viewport change: it empties the
font's caches (`0x5ca760`), runs `0x5ca030` and resolves every string's clamped size (`0x5ccb80`).

## From a UI size to pixels

A size set through the interface reaches `GxuFontCreateFont` as a fraction of the screen height after one division, and the
font turns it into a pixel em with `0x5ca030`. The em is `min(32, round(H · size))` and never exceeds 32.

Addresses: `0x44d040`, `0x44d073`, `0x41ae50`, `0x832a48`, `0x44d07b`, `0x44d088`, `0x44d202`, `0x44d20e`, `0x44d216`, `0x44d224`, `0x5ca160`, `0x7ce760`, `0x5ca1d0`
Confidence: verified
Proof: one
Test: none

The shared font bridge `0x44d040(name, flags, size)` calls `0x41ae50` (`size / [0x832a48]`, `fld [ebp+8]; fdiv`) at
`0x44d073`, stores the quotient as a float and replaces it with 1.0 when it is 1.0 or more
(`0x44d07b` to `0x44d088`). It maps the caller's flags to the font's: the bit tested at `0x44d202`
gives 1, bit 2 gives 8 (`0x44d20e`) and bit 1 gives 2 (`0x44d216`), and calls `0x5c1ae0` at `0x44d224`. The
requested size `+0x184` is that float; `0x5ca030` and `0x5ca160` pass the rendered size
`+0x24c` to the face's size call `0x7ce760(face, size, 0)`, so the face rasterizes at the em, never at the ascender-scaled value.
The rows of `0x5caea0` apply only to a face with fixed bitmap sizes; a scalable face returns the size unchanged.

## Glyph width and step base

A glyph's step base is its `+0x4c` metric, plus 1.0 for a thick outline. Its width metric is its `+0x50`
value, rounded up in pixel mode or scaled by the render size over the font's pixel size.

Addresses: `0x5ca2b0`, `0x5cb080`, `0x5ca240`, `0x5ca294`
Confidence: verified
Proof: one
Test: `font::glyph_metric_outline_bias_difftest`, `font::glyph_width_scaled_difftest`

`0x5ca2b0(desc)` loads `[desc + 0x4c]` and adds 1.0 when `[font + 0x180]` has bit 3. `0x5cb080(desc, mode, size)`:
mode 0 returns `ceil([desc + 0x50])`; otherwise `ScreenToPixelHeight(1, size) / [font + 0x24c] · [desc + 0x50]` (an
integer divide by the pixel size). `0x5ca240(code)` finds a glyph in the hash and returns its step
base, or 0.0 when there is none.

## Advance of a glyph

`0x5c6b70` returns a glyph's total advance in pixels: its width metric plus its integer pixel
advance `[desc + 0x48]`, or the explicit kerning when the text names a pair.

Addresses: `0x5c6b70`, `0x5cb080`, `0x5cabd0`
Confidence: verified
Proof: one
Test: `font::char_advance_difftest`

`0x5c6b70(font, text, ...)` returns 0.0 for a zero current code. It skips control codes from the
cursor with the markup decoder; on the first literal it measures the pair with the step functions below
and returns that value when it is non-zero. Otherwise it returns `0x5cb080(desc, ...) + (float)(s32)[desc + 0x48]` with
the descriptor from `0x5cabd0`; a missing glyph gives the last (zero) value.

## Step between two glyphs

`0x5ca2d0(prev, cur)` is the proportional step from one glyph to the next: the previous glyph's
step base plus its kerning with the next, rounded up. `0x5ca4b0` is the fixed-width step, which
centres each glyph in a cell of `[font + 0x178]` pixels.

Addresses: `0x5ca2d0`, `0x5ca4b0`, `0x5ca63a`, `0x5ca240`, `0x73fdf5`
Confidence: verified
Proof: one
Test: `font::compute_step_difftest`, `font::compute_step_fixedwidth_difftest`

Proportional: with `k` the raw kerning (from the face) the step is
`ceil((float)(stepBase(prev) + min(k, 0) · [font + 0x188]))`: `min(k, 0)` is `((−1 < k) − 1) & k`, the sum is an
`fild`, `fmul`, `faddp` and one `fstp dword`. Results are cached per pair in the kerning hash; a miss is
computed by the face. Fixed width, with `R = [font + 0x178]` and `g1`, `g2` the two glyphs: `m1 = ftol(stepBase(g1))`; if `m1 >= R`
(unsigned) the step is `R`. Else `d1 = R − m1`, `d1 −= d1 & 1`, `center = R − d1 / 2`. Then `m2 = ftol(stepBase(g2))` (0 for a missing glyph); if
`m2 >= R` the step is `center`, else `center + (d2 − (d2 & 1)) / 2` with `d2 = R − m2`. All of it is unsigned integer arithmetic
around float loads (`0x5ca63a` onward).
