# gx: the device's virtual slots

The 62 virtual slots of the device, `+0` to `+0xf4`, in the three tables the constructors store:
`CGxDevice::vftable` (`0x809da0`), `CGxDeviceOpenGl::vftable` (`0x809af8`) and
`CGxDeviceD3d::vftable` (`0x809ef8`). The base table points sixteen slots at the pure-call stub
`0x40a2d7` (pure in the tables below); a backend that keeps the base's function shares its
address (base). A slot has one method name across the three classes. The `Gx*` facade
(`device.md`) reaches every slot from `+0x18` on through the device at `0xc0ed38`; slots `+0` to
`+0x14` are called by the device's own code. Each table ends at `+0xf4`: the dword after it starts
other data; after the Direct3D table, `0x809ff0` is the exception scope table that the buffer lock
pushes (`0x59a4e5`, `draw.md`, Stream locks).

Roots: `0x589ac0`, `0x589ba0`

## Texture, state and cursor slots

Slot `+0` uploads a texture whose dirty byte `+0` is set, and the base counts the upload. Slot
`+0x18`, not slot `+0`, is the deleting destructor (Life cycle slots).

| slot | name | base | OpenGl | Direct3D | does |
|---|---|---|---|---|---|
| `+0x00` | `LoadTexture` | `0x594c80` | `0x59f5b0` | `0x5a0d70` | uploads a dirty texture; the base adds 1 to `+0x8c` and the dirty rectangle's bytes to `+0x90`, then clears the byte |
| `+0x04` | `ApplyRenderState` | pure | `0x59d350` | `0x5a2f00` | applies one render state (`renderstate.md`) |
| `+0x08` | `ApplyShaderParams` | pure | `0x59ee60` | `0x5a0420` | uploads each changed parameter of a shader's list, or every one when forced |
| `+0x0c` | `CreateCursor` | `0x595640` | base | `0x59a960` | picks the hardware or the software cursor and makes its 32 by 32 texture |
| `+0x10` | `DestroyCursor` | `0x5956d0` | base | `0x59a9e0` | frees the cursor texture |
| `+0x14` | `DrawCursor` | `0x5956f0` | base | `0x59a870` | draws the software cursor, or hands the hardware cursor its image (Cursor) |

Addresses: `0x594c80`, `0x594c87`, `0x594c90`, `0x594cb3`, `0x594cbe`, `0x59f5b0`, `0x59f7f9`, `0x5a0d70`, `0x5a0dcd`, `0x594b90`, `0x59d350`, `0x5a2f00`, `0x59ee60`, `0x5a0420`, `0x5a04a1`, `0x5a04b7`, `0x5a04e3`, `0x595640`, `0x59a960`, `0x5956d0`, `0x59a9e0`, `0x5956f0`, `0x59a870`
Confidence: verified
Proof: one
Test: none

`0x594c87` tests the byte; `0x594c90` increments `+0x8c`, `0x594cb3` adds `CGxDevice::TextureBytes` of the
rectangle at `+4` to the bytes at `+0x90`, and `0x594cbe` clears the byte after resetting the rectangle to the
whole texture. Direct3D creates the texture first when byte `+1` is set or `+0x48` is null, uploads,
then calls the base (`0x5a0dcd`); OpenGL calls it from its upload (`0x59f7f9`). `0x594b90` marks a
texture dirty: it sets byte `+0`, grows the rectangle by its argument (the whole texture for an
empty one) and calls slot `+0` when its third argument is set. `ApplyShaderParams(shader, list,
force)` walks the list and clears each parameter's dirty word `+0x50`: Direct3D sends constants
with `SetPixelShaderConstantF` (`0x5a04a1`) for a shader whose `+0x24` is 1 and
`SetVertexShaderConstantF` (`0x5a04b7`) otherwise, and a bump matrix as texture stage states 7 to
10 (`0x5a04e3`); OpenGL calls `0x59ece0` or `0x59ede0` by `+0x24`.

## Life cycle slots

The device is made with slot `+0x20` (`GxDevCreate`) and torn down with slots `+0x24` and
`+0x18`. Slot `+0x1c` makes it on a window the caller owns.

