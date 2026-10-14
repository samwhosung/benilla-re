# terrain: overview

What the terrain system is, where its code lies, and what the rest of the client calls.

terrain is the client's map: it reads the WDT, ADT and WDL files, builds each chunk's mesh, layers,
alpha, shadow and liquid, keeps the doodad and map object placements, culls and draws the terrain
and its liquids, and answers the world's queries (heights, areas, liquid, facets, traces). Its
code is 30 object files from `0x66ee70` to `0x6c51b0` (`Object files`), with the `CWorld` entry
points in `World.cpp`. The map object and model files between them belong to models, the weather
file to lighting, the options file to cvar and the video file to gx. The topics are `files`,
`chunk`, `liquid`, `queries`, `collide`, `trace`, `cull`, `lod`, `render`, `statics`, `world`, `zones`,
`scene`, `camstate`, `waterfx`, `liquidpass`, `detail`, `lights`, `doodads`, `mapupdate`, `footprints`,
`portals`, `groupdraw`, `watershade`, `mapentities` and `entities`; 2,967 functions have rows, 945 of them static
initializers with their 1,147 helpers. The rest of the
client reaches the system through the `CWorld` functions, the query entries and a few shared
helpers, listed below; the static initializers are the roots in `statics`.

Roots: `0x66f6c0`, `0x66fb00`, `0x66fbe0`, `0x66fc30`, `0x66fc40`, `0x66fc70`, `0x66fc80`, `0x66fcc0`, `0x66fd50`, `0x66ff20`, `0x66ff60`, `0x6701b0`, `0x6701c0`, `0x6701d0`, `0x6701e0`, `0x670210`, `0x670220`, `0x670240`, `0x6703a0`, `0x6704a0`, `0x6704c0`, `0x670510`, `0x670520`, `0x670540`, `0x670570`, `0x6705d0`, `0x670630`, `0x670690`, `0x6706d0`, `0x6706f0`, `0x6707c0`, `0x670910`, `0x670950`, `0x6709a0`, `0x6709f0`, `0x670a30`, `0x670a70`, `0x670cf0`, `0x670d40`, `0x670d50`, `0x670da0`, `0x670db0`, `0x671010`, `0x671380`, `0x6716f0`, `0x6717d0`, `0x671a90`, `0x671ac0`, `0x671b30`, `0x671cc0`, `0x672170`, `0x6721b0`, `0x672390`, `0x6723b0`, `0x6723f0`, `0x672470`, `0x672490`, `0x6724c0`, `0x672570`, `0x6725a0`, `0x6725c0`, `0x6725d0`, `0x672630`, `0x672640`, `0x672650`, `0x672680`, `0x6726c0`, `0x672c40`, `0x672d80`, `0x672df0`, `0x672ef0`, `0x674130`, `0x6744c0`, `0x674500`, `0x67e450`, `0x67e460`, `0x67e510`, `0x67e9c0`, `0x686500`, `0x686820`, `0x686c20`, `0x686db0`, `0x687080`, `0x688260`, `0x688280`, `0x688d40`, `0x688d90`, `0x689110`, `0x6891f0`, `0x689250`, `0x6904b0`, `0x6904e0`, `0x690ae0`, `0x694130`, `0x694180`, `0x6953d0`, `0x699250`, `0x6992c0`, `0x69eb70`, `0x6a02c0`, `0x6a0330`, `0x6a03a0`, `0x6a03f0`, `0x6a0510`, `0x6a69f0`, `0x6a98e0`, `0x6acdd0`, `0x6adbf0`, `0x6b11f0`, `0x6b7070`

## Map pools

The map initializer `0x691b10` makes eight fixed-size pools, each pointer kept in a global, and the allocators
and releasers of `0x6a05d0` to `0x6a10a3` take from them. The record sizes and counts are:

| pool | record | count | global | class, vtable |
|---|---|---|---|---|
| `WAREA` | 0x6a8 | 8 | `0xca7e18` | area, `0x810fdc` |
| `WCHUNK` | 0xf34 | 0x200 | `0xca7e1c` | chunk, `0x810bf8` |
| `WDOODADDEF` | 0x198 | 0x1388 | `0xca7e20` | doodad def, `0x810a74` |
| `WENTITY` | 0x108 | 0x80 | `0xca7e24` | map entity, `0x810810` |
| `WMAPOBJDEFGROUP` | 0xdc | 0x80 | `0xca7e28` | map object group, `0x810918` |
| `WMAPOBJDEF` | 0x168 | 0x40 | `0xca7e2c` | map object def, `0x810928` |
| `WCHUNKLIQUID` | 0x40c | 0x40 | `0xca7e30` | chunk liquid |
| `WSOUNDEMITTER` | 0x58 | 0x40 | `0xca7e34` | sound emitter |

Addresses: `0x691b10`, `0x691eb8`, `0x691ef7`, `0x691f36`, `0x691f75`, `0x691fb4`, `0x691ff3`, `0x692032`, `0x692071`, `0x6a05d0`, `0x6a06e0`, `0x6a07f0`, `0x6a0930`, `0x6a0ca0`, `0x6a0db0`, `0x6a0ee0`, `0x6a1000`, `0x6c1cfb`, `0x6af3c4`, `0x6a7eb0`, `0x69d6b1`, `0x6a1d98`, `0x6a236c`, `0x810fdc`, `0x810bf8`, `0x810a74`, `0x810810`, `0x810918`, `0x810928`, `0x810ef4`, `0x810824`, `0xca7e18`, `0xca7e1c`, `0xca7e20`, `0xca7e24`, `0xca7e28`, `0xca7e2c`, `0xca7e30`, `0xca7e34`
Confidence: verified
Proof: one
Test: none

Each push of the pool name is followed by the element size in `ecx` and the count in `edx` before the pool
maker `0x463920` (`0x691eb8` is `WAREA`; the rest follow in the order of the table). Each constructor stores its vtable
in the first word: `0x6c1cfb`, `0x6af3c4`, `0x6a7eb0`, `0x69d6b1`, `0x6a1d98` and `0x6a236c` for the area, chunk,
doodad def, map entity, group and def. The map light's vtable `0x810ef4` is stored at `0x6c0951`. The classes share the
slot at +4 (`0x6c31e0`, the fog) and have 4 or 5 slots, packed with float constants between them; the table at
`0x810824` is a base table whose fifth slot is the runtime stub `0x40a2d7`.

## Object files

The terrain's 30 object files are: World.cpp; zone names; the scene and cull; liquid; Map.cpp;
the map load; two small map files; the map update; footprints; the point queries; the segment
trace; entities; MapMem.cpp; the placement groups; area unload; lights; the collision gather;
the box query; the chunk; DetailDoodad.cpp; the map render; heights; the low-detail areas; the
chunk draw; CMapLight; a small file; the area reader; chunk creation; the shared base; and the
chunk index builder. Each file's run in the initializer table opens with +∞, 2π and 1/2π, and
its code starts at its lowest initializer, its +∞ thunk for every file but the map render, whose
first own initializer `0x6b34c0` lies ahead of its +∞ thunk.

Addresses: `0x82bfb0`, `0x82bfd4`, `0x82bfd8`, `0x82bfdc`, `0x82c7cc`, `0x810528`, `0x81052c`, `0x66ee70`, `0x674520`, `0x67e390`, `0x67ef10`, `0x67ef30`, `0x6882f0`, `0x689e20`, `0x6913c0`, `0x6913e0`, `0x692b80`, `0x694070`, `0x697ac0`, `0x697c00`, `0x698110`, `0x698370`, `0x699910`, `0x699930`, `0x69b0e0`, `0x69b100`, `0x69bff0`, `0x69c0d0`, `0x69d450`, `0x69d470`, `0x69f1f0`, `0x69f220`, `0x6a1ca0`, `0x6a1db0`, `0x6a2c50`, `0x6a6780`, `0x6a7260`, `0x6a7280`, `0x6a98e0`, `0x6a9af0`, `0x6ad2c0`, `0x6ad310`, `0x6ae0d0`, `0x6ae0f0`, `0x6b1860`, `0x6b1880`, `0x6b34a0`, `0x6b34c0`, `0x6b7070`, `0x6b7280`, `0x6b8380`, `0x6bd010`, `0x6be940`, `0x6be960`, `0x6c07d0`, `0x6c07f0`, `0x6c15d0`, `0x6c1690`, `0x6c1b80`, `0x6c1ba0`, `0x6c2720`, `0x6c2860`, `0x6c3020`, `0x6c30d0`, `0x6c3330`, `0x6c4ca0`, `0x6c51b0`
Confidence: verified
Proof: one
Test: none

