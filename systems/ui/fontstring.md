# ui: FontString and Font

What a `FontString` keeps, how its text is set and read, how its justify and its fit to its
rectangle are decided, how its shadow draws, and how it follows a `Font` object. The glyphs,
their measures and the line breaking belong to the font engine; this doc gives what the interface
passes it.

A `CSimpleFontString` is a region of 0x130 bytes with the vtable 0x81c778 and a font part at
`+0xcc` (vtable 0x81c744). Its fields are the font object it follows `+0xd0`, the group inherit
mask `+0xd4`, the engine font `+0xe0`, the text height `+0xe4`, a fixed text size `+0xe8`, the
text `+0xf0`, the spacing `+0xf4`, the built string `+0xf8`, the measured width and height
`+0xfc` and `+0x100`, the shadow colour `+0x104` and offset `+0x108`, the justify and flag word
`+0x120`, its inherit mask `+0x124`, the line limit `+0x128` and `+0x12c`, which says whether the
string reads its own font items from XML. A `CSimpleFont` is the `Font` object.

Roots: `0x770d30`, `0x783a40`, `0x79edf0`, `0x7a10d0`

## Construction

A new string has no text and no font. Its word `+0x120` is 0x212: `CENTER`, `MIDDLE` and the
one-to-one size bit 0x200. Its inherit masks take every justify bit and every group from its font,
its shadow colour is opaque black, its line limit is 0 (none) and it reads its own font items.

Addresses: `0x770d30`, `0x770d44`, `0x770d49`, `0x770da1`, `0x770dc9`, `0x770dd3`, `0x770ddd`, `0x770de7`, `0x770dee`
Confidence: verified
Proof: one
Test: none

`0x770d30` calls the region constructor `0x77f640` (`0x770d44`), stores 0x1f at `+0xd4`
(`0x770d49`), zeroes the text and measures, 0xff000000 at `+0x104` (`0x770da1`), −1 at `+0x118`
(`0x770dc9`), 0x212 at `+0x120` (`0x770dd3`), 0x3f at `+0x124` (`0x770ddd`), 1 at `+0x12c`
(`0x770de7`) and the vtables (`0x770dee`).

## Setting the text

Setting text that is empty, or null, empties the string's text but keeps its buffer; text equal to
the current one changes nothing. With a fixed size the text is copied into that many bytes;
otherwise the buffer grows to fit. Any change drops the measures and the built string and lays the
string out again. From Lua, `SetText` raises `%s:SetText(): Font not set` on a string with no font,
turns its argument into a string, nil into null, and expands the `|1` and `|2` grammar escapes
first. `GetText` returns nil for an empty string, never `""`.

Addresses: `0x771d80`, `0x771d8d`, `0x771d98`, `0x771daa`, `0x771dba`, `0x771dea`, `0x771dfc`, `0x771e0f`, `0x771e4d`, `0x771e7e`, `0x771e8b`, `0x771e99`, `0x771eac`, `0x7716f0`, `0x79d760`, `0x79d7e5`, `0x79d81f`, `0x79d829`, `0x79d690`, `0x79d73b`
Confidence: verified
Proof: one
Test: none

`0x771d80(text, expand)` with `expand` passes the text through `0x7716f0`, which rewrites each
`|` followed by a digit that `0x64ac60` reads as 1 or 2 through `0x771770` or `0x7719a0`, and
through `0x771c50` when `0xcf4d24` is set (`0x771daa`). Null or `""` returns when there is no text
(`0x771dba`), else stores 0 in its first byte (`0x771e7e`); `0x771dea` returns on equal text;
`+0xe8` set copies with that limit (`0x771e0f`), else `0x64a620` reallocates (`0x771e4d`). It
zeroes `+0xfc` and `+0x100` (`0x771e8b`), frees `+0xf8` (`0x771e99`) and calls `0x7680e0(0)`
(`0x771eac`). `0x79d760` tests `+0xe0` (`0x79d7e5`) and calls `0x771d80(text, 1)` (`0x79d829`);
`0x79d690` pushes nil for an empty text (`0x79d73b`).

## Justify

