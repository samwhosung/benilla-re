# playername: name records and plates

A name record (`PLAYERNAMEDESC`, 0x38 bytes) belongs to one unit. It owns the unit's name text,
up to four world texts and the cached height of the name block. The unit code creates the record
and the unit's model calls back into it each time the model draws. The name text is drawn
through a font batch the plate update flushes itself.

Roots: `0x6c76a0`, `0x6c77d0`, `0x6c7780`, `0x6c77f0`, `0x6c7800`, `0x6c78f0`, `0x6c78a0`, `0x6c78b0`, `0x6c7900`, `0x6c7950`, `0x6c7470`, `0x6c7630`

## Name record layout

| offset | holds |
|---|---|
| `+0x00`, `+0x04` | list link: previous link and next record |
| `+0x08` | the name text handle, 0 while none exists |
| `+0x0c` | the name colour dword |
| `+0x10`, `+0x14` | the unit's GUID, low and high |
| `+0x18` | flags: bit 0 rebuild the text, bit 1 refetch the colour |
| `+0x1c` | the frame tick of the last update |
| `+0x20` to `+0x2c` | four world text pointers |
| `+0x30` | float: name block height, lines times the plate scale |

Addresses: `0x6c6c90`, `0x6c6cd0`, `0x6c776b`, `0x6c76a0`, `0x6c77d0`, `0x6c7780`, `0x6c77f0`, `0x6c7800`, `0x6c78f0`, `0xce870c`, `0xce8728`, `0xce872c`, `0xce8730`, `0x6c6b90`
Confidence: verified
Proof: one
Test: none

The constructor `0x6c6c90` zeroes `+0x00` to `+0x14` and `+0x20` to `+0x30`, stores 3 at `+0x18`
and the tick `0xce8724` at `+0x1c`. `0x6c76a0` returns 0 for a zero GUID, for an unknown object
(`0x468460`, kind mask 8) and for an object whose `+0xd8` model pointer is 0. Otherwise it takes
a record from `0xce870c`, constructs it, stores the GUID, links it at the list tail and installs
`0x6c7780` as the model's draw callback with the record as argument (`0x710db0`, `0x6c776b`).
`0x6c77d0` runs the destructor `0x6c6cd0`, which destroys the name text and releases the
four world texts, unlinks the record, and returns it to the pool. `0x6c77f0` and `0x6c7800` set
bit 1 of one record or every record; `0x6c78f0` sets bit 0 of one. The list's link offset is 0
(`0xce8728`), its tail `0xce872c` and its head `0xce8730`, set up empty at `0x6c6b50`.

## Per-frame work

`0x6c78a0` increments the tick `0xce8724`. `0x6c78b0` releases the world texts of every record
whose `+0x1c` differs from the tick. `0x6c7900` stamps the clock (`0x42c010`), then for every
record calls `0x6c6d40` (draw its world texts) and `0x6c6e00` (queue its rank icons), and ends by
jumping to the icon draw `0x6c8820`. The model's draw callback `0x6c7780` calls the plate update
`0x6c6e90` when the unit still exists and its model is ready.

Addresses: `0x6c78a0`, `0x6c78b0`, `0x6c7900`, `0x6c7780`, `0x6c6d40`, `0x6c6e00`, `0x6c6e60`, `0x6c8820`, `0xce8724`, `0x6c7950`
Confidence: verified
Proof: one
Test: none

`0x6c6d40` resolves the unit, requires its model ready (`0x710450`) and visits the four slots
from the last. It releases a text through `0x6c86a0` when the record's GUID is the camera's and
`0x50d390` returns nonzero, or when the text's own draw `0x6c86c0` returns 0. `0x6c6e00` calls
`0x6c86d0` on each slot. `0x6c7950` returns 1 when `+0x08` is non-null; the quest marker code
calls it.

## Plate update

The plate update of a record resolves the unit, shows or hides the name block by the unit's own
rule, sizes it by height above the feet, rebuilds the text when bit 0 is set, positions it and
draws it. Its scale is 0.2 for a height `d` of 4 or less and `d / 4 · 1.5 · 0.2` above that.

Addresses: `0x6c6e90`, `0x6c6ed5`, `0x6c6ee2`, `0x6c6f10`, `0x6c6f25`, `0x6c6f3c`, `0x6c6fd5`, `0x6c6fea`, `0x6c7003`, `0x6c703f`, `0x6c7063`, `0x6c7071`, `0xce8720`, `0xce8734`, `0xce8738`
Confidence: verified
Proof: one
Test: `playername::pn_plate_render`, `playername::pn_plate_render_text_width_scale`, `playername::pn_plate_render_screen_y_accumulate`

