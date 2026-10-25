# collision: handlers

The movement-status handlers of Movement.cpp and the code that reports a mover's own state
changes. A unit's opcode switches call them with the unit's `CMovement` as `this`. Most changes
come in up to four forms: a forced form for the client's own mover, which queues a move event with
byte 1 at event `+0x4d` and the server's counter at `+0x48`; an own-relay form, which queues the
same tag with byte 0; a spline form, which applies at once; and a relay form for another unit,
which goes through `0x618f20` and applies its effect only when the status applies at once
(`status`, "Status entry points"). The tags are the immediates these pass; the queue replays the
events by tag. The broadcaster `0x61a820` turns a change of the mover's flags into an opcode.

Roots: `0x619500`, `0x619520`, `0x619530`, `0x619550`, `0x619590`, `0x6195b0`, `0x6195c0`, `0x6195e0`, `0x619620`, `0x619640`, `0x619650`, `0x619670`, `0x6196b0`, `0x6196d0`, `0x6196e0`, `0x619700`, `0x619740`, `0x619760`, `0x619770`, `0x619790`, `0x6197d0`, `0x6197f0`, `0x619800`, `0x619820`, `0x619860`, `0x6198a0`, `0x6198e0`, `0x619910`, `0x619960`, `0x6199c0`, `0x619a10`, `0x619a40`, `0x619a60`, `0x619c50`, `0x619fb0`, `0x61a030`, `0x61a070`, `0x61a0f0`, `0x61a130`, `0x61a160`, `0x61a190`, `0x61a1f0`, `0x61a240`, `0x61a290`, `0x61a2e0`, `0x61a320`, `0x61a380`, `0x61a3d0`, `0x61a430`, `0x61a490`, `0x61a4e0`, `0x61a550`, `0x61a5d0`, `0x61a620`, `0x61a6b0`, `0x61a700`

## Speed changes

Six speeds have four handlers each, differing only in tag and setter. Forced:
`0x6176a0(time, tag, 1, counter, speed, 0)`. Own relay: `0x6176a0(time, tag, 0, 0, speed, 0)`.
Spline: the setter at once. Relay: `0x618f20(eventTime, status, tag, speed)`, then the setter when
that applied, returning the setter's result, else 0. A setter returns 0 and stores nothing when
`|new − old|` < 2⁻²² (NaN stores); otherwise it stores, re-anchors (`0x7c5cd0`), updates the current
speed (`0x7c5c20`) except for the turn rate, and returns 1. The opcode reaching each handler is in
brackets; both relay forms take the same opcode, the switch choosing by the sender's GUID.

| speed | field, tag | setter | forced | spline | own relay | relay |
|---|---|---|---|---|---|---|
| run | `+0x8c`, 0x14 | `0x7c7030` | `0x619500` (0xe2) | `0x619520` (0x2fe) | `0x619530` | `0x619550` (0xcd) |
| run back | `+0x90`, 0x15 | `0x7c7080` | `0x619590` (0xe4) | `0x6195b0` (0x2ff) | `0x6195c0` | `0x6195e0` (0xcf) |
| walk | `+0x88`, 0x16 | `0x7c70d0` | `0x619620` (0x2da) | `0x619640` (0x301) | `0x619650` | `0x619670` (0xd1) |
| swim | `+0x94`, 0x17 | `0x7c7120` | `0x6196b0` (0xe6) | `0x6196d0` (0x300) | `0x6196e0` | `0x619700` (0xd3) |
| swim back | `+0x98`, 0x18 | `0x7c7170` | `0x619740` (0x2dc) | `0x619760` (0x302) | `0x619770` | `0x619790` (0xd5) |
| turn rate | `+0x9c`, 0x19 | `0x7c6ff0` | `0x6197d0` (0x2de) | `0x6197f0` (0x303) | `0x619800` | `0x619820` (0xd8) |

Addresses: `0x619500`, `0x619550`, `0x7c7030`, `0x7c6ff0`, `0x6176a0`, `0x618f20`, `0x602780`, `0x601360`, `0x6010e0`
Confidence: verified
Proof: one
Test: none

