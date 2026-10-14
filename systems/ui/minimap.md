# ui: Minimap, the widget

The `Minimap` frame's Lua methods, what a click on it does, the ping that follows, the zoom radius and the
textures it loads. The blips, arrows and dots it draws are in `minimapblips`; the map tiles live in
`Minimap.cpp`, outside this system.

`CGMinimapFrame` (`MinimapFrame.cpp`, constructor `0x4edbc0`) is a frame with 10 methods of its own.

Roots: `0x4ef2f0`

## Zoom

`GetZoomLevels` and `GetZoom` return integers from the minimap's state. `SetZoom(level)` takes a
number, truncated toward zero, and raises `Usage: SetZoom(level)` for anything else.

Addresses: `0x4eebe0`, `0x4eebe9`, `0x4eec20`, `0x4eec29`, `0x4eec60`, `0x4eec68`, `0x4eec7d`, `0x4eec84`, `0x4eec8d`
Confidence: verified
Proof: one
Test: none

`0x4eebe0` pushes `fild` of `0x6da9a0()` (`0x4eebe9`) and `0x4eec20` of `0x6da980()`
(`0x4eec29`). `0x4eec60` requires `0x6f34d0` on argument 2 (`0x4eec68`), truncates with
`0x40a2b0` (`0x4eec7d`) and calls `0x6da8e0` (`0x4eec84`); else it raises the usage string
(`0x4eec8d`).

## A click on the map

`PingLocation(x, y)` turns a centre-relative point in script units into a world point. The first number
is divided by the map's height and the second by its width, the reverse of the names: world X is the
player's X plus `y` over the width times the zoom diameter, and world Y the player's Y minus `x` over the
height times that. Without numbers it uses the player's position. A spell waiting for a location casts
at the ground there (a trace from 100,000 to −100,000, raised by 8) and no ping is sent. Otherwise, in a
party or raid it sends message 0x1d5 with X then Y, and in any case it applies the ping itself.

Addresses: `0x4eeca0`, `0x4eecf3`, `0x4eecf8`, `0x4eed0b`, `0x4eed1f`, `0x4eedcc`, `0x4eede6`, `0x4eedeb`, `0x4eedf7`, `0x4eee11`, `0x4eee1c`, `0x4eee21`, `0x4eee31`, `0x4eee48`, `0x4eee67`, `0x4eee74`, `0x4eeeac`, `0x4eeeb3`, `0x4eeed6`, `0x4eeee9`, `0x4eef05`, `0x4eef15`, `0x4eef26`, `0x4eef98`, `0x4eefb5`, `0x4eef31`, `0x4eef4f`, `0x4eef5b`, `0x4eef67`, `0x4eef72`
Confidence: verified
Proof: one
Test: `ui::glue_4ea_4eeca0_ping_projection`

`0x4eecf3` takes the radius from `0x6da9b0` and `0x4eecf8` doubles it. If the active player object is
missing nothing happens (`0x4eecdf`). Arguments 2 and 3 are read only when both are numbers (`0x4eed0b`,
`0x4eed1f`); the offsets start at 0.0. Both go through `0x41ae60` (`0x4eede6`, `0x4eee11`); only the first
is then negated (`0x4eedeb`), and the second is stored as converted (`0x4eee1c`); slot 0x1c, `GetWidth`, divides the second (`0x4eee21`) and slot 0x20,
`GetHeight`, the first (`0x4eee31`); each is multiplied by the diameter and the player's position added
(`0x4eee48`). With `0x6e48a0` and `0x6e6320` true (`0x4eee67`, `0x4eee74`) it traces from 100000.0 to
−100000.0 (`0x4eeeac`, `0x4eeeb3`) with `0x672170` (`0x4eeed6`), adds 8.0 (`0x4eeee9`) and calls
`0x6e60f0` (`0x4eef05`). Else with the party word `0xbc6f48` or the raid count `0xb713e0` set
(`0x4eef15`, `0x4eef26`) it builds 0x1d5 (`0x4eef31`) with the two floats (`0x4eef5b`, `0x4eef67`) and
sends it (`0x4eef72`). `0x4eef98` then calls `0x4ee330` with the player's GUID (`0x4eefb5`).

