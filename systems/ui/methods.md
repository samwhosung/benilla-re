# ui: the Frame and Region methods

How a Lua method call on a frame reaches the C++ object, and what the methods of the `Frame` and
`Region` tables do with it.

`CSimpleFrameScript.cpp` (`0x773640` to `0x778660`) holds the 68 methods of `Frame` and the
functions that install them; the run `0x7a11d0` to `0x7a2f60` holds the 19 methods every region
shares (`GetName`, `SetParent`, `SetPoint` and the rest) and their installers. A frame's method
lookup asks the `Frame` table first and then the `Region` table, so a frame answers both.

Roots: `0x778300`, `0x778560`, `0x778580`, `0x778590`, `0x7a2c10`, `0x7a2e70`, `0x7a2e90`, `0x7a2ea0`

## Method tables and lookup

The `Frame` methods are 68 pairs of a name and a function at `0x878ec0`, from `GetFrameType` to
`SetBackdropBorderColor`; the `Region` methods are 19 pairs at `0x87c9b8`, from `GetObjectType` to
`ClearAllPoints`. Each table is hashed by name into its own hash table, `0xcf4d38` and `0xcf54b4`.
A frame's slot 8 looks a name up in the `Frame` hash and, when it is not there, in the `Region`
hash.

Addresses: `0x878ec0`, `0x87c9b8`, `0x778560`, `0x778565`, `0x77856a`, `0x77856f`, `0x778580`, `0x778590`, `0x7785a7`, `0x7785c0`, `0x7a2e70`, `0x7a2e90`, `0x7a2ea0`, `0xcf4d38`, `0xcf54b4`, `0x701d80`, `0x701f20`, `0x702000`
Confidence: verified
Proof: one
Test: none

The two tables are read as pairs of a string pointer and a function, 68 and 19 of them.
`0x778560` pushes `0xcf4d38` and calls `0x701d80` with `edx` 0x44 and `ecx` `0x878ec0`
(`0x778565` to `0x77856f`); `0x778580` hands `0xcf4d38` to `0x701f20`. `0x778590` (slot 8 of
`0x81c498`) calls `0x702000` with `0xcf4d38` (`0x7785a7`) and, when it answers 0, `0x7a2ea0`
(`0x7785c0`), which calls `0x702000` with the `Region` hash. `0x7a2e70` and `0x7a2e90` install and
remove the `Region` table.

## Finding self

Every method takes the object from its first argument: the argument must be a table, else it
raises `Attempt to find 'this' in non-table object (used '.' instead of ':' ?)`; element 0 must hold
an object, else `Attempt to find 'this' in non-framescript object`; the object must pass the type
test for the class's id, else `Wrong object type for member function`. The `Frame` methods test
with the id `0xcf0c10`, numbered from the shared counter on first use.

Addresses: `0x775750`, `0x77575e`, `0x775769`, `0x775777`, `0x77577c`, `0x775781`, `0x77579c`, `0x7757a6`, `0x7757bd`, `0x775894`, `0xcf0c10`, `0x847ef8`, `0x847ec0`
Confidence: verified
Proof: one
Test: none

`Show` (`0x775750`) is the pattern: when `[0xcf0c10]` is 0 it increments `[0xceef6c]` and stores
it (`0x77575e` to `0x775769`); it reads the type of argument 1 (`0x775777`) and compares it with 5
(`0x77577c`), raising `0x847ef8` otherwise; it reads element 0 of the table (`0x77579c`,
`0x7757a6`) and raises `0x847ec0` for none; it calls slot 0x10 with the id (`0x775894`) and raises
the third message when it answers 0. The other methods of both tables repeat the sequence.

## What the Frame methods call

The `Frame` methods pass their arguments to the frame functions of the other topics. `Show` and
`Hide` set the shown flag and call slots 0x88 and 0x84 (`visibility`); `Raise` raises and `Lower`
does nothing (`strata`); `SetFrameStrata` and `SetFrameLevel` call the setters, refusing an unknown
stratum and a negative level; `RegisterEvent` and the other three event methods call the script
layer (`script`); `SetAlpha` and `SetScale` clamp or refuse as described in `visibility`;
`EnableMouse`, `EnableKeyboard` and `EnableMouseWheel` switch input kinds (`input`).

