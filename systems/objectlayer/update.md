# objectlayer: update

How the client applies the server's object updates: `SMSG_UPDATE_OBJECT` (`0xa9`),
`SMSG_COMPRESSED_UPDATE_OBJECT` (`0x1f6`) and `SMSG_DESTROY_OBJECT` (`0xaa`).

The object manager registers the three handlers when it is created. An update packet is a list
of blocks, and the client reads it three or four times: an out-of-range list first, when it is
the first block; then a pass that creates objects and writes their fields, run once, when the header
asks, for GUIDs whose top four bits are 6 or 8, and once for the rest; then a pass that reads the blocks again and fires the change
notifications. Objects the first passes reactivated get one more call after the last block. The
data store reads are `base` code; a read past the end leaves its output unwritten. The block
readers and the passes are in ObjectMgrClient.cpp; the movement block's codec is `movement`, the
objects' lifecycle `manager`, and the field machinery `fields`.

Roots: `0x4651a0`, `0x4672f0`, `0x4674a0`

## Handler registration

`0x465140` registers three handlers on the client connection the manager holds at `+0xd0`,
each with context 0: `0x4651a0` for `0xa9`, `0x4672f0` for `0x1f6` and `0x4674a0` for `0xaa`.
The manager's constructor calls it once, right after storing that connection.

Addresses: `0x465140`, `0x464ff0`, `0x465120`, `0x465125`, `0x4685b0`, `0x537a60`
Confidence: verified
Proof: one
Test: none

`0x465140` loads `[0xb41414]+0xd0` into `ecx` three times and calls `0x537a60` with pushes 0,
handler, opcode (`0x46514b` to `0x465191`). `0x464ff0` stores `[0xc28114]` at `+0xd0` through
`0x4685b0` (`0x465120`) and calls `0x465140` next (`0x465125`).

## Update packet

The packet is a u32 block count, a u8 flag, then the blocks, each a u8 type and its body. When
the first block's type is 4 the client handles that out-of-range list first and starts the
later passes at block 1. A nonzero flag runs a pass over the blocks whose GUID's top four bits
are 6 or 8; a second pass handles every other GUID. Both passes start after the
out-of-range block and end with the read position put back. The third pass rereads all blocks
from block 0 and notifies. Then every object a pass reactivated is taken off the manager's list
at `+0xb0` and gets its vtable slot 3 (`+0x0c`). The handler returns 1.

Addresses: `0x4651a0`, `0x418eb0`, `0x418cb0`, `0x4651b2`, `0x4651c1`, `0x4651dc`, `0x4651e1`, `0x4651e9`, `0x4651f7`, `0x46520e`, `0x465222`, `0x46522d`, `0x465290`, `0x4652ff`, `0x466010`, `0x465ec0`
Confidence: verified
Proof: one
Test: none

`0x4651b2` reads the count (`0x418eb0`), `0x4651c1` the flag into `ebx` (`0x418cb0`); the cursor `+0x14` is saved in
`[ebp-0x8]`. `0x4651e1` compares the next byte with 4: equal calls `0x465ec0` and sets `edi` to 1;
otherwise `0x4651f7` rewinds. With `ebx` nonzero, `0x46520e` calls `0x466010(time, count, edi,
store, 1)` and the cursor is restored; `0x465222` calls it with 0. `0x46522d` rewinds to
`[ebp-0x8]` and the loop reads each type. `0x465ec0` returns 0 (`0x465fbd`), so its result,
subtracted from `ebx`, changes nothing. `0x465290` to `0x4652ff` unlink each node of the list at
`+0xb0` and `call [eax+0xc]`.

## Block types

The two passes and the notify pass dispatch on the block type through jump tables. A type above 5
consumes nothing more in either, so the next byte is read as the next block's type.

