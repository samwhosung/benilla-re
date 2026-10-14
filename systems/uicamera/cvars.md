# uicamera: the camera CVars

The settings the player and the interface change: the registered camera CVars, the values each accepts, the
families of generated CVars, and the tests that turn some of them into conditions on the camera.

This is `Camera.cpp`, the world camera of the interface layer. The saved view and the camera's units are
`uicamera/view`; the camera object is `camera`.

Roots: `0x50b6f0` (register), `0x50b330` (range check), `0x50c0e0` (construct), `0x50ddc0` (cameraClip)

## Input and bobbing CVars

The camera registers 46 CVars by name when the interface starts, each in category 5 with a default string. A
value is read as a float at `+0x24` or an integer at `+0x28` of the CVar record. A set value goes through the
CVar's own check, if it has one, and the check refuses a value outside its range with `Value out of range (%f -
%f)` and keeps the old value. A range is inclusive at both ends. The tables list the default and the range
each CVar accepts. 

| CVar | default | accepts | variable |
|---|---|---|---|
| `mouseInvertYaw` | 0 | any | `0xbe1084` |
| `mouseInvertPitch` | 0 | any | `0xbe1040` |
| `cameraBobbing` | 0 | any | `0xbe10c0` |
| `cameraBobbingLRAmplitude` | 2.0 | any | `0xbe1064` |
| `cameraBobbingUDAmplitude` | 2.0 | any | `0xbe10c8` |
| `cameraBobbingFrequency` | 0.8 | 0.001 to 300 | `0xbe0cf8` |
| `cameraDistanceMoveSpeed` | 8.33 | 0.0027778 to 50 | `0xbe0fbc` |
| `cameraPitchMoveSpeed` | 90.0 | 0.1 to 360 | `0xbe0f78` |
| `cameraYawMoveSpeed` | 180.0 | 0.1 to 360 | `0xbe1094` |
| `cameraBobbingSmoothSpeed` | 0.8 | 0.0027778 to 50 | `0xbe10bc` |
| `cameraViewBlendStyle` | 1 | any | `0xbe10b0` |
| `cameraView` | 1 | 0 to 4 | `0xbe0e68` |

Addresses: `0x50b6f0`, `0x50b330`, `0x50b310`, `0x50b380`, `0x50b3c0`, `0x50bfc0`, `0x50bfe0`, `0x50c000`, `0x50c040`, `0x50c060`, `0xbe1084`, `0xbe1040`, `0xbe10c0`, `0xbe1064`, `0xbe10c8`, `0xbe0cf8`, `0xbe0fbc`, `0xbe0f78`, `0xbe1094`, `0xbe10bc`, `0xbe10b0`, `0xbe0e68`
Confidence: verified
Proof: one
Test: none

`0x50b6f0` calls `CVar::Register` (`0x63db90`) with the name in `ecx` and the default string, the check and the category 5
pushed; the record comes back in `eax` and is stored by the first `mov` to memory after the call (`0x50b748`
for the first, and so on). The checks are range tests through `0x50b330(string, low, high)`: `0x50b310` takes 0
to 50, `0x50b380` the pitch limit in degrees (the radian constants `0x8089d8`, `0x8089dc` times 57.29578),
`0x50b3c0` 0 to 360, `0x50bfc0` 0 to 4, `0x50c060` 0 to 3, `0x50bfe0` 0.001 to 300, `0x50c000` 0.1 to 360 degrees
and `0x50c040` 1/360 to 50. `0x50b330` accepts a value equal to either end (`0x50b343`, `0x50b348`).

## Smoothing speed CVars

The default and the accepted range of each CVar:

| CVar | default | accepts | variable |
|---|---|---|---|
| `cameraFoVSmoothSpeed` | 0.5 | 0.1 to 360 | `0xbe1068` |
| `cameraDistanceSmoothSpeed` | 8.33 | 0.0027778 to 50 | `0xbe106c` |
| `cameraGroundSmoothSpeed` | 7.5 | 0.1 to 360 | `0xbe0fc0` |
| `cameraHeightSmoothSpeed` | 1.2 | 0.0027778 to 50 | `0xbe0cd4` |
| `cameraPitchSmoothSpeed` | 45.0 | 0.1 to 360 | `0xbe0ce4` |
| `cameraTargetSmoothSpeed` | 90.0 | 0.1 to 360 | `0xbe0fc8` |
| `cameraYawSmoothSpeed` | 180.0 | 0.1 to 360 | `0xbe1070` |
| `camerasmooth` | 1 | any | `0xbe1074` |
| `cameraSmoothPitch` | 0 | any | `0xbe0fc4` |
| `cameraSmoothYaw` | 1 | any | `0xbe104c` |
| `cameraSmoothStyle` | 1 | 0 to 3 | `0xbe1098` |
| `cameraSmoothTrackingStyle` | 1 | 0 to 3 | `0xbe10c4` |

Addresses: `0xbe1068`, `0xbe106c`, `0xbe0fc0`, `0xbe0cd4`, `0xbe0ce4`, `0xbe0fc8`, `0xbe1070`, `0xbe1074`, `0xbe0fc4`, `0xbe104c`, `0xbe1098`, `0xbe10c4`
Confidence: verified
Proof: one
Test: none

The registration order is the order of `0x50b6f0`; each handle is stored after its own call.

## Terrain, water and pivot CVars

The default and the accepted range of each CVar:

| CVar | default | accepts | variable |
|---|---|---|---|
| `cameraCustomViewSmoothing` | 0 | any | `0xbe0f1c` |
| `cameraTerrainTilt` | 0 | any | `0xbe0fd4` |
| `cameraTerrainTiltTimeMin` | 3.0 | 0.001 to 300 | `0xbe1050` |
| `cameraTerrainTiltTimeMax` | 10.0 | 0.001 to 300 | `0xbe1054` |
| `cameraWaterCollision` | 1 | any | `0xbe1088` |
| `cameraHeightIgnoreStandState` | 0 | any | `0xbe0cf4` |
| `cameraPivot` | 1 | any | `0xbe10a4` |
| `cameraPivotDXMax` | 0.05 | any | `0xbe0f30` |
| `cameraPivotDYMin` | 0.00 | any | `0xbe0cec` |
| `cameraDive` | 1 | any | `0xbe0cd0` |
| `cameraSurfacePitch` | 0.0 | -89 to 89 | `0xbe0f4c` |
| `cameraSubmergePitch` | 18.0 | -89 to 89 | `0xbe0fd8` |

Addresses: `0xbe0f1c`, `0xbe0fd4`, `0xbe1050`, `0xbe1054`, `0xbe1088`, `0xbe0cf4`, `0xbe10a4`, `0xbe0f30`, `0xbe0cec`, `0xbe0cd0`, `0xbe0f4c`, `0xbe0fd8`
Confidence: verified
Proof: one
Test: none

The registration order is the order of `0x50b6f0`; each handle is stored after its own call.

## Final pitch and band CVars

The default and the accepted range of each CVar:

| CVar | default | accepts | variable |
|---|---|---|---|
| `cameraSurfaceFinalPitch` | 5.0 | -89 to 89 | `0xbe0f2c` |
| `cameraSubmergeFinalPitch` | 5.0 | -89 to 89 | `0xbe0fdc` |
| `cameraDistanceMax` | 15.0 | 0 to 50 | `0xbe0e6c` |
| `cameraDistanceMaxFactor` | 1.0 | any | `0xbe0fe0` |
| `cameraPitchSmoothMin` | 0.0 | -89 to 89 | `0xbe0f44` |
| `cameraPitchSmoothMax` | 30.0 | -89 to 89 | `0xbe0f48` |
| `cameraYawSmoothMin` | 0.0 | 0 to 360 | `0xbe0fb8` |
| `cameraYawSmoothMax` | 0.0 | 0 to 360 | `0xbe0fcc` |
| `cameraSmoothTimeMin` | 0.1 | 0.001 to 300 | `0xbe105c` |
| `cameraSmoothTimeMax` | 2.0 | 0.001 to 300 | `0xbe1038` |

Addresses: `0xbe0f2c`, `0xbe0fdc`, `0xbe0e6c`, `0xbe0fe0`, `0xbe0f44`, `0xbe0f48`, `0xbe0fb8`, `0xbe0fcc`, `0xbe105c`, `0xbe1038`
Confidence: verified
Proof: one
Test: none

The registration order is the order of `0x50b6f0`; each handle is stored after its own call.

## Saved view CVars

Fifteen more CVars hold the five views. The name is `camera`, then `Distance`, `Pitch` or `Yaw`, then the
view's suffix: none for view 0 (first person), `A` to `D` for views 1 to 4. So `cameraDistance` is view 0 and
`cameraPitchB` the pitch of view 2. The default of each is the string of the view table in
`uicamera/view#the-starting-view`, and each distance accepts 0 to 50, each pitch plus or minus 89 degrees and each
yaw 0 to 360. The camera keeps the current value of each in its view slots.

