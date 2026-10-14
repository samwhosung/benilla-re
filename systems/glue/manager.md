# glue: the manager

The glue manager's life: startup, the build of the glue screens, their teardown and shutdown.

The manager is a set of statics in `CGlueMgr.cpp`, which the community calls `CGlueMgr`. The
client starts it once (`0x46a400`): it reads the four agreement options, builds the glue screens
(`0x46a7b0`), starts the server alert's download and registers its per-frame function on the
frame-time event. A build makes a `CSimpleTop`, the cursor, the glue's script functions and event
names, loads `GlueXML.toc` behind a signature check and fires `FRAMES_LOADED`. Entering the world
suspends the glue (`0x46ac90`), leaving the per-frame function registered; leaving the world, a
reload request or a size change that moves the aspect rebuilds it. The context's stop shuts the
manager down (`0x46ad10`). The manager's globals live from `0xb41468` to `0xb41e2c`; the same
object file holds the queue of console lines other threads post (`0x46a610`) and the log status
whose destructor is slot 0 of `0x8037f4`.

Roots: `0x46a400`, `0x46a580`, `0x46a610`, `0x46a7b0`, `0x46aac0`, `0x46abe0`, `0x46ac90`, `0x46ad10`

## Startup entry

`0x46a400` runs once, from `0x402b36`. In order it: sets `0xb41d90` to 1; seeds the agreement
flags (`Agreement flags`); runs the resize test `0x46abe0` on a record holding the window's size
(`Screen size and aspect`); registers the `ModelFFX` frame factory `0x46fa00`; builds the glue
with `0x46a7b0(0)`; copies the locale index `0xc0e080` to `0xb41da8` and the `Wow.ini` region
`0x882730` to `0xb41da4`; builds the realm categories (`0x46e300`); registers `0x46b930` on event
category 5 with argument 0 and priority 0.0; starts the server alert's download (`Server alert
fetch`); and starts the LCD layer (`0x729dd0`).

Addresses: `0x46a400`, `0x402b36`, `0xb41d90`, `0x46a4ee`, `0x46a4fe`, `0x46a505`, `0x46a515`, `0x46a51b`, `0x46a537`, `0x46a574`, `0xb41da4`, `0xb41da8`, `0x882730`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

`0x46a414` stores 1. `0x46a4f4` and `0x46a4f9` pass `0x46fa00` and `ModelFFX` to `0x6ee880`, with
0 pushed (`ret 4`). `0x46a503` `xor ecx,ecx` before the build. The registration is
`EventRegisterEx` (`0x41fca0`) with `ecx` 5, `edx` `0x46b930`, then 0 and the float 0.0 pushed
(`0x46a525` to `0x46a537`). Category 5 is the loop's frame time
(`core/events`). `0x882730` is the output the startup's `Wow.ini` read passes for the key
`Region` (`0x4024ac`); `0xc0e080` is the index of the `Language`·`Country` text among `enUS` to
`xxYY` (`0x402562`). `0x729dd0` allocates under `Source\Ui\UILCD\LCD.CPP` and names
`Logs\LCDXML.log`. The only call to `0x46a400` is at `0x402b36` (census of `e8`/`e9`; no stored
address).

## Agreement flags

Each of the four agreement options gives two flags from its integer value `v` (the CVar
record's `+0x28`): accepted is `v == 1`, and show-notice is set to 1 when `v < 0` and otherwise
left at its default, 0. The `Accept…` functions set accepted to 1, show-notice to 0 and the
option to `"1"`; the `Show…Notice` and `…Accepted` bindings return the flags.

| option | record | show-notice | accepted | accept |
|---|---|---|---|---|
| `readTOS` | `0x882720` | `0xb41df8` | `0xb41dfc` | `0x46aef0` |
| `readEULA` | `0x882688` | `0xb41e00` | `0xb41e04` | `0x46af20` |
| `readScanning` | `0x8826d8` | `0xb41e08` | `0xb41e0c` | `0x46af50` |
| `readContest` | `0x88265c` | `0xb41e10` | `0xb41e14` | `0x46af80` |

Addresses: `0x46a400`, `0x46a41a`, `0x46a41f`, `0x46a42c`, `0x46a42e`, `0x46a48e`, `0xb41df8`, `0xb41dfc`, `0xb41e00`, `0xb41e04`, `0xb41e08`, `0xb41e0c`, `0xb41e10`, `0xb41e14`
Confidence: verified
Proof: one
Test: none

For `readTOS`: `0x46a41a` loads `+0x28`, `cmp eax,edi` (1) and `sete cl` store accepted at
`0x46a426`; `cmp eax,ebx` (0) with `jge` at `0x46a42c` skips the store of 1 at `0x46a42e`. The
other three repeat the pattern to `0x46a48e`. A writer census of the eight cells finds these
stores and one pair in each `Accept…` function (`0x46af03`, `0x46af0d` and so on), which then
call `0x63df50` with `"1"`. The readers are the bindings `0x46cfa0` to `0x46d120` and
`0x46afb0`, which tests the TOS and EULA accepted flags.

## Server alert fetch

The startup reads the Lua global `SERVER_ALERT_URL` (empty when not a string), sets pending
`0xb41e28` to 1, ready `0xb41e2c` to 0, empties the 2048-byte buffer `0xb414f8`, and starts a
download with the callback `0x46a580`; if it does not start, pending goes back to 0. Each call
with a size `n` above 0 copies at most `min(n, 2048) − 1` bytes, stopping at a NUL, and appends
them, keeping the buffer to 2047 characters and a NUL. The final call, flagged done, sets ready to 1 when the buffer
is not empty and begins with `SERVERALERT:` in any case, then clears pending. Nothing restarts
the download.

Addresses: `0x46a541`, `0x46a564`, `0x46a56e`, `0x46a580`, `0x46a5a5`, `0x46a5bb`, `0x46a5e8`, `0x46a5f1`, `0x46a5fb`, `0xb41e28`, `0xb41e2c`, `0xb414f8`, `0x836b3c`, `0x836ed8`, `0x7a6ee0`
Confidence: verified
Proof: one
Test: none

`0x703bf0` with `edx` −1 and 0 pushed looks the name up as a global and returns `""` unless it
is a string. `0x7a6ee0(url, 0x46a580, 0)` returns a byte tested at
`0x46a569`. The callback is `fastcall(context, data, size, done, extra)`, `ret 0xc`: the
downloader behind `0x7a6ee0` calls it with `(size, 0, 0)` per read and `(0, 1, x)` at the end. `0x46a5a5` is Storm's `SStrCopy` into a stack buffer with length `min(size, 0x800)`;
`SStrCopy` stops at `length − 1` bytes or a NUL (`0x64a5cf`). `0x46a5bb` is
`SStrPack` into `0xb414f8` with 0x800, which appends up to the 2047th byte (`0x64a7ac`). The prefix is `[0x836b3c]`, which points at `0x836ed8`,
compared with `SStrCmpI` over its `SStrLen` (`0x46a5e8`). While ready is set, `0x46b930` fires
`SHOW_SERVER_ALERT` with the text after the prefix (`0x46b964`) and clears it.

## Glue build

`0x46a7b0(reset)` zeroes the manager's state `0xb41da0` and the cells `0xb41d98`, `0xb41d9c`,
`0xb41df4`, `0xb41d94` and `0xb41d8c`; stores the screen's size and aspect (`Screen size and
aspect`); allocates a `CSimpleTop` (4408 bytes) at `0xb41d84` with `0x46abe0` as its size
callback at `+0x1130`; makes the cursor and sets it on the top; when `reset` is nonzero, replaces
the Lua state (`0x703b80`); registers the script functions (`0x46abb0`); installs the 26 glue
event names at `0xb41e70` (`0x703d90`); starts character creation's state (`0x470430`); then
checks and loads the GlueXML (`Signature and digest check`). The startup passes 0; every other
caller passes 1.

Addresses: `0x46a7b0`, `0x46a7c2`, `0x46a7e0`, `0x46a851`, `0x46a856`, `0x46a872`, `0x46a879`, `0x46a87b`, `0x46a880`, `0x46a88f`, `0x46a894`, `0xb41da0`, `0xb41d84`, `0xb41e70`, `0x703b80`, `0x703d90`, `0x764180`
Confidence: verified
Proof: one
Test: none

`0x46a7c2` to `0x46a7e0` store `ebx`, 0. The top is `SMemAlloc(0x1138)` under `CGlueMgr.cpp` line
336 and `0x764180`; `0x46a856` stores `0x46abe0`. `0x46a860` calls `0x46adb0`, then
`CSimpleTop::SetCursor` (`0x764d50`) gets `[0xb41d88]` (`0x46a872`). `0x46a877` `cmp esi,ebx;
je` skips `0x703b80`, which destroys the state and reruns the Lua init when one exists. `0x46a885`
passes 26 in `edx`; `0x703d90` frees the old name records and copies the new names. The callers
pass `ecx` 0 at `0x46a503` and 1 at `0x402064`, `0x46b998` and `0x46c60e`.

## Signature and digest check

The build opens `Logs\GlueXML.log` (`0x837024`) into a load-log status and asks `0x6f10f0`
about `GlueXML.toc`: 0 when `GlueXML.toc.sig` cannot be read, 1 when that file is not 276 bytes
or fails its verification, 2 when the digest `0x6f10f0` computes differs from the file's first
16 bytes, 3 when they match. Answers 0 to 2 add `GlueXML missing signature`, `GlueXML has
corrupt signature` or `GlueXML is modified or corrupt` to the status and quit with code 9,
loading nothing. Answer 3 runs the toc and compares the load's MD5 digest with that digest: a
mismatch quits with 9; a match fires `FRAMES_LOADED` (event 22), calls `0x46c6a0`, makes the
effect pair and sets `0x46b440` as opcode `0x2c8`'s handler.

Addresses: `0x46a8a0`, `0x46a8ce`, `0x46a8eb`, `0x46a8f6`, `0x46a90b`, `0x46a915`, `0x46aaac`, `0x46a941`, `0x46a94e`, `0x46a9b3`, `0x46a9c1`, `0x46a9d3`, `0x46a9dd`, `0x46aa34`, `0x46aa3e`, `0x46aa4e`, `0x836fb8`, `0x836f98`, `0x836f78`, `0x837024`, `0x6f10f0`, `0x6edb90`, `0x661da0`, `0x661e90`
Confidence: verified
Proof: one
Test: none

Before the check the build creates `Logs` (`0x46a8a0`), calls `0x51fb50`, which handles loose
`Interface\GlueXML` and `Interface\FrameXML` folders as `%s.old`, and starts the MD5 context
(`0x46a8f6`). `0x46aaac` holds `0x46a91c`, `0x46a929`, `0x46a936`, `0x46a99e`; `ja` sends anything above 3 to
`0x46a949`, the quit without a line, which `0x6f10f0` never reaches. In `0x6f10f0`: `0x6f1138`
returns 0 on a failed read; `0x6f1143` `cmp [ebp-0x8],0x114`; `0x6f1199` tests the verification; then
`0x6f11eb` `repz cmps` of the file's first 16 bytes with the digest its callee writes into the
caller's buffer, `sete`, `add eax,2`. `0x661da0` stores the MD5 initial words; `0x46a9c1` finishes the context into
`[ebp-0x48]` and `0x46a9d3` compares four dwords with `0x6f10f0`'s digest at `[ebp-0x38]`. The toc
runner `0x6edb90` gets the context in `edx` and adds the toc's bytes (`0x6edc66`); what each
listed file adds (`0x6edd51`) is not read here. Both quits are `0x401560` with `ecx` 9
(`0x46a949`, `0x46a9d8`), and the build goes on: every path with an open log writes the status's lines to it, closes it
(`0x46aa96`) and returns.

## Quit code

`0x401560(code)` stores the code at `0x882738` and closes the current context, so the client
quits at the end of the tick. At teardown, a nonzero code is shown: the text of
`Startup_Strings.dbc` record `code` in the client's locale, or `Unknown Error` when there is no
such record, with record 1's text, or `World of Warcraft`, as the second string, both passed to
`0x435f00`. It is not the process's exit status.

Addresses: `0x401560`, `0x882738`, `0x41f9a0`, `0x403bfb`, `0x403c26`, `0x403c39`, `0x403c40`, `0x403c53`, `0x435f00`
Confidence: verified
Proof: one
Test: none

`0x401560` is `mov [0x882738],ecx; jmp 0x41f9a0`. A census of `0x882738` finds that store and one
load, `0x403bfb` in `DestroyGlobal`. With the record index `[0x882504]`, its largest id
`[0x882508]` and the locale index `ebx`: `0x403c26` reads `[record1 + 4·locale + 8]`;
`0x403c35` to `0x403c40` read record `code` the same way when `0 ≤ code ≤ max` and the record
exists; `0x403c53` uses `Unknown Error`. `0x4025de` reads the index from
`DBFilesClient\Startup_Strings.dbc` after a `WDBC` check and 11 columns. Its other callers pass
0 (`QuitGame` and others) or `0xa`.

## Load log status

The build's status is a stack object of vtable `0x8037f4`, a status (`CStatus`, `0x7ffa10`)
whose list collects lines and whose `+0x14` holds a Storm log handle. Its destructor writes every
line, first added first, to the log, closes the log, and runs the status's destructor; slot 0,
`0x46aac0`, is that destructor in its deleting form. The build inlines it at its three exits.
In-game `LoadAddOn` keeps one on the heap and deletes it through slot 0, so the slot is live.

Addresses: `0x46aac0`, `0x8037f4`, `0x46a8b4`, `0x46a8bf`, `0x46a958`, `0x46aa9c`, `0x48ea36`, `0x48eab4`, `0x419e30`
Confidence: verified
Proof: one
Test: none

`0x8037f4` is `0x46aac0`, then the same four slots as `CStatus`'s vtable `0x7ffa10` after its
slot 0. The build stores it at `0x46a8b4`/`0x46a8bf`, `0x46c6c0` makes the list
(link offset 8), and `+0x10` and `+0x14` are zeroed. `0x41a000(status, 1, text)` adds a line
(`0x46a941`). `0x46aac0` walks from `+0xc`, `SLogWrite(+0x14, [node])`, next at `[+0x4] + node +
4`; `0x659ec0` closes; `0x419e30`; `test [ebp+0x8],1` frees; `ret 4`. A census of `0x8037f4`
finds 11 references, each installing it as a vtable: the build (3), `0x46aac0`, `0x48ea36` in
`LoadAddOn` (a heap object of 24 bytes), two in the UI's load and four in `LCD.CPP`. `0x48eab4`
is `call [edx]` with 1 on that object.

## Screen size and aspect

The build stores the screen's width and height, truncated, at `0xb41e1c` and `0xb41e20`, and the
aspect `0x46ab40` gives at `0xb41e24`, then sets it as the screen's aspect (`0x41ad10`).
`0x46ab40` is `gxResolution`'s `%d%c%d` width over height while `widescreen` is nonzero, else
4/3. The top's size callback `0x46abe0` returns 1 at once when the stored width and height are
positive and equal the event's; otherwise it stores them, gets the aspect, gives `GlueParent`, when it
is a frame, scale 1.0 forced and updates the layout, and, when the aspect differs from `0xb41e24` by 0.01 or more
or the difference is NaN, stores it and requests a rebuild (`0xb41d8c`).

Addresses: `0x46ab40`, `0x46abe0`, `0x46a7f2`, `0x46a808`, `0x46a812`, `0x46a81c`, `0x46a828`, `0x46abfe`, `0x46ac51`, `0x46ac66`, `0x46ac71`, `0x46ac76`, `0xb41e1c`, `0xb41e20`, `0xb41e24`, `0x7665c0`, `0x46ab97`
Confidence: verified
Proof: one
Test: `glue::glue_aspect_difftest`, `glue::glue_init_viewport_difftest`, `glue::glue_onresize_difftest`

Width is `__ftol(r[3] − r[1])`, height `__ftol(r[2] − r[0])` of `0x58a250`'s rectangle. `0x46ab40`:
`fild` width, `fidiv` height, at 53 bits, stored as a float at `0x46a81c`. `0x7665c0`, on category
0x1c, passes a record with the payload's width at `+0x10` and height at `+0x14`. `0x46abeb` to
`0x46abfe` are `jle`/`jne` tests. The globals are zero-filled at load, so the first call always
stores. `0x76c760("GlueParent")` must be a frame;
`0x76ac10(1.0, 1)`; `0x768ed0`. `0x46ac66` `fcomp` 0.01 (`0x3c23d70a`) of `|stored − new|`,
`test ah,5`, `jnp`: less skips; greater, equal and unordered store. The startup's call
(`0x46a4ee`) passes the window's size (`0x58a240`); each store it makes is overwritten by the
build that follows.
`glue_aspect_difftest` runs `0x46ab97` to `0x46ab9e`; `glue_init_viewport_difftest` runs `0x46a7f7`
to `0x46a817`; `glue_onresize_difftest` runs `0x46ac5b` to `0x46ac86`.

## Rebuild requests

`0xb41d8c` asks for a rebuild. The size callback and the `reloadUI` console command (`0x4035f0`)
set it; the build clears it. The per-frame function `0x46b930`, when the state is 0 and the flag
is set, suspends and rebuilds the glue (`0x46a7b0(1)`) and sets the current screen again, unless
the glue is suspended; either way it clears the flag.

Addresses: `0xb41d8c`, `0x4035f0`, `0x46ac76`, `0x46a7e0`, `0x46b97f`, `0x46b98b`, `0x46b993`, `0x46b99d`, `0x46b9ac`
Confidence: verified
Proof: one
Test: `glue::glue_onresize_difftest`

A census finds five references: stores of 1 at `0x4035f0` and `0x46ac76`, stores of 0 at
`0x46a7e0` and `0x46b9ac`, and the test at `0x46b97f`. `0x4035f0` is the handler the startup
registers with the console command `reloadUI`. In `0x46b930`, `0x46b977` skips the block for a
state other than 0; `0x46b98b` skips the rebuild while `0xb41d94` is set; `0x46b9a7` calls
`0x46b840` with the current screen's name.
`glue_onresize_difftest` runs `0x46ac5b` to `0x46ac86`.

## Suspend

`0x46ac90` marks the glue suspended (`0xb41d94` = 1), destroys the effect pair, frees the
character list's displays (`0x472090`) and character creation's state (`0x470640`), deletes the
top and clears `0xb41d84`, closes the cursor, unregisters the script functions, calls
`0x46c6b0`, and clears opcode `0x2c8`'s handler. It leaves the per-frame registration, the event
names, the Lua state, the realm categories, the console queue and the alert in place. Entering
the world, a rebuild and the shutdown call it.

Addresses: `0x46ac90`, `0xb41d94`, `0x46ac9a`, `0x46acb7`, `0x46acc4`, `0x46acc9`, `0x46acce`, `0x46acd8`
Confidence: verified
Proof: one
Test: none

The body is the store, calls to `0x46a770`, `0x472090` and `0x470640`, `call [eax+0x4]` with 1
on the top (`0x46acb7`), then `0x46ae30`, `0x46ace0`, `0x46c6b0`, and a tail jump to
`ClientServices::ClearMessageHandler` (`0x5ab670`) with `0x2c8`. Its callers are `0x46ad38`,
`0x46b993` and `0x46c219`. Its only writers are `0x46ac90` and the build's 0
(`0x46a7da`); glue reads it at `0x46b98b`, `0x46c1ce` and `0x46c606`.

## Shutdown

`0x46ad10` waits, sleeping 1 ms at a time, until the alert download has ended; frees
the realm categories (`0x46e470`); suspends the glue; clears `0xb41d90`; unregisters `0x46b930`
from category 5; frees every queued console line without writing it; and ends with the LCD layer's
shutdown (`0x72a200`). The client's stop sequence `0x4036c0` calls it once.

Addresses: `0x46ad10`, `0x46ad20`, `0x46ad25`, `0x46ad33`, `0x46ad38`, `0x46ad47`, `0x46ad51`, `0x46ad97`, `0x46ad9f`, `0x4036c0`, `0x72a200`
Confidence: verified
Proof: one
Test: none

`0x46ad10` to `0x46ad31` loop on `[0xb41e28]` with `OsSleep(1)` (`0x429d50`). `0x46ad51` is
`EventUnregister` (`0x41fd80`) with `ecx` 5 and `edx` `0x46b930`. `0x46ad57` to `0x46ad9c` unlink
the first node after `0xb41d70`'s terminator and `SMemFree` it under `.?AUConsoleString@@` until
the list is empty. `0x4036c0`, called from `ClientContextStop` (`0x403640`), calls it at
`0x4036d4`; no other `e8`/`e9` targets it and no section stores it.

## Script function groups

`0x46abb0` registers seven tables of `{name, function}` pairs with the Lua state, and
`UnloadScriptFunctions` (`0x46ace0`) unregisters the same tables by name, in the same order.

| registrar | unregistrar | table | pairs |
|---|---|---|---|
| `0x51bb20` | `0x51bb40` | `GetTime` to `AppendToFile` | 6 |
| `0x706350` | `0x7063d0` | `GetNumFrames` to `CreateFrame`, then 17 more registrars | 4 |
| `0x46ddf0` | `0x46de20` | `0x8373b8`, `GetBuildInfo` to `PINEntered` | 60 |
| `0x4736c0` | `0x4736e0` | `0x838578`, `SetCharSelectModelFrame` on | 11 |
| `0x471a20` | `0x471a50` | `0x8380f8`, `SetCharCustomizeFrame` on | 24 |
| `0x46f480` | `0x46f4a0` | `0x837ea0`, `RequestRealmList` on | 10 |
| `0x4587f0` | `0x458810` | `PlaySound` to `StopMusic` | 4 |

Addresses: `0x46abb0`, `0x46ace0`, `0x704120`, `0x704160`
Confidence: verified
Proof: one
Test: none

Each registrar loops `0x704120(name, function)` over its table in 8-byte steps to the bound in the
table (`0x30`, `0x1e0`, `0x58`, `0xc0`, `0x50`, `0x20`); each unregistrar loops `0x704160(name)`
over the same bound. `0x706350` also calls 16 registrars and tail-jumps to a seventeenth;
`0x7063d0` calls their partners in the same order. Both glue functions end with a tail jump.

## Cursor

`0x46adb0` loads `Interface\Cursor\Point.blp` as a texture handle and keeps it at `0xb41d88`; the
build hands it to the top. `0x46ae30` closes the handle, releasing it when its count reaches
zero, and clears `0xb41d88`.

Addresses: `0x46adb0`, `0x46ae0e`, `0x46ae16`, `0x46ae30`, `0x46ae3a`, `0xb41d88`, `0x449610`, `0x41aed0`
Confidence: verified
Proof: one
Test: none

`0x46ae0e` is `0x449610` with `ecx` the path, `edx` 2 and 1 pushed (`ret 4`); it allocates under
`HTEXTURE`. The texture flags it builds (masked with `0xffffc300`, OR `0x200`) and the status on its stack
are passed to nothing; the status is destroyed at `0x46ae1b`. `0x41aed0`
decrements the handle's `+0x4` and calls its slot 0 with 1 at zero. `CSimpleTop::SetCursor`
(`0x764d50`) copies 1024 dwords from the texture (`0x764d96`).

## Glue effect pair

After `FRAMES_LOADED`, `0x46a6f0` makes the glue's full-screen effects once, guarded by
`0xb414ec`: it starts the effect system (`0x6ccfc0`), then builds the glow (40 bytes, `0x6cc130`)
at `0xb414c4` and the death effect (24 bytes, `0x6cc690`) at `0xb41468`, null on a failed
allocation. `0x46a770`, from the suspend, clears the guard, deletes both and shuts the effect
system down (`0x6cd1d0`); with the guard clear it does nothing.

Addresses: `0x46a6f0`, `0x46a6f9`, `0x46a723`, `0x46a752`, `0x46a770`, `0x46a781`, `0x46a7a0`, `0xb414ec`, `0xb414c4`, `0xb41468`
Confidence: verified
Proof: one
Test: none

The allocations are `CGlueMgr.cpp` lines 296 and 297. `0x6cc130` stores its vtable and passes
the option name `ffxGlow` (`0x6cc178`); `0x6cc690` passes `ffxDeath` (`0x6cc6c3`). Deletion is
`call [eax]` with 1. The pointers' other readers are `0x46fa74`, `0x472fde` and `0x472ff7`.

## Console line queue

`0x46a610(format, …)` adds a line for the console from any thread: while `0xb41d90` is set and
the format is not empty, it allocates a 264-byte node, formats into its 256-byte text and links it at the tail of the list at `0xb41d70`, under the lock `0xb414cc`. The per-frame
function writes each line to the console in order and frees it under the same lock.

Addresses: `0x46a610`, `0x46a61a`, `0x46a62e`, `0x46a64e`, `0x46a6b8`, `0x46a6cb`, `0xb41d70`, `0xb414cc`, `0x46b9dc`
Confidence: verified
Proof: one
Test: none

`0x46a64e` is `SMemAlloc(0x108)` under `.?AUConsoleString@@`. `0x46a6a4` to `0x46a6b8` link the
node before the terminator: its link takes the old last link, the old last's next becomes the
node. `0x46a6cb` is `SStrVPrintf(node + 8, 0x100, format, &args)`. `0x46b9c3` to `0x46ba3b` in
`0x46b930` call `0x63cb50(text, 0)` for each node, then unlink and free them. Callers:
`0x46b113` (a logon state line) and `0x5aa83d` (`LogConnectionOperation`). The list head's link
offset is 0 and its terminator links are at `+0x4` and `+0x8` (`0x46a2e0`).

## Empty hooks

`0x46c6a0`, called after `FRAMES_LOADED`, and `0x46c6b0`, called by the suspend, are a lone
`ret` each.

Addresses: `0x46c6a0`, `0x46c6b0`, `0x46aa39`, `0x46acce`
Confidence: verified
Proof: one
Test: none

Each is the byte `c3` followed by padding to the next start. The census finds one call each, at
`0x46aa39` and `0x46acce`, and no stored address.
