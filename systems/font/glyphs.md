# font: glyph cache and atlas

A font caches glyphs in eight atlas pages of 256 by 256 ARGB4444 texels. A glyph that is not
cached is rasterized, baked into a cell of a page and recorded in a hash and a recency list. The
baking depends on the font's flags: plain, monochrome, or either of them with an outline.

Roots: `0x5cabd0`, `0x5cf310`, `0x5cf360`, `0x5cf5a0`, `0x5cf4d0`, `0x5c8710`, `0x5c9fa0`

## Finding or making a glyph

`0x5cabd0(code)` returns the cached descriptor of a character code, making it when missing. A hit
moves the descriptor to the front of the recency list. A miss rasterizes the glyph, places it
in a page (making a page if needed, evicting the least recently used glyph when the pages are
full) and enters it in the hash.

Addresses: `0x5cabd0`, `0x5cabe8`, `0x5cabf7`, `0x5cac3e`, `0x5cac71`, `0x5cacba`, `0x5cad06`, `0x5cad15`, `0x5cad60`, `0x5cad7e`, `0x5cada9`, `0x5cadf4`, `0x5cae16`, `0x5cae2e`, `0x5cae6c`, `0x5c7f00`, `0x5c7f20`, `0x5ca160`, `0x5cf5a0`, `0x5cf4d0`, `0x5cf360`, `0x5c8050`, `0x5c81b0`, `0x5c9f50`, `0x5c9490`, `0x5c9fa0`
Confidence: verified
Proof: one
Test: none

`0x5cabd0` looks the code up in the glyph hash (`+0x30`, mask `+0x38`; 12-byte buckets): a match is
unlinked from the recency list at `+0x68` and relinked at its front (`0x5cac5e` to `0x5cac98`) and returned.
On a miss it builds a descriptor on the stack (`0x5c7f00`) and rasterizes (`0x5ca160`; failure returns 0). Then
for each page from `+0x18c` in steps of 0x18 that has a texture it tries to place the glyph (`0x5cf5a0`); if no page has one
the first empty page is created (`0x5cf4d0`, then `0x5cf360(font, cellSize)`) and tried. Otherwise the oldest descriptor
(`[[+0x68]]`) is evicted: its cell is released (`0x5c8050`) and the glyph takes a cell by `0x5c81b0`. The descriptor
gets its page number at `+0x28`, is entered in the glyph hash (an empty hash is first given mask 3, `0x5cada9`),
stores the code at `+0`, its top byte at `+0x14`, goes to the front of the recency list and marks the page dirty (`[page + 8] = 1`).
`0x5c9fa0(text)` tells whether every character of a text, ignoring CR and LF, is already in the glyph hash.

## Cache containers

The font and the batch keep three hash tables of one template: the batch's runs by font, the font's glyphs by code and its kerning
pairs. Each table has 12-byte buckets, a mask that is −1 while empty, and an element manager whose
vtable has four slots: allocate, free, delete and clear.

Addresses: `0x80a884`, `0x80a8cc`, `0x80a8bc`, `0x5c3510`, `0x5c3540`, `0x5c3870`, `0x5c37b0`, `0x5cb170`, `0x5cb1e0`, `0x5cb6a0`, `0x5cb2d0`, `0x5cb420`, `0x5cb4a0`, `0x5cb760`, `0x5cb560`, `0x5c8e40`, `0x5c8e7a`, `0x5c8ea0`, `0x5c8ec3`, `0x5c8ed5`
Confidence: inferred
Proof: none
Test: none

The slots are read from the rodata at the vtable addresses: the batch table `0x80a884` holds `0x5c3510`, `0x5c3540`,
`0x5c3870` and `0x5c37b0`; the glyph table `0x80a8cc`
holds `0x5cb170`, `0x5cb1e0`, `0x5cb6a0`, `0x5cb2d0`; the kerning table `0x80a8bc` holds `0x5cb420`, `0x5cb4a0`,
`0x5cb760`, `0x5cb560`. The batch add `0x5c8e40` looks the string's font (`[+0x44]`) up in the run table: it compares the
entry's two key words with the font (`0x5c8e7a`), makes an entry through slot 1 of the manager (`0x5c8ed5`) when none is
found, sets the three key words to the font and puts the string at the head of the run's list. The roles of the
three tables, and the bodies of the bucket, list and growth functions, are not read here.

## Pages and cells

