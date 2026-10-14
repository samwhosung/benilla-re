# playername: world text

The floating combat text. The combat code submits a string over a unit; the string lives in a
text record for a fixed time, rises, scales and fades by a per-category table, and is placed on
the screen by projecting its world anchor. At most four texts exist per unit.

Roots: `0x6c7840`, `0x6c73f0`, `0x6c8570`, `0x6c8650`, `0x6c8470`, `0x6c8520`, `0x6c7900`

## Submitting a text

`0x6c7840` adds a text over the unit of a name record unless a gate holds: the record's GUID is
the camera's and `0x50d390` returns nonzero, or the object manager's dword at `+0xc8` is
nonzero. `0x6c73f0` takes the first free of the four slots and drops the text when all four are
used; it never replaces one. It looks the unit up, takes its name position, lowers the z by
0.3333333 (`0x3eaaaaab`, an f32 subtract rounded to f32) and constructs the text.

Addresses: `0x6c7840`, `0x6c73f0`, `0x6c743e`, `0x6c7444`, `0x6c7451`, `0x6c745c`, `0x6c8570`, `0x468570`, `0x50d390`
Confidence: verified
Proof: one
Test: `playername::pn_worldtext_anchor_lift`

`0x6c7840` compares the record's GUID with the two dwords at `+0x88`, `+0x8c` of the camera
(`+0x65b8` of the world frame `0xb4b2bc`) and calls `0x50d390`, which loads the float at `+0xec` and
compares it with 1.8315 (`0x3fea6e99`). `0x468570` returns the manager's `+0xc8` dword. With
no gate it forwards the category (`edx` on entry, pushed at `0x6c788a`) and the three stack arguments to `0x6c73f0`. That function scans `+0x20` to
`+0x2c` for the first zero (`0x6c7400`), stops when none is, resolves the unit with `0x468460`
(`0x6c742a`), fetches the name position (`0x608640`) and subtracts the float at `0x807a40` from z. `0x6c8570`
then builds the text and its pointer is stored in the slot (`0x6c7461`).

## Submitters

Four unit functions submit text through `0x6c7840` with the category they choose; the unit
docs (`objectlayer`) hold their gates and strings. The miss word `0x607140` passes the category
`[0x80c48c + 4·k]` for its word kind `k`: 6 for kind 0, 1 for kind 10 and 3 for the other
kinds up to 11. The damage number `0x6128b0` passes 2 when its flag argument is non-zero, else 0. The
experience gain `0x607260` passes 4 and the honor kill `0x6072e0` passes 5 with the address
of its rank value as the icon pointer.

Addresses: `0x607140`, `0x607215`, `0x6128b0`, `0x61297e`, `0x607260`, `0x6072cd`, `0x6072e0`, `0x60736c`, `0x80c48c`
Confidence: verified
Proof: one
Test: none

The call sites load the category into `edx` before the call: `0x607201` indexes the table at
`0x80c48c`, whose twelve dwords are 6, 3, 3, 3, 3, 3, 3, 3, 3, 3, 1, 3; `0x61297e` follows
`neg dl; sbb edx,edx; and edx,2`; `0x6072c8` and `0x607367` move 4 and 5. The text is the global
string a key table at `0x86582c` names (`0x703bf0`), the format `%s: %d` with the
global `XP`, or `%s: %s` with `HK` or `DK`. Kind 0 gives category 6, which has no
row in the table of six (the next dword after it is the +∞ copy `0xce88d0`).

## Text record

A text record (`WORLDTEXTSTRING`, 0x80 bytes, vtable `0x811314`) holds the category, the world
position, the colour, the start time and the string. The constructor takes the category, the
position, the text, an optional colour and an optional icon value.

| offset | holds |
|---|---|
| `+0x00` | vtable `0x811314`; slot 0 is `0x6c8650` |
| `+0x08` | category 0 to 5 |
| `+0x0c`, `+0x10` | half the string's width and height |
| `+0x14` to `+0x1c` | the world position |
| `+0x20` | the colour dword; byte 3 is the text alpha |
| `+0x24` | the start time in milliseconds |
| `+0x28` | the string handle |
| `+0x2c` | the cached size, −1.0 at construction |
| `+0x30` to `+0x38` | the screen anchor |
| `+0x3c` | the text, 0x40 bytes |
| `+0x7c` | the rank icon value, category 5 only |

