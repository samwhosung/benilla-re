# lua: objects and tables

The value types of the embedded Lua 5.0 core, from `lobject.c`, `lstring.c`, `lfunc.c` and
`ltable.c` of the upstream source, between `0x6f5810` and `0x6fad80`. The allocating functions
pass the path of their own file (`lstring.c`, `lfunc.c`, `ltable.c`) and a line to the allocator
`luaM_irealloc` (`0x6fc980`), which fixes which upstream file each belongs to. Objects start with
a link, a type tag at `+4` and the collector's mark byte at `+5`; a string is `+8` hash, `+0xc`
length and the bytes from `+0x10`; a table has its metatable at `+8`, array at `+0xc`, node
vector at `+0x10`, a log2 node count at `+7` and an array size at `+0x1c`, with 40-byte nodes
(key, value, next). The line numbers the allocating functions pass are those of the Lua 5.0 source
for `lstring.c` and `lfunc.c` and, in `ltable.c`, for the two vector allocators (5.0.1 and 5.0.2 have
the same lines, so they do not tell the releases apart). The names are
upstream's.

Roots: `0x6f5810`, `0x6f5830`, `0x6f58b0`, `0x6f5900`, `0x6f5990`, `0x6f5c20`, `0x6f5c40`, `0x6f9c30`, `0x6f9c50`, `0x6f9d00`, `0x6f9e30`, `0x6f9e70`, `0x6f9eb0`, `0x6f9f10`, `0x6f9f60`, `0x6f9fe0`, `0x6fa050`, `0x6fa110`, `0x6fa150`, `0x6fa420`, `0x6fa4f0`, `0x6fa690`, `0x6fa700`, `0x6fa760`, `0x6fa7a0`, `0x6fa840`, `0x6fad80`

## Basic object functions

`luaO_int2fb` and `luaO_log2` are the integer helpers: a floating-byte code of an integer and the
base-2 logarithm by table. `luaO_rawequalObj` compares two values by tag and then by number
or by address. `luaO_str2d` reads a number with `strtod` and accepts trailing spaces only.
`luaO_chunkid` builds the short source name used in messages, from `=name`, `@file` or a
string chunk (`[string "first line..."]`). `luaO_pushvfstring` joins the literal text and the converted arguments of a `%s`, `%c`, `%d`, `%f`
or `%%` format by `luaV_concat`; `luaO_pushfstring` calls it on its own arguments, and `pushstr`
pushes one string. The client's `pushstr` pushes `(null)` for a null pointer (`0x6f5bc8`); upstream's
(`lobject.c`, lines 104 to 107) has no such check.

| function | name | decided by |
|---|---|---|
| `0x6f5810` | `luaO_int2fb` | loop while `x > 7` (`0x6f5812`) halving with rounding up (`0x6f5818`), result is `e` shifted left by 3, or `x` (`0x6f5820`) |
| `0x6f5830` | `luaO_log2` | thresholds `0x10000` (`0x6f5834`), `0x1000000` (`0x6f583f`), `0x100` (`0x6f586b`), adds 24, 16, 8; -1 (`0x6f589a`) |
| `0x6f58b0` | `luaO_rawequalObj` | tags differ returns 0 (`0x6f58b3`), jump table on tags 0..3 (`0x6f58c0`), `fcompp` (`0x6f58cd`), payload subtract (`0x6f58e3`) |
| `0x6f5900` | `luaO_str2d` | `_strtod` (`0x6f5911`), `__isctype` 8 over the tail (`0x6f593e`), end test (`0x6f5964`) |
| `0x6f5990` | `luaO_pushvfstring` | empty string first (`0x6f59aa`), `strchr` for `%` (`0x6f59b2`), literal text by `luaS_newlstr` (`0x6f59f9`), letter switch (`0x6f5a40`), `fild` for `%d` (`0x6f5a97`), `fld qword` for `%f` (`0x6f5abe`), `luaV_concat` (`0x6f5b31`) |
| `0x6f5bc0` | `pushstr` | the text `(null)` (`0x6f5bc8`), `luaS_newlstr` (`0x6f5bee`), growth check (`0x6f5c0d`) |
| `0x6f5c20` | `luaO_pushfstring` | calls `luaO_pushvfstring` with the address of its own arguments (`0x6f5c2d`) |
| `0x6f5c40` | `luaO_chunkid` | `=` (`0x6f5c49`), `@` (`0x6f5c6b`), else `[string "` (`0x6f5ceb`), first line by `strcspn` (`0x6f5cd7`), `..."]` (`0x6f5d5d`) |

