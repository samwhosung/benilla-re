# uicamera: the camera's saved view

The two numbers the camera keeps per character, how the starting view is chosen, the units and sign of the
pitch, and when the camera is made and saved.

This is `Camera.cpp`. The camera object and its matrices are `camera`; the per-frame driver is
`uicamera/driver`; the camera CVars are `uicamera/cvars`; the commands that set the view from the keys are
`input`.

Roots: `0x50c0e0` (construct), `0x50c460` (destroy), `0x50c4d0` (save), `0x50c5a0` (load), `0x50de00` (place the eye)

## The settings file

The camera saves `cameraDistance` and `cameraPitch` to `camera-settings.txt` in the character's folder,
`WTF\Account\<account>\<realm>\<character>\`. The file has exactly two lines, the distance first, each ended
by a line feed alone: `cameraDistance %f` and `cameraPitch %f`, six decimals. The distance is the camera's
target distance, so it is the zoom the player chose and never the distance a wall pulled the camera in to. The
pitch is written in degrees. The camera reads the file when it is made and writes it when it is destroyed.

Addresses: `0x50c4d0`, `0x50c5a0`, `0x5af6b0`, `0x42a5d0`, `0x64a7f0`, `0x84f788`
Confidence: verified
Proof: one
Test: none

`0x50c4e3` opens the file for writing (create, replace) and `0x50c5b4` for reading. `0x50c4ff` loads `[camera+0x198]`, the
target distance, and `0x50c50b` pushes the format; `0x50c549` loads the pitch `[camera+0x1e0]`, `0x50c54f`
multiplies it by 57.295780181884766 and `0x50c561` pushes its format. The name is the string at 0x84fa00, reached
through the pointer `0x84f788`. Both functions end by closing the file (`0x50c594`, `0x50c720`).

## Reading the file

The reader reads the whole file and splits it into lines on carriage return and line feed, and each line into a
key and a value on a space. A key matches `cameraDistance` or `cameraPitch` without case; any other line is skipped.
A distance is stored unchanged into both the current and the target distance (`+0xec`, `+0x198`). A pitch is
multiplied by π/180 and stored into both the current and the target pitch (`+0xf4`, `+0x1e0`). Nothing clamps
either value, so a file can hold a distance outside the zoom range or a pitch past 89 degrees until the next
input.

Addresses: `0x50c5a0`, `0x64ae50`, `0x64a4c0`, `0x64aaa0`, `0x7ffaac`
Confidence: verified
Proof: one
Test: none

`0x50c637` splits a line, `0x50c66a` the key; the keys are the strings at 0x84fff8 and 0x84ffec, compared with `0x64a4c0` and
the length `0x7fffffff` (`0x50c680`, `0x50c6c9`). `0x50c6b0` and `0x50c6b6` are the two distance stores, `0x50c6f9` the multiply
by `0x7ffaac` and `0x50c6ff`, `0x50c705` the two pitch stores. `0x50c70b` to `0x50c715` loop while text remains.

## The starting view

The `cameraView` CVar (default `1`) picks one of five rows of default strings, a distance in yards and a pitch
in degrees: view 0 `FIRST_PERSON` 0.0 and 0.0, view 1 `THIRD_PERSON_A` 5.55 and 10.0, view 2 `THIRD_PERSON_B`
5.55 and 20.0, view 3 `THIRD_PERSON_C` 13.88 and 30.0, view 4 `THIRD_PERSON_D` 13.88 and 10.0. The camera constructor
converts them to numbers, the pitch to radians, stores them in the current distance and pitch and copies both into
the targets. The saved file then replaces them. So a character that never zoomed saves 5.55 and 10.0.

Addresses: `0x50c0e0`, `0x84f488`, `0x84f848`, `0xbe0e68`, `0x50b997`, `0x50bfc0`
Confidence: verified
Proof: one
Test: none

`0x50c17a` reads the view number from the CVar record `0xbe0e68` at `+0x28`; `0x50c18b` and `0x50c1ad` index the table at 0x84f488 (three
string pointers per row; the third is unused and "0.0") for the distance and the pitch; `0x50c1ba` multiplies the pitch by `0x7ffaac`.
`0x50c250` and `0x50c25c` copy to `+0x1e0` and `+0x198`; the saved file is read at `0x50c3ea`, later in the same
constructor. The row names are at 0x84f848. `0x50b997` registers the CVar.

## Pitch units and sign

Inside the camera the pitch is in radians, positive looking down. The view direction is
`(cos yaw cos pitch, sin yaw cos pitch, -sin pitch)` and the eye is the pivot minus the distance times that
direction, so a positive pitch puts the eye above the pivot. A pitch of 10 degrees is the normal over-the-shoulder
view. The pitch is clamped to plus or minus 1.5533430576324463 radians (89 degrees) wherever input changes
it; the file path applies none.

Addresses: `0x50de00`, `0x511f40`, `0x50ab50`, `0x7bf4b0`, `0x7bdfc0`, `0x5f8d20`, `0x8089d8`, `0x8089dc`
Confidence: verified
Proof: one
Test: none

`0x50de00(out, pivot, distance, offset)` calls the basis getter in vtable slot 1 (`0x50de20`), multiplies the
distance by its three floats and subtracts them from the pivot (`0x50de3d`, `0x50de45`, `0x50de4e`), then adds the
offset. Slot 1 of the camera's vtable at 0x808a9c is `0x511f40`, which returns the first row of the basis at
`+0x14` when the roll words `+0x98` and `+0x9c` are 0. `0x7bf4b0(yaw, pitch, roll)` builds `Rz(yaw)`, `Ry(pitch)`
and `Rx(roll)` and multiplies them in that order (`0x7bf5d2`, `0x7bf5de`), then hands the nine elements to
`0x5f8d20` in the order that stores the transpose (`0x7bf5e3` to `0x7bf605`); the first row is column 0 of the
product, `(cy cp, sy cp, -sp)`. The clamp constants are at `0x8089d8` and `0x8089dc`.

## Lifetime

The camera is made with the world frame and destroyed with it. The world frame's constructor makes the camera
(`0x50c0e0`, which loads the file) and stores it at `+0x65b8` of the frame; the frame's destructor calls the camera's
destructor `0x50c460`, which saves the file. The frame is created by name when the interface starts, so the file
is read once per interface load and written when the interface is torn down.

Addresses: `0x50c0e0`, `0x50c460`, `0x481960`, `0x481d30`, `0x481b4e`, `0x481e12`
Confidence: inferred
Proof: none
Test: none

The load is called at `0x50c3ea` and the save at `0x50c48a`. The constructor call is `0x481b4e` inside `0x481960`, the destructor call
`0x481e12` inside `0x481d30`. That the interface teardown reaches the frame's destructor was not followed through the
container that holds the frame.
