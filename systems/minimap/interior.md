# minimap: interior

How the interior tiles are chosen: the query the update makes of the map object, the flood through its portals, and the grid of tile boxes each group emits. The functions belong to the map object code; the minimap is their one caller. The tile records they fill are `tiles`, the window that bounds the query is `update`, and the draw is `draw`.

Roots: none, the query and the flood are map object code that the minimap calls

## Interior query

The update asks for the interior tiles with `0x670520(node, box, list, limit)`, which calls `0x69daa0`. That function asks `0x69d4f0` for the map object instance, its model, the player's group node and the group (four stack outputs; 0 when none), moves the box into the model frame with the matrix at `+0xd4`. It tail-calls `0x6a5200` on the model with the group index at `+0x7c` of the group node, the model-frame box, the list and the limit. `0x6a5200` counts one more visit pass, returns 0 when the byte at `+0x1d4` of the map object is clear or the seed group is not loaded, and otherwise floods from the seed with a copy of the box whose maximum z is the seed group's.

Addresses: `0x670520`, `0x69daa0`, `0x69dab9`, `0x69dacc`, `0x69dafc`, `0x69db17`, `0x6dc540`, `0x6dc470`, `0x6a5200`, `0x6a5208`, `0x6a520e`, `0x6a5238`, `0x6a5261`, `0x6a524b`, `0x6a5252`, `0x6a4a00`, `0xca7eac`, `0x6d9706`
Confidence: verified
Proof: one
Test: none

`0x6d9706` is the one caller of `0x670520` (`update#interior-window`), and `0x69dab9` is the one call of `0x69d4f0` in `0x69daa0`. `0x69dacc` points `ecx` at `+0xd4` of the instance and `0x69dafc` calls `0x6dc540`, which fills a model-frame box from the world box: the matrix rows are at `+0`, `+0x10` and `+0x20` and the translation T at `+0x30`. Both corners start at T, and `0x6dc470` adds for each output axis j and each row i the smaller of `M[i][j]·min_i` and `M[i][j]·max_i` to the minimum and the larger to the maximum. The product with the box maximum is rounded to a float; the one with the box minimum stays in a register until the add. `0x69db17` calls `0x6a5200` with the seed `[group node + 0x7c]` loaded at `0x69db0c`. `0x6a5208` increments the pass counter `0xca7eac`; `0x6a520e` tests the loaded byte; `0x6a5238` asks `0x6a4a00` for the seed group; `0x6a524b` and `0x6a5252` overwrite the copy's maximum z (the sixth dword of the `rep movs` at `0x6a522e`) with the seed group's `+0x28`; `0x6a5261` starts the flood with the seed as both the group and the group it came from.

## Portal flood

`0x6a5020(group, from, box, list, limit)` visits one group. It stops when the group is not loaded, has flag `0x8` (exterior) or has already been stamped with the current pass. Otherwise it stamps the group and stops when the group does not overlap the box in x and y. The box's top z is the seed group's ceiling, so the top plane of the portal test is that ceiling. If it overlaps, the group emits its tiles and the flood crosses each of its portals to the neighbour group unless the neighbour is none, is the group it came from, or the portal polygon lies wholly outside one of the six planes of the box.

Addresses: `0x6a5020`, `0x6a5046`, `0x6a5055`, `0x6a5064`, `0x6a506a`, `0x6a508b`, `0x6a50b9`, `0x6a50c3`, `0x6a50cc`, `0x6a50f0`, `0x6a50f4`, `0x6a5104`, `0x6a5140`, `0x6a51b4`, `0x6a51bb`, `0x6a51d1`, `0x6a51f4`
Confidence: verified
Proof: one
Test: none

`0x6a5046` tests flag `0x8` at `+0x10` and `0x6a5055` compares the stamp at `+0xb8` with the pass counter. `0x6a506a` to `0x6a50a7` compare the box minimum with the group's maximum and the box maximum with the group's minimum, in x and y, inclusive; a failure jumps to the return `0x6a51f4`, so a group outside the box neither emits nor floods on. The group box is `+0x14` to `+0x28`. `0x6a50b9` emits the tiles. The portal references start at `+0x2c` of the group in the array at `+0x13c` of the map object, `+0x30` of them, of 8 bytes: a portal index and a neighbour index. `0x6a50f4` and `0x6a5104` skip the neighbour `0xffff` and the one it came from. The portal records of 20 bytes at `+0x138` give a first vertex and a count of vertices at `+0x134`, 12 bytes each. `0x6a5140` to `0x6a51b4` set one outcode bit per plane from the sign of each difference between a vertex and a box bound and AND the codes of all vertices; a result of 0 (`0x6a51bb`) recurses at `0x6a51d1`.

