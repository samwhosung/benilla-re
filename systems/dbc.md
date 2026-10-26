# dbc

The client's database tables, the `DBFilesClient\*.dbc` files, read at start-up into stores that the rest of
the client indexes by id. There are 151 tables: `Startup_Strings` and `VideoHardware`, which have loaders of their
own, and the 149 stores listed in `tables/dbc_stores.tsv`, 144 of them loaded by one driver. Every table is a
`WDBC` file: a header, fixed-size records, a string block. Each record type has its own small object file with its
file name, `Read` and the shared header's initializers; the stores, their loaders and their start-up and exit
functions are one template, `WowClientDB<T>`. The record layouts are `tables/dbc_schemas.tsv`.

Roots: `0x53f8b0`, `0x545d50`, `0x579db0`, `0x579d80`, `0x579da0`, `0x53aa40`, `0x4567f0`, `0x456820`

## The store

A table lives in a store of five dwords: the records (`+0`), the record count (`+4`), an id map (`+8`), the
largest id (`+0xc`) and a loaded flag (`+0x10`). The id map holds `largest id + 1` pointers, each at the index
of its record's first column; an unused id is null. A start-up constructor zeroes the store and sets the
largest id to −1; users compare an id with `+0xc` and index `+8`.

Addresses: `0xc0dce4`, `0x53aa40`, `0x53aa50`, `0x53aa80`, `0x53aa90`, `0x5f4035`
Confidence: verified
Proof: one
Test: none

`0x53aa40` is the static initializer thunk (`call 0x53aa50; jmp 0x53aa80`); `0x53aa50` writes `+0`, `+8`, `+4`, `+0x10` with 0 and `+0xc` with
−1 (`0x53aa52` to `0x53aa6b`); `0x53aa80` registers the exit function `0x53aa90` with `atexit`, which runs only when the loaded flag is set, calls
each record's destructor (`0x579da0`), frees the record block (tag `..\..\DB/WowClientDB.h`, line `0x8e`) and
the id map. Its user `0x5f4035` tests the id against the store's `+0xc` (`cmp eax,…; jg`) and indexes `+8`. Census over the 149 stores:
144 have a `-1` store at `+0xc` in a 33-byte constructor with one thunk caller and an exit function; the five `Light*`
stores are not matched by it.

## The loader

`WowClientDB<T>::Load(store)` does nothing when the loaded flag is set. It opens the table's file (a failed open is the fatal
error `0x85100079`), reads the magic and returns, leaving the store unloaded, unless it is `WDBC` (`0x43424457`); reads the record
count into `+4` and returns on 0; reads the field count and record size and fails fatally on a mismatch with the
type's; allocates the records and string block as one block; reads each record with the type's `Read`, ignoring its
result, and keeps the largest first column in `+0xc`; reads the string block; closes the file; allocates and zeroes the
id map and stores each record's address at its first column; sets the loaded flag.

