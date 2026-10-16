# gx

The graphics device layer. One global device, `CGxDevice`, has two backends, Direct3D 9 and OpenGL; the client picks one at start-up and runs Direct3D unless the CVar `gxApi` is `OpenGl` or `-opengl` is given, and `-d3d` overrides both (`device.md`). gx also holds the texture objects and their cache (`textures.md`), the render states (`renderstate.md`), drawing, vertex formats and lights (`draw.md`), every device vtable slot (`slots.md`), the OpenGL context and extensions (`opengl.md`), screen capture (`capture.md`), the lightning bolt renderer (`lightning.md`) and glyph bitmaps (`glyphs.md`). It owns the device vtables, the render state tables and the static state of these files. The rest of the client reaches it through the `Gx*` facade at `0x589ca0` through `0x58b740`, the texture calls, the lightning calls and `0x5d1120`.

Roots: `0x589ac0`, `0x58a810`, `0x449d90`, `0x449610`, `0x47ae30`, `0x7b07f0`, `0x5d1120`, `0x589e60`

## Object files

gx is six object files. `Texture.cpp`: `0x4477d0` through `0x44bfa0`, with its container helpers `0x44c040` through `0x44ca10` (source path `0x8354e4`, initializer slots `0x8274e0` to `0x827510`). `TextureCache.cpp`: `0x47a9a0` through `0x47bec0` (path `0x838efc`, slots `0x827d5c` to `0x827d68`). The device layer: `0x5894c0` through `0x5a35d0`, the facade and the files `CGxDevice.cpp`, `CGxDeviceD3d.cpp`, `CGxDeviceOpenGl.cpp`, `CGxD3dDevice.cpp` and `CGxD3dTexture.cpp` (paths `0x85aea4`, `0x85a860`, `0x85aaf8`, `0x85b260`, `0x85c718`; slots `0x82a5c4` to `0x82a618`). `IGxuFontGlyph.cpp`: `0x5d1100` through `0x5d1400` (path `0x85f8c0`, slots `0x82aaa0` to `0x82aaa8`). `Lightning.cpp`: `0x7af5c0` through `0x7b0d90` (path `0x87d6dc`, slots `0x82d9a0` to `0x82d9ac`).

Addresses: `0x4477d0`, `0x44bfa0`, `0x44c040`, `0x44ca10`, `0x47a9a0`, `0x47bec0`, `0x5894c0`, `0x5a35d0`, `0x5d1100`, `0x5d1400`, `0x7af5c0`, `0x7b0d90`
Confidence: verified
Proof: one
Test: none

The extents come from the initializer table `0x827000` and the source paths. Each file's run opens with its +∞ initializer (`0x4477d0`, `0x47a9a0`, `0x5899d0`, `0x5d1100`, `0x7af990`) and holds the 2π and 1/2π copies (`0x44aea0`, `0x44af60`; `0x47be90`, `0x47beb0`; `0x58b650`, `0x58b670`; `0x5d13d0`, `0x5d13f0`; `0x7b0c10`, `0x7b0c30`). The device run also holds the vertex descriptor initializers `0x5894c0` through `0x590fc0`, so the file's code starts at `0x5894c0`, after the last initializer of the preceding file (`0x5894a0`). The helpers of `0x44c040` through `0x44ca10` are called only from `Texture.cpp` code. The device paths are in `0x58ba78` (D3d constructor allocation), `0x58dd78` (OpenGl), `0x5944ad` (CGxDevice.cpp), `0x59a638` and `0x5a0d04`.

## Unreached code

Ninety-two gx functions have no caller, no stored address and no live caller: unused facade wrappers, unused pool, buffer and device slots, and unused texture helpers. Their rows are of class `unreached` and owe no test.

Addresses: `0x448ea0`, `0x4490c0`, `0x4499d0`, `0x449a10`, `0x449aa0`, `0x449b30`, `0x44ae50`, `0x44b3a0`, `0x44b4f0`, `0x44b610`, `0x44b6c0`, `0x44b960`, `0x44c7d0`, `0x44c810`, `0x44c830`, `0x47adf0`, `0x47b0c0`, `0x47b0e0`, `0x589b20`, `0x589b80`, `0x589c30`, `0x589c40`, `0x589c60`, `0x589c80`, `0x589d10`, `0x589dc0`, `0x589e90`, `0x589ea0`, `0x589ec0`, `0x589f00`, `0x589f20`, `0x589fc0`, `0x589fe0`, `0x58a120`, `0x58a180`, `0x58a780`, `0x58a840`, `0x58a850`, `0x58a860`, `0x58a880`, `0x58a8a0`, `0x58a8c0`, `0x58a8e0`, `0x58a900`, `0x58aaf0`, `0x58acc0`, `0x58ad80`, `0x58aef0`, `0x58b290`, `0x58b370`, `0x58b390`, `0x58b410`, `0x58b420`, `0x58b510`, `0x58b550`, `0x58b560`, `0x58b580`, `0x58b610`, `0x58daa0`, `0x58e410`, `0x58e440`, `0x58e470`, `0x58e9d0`, `0x58f620`, `0x591360`, `0x5920b0`, `0x5920d0`, `0x592110`, `0x592920`, `0x5933c0`, `0x5938c0`, `0x5938e0`, `0x593900`, `0x593930`, `0x594c70`, `0x594e30`, `0x594e40`, `0x594f30`, `0x595480`, `0x595c10`, `0x595c90`, `0x597e10`, `0x597ef0`, `0x597fc0`, `0x598000`, `0x598030`, `0x598060`, `0x59aa20`, `0x59ea70`, `0x7b09e0`, `0x7b0a00`, `0x7b0a20`
Confidence: verified
Proof: one
Test: none

The census is `cargo xtask census`, confirmed by a scan for every `call` and `jmp` target and every little-endian occurrence of the address in the image at any alignment, then closed over callers (a function called only by unreached code is unreached). Its control is the live facade `0x589ac0`, which the scan finds with its caller `0x63a230`.