## The ping

One ping is kept, as a world point in two floats. The same routine applies a ping you click and one a
party member sends, so the two are indistinguishable. It does nothing without the active player object.
Otherwise it stores the point at once, resolves the pinger's unit token (`player`, `party` and the member's
1-based number, or `raid` and the number; a GUID that matches none leaves `player`) and signals the event
`MINIMAP_PING` with the token and two numbers: `−dy·k` and `dx·k`, where `dx` and `dy` are the point minus the
player's position and `k` is 1 over the zoom diameter. Nothing limits the rate and nothing checks that the
point lies inside the disc.

Addresses: `0x4ee330`, `0x4ee341`, `0x4ee35e`, `0x4ee366`, `0x4ee373`, `0x4ee3b4`, `0x4ee3c2`, `0x4ee3f2`, `0x4ee412`, `0x4ee41b`, `0x4ee437`, `0x4ee456`, `0x4ee45b`, `0x4ee463`, `0x4ee473`, `0x4ee476`, `0x4ee48d`, `0x5e3573`, `0x5e38c0`, `0x5e4185`, `0x5e41bd`, `0x703f50`, `0x51b3ab`, `0xbc787c`, `0xbc7880`, `0x4e86d0`, `0x491920`, `0x491940`
Confidence: verified
Proof: one
Test: `ui::glue_4ea_4ee330_scale`

`0x4ee330(guid, point)` is `__fastcall`. The two stores (`0x4ee366`, `0x4ee373`) come before the token. The
party scan counts with `0x4e86d0` and reads GUIDs with `0x491920` (`0x4ee3c2`), formatting `party%d`
(`0x4ee412`); the raid scan reads the count at `0xb713e0` and uses `0x491940` and `raid%d`
(`0x4ee41b`). `0x4ee437` calls slot 5 for the position. The event is 0x165, the entry the table write at
`0x51b3ab` names `MINIMAP_PING`, with the format `%s%f%f` through `0x703f50` (`0x4ee48d`). The first number
is rounded to a float (`0x4ee473`) before it is negated; the second stays a double (`0x4ee476`). The
inbound handler for opcode 0x1d5 is registered at `0x5e3573` and reads the sender GUID then two floats
(`0x5e4185`) before calling `0x4ee330` (`0x5e41bd`).

## Reading the ping

`GetPingPosition()` returns two numbers that it computes anew from the stored world point every call: `−dy·k`
then `dx·k`, from the player's current position and the current zoom. The marker therefore stays on its
world spot as the player moves, and a zoom change rescales it. With no player object it returns two zeros.
Before any ping the stored point is (0, 0), which the function does not detect. Nothing in the client clears
the stored point; its lifetime is the interface's own timer.

Addresses: `0x4eefd0`, `0x4eeff6`, `0x4ef021`, `0x4ef038`, `0x4ef044`, `0x4ef05b`, `0x4ef063`, `0x4ef07b`, `0x4ea1f0`, `0xbc787c`, `0xbc7880`, `0x6f3810`
Confidence: verified
Proof: one
Test: `ui::ping_position`

`0x4eeff6` skips the zero pushes when the player exists. It reads `0xbc787c` (`0x4ef021`) and `0xbc7880`
(`0x4ef038`), subtracts the player's position, doubles the radius from `0x6da9b0` (`0x4ef044`) and returns 2
values pushed with `0x6f3810`. The second value is stored as a float (`0x4ef05b`) and the first stays a
double (`0x4ef063`); `0x4ee330` rounds the other way. The cells `0xbc787c` and `0xbc7880` have six
references in the image: two initial zero stores in a static initializer (`0x4ea1f0`), the two stores in
`0x4ee330` and the two loads here.

