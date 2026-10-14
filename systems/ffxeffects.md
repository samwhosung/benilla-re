# ffxeffects

The full-screen effects: the glow (with its underwater variant) and the death tint, drawn as a
chain of passes between the world and the screen. The code only prepares each pass; the blur,
glow and tint are the device's fragment programs.

It is one object file, `0x6ca9d0` through `0x6cdf20`, which carries the source path
`FFXEffects.cpp` (`0x86f1c4`); the passes have no assertions, so the first reference to the path is
`0x6cbed3`. Its initializers are the twelve slots `0x82cdb4` to `0x82cde0`. The classes are
`FFX::Pass` (the type tag `.PAVPass@FFX@@`, `0x86f228`) and its subclasses, one per program:
`FFXBox4`, `FFXGauss4`, `FFXGlow`, `FFXGlowWave` and three death programs. The effect objects own
lists of passes; `0x6cd890` and `0x6cda70` bracket the world paint with the current effect.

Roots: `0x6ccfc0`, `0x6cd1d0`, `0x6cc130`, `0x6cc690`, `0x6cd890`, `0x6cda70`, `0x6cde60`, and the
initializers `0x6cbbc0`, `0x6cbc30`, `0x6cbc90`,
`0x6ccb20`, `0x6ccb90`, `0x6ccc00`, `0x6ccc70`, `0x6ccce0`, `0x6ccd40`, `0x6ccda0`, `0x6ccde0` and
`0x6cced0`.

## Pass and effect objects

A pass (`0x6cde70`, the `FFX::Pass` constructor) copies up to 8 source target pointers to `+4` and
keeps the destination target at `+0x24`, a dword at `+0x28` and a flag byte at `+0x2c`; a subclass
adds its program handles from `+0x30`. Its vtable is `0x81148c` (deleting destructor `0x6caa90`, two
pure slots) or a subclass's: slot 0 deletes, slot 1 asks whether the program loaded, slot 2 renders.
An effect holds its CVar record at `+4` and two pass lists, at `+8` and `+0x18`, each `{capacity,
count, data, quantum}`. Its slots `0x6cded0` and `0x6cdf20` run slots 1 and 2 over list A.

Addresses: `0x6cde70`, `0x6caa90`, `0x6cded0`, `0x6cdf20`, `0x86f228`, `0x6ca9d0`, `0x6caa30`, `0x6caa60`, `0x6caac0`, `0x6cac30`, `0x6cac90`, `0x6cacc0`, `0x6cacf0`, `0x6caf40`, `0x6cafa0`, `0x6cafd0`, `0x6cb000`, `0x6cb1f0`, `0x6cb290`, `0x6cb2c0`, `0x6cb2f0`, `0x6cb7e0`, `0x6cb890`, `0x6cb8c0`, `0x6cb910`, `0x6cba60`, `0x6cbaa0`, `0x6cbad0`, `0x6cbaf0`, `0x81148c`, `0x811480`, `0x811498`, `0x8114a4`, `0x8114b4`, `0x8114c0`, `0x8114cc`, `0x8114d8`, `0x8114e4`, `0x8114f0`, `0x8114fc`
Confidence: verified
Proof: one
Test: none

`0x6cde7c` stores `0x81148c` and `rep stos` zeroes the 8 source slots (`0x6cde89`); the loop at
`0x6cdea0` copies the pointers of the array argument, as many as its count argument; `+0x24`,
`+0x28` and `+0x2c` take the third to fifth arguments (`0x6cdeb4`, `0x6cdebe`, `0x6cdebb`).
Subclass vtables: Box4 `0x811480`, Gauss4 `0x811498`, Glow `0x8114a4`, GlowWave `0x8114b4`, death
`0x8114c0`, the combiner death pass `0x8114cc`, the wave map base `0x8114d8`; effects `0x8114e4`
(glow), `0x8114f0` (base) and `0x8114fc` (death). Each subclass has a constructor, a deleting
destructor, a destructor body and a readiness test at the addresses listed; the pass lists are
tagged `.PAVPass@FFX@@` (`0x86f228`). `0x6cded0` returns 0 for an empty list, else the
AND of slot 1 over the list with the effect's CVar integer (`[[this+4]+0x28]`, a null record
giving the list result alone); `0x6cdf20` calls slot 2 on each pass in order.

