# screencoord

The screen aspect scale: four dwords set from the display's aspect ratio, eleven small functions
that multiply or divide coordinates by them, and the conversion between script units and layout
units that every script function builds from them.

The object file is the eleven functions from `0x41ad10` through `0x41ae70`. It has no initializer
in the C runtime's run, no vtable and no source path, and it calls nothing. The four globals
`0x832a40`, `0x832a44`, `0x832a48` and `0x832a4c` are read or written only inside it: a scan of
the listing for each, and of the image for each little-endian dword, finds 2, 9, 9 and 2 places,
all in `0x41ad10` to `0x41ae7d`. Static-initializer thunks of other files stand on both sides of
it. Two callers set the aspect, glue and the interface; 446 other call sites scale, unscale or
read through it. Three functions have no caller. The script conversions are inlined at each call
site; the last entries state them and the units the layout works in.

Roots: `0x41ad10`, `0x41ad70`, `0x41ad80`, `0x41ade0`, `0x41ae40`, `0x41ae50`, `0x41ae60`,
`0x41ae70`

## Screen scale state

`0x41ad10` takes the display aspect `a`, a float, on the stack (`ret 4`) and sets four floats. With
`RN24` rounding to float and `L = 1/√(a² + 1)` computed at 53 bits:

| global | value |
|---|---|
| `0x832a40` | `a`, the dword as passed |
| `0x832a44` | `K1 = RN24(a·L)`, the screen's width in layout units |
| `0x832a48` | `K2 = RN24(L)`, the screen's height in layout units |
| `0x832a4c` | `A = RN24(0.75·a)` |

`(K1, K2)` is the unit vector along `(a, 1)`, so the screen's diagonal is one layout unit. The
file holds 4/3's values except `K2`: `a` 1.3333334, `K1` 0.8 (`0x3f4ccccd`), `K2` 0.6
(`0x3f19999a`), `A` 1.0; a call with `RN24(4/3)` stores `K2` as `0x3f199999`. `0x41ad70` returns
`A` in `st0`.

Addresses: `0x41ad10`, `0x41ad70`, `0x832a40`, `0x832a44`, `0x832a48`, `0x832a4c`, `0x46a828`, `0x48fe2b`
Confidence: verified
Proof: two
Test: `screencoord::sc_41ad10_set_scale`

`0x41ad13` to `0x41ad24`: `fld a; mov eax,a; fmul 0.75; mov [0x832a40],eax; fstp dword [0x832a4c]`.
`0x41ad2a` to `0x41ad47`: `fld a; fmul a; fadd 1.0; fsqrt; fdivr 1.0` (`d8 3d`, `st0 = 1.0 / st0`);
`fst dword [0x832a48]` (`0x41ad3e`) stores without popping, so `fmul a; fstp dword [0x832a44]`
multiplies the 53-bit `L`, not `K2`. `a²` and `0.75·a` of a float are exact at 53 bits. The test
runs `0x41ad10` whole over exact ratios, zero, a negative aspect, +∞, NaN and random floats; NaN
results are compared as NaN. `0x41ad70` is one `fld dword` and `ret`.

## When the cells change

Only the two callers of `0x41ad10` change the cells, each with the aspect it reads then:
`RN24(w/h)` of `gxResolution` while `widescreen` is nonzero (its default is `1`), else `RN24(4/3)`.
Glue passes it at every glue build (`0x46a7b0`): start-up, the rebuild that a window aspect change
of 0.01 or more requests, the return from the world and a disconnect. The interface passes it at
every build (`0x48fbf0`): the game's start and every reload, which a window aspect change of 0.001
or more requests. `widescreen` has no change callback, and nothing on the `uiScale` path writes a
cell. The in-game size path caches the new aspect after asking for the reload, whether or not the
protected-action gate let the reload through.

Addresses: `0x41ad10`, `0x46a7b0`, `0x46a81c`, `0x46a828`, `0x48fbf0`, `0x48fe1e`, `0x48fe2b`, `0x46ab40`, `0x4901e0`, `0x63a74f`, `0x46a505`, `0x46b99d`, `0x402069`, `0x46c613`, `0x48f681`, `0x495669`, `0x492f31`, `0x492f33`, `0x492f38`, `0x493038`, `0x49303a`, `0x49303f`, `0x491380`, `0xb4e30c`
Confidence: verified
Proof: one
Test: none

