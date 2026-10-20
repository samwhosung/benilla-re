# ui: regions in the layout

How a texture or a font string, the regions a frame draws, takes a rectangle: the units the layout
is measured in, the size it reports when none is authored, the anchor it is given when it has
none, and what happens to one whose rectangle cannot be resolved.

A region embeds a `CLayoutFrame` at `+0x24` like a frame (`anchors`), with its own vtable, so the
resolver asks the region for its size through a virtual call. The classes are `CSimpleTexture`
(`0x76fc40`, vtables `0x81c718` and `0x81c6ec`) and `CSimpleFontString` (`0x770d30`, vtables
`0x81c778` and `0x81c74c`). A region's script object, name and type test are the frame's
(`frames`); its drawing is in `texture` and `draworder`.

Roots: `0x7701c0`, `0x771480`, `0x770720`, `0x770790`, `0x772930`, `0x772a60`, `0x770670`, `0x772d40`, `0x772e50`, `0x7706e0`

## Layout space and units

The layout is measured in a space whose screen is the rectangle `[0, K1] × [0, K2]`, with `K1 = a /
√(a² + 1)` and `K2 = 1 / √(a² + 1)` for the configured aspect `a`, so the screen diagonal is 1. The
root's rectangle is set to exactly that when the top is built. A length in interface units `u` is `u ·
K1 / (0.75·a·1024)`, so the screen is about 768 units tall and `768·a` wide at any aspect, and one
unit is about the screen height over 768 in device pixels; the float steps are
`screencoord#script-units-and-layout-units`.

Addresses: `0x764180`, `0x764284`, `0x764287`, `0x76428f`, `0x41ad10`, `0x41ad80`, `0x41ae40`, `0x41ae50`, `0x41ae60`, `0x41ae70`, `0x41ad70`, `0x832a44`, `0x832a48`, `0x832a4c`, `0x7ffd68`, `0x6f1eb0`, `0x6f1eeb`, `0x6f1efd`
Confidence: verified
Proof: one
Test: none

`0x41ad10(a)` stores `a` at `0x832a40`, `a·0.75` at `0x832a4c`, `1/√(a²+1)` at `0x832a48` (`0x41ad3e`)
and that times `a` at `0x832a44` (`0x41ad47`). The top's constructor passes 1.0 and 1.0 to `0x41ad80`
with the right edge `+0x4c` and the top edge `+0x48` (`0x764284` to `0x76428f`), which stores `K1` and `K2`,
and then sets the valid bit (`0x764294`). `0x6f1eb0` converts an attribute with `fdivr` of the value by
`[0x832a4c]·1024` and `0x41ae60`, which multiplies by `K1`; the multiplier `0x7ffd68` is 1024.0.

## The layout vtable

The layout part of every frame and region has a vtable of eleven slots: 0 the rectangle-changed
callback, 1 the destructor, 2 the XML load, 3 the default anchor target, 4 set scale, 5 set width, 6 set
height, 7 get width, 8 get height, 9 a mirror test that answers 0 and 0xa the resolver of a relative
frame's name. Slots 1 and 2 differ by class in every table: each of the 31 eleven-slot tables has its own destructor,
and slot 2 is a loader shared by a few classes or the class's own. Slots 0, 7 and 8 differ in some. The font string's
two tables also replace slots 4, 5 and 6.

Addresses: `0x81c400`, `0x81c46c`, `0x81c718`, `0x81c778`, `0x81c6ec`, `0x81c74c`, `0x81cec4`, `0x81c5d8`, `0x81c8b4`, `0x81c8bc`, `0x81c8dc`, `0x768420`, `0x768410`, `0x770720`, `0x770790`, `0x772930`, `0x772a60`, `0x76d080`, `0x76d0d0`, `0x770670`, `0x772d40`, `0x76c700`, `0x76c6e0`, `0x46ff60`, `0x7671a0`, `0x767573`, `0x767579`
Confidence: verified
Proof: one
Test: `ui::layout_composed_resolve`

