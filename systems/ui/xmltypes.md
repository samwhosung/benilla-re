# ui: what each widget type reads from XML

The handler names each frame type adds to the frame's own, and the elements and attributes each
type's load reads after the frame's (`frames`, `Loading from XML`). The loaders live in the widget
files between `CSimpleModel` and `CSimpleMovieFrame`; this doc gives what they read and the few
rules a reimplementation gets wrong, not what each widget then does with a value, which belongs to
the widget types' own docs.

Roots: `0x7788c0`, `0x785170`, `0x779fb0`, `0x785910`, `0x76cac0`, `0x786bc0`, `0x787b20`, `0x789580`, `0x78a130`, `0x782ef0`, `0x78b3f0`, `0x778c50`, `0x77a310`, `0x76cc80`, `0x786c40`, `0x787d90`, `0x789750`, `0x7a3700`, `0x7830d0`, `0x78b4f0`, `0x78c2e0`

## Handler names by type

A frame finds a handler's slot by name through its slot 0xc; each type's resolver asks the frame's
first (`frames`, `Script slots by name`) and then compares its own names, ignoring case.
`CheckButton` shares `Button`'s resolver, `ScrollingMessageFrame` asks `SimpleHTML`'s first, and
`MessageFrame` and `Frame` add none.

| type | resolver | names it adds |
|---|---|---|
| `Button`, `CheckButton` | `0x778c50` | `OnClick`, `OnDoubleClick` |
| `EditBox` | `0x77a310` | `OnEnterPressed`, `OnEscapePressed`, `OnSpacePressed`, `OnTabPressed`, `OnTextChanged`, `OnTextSet`, `OnCursorChanged`, `OnInputLanguageChanged`, `OnEditFocusGained`, `OnEditFocusLost` |
| `Model` | `0x76cc80` | `OnUpdateModel`, `OnAnimFinished` |
| `ScrollFrame` | `0x786c40` | `OnHorizontalScroll`, `OnVerticalScroll`, `OnScrollRangeChanged` |
| `SimpleHTML` | `0x7a3700` | `OnHyperlinkEnter`, `OnHyperlinkLeave`, `OnHyperlinkClick` |
| `ScrollingMessageFrame` | `0x787d90` | `OnMessageScrollChanged` |
| `Slider` | `0x789750` | `OnValueChanged` |
| `StatusBar` | `0x7830d0` | `OnValueChanged` |
| `ColorSelect` | `0x78b4f0` | `OnColorSelect` |
| `MovieFrame` | `0x78c2e0` | `OnMovieFinished`, `OnMovieShowSubtitle`, `OnMovieHideSubtitle` |

Addresses: `0x778733`, `0x785033`, `0x779b28`, `0x76c98f`, `0x786a99`, `0x789e44`, `0x787742`, `0x78945a`, `0x782db0`, `0x78b2a2`, `0x78c14e`, `0x7856ca`, `0x7692e5`, `0x76a0d0`, `0x787d9b`, `0x7786a0`, `0x785010`, `0x7799b0`, `0x76c8e0`, `0x786a40`, `0x789dd0`, `0x787670`, `0x789400`, `0x782d80`, `0x78b220`, `0x78c0d0`, `0x785640`
Confidence: verified
Proof: one
Test: none

Each constructor stores its type's vtable: `0x778733` (Button), `0x785033` (CheckButton),
`0x779b28` (EditBox), `0x76c98f` (Model), `0x786a99` (ScrollFrame), `0x789e44` (SimpleHTML),
`0x787742` (ScrollingMessageFrame), `0x78945a` (Slider), `0x782db0` (StatusBar), `0x78b2a2`
(ColorSelect), `0x78c14e` (MovieFrame); dword 3 of each is the resolver in the table, and the
MessageFrame vtable's (`0x7856ca`) is `0x76a0d0`, as the frame's (`0x7692e5`). Each resolver
calls `0x76a0d0` first, or `0x7a3700` (`0x787d9b`), then `SStrCmpI` against its names in the
order above.

## Each type's load

A type's load is slot 8 of its layout part's vtable. Each runs the frame's load first, which applies
an inherited template through the same slot, so a template's typed elements are read before the
frame's own; `CheckButton` runs `Button`'s load first, and `MovieFrame` uses the frame's load
unchanged. Each then reads its own children and attributes, by name and ignoring case.

Addresses: `0x778739`, `0x785039`, `0x779b2e`, `0x7856d0`, `0x76c995`, `0x786a9f`, `0x787748`, `0x789460`, `0x789e4a`, `0x782db6`, `0x78b2a8`, `0x78c154`, `0x7692eb`, `0x7788d6`, `0x78517f`, `0x769820`, `0x76985c`
Confidence: verified
Proof: one
Test: none