Addresses: `0x6c8570`, `0x6c8650`, `0x6c86a0`, `0x6c86c0`, `0x6c86d0`, `0x811314`, `0x6c8616`, `0x6c8628`, `0x6c85f7`, `0x6c8639`
Confidence: verified
Proof: one
Test: none

`0x6c8570` returns 0 for a null or empty text. It allocates from `0xce880c` (tag
`WORLDTEXTSTRING`), clears the fields, stores category and position, takes the colour from the
caller's pointer or from the category row's `+0x18` (`0x6c85f7`), stores the clock `0x42c010` at
`+0x24` (`0x6c8616`), copies the text with `SStrCopy` limited to 0x40 bytes (`0x6c8628`) and, for
category 5 with an extra pointer, stores its value at `+0x7c` (`0x6c8639`). `0x6c86a0` calls the
destructor through slot 0 and frees the record. `0x6c86c0` forwards to the draw
`0x6c7cc0` and `0x6c86d0` to the icon record `0x6c7f10`, both after a null test.

## Category table

Six rows of seven dwords describe each category's timing, size and default colour.

| row | rise | fade-in end | fade-out start | duration | size low | size high | colour |
|---|---|---|---|---|---|---|---|
| 0 | 2.0 | 150 | 760 | 1500 | 0.0183333 | 0.0183333 | `0xffffffff` |
| 1 | 2.0 | 150 | 90 | 1500 | 0.0183333 | 0.0183333 | `0xffffffff` |
| 2 | 0 | 150 | 1000 | 1500 | 0 | 0.0275 | `0xffffffff` |
| 3 | 2.0 | 150 | 1000 | 1500 | 0.0183333 | 0.0183333 | `0xffffffff` |
| 4 | 0 | 500 | 2000 | 4500 | 0.0183333 | 0.0183333 | `0x8094008b` |
| 5 | 0 | 500 | 2000 | 4500 | 0.0183333 | 0.0183333 | `0xffe0ca0a` |

Addresses: `0x6c79a0`, `0x6c7b60`, `0x4a2c10`, `0xce8828`, `0xce88b4`
Confidence: verified
Proof: one
Test: none

`0x6c79a0` stores the rows at `0xce8828 + 0x1c·row` from immediates: for row 0 `0x40000000`,
150, `0x2f8`, `0x5dc`, `0x3c962fc9` twice and −1 (`0x6c79ca` to `0x6c79fe`); row 1 repeats it
with fade-out start `0x5a`; row 2 has rise 0, fade-out start `0x3e8`, size low 0 and size high
`0x3ce147ad`; row 3 has fade-out start `0x3e8`; row 4 (from `0x6c7aec`) has 0, `0x1f4`, `0x7d0`,
`0x1194`, and `0x4a2c10` packs its colour from the bytes 0x80, 0x94, 0 and 0x8b; row 5 is built
by `0x6c7b60` with colour bytes 0xff, 0xe0, 0xca, 0x0a. Rows 0 to 2 store 0xffffffff directly;
row 3 packs four 0xff bytes.

## Timing and scale

For a text of age `e` ms (now minus `+0x24`) and its category row, the rise, the size and the
two alpha bytes follow from `t = e / duration`. A text older than the duration is released.

Addresses: `0x6c8070`, `0x6c80b0`, `0x6c82e0`, `0x6c82c0`, `0x6c80ec`, `0x6c8188`, `0x6c8321`, `0x6c833c`, `0x6c8357`, `0x6c8443`, `0x6c8449`, `0x6c844f`, `0x6c845f`, `0xce8804`, `0x6c7c20`
Confidence: verified
Proof: one
Test: `playername::pn_interp_param`, `playername::pn_keyframe_interp`, `playername::pn_time_alpha_fade`, `playername::pn_width_dirty_guard`

