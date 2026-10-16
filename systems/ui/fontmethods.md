# ui: font accessors from Lua

How the Lua methods that set and read a font's colour, alpha, shadow, spacing and size convert their numbers, which widget
types have them, and what the three font objects of a `Button` do. The font state itself (`CSimpleFont` fields, inheriting a
`Font` object) is in `ui/fontstring`; the screen scale the sizes use is `screencoord`.

Roots: `0x79f4d0`, `0x79f680`, `0x79f730`, `0x79f910`, `0x79f9c0`, `0x79fad0`, `0x79fb40`, `0x79f3b0`, `0x79d840`,
`0x79d9c0`, `0x780880`, `0x794ed0`

## Which types answer

The shadow, offset, spacing and colour methods exist on exactly six method tables: `ScrollingMessageFrame` (`0x87b5c0`),
`MessageFrame` (`0x87b960`), `SimpleHTML` (`0x87ba80`), `EditBox` (`0x87bb68`), `FontString` (`0x87c1d8`) and `Font`
(`0x87c7c8`). `Button` (`0x879d00`) has `SetFont`, `GetFont` and the three text colour pairs (normal, `Disabled`,
`Highlight`) but no shadow, offset, spacing, justify, `SetFontObject` or `SetTextHeight` method; calling one is a nil-value
error. `Font:SetAlpha` and `Font:GetAlpha` write and read the alpha byte of the text colour; the `SetAlpha` of a `FontString` is
the region alpha. The five types other than `FontString` resolve their object and call a shared function with its name for the
usage text; `FontString` has its own copies with the same contracts but, for the text colour getter, different rounding.

Addresses: `0x879d00`, `0x87b5c0`, `0x87b960`, `0x87ba80`, `0x87bb68`, `0x87c1d8`, `0x87c7c8`, `0x792000`, `0x7945d0`, `0x7960d0`, `0x797390`, `0x79d9c0`, `0x7a0570`, `0x780ee0`, `0x781100`, `0x781260`, `0x781480`, `0x7815e0`, `0x781800`, `0x7a03f0`, `0x7a04b0`, `0x7a05f9`, `0x7a060d`, `0x7a0615`
Confidence: verified
Proof: one
Test: none

The membership is the `{name, function}` pairs of the six tables, listed with their arity in `tables/bindings.tsv`: `SetShadowColor`
has six rows, one per table above, `SetTextColor` seven, adding the `Button` row `0x780ee0`. `Button` has `SetDisabledTextColor` and
`SetHighlightTextColor` and no row for any other shadow, spacing or justify name. In `Font:SetTextColor` (`0x7a0570`) the call
at `0x7a05f9` asks the object for its name, `0x7a060d` passes it to the shared function `0x79f4d0`, and `0x7a0615` supplies the text
`<unnamed>` (at 0x84c7f0) for an object without one.

## Setting colours

The setters take red, green and blue as arguments 2 to 4 and an optional alpha as 5. Each channel is clamped to 0 to 1 (below 0
gives 0; above 1 or not a number gives 1) and a missing alpha is 1. The three colour channels are rounded to a float; the alpha is
not. Each channel is then multiplied by 255, increased by 0.5 and truncated, and the bytes are packed as `0xAARRGGBB`. A colour
equal to the stored one with its change flag set skips only the store and the change flags; the font still stops following its `Font` object for that group and is told through slot `+0x14` either way.

Addresses: `0x79f4d0`, `0x79f4e9`, `0x79f50c`, `0x79f5ce`, `0x79f5e8`, `0x79f5ef`, `0x79f5f5`, `0x79f65f`, `0x79f669`, `0x79f730`, `0x79f8ed`, `0x79d9c0`, `0x79db5d`, `0x79dd40`, `0x780ee0`, `0x781260`, `0x7815e0`, `0x7ffe58`, `0x7ffa24`
Confidence: verified
Proof: one
Test: none

