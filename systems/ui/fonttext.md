# ui: font string internals

The members behind `FontString` and `Font` (`fontstring`): merging a font's style blocks, the font string's own setters
and getters, the text offset and spacing, the layout slots and the rectangle callback, the destructor, building the engine's
string from the text, and the grammar escapes and number grouping applied to text a script sets. The object is a
`CSimpleFontString`, with its font part at `+0xcc` and its layout part at `+0x24`.

Roots: `0x770910`, `0x770ad0`, `0x7714f0`, `0x772100`, `0x772240`, `0x772460`, `0x7724a0`, `0x772ca0`, `0x772d40`, `0x7716f0`

## Merging font blocks

A font block, the record a `Font` keeps and a follower reads, holds a validity word at `+0x0`, the font name `+0x4`,
height `+0x10`, flags `+0x14`, spacing `+0x18`, the justify word `+0x1c`, the text colour `+0x20`, the shadow colour
`+0x24` and the shadow offsets `+0x28`, `+0x2c`. Bits 0x100 to 0x1000 say a group is set (font, justify, colour,
shadow, spacing) and bits 0x1 to 0x10 that it changed. `0x770ad0(source)` and `0x770910(destination, mask)` copy a set
group into the receiving block only when it differs (a font by name ignoring case, a height or spacing by 2^-22) or,
except for the font, the receiving block's set bit is still clear, and then set both bits of the group.

Addresses: `0x770910`, `0x770ad0`, `0x770915`, `0x77091c`, `0x770949`, `0x770973`, `0x770984`, `0x7709a7`, `0x7709b2`, `0x7709bd`, `0x7709d3`, `0x770a07`, `0x770a2b`, `0x770a78`, `0x44d040`, `0x41e4c0`, `0x41e3a0`, `0x64a4c0`, `0x8029d4`, `0x7709f9`, `0x770a1d`, `0x770a5b`, `0x770a9d`, `0x770b9e`, `0x770bbd`, `0x770bf6`, `0x770c33`
Confidence: verified
Proof: one
Test: `ui::glue_settled_eps`

`0x770ad0` takes the source's set bits (`test ah,1` ... `ah,0x10`); `0x770910` also requires the group's changed bit in
`edx = word & mask` (`0x770920`, `test dl,1`). The equal-value test reads the receiving set bit: `test ch,0x2`
(`0x7709f9`, `0x770b9e`), `ch,0x4` (`0x770a1d`, `0x770bbd`), `ah,0x8` (`0x770a5b`, `0x770bf6`), `ah,0x10` (`0x770a9d`,
`0x770c33`). `PushChanges` zeroes the changed bits after each push (`0x7841bb`). The font group has
no set-bit test: it reads both names through `0x41e4c0` (`0x770949`), skips when the source has no font (`0x770952`), copies when the
receiving block has none (`0x770962`), and skips when `SStrCmpI` (`0x770973`) finds them equal, the heights within 2^-22 and the flags equal, or the height is
not above 0 (`0x7709a7`). It calls `TextBlockGenerateFont(name, flags, height)` (`0x7709b2`) and, if the engine
builds a font, closes the handle (`0x7709bd`), assigns the name with `0x41e3a0`, stores height and flags and
ORs 0x101 (`0x7709d3`). The other groups store their words and OR 0x202, 0x404 and 0x808, and the spacing 0x1010
(`0x770aa7`). `ui::glue_settled_eps` runs the 2^-22 compare of the height (`0x77097c` to `0x77098c`, `0x770b29` to `0x770b39`) and the
spacing (`0x770a84`, `0x770c1a`) as slices that end at `fnstsw`; the same test covers `0x7714f0` and `0x771600`.

## Font string members

A `FontString`'s own state changes go through small members. `SetFont(name, height, flags, force)` (`0x7714f0`, `ret 0x10`)
returns 1 and skips the load when `force` is 0, the current path (`0x772790`) equals the name ignoring case, the height is
within 2^-22 and the flags (`0x772810`) are equal. Otherwise, for a height other than 0, it builds a font from the name,
flags and height times the layout scale `+0x7c` with `TextBlockGenerateFont` and returns 0 when none results. It then
stores the height at `+0xe4`, closes the built string `+0xf8` and the old font, stores the new font at `+0xe0` and, when
there is one, zeroes the measures `+0xfc` and `+0x100`, clears flag 1 at `+0x60` and calls `CLayoutFrame::Resize(0)`.

