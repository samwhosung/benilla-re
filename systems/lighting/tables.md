# lighting: tables

The five light tables as the day-night code reads them: the stores the loaders fill, the per-map
light list, the references between the tables, the band records and the conversions applied at
load. The loaders and record readers belong to the DBC stores (`dbc`); the lists, resolvers and
load conversions are the day-night file's.

Roots: `0x6d6160`, `0x6d6ad0`, `0x6d5c00`, `0x6d5cb0`, `0x6d5d70`, `0x6d5e30`, `0x6d5ef0`

## Table stores

Each table has a store of five dwords: the records, the record count, an id map, the largest id and
a loaded flag. A record is the file's columns in file order. The id map has largest id + 1 slots,
zeroed, and slot `id` points at the record whose first column is `id`, so every reference below is
an id used as an index, and an id with no record reads as null.

| table | store | columns, bytes | record |
|---|---|---|---|
| Light | `0xce9d60` | 12, 48 | id, map, x, y, z, inner radius, outer radius, five LightParams ids |
| LightParams | `0xce9d74` | 9, 36 | id, then eight columns the gather copies |
| LightFloatBand | `0xce9d88` | 34, 136 | id, key count, 16 key times, 16 floats |
| LightIntBand | `0xce9d9c` | 34, 136 | id, key count, 16 key times, 16 colours |
| LightSkybox | `0xce9db0` | 2, 8 | id, model path |

Addresses: `0x549700`, `0x5494d0`, `0x549030`, `0x549280`, `0x549940`, `0x589220`, `0x589030`, `0x5497ae`, `0x5497e6`, `0x549876`, `0x5498d2`, `0x5498fa`, `0x549919`, `0x5496d9`, `0x6d5c10`, `0x6d5cc0`, `0x6d5d80`, `0x6d5e40`, `0x6d5f00`, `0x6d5c50`
Confidence: verified
Proof: one
Test: none

The Light loader `0x549700` checks `WDBC`, 12 fields (`0x5497ae`) and 48-byte records (`0x5497e6`),
reads each record through `0x589220`, eight `SFile::Read` calls of 4 bytes and one of 20 into the
record in order, and keeps the largest first column in `+0xc` (`0x549876`). It allocates
`(largest + 1)·4` bytes (`0x5498d2`), zeroes them (`0x5498fa`) and stores each record's address at
`map[record[0]]` (`0x549919`). The LightParams loader `0x5494d0` checks 9 fields and 36 bytes and
builds its map the same way (`0x5496d9`); the band loaders check 34 and 136, the LightSkybox loader
2 and 8. The static constructors zero each store and set its largest id to −1, so before the load
every id is out of range; the destructors free the records and the map, and `0x6d5c50` frees each
LightSkybox path first.

## Per-map light list

Entering a map builds the light list `0xce98d8` from the Light records whose map column is the map
id. Slot 0 is the map's record at position (0, 0, 0); the other records follow from slot 1 in
table order. Without a record at the origin, slot 0 is the Light record with id 1, or null when the
largest id is below 1. The list never holds fewer than one slot.

Addresses: `0x6d6170`, `0x6d6188`, `0x6d61a9`, `0x6d61de`, `0x6d621a`, `0x6d6282`, `0x6d628d`, `0x6d62b2`, `0x6d62c9`, `0x6d1ba0`, `0x6d1bb5`, `0x6d6b30`, `0x6d6b70`, `0x6d6b90`, `0xce98d8`, `0xce98dc`, `0xce98e0`
Confidence: verified
Proof: one
Test: none

`0x6d6170(list, mapId)` counts from 1 (`0x6d6188 mov edi,1`) every record with `[row+4] == mapId`
(`0x6d61a9`) and a position with any coordinate not equal to 0.0 (`0x6d61de inc edi`; a NaN counts
as not equal), grows the array through `0x6d6b30`, `0x6d6b70` and `0x6d6b90` when needed, and
stores the count (`0x6d621a`). The second pass writes an origin record to slot 0 (`0x6d6282`,
last one wins) and the rest to slots 1 onward (`0x6d628d`). With no origin record it writes
`[[0xce9d68]+4]`, the id map's slot 1 (`0x6d62b2` to `0x6d62c9`). The light state's
initializer `0x6d1ba0` calls it at `0x6d1bb5` with the map id in edx.

## Light positions at load

At load the client converts every Light record not at the origin: it multiplies x, y, z and both
radii by f32(1/36) (0x3ce38e39) and moves the point into world axes, x' = c − z·(1/36),
y' = c − x·(1/36), z' = y·(1/36), with
c = 17066.666 when the map is not one object and 0 when it is. A record at (0, 0, 0) keeps its file
values, radii included.

Addresses: `0x6d6160`, `0x6d60c0`, `0x6d5fb0`, `0x6d5fe7`, `0x6d5fff`, `0x6d603f`, `0x6d6048`, `0x6d6062`, `0x53f4f0`, `0x66fc30`, `0xc9e384`
Confidence: verified
Proof: one
Test: `lighting::vec_scale_remap_difftest`