Addresses: `0x6f5bc8`, `0x6f5810`, `0x6f5812`, `0x6f5818`, `0x6f5820`, `0x6f5830`, `0x6f5834`, `0x6f583f`, `0x6f586b`, `0x6f589a`, `0x6f58b0`, `0x6f58b3`, `0x6f58c0`, `0x6f58cd`, `0x6f58e3`, `0x6f5900`, `0x6f5911`, `0x6f593e`, `0x6f5964`, `0x6f5990`, `0x6f59aa`, `0x6f59b2`, `0x6f59f9`, `0x6f5a40`, `0x6f5a97`, `0x6f5abe`, `0x6f5b31`, `0x6f5bc0`, `0x6f5bee`, `0x6f5c0d`, `0x6f5c20`, `0x6f5c2d`, `0x6f5c40`, `0x6f5c49`, `0x6f5c6b`, `0x6f5ceb`, `0x6f5cd7`, `0x6f5d5d`
Confidence: verified
Proof: one
Test: none

`luaO_int2fb` rounds up: `x = (x+1) >> 1` while the value exceeds 7, the exponent counting the
shifts. `luaO_log2` indexes a 256-byte table by the highest nonzero byte, adding 24, 16, 8 or 0.
`luaO_rawequalObj` treats nil as equal to nil, numbers by value (NaN compares unequal through
the `test ah,0x44` parity test), every other tag by payload address. For a `=` source,
`luaO_chunkid` copies the rest, cut to the buffer; for `@` it copies the file name, prefixed by
`...` when it does not fit; otherwise it takes the text up to the first newline, cut to fit.

## Strings

A string is interned: `luaS_newlstr` hashes a block (the length seeds `h`, then
`h ^= (h<<5) + (h>>2) + byte` for every `step`-th byte from the end, `step = (len>>5) + 1`), looks
in the bucket `hash & (size-1)` for equal length and bytes, and calls `newlstr` otherwise. `newlstr`
allocates `len + 0x11` bytes, links it into the bucket and doubles the table through
`luaS_resize` when the count passes the size. `luaS_newudata` allocates a userdata block.
`luaS_freeall` frees the string table.

| function | name | decided by |
|---|---|---|
| `0x6f9c30` | `luaS_freeall` | passes `lstring.c` and the vector size `<<2` to `luaM_irealloc` (`0x6f9c46`) |
| `0x6f9c50` | `luaS_resize` | allocates `size*4` (`0x6f9c76`), moves each chain by `hash & (new-1)`, frees the old vector (`0x6f9ce5`) |
| `0x6f9d00` | `luaS_newlstr` | step `len>>5` (`0x6f9d0f`), `h<<5` and `h>>2` mixing (`0x6f9d2a`, `0x6f9d31`), `newlstr` (`0x6f9d76`) |
| `0x6f9d90` | `newlstr` | allocates `len+0x11` (`0x6f9dae`), grows at `nuse > size` below `0x3ffffffe` (`0x6f9e0f`, `0x6f9e19`) |
| `0x6f9e30` | `luaS_newudata` | allocates `size+0x10` (`0x6f9e45`), tag 7, metatable `[G+0x48]`, linked into `[G+0x14]` |

Addresses: `0x6f9c30`, `0x6f9c46`, `0x6f9c50`, `0x6f9c76`, `0x6f9ce5`, `0x6f9d00`, `0x6f9d0f`, `0x6f9d2a`, `0x6f9d31`, `0x6f9d76`, `0x6f9d90`, `0x6f9dae`, `0x6f9e0f`, `0x6f9e19`, `0x6f9e30`, `0x6f9e45`
Confidence: verified
Proof: one
Test: none