Rise: `0x6c8070` returns `e / duration · rise` (an unsigned `fild`, an integer `fidiv`, f64 left
on `st0`). Size: `0x6c80b0` forms `t` (stored as f32); category 2 finds its segment in the table
at `0x8112dc` (stride 16: start, end, low, high; rows (0, 0.1, 0.1, 2.0), (0.1, 0.2, 2.0, 1.0),
(0.2, 1.0, 1.0, 1.0)), interpolates and multiplies by size high; any other category gives
`(high − low)·t + low`. A result at or below 0.001 becomes `0x3a83126f`. `0x6c82c0` stores
the size at `+0x2c` and rebuilds the string only when it changed. Alpha: `0x6c82e0` starts at
255 and 127. Below fade-in end they are `ftol(255·t)` capped at 255 and `ftol(127·t)` capped at
127. From the fade-out start, `s = (e − start) / (duration − start)` (0 when equal) gives
`ftol(255 − clamp(255·s, 0, 255))` and `ftol(255 − clamp(127·s, 0, 127))`: the shadow value
steps from 127 to 255 there and falls to 128. The first byte goes to `+0x23` and `0x5c2750` (`0x6c844f`); the second goes to `0x5c27a0` with the offsets
`0xce8804` (0.002 each). `0x5cd65e` to `0x5cd667` stores min(shadow, text alpha), so in the fade-out
the stored shadow alpha equals the text alpha and falls from 255 to 0.

## Placing a text

The draw `0x6c7cc0` ends a text whose age reaches its duration, applies the size, alpha and rise,
projects the anchor to the screen and clamps the text rectangle inside the viewport. A world
point that does not project releases the text.

Addresses: `0x6c7cc0`, `0x6c7cf7`, `0x6c7d0a`, `0x6c7d24`, `0x6c7d58`, `0x6c7d96`, `0x6c7da8`, `0x6c7dae`, `0x6c7dc0`, `0x6c7e7b`, `0x6c7ea3`, `0x509520`, `0x6c7eab`, `0x6c7ed5`, `0x6c7eff`, `0x832a44`, `0x832a48`
Confidence: verified
Proof: one
Test: `playername::pn_label_clip_quad`, `playername::pn_label_clip_quad_world_z`, `ui::glue_509_509520_viewport_clamp`

With `e = now − [+0x24]` and `d` the category duration, `0x6c7cc0` returns 0 when `e ≥ d`
(`0x6c7ce3`). It calls the size `0x6c80b0`; with no string it returns 1 there. Otherwise it calls the alpha
`0x6c82e0`, makes the world z `z + rise(e)` (`0x6c7d24`), fetches the world frame's rect
(`0x768320`, zero is a failure) and projects with `0x483ee0`: a false result returns 0 so the
caller releases the text. Then, in f32 stores, `x = sx + rect[1]`, `y = sy + rect[0]` and the half extents
`cx = [+0x0c]`, `cy = [+0x10]` bound the centre: `x` into `[cx, W − cx]` and `y` into
`[cy, H − cy]`, where `W` and `H` are `0x41ae60(1.0)` and `0x41ae70(1.0)` (the floats at
`0x832a44` and `0x832a48`; defaults 0.8 and 0.6). The rectangle {y + cy, x − cx, y − cy, x + cx} is then passed to `0x509520` (`0x6c7ea3`, `ecx = 1`,
`edx` the rectangle), and the anchor is computed from the rectangle as it leaves that call. The anchor stored at `+0x30` is
`((right + left)·0.5)/W − cx` for x and `((bottom + top)·0.5)/H` for y, with z 0, and
`0x5c2770` moves the string there.

## Rebuilding the string

When the size at `+0x2c` changes the string is destroyed and created again at the new size, and
its half width and height are stored.

Addresses: `0x6c81a0`, `0x6c81c8`, `0x6c81d4`, `0x6c8217`, `0x6c8222`, `0x6c823c`, `0x6c827d`, `0x6c828a`, `0x6c8292`, `0x6c82ae`, `0xce8800`, `0xce8820`
Confidence: verified
Proof: one
Test: `playername::pn_string_measure_layout`

`0x6c81a0` destroys the string (`0x5c1d00`), asks `0x44d340` for the font pointer of the handle
`0xce8820`, converts `+0x2c` through `0x41ae50`, and measures the text at `+0x3c` with `0x5c2050`
and `0x5c2070` (the width and then the block height for that width, f32). `0x5c1c30` creates
the string with the measured width and height as its box, justification 2 vertical and 0 horizontal,
flags 0 and the text colour `+0x20`, and `0x5c1e60` adds it to the batch `0xce8800`. The half sizes are the
measured width and height times 0.5 (`0x6c8292`, `0x6c82a2`), stored at `+0x0c` and `+0x10`.

## Rank icon records

