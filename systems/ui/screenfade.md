# ui: the screen fade

A full-screen quad whose alpha runs between 0 and 255 over a number of seconds, and a callback that runs when it ends. The cinematic uses it to darken the screen before a sequence starts and between shots (`uicamera/cinematic`), and entering the world uses it to start a pending cinematic. At most one fade exists. It draws as a screen layer, so the interface and the world draw first.

Roots: `0x4c0d10`, `0x4c1280`, `0x4c0f50`

## Fade state

A fade is ten words. The mode is 0 when idle, 2 while the screen darkens (alpha rises from 0 to 255) and 1 while it lightens (alpha falls from 255 to 0). With the duration, the callback and its argument, the start time and the layer and texture handles, they are the globals of this entry. The active byte is 1 from the start until `StopScreenFade`.

Addresses: `0xb71834`, `0xb71835`, `0xb717a0`, `0xb717f4`, `0xb717bc`, `0xb717c0`, `0xb7179c`, `0xb717b4`, `0xb71760`, `0xb71770`
Confidence: verified
Proof: one
Test: none

The names follow the reads of `0x4c0f50`. The finished byte is set by a frame that draws the end value, so the callback runs on the frame after the last interpolated one.

## Starting a fade

`0x4c0d10(callback, argument, seconds)` replaces any fade (`0x4c0eb0`), creates an 8 by 8 texture, a screen layer of the unit square at priority 10.0 with `0x4c0f50` as its draw, registers the no-op event handlers (`0x4c0e00`) and sets the mode to 2. The layer flags are 6 for a duration of 0 and 7 otherwise. The cinematic start calls it with the callback `0x48edd0` and 0.25 seconds, and so does `EnterWorldUI` when a cinematic is running.

Addresses: `0x4c0d10`, `0x4c0e00`, `0x4c0ea0`, `0x58aa30`, `0x442800`, `0x41fca0`, `0x48ed70`, `0x4908c0`, `0x48edd0`
Confidence: verified
Proof: one
Test: none

`0x4c0e00` registers `0x4c0ea0`, which returns 0, for the event categories 1, 8, 9, 10, 11, 14 and 12 at priority 8.0 (`EventRegisterEx`).

## Drawing a fade

`0x4c0f50` is the layer's draw. While the mode is set it takes the seconds since the first drawn frame, scales them by the duration to 0 to 255 and writes the quad's alpha: rising for mode 2, falling for mode 1. A frame at or past the duration draws the end value, 255 for mode 2 and 0 for mode 1. It then draws the quad with the fade texture. When the finished byte is set it stops the fade after mode 1, clears the mode and calls the callback with its argument in `ecx`.

Addresses: `0x4c0f50`, `0x42c010`, `0x589f40`, `0x58a2a0`, `0x58a2e0`, `0x589f50`
Confidence: verified
Proof: one
Test: none

The alpha is `((seconds / duration) * 255 + 512)` as a float whose mantissa bits `>> 14` leave the byte (`0x8029cc` is the bias 512.0); mode 1 subtracts it from 255. The first frame stores the clock and draws the start value (`0x4c10ed`). Its three function-level statics, the unit-quad positions, the texture coordinates and the normal, are built on the first call and registered with `_atexit`.

## Ending a fade

`0x4c1280(callback, argument, seconds)` turns an active fade around: it sets the mode to 1, the new duration and callback, clears the clock and the finished byte, and does nothing without a fade. The cinematic calls it when it plays a shot and when it ends, and `CGGameUI::Shutdown` calls it. `0x4c0eb0` closes the layer handle, destroys the texture, unregisters the event handlers (`0x4c0ee0`) and clears the active and clock bytes.

Addresses: `0x4c1280`, `0x4c0eb0`, `0x4c0ee0`, `0x41aed0`, `0x58ad60`, `0x41fd80`, `0x48edf0`, `0x48f080`, `0x490bd0`
Confidence: verified
Proof: one
Test: none

`0x4c0ee0` unregisters the same seven categories as `0x4c0e00` registers, in a different order, the last with a tail jump.