`0x6c6e90` returns at once for a zero GUID or unknown object. It stores the tick at `+0x1c`,
asks `0x6070a0` whether the name shows for the mask `0xce8720`, and takes the unit's name
position (`0x608640`) and the position of the unit's virtual slot `+0x14`. `d` is the first z
minus the second's `+8`; the scale is 0.2 (`0x3e4ccccd`), or `d/4.0·1.5·0.2` when `d > 4.0`
(`0x6c6f10` to `0x6c6f3c`, an f32 store). A NaN height gives 0.2 (`test ah,0x41`, `0x6c6f25`). With bit 1 set the unit's slot `+0x2c` writes the colour
into `+0x0c`. With bit 0 set the text is destroyed and rebuilt: `0x608f50` fills a 0x400-byte
buffer and returns the line count, `+0x30` becomes count·scale (`0x6c6fea`), and `0x5c1c30`
creates the string at size `scale` in a 100000.0 by 100000.0 box with flags `0xc8` and justification 2
vertical, 1 horizontal (font `0xce8738`, text through `0x7716f0`). A shown text joins the batch `0xce8734` (`0x5c1e60`), its z
gets `+0x30` added (`0x6c7071`), `0x5c2770` places it and `0x5c1ef0` flushes. A hidden name
destroys the text. A change of shown state calls `0x6076c0`. The tests run the slices
`0x6c6f10` to `0x6c6f43`, `0x6c6fe4` to `0x6c6fed` and `0x6c706e`.

## Raid target marker

A unit that carries a raid target mark and is alive gets a textured quad over the name:
the mark index `i` below 8 selects the cell `(i mod 4, i div 4)` of a 4 by 2 atlas, each cell
0.25 wide and high. The four corners are `(-0.5, 1)`, `(0.5, 1)`, `(0.5, 0)` and `(-0.5, 0)` times
the plate scale, with the cell offset added to each UV.

Addresses: `0x6c70a2`, `0x6c70ac`, `0x6c70c2`, `0x6c70d8`, `0x6c71c3`, `0x6c7200`, `0x6c7248`, `0x6c728b`, `0x6c72be`, `0x6c734c`, `0x6c7395`, `0x6c73b6`, `0xce873c`, `0xce875c`, `0xce878c`, `0x6c6a10`, `0x6c6ac0`
Confidence: verified
Proof: one
Test: `playername::pn_plate_render_billboard_quad`, `playername::pn_plate_render_accumulate2`

`0x4bb190` returns the mark index for the GUID; the code continues only for an index below 8,
a unit with no nameplate (`+0xe60` is 0) and a unit for which `0x605f30` is false. When the name is shown,
`scale` is added to the anchor z (`0x6c70d8`, tested). The corner loop (`0x6c7200` to `0x6c7248`, tested) copies
corner `k` from `0xce875c` and writes `u = (i mod 4)·0.25 + du[k]`, `v = (i div 4)·0.25 +
dv[k]` from `0xce873c`, whose pairs are (0,0), (0.25,0), (0.25,0.25), (0,0.25). Six 16-bit
indices `{0,1,3,3,1,2}` follow (`0x6c728b`), and the texture is `0xce878c`. The view
matrix is read from the device, the anchor is carried into view space (`0x7bca80`) and the quad
is placed by a translate and a scale matrix built through `0x7bdc40`, `0x7bcf90`, `0x7bd6c0` and
`0x7bce30`. Those four callees are not read here; the order and arguments are as stated.

## Name cvars and font

World entry creates the name font and registers five cvars. `0x6c7470` shuts the module down
first, reads the global `UNIT_NAME_FONT` and creates the font at size 0.99 with flags 4.

Addresses: `0x6c7470`, `0x6c7630`, `0x6c75c0`, `0x6c7483`, `0x6c749b`, `0x6c74ca`, `0x6c75a6`, `0xce8720`, `0xce8734`, `0xce8738`, `0xce878c`, `0x6c68e0`
Confidence: verified
Proof: one
Test: none

`0x704350` reads the global `UNIT_NAME_FONT`; on a hit `0x5c1ae0` creates the font into
`0xce8738` with flags 4 and size `0x3f7d70a4` (0.99), and a miss skips the creation. The text batch
`0xce8734` comes from `0x5c1d60`. `0x63db90` registers five cvars with callback `0x6c75c0`: `UnitNameOwn` and
`UnitNameNPC` (default "0") with masks 1 and 2, and `UnitNamePlayer`, `UnitNamePlayerGuild` and
`UnitNamePlayerPVPTitle` (default "1") with masks 4, 0x10 and 0x20. The callback parses the
cvar value as a decimal integer (`0x64ac60`), sets or clears the mask in `0xce8720`, and when the
word changed sets bit 0 on every record. The marker texture `Interface\TargetingFrame\UI-RaidTargetingIcons` loads into `0xce878c`. `0x6c7630` destroys the
font, the batch, the name pool and the texture.