`ui::ping_position` runs `0x4eefd0` whole, with the object lookups `0x468550` and `0x468460` and the zoom radius `0x6da9b0` replaced and `lua_pushnumber` captured.

## The zoom radius

The radius of the disc in yards is `0.5 · 33.333332 · {14, 12, 10, 8, 6, 4}[z]` outdoors, 233.3 yards down
to 66.7, and `{150, 120, 90, 60, 40, 25}[z]` indoors, where `z` is 0 to 5. A byte the minimap update sets
chooses between the tables. The two indices are the options `minimapZoom` and
`minimapInsideZoom`, default 3, read when the blips are rebuilt for a world. `SetZoom` stores an index
of 5 or more as 5, which includes a negative one, in the active index and its option. The ping, the blips
and the dots all use this radius.

Addresses: `0x6da9b0`, `0x6da9b6`, `0x6da9bf`, `0x6da9cf`, `0x6da9e9`, `0x6da9ef`, `0x6dab70`, `0x6da8e0`, `0x6da8fd`, `0x6da902`, `0x6da980`, `0x6da9a0`, `0x6d9013`, `0x6d9016`, `0x6d901f`, `0x6d902a`, `0x48fc6c`, `0x48fc88`, `0x86f698`, `0x86f69c`, `0x8116d0`, `0x8116e8`, `0x80fee4`, `0xceaa60`
Confidence: verified
Proof: one
Test: `minimap::zoom_to_scale_difftest`

`0x6da9b6` reads the indoor byte `0xceaa60`: set loads `[0x8116e8 + 4·index]` of the indoor index at `0x86f69c`
(`0x6da9bf`); clear loads the integer from `0x8116d0` at the outdoor index `0x86f698` as a 64-bit value,
multiplies by 0.5 and by 33.333332 (`0x6da9e9`, `0x6da9ef`). `0x6dab70` is the same body for the draw. The tables
are 6 entries each. `0x6da9a0` returns 6, `0x6da980` the active index. `0x6da8e0` picks the cell by the
indoor byte, caps the argument with an unsigned compare against 5 (`0x6da8fd`, `0x6da902`) and writes it with
`CVar::Set`. The options are registered at `0x48fc6c` and `0x48fc88`; `0x6d9016` and `0x6d901f` copy their
integers into the two indices, then the event 0xcf (`MINIMAP_UPDATE_ZOOM`) is signalled (`0x6d902a`).
`0xceaa60` has one writer, in the update `0x6d93a0`, which sets bit 0 of the dirty mask `0xcea8bc` and
signals the same event when it changes. The test runs `0x6da9b0` and `0x6dab70` whole, both
branches of `0xceaa60` and all six indices.

## Minimap textures

The minimap loads its mask (`Textures\MinimapMask`), landmark icons (`Interface\Minimap\POIIcons`) and dot
icons (`Interface\Minimap\ObjectIcons`) every time the world is entered, and releases them when it is left.
`SetMaskTexture`, `SetIconTexture` and `SetBlipTexture` replace the three handles and release the old ones.
The world entry loads before the interface is built, so a texture set by an add-on while it loads wins; one set
later lasts to the next logout, and `ReloadUI` keeps it.

Addresses: `0x4ed810`, `0x4ed874`, `0x4ed89d`, `0x4ed8c8`, `0x4edb70`, `0x4edb8a`, `0x4edb9f`, `0x4edbac`, `0x48f590`, `0x48f681`, `0x49118c`, `0x4ee5e9`, `0x4ee5af`, `0x4ee789`, `0x4ee929`, `0xbc7968`, `0xbc827c`, `0xbc822c`, `0x84c6e4`, `0x84c6c8`, `0x84c6a8`
Confidence: verified
Proof: one
Test: `ui::geo_4ed810`

