# ui: Button and CheckButton

What a `Button` does with its three states, its textures and its label, how a mouse press becomes
`OnClick` or `OnDoubleClick`, and what a `CheckButton` adds.

A `CSimpleButton` is a frame (`frames`) of 0x4dc bytes whose vtable `0x81c7f8` adds four slots to
the frame's: 0x90 enable, 0x94 click, 0x98 double click and 0x9c set state. Its own fields are
the state `+0x328` (0 disabled, 1 normal, 2 pushed), the state lock `+0x32c`, the click mask
`+0x330`, the last click time `+0x334`, the label `+0x338`, three fonts (normal `+0x33c`,
highlight `+0x3b8`, disabled `+0x434`), the pushed text offset `+0x4b0`, the state textures
`+0x4b8` to `+0x4c0` indexed by state, the shown one `+0x4c4`, the highlight texture `+0x4c8` and
the click scripts `+0x4cc` and `+0x4d4`. A `CSimpleCheckbox` extends it with the checked flag
`+0x4dc` and two textures. What each type reads from XML is in `xmltypes`.

Roots: `0x7786a0`, `0x785010`, `0x782c60`, `0x79a5a0`

## Construction and the type

The constructor builds the frame, zeroes the button's fields, sets the click mask to 0x100
(`LeftButtonUp`) and the pushed text offset to 0.001 and −0.001 in layout units, constructs the
three fonts, and enables the button, so a button starts NORMAL with no texture. It then calls
`0x76af00(2, -1)` on the frame. The type answers `IsObjectType` for its own id `0xcf4d88`, the
frame's `0xcf0c10` and the region's `0xcf0c3c`.

Addresses: `0x7786a0`, `0x7786c4`, `0x7786d0`, `0x778740`, `0x77874a`, `0x778762`, `0x778771`, `0x778780`, `0x81c7f8`, `0xcf4d88`
Confidence: verified
Proof: one
Test: none

`0x7786a0` calls `0x769090`, stores 0 at `+0x314`, `+0x31c`, `+0x328`, `+0x32c`, `+0x334`,
`+0x338`, 0x100 at `+0x330` (`0x7786d0`), calls `0x783a40` on `+0x33c`, `+0x3b8` and `+0x434`,
stores the vtables `0x81c7f8` and 0x81c7c8, 0x3a83126f at `+0x4b0` (`0x778740`) and 0xba83126f
at `+0x4b4` (`0x77874a`), zeroes the three state textures, and calls `0x779160(1)` (`0x778762`),
which with the state 0 sets state 1. `0x778780` (slot 4) answers 1 for the three ids.

## Setting the state

Setting the state (slot 0x9c, `0x779790(state, lock)`) stores the lock first, then does nothing
more when the state is unchanged. Otherwise, when the new state has a texture, it hides the shown
texture and shows the new state's; when the new state has none, the old texture stays shown. It
refreshes the label for the new state, then stores the state. So a button with no
`DisabledTexture` keeps drawing its normal or pushed texture while disabled, and one with no
`PushedTexture` looks unchanged while pressed.

Addresses: `0x779790`, `0x77979d`, `0x7797a9`, `0x7797b5`, `0x7797be`, `0x7797d9`, `0x7797e2`, `0x7797fc`, `0x779801`, `0x77fc60`, `0x77fcb0`
Confidence: verified
Proof: one
Test: none

`0x77979d` stores the second argument at `+0x32c`; `0x7797a9` returns when the state equals
`+0x328`. The hide at `0x7797c0` runs only when `+0x4c4` is set and `[this+state·4+0x4b8]` is set
(`0x7797b5`, `0x7797be`); it clears the texture's `+0xc4`, calls `0x77fc60` and zeroes `+0x4c4`.
The show at `0x7797e4` runs when the new slot is set (`0x7797e2`): `+0x4c4` takes it, its `+0xc4`
becomes 1, and `0x77fcb0` adds it to its layer. `0x7797fc` calls `0x779810(state)` and
`0x779801` stores `+0x328`.

## State and highlight textures

A state texture lives in the ARTWORK layer and shows only while its state is current. Installing
one deletes the texture it replaces; the new one is shown at once when its state is current. A
path reloads the slot's texture when there is one; otherwise it creates a
texture anchored to all of the button and drops it when the file does not load. The highlight
texture is in the HIGHLIGHT layer; Lua's `SetHighlightTexture` takes a blend mode, default `ADD`.
The Lua setters take a `Texture`, a string, or nil, which removes the slot's texture.

