# models: records

The records of an MD20 file the client reads in place after the header fixup (`shared`): a view
with its submeshes and texture units, the sequences, and the bones. Each table lists the fields
the code reads, with a reading instruction in the derivation; a field no code reads is left out.
The light record is `lights`, the particle record `emission`, the ribbon record `ribbons`.

Roots: none

## View record

A view is 0x2c bytes: five count and pointer pairs and the bone count the vertex shader palette
must hold.

| offset | holds |
|---|---|
| `+0x00`, `+0x04` | vertex count, vertex lookup (16-bit indices into the header's vertices) |
| `+0x08`, `+0x0c` | index count, indices (16-bit, into the lookup) |
| `+0x14` | four bone lookup bytes per vertex |
| `+0x18`, `+0x1c` | submesh count, submeshes (`Submesh record`) |
| `+0x20`, `+0x24` | texture unit count, texture units (`Texture unit record`) |
| `+0x28` | bones the view's palette needs |

Addresses: `0x71dc80`, `0x71dc9a`, `0x71dd8d`, `0x71dab0`, `0x71dacc`, `0x71dbc6`, `0x71ddb4`, `0x71db5c`, `0x71db75`, `0x711230`, `0x7112c4`, `0x7112f3`, `0x71d6c0`
Confidence: verified
Proof: one
Test: none

`0x71dc9a` reads the count at `+0x0` and `0x71dd8d` the lookup at `+0x4`; `0x71dacc` and `0x71dbc6`
read `+0x8` and `+0xc`; `0x71ddb4` reads the bone bytes `+0x14`; `0x71db5c` and `0x71db75` the
submeshes; `0x7112c4` and `0x7112f3` the units. `0x71d6c0` compares `+0x28` with the palette size
(`shared#view-selection`).

## Submesh record

A submesh is 0x20 bytes.

| offset | holds |
|---|---|
| `+0x00` | id, 32-bit, the key `SetGeosetRange` compares |
| `+0x04`, `+0x06` | first vertex, vertex count (16-bit) |
| `+0x08`, `+0x0a` | first index, index count (16-bit) |
| `+0x0c`, `+0x0e` | bone count, first bone lookup (16-bit) |
| `+0x12` | the bone the depth key centre rides |
| `+0x14` | depth key centre, three floats |

Addresses: `0x711106`, `0x70cf1e`, `0x70cf22`, `0x70cf18`, `0x70cf14`, `0x70cc04`, `0x70cc14`, `0x707d6a`, `0x707d7a`
Confidence: verified
Proof: one
Test: none

`0x711106` reads `+0x0` as a dword for the unsigned range test (`model#geoset-visibility`).
The draw reads `+0x4`, `+0x6`, `+0x8` and `+0xa` with `movzx` (`0x70cf14` to `0x70cf22`); the
palette `+0xc` and `+0xe` (`0x70cc04`, `0x70cc14`). The depth key reads `+0x12` and `+0x14`
(`scene#element-depth-keys`).

## Texture unit record

A texture unit is 0x18 bytes and pairs a submesh with its material and textures.

| offset | holds |
|---|---|
| `+0x00` | flags byte: 4 a special batch when the scene has a special callback, 0x10 may join the grouped list |
| `+0x02` | priority plane, signed, copied to the element `+0x28` |
| `+0x04` | submesh index |
| `+0x08`, `+0x0a` | colour index, material index |
| `+0x0c` | layer: the unit's rank, 0 to n−1, among the view's n units on its submesh (`Texture unit layer`) |
| `+0x0e` | texture count, the stage loop bound |
| `+0x10`, `+0x12` | first texture lookup, first coordinate lookup |
| `+0x14`, `+0x16` | first transparency lookup, first transform lookup |

Addresses: `0x707b6f`, `0x707c19`, `0x707c76`, `0x707aa2`, `0x70afa3`, `0x707af0`, `0x70d352`, `0x70b75e`, `0x70b768`, `0x70b8a4`, `0x707b19`, `0x70b897`
Confidence: verified
Proof: one
Test: none

`0x707b6f` tests flag 4 before taking scene `+0x11c`, the special callback, which makes the element
kind 1; `0x707c19` tests 0x10 for the grouped kind 2 (`scene#element-collection`). `0x707c76` loads `+0x2` with `movsx`. The other
fields are `movzx` reads at the addresses listed, in table order; `0x70afa3` loads `+0xc` as a
word.

## Texture unit layer

A batch's unit, element `+0x2c`, is the record in the loaded file, or, for an instance with its
own batch table, a whole copy of the first record of its run (`model#own-batches`). The copy keeps
`+0x0c`, no other instruction of the model files writes it, and only the transparent order reads
it, as its last key before the opaque order (`scene#element-sort-orders`). In the shipped models
the `+0x0c` words of a view's n units on one submesh are 0 to n−1, each once: the unit's layer on
its submesh. Layers 0, 1 and 2 occur, and layer order is not always file order. Two independent
readings agree on the code and on the counts.

Addresses: `0x70afa3`, `0x70afa7`, `0x71cdc0`, `0x707a33`, `0x707ac1`, `0x707a9f`, `0x707c89`, `0x7115aa`, `0x7115b6`, `0x706440`, `0x71faa0`
Confidence: verified
Proof: two
Test: none

`0x71cdc0` stores file base plus offset in view `+0x24`. Element `+0x2c` (`0x707c89`) is `+0x24`
of the shared model's view `+0x138` (`0x707a33`) plus 0x18 times the index (`0x707ac1`), or an
entry of instance `+0x3ec` (`0x707a9f`), which `0x7115aa` copies as six dwords before `0x7115b6`
rewrites `+0x4`. The census of 16-bit accesses at `+0xc`, any addressing form, over `0x706440`
through `0x71faa0` finds 24: `0x70afa3` and `0x70afa7`, the control, 15 on submesh `+0xc`, five on
track states, a stack slot and a texture slot table. Registers loaded from element `+0x2c`, render
`+0x3338` and `+0x333c`, and view `+0x24` reach no other `+0xc`. With the patches in the client's
order (`storm/mount#patch-archives`), the install's 9,663 `.m2` files hold 38,652 views and 83,667
units; all 76,940 view and submesh groups hold 0 to n−1, the words 0, 1 and 2 occurring 76,940,
5,796 and 931 times. Layer order is not file order in 206 groups of 57 files, 65 of them between
two units of blend 2 or above; in the view selected at the 256-bone limit (`shared#view-selection`),
31 of 19,020 groups in 27 files, 14 of them between such units.

## Sequence record

A sequence is 0x44 bytes.

| offset | holds |
|---|---|
| `+0x04`, `+0x08` | start and end, milliseconds |
| `+0x10` | flags: 1 plays once |
| `+0x14` | probability among its variations |
| `+0x18`, `+0x1c` | minimum and maximum repeats |
| `+0x20` | blend time, milliseconds |
| `+0x24`, `+0x3c` | bounding box, six floats; bounding radius |
| `+0x40` | next variation, 16-bit |

Addresses: `0x7128af`, `0x7128b3`, `0x714585`, `0x7124b9`, `0x712692`, `0x7126a0`, `0x7125f2`, `0x709bff`, `0x7124cf`
Confidence: verified
Proof: one
Test: `models::m2_set_bone_timed_difftest`

Each read multiplies the index by 0x44. `0x7128b3` and `0x7128af` read `+0x4` and `+0x8`;
`0x714585` tests `+0x10` bit 0; `0x7124b9` subtracts `+0x14` from the draw; `0x712692` and
`0x7126a0` read `+0x18` and `+0x1c`; the blend reads `+0x20` (`0x7125f2`); `0x709bff` takes the box at `+0x24` with the radius
`0x18` past it; `0x7124cf` follows `+0x40`.

`m2_set_bone_timed_difftest` runs `0x7127f0` whole.

## Bone record

A bone is 0x6c bytes.

| offset | holds |
|---|---|
| `+0x00` | key bone id, −1 for none |
| `+0x04` | flags, or'd with the runtime flags |
| `+0x08` | parent bone, 16-bit |
| `+0x0c`, `+0x28`, `+0x44` | translation, rotation and scale tracks |
| `+0x60` | pivot, three floats |

Addresses: `0x712297`, `0x71962b`, `0x714260`, `0x7151b0`
Confidence: verified
Proof: one
Test: none

`0x712297` compares `+0x0` with −1; `0x71962b` reads the parent with `movzx`. The flags, tracks and
pivot are read by the bone transform (`animation#bone-transform`).
