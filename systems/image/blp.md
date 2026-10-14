# image: BLP

The BLP2 reader of `blp.cpp`. A reader record holds the decoded mip chain, a copy of the file's
1172-byte header and a pointer into the file. The texture service opens a file, then either
decodes every level into a chain or takes single levels. Raw level data, and DXT data the device
can take as it is, are not copied.

Roots: `0x5a7aa0`, `0x5a7b40`, `0x5a7f00`, `0x5a8100`, `0x5a8220`, `0x5a8240`, `0x5a83b0`, `0x5a8430`, `0x5a80d0`

## Reader record

A reader record is 0x4b4 bytes. The file header is copied to `+0x04`, so a header field sits at its
file offset plus 4 (the magic at `+4` is file offset 0).

| offset | holds |
|---|---|
| `+0x00` | the decoded chain, 0 until built |
| `+0x04`, `+0x08` | magic `BLP2`, version 1 |
| `+0x0c` | compression: 1 palette, 2 DXT, 3 raw BGRA |
| `+0x0d`, `+0x0e`, `+0x0f` | alpha depth, alpha type, has mips |
| `+0x10`, `+0x14` | width, height |
| `+0x18` | 16 level offsets |
| `+0x58` | 16 level sizes |
| `+0x98` | 256 palette entries, BGRA |
| `+0x498` | the file image, 0 after a chain decode |
| `+0x49c`, `+0x4a0`, `+0x4b0` | 0, level count, temporary buffer of the last level |

Addresses: `0x5a83b0`, `0x5a83c3`, `0x5a83cf`, `0x5a83da`, `0x5a83dc`, `0x5a83e4`, `0x5a83fb`, `0x5a7aa0`, `0x5a8220`
Confidence: verified
Proof: one
Test: none

`0x5a83b0(file)` resets the record, stores the file pointer at `+0x498`, zero at `+0x49c` and
copies `0x125` dwords from the file to `+0x04` (`rep movsd`, `0x5a83da`). It fails unless the
magic is `0x32504c42` (`BLP2`) and `+0x08` is 1. The level count `+0x4a0` is
`0x5a4ac0(w, h)` when `+0x0f` is non-zero (`0x5a8220` returns `+0x0f`) and 1 otherwise. `0x5a7aa0`
clears `+0x498` and frees the chain through `+0x00`.

## Opening a file

`0x5a7b40(path, flag)` reads a whole file into one shared buffer and parses its header. The buffer
is a module static that every record's file pointer aims at, so only the record opened last is
valid.

Addresses: `0x5a7b40`, `0x5a7b79`, `0x5a7b90`, `0x5a7b97`, `0x5a7b9f`, `0x5a7bb7`, `0x5a7bd7`, `0x5a7bf7`, `0x5a7c0e`, `0xc15c40`, `0xc15c44`, `0xc15c48`, `0xc15c4c`
Confidence: verified
Proof: one
Test: none

A null path sets error `0x57`. After `0x5a7aa0` it opens the file (`0x6477c0`) and takes its size
(`0x6487f0`). When the size exceeds both `0xc15c44` and `0xc15c40` it takes the alignment from
`0xc15c4c`, or from `0x59fc50` when that is 0, rounds the size up to a multiple of it and
resizes the buffer (`0x5268b0`). The size goes to `0xc15c44`, the file is read into `0xc15c48`
and closed, and `0x5a83b0` parses it. The functions at `0x59fc50` and `0x5268b0` are not read here.

## Decoding one level

`0x5a7f00(fmt, level, &out, &size)` produces the pixels of a level in the graphics layer's format
`fmt` and returns 1. It needs the file pointer, a valid level and a non-zero level size; the
compression chooses the path.

| compression | result |
|---|---|
| 1 palette | a new buffer, converted by `0x5a8ce0` |
| 2 DXT, `fmt` 0, 1, 7 | the file's own bytes, no copy |
| 2 DXT, `fmt` 2 to 5 | a new buffer, converted through the matrix |
| 2 DXT, `fmt` 6 or above 7 | nothing written, returns 1 |
| 3 raw | the file's own bytes, no copy |

Addresses: `0x5a7f00`, `0x5a7f0a`, `0x5a7f37`, `0x5a7f49`, `0x5a7f53`, `0x5a7f6f`, `0x5a7f97`, `0x5a7fe9`, `0x5a8022`, `0x5a8035`, `0x5a8048`, `0x5a806a`, `0x5a808c`, `0x5a809f`, `0x5a80b8`, `0x5a80c4`, `0x5a7ec0`, `0x5a8100`
Confidence: verified
Proof: one
Test: none

`0x5a7f37` clears the temporary buffer `+0x4b0`; `0x5a7ec0(level)` accepts level 0, or a level below `+0x4a0`
when the record has mips. The data pointer is `fileImage + offsets[level]` and a size of 0 returns 0
(`0x5a7f53`). `dec eax` on the compression picks `0x5a805b` (palette), `0x5a7f8b` (DXT) or
`0x5a8048` (raw). The DXT path indexes the byte table `0x5a80c4`, `00 00 01 01 01 01 02 00`, into
`0x5a80b8` (`0x5a8048`, `0x5a7fa5`, `0x5a804d`). The converting case sizes the level with `0x5a8140`,
allocates it (`0x5a7fe9`, stored at `+0x4b0`) and calls the dispatcher `0x5a4f60` (`0x5a8035`) with
submode 0, the source class of the alpha type (`0x5a80d0`), the destination class of `fmt`, the
level's width and height shifted but not floored at 1 (`0x5a8590` floors them), the source row bytes (`0x5a62c0` of the
full width, shifted by the level) and the destination row bytes. The palette case allocates the
size and calls `0x5a8ce0(fmt, level, out, data)`. `0x5a8100(level)` frees the temporary buffer at `+0x4b0`.

