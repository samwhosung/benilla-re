# uicamera: the follow camera

The camera that follows the player in the world: its per-frame update, the keyboard and wheel axes that move
it, how the view angles, the pivot and the eye are made, how the camera is pulled in and glides back, and the
fade of the player's own model.

This is `Camera.cpp`. The saved view and the pitch convention are `uicamera/view`; the CVars it reads are
`uicamera/cvars`. The camera object's matrices are `camera`.

Roots: `0x50e8d0` (update), `0x5112d0` (axes), `0x50f710` (angles), `0x512e90` (set a view), `0x50fe30` (mouselook)

## The per-frame update

The world frame resolves the camera's target, a GUID at `+0x88`, and calls the driver `0x50e8d0(target, now)`
every frame. A null target raises an error. Unless mode bit 2 is set, `0x50ca90(target)` must succeed or the update
stops. Then in order: the auto-return edge (`0x5111f0`); the target's position, compared with the last one at `+0x174`;
the keyboard axes (`0x5112d0`, when any is active) and the transitions (`0x50f160`); the view angles (`0x50f710`);
bobbing (`0x511920`) and the water bands (`0x511ad0`); the near and far solver (`0x50e570`); the eye (`0x50de00`); the pull-in
glides, the self fade (`0x50f8f0`) and the pivot height (`0x50f880`). A taxi ride arms the yaw offset back to 0.

Addresses: `0x50e8d0`, `0x50ca90`, `0x5111f0`, `0x5112d0`, `0x50f160`, `0x50f710`, `0x511920`, `0x511ad0`, `0x50e570`, `0x5107f0`, `0x50de00`, `0x50f8f0`, `0x50f880`, `0x512c70`, `0x512a50`
Confidence: verified
Proof: one
Test: none

`0x50e8f4` tests mode bit 2 and `0x50e900` calls `0x50ca90`. `0x50e9c1` calls `0x5111f0`; `0x50e9ce` reads the position and `0x50e9d4` to
`0x50e9ff` compare it with `+0x174` to `+0x17c`. `0x50ea22` arms the yaw channel with `0x512c70`. `0x50ea48` calls `0x5112d0` only when
`+0x128` is nonzero, `0x50ea54` `0x50f160` and `0x50ea68` `0x50f710`. `0x50eb0a` calls the bob, `0x50eb45` the band, `0x50ed59` the solver, `0x50ed77` the
pivot bias test (`0x5107f0`, with `0x512a50` clearing the bias), `0x50edd9` the eye placement and `0x50f011` the height arm; `0x50f026` and `0x50f03b` store the
position and the taxi flag. The unit on a taxi is read from its unit flags bit 20 (`0x50e93e`).

## The keyboard axes

Six axes move the camera: 0 zoom in, 1 zoom out, 2 turn right, 3 turn left, 4 look up and 5 look down. The Lua
commands `MoveViewInStart` and `MoveViewInStop`, then Out, Right, Left, Up and Down, start and stop them. An active axis changes its target by a CVar speed times the elapsed milliseconds. Distance
moves `cameraDistanceMoveSpeed` yards a second and stays between 0 and the maximum distance. Yaw moves
`cameraYawMoveSpeed` degrees a second and wraps into 0 to 2π. Pitch moves `cameraPitchMoveSpeed` times 0.25 degrees
a second, so 90 is 22.5 degrees a second, and is clamped to plus or minus 89 degrees.

Addresses: `0x5112d0`, `0x50fd40`, `0x50fd90`, `0x50b4a0`, `0x50b4b0`, `0x50b4d0`, `0x50b4e0`, `0x50b500`, `0xbe0fbc`, `0xbe1094`, `0xbe0f78`
Confidence: verified
Proof: one
Test: none

`0x5112d0` loops over the axes (`0x511327`), testing bit 2i of `+0x128` (active) and bit 2i+1 (stopping), with start times at
`+0x12c`, end times at `+0x144` and queued ends at `+0x15c`. The branches by axis are in a table: axis 0 at `0x5113d3` (target
`+0x198` falls, `0x511402` clamps at 0), 1 at `0x51142d` (rises, clamped to the maximum), 2 at `0x511485` (`+0x210` rises, wraps at
`0x5114c0`), 3 at `0x5114ce`, 4 at `0x51153a` (`+0x1e0` rises, clamped at `0x8089d8`) and 5 at `0x51157d`. The live distance, yaw and pitch
(`+0xec`, `+0xf0`, `+0xf4`) are copied at `0x511422`, `0x51152f`, `0x5115d6` unless the glide bits 0x4000000, 0x1000000, 0x2000000 are set.
`0x50fd40(axis, start, extra)` sets the active bit and the start time, and the queued end when `extra` is nonzero;
`0x50fd90(axis, end)` sets the stopping bit and the end time. `0x50b4a0` (axis 0) and `0x50b500` (axis 1) set the axis number and reach `0x50b4b0`, `0x50b4d0` with `0x50b4e0`, which call the two with the time `0xcf0bc8`.