| slot | name | base | OpenGl | Direct3D | does |
|---|---|---|---|---|---|
| `+0x18` | `DeletingDestructor` | `0x591c90` | `0x58dfb0` | `0x598de0` | runs the destructor, frees the object when bit 0 of the argument is set and returns it |
| `+0x1c` | `CreateOnWindow` | `0x591fa0` | `0x58cce0` | `0x599bc0` | makes the context or device on a given window and copies the format |
| `+0x20` | `DeviceCreate` | `0x591f80` | `0x58cbb0` | `0x599a60` | saves the gamma ramp, registers a window class and calls slot `+0x28` |
| `+0x24` | `DestroyDevice` | `0x591fc0` | `0x58cda0` | `0x599c30` | releases what the create made, its own window and its class |
| `+0x28` | `DeviceSetFormat` | `0x591fd0` | `0x58ce40` | `0x599c90` | sets a format (`device.md`) |
| `+0x2c` | `SetTextureSkip` | `0x592050` | `0x58d060` | `0x599da0` | stores the top mip levels to skip and recreates every texture |
| `+0x30` | `SetGammaRamp` | `0x592080` | `0x58d0c0` | `0x599e10` | copies a ramp to `+0x328` and sends it in full screen |
| `+0x34` | `SetGamma` | `0x592060` | `0x58d080` | `0x599dc0` | builds `+0x328` from a gamma and sends it in full screen |
| `+0x38` | `SetFullColorTextures` | `0x5920a0` | `0x58d280` | `0x599e60` | stores whether textures are made 32-bit and recreates every texture |
| `+0x3c` | `GetWindow` | pure | `0x58d2a0` | `0x599e80` | returns the window, `+0x39e4` or `+0x3894` |

Addresses: `0x591c90`, `0x591c9b`, `0x591d00`, `0x58dfb0`, `0x58dfe0`, `0x598de0`, `0x598e10`, `0x591fa0`, `0x58cce0`, `0x599bc0`, `0x591f80`, `0x58cbb0`, `0x599a60`, `0x591fc0`, `0x58cda0`, `0x599c30`, `0x599720`, `0x592050`, `0x58d060`, `0x599da0`, `0x59f980`, `0x5a0700`, `0x6890c2`, `0x58d0c0`, `0x58d080`, `0x5920a0`, `0x58d280`, `0x599e60`, `0x59f467`, `0x5a0963`, `0x599129`, `0x589c30`, `0x58d2a0`, `0x599e80`, `0x589ba0`, `0x599330`, `0x598eb0`
Confidence: verified
Proof: one
Test: none

`0x591c9b` tests bit 0 before `SMemFree(this, "delete")`; the destructors are `0x591d00`, which stores
`0x809da0` back, and `0x58dfe0` and `0x598e10`, which end in it. `0x589ba0` calls slot `+0x24`, then
slot `+0x18` with 1. Both creates store whether the window is the device's own (`+0x39e8`,
`+0x389c`); destroy removes only an own window. Direct3D's destroy runs `0x599330`, which calls
`0x599720` (the cursor, the resources, the query, the default texture and the device), then
`0x598eb0` (the `IDirect3D9` and `d3d9.dll`).
`0x599a60` logs `CGxDeviceD3d::DeviceCreate()`. Texture recreation (`0x59f980`, `0x5a0700` with 1)
releases each API texture and marks the record for creation and a full upload through
`0x594b90`. The base mip option calls slot `+0x2c` (`0x6890c2`). OpenGL's gamma slots send the ramp
with `SetDeviceGammaRamp` unless byte `+0x20b` (windowed) is set. With `+0x320` set OpenGL creates
textures as `GL_RGBA8` (`0x59f467`) and Direct3D as `A8R8G8B8` (`0x5a0963`) when the adapter has it
(`0x599129`); slot `+0x38`'s facade `0x589c30` is unreached, so `+0x320` stays 0.

## Capture, window and frame slots

These slots read the frame back, follow the window, bind render targets and end the frame.
Rectangles of floats are `{top, left, bottom, right}`.