Each constructor stores the layout part's vtable right after the main one (`0x778739` to
`0x78c154`, and `0x7692eb` for `Frame`); dword 2 of each is `0x7788c0`, `0x785170`, `0x779fb0`,
`0x785910`, `0x76cac0`, `0x786bc0`, `0x787b20`, `0x789580`, `0x78a130`, `0x782ef0`, `0x78b3f0` and,
for `MovieFrame` as for `Frame`, `0x769820`. The first call of each is `0x769820` (`0x7788d6` for
`Button`), except `0x785170`, whose first is `0x7788c0` (`0x78517f`). `0x769820` re-enters slot 8
with the template (`0x76985c`).

## Elements and attributes by type

| type | elements | attributes |
|---|---|---|
| `Button` | `NormalTexture`, `PushedTexture`, `DisabledTexture`, `HighlightTexture`, `ButtonText`, `NormalFont`, `DisabledFont`, `HighlightFont`, `NormalColor`, `DisabledColor`, `HighlightColor`, `PushedTextOffset`, `NormalText`, `HighlightText`, `DisabledText` | `text` |
| `CheckButton` | `CheckedTexture`, `DisabledCheckedTexture` | `checked` |
| `EditBox` | `FontString` (with `bytes`), `HighlightColor`, `TextInsets` | `font`, `letters`, `blinkSpeed`, `autoFocus`, `multiLine`, `numeric`, `password`, `ignoreArrows`, `historyLines` |
| `MessageFrame` | `FontString` (with `bytes`), `TextInsets` | `font`, `fade`, `displayDuration`, `fadeDuration`, `insertMode` (`BOTTOM` or not) |
| `ScrollingMessageFrame` | `FontString` (with `bytes`), `TextInsets` | `font`, `fade`, `displayDuration`, `fadeDuration`, `maxLines` |
| `SimpleHTML` | `FontString`, `FontStringHeader1` to `3` | `font`, `file`, `hyperlinkFormat` |
| `Model` | `FogColor` | `file`, `scale`, `fogNear`, `fogFar` |
| `ScrollFrame` | `ScrollChild` | none |
| `Slider` | `ThumbTexture` | `drawLayer`, `minValue`, `maxValue`, `valueStep`, `defaultValue`, `orientation` |
| `StatusBar` | `BarTexture`, `BarColor` | `drawLayer`, `minValue`, `maxValue`, `defaultValue`, `orientation` |
| `ColorSelect` | `ColorWheelTexture`, `ColorWheelThumbTexture`, `ColorValueTexture`, `ColorValueThumbTexture` | none |

Addresses: `0x7788c0`, `0x785170`, `0x779fb0`, `0x785910`, `0x787b20`, `0x78a130`, `0x76cac0`, `0x786bc0`, `0x789580`, `0x782ef0`, `0x78b3f0`, `0x6f2cf0`, `0x6f2c50`, `0x64a4c0`
Confidence: verified
Proof: one
Test: none

Each name is the string a loader passes to `0x6f2cf0` (attributes), `0x6f2c50` (a named child) or
`SStrCmpI` against a child's tag, read in each body in the order of the table. `Button` reads
`text` last (`0x778c07`); `EditBox`, `MessageFrame` and `ScrollingMessageFrame` read `bytes` on the
`FontString` child; `MessageFrame` compares `insertMode` with `BOTTOM` (`0x785a50`); `Slider` and
`StatusBar` report `Frame %s: Unknown orientation %s in element %s` for an unknown orientation.

## The text attribute

`Button` and `FontString` read `text` the same way. An absent or empty attribute is skipped: the
label keeps what a template or the constructor gave it. Otherwise the value names a global string:
when that global is missing, is neither a string nor a number, or is empty, the value itself is
used, so
`text="Continue"` with no such global shows `Continue`. The keyboard display's `LCDText` loader
reads `text` the same way into a stack buffer of 1,028 bytes with no bound, and stores `""` for an
absent attribute.

Addresses: `0x778c07`, `0x778c15`, `0x778c23`, `0x778c31`, `0x771006`, `0x771016`, `0x771024`, `0x771032`, `0x729220`, `0x7292a6`, `0x7292c9`, `0x7292d6`, `0x7292ea`, `0x729319`, `0x703bf0`, `0x704350`, `0x8621e4`
Confidence: verified
Proof: one
Test: none