The word `+0x120` keeps the horizontal justify in bits 0 to 2 (`LEFT` 1, `CENTER` 2, `RIGHT` 4) and
the vertical in bits 3 to 5 (`TOP` 8, `MIDDLE` 0x10, `BOTTOM` 0x20). `SetJustifyH` and
`SetJustifyV` accept any of the six names, ignoring case, keep only their own axis's bits, so a
name of the other axis clears the axis, and stop the axis from following the font. A getter
answers the first name whose bit is set, in the order above, else `UNKNOWN`. A name not in the six
raises a usage error.

Addresses: `0x79e6b0`, `0x79e75a`, `0x79e774`, `0x79e779`, `0x79e788`, `0x79e79f`, `0x79e8b0`, `0x79e5f0`, `0x79e695`, `0x79e7f0`, `0x6f1990`, `0x6f1a00`, `0x811ad0`
Confidence: verified
Proof: one
Test: none

`0x6f1990` compares the name with the six names of `0x811ad0`, pairs of a bit and a name, through
`0x64a4c0`. `0x79e6b0` merges the bit into `+0x120` under 7 (`0x79e774`), clears bits 0 to 2 of
`+0x124` (`0x79e779`), stores and rebuilds when it changed (`0x79e788`), and clears bit 1 of
`+0xd4` (`0x79e79f`); `0x79e8b0` does the same for the vertical bits. `0x79e5f0` passes `+0x120 &
7` to `0x6f1a00` (`0x79e695`), which returns the first name whose bit is set, else 0x838044.

## How justify draws

The engine is given one horizontal and one vertical choice. `RIGHT` wins over `CENTER`, which wins
over `LEFT`; `TOP` wins over `BOTTOM`, which wins over `MIDDLE`. An axis with no bit set draws
centred: `CENTER` horizontally and `MIDDLE` vertically, the same as the default.

Addresses: `0x44d420`, `0x44d4f9`, `0x44d4fb`, `0x44d502`, `0x44d50d`, `0x44d518`, `0x44d51a`, `0x44d51c`, `0x44d523`, `0x44d52b`, `0x44d536`, `0x44d597`
Confidence: verified
Proof: one
Test: none

`0x44d420` starts the horizontal result at 1 (`0x44d4fb`): bit 4 makes it 2 (`0x44d502`), else bit 2
1 (`0x44d50d`), else bit 1 0 (`0x44d518`). The vertical starts at 1 (`0x44d51c`): bit 8 makes it 0
(`0x44d523`), else 0x20 2 (`0x44d52b`), else 0x10 1 (`0x44d536`). Both go to `0x5c1c30`
(`0x44d597`) with the other bits of the word translated to the engine's flags.

## Size and fitting

A string draws at its font's pixel size one to one while bit 0x200 is set; `SetTextHeight`
with a height that differs clears the bit and draws at that height. To fit its rectangle, a
string with a width and either a line limit or a height measures how much text fits, with the
height raised to one line, or set to the limit times a line. When the text does not fit, it backs
off one character at a time from the end, appending `...`, until the shortened text fits or
nothing is left. The result is built once per change, in one 4,096-byte buffer.

Addresses: `0x771600`, `0x771611`, `0x771627`, `0x771ec0`, `0x771ee4`, `0x771efb`, `0x771f0f`, `0x771f19`, `0x771f31`, `0x771f5c`, `0x771f9e`, `0x772034`, `0x772039`, `0x772041`, `0x772059`, `0x772064`, `0x772086`, `0x7720de`, `0x800188`, `0xcf0cc0`
Confidence: verified
Proof: one
Test: `ui::glue_settled_eps`, `ui::textglue_add_reload`, `ui::textglue_linecount_scale`, `ui::textglue_scale_mul`, `ui::textglue_settled_poseps`

`0x771600` returns when `|h − +0xe4|` is below 2^−22 (`0x771611`), else clears 0x200 (`0x771627`).
`0x771ec0(w, h)` returns the text unless there is a font and text (`0x771ee4`), `w > 0`
(`0x771efb`) and `+0x128` set (`0x771f0f`) or `h > 0` (`0x771f19`). A line is `0x7727b0(1)` plus
the spacing (`0x771f31`); a limit gives `h = limit · line` (`0x771f5c`), else `h` is at least a
line (`0x771f9e`). `0x44d960` returns the bytes that fit (`0x772034`); fewer than the length
(`0x772039`) copies at most 0xffc bytes into `0xcf0cc0` (`0x772041`, `0x772059`), steps back over
continuation bytes (`0x772064`), appends `"..."` from `0x800188` (`0x772086`) and loops while
the fit is below the prefix plus 3 (`0x7720de`).

