# lua: marking

The marking half of the collector, `lgc.c` of the upstream source, which the client keeps as Lua
5.0 has it but for the context word a nil it writes takes. `mark` (`0x6f73e0`) marks from the roots, propagates through a gray list, clears the
weak tables, separates the userdata to finalize, marks them and propagates again, and clears the
weak tables a second time. Its state is a `GCState` on the stack: the gray list at `+0`, the tables
with weak keys at `+4`, weak values at `+8`, both at `+0xc`, and the global state at `+0x10`. An
object's mark byte is at `+5`: bit 0 marked, bit 4 fixed, and in a table bit 1 weak keys and bit 2
weak values. A table holds its flags at `+6`, its node count's log at `+7`, its metatable at `+8`,
its array at `+0xc`, its nodes (40 bytes: key, value, next) at `+0x10`, its gray link at `+0x18`
and its array size at `+0x1c`. The names are upstream's.

Roots: `0x6f73e0`

## Traversing a table

`traversetable(st, h)` marks the metatable unless bit 0 or 4 of its byte is set. Unless the
metatable's flag bit 3 marks `__mode` absent, it asks `luaT_gettm`; a string mode containing `k`
makes the keys weak and `v` the values, either rewriting bits 1 and 2 of the table's byte and
pushing the table on the list for keys, values or both. Then, unless the values are weak,
the array slots from the last; and each node from the last whose value is not nil: its key unless
the keys are weak, then its value unless the values are weak. Only a collectable (tag 4 or more),
unmarked slot is marked; a node with a nil value keeps its key.

Addresses: `0x6f7590`, `0x6f75a6`, `0x6f75b1`, `0x6f75b9`, `0x6f75d2`, `0x6f75e1`, `0x6f75ef`, `0x6f7608`, `0x6f762e`, `0x6f7634`, `0x6f765a`, `0x6f765c`, `0x6f767b`, `0x6f76be`, `0x6f76c0`, `0x6f76ce`, `0x6f76dd`, `0x6f76ec`, `0x6f74a0`, `0x6f7ba0`, `0x6f7510`
Confidence: verified
Proof: one
Test: `lua::traversetable_whole`

`0x6f75a6` tests the metatable's byte with 0x11 and `0x6f75b1` calls `reallymarkobject` (`0x6f74a0`). `0x6f75b9`
tests flag bit 3; `0x6f75d2` calls `luaT_gettm(mt, 3, [g+0x8c])`; `0x6f75e1` keeps a string; `0x6f75ef` and `0x6f7608`
call `_strchr` with `k` and `v`. `0x6f762e` keeps the byte's other bits (`and al,0xf9`) under `(v << 2) | (k << 1)`;
`0x6f7634` to `0x6f765a` pick `st+0xc`, `st+4` or `st+8` and link the table at its head. `0x6f765c` skips the array
for weak values; `0x6f767b` is the collectable test. In the node loop `0x6f76be` passes a nil value by, and
`0x6f76c0` and `0x6f76ce` (key), `0x6f76dd` and `0x6f76ec` (value) test the tag, the mark and the weak flag. The gray
walk `0x6f7510` reaches it for a table, tag 5. The test runs the function whole with `reallymarkobject` and
`luaT_gettm` stubbed, recording the objects marked in order, over random arrays, nodes, mark bytes, metatable
flags and modes.

## Traversing a thread

`traversestack(st, L1)` marks the thread's globals table, finds `lim`, the stack top or the highest
top of a Lua call record (one without the C flag) above it, marks every collectable unmarked slot
from the stack's base below the top, sets every slot from the top up to `lim` to nil with the
current context word, and calls `checkstacksizes(L1, lim)`. That halves the call records,
`luaD_reallocCI(L1, size_ci / 2)`, when four times those in use is below `size_ci` and `size_ci` is
above 16; then the stack, `luaD_reallocstack(L1, stacksize / 2)`, when four times the slots below
`lim` is below `stacksize` and `stacksize` is above 90. Each happens at most once a collection.

