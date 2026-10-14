# ui: clickable links in message frames

How a chat window answers a click on a link although it does not take the mouse itself: the frame draws a
small button over every link, and the buttons hand their events to the frame's three hyperlink scripts.
The base class is shared by `ScrollingMessageFrame` and `SimpleHTML`.

The code is `CSimpleHyperlinkedFrame.cpp`, `0x7a3220` to `0x7a3a50`: the frame class `CSimpleHyperlinkedFrame`
(constructor `0x7a3540`) and the link button `CSimpleHyperlinkButton` (constructor `0x7a3240`). The mouse
rules are `mouse`; the chat window is `messageframe`.

Roots: `0x7a3540`, `0x7a3240`, `0x7a3880`, `0x7a3350`, `0x7a3920`, `0x7a3700`

## The link button

A link is covered by a `CSimpleHyperlinkButton`, 0x4ec bytes, whose constructor runs the button
constructor and so is born with the mouse enabled. It stores the link at `+0x4dc` and the shown text at
`+0x4e0` (both empty at birth) and sets its click mask `+0x330` to 0x500: it fires on release of the left
or right button, never on press and never for the middle button. A release outside the button fires
nothing. The click plays no button sound.

Addresses: `0x7a3240`, `0x7a324a`, `0x7a3251`, `0x7a325d`, `0x7a3264`, `0x7a326a`, `0x7a327d`, `0x7786a0`, `0x778771`, `0x779730`, `0x7786d0`, `0x779210`, `0x77924b`, `0x77926b`, `0x7792d0`, `0x779335`, `0x779345`, `0x779540`, `0x779588`, `0x7a3510`
Confidence: verified
Proof: one
Test: none

`0x7a3240` calls `0x7786a0` first, whose `0x778771` is the button's own mouse enable, then stores the
vtable `0x81d5b0` and `push 0x500; call 0x779730` (a one-line setter of `[this+0x330]`). The button
constructor default is `0x100` (`0x7786d0`), left release only. The mask halves are the press button in the
low byte (tested at `0x77926b`) and the release in the high (shifted by 8 at `0x77934b`); 1 is left, 2 middle, 4
right, so 0x500 is left release and right release. `0x7a3510` replaces the click slot and never calls the
button's sound handler (`0x779588` is in the base `0x779540`). The second click inside 300 ms goes to the double-click
slot only when `+0x4d4` is set, which a link button never does, so it clicks twice.

## The three hyperlink scripts

The frame class holds three script slots: `+0x320` `OnHyperlinkEnter`, `+0x328` `OnHyperlinkLeave` and
`+0x330` `OnHyperlinkClick`. Enter and leave pass the link as their only argument. Click passes the link,
the shown text and the name of the mouse button (`LeftButton`, `MiddleButton`, `RightButton`, `Button4`,
`Button5`, else `UNKNOWN`). A button forwards its three events (highlight on, highlight off, click) to
its parent through three frame virtuals at `+0x90`, `+0x94` and `+0x98`.

Addresses: `0x7a3700`, `0x7a3780`, `0x7a37b0`, `0x7a37e0`, `0x7a34b0`, `0x7a34e0`, `0x7a3510`, `0x7a37fc`, `0x7a3803`, `0x7a3850`, `0x7a3868`, `0x7a3793`, `0x7a379b`, `0x7a3759`, `0x7026f0`, `0x76a0d0`
Confidence: verified
Proof: one
Test: none

Each forwarder loads the parent from `[this+0x9c]`, pushes the text `[this+0x4e0]` and the link `[this+0x4dc]` and
calls `[parent_vtable+0x90]`, `+0x94` or `+0x98` (`0x7a34c9`, `0x7a3530`). The firers run a script through
`0x7026f0` when the slot is set: enter and leave are given two strings but the descriptor `%s`, so only the
link reaches Lua; click uses `%s%s%s` and maps the button argument through the byte table `0x7a3868` and the
jump table `0x7a3850` to the button names (`0x7a37fc`, `0x7a3803`). The script names are added by the override
`0x7a3700`, which calls the base resolver `0x76a0d0` first.

## The pool

