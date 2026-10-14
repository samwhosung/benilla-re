# loadingscreen: screen

The layer and what it shows besides a route: raising it, the input veto while it is up, the three
load fractions and the bar they fill, the paint pass, the plain screen with the map's art, and the
tip text. The raise creates the layer with the paint callback `0x406a60`; the loaders report
progress through `0x407f70`, `0x407fa0` and `0x407fd0`; the dismiss ends it.

Roots: `0x406630`, `0x406640`, `0x407e70`, `0x407e80`, `0x4083b0`, `0x4083c0`, `0x407f70`, `0x407fa0`, `0x407fd0`, `0x406a50`, `0x406a60`

## Raising the screen

`0x406800` clears bit 0 of the layer's flags (`0x4083c0` with `cl = 0`), zeroes the three progress
inputs, and, when no screen exists, loads the two bar textures into `0x882da8` and creates the
layer: rectangle (0, 0, 1, 1), flags 6, priority 9.0, parameter 0, paint callback `0x406a60`, the
handle stored at `0x882be0`. It then installs the input veto. Every call ends by repainting at
once (`0x406900`), so a raise while a screen is up resets the inputs and repaints. An arrival
aboard a transport sets bit 0 instead; the layer walk skips a call to `0x65bd90` for a layer that
has it.

Addresses: `0x406800`, `0x4083c0`, `0x5debb6`, `0x442800`, `0x442350`, `0x44242e`, `0x442437`, `0x406880`, `0x406891`, `0x882be0`, `0x882da8`, `0x7ffd34`
Confidence: verified
Proof: one
Test: none

`0x406808` calls `0x4083c0`, which loads `[0x882be0]` and, with a handle, reads the flags dword
(`0x4429e0`, `+0x2c`) and writes it back with bit 0 set for `cl` nonzero (`0x4083dc`) or cleared
(`0x4083ed`) through `0x4429f0`. `0x406814`, `0x40681e` and `0x406828` store 0 in `0x882894`,
`0x882bd0` and `0x882df4` before the `jne` at `0x406832`, so both legs reach `0x4068ef`. The
texture loop (`0x406868` to `0x4068a4`) loads a bar texture through `0x449d90` only for a null
handle: `0x406880` packs the flags with filter mode 1 and anisotropy 1, and `0x406891` creates each texture with the force flag set, so the global filter setting replaces the mode (`gx/textures#texture-flags-word`). `0x442800` takes the rectangle and `edx = 6` (`0x4068b7`, `0x4068bc`) and stores flags,
priority, parameter and callback at `+0x2c`, `+0x28`, `+0x30` and `+0x34`. `0x442350` skips its
`0x65bd90(…, 0, 4)` call when `[layer+0x2c]` has bit 0 (`0x442425`, `0x44242c`). `0x5debb6` is the
only caller with `cl = 1`.

## Input veto

While the screen is up, `0x406a50` (`xor eax,eax; ret`) is registered at priority 8.0 on the event
categories 1 (character), 8 (key down), 0xa (key repeat), 0xb (mouse button down) and 0xc
(absolute mouse move). The dispatcher stops at the first handler that returns 0, and the top
frame's handlers sit at 1.0, so none of those five events reaches them. Key up, button up, the
wheel, relative mouse moves and the input-method categories pass. `0x407e80` removes the five
registrations.

Addresses: `0x4069e0`, `0x406a50`, `0x41fca0`, `0x41fd80`, `0x4245b0`, `0x4246ad`, `0x4246b0`, `0x4246b2`, `0x765bf0`, `0x407ecd`, `0x407ee2`, `0x407ef1`, `0x407f00`, `0x407f0f`
Confidence: verified
Proof: one
Test: none

`0x4069e0` calls `0x41fca0` five times with `ecx` the category, `edx = 0x406a50`, argument 0 and
priority `0x41000000`. The dispatcher calls `[ebx+8]` (`0x4246ad`) and loops while the result is
not zero (`0x4246b0`, `0x4246b2`). `0x765bf0` registers the top frame's twelve input handlers, including
those five categories, with the float 1.0. The categories are those of
`core/input#kinds-to-categories` and `core/events#held-keys-and-repeats`. `0x407ecd` to `0x407f0f`
call `0x41fd80` with the same categories and handler.

## Progress fraction

The bar's fill is `0x882be4`. With the interface progress `P` (`0x882894`) zero it is
`0.7·A + 0.3·B`; otherwise `0.5·P + 0.35·A + 0.15·B`, where `A` is the asynchronous load fraction
(`0x882bd0`) and `B` the map load fraction (`0x882df4`). A sum below 0 gives 0, and 1 or above, or a
NaN, gives 1. Each input has a setter that stores a new value only when it is greater than the
stored one (or a NaN) and then repaints; the loaders call them as progress callbacks.

Addresses: `0x406920`, `0x406900`, `0x407f70`, `0x407fa0`, `0x407fd0`, `0x882be4`, `0x882894`, `0x882bd0`, `0x882df4`, `0x443620`, `0x66fc70`, `0x6ef160`, `0x443d30`
Confidence: verified
Proof: one
Test: `loadingscreen::progress_fraction_difftest`, `loadingscreen::progress_fraction_signed_zero_and_nan_difftest`

