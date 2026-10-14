# glue: the script functions

The Lua functions of the glue screens, the glue event names, and the helpers in the same file.

From `0x46c6c0` through `0x46de20`, after the glue manager, lie the glue's own script
functions: 60 Lua C functions paired with their names in the table `0x8373b8`, which `0x46ddf0`
sets as Lua globals and `0x46de20` clears. Each takes the Lua state in `ecx` and returns the
count of values it pushed. Below, a "string"
argument is one `lua_isstring` accepts (a string or a number), a "number" one `lua_isnumber`
accepts (a number or a numeric string), a usage error is `luaL_error` with the binding's usage
text, which does not return, and a "flag" result is the number 1 or nil.

The range also holds the event-name filler `0x46cc60`, four list helpers and a message printf
that other systems call too, and the destructors and default slots of the login listener, a class
defined inline in `WowSvcs/WowSvcsClientServices.h`.

Roots: `0x46c6c0`, `0x46c6f0`, `0x46cab0`, `0x46cad0`, `0x46cb00`, `0x46cc60`, `0x46ddf0`, `0x46de20`, the vtable slots `0x46cba0`, `0x46cbe0`, `0x46cbf0`, `0x46cc00`, and the 60 functions of the table 0x8373b8: `0x46cd70`, `0x46cdd0`, `0x46cdf0`, `0x46ce40`, `0x46ce60`, `0x46cea0`, `0x46ceb0`, `0x46cef0`, `0x46cf00`, `0x46cf10`, `0x46cf60`, `0x46cfa0`, `0x46cfd0`, `0x46d000`, `0x46d010`, `0x46d040`, `0x46d070`, `0x46d080`, `0x46d0b0`, `0x46d0e0`, `0x46d0f0`, `0x46d120`, `0x46d150`, `0x46d160`, `0x46d1c0`, `0x46d270`, `0x46d280`, `0x46d340`, `0x46d380`, `0x46d3c0`, `0x46d3d0`, `0x46d3e0`, `0x46d400`, `0x46d410`, `0x46d420`, `0x46d460`, `0x46d5e0`, `0x46d650`, `0x46d6f0`, `0x46d7b0`, `0x46d850`, `0x46d8a0`, `0x46d940`, `0x46d990`, `0x46d9a0`, `0x46d9b0`, `0x46d9e0`, `0x46da10`, `0x46da50`, `0x46dad0`, `0x46db60`, `0x46db70`, `0x46db80`, `0x46dbc0`, `0x46dbf0`, `0x46dc40`, `0x46dcf0`, `0x46dd30`, `0x46dd80`, `0x46ddd0`

## Binding table and registration

`0x8373b8` holds 60 eight-byte pairs, the name first and the function second. `0x46ddf0` passes
each pair to `0x704120`, which sets the Lua global of that name to the function; `0x46de20`
passes each name to `0x704160`, which sets that global to nil. `0x46abb0` calls the first and
`0x46ace0` the second, each among five other table registrars.

Addresses: `0x8373b8`, `0x46ddf0`, `0x46de20`, `0x704120`, `0x704160`, `0x46abb0`, `0x46ace0`
Confidence: verified
Proof: one
Test: none

`0x46ddf0` loads the pair's second dword into `edx` and `ecx = [esi+0x8373b8]` (`0x46ddf3`, `0x46ddf9`),
calls `0x704120`, adds 8 and loops while `esi < 0x1e0`, 480 bytes; `0x46de20` loads the name alone
and calls `0x704160`. `0x704120` pushes a C closure of the function with no upvalues, then the
name, moves the name under the closure and stores the pair into the table at pseudo-index −10001,
the globals; `0x704160` pushes nil in place of the closure. The calls are at `0x46abba` and
`0x46acea`. `tables/bindings.tsv` lists the pairs.

## Glue event names

`0x46cc60` stores the 26 glue event names into `0xb41e70`, one string pointer per id, from 0
(`SET_GLUE_SCREEN`) to 25 (`PLAYER_ENTER_PIN`), as `tables/glue_events.tsv` lists them. The
client's startup `0x402ad0` calls it once; `0x46a7b0` passes the array to the script system at
`0x46a88f`. Nothing else writes the array. It is not a C runtime initializer: nothing stores its
address.