Addresses: `0x554b00`, `0x545d50`, `0x579db0`, `0x648460`, `0x6477a0`, `0x648730`, `0x64cf80`, `0x64d0e0`, `0x85580c`, `0x82e490`, `0x82e460`, `0x82e440`, `0x5408d0`, `0x540b00`, `0x540d40`, `0x540f70`, `0x5411b0`, `0x5413f0`, `0x541620`, `0x541840`, `0x541a70`, `0x541cc0`, `0x541ef0`, `0x542120`, `0x542360`, `0x5425a0`, `0x5427e0`, `0x542a20`, `0x542c50`, `0x542e90`, `0x5430d0`, `0x543300`, `0x543540`, `0x543780`, `0x5439b0`, `0x543bf0`, `0x543e20`, `0x544060`, `0x544290`, `0x5444d0`, `0x544700`, `0x544930`, `0x544b70`, `0x544da0`, `0x544fe0`, `0x545220`, `0x545470`, `0x5456b0`, `0x5458e0`, `0x545b10`, `0x545f90`, `0x5461c0`, `0x5463f0`, `0x546620`, `0x546860`, `0x546a90`, `0x546cc0`, `0x546f00`, `0x547130`, `0x547360`, `0x547590`, `0x5477d0`, `0x547a00`, `0x547c30`, `0x547e70`, `0x5480c0`, `0x548300`, `0x548530`, `0x548760`, `0x548990`, `0x548bc0`, `0x548df0`, `0x549030`, `0x549280`, `0x5494d0`, `0x549700`, `0x549940`, `0x549b70`, `0x549db0`, `0x549fe0`, `0x54a220`, `0x54a470`, `0x54a6b0`, `0x54a8e0`, `0x54ab30`, `0x54ad60`, `0x54afa0`, `0x54b1d0`, `0x54b400`, `0x54b640`, `0x54b870`, `0x54bab0`, `0x54bce0`, `0x54bf20`, `0x54c150`, `0x54c380`, `0x54c5c0`, `0x54c7f0`, `0x54ca30`, `0x54cc60`, `0x54cea0`, `0x54d0e0`, `0x54d320`, `0x54d560`, `0x54d7b0`, `0x54d9e0`, `0x54dc20`, `0x54de60`, `0x54e0a0`, `0x54e2e0`, `0x54e510`, `0x54e750`, `0x54e980`, `0x54ebc0`, `0x54ee00`, `0x54f040`, `0x54f280`, `0x54f4b0`, `0x54f6e0`, `0x54f920`, `0x54fb50`, `0x54fd90`, `0x54ffd0`, `0x550220`, `0x550460`, `0x550690`, `0x5508e0`, `0x550b20`, `0x550d50`, `0x550f90`, `0x5511c0`, `0x551400`, `0x551640`, `0x551880`, `0x551ab0`, `0x551cf0`, `0x551f20`, `0x552150`, `0x552390`, `0x5525c0`, `0x552800`, `0x552a30`, `0x552c70`, `0x552eb0`, `0x5530f0`, `0x553330`, `0x553570`, `0x5537b0`, `0x5539f0`, `0x553c40`, `0x553e70`, `0x5540a0`, `0x5542d0`, `0x554500`, `0x554740`
Confidence: verified
Proof: one
Test: none

`GameObjectDisplayInfo`'s `0x545d50`: the guard `0x545d5a`; open `0x545d6f`; magic `0x545da5`; count `0x545dd2`; the
12-field test `0x545dfe` and 48-byte test `0x545e36` (formats `0x82e490`, `0x82e460`); the allocation, `count·0x30 + string size`,
tag `0x85580c` line `0x4c` (`0x545e76`); `Read` at `0x545eb9`, the maximum `0x545ec3` to `0x545ed4`; the string read `0x545ef6`, with
`0x82e440` and `0x64d0e0` on failure; the id map `0x545f2f` (line `0x61`) and its fill `0x545f60` to `0x545f69`; the flag `0x545f7c`. The inline loads of five tables in `0x53f8b0` read their header dwords through `0x554b00`, a four-byte `SFile::Read` (calls `0x53f91a`, `0x53fb2c`, `0x53fd43`, `0x53ff54`, `0x54016c`).
Census over the 144 out-of-line loaders (the addresses above): each has the `WDBC` compare, the call to `SFile::Open`, the allocations of lines `0x4c` and `0x61`, the loaded-flag store
and one `[reg+reg·4]` store of the record pointer, and the field count and record size it compares are the ones in `tables/dbc_schemas.tsv` for its table. The fatal messages are `%s has wrong number of
columns (found %i, expected %i)`, `%s has wrong row size (found %i, expected %i)` (`0x82e490`, `0x82e460`) and `%s: Cannot read string table` (`0x82e440`).

## Load order

