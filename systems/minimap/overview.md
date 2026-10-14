# minimap: overview

The minimap engine of `Minimap.cpp`: the map tiles that fill the disc, the maps between world and tile, the catalog of
tile names, the interior tiles, the per-frame update that picks between them and the blip lists the widget draws. The
widget, its Lua methods, the zoom radius and the ping are `ui/minimap`; the blips and arrows are `ui/minimapblips`; the tile
draw of the interface frame is `draw`.

The docs split by topic: `coordinates` (the tile grid, the world and tile maps, the screen scale and the orientation), `tiles`
(the catalog, the names, the outdoor window, the interior tiles and the tile records), `update` (the per-frame driver and the
reset), `interior` (the query, the portal flood and the tile grid of a group) and `draw` (the tile quads, the interior target and its blit, and the tint). This doc holds the object file, its static objects and its containers.

Roots: `0x6d85d0`, `0x6d8620`, `0x6d8650`, `0x6d8680`, `0x6d86b0`, `0x6d86e0`, `0x6d8730`, `0x6d8770`, `0x6d87b0`, `0x6d8890`, `0x6d8970`, `0x6d89e0`, `0x6d8a40`, `0x6d8aa0`, `0x6d8b00`, `0x6d8b60`, `0x6d8bc0`, `0x6d9040`, `0x6d92c0`, `0x6d93a0`, `0x6d9e10`, `0x6d9e30`, `0x6d9ed0`, `0x6da150`, `0x6da180`, `0x6da330`, `0x6da380`, `0x6da540`, `0x6da5b0`, `0x6da5e0`, `0x6da7b0`, `0x6daa00`, `0x6dabc0`, `0x6dad00`

## Object file

`Minimap.cpp` runs from its first static initializer `0x6d85d0` to the end of `0x6dbbe0`, at 0x6dbce8: the initializers, the update with
its tile loaders, the blip code and the container templates it instantiates. The path `0x86f7d4` is passed to the allocator at `0x6dad3a`
and `0x6dadc3`. The code after it, from `0x6dbcf0`, is record serialisation and belongs to another file. The run in the initializer table
`0x827000` is the slots `0x82ce64` to `0x82cea4`: a use count, four products and twelve groups. The next run opens at `0x82cea8`
with the +∞ thunk `0x6dc3d0`.

Addresses: `0x6d85d0`, `0x6dbbe0`, `0x86f7d4`, `0x6dad3a`, `0x6dadc3`, `0x827000`, `0x82ce64`, `0x82cea4`, `0x82cea8`, `0x6dc3d0`, `0x6dbcf0`
Confidence: verified
Proof: one
Test: none

`0x6dad3a` and `0x6dadc3` load `edx` with the path before the allocations of the party blip code. The file has no copy of the shared
header constants: its run begins with the use count `0x6d85d0`, which counts the file as one more user of a shared object (`0x6d85e0` adds 1 to
0xc2a3c4 and calls `0x5b71c0` on the first user; `0x6d8610` subtracts 1 and calls `0x5b71d0` on the last).

## Static objects

The file builds twelve groups of static state with its own slots, each a slot function that calls a body and registers an exit
destructor with `_atexit`. Four more slots store products of constants: the half tile width `0xceaa94` and the half tile height `0xceaa90`
(533.3333 times 0.5, each from its own copy of the constant), and the map origins `0xceaa8c` and `0xceaa88` (533.3333 times 64 times 0.5, which is 17066.666).

| group | body | what it initializes |
|---|---|---|
| `0x6d86e0` | `0x6d86f0` | the cached player position `0xceaa7c` to `0xceaa84`, zero |
| `0x6d8730` | `0x6d8740` | the cached tile indices `0xceaa74` and `0xceaa78` of the window's first tile, zero |
| `0x6d8770` | `0x6d8780` | the cached tile indices `0xceaa6c` and `0xceaa70` of its last tile, zero |
| `0x6d87b0` | `0x6d87c0` | the orientation matrix `0xceaa20`, identity |
| `0x6d8890` | `0x6d88a0` | the second matrix `0xcea9e0`, identity |
| `0x6d8970` | `0x6d8980` | the window bounds `0xcea9c4` to `0xcea9d8`, zero |
| `0x6d89e0` | `0x6d89f0` | the three static blip records, each built by `0x573ac0`, and `0x6d8a20` destroys them with `0x573ad0` |
| `0x6d8a40` | `0x6d8a50` | the candidate array at 0xcea694, empty |
| `0x6d8aa0` | `0x6d8ab0` | the flag array at 0xcea688, empty |
| `0x6d8b00` | `0x6d8b10` | the flag array at 0xcea67c, empty |
| `0x6d8b60` | `0x6d8b70` | the in-range array at 0xcea668, empty |
| `0x6d8bc0` | `0x6d8bd0` | the catalog table `0xcea628` to `0xcea64c`, empty |

Addresses: `0x6d8620`, `0x6d8630`, `0x6d8650`, `0x6d8660`, `0x6d8680`, `0x6d8690`, `0x6d86b0`, `0x6d86c0`, `0xceaa94`, `0xceaa90`, `0xceaa8c`, `0xceaa88`, `0x6d86e0`, `0x6d8730`, `0x6d8770`, `0x6d87b0`, `0x6d8890`, `0x6d8970`, `0x6d89e0`, `0x6d8a40`, `0x6d8aa0`, `0x6d8b00`, `0x6d8b60`, `0x6d8bc0`, `0x6d85d0`, `0x6d85e0`, `0x6d8600`, `0x6d8610`, `0x6d8710`, `0x6d8720`, `0x6d8750`, `0x6d8760`, `0x6d8790`, `0x6d87a0`, `0x6d8870`, `0x6d8880`, `0x6d8950`, `0x6d8960`, `0x6d89c0`, `0x6d89d0`, `0x6d8a10`, `0x6d8a70`, `0x6d8ad0`, `0x6d8b30`, `0x6d8b90`, `0x6d8c90`
Confidence: verified
Proof: one
Test: none