`0x619500`: `push 0; push a3; push a2; push 1; push 0x14; push time`. `0x619530`: `push 0; push a2;
push 0; push 0; push 0x14`. `0x619550`: `push speed; push 0x14; push status; push time`, then
`test eax,eax; jne` to the setter. The setters: `fld old; fsubr new; fabs; fcomp 2⁻²²
(0x34800000); test ah,5; jp` to the store, else `xor eax,eax`. The opcodes come from three owner
switches: `0x602780` (opcode − 0xc7 through a byte table, and compares from 0x2da), `0x601360`
(opcode − 0x2fe), and `0x6010e0`, whose two tables over opcode − 0xcd serve the client's own mover
(GUID equal to `0xc4da98`) and every other unit. The forced handlers' counter is the dword the
switch reads before the speed.

## Facing and pitch

`0x619860(time, facing)` queues `0x617570(time, 0x10, 1, 0, &zero, facing, 0)` and `0x6198a0(time,
pitch)` queues tag 0x11 with the pitch. `0x6198e0(facing)` sets the facing at once through
`0x7c6eb0` and, idle (flags & 0x20ff = 0), drains the queue and unlinks (`0x617fc0(1)`).
`0x619910`, for opcode 0xda, applies through `0x619280` (tag 0x10); `0x619960`, for 0xdb, through
`0x618f20` with tag 0x11 and the status pitch `+0x34`. Each of the two, when the status applied and
the mover is idle with an empty queue, unlinks it (`0x619e80(1)`), and returns 1; deferred, 0.

Addresses: `0x619860`, `0x6198a0`, `0x6198e0`, `0x619910`, `0x619960`, `0x617570`, `0x7c6eb0`, `0x617fc0`, `0x619280`, `0x619e80`
Confidence: verified
Proof: one
Test: none

`0x619860` pushes `0, facing, &zero, 0, 1, 0x10, time`; `0x6198a0` pushes
`pitch, 0, &zero, 0, 1, 0x11, time`. The idle test in `0x619910` and `0x619960` is
`test [esi+0x40],0x20ff; jne` and `0x6158f0`; `0x619960` loads `mov ecx,[eax+0x34]` from the
status. The opcodes come from `0x601580`, whose switch over opcode − 0xb5 sends 0xda and 0xdb to
the owner's forwarders of these two.

## Root and unroot

Opcode 0xec applies through `0x6199c0`: `0x618f20` with tag 0x23; applied, it roots (`0x619a00`)
and unlinks (`0x619e80(1)`). Opcode 0xed applies through `0x619a60` with tag 0x24; applied, it
unroots (`0x619a40`). Rooting stops every motion (`0x61aa70`) and then sets 0x1000, ends a fall
and keeps flags & 0xffe07f00 (`0x7c7340`). Unrooting clears 0x1000, starts a fall when it can
(`0x7c7370(1)`), and links the mover when flags & 0x20ff. The spline root, opcode 0x31a, is
`0x619a10`: root, `0x616d30` when a spline exists, unlink. The spline unroot,
0x304, is `0x619a40`. The forced pair 0xe8 and 0xea queue tag 0x23 or 0x24 with the counter
(`0x61a700`).

Addresses: `0x6199c0`, `0x619a60`, `0x619a00`, `0x619a10`, `0x619a40`, `0x61a700`, `0x61aa70`, `0x7c7340`, `0x7c7370`, `0x616d30`
Confidence: verified
Proof: one
Test: none

`0x6199c0`: `push 0; push 0x23`, `call 0x618f20`, then `0x619a00` and `push 1; call 0x619e80`.
`0x619a00` is `call 0x61aa70` and `jmp 0x7c7340`; `0x7c7340` does `or [esi+0x40],0x1000`, calls
`0x7c6290`, `and [esi+0x40],0xffe07f00`. `0x7c7370` is `and edx,0xffffefff` and, for a nonzero
argument, `call 0x7c61c0`. `0x61a700`: `neg eax; sbb eax,eax; add eax,0x24` gives 0x23 for a
nonzero `root`. The opcodes: 0xec and 0xed in `0x601580`'s table; 0x31a and 0x304 in the
spline-mode switch over opcode − 0x304; 0xe8 and 0xea in `0x602780`.

## Gravity toggle