Addresses: `0x7714f0`, `0x771502`, `0x77151b`, `0x771535`, `0x77153e`, `0x771556`, `0x77156c`, `0x771589`, `0x7715b3`, `0x7715e0`, `0x7715e9`, `0x772790`, `0x772810`, `0x44d040`, `0x7680e0`
Confidence: verified
Proof: one
Test: `ui::glue_7714f0_scale_mul`, `ui::glue_settled_eps`

`ui::glue_7714f0_scale_mul` runs the multiply at `0x771563` as a copied-byte island (`fld`, `fmul [esi+0x7c]`, `fstp`);
`ui::glue_settled_eps` the 2^-22 compare `0x77151f` to `0x771532` as a slice. A zero name stores the height, closes both objects, stores 0 at `+0xe0` and returns 1 without the layout call
(`0x7715b9`). `0x772790` and `0x772810` return the font's path and flags through `0x44d2c0` and `0x44d2e0`, or 0 without a font.

## Text offset and spacing

`0x772100(x, y)` sets the text offset at `+0x110`, `+0x114` when either differs, and with a built string moves it to
`(x · scale + left, y · scale + bottom)` of the stored rectangle (`+0x68`, `+0x64`) through `0x44d600`; a button's label
uses it for its pressed offset. `0x772240(v)` clamps a negative spacing to 0, and when it differs from `+0xf4` by 2^-22 or
more stores it, zeroes the height measure `+0x100` and rebuilds the built string with `0x7724a0`. `0x773500(word)` stores
the justify word `+0x120` when changed and rebuilds. `0x772460(start, length)` stores the alpha gradient pair at `+0x118`,
`+0x11c` and answers 1, or with a built string whether `0x44dc40` answers less than 2.

Addresses: `0x772100`, `0x77210f`, `0x772142`, `0x77216c`, `0x772240`, `0x772246`, `0x77225a`, `0x772275`, `0x77228f`, `0x773500`, `0x77351e`, `0x772460`, `0x77248b`, `0x7724a0`, `0x44d600`, `0x44dc40`
Confidence: verified
Proof: one
Test: `ui::textglue_scale_offset`, `ui::textglue_settled_eps`

`ui::textglue_scale_offset` runs the slice `0x772142` to `0x77216c` of `0x772100` (both position legs, halting before the call).
`ui::textglue_settled_eps` runs `0x77225a` to `0x77226d` of `0x772240`, halting before `test ah,5`. The compare `test ah,0x44; jp` at `0x772114` and `0x772124` stores when
either value differs or is NaN.

A NaN spacing skips the clamp (`jp` at `0x772251` is taken) and its difference from `+0xf4` is unordered, so `0x772270 jnp` is not taken and the NaN is stored.

## Text size reads

`0x7727b0(scaled)` returns the text height `+0xe4`; with `scaled` set and a font and bit 0x200 of `+0x120` it returns the font's
one-to-one height divided by the layout scale, times the vertical screen scale. `0x772bc0(text, length, height)` measures the
width of a text as the font engine gives it: with no font 0, otherwise `0x44d860` on the font with the text, the height
times the layout scale and the justify word, the result times the layout scale.

Addresses: `0x7727b0`, `0x7727c5`, `0x7727d1`, `0x7727da`, `0x7727de`, `0x7727e5`, `0x7727fb`, `0x772bc0`, `0x772bf2`, `0x772c0b`, `0x772c1a`, `0x44d340`, `0x5c2540`, `0x41ad80`, `0x44d860`
Confidence: verified
Proof: one
Test: `ui::textglue_div_scale`, `ui::textglue_scale_mul`

`ui::textglue_div_scale` runs the `fdiv [esi+0x7c]` of `0x7727ea` (and its three twins) as a copied-byte island; `ui::textglue_scale_mul`
the `fmul [+0x7c]` op of the family (`0x772bf2`, `0x772c0b` among them) the same way. `0x772bc0` computes the length with `0x64a6f0` when the argument is 0 (`0x772bdd`) and passes `0x7727b0(1)` as the height.

