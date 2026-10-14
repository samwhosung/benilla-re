# uicamera: following and the auto-return

What the camera does when the character moves: the input state that picks a row of the smoothing CVars, the
delay and factor each row gives, and how the yaw, pitch and pivot bias are armed to return to the view's saved
angles behind the character.

This is `Camera.cpp`. The channels it arms are `uicamera/transitions`; the CVars are `uicamera/cvars`; the
commands and mouse buttons that set the input word are `input`.

Roots: `0x510960` (arm), `0x5111f0` (external move edge), `0x511040` (track flag), `0x510fd0` (frame choice)

## When it runs

The auto-return is armed on edges, never continuously. Five call sites reach `0x510960(input, stopping)`: the press
and release handlers of the movement bindings (`0x514a4d`, `0x514d39`), the track-flag setter `0x511040` (`0x5110d7`) and
the external-move edge `0x5111f0` (`0x511218`, `0x51123c`). The driver calls `0x5111f0` every frame with whether the
character is moved from outside, but it acts only when that flag differs from the Fear bit (mode bit 12) it keeps. Once
armed, the transitions play out by themselves.

Addresses: `0x510960`, `0x5111f0`, `0x511040`, `0x5111d0`, `0x5111b0`, `0x514840`, `0x514b70`, `0x5143e0`
Confidence: verified
Proof: one
Test: none

`0x511205` and `0x51122a` are the two edge tests on mode bit 12 (`test ah,0x10`); `0x5111d0` sets the bit and raises the absolute frame, `0x5111b0` clears them.
The input handlers call `0x511040` with the external-movement bits `MF & 0xf00000` (`0x51497c`, `0x514c5e`), which sets or clears the Track bit 8 of the mode
word. `0x5143e0` returns the input control, whose word at `+4` is the command mask.

## The input state

`0x510960` turns the command mask `MF` and the mode word into a set of states. Fear is mode bit 12. Turn holds when a turn
key (0x300) or the right mouse button (bit 0) is down. Strafe holds for a strafe key (0xc0) or the right button with a turn
key. Move holds for a move key (0x1030) or both mouse buttons (bits 0 and 1). Track is mode bit 8. Stop is the second argument.
Idle holds when no move, strafe or turn key is down and none of the external-movement bits 0xf00000 is set. The
highest state wins, in the order Fear, Turn, Strafe, Move, Track, Stop, Idle. With no state nothing is armed.

Addresses: `0x510960`, `0x510976`, `0x510a6f`
Confidence: verified
Proof: one
Test: none

`0x51097c` tests Fear (value 0x40), `0x51099d` to `0x5109ad` Turn (0x20), `0x5109b3` to `0x5109ce` Strafe (0x10), `0x5109d4` to `0x5109e5` Move (8), `0x5109ec` Track (4), `0x5109f7` Stop (2)
and `0x510a00` to `0x510a23` Idle (1). `0x510a2e` keeps seven bits and returns if none is set. `0x510a6f` to `0x510a84` scan from bit 6 down and take the first set bit. The
command mask bits are those of the movement bindings of `input`.

## Delay and factor

The style comes from `cameraSmoothTrackingStyle` when the states include Track or Fear, else from `cameraSmoothStyle`; a
style of 3 or more arms nothing. The state's two CVars (`uicamera/cvars#smoothing-cvars`) give a delay in seconds and a
factor, each kept from 0 to 99. For each axis the view-data row of the same style adds its delay and multiplies its
factor into them. The pitch and yaw bands are the CVars `cameraPitchSmoothMin`, `Max` and `cameraYawSmoothMin`, `Max`, in radians.

Addresses: `0x510a51`, `0x510a90`, `0x510b00`, `0x510bbf`, `0x510ad1`
Confidence: verified
Proof: one
Test: none

`0x510a51` tests the mask for Track or Fear (0x44) and picks `0xbe10c4` or `0xbe1098`; `0x510a86` bounds the style by 3. `0x510a90` forms the index `style * 7 + state` into
the table at `0xbe0e70` (8 bytes per row). `0x510aa9` to `0x510ae1` clamp to 0 and `100 - 1.0`. `0x510b00` to `0x510bb4` loop over the axes of the view-data table `0xbe0fe8`; `0x510b6a` adds the state's
delay and `0x510b61` multiplies its factor. `0x510bbf` to `0x510c05` form the four band angles.

## Arming the channels

