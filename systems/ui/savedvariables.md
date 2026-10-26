# ui: saved variables

How the client reads each add-on's saved variables when it loads and the flat
`SavedVariables.lua`, and how it writes them back at shutdown: the files, the order, and
the text each value becomes. The add-on files are read and written by `AddOns.cpp`
(`ui/addons`), the flat file and the shutdown are in `GameUI.cpp`, and each variable's text is made
by `FrameScriptSerializeValue` (`0x704480`), which formats numbers through Storm and the C
runtime (`crt/printf`).

Roots: `0x51f650`, `0x4913b0`, `0x491730`, `0x4884e0`

## Saved variables, read

An add-on's account file `WTF\Account\<account>\SavedVariables\<Name>.lua` and then its character
file `WTF\Account\<account>\<realm>\<character>\SavedVariables\<Name>.lua`, or the realm-less
`WTF\Account\<account>\<character>\SavedVariables\<Name>.lua` when that is missing, run as Lua
after the add-on's own files and before its `ADDON_LOADED`, so their values replace what the files
set. They run in the shared state under the add-on's context word, and an error in one goes to the
error handler with nothing logged. The flat `WTF\Account\<account>\SavedVariables.lua` is not run:
each line `name = value` sets a string, from a quoted value, nil for an empty one, or a number
read as a single-precision float; other lines are ignored. `VARIABLES_LOADED` follows, file or not.

Addresses: `0x51f491`, `0x51f4a2`, `0x51f4b5`, `0x51f4df`, `0x51f518`, `0x51f53b`, `0x8539d4`, `0x8539a8`, `0x853980`, `0x4913b0`, `0x4913c4`, `0x4913e6`, `0x49144a`, `0x4914bf`, `0x4914ef`, `0x491534`, `0x491541`, `0x49156a`, `0x491580`
Confidence: verified
Proof: one
Test: none

`0x51f491` formats `0x8539d4`, `0x51f4a2` tests existence and `0x51f4b5` runs it with no MD5 and no
status; `0x51f4df` and `0x51f518` format the two character paths and `0x51f53b` runs the one
found. `0x4913b0` builds the path (`0x4913c4`), reads it (`0x4913e6`) and splits it at CR and LF;
the name ends at a space or `=` (`0x49144a`). A value beginning `"` (`0x4914bf`) is cut at its
last `"` and set with `0x704300` (`0x4914ef`); one beginning with a digit, or `-` or `.` then a
digit, goes through `atof` and `fstp dword` (`0x491534`) to `0x704250` (`0x491541`). `0x49156a`
signals 0x1ae on every path.

## Saved variables, written

The saved-variables files are written only in the interface's shutdown, after
`PLAYER_LEAVING_WORLD` and `PLAYER_LOGOUT`: the flat file, then each loaded add-on's files.
`AddOns.txt` follows them there, and the glue's `SaveAddOns` writes it too. For each loaded
add-on its account file and character file are rewritten from its declared names, an existing file
first renamed to `.bak`, each name read from the globals with the context word 0; with no names the
file is deleted. The realm-less character file is deleted. A written add-on is then marked not
loaded. The flat file holds the names `RegisterForSave` collected, which raises unless called with
context 0.

Addresses: `0x490bd0`, `0x490c20`, `0x490c2a`, `0x490c7e`, `0x490c83`, `0x490c88`, `0x51f650`, `0x51f711`, `0x51f74b`, `0x51f794`, `0x51f7c5`, `0x51f7e1`, `0x51f7fc`, `0x51f859`, `0x51f8eb`, `0x51f937`, `0x51f94f`, `0x51f9ac`, `0x51f9f7`, `0x51fa03`, `0x491730`, `0x4917a1`, `0x491812`, `0x4884e0`, `0x48851c`, `0x4915d0`, `0x8421a8`
Confidence: verified
Proof: one
Test: none

`0x490bd0` calls `0x490a80` (`0x490c20`), signals `PLAYER_LOGOUT` (`0x490c2a`), then `0x491730`
(`0x490c7e`), `0x51f650` (`0x490c83`) and `0x51ef20` (`0x490c88`). `0x51f650` skips records
with `+0x18` 0 (`0x51f711`); a nonzero `+0x6c` (`0x51f74b`) renames (`0x51f794`), creates
(`0x51f7c5`), stores context 0 (`0x51f7e1`) and serializes each name (`0x51f7fc`); zero deletes
(`0x51f859`). The character file repeats this with `+0x7c` (`0x51f8eb` to `0x51f9ac`); `0x51f9f7`
deletes the realm-less file and `0x51fa03` clears `+0x18`. `0x491730` renames (`0x4917a1`) and
serializes each registered name (`0x491812`); `RegisterForSave` adds a name with `0x4915d0`
(`0x48851c`).

## The saved variables format

Each variable is CR LF, its name, ` = ` and its value; the flat and account files end with CR LF,
a character file with CR alone. Nil and booleans are written as such, other numbers as
`#saved-numbers` says, a NaN of either sign as `nil --[[ nan ]]` and either infinity as
`nil --[[ inf ]]`. Strings are quoted, escaping only `\`, `"`, LF as `\n` and NUL as `\000`.
Functions, userdata and threads are written as `nil` with a comment.

Addresses: `0x7043f0`, `0x704422`, `0x704448`, `0x704455`, `0x872ce0`, `0xceeb60`, `0x51f80d`, `0x491846`, `0x51f960`, `0x704480`, `0x704a40`, `0x7044e9`, `0x704507`, `0x40a1b8`, `0x40a1a4`, `0x872e40`, `0x872e30`, `0x704560`, `0x70457f`, `0x704a5c`, `0x704a6c`, `0x70459e`, `0x7049a9`, `0x7049f0`
Confidence: verified
Proof: one
Test: none

