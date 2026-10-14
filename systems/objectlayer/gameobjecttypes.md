# objectlayer: game object types

The per-type handlers of [gameobjects](gameobjects.md): what each handler slot does, the animation
state machine family A handlers run on the model, the display's sounds and art kits, and the types
with their own slots: doors, quest givers, chairs, fishing nodes, text, mailboxes, lottery kiosks
and meeting stones. It also lists the class template and inline code GameObject_C.cpp
instantiates for other classes.

Every handler vtable starts from the base `0x80b100`. Family A tables add slots 34 and 35 and
replace 2, 8 to 16, 23, 26, 27 and 30 with the animation bodies; the map object's and the
transports' replace the slots of `transports`. A handler's object is `[handler+4]`; family A keeps
its animation state at `+0x10`, the state it played at `+0x14` and its looping sound at `+0x18`.

Roots: `0x5f3cb0`, `0x5f3e20`, `0x5f4120`, `0x5f8c30`, `0x5f8c40`, `0x5f1ed0`, `0x5f1ee0`, `0x5f1f10`, `0x5f1f30`, `0x5f1f40`, `0x5f1f60`, `0x5f1630`, `0x5f16f0`

## Handler slots 0 to 11

The base vtable's bodies and the object code that calls each slot. The other types' bodies are in
the entries below.

| slot | base | called for |
|---|---|---|
| 0 | `0x5f9f20` | deleting destructor |
| 1 | `0x5f9d90`, 1 | whether the object collides; 0 for map objects and transports |
| 2 | `0x5f9da0`, 1 | object slot 26, fade in on appear; family A: state not 0 |
| 3 | `0x5f9db0` | object slot 21, mouseover; jumps through slot 5 |
| 4 | `0x5f3070` | the cursor |
| 5 | `0x5f2f80` | interactable |
| 6 | `0x5f3130` | usable, with the error and range |
| 7 | `0x5f33e0` | use |
| 8 | `0x5f9dc0` | `GAMEOBJECT_FLAGS` changed, the changed bits |
| 9 | `0x5f9dd0` | `GAMEOBJECT_ANIMPROGRESS` changed |
| 10 | `0x5f3660` | the state changed, old and new |
| 11 | `0x5f9de0` | object slot 15, each frame's ground pass |

Addresses: `0x80b100`, `0x5f9f20`, `0x5f9d90`, `0x5f9da0`, `0x5f9db0`, `0x5f3070`, `0x5f2f80`, `0x5f3130`, `0x5f33e0`, `0x5f9dc0`, `0x5f9dd0`, `0x5f3660`, `0x5f9de0`, `0x5f37e0`, `0x5f85e0`, `0x5f8620`, `0x5f7c20`, `0x5f7a6d`
Confidence: verified
Proof: one
Test: none

The bases are the dwords of `0x80b100`. Slot 1 is called by the model-loaded pass (`0x5f7a6d`);
the map object's and transports' slot 1 is `0x5f4890`. Object slot 26 (`0x5f85e0`) returns slot
2's byte; `0x5f37e0` returns `[+0x10] != 0`. Object slot 21 (`0x5f8620`) and slot 15
(`0x5f7c20`) call slots 3 and 11. The watchers call 8, 9 and 10.

## Handler slots 12 to 23

| slot | base | called for |
|---|---|---|
| 12 | `0x5f3670` | a model event: id, data and two more |
| 13 | `0x5f9df0` | the model's done callback with its flag set |
| 14 | `0x5f9e00` | the done callback with the flag clear |
| 15 | `0x5f9e10` | an animation event |
| 16 | `0x5f3680`, `""` | the state's name |
| 17 | `0x5f3690` | object slot 5, the position: `GAMEOBJECT_POS_X` to `_Z` |
| 18 | `0x5f36c0` | object slot 6, the facing: `GAMEOBJECT_FACING` |
| 19 | `0x5f9e20` | a mover boarding |
| 20 | `0x5f9e30`, (0, 0, 0) | object slot 31, the velocity |
| 21 | `0x5f9e50`, 0 | object slot 33, a point inside |
| 22 | `0x5f9e60`, 0 | the world entity |
| 23 | `0x5f9e70` | the creation hook's third argument |

