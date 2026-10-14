# ui: CVar, option and engine information verbs

The Lua functions at the head of `GameUI.cpp` (`0x4884a0` to `0x489440`, `0x48b8b0`, `0x48d8b0`): reading and writing CVars
from scripts, the detail sliders that map onto CVars, the modifier keys, the layout mode, the console key, the debug
stubs, and the verbs that report the build, realm, platform and frame rate. The video verbs are in `ui/videoverbs`, the
saved variables in `ui/addons`, and the CVar registry itself in `cvar`.

Roots: `0x488b00`, `0x488ba0`, `0x488c10`, `0x488cf0`, `0x488d70`, `0x488dd0`, `0x488540`, `0x488590`, `0x488640`,
`0x4884a0`, `0x4893d0`, `0x4893e0`

## Reading CVars from Lua

`GetCVar("name")` returns the CVar's current text; `GetCVarDefault("name")` returns its default text, or nil when it has
none. Both raise `Usage: GetCVar("cvar")` (the second raises `Usage: SetCVarDefault("cvar")`) for a non-string argument, and
`Couldn't find CVar named '%s'` when the name is unknown. The name is matched without regard to case, and only a CVar that
Lua may read is found: `CVar::Register` sets bit 31 of the flag word unless its fifth stack argument is nonzero, and the
script lookup finds only records with that bit. A latched CVar answers with its live text, not its pending one.

Addresses: `0x488ba0`, `0x488bb1`, `0x488bd1`, `0x488bdc`, `0x488bf9`, `0x488cf0`, `0x488d01`, `0x488d49`, `0x63dec0`, `0x63df3c`, `0x63dc78`, `0x63ddb4`, `0x63de30`
Confidence: verified
Proof: one
Test: none

`0x488ba0` tests `lua_isstring` (`0x488bad`), looks the name up through `0x63dec0` (`0x488bd1`), which hashes with
`SStrHashHT` and compares with `SStrCmpI`, and returns `[record+0x20]` through `lua_pushstring` (`0x488bf9`). `0x63df3c`
loads the flag word and `jns` returns null when bit 31 is clear. `0x488d49` loads `[record+0x30]` and pushes nil when it is
null (`0x488d5e`). Registration sets bit 31 at `0x63dc78` and `0x63ddb4` when the fifth argument is zero, which the
`script_hidden` column of `tables/cvars.tsv` lists for every registration. `0x63de30`, the plain lookup, has no flag test.

## Writing CVars from Lua

`SetCVar("name", value [, token])` writes the text of `value`, or `0` when it is missing, through `CVar::Set` with the
callback flag on, and returns no value. It raises `Usage: SetCVar("cvar", value [, "scriptCvar")` for a non-string name,
the not-found error, and `"%s" is read-only` when bit 2 of the flag word is set. When the third argument is a string or a
number it then signals `CVAR_UPDATE` (event 312) with the token as `arg1` and the written text as `arg2`; two arguments
signal nothing. `RegisterCVar("name" [, default])` registers a script CVar once: an existing name is left alone, a new one
gets flags 0, category 9 and the default text, or `0`; the thirty-first registration raises `Attempted to register too
many CVars`.

Addresses: `0x488c10`, `0x488c4c`, `0x488c67`, `0x488c6d`, `0x488c98`, `0x488ca8`, `0x488cb4`, `0x488cd0`, `0x488cd5`, `0x703f50`, `0x488b00`, `0x488b25`, `0x488b81`, `0x488b8a`, `0x488b8f`, `0xb4e3c8`, `0x63db90`, `0x63df50`, `0x82e424`
Confidence: verified
Proof: one
Test: none

`0x488c67` to `0x488c6d` shift the flags right 2 and test bit 0. `0x488ca8` calls `CVar::Set(record, text, 1, 0, 0, 1)`. The
signal is `0x703f50(0x138, "%s%s", tostring(arg 3), text)` (`0x488cd5`), reached only when `0x488cb4` finds a string or number
at index 3. `0x488b25` compares the count `0xb4e3c8` with 0x1e and `0x488b8f` increments it after `0x63db90` with the
arguments flags 0, default, no callback, category 9 and a zero fifth argument.

## The detail verbs

Eight pairs read and write CVars through a conversion. The setters raise `Usage: Set<Name>(value)` without a numeric
argument, and the water setter does nothing else.

