# terrain: files

The map's files as the client reads them: the WDT that says which tiles exist, the WDL of
low-resolution heights, and each tile's ADT, whose header points at the chunk table, the
texture names and the doodad and object placements. The client reads every file by fixed sizes
and offsets and never checks a record's tag; the record names used here are the files' own.

The map load is `CMap::Load` (`0x6941f0`) in the map loading object file, `0x694070` through
`0x697ac0`, with the WDT and WDL readers and the placement builders; the tile's file is read by
the area object file, `0x6c1ba0` through `0x6c2720`. A tile is held by an area object of 1,704
bytes from the `WAREA` pool, keyed by its index row·64 + column; the class has no name in the
image. Coordinates follow the client: a tile's row gives world x and its column world y, both
falling from 17066.666 at index 0.

Roots: `0x694130`, `0x694180`

## Map load

`CMap::Load` (`0x6941f0`) loads the map named in ecx with the map id in edx. It sets the shader
flags from the world flags and the shaders' availability, builds the map directory
`World\Maps\<name>`, the name, and the WDT path `<dir>\<name>.wdt`, makes the map's light
object, reads the WDL, reports 0.1 progress, reads the WDT, reports 0.25, runs one map update and
loads the map's lights. While it runs, `0x86a2d0` is 1 and tile files are read at once rather
than queued.

Addresses: `0x6941f0`, `0x6944a0`, `0x694760`, `0x698390`, `0x6c09b0`, `0x86a2d0`, `0xc91f78`, `0xc961a0`, `0xc960a0`, `0x86a2cc`, `0x694430`, `0x694480`
Confidence: verified
Proof: one
Test: none

`0x694396` and the next two `SStrCopy` calls write `0xc91f78` and `0xc961a0`; `SStrPrintf` with
`%s\%s.wdt` writes `0xc960a0`. The map id is stored at `0x86a2cc`. `0x694430` sets `0x86a2d0`
to 1 and `0x694480` clears it. The progress callback `0xc962e8`, when set, gets 0x3dcccccd and
0x3e800000. `0x6c09b0(1, 0)` returns the light object kept in `0xc89ef0`.

## WDT

`CMap::LoadWdt` (`0x694760`) reads the WDT in fixed pieces: an 8-byte header, the 4-byte version
into `0xc96314`, a header, the 32-byte map header into `0xc9e35c`, a header, and the 32,768-byte
tile table into `0xc89f50`, 64×64 entries of two dwords, index row·64 + column. Bit 0 of an
entry's first dword means the tile has an ADT. When the map header's first dword has bit 0 set,
the map is one object: the reader takes a header, the object's name of the size that header
gives, a header and one 64-byte placement, and places it with the unique id taken from a counter
that counts down.

Addresses: `0x694760`, `0x694130`, `0xc96314`, `0xc9e35c`, `0xc89f50`, `0xc9a320`, `0x694810`, `0x694893`, `0x6948c5`, `0x694904`
Confidence: verified
Proof: one
Test: none

A failed open is fatal through `CMap::LoadWdt() failed %s`. The reads are `0x648460` calls of 8,
4, 8, 0x20, 8 and 0x8000 bytes; `0x694810 test byte [0xc9e35c],1` gates the object. Its name is
read with the size dword of the header just read, into a 256-byte local; the placement is 0x40
bytes. `0x694893` loads `0xc9a320` into the placement's unique id and decrements the counter;
`0x6948c5` calls `0x695650` with a zero origin, and the object's node joins the list at
`0xc9e350`. `0x694904` sets `0xc9e384` to 1. The client's own bit 1 of an entry marks a tile
requested (`files#tile-load`), and its second dword holds the pending read.

## WDL

`CMap::LoadWdl` (`0x6944a0`) reads `<dir>\<name>.wdl` when it exists: a header, the version, a
header and the 64×64 table of file offsets into `0xc9a350`. For each nonzero offset it seeks
there, skips a header, makes a `CMapAreaLow` kept in `0xc92078`, and reads 545 signed 16-bit
heights, stored as floats at +0x3c: 17×17 then 16×16, as the low-detail mesh reads them
(`lod`). The tile's box runs from (X − 533.33331, Y − 533.33331, the lowest height) to (X, Y,
the highest), with X = 17066.666 − 16·row·33.333332 and Y = 17066.666 − 16·column·33.333332, each
one rounding; the centre is half the sum and the radius the distance to the box's top corner.

Addresses: `0x6944a0`, `0xc9a350`, `0xc92078`, `0x80654c`, `0x8105b0`, `0x694602`, `0x694648`, `0x694660`, `0x694699`, `0x694710`
Confidence: verified
Proof: one
Test: `terrain::wdl_tile_geometry_difftest`

