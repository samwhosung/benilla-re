# ui: the Lua state, its library and C functions

The interface's scripts run in one Lua 5.0 state at a time, held in `0xceef74` by
`FrameScript.cpp`. This doc covers when that state is opened and replaced, what it holds when a
script first runs, how chunks reach the compiler, the contract every C function the client
registers follows, and the context word the client adds to every Lua value. The Lua core itself
belongs to `lua`; its functions are cited here by call order and arguments, except where the client
changed stock Lua 5.0 or a reimplementation on a later Lua would differ.

The base library is the client's own table of 36 functions at `0x811e28`, registered straight into
the globals; the string, table, math and bit libraries are opened through `luaL_openlib`
(`0x6f4dc0`), and a compiled-in chunk adds the Lua 4 aliases.

Roots: `0x7039e0`, `0x703b80`, `0x703ba0`, `0x703b40`, `0x7020b0`, `0x702770`, `0x7027b0`, `0x7027e0`, `0x7027f0`, `0x702800`, `0x702810`, `0x702820`, `0x702830`, `0x702900`, `0x702950`, `0x703760`, `0x704120`, `0x704160`

## Opening and replacing the state

`0x7039e0` creates a state, stores it in `0xceef74` and sets it up (next entry). `0x703b80` replaces
it: when a state exists, it closes it through `0x703ba0`, which destroys it and stores 0, and then
opens a new one. The client opens the state once at start-up and replaces it when the game interface
is built, when the game shuts down, and when the glue is built with a nonzero argument; it closes it
without a replacement at exit. Every global is lost at each replacement, so a logout to the
character screen runs on a state opened twice in a row.

Addresses: `0x7039e0`, `0x7039ed`, `0x703b80`, `0x703b8e`, `0x703ba0`, `0x703bab`, `0x402aee`, `0x403701`, `0x48fe97`, `0x491231`, `0x46a87b`, `0x6f6d20`, `0x6f6f80`, `0xceef74`
Confidence: verified
Proof: one
Test: none

`0x7039e0` calls `0x6f6d20` and stores the result at `0x7039ed`. `0x703b80` tests `0xceef74`,
calls `0x703ba0` and `jmp 0x7039e0` (`0x703b8e`); `0x703ba0` calls `0x6f6f80` and stores 0
(`0x703bab`). The census of `e8` and `e9` targets and of stored dwords: `0x7039e0` is called at
`0x402aee` (start-up) and jumped to at `0x703b8e`; `0x703b80` is called at `0x48fe97`
(`CGGameUI::Initialize`), `0x491231` (`CGGameUI::ShutdownGame`) and `0x46a87b`, which the glue
builder skips for a zero argument (`glue/manager`); `0x703ba0` at `0x403701` (exit) and
`0x703b89`. None is stored as a dword. `0xceef74` has 22 references: the two stores above, and
reads in `0x7039e0`, `0x703b80`, `0x703ba0` and `0x7040d0`. `0x6f6d20` and `0x6f6f80` have one
caller each, `0x7039e1` and `0x703ba6`.

## What a new state holds

Opening sets the collection threshold to 256 KB and then, in order: the global
`__framescript_meta`, a table whose `__index` is `0x7020b0`; the registry reference of the error
handler `0x703b40` in `0x8722c8`, with −1 in `0x8722cc`; a reference to a light userdata of 1 in
`0x8722d4`; the 36 base functions as globals; the `string`, `table`, `math` and `bit` libraries;
and the chunk `compat.lua`, run from the text `0x8722d8` points at, which sets the Lua 4 names
(`getn`, `tinsert`, `strfind`, `format`, `mod`, degree-based `sin` and the like).

Addresses: `0x7039e0`, `0x7039f2`, `0x703a2f`, `0x703a61`, `0x703a8c`, `0x703aa6`, `0x703ab0`, `0x703ae7`, `0x703af5`, `0x703b26`, `0x6f4400`, `0x6f4dc0`, `0x7fd810`, `0x7fbcd0`, `0x7fb470`, `0x7fadc0`, `0x704cd0`, `0x8722c8`, `0x8722cc`, `0x8722d0`, `0x8722d4`, `0x8722d8`, `0x811e28`, `0x8729ec`, `0x872cbc`
Confidence: verified
Proof: one
Test: none