| type | name | create pass | notify pass |
|---|---|---|---|
| 0 | values | `0x467160` writes the fields | `0x465330` fires watchers |
| 1 | movement | `0x467050` applies it | `0x465ea0` skips it |
| 2, 3 | create | `0x4660a0` creates or updates | `0x465c50` runs the create hook |
| 4 | out of range | reads nothing | `0x465fd0` skips the list |
| 5 | near objects | `0x467230` reactivates | `0x465fd0` skips the list |

Addresses: `0x466010`, `0x466084`, `0x465314`, `0x46524d`, `0x466044`, `0x465fd0`, `0x465ea0`
Confidence: verified
Proof: one
Test: none

The notify table `0x465314` holds `0x46526f`, `0x465266`, `0x465254`, `0x465254`, `0x46527e`,
`0x46527e`; the pass table `0x466084` holds `0x466062`, `0x466057`, `0x46604b`, `0x46604b`,
`0x466076`, `0x46606d`. Entry 4 of the pass table is the loop's decrement at `0x466076`, so the
pass reads no body for type 4: an out-of-range block anywhere but first throws that pass's reading
out of step for the rest of the packet. Both dispatches bound the type with `cmp eax,0x5; ja` to
the loop's end. The notify pass passes `type == 3` to `0x465c50` (`0x465254`, `sete`).

## Top-nibble 6 and 8 GUIDs

The passes split blocks by one test on the GUID: its top four bits are 6 or 8, `(guid >> 60) ==
6` or `== 8`. Each block handler of the two passes reads the block's GUID and handles the block
only when the pass's flag equals that test; otherwise it skips the block's body. So the flagged
pass handles those GUIDs before anything else in the packet, and the second pass everything else.
A packet whose header flag is 0 runs only the second pass, so its blocks for such GUIDs are
skipped by every pass.

Addresses: `0x4660d8`, `0x4660e6`, `0x4660f1`, `0x466105`, `0x467188`, `0x4671b5`, `0x467290`, `0x4672bf`
Confidence: verified
Proof: one
Test: none

`0x4660d8` takes the GUID's high dword, `and eax,0xf0000000`, and compares it with `0x60000000`
(`0x4660e6`) and `0x80000000` (`0x4660f1`); a match gives 1. `0x466105` XORs it with the pass flag
and skips on a difference (`0x466109` calls `0x466af0`). `0x467188`/`0x4671b5` repeat the test
for values and `0x467290`/`0x4672bf` for near objects. The movement handler `0x467050` does not
test the GUID: in the flagged pass it skips every movement block (`Movement blocks`).

## What the flagged GUIDs are

The header flag and the split are read as transports: vmangos names the flag `hasTransport`
(`Map.cpp:1695`), and the client runs these objects first. Nothing in the client ties the test to the
transport game object types. vmangos gives transports the high words `0xF120` and `0x1FC0`
(`ObjectGuid.h:72`, `:77`), top nibbles F and 1, so against such a server the flagged pass
handles no block and every transport goes through the second pass.

Addresses: `0x4660d8`, `0x465f10`
Confidence: inferred
Proof: none
Test: none

The test sites are the same as above; which object kinds the server gives top nibbles 6 and 8 is
not settled here.

## Out-of-range objects

An out-of-range block is a u32 count and that many packed GUIDs. Handled as the first block, it
releases each listed object (`manager`) in two rounds: every GUID whose top four bits are not 6
or 8, in list order and except the active player, then the others, without that exception. The read position ends after the list.
In any other position the notify pass skips the list and nothing is released.

Addresses: `0x465ec0`, `0x465ee1`, `0x465f04`, `0x465f10`, `0x465f39`, `0x465f4f`, `0x465f69`, `0x465fa6`, `0x465fb3`, `0x464920`, `0x465fd0`
Confidence: verified
Proof: one
Test: none