Addresses: `0x46cc60`, `0xb41e70`, `0x402ad0`, `0x402af8`, `0x46a7b0`, `0x46a88b`, `0xb41ebc`, `0xb41ec0`
Confidence: verified
Proof: one
Test: none

26 `mov dword [0xb41e70 + 4·id], imm32` from `0x46cc60` to `0x46cd5a`, then `ret`; each immediate
is the name string of that id in `tables/glue_events.tsv`. Census of each element's address as a
little-endian dword over the image: elements 1 to 25 appear once, in their store; `0xb41e70`
appears at `0x46cc62` and at `0x46a88b`, the push before the call at `0x46a88f`. `0x46cc60`
itself is never stored; one `e8` reaches it, at `0x402af8` among `0x402ad0`'s startup calls. Control: the same scans find `0x46ddf0`'s one call at `0x46abba`. Ids 19 and 20
(`0xb41ebc`, `0xb41ec0`) have no glue fire site in `tables/event_sites.tsv`; id 18 has two.

## Screens, build and quitting

These read at most one string.

| function | argument | effect | returns |
|---|---|---|---|
| `GetBuildInfo` `0x46cd70` | none | none | 5 strings: the script texts of `BETA_BUILD` and `RELEASE_BUILD`, then `1.12.1`, `5875`, `Sep 19 2006` |
| `GetLocale` `0x46ce40` | none | none | the client locale's name, such as `enUS` |
| `SetCurrentScreen` `0x46ce60` | string | `0x46b860` copies at most 63 bytes and a NUL into `0xb41478`; then every pending asynchronous file read is waited for | none |
| `QuitGame` `0x46cea0` | none | `0x46b870`, the client's quit with 0 | none |
| `StatusDialogClick` `0x46d270` | none | `0x46b720` | none |
| `EnterWorld` `0x46d3c0` | none | `CGlueMgr::EnterWorld` `0x46b500` | none |

Addresses: `0x46cd70`, `0x46ce40`, `0x46ce45`, `0x46ce60`, `0x46ce91`, `0x46ce96`, `0x46b860`, `0xb41478`, `0x46cea0`, `0x46b870`, `0x46d270`, `0x46b720`, `0x46d3c0`, `0x46b500`
Confidence: verified
Proof: one
Test: none

`GetBuildInfo` looks up `BETA_BUILD` (`0x837c1c`) and `RELEASE_BUILD` (`0x837c0c`) through the
script text lookup at `0x46cd7d` and `0x46cd95`, with `edx` −1 and 0 pushed, then pushes three
literals with `lua_pushstring` and returns `mov eax,5`. `GetLocale` loads the locale index global
and indexes a table of name pointers at `0x46ce45` (`enUS`, `koKR`, `frFR`, `deDE`, ...).
`SetCurrentScreen` tests argument 1 with `lua_isstring` (usage text `0x837c54`); `0x46b860` pushes
0x40, the name and `0xb41478` to Storm's string copy, which stops one byte short of the limit and
writes the NUL; the call at `0x46ce96` is the wait whose log lines read `AsyncFileReadWaitAll()
Enter` and `Leave`. `0x46b870` is `xor ecx,ecx` and a jump to the client's quit routine.
The other three are a call and `xor eax,eax`.

## Agreement notices

Four agreements share one mechanism: a CVar and two flags. At startup `0x46a400` sets
the accepted flag to whether the CVar's integer is 1, and sets the show flag to 1 when the
integer is negative; otherwise the show flag keeps its load-time zero. `Show…Notice` returns
the show flag and `…Accepted` the accepted flag, each as a flag. `Accept…` sets accepted to 1,
show to 0 and the CVar to `"1"`, and returns nothing. None reads an argument. The login refuses
to start until the TOS and the EULA are accepted (`Login and account name`).

