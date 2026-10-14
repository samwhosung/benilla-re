# ui: the frame backdrop

A backdrop is the background and the nine-piece border a frame can carry: the object that holds its
settings, how a script or an XML element fills it, the pieces it builds and where it anchors them, the
slices of the edge file and the texture coordinates of each, and how the two colours apply. The
pieces are ordinary textures of the frame, so `texture` and `draworder` describe how they draw.

A frame holds at most one backdrop, a pointer at `+0x1ac`. Installing a new one tears down the old
object and its textures first, and installing none removes it.

Roots: `0x76a5d0`, `0x7776e0`, `0x777370`, `0x777bc0`, `0x777d30`, `0x777f60`, `0x7780d0`, `0x77e6c0`, `0x77e8d0`, `0x77f0c0`

## The object

A backdrop is 0x68 bytes: the background texture at `+0x00`, the eight edge textures from `+0x04` (left, right, top,
bottom, top left, top right, bottom left, bottom right), the two file names at `+0x24` and `+0x30`, the edge bit
mask at `+0x3c`, the tile flag at `+0x40`, the edge size at `+0x48`, the tile size at `+0x4c`, the insets top, bottom,
left and right from `+0x50`, and the two colours at `+0x60` and `+0x64`. A fresh object has no textures, empty
names, an edge size of 0.025, zero insets and white colours. The dword at `+0x44` is never used.

Addresses: `0x77e5f0`, `0x77e61d`, `0x77e657`, `0x77e65d`, `0x77e662`, `0x77e67a`, `0x76a5d0`, `0x76a5e1`, `0x76a619`, `0x76a61e`, `0x81c9b0`
Confidence: verified
Proof: one
Test: none

The constructor `0x77e5f0` zeroes the nine pointers, builds the two strings with `0x41e3a0` (`0x77e623`, `0x77e63e`),
zeroes `+0x3c` and `+0x40` (`0x77e657`, `0x77e65a`), copies the float at `0x81c9b0`, which is 0.025, to `+0x48` (`0x77e65d`,
`0x77e662`) and stores `0xffffffff` in both colours (`0x77e67a` to `0x77e680`). The edge size is a device length:
0.025 is 32 interface units only in a 4:3 window (`regions`). `0x76a5d0(backdrop)` releases the
old object's textures with `0x77f4d0` and frees it (`0x76a5e1`), builds the pieces of the new one with `0x77e8d0` (`0x76a619`)
and stores the pointer (`0x76a61e`). Its callers are the Lua method, the XML load and `0x4b0940`.

## SetBackdrop from a script

`SetBackdrop(table)` reads `bgFile`, `edgeFile`, `tile`, `tileSize`, `edgeSize` and `insets` with `left`, `right`,
`top` and `bottom` from the table and builds a new object. `SetBackdrop(nil)` removes the backdrop. A
missing or non-number size keeps the default and an inset is 0. The edge mask is `0xff` for a nonempty `edgeFile`
and 0 otherwise. Lengths are interface units, converted as `SetWidth` converts them. No argument, or one that is
neither nil nor a table, raises the usage message
`Usage: %s:SetBackdrop(nil or {bgFile = "bgFile", edgeFile = "edgeFile", tile = false, tileSize = 0, edgeSize = 32, insets = { left = 0, right = 0, top = 0, bottom = 0 }})`.

Addresses: `0x7776e0`, `0x777792`, `0x777799`, `0x7777c3`, `0x7777e2`, `0x77780d`, `0x777823`, `0x777854`, `0x777884`, `0x7778a3`, `0x7778b7`, `0x7778e4`, `0x7778ef`, `0x77790c`, `0x777948`, `0x777965`, `0x7779a1`, `0x7779c3`, `0x7779d3`, `0x777b4c`, `0x777b76`, `0x777b9e`, `0x777bad`, `0x768320`, `0x77f0c0`, `0x41ad70`, `0x41ae60`, `0x7ffd68`
Confidence: verified
Proof: one
Test: none