| slot | name | base | OpenGl | Direct3D | does |
|---|---|---|---|---|---|
| `+0x40` | `RequestCapture` | `0x592130` | base | base | arms a capture at the next present (`capture.md`) |
| `+0x44` | `GetCapture` | `0x5921a0` | base | base | returns the capture's width, height and buffer |
| `+0x48` | `ReadPixels` | pure | `0x58e0c0` | `0x599e90` | reads a rectangle of the colour buffer as 32-bit BGRA, top row first |
| `+0x4c` | `ReadDepths` | pure | `0x58e220` | `0x59a130` | reads a rectangle of depths as floats; Direct3D writes zeros |
| `+0x50` | `HandleWindowEvent` | pure | `0x58ca60` | `0x59a1c0` | follows a window event (Window events) |
| `+0x54` | `SetRenderTarget` | `0x592230` | `0x58f5e0` | `0x59a2e0` | binds a texture, or none, as target 0 (colour) or 1 (depth) |
| `+0x58` | `DeviceOverride` | `0x592370` | `0x58e320` | `0x59a440` | applies an override (`device.md`) |
| `+0x5c` | `GetDrawRect` | pure | `0x5a0180` | `0x5a1a30` | copies the draw rectangle `+0x1ec` |
| `+0x60` | `GetScreenRect` | pure | `0x58d970` | `0x5a1a60` | returns the draw rectangle in screen coordinates when windowed |
| `+0x64` | `SetClearColor` | `0x592400` | base | base | stores the clear colour at `+0x324` |
| `+0x68` | `PresentFrame` | `0x592430` | `0x59ba10` | `0x5a1910` | draws the cursor and shows the frame; the base counts it (Frame counters) |
| `+0x6c` | `Clear` | `0x592520` | `0x59b910` | `0x5a17d0` | clears the buffers a mask names; the base does nothing |

Addresses: `0x592130`, `0x5921a0`, `0x58e0c0`, `0x599e90`, `0x58e220`, `0x58e2b2`, `0x59a130`, `0x58ca60`, `0x59a1c0`, `0x592230`, `0x58f5e0`, `0x58d100`, `0x58f270`, `0x58f620`, `0x59a2e0`, `0x59a3df`, `0x592370`, `0x58e320`, `0x59a440`, `0x5a0180`, `0x5a1a30`, `0x58d970`, `0x5a1a60`, `0x592400`, `0x592430`, `0x59ba10`, `0x59ba2b`, `0x5a1910`, `0x5a1943`, `0x592520`, `0x59b910`, `0x5a17d0`
Confidence: verified
Proof: one
Test: none

`0x58e2b2` is `glReadPixels` of `GL_DEPTH_COMPONENT` floats, rows then swapped top for bottom.
`SetRenderTarget(target, texture, face)` stores the pair in the 12-byte record `+0x2840 + 12 *
target`; with neither target bound the draw rectangle is the window's `+0x1dc`, otherwise
`{0, 0, height, width}` of the texture. Direct3D sets the texture's level 0, or its cube face,
with `SetRenderTarget` and `SetDepthStencilSurface` (`0x59a3df`). OpenGL dispatches on `+0x39e0`:
0 copies the old target into its texture and switches between the window and the pbuffer
(`0x58d100`); 1 uses framebuffer objects (`0x58f270`), and only the unreached `0x58f620` stores 1.
`GetScreenRect` maps the corners with `MapWindowPoints`; full screen it copies `+0x1ec`. Both
presents call slot `+0x14` (`0x59ba2b`, `0x5a1943`) before the swap.

## Transform and immediate slots

The transform slots store the matrix in the base and load it in the backend. The immediate slots
collect vertices one call at a time; their facade wrappers, `0x58a840` to `0x58a900`, are
unreached.