Addresses: `0x778fd0`, `0x778fe7`, `0x779002`, `0x779017`, `0x779027`, `0x778f10`, `0x778f3a`, `0x778f67`, `0x778f9d`, `0x778fb7`, `0x779050`, `0x779110`, `0x779139`, `0x781970`, `0x781960`, `0x781cc0`, `0x781ce0`, `0x781d00`, `0x781d92`, `0x778cb0`
Confidence: verified
Proof: one
Test: none

`0x778fd0(state, tex)` returns on the same texture (`0x778fe7`), clears `+0x4c4` if it held the
old one, deletes the old through slot 0 with 1 (`0x779002`), calls `0x77fd10(this, 2, 0)` on the
new (`0x779017`), stores it, and shows it when the state is `+0x328` (`0x779027`). `0x778f10`
calls `0x770200(path)` on a set slot (`0x778f3a`); else it creates one with `0x76fc40(0, 2, 1)`
(`0x778f67`), sets all points to the button (`0x778f9d`) and installs it, or deletes it
(`0x778fb7`). `0x779110` attaches with `0x77fd10(this, 4, 1)` (`0x779139`) and sets the blend.
`0x781970` takes a table of type `0xcf4cdc`, a string (`0x778f10`) or nil; `0x781960`,
`0x781cc0` and `0x781ce0` pass states 1, 2 and 0. `0x781d00` stages 3 (`0x781d92`). `0x778cb0`
clears every slot that holds a removed region.

## Enabling and disabling

Enabling (slot 0x90 with 1) moves a disabled button to NORMAL and, when the mouse is over it,
runs its enter notify. Disabling first runs the leave notify when the mouse is over it, turns
the HIGHLIGHT layer off and sets state 0; it does nothing to a button already disabled. Hiding an
enabled, unlocked button sets NORMAL. `IsEnabled` returns the number 1 or 0, never nil, so `0` is
true in Lua.

Addresses: `0x779160`, `0x77917d`, `0x779189`, `0x779192`, `0x7791a9`, `0x7791b4`, `0x7791bb`, `0x7791c8`, `0x7791e0`, `0x77fef0`, `0x77ffd0`, `0x7800b0`, `0x780166`
Confidence: verified
Proof: one
Test: none

`0x779160(enable)` with 1 and state 0 calls slot 0x9c with (1, 0) (`0x77917d`) and slot 0x4c when
the button is the top's hovered frame, `[[this+0xa0]+0x7c]` (`0x779189`, `0x779192`). With 0 and
a nonzero state it calls slot 0x50 with 1 when hovered (`0x7791a9`, `0x7791b4`),
`0x76a730(4)` (`0x7791bb`) and slot 0x9c with (0, 0) (`0x7791c8`). `0x7791e0` (slot 13) calls
slot 0x9c with (1, 0) when `+0x328` is nonzero and `+0x32c` is zero, then `0x76b290`. The Lua
`Enable` and `Disable` call slot 0x90 with 1 and 0; `IsEnabled` pushes `fild` of
`+0x328 != 0` (`0x780166`).

## The label and its fonts

The label is a `FontString` in the ARTWORK layer. Its font follows the state: the disabled font
in state 0, else the highlight font while the HIGHLIGHT layer is on, else the normal font. In the
pushed state the label moves by the pushed text offset. Installing a label deletes the old one
and, only for a label with no anchor, anchors it to the button by the normal font's justify:
`LEFT` to `LEFT`, `RIGHT` to `RIGHT`, else `CENTER`. `SetText` creates a label when there is none.
`GetText` returns nil for no label or an empty text.

Addresses: `0x779810`, `0x779824`, `0x779837`, `0x77983c`, `0x779853`, `0x77985c`, `0x779878`, `0x778d20`, `0x778d3c`, `0x778d49`, `0x778d56`, `0x778d65`, `0x778d96`, `0x778dc0`, `0x778de6`, `0x778df6`, `0x778e10`, `0x780d20`, `0x780e10`, `0x780ebb`
Confidence: verified
Proof: one
Test: none

