# minimap: tiles

How the minimap finds and loads its tile textures: the catalog that turns a logical tile name into a file, the two name forms,
the outdoor block of four tiles, the interior tiles and the records that hold them.

Roots: `0x6d9040`, `0x6da7b0`, `0x6da330`, `0x6da5e0`, `0x6d9ed0`, `0x6d9e30`, `0x6d9e10`, `0x6da150`

## Tile catalog

The tile files are not found by their logical names. The file `Textures\Minimap\md5translate.trs` maps each logical tile name to the file name that holds it,
and is read once for each world entry. A line is a name, a tab and a value. A line that begins with `dir:` and a line with no tab are skipped. A name already in the
catalog has its value replaced. A record holds the name, its hash and a value of at most 0x28 bytes. A lookup hashes the name with `SStrHashHT` and compares the stored hash and then the text without regard to case.

Addresses: `0x6d9040`, `0x6d9060`, `0x6d9079`, `0x6d90c7`, `0x6d90fd`, `0x6d9112`, `0x6d9127`, `0x6d9137`, `0x6d917d`, `0x6d91cf`, `0x6d91fe`, `0x6d922b`, `0x6d923a`, `0x6d923f`, `0x6d9275`, `0x6d928a`, `0x6d92a9`, `0x6d902f`, `0x6d8e10`, `0x6485f0`, `0x648700`, `0xcea628`, `0xcea644`, `0xcea64c`, `0x86f6a4`
Confidence: verified
Proof: one
Test: none

`0x6d9060` formats `%s\md5translate.trs` with the first entry of the string table at `0x86f6a4`, `Textures\Minimap`, and `0x6d9079` loads the file. `0x6d90c7` tokenizes the text with the delimiters CR and LF into a buffer of 0x104 bytes. `0x6d90fd` compares four bytes with
`dir:`; `0x6d9112` finds the tab and `0x6d9124` cuts the line there. A key found by the lookup at `0x6d9127` to `0x6d9184` has its value copied over; otherwise `0x6d91a6` hashes the key, builds four buckets and the mask 3 the first time (`0x6d91cf`), grows when a
chain is long (`0x6d91fe`), allocates with slot 1 (`0x6d922b`), links the record at the end of the list (`0x6d923a`, mode 2), stores the hash (`0x6d923f`) and duplicates the key (`0x6d9275`). The value is copied with `SStrCopy` and size 0x28 (`0x6d928a`). `0x6d92a9` frees the file. The caller is the end of `BuildBlipCandidates` (`0x6d902f`).

## Tile names

An outdoor tile is named `<directory>\map<row>_<column>.blp`, where the directory is the map's directory in `Map.dbc` and the row and column come from the tile maps (`coordinates#world-and-tile-maps`). An interior tile is named `<wmo>_<nnn>_<xx>_<yy>.blp`
from the world name, the group index in three digits and the two grid indices in two. The name is the catalog key. A hit gives the path `Textures\Minimap\<value>`; a miss writes the system message `No minimap texture: "<name>"` and leaves an empty path. A map with no directory writes
`NOCONTINENTNAME|<id>`. The world name is the group's WMO path without its first six bytes, `World\`, and without the extension after the last dot.

Addresses: `0x6da7b0`, `0x6da7bb`, `0x6da7c3`, `0x6da7c9`, `0x6da7e5`, `0x6da7e8`, `0x6da805`, `0x6da81d`, `0x6da85e`, `0x6da87f`, `0x6da8ae`, `0x6da8be`, `0x6da8c7`, `0x6d9f18`, `0x6d9fad`, `0x6d9fe3`, `0x6da330`, `0x6da337`, `0x6da359`, `0x6da365`, `0x6da36e`, `0x670510`, `0xc0daa8`, `0xc0daac`, `0x86f6a8`, `0x86f6ac`, `0xcea8c0`, `0x44cbd0`, `0x64a450`, `0x86f6a4`
Confidence: verified
Proof: one
Test: none

`0x6da7b0` clears the output (`0x6da7b8`), then needs the map held at `0x86f694` to be in range of the `Map.dbc` pointer table (`0xc0daa8`, the highest id at `0xc0daac`), its record and a non-empty directory string at `+4` (`0x6da7c3` to `0x6da7e8`); otherwise `0x6da8c7` writes the
message. `0x6da805` prints the name with the format at `0x86f6a8` (`%s\map%d_%d.blp`) from the directory and the two indices. `0x6da81d` hashes it and `0x6da85e` compares keys; `0x6da87f` writes the miss message and `0x6da8ae` composes `%s\%s`
from the string table and the value. The interior name is printed at `0x6d9f18` from the format at `0x86f6ac` (`%s_%03d_%02d_%02d.blp`), `0x6d9fad` is its miss message and `0x6d9fe3` its path. `0x6da330` asks `0x670510` for the WMO path (`0x6da337`), skips
`strlen("World\")` bytes (`0x6da341`, `0x6da349`), copies at most 0x104 bytes to `0xcea8c0` (`0x6da359`) and cuts at the last `.` found by `0x64a450` (`0x6da365`, `0x6da36e`).

## Outdoor tile window

The outdoor map is a block of 2 by 2 tiles held in the first four tile records. `LoadOutdoorTiles(block, changed, records)` clears mask `0x2` of the four records, and when `changed` is 0 (always, from its one caller `0x6da489`) moves a record whose tile is still wanted to its new slot, keeping its texture. Then it
loads every wanted tile that has no record: it builds the name, writes `MINIMAPCHUNKNOTFOUND|<row>|<col>` for a missing one, releases the slot's old texture, creates the new texture and stores the tile's two indices. The four tiles of the block are the base
tile plus the offsets (0, 0), (1, 0), (1, 1) and (0, 1).