`0x7039f2` calls `0x6f4400` with 0x100. `0x6f3920` pushes `0x7020b0` as a C closure (`0x703a2f`),
stored under `__index` in the new table, which is stored under `0x8729ec`. `0x703a61` stores −1 in
`0x8722cc`; `0x703a8c` stores `luaL_ref` of the closure `0x703b40`; `0x6f3a20` pushes a light
userdata of `[0x8722d0]` (1), whose reference `0x703aa6` stores. The loop `0x703ab0` pushes each
name and closure of `0x811e28` and sets it in the globals (`-10001`) until `cmp esi,0x120`
(`0x703ae7`). `0x703af5` to `0x703b16` call the four openers, each `luaL_openlib` with its name and
table. `0x703b26` runs `0x704cd0` on `[0x8722d8]` (1,310 bytes of Lua) with the name `0x872cbc`.

## The collector switch

The client adds a switch to the garbage collector: the automatic collection returns at once while
the state's word `+0x60` is 0. A new state sets it to 1. `0x703bc0(keep)` sets it, forces a full
collection by setting the threshold to 0, and clears it unless `keep` is nonzero; both callers pass
1, so the switch stays on.

Addresses: `0x703bc0`, `0x6f43c0`, `0x6f43d0`, `0x6f4400`, `0x6f6c40`, `0x6f6c76`, `0x6f7340`, `0x6f7343`, `0x490946`, `0x4955d5`
Confidence: verified
Proof: one
Test: none

`0x6f6c76` stores 1 (`edx`) at `+0x60`. `0x6f7340` loads `+0x60` (`0x6f7343`) and returns on 0;
otherwise it marks, sweeps and frees. `0x6f43c0` stores 1 and `0x6f43d0` stores 0. `0x703bc0` calls
`0x6f43c0`, then `0x6f4400(0)`, which collects when the count reaches the new threshold, then
jumps to `0x6f43d0` for a zero argument. Its callers, `0x490946` and `0x4955d5`, load `ecx` with 1;
no other code calls or stores `0x6f43c0` or `0x6f43d0`.

## The library scripts see

The base table holds `setglobal`, `getglobal`, eight `debug` functions, `seterrorhandler`,
`geterrorhandler`, `error`, `getmetatable`, `setmetatable`, `getfenv`, `setfenv`, `next`, `ipairs`,
`pairs`, `type`, `tonumber`, `tostring`, `unpack`, `rawequal`, `rawget`, `rawset`, `assert`,
`gcinfo`, `collectgarbage`, `loadstring`, `pcall`, `xpcall`, `date`, `time` and `debugstack`, 36 in
all. There is no `print`, `dofile`, `loadfile`, `require`, `_G`, `_VERSION`, no `io`, `os`, `debug`
or `coroutine` table. `debugload`, `debuginfo`, `debugprint`, `debugdump`, `debugbreak` and
`debugtimestamp` return nothing and do nothing; the other two are `debugprofilestart` and
`debugprofilestop`. `setglobal(name, v)` sets the global to the top value through `lua_settable`, so
`setglobal("x")` sets `x` to `"x"`; `getglobal(name)` reads it through `lua_gettable`, so the
globals' metatable applies. `math` adds `pi` and sets the global `__pow`; `bit` holds `bnot`,
`band`, `bor`, `bxor`, `lshift`, `rshift`, `arshift` and `mod`.

Addresses: `0x811e28`, `0x822d88`, `0x822d40`, `0x822c60`, `0x822c18`, `0x702770`, `0x7027b0`, `0x7027e0`, `0x7027f0`, `0x702800`, `0x702810`, `0x702820`, `0x702830`, `0x7fb470`, `0x7fb492`, `0x7fb4c4`
Confidence: verified
Proof: one
Test: none

