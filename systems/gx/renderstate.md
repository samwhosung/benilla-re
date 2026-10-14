# gx: render state

The device keeps 70 render states, ids `0` to `0x45`, in a cache of 24-byte entries at `[device+0x2824]` and applies the changed ones lazily through virtual slot `+4`, `ApplyRenderState`: `0x5a2f00` for Direct3D, `0x59d350` for OpenGL. Callers reach it through the facade `0x589e60` and its typed siblings. Both backends read the same ids with the same meaning; this doc gives the ids once and the points where the backends differ.

Roots: `0x589e60`, `0x589e00`, `0x589e30`, `0x589e80`, `0x589e90`, `0x589f40`, `0x589f50`, `0x589f60`, `0x589f70`, `0x594210`, `0x593620`

## Cache, dirty list and stack

An entry holds four value dwords (`+0` to `+0xc`), a stack generation (`+0x10`) and a dirty flag (`+0x14`). The setters (`0x593710` integers, `0x593770` floats, `0x5937b0`, `0x5937e0` and `0x593840` other values) compare with the cached value and on a change call `0x594010` and store. `0x594010` appends the id to the dirty list (`[dev+0x2c]`, count `[dev+0x28]`) once, and when the entry's generation differs from the device's (`[dev+0x18]`) pushes the old value as an undo record. Setting id 7 also sets id 8 to `0x85ad20[mode]`.

Addresses: `0x593710`, `0x593770`, `0x5937b0`, `0x5937e0`, `0x593840`, `0x594010`, `0x594210`, `0x594150`, `0x5941f0`, `0x593950`, `0x593a50`, `0x593be0`, `0x85ad20`, `0x589e00`, `0x589e30`, `0x589e60`, `0x589e80`, `0x589e90`, `0x589ea0`, `0x589ec0`, `0x589ee0`, `0x589f00`, `0x589f20`, `0x589f40`, `0x589f50`, `0x589f70`, `0x5936f0`, `0x5938a0`, `0x5938c0`, `0x5938e0`, `0x593900`, `0x593930`
Confidence: verified
Proof: one
Test: none

`0x594210(all)` flushes: with `all` set it first makes every id compare unequal (`0x5941f0` stores the complement of each value in the applied copy at `[dev+0x2830]`); it then pops the dirty list and calls slot `+4` for each id whose four dwords differ from the applied copy, copies the value over and clears the flag. `0x593950` pushes the current undo count as a mark and raises the generation; `0x593a50` pops the last mark and restores each newer undo record, marking it dirty. The alpha table is `0, 224, 1, 1, 1, 1, 1, 0, 0, 0, 0` for blend modes 0 to 10, so mode 1 gives reference 224.

## Constructor defaults

The device constructor `0x5916e0` calls `0x593bf0`, which zeroes the 70 entries and sets: id 1 `0xffffffff`; id 11 `1.0`; id 13 `0xffff0080`; ids 14, 15, 16, 18, 20 and 22 `1`; id 19 `0xf`; ids `0x1f` to `0x26` `1`; ids `0x41` and `0x44` `1.0`; id `0x42` the three floats at `0xc0ef70`. Every other id stays 0, so lighting, fog, depth test, depth write and culling start on, blending off and every stage on preset 1.

Addresses: `0x593bf0`, `0x5916e0`, `0xc0ef70`
Confidence: verified
Proof: one
Test: none

The stride is 24 bytes: id 13 is the entry at `+0x138`, written through `0x596560(13)` with `0xffff0080` at `0x593cfb`; the loop at `0x593d80` writes stage `s` of the five stage ids (`0x17`, `0x1f`, `0x27`, `0x2f`, `0x37` plus `s`) per step. `0x593cfb` is the only place the value `0xffff0080` is stored.

## Reset defaults

`0x593e60`, facade `0x589f60`, sets states through the setters: ids 0, 2, 3, 5, 6, 7, 9, 10, 12, 17, 21, `0x43` and `0x45` to 0, id 1 to `0xffffffff`, ids 11, `0x41` and `0x44` to `1.0`, id 13 to `0xffff0000`, ids 14, 15, 16 and 20 to 1, id `0x42` to (1, 0, 0), and ids `0x3f` and `0x40` to no object. For each stage `s` it sets the texture (id `0x17+s`) to none, the preset (id `0x1f+s`) to 1, the mip bias to 0 and the texture coordinate mode to 0. It leaves ids 4, 18, 19 and 22 unchanged. The only caller is code of `ui`.