## Effect pass lists

`0x6cc130` builds the glow effect: it registers the CVar `ffxGlow` and fills list A with Box4 (full
target to quarter), Gauss4 (quarter) and Glow (full and quarter to the screen), and list B with
Box4, Gauss4 and GlowWave (wave map, full and quarter to the screen). `0x6cc690` builds the death
effect; the capability word `caps+0x98` picks its list: 5 gives the combiner pass alone, 0 or more
(other than 5) gives Box4, Gauss4 and the death pass, a negative value gives none. The glow effect
runs list B when `0x467d00()` is nonzero and the camera liquid state `0x672470(0)` is not 0xf, else
list A.

Addresses: `0x6cc130`, `0x6cc690`, `0x6cc5a0`, `0x6cc630`, `0x6cc4a0`, `0x6cc4d0`, `0x6cc560`, `0x6cc8c0`, `0x6cc8f0`, `0x467d00`, `0x672470`, `0x6cc375`, `0x6cc6de`, `0x6cc73e`
Confidence: verified
Proof: one
Test: none

The passes are allocated by `0x6462e0` at `0x3c` bytes (Box4, Gauss4, Glow), `0x4c` (GlowWave),
`0x40` (death) and `0x34` (combiner). `0x6cc375` appends a second Glow instead of the GlowWave only
when `0xce89f8` is nonzero and `caps+0x5c` is nonzero. `0x6cc5a0` returns the CVar's integer `+0x28`
nonzero ANDed with slot 1 of each pass in the chosen list; `0x6cc630` renders them in order. The
test is the liquid state against 0xf alone, so water, ocean, magma and slime (`terrain/camstate`) all select list B. The death effect has one list and none of the nine callers of `0x672470` is in its code, so a ghost gets neither the warp nor the blur floor. `0x6cc6de` and
`0x6cc73e` compare `caps+0x98` with 5 and with −1.

## Start and shutdown

`0x6ccfc0` registers the CVars `ffx` (`full screen effects`) and `ffxRectangle` (`use rectangle
texture for full screen effects`), each with default `1` and the handler `0x6cde30`, looks up
`gxMultisample`, clears the target descriptors, and sets `0xce89f8` to 1 when `ffxRectangle`'s
integer is nonzero and `caps+0x98` is 6, 7, 8 or 10, else 0. `0x6cde30` prints `enabled` for a
nonzero `atoi` of the new value and `disabled` otherwise, and returns 1; the effects register
`ffxGlow` and `ffxDeath` with it too. `0x6cd1d0` destroys the textures of the seven target
descriptors (`0x58ad60` where a handle is set) and zeroes them.

Addresses: `0x6ccfc0`, `0x6cd1d0`, `0x6cde30`, `0x6cde60`, `0x6cd16f`, `0x6cd1ac`, `0x6cd390`, `0xce89f8`, `0xce89fc`, `0xce8a00`, `0xce8a04`, `0xce8bb4`, `0x46a700`, `0x481c46`, `0x481e4e`, `0x46a7a0`, `0x481cdb`, `0x481c63`, `0x481c8d`, `0x5de9c0`, `0xb4b350`, `0xb4b39c`
Confidence: verified
Proof: one
Test: none

The registrations are `0x6ccfe3` (`ffx`) and `0x6cd007` (`ffxRectangle`); the records go to
`0xce8a04` and `0xce8a00`, and `0x63de30` returns `gxMultisample`'s to `0xce89fc`. `0x6cd16f` tests
`[[0xce8a00]+0x28]`, `0x6cd179` to `0x6cd1aa` compare `caps+0x98` against 0xa, 6, 7 and 8, and
`0x6cd1ac` stores 1. The callers of `0x6ccfc0` are glue (`0x46a700`) and the world frame
(`0x481c46`), which then build the effects. `0x6cd1d0` is called from `0x481e4e` and jumped to from
`0x46a7a0`. `0x6cde60` stores the current effect (`0xce8bb4`) and returns the old one; its six
callers are glue, the world frame (`0x481cdb`, with the glow effect `0xb4b350` or the death effect
`0xb4b39c` its `0x481c63` and `0x481c8d` built) and `0x5de9c0`, which makes the death effect current
while `PLAYER_FLAGS` bit 0x10 is set and the glow effect otherwise. An effect is a single slot:
the death effect replaces the glow effect.