`0x7043f0` writes `0x82edfc` (2 bytes), the name and `0x872ce0`, then calls `0x704480` at depth 1,
building text in `0xceeb60`;
`0x51f80d`, `0x491846` and `0x51f960` pass 2, 2 and 1 bytes for the end. `0x704480` switches on
the type through `0x704a40`; a number takes `0x872e40` when `_isnan` (`0x7044e9`) returns
nonzero and `0x872e30` when `_finite` (`0x704507`) returns 0. `_isnan` (`0x40a1b8`) returns 1 when
the top word's bits 0x7ff8 are 0x7ff8, or are 0x7ff0 with a nonzero fraction; `_finite`
(`0x40a1a4`) when its bits 0x7ff0 are not all set. Strings open with `"` (`0x704560`), map bytes up
to `\` through `0x704a6c` and `0x704a5c` (`0x70457f`) and flush at 1,016 bytes (`0x70459e`). Functions are named through
`lua_getinfo` (`0x7049a9`); other types take `0x7049f0`.

## Saved numbers

A finite number is written as `%.16g` by `SStrPrintf`, which passes the spec unchanged to
`_snprintf`. The `g` form takes 17 digits from `I10_OUTPUT`, rounded on an 18th and
made with a 96-bit product and a truncated 0.1, not from the value's exact digits (`crt/decimal`),
and rounds them to 16 on the 17th, a next digit of 5 or more rounding the magnitude up each time.
So the exact digits rounded twice miss cases; `1234567890123456.5` is written `1234567890123457`.
X, the rounded digits' decimal exponent less one, below -4 or of 16 or more takes the e form with
a signed three-digit exponent (`1e+016`), else the f form (`0.0001`); trailing fraction zeros go,
and a bare point. `-0` is `0`.

Addresses: `0x70451d`, `0x704529`, `0x70452c`, `0x70453b`, `0x872e28`, `0xceeb60`, `0x402141`, `0xc53f10`, `0x64a820`, `0x6697d0`, `0x669b81`, `0x6698f6`, `0x669a27`, `0x740107`, `0x742f4f`, `0x742f87`, `0x40b76b`, `0x40b7b3`, `0x40b7c2`, `0x40ff90`, `0x4116fd`, `0x411930`, `0x411968`, `0x40fef9`, `0x40ff3d`, `0x40b5ad`, `0x40b6c4`, `0x40b4a8`, `0x8001e0`, `0x410024`, `0x411774`, `0x6f3642`, `0x41171e`, `0x412ef1`, `0x831714`
Confidence: verified
Proof: two
Test: `crt::crt_sprintf_saved_number_difftest`, `crt::crt_sprintf_float_difftest`, `storm::vsnoprintf_difftest`

`0x70451d` reads the number with `lua_tonumber`, whose `fld` (`0x6f3642`) `0x704529` stores, and
`0x70452c` pushes `0x872e28`, 0x400 and the buffer `0xceeb60` for `SStrPrintf` (`0x70453b`).
`0x402141` sets option 10 (`0xc53f10`), so `0x64a820` calls `vsnoprintf` (`0x6697d0`), which classes
`g` as a double (`0x669b81`), copies it at `0x6698f6` and calls `_snprintf` at `0x669a27`. `__output`
calls `_cfltcvt` (`0x742f4f`) and `_cropzeros` (`0x742f87`); `_fltout2` asks `I10_OUTPUT` for 17
digits (`0x40ff90`), rounded at `0x411930`, and `_fptostr` rounds at `0x40ff3d`. `_cftog` compares
the exponent at `0x40b7b3`. Either zero widens to ten zero bytes (`0x410024`) and is the digit `0`
with a space (`0x411774`). Only `fld` and `fstp` touch the value, so the precision control
changes nothing. `crt/printf` and `crt/decimal` give each step; the point is `0x831714`, `.` in
the C locale the client keeps (`crt/printf`). The test runs `sprintf` whole over 18 values with `%.16g`,
among them `5e-324` (`4.940656458412465e-324`), and `%.17g` of `235953666403114.375`, whose 17
digits end `37` where the value's own end `38`, against the expected text and the model.

## Saved tables

A table is written as `{` and CR LF, each entry one tab deeper as `[key] = value,` and CR LF, then
`}` at the parent's indent, in `next` order. A table already open on the current path is
`nil --[[ skipped recursive table ]]`, so a table reached twice by different paths is written
twice. A frame's table is its name, unquoted. Entries keyed by NaN, an infinity, a table, a
function or userdata are skipped with a comment.

Addresses: `0x704607`, `0x704613`, `0x704642`, `0x704670`, `0x7046bc`, `0x7046e7`, `0x704727`, `0x704744`, `0x704766`, `0x70488f`, `0x7048a3`, `0x704928`, `0x70497b`, `0x8722d4`
Confidence: verified
Proof: one
Test: none

A table first reads element 0 as a light userdata and writes `GetName` (`0x704613`, `0x704642`).
Otherwise the light userdata whose reference is `0x8722d4` marks it as a key (`0x704670`,
`0x7046bc`) until `0x70497b` removes the mark on the way out. `0x7046e7` writes the brace; the
indent grows by a tab per depth up to 128 (`0x704727`); `lua_next` walks the entries (`0x704744`).
Keys dispatch at `0x704766`, a light userdata equal to the mark being dropped (`0x70488f`) and
others reaching the skip comment (`0x7048a3`); `0x704928` writes the closing brace.
