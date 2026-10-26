# font: the FreeType calls

The client links FreeType 2: its modules carry version 0x10000 and require 0x20000. It is built
with `__fastcall` as the default convention, so every FreeType function takes its first two
arguments in `ecx` and `edx` and the rest on the stack. The face file (`overview`, Object files)
opens each font face from memory, selects its Unicode charmap and keeps it in a table by path; the
glyph file (`gx/glyphs`) loads and renders each glyph; the font class reads the kerning. The
default module table `0x81e068` adds, in order, the auto-hinter, CFF, CID, PSAux, PSNames,
raster1, SFNT, smooth, TrueType, Type 1 and Windows FNT. No FreeType function passes a source
path, so their rows stay `unassigned`.

Roots: none, the rows of these files are unassigned

## Opening a face

`0x5d0060(path, library)` returns a handle to the face of a font path, or 0. A null library or a
null or empty path returns 0. A path already open, matched by its `SStrHashHT` hash and then
`SStrCmpI` without a length limit, returns its handle with a reference added. Otherwise the whole
file is read with `SFile::Load` flags 3, a face is made from that memory with face index 0, and
the `unic` charmap is selected; a failure unloads the file and returns 0, and a face made before a
failed selection is not released. A new record keeps the file, the face and the path in the table
`0xc4ba7c`, and its handle is returned with a reference added.

Addresses: `0x5d0060`, `0x5d008c`, `0x5d00c9`, `0x5d00d8`, `0x5d00fd`, `0x5d0121`, `0x5d0145`, `0x5d015d`, `0x5d0169`, `0x5d0179`, `0x5d01a9`, `0x5d0241`, `0x5d0256`, `0x5d0271`, `0x648620`, `0x7cdde0`, `0xc4ba7c`
Confidence: verified
Proof: one
Test: none

`0x5d0075` to `0x5d0086` test `edx`, `ecx` and the first byte. With the table's mask `+0x24` not
−1 (`0x5d008c`), the path's bucket is walked: `0x5d00c9` compares the record's hash `+0x8`,
`0x5d00d8` its path `+0x1c` with the limit 0x7fffffff, and a hit returns `AddHandleRef` of `+0x28`
(`0x5d00fd`). `0x5d0121` calls `SFile::Load(0, path, &data, &size, 0, 3, 0)`. `0x7cdde0` is
`FT_New_Memory_Face(library, data, size, 0, &face)`, which calls `FT_Open_Face` with memory
arguments (`0x7cde09`, flags 1); `0x5d0145` passes the library in `ecx` and the data in `edx`.
`0x5d015d` passes 0x756e6963, `unic`, to `0x7ce8d0`, and `0x5d0169` sends any nonzero result to
the exit, which unloads the file (`0x5d0271`) and does nothing with the face. The record is 0x2c
bytes tagged `HFACE` (`0x5d0179`): the data at `+0x20` and the face at `+0x24` (`0x5d01a9`), the
hash at `+0x8`, the `SStrDupA` copy at `+0x1c` (`0x5d0241`) and the handle from `CreateHandle` at
`+0x28` (`0x5d0256`).

## Charmap selection

A face's charmaps are its cmap encoding records, one each, in file order. Each is tagged by the
first row of the table `0x81eb70` that names its platform and encoding:

| platform | encoding | tag |
|---|---|---|
| 2 | any | `unic` |
| 0 | any | `unic` |
| 1 | 0 | `armn` |
| 3 | 0, 1, 2, 3, 4, 5, 6 | `symb`, `unic`, `sjis`, `gb  `, `big5`, `wans`, `joha` |

Any other pair gets 0. The face's charmap starts as its first `unic` record. `FT_Select_Charmap`
sets the first charmap with the tag asked for and returns 0, or 6 when none has it, 0x23 for a
null face and 0x26 for a face without charmaps. A font whose only Unicode cmap is platform 3
encoding 10 fails to open.

Addresses: `0x7ce8d0`, `0x7d8e00`, `0x81eb70`, `0x7d7560`, `0x7d7651`, `0x7d767e`, `0x7d8920`, `0x7d8b66`, `0x7d8b78`, `0x7d8b87`
Confidence: verified
Proof: one
Test: `font::encoding_tag_difftest`

