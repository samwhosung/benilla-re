# sound: where the listener stands

The 3D listener's position and orientation come from the world frame's render callback. Each frame
it hands the sound engine either the unit the player drives or the camera. The call into FMOD and
the axis mapping are in `engine`; the `SoundListenerAtCharacter` variable is registered in `interface`.

Roots: `0x7a5eb0`

## The listener follows the mover

Each frame `0x482ea0` places the listener at the unit the client drives: the active mover, resolved from
the GUID `0xc4da98`, `0xc4da9c` with type mask 8, else the active player (mask 0x10). The position is the
unit's position with its height raised by the product of its `+0x90` and `+0xbc`. The forward vector is
the cosine and sine of the unit's facing with z 0, the up vector is (0, 0, 1) and the velocity is null.
The update is skipped when neither unit resolves. The `SoundListenerAtCharacter` variable at `0xb4b2b4`
being 0 (`0x48311f`), or a shot camera being armed, sends the update to the camera branch instead.

Addresses: `0x482ea0`, `0x48310b`, `0x483118`, `0x483125`, `0x483130`, `0x48314c`, `0x483157`, `0x483168`, `0x48317c`, `0x4831bb`, `0x4831cd`, `0x4831d0`, `0x4831d6`, `0x4831e4`, `0x483216`, `0x483218`, `0x483430`, `0x50c860`, `0x7a5eb0`, `0x481960`, `0x481bba`, `0xb4b2b4`, `0xb4b334`, `0xb4b36c`
Confidence: verified
Proof: one
Test: none

`0x48310b` calls `HasShotCamera` and `0x483125` reads the value at `+0x28` of the variable's record.
`0x483130` to `0x48314c` resolve the mover; `0x483157` to `0x483176` fall back to the player and
`0x48321d` skips. `0x483430` writes the forward vector from the facing (slot 6, `+0x18` of the unit):
`fcos` into `x`, `fsin` into `y`, 0 into `z`. The first use (`0x48317c`) sets bit 0 of `0xb4b36c` and
stores the up vector (0, 0, 1) at `0xb4b334`. `0x4831cd` takes the position through the unit's slot 5, and
`0x4831d0` to `0x4831e4` add `[+0x90]·[+0xbc]` to z. `0x483218` calls `0x7a5eb0` with the velocity null.
The world frame constructor stores the variable's record at `0xb4b2b4` (`0x481bba`). The height is a
different term from the camera's pivot height, which comes from the model (`uicamera/driver#pivot-height`).

## The camera branch

With a shot camera armed, or `SoundListenerAtCharacter` at 0, the listener sits at the camera instead,
with a null velocity.

Addresses: `0x4830b3`, `0x4830bb`, `0x4831f0`, `0x483203`, `0x48320f`, `0x483216`, `0x483218`, `0x7a5eb0`
Confidence: inferred
Proof: none
Test: none

`0x4831f0` loads the camera object `+0x65b8` of the frame and calls its slot 3 (`0x483203`) and slot 1
(`0x48320f`), whose results become the listener's vectors; `xor edx,edx` at `0x483216` passes the null
velocity. Which vectors the two slots return is not read.
