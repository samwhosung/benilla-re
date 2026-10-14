# uicamera: transitions

How the camera moves a value from its current setting to a target: the seven channels, the arming
setters, the cosine ease each frame applies, the relative and absolute frames of the yaw and pitch, the bob
decay, and the field of view.

This is `Camera.cpp`. The per-frame update that calls the consumer is `uicamera/driver`; what arms the yaw,
pitch and pivot channels on a key edge is `uicamera/follow`; the speed CVars are `uicamera/cvars`.

Roots: `0x50f160` (consumer), `0x512270` (field of view), `0x512490` (ground pitch), `0x512830` (pitch), `0x512a50` (pivot bias), `0x512c70` (yaw), `0x512120` (distance), `0x5126b0` (height)

## The channels

Seven values of the camera ease toward a target. Each has a live value, a target, a start time in milliseconds, a
duration in seconds, a start value and a flag in the mode word `+0x90` that says a transition is running.
The table gives the live word, the target, the start time, the duration, the start value, the flag and the speed
CVar its setter divides by.

| channel | live | target | start | duration | from | flag | speed CVar |
|---|---|---|---|---|---|---|---|
| distance | `+0xec` | `+0x198` | `+0x190` | `+0x194` | `+0x19c` | 0x4000000 | `cameraDistanceSmoothSpeed` |
| height | `+0xfc` | `+0x1c8` | `+0x1c0` | `+0x1c4` | `+0x1cc` | 0x20000000 | `cameraHeightSmoothSpeed` |
| yaw | `+0xf0` | `+0x210` | `+0x208` | `+0x20c` | `+0x214` | 0x1000000 | `cameraYawSmoothSpeed` |
| pitch | `+0xf4` | `+0x1e0` | `+0x1d8` | `+0x1dc` | `+0x1e4` | 0x2000000 | `cameraPitchSmoothSpeed` |
| pivot bias | `+0x104` | `+0x1f8` | `+0x1f0` | `+0x1f4` | `+0x1fc` | 0x8000000 | `cameraTargetSmoothSpeed` |
| ground pitch | `+0x108` | `+0x1b0` | `+0x1a8` | `+0x1ac` | `+0x1b4` | 0x10000000 | `cameraGroundSmoothSpeed` |
| field of view | `+0x10c` | `+0x228` | `+0x220` | `+0x224` | `+0x22c` | 0x40000000 | `cameraFoVSmoothSpeed` |

Addresses: `0x50f160`, `0x5b7bb0`, `0x512120`, `0x5126b0`, `0x512c70`, `0x512dc0`, `0x512830`, `0x512980`, `0x512a50`, `0x512ba0`, `0x512490`, `0x5125e0`, `0x512270`, `0x5123c0`
Confidence: verified
Proof: one
Test: `ui::glue_50f160_change_detect`, `ui::glue_50f160_rate_frac`

The speed CVar of each setter is read at `0x5121ca` (distance), `0x51274e` (height), `0x512d75` (yaw), `0x512935` (pitch), `0x512b55` (pivot bias),
`0x512595` (ground pitch) and `0x512375` (field of view). Each pair of angle setters is an outer and an inner function.

## The ease

Every frame `0x50f160(target, now)` advances each channel whose live value differs from its target by at least
2^-22. A channel with its flag set and a start time not after now takes `t = (now - start) * 0.001 / duration`. At
`t` of 1 or more the live value becomes the target; below that it is `from + (target - from) * (1 - cos(π t)) / 2`.
A channel whose start is still ahead is left alone. A channel that has reached its target has its flag cleared, its
target set to the live value and its times set to 0. The yaw channel is skipped in mouselook.

Addresses: `0x50f160`, `0x5b7bb0`, `0x40a2b0`, `0x50d720`, `0x50d900`, `0x50dbc0`
Confidence: verified
Proof: one
Test: `ui::glue_50f160_change_detect`, `ui::glue_50f160_rate_frac`

The consumer ends with the pitch clamp `0x50d720` and the terrain tilt kernels `0x50d900` and `0x50dbc0` (`uicamera/effects`). The seven blocks sit at `0x50f20b` (field of view), `0x50f2b8` (distance), `0x50f36a` (height), `0x50f417` (ground pitch), `0x50f4c4` (pivot bias), `0x50f576`
(pitch) and `0x50f623` (yaw, skipped by the test of mode bit 0 at `0x50f629`). Each compares target and live against `0x8029d4` (2^-22). The ease is `0x5b7bb0(from,
target, t)`, which multiplies `t` by π, takes the cosine and forms `a + (b - a) * (1 - cos) * 0.5`.

## Arming a transition

An angle setter `(target, delay, factor, now)` begins at `now + delay * 1000` milliseconds and lasts `factor` times
the angle to cover divided by the speed CVar in radians a second. The angle runs from the live value, wrapped to within π
of the target, to the target. Nothing is armed within 0.001 of the target or on a repeat of the armed target, delay and factor.
Yaw `0x512c70` (inner `0x512dc0`), pitch `0x512830` (`0x512980`), pivot bias `0x512a50` (`0x512ba0`), ground pitch `0x512490`
(`0x5125e0`) and field of view `0x512270` (`0x5123c0`) follow this. Distance `0x512120` (inner `0x512200`) and height `0x5126b0` (inner `0x512790`) do not wrap. A zero
factor gives a duration of 0, and the value arrives at the next frame.

