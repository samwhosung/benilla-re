# cvar: containers

The CVar registry and the console command registry keep their records in the same hash table
template of the Storm library. A table holds each record on two intrusive lists at once: a chain
for its hash bucket and one list of every record in registration order. The two instances compile
to near-identical code, so each function of the CVar file has a twin in the command registry file
(`console/commands#table-twins`). The console's line list uses the same link and list code
(`console/screen#line-records`).

Roots: `0x63e520`, `0x63e6d0`, `0x63e6f0`, `0x63ed80`, `0x63e9c0`, `0x63ec30`, `0x63e7b0`, `0x63e9f0`, `0x63ec40`, `0x63ee10`, `0x63edb0`, `0x63edf0`, `0x63e2b0`, `0x63e2e0`, `0x63e3a0`, `0x63e460`, `0x63e560`, `0x63e5f0`, `0x63e910`, `0x63ebe0`

## Links and lists

A link is two dwords: `+0` the previous link and `+4` the next object's own address, not its link, or the terminator's address with bit 0
set at the end; both are 0 while unlinked. A list is a link offset (an object's link is at its address plus the offset) and a terminator
link: its `+0` is the last link, its `+4` the first object, and an empty list points at itself. `LinkNode(node, mode, dest)` unlinks the node and
links it after `dest`'s link (the terminator when 0) for mode 1, else before it: with `dest` 0, mode 1 makes the node first and mode 2 last.

Addresses: `0x63e520`, `0x63e6d0`, `0x63e6f0`, `0x63ed80`, `0x63e9c0`, `0x63ec30`, `0x63e543`, `0x63e54a`, `0x63e538`, `0x63e540`, `0x63e74b`, `0x63e74f`, `0x63e767`, `0x6404a0`, `0x6406a0`, `0x6406c0`, `0x640670`, `0x640970`, `0x6409f0`, `0x63cfb0`, `0x63cf80`
Confidence: verified
Proof: one
Test: none

Unlinking points the next link back at the previous one, makes the previous link's next the removed link's next, and zeroes the link. `0x63e520` takes the next link: for an end mark (bit 0 set, or 0) it is the terminator, otherwise the next object plus
the offset, found as this link minus the previous link's next field (`0x63e538` to `0x63e540`). `0x63e543` stores the previous link in it, and
`0x63e54a` stores this link's next in the previous link's next. `0x63e6d0` returns `[list] + node`, or the terminator `list + 4`
for node 0. `0x63e6f0` decrements the mode at `0x63e74e`: 1 takes the branch at `0x63e767`, which links after `dest`;
the other path stores the link after `dest`'s previous link. `0x63ed80` returns the slot that holds the next link's back
pointer, or the terminator's for an end mark; with offset −1 it resolves the offset from the link itself.
`0x63e9c0` makes an empty list and `0x63ec30` re-initialises a terminator. The command registry's twins
and the console's `0x63cfb0` and `0x63cf80` are the same code.

## Hash table object

A table object is `+0` the vtable, `+4` the link offset of the all-records list, `+8` and `+0xc` its terminator,
`+0x10` a chain counter, `+0x14` the bucket vector (capacity, count, data, granularity) and `+0x24` the mask, −1 while
there are no buckets. A bucket is 12 bytes: the link offset (4 once set, `0xdddddddd` before), then a terminator. A record
holds its hash at `+0`, its bucket link at `+4` and its list link at `+0xc`. The index of a name is `SStrHashHT(name) & mask`; a
lookup walks the bucket from its first object and accepts the first whose stored hash is equal and whose name matches
`SStrCmpI`.

Addresses: `0x63d020`, `0x63d0be`, `0x63d0d2`, `0x63de79`, `0x63de87`, `0x64b3f0`, `0xc4ed94`, `0xc4ed98`, `0xc4ed9c`, `0xc4eda0`, `0xc4eda4`, `0xc4eda8`, `0xc4edac`, `0xc4edb0`, `0xc4edb4`, `0xc4edb8`, `0x80e1dc`, `0x80e268`, `0x865124`
Confidence: verified
Proof: one
Test: none

The CVar table is at `0xc4ed94` (vtable `0x80e1dc`), the command table at `0xc4f848` (vtable `0x80e268`). `0x63d020` builds the empty CVar table: the terminator `0xc4ed9c` points at itself, `0xc4eda0` holds its address with bit 0 set,
the vector words and the counter are 0, the offset holds `0xdddddddd` until `0x63d0be` stores `0xc`, and `0x63d0d2` stores the
mask −1. The vector's four words are `0xc4eda8` (capacity), `0xc4edac` (count), `0xc4edb0` (data) and `0xc4edb4` (granularity). A
lookup (`0x63de30`) hashes with `0x64b3f0`, reads the first object of the bucket at `data + 12·index + 8`, and compares
`[node]` with the hash (`0x63de79`) before the name (`0x63de87`); the walk ends at an end mark. The bucket array is allocated under the tag
`0x865124`.

## Growth

The first insertion makes four buckets with mask 3. Before each later insertion the table calls
`0x63e9f0` with the bucket index; it returns 1 when it grew. The call lowers the chain counter by 3 (to 0 if it is below 4),
adds 1 for each object it walks in that bucket, and grows when the counter passes 13, unless the mask is already `0x1fff`. Growth
unlinks every object into a temporary list in bucket order, resizes the vector to twice the buckets, empties all of
them with link offset 4, sets the mask to the new count minus one, and links each object at the end of
its new bucket.

Addresses: `0x63e7b0`, `0x63e9f0`, `0x63ea0a`, `0x63ea0f`, `0x63e7bb`, `0x63e85e`, `0x63ea40`, `0x63ea6b`, `0x63ea57`, `0x63ea74`, `0x63eb65`, `0x63eb90`, `0x63ec40`, `0x63ebe0`, `0x63ee10`, `0x63edb0`, `0x63edf0`, `0x63dcb4`, `0x640780`, `0x640a00`, `0x640b40`, `0x6409a0`
Confidence: verified
Proof: one
Test: none

`0x63e7b0` stores mask 3 (`0x63e7bb`), sizes the vector with `0x63edb0`, `0x63edf0` and `0x63ee10`, and stores the count 4 (`0x63e85e`). `0x63e9f0` first compares the mask with
`0x1fff` (`0x63e9f9`); `0x63ea0a` compares the counter with 3 and `0x63ea0f` subtracts 3. `0x63dcb4` is the call in the CVar registration. The loop `0x63ea40` to `0x63ea6b` adds 1 per object
and compares with 0xd at `0x63ea57`. At `0x63ea74` the new count is twice the old; `0x63eb65` stores the new mask, and `0x63eb90` calls
`LinkNode` with mode 2 and destination 0. `0x63ec40` sets the count: it rounds a larger size up to a multiple of the granularity
(`0x63edf0`), where the granularity is the largest power of two not above the size, at most 0x15 (`0x63edb0`), and reallocates through
`0x63ee10`, which tries `SMemReAlloc` and otherwise allocates and copies. `0x63ebe0` unlinks a bucket's whole chain. The command table
inlines the first-time sizing at `0x63fa95`; `0x640780`, `0x640a00` and `0x640b40` are the twins.

## Records and virtual slots

The vtable has four slots. Slot 0 frees a record (`0x63e2b0`: the CVar destructor, then `SMemFree`). Slot 1 allocates a record
(`0x63e2e0`: `0xc4` plus the extra size, zero-filled, link words cleared, and linked first in the bucket given). Slot 2 is the deleting
destructor (`0x63e460`) and slot 3 clears the table (`0x63e3a0`): it unlinks every record from the list and the buckets, sets
the mask to −1 and frees the bucket array. Neither frees the records themselves.

Addresses: `0x63e2b0`, `0x63e2e0`, `0x63e3a0`, `0x63e460`, `0x63e560`, `0x63e5f0`, `0x63e910`, `0x63d260`, `0x63d280`, `0x63e358`, `0x63e407`, `0x80e1e0`, `0x80e1e4`, `0x80e1e8`, `0x865388`, `0x646430`, `0x6462e0`
Confidence: verified
Proof: one
Test: none

The vtable words `0x80e1dc` to `0x80e1e8` hold these four in order. `0x63e2e0` allocates under the tag `0x865388` with flag 8
(zero fill), calls `0x63d260` to clear the five link words `+4` to `+0x14`, and from `0x63e358` links the record after the bucket's terminator
(mode 1). `0x63e3a0` stores the mask −1 at `0x63e407`. `0x63e460` resets the vtable, unlinks the list, empties each bucket,
frees the vector through `0x63e5f0` and, for a deleting call (`bit 0` of the argument), frees the table. `0x63e560` empties
a bucket and unlinks its terminator; `0x63e910` does that for a bucket and optionally frees it.