Addresses: `0x5f3670`, `0x5f9df0`, `0x5f9e00`, `0x5f9e10`, `0x5f3680`, `0x5f3690`, `0x5f36c0`, `0x5f9e20`, `0x5f9e30`, `0x5f9e50`, `0x5f9e60`, `0x5f9e70`, `0x5f3710`, `0x5f7da0`, `0x5f7df0`, `0x5f9f50`, `0x5f9fb0`, `0x5f9fd0`, `0x5fa030`, `0x5f8140`
Confidence: verified
Proof: one
Test: none

`0x5f7da0` passes the model's event to slot 12; `0x5f7df0` picks 13 or 14 by its second argument.
Object slots 5 and 6 (`0x5f9f50`, `0x5f9fb0`) read the fields when the object has no handler.
Slot 16 is called only by `0x5f8140`, which nothing reaches; the static handler's is `0x5f3710`,
`"Unknown Object Type"`. The creation hook passes its third argument to 23.

## Handler slots 24 to 35

| slot | base | called for |
|---|---|---|
| 24 | `0x5f9e80` | the record arrived |
| 25 | `0x5f9e90` | object slot 2, reactivation |
| 26 | `0x5f9ea0` | object slot 1, deactivation |
| 27 | `0x5f9eb0` | object slot 3, after reactivation |
| 28 | `0x5f9ec0`, 0 | object slot 32, the path |
| 29 | `0x5f9ed0` | the transport list's tick, now and elapsed |
| 30 | `0x5f9ee0` | the model loaded |
| 31 | `0x5f9ef0` | the interface opening the object's frame |
| 32 | `0x5f9f00` | the interface closing it |
| 33 | `0x5f9f10` | the tooltip's extra lines |
| 34 | family A `0x5f3b50` | set the animation state |
| 35 | family A `0x5f3780` | move the looping sound to the position |

Addresses: `0x5f9e80`, `0x5f9e90`, `0x5f9ea0`, `0x5f9eb0`, `0x5f9ec0`, `0x5f9ed0`, `0x5f9ee0`, `0x5f9ef0`, `0x5f9f00`, `0x5f9f10`, `0x5f3b50`, `0x5f3780`, `0x5f8c30`, `0x5f8c40`, `0x5f9ff0`, `0x630970`
Confidence: verified
Proof: one
Test: none

The record arrival (`0x5f7fb0`) calls 24; object slots 1 to 3 call 26, 25 and 27 only once the
record is there. `UpdateListedTransports` (`0x630970`) calls 29. Object slot 32 (`0x5f9ff0`) calls
28 or returns 0 without a handler. `0x5f8c30` and `0x5f8c40` jump through 31 and 32 for the
interface. `0x5f3780` passes the slot 17 position to 0x462000 with the sound at `+0x18`.

## Animation states

A family A handler plays one of 13 animation states. A change of `GAMEOBJECT_STATE` picks a moving
state and its end: to 0, state 2 then 3; to 1, 4 then 1; to 2, 5 then 6. It goes straight to the
end state when `GAMEOBJECT_ANIMPROGRESS` is 100 and the old state is not 1 (for 0 and 2) or 0 (for
1); to 1 from 2 it then plays 7. When a played state ends, 2 goes to 3, 4 and 7 to 1, 5 to 6; 0
and the custom states 8 to 11 return to the object's state; 1, 3 and 6 play again. A door
collides in state 1, so `GAMEOBJECT_STATE` 1 is closed.

Addresses: `0x5f3cb0`, `0x5f3ce3`, `0x5f3d1a`, `0x5f3d66`, `0x5f4120`, `0x5f41e4`, `0x5f3c30`, `0x5f40c0`, `0x8607e4`, `0x860818`
Confidence: verified
Proof: one
Test: none

