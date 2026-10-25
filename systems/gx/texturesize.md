# gx: texture sizes and formats

How a request for a texture becomes a device texture of some size and format: the device's size limit, the format a BLP header chooses, the checks `0x448450` and `GxTexCreate` apply, the pool of free textures, and the BLP level that becomes level 0. The texture objects and their loaders are in `textures.md`; the upload of a level is there too.

Roots: `0x448450`, `0x448b60`, `0x44a560`, `0x44a790`, `0x58ab10`

## Device size limit

`0x448b60(&width, &height, &dropped)` halves the width and the height together while either exceeds the device limit, adds one to `dropped` for each halving and keeps each side at 1 or more. The limit is `caps + 0x60`: the larger of 256 and the device's largest texture side, `MaxTextureWidth` on Direct3D and `GL_MAX_TEXTURE_SIZE` on OpenGL. A limit of 0 sets Storm error `0x57` and changes nothing. A size within the limit drops nothing.

Addresses: `0x448b60`, `0x448b85`, `0x448ba7`, `0x448bab`, `0x448baf`, `0x448b8f`, `0x598f30`, `0x58e4cd`, `0x58e4ef`, `0x58a230`
Confidence: verified
Proof: one
Test: `gx::texture_limit_size_difftest`

`0x448b71` copies the 0x38-dword capability block and `0x448b85` reads its `+0x60`. `0x448ba7` and `0x448bab` leave when the width and the height are within the limit; `0x448baf` to `0x448bc7` halve both, add to the count and floor each side at 1. Direct3D stores the larger of 256 and the caps copy's `MaxTextureWidth` (D3DCAPS9 `+0x58`, the copy at `[dev+0x38ac]`) at `0x598f30`; OpenGL stores the larger of 256 and the `0xd33` query at `0x58e4ef`. The test runs the whole function over sides from 0 to `0xffffffff`, limits from 0 to 16384 and a non-zero start count.

## BLP format chooser

`0x44a790(alphaType, alphaDepth)` chooses a pixel type code and a `GxTex` format for the BLP callback `0x44a560`. It reads the header byte at reader `+0x0e` (0 to 8) and the alpha depth, never the compression byte, and the device flags `+0x40` (DXT1), `+0x44` (DXT3) and `+0x48` (DXT5) as non-zero words. A byte of 6 or above 8 stores nothing. The code goes to the first output (`ecx`) and the format to the second (`edx`). Bytes 0, 1 and 7 depend on the device:

| byte | device flag | type code | format |
|---|---|---|---|
| 0 | `+0x40` set | 0 | `Dxt1` |
| 0 | clear, depth 0 | 5 | `Rgb565` |
| 0 | clear, depth above 0 | 3 | `Argb1555` |
| 1 | `+0x44` set | 1 | `Dxt3` |
| 1 | clear | 4 | `Argb4444` |
| 7 | `+0x48` set | 7 | `Dxt5` |
| 7 | clear | 4 | `Argb4444` |

Addresses: `0x44a790`, `0x44a8d4`, `0x44a8cb`, `0x44a63b`, `0x44a64a`, `0x58a230`
Confidence: verified
Proof: one
Test: `gx::texture_format_choice_difftest`

The jump table `0x44a8d4` holds, for the bytes 0 to 8: `0x44a83a`, `0x44a883`, `0x44a804`, `0x44a7e0`, `0x44a816`, `0x44a828`, `0x44a8cb`, `0x44a8a1`, `0x44a7ac`. `0x44a8cb` pops and returns with both outputs unwritten, so the callback reads two uninitialised words. `0x4489a0` tests the compression byte first and gives `Argb8888` for anything but DXT (`textures.md`). The type codes are those of `0x835258`. The test runs the whole function over the bytes 0 to 12 and larger, eight alpha depths and all eight combinations of the three flags, and checks that a byte of 6 or above 8 leaves both outputs as they were.

## BLP fixed formats