All tables are loaded before the login screen is built, in three places: the inline load of `Startup_Strings` in the
start-up routine `0x402350`; `VideoHardware` through `0x63a230`; and 149 by `StaticDBLoadAll`, which
`ClientDBInitialize` (`0x53f4f0`, called from `0x402adf`) calls first. `0x53f8b0` runs five loads inline
(`AnimationData`, `AreaPOI`, `AreaTable`, `AreaTrigger`, `AttackAnimKits`), then 144 times `mov ecx,<store>; call <loader>`, ending with `ZoneMusic`,
with no branch. `ClientDBInitialize` then runs four fix-ups, `0x53f510`, `0x53f5c0`, `0x53f640` and `0x588bf0`, and jumps to
`0x6d6160` with `cl = 1`. No other code calls a loader.

Addresses: `0x402350`, `0x53f4f0`, `0x53f8b0`, `0x63a230`, `0x63a26b`, `0x641260`, `0x53f510`, `0x53f5c0`, `0x53f640`, `0x588bf0`, `0x6d6160`
Confidence: verified
Proof: one
Test: none

`0x53f4f0` is seven instructions (`0x53f4f0` to `0x53f50b`). In `0x53f8b0` the 144 calls (`0x53fac0` on) all
target `0x5408d0` to `0x554740`; the last pair is `0x5408b7`, `0x5408bc`. Census: the 145 loader addresses (the 144 and `0x53f8b0`) are each the
target of exactly one `e8` or `e9` (the 144 from inside `0x53f8b0`, `0x53f8b0` from `0x53f4f0`), and none occurs as a
dword in the image. The one caller of `0x641260` is `0x63a26b`.

## Damage class check

`0x53f510` builds a pointer array over the `Resistances` rows, in record order, once the tables are loaded. A table of other
than 7 rows prints `Warning: The Resistances table has the wrong number of entries`, and at most 7 rows
are taken. The array at `0xc0d530` is sized to that count and holds a pointer to each row in record
order, and `0x855808` takes the position of the last row whose `Flags` (`+0x4`) has bit 0; with none it
stays −1.

Addresses: `0x53f510`, `0x53f51b`, `0x53f52c`, `0x53f537`, `0x53f552`, `0x53f56c`, `0x53f598`, `0x53f59e`, `0x53f5a9`, `0x53f4f5`, `0xc0d530`, `0x855808`
Confidence: verified
Proof: one
Test: none

`0x53f51b` stores −1 in `0x855808` first. The count is compared with 7, `0x53f52c` calls `ConsolePrintf`
with the warning, and `0x53f537` caps the count at 7. The array is `{capacity, count, base}`: for 0 rows
`0x53f552` frees it and zeroes the three words, otherwise `0x53f56c` reallocates it to the count, which
is stored at `+4`. The loop steps the row by 0x30 bytes; `0x53f598` tests `[row+4]` bit 0 and `0x53f59e`
stores the index, so a later flagged row replaces an earlier one; `0x53f5a9` stores the row pointer. The
one caller is `ClientDBInitialize` (`0x53f4f5`).

## Record files

Each table's record type has an object file of ten functions, in this order: a file-name getter (`mov eax,<string>; ret`),
a constructor that returns its record (`mov eax,ecx; ret`), a destructor (`ret`), `Read`, and the three
header initializers (+∞, 2π, 1/2π), each a `jmp` thunk and a body. Each `Read` is the only code that names its
`Error reading <Type>Rec` string, and the file name in the getter is the type's.

Addresses: `0x579d80`, `0x579d90`, `0x579da0`, `0x579db0`, `0x579e60`, `0x579e70`, `0x579e80`, `0x579e90`, `0x579eb0`, `0x579ec0`, `0x4567f0`, `0x456820`
Confidence: verified
Proof: one
Test: none