A category 5 text whose `+0x7c` holds a value from 5 to 19 also queues an icon: the PvP rank
badge number `value − 4` of 15 textures, drawn at the text's screen anchor.

Addresses: `0x6c7f10`, `0x6c7f54`, `0x6c8012`, `0x6c8017`, `0x6c803c`, `0x6c805b`, `0x6c8470`, `0x6c8520`, `0xce87f0`, `0xce87f4`, `0xce87f8`, `0xce88dc`, `0x6c8eb0`, `0x6c8ef0`, `0x6c7c70`, `0x6c7ca0`
Confidence: verified
Proof: one
Test: `playername::pn_glyph_quad_build`, `playername::pn_glyph_quad_build_deltas`

`0x6c7f10` skips a value outside 5 to 19. It fetches the window rectangle (`0x58a240`), takes
`w = r[3] − r[1]` and `h = r[2] − r[0]`, and appends a record of 0x18 bytes to the ring
`0xce87f8` (capacity `0xce87f0`, count `0xce87f4`, grown through `0x6c8ef0`). The record holds:
`[0x00]` = `w·anchor.x − 40.0`, `[0x04]` = `h·anchor.y − ((s − 32.0)·0.5)` with
`s = 2·h·[+0x10]`, `[0x08]` = anchor z, `[0x0c]` = `s`, `[0x10]` = the colour dword at `+0x20`,
`[0x14]` = the texture `0xce88dc[value − 5]`. `0x6c8470` loads the 15 textures at world
entry from `Interface\PvPRankBadges\PvPRank01.blp` to `PvPRank15.blp`, and `0x6c8520`
releases them.

## Drawing icons

The per-frame draw flushes the text batch, then draws every queued icon as a 32 by 32 pixel
quad under a pixel orthographic projection and empties the ring.

Addresses: `0x6c8820`, `0x6c8832`, `0x6c8a9e`, `0x6c8ae7`, `0x6c8b3b`, `0x6c8b78`, `0x6c8bc7`, `0x6c8cc6`, `0x6c8ccb`, `0x6c8d00`, `0x6c8d22`, `0x6c8d35`, `0x6c8e1a`, `0x6c8e96`, `0x5c3d90`, `0xce87a0`, `0xce87c0`, `0x6c86f0`, `0x6c87a0`
Confidence: verified
Proof: one
Test: `playername::pn_render_driver`, `playername::pn_render_driver_index_color`, `playername::pn_matrix_scale`

`0x6c8820` flushes `0xce8800` (`0x5c1ef0`) and returns when the ring holds no record. It
saves the device matrices, sets a pixel orthographic matrix through `0x6c8d00`, builds the four
vertices from `0xce87c0` times 32.0 with the UVs of `0xce87a0` (`0x6c8ae7`), and the indices
`{0,1,3,3,1,2}` (`0x6c8b78`). Per record it sets the diffuse colour to
`(byte 3 of the record's colour dword << 24) | 0xffffff` (`0x6c8bc7`), the record's texture, a
translation by the record's first three floats, and draws; it then sets the count to 0 and
restores the matrices. `0x6c8d00` reads the window rectangle and the viewport fractions,
truncates `r[3] − r[1]` and `r[2] − r[0]` (`__ftol`), and passes `floor(W·x)` and `floor(H·y)`
for each edge, offset by (−0.5, +0.5) when the device caps field `+4` is 0, to the orthographic
builder `0x5c3d90` with the depth range −1 to 1.

## Damage text font

At world entry the damage text font is the global `DAMAGE_TEXT_FONT`, created once.

Addresses: `0x6c8470`, `0x6c8481`, `0x6c848d`, `0x6c849a`, `0x6c84a3`, `0x6c84a8`, `0x6c84b0`, `0x6c8520`, `0xce8820`, `0xce8800`, `0x6c7470`
Confidence: verified
Proof: one
Test: none

`0x6c8470` reads the global through `0x703bf0` (an empty string on a miss), converts the size
0.0183333 (`0x3c962fc9`) with `0x41ae50` and creates the font with `0x44d040` and flags 0,
storing the handle in `0xce8820` (`0x6c84a3`) and a batch from `0x5c1d60` in `0xce8800`. A census
of the literal `0xce8820` finds three sites: the read in `0x6c81c2`, the write here and the
release in `0x6c8520`. The caller `0x401570` runs once at world entry (callers `0x401620` and
`0x401625`), after the interface load.
