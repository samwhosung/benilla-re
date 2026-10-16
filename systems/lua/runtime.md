# lua: calls, states and the collector

The run-time core of the embedded Lua 5.0, from `ldo.c`, `lstate.c`, `lgc.c`, `lvm.c` and
`ldebug.c` of the upstream source, between `0x6f5e20` and `0x6fc8a0`. The functions that allocate
pass the path of their own file to `luaM_irealloc`, which fixes which file they belong to. The
state keeps the call records in a vector of 24-byte entries (`[L+0x28]` the first, `[L+0x14]` the
current, `[L+0x24]` the end): base, top, a flag word (1 C function, 2 has a frame, 4 calling, 8 saved
pc valid, 0x10 suspended), the saved pc, the address of the running pc and a tail-call count. The
global state at `[L+0x10]` holds the string table at `+4`, the collector lists (`+0x14`
userdata, `+0x18` to finalize), the registry (`+0x30`), the default metatable (`+0x40`), the
threshold (`+0x24`) and the block count (`+0x28`). The names are upstream's.

Roots: `0x6f5e20`, `0x6f5ef0`, `0x6f5f60`, `0x6f61c0`, `0x6f6200`, `0x6f6370`, `0x6f6470`, `0x6f65a0`, `0x6f6680`, `0x6f6960`, `0x6f6b10`, `0x6f6b80`, `0x6f6ef0`, `0x6f7080`, `0x6f71d0`, `0x6f7370`, `0x6f7b20`, `0x6f7ba0`, `0x6f7cf0`, `0x6f7d60`, `0x6f7ee0`, `0x6f7f40`, `0x6f8190`, `0x6f8330`, `0x6f84d0`, `0x6f8670`, `0x6f9900`, `0x6f99e0`, `0x6f9a80`, `0x6fb480`, `0x6fb4c0`, `0x6fb4f0`, `0x6fb520`, `0x6fb5c0`, `0x6fba10`, `0x6fbb90`, `0x6fbef0`, `0x6fbf20`, `0x6fc4c0`, `0x6fc6f0`, `0x6fc8a0`

## Stack and call records

The stack and the call-record vector grow by reallocation. `luaD_growstack` doubles the stack
or adds the needed size plus five, `luaD_reallocstack` makes the new stack and `correctstack`
rebases the top, the open upvalues and every call record. `luaD_reallocCI` resizes the records,
`luaD_growCI` doubles them and raises `stack overflow` when the new size passes 4096,
`restore_stack_limit` resets the limits after an error. `luaD_call` counts C nesting, runs
`luaD_precall` and the machine, then `luaD_poscall`, which moves the results down and pads with
nil. `tryfuncTM` makes a non-function callable through `__call` and `adjust_varargs` builds the
`arg` table.

| function | name | decided by |
|---|---|---|
| `0x6f5e20` | `luaD_reallocstack` | `luaM_irealloc` in `ldo.c` (`0x6f5e45`), `stack_last` six slots before the end (`0x6f5e4d`), tail `jmp` to `correctstack` (`0x6f5e62`) |
| `0x6f5e70` | `correctstack` | top (`0x6f5e81`), open upvalue list `[L+0x50]` (`0x6f5e84`), call records 0x18 apart (`0x6f5ed8`), base (`0x6f5ee5`) |
| `0x6f5ef0` | `luaD_reallocCI` | `luaM_irealloc` in `ldo.c` for 24 bytes a record (`0x6f5f1a`), division by 24 (`0x6f5f2b`) |
| `0x6f5f60` | `luaD_growstack` | compare (`0x6f5f63`), double (`0x6f5f67`), size plus n plus 5 (`0x6f5f6f`), both ending in `luaD_reallocstack` |
| `0x6f61c0` | `luaD_growCI` | size test 4096 (`0x6f61c7`), `luaD_throw` tail (`0x6f61d3`), `luaD_reallocCI` (`0x6f61df`), `stack overflow` (`0x6f61f2`) |
| `0x6f6200` | `adjust_varargs` | `luaH_new` (`0x6f6266`), `luaH_setnum` (`0x6f6294`), the key `n` (`0x6f62f2`), `luaH_set` (`0x6f6302`) |
| `0x6f6370` | `tryfuncTM` | `luaT_gettmbyobj` (`0x6f6382`), tag 6 test (`0x6f6393`), `call` (`0x6f639b`), `luaG_typeerror` (`0x6f63a4`) |
| `0x6f6470` | `luaD_poscall` | return hook test (`0x6f6479`), `callrethooks` (`0x6f648b`), record popped (`0x6f649c`), result copy (`0x6f64b2`), nil fill (`0x6f6517`) |
| `0x6f6540` | `callrethooks` | `luaD_callhook` event 1 (`0x6f6550`), event 4 per tail call (`0x6f6574`) |
| `0x6f65a0` | `luaD_call` | limit 200 (`0x6f65ae`), `C stack overflow` (`0x6f65bf`), `luaD_throw` 5 (`0x6f65cf`), `luaD_precall` (`0x6f65df`), `luaV_execute` (`0x6f65ea`), `luaD_poscall` (`0x6f65f5`) |
| `0x6f6760` | `restore_stack_limit` | `stack_last` from the size (`0x6f6773`), 4096 test (`0x6f6769`), tail `jmp` to `luaD_reallocCI` (`0x6f67a0`) |

