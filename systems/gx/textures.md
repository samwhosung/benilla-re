# gx: textures

How the client turns a texture name into a device texture: `Texture.cpp` (`0x4477d0` through `0x44bfa0`, with its container helpers `0x44c040` through `0x44ca10`) and the named-texture cache in `TextureCache.cpp` (`0x47a9a0` through `0x47bea0`). A texture is an `HTEXTURE` handle on a `CTexture` object; the device texture under it is a `GxTex`, reached through the device layer (`device.md`). File bytes arrive through the client's async load handles; BLP and TGA decoding belong to `image`.

Roots: `0x449610`, `0x449d90`, `0x449b30`, `0x449bf0`, `0x449cc0`, `0x44a9c0`, `0x44acf0`, `0x44ae00`, `0x44b3b0`, `0x44b430`, `0x449ac0`, `0x449ad0`, `0x47ae30`, `0x47b010`

## Texture objects

A `CTexture` is `0x15c` bytes, allocated under the tag `HTEXTURE` (`0x835604`) and wrapped by `CreateHandle`. The constructor `0x448190` stores the vtable `0x8026b4` (slot 0 the deleting destructor `0x4482b0`, slot 1 `0x4482a0`, which returns `this + 0xc`). Fields: `+0xc` name (260 bytes), `+0x110` flags (bit 0 set when the image has no alpha, bit 1 set when the caller's file flag was non-zero), `+0x114` mip levels dropped, `+0x118` pixel type code (below), `+0x11c` alpha bits, `+0x120` decoded mip buffer, `+0x138` async load record, `+0x140` device texture, `+0x144`, `+0x148` width and height after the drop, `+0x14c` mip count, `+0x150` the `GxTex` flags word.

Addresses: `0x448190`, `0x4482a0`, `0x4482b0`, `0x4482e0`, `0x41af10`
Confidence: verified
Proof: one
Test: none

`0x4482e0` releases in order: the hash entry at `+8`, the device texture at `+0x140` through `0x448670`, the decoded buffer through `0x44b110`, the async record at `+0x138` (its file closed, `0xb05d20` or `0xb05d1c` decremented by pinned state, then `0x4439b0`), and the sub-object at `+0x124`. Every creator below ends with `CreateHandle(object, 0x835604)`.

## Texture creation

`0x449d90(name, log, filterIn, force, fileFlag)` returns a handle; `log` is a status object (vtable slot `+0xc` takes severity, format, arguments). A null or empty name, or a null `log`, sets Storm error 0x57 and returns 0. Otherwise it splits the name (`Texture name split`), builds the flags word (`Texture flags word`) and looks the stripped name and flags up in the hash; a hit is returned through `0x41af10`, which adds one to the handle's count at `+4`: the caller owns a new reference. A miss tries the candidate files (`Texture name candidates`), caches the loaded texture under the stripped name and flags, and returns it. When every candidate fails it tells `log` and returns the solid texture (`Texture load failure`).

Addresses: `0x449d90`, `0x449eb0`, `0x449fb0`, `0x44a9c0`, `0x44aa20`, `0x44ab30`, `0x44a900`, `0x90582c`, `0x449db1`, `0x449dec`, `0x449df3`, `0x449e99`, `0x44a260`
Confidence: verified
Proof: one
Test: none

`0x449eb0` hashes the stripped name (`SStrHash`, `0x64af90`, called with flag argument 0, so it folds case and maps `/` to `\`) with mask `[0x905bc4]` and matches the stored hash, the flags at `+0x18` and the name at `+0x14` (`SStrCmpI`, at most 0x104 bytes, case-insensitive and slash-literal). `0x449dec` calls it with the stripped name in the stack buffer and `0x449df3` returns a hit. `0x449e99` inserts only on the success path, through `0x449fb0`; the failure path at `0x449e65` never reaches it, so a name that does not resolve runs both opens again on every call. `0x44a9c0(colour ptr)` first asks `0x44aa20` for a texture cached under the name `Solid Texture` (`0x835708`) and flags of that colour; on a miss `0x44a900` fills an 8 by 8 texture through `0x448450` with the callback `0x5c5350` (it fills a 64-dword buffer with the colour), bit 0 of the flags set when the colour's alpha byte is `0xfe` or more, and `0x44ab30` caches it. `0x44a0e0` builds its message from `0x64df50`. `0x44a140` loads a TGA through the pixel callback `0x44a260`.

## Texture name split

`0x449590(name, buffer, &split)` copies the name into the 0x104-byte buffer (`SStrCopy`, at most 0x103 bytes) and sets `split` to the dot when the byte four from the end is `.`, else to the end of the copy; a copy shorter than four bytes has no dot. At a dot it lowercases the three bytes after it (`0x64b480`, `_strlwr`), writes 0 over the dot, and returns 0 when they are `tga` and 1 for every other three bytes, `blp` included. With no dot at that place it returns 1 and strips nothing: `Foo.jpeg` keeps its extension, `Foo` has none. The extension is not checked beyond its length: `Foo.bmp` becomes `Foo`.

Addresses: `0x449590`, `0x4495a3`, `0x4495a8`, `0x4495ad`, `0x4495b4`, `0x4495be`, `0x4495c5`, `0x4495ea`, `0x449603`, `0x64a5a0`, `0x64b480`
Confidence: verified
Proof: one
Test: none

`0x4495a8` compares the copy's length with 4 (`jb` skips the dot test), `0x4495ad` tests `[len - 4] == 0x2e` and `0x4495b4` backs the pointer up by four. `0x4495be` tests the byte at the split point; anything but `.` returns 1 at `0x449606`. `0x4495cf` to `0x4495fb` compare the lowered bytes: the `tga` path reaches `0x449601` and returns 0, the `blp` path stores 0 at `0x4495ea` and returns 1, and every other case jumps to `0x449603`, which stores 0 over the dot with the return value still 1. The lowering touches the buffer, never the caller's string.

## Texture name candidates

`0x449d90` appends `.tga` (index 0) or `.blp` (index 1) at the split point from the table `0x835248` and loads `<buffer>` through `0x44a140` (TGA) or `0x44a310` (BLP), starting at the index `0x449590` returned and trying the other once. A name with no three-byte extension, or `.blp` or any other extension, goes `.blp` first; only `.tga` goes `.tga` first. A kept long extension stays in front of the candidate. The base name and its slashes reach Storm as written; only the stripped three bytes were lowered, and they are gone. Both loaders return 0 without writing to `log` when the open fails.

| name | first | second |
|---|---|---|
| `X`, `X.blp`, `X.bmp` | `X.blp` | `X.tga` |
| `X.tga` | `X.tga` | `X.blp` |
| `X.jpeg` | `X.jpeg.blp` | `X.jpeg.tga` |

Addresses: `0x449d90`, `0x449e00`, `0x449e11`, `0x449e2b`, `0x449e3d`, `0x449e4b`, `0x449e5d`, `0x44a140`, `0x44a169`, `0x44a310`, `0x44a399`, `0x835248`, `0x64a5a0`
Confidence: verified
Proof: one
Test: none

`0x449e00` loads `[4*edi + 0x835248]`, `0x449e11` copies it over the split point with no length limit, and `0x449e16` to `0x449e3d` choose `0x44a310` for 1 and `0x44a140` for 0. `0x449e4b` flips `edi` and `0x449e5d` ends the loop after two passes. The copy at `0x449e11` is unbounded and the buffer is 0x104 bytes at `[ebp-0x110]`: a name of 0x100 bytes or more (the copy of the name keeps 0x103 bytes) with no dot four from the end writes the candidate's 5 bytes up to index `len + 4`, past the buffer, over the saved name pointer at `[ebp-0xc]` (the low byte first), which `0x449e65` reads for the message. `0x44a169` and `0x44a399` return 0 on a failed open with no status write.

## Texture load failure

When both candidates fail, `0x449d90` calls `0x44a0e0` with `log`, the word `texture` and the caller's original name, and returns the solid texture of colour `[0x90582c]`: an 8 by 8 texture of format 2 (`0x44a900`), the colour `0xff00ff00`. `[0x90582c]` is zero until the static initializer `0x447bf0` calls `0x447c00`, which stores `0xff00ff00`. `0x44a0e0` calls `log` slot `+0xc` with severity 3 and the format `Error loading %s file "%s": %s\n`, the last Storm error text as the third argument, then clears the Storm error with `SErrSetLastError(0)` (`0x44a124`). `CStatus::Add` raises the status's highest severity to 3. The name in the message is the argument as the caller wrote it, extension included, not the stripped name.

Addresses: `0x449e65`, `0x449e70`, `0x449e77`, `0x44a0e0`, `0x44a11c`, `0x41a000`, `0x41a0f6`, `0x90582c`, `0x835654`, `0x83564c`
Confidence: verified
Proof: one
Test: none

`0x449e65` pushes the saved original name (`[ebp-0xc]`), `0x449e69` the word at `0x83564c` and `0x449e70` calls `0x44a0e0`; `0x449e77` loads `0x90582c` and calls `0x44a9c0`, whose result is the return value. `0x44a0e0` pushes severity 3 and the format at `0x835654` and calls `[ecx+0xc]` (`0x44a11c`); `0x41a0f6` stores the severity at `+0x10` when it is higher. The caller decides what the placeholder is worth: the widget path (`ui/texture#settexture-failure`) discards it.

## Texture basename substitution

`0x44a310` replaces the file name of a BLP path before it opens it. When `[0x905bd4]` is non-null and the part of the path after its last backslash equals that string ignoring case, the part is replaced by `[0x905bd8]` in a 0x104-byte copy and the copy is opened. The compared part is the candidate with its `.blp`. `0x44b4e0(from = ecx, to = edx)` stores the two pointers. The violence-level handler `0x6c5af0` calls it: at level 2 with both 0, so nothing is replaced; at level 0 or 1 with `BloodSpurtSmall01.blp` (`[0x86c420]`) and `BloodSpurtSmall01_Low.blp` or `BloodSpurtSmall01_Medium.blp` (`[0x86c418]` and `[0x86c41c]`).

Addresses: `0x44a310`, `0x44a319`, `0x44a329`, `0x44a347`, `0x44a35f`, `0x44a37a`, `0x905bd4`, `0x905bd8`, `0x44b4e0`, `0x6c5af0`, `0x6c5b29`, `0x6c5b41`, `0x86c418`, `0x86c420`, `0x64a420`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

`0x44a319` skips everything while `[0x905bd4]` is null. `0x44a32b` finds the last `\` (`0x64a420` keeps the last match), `0x44a347` compares with `SStrCmpI` (0 means equal) and `0x44a34e` skips the rest on a difference. On a match `0x44a35f` copies the whole path and `0x44a37a` copies `[0x905bd8]` over it from the file name's offset, limited by what remains of 0x104, and `0x44a37f` points the open at the copy. Only the BLP loader does this; `0x44a140` has no such step. `0x6c5b29` is the level 2 call (both registers cleared); `0x6c5b41` loads `ecx` from `[0x86c420]` and `edx` from `[eax*4 + 0x86c418]`.

## Texture file name builders

Callers that build a texture file name themselves do not all split it. `0x449610` splits the name with `0x449590` and then always appends `.blp`, so `Foo.tga` opens `Foo.blp`. `0x44b320(name, buffer, type, size)` copies the name, cuts it at its last dot of any length (at the end when it has none), and appends `.BLP` for type 1 and returns 2, or `.TGA` for type 2 and returns 1; type 0 returns 0 with the copy uncut, and another type returns itself with the cut and no suffix. The builders that format a name append `.blp` to whatever they are given, with no split: a name ending in `.tga` gives `<name>.tga.blp`.

Addresses: `0x449610`, `0x44963c`, `0x449641`, `0x44b320`, `0x44b2c0`, `0x47ac57`, `0x525e0f`, `0x476e4c`, `0x479a73`, `0x479bb5`, `0x479cc3`, `0x479d85`, `0x479e30`, `0x47a1d4`, `0x523705`, `0x5f87b9`, `0x50338a`, `0x5037b9`, `0x64a450`, `0x64a760`, `0x64a7f0`
Confidence: verified
Proof: one
Test: none

`0x449590` has four `call` sites and no `jmp` or stored address: `0x449111` (in `0x4490c0`, reached only from `0x4499d0` at `0x449a05`; `0x4499d0` has no caller and no stored address), `0x44963c`, `0x449dcd` and `0x44ac8a` (in `0x44ac70`). `0x44b320` cuts at the last `.` found by `0x64a450` (`0x44b34b`) and appends with `0x64a760` (`0x44b366`, `0x44b380`); `0x47ac57` passes it the type `0x44b2c0` read from the last four characters. The formats are `%s.blp` (`0x525e0f`, the portrait name, `ui/portraittextures#portrait-sink`), `Item\TextureComponents\%s\%s_%s.blp` (`0x476e4c`), `%s%s.blp` (six sites from `0x479a73` to `0x47a1d4`, into a 0x104-byte buffer) and `Interface\Cursor\%s.blp` (`0x523705`, `0x5f87b9`). `0x50338a` and `0x5037b9` append `.BLP` to the emblem name built by `0x47a520`.

## Texture creation variants

`0x449b30(w, h, mips, format, filter, force)`, `0x449bf0(w, h, mips, format, filter, owner, callback, name, force)` and `0x449cc0(name, w, h, mips, format, filter, force)` create a texture with no file. Each allocates a `CTexture`, builds the flags word with `0x449ae0`, gets a device texture from `0x448450`, and stores the device texture, format, size, mip count and flags. The callback is the no-op `0x448830` except in `0x449bf0`, which passes the caller's callback and owner. The name is `Unique Texture` (`0x835624`) for `0x449b30`, and the given name or `Unique Texture No Name` (`0x835634`) for the others.

Addresses: `0x449b30`, `0x449bf0`, `0x449cc0`, `0x448830`, `0x449ae0`, `0x448450`
Confidence: verified
Proof: one
Test: none

The callbacks take `(mode, ...)`: mode 0 starts an update, 1 asks for a level's pitch and pointer, 2 ends it; `0x448830` ignores all three, so the owner fills these textures through `GxTexUpdateRect`.

## Texture file creation

`0x449610(path, format, deferred)` opens `path` with the extension `.blp` appended (`0x83524c`) after splitting it with `0x449590` through `SFile::OpenEx(0, name, deferred != 0, &handle)`, returns 0 when it fails, and otherwise allocates a `CTexture`, copies the name, stores `format` at `+0x118` and `0` at `+0x120`, and takes an async record from `0x4438b0`: record `+0` the file, `+8` its size, `+0xc` the texture, `+0x10` the completion callback `0x4497f0`. When the file's size fits in what remains of 2 MiB (counter `0xb05d18`) the record's buffer is the next slice of the static area at `0x905be4` and `0x443ae0` queues the read; otherwise the record joins the waiting list. `0x44a310` does the same for a name that already carries its extension, with callback `0x44a500`.

Addresses: `0x449610`, `0x44a310`, `0x4497f0`, `0x44a500`, `0x4438b0`, `0x443ae0`, `0x448c10`, `0x449840`, `0x44a560`, `0x448840`, `0x448920`, `0x4489a0`, `0x905be4`, `0xb05d18`
Confidence: verified
Proof: one
Test: none

The callbacks decode once the bytes are in memory. `0x4497f0` calls `0x449840`, which opens the buffer as BLP (`0x5a83b0`), sets bit 0 of `+0x110` when the alpha depth is 0, drops mip levels with `0x448b60`, allocates the decoded buffer through `0x44af90` and decodes into it (`0x5a8430`); the file is then closed and the record freed. `0x44a500` runs `0x44a560`, which does the same but creates the device texture with `0x448450` and uploads through `GxTexUpdateRect`; on failure it substitutes the solid texture. `0x448c10` gives the waiting records buffer slices up to the budget.

## Texture flags word

`0x449ae0(out, force, in)` writes `out = in`, except that when `force` is non-zero the low three bits (the filter mode) are replaced by the global `[0x835250]`; then, when the resulting mode is 5, bits 9 to 13 are replaced by `[0x835254]`, otherwise by 1. `0x449ac0` and `0x449ad0` store `[0x835250]` and `[0x835254]`; their static defaults are 3 and 1. The word is the `GxTex` flags: bits 0 to 2 filter mode, bit 3 wrap in U, bit 4 wrap in V, bit 6 generate mip maps, bits 9 to 13 maximum anisotropy.

Addresses: `0x449ae0`, `0x449ac0`, `0x449ad0`, `0x835250`, `0x835254`, `0x59f170`, `0x5a29d0`, `0x58a980`
Confidence: verified
Proof: one
Test: none

The ordinals come from the readers: `0x59f170` reads `(flags >> 3) & 1`, `(flags >> 4) & 1`, `(flags >> 6) & 1` and `(flags >> 9) & 0x1f`; `0x5a29d0` reads the same bits, so one word serves both backends. Who writes `[0x835250]` and `[0x835254]` is the CVar code; the stored values are not interpreted here.

## Device texture pool

`0x448450(width, height, mips, flags, owner, callback, format)` returns a device texture. When both sizes are powers of two, above 16 and below 1024, and `mips` is 7 or less, it reuses a free pooled texture with the same size class, mip count and filter bits, moving it to the busy list with the new callback, owner and flags; otherwise `0x58ac40` makes a new one. The size class is `log2(width / 32) * 5 + log2(height / 32)` into the 25 lists at `0xb05bec`. `0x448670` returns a texture to its class when the sizes qualify, stamped with the time (`0x42c010`), else destroys it; `0x448ec0` destroys pooled textures idle more than 30000 ms.

Addresses: `0x448450`, `0x448670`, `0x448ec0`, `0x449010`, `0x58ac40`, `0xb05bec`, `0x58aa30`, `0x58ab10`
Confidence: verified
Proof: one
Test: none

The pixel-format code is mapped through the table `0x835258` (`5, 6, 1, 3, 2, 4, 0, 7` for codes 0 to 7), and the flags passed to the device have bit 6 cleared (`and 0xffffffbf` at `0x448494`).

## Format choice and size limit

`0x4489a0` opens BLP bytes and decides two values from the colour encoding byte and the alpha depth: the `GxTex` format and the pixel type code. Alpha encoding 0 (DXT1) gives `Dxt1` when the device block `0x58a230` has `+0x40` set, otherwise `Rgb565` for alpha depth 0 and `Argb1555` for more; encoding 1 gives `Dxt3` when `+0x44` is set, otherwise `Argb4444`; encoding 7 gives `Dxt5` when `+0x48` is set, otherwise `Argb4444`; a colour encoding other than 2 gives `Argb8888`. `0x448b60(width, height, dropped)` halves both sizes, counting each halving, until neither exceeds `caps+0x60`, never below 1.

Addresses: `0x4489a0`, `0x448b60`, `0x58a230`, `0x449840`, `0x835258`, `0x835280`
Confidence: verified
Proof: one
Test: none

The `GxTex` formats are the order of the names at `0x835280`: Unknown, Argb8888, Argb4444, Argb1555, Rgb565, Dxt1, Dxt3, Dxt5. The type codes in `+0x118` are 0 Dxt1, 1 Dxt3, 2 Argb8888, 3 Argb1555, 4 Argb4444, 5 Rgb565, 7 Dxt5; `0x835258` maps a code to its format (5, 6, 1, 3, 2, 4, 0, 7). A creation hint of 8 becomes 5 for alpha depth 0, 3 for depth 1 and 4 otherwise (`0x4498fc` to `0x449932`); a hint of 9 becomes 5 for depth 0.

## Texture readiness and info

`0x44ad50` waits for an asynchronous load: it logs `AsyncTextureWait(): %s`, moves the async record to the pinned list (`+0x13c = 1`) and waits through `0x443bd0`. `0x44ae00(texture, wait)` returns the decoded buffer at `+0x120`, waiting first when it is null and `wait` is set. `0x44b3b0(texture, &w, &h)` returns the width and height at `+0x144` and `+0x148`, `0x44b430` also the mip count, format, opaque bit and alpha depth. A texture with no device texture and `wait` clear returns 0. `0x44acf0` is `GetTextureDeviceObject`.

Addresses: `0x44ad50`, `0x44ae00`, `0x44b3b0`, `0x44b430`, `0x44acf0`, `0x44ae70`, `0x44aec0`, `0x44af90`, `0x44b110`
Confidence: verified
Proof: one
Test: none

`0x44ae70(w, h)` counts the mip levels down to 1 by 1. `0x44af90(format, w, h)` hands out a decoded-chain buffer from the per-size lists `0x905844`; `0x44b110` returns it. `0x44b610(format, w, h)` gives the byte size of a mip chain from the block tables `0x80264c` (block edge) and `0x802670` (block bytes).

## Texture cache

`0x47ae30(name, flags)` returns a cache entry for a texture file name, creating it on a miss and adding 1 to its count at `+0xbc`. An entry is `0xc8` bytes from an object heap (`0x47aca0` makes it on first use with `0x463920(size 0xc8, count 0x400)`, which stores the size at `+0x10` and the count at `+0x14`), keyed by the hash of the name (`0x64af90`), name at `+0x38` (127 bytes), texture handle at `+0x18`, a failed-load mark at `+0xc4`. `0x47b0c0` loads it through `0x47ac10`, which tries `0x449610` on the name and again on the alternate name of `0x44b320`, marking `+0xc4` on failure. `0x47b010` subtracts 1 and frees the entry at 0 or less.

Addresses: `0x47ae30`, `0x47aca0`, `0x47ac10`, `0x47b0c0`, `0x47b010`, `0x47b000`, `0x47b120`, `0x47b150`, `0x47b0e0`, `0x47b1f0`
Confidence: verified
Proof: one
Test: none

`0x47b000` adds 1 without a lookup, `0x47b120` waits for the texture and returns its buffer (`0x44ae00`), `0x47b150(&out)` returns six dwords of size and format cached at `+0x1c`, and `0x47b0e0` drops the handle and the cached values of a loaded entry. `0x47b1f0` zeroes the lookup counter at `0xb42764` that `0x47ac10` increments.

## Device texture upload

Slot 0 of the device uploads a texture's levels when the texture is bound or touched: `0x59f5b0` (OpenGL, through `0x59f5d0`) and `0x5a0d70` (Direct3D, through `0x5a0a80`). Both ask the texture's update callback (`+0x44`, with the owner at `+0x40`) for each level: mode 0 begins, mode 1 returns a level's pitch and pixels, mode 2 ends; a texture whose flags have bit 7 set is dynamic and is not uploaded here. The first level uploaded is `min([dev+0x31c], levels - 1)`, so that many top levels are skipped, and each next level has half the size down to 1 by 1.

Addresses: `0x59f5b0`, `0x59f5d0`, `0x59f0f0`, `0x59f270`, `0x5a0d70`, `0x5a0a80`, `0x5a0880`, `0x5a4f60`, `0x592050`, `0x80a034`, `0x80a07c`, `0x80a0a0`
Confidence: verified
Proof: one
Test: none

The first level and count come from one rule (`0x5a0880` for Direct3D, inline in `0x59f5d0`). Filter mode (the low three flag bits) 0 or 1, or flag bit 6, with bit 5 clear: one level at full size, first level 0, count 1, no skip. Otherwise the count is `floor(log2(max(width, height))) + 1`, and with bit 5 set it is the first level plus 1: one level, the first after the skip.

OpenGL creates the texture name with `glGenTextures` on first use, binds it (`0x59f0f0`), applies the sampler (`0x59f170`) and, for the compressed formats 5 to 7 when the extension flag `[0xc0f274]` is set, limits `GL_TEXTURE_MAX_LEVEL` (`0x813b`) to the count of halvings until the smaller side is 4 or less; `0x59f270` uploads one level with `glTexSubImage2D`, `glCompressedTexSubImage` or the rectangle variants by the tables at `0x80a034` (internal format), `0x80a07c` (format) and `0x80a0a0` (type), indexed by `GxTex` format. Direct3D locks each level of the texture (`GetSurfaceLevel`, `LockRect` with `0x800`) and copies the rows (`0x5a4f60`). `0x592050` stores the skip count at `+0x31c`.

## Device texture record

`0x594a80` (`CGxDevice::CreateTexture`, reached by through the Direct3D forwarder `0x5a0810`) allocates a `0x54`-byte record with `SMemAlloc` (`CGxDevice.cpp` line `0x86b`), constructs it from its nine arguments (`0x5915a0`, which builds the flags at `+0x3c`), returns it through the last argument, links it into the device's texture list (head at `[dev+0x2838]`), adds 1 to the count at `[dev+0x84]` and adds the record's byte size to `[dev+0x88]`. `0x594ce0(record, width, height)` returns that size: the top kept level (width and height shifted right by the skip count `[dev+0x31c]`) as blocks of the format's edge `ceil(w / e) * ceil(h / e) * bytes`, plus each smaller level down to 1 by 1 when the flags' low three bits exceed 1.

Addresses: `0x594a80`, `0x594ce0`, `0x5915a0`, `0x5a0810`, `0x809d48`, `0x809d6c`
Confidence: verified
Proof: one
Test: none

The block edge table `0x809d48` is 1 for formats 0 to 4 and 8 and 4 for formats 5 to 7; the bytes per block table `0x809d6c` is 0, 4, 2, 2, 2, 8, 16, 16, 2 for formats 0 to 8, indexed by the format at `+0x34` of the record. Passing -1 for width or height takes the record's own size at `+0x24` and `+0x28`.