For the yaw, the pitch and the pivot bias the function cancels or arms. A factor of exactly 0 cancels: the target
becomes the live value and the transition is cleared. Otherwise the target is the view's saved angle. In the
absolute frame the yaw target is the saved yaw plus the followed unit's facing; in the relative frame it is the
saved angle unless the live angle lies outside the band, when it is the bound that was crossed. The pitch is the
same with the pitch band and no facing. The pivot bias returns to 0 with the state's own delay and factor. Every
armed channel gets the longest duration, kept from 0.1 to 2.0 seconds.

Addresses: `0x510c4f`, `0x510ce7`, `0x510d9f`, `0x510e11`, `0x510f4d`, `0x512c70`, `0x512830`, `0x512a50`, `0x600cd0`, `0x611660`
Confidence: verified
Proof: one
Test: none

`0x510e1a` tests the factor against 0 and `0x510e2a` cancels the yaw; `0x510e5a` to `0x510e6a` take the view's yaw from `+0xb8` of its slot; `0x510e76` tests the frame; `0x510e78` to `0x510ea4` substitute a band
bound; `0x510ea6` to `0x510f0b` add the facing and normalise; `0x510f28` arms. The pitch is `0x510cf7` to `0x510d75` and the pivot bias `0x510daf` to `0x510dea`. `0x510f4d` to `0x510f88` clamp the
common duration against `0xbe105c` and `0xbe1038` and `0x510f8f` to `0x510fb9` store it in the armed channels.

## Gates

Four tests stand between an edge and an armed channel. The view must be uncustomised, or `cameraCustomViewSmoothing` set, and the camera's mode
bit 5 clear: the view is uncustomised while its saved distance, pitch and yaw match the default strings within 0.001. Yaw smoothing
needs `cameraSmoothYaw` set and the target not in mouselook. Pitch smoothing needs `cameraSmoothPitch` set, which is off by
default, the target not in mouselook and not swimming. Distance smoothing never applies.

Addresses: `0x510940`, `0x50fc30`, `0x50fbc0`, `0x510850`, `0x510760`, `0x510750`
Confidence: verified
Proof: one
Test: none

`0x510940` returns 0 for mode bit 5 (0x20) and otherwise `0x50fc30(view)`, which reads `cameraCustomViewSmoothing` at `0xbe0f1c` and else `0x50fbc0(view)`. `0x50fbc0` parses the
three default strings of the view (`0x84f488`, pitch and yaw times π/180 per the flag table 0x84f474) and compares each slot value with 0.001 (`0x50fbfc`).

## The frame of a followed unit

While the Track bit is set, whether the camera sits in the absolute frame follows `cameraSmoothTrackingStyle`: style 0 keeps it, style 2 does not,
and style 1 asks a global (`0x60f8f0`) that decides for the values 1 and 3. `0x511040` raises or lowers the frame when
that answer changes.

Addresses: `0x510fd0`, `0x60f8f0`, `0x50d3b0`, `0x50d440`
Confidence: inferred
Proof: none
Test: none

`0x510fd6` requires the Track bit, `0x510fde` reads the style, `0x510ff0` the global; `0x51109f` and `0x5110b7` raise and lower the frame with mode bit 13. What the global
holds was not read here.

## Leaving mouselook

When the last primary button is released, `0x514b70` calls `0x50f940(extra)` with its third argument. The camera leaves mouselook (`ui/mouseover#mouselook`), sets mode bit 5 when the argument is nonzero and clears it otherwise,
and copies the live yaw, pitch and pivot bias into their transition targets so nothing returns to the view. While bit 5 is set the auto-return is refused (`uicamera/cvars#clear-and-band-tests`).
`ToggleMouseMove` runs `0x50f910`: it leaves mouselook with the argument 0 when mode bit 0 is set and enters it otherwise.

Addresses: `0x50f940`, `0x50f910`, `0x50f930`, `0x514b70`, `0x50b5a0`, `0x50fdd0`, `0x510940`
Confidence: verified
Proof: one
Test: none

`0x514c48` is the call in `0x514b70`. `0x50f946` calls the leave routine `0x50fdd0`; `0x50f958` and `0x50f95d` set or clear bit 5 (0x20); `0x50f966` to `0x50f984` copy `+0xf4`, `+0xf0` and `+0x104` into `+0x1e0`, `+0x210` and `+0x1f8`. `0x50f910` tests bit 0 and `0x50f91b` calls `0x50f940(0)`, else `0x50f921` goes to the enter thunk `0x50f930`. `0x510940` returns 0 for a camera whose bit 5 is set.