| verb | CVar | get returns | set stores |
|---|---|---|---|
| `WorldDetail` | `smallCull`, `frillDensity` | 0, 1 or 2 from `smallCull` | `frillDensity` 16, 32, 48 and `smallCull` 0.07, 0.04, 0.01 for 0, 1, 2 |
| `WaterDetail` | none | always 0 | nothing |
| `Farclip` | `farclip` | the number | `%f` text |
| `TerrainMip` | `shadowLevel` | 1 minus the CVar's integer | `%d` of 1 minus the argument |
| `DoodadAnim` | `doodadAnim` | the number | `%f` text |
| `TexLodBias` | `texLodBias` | the number | `%f` text |
| `BaseMip` | `baseMip` | 1.0 minus the CVar's number | `%d` of 1.0 minus the argument |

Addresses: `0x488d70`, `0x488d90`, `0x488dd0`, `0x488e19`, `0x488e9b`, `0x488ec0`, `0x488ed0`, `0x488f00`, `0x488f30`, `0x488fb0`, `0x488fe0`, `0x489060`, `0x489090`, `0x489110`, `0x489140`, `0x489280`, `0x4892b0`
Confidence: verified
Proof: one
Test: none

`GetWorldDetail` compares `smallCull` with the table 0.07, 0.04, 0.01 in order (`0x488d90`) and keeps the last index whose
entry is at least the value, starting from 2, so every value above 0.07 answers 2. `SetWorldDetail` truncates its argument
with `__ftol`, accepts 0 to 2 only and raises `value must be in the range 0, 2` otherwise (`0x488e19`, `0x488e9b`); the
integer table is 16, 32, 48. Farclip, DoodadAnim and TexLodBias format into 16 bytes with `%f`. The Get/Set pair names
are capitalised exactly as registered, `GetFarclip` and `SetFarclip`.

## Modifier keys and layout mode

`IsShiftKeyDown()`, `IsControlKeyDown()` and `IsAltKeyDown()` ask `EventIsKeyDown` for modifier 0, 1 and 2 and return the
number 1 or nil. `SetLayoutMode([n])` stores the truncated n, or 1 when n is not a number, in the top frame's field
`+0xcf8`. `GetMouseFocus()` returns the object the top frame holds at `+0x7c` as a Lua frame, binding its registry
reference first when it has none, or nil. `SetEuropeanNumbers([flag])` stores its flag, true when absent, in the number
formatting switch `0xcf4d24`.

Addresses: `0x488590`, `0x4885c0`, `0x488600`, `0x41f8f0`, `0x488540`, `0x48855d`, `0x488569`, `0x488579`, `0x48df40`, `0x48df46`, `0x48df5b`, `0x701bd0`, `0x48dec0`, `0x771c40`, `0xcf0bd8`, `0xcf4d24`
Confidence: verified
Proof: one
Test: none

`0x41f8f0` is called with `ecx` 0, 1 and 2 (`0x488593`, `0x4885c3`, `0x488603`). `0x488562` loads the top frame
`0xcf0bd8` and stores the `__ftol` result at `+0xcf8` (`0x488569`); a non-number takes `0x488573` and stores 1 at `0x488579`.
`0x48df40` reads `[top+0x7c]`, calls `0x701bd0` when the object's registry word `+4` is 0 and fetches the reference at
`+8` through `lua_rawgeti`. `0x48dec0` passes `GetScriptFlagArg(index 1, default 1)` to `0x771c40`, which stores it at
`0xcf4d24`.

## Build, realm, platform and rate

`GetBuildInfo()` returns the strings `1.12.1`, `5875` and `Sep 19 2006`. `GetRealmName()` pushes the realm CVar's text,
`GetLocale()` the locale name string of the current locale index. `IsWindowsClient()` returns the number 1 and
`IsMacClient()` and `IsLinuxClient()` return nil. `GetFramerate()` returns the smoothed frame rate as a number,
`GetNetStats()` the incoming KB/s, outgoing KB/s and latency in milliseconds that `NetClient::GetNetStats` reports.
`GetDebugStats()`, `ToggleTris()`, `TogglePortals()`, `ToggleCollision()`, `ToggleCollisionDisplay()` and
`TogglePlayerBounds()` are the three-byte `xor eax,eax; ret` and do nothing.

Addresses: `0x4884a0`, `0x4884a1`, `0x4884ad`, `0x4884b9`, `0x4884c5`, `0x48df90`, `0x5ab7d0`, `0x48d8b0`, `0x48c960`, `0x48c980`, `0x48c990`, `0x4893e0`, `0x6724c0`, `0x48b8b0`, `0x537f20`, `0x488af0`, `0x489330`, `0x489340`, `0x489350`, `0x489360`, `0x489440`
Confidence: verified
Proof: one
Test: none