| agreement | CVar | show | accepted | Show…Notice, …Accepted, Accept… |
|---|---|---|---|---|
| TOS | `readTOS` | `0xb41df8` | `0xb41dfc` | `0x46cfa0`, `0x46cfd0`, `0x46d000` → `0x46aef0` |
| EULA | `readEULA` | `0xb41e00` | `0xb41e04` | `0x46d010`, `0x46d040`, `0x46d070` → `0x46af20` |
| Scanning | `readScanning` | `0xb41e08` | `0xb41e0c` | `0x46d080`, `0x46d0b0`, `0x46d0e0` → `0x46af50` |
| Contest | `readContest` | `0xb41e10` | `0xb41e14` | `0x46d0f0`, `0x46d120`, `0x46d150` → `0x46af80` |

Addresses: `0x46a400`, `0x46a41a`, `0x46a426`, `0x46a42e`, `0x46aef0`, `0x46af20`, `0x46af50`, `0x46af80`, `0x46afb1`, `0x46afc1`
Confidence: verified
Proof: one
Test: none

One call, at `0x402b36`, runs `0x46a400`. `0x46a41a` loads `readTOS`'s integer at `+0x28` of its record; `cmp eax,1; sete` goes to
`0xb41dfc` (`0x46a426`), and `cmp eax,0; jge` skips the store of 1 to `0xb41df8` at `0x46a42e`.
The other three repeat the pattern at `0x46a43a`, `0x46a45a` and `0x46a47a`. Each getter loads
its flag and `test; je` to `lua_pushnil`; otherwise it pushes the double 1.0; both paths return 1. `0x46aef0` stores 1 to `0xb41dfc` and 0 to `0xb41df8`, then
calls `CVar::Set` on `readTOS` with `"1"`; its three siblings do the same on their pairs. Census
of each flag's address: the seeding store, the accept's store and the getter's load, plus, for
`0xb41dfc` and `0xb41e04`, the login's tests at `0x46afb1` and `0x46afc1`. Control: the getters' loads.

## Glue music

`PlayGlueMusic` (`0x46ceb0`) passes its string, the music file, to the sound system's glue
music setter. `PlayCreditsMusic` (`0x46cef0`) calls its credits music start, and `StopGlueMusic`
(`0x46cf00`) its glue music stop with the float 1.0. None returns a value.

Addresses: `0x46ceb0`, `0x46cee1`, `0x46cef0`, `0x46cf00`, `0x46cf05`
Confidence: verified
Proof: one
Test: none

`0x46ceb0` tests argument 1 with `lua_isstring` (usage `0x837c78`), converts it and calls the
setter with it in `ecx` at `0x46cee1`. `0x46cef0` is a call and `xor eax,eax`. `0x46cf00` pushes
the float bits of 1.0 and calls the stop at `0x46cf05`, which takes one stack argument.

## Movie resolution and subtitles

`GetMovieResolution` (`0x46cf10`) scans the `gxResolution` CVar's string as `"%dx%d"` and returns
the width as a number; without a leading integer the width slot is never written, and the result
is whatever the stack held. `GetMovieSubtitles` (`0x46dbc0`) returns the flag `movieSubtitle ≠ 0`.
`SetMovieSubtitles` (`0x46db80`) sets `movieSubtitle` to `"1"` or `"0"` by its argument, read by
the flag reader `0x6f1c10` with default 1:

| argument | reads as |
|---|---|
| absent, light userdata, table, function, userdata, thread | the default |
| nil | false |
| boolean | its value |
| number | its `__ftol` nonzero, so 0.5 is false |
| string starting `0`, `F`, `f`, `N` or `n` | false |
| string starting `1` to `9`, `T`, `t`, `Y` or `y` | true |
| any other string | false for `off`, `disabled`; true for `on`, `enabled`; else the default |

Addresses: `0x46cf10`, `0x837c9c`, `0x46cf39`, `0x46dbc0`, `0x46db80`, `0x6f1c10`, `0x6f1c27`, `0x6f1c72`, `0x6f1c81`
Confidence: verified
Proof: one
Test: none