`0x7776e0` allocates the object with `SMemAlloc(0x68)` (`0x77780d`) and constructs it. Each key is read with `0x6f3890` and
`0x6f3a40`: `bgFile` at `0x777823`, `edgeFile` at `0x777854`, `tile` at `0x777884` through `0x6f1c10`
with default false (`0x7778a3`), `tileSize` at `0x7778ef`, `edgeSize` at `0x777948` and `insets` at `0x7779a1`, which is read
only for a table (`0x7779c3`). A number goes through `0x6f34d0`; the length is `x / (G · 1024)` run through `0x41ae60`
for the layout scale, with `G` from `0x41ad70` and 1024.0 at `0x7ffd68` (`0x77790c`, `0x777965`). The four inset locals
are zeroed before the reads (`0x7779d3`) and stored left, right, top, bottom into `+0x58`, `+0x5c`, `+0x50`, `+0x54`
(`0x777b4c` to `0x777b61`). `0x76a5d0` runs at `0x777b76`; if `0x768320` then answers that the frame's rectangle is
valid (`0x777b9e`), `0x77f0c0` computes the texture coordinates at once (`0x777bad`).

## Reading a backdrop back

`GetBackdrop([table])` returns nothing for a frame without a backdrop; otherwise it fills the table given as
argument 2, or a new one, and returns it. The keys are `bgFile`, `edgeFile` (empty strings when unset), `tile`
(the number 1, or nil when tiling is off), `tileSize`, `edgeSize` and `insets`, with lengths converted back to interface units.
`GetBackdropColor` and `GetBackdropBorderColor` return the four components `r`, `g`, `b`, `a` as the
byte times the float `0x3b80806e` (0.00392156, at `0x81c6b4`), not divided by 255, so 255 reads back as 0.9999978 and not 1.0, or nothing without a backdrop.

Addresses: `0x777370`, `0x777401`, `0x777415`, `0x777421`, `0x777426`, `0x77743a`, `0x77744f`, `0x7774ec`, `0x7776cc`, `0x777bc0`, `0x777c68`, `0x777c78`, `0x777d1c`, `0x777f60`, `0x41ae40`
Confidence: verified
Proof: one
Test: none

`0x777370` returns 0 values when `+0x1ac` is null (`0x777401` to `0x777407`), reuses a table at argument 2 or makes one
with `0x6f3c90` (`0x777415` to `0x777421`), replaces an unset name by the empty string (`0x77743a`, `0x77744f`), converts
lengths with `0x41ae40` (`0x7774ec`) and returns 1 (`0x7776cc`). `0x777bc0` and `0x777f60` read the dword at `+0x60`
and `+0x64`: `r` is bits 16 to 23, `g` bits 8 to 15, `b` bits 0 to 7 and `a` bits 24 to 31 (`0x777c78` to `0x777d1c`), each
times the float 1/255, and they return 4.

## The XML element

A `<Backdrop>` reads `bgFile`, `edgeFile` and `tile` as attributes and then its children `TileSize`, `EdgeSize`,
`BackgroundInsets`, `Color` and `BorderColor`. Any other child logs `Unknown child node in %s element: %s` and
loading goes on. A length is the `val` of the element or of an `AbsValue` child, converted from interface units, or
the `val` of a `RelValue` child, stored as it is; with none the loader logs `No "val" attribute in element: %s`.
The insets read `left`, `right`, `top` and `bottom` the same way, with `AbsInset` and `RelInset` children. A colour
reads `r`, `g`, `b` and `a` clamped to 0 to 1; a missing `r`, `g` or `b` is 0 and a missing `a` is 1.

Addresses: `0x77e6c0`, `0x77e6ce`, `0x77e6da`, `0x77e6e9`, `0x77e709`, `0x77e73a`, `0x77e7d8`, `0x77e7fa`, `0x77e831`, `0x77e86e`, `0x77e87b`, `0x769c5f`, `0x769c71`, `0x769c7a`, `0x6f1d60`, `0x6f1d8c`, `0x6f1e6a`, `0x6f20a0`, `0x6f23d0`, `0x6f23e2`, `0x6f23f7`, `0x77f410`, `0x77f440`
Confidence: verified
Proof: one
Test: none

`0x77e6c0` reads the three attributes with `0x6f2cf0` (`0x77e6ce`, `0x77e6da`, `0x77e6e9`), parses `tile` with `0x6f1b30` and
sets the edge mask like the Lua path (`0x77e73a`). It compares each child name case-blind against `TileSize`,
`EdgeSize`, `BackgroundInsets`, `Color`, `BorderColor` (`0x64a4c0`, `0x7fffffff`). `TileSize` and `EdgeSize` call `0x6f1d60`, which converts the `val`
(`0x6f1d8c`) and stores a `RelValue` unconverted (`0x6f1e6a`). `0x6f20a0` fills four out values stored as in the
Lua path (`0x77e7fa` to `0x77e80c`). `0x6f23d0` starts from 0, 0, 0 and 1.0 (`0x6f23e2` to `0x6f23f7`). The colours go to
`0x77f410` and `0x77f440` at once. The frame loader allocates the object, runs `0x77e6c0` and installs it with `0x76a5d0`
(`0x769c5f`, `0x769c71`, `0x769c7a`).

