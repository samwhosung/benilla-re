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

Addresses: `0x545d50`, `0x579db0`, `0x648460`, `0x6477a0`, `0x648730`, `0x64cf80`, `0x64d0e0`, `0x85580c`, `0x82e490`, `0x82e460`, `0x82e440`
Confidence: verified
Proof: one
Test: none

`GameObjectDisplayInfo`'s `0x545d50`: the guard `0x545d5a`; open `0x545d6f`; magic `0x545da5`; count `0x545dd2`; the
12-field test `0x545dfe` and 48-byte test `0x545e36` (formats `0x82e490`, `0x82e460`); the allocation, `count·0x30 + string size`,
tag `0x85580c` line `0x4c` (`0x545e76`); `Read` at `0x545eb9`, the maximum `0x545ec3` to `0x545ed4`; the string read `0x545ef6`, with
`0x82e440` and `0x64d0e0` on failure; the id map `0x545f2f` (line `0x61`) and its fill `0x545f60` to `0x545f69`; the flag `0x545f7c`. Each of the 144 out-of-line loaders has one `[reg+reg·4]` store of the record pointer. The fatal messages are `%s has wrong number of
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

## Localized strings

A localized string is eight offset columns followed by a flags column, nine dwords. `Read` stores the eight offsets as pointers at the
column's first eight cells and the flags at the ninth. A consumer picks one with the locale index: `record + base·4 + locale·4`.
The locale index is the position of the configured country in the eight names `enUS`, `koKR`, `frFR`, `deDE`, `zhCN`, `zhTW`,
`esES`, `xxYY`; no match leaves it 0.

Addresses: `0x574070`, `0x57435c`, `0x8558a4`, `0xc0e080`, `0xc2a2a4`, `0x402540`, `0x402562`
Confidence: verified
Proof: one
Test: none

`AreaTable`'s `Read` reads eight offsets into temporaries (`0x5741ec` and before) and the flags into `+0x4c` (`0x574211`), then `0x57436d`
to `0x5743a0` store the eight pointers at `+0x2c` to `+0x48`. The names are the eight `char *` at `0x8558a4`; the loop at `0x402540`
compares the configured country (`0x64a4c0`) and stores the index at `0xc0e080` (`0x402562`); with no match nothing is stored and the word stays 0. The four packed
bytes of the country and language are stored at `0xc2a2a4` (`0x402537`).

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