## Decoding the chain

`0x5a8240(fmt, &chain, start)` decodes every level from `start` and copies each into a chain,
allocating the chain when `*chain` is null and using the caller's otherwise.

Addresses: `0x5a8240`, `0x5a824f`, `0x5a82a7`, `0x5a82e5`, `0x5a8313`, `0x5a8326`, `0x5a8349`, `0x5a8379`, `0x5a8387`, `0x5a8392`, `0x5a7ec0`
Confidence: verified
Proof: one
Test: none

It fails on an invalid `start` (`0x5a7ec0`). With `*chain` null it calls `0x5a4af0` for the
level dimensions `(w >> start, h >> start)` floored at 1 (the file and line `0x85c9d0` and 0x228);
otherwise `0x5a4bb0` fills the table of the supplied block. For each level `i` from `start` it calls
`0x5a7f00`, copies `0x5a4a00(i, w, h, fmt)` bytes into chain entry `i − start`, and frees the temporary
buffer
(`0x5a8100`). A failed level frees a chain this call allocated, clears `*chain` and returns 0.

## Level pointers without copies

`0x5a8430(fmt, &chain, start, flag)` is the variant that can point the chain at the file instead of
copying. When the caller supplies a chain, `flag` is non-zero and the data is DXT for a format outside
2 to 5 or raw, it sets entry `i` to `fileImage + offsets[i]` for each level whose size is non-zero.

Addresses: `0x5a8430`, `0x5a844c`, `0x5a848c`, `0x5a84a1`, `0x5a84ab`, `0x5a84c1`, `0x5a84c5`, `0x5a84fd`, `0x5a8524`, `0x5a8529`, `0x5a853f`, `0x5a8555`, `0x5a8570`, `0x5a8585`, `0x5a8590`, `0x5a8615`, `0x5a8634`, `0x5a86af`, `0x5a86f0`, `0x5a86fc`, `0x5a8710`
Confidence: verified
Proof: one
Test: none

A null `*chain` is allocated by `0x5a4af0` at `0x5a848c` and goes straight to the level loop at
`0x5a8502`. Otherwise `0x5a84a1` loads the compression: for 2 the formats 4, 5, 3 and 2 go to the
populate at `0x5a84d0` and any other format falls to `0x5a84c5`; compression 3 falls to
`0x5a84c5` too; any other compression populates. `0x5a84c5` jumps to `0x5a8555` when `flag` is non-zero. That loop reads the size at
`+0x58 + 4i`, stops at the first zero (the test is made before the first and after each write), and
stores `fileImage + [+0x18 + 4i]` into the table. The table is not sized by `0x5a8430` in this
path. Otherwise `0x5a84fd` populates a table and `0x5a8590` converts each level into it: for
palette data `0x5a86af` sizes the level (`0x5a8140`) and calls `0x5a8ce0`; for DXT data the byte
table `0x5a86fc` (`00 00 01 01 01 01 02 00`) and `0x5a86f0` pick a raw copy (`0x5a8615`, the level's
stored size), the matrix conversion (`0x5a8634`) or nothing; raw data copies the stored size. Afterwards
`0x5a853f` clears `+0x498`: the file pointer is gone, even when the chain points into the file.

## DXT level span

A DXT level that stays in the file's own format is the bytes of its dimensions, not of its stored size:
`bpp · max(4, w) · max(4, h) / 8` bytes from `fileImage + offsets[level]`, with `bpp` 4 for DXT1 and 8 for
DXT3 and DXT5. The stored size is read only as a test for non-zero, except by the stage-1 copy.

Addresses: `0x5a7f53`, `0x5a8048`, `0x5a8326`, `0x5a8555`, `0x5a8558`, `0x5a8615`, `0x5a4a00`, `0x5a5740`, `0x5a5780`, `0x5a0cdf`, `0x59f416`
Confidence: verified
Proof: one
Test: `image::image_level_size`, `image::image_dxt1_block_copy`, `image::image_dxt35_block_copy`

`0x5a7f00` hands the pointer on without a size (`0x5a8048`) and tests the stored size against zero (`0x5a7f53`);
`0x5a8430` writes pointers while the stored size is non-zero (`0x5a8558`). `0x5a8240` copies `0x5a4a00(level, w, h, fmt)`
bytes (`0x5a8326`), which for formats 0, 1 and 7 clamps both dimensions to 4 and multiplies by the bits per pixel
(4, 8, 8). The block copies `0x5a5740` and `0x5a5780` move `max(w, 4) · max(h, 4) · 4 / 8` and `· 8 / 8`
bytes from the dimensions in `ecx` (`0x5a5740`: `imul; shl ecx,2; shr ecx,3`). The graphics layer's two
surface copies call the matrix at `0x5a0cdf` and `0x59f416`, whose DXT3 and DXT5 cells are `0x5a5780`. Only the stage-1 copy `0x5a8615`
(formats 0, 1 and 7) moves the stored size.

## Format classes

`0x5a80d0(index)` returns the conversion class of a format or alpha-type number by indexing the
ten dwords at `0x85c8c8`: 5, 6, 1, 3, 2, 4, 0, 7, 0, 0.

Addresses: `0x5a80d0`, `0x85c8c8`
Confidence: verified
Proof: one
Test: `image::image_format_lut`

The classes are 1 BGRA8888, 2 ARGB4444, 3 ARGB1555, 4 RGB565, 5 DXT1, 6 DXT3, 7 DXT5 and 8 any
other 16-bit format. The table is indexed by two different numbers: the file's alpha type 0, 1 and
7 (the DXT variants) and the graphics layer's format. The function does not check the index.
