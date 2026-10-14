# playername: the object file

`PlayerName.cpp` draws the text that floats over world objects: the name block over a unit with
its raid-target marker, the combat numbers and words that rise over a unit, the honor rank icons
beside them, and the weapon swing trails. Its code runs from `0x6c63e0` to its last function `0x6c8ef0`.
Its run in the initializer table is slots `0x82cd58` to `0x82cd90`; the next run, from
`0x82cd94`, is the name filter. The loot currency code before it (`0x6c6220` to `0x6c63a0`) is a
different file. The other topic docs are `plates`, `worldtext` and `swing`.

The client reaches the file at world entry (`0x6c7470`, `0x6c8470`), once per frame
(`0x6c78a0`, `0x6c78b0`, `0x6c7900`), from the unit code that creates and dirties name records
(`0x6c76a0`, `0x6c77d0`, `0x6c77f0`, `0x6c7800`, `0x6c78f0`), from the combat code that submits
text (`0x6c7840`) and from the model code that attaches trails (`0x6c6920`, `0x6c6970`, `0x6c69c0`).

Roots: `0x6c68e0`, `0x6c68f0`, `0x6c6920`, `0x6c6970`, `0x6c69c0`, `0x6c7470`, `0x6c7630`, `0x6c76a0`, `0x6c77d0`, `0x6c77f0`, `0x6c7800`, `0x6c7840`, `0x6c78a0`, `0x6c78b0`, `0x6c78f0`, `0x6c7900`, `0x6c7950`, `0x6c8470`, `0x6c8520`, `0x6c8650`

## Infinity copies and pools

The file runs 15 static initializers from slots `0x82cd58` to `0x82cd90`. Three copy +∞ into
`0xce8700`, `0xce8790` and `0xce88d0`; none stores 2π. Four build block pools.

| slot | initializer | effect |
|---|---|---|
| `0x82cd58` | `0x6c63e0` | +∞ into `0xce8700` |
| `0x82cd5c` | `0x6c6400` | swing pool: records of 0x618 bytes, 8 per block |
| `0x82cd60` | `0x6c6450` | trail object pool: 8-byte records, 128 per block |
| `0x82cd64` | `0x6c69e0` | +∞ into `0xce8790` |
| `0x82cd74` | `0x6c6c40` | name pool: 0x38-byte records, 256 per block |
| `0x82cd78` | `0x6c7970` | +∞ into `0xce88d0` |
| `0x82cd80` | `0x6c7bc0` | text pool: 0x80-byte records, 30 per block |

Addresses: `0x827000`, `0x82cd58`, `0x82cd90`, `0x6c63e0`, `0x6c6400`, `0x6c6450`, `0x6c69e0`, `0x6c6c40`, `0x6c7970`, `0x6c7bc0`, `0xce8700`, `0xce8790`, `0xce88d0`, `0x6c63f0`, `0x6c6430`, `0x6c6480`, `0x6c69f0`, `0x6c6c70`, `0x6c7980`, `0x6c7bf0`
Confidence: verified
Proof: one
Test: none

Each thunk jumps to a body: `mov eax,[rdata]; mov [g],eax` for the +∞ copies, or a call to the
body followed by a jump that registers its destructor with `_atexit` for the pools. The slots
are read from the table at the addresses above and every target is checked against the
references to the table. The run ends where slot `0x82cd94` names `0x6c8fc0`, a regex function
with no call into this file.

## Table initializers

The other eight initializers fill constant tables and the empty name list, and clear the icon
ring.

| slot | initializer | effect |
|---|---|---|
| `0x82cd68` | `0x6c6a00` | marker corner table `0xce875c` |
| `0x82cd6c` | `0x6c6ab0` | marker UV offsets `0xce873c` |
| `0x82cd70` | `0x6c6b40` | empty name list `0xce8728` |
| `0x82cd7c` | `0x6c7990` | category table `0xce8828` |
| `0x82cd84` | `0x6c7c10` | shadow offsets `0xce8804`, `0xce8808` |
| `0x82cd88` | `0x6c7c60` | rank icon ring `0xce87f0` cleared |
| `0x82cd8c` | `0x6c86e0` | icon quad positions `0xce87c0` |
| `0x82cd90` | `0x6c8790` | icon quad UVs `0xce87a0` |