The example is `GameObjectDisplayInfo` (`0x579d80` returns `0x858b00`). `0x579e70` stores π + π in `0xc0e5b4`, `0x579e90` stores
1 divided by it in `0xc0e5b0` and `0x579ec0` copies +∞ to `0xc0e5b8`. Census over the 1,514 functions from `0x573000` to `0x58a000`: 149
getters, 149 constructors, 150 destructors, 149 each of the two arithmetic initializers and 150 of the +∞ copy, 454 thunks; 151 `Error reading` strings
(`Startup_Strings` and `VideoHardware` included), each referenced by exactly one function, and each getter's table the one its
`Read` names. The control is the 149 groups of getter file name against `Read` error string: no mismatch. Three tables have extra functions: `AnimationData`, `ZoneMusic`
and `LightSkybox`.

## Record reads

A `Read(this = record, file, stringBase)` calls `SFile::Read` (`0x648460`) once for each column, in order, into the
record or a stack temporary, with the column's size in bytes; a read that returns 0 logs `Error reading <Type>Rec` and
makes `Read` return false. Columns that are string offsets go to temporaries and, on success, are stored as `stringBase + offset`,
or the address of an empty string (`0x882748`) when `stringBase` is null. A record's size is the sum of the sizes, its
field count the number of dwords, a byte counting one; `tables/dbc_schemas.tsv` holds both and the string columns.

Addresses: `0x579db0`, `0x648460`, `0x574070`, `0x882748`, `0x63cb50`, `0x6485d8`, `0x53fd6d`, `0x53fd9b`
Confidence: verified
Proof: one
Test: none

`AreaTable`'s `Read` (`0x574070`) makes 21 four-byte reads and one of 16 bytes, 25 fields of 100 bytes, which its inline load checks (`0x53fd6d`,
`0x53fd9b`). `GameObjectDisplayInfo`'s `0x579db0` reads an id (4 bytes), a name offset (4) and `Sound[10]` (40) into `+0`, a temporary
and `+8`, and stores `stringBase + offset` at `+4` (`0x579e2e` to `0x579e34`). `SFile::Read` is not a column reader: it tests the
file's kind at `[file+0]` and jumps through `0x6485d8` (`0x6484b9`): kind 0 `_fread`, 1 `0x6476e0`, 2 and 3 the archive read
`0x656b40`, 4 the zip read `0x668750`.

## Six readers column by column

Each of these `Read`s makes every one of its column reads, a failed one included, before it tests
any: the result is false when any read returned 0, and then no string column is stored. The scalar
columns land in the record as they are read, so a failed record keeps what its reads wrote. A
string column holds `stringBase + offset` in 32 bits, or the empty string `0x882748` in every
string column when `stringBase` is null.

| reader | record | reads, in order |
|---|---|---|
| `CharHairGeosets` `0x5753b0` | 24 | six dwords |
| `CharSections` `0x575540` | 40 | six dwords, three strings at `+0x18` to `+0x20`, a dword |
| `CharacterFacialHairStyles` `0x575a80` | 36 | six dwords, 12 bytes at `+0x18` |
| `WeaponImpactSounds` `0x587450` | 92 | three dwords, 40 bytes at `+0xc`, 40 at `+0x34` |
| `LightParams` `0x589030` | 36 | nine dwords |
| `Spell` `0x583750` | 692 | 118 reads; four localized strings at `+0x1e0`, `+0x204`, `+0x228`, `+0x24c` |

Addresses: `0x5753b0`, `0x575476`, `0x575540`, `0x575687`, `0x5756a5`, `0x5756ca`, `0x575a80`, `0x575b67`, `0x587450`, `0x5874f5`, `0x589030`, `0x589159`, `0x583750`, `0x58497a`, `0x584998`, `0x584b69`, `0x648460`, `0x63cb50`, `0x882748`
Confidence: verified
Proof: one
Test: `dbc::char_hair_geosets_read_difftest`, `dbc::char_sections_read_difftest`, `dbc::character_facial_hair_styles_read_difftest`, `dbc::spell_read_difftest`, `dbc::weapon_impact_sounds_read_difftest`, `dbc::light_params_read_difftest`

