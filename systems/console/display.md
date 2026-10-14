# console: display

The display settings the console file keeps for the graphics device: the 14-dword device-format
record, the CVars that change it, their validation, the device start-up that picks a format and creates
the device, and the restart and defaults commands. The CVar table of the whole client is
`tables/cvars.tsv`; this doc holds what the console file does with the display ones.

Roots: `0x63a230`, `0x63b440`, `0x639a20`, `0x639a00`, `0x639f60`, `0x63aa00`, `0x63adb0`, `0x63ae40`, `0x63aed0`, `0x63afe0`, `0x63b030`, `0x63b160`, `0x63b190`, `0x63b1c0`, `0x63b1f0`, `0x63b220`, `0x63b250`, `0x63b290`, `0x63b310`, `0x63b340`

## Device format record

A device format is 14 dwords: bytes `+4` hardware transform-and-lighting (true unless `-swtnl`), `+5` hardware
cursor, `+6` fix-lag, `+7` windowed, `+8` constrain aspect and `+9` maximise; `+0xc` depth index, `+0x10` width,
`+0x14` height, `+0x18` triple buffering (1 or 2), `+0x1c` multisample count, `+0x20` multisample quality (a float),
`+0x24` colour index, `+0x28` refresh rate and byte `+0x2c` vertical sync. The working record is `0xc4eb98`;
`0xc4ea08` keeps the last format the device accepted; fixed records hold 640 × 480 (colour 0, depth 4) at `0xc4ebd0`
and 800 × 600 (1, 5) at `0xc4e6fc`, both 60 Hz. Colour index 0, 1, 3 is 16, 24, 30 bits; depth index 4, 5, 7 is 16, 24, 32.

Addresses: `0xc4eb98`, `0xc4ea08`, `0xc4ebd0`, `0xc4e6fc`, `0x5898d0`, `0x589900`, `0x638b10`, `0x638b80`, `0x86460c`, `0x86462c`, `0xc4eb9c`, `0xc4eb9d`, `0xc4eb9e`, `0xc4eb9f`, `0xc4eba0`, `0xc4eba1`, `0xc4eba4`, `0xc4eba8`, `0xc4ebac`, `0xc4ebb0`, `0xc4ebb4`, `0xc4ebb8`, `0xc4ebbc`, `0xc4ebc0`, `0xc4ebc4`, `0xc4ebc8`
Confidence: verified
Proof: one
Test: none

`0x589900` stores its eleven arguments into the record (`+7` from the first, the size from the second,
`+0x24` the third, `+0xc` the fourth, `+0x28` the fifth, `+0x2c` the sixth, `+4`, `+6`, `+5`, `+8`
and `+9`) and sets `+0x18` and `+0x1c` to 1 and `+0x20` to 0. The static initializers `0x638b10` and
`0x638b80` call it with `(0, 640×480, 0, 4, 60, 1, 1, 0, 1, 1, 0)` and `(0, 800×600, 1, 5, 60, 1, 1, 0, 1, 1, 0)`.
`0x5898d0` is the plain constructor used for `0xc4eb98` and `0xc4ea08`. The index table `0x86460c`
maps an index to its text: 0 `16`, 1 `24`, 2 `24`, 3 `30`, 4 `16`, 5 `24`, 6 `24`, 7 `32`.

## Display CVars

`ConsoleDeviceInitialize` registers `hwDetect` (default 1), and `0x63a720` registers 16 more, all in category 1:
`widescreen` (1), and with flags 3 `gxColorBits` (16), `gxDepthBits` (16), `gxResolution` (`640x480`),
`gxRefresh` (75), `gxTripleBuffer` (0), `gxApi` (`direct3d`), `gxVSync` (1), `gxWindow`, `gxAspect` (1),
`gxMaximize`, `gxCursor` (1), `gxMultisample`, `gxMultisampleQuality` (0.0), `gxFixLag` and `gxOverride` (empty).
`gxWindow` and `gxMaximize` default to 1 for the locale zhCN and 0 otherwise. `gxMultisample` and
`gxFixLag` have no literal default: each formats the `multisample` or `fix-lag` column of the matched
`VideoHardware.dbc` row (`+0x54` and `+0x50`).

