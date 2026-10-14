# font: object files and roots

The client's text engine: a font (`CGxFont`) owns a FreeType face and a cache of glyphs baked into
256 by 256 atlas pages; a string (`CGxString`) holds text, a size and a layout; a batch
(`CGxStringBatch`) groups strings of one font for drawing. The interface, the console and the
world text (`playername`) call it through the `GxuFont` functions. Topic docs: `api`, `markup`,
`metrics`, `measure`, `string` and `glyphs`.

The code lies in eight object files, from `0x5c1560` to `0x5d1100`; the table below gives each
file's code and its run of slots in the initializer table. Only three cite their source path.

Roots: `0x5cd6d0`, `0x5cd830`, `0x5cd950`, `0x5ca1d0`, `0x5ca6f0`, `0x5ccb10`, `0x5ccb40`, `0x5cd650`, `0x5cdf40`, `0x5c6fa0`, `0x5c7010`, `0x5c7080`, `0x5c7300`, `0x5c7470`, `0x5cae90`, `0x5cf310`

## Object files

The C API file (no source path) is `0x5c1560` to `0x5c3c90`. A file for the maths library follows
(`gxumath`). `GxuFontUtil.cpp` holds the kernels and the screen scaling, `GxuFontMiscClasses.cpp`
the glyph and atlas containers and the quad emission, `GxuFontString.cpp` the string. Two more
files with no source path lie between and after them: `0x5c9f30` up to `0x5ccaf0` (the font class)
and `0x5ce420` to the face file. The face and its FreeType calls  are
unassigned, and `IGxuFontGlyph.cpp` (`gx`) ends the run.

| file | code from | initializer slots |
|---|---|---|
| C API | `0x5c1560` | `0x82aa18` to `0x82aa34` |
| `GxuFontUtil.cpp` | `0x5c6650` | `0x82aa44` to `0x82aa58` |
| `GxuFontMiscClasses.cpp` | `0x5c7de0` | `0x82aa5c` to `0x82aa68` |
| font class | `0x5c9f30` | `0x82aa6c` to `0x82aa74` |
| `GxuFontString.cpp` | `0x5ccaf0` | `0x82aa78` to `0x82aa80` |
| glyph rasterizer | `0x5ce420` | `0x82aa84` to `0x82aa8c` |
| face | `0x5cfdf0` | `0x82aa90` to `0x82aa9c` |
| `IGxuFontGlyph.cpp` | `0x5d1100` | `0x82aaa0` to `0x82aaa8` |

Addresses: `0x827000`, `0x82aa18`, `0x82aa34`, `0x82aa44`, `0x82aa58`, `0x82aa5c`, `0x82aa68`, `0x82aa6c`, `0x82aa74`, `0x82aa78`, `0x82aa80`, `0x82aa84`, `0x82aa8c`, `0x82aa90`, `0x82aa9c`, `0x82aaa0`, `0x82aaa8`, `0x5c1560`, `0x5c6650`, `0x5c7de0`, `0x5c9f30`, `0x5ccaf0`, `0x5ce420`, `0x5cfdf0`, `0x5d1100`, `0x5c6660`, `0x5c7d90`, `0x5c7da0`, `0x5c7db0`, `0x5c7dc0`, `0x5c7df0`, `0x5c9550`, `0x5c9560`, `0x5c9570`, `0x5c9580`, `0x5c9f40`, `0x5cb820`, `0x5cb830`, `0x5cb840`, `0x5cb850`, `0x5ccb00`, `0x5ce170`, `0x5ce180`, `0x5ce190`, `0x5ce1a0`, `0x5ce430`, `0x5cfc80`, `0x5cfc90`, `0x5cfcd0`, `0x5cfce0`
Confidence: verified
Proof: one
Test: none