A failed open returns. `0x694602 movsx` and `fild` convert each height; one `fcom` keeps the
lowest in a local seeded with 3.4028235e38, and the x87 stack keeps the highest from
−3.4028235e38. X is `fild` 16·row, `fmul` 33.333332, `fchs`, `fadd` 17066.666 at 53 bits, stored
at +0x0c, +0x28 and as the box's x; Y (`0x694660`) multiplies by −33.333332 (`0x8105b0`) and adds.
`0x694699` subtracts 533.33331 (`0x80654c`), storing the minimum and keeping the unrounded value
for the centre sum; each centre coordinate is the sum times 0.5 stored once. The radius is one
`fsqrt` (`0x694710`) of dz² + dy² + dx² at 53 bits, stored at +0x24. The heights are not scaled.

`terrain::wdl_tile_geometry_difftest` runs a copy of `0x694648` to `0x69471b`.

## Tile load

`0x6c2720(column, row)` starts a tile: it marks the WDT entry with bit 1, makes the area, links
it into the map's area list, and stores column +0x88, row +0x8c, the index row·64 + column +0x98,
16·column +0x90 and 16·row +0x94, and the tile origin X = 17066.666 − 16·row·33.333332 at +0x7c
and Y at +0x80, each one rounding. The file is `<dir>\<name>_<column>_<row>.adt`.

Addresses: `0x6c2720`, `0x6a05d0`, `0x6c1e70`, `0x6c274d`, `0x6c2760`, `0x6c2808`, `0x6c284a`
Confidence: verified
Proof: one
Test: `terrain::adt_area_origin_difftest`

The format `%s\%s_%d_%d.adt` takes the directory, the name, ecx and edx in that order
(`0x6c274d`). `0x6c2760 or edx,2` marks entry `[0xc89f50 + 8·(row·64 + column)]`. `0x6c2808`
multiplies 16·column by −33.333332 and adds 17066.666, then the same for 16·row; the two `fstp`
from `0x6c284a` store the row's value at +0x7c and the column's at +0x80. The word +0xc and +0x9c
are zeroed.

## Tile file read

`0x6c1e70(path)` opens the ADT, takes its size into +0x67c and a buffer of that size into
+0x678, and reads the whole file. During a map load it reads and parses at once under the
critical section `0xce8270`; otherwise it queues an asynchronous read of the whole file whose
completion, `0x6c21f0`, parses it, releases the request and closes the file. The pending request
is kept in the WDT entry's second dword.

Addresses: `0x6c1e70`, `0x6c21f0`, `0x6c2010`, `0x694180`, `0xce8270`, `0x6c1ea7`, `0x6c1f33`, `0x6c1f60`
Confidence: verified
Proof: one
Test: none

The file handle is +0xe0, opened by `0x694130`, which tries ten times and ends the program
through `CMap::SafeOpen() failed %s`; `0x694180` tries ten reads and returns 0 after the tenth
failure. `0x6c1ea7` tests `0x86a2d0`. The request (+0xe4) holds the file, the buffer, the size,
the area, the callback `0x6c1f33` stores, and the critical section; `0x6c1f60` stores it at
`0xc89f54 + 8·index`, which the parse zeroes.

## Tile file header

`0x6c2010` reads the ADT from the buffer. The header's data starts after the version record, at
buffer + 16 + the version's size, and holds offsets from that start, each to a record whose data
is 8 bytes on: +0x04 the chunk table, +0x08 the texture names, +0x0c and +0x10 the model names
and their offsets, +0x14 and +0x18 the object names and offsets, +0x1c the doodad placements and
+0x20 the object placements. It counts the placements as size/36 and size/64, files the area in
the global table `0xc96318`, and clears the pending read.

| header | data | area |
|---|---|---|
| +0x04 | chunk table (MCIN) | +0x680 |
| +0x08 | texture names (MTEX) | through `0x6c20e0` |
| +0x0c | model names (MMDX) | +0x698 |
| +0x10 | model name offsets (MMID) | +0x6a0 |
| +0x14 | object names (MWMO) | +0x69c |
| +0x18 | object name offsets (MWID) | +0x6a4 |
| +0x1c | doodad placements (MDDF) | +0x688, count +0x690 |
| +0x20 | object placements (MODF) | +0x68c, count +0x694 |

Addresses: `0x6c2010`, `0xc96318`, `0x6c2021`, `0x6c2097`, `0x6c20b6`, `0x6c20bf`
Confidence: verified
Proof: one
Test: none

`0x6c2021 lea esi,[ecx+eax+0x10]` with ecx the dword at buffer + 4. Each table entry is `lea
edx,[ofs+esi+8]`. The doodad count is the record's size times 0x38e38e39, high dword shifted
right 3 (`0x6c2097`), which is size/36; the object count is size >> 6 (`0x6c20b6`). The texture
walk gets the MTEX data and its size, the dword before it. A chunk table entry is 16 bytes, its
first dword the MCNK record's file offset (`chunk`).