Addresses: `0x82cd68`, `0x6c6a00`, `0x6c6ab0`, `0x6c6b40`, `0x6c7990`, `0x6c7c10`, `0x6c7c60`, `0x6c86e0`, `0x6c8790`, `0xce875c`, `0xce873c`, `0xce8728`, `0xce8828`, `0xce8804`, `0xce87f0`, `0xce87c0`, `0xce87a0`, `0x6c6a90`, `0x6c6aa0`, `0x6c6b20`, `0x6c6b30`, `0x6c6b80`, `0x6c7ba0`, `0x6c7bb0`, `0x6c7c40`, `0x6c7c50`, `0x6c7c90`, `0x6c8770`, `0x6c8780`, `0x6c8800`, `0x6c8810`
Confidence: verified
Proof: one
Test: none

Each thunk calls its body and jumps to a registration of the body's destructor with `_atexit`;
the destructors return at once except the list's, which unlinks its records, and the ring's,
which frees its buffer. The bodies store immediates: `0x6c6a10` the marker corners, `0x6c6ac0`
the UV offsets, `0x6c79a0` the category rows, `0x6c7c20` 0.002 twice, `0x6c86f0` and `0x6c87a0`
the icon quad. Their values are in `plates` and `worldtext`.

## Block pools

Four pools of `CDataAllocator` records hold the file's objects: trail objects, swing records,
name records and text records. A pool hands out zeroed records through `0x760450` and takes them
back through `0x7604f0`; `0x760400` frees every block.

| pool | record | per block | holds |
|---|---|---|---|
| `0xce86d8` | 8 bytes | 128 | trail object: swing pointer and model |
| `0xce86ec` | 0x618 bytes | 8 | swing record |
| `0xce870c` | 0x38 bytes | 256 | name record |
| `0xce880c` | 0x80 bytes | 30 | text record |

Addresses: `0x6c6410`, `0x6c6460`, `0x6c6c50`, `0x6c7bd0`, `0x6c6773`, `0x6c693d`, `0x6c76ef`, `0x6c859c`, `0x760450`, `0x7604f0`, `0x760400`, `0xce86d8`, `0xce86ec`, `0xce870c`, `0xce880c`, `0x6c6440`, `0x6c6490`, `0x6c6c80`, `0x6c7c00`
Confidence: verified
Proof: one
Test: none

The four body functions each push the record size and the count per block and call `0x760390`
on the pool: `0x6c6410` pushes 0x618 and 8, `0x6c6460` 8 and 0x80, `0x6c6c50` 0x38 and 0x100,
`0x6c7bd0` 0x80 and 0x1e. The allocation sites name their tags: `0x6c6773` allocates from
`0xce86ec`, `0x6c693d` from `0xce86d8`, `0x6c76ef` from `0xce870c` and `0x6c859c` from
`0xce880c`. The shutdowns `0x6c68f0`, `0x6c7630` and `0x6c8520` free the blocks of the first
three and of the text pool.

## Unreached functions

`0x6c69a0` frees a swing record and `0x6c8690` stores a value at `+0x7c` of a text record. No
call, jump or stored address in the image reaches either one, and no other unreached code does.

Addresses: `0x6c69a0`, `0x6c8690`
Confidence: verified
Proof: one
Test: none

Census: every `e8` and `e9` whose target is the function, and every little-endian occurrence of
its address in every section, 0 for each. The same scan finds `0x6c6970` (3 call sites) and
`0x6c7840` (4) as controls. The two functions are not alignment-padded fall-throughs: each
follows a `ret`.
