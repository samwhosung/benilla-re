# console: hardware

The hardware step of `ConsoleDeviceInitialize`: the table of known video hardware, the match of the running machine against it, the question asked when the hardware changed, and the quality values a matched row gives. `display` holds the order of the start-up and the settings the step overwrites. The code is a file of its own that follows the command registry and has no path string; its rows stay unassigned.

Roots: none, the rows of this file are unassigned

## Hardware table

`0x641260` loads the table on its first call and is called each start-up. The file is `DBFilesClient\VideoHardware.dbc` (22 fields, `0x58` bytes a row; `tables/dbc_schemas.tsv`). The loader checks the `WDBC` magic, the field count and the row size, reads the rows and the string block, and builds an index by row id. A file that will not open, or whose field count or row size differs, is reported through the fatal-error function `0x64cf80`; a bad magic or an empty table leaves the table empty, and a failed string read is reported through `0x64d0e0`.
The store is five words at `0xc518c8`: the rows, their count, the id index, the largest id and the loaded flag.

Addresses: `0x641260`, `0x689a70`, `0x869fb8`, `0x6477a0`, `0x648460`, `0x648730`, `0x64cf80`, `0x64d0e0`, `0x6412bd`, `0x6412e7`, `0x641312`, `0x641347`, `0x641388`, `0x6413c9`, `0x64140a`, `0x641445`, `0x641490`, `0x6414a0`, `0xc518c8`, `0x82e4c8`, `0x82e490`, `0x82e460`, `0x82e440`, `0x865724`
Confidence: verified
Proof: one
Test: none

`0x641269` tests the loaded flag. `0x689a70` returns the file name (`0x869fb8`), `0x6477a0` opens it and `0x64cf80` reports `Unable to open %s` (`0x82e4c8`). `0x6412bd` compares the first word with `0x43424457`, `0x6412e7` the row count with 0, `0x641312` the field count with 0x16 (message `0x82e490`) and `0x641347` the row size with `0x58` (message `0x82e460`). `0x641388` allocates the rows and strings under the path `0x865724`, `0x6413c9` copies each row and `0x64140a` reads the strings (message `0x82e440`).
`0x641445` allocates the id index, `0x641490` fills it and `0x6414a0` sets the loaded flag.

## Detection record

The record at `0xc4e6bc` holds what the step learns: the vendor and device words at `+0` and `+2` (both `0xffff` when unknown), the two driver version words at `+4` and `+8`, the processor class at `+0x18`, the matched row id at `+0x1c`, the sound index at `+0x20`, the memory index at `+0x24`, the matched row at `+0x28` and the bias row at `+0x2c`. The processor class is 1 when the processor frequency exceeds 1500 MHz and 0 otherwise; the sound and memory indices are always 0.
The step logs `DetectHardware():` and the four indices.

Addresses: `0xc4e6bc`, `0x6414ac`, `0x6414b7`, `0x6414ba`, `0x6414c0`, `0x6414cd`, `0x6414d6`, `0x6414e8`, `0x6414eb`, `0x6414ee`, `0x64152f`, `0x64154e`, `0x641558`, `0x641562`, `0x42c060`, `0x589990`, `0x58baa0`, `0x80e2e0`, `0x8655d8`, `0xc518d0`, `0xc518d4`
Confidence: verified
Proof: one
Test: none

`0x6414ac` reads the processor frequency (`0x42c060`), `0x6414ba` scales it to MHz and `0x6414c0` compares it with 1500.0 (`0x80e2e0`); `0x6414cd` stores 1 and `0x6414d6` stores 0. `0x6414e8` and `0x6414eb` clear the memory index and the row id. `0x6414ee` asks the adapter for its ids (`0x589990` reaches `0x58baa0`, which logs `CGxDevice::DeviceAdapterID()`) and the match runs only when it succeeds. The bias row is the entry of the table at `0x8655d8` for the processor class (`0x64152f`), 24 bytes each.
`0x64154e` stores the row found through the index (`0xc518d0`, bounded by the largest id at `0xc518d4`), or 0. `0x641558` runs the change check and `0x641562` onward write the log.

## Matching a row

`0x641610` scans the rows from index 1 for the first one whose second field equals the vendor word and whose third field equals the device word, and stores its first field, the row id, as the matched id. Row 0 is never examined. When the first match leaves the id at 0, the vendor word becomes `0xffff` and the adapter class (`gx/device#adapter-class`: 2, 1 or 0) replaces the device word, and the scan runs again; a failed class query skips it. A machine with no matching row so matches only a row whose vendor field is `0xffff` and whose device field is its class.

