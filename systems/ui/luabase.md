# ui: the Lua base library

The functions `FrameScript.cpp` registers as globals of the script state: the Lua 5.0 base library as the client
builds it, from `error` to `date`. `0x7039e0` installs them from the table at `0x811e28`, pairs of a name and a
function (`luastate`). This doc holds those with a body of their own: error and call control, metatables and
environments, iteration, conversion, raw access, the collector pair and the date and time pair. The setters, the
`debug` stubs, the error-handler pair, `loadstring` and `debugstack` are in `luastate`. Each is a `lua_CFunction`:
the state in `ecx`, the number of results in `eax`.

The Lua API calls are Lua 5.0's, derived in `lua/api` and `lua/auxlib`: `lua_type` `0x6f3400`,
`lua_settop` `0x6f3080`, `lua_pushvalue` `0x6f3350`, `lua_pushnumber` `0x6f3810`, `lua_pushstring` `0x6f3890`,
`lua_rawget` `0x6f3b00`, `lua_rawgeti` `0x6f3bc0`, `luaL_checktype` `0x6f4b50`, `luaL_checkany` `0x6f4bb0`,
`luaL_error` `0x6f4940`.

Roots: `0x702970`, `0x7029f0`, `0x702a40`, `0x702ac0`, `0x702be0`, `0x702c90`, `0x702cd0`, `0x702da0`, `0x702df0`, `0x702e30`,
`0x702f40`, `0x7030a0`, `0x7030f0`, `0x703130`, `0x703160`, `0x7031a0`, `0x703200`, `0x703250`, `0x7032f0`, `0x703340`,
`0x7033a0`, `0x7035a0`

## Error and assert

`error(message [, level])` takes the level as `__ftol` of the optional number, default 1. A string message at a nonzero
level has the caller's position (`0x6f48e0` with the level) concatenated before it; any other message is raised as it is.
`assert(v [, message])` needs an argument; when it is false or nil the message, `assertion failed!` by default, is raised
through `luaL_error("%s", message)`. Otherwise the stack is cut to the first argument and 1 is returned.

Addresses: `0x702970`, `0x702985`, `0x7029a8`, `0x7029b0`, `0x7029c8`, `0x7029d1`, `0x7031a0`, `0x7031d6`, `0x7031e5`, `0x6f4440`, `0x6f48e0`, `0x6f44e0`, `0x6f4c20`
Confidence: verified
Proof: one
Test: none

`0x702970` reads argument 2 with the optional-number helper `0x6f4cd0` (default bits `0x3ff00000`, 1.0) and `__ftol`
(`0x702985`), requires argument 1, and when `lua_isstring(1)` and the level is nonzero (`0x7029a8`, `0x7029aa`) calls
`0x6f48e0(level)`, pushes argument 1, calls `0x6f44e0(2)` (`0x7029c8`) and tail-jumps to `lua_error`
(`0x7029d1`); the other path pushes argument 1 and jumps there. A negative level reaches `0x6f48e0`. `0x7031a0` takes the
optional string through `0x6f4c20` with the default `assertion failed!` (`0x872bdc`), passes `%s` (`0x82e280`) to
`luaL_error`, and otherwise calls `lua_settop(1)` (`0x7031e5`).

## Protected calls

`pcall(f, ...)` requires `f`, calls it with every other argument and keeps all its results, with no handler. It pushes
`true` for status 0 and `false` otherwise, inserts that at index 1 and returns every value on the stack.
`xpcall(f, handler)` requires the handler, leaves `[handler, f]`, calls `f` with no arguments and the handler as error
function, pushes the boolean, replaces index 1 with it and returns the stack. After a failure the error value is the second result.

Addresses: `0x7032f0`, `0x703303`, `0x70330d`, `0x703316`, `0x703327`, `0x70332f`, `0x703340`, `0x703360`, `0x70336d`, `0x703376`, `0x703387`, `0x70338f`, `0x6f41a0`
Confidence: verified
Proof: one
Test: none

`0x7032f0` calls `lua_gettop` (`0x703303`) and `lua_pcall(n − 1, −1, 0)` (`0x70330d`; the pushes before it are `0`, the
handler, and `-1`, the result count), tests the status with `sete` (`0x703316`), pushes it, inserts it at 1 and
tail-jumps to `lua_gettop`. `0x703340` sets the top to 2 (`0x703354`), inserts the top at 1 (`0x703360`), calls
`lua_pcall(0, −1, 1)` (`0x70336d`), pushes the status, replaces index 1 with it (`0x6f32b0`, `edx = 1`) and tail-jumps to
`lua_gettop`.

