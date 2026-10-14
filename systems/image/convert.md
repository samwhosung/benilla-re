# image: pixel format conversion

How a decoded level is turned into the pixel format the device wants. The source pixels are BGRA
bytes (`[0]` B, `[1]` G, `[2]` R, `[3]` A), 16-bit words, DXT blocks or palette indices. Every
function here is integer code. The tests run each function whole in the emulator, with the
allocator and the C runtime's `atexit` stubbed.

Roots: `0x5a4f60`, `0x5a8ce0`, `0x5a5740`, `0x5a5780`, `0x5a62c0`, `0x5a7910`

## Conversion matrix

The dispatcher `0x5a4f60` picks a converter from a matrix of function pointers at `0xc0f560` and
calls it. The matrix is indexed by source class, destination class and a submode; 17 cells hold
a converter and every other cell is 0, which makes the call do nothing. The class numbers are the
ones `blp` takes from `0x5a80d0`: 1 BGRA8888, 2 ARGB4444, 3 ARGB1555, 4 RGB565, 5 DXT1, 6 DXT3,
7 DXT5, 8 any other 16-bit.

| source, destination, submode | converter |
|---|---|
| 1, 1, 0 / 1 / 2 | `0x5a5480` copy / `0x5a5510` alpha key / `0x5a5590` blend |
| 1, 2 / 3 / 4, 0 | `0x5a5070` / `0x5a51d0` / `0x5a5330` pack to 4444 / 1555 / 565 |
| 2, 2 / 3, 3 / 4, 4 / 8, 8 | `0x5a56b0` 16-bit copy |
| 5, 5 | `0x5a5740` DXT1 block copy |
| 6, 6 and 7, 7 | `0x5a5780` DXT3 or DXT5 block copy |
| 5, 4 / 3 / 1 | `0x5a57c0` / `0x5a5a20` / `0x5a5c80` DXT1 to 565 / 1555 / 8888 |
| 6, 2 / 1 | `0x5a5ef0` / `0x5a60c0` DXT3 to 4444 / 8888 |

DXT5 has no software decoder: a DXT5 source to any other class is an empty cell.

Addresses: `0x5a4f60`, `0x5a4f63`, `0x5a4f72`, `0x5a4f77`, `0x5a4f8f`, `0x5a4fab`, `0x5a4fc0`, `0x5a4fca`, `0xc0f558`, `0xc0f560`
Confidence: verified
Proof: one
Test: none

The call is `0x5a4f60(ecx = &{w, h}, edx = submode, src, srcStride, srcClass, dst, dstStride,
dstClass)` with `ret 0x18`. The cell is `[0xc0f560 + 4 · (submode + 4 · dstClass + 36 · srcClass)]`
(`lea`, `add`, `lea eax,[esi+eax*4]`, `0x5a4f87` to `0x5a4f8f`); a non-null cell is called with
`ecx = &dims`, `edx = src` and the strides and destination pushed (`0x5a4fab`). The first
call fills the matrix with `0x5a4fc0` and sets the guard `0xc0f558`; each cell address is a `mov`
immediate there (`0xc0f600` is cell 1, 1, 0).

## Packing to 16 bits

The three packers narrow each BGRA pixel by dropping low bits and keep the pixel order.

| function | result |
|---|---|
| `0x5a5070` | `(A >> 4) << 12 \| (R >> 4) << 8 \| (G >> 4) << 4 \| (B >> 4)` |
| `0x5a51d0` | `(A >> 7) << 15 \| (R >> 3) << 10 \| (G >> 3) << 5 \| (B >> 3)` |
| `0x5a5330` | `(R >> 3) << 11 \| (G >> 2) << 5 \| (B >> 3)` |

Addresses: `0x5a5070`, `0x5a51d0`, `0x5a5330`, `0x5a5336`, `0x5a533e`, `0x5a53b7`, `0x5a53ce`, `0x5a53d1`, `0x5a53d6`, `0x5a5404`
Confidence: verified
Proof: one
Test: `image::image_bgra8888_to_argb4444`, `image::image_bgra8888_to_argb1555`, `image::image_bgra8888_to_rgb565`

A width below 2 takes the single-pixel loop (`0x5a5404`). Otherwise each pass converts two pixels
and stores one dword: the column advances by 2 while it is below the width (`0x5a53d1`, `0x5a53d6`), so
an odd width of 3 or more also converts the pixel after the row's last and writes it past the row. The rows
advance by the strides, in bytes.

## Copies and blends