`0x779810(state)` calls `0x772100` on the label with `+0x4b0`, `+0x4b4` for state 2 (`0x779837`)
and 0, 0 otherwise (`0x779853`); state 0 picks `+0x434` (`0x77985c`), else `+0x1a8`, the
HIGHLIGHT layer switch, picks `+0x3b8` or `+0x33c` (`0x77983c`), for `0x770c60` on the label's
`+0xcc` (`0x779878`). `0x778d20` deletes the old label (`0x778d3c`), calls `0x77fd10(this, 2, 1)`
(`0x778d49`), tests the nine point slots (`0x778d56`), and reads `+0x390`: bit 1 `LEFT` (3), bit
4 `RIGHT` (5), else `CENTER` (4) for `0x767c70` (`0x778d96`). `0x778dc0` creates one through
`0x770d30(this, 2, 1)` (`0x778df6`) and calls `0x771d80(text, 1)` (`0x778e10`). `0x780d20`
raises a usage error for a non-string; `0x780e10` reads `+0xf0` (`0x780ebb`).

## Label elements from XML

`ButtonText` builds an ordinary `FontString` region, which anchors itself by its own justify when
it has no anchor, so the button's anchoring never applies to it. `NormalText` builds the label
with its font items switched off, anchors it by its own justify (by default `CENTER`), installs
it, and then reads the same element into the normal font. `HighlightText` and `DisabledText`
read into the highlight and disabled fonts and build nothing. The `text` attribute is read after
all children.

Addresses: `0x7788c0`, `0x7789d0`, `0x7789d9`, `0x778b6c`, `0x778b7b`, `0x778b96`, `0x778ba2`, `0x778baf`, `0x778bf4`, `0x778c37`, `0x6f2780`, `0x771480`, `0x7714a3`, `0x770dd3`
Confidence: verified
Proof: one
Test: none

`0x7789d0` calls `0x6f2780`, the region builder whose last step is `0x771480`, then `0x778d20`
(`0x7789d9`). For `NormalText`, `0x778b6c` constructs the label, `0x778b7b` stores 0 in its
`+0x12c`, slot 8 of its layout part loads it, then `0x771480` (`0x778b96`), `0x778d20`
(`0x778ba2`) and `0x783c30` on `+0x33c` (`0x778baf`). `HighlightText` and `HighlightFont` both
reach `0x783c30` on `+0x3b8`, `DisabledText` and `DisabledFont` on `+0x434` (`0x778bf4`).
`0x771480` acts when the label has a parent and no point, reading its own `+0x120` (`0x7714a3`)
with the bit test of `0x778d20`; the constructor's `+0x120` is 0x212 (`0x770dd3`), `CENTER`.

## Mouse presses and clicks

A press reaches the button only after the frame's own handler declines it, while the button is
enabled, registered for the button's down or up, and under the mouse. A press registered as a
down click runs the click at once. Any such press then sets PUSHED unless the state is locked. A
release first goes to the frame: one that ends a drag clicks nothing, any other runs `OnMouseUp`
first. Only a PUSHED button clicks: registered as an up click and under the mouse, it runs the
click, or the double click (next entry), then sets NORMAL unless locked. A drag start sets
NORMAL, unless locked, before the frame's drag.

Addresses: `0x779210`, `0x77921f`, `0x77922e`, `0x779240`, `0x77925c`, `0x77926b`, `0x779285`, `0x77929d`, `0x7792b1`, `0x7792d0`, `0x7792df`, `0x7792e6`, `0x76c040`, `0x76c08f`, `0x76c0f2`, `0x76c0fb`, `0x7792fc`, `0x77934e`, `0x7793de`, `0x7793f0`, `0x779426`
Confidence: verified
Proof: one
Test: none

`0x779210` (slot 26) returns the result of `0x76bf70` when nonzero (`0x77921f`) and 0 for state 0
(`0x77922e`). It tests `(b << 8) | b` of the event's button `b` (`+0x14`) against `+0x330`
(`0x779240`) and the hit test `0x76b020` (`0x77925c`), then `b` alone (`0x77926b`) for slot 0x94
with `(b, 0)` (`0x779285`), and slot 0x9c with (2, 0) when `+0x32c` is clear (`0x7792b1`). The
release `0x7792d0` (slot 27) calls `0x76c040` (`0x7792df`) and returns when it answers nonzero
(`0x7792e6`), which it does after ending a drag (`0x76c08f`) and otherwise runs `+0x158`, the
frame's `OnMouseUp`, and answers 0 (`0x76c0f2`, `0x76c0fb`); then it requires state 2 (`0x7792fc`)
and `b << 8` in the mask (`0x77934e`), and calls slot 0x9c with (1, 0) (`0x7793de`). `0x7793f0`
(slot 29) ends with `0x76c1e0` (`0x779426`).