| slot | name | base | OpenGl | Direct3D | does |
|---|---|---|---|---|---|
| `+0x70` | `SetProjection` | `0x5925e0` | `0x5a0140` | `0x5a11d0` | stores the projection at `+0xf50` and loads it |
| `+0x74` | `SetView` | `0x592610` | `0x5a0160` | `0x5a1450` | stores the view; OpenGL marks the model-view and lights stale |
| `+0x78` | `PrimRender` | `0x592b00` | `0x59fc90` | `0x5a0f50` | draws primitive records; the base counts them (`draw.md`) |
| `+0x7c` | `BeginPrimitive` | `0x592c40` | base | base | starts a primitive of a type at `+0xac` and clears the vertex arrays |
| `+0x80` | `EndPrimitive` | `0x592fd0` | base | base | builds a vertex buffer of the vertices and draws them |
| `+0x84` | `AddVertex` | `0x592cf0` | base | base | appends a position with the current colour, normal and coordinates |
| `+0x88` | `SetVertexTexCoord` | `0x592f90` | base | base | sets the coordinate pair of a unit, `+0xc8 + 8 * unit` |
| `+0x8c` | `SetVertexNormal` | `0x592f40` | base | base | sets the normal, `+0x108` |
| `+0x90` | `SetVertexColor` | `0x592f70` | base | base | sets the colour, `+0x114` |
| `+0x94` | `SetPointSize` | `0x597b00` | `0x59fde0` | base | calls `glPointSize`; the base does nothing |
| `+0x98` | `SetLineWidth` | `0x597b10` | `0x59fe00` | base | calls `glLineWidth`; the base does nothing |
| `+0x9c` | `SetCap` | `0x593620` | base | base | sets a capability bit (`renderstate.md`) |

Addresses: `0x5925e0`, `0x5a0140`, `0x59fe20`, `0x5a11d0`, `0x592610`, `0x5a0160`, `0x5a016f`, `0x59ca60`, `0x59c820`, `0x5a1450`, `0x592b00`, `0x59fc90`, `0x5a0f50`, `0x592c40`, `0x592fd0`, `0x592cf0`, `0x592ef3`, `0x592f15`, `0x592f90`, `0x592f40`, `0x592f70`, `0x597b00`, `0x59fde0`, `0x597b10`, `0x59fe00`, `0x593620`, `0x58a840`, `0x58a900`
Confidence: verified
Proof: one
Test: none

OpenGL's projection slot calls `0x59fe20` after the store; its view slot sets `+0x3a50`
(`0x5a016f`), which makes `0x59ca60` reload the model-view and `0x59c820` resend every light's
position. `+0x1d8` marks which attributes the vertices carry: bit 0 position, bit `1 << (unit + 1)`
a unit's coordinates, `0x200` the normal, `0x400` the colour. `AddVertex` ends and begins the
primitive again (`0x592f15`) when its vertex count reaches the count that holds 0x20000
primitives (`0x592ef3`). `EndPrimitive` builds the buffer with `GxBuildVerticesCounted` and draws
through the facade.

## Buffer and texture slots

A pool (`CGxPool`) is a block of vertex or index memory and a buffer (`CGxBuf`) a range of
one; `device.md` has the two stream pools. Textures are `CGxTexture` records (`textures.md`).

| slot | name | base | OpenGl | Direct3D | does |
|---|---|---|---|---|---|
| `+0xa0` | `PoolSizeSet` | pure | `0x58ea30` | `0x59a480` | gives a pool storage of a new size |
| `+0xa4` | `DestroyPool` | `0x594840` | `0x58eb00` | `0x59a4b0` | frees a pool's storage, its buffers and the pool |
| `+0xa8` | `LockBuffer` | `0x594690` | `0x58eb20` | `0x59a690` | returns writable memory for a buffer and marks it locked |
| `+0xac` | `UnlockBuffer` | `0x5946c0` | `0x58ed10` | `0x59a720` | ends the write and adds its bytes to `+0xa8` |
| `+0xb0` | `WriteBuffer` | `0x594660` | `0x58ee10` | `0x59a750` | copies data into a buffer at an offset |
| `+0xb4` | `CreateTexture` | `0x594a80` | base | `0x5a0810` | makes a texture record (`textures.md`) |
| `+0xb8` | `CreateTexture2D` | `0x594980` | `0x59f140` | `0x5a07e0` | makes a two-dimensional texture record from seven arguments |
| `+0xbc` | `DestroyTexture` | `0x594e60` | `0x59f810` | `0x5a0850` | frees the API texture, unlinks and frees the record |
| `+0xc0` | `CopyToTexture` | pure | `0x59fb80` | `0x5a0de0` | copies a rectangle of the render target into a texture level |

