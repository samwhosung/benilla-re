# dbcache

The client's caches of records the server streams: creatures, game objects, items, quests, names, guilds, pages,
pets and petitions. `DBCache.cpp`, `0x554b10` through `0x5737b0`, holds thirteen cache objects of one template,
`DBCache<T>`; the record files `CreatureStats.cpp`, `GameObjectStats.cpp`, `ItemName.cpp`, `ItemStats.cpp` and `NPCText.cpp` (`0x7c8120`
on) hold five of the twelve record decoders. A caller asks a cache for a record by id; on a miss the cache sends a
query and keeps the caller's callback; the server's response is decoded into a node, the callbacks run, and nine
caches save their records to `WDB\<name>.wdb`. The template instantiations are clones: 13 of each method.

Roots: `0x554ff0`, `0x554f50`, `0x555740`, `0x55f080`, `0x560e30`, `0x562a40`, `0x564730`, `0x55ba30`

## Caches that are saved

Nine caches persist their records. Each is constructed at start-up with a four-character file tag, a file name, the query
opcode, a dword and two flags, and its store address is the base of an object of `0x3c` bytes.

| store | file | tag | query opcode | record bytes | version |
|---|---|---|---|---|---|
| `0xc0e354` | `creaturecache.wdb` | `BOMW` | `0x60` | `0x34` | 1 |
| `0xc0e318` | `gameobjectcache.wdb` | `BOGW` | `0x5e` | `0x7c` | 1 |
| `0xc0e2dc` | `itemnamecache.wdb` | `BDNW` | `0x2c4` | 4 | 1 |
| `0xc0e2a0` | `itemcache.wdb` | `BDIW` | `0x56` | `0x1d4` | 5 |
| `0xc0e264` | `npccache.wdb` | `CPNW` | `0x17f` | `0x140` | 1 |
| `0xc0e1b0` | `questcache.wdb` | `TSQW` | `0x5c` | `0x18dc` | 3 |
| `0xc0e174` | `pagetextcache.wdb` | `XTPW` | `0x5a` | `0x404` | 1 |
| `0xc0e0c0` | `itemtextcache.wdb` | `XTIW` | `0x243` | `0x1f40` | 1 |
| `0xc0e084` | `wowcache.wdb` | `NDRW` | 0 | 8 | 1 |

Addresses: `0x554b40`, `0x556060`, `0x5566d0`, `0x558180`, `0x559c10`, `0x55b650`, `0x55d120`, `0x562630`, `0x564320`, `0x569570`, `0x56b040`, `0xc0e354`, `0xc0e318`, `0xc0e2dc`, `0xc0e2a0`, `0xc0e264`, `0xc0e1b0`, `0xc0e174`, `0xc0e0c0`, `0xc0e084`
Confidence: verified
Proof: one
Test: none