`0x46cf10` looks up `gxResolution`, passes its string at `+0x20` to `_sscanf` with `0x837c9c` and
the slots `[ebp-0x4]` (first) and `[ebp-0x8]`, neither initialized, and converts `[ebp-0x4]` with
`fild` at `0x46cf39`, exactly. `0x46dbc0` tests `[record+0x28]`. `0x46db80` calls `0x6f1c10` with
argument index 1 and 1 pushed, then `CVar::Set` with `"1"` or `"0"`. In `0x6f1c10`, `lua_type`
indexes the jump table at `0x6f1c27` for types 0 to 4 (`ja` sends −1 and 5 to 8 to the default):
nil returns 0, a boolean `lua_toboolean`, light userdata the default, a number `__ftol` of
`lua_tonumber`. A string's first byte minus `'0'` indexes the byte map at `0x6f1c72` over `'0'` to
`'y'`; out of range, or a byte mapped to 2, compares the string with `off`, `disabled`, `on`,
`enabled` in turn from `0x6f1c81`.

## URLs

`LaunchURL` (`0x46cf60`) takes a string and passes it to `0x5abd00`, which opens it in the
browser only when net's URL check accepts it. `LaunchAddOnURL` (`0x46d5e0`) takes an add-on index
(`Add-on list`) and, when that add-on has a URL, hands it to the browser opener with no check.
Neither returns a value.

Addresses: `0x46cf60`, `0x46cf91`, `0x5abd00`, `0x46d5e0`, `0x46d63a`, `0x46d645`
Confidence: verified
Proof: one
Test: none

`0x46cf60` tests argument 1 with `lua_isstring` (usage `0x837ca4`) and calls `0x5abd00` at
`0x46cf91`. `0x46d5e0` resolves its index as every add-on index is resolved (usage `0x837db8`),
gets the URL by name at `0x46d63a`, and calls the opener at `0x46d645` only for a non-null string.

## Login and account name

`GetSavedAccountName` (`0x46cdd0`) returns the `accountName` CVar's string. `SetSavedAccountName`
(`0x46cdf0`) sets that CVar to its string argument. `DefaultServerLogin` (`0x46d160`) takes the
account and the password, both strings, and calls `CGlueMgr::DefaultServerLogin` (`0x46afb0`),
which returns at once unless the TOS and EULA are accepted and the glue is idle. `PINEntered`
(`0x46d1c0`) reads arguments 1 to 10 up to the first nil or absent one; each must be a number,
else the usage error names its position; the count must be 4 to 10, else the usage error gives
the count. Each digit is the low byte of the number's `__ftol`; count and digits go to `0x46b0c0`.
The other three return nothing.

Addresses: `0x46cdd0`, `0x46cdf0`, `0x46ce2e`, `0x46d160`, `0x46d1a0`, `0x46afb0`, `0x46d1c0`, `0x46d20f`, `0x46d21b`, `0x46d22a`, `0x46b0c0`
Confidence: verified
Proof: one
Test: none

`0x46cdd0` pushes `[record+0x20]` and returns 1. `0x46cdf0` tests `lua_isstring` (usage
`0x837c28`) and calls `CVar::Set` at `0x46ce2e`. `0x46d160` tests arguments 1 and 2 (usage
`0x837cbc`), converts 2 then 1, and calls `0x46afb0` at `0x46d1a0` with the account in `ecx` and
the password in `edx`; `0x46afb0` first tests `0xb41dfc`, `0xb41e04` and the screen state. In
`0x46d1c0` the loop runs `esi` from 1 while below 11: `lua_type ≤ 0` ends it; `lua_isnumber` false
raises `0x837d34` with the position; the digit is stored at `0x46d20f` into a 20-byte buffer.
`0x46d21b` and `0x46d220` require 4 to 10, else `0x837cf4` with the count; `0x46d22a` calls
`0x46b0c0` with the count in `ecx` and the buffer in `edx`.

## Server name and connection

`GetServerName` (`0x46d280`) returns three values: the `realmName` CVar's string, then two flags
from the `Cfg_Configs` row whose `+0x04` equals the selected realm record's `+0x04`: the row's
`+0x08` nonzero, then its `+0x0c` nonzero; with no selected realm, an empty table or no match,
nil and nil. `IsConnectedToServer` (`0x46d380`) returns the flag of the world session's `+0x1b00`.
`DisconnectFromServer` (`0x46d340`) with that flag set stores 1 to `0xb41d98` and drops the world
connection; then, in every case, it cancels the logon on the login session, with no null test:
before the first logon creates that session, the cancel reads `+0x05` of a null pointer and
faults.

