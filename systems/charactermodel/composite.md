# charactermodel: composite

The body texture of a character is a 256 × 256 RGB565 texture cut into ten fixed tiles. Each tile has a handler that blits the skin and then each equipment or customization cell over it. This doc covers the tile table, the
handlers, the two blit helpers and the three pixel kernels. The cells come from `customization` and `equipment`, and `component#update-driver` calls the handlers and uploads the tiles.

Roots: none

## Tile table

`0x475c50` stores ten rectangles at `0xb42450`, 16 bytes each: x, y, width and height in pixels. Groups 0 to 2 and 8 and 9 are the left column, groups 3 to 7 the right.

| group | x, y | width × height | tile |
|---|---|---|---|
| 0 | 0, 0 | 128 × 64 | upper arm |
| 1 | 0, 64 | 128 × 64 | lower arm |
| 2 | 0, 128 | 128 × 32 | hand |
| 3 | 128, 0 | 128 × 64 | upper torso |
| 4 | 128, 64 | 128 × 32 | lower torso |
| 5 | 128, 96 | 128 × 64 | upper leg |
| 6 | 128, 160 | 128 × 64 | lower leg |
| 7 | 128, 224 | 128 × 32 | foot |
| 8 | 0, 160 | 128 × 32 | face, upper |
| 9 | 0, 192 | 128 × 64 | face, lower |

Addresses: `0x475c50`, `0x475c40`, `0x475c58`, `0x475c5e`, `0x475d44`, `0xb42450`
Confidence: verified
Proof: one
Test: none

The stores are constants (`0x475c53 mov eax,0x80` for the width, `0x475c5e mov ecx,0x40`), not computed from any image. Group `g` is at `0xb42450 + 16·g`; group 9's height is stored at `0xb424ec` (`0x475d82`). `0x475c40` calls `0x475c50` as a static
initializer.

## Group handlers

Ten handlers sit in a table at `0xb422f4`, one per group, filled by `0x476020`. Each blits the skin cell onto its tile and then its overlay cells in column order, so a later column paints over an earlier one. Groups 0 to 7 take row `g`
of the equipment cells, with 2, 7, 1, 5, 5, 3, 6 and 1 columns. Groups 3 and 5 first draw an underwear cell when the leading equipment cells of their row are empty. Groups 8 and 9 draw the skin only when the skin colour is not below the base count, then the face, facial hair and hair cells.

Addresses: `0x477170`, `0x4771b0`, `0x4771f0`, `0x477210`, `0x477280`, `0x4772f0`, `0x477360`, `0x4773a0`, `0x477400`, `0x477440`, `0x4760b6`, `0xb422f4`, `0x4772fc`, `0x47730c`, `0x477316`, `0x477320`, `0x4773ac`, `0x4773bc`, `0x4773c6`, `0x477320`
Confidence: verified
Proof: one
Test: none

`0x4760b6` to `0x476110` store the handlers: slot 0 `0x477170`, 1 `0x4771b0`, 2 `0x4771f0`, 3 `0x4772f0`, 4 `0x477360`, 5 `0x4773a0`, 6 `0x477400`, 7 `0x477440`, 8 `0x477210`, 9 `0x477280`. Group `g` starts with `0x477070(g)`. Its cells are the loop over `cc + 0x228 + 0x40·g`: 2, 7, 1, 5, 5, 3, 6, 1 cells (the `ebx` counts at `0x477182`, `0x4771c2`, and so on). The underwear gate of group 3 tests
`+0x2e8`, `+0x2ec` and `+0x2f0` (`0x4772fc`, `0x47730c`, `0x477316`) and then draws `+0x20c`; group 5's tests `+0x368` and `+0x36c` and draws `+0x208` (`0x4773c6`). Groups 8 and 9 compare `+0x24` with `[recptr + 4]` (`0x47722b`, `0x47729b`) and draw `+0x1ac`, `+0x1cc`, `+0x1f0` (group 8) or `+0x1a8`, `+0x1c8`, `+0x1ec` (group 9).

## Blit helpers

`0x477070(group)` blits the skin cell `+0x188` onto the tile of `group`: source and destination positions are both the tile's top-left, and the size is the tile's. `0x4770f0(group, cell)` blits an overlay cell with the source position at 0, 0, the destination at the tile's top-left
and the tile's size. Both ask for the cell's data with `0x47b120` and its descriptor with `0x47b150` (forced), skip a cell with no data, and hand the data, the descriptor's format dword, both positions, the size and the descriptor to `0x477460`, which calls the kernel for the format: 0 replace, 1 key, any other value blend.