`0x465ee1` reads the count. Each GUID (`0x465f04`, `0x642ed0`) whose high dword passes the
top-nibble test at `0x465f10` counts one in
`[ebp-0x8]` (`0x465fc1`); any other is compared with `0x468550` (`0x465f39`) and released through
`0x464920` unless equal (`0x465f4f`). With a nonzero count of those, `0x465f69` saves the
position and rewinds to the list's start, and the loop releases those GUIDs (`0x465fa6`)
until it has seen that many, without the active-player test; `0x465fb3` restores the position.
`0x465fd0` reads the count and the GUIDs and does nothing with them.

## Create block

A create block (types 2 and 3) is a packed GUID, a u8 object type, a u8 update flag set, the
flagged fields, then a values block. The flags, in read order:

| flag | read |
|---|---|
| 0x20 | the movement block (`movement`) |
| 0x40, without 0x20 | position: four floats, x, y, z, facing |
| 0x08 | a u32 |
| 0x10 | a u32 |
| 0x04 | a packed GUID |
| 0x02 | a u32 |
| 0x01 | nothing; marks the receiver's own player, for a new object only |

Addresses: `0x4660a0`, `0x4660c8`, `0x4660d3`, `0x466185`, `0x46618d`, `0x4661a0`, `0x4661dc`, `0x4661f6`, `0x46620e`, `0x466226`, `0x466237`, `0x46623b`, `0x466245`, `0x466254`, `0x466af0`
Confidence: verified
Proof: one
Test: none

`0x4660c8` reads the GUID, `0x4660d3` the type byte, `0x466185` the flags. `test al,0x20` at
`0x46618d` calls `0x47f050`; else `test al,0x40` reads four floats with `0x419130`. The u32s are
read by `0x418eb0` at `0x4661e8` and `0x466202`, the packed GUID by `0x642ed0`, the last u32 by
`0x418fb0` at `0x466232`. Flag 1 stores the block's GUID at the manager's `+0xc0`/`+0xc4`
(`0x466245`, `0x466254`), which `0x468550` returns, only on the path for a GUID the lookup did
not find (`0x46623b`, ahead of `0x466e00`); `0x466350`, the update of a known object, reads the
flag and stores nothing. The same order and gates are read by the skip `0x466af0`, by `0x466350`
and by the notify pass's `0x465c50`. All of it lands in one structure at `[ebp-0x118]`,
the movement block from `+0x00`, the flags at `+0xd8`, the u32s at `+0xdc` and `+0xe0`, the GUID
at `+0xe8` and the last u32 at `+0xf0`, which the constructor and the create hooks receive.

## Creating an object

The create pass looks the GUID up, reactivating it when it was released (`manager`). An existing
object is updated by `0x466350`: a unit applies the block's movement through `0x5ff070`, with
whether it is the active player; any other type reads and drops the movement fields. Its values
are then written with every unset field cleared to 0. A new object is allocated for its type,
its field array zeroed, its type id and mask stored, its values written, and its class
constructor run with the time and the movement block; it then joins the manager's object list.

Addresses: `0x4660a0`, `0x46612d`, `0x46613c`, `0x46614c`, `0x466350`, `0x46638c`, `0x466498`, `0x466582`, `0x466265`, `0x466279`, `0x46628e`, `0x466320`, `0x46629f`, `0x4662c7`, `0x466e00`, `0x466c70`, `0x613880`, `0x466a20`, `0x466bc0`, `0x44cb20`
Confidence: verified
Proof: one
Test: `objectlayer::zero_descriptor_block_difftest`

`0x46612d` calls `0x464530`; found, `0x46613c` calls `0x466350` and, when the lookup reactivated
it, slot 2 (`0x46614c`). `0x46638c` tests bit 3 of the type mask (a unit); `0x466498` passes the
parsed block and the active-player compare. `0x466582` calls `0x466590` with flags 0 and 1, the
second clearing unset fields. A new object: `0x466265` calls `0x466e00(type, guid)`, `0x466279`
`0x466c70`, `0x46628e` `0x466320`, which calls `0x613880` and `0x466590` with flags 1 and 0.
`0x46629f` calls `0x466a20(time, object, block)`. `0x4662c7` links the object into the list at
the manager's `+0xa4`; `0x466bc0` then passes the type's `Creating …` text to `0x44cb20`, which
returns 1 for a text and a level below 4 and has no other effect.