## The pieces and their anchors

Installing builds the background texture for a nonempty `bgFile` and an edge texture for each set mask bit (1 left,
2 right, 4 top, 8 bottom, 0x10 to 0x80 the corners top left, top right, bottom left, bottom right). The background
lies on the `BACKGROUND` layer and the edges on `BORDER`. The border sits inside the frame rectangle and the
insets act on the background alone. The background's bottom right anchor is raised by the top inset, not the
bottom one, so with different top and bottom insets that corner rides high.

| piece | anchors, as point of frame (x, y) | size |
|---|---|---|
| background | top left (left, −top); top right (−right, −top); bottom left (left, bottom); bottom right (−right, top) | none |
| left | top left (0, −edge); bottom left (0, edge) | width edge |
| right | top right (0, −edge); bottom right (0, edge) | width edge |
| top | top left (edge, 0); top right (−edge, 0) | height edge |
| bottom | bottom left (edge, 0); bottom right (−edge, 0) | height edge |
| each corner | its own corner (0, 0) | width and height edge |

Addresses: `0x77e8d0`, `0x77e8e4`, `0x77e91f`, `0x77e941`, `0x77e96e`, `0x77e990`, `0x77e9a6`, `0x77e9ae`, `0x77e9c1`, `0x77e9ca`, `0x77e9f1`, `0x77e9fb`, `0x77ea3f`, `0x77ea59`, `0x77ea75`, `0x77ea79`, `0x77eb93`, `0x77ecf5`, `0x77ed05`, `0x767c70`, `0x76fc40`, `0x7680e0`, `0x811a80`
Confidence: verified
Proof: one
Test: none

`0x77e8d0` asserts a nonnull frame (`0x77e8e4`) and creates each texture with `0x76fc40(frame, layer, 1)`, layer 0 for the
background (`0x77e941`) and 1 for an edge (`0x77ea22`). Points are the 3 by 3 grid numbers 0, 2, 6 and 8 for top left, top
right, bottom left and bottom right, and the relative frame is the frame's own layout part. Each `SetPoint` is `0x767c70`
with six arguments, the last asking for an immediate resolve, which only the corners pass as 1; the multi-anchor
pieces call `0x7680e0` themselves (`0x77e9ca`, `0x77ea79`). The instruction `mov eax,[esi+0x50]` at `0x77e9ae`
loads the top inset for the background's bottom right anchor; the bottom left one loads `+0x54` at `0x77e995`.

## The edge file

The edge file is eight slices across its width, each 0.125 wide, in the order left, right, top, bottom, top left,
top right, bottom left, bottom right. A texture's vertices run top left, bottom left, top right, bottom right.
A side strip repeats its slice along its length with a period of one edge size: the left and right slices run in `v`
from 0 to the repeat count, and the top and bottom slices are turned, with `u` across the strip and `v` from the
repeat count at the left end to 0 at the right. A corner shows its slice whole. A side loads its file with the
wrap flag on and a corner with it off.

Addresses: `0x77f0c0`, `0x77f101`, `0x77f10c`, `0x77f128`, `0x77f150`, `0x77f289`, `0x77f2fc`, `0x77f36d`, `0x77f3de`, `0x77f3e9`, `0x77f3f4`, `0x77ecf5`, `0x77ed8c`, `0x7704c0`, `0x7705b0`, `0x770200`, `0x58a980`, `0x801628`, `0x7ffd74`
Confidence: verified
Proof: one
Test: none

