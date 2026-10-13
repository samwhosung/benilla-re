# terrain: chunk

The map chunk: one 16th of an ADT tile on a side, 33.333332 yards, read from its MCNK record into a
`CMapChunk`. This doc covers the record as the client reads it, the 145 heights and normals, the
texture layers with their alpha maps, the shadow map, the doodad and object references, the sound
emitters and the links to neighbouring chunks.

The code is the chunk object file, `0x6ae0f0` through `0x6b1860`, and the area's chunk creation
`0x6c2880`. A `CMapChunk` is 3,892 bytes, taken from the `WCHUNK` pool (`overview`); its vtable is
`0x810bf8`. The name comes from the chunk's vertex and index buffer pools, `CMapChunk_vtx` and
`CMapChunk_idx`. Offsets in the MCNK header are from the record's start, its 8-byte tag and size,
and each lands on a sub-record whose data begins 8 bytes on; the client never reads a tag.

Roots: `0x6b11f0`

## Chunk creation

`0x6c2880(area, column, row)` makes the chunk at (column, row) of an area: it takes a `CMapChunk`
from the pool, links it into the area's chunk list, sets its indices and loads it from the MCNK
record the area's MCIN entry row·16 + column points at. The MCIN entry's third dword is the
client's own: bit 0 set means the chunk was made before, and the load is told whether this is the
first time.

Addresses: `0x6c2880`, `0x6a06e0`, `0x6a0450`, `0x6af5f0`, `0x6c28a6`, `0x6c28bc`, `0x6c2949`, `0x6c299d`
Confidence: verified
Proof: one
Test: none

`ret 4`, the area in ecx, the column in edx. The entry is `[area+0x680] + 16·(row·16 + column)`;
`0x6c28a6 test al,1` on its dword +8 sets the flag argument to 0 when the bit is set, and
`0x6c28bc` sets it. The chunk is `0x6a06e0()`, its list node `0x6a0450(chunk)` with the area at
node +8, appended at the area's list (+0x26c, terminator +0x270). The chunk gets column +0x7c,
row +0x80, index +0xb4 (`0x6c2949`), the area's +0x90 + column at +0x8c and +0x94 + row at
+0x90, the same times 8 at +0x84 and +0x88, and the word +0xc zeroed. `0x6c299d` forms the
record `[entry] + [area+0x678]`, the entry's first dword offset into the file held in memory.

## MCNK header

`CMapChunk::Load` (`0x6af5f0`) keeps the record, resolves eight sub-record pointers, and reads the
header: flags +0x00, layer count +0x0c, doodad references +0x10, area id +0x34, object
references +0x38, the 16-byte low-resolution texture map +0x40, the 8-byte doodad map +0x50,
sound emitters +0x5c and position +0x68. In order it builds vertices, normals, liquid, sound
emitters and neighbour links; sets the chunk flag word +0xc to 0x40 when flags bit 1 is set, else
0; then reads references and layers, the shadow map when flags bit 0 is set and world flag 0x40
is on, the combined texture under either shader flag, links lights, and files the chunk in the
area's table.

| header | sub-record | chunk field |
|---|---|---|
| +0x14 | heights (MCVT) | +0xf14 |
| +0x18 | normals (MCNR) | +0xf18 |
| +0x1c | layers (MCLY) | +0xf20 |
| +0x20 | references (MCRF) | +0xf28 |
| +0x24 | alpha maps (MCAL) | +0xf24 |
| +0x2c | shadow map (MCSH) | +0xf1c |
| +0x58 | sound emitters (MCSE) | +0xf30 |
| +0x60 | liquid (MCLQ) | +0xf2c |

Addresses: `0x6af5f0`, `0x6af970`, `0x6af672`, `0x6af677`, `0x6af6ea`, `0x6af6ef`, `0x6af71f`, `0x6af74c`, `0xc7b2a4`, `0xc89f44`, `0xc9a31c`
Confidence: verified
Proof: one
Test: none

