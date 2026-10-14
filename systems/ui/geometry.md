# ui: map geometry

The data behind the world map's cell lookup and the projections between world positions and the map's 0 to 1
coordinates: the continent records, the area bitmap each one holds, the rectangle of a continent, the cell of
a point, and the formulas at the world level and the zone level. The selection, the lists and the Lua API are
in `worldmap`; the markers placed with these projections are `worldmapmarkers`.

A continent record is the unit of this data. The builder `0x4a5d00` makes one per `WorldMapArea` row with area id 0,
in the order of the file.

Roots: `0x4a5d00`, `0x4a6ec0`, `0x4a7100`, `0x4a72b0`, `0x4a7320`

## The continent record

A record is 0x10024 bytes, which is 0x4009 dwords: the map id at `+0x00`, the id of the continent's own `WorldMapArea` row
at `+0x04`, a growable array of the ids of its zones at `+0x08` (management word, count at `+0x0c`, data pointer at `+0x10`),
the area bitmap of 16384 dwords from `+0x14`, and four floats from `+0x10014`: the low and high edges of the continent on the
vertical axis at `+0x10014` and `+0x1001c`, and on the horizontal axis at `+0x10018` and `+0x10020`. The records live in
the array at `0xb6e668`, counted at `0xb6e664`. A zone of a continent is a row with a nonzero area id and the same map id.

Addresses: `0x4a5d00`, `0x4a5d0b`, `0x4a5d1f`, `0x4a5d31`, `0x4a5d7d`, `0x4a5e0c`, `0x4a5e16`, `0x4a5e66`, `0x4a5ebb`, `0x4a5ed6`, `0x4a5ef1`, `0x4a6390`, `0x4a6019`, `0x4a6104`, `0xb6e664`, `0xb6e668`
Confidence: verified
Proof: one
Test: none

`0x4a5d00` counts the `WorldMapArea` rows (32 bytes) whose area id at `+0x08` is 0 (`0x4a5d1f` to `0x4a5d3f`) and resizes the array
to that many records of `0x10024` bytes (`0x4a5d7d`). For each such row it stores the map id and the row id (`0x4a5e0c`,
`0x4a5e16`), counts the rows of the same map id with a nonzero area id (`0x4a5e30` to `0x4a5e5e`), resizes the zone array to that
count, stores the ids in file order (`0x4a5ed6`) and sorts them by `0x4a6390` (`0x4a5ef1`). The next record starts 0x10024 bytes on
(`0x4a6104`).

## The rectangle of a continent

The four edge floats of a record come from the `WorldMapContinent` row with the same map id. With `s` the row's scale
at `+0x20`, the offsets `xo = 31.3125 − 32·s + f6` and `yo = 20.875 − 32·s + f7` from the floats at `+0x18` and `+0x1c`, and the
integers `f2` to `f5` at `+0x08` to `+0x14`, the edges are `(f2·s + xo) / 62.625` and `((f3 + 1)·s + xo) / 62.625` along the
horizontal axis, and `(f4·s + yo) / 41.75` and `((f5 + 1)·s + yo) / 41.75` along the vertical one. A map id with no row leaves
the four floats as the record held them.

Addresses: `0x4a5f22`, `0x4a5f30`, `0x4a5f36`, `0x4a5f42`, `0x4a5f4b`, `0x4a5f57`, `0x4a5f64`, `0x4a5f78`, `0x4a5f9a`, `0x4a5fba`, `0x4a5fdd`, `0x806524`, `0x806528`, `0x806530`, `0x80652c`, `0x806534`
Confidence: verified
Proof: one
Test: none

The loop over the `WorldMapContinent` rows (0x34 bytes apart, `0x4a5f36`) compares the row's map id at `+0x04` with the record's (`0x4a5f30`).
`0x4a5f42` loads `s`, multiplies it by 32.0 and subtracts it from 31.3125 and from 20.875 before adding the two offsets
(`0x4a5f57`, `0x4a5f64`). The horizontal edges are stored at `+0x10018` and `+0x10020` after the multiply by 1/62.625 (`0x4a5f78`,
`0x4a5f9a`), the vertical ones at `+0x10014` and `+0x1001c` after the multiply by 1/41.75 (`0x4a5fba`, `0x4a5fdd`).

## The area bitmap