`0x406926` zeroes the fraction; `fcomp` of `P` with 0 and `test ah,0x44; jnp` (`0x406947`) select
the unphased weights when `P` equals 0. The sum is `(A·wA + base) + B·wB` in 53-bit precision, stored
as a float. `0x406993` to `0x40699e` give 0 when 0 is greater than the sum, and `0x4069a6` to
`0x4069b1` give 1.0 for a NaN or a sum at or above 1. The setters compare with `fcomp` and `test
ah,0x41; jnp` (`0x407f7e`, `0x407fae`, `0x407fde`): equal or less skips the store. The wait for
asynchronous loads `0x443d30` reports `A` through `0x443620`. `CMap::Load` reports `B` (0.1, then 0.25) and
`CMap::Update` (0.5, then 1.0) through `0x66fc70`, and `LoadTocFile` reports `P` through
`0x6ef160`. The test runs `0x406920` whole.

## Paint pass

The layer's callback `0x406a60` ignores its arguments. It saves the device viewport, sets the
unit box, clears with mask 1, and sets the viewport to the rectangle fitted to the scaled aspect
`a` (`[0x832a4c]`): for `a > 1` width `1/a`, left `(1 − w)·0.5`, full height; for `a < 1` height
`a`, bottom `(1 − a)·0.5`, full width; otherwise, and for a NaN, the unit box. It loads an identity
view and a projection from (0, 1, 0, 1, 0, 500), sets render states 0xe and 0xf to 0, draws
`0x406cd0`, then the bar when that returns nonzero, and restores the viewport. `0x406900` calls
it directly, then device slot +0x68 with 3.

Addresses: `0x406a60`, `0x406900`, `0x406cd0`, `0x41ad70`, `0x832a4c`, `0x58b060`, `0x58af60`, `0x58a970`, `0x5c3d90`, `0x58b050`, `0x58b040`, `0x589f40`, `0x589e60`, `0x589f50`, `0x58a960`, `0x406ac4`, `0x406b3d`
Confidence: verified
Proof: one
Test: `loadingscreen::aspect_fit_ortho_difftest`

The letterbox arithmetic is the slice `0x406ac4` to `0x406b3d`: `fcom` with 1.0 and `test ah,0x41;
jne` send `a ≤ 1` and a NaN to `0x406af7`, where `test ah,5; jp` keeps the defaults for 1 and a NaN.
The values are stored as floats. The viewport call `0x58af60` takes (xmin, xmax, ymin, ymax, 0, 1),
with `w + xlo` and `h + ylo` as the maxima. `0x58b060` and `0x58af60` at `0x406a7f` and `0x406cb8`
save and restore the viewport. The test runs the slice.

## Plain screen

`0x406cd0` draws the taxi screen `0x406f20` first and returns 0 when it drew; else it draws the plain
screen `0x406cf0` and returns 1 whether or not that drew. The plain screen draws one quad, (0, 0) to
(1, 1) in the unit box, textured with the map's loading art, then the tip batch. With the map id
`0x82f00c` equal to −1 it draws nothing. The art is resolved only while the handle `0x882e04` is
null: map id → Map row field 38 → LoadingScreens row field 2, a path; the texture of that file
when it exists, else `Interface\Glues\loading`. A failed resolve sets the map id to −1.

Addresses: `0x406cd0`, `0x406cf0`, `0x406e20`, `0x406f10`, `0x82f00c`, `0x882e04`, `0x882db0`, `0x82f010`, `0xc0daa8`, `0xc0daac`, `0xc0db04`, `0x6548b0`, `0x449d90`, `0x58a2a0`, `0x58a2e0`, `0x882dc0`, `0x882b8c`
Confidence: verified
Proof: one
Test: none

`0x406cf0` compares the map id with −1 (`0x406cf0`), tests the handle (`0x406cfc`), calls `0x406e20` for a
null one and clears the id when it returns 0 (`0x406d0e`). `0x406e20` bounds the id by `[0xc0daac]`
and reads the row's `+0x98` (`0x406e69`), bounds that by the store's id bound (`[0xc0db10]`) and reads `+8` (`0x406e88`); a null
row, a failed bound or `0x6548b0` returning 0 skips to `0x406ec6`, where a null handle loads the default
(`0x406ef1`). The quad's four corners (a strip) are stored on the first draw (`0x406d19` to
`0x406dae`) in `0x882dc0`, the coordinates are `0x882b8c`, and the indices `0, 1, 2, 3` are the words
at `0x82f010`. The draw is `0x58a2a0` (stride 0xc, coordinate stride 8) and `0x58a2e0` with kind 4 and
four indices.

## Progress bar

The bar draws the fill quad, then the border quad, each from a 24-byte entry of the table `0x7ffd34`
(path, fill flag, centre x and y, width, height): the fill `Interface\Glues\LoadingBar\Loading-BarFill`
(0.5, 0.075, 0.525, 0.025) and the border `Loading-BarBorder` (0.5, 0.075, 0.6, 0.05). A quad spans
`centre ∓ size·0.5`, and the fill's right edge is `fraction·width + left`; the texture coordinates
do not change. An entry with a null handle is skipped, and render state 7 is set to 2.