The BGRA copy moves `w · h` dwords in one run when both strides equal `4 · w`, else row by row.
The alpha key copy `0x5a5510` writes `(dst & 0xff000000) | (src & 0xffffff)` for each pixel whose
source alpha is not 0. The blend `0x5a5590` skips a source alpha of 0, treats 0xff like the key copy, and
otherwise sets each colour channel to `dst + ((src − dst) · a >> 8)` and keeps the destination
alpha. The 16-bit copy does the same moves with `2 · w`.

Addresses: `0x5a5480`, `0x5a549a`, `0x5a5510`, `0x5a5543`, `0x5a554c`, `0x5a5590`, `0x5a55d3`, `0x5a55ee`, `0x5a5622`, `0x5a5627`, `0x5a565a`, `0x5a56b0`, `0x5a56bf`
Confidence: verified
Proof: one
Test: `image::image_bgra8888_copy`, `image::image_bgra8888_alpha_key`, `image::image_bgra8888_blend`, `image::image_copy16`

The key copy is `xor, and 0xffffff, xor` (`0x5a554c` to `0x5a5553`). The blend's per-channel
step is `sub edx,ecx; imul edx,esi; shr edx,8; add dl,cl` (`0x5a5622` to `0x5a562a`): the
`shr` is logical on a 32-bit product, so a negative difference becomes the floor of
`(src − dst) · a / 256` in the low byte. The copy's fast test compares `4 · w` with both strides (`0x5a5493`, `0x5a549a`).

## DXT interpolation table

`0x5a7910(base)` fills the table at `0xc0fa80` that every DXT decoder reads: four tables of 16-bit
values `la(n) = n · 256 / 3 + 1` and `lb(n) = n · 512 / 3 + 1`, at 5 bits (32 entries each at `+0x00`
and `+0x40`) and 6 bits (64 entries each at `+0x80` and `+0x100`).

Addresses: `0x5a7910`, `0x5a7900`, `0x5a7917`, `0x5a7921`, `0x5a7926`, `0x5a7934`, `0x5a7961`, `0x5a796f`, `0xc0fa80`
Confidence: verified
Proof: one
Test: `image::image_dxt_interp_lut`

The thunk `0x5a78f0` runs `0x5a7900`, which loads `ecx = 0xc0fa80` and jumps here. The loop
multiplies `n << 8` or `n << 9` by `0xaaaaaaab`, shifts the high dword right by 1 (division
by 3), adds 1 and stores a word (`0x5a7926`, `0x5a7934` for the 5-bit tables; `0x5a7961`,
`0x5a796f` for the 6-bit). The tables give `(2 · c0 + c1) / 3 = (lb[c0] + la[c1]) >> 8` and
`(c0 + 2 · c1) / 3 = (la[c0] + lb[c1]) >> 8`, truncated.

## DXT1 colours

A DXT1 block is two 565 endpoints and 32 index bits. If `color0 > color1` (unsigned) the four
colours are `c0`, `c1`, `(2 · c0 + c1) / 3` and `(c0 + 2 · c1) / 3`; otherwise `c0`, `c1`, `(c0 + c1) / 2`
and transparent black. Each channel is computed on its own and truncated.

Addresses: `0x5a7200`, `0x5a73b0`, `0x5a7640`, `0x5a7207`, `0x5a7213`, `0x5a7230`, `0x5a7246`, `0x5a7333`, `0x5a7352`, `0x5a7396`, `0x5a6990`, `0x5a6be0`, `0x5a6ef0`, `0x5a6590`, `0x5a6880`, `0x5a6e70`, `0x5a6ea0`, `0x5a7190`
Confidence: verified
Proof: one
Test: `image::image_dxt1_palette_builders`, `image::image_dxt1_block_decoders`, `image::image_dxt_pack_565comp`, `image::image_dxt_pack_word`

`0x5a7200(block, palette)` is the 565 builder: it copies both endpoints, then `jbe 0x5a7333`
when `color0 <= color1`. The four-colour path reads `lb` of one endpoint and `la` of the other
from `0xc0fb80`, `0xc0fb00` (6-bit green) and `0xc0fac0`, `0xc0fa80` (5-bit red and blue), adds, shifts
right 8 and packs. The three-colour path adds the channels, halves (`sar 1`) and stores 0 as the
fourth colour (`0x5a7396`). `0x5a73b0` and `0x5a7640` build the 1555 and 8888 palettes the same way,
through `0x5a6e70`/`0x5a6ea0` and `0x5a6590`/`0x5a7190`. The packers join channels: ARGB8888 is
`0xff` alpha, `r << 3`, `g << 2`, `b << 3` with no low-bit replication (`(((r | 0xffffffe0) << 9 | g) << 7 | b) << 3`);
`0x5a6880` packs ARGB4444 as `((r & 0xfffe | 0xffe0) << 5 | g & 0xfffc) << 2 | b >> 1`. The block
decoders write one 4 by 4 block through a table of four row pointers and a bounds record
`{x0, y0, x1, y1, advance}`, then advance each row pointer by `advance` pixels.

