# ui: message frame members

The members behind `MessageFrame` and `ScrollingMessageFrame` (`messageframe`): the line objects, teardown, the layout of lines to the frame's height, the scroll frame's setters and its draw
callback. A `CSimpleMessageFrame` keeps its queued messages at `+0x358` and its line holders at `+0x368` (capacity, count `+0x36c`, data `+0x370`, 12 bytes each); a holder points to a
reference-counted line, a `FontString` with two times and an active byte. A `CSimpleMessageScrollFrame` keeps its ring of line records at `+0x394`, its display nodes at `+0x3a0` (count) and `+0x3a4` (data, 16 bytes each).

Roots: `0x785830`, `0x785e80`, `0x785ea0`, `0x7863b0`, `0x7871c0`, `0x787220`, `0x7879e0`, `0x788310`, `0x788750`

## Message frame teardown

The destructor body `0x785830` stores the vtables back, releases the line holders (`0x786720`: each line loses a reference and is destroyed at zero, then the array is freed), empties the queued messages (`0x785e00`), deletes the font `+0x344` and ends in the frame
destructor. `0x785e00` frees each queued text and then the queue at `+0x358`, `+0x35c`, `+0x360`. A line is built by `0x7865f0` and destroyed by `0x7866b0`, which deletes its `FontString`; `0x7867a0` releases a holder's line and frees the holder when bit 0 of its flag is set, and `0x786770` frees a queued record on the same flag.

Addresses: `0x785830`, `0x785858`, `0x785869`, `0x78587c`, `0x7858b9`, `0x7858d8`, `0x786720`, `0x785e00`, `0x785e29`, `0x785e53`, `0x7865f0`, `0x7866b0`, `0x7867a0`, `0x786770`, `0x7693b0`, `0x81cc68`, `0x81cc38`, `0x87a5f8`, `0x87a5dc`
Confidence: verified
Proof: one
Test: none

The vtables stored are `0x81cc68`, `0x81cc38` (at `+0x24`) and `0x81cc34` (at `+0x314`). `0x786720` frees the holder array with the tag `0x87a5f8` (`CSimpleMessageFrameLine`) and `0x785e00` the queue with `0x87a5dc` (`MessageData`).

## Making a line

`0x7861c0` builds a line holder: it allocates a 0x1c-byte line (`SMemAlloc`, line 0x22 of `CSimpleMessageFrame.h`), builds it with `0x7865f0`, stores it at `+0x8` of the holder, counts a reference at `[line+4]` and zeroes the holder's two floats. `0x7865f0` makes the line's `FontString` with
`0x770d30` (layer 2, 1) and sets bit 0x4000 of its word `+0x120`, rebuilding the string when that changed.

Addresses: `0x7861c0`, `0x7861d3`, `0x7861d9`, `0x7861e5`, `0x7865f0`, `0x786612`, `0x786628`, `0x78664e`, `0x786666`, `0x770d30`, `0x7724a0`, `0x81ccfc`, `0x87a64c`
Confidence: verified
Proof: one
Test: none

`0x7865f0` stores the line vtable `0x81ccfc` (`0x786612`) and takes the `FontString` from the region pool through `GetBlockPoolData` with the tag `0x846544` (`0x786618`). The allocation names `0x87a64c`.

## Activating a line

`0x7865b0(line, a, b)` activates a line: it sets `+0xc4` of its `FontString`, registers it (`0x77fcb0`), stores the two times at `+0x10` and `+0x14`, sets the active byte `+0x18` and counts it in `+0x31c`. `0x786570(line)` reverses an active line: it clears `+0xc4`, unregisters it
(`0x77fc60`), zeroes the times and the byte and decrements `+0x31c`.

Addresses: `0x7865b0`, `0x7865bd`, `0x7865c7`, `0x7865d8`, `0x7865dc`, `0x786570`, `0x786587`, `0x78658d`, `0x78659b`, `0x77fcb0`, `0x77fc60`
Confidence: verified
Proof: one
Test: none

`0x786570` tests the active byte first (`0x786578`) and does nothing for an inactive line.

## Adding a message

`0x7863b0(text, colour)` adds one message: a null text sets error 0x57. With lines, it opens slot 0 (`0x786490(0)`), stores the colour and the text (`0x771d80(text, 1)`), activates the line with the frame's two times (`0x7865b0`) and, from the line's height over the font height plus 0.5, pushes older lines down.
`0x786490(n)` moves the holders from index `n` toward the end one place and re-anchors each (`0x767c70` with the negated offset at `+0x4` of its record), then puts the last holder at `n`.