Addresses: `0x775750`, `0x775810`, `0x775a50`, `0x775b20`, `0x774360`, `0x774560`, `0x774a40`, `0x774b30`, `0x774c20`, `0x774cf0`, `0x774e90`, `0x775140`, `0x777070`, `0x776ec0`, `0x7771f0`
Confidence: verified
Proof: one
Test: none

The calls are read from each method's body: `0x775a50` calls `0x76a5b0` and `0x775b20` calls
`0x76a5c0`; `0x774360` calls `0x6f17d0` and `0x76a470`; `0x774560` calls `0x76a4f0`; `0x774a40`,
`0x774b30`, `0x774c20` and `0x774cf0` call `0x702140`, `0x702280`, `0x7023e0` and `0x7024f0`;
`0x777070` calls `0x76af00` or `0x76af50` with kind 2, `0x7771f0` with 3, and `0x776ec0` both
with kinds 1 and 0.

## Coordinates the methods convert

`GetWidth` answers `G(w) = RN24(1024·A·w) / K1` (`screencoord#script-units-and-layout-units`), `w`
the answer of the layout part's slot 0x1c rounded to a float: a frame's is `+0x50` (`0x768420`);
textures and font strings override it (next entry). A `w` equal to 0 becomes `RN24(right − left)`
when the rectangle is valid, after a pending region is laid out. `GetLeft` lays out a pending
region and answers `E(left) = RN24(1024·A·(left/s)) / K1`, `s` the region's layout scale `+0x7c`,
or nil for an invalid rectangle. Each quotient is at 53 bits and pushed as a double. The span
`GetWidth` reads is not divided by `s`: a frame sized only by its anchors answers its width times
its effective scale.

Addresses: `0x7a1e00`, `0x7a1980`, `0x81c400`, `0x768420`, `0x768310`, `0x7680e0`, `0x768320`, `0x7a1e99`, `0x7a1ee6`, `0x7a1a4e`, `0x7a1a63`, `0x7a1a88`, `0x6f3810`, `0x6f37f0`
Confidence: verified
Proof: two
Test: `ui::region_size_getters_whole`, `ui::region_edge_getters_whole`

`GetWidth` calls slot 0x1c of `+0x24` and `fst [ebp-4]`; `fcomp 0.0; test ah,0x44; jp` (`0x7a1e99`)
falls through only for an equal value, so NaN keeps the size. Then `0x768310`, `0x7680e0(1)` when
pending, `0x768320` and, when valid, `fld [ebp-8]; fsub [ebp-0x10]; fstp [ebp-4]` (`0x7a1ee6`):
right `+0xc` minus left `+0x4` of `{bottom, left, top, right}`. `GetLeft` calls `0x768310`, `0x7680e0(1)`
when pending and `0x768320`, reads `s` from `[edi+0x7c]` (`0x7a1a4e`), `fld left; fdiv s; fmulp`
(`0x7a1a63`), and pushes nil on failure (`0x7a1a88`). The tests run both whole over six aspects,
five scales, NaN, zero, pending and invalid rectangles.

## The other getters

The other size and position getters take the same steps:

| method | answers |
|---|---|
| `GetHeight` | as `GetWidth`, with slot 0x20 (`+0x54`) and `top − bottom` |
| `GetWidth`, `GetHeight` of a texture | slot 0x1c or 0x20 is `0x770720` or `0x770790`: `+0x50` or `+0x54`, or when that is 0 and an image is set, the image's width or height in texels as `S` stores it, at 53 bits; a texture without an image, or with a zero-sized one, falls back to the span |
| `GetWidth`, `GetHeight` of a font string | slot 0x1c or 0x20 is `0x772930` or `0x772a60`: `+0x50` or `+0x54`, or when that is 0 the text's width or height, raised to `S(1)` when not above it; never 0, so never the span |
| `GetRight`, `GetTop`, `GetBottom` | as `GetLeft`, with the right, top or bottom edge |
| `GetCenter` | `E((right − left)·0.5 + left)`, then `E((top − bottom)·0.5 + bottom)`, the midpoints at 53 bits; two nils for an invalid rectangle |
| `GetPoint` | `G` of the point's x and y offsets |
| `GetHitRectInsets` | `G` of the four insets, in the order `SetHitRectInsets` takes them |