Addresses: `0x46d280`, `0xc0dfa0`, `0x46d2c0`, `0x46d303`, `0x46d380`, `0x46d340`, `0x46d34f`, `0x46d360`, `0x46d36c`, `0xb41d98`, `0x5ab7d0`, `0x5ab9a0`, `0x5ab1a0`, `0x5ab4a0`, `0x5b3320`, `0xc28120`
Confidence: verified
Proof: one
Test: none

`0x46d289` pushes `0x5ab7d0()`. `0x46d29c` calls `0x5ab9a0`; null skips to two nils. The walk
reads the count at `+0x04` and the records at `+0x00` of the store `0xc0dfa0`, whose loader reads
`DBFilesClient\Cfg_Configs.dbc`, in 0x14-byte steps, and stops at the first row whose `+0x04`
equals the record's `+0x04` (`0x46d2c0` to `0x46d2d9`). `0x46d303` sets `ecx = (+0x08 ≠ 0)`
and `ebx = (+0x0c ≠ 0)`; each pushes 1.0 or nil in turn; every path returns 3. `0x46d380` and
`0x46d340` read `[0x5ab490()+0x1b00]`. `0x46d34f` stores the 1 and `0x46d360` calls `0x5ab1a0`;
`0x46d36c` calls `0x5b3320` with `0x5ab4a0()`, which returns `[0xc28120]`. `0x5b3320` begins `mov
al,[ecx+0x5]`. `0xc28120` has no file bytes; the only store of a non-null value is `0x5ab567` in
`0x5ab4b0`, and `0x5ab447` and `0x5ab4ed` store 0.

## Add-on list

An add-on index argument must be a number, else the function's usage error; `__ftol` of it minus 1
selects from the list of add-on names, and anything outside 1 to the count, compared unsigned,
raises `AddOn index must be in the range of 1 to %d` with the count. A character argument must be
a string, a number or nil, nil meaning none; absent is the usage error. The functions:

| function | arguments | returns, effect |
|---|---|---|
| `GetNumAddOns` `0x46d420` | none | the count |
| `GetAddOnInfo` `0x46d460` | index | 8: name; title, notes and URL, each a string or nil; loadable flag; reason or nil; security; a flag |
| `GetAddOnDependencies` `0x46d650` | index | one string per dependency, none for none |
| `GetAddOnEnableState` `0x46d6f0` | character, index | the state the add-on code returns, a number |
| `EnableAddOn`, `DisableAddOn` `0x46d7b0`, `0x46d8a0` | character, index | sets state 2 or 0 |
| `EnableAllAddOns`, `DisableAllAddOns` `0x46d850`, `0x46d940` | character, unchecked | sets 2 or 0 for every index |
| `SaveAddOns`, `ResetAddOns` `0x46d990`, `0x46d9a0` | none | saves; reloads all, null passed |
| `IsAddonVersionCheckEnabled` `0x46d9b0` | none | the flag `checkAddonVersion ≠ 0` |
| `SetAddonVersionCheck` `0x46d9e0` | flag, default 1 | sets `checkAddonVersion` to `"1"` or `"0"` |

Addresses: `0x46d4a2`, `0x46d4a5`, `0x837d70`, `0x46d526`, `0x46d542`, `0x46d577`, `0x46d5a8`, `0x46d6c1`, `0x46d702`, `0x46d789`, `0x46d83f`, `0x46d92f`, `0x46d883`
Confidence: verified
Proof: one
Test: none

