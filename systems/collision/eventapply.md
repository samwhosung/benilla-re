# collision: event applies

What the queue's consumers and the relay timing do with an event, in `Movement.cpp` beside the
queue (`collision/events`). The flush `0x6185f0` empties a movement's queue at once through the
dispatcher `0x618080`, which applies only mode, speed, root and teleport changes and their
acknowledgements; its callers precede a placement, a spline stop or a new spline path. The
knockback launch and the teleport apply serve both the per-frame drain and the apply-now path.
The transport setter boards and leaves a transport, the lateness ring and the late-start step
time a remote unit's replay, and three setters store a spline's facing data. The unit's movement
code outside the system reaches the flush callers, the knockback and teleport producers and
applies, and the spline setters; `0x618680` and `0x6188c0` are unreached.

Roots: `0x617a30`, `0x617a60`, `0x6186b0`, `0x618720`, `0x618750`, `0x6187a0`, `0x618840`,
`0x618870`, `0x6188a0`

## Queue flush

`0x6185f0(keep, endSpline)` zeroes +0x148 and +0x14c and dispatches every queued event at once
through `0x618080`, first to last, whatever its time. With `keep` 0 it saves flag 0x800, holds it
set during the flush and restores it after. Its callers:

| caller | flush | then |
|---|---|---|
| `0x617fc0(arg)` | (0, arg) | `0x619e80(arg)` |
| `0x618680(position, facing)`, unreached | (0, 1) | `0x617e90(0, 0, position, facing, 1, 0)` |
| `0x618750(a, point)` | (0, 0) | `0x61aa70`, `0x619300(point)`, `0x7c6a20(a)`, `0x616d30`, `0x619de0` |
| `0x6187a0(points, n, …)` | (0, 0) | `0x61aa70`, `0x619e80(0)`; with 0x1000 set returns 0 |

Unrooted, `0x6187a0` gives the spline at +0xa4 the last point (`0x7c62c0`), calls `0x7c6a50`,
which hands the points to the path object, activates the movement, calls `0x453390(1.0,
spline+0x58, 1)` on spline +0x2c and returns 1; what the path object and `0x453390` do with them
is not read here.

Addresses: `0x6185f0`, `0x617fc0`, `0x618680`, `0x618750`, `0x6187a0`, `0x6187c2`
Confidence: verified
Proof: one
Test: none

`0x618607 and edi,0x800; or eax,0x800` runs only for `keep` 0, and `0x618661 or edi,0xfffff7ff;
and eax,edi` restores. The loop `0x618633` to `0x618654` passes the first node (`0x6158e0`),
`keep`, `endSpline` and the saved bit. In `0x6187a0`, the flush, `0x61aa70` and `0x619e80(0)`
precede `0x6187c2 test ah,0x10`; the last point is `lea ecx,[ebx+eax*4-0xc]` with `eax = 3n`,
and `0x7c6a50` gets the time at globals +0x128 and all five arguments. `0x619d50` flushes with
`keep` 1. With `esi` the spline (`0x61880d`), `0x618821 call 0x453390` follows `push 1;
lea ecx,[esi+0x58]; push ecx; push 0x3f800000; lea ecx,[esi+0x2c]`. The meaning of `0x618750`
rests on callees read here only in part.

## Flush dispatcher

`0x618080(event, keep, endSpline, saved)` pops the queue's first node, which its caller passes
as `event`, then switches on its tag. The
send helper `0x617fe0` calls `0x60e0a0(time, opcode, +0x4d, value, +0x48)` on the unit at
`CMovement+0x15c`, the time from movement globals +0x12c; with `keep` 0 it puts flag 0x800 back
to its saved value for the call and sets it again after.