Addresses: `0x593e60`, `0x589f60`, `0x593840`
Confidence: verified
Proof: one
Test: none

Id 7 set to 0 also sets id 8 to 0 through the alpha cascade. The `1.0` of ids `0x41` and `0x44` is pushed as `0x3f800000` at `0x593fad` and `0x593fe8`.

## Capability bits

The word `[dev+0x269c]` starts at `0x3ff` in `0x5916e0` and changes only through `0x593620(bit, on)`, which sets bit `bit` to `on & 1` and re-queues one state: bit 0 id 14, 1 id 15, 2 id 16, 3 id 18, 4 id 19, 5 id 20. Each backend applies a gated state as off when its bit is clear: lighting (id 14, bit 0), fog (15, bit 1), depth test (16, bit 2), depth write (18, bit 3), colour mask (19, bit 4) and culling (20, bit 5).

Addresses: `0x593620`, `0x5916e0`, `0x5a317e`, `0x5a320c`, `0x59d749`, `0x59d7d8`
Confidence: verified
Proof: one
Test: none

`0x593620` writes the word at `0x593647`; `0x5916e0` at `0x591b76` is the only other writer.

## Render state ids

Both applicators read the id and act as below; a state the table gives to the cache only is applied by a later flush.

| ids | state |
|---|---|
| 0 | depth bias value (`Depth bias`) |
| 1, 2, 3, 4 | material: ambient and diffuse colour, emissive colour, specular colour, specular power |
| 5 | normalise normals |
| 6 | global ambient colour |
| 7, 8 | blend mode 0 to 10; alpha reference |
| 9 to 13 | fog mode (0 linear, 1 exp, 2 exp2), start, end, density, colour |
| 14 to 17 | lighting on; fog on; depth test on; depth function 0 to 3 |
| 18, 19 | depth write; colour write mask (4 bits) |
| 20 to 22 | cull on; clip plane mask (6 bits); multisample on |
| `0x17` to `0x1e` | texture of stage `id - 0x17` |
| `0x1f` to `0x26` | combine preset of stage `id - 0x1f` |
| `0x27` to `0x2e` | mip bias of the stage (float) |

Addresses: `0x5a2f00`, `0x59d350`, `0x5a2570`, `0x59baa0`, `0x5a326e`, `0x5a3283`, `0x5a32ad`, `0x59d84f`, `0x59d866`, `0x59d894`, `0x59d0e0`, `0x5a2d90`
Confidence: verified
Proof: one
Test: none

The jump tables are `0x5a34dc` and `0x5a3460` (Direct3D), `0x59da24` and `0x59d998` (OpenGL). `0x5a2570(index, value)` is the Direct3D shadow cache: it skips an unchanged value in `[dev+0x3a6c + 4*index]` and issues `SetRenderState` (slot `+0xe4`), `SetSamplerState` (`+0x114`) or `SetTextureStageState` (`+0x10c`) by index range; `0x59baa0(index, value, force)` is the same cache on `[dev+0x3894]` for OpenGL.

## Render state ids, continued

| ids | state |
|---|---|
| `0x2f` to `0x36` | texture coordinate mode 0 to 6 of the stage |
| `0x37` to `0x3e` | nothing: both applicators return |
| `0x3f`, `0x40` | pixel program object; vertex program object |
| `0x41` | point size |
| `0x42` | point attenuation, three floats |
| `0x43`, `0x44` | point size minimum and maximum |
| `0x45` | point sprites on |

Addresses: `0x5a2f00`, `0x59d350`, `0x5a2b40`, `0x59d120`, `0x5a0570`, `0x59e310`
Confidence: verified
Proof: one
Test: none

Ids `0x2f` to `0x36` call `0x5a2b40(stage, mode)` and `0x59d120`, ids `0x3f` and `0x40` the shader binders, ids `0x41` to `0x45` the point calls; ids `0x37` to `0x3e` reach the common return `0x5a3458` and `0x59d98d`.

## Programs

Direct3D sets the pixel shader (device slot `+0x1ac`) for id `0x3f` and the vertex shader (`+0x170`) for id `0x40`, creating the shader from the object's code on first use (`0x5a0250`, `0x5a02e0`) and calling `0x5955f0(object, 1)` after it. OpenGL binds the program targets `0x8804` (fragment) and `0x8620` (vertex) through `0x59e310` and `0x59e480`, after building the program on first use (`0x59e530`, `0x59e700`). A null object unbinds.