`0x41ad10` has two callers, `0x46a828` in `0x46a7b0` and `0x48fe2b` in `0x48fbf0`, and no stored
address (the controls `0x41ad70` and `0x7a1e00` find 162 calls and one stored address). Both push
the float that `0x46ab40` or `0x4901e0` returned and the caller stored with `fstp dword`
(`0x46a81c`, `0x48fe1e`): `fild` width, `fidiv` height of `gxResolution`'s `%d%c%d`, or 4 and 3.
`0x46a7b0` is called at `0x46a505` (start-up), `0x46b99d` (`glue/manager#rebuild-requests`),
`0x402069` and `0x46c613`; `0x48fbf0` at `0x48f681`, the tail of `0x48f480`, and `0x495669`
(`ui/bindings#reloading-the-interface`). `0x63a720` registers `widescreen` with a null callback
(the `push 0` before the default `1`, then `0x63a74f`). `0x492e90` and `0x492f70` test `|cached − new|` against 0.001 (`jnp 0x492f61` at
`0x492f31`, `jnp 0x493048` at `0x493038`, which skip on less); past it they call `0x491380` and
then store the new aspect at `0xb4e30c`, whatever the call did (`0x492f33`, `0x492f38`;
`0x49303a`, `0x49303f`).

## Scaling and unscaling

Eight functions convert a coordinate between layout units and raw ones, x by `K1` and y by `K2`:

| function | takes | result |
|---|---|---|
| `0x41ae60` | `x` on the stack, `ret 4` | `K1·x` in `st0` |
| `0x41ae70` | `y` on the stack, `ret 4` | `K2·y` in `st0` |
| `0x41ae40` | `x` on the stack, `ret 4` | `x/K1` in `st0` |
| `0x41ae50` | `y` on the stack, `ret 4` | `y/K2` in `st0` |
| `0x41ad80` | `ecx` and `edx` pointers, `x`, `y` on the stack, `ret 8` | stores `K1·x` through `ecx` and `K2·y` through `edx`, each only when its pointer is not null |
| `0x41ade0` | the same | stores `x/K1` and `y/K2`, each only when its pointer is not null |
| `0x41adb0` | `ecx` a rectangle of four floats, `edx` the output | writes `K1·r[0]`, `K1·r[2]`, `K2·r[1]`, `K2·r[3]` in that order |
| `0x41ae10` | the same | writes `r[0]/K1`, `r[2]/K1`, `r[1]/K2`, `r[3]/K2` in that order |

The single-value functions return the 53-bit product or quotient without rounding to float; the
pointer forms store each result rounded to float. Nothing tests for a zero scale: a zero divisor
gives an infinity, or NaN for a zero argument. `0x41adb0` and `0x41ae10` have no caller and are in
the last entry.

Addresses: `0x41ae60`, `0x41ae70`, `0x41ae40`, `0x41ae50`, `0x41ad80`, `0x41ade0`, `0x41adb0`, `0x41ae10`, `0x832a44`, `0x832a48`
Confidence: verified
Proof: one
Test: `screencoord::sc_41ae60_scale_x`, `screencoord::sc_41ae70_scale_y`, `screencoord::sc_41ae40_unscale_x`, `screencoord::sc_41ae50_unscale_y`, `screencoord::sc_41ad80_scale_xy`, `screencoord::sc_41ade0_unscale_xy`

`0x41ae63` `fld [0x832a44]; fmul [ebp+8]`, `0x41ae73` the same on `0x832a48`; `0x41ae46`
`fld [ebp+8]; fdiv [0x832a44]`, `0x41ae56` on `0x832a48`. `0x41ad85` `je` skips the `x` store on a
null `ecx`, `0x41ad94` the `y` store on a null `edx`; `0x41ade5` and `0x41adf4` do the same. The
rectangle forms are `fld; fmul` or `fdiv; fstp` four times with no frame. Each test runs its
function whole. The single-value tests read `st0` through an `fstp qword`, over the 4:3 and 16:9
scales, zero, ±∞, NaN and random scales against arguments that include ±0, ±∞, NaN and the
float extremes; NaN results are compared as NaN. The pointer tests keep the divisor at least
0.05 from zero.

## Script units and layout units