Each record holds a 128 by 128 grid of area ids. The grid is read from the file named by the continent's row, `Interface\WorldMap\%s.zmp`
with the row's name, 0x10000 bytes straight into the record. A cell holds an `AreaTable` id on disk. It becomes
the id of the `WorldMapArea` row that has the record's map id and an area id equal to the cell's, using the cell's parent
area instead when the area has one; a cell with no area, no `AreaTable` record or no such row becomes 0. When the file does not open,
the record keeps its grid and its remap is skipped.

Addresses: `0x4a5feb`, `0x4a5ffc`, `0x4a600f`, `0x4a601b`, `0x4a604b`, `0x4a6054`, `0x4a6070`, `0x4a607a`, `0x4a6085`, `0x4a6095`, `0x4a6099`, `0x4a60a2`, `0x4a60c7`, `0x4a60e0`, `0x4a60ed`, `0x6477a0`, `0x648460`, `0x648730`, `0x845374`, `0xc0e048`, `0xc0e04c`
Confidence: verified
Proof: one
Test: none

`0x4a5feb` pushes the format at `0x845374` and the row's name at `+0x0c` to the formatter `0x64a7f0` (`0x4a5ffc`), and `0x6477a0` opens the
file (`0x4a600f`); a failure advances to the next record (`0x4a601b`). `0x648460` reads 0x10000 bytes to `record + 0x14` (`0x4a604b`) and `0x648730` closes
the file (`0x4a6054`). The loop of 128 rows of 128 cells (`0x4a6070` to `0x4a60ed`) zeroes the cell (`0x4a607a`), rejects an id of 0 or above the
`AreaTable` maximum (`0x4a6085`) or without a record (`0x4a6095`), takes the parent at `+0x08` of the record when it is nonzero (`0x4a6099`, `0x4a60a2`),
scans the `WorldMapArea` rows for the map id and the area id (`0x4a60c7`) and stores the row id (`0x4a60e0`).

## The cell of a point

`0x4a6ec0(u, v)` returns the area id under a point of the displayed map, or 0. The row is the selection's: the
selected zone, else the continent's own row, else the direct-area word. Both arguments are clamped to 0 to 1. The row's two extents
(`+0x10` less `+0x14`, `+0x18` less `+0x1c`) must be nonzero. The position is `lo + (1 − u)·extent`, and a cell's coordinate on each axis is
`0.5 − position / 34133.33`, which must lie in 0 to 1; the cell is column `trunc(128·x)` and row `trunc(128·y)` of the continent's
grid. A direct area, the world view and a missing row give 0.

Addresses: `0x4a6ec0`, `0x4a6ec7`, `0x4a6ed6`, `0x4a6f05`, `0x4a6f0e`, `0x4a6f1f`, `0x4a6f31`, `0x4a6fbc`, `0x4a6fd1`, `0x4a6ff6`, `0x4a7005`, `0x4a7020`, `0x4a70b8`, `0x4a70c6`, `0x4a70cc`, `0x4a70d6`, `0x4a70dc`, `0x4a70e0`, `0x84506c`, `0x845070`, `0x845074`, `0x806544`, `0x80653c`, `0x806540`, `0xc0d5bc`, `0xc0d5c0`
Confidence: verified
Proof: one
Test: `ui::geo_4a6ec0`

`0x4a6ec7` reads the continent word; a value below 0 takes the direct-area word (`0x4a6f00`), otherwise a zone of 0 or more
takes the child at that index (`0x4a6eed`) and a zone below 0 the continent's own id (`0x4a6efa`). The id must lie in 0 to the maximum of the index
at `0xc0d5bc` (`0x4a6f1f`). `0x4a6f0e` returns 0 for a continent of −2. The two extents are tested against 0.0 (`0x4a6fd1`, `0x4a6fe5`).
`0x4a7005` multiplies `(1 − u)·extent + lo` by 2.9296876e-05, the float at `0x806544`, and subtracts it from 0.5; `0x4a7020` does the same
for `v` with the other pair. `0x4a70b8` multiplies the second by −128.0 (`0x806540`) and `0x4a70c6` the first by 128.0 (`0x80653c`),
both truncated; the index is the column plus 128 times the row plus 0x4009 times the continent's index (`0x4a70cc` to `0x4a70dc`) into the record array, and the cell is read at `+0x14` (`0x4a70e0`).

