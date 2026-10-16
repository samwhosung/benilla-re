# gx: the device

`CGxDevice` and its two backends, `CGxDeviceD3d` (Direct3D 9) and `CGxDeviceOpenGl`. The backend is chosen at start-up; the client then talks to one global device through the `Gx*` facade (`0x589ca0` to `0x58b740`), a run of thin functions that load the device from `0xc0ed38` and call a base-class method or a virtual slot. The device object has 62 virtual slots (`+0` to `+0xf4`).

Roots: `0x589ac0`, `0x589a00`, `0x589ca0`, `0x58b740`, `0x58bc90`, `0x5899f0`, `0x5899c0`, `0x58b5e0`

## Backend selection and creation

`0x63a230` (the console's graphics start-up) sets the device type to 1 (Direct3D), then to 0 (OpenGl) when the CVar `gxApi` equals `OpenGl` ignoring case, then to 0 when command-line flag 5 (`-opengl`) is present, then to 1 when flag 0 (`-d3d`) is present. `gxApi` is registered at `0x63a833` with the default value `direct3d` (`0x864f7c`). It calls `GxDevCreate` (`0x589ac0`) with the type, the window procedure `0x42cfe0` and the format record. `GxDevCreate` allocates and constructs a `CGxDeviceOpenGl` (type 0, `0x58dd70`) or a `CGxDeviceD3d` (type 1, `0x58ba70`), stores it at `0xc0ed38`, and calls its slot `+0x20` with the format and window procedure; on failure it calls slot `+0x18` with 1 and returns 0.

Addresses: `0x63a230`, `0x63a3bb`, `0x63a3c9`, `0x63a3d5`, `0x63a3e3`, `0x63a82a`, `0x589ac0`, `0x58dd70`, `0x58ba70`, `0xc0ed38`, `0x419c10`
Confidence: verified
Proof: one
Test: none

`0x419c10(index)` reads a command-line switch; the switch table at `0x801088` (records of flags, id, name, 0) gives id 0 to `d3d` and id 5 to `opengl` (`storm/scmd`, The client's switch readers). The compare is `SStrCmpI` against `OpenGl` (`0x842a5c`); its result 0 means equal. A later assignment wins, so `-d3d` beats `-opengl`. The Direct3D object is `0x3c04` bytes (`0x58ba70`) and the OpenGL object `0x3ab4` (`0x58dd70`).

## Device object

The base constructor `0x5916e0` zeroes the object, stores vtable `0x809da0` (whose unimplemented slots point at the pure-call stub `0x40a2d7`) and tag 2 at `+0x1fc`; `CGxDeviceD3d::CGxDeviceD3d` (`0x598ce0`) then stores vtable `0x809ef8`, tag 1 and `0` at `+0x258`, and `CGxDeviceOpenGl` (`0x58dda0`) vtable `0x809af8`, tag 0 and `1` at `+0x258`. The accessor `0x589ca0` returns the tag. Fields: `+0x204` a 14-dword copy of the format record, `+0x23c` the capability block, `+0x2824` the render state entries, `+0x328` and `+0x928` the two gamma ramps, `+0x24fc` four light records, `+0x2864` the capture flag. Direct3D adds `+0x3894` the window, `+0x38a4` the `IDirect3D9`, `+0x38a8` the `IDirect3DDevice9` and `+0x38ac` its caps.

Addresses: `0x5916e0`, `0x598ce0`, `0x58dda0`, `0x589ca0`, `0x592120`, `0x809da0`, `0x809ef8`, `0x809af8`, `0x40a2d7`
Confidence: verified
Proof: one
Test: none

`+0x258` is the colour component order flag the emitters read (`draw.md`): 1 for OpenGl, 0 for Direct3D. The stores are `0x58df39` and `0x598d31`; `0x5916e0` clears the block first. Slot 0 is the deleting destructor in each table (`0x594c80` base, `0x59f5b0`, `0x5a0d70`).

## Base format setter

`CGxDevice::DeviceSetFormat(format)`, slot `+0x28`, copies the 14-dword format to `+0x204`. When
both its width and its height are below the minimum, 320 × 240, it raises them to it; one below
alone is kept. With the format's constrain-aspect byte set it stores width / height as the aspect
ratio `0xc0ef60`, and 0 otherwise, and it returns 1. Each backend's setter calls it once its
window and its context or device are made. The two window procedures' sizing helpers read the ratio and the minimum.

Addresses: `0x591fd0`, `0x591fe5`, `0x591fed`, `0x591ffb`, `0x592003`, `0x592015`, `0x592021`, `0x59203c`, `0x590fd0`, `0xc0ef60`, `0xc0ef64`, `0x58ced6`, `0x599cff`, `0x5910a0`, `0x5911a0`
Confidence: verified
Proof: one
Test: `gx::aspect_ratio_matches_binary`

`0x591fd0` is at `+0x28` of the base vtable `0x809da0`, the slot where `0x809af8` and `0x809ef8` hold
`0x58ce40` and `0x599c90`, which call it at `0x58ced6` and `0x599cff`. `0x591fe5` is the `rep movsd` of
0xe dwords; `0x591fed` (`jge`) skips when the width `+0x214` is not below `0xc0ef64`, and `0x591ffb` when
the height `+0x218` is not below the next dword; `0x592003` stores both. `0x592015` tests byte `+0x20c`,
the format's `+8`; `0x592021` is `fild`/`fidiv` of width by height, and `0x59203c` stores 0. The static
initializer `0x590fd0` sets the minimum to 0x140 and 0xf0. `0x5910a0` to `0x591160` read the ratio
and `0x5911a0` to `0x591200` the minimum; their callers are `0x58c6d0` and `0x598750`. The test runs `0x592021` to `0x592038`.

## Direct3D device start

Slot `+0x20` of `CGxDeviceD3d` (`0x599a60`, arguments the window procedure and the format) saves the screen's gamma ramp (`GetDeviceGammaRamp` into `+0x928`, copied to `+0x328`), registers the window class `GxWindowClassD3d` (`0x599b30`, icon `BlizzardIcon.ico`, cursor `BlizzardCursor.cur`, procedure `0x598750`) and runs `0x5992a0`, which loads `d3d9.dll` (`0x598e40`, `Direct3DCreate9(32)`), reads `GetDeviceCaps` into `+0x38ac` and `GetAdapterDisplayMode` into `+0x3a24`. It stores the window procedure at `+0xf28` and calls slot `+0x28` (`0x599c90`) with the format: that copies the format to `+0x204`, creates the window (`0x598b50`), creates the device (`0x5995c0`) and shows the window.

Addresses: `0x599a60`, `0x599b30`, `0x5992a0`, `0x598e40`, `0x599c90`, `0x598b50`, `0x5995c0`, `0x591f80`, `0x598750`
Confidence: verified
Proof: one
Test: none

`0x5995c0` builds the present parameters, calls `IDirect3D9::CreateDevice(0, HAL, window, flags, params)` with flags `0x52` when the format asks for hardware vertex processing and the device reports hardware transform and light (`DevCaps & 0x10000`), else `0x22`, fills the capability block (`0x598ee0`), runs the state initialiser `0x5a24a0` and makes an 8 by 8 default texture of colour `0xff00ff00` (`0xc0f22c`) at `+0x3a68`. The flags are `0x40 | 0x10 | 0x2` (hardware vertex processing, pure device, FPU preserve) and `0x20 | 0x2`. `0x5a24a0` sets `ZENABLE` 1, `LOCALVIEWER` 1 and `ALPHAFUNC` greater-equal.

## Direct3D window

`0x598b50(format)` creates the window with `CreateWindowExA(WS_EX_APPWINDOW, "GxWindowClassD3d", ...)`. Full screen (format byte `+7` clear) uses style `0x86080000`. Windowed uses `0x06cf0000`, or `0x90000000` (popup, visible) when byte `+9` is set. A windowed overlapped window with a size has the size grown by `AdjustWindowRectEx` to the outer size; a popup takes the screen size from `GetSystemMetrics`, at position (0, 0), and an overlapped one the top-left of the primary monitor's work area. A zero size becomes `CW_USEDEFAULT`. The window is shown at once (`ShowWindow`, 1).

Addresses: `0x598b50`, `0x598cc0`, `0x809eac`
Confidence: verified
Proof: one
Test: none

The position is stored in format `+0x30` and `+0x34`, the size in `+0x10` and `+0x14`. `0x598cc0` destroys the window and clears `+0x3894`.

## Present parameters

`0x599790(out, format)` builds the `D3DPRESENT_PARAMETERS`. Windowed (format byte `+7` set): width and height are the window's float size converted with `__ftol`, the back buffer format is the desktop's, refresh 0. Full screen: width `format+0x10`, height `format+0x14`, back buffer format `0x809ec0[format+0x24]`, refresh `format+0x28`. Always: swap effect discard (1), the window at `+0x3894`, auto depth stencil on with the depth format `0x809ec0[format+0xc]`, back buffer count `format+0x18` when byte `+0x2c` is set and 1 otherwise, and presentation interval 1 with that byte set, immediate (`0x80000000`) otherwise.

Addresses: `0x599790`, `0x809ec0`, `0x5922e0`
Confidence: verified
Proof: one
Test: none

A sample count above 1 at `format+0x1c` becomes `MultiSampleType` (flags 0); the quality is `(levels - 1) * format+0x20` through `__ftol`, where `levels` is what `CheckDeviceMultiSampleType` reports for the back buffer format (`0x5998bb`). With no multisampling `Flags` is 1 (lockable back buffer).

## Capability block

`0x598ee0` fills the block at `[dev+0x23c]` from the `D3DCAPS9` copy at `+0x38ac`: the first dword is the smaller of `MaxSimultaneousTextures` and 8, and `+0x60`, `+0x64`, `+0x68` are the larger of `MaxTextureWidth` and 256. From `+0x2c` it holds one flag per `GxTex` format 0 to 8, set when `CheckDeviceFormat` accepts that format as a texture for the current display format (`0x599350`); the formats are the codes at `0x80a100`: none, `A8R8G8B8`, `A4R4G4B4`, `A1R5G5B5`, `R5G6B5`, `DXT1`, `DXT3`, `DXT5`, `V8U8`. The texture layer reads `+0x40`, `+0x44` and `+0x48` as the DXT support flags and `+0x60` as the size limit.

Addresses: `0x598ee0`, `0x599350`, `0x80a100`, `0x58a230`, `0x5923d0`
Confidence: verified
Proof: one
Test: none

`0x58a230` returns the block (`0x5923d0`: `this + 0x23c`). The remaining fields come from `DevCaps`, `TextureOpCaps`, the shader versions and similar bits at `0x598f5d` to `0x599290` and are not decoded here.

## Adapter class

`0x58bc90(out)` loads Direct3D, calls `IDirect3D9::GetDeviceCaps` for adapter 0 and stores a class in the word at `out`: 2 when the device has hardware transform and light (`DevCaps & 0x10000`), more than 2 simultaneous textures and a pixel shader version of 1.1 or more; 1 when it has hardware transform and light and 2 or more textures; otherwise 0. It returns 1 when the caps call succeeded, and logs `CGxDevice::DeviceAdapterInfer(): RET: %d, DID: %x`.

Addresses: `0x58bc90`, `0x5899b0`, `0x598e40`
Confidence: verified
Proof: one
Test: none

The compares are on the caps copy at stack offsets that are `D3DCAPS9` `DevCaps` (`+0x1c`), `MaxSimultaneousTextures` (`+0x98`) and `PixelShaderVersion` (`+0xcc`, low word `0x101`). `0x5899b0` is the facade.

## Display modes

`0x58b740(list)` fills a list of device format records (0x38 bytes) for Direct3D: for each of three adapter display formats (`0x85a820`: `0x17`, `0x16`, `0x15`) it walks the adapter's modes (`GetAdapterModeCount`, `EnumAdapterModes`) and, for each depth format of `0x85a82c` (`0x50`, `0x4d`, `0x4b`, `0x47`) that `CheckDeviceFormat` accepts as a depth-stencil surface, and each sample count of `0x85a83c` (0, 2, 4, ..., 16) that `CheckDeviceMultiSampleType` accepts for the display and the depth format, appends a record through `0x591220`, which adds it when no equal one is held. `0x58bd40(list)` does the OpenGL case from `EnumDisplaySettings`, keeping modes of 640 by 480 or more at 16 bits or more, and sorts them by area with `0x58be40`.

Addresses: `0x58b740`, `0x58bd40`, `0x58be40`, `0x591220`, `0x85a820`, `0x85a82c`, `0x85a83c`, `0x5899c0`, `0x5899f0`, `0x58be60`
Confidence: verified
Proof: one
Test: none

The record fields set at `0x58b7b1` to `0x58b903` include a display format index (2 for `0x15`, 1 for `0x16`, 0 for `0x17`), a depth index and a sample index. `0x58be40(a, b)` returns `a.height * a.width - b.height * b.width`, so the list is ascending in area. The records come from `0x589900`, which stores its 11 arguments into the record.

## Gamma ramp

`0x591680(ramp, g)` fills three 256-entry tables of 16-bit values at `ramp +0`, `+0x200` and `+0x400`: entry `i` is `__ftol(pow(i * (1/255), g) * 65535)` in each. The constructor builds the two device ramps for `g = 1.0` (`+0x328` and `+0x928`). The device slot `+0x34` builds the working ramp `+0x328` from a gamma value, slot `+0x30` copies in a whole ramp, and the Direct3D versions then call `IDirect3DDevice9::SetGammaRamp(0, 0, ramp)` only when `[dev+0x38b8] & 0x20000` is set and the device is not windowed (`[dev+0x20b]` clear).

Addresses: `0x591680`, `0x592060`, `0x592080`, `0x599dc0`, `0x599e10`, `0x8026c8`, `0x809d98`
Confidence: verified
Proof: one
Test: none

`0x8026c8` is the `f32` 1/255 and `0x809d98` the `f32` 65535; `pow` is the C runtime's `__CIpow` (`0x73f940`) and the truncation `__ftol` (`0x40a2b0`). `0x592060` calls `0x591680` on `this + 0x328`; `0x592080` copies `0x180` dwords (0x600 bytes) into it.

## Gamma setting

`ClientRegisterConsoleCommands` registers `Gamma` with the callback `0x4034d0`. It reads the new
value with `SStrToFloat` and, when the `DesktopGamma` record is registered and its integer value is
0, passes the float to `0x589c00`, which calls slot `+0x34` of the device in `s_gxDevice`: the
working ramp is rebuilt from that gamma (Gamma ramp). With `DesktopGamma` set the value is kept and
not applied. It returns 1 either way.

Addresses: `0x4034d0`, `0x402d58`, `0x4034df`, `0x4034e8`, `0x589c00`, `0x8826d4`, `0xc0ed38`
Confidence: verified
Proof: one
Test: none

`0x402d58` pushes the callback. `0x4034df` loads `0x8826d4`, where the registration of
`DesktopGamma` stores its record (`tables/cvars.tsv`), and skips on null; `0x4034e8` reads the
record's `+0x28`, the value as an integer (`cvar/registry`), and skips on non-zero. The float is
stored back over the callback's own argument and pushed. `0x589c00` loads `[0xc0ed38]` into `ecx`
and calls `[vtable+0x34]` with the one argument, `ret 4`. `GammaCallback` is named for the setting.

## Direct3D clear and viewport

`0x5a17d0(mask)`, slot `+0x6c`, clears: bit 0 of the mask is `D3DCLEAR_TARGET`, bit 1 `D3DCLEAR_ZBUFFER`; the colour is `[dev+0x324]` (default `0xff000000`) unless bit 8 of the capability word is clear, when it cycles through three sine waves; depth 1.0, stencil 0; the viewport is applied first when dirty. `0x5a1100` builds the `D3DVIEWPORT9` from the fractions at `[dev+0xf38]` to `+0xf4c`: `X = ftol([+0xf38] * W)`, `Y = ftol((1 - [+0xf44]) * H)`, `Width = ftol([+0xf3c] * W) - X`, `Height = ftol((1 - [+0xf40]) * H) - Y`, `MinZ` and `MaxZ` from `+0xf48` and `+0xf4c`.

Addresses: `0x5a17d0`, `0x5a1100`, `0x5922e0`, `0x592530`
Confidence: verified
Proof: one
Test: none

`0x5922e0` returns the float block whose `+8` is the height and `+0xc` the width. When `[dev+0x2848]` or `[dev+0x2854]` is set `Y = ftol([+0xf40] * H)`. `0x592530` stores the six fractions and marks the viewport dirty.

## Direct3D present

`0x5a1910(mask)`, slot `+0x68`, presents: it ends the scene (`0x5a17a0`), flushes (slot `+0x14`), optionally waits for the GPU, captures if armed (`0x592140`), calls `Present(0, 0, 0, 0)` and clears `[dev+0xf2c]` on failure. It then runs `0x5a1680(mask)`: a lost device that reports not-reset is reset with new present parameters; `mask` clears the targets when capability bit 6 is set; `BeginScene` starts the next frame and sets `[dev+0x3a38]`.

Addresses: `0x5a1910`, `0x5a17a0`, `0x5a1680`, `0x592140`, `0x599900`, `0x599790`
Confidence: verified
Proof: one
Test: none

The wait runs when byte `+0x20a` is set, spinning on an event query at `+0x3a58` (`GetData`, flag 1, `OsSleep(0)` between tries) or, with no query, reading one back-buffer pixel with lock flags `0x810`. `0x5a1680` tests `TestCooperativeLevel` against `0x88760869` before `Reset`.

## OpenGL window and pixel formats

`0x58cf10(window procedure, format)` creates the OpenGL window with `CreateWindowExA`, class `GxWindowClassOpenGl` (`0x809a5c`), title `World of Warcraft` (`0x82e42c`) and style `0x87000000` for full screen (format byte `+7` clear). Windowed it uses `0x06cf0000`, or `0x90000000` when byte `+9` is set, and sizes the outer rectangle for the client size with `AdjustWindowRectEx`. A popup takes its size from `GetSystemMetrics` and a full-screen window from the monitor rectangle. `0x58d470` lists pixel formats through `wglGetPixelFormatAttribivARB` (`0x85a9ec`), `0x58d770` through `DescribePixelFormat`; both add records with `0x591220`.

Addresses: `0x58cf10`, `0x58d470`, `0x58d770`, `0x591220`, `0x809a5c`, `0x82e42c`, `0x85a9ec`, `0xc0f2e8`
Confidence: verified
Proof: one
Test: none

The ARB path asks for the number of formats (`0x2000`), then per format window drawing (`0x2010`), acceleration (`0x2003`), pixel type (`0x2013`), colour bits (`0x2014`), alpha bits (`0x201b`), depth (`0x2022`), stencil (`0x2023`) and samples (`0x2042`), and keeps the format only if the checks on the first six pass. The fallback skips a format whose flag byte lacks `0x24` or has `0x50` and reads its colour and depth sizes from the descriptor. Both grow the list by doubling a power-of-two capacity with `0x58da30` and `0x58b6a0`.

## OpenGL pixel format choice

`0x59dcf0(dc, format)` (`ecx` is the device context, `edx` the format record) builds an attribute list for the entry point at `0xc0f2e0` (`wglChoosePixelFormatARB`, called with the list, no float list and room for one format): draw to window `0x2001`, support OpenGL `0x2010`, full acceleration `0x2027` for `0x2003`, double buffer `0x2011`, RGBA pixel type `0x2013`, the red, green and blue sizes (`0x2015`, `0x2017`, `0x2019`), alpha `0x201b`, depth `0x2022` and stencil `0x2023` sizes chosen from the format's bit-depth codes, and, when the multisample extension flag `0xc0f2ac` is set and the format's sample count at `+0x1c` is above 1, sample buffers `0x2041` and samples `0x2042`.

Addresses: `0x59dcf0`, `0x59e070`, `0x595e70`, `0x85c434`, `0xc0f2e0`, `0xc0f2e8`, `0xc0f2ac`, `0x59da70`, `0x59daa0`, `0xc0f23c`
Confidence: verified
Proof: one
Test: none

`0x59da70` calls `0x59dcf0` when the word `0xc0f23c` is set and the GDI chooser `0x59daa0` otherwise; the GDI chooser returns 0 at once when the format's sample count at `+0x1c` is above 1 (`0x59daab`, `0x59dab4`), so it refuses multisampling. With the format it got back, `0x59dcf0` asks `0x59e070(dc, format, attribute, &value)` (a wrapper of `wglGetPixelFormatAttribivARB` through `0xc0f2e8`, one attribute) for each field and logs `SetupPixelFormatWGL(): pixelFormat does not match requested` through `0x595e70` when one differs; it returns 0 when the choice fails or a field differs. The fields read back, in order, are, only when the byte at `+7` is 0, alpha `0x201b` (skipped when the format's `+0x24` is 1), red, green and blue; then stencil `0x2023` (unless the format's `+0xc` is 5), depth `0x2022` and samples `0x2042`; when all match, `0x59dcf0` ends with `SetPixelFormat` (`0x59e047`) on a zeroed `0x28`-byte descriptor and returns its result. The reset ends with `glEnable` of capability `0xb57` (colour material) and `0xc11` (scissor), `glColorMaterial(0x408, 0x1602)` and `glPixelStorei(0xcf5, 1)`, unpack alignment 1.

## OpenGL context creation

`0x59da70(dc, format)` sets a pixel format on the device context with the chooser the previous
entry names, `0x59dcf0` when `0xc0f23c` is set and `0x59daa0` otherwise, passing both arguments on.
When the chooser returns non-zero it returns `wglCreateContext(dc)`; otherwise it returns 0 without
creating one. Its one caller is `0x58c340`.

Addresses: `0x59da70`, `0x59da7a`, `0x59da8c`, `0x59da8f`, `0x58c340`, `0x58c36a`
Confidence: verified
Proof: one
Test: none

`0x59da70` keeps `ecx` in `esi` and leaves `ecx` and `edx` unchanged into either call. `0x59da7a`
branches on the word; `0x59da8c` tests the result; `0x59da8f` calls through the import slot
0x7ff4b4, `OPENGL32.dll!wglCreateContext`, with `esi` pushed. The census of `0x59da70` finds one `e8`,
at `0x58c36a`, and no stored address.

## OpenGL initial state

`0x59ce30` sets the context's initial state: it clears the shadow cache (`0x59ba70`), loads the model-view identity, and on each texture unit sets the R and Q generation planes, eye and object, to `(0, 0, 1, 0)` and `(0, 0, 0, 1)`. When the flag at `0xc0f2a0` is set it enables point-sprite coordinate replacement on each unit. It then applies the lights (`0x591ef0`), sets a zero light-model ambient, separate specular and local viewer, loads the product of two cached matrices through `0x5a0090`, enables the vertex array (`0x8074`), capability `0x8650` when the vertex program flag `0xc0f254` is set, colour material (`0xb57`, `glColorMaterial(0x408, 0x1602)`) and scissor (`0xc11`), and sets unpack alignment 1 (`0xcf5`).

Addresses: `0x59ce30`, `0x59ba70`, `0x591ef0`, `0x5a0090`, `0xc0f2a0`, `0xc0f254`, `0xc0f3ec`
Confidence: verified
Proof: one
Test: none

The flag at `0xc0f2a0` is 1 when the GL version at `0xc0f3ec` is at least 2.0 (`0x20000`) or the extension `GL_ARB_point_sprite` is listed (`0x59b7a0`), otherwise 0; `0x58e797` copies it to `+0x310` of the device.

## Device log

The device logs to `Logs\gx.log`. The console's device start calls `0x58b5e0`, which opens the log
once, after creating `Logs`. Without the log a line is dropped; otherwise it is formatted with `vsnprintf` into 2 KiB and
written while fewer than 2 MiB (0x200000 bytes) have gone to it, and passed with the prefix
`CGxDevice::Log(): ` to `0x590630`, which only returns. The console's device start and the
hardware detection print through `0x58b600` and the device through `0x595e70`; both reach
`0x595df0`, so they act alike. The device logs its caps (`0x595e90`) and each format
it sets (`0x595f40`), as `Format: W x H Window` or `Format W x H @ R Fullscreen`, with the colour and
depth names and the sample count.

Addresses: `0x58b5e0`, `0x595d90`, `0x595da0`, `0x595db1`, `0x58b600`, `0x595df0`, `0x595e00`, `0x595e10`, `0x595e1d`, `0x595e33`, `0x595e64`, `0x590630`, `0x595e70`, `0x595e90`, `0x595f40`, `0xc0ef80`, `0xc0ef7c`
Confidence: verified
Proof: one
Test: none

`0x58b5e0` is a jump to `0x595d90`, which returns when `0xc0ef80` is set, else calls `OsCreateDirectory`
(`0x595da0`) and `SLogCreate` (`0x595db1`) into `0xc0ef80` and zeroes the count `0xc0ef7c`.
`0x595df0(format, args)` returns at `0x595e00` without a log, formats at `0x595e10`, compares the count
with 0x200000 (`0x595e1d`, `jae`), writes with `SLogWrite` (`0x595e33`) and adds the line's length.
`0x58b600` passes its own argument list without an open test and `0x595e70` tests the handle first;
`0x595df0` makes the same test at `0x595e00`, so neither prints without the log.
`0x595f40` picks the windowed form on byte `+7`, the names from `0x85ad54` by `+0x24` and `+0xc`.

## Device overrides

`CGxDevice::DeviceOverride(slot, value)` logs the pair; for slot 8 it stores `value > 1` at `+0x308` and `+0x310` and the value as a float at `+0x30c`. Each backend's
override calls it first. Both store slot 0 at `+0x2d4` and slot 7 as `value != 0` at `+0x298`. OpenGL
also takes: slot 1 = 0 turns buffer objects off (`opengl.md`); slots 2 and 3 = 0 clear the
buffer-object choice for the vertex and the index pools of usage 2, and slots 4 and 5 non-zero set
their broker choice; slot 6 picks the texture generation of mode 6, `0x8512` when non-zero and
`0x2402` when 0. Other values of slots 1 to 5 change nothing.

Addresses: `0x592370`, `0x592381`, `0x592390`, `0x5923a6`, `0x5923ad`, `0x58e320`, `0x58e33e`, `0x58e35b`, `0x58e36b`, `0x58e37c`, `0x58e38d`, `0x58e3a2`, `0x58e3b7`, `0x58e3c8`, `0x59a440`, `0x59a466`, `0x59a472`, `0xc0f2bc`
Confidence: verified
Proof: one
Test: none

`0x592390` compares the slot with 8; `cmp 1, value; sbb; neg` is `value > 1`, unsigned, and `0x5923a6`
loads the value as a 64-bit integer. `0x58e320` dispatches slots 0 to 7 through the table `0x58e3f0`
(`0x58e33e`): slot 1 stores 0 in `0xc0f2bc` (`0x58e35b`), slots 2 and 3 store 0 at `+0x3a78` and `+0x3a84`,
slots 4 and 5 store 1 at `+0x3a60` and `+0x3a6c`, slot 6 writes `+0x39dc` (`0x58e3b7`, `0x58e3c8`).
`0x59a440` handles 0 (`0x59a472`) and 7 (`0x59a466`). The slots come from the override string
(`console/display`, Override string).

## Stream pools

Both device constructors create two pools: `Stream_vtx`, of kind 0, usage 2 and 2,883,540 bytes,
at `+0x26c4`, and `Stream_idx`, of kind 1, usage 2 and 262,144 bytes, at `+0x26c8`.

Addresses: `0x594730`, `0x594744`, `0x594763`, `0x85ad4c`, `0x85ad50`, `0x5947a0`, `0x594320`
Confidence: verified
Proof: one
Test: none

`0x594744` and `0x594763` call `0x5947a0(kind, 2, size, 3, name)`, the sizes loaded from `0x85ad4c` and
`0x85ad50`, which nothing writes; the results go to `+0x26c4` and `+0x26c8`. `0x5947a0` allocates 0x34
bytes, runs `0x594320`, which stores the kind at `+0x8`, the usage at `+0xc`, the size at `+0x10`, the
fourth argument at `+0x2c` and the name at `+0x30`, and links the pool into the list at `+0x26a8`. The callers are
`0x58df9a` and `0x598dc0`.

## Format lists

`0x589a00(api, list)` clears the list, reserves room for 256 records, sets its growth step to 64,
and fills it from the backend `api` names: 0 OpenGL, 1 Direct3D, any other none. It returns whether
the list holds a record. The OpenGL fill opens a hidden window of class `adfmt`, takes its first
suitable pixel format and a context as the probe does (`opengl.md`), and lists the formats through
`wglGetPixelFormatAttribivARB` when `WGL_ARB_pixel_format` is listed, else through
`DescribePixelFormat`, then puts the previous context back.

Addresses: `0x589a00`, `0x589a1c`, `0x589a4b`, `0x589a5d`, `0x589a73`, `0x58d2b0`, `0x58d2f1`, `0x58d3a3`, `0x58d3e2`, `0x58d3fd`, `0x58d40b`, `0x58d412`, `0x58d41f`, `0x48c455`
Confidence: verified
Proof: one
Test: none

`0x589a1c` destroys each 0x38-byte record, `0x589a4b` grows the capacity to 0x100 when it is less, and
the step `+0xc` is set to 0x40. `0x589a5d` calls `CGxDeviceD3d::EnumerateFormats` and `0x589a73`
`0x58d2b0`. `0x58d3e2` makes the new context current after saving the current one, `0x58d3fd` tests
the extension and `0x58d40b` or `0x58d412` lists; `0x58d41f` restores. The caller is
`BuildMultisampleList` (`0x48c455`).