A script getter answers a layout length `l` as `RN24(1024·A·l) / K1`, the quotient at 53 bits
pushed as a double, and a setter stores a script number `v` as `RN24(K1 · RN24(RN24(v) /
(1024·A)))`, on both axes, as `GetWidth` and `SetWidth` do. One layout unit is
`1024·A/K1 ≈ 768·√(a²+1)` script units, so the screen, `K1` by `K2` layout units, is about `768·a`
by 768. At `a = RN24(4/3)` a getter of `K2` answers 767.9999427795418 and of `K1` 1024.0 (with the
file's cells, 768.0000190734861 and 1024.0), and a stored 100 reads back as 99.99999850988391.
Every step runs at the 53-bit precision the client sets at start-up.

Addresses: `0x41ad70`, `0x7ffd68`, `0x41ae40`, `0x41ae60`, `0x832a4c`, `0x832a44`, `0x7a1e00`, `0x7a1f20`, `0x7a1ef4`, `0x7a1efe`, `0x7a1f01`, `0x7a1fcb`, `0x7a1fda`, `0x7a1fdd`, `0x7a1fe0`, `0x40410c`
Confidence: verified
Proof: two
Test: `ui::region_size_getters_whole`, `ui::region_size_setters_whole`, `ui::ui_screen_size_whole`

`GetWidth` (`0x7a1ef4` to `0x7a1f01`): `call 0x41ad70; fmul dword [0x7ffd68]` (1024.0, `00 00 80
44`, exact on `A`); `fmul [ebp-4]`; `fstp dword [esp]`; `call 0x41ae40`, which divides by `K1` and
returns `st0`; `fstp qword` for the push. `SetWidth`: `fstp dword [ebp-4]` of the number
(`0x7a1fcb`); `call 0x41ad70; fmul [0x7ffd68]`; `fdivr [ebp-4]` (`d8 7d`, `st0 = m32 / st0`,
`0x7a1fda`); `fstp dword [esp]`; `call 0x41ae60` (`0x7a1fe0`); `fstp dword`. The precision is
`0x40410c`'s `__control87` (`METHOD#the-harness`). The screen root's rectangle is `K1` by `K2`
(`0x764284`, `ui/regions#layout-space-and-units`). The values apply the formulas; the tests run
the methods whole over six aspects.

## The script conversion census

The steps above are the only way the script functions combine `A` with 1024: every such site
divides or multiplies by `K1`, so `K2` enters no script conversion and the y axis converts as x
does.

Addresses: `0x41ad70`, `0x7ffd68`, `0x41ae40`, `0x41ae60`, `0x41ae50`, `0x41ae70`, `0x490e60`, `0x7a18e8`, `0x7a1923`, `0x4066b9`, `0x406ac4`, `0x48c306`, `0x78d1ad`
Confidence: verified
Proof: one
Test: none

`0x41ad70` has 162 call sites; 158 are followed by `fmul dword [0x7ffd68]`. At 67 the next steps
end in `fstp dword [esp]` and `call 0x41ae40`: `fmul m32` at 57, `fld e; fdiv s; fmulp` at 5,
`fld m32; fsub m32; fdiv m32; fmulp` at 3 (all in `0x490e60`) and the centre's `fld far; fsub
near; fmul 0.5; fadd near; fdiv s; fmulp` at 2 (`0x7a18e8`, `0x7a1923`). At 91 they are `fdivr
m32`, `fstp dword [esp]` and `call 0x41ae60`, the operand a script number already stored with
`fstp dword`, or a constant. The other four, `0x4066b9`, `0x406ac4`, `0x48c306` and `0x78d1ad`,
use `A` alone. No site pairs `A` with `0x41ae50` or `0x41ae70`.

## Layout units in the resolver

The layout keeps sizes and anchor offsets in the region's own layout units, as the setters store
them. The resolver multiplies each by the layout scale, the frame's effective scale, so the
resolved rectangle is in screen layout units, where the screen is `K1` by `K2`. It rounds a size
times the scale to a float, each neighbouring edge to a float as it is passed, and each edge to a
float as it is stored in the rectangle; a point's `scale·offset + edge` stays at 53 bits until
then. Nothing in it reads the cells except the clamp to the screen.

Addresses: `0x7673d0`, `0x767414`, `0x76741f`, `0x767428`, `0x7671a0`, `0x767a20`, `0x767a30`, `0x767aea`, `0x767b19`, `0x41ae60`, `0x41ae70`
Confidence: verified
Proof: one
Test: `ui::layout_composed_resolve`, `ui::anchor_resolve_x`, `ui::layout_combine_edge`

`0x7673d0` (left) calls slot 0x1c and `fmul [esi+0x58]`, then `fstp dword [esp]` (`0x767414`) for
the size argument, and stores the right edge and the centre the same way (`0x76741f`, `0x767428`).
`0x7671a0` passes `[edi+0x58]` to each point's coordinate slot and keeps its `st0` result. `0x767a20`
stores each edge with `fst dword` (`0x767a30`). The point and edge arithmetic is
`ui/anchors#relative-point-coordinates` and `ui/anchors#edge-resolution-from-size`; the layout
scale is `ui/visibility#scale`; the clamp calls `0x41ae60(1.0)` and `0x41ae70(1.0)`
(`ui/anchors#rectangle-calculation-and-screen-clamp`), at `0x767aea` and `0x767b19`, the only calls
of the eight helpers from `CLayoutFrame.cpp` and the relative point getters.

## Unreached code

`0x41ad60`, `0x41adb0` and `0x41ae10` are unreached: no `call` or `jmp` in the listing targets
them, and no dword of the image holds their addresses. The control is `0x41ad70`, which a scan of
the same kind finds at 162 call sites. Nothing falls into them: each follows padding.

Addresses: `0x41ad60`, `0x41adb0`, `0x41ae10`, `0x41ad70`
Confidence: verified
Proof: one
Test: none

The scan matches `call`, `jmp` and every `jcc` to the address in the listing, and every aligned
or unaligned dword in all sections of the image.