`0x7d8e00(platform, encoding)` walks the 12-byte rows from `0x81eb70` to 0x81ebe8; a row matches
when its platform equals `ecx` and its encoding equals `edx` or is −1. The cmap loader `0x7d7560`
stores the record count at `+0x1f0` (`0x7d7651`) and reads every record, 0x38 bytes each, in order
(`0x7d767e`: platform `+0xc`, encoding `+0xe`, offset `+0x18`); an error in any record fails the
whole load. `0x7d8920` makes charmap n from record n (`0x7d8b66` to `0x7d8ba6`), tags it with
`0x7d8e00` (`0x7d8b78`) and keeps the first `unic` one as the face's charmap `+0x5c` (`0x7d8b87`).
`0x7ce8d0` scans the `+0x24` pointers at `+0x28` for the tag at `+0x4` and stores the match at
`+0x5c`.

## Character index

`FT_Get_Char_Index` (`0x7ce960`) returns 0 for a null face or a face without a charmap, and
otherwise the glyph the face's driver gives for the code. The TrueType driver's lookup loads the
charmap's subtable on first use, returns 0 when that fails or when the subtable's format has no
lookup, and otherwise jumps to the format's lookup, which this entry does not read.

Addresses: `0x7ce960`, `0x7ce977`, `0x7d3c10`, `0x7d3c2c`, `0x7d3c44`
Confidence: verified
Proof: one
Test: none

`0x7ce967` loads the charmap `+0x5c` and `0x7ce96e` the driver `+0x60`; `0x7ce977` jumps through
slot `+0x54` of the driver's class `+0x14`, with the charmap in `ecx` and the code in `edx`. The
TrueType class `0x81e920` holds `0x7d3c10` there. It calls the SFNT charmap loader through
`[face+0x210]+0x70` when the charmap's loaded byte `+0x16` is 0 (`0x7d3c2c`) and sets the byte on
success; it then jumps to `+0x34` of the charmap (`0x7d3c44`), or returns 0 when that is null.

## Loading a glyph

`FT_Load_Glyph(face, glyph, flags)` (`0x7cdb40`) returns 0x23 without a face, a size or a glyph
slot, and 6 for an index at or past the glyph count. `NO_RECURSE` (0x400) clears `RENDER` (4) and
sets `NO_SCALE`, `NO_HINTING` and `IGNORE_TRANSFORM` (0x803). Without `NO_SCALE` or `NO_HINTING` a
scalable driver with no hinter of its own loads through the auto-hinter; otherwise the driver
loads. The advance is the horizontal one unless `VERTICAL_LAYOUT` (0x10); the linear advances are
scaled by the ppem over the units per em unless `LINEAR_DESIGN` (0x2000); the face's transform
applies unless `IGNORE_TRANSFORM`; and `RENDER` renders an outline in mode 1 under `MONOCHROME`
(0x1000), else 0. The TrueType loader marks the outline high precision (0x100) when the size's y
ppem is under 24.

Addresses: `0x7cdb40`, `0x7cdb6b`, `0x7cdb88`, `0x7cdba6`, `0x7cdbaf`, `0x7cdbb9`, `0x7cdbe5`, `0x7cdbf8`, `0x7cdc11`, `0x7cdc4f`, `0x7cdcaa`, `0x7cdcb9`, `0x7d3ba0`, `0x7d3c80`, `0x7d3e04`, `0x7d3e0b`, `0x81e920`, `0x7d3310`, `0x7d34b4`
Confidence: verified
Proof: one
Test: none

`0x7cdb6b` compares the index with the face's `+0x10`; `0x7cdb88` handles 0x400. The auto-hinter
path (`0x7cdba6` to `0x7cdbc3`) needs the library's hinter, flags without 3, and the driver flags
0x100 without 0x200, and with 0x400 also the load flag 0x20; the TrueType class `0x81e920` has the
flags 0x101, no hinter of its own. Else `0x7cdbe5` calls the class's `+0x50`, which for TrueType
is `0x7d3ba0`, calling `TT_Load_Glyph` `0x7d3c80`. `0x7cdbf8` takes the advance, `0x7cdc11` tests
0x2000, `0x7cdc4f` 0x800, `0x7cdcaa` 4, and `0x7cdcb9` calls `0x7cec20` with `(flags >> 12) & 1`;
a bitmap or composite slot is not rendered. `0x7d3e04` compares the size's `+0xe` with 24 and
`0x7d3e0b` stores 0x100 into the outline flags `+0x7c` of the slot. `0x7d3310` is the Type 1
driver's loader, the slot `+0x50` of its class `0x81e8b8`; its own test of the same size is at
`0x7d34b4`.

## Composite glyphs

