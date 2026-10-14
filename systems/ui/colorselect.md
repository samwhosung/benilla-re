# ui: ColorSelect

How a `ColorSelect` keeps its colour, converts it on the way in and out, places its two thumbs and
follows the mouse. The colour conversions themselves are `RgbToHsv` and `HsvToRgb`
(`lighting/daynight`, Underwater tint).

A `CSimpleColorSelect` is a frame with the vtable 0x81d108. Its fields are the wheel and value
drag flags `+0x314` and `+0x315`, the four textures `ColorWheelTexture` `+0x318`,
`ColorWheelThumbTexture` `+0x31c`, `ColorValueTexture` `+0x320` and `ColorValueThumbTexture`
`+0x324`, the colour as hue in degrees `+0x328`, saturation `+0x32c` and value `+0x330`, the colour
at full value `+0x334` for the value strip, and the `OnColorSelect` slot `+0x338`. It starts white:
hue 0, saturation 0, value 1. There is no alpha.

Roots: `0x78b220`, `0x78f0b0`

## Setting the colour

`SetColorRGB(r, g, b)` clamps each channel to 0 to 1, turns it into a byte by
`trunc(v·255 + 0.5)`, divides the bytes by 255 and converts them to hue, saturation and value. A
fifth argument is read and discarded. `SetColorHSV(h, s, v)` stores its three numbers as given,
unclamped. Both then redraw and run `OnColorSelect`, on every call, changed or not.

Addresses: `0x78eae0`, `0x78ec7d`, `0x78ec83`, `0x78ec89`, `0x78ecec`, `0x78ecfa`, `0x78ed01`, `0x7bbf20`, `0x8026c8`, `0x78e920`, `0x78e9b2`, `0x78e9de`, `0x78e9f1`, `0x7bbc80`, `0x78b220`
Confidence: verified
Proof: one
Test: `cmath::color_unpack_rgb`

`0x78eae0` clamps arguments 2 to 4 with `test ah,5; jp` against 0.0 and 1.0 and argument 5 the same
way, then multiplies each by 255.0, adds 0.5 and truncates with `0x40a2b0` (`0x78ec7d` to
`0x78ec89`). `0x7bbf20` multiplies only bytes 0 to 2 by 1/255 as a float (`0x8026c8`), and
`0x7bbc80` writes `+0x328` (`0x78ecfa`) before `0x78bae0` (`0x78ed01`). `0x78e920` reads three
numbers (`0x78e9b2`) into `+0x328` (`0x78e9de`) and calls `0x78bae0` (`0x78e9f1`).

`cmath::color_unpack_rgb` runs `0x7bbf20` whole.

## Reading the colour

`OnColorSelect` gets the colour as red, green and blue, and `GetColorRGB` returns the same three:
the colour converted back to channels, each made a byte by adding 512 to `v·255`, rounding the sum
to a float and keeping its integer part, then divided by 255. The float holds the sum to the
nearest 1/16384, so a channel can read back one step below what `SetColorRGB` stored, and one
within 1/32768 below a whole step reads back as that step. `GetColorHSV` returns the three fields
as they are. A grey colour has no hue: converting one with saturation 0 stores the hue −1, so after
`SetColorRGB` of a grey `GetColorHSV` returns −1 as the hue.

Addresses: `0x78bae0`, `0x78bb05`, `0x78bb36`, `0x78bb42`, `0x78bb62`, `0x78bb8f`, `0x7bbec0`, `0x7bbec8`, `0x7bbed4`, `0x7bbeda`, `0x7bbee0`, `0x78ed10`, `0x78edda`, `0x78ede6`, `0x78edfa`, `0x78ea00`, `0x7bbd60`, `0x8029cc`, `0x804578`, `0x7bbc80`, `0x7bbcbe`, `0x7bbccd`
Confidence: verified
Proof: one
Test: `cmath::color_pack_rgb`

`0x78bae0` redraws through `0x78bba0`, `0x78bc20` and `0x78bcf0`; with `+0x338` set (`0x78bb05`)
it converts with `0x7bbd60` (`0x78bb36`), packs with `0x7bbec0` (`0x78bb42`), multiplies each byte
by 1/255 as a double (`0x78bb62`) and runs the slot with `"%f%f%f"` (`0x78bb8f`). `0x7bbec0`
writes 0xff to byte 3 (`0x7bbec8`), adds 512.0 (`0x8029cc`) to each `v·255` (`0x7bbed4`), stores
the sum with `fstp dword` (`0x7bbeda`), which rounds it to a 24-bit significand, an ulp of 2^−14
between 512 and 1024, to nearest under the C runtime's default control word 0x027F (the path has no `fldcw`), and
shifts the float's bits right by 14 (`0x7bbee0`). `0x7bbc80` stores the value, then the
saturation, compares it with 0.0 (`0x7bbcbe`) and for 0 writes −1.0 (0xbf800000) to the hue and
returns (`0x7bbccd`). `0x78ed10` repeats the conversion (`0x78edda` to
`0x78edfa`); `0x78ea00` pushes `+0x328` to `+0x330`.

`cmath::color_pack_rgb` runs `0x7bbec0` whole.

## The thumbs

The wheel thumb's centre sits at the wheel's centre plus half the wheel's width times the
saturation, in the direction opposite to the hue's angle: `−r·(cos h, sin h)`, with `h` in
radians. The value thumb's centre sits above the value strip's bottom centre by the wheel's height
times the value, the wheel's and not the strip's. The value strip is redrawn from the colour at
full value.