The four `luaL_reg` tables hold 12 string functions (`len` to `gsub`, `gfind` among them), 8 table
functions (`concat` to `remove`, `setn` among them), 24 math functions (`abs` to `randomseed`) and
the 8 bit functions, each ending in a null pair. The six stubs are `xor eax,eax; ret`. The census
of the names: no `_G`, `dofile`, `loadlib`, `require` or `newproxy` string exists in the image;
the one `print` is a character class name and the one `_VERSION` is inside `INTERFACE_VERSION`;
the control finds `string` (`0x871938`) and `gcinfo`. `0x7fb492` pushes π (0x400921fb54442d18)
under `pi`; `0x7fb4c4` pushes `pow` under `__pow` into the globals.

## Error handlers

A script error inside a handler is passed to the error handler `0x703b40`: when a function has been
set with `seterrorhandler`, it calls that function with the message and returns its result;
otherwise the message is returned unchanged. `seterrorhandler(f)` raises
`Usage: seterrorhandler(errfunc)` unless its first argument is a function, frees the old reference
and keeps a reference to the top value; `geterrorhandler()` answers the function, or nil before
one is set.

Addresses: `0x703b40`, `0x702900`, `0x702950`, `0x8722cc`, `0x872a30`
Confidence: verified
Proof: one
Test: none

`0x703b40` tests `[0x8722cc]` against −1, pushes the registry value, moves it under the message
(`lua_insert(-2)`) and calls it with one argument and one result (`0x6f4180`); it returns 1.
`0x702900` compares the type of argument 1 with 6, calls `luaL_unref` on the old reference when
not −1 and stores `luaL_ref` of the top in `0x8722cc`. `0x702950` pushes `rawgeti` of the registry
at `[0x8722cc]`, nil for −1.

## How a chunk reaches the compiler

Every chunk the client compiles passes `luaL_loadbuffer`, which the client changed: when the
buffer holds at least three bytes and begins `EF BB BF`, it drops those three before the reader
sees them. Exactly one mark is dropped, at offset 0 only. A precompiled chunk never loads: the
parser always compiles the bytes as source, so one beginning `\033Lua` is a syntax error. Stock
Lua 5.0's skip of a first line beginning `#` applies to every chunk. `loadstring(s, name)` names
the chunk `s` when `name` is absent and answers nil and the message on failure.

Addresses: `0x6f5690`, `0x6f5699`, `0x6f56af`, `0x6f4320`, `0x6f6a00`, `0x6f6a15`, `0x6f6a80`, `0x6f6a98`, `0x6fec00`, `0x6fc9b0`, `0x6ff580`, `0x6ff5cd`, `0x703280`, `0x704b0c`, `0x704c87`, `0x7032b0`, `0x6f57d0`
Confidence: verified
Proof: one
Test: none

`0x6f5699` is `cmp eax,3; jb`, then three byte compares, then `add edx,3; sub eax,3` (`0x6f56af`)
before `lua_load` (`0x6f4320`). Its callers are `0x704b0c` (files and buffers), `0x704c87` (script
elements), `0x7032b0` (`loadstring`) and `0x6f57d0`; no dword holds its address. `0x6f6a00` zeroes
the parser's binary flag (`0x6f6a15`) and never reads its argument, so `0x6f6a80`'s test of it
(`0x6f6a98`) always takes `0x6fc9b0`; the undump `0x6fec00` has that one caller.
`0x6ff5cd` is `cmp eax,0x23` in `0x6ff580`. `0x703280` passes `luaL_optstring(2, s)` as the name and
on failure pushes nil under the message and returns 2.

## Lua 5.0 behaviour kept

The client keeps Lua 5.0 where Lua 5.1 differs. `for k, v in t do` with a table `t` is the
`TFORPREP` opcode: it copies the table into the state slot and replaces the generator with a raw
lookup of the global `next`, once on entry, so a script that overwrites `next` changes such loops.
`next` walks the array part by index and then the node vector by slot, skipping nil values. Only
tables and full userdata have metatables: a string has none, so `s:upper()` is an error.