Addresses: `0x407150`, `0x4072b0`, `0x7ffd34`, `0x882da8`, `0x882be4`, `0x882b48`, `0x882bd4`, `0x4071d1`, `0x407229`, `0x407217`, `0x589e80`
Confidence: verified
Proof: one
Test: `loadingscreen::progress_bar_quad_difftest`

`0x407150` walks `0x7ffd34` in steps of 0x18 to its end, paired with the handles `0x882da8`. The
half sizes `w·0.5` and `h·0.5` stay in 53-bit precision across each pair of corners; `0x407217`
replaces the right edge for the entry whose flag is nonzero. The corners are stored in strip order
(left bottom, right bottom, left top, right top) with the coordinates `0x882b8c`. The test runs the
slice `0x4071d1` to `0x407229`.

## Tip text

`0x406630` stores the tip text pointer in `0x882e10`. When it is set, `0x406640` lays the text out
once into a one-string batch that the plain screen draws every frame. The font is `Fonts\FRIZQT__.TTF`,
except `Fonts\2002.TTF` for koKR and `Fonts\FZLBJW.TTF` for zhCN and zhTW, at 0.018 of the scaled
height. With the scaled aspect `a`, the wrap width is `s = 515 / (a·1024)` rounded through the
horizontal scale and back, and the position `(0.5 − s·0.5, 0.025 + 0.075 + yoff, 0)` with
`yoff = (1 − a)·0.5` for `a < 1`, else 0. The text is a copy with trailing CR and LF cut; the colour
is `0xd7c8c8c8`, with a black shadow offset (0.001, −0.001).

Addresses: `0x406630`, `0x406640`, `0x882e10`, `0x882e08`, `0x882e0c`, `0x882e14`, `0x82f018`, `0x832a4c`, `0xc0e080`, `0x44d040`, `0x44d340`, `0x5c1c30`, `0x5c27a0`, `0x5c1d60`, `0x5c1e60`, `0x5c1ef0`, `0x41ae70`, `0x41ae60`, `0x41ae40`
Confidence: verified
Proof: one
Test: none

Only the glue's world entry calls `0x406630`. `0x406646` and `0x406653` skip the block `0x406659` to `0x4067e3` for a null tip. `0x406669` indexes `0x82f018`
with the locale index `[0xc0e080]` (enUS, koKR, frFR, deDE, zhCN, zhTW, esES, xxYY): `FRIZQT__` for
all but koKR (`2002`) and zhCN and zhTW (`FZLBJW`). `0x41ae70` multiplies by the vertical scale. `0x44d040` creates the font
and `0x44d340` takes its pointer. `0x4066b9` and `0x4066be` read `a` from `0x41ad70`, the aspect times 0.75, and
`s` goes through `0x41ae60` and `0x41ae40` with a float store between. The trim is
the loop `0x406758` to `0x406766`, which reads below the copy for an empty or all-CR/LF string.
`0x5c1c30` makes the string (height 0.018, position, `s`, 1.0, 0.005, justification 2 and 0), `0x5c27a0`
shadows it, `0x5c1d60` and `0x5c1e60` batch it.

## Ending the screen

`0x407e80` closes the layer handle, the two bar textures and the art, removes the five veto
registrations, clears the route, and frees the tip: it zeroes `0x882e10`, then destroys the batch,
the string and the font. The map id and the progress stay. Callers end the screen on client
shutdown (`0x402057`, and `0x40370b` through the thunk `0x4083b0`), an aborted transfer (`0x401a11`),
a cinematic's black frame (`0x48edd0`), the active player's arrival (`0x5debbd`; skipped for a pending
cinematic and for an arrival aboard a transport object), a transport leg change (`0x5f54de`), and the
unit tick (`0x60a920`) when the player's transport lookup gives no object or its model reports
ready. `0x407e70` reports whether the handle exists.

Addresses: `0x407e80`, `0x4083b0`, `0x407e70`, `0x41aed0`, `0x5c1f70`, `0x5c1d00`, `0x407f2b`, `0x407ed7`, `0x401a11`, `0x402057`, `0x40370b`, `0x48edd0`, `0x5debbd`, `0x5f54de`, `0x60a920`
Confidence: verified
Proof: one
Test: none

The order is `0x407e8d` (layer), `0x407ea0` to `0x407eb5` (bars), `0x407ebd` (art), `0x407ecd` to `0x407f14`
(veto), `0x407f1e` (`0x407410`) and `0x407f23` to `0x407f65` (tip). `0x407f2b` zeroes the tip text and
`0x407ed7` the art handle. Six calls and the thunk's `jmp` reach `0x407e80`, and one call reaches
the thunk; no stored address names either (the dword at `0x41100b` is the bytes of a `cmp`). `0x5debbd` follows a test of the pending-cinematic cell and of the player's
transport lookup (`0x5deb78` to `0x5debb2`); `0x60a920` follows `0x60a8dc` to `0x60a91c`.

