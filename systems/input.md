# input

The world input code: the key-binding commands that drive the mouse and the player, and the
state they share.

The object file's functions pass the source path `...\UIUtil\InputControl.cpp` and a line number
in their calls. Its state is one `CInputControl`. A table pairs the 26 movement and mouse binding
commands with their handlers. Each press or release goes through `0x515090` to the press handler
`0x514840` or the release handler `0x514b70`, which track the held commands, the mouse travel
since the first press, and the click the press armed.

This doc covers the mouse gesture as far as it decides a click, the movement commands and the per-frame
input tick that turns the command bits into movement.

Roots: `0x515090`, `0x514400`, `0x5144c0`, `0x514b40`, `0x5143e0`, `0x514120`, `0x514190`, `0x514640`, `0x5144e0`, `0x514560`, `0x5145b0`, `0x514da0`, `0x513de0`

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

## Joystick setting

Start-up registers `Joystick` (help `enable joystick control`, default `0`) before it creates the
input control. When the new text begins with the character `1` and the handle `0x8500b4` is −1, its
callback calls `0x7a71d0(0)` and keeps the result there; any other text, `2` and `true` included,
closes a handle that is not −1 and stores −1. Closing calls `0x7a7320`, which only returns, so nothing is released. The
callback returns 1 either way.

Addresses: `0x5142a0`, `0x5142bc`, `0x5142cc`, `0x5142d7`, `0x5142f0`, `0x5142f6`, `0x5142fb`, `0x514306`, `0x514321`, `0x8500b4`, `0x7a71d0`, `0x7a7320`
Confidence: verified
Proof: one
Test: none

`0x5142bc` registers the setting with flags 5 and the callback `0x5142f0`; `0x5142cc` allocates the
control and `0x5142d7` constructs it (Input state). `0x5142f6` compares the first byte with 0x31;
`0x5142fb` tests the handle against −1 and `0x514306` calls `0x7a71d0` with `ecx` 0. The other path
calls `0x7a7320` (`0x514321`) when the handle is not −1 and stores −1. `0x7a71d0` is not read past its
start. `0x5142a0` is called from `WowClientInit` (`0x402b6a`).

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

## Toggle run

The Lua function `ToggleRun` (`0x513d50`, bound in the table `0x8500b8`) flips the walk mode of the
unit the client is driving, and refuses silently, returning no result, in this order: the action
gate `0x494a50(0)` returns 0 (`ui/protected`); the GUID at `0xc4da98` does not resolve with type
mask 8; the unit's health dword is not above 0 (signed); a movement spline exists and its bit
`0x4` is clear; health again; its movement flags have `0x1000` or `0x200`; its stand state is 7
(dead). Otherwise it calls `0x60e080` with the input clock `0xcf0bc8`
(`collision/walkmode#walk-toggle-request`).

Addresses: `0x513d50`, `0x513d52`, `0x513d59`, `0x513d77`, `0x513d7e`, `0x513d8e`, `0x513d93`, `0x513da5`, `0x513da9`, `0x513db6`, `0x513dc1`, `0x513dc4`, `0x513dc6`, `0x513dcd`, `0x513dd2`, `0xc4da98`, `0xcf0bc8`, `0x468460`, `0x60e080`, `0x850340`, `0x8500b8`
Confidence: verified
Proof: one
Test: none

The unit is `0x468460` (mask in `ecx` = 8) on the GUID pair `0xc4da98`/`0xc4da9c`, the unit the
client moves (`objectlayer/unitmovement#the-active-mover`), so a controlled non-player unit is
toggled. `[unit+0x110]` is the field block plus `0x18` (`0x5fad25`, `0x5fad28`): `+0x40` is field
22 (health) and `+0x210` field 138 (the stand state byte). The tests are `jle` after `test esi,esi`
(`0x513d93`, `0x513db6`), `je` on `test BYTE [spline+0x18],4` (`0x513da5`, `0x513da9`), `test bh,0x12`
on `CMovement+0x40` through `[unit+0x118]` (`0x513dc1`) and `cmp BYTE [desc+0x210],7`
(`0x513dc6`). A spline pointer of 0 skips its test.

## The commands

