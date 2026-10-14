# gx: glyph bitmaps

`IGxuFontGlyph.cpp` (`0x5d1110` through `0x5d1400`) turns one glyph of a font face into a bitmap record for the font layer (`font`). It calls the font engine (a FreeType-style face and glyph slot, `0x7ce960`, `0x7cdb40`, `0x7cec20`) and copies the result; the text layout that uses the records is not here.

Roots: `0x5d1120`, `0x5d1110`, `0x5d13d0`, `0x5d13f0`

## Glyph bitmap load

`0x5d1120(face, maxRows, char, maxWidth, out, flag, pad)` returns 0 for a null face, face size or `out` (error `0x57`), or when loading fails; otherwise 1. It loads and renders the glyph (`0x5d1300`): `0x7ce960` first, then `0x7cdb40` with the load flags `0x208a`, or `0x20c2` when `flag` is non-zero, then `0x7cec20` with render mode 0, or 1 when `flag` is set. It then reads the glyph slot's bitmap (`+0x4c` rows, `+0x50` width, `+0x54` pitch, `+0x58` pixels) and allocates a zeroed buffer of rows times pitch bytes under `IGxuFontGlyph.cpp` (line `0x93`), copying the pixels in; the rows are clipped to `maxRows`. A glyph with no bitmap gets a blank buffer of `maxRows` rows and `(maxRows + 3) / 4` columns.

Addresses: `0x5d1120`, `0x5d1300`, `0x5d1360`, `0x7ce960`, `0x7cdb40`, `0x7cec20`
Confidence: verified
Proof: one
Test: none

The record fills `out`: `+0` the buffer, `+4` its size, `+8` the width, `+0xc` the rows, `+0x10` the width plus `pad`, `+0x1c` the pitch (width rounded up to 8 when `flag` is set and the glyph is blank), `+0x14` the advance in whole pixels plus 1.0 (`metrics.horiAdvance / 64`, truncated toward zero) and `+0x18` the bearing `metrics.horiBearingX * (1 / 64)`. `0x5d1360(out + 0x24, maxWidth, bitmap_top, maxRows)` computes the clipping offsets of the glyph against the line (the slot's `+0x68`), and runs only for a real bitmap.

## Static initializers

The file's run is `0x5d1100` (+∞ from `0x80a924` to `0xc4baa8`), `0x5d13d0` (`2π`, `0xc4bab0`) and `0x5d13f0` (`1 / 2π`, `0xc4baac`).

Addresses: `0x5d1100`, `0x5d1110`, `0x5d13d0`, `0x5d13e0`, `0x5d13f0`, `0x5d1400`
Confidence: verified
Proof: one
Test: none

`0x5d1110` copies the dword at `0x80a924`; `0x5d13e0` stores `π + π`; `0x5d1400` stores `1.0` divided by the stored `2π`.
