# font: fonts, strings and batches

The `GxuFont` functions the rest of the client calls. A font is created from a face file name, a
set of flags and a size; a string is created in a font with a text, a size, a box and a
justification; strings are drawn through a batch. Every function checks its arguments and sets
error `0x57` on a bad one.

Roots: `0x5cd6d0`, `0x5cd830`, `0x5cd950`, `0x5ccb10`, `0x5ccb40`, `0x5cd650`, `0x5cdef0`, `0x5cdf40`, `0x5cae90`, `0x5ca1d0`, `0x5ca760`, `0x5caa20`, `0x5cd6a0`

## Font object

A font is 0x250 bytes. Its fields, by offset:

| offset | holds |
|---|---|
| `+0x00`, `+0x04` | registry link: next and previous font |
| `+0x08`, `+0x0c` | link offset (the strings' link is at `+0x10`) and head of the font's string list |
| `+0x14` | the glyph hash (its vtable `0x80a8cc`); buckets `+0x30`, mask `+0x38` |
| `+0x3c` | the kerning hash (vtable `0x80a8bc`); buckets `+0x58`, mask `+0x60` |
| `+0x64`, `+0x68` | link offset and head of the glyph recency list |
| `+0x70` | the face handle |
| `+0x74` | the face file name, 0x104 bytes |
| `+0x178`, `+0x17c` | atlas cell size; baseline ascender in pixels |
| `+0x180`, `+0x184` | flags; the requested size |
| `+0x188` | advance adjustment ratio |
| `+0x18c` | eight atlas pages of 0x18 bytes |
| `+0x24c` | the rendered pixel size |

Addresses: `0x5c1ae0`, `0x5ca1d0`, `0x5ca920`, `0x5caa20`, `0x5cae90`, `0x5c1bf0`, `0x80a8cc`, `0x80a8bc`, `0xc2b978`
Confidence: verified
Proof: one
Test: none

The flags are bit 0 outline, bit 1 monochrome, bit 2 a page-texture variant that only the name
font of `playername` sets, and bit 3 thick outline. `0x5cae90` returns `[font + 0x24c]`.
`0x5c1bf0` returns `[font + 0x180]` or 0. The offsets are those the functions of this doc load
and store: `0x5ca1d0` stores `+0x180`, `+0x184` and the name (`0x5ca1ee`, `0x5ca1f1`), `0x5c1ae0` links the
font into the registry at `0xc2b978`, and the glyph code reads `+0x30`, `+0x38`, `+0x64`, `+0x68`,
`+0x178`, `+0x18c`.

## Creating a font

`GxuFontCreateFont(name, &font, size, flags)` makes a font and returns whether its face loaded.
The name must be non-empty, the size strictly between 0 and 1 (a fraction of the screen height).
Flag bit 3 also sets bit 0.

Addresses: `0x5c1ae0`, `0x5c1af3`, `0x5c1b03`, `0x5c1b33`, `0x5c1b42`, `0x5c1b8e`, `0x5c1ba5`, `0x5c1bc5`, `0x5ca1d0`, `0x5ca1fc`, `0x5ca21e`, `0x5ca232`, `0x5ca920`, `0x5ca740`, `0x5ca760`, `0x5ca730`, `0x85f554`
Confidence: verified
Proof: one
Test: none

`0x5c1af3` and `0x5c1b03` compare the size against 1.0 and 0.0 (`fcomp`, `test ah,5` and
`test ah,0x41`); a size of 1 or more, 0 or less, or NaN reaches error `0x57`. The allocation is 0x250
bytes (`0x5c1b33`). The font is linked at the head of the registry, flag bit 3 is turned into bit 0
too (`or eax,1`, `0x5c1b9a`), and `0x5ca1d0(name, flags, size)` runs. It copies the name to `+0x74`
(`SStrPrintf` with `"%s"`, 0x104 bytes), the size to `+0x184`, clears the caches
(`0x5ca740`, which frees the old face and calls `0x5ca760`), stores the flags, loads the face
(`0x5d0060(name, 0xc2b9a8)`) and, when it loads, calls the metrics `0x5ca030`. On failure `0x5c1bb2`
destroys and frees the font and `*font` is 0.

## Destroying a font

`0x5c1c00(&font)` destroys the font that a pointer names and clears the pointer. `0x5ca760`
empties a font: it releases the eight atlas pages and their cell packers, drains the glyph hash
and the recency list, and frees each cached glyph.

Addresses: `0x5c1c00`, `0x5c1c0c`, `0x5caa20`, `0x5ca760`, `0x5ca77e`, `0x5ca788`, `0x5ca81f`, `0x5ca8a0`, `0x5c19e0`
Confidence: verified
Proof: one
Test: none

`0x5c1c00` runs `0x5caa20` on a non-null `*font`, frees the 0x250 bytes (tag `0x85f554`) and
stores 0. `0x5ca760` visits the eight page records from `+0x198` in steps of 0x18: it calls
`0x58ad60` on a non-null texture at `page+0`, `0x5cb0e0` on the page's packer and zeroes three
words; then pops every entry of the glyph hash and of the recency list at `+0x6c` (`0x5ca7f9`,
frees of tag `0x85f674`) and of the kerning hash. `0x5ca8a0` unlinks a glyph from its two lists.
The internals of `0x5caa20`, `0x5cb0e0` and `0x58ad60` are not read here.

## Creating a string

`GxuFontCreateString` makes a string in a font. Its arguments are the font in `ecx`, the text in
`edx`, then `size`, a pointer to the position (3 floats), the box width and height, the line
spacing, a pointer for the handle, the vertical and horizontal justification (each 0 to 2), the
flags and a pointer to the colour.

Addresses: `0x5c1c30`, `0x5c1c44`, `0x5c1c57`, `0x5c1c7c`, `0x5c1c9c`, `0x5c1ca1`, `0x5c1cd1`, `0x5c1cdf`, `0x5cd6d0`, `0x5cd830`
Confidence: verified
Proof: one
Test: none

The call fails (error `0x57`, result 0) when the font or text is null; when the size is 0 and flag
bit 2 is clear; when the box width or height is 0; and when either justification is 3 or more. It then takes
a string from the pool (`0x5cd830`, argument 1), calls `0x5cd6d0` with the flags with bit 0
cleared and stores the handle; if that fails it destroys the string (`0x5c1cdf`). The result is
what `0x5cd6d0` returns.

## Destroying a string

`GxuFontDestroyString(&string)` unlinks the string from the font's list and recycles it.

Addresses: `0x5c1d00`, `0x5c1d45`, `0x5cd950`, `0x5cd9e2`, `0x5cdef0`
Confidence: verified
Proof: one
Test: none

`0x5c1d00` returns if `*string` is 0, unlinks the record from the global string list (the
standard explicit-list unlink on its first two words), calls `0x5cd950` and stores 0. `0x5cd950`
unlinks the record from its font's list (link at `+0x10`), puts it at the head of the pool list
`0xc2b98c` and finishes through the invalidation `0x5cdef0`.

## String pool

Strings come from a pool: `0x5cd830(flag)` takes the record at `[0xc2b990]` from the pool list
and puts it at the head of the global string list `0xc2b998`, or, with an empty pool, allocates a
record of 0xc4 bytes (tag `0x85f56c`), constructs it with `0x5cda00` and, when `flag` is non-zero,
links it at the head of the global list. The record is the string object.

Addresses: `0x5cd830`, `0x5cd835`, `0x5cd88f`, `0x5cd8a0`, `0x5cd8b2`, `0x5cd914`, `0x5cd91a`, `0x5cd928`, `0x5cda00`, `0xc2b990`, `0xc2b998`, `0xc2b99c`
Confidence: verified
Proof: one
Test: none

For the allocation path the flag becomes 2 or 0 (`neg ecx; sbb ecx,ecx; and ecx,2`, `0x5cd892` to
`0x5cd89d`) and a 0 skips the linking. A pooled record is always linked. The constructor
`0x5cda00` is not read here.

## Batches

A batch groups the strings drawn in one flush. `GxuFontCreateBatch(a, b)` takes a batch (0x34 bytes) from the free list
`0xc2b844` or allocates one and sets bit 0 of its flags word at `+8` from `a` (world space) and bit 1 from
`b` (empty after drawing). `0x5c1e60(batch, string)` adds a string, `0x5c1ef0(batch)` draws it, `0x5c1f00` clears it
and `0x5c1f70` returns it to the free list.

Addresses: `0x5c1d60`, `0x5c1dbc`, `0x5c1e05`, `0x5c1e2a`, `0x5c1e3f`, `0x5c1e60`, `0x5c1e64`, `0x5c1e69`, `0x5c1ef0`, `0x5c1ef4`, `0x5c1f00`, `0x5c1f70`, `0x5c8e40`, `0xc2b844`, `0xc2b848`, `0x80a884`, `0xc2b9b0`
Confidence: verified
Proof: one
Test: none

`0x5c1d60` pops a batch from the free list `0xc2b848` or allocates 0x34 bytes (tag `0x85f534`), stores the vtable `0x80a884` at
`+0x0c` and sets the mask at `+0x30` to −1. `a` and `b` set or clear bit 0 and bit 1 of `+8` (`0x5c1e2a`, `0x5c1e3f`).
`0x5c1e60` returns 0 when either argument is 0, otherwise calls `0x5c8e40(string)` on the batch
and returns 1. `0x5c8e40` looks the string's font (`[+0x44]`) up in the batch's run table by
two key words, makes a run when none exists and puts the string at the head of the run's list. `0x5c1ef0` jumps to `0x5c8b70` for a non-null batch.
The bodies of `0x5c1f00`, `0x5c1f70` and the table functions are not read here. The name font of `playername` makes
its batch (1, 1) and the damage text font its own (0, 0).

## Drawing a batch

`0x5c8b70` draws every run of a batch. A world-space batch draws under the device's current matrices; any other sets the
view to identity and a pixel orthographic projection first. A batch with bit 1 empties each run afterwards.

Addresses: `0x5c8b70`, `0x5c8b79`, `0x5c8c6d`, `0x5c8c72`, `0x5c8cb2`, `0x5c8cc2`, `0x5c8d97`, `0x5c8d9c`, `0x5c8dbb`, `0x5c8ddb`, `0x5c8de0`, `0x5c8df6`, `0x5c8e11`, `0x5c8e18`, `0x5c8e20`, `0x5c8e28`, `0x5c8fe0`, `0xc2b9dc`, `0xc2b9b0`, `0x5c8f40`, `0x5c92f0`, `0x5c9330`, `0x5c9380`, `0x5cfc50`, `0xc2b9d4`, `0xc2b9d8`
Confidence: verified
Proof: one
Test: none

`0x5c8b70` returns when `0xc2b9b0` is set. It saves the device projection (`0x58b090`) and view (`0x58b0b0`) and sets the
render states 0xe, 0xf and 0x14 to 0 and state 7 to 2. With bit 0 of `+8` it stores 1 in `0xc2b9dc` and sets
states 0x10, 0x12 and 8 to 1. Otherwise it sets an identity view (`0x58b050`), calls `0x5c70b0` (the pixel
projection), sets states 0x10 and 0x12 to 0 and draws nothing when `0x5c70b0` returns 0. Then it passes each run
(from `+0x18` by the link offset `+0x10`) to `0x5c8fe0`; with bit 1 it pops every string of the run (`0x5c9730`).
It clears `0xc2b9dc` and restores the view and projection.

## Drawing a run

`0x5c8fe0` draws the strings of one run: it lays them out, uploads the font's dirty atlas pages, and draws page by
page, strings in list order, a line's shadow quads before its main quads.

Addresses: `0x5c8fe0`, `0x5c8fe7`, `0x5c901b`, `0x5c903a`, `0x5c9056`, `0x5c9098`, `0x5c90db`, `0x5c9108`, `0x5c9131`, `0x5c916f`, `0x5c91a2`, `0x5c8f40`, `0x5c8f65`, `0x5c8f79`, `0x5c8f94`, `0x5c8f9f`, `0x5c8fc5`, `0x5c8fcc`, `0x5c92f0`, `0x5c9330`, `0x5c9380`, `0x5cfc50`, `0x5cfc70`, `0xc2b9d4`, `0xc2b9d8`
Confidence: verified
Proof: one
Test: none

`0x5c8fe0` returns if the quad index buffer `0xc2b9d4` does not exist. It lays out each string (`0x5cd6a0`) and uploads each dirty
page of the run's font (`0x5cfc50`: the whole 256 by 256 rectangle, then the page's dirty byte is cleared). It makes a
vertex buffer of 2048 vertices of 0x18 bytes (`0x58a140`). For each of the 8 pages with a texture it sets the texture (state
0x17) and, for each string with a line buffer for the page, emits the line through `0x5ce090` and `0x5ce0c0` into the
free space. `0x5c8f40` flushes when the space is gone and at the end of a page: it unlocks the buffer, draws `count / 4 · 6`
indices as a triangle list (`0x58a830`, type 3) and locks again. The shared index buffer is made by `0x5c92f0` (0x1800 bytes), filled by
`0x5c9330` at the first flush with `{k, k+2, k+1, k+2, k+3, k+1}` for each of 512 quads, and freed by `0x5c9380`.