`0x77f0c0(rect)` divides 1.0 by the edge size (`0x77f101`) and makes the width and height runs as `extent · (1/edge) − 2.0`
(`0x77f10c`, `0x77f128`), the 2.0 being the two corners, and zeroes a negative run. The sides send their pairs through
`0x7704c0` for slices at `u` 0 to 0.125, 0.125 to 0.25, 0.25 to 0.375 and 0.375 to 0.5 (`0x77f289`, `0x77f2fc`, `0x77f36d`,
`0x77f3de`). The corners are sent once at creation, with `v` from 0 to 1 and `u` from 0.5, 0.625, 0.75 and 0.875
(`0x77ecf5` onward). `0x7705b0` writes the four vertices in that order. `0x770200` takes the wrap flag as its second
argument, 1 for a side and 0 for a corner (`0x77ed8c`); `0x58a980` puts it in bits 3 and 4 of the texture flags, wrap in U and in V.
Both colours are applied last (`0x77f3e9`, `0x77f3f4`).

## Tiling the background

The background shows its whole file unless `tile` is set. With `tile` and a nonempty `bgFile` it repeats with a period of
the tile size, or of the edge size when the tile size is 0, so its coordinates run from 0 to the width and height
over the period. The background loads its file with the tile flag as the wrap flag.

Addresses: `0x77f0c0`, `0x77f164`, `0x77f16a`, `0x77f175`, `0x77f216`, `0x77f21b`, `0x77e9f1`, `0x770200`
Confidence: verified
Proof: one
Test: none

`0x77f0c0` skips the background unless its first name byte is nonzero (`0x77f164`) and `+0x40` is (`0x77f16a`). It loads `+0x4c`,
compares it with 0.0 and takes `+0x48` instead when it is 0 (`0x77f175`), divides each extent by it, and sends the pairs
(0, 0), (0, h), (w, 0), (w, h) to the texture (`0x77f216`). Without `tile` no call is made (`0x77f21b`) and the texture keeps
its default coordinates. The background's `0x770200` call passes `+0x40` as the wrap flag (`0x77e9f1`).

## The two colours

The background colour applies to the background texture alone and the border colour to all eight edge textures,
never the background. Each is stored in the object and set on its textures as their vertex colour. A script sets
them with `SetBackdropColor(r, g, b[, a])` and `SetBackdropBorderColor(r, g, b[, a])`: each channel is clamped to 0 to 1
with a missing alpha 1, and packed as alpha, red, green, blue bytes rounded as `×255 + 0.5`. A frame without a
backdrop keeps nothing, so a colour set before `SetBackdrop` is lost, and a new object starts white.

Addresses: `0x777d30`, `0x777dc9`, `0x777e87`, `0x777ed9`, `0x777f35`, `0x777f46`, `0x7780d0`, `0x7782d5`, `0x7782e6`, `0x77f410`, `0x77f440`, `0x77f750`, `0x7ffe58`, `0x7ffa24`
Confidence: verified
Proof: one
Test: none

`0x777d30` reads arguments 2 to 4 with `0x6f3620` (`0x777dc9`), clamps each, takes the alpha from argument 5 only when
`0x6f34d0` finds a number (`0x777e87`), multiplies by 255.0 and adds 0.5 and truncates with `0x40a2b0` (`0x777ed9`), and
packs them into one dword with red in bits 16 to 23. With a backdrop (`0x777f35`) it calls `0x77f410`, which stores the bytes at
`+0x60` to `+0x63` and, for a background texture, calls `0x77f750` with them. `0x77f440` stores `+0x64` to `+0x67` and calls
`0x77f750` on each of the eight edge pointers that is set. `0x7780d0` is the same code for the border (`0x7782d5`,
`0x7782e6`).

## When the coordinates are computed

Texture coordinates are recomputed whenever the frame's size changes: the frame's size callback `0x76b580` compares
the new width and height with the previous rectangle's and calls `0x77f0c0` when either differs by at least the float 2.38e-07,
and the pieces' positions follow the anchors without it. Installing a backdrop from XML computes nothing at
once; the first resolution of the frame's rectangle does.

Addresses: `0x76b580`, `0x76b5db`, `0x76b5eb`, `0x76b5f6`, `0x76b611`, `0x76b613`, `0x76b61e`, `0x8029d4`, `0x769c7a`
Confidence: verified
Proof: one
Test: none

`0x76b580` subtracts the stored rectangle's width from the argument's (`0x76b5db` to `0x76b5e7`), takes the absolute value
and compares it with the float at `0x8029d4` (`0x76b5eb`); an equal or larger difference goes to the update, and so does the
height (`0x76b602` to `0x76b611`). At `0x76b613` it reads the backdrop at `+0x1ac` of the frame's layout owner and, when set, calls
`0x77f0c0` with the current rectangle (`0x76b61e`).