Addresses: `0x7863b0`, `0x7863bf`, `0x7863df`, `0x7863fe`, `0x78641d`, `0x78644a`, `0x786477`, `0x786490`, `0x7864fd`, `0x78654c`, `0x771d80`, `0x767c70`, `0x7865b0`, `0x803078`
Confidence: verified
Proof: one
Test: `ui::glue_7863b0_level_fraction`, `ui::glue_786490_negate`

`ui::glue_7863b0_level_fraction` runs the `fdivr` of the line height by the font height and the `fadd` of 0.5 (`0x803078`, a double) as a copied-byte island. `ui::glue_786490_negate` runs the `fchs` at `0x7864e1` as a slice; the twin at `0x786532` has the same bytes.

## Message frame layout

`0x785ea0` lays the lines out when flag 1 of `+0x60` is set. The inner rectangle `+0x320` to `+0x32c` is the insets `+0x330` to `+0x33c` times the scale added to the frame's rectangle. A row is the pixel-aligned (`0x766750`) font height times the scale plus the aligned spacing. The line count `+0x318` is the inner height
over a row, truncated, and one more when the whole rows leave less than 2^-22 over; with a row of 0 or less it is 0. Lines past the count are deactivated and the holder array is brought to the count.

Addresses: `0x785ea0`, `0x785eaa`, `0x785ec7`, `0x785ef0`, `0x785f13`, `0x785f4c`, `0x785f61`, `0x785f7a`, `0x785fbd`, `0x785ff7`, `0x78602e`, `0x786057`, `0x766750`, `0x7861c0`, `0x7867a0`, `0x7868e0`
Confidence: verified
Proof: one
Test: `ui::glue_785ea0_header`, `ui::glue_785ea0_sum_divide`, `ui::glue_785ea0_quantize`, `ui::glue_785ea0_tail`

The slices are `0x785eb5` to `0x785efd` (the inner rectangle), `0x785f03` to `0x785f51` (the row sum and the division), `0x785f5e` to `0x785f82` (the quantise compare against 2^-22) and the tail that anchors each line (slope, coordinates and the negated accumulate, the last as a copied-byte island). The array is resized by `0x7868e0(n)`, which releases the holders from `n` up, reallocates under the tag `0x87a5f8` and copies the kept holders with a reference added before releasing the old ones. New holders come from
`0x7861c0` and dropped ones go through `0x7867a0`; each new line gets its parent and layer (`0x77fd10`), font (`0x770c60`), anchors (`0x767c70`) and fixed length (`0x771680`).

## Message frame setters

The layout callback `0x785e80` runs the frame's (`0x76b580`) and then `0x785ea0`; `0x785c00(a, flags)` runs the layout when bit 0 or 4 of `flags` is set. `0x785c20` and `0x785c80` store a time at `+0x34c` or `+0x350` and copy it into every line whose own time is 0.
`0x785b80` stores four insets (`+0x330` to `+0x33c`) and lays out; `0x785dc0` deactivates every line and clears the queue. `0x785bb0` stores a length at `+0x340` and sets the fixed length of each line's `FontString` through `0x771680`; nothing calls it.

Addresses: `0x785e80`, `0x785cec`, `0x785e8a`, `0x785c00`, `0x785c0f`, `0x785c20`, `0x785c4b`, `0x785c80`, `0x785cab`, `0x785b80`, `0x785ba7`, `0x785dc0`, `0x785df8`, `0x785bb0`, `0x785be1`, `0x76b580`, `0x771680`
Confidence: verified
Proof: one
Test: none

The unreached claim is a census: every `call` and `jmp` to `0x785bb0` in the listing and every little-endian occurrence of the address in each section of the image, none of either; the same two scans find the callers of `0x785ea0` (`0x785b80`, `0x785c00`, `0x785e80`, and `0x785cec` inside `0x785ce0`), the control. `0x785c20` tests the line's time against 0.0 with `test ah,0x44` (`0x785c4b`).

## Scroll frame teardown

`0x7879e0` is the scroll frame's destructor body: it stores the vtables, frees the arrays (`0x788f40`, `0x788f90`), releases the link buttons (`0x788c00`) and the script slot, frees each line record's text (`0x7878f0`) and the arrays, and ends in the hyperlink frame destructor (`0x7a35e0`).
`0x788020`, a slot of the sub-object at `+0x338`, destroys the display nodes when a changed word has bits 0x11, frees their array and lays the frame out again with `0x788750`.

Addresses: `0x7879e0`, `0x7879f1`, `0x787a44`, `0x787a52`, `0x787b0b`, `0x7878f0`, `0x788020`, `0x788023`, `0x788080`, `0x788750`, `0x788c00`, `0x7a35e0`, `0x81ce18`, `0x81cde8`, `0x81cde4`, `0x87a7d0`
Confidence: verified
Proof: one
Test: none