## DXT3 colours

A DXT3 block is 8 bytes of explicit 4-bit alpha (row `y` in the word at `y · 2`) and a DXT1-style colour
half at `+8`, whose palette is always the four-colour one. The decoders write colour with the
alpha: ARGB4444 stores `(colour & 0xfff) | alpha << 12`, ARGB8888 stores the packed colour with
the alpha byte from a callback.

Addresses: `0x5a62f0`, `0x5a65c0`, `0x5a60b0`, `0x5a62b0`
Confidence: verified
Proof: one
Test: `image::image_dxt3_block_decoders`

The callback `0x5a60b0` returns its nibble unchanged (for 4444) and `0x5a62b0` returns `nibble << 4`
(for 8888, so alpha 0xf is 0xf0, not 0xff). Colour index bits come from the byte at `+12 + y`, two
bits per pixel, least significant first.

## DXT drivers

A driver walks a surface in 4 by 4 blocks, building four destination row pointers per block row and
calling the block decoder. For a surface whose width and height are at least 4 and multiples of 4 the
decoder gets the bounds `{0, 0, 3, 3, 4}`; otherwise the bounds are `{0, 0, min(w − 1, 3), min(h − 1, 3), min(w − 1, 3) + 1}`.

Addresses: `0x5a57c0`, `0x5a5a20`, `0x5a5c80`, `0x5a5ef0`, `0x5a60c0`, `0x5a57e0`, `0x5a5948`, `0x5a5817`, `0x5a58b6`, `0x5a58d5`, `0xcf78a8`, `0xcf78a1`, `0xcf78a2`, `0xcf78b0`, `0x5a5a10`, `0x5a5c70`, `0x5a68c0`, `0x5a68d0`, `0x5a68e0`, `0x5a68f0`, `0x5a6900`, `0x5a6910`, `0x5a6920`, `0x5a6930`, `0x5a6940`, `0x5a6950`, `0x5a6960`, `0x5a6970`, `0x5a6980`, `0x5a71d0`, `0x5a71e0`, `0x5a71f0`
Confidence: verified
Proof: one
Test: `image::image_dxt1_drivers`, `image::image_dxt3_drivers`

`0x5a57e0` to `0x5a57e8` test `w >= 4`, `h >= 4`, `w & 3 == 0` and `h & 3 == 0`; the full case
(`0x5a5948`) calls the block decoder with the 4 by 4 bounds. The other case (`0x5a57ee`) builds
the palette of each block into a function-local static at `0xcf78a8` (guard bytes at `0xcf78a1` and
`0xcf78a2`, `atexit` registrations at `0x5a5804` and `0x5a589d`) and writes each pixel from its
two index bits, using the clamped counts. The clamp uses the surface dimension, not the block's
remainder: a surface of 4 or more that is not a multiple of 4 gets its last block column and row
written in full, up to 3 pixels beyond the row. The DXT1 and DXT3 block copies `0x5a5740` and `0x5a5780` move
`max(w, 4) · max(h, 4) · bpp / 8` bytes whatever the strides are.

## Palette to BGRA

`0x5a8790(dst, src, count)` expands byte indices to BGRA dwords from the record's palette at
`+0x98` and then overwrites each alpha byte from a following alpha plane. `0x5a87d0` writes
alpha 0xff and then fills the alpha from the plane by the record's alpha depth.

Addresses: `0x5a8790`, `0x5a87d0`, `0x5a8801`, `0x5a8837`, `0x5a8888`, `0x5a8ce0`, `0x5a8cfc`, `0x5a8d7e`, `0x5a8dbc`, `0x5a8dfa`, `0x5a8e38`, `0x85c8b0`, `0x85c8c0`
Confidence: verified
Proof: one
Test: `image::image_palconv_expand_bgra8888_a8`, `image::image_palconv_expand_bgra8888_adepth`