The hash loop of `luaS_newlstr` is the one in the upstream 5.0 source: `l1 = len`, stepping down by
`(len>>5)+1`, so a string under 32 bytes hashes every byte. Equal hashes still compare length and
bytes. `newlstr` stores the hash at `+8` and the length at
`+0xc`, the NUL at `+0x10+len`.

## Closures and prototypes

A C closure is `(n+1)*16` bytes with the function at `+0xc`, its upvalue count at `+7` and a flag at
`+6`; a Lua closure is `n*4+0x24` bytes with its prototype at `+0xc` and the environment copied
to `+0x10`. `luaF_findupval` keeps the open upvalues of a state in a list sorted by stack level,
`luaF_close` copies the open ones above a level into themselves, `luaF_newproto` makes a zeroed
prototype, `luaF_freeproto` and `luaF_freeclosure` free them, and `luaF_getlocalname` finds the
nth local variable name alive at an instruction.

| function | name | decided by |
|---|---|---|
| `0x6f9e70` | `luaF_newCclosure` | allocates `(n+1)*16` (`0x6f9e89`), `luaC_link` (`0x6f9e96`) |
| `0x6f9eb0` | `luaF_newLclosure` | allocates `n*4+0x24` (`0x6f9ecd`), environment copied to `+0x10`, `luaC_link` (`0x6f9eda`) |
| `0x6f9f10` | `luaF_findupval` | list head `[L+0x50]`, `jb`/`je` on the level (`0x6f9f25`, `0x6f9f27`), allocates 0x20 bytes (`0x6f9f3e`), tag 10 (`0x6f9f43`) |
| `0x6f9f60` | `luaF_close` | level test (`0x6f9f79`), value copied into the upvalue at `+0x10` (`0x6f9f87`), pointer set (`0x6f9fb6`), `luaC_link` with type 10 (`0x6f9fc0`) |
| `0x6f9fe0` | `luaF_newproto` | allocates 0x48 bytes (`0x6f9ff3`), `luaC_link` (`0x6fa000`) |
| `0x6fa050` | `luaF_freeproto` | seven `luaM_irealloc` calls (`0x6fa069`..`0x6fa102`) of code, nested protos, constants, line info, locals, upvalue names and the record |
| `0x6fa110` | `luaF_freeclosure` | C flag at `+6` selects `(n+1)*16` (`0x6fa129`) or `n*4+0x24` (`0x6fa140`) |
| `0x6fa150` | `luaF_getlocalname` | 12-byte records `[name, startpc, endpc]` (`0x6fa175`), start and end tests (`0x6fa168`, `0x6fa16d`), name at `+0x10` (`0x6fa18d`) |

Addresses: `0x6f9e70`, `0x6f9e89`, `0x6f9e96`, `0x6f9eb0`, `0x6f9ecd`, `0x6f9eda`, `0x6f9f10`, `0x6f9f25`, `0x6f9f27`, `0x6f9f3e`, `0x6f9f43`, `0x6f9f60`, `0x6f9f79`, `0x6f9f87`, `0x6f9fb6`, `0x6f9fc0`, `0x6f9fe0`, `0x6f9ff3`, `0x6fa000`, `0x6fa050`, `0x6fa069`, `0x6fa102`, `0x6fa110`, `0x6fa129`, `0x6fa140`, `0x6fa150`, `0x6fa175`, `0x6fa168`, `0x6fa16d`, `0x6fa18d`, `0x6fbb54`, `0x6fbbf2`, `0x6fc4f5`
Confidence: verified
Proof: one
Test: none

The open-upvalue list is sorted by decreasing level: `luaF_findupval` walks it while the level is
at or above the target, returning an equal one. `luaF_close` unlinks each open upvalue at or above the
level, copies the value into the upvalue and points its value field at that copy, and links the upvalue
as a collectable object of type 10. `luaF_getlocalname` is called at `0x6fbb54`, `0x6fbbf2` and
`0x6fc4f5`.