`dec eax` after `__ftol` (`0x46d4a2`); the lookup (`0x46d4a5`) tests `cmp ecx,count; jb`; the
range error is `0x837d70`, the usage texts `0x837d9c`, `0x837dd8`, `0x837dfc`, `0x837e2c`, `0x837e54`. `GetAddOnInfo` pushes the name, then the TOC's `Title` and `Notes` and a URL, nil for
null; the load check is called with `dl = 0`, no character (`0x46d526` to `0x46d542`), and leaves
two codes; when either is nonzero, `0x46d577` builds the reason in 128 bytes: the first code's
name from `LOADABLE`, `MISSING`, `DISABLED`, `BANNED`, `CORRUPT`, `INSECURE`, `NOT_DEMAND_LOADED`,
`INTERFACE_VERSION`, or, when it is 0, `DEP_` and the second code's name. Security
(`0x46d5a8`) is `SECURE`, `INSECURE` or `BANNED`; the eighth value is a byte at `+0xc0` of the
add-on's record. `GetAddOnDependencies` grows the stack for the count (`0x46d6c1`, result
ignored) and returns it. The character test (`0x46d702`) passes `lua_isstring`, or `lua_type =
0`. The state calls are at `0x46d789` (third argument 1), `0x46d83f` (2) and `0x46d92f` (0);
the all-loops re-read the count each pass (`0x46d883`).

## Script memory

`GetScriptMemory` (`0x46da10`) returns the `scriptMemory` CVar's integer, in KB, divided by 1024
with truncation toward zero, as a number: the default 49152 gives 48. `SetScriptMemory`
(`0x46da50`) takes a number of MB, else its usage error, and sets the CVar to the decimal text of
the low 32 bits of its `__ftol`, shifted left by 10 with wrap-around. It returns nothing.

Addresses: `0x46da10`, `0x46da1c`, `0x46da50`, `0x46da8d`, `0x46da9c`
Confidence: verified
Proof: one
Test: none

`0x46da1c` to `0x46da25`: `cdq; and edx,0x3ff; add eax,edx; sar eax,0xa`, then `fild` and
`lua_pushnumber`. `0x46da50` tests `lua_isnumber` (usage `0x837e7c`), `__ftol`, `shl eax,0xa`
(`0x46da8d`), and formats `%d` into 32 bytes with `SStrPrintf` (`0x46da9c`) before `CVar::Set`.

## Cursor and screen size

With `K1`, `K2` and `A = RN24(0.75·a)` the screen scales `0x41ad10` stores for the aspect `a`,
`RN24` rounding to float and `RN53` to double:

- `GetCursorPosition` (`0x46dad0`): from the cursor fractions `fx`, `fy` the interface root keeps,
  `x = RN53(RN24(1024·A·RN24(K1·fx)) / K1)` and `y = RN53(RN24(1024·A·RN24(K2·fy)) / K1)`; both
  divide by `K1`.
- `GetScreenWidth` (`0x46dd30`): `RN53(RN24(1024·A·K1) / K1)`.
- `GetScreenHeight` (`0x46dd80`): `RN53(RN24(1024·A·K2) / K1)`.

So `x ≈ 768·a·fx`, `y ≈ 768·fy`, the width `≈ 768·a` and the height `≈ 768`. With the file values
(`K1` 0.8, `K2` 0.6, `A` 1.0) they give 1024.0 and 768.0000190734861; at `a = RN24(4/3)`,
1024.0 and 767.9999427795418. `ShowCursor` (`0x46db60`) and `HideCursor` (`0x46db70`) call the
cursor switch `0x58b590` with 1 and 0.

Addresses: `0x46dad0`, `0x46dd30`, `0x46dd80`, `0x41ad10`, `0x832a44`, `0x832a48`, `0x832a4c`, `0xcf0bd8`, `0x41ad80`, `0x41ad70`, `0x41ae40`, `0x41ae60`, `0x41ae70`, `0x46db17`, `0x46db3b`, `0x46db60`, `0x46db70`, `0x58b590`, `0x46db05`, `0x46dd44`, `0x46dd94`
Confidence: verified
Proof: one
Test: `glue::lua_model_scale_difftest`