Addresses: `0x641610`, `0x641619`, `0x641627`, `0x64163e`, `0x641641`, `0x64164a`, `0x64165b`, `0x64165d`, `0x641503`, `0x641505`, `0x64150a`, `0x641515`, `0x5899b0`, `0x58bc90`, `0x64154e`, `0x63a915`
Confidence: verified
Proof: one
Test: none

`0x641619` starts the index at 1 and `0x641627` points at the second row. `0x64163e` and `0x641641` compare the vendor word with the row's `+4`; `0x64164a` compares the device word with `+8`. The first hit stores the id of `+0` at `0x64165d`. `0x641503` begins the second pass: `0x641505` stores `0xffff`, `0x64150a` asks `0x5899b0`, a jump to `0x58bc90`, for the class, and `0x641515` repeats the match. A machine that matches in neither pass keeps the id 0, and the row pointer stored at `0x64154e` is the id map entry of id 0, which is 0 unless a row has that id; the registration of `gxMultisample` then reads `+0x54` through it (`0x63a915`) with no test.

## Hardware change check

`0x641670` compares the detection record with the values the last run saved in the registry key `World of Warcraft\Client` as `HWCpuIdx`, `HWMemIdx`, `HWVideoID` and `HWSoundIdx`; a value that cannot be read counts as equal. A difference in the processor class, the memory index or the sound index asks the question `Hardware changed.  Reload default settings?`, and so does a different row id unless the saved id is 1, `0xa8`, `0xa9` or `0xaa`.
The question is a yes-no box. Yes sets the byte the caller gave, which makes the quality step run; the four current values are always saved.

Addresses: `0x641670`, `0x64168a`, `0x641692`, `0x6416b3`, `0x6416d4`, `0x6416f5`, `0x641704`, `0x64172a`, `0x641750`, `0x641756`, `0x641779`, `0x641797`, `0x6417a3`, `0x6417b1`, `0x6417c4`, `0x64c650`, `0x64c9a0`, `0x435c30`, `0x435f00`, `0x8656e4`, `0x80e2c4`, `0x82e42c`, `0x865784`
Confidence: verified
Proof: one
Test: none

The four value names are the pointers at `0x8656e4`, read at `0x641692`, `0x6416b3`, `0x6416d4` and `0x6416f5` through `0x64c650` under the key `0x80e2c4`. `0x641704` to `0x64172a` build the saved-id test and `0x64172a` to `0x641750` the comparison. `0x641756` and `0x641779` choose the title and text: `0x82e42c` and `0x865784` unless the string table holds localized ones. `0x641797` asks `0x435c30` for the foreground window and `0x6417a3` shows the box with the yes-no style (`0x435f00` returns 0 for yes and 1 for no) and `0x6417b1` stores 1 for yes.
`0x6417c4` onward write the four values with `0x64c9a0`.

## Quality block

`0x641820` fills a block of `0x41` bytes at `0xc4e678` from the matched row and the bias row. Each value reads a column of the row; the columns with a table add twice the column value to a bias word of the bias row and index the table. The bias words are 0 for a slow processor and 1 for a fast one (the two rows at `0x8655d8`).

| block | row column | table and index | setting |
|---|---|---|---|
| `+0x00` | `+0x0c` | `0x8656c4` at 2 · column + bias 0 | `farclip` |
| `+0x04` | `+0x10` | `0x865610` at the column | `lodDist` |
| `+0x08` | `+0x14` | the column | `shadowLevel` |
| `+0x0c` | `+0x18` | `0x865620` at the column | `frillDensity` |
| `+0x10` | `+0x1c` | the column | `fullAlpha` |
| `+0x14` | `+0x20` | `0x865630` at 2 · column + bias 1 | `doodadAnim` |
| `+0x15` | `+0x24` | nonzero | `trilinear` |
| `+0x18` | `+0x28` | the column | `MaxLights` |

Addresses: `0x641820`, `0xc4e678`, `0x8655d8`, `0x8656c4`, `0x865610`, `0x865620`, `0x865630`, `0x641830`, `0x641846`, `0x641849`, `0x64184f`, `0x64185f`, `0x641868`, `0x641877`, `0x64188c`, `0x641898`
Confidence: verified
Proof: one
Test: none

