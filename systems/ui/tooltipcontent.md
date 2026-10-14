# ui: Tooltip lines, colours and sizing

How a `GameTooltip` holds its lines, the colours the builders pass, how the `AddLine` family reads its
Lua arguments, how a localized line is resolved and formatted, and how the plate is sized. What the
builders put in the lines is in `tooltipitem`, `tooltipunit` and `tooltipspell`; the widget's owner,
anchors and fading are in `tooltip`.

A line is a pair of font strings, left and right, kept in parallel arrays on the tooltip. The builders
are C++ functions that call the line core directly; the `Set…` Lua methods only choose a builder and its
arguments.

Roots: `0x530270`, `0x5303b0`, `0x530380`, `0x530050`, `0x530640`, `0x531630`, `0x531840`, `0x531b90`, `0x703bf0`, `0x52fa50`

## The line arrays

A tooltip counts its lines at `+0x31c` and caps them at `+0x320`. The left font strings are an array at
`+0x32c`, the right ones at `+0x338` and each line's wrap flag is in the array at `+0x344`. The line core
adds one line; two wrappers add a single left line in a given colour and a double line in gold. A line
with no text on either side is not added, and a line with a right text never wraps. Clearing, the first
act of every fresh build, hides the status bar and all font strings, empties their texts and wrap flags,
sets the count and the minimum width to 0 and runs the script slot at `+0x44c`.

Addresses: `0x530270`, `0x530277`, `0x53027d`, `0x530284`, `0x530291`, `0x5302a9`, `0x5302b4`, `0x5302d0`, `0x5302e7`, `0x5302fb`, `0x530326`, `0x53033d`, `0x53036f`, `0x530372`, `0x5303b0`, `0x5303c2`, `0x530380`, `0x530399`, `0x530050`, `0x5300a1`, `0x5301c7`, `0x530203`, `0x530244`, `0x530258`, `0x530263`, `0x77f750`, `0x771d80`
Confidence: verified
Proof: one
Test: none

`0x530270(left, right, left colour pointer, right colour pointer, wrap)` zeroes the wrap argument when
the right text is nonempty (`0x53027d` to `0x530284`), returns at a full array (`0x530291`) or when both
texts are empty (`0x5302a9` to `0x5302b4`), sets each nonempty side's colour with `0x77f750` and text with
`0x771d80`, marks it shown (`+0xc4`), stores the wrap flag (`0x53036f`) and increments the count
(`0x530372`). `0x530050` hides the status bar (`0x5300a1`), walks the attached frames (`0x5301c7`), writes the empty string
to each left text (`0x530203`), zeroes the count (`0x530244`) and the minimum width (`0x530258`), and runs
the slot (`0x530263`). `0x5303b0(text, colour pointer, wrap)` calls it with no right text and the one colour
(`0x5303c2`); `0x530380(left, right, wrap)` pushes the gold pointer `0xc0d3e8` twice (`0x53038d`,
`0x530392`).

## The colours

Builders pass a pointer to a packed `0xAARRGGBB` dword, never the value. The pointers live in `.data` and
are filled by one small initializer each. Gold `0xffffd200` is the default name and description colour,
white the ordinary line, red the failed requirement, green the on-use and learnable line, gray the
inactive one. The quality table has seven entries for Poor to Artifact.

| pointer | value | colour |
|---|---|---|
| `0xc0d3e8` | `0xffffd200` | gold |
| `0xc0cf60` | `0xffffffff` | white |
| `0xc0d390`, `0xc0d3a8` | `0xffff2020` | red |
| `0xc0d398` | `0xffff0000` | pure red |
| `0xc0d3ac` | `0xff00ff00` | green |
| `0xc0d3c4` | `0xff808080` | gray |
| `0xc0d368` | `0xffffff97` | pale cream |
| `0xc0d3c8` + 4·q | 9d9d9d, ffffff, 1eff00, 0070dd, a335ee, ff8000, e6cc80 | quality 0 to 6 |

Addresses: `0x528e50`, `0x528e90`, `0x528ed0`, `0x529190`, `0x528fd0`, `0x528f50`, `0x528f10`, `0x529050`, `0x5291d0`, `0x5291da`, `0x5291e4`, `0x5291ee`, `0x5291f8`, `0x529202`, `0x52920c`, `0x52ad70`, `0x52ad90`, `0xc0d3e8`, `0xc0cf60`, `0xc0d390`, `0xc0d3a8`, `0xc0d398`, `0xc0d3ac`, `0xc0d3c4`, `0xc0d368`, `0xc0d3c8`
Confidence: verified
Proof: one
Test: none