Addresses: `0x6f5e20`, `0x6f5e45`, `0x6f5e4d`, `0x6f5e62`, `0x6f5e70`, `0x6f5e81`, `0x6f5e84`, `0x6f5ed8`, `0x6f5ee5`, `0x6f5ef0`, `0x6f5f1a`, `0x6f5f2b`, `0x6f5f60`, `0x6f5f63`, `0x6f5f67`, `0x6f5f6f`, `0x6f61c0`, `0x6f61c7`, `0x6f61d3`, `0x6f61df`, `0x6f61f2`, `0x6f6200`, `0x6f6266`, `0x6f6294`, `0x6f62f2`, `0x6f6302`, `0x6f6370`, `0x6f6382`, `0x6f6393`, `0x6f639b`, `0x6f63a4`, `0x6f6470`, `0x6f6479`, `0x6f648b`, `0x6f649c`, `0x6f64b2`, `0x6f6517`, `0x6f6540`, `0x6f6550`, `0x6f6574`, `0x6f65a0`, `0x6f65ae`, `0x6f65bf`, `0x6f65cf`, `0x6f65df`, `0x6f65ea`, `0x6f65f5`, `0x6f6760`, `0x6f6773`, `0x6f6769`, `0x6f67a0`
Confidence: verified
Proof: one
Test: none

`luaD_reallocstack` stores `size - 6` slots as the usable end (`add edi,0xfffffffa`): the stack
keeps five extra slots. The line numbers 153 and 162 that `luaD_reallocstack` and
`luaD_reallocCI` pass are upstream's 122 and 131 plus 31 each, so the client's `ldo.c` has 31
added lines before them. `luaD_call` raises `C stack overflow` when the count reaches exactly 200
(`0xc8`) and throws status 5 from 225 (`0xe1`) on. The return-hook test is bit 2 of the hook
mask at `[L+0x30]` (`0x6f6479`).

## Errors and protected calls

`luaD_pcall` runs a function through `luaD_rawrunprotected` and, on an error, restores the saved
nesting count, call record and hook flag, closes the open upvalues and puts the error value at the
saved top through `seterrorobj`: the message itself for a runtime or syntax error, `not enough
memory` for status 4 and `error in error handling` for 5. `lua_resume` runs `resume` the same way,
`resume` starts a coroutine or finishes the call it yielded in, and `lua_yield` marks the call
record suspended and returns -1.

| function | name | decided by |
|---|---|---|
| `0x6f6680` | `seterrorobj` | status 4 `not enough memory` (`0x6f66aa`), 5 `error in error handling` (`0x6f66d9`), the others copy the message |
| `0x6f6960` | `luaD_pcall` | saves nesting, record, hook flag and handler (`0x6f696a`, `0x6f6975`, `0x6f697a`, `0x6f6984`), `luaD_rawrunprotected` (`0x6f6996`), `luaF_close` (`0x6f69ab`), `seterrorobj` (`0x6f69b5`), `restore_stack_limit` (`0x6f69da`) |
| `0x6f6620` | `lua_resume` | `luaD_rawrunprotected` (`0x6f6638`), `luaF_close` (`0x6f6658`), `seterrorobj` (`0x6f6665`), `restore_stack_limit` (`0x6f666f`) |
| `0x6f67b0` | `resume` | `cannot resume dead coroutine` (`0x6f67d2`), `luaD_precall` (`0x6f67e5`), flag tests (`0x6f67ef`, `0x6f67f4`), `non-suspended` (`0x6f6838`), `luaV_execute` (`0x6f6842`) |
| `0x6f6860` | `lua_yield` | `attempt to yield across metamethod/C-call boundary` (`0x6f6880`), `cannot yield a C function` (`0x6f68a0`), flag `0x10` set (`0x6f694c`), -1 (`0x6f6953`) |