Both read the colour from `[+0x98 + 4 · index]` and so keep the palette's own alpha byte until the
alpha stage. The alpha plane follows the index plane, `count` bytes on. Depth 8 copies
the bytes (`0x5a8790` always does). Depth 4 (`0x5a8837`) gives two pixels per byte, low nibble first, looked
up in `0x85c8b0` (`n · 0x11`). Depth 1 (`0x5a8888`) gives eight pixels per byte, low bit first, looked up in
`0x85c8c0` (`0x00`, `0xff`). Any other depth leaves 0xff. The stage-2 dispatcher `0x5a8ce0(fmt, level,
dst, src)` switches on `fmt − 2`: 2 to `0x5a8790` for alpha depth 8 and `0x5a87d0` otherwise, 3 to
`0x5a9580`, 4 to `0x5a92d0`, 5 to `0x5a9860` and 9 to `0x5a9a80`; 6 to 8 and above 9 return 0. Its
counts are `w · h` of the level.

## Dithered 16-bit palettes

The four live converters turn indices into 16-bit colour with Floyd-Steinberg error diffusion:
`0x5a92d0` RGB444 with alpha merged to 4444, `0x5a9580` RGB555 with alpha merged to 1555,
`0x5a9860` RGB565 without alpha, and `0x5a9a80` RGB565 followed by a 2-bit alpha plane.

Addresses: `0x5a92d0`, `0x5a9580`, `0x5a9860`, `0x5a9a80`, `0x5a98b6`, `0x5a991f`, `0x5a9947`, `0x5a9959`, `0x5a9979`, `0x5a998a`, `0x5a99e5`, `0xc0fc10`, `0xc1bc98`, `0xc21cd0`, `0xc15c68`, `0x85c8b0`, `0x85c9b0`, `0x85c9c8`
Confidence: verified
Proof: one
Test: `image::image_palconv_fs_rgb444`, `image::image_palconv_fs_rgb555`, `image::image_palconv_fs_rgb565`, `image::image_palconv_fs_rgb565_a2`

Each call zeroes its own two-row error buffer (a static per function: `0xc0fc10`, `0xc1bc98`,
`0xc21cd0`, `0xc15c68`; rows `0x3018` bytes apart, row parity selects the row, three dwords per pixel
for R, G, B). Per pixel and channel `accum = (palette channel << 16) + (error >> 4)`,
`q = clamp((accum + bias) >> shift, 0, max)` and the new error is `accum − ((accum + bias) & mask)`.
RGB565 uses bias `0x40000`, `0x20000`, `0x40000`, shifts `0x13`, `0x12`, `0x13`, masks `0xfff80000`,
`0xfffc0000`, `0xfff80000` and maxima 31, 63, 31 (`0x5a9947` to `0x5a99b3`); RGB555 uses `0x40000`, `0x13`
and 31; RGB444 `0x80000`, `0x14` and 15. The error goes `· 7` to the next pixel of the row, and to
the next row `· 5` below-left, `· 3` below and, by a store, `· 1` below-right (`imul ebx,ebx,7` at
`0x5a99e5`, `lea ebx,[edx+edx*4]`, `lea ebx,[edx+edx*2]`): the classic kernel has the 3 and 5
the other way round. The palette is read R at `+0x9a`, G `+0x99`, B `+0x98`.

## Alpha merge into 16 bits

After the colour pass `0x5a92d0` and `0x5a9580` OR the file's alpha plane into the words, and
`0x5a9a80` packs a 2-bit alpha plane after the colours.

Addresses: `0x5a92d0`, `0x5a9580`, `0x5a9a80`, `0x85c8b0`, `0x85c8c4`, `0x85c9b0`, `0x85c9b4`, `0x85c9c8`
Confidence: verified
Proof: one
Test: `image::image_palconv_fs_rgb444`, `image::image_palconv_fs_rgb555`, `image::image_palconv_fs_rgb565_a2`

The alpha plane follows the indices. For 4444 the depth 4 nibble goes to bits 12 to 15 and depth 8
takes its top nibble; depth 1 indexes the table `0x85c8c4` by the masked bit, not shifted down (`and edx,eax`,
`0x5a9553`: 1, 2, 4 … 128), a two-entry table that is not meant for it. Bit 0 reads `0xf000`;
bits 1 to 4 read the words 5, 6, 3, 7 of the format-class table `0x85c8c8`; bits 5 and 6 read
the Bayer tables (the words 4 and 2, at the table's offsets 64 and 128); bit 7 reads the word at
offset 256, `0xf00f`. For 1555 the top
bit of each alpha is ORed into bit 15. `0x5a9a80` writes its plane at `dst + 2 · count`, four
pixels per byte, with masks `0x85c9b0`, shifts `0x85c9b4` and nibble shifts `0x85c9c8`.
