# lua: the auxiliary library

The auxiliary library of the embedded Lua 5.0, `lauxlib.c` of the upstream source, from
`0x6f4810` to `0x6f57f0`. The client's C functions check their arguments, build strings and keep
references through it. It has the upstream 5.0 shape: error text with a backquote and an
apostrophe (``bad argument #%d to `%s' (%s)``), a string buffer that holds 512 bytes in the
caller's frame, and the file loader and the `lua_do*` family, which no live code reaches.
Each function is a `__fastcall` with the state in `ecx` and the first argument in `edx`, as in
`api`. The names are upstream's.

Roots: `0x6f4810`, `0x6f48e0`, `0x6f4b20`, `0x6f4b50`, `0x6f4bb0`, `0x6f4be0`, `0x6f4c20`, `0x6f4c80`, `0x6f4cd0`, `0x6f4d00`, `0x6f4d60`, `0x6f4ea0`, `0x6f5050`, `0x6f5120`, `0x6f51f0`, `0x6f5250`, `0x6f5270`, `0x6f5300`, `0x6f5310`, `0x6f5400`

## Argument checks

Argument errors go through `luaL_argerror`, which names the function from `lua_getinfo` and raises
with `luaL_error`: ``bad argument #%d to `%s' (%s)``, or ``calling `%s' on bad self (%s)`` when the
name kind is `method`. `luaL_typerror` and `tag_error` make the `X expected, got Y` message from a type
name or tag. The checks are `luaL_checktype`, `luaL_checkany`, `luaL_checkstack`,
`luaL_checklstring`, `luaL_checknumber` and the optional forms `luaL_optlstring` and
`luaL_optnumber`. `luaL_where` pushes `chunk:line: ` of a call level.

| function | name | decided by |
|---|---|---|
| `0x6f4810` | `luaL_argerror` | `lua_getstack` (`0x6f4824`), `lua_getinfo` (`0x6f4834`), the text `method` (`0x6f483c`), the two formats |
| `0x6f48a0` | `luaL_typerror` | `lua_type`, `lua_typename` (`0x6f48b2`), the format `%s expected, got %s` (`0x6f48bc`) |
| `0x6f48e0` | `luaL_where` | `lua_getstack` (`0x6f48ed`), `lua_getinfo` with `Snl` (`0x6f48fa`), the format `%s:%d: ` (`0x6f4912`) |
| `0x6f4b20` | `luaL_checkstack` | `lua_checkstack` (`0x6f4b26`), the format `stack overflow (%s)` (`0x6f4b33`) |
| `0x6f4b50` | `luaL_checktype` | `lua_type` compared with the argument (`0x6f4b59`), `tag_error` (`0x6f4b6a`) |
| `0x6f4b80` | `tag_error` | `lua_typename` (`0x6f4b8c`) then `luaL_typerror` (`0x6f4b96`) |
| `0x6f4bb0` | `luaL_checkany` | `lua_type` equal to -1 (`0x6f4bbb`), the text `value expected` (`0x6f4bc0`) |
| `0x6f4be0` | `luaL_checklstring` | `lua_tostring` (`0x6f4bea`), `tag_error` with tag 4 (`0x6f4bfb`), `lua_strlen` (`0x6f4c0b`) |
| `0x6f4c20` | `luaL_optlstring` | `lua_type` above 0 selects `luaL_checklstring` (`0x6f4c6a`), else the default and its length (`0x6f4c40`) |
| `0x6f4c80` | `luaL_checknumber` | `lua_tonumber` (`0x6f4c8c`), `lua_isnumber` only when it is 0.0 (`0x6f4ca7`), `tag_error` with tag 3 (`0x6f4cb6`) |
| `0x6f4cd0` | `luaL_optnumber` | `lua_type` (`0x6f4cd9`) selects `luaL_checknumber` (`0x6f4cef`) or the default |

Addresses: `0x6f4810`, `0x6f4824`, `0x6f4834`, `0x6f483c`, `0x6f48a0`, `0x6f48b2`, `0x6f48bc`, `0x6f48e0`, `0x6f48ed`, `0x6f48fa`, `0x6f4912`, `0x6f4b20`, `0x6f4b26`, `0x6f4b33`, `0x6f4b50`, `0x6f4b59`, `0x6f4b6a`, `0x6f4b80`, `0x6f4b8c`, `0x6f4b96`, `0x6f4bb0`, `0x6f4bbb`, `0x6f4bc0`, `0x6f4be0`, `0x6f4bea`, `0x6f4bfb`, `0x6f4c0b`, `0x6f4c20`, `0x6f4c6a`, `0x6f4c40`, `0x6f4c80`, `0x6f4c8c`, `0x6f4ca7`, `0x6f4cb6`, `0x6f4cd0`, `0x6f4cd9`, `0x6f4cef`
Confidence: verified
Proof: one
Test: none

These follow the 5.0 text of upstream: the optional forms test `lua_type` against 0 and -1, and
`luaL_checknumber` calls `lua_isnumber` only when `lua_tonumber` gave 0.0 (`0x6f4ca7`). The
formats use the 5.0 quotes, a backquote and an apostrophe. The default of `luaL_optnumber` is a
double on the stack, so it takes `ret 8`.

## Metatables and fields

`luaL_newmetatable` makes the registry entry of a type name, storing the table under the name
and the name under the table, and answers 0 when it exists. `luaL_getmetatable` reads it,
`luaL_checkudata` answers the block of a userdata whose metatable is registered under a name, and
`luaL_getmetafield` and `luaL_callmeta` read and call a metatable field. `luaL_findstring`
returns the position of a string in a null-ended list.

| function | name | decided by |
|---|---|---|
| `0x6f4980` | `luaL_findstring` | the compare loop steps both pointers by 2 (`0x6f49ab`), `-1` at the list end (`0x6f49cb`) |
| `0x6f49e0` | `luaL_newmetatable` | `lua_rawget` (`0x6f49f2`), `lua_newtable` (`0x6f4a18`), two `lua_rawset` (`0x6f4a39`, `0x6f4a58`) |
| `0x6f4a70` | `luaL_getmetatable` | `lua_pushstring` then a tail `jmp` to `lua_rawget` (`0x6f4a80`) |
| `0x6f4a90` | `luaL_checkudata` | `lua_getmetatable` (`0x6f4a99`), `lua_rawget` on the registry (`0x6f4aa9`), `lua_tostring`, compare, `lua_touserdata` (`0x6f4afe`) |
| `0x6f4d00` | `luaL_getmetafield` | `lua_getmetatable` (`0x6f4d06`), `lua_rawget` (`0x6f4d25`), `lua_remove` of the metatable (`0x6f4d50`) |
| `0x6f4d60` | `luaL_callmeta` | `luaL_getmetafield` (`0x6f4d86`), `lua_pushvalue` (`0x6f4d99`), `lua_call` (`0x6f4da7`) |

Addresses: `0x6f4980`, `0x6f49ab`, `0x6f49cb`, `0x6f49e0`, `0x6f49f2`, `0x6f4a18`, `0x6f4a39`, `0x6f4a58`, `0x6f4a70`, `0x6f4a80`, `0x6f4a90`, `0x6f4a99`, `0x6f4aa9`, `0x6f4afe`, `0x6f4d00`, `0x6f4d06`, `0x6f4d25`, `0x6f4d50`, `0x6f4d60`, `0x6f4d86`, `0x6f4d99`, `0x6f4da7`, `0x6f4ad6`
Confidence: verified
Proof: one
Test: none

`luaL_checkudata` looks the value's metatable up as a key of the registry, takes the string it
finds there and compares it with the requested name by the byte loop at `0x6f4ad6`: the 5.0
scheme, where the registry maps a metatable to its name (the second `lua_rawset` of
`luaL_newmetatable`) as well as a name to its metatable (the first).

## Table sizes and references

`luaL_getn` and `luaL_setn` keep the length of a table. A table with an `n` field uses it. Else the
length is held in a weak-key table at registry reference 2 (`getsizes`), keyed by the table, and
`luaL_getn` counts upward from index 1 when neither exists. `checkint` reads an
integer and answers -1 for a non-number. `luaL_ref` stores the top value in a table and returns
a number for it, reusing freed numbers from slot 1; `luaL_unref` returns one.

| function | name | decided by |
|---|---|---|
| `0x6f4ea0` | `luaL_setn` | `lua_rawget` of `n` (`0x6f4ed1`), `checkint` (`0x6f4edd`), `getsizes` (`0x6f4f2f`), `lua_rawset` (`0x6f4f54`) |
| `0x6f4f80` | `checkint` | `lua_tonumber`, `__ftol` (`0x6f4f8f`), `lua_isnumber` (`0x6f4f9f`), -1 (`0x6f4fa8`), `lua_settop` (`0x6f4fb2`) |
| `0x6f4fc0` | `getsizes` | `lua_rawgeti` 2 (`0x6f4fca`), the `__mode` field `k` (`0x6f5008`), `lua_rawseti` (`0x6f5041`) |
| `0x6f5050` | `luaL_getn` | `lua_rawget` of `n` (`0x6f507d`), `getsizes` (`0x6f5098`), `lua_rawgeti` probe loop (`0x6f50cd`) |
| `0x6f5310` | `luaL_ref` | nil returns -1 (`0x6f534a`), free list at slot 1 (`push 1` at `0x6f536e`, read `0x6f5374`), else `luaL_getn` (`0x6f53b1`) at least 2, plus 1, so the first number is 3 (`0x6f53b8`) |
| `0x6f5400` | `luaL_unref` | a negative reference returns at once (`0x6f5410`); slot 1 read (`0x6f5444`, `0x6f544a`), relinked (`0x6f5454`, `0x6f5469`, `0x6f546f`) |

Addresses: `0x6f4ea0`, `0x6f4ed1`, `0x6f4edd`, `0x6f4f2f`, `0x6f4f54`, `0x6f4f80`, `0x6f4f8f`, `0x6f4f9f`, `0x6f4fa8`, `0x6f4fb2`, `0x6f4fc0`, `0x6f4fca`, `0x6f5008`, `0x6f5041`, `0x6f5050`, `0x6f507d`, `0x6f5098`, `0x6f50cd`, `0x6f5310`, `0x6f534a`, `0x6f536e`, `0x6f5374`, `0x6f53b1`, `0x6f53b8`, `0x6f5400`, `0x6f5410`, `0x6f5444`, `0x6f544a`, `0x6f5454`, `0x6f5469`, `0x6f546f`, `0x6f5363`, `0x6f53e0`, `0x6f5423`, `0x6f5480`, `0x6f4f23`, `0x6f4f6e`, `0x6f4f11`
Confidence: verified
Proof: one
Test: none

`getsizes` is the 5.0 mechanism that lets `table.setn` work on tables that carry no `n`
field. `luaL_ref` and `luaL_unref` follow upstream: slot 1 (`FREELIST_REF`) holds the head of the
free list, the numbers 1 and 2 are reserved, and a reference is the index of the slot that holds the
value. The client's `luaL_ref` and `luaL_unref` clear the context word while the nesting count is
nonzero and restore it afterwards: `luaL_ref` only for a non-nil value (`0x6f5363`, `0x6f53e0`),
`luaL_unref` at `0x6f5423` and `0x6f5480`. `luaL_setn` does so only on the size-table path
(`0x6f4f23` to `0x6f4f6e`); the path that stores the `n` field returns at `0x6f4f11`.

## String buffers

A `luaL_Buffer` is `p` at `+0`, the number of stack pieces at `+4`, the state at `+8` and 512
bytes at `+0xc`. `luaL_buffinit` sets it up. Adding bytes fills the area; when it is full
`emptybuffer` pushes its contents as a string and `adjuststack` merges the pushed pieces from the
top by `lua_concat` while the pieces taken so far are longer than the next one, or ten are pushed. `luaL_pushresult` pushes
the rest and joins them, and `luaL_addvalue` adds the string on the stack top.

| function | name | decided by |
|---|---|---|
| `0x6f5120` | `luaL_prepbuffer` | `emptybuffer` (`0x6f5123`), `adjuststack` when it pushed (`0x6f512e`), returns `buf+0xc` |
| `0x6f5140` | `emptybuffer` | length `p-buf-0xc` (`0x6f5147`), `lua_pushlstring` (`0x6f515a`), level count `+4` raised (`0x6f5164`) |
| `0x6f5170` | `adjuststack` | `lua_strlen` pairs (`0x6f5190`, `0x6f51a5`), limit 10 (`0x6f51b2`), `lua_concat` (`0x6f51cf`) |
| `0x6f51f0` | `luaL_addlstring` | calls `luaL_prepbuffer` when the area is full (`0x6f520f`) |
| `0x6f5230` | `luaL_addstring` | length by `repne scasb`, then `luaL_addlstring` (`0x6f5243`) |
| `0x6f5250` | `luaL_pushresult` | `emptybuffer` (`0x6f5253`), `lua_concat` (`0x6f525e`), level reset to 1 |
| `0x6f5270` | `luaL_addvalue` | `lua_strlen`, `lua_tostring` (`0x6f52a3`), copy when it fits, else `lua_insert` and `adjuststack` (`0x6f52e7`, `0x6f52f5`) |
| `0x6f5300` | `luaL_buffinit` | stores `L` at `+8`, `p = buf+0xc`, level 0 (`0x6f5303`, `0x6f5306`, `0x6f5308`) |

Addresses: `0x6f5120`, `0x6f5123`, `0x6f512e`, `0x6f5140`, `0x6f5147`, `0x6f515a`, `0x6f5164`, `0x6f5170`, `0x6f5190`, `0x6f51a5`, `0x6f51b2`, `0x6f51cf`, `0x6f51f0`, `0x6f520f`, `0x6f5230`, `0x6f5243`, `0x6f5250`, `0x6f5253`, `0x6f525e`, `0x6f5270`, `0x6f52a3`, `0x6f52e7`, `0x6f52f5`, `0x6f5300`, `0x6f5303`, `0x6f5306`, `0x6f5308`, `0x6f5290`
Confidence: verified
Proof: one
Test: none

The buffer area ends at `buf+0x20c` (`0x6f5290`), the limit at which `luaL_addvalue` pushes
instead of copying.

## Loading chunks

`luaL_loadfile` opens a file (or standard input for a null name), reopens it in binary mode when
its first byte is neither a space nor printable, and reads it through `lua_load` with the reader
`getF`, 512 bytes at a time; `errfile` makes the `cannot read` message. `getS` is the string reader of `luaL_loadbuffer`.
`lua_dofile`, `lua_dobuffer` and `lua_dostring` load and run a chunk, `aux_do` calling it in
protected mode and `callalert` the global `_ALERT`, or printing the message, on failure.

| function | name | decided by |
|---|---|---|
| `0x6f5490` | `luaL_loadfile` | `fopen` (`0x6f5506`), `getc` and `ungetc` (`0x6f54ca`, `0x6f54d3`), the `rb` reopen (`0x6f557f`), `lua_load` (`0x6f55bb`), the names `=stdin` and `@%s` |
| `0x6f5610` | `getF` | end-of-file flag test (`0x6f5615`), `fread` of 0x200 (`0x6f562e`) |
| `0x6f5650` | `errfile` | `lua_tostring`, `strerror` (`0x6f5667`), the format `cannot read %s: %s` (`0x6f566e`), `lua_remove` (`0x6f5680`) |
| `0x6f56e0` | `getS` | size zero returns null (`0x6f56e6`, `0x6f56eb`); else the size is stored (`0x6f56f1`) and cleared (`0x6f56f5`) |
| `0x6f5700` | `lua_dofile` | `luaL_loadfile` (`0x6f5703`), tail `jmp` to `aux_do` (`0x6f570d`) |
| `0x6f5720` | `aux_do` | `lua_pcall` (`0x6f572d`), `callalert` (`0x6f5738`) |
| `0x6f5750` | `callalert` | the name `_ALERT` (`0x6f5757`), `lua_gettable`, type 6 (`0x6f5777`), `lua_call` (`0x6f5791`), `fprintf` (`0x6f57a8`) |
| `0x6f57c0` | `lua_dobuffer` | `luaL_loadbuffer` (`0x6f57d0`), `aux_do` (`0x6f57d9`) |
| `0x6f57f0` | `lua_dostring` | length by `repne scasb`, `lua_dobuffer` (`0x6f5804`) |

Addresses: `0x6f5490`, `0x6f5506`, `0x6f54ca`, `0x6f54d3`, `0x6f557f`, `0x6f55bb`, `0x6f5610`, `0x6f5615`, `0x6f562e`, `0x6f5650`, `0x6f5667`, `0x6f566e`, `0x6f5680`, `0x6f56e0`, `0x6f56e6`, `0x6f56eb`, `0x6f56f1`, `0x6f56f5`, `0x6f5700`, `0x6f5703`, `0x6f570d`, `0x6f5720`, `0x6f572d`, `0x6f5738`, `0x6f5750`, `0x6f5757`, `0x6f5777`, `0x6f5791`, `0x6f57a8`, `0x6f57c0`, `0x6f57d0`, `0x6f57d9`, `0x6f57f0`, `0x6f5804`, `0x6f5690`, `0x6f56c3`
Confidence: verified
Proof: one
Test: none

`luaL_loadbuffer` (`0x6f5690`) passes `getS` to `lua_load` as the reader (`0x6f56c3`). The file
loader and `_ALERT` follow the 5.0 text of upstream.

## Unreached functions

Nothing live reaches these 13 functions, which belong to the library the client links.

| functions |
|---|
| `luaL_findstring` `0x6f4980`, `luaL_newmetatable` `0x6f49e0`, `luaL_getmetatable` `0x6f4a70`, `luaL_checkudata` `0x6f4a90`, `luaL_addstring` `0x6f5230`, `luaL_loadfile` `0x6f5490`, `getF` `0x6f5610`, `errfile` `0x6f5650` |
| `lua_dofile` `0x6f5700`, `aux_do` `0x6f5720`, `callalert` `0x6f5750`, `lua_dobuffer` `0x6f57c0`, `lua_dostring` `0x6f57f0` |

Addresses: `0x6f4980`, `0x6f49e0`, `0x6f4a70`, `0x6f4a90`, `0x6f5230`, `0x6f5490`, `0x6f5610`, `0x6f5650`, `0x6f5700`, `0x6f5720`, `0x6f5750`, `0x6f57c0`, `0x6f57f0`
Confidence: verified
Proof: one
Test: none

`cargo xtask census --all` marks each of these rows `unreached`: no `call` or `jmp` target, no stored dword and no fall-through reaches it from live code. The control is the same run listing `lua_gettop` (`0x6f3070`) as live, with 582 call sites in the image.
