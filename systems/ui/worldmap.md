# ui: the world map

The world map keeps one selection, three integers, and the lists of continents, zones, landmarks and overlays built from the `WorldMapArea`, `AreaPOI` and `WorldMapOverlay` tables. This doc is the selection and how it moves, the Lua functions that read and change it, the hover and click lookups, the landmark and overlay lists and the position functions. The arrow models and the battleground markers are `worldmapmarkers`.

Roots: `0x4a67a0`, `0x4a6650`, `0x4a5d00`, `0x4a7540`, `0x4a7fa0`, `0x4a7360`, `0x4a8610`

## The selection

Three words hold the selection: the continent `0x84506c`, the zone `0x845070` and the direct area `0x845074`, all -1 at start. A continent of -1 is the world view and -2 a direct area, the map of an instance such as a battleground; 0 or more indexes the continent records `0xb6e668` (0x10024 bytes each, count `0xb6e664`) and a zone of 0 or more indexes the continent's sorted children (`record+0x10`, count `record+0xc`).
`0x4a67a0(continent, zone)` is the only writer. It stores the continent unchecked; the zone only for a valid continent index and a zone below the child count, else -1; and the direct area, the second argument as a row id, only when the continent is exactly -2, else -1.

Addresses: `0x4a67a0`, `0x4a67a6`, `0x4a67ac`, `0x4a67b2`, `0x4a67ba`, `0x4a67c1`, `0x4a67d4`, `0x4a67dd`, `0x4a67e4`, `0x4a67ea`, `0x84506c`, `0x845070`, `0x845074`, `0xb6e664`, `0xb6e668`
Confidence: verified
Proof: one
Test: `ui::worldmap_set_map_head`, `ui::glue_4a6_4a67a0_near_zero_gate`

The two-word model of a continent and a zone cannot express a direct area; the third word is the row id. The rest of `0x4a67a0` rebuilds the landmark and overlay lists and ends by signalling `WORLD_MAP_UPDATE` (event 365, `0x4a6ce4`). A rebuild without a move passes `(continent, direct area if it is set, else zone)`, as the five callers at `0x48f9f6`, `0x4a6460`, `0x6d93d8`, `0x6d9a17` and `0x6dac72` do.

`ui::worldmap_set_map_head` runs `0x4a67a0` from its entry to `0x4a67ef` and reads the three cells.

## The lists

`0x4a5d00` builds the lists from `WorldMapArea` (rows `0xc0d5b4`, 32 bytes, fields id, map id, area id, name, four rectangle floats). A continent is a row with area id 0, in file order. A zone of a continent is a row with a nonzero area id and the same map id; the ids of a continent's zones are sorted by `0x4a6390`, the name of their area. An orphan is a row with a nonzero area id whose map id
matches no continent; their ids are kept in file order, unsorted, in the array at `0xb6e610` (count `0xb6e60c`).

Addresses: `0x4a5d00`, `0x4a5d31`, `0x4a5eae`, `0x4a5ebb`, `0x4a5ef2`, `0x4a5efe`, `0x4a6390`, `0x4a6130`, `0x4a6157`, `0x4a615c`, `0x4a6170`, `0x4a6190`, `0x4a61c3`, `0x4a61ff`, `0x4a6232`, `0x4a623f`, `0x4a6262`, `0xb6e608`, `0xb6e60c`, `0xb6e610`, `0xc0d5b4`, `0xc0d5b8`, `0xc0d5bc`, `0xc0d5c0`, `0x48f5d1`
Confidence: verified
Proof: one
Test: `ui::glue_geom_4a5d00_poi_transform`

`0x4a5d00` has one caller, `0x48f5d1`. Its orphan passes test the area id before the continent scan, a test the data never fails because every area-id-0 row is a continent. `0x4a6262` resets the direct area word to -1.

## The resolver

`SetMapToCurrentZone()` (`0x4a7e20`) runs `0x4a6650`, which takes the active player and selects its map. With no player it selects the world. It looks for a continent whose map id equals the player's (`0x5ea700`); on a match it selects the continent's child whose area id equals the player's zone (`0xb4e314`), else the continent alone.
Only when no continent matches does it look through the orphans for a row with the player's map id and select `(-2, row id)`; with none it selects the world. The engine runs the same resolver at the first zone commit of a world session (`0x4947ac`).