## Group tile grid

`0x6a5270(group, index, box, list, limit)` emits the tiles of one group, unless the group's flags have `0x88` set. The group's model-space box is cut into a grid; each cell is a tile whose width and height are the next power of two of the texel extent, between 32 and 256 texels, at 0.5 yard per texel. A cell is emitted when its box overlaps the query box in x and y and the list has room. An entry has the group index, the column, the row and the six floats of the cell box.

| offset | holds |
|---|---|
| `+0x00` | group index |
| `+0x04` | column |
| `+0x08` | row |
| `+0x0c` | min x, min y, then z |
| `+0x18` | max x, max y, then z |

Addresses: `0x6a5270`, `0x6a5276`, `0x6a5297`, `0x6a52a1`, `0x6a52ac`, `0x6a52d9`, `0x6a5346`, `0x6a53a4`, `0x6a53b8`, `0x6a53dc`, `0x6a542f`, `0x6a5438`, `0x6a54e4`, `0x6a551f`, `0x6a5547`, `0x6a554d`, `0x6a5550`, `0x6a555d`, `0x6a5565`, `0x6a5590`, `0x6a663c`, `0x6a666c`, `0xca7ebc`, `0xca7eec`
Confidence: verified
Proof: one
Test: none

`0x6a5276` skips groups with flag `0x8` or `0x80`. `0x6a5297` and `0x6a52a1` form the half-sum of the box corners, whose z is the tile z. `0x6a663c` stores 0.5, `18 * (1/36)`, in `0xca7ebc`, and `0x6a666c` stores 128 (that times 256) in `0xca7eec`. `0x6a52ac` divides by 128; the extent in x and y over 128 is the cell count after `ceil` (`0x6a53a4`, `0x6a53b8`), and times 256 is the texel extent that `0x6a52d9` and `0x6a5346` round up to a power of two with `CeilLog2` (`0x6a5590`, the quotient of two rounded base-10 logarithms, so an exact power of two is not guaranteed to land on its own exponent) and clamp to 32 to 256. The count is the low byte of the float `512 + n` shifted right by 14 bits (`0x6a53c8`, `0x6a53d4`). `0x6a53dc` scales the sizes by 0.5. `0x6a542f` and `0x6a5438` place a cell at the group minimum plus its index times its size, rows outer, columns inner. `0x6a54e4` tests overlap with the query box and `0x6a551f` the limit; `0x6a554d`, `0x6a5550` and `0x6a5565` store the row, the group and the column, and `0x6a555d` copies the six floats.

## Tile edge bleed

The cells on the outer edge of a group's grid grow by one yard. A cell in column 0 lowers its min x by 1, a cell in row 0 lowers its min y, a cell in the last column raises its max x and a cell in the last row raises its max y; the interior edges are shared exactly. A group of one cell grows on all four sides.

Addresses: `0x6a5489`, `0x6a5491`, `0x6a5498`, `0x6a549e`, `0x6a54a5`, `0x6a54ae`, `0x6a54b5`, `0x6a54be`, `0x6a54c8`, `0x6a54d1`, `0x6a54db`, `0xca7ed0`, `0xca8098`
Confidence: verified
Proof: one
Test: none

`0x6a5489` to `0x6a5498` compute the bleed once, the first time a cell is built: they load `0xca7ebc`, double it (`0x6a5491` adds it to itself) and store 1.0 in `0xca8098`, with mask `0x1` of `0xca7ed0` as the guard. `0x6a549e` and `0x6a54a5` lower the min x by it when the column is 0; `0x6a54ae` and `0x6a54b5` lower the min y when the row is 0; `0x6a54be` to `0x6a54c8` raise the max x when the column is the count less one; `0x6a54d1` to `0x6a54db` raise the max y for the last row. A group of one column and one row meets all four tests.