Addresses: `0x58ea30`, `0x59a480`, `0x594840`, `0x594770`, `0x58eb00`, `0x59a4b0`, `0x594690`, `0x58eb20`, `0x59a690`, `0x59a4e0`, `0x5946c0`, `0x58ed10`, `0x59a720`, `0x594660`, `0x58ee10`, `0x59a750`, `0x594a80`, `0x5a0810`, `0x594980`, `0x591540`, `0x59f140`, `0x5a07e0`, `0x594e60`, `0x59f810`, `0x5a0850`, `0x59fb80`, `0x59fc36`, `0x5a0de0`, `0x5a0ed3`, `0x591600`
Confidence: verified
Proof: one
Test: none

The base destructor's `0x594770` passes the two stream pools to slot `+0xa4`. OpenGL backs a pool
with a buffer object through `glBufferDataARB`, `glMapBufferARB` (`GL_WRITE_ONLY_ARB`, `0x88b9`),
`glUnmapBufferARB` and `glBufferSubDataARB`, or with system memory (`opengl.md`). Direct3D locks
through `0x59a4e0` (`draw.md`, Stream locks). `CreateTexture2D` builds the record with `0x591540`, which
passes 0 for the target `+0x30` and for `+0x2c`, and the format for both `+0x34` and `+0x38`, to the
common initialiser `0x591600`, and links it like `CreateTexture`. `CopyToTexture(texture, destination, source, size, level, face)` is
`glCopyTexSubImage2D` (`0x59fc36`) or `StretchRect` from the target surface `+0x3a4c`
(`0x5a0ed3`); both make the API texture first when it has none.

## Query slots

The queries are occlusion queries; the sun glare (`0x7e5238`) counts its samples with them.
OpenGL's result for query type 0 is the passed sample count divided by the format's sample count
`+0x220`; Direct3D's is the count itself.

| slot | name | base | OpenGl | Direct3D | does |
|---|---|---|---|---|---|
| `+0xc4` | `CreateQuery` | `0x594fd0` | `0x58f060` | `0x59ade0` | allocates a 16-byte query of a type, links it at `+0x2858` and makes the API query |
| `+0xc8` | `DestroyQuery` | `0x595070` | `0x58f090` | `0x59ae10` | deletes the API query, unlinks and frees the record |
| `+0xcc` | `BeginQuery` | pure | `0x58f0b0` | `0x59ae30` | starts counting |
| `+0xd0` | `EndQuery` | pure | `0x58f100` | `0x59ae80` | stops counting |
| `+0xd4` | `GetQueryParam` | pure | `0x58f150` | `0x59aec0` | returns the result size in bytes (0) or whether it is ready (1) |
| `+0xd8` | `GetQueryResult` | pure | `0x58f200` | `0x59af30` | returns the result |

Addresses: `0x594fd0`, `0x58f060`, `0x58efc0`, `0x59ade0`, `0x59ad40`, `0x595070`, `0x58f090`, `0x59ae10`, `0x58f0b0`, `0x59ae30`, `0x58f100`, `0x59ae80`, `0x58f150`, `0x59aec0`, `0x58f200`, `0x58f250`, `0x59af30`, `0x59af6b`, `0x809ac4`, `0x809ef4`, `0x7e5238`
Confidence: verified
Proof: one
Test: none

The type indexes `0x809ac4` (OpenGL, `GL_SAMPLES_PASSED_ARB`) and `0x809ef4` (Direct3D, 9,
`D3DQUERYTYPE_OCCLUSION`). OpenGL uses `glGenQueriesARB` when its extension flag is set,
`glBeginQueryARB`, `glEndQueryARB`, `glGetQueryivARB` (`GL_QUERY_COUNTER_BITS_ARB`, divided by 8)
and `glGetQueryObjectuivARB` (`GL_QUERY_RESULT_AVAILABLE_ARB`, `GL_QUERY_RESULT_ARB`); each returns 1
when `glGetError` is 0. `0x58f250` divides by `+0x220` for type 0. Direct3D issues begin (2) and end
(1), and `0x59af6b` repeats `GetData` with the flush flag while it returns `S_FALSE`, so it spins
until the result is ready. Begin, end and the two reads return 0 before the device is ready
(`+0xf2c`).

