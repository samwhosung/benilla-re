# lua: the C API

The embedded Lua 5.0 core's public functions, `lapi.c` of the upstream source, laid out in the
exe between `0x6f2ec0` and `0x6f47b0`. Each is a `__fastcall`: the state `L` is in `ecx` and the
first argument in `edx`, the rest on the stack. A Lua value is 16 bytes: a type tag (0 nil, 1
boolean, 2 light userdata, 3 number, 4 string, 5 table, 6 function, 7 userdata, 8 thread), the
client's context word at `+4` and the payload at `+8`. The state keeps the stack top in
`[L+8]`, its base in `[L+0xc]`, the global state in `[L+0x10]`, the current call record in
`[L+0x14]`, the last usable slot in `[L+0x18]` and the stack in `[L+0x1c]`. The function bodies
match upstream Lua 5.0 (`lua_version` answers `Lua 5.0`) except where the entry says so; the
names are upstream's. `luastate` in `ui` holds the functions the client changed.

Roots: `0x6f2ec0`, `0x6f3080`, `0x6f30d0`, `0x6f31a0`, `0x6f32b0`, `0x6f3350`, `0x6f3410`, `0x6f3480`, `0x6f3550`, `0x6f35e0`, `0x6f3660`, `0x6f36e0`, `0x6f3740`, `0x6f3770`, `0x6f3790`, `0x6f3810`, `0x6f3840`, `0x6f38c0`, `0x6f38f0`, `0x6f3a40`, `0x6f3b00`, `0x6f3bc0`, `0x6f3cf0`, `0x6f3d50`, `0x6f3e20`, `0x6f3ea0`, `0x6f3f60`, `0x6f4020`, `0x6f40d0`, `0x6f41a0`, `0x6f4370`, `0x6f43e0`, `0x6f43f0`, `0x6f4450`, `0x6f44e0`

## Stack moves

Seven functions move values on the stack. `luaA_pushobject` copies a value to the top and
makes room first, `lua_xmove` moves the top `n` values of one state onto another, `lua_settop`
sets the top (filling with nil), `lua_remove` and `lua_insert` shift the values above an index
down or up one slot, `lua_replace` pops the top into an index and `lua_pushvalue` pushes a copy
of an index. Each resolves its index by the same inlined walk: positive from the base, negative
from the top, `-10000` the registry, `-10001` the globals, below that an upvalue of the running C
function.

| function | name | decided by |
|---|---|---|
| `0x6f2ec0` | `luaA_pushobject` | copies 16 bytes to `[L+8]` (`0x6f2ecc`), calls `luaD_growstack` at `0x6f2f19`, adds 16 at `0x6f2f24` |
| `0x6f2f80` | `lua_xmove` | lowers the first top by `n*16` (`0x6f2f9c`), copies each to `[to+8]` and raises it (`0x6f3007`); `ret 4` |
| `0x6f3080` | `lua_settop` | nil tag 0 stores at `0x6f309a`, top from base at `0x6f30bb`, negative index adds `(idx+1)*16` at `0x6f30c5` |
| `0x6f30d0` | `lua_remove` | copy loop from `idx+16` to `idx` (`0x6f3140`), top lowered by 16 at `0x6f318b` |
| `0x6f31a0` | `lua_insert` | copy loop upward from the top (`0x6f3218`), top value stored at the index (`0x6f326b`) |
| `0x6f32b0` | `lua_replace` | copies `[top-16]` to the index slot, then `add [ecx+8],-16` (`0x6f3347`) |
| `0x6f3350` | `lua_pushvalue` | copies the index slot to `[L+8]`, then `add [ecx+8],16` (`0x6f33eb`) |

Addresses: `0x6f2ec0`, `0x6f2ecc`, `0x6f2f19`, `0x6f2f24`, `0x6f2f80`, `0x6f2f9c`, `0x6f3007`, `0x6f3080`, `0x6f309a`, `0x6f30bb`, `0x6f30c5`, `0x6f30d0`, `0x6f3140`, `0x6f318b`, `0x6f31a0`, `0x6f3218`, `0x6f326b`, `0x6f32b0`, `0x6f3347`, `0x6f3350`, `0x6f33eb`, `0x6f3363`, `0x6f33ad`, `0x6f30e1`, `0x6f31b9`, `0x6f32c8`
Confidence: verified
Proof: one
Test: none