## Table creation and growth

`luaH_new` makes a table: a 32-byte record with the default metatable, no array and no node
vector, then sized by `setarrayvector` and `setnodevector`. `setnodevector` raises `table
overflow` above 2^24 nodes, uses the shared dummy node for a log size of 0, and otherwise allocates 40
bytes a node, all nil. `rehash` counts the live keys (`numuse`, which fills a count per power
of two) and picks the sizes with `computesizes`: the largest array part more than half full.
`resize` moves the entries into the new vectors and `luaH_free` frees a table.

| function | name | decided by |
|---|---|---|
| `0x6fa4f0` | `luaH_new` | allocates 0x20 bytes (`0x6fa50a`), `luaC_link` type 5 (`0x6fa517`), flags 0xff (`0x6fa52c`), `setarrayvector` (`0x6fa53c`), `setnodevector` (`0x6fa549`) |
| `0x6fa560` | `setarrayvector` | reallocates `size*16` in `ltable.c` (`0x6fa584`), new slots set to tag 0 |
| `0x6fa5d0` | `setnodevector` | limit 24 (`0x6fa5e2`), `table overflow` (`0x6fa5e9`), dummy node `[G+0x58]` for size 0 (`0x6fa601`), 40 bytes a node (`0x6fa620`) |
| `0x6fa690` | `luaH_free` | three `luaM_irealloc` calls in `ltable.c` (`0x6fa6bc`, `0x6fa6d9`, `0x6fa6f0`), the first only for a nonzero node size |
| `0x6fa9c0` | `rehash` | `numuse` (`0x6fa9d5`), `luaO_log2` (`0x6fa9dd`), `resize` (`0x6fa9ec`) |
| `0x6faa00` | `numuse` | slice loop to 24 (`0x6faa67`), `arrayindex` (`0x6faab6`), `luaO_log2` (`0x6faac2`), `computesizes` (`0x6faae7`) |
| `0x6fab00` | `computesizes` | array size a power of two, hash count `total - na`; n of -1 gives 0 (`0x6fab69`) |
| `0x6fab90` | `resize` | `setarrayvector` (`0x6fac1f`), `setnodevector` (`0x6fac2f`), `luaH_setnum` (`0x6fac63`), `luaH_set` (`0x6facfd`), old vector freed (`0x6fad5e`) |

Addresses: `0x6fa4f0`, `0x6fa50a`, `0x6fa517`, `0x6fa52c`, `0x6fa53c`, `0x6fa549`, `0x6fa560`, `0x6fa584`, `0x6fa5d0`, `0x6fa5e2`, `0x6fa5e9`, `0x6fa601`, `0x6fa620`, `0x6fa690`, `0x6fa6bc`, `0x6fa6d9`, `0x6fa6f0`, `0x6fa9c0`, `0x6fa9d5`, `0x6fa9dd`, `0x6fa9ec`, `0x6faa00`, `0x6faa67`, `0x6faab6`, `0x6faac2`, `0x6faae7`, `0x6fab00`, `0x6fab69`, `0x6fab90`, `0x6fac1f`, `0x6fac2f`, `0x6fac63`, `0x6facfd`, `0x6fad5e`, `0x6fabe1`, `0x6fa98c`
Confidence: verified
Proof: one
Test: none

`resize` keeps the old node vector, or copies the dummy node into a temporary when the old size
is 0 (`rep movs` at `0x6fabe1`), grows the array first (`0x6fac1f`) and shrinks it after the new
node vector exists, reinserting the vanishing array slice with `luaH_setnum` and then the old nodes
with `luaH_set`. `newkey` calls `rehash` when no free node is left (`0x6fa98c`). The array part is
the largest `2^n` for which more than `2^(n-1)` of the keys `1` to `2^n` are in use, as in
upstream 5.0.

## Table access