`0x79f4e9` compares each argument with 0.0 and `0x79f4ff` with 1.0, using `test ah,5` and `jp`, so NaN takes the upper branch and stores 1.0
(`0x79f511`); `fstp dword` (`0x79f50c`) rounds the three colour channels. The alpha is the live x87 value (`0x79f5db`). `0x79f5e8` multiplies by the float 255
(`0x7ffe58`), `0x79f5ef` adds the float 0.5 (`0x7ffa24`) and `0x79f5f5` is `__ftol`. The bytes are assembled alpha first (`0x79f605`), then red, green and
blue. `0x79f65f` stores at `+0x58` and sets 0x404 in `+0x38`; `0x79f65d` jumps over both to `0x79f669` for an unchanged colour, which clears bit 2 of `+0x2c` and calls the slot at `0x79f671`. The shadow setter `0x79f730` has the
same shape with the colour at `+0x5c`, flags 0x808 and the inherit bit 8. `FontString` and the three `Button` pairs repeat it.

## Reading colours

The getters unpack the bytes of the stored colour and multiply each by the float 1/255, returning red, green, blue and alpha as
doubles. `FontString:GetTextColor` returns red as the unrounded product and rounds green, blue and alpha to a float before
widening them; the shared getter, the shadow getters and the `Button` getters return all four unrounded.

Addresses: `0x79f680`, `0x79f6a7`, `0x79f910`, `0x79d840`, `0x79d918`, `0x79d92a`, `0x79d958`, `0x79d975`, `0x79dbe0`, `0x781100`, `0x781480`, `0x781800`, `0x81c6b4`
Confidence: verified
Proof: one
Test: none

Each getter shifts the colour dword for a channel (`0x79f68d` red, from bit 16), converts with `fild` and multiplies by the float at
`0x81c6b4` (0.003921560011804104, one float step below 1/255), then `fstp qword` pushes it; the order is red, green, blue, alpha.
`FontString:GetTextColor` stores alpha (`0x79d92a`), green (`0x79d958`) and blue (`0x79d969`) with `fstp dword`, which rounds
them to a float, and pushes red (`0x79d975`) as the double product.

## Alpha, spacing and offsets

`Font:SetAlpha(a)` stores the low byte of the truncated product of a and 255 in the top byte of the text colour, with no clamp and
no rounding, and `Font:GetAlpha()` returns that byte times 1/255. `SetSpacing(s)`, `SetShadowOffset(x, y)` and the font height
convert between Lua units and layout units with the screen scale: a Lua value `v` is stored as `K1 · float(v / (1024 · A))` and read
back as `float(1024 · A · stored) / K1`, with `A` from `GetScaledScreenAspect()` and `K1` the x scale (1 and 0.8 at 4:3).

Addresses: `0x79f420`, `0x79f449`, `0x79f454`, `0x79f464`, `0x79f4a0`, `0x79f4b4`, `0x79f9c0`, `0x79fa16`, `0x79fa1b`, `0x79fa22`, `0x79fa28`, `0x79fad0`, `0x79fae8`, `0x79fb40`, `0x79fb6c`, `0x79f3b0`, `0x79f3d1`, `0x79f3d6`, `0x79f3e3`, `0x41ad70`, `0x41ae40`, `0x41ae60`, `0x7ffd68`, `0x806498`
Confidence: verified
Proof: one
Test: none

`0x79f449` multiplies by the double 255 (`0x806498`) and `0x79f44f` truncates; `0x79f464` stores the byte at `+0x5b` and sets 0x404. The setters
call `0x41ad70` for `A`, multiply by 1024 (`0x7ffd68`), divide the argument by it with `fdivr` (`0x79fa22`), narrow to a float and pass it to
`MultiplyByScreenScaleX` (`0x79fa28`). `GetFont` (`0x79f3b0`) returns the file name, then `DivideByScreenScaleX` of the float
`1024·A·height` (`0x79f3d6`, `0x79f3e3`), then the flags text.

## Shadow offsets and spacing

`SetShadowOffset(x, y)` needs two numbers and raises `Usage: %s:SetShadowOffset(x, y)` otherwise; `SetSpacing(s)` needs one and raises
`Usage: %s:SetSpacing(spacing)`. A stored spacing within 2^-22 of the new one is kept when its change flag is already set. On a
`FontString`, `SetShadowColor` only stores the colour; `SetShadowOffset(0, 0)` clears the shadow bit 0x100 of `+0x120`, and any other
offset sets it, so a shadow is drawn only after a nonzero offset.

