# ui: the video option verbs

The Lua functions the stock video options window calls: the list of screen resolutions and refresh rates, the display
capabilities, the gamma, the two verbs that rebuild the display, and the engine's answer to a window size change. They are
in `GameUI.cpp` (`0x48bb60` to `0x48c7a8`, `0x48db40`) with the gamma pair beside the option setters (`0x4891c0`). The mode
enumeration and the capability block belong to `gx`, and the `gxRestart` handler and the display options to the console
(`console`); both are stated here as call order and arguments.

Roots: `0x48bc20`, `0x48bf10`, `0x48bfd0`, `0x48c0d0`, `0x48c360`, `0x48c580`, `0x48c640`, `0x48db40`, `0x4891c0`,
`0x4891f0`, `0x48dab0`, `0x48dad0`, `0x492e90`

## The resolution list

The list of `{width, height}` pairs (8 bytes each) is built once, on first use by any of the four resolution verbs, and
never rebuilt. With the `widescreen` option (default 1) set, the connected display's modes are taken in ascending pixel
area and kept when width over height is at least 1.248, the width at least 800 and the height at least 600, skipping a pair
equal to the last one kept. With the option clear, missing, or no mode surviving, the list is 800x600, 1024x768,
1280x1024 and 1600x1200, never mixed with kept modes.

Addresses: `0x48bc90`, `0x48bd03`, `0x804570`, `0x48bd0b`, `0x48bd10`, `0x48bd18`, `0x48bd23`, `0x48bd79`, `0x48bd8f`, `0x48bda2`, `0x48bddf`, `0x48be43`, `0x48bea7`, `0x5899c0`, `0x58bd40`, `0x58be40`, `0xb4b3d0`, `0xb4b3d4`, `0xb4b3d8`, `0xb4b3dc`, `0x48bb60`, `0x48bb70`, `0x48bb90`, `0x48bba0`
Confidence: verified
Proof: one
Test: `ui::luabind_48b_aspect_filter`

`0x48bc90` returns at once when the count `0xb4b3d4` is nonzero (`0x48bc9e`), looks up `widescreen` and takes the fixed path
when it is absent or zero (`0x48bcb8`, `0x48bcc1`). The aspect test is `fild`, `fidiv`, `fcomp` against 1.248 and
`test ah,1` skipping on C0 (`0x48bd03` to `0x48bd0e`). The bytes at `0x48bd79` to `0x48bd91` make the fixed path run only
when nothing was kept. The enumerator `0x58bd40`, called through `0x5899c0`, reads display 0 only, keeps modes of 640x480 and
16 bits or more, stores 16-byte records of width, height, bits and rate, and sorts by `height·width` (`0x58be40`) with no
tie-break. The array is `0xb4b3d0` capacity, count, data and quantum, zeroed by `0x48bb70` (run from `0x48bb60`, which `0x48bb90` follows by registering `0x48bba0`) and freed by `0x48bba0`.

`ui::luabind_48b_aspect_filter` runs the aspect test, the slice `0x48bcfa` to `0x48bd0b` (`fild`, `fidiv`, `fcomp` against 1.248), and reads the kept bit from C0.

## The resolution verbs

`GetScreenResolutions` pushes one string `%dx%d` per list entry and returns the count. `GetCurrentResolution` scans the
`gxResolution` option's text with `%d%c%d`, so any one-character separator parses, and returns the 1-based index of the
matching entry; on a miss, with the option or its text absent, or with an empty list, it returns the number 1. The option's
default text 640x480 is below the list floor, so a fresh configuration answers 1 by a miss.

Addresses: `0x48bc20`, `0x48bc60`, `0x48bc6d`, `0x48bc7c`, `0x48bf10`, `0x48bf20`, `0x48bf55`, `0x48bf88`, `0x48bfa2`, `0x73fb5d`
Confidence: verified
Proof: one
Test: none

`0x48bc20` sizes the Lua stack by the count (`0x6f2f30`), formats into a 64-byte buffer through `0x64a7f0` and returns
`[0xb4b3d4]` through both exits. `0x48bf10` zeroes the width and height before `0x73fb5d` and does not read the result, so
a malformed text leaves 0 by 0; the index store is `inc eax` (`0x48bfa2`) and the miss pushes the double 1.0
(`0x48bf88`, `0x48bf8d`).

## Choosing a resolution

