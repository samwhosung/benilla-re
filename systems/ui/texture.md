# ui: texture colour and drawing

How a texture region holds its image and its colour, what a `<Color>`, a vertex colour and the frame's alpha each
do, the record it leaves in a layer batch, how the batch is submitted, and what desaturating changes. The
batches and their order are `draworder`; the texture's size and vertices are `regions`; the backdrop pieces are
textures of this kind (`backdrop`).

A texture keeps its image as a handle at `+0xcc`, its blend mode at `+0xd0`, its four vertices from `+0xd4`, its eight
texture coordinates from `+0x104` and its pixel-shader slot at `+0x128`. Its vertex colour is two parallel arrays:
alpha bytes (count `+0xa4`, data `+0xa8`) and colour dwords (count `+0xb4`, data `+0xb8`). A colour dword is stored
with the bytes blue, green, red, alpha.

Roots: `0x76fe20`, `0x770200`, `0x770360`, `0x77f750`, `0x77fac0`, `0x772fd0`, `0x76fb00`, `0x79c1e0`, `0x79c2c0`, `0x79bb40`

## Two colour stores

A texture has two colour inputs that never overwrite each other: the image at `+0xcc`, and a vertex colour that
multiplies it. `<Color>` and `SetTexture(r, g, b, a)` replace the image with a solid one; `SetVertexColor` and
`SetAlpha` write the vertex colour. `GetVertexColor` answers opaque white unless the vertex colour holds exactly one entry, so a
vertex colour of two or more entries, a gradient, reads back white as well as an empty one.

Addresses: `0x77f8c0`, `0x770300`, `0x77f8c8`, `0x77f900`, `0x77f903`, `0x77f750`, `0x7706e0`, `0x7706e3`, `0x7706eb`, `0x79aa50`
Confidence: verified
Proof: one
Test: none

`0x77f8c0` copies the four bytes from the arrays when the colour count is 1 (`0x77f8c3`, `0x77f8c8`) and stores `0xffffffff` for any other count
(`0x77f900`, `0x77f903`): none, or the four of a gradient. `0x770300` replaces the image handle with a referenced one, closing the old and marking the texture dirty. The emit `0x7706e0` tests the image handle alone (`0x7706e3`, `0x7706eb`), so a texture with neither a
file nor a colour draws nothing, and a vertex colour alone never draws.

## A colour without a file

A `<Texture>` with a `<Color>` child and no `file` shows a solid fill of that colour. The fill is a shared 8 by 8 image named
`Solid Texture` whose every texel holds the colour, alpha included, and it counts as having alpha when the colour's
alpha byte is below `0xfe`. With both a `file` and a `<Color>`, the colour is discarded: the loader sets the solid image
first and the file's image replaces it. A file that fails to load logs `Texture %s: Unable to load texture file %s` and
falls back to an opaque green fill, not to the `<Color>`. A `<Gradient>` child sets the vertex colours of the
four corners.

Addresses: `0x76fe20`, `0x76fe3b`, `0x770088`, `0x77009d`, `0x7700a9`, `0x770102`, `0x77010a`, `0x77012d`, `0x770156`, `0x77016a`, `0x770171`, `0x770360`, `0x770366`, `0x77036c`, `0x770384`, `0x44a9c0`, `0x44a900`, `0x44a946`, `0x5c5350`, `0x5c53b1`, `0x5c53b9`, `0x7700de`, `0x7700f2`, `0x77f910`
Confidence: verified
Proof: one
Test: none

`0x76fe20` walks the children for `TexCoords`, `Color` and `Gradient` (`0x76fee3`, `0x770088`, `0x7700c3`), then reads `file` after the loop
(`0x77010a`). The `<Color>` arm runs `0x6f23d0` and `0x770360` (`0x77009d`, `0x7700a9`) whether or not there is a file.
`0x770360` takes the image from `0x44a9c0`, which finds or builds the 8 by 8 solid image (`0x44a900`, size 8, name pushed at
`0x44a9a5`), releases the old handle and stores the new one at `+0xcc` (`0x770384`). The generator `0x5c5350` fills 64
dwords with the colour (`0x5c53b1` to `0x5c53b9`), and `0x44a946` compares the alpha byte with `0xfe`. A failed `0x770200`
(`0x77012d`) logs (`0x770156`) and calls `0x770360` with `0xff00ff00` (`0x77016a`, `0x770171`). `<Gradient>` calls `0x77f910`
(`0x7700f2`).

