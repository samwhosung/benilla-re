# dbcache: callbacks

The callbacks a cache keeps for the callers waiting on a record: how a lookup keeps one, the list link
that orders them, the order a response runs them in, cancelling one, and the item cache methods that
would run them and have no caller. The lookups and the response handlers are in `dbcache/overview`; the
item cache, at `0xc0e2a0`, is the one read here, and the other twelve caches share its template. A
callback is a `DBCACHECALLBACK` of 0x20 bytes in the list at `+0x1f4` of an item node.

Roots: `0x55ba30`, `0x55bdb0`, `0x55c0c0`

## Callback list links

`0x56d0e0(node, mode, reference)` links a callback into the list in `ecx`, first unlinking the node if it is linked. Mode 1 links it directly after `reference`, any other mode directly before it, and a `reference` of 0 stands for the list's terminator: mode 1 makes the node the head and mode 2 the tail. A link is two dwords at offset 0 of the node: the previous link's address, then the next node, which at the end of the list is the terminator's address with bit 0 set. The terminator is the list object's own link at `+4`; the list object of an item node is at `+0x1f4` and starts empty, the terminator pointing at itself.

Addresses: `0x56d0e0`, `0x56d0d0`, `0x56d0f2`, `0x56d127`, `0x56d134`, `0x56d137`, `0x56d14d`, `0x55b250`, `0x55b2aa`, `0x55b2b5`, `0x55b2bb`
Confidence: verified
Proof: one
Test: `dbcache::callback_list_link_difftest`

`0x56d0f2 mov edi,[eax]; test; je` skips the unlink for an unlinked node; the unlink ends at `0x56d120`, which zeroes both dwords. `0x56d12e` replaces a null reference with `ecx+4`, the terminator, and `0x56d135 je 0x56d14d` takes mode 1. Mode 1 (`0x56d14d` to `0x56d188`) stores the reference as the link's previous, copies the reference's next, points the old next's previous at the link (the terminator when the next is tagged, `0x56d164`) and the reference's next at the node. The other modes (`0x56d137` to `0x56d14a`) copy the reference's previous link and its next, point that previous link's next at the node and the reference's previous at the link. `0x55b2aa` to `0x55b2bb` build the empty list. The test runs the function whole but for a null node (`0x56d0ef`) and a negative link offset (`0x56d176`), on lists of link offset 0, in both modes, with the terminator, a head, a middle and a tail as reference, linking new nodes and moving linked ones, and compares every dword.

## Item node allocation

`0x55b250(bucket, extra, flags)` is slot 1 of the item cache's vtable, which the hash insert `0x56d610` calls for a key with no node. It allocates `extra + 0x204` zeroed bytes (flags OR 8), clears the links, the record at `+0x18` and the valid and dispatching bytes (`+0x1f0`, `+0x201`), builds the empty callback list at `+0x1f4`, links the node's hash link (`node+4`) at the head of the bucket's list and returns the node.

Addresses: `0x55b250`, `0x80915c`, `0x56d610`, `0x56d72e`, `0x56d674`, `0x55b25c`, `0x55b267`, `0x55b26e`, `0x55b27b`, `0x55b2aa`, `0x55b2c8`, `0x55b2cd`, `0x55b2df`, `0x55b30f`, `0x55b332`
Confidence: verified
Proof: one
Test: none

