# ui: zone text, zone events and zone PvP

The client names the area under the active player or mover every tenth update and turns a change into one of three zone events. This doc is the poll, the zone commit `0x494780` with its events and name caches, the four text functions, the indoor election, `GetZonePVPInfo` with its gate, and the one place the engine moves the world map selection on its own. The area tables and the terrain lookups are `terrain`.

Roots: `0x494780`, `0x494970`, `0x48a0a0`, `0x48a0c0`, `0x48a0e0`, `0x48a100`, `0x48d540`

## The poll

`0x5db900` counts its calls in `0xc4d790` and on every tenth call takes the active mover's object, else the active player's, and calls `0x67e510(map id, area node)` with the node at `+0xe0` of the object. `0x67e510` asks the node for its indoor bit (`0x6704a0`) and the leaf area id under it (`0x6703a0`), looks the leaf up in the `AreaTable` id map, and takes the zone from the leaf's parent word (`+0x8`): a leaf with no parent is the zone itself and has no subzone.
It does nothing for a zero or unknown area. The names are the locale strings at `+0x2c` of the leaf and the zone rows.

Addresses: `0x5db900`, `0x5db906`, `0x5db937`, `0x5db956`, `0x5db960`, `0x5db96f`, `0x67e510`, `0x67e52d`, `0x67e546`, `0x67e5a7`, `0x67e57d`, `0x67e58a`, `0x67e5b6`, `0x67e5bd`, `0x6704a0`, `0x6703a0`, `0x468580`, `0xc4d790`, `0xc0e048`, `0xc0e04c`
Confidence: verified
Proof: one
Test: none

The mover is checked first, as a unit by `0xc4da98` and `0xc4da9c`, and in normal play is the player. A zero id after the parent step ends the call (`0x67e5a7`), so `0x494780` only sees a nonzero zone. The indoor flag is booleanised at `0x67e532`.

## The indoor choice

The indoor bit is bit 0 of the word at `+0x90` of the entity's light node (`0x6704a0`). Its one setter is `0x6a87f0`, run when the node is attached to a map object: a group whose flags have bit 0x8 (exterior) sets only a lighting bit and leaves the indoor bit clear; any other group sets it. A player or unit attaches by a ray straight down 1000 yards from its position (`0x6a8a20`), so it is
indoors exactly when the nearest surface below it is a face of a map object group without flag 0x8. The refresh `0x69e280` clears the bit before each re-attachment.

Addresses: `0x6704a0`, `0x6704a7`, `0x6a87f0`, `0x6a880d`, `0x6a8811`, `0x6a8820`, `0x6a8825`, `0x6a86d0`, `0x6a8714`, `0x6a8721`, `0x6a8a20`, `0x6a8a54`, `0x6a8b47`, `0x69e280`, `0x69e295`, `0x6a4a00`, `0x8022cc`
Confidence: verified
Proof: one
Test: none

A game object attaches by containment (`0x6a8c10`, mode bit 13 of `+0x90`), not by a ray. Terrain wins when it is nearer than the map object; the map object keeps a tie. When neither is hit below, the ray is retried from the same origin to a point 1000 above the top of the object's bounding sphere. Standing on an exterior group's face reads outdoors, and a porch whose group is exterior stays outdoors until the first face of an interior group. The group flags are the first word after the group header's second field, copied unchanged from the file (`0x6c400b`).

## The map object names

Indoors `0x67e670` can replace the zone name and clear the subzone name. It asks two `WMOAreaTable` questions of the node: the whole map object's default row (group id -1) and the hit group's own row. It returns 0, and `0x67e510` then skips `0x494780`, when the last update was also indoors and the hit group index is unchanged (`0x86860c`).
It overwrites the zone name only when the default row's name is nonempty and differs from the current subzone name, and then clears the subzone; a nonempty group row name then refills the subzone. Outdoors `0x67e7f0` only compares the zone with the last one.

Addresses: `0x67e670`, `0x67e688`, `0x67e698`, `0x67e6a0`, `0x67e6ac`, `0x67e6bf`, `0x67e6ce`, `0x67e6e4`, `0x67e6f0`, `0x67e6f2`, `0x67e6f8`, `0x67e70d`, `0x67e600`, `0x67e602`, `0x67e7f0`, `0x6704e0`, `0x6704f0`, `0x868608`, `0x86860c`
Confidence: verified
Proof: one
Test: none

The call at `0x67e5dc` passes the node, the address of the zone name and the address of the subzone name; the zone name that `GetRealZoneText` keeps is read before the call (`esi`, `0x67e5bd`). The rows are looked up in the `WMOAreaTable` store by a binary search on the map object id, the name set and the group id.

## The commit and its events

`0x494780(zoneId, areaId, zoneName, subzoneName, realZoneName, indoor)` stores the zone id at `0xb4e314` and the area id at `0xb4e318`, copies each nonempty name into its cache (`0xb4b3f8`, `0xb4e280`, `0xb4b404`) and empties a cache for an empty name. At most one event follows. `ZONE_CHANGED_NEW_AREA` (205) fires, alone, when the zone id changed.
Otherwise a change of the zone name or the subzone name fires `ZONE_CHANGED` (203), or `ZONE_CHANGED_INDOORS` (204) when the indoor argument is nonzero. A change of the real zone name alone fires nothing.

