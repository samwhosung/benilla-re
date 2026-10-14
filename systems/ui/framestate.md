# ui: frame state methods

The Lua methods of a frame that read or set one piece of its state: its type, title region, id, toplevel
flag, draw layers, the move, size and placement flags, the drag buttons and the keyboard switch. The stock
methods that take a frame through a larger mechanism are described where the mechanism is: `strata` for the
level and strata, `visibility` for show and hide, `mouse` for the mouse switches, `backdrop` for the backdrop methods and
`frames` for the child and region lists.

Every one of these methods starts with the check of `methods`, "Finding self": a table whose element 0 is a frame, else an error. The
flag arguments of the setters are read as a script flag with default true (`actionuse`, "Reading a script flag"), so a call with no argument
sets the flag and nil clears it. The flag bits are those of `+0xb4` (`frames`).

Roots: `0x773640`, `0x773700`, `0x773820`, `0x773910`, `0x775280`, `0x775340`, `0x775440`, `0x7755b0`, `0x776420`, `0x776700`, `0x776d60`, `0x776ec0`

## Frame type

`GetFrameType()` answers the frame's type name. `IsFrameType(name)` answers the number 1 when the frame's class
or a base answers to the name, ignoring case, and nil otherwise; it takes a string or a number, and any other
argument raises `Usage: %s:IsObjectType("TYPE")`, naming `IsObjectType` for a method called `IsFrameType`. Both
call the class slots that `widgettype` describes.

Addresses: `0x773640`, `0x7736e1`, `0x7736e8`, `0x773700`, `0x77378c`, `0x7737af`, `0x7737d4`, `0x7737dd`, `0x7737f2`
Confidence: verified
Proof: one
Test: none

`0x773640` calls slot 7 (`0x7736e1`) for the name and pushes it with `lua_pushstring` (`0x7736e8`). `0x773700` tests argument 2 with
`lua_isstring` (`0x77378c`) and raises the usage string with `<unnamed>` or the frame's name (`0x7737af`); otherwise it calls slot 6 with
`lua_tostring` of the argument (`0x7737d4`) and pushes the number 1.0 (`0x7737dd`) or nil (`0x7737f2`).

## The title region

`GetTitleRegion()` answers the title region's table or nil. `CreateTitleRegion()` answers the existing one, after
clearing its anchors, or makes a new one with the frame as its parent, and always answers it. The region is a bare
layout object: it has no draw layer and is not in the frame's region list (`frames`).

Addresses: `0x773820`, `0x7738bf`, `0x7738c8`, `0x7738d5`, `0x7738f4`, `0x773910`, `0x773995`, `0x7739a2`, `0x7739b7`, `0x7739c4`, `0x7739df`, `0x7739ec`, `0x7739f2`, `0x767ed0`, `0x768060`
Confidence: verified
Proof: one
Test: none

`0x773820` reads `+0xa8` (`0x7738bf`); a null answers nil (`0x7738f4`), else the region's Lua table, made on first use by
`0x701bd0` (`0x7738d5`). `0x773910` reads `+0xa8` too (`0x773995`): with a region it clears its anchors (`0x7739a2`); without one it
allocates 0xa0 bytes from `CSimpleFrameScript.cpp` (`0x7739b7`), builds a `CScriptRegion` (`0x7739c4`), calls its parent slot with the frame
(`0x7739df`), stores it at `+0xa8` (`0x7739ec`), and asks for a deferred layout (`0x7739f2`).

## The id

`SetID(n)` stores the number, truncated toward zero, as the frame's id at `+0xb0`; a non-number raises `Usage: %s:SetID(ID)`. `GetID()` answers
the number stored.

Addresses: `0x775280`, `0x77532d`, `0x775340`, `0x7753cc`, `0x7753dd`, `0x7753e2`, `0x7753e7`, `0x77540c`, `0x775405`, `0x40a2b0`
Confidence: verified
Proof: one
Test: none

`0x775340` tests argument 2 with `lua_isnumber` (`0x7753cc`), converts it with `0x40a2b0` (`0x7753e2`) and stores it at `+0xb0` (`0x7753e7`). The stored value
is not range checked.

## The toplevel flag

`SetToplevel(flag)` sets or clears bit 1 of `+0xb4`, and `IsToplevel()` answers the number 1 when it is set and nil
when it is not. A frame that is not toplevel is not raised by a click on it (`strata`).

Addresses: `0x775440`, `0x7754ce`, `0x7754d8`, `0x76a3c0`, `0x7754f0`, `0x775575`, `0x775582`, `0x775595`
Confidence: verified
Proof: one
Test: none

`0x775440` reads the flag with `0x6f1c10` and default true (`0x7754ce`) and calls `SetFrameFlag(1, flag)` (`0x7754d8`), which ORs or clears the bit in
`+0xb4` and does nothing else (`0x76a3c0`). `0x7754f0` tests the bit (`0x775575`).

## Draw layers

`EnableDrawLayer(name)` and `DisableDrawLayer(name)` switch one of the five layers on or off. A layer name that is
missing or not recognised, ignoring case, means ARTWORK. A switch changes what the frame's regions draw at once:
the frame marks its own batches dirty when it draws itself, otherwise the level that holds it.

Addresses: `0x7755b0`, `0x775640`, `0x775647`, `0x775662`, `0x77566d`, `0x775680`, `0x77573d`, `0x76a710`, `0x76a717`, `0x76a722`, `0x76a730`, `0x76a742`, `0x76a990`, `0x76a999`, `0x76a9a2`, `0x76a9c6`, `0x6f18b0`, `0x811a80`
Confidence: verified
Proof: one
Test: none

