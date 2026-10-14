# image: TGA

The TGA reader and writer of `tga.cpp`. An image record holds the file, the pixels and the
18-byte TGA header; the decoder turns types 1, 2, 9 and 10 into pixels, and callers then
flip, add or strip alpha, and save. Errors go through `SErrSetLastError`; the codes are the last
entry.

Roots: `0x5a3600`, `0x5a3660`, `0x5a3a30`, `0x5a3fc0`, `0x5a4070`, `0x5a41b0`, `0x5a41f0`, `0x5a4250`, `0x5a43c0`, `0x5a4810`

## Image record

An image record is 0x44 bytes. The TGA header is read straight into `+0x08`, so its fields sit at
the offsets of the file's own layout plus 8.

| offset | holds |
|---|---|
| `+0x00` | file handle |
| `+0x04` | pixels, 0 until decoded |
| `+0x08` | ID length |
| `+0x09`, `+0x0a` | colour map type, image type |
| `+0x0b`, `+0x0d`, `+0x0f` | colour map first index, length, entry size in bits |
| `+0x14`, `+0x16` | width, height |
| `+0x18`, `+0x19` | bits per pixel; descriptor |
| `+0x1c` | ID buffer |
| `+0x20` to `+0x39` | the 26-byte footer |
| `+0x3c`, `+0x40` | pixel bytes, palette |

Addresses: `0x5a3b40`, `0x5a3600`, `0x5a4320`, `0x5a3b6f`, `0x5a37e0`
Confidence: verified
Proof: one
Test: `image::image_tga_pixel_offset`

The descriptor's low nibble is the attribute (alpha) bit count, bit 4 flips horizontally and
bit 5 puts the origin at the top. Two idioms repeat: bytes per pixel is `(bpp + 7) >> 3`, and a
palette entry is `(min(size / 3, 8) · 3) >> 3` bytes (`imul 0x55555556` for the division), so
24 and 32-bit entries are 3 bytes, 16-bit entries 1 and 8-bit entries 0. `0x5a3b40` returns the
pixel data offset `entry · length + idLength + 0x12`. `0x5a3600` frees the pixels, closes the file,
frees the ID buffer (`0x5a4320`, which also clears `+0x08`) and the palette.

## Opening a file

`0x5a3660(path, flag)` opens the file through `0x6477c0`, reads the header, then the ID field
and the colour map, and returns 1. A null or empty path sets error `0x57`. A 24-bit image whose
attribute is 8 has the attribute cleared and a warning set.

Addresses: `0x5a3660`, `0x5a3684`, `0x5a3698`, `0x5a36b6`, `0x5a371c`, `0x5a3760`, `0x5a37c5`, `0x5a37ca`
Confidence: verified
Proof: one
Test: none

It first clears the record (`0x5a3600`), calls `0x6477c0(0, path, flag != 0, &record)` and reads
0x12 bytes with `0x648460` to `+0x08` (`0x5a36b6`). A non-zero ID length allocates that many
bytes at `+0x1c` and reads them. With a colour map it allocates and reads `entry · length`
bytes at `+0x40` (`0x5a371c`, `0x5a3760`); without one `+0x40` is 0. `0x5a37c5` compares `bpp`
with 0x18 and the attribute with 8, clears the attribute and sets `0x8720012e`.

## Decoding

`0x5a3a30(flags)` decodes into `+0x04`. Image types 1 and 9 go to the colour-mapped path, 2 to
the plain path and 10 to the run-length path; 0, 3 and 11 and the rest are refused. Flag bit 0
adds an opaque alpha channel where the file has none; flag bit 1 expands a palette to colours.

Addresses: `0x5a3a30`, `0x5a3a95`, `0x5a3b18`, `0x5a3b30`, `0x5a39d0`, `0x5a3c80`, `0x5a3d70`, `0x5a3ed0`, `0x5a3820`, `0x5a37e0`, `0x5a3b80`
Confidence: verified
Proof: one
Test: `image::image_tga_rle_decode`, `image::image_tga_palette_expand`, `image::image_tga_append_alpha`

The jump table at `0x5a3b18` is read through the byte table `0x5a3b30` (0 to 11):
`00 01 02 03 05 05 05 05 05 01 04 03`. Entries 1 and 9 reach `0x5a39d0`, entry 2 reaches
`0x5a3ace` and entry 10 `0x5a3ae9`; both first call `0x5a37e0`, which accepts only
`bpp − attribute == 24`. Pixels are the file's data from `0x5a3b40`: `0x5a3c80` allocates
`bytes · w · h` plus `w · h` when alpha is wanted (flag bit 0 and attribute 0), seeks, and reads the
pixels after the spare plane; `0x5a3d70` reads the compressed bytes to a temporary buffer, decodes with
`0x5a3ed0` and frees it. The alpha plane is interleaved by `0x5a3b80`. `0x5a39d0` requires a colour
map, runs the type 2 or 10 reader with flags 0, then calls `0x5a3820` when bit 1 is set.
An existing pixel buffer returns 1 with error `0x57`.