## Begin and end

`0x6cd890` runs before the world paint and `0x6cda70` after. Both do nothing unless the `ffx`
CVar's integer, the `pixelShaders` CVar's integer (`0xc7f360`), the current effect `0xce8bb4` and its
slot 1 are all nonzero. Begin calls `0x58a1e0(6, 1)` on entry and `(6, 0)` once the checks pass, and
resizes the targets by `0x6cdb40` when the window size changed. With `gxMultisample` 1 it binds the
full target and sets the viewport to the window's extents over the full target's allocation (at most
1); otherwise to the full target's size over the window's (at most 1). End unbinds the full target
(`gxMultisample` 1) or copies the backbuffer into it (`0x58ae00`), renders the effect through slot 2,
and sets the whole viewport.

Addresses: `0x6cd890`, `0x6cda70`, `0x6cd93a`, `0x6cd944`, `0x6cd98a`, `0x6cd9f5`, `0x6cdaa8`, `0x6cdad9`, `0x6cdae6`, `0x58af60`, `0x58a240`, `0x58ae00`, `0x589d30`, `0xc7f360`, `0x46fad3`, `0x48350e`, `0x48379d`
Confidence: verified
Proof: one
Test: none

`0xc7f360` is the record of the CVar `pixelShaders` (registered at `0x688712`). `0x6cd918` and
`0x6cd925` take `__ftol` of `rect3 − rect1` and `rect2 − rect0` of `0x58a240`, compared at `0x6cd92a`
with `0xce8ba4` and `0xce8ba8`. `0x6cd944` tests `gxMultisample` against 1. `0x6cd98a` to
`0x6cd9ab` divide the window extents by the full target's `+4` and `+8` and replace a quotient that
is not below 1.0 by 1.0; `0x6cd9f5` on gives `full/window` where the full target is smaller than the window, else 1.0. The
viewport arguments are `(0, x, 0, y, 0, 1)`. The call sites are glue (`0x46fad3`; `0x46fae0` jumps
to `0x6cda70`) and the world frame (`0x48350e`, `0x48379d`).

## Render target sizes

`0x6cdb40` sizes the targets from the window rectangle. With `w` and `h` the `__ftol` of its extents,
it stores them and `1/w`, `1/h` (rounded to float) in the screen descriptor `0xce8b98`. With `m` the
maximum texture size of the mode `0x6cdb10` gives (`caps[0x60 + 4·mode]`), it allocates the full
target at `max(8, min(w, m)) × max(8, min(h, m))`, then the half (`/2`, toward zero) and quarter (`/4`,
toward zero) of that size. `0x6cdb10` returns 2 when `ffxRectangle` is nonzero, `caps+0x58` is nonzero
and `caps+0x5c` is 0, else 0.

Addresses: `0x6cdb40`, `0x6cdb10`, `0x6cdbd8`, `0x6cdbe8`, `0x6cdc1d`, `0x6cdc2c`, `0x6cdc51`, `0x6cdc60`, `0x6cdc8d`, `0x6cdc9c`, `0xce8ba4`, `0xce8bac`, `0xce8bb0`, `0x58a230`
Confidence: verified
Proof: one
Test: `ffxeffects::ffx_compute_rt_dims_difftest`

`0x6cdb73` and `0x6cdb83` truncate; `0x6cdb90` and `0x6cdbc6` are `fdivr 1.0`. The clamp is `cmp; jl` (signed)
then `cmp 8; ja` (unsigned), at `0x6cdbe8` to `0x6cdc08`. The half is `cdq; sub eax,edx; sar
eax,1` (`0x6cdc34`), the quarter `cdq; and edx,3; add eax,edx; sar eax,2` (`0x6cdc68`). The six
calls to `0x6cdcb0` are the full target (`0xce8b5c`, flag 1), a twin `0xce8b78`, the halves
`0xce8b20` and `0xce8b3c`, and the quarters `0xce8ae8` and `0xce8b04`, flag 0. Only `0xce8b5c`,
`0xce8ae8`, `0xce8b04` and `0xce8b98` are read by a pass; the twin and the halves are allocated and
used by nothing else. The test runs the reciprocal instructions in the emulator and compares the
integer sizes with its transcription.

## Render target allocation