| tags | index | case | action |
|---|---|---|---|
| 6 | 0 | `0x6180be` | with `keep` nonzero, `0x7c61c0`, and when it starts a fall a unit hook |
| 14, 15 | 1, 2 | `0x6180ea`, `0x6180fa` | `0x7c71c0(1)`, `0x7c71c0(0)`, no send |
| 20 to 25 | 3 to 8 | `0x61810a` to `0x618213` | speed setter and send |
| 29 to 36 | 9 to 16 | `0x618248` to `0x618472` | mode toggle and send |
| 38 | 17 | `0x6184d0` | teleport and send 0xc7 |
| every other | 18 | `0x61851e` | none |

Addresses: `0x618080`, `0x618528`, `0x618574`, `0x61851e`, `0x617fe0`, `0x6184d0`
Confidence: verified
Proof: one
Test: none

`0x6180a1 mov eax,[esi+0xc]; add eax,-6; cmp eax,0x20; ja 0x61851e`, then `movzx eax,byte
[eax+0x618574]; jmp [eax*4+0x618528]`. The 19 dwords are `0x6180be 0x6180ea 0x6180fa 0x61810a
0x61813f 0x618174 0x6181a9 0x6181de 0x618213 0x618248 0x61828e 0x6182d1 0x618334 0x618394
0x6183c8 0x618425 0x618472 0x6184d0 0x61851e`. The 33 index bytes, tags 6 to 38: `00, 12 × 7,
01 02, 12 × 4, 03 to 08, 12 × 3, 09 to 10, 12, 11` (hex). Tags below 6 wrap to large values and
take `ja`. The teleport case passes +0x28 and +0x2c when +0x4c is set, else 0 and 0, with
+0x10, +0x1c, `endSpline` and +0x4d (`0x6184d0`). `0x617fe0` tests its fourth argument at
`0x617fe8` and `0x61801d`.

## Knockback launch

`0x6179c0(direction, speedXY, speedZ)` launches in this order: when flag 0x200000 is set, it
calls `0x7c6e80`; then `0x7c61f0(speedZ)`, which re-anchors through `0x7c5cd0(0)`, then sets
0x2000, clears 0x4000000 and 0x200000, zeroes +0x78, copies +0x18 to +0x7c and stores speedZ at
+0xa0. It copies the direction pair to
+0x68, +0x6c and to +0x5c, +0x60 with 0 at +0x64, clears flag 0x2, sets 0x8001 whatever the
speeds, and stores speedXY at +0x84. `0x617a30` queues tag 28 for the drain, which calls
`0x6179c0` with +0x38, +0x40 and +0x44 (`0x61624d`) and then sends 0xf0 (`0x616261`).
`0x617a60` launches at once when `0x618eb0` accepts the status, then activates the movement.

Addresses: `0x6179c0`, `0x617a0f`, `0x617a18`, `0x7c61f0`, `0x617a30`, `0x617a60`, `0x61624d`, `0x616261`
Confidence: verified
Proof: one
Test: none

`0x6179c9 test [esi+0x40],0x200000` guards `call 0x7c6e80`. `0x7c6206 and eax,0xfbdfffff; or
eax,0x2000` and the stores at `0x7c6213` to `0x7c621d`. The flag word is read after the fall
start (`0x617a0c`), so `0x617a0f and edx,0xfffffffd` and `0x617a18 or edx,0x8001` act on it
with 0x2000 set; `0x617a1e` stores speedXY. `0x6179c0` itself runs no x87 instruction: its
copies are integer moves, so no value is rounded. `0x7c61f0` calls `0x7c5cd0` (`0x7c61f8`)
before its flag store (`0x7c6210`), so the direction it computes into +0x5c through `0x7c5a20` from
the old flags is overwritten by the direction pair. The send mask 0x75a07dff
keeps 0x1 and clears 0x8000. `0x617a60` returns 1 after `0x619ca0` (`0x617a98`), 0 when queued.

## Teleport apply