## Shader and cursor slots

Shaders come from `.bls` files (Shader files); the cursor is a 32 by 32 image at `+0x2890`.

| slot | name | base | OpenGl | Direct3D | does |
|---|---|---|---|---|---|
| `+0xdc` | `LoadShaders` | `0x595240` | `0x59eb30` | `0x5a0370` | finds or makes the shaders of a target and file, then builds their programs |
| `+0xe0` | `ReleaseShader` | `0x595520` | `0x59ebb0` | `0x5a03d0` | drops a reference; the last frees the program and the shader |
| `+0xe4` | `SetShaderConstants` | pure | `0x59ec00` | `0x5a0630` | sends constant registers of the vertex (0) or pixel (1) program |
| `+0xe8` | `ReloadShader` | pure | `0x59f070` | `0x5a0680` | rereads one shader from a named file and rebuilds its program |
| `+0xec` | `ShowCursor` | `0x595c20` | base | `0x59a7b0` | stores whether the cursor is shown at `+0x2880` |
| `+0xf0` | `GetCursorPixels` | `0x595c30` | base | base | returns the cursor's pixels, 32-bit ARGB rows of 0x80 bytes |
| `+0xf4` | `SetCursorHotspot` | `0x595c40` | base | `0x59a840` | stores the hot spot and marks the cursor image changed |

Addresses: `0x595240`, `0x5953c0`, `0x59546e`, `0x59eb30`, `0x5a0370`, `0x595520`, `0x59ebb0`, `0x5a03d0`, `0x59ec00`, `0x5a0630`, `0x59f070`, `0x5a0680`, `0x595c20`, `0x59a7b0`, `0x59a82c`, `0x595c30`, `0x595c40`, `0x59a840`, `0x58b330`, `0x58b390`
Confidence: verified
Proof: one
Test: none

`LoadShaders(shaders, target, name, count)` keys its hash at `+0x264c + 0x28 * target` by name;
with a count above 1 it makes `count` shaders named `%s:%d` (`0x5953c0`), then reads them from the
file (`0x59546e`). A shader's reference count is `+0x1c`. Direct3D sends constants with
`SetVertexShaderConstantF` or `SetPixelShaderConstantF`; OpenGL with
`glProgramEnvParameter4fvARB` at vertex level 2, `glProgramParameters4fvNV` at levels 3 and 4, and
`glProgramLocalParameter4fvARB` for pixel level `0xa`, after flushing the render states. Model
batches call slot `+0xe4` through `0x58b330`; `ReloadShader`'s facade `0x58b390` is unreached.
Direct3D's `ShowCursor` also calls `IDirect3DDevice9::ShowCursor` (`0x59a82c`) when the hardware
cursor is in use and inside the client rectangle.

## Cursor

The cursor is the 32 by 32 image at `+0x2890` with its hot spot at `+0x2888`, `+0x288c`. It is a
hardware cursor when the format's hardware cursor byte (`+5`) is set and the device reports colour
cursors (`+0x300`); OpenGL never does. Otherwise the base draws it at each present while it is
shown (`+0x2880`, 1 at construction) and the pointer is inside the draw rectangle: a 32 by 32
textured quad whose top left is the pointer minus the hot spot, which Direct3D moves half a pixel
left and up. A hardware cursor gets its image at the present after the hot spot is set: the 32
rows go into its surface and to `SetCursorProperties`.

Addresses: `0x595640`, `0x595651`, `0x5991b0`, `0x58e759`, `0x595be0`, `0x5956f0`, `0x595720`, `0x5957f0`, `0x595a50`, `0x595bad`, `0x59a870`, `0x59a885`, `0x59a89d`, `0x5a193a`, `0x59a8e4`, `0x59a93f`, `0x595c40`, `0x58acd0`, `0x59a840`, `0x59891d`
Confidence: verified
Proof: one
Test: none