Slot 0 is the callback `0x76b580` in most tables, and slots 7 and 8 are the plain frame's `0x768420` and `0x768410` or a class's own. The font string's tables store `0x772ca0`, `0x772830` and `0x772860` in slots 4 to 6. Slot 2 is `0x769820` in the plain frames, `0x76cac0` in the model tables, and `0x7788c0`, `0x782ef0`, `0x767800` or `0x770f40` in others. The edge resolvers read the size with `mov eax,[esi]; call [eax+0x1c]` (`0x767573`, `0x767579`) and
`[eax+0x20]`. Of the 34 vtables stored at `+0x24` by a constructor (a census of `mov [reg+0x24], 0x8xxxxx`),
the width and height slots differ from the plain pair in the texture's `0x81c6ec`, the font string's
`0x81c74c` and `0x81cec4`, and the five model tables `0x803934`, `0x808234`, `0x80837c`, `0x8084cc` and `0x81c5d8`
(`modelview`). Of the other 26, 23 keep the plain pair, `0x81cb20` holds two slots before a Font's vtable `0x81cb28`, `0x81c8b4` two before the edit box's own font's main table `0x81c8bc` (the edit box's
one-slot `0x81c8dc` at `+0x314` follows it), and
`0x81c7a4` is the font object's base table: three slots followed by floats (`0x7f800000`, `0x40490fdb`,
`0x41200000`, repeated). Slot 9 is `0x46ff60`, `xor eax,eax; ret`.

`ui::layout_composed_resolve` runs the real edge resolvers through `0x767a20`, with the geometry vtable `0x81c400` in place.

## A texture's size

A texture's width is the authored width, and when that is exactly 0.0 and the texture has an image,
the image's width in texels taken as interface units; its height likewise. Without an image the
answer is 0.0. The image is loaded on demand.

Addresses: `0x770720`, `0x770790`, `0x770729`, `0x77072e`, `0x770736`, `0x770739`, `0x77073b`, `0x77074e`, `0x770778`, `0x44b3b0`, `0x44b3c1`, `0x44b3ee`, `0x44b3fd`, `0x6f1eb0`
Confidence: verified
Proof: one
Test: `ui::glue_unit_convert_1024`