Each read's result goes to a register or a stack slot; after the last call `neg; sbb; inc` and
`test; sete` per result OR a failure flag together, and one `je` (`0x575476`, `0x575687`, `0x575b67`,
`0x5874f5`, `0x589159`, `0x58497a`) skips `ConsoleWrite(text, 0)` (`0x63cb50`) with the reader's
`Error reading` line and `xor al,al`. `0x5756a5` and `0x584998` test `[ebp+0xc]`; null goes to
`0x5756ca` and `0x584b69`, which store `0x882748` in every string column. The four readers without
strings never load `[ebp+0xc]` (`0x575540` does once, the control). The record offsets run on from 0
in read order, the scalars at the record and the strings through stack temporaries. The `Spell`
reads are 40 dwords, 8, 32 and 32 bytes, three dwords, eighteen reads of 12, 8, three dwords, the 36
columns of the strings (four runs of eight offsets, each followed by a dword, to `+0x26c`), then 4, 4, 4, 4, 4, 8, 4, 4,
4, 4, 12, 4, 4 and 4. The tests run each reader whole with `SFile::Read` and `ConsoleWrite` stubbed:
each column failing alone, a null base, wrapping offsets and a fuzz.

## Spell visual columns that are read

No reader found touches three columns of the spell visual tables: the effect name's `+0x0c`, the kit's `+0x04` and the visual's `+0x20`. Each table's `Read` fills every column, so they are loaded and unused.

Addresses: `0xc0d730`, `0xc0d744`, `0xc0d758`, `0x61fdd0`, `0x5d52f0`, `0x5d53cf`, `0x5d57c0`, `0x60edf0`, `0x60d7c0`, `0x60a3d0`, `0x60d450`
Confidence: inferred
Proof: none
Test: none

`SpellVisual` has 16 fields in 64 bytes (store `0xc0d730`, id index `0xc0d738`), `SpellVisualKit` 35 in 140 (`0xc0d744`, `0xc0d74c`) and `SpellVisualEffectName` 5 in 20 (`0xc0d758`, `0xc0d760`, largest id `0xc0d764`).
The effect constructor `0x61fdd0` reads the effect name's `+0x08`, the model path (`0x61fe5e`), and keeps the record at `+0x20` of the effect (`0x61fe0c`); `CGDynamicObject_C::ModelLoaded` (`0x5d52f0`)
reads `+0x10` as the divisor of the object's radius (`0x5d53cf`), the record coming from `0x5d57c0`. Neither, nor the 21 loads of `0xc0d760`, reads `+0x0c`. The kit player `0x60edf0` reads `+0x08` (`0x60ee57`),
`+0x0c` to `+0x30` (`0x60f01d` for the last), `+0x34` (`0x60ee3d`) and `+0x38` (`0x60ee2e`), and passes the record to `0x60d7c0` (`0x60f358`), which takes `+0x3c` on; it never reads `+0x04`.
The missile launcher `0x60a3d0` reads the visual's `+0x1c` (`0x60a409`), `+0x24` (`0x60a523`) and `+0x28` (`0x60a510`) and not `+0x20`, which the merge `0x60d450` only copies (`0x60d518` to `0x60d522`).
The absences are of the readers named: callees past them were not read. Column meanings are in `systems/spell/shakes.md` and `systems/objectlayer/auras.md`.

## Reading Wow.ini

Start-up loads `Wow.ini` into a profile (`0x4442c0`, `0x444460`), read to its first NUL by lines.
`//` opens a comment. `[name]` makes `name` the section, ignoring what follows `]`; with no `]`
the section stays. Any other line is a key, ending at the first `=` after its first character, then
the value, nothing trimmed. A line with no `=`, or before any section, adds nothing. Names compare
ignoring case. Each line appends its value's pieces to its key: a `,` ends a piece, so `a,,b` gives an
empty middle one; a piece opening with `"` runs to the next `"`, a `,` right after it skipped, and any
other text there starts the next piece. An empty value appends nothing.

