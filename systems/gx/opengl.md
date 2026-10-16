# gx: the OpenGL context and extensions

How the OpenGL backend finds its extensions, creates and releases its context, and backs its
vertex and index pools with buffer objects. The code is in `CGxDeviceOpenGl.cpp` and the
extension file beside it, inside the device layer's extent (`overview.md`). `device.md` has the
window and the pixel format; `renderstate.md` the programs and the state cache.

Roots: `0x58ce40`, `0x58d2b0`, `0x58e7e0`

## Loading the entry points

The first context the backend makes is preceded, once per run, by a probe: a hidden window of
class `hwacc`, the first pixel format of its device context whose flags have `0x4` or `0x20` and
neither `0x10` nor `0x40`, and a context made current on it. With it current the probe reads the
GL extensions and entry points (`0x59aff0`) and the WGL ones (`0x58c580`), then deletes the
context, the window and the class. The flag `0xc0ed3c` is set before anything is tried, so a probe
that fails is not repeated.

Addresses: `0x58c400`, `0x58c417`, `0x58c4fe`, `0x58c502`, `0x58c516`, `0x58c533`, `0x58c538`, `0x58c544`, `0x58c55c`, `0x58c567`, `0xc0ed3c`, `0x58c347`
Confidence: verified
Proof: one
Test: none

`0x58c347` calls `0x58c400` at the start of every context creation; `0x58c40f` returns when the flag
is set and `0x58c417` sets it. The pixel format loop asks `DescribePixelFormat` for the count, then
tests the flag byte with `test al,0x24` (`0x58c4fe`, skipping on zero) and `test al,0x50`
(`0x58c502`, skipping on non-zero), and takes the first that passes with `SetPixelFormat`
(`0x58c516`). A null context skips both loaders and still reaches `wglDeleteContext` (`0x58c544`).

## Extension tests

An extension is listed when its name starts a space-separated word of the extension string,
compared ignoring case over the name's length only, so `GL_EXT_texture` matches
`GL_EXT_texture3D`. For a GL extension the test first looks the name up in the environment: a
variable of that name whose text converts to 0 makes the extension absent. The GL string is
`glGetString(GL_EXTENSIONS)`; the WGL string comes from `wglGetExtensionsStringARB` on the current
device context, and with no such entry point every WGL extension is absent. Both strings are
cached until the context is released.

Addresses: `0x59b7a0`, `0x59b7a4`, `0x59b7b1`, `0x59b7c7`, `0x59b850`, `0x59b85a`, `0x59b869`, `0x59b7f0`, `0x59b813`, `0x59b822`, `0x59b790`, `0xc0f3f4`, `0xc0f3f0`, `0xc0f2ec`, `0x414e94`
Confidence: verified
Proof: one
Test: none

`0x59b7a4` calls the C runtime's `_getenv` (`0x414e94`: `__lock(0xc)`, `__getenv_lk` `0x414eb5`, `__unlock`) and `0x59b7b1` `SStrToInt`; a zero
returns 0. `0x59b7c7` pushes `0x1f03`. `0x59b85a` returns 0 for a null `0xc0f2ec`, and `0x59b869`
passes `wglGetCurrentDC()` to it. `0x59b7f0(list, name)` calls `SStrCmpI(position, name,
strlen(name))` (`0x59b813`), then skips to the next space and over a run of spaces (`0x59b822` to
`0x59b839`). `0x59b790` zeroes both caches; its caller is the context release.

## GL and WGL extension loads

`0x59aff0` parses `GL_VERSION` as `%d.%d` into `major << 16 | minor` (−1 for an unread part), then
tests thirty-six GL extensions, from `GL_EXT_compiled_vertex_array` to
`GL_EXT_framebuffer_multisample`, storing their flags and taking their entry points with
`wglGetProcAddress`. Without `GL_ARB_multitexture` the texture unit count is 1. `0x58c580` takes
`wglGetExtensionsStringARB`, then, for `WGL_ARB_pixel_format`, `WGL_ARB_pbuffer` and
`WGL_EXT_swap_control`, a flag and their entry points: the pixel format pair, five pbuffer calls and
`wglSwapIntervalEXT`.

Addresses: `0x59aff0`, `0x59b019`, `0x59b031`, `0x59b0a4`, `0x59b1ff`, `0x59b212`, `0x59b22a`, `0x59b236`, `0xc0f3ec`, `0xc0f2bc`, `0x58c580`, `0x58c593`, `0x58c5a6`, `0x58c5d6`, `0x58c62a`, `0x58c636`, `0xc0f234`, `0xc0f2c8`
Confidence: verified
Proof: one
Test: none

