# core: the frame pass

The frame pass draws one frame. It is the one handler of event category 0x11, which the loop
dispatches last in each tick of the running context (`core/loop#what-one-tick-runs`). It walks the
screen layers from the lowest priority to the highest and calls each visible layer's callback with
the time since the last pass, then presents. The interface's top is a layer (`ui/strata`), and the
3D world draws inside the top's draw walk (`ui/worldframe#drawing-the-world`), so the interface's
`OnUpdate` walk, the world and the interface's paint all run inside this pass.

The code is the screen-layer object file around `0x442350`, which this repo does not own: its rows
stay `unassigned`, and the client enters it at `0x442170`, `0x442140`, `0x442800` and `0x442190`.
Its state is the layer list `0x885610` and the elapsed time `0x885608`. A layer is a `ScreenLayer`
of 0x40 bytes. The pass clips the layers with the region calls at `0x65c610` and after, whose
object file is not owned here either.

Roots: none

## Registration

The client's start handler registers two functions through `0x442170`: the elapsed counter
`0x442330` on category 5, the frame time, and the frame pass `0x442350` on category 0x11, both with
argument 0 and priority 0.0. The stop handler removes both. No other registration in the image
names category 0x11, so the loop runs the pass once in each tick of a running context and not in a
closing one (`core/loop#one-tick`).

Addresses: `0x402840`, `0x402851`, `0x442170`, `0x4427b0`, `0x4427c4`, `0x4427c9`, `0x403640`, `0x40367e`, `0x442140`, `0x442310`, `0x41fc90`, `0x41fca0`, `0x41fc98`, `0x424f00`
Confidence: verified
Proof: one
Test: none

`0x402851` calls `0x442170`, which calls `0x4427b0`: `mov edx,0x442330; mov ecx,5; call 0x41fc90`,
then `mov edx,0x442350; mov ecx,0x11` and a tail jump to `0x41fc90` (`0x4427c9`). `0x40367e`
calls `0x442140`, which calls `0x442310`, the same two pairs for `EventUnregister`. A census of the
calls of `0x41fc90` (12) and `0x41fca0` (49, one of them `0x41fc98` inside `0x41fc90`), with
`tools/callers.py` and the control `0x420c00`, finds no stored address of either, and every site
loads its category into `ecx` as an immediate: 0x11 at `0x4427c4` alone. Handler records come only
from `0x424f00`, whose other callers are the context constructor's categories 7 and 4.

## Elapsed time

Each dispatch of category 5 adds the frame time to the float `0x885608`: the sum at 53 bits,
rounded once to a float, which equals the float sum. The pass hands `[0x885608]` to each layer
callback it calls and stores 0 after its present, called or not. So with one category-5 dispatch
before each pass, a layer gets the tick's frame time, and the time of a pass that skips a layer is
lost to it.

Addresses: `0x442330`, `0x885608`, `0x44233d`, `0x44263d`, `0x4426c3`
Confidence: verified
Proof: one
Test: `core::frame_time_accumulate_matches_binary`

`0x442330` is `fld dword [0x885608]; mov eax,1; fadd dword [ecx]; fstp dword [0x885608]; ret`: `ecx`
is the frame-time payload, whose first dword is the seconds (`core/loop#frame-time`). The listing
names `0x885608` at four instructions: the load and the store of `0x442330`, the load at `0x44263d`
and the store of 0 at `0x4426c3`. Two `f32` summed at 53 bits and rounded to 24 round once in
effect, since 53 ≥ 2·24 + 2. The test runs `0x442330` whole on finite sums of both signs, zero,
the infinities and both sides of the 53-bit rounding.

## Screen layers

`ScrnLayerCreate` (`0x442800`) makes a layer from a rectangle (null for the unit square), flags, a
priority, a parameter and a callback, and writes its handle. The list runs from the lowest priority
to the highest: a new layer goes after the last one, counting from the tail, whose priority is not
greater than its own, or at the head; so among equal priorities the newer comes later. Six calls
make the client's layers:

| layer | priority | flags | callback | made at |
|---|---|---|---|---|
| interface top | 1.0 | 4 | `0x764330` | `0x764264` |
| console | 6.0 | 3 | `0x63b5c0` | `0x63b538` |
| console | 7.0 | 3 | `0x63b800` | `0x63b558` |
| loading screen | 9.0 | 6 | `0x406a60` | `0x4068db` |
| system messages | 10.0 | 7 | `0x4092c0` | `0x4090d5` |
| screen fade | 10.0 | 7, 6 for 0 seconds | `0x4c0f50` | `0x4c0dac` |

Addresses: `0x442800`, `0x44280e`, `0x442810`, `0x44281f`, `0x442831`, `0x44285c`, `0x442862`, `0x442868`, `0x442885`, `0x442890`, `0x4428fa`, `0x44293a`, `0x442220`, `0x885610`, `0x802564`, `0x802574`, `0x764330`, `0x63b5c0`, `0x63b800`, `0x406a60`, `0x4092c0`, `0x4c0f50`
Confidence: verified
Proof: one
Test: `core::frame_pass_matches_binary`

`0x44281f` allocates 0x40 bytes tagged `HLAYER` and `0x442831` stores the vtable `0x802574`. A null
`ecx` takes the rectangle at `0x802564`, (0, 0, 1, 1) (`0x44280e`, `0x442810`); it lands at `+0x08`
as left, bottom, right, top. The priority goes to `+0x28` (`0x44285c`), `edx` to `+0x2c`
(`0x442862`), the parameter to `+0x30` and the callback to `+0x34` (`0x442868`). The walk from the
tail `fcomp`s each `+0x28` with the new priority and stops on `test ah,0x41; jne` (`0x442885`,
`0x442890`), not greater or NaN; the link at `+0x38` goes after the stop (`0x4428fa`). The handle
is stored through the fourth stack argument (`0x44293a`). `0x442220` sets the list empty with link
offset 0x38. The table reads the six calls, the census of `0x442800`'s callers; the fade's flags are
`ui/screenfade#starting-a-fade`, and the loading screen's bit 0 `loadingscreen/screen#raising-the-screen`.

## Layer occlusion

Before drawing, the pass walks from the tail with a region holding the unit square. A layer's
visible rectangle is the region's bounding rectangle clamped to its own; then, when its
flag bit 0 is clear, its rectangle is taken out of the region. The draw walk calls a layer only
when its visible rectangle has right above left and top above bottom. So a layer under one with
bit 0 clear that covers it is not called. The loading screen covers the unit square with bit 0
clear: while it is up the interface top is not called, so neither `OnUpdate`, the world nor the
interface's paint runs, unless an arrival aboard a transport has set its bit 0.

Addresses: `0x442362`, `0x44238f`, `0x4423bc`, `0x4423c1`, `0x442425`, `0x442437`, `0x44245f`, `0x4424b9`, `0x4424ca`, `0x65c610`, `0x65bd90`, `0x65ca80`, `0x65c710`, `0x65c565`, `0x65c5ac`, `0x65cb77`, `0x65cb90`
Confidence: verified
Proof: one
Test: `core::region_bounding_rect_matches_binary`, `core::frame_pass_matches_binary`

`0x442362` makes the region and `0x44238f` adds the unit square with mode 2. Per layer, `0x4423bc`
stores the bounding rectangle at `+0x18`; `test ah,5; jp` keeps the larger left and bottom and
`test ah,0x41; jne` the smaller right and top, the layer's own on a NaN (`0x4423c1` to `0x442422`).
Bit 0 (`0x442425`) skips the mode 4 combine of `+0x08` (`0x442437`); `0x44245f` frees the region.
`test ah,0x41; jnp` skips a layer whose right is not above its left (`0x4424b9`) or whose top is not
above its bottom (`0x4424ca`); a NaN is called. Mode 4 drops the pieces the split marks (table
`0x65c5ac`, case `0x65c565`), and `0x65ca80` takes the bounds of the pieces left, all four 0 when
they are empty (`0x65cb77` to `0x65cb90`). The tests run the region calls whole on rectangles of
quarters, and `0x442350` whole.