Addresses: `0x4a6650`, `0x4a6674`, `0x4a6697`, `0x4a66a2`, `0x4a66c3`, `0x4a66d5`, `0x4a66dc`, `0x4a66ea`, `0x4a66f6`, `0x4a6717`, `0x4a671e`, `0x4a6765`, `0x4a676b`, `0x4a7e20`, `0x5ea700`, `0xb4e314`, `0x4947ac`, `0x468580`
Confidence: verified
Proof: one
Test: none

The orphan row id is taken from the same id index every reader uses, so it passes the later bound and null tests. `0x4a7e20` pushes no value.

## The getters

`GetMapInfo()` pushes three values: the selected row's area name (the row at the direct area for a continent of -1 or less, the continent's own row, or its child), the texture height and its power-of-two round-up. A direct area has no continent row, so its height and round-up are 0. `GetCurrentMapContinent()` pushes the continent word plus one, so -1 for a direct area, and `GetCurrentMapZone()` pushes the zone plus one.
`GetMapContinents()` and `GetMapZones(continent)` push the names of the lists; `GetMapZones` uses an unsigned bound on `continent - 1`, so a continent of -1 or -2 pushes none. `SetMapZoom(continent [, zone])` calls `0x4a67a0(continent - 1, zone - 1)` with -1 for a missing zone.

Addresses: `0x4a7e30`, `0x4a7e3a`, `0x4a7e48`, `0x4a7e69`, `0x4a6cf0`, `0x4a6d50`, `0x4a7ed0`, `0x4a7f00`, `0x4a7ce0`, `0x4a65a0`, `0x4a7d10`, `0x4a7d57`, `0x4a7d5a`, `0x4a65e0`, `0x4a7db0`, `0x4a7def`, `0x4a7e0f`
Confidence: verified
Proof: one
Test: `ui::worldmap_selected_area_name`, `ui::worldmap_getmapinfo_pot_round`, `ui::glue_4a6_4a6d50_aspect_ftol`

`0x4a6cf0` returns the name of the selected row (field 3 of `WorldMapArea`), from the direct area word when the continent is below 0. `0x4a6d50` returns the texture height as `ftol(H * 1002.0 / W)` of the matching `WorldMapContinent` row's tile counts, or the rectangle ratio when a zone is selected, and 0 when no row matches or W or H is 0.
The round-up keeps the high part of the height and rounds its low byte to a power of two. `GetCurrentMapAreaID` does not exist in this build.

`ui::worldmap_selected_area_name` runs `0x4a6cf0` whole, and `ui::worldmap_getmapinfo_pot_round` runs `0x4a7e69` to `0x4a7e9a` of `0x4a7e30`, the round-up.

## Hover and click

`UpdateMapHighlight(x, y)` pushes eight values and `ProcessMapClick(x, y)` none; both take the cursor as fractions of the map. `0x4a7620` finds the hovered row: none for a direct area; at the world level the continent whose normalised rectangle holds the point (edges inclusive, record offsets `+0x10014` to `+0x10020`); otherwise the `.zmp` cell id from `0x4a6ec0`, which is 0 when the cell is the selected zone's own row.
`0x4a7540` drills: nothing for a direct area; at the world level `SetMap(continent, -1)` for the continent hit; otherwise it finds the cell's row among the current continent's children and selects it, whether a continent or a zone is displayed.

Addresses: `0x4a7f30`, `0x4a7f7d`, `0x4a7540`, `0x4a754b`, `0x4a755a`, `0x4a75ba`, `0x4a75d1`, `0x4a75e8`, `0x4a7608`, `0x4a7fa0`, `0x4a7620`, `0x4a762a`, `0x4a7639`, `0x4a769b`, `0x4a76ab`, `0x4a76b3`, `0x4a76d7`, `0x4a6ec0`, `0x4a6f0e`
Confidence: verified
Proof: one
Test: `ui::geo_4a6ec0`, `ui::worldmap_highlight`