## String setters

Three small functions change a string. `0x5ccb10(&pos)` stores a new position (3 floats at `+0x20`)
and recomputes the anchor with `0x5cdf70`. `0x5ccb40(&color)` stores the main colour at `+0x2c`
and lowers the shadow alpha byte `+0x33` to the colour's alpha. `0x5cd650(&offset, &color)` stores the
shadow colour and offsets.

Addresses: `0x5c2750`, `0x5c2770`, `0x5c27a0`, `0x5ccb10`, `0x5ccb40`, `0x5ccb48`, `0x5ccb62`, `0x5cd650`, `0x5cd658`, `0x5cd663`, `0x5cd667`, `0x5cd678`, `0x5cd687`, `0x5cdef0`, `0x5cdf70`
Confidence: verified
Proof: one
Test: none

`0x5c2750`, `0x5c2770` and `0x5c27a0` check the string pointer and call `0x5ccb40`, `0x5ccb10` and
`0x5cd650`. `0x5ccb40` returns when the colour is unchanged; otherwise it stores it and
`+0x33 = min(color.alpha, +0x33)`, then invalidates the layout unless flag `+0x5c` bit 3 is
set with bit 5 clear and the string has no layout (`[+0x90] == 0`). `0x5cd650` stores the shadow colour at `+0x30`, `+0x33 = min(shadowAlpha, +0x2f)`, the offsets at `+0x34`
and `+0x38`, sets flag bit 0 and invalidates when bit 5 is set. The layout is rebuilt on the next draw.