Flag 0x800 suspends falling. `0x619aa0` acts on the flags as they stand: when 0x800 is set, or a
running spline has flag 0x200, it records `fallMoving` (0x2000 with `+0xa0` ≠ 0.0, NaN counting
as nonzero) and the flags without 0x800, ends the fall (`0x7c6290`), applies the latched
directions (`0x61ab20`), broadcasts with those old flags at the update time
(`0x61a820(t, 0, old, fallMoving, 0)`), and, idle with an empty queue, unlinks. Otherwise it
tries to start a fall and links the mover only when the fall starts (`0x617b00`). Opcode 0x2ad applies through `0x619c50`: tag 0x1b
when the status has 0x800, else 0x1a; applied, it re-anchors and runs `0x619aa0`.

Addresses: `0x619aa0`, `0x619c50`, `0x619adc`, `0x619ae7`, `0x619c65`, `0x7c6290`, `0x61ab20`, `0x617b00`
Confidence: verified
Proof: one
Test: none

The test is `test al,4; jne` and `test ah,2; jne` on spline `+0x18`, then `test ah,8` on the
flags. `fcomp 0.0; test ah,0x44; jnp` at `0x619adc` takes `ebx` = 0 only for equal;
`and ecx,0xfffff7ff` at `0x619ae7`. The broadcast pushes `0, ebx, edi, 0, [globals+0x128]`.
`0x619b3e` is `jmp 0x617b00`. In `0x619c50`, `and ecx,0x800; or ecx,0xd000; shr ecx,0xb` at
`0x619c65` yields 0x1a or 0x1b; on apply it calls `0x7c5cd0(0)` and `0x619aa0`. The drain calls
`0x619aa0` after `0x7c7240` changes the bit.

## Swim start and stop

Swimming is 0x200000. `0x61a230` enters it (`0x7c6e50`) and applies the latched directions. `0x61a130`, opcode 0x30b, enters and unlinks an idle
unqueued mover; `0x61a160`, 0x30c, leaves through `0x7c6e80` (clears 0x200000 and 0xc0,
zeroes `+0x20` and `+0x54`, starts a fall) and links a mover that is busy or queued. The relays
0xca and 0xcb are `0x61a190` (tag 0x12, then as `0x61a130`) and `0x61a1f0` (tag 0x13, then
`0x7c6e80` and a link). For the client's own mover, `0x619fb0` and `0x61a070` queue tag 0x12 or
0x13 with byte 1 unless a spline is running; with one, they apply at once (`0x61a130`,
`0x61a160`) and then send 0xca or 0xcb through `0x60e0a0` with its send flag 1. `0x61a030` and
`0x61a0f0` queue the tags with byte 0.

Addresses: `0x61a230`, `0x61a130`, `0x61a160`, `0x61a190`, `0x61a1af`, `0x61a1f0`, `0x619fb0`, `0x61a070`, `0x61a030`, `0x61a0f0`, `0x7c6e50`, `0x7c6e80`, `0x60e0a0`
Confidence: verified
Proof: one
Test: none

`0x61a230` is `call 0x7c6e50; jmp 0x61ab20`; `0x7c6e50` sets 0x200000, clears 0x4000000 and ends a
fall. `0x61a1af` calls it after `0x618f20` with
`push 0; push 0x12`. `0x619fb0`: `test byte [eax+0x18],4; je 0x619ffe` for a running spline; the
queue arm pushes `0, 0, &zero, 0, 1, 0x12, time`; the other arm calls `0x61a130` and pushes
`0, 0, 1, 0xca, time` for `0x60e0a0`. `0x61a070` is the same with 0x13, `0x61a160` and 0xcb.
`0x61a160` ends with `jmp 0x619ca0` when `test [esi+0x40],0x20ff` is nonzero or the queue is not
empty. The opcodes 0x30b and 0x30c reach `0x61a130` and `0x61a160` through the spline-mode
switch.

## Pitch start and stop

Pitch up is 0x40 and pitch down 0x80, set only while swimming. `0x61a290`, for opcodes 0xbf and
0xc0, applies tag 0xb (up) or 0xc (down) through `0x618f20`; applied, `0x7c6df0(up)` sets one bit
and clears the other when swimming and re-anchors, and the mover is linked. `0x61a320`, for 0xc1,
applies tag 0xd; applied, `0x7c6e30` clears both bits, returning 1 if one was set, and an idle
mover with an empty queue is unlinked; it returns that result, or 0 when deferred. The own forms
`0x61a240(time, up)` and `0x61a2e0(time)` queue tags 0xb/0xc and 0xd with byte 1.

