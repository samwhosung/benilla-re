# ui: Frame method contracts

What the `Frame` methods of `CSimpleFrameScript.cpp` that only read or set one field do: the type test, the script test, the
identity and scale getters, the visibility getters, the movable, resizable, user-placed and clamped flags, moving, the
drag buttons, keyboard input, and the title region. Every method begins with the receiver check of `ui/methods`
(`Finding self`), and the layout, strata, draw-layer and backdrop methods are in the frame core docs.

Roots: `0x773700`, `0x773820`, `0x773910`, `0x774680`, `0x774dc0`, `0x774fc0`, `0x775080`, `0x775280`, `0x775340`,
`0x776420`, `0x776700`, `0x776a50`, `0x776c00`, `0x776d60`, `0x776ec0`

## Types, scripts and identity

`IsFrameType("type")` asks the frame's virtual slot `+0x18` whether it is of the named type and returns 1 or nil; without
a string it raises `Usage: %s:IsObjectType("TYPE")` with the frame's name or `<unnamed>`. `HasScript("type")` asks slot `+0xc`
whether the frame has that script and returns 1 or nil; without a string it raises `Usage: %s:HasScript("type")`.
`GetID()` returns the integer at `+0xb0` and `SetID(n)` stores the truncated n there, raising `Usage: %s:SetID(ID)` for a
non-number. `GetScale()` returns the float at `+0xb8`, `GetEffectiveScale()` the layout scale at `+0x7c`, and `GetAlpha()` the
byte at `+0xc8` times 1/255.

Addresses: `0x773700`, `0x77376e`, `0x77379c`, `0x7737d4`, `0x7737df`, `0x774680`, `0x774754`, `0x774dc0`, `0x774e63`, `0x774e75`, `0x774fc0`, `0x77505f`, `0x775080`, `0x77511f`, `0x775280`, `0x77531f`, `0x775340`, `0x7753d6`, `0x7753e7`, `0x804578`
Confidence: verified
Proof: one
Test: `ui::luabind_774dc0_get_alpha`

`0x7737d4` and `0x774754` call the slots with the type text as the one stack argument. `0x77379c` and `0x7737a4` build the
name for the usage error through slot `+0x4`. `0x774e63` loads the alpha byte and `0x774e75` multiplies by the double 1/255
(`0x804578`); the value is not rounded. `0x7753e7` stores the `__ftol` result with no clamp.

`ui::luabind_774dc0_get_alpha` runs the channel normalisation of `GetAlpha`, the slice `0x774e72` to `0x774e7e` (`fild`, `fmul` by 1/255, `fstp`).

## Visibility getters and the toplevel flag

`IsShown()` returns 1 when the shown dword at `+0xd0` is nonzero, `IsVisible()` when the visible dword at `+0xd4` is, and
`IsToplevel()` when bit 0 of the flags at `+0xb4` is set; each returns nil otherwise. `SetToplevel([flag])` sets or clears
that bit through `CSimpleFrame::SetFrameFlag` and returns nothing; the flag argument is read at index 2 with the default
true.

Addresses: `0x7758d0`, `0x775955`, `0x775990`, `0x775a15`, `0x7754f0`, `0x775575`, `0x775440`, `0x7754ce`, `0x7754d8`, `0x76a3c0`
Confidence: verified
Proof: one
Test: none

`0x775955` tests `[frame+0xd4]` and `0x775a15` `[frame+0xd0]`. `0x775575` is `test byte [edi+0xb4],1`. `0x7754d8` calls
`0x76a3c0` with the mask 1 and the flag. The raise and strata effects of the bit are in `ui/strata`.

## Movable, resizable, user placed and clamped

`SetMovable([flag])` and `SetResizable([flag])` set or clear bits 0x100 and 0x200 of the frame flags; `IsMovable()` and
`IsResizable()` return 1 or nil from them. `SetUserPlaced([flag])` sets or clears bit 0x1000 and raises
`Frame %s is not movable or resizable` unless one of the two is set; `IsUserPlaced()` tests bit 0x1000. `SetClampedToScreen([flag])`
sets or clears bit 0x10 of the layout flags at `+0x60` and resizes; `IsClampedToScreen()` tests it. The flag argument is read at
index 2 and is true when absent.

Addresses: `0x776420`, `0x7764b4`, `0x7764d0`, `0x776555`, `0x776590`, `0x776624`, `0x776640`, `0x7766c5`, `0x776a50`, `0x776adb`, `0x776b2f`, `0x776b40`, `0x776bc5`, `0x776c00`, `0x776c94`, `0x768cc0`, `0x768cd2`, `0x776cb0`, `0x776d35`, `0x76a3c0`, `0x7680e0`
Confidence: verified
Proof: one
Test: none

The setters push the mask (`0x100` at `0x7764b4`, `0x200` at `0x776624`, `0x1000` at `0x776b2f`) and the flag for `0x76a3c0`. The
getters test `ah` against 1 (`0x776555`), 2 (`0x7766c5`) and 0x10 (`0x776bc5`) of the dword `+0xb4`. `0x776adb` tests `ah,3` before
the user-placed store. `0x768cc0` sets bit 0x10 at `+0x3c` of the layout part (frame `+0x60`) and calls `Resize(0)`.