`0x770720` calls the plain getter, compares with 0.0 by `fcom; test ah,0x44; jp`, which falls through
on equal only, loads the image object `[this+0xa8]` (the texture's `+0xcc`) and calls `0x44b3b0` with the load
flag set and a null height out-pointer. The texel width is converted as `0x6f1eb0` does: divided by
`[0x832a4c]·1024` and multiplied by `K1`. `0x44b3b0` loads the image when `[+0x120]` is 0 and the flag is set
(`0x44b3c1` to `0x44b3e5`), then stores `+0x144` and `+0x148` through the non-null out-pointers. `0x770790` is
the height twin with `+0x54` and `+0x148`. `ui::glue_unit_convert_1024` runs the `fmul` by 1024.0 and `fdivr` of the unit conversion as a copied-byte island of the family (`0x770720`, `0x770790`, `0x770f40`).

## A font string's size

A font string's width is the authored width, or when that is exactly 0.0 the width of its text,
and is never less than one interface unit; its height likewise. A single anchor therefore always
fixes the opposite edge of a font string.

Addresses: `0x772930`, `0x772a60`, `0x772890`, `0x7729b0`, `0x772937`, `0x77294a`, `0x772957`, `0x772971`, `0x77297a`, `0x772983`
Confidence: verified
Proof: one
Test: `ui::textglue_set_scaled_chain`

`0x772930` stores the plain width, tests it against 0.0 and, when equal, calls `0x772890` for the
measured width (`0x77294f`). The floor is `1.0 / ([0x832a4c]·1024)` times `K1` (`0x772957` to
`0x77296c`). `fcomp` of the floor with the candidate and `test ah,5; jp` return the floor when the
floor is at least the candidate or either is NaN, the candidate otherwise. `0x772a60` repeats it with
`0x768410` and `0x7729b0`. `0x772890` returns the cached `+0xfc` when it is nonzero, else measures the text with the font
`+0xe0` and divides by the scale `+0x7c`; it is 0.0 without a font or text. `ui::textglue_set_scaled_chain` runs the floor of `0x772930` and `0x772a60`: the `fmul`/`fdivr` bytes of `0x77295c` and `0x772963` as a copied-byte island, then `0x41ae60`.

## The implicit anchor

When a layer region is created from XML with a parent and no anchor point in any of its nine slots,
a texture is pinned to its parent with `SetAllPoints` and a font string gets one point on its
parent chosen by its horizontal justification: LEFT to LEFT, RIGHT to RIGHT, otherwise CENTER to
CENTER, with zero offsets. The test is on the slots, so an `<Anchor>`, a template's anchors or
`setAllPoints` suppress it alike. A texture so pinned ignores its authored size while both corners
stand. The frame loader applies none; a frame without anchors stays rectless.

Addresses: `0x7701c0`, `0x7701c1`, `0x7701cc`, `0x7701d4`, `0x7701dd`, `0x7701ec`, `0x771480`, `0x7714a3`, `0x7714ae`, `0x7714bc`, `0x7714c6`, `0x7714d2`, `0x7714e4`, `0x767db0`, `0x767c70`, `0x767800`, `0x767a06`, `0x767a12`
Confidence: verified
Proof: one
Test: none

Both functions load the parent `[this+0x9c]`, return without it, and scan the slots `[this+0x28]` to
`[this+0x48]` for a non-null one. The texture calls `0x767db0(parent layout, 1)`: corners 0 and 8,
offsets 0 (`anchors`). The font string tests the justify word `[this+0x120]` for bit 1 then bit 4 and calls `0x767c70(3,
parent, 3, 0, 0, 1)`, `(5, parent, 5, …)` or `(4, parent, 4, …)`. `0x767800` calls `0x767db0` only when the
`setAllPoints` attribute is present (`0x767a06` to `0x767a12`). With both corners pinned every edge
resolves from a point and the size is never asked.

## Where it applies

The implicit anchor runs after the region's own load, at creation from XML, and for a Lua-created
region only when it inherits a template. `CreateTexture` and `CreateFontString` without a template
run no load and no implicit anchor, so such a region stays rectless until it is anchored.

Addresses: `0x6f26f0`, `0x6f2751`, `0x6f275e`, `0x6f2765`, `0x6f2780`, `0x6f27f5`, `0x773a20`, `0x773ba6`, `0x773ba8`, `0x773bef`, `0x773c30`, `0x773ddc`, `0x773e25`, `0x778b96`, `0x7788c0`, `0x769d70`
Confidence: verified
Proof: one
Test: none

`0x7701c0` has two callers, `0x6f2765` in the texture element adder `0x6f26f0` and `0x773bef` in
`CreateTexture`. `0x771480` has three: `0x6f27f5` in the font string adder `0x6f2780` (which the layer
walker `0x769d70` and a button's text element both reach), `0x773e25` in `CreateFontString` and
`0x778b96` in the button load. The adder runs the pre-load `0x76fdf0`, the virtual load `[vtable+8]`
(`0x6f275e`), then the anchor. In `CreateTexture` `ebx` is zero unless the template name resolved
(`0x773ab5`, `0x773b29`) and `test ebx,ebx; je` (`0x773ba6`) skips the load and the anchor; the
font string has the same gate at `0x773ddc`. Thirteen sites call `0x6f26f0`: the layer walker, four
button state textures and eight other widgets' texture elements.

## A region that does not resolve

A region whose rectangle fails to resolve is retried six times and then left. A texture that never
resolved still emits a quad, with all four vertices at the origin; a texture whose rectangle
resolved once and then failed keeps drawing at the last good rectangle. A font string whose
rectangle is invalid emits nothing. Nothing in the resolver or the draw falls back to the parent's
edges.

Addresses: `0x768d20`, `0x768d55`, `0x768d58`, `0x7680e0`, `0x7680ee`, `0x768140`, `0x76815f`, `0x768ed0`, `0x76fc6e`, `0x7705b0`, `0x770670`, `0x7706e0`, `0x772fd0`, `0x773080`, `0x7730b0`, `0x7730b5`, `0x7730bc`, `0x772e00`, `0x768320`
Confidence: verified
Proof: one
Test: `ui::layout_recompute`

`0x768d20` on failure clears flag 1, sets flag 8 and returns before the callback (`anchors`). The
texture's callback `0x770670` calls `0x768cf0`, adjusts the rectangle by the texture coordinates
when `+0x124` is set and writes the vertices with `0x7705b0`, so a failure leaves them as they were. The
vertices are `(left, top)`, `(left, bottom)`, `(right, top)`, `(right, bottom)` at `+0xd4`, `+0xe0`,
`+0xec` and `+0xf8` with z zero, and the constructor zero-fills all twelve floats (`0x76fc6e`). The
emit `0x7706e0` tests the image handle only and `0x772fd0` appends the record by pointer. The font
string's `0x773080` calls `0x768320`, which answers 0 while flag 1 is clear (`0x7730b5`), and then
`0x772e00`, which answers 1 for a rectangle with its top or right below 0 or its bottom or left above
1.0, and the record is not appended. `0x7680e0` tests flag 8 only on its resolve-now leg (`0x7680ee`); the
queue leg `0x768140` never reads it, so a model's ready hook that lays out with 0 (`0x76ce6b`) retries.

`ui::layout_recompute` runs `0x768d20` whole with `0x767a20` stubbed to fail or to succeed, and reads the flags and the stored rectangle.

## The region base

Textures and font strings share a region base. `0x77f640(parent, layer, sublevel)` runs `CScriptRegion::CScriptRegion`, zeroes the eight dwords `+0xa0` to `+0xbc` and
`+0xc0` to `+0xc8`, stores the vtables `0x81c9f4` and `0x81c9c8` (at `+0x24`) and, for a nonnull parent, calls `CScriptRegion::SetRegionParent` with the parent, layer
and sublevel. `0x77f6f0` is its destructor body: it stores the vtables, calls `SetRegionParent(0, 1, 0)`, frees the arrays at `+0xb8` and `+0xa8` and ends in
`CScriptRegion::~CScriptRegion`. `0x77f570(p)` stores `p` at `this` and, for a nonnull `p`, increments the count at `p+4`.

Addresses: `0x77f640`, `0x77f646`, `0x77f682`, `0x77f688`, `0x77f6ae`, `0x77f6f0`, `0x77f6f9`, `0x77f706`, `0x77f71e`, `0x77f737`, `0x77f740`, `0x77f570`, `0x77fd10`, `0x76c4f0`, `0x76c560`, `0x81c9c8`, `0x84f09c`, `0x854040`
Confidence: verified
Proof: one
Test: none

`0x77f6f0` pushes `0, 1, 0` before `0x77fd10` (`0x77f706`); the two frees pass the tags `0x84f09c` (line −2) and `0x854040`. `0x77f640` is called by the texture
constructor (`0x76fc57`) and the font string constructor (`0x770d44`), `0x77f6f0` by the font string destructor body (`0x770efc`) and `0x76fd76`. `0x77f570` has two callers, in the backdrop constructor (`0x77e63e`)
and the font object constructor (`0x783a5c`).

## Pixel alignment

`PixelAlignX(x)` and `PixelAlignY(y)` round a layout length up to a whole number of device pixels. A length below 2^-22
in size, and any length on a viewport of no width or height, comes back unchanged. Otherwise the length is
divided by the horizontal or vertical screen scale, multiplied by the viewport's size in pixels, increased by 0.99999 when it is positive,
truncated toward zero, divided by the pixel size again and multiplied by the screen scale. A positive length therefore gains a pixel
unless it already is a whole number of pixels, and a negative length is truncated.

Addresses: `0x7666a0`, `0x7666b3`, `0x7666e0`, `0x7666f6`, `0x76670a`, `0x76671f`, `0x766725`, `0x766737`, `0x766750`, `0x7667ba`, `0x7667cf`, `0x7667e7`, `0x41ae40`, `0x41ae50`, `0x41ae60`, `0x41ae70`, `0x58a250`, `0x808120`
Confidence: verified
Proof: one
Test: `ui::glue_765_7666a0_ratio_round_x`, `ui::glue_765_766750_ratio_round_y`

`0x7666b3` compares the length with the float 2.38e-07 (`0x8029d4`). `0x7666e0` reads the viewport as four floats from the device (`0x58a250`) and takes right less left
as the horizontal pixel count (`0x7666e8`); `0x766750` takes top less bottom (`0x766798`). `0x76670a` divides by the screen scale with `0x41ae40` (`0x7667ba` with `0x41ae50` for the vertical), `0x76671f`
adds the float at `0x808120`, 0.99999, when the product is above zero, and `0x766725` truncates. `0x766737` multiplies by the scale again with `0x41ae60` (`0x7667e7` with `0x41ae70`).
`ui::glue_765_7666a0_ratio_round_x` and `ui::glue_765_766750_ratio_round_y` run the arithmetic of `0x7666a0` and `0x766750` (two `fabs`, a `fsub`, `fmul`, `fadd` and `fdiv`) with the divisors 0.8 and 0.6.
