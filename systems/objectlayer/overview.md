# objectlayer: overview

The client's object model: the object manager, the objects the server describes, their update
fields, and the per-type code for items, containers, units, players, game objects, dynamic objects
and corpses, with the spell effects, missiles and combat log they carry.

The system owns five runs of code. ObjectMgrClient.cpp, `0x464190` to `0x46a250`, holds the manager
and the update packet. Ten small files, `0x47bee0` to `0x480360`, hold 64-bit copy helpers, each
type's field counts, the watched-field tables and the movement codec. The `Object/ObjectClient`
files run from `0x5d3f40` to `0x615710` and from `0x61b130` to `0x630780`, around the physics'
Movement.cpp (`collision`). UnitCombatLogXP_C.cpp runs from `0x637a80` to `0x6384e0`, after the
physics' resolution layer. The topics are `manager`, `update`, `fields`, `movement` and
`initializers` for the core, `classes` for the hierarchy, `reaction` for how units react to one
another, `trade` for the trade window, and `units`, `unitmovement`, `unitanim`, `auras`,
`unitdisplay`, `unitbody` and `unitnames` for Unit_C.cpp, `players`, `playerfields`,
`playermessages`, `quests`, `party`, `inventory`, `interaction` and `playercombat` for Player_C.cpp
and Bag_C.cpp, `items` for Item_C.cpp and the container, and `gameobjects`, `gameobjecttypes` and
`transports` for GameObject_C.cpp; `missiles`, `effects`, `unitcombat`, `sounds`, `combatlog`
and `combatlogmessages` for the missile, effect, combat, sound and log files; the other object
files have no topic of their own.

Roots: `0x4651a0`, `0x4672f0`, `0x4674a0`, `0x468460`, `0x468550`

## Object files

The runs are bounded by the source paths their functions pass, by the neighbouring systems'
first and last functions, and by the initializer runs at each file's edge.

Addresses: `0x464190`, `0x46a250`, `0x464140`, `0x464150`, `0x464160`, `0x464170`, `0x47bee0`, `0x480360`, `0x47bf60`, `0x47bfa0`, `0x47fea0`, `0x5d3f40`, `0x615710`, `0x61b130`, `0x630780`, `0x637a80`, `0x6384e0`, `0x638500`, `0x638510`, `0x82b74c`, `0x613ad0`, `0x6151f0`, `0x615700`, `0x61b130`, `0x5d3f20`, `0x615730`, `0x61b110`, `0x6307a0`, `0x637a60`
Confidence: verified
Proof: one
Test: none

Source paths: `ObjectMgrClient.cpp` from `0x46501c`, after its +∞ `0x464190` and ObjectAlloc.cpp's
2π and 1/2π `0x464140` and `0x464160` (bodies `0x464150`, `0x464170`); in the second run
`Trade_C.cpp` (`0x5d43ca`), `DynamicObject_C.cpp` (`0x5d54cd`), `Corpse_C.cpp` (`0x5d5fae`),
`Item_C.cpp` (`0x5d836a`), `Player_C.cpp` (`0x5db92e`), `GameObject_C.cpp` (`0x5f2f91`),
`Unit_C.cpp` (`0x5fa5a6`), `Object_C.cpp` (`0x614a11`), then `LootRoll.cpp` (`0x61b55b`),
`Missile_C.cpp` (`0x61cbf6`), `Effect_C.cpp` (`0x61f704`), `Bag_C.cpp` (`0x6224c7`),
`UnitSound_C.cpp` (`0x623658`), `UnitCombat_C.cpp` (`0x624811`), `UnitCombatLog_C.cpp`
(`0x62650a`), `PlayerSound_C.cpp` (`0x62fa5b`), and `UnitCombatLogXP_C.cpp` (`0x637a9f`). `net`'s
`0x5d3f20` ends just before the second run and `glue`'s run starts at `0x46a260`, after the first;
`collision` owns `0x615730` to `0x61b110` and `0x6307a0` to `0x637a60`. The nine field files are
ordered as `0x4641c0` reads their counts: corpse, dynamic object, game object, player, unit,
container, item, object (with the movement codec), and the watcher tables, whose run ends with the
header block `0x47fee0` to `0x480350`. A tenth file comes first: its run opens at slot `0x827d6c`
with its +∞ `0x47bee0` (0x803df0, the π at +4 doubled by `0x47bfa0`), and the 64-bit copy helpers
0x47bf00 to `0x47bf60` lie between. Object_C.cpp's run opens with its +∞ `0x613ad0` and ends with
its header block, `0x6151f0` to `0x615700`; LootRoll.cpp's opens with `0x61b130`. From `0x638500`,
the console file's +∞ (slot `0x82b74c`, body `0x638510`), the initializers are the console's.