`0x5f3cb0(old, new)` returns when `0x614ef0` is set, stops the looping sound (`0x5f40c0`) and calls
slot 34: new 0 (`0x5f3d66`) with old 1 or progress below 100 gives 2, else 3; new 1 (`0x5f3d1a`)
with old 0 or progress below 100 gives 4, else old 2 gives 7, else 1; new 2 (`0x5f3ce3`) with old
1 or progress below 100 gives 5, else 6. `0x5f4120` switches on `+0x10` through `0x5f41e4`: 2 to
slot 34 with 3, 4 and 7 with 1, 5 with 6; 1, 3, 6 replay; 0 and 8 to 11 call slot 10 with
`(+0x27c, +0x27c)` unless a release is pending. `0x5f3c30`, after reactivation, picks 3 or 2, 1 or
4, 6 or 5 from the state and progress. The states play the animations 0x91, 0x93, 0x94, 0x95,
0x92, 0x96, 0x97, 0x98, 0x99 to 0x9c and 0x9d (`0x8607e4`); `0x860818` names states 0
to 8, Closed to Despawn; states 9 to 12 read the null dwords at 0x86083c to 0x86084b.

## Playing a state

Setting a new state plays its animation unless the model's key bone already plays it. A missing
open or close animation falls back to a neighbour, some frozen at speed 0. Playing starts at the
saved progress when `GAMEOBJECT_ANIMPROGRESS` is below 100, then stores 100. States 2, 4, 5 and 7
pause when `GAMEOBJECT_FLAGS` has 0x80. Every state but 1, 3 and 6 holds the object until its end.
Reversing a running open or close at full progress, 2 and 4 or 5 and 7, stores the complement of
the elapsed percentage, so the reverse starts where the forward stopped.

Addresses: `0x5f3930`, `0x5f3941`, `0x5f397d`, `0x5f3b40`, `0x5f39fa`, `0x5f3a6b`, `0x5f3ac9`, `0x5f3af8`, `0x5f3b27`, `0x5f3b50`, `0x5f3bcb`, `0x5f3bfd`, `0x5f40e0`, `0x5f3880`, `0x5f8c10`, `0x4683e0`, `0x468410`
Confidence: verified
Proof: one
Test: none

`0x5f3930(model)`: for 0x92 to 0x95 missing (`0x5f3b40`), 0x92 keeps itself when the model has
0x94, else 0x93; 0x93 keeps itself with 0x92, else 0x94 at speed 0, else 0; 0x94 keeps itself
with 0x92, else 0x96 if present, else 0x95; 0x95 keeps itself with 0x94, else 0x92 at speed 0,
else 0x97. The start time is `progress · duration / 100`, unsigned (`0x5f3ac9`). `PlayAnimation(-1,
anim, -1, time, speed, state != 0, 1)`; `0x5f3b27` calls `ClntObjMgrHoldObject`, and slots 13 and
14 call `ClntObjMgrUnholdObject` for the played state. `0x5f3bfd`: `(scene time·100 − start·100) /
(end − start)`, unsigned; below 100 it writes `100 − p` through `0x5f8c10`. Slot 8 (`0x5f3880`)
re-pauses on a 0x80 change.

## Progress, frame and reload slots

Three more family A slots drive the state from outside. A change of `GAMEOBJECT_ANIMPROGRESS`
(slot 9) replays the state on the loaded model, except during a reactivation. Each frame (slot
11), an object whose release is pending and whose loaded model is not animating ends its state
(slot 13) once the key bone's state time has passed, or at once when the model is paused. After reactivation (slot 27) a loaded model
picks the state from the object's state and progress again.

Addresses: `0x5f38f0`, `0x5f3da0`, `0x5f3840`, `0x5f3930`, `0x5f3c30`, `0x614ef0`, `0x614eb0`, `0x614f40`
Confidence: verified
Proof: one
Test: none

`0x5f38f0` calls `0x5f3930(model)` when `0x614ef0` (`+0xe4` bit 0x2) is clear and `0x614eb0` finds
the model at `+0xdc`, else `+0xd8`, loaded. `0x5f3da0` needs `0x614f40` (bit 0x10), `CM2Model::IsLoaded` and not `IsAnimated`;
it reads `GetKeyBoneAnimState(-1)` and calls slot 13 when `GetPauseTime` is non-zero or now less
the state's `+0x14` is not negative (`0x5f3e0a js`). `0x5f3840` jumps to `0x5f3c30` once
`IsLoaded`. The tables hold them at slots 9, 11 and 27, as in `0x80b2a0`.

## Animation events

Animation events come from messages and the interface: a custom animation 0 to 3 plays states 8 to
11, the despawn message state 12, and the creation hook's flag state 0. An event waits
at `+0x280` until the model loads. A family A handler plays the state only if the model has its
animation; state 0 also starts the object's fade.