## Pass order

The pass reads the device viewport, runs the clip walk, then calls the visible layers from the head
as `fastcall(parameter, &rectangle)` with the visible rectangle and the elapsed time pushed. Before
each call it sets the viewport to the visible rectangle, or to the unit square with flag bit 2, and
with bit 1 an identity view and an orthographic projection of the visible rectangle from 0 to 500.
It then restores the viewport, runs the font frame, presents (`gx/capture`), clears the elapsed
time, and sleeps 250 ms when the device reports itself inactive. It returns 1.

Addresses: `0x442350`, `0x44247a`, `0x44247f`, `0x4424d0`, `0x4424fc`, `0x442501`, `0x442625`, `0x442630`, `0x442638`, `0x44264b`, `0x44266d`, `0x442672`, `0x4426c3`, `0x4426cd`, `0x4426db`, `0x58b060`, `0x58af60`, `0x5c3d90`, `0x58b050`, `0x58b040`, `0x5c2ce0`, `0x58a260`, `0x5923e0`, `0x429d50`
Confidence: verified
Proof: one
Test: `core::frame_pass_matches_binary`

`0x44247a` reads the viewport and `0x44247f` starts at the head `[0x885618]`. `0x4424d0` tests bit 2
for the viewport `0x58af60` (`0x4424fc`); bit 1 (`0x442501`) builds the projection with far 500.0
(`0x43fa0000`) at `0x442625` and loads the view and projection (`0x442630`, `0x442638`). The call
is `call [ebx+0x34]` with `ecx` `+0x30`, `edx` `&+0x08`, and `+0x18`'s address and `[0x885608]`
pushed (`0x44264b`). `0x44266d` restores the viewport and `0x442672` calls `0x5c2ce0`. After the
present block, `0x4426c3` clears the time; `0x58a260` returns the device's active flag `+0xf30`
(`0x5923e0`, `gx/slots#window-events`), and on 0 `0x4426db` calls `0x429d50(0xfa)`. The test runs
`0x442350` whole over layers `0x442800` linked, with the device calls stubbed, and checks the calls,
their order, rectangles and time, the cleared time and the sleep.

## A pass inside a pass

`0x442190` runs the pass at once, outside category 0x11. Its callers are the cinematic's shot start
and end, which are reached only from the callbacks of screen fades, and the fade layer's draw calls
those. So the
inner pass runs inside the outer pass's call of the fade layer: it calls the visible layers again,
the interface top included, with the same elapsed time, presents and clears the time. The outer
pass then calls the layers after the fade with 0 and presents again: with the top visible, two
`OnUpdate` walks and two presents in that tick.

Addresses: `0x442190`, `0x4427d0`, `0x48eedd`, `0x48f129`, `0x48edf0`, `0x48f080`, `0x48edd0`, `0x48efe0`, `0x48f050`, `0x4c0d10`, `0x4c120d`, `0x4c122d`
Confidence: verified
Proof: one
Test: none

`0x442190` jumps to `0x4427d0`, `xor edx,edx; xor ecx,ecx; jmp 0x442350`. The census of `0x442190`
finds `0x48eedd` in `0x48edf0` and `0x48f129` in `0x48f080`, and of `0x4427d0` the one jump. `0x48edf0`
has two callers, `0x48edd0` and `0x48efe0`; `0x48f080` is reached from those two and stored by
`0x48f050`. Each of `0x48edd0`, `0x48efe0` and `0x48f080` is reached otherwise only as the callback
`0x4c0d10` stores, and the fade's draw reads the callback at `0x4c120d`, its one reader, and calls
it at `0x4c122d`, after clearing the mode. The clear at `0x4426c3` comes after the present, so the
inner pass reads the time the outer one read.