`0x6cdcb0` (`ecx` a descriptor, `edx` the wanted `{w, h}`, a flag on the stack) makes a target
texture. In mode 0 each dimension is rounded up to a power of 2; each is then clamped to
`caps[0x60 + 4·mode]` and to at least 8. When the allocated size differs from the descriptor's `+4`
and `+8` it frees any old texture, creates a new one and clears it. It stores the wanted size at
`+0xc` and `+0x10`, the allocated at `+4` and `+8`, and `1/allocW`, `1/allocH` as floats at `+0x14`
and `+0x18`; `+0` is the handle, and the descriptor is 7 dwords (`0x1c` bytes).

Addresses: `0x6cdcb0`, `0x58a980`, `0x58ab10`, `0x58ad60`, `0x6cdd2a`, `0x6cdd60`, `0x6cdd97`, `0x6cdde4`, `0x6cddf6`, `0x6cde0b`, `0x6cd390`, `0xce8ae8`, `0xce8b04`, `0xce8b5c`
Confidence: verified
Proof: one
Test: none

`0x6cdcc5` loads `caps+0x70`: `neg; sbb; and −3; add 4` gives the texture format 1 when it is nonzero
and 4 when it is 0 (`0x6cdcd4` to `0x6cdce4`). The power-of-2 loops are at `0x6cdd10` and
`0x6cdd21`. The unsigned `ja` tests (`0x6cdd4b`, `0x6cdd59`) make the floor 8. The new texture is
created by `0x58ab10` with the size, format and mode, bound with `0x589d30`, cleared through `0x58a920`
and unbound. `0x6cd390` zeroes a descriptor.

## Pass target and draw state

`0x6cd3c0` (`ecx` a target descriptor) prepares a pass: it saves the device's projection (`0x58b090`)
in `0xce8a48` and its view (`0x58b0b0`) in `0xce8a08`, installs the orthographic matrix `(0, w, 0, h,
−1, 1)` of the target's `+0xc` and `+0x10` and an identity view, clears the render states 0x14,
0xe, 0x10, 0x12 and 0xf, binds the target, and sets the viewport to `(0, w/allocW, 0, h/allocH, 0,
1)`. `0x6cd840` restores the saved matrices, rebinds the screen and sets the viewport `(0, 1, 0, 1,
0, 1)`. `0x6cd810` builds a 4-index buffer from the strip order 0, 3, 1, 2 (`0x86f260`) and passes
it to the device.

Addresses: `0x6cd3c0`, `0x6cd840`, `0x6cd810`, `0x86f260`, `0x58b090`, `0x58b0b0`, `0x58b040`, `0x58b050`, `0x5c3d90`, `0x589e60`, `0x58af60`, `0xce8a08`, `0xce8a48`, `0x6cd465`, `0x6cd554`, `0x6cd573`
Confidence: verified
Proof: one
Test: none

`0x6cd465` calls `0x5c3d90` with `l = 0`, `r` the `fild [esi+0xc]` float, `b = 0`, `t` the `fild
[esi+0x10]` float, `n = −1.0`, `f = 1.0`. The five `0x589e60` calls set states 0x14, 0xe, 0x10, 0x12
and 0xf to 0. `0x6cd554` to `0x6cd573` push `w/allocW` and `h/allocH` through `fidiv` and call
`0x58af60`.

## Full-screen quad

`0x6cd580` (`ecx` the destination `{w, h}`, `edx` the source's used `{w, h}`; the source's
allocated `{w, h}`, a position array, a UV array and a flag byte stacked; `ret 0x10`) writes a quad.
Positions are `(x0, y0)`, `(x0, y1)`, `(x1, y1)`, `(x1, y0)`, z 0: `(0, 0)` to `(w, h)` when the
device word `+0x1fc` is nonzero, else `(0.5, −0.5)` to `(w + 0.5, h − 0.5)`; with the flag byte set
and `gxMultisample` 2 or more, `y0 = h + 0.5` and `y1 = 0.5`. The UV pairs are `(hu, V)`, `(hu, hv)`,
`(U, hv)`, `(U, V)`, with `U = (sw + 0.5)/allocW`, `V = (sh + 0.5)/allocH` and `hu`, `hv` half a texel.

