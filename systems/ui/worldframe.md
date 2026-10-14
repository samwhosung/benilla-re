# ui: the world frame as a widget

What the `WorldFrame` is as a frame: its type, its place in the stratum and input lists, which
slots it overrides, what a press, a release and a wheel notch do on it, how the 3D world is drawn
through it, and why turning the interface off leaves it alone. The pick, the targeting and the
bindings it feeds are `mouseover`, `worldpick` and `keybindings`.

The frame is `CGWorldFrame`, constructed by `0x481960` from the factory `0x4959d0` registered as
`WorldFrame`. It is a `CSimpleFrame` with two vtables, `0x8043e8` and `0x8043b8`, a singleton
stored at `0xb4b2bc`. The tag `WorldFrame` is a registry name only and never a Lua type.

Roots: `0x481960`, `0x482aa0`, `0x482d40`, `0x482d70`, `0x483c40`, `0x483cb0`, `0x483d20`, `0x4959d0`

## Class and place

A `WorldFrame` answers `Frame` to `GetObjectType` and to `IsObjectType("Frame")` and `("Region")`, and
nil to `IsObjectType("WorldFrame")`. It has no parent, as a root next to `UIParent`, sits in stratum
0 at level 0, and is enabled for keys, the mouse and the wheel but not characters. The registered
name is a one-shot: the registry record is removed when the first `<WorldFrame>` is made, so a
second one is an unknown frame type.

Addresses: `0x481960`, `0x48196f`, `0x481ae9`, `0x481aef`, `0x481af9`, `0x481aff`, `0x481b09`, `0x481b14`, `0x481b1f`, `0x484780`, `0x4847b0`, `0x484800`, `0x495942`, `0x495947`, `0x49594c`, `0x4959d0`, `0x6ee439`, `0x6ee1dd`, `0x76ab20`, `0x76ab22`, `0x769307`, `0x76c50b`, `0xb4b2bc`
Confidence: verified
Proof: one
Test: none

The constructor calls the frame constructor `0x48196f`, stores the two vtables (`0x481ae9`, `0x481aef`) and
the singleton (`0x481af9`), calls `0x76a470(0)` (`0x481aff`) and `0x76af00` with kinds 1, 2 and 3 and
`-1` (`0x481b09` to `0x481b1f`). The type slots of `0x8043e8` equal the base's, which is the frame's:
`0x484780` accepts the Frame and Region ids, `0x4847b0` compares with `Frame` and `Region`, and `0x484800`
returns `Frame`. The registration pushes 1 only for this type (`0x495940`); `0x6ee439` tests the flag after the
factory ran and unlinks the record. A top-level XML element is built with a null parent (`0x6ee1dd`), and the base
constructor's `SetParent` then finds old and new both null and returns (`0x76ab20`).

## Overrides

The frame overrides ten of the 36 slots of the base vtable and two of the 11 of its layout part.
The rest, including the type slots, the show and hide pair and the drag slots, are the frame's.

| slot | override | base |
|---|---|---|
| 0x00 | `0x481d00` destructor | `0x769380` |
| 0x38 | `0x481790` update | `0x76b2c0` |
| 0x3c | `0x482aa0` hit test | `0x76b320` |
| 0x44 | `0x482d40` emit | `0x76b3a0` |
| 0x50 | `0x483b10` leave | `0x76b6f0` |
| 0x60, 0x64 | `0x483b70`, `0x483bd0` key down, up | `0x76b7d0`, `0x76bba0` |
| 0x68, 0x6c | `0x483c40`, `0x483cb0` mouse down, up | `0x76bf70`, `0x76c040` |
| 0x70 | `0x483d20` wheel | `0x76c180` |
| layout 0x00, 0x04 | `0x483970`, `0x484770` | `0x76b580`, `0x76c410` |

Addresses: `0x8043e8`, `0x8043b8`, `0x81c498`, `0x81c46c`, `0x481d00`, `0x481790`, `0x482aa0`, `0x482d40`, `0x483b10`, `0x483b70`, `0x483bd0`, `0x483c40`, `0x483cb0`, `0x483d20`, `0x483970`, `0x484770`, `0x482ab4`, `0x5143e0`
Confidence: verified
Proof: one
Test: none

The table is the slot-by-slot difference of the two vtables read from `.rdata`; slots 1 to 0x8 and the
rest of the layout part are equal. `0x482aa0` calls the base probe `0x76b320`, answers 0 on a miss, and on a
hit calls the accessor `0x5143e0` and returns 1 (`0x482ab4`): the frame is hit wherever the point is in its rectangle,
which with `setAllPoints` is the whole screen. It is the last frame in the hover walk.

## Press, release and wheel

A press over the world frame captures it and runs, in order, the Lua `OnMouseDown` script if one is set, the
right-button targeting hook, and the `BUTTONn` binding with `isDown` 1. The Lua script cannot consume the
press. A release goes to the capture and runs the Lua script, then the binding with `isDown` 0,
unless the release ends a drag of the frame itself. A wheel notch is consumed by a Lua `OnMouseWheel` script
and otherwise runs the binding down and up.

Addresses: `0x483c40`, `0x483c4b`, `0x483c50`, `0x483c62`, `0x483c7e`, `0x483c93`, `0x483cb0`, `0x483cbb`, `0x483d20`, `0x483d2b`, `0x483d40`, `0x483d47`, `0x483d6f`, `0x483d82`, `0x76bf70`, `0x76c011`, `0x76c040`, `0x76c08f`, `0x76c180`, `0x76c1c1`, `0x492c20`, `0x492c77`, `0x4b6aa0`, `0x4b7990`, `0x7662c0`, `0x7663e9`, `0x7663f4`, `0x7663f7`, `0x766420`, `0x7664a4`
Confidence: verified
Proof: one
Test: none