Each Lua movement function is a pair of handlers that call `0x515090(bits, press, time, 0)` on the input control (`0x5143e0`) once `0x494a50(0)` allows it (the gate that refuses a tainted caller). The bits in the word `+0x4`: MoveForward 0x10, MoveBackward 0x20, StrafeLeft 0x40, StrafeRight 0x80, TurnLeft 0x100, TurnRight 0x200, PitchUp 0x400, PitchDown 0x800,
the mouse turn command 0x1 and the mouse camera command 0x2. `ToggleAutoRun` (`0x513de0`) sets or clears 0x1000 by inverting the current bit. `Jump` and `ToggleRun` do not use the word.

Addresses: `0x513e20`, `0x513e50`, `0x513e80`, `0x513eb0`, `0x513ee0`, `0x513f10`, `0x513f40`, `0x513f70`, `0x513fa0`, `0x513fd0`, `0x514000`, `0x514030`, `0x514060`, `0x514090`, `0x5140c0`, `0x5140f0`, `0x514120`, `0x514160`, `0x514190`, `0x5141d0`, `0x513de0`, `0x513e14`, `0x5143e0`, `0x494a50`, `0x515090`, `0x8500b8`
Confidence: verified
Proof: one
Test: none

The pairs are Start and Stop, the Start passing press 1. `0x515090` sends a press to `0x514840` and a release to `0x514b70`; each answers whether a bit changed, and nothing else happens when none did. A press that changed a movement bit (0x10f0, or both mouse buttons) first cancels a cast whose spell has `InterruptFlags` bit 0 and a channel whose spell has bit 0x8 at `+0x5c` (`spell/cancel`).
It ends by running the tick with the press flag set (`0x5151a0`).

## The tick

`0x514640(time, apply)` acts on the unit the client drives (`0xc4da98`, type mask 8); with none it returns. With `apply` nonzero it evaluates `0x514560` (may translate) and `0x5145b0` (not stunned); with `apply` zero both read as false. If it may translate, it runs the forward and strafe emitters (`0x514da0`, `0x514e80`). If it may not, it stops auto-movement (`0x60fb60(0, 1)`) when the unit is also stunned, stops a pending forward move (bit 0x10000) and strafe move (0x20000) and clears the autorun bit 0x1000. Then a unit that is not stunned runs the turn emitter (`0x514f50`) and, while swimming, the pitch emitter (`0x515010`); a stunned unit stops a pending turn (bit 0x40000) and pitch (0x80000) instead.

Addresses: `0x514640`, `0x51464c`, `0x514692`, `0x514697`, `0x5146ab`, `0x5146b0`, `0x5146c3`, `0x5146c9`, `0x5146d6`, `0x5146db`, `0x51470c`, `0x51473d`, `0x514748`, `0x514755`, `0x51475b`, `0x514766`, `0x514777`, `0x514785`, `0x51479c`, `0x5147be`, `0x5147f5`, `0x60fb60`, `0x514f50`, `0x515010`, `0x514da0`, `0x514e80`
Confidence: verified
Proof: one
Test: none

The two predicates are independent: a root leaves turning, a stun removes it. The tick has 13 callers: the setter tail, the stun edge below, `0x514490` (a release of everything) and ten in the unit and player code. The call at `0x5146d6` is reached only when the unit may not translate and is not allowed to turn: a dead unit, or a stunned one.
The autorun clear at `0x514748` is on the may-not-translate leg only.

## May it move, may it turn

`0x5144e0(unit)` is the precondition of both: the unit exists, `UNIT_FIELD_HEALTH` is above 0 (signed), its movement object's auxiliary record (`+0xa4`) is absent or has bit 0x4 at `+0x18`, it is not in a current animation whose `AnimationData` word (column 3) has bit 0x80 (the knockdown lockout, `0x60f5b0`), and it is not the active player with far sight engaged (`+0x1c70` bit 0).
`0x514560` adds that the movement flags (`+0x40` of the movement object) have none of 0x1200 and the stand state is not 7. `0x5145b0` adds that `UNIT_FIELD_FLAGS` has bit 0x40000 clear. A dead unit therefore fails both: it neither moves nor turns.