Addresses: `0x61a290`, `0x61a320`, `0x61a240`, `0x61a2e0`, `0x7c6df0`, `0x7c6e30`
Confidence: verified
Proof: one
Test: none

The tag is `neg; sbb; add 0xc`: 0xb for a nonzero `up`. `0x7c6df0` tests 0x200000 and stores
`& 0xffffff7f | 0x40` or `& 0xffffffbf | 0x80`. `0x61a320` keeps `0x7c6e30`'s result in `edi`.
The opcodes reach these through the owner's forwarders from `0x601580`'s table, 0xbf and 0xc0
sharing one forwarder with a constant first argument.

## Water walk and feather fall

Water walk is 0x10000000 and feather fall 0x20000000. The relay `0x61a430`, opcode 0x2b1, applies
tag 0x21 when the status has water walk, else 0x22; applied, a mover now without water walk starts
a fall and, when the fall starts, is linked. The relay `0x61a550`, 0x2b0, applies tag 0x1d or 0x1e
by the status's feather fall and does nothing more: the merge already set the bit. The spline
forms set the bit at once: `0x61a3d0` (0x309, 0x30a) and `0x61a4e0` (0x305, 0x306), which also
restarts a running fall's vertical speed through `0x7c72b0`. The forced forms queue with byte 1
and the counter: `0x61a380` (0xde, 0xdf; tags 0x21, 0x22) and `0x61a490` (0xf2, 0xf3; 0x1d, 0x1e).

Addresses: `0x61a430`, `0x61a550`, `0x61a3d0`, `0x61a4e0`, `0x61a380`, `0x61a490`, `0x61a45c`, `0x7c7280`, `0x7c72e0`, `0x7c72b0`
Confidence: verified
Proof: one
Test: none

`0x61a43c`: `and ecx,0x10000000; neg; sbb; add ecx,0x22`. `0x61a45c` tests the mover's flags after
the merge, then `0x7c61c0` and `0x619ca0`. `0x61a550` ends with `call 0x618f20; ret 8`.
`0x61a4e0` calls `0x7c72e0(on)` and `0x7c72b0`, which, falling, reloads `+0xa0` from `0x7c5d70`,
zeroes `+0x78` and copies the height to `+0x7c`. The opcodes: 0x2b0 and 0x2b1 by compares in
`0x601580`; 0x305 to 0x30a in the spline-mode switch; 0xde, 0xdf, 0xf2, 0xf3 in `0x602780`.

## Hover

Hover is 0x40000000. `0x61a620(on)`, for spline opcodes 0x307 and 0x308 and after the relay, jumps
when enabling, through `0x7c6230` with argument 0 (pushed at `0x61a62e`, called at `0x61a630`),
or starts a fall when disabling (`0x61a637`); then links the mover and sets or clears the bit. The
relay `0x61a6b0`, opcode 0xf7, applies tag 0x1f or 0x20 by the status's hover and, applied, calls
`0x61a620` with the merged bit. The forced form `0x61a5d0` (0xf4, 0xf5) queues tag 0x1f or 0x20
with byte 1; its replay `0x61a580(on, launch)` jumps with argument 1 or, disabling, starts a fall
and on success calls the owner's `0x602c40`, but only for a nonzero `launch`; then sets the bit.

Addresses: `0x61a620`, `0x61a62e`, `0x61a630`, `0x61a637`, `0x61a6b0`, `0x61a5d0`, `0x61a580`, `0x61a597`, `0x7c6230`, `0x7c61c0`, `0x7c7310`, `0x602c40`
Confidence: verified
Proof: one
Test: none

