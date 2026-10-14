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
counts. `0x467d40` creates the seven pools; `0x467d10` is its shutdown. Not read instruction by
instruction.