Addresses: `0x79fb8a`, `0x79fbc6`, `0x79df4b`, `0x79e17e`, `0x79e070`, `0x79fbe0`, `0x7723c0`, `0x7723f0`, `0x772390`, `0x7722a0`, `0x8029d4`
Confidence: verified
Proof: one
Test: none

`0x79fb8a` compares the absolute difference with the epsilon `0x8029d4`. `0x79df4b` calls `0x7723c0`, which stores the colour at
`+0x104`; `0x79e17e` calls `0x7723f0`, which returns for an unchanged offset, stores a changed one at `+0x108` and `+0x10c`, and calls
`0x772390` (clear bit 0x100) when both parts are zero or `0x7722a0` (set it and pass the colour and offset to the font) otherwise.

## Fonts of a button

`Button:SetFont(file, height [, flags])` sets all three of the button's fonts: the normal one at `+0x33c`, the disabled one at
`+0x434` and the highlight one at `+0x3b8`, in that order, each through the shared font setter with the button's name, and returns
no value even when the font fails to load. `Button:GetFont()` reads only the normal font. A button with no label receives the call
without error: the fonts are separate objects, and the label is not touched.

Addresses: `0x780880`, `0x780927`, `0x78094e`, `0x78096e`, `0x7809a0`, `0x780a41`, `0x79f210`, `0x79f3b0`
Confidence: verified
Proof: one
Test: none

`0x780927`, `0x78094e` and `0x78096e` call `0x79f210` with `edx` set to `+0x33c`, `+0x434` and `+0x3b8` and discard its result;
the method ends with `xor eax,eax`. `0x780a41` calls `0x79f3b0` on `+0x33c` only.

## Message frame insert mode

`MessageFrame:SetInsertMode("TOP" or "BOTTOM")` compares its argument without regard to case and sets the insert mode through
`0x785ce0`; any other value raises `Usage: %s:SetInsertMode("TOP" or "BOTTOM")`. `GetInsertMode()` returns the mode as `TOP` or
`BOTTOM`. Only `MessageFrame` (`0x87b960`) has the pair; `ScrollingMessageFrame` has neither method.

Addresses: `0x794ed0`, `0x794f78`, `0x794f8a`, `0x794f9a`, `0x794fab`, `0x794fd2`, `0x794ff0`, `0x785ce0`
Confidence: verified
Proof: one
Test: none

`0x794f78` compares with `TOP` and `0x794f9a` with `BOTTOM`, each through `SStrCmpI` with the limit `0x7fffffff`. Both
branches call `0x785ce0(frame, mode)`; the error text is pushed at `0x794fd2`.

## Copying a font object

`CopyFontObject(font)` on a `Font` sets that font object's font from argument 2: a font object
table, whose `this` must be a `Font`, or the name of an existing font object. A table without
`this` raises `%s:CopyFontObject(): Couldn't find 'this' in font object`, one of another type
`... Wrong object type, expected font`, an unknown name `... Couldn't find font named %s`, and
any other argument, a number included, `Usage: %s:CopyFontObject(font or "font")`, `%s` being the
object's name or `<unnamed>`.

Addresses: `0x7a01b0`, `0x7a024d`, `0x7a025f`, `0x79f0e0`, `0x79f0fb`, `0x79f10e`, `0x79f12f`, `0x79f149`, `0x79f167`, `0x79f18b`, `0x79f1a5`, `0x79f1bd`, `0x79f1db`, `0x79f1eb`
Confidence: verified
Proof: one
Test: none

`0x7a01b0` checks its own `this`, then calls `0x79f0e0(name, font, L)` with the name from slot `+0x4`
(`0x7a024d`) or `<unnamed>` (`0x7a025f`). `0x79f0fb` tests for a table (type 5): `0x79f10e` reads index 0 as
light userdata, `0x79f12f` errors on null, `0x79f149` takes the Font type id, assigned from
`s_lastTypeId` on first use, and `0x79f167` asks the object's slot `+0x10`. `0x79f18b` tests for a string
(type 4) and `0x79f1a5` calls `0x783870(name, 0)`; `0x79f1bd` errors on null. `0x79f1db` is
`CSimpleFont::SetFontObject`; `0x79f1eb` is the usage error.