Addresses: `0x6f6680`, `0x6f66aa`, `0x6f66d9`, `0x6f6960`, `0x6f696a`, `0x6f6975`, `0x6f697a`, `0x6f6984`, `0x6f6996`, `0x6f69ab`, `0x6f69b5`, `0x6f69da`, `0x6f6620`, `0x6f6638`, `0x6f6658`, `0x6f6665`, `0x6f666f`, `0x6f67b0`, `0x6f67d2`, `0x6f67e5`, `0x6f67ef`, `0x6f67f4`, `0x6f6838`, `0x6f6842`, `0x6f6860`, `0x6f6880`, `0x6f68a0`, `0x6f694c`, `0x6f6953`
Confidence: verified
Proof: one
Test: none

`resume` raises its two messages itself with `luaG_runerror`; the 5.0.1 and later sources return
them from a separate `resume_error`.

## State open and close

`lua_open` (`ui/luastate`) runs `f_luaopen` in protected mode. It allocates the 192-byte global
state and stores `default_panic` in it, calls `stack_init` (a stack of 45 slots and 8 call
records), makes the default metatable (its own metatable), the globals table and the registry,
sizes the string table at 32, runs `luaT_init` and `luaX_init`, fixes the `not enough memory`
string and sets the first threshold to four times the block count. `luaE_newthread` makes a
coroutine state. `close_state` closes the upvalues, sweeps every object, frees the stacks, calls the
client's memory-pool teardown `0x6fb2f0` (`0x6f6f54`) and frees the global state. `mallocstate`, `freestate`, `freestack` and `luaE_freethread` allocate and free the
records, and `callallgcTM` is the protected function that runs the last finalizers.

| function | name | decided by |
|---|---|---|
| `0x6f6b10` | `luaE_newthread` | `mallocstate` (`0x6f6b14`), `luaC_link` type 8 (`0x6f6b21`), `preinit_state` (`0x6f6b28`), `stack_init` (`0x6f6b37`) |
| `0x6f6b80` | `mallocstate` | `luaM_irealloc` in `lstate.c` for 0x68 bytes (`0x6f6b8d`) |
| `0x6f6ba0` | `stack_init` | two `luaM_irealloc` in `lstate.c` for 0x2d0 and 0xc0 bytes (`0x6f6bb8`, `0x6f6be4`) |
| `0x6f6c90` | `luaE_freethread` | `luaF_close` (`0x6f6c9b`), `freestack` (`0x6f6ca4`), tail `jmp` to `freestate` (`0x6f6caf`) |
| `0x6f6cc0` | `freestate` | `luaM_irealloc` in `lstate.c` (`0x6f6ccb`) |
| `0x6f6ce0` | `freestack` | two `luaM_irealloc` in `lstate.c` (`0x6f6cfd`, `0x6f6d17`) |
| `0x6f6d70` | `f_luaopen` | allocates 0xc0 bytes (`0x6f6d88`), panic store (`0x6f6ddd`), three `luaH_new` (`0x6f6e3b`, `0x6f6e62`, `0x6f6e87`), `luaS_resize` 32 (`0x6f6e96`), `luaT_init` (`0x6f6e9d`), threshold `shl 2` (`0x6f6ec8`) |
| `0x6f6ee0` | `default_panic` | three bytes: `xor eax,eax; ret` |
| `0x6f6ef0` | `close_state` | `luaF_close` (`0x6f6ef6`), `luaC_sweep` (`0x6f6f09`), `luaS_freeall` (`0x6f6f10`), `freestack` (`0x6f6f46`), pool teardown `0x6fb2f0` (`0x6f6f54`), `freestate` (`0x6f6f79`) |
| `0x6f6fe0` | `callallgcTM` | tail `jmp` to `luaC_callGCTM` (`0x6f6fe0`) |
| `0x6f7b40` | `luaT_init` | loop over the name table (`0x6f7b72`), `luaS_newlstr`, limit `0xbc` (`0x6f7b8c`), the fixed bit `0x10` |

Addresses: `0x6fb2f0`, `0x6f6f54`, `0x6f6b10`, `0x6f6b14`, `0x6f6b21`, `0x6f6b28`, `0x6f6b37`, `0x6f6b80`, `0x6f6b8d`, `0x6f6ba0`, `0x6f6bb8`, `0x6f6be4`, `0x6f6c90`, `0x6f6c9b`, `0x6f6ca4`, `0x6f6caf`, `0x6f6cc0`, `0x6f6ccb`, `0x6f6ce0`, `0x6f6cfd`, `0x6f6d17`, `0x6f6d70`, `0x6f6d88`, `0x6f6ddd`, `0x6f6e3b`, `0x6f6e62`, `0x6f6e87`, `0x6f6e96`, `0x6f6e9d`, `0x6f6ec8`, `0x6f6ee0`, `0x6f6ef0`, `0x6f6ef6`, `0x6f6f09`, `0x6f6f10`, `0x6f6f46`, `0x6f6f79`, `0x6f6fe0`, `0x6f7b40`, `0x6f7b72`, `0x6f7b8c`, `0x6f6f1e`, `0x6f6f5c`, `0x6f6e0b`, `0x6f6d63`, `0x6f6fc5`, `0x6f6fa9`
Confidence: verified
Proof: one
Test: none

