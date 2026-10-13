# objectlayer: unit movement

How a unit takes movement: the server's movement messages, monster moves and spline toggles; the
local commands the input code queues on the unit's mover; the display and body facing; and the
auto-move machine behind click-to-move, follow and walking up to an interaction. The mover is the
`CMovement` embedded at `+0x9a8` (`collision` owns it); `+0x118` points at it, so its movement
flags are `[unit+0x9e8]`. The functions here pick the unit, forward to the mover, and refresh the
unit's animation and state around it.

Roots: `0x603bb0`, `0x603f00`, `0x601420`, `0x6106b0`, `0x611130`

## Movement message handlers

`0x6033c0` registers these handlers. Each reads the unit's GUID, packed except where noted, and
forwards; an unknown unit moves the store's cursor to its end and returns 0, except `0x603f90`,
which returns 1.

| handler | opcodes | forwards to |
|---|---|---|
| `0x603bb0` | 30 relayed moves, 0xb5 to 0x2b1 | `0x601580` (time, opcode, store) |
| `0x603ae0` | 0xcd, 0xcf, 0xd1, 0xd3, 0xd5, 0xd8 | `0x6010e0` (time, opcode, store) |
| `0x603c10` | 0x2fe to 0x303, then a float | `0x601360` (opcode, speed) |
| `0x603c80` | 0x304 to 0x30e and 0x31a | `0x601420` (opcode) |
| `0x603f90` | 16 forced changes, 0xc7 to 0x2de | `0x602780` (time, opcode, store) |
| `0x603b40` | 0x319, then a u32 | `0x601560` |
| `0x603f00` | 0xdd; 0x2ae with a second packed GUID | `0x617100` transport, then `0x6018f0` |

Addresses: `0x6033c0`, `0x603bb0`, `0x603ae0`, `0x603c10`, `0x603c80`, `0x603f90`, `0x603b40`, `0x603f00`, `0x601560`
Confidence: verified
Proof: one
Test: none

`0x6033c0` pairs each `mov ecx,opcode` with `mov edx,handler` before `0x5ab650`. The relayed set is
0xb5 to 0xbe, 0xbf to 0xc3, 0xc5, 0xc9 to 0xcb, 0xd9 to 0xdb, 0xec to 0xee, 0xf1, 0xf7, 0x2ad,
0x2b0, 0x2b1; the forced set 0xc7, 0xde, 0xdf, 0xe2, 0xe4, 0xe6, 0xe8, 0xea, 0xef, 0xf2 to 0xf5,
0x2da, 0x2dc, 0x2de. The handlers call `0x642ed0` for the packed GUID and `0x468460` with mask 8;
the miss path stores `[store+0x10]` into `[store+0x14]`. `0x603f00` passes 0 as the transport for
0xdd. `0x601560` passes its u32 to the mover's skipped-time call and returns 1. The dispatchers
switch inline: `0x6010e0` through `0x6012fc` and `0x60132c` (12 cases each, value − 0xcd),
`0x601360` through `0x601408` (6, value − 0x2fe), `0x601580` through the bytes `0x6018ac` (61,
value − 0xb5) into `0x601844` (26), and `0x602780` through `0x602a50` (36, value − 0xc7) into
`0x602a2c` (9); `0x60e0a0` reads `0x60e408` into `0x60e3fc` and `0x60e448` into `0x60e438`.

## Compressed moves

`0x603ce0` handles 0x2fb: a u32 inflated size, then a zlib stream. It reserves the size, rounded
up to 4, on the stack, inflates into it with the size as the output limit, and then dispatches
each `u8 length` and `length` bytes as one message through `0x537aa0`. Nothing bounds the size:
one above the free stack faults in `0x40a280`, and 0xfffffffd to 0xffffffff round to 0, so the
inflate writes past a zero-byte buffer.

Addresses: `0x603ce0`, `0x603cf7`, `0x603d00`, `0x603d33`, `0x603d67`, `0x603da6`
Confidence: verified
Proof: one
Test: none