Addresses: `0x6cd580`, `0x6cd750`, `0x589ca0`, `0x6cd5a2`, `0x6cd5ae`, `0x6cd5e0`, `0x6cd695`, `0x6cd6ba`, `0x6cd6d4`, `0x6cd7a3`, `0x6cd7b8`, `0x6cd7c0`, `0xce8a98`, `0xce8ab8`
Confidence: verified
Proof: one
Test: `ffxeffects::ffx_fullscreen_quad_difftest`, `ffxeffects::ffx_transform_quad_verts_difftest`

`0x6cd750` (`ecx` the positions, `edx` the offsets, then the colour, a UV table of 4 pairs and a scale pair) builds a 4-vertex buffer: position, colour and four UV sets `uv·scale + offset`. `0x6cd5a2` calls `0x589ca0`, which returns the device word `+0x1fc`: 1 from the Direct3D device's constructor (`0x598d09`), 0 from the OpenGL one (`0x58deed`), 2 from the base's (`0x591af3`). `0x6cd5ae` and `0x6cd5bc` subtract and add 0.5; the multisample flip
is `0x6cd5d5` to `0x6cd5f8`, where the float at `0x86f268` is 1.0. `1/allocW` is kept at 53 bits
(`0x6cd6ab`) and `1/allocH` is stored as a float (`0x6cd6ba`), so `U` and `hu` use the first and `V`
and `hv` the second. The eight UV floats go out in the order `hu, V, hu, hv, U, hv, U, V`
(`0x6cd6f1` to `0x6cd73e`). In `0x6cd750` each UV is `fld; fmul scale; fadd offset; fstp`, one rounding
(`0x6cd7a3` to `0x6cd7c6`); a vertex is `0x30` bytes. Each test runs the exe's floating-point
instructions between a load and a store.

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
as a third source. It draws the quad with three UV sets: the wave map's, transformed by `R(10°, Z)
· S(W/128, 0.88·H/128) · T(p1, p2)` in row-vector order, the scene's and the blur's. `p1` is `(t mod
3174)/3174` and `p2` is `(t mod 2805)/2805` for the clock `t` in milliseconds. The colour is the Glow
pass's, but without the player check. It sets two program constants: with `0xce89f8` zero, `screenMtx
= (3/allocW, 0, 0, 3/allocH)` of the scene target and `blurMtx = (0.75/allocW, 0, 0, 0.75/allocH)`
of the blur target; with it set, `(3, 0, 0, 3)` and `(0.75, 0, 0, 0.75)`.

Addresses: `0x6cb310`, `0x6cb1f0`, `0x42c010`, `0x7bdc40`, `0x7bdca0`, `0x7bdd60`, `0x7bca80`, `0x6cb43a`, `0x6cb443`, `0x6cb467`, `0x6cb4e6`, `0x6cb6fa`, `0x6cb72f`, `0x6cb77a`, `0x6cb7c2`, `0x58b3c0`, `0x58b4d0`, `0xce89bc`, `0xce89c0`, `0xce89c4`, `0xce89c8`
Confidence: verified
Proof: one
Test: `ffxeffects::ffx_glow_pass_render_difftest`

`0x6cb43a` reads the clock, `div [0xce89c4]` (3174) and `div [0xce89c8]` (2805, of the same `t`);
`fild qword; fidiv` give `p1` and `p2`, stored as floats (`0x6cb477`, `0x6cb483`). `0x7bdc40`
translates by `(p1, p2, 0)`, `0x7bdca0` scales the rows by `sx = W·[0xce89bc]/128` and `sy =
H·[0xce89c0]/128` with `[0xce89bc] = 1.0` and `[0xce89c0] = 0.88` (`0x6cb48e` to `0x6cb4bd`; `W`
and `H` are the target's `+0xc` and `+0x10`), and `0x7bdd60` multiplies on the left by the rotation of
`0x3e32b8c3` (10° as a float) about `(0, 0, 1)`; the wave UVs go through `0x7bca80`. 3.0 is the
float at `0x81147c`. `0x6cb25e` and `0x6cb272` look up the program constants `screenMtx` and
`blurMtx` by name. The player lookup at `0x6cb58d` is not tested for null: `0x6cb592` adds `0xe68` to
the result. The test runs the phase, the scroll and the 3.0 scale.

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

## Pass list container

The pass lists are growable arrays of pass pointers with the tag `.PAVPass@FFX@@`: `0x6cc940`
(`ecx` the list, an extra count and a flag) grows the capacity to hold `count + n`, rounded up to a
multiple of the quantum; `0x6cca30` sets the quantum (0x40 when the request is 0x40 or more, else the
largest power of 2 not above it, at least 1); `0x6cca70` rounds a size up to a multiple; `0x6cca90`
reallocates to a capacity, keeping the first `min(old, new)` pointers.

Addresses: `0x6cc940`, `0x6cca30`, `0x6cca70`, `0x6cca90`, `0x6cc968`, `0x6cc999`, `0x6cc9a1`, `0x6cc9c1`, `0x6cc9d6`, `0x6cc9f4`
Confidence: verified
Proof: one
Test: none

`0x6cc968` compares with 0x40 and stores 0x40 at `+0xc`; `0x6cc976` to `0x6cc97d` clears the lowest
set bit until one remains; `0x6cc999` is `div ecx` and `0x6cc9a1` adds `quantum − remainder`.
`0x6cc9c1` calls the reallocator `0x646320`, `0x6cc9d6` the allocator when the reallocation fails,
and the loop at `0x6cc9f4` copies the pointers.

## Static initializers

The twelve initializer slots each hold a thunk that calls a body and registers an exit stub with
`_atexit`. The bodies set: the wave map descriptor fields (`0x6cbbd0`); the glow wave periods
`0xce89c4 = 3174` and `0xce89c8 = 2805` (`0x6cbc40`); the scales `0xce89bc = 1.0` and `0xce89c0 =
0.88` (`0x6cbca0`); the screen descriptor (`0x6ccb30`); the full and twin targets (`0x6ccba0`), the half
pair (`0x6ccc10`) and the quarter pair (`0x6ccc80`); the position array `0xce8ab8` (`0x6cccf0`) and
UV array `0xce8a98` (`0x6ccd50`); the draw descriptor `0xce8a88` (`0x6ccdb0`); and the two identity
matrices `0xce8a48` and `0xce8a08` (`0x6ccdf0`, `0x6ccee0`).

Addresses: `0x6cbbc0`, `0x6cbc30`, `0x6cbc90`, `0x6ccb20`, `0x6ccb90`, `0x6ccc00`, `0x6ccc70`, `0x6ccce0`, `0x6ccd40`, `0x6ccda0`, `0x6ccde0`, `0x6cced0`, `0x6cbbd0`, `0x6cbc40`, `0x6cbca0`, `0x6ccb30`, `0x6ccba0`, `0x6ccc10`, `0x6ccc80`, `0x6cccf0`, `0x6ccd50`, `0x6ccdb0`, `0x6ccdf0`, `0x6ccee0`, `0xce8a88`, `0xce8a98`, `0xce8ab8`, `0x82cdb4`, `0x82cde0`, `0x6ccb60`, `0x6ccb70`, `0x6ccbd0`, `0x6ccbe0`, `0x6ccc40`, `0x6ccc50`, `0x6cccb0`, `0x6cccc0`, `0x6ccd10`, `0x6ccd20`, `0x6ccd70`, `0x6ccd80`, `0x6ccea0`, `0x6cceb0`, `0x6ccf90`, `0x6ccfa0`, `0x6cbc00`, `0x6cbc10`, `0x6cbc60`, `0x6cbc70`, `0x6cbcc0`, `0x6cbcd0`
Confidence: verified
Proof: one
Test: none

Each thunk is `call body; jmp` the registrar (`push stub; call 0x409aef; pop; ret`), except
`0x6ccda0`, a `jmp` to its body. The stubs (`0x6cbc10`, `0x6ccb70` and the like) set one bit of a
flag byte (`0xce899c`, `0xce8b94`, `0xce8b58`) and do nothing else. The draw descriptor holds 4, 0,
the words 4 and 0, and 3, which `0x58a830` takes as counts and the strip type. The slot run
`0x82cdb4` to `0x82cde0` is followed by `0x82cde4`, which opens `0x6cdf50`'s file; the slot below
it, `0x82cdb0`, is `0x6ca170`, in the Warden file. The file's extent rests on its variables: a scan of
the listing and of every dword of the image finds 331 references to `0xce898c` through `0xce8bb4`, all
in `0x6ca9d0` to `0x6cdf20`; the control is `0x6ca960`, the last Warden function, which reads
variables just below them.
