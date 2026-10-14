# objectlayer: manager

The object manager: where every object the server has described lives, how it is found by GUID,
and how it is released, reactivated, reused and destroyed.

The manager is one 0xe0-byte block at `[0xb41414]`, built when the client enters the world. It
holds two GUID hash tables, the live objects and the released ones; a list of every live object;
seven reuse lists, one per type 1 to 7; a list of objects reactivated during the current update
packet; the active player's GUID; the map id; and the client connection the update handlers are
registered on (`update`). Objects come from per-type pools of the ObjectAlloc code. Released
objects are kept, not freed: a later create, values, movement or near-objects block for the same
GUID brings the object back, and only the destroy message or the pool's need for memory frees it.

Roots: `0x464ff0`, `0x464870`, `0x464890`, `0x468460`, `0x468550`, `0x468570`, `0x468580`, `0x4685e0`, `0x468600`, `0x464fb0`, `0x464fd0`, `0x467700`, `0x468490`, `0x468380`, `0x4683e0`, `0x468410`, `0x467d40`, `0x467d10`

## Manager block

`0x464ff0(map)` allocates the 0xe0-byte manager and lays it out: the live table at `+0x00`
(buckets `+0x1c`, mask `+0x24`), the released table at `+0x28` (buckets `+0x44`, mask `+0x4c`),
both masks −1 until first use; seven reuse lists of 12 bytes from `+0x50`; the object list at
`+0xa4` with link offset 0x38; the reactivated list at `+0xb0`; and zeros from `+0xc0` to
`+0xd8`. It stores the block at `0xb41414` and at `+0x1ad8` of the client `[0xc28114]`, the
client at `+0xd0`, registers the update handlers, and stores the map argument at `+0xcc`.

Addresses: `0x464ff0`, `0x465025`, `0x46504e`, `0x465061`, `0x465088`, `0x4650c2`, `0x4650cd`, `0x4650db`, `0x46510e`, `0x465114`, `0x465120`, `0x465125`, `0x46512c`, `0x4685a0`, `0x4685b0`, `0xb41414`
Confidence: verified
Proof: one
Test: none

`0x465025` calls `SMemAlloc(0xe0)`; `0x46504e` and `0x465079` store the table vtable `0x803600`,
`0x465061` and `0x465088` the masks. The loop at `0x4650a0` builds seven lists 12 bytes apart
from `+0x50`; `0x4650c2` stores link offset 0x38 at `+0xa4`; `0x4650cd` builds `+0xb0`.
`0x46510e` and `0x465114` store the block; `0x465120` calls `0x4685b0` (store `ecx` at `+0xd0`),
`0x465125` calls `0x465140`, `0x46512c` calls `0x4685a0` (store `ecx`, the argument, at `+0xcc`).

## Finding an object

An object is keyed by its GUID in the live table: the bucket is the GUID's low dword ANDed with
the table's mask, and a bucket's chain holds objects whose `+0x30`/`+0x34` equal the GUID. A zero
GUID finds nothing. The typed lookup also requires the object's type mask, field 2, to share a
bit with the caller's mask. Neither looks in the released table.

Addresses: `0x464870`, `0x464890`, `0x4648ac`, `0x4648bc`, `0x4648da`, `0x4648ee`, `0x468460`, `0x46847a`
Confidence: verified
Proof: one
Test: none

`0x464870` returns 0 when `lo | hi` is 0, else calls `0x464890`. `0x464890` returns 0 when the
mask at `+0x24` is −1 (`0x4648ac`); else the bucket is `[+0x1c] + 12·(lo & mask)` (`0x4648bc`) and
the walk compares `+0x18` and `+0x30` with `lo` and `+0x34` with `hi` (`0x4648da`), following the
chain by the bucket's link offset (`0x4648ee`). `0x468460(lo, hi, line)`, with the mask in `ecx`,
calls `0x464870` and clears the result when `test [[eax+8]+8],esi` is zero (`0x46847a`).

## Releasing an object

Releasing takes a live object out of the world without freeing it. An object with a nonzero hold
count (`+0xe8`) only gets its release-pending bit, `0x10` of `+0xe4`, and stays live. Otherwise
the bit is cleared, its per-GUID record is forgotten, its vtable slot 1 runs, its watchers are
freed, and it moves from the live table and the object list into the released table and onto the
reuse list of its type. The out-of-range list and the destroy message release this way (`update`).