`0x603cf7` loads the size, `add eax,3; and eax,-4`, and `0x603d00` calls `0x40a280` with it; the
limit `[ebp-8]` keeps the unrounded size and `0x603d33` passes it to `0x660740`. The loop reads the
length with `0x418cb0` (`0x603d67`), wraps the bytes in a second store and calls
`0x537aa0` on `0x5ab490()` (`0x603da6`) until the cursor reaches the inflated length.

## Spline toggles

`0x601420` maps opcodes 0x304 to 0x31a through the byte table `0x601544` to the case table
`0x601510`; opcodes 0x30f to 0x319 return 0. After a case it calls `0x60e480(opcode)` and
`0x5fd9e0(-1)` and returns 1.

| opcode | mover call |
|---|---|
| 0x304, 0x31a | unroot `0x619a40`, root `0x619a10` |
| 0x305, 0x306 | feather fall `0x61a4e0` on, off |
| 0x307, 0x308 | hover `0x61a620` on, off |
| 0x309, 0x30a | water walk `0x61a3d0` on, off |
| 0x30b, 0x30c | swim start `0x61a130`, stop `0x61a160` |
| 0x30d, 0x30e | run mode `0x617e80` with 1, 0 |

Addresses: `0x601420`, `0x601510`, `0x601544`, `0x6014ec`
Confidence: verified
Proof: one
Test: none

The bound is `lea eax,[edi-0x304]; cmp eax,0x16`. The byte table holds 0 to 10 for 0x304 to
0x30e, 12 (the return-0 case `0x601508`) for 0x30f to 0x319, and 11 for 0x31a; the case table has
13 dwords. `0x6014ec` is the shared tail.

## Monster moves

`0x6018f0(store, opcode)` reads the start point, the spline id and a type byte. Type 1 stops:
within 3 yards the mover flushes and stops at the point. Types 2, 3 and 4 add a face point, a GUID or an angle.
Then come the spline flags, the duration and a u32 count of points. Flag 0x200 sends every point;
otherwise the last point comes whole and the middle points as packed offsets. The path is anchored
at the unit (`0x6021c0`), started (`0x6187a0`) and given its facing; the active mover's input
stops; the precast, ranged pose and spell effect are cleared.

Addresses: `0x6018f0`, `0x601a58`, `0x601a68`, `0x601ccd`, `0x601e05`, `0x602130`, `0x6021c0`, `0x602e80`, `0x6187a0`
Confidence: verified
Proof: one
Test: `objectlayer::monster_move_case1_near_gate_difftest`, `objectlayer::monster_move_direction_sincos_difftest`, `objectlayer::monster_move_case_delta_sub_difftest`, `objectlayer::monster_move_case_delta_sub_clone_difftest`, `objectlayer::monster_move_case2_coincident_gate_difftest`, `objectlayer::monster_move_case34_coincident_gate_difftest`, `objectlayer::monster_move_reflect_through_difftest`, `objectlayer::monster_move_arc_length_difftest`, `objectlayer::monster_move_speed_duration_difftest`, `objectlayer::move_event_packed_delta`, `objectlayer::anchor_region_a_difftest`, `objectlayer::anchor_region_b_difftest`, `objectlayer::anchor_region_c_difftest`, `objectlayer::anchor_region_d_difftest`, `objectlayer::anchor_region_e_difftest`, `objectlayer::anchor_region_f_difftest`