The line numbers `lstate.c` passes in the allocating functions are one above upstream's: 50, 66, 70,
82, 83 and 92 against 49, 65, 69, 81, 82 and 91. `close_state` passes 154 (`0x6f6f1e`) and 160
(`0x6f6f5c`) against upstream's 151 and 156. The initial block count is 296 (`0x6f6e0b`, `0x128`), the sum of the two
records' sizes in upstream. The stack size 45 is `BASIC_STACK_SIZE + EXTRA_STACK`, 40 plus 5, and
the record count 8 is `BASIC_CI_SIZE`. `close_state` is called at `0x6f6d63` when
`lua_open` fails and at `0x6f6fc5` in `lua_close`, which hands `0x6f6fe0` to
`luaD_rawrunprotected` (`0x6f6fa9`) until the finalizers stop failing.

## Sweeping and finalizers

A collection frees what is unmarked. `luaC_sweep` sweeps the main list and every string bucket
(`sweepstrings`) with `sweeplist`, which frees an unmarked object through `freeobj` by its tag and
clears the mark of a survivor. `luaC_callGCTM` calls the `__gc` metamethods of the userdata set aside
for finalizing, one at a time through `do1gcTM`, with hooks off. `checkSizes` halves an oversized
string table and work buffer, and then sets the next threshold in a function the client added.
`luaC_link` puts a new object on the main list with a tag and clear marks.

| function | name | decided by |
|---|---|---|
| `0x6f7080` | `luaC_callGCTM` | hooks off (`0x6f7099`), each userdata moved to the main list (`0x6f70aa`), `do1gcTM` (`0x6f70da`) |
| `0x6f7120` | `do1gcTM` | no-tag-method flag test (`0x6f712a`), `luaT_gettm` (`0x6f7146`), `luaD_call` (`0x6f71bd`) |
| `0x6f71d0` | `luaC_sweep` | limit 0x100 for the free-all argument (`0x6f71da`), `sweeplist` (`0x6f71e8`), `sweepstrings` (`0x6f71f1`), `sweeplist` (`0x6f71ff`) |
| `0x6f7210` | `sweeplist` | mark byte against the limit (`0x6f7229`), low mark bit of a survivor cleared (`0x6f722e`), `freeobj` (`0x6f723e`) |
| `0x6f7260` | `freeobj` | tag test (`0x6f7267`): proto `0x6fa050` (`0x6f7273`), closure `0x6fa110` (`0x6f7278`), table `0x6fa690` (`0x6f7291`), thread `0x6f6c90` (`0x6f7296`) |
| `0x6f72f0` | `sweepstrings` | `sweeplist` per bucket (`0x6f7317`), string count lowered by the freed number (`0x6f731c`) |
| `0x6f7370` | `checkSizes` | string count under a quarter and size over 64 (`0x6f738c`), `luaS_resize` (`0x6f739c`), buffer realloc in `lgc.c` (`0x6f73c1`), tail `jmp` to the threshold update (`0x6f73d6`) |
| `0x6f7b20` | `luaC_link` | old head stored in the object (`0x6f7b29`), object stored in `[G+0x10]` (`0x6f7b31`), mark 0 (`0x6f7b34`), tag from the argument (`0x6f7b38`) |

Addresses: `0x6f7080`, `0x6f7099`, `0x6f70aa`, `0x6f70da`, `0x6f7120`, `0x6f712a`, `0x6f7146`, `0x6f71bd`, `0x6f71d0`, `0x6f71da`, `0x6f71e8`, `0x6f71f1`, `0x6f71ff`, `0x6f7210`, `0x6f7229`, `0x6f722e`, `0x6f723e`, `0x6f7260`, `0x6f7267`, `0x6fa050`, `0x6f7273`, `0x6fa110`, `0x6f7278`, `0x6fa690`, `0x6f7291`, `0x6f6c90`, `0x6f7296`, `0x6f72f0`, `0x6f7317`, `0x6f731c`, `0x6f7370`, `0x6f738c`, `0x6f739c`, `0x6f73c1`, `0x6f73d6`, `0x6f7b20`, `0x6f7b29`, `0x6f7b31`, `0x6f7b34`, `0x6f7b38`, `0x6f7340`, `0x6f73e0`, `0x6f734a`, `0x6f7353`, `0x6f735a`, `0x6f7362`, `0x6fae00`, `0x6fae67`, `0x6fae83`, `0x6f727d`, `0x6f729e`, `0x6f72b7`, `0x6f73af`
Confidence: verified
Proof: one
Test: none