`0x595651` reads `+0x300`; Direct3D's caps fill stores `CursorCaps & 1` there (`0x5991b0`) and
OpenGL's stores 0 (`0x58e759`). The software texture's callback `0x595be0` hands out `+0x2890`
with a pitch of 0x80, and `SetCursorHotspot` marks the whole texture dirty (`0x58acd0`). The draw
reads the pointer with `OsInputGetCursorPos` (`0x595720`), binds the texture as state 0x17
(`0x5957f0`) and draws one record (`0x595bad`) between a render state push and pop. The corners
are `(px - hx) / width` and `1 - (py - hy) / height`, and their sums with 32 pixels; for tag 1
(`0x595a50`) it subtracts 0.5 / width from x and adds 0.5 / height to y. Direct3D's `DrawCursor`
draws the software cursor in a scene of its own, between `0x5a1680(0)` (`0x59a885`) and
`0x5a17a0` (`0x59a89d`), after the frame's `EndScene` (`0x5a193a`) and before `Present`. It
copies the rows under `LockRect` flags `0x810` (`0x59a8e4`) when `+0x3a5c` is set, then calls
`SetCursorProperties` (`0x59a93f`) and clears the flag. Its window procedure hides the system
cursor over the client area and calls `ShowCursor` with shown-and-hardware (`0x59891d`).

## Window events

Both window procedures pass five events to slot `+0x50` with a rectangle and a size type: 0 on
`WM_SIZE` (type 1 minimised, 2 `SIZE_MAXHIDE`) and on a full-screen deactivation (type 1), 1 on a
full-screen `WM_DISPLAYCHANGE`, 2 on `WM_DESTROY`, 3 on `WM_SETFOCUS`, 4 on `WM_KILLFOCUS`. Event 0
of type 1 or 2 clears the active flag `+0xf30`; any other sets it, stores the window and draw
rectangles and dirties the viewport, then Direct3D, when ready, resets its device and OpenGL
remakes its pbuffer if the window grew. In full screen OpenGL takes the format's display mode, maximised with the pointer clipped, on event 3,
and restores the desktop mode, minimised, on 2 and 4; Direct3D clips the pointer on 3.

Addresses: `0x598750`, `0x5987b9`, `0x59881d`, `0x598866`, `0x598883`, `0x598895`, `0x5988ad`, `0x598918`, `0x58c6d0`, `0x58c739`, `0x58c7fa`, `0x58c812`, `0x58c9c5`, `0x58ca60`, `0x58ca79`, `0x58cacd`, `0x58cafe`, `0x58c140`, `0x59a1c0`, `0x59a256`, `0x599d50`, `0x5922f0`
Confidence: verified
Proof: one
Test: none

Event 1 stores the rectangles while active. The switch tables are `0x598b00` and `0x58ca04`; `WM_ACTIVATE` is passed only when the low word of
`wParam` is 0 and the device is not windowed, and `WM_DISPLAYCHANGE` only when not windowed.
`0x5922f0` stores both rectangles. OpenGL compares the old and new widths, then heights
(`0x58cacd`): a larger one releases and remakes the pbuffer (`0x58cafe`), else it remakes it only
when `WGL_PBUFFER_LOST_ARB` reports it lost. Direct3D's reset is `Reset` (`0x59a256`) after
`0x599900` and the present parameters, then the state initialiser `0x5a24a0`. OpenGL's event 2
destroys the context (`0x58ca79`); `0x58c140` changes the display mode with
`ChangeDisplaySettingsExA` and flag 4 or restores it with no mode; events 3 and 4 do nothing while
OpenGL makes its context, sets a format or is destroyed (`0xc0ed40`). Direct3D's `0x599d50` marks the hardware cursor changed and, full
screen, calls `ClipCursor` with the window rectangle; events 2 and 4 pass it 0, which does nothing.

## Frame counters