## Double click

A release that clicks fires `OnDoubleClick` instead of `OnClick` when the button has an
`OnDoubleClick` script, a previous click time is stored, and at most 300 ms have passed since it,
by the unsigned difference of the millisecond clock. A double click clears the stored time, so a
third click is a single click; a single click stores the current time. Only the release path can
double click. The time is the button's, not the mouse button's: a left click then a right click
within 300 ms, both registered as up clicks, runs `OnDoubleClick` with `RightButton`. Hiding the
button or the mouse leaving it keeps the stored time.

Addresses: `0x77935c`, `0x779363`, `0x77936d`, `0x779375`, `0x77937b`, `0x77938d`, `0x779393`, `0x7793a9`, `0x7793af`, `0x779650`, `0x7786da`
Confidence: verified
Proof: one
Test: none

`0x77935c` calls `0x42c010` for the time. `0x779363` tests the script slot `+0x4d4`, `0x779375`
`jbe` on `+0x334`, and `0x77937b` `cmp ecx,0x12c; ja` on the difference. `0x77938d` calls slot 0x98
with `(b, 0)` and `0x779393` zeroes `+0x334`; otherwise `0x7793a9` calls slot 0x94 and `0x7793af`
stores the time. The constructor (`0x7786da`), `0x779393` and `0x7793af` are the only writes of
`+0x334`, and the compare uses no button. Slot 0x98 is in no other call; `0x779650` mirrors
`0x779540` (next entry) with `+0x4d4`.

## Running a click

A click on an enabled button first calls the button's C++ listener, when it has one, then runs
`OnClick` with the button's name as `arg1`: `LeftButton`, `MiddleButton`, `RightButton`,
`Button4`, `Button5`, else `UNKNOWN`. A disabled button runs nothing. While the mouse runs a click
the client sets a flag that marks it as coming from the hardware; `Click()` does not.

Addresses: `0x779540`, `0x779549`, `0x779558`, `0x779588`, `0x77959b`, `0x7795aa`, `0x7795e1`, `0x779275`, `0x77928b`, `0x779352`, `0x7793b8`, `0x779890`, `0xcf4da0`, `0x494a50`
Confidence: verified
Proof: one
Test: none

`0x779540(b, scripted)` returns for state 0 (`0x779549`), calls slot 4 of the listener `+0x314`
with a record of `+0x318`, the button and `b` (`0x779588`), and with `+0x4cc` set calls `0x7026f0`
with `"%s"` and the name (`0x7795e1`), picked by the byte table 0x77960c from `b − 1` below 16
(`0x77959b`, `0x7795aa`) for bits 1, 2, 4, 8 and 16. `0xcf4da0` is 1 from `0x779275` to
`0x77928b` and from `0x779352` to `0x7793b8`; its one reader `0x779890` is called by
`0x494a50`.

## RegisterForClicks and Click

`RegisterForClicks` replaces the mask with the union of its arguments: `LeftButtonDown` 0x1,
`MiddleButtonDown` 0x2, `RightButtonDown` 0x4, `Button4Down` 0x8, `Button5Down` 0x10, and the `Up`
names the same bits shifted left by 8, ignoring case. An unknown name adds nothing, and no
argument registers nothing. `Click(name)` runs slot 0x94 for `LeftButton`, `MiddleButton` or
`RightButton`, ignoring case; no argument or a value neither a string nor a number means
`LeftButton`, and any other string 0, which the click names `UNKNOWN`. A number counts as a
string, so `Click(1)` compares `"1"` and runs as `UNKNOWN`. It does not test the state itself.

Addresses: `0x782490`, `0x78255e`, `0x782672`, `0x782685`, `0x7826a3`, `0x779730`, `0x7826c0`, `0x782766`, `0x78276b`, `0x78277f`, `0x782797`, `0x7827b6`, `0x782802`, `0x78281d`, `0x782826`
Confidence: verified
Proof: one
Test: none