Addresses: `0xbe0f7c`, `0x50b9b0`
Confidence: verified
Proof: one
Test: none

The loop at `0x50b9b0` runs five times over the suffix table at 0x84f454 (the first is empty) and three times over
the axis table at 0x84f468, builds the name with `0x64a760` from `camera` (0x84fdc0), the axis and the suffix, and
registers it with the default `[esi+edi+0x84f488]` and the check `[esi+0x84f790]`; the handle goes to
`[esi+edi+0xbe0f7c]`, 12 bytes per view and 4 per axis. The checks are `0x50b310`, `0x50b380` and `0x50b3c0` in that order.

## Smoothing CVars

Three families are built from a style (`Never`, `Smart`, `Always`) and a state. They all have the style first, so
the `cameraSmoothStyle` value 0, 1 or 2 selects a block of each. A: `cameraSmooth<style><state><Delay or
Factor>` with the seven states Idle, Stop, Track, Move, Strafe, Turn and Fear. B: `cameraSmoothViewData<style>
<axis><Delay or Factor>` with the axes Distance, Pitch and Yaw. C: `cameraTerrainTilt<style><state><Absorb,
Delay or Factor>` with the ten states Fall, Fear, Idle, Jump, Move, Strafe, Swim, Taxi, Track and Turn. None has a
check. The prefixes `cameraSmooth` and `cameraSmoothViewData` are not CVars themselves.