## Values block

A values block is a u8 count `n`, `n` u32 mask words, then one u32 for each set mask bit, in
increasing field index. Bit `i` is word `i >> 5`, bit `i & 31`. Only indices below the object's
field count are visited: 6 for an object, 48 item, 122 container, 188 unit, 1282 for the active
player and 486 for another player, 26 game object, 16 dynamic object, 38 corpse. Words past
`n` count as zero up to 41; a bit set at or above the field count is never read, and its value
stays in the packet, out of step with the reader.

Addresses: `0x466590`, `0x4665a7`, `0x4665c4`, `0x4665d5`, `0x466606`, `0x465a80`, `0x4666bb`, `0x4666e3`, `0x465550`, `0x465bb0`
Confidence: verified
Proof: one
Test: `objectlayer::apply_update_fields_difftest`, `objectlayer::updatemask_test_bit_difftest`, `objectlayer::updatefield_count_difftest`

`0x4665a7` reads `n` and `0x4665c4` the words into a local array; `cmp edi,0x29` at `0x4665d5`
zero-fills up to 41 (`Mask words past 41`). `0x466606` gets the count from `0x465a80`, which maps the type mask
through the byte table `0x465b2c` to the jump table `0x465b08`; mask `0x19` returns `0x502` when
the GUID is the active player's, else `0x1e6`. The loop runs `edi` below that count; `0x4666bb`
tests the bit with `0x465550`, `(1 << (i & 31)) & word[i >> 5]`, and `0x4666e3` reads 4 bytes.
`0x465bb0`, the skip, reads `n`, the words, and a u32 for each set bit below `32n`.

## Mask words past 41

The three values readers, the create pass's, the skip and the notify pass's, read the mask words
into a stack array of 41 dwords, enough for 1282 fields, and bound the read only by the count
byte. A count above 41 writes past the array. In the create pass's reader and the skip, the
43rd word lands on the saved data store pointer, so a count of 43 or more sends the reads that
follow through that word's value.

Addresses: `0x466590`, `0x4665a4`, `0x4665b5`, `0x4665c0`, `0x4665d1`, `0x4666e0`, `0x465bb0`, `0x465bc0`, `0x465bd1`, `0x465bd7`, `0x465330`, `0x465395`, `0x4653b4`
Confidence: verified
Proof: one
Test: `objectlayer::apply_update_fields_difftest`

`0x466590` reserves `0xc4` bytes, saves the store at `[ebp-0x1c]` (`0x4665a4`) and reads the words
from `[ebp-0xc4]` (`0x4665b5`), so 41 fit below `[ebp-0x20]`; the loop's one bound is `cmp edi,ecx`
against the count (`0x4665d1`), and word 42, counting from 0, lands on `[ebp-0x1c]`, reloaded at
`0x4665c0` and `0x4666e0`. `0x465bb0` saves the store at `[ebp-0x8]` (`0x465bc0`) and fills from
`[ebp-0xb0]` (`0x465bd1`): word 42 is `[ebp-0x8]`, reloaded at `0x465bd7`. `0x465330` fills from
`[ebp-0xcc]` (`0x465395`) and zero-fills to 41 (`0x4653b4`); its overrun reaches other locals.

## Writing the values

The create pass writes each sent value straight into the field array, `[object+0x8] + 4i`, as a
raw dword. For an update of an existing object (flag 0) it first copies the current bytes of any
watched field into the object's watch copy, so the notify pass can compare (`fields`). With the
clearing flag, every unsent field below the count is written 0. A values block for an object the
manager does not know, live or released, is skipped.