Addresses: `0x5a0570`, `0x5a05d0`, `0x59e310`, `0x59e480`, `0x5955f0`
Confidence: verified
Proof: one
Test: none

The slot numbers are the `IDirect3DDevice9` vtable offsets of `SetPixelShader` (index 107) and `SetVertexShader` (index 92). The object's `+0x20` holds the created shader and `+0x2c` its code.

## OpenGL fragment program build

`0x59e530(program)` builds an OpenGL fragment program from a program object's text (`+0x5c`, length `+0x58`) when the device's program level `[dev+0x2d4]` is 10: the text must begin with `!!ARBfp1.0`; the function splices `\nOPTION ARB_fog_linear;\n`, `ARB_fog_exp` or `ARB_fog_exp2` after it when the fog state (id 15) is on, by the fog mode (id 9), then generates a program name, binds target `0x8804` and passes the text with `glProgramStringARB` (format `0x8875`). The program counts as loaded (`+0x2c` = 1) when `GL_PROGRAM_ERROR_POSITION` (`0x864b`) is -1 and `GL_PROGRAM_UNDER_NATIVE_LIMITS` (`0x88b6`) is non-zero. Levels 5 to 8 set the name field and the loaded flag to 1 without a program.

Addresses: `0x59e530`, `0x85c4bc`, `0x85c4a0`, `0x85c488`, `0x85c470`, `0x59e700`
Confidence: verified
Proof: one
Test: none

The fog options are chosen by `0x5938a0(9)` and `0x5938a0(15)` through the table of three string pointers at `[ebp-0x20]`. The vertex counterpart is `0x59e700`.

## Point state

Direct3D maps id `0x41` to `D3DRS_POINTSIZE` (154), divided by a term from the viewport when id `0x42` is not (1, 0, 0); id `0x42` to `POINTSCALE_A`, `_B` and `_C` (`0x9e` to `0xa0`), with `POINTSCALEENABLE` (`0x9d`) set when the triple differs from (1, 0, 0); ids `0x43` and `0x44` to `POINTSIZE_MIN` and `_MAX` (`0x9b`, `0xa6`); id `0x45` to `POINTSPRITEENABLE` (`0x9c`). OpenGL passes id `0x41` to `glPointSize`, ids `0x42` to `0x44` to the point parameter call under the extension flag `[0xc0f2a4]`, and id `0x45` to `0x8861` and `0x8642`.

Addresses: `0x5a32e7`, `0x5a3365`, `0x5a33f2`, `0x5a3416`, `0x5a343a`, `0x59d8d4`, `0x59d8e9`, `0x59d951`, `0xc0ef70`
Confidence: verified
Proof: one
Test: none

The default (1, 0, 0) is written by an initializer at `0x590f70` to `0xc0ef70`; `0x5a32e7` compares id `0x42`'s cached triple with it. The viewport term of `0x5a32e7` is `(1 - [dev+0xf40]) * h - (1 - [dev+0xf44]) * h` for the height `h` at `+8` of the block `0x5922e0` returns.

## Texture coordinate modes

Ids `0x2f` to `0x36` give a stage a coordinate mode 0 to 6. Direct3D (`0x5a2b40`) sets the stage's `TEXCOORDINDEX` to the stage number or-ed with a generator: mode 0 none; modes 1, 2 and 3 `0x20000` (camera-space position); mode 4 `0x30000` (reflection vector); mode 5 `0x10000` (camera-space normal); mode 6 `0x30000`. The texture matrix is the identity for modes 0, 3, 4 and 5, a copy of the top view matrix with its 3 by 3 part transposed and its translation row negated for modes 1 and 2 (mode 1 multiplies it further), and for mode 6 a fixed matrix with 0.5 entries. A stage at or above the unit count `[dev+0x23c]` is ignored.

Addresses: `0x5a2b40`, `0x5a2d6c`, `0x5a2c6e`
Confidence: verified
Proof: one
Test: none

The Direct3D jump table `0x5a2d6c` lists the handlers `0x5a2b6b`, `0x5a2c4f` for modes 1 to 3, `0x5a2b92`, `0x5a2c60` and `0x5a2b92`; the matrix code is `0x5a2c6e` to `0x5a2d34`.

## OpenGL coordinate modes