## Layout slots of a font string

The font string's layout vtable replaces slots 4 to 6. `0x772ca0(scale, force)` is slot 4: it sets the layout scale with
`CLayoutFrame::SetLayoutScale` and returns 0 when that refuses it; otherwise, with a font built, it copies the font path to a
1,024-byte stack buffer and calls `0x7714f0` with the same path, flags and the stored height, and returns 1. `0x772830(w)` and
`0x772860(h)` are slots 5 and 6: they set the width or height and zero the measures `+0xd8` and `+0xdc` of the layout part.
Main slot 9, `0x772d20`, zeroes the text measures `+0xfc`, `+0x100` and calls `Resize(0)`.

Addresses: `0x772ca0`, `0x772cbb`, `0x772cda`, `0x772d0c`, `0x772830`, `0x772860`, `0x772d20`, `0x768360`, `0x7683d0`, `0x7683f0`, `0x7680e0`
Confidence: verified
Proof: one
Test: none

`0x768360` is called at `0x772cb6`, `0x7683d0` at `0x77283a` and `0x7683f0` at `0x77286a`; the zeroing stores are `0x772841`, `0x772847`, `0x77286f`, `0x772877`.
The three tables that hold them are `0x81c74c` (slots 4 to 6 at `0x81c75c`, `0x81c760`, `0x81c764`) and the message line table `0x81cec4`.

## Font string rectangle callback

The font string's rectangle-changed callback, slot 0 of its layout vtable (`0x772d40`), runs the base callback `0x768cf0` and then
rebuilds the text (`0x7724a0`) when there is no built string or the new rectangle's width or height differs from the stored one by
1e-5 or more. Otherwise it moves the built string to `(+0x110 · scale + left, +0x114 · scale + bottom)` of the stored rectangle,
and notifies the layer (`0x77fdf0`) when the old and the new rectangle differ in `RectWhollyOffScreen`.

Addresses: `0x772d40`, `0x772d4f`, `0x772d62`, `0x772d82`, `0x772d9d`, `0x772dc4`, `0x772dc9`, `0x772dd9`, `0x772de0`, `0x772dee`, `0x768cf0`, `0x44d600`, `0x772e00`, `0x77fdf0`, `0x7724a0`, `0x80c5c8`
Confidence: verified
Proof: one
Test: `ui::textglue_rect_axis_settled`, `ui::textglue_rect_axis2_settled`, `ui::textglue_rect_pos_transform`

The width compare is `|(new.right − new.left) − (old.right − old.left)|` against 1e-5 (`0x772d62` to `0x772d80`), the height compare the same on the
other pair of edges (`0x772d82` to `0x772d9b`); either `jp` at `0x772d80`, `0x772d9b` rebuilds. The three tests run slices of it, each halting
before its call or `test ah,5`: `0x772d62` to `0x772d7d`, `0x772d82` to `0x772d98` and `0x772d9d` to `0x772dc4`. The old rectangle is `this+0x40` and its off-screen
answer `0x772e00(old)` is compared with `0x772e00(new)` (`0x772dd9`); a difference calls `0x77fdf0` on the region (`0x772de0`).

## Creating and freeing a string

`0x770e50` is the string's destructor body: it frees the text `+0xf0`, calls `SetFont(0, 0, 0, 0)`, `UnlockScriptObject(0)`, decrements
the string count `0xcf1cc4`, unlinks the font part from its font (`0x770c60(0)`) and ends in the region destructor `0x77f6f0`. `0x770e20` is the deleting form:
with bit 0 of its flag it returns the object to the block pool at `0xcf4d10` through `CDataAllocator::PutData`. `0x771680(n)` sets the fixed text
size `+0xe8`: a nonzero `n` different from the current one allocates an `n + 1`-byte text buffer from `CSimpleRender.cpp` line
863, writes a 0 first byte, frees the old buffer and stores `n` at `+0xec`. `0x770f10(node, region)` reads the XML attribute `name` and
sets it as the region's name when nonempty.