`0x46dad0` passes `[[0xcf0bd8]+0x1118]` and `+0x111c` to `0x41ad80`, which stores `K1·fx` and
`K2·fy` as floats. `0x41ad70` loads `A` from `0x832a4c`; `fmul` by 1024.0 and by the float are exact
at 53 bits, two 24-bit significands; `fstp dword` rounds to float; `0x41ae40` loads it and `fdiv`s
by `[0x832a44]`, rounding to 53 bits, and returns in `st0`; `fstp qword` stores it unchanged for
`lua_pushnumber`. The calls are at `0x46db17` and `0x46db3b`, both `0x41ae40`. The size functions
push 1.0 to `0x41ae60` (`K1·1`) or `0x41ae70` (`K2·1`), store that as a float and follow the same
steps. `0x41ad10` stores `a·0.75` rounded to float at `0x832a4c` (`fstp dword`, `0x41ad24`) and the
scales at `0x832a44` and `0x832a48`; a scan of the image for the dword `0x832a4c` finds only that
store and `0x41ad70`'s `fld dword`, and the same scan finds the nine uses of `0x832a48`; the
examples apply its arithmetic. `0x58b590` calls slot `+0xec` of the device object with the argument.
`lua_model_scale_difftest` runs `0x46db05` to `0x46db17`, `0x46dd44` to `0x46dd56` and `0x46dd94` to
`0x46dda6`.

## Billing

The three billing functions return numbers from the world session.

| function | returns |
|---|---|
| `GetBillingTimeRemaining` `0x46dbf0` | `+0x1af4`, unsigned; 0 when bit 1 of `+0x1afc` is set |
| `GetBillingPlan` `0x46dc40` | with `f` the byte `+0x1afc`: 1, 2, 3 or 4 for `f & 0x16` equal to 2, 4, 0 or 0x10, else 0; then `(f >> 5) & 1`; then `(f >> 3) & 1` |
| `GetBillingTimeRested` `0x46dcf0` | `+0x1af8`, unsigned |

Addresses: `0x46dbf0`, `0x46dc0a`, `0x46dc1f`, `0x46dc40`, `0x46dc5e`, `0x46dca8`, `0x46dcc5`, `0x46dcf0`, `0x5ab490`, `0x5b41b0`, `0x5b41fe`
Confidence: verified
Proof: one
Test: none

Each reads `0x5ab490()`. `0x46dc0a` `test byte [eax+0x1afc],2; je` keeps the time; `0x46dc1f`
`fild qword` over the dword and a zero high dword, so values from 2³¹ up stay positive.
`0x46dc5e` `and eax,0x16`, then the compares with 2, 4, 0 and 0x10; the plan starts at 0.
`0x46dca8` `shr cl,5; and cl,1`, `0x46dcc5` `shr bl,3; and bl,1`. The writer is net's
`0x5b41b0`: after result 0x0c or 0x1b with at least 5 bytes left, it reads a dword into `+0x1af4`,
a byte into `+0x1afc` and a dword into `+0x1af8` (`0x5b41fe` to `0x5b4223`).

## Screenshot, patch and survey

`Screenshot` (`0x46d3d0`) arms a screenshot with `0x46c520` as its callback, which fires glue
event 14 on success and 15 on failure. `PatchDownloadProgress` (`0x46d3e0`) returns the float
`0xb414c0` as a number. `PatchDownloadCancel` (`0x46d400`) calls `0x474710` and
`PatchDownloadApply` (`0x46d410`) `0x4749f0`. `SurveyNotificationDone` (`0x46ddd0`) reads a flag,
default 0: when true it sends a pending survey result (`0x473df0`); then it always clears the
survey state (`0x473e30`). It returns 1 without pushing, so its result is the top of the Lua
stack, the last argument when one is passed. The others return nothing.

Addresses: `0x46d3d0`, `0x46d3d5`, `0x46c520`, `0x46d3e0`, `0xb414c0`, `0x46d400`, `0x474710`, `0x46d410`, `0x4749f0`, `0x46ddd0`, `0x46ddd7`, `0x473df0`, `0x473e30`, `0x46ddea`
Confidence: verified
Proof: one
Test: none

