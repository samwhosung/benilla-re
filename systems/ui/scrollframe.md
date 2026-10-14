# ui: ScrollFrame

How a `ScrollFrame` places its scroll child at its offsets and measures how far the child can
scroll. Clamping the offsets to the range is left to scripts.

A `CSimpleScrollFrame` is a frame with the vtable 0x81cd40. Its fields are the range-dirty flag
`+0x314`, the scroll child `+0x318`, the horizontal and vertical ranges `+0x31c` and `+0x320`, the
horizontal and vertical offsets `+0x324` and `+0x328`, and the script slots `OnHorizontalScroll`
`+0x32c`, `OnVerticalScroll` `+0x334` and `OnScrollRangeChanged` `+0x33c`. How the child is read
from XML is in `xmltypes`.

Roots: `0x786a40`, `0x791bc0`

## Offsets

Setting an offset stores it as given, with no clamp, when it differs from the current one by
2^−22 or more; it then anchors the child's `TOPLEFT` to the frame's `TOPLEFT` at the horizontal
offset across and the vertical offset up, and runs `OnHorizontalScroll` or `OnVerticalScroll` with
the offset in script units. A nearly equal offset does nothing, script included.

Addresses: `0x786db0`, `0x786dc2`, `0x786dcd`, `0x786dd2`, `0x786dd8`, `0x786e1a`, `0x786d30`, `0x786d52`, `0x786d9a`, `0x787100`, `0x787110`, `0x787136`, `0x7912f0`, `0x791400`
Confidence: verified
Proof: one
Test: none

`0x786db0` compares `|v − +0x328|` with 2^−22 (`0x786dc2`) and returns below it (`0x786dcd`); it
stores `v` (`0x786dd2`), calls `0x787100` (`0x786dd8`) and runs `+0x334` with `"%f"` of the value
converted through `0x41ae40` (`0x786e1a`). `0x786d30` does the same with `+0x324` and `+0x32c`.
`0x787100` clears the child's points (`0x787110`) and calls `0x767c70(0, frame, 0, +0x324, +0x328,
1)` (`0x787136`). The Lua setters `0x7912f0` and `0x791400` pass a number to them.

## The scroll range

The range is measured on the frame's update after something marks it: a new scroll child, a change
of the frame's size, or `UpdateScrollChildRect`. A change inside the child does not mark it. The
child's rectangle is joined with those of its shown regions and its shown child frames, from a
box whose right and top start at 0; each range is that box's width or height less the frame's, at
least 0, divided by the frame's scale. When either range changed, `OnScrollRangeChanged` runs with
both.

Addresses: `0x787140`, `0x787158`, `0x787184`, `0x7871a0`, `0x7871a5`, `0x786e30`, `0x786e5d`, `0x786e64`, `0x786e79`, `0x786e84`, `0x786e8f`, `0x786e9f`, `0x786eaa`, `0x786ed1`, `0x786ede`, `0x786ef2`, `0x786f56`, `0x786f80`, `0x786d16`, `0x787205`, `0x7871f0`, `0x7918b0`
Confidence: verified
Proof: one
Test: none

`0x787140` (slot 14) acts when `+0x314` is set (`0x787158`), passes the frame's width and height
(`0x7871a0`) and clears the flag (`0x7871a5`). `0x786e30` seeds the left and bottom at 3.4·10^38
and the right and top at 0 (`0x786e5d`, `0x786e64`), joins through `0x786f80` (`0x786e79`),
subtracts the width (`0x786e84`), raises a negative to 0 (`0x786e8f`) and multiplies by `1/+0x7c`
(`0x786e9f`, `0x786eaa`), likewise the height (`0x786ed1`). Unless both moved less than 2^−22
(`0x786ede`, `0x786ef2`) it runs `+0x33c` with two numbers (`0x786f56`). The flag's writers are
`0x786d16`, `0x787205` in `0x7871f0` (slot 18) and `0x7918b0`.

## The scroll child

`SetScrollChild` replaces the child, places the new one at the current offsets and marks the range.
A nil child leaves the frame with no range to measure.

Addresses: `0x786ce0`, `0x786cf4`, `0x786cfe`, `0x786d0a`, `0x786d11`, `0x786d16`, `0x790fa0`, `0x791200`
Confidence: verified
Proof: one
Test: none

`0x786ce0` calls `0x76a400(0, 0)` on the old child (`0x786cf4`), stores the new (`0x786cfe`), calls
`0x76a400(1, 1)` on it (`0x786d0a`), then `0x787100` (`0x786d11`) and sets `+0x314` (`0x786d16`).
`0x786e30` returns at once without a child. `0x790fa0` and `0x791200` are the Lua setter and getter.

## Lua methods

`ScrollFrame` has 9 methods at `0x87b3c0`, registered by `0x791bc0`, and then asks the `Frame`
methods.

Addresses: `0x87b3c0`, `0x791bc0`, `0x791bf0`, `0x790fa0`, `0x791200`, `0x7912f0`, `0x791400`, `0x791530`, `0x791610`, `0x7916f0`, `0x7917d0`, `0x7918b0`
Confidence: verified
Proof: one
Test: none

The table holds pairs from `SetScrollChild` (`0x790fa0`) to `UpdateScrollChildRect` (`0x7918b0`);
`0x791bc0` calls `0x701d80` with 9. `0x791bf0`, slot 2, looks in the `ScrollFrame` hash and then
calls `0x778590`. `UpdateScrollChildRect` stores 1 in the frame's `+0x314`.