A page is cut into rows of the font's cell size and each row is a list of cells. `0x5cf360` sets
how many rows a page has: `256 / cellSize` (an unguarded `div`), with 256 free pixels in each row. `0x5cf5a0` tries
each row in turn to find a free span for a glyph through `0x5c81b0`. `0x5cfca0` (`0x5cf491`) constructs
each 16-byte row in `0x5cf360`: its list at `+4` is made empty (`0x5cfcb2`) and `0x18` is stored at `+0` (`0x5cfcb7`).

Addresses: `0x5cf360`, `0x5cf491`, `0x5cfca0`, `0x5cf5a0`, `0x5cf5b7`, `0x5cf5cb`, `0x5cf5f0`, `0x5c8010`, `0x5c8030`, `0x5c7f80`, `0xc2ba48`
Confidence: verified
Proof: one
Test: `font::texcache_set_cellsize_difftest`, `font::atlas_texel_addr_difftest`, `font::glyph_uv_rect_difftest`, `font::cell_free_after_difftest`, `font::cell_free_before_difftest`

`0x5cf5f0` is the address of a glyph's top-left texel in the staging buffer `0xc2ba48`:
`0xc2ba48 + 2 · ((page · cellSize) · 256 + x0)`, 256 texels of 2 bytes per row. `0x5c7f80` sets the glyph's
UV rectangle, each value times `1/256` (`0x80a8ac`): `v_top = a · b` (the cell's first texel row), `u_left = x0`, `v_bottom = a · b + b` (one past its last), `u_right = advance + x0`,
stored at `+0x60`, `+0x64`, `+0x68`, `+0x6c`. `0x5c8010` returns the free span to the right of a cell: with a next cell
`next.x0 − this.x1 − 1`, else `255 − this.x1`; `0x5c8030` the span to the left: with a previous cell
`this.x0 − prev.x1 − 1`, else `this.x0`. A cell link with bit 0 set is the list's end.

## Page texture

`0x5cf4d0` makes the page texture: 256 by 256, format ARGB4444, with a callback `0x5cf520` that supplies the
pixel data. The callback reports a row pitch of 0x200 and the buffer `0xc2ba48`.

Addresses: `0x5cf4d0`, `0x5cf4f2`, `0x5cf501`, `0x5cf50a`, `0x5cf520`, `0x5cf56c`, `0x5cf58e`, `0x5cf594`, `0x58aa30`, `0x58a980`, `0xc2ba48`
Confidence: verified
Proof: one
Test: none

`0x5cf4d0(flag)` calls `0x58aa30` with 0x100 and the callback `0x5cf520` and the page as its arguments. The
callback returns at once for a state of 0 or 2; otherwise it walks the page's rows and the cells of
each (`0x5cf5f0`) and stores 0x200 and `0xc2ba48` in its two out values. The device calls
`0x58aa30` and `0x58a980` are not read here.

## Baking a glyph

`0x5cf310` bakes the rasterized bitmap into a cell by the font's flags: outline and monochrome
use `0x5ce440`, outline alone `0x5cea30`, monochrome alone `0x5ce910`, and neither `0x5cf220`.

Addresses: `0x5cf310`, `0x5cf316`, `0x5cf32c`, `0x5cf335`, `0x5cf346`, `0x5cf34f`, `0x5cf220`, `0x5ce910`, `0x5ce440`, `0x5cea30`
Confidence: verified
Proof: one
Test: none

`[[page + 4] + 0x180]` bit 0 selects the outline pair and bit 1 monochrome within it
(`0x5cf31c`, `0x5cf324`, `0x5cf33e`); the two arguments pass through. The cell is first cleared
(`rep stos` of the pad rows) and then filled; each function writes the cell's `cellSize` rows of
`clearWidth` texels.

## Plain and monochrome bakes

The plain bake writes each coverage byte as `(src << 8) | 0x0fff`: alpha is the top four bits of the coverage and
the colour is white. The monochrome bake writes `0xffff` for a set bit and 0 for a clear one.

Addresses: `0x5cf220`, `0x5cf298`, `0x5cf29b`, `0x5cf2a1`, `0x5ce910`
Confidence: verified
Proof: one
Test: `font::glyph_blit_argb4444_difftest`, `font::glyph_blit_mono_difftest`

Both write `top_pad` blank rows, then the glyph's rows, then blank rows to the cell height; a row is
`0x100` texels apart (`add esi,0x200`, `0x5cf26e`). The monochrome source bit of (row `r`, column `c`) is
`(src[r · stride + (c >> 3)] >> (7 − (c & 7))) & 1`.

## Outlined bakes

The outline is baked into the cell: the monochrome form grows the glyph by one ring of texels, two rings when the font is
thick; the antialiased form also derives a soft alpha from the neighbours.

Addresses: `0x5ce440`, `0x5cea30`, `0x80a8ec`
Confidence: verified
Proof: one
Test: `font::glyph_blit_outline_difftest`, `font::glyph_blit_aa_outline_difftest`

`0x5ce440` rasterizes into a working grid of 256-texel rows starting `top_pad + 1` rows down and one column right (two columns with a
thick outline; the row offset is 1 for both widths, `0x5ce481`, `0x5cea7e`). Pass 0 marks every free cell next to a glyph cell as 4; with the thick flag pass 1 marks the free cells
next to a 4 as 2. It packs `cellSize × clearWidth` texels: glyph `0xffff`, ring 2 `0x7000`, ring 4 `0xf000`, else 0
(black outline, white fill). `0x5cea30` keeps an outline map (1 where coverage is non-zero), the coverage map and an alpha map; its
two passes dilate by one (mask 1 to 2; thick: mask 3 to 4). Each cell's alpha nibble is
`LUT[n]` with `n` the count of non-zero outline cells in its 3 by 3 box and `LUT = 0, 1, 1, 3, 5, 7, 9, 11, 13, 15`
(the bytes at `0x80a8ec`). A cell with outline 0 is 0; with coverage 0 it is `alpha << 12`; otherwise
`{A = alpha, R = G = B = (coverage · 0xff) >> 12}`.

## Placing the bitmap

The rasterizer seats the bitmap vertically in its cell and converts the face's 26.6 values to
pixels. Both functions are in the glyph file of `gx`. The cell height is the font's rendered pixel
size, its em: a glyph's record keeps at most em rows of its bitmap, from the top, so a glyph taller
than the em loses its bottom rows.

Addresses: `0x5d1360`, `0x5d1371`, `0x5d1376`, `0x5d1393`, `0x5d13a1`, `0x5d1120`, `0x5d1195`, `0x5d11a2`, `0x5d11da`, `0x5d12eb`, `0x5ca1b5`, `0x5ca1be`, `0x5cae90`, `0x5cf27f`, `0x5ce96c`, `0x5ce490`, `0x5ceaa0`
Confidence: verified
Proof: one
Test: `font::glyph_vplace_difftest`, `font::rasterize_glyph_difftest`

`0x5d1360(gap, pad, cellHeight, ascender, rows)` reads `bitmapTop` from the face's slot (`+0x68`).
Rows above the cell height, unsigned (`0x5d1371`), zero the pad (`0x5d1376`); otherwise a `bitmapTop`
above the ascender gives `gap = bitmapTop − ascender` and no pad, else `pad = ascender − bitmapTop`,
at least 0 (`0x5d1393`). Then if `cellHeight − rows < pad`, signed (`0x5d13a1`), `gap = cellHeight −
pad − rows` and `pad = cellHeight − rows`, so rows above the cell give both `cellHeight − rows`, below
0. The `ascender` is `[font + 0x17c]`. `0x5d1120(face, em, code, ascender, out, flag, pad)` takes `em`
from `0x5cae90`, `[font + 0x24c]` (`0x5ca1b5` to `0x5ca1be`). Its record holds `min(em, rows)` rows
(`0x5d1195` to `0x5d11a2`), which is what reaches `0x5d1360` as `rows`, with `em` as the cell height
(`0x5d12eb`), so `0x5d1120` never takes the over-tall branch. The advance is
`(float)((advance + (advance >> 31 & 0x3f)) >> 6) + 1.0` and the bearing `bearing · 1/64`; a blank
glyph has `em` rows and `(em + 3) >> 2` columns (`0x5d11da`). Each bake loops the record's rows from
the buffer's start (`0x5cf27f`, `0x5ce96c`, `0x5ce490`, `0x5ceaa0`). The face's calls are not read here.

## Glyph vertical terms

A glyph record holds two vertical terms after the bitmap record `0x5d1120` fills moves into it: `+0x58` is the gap and `+0x5c` the top padding that `0x5d1360` returns. `0x5c7f50(source)` frees the record's buffer at `+0` (`0x5c7f30`), copies 14 dwords from the source to `+0x38` and clears the source's first word, so the source's `+0x20` and `+0x24` become `+0x58` and `+0x5c`. The quad builder reads `+0x58` only: `proto.y` rises by `gap · scale`, which is 0 unless the glyph's `bitmap_top` exceeds the ascender or its ink overflows the cell (the cases of `0x5d1360`, above). The bakes read the padding at the record's `+0x24` to seat the ink in the cell.

Addresses: `0x5c7f50`, `0x5c7f30`, `0x5c7f6b`, `0x5c7f6d`, `0x5c8286`, `0x5c8360`, `0x5c8494`, `0x5d12eb`, `0x5d129e`, `0x5d12a4`, `0x5cd0fa`, `0x5cf237`, `0x5ce454`, `0x5cea70`, `0x5cabd0`
Confidence: verified
Proof: one
Test: none

`0x5c81b0` calls `0x5c7f50` at `0x5c8286`, `0x5c8360` and `0x5c8494`, each with `ecx` = glyph record + `0x38` (`0x5c827d`). `0x5d129e` and `0x5d12a4` give `0x5d1360` the addresses of the bitmap record's `+0x20` (gap) and `+0x24` (padding), set to 0 first (`0x5d12b8`, `0x5d12be`), and `0x5d12eb` calls it only when the bitmap exists. `0x5cd0fa` loads `[glyph + 0x58]` with `fild`; the plain, monochrome and outlined bakes load `+0x24` of their record (`0x5cf237`, `0x5ce454`, `0x5cea70`). The census of other readers of `+0x58` and `+0x5c` was not made.

## Emitting quads

`0x5c8710` copies the quads of one line buffer into the batch's vertex buffer with the pen added, and
with the shadow flag set first draws the shadow copy. The string's `0x5ce0c0` calls it with the main colour
`+0x2c`, the shadow offsets `+0x34`, the shadow colour `+0x30`, the anchor `+0x70`, the shadow bit (flag bit 0) and flag bit 5.

Addresses: `0x5c8710`, `0x5ce0c0`, `0x5ce090`, `0x5c8752`, `0x5c8775`, `0x5c879b`, `0x5c879e`, `0x5c8800`, `0x5c8842`, `0x5c8847`, `0x5c88af`, `0x5c6fa0`, `0x5c7010`
Confidence: verified
Proof: one
Test: `font::glyph_quad_emit_difftest`

In the shadow pass the pen is `penX = floor(ScreenToPixelWidth(offset.x)) + anchor.x` and `penY = floor(ScreenToPixelHeight(offset.y)) + anchor.y`
(the raw flag clear; the width result goes through a float store, `0x5c879b`), so the shadow lies a whole
number of pixels from the text: the offsets are rounded to pixels and floored again. A vertex is `{x + penX, penY + y, z, u, v}`. Without
a fade the colour is the given colour, with red and blue swapped when the device asks
(`[0x5c8230() + 0x1c] == 1`). A shadow pass with a fade multiplies the two alphas and `0x80a8b0`. `0x5ce090(line)` returns
a line buffer's count (`[buffer + 0xc]`), doubled when the shadow flag is set. The fade cases are not covered by the test.

## Main pass pen

In the main pass `0x5c8710` adds the string's anchor to each vertex and nothing else: `x + anchor.x`, `y + anchor.y`, with the vertex z unchanged, and the colour is the main colour or the glyph's own colour. The shadow arm is the only one that adds an offset. A world-space batch takes another path: with `[0xc2b9dc]` non-zero each quad's position goes through the matrix function `0x7bca80` instead.

Addresses: `0x5c8710`, `0x5c874a`, `0x5c88f9`, `0x5c8ab3`, `0x5c8aba`, `0x5c8abc`, `0x5c8ad7`, `0x5c8ae7`, `0x5c8af5`, `0x7bca80`, `0xc2b9dc`, `0x5ce0c0`, `0x5ce0e8`, `0x5ce0f3`
Confidence: verified
Proof: one
Test: none

`0x5ce0c0` passes the anchor `+0x70` as the fifth argument and the shadow bit (flag bit 0) as the sixth (`0x5ce0e8` to `0x5ce0f3`). `0x5c874a` tests that bit: clear goes to the main arm `0x5c88f9`, and with it set, a start index below the buffer's quad count goes to the shadow arm (`0x5c875a`) and one at or above it is reduced by the count (`0x5c875c`) and goes here. `0x5c8ab3` tests `[0xc2b9dc]`: zero takes `0x5c8ad7`, which adds `[anchor]` to x (`0x5c8ae7`) and `[anchor + 4]` to y (`0x5c8af5`) and copies z unchanged. The batch drawer sets the flag only for a world-space batch and clears it afterwards (`font/api#drawing-a-batch`).