Addresses: `0x512270`, `0x5123c0`, `0x512200`, `0x512120`, `0x5126b0`, `0x512790`, `0x512c70`, `0x512dc0`, `0x512830`, `0x512980`, `0x512a50`, `0x512ba0`, `0x512490`, `0x5125e0`
Confidence: verified
Proof: one
Test: `ui::cam_512270`, `ui::cam_5123c0`, `ui::cam_512490`, `ui::cam_5125e0`, `ui::cam_512830`, `ui::cam_512980`, `ui::cam_512a50`, `ui::cam_512ba0`, `ui::cam_512c70`, `ui::cam_512dc0`, `ui::glue_511_512120_body`, `ui::glue_511_512120_quartet`, `ui::glue_511_commit_if_moved`

`0x5122de` stores the wrapped live value; `0x51230a` to `0x51232c` are the repeat test against the memo words `+0x228`, `+0x234`, `+0x230`; `0x51235f` is the
"already there" return. `0x512387` multiplies the delay by -1000 (0x801448) and truncates; `0x51239e` subtracts that from `now`; `0x5123a1` to `0x5123a9` form the
duration. The inner `0x5123c0(target, duration, start)` wraps the live value again (`0x5123d5` to `0x512426`), stores the start value, target, start time and duration
(`0x51244f` to `0x512471`) and sets the flag.

Each setter runs whole under `ui::cam_` and its address, with the pointer chain to its speed CVar built in memory.

## Relative and absolute frames

The yaw offset is normally relative to the target's facing. Two counters change that. While `+0xa4` is above 0 the
camera is in the absolute frame: the view yaw is the offset alone. Raising the counter from 0 adds the target's facing to
the yaw offsets so the picture does not move, and lowering it to 0 takes the facing off again. `+0xa8` does the same for the
terrain pitch, which is added to the view pitch only while the counter is 0. Mouselook raises both counters when it begins and
lowers them when it ends; the external-move flag of `uicamera/follow` raises the first.

Addresses: `0x50d3b0`, `0x50d440`, `0x50d500`, `0x50d520`, `0x5102b0`, `0x510120`, `0x50fe30`, `0x50fdd0`
Confidence: verified
Proof: one
Test: `ui::cam_510120`, `ui::cam_5102b0`, `ui::glue_50d440_neg_push`

`0x50d3b7` to `0x50d3c2` raise `+0xa4` and test it against 0; `0x50d40d` and `0x50d425` call the yaw integrator `0x5102b0` with the facing (`[unit+0xc98]` for a unit, else slot
0x18). `0x50d447` lowers it and `0x50d4aa` to `0x50d4db` pass the facing negated. `0x50d500` and `0x50d520` do the same for `+0xa8` with the terrain
pitch `+0x108` and the pitch integrator `0x510120`. `0x50fe72` and `0x50fe79` call `0x50d3b0` and `0x50d500` when mouselook begins; `0x50fde6` and `0x50fdeb` call
`0x50d440` and `0x50d520` when it ends.

`ui::cam_510120` and `ui::cam_5102b0` run `0x510120` and `0x5102b0` whole on the camera object's words.

## Field of view

The field of view is the base `+0x40` plus the eased bias `+0x10c`, in radians, kept between 0 and 179 degrees; it is the camera's
vtable slot 0. The base is π/2 (90 degrees). `SetFovOverride(degrees)` with a nonzero value switches at once to view 0 (first
person) and sets the far-sight bit (mode bit 3) and the base to that angle; with 0 it clears the bit, restores π/2 and returns to view 1. With the
bit set the bias counts as 0.

Addresses: `0x50d6d0`, `0x50d320`, `0x512e90`
Confidence: verified
Proof: one
Test: `ui::glue_50d320_deg2rad_set`, `ui::glue_50d6d0_pitch_clamp`

`0x50d6d0` loads 0 when bit 3 is set (`0x50d6d9`), else `+0x10c`, adds `+0x40`, and clamps to the constants 0.0 (0x8089bc) and 3.1241393 (0x8089b8, 179 degrees).
`0x50d331` calls `0x512e90` with view 0 and blend 2, `0x50d33f` sets bit 3 and `0x50d34e` stores the angle; `0x50d360` to `0x50d378` do the reverse.

## Bob decay

When bobbing stops, the bob offsets (`+0x110` to `+0x118`) decay to 0. The duration is the largest offset divided by
`cameraBobbingSmoothSpeed`, started at the stop time from the offsets as they were; the offsets then ease to 0 with
the same cosine. Starting to bob sets mode bit 9 and the start time.

Addresses: `0x511110`, `0x5110f0`, `0x511170`, `0x7bf680`, `0xbe10bc`
Confidence: verified
Proof: one
Test: `ui::glue_511_511110_abs_div`

`0x5110f0` sets bit 0x200 and stores the time at `+0x238`. `0x511110` clears the bit, copies the offsets to `+0x240` to `+0x248`, divides the largest by the CVar into `+0x23c` and
stores the time. `0x511170(flag)` calls the first on a rising flag and the second on a falling one. `0x50f16b` to `0x50f1c4` ease the offsets from the copies.