Addresses: `0x7a2030`, `0x7a1aa0`, `0x7a1bc0`, `0x7a1ce0`, `0x7a1810`, `0x7a2340`, `0x775bf0`, `0x768410`, `0x81c6ec`, `0x81c74c`, `0x770720`, `0x770790`, `0x772930`, `0x772a60`, `0x770739`, `0x770743`, `0x77074e`, `0x772890`, `0x772971`, `0x77297a`, `0x76b1e0`, `0x76b1b0`, `0x7a18f7`, `0x7a2483`
Confidence: verified
Proof: one
Test: `ui::region_size_getters_whole`, `ui::region_edge_getters_whole`, `ui::luabind_7a1_get_point`, `ui::luabind_775_get_hit_rect_insets`

`GetHeight` uses `[ebp-0xc]` minus `[ebp-0x14]`; right, top and bottom read `+0xc`, `+0x8`, `+0x0`.
The layout vtables `0x81c6ec` and `0x81c74c` hold the texture and font string slots at 0x1c and
0x20. `0x770720` keeps `+0x50` unless it equals 0 (`jp` at `0x770739`) and an image is set
(`0x770743`), then converts `0x44b3b0`'s width (`0x77074e`) with `fild; fstp dword` and the setter
steps, returning `st0`. `0x772930` takes `0x772890`'s text width for a 0, then `fcomp` of `S(1)`
against it (`0x772971`); `test ah,5; jp` (`0x77297a`) keeps the width only when it is above
`S(1)`, so NaN gives `S(1)`. The `0x770790` and `0x772a60` bodies are the same on the height.
`GetCenter` is `fld far; fsub near; fmul 0.5; fadd near; fdiv s` (`0x7a18f7`). `GetPoint` takes
`[edi+4]` and `[edi+8]` (`0x7a2483`). `GetHitRectInsets` reads the four through `0x76b1e0`, which
writes the fields `0x76b1b0` stores, argument for argument. The first three rows run whole as
`GetWidth` and `GetLeft` do; the other two are slices `0x7a2483` to `0x7a24c6`, `0x7a24e7` to
`0x7a252a` and `0x775caa` to `0x775d35`, at 4:3.

## Script numbers the setters store

`SetWidth(v)` passes `S(v) = RN24(K1 · RN24(RN24(v) / (1024·A)))` to the layout part's slot 0x14,
with no layout scale; the layout part multiplies by the layout scale only when it resolves. A
non-number raises `Usage: %s:SetWidth(width)`.

Addresses: `0x7a1f20`, `0x7a1fb0`, `0x7a1fcb`, `0x7a1fda`, `0x7a1fe0`, `0x7a1feb`, `0x6f34d0`, `0x6f3620`
Confidence: verified
Proof: two
Test: `ui::region_size_setters_whole`, `ui::luabind_7a1_setters`

`SetWidth` tests argument 2 with `0x6f34d0` (`0x7a1fb0`), reads it with `0x6f3620` and stores it
with `fstp dword [ebp-4]` (`0x7a1fcb`); `fdivr [ebp-4]` (`d8 7d`, `0x7a1fda`) divides it by
`1024·A`; `fstp dword [esp]` and `0x41ae60` (`0x7a1fe0`); `fstp dword` and `call [ebx+0x14]`
(`0x7a1feb`). `region_size_setters_whole` runs it whole over six aspects, ±0, ±∞, NaN, doubles
that round to a float tie and random numbers.

## The other setters

| method | stores |
|---|---|
| `SetHeight` | as `SetWidth`, through slot 0x18 and also by `K1`; a non-number raises `Usage: %s:SetHeight(height)` |
| `SetPoint` | `S(x)` and `S(y)` as the point's offsets when both are numbers, else 0 and 0 |
| `SetHitRectInsets` | `S` of each of its four numbers, through `0x76b1b0`; a missing number raises `Usage: %s:SetHitRectInsets(left, right, top, bottom)` |

Addresses: `0x7a2150`, `0x7a2540`, `0x775d50`, `0x7a221b`, `0x7a263c`, `0x7a2888`, `0x7a2898`, `0x7a28a6`, `0x7a28ff`, `0x775e38`, `0x775ee2`, `0x76b1b0`, `0x767c70`
Confidence: verified
Proof: one
Test: `ui::region_size_setters_whole`, `ui::luabind_7a1_setters`, `ui::luabind_775_set_hit_rect_insets`