## Starts the listing hides

Eighteen function starts lie where the shared listing, decoding linearly through a jump table, is
out of step, so the listing has no instruction at them. Each is a call target or a stored
address, and a disassembly from the address decodes a normal function.

Addresses: `0x465330`, `0x465970`, `0x4660a0`, `0x466af0`, `0x468240`, `0x5e1e70`, `0x5e59b0`, `0x5e5eb0`, `0x5e6140`, `0x5ebc90`, `0x5fd680`, `0x5fee80`, `0x601420`, `0x60dc30`, `0x60f510`, `0x6100a0`, `0x6142e0`, `0x61d790`
Confidence: verified
Proof: one
Test: none

The functions of the runs were found by sweeping each run from its first byte: decode from a start,
walk the body by the rule in `symbols/README.md`, skip its jump and index tables and padding, and
start the next function there. Every call target in the runs is a start so found; a `call` at
0x5e4b0c and a `jcc` at 0x6271d0 that the listing shows are table bytes, not instructions.
The eighteen are the starts with no line in `dumps/disasm.txt`; `0x465330`, for one, follows the
jump table `0x465314`.

## Switch tables

The code of the runs carries its switches' tables inline, after the function that jumps through
them. A dword table holds case addresses, every one inside the function that jumps through it,
and its length is the bounds check ahead of the jump plus one.

Addresses: `0x5e6a14`, `0x5e74dc`, `0x5ebc64`, `0x5f3b40`, `0x5f41e4`, `0x5f76cc`, `0x602114`, `0x60f4f8`, `0x612ca0`, `0x622794`, `0x623afc`, `0x62a8ec`, `0x62bde8`, `0x62be10`
Confidence: verified
Proof: one
Test: none

| table | entries | function | bound |
|---|---|---|---|
| `0x5e6a14` | 8 | `0x5e68b0` | `dec`, `cmp 7` |
| `0x5e74dc` | 14 | `0x5e7180` | `cmp 0xd` |
| `0x5ebc64` | 11 | `0x5eb900` | value − 4, `cmp 0xa` |
| `0x5f3b40` | 4 | `0x5f3930` | value − 0x92, `cmp 3` |
| `0x5f41e4` | 12 | `0x5f4120` | `cmp 0xb` |
| `0x5f76cc` | 31 | `0x5f6f30` | `cmp 0x1e` |
| `0x602114` | 4 | `0x6018f0` | `dec`, `cmp 3` |
| `0x60f4f8` | 5 | `0x60edf0` | `cmp 4` |
| `0x612ca0` | 6 | `0x612c50` | value + 2, `cmp 5` |
| `0x622794` | 67 | `0x622630` | `cmp 0x42` |
| `0x623afc` | 5 | `0x623a40` | `cmp 4` |
| `0x62a8ec` | 7 | `0x62a710` | value − 2, `cmp 6` |
| `0x62bde8` | 10 | `0x62bab0` | value − 2, `cmp 9` |
| `0x62be10` | 10 | `0x62bab0` | the same value, `cmp 9` |

## Indexed switch tables

