# objectlayer: transports

The game objects that carry movers: map objects (type 14), the moving transports (type 15, the
ships and zeppelins) and the transports (type 11, the elevators and lifts). A moving transport
builds its path from `TaxiPathNode` once its record arrives, splits it into legs by map, times
each leg with a constant acceleration and steps along it on the transport list's tick. A
transport follows its `TransportAnimation` rows. Both move the movers aboard. The handlers'
slot numbers are those of [gameobjecttypes](gameobjecttypes.md); the transport list and the
carried mover's sweep are `collision`.

The code is GameObject_C.cpp's, between the type handlers and the field watchers; the ship path
helpers sit after the watchers, from `0x5f90e0` to `0x5f9610`, and `0x5f99e0`.

Roots: `0x5f4cc0`, `0x5f50a0`, `0x5f5f10`, `0x5f4f80`, `0x5f4fb0`, `0x5f5500`, `0x5f5d70`, `0x5f5da0`, `0x5f5dd0`, `0x5f6490`, `0x5f4970`

## Map objects

A type 14 handler holds a world entity at `+0x10`. Its record arrival and reactivation create
it: `CWorld::CreateMapEntity` with the model path of the `GameObjectDisplayInfo` row of
`GAMEOBJECT_DISPLAYID`, the object's position and facing, and its GUID for types 11 and 15, 0
for others. Without the row it prints `NOOBJECTFILENAME|%d|%d|Game` with the display and entry
ids and passes no path. Deactivation and the destructor destroy the entity. The entity's done
callback chains the transports' animations: accelerating (0xa2) plays cruising (0xa3),
decelerating (0xa4) plays standing (0); after standing or cruising nothing plays, and any other
animation replays itself.

Addresses: `0x5f4860`, `0x5f4970`, `0x5f8030`, `0x5f8090`, `0x5f4950`, `0x5f4910`, `0x5f48d0`, `0x5f48c0`, `0x5f4900`, `0x5f4930`, `0x5f4940`, `0x5f4890`, `0x5f48a0`, `0x5f48b0`, `0x5f49b0`
Confidence: verified
Proof: one
Test: none

`0x5f4970` tests `[fields+0x3c]` (`GAMEOBJECT_TYPE_ID`) against 0xb and 0xf; `0x5f8030` pushes
the GUID halves, 0, 0 and the object slot 6 facing, reads `GAMEOBJECT_POS_X` to `_Z` and calls
`0x6707c0` with `ecx` = `0x5f8090`'s path, the row's `+4`. Slots 24, 27 jump to `0x5f4970`, 26 to
`0x5f4950`; 22 returns `+0x10`; 1, 3 and 5 return 0. `0x5f49b0` returns at once on a non-zero
second argument, else maps the animation through the bytes `0x5f4a04` (0xa5 entries) to four cases
and calls `PlayAnimation(-1, anim, -1, 0, 1.0, 1, 1)`.

## The moving transport

A type 15 handler, 0x60 bytes, extends the map object's: a passenger list at `+0x14` (link offset
8), the ship path at `+0x20`, the position `+0x44` and facing `+0x50`, the animation it played
`+0x54` (−1 for none), the leg it last showed `+0x58` and an entity-ready byte `+0x5c`. The
constructor copies the position and facing from the fields. Slots 17, 18 and 28 return the
position, the facing and the path. The destructor removes it from the transport list when it has
an entity, frees each leg's stops and spline and the legs, and unlinks the passengers.

| path | holds |
|---|---|
| `+0x00` | the `TaxiPathNode` store |
| `+0x04` | `taxiPathID` |
| `+0x0c`, `+0x10` | the leg count and the legs, 0x4c each |
| `+0x18` | `moveSpeed` |
| `+0x1c` | the period, ms |
| `+0x20` | `accelRate` |

| leg | holds |
|---|---|
| `+0x00` | the map |
| `+0x04` | the spline; `+0x08` its length |
| `+0x30`, `+0x34`, `+0x38` | start, travel time and end, ms |
| `+0x40`, `+0x44` | the stop count and the stops, 12 bytes: time, distance, wait |