`SetScreenResolution(n)` takes the list entry n−1, an absent or non-numeric n meaning entry 0, with n truncated toward zero.
An n of count+1 reads the entry one past the last, and so do 0, negative values and anything larger, because the
check compares unsigned. When the chosen pair differs from the `gxResolution` text it writes `%dx%d` into the option and
runs the console command `gxRestart`; an equal pair does nothing. The option is staged, not applied, until the restart.

Addresses: `0x48bfd0`, `0x48bfe8`, `0x48c003`, `0x48c008`, `0x48c009`, `0x48c00b`, `0x48c070`, `0x48c081`, `0x48c097`, `0x48c0ad`, `0x48c0bc`, `0x63ce00`, `0x63df50`
Confidence: verified
Proof: one
Test: none

`0x6f34d0` decides the numeric test (`0x48bfe8`); a false result leaves `esi` at 0. After `__ftol` and `dec eax` the compare
`cmp esi,eax; jae` (`0x48c00b`) keeps the index when the count is at or above it, else the failure block loads the count
(`0x48c00d`); the entry read at `0x48c070` is unguarded. `0x63df50(option, text, 1, 0, 0, 1)` stores the text and
`0x63ce00("gxRestart", 1)` runs the command at once.

## The refresh rates

`GetRefreshRates([n])` selects the list entry n−1 by the same rule as `SetScreenResolution`, with entry 0 for a missing n,
and returns the refresh rates of the display modes of that size, ascending with equal neighbours merged, as numbers. An index
equal to or past the count returns no value at all. It does not filter by the rates the `gxRefresh` option accepts.

Addresses: `0x48c0d0`, `0x48c0ec`, `0x48c12e`, `0x48c134`, `0x48c136`, `0x48c1a2`, `0x48c1c9`, `0x48c1ea`, `0x48c1f3`, `0x48c240`, `0x5899c0`, `0x73f727`
Confidence: verified
Proof: one
Test: none

With no argument `esi` stays 0 (`0x48c0ec`) and the jump at `0x48c0f5` skips the index arithmetic. `0x48c12e` compares the
index with the count and `0x48c136` returns 0 values. The enumeration is repeated through `0x5899c0`; each record whose
width and height equal the entry contributes its rate (`+0xc`, `0x48c1a2`), then `0x73f727` sorts the rates with `0x48c240`
(`a<b` gives −1, `b<a` gives 1, else 0). A rate equal to the one before is skipped (`0x48c1ea`). The count returned is
the number pushed.

## The capability verbs

`GetVideoCaps` returns seven values from the graphics capability block: anisotropic filtering, pixel shaders, vertex
shaders, trilinear filtering, a number that is 1 on Direct3D and 0 on OpenGL, the maximum anisotropy, and the hardware
cursor. Slots 1, 2, 3, 4 and 7 are nil or the number 1; slot 5 is always a number; slot 6 is nil for 0 and else the value.
Pixel and vertex shaders are nil only when their field is −1.

Addresses: `0x48db40`, `0x48db4a`, `0x48db57`, `0x48db70`, `0x48db8e`, `0x48dbac`, `0x48dbcb`, `0x48dbe1`, `0x48dc06`, `0x48dc1f`, `0x48dc2f`, `0x58a230`, `0x5923d0`
Confidence: verified
Proof: one
Test: none

`0x58a230` returns the block (`0x5923d0`: device + 0x23c). The fields read, in order: `+0xa8` (`test`, `0x48db57`), `+0x98` and
`+0x94` (`cmp -1`, `0x48db70`, `0x48db8e`), `+0xa4`, `+0x20` (`fild`, no test), `+0xac` (`test`, then `fild qword`) and `+0xc4`.
Every present flag is pushed through `0x6f3810` as the double 1.0, every absent one through `0x6f37f0`; the function returns
7 at both ends. No boolean is pushed.

## The capability fields

Each field comes from the device constructor and its capability fill. On Direct3D (`0x598ee0`, the fill) and OpenGL
(`0x58e480`) they are:

| field | Direct3D | OpenGL |
|---|---|---|
| `+0x20` | 1 | 0 |
| `+0x94` | −1, 0 for vs_1_1, 1 from vs_2_0 | 2 with the vertex program extension, else −1 |
| `+0x98` | −1, then 0 to 3 for ps_1_1 to ps_1_4, 4 from ps_2_0 | 10 to 5 by extension, else −1 |
| `+0xa4` | bit 17 of the texture filter caps | 1 |
| `+0xa8` | anisotropic filter bits, cleared when the maximum is under 2 | extension present and maximum over 1 |
| `+0xac` | the device's maximum anisotropy | the query result, else 1 |
| `+0xc4` | bit 0 of the cursor caps | 0 |

