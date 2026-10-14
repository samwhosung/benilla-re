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
the offset in script units. A nearly equal offset does nothing, script included. The Lua setters
and getters convert between script units and the layout units the fields hold, `v·K1 / (1024·A)`
on the way in and its inverse on the way out (`glue/script`, Cursor and screen size).

Addresses: `0x786db0`, `0x786dc2`, `0x786dcd`, `0x786dd2`, `0x786dd8`, `0x786e1a`, `0x786d30`, `0x786d52`, `0x786d9a`, `0x787100`, `0x787110`, `0x787136`, `0x7912f0`, `0x791400`, `0x7914a9`, `0x7914bb`, `0x791530`, `0x791610`, `0x41ad70`, `0x41ae40`, `0x41ae60`
Confidence: verified
Proof: one
Test: `ui::glue_786d30_display`, `ui::luabind_791_setters`

`0x786db0` compares `|v − +0x328|` with 2^−22 (`0x786dc2`) and returns below it (`0x786dcd`); it
stores `v` (`0x786dd2`), calls `0x787100` (`0x786dd8`) and runs `+0x334` with `"%f"` of the value
converted through `0x41ae40` (`0x786e1a`). `0x786d30` does the same with `+0x324` and `+0x32c`.
`0x787100` clears the child's points (`0x787110`) and calls `0x767c70(0, frame, 0, +0x324, +0x328,
1)` (`0x787136`). The Lua setters `0x7912f0` and `0x791400` read a number, divide it by `A`
(`0x41ad70`) times 1024.0 and multiply by `K1` (`0x41ae60`) before passing it (`0x7914a9` to
`0x7914bb`); the getters `0x791530` and `0x791610` multiply the field by `A` times 1024.0 and
divide by `K1` (`0x41ae40`), as the scripts' argument does.

Bit-exact coverage: `ui::luabind_791_setters` covers `0x791399` to `0x7913b9` of `0x7912f0`; `ui::luabind_791_setters` covers `0x7914a9` to `0x7914c9` of `0x791400`.

## The scroll range

The range is measured on the frame's update after something marks it: a new scroll child, a change
of the frame's size, or `UpdateScrollChildRect`. A change inside the child does not mark it. The
child's rectangle, shown or not, is joined with those of its shown regions and then, in turn, with
each shown child frame's, which joins its own the same way: every descendant reached through shown
frames counts, and nothing under a hidden one. Only resolved rectangles count, and the box's right
and top start at 0. Each range is that box's width or height less the frame's, at least 0, divided
by the frame's scale. When either range changed, `OnScrollRangeChanged` runs with both.

Addresses: `0x787140`, `0x787158`, `0x787184`, `0x7871a0`, `0x7871a5`, `0x786e30`, `0x786e5d`, `0x786e64`, `0x786e79`, `0x786e84`, `0x786e8f`, `0x786e9f`, `0x786eaa`, `0x786ed1`, `0x786ede`, `0x786ef2`, `0x786f56`, `0x786f80`, `0x786fb0`, `0x78703b`, `0x7870e0`, `0x7870ec`, `0x786d16`, `0x787205`, `0x7871f0`, `0x7918b0`
Confidence: verified
Proof: one
Test: `ui::glue_786e30_product`, `ui::glue_786e30_scale`, `ui::glue_786e30_settled`, `ui::glue_787140_rect_delta`

`0x787140` (slot 14) acts when `+0x314` is set (`0x787158`), passes the frame's width and height
(`0x7871a0`) and clears the flag (`0x7871a5`). `0x786e30` seeds the left and bottom at 3.4·10^38
and the right and top at 0 (`0x786e5d`, `0x786e64`), joins through `0x786f80` (`0x786e79`),
which reads its own rectangle with `0x768320` (`0x786fb0`) and no shown test, each region with `+0xc4` set
(`0x78703b`) and calls itself for each child frame with `+0xd0` set (`0x7870e0`, `0x7870ec`),
subtracts the width (`0x786e84`), raises a negative to 0 (`0x786e8f`) and multiplies by `1/+0x7c`
(`0x786e9f`, `0x786eaa`) at full precision, likewise the height (`0x786ed1`), but by the float-rounded copy of that reciprocal (`0x786ea5` stores it, `0x786ece` multiplies by it). Unless both moved less than 2^−22
(`0x786ede`, `0x786ef2`) it runs `+0x33c` with two numbers (`0x786f56`). The flag's writers are
`0x786d16`, `0x787205` in `0x7871f0` (slot 18) and `0x7918b0`.

## The scroll child

`SetScrollChild` replaces the child, places the new one at the current offsets and marks the range.
A nil child leaves the frame with no range to measure: the ranges keep their last values, and the
range getters go on reporting them.

The range and the offsets are in different scales. The range is divided by the frame's scale; an
offset is stored as given and becomes the child's anchor offset, which the layout multiplies by the
child's own scale (`anchors`, Relative point coordinates). Only when the two scales are equal does
`SetVerticalScroll(GetVerticalScrollRange())` bring the child's bottom to the frame's bottom.

Addresses: `0x786ce0`, `0x786cf4`, `0x786cfe`, `0x786d0a`, `0x786d11`, `0x786d16`, `0x790fa0`, `0x791200`, `0x786e42`, `0x7916f0`, `0x7917d0`, `0x786e9c`, `0x787136`, `0x7671bf`, `0x7a300a`
Confidence: verified
Proof: one
Test: none

`0x786ce0` calls `0x76a400(0, 0)` on the old child (`0x786cf4`), stores the new (`0x786cfe`), calls
`0x76a400(1, 1)` on it (`0x786d0a`), then `0x787100` (`0x786d11`) and sets `+0x314` (`0x786d16`).
`0x786e30` returns at once without a child (`0x786e42`), leaving `+0x31c` and `+0x320`, which
`0x7916f0` and `0x7917d0` read and convert. `0x790fa0` and `0x791200` are the Lua setter and
getter. `0x786e9c` loads the frame's own scale `+0x7c`; `0x787100` hands `+0x324` and `+0x328` to
`0x767c70` unscaled (`0x787136`), and the point's coordinate multiplies its offset by the same
field of the child, its layout part's `+0x58` (`0x7671bf`, `0x7a300a`).

## Teardown

The scroll frame's destructor body `0x786b80` stores its vtables back (`0x81cd40`, `0x81cd10` at `+0x24`), releases the script slots `+0x32c`, `+0x334` and `+0x33c` (`0x702670`) and ends in the frame destructor (`0x7693b0`).

Addresses: `0x786b80`, `0x786b8a`, `0x786b97`, `0x786ba5`, `0x786bbb`, `0x702670`, `0x7693b0`
Confidence: verified
Proof: one
Test: none

The three slots are released in the order `+0x32c`, `+0x334`, `+0x33c` (`0x786b83`, `0x786b9c`, `0x786baa`).

## Lua methods

`ScrollFrame` has 9 methods at `0x87b3c0`, registered by `0x791bc0`, and then asks the `Frame`
methods.

Addresses: `0x87b3c0`, `0x791bc0`, `0x791bf0`, `0x790fa0`, `0x791200`, `0x7912f0`, `0x791400`, `0x791530`, `0x791610`, `0x7916f0`, `0x7917d0`, `0x7918b0`
Confidence: verified
Proof: one
Test: `ui::luabind_791_getters`

The table holds pairs from `SetScrollChild` (`0x790fa0`) to `UpdateScrollChildRect` (`0x7918b0`);
`0x791bc0` calls `0x701d80` with 9. `0x791bf0`, slot 2, looks in the `ScrollFrame` hash and then
calls `0x778590`. `UpdateScrollChildRect` stores 1 in the frame's `+0x314`.

Bit-exact coverage: `ui::luabind_791_getters` covers `0x7915dc` to `0x7915fb` of `0x791530`; `ui::luabind_791_getters` covers `0x7916bc` to `0x7916db` of `0x791610`; `ui::luabind_791_getters` covers `0x79179c` to `0x7917bb` of `0x7916f0`; `ui::luabind_791_getters` covers `0x79187c` to `0x79189b` of `0x7917d0`.