The DBC finalizer `0x53f4f0` jumps to `0x6d6160` with cl = 1, which runs `0x6d60c0` over every
Light record with that flag, then the band pass below. `0x6d5fb0` skips a record whose three
coordinates compare equal to 0.0 (`0x6d5fe7 jnp`), multiplies the five floats by 1/36
(0x3ce38e39, `0x6d5fff`), and asks `0x66fc30`, which returns whether the one-object map flag
`0xc9e384` is set (`0x6d603f`). Clear, both offsets are 0x46855555 (`0x6d6048`). `0x6d6062` loads
z − c and negates it, then x − c negated, stores the first at `+0x8` and the second at `+0xc`, and
moves y to `+0x10`; each store rounds to a float.

## Fog distances at load

At load the client multiplies every key value of every sixth LightFloatBand record, counted by
position in the file from 0, by f32(1/36) (0x3ce38e39): the records of scalar 0, the fog end, when the file holds the
bands of each LightParams group in id order with no gap. A record with no keys is untouched.

Addresses: `0x6d6100`, `0x6d6090`, `0x6d6118`, `0x6d6124`, `0x6d60a4`
Confidence: verified
Proof: one
Test: `lighting::dn_array_scale_36_difftest`

`0x6d6100` walks the records by index, `idiv` by 6 at `0x6d6118`, and calls `0x6d6090` only on a
zero remainder (`0x6d6124`), passing the flag in dl. `0x6d6090` returns when the flag is clear or
the count `[row+4]` is not positive, else multiplies `[row+0x48 + 4i]` by 1/36 for i below the
count, rounding each to a float (`0x6d60a4`). The selection is by record position, not by id.

## Band records

A band record holds up to 16 keys: the count at `+0x4`, the key times at `+0x8` as half-minutes of
the day in [0, 2880), and the values at `+0x48`, a float for a LightFloatBand record and a colour
for a LightIntBand record, its bytes blue, green and red with the fourth unused.

Addresses: `0x6d62ec`, `0x6d6313`, `0x6d6326`, `0x6d6341`, `0x6d6489`, `0x6d64a7`, `0x6d64b6`
Confidence: verified
Proof: one
Test: `lighting::dn_band_lerp_bgra_difftest`, `lighting::dn_band_lerp_scalar_difftest`, `lighting::dn_band_repack_half_boundary_difftest`, `lighting::dn_band_single_key_constant_difftest`, `lighting::dn_band_zero_key_commit_difftest`

Both evaluators read the count at `+0x4` (`0x6d62ec`, `0x6d6489`) and pass `row + 8` as the key
times (`0x6d6313`, `0x6d64a7`). The colour evaluator reads byte `+0x4a` of a value as one channel
(`0x6d6326`) and the dword at `+0x48` for the other two (`0x6d6341`); the scalar evaluator loads
the float at `+0x48` (`0x6d64b6`). 34 columns of 4 bytes leave room for 16 times and 16 values.
The wrap at 2880 is the key search's (`daynight#key-search`).

## Band references

A LightParams record with id g owns 18 colour bands and 6 scalar bands. Colour band k
(0 to 17) is the LightIntBand record with id 18·g − 17 + k; scalar band k (0 to 5) is the
LightFloatBand record with id 6·g − 5 + k. Light reference k (0 to 4) of a Light record is the
LightParams record whose id is in column 7 + k. Each resolver returns null for an id below 0 or
above the table's largest id.

Addresses: `0x6d6ab0`, `0x6d6ad0`, `0x6d6b00`, `0x6d6ad5`, `0x6d6b05`, `0x6d6aef`, `0xce9d7c`, `0xce9d80`, `0xce9d90`, `0xce9d94`, `0xce9da4`, `0xce9da8`
Confidence: verified
Proof: one
Test: none

`0x6d6ad0(row, k)` loads `[row]`, the id, and computes `k + 2·(9·id) − 0x11` (`0x6d6ad5`), bounded
by `jl` against 0 and `jg` against `[0xce9da8]` before indexing `[0xce9da4]`. `0x6d6b00` computes
`k + 2·(3·id) − 5` (`0x6d6b05`) against `[0xce9d94]` and `[0xce9d90]`. `0x6d6ab0(row, k)` takes
`[row + 0x1c + 4k]` against `[0xce9d80]` into `[0xce9d7c]`. The id is the first column because the
loader's map holds each record's base (`0x5496d9`).

## Skybox models

A skybox path is interned once: the first request for a path creates an entry in a name-keyed
hash table and loads the path as a model in the sky's own model scene, which it creates on first
use; later requests return the same entry. An empty or null path returns null.

Addresses: `0x6d4640`, `0x6d4658`, `0x6d466b`, `0x6d46a2`, `0x6d46f7`, `0x6d47b7`, `0x6d47d4`, `0x6d47f0`, `0xce99fc`, `0xce9a20`, `0xce9d54`, `0xce9d58`
Confidence: verified
Proof: one
Test: none

`0x6d4640(path)` returns 0 for a null or empty path (`0x6d4658`). It hashes the path
(`0x6d466b`), walks the bucket `hash & [0xce9a20]` comparing the stored hash and then the
name case-insensitively (`0x6d46a2`), and returns a match. Otherwise it sets the mask to 3 on first
use (`0x6d46f7`), inserts an entry, copies the path into `+0x14`, creates the scene `0xce9d54`
through `0x706e10` with its start time `0xce9d58` when there is none (`0x6d47b7`), and stores
`0x707350(scene, path, 0)` in `+0x18` (`0x6d47d4`). `0x6d47f0(path, weight)`, which would make the
result the override skybox, has no caller (`overview#unreached-code`).