## Maximum distance and the wheel

The farthest the camera may be is `cameraDistanceMax` times `cameraDistanceMaxFactor`, kept from 0 to 50. First
person is distance 0. `CameraZoomIn(amount)` and `CameraZoomOut(amount)` take an optional number, 1 by default,
and move the distance target by that many yards: the axis runs for `amount / cameraDistanceMoveSpeed` seconds. The
opposite axis is stopped first, and a notch that arrives while the axis still runs adds to its queued time.

Addresses: `0x50b400`, `0x50b450`, `0x50fc60`, `0x50fcd0`, `0x4818f0`, `0xbe0e6c`, `0xbe0fe0`
Confidence: verified
Proof: one
Test: none

`0x5112d6` and `0x5112ee` multiply the factor `0xbe0fe0` by the maximum `0xbe0e6c`; `0x5112f2` to `0x511318` clamp to the constants 0.0 and 50.0
(`0x8089a8`, `0x8089a4`). `0x50b42d` stores the default 1.0; `0x50fc6c` divides by the speed and `0x50fc73` multiplies by 1000, `0x50fc79` truncates.
`0x50fc83` stops the opposite axis, `0x50fca9` extends the queue and `0x50fcbe` starts the axis. `0x4818f0` returns the camera of the world frame (`+0x65b8`).

## View angles

Each update makes the view angles from the live offsets and the target's facing. The yaw is the yaw offset
(`+0xf0`) plus the flip bias (`+0x100`), plus the target's facing unless the camera is in the absolute frame
(`+0xa4` above 0). The facing is the unit's own, smoothed by `0x50d810` for a unit that is not the active mover. The
pitch is the live pitch (`+0xf4`) plus the ground pitch (`+0x108`) when `+0xa8` is 0 or less, clamped to plus or minus
89 degrees; the roll is `+0xf8`. `FlipCameraYaw(degrees)` adds degrees to the flip bias, in radians.

Addresses: `0x50f710`, `0x600cd0`, `0x611660`, `0x50d810`, `0x455b30`, `0x50ab50`, `0x50b6a0`
Confidence: verified
Proof: one
Test: none

`0x50f720` to `0x50f736` copy `+0xf0`, `+0xf4`, `+0xf8` to the outputs. `0x50f7a1` and `0x50f7a6` take the facing at `[unit+0xc98]` (type mask bit 3), else slot
0x18 (`0x50f7f8`); `0x50f7b4` compares the target GUID with the active mover `0xc4da98`; `0x50f7d3` calls `0x50d810`. `0x50f7de` adds `+0x100`;
`0x50f7f0` tests `+0xa4` and `0x50f7f2` adds the facing; `0x50f80e` tests `+0xa8` and `0x50f816` adds `+0x108`. `0x50f81d` normalises, `0x50f826` to `0x50f84c` clamp, `0x50f866` builds the basis.
`0x50b6c9` to `0x50b6d2` convert the argument of `FlipCameraYaw` and add it to `+0x100`.

## The pivot and the eye

The camera looks at a pivot above the target's feet: the pivot is the target's position plus the live pivot
height `+0xfc` in z (an eased value, below). The eye is the pivot minus the orbit distance along the view direction
plus the bob offset (`uicamera/view#pitch-units-and-sign`). With the far-sight aura (mode bit 3) the orbit distance is
forced to 0. At the distance of first person, the eye is the pivot.

Addresses: `0x50ea8d`, `0x50ea93`, `0x50edb7`, `0x50edc0`, `0x50edd9`, `0x5f1f10`
Confidence: verified
Proof: one
Test: none

`0x50ea8d` loads the target's z and `0x50ea93` adds `+0xfc`. `0x50edb7` tests mode bit 3 (0x8) and `0x50edc0` replaces the distance by the constant at
`0x8089a8` (0.0). `0x50edd9` calls `0x50de00` with the pivot, the distance and the offset and `0x50edeb` stores the eye at `+0x8`. The target's
position slot (vtable `+0x14`) of a unit is `0x5f1f10`, which copies the movement object's position.

## Pull-in and the glide back

The near and far solver (`0x50e570`) gives a distance and a height that clear walls and ceilings. When the live
distance (`+0xec`) is more than 1/9 yard beyond the solved one, the camera jumps to the solved distance plus 1/9
yard and glides back out over 2 seconds to the target distance. The height (`+0xfc`) does the same against its
solved value. A shorter pull-in is taken at once.