`SetHeight` is `SetWidth` with slot 0x18 (`0x7a221b`). `SetPoint` zeroes both offsets
(`0x7a263c`), converts them only after both `lua_isnumber` tests pass (`0x7a2888`, `0x7a2898`) and
passes them to `0x767c70` (`0x7a28ff`). `SetHitRectInsets` converts arguments 2 to 5 from
`0x775e38` and calls `0x76b1b0` (`0x775ee2`). `SetHeight` runs whole with `SetWidth`; the other two
are the 4:3 slices `0x7a28a6` to `0x7a28ff` and `0x775e3b` to `0x775ee2`.

## Screen size and cursor globals

With `s_U` the layout scale of `UIParent` (`+0x7c`, 1.0 when it does not exist), `fx` and `fy` the
cursor dwords the screen root keeps at `+0x1118` and `+0x111c`:

| function | answers |
|---|---|
| game `GetScreenWidth`, `GetScreenHeight` | `G(RN24(K1/s_U))`, `G(RN24(K2/s_U))` |
| glue `GetScreenWidth`, `GetScreenHeight` | `G(K1)`, `G(K2)` |
| `GetCursorPosition`, game and glue | `G(RN24(K1·fx))`, `G(RN24(K2·fy))` |

At `a = RN24(4/3)` the height is 767.9999427795418 in glue and in the game with `s_U` 1;
`uiScale` enters only through `s_U`.

Addresses: `0x48b480`, `0x48b4d0`, `0x46dd30`, `0x46dd80`, `0x48b820`, `0x46dad0`, `0x494590`, `0x48b499`, `0x48b4e9`, `0x48b850`, `0x48b867`, `0x48b88b`, `0xb4b44c`, `0xcf0bd8`
Confidence: verified
Proof: two
Test: `ui::ui_screen_size_whole`, `ui::ui_cursor_position_whole`, `ui::luabind_48b_screen_getters`, `glue::lua_model_scale_difftest`

The game pair pushes 1.0 to `0x41ae60` or `0x41ae70`, stores the product as a float, divides it
by `0x494590` with `fdivr [ebp-4]` (`0x48b499`, `0x48b4e9`) and stores it again before the getter
steps. `0x494590` loads `[[0xb4b44c]+0x7c]` or 1.0. The glue pair has no division. Both cursor
functions pass `[[0xcf0bd8]+0x1118]` and `+0x111c` to `0x41ad80` (`0x48b850`), which stores
`K1·fx` and `K2·fy` as floats, then take the getter steps for each, both through `0x41ae40`
(`0x48b867`, `0x48b88b`). `ui_screen_size_whole` runs the game pair whole over six aspects and six
scales and without `UIParent`, and `ui_cursor_position_whole` runs `0x48b820` whole. The glue
functions have only `lua_model_scale_difftest`, which runs the steps from the `call 0x41ad70`,
replaced by a load of `A`, to before the `call 0x41ae40`, for `0x46dd30`, `0x46dd80` and the first axis of `0x46dad0`.

## Other methods that convert

The census of `0x41ad70`'s sites finds the getter step `G` and the setter step `S` at 158 sites,
each value through `K1`: 113 in the functions of these entries and of the table's first three
rows, 45 in the 20 functions of its last row. Which stored field or argument each converts is in its own
topic.

| shape | functions |
|---|---|
| `G` | `Frame:GetMinResize`, `GetMaxResize`, `GetBackdrop`; `Button:GetPushedTextOffset`, `GetTextWidth`, `GetTextHeight`; the four `ScrollFrame` scroll getters; `EditBox:GetTextInsets`; `FontString:GetFont`, `GetShadowOffset`, `GetSpacing`, `GetStringWidth`; the `Font` methods `GetFont`, `GetShadowOffset`, `GetSpacing` |
| `S` | `Frame:SetMinResize`, `SetMaxResize`, `SetBackdrop`; `Button:SetPushedTextOffset`; `ScrollFrame:SetHorizontalScroll`, `SetVerticalScroll`; `EditBox:SetTextInsets`; `FontString:SetFont`, `SetShadowOffset`, `SetSpacing`, `SetTextHeight`; the `Font` methods `SetShadowOffset`, `SetSpacing`; `0x79f210`; `GameTooltip:SetOwner`, `SetPadding`; `PingLocation`, `PositionWorldMapArrowFrame`, `PositionMiniWorldMapArrowFrame` |
| `S` from XML | `0x6f1d60` (`val`, `AbsValue`), `0x6f1eb0` (`x`, `y`, `AbsDimension`), `0x6f20a0`; `RelValue` and `RelDimension` are stored as parsed |
| internal | the XML loaders `0x770f40` and `0x783c30`; the layout cache `0x4905e0` and `0x490e60`; the tooltip `0x530050`, `0x5303d0`, `0x530640`; the chat bubble `0x4b0940`, `0x4b1600`; the edit box `0x77b8c0`, `0x77da80`; the scroll frame `0x786d30`, `0x786db0`, `0x786e30`; `0x78ab40`; the texture and font string size slots `0x770720`, `0x770790`, `0x772930`, `0x772a60`; the loading screen `0x406640` |