`0x46d3d5` calls a one-store function that keeps `0x46c520` as the callback; `0x46c520` picks
`ecx` 0xe for a nonzero argument, else 0xf, and jumps to `FrameScript_SignalEvent`.
`0x46d3e0` `fld dword [0xb414c0]` and `fstp qword` for `lua_pushnumber`. `0xb414c0` has no file
bytes; its writers are `0x474b8a` (1.0) and `0x474c76`. `0x46ddd7` calls `0x6f1c10` with index 1
and 0 pushed; `0x473df0` sends when its data pointer and 16-bit size are both nonzero. `0x46ddea`
is `mov eax,1` with nothing pushed.

## List helpers and message printf

Four helpers work on intrusive lists: a link is a previous-link pointer and a next-node pointer,
and a pointer to the head link is marked by bit 0. `0x46c6c0` empties a list whose
nodes keep their link at `+0x08`. `0x46cab0` returns a node's link, or the head link for null.
`0x46cad0` returns the link of the next node, or the next pointer with bit 0 cleared when it is
marked or null; a negative link offset is recovered from the previous link. `0x46cb00` unlinks a link
and zeroes it. `0x46c6f0` formats a message and passes it on with its first argument.

Addresses: `0x46c6c0`, `0x46c6d4`, `0x46cab0`, `0x46cad0`, `0x46caee`, `0x46cb00`, `0x46c6f0`, `0x44cb90`
Confidence: verified
Proof: one
Test: none

`0x46c6c0` stores 8 at `+0x00`, the address of `+0x04` into `+0x04` and that address with bit 0
set (`0x46c6d4`) into `+0x08`; it returns the list. `0x46cab0`, `ret 4`: a null node gives `list
+ 4`, else `node + [list]`. `0x46cad0`, `ret 4`: `next = [link+4]`; bit 0 set or zero returns
`next & ~1`; else `next + offset`, where a negative offset becomes `link − [[link]+4]`
(`0x46caee`). `0x46cb00` returns at once for a null previous link; else it stores the previous
link into the next node's link and the next pointer into the previous link's `+0x04`, then
zeroes both. `0x46c6f0` calls `0x44cb90(a, 1, format, va)`, which formats into 256 bytes. Glue's
call (`0x46a8e3`) reports failing to create `Logs\GlueXML.log`.

## Login listener base and destructors

The login listener at `0xc28118`, created on first use, is an 0x14-byte object of vtable
`0x803808`, six slots: its
deleting destructor `0x46cba0` and five net functions. Its base vtable `0x803820` holds the
base's deleting destructor `0x46cc00`, the defaults `0x46cbe0` (returns false) and `0x46cbf0`
(does nothing) for slots 1 and 2, and `__purecall` for slots 3 to 5. Both destructors store
`0x803820`, free the realm records at `+0x0c` when present and, for bit 0 of their argument, the
object. Live code installs both vtables: the base's only inside `0x46f9e0` and the destructors.

Addresses: `0xc28118`, `0x803808`, `0x803820`, `0x46cba0`, `0x46cc00`, `0x46cbe0`, `0x46cbf0`, `0x40a2d7`, `0x46f9e0`, `0x46f9ed`, `0x46e561`, `0x46e566`, `0x5ab9b0`, `0x5abae0`
Confidence: verified
Proof: one
Test: none

`0x803808` holds `0x46cba0`, `0x5ab9b0`, `0x5abae0`, `0x5abb00`, `0x5abb10`, `0x5abb90`;
`0x803820` holds `0x46cc00`, `0x46cbe0`, `0x46cbf0` and three times `0x40a2d7`, `push 0x19;
call` the runtime's fatal exit. `0x46cbe0` is `xor al,al; ret 4`, slot 1's one argument as in
`0x5ab9b0`; `0x46cbf0` is `ret 8`, slot 2's two as in `0x5abae0`. The destructors free `+0x0c`
with the tag `.?AUREALM_INFO@@` and line −2, then the object with `delete` and −1, `ret 4`.
Census of `0x803808`: eight stores, each after a 0x14-byte allocation tagged with the header's
path and line 210, ending in a store to `0xc28118`; seven zero `+0x04` to `+0x0c` inline, and at
`0x46e561` the call to `0x46f9e0` does, storing `0x803820` at `0x46f9ed` before `0x46e566`.
Census of `0x803820`: that store and the two destructors'.