Addresses: `0x5f4ab0`, `0x5f4b40`, `0x5f4b70`, `0x5f4b80`, `0x5f4b90`, `0x5f4bc0`, `0x5f9380`, `0x5f93a0`, `0x5f9ad0`, `0x5f9c10`, `0x5f9d10`, `0x5f94d0`, `0x80b798`
Confidence: verified
Proof: one
Test: none

`0x5f4ab0` runs `0x5f4860`, sets `+0x14` to 8 and the list head, −1 at `+0x24` and `+0x54`, and
copies `[fields+0x24]` to `+0x44` and `[fields+0x30]` to `+0x50`. `0x5f93a0` resizes the legs,
storing the spline vtable at `leg+4` and mode 1 at `leg+0x2c`. `0x5f4bc0` frees through
`0x5f9380(leg+0x3c)` and `0x469b30(leg+4)`, then the legs with the `ShipPath<…>::Leg` tag. `0x5f9ad0`
reallocates the legs, copying each leg's spline arrays (`0x5f9c10`) and stops (`0x5f9d10`);
`0x5f94d0` zeros a leg's four dwords.

## Passengers

Boarding (slot 19) unlinks the mover from its list and appends it to the passengers; while the
entity is missing or not ready, the mover gets flag 0x8000000. Deactivation (slot 26) takes each
passenger off: the camera following it loses its transport, the active mover queues a fall start
and any other starts falling, and its transport is cleared. Then the object leaves the transport
list and the entity is destroyed. A point is inside (slot 21) when there is no entity, else by
`IsPointInEntity`. The transport type does the same; its boarding tests the object through
0x614eb0 instead of the entity.

Addresses: `0x5f5500`, `0x5f4fb0`, `0x5f55c0`, `0x5f6490`, `0x5f5dd0`, `0x5f6540`, `0x7c6500`, `0x511da0`, `0x617ac0`, `0x617b00`, `0x617100`, `0x630920`
Confidence: verified
Proof: one
Test: none

`0x5f5500` unlinks `[mover+8]`, links it before the head `+0x18` and calls `0x7c6500` when
`IsEntityReady` fails. `0x5f4fb0` compares `[camera+0x88]` with `[mover+0x30]` before
`SetCameraTransport(0, 0)`, compares the unit's GUID with `0xc4da98` to pick `0x617ac0(now)` or
`0x617b00`, then calls `0x617100(0, 0)`. `0x5f5dd0` is the same body, `0x5f6490` the same
boarding with `0x614eb0`.

## The ship path

On the record's arrival the handler creates its entity and reads `accelRate`, `moveSpeed` and
`taxiPathID`. The nodes are the run of `TaxiPathNode` rows from the first with that
path. A leg starts at the first node, at a node on another map and after a node with flag 0x1. A
node with flag 0x2, not a leg's first, is a stop of its delay × 1000 ms. Each leg is timed, then
laid end to end: its start is the total so far, its stops' times gain that start, and the total
gains its travel time and the last leg's stop waits (Path faults). The total is the
period. The object joins the transport list and ticks once.

Addresses: `0x5f4cc0`, `0x5f4f80`, `0x5f90e0`, `0x5f94f0`, `0x5f9530`, `0x5f9570`, `0x5f95d0`, `0x5f9610`, `0x5f99e0`, `0x6308b0`
Confidence: verified
Proof: one
Test: none

The columns are `0x621b00` ids 0x2b, 0x24 and 0x23, the first two made floats through `fild` of a
zero-extended qword. The rows of `0xc0d6a4` are 0x24 bytes: path `+4`, map `+0xc`, position `+0x10`, flags `+0x1c`, delay `+0x20`.
`0x5f4dda`–`0x5f4de9` start a leg on no leg, a map change or the previous node's bit 0x1; `0x5f4e37`
needs a non-zero point count for a stop. The node and stop arrays live on the stack, 0x3000 and
0x2000 bytes, 1024 entries each. Slot 27 (`0x5f4f80`) recreates the entity, clears `+0x5c`, sets
`+0x54` to −1, rejoins the transport list (`0x5f4f97`) and ticks.

## Path faults

The path build has three faults. The install's `TaxiPathNode.dbc` (patch.MPQ, 288 paths) reaches
none: no leg ends on a stop, and its eight two-leg paths have equal waits in both legs (60 and
60 seconds, or none).