Addresses: `0x5f4220`, `0x5f424a`, `0x5f426d`, `0x5f3820`, `0x80b0e0`, `0x5f8c50`, `0x614f80`
Confidence: verified
Proof: one
Test: none

`0x80b0e0` maps events 1 to 6 to states 0, 8, 9, 10, 11, 12. `0x5f3820(flag)` sends event 1 when
its byte is set. `0x5f4220(state)` tests `HasAnimation(0x8607e4[state])`, calls slot 34 and, for
state 0, `0x614f80` with the object's slot 27 alpha.

## Display sounds and shakes

A family A handler answers the model's events: `$GO0` to `$GO5` and `$GC0` to `$GC3` play sound
slots 0 to 9 of the `GameObjectDisplayInfo` row, a looping sound replacing the running one;
`$SND` and `$DSO` play the event's sound; `$SHK` adds the three camera shakes of a
`SpellEffectCameraShakes` row; `$DSL` starts or moves a looping sound.

Addresses: `0x5f3e20`, `0x5f3e44`, `0x5f3f5b`, `0x5f3fa6`, `0x5f3fe5`, `0x5f4010`, `0x5f404a`, `0x5f4080`, `0x5f40c0`, `0x5f8930`, `0x5f8c50`, `0xc0dce4`, `0xc0d80c`, `0x458870`, `0x511d40`
Confidence: verified
Proof: one
Test: none

`0x5f3e20(id, data, a, b)` compares the FourCC with constants such as `0x304f4724` (`$GO0`) and
`0x30434724` (`$GC0`). `0x5f4010(slot, a, b)` reads `[row + 8 + 4·slot]` of the display row and
0x458830's looping flag: a one-shot goes to `PlaySoundAtArgs(id, a, −1, 1.0)`; a looping one, when
0x462120 says it is not already playing, stops the old (0x461f80) and starts it with 0x461d80.
`$SHK` (`0x5f3fa6`) passes each of the row's three shakes and the data to `CameraAddShake`; `$DSL`
calls 0x461d80 or 0x462000. `0x5f4010`, which reads
the row's sound columns, is called only from `0x5f3e20`: the custom animation message
(`0x5f8930`, `0x5f8c50`) plays no sound itself and a sound follows only when the armed clip carries a
`$GC` event.

## Art kits

`GAMEOBJECT_ARTKIT` names a `GameObjectArtKit` row: its three textures replace the model's
textures 11 to 13 and its four models attach at points 0 to 3. The model's four attachments go
first; an art kit of 0 or without a row leaves none.

Addresses: `0x5f7b00`, `0x5f7b30`, `0x5f7b80`, `0x5f7bc5`, `0xc0dcf8`, `0x7130a0`, `0x710ec0`, `0x712f70`
Confidence: verified
Proof: one
Test: none

`0x5f7b00(model, kit)` reads `0xc0dcf8`'s range, calls `CM2Model::DetachAt(i)` for 0 to 3
(`0x5f7b30`), then for each non-empty texture at row `+0x4` to `+0xc` `TextureCreate` and
`SetReplaceableTexture(i + 11, texture)`, and for each path at `+0x10` to `+0x1c`
`CreateModelFromPath` in the model's scene and `Attach(model, i)`.

## Doors and buttons

A door is usable only while it is not moving: with `autoClose` set, a door that starts open must
not be in state 1, one that starts closed not in state 3; otherwise `ERR_USE_OBJECT_MOVING`. A
door's state change stores whether the state is 1 as its collision. A locked door shows
`ERR_DOOR_LOCKED`, a locked button `ERR_BUTTON_LOCKED`.

Addresses: `0x5f4360`, `0x5f42f0`, `0x5f43a0`, `0x5f4290`, `0x5f43d0`
Confidence: verified
Proof: one
Test: none

`0x5f42f0` reads columns 3 (`autoClose`) and 1 (`startOpen`): no `autoClose` is 1; else
`startOpen ? state != 1 : state != 3`. `0x5f4360(error, range)` gives 0xe3 or runs `0x5f3130`.
`0x5f43a0(state)` runs slot 34 and stores `[+0x10] == 1` at the object's `+0x284`.