## Run-length packets

A control byte with bit 7 set repeats the next pixel `(b & 0x7f) + 1` times; a clear bit 7
copies `b + 1` pixels. The decode ends when `width · height` pixels have been written, and the image
type then drops by 8.

Addresses: `0x5a3ed0`, `0x5a3f10`, `0x5a3f16`, `0x5a3f51`, `0x5a3f84`, `0x5a3f8e`, `0x5a3fa0`
Confidence: verified
Proof: one
Test: `image::image_tga_rle_decode`

`0x5a3ed0(packets, out)` keeps the remaining pixel count in `esi`. After each packet it
subtracts the count (`0x5a3f16`, `0x5a3f51`), then `jl 0x5a3fa0` fails with `0xf7200077` and `jne`
loops (`0x5a3f84`); exact zero adds `0xf8` to the type (`0x5a3f8e`). The packet is copied
before the count is compared and the source is never bounded, so a final packet that
overshoots writes up to 128 pixels past the buffer and an unterminated stream reads past the
compressed data.

## Palette expansion

`0x5a3820(flags)` replaces the indices with colours: it allocates the new buffer, copies for each
pixel the palette entry `(index − firstIndex) · entry` (the entry size above), frees the indices
and the palette, and sets the type to 2, the colour map to none and `bpp` to `attribute + 24`.

Addresses: `0x5a3820`, `0x5a3840`, `0x5a3865`, `0x5a38bd`, `0x5a38f3`, `0x5a3954`, `0x5a3967`, `0x5a3988`, `0x5a39b3`
Confidence: verified
Proof: one
Test: `image::image_tga_palette_expand`

With flag bit 0 the colours are written `w · h` bytes into the buffer and `0x5a3b80` then
interleaves them with an opaque alpha at the front (`0x5a39b3`); the record's `bpp` and
descriptor are rewritten by that call. The new `+0x3c` is `bytes(bpp) · w · h` computed before the
alpha step (`0x5a3988`). The index is a byte (`movzx edi,[esi]`) and an index below the
first index reads before the palette.

## Alpha channel

`0x5a3b80(dst, src, alpha)` copies each pixel and appends one byte: the next byte of `alpha`, or
0xff for a null `alpha`. It then sets the attribute to 8, adds `8 − attribute` to `bpp` and
recomputes `+0x3c`. `0x5a3fc0(alpha)` adds an alpha plane to a decoded image and `0x5a4250`
removes it.

Addresses: `0x5a3b80`, `0x5a3bd0`, `0x5a3c32`, `0x5a3c4b`, `0x5a3c75`, `0x5a3fc0`, `0x5a3fc7`, `0x5a3fdd`, `0x5a3ff9`, `0x5a4048`, `0x5a4250`, `0x5a4265`, `0x5a4275`, `0x5a42bc`, `0x5a42e5`, `0x5a41b0`, `0x5a41f0`
Confidence: verified
Proof: one
Test: `image::image_tga_append_alpha`, `image::image_tga_alpha_strip`

`0x5a3b80` calls `0x4100f0` (a move) for `bytes(bpp)` bytes per pixel, then stores the alpha
(`0x5a3c32` stores 0xff). The header rewrite is `descriptor = (d & 0xf8) | 8` and `bpp += 8 − (d & 0xf)`
(`0x5a3c4b` to `0x5a3c75`). `0x5a3fc0` requires pixels (`0x5a41b0`), refuses run-length types
(`0x5a3fdd`), strips an existing alpha first (`0x5a3ff9`), allocates `(bytes + 1) · w · h`, calls
`0x5a3b80` and frees the old buffer. `0x5a4250` requires pixels. With a zero attribute it
returns 1 for `bpp == 24`, otherwise warns (`0x8720012d`) and takes `(bpp − 8) & 0xf` as the attribute;
an attribute other than 8 fails with `0xf7200082`. It lowers `bpp` by 8, clears the attribute and
packs each pixel in place. `0x5a4250` does not change `+0x3c`. `0x5a41f0` is the check
`pixels != 0 and bpp == 32`.

## Flipping