Addresses: `0x770e50`, `0x770e20`, `0x770e84`, `0x770e9d`, `0x770ea6`, `0x770eab`, `0x770ebb`, `0x770efc`, `0x771680`, `0x771697`, `0x7716a7`, `0x7716cd`, `0x770f10`, `0x770f1e`, `0x770f2f`, `0xcf1cc4`, `0xcf4d10`, `0x701cd0`, `0x77f6f0`, `0x6f2cf0`, `0x76c650`, `0x7604f0`
Confidence: verified
Proof: one
Test: none

`0x770e50` stores the three vtables back first (`0x770e62`, `0x770e68`, `0x770e6f`). `0x771680` allocates through `0x6462e0` with the file `0x878d70`, line 0x35f, and frees through `SMemFree` with line 0x362.
`0x770f10` passes the string `name` (`0x838090`) to `GetXMLAttribute` (`0x770f1e`).

## Building the string

`0x7724a0` rebuilds a string's built object `+0xf8`. It does nothing while flag 1 of `+0x60` is clear. Otherwise it closes the
old object and, for a rectangle with `bottom < top` and `left < right`, a font and a nonempty text, builds a new one: the text fitted to
the rectangle over the scale (`0x771ec0`), the size snapped to the authored or measured one within 1e-5, and `0x44d420` given the text, the
origin `(+0x110 · scale + left, +0x114 · scale + bottom)`, the size, spacing and justify word. Text with no `|c` escape gets 0x400
added to the word. A shadow and a gradient follow. Every path past the area test ends in the layer notification `0x77fdf0`.

Addresses: `0x7724a0`, `0x7724aa`, `0x7724d9`, `0x7724ea`, `0x7724f8`, `0x772565`, `0x7725bb`, `0x7725c4`, `0x7725fb`, `0x772632`, `0x772693`, `0x7726a4`, `0x772703`, `0x77270c`, `0x772727`, `0x77272f`, `0x771ec0`, `0x44d420`, `0x597ac0`, `0x44d3b0`, `0x44dc40`, `0x77fdf0`, `0x80c5c8`
Confidence: verified
Proof: one
Test: `ui::textglue_scale_offset_7724a0`, `ui::textglue_recip_span_pair`, `ui::textglue_settled_poseps`

The area test compares `+0x64` with `+0x6c` and `+0x68` with `+0x70` (`test ah,5; jp` leaves for anything not strictly less, NaN included). The
scan at `0x772590` walks the fitted text: `||` skips one byte, `|` then `C` or `c` jumps to `0x7725c4` and skips the 0x400. The width snap
is `|w − width| < 1e-5` against `0x768420`, or `+0xfc` when that is 0.0 (`0x7725e3` to `0x772611`), the height likewise with `0x768410`
and `+0x100`. The shadow alpha is the parent's alpha byte `+0xc8` times `+0x107`, divided by 255 with `0x80808081`. The three tests run slices:
`0x772515` to `0x77253a`, `0x77253a` to `0x77255c` (the `1/scale` live on the x87 across both legs) and the two compares at `0x7725fb`, `0x772632`.

## Text block creation

`TextBlockCreate(font, text, colour, position, size, width, height, word, extra, spacing)` makes an
`HTEXTBLOCK` handle on a new engine string. It fails with error 0x57 and 0 for a null font or text.
It unscales the position's x and y, divides the size, height and spacing by the y screen scale and
the width and `extra` by the x screen scale, and passes the word on as three values:

| word bits | engine value |
|---|---|
| 0x1, 0x2, 0x4, none | horizontal justification 0, 1, 2, 1 (0x4 first, then 0x2, then 0x1) |
| 0x8, 0x20, 0x10, none | vertical justification 0, 2, 1, 1 (0x8 first, then 0x20, then 0x10) |
| 0x40, 0x80, 0x100, 0x200, 0x400, 0x800 | flags 0x2, 0x800, 0x1, 0x4, 0x8, 0x10 |
| 0x1000, 0x2000, 0x4000, 0x8000 | flags 0x40, 0x100, 0x200, 0x400 |

Addresses: `0x44d420`, `0x44d437`, `0x44d451`, `0x44d494`, `0x44d49e`, `0x44d4f9`, `0x44d51a`, `0x44d53f`, `0x44d597`, `0x44d5a3`
Confidence: verified
Proof: one
Test: none

