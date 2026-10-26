# gx: facade calls

The device layer's object file opens with thin functions that load the device from `0xc0ed38`
and call one of its slots or a plain method of the base device (`device.md`). This doc holds the
ones whose meaning the rest of the client rests on and that no other gx doc derives: the cursor's
shown flag, the draw rectangle, the texture mip skip and a shader's loaded flag.

Roots: `0x58b590`, `0x58a240`, `0x589bf0`, `0x589c50`, `0x58b360`

## Cursor shown flag

`0x58b590(shown)` calls slot `+0xec` (`ShowCursor`, `slots.md`) with its argument. The base slot
`0x595c20` stores the argument at `+0x2880`, so the last call decides and nothing counts hides;
the constructor stores 1. Direct3D's slot `0x59a7b0` stores it the same way and also passes it to
`IDirect3DDevice9::ShowCursor` when the hardware cursor is in use (`+0x2884`), the device is
ready (`+0xf2c`) and the pointer lies inside the client rectangle. Three callers hide the cursor:
the glue's `HideCursor` (`0x46db70`), `BeginCinematicUI` (`0x48ed70`) and `CameraEnterFreelook`
(`0x50fe30`). Five show it: the glue's `ShowCursor` (`0x46db60`), `EndCinematic` (`0x48f080`),
`LeaveWorldUI` (`0x490a80`), `CameraLeaveFreelook` (`0x50fdd0`) and `CSimpleTop::SetCursor`
(`0x764d50`), each time it is given a cursor texture.

Addresses: `0x58b590`, `0x58b59b`, `0x595c20`, `0x595c26`, `0x591c64`, `0x59a7b0`, `0x59a7bd`, `0x59a7c2`, `0x59a7cc`, `0x59a808`, `0x59a82c`, `0x46db70`, `0x46db60`, `0x48ed70`, `0x48ed88`, `0x48f080`, `0x490a80`, `0x490bbb`, `0x50fdd0`, `0x50fe30`, `0x50fe44`, `0x764d50`, `0x764d78`, `0x764da6`
Confidence: verified
Proof: one
Test: none

`0x58b59b` is `call [edx+0xec]` on `[0xc0ed38]` with `ecx` pushed. `0x595c26` is the store and
`0x591c64` the constructor's 1. `0x59a7bd` calls the base, `0x59a7c2` and `0x59a7cc` test the two
fields, and the bounds `0x59a808` to `0x59a81a` keep a point with left and top inclusive, right
and bottom exclusive, from `GetCursorPos`, `ScreenToClient` and `GetClientRect`; `0x59a82c` is the
call. The census is every `call` and `jmp` to `0x58b590`, eight, and no stored address; `ecx` is
cleared before `0x46db72`, `0x48ed94` (`0x48ed88`) and `0x50fe4c` (`0x50fe44`) and is 1 at the
other five, `0x490bbb` a tail jump. `0x764d78` skips the call for a null texture.

## Draw rectangle

`0x58a240(rect)` writes the device's draw rectangle `+0x1ec` to `rect` through slot `+0x5c`
(`GetDrawRect`), four floats `{top, left, bottom, right}`. Both backends' slots copy it through
`0x5922e0`. The window events store it with the window rectangle (`0x5922f0`, then `0x592340`), so
after a `WM_SIZE` it is `{0, 0, client height, client width}` (`window.md`); while a render target
is bound it is the target's size instead (`CGxDevice::SetRenderTarget`, `0x592230`). Twelve call
sites read it, the font layer's viewport `0x5c2b50` among them.

Addresses: `0x58a240`, `0x58a24b`, `0x5a1a30`, `0x5a0180`, `0x5922e0`, `0x5922f0`, `0x592340`, `0x592230`, `0x592330`, `0x5c2b50`, `0x5c2bbe`
Confidence: verified
Proof: one
Test: none

`0x58a24b` is `call [edx+0x5c]` with `ecx` pushed. `0x5922e0` returns `this + 0x1ec`; `0x5922f0`
copies its four dwords to `+0x1dc` and calls `0x592340`, which copies them to `+0x1ec`; `0x592230`
passes `0x592340` the window rectangle (`0x592330`, `+0x1dc`) with no target bound. The call sites are every `call` to `0x58a240`; no `jmp` or stored address reaches it.

## Texture mip skip

`0x589bf0(levels)` calls slot `+0x2c` (`SetTextureSkip`). Both backends call `0x592050`, which
stores the count at `+0x31c`, and then recreate every texture (`slots.md`, Life cycle slots).
`0x589c50` returns `+0x31c` through `0x5920c0`. The constructor stores 0, and nothing else in the
device files writes it. The one caller of `0x589bf0` is the `baseMip` callback `0x689090`, which
compares the new value with `0x589c50`'s: an equal level returns 1 at once, a level outside 0 to
1 prints `BaseMip must be 0 or 1` and returns 0, and 0 or 1 is applied and printed as `BaseMip
level changed to %d`. The uploads read it at `0x59f6d1` (OpenGL) and `0x5a08fc` (Direct3D).

Addresses: `0x589bf0`, `0x589bfb`, `0x589c50`, `0x5920c0`, `0x592050`, `0x592056`, `0x58d060`, `0x599da0`, `0x591b1d`, `0x689090`, `0x6890a5`, `0x6890ac`, `0x6890b9`, `0x6890c2`, `0x59f6d1`, `0x5a08fc`, `0x44b610`
Confidence: verified
Proof: one
Test: none

The writes of `+0x31c` in `0x5894c0` through `0x5a35d0` are two, `0x591b1d` (`ebx`, cleared at
`0x5916e7`) and `0x592056`; the same scan finds the two writes of `+0x2880`. `0x592050` has two
callers, `0x58d06a` and `0x599daa`. A scan for `call [reg+0x2c]` within eight instructions of a
load of `0xc0ed38` finds only `0x589bfb`; the same scan for `+0xec` finds `0x58b59b`. `0x689090`
reads the value with `SStrToInt` and compares at `0x6890ac`; `0x6890b9` and `0x6890be` bound it.
`0x589c50` has a second caller, `0x44b61c`, in the unreached `0x44b610`.

## Shader loaded flag

`0x58b360(shader)` returns 0 for a null shader, else its `+0x2c`: the loaded flag. The shader file
read sets it to 1 for each shader it reads (`0x595216`, `slots.md`, Shader files); an OpenGL
program build clears it and sets it to whether the program compiled within the native limits
(`0x59e544`, `0x59e6be`, `renderstate.md`). On Direct3D it is 1 once the file has a program for
the device's level. Twenty call sites test it; the weather turns its shader leg off when either
of its two shaders reports 0 (`0x6747f5`, `0x674804`, `0x67480d`).

Addresses: `0x58b360`, `0x58b362`, `0x58b367`, `0x595216`, `0x59e544`, `0x59e6be`, `0x6747f5`, `0x674804`, `0x67480d`
Confidence: verified
Proof: one
Test: none

`0x58b362` returns 0 for a null `ecx`, `0x58b367` loads `[ecx+0x2c]`. The census is every `call`
to `0x58b360`: twenty, from `0x6747f5` to `0x6b6891`, and no `jmp` or stored address.