`ProcessMapClick` raises `Usage: ProcessMapClick(x, y)` for a non-number. It never reads the zone word, so a click on a zone map uses the zone's rectangle on the continent's cell grid and can open a neighbouring zone. `0x4a6ec0` refuses the direct area (`0x4a6f0e`). The cell grid is `geometry`'s continent bitmap; its construction is not in this doc.

`ui::geo_4a6ec0` runs `0x4a6ec0` whole on seeded map state. `ui::worldmap_highlight` runs the slices `0x4a8041` to the branch at `0x4a80cb` or `0x4a80ea`, `0x4a82ed` to `0x4a833c`, `0x4a8319` to `0x4a833c`, `0x4a833c` to `0x4a83ac`, `0x4a83b9` to `0x4a858e`, `0x4a84a0` to `0x4a84c4` and `0x4a850b` to `0x4a858e` of `0x4a7fa0`.

## The landmarks

`GetNumMapLandmarks()` pushes the count `0xb6e6b0`. `GetMapLandmarkInfo(index)` is 1-based and pushes five values: the name, the description, the icon index, x and y; out of range it pushes two nils and three zeros. The list is rebuilt by `0x4a67a0` from the `AreaPOI` rows (`0xc0e054`, 0x74 bytes each) and then the one gossip marker.
An `AreaPOI` row is kept when its position (`0x4a79b0`) is not zero in both coordinates, its area passes the exploration test below and its world state (`+0x70`, when nonzero) has a nonzero value (`0x4c5810`).

Addresses: `0x4a8710`, `0x4a8740`, `0x4a876e`, `0x4a877f`, `0x4a87d6`, `0x4a8827`, `0x4a883d`, `0x4a884b`, `0x4a8856`, `0x4a885f`, `0x4a8868`, `0x4a88b3`, `0x4a79b0`, `0x4a79de`, `0x4a79e3`, `0x4a79ee`, `0x4a7a01`, `0x4a7a1b`, `0x4a6812`, `0x4a6863`, `0x4a6868`, `0x4a688a`, `0x4a6890`, `0x4a68bf`, `0x4a68ed`, `0x4a6903`, `0x4a690c`, `0x4a694b`, `0x4a69a6`, `0x4c5810`, `0xb6e648`, `0xb6e650`, `0xb6e6b0`, `0xc0e054`, `0xc0e058`
Confidence: verified
Proof: one
Test: none

An entry stores the row id, x, y and two flag words (`+0xc`, `+0x10`). `0x4a79b0` projects the row (`+0x10`, `+0x14` under the map id at `+0x1c`) only when the row's flags (`+0x20`) allow the displayed level: bit 0x10 for the world, 0x8 for a continent, 0x4 for a zone or a direct area. The icon is the row's `+0x8` unless the entry is not the gossip marker, the flags have bit 0x80 clear and the zone is not -1 or the continent is -2: then it is 15.
The name is the row's locale string at `+0x28` and the description the one at `+0x4c`. The gossip marker (`objectlayer/interaction`, `ui/gossip#the-gossip-point-of-interest`) is appended with the flag at `+0x10` set.

## The exploration test

An `AreaPOI` row with a positive area id, or a `WorldMapOverlay` slot, is kept only if the area (`AreaTable`, found in `0xc0e048`) has `ExplorationLevel` (`+0x28`) below 0, or the object is the active player and its explored bit for the area's bit number (`+0xc`) is set. The bits are 256 bytes at `+0xe6c` of the player block; a bit number of 2048 or more counts as explored. `0x4a9a40` is the bit test.

Addresses: `0x4a9a40`, `0x4a9a54`, `0x4a9a72`, `0x4a9a81`, `0x4a68bf`, `0x4a68d4`, `0x4a68ed`, `0x4a6ba7`, `0x4a6bac`, `0x4a6bc1`, `0x4a6bd4`, `0x4a6bd9`, `0x4a6bfa`, `0x4a6c07`, `0xc0e048`, `0xc0e04c`
Confidence: verified
Proof: one
Test: none

The bit test needs the object to be the active player (`0x468550`), so other players never reveal. In the overlay loop a failing slot is skipped and the next of the four is tried. In the landmark loop the area gate sits before the world-state gate.