Addresses: `0x6f8720`, `0x6f87c6`, `0x6f94a2`, `0x6f94f5`, `0x6f94ff`, `0x6f9870`, `0x6fa760`, `0x6fa2a0`, `0x6f7bd0`, `0x8719b0`
Confidence: verified
Proof: one
Test: none

`luaV_execute` (`0x6f8720`) dispatches 35 opcodes through `0x6f9870` (`0x6f87c6`); entry 30 is
`0x6f94a2`, which compares the tag with 5, copies the slot up one place, interns `next` (`0x8719b0`,
`0x6f94f5`) and reads it with `luaH_getstr` from the globals table `[L+0x48]` (`0x6f94ff`), with no
metamethod. `0x6fa2a0` scans the array part from the key's index plus one below `[t+0x1c]`, then the
nodes up to `1 << [t+7]`, stride 0x28, testing each value's tag. `0x6f7bd0` reads a metatable only
for tags 5 and 7 and answers the nil object otherwise.

## Function names from globals

When the calling instruction does not name a function, `lua_getinfo`'s `n` option scans the
globals table's hash part from its last slot down (`travglobals`, as in stock Lua 5.0) and, at the
first string key whose value is the function, answers that key with the kind `global`. So
`debugstack`, error messages and the saved variables writer name a function by a global that holds
it, the last such key in the hash part.

Addresses: `0x6fbd70`, `0x6fbdf8`, `0x6fbe11`, `0x6fbe22`, `0x6fbe2b`, `0x6fbf90`, `0x6fbfc6`, `0x6fbfcf`, `0x6fbfd8`, `0x864a48`
Confidence: verified
Proof: one
Test: none

In `auxgetinfo`'s (`0x6fbd70`) `n` case (`0x6fbdf8`) a null answer from `getfuncname` (`0x6fc460`)
leads to `travglobals` (`0x6fbf90`, call at `0x6fbe11`), which walks `1 << [t+7]` nodes of `[L+0x48]`
downward (`0x6fbfc6`), compares each value with `luaO_rawequalObj` (`0x6fbfcf`) and requires a
string key (`0x6fbfd8`). A hit stores `global` (`0x864a48`) as the kind (`0x6fbe22`); a miss stores
`""` (`0x6fbe2b`).

## math.ceil

`math.ceil` is `0x7faf70`, the `ceil` entry of `mathlib`. It reads argument 1 with `0x6f4c80`,
which returns `lua_tonumber`'s double and, when that is 0.0 and `lua_isnumber` fails, first calls
`0x6f4b80` with the index and type 3. It takes the C runtime's `ceil` of the double, pushes it
with `lua_pushnumber` and returns one result.

Addresses: `0x7faf70`, `0x822c60`, `0x6f4c80`, `0x6f4c91`, `0x6f4ca7`, `0x6f4cb6`, `0x73fdf5`, `0x6f3810`
Confidence: verified
Proof: one
Test: none

The pair at offset 0x40 of `mathlib` holds `ceil` and `0x7faf70`. `0x7faf70` calls `0x6f4c80`
with `edx = 1`. `0x6f4c80` compares the double with 0.0 (`fcom` with `0x802970` at `0x6f4c91`,
`test ah,0x44; jp`), so a non-zero value or a NaN skips the test; then `lua_isnumber` (`0x6f4ca7`)
and the call with `push 3` (`0x6f4cb6`). The double passes on the stack to `ceil` and from it to
`lua_pushnumber`; `eax = 1`.

## The C function contract

Every C function the client registers is `int __fastcall f(lua_State *L)` that answers its result
count. Registering sets the global of the name to a closure with no upvalues. Before the VM calls
any C function, `0x42a320` checks that its address lies in the exe's `.text` section and stops the
client with `Invalid function pointer: %p` otherwise. Arguments are read by index. An absent one has
the type none, −1, not nil, so a function that tests the type against nil's 0 tells the two apart:
glue's `GetAddOnEnableState` raises its usage for an absent first argument and accepts an explicit
nil. A `Usage:` failure calls `luaL_error`, which never returns: the error unwinds to the enclosing
protected call.