Each load is `push name; mov [slot], eax; call esi`, so a slot receives the answer of the call before
it. `0x59b019` is `sscanf` with both outputs preset to −1; `0x59b031` stores the version. `0x59b0a4`
stores 1 in `s_textureUnitCount`. `GL_ARB_vertex_buffer_object` sets `0xc0f2bc` (`0x59b1ff`) and loads
`glBindBufferARB` into `0xc0f35c` (`0x59b212`), `glGenBuffersARB` into `0xc0f354` (`0x59b22a`) and
`glBufferDataARB` into `0xc0f350` (`0x59b236`). `0x58c593` stores the WGL string entry; `0x58c5a6`,
`0x58c5d6` and `0x58c62a` set the three WGL flags, and `0x58c636` stores `wglSwapIntervalEXT`.

## Context creation and release

`CGxDeviceOpenGl::DeviceSetFormat(format)` logs the format, releases the context, copies the
format, opens the window for it and creates the context. Creation gets the window's device
context, sets the pixel format and makes a context current; with `WGL_EXT_swap_control` it passes
the format's vertical sync byte, as 0 or 1, to `wglSwapIntervalEXT`. It then reads the caps, calls
`0x58bfd0` and the device's slot `+0xc` with the format, sets the initial state, flushes the render
states and calls `0x59b890(3)`. A failed format or context leaves the device without one and returns 0; success uploads the working
gamma ramp. Release restores the saved desktop ramp to the device context before letting it go.

Addresses: `0x58ce40`, `0x58ce49`, `0x58ce6f`, `0x58ce8f`, `0x58cec1`, `0x58cefb`, `0x58c340`, `0x58c353`, `0x58c36a`, `0x58c390`, `0x58c3a0`, `0x58c3b4`, `0x58c3bc`, `0x58c3d2`, `0x58c640`, `0x58c678`, `0x58c6a0`, `0x58c6b4`
Confidence: verified
Proof: one
Test: none

`0x58ce49` logs `CGxDeviceOpenGl::DeviceSetFormat():` and `0x58ce5e` the format; `0x58ce6f` releases,
`0x58ce8f` validates the copy and `0x58cec1` creates. `0x58c353` is `GetDC` on `+0x39e4` into `+0x39f0`,
`0x58c36a` the format and context into `+0x39f4`, `0x58c390` `wglMakeCurrent`; a failure releases (`0x58c37b`). `0x58c3a0` tests
`0xc0f234` and `0x58c3b4` calls `0xc0f2c8` with `setne` of byte `+0x2c`. `0x58c3bc` reads the caps
(`0x58e480`, which logs them) and `0x58c3d2` is `CGxDeviceOpenGl::InitState`. `0x58cefb` calls slot
`+0x30` with `+0x328`. `0x58c678` clears the extension caches; `0x58c6a0` is
`SetDeviceGammaRamp(dc, +0x928)` and `0x58c6b4` `ReleaseDC`.

## Pool buffers and the broker

`CGxDeviceOpenGl::CreatePoolAPI(pool)` backs a pool with system memory of its size when buffer
objects are off. Otherwise, by the pool's kind (0 vertex, 1 index) and usage, a device table
chooses a broker, one buffer object of the pool's size, or system memory. A broker holds, for each
power-of-two size 2^k from 2^12 bytes to 2^20 (vertex) or 2^17 (index), `1200 >> (k − 10)` buffers
above 2^12, 1200 at 2^12 and 32 at 2^18, each given its size with no data and `GL_STREAM_DRAW_ARB`.

Addresses: `0x58e7e0`, `0x58e7e3`, `0x58e803`, `0x58e817`, `0x58e81e`, `0x58e84c`, `0x58e867`, `0x58e8bf`, `0x58e8e0`, `0x58dab0`, `0x58db2f`, `0x58db39`, `0x58db54`, `0x58db8d`, `0x58dbb3`
Confidence: verified
Proof: one
Test: none

`0x58e7e3` tests `0xc0f2bc`; `0x58e803` reads `[dev + 0x3a58 + 4·(3·kind + usage)]` for a broker and
`0x58e867` the table at `+0x3a70` for a buffer object; `0x58e817` and `0x58e81e` pick 0x11 and 0x14.
`0x58e84c` builds the broker with the target `0x8892` or `0x8893`. In `0x58dab0` the count is 1200
(`0x4b0`), shifted right by `size − 10` above 12 (`0x58db2f`), and 32 at 18 (`0x58db39`); `0x58db54`
generates the names and `0x58db8d` calls `glBufferDataARB(target, 2^size, 0, 0x88e0)`. The two lines
`0x58dbb3` formats go to the empty print `0x4420b0`.