`0x44d437` raises 0x57. `0x44d451` allocates 12 bytes tagged `HTEXTBLOCK` and stores its vtable.
`0x44d494` is `UnscaleScreenXY` on the position's x and y; its z is passed as is. `0x44d49e` to `0x44d4f3`
test the bits one by one into `edi`; `0x44d4f9` sets `ebx` (horizontal) and `0x44d51a` `esi` (vertical),
each starting at 1. The size, height and spacing go through `DivideByScreenScaleY`, the width and
`extra` through `DivideByScreenScaleX` (`0x44d53f`). `0x44d597` calls `GxuFontCreateString` with
the font `+0x20` of the font handle, the string's handle stored at `+0x8`, and `0x44d5a3` wraps the
object with `CreateHandle`. The one caller is `0x772693`.

## Stock text block fonts

At start-up, after the font library starts, `0x442b00` makes the two stock text-block fonts, 0 and
1, both from `Fonts\FRIZQT__.TTF` at the height 0.01953125 (`0x3ca00000`), turned into the screen's
device height and passed to `TextBlockGenerateFont`; each replaces and closes a font already in
its slot. `0x442b50(i)` returns stock font `i` and `0x442bd0(i)` its height; both, and the maker
`0x442b70`, raise error 0x57 for an index of 2 or more, the height getter then answering 0.0. The
maker also refuses a null or empty file name. The system message layer takes stock font 1.

Addresses: `0x442170`, `0x442184`, `0x442b00`, `0x442b09`, `0x442b1e`, `0x442b2b`, `0x442b3b`, `0x442b50`, `0x442b70`, `0x442b90`, `0x442ba9`, `0x442bb6`, `0x442bc3`, `0x442bd0`, `0x88562c`, `0x80258c`, `0x40906e`
Confidence: verified
Proof: one
Test: none

`0x442184` is the tail of `0x442170`, which `0x402840` calls in start-up. `0x442b09` calls
`GxuFontInitialize` and `0x442b1e` `FormatFontsPath`; `0x442b2b` and `0x442b3b` call `0x442b70` with 0 and 1,
which jumps to `0x442b90` after its tests. `0x442b90` closes the old handle in `0x88562c + 4·i` (`0x442ba9`),
converts the height from `0x80258c + 4·i` with `MultiplyByScreenScaleY` (`0x442bb6`) and stores the new font
(`0x442bc3`). The getters compare the index with 2 (`jl`), so a negative index reads before the tables.

## Korean escape

In a text set from Lua, `|1` followed by a non-digit opens a two-way particle, `|1first;second;`, which becomes one part by the sound that
ends the text before it. A Hangul syllable with a final consonant picks the first part, one without it the second, except that a syllable
ending in `ㄹ` (final index 8) picks the second when the first starts with `으`. A digit picks by a ten-byte table (1 for 0, 1, 3, 6, 7, 8),
a Latin letter by a 26-byte table (1 for b, c, k, l, m, n, p, q); other characters and bytes after `|` are skipped, and with nothing to
look back at the first part wins.

Addresses: `0x771770`, `0x7717c7`, `0x7717e4`, `0x771836`, `0x7716f0`, `0x771730`, `0x7718b0`, `0x7718c9`, `0x7718e5`, `0x7718ef`, `0x771938`, `0x771954`, `0x77196e`, `0x771972`, `0x81c6c4`, `0x81c6d0`, `0x65b350`, `0x65b080`, `0x40f92b`
Confidence: verified
Proof: one
Test: none

`0x7716f0` finds each `|` not followed by `|` or by a digit other than the one `0x64ac60` reads, and for 1 calls `0x771770`, which
converts the text to UTF-16 into a 0x2000-byte stack buffer (`0x771795`), copies it to `0xcf2cd0` while expanding and converts back to the static `0xcf1cc8`
(`0x771896`). `0x7718b0(out, index, first, second)` returns `first` or `second`; it accepts `0xac00 ≤ ax ≤ 0xd7a3` (`cmp ax,0xac00; jb` at `0x7718c9`, `cmp ax,0xd7a3; ja` at `0x7718cf`) and tests `(ax − 0xac08) % 28 == 0` (`0x7718e5`). Letters
are lower-cased with `0x40f92b` and index the table at `0x81c6d0` (`0x77197c`); digits index `0x81c6c4`.