Addresses: `0x78bc20`, `0x78bc4e`, `0x78bc77`, `0x78bc89`, `0x78bc9a`, `0x78bcc2`, `0x78bcd1`, `0x78bcdb`, `0x78bcf0`, `0x78bd16`, `0x78bd19`, `0x78bd49`, `0x78bba0`, `0x78bbe7`, `0x78bbf6`, `0x78bc12`, `0x7ffaac`
Confidence: verified
Proof: one
Test: none

`0x78bc20` multiplies the hue by π/180 (`0x7ffaac`, `0x78bc4e`), takes `fsincos` (`0x78bc77`),
multiplies both by the wheel's width (slot 0x1c, `0x78bc89`) times 0.5 times `+0x32c` (`0x78bc9a`),
negates both (`0x78bcc2`, `0x78bcd1`) and anchors point 4 to the wheel's point 4 (`0x78bcdb`).
`0x78bcf0` multiplies the wheel's height (slot 0x20, `0x78bd16`) by `+0x330` (`0x78bd19`) for point
4 to the strip's point 7 (`0x78bd49`). `0x78bba0` stores `HsvToRgb(h, s, 1)` in `+0x334`
(`0x78bbe7`, `0x78bbf6`) and refreshes the strip's texture (`0x78bc12`).

## A missing wheel faults

Placing the value thumb tests that the strip and the value thumb exist, but not the wheel whose
height it reads. A colour select with both and no wheel, never given one or after
`SetColorWheelTexture(nil)`, faults on its next redraw (`SetColorRGB`, `SetColorHSV`, a press in
the strip, a new value thumb) by reading address 0x24. A client fault.

Addresses: `0x78bcf0`, `0x78bcff`, `0x78bd09`, `0x78bd0b`, `0x78bd11`, `0x78bc20`, `0x78bc29`, `0x78bc37`, `0x78de90`, `0x78e02c`, `0x78b580`, `0x78b5a9`, `0x78b74f`, `0x78bae0`, `0x78baf8`, `0x78ba90`, `0x78bace`
Confidence: verified
Proof: one
Test: none

`0x78bcf0` returns without `+0x320` or `+0x324` (`0x78bcff`, `0x78bd09`), then loads `+0x318` and
adds 0x24 (`0x78bd0b`, `0x78bd11`) with no test; `0x78bc20` tests `+0x318` and `+0x31c` first
(`0x78bc29`, `0x78bc37`). `SetColorWheelTexture` (`0x78de90`) passes nil to `0x78b580`
(`0x78e02c`), which deletes the old wheel (`0x78b5a9`) and stores 0 (`0x78b74f`). The redraw
`0x78bae0` reaches `0x78bcf0` at `0x78baf8`, and the value-thumb installer `0x78ba90` at
`0x78bace`.

## Dragging

A press inside the wheel's rectangle starts a wheel drag, inside the strip's a value drag, and the
press is applied at once. During a wheel drag each move sets the hue to the angle of the cursor
from the wheel's centre, `(atan2 + π)` in degrees, and the saturation to its distance in half
widths and half heights, at most 1; the wheel's corners count. During a value drag the value is
the cursor's height in the strip, clamped to 0 to 1. Each move redraws and runs `OnColorSelect`.

Addresses: `0x78bf10`, `0x78bf3a`, `0x78bf43`, `0x78bf5b`, `0x78bf64`, `0x78bf70`, `0x78bd80`, `0x78bde7`, `0x78be1c`, `0x78be2b`, `0x78be39`, `0x78be40`, `0x78be42`, `0x78be48`, `0x78be57`, `0x78bebf`, `0x78bec9`, `0x78beed`, `0x78bef5`, `0x78bf90`, `0x78bd60`, `0x803a78`
Confidence: verified
Proof: one
Test: none

`0x78bf10` (slot 26) tests the cursor with `0x768c70` against the wheel (`0x78bf3a`) and the strip
(`0x78bf5b`), sets `+0x314` or `+0x315`, and calls slot 0x3c (`0x78bf70`). `0x78bd80` (slot 15)
divides the offsets from the centre by half the width and height (`0x78be1c`, `0x78be2b`), takes
the root of their squares (`0x78be39`) and `fpatan` (`0x78be40`), adds π (0x81d0d0) and multiplies
by 57.29578 (`0x803a78`), and caps the distance at 1 (`0x78be57`). The strip gives `(y − bottom) /
(top − bottom)` (`0x78bebf`), clamped (`0x78bec9`). `0x78bae0` follows (`0x78bef5`). Slots 27 and
13 clear both flags.

## Lua methods

`ColorSelect` has 12 methods at `0x87abb0`, registered by `0x78f0b0`, and then asks the `Frame`
methods.

Addresses: `0x87abb0`, `0x78f0b0`, `0x78f0e0`, `0x78dd80`, `0x78de90`, `0x78e070`, `0x78e160`, `0x78e360`, `0x78e450`, `0x78e630`, `0x78e720`, `0x78e920`, `0x78ea00`, `0x78eae0`, `0x78ed10`
Confidence: verified
Proof: one
Test: none

The table holds pairs from `GetColorWheelTexture` (`0x78dd80`) to `GetColorRGB` (`0x78ed10`);
`0x78f0b0` calls `0x701d80` with 0xc. `0x78f0e0`, slot 2, looks in the `ColorSelect` hash and then
calls `0x778590`.