Addresses: `0x4442c0`, `0x444460`, `0x444480`, `0x444507`, `0x44451f`, `0x444540`, `0x4445f0`, `0x44463c`, `0x444661`, `0x444671`, `0x44469b`, `0x4446c8`, `0x4446e5`, `0x444710`, `0x44471e`, `0x444732`, `0x444750`, `0x444769`, `0x44476d`, `0x444779`, `0x444790`, `0x4448d9`, `0x44488e`, `0x444e10`, `0x402496`
Confidence: verified
Proof: one
Test: none

`0x444460` refuses a null name (error 0x57) and jumps to `0x444480`, which copies the name into 260
bytes, cuts trailing `__ismbcspace` characters with no lower bound (spaces alone walk below the buffer),
loads the file with `SFile::LoadByName` (`0x444507`), parses it (`0x44451f`) and frees the buffer, or
returns 0. `0x444540` walks its NUL-terminated copy (`0x4446c8`) through six states: line start
(`0x4445f0`: CR or LF stays, `//` by a two-byte `SStrCmp`, `[`, else a key), comment and after `]`
(`0x444661`), section (`0x44463c`: a line end restores the last closed section, `]` closes it), key
(`0x444671`: a line end drops it, `=` ends it) and value (`0x44469b`: a line end hands it to
`0x444710`). A state takes effect from the next character, so a key's first character is never its `=`.
`0x4446e5` hands over a value open at the end. `0x444710` returns for a null section (`0x44471e`) or an
empty value (`0x444732`), else calls `0x444790(section, key, piece, 0, 1)` per piece (`0x444779`); a `,`
ends a piece (`0x444750`, `0x44476d`), and after a closing quote the next starts at once (`0x444769`).
The 0 skips the clearing at `0x4448d9`. `0x444790` finds a section by `SStrHashHT` and `0x444e10`, a key
by `SStrCmpI` (`0x44488e`), creating either.

## Wow.ini values

`0x4452c0(section, key, index)` returns the key's value at `index`, or null when the section, the key
or the index is missing; `0x4452f0` finds the key. `0x4455d0` empties its buffer, then copies the value
with `SStrCopy` to the buffer's size and returns 1, or returns 0 with no value. `0x445410` stores 0,
then the value as an integer and returns 1: a value opening with `'` packs the up to four characters
that follow, up to a `'` or the end, unsigned and first character highest; any other is `SStrToInt`.
Start-up reads value 0 of `Region` in `[WoW Config]` into `0x882730`, which keeps 0 without the file
or the key, and `0x6c91a0` copies it to `0x86c9e0`, the name charsets' region.

Addresses: `0x4452c0`, `0x4452d6`, `0x4452f0`, `0x4452f8`, `0x445356`, `0x4453cb`, `0x445410`, `0x44542c`, `0x445470`, `0x445480`, `0x445497`, `0x44549c`, `0x4455d0`, `0x4455ec`, `0x445612`, `0x4024a4`, `0x4024bd`, `0x40256a`, `0x445900`, `0x882730`, `0x402b79`, `0x6c91a0`, `0x6c91b8`, `0x86c9e0`
Confidence: verified
Proof: one
Test: `dbc::wow_ini_value_int_difftest`