Seven switches read a byte table first, which maps the switch value to a case of the dword table
just before it. The byte table's length is the bounds check plus one; the dword table has as many
entries as the byte table's largest value plus one, which is also the gap between the two.

Addresses: `0x5fc848`, `0x5fc7ec`, `0x5fd530`, `0x5fd51c`, `0x5fdd90`, `0x5fdd7c`, `0x60dc20`, `0x60dbfc`, `0x60ec98`, `0x60ec84`, `0x61d784`, `0x61d76c`, `0x62c320`, `0x62c318`
Confidence: verified
Proof: one
Test: none

| bytes → cases | entries | function | bound |
|---|---|---|---|
| `0x5fc848` → `0x5fc7ec` | 201 → 23 | `0x5fc3f0` | `dec`, `cmp 0xc8` |
| `0x5fd530` → `0x5fd51c` | 18 → 5 | `0x5fd460` | value − 2, `cmp 0x11` |
| `0x5fdd90` → `0x5fdd7c` | 162 → 5 | `0x5fdba0` | value − 0x27, `cmp 0xa1` |
| `0x60dc20` → `0x60dbfc` | 16 → 9 | `0x60d7c0` | `cmp 0xf` |
| `0x60ec98` → `0x60ec84` | 18 → 5 | `0x60ec00` | `cmp 0x11` |
| `0x61d784` → `0x61d76c` | 11 → 6 | `0x61d720` | `dec`, `cmp 0xa` |
| `0x62c320` → `0x62c318` | 58 → 2 | `0x62c2d0` | value − 0x32, `cmp 0x39` |

## Sound and log switch tables

UnitSound_C.cpp and the combat log carry more of the same tables: ten dword tables, among them
the six that turn a relation class into a chat type, and eight indexed ones.

Addresses: `0x623064`, `0x626c64`, `0x6274d0`, `0x6278a4`, `0x627db0`, `0x62959c`, `0x62a154`, `0x62a364`, `0x62a908`, `0x62b7f0`
Confidence: verified
Proof: one
Test: none

| table | entries | function | bound |
|---|---|---|---|
| `0x623064` | 13 | `0x623020` | `cmp 0xc` |
| `0x626c64` | 10 | `0x626be0` | `cmp 9` |
| `0x6274d0` | 10 | `0x6274a0` | `cmp 9` |
| `0x6278a4` | 10 | `0x627820` | `cmp 9` |
| `0x627db0` | 10 | `0x627d80` | `cmp 9` |
| `0x62959c` | 4 | `0x6294b0` | `dec`, `cmp 3` |
| `0x62a154` | 10 | `0x62a0d0` | `cmp 9` |
| `0x62a364` | 10 | `0x62a2e0` | `cmp 9` |
| `0x62a908` | 7 | `0x62a710` | the same value, `cmp 6` |
| `0x62b7f0` | 4 | `0x62b7d0` | `cmp 3` |

## Sound and log indexed switches

The indexed tables of UnitSound_C.cpp, UnitCombat_C.cpp and the combat log follow the same shape.

Addresses: `0x623b64`, `0x623b58`, `0x6247ac`, `0x624794`, `0x625b0c`, `0x625aec`, `0x626c98`, `0x626c8c`, `0x6271e4`, `0x6271d0`, `0x6278d8`, `0x6278cc`, `0x62a188`, `0x62a17c`, `0x62a398`, `0x62a38c`
Confidence: verified
Proof: one
Test: none

| bytes → cases | entries | function | bound |
|---|---|---|---|
| `0x623b64` → `0x623b58` | 10 → 3 | `0x623b10` | `cmp 9` |
| `0x6247ac` → `0x624794` | 21 → 6 | `0x6246a0` | `cmp 0x14` |
| `0x625b0c` → `0x625aec` | 186 → 8 | `0x6255b0` | value − 0x143, `cmp 0xb9` |
| `0x626c98` → `0x626c8c` | 10 → 3 | `0x626be0` | `cmp 9` |
| `0x6271e4` → `0x6271d0` | 87 → 5 | `0x626dd0` | value − 3, `cmp 0x56` |
| `0x6278d8` → `0x6278cc` | 10 → 3 | `0x627820` | `cmp 9` |
| `0x62a188` → `0x62a17c` | 10 → 3 | `0x62a0d0` | `cmp 9` |
| `0x62a398` → `0x62a38c` | 10 → 3 | `0x62a2e0` | `cmp 9` |