## Quest givers, text and mailboxes

When the interface opens a quest giver's frame, one without `customAnim` plays its open state 2
unless the model already plays that animation, and closing plays 4 from 2 or 3; one with
`customAnim` sends its custom animation event. Text objects play 2 and 4 the same way; using one
passes its GUID to the interface, and the cursor is `Inspect`. Mailboxes and lottery kiosks pass
the GUID to another interface function, show `Mail`, and play custom animation 0 on opening.

Addresses: `0x5f4600`, `0x5f4720`, `0x5f5a30`, `0x5f5a40`, `0x5f5890`, `0x5f58c0`, `0x5f59d0`, `0x5f6820`, `0x5f6840`, `0x5f6870`, `0x5f6e10`, `0x5f6e30`, `0x5f6e60`, `0x5f5900`, `0x5f5930`, `0x5f5950`
Confidence: verified
Proof: one
Test: none

`0x5f4600` uses the open animation `[0x8607ec]`, or the model's fallback sequence for it, against
`GetKeyBoneAnimId`; with column 0x16 it calls `0x5f8c50(column + 1)`. Text: slot 4 returns 7
(`Inspect`) or 27, slot 7 calls 0x4e32e0(&guid, 0), slot 24 refreshes with 1 when the GUID is
0xbc3fa0's. Mailbox and lottery slot 7 call 0x4acd10(guid), slot 4 returns 15 (`Mail`) or 35, slot
31 sends event 2. The object's slot 29 and `0x5f5930`, `0x5f5950` read `pageID`, `language` and
`pageMaterial`.

## Chairs

A chair is usable within 3 yards of one of its seats, which lie 1 yard apart along the chair's
local y axis, centred on it, up to `chairslots` of them; past every seat it refuses with
`ERR_USE_TOO_FAR`, and a chair without seats is never usable. The seats are placed when the record
arrives, through the object's world matrix.

Addresses: `0x5f5670`, `0x5f56c4`, `0x5f571a`, `0x5f5760`, `0x47cf60`, `0xc4d808`, `0x80b0a0`, `0x7bca80`
Confidence: verified
Proof: one
Test: `objectlayer::diff_interact_range`, `objectlayer::ot_transport_segment_offset0_difftest`, `objectlayer::ot_transport_segment_local_y_difftest`

`0x5f5670` runs `0x5f3130`, sets the range to 3.0 (`0x80b0a0`), and from the last seat down
compares `(dz² + dx²) + dy²` with 9.0 at `0xc4d808` (`0x5f571a`): below succeeds. `0x5f5760` calls
`0x47cf60(matrix, count, +0x10)`, which starts at `y = −((count − 1.0)·0.5)`, a float, and for each
seat stores `TransformPoint(matrix, (0, y, 0))`, then `y += 1.0`. The handler keeps five seats. Slices: diff_interact_range `0x5f56fa`-`0x5f5724`; ot_transport_segment_offset0_difftest `0x47cf66`-`0x47cfa2`; ot_transport_segment_local_y_difftest `0x47cf66`-`0x47cfe3`.

## Fishing nodes and meeting stones

A fishing node is interactable only for the player channelling it, and its use never walks there.
A meeting stone is interactable unless its area is the one in 0xb72038. Using it refuses with
`ERR_MEETING_STONE_MUST_BE_LEADER`, `ERR_MEETING_STONE_GROUP_FULL`,
`ERR_MEETING_STONE_INVALID_LEVEL` (outside its `minLevel` to `maxLevel`) or
`ERR_MEETING_STONE_NO_RAID_GROUP`, and otherwise passes the GUID to the interface. Its tooltip adds
the area's name and `MEETINGSTONE_LEVEL`.

Addresses: `0x5f6710`, `0x5f6990`, `0x5f69d0`, `0x5f6a2f`, `0x5f6a4f`, `0x5f6ab4`, `0x5f6ad3`, `0x5f6b10`, `0x5f86fd`
Confidence: verified
Proof: one
Test: none