OpenGL (`0x59d120`) sets generation per stage: mode 0 off; mode 1 `GL_OBJECT_LINEAR` (`0x2401`) on S, T, R; modes 2 and 3 `GL_EYE_LINEAR` (`0x2400`); mode 4 `0x8512`; mode 5 `0x8511`; mode 6 follows `[dev+0x39dc]`. Only mode 2 loads the top view matrix, flipped through `0x5a0000`; the other modes use the identity (`0x591370`).

Addresses: `0x59d120`, `0x59d334`, `0x5a0000`, `0x591370`
Confidence: verified
Proof: one
Test: none

The jump table `0x59d334` sends mode 0 to `0x59d156`, 1 to `0x59d1de`, 2 and 3 to `0x59d1ae`, 4 to `0x59d21e`, 5 to `0x59d25e` and 6 to `0x59d17d`.

## Blend, alpha and depth tables

Id 7 with mode 0 or 1 disables blending; modes 2 to 10 enable it with source and destination factors. Direct3D uses `D3DBLEND` tables `0x80a1fc` (`2, 2, 5, 5, 9, 9, 9, 6, 6, 5, 2`) and `0x80a228` (`1, 1, 6, 2, 1, 3, 2, 2, 1, 1, 2`); OpenGL the enum tables `0x85c1f8` and `0x85c224`. Id 8 above 0 sets the alpha test with reference `ref` (Direct3D `ALPHAREF = ref`, test always `GREATEREQUAL`; OpenGL `glAlphaFunc(GL_GEQUAL, ref / 255)`), and 0 or less turns it off. Id 17 picks the depth function from `0x80a298` (`4, 3, 7, 2`: lessequal, equal, greaterequal, less) or `0x85c364` (`LEQUAL, EQUAL, GEQUAL, LESS`).

Addresses: `0x80a1fc`, `0x80a228`, `0x85c1f8`, `0x85c224`, `0x80a298`, `0x85c364`, `0x5a3057`, `0x5a30a8`, `0x5a317e`, `0x59d563`, `0x59d5b9`, `0x59d77f`
Confidence: verified
Proof: one
Test: none

The mode pairs, source over destination: 2 `SRC_ALPHA/INV_SRC_ALPHA`, 3 `SRC_ALPHA/ONE`, 4 `DST_COLOR/ZERO`, 5 `DST_COLOR/SRC_COLOR`, 6 `DST_COLOR/ONE`, 7 `INV_SRC_ALPHA/ONE`, 8 `INV_SRC_ALPHA/ZERO`, 9 `SRC_ALPHA/ZERO`, 10 `ONE/ONE`. Direct3D has no depth-test enable: id 16 off sets `ZFUNC` to always (8) through `0x5a317e`, and `ZENABLE` is 1 from `0x5a24a0`. OpenGL toggles `GL_DEPTH_TEST` (`0xb71`) for id 16 and calls `glDepthFunc` for id 17.

## Cull, fog and masks

Id 20 with its capability bit set and a non-zero value culls; Direct3D sets `CULLMODE` to `D3DCULL_CW` (2) for it and `D3DCULL_NONE` (1) otherwise (`0x80a2a8`); OpenGL toggles `GL_CULL_FACE` (`0xb44`). The face to cull is the separate OpenGL state `0x4e`, `glCullFace`, set by other code. Id 9 picks the fog mode from `0x80a28c` (`3, 1, 2`: linear, exp, exp2) or `0x85c358` (`GL_LINEAR`, `GL_EXP`, `GL_EXP2`); ids 10 to 13 are start, end, density and colour. The colour mask of id 19 is passed whole to `COLORWRITEENABLE` (`0xa8`) or split into four `glColorMask` booleans.

Addresses: `0x5a320c`, `0x80a2a8`, `0x59d7d8`, `0x59bfd9`, `0x5a30da`, `0x80a28c`, `0x85c358`, `0x5a31f0`, `0x59d7b8`, `0x59bafe`
Confidence: verified
Proof: one
Test: none

`glFrontFace` is not in the import table, so OpenGL keeps counter-clockwise front faces. Clip planes (id 21) are a six-bit mask: OpenGL enables `GL_CLIP_PLANE0` plus the bit number for each bit that changed (`0x59bfe1`), Direct3D passes the mask to `CLIPPLANEENABLE` (`0x98`).

## Depth bias