The path buffer is `0x40a280((3·count + 12)·4)`, computed in 32 bits (`0x601a58` to `0x601a68`),
and the loops at `0x601ccd` and `0x601e05` run to the count. A count of 0x15555552 or more wraps
the size, and the loops write points past the buffer. Type 1's 3 yards is a squared distance
under 9 (0x80c5bc) before `0x618750`; farther, the stop point runs as a one-point path. `0x602130` turns a u32 into x, y and z from
bits 0 to 10, 11 to 21 and 22 to 31, each sign-extended and times 0.25, and subtracts them from
the reference point. `0x6021c0` empties a one-point path within 1/36 yard of the unit (squared
against `0x80c450`) and otherwise prepends the unit's position or cuts the points it has passed.
`0x602e80(guid)` sets the facing toward the object the GUID names through `0x47f220` and
`0x6198e0`. Facing calls: `0x6188a0`, `0x618870`, `0x618840`.
The tests run `0x601982` to `0x6019cf`, `0x601ab7` to `0x601ac0`, `0x601aca` to `0x601b1a`,
`0x601b7e` to `0x601be2`, `0x601c1d` to `0x601c50`, `0x601d41` to `0x601d79`, `0x601e83` to
`0x601eca`, `0x601eda` to `0x601f7e`, `0x601eda` to `0x601faf`, `0x602148` to `0x6021b5`, `0x6021da`
to `0x60221e`, `0x602252` to `0x6022bb`, `0x602324` to `0x602342`, `0x60237b` to `0x6023ad`,
`0x602426` to `0x6024c3` and `0x602426` to `0x60257b`.

## Relayed move events

Thin wrappers forward a relayed or replayed move to the mover. Turn and pitch starts are refused,
returning 0, while bit 0x40000 of `UNIT_FIELD_FLAGS` (stunned) is set. `0x602670` and `0x6026f0`
read four floats, a direction and two speeds, and queue or apply a knockback. `0x602fb0` reads a
movement status; without the manager's `+0xc8` it queues a teleport, else it answers with opcode
0xc7 carrying the GUID.

Addresses: `0x602a80`, `0x602aa0`, `0x602ac0`, `0x602ae0`, `0x602b00`, `0x602b20`, `0x602b60`, `0x602b80`, `0x602bc0`, `0x602be0`, `0x602c00`, `0x602c20`, `0x602e30`, `0x602e60`, `0x602ef0`, `0x602f10`, `0x602f90`, `0x602760`, `0x602670`, `0x6026f0`, `0x602fb0`, `0x602c60`
Confidence: verified
Proof: one
Test: none

Each wrapper is `add ecx,0x9a8` and a jump-free call: move start `0x617520`, strafe `0x6178e0`,
stops `0x617b60`, `0x617c10`, jump `0x617970`, turn `0x617cf0`, `0x617d80`, pitch `0x61a290`,
`0x61a320`, swim `0x61a190`, `0x61a1f0`, heartbeat `0x61a750`, run mode `0x617e30`, gravity
`0x619c50`, facing `0x619910`, pitch `0x619960`, teleport `0x6186b0`, hover `0x61a6b0`. `0x602b20`
and `0x602b80` test `[[unit+0x110]+0xa0]` for 0x40000. `0x602670` calls `0x60e990(1,0)` and
`0x617a30`; `0x6026f0` calls `0x617a60`. `0x602c60(flags, force)` picks a landing's animation:
without `force` or bit 0x4000 it re-picks only when the direction bits 0xf change; else a swimmer,
a backward or a walking unit re-picks, no direction plays 39 (JumpEnd), and the rest 187
(JumpLandRun).

## Local movement commands

The input code moves its unit through these. Each calls `0x60e990` first, which stops an auto-move
the input did not start and closes the loot window unless told not to, then queues the change on
the mover. A jump while mounted and standing still is no jump: the active player plays MountSpecial
and sends 0x171 with no body. A facing change wakes a sleeping unit (stand state 3).

Addresses: `0x60dc60`, `0x60dc90`, `0x60dcb0`, `0x60dce0`, `0x60dd00`, `0x60dd80`, `0x60dda0`, `0x60ddd0`, `0x60de00`, `0x60de30`, `0x60de70`, `0x60dea0`, `0x60e060`, `0x60e080`, `0x60e990`, `0x60e860`, `0x60e880`, `0x60e8a0`, `0x60e8f0`, `0x60e920`, `0x60e940`
Confidence: verified
Proof: one
Test: none