Each initializer is a single `mov` of the literal into the global: `0x528e50` gold, `0x528e90` white,
`0x528ed0` and `0x529190` red, `0x528fd0` pure red, `0x528f50` green, `0x528f10` gray, `0x529050`
cream; `0x5291d0` to `0x52920c` fill the quality entries. White is never rewritten by a builder. A quality
index in `ecx` (both take it in `ecx` and no stack argument) is tested as unsigned against 7 (`cmp ecx,7; jb`):
an index of 7 or more, negatives included, takes entry 1, white. `0x52ad70` returns the address of the
dword at `0xc0d3c8` + 4·index, and `0x52ad90` returns the matching inline escape string for chat text from
the table at `0x854124`.

## The line bindings

`AddLine(text, r, g, b, wrap)`, `AddDoubleLine(left, right, rL, gL, bL, rR, gR, bR, wrap)` and
`SetText(text, r, g, b, a, wrap)` read their colour only when the red argument is a number: if it is not,
the whole colour stays gold. A given red takes green and blue without their own test, so a missing one is
0.0. Each component is clamped to [0, 1], and NaN becomes 1.0. Each is then scaled by 255 and rounded half up. `AddLine` and `AddDoubleLine`
force alpha to 0xff; `SetText` reads alpha at its own index 6 with default 1.0 and clears the tooltip first.
A wrap argument is read as a boolean; a number is truncated toward zero.

Addresses: `0x531630`, `0x531840`, `0x531b90`, `0x6f3400`, `0x6f3510`, `0x6f34d0`, `0x6f3620`, `0x6f3690`, `0x6f1c10`, `0x40a2b0`, `0x6f4940`, `0x7ffd74`, `0x7ff9d8`, `0x7ffe58`, `0x7ffa24`
Confidence: verified
Proof: one
Test: none

The clamp is an `fcom` against 0.0 (`0x7ffd74`) and 1.0 (`0x7ff9d8`) with a `jp` on `test ah,5` after each
(red of `AddLine`: `0x531720`, `0x531736`). An unordered compare sets C0, C2 and C3, so `jp` (taken when neither C0 nor C2 is set, and when both are) is taken on both tests:
NaN is neither below 0 nor below 1. Every other component of the three bindings uses the same two tests.

Index 1 is the tooltip, checked as a table with a widget and the class tag; a bad one raises through
`0x6f4940`, which does not return. The text is index 2 for all three, read when `lua_isstring` (`0x6f3510`)
accepts it and null otherwise; `SetText` with no string raises its usage string and adds no line.
`AddDoubleLine` gates its left colour on index 4 and its right on index 7, and reads wrap at index 10, which
`0x530270` then zeroes for a nonempty right text. `lua_tonumber` (`0x6f3620`) returns 0.0 for a missing
slot. `0x6f1c10` takes nil as false, a number by truncation (so 0.6 is false), the strings `0`, `off` and
`disabled` as false and `1` to `9`, `on` and `enabled` as true, else the default.

## Localized lines

A builder gets the template of a line from a global string by name, and a missing one drops the line. The
lookup reads the Lua global of that name and returns its text, or the empty string; the line core then
refuses the empty text. The template is copied into a buffer of 0x80 bytes and used as a format, with
numbers passed as 32-bit values or, for the cast time, the cooldown and the chance lines, as an 8-byte
double. The formatter passes a `%g` conversion to the C runtime with the spec unchanged, so `%.3g` gives
three significant digits: a cooldown of 100000 ms reads 1.67 min.

Addresses: `0x703bf0`, `0x703c02`, `0x703cca`, `0x64a7f0`, `0x64a820`, `0x64a834`, `0x6697d0`, `0x669ab0`, `0x669b81`, `0x669a27`, `0x740107`, `0x52eb68`, `0x52ebb5`, `0x52ebc1`, `0x52ebc6`, `0x882748`, `0xc53f10`
Confidence: verified
Proof: one
Test: none

`0x703bf0(key, count, count)` presets its result to the empty string `0x882748` (`0x703c02`) and overwrites
it after `lua_tostring` succeeds (`0x703cca`). `0x64a7f0` is `SStrPrintf`; it jumps to `0x64a820`, which
branches on `[0xc53f10]` (`0x64a834`). That cell has one writer, option 10 of `0x64ec80`, which `WinMain`'s
start-up sets, so the client's own formatter `0x6697d0` runs. It classifies the conversion with a byte table
and sends `e f g E F G` down the double path (`0x669b81`), which passes the spec to the CRT's `_snprintf`
(`0x669a27`, `0x740107`). The cast-time line pushes the value with `fstp qword [esp]` (`0x52ebb5`) and cleans
three dwords plus the double (`0x52ebc6`).

## Durations

A duration in milliseconds is written by one helper from a key prefix. It picks `_DAYS` at 86,400,000 and
more, `_HOURS` at 3,600,000, `_MIN` at 60,000 and else `_SEC`, takes the ceiling of the count in the first
three when asked and a floor in seconds, looks the key up, and prints the count; an extra argument is
passed after the count. The plural variant of the key is chosen by the interface's own function: a count of
1 uses the singular spelling and every other count, 0 included, the plural.