Id 0 is a float. Direct3D writes `D3DRS_DEPTHBIAS` (`0xc3`) as `-value` when the device flag `[dev+0x2ec]` is set, directly, with no slope term; `SLOPESCALEDEPTHBIAS` is never written. OpenGL, for a non-zero value, enables `GL_POLYGON_OFFSET_FILL` (`0x8037`) and calls `glPolygonOffset(-4.0, -value * 65536)`; for zero it disables it.

Addresses: `0x5a2f30`, `0x59d383`, `0x59bef7`, `0x59bf0a`, `0x59baa0`
Confidence: verified
Proof: one
Test: none

The `-4.0` is the literal `0xc0800000` pushed at `0x59bf0a`; the product is `value * 65536.0 * -1.0` (`0x80a000`, `0x8030a0`). One OpenGL unit is the smallest depth step, so on a 24-bit buffer the OpenGL bias is `value / 256` of the Direct3D one for the same `value`; the buffer depth is not read here.

## Texture stage combine

A stage's combine preset (ids `0x1f` to `0x26`, values 0 to 5) selects Direct3D operations from `0x80a25c` (colour: `4, 4, 13, 7, 5, 12`) and `0x80a274` (alpha: `3, 4, 3, 7, 5, 3`), as `D3DTOP` values, or an OpenGL record of the table `0x85c250` (six records of `0x2c` bytes). OpenGL sets `GL_COMBINE` with the record's scales and sources (`0x59bc79`), or only the texture environment mode where the combine extension is missing.

| preset | colour (D3D, GL) | alpha (D3D, GL) |
|---|---|---|
| 0 | modulate, modulate | arg 2, previous |
| 1 | modulate, modulate | modulate, modulate |
| 2 | blend texture alpha, interpolate | arg 2, previous |
| 3 | add, add | add, add |
| 4 | modulate 2x, modulate with scale 2 | modulate 2x, scale 2 |
| 5 | blend diffuse alpha, interpolate | arg 2, previous |

Addresses: `0x80a25c`, `0x80a274`, `0x85c250`, `0x59bc79`, `0x5a2dd0`, `0x59d0a0`
Confidence: verified
Proof: one
Test: none

The GL records read (mode, RGB scale, three RGB sources, RGB operation, alpha scale, three alpha sources, alpha operation). Preset 2's GL interpolation takes its weight from the alpha of the constant source (`0x8576`), Direct3D's `BLENDTEXTUREALPHA` (13) from the texture's alpha. Preset 5's GL interpolation takes it from the alpha of the previous stage (`0x8578`), Direct3D's `BLENDDIFFUSEALPHA` (12) from the vertex colour's: the same on stage 0. The sources are texture `0x1702`, previous `0x8578`, constant `0x8576`; the alpha operation of presets 0, 2 and 5 is `GL_REPLACE` of the previous alpha.

## Stage sampler state

Binding a texture to a stage (ids `0x17` to `0x1e`) also sets the stage's sampler from the texture's flags word (`textures.md`, Texture flags word). Direct3D (`0x5a29d0`) sets the texture, then magnification, minimisation and mip filters from the table `0x85c7d8` by the low three bits, `ADDRESSU` and `ADDRESSV` from bits 3 and 4 (`0x80a254`: clear `CLAMP` 3, set `WRAP` 1), and `MAXANISOTROPY` from bits 9 to 13. OpenGL (`0x59f170`) sets `GL_TEXTURE_MAG_FILTER` and `MIN_FILTER` from `0x80a01c` and `0x80a004`, wrap from bits 3 and 4 (`GL_REPEAT` when set, `GL_CLAMP_TO_EDGE` or `GL_CLAMP` when clear), mip generation from bit 6 and anisotropy from bits 9 to 13.

Addresses: `0x5a29d0`, `0x85c7d8`, `0x80a254`, `0x59f170`, `0x80a01c`, `0x80a004`, `0x594d90`, `0x59d020`
Confidence: verified
Proof: one
Test: none

Direct3D filters by mode (magnify, minify, mip) as `D3DTEXF`: 0 point, point, none; 1 linear, linear, none; 2 point, point, point; 3 linear, linear, point; 4 and 5 linear, linear, linear. OpenGL by mode: magnify nearest for 0 and linear for 1 to 5; minify nearest, linear, nearest-mip-nearest, linear-mip-nearest, linear-mip-linear, linear-mip-linear. Mode 2 therefore magnifies with point sampling on Direct3D and linear on OpenGL. A stage first set clears its flag at `[dev+stage+0x3bfc]` through `0x5a2dd0`, which applies the stage's preset once.