Addresses: `0x464920`, `0x464931`, `0x464942`, `0x46494f`, `0x46495f`, `0x464966`, `0x46496f`, `0x464974`, `0x464a37`, `0x464a83`, `0x614f00`, `0x614f50`, `0x464ad0`, `0x464c40`, `0x464c59`, `0x464c8e`, `0x464d26`, `0x464e5e`
Confidence: verified
Proof: one
Test: none

`0x464931` looks the GUID up in the live table only; none returns. `0x464942` calls `0x614f00`,
nonzero when `+0xe8` is; then `0x46494f` calls `0x614f50(1)`, which ORs `0x10` into `+0xe4`, and
returns. Else `0x46495f` calls `0x614f50(0)`, `0x464966` `0x464ad0(guid)`, `0x46496f`
`call [eax+0x4]`, `0x464974` `0x464c40`, which frees the object's `CMirrorHandler` records. The
object is unlinked at `+0x1c`, `+0x24` and from the list at `+0xa4`, inserted into the released
table (`0x464a37`, `0x468940`) and linked onto the list at the manager's `+0x44 + 12·type`
(`0x464a83`), the reuse list of type id `+0x14`. `0x464c40` switches on the type id through `0x464e98` and walks the
object's own 12-byte watcher lists, one per field of its sections: 42 from `+0x124` for an item,
74 from `+0x358` then the item's for a container, 182 from `+0x124` for a unit, 298 from `+0xe78`
then, for the active player, 796 from `[+0x1c68]+8`, then the unit's, for a player, 20, 10 and 32
from `+0x120` for a game object, dynamic object and corpse, and last 6 from `+0x50` for every
type. Each record it finds it unlinks (`0x464ec0`) and passes to `SMemFree` with the
`CMirrorHandler` tag (`0x464c8e`).

## Holding an object

A hold keeps a live object from being released while some code still points at it. Taking a hold
increments the object's hold count; dropping one decrements it, and when the count reaches zero
with the release-pending bit set, the object is released then.

Addresses: `0x4683e0`, `0x468410`, `0x614f10`, `0x614f40`, `0x614f00`, `0x464920`
Confidence: verified
Proof: one
Test: none

`0x4683e0(lo, hi)` looks the object up in the live table and calls `0x614f10(1)`, which increments
`+0xe8`; `0x614f10(0)` decrements it when nonzero. `0x468410` calls `0x614f10(0)`, then, when
`0x614f00` gives 0 and `0x614f40` gives the pending bit, `0x464920` with the GUID. The callers
of both are game object code (`0x5f3930`, `0x5f40e0`, `0x5f4120`).

## Reactivating an object

The create, values, movement and near-objects blocks look their GUID up so that a released object
comes back: when the live table misses and the released table has the GUID, the object is
unlinked from the released table and its reuse list and put back into the live table; when it is
not on the object list it is linked onto it and onto the reactivated list, and the caller is told
so. A found live object has its
release-pending bit cleared.

Addresses: `0x464530`, `0x4645ae`, `0x4645b2`, `0x4645c8`, `0x464635`, `0x46464c`, `0x464680`, `0x4646d9`, `0x4646ed`
Confidence: verified
Proof: one
Test: none

`0x464530(&reactivated, lo, hi)` clears the flag, walks the live table as `0x464890` does, and on a
match calls `0x614f50(0)` (`0x4645b2`). On a miss it walks the released table at `+0x44`/`+0x4c`
(`0x4645c8`); a match is unlinked at `+0x1c` and `+0x24` (`0x464635`) and `+0x38` (`0x46464c`),
inserted into the live table by `0x468d30` (`0x464680`). Only when its object-list link is empty
(`0x4646a9`) is it linked in, the flag set to 1 (`0x4646d9`) and `0x468940` called to link it
onto the list at `+0xb0` (`0x4646ed`). Its callers run vtable slot 2 on a set flag; the update handler's end runs
slot 3 on each object of that list (`update`).

## Allocating and reusing

A new object of type 1 to 7 comes from its type's pool. Before allocating, the manager takes the
oldest object off that type's reuse list, if any, unlinks it from the released table, runs its
vtable slot 0 with 0 and returns it to the pool; so a released object survives until the next
object of its type is created or the destroy message names it. The active player is allocated
from the heap: its 0x3178-byte members and fields, 4 bytes per watched field, and a 0x2550-byte
block it points to at `+0x1c68`.