A TrueType composite loads each component with the scale applied, in order, and then moves it.
A component with a scale, an x and y scale or a 2×2 matrix transforms its new points by
`x' = x·m0 + y·m1`, `y' = x·m2 + y·m3`, where `m0` to `m3` are the matrix's values in file order
shifted left by 2, from 2.14 to 16.16. With `ARGS_ARE_XY_VALUES` (2) the offset is the two
arguments, each scaled by its axis's scale unless `NO_SCALE` and then, under `ROUND_XY_TO_GRID`
(4), rounded to `(v + 32) & −64`; hinting does not matter. Without it the offset is point `k` of
the composite minus point `l` of the component. `USE_MY_METRICS` (0x200) keeps the component's
phantom points.

Addresses: `0x7d3ed0`, `0x7d4078`, `0x7d411b`, `0x7d4138`, `0x7d415b`, `0x7d4195`, `0x7d41c1`, `0x7d41d7`, `0x7d41f0`, `0x7d4202`, `0x7d4208`, `0x7d4213`, `0x7d4229`, `0x7d4246`, `0x7d4260`, `0x7d4d90`, `0x7d4e66`, `0x7d0340`, `0x7d4310`
Confidence: verified
Proof: one
Test: `font::vector_transform_difftest`

`0x7d4d90` reads each component's flags and index, two words or two signed bytes, and the matrix
into `+0x10` to `+0x1c` (`0x7d4e66`: the first, second, third and fourth values, `shl 2`), with
1.0 and 0 by default. `0x7d4078` reads the components; `0x7d411b` loads one recursively;
`0x7d4138` restores the phantom points unless 0x200; `0x7d415b` tests 0xc8 and `0x7d4195`
transforms each new point with `0x7d0340`. `0x7d41c1` tests 2: the point pair is checked
(`0x7d41d7`, error 0x15) and subtracted (`0x7d41f0`), the point `l` read at index start + base +
`l`. Else `0x7d4202` takes the arguments, `0x7d4208` tests `NO_SCALE`, `0x7d4213` scales and
`0x7d4229` rounds. `0x7d4246` adds the offset to the new points and `0x7d4260` copies them to the
original zone. The point `l` counts the composite's start twice, so in a composite nested inside
another the client reads a point past the one the font names.

## Fixed-point multiply

`FT_MulFix(a, b)` returns `a` when `a` is 0 or `b` is 0x10000. Otherwise it multiplies the
magnitudes and gives the result the sign of `a ^ b`. With `|a|` at most 0x800 and `|b|` at most
0x100000 the product rounds, `(|a|·|b| + 0x8000) >> 16`. Above either bound it truncates:
`((|b| & 0xffff)·(|a| & 0xffff) >> 16) + (|b| >> 16)·(|a| & 0xffff) + (|a| >> 16)·|b|`, in
32 bits. `FT_Vector_Transform` applies a 16.16 matrix with it.

Addresses: `0x7cd0e0`, `0x7cd0e4`, `0x7cd0f7`, `0x7cd0ff`, `0x7cd106`, `0x7cd10e`, `0x7cd11d`, `0x7cd140`, `0x7cd149`, `0x7d0340`
Confidence: verified
Proof: one
Test: `font::mul_fix_difftest`, `font::vector_transform_difftest`

`0x7cd0e4` and `0x7cd0e6` return `a`; `0x7cd0f7` keeps `a ^ b` for the sign; `0x7cd0ff` and
`0x7cd106` are unsigned `ja` against 0x800 and 0x100000. `0x7cd10e` is `imul; add 0x8000; shr 16`.
The other path (`0x7cd11d` to `0x7cd146`) has no rounding term: `0x7cd140` shifts the low product
right by 16 and adds the two cross products. `0x7cd149` negates for a negative sign. `0x7d0340`
computes `y'` first, `MulFix(x, m+8) + MulFix(y, m+0xc)`, then `x'`, `MulFix(y, m+4) +
MulFix(x, m)`, and does nothing for a null vector or matrix. The tests run both functions whole.

## Kerning

The TrueType kerning is the first `kern` subtable whose coverage is exactly 0x0001: format 0,
horizontal. `FT_Get_Kerning(face, left, right, mode, out)` sets the vector to 0, asks the driver,
and for mode 2 returns the value in font units. Any other mode scales x by the size's x scale and
y by its y scale; mode 1 stops there, and every other mode rounds each to `(v + 32) & −64`. It
returns 0x23 for a null face and 6 for a null output.

Addresses: `0x7ce830`, `0x7ce860`, `0x7ce876`, `0x7ce87e`, `0x7ce88c`, `0x7ce8a4`, `0x7ce8af`, `0x7d7d50`, `0x7d7dfb`, `0x7d7e04`
Confidence: verified
Proof: one
Test: `font::get_kerning_difftest`