## The overlays

`GetNumMapOverlays()` pushes `0xb6e6b4` and `GetMapOverlayInfo(index)` is 1-based and pushes seven values: the texture path, the texture width and height, the two offsets and the two map point coordinates. An index out of range looks up overlay row 0 and, with no such row, pushes nil and six zeros. The list is rebuilt by `0x4a67a0` as row ids of `WorldMapOverlay` (0x44 bytes each). It stays empty with no player, and for a continent or world view: only a zone (zone 0 or more) or a direct area above 0 shows overlays.

Addresses: `0x4a89d0`, `0x4a8a00`, `0x4a8a48`, `0x4a8a62`, `0x4a8abe`, `0x4a8b53`, `0x4a6ad9`, `0x4a6adc`, `0x4a6ae2`, `0x4a6ae8`, `0x4a6af0`, `0x4a6afa`, `0x4a6b00`, `0x4a6b47`, `0x4a6b6e`, `0x4a6b74`, `0x4a6b87`, `0x4a6b8e`, `0x4a6c1c`, `0x4a6c27`, `0x4a6cb0`, `0x4a6cc1`, `0x4a6cdf`, `0xb6e628`, `0xb6e62c`, `0xb6e630`, `0xb6e6b4`, `0xc0d58c`, `0xc0d590`, `0xc0d594`
Confidence: verified
Proof: one
Test: `ui::worldmap_overlay_map_level_gate`, `ui::worldmap_overlay_reveal_gate`

A row is kept when its map-area word (`+0x4`) equals the displayed row id and one of its four area slots (`+0x8`) passes the exploration test; at most one append per row. The displayed row id is the direct area, the continent's own row, or the zone's row. The path is the displayed row's name and the overlay's texture name `+0x20`; a missing row falls back to the `World` folder.

`ui::worldmap_overlay_reveal_gate` runs `0x4a6b6b` to `0x4a6cc4` of `0x4a67a0`, and `ui::worldmap_overlay_map_level_gate` runs `0x4a6ad9` to `0x4a6cdf`, on seeded stores.

## Unit and corpse positions

`0x4a7360(&(x, y), mapId, &u, &v)` normalises a world position. In the world view a `WorldMapContinent` row for the map id gives the position from its scale; otherwise the displayed row (the continent's child, its own row, or the direct area) must belong to that map id and gives `u = 1 - (y - right) / (left - right)` and `v = 1 - (x - bottom) / (top - bottom)`, each 0 for a zero numerator or divisor, and both 0 if either leaves 0 to 1.
`GetPlayerMapPosition(unit)` pushes `u` and `v` for the player or a group member, else 0 and 0. `GetCorpseMapPosition()` projects the cached corpse position under its display map id.

Addresses: `0x4a7360`, `0x4a737f`, `0x4a738e`, `0x4a7393`, `0x4a73de`, `0x4a73f9`, `0x4a7437`, `0x4a7474`, `0x4a74bb`, `0x4a74e5`, `0x4a750e`, `0x4a7320`, `0x4a7870`, `0x4a78ec`, `0x4a78fd`, `0x4a7906`, `0x4a790c`, `0x4a794b`, `0x4a8610`, `0x4a8698`, `0x4a86d0`, `0x4a86df`, `0x4a7960`, `0x4a79a4`, `0x492090`, `0x496400`, `0xc0d5a0`, `0xc0d5a4`
Confidence: verified
Proof: one
Test: `ui::geo_4a7360`

The axes are swapped: `u` follows world Y and `v` world X. `0x4a7870` takes the position of the unit object when it is a player and projects it under the active player's map id; with no object it reads the group record `0x496400` finds unless its status has bit 0x20, with the zone word at `+0x14` giving the map id through `AreaTable` and the signed words at `+0x16` and `+0x18` giving x and y.
A unit in another map, or a map different from the displayed continent's, gives 0 and 0. `GetPlayerMapPosition` raises `Usage: GetPlayerMapPosition("player")` for a non-string.

`ui::geo_4a7360` runs `0x4a7360` whole on seeded map state.
