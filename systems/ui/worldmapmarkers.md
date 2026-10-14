# ui: world map arrows and battleground markers

The arrow models that the world map and the battleground minimap show for the player, and the Lua functions that give the positions of battleground teammates and the flag carrier. The selection and the projection of a position onto the map are `worldmap`; the battleground queue and score are `battlefield`.

Roots: `0x4a8bb0`, `0x4a7c20`, `0x4a8d20`, `0x4a9120`, `0x4aad40`, `0x4abf90`, `0x4ac230`, `0x4ac3d0`

## The arrow frame

`CreateWorldMapArrowFrame(parent)` and `CreateMiniWorldMapArrowFrame(parent)` make one model widget each, once: a 0x3dc-byte `Model` frame under the given frame, loaded with the minimap arrow model, kept in `0xb6e6b8` (world map) and `0xb6e6bc` (battleground minimap). A second call changes nothing, even with another parent. The model scale is the screen height scale times 5/3 for the world map and 10/9 for the minimap.
Both raise `Usage: CreateWorldMapArrowFrame(parent)` (and the mini form) for a non-table argument, and an error when the table has no frame object or a wrong type.

Addresses: `0x4a8bb0`, `0x4a8bb9`, `0x4a8bd0`, `0x4a8c1d`, `0x4a8c3e`, `0x4a8c60`, `0x4a7a80`, `0x4a7a8a`, `0x4a7aa5`, `0x4a7ab1`, `0x4a7ac8`, `0x4a7ad3`, `0x4a7ad8`, `0x4a7b80`, `0x76c8e0`, `0x80655c`, `0xb6e6b8`, `0xb6e6bc`
Confidence: verified
Proof: one
Test: none

The scale is stored at `+0x3a0` of the model (`0x4a7ae3`) and the constant at `0x80655c` is 5/3. The frame base constructor shows the new frame; this family does not hide it. The caches are cleared by the interface shutdown (`0x490c44`) and the world-leave path (`0x4a64be`).

## Updating and showing the arrows

`UpdateWorldMapArrowFrames()` does two things for each arrow whose model is loaded: it centres the model in its own rectangle (`0x4a7b20` stores half the width, half the height and 0 at `+0x390`), and it copies the facing of the camera's followed object into the model at `+0x39c`. With no object the facing is unchanged.
`ShowWorldMapArrowFrame([shown])` and the mini form show or hide the model exactly as `Frame:Show` and `Frame:Hide` do, with no argument meaning show; with no arrow they do nothing. None of the functions pushes a value.

Addresses: `0x4a8d10`, `0x4a7c20`, `0x4a7c29`, `0x4a7c33`, `0x4a7c46`, `0x4a7c77`, `0x4a7c99`, `0x4a7cb1`, `0x4a7cba`, `0x4a7b20`, `0x4a7b32`, `0x4a7b42`, `0x4a9120`, `0x4a9132`, `0x4a913f`, `0x4a9149`, `0x4a9155`, `0x4a915f`, `0x4a9170`, `0x4818f0`
Confidence: verified
Proof: one
Test: none

`0x4a7c20` looks up the object at the camera's followed GUID (`+0x88`, `+0x8c`) with type mask 1 and takes its facing from slot `+0x18`. `0x6f1c10` coerces the argument. The centring needs the model's size, which is its bounding box, so it takes effect once the model file is resident.

## Positioning an arrow

`PositionWorldMapArrowFrame(point, frame [, relativePoint [, x, y]])` is `SetPoint` on the model. The first two arguments must be strings (else the usage error), the point names parse through `0x6f1840` (`Unknown frame point`), the frame is found by name (`Couldn't find frame named '%s'`), and a frame that is the arrow itself raises `Error: %s is anchored to itself`.
A relative point that is not a string defaults to the point, and offsets that are not both numbers are 0. Each offset is divided by 1024 times the scaled aspect (`0x41ad70`) and multiplied by the horizontal screen scale (`0x41ae60`) before `0x767c70`.

Addresses: `0x4a8d20`, `0x4a8d3c`, `0x4a8d81`, `0x4a8da1`, `0x4a8dbe`, `0x4a8df8`, `0x4a8e2c`, `0x4a8e51`, `0x4a8e98`, `0x4a8ea0`, `0x4a8eb2`, `0x4a8f20`, `0x767c70`, `0x6f1840`, `0x41ad70`, `0x41ae60`, `0x7ffd68`
Confidence: verified
Proof: one
Test: none

With no arrow the function does nothing. The mini form `0x4a8f20` is identical on `0xb6e6bc`. The offset conversion is the inverse of the size getters (`nameplates#size-and-anchors`).

## The position packet