## Layout lifetime

A string's layout is kept for use by the next draw and dropped after 31 frames of disuse. The
frame pass `0x5c2ce0` walks the global string list and calls `0x5cdf40` on each string.

Addresses: `0x5c2ce0`, `0x5c2d0b`, `0x5c2d15`, `0x5cdf40`, `0x5cdf4a`, `0x5cdf5e`, `0x5cdef0`, `0x5cd6a0`, `0x5cd6b9`
Confidence: verified
Proof: one
Test: none

`0x5cdf40` does nothing for a string with no layout (`[+0x9c] == 0`); otherwise it adds 1 to
the frame counter at `+0xc0` and, when the old value is above 30, calls `0x5cdef0`. `0x5cdef0`
clears `+0x80`, `+0x90` and `+0x9c`, sets `+0x68` and `+0x6c` to −1 and frees the eight line
buffers at `+0xa0`. `0x5cd6a0` rebuilds the layout (`0x5cd3f0`, then `0x5cdc20`), sets the counter to 0
and returns whether `+0x9c` is non-zero. After the list walk `0x5c2ce0` returns at once while
`0xc2b9b0` is 0 (`0x5c2d1c`), which it always is.

## Frame rate bar

The rest of the frame pass `0x5c2ce0` is a frame-rate bar. It is skipped while the debug flag `0xc2b9b0` is 0,
and no live code sets the flag, so the bar is never drawn.

Addresses: `0x5c2ce0`, `0x5c2d1c`, `0x5c2d22`, `0x5c2d54`, `0x5c2d6c`, `0x5c3440`, `0xc2a9b8`, `0xc2b6b8`, `0xc2b84c`, `0xc2b9b0`, `0x42c010`
Confidence: verified
Proof: one
Test: none

`0x5c2d1c` leaves when `[0xc2b9b0]` is 0. Otherwise `0x5c2d22` reads the clock (`0x42c010`), stores the
difference to `0xc2b6b8` in seconds (`× 0.001`, a float) in a ring of 30 floats at `0xc2a9b8`
(`0x5c2d54`; the index at `0xc2b84c` wraps at 30) and calls `0x5c3440`, which sums the 30 floats, multiplies by `1/30` and
returns 100.0 for a mean under 0.01 second and `1.0 / mean` otherwise. The result is split into tens and ones by
`idiv 10`, the matching digit quads are copied into the work buffer and drawn under a unit orthographic
projection. The flag is cleared at shutdown (`0x5c1aca`) and written otherwise only by the
unreached `0x5c3490`.
