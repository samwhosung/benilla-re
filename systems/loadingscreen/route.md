# loadingscreen: route

The transport route a transfer or a leg change shows: the entries that find the transport's taxi
path and leg, the leg table built from the TaxiPathNode store at start-up, the spline ribbon and its
marker quads, their drawing over the 12 world map tiles, and the tiles' geometry. A built route
makes the screen a world map instead of the map's art.

Roots: `0x4072c0`, `0x407de0`, `0x408000`

## Transfer entries

A transfer raises the screen through `0x4072c0`. It stores the destination map in `0x82f00c`, then,
if the active player rides a transport game object whose entry equals the first argument, finds the
transport's taxi path and the leg whose first stop lies on the map of the second argument, and builds
the route. A route that builds sets the map id to −1; any failure leaves the plain screen.
`0x407de0` takes the leg as an argument and builds the route for the player's transport when its
entry matches. Both end in a raise.

Addresses: `0x4072c0`, `0x407de0`, `0x401900`, `0x5f50a0`, `0x468550`, `0x468460`, `0x5f8150`, `0x621b00`, `0x4073f0`, `0x4073de`, `0x407e66`, `0x82f00c`, `0xc0d6a4`, `0xc0d6ac`, `0xc0d6b0`, `0x4073d4`, `0x407e59`
Confidence: verified
Proof: one
Test: none

`TransferPendingHandler` (`0x401900`) calls `0x4072c0` with `ecx` the transport entry, `edx` the second
value and the map on the stack when both values are present and the second is not −1 (`0x40196f`),
else `0x406640`. `0x4072d3` stores the map first. The player and transport are `0x468460` lookups
(`0x4072e9`, `0x40730e`; the guid comes from the player's slot +0x24, `0x4072fa`); `0x407320`
compares the transport's entry field with `ecx`. The path id is the record's column for semantic
id 0x23 (`0x621b00`, `0x5f8150`). The loop `0x407364` to `0x4073c3` reads the path's leg list, stops at
a pair with a stop id of 0 or less, and tests the continent (`[node+0xc]`) of each leg's first stop
node against `edx`; no match calls `0x4073f0` with leg −1, which fails. `0x4073d4` stores −1 in the map
id when `0x4073f0` succeeds. `0x407de0` is the same with `edx` the leg; it compares the entry at
`0x407e2c`, and its sole caller is the transport tick (`0x5f54c2`). Both reach `0x406800`.

## Taxi leg table

At startup `0x408000` builds the per-path leg table from the TaxiPathNode store, one outer entry
for every taxi path id up to the TaxiPath store's highest. Walking the nodes in order, a node with
flag 2 records its id as the run's last stop, and as its first stop when the run has none; a node
with flag 1, or the first node on a new continent within a path, closes the run: it appends the pair
(first stop, last stop) to the pair array and the pair's index to the path's leg list, and clears
the first stop. A change of path closes a run that has a first stop, in the path before it.

Addresses: `0x408000`, `0x402ad0`, `0x402ae4`, `0xc0d6a8`, `0xc0d6a4`, `0x882b50`, `0x882b54`, `0x882b58`, `0x882b60`, `0x882b64`, `0x882b68`, `0x408a40`, `0x408d00`, `0x408b40`
Confidence: verified
Proof: one
Test: none

The node record is 0x24 bytes: id `+0`, path `+4`, continent `+0xc`, flags `+0x1c`. `0x408006`
sizes the outer array to the TaxiPath store's highest id plus 1 (`0x408026` grows it through `0x408b40`, `0x40806d` shrinks
it). `0x40811f` loads the flags: bit 0 (`0x408122`) appends at `0x408141` and `0x40814b` without testing
the first stop; a path change (`0x4080cb`) appends only when the first stop is above 0 (`0x4080d0`); the
continent change is tested at `0x408163` and `0x40816b` for equal paths; bit 1 (`0x4081a2`) writes the last
stop (`[ebp-4]`) and the first stop (`edi`) when it is 0. The last stop is never cleared, and a run
still open after the last node is dropped. `0x4081d3` calls `0x4081e0`. `0x402ae4` is the only call.

## Route ribbon