| fault | effect |
|---|---|
| Every leg's end adds W, the waits of the last leg's stops, not W_k, its own | W > W_k: the ship sits at the leg's end playing 0xa3 for W − W_k ms; W < W_k: the next leg cuts the last W_k − W ms of this one |
| A stop on a leg's last node: the stop table has n + 1 entries but the timing loop stops at it | entry n stays unwritten, gains the leg start and ends the last span; the stop's wait counts in W but not in the leg's entries; during it the ship clamps to the leg end playing 0xa2 or 0xa3 |
| More than 1024 nodes in one leg | the appends overrun the stack arrays |

Addresses: `0x5f4e32`, `0x5f4eeb`, `0x5f4f01`, `0x5f4f10`, `0x5f916e`, `0x5f91a1`, `0x5f519e`, `0x5f9530`, `0x5f9610`
Confidence: verified
Proof: one
Test: none

The stop array is emptied only when a new leg starts (`0x5f4e32`), and the post-loop sum at
`0x5f4eeb`–`0x5f4f01` reads it for every leg. A leg's last entry holds its travel plus its own
waits; past it and before the leg's end the tick runs span n with decelerates 0, whose distance
passes the leg length, so the fraction clamps to 1. `0x5f916e` sizes the stops to n + 1;
`0x5f91a1` leaves the loop at a stop on the last node, and the end span writes the entry at that
index, never entry n, which keeps what the allocation left. `0x5f4f10` adds the leg start to each
of the n + 1 entries; `0x5f519e` reads entry n's time as the last span's end, and that span runs
with accelerates 1 and decelerates 0. The appends `0x5f9530` and `0x5f9610` check no bound. The
scan follows the client: the run of rows from the path's first, split at a map change and after
flag 0x1, a stop being flag 0x2 past a leg's first node.

## Leg timing

A leg's spline runs through its nodes. Each span between stops, and the last to the leg's end,
takes the time of a constant acceleration a to the speed v; a span with a stop at both ends
speeds up and slows down, one at a leg's edge does only one. With ta = float((1/a)·v) and
da = (v·0.5)·ta, each test compares da at 53 bits and each formula uses float(da):

| span | time, s |
|---|---|
| no stops | seg / v |
| first or last | da < seg: (seg − da) / v + ta; else sqrt(seg·(1/a)·2) |
| between stops | da < seg/2: (seg − 2·da) / v + 2·ta; else 2·sqrt(seg·(1/a)) |

Each span rounds to ms on its own: t·1000, ±0.5 away from zero, truncated. Stop i holds the
leg's travel time to it plus the waits before it, its distance and its wait; the last entry holds
the whole leg and wait 0.

Addresses: `0x5f9120`
Confidence: verified
Proof: one
Test: `objectlayer::leg_time_difftest`, `objectlayer::leg_time_special_values_difftest`

`0x5f9120(points, stops, leg)` calls `SetSplinePath`, sizes the stops to n + 1 and zeros `+0x34`.
Stop i's distance is 0x453300(spline, node), a cumulative length; seg is it less the last. A stop
at the leg's last node ends the loop (`0x5f91a1`). `0x5f91db` picks the first span's form,
`0x5f9205` the middle form; the end runs from `0x5f92b8` with seg = length − last. `fst` at
`0x5f91d8` and `0x5f92f0` stores float(da) and keeps the 53-bit da on the stack, which the compares
at `0x5f91dd`, `0x5f9205` (against seg·0.5) and `0x5f92f3` read; the formulas load the stored
float. Branches test `ah & 5` and `jp`, so a NaN takes the square root. The x87 runs at 53 bits. Slices: leg_time_difftest, leg_time_special_values_difftest `0x5f91af`-`0x5f924f` `0x5f92b8`-`0x5f9335`.

## Speed curve

`0x5f8dc0(t, T, state, accelerates, decelerates)` gives the distance covered t seconds into a span
of T seconds and its animation: 0xa2 speeding up, 0xa3 cruising, 0xa4 slowing down.

