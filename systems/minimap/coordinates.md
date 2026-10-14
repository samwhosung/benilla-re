# minimap: coordinates

The maps between world positions and map tiles, the scale that places the tile block on the disc, and the orientation the interior
tiles are drawn in. The zoom radius is `ui/minimap#the-zoom-radius`.

Roots: `0x6da540`, `0x6da5b0`, `0x6da380`, `0x6da180`

## Tile grid

The world is cut into a 64 by 64 grid of tiles of 533.3333 yards. Row 0 and column 0 lie at world position 17066.666, which is 32 tiles, and
the indices grow as the world coordinates fall. The row index comes from the world y and the column index from the world x. The client
holds the size, its reciprocal 0.001875 and the origin as constants, and builds the half tile 266.667 and the origin again at start-up.

Addresses: `0x80654c`, `0x7ffab4`, `0xceaa88`, `0xceaa8c`, `0xceaa90`, `0xceaa94`, `0x6d8630`, `0x6d8660`, `0x6d8690`, `0x6d86c0`, `0x8116c4`, `0x8116c8`, `0x811734`
Confidence: verified
Proof: one
Test: none

The constants are `0x80654c` (533.3333), 0.001875 at `0x811734` and `0x7ffab4` (17066.666). The four statics are the products of 533.3333 with 0.5 (`0x6d8630`, `0x6d8660`) and with 64 and 0.5 (`0x6d8690`, `0x6d86c0`); the
last two equal the constant `0x7ffab4`. They read the width `0x8116c4` for x and the height `0x8116c8` for y. `0xceaa8c` is stored there and read by nothing. The tile maps read `0x7ffab4` for x and `0xceaa88` for y, so both origins are 17066.666.

## World and tile maps

`TileToWorld(out, tile)` writes the world position of a tile's corner: `x = 17066.666 − tile[1] · 533.3333`, `y = 17066.666 − tile[0] · 533.3333` and `z = 0`.
`WorldToTile(out, world)` is its inverse: for each axis `t = (17066.666 − w) · 0.001875`, less 1.0 when `t` is negative so the cut is a floor, and
then truncated to an integer. `tile[0]` comes from the world y and `tile[1]` from the world x. Both return the output pointer.

Addresses: `0x6da540`, `0x6da5b0`, `0x6da546`, `0x6da54c`, `0x6da55a`, `0x6da56d`, `0x6da586`, `0x6da58e`, `0x6da5b0`, `0x6da5b4`, `0x6da5ba`, `0x6da5c0`, `0x6da5d6`, `0x6da5d8`, `0x40a2b0`
Confidence: verified
Proof: one
Test: `minimap::world_to_tile_difftest`, `minimap::tile_to_world_difftest`

`0x6da5b0` computes `fild [tile] · 533.3333` and subtracts it from `0xceaa88` for the value stored at `+4` (`0x6da5ba`, `0x6da5d8`), then `fild [tile+4] · 533.3333` from `0x7ffab4` for the value at `+0` (`0x6da5d0`, `0x6da5d6`).
`0x6da540` computes the y term first (`0x6da547`), then the x term (`0x6da558`), tests each against 0.0 and subtracts 1.0 when it is below (`0x6da571`, `0x6da586`), and truncates with `0x40a2b0` (`0x6da58e`, `0x6da595`);
the y term is stored first. Both tests run each function whole.

## Floor and NaN

The subtract makes the truncation a floor because `t` is not a negative integer in the world: it is a float difference times a float, exact in 53 bits and never rounded before the compare, and a negative integer would need `|t|` to be a multiple of 1006633, the odd part of the factor's mantissa. A NaN coordinate skips the subtract and gives tile 0.

Addresses: `0x6da540`, `0x6da56d`, `0x6da584`, `0x40a2b0`, `0x811734`
Confidence: verified
Proof: one
Test: none

The compare with 0.0 at `0x6da562` and `0x6da579` tests `ah` with 5 and takes `jp` at `0x6da56d` and `0x6da584`, which also catches the unordered result of a NaN, so the subtract at `0x6da571` and `0x6da586` is skipped. `0x40a2b0` is `fistp` of a 64-bit integer with truncation; a NaN stores the integer indefinite, whose low dword is 0, and that is returned.

## Screen scale

`WorldToScreenScale(map, player, offset, scale, records)` prepares the outdoor tile block and the scale that places it. It stores the player position and the map, runs the blip pass, and finds the tiles of the player moved half a tile each way;
an axis where both land in one tile takes the next tile (the previous at the grid edge), so the block is 2 by 2. It reloads the tiles when mask `0x1` of the dirty word is set or a cached block index differs, and returns 1.
`offset[0]` is the first tile's world y less the player's, over 1066.667; `offset[1]` the world x less the player's, over 1066.667; `scale` is half the outdoor chunk count times 33.3333, over 1066.667.