`0x7c6230(arg)` refuses when `arg` is nonzero and hover is set, when a running spline has flag
0x200, and when flags & 0x3000; otherwise it starts a fall with vertical speed −9.096748
(`0xc1118c48`) while swimming or −7.955547 (`0xc0fe93d8`). Its four callers pass 1 except
`0x61a630`. `0x61a580` tests `launch` first (`je 0x61a5be`), then `on`; `0x61a597` calls
`0x7c6230(1)`. `0x61a6b0` passes `[esi+0x40] & 0x40000000`. `0x602c40` passes 40 to a unit call
when the unit's movement flags have 0x4000 and the dword `+0x40` of the block at unit `+0x110` is
positive. The drain and the replay call `0x61a580` with `on` 1 or 0.

## State-change broadcaster

`0x61a820(t, skipSpline, old, fallMoving, heartbeat)` reports the mover's flag change against
`old`. The owner is the active mover when its GUID equals `0xc4da98`; a spline is idle when absent
or done. Steps, in order:

| step | condition | effect |
|---|---|---|
| 1 | `skipSpline` 0, running spline, flags & 3, spline flag 1 | stop (`0x7c6c00`), snap to spline `+0x58`, face by spline flag, complete, `0x60e0a0(t, 0xb7, 0)`; go to 3 |
| 2 | otherwise a done spline | snap to spline `+0x58`, complete (`0x619de0`) |
| 3 | old had 0x2000, flags lack it | landing hook `0x602d00(t, old, fallMoving)`; nonzero skips to 6 |
| 4 | `heartbeat`, active mover, spline idle | send 0xee through `0x600a30`; go to 6 |
| 5 | (flags ^ old) & 0xf, active mover, spline idle | send `0x619f00(old, fallMoving)` through `0x600a30` |
| 6 | 0x2000 or 0x4000 newly set | the owner's `0x602c40` |
| 7 | 0x200000 cleared, active mover, spline idle | `0x60e0a0(t, 0xcb, 1)` |

Addresses: `0x61a820`, `0x61a859`, `0x61a8f5`, `0x61a947`, `0x61a990`, `0x61a99d`, `0x61a9e6`, `0x61a9eb`, `0x61aa55`, `0x616d60`, `0x47f220`, `0x468460`, `0x600a30`, `0xc4da98`
Confidence: verified
Proof: one
Test: none

Step 1's facing: spline flag 0x40000 takes spline `+0x14`; 0x20000 the heading from `+0x10` to the
object of GUID spline `+0x00` (`0x468460`, then its vtable `+0x14` position); 0x10000 the heading
to the point at spline `+0x08` (`0x47f220`). `0x61a8f5` pushes 0xb7 with send flag 0, which
`0x60e0a0` passes to no send (`je 0x60e300` at `0x60e1c8`). `0x61a990` pushes 0xee and `0x61a9e6`
calls `0x600a30` with `ecx` the owner; `0x600a30` builds the message for the opcode and hands it
to the network layer, read as far as those calls. `0x61a99d` is `test al,0xf` on `flags ^ old`, so only the
direction nibble opens step 5. Step 6, from `0x61a9eb`: `test ah,0x20` and `test bh,0x20`, then
`test ah,0x40` and `test bh,0x40`. `0x61aa55` pushes 0xcb with send flag 1.

## Opcode from a flag change

`0x619f00(old, noJump)` picks a movement opcode from the mover's flags against `old`; the first
row that holds decides.

| condition | opcode |
|---|---|
| (flags ^ old) & 3; flags & 1, else & 2, else neither | 0xb5, 0xb6, 0xb7 |
| (flags ^ old) & 0xc; flags & 4, else & 8, else neither | 0xb8, 0xb9, 0xba |
| flags & 0x2000, `+0xa0` ≠ 0.0 or NaN, `noJump` 0 | 0xbb |
| (flags ^ old) & 0x200000; flags swimming, else not | 0xca, 0xcb |
| otherwise | 0xb7 |

Its one caller, step 5 of the broadcaster, runs it only when the direction nibble changed, so it
returns 0xb5 to 0xba there.

Addresses: `0x619f00`, `0x619f2a`, `0x619f50`, `0x619f71`, `0x619f98`
Confidence: verified
Proof: one
Test: none