`luaC_collectgarbage` (`0x6f7340`, `ui/luastate`) calls the marking at `0x6f73e0` (`0x6f734a`),
`luaC_sweep` (`0x6f7353`) and `checkSizes` (`0x6f735a`), then jumps to `luaC_callGCTM` (`0x6f7362`). Where
upstream 5.0 ends `checkSizes` with a threshold of twice the block count, the client jumps to
`0x6fae00`. That function takes the headroom `[G+0x24] - [G+0x28]`, lowers it to the least free
space among the six block pools at `[G+0]`, and stores the block count plus the larger of that and
a quarter of the block count (`0x6fae67`..`0x6fae83`). The allocation lines `freeobj` and
`checkSizes` pass (345, 353 and 357 at `0x6f727d`, `0x6f729e` and `0x6f72b7`, 399 at `0x6f73af`) are
5.0's 344, 352, 356 and 398 plus 1; 5.0.1 and 5.0.2 have 347, 355, 359 and 401, so the client's
`lgc.c` descends from the 5.0 release.

## Metamethod access

Indexing goes through `luaV_gettable` and `luaV_settable`. `luaV_gettable` reads a table directly
and, for a nil result, follows the `__index` of its metatable (`luaV_index`); a value that is no
table asks `luaV_getnotable` for `__index`, which raises `attempt to index` when there is none.
A function metamethod is called through `callTMres` (one result) or `callTM` (none), a table is
indexed again, at most 100 times (`loop in gettable`, `loop in settable`). `call_binTM` tries the
metamethod of each operand of an operator.

| function | name | decided by |
|---|---|---|
| `0x6f7cf0` | `luaV_gettable` | loop limit 100 (`0x6f7cf7`), `loop in gettable` (`0x6f7d08`), table test (`0x6f7d10`), `luaH_get` (`0x6f7d1d`), `luaV_index` (`0x6f7d31`), `luaV_getnotable` (`0x6f7d47`) |
| `0x6f7d60` | `luaV_index` | cached absence flag (`0x6f7d6f`), `luaT_gettm` (`0x6f7d81`), function tag (`0x6f7d97`) to `callTMres` (`0x6f7da3`), else `luaV_gettable` (`0x6f7db8`) |
| `0x6f7ba0` | `luaT_gettm` | `luaH_getstr` (`0x6f7bac`), nil result sets bit `event` of `[t+6]` (`0x6f7bbc`) and answers null |
| `0x6f7dd0` | `callTMres` | three 16-byte pushes, growth check (`0x6f7ea4`), `luaD_call` with one result (`0x6f7ec7`) |
| `0x6f7ee0` | `luaV_getnotable` | `luaT_gettmbyobj` (`0x6f7eec`), nil raises `index` (`0x6f7f01`), function to `callTMres` (`0x6f7f14`), else `luaV_gettable` (`0x6f7f2f`) |
| `0x6f7f40` | `luaV_settable` | `luaH_set` (`0x6f7f64`), flag test (`0x6f7f74`), `luaT_gettm` (`0x6f7f8b`), `luaT_gettmbyobj` (`0x6f7fd7`), limit (`0x6f7ffa`), `callTM` (`0x6f802c`) |
| `0x6f8040` | `callTM` | four 16-byte pushes, growth check (`0x6f8158`), `luaD_call` with no result (`0x6f817b`) |
| `0x6f8670` | `call_binTM` | `luaT_gettmbyobj` twice (`0x6f8684`, `0x6f8697`), function test (`0x6f869c`), `callTMres` (`0x6f86b3`) |

Addresses: `0x6f7cf0`, `0x6f7cf7`, `0x6f7d08`, `0x6f7d10`, `0x6f7d1d`, `0x6f7d31`, `0x6f7d47`, `0x6f7d60`, `0x6f7d6f`, `0x6f7d81`, `0x6f7d97`, `0x6f7da3`, `0x6f7db8`, `0x6f7ba0`, `0x6f7bac`, `0x6f7bbc`, `0x6f7dd0`, `0x6f7ea4`, `0x6f7ec7`, `0x6f7ee0`, `0x6f7eec`, `0x6f7f01`, `0x6f7f14`, `0x6f7f2f`, `0x6f7f40`, `0x6f7f64`, `0x6f7f74`, `0x6f7f8b`, `0x6f7fd7`, `0x6f7ffa`, `0x6f802c`, `0x6f8040`, `0x6f8158`, `0x6f817b`, `0x6f8670`, `0x6f8684`, `0x6f8697`, `0x6f869c`, `0x6f86b3`, `0x6f83c4`, `0x6f6382`
Confidence: verified
Proof: one
Test: none