The packet 0x2e9 (handler `0x4aad40`, always answering 1) is `u32 count`, then `count` records of `u64 guid, f32 x, f32 y`, then `u8 hasCarrier`, and with it set `u64 guid, f32 x, f32 y`. The records go to `0xb6e710` (guids) and `0xb6ea30` (x and y, 8 bytes each), the count to `0xb6ebdc`, the carrier to `0xb6e6d8` and `0xb6eb94`.
A clear flag zeroes the carrier GUID and both floats. The loop is bounded only by the wire count, and the arrays hold 40.

Addresses: `0x4aad40`, `0x4aad4f`, `0x4aad60`, `0x4aad6a`, `0x4aad79`, `0x4aad88`, `0x4aad9d`, `0x4aada5`, `0x4aadb0`, `0x4aadbc`, `0x4aadc8`, `0x4aadd8`, `0x4a9b90`, `0x4a9c84`, `0xb6e6d8`, `0xb6e710`, `0xb6ea30`, `0xb6eb94`, `0xb6ebdc`
Confidence: verified
Proof: one
Test: none

A count above 40 writes past the guid array into the globals after it and past the position array: a client fault. The handler is registered at `0x4a9c84`; the position request is sent at most once per 5000 milliseconds while a battleground slot is active (`battlefield#joining-and-leaving`). No function here signals an event.

## Teammate positions

`GetNumBattlefieldPositions()` pushes the number of stored teammates that are not the active player, not one of the four party slots and not on the raid roster. `GetBattlefieldPosition(index)` pushes three values for the index-th of those (1-based, `ftol`): the map fractions `u` and `v` and the name, or 0, 0 and nil for an index of 0 or less or past the end.
The position is the live object's when a player object exists for the GUID, else the packet's; `0x4a7360` normalises it under the map id of the active queue slot (0 with no slot). The name is the name cache's, `Name-Realm` when the realm is not empty, and nil on a miss, which registers `0x4ac190`.

Addresses: `0x4abf00`, `0x4abf45`, `0x4abf51`, `0x4abf90`, `0x4abfa6`, `0x4abfd2`, `0x4abfe6`, `0x4ac00b`, `0x4ac01c`, `0x4ac020`, `0x4ac02a`, `0x4ac056`, `0x4ac07b`, `0x4ac08c`, `0x4ac0b5`, `0x4ac0d6`, `0x4ac118`, `0x4ac13c`, `0x4ac16d`, `0x4ac190`, `0x4e7f70`, `0x4baee0`, `0x55f080`, `0x8457cc`, `0xb6e9d0`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

The filter is shared by the count and the index walk, and the index counts survivors only. `0x4e7f70` is true for the player and the four party GUIDs, `0x4baee0` for a raid member. `0x4ac190` signals `WORLD_MAP_NAME_UPDATE` (event 366) and is the only event of the family. The usage error is `Usage: GetBattlefieldPosition(index)`.

## The flag carrier

`GetNumBattlefieldFlagPositions()` pushes 1 when the carrier GUID is nonzero, else 0. `GetBattlefieldFlagPosition(index)` takes only index 1 and pushes the map fractions `u` and `v` and a token. The position is the carrier's live position or its group record's (`0x4a7870`) when either fraction is nonzero, else the packet's floats under the active slot's map id.
The token is `HordeFlag` for an Alliance player and `AllianceFlag` for a Horde player, nil with no carrier, so it names the flag the carrier holds, not the carrier's side.

Addresses: `0x4ac1c0`, `0x4ac230`, `0x4ac269`, `0x4ac282`, `0x4ac2ad`, `0x4ac2b5`, `0x4ac2c5`, `0x4ac2e4`, `0x4ac35e`, `0x4aa670`, `0x4aa682`, `0x4aa6a3`, `0x4aa6ac`, `0x5efe00`, `0x4a7870`, `0x8457d4`
Confidence: verified
Proof: one
Test: none

`0x4aa670` returns the token from the player's faction side (`0x5efe00`): side 0 gives the first string of the table at `0x8457d4` and side 1 the second; a negative side or a zero GUID gives null. An index of 0, a negative index or an index of 2 or more selects nothing and pushes zeros and nil.

## The icon scale

`GetBattlefieldMapIconScale()` pushes the float at `+0xa4` of the `Map.dbc` row of the active queue slot's map id, or of row 0 when no slot is active, and the number 1.0 when the id is out of range or its row is missing.

Addresses: `0x4ac3d0`, `0x4ac3d5`, `0x4ac3dc`, `0x4ac3e6`, `0x4ac3fb`, `0x4ac412`, `0x4ac416`, `0xc0daa8`, `0xc0daac`
Confidence: verified
Proof: one
Test: none

The neighbouring `CanJoinBattlefieldAsGroup` reads the field before it (`battlefield`).