Addresses: `0xbe0e70`, `0xbe0fe8`, `0xbe0d00`, `0x50baf0`
Confidence: verified
Proof: one
Test: none

The loops start at `0x50baf0`; the style table is at 0x84f4cc, the state tables at 0x84f4d8 (A), 0x84f468 (B) and 0x84f5e8 (C),
and the suffix tables at 0x84f4c4 (A, B) and 0x84f610 (C). The handles are stored 8 bytes per state in A (7 states, 56
per style), 24 per style in B and 120 per style in C. The default strings are the tables at 0x84f4f8 (42 entries),
0x84f5a0 (18) and 0x84f620 (90).

## Smoothing defaults

Family A, delay and factor in seconds and a ratio: `Never` is 0.0 and 0.0 for every state. `Smart` is 0.0 and 0.0
for Idle and Stop, 0.4 and 10.0 for Track and Fear, and 0.0 and 1.0 for Move, Strafe and Turn. `Always` is 0.0
and 1.0 for all seven. Family B: `Never` is all 0.0; `Smart` has distance 0.0 and 0.0, pitch 0.0 and 1.0, yaw 0.0
and 1.0; `Always` has distance 0.0 and 0.0, pitch 0.0 and 0.0, yaw 0.0 and 1.0 (delay first, factor second).

Addresses: `0x50baf0`, `0xbe0e70`
Confidence: verified
Proof: one
Test: none