The event numbers the code passes are upstream's `TMS` order: 0 `__index` (`0x6f7eec`), 1
`__newindex`, 4 `__eq` (`0x6f83c4`), 5 to 9 the operators, 0xb `__lt`, 0xc `__le`, 0xd `__concat`,
0xe `__call` (`0x6f6382`). `luaV_gettable` passes `loop + 1` on each step (`0x6f7d31`, `0x6f7db8`).

## Order, equality and arithmetic

Comparisons: `luaV_lessthan` and `luaV_lessequal` compare numbers by value and strings through
`luaV_strcmp` (`strcoll` over embedded zeros), else try `__lt` or `__le` through `call_orderTM`
(`__le` falls back to the negated `__lt` with swapped operands) and raise `luaG_ordererror`.
`luaV_equalval` compares by tag, then values, then `__eq` through `get_compTM`. `luaV_concat`
joins the top values, strings and numbers directly and others through `__concat`. `Arith` performs
`+ - * / ^` on numbers, the metamethod, or `luaG_aritherror`; `^` calls the global `__pow`.
`traceexec` calls the count and line hooks.

| function | name | decided by |
|---|---|---|
| `0x6f8190` | `luaV_lessthan` | number tag (`0x6f81a3`), `fcom` parity test (`0x6f81b2`), string tag (`0x6f81cc`), `luaV_strcmp` (`0x6f81d7`), `call_orderTM` (`0x6f81f3`), `luaG_ordererror` (`0x6f8202`) |
| `0x6f99e0` | `luaV_lessequal` | numbers (`0x6f99f3`, `0x6f9a02`), strings (`0x6f9a1c`, `0x6f9a27`), `call_orderTM` for `__le` (`0x6f9a43`) and `__lt` (`0x6f9a54`), `luaG_ordererror` (`0x6f9a73`) |
| `0x6f8210` | `call_orderTM` | `luaT_gettmbyobj` (`0x6f821e`, `0x6f823d`), `luaO_rawequalObj` (`0x6f8246`), `callTMres` (`0x6f8262`), truth test (`0x6f8270`) |
| `0x6f82a0` | `luaV_strcmp` | `strcoll` (`0x6f82bd`, `0x6f82fe`) |
| `0x6f8330` | `luaV_equalval` | tag switch (`0x6f8339`), number compare (`0x6f8359`), `get_compTM` (`0x6f83c4`), `callTMres` (`0x6f83dc`), truth test (`0x6f83ea`) |
| `0x6f8440` | `get_compTM` | `luaT_gettm` for each (`0x6f846e`, `0x6f84a1`), equal metatables (`0x6f8481`), `luaO_rawequalObj` (`0x6f84bc`) |
| `0x6f84d0` | `luaV_concat` | `luaV_tostring` (`0x6f8507`, `0x6f851e`), `call_binTM` (`0x6f8533`), `luaG_concaterror` (`0x6f8546`), `string size overflow` (`0x6f85ad`), `luaS_newlstr` (`0x6f8635`) |
| `0x6f9a80` | `Arith` | `luaV_tonumber` twice (`0x6f9a93`, `0x6f9aa8`), operator switch, `__pow` lookup (`0x6f9b5e`), `call_binTM` (`0x6f9bf3`), `luaG_aritherror` (`0x6f9c01`) |
| `0x6f9900` | `traceexec` | mask above 4 (`0x6f990b`), `luaD_callhook` (`0x6f9926`), line mask test (`0x6f9932`), `luaG_inithooks` (`0x6f9970`), `luaD_callhook` (`0x6f99bb`) |

Addresses: `0x6f8190`, `0x6f81a3`, `0x6f81b2`, `0x6f81cc`, `0x6f81d7`, `0x6f81f3`, `0x6f8202`, `0x6f99e0`, `0x6f99f3`, `0x6f9a02`, `0x6f9a1c`, `0x6f9a27`, `0x6f9a43`, `0x6f9a54`, `0x6f9a73`, `0x6f8210`, `0x6f821e`, `0x6f823d`, `0x6f8246`, `0x6f8262`, `0x6f8270`, `0x6f82a0`, `0x6f82bd`, `0x6f82fe`, `0x6f8330`, `0x6f8339`, `0x6f8359`, `0x6f83c4`, `0x6f83dc`, `0x6f83ea`, `0x6f8440`, `0x6f846e`, `0x6f84a1`, `0x6f8481`, `0x6f84bc`, `0x6f84d0`, `0x6f8507`, `0x6f851e`, `0x6f8533`, `0x6f8546`, `0x6f85ad`, `0x6f8635`, `0x6f9a80`, `0x6f9a93`, `0x6f9aa8`, `0x6f9b5e`, `0x6f9bf3`, `0x6f9c01`, `0x6f9900`, `0x6f990b`, `0x6f9926`, `0x6f9932`, `0x6f9970`, `0x6f99bb`, `0x6f9b72`
Confidence: verified
Proof: one
Test: none