`0x4ed810` has no guard flag: it loads the three names with `0x449d90` and stores the results at `0xbc7968`
(`0x4ed874`), `0xbc827c` (`0x4ed89d`) and `0xbc822c` (`0x4ed8c8`), overwriting the old handles. Its caller
runs at `0x48f590` in `CGGameUI::InitializeGame`, before the jump to the interface build `0x48fbf0`
(`0x48f681`). `0x4edb70` releases each non-null handle through `0x41aed0` and zeroes it (`0x4edb8a`,
`0x4edb9f`, `0x4edbac`), called from `ShutdownGame` (`0x49118c`). The three setters store at `0x4ee5e9`,
`0x4ee789` and `0x4ee929` after releasing the old handle (`0x4ee5af`). The reload path does not reach either
function.

`ui::geo_4ed810` runs `0x4ed810` whole, with the texture and mesh calls stubbed, and compares both tables it fills.

## The engine children

The minimap's constructor makes nine model frames with the minimap as their parent before its XML children exist,
so the first nine entries of `GetChildren()` are these, in the order they are made, and the XML children follow in
declaration order. Five are kept in a group at `+0x320`, three at `+0x314` and one at `+0x338`; the ninth is the
player arrow. `minimapArrowModel` names the model of the eight in the two groups and `minimapPlayerModel` the model
of the ninth alone. A missing attribute takes `Interface\Minimap\Rotating-MinimapArrow.mdx` and
`Interface\Minimap\MinimapArrow.mdx`, and a model that does not load logs `Invalid minimapArrowModel in Minimap.xml`
or `Invalid minimapPlayerModel in Minimap.xml`.

Addresses: `0x4edbc0`, `0x4ede3f`, `0x4ede48`, `0x4ede8b`, `0x4ede9e`, `0x4edeaf`, `0x4edece`, `0x4edee0`, `0x4edf38`, `0x4edf7b`, `0x4edfd1`, `0x4ede4e`, `0x4ede66`, `0x4ede72`, `0x4edef5`, `0x4edefb`, `0x4edf1f`, `0x4edfa2`, `0x4edfae`, `0x76c8e0`, `0x4ee170`, `0x4ee1b7`, `0x4ee204`, `0x4ee260`, `0x4ee269`, `0x4ee2b0`, `0x4ee2c5`, `0x4ee2d5`, `0x4ee2f6`, `0x4ee306`, `0x707350`, `0x6f2cf0`
Confidence: verified
Proof: one
Test: none

The constructor runs `0x6462e0` with the size 0x3dc and `MinimapFrame.cpp` at three sites (`0x4ede66`, loop count 5 at
`0x4ede4e`; the three-loop from `0x4edefb`; `0x4edfa2`), each followed by the model frame constructor `0x76c8e0` with
the minimap as parent (`0x4ede72`, `0x4edf1f`, `0x4edfae`). `0x4ee2b0` reads each attribute with `0x6f2cf0` (`0x4ee2cc`,
`0x4ee2fd`) and falls back to the default string (`0x4ee2d5`, `0x4ee306`). `0x4ee170` creates a model from the file with
`0x707350` and loops over the five and the three (`0x4ee1b7`, `0x4ee204`); `0x4ee260` sets the file on the single child
at `+0x338` (`0x4ee269`).

The constructor stores the minimap's level plus 3 at `+0x334` (`0x4ede3f` to `0x4ede48`) and relevels each of the five of the first group to
that minus 1, the minimap's level plus 2, with `0x76a4f0(level, 0)` (`0x4edee0`); the three of the second group are not relevelled. Each of those
eight is anchored CENTER to the minimap's CENTER with offsets 0 and 0 through `0x767c70(4, minimap, 4, 0, 0, 1)` (`0x4ede8b`, `0x4edf38`),
given the size `[0xbc7970]` through slots 5 and 6 (`0x4ede9e`, `0x4edeaf`), and hidden through slot 0x84 (`0x4edece`, `0x4edf7b`). The
ninth is anchored the same way with offsets 0.001 and 0.001 (`0x3a83126f`, `0x4edfd1`) and the constructor makes no hide call on it.

## Two small helpers