## A gradient element

A `<Gradient>` child reads `orientation` (`HORIZONTAL` or `VERTICAL`, ignoring case) and the children `MinColor` and `MaxColor`, each parsed like a `<Color>`. An absent or
empty orientation is `HORIZONTAL`, and an unknown one logs `Unknown orientation %s in element %s` and is also `HORIZONTAL`. A colour whose child is missing stays
0, which is transparent black. The element applies the same corner colours as `SetGradient` (`Gradients`).

Addresses: `0x76fe20`, `0x7700b8`, `0x7700c3`, `0x7700d8`, `0x7700de`, `0x7700f2`, `0x6f2630`, `0x6f2638`, `0x6f2645`, `0x6f2659`, `0x6f266c`, `0x6f269e`, `0x6f26ad`, `0x6f26c0`, `0x6f26ce`, `0x6f1a20`, `0x6f23d0`, `0x77f910`
Confidence: verified
Proof: one
Test: none

`0x7700c3` matches the child name `Gradient`; `0x7700d8` zeroes the two colour words and `0x7700de` calls `0x6f2630`, which zeroes the orientation (`0x6f263f`), reads the attribute (`0x6f2645`), maps it with
`0x6f1a20` (`0x6f2659`), logs the unknown name (`0x6f266c`) and walks the children comparing `MinColor` (`0x6f269e`) and `MaxColor` (`0x6f26c0`) with `0x6f23d0` (`0x6f26ad`, `0x6f26ce`). The loader then calls `0x77f910` (`0x7700f2`).

## Texture coordinates in XML

A `<TexCoords>` child gives `left`, `right`, `top` and `bottom` (defaults 0, 1, 0, 1) and sets the eight coordinates
as top left, bottom left, top right, bottom right. When any of the four is below 0 or above 1, the file loads with
the wrap flag on, so the image repeats; otherwise it clamps.

Addresses: `0x76fee3`, `0x76feeb`, `0x76ff1c`, `0x76ffe8`, `0x76fff0`, `0x770008`, `0x77006b`, `0x770071`, `0x770127`, `0x77012d`, `0x7704c0`, `0x770200`, `0x878cf0`
Confidence: verified
Proof: one
Test: none

The defaults are stored before the attributes are read (`0x76fef5`, `0x76feff`). `0x7704c0` receives the eight values (`0x76ffe8`). Each
value is compared with 0.0 and 1.0 (`0x76fff0` to `0x77006b`); a failing comparison sets the flag at `[ebp-0x1c]` (`0x770071`),
which `0x770200` takes as its second argument (`0x770127`, `0x77012d`) with the filter value at `0x878cf0` and shader index 0.

## SetTexture

`SetTexture(file)` loads the image with the wrap flag off, the filter value at `0x878cf0` and shader index 0, and answers the number 1,
or nil when the load fails. `SetTexture(r, g, b[, a])` with a number sets the solid fill instead. `SetTexture(nil)`, a value that is not a string or
a number, and the empty string `""` all clear the image, keep the shader slot and answer 1. Setting the file the texture already shows is a no-op that keeps everything else. A different
file also replaces the pixel-shader slot with the shader of index 0, which is none, so it ends desaturation.

Addresses: `0x79bb40`, `0x79bbd5`, `0x79bd63`, `0x79bd82`, `0x79bd89`, `0x79bd93`, `0x79bda5`, `0x79bdc8`, `0x79bdd5`, `0x79bddc`, `0x770200`, `0x77020e`, `0x77021e`, `0x770225`, `0x77022b`, `0x770281`, `0x7702b8`, `0x7702c0`, `0x7702cb`, `0x7702dc`, `0x7702e7`, `0x44ac70`, `0x449d90`, `0x770620`
Confidence: verified
Proof: one
Test: none

`0x79bb40` tests argument 2 with `0x6f34d0` (`0x79bbd5`): a number goes to `0x770360` (`0x79bd63`); a string calls `0x770200` with name, wrap 0,
filter and shader 0 (`0x79bd89` to `0x79bda5`); anything else calls it with a null name (`0x79bdc8`, `0x79bdd5`). A result of 0
returns nil (`0x79bddc`). The filter at `0x878cf0` is a `.data` default of 1: 39 absolute reads find no absolute writer, but writes through a register were not censused. `0x770200(name, wrap, filter, shader)` asks `0x44ac70` whether the held image has this file
name and returns 1 if so (`0x77021e` to `0x770225`, `0x7702e7`), loads with `0x449d90` (`0x770281`), and stores the shader from `0x770620` at
`+0x128` (`0x7702b8`, `0x7702c0`) before the handle (`0x7702dc`). A null or empty name skips the load and the shader store: it releases the old handle, stores a null one and answers 1 (`0x770218`, `0x77022f`, `0x770235` to `0x7702cb`).