## French escape and digit groups

`|2` followed by a non-digit and optional spaces turns the word that follows into its "of the" form. `La`, `Le` and `L'` become `de la `, `du ` and `l'`;
any other word gets `d'` when it starts with a vowel (ASCII or the Latin-1 letters `0xc0`-`0xc6`, `0xc8`-`0xcf`, `0xd2`-`0xd6`, `0xd8`-`0xdc`
and their lower-case forms), with `h` or `H`, or with `y` or `Y` before a vowel, else `de `. When `0xcf4d24` is set, a run of digits,
`.` and `,` from a digit swaps them if it has one `.`, no `,` after it, the next byte at least `0` and the one after
at most `9` (signed): `1,234.56` gives `1.234,56`, `1.5` `1,5`, `1.55g` `1,55g`, `1.555` `1,555`; `1.5g`, `1,234,567` stay.

Addresses: `0x7719a0`, `0x7719f5`, `0x771a2b`, `0x771a3b`, `0x771a4e`, `0x771a75`, `0x771aae`, `0x771aec`, `0x771b03`, `0x771b60`, `0x771b74`, `0x771c50`, `0x771cc3`, `0x771ccc`, `0x771cd1`, `0x771cd7`, `0x771d16`, `0x771d23`, `0x771d74`, `0xcf4d24`, `0xcf1cc8`
Confidence: verified
Proof: one
Test: none

`0x7719a0` writes `d`, `e`, space at `0x771aec` to `0x771b01`, and `d`, `'` at `0x771b03`. `0x771b60` answers 1 for an ASCII vowel through the jump table `0x771bf4` indexed by the byte
table `0x771bfc` (`ch − 0x41`), or for a code in one of the Latin-1 ranges; the other characters give 0. `0x771c50` walks the text into `0xcf1cc8` and stops at 4,095 bytes (`cmp esi,0xcf2cc7; jae`). At a digit it scans the run, counting dots and
commas and keeping the last of each; it swaps only when the count of dots is 1 (`0x771cc3`), no comma lies at or past the dot
(`0x771ccc`, `jae`), `[dot+1]` passes `cmp 0x30; jl` (`0x771cd1`) and `[dot+2]` passes `cmp 0x39; jg` (`0x771cd7`). The terminating
NUL is below `9`, so `1.5` swaps, and a letter after the dot is above `0`, so `1.g5` swaps too. The copy writes `.` as `,` and `,`
as `.` (`0x771d16`, `0x771d23`); any other run, and each other byte, is copied unchanged.

## Font part and region slots

The font part at `+0xcc` has the vtable `0x81c744`: slot 0 `0x7733b0` moves `this` back by 0xcc and jumps to the deleting destructor `0x770e20`;
slot 1 `0x773530(block)` applies a font block to the string through `0x770800` with the string's inherit mask `+0xd4`. A region slot
shared by four vtables, `0x7733d0(parent)`, calls `CScriptRegion::SetRegionParent` with the region's own layer `+0xc0` and sublevel `+0xc4`; `0x7733f0` is a
`ret`, the empty slot of two vtables. `0x773490` is the `Font` object's deleting destructor (table `0x81c7a4`): it
detaches the object from its font with `0x770c60(0)`, unlinks its list node at `+0xc` and frees the object when bit 0 of its flag is set.

Addresses: `0x7733b0`, `0x7733b6`, `0x773530`, `0x773541`, `0x7733d0`, `0x7733e5`, `0x7733f0`, `0x773490`, `0x77349f`, `0x7734e1`, `0x7734f1`, `0x770e20`, `0x770800`, `0x77fd10`, `0x770c60`, `0x81c744`, `0x81c7a4`
Confidence: verified
Proof: one
Test: none

`0x81c744` stores `0x7733b0` and `0x773530` at its first two slots (`0x81c744`, `0x81c748`), and `0x773530` is also in the message line's table. `0x7733d0` is stored in four vtables (`0x81c72c` among them); `0x7733f0` in two. `0x773530` reads `[this+8]` before it subtracts 0xcc.

## The Font object