A node vector is a chained scatter table with Brent's variation. `luaH_mainposition` hashes a
key to its node: a boolean by value, a light userdata or any other non-string by address modulo
`size-1` made odd, a number through `hashnum`, a string by its hash, masked. `luaH_getnum`,
`luaH_get` and `luaH_getany` read; `luaH_set`, `luaH_setnum` and `newkey` write, raising `table
index is nil` and `table index is NaN`. `luaH_index` and `arrayindex` give a key's traversal
position.

| function | name | decided by |
|---|---|---|
| `0x6fa1a0` | `luaH_mainposition` | jump table on the tag minus 1 (`0x6fa1ac`): boolean `0x6fa1e5`, light userdata `0x6fa201`, number `0x6fa1b3`, string `0x6fa1c6`, other `0x6fa222` |
| `0x6fa260` | `hashnum` | `fadd` of 1.0 (`0x6fa267`), word sum (`0x6fa27e`), `div edi` by `size-1` made odd (`0x6fa28b`) |
| `0x6fa420` | `luaH_index` | nil gives -1 (`0x6fa428`), `arrayindex` (`0x6fa43e`), `luaH_get` (`0x6fa458`), the error ``invalid key for `next'`` (`0x6fa467`), division by 40 (`0x6fa485`) |
| `0x6fa4a0` | `arrayindex` | `__ftol` (`0x6fa4af`), `fcompp` with the key (`0x6fa4bf`), `k-1` below 2^24 (`0x6fa4d0`) |
| `0x6fa700` | `luaH_getnum` | array bound tests (`0x6fa706`, `0x6fa70e`), `hashnum` (`0x6fa72f`), number tag and equal value (`0x6fa736`, `0x6fa743`), next link (`0x6fa748`) |
| `0x6fa760` | `luaH_getstr` | main position by the string hash (`0x6fa771`), tag 4 and pointer equality (`0x6fa780`, `0x6fa785`), chain link (`0x6fa78a`) |
| `0x6fa7a0` | `luaH_get` | integer number keys to `luaH_getnum` (`0x6fa7e0`), strings to `luaH_getstr` (`0x6fa7b7`), others to `luaH_getany` (`0x6fa7ef`) |
| `0x6fa800` | `luaH_getany` | nil key skips (`0x6fa804`), `luaH_mainposition` (`0x6fa809`), `luaO_rawequalObj` (`0x6fa814`) |
| `0x6fa840` | `luaH_set` | `luaH_get` (`0x6fa851`), flags cleared (`0x6fa85b`), `table index is nil` (`0x6fa867`), `NaN` (`0x6fa87f`), `newkey` (`0x6fa892`) |
| `0x6fa8a0` | `newkey` | `luaH_mainposition` twice (`0x6fa8b8`, `0x6fa8ca`), 10-dword node copy (`0x6fa8f7`), free pointer down by 40 (`0x6fa95e`), `rehash` (`0x6fa98c`), `luaH_get` (`0x6fa995`) |
| `0x6fad80` | `luaH_setnum` | `luaH_getnum` (`0x6fad91`), `newkey` (`0x6fadba`) with a number built on the stack |

Addresses: `0x6fa1a0`, `0x6fa1ac`, `0x6fa1e5`, `0x6fa201`, `0x6fa1b3`, `0x6fa1c6`, `0x6fa222`, `0x6fa260`, `0x6fa267`, `0x6fa27e`, `0x6fa28b`, `0x6fa420`, `0x6fa428`, `0x6fa43e`, `0x6fa458`, `0x6fa467`, `0x6fa485`, `0x6fa4a0`, `0x6fa4af`, `0x6fa4bf`, `0x6fa4d0`, `0x6fa700`, `0x6fa706`, `0x6fa70e`, `0x6fa72f`, `0x6fa736`, `0x6fa743`, `0x6fa748`, `0x6fa760`, `0x6fa771`, `0x6fa780`, `0x6fa785`, `0x6fa78a`, `0x6fa7a0`, `0x6fa7e0`, `0x6fa7b7`, `0x6fa7ef`, `0x6fa800`, `0x6fa804`, `0x6fa809`, `0x6fa814`, `0x6fa840`, `0x6fa851`, `0x6fa85b`, `0x6fa867`, `0x6fa87f`, `0x6fa892`, `0x6fa8a0`, `0x6fa8b8`, `0x6fa8ca`, `0x6fa8f7`, `0x6fa95e`, `0x6fa98c`, `0x6fa995`, `0x6fad80`, `0x6fad91`, `0x6fadba`, `0x6fa48a`
Confidence: verified
Proof: one
Test: none

`hashnum` adds 1.0 before it sums the two 32-bit words of the double, as upstream does to make
`-0` and `0` hash alike. `arrayindex` accepts an integer from 1 to 2^24. `luaH_index` gives
`i - 1` for an array position and the node index plus the array size for a node, the index from
the entry address minus the vector base, divided by 40 by multiplying with `0xcccccccd`
(`0x6fa485`) and shifting (`0x6fa48a`). `newkey` moves the node found in the main position of a key
that is not its own to the free node and puts the new key in the main position.

## Allocation

The Lua core allocates through `luaM_irealloc`, which takes the state, the block, the old size, the
new size, a source path and a line, and is the one caller of `luaM_reallocPool`. It adds the new size less the old to the block
count at `+0x28` of the global state when there is a state, then reallocates through
`luaM_reallocPool` with the block and the new size; the path and the line are never read.
`luaM_growaux` grows a vector: it doubles the capacity, takes 4 below 4, and near the limit takes the
limit while 4 places remain, else raises the caller's message.

| function | name | decided by |
|---|---|---|
| `0x6fc980` | `luaM_irealloc` | no state skips the count (`0x6fc983`, `0x6fc989`), old size out and new size in (`0x6fc996`..`0x6fc99e`), `luaM_reallocPool` (`0x6fc9a4`), `ret 0x10` (`0x6fc9ab`) |
| `0x6fc900` | `luaM_growaux` | doubled, at least 4 (`0x6fc90e`..`0x6fc919`), half the limit (`0x6fc920`..`0x6fc92a`), the limit (`0x6fc92c`..`0x6fc936`), `luaG_runerror` (`0x6fc93f`), `luaM_irealloc` in `lmem.c` (`0x6fc962`), size stored (`0x6fc967`) |

Addresses: `0x6fc980`, `0x6fc983`, `0x6fc989`, `0x6fc996`, `0x6fc99e`, `0x6fc9a4`, `0x6fc9ab`, `0x6fc900`, `0x6fc90e`, `0x6fc919`, `0x6fc920`, `0x6fc92a`, `0x6fc92c`, `0x6fc936`, `0x6fc93f`, `0x6fc962`, `0x6fc967`, `0x6fc9a3`, `0x6fc954`, `0x6fc94e`, `0x6fc951`
Confidence: verified
Proof: one
Test: none

A census of `call` and `jmp` targets finds 56 sites calling `luaM_irealloc` and one calling
`luaM_reallocPool` (`0x6fc9a4`); the control is the same search finding `luaM_growaux`'s
callers in the compiler. `luaM_irealloc` reads only `[ebp+0x8]` and `[ebp+0xc]` of its four stack arguments and pushes the
new size alone before the call (`0x6fc9a3`), with the state in `ecx` and the block in `edx`.
`luaM_growaux` passes `lmem.c` with the line 62 (`0x6fc954`), where the call stands at line 54 of
the 5.0 source, and the sizes as count times element size (`0x6fc94e`, `0x6fc951`). Its callers in
the compiler pass the limits `0x3ffff` and `0x7ffffffd`.

## Unreached functions

Nothing live reaches this function, which belongs to the library the client links.

| functions |
|---|
| `luaS_newudata` `0x6f9e30` |

Addresses: `0x6f9e30`
Confidence: verified
Proof: one
Test: none

`cargo xtask census --all` marks each of these rows `unreached`: no `call` or `jmp` target, no stored dword and no fall-through reaches it from live code. The control is the same run listing `lua_gettop` (`0x6f3070`) as live, with 582 call sites in the image.