`0x6af970` stores record + 8 at +0xf08 and each `lea esi,[ofs+record+8]` in the table. Load
stores the record at +0x168, +0x34 at +0x16c, +0x68 to +0x70 at +0x6c to +0x74, the addresses of
+0x40 and +0x50 at +0xf0c and +0xf10. `0x6af672 test byte [eax],2` gates the 0x40 store
(`0x6af677`) over a zeroed word. The layer loop runs `[hdr+0xc]` times, 16 bytes a step. The
shadow needs `0x6af6ea test [eax],1` and `0x6af6ef test [0xc7b2a4],0x40`; otherwise +0x144 and
+0x148 are zeroed. `0x6af71f` reads the shader flags `0xc89f44` and `0xc9a31c`. The word +0xc
gains bit 0 before the lights and bit 7 after `0x6af74c` stores the chunk at area +0x278 +
4·index. The header fields +0x04, +0x08, +0x28 and +0x64 are never read.

## Chunk vertices

`CMapChunk::BuildVertices` (`0x6b0e50`) writes 145 vertices relative to the chunk position: nine
rows of nine outer vertices, each but the last followed by eight inner ones. With g the chunk's
global row (+0x90) and h its global column (+0x8c), and every step stored as a float,

o(n) = 17066.666 − n·33.333332, sx = −(o(g) − o(g+1))/8, sy = −(o(h) − o(h+1))/8.

Outer vertex (j, i) is (j·sx, i·sy, z); inner vertex (j, i) is (j·sx + sx/2, i·sy + sy/2, z),
with z the next MCVT float copied unchanged. The box is the vertices' minimum and maximum plus the
position; the centre is (min + max)/2 and the radius |max − centre|.

Addresses: `0x6b0e50`, `0x699250`, `0x6b11f0`, `0x741760`, `0x80fee4`, `0x7ffab4`, `0x6b0f20`, `0x6b0fb1`, `0x6b101f`, `0x6b106c`, `0x6b112d`
Confidence: verified
Proof: one
Test: `terrain::vec3_min`, `terrain::vec3_max_inplace`, `terrain::mcvt_world_verts`, `terrain::vec3_max_inplace_difftest`

Each product is `fild; fstp dword; fld; fmul [0x80fee4]` (33.333332, 0x42055555) then `fstp`, and
o adds 17066.666 (0x46855555, `0x7ffab4`) to the negation and stores. `0x6b0f20` forms o(h) −
o(h+1); the negations, the divisions by 8 and the halves by 2 are each stored. A vertex x is the
float j·sx copied (`mov`), an outer y one rounding of i·sy (`0x6b0fb1`), an inner y one rounding
of i·sy + sy/2 (`0x6b101f`), and an inner x the float j·sx + sx/2. `0x699250` keeps the
minimum, taking the vertex on a tie or a NaN; `0x6b11f0` keeps the maximum, also taking the
vertex. From `0x6b106c` the box min (+0x44) and max (+0x50) each add the position (+0x6c), the
centre +0x5c is each sum stored then halved, and the radius +0x68 is `_CIsqrt` (`0x6b112d`) of
dx² + dy² + dz², the deltas stored, the sum at 53 bits then stored; `rep movs` copies the box to
+0x94. Because sx depends on rounding at g, it is not always −4.1666665.

`vec3_max_inplace_difftest` runs `0x6b11f0` whole.

## Chunk normals

`CMapChunk::LoadNormals` (`0x6aff10`) reads 145 normals of three signed bytes each, in the
vertices' order, and stores each component as the float s·(1/127) at +0x170.

Addresses: `0x6aff10`, `0x810c08`
Confidence: verified
Proof: one
Test: `terrain::mcnr_normals_difftest`

The loop runs 0x91 times with a stride of 3 bytes: `movsx`, `fild`, `fmul [0x810c08]` and
`fstp dword` for each of the three bytes. The constant is 0x3c010204, 1/127 as a float, so each
component is one rounding of s times that float. No byte past the 435th is read.

## Texture layers

