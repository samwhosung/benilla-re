# input

The world input code: the key-binding commands that drive the mouse and the player, and the
state they share.

The object file's functions pass the source path `...\UIUtil\InputControl.cpp` and a line number
in their calls. Its state is one `CInputControl`. A table pairs the 26 movement and mouse binding
commands with their handlers. Each press or release goes through `0x515090` to the press handler
`0x514840` or the release handler `0x514b70`, which track the held commands, the mouse travel
since the first press, and the click the press armed.

This doc covers the mouse gesture as far as it decides a click. The movement commands in the
same object file are not read here.

Roots: `0x515090`, `0x514400`, `0x5144c0`, `0x514b40`, `0x5143e0`, `0x514120`, `0x514190`

## Input state

One `CInputControl` exists: 28 bytes, allocated with the tag `...\UIUtil\InputControl.h`
(`0x850378`), built by `0x5143c0` and kept at `0xbe1148`, which `0x5143e0` returns. The
constructor clears the held commands (`+0x04`), the press time (`+0x14`) and the click mode
(`+0x18`), stores the client clock at `+0x00`, and leaves the travel sums alone.

Addresses: `0x5143c0`, `0x5143e0`, `0xbe1148`, `0x850378`, `0x850340`
Confidence: verified
Proof: one
Test: none

The allocation passes 28 and `0x850378` to `SMemAlloc` (`0x6462e0`), calls `0x5143c0` on the
result and stores it at `0xbe1148`; the only other stores there write 0, on a failed allocation
and at teardown. `0x5143c0` stores 0 at `+0x04`, `+0x14` and `+0x18` and `0x42c010()` at
`+0x00`. `0x5143e0` is `mov eax,[0xbe1148]; ret`. The source path is `0x850340`, passed with a
line number at `0x51445d` in `0x514400` and at `0x514948` in `0x514840`.

## Mouse commands

The binding table `0x8500b8` pairs each command's name with its handler. `TurnOrActionStart`
(`0x514120`) arms click mode 2 and presses command bit `0x1`; `CameraOrSelectOrMoveStart`
(`0x514190`) arms click mode 1 and presses bit `0x2`. Each does both only when `0x494a50(0)`, a
check not read here, returns nonzero. Arming (`0x514810`) stores the mode only when no
primary command, `0x1` or `0x2`, is held, and for a nonzero mode picks the world object under
the cursor with `0x481f00`. `0x515090` sends a press to `0x514840` and a release to `0x514b70`.

Addresses: `0x8500b8`, `0x514120`, `0x514190`, `0x494a50`, `0x514810`, `0x515090`, `0x481f00`
Confidence: verified
Proof: one
Test: none

The table holds 26 name and handler pairs, from `Jump` at `0x8500b8`; `TurnOrActionStart` with
`0x514120` is at `0x850150`, `CameraOrSelectOrMoveStart` with `0x514190` at `0x850160`, and the
matching stops, mouselook and `IsMouselooking` end it. `0x514120` calls `0x494a50` with `ecx` 0
and skips the rest on 0 (`0x514131`), then calls `0x514810` with 2 and `0x515090` with commands
1 and press 1; `0x514190` passes 1, then 2 and 1. In `0x514810`, `test byte ptr [ecx+4],3`
skips the store for a nonzero mode while a primary is held, and `0x51482d` calls the pick.
`0x515090` tests its second argument and calls `0x514840` (`0x5150a7`) or `0x514b70`
(`0x5150b9`).

## Click versus drag

When the last held primary command is released, the click is dispatched only if `0x514ae0`
returns 0. With `t` the release time minus the press time, as a wrapping `int32` in
milliseconds: it returns 1 when `t ≥ 800`; otherwise, when either travel sum is at least 8.0, it
returns 1 if `t ≥ 200` and 0 if not; otherwise 0. So a release within 200 ms always clicks, and
one within 800 ms clicks while both sums stay under 8.0. The sums are raw mouse motion scaled by
the screen aspect: 8.0 is 10 raw units of x or 13.33 of y at 4:3, and 9.18 or 16.32 at 16:9.

Addresses: `0x514ae0`, `0x514b70`, `0x514840`, `0x514400`, `0x808b5c`, `0x41ad10`, `0x41ad80`, `0x766f70`
Confidence: verified
Proof: two
Test: `screencoord::sc_41ad10_set_scale`, `screencoord::sc_41ad80_scale_xy`

`0x514aeb` `lea eax,[edx−0x320]`, `jl`: 800. `0x514b01`/`0x514b11` `fcomp [0x808b5c]` (8.0),
`test ah,1`, `je`: at least 8.0, NaN under. `0x514b24` adds −200, `setge`. The release handler
`0x514b70` calls it at `0x514d4b` only when a primary was held before the release (`0x514ba4`,
`0x514ba8`) and none after (`0x514c35`). On 0 it dispatches `0x481f60(1)` for click mode 1 and
`0x481f60(4)` for mode 2, else it clears the mode.

The press time is latched at `0x514919` when the first primary goes down, which also zeroes both
sums (`0x514910`). With a command held, `0x514400` adds each mouse message's absolute x and
y motion times `K1` and `K2` to the sums (`0x41ad80`, called at `0x766fb1` in `0x766f70`).
`0x41ad10` sets them, at `0x832a44` and `0x832a48`, from an aspect `a`: `K2 = 1/√(a² + 1)` and
`K1 = a·K2` with `K2` unrounded in `st0` after `fst` (`0x41ad3e`). Tests run both whole.
`0x46ab40` gives `a` as `gxResolution`'s width over height, or 4/3 with the `widescreen` option
(default `1`) off.

Times are client clock values (`0x42c010`) in milliseconds: the input queue stamps messages at
`0x42d7bb` in `0x42d780`, and `0x42b8b0` scales the counter to 1000 per second. `0x5144c0`
releases every held command through `0x514b70`, and `0x514b40` asks `0x514ae0`, both at the
current clock.