Bit-exact coverage: `ui::textglue_add_reload` covers `0x771f45` to `0x771f50` of `0x771ec0`; `ui::textglue_linecount_scale` covers `0x771f5c` to `0x771f6b` of `0x771ec0`; `ui::textglue_scale_mul` covers `0x771ff4` to `0x772034` of `0x771ec0`; `ui::textglue_settled_poseps` covers `0x771f88` to `0x771f97`, `0x771fd0` to `0x771fdf` of `0x771ec0`.

## Shadow

A string with bit 0x100 draws a second time in its shadow colour, at its shadow offset times its
layout scale. The shadow's alpha is the shadow colour's alpha times the parent frame's alpha, over
255, rounded down.

Addresses: `0x772180`, `0x77218d`, `0x7721be`, `0x7721c9`, `0x7721e0`, `0x7721f4`, `0x7721fc`, `0x77221a`, `0x772231`
Confidence: verified
Proof: one
Test: `ui::textglue_scale_mul`

`0x772180` (slot 8) calls `0x77fac0` and, with a built string, `0x44d650` for the main colour
(`0x7721be`). With `+0x120 & 0x100` (`0x7721c9`) it multiplies `+0x108` and `+0x10c` by `+0x7c`
(`0x7721e0`, `0x77221a`), multiplies the parent `+0x9c`'s alpha byte `+0xc8` by `+0x107`
(`0x7721f4`) and divides by 255 with the multiplier 0x80808081 (`0x7721fc`), and passes the colour
and offset to `0x44d3b0` (`0x772231`).

Bit-exact coverage: `ui::textglue_scale_mul` covers `0x7721e0` to `0x772223` of `0x772180`.

## Shadow default and units

A `<Shadow>` element, in a `FontString` or a `Font`, presets the colour `0xff000000`, opaque black, and the
offset (0.001, −0.001) before it reads its children; a `FontString` then sets its shadow bit whatever the
offset, zero included. `<Color>` replaces the colour. `<Offset>` replaces the offset through `0x6f1eb0`, which divides `x` and `y` by
`1024·A` and multiplies by `K1`. The default skips that conversion: it is 0.001 layout units, `0.768·√(a²+1)`
FrameXML units at aspect a, 1.28 at 4:3. The draw multiplies the stored offset by the string's scale `+0x7c`,
rounds each part to a float, and divides x by `K1` and y by `K2`. `GetShadowOffset` answers `float(1024·A·v) /
K1` for each part: ±1.2800000417232507 for the default at 4:3, ±1.566511890386744 at 16:9.

Addresses: `0x770f40`, `0x7713e4`, `0x7713f2`, `0x7713f9`, `0x77142d`, `0x771449`, `0x783c30`, `0x78405f`, `0x78406d`, `0x7840e3`, `0x7840fa`, `0x6f1eb0`, `0x6f1ee3`, `0x6f1ef7`, `0x6f1efd`, `0x6f1f36`, `0x7722a0`, `0x7722e8`, `0x7726af`, `0x7721e0`, `0x44d3b0`, `0x44d3de`, `0x44d3ed`, `0x41ade0`, `0x5c27a0`, `0x79df60`, `0x79fad0`, `0x79fae8`, `0x79faf5`
Confidence: verified
Proof: one
Test: `ui::get_scaled_pair_79fad0`, `ui::shadow_offset_default_79fad0`, `ui::textglue_scale_mul`