Addresses: `0x598ee0`, `0x598f84`, `0x598fa2`, `0x598fd9`, `0x598fe6`, `0x599040`, `0x59908c`, `0x59909f`, `0x5990ae`, `0x5990b6`, `0x5991b0`, `0x58e480`, `0x58e615`, `0x58e69d`, `0x58e6b1`, `0x58e6c7`, `0x58e6d7`, `0x58e6f4`, `0x58e759`, `0x598d37`, `0x58df43`, `0x5916e0`, `0x591b10`, `0x591b2e`
Confidence: verified
Proof: one
Test: none

The base constructor clears the block with `rep stosd` (0x38 dwords, `0x591b15`) and stores 1 at `+0xac` (`0x591b2e`), so
that field is never 0. The only writers of `+0x20` are the two device constructors (`0x598d37` stores 1, `0x58df43` stores
0) and its only reader is `GetVideoCaps`. The Direct3D fill tests the pixel shader token against `0xffff0200`, `0xffff0104`
to `0xffff0101` and the vertex token against `0xfffe0200` and `0xfffe0101`. The meaning of `+0x20` is the backend, inferred
from the two constant stores.

## Gamma

`GetGamma()` returns 1.0 minus the `gamma` option's number and `SetGamma(v)` stores 1.0 minus v as `%f` text, both with
`fsubr` against the double 1.0. With the default `gamma` 1.0 the slider value is 0. `SetGamma` raises
`Usage: SetGamma(value)` without a numeric argument and applies no clamp: v = 5 stores −4.000000. The change callback
`0x4034d0` converts the new text and, only while `DesktopGamma` is 0, rebuilds and uploads the ramp at once through the
device slot at `+0x34`. `DesktopGamma` set to nonzero restores the desktop's ramp, and set to 0 re-applies `gamma`.

Addresses: `0x4891c0`, `0x4891d0`, `0x4891f0`, `0x4891fe`, `0x48920d`, `0x8424cc`, `0x489235`, `0x489251`, `0x489267`, `0x8015b8`, `0x4034d0`, `0x4034e8`, `0x4034f3`, `0x403500`, `0x589c00`, `0x58d080`, `0x599dc0`, `0x592220`
Confidence: verified
Proof: one
Test: `ui::luabind_488_set_gamma`, `ui::luabind_488_get_gamma_basemip`

`0x4891d0` is `dc 2d`, `fsubr qword`, so the result is the constant minus the option's float at `+0x24`. `0x489251` formats
into a 16-byte buffer through `0x64a7f0`. The callback returns 1 at `0x4034f8`, so it never vetoes. `0x403500` converts the
text with `0x64ac60`; nonzero copies the device's saved ramp (`0x589c70`, from device `+0x928`) and sends it through slot
`+0x30` (`0x589c20`), zero uploads `gamma` from its record. `0x589c00` calls device
slot `+0x34`. Both upload paths skip the hardware write when the format's windowed byte (device `+0x20b`) is set; Direct3D
also requires bit 0x20000 of the device's `Caps2`.

`ui::luabind_488_get_gamma_basemip` runs the complement slice of `GetGamma` (`0x4891cd` to `0x4891de`) and `ui::luabind_488_set_gamma` that of `SetGamma` (`0x489230` to `0x489241`); the callback and the ramp upload are not run.

## Restarting the display

`RestartGx()` runs the console command `gxRestart` with history on and returns no value. The handler applies the pending
display format, rebuilds the device and commits the latched display options; it is `console/display#restart-and-defaults`,
the latch is `cvar/registry#setting-a-value` and the validators are `console/display#display-callbacks`. No function of the chain
contains a signal call, and the device class is not changed.

Addresses: `0x48dab0`, `0x842978`, `0x63ce00`, `0x589ac0`, `0x63a230`, `0x63a47e`
Confidence: verified
Proof: one
Test: none

`0x63ce00(command, history)` looks the name up and calls the handler at `+0x18` synchronously. The device class is chosen by
`0x589ac0`, called only from `0x63a230` (`0x63a47e`), which `gxRestart` does not reach.

## Restoring the video defaults

