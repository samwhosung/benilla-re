# ui: Slider and StatusBar

How a `Slider` keeps and steps its value and follows the mouse, and how a `StatusBar` fills. The
two share the shape of their fields but no code. What each reads from XML is in `xmltypes`.

A `CSimpleSlider` is a frame with the vtable 0x81cf60 whose fields are the flags `+0x314` (1
the thumb needs placing, 2 a range is set, 4 a value is set, 8 the mouse is dragging), the minimum
`+0x318`, the span `+0x31c` (maximum minus minimum), the value `+0x320`, the step `+0x324`, the
thumb texture `+0x328`, the orientation `+0x32c` (0 `HORIZONTAL`, 1 `VERTICAL`) and the
`OnValueChanged` slot `+0x330`. A `CSimpleStatusBar` has the vtable 0x81ca68: the same flags, the
minimum `+0x318` and maximum `+0x31c`, the value `+0x320`, the bar texture `+0x324`, the
orientation `+0x328` and `OnValueChanged` at `+0x32c`.

Roots: `0x789400`, `0x782d80`, `0x790e80`, `0x7900a0`

## Construction

A slider starts `VERTICAL`, with no range, no value and no step, and takes mouse input whatever
its scripts. A status bar starts `HORIZONTAL` with no range and no value.

Addresses: `0x789400`, `0x78943e`, `0x789456`, `0x789467`, `0x782d80`, `0x782d8f`, `0x782da4`, `0x6f1a20`
Confidence: verified
Proof: one
Test: none

`0x789400` zeroes `+0x318` to `+0x328` and `+0x330`, stores 1 at `+0x32c` (`0x78943e`) and calls
`0x76af00(2, −1)` (`0x789456`, `0x789467`). `0x782d80` clears bits 0 to 2 of `+0x314` (`0x782d8f`)
and zeroes `+0x324` to `+0x330` (`0x782da4`). `0x6f1a20` maps `HORIZONTAL` to 0 and `VERTICAL` to 1
from the pairs at 0x811b00.

## A slider's value

`SetValue` does nothing until a range is set. It clamps the value to the range and, with a nonzero
step, rounds it to the nearest multiple of the step above the minimum, halves away from the
minimum. When a value was set before and the result equals it, nothing happens; otherwise it is
stored and `OnValueChanged` runs with it, so the first `SetValue` always runs it. Setting the range,
or the step once a range is set, applies the current value again.

Addresses: `0x789930`, `0x78993e`, `0x789947`, `0x789962`, `0x789977`, `0x78999c`, `0x7899c9`, `0x7899ce`, `0x7899e5`, `0x7899fa`, `0x789a06`, `0x789a16`, `0x789a1e`, `0x789a44`, `0x7898f0`, `0x789905`, `0x789913`, `0x789927`, `0x789a60`, `0x789a91`, `0x835160`
Confidence: verified
Proof: one
Test: none

`0x789930` returns without bit 2 (`0x78993e`). It takes the larger of the minimum and the value
(`0x789947`), then the maximum `+0x318 + +0x31c` when below the result (`0x789962`, `0x789977`).
With `+0x324` nonzero (`0x78999c`) it computes `d = v − min` and `trunc((d + step/2)/step)` for
`d > 0` (`0x7899ce`), `trunc((d − step/2)/step)` otherwise (`0x7899e5`), times the step plus the
minimum (`0x7899fa`). With bit 4 an equal value returns (`0x789a16`); else it stores it
(`0x789a1e`) and runs `+0x330` with `"%f"` (`0x789a44`). `0x7898f0(min, max)` stores the minimum
and `max − min` (`0x789905`) and, with bit 4, calls `0x789930` (`0x789927`); `0x789a60` stores the
step and calls `0x7898f0` with a range (`0x789a91`).

## The slider's thumb

On its update, a slider with a range, a value and a thumb places the thumb when asked: its
fraction `(value − minimum)/span`, 0 for a span below 2^−22, moves the thumb's `LEFT` from the
slider's `LEFT` by the fraction of the slider's width less the thumb's, or for a vertical slider
the thumb's `TOP` down from the slider's `TOP` by the fraction of the height less the thumb's.

Addresses: `0x789aa0`, `0x789ab6`, `0x789ac6`, `0x789ad4`, `0x789af2`, `0x789b0e`, `0x789b2a`, `0x789b3e`, `0x789b52`, `0x789b75`, `0x789b7f`, `0x789b8a`, `0x789b8f`, `0x8029d4`
Confidence: verified
Proof: one
Test: none

`0x789aa0` (slot 14) requires bits 1, 2 and 4 and `+0x328` (`0x789ab6` to `0x789ad4`). It compares
`|span|` with 2^−22 at `0x8029d4` (`0x789af2`) and divides `value − min` by the span (`0x789b0e`).
Horizontal (`0x789b2a`) multiplies `(right − left)/scale − thumb width` by it for point 3 to point 3
(`0x789b3e`, `0x789b52`); vertical negates `(top − bottom)/scale − thumb height` times it for point
1 to point 1 (`0x789b75`, `0x789b7f`), through `0x767c70` (`0x789b8a`). It clears bit 1
(`0x789b8f`).

## Dragging a slider