The base present adds 1 to the frame count `+0x74`, stores the frame rate at `+0x70`, clears the
capture request and moves the frame's fifteen counters `+0x70` to `+0xa8` into `+0x34` to
`+0x6c`, zeroing all but the frame count and the texture count and bytes (`+0x84`, `+0x88`). The
rate is `__ftol(f * 64.0 / s)`, where `s` is the sum of the last 64 steps of the time-stamp counter
between presents and `f` its ticks per second, and 99 when `s` is at most 1024. `f` is measured once:
the counter's advance over `Sleep(250)`, started at a `GetTickCount` change, times 4.

Addresses: `0x592430`, `0x592433`, `0x59243d`, `0x592440`, `0x59244a`, `0x592460`, `0x595d80`, `0x59248f`, `0x5924a5`, `0x5924cc`, `0x5924d9`, `0x5924e2`, `0x5924f7`, `0x592504`, `0x59250e`, `0x595cc0`, `0x595cf0`, `0x595d45`, `0x595d6e`, `0x5924c8`, `0x808de4`
Confidence: verified
Proof: one
Test: none

`0x595d80` is `rdtsc`. `0x59248f` stores the step from the last stamp (`0xc0ed48`) in the ring
`0xc0ed58` at index `0xc0ed50`, which wraps at 64 (`0x592504`); `0x5924a5` sums the 64 qwords and
`0x5924cc` compares the sum with 0x400. `0x5924d9` multiplies `f` by 64.0 (`0x808de4`) and
`0x5924e2` divides by the sum loaded as a 64-bit integer. The new stamp waits at `0xc0ef58` until
`0x59250e` makes it the last. The stamps and the ring lie in `.data`
past its file bytes and only `0x592460` writes them, so the first step is the whole counter value
and the rate reads far low until it leaves the ring 64 presents later. `0x595cf0` keeps `f` as a
float at `0xc0f228` and returns that float once it is non-zero; `0x595d45` is the `Sleep`. The
measuring call stores with `fst` (`0x595d6e`) and returns the integer `fild` loaded, unrounded,
so the first rate uses the unrounded `f` and later rates the float. A sum that is negative as a
signed 64-bit integer also gives 99 (`0x5924c8`).

## Shader files

A shader file starts with a magic dword, `SVXG` for vertex shaders and `SPXG` for pixel shaders,
the version 0x10003, one dword the loader skips, and a table of file offsets: 6 for vertex and 12
for pixel shaders, one per program level. The device takes the offset at its level for the target
(`+0x2d0` vertex, `+0x2d4` pixel), seeks there and has each shader read its code, then marks it
loaded (`+0x2c` = 1) and clears `+0x34`. A negative level, a missing file, a wrong magic or
version, or an offset of 0 loads nothing.

Addresses: `0x595130`, `0x59513d`, `0x595154`, `0x595177`, `0x59519c`, `0x5951c1`, `0x5951e6`, `0x5951f7`, `0x595211`, `0x809d20`, `0x809d28`, `0x809d90`, `0x598320`, `0x598180`, `0x5990a7`, `0x58e6b1`, `0x58e6c7`, `0x5a02ff`
Confidence: verified
Proof: one
Test: none

`0x59513d` tests the level; `0x595154` opens the file with `SFile::Open`. `0x595177` compares the
first dword with `0x809d20[target]` (0x47585653, 0x47585053) and `0x59519c` the second with
`0x809d28[target]`; the table is `0x809d90[target]` dwords (6, 12). `0x5951e6` picks the entry at the
level and `0x5951f7` seeks to it (`0x648a60`); `0x595211` calls slot 0 of each shader with the file,
in array order. Slot 0 (`0x598320`) reads a record: two parameter lists (`0x598180`), the format tag
at `+0x50`, the code size, the code and padding to 4 bytes. The tag is 2 in a Direct3D set, which
the creation requires (`0x5a02ff`), and 0 in an ARB set. The vertex level is 1 on Direct3D with
vertex shader 2.0 or later (`0x5990a7`; 1.1 gives 0), 2 on OpenGL with ARB vertex programs
(`0x58e6b1`), else -1 (`0x58e6c7`). The file closes either way. Its callers are `LoadShaders` and
`ReloadShader`, which pass the target and the file name, as the weather, liquid, map object, effect,
model and interface code do.