`luaV_lessequal` calls `call_orderTM` with the event `0xc` (`0x6f9a43`) and, when it answers -1,
with `0xb` and the operands swapped (`0x6f9a54`), returning the negated answer. `Arith` raises
`` `__pow' (`^' operator) is not a function `` when the global `__pow` is not a function
(`0x6f9b72`). The comparison of the type names in `luaG_ordererror` is on the
third character, `t1[2] == t2[2]`, which is how upstream 5.0 tells `attempt to compare two %s values`
from `attempt to compare %s with %s`.

## Error messages

Run-time errors name what failed. `luaG_typeerror` builds ``attempt to <op> <kind> `<name>' (a
<type> value)`` when the operand is on the stack and `getobjname` finds a name for it (a local, a
global, a field or a method), else ``attempt to <op> a <type> value``; `luaG_concaterror`,
`luaG_aritherror` and `luaG_ordererror` pick the operand and the verb. `addinfo` prefixes the
message with `source:line:` of a Lua function. `currentpc` and `currentline` read the position,
`funcinfo` the source and kind of a function, `getluaproto` the prototype of a call record.

| function | name | decided by |
|---|---|---|
| `0x6fba10` | `currentpc` | C flag test (`0x6fba13`), frame flag copies the live pc (`0x6fba1b`), `(pc - code) >> 2` minus 1 (`0x6fba35`) |
| `0x6fbb90` | `getluaproto` | C flag test (`0x6fbb90`), function at `base[-1]`, prototype at closure `+0xc` (`0x6fbb9b`) |
| `0x6fbef0` | `currentline` | `currentpc` (`0x6fbef0`), negative gives -1 (`0x6fbef9`), the line table of the prototype indexed by the pc (`0x6fbf0c`) |
| `0x6fbf20` | `funcinfo` | C: `=[C]` (`0x6fbf2c`) and `luaO_chunkid` (`0x6fbf49`); Lua: `main` (`0x6fbf66`) or `Lua` (`0x6fbf6d`), `luaO_chunkid` (`0x6fbf7d`) |
| `0x6fc4c0` | `getobjname` | `luaF_getlocalname` (`0x6fc4f5`), `luaG_symbexec` (`0x6fc508`), opcode 5 gives `global` (`0x6fc577`), 6 `field` (`0x6fc59a`), 0xb `method` (`0x6fc5bd`) |
| `0x6fc5f0` | `kname` | index minus 250 (`0x6fc5f0`), negative gives the question mark, string tag test (`0x6fc603`), text at `+0x10` (`0x6fc60b`) |
| `0x6fc620` | `luaG_typeerror` | `isinstack` (`0x6fc63e`), `getobjname` (`0x6fc658`), the formats ``attempt to %s %s `%s' (a %s value)`` (`0x6fc66b`) and `attempt to %s a %s value` (`0x6fc687`) |
| `0x6fc6a0` | `isinstack` | slot loop from base to top in steps of 16 (`0x6fc6b0`..`0x6fc6b9`), 1 on equal (`0x6fc6be`) |
| `0x6fc6d0` | `luaG_concaterror` | string tag test (`0x6fc6d3`), the verb `concatenate` (`0x6fc6db`), `luaG_typeerror` (`0x6fc6e0`) |
| `0x6fc6f0` | `luaG_aritherror` | `luaV_tonumber` (`0x6fc701`), the verb `perform arithmetic on` (`0x6fc70f`), `luaG_typeerror` (`0x6fc716`) |
| `0x6fc730` | `luaG_ordererror` | third characters of the type names compared, the two formats (`0x6fc754`, `0x6fc76b`) |
| `0x6fc8a0` | `addinfo` | C flag test (`0x6fc8ad`), `currentline` (`0x6fc8b9`), `getluaproto` (`0x6fc8c4`), `luaO_chunkid` (`0x6fc8d2`), the format `%s:%d: %s` (`0x6fc8e0`), `luaO_pushfstring` (`0x6fc8e6`) |

Addresses: `0x6fba10`, `0x6fba13`, `0x6fba1b`, `0x6fba35`, `0x6fbb90`, `0x6fbb9b`, `0x6fbef0`, `0x6fbef9`, `0x6fbf0c`, `0x6fbf20`, `0x6fbf2c`, `0x6fbf49`, `0x6fbf66`, `0x6fbf6d`, `0x6fbf7d`, `0x6fc4c0`, `0x6fc4f5`, `0x6fc508`, `0x6fc577`, `0x6fc59a`, `0x6fc5bd`, `0x6fc5f0`, `0x6fc603`, `0x6fc60b`, `0x6fc620`, `0x6fc63e`, `0x6fc658`, `0x6fc66b`, `0x6fc687`, `0x6fc6a0`, `0x6fc6b0`, `0x6fc6b9`, `0x6fc6be`, `0x6fc6d0`, `0x6fc6d3`, `0x6fc6db`, `0x6fc6e0`, `0x6fc6f0`, `0x6fc701`, `0x6fc70f`, `0x6fc716`, `0x6fc730`, `0x6fc754`, `0x6fc76b`, `0x6fc8a0`, `0x6fc8ad`, `0x6fc8b9`, `0x6fc8c4`, `0x6fc8d2`, `0x6fc8e0`, `0x6fc8e6`
Confidence: verified
Proof: one
Test: none