## Moving and sizing

`StartMoving()` raises `Frame %s is not movable` unless bit 0x100 is set; otherwise, when the top frame is not already
moving or sizing a frame, it starts a move of this frame in mode 3 from the current cursor, with point 4. `StopMovingOrSizing()`
ends the operation when this frame is the one in progress. The machinery is in `ui/input`.

Addresses: `0x776700`, `0x77678b`, `0x7767aa`, `0x7767dc`, `0x7767e2`, `0x77680e`, `0x77681d`, `0x776822`, `0x776990`, `0x776a35`, `0x776a40`, `0x7652b0`, `0x765640`, `0x41ad80`
Confidence: verified
Proof: one
Test: none

`0x77678b` tests bit 0x100 of `+0xb4`, `0x7767dc` loads the top frame from `+0xa0` and `0x7767e2` its in-progress frame `+0xcfc`.
`0x776822` calls `StartMovingOrSizing(frame, 3, x, y, 4)` with the screen delta of the cursor from `0x41ad80`. `0x776a35`
compares `+0xcfc` with the frame and `0x776a40` calls `0x765640`.

## Drag buttons and keyboard input

`RegisterForDrag(...)` takes any number of button names from argument 2 and stores their mask at `+0xfc`: `LeftButton` is
1, `MiddleButton` 2 and `RightButton` 4, compared without regard to case; any other string adds nothing, and no string
clears the mask. `EnableKeyboard([flag])` enables or disables both the key bit and the character bit of the input mask;
`IsKeyboardEnabled()` returns 1 when either bit is set.

Addresses: `0x776d60`, `0x776e29`, `0x776e44`, `0x776e5f`, `0x776e6e`, `0x776e92`, `0x776ec0`, `0x776f53`, `0x776f5d`, `0x776f68`, `0x776f75`, `0x76af00`, `0x76af50`, `0x776f90`, `0x777035`, `0x77703e`
Confidence: verified
Proof: one
Test: none

`0x776e24` to `0x776e74` compare each name; `0x776e6e` makes 4 for an equal `RightButton` and 0 for any other unknown name. The mask is
ORed over the arguments (`0x776e7b`) and stored at `0x776e95`. `EnableKeyboard` calls `EnableEvent` for bits 1 and 0
(`0x776f5d`, `0x776f68`) or `DisableEvent` (`0x776f75`). `0x777035` tests bit 1 and `0x77703e` bit 0 of `+0xcc`.

## The title region

`CreateTitleRegion()` returns the frame's title region, making one when there is none: a plain region of 0xa0 bytes with the
title region vtables, parented to the frame and stored at `+0xa8`. When one exists it calls `ClearAllPoints` on it first and returns it, so
the anchors of a title region made in XML are lost. `GetTitleRegion()` returns the region or nil. The region draws
nothing; a press inside it starts a drag of the frame (`ui/input`).

Addresses: `0x773910`, `0x773995`, `0x7739a2`, `0x7739ad`, `0x7739b7`, `0x7739c4`, `0x7739c9`, `0x7739cf`, `0x7739df`, `0x7739f2`, `0x773a12`, `0x773820`, `0x7738bf`, `0x7738f4`, `0x767ed0`, `0x768060`, `0x76c4f0`
Confidence: verified
Proof: one
Test: none

`0x773995` loads `[frame+0xa8]`; a nonzero value takes `0x7739a2`, `ClearAllPoints`. A new region is `SMemAlloc(0xa0)` from
`CSimpleFrameScript.cpp` line 49 (`0x7739ad`, `0x7739b7`), built by `0x76c4f0` and given the vtables at `0x7739c9` and `0x7739cf`. `0x7739df`
calls slot `+0x14` (set parent) with the frame, and `0x7739f2` passes bit 2 of the frame's layout flags to
`SetDeferResize`. The Lua object is fetched with `lua_rawgeti` after binding its reference with `0x701bd0`. `GetTitleRegion` pushes nil at
`0x7738f4`.

## Strata, level and texture alpha

`Frame:GetFrameStrata()` pushes the name of the stratum index at `+0xc0` of the frame, `UNKNOWN` for an index outside the eight. `Frame:GetFrameLevel()` pushes the level at `+0xc4` as a number. `Texture:GetAlpha()` pushes the alpha byte of the texture's vertex colour times 1/255, as a double. All three check the receiver as the other methods do and push one value.

Addresses: `0x7742a0`, `0x7744a0`, `0x79b690`, `0x6f1820`, `0x77f8c0`, `0x804578`
Confidence: verified
Proof: one
Test: none

`0x77433f` loads `[frame+0xc0]` into `ecx` for `GetFrameStrataName` and `0x77434e` pushes the returned name with `lua_pushstring`. `0x77453f` is `fild [frame+0xc4]` and `0x77454d` `lua_pushnumber`. `0x79b740` calls `CScriptRegion::GetVertexColor`, `0x79b745` loads the byte at `+3`, `0x79b754` multiplies by the double at `0x804578`, 0.00392156862745098.