The vtables stored are `0x81ce18`, `0x81cde8` (at `+0x24`) and `0x81cde4` (at `+0x338`). `0x788020` calls each node's slot 1 with 0 (`0x78804b`), frees the array with the tag `0x87a7d0` (`CSimpleMessageScrollFrameDisplayNode`) and zeroes its header (`0x788073`).

## Scroll frame setters

`0x788090(t)` and `0x7880f0(t)` store the visible and the fade time at `+0x364` and `+0x368` and copy them into every shown record whose node is set (`+0xc`, `+0x10`). `0x7882b0` empties the frame: the count and the write position to 0, a layout with `0x788750`, the scroll to 0 and the position to −1.
`0x788440` runs the frame's show and then `0x788750` with `+0x348`. `0x788690` and `0x7886d0` are the bodies of `ScrollToTop` and `ScrollToBottom`; `0x787fb0` stores four insets (`+0x37c` to `+0x388`) and runs `0x788310`.

Addresses: `0x7882f0`, `0x7882fa`, `0x788090`, `0x7880ba`, `0x7880c8`, `0x7880f0`, `0x788114`, `0x788128`, `0x7882b0`, `0x7882c9`, `0x788440`, `0x788451`, `0x788690`, `0x7886ba`, `0x7886d0`, `0x7886e5`, `0x7886eb`, `0x787fb0`, `0x787fd7`, `0x788750`, `0x788310`
Confidence: verified
Proof: one
Test: none

`0x788690` passes `0x788750` the count compared with the limit: an unfilled ring gives 0 and a full one the write position plus one, wrapping to 0 at the limit (`0x7886ae` to `0x7886b7`). `0x7886d0` jumps to `0x788b80` when `+0x354` is set and otherwise calls `0x788750` with the position.

## Scroll frame draw

The scroll frame registers a draw callback for its layer 4: `0x7871c0` runs the frame's emit `0x76b3a0` and, for layer 4, `BatchAddCallback` with `0x787220`. The callback computes the frame's projected rectangle, clamps its texture coordinates to 0 to 1 on each axis (a coordinate below 0 moves the other end down by the same amount and becomes 0;
one above 1 moves the other end up and becomes 1) and draws between the matrices it pushes.

Addresses: `0x7871c0`, `0x7871d0`, `0x7871e2`, `0x787220`, `0x787388`, `0x7873dd`, `0x787457`, `0x787482`, `0x787502`, `0x78754c`, `0x773110`, `0x76b3a0`
Confidence: inferred
Proof: none
Test: `ui::glue_787220_clamp01`

The clamp arithmetic is read; what the callback draws is not. `ui::glue_787220_clamp01` runs both axes as one slice (`0x787482` to `0x78750e`, six subtractions): axis B starts where axis A ends.

## Scroll frame layout

`0x788310` is the callback for a changed rectangle: it recomputes the four edges into `+0x36c` to `+0x378` and, when the width or height changed by 1e-5 or more, frees the line nodes and lays the lines out again with `0x788750`. `0x788750` places the shown lines: it counts the lines that fit by dividing the height by each line's measured
height (`0x7729b0`) and anchors a node per line (`0x77fd10`, `0x767c70`, `0x7680e0`), showing it (`0x77fcb0`) or hiding it (`0x77fc60`).

Addresses: `0x788310`, `0x788321`, `0x788394`, `0x7883a1`, `0x7883b2`, `0x788427`, `0x788750`, `0x7887be`, `0x788902`, `0x7889a8`, `0x7889dd`, `0x7889ee`, `0x7889f3`, `0x80c5c8`
Confidence: inferred
Proof: none
Test: `ui::glue_788_788310_rect_changed`, `ui::glue_geom_788750_bounds_ratios`, `ui::glue_geom_788750_acc_and_sum`

The arithmetic is covered; the placement loop is read, not derived. `ui::glue_788_788310_rect_changed` runs the edge recompute and the X and Y compares (`0x788321` to `0x78839c`, `0x7883a1` to `0x7883ba`); `ui::glue_geom_788750_bounds_ratios` the two `(hi − lo) / scale` blocks (`0x7887be` to `0x7887da`, `0x788902` to `0x78891b`);
`ui::glue_geom_788750_acc_and_sum` the accumulate `fsub` (`0x7889f3` to `0x788a03`) and, as a copied-byte island, the `fadd` of `0x7889ad`.

## Unreached scroll frame code

`0x787fe0` stores a length at `+0x358` and sets the fixed length of each display node's `FontString` through `0x771680`. Nothing calls it and no section stores its address.

Addresses: `0x787fe0`, `0x787ff2`, `0x788011`
Confidence: verified
Proof: one
Test: none

The census is every `call` and `jmp` to `0x787fe0` in the listing and every little-endian occurrence of the address in each section of the image: none of either. The same scans find the stored address of `0x788020` (`0x81cde4`) and the callers of `0x788690`, the controls.