## Unreached functions

Nothing live reaches these 118 functions. Their rows keep a one-line meaning; those
without a recorded derivation are `inferred`.

Addresses: `0x4685c0`, `0x468610`, `0x47d5b0`, `0x47eb80`, `0x47ee80`, `0x47f1a0`, `0x47f9c0`, `0x47fae0`, `0x47fc10`, `0x47fc40`, `0x47fc60`, `0x47fd40`, `0x5d52e0`, `0x5d5760`, `0x5d5770`, `0x5d57a0`, `0x5d6020`, `0x5d6680`, `0x5d66b0`, `0x5d66e0`, `0x5d78f0`, `0x5d7900`, `0x5d7910`, `0x5d7920`, `0x5d7930`, `0x5d7940`, `0x5d7960`, `0x5d7970`, `0x5d7980`, `0x5d79b0`, `0x5d92e0`, `0x5d9310`, `0x5d9340`, `0x5d9a40`, `0x5d9b70`, `0x5d9ed0`, `0x5da250`, `0x5e09a0`, `0x5e0a60`, `0x5e0b50`, `0x5e1400`, `0x5e1d40`, `0x5e1df0`, `0x5e1e00`, `0x5e1e30`, `0x5e9950`, `0x5ea380`, `0x5ea710`, `0x5eae50`, `0x5eb1f0`, `0x5eb220`, `0x5eb2b0`, `0x5eb300`, `0x5eb330`, `0x5ed330`, `0x5ed410`, `0x5ee0e0`, `0x5ee130`, `0x5f3680`, `0x5f3710`, `0x5f4280`, `0x5f7f50`, `0x5f7f60`, `0x5f7f90`, `0x5f8120`, `0x5f8140`, `0x5fccd0`, `0x6010d0`, `0x602650`, `0x602f30`, `0x603210`, `0x603220`, `0x603250`, `0x606080`, `0x6060a0`, `0x606940`, `0x6079b0`, `0x608c20`, `0x608dd0`, `0x608f40`, `0x60b580`, `0x60bba0`, `0x60c700`, `0x60c710`, `0x60c720`, `0x60c730`, `0x60d600`, `0x611670`, `0x6116a0`, `0x612530`, `0x613930`, `0x614300`, `0x614330`, `0x614340`, `0x614380`, `0x6143d0`, `0x614410`, `0x614450`, `0x614470`, `0x614490`, `0x6144b0`, `0x620010`, `0x620100`, `0x6219f0`, `0x621a10`, `0x621a50`, `0x621a60`, `0x621a70`, `0x621a90`, `0x621ad0`, `0x621af0`, `0x621b40`, `0x621b50`, `0x6220f0`, `0x622110`, `0x622360`, `0x62e9c0`, `0x62ec60`
Confidence: verified
Proof: one
Test: none

The census is two scans over the whole image: every `e8` or `e9` whose target is the function,
and every dword equal to its address in any section, each site then placed in the function or the
table that holds it. A site in an unreached function, in a table whose installing code is
unreached, or in resource data does not count: `0x606060` and `0x606080` also appear side by
side among a bitmap palette's colour entries in `.rsrc`, the file's only copy of `0x606080`; the rest make a function live, and liveness
propagates through calls, jumps and stored addresses. The controls are `0x468460`, with 1,090
`call` sites, and `0x4651a0`, stored once, at `0x46514e`. A vtable slot that only unreached
code calls leaves its bodies unreached: the game object handlers' slot 16 is called only by
`0x5f8140`.