## SetTexture failure

When `0x770200` loads a non-empty name that no candidate resolves, the texture keeps what it held: the loaded placeholder is released, the held handle and the shader slot are not written, and the call answers 0, so the Lua method answers nil. The status the load fills is a stack object of vtable `CStatus` with its highest severity at `+0x10` set to 0; a load failure puts 3 there, and any value of 2 or more takes the failure path. The solid green texture `TextureCreate` returns on failure never reaches the widget.

Addresses: `0x770200`, `0x770266`, `0x770269`, `0x770281`, `0x770288`, `0x77028c`, `0x770290`, `0x77029d`, `0x7702a5`, `0x7702ac`, `0x7702b5`, `0x7702dc`, `0x41aed0`, `0x44cb40`, `0x419e30`, `0x7ffa10`, `0x79bb40`, `0x79bda5`, `0x79bdac`, `0x79bdae`, `0x79bddc`, `0x6f37f0`, `0x6f3810`, `0x449d90`, `0x41a0f6`
Confidence: verified
Proof: one
Test: none

`0x770266` and `0x770269` store 0 at `+0x10` of the status and the `CStatus` vtable. `0x770281` calls `TextureCreate` and `0x770288` compares the severity with 2: at 2 or more `0x770290` closes the returned handle (`HandleClose`), `0x77029d` passes the status to `0x44cb40` with 4, `0x7702a5` destroys it, and `0x7702ac` returns 0 before the stores at `0x7702b5` and `0x7702dc`. Below 2 the shader slot and the new handle are stored. `0x79bda5` calls `0x770200`; `0x79bdac` branches on 0 to `0x79bddc`, which pushes nil (`0x6f37f0`), else `0x79bdae` pushes the number 1 (`0x6f3810`); both paths return one value. A candidate that loads after a failed first one leaves the status at 0, since the loaders write nothing on a failed open (`gx/textures#texture-name-candidates`).

## Setting a vertex colour