Addresses: `0x494780`, `0x494787`, `0x49478f`, `0x494794`, `0x49479a`, `0x4947a0`, `0x4947bf`, `0x494809`, `0x49481a`, `0x49482b`, `0x494876`, `0x494887`, `0x4948a0`, `0x494936`, `0x49493a`, `0x49493f`, `0x49494b`, `0x49495f`, `0xb4e314`, `0xb4e318`, `0xb4b3f8`, `0xb4e280`, `0xb4b404`
Confidence: verified
Proof: one
Test: none

`0x494780` has the one caller `0x67e645`. A change of the real zone name also asks for a refresh of the zone chat channels (`0x49a210` through `0x494931`; `chatchannels#the-zone-channels`) when the player is not in a cinematic and has no pending cinematic. The first commit of a session sees an old zone id of 0, so it always fires `ZONE_CHANGED_NEW_AREA`.

## The text functions

`GetZoneText()`, `GetRealZoneText()`, `GetSubZoneText()` and `GetMinimapZoneText()` each push the string of one cache, or the empty string when it is null: `0xb4b3f8`, `0xb4b404`, `0xb4e280` and `0xb4da28`. The zone text is the zone's name, or indoors the map object's name; the real zone text is the zone's own name before the override. The subzone text is empty when the leaf is the zone and may be empty indoors.
The minimap text is the subzone name when there is one, else the zone name, after the override.

Addresses: `0x48a0a0`, `0x48a0c0`, `0x48a0e0`, `0x48a100`, `0x494970`, `0x67e60f`, `0x67e621`, `0x67e63d`, `0xb4da28`
Confidence: verified
Proof: one
Test: none

Each function pushes one value and takes no argument. `0x494970` updates the minimap text and signals `MINIMAP_ZONE_CHANGED` (206); `0x67e510` calls it with the subzone name or else the zone name. The two names passed to `0x494780` are the ones after the override (`0x67e60f`, `0x67e621`), the real zone name the one before it.

## GetZonePVPInfo

`GetZonePVPInfo()` always pushes three values. The third is 1 when the leaf area's flags (`+0x10`) have bit 0x80, else nil, computed before any bail. The first two are nil unless the gate is open: the world-entry word `0x88272c` is nonzero or the zone row's flags have bit 0x10, and there is an active player. With the gate open the first is `friendly` when the player's faction template friend mask (`+0x10`) meets the zone's faction group mask (`+0x50`),
else `hostile` when its enemy mask (`+0x14`) does, else `contested`; the second is the name of the first `FactionGroup` row whose mask bit is in the zone mask and which also has a nonempty localised name (`0x48d671`), or the empty string.

Addresses: `0x48d540`, `0x48d575`, `0x48d57a`, `0x48d5b4`, `0x48d5c2`, `0x48d5cc`, `0x48d5f9`, `0x48d604`, `0x48d607`, `0x48d613`, `0x48d629`, `0x48d65e`, `0x48d66d`, `0x48d697`, `0x48d6b5`, `0x88272c`, `0xc0dd3c`, `0xc0dd40`, `0xc0dd5c`, `0xc0dd60`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

A zone with no faction group mask answers `contested`, not nil. Nil comes only from a missing zone row, a missing player, a closed gate or a missing faction template row.

## The realm gate

`0x88272c` is set once per world entry from the character list by `ClientInitializeGame`: it is 1 when the `Cfg_Configs` row whose realm type (`+0x4`) equals the selected realm's type (`+0x4` of the record `0x5ab9a0` returns) has a nonzero third field, 0 when the field is 0, and unchanged when no row matches. Nothing clears it; it reads 0 until the first world entry.

Addresses: `0x401570`, `0x40159d`, `0x4015a2`, `0x4015ae`, `0x4015b1`, `0x4015c5`, `0x4015d4`, `0x4015de`, `0x88272c`, `0x5ab9a0`, `0xc27fb8`, `0xc28134`, `0xc0dfa0`, `0xc0dfa4`, `0x46c236`
Confidence: verified
Proof: one
Test: none

The selected realm record is copied by `SelectRealmByName` and held in `0xc27fb8`. The row stride in memory is 0x14. `ClientInitializeGame` runs from the glue screen state 8 (`0x46c236`).

## Map selection at first zone

When the zone id before `0x494780` is 0 (the first commit of a world session), it calls `0x4a6650` (`worldmap#the-resolver`), the world map's resolver, after the new ids are stored. The call selects the player's own continent and zone, so a client that has not opened the map answers `GetCurrentMapContinent` and `GetCurrentMapZone` with the player's own after about ten updates.
The engine never moves the selection again; only `ProcessMapClick`, `SetMapZoom` and `SetMapToCurrentZone` do.

Addresses: `0x4947a6`, `0x4947aa`, `0x4947ac`, `0x4a6650`, `0x491180`, `0x491266`, `0x49126c`, `0x4911d7`, `0x4a64b0`, `0x402039`, `0x401ec0`, `0x401ece`
Confidence: verified
Proof: one
Test: none

The world-session teardown `0x491180` zeroes the zone id, so the next session fires the resolver again; it does not reset the continent and zone words, which keep the previous session's values until the first resolve. The poll is registered after `0x4a5d00` has built the continent records, so the resolver finds them.