Addresses: `0x6da5e0`, `0x6da5ef`, `0x6da600`, `0x6da60b`, `0x6da622`, `0x6da62b`, `0x6da643`, `0x6da64d`, `0x6da657`, `0x6da66f`, `0x6da675`, `0x6da6b0`, `0x6da6e1`, `0x6da701`, `0x6da711`, `0x6da764`, `0x6da76c`, `0x6da771`, `0x6da77a`, `0x6da8be`, `0x811700`, `0x44cbd0`, `0x449d90`, `0x41aed0`, `0x58a980`
Confidence: verified
Proof: one
Test: none

`0x6da600` clears mask `0x2` of the first four records (stride `0xa4`). `0x6da60b` skips the reuse pass when `edx` is nonzero. The pass compares each record's tile (`+0x98`, `+0x9c`) with the four wanted tiles, the base plus the pair `[0x811700 + 8k]`, `[0x811704 + 8k]` (`0x6da622`, `0x6da62b`, `0x6da643`, `0x6da648`);
a match in another slot releases the slot's handle (`0x6da657`), copies the handle and the key (`0x6da664`, `0x6da669`), zeroes the source handle (`0x6da66f`) and sets mask `0x2` (`0x6da675`). The loader loop is `0x6da6b0` to `0x6da796`: it skips records with mask `0x2`, calls the name builder (`0x6da6e1`), warns for an empty name (`0x6da701`),
releases the old handle (`0x6da711`) and creates the texture with `TextureCreate` (`0x6da764`), storing the handle and the key (`0x6da76c`, `0x6da771`, `0x6da77a`).

## Interior tiles

`LoadInteriorTile(records, group, desc, bias, nameBuf)` loads one tile of the interior descriptor `desc`: it claims a free record, builds the name from the world name and the descriptor (`desc[0]` the group index, `desc[1]` and `desc[2]` the grid indices), replaces the record's texture, stores the
key and the six bounding floats, and sets the distance: unbounded for the player's own group, otherwise the middle of the vertical extent less the bias. `TileLodDistance(records, group, desc, bias)` does the last step for a tile that is already loaded and returns its record, or 0.
`ClearTileKeys` sets the key word of all 512 records to −1. `ClaimTileSlot` returns the first record without mask `0x2`, with it set, or 0.

Addresses: `0x6d9ed0`, `0x6d9ee4`, `0x6d9f03`, `0x6d9ffa`, `0x6da05f`, `0x6da06b`, `0x6da07e`, `0x6da093`, `0x6da0b5`, `0x6da0c1`, `0x6da0ca`, `0x6da0dc`, `0x6da13a`, `0x6d9e30`, `0x6d9e40`, `0x6d9e75`, `0x6d9e88`, `0x6d9ea6`, `0x6d9eb3`, `0x6d9e10`, `0x6d9e20`, `0x6da150`, `0x6da160`, `0x6da179`, `0x6da150`, `0x449d90`
Confidence: verified
Proof: one
Test: `minimap::blip_lod_distance_difftest`

`0x6d9ed0` takes a record from `0x6da150` and returns when none is free (`0x6d9ee4`). `0x6d9ffa` releases the old handle; a non-empty path creates the texture and sets mask `0x4` (`0x6da06b`); an empty path clears mask `0x2` and the handle (`0x6da07e`). `0x6da093` stores `desc[1]` and `desc[2]` at `+0x98` and `+0x9c`, `0x6da0b5`
copies the six floats to `+0x70` and `0x6da0c1` stores `desc[0]` at `+0xa0`. A tile of the player's own group gets the distance 3.4028235e38 (`0x6da0ca`); another gets `(minZ + maxZ) · 0.5 − bias` (`0x6da0dc` to `0x6da13a`). `0x6d9e30` looks for the record with the key (`0x6d9e40`), returns 0 without one, sets mask `0x2` and gives the same two distances
(`0x6d9e75`, `0x6d9e88`, `0x6d9ea6`, `0x6d9eb3`). The test runs `0x6d9e30` whole over a table of records.

## Tile records

The update owns no tile storage; the frame code hands it a buffer of 512 records of `0xa4` bytes, which the draw reads. The fields the minimap uses are in the table.

| offset | holds |
|---|---|
| `+0x70` | the tile's bounding box: min x, y, z then max x, y, z |
| `+0x8c` | the distance that orders the interior tiles |
| `+0x90` | the texture handle, or 0 |
| `+0x94` | flags: 0x1 the draw tests it before it draws a quad, 0x2 claimed, 0x4 texture created |
| `+0x98`, `+0x9c` | the tile key: for an outdoor tile the row and column, for an interior one the two grid indices |
| `+0xa0` | the first key word: the group index of an interior tile, −1 for none |

Addresses: `0x4eae10`, `0x4eae19`, `0x6d97f6`, `0x6d9807`, `0x6d9810`, `0x6d9817`, `0x6d981f`, `0x6d982b`, `0x44acf0`
Confidence: verified
Proof: one
Test: none

The draw `0x4eae10` draws a record only when mask `0x1` and the handle are set (`0x4eae19`, `0x4eae26`). The update's promotion loop (`0x6d97f6` to `0x6d982b`) walks the 512 records: a record with both mask `0x2` and mask `0x4` whose handle is 0 loses mask `0x2` (`0x6d9807`); one with a handle asks
the texture object (`0x44acf0`, `0x6d9810`) and, when it answers, loses mask `0x4` (`0x6d981f`), and the loop gives up for this frame when it does not (`0x6d9817`). The frame code keeps them at 0xbc83a8.