Addresses: `0x467160`, `0x4671e2`, `0x46720f`, `0x46721f`, `0x4666f0`, `0x466a00`, `0x6142e0`, `0x46664b`, `0x466686`, `0x4666cf`
Confidence: verified
Proof: one
Test: `objectlayer::apply_update_fields_difftest`

`0x467160` reads the GUID, applies the top-nibble test and looks the object up with `0x464530`
(`0x4671e2`); none skips with `0x465bb0`. `0x46720f` calls `0x466590(object, store, guid, 0, 0)`,
then slot 2 for a reactivated object (`0x46721f`). In `0x466590`, `0x4666f0` calls `0x466a00`,
which calls `0x6142e0`: `mov [eax+edx*4],ecx` on `[ecx+0x8]`, `ret 8`. `0x46664b` skips the copy
when the first flag is set; `0x466686` copies through `0x4667a0`. `0x4666cf` tests the second flag
and stores 0 into the value slot `[ebp+0xc]` before the write.

## Notify pass

The notify pass reads every block again. A values block's object, live only, has each watcher of
each sent field run its change check (`fields`); the values themselves are read and dropped. A
create block's object either gets the same check over all its fields, when bit 4 of its `+0xe4` is set, or
its type's create hook with the time, the movement block and whether the block's type was 3; then
its values block is skipped. The hooks: `0x613af0` object, `0x5d8400` item and container,
`0x5fb880` unit, `0x5debe0` player, `0x5f7c40` game object, `0x5d5420` dynamic object, `0x5d6260`
corpse.

Addresses: `0x465330`, `0x465365`, `0x465371`, `0x465487`, `0x4654a4`, `0x465c50`, `0x465c97`, `0x465d79`, `0x465d8e`, `0x465da4`, `0x465e56`, `0x465e78`, `0x614e90`
Confidence: verified
Proof: one
Test: none

`0x465330` reads the GUID unless `dl` is set and looks it up with `0x464890` (`0x465365`), table
one only; none skips the values (`0x465371`). For a set bit, `0x465487` reads the value into a
local, and `0x4654a4` calls `0x465570`; with `dl` set it also calls it for clear bits.
`0x465c50` looks the object up with `0x464870` (`0x465c97`) and rereads the create fields.
`0x465d79` calls `0x614e90`, which returns `[ecx+0xe4] & 4`; nonzero calls `0x465330` with `dl` 1
(`0x465d8e`). Otherwise the
type id indexes the table `0x465e78`, whose eight entries call the hooks with `ecx` the object,
then `0x465e56` skips the values.

## A create notify with no object

The create notify does not check its lookup, unlike the values notify. A create block whose
object is not live when the notify pass runs reaches the flag test with a null object and reads
address `0xe4`. One packet does this from the bytes alone: a create block for a top-nibble 6 or 8
GUID not yet live, in a packet whose header flag is 0, which no create pass handles.

Addresses: `0x465c50`, `0x465c97`, `0x465d77`, `0x465d79`, `0x614e90`, `0x466105`, `0x466af0`, `0x465371`
Confidence: verified
Proof: one
Test: none

`0x465c97` calls `0x464870` and `0x465ca2` moves the result to `edi`; no instruction tests `edi`
before `0x465d77` moves it to `ecx` and `0x465d79` calls `0x614e90`, `mov eax,[ecx+0xe4]`. In the
create pass, `0x466105` compares the GUID test with the pass flag and sends a mismatch to the skip
`0x466af0`; with the header flag 0 only the pass with flag 0 runs. `0x465330` tests its lookup
and skips the values on none (`0x465371`).

## Movement blocks

A movement block is a packed GUID and a movement block (`movement`). The flagged pass skips it as
8 raw bytes and a movement block, so a packed GUID shorter than 8 bytes throws the rest of that
pass out of step. The second pass parses it and drops it when the GUID is the unit the client
moves (`0xc4da98`); otherwise a known object, reactivated if released, applies it through
`0x5ff030`. The notify pass skips it.

