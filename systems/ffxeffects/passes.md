# ffxeffects: the passes

The passes themselves: the box and Gauss blurs, the glow combine and its underwater variant with
the wave map, the readers of the drunk byte that feed the combine, and the death passes.

Roots: none

## Box and Gauss passes

Box4 (`0x6caae0`) draws its source into the destination with four taps at `(−1.5, −1.5)`, `(0.5,
−1.5)`, `(0.5, 0.5)` and `(−1.5, 0.5)` source texels; in the glow effect it takes the full target to
the quarter. Gauss4 (`0x6cad10`) blurs in two draws: horizontally into `0xce8b04` with taps `(−2.5,
0)`, `(−0.5, 0)`, `(0.5, 0)`, `(2.5, 0)`, then vertically from `0xce8b04` to its destination with the same
four in y. The taps are scaled by the source's `1/alloc` (`+0x14`, `+0x18`) and added to the quad's
UV offsets, so they are texel counts. Each draw binds the source at stages 0x17 to 0x1a and the
program at 0x3f, and draws the strip.

Addresses: `0x6caae0`, `0x6cad10`, `0xce89cc`, `0x6cab06`, `0x6cad1c`, `0x6cad8c`, `0x6cadf5`, `0x6caea3`, `0x6cabbf`, `0x6cacf3`, `0x6cac20`, `0x6cd750`, `0x58a830`, `0x589e80`
Confidence: verified
Proof: one
Test: none

The Box4 table `0xce89cc` is written once (`0x6cab06` to `0x6cab42`, flag byte `0xce89ec`) with the
floats above. The Gauss4 arrays are built on the stack at `0x6cad1c` to `0x6cad85`: `0xc0200000`, 0,
`0xbf000000`, 0, `0x3f000000`, 0, `0x40200000`, 0 for x, and the same values at the odd places for y.
Both passes call `0x6cd3c0`, `0x6cd580` and `0x6cd750` (colour `0xffffffff` from `0x6acdd0`, scale
`[this+4] + 0x14`), `0x6cd810`, bind four stages with `0x589e80` (0x17 to 0x1a) and the program (0x3f),
call `0x58a830` and end with `0x6cd840`. Gauss4's first draw binds `0xce8b04` with `0x589d30`
(`0x6cadfa`); its second binds the destination. The kernel weights are in the programs
`FFXBox4.bls` and `FFXGauss4.bls` under `Shaders\Pixel`, whose names the exe holds.

## Glow combine pass