Addresses: `0x52fa50`, `0x52fb43`, `0x52fb8c`, `0x52fb9f`, `0x52fbc4`, `0x703cf0`, `0x703bf0`, `0x703c89`
Confidence: verified
Proof: one
Test: none

`0x52fa50(dest, size; ms, prefix, extra, round up)` composes the key by appending the unit to the prefix
and passes it to `0x703bf0`, which with a count asks the Lua function `GetPluralIndex` through `0x703cf0`
(zero when the call fails). The unit thresholds are the immediates 86400000, 3600000 and 60000;
the extra argument is pushed after the count (`0x52fb8c`) and adds a leading argument to the format
(`0x52fb9f`).

## The spacer line

A blank line in a tooltip is written as a space and a newline and is one line tall, not two. The newline
that ends the text opens no further line, and the empty string adds no line at all, so no builder or
caller can ask for a line of zero height. The item set block passes this literal twice, in gold with wrap
on. A font string never reads back a height below one device pixel.

Addresses: `0x52dd5e`, `0x52dd68`, `0x52dfb9`, `0x52dfc3`, `0x854b2c`, `0x5c2070`, `0x5c20d6`, `0x5c2810`, `0x5c2896`, `0x5c7470`, `0x5c7659`, `0x5c7752`, `0x5c7260`, `0x5c2116`, `0x772a60`, `0x7729b0`, `0x771e87`
Confidence: verified
Proof: one
Test: none

`0x854b2c` holds `20 0a 00`. The classifier `0x5c2810` makes a line feed a hard break of length 1
(`0x5c2896`); the line counter `0x5c2070` increments at `0x5c2116` once per step. The wrapping stepper
`0x5c7470`, which every interface font string takes, adds the break's length to its cursor at `0x5c7659`
and returns a line end one past the newline (`0x5c7752`), so the text ends and counts as one line. The height
of N lines is `N·size + (N−1)·snapped spacing`, with the spacing zero by default. `0x772a60` returns the
larger of the declared or measured height and one device pixel.

## Sizing the tooltip

The plate is as wide as its widest line plus twice 10.24 units and the padding, and as tall as the sum of its
left lines' heights plus a gap of 2.0 units between shown lines and twice 10.24 units. A single line's width
is its left text, its right text and, when both exist, a gap of 38.4 units; a wrapped line is limited to
230.4 units and the plate widens to the longest of its pieces. The minimum width is a floor. The right
texts are then anchored at the widest line's width. All lengths are FrameXML units converted to layout units.

Addresses: `0x530640`, `0x530649`, `0x530665`, `0x5306cf`, `0x5306f8`, `0x53071a`, `0x530790`, `0x5307bd`, `0x5307cb`, `0x530837`, `0x5308a8`, `0x5308ef`, `0x53094f`, `0x530983`, `0x530995`, `0x5309c4`, `0x5309e4`, `0x5309f4`, `0x530a0c`, `0x530a2e`, `0x530a39`, `0x530a3c`, `0x530a49`, `0x530a52`, `0x808e6c`, `0x808e70`, `0x808e74`, `0x808e78`, `0x808e80`, `0x772890`, `0x772b60`, `0x772ae0`
Confidence: verified
Proof: one
Test: none

`0x530640` returns without an owner (`0x530649`). The first loop starts from the minimum width `+0x3c0`
converted to layout units (`0x530665`), and for each unwrapped shown line adds the left width, the right
width (`0x5306f8`, `0x53071a`) and the gap `0x808e70` (38.4, `0x5306cf`) when both are shown, plus the width
of any attached frame in the table at `+0x358`. The second pass, for wrapped lines (`0x530790`), compares
`0x808e80` (230.4, `0x5307bd`) with the text width (`0x772890`), splits the text with `0x772b60` into at most
20 pieces and measures each with `0x772ae0` (`0x5308a8`), and calls the line's `SetWidth` (`0x5308ef`).
The right texts are anchored with `0x767c70` at `0x53094f`. The height sum skips hidden lines (`0x530983`),
adds `0x808e6c` (2.0) for every shown line after the first (`0x530995`) and each left font string's
`GetHeight` through slot 0x20 of its geometry vtable (`0x5309c4`). The pads are `0x808e74` and `0x808e78`
(10.24, `0x5309e4`, `0x530a0c`); width is `2·pad + [+0x3bc] + widest` (`0x5309f4`) and the plate is sized by
slots 0x14 and 0x18 (`0x530a2e`, `0x530a39`). Anchor mode 6 is not re-anchored (`0x530a3c`); the others
are, with `0x52fe90(0)` (`0x530a49`), and the layout is invalidated (`0x530a52`).