Addresses: `0x467050`, `0x467060`, `0x467140`, `0x46714d`, `0x467080`, `0x4670a0`, `0x4670a5`, `0x4670bf`, `0x467110`, `0x465ea0`, `0xc4da98`
Confidence: verified
Proof: one
Test: none

`0x467050` tests `dl`; set, `0x467060` calls `0x467140`, which calls `0x419ac0` for 8 bytes
(`0x46714d`) and then `0x47ede0`. Clear, `0x467080` reads the packed GUID, `0x4670a0` parses with
`0x47f050`, and `0x4670a5` compares the GUID with `0xc4da98`/`0xc4da9c`; equal returns. `0x4670bf`
calls `0x464530`; the object's own GUID is compared with `0xc4da98` again for the third argument
of `0x5ff030` at `0x467110`, then slot 2 for a reactivated object. `0x465ea0` is `0x642ed0` and
`0x47ede0`.

## Near objects

A near-objects block is a u32 count and that many packed GUIDs. In each pass, every listed GUID of
the pass's kind except the active player's is looked up, which reactivates a released object; a
reactivated one gets slot 2 and `0x464700`.

Addresses: `0x467230`, `0x467257`, `0x467278`, `0x467283`, `0x4672c8`, `0x4644f0`, `0x464530`
Confidence: verified
Proof: one
Test: none

`0x467257` reads the count; each GUID (`0x467278`) is compared with `0x468550` (`0x467283`), then
tested by the top-nibble test against the pass flag; a match calls `0x4644f0` (`0x4672c8`). `0x4644f0`
calls `0x464530` and, when it reactivated the object, slot 2 and `0x464700(guid)`. The function
returns the number it passed on, which the caller drops.

## Compressed update

`SMSG_COMPRESSED_UPDATE_OBJECT` is a u32 uncompressed size and a zlib stream filling the rest of
the packet. The client inflates it into a buffer of that size, on the stack below 900,000 bytes,
else on the heap, and runs the update handler on the result as opcode `0xa9`, with the
inflated length as the store's size. The inflate's return value is not checked.

Addresses: `0x4672f0`, `0x467302`, `0x467314`, `0x46731c`, `0x46732c`, `0x467346`, `0x467361`, `0x46739f`, `0x4673b6`, `0x660740`
Confidence: verified
Proof: one
Test: none

`0x467302` reads the size; `0x467314` takes a pointer to the remaining `[+0x10] − [+0x14]` bytes.
`cmp ebx,0xdbba0` (`0x46731c`) chooses `_alloca` (`0x46732c`) or `SMemAlloc` (`0x467346`).
`0x467361` calls `0x660740(out, &length, in, inLength)` and ignores `eax`; `0x46739f` puts
`length` bytes into a fresh `WDataStore`, and `0x4673b6` calls `0x4651a0` with `edx` `0xa9`.

## Destroy object

`SMSG_DESTROY_OBJECT` is a full u64 GUID. The client releases the object (`manager`), and when the
release left it in the released table, destroys it at once: it unlinks it from that table and
its type's reuse list, runs its vtable slot 0 with 0, and frees its memory, the active player's
from the heap and any other's into its type's pool.

Addresses: `0x4674a0`, `0x4674c2`, `0x4674cf`, `0x467521`, `0x467551`, `0x4675a8`, `0x467609`, `0x46767c`, `0x467628`, `0x4190b0`
Confidence: verified
Proof: one
Test: none

`0x4674c2` reads 8 bytes with `0x4190b0`; `0x4674cf` calls `0x464920`. The walk from `0x4674e9`
searches the released table at the manager's `+0x44`/`+0x4c` (`0x467521`). A match is unlinked at
`+0x1c`, `+0x24` (`0x467551`) and `+0x38` (`0x4675a8`). The type mask minus 1 indexes the byte
table `0x46767c`, whose two used entries both `call [vtable]` with 0. `0x467628` frees through
`SMemFree` for the active player's GUID, else through the type's pool, as `0x466e00` does.