`not dl; movsx eax,dl; and eax,2; or eax,0x16c; shr eax,1` at `0x619f2a` gives 0xb6 when flags
have 2 and 0xb7 otherwise. `and dl,8; neg dl; sbb edx,edx; add edx,0xba` at `0x619f50` gives
0xb9 or 0xba. `test ah,0x44; jnp` at `0x619f71` skips the jump arm only on equal. `not eax;
and eax,0x200000; or eax,0x19400000; shr eax,0x15` at `0x619f98` gives 0xca when swimming, 0xcb
when not. The sole call is `0x61a9da`.

## Stopping every motion

`0x61aa70` stops a mover: it ends a fall (`0x7c6290`), clears the latches and 0x4000000
(flags & 0xfbe07fff), stops turning (`0x7c6dd0`) and pitching (`0x7c6e30`), each reported as 0xbe
or 0xc1 when it changed something, stops strafing (`0x7c6ce0`) with 0xba when flags & 0xc, then
stops moving (`0x7c6c00`) and clears 0x2000 and 0x4000. The reports go to `0x60e0a0` at the update
time with send flag 0, which runs the owner's local reaction and sends nothing.

Addresses: `0x61aa70`, `0x61aa7b`, `0x61aaa6`, `0x61aacd`, `0x61ab02`, `0x61ab14`, `0x7c6dd0`, `0x7c6ce0`, `0x7c6c00`
Confidence: verified
Proof: one
Test: none

`and edx,0xfbe07fff` at `0x61aa7b`. The three reports push `0, 0, 0, opcode, [globals+0x128]` at
`0x61aaa6`, `0x61aacd` and `0x61ab02`. `0x7c6dd0` tests `al,0x30`, clears it, re-anchors and
returns 1. `and [esi+0x40],0xffff9fff` at `0x61ab14`. No move stop (0xb7) is reported.

## Deferred direction latches

While falling, the movement commands record a press in latch bits instead of the direction:
0x8000 stop, 0x10000 stop strafe, 0x20000 or 0x40000 start forward or back, 0x80000 or 0x100000
start strafe. `0x61ab20` applies them in that order through `0x7c6c00`, `0x7c6ce0`,
`0x7c6ae0(flags & 0x20000)` and `0x7c6c50(flags & 0x80000)`, clears bits 0x1f8000 and returns
`(flags ^ entry flags) & 0xf`.

Addresses: `0x61ab20`, `0x61ab2b`, `0x61ab73`, `0x7c6ae0`, `0x7c6c50`
Confidence: verified
Proof: one
Test: none

`test ah,ah; jns` at `0x61ab2b` reads bit 15. `test eax,0x60000` and `test eax,0x180000` gate the
two starts. `and eax,0xffe07fff` at `0x61ab73`, then `xor eax,edi; and eax,0xf`. `0x7c6ae0`,
falling with a direction set, stores 0x20000 for a nonzero argument and 0x40000 for zero instead
of the direction.

## Opcode 0x2ca report

`0x61a800(t)` returns the owner's `0x602cd0(t)`, which sends opcode 0x2ca through `0x600a30` when
the owner's byte `+0xc58` has bit 7 set and returns 0 otherwise. The resolution layer calls it.

Addresses: `0x61a800`, `0x602cd0`
Confidence: verified
Proof: one
Test: none

`0x61a800`: `mov ecx,[ecx+0x15c]; push eax; call 0x602cd0; ret 4`. `0x602cd0`: `test al,al; js`,
then `push 0; push 0; push 0x2ca; push t; call 0x600a30`. The caller is `0x635a59`.

## Unreached functions

Five functions of the file have no caller: `0x619b50` (the gravity apply with an owner report),
`0x619c20` (set gravity, then `0x619aa0` when the bit changed), `0x61a3e0` (own water walk, byte
0), `0x61a500` (own feather fall, byte 0) and `0x61a660` (own hover, byte 0). `0x61a3e0` starts
right after the `ret 4` of `0x61a3d0`, with no padding.

Addresses: `0x619b50`, `0x619c20`, `0x61a3e0`, `0x61a500`, `0x61a660`
Confidence: verified
Proof: one
Test: none

Census: every `e8` and `e9` in `.text` whose 32-bit target is the address, and every
little-endian occurrence of the address anywhere in the file. All five give zero of each. Controls on the same scan:
`0x514120` occurs once as a stored address, `0x619280` once as a call target (`0x619920`). Each
address follows a `ret` or a `jmp`, so nothing falls into it.