`0x4884a0` pushes three string constants and returns 3. `0x48d8b0` indexes a pointer table at `0x8558a4` by the locale
index and returns 1. `0x48c960` pushes the double 1.0 (`push 0x3ff00000`) and the other two call `lua_pushnil`. `0x48b8b0` calls
`0x5ab490` for the connection and pushes the two floats and the dword, returning 3. The five debug verbs are three bytes each.

## Performance display and screenshots

`TogglePerformanceDisplay()` flips the performance display switch; `TogglePerformanceValues()` and
`ResetPerformanceValues()` call functions that return at once. `Screenshot()` installs the one-shot hook `0x4930b0`; the
frame loop takes the capture after drawing, calls the hook with its result and clears it. The hook signals
`SCREENSHOT_SUCCEEDED` (208) for a nonzero result and `SCREENSHOT_FAILED` (209) for 0, and only after the interface is up.
`Stuck()` casts the spell remembered for it with no item and no target.

Addresses: `0x489400`, `0x442d00`, `0x442cf0`, `0x489420`, `0x442db0`, `0x489430`, `0x442dc0`, `0x4893d0`, `0x442a10`, `0x4930b0`, `0x4930cc`, `0x442350`, `0x442683`, `0x4426a2`, `0x4426ad`, `0x489370`, `0x6e5a90`, `0x4b261d`, `0x4b25b0`, `0xb70098`, `0xb4e36c`
Confidence: verified
Proof: one
Test: none

`0x489400` calls `0x442d00` (loads the switch) and `0x442cf0` with the complement. `0x4893d0` calls `0x442a10` with
`0x4930b0`, which stores the hook. The frame function `0x442350` tests it (`0x442683`), calls it with the capture result
(`0x4426a2`) and zeroes it (`0x4426ad`). `0x4930b0` returns when `0xb4e36c` is 0 and loads `ecx` 0xd0 or 0xd1 for
`0x703e50`. `0x489370` calls `0x6e5a90(spell, 0, 0)` with the dword `0xb70098`, which `0x4b25b0` stores at `0x4b261d` when
a learned spell's record word `+0xf4` equals 0x54.

## The console key

`SetConsoleKey("name")` chooses the key that opens the console, by storing a key code at `0x864554`. A single digit or
letter gives its upper-case character code. `F` and a digit give `0x2ff` plus the number, so `F1` is `0x300`. `NUMPAD`, spaces and a
digit give the digit's character code plus 0xd0 (`NUMPAD0` is `0x100`). Anything else not in the lists below, and any
non-string argument, gives `0x30e`.

| name | code | name | code | name | code |
|---|---|---|---|---|---|
| `PLUS`, `+` | `0x10a` | `ESCAPE` | `0x200` | `INSERT` | `0x208` |
| `MINUS`, `-` | `0x10b` | `ENTER` | `0x201` | `DELETE` | `0x209` |
| `MULTIPLY`, `*` | `0x10c` | `BACKSPACE` | `0x202` | `HOME` | `0x20a` |
| `DIVIDE`, `/` | `0x10d` | `TAB` | `0x203` | `END` | `0x20b` |
| `DECIMAL`, `.` | `0x10e` | `LEFT`, `UP` | `0x204`, `0x205` | `PAGEUP`, `PAGEDOWN` | `0x20c`, `0x20d` |
| `EQUALS`, `=` | `0x30c` | `RIGHT`, `DOWN` | `0x206`, `0x207` | `NUMLOCK`, `PRINTSCREEN` | `0x20f`, `0x212` |

Addresses: `0x488640`, `0x488670`, `0x4886e3`, `0x4886f7`, `0x48870a`, `0x488748`, `0x48875b`, `0x488820`, `0x4888a0`, `0x488ada`, `0x488adf`, `0x488ae1`, `0x63cb00`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

`0x48864a` loads the default `0x30e` into `edi`, and every path ends at `0x488adf`, which passes `edi` to `0x63cb00`
(`mov [0x864554],ecx`). The NUMPAD names follow a `SStrCmpI` of the first six characters (`0x488715`) and then compare
the rest. The names beyond NUMPAD are compared with `SStrCmpI` and the length limit `0x7fffffff` in the order of the table.