A `Font` object (`CSimpleFont`, vtable `0x81cb28`, layout part `0x81cb20`) holds a font block at `+0x38`, a justify word `+0x54` (0x212 when built), a colour word
`+0x58` (−1), its inherit mask `+0x2c` (0x1f), the word `+0x6c` (0) that `SetFontObject` copies, the font it follows `+0x28` and the list of its own followers `+0x74` with their link offset `+0x70` (0xc). `0x783a40` builds one:
it constructs the script object part (`0x701ae0`), clears the block and the follow fields, makes `+0x74` an empty list and counts the object in `0xcf4e78`. `0x783b20` destroys one: it clears its
name, detaches every follower (`0x770c60(0)` until the list is empty), decrements the count, unlinks itself from its own font and ends in the script object destructor (`0x701b40`).

Addresses: `0x783a40`, `0x783a45`, `0x783a5c`, `0x783ad3`, `0x783ae1`, `0x783b20`, `0x783b34`, `0x783b59`, `0x783b60`, `0x783c1f`, `0x783c29`, `0x701ae0`, `0x701b40`, `0x770c60`, `0x784150`, `0x81cb20`, `0xcf4e78`
Confidence: verified
Proof: one
Test: none

`0x783a40` stores 0x212 at `+0x54` (`0x783a87`), −1 at `+0x58` (`0x783a8e`), 0x1f at `+0x2c` (`0x783a98`), the list sentinel at `+0x74` with its tag (`0x783ab5` to `0x783ac4`), the vtables
`0x81cb28` and `0x81cb20` (`0x783acd`, `0x783ad3`) and increments `0xcf4e78` (`0x783adf`). `0x783b40` loops `0x770c60(follower, 0)` while the list head is not tagged.

## Font tables

A `Font`'s constructor stores two tables: the pair `0x81cb20` at `+0x24` and the vtable `0x81cb28` in
`[this]`. The pair is the thunk `0x784f80`, which steps `this` back 0x24 into the deleting destructor,
and the block merge `0x77e4b0`. The vtable has eight slots: 0 the deleting destructor `0x783af0`, 1
`0x77e4a0` (the dword at `+0x68`), 2 `0x7a1100` (a script method lookup), 3 `0x702590` (the OnEvent
script by name), 4 `0x77e4e0` (a type id test), 5 `0x784180` (`PushChanges`), 6 `0x77e500` (a type name
test) and 7 `0x77e520` (the name Font). The three floats from `0x81cb48` are the check button's constants.

Addresses: `0x81cb20`, `0x81cb28`, `0x81cb48`, `0x783acd`, `0x783ad3`, `0x784f80`, `0x77e4b0`, `0x783af0`, `0x77e4a0`, `0x7a1100`, `0x702590`, `0x77e4e0`, `0x784180`, `0x77e500`, `0x77e520`, `0x785000`, `0x785590`, `0x7855f0`, `0x7842d0`, `0x81c8b4`, `0x81c8bc`
Confidence: verified
Proof: one
Test: none

`0x783acd` and `0x783ad3` store `0x81cb28` and `0x81cb20`. The dwords from `0x81cb28` are code addresses
through `0x81cb44`; `0x81cb48` holds `0x7f800000`, `0x81cb4c` `0x40490fdb` and `0x81cb50` `0x41200000`,
which `0x785000` (+∞), `0x785590` (π + π) and `0x7855f0` (10 × 10) read. `0x7842d0` calls slot 5.
`0x784f80` is `sub ecx,0x24; jmp 0x783af0`; `0x77e4a0` loads `[ecx+0x68]`; `0x702590` compares its
argument with OnEvent (`0x872a20`) and answers `this + 0xc`; `0x77e4e0` compares with `[0xcf4ea8]`;
`0x77e500` compares a name with `[0x87a450]`, the string Font, and `0x77e520` returns it. The edit box's own
font keeps the same shape, `0x81c8b4` and `0x81c8bc`, and differs in slots 0 (`0x77e550`) and 5 (`0x77e530`).

## Font object SetFontObject