The addresses are each file's first and last function. The table from `0x827000` holds each file's
initializers together. Each of the 30 runs, and the 7 other files' runs among them from `0x82bb98`
to `0x82ccdc`, opens with a body copying the file's own +∞ literal, then 2π and 1/2π, the π
literal 4 bytes after the ∞ one: `0x82bfd8` copies `0x810528` and `0x82bfdc` doubles `0x81052c`
for the map load, after the BSP file's run `0x82bfb0` to `0x82bfd4`, whose own +∞ and π literals
open its run. Every slot of a run lies in its file's code but `0x82c7cc`, the map render's
`0x6b34c0`. A file ends where the next starts, except that the function just before the map object
reader's +∞ thunk is that file's: only its code references it. Four names come from source paths
passed to `SMemAlloc` and `SMemFree`: World.cpp (`0x867a50`), Map.cpp (`0x86a38c`), MapMem.cpp
(`0x86a798`) and DetailDoodad.cpp (`0x86b608`). Those 7 are the weather, options, video, BSP, map
object, map object group and map object reader files, in that order.

## Unreached functions

Nothing live reaches these 91 functions: the console toggles no command registers, the wave
field and its FFT, the sound emitter setter, an area id query, debug draws, and copies and no-ops the compiler
kept.

Addresses: `0x66fc60`, `0x670250`, `0x670470`, `0x671aa0`, `0x6723c0`, `0x672510`, `0x672520`, `0x672530`, `0x672540`, `0x672550`, `0x672560`, `0x672580`, `0x6730a0`, `0x6730f0`, `0x673130`, `0x673170`, `0x6731c0`, `0x673360`, `0x6733a0`, `0x6733e0`, `0x673430`, `0x673470`, `0x6734b0`, `0x6734f0`, `0x673530`, `0x673570`, `0x6735b0`, `0x6735f0`, `0x673670`, `0x6737c0`, `0x6737f0`, `0x673820`, `0x673850`, `0x673910`, `0x673960`, `0x6739f0`, `0x673b00`, `0x673bf0`, `0x680370`, `0x680380`, `0x680600`, `0x681d20`, `0x681ed0`, `0x682020`, `0x682370`, `0x686340`, `0x686b50`, `0x686bd0`, `0x686c90`, `0x68b410`, `0x68b600`, `0x68b890`, `0x68b900`, `0x68b990`, `0x68ba70`, `0x68bb20`, `0x68bee0`, `0x68bfd0`, `0x68c0f0`, `0x68c210`, `0x68f3f0`, `0x6922d0`, `0x692300`, `0x692310`, `0x6925e0`, `0x692630`, `0x694970`, `0x698d20`, `0x69b4a0`, `0x69b5d0`, `0x69da60`, `0x6a0110`, `0x6a01f0`, `0x6a1650`, `0x6a9b10`, `0x6a9cb0`, `0x6accb0`, `0x6accf0`, `0x6acd10`, `0x6af560`, `0x6aff70`, `0x6b23f0`, `0x6b2ac0`, `0x6b3b10`, `0x6b7c00`, `0x6b7d10`, `0x6bd430`, `0x6bda60`, `0x6bde30`, `0x6bde40`, `0x6c0cb0`
Confidence: verified
Proof: one
Test: none

The census (`cargo xtask census --all`) finds live code reaching 2,876 of the 2,967 functions: from
the 945 initializer slots, and from every terrain function that a `call`, `jmp`, pushed or stored immediate,
or a dword of a vtable or other table outside the terrain's files names, through calls, jumps, immediates
and table dwords. For each of the 91 left, a scan of the listing and of the
image's dwords for its address finds no reference, or one only from another of the 90. Three,
`0x6733e0`, `0x6734b0` and `0x6734f0`, start right after a `ret` with no padding between. `0x670250`'s only
caller, `0x4020a0`, is itself unreferenced, so it is no root.