| span | distance |
|---|---|
| neither | t·v, 0xa3 |
| speeds up | t ≤ r: ((t·a)·0.5)·t, 0xa2; else (t − ta)·v + (v·0.5)·ta, 0xa3 |
| slows down | t ≤ T − r: t·v, 0xa3; else ((2·(r·a) − d·a)·0.5)·d + (T − r)·v, 0xa4 |
| both | t > T − r: ((2·(r·a) − d·a)·0.5)·d + (T − 2r)·v + ((r·a)·0.5)·r, 0xa4; t ≤ r: as speeding up; else as cruising |

ta = v / a; r = min(T, ta), and min(T·0.5, ta) for both; d = t − (T − r).

Addresses: `0x5f8dc0`
Confidence: verified
Proof: one
Test: `objectlayer::transport_speed_easing_difftest`, `objectlayer::transport_speed_easing_special_values_difftest`

v is `[ecx+0x18]`, a `[ecx+0x20]`. Both ends store ta and T − r as floats (`0x5f8ddf`, `0x5f8dff`);
the one-ended forms keep them at 53 bits. A NaN comparison picks ta for r, and `test ah,0x41` /
`jne` sends t ≤ x and unordered to the lower form of each `t` test. Its time form and
`0x5f9120`'s agree only up to rounding: the builder's ta is (1/a)·v.

## Moving the ship

Each tick takes t = (now + `+0x278`) mod the period (1 when 0) and the first leg ending after t.
Within a stop's wait the ship sits at the stop, animation 0; past it the span starts there. Else
the speed curve gives the distance into the span, T and t in seconds. The fraction of the leg,
clamped to [0, 1] with NaN as 1, gives the spline frame. The heading is the frame's first row's x
and y negated and normalised; the facing is its atan2. A leg on another map ends the tick. Else
the position, facing and entity move, and a changed animation plays once the entity is ready.

Addresses: `0x5f50a0`, `0x5f8cb0`, `0x5f8d60`, `0x453480`, `0x468580`, `0x6709a0`, `0x670950`, `0x670a30`, `0x6709f0`, `0x801360`
Confidence: verified
Proof: one
Test: `objectlayer::diff_normalize_vec2`, `objectlayer::g3_transport_ms_scale`, `objectlayer::g3_transport_leg_fraction`, `objectlayer::g3_transport_heading`, `objectlayer::g3_transport_heading_angle`

`0x5f50e1` divides unsigned. A span's T is stop j's time less the span start and t the elapsed
part, each `fild` of a zero-extended qword times `[0x801360]`, stored as floats; the curve gets
accelerates = j ≠ 0 and decelerates = j ≠ count − 1. `0x5f8cb0` leaves a vector shorter than
2^-22 alone, else scales by 1/len. `fpatan` takes the y, x pair at `0x5f528e`. The map test is
`0x5f5274`; it stores the leg in `+0x58`. Readiness sets the done callback `0x5f49b0` once and
the byte `+0x5c`. Slices: g3_transport_ms_scale `0x5f51b7`-`0x5f51ee`; g3_transport_leg_fraction `0x5f51ee`-`0x5f5230`; g3_transport_heading `0x5f5251`-`0x5f528e`; g3_transport_heading_angle `0x5f5251`-`0x5f52ae`.

## Carrying and changing legs

Then it builds the frame L, a translation to the position turned about z by the facing. For each
passenger: once the entity is ready, flag 0x8000000 clears; the unit's body is refreshed and its
model gets L as its transport frame. When the leg differs from `+0x58`, `+0x58` takes it, and if
the camera follows a passenger and this is not the first tick, the client shows a loading screen
and unloads the map around the camera's unit.

Addresses: `0x5f8da0`, `0x7bdc40`, `0x7bdd60`, `0x7c6510`, `0x613ef0`, `0x718910`, `0x66fc70`, `0x66fc80`, `0x615020`, `0x407e80`
Confidence: verified
Proof: one
Test: `objectlayer::world_model_scale_difftest`