Addresses: `0x477070`, `0x4770f0`, `0x477460`, `0x47b120`, `0x47b150`, `0x4770d7`, `0x477155`, `0x44b430`, `0x44ae70`, `0x4498c4`
Confidence: verified
Proof: one
Test: none

`0x477070` pushes the tile address twice (`0x4770ce`, `0x4770cf`) as destination and source position, so a skin source is read at the tile's own place in the image; `0x4770f0` builds a zero source position on its frame (`0x477155 mov [ebp-8],esi`)
and the tile's top-left as destination (`0x477147`). `0x47b150` fills the cell's six descriptor dwords once, through `0x44b430`: width and height (image `+0x144`, `+0x148`), image `+0x118`, the mip count `1 + floor(log2(max(w, h)))` from `0x44ae70`, bit 0 of image `+0x110` and the format, image `+0x11c`, which the BLP loader sets from the header's alpha depth (`0x4498c4`).
`0x477460` copies the three position and size pairs to its frame so the kernels can halve them, then calls by format: 0 `0x477c80`, 1 `0x477db0`, anything else `0x477f20`.

## Replace kernel

`0x477c80(source mips, target mips, dest position, source position, size, descriptor)` copies a rectangle of 16-bit pixels with no blending. For each mip level from 0 to the descriptor's mip count it copies `size.h` rows of `size.w` pixels from the source level at its position to the target level at its position, then halves the
pitches, the positions and the size, keeping the size at least 1. The target pitch starts at 0x200 bytes and the source pitch at twice the descriptor's width.

Addresses: `0x477c80`, `0x477cb2`, `0x477cdd`, `0x477cf3`, `0x477d11`, `0x477d38`, `0x477d71`, `0x477da8`
Confidence: verified
Proof: one
Test: none

`0x477c9e mov [ebp-8],0x200` sets the target pitch and `0x477c9a shl ecx,1` the source pitch. `0x477cb2 jbe` leaves when the mip count is 0. `0x477cd6 imul edx,[ebp-8]` forms the target row address and `0x477cea imul edx,ecx` the source's. The row copy is the word loop `0x477d11` to `0x477d1f`.
The level step halves the size with a floor of 1 (`0x477d38` to `0x477d58`), the positions with `sar` (`0x477d65`, `0x477d69`, `0x477d76`, `0x477d7d`), and both pitches with `shr` (`0x477d63`, `0x477d88`). The level count is the source's: nothing compares it with the target's.

## Key kernel

`0x477db0` has the same geometry but copies a pixel only when its coverage is not 0. The coverage is a 2-bit plane that starts after the source level's colour rows, at `source level + size.h · source pitch`, with the pixels counted
continuously from the first one of the rectangle: the pixel number `i` has its coverage in byte `i >> 2`, selected by the mask `{3, 0xc, 0x30, 0xc0}[i & 3]`. A source whose width differs from the rectangle's width has its coverage read at the wrong pixels.

Addresses: `0x477db0`, `0x477e31`, `0x477e37`, `0x477e65`, `0x477e68`, `0x477e70`, `0x838ae4`
Confidence: verified
Proof: one
Test: none

`0x477e2e imul eax,edi` forms `size.h · source pitch`, `0x477e37 add edx,eax` the plane address (`0x477e40` stores it). The index in `esi` counts from 0 per level and is never reset between rows (`0x477e3c`, then `0x477e88 inc esi`). `0x477e65 shr edx,2` and `0x477e68 mov dl,[edx+ebx]` fetch the
byte, `0x477e6d and edi,3` and `0x477e70 test [edi+0x838ae4],dl` apply the mask table, whose bytes are `03 0c 30 c0`. `0x477e76 je` skips a pixel whose masked bits are 0.

## Blend kernel

`0x477f20` reads the same plane, shifts the masked bits down by `{0, 2, 4, 6}[i & 3]` to a level `v` of 0 to 3 and writes per pixel by level: 0 keeps the target, 3 copies the source, and 1 or 2 blend per channel. The blend of a channel is
`(source · SW + target · DW) >> 16`, masked to the channel, with `SW = {0, 0x5555, 0xaaaa, 0x10000}[v]` and `DW = SW[3 - v]`, on the green (`0x7e0`), blue (`0x1f`) and red (`0xf800`) fields of the 16-bit pixel.