## Small bodies

These functions are a single load, store, return or jump, short enough that the row's meaning
is the whole body: an empty return, a constant, a member or global dword, byte or float of the
object in `ecx`, a member's address, a tail call through the vtable, or a jump.

Addresses: `0x467d00`, `0x469bb0`, `0x469ed0`, `0x469f00`, `0x469f10`, `0x469f50`, `0x469f80`, `0x469f90`, `0x469fa0`, `0x469fb0`, `0x469fc0`, `0x469fd0`, `0x469fe0`, `0x469ff0`, `0x46a000`, `0x46a050`, `0x46a060`, `0x46a070`, `0x46a080`, `0x5d52d0`, `0x5d52e0`, `0x5d5760`, `0x5d5f40`, `0x5d5f50`, `0x5d76c0`, `0x5d78e0`, `0x5d78f0`, `0x5d7900`, `0x5d7910`, `0x5d7920`, `0x5d7930`, `0x5d7970`, `0x5d7f30`, `0x5d8430`, `0x5d8bf0`, `0x5e1d40`, `0x5e1df0`, `0x5ea700`, `0x5ea710`, `0x5ed410`, `0x5ed420`, `0x5efc90`, `0x5f1260`, `0x5f1270`, `0x5f1280`, `0x5f1ea0`, `0x5f1eb0`, `0x5f1ec0`, `0x5f1f70`, `0x5f1f80`, `0x5f3660`, `0x5f3670`, `0x5f3680`, `0x5f36f0`, `0x5f3700`, `0x5f3710`, `0x5f3770`, `0x5f37b0`, `0x5f47f0`, `0x5f4890`, `0x5f48a0`, `0x5f48b0`, `0x5f48c0`, `0x5f4900`, `0x5f4930`, `0x5f4940`, `0x5f4b70`, `0x5f4b80`, `0x5f57e0`, `0x5f57f0`, `0x5f5c70`, `0x5f65f0`, `0x5f6600`, `0x5f6660`, `0x5f6670`, `0x5f68a0`, `0x5f6d40`, `0x5f6ea0`, `0x5f6eb0`, `0x5f7f50`, `0x5f94e0`, `0x5f95c0`, `0x5f9d90`, `0x5f9da0`, `0x5f9db0`, `0x5f9dc0`, `0x5f9dd0`, `0x5f9de0`, `0x5f9df0`, `0x5f9e00`, `0x5f9e10`, `0x5f9e20`, `0x5f9e50`, `0x5f9e60`, `0x5f9e70`, `0x5f9e80`, `0x5f9e90`, `0x5f9ea0`, `0x5f9eb0`, `0x5f9ec0`, `0x5f9ed0`, `0x5f9ee0`, `0x5f9ef0`, `0x5f9f00`, `0x5f9f10`, `0x5fa070`, `0x6010d0`, `0x602e50`, `0x603210`, `0x605d50`, `0x605d60`, `0x605d70`, `0x605d80`, `0x605d90`, `0x607ea0`, `0x608a00`, `0x60c700`, `0x60c710`, `0x60c720`, `0x60c730`, `0x60f8f0`, `0x611660`, `0x613650`, `0x613660`, `0x613670`, `0x614330`, `0x61e9c0`, `0x61eef0`, `0x61ef00`, `0x61f5a0`, `0x621b50`, `0x621b60`, `0x625450`, `0x626620`, `0x62f860`, `0x62f870`, `0x6301c0`, `0x6301d0`, `0x6301e0`, `0x6301f0`, `0x630200`, `0x630210`, `0x630220`, `0x630230`
Confidence: verified
Proof: one
Test: none

Each body was matched whole against one pattern, such as `mov eax,[ecx+off]; ret` or
`xor eax,eax; ret N`, after decoding from the function's start; a body with any other
instruction is not in this list. The offsets and constants in the meanings are the pattern's
operands.