`0x782490` loops over arguments 2 onward while `0x6f3510` holds, comparing with `0x64a4c0` from
`0x78255e` in the order above; the last, `Button5Up`, is `neg; sbb; and 0xfffff000; add 0x1000`
(`0x782672`). It ORs into a zero local (`0x782685`) and stores through `0x779730` (`0x7826a3`).
`Click` starts with 1 (`0x782766`) and tests argument 2 with `lua_isstring` (`0x6f3510`,
`0x78276b`), true for a number, which `lua_tostring` (`0x78277f`) converts; a string is compared
with the three names (`0x782797`, `0x7827b6`, `0x782802`); a null or empty one gives 0 (`0x78281d`).
Every path calls slot 0x94 with `(b, 1)` (`0x782826`).

## Button state from Lua

`SetButtonState(state, lock)` takes `DISABLED`, `NORMAL` or `PUSHED`, ignoring case, and sets that
state directly through slot 0x9c, with the lock read as a flag (default 0); any other name raises
`Usage: %s:SetButtonState("state", lock)`. It bypasses enabling, so `DISABLED` this way neither
runs the leave notify nor turns the highlight off. `GetButtonState` returns the state's name.
`LockHighlight` and `UnlockHighlight` lock the HIGHLIGHT layer on or release it; the button then
refreshes its label font. A release leaves the layer on while the mouse is over the button.

Addresses: `0x780270`, `0x780321`, `0x780335`, `0x780341`, `0x780390`, `0x780180`, `0x780240`, `0x782840`, `0x782920`, `0x495a80`, `0x76c2e0`
Confidence: verified
Proof: one
Test: none

`0x780390` compares with `DISABLED` 0, `NORMAL` 1, `PUSHED` 2 through `0x64a4c0`. `0x780335` reads
argument 3 with `0x6f1c10` and default 0 (`glue/script`, Movie resolution and subtitles).
`0x780240` maps 0 to 2 back to the names, else `UNKNOWN`. `0x782840` and `0x782920` call slot
0x80 with 1 and 0, which is `0x495a80`: `0x76c2e0` then `0x779810(+0x328)`. `0x76c2e0` stores
`+0xf8` and calls `0x76a710(4)`, or `0x76a730(4)` unless the button is `[[+0xa0]+0x7c]`.

## Entering and leaving

The mouse entering or leaving a disabled button runs neither the frame's enter or leave
(the `OnEnter` and `OnLeave` scripts and the highlight) nor the label refresh, so a disabled
button shows no tooltip from these scripts. An enabled one runs the frame's, then refreshes the
label font. Either way the button then notifies its C++ listener at `+0x31c`, when set.

Addresses: `0x779490`, `0x77949b`, `0x77949d`, `0x7794ab`, `0x7794d9`, `0x7794e0`, `0x7794ee`, `0x7794f4`, `0x779502`, `0x779530`
Confidence: verified
Proof: one
Test: none

`0x779490` (slot 19) skips `0x76b6a0` and `0x779810` for state 0 (`0x77949b`); `0x7794e0`
(slot 20) skips `0x76b6f0` and `0x779810` (`0x7794ee`). Each then stores `+0x320` or `+0x324` and
the button in 0xcf4d98 and 0xcf4d9c and calls slot 4 of `+0x31c` with 0xcf4d90
(`0x7794d9`, `0x779530`).

## CheckButton

`SetChecked(checked, force)` stores the flag unless unchanged and not forced, hides both checked
textures and, when checked, shows the disabled one if the button is disabled and has it, else the
checked one. Enabling or disabling refreshes this. A click toggles the check before running the
button's click, so `Click()` toggles even a disabled check button, which then runs no script. A
double click does not toggle: the second release of a quick pair runs `OnDoubleClick` alone, so
the pair toggles once.
Lua's `SetChecked` reads its argument as a flag with default 1, so no argument checks;
`GetChecked` returns 1 or nil.

Addresses: `0x785010`, `0x78501a`, `0x7854c0`, `0x7854d0`, `0x785516`, `0x785524`, `0x785538`, `0x785490`, `0x7854aa`, `0x785550`, `0x785566`, `0x785575`, `0x799bf0`, `0x799c77`, `0x799ca0`, `0x799d25`, `0x785230`, `0x779650`
Confidence: verified
Proof: one
Test: none

