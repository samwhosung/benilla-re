# dbcache: overview

The client's caches of records the server streams: creatures, game objects, items, quests, names, guilds, pages,
pets and petitions. `DBCache.cpp`, `0x554b10` through `0x5737b0`, holds thirteen cache objects of one template,
`DBCache<T>`; the record files `CreatureStats.cpp`, `GameObjectStats.cpp`, `ItemName.cpp`, `ItemStats.cpp` and `NPCText.cpp` (`0x7c8120`
on) hold five of the twelve record decoders, which `dbcache/records` has. A caller asks a cache for a record by id; on a miss the cache sends a
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

The constructor calls are `0x554b40` and the eight like it: six pushes (persist flag, owner flag, a dword, opcode, file name,
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

Addresses: `0x5566d0`, `0x5576d0`, `0x42a460`, `0x556733`, `0x556747`, `0x4420b0`, `0x556821`, `0x5568e0`, `0x5567cc`, `0x5567d9`, `0x5567e8`, `0x5567f2`, `0x5568f9`, `0x7c8080`, `0x5568ac`, `0x417e10`, `0x6ca120`, `0x56b162`
Confidence: verified
Proof: two
Test: none

The save writes the header from `0x5567a9` on with `CDataStore::PutUInt32`: the tag (`0x5567c7`), the build push `0x16f3` (`0x5567cc`), the locale
(`0x5567d9`), the record size push (`0x5567e8`, `0x34` for creatures) and the version push (`0x5567f2`, 1 for creatures). The wow cache's version
push is the call `0x6ca120` (`0x56b162`; its load compares the file's with the call at `0x56bff3`), which is `mov eax,1; ret`, so its version is 1. A failed open for writing (`0x42a460` at `0x55672c`, tested at `0x556733`) skips the writing and prints nothing: the log call `0x556747` goes to `0x4420b0`, a bare `ret`.
Per node, walking the all-nodes list from its tail, `[cache+0xc]`, along the previous links (`0x556821`, `0x5568e0`), an entry is written only when the valid byte `+0x50` is set and the byte `+0x60` is clear (`0x556861` to `0x55686b`): the key
copy `+0x4c`, a zero size, the record encoded by the type's writer (`0x7c8080` for creatures), and the size patched in place by `0x417e10(position 4,
length − 8)` at `0x5568ac`. `0x7c8080` puts the four names and the subname as strings, seven dwords, and two flag bytes as 0 or 1. The two zero dwords are written at `0x5568f9` to `0x55691d`, then the file is closed.

## Loading a .wdb file

The load returns at once for a cache whose persist byte is 0. Otherwise it reads the 20-byte header and compares the tag, the build, the locale, the record size and the
version with its own. Any difference closes the file and leaves the cache empty; the file stays on disk. The body is read entry by entry until a key of 0, each record decoded by the
type's reader into a node found or created for the key, with the valid byte `+0x50` set. A short read clears the nodes already loaded.

Addresses: `0x5572b0`, `0x557342`, `0x557390`, `0x5573ae`, `0x5573c6`, `0x5573e7`, `0x55742e`, `0x5574be`, `0x55761a`, `0x557665`, `0x56bff3`
Confidence: verified
Proof: two
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
constructors get 0 for the persist byte, which is `+0x39`; the load returns at once when it is 0, and the save writes nothing and tears the cache down. When the
game interface is initialized, every cache whose `+0x39` is 0 is cleared, so its records last until the interface is next initialized.

Addresses: `0x554cd0`, `0x554cd2`, `0x554d22`, `0x554e12`, `0x554e62`, `0x55ec83`, `0x55ecef`, `0x55e6a0`, `0x55ec70`, `0x55fa50`, `0x555740`, `0x560400`, `0x48f480`, `0x48f586`
Confidence: verified
Proof: two
Test: none

`0x55e6a0` stores the persist byte at `+0x39` (`0x55e770`) and the owner flag at `+0x38`; `0x55ec7d` tests `[esi+0x39]` and
skips to the teardown when it is 0 (`0x55ec83` to `0x55ecef`). The thunk bodies push 0 for the persist byte (`0x554cd2`, `0x554d22`, `0x554e12`, `0x554e62`), and nothing sets it later: in `0x554b00` to `0x5738c0` the only byte stores to `+0x39` are the thirteen constructors' (control), so the four saves never write. `0x555740` reads each cache's `+0x39` (`0xc0e38d` for the creature cache and so on) and calls its
clear when it is 0 (`0x55574e`, `0x5557ad` for names): it has one caller, `0x48f586` in `CGGameUI::InitializeGame`. `0x554f50`
calls the load function of each of the thirteen caches; the four that are not saved return at once (`0x55fa74` for names).

## Cache objects and nodes

A cache object holds: the vtable at `+0`; the all-nodes list, header `+4` (link offset 0xc, head `+8`, tail `+0xc`); a growth counter `+0x10`; a bucket array `+0x1c` of `mask + 1` 12-byte list headers; the mask `+0x24` (−1 until the first node); the tag `+0x28`; the file name `+0x2c`; the query opcode `+0x30`; a dword `+0x34`; the owner flag `+0x38`; the persist flag `+0x39`. A node holds its key at `+0`, the hash link `+4`, the all-nodes link `+0xc`, the record from `+0x18`, a key copy `+0x4c`, a valid byte `+0x50`, a callback list header `+0x54` and a dispatching byte `+0x61`. The hash is `key & mask`.

Addresses: `0x55e6a0`, `0x80917c`, `0x55e740`, `0x55e74e`, `0x56d910`, `0x556ff0`, `0x55625a`
Confidence: verified
Proof: two
Test: none

The constructor `0x55e6a0` builds the node list's header (the link offset 0xc at `0x55e740`), zeroes `+0x14` to `+0x20`, stores the vtable (`0x80917c` for names), sets `+0x24` to −1 and stores its arguments
(`0x55e74e` to `0x55e770`). The dword goes to `+0x34` at `0x55e760`; the item cache's copy `0x55b0a0` stores its `0x57` at `0x55b160` (the constructor calls, above). The creature node's callback list header is built at `0x55625a`. Nothing in `0x554b00` to `0x5738c0` reads a node's age, and a hit changes nothing. `0x556ff0`, the creature cache's counterpart of the name removal `0x560170`, has no `e8`, `e9` or dword reference in the image.

## Hash growth

The first insert into a cache sets its mask to 3. While the mask is below `0x1fff`, each insert lowers the counter at `+0x10` by 3 (to 0 when it was 3 or less), then adds 1 for each node it meets in its bucket; when the count passes 13, that insert resets it to 0, doubles the buckets to `2·mask + 2` and takes its new bucket. A chain under 14 nodes can still trip the counter. Nothing is evicted by size or age.

Addresses: `0x56d910`, `0x56d926`, `0x56d9a5`, `0x56d9b5`, `0x56d9de`, `0x56d9e4`, `0x56d9e7`, `0x56d99e`, `0x56d9ff`, `0x56da06`, `0x56da2e`
Confidence: verified
Proof: two
Test: none

`0x56d926` stores 3 in `+0x24`; `0x56d99e` skips growth at `0x1fff`; `0x56d9a5` to `0x56d9b5` lower the counter `+0x10` by 3 or zero it; `0x56d9de` to `0x56d9e7` increment it per node walked; `0x56d9e4` compares it with `0xd` (`ja` to the rehash); `0x56da06` zeroes it and `0x56d9ff`
computes `2·mask + 2` (`lea edx,[ecx+ecx+2]`), before the node is allocated (`0x56da2e`). The 13 clones of the insert, `0x56d190` to `0x56e390`, are the same instructions but for their addresses.

## Handlers

`0x554ff0` registers fourteen handlers with `0x5ab650`: opcode `0x61` creatures, `0x5f` game objects, `0x2c5` item names, `0x58` and
`0x59` items, `0x180` NPC text, `0x51` names, `0x55` guilds, `0x5d` quests, `0x5b` page text, `0x53` pet names, `0x1c7` petitions,
`0x244` item text and `0x31c` the removal of a name. The creature, game object, item name, item, NPC text, page text and item text handlers call the cache's `Receive(stream, single)`: single is 1 except for `0x59`, which reads
a count byte first and then that many records. The name, guild, quest, pet name and petition handlers decode the record themselves and call the cache's store with the record and key, or its refusal with the key. `0x31c` reads a guid and removes that name.

Addresses: `0x554ff0`, `0x5550e0`, `0x555100`, `0x555120`, `0x555140`, `0x555160`, `0x555180`, `0x5551a0`, `0x555290`, `0x555300`, `0x555460`, `0x555480`, `0x555500`, `0x5555e0`, `0x555600`, `0x5ab650`, `0x561390`, `0x563150`, `0x566750`, `0x5683a0`, `0x5552a3`, `0x55540b`, `0x555598`
Confidence: verified
Proof: two
Test: none

Each `push 0; mov edx,<handler>; mov ecx,<opcode>; call 0x5ab650` (`0x554ff0` to `0x5550d9`), the call from `0x402b16`. The receive calls are `0x556e20`,
`0x5588e0`, `0x55a310`, `0x55bdb0` (flag 1, then 0 for `0x59`), `0x55d8b0`, `0x564ab0` and `0x569d00`. The handler
for `0x51` decodes its record in place and calls `0x55f310` at `0x555230`. The guild handler decodes with `0x62f260` (`0x5552a3`) and tests the name byte (`0x5552a8`) for `0x561070` or `0x561390`; the quest handler decodes with `0x6dbf60` (`0x55540b`) and tests the title (`0x555410`) for `0x562c80` or `0x563150`; the pet name handler reads its fields itself (`0x5554a2` to `0x5554ba`) and tests a byte (`0x5554c2`) for `0x566470` or `0x566750`; the petition handler decodes with `0x7c7a30` (`0x555598`) and, for a positive first dword (`jle` `0x5555a6`), calls `0x567fa0` with it, else `0x5683a0` with its negation (`0x5555c5`). `0x555600` calls `0x560170` with the guid.

## Name and guild responses

The names handler `0x5551a0` reads a guid, a name of up to `0x30` bytes, a server name of up to `0x100` bytes and three dwords. A non-empty name is stored
with `0x55f310`; if the high dword of the guid has its top four bits equal to 9, `0x560370` runs too. An empty name is a refusal: `0x55f6f0` runs the guid's callbacks with
status 0 and frees the node. The guild handler `0x555290` decodes the record on the stack; a non-empty name goes to `0x561070`, an empty one to `0x561390`, which runs the callbacks with status 0 and frees the node.

Addresses: `0x5551a0`, `0x55f310`, `0x560370`, `0x55f6f0`, `0x555290`, `0x561070`, `0x561390`, `0x55f781`
Confidence: verified
Proof: two
Test: none

`0x555217` tests the first name byte; `0x55521a` (`je 0x555266`) takes the refusal to `0x55f6f0` (`0x555273`); `0x555245` compares `high & 0xf0000000` with `0x90000000`. `0x55f781` sets the
node's dispatching byte `+0x181` before the callbacks. The guild handler tests the record's name byte at `0x5552a8` and calls `0x561070` (`0x5552c5`) or `0x561390` (`0x5552e1`).

## Receiving a record

`Receive` reads, for each record, the key as a dword. A key with its top bit set is a refusal: the bit is cleared, the node's callbacks
run with status 0, and the node is freed. Otherwise the node is found or created, `T::Read` decodes the record into it, the valid byte `+0x50`
is set, the callbacks run with status 1 and are freed, and the dispatching byte `+0x61` is set during the run and cleared after.

Addresses: `0x556e20`, `0x556e5b`, `0x556f59`, `0x556f97`, `0x556f0e`, `0x556fa0`
Confidence: verified
Proof: two
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

| lookup | cache | owner flag | key copy | valid byte | callback list |
|---|---|---|---|---|---|
| creature `0x556aa0` | `0xc0e354` | 1 | `+0x4c` | `+0x50` | `+0x54` |
| game object `0x558560` | `0xc0e318` | 1 | `+0x94` | `+0x98` | `+0x9c` |
| item name `0x559fe0` | `0xc0e2dc` | 1 | `+0x1c` | `+0x20` | `+0x24` |
| item `0x55ba30` | `0xc0e2a0` | 1 | `+0x1ec` | `+0x1f0` | `+0x1f4` |
| NPC text `0x55d530` | `0xc0e264` | 1 | `+0x158` | `+0x15c` | `+0x160` |
| guild `0x560e30` | `0xc0e1ec` | 0 | `+0x310` | `+0x314` | `+0x318` |
| quest `0x562a40` | `0xc0e1b0` | 0 | `+0x18f4` | `+0x18f8` | `+0x18fc` |
| page text `0x564730` | `0xc0e174` | 1 | `+0x41c` | `+0x420` | `+0x424` |
| pet name `0x566240` | `0xc0e138` | 1 | `+0x70` | `+0x74` | `+0x78` |
| petition `0x567d60` | `0xc0e0fc` | 1 | `+0x13e0` | `+0x13e4` | `+0x13e8` |
| item text `0x569980` | `0xc0e0c0` | 1 | `+0x1f58` | `+0x1f5c` | `+0x1f60` |

Addresses: `0x558560`, `0x556aa0`, `0x559fe0`, `0x55ba30`, `0x55d530`, `0x560e30`, `0x562a40`, `0x564730`, `0x566240`, `0x567d60`, `0x569980`, `0x55f080`, `0x5ab630`, `0x558160`, `0x557d80`, `0x558572`, `0x558582`, `0x5585b8`, `0x5585cf`, `0x5585dc`, `0x5585ea`, `0x5586ca`, `0x5586e3`, `0x558708`, `0x558710`, `0x55876c`, `0x557dfa`, `0x558648`, `0x558698`
Confidence: verified
Proof: two
Test: none

`0x558560` (`ret 0x14`: key, owner pointer, callback, data, flag) is the model. The other ten key-dword lookups run the same instructions with their own offsets and helper addresses (compared with those masked; the creature, item name and pet name lookups differ also in short displacement encodings, and the pet name one in a loop-alignment filler). The zero key returns at `0x558572`; an empty cache (mask −1) goes to the miss (`0x558582`); the chain walk calls the next-link accessor `0x558160` (`0x5585cf`) and compares the key at `node+0` (`0x5585b8`). A hit with the valid byte `[node+0x98]` set (`0x5586ca`) returns `node + 0x18` (`0x55876c`). The pending hit scans the callback list, whose header is at `+0x9c` (`0x558623`), from its tail `+0xa4`, for the same function (`0x558708`) and data (`0x558710`) when the flag is set (`0x5586e3`). The miss without a callback returns at `0x5585dc`. The new node (`0x5585ea`) comes from the cache's vtable slot 1, `0x557d80`, which clears the valid byte (`0x557dfa`). The query is `0x558648` to `0x558698`: `PutUInt32` of `[cache+0x30]` and of the key, `PutGuid` of the owner id when `[cache+0x38]` is set, then `ClientServices::Send`. The names cache `0x55f080` takes a 64-bit key; its record is at `node + 0x20`, valid byte `+0x170`.

## Name cache node

A name node is `0x188` bytes plus an extra, allocated by `0x55e810` with the type name `0x855ec8`. Its record starts at `+0x20` and is `0x148` bytes. The valid byte is `+0x170`, the callback list
header `+0x174` (head `+0x178`, tail `+0x17c`), a skip-from-save flag `+0x180` and the dispatching byte `+0x181`. In the record, `+0x13c` is the sex.

Addresses: `0x55e810`, `0x855ec8`, `0x55e86b`, `0x55e87c`, `0x55f17a`, `0x55f788`, `0x517f86`, `0x808be4`, `0x554d05`, `0x55fb9f`, `0x55fbeb`, `0x560370`, `0x55e894`, `0x5603e5`, `0x55ee0b`, `0x560033`, `0x517f80`, `0x55ec70`, `0x55fa50`, `0x55fee0`
Confidence: verified
Proof: two
Test: none

`SMemAlloc` gets `0x188` plus the extra and the type name `.?AUDBCACHEHASH@?$DBCache@VNameCache@@_KVCHashKeyGUID@@@@` (`0x55e820` to `0x55e82e`). It stores the callback list's link offset at `+0x174` (`0x55e87c`), its head `+0x178` and tail `+0x17c` (`0x55e86b` to `0x55e876`), clears the record with `rep stos` of `0x52` dwords (`0x55e884`) and zeroes `+0x180` and `+0x181` (`0x55e894`, `0x55e89a`). The lookup links callbacks into `+0x174` (`0x55f17a`); the response walks them from `+0x17c` (`0x55f788`). In the record: name `+0`, server name `+0x30`, guid `+0x130` and the dwords `+0x138`, `+0x13c`, `+0x140`: race, sex and class, in the handler's read order (`0x5551f6`, `0x5551ff`, `0x55520c`). `UnitSex` (`0x517e90`) reads `+0x13c` at `0x517f80` and indexes the integer table `0x808be4` (2, 3, 1, 6) with `fild dword` (`0x517f86`). Both saves (`0x55ec70`, the exit function, and the unreached `0x55fee0`) test the valid byte and then `+0x180` (`0x55ee0b`, `0x560033`) and write only when it is 0; `0x560370` sets it (`0x5603e5`), and the names handler calls it when the guid's top four bits are 9 (`0x555245`). With persist 0 the exit save writes nothing and tears the cache down (`0x554d05`, `0x55ec83` to `0x55ecef`). The load `0x55fa50` returns at once; past that it compares the header's record size with `0x148` (`0x55fb9f`) and the version with 4 (`0x55fbeb`).

## Name cache writers

The names cache has two row writers. The lookup `0x55f080` inserts an empty, pending node on a miss when it has a callback; `0x55f310`, called only from
the handler for opcode `0x51` (`0x555230`), writes the payload and sets the valid byte `+0x170`. The payload of a node reaches a caller only through
the lookup's return (`add eax,0x20` occurs once in `0x554b00` to `0x5738c0`, at `0x55f2e5`); the bulk reader `0x55f4a0` has no caller. A removal, `0x555600` to `0x560170`,
frees a name.

Addresses: `0x55f080`, `0x55f310`, `0x555230`, `0x55f2e5`, `0x55f4a0`, `0x560170`, `0x55f413`, `0x555600`
Confidence: verified
Proof: two
Test: none

`0x55f125` (`je 0x55f221`) returns null without sending when the callback is null; `0x55f132` inserts the node with `0x56d910`; `0x55f413` stores the valid byte.
Census: one `add eax,0x20` in the range; `0x55f310` has one `e8` reference and `0x55f4a0` none (control: `0x55f080` has several).