`0x617e90(transport, position, facing, endSpline, send)` sets the transport through `0x617170`
and stops unless `CMovement+0x38` then equals the given GUID. It then computes
`d² = (dz² + dy²) + dx²` of the old position +0x10 minus the new one, at 53 bits; places it
(`0x7c69a0`); deactivates it when the queue is empty; unless `send` is nonzero and the unit
is not the one this client moves, starts a fall and activates the movement when one starts;
re-resolves the world position (`0x7c4930`, `0x634030`); calls `0x6309f0`, and `0x630bd0` when
`d²` exceeds 900.0; and zeroes +0x148 and +0x14c. `0x6186b0` applies tag 38 with the status's
transport-local pose, or its world pose off a transport; `0x618720` queues tag 38 with a
counter, position and facing.

Addresses: `0x617e90`, `0x617ead`, `0x617ef1`, `0x617fa9`, `0x6186b0`, `0x6186bd`, `0x618720`, `0x618736`
Confidence: verified
Proof: one
Test: `collision::facing_pos_gate_difftest`

```
617ee5: fld    DWORD PTR ds:0x80c730
617eeb: fmul   DWORD PTR ds:0x80c730
617ef1: fcompp
617ef9: test   ah,0x5
617efe: jp     0x617f07
```

The constant is 30.0 (`0x41f00000`). `fcompp` compares 900.0 with `d²`: 900 below `d²` sets C0
alone and falls through to the far flag; equal or above leaves C0 and C2 clear and NaN sets both,
and all three take `jp`. The
two GUID compares are at `0x617ead` and `0x617eb8`. `0x6186bd push 0x26` feeds `0x618f20`;
`0x6186e0` picks status +0x14 and +0x20 with the GUID, +0x24 and +0x30 without, and passes 1
and 0 as the last two arguments. `0x618736 push 0x26` and `push 1` feed `0x617570`.
The tests run `0x617ebe` to `0x617f09`.

## Transport attach

`0x618970(guid)` returns 1 at once when the GUID is the current one. Leaving a transport, it
reads that transport's angle (`0x630b70`), unlinks the movement's rider link at +0x08, zeroes
+0x38 and +0x3c, clears flag 0x8000000 (`0x7c6510`), clears the model's parent frame and passes
the angle to `0x601050` on the unit. Boarding, it returns 0 when `0x6309b0` refuses; otherwise it
stores the GUID, takes the negated angle (`fchs`, stored as a float), fills an identity matrix
through `0x630ac0`, sets it as the model's parent frame and passes the negated angle. The model is
the unit's +0xdc, else +0xd8. `0x618b30(status)` passes the GUID `0x7c6900` takes from a status.

Addresses: `0x618970`, `0x618a59`, `0x618b30`, `0x7c6510`, `0x7c6900`
Confidence: inferred
Proof: none
Test: `collision::negate_facing_618970_difftest`

`0x61898d je 0x618b1a` is the early return. The leave leg runs `0x618993` to `0x618a29`, the
board leg `0x618a2c` to `0x618b15`; a zero new GUID returns 1 at `0x618a30`. `0x618a59 fchs` and
`fstp [ebp+0xc]` store the negated angle as a float; the matrix is 16 floats from `0x618a63`,
1.0 on the diagonal. The callees' effects on frames and facing are not read here.
The tests run `0x618a4c` to `0x618a5f`, with `0x630b70` stubbed.

## Lateness ring

`0x618b50(wireDelta, localDelta)` stores `localDelta − wireDelta + base` (base at +0x140) in
slot +0x13c of the ring at +0xbc, advances the slot modulo 32, and returns the largest of all 32
slots, the new one included, as signed 32-bit integers. The constructor `0x616fa0` fills slots 0
to 15 with −50 and 16 to 31 with 50, and sets the base to 50 and the slot to 0 (`0x616fbc`), so
a fresh unit's maximum stays at least 50 until 32 samples have replaced them. `0x618c30` passes the status time's forward
step, 0 when it does not advance, and the local time since the last fire.

Addresses: `0x618b50`, `0x618b68`, `0x618b94`
Confidence: verified
Proof: one
Test: none