## Texture names

`0x6c20e0(names, size)` makes one texture per NUL-terminated name in the MTEX data, storing the
handles from area +0xec with the count at +0xe8. Under the terrain shader flag `0xc9a31c` the
name has `_s` put before its last `.`, so `x.blp` loads as `x_s.blp`.

Addresses: `0x6c20e0`, `0x6c4c20`, `0x64a450`, `0x86c35c`, `0x6c2104`, `0x6c2143`, `0x6c21b7`
Confidence: verified
Proof: one
Test: none

`0x6c2104` tests `0xc9a31c`. The name is copied to a local; `0x64a450` with dl = 0x2e returns the
last `.`, and `0x6c2143` copies `_s` (`0x86c35c`) over it; two `repnz scas` and `rep movs` then
append the original name from its last `.` on, NUL included. `0x6c4c20` makes the texture; `0x6c21b7` also counts area +0x9c. The walk advances past each NUL.

## Doodad placements

`0x694bc0(name, entry, origin)` makes the `CMapDoodadDef` for a 36-byte MDDF entry, or returns the
existing one with the same unique id (+0x04). The position is (o.x − f+0x10, o.y − f+0x08,
o.z + f+0x0c), o being (17066.666, 17066.666, 0); the scale is the word +0x20 / 1024. The matrix
at +0xcc is the identity translated to the position, rotated about z by f+0x18 degrees + π,
about y by f+0x14 degrees and about x by f+0x1c degrees, then scaled.

Addresses: `0x694bc0`, `0x6a07f0`, `0x695100`, `0x7bdc40`, `0x7bdd60`, `0x7bdd00`, `0x7ffaac`, `0x81052c`, `0x8090d8`, `0x694c08`, `0x694c97`, `0x694cda`, `0x694d4c`
Confidence: verified
Proof: one
Test: `terrain::mddf_create_xform_difftest`

The lookup hashes the unique id with mask `0xca7de4` into `0xca7ddc` and returns a def whose +0xb4
matches and whose +0xc8 is 0 (`0x694c08`). A new def stores the negated floats as floats and then
each sum (`0x694c97`); the scale is `fild` of the word times 1/1024 (`0x8090d8`). Each angle is
the float times π/180 as a float (0x3c8efa35, `0x7ffaac`), the z angle plus π (`0x81052c`)
before its store. `TranslateMatrix` and three `RotateMatrix` calls with the axes (0, 0, 1),
(0, 1, 0) and (1, 0, 0) follow, then `0x7bdd00`, which multiplies the nine entries of the
upper-left 3×3 by the scale, each product stored, and leaves the translation. The position is copied to +0x44,
+0x50 and +0x5c; `0x695100` creates the model.

Of `0x694bc0`, `mddf_create_xform_difftest` runs `0x694c8b`–`0x694d8c`.

## Object placements

`0x695650(name, entry, origin)` makes the `CMapObjDef` for a 64-byte MODF entry, or returns the
existing one with the unique id +0x04. It places it as a doodad without scale, keeps the rigid inverse
matrix at +0xd4, and takes its box from the entry's extents: min (o.x − f+0x34, o.y − f+0x2c,
o.z + f+0x24) and max (o.x − f+0x28, o.y − f+0x20, o.z + f+0x30), the negation swapping the
x and y bounds. The centre is (min + max)·0.5 and the radius |max − centre|. It keeps the name
index +0x00, the doodad set word +0x3a and the name set word +0x3c.

Addresses: `0x695650`, `0x6a0db0`, `0x7bd820`, `0x7bd700`, `0x69569b`, `0x695754`, `0x695831`, `0x69585a`, `0x695923`
Confidence: verified
Proof: one
Test: `terrain::placement_setup_difftest`

The hash is `0xca7db4` with mask `0xca7dbc`, matching +0x7c (`0x69569b`). The matrix at +0x94 is
built by the same calls as a doodad's, and `0x695831` inverts it into a 64-byte local copied to
+0xd4: `0x7bd820` with scale 1.0 takes its rigid branch `0x7bd700`, Rᵀ with −t·Rᵀ
(`collision/contact#facet-gather`). From `0x69585a` each box bound is a stored negation plus the origin, stored. The centre
sums are 53-bit, times 0.5, stored; the radius is `fsqrt` of dz² + dy² + dx² (`0x695923`),
stored at +0x68. +0x114 holds the name index, +0x124 and +0x128 the two words; the map object models finds or
loads by name is kept at +0x118. +0x90 takes the byte `0xc89f04`.

`terrain::placement_setup_difftest` runs `0x6956e5` to `0x69578b` and `0x695854` to `0x695931`.