## Metatables

`getmetatable(v)` requires an argument and answers nil for a value without a metatable. Otherwise it answers the
metatable's `__metatable` entry when there is one, the metatable itself when not. `setmetatable(t, mt)` requires a
table at 1 and nil or a table at 2 (argument 2 error `nil or table expected`), raises `cannot change a protected
metatable` when the table's metatable has a `__metatable` entry, then sets the metatable and returns the table.

Addresses: `0x7029f0`, `0x702a04`, `0x702a0f`, `0x702a25`, `0x702a40`, `0x702a62`, `0x702a73`, `0x702a82`, `0x702a93`, `0x702aae`, `0x6f4d00`, `0x6f3cf0`, `0x6f4020`, `0x872a50`, `0x872a5c`, `0x872a80`
Confidence: verified
Proof: one
Test: none

`0x6f4d00(L, 1, name)` is the metatable-field lookup: it answers 0 with nothing pushed when the object has no metatable
(`0x6f3cf0`), otherwise pushes the name, raw-gets it from the metatable, pops both and answers 0 on nil
(`0x6f4d3f`), and on a value removes the metatable under it and answers 1 (`0x6f4d50`). `0x7029f0` calls it with
`__metatable` (`0x872a50`) and returns 1 whatever it answers, so the metatable stays on top when there is no field.
`0x702a40` reads the type of argument 2 first (`0x702a49`), accepts 0 and 5 only (`0x702a5e`, `0x702a62`), runs the same
lookup (`0x702a82`), raises through `luaL_error` (`0x702a93`), then sets the top to 2 and calls `0x6f4020`.

## Function environments