The frame starts at identity; `RotateMatrix` takes the axis (0, 0, 1). The passengers are walked through `0x5f8da0`, a link getter. The unit is
`ClntObjMgrObjectPtr([mover+0x30], 8)`; `0x613ef0(unit, 0)` runs before `SetTransportFrame` on
`+0xdc`, else `+0xd8`. The first tick is `+0x54` = −1 on entry. The loading sequence is
0x443620(0x407f70, 0), `SetLoadProgressCallback(0x407fa0, 0)`, 0x407de0 with the entry and the
old leg, `UnloadAreas` at the camera unit's slot 5 position, `0x615020`, `EndLoadingScreen`, then
both callbacks cleared.

## The transport's path

A type 11 handler, 0x5c bytes, holds its `TransportAnimation` rows (`+0x20`, count `+0x24`), the
current row `+0x28`, its base position `+0x2c`, speed `+0x38`, heading `+0x3c` and the
animation it played `+0x58`, 0xd0 at first. Its rows are the first run with the object's entry.
The offset at a time t = (now + `+0x278`) mod the last row's time lies between the row A at or
before t and the next row B: f = (t − A) / (B − A), p = A·(1 − f) + B·f, turned by the object's
rotation quaternion. A's animation plays when it changes.

Addresses: `0x5f5ab0`, `0x5f5ec0`, `0x5f6260`, `0x5f6280`, `0x5f8d20`, `0x5f5c80`, `0x5f5cb0`, `0x5f5d70`, `0x5f5da0`, `0x5f9110`, `0x80ba58`, `0xc0d654`
Confidence: verified
Proof: one
Test: `objectlayer::gameobject_path_eval_difftest`, `objectlayer::gameobject_path_eval_large_times_difftest`

The constructor makes the passenger list through `0x5f9110`. Rows are 0x1c bytes: entry `+4`, time `+8`, position `+0xc`, animation `+0x18`. `0x5f5ec0` stops
at the first entry above the object's: the store is taken as sorted. With no row the constructor
leaves `+0x20` and `+0x24` unset, and the tick reads through them. `0x5f62bc`–`0x5f62e7` advance
A with no bound: a t below the first row's time matches no pair and never ends. f is a `fidiv`
of the zero-extended difference by the signed span, stored as a float. With x2 = 2x and the
products stored as floats, M = [1 − (yy + zz), xy + wz, xz − wy; xy − wz, 1 − (xx + zz), yz + wx;
xz + wy, yz − wx, 1 − (xx + yy)], and out_j = (M[2][j]·pz + M[1][j]·py) + M[0][j]·px. px rounds
gA + fB to float; py adds f·By to float(g·Ay) at 53 bits; pz adds float(g·Az) and float(f·Bz).

## The transport tick

The new position is the fields' position plus the path offset. When the squared move from the
base is below 2^-20 nothing moves: a ready entity clears flag 0x8000000 on its passengers. With
no passengers the base and entity take the new position. Otherwise each passenger is swept along
the move by `SweepCarrierMove` in the frame of the old base, then the base, the frame and the
entity move, the speed becomes the distance over the elapsed seconds and the passengers'
models take the frame. The velocity (slot 20) is speed × heading, kept from the last carried
move.

Addresses: `0x5f5f10`, `0x5f5c10`, `0x5f5c40`, `0x6343b0`
Confidence: verified
Proof: one
Test: `objectlayer::diff_scale_vec3`, `objectlayer::g3_move_pos_add`, `objectlayer::g3_move_delta_sub`, `objectlayer::g3_move_dist2_gate`, `objectlayer::g3_move_normalize`, `objectlayer::g3_move_leg_speed`

The squared length is (dy² + dz²) + dx², stored as a float and compared by magnitude at
`0x5f5fb3`. The distance is float(sqrt) and the heading each delta times 1/distance, stored.
`0x5f61ba` divides the distance by float(elapsed)·0.001 at 53 bits. The frame is L of the base and
the object slot 6 facing; its translation row takes the new position after the sweeps. Slot 17
(`0x5f5c10`) returns the base. Slices: g3_move_pos_add `0x5f5f47`-`0x5f5f69`; g3_move_delta_sub `0x5f5f6e`-`0x5f5f93`; g3_move_dist2_gate `0x5f5f98`-`0x5f5fbb`; g3_move_normalize `0x5f60f2`-`0x5f612d`; g3_move_leg_speed `0x5f61ac`-`0x5f61c0`.