The index walk is `0x6f3363` to `0x6f33ad` in `lua_pushvalue`: `idx > 0` gives `base + idx*16 - 16`,
`idx > -10000` gives `top + idx*16`, `-10001` (`0xffffd8ef`) gives `L+0x40`, `-10000`
(`0xffffd8f0`) gives `[G+0x30]` with `G = [L+0x10]`, and any other negative `idx` indexes the running
function's upvalues from `closure+0x10` when `0xffffd8ef - idx` does not exceed the byte at
`closure+7`. The same walk is inlined at `0x6f30e1`, `0x6f31b9`, `0x6f32c8`. Upstream `lua_remove`,
`lua_insert`, `lua_replace`, `lua_pushvalue` and `lua_xmove` are these loops.

## Index lookup and tests

`luaA_indexAcceptable` resolves an index to its slot and answers null where upstream's
acceptable-index rule does: a positive index at or past the top, and an upvalue past the running
function's count. `lua_typename` maps a tag to its name and `-1` to `no value`. The five tests
answer 0 or 1: `lua_iscfunction` for a function with a C body, `lua_isuserdata` for tags 7 and 2,
`lua_rawequal` through the raw comparison, `lua_equal` when the tags match and
`luaV_equalval` agrees, `lua_lessthan` through `luaV_lessthan`. A null slot answers 0.

| function | name | decided by |
|---|---|---|
| `0x6f3410` | `luaA_indexAcceptable` | `cmp edx,[ecx+8]` with `sbb eax,eax; and eax,edx` (`0x6f341e`) nulls a positive slot at or past the top |
| `0x6f3480` | `lua_typename` | `cmp edx,-1` (`0x6f3480`), the name table `s_luaTypeNames` indexed at `0x6f348a` |
| `0x6f34a0` | `lua_iscfunction` | tag 6 at `0x6f34a9`, then the C-closure byte at `closure+6` |
| `0x6f3530` | `lua_isuserdata` | compares the tag with 7 (`0x6f353b`) and 2 |
| `0x6f3550` | `lua_rawequal` | calls `luaO_rawequalObj` (`0x6f3574`) only when both slots are non-null |
| `0x6f3590` | `lua_equal` | tags compared, then `luaV_equalval` (`0x6f35bb`) |
| `0x6f35e0` | `lua_lessthan` | calls `luaV_lessthan` (`0x6f3605`) only when both slots are non-null |

Addresses: `0x6f3410`, `0x6f341e`, `0x6f3480`, `0x6f348a`, `0x6f34a0`, `0x6f34a9`, `0x6f3530`, `0x6f353b`, `0x6f3550`, `0x6f3574`, `0x6f3590`, `0x6f35bb`, `0x6f35e0`, `0x6f3605`, `0x6f3400`, `0x6f3409`
Confidence: verified
Proof: one
Test: none

`luaA_indexAcceptable` is the walk of `Stack moves` plus the positive-index bound: the positive
arm builds `base + idx*16 - 16` and masks it to 0 when it is not below the top, where the
checked `luaA_index` of upstream only asserts (`api_check`, compiled out by default). `lua_type` (`0x6f3400`) calls it and answers `-1` for a
null slot (`0x6f3409`), the value `lua_typename` names `no value`.

## Reading values

Six functions read a value out of its slot. `lua_toboolean` answers 0 for nil and false and 1
for anything else, `lua_strlen` the length of a string or of a number converted to one,
`lua_tocfunction` the C function of a C closure, `lua_touserdata` the block of a full userdata
or the pointer of a light one, `lua_tothread` the state of a thread, and `lua_topointer` a
pointer for a userdata, table, function or thread. A slot that does not fit answers 0.

| function | name | decided by |
|---|---|---|
| `0x6f3660` | `lua_toboolean` | tag 0 and tag 1 with payload 0 answer 0 (`0x6f366b`, `0x6f366f`, `0x6f3677`) |
| `0x6f36e0` | `lua_strlen` | string length at `[str+0xc]` (`0x6f36f7`), `luaV_tostring` first for a number (`0x6f3701`) |
| `0x6f3720` | `lua_tocfunction` | tag 6 at `0x6f3729`, C byte at `closure+6`, function at `closure+0xc` |
| `0x6f3740` | `lua_touserdata` | tag 2 returns the payload, tag 7 returns the payload plus 16 (`0x6f375c`) |
| `0x6f3770` | `lua_tothread` | tag 8 at `0x6f3779`, payload returned |
| `0x6f3790` | `lua_topointer` | jump table at `0x6f37a9` on tag minus 2; userdata tags jump to `0x6f3740` (`0x6f37bc`), tags 5, 6 and 8 return the payload |