`0x60e990(stops, keepsLoot)`: with `stops`, an auto-move in progress (`0x6103a0`) and bit 0 of
`0xc4da48` clear, it calls `0x60fb60(0,1)`; unless `keepsLoot`, an active player who is looting
(`0x6126b0`) calls 0x48f200. `0x60dd00` (turn start) also sends the channel
cancel `0x6e9b70` for a channel spell with AttributesEx 0x4000 while auto-moving. `0x60dea0`
tests MOUNTDISPLAYID, the direction bits 0xf, turning 0x30 and `0x605650` before the 0x171 path.
`0x60de30` compares slot 41 with 3 and calls `0x60be30(0)`. `0x60e080` passes the walk bit 0x100
to `0x60e060`. The input setters `0x60e8a0` and `0x60e940` act only while the input owns the
mover, with bit 0 of `0xc4da48` set around the call; `0x60e860`, `0x60e880`, `0x60e8f0`,
`0x60e920` press and release input commands 0x200000 and 0x800000 through `0x515090`;
`0x60e8f0` keeps its facing at `0xc4d8c4`.

## Movement packets

`0x600860` writes a movement message: the opcode, the GUID for relayed opcodes, a u32 for those
that carry one, the status, and a float for nine opcodes. Falling (0x2000) sets bit 0x80 of the
unit's state word `+0xc58`. `0x600a10` sends with the mover's step time.

Addresses: `0x600860`, `0x6009b0`, `0x600a10`
Confidence: verified
Proof: one
Test: none

The GUID test is `0x7ccc10(op)` and the u32 test `0x7ccb90(op)`; the status goes through the
mover's init and write calls. `0x6009b0` returns 1 for 0xe3, 0xe5, 0xe7, 0xf6, 0x2cf, 0x2d0,
0x2db, 0x2dd and 0x2df. `0x600a10(op)` calls `0x600a30(op, step, 0, 0)`.

## The active mover

The active mover is the GUID at `0xc4da98`. `0x6006e0(guid)` switches it: the old mover's
auto-move stops, bit 31 of its flags clears and, with no running spline, its queue resets; the
active player's old mover sends 0x2d1. The new GUID is stored and, when nonzero, sent with 0x26a;
the new mover resets and the input refreshes. `0x6006b0` stops the auto-move and clears it.

Addresses: `0x6006e0`, `0x6006b0`, `0x600753`, `0x600783`, `0x6007ae`, `0x5fa5f0`, `0x5fa600`, `0x603ea0`
Confidence: verified
Proof: one
Test: none

`0x600753` clears bit 31 of `[+0x9e8]`; `0x619d50(0)` resets the old mover when `+0xa4` is empty
or has bit 4 of `+0x18`. `0x600783` pushes 0x2d1 for `0x600a10`; `0x6007ae` 0x26a with the GUID.
The new mover gets `0x619d50(1)` and `0x514640(now,1)`. Client control is bit 0x400 of `+0xc58`:
`0x5fa5f0` reads it, `0x5fa600` sets it and re-picks the camera's mover, and `0x603ea0` (0x159)
reads a packed GUID and a byte and calls `0x5fa600(byte != 0)`.

## Facing

A unit keeps two facings: the body's at `+0xc94` and the display's at `+0xc98`. The active mover's
display follows the mover; another unit's turns toward its move direction or, idle, toward its
target, averaging four samples at `+0xc9c`. `0x6084f0` wraps an angle into −π to π.

Addresses: `0x600cd0`, `0x600cb0`, `0x601020`, `0x601050`, `0x607eb0`, `0x6084f0`, `0x60a110`, `0x608560`
Confidence: verified
Proof: one
Test: `objectlayer::wrap_angle_difftest`

`0x600cd0` sets bits 0x1 and 0x2 of `+0xd58` for key and mouse turning on the active mover.
`0x600cb0` and `0x607eb0` convert `+0xc98` and `+0xc94` to world facings through the transport.
`0x601020` resets `+0xc98` from the mover's facing and zeroes `+0xc9c` to `+0xca8`; `0x601050`
adds a transport's turn to both facings, wrapping each once into 0 to 2π. `0x6084f0` uses
`fmod` past ±π. `0x608560` (slot 15) places a swimming unit's model with the pitched transform
`0x60a110`: scale `[+0x94]·[+0x90]` times the mount's `+0x9c` or the unit's `+0x98`.