Addresses: `0x6da380`, `0x6da38a`, `0x6da3b5`, `0x6da3bb`, `0x6da3c5`, `0x6da3d0`, `0x6da3e2`, `0x6da3ee`, `0x6da3f5`, `0x6da417`, `0x6da426`, `0x6da429`, `0x6da435`, `0x6da456`, `0x6da45f`, `0x6da477`, `0x6da489`, `0x6da497`, `0x6da4a6`, `0x6da4b1`, `0x6da4be`, `0x6da4e0`, `0x6da4e6`, `0x6da500`, `0x6da510`, `0x6da51f`, `0x6da528`, `0x6d9a90`, `0xceaa7c`, `0xceaa94`, `0xceaa90`, `0xcea8bc`, `0xceaa74`, `0xceaa78`, `0xceaa6c`, `0xceaa70`, `0x86f698`, `0x86f6b8`, `0x86f6bc`, `0x8116d0`, `0x80fee4`
Confidence: verified
Proof: one
Test: `minimap::world_to_screen_scale_difftest`

`0x6da38a` to `0x6da3b5` store the player position at `0xceaa7c` to `0xceaa84` and the map at `0x86f694`; `0x6da3bb` runs the blip pass `0x6d9a90`. `0x6da3c5` maps the player, `0x6da3ee` the player plus
half a tile (`0xceaa94`, `0xceaa90`) and `0x6da417` the player minus half a tile. The equal-index tests are `0x6da422` and `0x6da439`: an equal pair becomes the next index when below 0x40 (`0x6da429`, `0x6da446`) and the previous index otherwise (`0x6da435`, `0x6da452`).
The reload test is `0x6da456` to `0x6da47d`: mask `0x1` of the dirty word `0xcea8bc`, or any of the four cached indices `0xceaa74`, `0xceaa78`, `0xceaa6c`, `0xceaa70` differing. It calls `0x6da5e0` (`0x6da489`) and caches the indices (`0x6da497`, `0x6da4a6`), clears `0x1`
(`0x6da4b1`) and maps the first tile to the world (`0x6da4be`). The outputs are `(corner.y − player.y) · (1/1066.667)` (`0x6da500`), `(corner.x − player.x) / 1066.667` (`0x6da510`) and `chunkTable[0x86f698] >> 1` as an integer times 33.3333 times the reciprocal (`0x6da4e6`, `0x6da51f`, `0x6da528`).
The chunk table is `{14, 12, 10, 8, 6, 4}` at `0x8116d0`.

## Map change in the scale

`WorldToScreenScale` computes a changed-map flag, but it is always 0, so the flag that reaches `0x6da5e0` is dead. A map change reloads the tiles only through the block indices, or through the reset's −1 indices (`update#reset`).

Addresses: `0x6da380`, `0x6da394`, `0x6da482`, `0x6da5e0`, `0x6d94bb`, `0x6d999f`, `0x6d9a75`, `0x86f694`
Confidence: verified
Proof: one
Test: none

`0x6da394` sets `[ebp-4]` to 1 when the map argument differs from `0x86f694`. Both callers in `0x6d93a0` run after `0x6d94bb` stored the map, or on the branch where the map is equal. `0x86f694` has 8 references and three writers (`0x6d92f9`, `0x6d94bb`, `0x6da3b5`). `0x6da482` passes the flag as `edx`, which `0x6da60b` tests.

## Orientation basis

`UpdateOrientation(node)` refreshes the matrix `0xceaa20`: it asks the node for its orientation and a second matrix (`0xcea9e0`), renormalizes the first, and turns it by 90 degrees about the Z axis. Only the first three rows are used. When the squared length of row 0, `s`, differs from 1.0
by 2.38e-7 or more, every row is scaled so its length becomes `1/√s`: row `k` is multiplied by `(1/√s) / √(s_k)`. The turn is the constant 90.0 times π/180 about the axis (0, 0, 1), with the translation row zeroed first.

Addresses: `0x6da180`, `0x6da186`, `0x6da190`, `0x6da1d5`, `0x6da1e6`, `0x6da1f9`, `0x6da251`, `0x6da2a9`, `0x6da2d7`, `0x6da2d9`, `0x6da2e2`, `0x6da2f2`, `0x6da325`, `0x670570`, `0x7bdd60`, `0xceaa20`, `0xceaa30`, `0xceaa40`, `0xceaa50`, `0xcea9e0`, `0x86f6b4`
Confidence: verified
Proof: one
Test: `minimap::orientation_renormalize_difftest`

`0x6da190` calls `0x670570` with the node, the matrix `0xceaa20` and the second matrix on the stack. `0x6da1b3` to `0x6da1c8` sum the squares of row 0 (`0xceaa20`, `0xceaa24`, `0xceaa28`); `0x6da1d5` compares `|s − 1|` with the float 2.38418579e-07 and
skips to `0x6da2d5` below it; a NaN `s` does not skip and takes the renormalize path. `0x6da1e6` and `0x6da1e8` make `1/√s`; each row is scaled by that divided by its own root (`0x6da1f9` for row 0, `0x6da251` for row 1, `0x6da2a9` for row 2). `0x6da2d9` loads the float at `0x86f6b4`, 90.0, and multiplies it by 0.0174532924; `0x6da2f2`
to `0x6da310` zero the row at `0xceaa50` and build the axis; `0x6da325` calls `RotateMatrix` (`0x7bdd60`) with the angle, the axis and 1. The test runs the function whole with `0x670570` and `0x7bdd60` bracketing.