The other bytes of `0x44a790` do not read the device flags, and always give type code 2 (`Argb8888` data). Byte 8 chooses the format from the alpha depth.

| byte | alpha depth | type code | format |
|---|---|---|---|
| 2 | any | 2 | `Argb8888` |
| 3 | any | 2 | `Argb1555` |
| 4 | any | 2 | `Argb4444` |
| 5 | any | 2 | `Rgb565` |
| 8 | 0 | 2 | `Rgb565` |
| 8 | 1 | 2 | `Argb1555` |
| 8 | 4 | 2 | `Argb4444` |
| 8 | other | 2 | `Argb8888` |

Addresses: `0x44a790`, `0x44a7ac`, `0x44a7e0`, `0x44a804`, `0x44a816`, `0x44a828`
Confidence: verified
Proof: one
Test: `gx::texture_format_choice_difftest`

`0x44a7ac` switches on the alpha depth (`0x44a7af` to `0x44a7b7`): 0 gives format 4, 1 gives 3, 4 gives 2 and any other gives 1, each with code 2. `0x44a804`, `0x44a7e0`, `0x44a816` and `0x44a828` store the formats 1, 3, 2 and 4 for the bytes 2, 3, 4 and 5.

## Texture request routes

`0x448450(width, height, format, flags, owner, callback, typeCode)` returns a device texture, or 0. A width or height that is not a power of two (0 counts as one) or is 1024 or more returns 0 at `0x448665`: no texture, no Storm error, no log. A side of 16 or less, or a `format` above 7 (a signed compare), makes a new texture through `GxTexCreate`. Otherwise both sides are 32 to 512 and the request takes the pool. `format` is the device texture's `GxTex` format; `typeCode` is the type of the data the callback supplies, mapped by `0x835258` to the data format. The flags reach the device with bit 6 cleared.

Addresses: `0x448450`, `0x4484ae`, `0x4484b9`, `0x4484c5`, `0x4484d0`, `0x4484d6`, `0x4484df`, `0x4484e8`, `0x448494`, `0x44864b`, `0x448665`, `0x58ac40`, `0x835258`
Confidence: verified
Proof: one
Test: `gx::texture_pool_route_difftest`

`0x4484ae` and `0x4484b9` test `v & (v - 1)` for the width and the height; `0x4484bf` to `0x4484d0` compare each with `0x400` (`jae`); `0x4484d6` to `0x4484e8` compare with `0x10` (`jbe`) and the first stack argument with 7 (`jg`). `0x44864b` calls `0x58ac40` with a record of the type 0, the width, the height, 0, `format`, the data format, the flags and the owner and callback, and returns what `GxTexCreate` stored. The test stubs the capability block, the pool getter and `0x58ac40`, and checks the refusals, the record's fields and its table lookup; the reuse arm from `0x448572` is not run.

## Refused requests

A caller sees the 0. The BLP callback `0x44a560` logs `Texture failure: "%s" allocating %dx%d texture failed.` (`0x835674`, severity 3, the name and the size after the drop) and returns 0 (`0x44a715` to `0x44a756`); `0x44a500` then fills the texture with the solid one (`0x44a50e`, an 8 by 8 green). The TGA loader `0x44a140` destroys its new `CTexture` (`0x44a1fc` to `0x44a210`) and returns 0, so the candidate fails and the next is tried.

Addresses: `0x44a560`, `0x44a500`, `0x44a140`, `0x44a715`, `0x44a724`, `0x44a73e`, `0x44a756`, `0x44a50e`, `0x44a1fc`, `0x44a20b`, `0x835674`
Confidence: verified
Proof: one
Test: none

`0x44a71a` tests the result and `0x44a722` jumps past the log on a texture; at `0x44a724` the log call pushes the height, the width, the name at `+0xc` and the format, then calls the status object's slot `+0xc` (`0x44a73e`). `0x44a500` tests `0x44a560`'s result at `0x44a50a` and calls `0x44a900` on a 0. `0x44a201` tests `0x448450`'s result in `0x44a140`; on 0 `0x44a210` calls the deleting destructor.