A press anywhere in the slider starts a drag and sets the value at once, as if the thumb's centre
were under the cursor; there is no paging and no grab offset. While dragging, each mouse move
sets the value to the minimum plus the span times the cursor's position along the track, from half
a thumb inside one end to half a thumb inside the other, through `SetValue`'s clamp and step. A
release or a hide ends the drag.

Addresses: `0x789ca0`, `0x789cb2`, `0x789cbc`, `0x789cc2`, `0x789ba0`, `0x789ba6`, `0x789bf5`, `0x789bfe`, `0x789c24`, `0x789c47`, `0x789c4c`, `0x789c55`, `0x789c5d`, `0x789cd0`, `0x789cd3`, `0x789c90`
Confidence: verified
Proof: one
Test: none

`0x789ca0` (slot 26) sets bit 8 (`0x789cb2`), calls slot 0x3c with the event (`0x789cbc`) and then
`0x76bf70` (`0x789cc2`). `0x789ba0` (slot 15), with bit 8 and a thumb, takes `x − (left + thumb
width·0.5)` (`0x789bf5`, `0x789bfe`) or `top − thumb height·0.5 − y` (`0x789c24`), divides by the
track length less the thumb (`0x789c47`), multiplies by the span and adds the minimum (`0x789c4c`)
for `0x789930` (`0x789c55`), then calls `0x76b320` (`0x789c5d`). `0x789cd0` (slot 27) and
`0x789c90` (slot 13) clear bit 8.

## A status bar's value

`SetMinMaxValues(a, b)` stores the smaller of the two as the minimum and `b` as the maximum, so a
reversed pair gives an empty range at `b`, and runs `OnValueChanged` again for the current value.
`SetValue` clamps to the range, with no step; an unchanged value does nothing, any other is stored
and `OnValueChanged` runs with it. The fill is `(value − minimum)/(maximum − minimum)`, or 0 for an
empty range.

Addresses: `0x783270`, `0x783276`, `0x78327e`, `0x78328e`, `0x7832a5`, `0x7832af`, `0x7832bc`, `0x7832d0`, `0x7832dc`, `0x7832e2`, `0x7832fd`, `0x78332f`, `0x783342`, `0x783371`, `0x783380`, `0x783394`
Confidence: verified
Proof: one
Test: none

`0x783270` compares `a` with `b` and keeps `a` when not above (`0x783276`, `0x78327e`), stores `b`
at `+0x31c` (`0x78328e`) and, with bit 4 (`0x7832a5`), clears it (`0x7832af`) and calls slot 0x90,
`0x7832d0`, with the value (`0x7832bc`). `0x7832d0` requires bit 2 (`0x7832dc`), clamps
(`0x7832e2`, `0x7832fd`), returns for an equal value with bit 4 (`0x783342`) and runs `+0x32c`
with `"%f"` (`0x783371`). `0x783380` tests `max − min` against 0 (`0x783394`).

## A status bar's fill

The bar texture keeps its full rectangle in the frame but draws only the filled part: on its
update, a status bar with a range, a value and a texture sets the texture's coordinates to 0 to
the fill across and 0 to 1 down when `HORIZONTAL`, or 0 to 1 across and 0 to the fill down when
`VERTICAL`. Installing a bar texture sets it to change its rectangle with its coordinates.

Addresses: `0x7833c0`, `0x7833d8`, `0x7833e4`, `0x7833fa`, `0x78340e`, `0x78341e`, `0x78342f`, `0x783436`, `0x783449`, `0x770410`, `0x7831f0`, `0x783220`
Confidence: verified
Proof: one
Test: none

`0x7833c0` (slot 14) requires bits 1, 2 and 4 and `+0x324` (`0x7833d8`, `0x7833e4`). With `+0x328`
0 (`0x7833fa`) it builds `{0, 0, 1, fill}` (`0x78340e`, `0x78341e`), else `{0, 0, fill, 1}`
(`0x78342f`, `0x783436`), and calls `0x770410` (`0x783449`), which stores the four corners'
coordinates as (left, top) = (`[+4]`, `[+0]`) to (right, bottom) = (`[+0xc]`, `[+8]`). `0x7831f0`
stores 1 at the texture's `+0x124` (`0x783220`).

## Lua methods

`Slider` has 10 methods at `0x87b260`, registered by `0x790e80`; `StatusBar` has 10 at `0x87b010`,
registered by `0x7900a0`. Each type then asks the `Frame` methods.

Addresses: `0x87b260`, `0x790e80`, `0x790eb0`, `0x87b010`, `0x7900a0`, `0x7900d0`, `0x7901c0`, `0x7902b0`, `0x7904c0`, `0x790580`, `0x7906e0`, `0x7907a0`, `0x7908c0`, `0x790980`, `0x790a70`, `0x790b30`, `0x78f1d0`, `0x78f290`, `0x78f3f0`, `0x78f4b0`, `0x78f5d0`, `0x78f690`, `0x78f780`, `0x78f870`, `0x78fab0`, `0x78fc20`
Confidence: verified
Proof: one
Test: none

The tables are pairs from `GetThumbTexture` (`0x7901c0`) to `SetValueStep` (`0x790b30`), and from
`GetOrientation` (`0x78f1d0`) to `SetStatusBarColor` (`0x78fc20`); the registrars call `0x701d80`
with 0xa. `0x790eb0` and `0x7900d0`, slot 2, look in the type's hash and then call `0x778590`.
`SetOrientation` (`0x790580`) parses its name with `0x6f1a20` and calls `0x7898c0`.