`SetVertexColor(r, g, b[, a])` clamps each value to 0 to 1, takes a missing alpha as the current alpha, packs it as
`×255 + 0.5` truncated, and stores it as the only entry of both arrays. With no vertex colour yet, white of alpha `0xfe` or `0xff` stores nothing; repeating the one entry does nothing.
`SetAlpha(a)` replaces the alpha of the colour `GetVertexColor` answers with `a × 255` truncated, with no clamp. That equals
`SetVertexColor` with the current colour only for exactly one entry. With a gradient the read answers white and the
store leaves one entry: `SetAlpha` gives flat white, and `SetVertexColor` without an alpha the given colour at alpha
`0xff` (`0x79ae0a` copies only the read's alpha byte).

Addresses: `0x79abd0`, `0x79ac81`, `0x79ad43`, `0x79ae0a`, `0x79ae17`, `0x77f750`, `0x77f766`, `0x77f77c`, `0x77f782`, `0x77f7b3`, `0x77f7bc`, `0x77f88a`, `0x77f8a7`, `0x79b580`, `0x79b627`, `0x79b638`, `0x79b649`, `0x79b64c`, `0x40a2b0`, `0x806498`, `0x7ffe58`, `0x7ffa24`
Confidence: verified
Proof: one
Test: none

`0x79abd0` reads the current colour with `0x77f8c0` when alpha is absent (`0x79ac81`, `0x79ad43`), scales by 255.0 and adds 0.5
(`0x7ffe58`, `0x7ffa24`), truncates with `0x40a2b0`, packs alpha in bits 24 to 31, and calls `0x77f750` (`0x79ae17`). `0x77f750(colour)`
returns when the count is 0 and the colour is white with alpha `0xfe` or more (`cmp BYTE PTR [edi+3],0xfe; jb`, `0x77f766`; red, green and blue all `0xff`, `0x77f76c` to `0x77f77c`), or when one entry already equals it
(`0x77f782` to `0x77f7b3`); otherwise it resizes both arrays to 1 (`0x77f7bc` onward, a gradient's four entries shrink to one), stores the alpha byte and the dword
(`0x77f88a` to `0x77f89d`) and calls the region's slot 8 (`0x77f8a7`), which is `0x77fac0`. `0x79b580` reads the colour with `0x77f8c0` (`0x79b627`), multiplies the alpha by 255.0
(`0x806498`), truncates (`0x79b638`), replaces byte 3 of the read-back colour (`0x79b649`) and calls `0x77f750` with it (`0x79b64c`).

## The frame's alpha in the colour

A texture draws with its frame's alpha folded into its vertex colour. The authored alpha stays in the alpha array
and the colour array holds `authored × frame alpha / 255`. A frame alpha of `0xfe` or more with no colour entry that
is not opaque white drops the arrays, and a frame alpha below `0xfe` gives a texture with no colour a white entry so
the fold has something to scale. A change of the frame's alpha refreshes every region of the frame.

Addresses: `0x77fac0`, `0x77fad6`, `0x77fae3`, `0x77fb29`, `0x77fb2d`, `0x77fb40`, `0x77fb6a`, `0x77fbcc`, `0x77fc01`, `0x77fc07`, `0x77fc1d`, `0x77fc38`, `0x77fc49`, `0x76a690`, `0x76a6a2`, `0x76a6d0`, `0x81c738`
Confidence: verified
Proof: one
Test: none

`0x77fac0` reads the frame's alpha byte at `+0xc8` (`0x77fad6`). For `0xfe` or more it walks the entries from the last and
returns to the refresh at the first that is not opaque white (`0x77faf5` to `0x77fb23`), and after the last it zeroes both counts
(`0x77fb2d`). Below `0xfe` with no entry it builds one white entry (`0x77fb6a` to `0x77fbf0`). It then writes each colour
byte 3 as the alpha byte times the frame alpha divided by 255, with the multiply-by-reciprocal `0x80808081`
(`0x77fc07` to `0x77fc38`). The slot is entry 8 of the texture vtable `0x81c718` at `0x81c738`, and `0x76a690`, the frame's alpha
setter, calls it with 0 on each region (`0x76a6d0`).

## The quad record

A texture adds one 0x1c-byte record to its layer batch: the image handle, a pointer to the four vertices, a
pointer to the eight coordinates, the blend mode, a pointer to the colour array or none, a colour stride and the
pixel-shader slot. The colour pointer is none when the count is 0, and the stride is 4 when there is more than one
entry and 0 otherwise, so one colour covers all four vertices and a gradient gives one per vertex.

Addresses: `0x772fd0`, `0x772fdc`, `0x77300d`, `0x773018`, `0x773026`, `0x773029`, `0x773032`, `0x773035`, `0x77304a`, `0x77305f`, `0x773068`, `0x770700`, `0x44acf0`, `0x7703f0`
Confidence: verified
Proof: one
Test: none

`0x772fd0` resolves the image to a device texture with `0x770700` (`0x772fdc`), grows the batch array by one record (`0x77300d` scales
the index by 0x1c) and fills the record: handle `+0x00` (`0x773018`), `&this+0xd4` at `+0x04` (`0x773026`), `&this+0x104` at `+0x08`
(`0x773029`), `+0xd0` at `+0x0c` (`0x773032`), the colour pointer or null at `+0x10` (`0x773035` to `0x77304a`), the stride
`4 & -(1 < count)` at `+0x14` (`0x77304d` to `0x77305f`) and `+0x128` at `+0x18` (`0x773068`). Nothing is added when `0x770700` returns 0.

## Submitting a quad

The submit draws each record in order. It binds the image only when the handle changes, sets the blend state from
the record, binds the record's pixel shader, and draws four vertices with the colour pointer and stride. A record with
a vertex colour and a blend mode below 2 is drawn with blend mode 2, ordinary alpha blending, so any vertex colour
turns blending on. The submit never writes the combine preset of the first stage, so it draws with the device's, which is
1 (see the combine preset): texel and vertex colour multiply, colour and alpha alike. The result per channel is `texel × vertex colour`, alpha
included, when no pixel shader is set.

Addresses: `0x76fb00`, `0x76fb20`, `0x76fb2c`, `0x76fb75`, `0x76fb85`, `0x76fb93`, `0x76fb98`, `0x76fb9b`, `0x76fba2`, `0x76fbaf`, `0x76fbdb`, `0x76fc03`, `0x593bf0`, `0x593d93`, `0x85c250`
Confidence: verified
Proof: one
Test: none

`0x76fb00` opens a render-state bracket (`0x76fb20`), sets states 0xe, 0xf, 0x10 and 0x12 to 0 (`0x76fb2c` to `0x76fb50`), and per record binds
the image as state 0x17 (`0x76fb75`), tests the colour pointer (`0x76fb85`), compares the blend mode with 2 (`0x76fb93`), sets state 7 to
2 for a colour with a mode below 2 and to the record's mode otherwise unless it is 0xb (`0x76fb98`, `0x76fb9b`, `0x76fba2`), binds state 0x3f
(`0x76fbaf`), and draws (`0x76fbdb`). It sets no state from 0x1f to 0x26. The census of those states is under the combine preset. The device reset `0x593bf0` stores 1 in all eight (`0x593d93`);
preset 1 modulates colour and alpha (the preset table `0x85c250`, `watershade`). The bracket ends at `0x76fc03`.

## The combine preset

The submit draws with whatever combine preset the device holds for the first stage. The device reset stores 1, and the census of writes of the
eight states `0x1f` to `0x26` finds 17, all of state `0x1f` or `0x20`. Eleven store 1; the six that store another value, 0, 3 or 5, each sit inside
their own render state push and pop, so the UI always draws with 1.

Addresses: `0x589f40`, `0x589f50`, `0x589e60`, `0x593950`, `0x593a50`, `0x5c33ba`, `0x6b63a9`, `0x6d7fa8`, `0x4eae56`, `0x4eae83`
Confidence: verified
Proof: one
Test: none

`0x589f40` and `0x589f50` push and pop the render state stack (`GxRsPush`, `GxRsPop`, `0x593950`, `0x593a50`), so a state changed inside a bracket is restored at
its pop. The census is `mov ecx, 0x1f` to `0x26` within five instructions of `call 0x589e60`, the integer state setter; the control is the minimap's
`0x4eae56` and `0x4eae83`. No write has a state id from `0x21` to `0x26`. Eleven store 1, the reset value: `0x4eae56`, `0x4eae83`, `0x595806`, `0x6b2b08`, `0x6b2b8f`,
`0x6b2bfc`, `0x6bf02b`, `0x6bf4bc`, `0x6bf5e2`, `0x7e5a75` and `0x7e5dca`. The other six: `0x5c33ba` stores 0 (the font frame, push `0x5c3372`, pop `0x5c3402`),
`0x6850b9`, `0x685463`, `0x6b63a9` and `0x6d74fc` store 3 (ocean, river, group liquid, decal) and `0x6d7fa8` stores 5 (blob shadow). Writes whose state id is not a
literal in `ecx`, such as a loop over stages, were not censused.

## Draw layer and blend mode

`SetDrawLayer(layer)` moves the texture to one of the five draw layers and `GetDrawLayer()` answers the layer's name. The names
are `BACKGROUND`, `BORDER`, `ARTWORK`, `OVERLAY` and `HIGHLIGHT`, matched ignoring case. A texture starts on the layer its
creator gives it with the blend mode `BLEND`. `SetBlendMode(mode)` takes `DISABLE`, `ALPHAKEY`, `BLEND`, `ADD` or `MOD`, which are
modes 0 to 4 in the order 0 `DISABLE`, 1 `ALPHAKEY`, 2 `BLEND`, 3 `ADD`, 4 `MOD`, and `GetBlendMode()` answers the name, `UNKNOWN`
for any other value. A name that is not a string or not in the list raises `Usage: %s:SetDrawLayer("layer")` or
`Usage: %s:SetBlendMode("mode")` and changes nothing.

Addresses: `0x79a6c0`, `0x79a75f`, `0x79a765`, `0x79a780`, `0x79a810`, `0x79a834`, `0x79a851`, `0x79a890`, `0x79a935`, `0x79a950`, `0x79a9e0`, `0x79aa04`, `0x79aa13`, `0x6f18b0`, `0x6f1900`, `0x6f1920`, `0x6f1970`, `0x77fd10`, `0x7703f0`, `0x76fc40`, `0x76fc64`
Confidence: verified
Proof: one
Test: none

The layer is `+0xc0` and the blend mode `+0xd0`. `0x79a780` reads the name with `0x6f18b0` over the current layer (`0x79a81a` to `0x79a834`) and calls
`0x77fd10(parent, layer, +0xc4)` (`0x79a851`), which relinks the texture in its parent's layer list and keeps its shown flag. `0x79a950` parses the name with
`0x6f1920` (`0x79aa04`) and stores it through `0x7703f0` (`0x79aa13`). The constructor stores mode 2 (`0x76fc64`). The mode of the quad record is this value (`0x773032`).

## Gradients

`SetGradient(orientation, minR, minG, minB, maxR, maxG, maxB)` and `SetGradientAlpha(orientation, minR, minG, minB, minA, maxR, maxG, maxB, maxA)` give the
four corners their own vertex colours. The orientation is `HORIZONTAL` or `VERTICAL`, ignoring case; any other string raises the usage message. Each channel
is clamped to 0 to 1 and the alpha of `SetGradient` is 1. `HORIZONTAL` gives the two left corners the minimum colour and the two right corners the maximum;
`VERTICAL` gives the two bottom corners the minimum and the two top corners the maximum. The result is four entries in both colour arrays, so the quad
record's stride is 4.

Addresses: `0x79ae30`, `0x79aee4`, `0x79b119`, `0x79b180`, `0x79b233`, `0x79b53c`, `0x6f1a20`, `0x77f910`, `0x77f918`, `0x77f929`, `0x77f9b2`, `0x77f9c3`, `0x77f9c5`, `0x77fa2c`, `0x77fa21`
Confidence: verified
Proof: one
Test: none

The bindings parse the orientation with `0x6f1a20` (`0x79aee4`, `0x79b233`) over the names `HORIZONTAL` (0) and `VERTICAL` (1), pack each colour as `×255 + 0.5`
truncated, and call `0x77f910(orientation, min, max)` (`0x79b119`, `0x79b53c`). `0x77f910` makes both arrays four entries long unless the colour array already holds
four (`0x77f918`, `0x77f929`) and, for orientation 0 (`0x77f9c3`), stores the minimum in entries 0 and 1 (vertices top left and bottom left) and the maximum in 2 and 3
(`0x77f9c5` onward); for 1 (`0x77fa2c`) the minimum in entries 1 and 3 and the maximum in 0 and 2. It calls the region's alpha slot, `0x77fac0` (`0x77fa21`).

## Showing and hiding a texture

`Show()` and `Hide()` set the texture's shown flag at `+0xc4` and register it in its parent's draw layer list or take it out. `IsShown()` answers 1 or nil by the
flag, and `IsVisible()` answers 1 or nil by the visible flag at `+0xc8`, which is set only while the texture is registered, that is shown inside a
visible frame.

Addresses: `0x79b770`, `0x79b80f`, `0x79b819`, `0x79b830`, `0x79b8cf`, `0x79b8d9`, `0x79b8f0`, `0x79b975`, `0x79b9b0`, `0x79ba35`, `0x77fcb0`, `0x77fc60`, `0x77fdf0`
Confidence: verified
Proof: one
Test: none

`0x79b770` stores 1 at `+0xc4` and calls `0x77fcb0` (`0x79b80f`, `0x79b819`), which links a shown, unregistered region of a visible parent into the layer list and sets `+0xc8`.
`0x79b830` stores 0 and calls `0x77fc60`, which takes it out and clears `+0xc8` (`0x79b8cf`, `0x79b8d9`). `0x77fdf0` tells the parent frame that the texture's layer changed, but only while the texture is registered. `IsVisible` reads `+0xc8` (`0x79b975`) and `IsShown` `+0xc4` (`0x79ba35`).

## Reading the image and the coordinates

`GetTexture()` answers the name of the image without its extension, or nil when there is no image. A solid fill is named `Solid Texture`. `GetTexCoord()` answers
the eight coordinates in the order top left, bottom left, top right, bottom right, each as a pair. `SetTexCoord(minX, maxX, minY, maxY)` sets the same four corners
from a rectangle, and `SetTexCoord(ULx, ULy, LLx, LLy, URx, URy, LRx, LRy)` sets all eight; any other number of arguments raises
`Usage: %s:SetTexCoord(minX, maxX, minY, maxY) or SetTexCoord(ULx, ULy, LLx, LLy, URx, URy, LRx, LRy)`. When `SetTexCoordModifiesRect` is on, a change of coordinates
recomputes the vertices from the resolved rectangle.

Addresses: `0x79ba70`, `0x79bafe`, `0x79bb0c`, `0x7703a0`, `0x44b410`, `0x64a450`, `0x79bdf0`, `0x79be86`, `0x79bead`, `0x79beb0`, `0x79bf58`, `0x79bff9`, `0x79c065`, `0x770410`, `0x7704c0`, `0x770462`, `0x770498`, `0x79c080`, `0x79c113`, `0x79c120`, `0x44a9ae`
Confidence: verified
Proof: one
Test: none

`0x7703a0` takes the image's name (`0x44b410`, the handle's `+0x0c`) into a 0x104-byte buffer and cuts it at the last dot (`0x64a450`, `0x7703ce`), and `0x79ba70` pushes it
(`0x79bb0c`); the solid fill's name is written at `0x44a9ae`. `0x79bdf0` pushes eight numbers (`0x79be86`, `0x79bead`). `0x79beb0` reads the argument count with `lua_gettop` (`0x79bf58`): eight
numbers go straight to `0x7704c0` (`0x79bff9`) and four become `(minY, minX, maxY, maxX)` for `0x770410` (`0x79c065`), which makes the four pairs. `0x7704c0` copies eight floats to `+0x104` and,
with `+0x124` set, calls `0x768320`, `0x770570` and `0x7705b0` on the resolved rectangle (`0x770462` to `0x770498`). `SetTexCoordModifiesRect` stores its flag at `+0x124` (`0x79c113`).

## Desaturating

`Texture:SetDesaturated(flag)` binds a pixel shader to the texture and answers the number 1, or nil when the device
cannot run the shader. A missing argument is true. Turning it off clears the slot only when it holds that shader.
`IsDesaturated()` answers 1 only when the slot holds the usable shader, else nil, so a device without the shader cannot
tell "off" from "unsupported". A desaturated quad takes its colour from the shader and not from the fixed combine; the
weights are in the shader file `Shaders\Pixel\Desaturate.bls`, not in the executable (see the saturation weights).

Addresses: `0x79c1e0`, `0x79c26a`, `0x79c275`, `0x79c27e`, `0x79c28a`, `0x79c296`, `0x79c2a4`, `0x79c2b2`, `0x79c2c0`, `0x79c34a`, `0x79c354`, `0x770620`, `0x770650`, `0x76fda0`, `0x76fdb9`, `0x58b2b0`, `0x58b360`, `0x6f1c10`
Confidence: verified
Proof: one
Test: none

`0x770620(i)` returns the shader at index `i` of the two-entry table at `0xcf4cd0` when its usable flag at `+0x2c`
is set, else 0 (`0x58b360`). `0x76fda0` fills the table at startup from a name list, leaving slot 0 empty and loading
`Shaders\Pixel\Desaturate.bls` into slot 1 (`0x76fdb9`, `0x58b2b0`). `0x79c1e0` asks `0x770620(1)` (`0x79c26a`) and returns nil when it is 0
(`0x79c2b2`); reads the flag with `0x6f1c10`, default true (`0x79c27e`); stores the shader or, for false and the same shader
already held, 0 through `0x770650` (`0x79c28a` to `0x79c296`); and pushes the number 1.0 (`0x79c2a4`). `0x79c2c0` compares `+0x128` with the
usable shader (`0x79c354`). The constructor stores 0 at `+0x128`.

## The saturation weights in the exe

The executable holds Rec.709-style luminance weights, 0.2125, 0.7154 and 0.0721, but only D3DX's colour saturation code reads them. The texture submit sets no
shader constant, so a desaturated quad takes its weights from the shader file.

Addresses: `0x74d794`, `0x74d7a1`, `0x75e12b`, `0x818878`, `0x818a28`, `0x76fb00`
Confidence: verified
Proof: one
Test: none

The floats are `0x3e59999a`, `0x3f372474` and `0x3d93a92a`. The x87 routine at `0x74d794` multiplies red by the dword at `0x818880` (`0x74d7a1`), blue by `0x81887c`
(`0x74d7aa`) and green by `0x818878` (`0x74d7b5`), and the 3DNow copy reads the pairs at `0x818a28` (`0x75e12b`) and `0x818a30` (`0x75e133`); a search of the listing finds no other read. The
submit `0x76fb00` calls `0x589f40`, `0x589e60`, `0x589e80`, `0x58a2a0`, `0x58a2e0`, `0x58a340`, `0x589f50`, `0x5c1ef0` and the callbacks, and none sets a shader constant.