Addresses: `0x6f3660`, `0x6f366b`, `0x6f366f`, `0x6f3677`, `0x6f36e0`, `0x6f36f7`, `0x6f3701`, `0x6f3720`, `0x6f3729`, `0x6f3740`, `0x6f375c`, `0x6f3770`, `0x6f3779`, `0x6f3790`, `0x6f37a9`, `0x6f37bc`, `0x6f45a0`
Confidence: verified
Proof: one
Test: none

A full userdata's block begins 16 bytes past its header (`0x6f375c`), the offset
`lua_newuserdata` returns (`0x6f45a0`).

## Pushing values

Seven functions create a value on the stack. `lua_pushnumber` stores tag 3 and a double,
`lua_pushlstring` and `lua_pushvfstring` and `lua_pushfstring` intern or format a string, and
`lua_newuserdata` and `lua_newthread` allocate an object, the first returning the address of its
block. `lua_pushupvalues` pushes every upvalue of the running C function and answers their count.
The five that allocate first run the collector when the allocation count `[G+0x28]` has reached
the threshold `[G+0x24]`.

| function | name | decided by |
|---|---|---|
| `0x6f3810` | `lua_pushnumber` | tag 3 stored (`0x6f3816`), the double from the stack to `+8` (`0x6f382f`), `ret 8` |
| `0x6f3840` | `lua_pushlstring` | collector test (`0x6f384f`), tag 4 (`0x6f3862`), `luaS_newlstr` (`0x6f3876`) |
| `0x6f38c0` | `lua_pushvfstring` | collector test, then `luaO_pushvfstring` (`0x6f38e3`) with the caller's list |
| `0x6f38f0` | `lua_pushfstring` | collector test, `luaO_pushvfstring` (`0x6f3912`) on the address of its own arguments |
| `0x6f3030` | `lua_newthread` | collector test (`0x6f3039`), `luaE_newthread` (`0x6f3047`), tag 8 (`0x6f304f`) |
| `0x6f4560` | `lua_newuserdata` | `luaS_newudata` (`0x6f457c`), tag 7 (`0x6f4584`), result `eax+16` (`0x6f45a0`) |
| `0x6f45b0` | `lua_pushupvalues` | count from `closure+7` (`0x6f45c7`), growth check (`0x6f45de`), one 16-byte copy per upvalue (`0x6f45f0` loop) |

Addresses: `0x6f3810`, `0x6f3816`, `0x6f382f`, `0x6f3840`, `0x6f384f`, `0x6f3862`, `0x6f3876`, `0x6f38c0`, `0x6f38e3`, `0x6f38f0`, `0x6f3912`, `0x6f3030`, `0x6f3039`, `0x6f3047`, `0x6f304f`, `0x6f4560`, `0x6f457c`, `0x6f4584`, `0x6f45a0`, `0x6f45b0`, `0x6f45c7`, `0x6f45de`, `0x6f45f0`, `0x6f3857`, `0x6f4573`
Confidence: verified
Proof: one
Test: none

The threshold test is `cmp ecx,[eax+0x24]; jb` before the call to `luaC_collectgarbage`
(`0x6f3857`, `0x6f4573`). The double of `lua_pushnumber` sits on the stack and `ret 8` pops
it.

## Tables and metatables

Twelve functions read and write tables. `lua_gettable` and `lua_settable` go through
metamethods (`luaV_gettable`, `luaV_settable`), `lua_rawget`, `lua_rawgeti`, `lua_rawset` and
`lua_rawseti` do not. `lua_getmetatable` and `lua_setmetatable` read and write the metatable of a
table or full userdata, and `lua_getfenv` and `lua_setfenv` the environment of a Lua function.
`lua_next` steps a traversal and `lua_concat` joins the top `n` values. The default metatable
`[G+0x48]` reads as no metatable.