Addresses: `0x5144e0`, `0x5144f8`, `0x5144fd`, `0x514506`, `0x514516`, `0x514525`, `0x514537`, `0x514540`, `0x60f5b0`, `0x5fa6d0`, `0x514560`, `0x514568`, `0x51458c`, `0x514591`, `0x5145b0`, `0x5145b8`, `0x5145d2`, `0x5145d4`
Confidence: verified
Proof: one
Test: none

The same movement test (`test dh,0x12`) is inline in `Jump` (`0x513cee`) and `ToggleRun` (`0x513dc1`). `UNIT_FIELD_FLAGS` bit 0x40000 is read through `not; shr 0x12; and 1`. The `+0x40` word is signed 32-bit, as the `UnitHealth` function clamps it (`unitplayer`, `ui/party#reading-a-member-without-its-object`).

## Mouse turning and the camera

A mouse motion (`0x514400`, the delta message class) first rotates the camera and then hands the new facing to the body only when `0x5145e0(unit)` holds: not stunned (so alive), the mouse-turn bit 0x1 is held and the unit's stand state is 0. The hand-off functions `0x5152e0` and `0x515330` repeat the health, flags and stand tests (`0x5151b0`, `0x515250`).
A dead player therefore still orbits the camera and cannot turn the body.

Addresses: `0x514400`, `0x514446`, `0x51446f`, `0x514474`, `0x514479`, `0x5145e0`, `0x5145eb`, `0x5145fa`, `0x51460c`, `0x5103e0`, `0x5152e0`, `0x515330`, `0x5151b0`, `0x5151e1`, `0x5151e6`, `0x515250`, `0x51527e`, `0x515283`, `0x50fee0`, `0x60de30`, `0x60de70`
Confidence: verified
Proof: one
Test: `ui::glue_503_514400_abs_accumulate`

The stand state is slot `+0xa4` of the unit, which for the active player returns the predicted value. Nine call sites reach the facing and pitch setters `0x60de30` and `0x60de70`, and each sits behind a health test.

## A stun re-runs the tick

When `UNIT_FIELD_FLAGS` of the active player changes bit 0x40000, `0x5ff4e0` runs the tick with the new flags (`0x5ff501`), so a stun that begins stops a turn at once and one that ends lets pending input apply. The other incapacitation bits (0x1c20000) only refresh the usable-action events.

Addresses: `0x5ff4e0`, `0x5ff4ef`, `0x5ff4fa`, `0x5ff501`, `0x5ff506`, `0x5ff580`
Confidence: verified
Proof: one
Test: none

The delta word is the xor of the old and new flags. The rest of the function is `actionbar` and `chatchannels`.

## The forward axis

`0x514da0` adds up the forward direction: autorun (0x1000) +1, MoveForward (0x10) +1, the mouse turn and mouse camera commands both held (0x1 and 0x2) +1, MoveBackward (0x20) -1. A nonzero total starts movement forward (positive) or backward (negative) through `0x60dc60`; a zero total with the latch bit 0x10000 set stops movement and clears the latch. A start sets the latch.

Addresses: `0x514da0`, `0x514dac`, `0x514db7`, `0x514dbc`, `0x514dc5`, `0x514dcc`, `0x514dce`, `0x514de0`, `0x514de8`, `0x514dfe`, `0x514e0c`, `0x514e23`, `0x514e38`, `0x514e47`, `0x514e69`, `0x514e73`, `0x60dc60`, `0x60dc90`, `0x60be30`, `0x5eb830`
Confidence: verified
Proof: one
Test: none

The stand-up (`0x60be30`, at `0x514e0c`) and the away-from-keyboard clear for a player (`0x5eb830`, at `0x514e23`) run on every tick with a nonzero total, before the latch test at `0x514e34`, not only on a start. A positive total is silent when the latch is set and the unit is not moving backward. A negative total is also silent when the latch is set and movement flag 0x1 is clear (`0x514e60`). The latch is bit 16 of the input word and records that a start was sent; it is not a key. Holding S and then toggling autorun gives a total of 0 and stops the unit while S stays down; pressing S while autorun is on removes autorun first (below) and reverses.

## What clears autorun