`0x7ce860` zeroes the output and `0x7ce876` calls the driver class's `+0x58`, skipped when null.
`0x7ce87e` returns for mode 2; `0x7ce88c` and `0x7ce89c` scale by `+0x10` and `+0x14` of the size
`+0x58`; `0x7ce8a4` returns for mode 1 and `0x7ce8af` rounds. The loader `0x7d7d50` reads each
subtable's version, length and coverage; `0x7d7dfb` takes the first with coverage 1 and reads its
pairs, and `0x7d7e04` skips any other by its length less 6. The test stubs the driver's slot and
runs the rest whole.

## Rendering

`FT_Render_Glyph(slot, mode)` renders an outline through the current renderer, the head of the
renderer list. A renderer that refuses the mode with 0x13 hands the glyph to the next renderer of
the list, and one that renders after a refusal moves to the head and becomes current. raster1
refuses every mode but 1 and smooth refuses every mode but 0, so mode 1 makes a 1-bit bitmap
through raster1 and mode 0 an anti-aliased one through smooth, whichever was current. A slot
already a bitmap returns 0.

Addresses: `0x7cec20`, `0x7ceb70`, `0x7ceb7f`, `0x7ceb9d`, `0x7cebca`, `0x7cebd3`, `0x7cebe2`, `0x7cec0c`, `0x7cea30`, `0x7ceac0`, `0x7cfad0`, `0x7cee10`, `0x7cfa70`, `0x7ceeb0`, `0x81e068`, `0x81ecec`, `0x81e9b4`, `0x7dc5b0`, `0x7dc5dd`, `0x7d68f0`, `0x7d6914`
Confidence: verified
Proof: one
Test: none

`0x7cec20` passes the library of the face's driver. `0x7ceb7f` returns 0 for `bits`;
`0x7ceb9d` takes the list's head node `+0x50` and the current renderer `+0x58` for `outl`.
`0x7cebca` calls the renderer's `+0x44`; on 0x13 (`0x7cebd3`) `0x7cebe2` looks for the next
renderer of the format after the node with `0x7cea30`, and after a success `0x7cec0c` calls
`FT_Set_Renderer` (`0x7ceac0`), which moves the node to the head (`0x7cfad0`) and sets it current.
Adding a renderer, `0x7cee10` appends it to the list (`0x7cfa70`) and `0x7ceeb0` makes the list's
first outline renderer current; raster1 (`0x81ecec`) comes before smooth (`0x81e9b4`) in
`0x81e068`. `0x7dc5b0` returns 0x13 for a mode other than 1 when its
class is raster1's (`0x7dc5dd`); `0x7d68f0` returns 0x13 for a nonzero mode (`0x7d6914`).

## Mono scan conversion

raster1 sets its precision from the outline's high-precision flag: with it, 10 bits per pixel and
a jitter of 24 units, 24/1024 px; without it, 6 bits and a jitter of 2, 2/64 px. It sweeps the
rows with drop-out control 2 and then, unless the outline flag 0x200 is set, the columns. A row
span from `x1` to `x2` fills the pixels from `ceil(x1)` to `floor(x2)`, but only pixel `ceil(x1)`
when `x2 − x1 − 1 px` is at most the jitter; a span past either end of the row fills nothing, and
one partly outside is clamped to the row.

Addresses: `0x7daae0`, `0x7daae4`, `0x7daaf0`, `0x7dab02`, `0x7dab2c`, `0x7dab3d`, `0x7dab75`, `0x7dabe0`, `0x7dabe8`, `0x7dabfe`, `0x7dac12`, `0x7dac90`, `0x7daca5`, `0x7dacb2`, `0x7dacb9`, `0x7dacbd`, `0x7dacc1`, `0x7dacc8`, `0x7dace2`
Confidence: verified
Proof: one
Test: `font::set_high_precision_difftest`, `font::vertical_span_difftest`

`0x7daae4` passes `flags & 0x100` to `0x7dabe0` (`0x7daaf0`), which stores 10, 0x80 and 24
(`0x7dabe8`) or 6, 0x20 and 2 (`0x7dabfe`), then the precision `1 << bits`, half, mask and
`bits − 6` (`0x7dac12` on). `0x7dab02` keeps the second pass when 0x200 is clear, `0x7dab2c`
stores drop-out mode 2, `0x7dab3d` installs the row span `0x7dac90`, and `0x7dab75` runs the column
sweep. In `0x7dac90` `0x7daca5` rounds `x1` up, `0x7dacb2` to `0x7dacbd` set `e2 = e1` when
`x2 − precision − x1 <= jitter` (`jg`), and `0x7dacc1` rounds `x2` down; `0x7dacc8` and the next
test drop a span off the row and `0x7dace2` clamps. The tests run both functions whole.