## Pool lists

A request that takes the pool reads list `log2(width / 32) * 5 + log2(height / 32)` of the 25 at `0xb05bec`, 12 bytes each with the first free texture's link at `+4`. It reuses the first free texture whose flags agree in the low three bits (the filter mode) and whose format equals `format`: the data format, the owner, the callback and the flags are set on it, it moves to the busy list and is returned; with no match a new texture is made. `0x448670` returns a texture to its class when the sizes qualify, stamped with the time (`0x42c010`), else destroys it. `0x448ec0` destroys pooled textures idle more than 30000 ms.

Addresses: `0x448450`, `0x448517`, `0x448529`, `0x448531`, `0x448555`, `0x448560`, `0x448572`, `0x448622`, `0x448670`, `0x448ec0`, `0x449010`, `0xb05bec`, `0x58ada0`, `0x58adc0`, `0x58ad00`, `0x58ad30`
Confidence: verified
Proof: one
Test: `gx::texture_pool_route_difftest`

`0x4484f1` to `0x448515` count the trailing zero bits of `width >> 5` and `height >> 5`; `0x448529` forms `log2(width / 32) * 5 + log2(height / 32)` and `0x44852e` multiplies by 12. `0x448531` loads the list's first link; a link with bit 0 set or equal to 0 ends the walk to the new arm (`0x44853f` to `0x44854a`). `0x448555` reads the candidate's description through `0x58ada0`; `0x448560` compares the flags' low three bits and `0x448568` the format. A match is unlinked and linked to the busy list, then `0x58adc0`, `0x58ad00` and `0x58ad30` store the data format, the owner with the callback and the flags. The test seeds each list with one record that never matches and checks that exactly the size class's list is read; a match is not run.

## Texture create checks

`GxTexCreate(type, w, h, arg4, format, dataFormat, flags, owner, callback, &out)` (`0x58ab10`) sets `out` to 0 and either calls the device's slot `+0xb4` or sets Storm error `0x57` and returns 0. For type 0 it refuses when the device does not report the type (`caps + 0x50` is not 1), a side is below 8 or above `caps + 0x60`, a side is not a power of two, `format` or `dataFormat` is above 9 (signed), bit 6 of the flags is set while `caps + 0x28` is 0 or `format` is 5 to 7, the filter mode is 5 while `caps + 0xa8` is 0, or the callback is null.

Addresses: `0x58ab10`, `0x58ab40`, `0x58ab4c`, `0x58ab5a`, `0x58ab69`, `0x58ab77`, `0x58aba6`, `0x58abab`, `0x58abc6`, `0x58abe1`, `0x58abe9`, `0x58abed`, `0x58ac22`, `0x58ac40`, `0x58aa30`, `0x58aa48`, `0x58aab3`
Confidence: verified
Proof: one
Test: `gx::texture_create_gate_difftest`

`0x58ab1c` refuses a type above 3 and `0x58ab35` a type whose `caps + 0x50 + 4 * type` is not 1. `0x58ab40` and `0x58ab4c` refuse a width and a height below 8; `0x58ab5a` and `0x58ab69` compare them with `caps + 0x60 + 4 * type`; `0x58ab77` to `0x58ab9e` test the power of two for types other than 2, where `0x58ab93` tests the filter mode instead. `0x58aba6` refuses a format above 9, `0x58abab` to `0x58abc4` the bit 6 cases, `0x58abc6` to `0x58abdc` the filter mode 5 case, `0x58abe1` a data format above 9 and `0x58abe9` a null callback; all jump to `0x58abed`. `0x58ac22` is the device call, ten pushes: the type, the size, the three fields from `arg4` to `dataFormat`, the flags, the owner, the callback and the out pointer. `0x58aa30` repeats the size, power-of-two, format, bit 6, filter mode 5 and callback checks for a two-dimensional request (`0x58aa48` to `0x58aab3`). The test runs type 0 only up to the device call, which it stubs, and not the type tests or the type 2 arm.