The cache's vtable `0x80915c` holds `0x55b250` in slot 1. `0x56d72e call [eax+4]` passes the bucket's list object, `[ebp+0xc]` and `[ebp+0x10]`. `0x55b25c or eax,8` and `0x55b267 add ecx,0x204` make the allocation's flags and size (`0x55b26e` calls `0x6462e0`, tagged `DBCACHEHASH`). `0x55b27b` to `0x55b2a4` clear the two links and `+0x1b0`; `0x55b2aa` to `0x55b2bb` build the empty list; `0x55b2c8 rep stos` of 0x75 dwords clears the record; `0x55b2cd` to `0x55b2d9` clear the three bytes. `0x55b2df` adds the bucket's link offset, 4 (`0x56d674`), to the node; `0x55b30f` to `0x55b332` make the terminator `bucket+4` the link's previous, the terminator's next the link's next, point the old head's previous (or, when the list was empty, the terminator's) at the link and store the node as the terminator's next. The allocation-failure arm (`0x55b2e8`) is not read.

## Callback order

A lookup appends the callback it keeps to the end of its key's list, and a response runs the list from the head: callbacks run in the order they were kept. `0x55ba30` allocates a 0x20-byte `DBCACHECALLBACK`, links it with `0x56d0e0(node, 2, 0)` and fills in the function (`+8`), the caller's id (`+0x10`) and the data (`+0x18`). For a key whose node exists with no record, a set dedupe flag keeps no second callback with the same function and data. A response calls `function(key, &id, data, status)` with status 1 after decoding and 0 for a refusal, then frees the list. All 26 link sites of the thirteen caches use mode 2 and no reference.

Addresses: `0x55ba30`, `0x55bac8`, `0x55baee`, `0x55baf9`, `0x55bbb3`, `0x55bbd8`, `0x55bbe0`, `0x55bc0b`, `0x55bc16`, `0x55bdb0`, `0x55be81`, `0x55bea8`, `0x55bef6`, `0x55bf12`, `0x55bf30`, `0x55bf39`, `0x55bf40`, `0x56d0e0`, `0x56d0d0`, `0x554c30`, `0x555140`, `0x555160`
Confidence: verified
Proof: one
Test: `dbcache::callback_order_difftest`

`0x55bac8` pushes the size 0x20 for the `DBCACHECALLBACK` allocation (flags 8, zeroed); `0x56d0d0` clears its link. `0x55baee` and `0x55bc0b` push 0, 2 and the node before `0x56d0e0` on `node+0x1f4` (`0x55baf3`, `0x55bc10`). The dedupe scan (`0x55bbb3` to `0x55bbeb`) compares `[cb+8]` and `[cb+0x18]` only when `[ebp+0x18]` is set. `0x55bf12` loads `[node+0x1fc]`, the head; `0x55bf30` reads `[cb+4]` before `0x55bf39 call [eax+8]`, with `push 1` (the status) and the data pushed, `ecx` the key and `edx` the id at `cb+0x10`, the callee returning `ret 8`; the refusal walk is the same with `push 0` (`0x55be81` to `0x55bead`); `0x55bf40` to `0x55bf6c` free the list. Each of the 26 `call 0x56d0e0` sites is preceded by `push 0; push 2; push esi` (a census over the listing). The other twelve caches' response walks were not read. The test builds the cache with `0x554c30`, stubs the allocator, the send, the console and the decoder `0x7c9640`, and runs `0x55ba30` and `0x55bdb0` over single and batched answers; it does not run `0x55bae9` (allocation failure) and `0x55bebd` to `0x55bec8`.

## Cancelling a callback

`0x55c0c0(key, function, data)` removes from the key's pending list every callback with that function and data, not the first only, and frees each. It does nothing for a key with no node. While the key's list is being run, marked by the node's byte `+0x201`, it removes nothing and prints `DBCache::CancelCallback ignored for id %016I64X.` on the console.

Addresses: `0x55c0c0`, `0x55c117`, `0x55c11d`, `0x55c121`, `0x55c12b`, `0x55c15c`, `0x55c167`, `0x55c1a3`, `0x55c1aa`
Confidence: verified
Proof: one
Test: `dbcache::callback_order_difftest`

`0x55c11d` tests `[node+0x201]`; set goes to the message (`0x55c126` pushes the format at `0x855b3c`, `0x55c12b` calls `0x63cd50`) and returns. Otherwise the loop at `0x55c150` compares `[cb+8]` with the function (`0x55c15c`) and `[cb+0x18]` with the data (`0x55c167`), unlinks and frees a match (`0x55c1a3`) and continues with the next link read at `0x55c15f` (`0x55c1a8`, `0x55c1aa`) until the terminator. The test cancels pending callbacks, empties lists, and cancels while the byte is set; it does not run the relink at `0x55c179` to `0x55c192`, which follows an unlink that already cleared the link.

## Unreached item cache methods

Three methods of the item cache have no caller. `0x55bc70(record, key)` copies a record into the key's node, creating it, sets the valid byte and runs the key's callbacks with status 1. `0x55bf90(key)` runs them with status 0 and destroys the node through the cache's slot 0. `0x55c920(key)` destroys a node that has a record, and runs a pending node's callbacks with status 0 before `0x56caa0` removes it. The receive `0x55bdb0` (called by the handlers `0x555140` and `0x555160`) holds the only item-cache callback sites that run, `0x55bea8` with status 0 and `0x55bf39` with 1.

Addresses: `0x55bc70`, `0x55bd22`, `0x55bd2a`, `0x55bf90`, `0x55c01e`, `0x55c026`, `0x55c920`, `0x55ca2b`, `0x55ca36`, `0x56caa0`, `0x55bdb0`, `0x55514e`, `0x55516e`, `0x55bea8`, `0x55bf39`
Confidence: verified
Proof: one
Test: none

The census scans the 4,775,986-byte image for the four addresses `0x55bc70`, `0x55bf90`, `0x55c920` and `0x56caa0`: every `e8` and `e9` whose displacement lands on one, every `0f 80` to `0f 8f` near jump that does, and every little-endian dword equal to one at any alignment. It finds nothing for any; `cargo xtask census --why` also finds no reference. The control, the same scans, finds `0x55c0c0` (10 calls), `0x55bdb0` (the calls at `0x55514e` and `0x55516e`) and `0x4c2ac0` (three stored dwords), and `--why` walks `0x55bdb0` back to `WinMain`. The pushes before the unreached calls are `0x55bd22 push 1`, `0x55c01e push 0` and `0x55ca2b push 0`; in `0x55b040` to `0x55cb3f` the calls through `[reg+8]` are these three and the two live ones. The same methods of the other twelve caches were not censused.