Addresses: `0x63a720`, `0x63a230`, `0x63a259`, `0x63a74f`, `0x63a910`, `0x63a97b`, `0xc4e6e4`, `0x64154e`, `0x63db90`, `0x6cd016`, `0x6cd5da`, `0x6cd944`, `0x6cdaa8`, `0xc0e080`, `0x82e748`, `0x82e570`, `0xc4e954`, `0xc4e9a0`, `0xc4e9dc`, `0xc4e9e0`, `0xc4ea7c`, `0xc4ea84`, `0xc4ea90`, `0xc4ea94`, `0xc4eab0`, `0xc4eab4`, `0xc4eab8`, `0xc4eac4`, `0xc4eacc`, `0xc4eb88`, `0xc4ec08`, `0xc4ec0c`
Confidence: verified
Proof: one
Test: none

`0x63a72f` loads the locale index and `0x63a738` compares it with 4 (zhCN); `0x63a85e` to `0x63a871` and `0x63a8b9` to
`0x63a8c8` pick the default text `1` or `0` for `gxWindow` and `gxMaximize` from it. `0x63a910` and `0x63a97b`
read `[0xc4e6e4] + 0x54` and `+ 0x50`, and `0x64154e` stores the matched row there. The names,
defaults, categories and callbacks of all 17 are the rows of `tables/cvars.tsv` for the sites `0x63a259` to
`0x63a9e7`. The record's fields are written by the callbacks below, not by these registrations. The full-screen effects code looks up `gxMultisample` by name once (`0x6cd016`) and compares the integer of its record with 2 and with 1 to choose its route (`0x6cd5da`, `0x6cd944`, `0x6cdaa8`).

## Display callbacks

Each CVar's change callback validates the new text and, when it accepts, writes a field of the working record
and prints `set pending gxRestart`. `gxColorBits` accepts 16, 24 and 30 (index 0, 1, 3), `gxDepthBits` 16,
24 and 32 (4, 5, 7), `gxRefresh` one of 60, 70, 72, 75, 85, 90, 100, 120, 160, 180 and 200, `gxTripleBuffer` 0
or 1 (stored as 1 or 2), `gxApi` `OpenGL` or `Direct3D` (nothing stored). `gxVSync`, `gxWindow`, `gxAspect`,
`gxMaximize`, `gxCursor` and `gxFixLag` store whether the number is nonzero. `gxMultisample` stores
the number clamped to 1 through 16; `gxMultisampleQuality` clamps to 0 through 1. A rejection writes
the reason and returns 0.

Addresses: `0x63adb0`, `0x63ae40`, `0x63aed0`, `0x63afe0`, `0x63b030`, `0x63b160`, `0x63b190`, `0x63b1c0`, `0x63b1f0`, `0x63b220`, `0x63b250`, `0x63b290`, `0x63b310`, `0x63b340`, `0x64ac60`, `0x8645e0`, `0x80e128`, `0xc4ebbc`, `0xc4eba4`, `0xc4ebc0`, `0xc4ebb0`, `0xc4ebc4`, `0xc4eb9f`, `0xc4eba0`, `0xc4eba1`, `0xc4eb9d`, `0xc4ebb4`, `0xc4ebb8`, `0xc4eb9e`, `0x86460c`
Confidence: verified
Proof: one
Test: none

The fields are `0xc4ebbc` (`+0x24`), `0xc4eba4` (`+0xc`), `0xc4ebc0` (`+0x28`), `0xc4ebb0` (`+0x18`),
`0xc4ebc4` (`+0x2c`), `0xc4eb9f` (`+7`), `0xc4eba0` (`+8`), `0xc4eba1` (`+9`), `0xc4eb9d` (`+5`), `0xc4ebb4` (`+0x1c`),
`0xc4ebb8` (`+0x20`) and `0xc4eb9e` (`+6`). All call `0x64ac60` (decimal to integer) except the quality (`0x64aaa0`). The
refresh list is the table `0x8645e0` of 11 words, scanned at `0x63aee4`. The quality tests are `0x63b29c` to
`0x63b2ec`: below 0 gives 0, 1 and above or a NaN gives 1, otherwise the value. `0x63b340` passes its text to the
override parser `0x639950`.

## Resolution

`gxResolution` accepts `<w>x<h>` (any separator character between the numbers: the scan is `%d%c%d`) when it is in a
list. With `widescreen` on, the list is the device's modes with `w / h ≥ 1.248`, `w ≥ 640` and `h ≥ 480`, a mode equal to
the previous one skipped; with it off, or when that list is empty, it is 640 × 480, 800 × 600, 1024 × 768, 1152 × 864, 1280 × 960,
1280 × 1024 and 1600 × 1200. An accepted size is stored in the record (`+0x10`, `+0x14`); a rejected one writes
`invalid resolution, must be one of ` and the list, a new line after 100 characters.