Addresses: `0x704120`, `0x704160`, `0x490250`, `0x490276`, `0x83de68`, `0x6f6050`, `0x6f61a1`, `0x6f61a8`, `0x6f5de8`, `0x6f601b`, `0x42a320`, `0x42a380`, `0x884800`, `0x884c00`, `0x833124`, `0x6f4940`, `0x6f496f`, `0x6f4440`, `0x6fc780`, `0x6fc85c`, `0x6f5d80`, `0x6f3409`, `0x46d70d`, `0x46d726`
Confidence: verified
Proof: one
Test: none

`0x704120` pushes `lua_pushcclosure(fn, 0)` and the name and sets them into `-10001`; `0x704160`
pushes nil instead. `0x490250` registers the frame types, six files' functions, the 216 pairs of
`0x83de68` (`0x490276`), and 47 more files' functions. `luaD_precall` calls `0x42a320` on the
function (`0x6f61a1`) and then `call edi` with `ecx` = L (`0x6f61a8`), as do `0x6f5de8` in
`0x6f5db0` and `0x6f601b` in `0x6f5f80`. `0x42a380` finds `.text` in the image's section table and
stores its start and end in `0x884800` and `0x884c00`. `0x6f4940` ends `lua_error` (`0x6f496f`);
`0x6f4440` calls `0x6fc780`, whose exit is `jmp 0x6f5d80` (`0x6fc85c`), which `longjmp`s or calls
the panic and `exit(1)`. `lua_type` answers −1 for an index past the top (`0x6f3409`); 42 call sites
compare its answer with 0. `0x46d70d` tests argument 1 after `lua_isstring` fails and raises at
`0x46d726` unless it is 0.

## Reading numeric arguments