`getfenv([f])` takes a function or a level (default 1; `level must be non-negative` below 0, `invalid level` past the
stack) and answers the function's environment table, or that table's `__fenv` entry when it has one. `setfenv(f, table)`
requires a table at 2 and the same function-or-level at 1, raises `` `setfenv' cannot change a protected environment``
when the environment has a `__fenv` entry, and otherwise sets it. A numeric `f` equal to 0 replaces the state's global
table with the table and returns nothing. A function the call cannot set raises
`` `setfenv' cannot change environment of given function``. A level naming a tail call raises `no function
environment for tail call at level %d`.

Addresses: `0x702ac0`, `0x702af0`, `0x702ba0`, `0x702be0`, `0x702b04`, `0x702b27`, `0x702b4b`, `0x702b70`, `0x702b8a`, `0x702bc0`, `0x702c45`, `0x702c59`, `0x702c69`, `0x6fbaa0`, `0x6fbc70`, `0x6f3d50`, `0x6f40d0`, `0x6f32b0`, `0x872acc`, `0x872afc`, `0x802970`
Confidence: verified
Proof: one
Test: none

`0x702af0` is the function lookup: a function at 1 (`lua_type == 6`, `0x702b04`) is pushed; otherwise the level is `__ftol` of the
optional number (default 1.0, `0x702b27`), `lua_getstack` and `lua_getinfo` with the option `f` (`0x872acc`) push the function at that level,
and a nil there raises (`0x702b8a`). `0x702ba0` pushes the function's environment (`0x6f3d50`), the six-byte string `__fenv`
(`0x872afc`), raw-gets it and answers whether the result is not nil. `getfenv` (`0x702ac0`) pops that result only when it is nil. `0x702be0`
compares argument 1, when numeric, with 0.0 (`0x702c45`, `test ah,0x44`, `jp` taken for any nonzero or NaN): equal sends the table to
`0x6f32b0` at index −10001, the globals. Otherwise `0x6f40d0` sets the environment and a 0 answer raises (`0x702c78`).

## Iteration

`next(t [, key])` requires a table and answers the key and value after `key`, or nil. `pairs(t)` requires a table and
returns the global `next`, the table and nil. `ipairs(t [, i])` called with one argument answers `(ipairs, t, 0)`; with
an index it adds 1.0, pushes the new index and `t[__ftol(index)]`, and returns both when the element is not nil, nothing
otherwise. `unpack(t)` requires a table, takes its length from `0x6f5050`, raises `table too big to unpack` when the stack
cannot grow by that count and pushes `t[1]` to `t[n]`.

Addresses: `0x702c90`, `0x702cb2`, `0x702cd0`, `0x702cf7`, `0x702d10`, `0x702d7b`, `0x702d8a`, `0x702da0`, `0x7030a0`, `0x7030b8`, `0x7030c8`, `0x7030d6`, `0x6f4450`, `0x6fa2a0`, `0x6f5050`, `0x6f4b20`, `0x8719b0`, `0x8728a4`, `0x872bc4`, `0x8015b8`
Confidence: verified
Proof: one
Test: none

`0x702c90` sets the top to 2 and calls `0x6f4450` (which calls `0x6fa2a0`): 2 results, else nil and 1. `0x702da0` pushes the
four-byte `next` (`0x8719b0`), raw-gets it from the globals (`0xffffd8ef` is −10001), pushes argument 1 and nil and returns
3, so a script that replaces the global `next` changes what `pairs` returns. `0x702cd0` compares argument 2 as a number with
0.0 (`0x702cf7`): equal, with `lua_type(2)` of −1 (`0x702d10`), it pushes the six-byte `ipairs` (`0x8728a4`), raw-gets it and
returns 3. It returns 2 only for a non-nil element (`neg`, `sbb`, `and eax,2` at `0x702d8a`). `0x7030a0` loops `lua_rawgeti`
from 1 to n (`0x7030d6`).

## Type and tostring

`type(v)` requires an argument and answers the name of its tag: `nil`, `boolean`, `userdata`, `number`, `string`,
`table`, `function`, `userdata` for tag 7 and `thread`; an absent index would answer `no value`. `tostring(v)` answers
the metatable's `__tostring` result when there is one; otherwise a number as its text, a string as itself, `nil`, `true`
or `false`, and for the rest `table: %p`, `function: %p`, `userdata: %p` (also for a light userdata) or `thread: %p`
with the object's address.

Addresses: `0x702df0`, `0x702f40`, `0x702f5f`, `0x702f81`, `0x703078`, `0x6f3480`, `0x6f4d60`, `0x811cd0`, `0x872bb8`, `0x871624`
Confidence: verified
Proof: one
Test: none

`0x702df0` takes `lua_type` through `0x6f3480`, which indexes the name table `0x811cd0` and answers the string at `0x871624` for
−1. `0x702f40` tries the `__tostring` metamethod (`0x702f5f`, string `0x872bb8`) and jumps through the nine-entry table at
`0x703078` by tag: nil, boolean, light userdata, number, string, table, function, userdata, thread.

## tonumber

`tonumber(v [, base])` takes the base as `__ftol` of the optional number, default 10. Base 10 requires an argument and
answers its number value when `lua_isnumber` holds, else nil. Another base must be 2 to 36 (`base out of range`) and `v`
a string: `strtoul` (`0x40dd0e`, which calls `_strtoxl` `0x40db09` with the unsigned flag) parses it, and the answer is the unsigned value when nothing but white space follows the digits, else
nil. A string with no digits answers nil.

Addresses: `0x702e30`, `0x702e52`, `0x702ead`, `0x702ec6`, `0x702ed3`, `0x702f02`, `0x702f11`, `0x40dd0e`, `0x40db09`, `0x40db59`, `0x40dbc6`, `0x40dccc`, `0x40dcf4`, `0x73fc64`
Confidence: verified
Proof: one
Test: none

`0x702e30` compares the base with 10 (`0x702e52`), range-checks 2 to 0x24 (`0x702ea3`, `0x702ead`), calls `0x40dd0e` with the
end pointer (`0x702ec6`), answers nil when the end equals the start (`0x702ed3`), skips bytes while `0x73fc64` holds and
requires the terminator (`0x702f02`). The result is the `fild` of a quad whose high dword is 0 (`0x702f11`), so values past
2^31 stay positive. A leading `-` negates the parsed value modulo 2^32 (`0x40db59`, `0x40dcf4`), so `tonumber("-1", 16)` is 4294967295. An overflow returns 0xffffffff (`0x40dccc`), so `tonumber("fffffffff", 16)` is 4294967295, but with a `-` the overflow value is negated and gives 1. In base 16 a `0x` or `0X` prefix is skipped (`0x40dbc6`).

## Raw access

`rawequal(a, b)` requires both arguments and pushes whether they are primitively equal. `rawget(t, k)` requires a table
and a key and pushes `t[k]` without metamethods. `rawset(t, k, v)` requires a table, a key and a value, sets `t[k]` without
metamethods and returns the table.

Addresses: `0x7030f0`, `0x703112`, `0x703130`, `0x703152`, `0x703160`, `0x70318e`, `0x6f3550`, `0x6f3b00`, `0x6f3ea0`
Confidence: verified
Proof: one
Test: none

`0x7030f0` calls `0x6f3550(1, 2)` and `lua_pushboolean`. `0x703130` is `luaL_checktype(1, 5)`, `luaL_checkany(2)` and `0x6f3b00(1)`;
`0x703160` adds `luaL_checkany(3)` and calls `0x6f3ea0(1)`. Both return 1.

## The collector pair

`gcinfo()` returns two numbers: the word at `+0x28` of the state's global record shifted right by 10, then the one at `+0x24`
shifted right by 10. `collectgarbage([limit])` passes `__ftol` of the optional number (default 0) to
`lua_setgcthreshold` and returns nothing.

Addresses: `0x703200`, `0x703207`, `0x703221`, `0x703250`, `0x703261`, `0x70326a`, `0x6f43f0`, `0x6f43e0`, `0x6f4400`
Confidence: verified
Proof: one
Test: none

`0x6f43f0` reads `[[L+0x10]+0x28] >> 10` and `0x6f43e0` reads `[[L+0x10]+0x24] >> 10`; `0x703200` pushes each through `fild`
and `lua_pushnumber` and returns 2. `0x703250` passes the `__ftol` result to `0x6f4400` (`luastate#the-collector-switch`).