`0x4075a0` builds the route for a taxi path and leg from the LoadingScreenTaxiSplines row with that
path and leg (`0x407da0`): point A (the last stop of the leg's pair), point B (the first stop of the
next leg's pair, wrapping), each in world map coordinates (`0x4a72b0`), and the row's waypoints between
(an x or a y above 0, up to 8). The spline over A, A, the waypoints, B, B has arc length `L`; the route
has `segs = max(trunc(L · 66.66667), 1)` and `4·segs + 12` vertices. `0x407900` fills the first 8 with
marker quads (±0.02, ±0.0265 around A and B), `0x407c80` the rest in `segs + 1` steps of four at the
spline point plus the ship offsets.

Addresses: `0x4075a0`, `0x407da0`, `0x407900`, `0x407c80`, `0x4073f0`, `0x4a72b0`, `0x453360`, `0x453390`, `0x4088c0`, `0x7ffd84`, `0x8828e0`, `0x8828e4`, `0x8828e8`, `0xc0daf0`, `0x4077f4`, `0x407814`
Confidence: verified
Proof: one
Test: `loadingscreen::route_tessellate_segs_difftest`, `loadingscreen::route_endpoint_quads_difftest`

A route fails when `0x407da0` finds no row, the path or leg is out of range (`0x4075bd`, `0x407602`),
a node id is 0 or has no node (`0x40761e` to `0x40764c`, `0x40767c`, `0x407684`), or `0x4a72b0` returns 0
(`0x4076cc`, `0x4076eb`). The next leg is `(i + 1) mod n` (`0x4075e4`). The spline has the vtable
`0x7ffd84` and is built over `n + 4` points (`0x4077d8`, `0x4077db`). `0x453360` returns the arc length, and
`0x4077f4` to `0x407814` multiply it by 66.66667 and truncate. `0x407750` to `0x407798` store a waypoint while its x or y is above 0 (a NaN counts) and end at the first pair with neither. A row is 0x4c bytes: the path at `+4`, 8 x
values at `+8`, 8 y values at `+0x28`, the leg at `+0x48`. A vertex is 24 bytes (x, y, z, u, v,
colour). In `0x407c80` the parameter starts at 0 and gains `1/segs` as a float each step, clamped to 0 below and
1 for 1 and above or a NaN; the colour dword is not written. The tests run that slice and `0x407900` whole.

## Route drawing

When the route has vertices, `0x406f20` draws the 12 tiles `Interface\WorldMap\World\World1` to
`World12` in a 4 × 3 grid, then the route, and returns 1, so the bar is skipped. The route draws
the two marker quads and the ribbon quads whose index `k` satisfies `k/(n − 1) ≤ fraction`, with
`n = segs + 1`. A ribbon quad's colour is set to white when the loop reaches it, and the first quad
that fails is coloured but not drawn. The `DynamicElements` texture is bound, and the ribbon is a
triangle list of six indices per quad.

Addresses: `0x406f20`, `0x406fb0`, `0x4074b0`, `0x407410`, `0x882d70`, `0x882b70`, `0x8828a4`, `0x40701a`, `0x407025`
Confidence: verified
Proof: one
Test: none

`0x406fb0` computes the quad count `(count − 8) / 4`, truncated toward zero, and for each quad stores
`0x8828a4` in the four vertices' colour dwords (`0x406ffd` to `0x407017`) before comparing `k/(n − 1)` with
`0x882be4` (`0x40701a`); `je` at `0x407025` leaves the loop when the ratio is greater. The index list is
built on the stack: `(b, b+1, b+2, b+1, b+3, b+2)` with `b = 8 + 4k`, then `(0, 1, 2, 1, 3, 2)` and
`(4, 5, 6, 5, 7, 6)` for the markers. The draw passes the vertex count `8 + 4·revealed` and the index count
`6·revealed + 12`. `0x4074b0` loads `DynamicElements` into `0x882b70` and the twelve tiles by
`SStrPrintf` of `World%d`; `0x407410` closes all thirteen handles and frees the vertex array, and
`0x4073f0` runs it when a build fails.

## World map tiles

`0x4081e0` fills the arrays of the 12 tiles for a 1002 × 668 pixel map in 256-pixel tiles, 4
columns and 3 rows. Tile `(c, r)` has the screen corners `x = px/1002` and `y = 1 − py/668` for its
pixel rectangle clipped to 1002 and 668, and the texture rectangle (0, 0) to (w, h), with w = 1 and
h = 1 except w = 0.9140625 in column 3 and h = 0.609375 in row 2. Corners are at `0x8828f8`, three
floats each, and texture coordinates at `0x882be8`, two floats each, in strip order: lower left, lower
right, upper left, upper right.

Addresses: `0x4081e0`, `0x8828f8`, `0x882be8`, `0x40831a`, `0x408328`
Confidence: verified
Proof: one
Test: `loadingscreen::worldmap_tile_geometry_difftest`

`fild` widens the pixel counts, and `fmul` by 1/1002 and 1/668 and `fsubr` from 1.0 run in 53-bit
precision, each store a float. The corners of a tile are stored at `0x8828f8 + 48·i` and the coordinates at
`0x882be8 + 32·i`, `i = c + 4r`. The partial column's w is the dword `0x3f6a0000` (`0x40831a`), and the
partial row's h is a constant loaded at `0x408328`. The test runs `0x4081e0` whole.