`lua_tonumber` answers the number, or a string's value when it parses fully; for anything else,
an absent argument included, it answers 0.0 and never raises. `lua_isnumber` accepts exactly what
converts; `lua_isstring` accepts numbers and strings. Integers come from `__ftol`, truncating
toward zero. What a function does with a missing number is its own: some raise (`SetAlpha`), some
use a default (a colour's alpha of 1), some convert anyway (`SetMaxLetters` stores 0), and some
return having done nothing (`SetScrollFromBottom`). An argument count is checked by few functions,
and exactly (`SetMaxLetters` wants two values, `self` included).

Addresses: `0x6f3410`, `0x6f3400`, `0x6f34d0`, `0x6f3510`, `0x6f3620`, `0x6f3649`, `0x6f7c20`, `0x6f7c32`, `0x6f3070`, `0x40a2b0`, `0x802970`, `0x774e90`, `0x774f1c`, `0x774f8f`, `0x799110`, `0x79919c`, `0x7991e8`, `0x793070`, `0x793078`
Confidence: verified
Proof: one
Test: none

`0x6f3410` answers null for a positive index at or past the top. `0x6f3620` loads `0.0`
(`0x802970`) for null (`0x6f362b`) and when `0x6f7c20` fails, which it does for every tag but 3
and 4 (`0x6f7c32`). `0x6f34d0` mirrors those tests; `0x6f3510` compares `lua_type` with 4 and 3.
`0x40a2b0` sets rounding to chop around `fistp`. `Frame:SetAlpha` gates argument 2 with
`0x6f34d0` (`0x774f1c`) and raises at `0x774f8f`. `SetMaxLetters` compares `lua_gettop` with 2
(`0x79919c`), raises otherwise, and stores `__ftol(lua_tonumber(2))` (`0x7991e8`).
`SetScrollFromBottom` tests argument 2 first (`0x793078`) and returns 0 on failure.

## Stack room and run errors

`lua_checkstack(L, n)` (`0x6f2f30`) returns 0 when the slots in use, `(top − base) / 16`, plus `n`
exceed 2048. Otherwise it grows the stack with `luaD_growstack(L, n)` when `stack_last − top` is at
most `16·n` bytes, raises the current call's top to `top + 16·n` when it is lower, and returns 1.
`luaG_runerror(L, fmt, ...)` (`0x6fc870`) formats the message with `luaO_pushvfstring`, passes it
to `0x6fc8a0` and calls `luaG_errormsg`, which throws. When bit 0 of the current call's `+8` is
clear, `0x6fc8a0` pushes `%s:%d: %s` of the text `0x6f5c40` writes, the number `0x6fbef0` returns
and the message. The census finds 37 calls of `0x6f2f30` and 32 of `0x6fc870`.

Addresses: `0x6f2f30`, `0x6f2f43`, `0x6f2f5a`, `0x6f2f71`, `0x6fc870`, `0x6fc8a0`, `0x6fc8ad`, `0x6f5990`, `0x6f5f60`, `0x6fc780`
Confidence: verified
Proof: one
Test: none

`0x6f2f30` takes `L` in `ecx` and `n` in `edx`: `sar ecx,4` on `[L+8] − [L+0xc]`, `cmp ecx,0x800;
jle` (`0x6f2f43`), so 2048 in all passes; `[L+0x18] − [L+8]` against `n shl 4` with `jg` past the
grow (`0x6f2f5a`); `[L+0x14]+4` against `[L+8] + 16·n` with `jae` (`0x6f2f71`). `0x6fc870` is `cdecl`
and passes the address of its variable arguments; `0x6fc8a0` tests bit 0 of `[[L+0x14]+8]`
(`0x6fc8ad`) and, when clear, calls `0x6fbef0` and `0x6fbb90` on the call, then `0x6f5c40` with a
64-byte stack buffer, `[r+0x20] + 0x10` (`r` the result of `0x6fbb90`) and the length 60, and
formats through `0x6f5c20`.

## The context word

Each Lua value is 16 bytes: a type tag, a context word at `+0x4` (padding in stock Lua 5.0) and the
value at `+0x8`. A new value takes the current context `0xceeac0`. Copying a value with a nonzero
word makes that word current while the nesting count `0xceeac4` is above 0; copying one with a zero
word gives it the current context. The per-frame update zeroes the count. A script slot runs under
its word, `RunScript` under `[0x83de24]`, an add-on's files under its name or 0. Context 0 is the
client's own code: `RegisterForSave` raises otherwise, and `0x494a50` cases 0 and 11 refuse,
signalling `MACRO_ACTION_FORBIDDEN` for `RunScript`'s word and `ADDON_ACTION_FORBIDDEN` with the
word's text for any other.

Addresses: `0x6f37f0`, `0x6f3ca9`, `0x6f94c8`, `0x6fc7c2`, `0x702690`, `0x48b980`, `0x48b9bb`, `0x51f387`, `0x4884e0`, `0x494a50`, `0x494a84`, `0x494ab8`, `0x494ac6`, `0x494add`, `0x494afb`, `0x4955a2`, `0x51ab30`, `0xceeac0`, `0xceeac4`, `0x83de24`, `0x8421cc`, `0xb4e3e4`
Confidence: verified
Proof: one
Test: none

`lua_newtable` (`0x6f3c90`) writes tag 5 and `[0xceeac0]` at `+0x4` (`0x6f3ca9`). The copy in
`TFORPREP` (`0x6f94c8`) and in the error path (`0x6fc7c2`) is the pattern: test `[src+4]`, store it
in `0xceeac0` when `0xceeac4` is nonzero, else store `0xceeac0` into `[dst+4]`. `0x48b980` saves the
context, increments the count and stores `[0x83de24]` (`0x48b9bb`), the empty string's address,
around `0x704cd0`. `0x51f387` stores the addon's name pointer or 0 by its `+0x24`. `0x4884e0` raises
`0x8421cc` for a nonzero context. `0x494a50` maps its argument through `0x494b54` (`0x494a84`):
cases 0 and 11 reach `0x494ab8`, which under the guard `0xb4e3e4` signals 0x1af (`0x494add`) when
the context equals `[0x83de24]` (`0x494ac6`), else 0x1b0 with `%s` (`0x494afb`), and returns 0.
Event 0x1af is `MACRO_ACTION_FORBIDDEN` and 0x1b0 `ADDON_ACTION_FORBIDDEN` in the table `0x51ab30`
builds. `0x495590` zeroes the count (`0x4955a2`).

## debugstack

`debugstack(start, head, tail)` answers the call stack as text, one line per level, each ending in a
newline, with no header. `start` defaults to 1, the function that called `debugstack`; `head` to 12;
`tail` to 10. Each number is taken from the first argument while it is a number or numeric string,
truncated, and removed. When arguments remain and the first is a string or number, they are joined
in front of the text with no separator, and a later one that is neither raises in the join; when the
first is anything else, nothing is built and the last argument is returned. A start past the stack
answers the joined arguments alone, `""` by default.

Addresses: `0x703760`, `0x703768`, `0x703771`, `0x703778`, `0x703782`, `0x7037a2`, `0x703809`, `0x70382d`, `0x70383f`, `0x703857`, `0x703863`, `0x703877`, `0x703889`, `0x7038cd`, `0x7038fa`, `0x7039c4`, `0x7039cc`, `0x70397a`, `0x6fbaa0`, `0x6fbc70`
Confidence: verified
Proof: one
Test: none

`0x703768` to `0x703778` set 1, 12 and 10. Each of the three slots tests `lua_isnumber(1)`
(`0x703782`), converts with `0x40a2b0` and calls `lua_remove(1)` (`0x7037a2`). `0x703809` reads
`lua_gettop`: 0 pushes `""`; otherwise `0x70382d` jumps to `0x7039b6` when `lua_isstring(1)` fails,
which returns 1, the top of the stack. Every concatenation takes the whole stack (`0x7039b0`), so
all remaining arguments join. The loop and the line are the next entry.

## debugstack's lines

A line is `short_src`, `:`, the current line and `:` when above 0, the description and a newline.
The description is `` in function `name' `` when the name kind begins with `f`, `g`, `l` or `m`;
else ` in main chunk` for a main chunk, ` ?` for a C function or a tail call, and
` in function <short_src:linedefined>` otherwise. Levels `start` to `start + head − 1` print. At a
later level L, when level `L + 1 + tail` exists, `...` and a newline replace the levels before the
last `tail`; otherwise every level prints, so up to `head + tail + 1` levels print whole.

Addresses: `0x703760`, `0x70383c`, `0x70384c`, `0x703857`, `0x703863`, `0x70386e`, `0x703877`, `0x703880`, `0x7038a0`, `0x7038dc`, `0x7038e7`, `0x703903`, `0x703908`, `0x703916`, `0x70392a`, `0x703943`, `0x70394b`, `0x703963`, `0x70397a`, `0x703998`, `0x7039b0`, `0x872cb4`, `0x872c98`, `0x872c88`, `0x872c6c`, `0x872c70`
Confidence: verified
Proof: one
Test: none

`ebx` holds the next level (`0x70383c`); the limit is `head + start` (`0x70384c`) and `jbe`
(`0x703857`) formats while `ebx` is at most it. Past it, `0x703863` probes `ebx + tail`; a miss
decrements `ebx` and clears the flag (`0x703877`), so the same level prints; a hit pushes `...\n`
(`0x703880`) and advances `ebx` while `ebx + tail` exists (`0x7038a0`). `lua_getinfo("Snl")`
precedes `"%s:"` (`0x7038dc`) and `"%d:"` when `currentline > 0` (`0x7038e7`). The name kind's
first byte minus `f` indexes `0x7039cc` (`00 00 01 01 01 01 00 00`) into `0x7039c4`; the other arm
tests `what` for `m`, `C` and `t` (`0x70392a`, `0x703943`). Each line ends with `"\n"`
(`0x70397a`) and a concatenation of the whole stack; `0x7039b0` concatenates once more.