`0x641820` takes the block in `ecx` and the detection record in `edx`. The farclip table `0x8656c4` holds 200, 250, 300, 350, 350, 400, 450, 500; the lodDist table `0x865610` 80, 80, 100, 100; the frill table `0x865620` 8, 12, 16, 24; the doodad table `0x865630` the bytes 0, 0, 0, 1. `0x641830` indexes the farclip table, `0x641846` stores lodDist, `0x641849` and `0x64184f` copy shadowLevel, `0x64185f` indexes frill, `0x641868` copies fullAlpha,
`0x641877` indexes the doodad bytes, `0x64188c` tests the trilinear column and `0x641898` copies MaxLights.

## Quality block, rest

The rest of the block, with the same indexing. The pixel shader column feeds two settings, and the smallest cull column indexes two tables.

| block | row column | table and index | setting |
|---|---|---|---|
| `+0x1c` | `+0x2c` | nonzero | `pixelShaders` and `specular` |
| `+0x20` | `+0x30` | `0x865634` at 2 · column + bias 2 | `waterLOD` |
| `+0x24` | `+0x34` | `0x865644` at 2 · column + bias 3 | `particleDensity` |
| `+0x28` | `+0x38` | `0x865664` at 2 · column + bias 5 | `unitDrawDist` |
| `+0x2c` | `+0x3c` | `0x865684` at 2 · column + bias 4 | `SmallCull` |
| `+0x30` | `+0x3c` | `0x8656a4` at 2 · column + bias 4 | `DistCull` |
| `+0x34` | `+0x40` | `0xc518e0` plus `0x38` times the column | the device format record |
| `+0x38` | `+0x44` | the column | `baseMip` |

Addresses: `0x865634`, `0x865644`, `0x865664`, `0x865684`, `0x8656a4`, `0x6418a4`, `0x6418b6`, `0x6418cf`, `0x6418e8`, `0x641901`, `0x64191a`, `0x64192a`, `0x641939`, `0x639a60`, `0x639a31`, `0xc4e6ac`
Confidence: verified
Proof: one
Test: none

The tables hold the floats 0.3, 0.4, 0.5, 0.6, 0.9, 1.0, 1.0, 1.0 (`0x865644`), 35, 50, 50, 75, 75, 300, 300, 300 (`0x865664`), 0.08, 0.08, 0.07, 0.07, 0.07, 0.07, 0.04, 0.04 (`0x865684`) and 350, 350, 400, 400, 450, 450, 500, 500 (`0x8656a4`); the water table `0x865634` holds 0, 0, 0, 1 and ends where the particle table begins. `0x6418a4` stores the shader flag, `0x6418b6` indexes the water table, `0x6418cf` the particle table, `0x6418e8` the unit distance table,
`0x641901` the small cull table and `0x64191a` the distance cull table. `0x64192a` forms the format record address and `0x641939` copies baseMip. The apply pass (`0x639a60`, `display`) sets the settings from the block words; the format pointer at `0xc4e6ac` is the block word at `+0x34`.

## Default formats

The file builds seven device format records at start-up, in the array at `0xc518e0` with `0x38` bytes each: 640 × 480 and 800 × 600 with colour index 0 and depth index 4, then 640 × 480, 800 × 600, 1024 × 768, 1280 × 1024 and 1600 × 1200 with colour index 1 and depth index 5. All run at 60 Hz with the other fields of the fixed records in `display`. The column of a hardware row at `+0x40` selects one as the machine's default format.

Addresses: `0x6410d0`, `0x6410e0`, `0x641111`, `0x641141`, `0x641171`, `0x6411a1`, `0x6411d1`, `0x641201`, `0x641231`, `0x641240`, `0x641250`, `0x589900`, `0xc518e0`, `0xc51918`, `0xc51950`, `0xc51988`, `0xc519c0`, `0xc519f8`, `0xc51a30`, `0x641950`
Confidence: verified
Proof: one
Test: none

`0x6410e0` calls the format constructor `0x589900` seven times: with the sizes `0x280` by `0x1e0` (`0x641111`), `0x320` by `0x258` (`0x641141`), `0x280` by `0x1e0` (`0x641171`), `0x320` by `0x258` (`0x6411a1`), `0x400` by `0x300` (`0x6411d1`), `0x500` by `0x400` (`0x641201`) and `0x640` by `0x4b0` (`0x641231`), each with refresh `0x3c`. The first two pass colour 0 and depth 4, the rest colour 1 and depth 5. `0x641240` and `0x641250` register an exit function that does nothing.
`0x641950` stores the address of a row's default format in a record: the `+0x40` column times `0x38` plus `0xc518e0`.