## Auto-move modes

Auto-move drives the active mover toward a target in a mode, `0xc4d888`, 12 meaning none. Each
mode has a 12-byte row of `0x860a58`: enabled, a maximum squared distance, and two flags.

| mode | use | maximum | stop distance² |
|---|---|---|---|
| 0, 1 | keep facing the target or channel object | none | none |
| 2 | face a direction | none | none |
| 8 | walk along a heading | none | none |
| 3 | follow (`AUTOFOLLOW_BEGIN`) | 900 | 9 |
| 4 | walk to a point | none | 0.25 |
| 5 | walk to an NPC | 6400 | 7.7160487 |
| 6, 10 | loot; walk to attack | 6400 | the reaches + 4/3, at least 5 |
| 7 | walk to a game object | 6400 | `0xc4d998`² |
| 9 | walk into spell range | 6400 | range · 0.9, squared |

Addresses: `0x860a58`, `0x60e590`, `0x611240`, `0x6113d8`, `0x611310`, `0x61132c`, `0x6111ab`, `0x6110a0`, `0x6103a0`, `0x611590`, `0x60f9fc`, `0xc4d888`, `0xc4d9c0`, `0xc4d910`
Confidence: verified
Proof: one
Test: none

`0x60e590` writes rows 3 to 10 at start-up: 900 at `0x860a80`, 6400 at rows 5, 6, 7, 9 and 10,
the `+8` byte 1 and `+9` set only on row 8; row 3's enabled byte is the file's. `0x611240(mode,
guid)` takes row `+0` and `+9` and switches modes 3 to 10 through `0x6113d8`: 3 `0xc4d9c4`
(3², `0x60e6e0`), 4 `0xc4d9b0` (0.5²), 5 `0xc4d958` (2.7777777²), 6 and 10 the combat reaches
(field 130) plus 4/3, at least 5, mode 10 less 4/3, 7 `0xc4d998` squared, 9 `0x6e3480`'s range
of the spell in 0xb700e4 times `0x804588` (0.9), squared. `0x611130` keeps the value at
`0xc4d9c0` and its square root at `0xc4d910` (`0x6111ab`), so modes 6 and 10, whose value
`0x611310` and `0x61132c` store unsquared, stop at the root of the reach, not at the reach. `0x6110a0` refuses a target past the
row's maximum with message 0x126, `ERR_AUTOFOLLOW_TOO_FAR`. `0x6103a0` is the active mover with a
mode set; `0x611590(mask)` tests `1 << mode`.

## Starting and ending auto-move

`0x611130(mode, guid, point, facing)` stops any auto-move, checks the range and the stop distance,
and stores the mode, start time, start position, target GUID, point (`0xc4d890`) and facing
(`0xc4d9a8`); a seated unit stands. `0x60fed0(guid, mode)` starts one toward an object after per-mode checks. `0x60fb60(face,
stop)` ends one: follow signals `AUTOFOLLOW_END`, `face` turns toward the target, `stop` releases
the move, turn and pitch commands, and the mode returns to 12.

Addresses: `0x611130`, `0x60fed0`, `0x610094`, `0x610084`, `0x60fb60`, `0x60fc50`, `0x60fcc0`, `0x6100a0`, `0x610100`, `0x6102b0`, `0x610300`, `0x60fe90`, `0x60e790`, `0x60e7f0`, `0x60f900`
Confidence: verified
Proof: one
Test: none

`0x60fed0` maps modes 0, 1, 5 to "any unit", 3 to follow, 6 and 9 to "a corpse or a dead unit",
the rest to refusal, through `0x610094` and `0x610084`. Follow needs a cooperative player and a
live, unstunned, unchannelled follower: messages 0x7e, 0x191, 0x134, else 0x128. `0x60fb60` signals
event 0x170 for mode 3, turns through `0x60e8f0` and stops through `0x60e7f0`, `0x60e920`,
`0x60e880`. `0x60fc50` clears the precast bit 0x400 of `+0xd58`, ends the ranged pose when the ranged weapon
is out, releases the spell effect and stops an auto-move whose mode is in `0x860adc`. The walks
need `0x60f900`: a live active mover and the `AutoInteract` CVar (`0xc4d9a4`) on. `0x60fcc0`
walks to attack in mode 10, or faces (`0x6100a0`, mode 2) within reach. `0x610100` (mode 8) pitches a swimmer toward
the clicked point first. `0x6102b0` is mode 4, `0x610300` mode 7. `0x60e790` and `0x60e7f0` press and
release command 0x100000 and start or stop the move.