Addresses: `0x466e00`, `0x466e0c`, `0x466e2b`, `0x466e3f`, `0x466e55`, `0x466efb`, `0x466f2e`, `0x466f6c`, `0x466f80`, `0x466fa4`, `0x467040`, `0x466de0`, `0x47d540`, `0x463a60`, `0x463b00`, `0x463b30`
Confidence: verified
Proof: one
Test: `objectlayer::player_dynfield_base_difftest`

`0x466e00(type, lo, hi)` compares the GUID with `0x468550` (`0x466e0c`): equal allocates
`0x2550 + 4·0x47d540() + 0x3178` bytes (`0x466e2b` to `0x466e3f`; `0x467040` returns 0x2550), and
`0x466de0` later points `+0x1c68` past the watch copy. Otherwise `0x466e55` reads the first node of the
list at `+0x44 + 12·type`; a node is unlinked from `+0x1c`, `+0x24` and `+0x38`, destroyed through
slot 0 by its type mask (`0x466f2e`) and freed to its pool (`0x466f6c`, `0x463b00`). `0x466f80`
allocates from `[0xb232e4 + 4·type]` with `0x463a60`, `0x463b30` maps the handle to the object,
and `0x466fa4` stores the handle at `+0x10`.

## Accessors and enumeration

The manager's own fields are read and written through small accessors: the active player's GUID
at `+0xc0`, a dword at `+0xc8`, the map id at `+0xcc`, the movement globals of `collision` at `+0xd4`, and two unreached readers of
`+0xd0` and `+0xd8`. `0x468380` walks the object list in order and calls a callback with each
object's GUID, stopping early when it returns 0. Two functions swap the current manager for the
network code and put it back.

Addresses: `0x468550`, `0x468570`, `0x468580`, `0x4685c0`, `0x4685e0`, `0x468600`, `0x468610`, `0x468630`, `0x468380`, `0x464fa0`, `0x464fb0`, `0x464fd0`, `0xb41418`
Confidence: verified
Proof: one
Test: none

`0x468550` returns `+0xc0`/`+0xc4` in `edx:eax`, or 0 with no manager; `0x468570` returns `+0xc8`
without a test; `0x468580` returns `+0xcc` or 0; `0x4685e0` and `0x468600` read and write `+0xd4`;
`0x4685c0` and `0x468610` return `+0xd0` and `+0xd8`; `0x468630` is a lone `ret`. `0x468380`
starts at `[+0xac]`, calls `edi(ecx = edx, lo, hi)` and follows the link at `[+0xa4]`; a 0
returns 0, the list's end returns 1. `0x464fb0` saves `[0xb41414]` in `0xb41418` when that is
empty and installs `ecx`; `0x464fd0` puts the saved one back and clears the save.

## Leaving the world

The manager is torn down when the client leaves the world or loads a new one: `0x467700` frees
every object, live, released and pending, through their slots 0 and their pools, then the tables
and the manager, and clears `0xb41414`; `0x468490` frees the field watchers left on the global
lists and unregisters the `ObjUsage` console command. The command prints the live and pending
object counts.

Addresses: `0x467700`, `0x467711`, `0x46771e`, `0x467800`, `0x467bf0`, `0x468490`, `0x468530`, `0x467da0`, `0x467e34`, `0x467d40`, `0x467d10`
Confidence: inferred
Proof: none
Test: none

`0x467700` is called from `0x401bc0` at `0x401bf7` and from one other site, calls `0x467800` (`0x467711`),
frees the tables and stores 0 at `0xb41414` (`0x46771e`). `0x467800` and `0x467bf0` walk the lists
calling slot 0 and `SMemFree` or `0x463b00`. `0x468490` frees `CMirrorHandler` records and passes
`ObjUsage` to the console (`0x468530`); `0x467da0` writes `Object manager list status:` and the
counts. `0x467d40` creates the seven pools; `0x467d10` tests the manager's block for the debug dump.
Not read instruction by instruction.

## Freeing every object

`0x467800` frees all objects when the manager is destroyed. It walks the live table's iteration
list twice: first every object whose GUID's top four bits are neither 6 nor 8, then the rest.
Then it walks the released table's list, and removes the three update handlers.