Addresses: `0x6f7860`, `0x6f7869`, `0x6f787b`, `0x6f7880`, `0x6f7890`, `0x6f7899`, `0x6f78a4`, `0x6f78b0`, `0x6f78d4`, `0x6f78da`, `0x6f78ed`, `0x6f7900`, `0x6f7911`, `0x6f7925`, `0x6f7929`, `0x6f7936`, `0x6f7943`, `0x6f7949`, `0x6f794d`, `0x6f7952`, `0x6f795e`, `0x6f5ef0`, `0x6f5e20`, `0xceeac0`
Confidence: verified
Proof: one
Test: `lua::checkstacksizes_whole`

`0x6f7869` and `0x6f787b` mark `[L1+0x40]`. `0x6f7880` starts `lim` at `[L1+8]`; the loop `0x6f7890` to `0x6f78a4`
walks the 24-byte records from `[L1+0x28]` to `[L1+0x14]`, skipping flag bit 0 and raising `lim` to `[ci+4]` (`0x6f7899`).
`0x6f78b0` marks from `[L1+0x1c]` below the top; `0x6f78d4` and `0x6f78da` store tag 0 and `[0xceeac0]`. In
`checkstacksizes` the records in use are `(ci − base_ci) / 24` by the multiply `0x2aaaaaab` (`0x6f7911`), times 4 against
the 16-bit `[L+0x2c]` (`0x6f7925`, signed `jge`), and `0x6f7929` takes a size above 16; `0x6f7936` calls with the size
shifted right once. The slots are `(lim − stack) >> 4` (`0x6f7943`), times 4 against `[L+0x20]` (`0x6f7949`); `0x6f794d`
takes a size above 0x5a, `0x6f7952` halves it signed, and `0x6f795e` jumps. The test runs `checkstacksizes` whole with
both resizes stubbed, over record counts, sizes and slot offsets around each bound.

## Clearing weak keys

`cleartablekeys(list)` walks a list of tables through `+0x18`, each one's nodes from the last; a
node whose key `valismarked` refuses is removed by `removekey`: its value made nil with the current
context word, a collectable key's tag set to −1 (`LUA_TNONE`), the node kept. `valismarked(o)`
first marks a string, so every string key of a weak-keyed table survives, its value nil or not;
then it answers 1 for a tag below 4, else the mark bit. `mark` clears the values of the both-weak
and value-weak lists, then, after the second propagation, the keys of the first both-weak list and
of the key-weak list, the values of the value-weak list, and both of the second both-weak list.

Addresses: `0x6f7970`, `0x6f7980`, `0x6f7997`, `0x6f79a1`, `0x6f79ac`, `0x6f79b6`, `0x6f79f0`, `0x6f79f8`, `0x6f79fd`, `0x6f7a0a`, `0x6f79d0`, `0x6f79d7`, `0x6f79e4`, `0x6f73e0`, `0x6f7414`, `0x6f741c`, `0x6f742c`, `0x6f7443`, `0x6f744b`, `0x6f7453`, `0x6f745b`, `0x6f7463`, `0x6f7a20`
Confidence: verified
Proof: one
Test: `lua::cleartablekeys_whole`, `lua::valismarked_whole`

`0x6f7980` to `0x6f7997` count `1 << [h+7]` nodes of 0x28 bytes from the last; `0x6f79a1` and `0x6f79ac` call the two
helpers; `0x6f79b6` follows `+0x18`. `valismarked` tests tag 4 (`0x6f79f8`), sets bit 0 (`0x6f79fd`, `or [eax+5],dl`) and
tests it (`0x6f7a0a`). `removekey` stores tag 0 at `+0x10` and `[0xceeac0]` at `+0x14` (`0x6f79d7`) and −1 in a key of tag
4 or more (`0x6f79e4`). In `mark` the clears are `0x6f7414` and `0x6f741c` (values, `st+0xc` and `st+8`), then after
`luaC_separateudata` (`0x6f742c`) `0x6f7443` (the saved `st+0xc`), `0x6f744b` (`st+4`), `0x6f7453` (`st+8`, values),
`0x6f745b` and `0x6f7463` (`st+0xc`); `0x6f7a20` is `cleartablevalues`. The tests run `valismarked` whole for every tag
and mark byte, and `cleartablekeys` whole over chains of tables of random keys, values and bytes.
