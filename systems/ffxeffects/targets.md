# ffxeffects: render targets and the quad

The render targets the passes draw between and the quad every pass draws: their sizes and
allocation, the draw state a pass sets up and the full-screen quad with its texture coordinates.

Roots: none

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
(`0x6cd7a3` to `0x6cd7c6`); a vertex is `0x30` bytes. `0x6cd580` has 10 call sites, all in this file.