`CMapChunk::AddLayer` (`0x6aff80`) makes one `CChunkLayer` (28 bytes) per 16-byte MCLY entry, up to
four at +0x128 with the count at +0x138. A layer holds the area's texture for the entry's index
(+0x0c), the entry's flags word (+0x08) and effect id word (+0x0a), and the address of its alpha
map, the entry's offset added to the MCAL data (+0x10). On the fixed-function path, when flags
bit 8 is set, the layer gets an alpha texture of 32 texels square when `shadowLevel` is exactly 1
and 64 otherwise. The decoders choose 32×32 for any nonzero value, and the option's check accepts
any integer up to 1, so a negative `shadowLevel` gives a 64×64 texture filled with 32×32 data.

Addresses: `0x6aff80`, `0x69ff60`, `0x6aeaf0`, `0x6b1150`, `0x58ac70`, `0x688c10`, `0x6affad`, `0x6affe2`, `0x6afffb`, `0x6b001e`, `0xc63284`
Confidence: verified
Proof: one
Test: none

`ret 0xc` with the area, the entry and the MCAL base. `0x6affad` loads the area's +0xec slot for
the entry's first dword. The flags and effect id are word loads from +0x04 and +0x0c, the alpha
offset a dword from +0x08. The texture is skipped when either shader flag is set; `0x6affe2`
compares `0xc63284` with 1, and `0x6afffb test byte [esi+9],1` tests bit 8. `0x6aeaf0` pops a
texture from the chunk texture stack or creates one with fill callback `0x6b1150`, and
`0x6b001e` asks for the fill through `0x58ac70` with 0, 0 and the size twice, so the rectangle is
n×n whichever way it orders them. The layer keeps its chunk at +0x18. `0x688c10` accepts a value
whose `atoi` is at most 1 (`cmp eax,1; jle`, signed); the decoders test `0xc63284` for zero.

## Alpha maps

A layer's alpha map is 2,048 bytes of 4-bit values, low nibble first, decoded to 16-bit texels
(a << 12) | 0x0fff. When MCNK flags bit 15 is set, the map is 64×64; otherwise it holds 63 useful
values per 64-value row and the decoder copies column 62 into 63 and row 62 into 63. With
`shadowLevel` 0 the texture is 64×64; otherwise 32×32, sampling every other row and column. For a
64×64 map the 32 samples of a row or column are 0, 2, …, 60 and 63; for the other kind 0, 2, …,
62.

Addresses: `0x6b0a60`, `0x6b0ac0`, `0x6b0b30`, `0x6b0bf0`, `0x6b0c80`, `0x6b02d0`, `0x6b1150`, `0x810ba4`, `0x810bac`, `0xca8398`, `0x6b0a6b`, `0x6b0bbf`
Confidence: verified
Proof: one
Test: `terrain::mcal_decode64`, `terrain::mcal_decode32`, `terrain::mcal_decode63`, `terrain::mcal_decode32_63`