Addresses: `0x63aa00`, `0x5899c0`, `0x496d70`, `0xc4ec08`, `0xc4eba8`, `0xc4ebac`, `0x63aa8a`, `0x63aa9b`
Confidence: verified
Proof: one
Test: `console::resolution_validate_difftest`

`0x63aa2a` scans the text with `%d%c%d` and `0x63aa43` loads the widescreen CVar's integer. `0x63aa8a` to `0x63aa9e`
compute `fild w; fidiv h; fcomp 1.248` and skip a mode on `test ah,1` (below, or a NaN). `0x63aaa0` and `0x63aaa8`
bound the width and height. `0x63aab1` to `0x63aabd` skip a mode equal to the last kept. The fixed list is stored at
`0x63ab0f` to `0x63ac43`. The search is `0x63ac4f` to `0x63ac65`; an accepted size is stored at `0x63ad70`. The test runs
`0x63aa8a` to `0x63aa9b`.

## Start-up record

`ConsoleDeviceInitialize` (`0x63a230`, window title in `ecx`) registers `hwDetect`, matches the hardware (`0x641260`),
registers the display CVars and `gxRestart`, lists the device's modes (`0x5899c0`) and builds the working record. Detection
applies for `-hwdetect`, a nonzero `hwDetect` or a hardware change: the record is the hardware row's default format with the window and
maximise flags of their CVars, and `hwDetect` becomes 0. Otherwise the record is what the callbacks stored. It ends with the
override strings, the window title, the video options (`cvar/options`) and, when detection applied, the quality CVars (`0x639a60`).

Addresses: `0x63a230`, `0x63a720`, `0x63b360`, `0x641260`, `0x5899c0`, `0x419c10`, `0x639c40`, `0xc4ec54`, `0xc4ea7c`, `0xc4e6ac`, `0x639a00`, `0xc4e6e4`, `0xc4e6e8`
Confidence: verified
Proof: one
Test: none

`0x63a259` registers `hwDetect`; `0x63a26b` calls `0x641260` with the record `0xc4e6bc`. `0x63a27c` to `0x63a2ac`
test `-hwdetect` (index 0x23) and the CVar: with neither, the hardware step is skipped; otherwise `hwDetect` is
set to "0". `0x63a363` to `0x63a3a1` copy the default format `[0xc4e6ac]` and write the record to the CVars.
`0x63a3a8` to `0x63a3ec` choose the API; `0x63a400` to `0x63a416` set `+4` from `-swtnl` (index 6); `0x63a41e` to
`0x63a443` the window flag from `-fullscreen` (index 0xc) and `-windowed` (0x1f). `0x63b360` registers `gxRestart`
with the handler `0x639f60`. The API is Direct3D unless `gxApi` is `OpenGl` or `-opengl` is given (`-d3d` overrides); `-fullscreen` wins over `-windowed` for the window flag; `-swtnl` clears `+4`.

## Device creation