`0x778c15` and `0x771016` skip on a null or empty value. `0x703bf0` with a count of −1 and no
gender reads the global through `0x704350`, answering `""` when it is not a string or number;
`0x778c31` and `0x771032` replace an empty answer with the value (`mov eax,esi`). `0x7292d6` copies
the answer into `[ebp-0x424]` with the limit 0x7fffffff, `0x7292ea` copies the value instead when
it is empty, and `0x729319` copies `""` for an absent one; the frame is 0x424 bytes and locals sit
at `[ebp-0x20]` and above.

## EditBox flags

The constructor sets the flag word `+0x318` to 1, so `autoFocus` is on and `multiLine`, `numeric`,
`password` and `ignoreArrows` are off until the XML says otherwise. An absent or empty flag
attribute changes nothing; a present one sets or clears its bit by whether it reads as true.

Addresses: `0x7799b0`, `0x779a29`, `0x779a2e`, `0x77a0a5`, `0x77a0b3`, `0x77a0bc`, `0x77a0c5`, `0x77a470`, `0x6f1b30`
Confidence: verified
Proof: one
Test: none

`0x779a2e` stores 1 at `+0x318`. For `autoFocus` (`0x77a0a5`) a null or empty value jumps past the
setter (`0x77a0b3`); otherwise `0x6f1b30` converts it (`0x77a0bc`) and `0x77a470` (`0x77a0c5`) sets
or clears bit 0. The other four flags follow the same shape with their own setters.

## Slider and StatusBar values

`Slider` reads `drawLayer` (default `OVERLAY`, 3) for its thumb, then each `ThumbTexture`. Only when
both `minValue` and `maxValue` are present does it set the range, then `valueStep` and
`defaultValue` when present; `orientation` is read either way. `StatusBar` sets its range and then
`defaultValue` under the same condition.

Addresses: `0x789580`, `0x7895a2`, `0x78960c`, `0x789632`, `0x789656`, `0x78967c`, `0x7896a2`, `0x7896ae`, `0x782ef0`, `0x782fb7`, `0x782fd5`, `0x782ff9`, `0x783021`
Confidence: verified
Proof: one
Test: none

`0x7895a2` stages 3 before `drawLayer` is read. A null or empty `minValue` (`0x78960c`) or
`maxValue` (`0x789632`) jumps to `orientation` (`0x7896ae`); else `0x789656` calls `0x7898f0` with
both, `0x78967c` `0x789a60` with the step and `0x7896a2` `0x789930` with the default. `0x782fb7` and
`0x782fd5` gate `0x783270` (`0x782ff9`) and the default's slot 0x90 (`0x783021`) the same way.

## Scroll child

`ScrollFrame` creates the first child frame of its `ScrollChild` element from XML with itself as
the parent argument and makes it the scroll child. An empty `ScrollChild` reports
`Frame %s: Scroll frame created without scroll child`; a child that fails to create sets nothing.

Addresses: `0x786bc0`, `0x786bdc`, `0x786be5`, `0x786bf4`, `0x786c00`, `0x786c28`, `0x87a6c0`
Confidence: verified
Proof: one
Test: none

`0x786bdc` finds `ScrollChild` (`0x87a6c0`); `0x786be5` reads its first child; `0x786bf4` calls
`0x6ee280` with the frame's base in `edx`; a nonzero result goes to `0x786ce0` (`0x786c00`); an
absent child reports at `0x786c28`.

## Regions and fonts

`Texture` reads `inherits` (a miss reports `Couldn't find inherited node: %s`), `hidden`, then
`TexCoords` (`left`, `right`, `top`, `bottom`), `Color`, `Gradient`, `file` and `alphaMode`.
`FontString` reads `inherits`, `hidden`, `text`, `nonspacewrap`, `bytes`, `maxLines`, `font` with a
`FontHeight` child, `outline` (`NORMAL` or `THICK`), `monochrome`, `spacing`, `justifyV`,
`justifyH`, `Color` and `Shadow` (with `Color` and `Offset`). A `Font` reads `inherits` among
fonts, then the same font items from `font` to `Shadow`.

Addresses: `0x76fe20`, `0x76fe2e`, `0x76fe66`, `0x770179`, `0x770f40`, `0x770f4e`, `0x771410`, `0x783c30`, `0x783c3e`, `0x783d38`
Confidence: verified
Proof: one
Test: none

The names are the strings `0x76fe20`, `0x770f40` and `0x783c30` pass to `0x6f2cf0`, `0x6f2c50` and
`SStrCmpI`, in the order given: `0x76fe2e` to `0x770179` for `Texture`, `0x770f4e` to the `Offset`
child for `FontString`, `0x783c3e` to the `Offset` child for `Font`, whose `inherits` miss reports
`Couldn't find inherited font: %s` (`0x783d38`).