Six writers clear the autorun bit. The second press of `ToggleAutoRun` (a release through `0x514b70`); any press whose bits include 0x10 or 0x20, inside `0x514840` at `0x514a5a`, before the axis is computed; the transition into both mouse buttons held (`0x514a73`); the tick, when the unit may not translate (`0x514748`);
`0x514630`, which clears 0x1000 and 0x10000 for the force-stop routine `0x60e7f0`; and the one-shot release of everything `0x5144c0` (bits -1). A key release never clears it.

Addresses: `0x514a52`, `0x514a55`, `0x514a5a`, `0x514a68`, `0x514a73`, `0x514748`, `0x514630`, `0x60e7f0`, `0x60e83f`, `0x5144c0`, `0x5144d2`, `0x514b70`, `0x514c16`, `0x514c30`, `0x513e14`
Confidence: verified
Proof: one
Test: none

Writer 2 tests the mask being set, not the key: strafe, turn and mouse presses miss it. The bit `0x1000` is pushed by one call to `0x515090` only, in `ToggleAutoRun`. The press helper returns early when the bit is already set, so key autorepeat does nothing.

## What a press latches

The first primary command to go down (`0x514840` with no primary held before and bit 0x1 or 0x2 now) zeroes the two travel sums, stores the press time at `+0x14` and enters the camera's freelook (`0x50f930`, a jump to `0x50fe30`). A second primary pressed while the first is held does none of this.
The world pick under the cursor is made once, at the press, by `0x514810` (its only caller `0x51482d`), and the release replays it; nothing re-picks on a move or on the release, and no cursor position is stored. A press of any command bits other than the held primary channel alone clears the armed click mode (`0x514ac1`).

Addresses: `0x514840`, `0x5148f4`, `0x514905`, `0x514910`, `0x514919`, `0x51491f`, `0x514ab4`, `0x514ac1`, `0x514810`, `0x514822`, `0x51482d`, `0x481f00`, `0x50f930`, `0x50fe30`
Confidence: verified
Proof: one
Test: none

The pick result is held in the world frame (`+0x350`, `+0x358`, `+0x35c`). The chord clear means that holding both mouse buttons, or pressing a movement key while one is down, leaves the release without a click.

## Orbit and mouselook

The camera orbit of a held mouse button and mouselook are the same state, bit 0 of the camera's `+0x90` word. It is entered by `0x50fe30` from the press, and by the toggle `0x50f910` (the `ToggleMouseMove` binding), and left by `0x50fdd0`, called from the release through `0x50f940` and from the toggle. Entry has no motion threshold. `MouselookStart` and `MouselookStop` (`0x514210`, `0x514240`)
press and release command bit 0x1 with click mode 0, so they arm no click. `IsMouselooking()` (`0x514270`) pushes 1 when command bit 0x1 is held, else nil: the channel, not the camera's bit.

Addresses: `0x50f930`, `0x50f940`, `0x50f910`, `0x50fe30`, `0x50fdd0`, `0x514210`, `0x514240`, `0x514270`, `0x514c48`, `0x50b5a7`
Confidence: verified
Proof: one
Test: none

`0x50f940(arg)` leaves the freelook, then sets bit 0x20 of the camera word when its argument is nonzero and clears it otherwise; every mouse command passes 0 but `CameraOrSelectOrMoveStop`, which computes it with `0x6f1c10`. Freelook can be entered by channel 2 alone, so the channel and the camera bit are correlated and not equal.

## A click and an orbit together

The orbit starts at the press and the select is decided at the release from the latched pick, so one gesture can do both. With `t` the release interval: under 200 ms the release always clicks, however far the mouse swept; from 200 to 799 ms it clicks while both travel sums stay under 8.0; from 800 ms it never does. A right-button release that clicks interacts with the picked object,
which can stand a sitting player (`StartAttack` `0x5ecb70` reaches `0x5ed430(0)`, and `0x611130` calls `0x60be30(0)`).

Addresses: `0x514ae0`, `0x514af3`, `0x514b24`, `0x514b2a`, `0x514b2e`, `0x514b70`, `0x514d4b`, `0x514d79`, `0x481f60`, `0x60be30`, `0x5ecb70`
Confidence: verified
Proof: one
Test: none

The clock is the input record's stamp (`0xcf0bc8`), set by the input ring enqueue and read in milliseconds; the press and release use the same one. The regime table repeats `Click versus drag` and adds that motion past 8.0 in under 200 ms still clicks.