## Time from a table

`time()` returns the current C time. `time(t)` needs a table with `sec` and `min` (default 0), `hour` (default 12), and
`day`, `month` and `year` (required: ``field `%s' missing in date table``), and reads `isdst` as a boolean. `month` is
reduced by 1 and `year` by 1900, `mktime` runs on them, and the answer is the number, or nil for −1. Each field is read with `lua_gettable` (`0x6f3a40`, so `__index` is honoured), and a field that is not a number or numeric string (`lua_isnumber`, `0x70370f`) counts as absent; `isdst` is `lua_toboolean`, so it is 0 or 1, never −1.

Addresses: `0x7035a0`, `0x7035b9`, `0x7035fc`, `0x703672`, `0x7036b0`, `0x7036f0`, `0x70370f`, `0x6f3a40`, `0x73f049`, `0x74066b`
Confidence: verified
Proof: one
Test: none

`0x7035a0` calls `time(0)` (`0x73f049`) when `lua_type(1)` is not above 0, otherwise fills the nine-int `tm` at `[ebp-0x28]`
through `0x7036f0` (name and default; the default −2 is "required"), reads `isdst` through `0x7036b0`, and calls `_mktime`
(`0x74066b`) at `0x703672`; `wday` (`[ebp-0x10]`) and `yday` (`[ebp-0xc]`) are never written, so they hold whatever the stack held; the C runtime's `mktime` is not read here.

## Date

`date([format [, time]])` takes `%c` and −1 as defaults, uses the current C time for −1, and breaks the time down as UTC when
the format starts with `!`, local otherwise. The format `*t` answers a table of `sec`, `min`, `hour`, `day`, `month` (+1),
`year` (+1900), `wday` (+1), `yday` (+1) and `isdst`; any other format goes to `strftime` with a 256-byte buffer, and a 0
result raises ``` `date' format too long```. An empty result is also 0, so `date("")` raises it too.

Addresses: `0x7033a0`, `0x7033d6`, `0x7033db`, `0x7033f0`, `0x7033f9`, `0x70340e`, `0x70343c`, `0x703503`, `0x70352c`, `0x703540`, `0x703570`, `0x73f125`, `0x7421da`, `0x4134e3`, `0x872c44`
Confidence: verified
Proof: one
Test: none

`0x7033a0` reads the time as `__ftol` of the optional number with default −1.0 (`0xbff00000`, `0x7033d6`) and calls `time(0)` for −1
(`0x7033e5`); a leading `!` (`0x7033f0`) selects `gmtime` (`0x7421da`) and skips the byte, else `localtime` (`0x73f125`); a null
result answers nil. `repz cmps` of three bytes against `*t\0` (`0x70343c`) selects the table; `0x703540` and `0x703570` set a number
and a boolean field. Otherwise `0x4134e3` (a wrapper that pushes 0 and forwards its four arguments to `0x4134fe`) formats into the
0x100-byte buffer (`0x703503`).