A frame keeps its link buttons in a pool. `0x7a3880` takes a button from the free list at `+0x318` or
allocates 0x4ec bytes and constructs one; `0x7a3350(frame, span)` copies the span's link and text into the
button, anchors it over the span's rectangle with the point and size calls, and shows it, which puts it in
the mouse hit list; with no frame or span it hides it. `0x7a3920` hides a button and returns it to the free
list. Nothing is freed, so the pool grows to the most links shown together.

Addresses: `0x7a3880`, `0x7a3883`, `0x7a3891`, `0x7a389f`, `0x7a38ab`, `0x7a38b6`, `0x7a3350`, `0x7a3371`, `0x7a3410`, `0x7a3415`, `0x7a3441`, `0x7a3457`, `0x7a346b`, `0x7a3472`, `0x7a347c`, `0x7a348b`, `0x7a3920`, `0x7a392f`, `0x7a3939`, `0x6462e0`
Confidence: verified
Proof: one
Test: none

The allocation is `0x6462e0(0x4ec, "CSimpleHyperlinkedFrame.cpp", 0x7e, 0)`, not zero filled, so the constructor
sets everything. Configure reallocates `+0x4dc` and `+0x4e0` to the span's lengths plus 1 and copies them,
then `ClearAllPoints` (`0x767ed0`), `SetPoint` (`0x767c70`), width and height, each the span extent divided by the
button's scale `[+0x7c]`, and ends with `[this+0xd0] = 1` and the show slot `+0x88`.

## What fills the pool

The pool is rebuilt when a message frame is dirty. For a `ScrollingMessageFrame` the rebuild `0x788c00` runs in
the per-frame update `0x788460` while `[frame+0x3b8]` is set; it releases every live button, then for each
shown line asks the line's string for its hyperlink spans (`0x5c27e0`, 32 bytes a span) and configures one
button per span. A `SimpleHTML` frame rebuilds with `0x78af70` under `[frame+0x370]`. A span is four
floats of the rectangle, then the link pointer and length, then the shown text pointer and length; the font
layer's escape reader writes them when it reads `|H...|h`.

Addresses: `0x788c00`, `0x788460`, `0x788595`, `0x78859a`, `0x788538`, `0x788a8e`, `0x788bf1`, `0x788c78`, `0x788ce7`, `0x788d5f`, `0x788d67`, `0x78af70`, `0x78a30b`, `0x5c27e0`, `0x5cd420`, `0x5ccdf2`, `0x5ccdf5`
Confidence: verified
Proof: one
Test: none

`[frame+0x3b8]` is zeroed by the constructor (`0x7878d0`), set by the fade tick (`0x788538`) and the relayout
(`0x788a8e`, `0x788bf1`) and cleared at `0x78859a`. The span array is `[gxstring+0x84]` with its count at
`+0x80`, returned only while the string is laid out and two measurement globals are non-zero (`0x5cd431` to
`0x5cd447`). There is no mouse-enable test on the path, which is why a chat window whose frame has the
mouse disabled is still clickable over its links and click-transparent elsewhere.

## The two classes

`CSimpleHyperlinkedFrame` is a plain frame (mouse bits 0), the shared base of `ScrollingMessageFrame` and
`SimpleHTML` and of nothing else; its constructor writes only the pool fields (`+0x314` the 0x4e4 element
size, `+0x318` and `+0x31c` the free list, the three script slots). The link button derives from the button
class, not from this one. The link button's name is `CSimpleHyperlinkButton`, which a type descriptor carries (the string `.?AVCSimpleHyperlinkButton@@`, used as the tag of an inlined delete).

Addresses: `0x7a3540`, `0x7a354a`, `0x7a359c`, `0x7a35a2`, `0x7a35ad`, `0x787670`, `0x78767f`, `0x789dd0`, `0x7a3631`, `0x81d680`, `0x81d5b0`
Confidence: verified
Proof: one
Test: none

`0x7a3540` calls `CSimpleFrame`'s constructor `0x769090` (`0x7a354a`) and stores the vtable `0x81d680`;
`0x787670` and `0x789dd0` are its only two callers. The link button vtable `0x81d5b0` differs from the button's
`0x81c7f8` in four code slots: the destructor, `+0x4c`, `+0x50` and `+0x94`. The tag push `0x7a3631` is in
the pool teardown.