Addresses: `0x477f20`, `0x477fe8`, `0x477ff1`, `0x477ff8`, `0x478001`, `0x47800b`, `0x478010`, `0x478025`, `0x47802c`, `0x478032`, `0x838ae8`, `0x838af8`, `0x838b04`
Confidence: verified
Proof: one
Test: none

`0x477ff8 mov cl,[ecx*4+0x838ae8]` reads the shift from a dword table `0, 2, 4, 6`, `0x478001 shr edx,cl` applies it, and `0x478005 jbe` skips level 0. `0x47800b cmp edx,2; jbe` sends 1 and 2 to the blend and `0x478010 cmp edx,3; jne` skips other values, while 3 copies at `0x478019`.
`0x47802c mov edx,[ecx+0x838af8]` is `SW` and `0x478032 mov edi,0x838b04; sub edi,ecx` points `DW` at `0x838b04 - 4·v`, which is `SW[3 - v]`. The multiplies are `0x47804c`, `0x478055`, `0x478064`, `0x47806f`, `0x478072`, `0x47807c`, and each field is shifted right 16 and masked at `0x478082` to `0x4780a0`.

## Oversized sources

A source larger than its tile is cropped and never scaled. Each kernel reads the top-left `size.w × size.h` block of every source level from the source position, and an overlay's position is 0, 0. A key or blend source also reads its coverage plane for the tile's
size, so its layout is right only for a source authored at the tile's width. The loop runs for the source's level count and indexes the target's table by the same level number. The texture service halves a source whose side exceeds the device's texture size limit before the blit.

Addresses: `0x477c80`, `0x477db0`, `0x477f20`, `0x448b60`, `0x4498f1`, `0x58a230`, `0x477d94`
Confidence: verified
Proof: one
Test: none

The kernels step the level through the source's mip count (`0x477d90 mov edi,[edx+0xc]`, `0x477d94 cmp eax,edi`) and take the destination row from `dstPos` and the source row from `srcPos`, with no scaling. `0x4498f1 call 0x448b60` halves the loaded width and height while either exceeds the device limit, the dword at `+0x60` of the device caps from `0x58a230`
(`0x448ba7`, `0x448bab`, `0x448baf`), counting the halvings. The routine applies no floor at 256 to that limit.

## Source planes

The palette converter `0x5a9a80` writes the layout the key and blend kernels read: the 16-bit colour rows, then the 2-bit coverage plane. A colour is the palette entry rounded to the nearest 5, 6 and 5 bit levels with the error of each pixel spread to its neighbours in sixteenths, 7 to the next pixel and 3, 5 and 1 to the next row. The coverage of a pixel is the top two bits of an 8-bit alpha, the top two bits of each 4-bit alpha nibble, or a 1-bit alpha as 0 or 2. Four pixels share a byte, in the slots with masks `03 0c 30 c0`.

Addresses: `0x5a9a80`, `0x5a8ce0`, `0x5a9b6b`, `0x5a9b71`, `0x5a9b77`, `0x5a9c04`, `0x5a9cd2`, `0x5a9d1e`, `0x5a9d57`, `0x85c9b0`, `0x85c9c8`
Confidence: verified
Proof: one
Test: none

The rounding adds `0x40000` to red and blue and `0x20000` to green (`0x5a9b71`, `0x5a9b77`, `0x5a9b6b`) and masks, shifting by 19, 18 and 19 bits, and each result is clamped to 31, 63 and 31. The error scale is `sar 4` of the carried error (`0x5a9b44`), the weights 7 (`0x5a9c04`), 5 and 3 (`0x5a9c19` to `0x5a9c36`) and the stored 1.
`0x5a9cd2 shr bl,6` takes the 8-bit alpha's top bits; the 4-bit branch (`0x5a9cf8`) shifts by 2 or 6 from `0x85c9c8` and the 1-bit branch (`0x5a9d57`) shifts left once. The mask bytes at `0x85c9b0` are `03 0c 30 c0`, the kernels' table `0x838ae4`. `0x5a8ce0` calls this converter among five others.