`0x4ef370` stores its four float arguments in order in a 16-byte record and returns it. `0x4ead90(out, i, a, b, c, d)` takes the pair of floats
`t0` and `t1` of record `i` of a table of 24-byte records and writes `out[0] = 2·(a − t1)`, `out[1] = 2·(b − t0)`, `out[2] = 2·(c − t1)` and
`out[3] = 2·(d − t0)`. The rounding is uneven: outputs 0 and 3 are doubled from a difference rounded to float, outputs 1 and 2 from one held in a register.

Addresses: `0x4ef370`, `0x4ef37b`, `0x4ef389`, `0x4eb962`, `0x76b119`, `0x4ead90`, `0x4eadc5`, `0x4eaddd`, `0x4eadeb`, `0x4eadf0`, `0x4eb9fe`, `0xbc7810`
Confidence: verified
Proof: one
Test: `ui::geo_4ead90`, `ui::geo_4eb910`

`0x4ef370` is a `__thiscall` that moves its arguments to `[ecx]`, `[ecx+4]`, `[ecx+8]` and `[ecx+0xc]` as plain dwords and returns `ecx` (`ret 0x10`,
`0x4ef37b` to `0x4ef389`). Its callers are `0x4eb962`, which passes four differences of a rectangle and a point (`0x4eb93f` to `0x4eb95c`), and `0x76b119`.

`0x4ead90` (`ret 0x18`, `ecx` unused) reads `t0` at `0xbc7810 + 24·i` and `t1` at `0xbc7814 + 24·i`. The difference `a − t1` is stored to a float
(`0x4eadc5`) before it is doubled, as is `d − t0` (`0x4eaddd`). `b − t0` and `c − t1` stay in x87 registers at the control word's 53-bit precision,
are doubled there and rounded once to float on the store (`0x4eadeb`, `0x4eadf0`). The two orders differ only below the float normal range, since
doubling is exact otherwise. Its one caller is `0x4eb9fe` in `0x4eb910`, which copies a four-float rectangle from `[ebp+8]` as the last four
arguments. The table has four records, the last two floats of each unread; `0x4eaf00` and `0x4eb910` follow.

## Blip rectangle clip

`0x4eaf00(index, p, q, h, u, v)` clips the square `[p − h, p + h] × [q − h, q + h]` against record `index` of the table at `0xbc7810`, whose first four floats are `x min`, `y min`, `x max` and `y max`. It clears bit 0 of the object's `+0x94`, ignores an index of 4 or more, and calls `0x4eb910` when the clipped rectangle overlaps the record with positive extent, then sets that bit. A clipped edge is the record's unless the square's edge is strictly inside it (ties and NaN keep the record's). The overlap test is strict; a NaN comparison does not reject.

Addresses: `0x4eaf00`, `0x4eaf06`, `0x4eaf1e`, `0x4eaf59`, `0x4eaf8f`, `0x4eafd9`, `0x4eafe6`, `0x4eaff6`, `0x4eb006`, `0x4eb023`, `0x4eb028`, `0x4ebbd3`, `0x4ebb90`, `0xbc7810`
Confidence: verified
Proof: one
Test: `ui::glue_4ea_4eaf00_bbox_edges`

The test runs `0x4eaf24` to `0x4eaf4e`. The square's edges are stored in the order `q − h`, `p − h`, `q + h`, `p + h` (`0x4eaf24` to `0x4eaf4b`), a y, x, y, x order that the clipped rectangle keeps. `0x4eaf59` (`test ah,0x41; jne`) keeps the record's `x max` or `y max` when it is at most the edge or unordered, else the edge; `0x4eaf8f` and `0x4eafad` (`test ah,5; jp`) keep the record's `x min` or `y min` when it is at least the edge or unordered. All four ties and NaNs therefore give the record's value. The overlap tests are: clipped `y1` above `y min` (`0x4eafd9`, leaves when below or equal), `y0` below `y max` (`0x4eafe6`, leaves at greater or equal), `x1` above `x min` (`0x4eaff6`) and `x0` below `x max` (`0x4eb006`); unordered continues in all four. The call passes the clipped rectangle, the index, the address of `p` (`p` and `q` are adjacent), `h`, the address of the square's rectangle, `u` and `v` (`0x4eb016`, `0x4eb017`). The one caller `0x4ebb90` calls it for 512 objects with the index 0 to 511 (`0x4ebbb0` to `0x4ebbe6`); only the first four reach the clip.