The methods start the layer at 2 (`0x775640`), replace it when `lua_isstring` holds and `0x6f18b0` finds the name (`0x775647`, `0x775662`) and call
`0x76a710` (`0x77566d`) or `0x76a730` (`0x77573d`). These store 1 or 0 at `+0x198 + 4·layer` (`0x76a717`) and call `0x76a990` (`0x76a722`, `0x76a742`),
which tests bit 0x2000 of `+0xb4` (`0x76a999`) for the frame's own batches (`0x76b530`, `0x76a9a2`) and otherwise, with a top and a visible
frame, marks the level (`0x764a60`, `0x76a9c6`).

## Moving and sizing

`SetMovable(flag)` and `SetResizable(flag)` set bits 0x100 and 0x200 of `+0xb4`, and `IsMovable()` and `IsResizable()` answer 1 or nil
by them. `StartMoving()` starts a move from the cursor and `StartSizing([point])` a size from the point, default `BOTTOMRIGHT`. Each
raises `Frame %s is not movable` or `Frame %s is not resizable` when its bit is clear and does nothing while the top already moves
a frame. `StopMovingOrSizing()` ends the move or size, but only for the frame the top is moving.

Addresses: `0x776420`, `0x7764ae`, `0x7764bb`, `0x7764d0`, `0x776590`, `0x77662b`, `0x776640`, `0x776700`, `0x77678b`, `0x7767aa`, `0x7767dc`, `0x7767e2`, `0x77681d`, `0x776822`, `0x776830`, `0x7768bb`, `0x7768da`, `0x776934`, `0x77693f`, `0x776981`, `0x776990`, `0x776a35`, `0x776a40`, `0x7652b0`, `0x765640`, `0x6f1840`
Confidence: verified
Proof: one
Test: none

The setters call `SetFrameFlag` with 0x100 (`0x7764bb`) and 0x200 (`0x77662b`). `0x776700` tests the movable bit of `+0xb4` (`0x77678b`) and raises its message
(`0x7767aa`); it then reads the top's moving-frame word `+0xcfc` (`0x7767e2`) and, when it is clear, converts the cursor at `+0x1118` and `+0x111c` with
`0x41ad80` and calls `0x7652b0(frame, 3, x, y, 4)` (`0x77681d`, `0x776822`). `0x776830` does the same with the resizable bit, a point read from
argument 2 by `0x6f1840` over a default of 8 (`0x776934`), and `0x7652b0(frame, 3, x, y, point)` (`0x776981`). `0x776990` calls `0x765640` only when the top's word
equals the frame (`0x776a35`, `0x776a40`).

## Placement flags

`SetUserPlaced(flag)` sets or clears bit 0x1000 of `+0xb4` and raises `Frame %s is not movable or resizable` when neither bit
0x100 nor 0x200 is set; `IsUserPlaced()` answers 1 or nil by bit 0x1000. `SetClampedToScreen(flag)` and `IsClampedToScreen()` set and read
bit 0x10 of the layout flags at `+0x60`.

Addresses: `0x776a50`, `0x776ad5`, `0x776adb`, `0x776aea`, `0x776b36`, `0x776b40`, `0x776bc5`, `0x776c00`, `0x776c97`, `0x776cb0`, `0x776d35`, `0x768cc0`, `0x76a3c0`
Confidence: verified
Proof: one
Test: none

`0x776ad5` reads `+0xb4` and tests `ah & 3`, which is the two bits 0x100 and 0x200 (`0x776adb`); a frame with neither raises the message (`0x776aea`), and
the rest calls `SetFrameFlag(0x1000, flag)` (`0x776b36`). `0x776c97` calls `0x768cc0` with the flag, and `0x776d35` tests bit 0x10 of `+0x60`.
The bit 0x1000 is the one the layout cache saves and a move or size sets (`layoutcache`).

## The drag buttons

`RegisterForDrag(button, ...)` sets the mouse buttons that start a drag from the frame, replacing the old set. The arguments are
`LeftButton`, `MiddleButton` and `RightButton`, ignoring case; an unknown string adds nothing, and the list ends at the first argument that is not a string.
With no argument the frame registers for no button.

Addresses: `0x776d60`, `0x776e03`, `0x776e14`, `0x776e2f`, `0x776e38`, `0x776e4a`, `0x776e53`, `0x776e65`, `0x776e6a`, `0x776e7b`, `0x776e85`, `0x776e95`, `0x776ea4`
Confidence: verified
Proof: one
Test: none

The mask starts at 0 (`0x776e00`) and each string argument from 2 ORs in 1 for `LeftButton` (`0x776e38`), 2 for `MiddleButton` (`0x776e53`) or 4 for
`RightButton` (`0x776e6a` to `0x776e71`), 0 otherwise (`0x776e76`), while `lua_isstring` holds (`0x776e85`). The result replaces `+0xfc` (`0x776e95`);
a first argument that is not a string stores 0 (`0x776ea4`). The drag test of `mouse` reads this mask.

## The keyboard

`EnableKeyboard(flag)` enters the frame in the key and character lists, or removes it; `IsKeyboardEnabled()` answers 1 or nil by the character list's bit alone.

Addresses: `0x776ec0`, `0x776f4e`, `0x776f57`, `0x776f5d`, `0x776f68`, `0x776f75`, `0x776f7e`, `0x776f90`, `0x77702f`, `0x77703e`, `0x76af00`, `0x76af50`
Confidence: verified
Proof: one
Test: none

`0x776ec0` reads the flag with `0x6f1c10` and default true (`0x776f4e`). True calls `0x76af00(1, -1)` and `0x76af00(0, -1)` (`0x776f5d`, `0x776f68`) and false `0x76af50(1)` and
`0x76af50(0)` (`0x776f75`, `0x776f7e`). `0x776f90` tests bit 0 of `+0xcc` (`0x77703e`), the bit of input kind 0.