`0x5f6710` compares the player's `UNIT_FIELD_CHANNEL_OBJECT` with the GUID before `0x5f2f80`;
`0x5f86fd` skips the walk for type 0x11. `0x5f6990` compares column `areaID` with 0xb72038.
`0x5f69d0`: with a party GUID at 0xbc6f48, a leader 0xbc75f8 not the player gives 0x1b1 and
0x4e86d0() of 4 or more 0x1ae; the level outside `minLevel` to `maxLevel` 0x1b0; 0xb713e0
positive 0x1b2; then 0x4c9ff0(guid). `0x5f6b10` reads the area row's `+0x2c + 4·locale` and
formats `MEETINGSTONE_LEVEL` with the levels for 0x530380.

## Constant mouseover and interaction

Several types replace slots 3 and 5 with constants or a data column. Spell foci, duel flags,
fishing holes and aura generators always pass mouseover but are never interactable; generic objects
and capture points pass mouseover by their `highlight` column; map objects and transports pass
neither; auction houses and the static handler pass mouseover through slot 5, which refuses.

Addresses: `0x5f57e0`, `0x5f57f0`, `0x5f65f0`, `0x5f6600`, `0x5f6660`, `0x5f6670`, `0x5f6ea0`, `0x5f6eb0`, `0x5f4830`, `0x5f6d80`, `0x5f6d40`, `0x5f47f0`, `0x5f48a0`, `0x5f48b0`, `0x5f5c70`, `0x5f68a0`, `0x5f36f0`, `0x5f3700`
Confidence: verified
Proof: one
Test: none

The constants are `mov al,1; ret` and `xor al,al; ret`; the static handler's slot 6 is `0x5f3700`,
0. `0x5f4830` and `0x5f6d80` read column 0x12 and return it as a boolean.

## Shared template and inline code

GameObject_C.cpp also holds code other classes use: the bodies the unit and player vtables take
from the class header, the player's trade skill and proficiency hash tables, three growable arrays
of the player, and small matrix and list helpers.

Addresses: `0x5f1ea0`, `0x5f1eb0`, `0x5f1ec0`, `0x5f1ed0`, `0x5f1ee0`, `0x5f1f10`, `0x5f1f30`, `0x5f1f40`, `0x5f1f60`, `0x5f1f70`, `0x5f1f80`, `0x5f1630`, `0x5f16f0`, `0x5f18f0`, `0x5f2430`, `0x5f24e0`, `0x5f2570`, `0x5f8d20`, `0x5f8d60`, `0x5f17b0`, `0x5f17f0`, `0x5f1a70`, `0x5f1aa0`, `0x5f1b30`, `0x5f1b40`, `0x5f1bd0`, `0x5f1cd0`, `0x5f1d00`, `0x5f1d90`, `0x5f1e00`, `0x5f1e10`, `0x5f1f90`, `0x5f1fd0`, `0x5f2010`, `0x5f2040`, `0x5f2060`, `0x5f20b0`, `0x5f22a0`, `0x5f22c0`, `0x5f22f0`, `0x5f23b0`, `0x5f2400`, `0x5f2410`, `0x5f24c0`, `0x5f2610`, `0x5f2630`, `0x5f2660`, `0x5f2690`, `0x5f2880`, `0x5f2930`, `0x5f2940`, `0x5f2a80`, `0x5f2a90`, `0x5f2bd0`, `0x5f2d10`, `0x5f2e20`
Confidence: verified
Proof: one
Test: none

Unit slots: 28 `0x5f1ed0` is `GetUnitName(0)`; 26 `0x5f1ee0` returns 0 for `UNIT_FIELD_FLAGS` bit
0x2 or byte `+0x213` bit 0x2; 5, 6 and 8 pass the mover through `TransportToWorld`,
`TransportFacingToWorld` and `GetWorldUpVector`; 9 returns `+0x9e0`. The tables' destructors
store `0x80b044` and `0x80b034`; `0x5f18f0(hash, a, b)` inserts. The arrays reallocate their
`InitialSpellStruct`, `GUILDUPDATETIMESTAMP` and `ITEMSWAP` blocks. `0x5f8d20` stores nine floats,
`0x5f8d60` an identity 4×4. `0x5f17f0` and `0x5f1bd0` empty the two tables, unlinking each node or,
with a zero argument, deleting it through the table's slot 0; `0x5f20b0` doubles a table's
buckets, up to 0x2000, when a lookup walks more than 13 nodes. The other bodies listed match the
core's list, bucket and array helpers instruction for instruction.