Each file's run starts with its +∞ copy, then its 2π and 1/2π thunks (for `GxuFontUtil.cpp` the
slots `0x82aa44`, `0x82aa48`, `0x82aa4c`); the slots after them (`0x82aa50` to `0x82aa58`) are
its static objects. The gxumath file keeps its own trio; the next trio `0xc2b980`, `0xc2b9ac`, `0xc2b984` to `GxuFontUtil.cpp`, and the
trios `0xc2b9c0`, `0xc2b9cc`, `0xc2b9c4`; `0xc2b9e0`, `0xc2b9e8`, `0xc2b9e4`; `0xc2b9ec`, `0xc2ba08`,
`0xc2b9f4`; `0xc2ba40`, `0xc4ba70`, `0xc4ba48`; the trio of the face file to the next
six files, and the last trio to `IGxuFontGlyph.cpp`. The three paths are at
`0x85f5fc`, `0x85f630` and `0x85f7f0` and are used at `0x5c6f48` to `0x5c6f91`, `0x5c7f3e` to
`0x5c868b` and `0x5cd6fd` to `0x5cdb10`.

## Static objects

The files build lists and tables before the first frame: the batch free list `0xc2b840`, the
font registry `0xc2b974`, the string pool `0xc2b988`, the string list `0xc2b994` and a
list of `GxuFontMiscClasses.cpp` at `0xc2b9b4`, each an empty explicit list; and four zeroed
buffers for the frame-rate bar: three sets of digit quads and a work buffer.

Addresses: `0x5c1580`, `0x5c1590`, `0x5c1670`, `0x5c1680`, `0x5c16d0`, `0x5c16e0`, `0x5c1730`, `0x5c1740`, `0x5c1790`, `0x5c17a0`, `0x5c6670`, `0x5c6680`, `0x5c6760`, `0x5c6770`, `0x5c6850`, `0x5c6860`, `0x5c7e00`, `0x5c7e10`, `0x5c1820`, `0xc2b840`, `0xc2b974`, `0xc2b988`, `0xc2b994`, `0xc2b9b4`, `0xc2a738`, `0xc2aa38`, `0xc2acb8`, `0xc2af38`, `0xc2b6c0`, `0x5c17f0`, `0x5c66b0`, `0x5c66c0`, `0x5c67a0`, `0x5c67b0`, `0x5c6890`, `0x5c68a0`, `0x5c7e40`, `0x5c7e50`
Confidence: verified
Proof: one
Test: none

A list body stores its own address in its head word and the same address with bit 0 set in its tail
word (`0x5c1590`: `mov [0xc2b844],0xc2b844`, `mov [0xc2b848],eax` with `eax = 0xc2b844 | 1`). The
buffer bodies zero 0x28 records of 16 bytes from `0xc2a738`, `0xc2aa38` and `0xc2acb8`, and 0x78 records
from `0xc2af38`. `0x5c1820` fills the three 640-byte buffers
with quads coloured red, yellow and green (`0xffff0000`, `0xffffff00`, `0xff00ff00`) and writes an index
table of 30 quads (`{4k, 4k+1, 4k+2, 4k+1, 4k+3, 4k+2}` as words from `0xc2b6c0`); the font module's
start-up reaches it by the jump at `0x5c1814`.

## Unreached functions

These have no call, jump or stored address outside unreached code.

Addresses: `0x5cd690`, `0x5ce120`, `0x5c1e80`, `0x5c1ea0`, `0x5c2700`, `0x5c2790`, `0x5c27d0`, `0x5c3490`, `0x5c34a0`, `0x5caef0`, `0x5cd2d0`, `0x5cf640`, `0x5cfc20`, `0x5cfc30`, `0x5cfc40`
Confidence: verified
Proof: one
Test: none

Census: every `e8` and `e9` whose target is the function and every little-endian occurrence of its
address in any section, repeated without sites in unreached code until nothing changes. Controls:
`0x5c1ae0` (callers in two systems) and `0x5c6fa0` (many). `0x5cd690` and `0x5ce120` follow a
`ret`.