`0x7841d0(source)` is a `Font`'s `SetFontObject`, the call under `CopyFontObject`. In order it copies
the source's `+0x6c` into its own, merges the source's block into its own with `FontBlockMerge`,
ANDs its block word with that `+0x6c`, copies the source's follow mask `+0x2c`, follows the font the
source follows (`+0x28`), or none when that chain reaches itself, moving between the two fonts'
follower lists, and runs slot 5, `PushChanges`. `+0x6c` is 0 in every font (Writers of Font +0x6c), so the copy keeps the
merged values under a block word of 0: `PushChanges` sends nothing, and a follower that links to the
copy later takes nothing either, until a setter or a push from its new font sets a group again.

Addresses: `0x7841d0`, `0x7841d8`, `0x7841e7`, `0x7841ea`, `0x7841f4`, `0x7841fb`, `0x78420b`, `0x7842cc`, `0x7842d0`, `0x770ad0`, `0x783aca`, `0x784180`, `0x78418c`, `0x77e4b0`, `0x770c60`, `0x770d0c`, `0x81cb3c`
Confidence: verified
Proof: one
Test: none

`0x7841e7` stores the source's `+0x6c` (`0x7841d8`), and `0x7841f4` ANDs the word after `0x770ad0`
(`0x7841ea`); `0x7841fb` stores `+0x2c`. The chain walk from the source's `+0x28` (`0x78420b`) clears
the new font when it reaches `this`; slot 5 is `call [eax+0x14]` (`0x7842d0`), `0x784180` at
`0x81cb3c`, which returns at once for a block word of 0 (`0x78418c`). Neither merge stores past the
block, and the follower slot `0x77e4b0` merges under `+0x2c`. `0x770c60` ORs only the changed bits 0x1f
into the font's word before its follower reads it (`0x770d0c`).

## Writers of Font +0x6c

The word `+0x6c` of a `Font` is 0 from construction (`0x783aca`), and the one later store to it is
`SetFontObject` copying another font's (`0x7841e7`), so it is 0 in every font. A font exists only through the
constructor `0x783a40`, the one place that stores its vtable `0x81cb28` outside the destructor. The census is the
destination operand `[reg+0x6c]`, with or without a scaled index, of `mov`, `fst`, `fstp` and `movq` over the whole
listing: 112 stores in 89 functions; and the same at `+0x3a8`, `+0x424` and `+0x4a0`, where a Button's three
embedded fonts (`+0x33c`, `+0x3b8`, `+0x434`) keep it: 18 stores in 15 functions. Within the `ui` rows `lea` and
`rep movs` add no writer.

Addresses: `0x783a40`, `0x783aca`, `0x783acd`, `0x783b27`, `0x7841e7`, `0x7786e6`, `0x757b04`, `0x757f5c`, `0x72dd78`, `0x72df59`, `0x4eb9d7`, `0x7676c2`, `0x788079`, `0x767733`, `0x76d6a1`, `0x7786a0`, `0x5adf60`, `0x6d30e0`, `0x81cb28`
Confidence: verified
Proof: one
Test: none

The 112 are 95 `mov`, 13 `fstp`, 2 `fst` and 2 `movq`; six are `[esp+0x6c]` locals, the `movq` pair (`0x757b04`,
`0x757f5c`) among them. The constructor's and `SetFontObject`'s stores are in the count, its control. No arithmetic or
logic instruction has such a destination. Seven holders are `ui` rows: the two font writers, `CLayoutFrame`'s constructor
(`0x7676c2`), `MinimapBuildBlipQuad` (`0x4eb9d7`), the scroll frame's array pointer (`0x788079`) and two LCD objects
(`0x72dd78`, `0x72df59`). The 82 others are movement, model, lighting, record, sound, network, terrain, text and C-runtime
code (one in a `__finally` block with no row); `OnWhoList` and `OverblendAreaLight` call `ui` code, none a font function.
The 18 at `+0x3a8`, `+0x424` and `+0x4a0` are in other classes' constructors and in image, storm and cache code, none a
Button or built on its constructor (`0x7786a0`). The `ui` rows' `lea` of the four offsets is two, `CLayoutFrame`'s own list
(`0x767733`) and a model's field (`0x76d6a1`); their 76 `rep movs` (58 `DWORD`, 18 `BYTE`) copy into globals, locals,
array and text buffers and the fields of the world frame, the top frame, models and the LCD, none a font. `0x783a40` has 8
callers: the Button's three (`0x7786e6` to `0x7786fc`) and five on heap blocks.