Addresses: `0x50e570`, `0x50ee5d`, `0x50eeb0`, `0x808a04`
Confidence: verified
Proof: one
Test: none

`0x50ee68` subtracts the solved distance and `0x50ee6b` compares with 0x808a00 (1/9). `0x50ee87` sets the glide bit 0x4000000, `0x50ee8c` stores 2.0
(`0x40000000`) at `+0x194` and `0x50ee92` the start time at `+0x190`; `0x50eea4` and `0x50eeaa` store the new distance, plus 9.5e-7, in `+0xec`
and `+0x19c`. `0x50eeb6` to `0x50eef8` do the same for the height with bit 0x20000000 and `+0x1c0`, `+0x1c4`, `+0x1cc`.

## The self fade

The player's own model fades as the camera comes close and is hidden at first person. With `D` the orbit distance
minus the near clip offset (`+0x38`) and `F` a threshold of 1.8315 yards, the alpha is 255 for `D` at or above `F`, 0 for `D` at or
below 1/360, and in between `255 * (1 - cos(π D / F)) / 2` truncated. When the distance is below the pivot height
and the pitch is below -68 degrees, `F` becomes `1.8315 / s^2` with `s = (-π/2 - pitch) / (-π/2 + 68°)`, so the model
stays faded farther out when the camera is below the character.

Addresses: `0x50eefe`, `0x50f8f0`, `0x482c10`, `0x5b7bb0`, `0x8089b0`
Confidence: verified
Proof: one
Test: none

`0x50ef07` to `0x50ef5b` make `F`; `0x50ef64` forms `D`; `0x50ef6e` sends `D` at or above `F` to alpha 255 (`0x50ef73`); `0x50ef89` to
`0x50efc0` compare with 0x808abc and compute the smoothstep `0x5b7bb0(0, 255.0, D/F)` then truncate; `0x50efc2` hides. `0x50f8f0` forwards the
alpha to the world frame's `0x482c10`. The smoothstep is `a + (b - a) * (1 - cos(π t)) * 0.5` with constants 0x80a714, `0x7ff9d8`, `0x7ffa24`.

## Pivot height

The pivot height is chosen from three model values the camera measures when it starts: a swimming unit uses the swim
value, otherwise the near value when the target orbit distance is below 1.8315 yards and the far value from
there up. The chosen value is armed as the target of the height transition, which `0x50f160` eases.

Addresses: `0x50f880`, `0x5126b0`, `0x50cbc0`
Confidence: inferred
Proof: none
Test: none

`0x50f89e` tests the movement word for swimming (0x200000) and loads `+0x124`; otherwise `0x50f8b5` compares the target distance `+0x198` with
`0x8089b0` and loads `+0x11c` below it or `+0x120` from it up (`0x50f8c2`, `0x50f8ca`). The result is armed through `0x5126b0` with a duration of 1
second (`0x3f800000`). How `0x50cbc0` measures the three values from the model was not read here.

## Setting a view

`SetView(n)` for n from 1 to 5, `NextView()` and `PrevView()` switch to a saved view with the blend style in
`cameraViewBlendStyle`: it prints the view's name, makes the view current in `cameraView`, copies the live distance,
pitch and yaw to the targets and clears every transition. `SaveView(n)` stores the target distance, pitch and yaw in
the view's slot and its CVars (pitch and yaw in degrees); `ResetView(n)` restores the slot from the default strings.
`PrevView` stops at the first view and `NextView` at the last.

Addresses: `0x512e90`, `0x50b5b0`, `0x50b600`, `0x50b640`, `0x50b680`, `0x50b690`, `0x50fa30`, `0x50f990`, `0x50fae0`, `0x50faa0`, `0x50fac0`, `0x50fa60`
Confidence: inferred
Proof: none
Test: none

`0x50b5d6` bounds the argument to 1 to 5 and `0x50b5e5` makes it zero-based. `0x512e90` formats `Camera view %s` (0x8500a0) with the view's name and stores the view in
`cameraView` through `0x63df50` (`0x512f17`); `0x512f2e` to `0x512f99` copy the live values to the targets and clear the transitions. Its blend style,
the third word, is 1 for a timed blend and 2 for an instant one; the arms at `0x513175` and `0x513024` were read, not traced to the end.
`0x50f990` writes the distance, pitch and yaw CVars with `%f` (`0x835160`) after converting pitch and yaw to degrees. `0x50fa60` flips between the views
1 to 3, a back-and-forth cycle the Lua commands do not reach.
