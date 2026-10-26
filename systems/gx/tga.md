# gx: TGA textures

How `Texture.cpp` turns a TGA file into the decoded mip chain of a texture: the decode of the
file itself and the lower levels it reads from the files beside it or builds. The TGA record and
its decoder belong to `image` (`image/tga#image-record`); the texture object, its callback and the name of
the mip files are in `textures.md`.

Roots: `0x4491f0`

## TGA texture decode

`0x4491f0(name, fileFlag, chain, width, height, arg6, opaque, alphaBits, typeCode)` opens the
file (`0x5a3660`), sets `opaque` when its attribute bits are 0, decodes with flags 3 (opaque alpha
added, palette expanded), sets the top origin (`0x5a4070`) and makes a 32-bit chain when `*chain`
is null (`0x44af90`), else lays out the one it holds (`0x5a4bb0`). Level 0 gets the decoded
pixels as they are, B, G, R, A: `Argb8888`. The mip files give the next levels and `0x44aec0`
builds the rest. Non-null outputs get the width, the height, 1, the attribute bits after the
decode and type code 2. It returns 1, or 0 when the open or decode fails; a depth other than 32
after the decode crashes.

Addresses: `0x4491f0`, `0x449216`, `0x449237`, `0x449249`, `0x449265`, `0x44927b`, `0x449294`, `0x44929f`, `0x4492a7`, `0x4492b8`, `0x4492c4`, `0x449314`, `0x449329`, `0x44933e`, `0x449375`, `0x449384`, `0x5a3660`, `0x5a3a30`, `0x5a4070`, `0x5a41f0`, `0x44af90`, `0x5a4bb0`, `0x44aec0`, `0x44a2db`, `0x44918a`
Confidence: verified
Proof: one
Test: `gx::tga_opaque_flag_difftest`, `gx::tga_pixel_copy_difftest`

`ret 0x1c`: the name and the open flag in `ecx`, `edx`. `0x449237` is `and cl,0xf; neg cl; sbb
ecx,ecx; inc ecx`, read before the decode; `0x449375` reads the nibble again after it, 8 for a file
that had none. `0x44927b` counts the levels (`0x44ae70`). `0x5a41f0` returns the pixels only when
the depth is 32 and 0 otherwise, and the copy at `0x4492c4` reads through what it returns, so a
file the depth check `0x5a37e0` admits with 25 to 31 or 33 to 39 bits faults there. The loop
packs four bytes and stores them back in the same order, `w · h` times; a zero size skips it
(`0x4492b8`). `0x449314` builds the mip name pattern and `0x449329` passes it with the open flag
and the chain. The live caller is the TGA texture callback (`0x44a2db`), with `&texture+0x120` and
the open flag `+0x110 & 2`, and every other output null; `0x44918a` is in the unreached
`0x4490c0`. The tests run `0x449237` to `0x449244` and the copy loop `0x4492c0` to `0x449300`.

## TGA mip files

`0x4493a0(record, pattern, fileFlag, levels)` reads lower mip levels from files: for level n from
1 while the base image has smaller levels, it formats the pattern with n, stops when that file
does not resolve or does not open, or when its size is not the expected one (the base's sides
halved for level 1, a side of 1 giving 0, and each later level halving the sides above 1), decodes it with flags 2, adds an opaque alpha when it
has none, sets the top origin and copies its 32-bit pixels into chain level n. It returns 1 plus
the number of files read; `0x4491f0` passes that count to `0x44aec0` as the first level it builds.

Addresses: `0x4493a0`, `0x4493c0`, `0x4493c6`, `0x4493c8`, `0x449405`, `0x449418`, `0x449444`, `0x449457`, `0x449463`, `0x44946e`, `0x449486`, `0x449490`, `0x449498`, `0x4494a0`, `0x4494b6`, `0x4494f9`, `0x44952d`, `0x648a30`
Confidence: verified
Proof: one
Test: `gx::tga_mip_copy_difftest`

`ret 8`. `0x4493c0` counts the base's levels; one level returns 1 at once. `0x449405` is
`SStrPrintf` into a 0x104-byte buffer and `0x449418` `SFileExistsEx` with the open flag. The size
tests are `0x449457` and `0x449463`; `0x449486` calls `0x5a3fc0` when the nibble is 0 (`0x44947b`).
`0x4494a0` takes the destination `levels[n]`; the copy is the same loop as `0x4491f0`'s, and the
same depth fault applies; the test runs it, `0x4494b6` to `0x4494f6`. `0x4493c6` and `0x4493c8` halve the base's sides with `shr`; `0x4494f9` to `0x449515` halve each
side above 1.