`0x4452c0` compares the index with the count `+0x1c` of the key (`jae` at `0x4452d6`) and reads the
array `+0x20`. `0x4452f0` returns 0 for an empty table (mask −1, `0x4452f8`) and compares the section
and key names at `0x445356` and `0x4453cb`, each after a pointer-equality test. `0x445410` and
`0x4455d0` raise error 0x57 for a null section, key or output; their stores before the lookup are
`0x44542c` and `0x4455ec`, and `0x445612` is the copy. `0x445470` runs `shl eax,8; or eax,edx` with
`movzx` over at most four bytes (`0x445480` to `0x445497`), else calls `SStrToInt` (`0x44549c`).
`0x4024a4` skips every read when the load fails; `0x4024bd` passes `WoW Config`, `Region`, `0x882730`
and index 0; `0x40256a` deletes the profile through `0x445900`, slot 0 of its vtable with 1. Census of
the instructions that name `0x882730`: the push at `0x4024ac` and the reads at `0x402b79` and
`0x46a510` (control: the same scan finds the one store to `0xc0e080`, `0x402562`); its file bytes
are 0. `0x402b79` passes it in `ecx` to `0x6c91a0`, which stores it at `0x6c91b8`.

## Localized strings

A localized string is eight offset columns followed by a flags column, nine dwords. `Read` stores the eight offsets as pointers at the
column's first eight cells and the flags at the ninth. A consumer picks one with the locale index: `record + base·4 + locale·4`.
Start-up loads `Wow.ini`, reads `Language` and then `Country` from `[WoW Config]`, and joins them Language first into a tag of at most
four characters: `en` and `GB` give `enGB`. The tag's bytes are packed into `0xc2a2a4`, and the locale index is the tag's position
among `enUS`, `koKR`, `frFR`, `deDE`, `zhCN`, `zhTW`, `esES`, `xxYY`, compared ignoring case; no match, as for `enGB`, leaves it 0.
Without the file or either key neither word is written.

Addresses: `0x574070`, `0x57435c`, `0x8558a4`, `0xc0e080`, `0xc2a2a4`, `0x402496`, `0x4024a4`, `0x4024d6`, `0x4024f7`, `0x402513`, `0x402528`, `0x402537`, `0x402540`, `0x402551`, `0x402562`, `0x444460`, `0x4455d0`
Confidence: verified
Proof: one
Test: none

`AreaTable`'s `Read` reads eight offsets into temporaries (`0x5741ec` and before) and the flags into `+0x4c` (`0x574211`), then `0x57436d`
to `0x5743a0` store the eight pointers at `+0x2c` to `+0x48`. `0x402496` opens `Wow.ini` through `0x444460`, which loads it with `SFile::LoadByName`; a
zero return skips to `0x402568` (`0x4024a4`). `0x4024d6` and `0x4024f7` call `0x4455d0` with `WoW Config` and `Language`, then `Country`,
into 16-byte buffers, each zero return skipping too. `0x402513` is `SStrPrintf(tag, 5, "%s%s", language, country)`. `0x402528` packs
up to four bytes as `eax = (eax << 8) + byte`, each byte sign-extended, the first byte highest, and `0x402537` stores it. The loop at
`0x402540` calls `SStrCmpI(tag, name, 0x7fffffff)` (`0x402551`) over the eight `char *` at `0x8558a4` and stores the first match's
index at `0xc0e080` (`0x402562`); with no match nothing is stored. `0x402562` is the one store to `0xc0e080`: of the 397
instructions that name it, the others read it.

## Startup strings

`Startup_Strings.dbc`, 11 fields of 44 bytes, is loaded by the same template inlined in `0x402350`, with its store at `0x8824fc`, before the console
device. Row 1's string in the locale's column, the field at `+8 + locale·4`, is the title it passes to `0x63a230`; with no row
the title is `World of Warcraft`.

Addresses: `0x402350`, `0x4567f0`, `0x456820`, `0x8824fc`, `0x882500`, `0x88250c`, `0x63a230`, `0x82e42c`
Confidence: verified
Proof: one
Test: none

`0x402580` to `0x402797` is the load, into the store at `0x8824fc`: record reads through `0x456820` with a constructor call `0x456800`, the
maximum in `+0xc`, the id map in `+8`, the loaded flag in `+0x10`. `0x4027a1` to `0x4027c4` take the id map's entry for id 1, `0x4027c6`
loads the default, and `0x4027e7` calls `0x63a230` with the string.