Each slot function is `call body; jmp atexit-register`. The matrices store 1.0 on the diagonal (`0x3f800000` at `0xceaa20`, `0xceaa34`, `0xceaa48`, `0xceaa5c` and at `0xcea9e0`, `0xcea9f4`, `0xceaa08`,
`0xceaa1c`). The destructors of the three pointer arrays free their buffers under the tag of `AreaPOIRec` pointers (`0x6d8a80`, `0x6d8ba0`) or a second tag (`0x6d8ae0`, `0x6d8b40`). The catalog's destructor `0x6d8ca0`
empties the table and frees the buckets under the tag of its bucket lists. The products are `fld; fmul 0.5; fstp` (`0x6d8630`, `0x6d8660`) and `fld; fmul 64.0; fmul 0.5; fstp` (`0x6d8690`, `0x6d86c0`).

## Containers

The catalog and the pointer arrays are the hash table and list templates of `cvar/containers`. The table `0xcea628` (vtable `0x811720`, link offset `0xc`, bucket vector `0xcea63c` to `0xcea648`, mask `0xcea64c`)
holds `MINIMAPMD5NAME` records. Each function below has the same code as the twin named (`console/commands#table-twins`).

| this file | twin | does |
|---|---|---|
| `0x6dafd0` | `0x63e2b0` | frees a record (slot 0), unlinking its links first |
| `0x6db040` | `0x63e2e0` | allocates a record `0x40` bytes long and links it first in the bucket (slot 1) |
| `0x6db100` | `0x63e3a0` | clears the table (slot 3) |
| `0x6db1c0` | `0x63e460` | deleting destructor (slot 2) |
| `0x6db400` | `0x63e560` | empties a bucket and its terminator |
| `0x6db500` | `0x63e5f0` | frees the bucket vector |
| `0x6db5d0` | `0x63e520` | unlinks a link |
| `0x6db610` | `0x63e6d0` | returns an object's link |
| `0x6db630` | `0x63e6f0` | links a node |
| `0x6db7f0` | `0x63ed80` | resolves the next link's back slot |
| `0x6db820` | `0x63e9f0` | doubles the buckets when a chain is long |

Addresses: `0x6dafd0`, `0x6db040`, `0x6db100`, `0x6db1c0`, `0x6db400`, `0x6db500`, `0x6db5d0`, `0x6db610`, `0x6db630`, `0x6db7f0`, `0x6db820`, `0x63e2b0`, `0x63e2e0`, `0x63e3a0`, `0x63e460`, `0x63e560`, `0x63e5f0`, `0x63e520`, `0x63e6d0`, `0x63e6f0`, `0x63ed80`, `0x63e9f0`, `0xcea628`, `0xcea62c`, `0xcea630`, `0xcea63c`, `0xcea640`, `0xcea644`, `0xcea64c`, `0x811720`
Confidence: verified
Proof: one
Test: none

Each pair compares equal instruction by instruction with addresses masked. `0x6dafd0` differs from `0x63e2b0` only in having the record's destructor inline, and `0x6db040` from `0x63e2e0` in having the record's field clearing inline and the size
`0x40`. The table's fields are written by `0x6d8bd0`: the terminator `0xcea630`, the link offset placeholder `0xdddddddd` at `0xcea62c`, replaced by `0xc` at `0x6d8c6e`, and the mask `-1` at `0x6d8c82`.

## Array and list helpers

The rest of the templates: the list offset setter `0x6db490` (twin `0x640570`), the list constructor `0x6dba10` (`0x63e9c0`), the chain drain `0x6dba40` (`0x63ebe0`), the terminator reset `0x6dba90` (`0x63ec30`), the bucket count setter `0x6dbaa0` (`0x63ec40`),
the bucket reallocation `0x6dbbe0` (`0x63ee10`), the sizing helpers `0x6db6f0` (`0x63edb0`) and `0x6db730` (`0x63edf0`). `0x6db6f0` caps the granularity at 8, where `0x63edb0` caps it at 0x15. Three growable arrays use the same pattern:
`0x6db280` sets the capacity of a pointer array, `0x6db310` grows it by a count to a multiple of its granularity (at most 0x40), `0x6db750` sets the capacity of the arrow record array (`0x4c` bytes each) and `0x6db590` frees it.

Addresses: `0x6db490`, `0x6dba10`, `0x6dba40`, `0x6dba90`, `0x6dbaa0`, `0x6dbbe0`, `0x6db6f0`, `0x6db730`, `0x6db280`, `0x6db310`, `0x6db750`, `0x6db590`, `0x640570`, `0x63e9c0`, `0x63ebe0`, `0x63ec30`, `0x63ec40`, `0x63ee10`, `0x63edb0`, `0x63edf0`
Confidence: verified
Proof: one
Test: none

The same comparison finds `0x6db490`, `0x6dba10`, `0x6dba40`, `0x6dba90`, `0x6dbbe0` and `0x6db730` identical to their twins; `0x6dbaa0` holds a register differently from `0x63ec40`. `0x6db6f0` has the constant 8 at `0x6db6f6`, `0x6db719` and `0x6db720` where `0x63edb0` has 0x15. `0x6db280` reallocates to four bytes per entry under the `AreaPOIRec` pointer tag; `0x6db310` caps the granularity at 0x40 (`0x6db338`); `0x6db750` reallocates `0x4c` bytes per entry (`0x6db761`) under the tag `POIDIRECTIONDATA`.
