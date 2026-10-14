# screencoord

The screen aspect scale: four dwords set from the display's aspect ratio, and eleven small
functions that multiply or divide interface coordinates by them.

The object file is the eleven functions from `0x41ad10` through `0x41ae70`. It has no initializer
in the C runtime's run, no vtable and no source path, and it calls nothing. The four globals
`0x832a40`, `0x832a44`, `0x832a48` and `0x832a4c` are read or written only inside it: a scan of
the listing for each, and of the image for each little-endian dword, finds 2, 9, 9 and 2 places,
all in `0x41ad10` to `0x41ae7d`. Static-initializer thunks of other files stand on both sides of
it. Two callers set the aspect, glue and the interface; 446 other call sites scale, unscale or
read through it. Three functions have no caller.

Roots: `0x41ad10`, `0x41ad70`, `0x41ad80`, `0x41ade0`, `0x41ae40`, `0x41ae50`, `0x41ae60`,
`0x41ae70`

## Screen scale state

`0x41ad10` takes the display aspect `a`, a float, on the stack (`ret 4`) and sets four floats. With
`RN24` rounding to float and `L = 1/√(a² + 1)` computed at 53 bits:

| global | value |
|---|---|
| `0x832a40` | `a`, the dword as passed |
| `0x832a44` | `K1 = RN24(a·L)` |
| `0x832a48` | `K2 = RN24(L)` |
| `0x832a4c` | `RN24(0.75·a)` |

`(K1, K2)` is the unit vector along `(a, 1)`. The file holds the values for 4/3: `a` 1.3333334,
`K1` 0.8, `K2` 0.6, the last 1.0. `0x41ad60` returns the first and `0x41ad70` the last in `st0`.
`K1` multiplies by the unrounded `L` still in `st0` after `K2` is stored.

Addresses: `0x41ad10`, `0x41ad60`, `0x41ad70`, `0x832a40`, `0x832a44`, `0x832a48`, `0x832a4c`, `0x46a828`, `0x48fe2b`
Confidence: verified
Proof: one
Test: `screencoord::sc_41ad10_set_scale`

`0x41ad13` to `0x41ad24`: `fld a; mov eax,a; fmul 0.75; mov [0x832a40],eax; fstp dword [0x832a4c]`.
`0x41ad2a` to `0x41ad47`: `fld a; fmul a; fadd 1.0; fsqrt; fdivr 1.0` (`d8 3d`, `st0 = 1.0 / st0`);
`fst dword [0x832a48]` (`0x41ad3e`) stores without popping, so `fmul a; fstp dword [0x832a44]`
multiplies the 53-bit `L`, not `K2`. `a²` of a float is exact at 53 bits. The test runs `0x41ad10`
whole over exact ratios and random floats. Callers: glue at `0x46a828` with the aspect `0x46ab40`
gives (`glue/manager#screen-size-and-aspect`), and the interface start-up at `0x48fe2b` with the
aspect `0x4901e0` gives, which it first stores at `0xb4e30c`. The getters `0x41ad60` and `0x41ad70`
are one `fld dword` and `ret`.

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
pointer forms store each result rounded to float. Nothing tests for a zero scale. `0x41adb0` and
`0x41ae10` have no caller and are in the next entry.

Addresses: `0x41ae60`, `0x41ae70`, `0x41ae40`, `0x41ae50`, `0x41ad80`, `0x41ade0`, `0x41adb0`, `0x41ae10`, `0x832a44`, `0x832a48`
Confidence: verified
Proof: one
Test: `screencoord::sc_41ae60_scale_x`, `screencoord::sc_41ae70_scale_y`, `screencoord::sc_41ae40_unscale_x`, `screencoord::sc_41ae50_unscale_y`, `screencoord::sc_41ad80_scale_xy`, `screencoord::sc_41ade0_unscale_xy`

`0x41ae63` `fld [0x832a44]; fmul [ebp+8]`, `0x41ae73` the same on `0x832a48`; `0x41ae46`
`fld [ebp+8]; fdiv [0x832a44]`, `0x41ae56` on `0x832a48`. `0x41ad85` `je` skips the `x` store on a
null `ecx`, `0x41ad94` the `y` store on a null `edx`; `0x41ade5` and `0x41adf4` do the same. The
rectangle forms are `fld; fmul` or `fdiv; fstp` four times with no frame. Each test runs its
function whole against random operands, the divisors held at least 0.05 from zero.

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