`0x5a4070(top)` sets the origin: no work when bit 5 already matches `top != 0`; otherwise it
reverses the row order into a new buffer and sets bit 5 to `top != 0`. Run-length types fail.

Addresses: `0x5a4070`, `0x5a4083`, `0x5a408d`, `0x5a409c`, `0x5a40e0`, `0x5a4128`, `0x5a415a`, `0x5a4193`
Confidence: verified
Proof: one
Test: `image::image_tga_flip`

`0x5a4083` returns 1 when `top` is set and bit 5 is set, `0x5a408d` when `top` is clear and bit 5
is clear. Types 9 to 11 set `0xf7200083` and return 0 (`0x5a409c`). The row stride is
`bytes(bpp) · width`; row `h − 1 − i` of the output is row `i` of the input
(`0x5a4128` to `0x5a415a`), and the old buffer is freed.

## Building and saving

`0x5a43c0(src, w, h, bpp, attribute, top, flip)` fills a record from a pixel block; `0x5a4810(path)` writes
the record to a file. The screenshot code does both and strips alpha between.

Addresses: `0x5a43c0`, `0x5a43d2`, `0x5a43e6`, `0x5a43f6`, `0x5a4463`, `0x5a4471`, `0x5a44c4`, `0x5a44d9`, `0x5a4810`, `0x5a4860`, `0x5a4890`, `0x5a48c3`, `0x5a4903`, `0x5a4959`, `0x5a4975`, `0x5a498c`, `0x5a4998`, `0x442783`, `0x44277b`, `0x442792`
Confidence: verified
Proof: one
Test: `image::image_tga_init_from_source`

`0x5a43c0` fails (`0x57`) unless the source is non-null, `bpp` is 0x18 or 0x20 and the attribute is 0
or 8. It clears the header words, stores `bpp`, builds the descriptor
(attribute, bit 4 from `flip`, bit 5 from `top`), image type 2, the size, sets `+0x3c` to
`bytes(bpp) · h · w`, copies the pixels, zeroes the two footer offsets and stamps the signature
`TRUEVISION-XFILE.` into the footer (`0x5a44d9`, `0x64a5a0` with 0x12). `0x5a4810` refuses a null path
or no pixels (`0xf7200081`), creates the file (`0x42a460`), writes the 18-byte header, the ID,
the colour map, `+0x3c` pixel bytes and the 26-byte footer, closes the file and deletes it on
any failed write. The screenshot calls `0x5a43c0` with `bpp` 32, attribute 0 and `top` 1, strips alpha
(`0x442783`) and saves (`0x442792`); the stripped image keeps `+0x3c` at four bytes per pixel,
so the file holds `4 · w · h` bytes of pixel data for a 24-bit header.

## Error codes

Failures set a code with `SErrSetLastError` and return 0; a few paths set a code and carry on.

| code | where | meaning |
|---|---|---|
| `0x57` | many | a null argument, or pixels already decoded |
| `0xf7200077` | `0x5a3fa0` | run-length stream longer than the image |
| `0xf7200078`, `0xf720007a`, `0xf720007b` | `0x5a3a9c`, `0x5a3abd`, `0x5a3b04` | image type 0, grey (3, 11), anything else |
| `0xf720007c`, `0xf720007d` | `0x5a3802`, `0x5a420a` | bit depth not 24 less attribute |
| `0xf720007e`, `0xf720007f` | `0x5a3a53`, `0x5a41b7` | no file, no pixels |
| `0xf7200081`, `0xf7200082`, `0xf7200083` | `0x5a4839`, `0x5a429f`, `0x5a409c` | save without pixels, alpha not 8, run-length image |
| `0xf7200084` | `0x5a39dd` | colour-mapped decode without a colour map |
| `0x8720012d`, `0x8720012e` | `0x5a4275`, `0x5a37c5` | warnings: depth lacks the attribute bits, 24 bits with attribute 8 |

Addresses: `0x5a3fa0`, `0x5a3a9c`, `0x5a3abd`, `0x5a3b04`, `0x5a3802`, `0x5a420a`, `0x5a3a53`, `0x5a41b7`, `0x5a4839`, `0x5a429f`, `0x5a409c`, `0x5a39dd`, `0x5a4275`, `0x5a37c5`
Confidence: verified
Proof: one
Test: none

Each address is the `push imm` or `add ecx,imm` that builds the code before `call 0x64e850`.
`0x5a3802` adds `0xf720007c` to 1 when `bpp` is not 16 and to 0 when it is, so a 16-bit image
sets `0xf720007c` and every other depth `0xf720007d`.