Addresses: `0x467800`, `0x46784d`, `0x467820`, `0x467954`, `0x467a27`, `0x467a30`, `0x467a84`, `0x467bf0`, `0x467c5b`, `0x464c40`, `0x463b00`, `0x537a80`
Confidence: verified
Proof: one
Test: none

Each object is unlinked from the live table's two links (`+0x1c`, `+0x24`) when `+0x20` is set, and
from the object list when it is linked (`0x46789a`); `0x464c40` frees its watcher records; slot 0
runs by the type mask (`0x4678dc` to `0x46790a`); and the memory goes back to the pool of its
type through `ObjectFree` (`0x46792e`), except the active player's, which `SMemFree` frees
(`0x46790a`). The top-nibble test is at `0x46784d`: `0x467877` and `0x467881` skip 6 and 8 in the first
round; the second round starts at `0x467954`. `0x467a30` takes each node of the released list
(`+0x34`), unlinks it at `+0x38`, and `0x467bf0` frees it the same way. `0x467a84` calls
`ClearMessageHandler` on the network client for opcodes 0xa9, 0x1f6 and 0xaa.

## Adding an object

`0x4643b0` links a new object into the live table under its GUID. The first insert makes the
table: mask 3 and four buckets, each a list linked through offset `0x1c`. The bucket is `lo &
mask`. A credit at `+0x10` falls by 3 per insert, to 0; each node in the chosen chain adds 1, and
a total over 13 rehashes to `2·mask + 2` buckets and starts again. The table stops growing at
mask `0x1fff`. The object goes onto its bucket and the iteration list (`+0x4` of the table), and
`+0x18`, `+0x30` and `+0x34` take the GUID.

Addresses: `0x4643b0`, `0x4643ed`, `0x4643fa`, `0x46440e`, `0x46442c`, `0x464437`, `0x46447c`, `0x46448b`, `0x46449c`, `0x4644b9`, `0x4644c8`, `0x4699f0`, `0x468f10`, `0x469810`, `0x468940`
Confidence: verified
Proof: one
Test: none

`0x4643ed` stores 3 in the mask; `0x4643fa` sizes the bucket array (`0x4699f0`, mask + 1);
`0x46440e` makes each bucket list (`0x468f10` with offset `0x1c`). `0x46442c` ends growth at
`0x1fff`. `0x464437` subtracts 3 from `+0x10` (to 0 below 4). The chain walk `0x46447c` counts
nodes (`inc`, `cmp 0xd`); `0x46449c` rehashes with `0x469810` and the doubled size. `0x4644b9` and
`0x4644c8` link with `0x468940`. The constructor `0x6139a0` calls it.

## Trace lines

`0x464700` and `0x464ad0` write a status line naming the object's type: `Remembering object`,
`item`, `container`, `unit`, `player`, `game object`, `dynamic object` or `corpse` when an object is
created or reactivated (`0x464700`), and `Forgetting …` before one is released (`0x464ad0`). The
line goes through `ValidateStatusLevel` with level 0x20; a GUID that is not live, or a type outside
1 to 7, writes nothing.

Addresses: `0x464700`, `0x464ad0`, `0x464519`, `0x464966`, `0x44cb20`
Confidence: verified
Proof: one
Test: none

Both look the GUID up with `0x464870` and index a jump table by the object's type id minus 1
(`0x464700`: the table at 0x4647c8). The strings are `Remembering …` from 0x8367b8 to 0x836854 and
`Forgetting …` from 0x836868 to 0x8368fc. `0x464519` is the call in the find-or-create function
(`0x4644f0`) and `0x464966` the call in the release (`0x464920`).

## Watcher copy offsets