## Steering an auto-move

`0x6106b0` runs each frame for the active mover. It reads the target (`0x610e40`), turns toward it
by at most the CVar turn speed, pitches a swimmer, and arrives inside the stop distance. Arrival
runs the mode's action through `0x60f940`, and `0x60fa20` completes a pending interaction after
the move.

Addresses: `0x6106b0`, `0x610e40`, `0x6103d0`, `0x610520`, `0x610d50`, `0x610da0`, `0x60f940`, `0x60fa20`, `0x611400`, `0x6115e0`
Confidence: verified
Proof: one
Test: `objectlayer::broadcast_wrap_angle_difftest`

`0x610e40` returns the transport-local point for modes 4 and 10, nothing for 2 and 8, else the
object `0xc4d980`, refusing a taxi rider and, outside modes 6 and 9, a dead unit; its distance
drops the height near the target (next entry). `0x6103d0` turns at `automoveturnspeedwide` (1200, used between 2π/3
and 4π/3 off) or `automoveturnspeednarrow` (800), in degrees, latched at `0xc4d93c`, and stops
within 0.001. `0x610d50`
compares squared distance with 0.002777778². `0x610da0` is `asin` of the normalized height. On
arrival `0x60f940` stores the GUID for mode 5 at `0xc4da28`, 6 at `0xc4da78`, 7 at `0xc4d950`,
9 at `0xc4da20`, and `0x60fa20` hands each to `0x5f0130`, `0x5df2a0`, `0x5f86b0` or `0x5f05e0`.
`0x611400` draws a pulsing box at mode 4 and 10 targets. `0x6115e0` moves the target point and
facing with a transport.

## Arrival, height and stalls

Speed widens the arrival: a mover faster than 7 yards a second arrives that much farther out.
Near the target the height stops counting unless the mover swims. A frame that brings the mover
no closer, a target behind a drunk mover, a stall while walking forward, or a target straight
above or below ends the walk. A walk along a heading (mode 8) first pitches a swimmer level, steep or straight.

Addresses: `0x6106b0`, `0x6107e5`, `0x6108e6`, `0x610a7b`, `0x610aaa`, `0x610c63`, `0x610d32`, `0x610e40`, `0x610100`
Confidence: verified
Proof: one
Test: none

`0x610e40` ignores height within 0.5 yard (0x80c4c8), or the stop distance in mode 10, times
max(speed/7, 1) (0x80c4d0); `0x610aaa` scales the stop distance the same way, capped in modes 5
and 10. `0x6107e5` stops when the distance to the target is not below the distance from the last
frame's position (0xc4da84) while bit 1 of `0xc4da48`, set once the turn is done (`0x6108e6`),
holds, outside modes 3 and 5. With a drunkenness argument above 0 and bit 1, `0x610a7b` stops when
the target lies 160° to 220° off the facing (0x80c604, 0x80c600), and mode 3 drops bit 1 when a
random fraction times it tops 0.07 (0x80c608). Mode 3 starts walking only beyond
4.5·max(speed/7, 1) yards (3, 0x80c4c0, times 1.5). `0x610c63` stops a
forward walk that moved less than 1/360 yard (0x80c4dc, through `0x610d50`) in modes with flag
byte +8. `0x610d32` stops when the direction's height part tops cos 1° (0xc4d9d0, from 0x80c5f8).
`0x610100` pitches a swimmer to ±π/2 when the height part of the heading is at least 0.86602
(0x80c4d4), to its `asin` from 0.17364 (0x80c4d8), else to 0.