`RestoreVideoDefaults()` installs the recommended settings for the adapter and restarts the display, returning no value. The
installer is `console/display#restart-and-defaults` (`0x639a20`): it writes the twelve display options and then sixteen
graphics options from the hardware row's settings block, not from the registered defaults. The block's `+0x2c` field goes to
`SmallCull` and `+0x30` to `DistCull`.

Addresses: `0x48dad0`, `0x639a20`, `0x639a60`, `0x639b3f`, `0x639b51`, `0x641820`, `0x641924`, `0x641933`, `0x641260`, `0x63a26b`, `0xc4e6ac`
Confidence: verified
Proof: one
Test: none

`0x639a20` calls `0x641820` with the settings block at 0xc4e678 and the matched row at 0xc4e6bc; `0x641924` to
`0x641933` store at the block's `+0x34` the address 0xc518e0 plus 0x38 times the row's field at `+0x40`, so the dword at
`0xc4e6ac` is a pointer into a preset table. `0x639b3f` and `0x639b51` store the block's `+0x2c` and `+0x30` into
`SmallCull` and `DistCull`. `0x641260`, the detection, has one caller, `0x63a26b`.

## The multisample verbs

`GetMultisampleFormats` returns three numbers per entry of a list of colour bits, depth bits and sample counts built once from
the device's formats, in the class `gxApi` names. `GetCurrentMultisampleFormat` returns the 1-based index of the entry
equal to `gxColorBits`, `gxDepthBits` and `gxMultisample`, or 1 on a miss. `SetMultisampleFormat(n)` chooses an entry by the
index rule of `SetScreenResolution`, writes the three options and runs `gxRestart` when they differ.

Addresses: `0x48c360`, `0x48c3e0`, `0x48c580`, `0x48c640`, `0x48c3f8`, `0x48c419`, `0x48c455`, `0x48c480`, `0x48c4b0`, `0x48c5f6`, `0x48c611`, `0x48c705`, `0x48c791`, `0x589a00`, `0xb4b43c`, `0xb4b440`, `0xb4b444`, `0xb4b448`, `0x48bbc0`, `0x48bbd0`, `0x48bbf0`, `0x48bc00`
Confidence: verified
Proof: one
Test: none

`0x48c3e0` returns when the count `0xb4b440` is set. Otherwise it chooses the class from `gxApi` (OpenGl, compared with
`SStrCmpI`, is class 0, else 1; two command-line switches override it, `0x48c424` and `0x48c434`), calls `0x589a00` for the
0x38-byte format records, maps the colour and depth fields through the dword table at 0x804530, and appends each triple not yet in the
12-byte array `0xb4b444`. `0x48c4b0` is the duplicate test. The index rule and the unguarded read are those of
`0x48bfd0`. The list cells are zeroed by `0x48bbd0` and freed by `0x48bc00`, as for the resolution list.

## After a size change

When the window size changes, the top frame calls the host callback at its `+0x1130`, which is `0x492e90`. With the
`useUiScale` option clear it applies the automatic scale for the new size and signals `DISPLAY_SIZE_CHANGED` (event 511).
With it set, it sets the `UIParent` scale from `uiScale`, capped for aspects below 4:3, and, when the configured aspect
changed by more than 0.001, reloads the interface and signals nothing. It returns 1.

Addresses: `0x492e90`, `0x48fe78`, `0x49010e`, `0x7665c0`, `0x7665ca`, `0x492f4a`, `0x492f52`, `0x492f57`, `0x492f0e`, `0x492f13`, `0x492f33`, `0x494550`, `0x492f70`, `0x4901e0`, `0x491380`, `0x703e50`, `0xb4e2b0`, `0xb4e2b4`, `0xb4e30c`
Confidence: verified
Proof: one
Test: none

`0x48fe78` stores `0x492e90` at the top frame's `+0x1130`; `0x7665c0` (`CSimpleTop::OnWindowSize`) loads it at `0x7665ca`.
`0x492e9b` and `0x492ea6` branch to `0x492f4a` on a missing or zero `useUiScale`; `0x492f52` calls `0x492f70` with force 0 and
`0x492f57` signals index 511, the position of `DISPLAY_SIZE_CHANGED` in the 549 names installed by `0x48feab`. The other path
calls `0x494550` (`0x492f0e`), compares `0x4901e0` with the stored aspect and reloads through `0x491380` (`0x492f33`). Every
call of `0x703e50` loads an immediate index, and 0x1ff is loaded only here; five calls of `0x703f50` pass a computed index
and are not traced here.