The Glow pass (`0x6cb020`) draws the full target and the quarter blur to the screen with the vertex
colour `ARGB = (w, z, z, z)`. `w` is the byte of `light.glow·255` (the float at `+0x110` of
`0x6d48b0`'s record). `z` is 0 with no active player, else the byte of `min(drunk, 100)·2.55`, `drunk`
the byte at `[player+0xe68]+0x1d` (`ui/tooltipunit`), raised to at least 84 when the camera
liquid state `0x672470(0)` is not 0xf. A byte is `bits(x + 512) >> 14`, the +512 trick. It binds the
full target at stage 0x17, the quarter at 0x18 and its program, and draws the strip.

Addresses: `0x6cb020`, `0x6d48b0`, `0x6cb0e7`, `0x6cb0f5`, `0x6cb101`, `0x6cb104`, `0x6cb123`, `0x6cb12e`, `0x6cb144`, `0x6cb14d`, `0x6cb155`, `0x6cb172`, `0x6cb17a`, `0x6cb180`, `0x6cb192`, `0x6cb1be`, `0x468550`, `0x468460`, `0x467d00`
Confidence: verified
Proof: one
Test: `ffxeffects::ffx_glow_color_quantize_difftest`

`0x6cb0e7` to `0x6cb101`: `fld [+0x110]; fmul 255.0; fadd 512.0; fstp dword; shr ebx,0xe`; the register
keeps the whole shifted dword and the colour takes its low byte (`mov dh,bl`, `0x6cb188`). `0x6cb104`
calls `0x467d00`; when nonzero `0x6cb112` and `0x6cb123` find the active player (`0x468550`) by type mask
0x10 with `0x468460`, and a null result gives 0. `0x6cb137` caps the byte at 0x64, `0x6cb147`
multiplies by the float 2.55, `0x6cb155` adds 512. The unsigned compare `cmp al,cl; ja` (`0x6cb180`)
takes the larger of the byte and 84. `0x6cb192` to `0x6cb19c` shift the bytes into `w:z:z:z`. The test
runs the packing instructions.

## Drunk byte readers

Fifteen places read the inebriation byte `[player+0xe68]+0x1d`, each cutting it to 100: the two glow
passes, the chat text builder (`0x49f614`), the camera drift (`0x50d1de`, `0x50eb7d`, `0x5104a5`), the
displayed level of an attackable unit (`0x51809a`, `0x7cbcf9`), the unit tooltip (`0x52a391`), the drunk
notices (`0x5e2ac3`), an effect kind (`0x60001a`) and the mover's wobble (`0x60a9bc`). Only the glow passes
reach the post-process. The displayed level is `max(1, level − ⌊min(drunk, 100)·0.05 + 0.5⌋)`.

Addresses: `0x6cb12e`, `0x6cb592`, `0x49f614`, `0x50d1de`, `0x50eb7d`, `0x5104a5`, `0x51809a`, `0x5180b3`, `0x52a391`, `0x5e2ac3`, `0x60001a`, `0x60a9bc`, `0x7cbcf9`, `0x7cbce0`, `0x517fc0`, `0x606980`, `0x508680`, `0x8029c8`, `0x8029d0`, `0x808bf4`, `0x80c488`
Confidence: verified
Proof: one
Test: none

A scan finds a load of `+0xe68` followed within three instructions by a byte read at `+0x1d` 14 times;
`0x6cb592` adds `0xe68` instead. `0x60001a` multiplies the byte by 0.01 (`0x8029d0`) and takes effect kind 7
from 0.5 (`0x80c488`); `0x49f651` passes that fraction to `0x508680`. `0x7cbce0(player, unit)` needs the
unit's level (`[[unit+0x110]+0x70]`) above 0, a nonzero byte and `0x606980` true, then clamps at 1
(`0x7cbd3d`); `UnitLevel` (`0x517fc0`) does the same at `0x5180b3` with the byte times 5.0 (`0x808bf4`) then 0.01.

## Glow wave pass

The GlowWave pass (`0x6cb310`, ctor `0x6cb1f0`) is the Glow pass under water with a displacement map
as a third source. Its quad has three UV sets: the wave map's, transformed by `R(10°, Z)
· S(W/128, 0.88·H/128) · T(p1, p2)` in row-vector order, the scene's and the blur's. `p1` and `p2` are the clock `t` in
milliseconds mod 3174 and mod 2805, each over its modulus. The colour is Glow's without the `0x467d00` gate: the floor of 84 applies whenever the liquid state `0x672470(0)` is not 0xf, where Glow needs a player.

Addresses: `0x6cb310`, `0x6cb1f0`, `0x42c010`, `0x7bdc40`, `0x7bdca0`, `0x7bdd60`, `0x7bca80`, `0x6cb43a`, `0x6cb443`, `0x6cb467`, `0x6cb4e6`, `0x6cb6fa`, `0x6cb72f`, `0x6cb77a`, `0x6cb7c2`, `0x6cb5d5`, `0x672470`, `0x58b3c0`, `0x58b4d0`, `0xce89bc`, `0xce89c0`, `0xce89c4`, `0xce89c8`, `0x467d00`
Confidence: verified
Proof: one
Test: `ffxeffects::ffx_glow_pass_render_difftest`

`0x6cb43a` reads the clock, `div [0xce89c4]` (3174) and `div [0xce89c8]` (2805, of the same `t`);
`fild qword; fidiv` give `p1` and `p2`, stored as floats (`0x6cb477`, `0x6cb483`). `0x7bdc40`
translates by `(p1, p2, 0)`, `0x7bdca0` scales the rows by `sx = W·[0xce89bc]/128` and `sy =
H·[0xce89c0]/128` with `[0xce89bc] = 1.0` and `[0xce89c0] = 0.88` (`0x6cb48e` to `0x6cb4bd`; `W`
and `H` are the target's `+0xc` and `+0x10`), and `0x7bdd60` multiplies on the left by the rotation of
`0x3e32b8c3` (10° as a float) about `(0, 0, 1)`; the wave UVs go through `0x7bca80`. 3.0 is the
float at `0x81147c`. The constructor `0x6cb1f0` sets `+0x30` to 0, plus 1 when `0xce89f8` is nonzero (`0x6cb21e`) and plus 2 when `caps+0x4c` is 0 (`0x6cb22e`), loads the four programs of `FFXGlowWave.bls` to `+0x34` (`0x58b2e0`) and keeps the constant handles at `+0x44` and `+0x48`. The base constructor clears `caps+0x4c` and no store to it in the device code, `0x58a000` to `0x5a8000`, is outside the OpenGL path (`0x58e5d6`), so on Direct3D the index is 2. The player lookup at `0x6cb58d` is not tested for null (`0x6cb592` adds `0xe68`), and `0x467d00` is not called in `0x6cb310` to `0x6cb7df`; the floor is `and eax,0x54` at `0x6cb5d5`.

## Glow wave constants

The pass sets two program constants: with `0xce89f8` zero, `screenMtx
= (3/allocW, 0, 0, 3/allocH)` of the scene target and `blurMtx = (0.75/allocW, 0, 0, 0.75/allocH)`
of the blur target; with it set, `(3, 0, 0, 3)` and `(0.75, 0, 0, 0.75)`.

Addresses: `0xce89f8`, `0x6cb72f`, `0x6cb77a`, `0x6cb7c2`
Confidence: verified
Proof: one
Test: none

## Wave map table

`0x6cbea0` is the data callback of the 128 by 128 wave map `0xce89a0`, created by the pass base
`0x6cbcf0` when the first such pass is built (a count in `0xce8998`). With `ecx` 0 it allocates
`texels·16384` bytes and fills them; with 1 it stores the pitch and the address; with 2 it frees the
buffer. For format 8 a texel is two signed bytes, low `trunc(clamp(128·sin(2πx/128), −128, 127))`
and high the same of `y`. For format 1 a texel is the dword `(cx << 16) | (cy << 8)` with `c =
trunc(clamp((sin·0.5 + 0.5)·255, 0, 255))` of x and y. The format is 1 when `caps+0x98` is 0xa, 3
or 4, else 8.

Addresses: `0x6cbea0`, `0x6cbcf0`, `0x6cbe30`, `0x6cbe00`, `0xce8994`, `0xce8998`, `0xce898c`, `0xce89a0`, `0x6cbd1a`, `0x6cbd8f`, `0x6cbdd1`, `0x6cbf3f`, `0x6cbf4e`, `0x6cbf71`, `0x6cc03a`, `0x6cc0dc`
Confidence: verified
Proof: one
Test: `ffxeffects::ffx_glow_wave_lut_difftest`

`0x6cbeb6` gives 4 bytes for format 1 and 2 otherwise; mode 0 is `0x6cbf08` (`shl eax,0xe`, the
allocation at line 619 with the source path `0x86f1c4`), mode 1 `0x6cbeee` (`shl eax,7` for the pitch), mode 2 `0x6cbec6` (line 653).
`0x6cbf3f` to `0x6cbf50` are `fild; fmul 1/128; fmul 2π; fsin; fmul 128.0`; `fsin` is the x87
transcendental, whose result the host's `sin` approximates. The compares are against −128.0
(`0x806540`) and 127.0 (`0x811310`), then `__ftol` truncates. The unsigned packer (`0x6cc03a`) is
`fmul 0.5; fadd 0.5; fmul 255.0`. The descriptor `0xce89a0` has 128 at `+4` to `+0x10` and `1/128` at
`+0x14` and `+0x18` (`0x6cbd3a` to `0x6cbd7b`). The test runs the pack slice with the sine given. `0x6cbd27` to `0x6cbd66` push 1 for the filter and both wraps, 0 for the next three flags, 1 for the anisotropy and 0 for the last; `0x58a980` packs them into bits 0 to 2, 3, 4, 5 to 7, 9 to 13 and 8, the anisotropy cut to `caps+0xac`, so the sampler is linear and repeats in both axes (`0x219`). The one reader of the map is `0x6cc1ae`.

## Death passes

The death pass (`0x6cb930`, ctor `0x6cb7e0`) draws the full target and the quarter blur to the screen
with the vertex colour `(byte << 24) | 0x5393a8`, the byte of `light.glow·255` as in the glow pass.
Its sources are bound at stages 0x17 and 0x18. The ctor loads `FFXDeath_nvts_rect.bls` or
`FFXDeath_nvts_2d.bls` when `caps+0x98` is 6, 7 or 8, by `0xce89f8`, else the two-program set of
`FFXDeath.bls`. The combiner pass (`0x6cbb00`, ctor `0x6cba60`, program `FFXDeath_nvrc.bls`) draws one
source with the constant colour `0xff5393a8` and sets render state 7 to its `+0x28`.

Addresses: `0x6cb930`, `0x6cb7e0`, `0x6cbb00`, `0x6cba60`, `0x6cb817`, `0x6cb83c`, `0x6cb863`, `0x6cb872`, `0x6cba17`, `0x6cba1a`, `0x6cbb55`, `0x6cbb8f`, `0x6d48b0`
Confidence: verified
Proof: one
Test: `ffxeffects::ffx_death_pass_pack_difftest`

`0x6cb9e6` to `0x6cba0b`: `fld [+0x110]; fmul 255.0; fadd 512.0; fstp dword; shr eax,0xe; mov cl,al`;
`0x6cba17` is `shl ecx,0x18` and `0x6cba1a` `or ecx,0x5393a8`. `0x6cb817` to `0x6cb83a` compare
`caps+0x98` with 6, 7 and 8; the rectangle program is chosen when `0xce89f8` is nonzero (`0x6cb872`).
The pass has no time input. The combiner draws with the UV array `0xce8a98` as it stands and sets
state 7 at `0x6cbb8f`. The test runs the packing instructions.