The tables are read straight from the image: 42 strings for A (style-major, then state, then delay before factor),
18 for B (style, axis, suffix) and 90 for C (style, state, absorb, delay, factor).

## Terrain tilt defaults

Family C, absorb, delay and factor: `Never` is 0.0, 0.0 and -1.0 everywhere. `Smart` is 1.0, 0.0 and 0.75 for Fall;
1.0, 0.0 and 1.0 for Fear, Move, Strafe, Track and Turn; 0.0, 0.0 and -1.0 for Idle and Jump; 0.0, 0.0 and 1.0 for
Swim and Taxi. `Always` is the same as `Smart` except Idle, which is 1.0, 0.0 and 1.0.

Addresses: `0xbe0d00`
Confidence: verified
Proof: one
Test: none

The 90 strings of the table at 0x84f620 are three per state, in the order of the suffix table, ten states per style.

## cameraClip

`cameraClip` is a console command, not a CVar. With a number it stores that number in a word, with no argument it
flips it. Nothing reads the word, so the command changes nothing.

Addresses: `0x50ddc0`, `0x63f9e0`, `0x50c3fb`
Confidence: verified
Proof: one
Test: none

`0x50c3fb` registers `cameraClip` with the handler `0x50ddc0` (`0x63f9e0` is the command registrar) in the camera's constructor,
and `0x50c483` removes it in the destructor. The only references to the word at 0x84f78c, which starts at 1, are the three stores
in `0x50ddc0` (`0x50ddcf`, `0x50dde7`).

## Bobbing and pivot tests

`0x5105a0(unit)` holds for a unit or player when `cameraTerrainTilt` is nonzero. `0x5105e0(unit)` is the bobbing
condition: the camera distance is at most 1/6 yard, the unit is a unit or player, `cameraBobbing` is nonzero, the
unit is not swimming, falling, mounted or on a taxi, the camera's far-sight bit (`+0x90` bit 3) is clear, and it
returns the camera's bit 9. `0x510690(unit)` is the pivot condition: `cameraPivot` is nonzero, the unit has no
forward, backward or strafe bit set, the camera pitch is 0 or less, and it returns the camera's bits 16 and 17.

Addresses: `0x5105a0`, `0x5105e0`, `0x510690`
Confidence: verified
Proof: one
Test: none

Each test checks the object's type mask for bit 3 (`shr ecx,3; test cl,1`), so a player passes as well as a unit. `0x5105b8` and
`0x51061a` read `cameraTerrainTilt` and `cameraBobbing` at `+0x28`; `0x5105ea` compares the distance at `+0xec` with the constant 0x8089ac
(1/6); `0x510630` and `0x510638` test the movement word `[unit+0x118]+0x40` for swimming (0x200000) and falling (0x2000); `0x510655`
tests the mount field and the taxi flag (unit flags bit 20). `0x5106bd` tests the movement bits 0xf and `0x5106c3` the pitch.

## Clear and band tests

`0x5106f0` and `0x5107f0` say the bob offsets (`+0x110` to `+0x118`) or the pivot bias (`+0x104`) are at least 0.001 in
size while the matching condition is false, so the effect has to be cleared. `0x510760(low, high)` and
`0x510850(low, high)` say the pitch or the yaw is outside the band and the matching smoothing CVar
(`cameraSmoothPitch`, `cameraSmoothYaw`) is nonzero; the yaw test also needs the target to exist and the camera
not in mouselook. The distance test `0x510750` always answers 0. `0x510940` answers 0 when the camera's bit 5 is set
and otherwise the view-customised test of `uicamera/follow`.

Addresses: `0x5106f0`, `0x5107f0`, `0x510750`, `0x510760`, `0x510850`, `0x510940`
Confidence: verified
Proof: one
Test: none

`0x5106f0` and `0x5107f0` sum the three floats and compare the absolute value with 0.001 (`0x510707`, `0x5107fb`). `0x510760` is
the pitch band (`0x5107b1`, `0x5107cb`) and `0x510850` the yaw band; `0x510750` is `xor eax,eax; ret`.
