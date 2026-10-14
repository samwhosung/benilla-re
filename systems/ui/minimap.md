# ui: Minimap, the widget

The `Minimap` frame's Lua methods and what a click on it does. The map it draws, its blips and the
zoom state live in `Minimap.cpp`, outside this system; this doc gives the call order into them.

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

`PingLocation(x, y)` turns a point given relative to the map's centre, in script units, into a
world point: world X is the player's X plus `y` over the map's width times the zoom's radius, and
world Y the player's Y minus `x` over the map's height times that radius. When a spell is waiting
for a location, it casts there instead, at the ground found by a vertical trace from 100,000 down
to −100,000, raised by 8. Otherwise, in a party or a raid, it sends message 0x1d5 with world X then
world Y.

Addresses: `0x4eeca0`, `0x4eecf3`, `0x4eedcc`, `0x4eedeb`, `0x4eedf7`, `0x4eee21`, `0x4eee31`, `0x4eee48`, `0x4eee67`, `0x4eee74`, `0x4eeeac`, `0x4eeeb3`, `0x4eeed6`, `0x4eeee9`, `0x4eef05`, `0x4eef15`, `0x4eef26`, `0x4eef31`, `0x4eef5b`, `0x4eef67`
Confidence: verified
Proof: one
Test: none

`0x4eeca0` takes the radius from `0x6da9b0` (`0x4eecf3`), reads argument 2 and negates it
(`0x4eedcc`, `0x4eedeb`) and argument 3 (`0x4eedf7`), converts both with `0x41ae60`, divides the
second by the width (slot 0x1c, `0x4eee21`) and the first by the height (slot 0x20, `0x4eee31`),
and adds the player's position (`0x4eee48`). With `0x6e48a0` and `0x6e6320` true (`0x4eee67`,
`0x4eee74`) it traces from 100000.0 to −100000.0 (`0x4eeeac`, `0x4eeeb3`) with `0x672170`
(`0x4eeed6`), adds 8.0 (`0x4eeee9`) and calls `0x6e60f0` (`0x4eef05`); else, with the
party word 0xbc6f48 or 0xb713e0 set (`0x4eef15`, `0x4eef26`), it builds 0x1d5 (`0x4eef31`) and puts
the two floats (`0x4eef5b`, `0x4eef67`).

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