| function | name | decided by |
|---|---|---|
| `0x6f3a40` | `lua_gettable` | calls `luaV_gettable` with loop 0 (`0x6f3ab3`) |
| `0x6f3b00` | `lua_rawget` | calls `luaH_get` (`0x6f3b6f`), copies the result over the key |
| `0x6f3bc0` | `lua_rawgeti` | calls `luaH_getnum` (`0x6f3c30`), top raised by 16 (`0x6f3c6a`) |
| `0x6f3cf0` | `lua_getmetatable` | tag 5 or 7 (`0x6f3cfe`, `0x6f3d03`), result compared with `[G+0x48]`, tag 5 pushed (`0x6f3d37`) |
| `0x6f3d50` | `lua_getfenv` | tag 6 and the C byte (`0x6f3db5`), `closure+0x10` (`0x6f3dc4`), else `L+0x40` |
| `0x6f3e20` | `lua_settable` | calls `luaV_settable` on `top-0x20` (`0x6f3e94`), pops two (`0x6f3e99`) |
| `0x6f3ea0` | `lua_rawset` | calls `luaH_set` on `top-0x20` (`0x6f3f17`), pops two |
| `0x6f3f60` | `lua_rawseti` | calls `luaH_setnum` (`0x6f3fd8`), pops one, `ret 4` |
| `0x6f4020` | `lua_setmetatable` | nil becomes `G+0x40` (`0x6f4099`), tags 5 and 7 (`0x6f409e`, `0x6f40a3`), pointer stored at `+8` of the object |
| `0x6f40d0` | `lua_setfenv` | tag 6 and the C byte (`0x6f413f`), 16 bytes stored at `closure+0x10` |
| `0x6f4450` | `lua_next` | calls `luaH_next` (`0x6f44c1`), top raised on a result, lowered at the end |
| `0x6f44e0` | `lua_concat` | `n >= 2` calls `luaV_concat` (`0x6f450c`); `n == 0` interns the empty string (`0x6f4542`) |

Addresses: `0x6f3a40`, `0x6f3ab3`, `0x6f3b00`, `0x6f3b6f`, `0x6f3bc0`, `0x6f3c30`, `0x6f3c6a`, `0x6f3cf0`, `0x6f3cfe`, `0x6f3d03`, `0x6f3d37`, `0x6f3d50`, `0x6f3db5`, `0x6f3dc4`, `0x6f3e20`, `0x6f3e94`, `0x6f3e99`, `0x6f3ea0`, `0x6f3f17`, `0x6f3f60`, `0x6f3fd8`, `0x6f4020`, `0x6f4099`, `0x6f409e`, `0x6f40a3`, `0x6f40d0`, `0x6f413f`, `0x6f4450`, `0x6f44c1`, `0x6f44e0`, `0x6f450c`, `0x6f4542`, `0x6f44cd`, `0x6f44d5`
Confidence: verified
Proof: one
Test: none

`lua_next` raises the top by one slot on a result and lowers it by one at the end (`0x6f44cd`,
`0x6f44d5`). `lua_setmetatable` stores into the
object at `[obj+8]+8`, which is the metatable field of a table and of a userdata header. Upstream
`lua_getfenv` and `lua_setfenv` do the same on the `g` field the closure holds at `+0x10`.

## Calls and chunks

`lua_pcall` runs a call in protected mode with an optional handler index: it saves the function
position, builds the argument record on the stack and calls `luaD_pcall`. `f_call`, the record's
function, calls `luaD_call`. `lua_cpcall` and `f_Ccall` call a C function in protected mode, with
a light userdata argument. `lua_dump` writes a Lua function that has no upvalues through
`luaU_dump` and answers 0 for any other value.

| function | name | decided by |
|---|---|---|
| `0x6f41a0` | `lua_pcall` | record of the function and result count on the stack, `luaD_pcall` (`0x6f4237`), `ret 8` |
| `0x6f4250` | `f_call` | loads `[edx]` and `[edx+4]` (`0x6f4250`), calls `luaD_call` (`0x6f4256`) |
| `0x6f4260` | `lua_cpcall` | `mov edx,0x6f4290` (`0x6f427c`) names `f_Ccall` as the protected function |
| `0x6f4290` | `f_Ccall` | `luaF_newCclosure` (`0x6f4298`), tag 2 stored (`0x6f42d7`), `luaD_call` (`0x6f4315`) |
| `0x6f4370` | `lua_dump` | tag 6 and the C and upvalue bytes (`0x6f437b`, `0x6f4383`, `0x6f438a`), `luaU_dump` (`0x6f4399`) |