`luaG_runerror` (`ui/luastate`) calls `addinfo` and then `luaG_errormsg`. `getobjname` finds the
instruction that set a register by `luaG_symbexec` (`0x6fc508`) and maps the opcode to the kind:
`OP_GETGLOBAL`, `OP_GETTABLE` or `OP_SELF`. `isinstack` is a scan, not a range test.

## Input streams

The compiler reads its input through a stream record: a count `n` at `+0`, the next byte `p`
at `+4`, the reader function at `+8`, its data at `+0xc` and the chunk name at `+0x10`. `luaZ_init`
fills it and `luaZ_fill` calls the reader with a null state, the data and the address of a size,
keeping the block it returns. `luaZ_lookahead` peeks at the next byte, `luaZ_read` copies a number
of bytes out. `luaZ_openspace` grows the lexer's work buffer to at least 32 bytes.

| function | name | decided by |
|---|---|---|
| `0x6fb480` | `luaZ_fill` | reader called through `[esi+8]` (`0x6fb490`), null or empty block gives -1 (`0x6fb4b2`), count minus 1 (`0x6fb4a0`) |
| `0x6fb4c0` | `luaZ_lookahead` | `luaZ_fill` when the count is 0 (`0x6fb4c8`), the byte put back (`0x6fb4db`, `0x6fb4dc`) |
| `0x6fb4f0` | `luaZ_init` | stores at `+8`, `+0xc`, `+0x10` (`0x6fb4f6`, `0x6fb4fc`, `0x6fb4ff`), count and pointer zeroed (`0x6fb502`, `0x6fb508`) |
| `0x6fb520` | `luaZ_read` | `luaZ_fill` (`0x6fb548`), `rep movs` (`0x6fb573`, `0x6fb57d`), 0 when all bytes were read (`0x6fb59e`), the bytes still wanted at the end of the stream (`0x6fb5a8`) |
| `0x6fb5c0` | `luaZ_openspace` | size test (`0x6fb5cd`), floor 32 (`0x6fb5d1`), `luaM_irealloc` in `lzio.c` (`0x6fb5e6`) |

Addresses: `0x6fb480`, `0x6fb490`, `0x6fb4b2`, `0x6fb4a0`, `0x6fb4c0`, `0x6fb4c8`, `0x6fb4db`, `0x6fb4dc`, `0x6fb4f0`, `0x6fb4f6`, `0x6fb4fc`, `0x6fb4ff`, `0x6fb502`, `0x6fb508`, `0x6fb520`, `0x6fb548`, `0x6fb573`, `0x6fb57d`, `0x6fb59e`, `0x6fb5a8`, `0x6fb5c0`, `0x6fb5cd`, `0x6fb5d1`, `0x6fb5e6`, `0x6fb4a4`, `0x6fb4aa`, `0x700067`, `0x700104`, `0x70016e`, `0x7001cb`, `0x7001eb`, `0x7002ee`
Confidence: verified
Proof: one
Test: none

`luaZ_fill` stores the size minus 1 as the count and the block as `p`, then returns the byte at `p` and
advances it (`0x6fb4a4`..`0x6fb4aa`), as upstream 5.0 does; `luaM_irealloc` here passes the line 75,
the line of the allocation in upstream `luaZ_openspace`. `luaZ_read` returns the number of bytes it
could not read. The lexer calls `luaZ_fill` from six sites (`0x700067`, `0x700104`, `0x70016e`,
`0x7001cb`, `0x7001eb`, `0x7002ee`).

## Unreached functions

Nothing live reaches these 4 functions, which belong to the library the client links.

| functions |
|---|
| `lua_resume` `0x6f6620`, `resume` `0x6f67b0`, `lua_yield` `0x6f6860`, `luaE_newthread` `0x6f6b10` |

Addresses: `0x6f6620`, `0x6f67b0`, `0x6f6860`, `0x6f6b10`
Confidence: verified
Proof: one
Test: none

`cargo xtask census --all` marks each of these rows `unreached`: no `call` or `jmp` target, no stored dword and no fall-through reaches it from live code. The control is the same run listing `lua_gettop` (`0x6f3070`) as live, with 582 call sites in the image.