```
618b59: mov    eax,DWORD PTR [ebp+0x8]
618b5c: sub    eax,edx
618b5e: mov    edx,DWORD PTR [ecx+0x13c]
618b65: sub    eax,DWORD PTR [ebp+0xc]
618b68: neg    eax
```

`edx` holds +0x140 at `0x618b5c`, so the store is `−(wireDelta − base − localDelta)`. The
constructor runs `rep stos` of 0xffffffce and then 0x32 with a count of 16 from +0xbc and +0xfc,
and stores 0x32 at +0x140 (`0x617026` to `0x617044`). The loop
from `0x618b90` over 32 dwords keeps the larger (`cmp ecx,eax; jle`, `0x618b94`). `ret 8`.

## Late start catch-up

`0x618bb0(time, late)` does nothing for `late` 0, for a unit with no flag of 0x200f, or for one
with a live spline whose bit 0x4 is clear. Otherwise it adds `late` to +0x58 when a flag of 0xf
is set, calls `0x7c5360(+0x58, 0x618920())`, and steps the unit with `0x616cb0(time, +0x58, 0)`.
`0x618920` returns the address of +0x144 when the float at +0x148 is nonzero or NaN, else 0.
`0x618c30` calls `0x618bb0` with the fire time and the movement clock minus it.

Addresses: `0x618bb0`, `0x618be0`, `0x618920`, `0x618926`
Confidence: verified
Proof: one
Test: none

`0x618bb9 test edx,edx; je` on `late`, `0x618bc3 test ecx,0x200f`, then `0x618bd5 test byte
[eax+0x18],4; je` with a spline. `0x618be0 add [esi+0x58],edx` under `test cl,0xf`. The third
argument of `0x616cb0` is a zeroed vector. `0x618926 fcomp 0.0; test ah,0x44; jnp` returns 0 only
for equality, C3 alone; NaN sets C3 and C2 and returns the address.

## Spline facing setters

Each setter acts on the spline at `CMovement+0xa4`. `0x618840` sets its flag 0x10000 and copies a
point to its +0x08; `0x618870` sets 0x20000 and copies a GUID to +0x00; `0x6188a0` sets 0x40000
and stores an angle at +0x14. The three are the arms of one switch on a byte of the monster-move
data, and the names read the three values as a facing point, target and angle. `0x6188c0(on)`
sets or clears 0x80000000 only when a spline exists, its bit 0x4 is clear and its bit 0x200 is
set.

Addresses: `0x618840`, `0x618870`, `0x6188a0`, `0x6188c0`, `0x6188d4`
Confidence: verified
Proof: one
Test: none

`0x618849 or [eax+0x18],0x10000`, `0x618879 or [eax+0x18],0x20000`, `0x6188a9 or
[eax+0x18],0x40000`, each `ret 4`. `0x6188d0 test al,4; jne` and `0x6188d4 test ah,2; je` gate
`or eax,0x80000000` or `and eax,0x7fffffff`; no spline returns at once (`0x6188cb`). A call
census finds one site each: `0x6188a0`, `0x618870` and `0x618840` at 0x602005, 0x602012 and
0x60201f, the byte minus 2 selecting 0x618840 for 0, 0x618870 for 1 and 0x6188a0 for 2. That the
values aim the facing is inferred from those names.

## Unreached functions

`0x618680` and `0x6188c0` are reached only from unreached code. `0x618680`'s one caller is
`0x602f30` and `0x6188c0`'s one caller is `0x602650`, and nothing reaches either caller.

Addresses: `0x618680`, `0x6188c0`, `0x602f30`, `0x602650`
Confidence: verified
Proof: one
Test: none

A census of `e8` and `e9` targets and of every little-endian dword in the image finds
`0x618680` only at 0x602f4d and `0x6188c0` only at 0x60265d, and nothing for `0x602f30` or
`0x602650`, neither of them as an immediate operand. Each follows a `ret` and padding, and no
jump lands on it. Control: the same census finds `0x514120` at 0x850154.