Addresses: `0x41ad70`, `0x775f20`, `0x7761a0`, `0x777370`, `0x782180`, `0x782290`, `0x782390`, `0x791530`, `0x791610`, `0x7916f0`, `0x7917d0`, `0x798c30`, `0x79d400`, `0x79df60`, `0x79e1d0`, `0x79e510`, `0x79f3b0`, `0x79fad0`, `0x79fbe0`, `0x776020`, `0x7762a0`, `0x7776e0`, `0x782030`, `0x7912f0`, `0x791400`, `0x798a60`, `0x79d4f0`, `0x79e070`, `0x79e2b0`, `0x79e3c0`, `0x79f9c0`, `0x79fb40`, `0x79f210`, `0x5310d0`, `0x530f10`, `0x4eeca0`, `0x4a8d20`, `0x4a8f20`, `0x6f1d60`, `0x6f1eb0`, `0x6f20a0`, `0x6f1d91`, `0x6f1e72`, `0x6f1ee8`, `0x6f2060`, `0x770f40`, `0x783c30`, `0x4905e0`, `0x490e60`, `0x530050`, `0x5303d0`, `0x530640`, `0x4b0940`, `0x4b1600`, `0x77b8c0`, `0x77da80`, `0x786d30`, `0x786db0`, `0x786e30`, `0x78ab40`, `0x406640`
Confidence: verified
Proof: one
Test: none

Each site was read as the instructions from the `call 0x41ad70` to the next call: `G` is `fmul
[0x7ffd68]; fmul m32; fstp dword; call 0x41ae40`, `S` is `fmul [0x7ffd68]; fdivr m32; fstp dword;
call 0x41ae60`. The XML readers parse with `0x64aaa0` and store with `fstp dword` before the step
(`0x6f1d91`, `0x6f1ee8`); the relative children's values are stored straight from the parse
(`0x6f1e72`, `0x6f2060`). The step proves the conversion only; which operand each method loads is
not read here.

## Region parent and anchor methods

`SetParent` takes a frame object or a frame name, refuses a parent that would make a loop, and
calls the object's slot 0x14. `SetPoint` takes a point name, an optional relative region or name,
an optional relative point and two offsets; it refuses an unknown point, a missing region, the
region itself and a region that depends on it. `SetAllPoints` does the same checks for one
relative region. `ClearAllPoints` clears every point.

Addresses: `0x7a1550`, `0x7a2540`, `0x7a2940`, `0x7a2b70`, `0x767ed0`, `0x7a2340`, `0x7a2260`, `0x7a1460`, `0x7a1390`, `0x7a1290`, `0x7a11d0`
Confidence: inferred
Proof: none
Test: `ui::luabind_7a1_get_point`, `ui::luabind_7a1_setters`

The refusals are the error strings each formats: `Would create a loop parenting to %s`, `Unknown
region point`, `Couldn't find region named '%s'`, `trying to anchor to itself` and `%s is dependent
on this`; the dependency test is `0x768010` and the setters `0x767c70` through slot 0x28 and
`0x767db0`. `0x7a2b70` calls `0x767ed0`, a `jmp` to `0x767620`. The argument shapes are from the
usage strings.

Bit-exact coverage: `ui::luabind_7a1_get_point` covers `0x7a2483` to `0x7a24c6`, `0x7a24e7` to `0x7a252a` of `0x7a2340`; `ui::luabind_7a1_setters` covers `0x7a28a6` to `0x7a28c4`, `0x7a28cc` to `0x7a28ff` of `0x7a2540`.