A watcher's copy offset says where in the copy array its range lives. `0x4680c0(type, offset,
length)` takes the type's section: the item, container, unit, player, game object, dynamic object
or corpse index function with `offset / 4` and `(length + 3) / 4`, adds the count of the section's
parent (the unit's or the object's, the item's for a container), and returns that sum times 4 plus
`offset & 3`. A type outside 1 to 7 uses the object section alone.

Addresses: `0x4680c0`, `0x4680ea`, `0x468114`, `0x46813e`, `0x468168`, `0x468192`, `0x4681bc`, `0x4681e6`, `0x468200`
Confidence: verified
Proof: one
Test: none

The jump table at 0x468224 sends the types 1 to 7 to the blocks of the item (`0x468158`), container
(`0x46812e`), unit (`0x468104`), player (`0x4680da`), game object (`0x468182`), dynamic object
(`0x4681ac`) and corpse (`0x4681d6`). The calls are the sections' `…WatchedIndex` functions
(`fields`) and the `…WatchedCount` functions of the parents (`0x47db10`, `0x47f2b0`, `0x47e5d0`).

## The object list

An object an enumeration should not see is taken off the object list, and put back when it may be
seen. `0x468270(guid)` unlinks the live object with that GUID from the list at `+0xa4` if it is
on it; `0x4682f0(guid)` links it at the list's head if it is not on it. Items and game objects
call the first at construction and reactivation, and a game object calls the second when its
record arrives.

Addresses: `0x468270`, `0x4682f0`, `0x5d8267`, `0x5f703e`, `0x5f7ec3`, `0x5f7d00`, `0x464890`
Confidence: verified
Proof: one
Test: none

Both find the object with `0x464890` and test its link at the list offset `[+0xa4]`: `0x468270` acts when
the link's next is set, `0x4682f0` when it is clear.

## List code

The rest of the file is container template code, instantiated for `CGObject_C` and the GUID
tables. A list is a link offset, a first link and an end link marked by bit 0.

Addresses: `0x468640`, `0x468680`, `0x468b20`, `0x468b90`, `0x468b60`, `0x468bd0`, `0x468940`, `0x468a00`, `0x468a30`, `0x468e90`, `0x468ee0`, `0x468f10`, `0x468f80`, `0x469070`, `0x4694b0`, `0x468ec0`, `0x4688b0`, `0x467430`, `0x464ec0`
Confidence: verified
Proof: one
Test: none

`0x468640`, `0x468680`, `0x468b20` and `0x468b90` are one body, as are `0x468b60` and `0x468bd0`: the
first unlinks a node (the neighbours are joined through the bit-0 end marks) and the second returns
the address a link names, negative offsets resolving through the list's offset. `0x468a00`
(offset `0x38`), `0x468a30` (`0x40`), `0x468e90` (−0x22222223), `0x468ee0` (0) make an empty list;
`0x468f10` empties one and sets its offset, `0x468f80` and `0x4688b0` empty one, and `0x469070` and
`0x4694b0` make a bare node. `0x468ec0` returns a node's link address, and `0x468940` unlinks a
node and links it again by a placement argument. `0x467430` and `0x464ec0` are destructors that
unlink and free.

## Table and array code

A hash table is a vtable `0x803600`, a list, a bucket array and a mask, −1 until used. The code
inserts, rehashes and frees its buckets, and grows the arrays behind them.

Addresses: `0x468d30`, `0x469810`, `0x4699f0`, `0x469080`, `0x469290`, `0x468c30`, `0x468fe0`, `0x4691e0`, `0x469c20`, `0x469dc0`, `0x469d60`, `0x469da0`, `0x4686c0`, `0x4686f0`, `0x468a60`, `0x4687f0`, `0x468c00`, `0x468fd0`, `0x469b70`, `0x469c10`, `0x469bc0`, `0x469b30`, `0x47fea0`, `0x47fd40`, `0x803600`
Confidence: verified
Proof: one
Test: none

`0x468d30` inserts into a table as `0x4643b0` does. `0x469810`, `0x4699f0`, `0x469080`, `0x469290`,
`0x468c30`, `0x468fe0` and `0x4691e0` rehash, size, empty and free the bucket array; `0x469c20`
and `0x469dc0` grow arrays with `SMemReAlloc`, and `0x469d60` and `0x469da0` round a size to a
power of two (at most 0x15) and up to a multiple. `0x4686c0`, `0x4686f0`, `0x468a60` and
`0x4687f0` are slots 0 to 3 of the table vtable: a deleting destructor, a `CGObject_C` factory
that builds the six watcher lists, and two destructors of the table. `0x468c00` reads three
floats from a packet, and `0x468fd0`, `0x469b70`, `0x469c10`, `0x469bc0` and `0x469b30` zero, set
up or free small members. `0x47fea0(count)` stores the count at `this` and resizes the buffer at
`this+8` to four bytes per element with `SMemReAlloc` (allocation tag 0x82f25c) when a buffer exists or the
count is not 0; `0x47fd40` and the `ui` code at `0x4a96b0` call it.