## BLP load order

`0x44a560` decides which BLP level becomes level 0 of a file texture, in this order: it parses the header (`0x5a83b0`); it halves the width and height with `0x448b60`, which leaves `dropped` at `+0x114`; it picks the type code and format with `0x44a790`; it decodes the chain from file level `dropped` with `0x5a8430`; and it asks `0x448450` for a device texture of the size after the drop. A failure at any step returns 0 and `0x44a500` substitutes the solid texture. File level `dropped` is level 0 of the chain. `dropped` starts at the constructor's 0 (`0x44820b`) and `0x448b60` adds to it, so within this function only the device limit raises it.

Addresses: `0x44a560`, `0x44a5be`, `0x44a5c7`, `0x44a62f`, `0x44a64a`, `0x44a670`, `0x44a679`, `0x44a6c0`, `0x44a715`, `0x5a8430`, `0x5a7ec0`, `0x5a4ac0`, `0x5a8220`, `0x448b60`, `0x449840`, `0x8356ac`, `0x8356dc`
Confidence: verified
Proof: one
Test: `gx::texture_limit_size_difftest`, `gx::texture_pool_route_difftest`

`0x44a5c7` logs `invalid file version` (`0x8356dc`) when the parse fails. `0x44a62f` passes the width (reader `+0x10`) and height (`+0x14`) and `&[esi+0x114]`; `0x44a6c0` and `0x44a6c6` store the sizes after the drop at `+0x144` and `+0x148`. `0x44a670` calls `0x5a8430` with the type code, the chain at `+0x120`, `dropped` and 1. `0x5a7ec0` accepts a start of 0, or a start below the level count `0x5a4ac0(w, h)` when the has-mips byte `0x5a8220` is non-zero; a BLP without mips has level 0 only, so `dropped` above 0 fails there and `0x44a679` logs `decompression failed` (`0x8356ac`). `0x449840`, the loader of `0x449610`, drops the same way. The has-mips rule and the log lines are read, not run.

## BLP size cases

For a BLP 1024 pixels on a side the outcome depends on the device limit and on the has-mips byte. The device then applies its own skip: with a mip chain kept, the texture is made at the request's sides shifted right by `first = min([dev+0x31c], levels - 1)` and the first level uploaded is chain entry `first`, file level `dropped + first`. `[dev+0x31c]` is the CVar `baseMip` (0 or 1, default 0). The refusals of `0x448450` see the size before this shift.

| device limit | with mips | without mips |
|---|---|---|
| 1024 or more | `dropped` 0; `0x448450` refuses 1024; the solid texture | the same |
| 512 | `dropped` 1; file level 1 (512 by 512) is level 0 | `0x5a7ec0` refuses level 1; the solid texture |
| 256 | `dropped` 2; file level 2 (256 by 256) is level 0 | the same |

Addresses: `0x44a560`, `0x448b60`, `0x448450`, `0x4484c5`, `0x4484d0`, `0x5a7ec0`, `0x689090`, `0x589bf0`, `0x592050`, `0x5a0880`, `0x44a6d9`
Confidence: verified
Proof: one
Test: `gx::texture_limit_size_difftest`, `gx::texture_pool_route_difftest`

`0x689090` passes the CVar's value (0 or 1) to `0x589bf0`, which calls slot `+0x2c` (`0x592050`); `0x5a0880` applies `[dev+0x31c]` (`textures.md`, Device texture upload). A BLP of 512 or less on a side, a power of two, drops nothing on any device. Which filter bits a widget texture carries decides whether a chain is kept and is not settled here. `0x44a6d9` sets bit 6 of the flags word at `+0x150` when the format is not 5 to 7, `caps + 0x28` is set and the has-mips byte is 1; `0x448450` clears bit 6 before the device sees the flags.