`0x483c40` calls the base `0x76bf70`, whose only return is 0 (`0x76c011`), so the test at `0x483c50` is
never taken; then `0x492c20`, which ends in `xor eax,eax` (`0x492c77`) on every path; then
`0x4b6aa0` builds `BUTTONn` and `0x4b7990` runs it with 1 (`0x483c93`). The press handler `0x7662c0` stores the capture
before the call (`0x7663e9`) and overwrites the answer with 1 afterwards (`0x7663f7`). The release
`0x766420` sends to the capture alone (`0x7664a4`). The base release `0x76c040` returns 1 when it
ended a drag (`0x76c08f`), which skips the binding. The base wheel `0x76c180` returns 1 after firing a script (`0x76c1c1`). When it returns 0, `0x483d20` reads the event's dword at `[+0x2c]` (`0x483d40`) and, if it is 0, returns 1 without running a binding (`0x483d47`); otherwise it builds the binding name with `0x4b6aa0` and runs it down and up (`0x483d6f`, `0x483d82`).

## Drawing the world

The 3D world is drawn by a callback the frame registers on its layer 0 batch, in the same walk
that draws every other frame, below all of them. The frame's emit draws its own regions and, for
layer 0 only, registers `0x482d70`, which pushes identity matrices, runs the per-frame world driver
`0x482ea0`, the scene render `0x483460` and the combat text pass `0x6c7900`, and pops them.

Addresses: `0x482d40`, `0x482d50`, `0x482d55`, `0x482d5a`, `0x482d61`, `0x482d70`, `0x482e5f`, `0x482e67`, `0x482e6e`, `0x482e75`, `0x482e7a`, `0x482e7f`, `0x482e8a`, `0x482ea0`, `0x483460`, `0x6c7900`, `0x773110`, `0x58b090`, `0x58b0b0`, `0x58b040`, `0x58b050`
Confidence: verified
Proof: one
Test: none

`0x482d40` calls the base emit `0x76b3a0` (`0x482d50`), then registers when the layer is 0
(`0x482d55` to `0x482d61`) with `0x773110(batch, 0x482d70, this)`. `0x482d70` builds two identity
matrices, pushes them with `0x58b090` and `0x58b0b0` (`0x482e5f`, `0x482e67`), calls `0x482ea0`
(`0x482e6e`), `0x483460` (`0x482e75`) and `0x6c7900` (`0x482e7a`) and pops with `0x58b040` and `0x58b050`. Hiding the
world frame would remove this callback with it.

## Projecting a world point

A world point is projected to the frame's own space by subtracting the camera position, transforming
by the frame's cached matrix, dropping a point whose clip depth is below the camera's near
distance, dividing by w and mapping to 0..1. The result is scaled by the frame's viewport width and
height in layout units, less the frame's left and bottom when they are nonzero. It answers whether
the point is inside the frame, edges included, and writes the point unless the depth test failed.

Addresses: `0x483ee0`, `0x483efa`, `0x483f4d`, `0x483f6f`, `0x483f7a`, `0x483f89`, `0x483fe0`, `0x484029`, `0x48402e`, `0x484065`, `0x484075`, `0x484086`, `0x484093`, `0x4840a3`, `0x483970`, `0x483982`, `0x483a67`, `0x483a86`, `0x7bcb40`, `0x41ad80`, `0x41ade0`, `0x76b580`
Confidence: verified
Proof: one
Test: `ui::geo_483ee0`, `ui::apply_rect_screen`

`0x483ee0` builds `(p − camera position, 0)` from the camera at `[this+0x65b8]+8`, transforms it with `0x7bcb40`
and the matrix at `this+0x3b0`, and returns 0 when `[camera+0x38]` is above the clip z (`0x483f7a`), without
writing the output. It multiplies by `1/w`, adds 1, multiplies by 0.5 (`0x7ffa24`), scales x by `(right − left)` and y by
`(top − bottom)` of the normalised viewport at `+0x3a0` to `+0x3ac` and converts with `0x41ad80`. The layout callback
`0x483970` runs the base callback `0x76b580`, copies the resolved rectangle to `+0x390`, clamps it to the screen and
stores it divided by `K1` and `K2` with `0x41ade0` (`0x483a67`, `0x483a86`), clamped to 0..1. The range test
(`0x484075` to `0x4840a3`) writes the point first (`0x484065`).

`ui::geo_483ee0` runs `0x483ee0` whole, with the matrix transform `0x7bcb40` stubbed to return a chosen clip vector, and `ui::apply_rect_screen` runs `0x76b580` whole.

## Turning the interface off

The interface is turned off by hiding `UIParent`: a hide walks `UIParent`'s child list, and the
world frame is not in it, being a root of its own. The engine has no flag, string or render gate
for it. The engine reads `UIParent` only to set its scale, to own the game tooltip and to find the
interface's scale.

Addresses: `0x76ad50`, `0x76ade9`, `0x76adee`, `0x76ae23`, `0x76ae35`, `0x775810`, `0x76ab49`, `0x490065`, `0x49006a`, `0x490077`, `0xb4b44c`, `0x49456a`, `0x494590`, `0x492a02`
Confidence: verified
Proof: one
Test: none

`0x76ad50` walks `[esi+0x300]` and calls each child's own hide (`0x76adee`); it does not touch a child's
shown flag, so `IsShown` stays true under a hidden parent. The show gate reads the parent's visible
flag only when there is a parent (`0x76ae23` to `0x76ae35`). The string `UIParent` is used once, at
`0x49006a`, to store the global lookup in `0xb4b44c`; its other four uses set the scale (`0x49456a`), read it
(`0x494590`), and pass it as the owner of the game tooltip (`0x492a02`).