`CSimpleFontString::LoadXML` stores `0xff000000`, `0x3a83126f` and `0xba83126f` into its locals (`0x7713e4` to
`0x7713f2`) before it looks up `Color` (`0x7713f9`) and `Offset`, which `0x6f1eb0` reads (`0x77142d`), and then calls
`0x7722a0` (`0x771449`). `CFontObject::LoadXML` presets the same (`0x78405f` to `0x78406d`) and stores the colour and
offset at `+0x5c` to `+0x64` with the flags 0x808 (`0x7840e3` to `0x7840fa`). `0x6f1eb0` reads `x` with `SStrToFloat`
(`0x6f1ee3`), divides it by `A·1024` with `fdivr` (`0x6f1ef7`), stores a float and calls `0x41ae60` (`0x6f1efd`);
`y` takes `0x41ae60` too (`0x6f1f36`). `0x7722a0` sets bit 0x100 of `+0x120` (`0x7722e8`) and stores `+0x104` to
`+0x10c`. It, `0x772180` (`0x7721e0`) and `0x7724a0` (`0x7726af`) multiply by `+0x7c`, store floats and call
`0x44d3b0`, which divides through `0x41ade0` (`0x44d3de`) and hands the result to `0x5c27a0` (`0x44d3ed`).
`0x79df60` and `0x79fad0` store `1024·A·v` as a float and call `0x41ae40` (`0x79fae8` to `0x79faf5`). The test runs
`0x79fad0` whole at the cells `0x41ad10` writes for 4:3 and 16:9.

## Following a font

A string, an edit box, a message frame or a `Font` follows one `Font` object, which keeps a list
of those that follow it. A change to the font pushes each changed group down the list: the font
file and size, the justify, the colour, the shadow and the spacing. A follower takes a group only
while its masks inherit it; its own justify setters stop the inheriting. `SetFontObject` takes a
`Font`, the name of one, or nil, and raises on a missing name or a loop.

Addresses: `0x770c60`, `0x784180`, `0x784195`, `0x7841ae`, `0x7841bb`, `0x770800`, `0x770826`, `0x770848`, `0x77084f`, `0x770868`, `0x770876`, `0x770892`, `0x7708a2`, `0x7708a9`, `0x7708fd`, `0x79ef10`, `0x79efcc`, `0x79efe1`, `0x79f016`, `0x79f045`, `0x79f054`, `0x783870`
Confidence: verified
Proof: one
Test: none

`0x784180` walks the list from `+0x78` (`0x784195`), calls slot 1 of each follower with the change
block (`0x7841ae`) and clears its low five bits (`0x7841bb`). `0x770800` applies a group when its
bit is set in the block (0x100, 0x200, 0x400, 0x800) and in the mask: the font through `0x7714f0`
(`0x770848`), the justify as `(~mask & word) | (font & mask)` (`0x770868`, `0x770876`), the colour
through `0x77f750` (`0x7708a2`), the shadow (`0x7708a9`), the spacing (0x1000) through `0x772240`
(`0x7708fd`). `0x79ef10` looks a string up with
`0x783870` (`0x79efe1`), walks the parents `+0x28` for the target (`0x79f045`) and links with
`0x770c60` (`0x79f054`).

## SetFont

`SetFont(file, height, flags)` takes a string and a number and raises a usage error otherwise. The
flags are found as substrings, ignoring case: `OUTLINE` 1, `THICKOUTLINE` 4, `MONOCHROME` 2, so
`THICKOUTLINE` sets 5. The same file, height and flags skip the load. A height of 0 or less, or a
file the engine cannot load, returns nil; otherwise the font is replaced, its followers updated,
and it returns the number 1.

Addresses: `0x79f210`, `0x79f225`, `0x79f276`, `0x79f29e`, `0x79f2ce`, `0x79f2ec`, `0x79f2ff`, `0x79f30a`, `0x79f342`, `0x79f34e`, `0x79f363`, `0x6f1a90`, `0x811b10`, `0x64b540`
Confidence: verified
Proof: one
Test: `ui::set_scaled_changed_spilled_79f210`

`0x79f210` requires a string and a number (`0x79f225`), converts the height with `0x41ae60`
(`0x79f276`), reads the flags with `0x6f1a90` (`0x79f29e`), which ORs the bit of each name in
`0x811b10` that `0x64b540` finds in the string. Equal file (`0x79f2ce`), height and flags
(`0x79f2ec`) skip the load; a height not above 0 pushes nil (`0x79f2ff`); `0x44d040` builds the font
(`0x79f30a`), and slot 0x14 notifies (`0x79f342`) before 1.0 is pushed (`0x79f34e`).

Bit-exact coverage: `ui::set_scaled_changed_spilled_79f210` covers `0x79f264` to `0x79f287`, `0x79f2d7` to `0x79f2ea` of `0x79f210`.