## Blip quad geometry

`0x4eb910(clipped, index, pq, h, square, u, v)` fills the object in `ecx` from a clipped `(y0, x0, y1, x1)`. With `d0 .. d3` the clipped `y0, x0, y1, x1` minus the square's `q−h`, `p−h`, `q−h`, `p−h`, `inv = 1/(2h)`, `y0f = inv·d0`, `x0f = inv·d1`, `y1f = inv·d2`, `x1f = inv·d3` and `s = u·v`, the corners at `+0`, `+0xc`, `+0x18`, `+0x24` are `(s·x0f, s − s·y1f, 0)`, `(s·x1f, s − s·y1f, 0)`, `(s·x0f, s − X0, 0)` and `(s·x1f, s − X0, 0)` with `X0 = d0·inv·u·v`. `+0x30` holds the `0x4ead90` offsets and `+0x50` the fractions, as pairs in the corner order `(x0, y1)`, `(x1, y1)`, `(x0, y0)`, `(x1, y0)`.

Addresses: `0x4eb910`, `0x4eb939`, `0x4eb962`, `0x4eb967`, `0x4eb97b`, `0x4eb988`, `0x4eb9fe`, `0x4eba03`, `0x4eba98`, `0x4ebace`, `0x4ebaf1`, `0x4ebaf9`, `0x4ef370`, `0x4ead90`
Confidence: verified
Proof: one
Test: `ui::geo_4eb910`

`0x4eb939` to `0x4eb962` store the four differences as floats through `0x4ef370` at `[ebp-0x1c]`. `0x4eb967` doubles `h` with a float store and divides 1.0 by it on the x87 stack, so `inv` keeps 53 bits; each fraction is the product, rounded to 53 bits on the stack, then stored as a float (`0x4eb97b` to `0x4eb9ce`, stored at `+0x50` to `+0x6c`). `0x4eba03` to `0x4eba60` copy the output of `0x4ead90` (called at `0x4eb9fe` with the clipped rectangle) to `+0x30`. `0x4ebace` forms `u·v` at 53 bits and stores it as a float. `s·x0f` and `s·x1f` multiply that float by the float fractions and store the product; `s − s·y1f` subtracts the 53-bit product from the float `s` and stores the result (`0x4ebaf9`, `fsubr` is `m32 − st0`); `X0` multiplies two unrounded 53-bit values (`0x4ebada`), which differs from `s·y0f`.

`ui::geo_4ead90` runs `0x4ead90` whole on a seeded table entry, and `ui::geo_4eb910` runs `0x4eb910` whole with `0x4ead90` real.

## Lua methods

`Minimap` has 10 methods at `0x84c538`, registered by `0x4ef2f0`: `SetMaskTexture`,
`SetIconTexture`, `SetBlipTexture`, `SetArrowModel`, `SetPlayerModel`, `GetZoomLevels`, `GetZoom`,
`SetZoom`, `PingLocation` and `GetPingPosition`.

Addresses: `0x84c538`, `0x4ef2f0`, `0x4ef320`, `0x4ee4a0`, `0x4ee640`, `0x4ee7e0`, `0x4ee980`, `0x4eeab0`, `0x4eebe0`, `0x4eec20`, `0x4eec60`, `0x4eeca0`, `0x4eefd0`
Confidence: verified
Proof: one
Test: none

The table is 10 pairs of a name and a function in the order above; `0x4ef2f0` calls `0x701d80` with
0xa. `0x4ef320`, slot 2, looks in the `Minimap` hash and then calls `0x778590`.