`ui::geo_4a6ec0` runs `0x4a6ec0` whole on seeded map state.

## Click to world

`0x4a7100(u, v)` turns a point of the displayed map into a world position. At the world level it finds the continent whose rectangle
holds the point, edges included, and returns the position and that continent's map id. For a `WorldMapContinent` row
with scale `s`, the first coordinate is `(f7 − ((v − 0.5) / s)·41.75)·533.33` and the second `(f6 − ((u − 0.5) / s)·62.625)·533.33`. At the zone level it
reads the selected row's rectangle and gives `lo − (lo − hi)·u` for the first coordinate and the same form on the other
pair for the second; `v` is not read. No call reaches `0x4a7100`: its only caller is in an unreached function.

Addresses: `0x4a7100`, `0x4a7117`, `0x4a7145`, `0x4a7152`, `0x4a715e`, `0x4a716b`, `0x4a7195`, `0x4a71c4`, `0x4a71d6`, `0x4a71df`, `0x4a71f3`, `0x4a71fc`, `0x4a7207`, `0x4a726f`, `0x4a727d`, `0x4a7286`, `0x4a7292`, `0x4a7712`, `0x80654c`, `0x806548`, `0x806550`
Confidence: verified
Proof: one
Test: none

At the world level (`0x4a7117`) the loop tests `u` and `v` against the edges `+0x10018`, `+0x10020`, `+0x10014` and `+0x1001c` (`0x4a7145`, `0x4a7152`,
`0x4a715e`, `0x4a716b`), finds the `WorldMapContinent` row by map id (`0x4a7195`) and computes (`0x4a71c4` to `0x4a7202`) with 41.75 at
`0x806550`, 62.625 at `0x806548` and 533.33 at `0x80654c`. The zone level (`0x4a7207`) selects the row as `0x4a6ec0` does and computes
from its four floats (`0x4a726f` to `0x4a7289`); both products use the argument at `[ebp+8]` (`0x4a727d`, `0x4a7286`).

## World to map

`0x4a72b0(a, b)` and the world leg of `0x4a7360` put a world position on the world view. For the `WorldMapContinent` row of the map id, the
horizontal result is `f6/62.625 + 0.5 − b·(1/(62.625·533.33))·s` and the vertical one `f7/41.75 + 0.5 − a·(1/(41.75·533.33))·s`.
`0x4a72b0` returns the horizontal result and one minus the vertical one; `0x4a7360` returns both unchanged. The two
projections do not invert `0x4a7100`: its offset term is `f6·533.33` where the inverse of this formula has `f6·533.33 / s`, so a round trip
closes only when `s` is 1, or when `f6` is 0 on the horizontal axis and `f7` is 0 on the vertical one.

Addresses: `0x4a72b0`, `0x4a72b6`, `0x4a72c9`, `0x4a72d8`, `0x4a72de`, `0x4a72e6`, `0x4a72f5`, `0x4a72fb`, `0x4a7303`, `0x4a7305`, `0x4a730e`, `0x4a7320`, `0x4a7360`, `0x4a7393`, `0x4a73b3`, `0x4a73d1`, `0x4a73d5`, `0x806554`, `0x806558`, `0x7ff9d8`
Confidence: verified
Proof: one
Test: `ui::geo_4a72b0`, `ui::geo_4a7360`

`0x4a7320` finds the `WorldMapContinent` row of the map id in `ecx` (`0x4a7352`). `0x4a72b0` scales `f7` by 1/41.75 (`0x806524`) and adds 0.5 (`0x4a72c9`, `0x4a72cf`), takes `a` times 4.491e-05 (`0x806558`)
times `s` (`0x4a72d8`, `0x4a72de`) away (`0x4a72e1`), does the same for the horizontal pair with 2.994e-05 (`0x806554`, `0x4a72f5`), stores the horizontal result (`0x4a7303`) and
`1.0 − vertical` (`0x4a7305`, `0x4a730e`). The world leg of `0x4a7360` (`0x4a7393`) stores the horizontal result at `0x4a73b3`, the vertical one at `0x4a73d1`, and returns
0 in `al` (`0x4a73d5`).

`ui::geo_4a72b0` and `ui::geo_4a7360` run `0x4a72b0` and `0x4a7360` whole on seeded map state.