## Lua methods

`FontString` has 32 methods at `0x87c1d8`, registered by `0x79edf0`, each its own function. `Font`
has 22 at `0x87c7c8`, registered by `0x7a10d0`, which call the shared font functions that
`EditBox`, the message frames and `SimpleHTML` also call. `Texture` has 22 at `0x87c128`. Each type
then asks the `Region` methods (`methods`).

Addresses: `0x87c1d8`, `0x79edf0`, `0x87c7c8`, `0x7a10d0`, `0x87c128`, `0x79c5f0`, `0x79c660`, `0x79c720`, `0x79c830`, `0x79ca90`, `0x79cb70`, `0x79cc80`, `0x79cdb0`, `0x79ce70`, `0x79cf30`, `0x79cff0`, `0x79d0b0`, `0x79d1a0`, `0x79d400`, `0x79d4f0`, `0x79d690`, `0x79d760`, `0x79d840`, `0x79d9c0`, `0x79dbe0`, `0x79dd40`, `0x79df60`, `0x79e070`, `0x79e1d0`, `0x79e2b0`, `0x79e3c0`, `0x79e510`, `0x79e5f0`, `0x79e6b0`, `0x79e7f0`, `0x79e8b0`, `0x79e9f0`, `0x79ead0`, `0x79fda0`, `0x79fe60`, `0x79ff60`, `0x7a0030`, `0x7a00f0`, `0x7a01b0`, `0x7a0270`, `0x7a0330`, `0x7a03f0`, `0x7a04b0`, `0x7a0570`, `0x7a0630`, `0x7a06f0`, `0x7a07b0`, `0x7a0870`, `0x7a0930`, `0x7a09f0`, `0x7a0ab0`, `0x7a0b70`, `0x7a0c30`, `0x7a0cf0`, `0x7a0db0`, `0x79ef10`, `0x79f090`, `0x79f0e0`, `0x79f3b0`, `0x79f420`, `0x79f4a0`, `0x79f4d0`, `0x79f680`, `0x79f730`, `0x79f910`, `0x79f9c0`, `0x79fad0`, `0x79fb40`, `0x79fbe0`, `0x79fc20`, `0x79fcb0`, `0x79fce0`, `0x79fd70`
Confidence: verified
Proof: one
Test: `ui::color_byte_79f420`, `ui::get_scaled_79d400`, `ui::get_scaled_79e510`, `ui::get_scaled_79f3b0`, `ui::get_scaled_79fbe0`, `ui::get_scaled_pair_79df60`, `ui::get_scaled_pair_79fad0`, `ui::get_scaled_slice`, `ui::luabind_79c830_set_vertex_color_quantize`, `ui::luabind_79ca90_get_vertex_color`, `ui::luabind_79cb70_set_alpha`, `ui::normalize_byte_79f4a0`, `ui::normalize_channel_full_domain`, `ui::pack_color_79d9c0`, `ui::pack_color_79dd40`, `ui::pack_color_79f4d0`, `ui::pack_color_79f730`, `ui::set_scaled_79e2b0`, `ui::set_scaled_changed_79fb40`, `ui::set_scaled_pair_79e070`, `ui::set_scaled_pair_79f9c0`, `ui::set_scaled_slice`, `ui::set_spacing_guarded_79e3c0`, `ui::unpack_color_79d840`, `ui::unpack_color_79dbe0`, `ui::unpack_color_79f680`, `ui::unpack_color_79f910`

The tables are pairs of a name and a function: `FontString` from `GetDrawLayer` (`0x79c660`) to
`CanNonSpaceWrap` (`0x79ead0`), `Font` from `GetObjectType` (`0x79fda0`) to `GetJustifyV`
(`0x7a0db0`), `Texture` from `GetDrawLayer` to `IsDesaturated`. The registrars call `0x701d80`
with 0x20, 0x16 and 0x16. Each `Font` setter or getter calls one shared function, `SetFontObject`
`0x79ef10` to `GetJustifyV` `0x79fd70`, which `EditBox`'s font methods call too. No table holds
`GetStringHeight`: the image has no such string, against one `GetStringWidth`.

The `ui::` tests cover slices of these functions; each test header names its instructions.