The fill callback `0x6b1150` acts only on request 1: it decodes through `0x6b02d0` into the
scratch buffer `0xca8398` and returns a pitch of twice the width. `0x6b0a60` tests the flags'
high byte (`0x6b0a6b test ch,ch; jns`) and `0xc63284` for zero. A value n is byte n >> 1 masked
by `0x810ba4` (0x0f, 0xf0) and shifted left by `0x810bac` (4, 0) in a byte, then placed in the
texel's high byte with `or ecx,0xfff`. `0x6b0bf0` decodes 63 values a row, copies the last texel,
skips one value, and after 63 rows copies the last row with `rep movs`. `0x6b0b30` takes 31 values
two apart, then the row's last, and adds an extra row before the final one (`0x6b0bbf cmp
eax,0x3c`); `0x6b0c80` steps two values and two rows.

## Shadow maps

On the fixed-function path, a chunk with a shadow map gets its own texture (+0x140), sized as an
alpha texture. The map is 512 bytes of bits, least significant first, decoded to the texels 0x0000
for a clear bit and 0xffff for a set one, 64×64 with `shadowLevel` 0 and otherwise 32×32 on the
0, 2, …, 60, 63 samples; the texture is 32 square only when the value is exactly 1. The MCNK flag bit 15 does not change the shadow walk.

Addresses: `0x6b0030`, `0x6b0051`, `0x6b0d00`, `0x6b0d30`, `0x6b0da0`, `0x6b02b0`, `0x6b1180`, `0x6aed20`, `0x810bb4`, `0x810bd4`, `0x86b590`
Confidence: verified
Proof: one
Test: `terrain::mcsh_decode`, `terrain::mcsh_decode32`

`0x6b0030` stores the map's address at +0x144, then, with neither shader flag set, creates the
texture through `0x6aed20` with fill callback `0x6b1180`. That callback runs `0x6b02b0`, which
decodes +0x144 into `0xca8398` through `0x6b0d00`; the dispatcher reads only `0xc63284`. A bit n
is byte n >> 3 masked by `0x810bb4` (1 << k) and shifted right by `0x810bd4` (k) for k = n & 7;
the texel is the word at `0x86b590` it indexes, 0x0000 then 0xffff. `0x6b0da0` walks like
`0x6b0b30`.

## Combined chunk texture

With either shader flag set, a chunk gets one texture (+0x13c) that packs its shadow and the
alpha maps of layers 1 to 3: texel = s << 12 | a1 << 8 | a2 << 4 | a3, each a 4-bit value. A
layer without an alpha map, or a missing layer, gives 0xf; the shadow is 0xf for a clear bit or
no map and 0 for a set bit. The size and the sample walks follow the alpha maps.

Addresses: `0x6b00a0`, `0x6b11b0`, `0x6b02f0`, `0x6b0350`, `0x6b03d0`, `0x6b0510`, `0x6b0770`, `0x6b08d0`, `0x810bf4`, `0x6b043e`, `0x6b049b`, `0x6b04c1`
Confidence: verified
Proof: one
Test: `terrain::alpha_texel_64`, `terrain::alpha_texel_32_edge`, `terrain::alpha_texel_dup`, `terrain::alpha_texel_32`

`0x6b02f0` gathers four alpha pointers, null unless the layer exists and has flags bit 8, and
passes them with +0x144 to `0x6b0350`, which picks a packer as `0x6b0a60` picks a decoder. In
`0x6b03d0` each alpha value is the masked, shifted nibble as in the alpha decoder (`0x6b043e`),
or 0xff without a map; the shadow is the byte `0x810bf4` indexes by the bit, 0xff then 0x00
(`0x6b049b`), or 0xff. From `0x6b04c1` the texel is ((s & 0xf0) << 4 | (a1 & 0xf0)) << 4 |
(a2 & 0xf0) | a3 >> 4. Layer 0's pointer is gathered but never read. The other three packers
use the same tables and the same mask, shift and or sequence over the walks of `0x6b0b30`,
`0x6b0bf0` and `0x6b0c80`.

## Doodad and object references

`CMapChunk::LoadReferences` (`0x6b00f0`) reads the MCRF list: first the doodad references, then
the object references, each a dword index into the area's MDDF (36-byte entries) or MODF (64-byte
entries). Each placement is made through `0x694bc0` or `0x695650` with its model name, from the
MMID and MMDX or the MWID and MWMO tables, and linked into the chunk's doodad list (+0xdc) or
object list (+0xe8). A doodad gains flag 4 at +0xc and a fade of 1.0 at +0xa4.

Addresses: `0x6b00f0`, `0x694bc0`, `0x695650`, `0x698b70`, `0x6b0125`, `0x6b021a`, `0x6b01bb`
Confidence: verified
Proof: one
Test: none

`ret 0x10` with the area, the list and the two counts. `0x6b0125` scales the index by 9 dwords
into the area's +0x688; the name is `[area+0x6a0][entry's dword 0] + [area+0x698]`.
`0x6b021a shl edx,6` indexes +0x68c, the name from +0x6a4 and +0x69c. Both get an origin of
(17066.666, 17066.666, 0) on the stack. Each node from `0x6a0450` has the chunk at +8 and is
appended after an unlink. `0x6b01bb` stores 1.0; when the doodad has a model at +0x88 that
`0x7103d0(0, 0)` accepts, `0x698b70` runs on it with the chunk.

## Sound emitters

`CMapChunk::LoadSoundEmitters` (`0x6af810`) would make one 88-byte record per 52-byte MCSE entry
and pass it to the callback `0xca82c4`, but the callback is always null in this build, so the
records are never made and MCSE is never read. The walk copies the entry's first eight dwords,
widens its words +0x20, +0x22, +0x24, +0x28, +0x2a, +0x2c, +0x2e, +0x30 and +0x32 and bytes +0x26
and +0x27 to dwords, links the record into the chunk's list at +0x10c, and calls the callback
with the record's +4.

Addresses: `0x6af810`, `0x6a1000`, `0xca82c4`, `0x6af816`, `0x6af93b`, `0x6ae4f0`, `0x6ae508`, `0x6723c0`
Confidence: verified
Proof: one
Test: none

`0x6af816` loads the callback and skips the walk when it is 0. A scan of the image for the bytes
of `0xca82c4` finds four, all operands: the load, the call `0x6af93b`, the store of 0 at
`0x6ae508` in the map's start-up `0x6ae4f0`, and the store of ecx at `0x6723c0`, a setter that
nothing calls or stores (its address appears nowhere in the image). The same scan finds the
known store at `0x6ae508`, the control. The record's meaning is inferred from the pool's name,
`WSOUNDEMITTER`.

## Liquid blocks

`CMapChunk::LoadLiquids` (`0x6af760`) reads one 804-byte MCLQ block for each of MCNK flags bits 2
to 5 that is set, in bit order, into liquid slot 0 to 3 (+0x118): a `CChunkLiquid` keeps the
block's two floats at +0x04 and +0x08, and the addresses of its 81 eight-byte vertices (+0x0c),
64 tile bytes (+0x10) and 80-byte trailer (+0x18), with the dword before the trailer at +0x14.
A clear bit frees its slot. `liquid` builds the surface.

Addresses: `0x6af760`, `0x6a0ee0`, `0x6a0f90`, `0x68d540`, `0x6af7a3`, `0x6af7cb`
Confidence: verified
Proof: one
Test: none

The bit starts at 4 and doubles four times. A missing slot is made by `0x6a0ee0` and gets the
chunk at +0x1c. From `0x6af7a3` the cursor advances 8, 0x288, 0x40, 4 and 0x50 bytes, 0x324 in
all; `0x68d540` runs at `0x6af7ce`. A clear bit with a slot calls `0x6a0f90` and stores 0.

## Neighbour links

`CMapChunk::LinkNeighbours` (`0x6c29c0`) links the chunk to its four neighbours and they to it:
+0xc4 the chunk a row before, +0xc8 a column after, +0xcc a row after and +0xd0 a column before.
At an area's edge the neighbour comes from the adjacent area in the global 64×64 area table,
when that area is loaded; at the map's edge there is none.

Addresses: `0x6c29c0`, `0xc96318`, `0x6c2a3b`, `0x6c2a7f`, `0x6c2abf`, `0x6c2b00`
Confidence: verified
Proof: one
Test: none

The area is the owner at +8 of the chunk's node +0x20, t its row (+0x8c) times 64 plus its column
(+0x88). Inside the area the neighbours are entries index − 16, + 1, + 16 and − 1 of +0x278. At
row 0 `0x6c2a3b` reads area t − 64 (`0xc96218`) for its entry 240 + column; at column 15
`0x6c2a7f` reads t + 1 for index − 15; at row 15 `0x6c2abf` reads t + 64 for the column; at
column 0 `0x6c2b00` reads t − 1 for index + 15. Each found neighbour stores the chunk in its
opposite slot.