`0x785010` builds a button (`0x78501a`), zeroes `+0x4dc` to `+0x4e4` and stores 0x81cb80. `0x7854c0`
returns at `0x7854d0` when the second argument is 0 and the flag equal; it hides `+0x4e0` and
`+0x4e4` with `0x77fc60` and shows `+0x4e4` when it is set and `+0x328` is 0 (`0x785524`), else
`+0x4e0` (`0x785538`). `0x785490` (slot 0x90) calls `0x779160` then `0x7854c0(flag, 1)`. `0x785550`
(slot 0x94) calls `0x7854c0(!flag, 0)` then `0x779540`. Slot 0x98 of 0x81cb80 is the button's
`0x779650`. `0x799bf0` pushes 1 as the default (`0x799c77`); `0x799ca0` pushes 1.0 or nil
(`0x799d25`).

## Checked textures

The checked textures live in the OVERLAY layer. Installing one deletes the old and refreshes the
check. A path reloads the checked texture when there is one; otherwise it creates one anchored
to all of the button with blend mode `ADD`. `SetDisabledCheckedTexture` with a path tests the
checked slot instead of its own: with a checked texture present it loads the path into the
checked texture and leaves the disabled one unchanged.

Addresses: `0x785330`, `0x785359`, `0x78536f`, `0x785440`, `0x785270`, `0x785295`, `0x7852f8`, `0x785380`, `0x785388`, `0x7853a5`, `0x799e50`, `0x79a140`, `0x6f1920`, `0x6f1953`
Confidence: verified
Proof: one
Test: none

`0x785330` and `0x785440` delete the old texture, call `0x77fd10(this, 3, 0)` (`0x785359`), store
`+0x4e0` or `+0x4e4` and call `0x7854c0(flag, 1)` (`0x78536f`). `0x785270` tests `+0x4e0` and
reloads it (`0x785295`), else creates, sets all points and `0x7703f0(3)` (`0x7852f8`). `0x785380`
tests `+0x4e0` at `0x785388` and reloads it at `0x7853a5`; its creating path ends in `0x785440`.
`0x6f1920` reads the blend names as (value, name) pairs from 0x811aa8 (`0x6f1953`); 3 pairs with
`ADD` at 0x811ac0. `0x799e50` and `0x79a140` call these for a string and the installers for a
texture or nil.

## Lua methods

`Button` has 39 methods at `0x879d00`, registered by `0x782c60`; `CheckButton` adds 6 at
`0x87bf74`, registered by `0x79a5a0`, and finds the rest in `Button`'s. Each takes self as the
`Frame` methods do (`methods`, Finding self) with the type's id.

Addresses: `0x879d00`, `0x782c60`, `0x782c90`, `0x87bf74`, `0x79a5a0`, `0x79a5d0`, `0x77fef0`, `0x77ffd0`, `0x7800b0`, `0x780180`, `0x780270`, `0x780400`, `0x7804c0`, `0x780580`, `0x780640`, `0x780700`, `0x7807c0`, `0x780880`, `0x7809a0`, `0x780a60`, `0x780c30`, `0x780d20`, `0x780e10`, `0x780ee0`, `0x781100`, `0x781260`, `0x781480`, `0x7815e0`, `0x781800`, `0x781960`, `0x781bb0`, `0x781cc0`, `0x781cd0`, `0x781ce0`, `0x781cf0`, `0x781d00`, `0x781f40`, `0x782030`, `0x782180`, `0x782290`, `0x782390`, `0x782490`, `0x7826c0`, `0x782840`, `0x782920`, `0x799bf0`, `0x799ca0`, `0x799d60`, `0x799e50`, `0x79a050`, `0x79a140`
Confidence: verified
Proof: one
Test: none

The tables are read as pairs of a name and a function, 39 from `Enable` (`0x77fef0`) to
`UnlockHighlight` (`0x782920`) and 6 from `SetChecked` (`0x799bf0`) to `SetDisabledCheckedTexture`
(`0x79a140`). `0x782c60` and `0x79a5a0` call `0x701d80` with counts 0x27 and 6 and the hashes
0xcf4e14 and 0xcf53b4. `0x782c90` (slot 2) looks a name up in the `Button` hash, then calls
`0x778590`; `0x79a5d0` looks in the `CheckButton` hash, then calls `0x782c90`.