The device is created from a copy of the record by `0x589ac0` with the window procedure `0x42cfe0`. On failure the copy is
lowered (`0x63b380`): the multisample count by 2, to at least 1; then, while the colour index is above 0, the depth index down to 4,
then the colour index by one with the depth index back at 7 (4 at colour 0). When it cannot be lowered, the second fixed record is
tried once and lowered the same way. With no device, or one with fewer than 2 texture units or no streams, the client shows an
error (the locale's startup text, else English) and exits.

Addresses: `0x5899f0`, `0x589ac0`, `0x42cfe0`, `0x63b380`, `0x63b3d0`, `0x63a060`, `0x58a230`, `0x435f00`, `0x40a30d`, `0x58b600`, `0xc4ec1c`, `0x63b440`
Confidence: verified
Proof: one
Test: none

The loop is `0x63a47c` to `0x63a4d7`: `0x63b380` takes 2 off `+0x1c` when it exceeds 1, else decrements `+0xc` while above 4,
else decrements `+0x24` and sets `+0xc` to 7 while the colour index is nonzero, else 4. When it returns 0 the fallback copy
is `0x63a4a4` to `0x63a4b6` (done once, flag in `bl`). `0x63b3d0` raises a candidate below 800 × 600 to that size, a depth index below 5 to 5 and a colour index below 1 to 1, each only when the hardware
default allows it, and never touches `+0x1c`. The failure text is chosen at `0x63a561` to
`0x63a5e3`; `0x63a504` and `0x63a61b` test the capabilities (`0x58a230`: word `+0` below 2, word `+0xc` below 1);
`0x435f00` shows the text and `0x40a30d(0)` exits. When the desktop query `0x5899f0` succeeds at start-up, the second fixed record takes the desktop mode's size and refresh rate, and colour class 1 above 16 bits.

## Format validation

`0x63a060` makes a record agree with the device's modes. It builds, once, seven stock sizes from 1600 × 1200 down to
640 × 480. If no device mode has the record's width and height, it takes the first stock size smaller in both and searches again,
until a mode matches or the list ends. It keeps the record's refresh rate when a mode of that size and format word has it;
otherwise it takes the lowest refresh of those modes, or 60 when none, and logs `ValidateFormatMonitor(): invalid refresh rate <old>, set to <new>`.

Addresses: `0x63a060`, `0xc4ea40`, `0xc4ea9c`, `0x86462c`, `0x58b600`, `0x63a220`, `0xc4e958`
Confidence: verified
Proof: one
Test: none

The stock sizes are stored at `0x63a094` to `0x63a10e`. The size search is `0x63a133` to `0x63a18f`: it compares the
record's width and height with each device mode (16 bytes: width, height, format word, refresh); the stock step
is `0x63a15a` to `0x63a17a` (`cmp stock.w, w; jge` and `cmp stock.h, h; jl`). `[eax*4+0x86462c]` (`0x63a119`) gives the
format word of the record's colour index. With no such mode it first logs `unable to find monitor refresh`. The format word is 16 or 32 by the colour index. The refresh search starts at 9999 (`0x63a129`) and takes the smallest
match (`0x63a1c5` to `0x63a1cc`); an exact match returns at `0x63a1d2`.

## Restart and defaults

`gxRestart` (`0x639f60`) validates the working record, asks the device to switch (`0x589be0`) and on success keeps the record as
the last good one and applies the latched display CVars (`0x639ec0`). On failure it writes `unable to set requested display mode`, retries with
the last good record (`unable to set last good mode`), then the second fixed record (`unable to set default format`), then 640 × 480,
and after any retry writes the working record into the CVars (`0x639c40`). It ends by setting the window title and returns 1.
`RestoreVideoDefaults` (`0x639a20`) sets the working record to the hardware row's default format, writes it, restarts, and sets
the quality CVars with `0x639a60`.

Addresses: `0x639f60`, `0x639a20`, `0x639a60`, `0x639b80`, `0x639bc0`, `0x639c00`, `0x639c40`, `0x639ec0`, `0x589be0`, `0x641820`, `0xc4e678`, `0x63cb10`, `0x435a50`
Confidence: verified
Proof: one
Test: none

`0x639f60` calls `0x63a060` then `0x589be0` with the working record (`0x639f6c`, `0x639f76`); success continues at `0x639f90`, the retries are
`0x639f9a` to `0x63a017` and the write-back is `0x63a02c`. `0x639c40` formats twelve fields into their CVars with `CVar::Set` (`0x63df50`) and `0x639ec0`
calls `0x63e060` on fourteen CVars. `0x639b80`, `0x639bc0` and `0x639c00` set a CVar from a float, an integer
or a flag. The quality values are the words at `0xc4e678` to `0xc4e6b0`, filled by `0x641820`; `0x639a60` sets `farclip`, `lodDist`, `shadowLevel`, `frillDensity`, `fullAlpha`, `doodadAnim`, `trilinear`, `MaxLights`, `pixelShaders`, `specular`, `waterLOD`, `particleDensity`, `unitDrawDist`, `SmallCull`, `DistCull` and `baseMip` from them. `0x63cb10` stores the
window title in `0xc4ec58` and sets it (`0x435a50`).

## Override string

The text `gxOverride`, the hardware row's override text (Direct3D `+0x4c`, OpenGL `+0x48`) and the switch
`-gxoverride` are lists of `<index>,<value>` pairs. A pair with an index below 9 sets that slot's flag
(`0xc4e9e4 + 4·index`) and value (`0xc4e9b8`), and each flagged slot is then applied to the device (`0x589d60`).

Addresses: `0x639950`, `0x589d60`, `0xc4e9b8`, `0xc4e9e4`, `0x419c20`, `0x63a690`, `0x63a6c1`, `0x63b340`
Confidence: verified
Proof: one
Test: none

`0x639950` tokenises with `SStrTokenize` (`0x64ae50`) twice per pair, with the separators ` ,` and ` ;`, converts both tokens with `_atoi` (`0x41449c`) and stores at `0x6399d4` and `0x6399df` when the index is
below 9 (`0x6399cf`). `0x63a690` to `0x63a6c1` call it for the API's row text and the switch string `0x419c20(0x26)`;
`0x63a6c8` to `0x63a6e5` apply the flagged slots.