The constructor calls are `0x554b40` and the nine like it: six pushes (persist flag, owner flag, a dword, opcode, file name,
tag) and `mov ecx,<store>`. Each cache type has its own copy of the constructor, which stores the dword at `+0x34`: `0x57` for the item cache
(`0x554c47` calls `0x55b0a0`, store `0x55b160`) and 0 for every other cache (the name cache's copy stores it at `0x55e760`); nothing in the cache code reads it back. Tags are dwords, so `BOMW` is the bytes `42 4f 4d 57`.

The file is written by the exit function of each cache (`0x5566d0` for creatures), which the cache's `_atexit` thunk jumps to
(`0x554b60` registers `0x554b70`, which is `jmp 0x5566d0`). `0x5576d0`, a second writer of the creature file, has no branch or
dword reference in the image (control: `0x5566d0` has the thunk's `jmp`), so it never runs. The saved file is in the next two entries.

## The .wdb file

A file is a 20-byte header, then the records. The header is five dwords: the tag, the build `0x16f3`, the locale tag `[0xc2a2a4]`,
the record size of the cache's type and the version. The path is `WDB/<file name>`, the directory made first (`0x42a960`).
The body is a sequence of entries `{key, size, record}`, the key a dword, `size` the length of the encoded record, and ends with two zero
dwords.

Addresses: `0x5566d0`, `0x5576d0`, `0x5567cc`, `0x5567d9`, `0x5567e8`, `0x5567f2`, `0x5568f9`, `0x7c8080`, `0x5568ac`, `0x417e10`, `0x6ca120`, `0x56b162`
Confidence: verified
Proof: one
Test: none

The save writes the header from `0x5567a9` on with `CDataStore::PutUInt32`: the tag (`0x5567c7`), the build push `0x16f3` (`0x5567cc`), the locale
(`0x5567d9`), the record size push (`0x5567e8`, `0x34` for creatures) and the version push (`0x5567f2`, 1 for creatures). The wow cache's version
push is the call `0x6ca120` (`0x56b162`; its load compares the file's with the call at `0x56bff3`), which is `mov eax,1; ret`, so its version is 1; the item cache's is 5, the quest cache's 3 and the
others' 1 (the table above). A failed open for writing (`0x556721`) is logged and the save skips the writing.
Per node, walking the list at `+0xc`, an entry is written only when the valid byte `+0x50` is set and the byte `+0x60` is clear (`0x556861` to `0x55686b`): the key
copy `+0x4c`, a zero size, the record encoded by the type's writer (`0x7c8080` for creatures), and the size patched in place by `0x417e10(position 4,
length − 8)` at `0x5568ac`. `0x7c8080` puts the four names and the subname as strings, seven dwords, and two flag bytes as 0 or 1. The two zero dwords are written at `0x5568f9` to `0x55691d`, then the file is closed.

## Loading a .wdb file

The load returns at once for a cache whose persist byte is 0. Otherwise it reads the 20-byte header and compares the tag, the build, the locale, the record size and the
version with its own. Any difference closes the file and leaves the cache empty; the file stays on disk. The body is read entry by entry until a key of 0, each record decoded by the
type's reader into a node found or created for the key, with the valid byte `+0x50` set. A short read clears the nodes already loaded.

Addresses: `0x5572b0`, `0x557342`, `0x557390`, `0x5573ae`, `0x5573c6`, `0x5573e7`, `0x55742e`, `0x5574be`, `0x55761a`, `0x557665`, `0x56bff3`
Confidence: verified
Proof: one
Test: none

`0x5572b0` is the creature cache's load; each cache's has its own constants. It opens the file and reads
the 20 bytes (`0x557342`, short read: close and return). It compares the tag with `+0x28` (`0x557390`), the build with `0x16f3` (`0x5573ae`), the locale
with `[0xc2a2a4]` (`0x5573c6`), the record size with the type's (`0x34`, `0x5573e7`) and the version (1, `0x55742e`). Any difference closes the file and
returns with the cache empty, and nothing is deleted from disk. The body is read entry by entry: eight bytes (key, size), and a key of 0 ends the file (`0x5574be`, `0x55761a`).
Otherwise the size bytes are read (a size above `0x4000` makes a larger buffer) and decoded by the type's reader into a node found or created for the key, the valid byte
`+0x50` is set and `+0x4c` takes the key. A short read of an entry, or of its record, sets an error byte, and the cache then clears every node it has loaded
(`0x557665` to `0x5576a4`).

## Caches that are not saved

Four caches are never read from or written to disk: names (`0xc0e228`, `namecache.wdb`, tag `MANW`, opcode `0x50`), guilds
(`0xc0e1ec`, `DLGW`, `0x54`), pet names (`0xc0e138`, `MNPW`, `0x52`) and petitions (`0xc0e0fc`, `NTPW`, `0x1c6`). Their
constructors get 0 for the persist byte, which is `+0x39`; the load and the save each return at once when it is 0. When the
game interface is initialized, every cache whose `+0x39` is 0 is cleared, so its records last until the interface is next initialized.

Addresses: `0x554cd0`, `0x55e6a0`, `0x55ec70`, `0x55fa50`, `0x555740`, `0x560400`, `0x48f480`, `0x48f586`
Confidence: verified
Proof: one
Test: none

`0x55e6a0` stores the persist byte at `+0x39` (`0x55e770`) and the owner flag at `+0x38`; `0x55ec7d` tests `[esi+0x39]` and
skips to the object teardown when it is 0. `0x555740` reads each cache's `+0x39` (`0xc0e38d` for the creature cache and so on) and calls its
clear when it is 0 (`0x55574e`, `0x5557ad` for names): it has one caller, `0x48f586` in `CGGameUI::InitializeGame`. `0x554f50`
calls the load function of each of the thirteen caches; the four that are not saved return at once (`0x55fa74` for names).

## Cache objects and nodes

A cache object holds: the vtable at `+0`; a list of all nodes at `+8`; a chain counter at `+0x10`; a bucket array at `+0x1c`
of `mask + 1` 12-byte heads; the mask at `+0x24` (−1 until the first node); the tag at `+0x28`; the file name at `+0x2c`; the query opcode at
`+0x30`; a dword at `+0x34`; the owner flag at `+0x38`; the persist flag at `+0x39`. A node holds its key at `+0`, the
hash chain link at `+4`, the list link at `+0xc`, the record from `+0x18`, a copy of the key at `+0x4c`, a valid byte at `+0x50`, a callback list at `+0x5c` and a
dispatching byte at `+0x61`. The hash is `key & mask`.

Addresses: `0x55e6a0`, `0x80917c`, `0x55e74e`, `0x56d910`, `0x556ff0`
Confidence: verified
Proof: one
Test: none

The constructor `0x55e6a0` zeroes `+0x14` to `+0x20`, stores the vtable (`0x80917c` for names), sets `+0x24` to −1 and stores its arguments
(`0x55e74e` to `0x55e770`). The dword goes to `+0x34` at `0x55e760`; the item cache's copy `0x55b0a0` stores its `0x57` at `0x55b160` (the constructor calls, above). Nothing in `0x554b00` to `0x5738c0` reads a node's age, and a hit changes nothing. `0x556ff0`, the creature cache's counterpart of the name removal `0x560170`, has no `e8`, `e9` or dword reference in the image.

## Hash growth

The first insert into a cache sets its mask to 3. Each later insert counts the length of the chain it lands in; a chain past 13 nodes makes the next
insert double the buckets to `2·mask + 2`, and growth stops once the mask reaches `0x1fff`. Nothing is evicted by size or age.

Addresses: `0x56d910`, `0x56d926`, `0x56d9de`, `0x56d9e4`, `0x56d99e`, `0x56d9ff`
Confidence: verified
Proof: one
Test: none

`0x56d926` stores 3 in `+0x24`; `0x56d9de` increments the counter `+0x10`; `0x56d9e4` compares it with `0xd` (`ja` to the rehash); `0x56d9ff`
computes `2·mask + 2` (`lea edx,[ecx+ecx+2]`); `0x56d99e` skips growth at `0x1fff`. The 13 clones of the allocator are `0x56d190` to `0x56e390`.

## Handlers

`0x554ff0` registers fourteen handlers with `0x5ab650`: opcode `0x61` creatures, `0x5f` game objects, `0x2c5` item names, `0x58` and
`0x59` items, `0x180` NPC text, `0x51` names, `0x55` guilds, `0x5d` quests, `0x5b` page text, `0x53` pet names, `0x1c7` petitions,
`0x244` item text and `0x31c` the removal of a name. The handler calls the cache's `Receive(stream, single)`: single is 1 except for `0x59`, which reads
a count byte first and then that many records. `0x31c` reads a guid and removes that name from the names cache.

Addresses: `0x554ff0`, `0x5550e0`, `0x555100`, `0x555120`, `0x555140`, `0x555160`, `0x555180`, `0x5551a0`, `0x555290`, `0x555300`, `0x555460`, `0x555480`, `0x555500`, `0x5555e0`, `0x555600`, `0x5ab650`
Confidence: verified
Proof: one
Test: none

Each `push 0; mov edx,<handler>; mov ecx,<opcode>; call 0x5ab650` (`0x554ff0` to `0x5550d9`), the call from `0x402b16`. The receive calls are `0x556e20`,
`0x5588e0`, `0x55a310`, `0x55bdb0` (flag 1, then 0 for `0x59`), `0x55d8b0`, `0x561070`, `0x562c80`, `0x564ab0`, `0x566470`, `0x567fa0` and `0x569d00`; the handler
for `0x51` decodes its record in place and calls `0x55f310` at `0x555230`; for `0x1c7` it calls `0x7c7a30` and then `0x567fa0`. `0x555600` calls `0x560170` with the guid.

## Name and guild responses

The names handler `0x5551a0` reads a guid, a name of up to `0x30` bytes, a server name of up to `0x100` bytes and three dwords. A non-empty name is stored
with `0x55f310`; if the high dword of the guid has its top four bits equal to 9, `0x560370` runs too. An empty name is a refusal: `0x55f6f0` runs the guid's callbacks with
status 0 and frees the node. The guild handler `0x555290` decodes the record on the stack; a non-empty name goes to `0x561070`, an empty one to `0x561390`, which runs the callbacks with status 0 and frees the node.

Addresses: `0x5551a0`, `0x55f310`, `0x560370`, `0x55f6f0`, `0x555290`, `0x561070`, `0x561390`, `0x55f781`
Confidence: verified
Proof: one
Test: none

`0x555217` tests the first name byte; `0x55521a` (`je 0x555266`) takes the refusal to `0x55f6f0` (`0x555273`); `0x555245` compares `high & 0xf0000000` with `0x90000000`. `0x55f781` sets the
node's dispatching byte `+0x181` before the callbacks. The guild handler tests the record's name byte at `0x5552a8` and calls `0x561070` (`0x5552c5`) or `0x561390` (`0x5552e1`).

## Receiving a record

`Receive` reads, for each record, the key as a dword. A key with its top bit set is a refusal: the bit is cleared, the node's callbacks
run with status 0, and the node is freed. Otherwise the node is found or created, `T::Read` decodes the record into it, the valid byte `+0x50`
is set, the callbacks run with status 1 and are freed, and the dispatching byte `+0x61` is set during the run and cleared after.

Addresses: `0x556e20`, `0x556e5b`, `0x556f59`, `0x556f97`, `0x556f0e`, `0x556fa0`
Confidence: verified
Proof: one
Test: none

`0x556e5b` reads the key (`0x418e30`); `0x556e63` to `0x556e71` clear the top bit and keep it in `[ebp-1]`. The refusal leg `0x556ec0` to `0x556f37` sets
`+0x61` (`0x556ed4`), calls each callback with `push 0` (`0x556f05`) and frees the node through vtable slot 0 (`0x556f35`). The accept leg creates the node with
`0x56d190` (`0x556f49`), calls the decoder at `0x556f59`, stores `+0x50` and `+0x4c`, and calls each callback with `push 1` (`0x556f91`); the callbacks are
freed at `0x556fa0` to `0x556fc4`. A callback is `call [record+8]` with the key in `ecx`, a pointer to its owner id in `edx`, the status and its data.

## Asking for a record

A lookup takes a key, a pointer to the caller's 64-bit id, a callback, its data and a flag. A zero key returns null. A node that is valid
returns its record, with nothing touched. Otherwise, with no callback it returns null and sends nothing. With one, a node is made if absent, the callback is added unless the flag is set and an
equal one (same function and data) is there, and for a new node the query is sent: the cache's opcode, the key and, when the owner flag `+0x38` is set, the caller's id. A
pending node leaves the query to the first caller. Eleven lookups are copies of one body; the table gives their node offsets.

| lookup | cache | owner flag | key copy | valid byte | callback list link |
|---|---|---|---|---|---|
| creature `0x556aa0` | `0xc0e354` | 1 | `+0x4c` | `+0x50` | `+0x5c` |
| game object `0x558560` | `0xc0e318` | 1 | `+0x94` | `+0x98` | `+0xa4` |
| item name `0x559fe0` | `0xc0e2dc` | 1 | `+0x1c` | `+0x20` | `+0x2c` |
| item `0x55ba30` | `0xc0e2a0` | 1 | `+0x1ec` | `+0x1f0` | `+0x1fc` |
| NPC text `0x55d530` | `0xc0e264` | 1 | `+0x158` | `+0x15c` | `+0x168` |
| guild `0x560e30` | `0xc0e1ec` | 0 | `+0x310` | `+0x314` | `+0x320` |
| quest `0x562a40` | `0xc0e1b0` | 0 | `+0x18f4` | `+0x18f8` | `+0x1904` |
| page text `0x564730` | `0xc0e174` | 1 | `+0x41c` | `+0x420` | `+0x42c` |
| pet name `0x566240` | `0xc0e138` | 1 | `+0x70` | `+0x74` | `+0x80` |
| petition `0x567d60` | `0xc0e0fc` | 1 | `+0x13e0` | `+0x13e4` | `+0x13f0` |
| item text `0x569980` | `0xc0e0c0` | 1 | `+0x1f58` | `+0x1f5c` | `+0x1f68` |

Addresses: `0x558560`, `0x556aa0`, `0x559fe0`, `0x55ba30`, `0x55d530`, `0x560e30`, `0x562a40`, `0x564730`, `0x566240`, `0x567d60`, `0x569980`, `0x55f080`, `0x5ab630`, `0x558160`, `0x557d80`, `0x558572`, `0x558582`, `0x5585b8`, `0x5585cf`, `0x5585dc`, `0x5585ea`, `0x5586ca`, `0x5586e3`, `0x558708`, `0x558710`, `0x55876c`, `0x557dfa`, `0x558648`, `0x558698`
Confidence: verified
Proof: one
Test: none

`0x558560` (`ret 0x14`: key, owner pointer, callback, data, flag) is the model. The other ten key-dword lookups run the same instructions with their own offsets and helper addresses (compared with those masked; the creature, item name and pet name lookups differ also in short displacement encodings, and the pet name one in a loop-alignment filler). The zero key returns at `0x558572`; an empty cache (mask −1) goes to the miss (`0x558582`); the chain walk calls the next-link accessor `0x558160` (`0x5585cf`) and compares the key at `node+0` (`0x5585b8`). A hit with the valid byte `[node+0x98]` set (`0x5586ca`) returns `node + 0x18` (`0x55876c`). The pending hit scans the callback list, an object at `+0x9c` (`0x558623`) whose first link is `+0xa4`, for the same function (`0x558708`) and data (`0x558710`) when the flag is set (`0x5586e3`). The miss without a callback returns at `0x5585dc`. The new node (`0x5585ea`) comes from the cache's vtable slot 1, `0x557d80`, which clears the valid byte (`0x557dfa`). The query is `0x558648` to `0x558698`: `PutUInt32` of `[cache+0x30]` and of the key, `PutGuid` of the owner id when `[cache+0x38]` is set, then `ClientServices::Send`. The names cache `0x55f080` takes a 64-bit key; its record is at `node + 0x20`, valid byte `+0x170`.

## Record decoders

Each type has a `Read(this = record, stream)` that fills a record in a fixed order, the key having been read by `Receive`. The saved record sizes are creature `0x34`,
game object `0x7c`, item name 4, item `0x1d4`, NPC text `0x140`, guild `0x2f8`, quest `0x18dc`, page text `0x404`, pet name `0x58`, petition `0x13c8`, name `0x148`
and item text `0x1f40`. A string is either read into the record, or duplicated with `SStrDupA` and stored as a pointer; an empty one after the first in a run of names is
duplicated from the previous slot.

Addresses: `0x7c8120`, `0x7c87c0`, `0x7c8dc0`, `0x7c9640`, `0x7c9fa0`, `0x62f260`, `0x6dbf60`, `0x7cabd0`, `0x6dc290`, `0x7c7a30`, `0x6dc1f0`, `0x569d00`, `0x64a620`
Confidence: verified
Proof: one
Test: `dbcache::dbcache_creature`, `dbcache::dbcache_gameobject`, `dbcache::dbcache_itemstats`, `dbcache::dbcache_npctext`, `dbcache::dbcache_item_name`, `dbcache::dbcache_pet_name`, `dbcache::dbcache_page_text`, `dbcache::dbcache_name`, `dbcache::dbcache_guild`, `dbcache::dbcache_quest`, `dbcache::dbcache_petition`, `dbcache::dbcache_item_text`

The tests run each decoder whole against the exe's own code: `0x7c8120` creature, `0x7c87c0` game object, `0x7c8dc0` item name, `0x7c9640` item, `0x7c9fa0` NPC text, `0x62f260` guild,
`0x6dbf60` quest, `0x7cabd0` page text, `0x6dc290` pet name, `0x7c7a30` petition, `0x6dc1f0` name, and the item text inside `0x569d00`. The sizes are the pushes of the saving
functions (`0x5567e8` and the like).

## Creature and game object

Creature (`0x7c8120`): four name pointers at `+0`, `+4`, `+8`, `+0xc`; the subname pointer at `+0x10`, always duplicated; seven dwords at `+0x14` to `+0x2c`; two bytes stored as
0 or 1 at `+0x30` and `+0x31`. Game object (`0x7c87c0`): dwords at `+0` and `+4`; five string pointers at `+8` to `+0x18`; 24 dwords at `+0x1c` to `+0x78`.

Addresses: `0x7c8120`, `0x7c87c0`
Confidence: verified
Proof: one
Test: `dbcache::dbcache_creature`, `dbcache::dbcache_gameobject`

Read orders as the decoders make them, in the stream's order. The first four names are read in a loop; the fifth string is read apart.

## Item and NPC text

Item (`0x7c9640`, `0x1d4` bytes): dwords at `+0`, `+4`; four string pointers `+8` to `+0x14`; 20 dwords `+0x18` to `+0x64`; two arrays of ten at `+0x68` and `+0x90`, read as
pairs; five groups of a float at `+0xb8…`, a float at `+0xcc…` and a dword at `+0xe0…`; seven dwords from `+0xf4`; the rest to `+0x1d0`, with a description pointer at `+0x198`, 0 when empty.
NPC text (`0x7c9fa0`, `0x140`): eight times: a float at `+i·4`, two string pointers at `+0x20 + i·8` and `+0x24 + i·8`, a dword at `+0x60 + i·4`, then three emote pairs at `+0x80` and `+0xe0` strides.

Addresses: `0x7c9640`, `0x7c9fa0`
Confidence: verified
Proof: one
Test: `dbcache::dbcache_itemstats`, `dbcache::dbcache_npctext`

The item description and each NPC text string are duplicated only when non-empty; otherwise the pointer is 0.

## Guild and quest

Guild (`0x62f260`, `0x2f8`): a dword at `+0`; the name, 96 bytes at `+4`; ten rank names of 64 bytes from `+0x64`; five dwords at `+0x2e4` to `+0x2f4`. Quest (`0x6dbf60`, `0x18dc`): 15 dwords `+0`
to `+0x38`; four pairs, six pairs and a run of dwords with two floats at `+0x90`, `+0x94`; Title at `+0x9c`, Objectives `+0x29c`, Details `+0xa9c`, EndText `+0x129c`; four groups of four objective dwords
interleaved at `+0x14ac…`; four objective texts of 256 bytes from `+0x14dc`.

Addresses: `0x62f260`, `0x6dbf60`
Confidence: verified
Proof: one
Test: `dbcache::dbcache_guild`, `dbcache::dbcache_quest`

Strings are read without a bound (`0x7fffffff`), so a field's size is the layout.

## Quest record dwords

`+0x24` is the id of the quest that follows; `+0x34` is the item the quest hands out; `+0x8c` is a column nothing reads.

Addresses: `0x5dc4d7`, `0x5dc6d7`, `0x5dc751`, `0x4e00ce`, `0x4e04fa`, `0x6dc05f`, `0x562286`, `0x562da9`, `0x6dbe5c`
Confidence: inferred
Proof: none
Test: none

`+0x24` is only tested for zero: `0x5dc4d7` and `0x5dc751`, in the quest-complete readers `0x5dc400` and `0x5dc710`, print message `0x8a` with the title (`+0x9c`) when it is 0, and `0x5dc6d7` turns it into 0 or 1. Where the install's `questcache.wdb` has it set, the value is another quest's id. `+0x34` is compared with each required item id (`0x4e00ce` in `GetNumQuestLeaderBoards`, `0x4e04fa` in `GetQuestLogLeaderBoard`): a required item equal to it is not counted as an objective line. `+0x8c` is read by the decoder (`0x6dc05f`), zeroed by the record clear (`0x562286`), moved by the node copy (`0x562da9`) and put by the file writer (`0x6dbe5c`); no instruction in the 42 functions that call `0x562a40`, nor in the code around them (`0x4de510` to `0x4e1600`, `0x4e8c80` to `0x4e9bd0`, `0x5dc400` to `0x5ddf30`, `0x5e5ad0` to `0x5e5ea0`), reads it. The control, `+0x24`, shows three reads in the same ranges.

## Pages, pets and item texts

Page text (`0x7cabd0`, `0x404`): a string of up to `0x400` bytes at `+0`, a dword at `+0x400`. Pet name (`0x6dc290`, `0x58`): a dword at `+0x50`, a string of up to `0x50` bytes at `+0`, a dword at `+0x54`. Item name
(`0x7c8dc0`): one string, duplicated, at `+0`. Item text: a key with its top bit set refuses; otherwise a string of up to 8,000 bytes at `+0`, and the key stored at `+0x1f40` with a valid byte at `+0x1f44`.

Addresses: `0x7cabd0`, `0x6dc290`, `0x7c8dc0`, `0x569d00`
Confidence: verified
Proof: one
Test: `dbcache::dbcache_page_text`, `dbcache::dbcache_pet_name`, `dbcache::dbcache_item_name`, `dbcache::dbcache_item_text`

The item text is decoded inside the receive function `0x569d00`.

## Names and petitions

Name (`0x6dc1f0`): a guid at `+0x130`, a name of up to `0x30` bytes at `+0`, a server name of up to `0x100` at `+0x30`, then dwords at `+0x138`, `+0x13c`, `+0x140`. Petition (`0x7c7a30`): a dword at `+0`; a guid at `+8`;
strings at `+0x10` and `+0x110`; eight dwords from `+0x1110`; a 16-bit value at `+0x1130`; dwords at `+0x1134`, `+0x1138`; a count at `+0x13bc` and that many 64-byte strings from `+0x113c`; a dword at `+0x13c0`.

Addresses: `0x6dc1f0`, `0x7c7a30`
Confidence: verified
Proof: one
Test: `dbcache::dbcache_name`, `dbcache::dbcache_petition`

The petition's count is a signed dword; the loop runs zero times for 0 or less. The three trailing name dwords are in the wire's order, which is not the order of the unit update fields.

## Name cache node

A name node is `0x188` bytes plus an extra, allocated by `0x55e810` with the type name `0x855ec8`. Its record starts at `+0x20` and is `0x148` bytes. The valid byte is `+0x170`, the callback list
`+0x178`, a skip-from-save flag `+0x180` and the dispatching byte `+0x181`. In the record, `+0x13c` is the sex.

Addresses: `0x55e810`, `0x855ec8`, `0x55fb9f`, `0x55fbeb`, `0x560370`, `0x55e894`, `0x5603e5`, `0x55ee0b`, `0x560033`, `0x517f80`, `0x55ec70`, `0x55fa50`, `0x55fee0`
Confidence: verified
Proof: one
Test: none

The allocator calls `SMemAlloc` with `0x188` plus the extra and the type name `.?AUDBCACHEHASH@?$DBCache@VNameCache@@_KVCHashKeyGUID@@@@` (`0x55e820` to `0x55e82e`). It clears the
record with `rep stos` of `0x52` dwords (`0x55e884`); `+0x180` and `+0x181` start 0 (`0x55e894`, `0x55e89a`). In the record the name is at `+0`, the server name at `+0x30`, the guid at `+0x130` and the three
dwords at `+0x138`, `+0x13c` and `+0x140`: race, sex and class, in the order the handler reads them (`0x5551f6`, `0x5551ff`, `0x55520c`). `UnitSex` (`0x517e90`) reads `+0x13c` at `0x517f80` and indexes
the float table `0x808be4` with it. Both save routines (`0x55ec70`, the exit function, and the unreached `0x55fee0`) test the valid byte and then `+0x180` (`0x55ee0b`, `0x560033`) and write the node only
when `+0x180` is 0. `0x560370` sets it (`0x5603e5`) on the node whose guid matches, and the names handler calls it when the guid's top four bits are 9 (`0x555245`). The names cache has persist 0, so neither save runs.
The load `0x55fa50` returns at once for the same reason; past that test it compares the file header's record size with `0x148` (`0x55fb9f`) and the version with 4 (`0x55fbeb`), and closes the file on a difference.

## Name cache writers

The names cache has two row writers. The lookup `0x55f080` inserts an empty, pending node on a miss when it has a callback; `0x55f310`, called only from
the handler for opcode `0x51` (`0x555230`), writes the payload and sets the valid byte `+0x170`. The payload of a node reaches a caller only through
the lookup's return (`add eax,0x20` occurs once in `0x554b00` to `0x5738c0`, at `0x55f2e5`); the bulk reader `0x55f4a0` has no caller. A removal, `0x555600` to `0x560170`,
frees a name.

Addresses: `0x55f080`, `0x55f310`, `0x555230`, `0x55f2e5`, `0x55f4a0`, `0x560170`, `0x55f413`, `0x555600`
Confidence: verified
Proof: one
Test: none

`0x55f125` (`je 0x55f221`) returns null without sending when the callback is null; `0x55f132` inserts the node with `0x56d910`; `0x55f413` stores the valid byte.
Census: one `add eax,0x20` in the range; `0x55f310` has one `e8` reference and `0x55f4a0` none (control: `0x55f080` has several).