Addresses: `0x6f41a0`, `0x6f4237`, `0x6f4250`, `0x6f4256`, `0x6f4260`, `0x6f427c`, `0x6f4290`, `0x6f4298`, `0x6f42d7`, `0x6f4315`, `0x6f4370`, `0x6f437b`, `0x6f4383`, `0x6f438a`, `0x6f4399`, `0x6f4224`, `0x6f422e`, `0x6f4232`, `0x6f41af`, `0x6f6960`
Confidence: verified
Proof: one
Test: none

`lua_pcall` stores the function slot and the result count in a two-word record on its frame
(`0x6f4224`, `0x6f422e`), loads `edx = 0x6f4250` (`0x6f4232`), and passes the record, the
function's offset from the stack start and the handler's offset, 0 for none (`0x6f41af`), to
`luaD_pcall` (`0x6f6960`). `lua_cpcall` does the same with `0x6f4290`. The string library's `dump`
calls `lua_dump`.

## State and upvalues

`lua_atpanic` swaps the panic function of the global state, `lua_getgcthreshold` and
`lua_getgccount` answer the collector threshold and the allocated bytes in kilobytes, and
`lua_version` answers the text `Lua 5.0`. `lua_getupvalue` and `lua_setupvalue` read and write
the nth upvalue of a function through `aux_upvalue`, which answers its name: the empty string
for a C function, the debug name for a Lua one, null past the count.

| function | name | decided by |
|---|---|---|
| `0x6f3020` | `lua_atpanic` | swaps `[G+0x2c]` with `edx` (`0x6f3023`, `0x6f3026`) |
| `0x6f43e0` | `lua_getgcthreshold` | `[G+0x24] >> 10` (`0x6f43e6`) |
| `0x6f43f0` | `lua_getgccount` | `[G+0x28] >> 10` (`0x6f43f6`) |
| `0x6f4430` | `lua_version` | returns the string `Lua 5.0` (`0x6f4430`) |
| `0x6f4660` | `lua_getupvalue` | calls `aux_upvalue` (`0x6f4671`), copies the value to the top (`0x6f46b3`) |
| `0x6f46e0` | `aux_upvalue` | tag 6 (`0x6f4745`), C byte; a C function answers the empty string and `closure+n*16`, a Lua one the debug name |
| `0x6f47b0` | `lua_setupvalue` | calls `aux_upvalue` (`0x6f47c0`), copies the top into the slot |

Addresses: `0x6f3020`, `0x6f3023`, `0x6f3026`, `0x6f43e0`, `0x6f43e6`, `0x6f43f0`, `0x6f43f6`, `0x6f4430`, `0x6f4660`, `0x6f4671`, `0x6f46b3`, `0x6f46e0`, `0x6f4745`, `0x6f47b0`, `0x6f47c0`, `0x6f6ee0`
Confidence: verified
Proof: one
Test: none

`lua_atpanic` returns the old value, which opening a state sets to `0x6f6ee0` at `[G+0x2c]`.

## Unreached functions

Nothing live reaches these 16 functions, which belong to the library the client links.

| functions |
|---|
| `luaA_pushobject` `0x6f2ec0`, `lua_xmove` `0x6f2f80`, `lua_atpanic` `0x6f3020`, `lua_newthread` `0x6f3030`, `lua_iscfunction` `0x6f34a0`, `lua_isuserdata` `0x6f3530`, `lua_equal` `0x6f3590`, `lua_tocfunction` `0x6f3720` |
| `lua_cpcall` `0x6f4260`, `f_Ccall` `0x6f4290`, `lua_version` `0x6f4430`, `lua_newuserdata` `0x6f4560`, `lua_pushupvalues` `0x6f45b0`, `lua_getupvalue` `0x6f4660`, `aux_upvalue` `0x6f46e0`, `lua_setupvalue` `0x6f47b0` |

Addresses: `0x6f2ec0`, `0x6f2f80`, `0x6f3020`, `0x6f3030`, `0x6f34a0`, `0x6f3530`, `0x6f3590`, `0x6f3720`, `0x6f4260`, `0x6f4290`, `0x6f4430`, `0x6f4560`, `0x6f45b0`, `0x6f4660`, `0x6f46e0`, `0x6f47b0`
Confidence: verified
Proof: one
Test: none

`cargo xtask census --all` marks each of these rows `unreached`: no `call` or `jmp` target, no stored dword and no fall-through reaches it from live code. The control is the same run listing `lua_gettop` (`0x6f3070`) as live, with 582 call sites in the image.
