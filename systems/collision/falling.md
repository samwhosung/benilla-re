# collision: falling

The vertical half of movement: the closed-form fall arc, the jump, starting and ending a fall,
the position and fall fields of the movement status record, and the change of frame when a mover
boards or leaves a transport. The code is the middle of the shared movement object file, methods
of `CMovement`, the per-unit movement object.

An arc is three fields: the elapsed fall time `+0x78` in milliseconds, the start height `+0x7c`,
and the launch speed `+0xa0`, down-positive. Every speed and height on the arc is computed from
them in closed form; nothing integrates the vertical per frame. Two `.data` floats hold the
terminal speeds. The rest of the client reaches this part through the rising test `0x7c5de0`
(the swim decision) and the fall height `0x7c60c0` (the landing handler); everything else here
is called from collision's mover, command queue, status handlers and resolvers. The movement
commands in the same file are the `commands` topic.

Roots: `0x7c5de0`, `0x7c60c0`

## Arc fields and constants

The vertical speed is down-positive: a positive `+0xa0` falls and a jump launches at a negative
one. A body t seconds into the arc is at `+0x7c` − D(t). Gravity g is 19.291105 (`0x419a542f`,
at `0x81da58`). The cap T is `[0x87d894]`, 60.148003 (`0x4270978e`), or `[0x87d898]`, 7.0
(`0x40e00000`), whenever flag 0x20000000 is set. The other constants are the float roundings of
g's multiples:

| address | value | dword | is |
|---|---|---|---|
| `0x81da60` | 9.6455526 | `0x411a542f` | g/2 |
| `0x81da64` | 38.582211 | `0x421a542f` | 2g |
| `0x80e020` | 0.051837362 | `0x3d54536a` | 1/g |
| `0x81da5c` | −0.051837362 | `0xbd54536a` | −1/g |
| `0x81da68` | 0.10367472 | `0x3dd4536a` | 2/g |
| `0x81da6c` | 0.025918681 | `0x3cd4536a` | 1/(2g) |
| `0x81da70` | −2⁻²² | `0xb4800000` | rising limit |

Addresses: `0x81da58`, `0x87d894`, `0x87d898`, `0x7c5f30`, `0x633a42`, `0x7c5d23`, `0x7c61b0`
Confidence: verified
Proof: one
Test: none

`0x7c5f30` returns `+0x18` + D(ms) (`0x7c5f3c` `fadd [ecx+0x18]`), and its four callers store
that at `+0x7c` (`0x616bcc`, `0x61a7dc`, `0x7c64b2`, `0x7c6974`), so the body sits D below the
start. The resolver `0x633840` applies a step as a subtraction: `0x633a42` `d8 e1`,
`fsub st,st(1)`, takes `0x7c6140`'s drop from the target z.
`0x7c5e70` adds g·t to the speed and caps it from above, so a fall grows positive, and the jump
stores −7.955547. The caps are `.data`; their only writers, `0x7c616c` and `0x7c618c`, are in
unreached code, so they keep their file values. Census: the image holds the two addresses as
dwords only inside the twelve instructions that load or store them, from `0x7c5d2c` to
`0x7c61b0`. The selection is `test [ecx+0x40],0x20000000` at `0x7c5d23`, `0x7c5e74` and
`0x7c5f59`. `0x7c61b0` returns `[0x87d898]`; its one caller is the walk resolver `0x6367b0`.
60.148 is 55 × 1.0936, the factor the unreached setters apply.

## Fall speed at a time

`0x7c5d20(t)` returns the vertical speed t seconds into the arc. With v = `+0xa0` and T the cap:
vc = T when v > T, else v (NaN keeps v); w = float(t·g + vc); it returns T when T < w, else w,
NaN included. The sum is rounded to float before the compare. `0x7c5d70` calls it with
t = float(`+0x78` · 0.001), the count read as unsigned. Its one user is `0x7c72b0`: while 0x2000
is set it stores the result at `+0xa0`, zeroes `+0x78` and copies z to `+0x7c`, restarting the
arc from the current height at the current speed. `0x61a4e0` runs it right after the feather-fall
setter `0x7c72e0`, so a changed cap takes hold from that moment.

Addresses: `0x7c5d20`, `0x7c5d70`, `0x7c72b0`, `0x7c5d40`, `0x7c5d58`, `0x7c5d62`, `0x61a4e0`
Confidence: verified
Proof: one
Test: `collision::gravity_fall_difftest`, `collision::gravity_integrate_trajectory`

```
7c5d4d  fld   dword [ebp+8]       ; t
7c5d50  fmul  dword [0x81da58]    ; g
7c5d56  fadd  st,st(1)            ; d8 c1: + vc
7c5d58  fstp  dword [ebp+8]       ; w, rounded to float
7c5d5b  fstp  st(0)               ; T left
7c5d5d  fcom  dword [ebp+8]
7c5d62  test  ah,5
7c5d65  jnp   0x7c5d6c            ; C0 alone: T < w, return T
```

The first cap test, `0x7c5d40` `fcom st(1)`, `test ah,0x41`, `jne`, keeps v for less, equal and
unordered. At `0x7c5d62` odd parity of C0 and C2 is C0 alone; equal and unordered fall through to
`fld [ebp+8]`, w. `0x7c5d70` loads the time with `fild qword` over `+0x78` and a zero high dword,
`fmul` 0.001 (`0x3a83126f`) and `fstp` into the argument slot. `0x7c72b0` tests `ah,0x20` and
stores at `0x7c72bd`, `0x7c72c6` and `0x7c72cd`.

## Fall distance

`0x7c5e70(t)` returns the drop D(t) at 53 bits. With vc as in `0x7c5d20`: while vc + g·t is
below or equal to T, D = (vc + (g/2)·t)·t. Above it, or NaN, with tT = (T − vc)·(1/g),
D = (vc + (g/2)·tT)·tT + (t − tT)·T: the parabola to the cap, then straight at T. `0x7c5f00(ms)`
calls it with t = float(ms · 0.001). `0x7c5f30(ms)` returns z + D. `0x7c6110(ms, z)` returns D,
less (`+0x7c` − z) while 0x2000 is set. `0x7c6140(ms)` passes the mover's own z: the further
drop from the current height to the arc at ms. The resolvers `0x633840` and `0x635b00` call it
with `+0x78` plus the step's milliseconds (`0x633a31`, `0x635b2e`), and `0x635f80` too.

Addresses: `0x7c5e70`, `0x7c5f00`, `0x7c5f30`, `0x7c6110`, `0x7c6140`, `0x7c5eb8`, `0x7c5ece`, `0x633840`, `0x635b00`, `0x635f80`
Confidence: verified
Proof: one
Test: `collision::gravity_fall_difftest`

The branch is `0x7c5eae` `fadd` (vc + g·t), `fcomp [ebp−4]` (T), `test ah,0x41`,
`jp 0x7c5ece` at `0x7c5eb8`: even parity of C3 and C0, taken for greater and unordered. The
quadratic is `fld t`, `fmul g/2`, `faddp`, `fmul t`. The capped block, by opcode: `0x7c5ed1`
`d8 e1` T − vc; `fmul 1/g`; `fld g/2`, `fmul st,st(1)`; `faddp st(2)` vc + (g/2)·tT; `fxch`,
`fmul st,st(1)` by tT; `0x7c5eea` `d8 e2` t − tT; `fmul T`; `faddp`; `fstp st(1)`. Every step is
on the stack; the result is returned unrounded. `0x7c6110` tests `ah,0x20` at `0x7c6116` and
forms z − `+0x7c` with `0x7c6134` `d8 61 7c` before `faddp`; without 0x2000 it returns D alone.

## Fall time for a drop

`0x7c5f50(d, first)` returns the arc time at which the drop reaches d, inverting `0x7c5e70`; the
fall resolver `0x635450` calls it. With v = min(`+0xa0`, T) as in `0x7c5d20`:

- |v| below 2⁻²²: tT = (1/g)·T and dT = tT·T·0.5. For d < dT it returns √(d·(2/g)), or 0 for
  d ≤ 0; otherwise, NaN too, (d − dT)/T + tT. `first` is not read.
- otherwise: s = √(v·v + d·2g), or 0 when that sum is not positive; t2 = (s − v)·(1/g); when
  t2 exceeds tT = float((T − v)·(1/g)), t2 = (d − (tT·(g/2) + v)·tT)/T + tT. With `first` 0 it
  returns t2, else t1 = float((−v − s)·(1/g)), raised to 0 when negative.

Addresses: `0x7c5f50`, `0x635450`, `0x7c5f97`, `0x7c5fc7`, `0x7c6004`, `0x7c6030`, `0x7c606b`, `0x7c609e`
Confidence: verified
Proof: one
Test: `collision::fall_physics_difftest`

`0x7c5f8d` `fabs`, `fcomp` 2⁻²² (`0x34800000`), `test ah,5`, `jp 0x7c6014` at `0x7c5f97`: taken
for greater, equal and unordered, so only |v| < 2⁻²² takes the low branch. There `0x7c5fc7`
sends d ≥ dT and NaN to `0x7c5fff`: `d8 e1` d − dT, then `de f3` at `0x7c6004`, `fdivrp
st(3),st`, which leaves st(0)/st(3) = (d − dT)/T, and `fadd` tT. The root guards are
`test ah,0x41; jne` at `0x7c5fe0` and `0x7c6030`: zero, negative and NaN give 0.0, though a NaN d
never reaches the first, `0x7c5fc7` having sent it to `0x7c5fff`. In the
general branch t1 and tT are stored as floats (`0x7c604b`, `0x7c6063`); t2 stays at 53 bits. The
cap test `0x7c606b` replaces t2 only when it is greater. The capped tail: `fmul g/2`, `fadd v`,
`fmul tT`, `0x7c6081` `d8 6d 08` `fsubr d`, `fdiv T`, `fadd tT`. `0x7c609e` `test ah,5; jp`
keeps t1 unless it is below 0. `ret 8`.

## Apex time and rising test

`0x7c5da0` returns v·(−1/g) for v = `+0xa0` while flag 0x2000 is set, else 0.0: the arc time at
which the vertical speed is zero. `0x7c5de0` returns 1 when 0x2000 is set, v ≠ 0 (NaN counts as
nonzero) and that time is strictly greater than t = float(`+0x78` · 0.001); otherwise 0. So it
holds 1 from the launch of an upward arc to its apex: 0.41239 s after a land jump and 0.47155 s
after a swim jump. Its one caller outside collision is the swim decision `0x6030c0`, which skips
the swim start while it returns 1 (`0x603132`).

Addresses: `0x7c5da0`, `0x7c5de0`, `0x7c5dfc`, `0x7c5e24`, `0x6030c0`, `0x603132`, `0x81da5c`
Confidence: verified
Proof: one
Test: `collision::gravity_fall_difftest`, `collision::fall_physics_difftest`, `objectlayer::move_event_pitch_rate`, `objectlayer::move_event_pitch_thresh`

`0x7c5da3` `test ah,0x20` selects `fld [ecx+0xa0]; fmul [0x81da5c]`. In `0x7c5de0` the speed
test is `fcomp 0.0`, `test ah,0x44` at `0x7c5dfc`, `jnp 0x7c5e32`: odd parity of C3 and C2 is
C3 alone, equality, which returns 0. The time is `fild qword` over `+0x78` and a zero dword,
`fmul` 0.001, `fstp [ebp−4]`. After `call 0x7c5da0`, `fcomp [ebp−4]`, `test ah,0x41` at
`0x7c5e24`, `jne 0x7c5e32` returns 0 for less, equal and unordered, so 1 needs −v/g > t. The
caller's `test eax,eax; jne 0x60314a` after `0x603132` jumps over the swim-start call on 1. The
two times are −v/g for −7.955547 and −9.096748.
The tests run `0x6030e0` to `0x6030f7` and `0x60314a` to `0x60315d`.

## Fall height from the apex

`0x7c60c0` returns h = `+0x7c` − z, plus v·v·(1/(2g)) when v = `+0xa0` is below −2⁻²², and 0.0
in place of a negative h; NaN is returned as it is. For a rising launch the added term is the
climb to the apex, so h is the height from the arc's highest point down to z; for a level or
falling launch it is the drop from the start. After a land jump the term is 1.64041 yd. The
landing handler `0x602d00` is its one caller and stores h as a float.

Addresses: `0x7c60c0`, `0x81da70`, `0x81da6c`, `0x602d00`, `0x7c60d7`, `0x7c60f8`
Confidence: verified
Proof: one
Test: `collision::gravity_fall_difftest`

`0x7c60c0` `d9 41 7c` `fld [ecx+0x7c]`, `d8 61 18` `fsub [ecx+0x18]`. `fcomp [0x81da70]`,
`test ah,5`, `jp 0x7c60ed` at `0x7c60d7`: taken for greater, equal and unordered, so the term is
added only for v < −2⁻²². The term is `fld st(0); fmul st,st(1)`, `fmul [0x81da6c]`,
`faddp st(2),st`. The clamp is `fcom 0.0`, `test ah,5`, `jp 0x7c6102` at `0x7c60f8`: C0 alone
falls through to load 0.0. Every step is at 53 bits until the caller's `fstp` at `0x602d24`.

## Starting a fall

`0x7c61f0(v)` re-anchors with `0x7c5cd0(0)` while the old flags still stand, then clears
0x200000 and 0x4000000, sets 0x2000, zeroes `+0x78`, copies z to `+0x7c` and stores v verbatim
at `+0xa0`. It leaves 0x4000 and the current speed `+0x84` alone. Because the anchor runs before
0x2000 is set, a fall begun on the ground recomputes the direction from the held bits, and one
begun in the air keeps it. `0x7c61c0` calls `0x7c61f0(0)` and returns 1, unless a spline is live
(present at `+0xa4`, its flags without 0x4 and with 0x200) or `+0x40` has any of 0x203800:
swimming, falling, root or 0x800 (`0x7c61d6`). Then it returns 0 and changes nothing.

Addresses: `0x7c61f0`, `0x7c61c0`, `0x7c61d6`, `0x7c5cd0`, `0x7c5a20`, `0x7c5880`
Confidence: verified
Proof: one
Test: none

`0x7c61f8` `call 0x7c5cd0` precedes `0x7c6206` `and eax,0xfbdfffff` and `0x7c620b`
`or eax,0x2000`; the stores run from `0x7c6210` to `0x7c621d`, the speed as a dword; `ret 4`.
`0x7c5cd0` hands its argument to `0x7c5a20`, which returns without touching the direction when
0x2000 is set and the argument is 0 (`0x7c5a2c` to `0x7c5a36`); otherwise it rebuilds `+0x5c`
through `0x7c5880`, which tilts it by the anchor pitch while 0x200000 is still set (`0x7c58b3`),
so a jump from swimming launches along the pitched swim direction. In `0x7c61c0`, `0x7c61c8` tests
for no spline, `0x7c61cd` `test al,4` and `0x7c61d1` `test ah,2` read the spline's `+0x18`, and
`0x7c61d6` is `test [ecx+0x40],0x203800`. A plain `ret`: `this` is the only input.

## Jump

`0x7c6230(check)` returns 0 and changes nothing when check is nonzero and hover, 0x40000000, is
set (`0x7c6236`, `0x7c623a`); when the spline test of `0x7c61c0` refuses; or when root or falling,
0x3000, is set (`0x7c625c`). Otherwise it starts the fall through `0x7c61f0` with the initial
vertical speed −7.955547 (`0xc0fe93d8`), or −9.096748 (`0xc1118c48`) while swimming
(`0x7c6261`), and returns 1. Swimming does not refuse a jump, and the jump clears it. A land jump
rises 1.64041 yd in 0.41239 s, a swim jump 2.14479 yd in 0.47155 s. The command paths pass
check 1; the hover handler `0x61a620` passes 0.

Addresses: `0x7c6230`, `0x7c6236`, `0x7c623a`, `0x7c625c`, `0x7c6261`, `0x7c6288`, `0x61a620`
Confidence: verified
Proof: one
Test: none

`0x7c6236` `test eax,eax; je 0x7c6243` skips the hover test for check 0. `0x7c625c` is
`test ah,0x30`. `0x7c6266` stores `0xc1118c48` in the argument slot and `0x7c626f` overwrites it
with `0xc0fe93d8` unless `0x7c6261` `test eax,0x200000` is nonzero. Both are dword immediates,
so the speed is exactly that float. Refusals return through `0x7c6288` `xor eax,eax`. The callers
push 1 at `0x615eba`, `0x61798d` and `0x61a595`, and 0 at `0x61a62e`. The rise is
v·v/(2g) and −v/g with the float g.

## Landing

`0x7c6290` acts only while 0x2000 is set: it clears 0x2000 and 0x4000, then re-anchors with
`0x7c5cd0(0)` and updates the current speed with `0x7c5c20(0)`, in that order. With 0x2000
already clear, both recompute: the anchor takes the landing position, the direction comes from
the held bits, and the speed from the mode. `+0x78`, `+0x7c` and `+0xa0` keep their values. Of the
file's `and` masks on the flags, only its own clears 0x4000.

Addresses: `0x7c6290`, `0x7c629b`, `0x7c5c20`
Confidence: verified
Proof: one
Test: none

`0x7c6296` `test ah,0x20; je 0x7c62b3`; `0x7c629b` `and eax,0xffff9fff`, stored before
`0x7c62a5` `call 0x7c5cd0` and `0x7c62ae` `call 0x7c5c20`. `0x7c5c20` skips its update while
0x2000 is set and its argument is 0 (`0x7c5c26` to `0x7c5c30`), as `0x7c5a20` does, so clearing
first is what lets both run. Census: of the 42 `and` instructions in the file with a mask of
0x80000000 or more, only `0x7c629b` clears bit 0x4000.

## Status record write

`0x7c6340(time, status)` fills a `CMovementStatus`. `0x618900` stores the flags masked with
0x75a07dff at status +0x10 first; this function adds the transport bit and the fields below.

| status | value |
|---|---|
| +0x00 | `time` |
| +0x08 | the transport GUID `+0x38`, or 0 |
| +0x10 | bit 0x2000000 set with a transport, cleared without |
| +0x14, +0x20 | position `+0x10` and facing `+0x1c` |
| +0x24, +0x30 | world position and facing, from `0x7c4930` and `0x7c4ae0` |
| +0x34, +0x38 | pitch `+0x20`, fall time `+0x78` |
| +0x3c to +0x48 | `+0xa0`, `+0x68`, `+0x6c`, `+0x84`, when status +0x10 has 0x2000 |
| +0x4c | `+0x80`, when the mover's `+0x40` has 0x4000000 |

Addresses: `0x7c6340`, `0x618900`, `0x7c4930`, `0x7c4ae0`, `0x7c63d4`, `0x7c6401`
Confidence: verified
Proof: one
Test: none

`0x7c6350` stores the first argument at status +0x00. `0x7c635c` tests the GUID for zero;
`0x7c6367` `or eax,0x2000000` and `0x7c6379` `and eax,0xfdffffff` write status +0x10. The world
position is `0x7c4930(&local, &+0x10)`, copied from the stack to +0x24; the world facing is
`0x7c4ae0(+0x1c)`, stored by `0x7c63cb` `fstp`. Facing and pitch pass through `fld`/`fstp`
dword pairs, which change no finite value. The fall test at `0x7c63d4` reads the status flags
already in the record; the +0x4c test at `0x7c6401` reads the mover's own. `ret 8`.

## Status record read

`0x7c6420(status)` copies status +0x24 to the position `+0x10` and the anchor `+0x44`, +0x30 to
the facing `+0x1c` and `+0x50`, +0x34 to the pitch `+0x20` and `+0x54`, and +0x4c to `+0x80`
whatever the flags; it zeroes `+0x58` and calls `0x7c5c20(0)`. `0x7c6490(status, unused)` copies
+0x38 to `+0x78` and +0x3c to `+0xa0`, sets `+0x7c` to `0x7c5f30(+0x38)`, copies +0x40 and
+0x44 to `+0x68`, `+0x6c` and to `+0x5c`, `+0x60` with `+0x64` = 0, and +0x48 to `+0x84`.
`0x7c6900(status)` acts when status +0x08 names a transport that `0x630a80` accepts: it puts
status +0x14 in `+0x10` and `+0x44`, +0x20 in `+0x1c` and `+0x50`, sets `+0x7c` to
`0x7c5f30(+0x78)`, and subtracts (+0x2c − +0x1c) from `+0x80`. It returns that GUID, else 0.

Addresses: `0x7c6420`, `0x7c6490`, `0x7c6900`, `0x630a80`, `0x618c30`, `0x618b30`, `0x7c6984`
Confidence: verified
Proof: one
Test: `collision::tf_setfacingto_residual_difftest`

In `0x7c6420` the facing and pitch go `fld`, `fst`, `fstp` (`0x7c6454` to `0x7c6465`) and +0x4c
goes as a dword (`0x7c6468`). `0x7c6490` reads only `[ebp+8]` and returns with `ret 8`; its call
to `0x7c5f30` at `0x7c64ad` follows the `+0x78` and `+0xa0` stores, so z + D uses the new launch
speed and the z already in the mover. `0x7c6900` returns at `0x7c691d` for a zero GUID and at `0x7c692b` with
`eax` 0 and `edx` 0 when the lookup fails; the success path loads the GUID into `eax:edx`
(`0x7c697a`, `0x7c6980`). The height is `fld [edi+0x2c]`, `fsub [edi+0x1c]`, then `0x7c6984`
`d8 ae 80 00 00 00`, `fsubr [esi+0x80]`: `+0x80` minus the difference. `0x618c30` calls the
first two; `0x618b30` calls `0x7c6900`.
The tests run `0x7c6977` to `0x7c6990`.

## Frame transform

`0x7c6620(m, a)` re-expresses the mover in another frame, given a 4×4 matrix m and an angle a.
It transforms the position `+0x10` and the anchor `+0x44` in place with `0x7bcc60`; adds a to
the facing `+0x1c` and the anchor facing `+0x50`, each wrapped once; maps the heights `+0x7c` and
`+0x80` to z·m[10] + m[14]; turns the direction `+0x5c` by m's upper 3×3; and sets `+0x68`,
`+0x6c` to its x and y, each times 1/len when the length is 2⁻²² or more, or NaN. With a live
spline, flag 0x4 clear, it ends with `0x7c6520(m, a)`, which moves the spline's point at +0x08,
its path (through `0x7bca80`) and its facing at +0x14 the same way.

Addresses: `0x7c6620`, `0x7c6520`, `0x7bcc60`, `0x7bca80`, `0x7c6656`, `0x7c666b`, `0x7c6752`, `0x7c6754`
Confidence: verified
Proof: one
Test: `collision::frame_transform_facing_advance_a_difftest`, `collision::frame_transform_facing_advance_b_difftest`, `collision::frame_transform_transport_delta_affine_difftest`, `collision::frame_transform_matvec3_difftest`, `collision::frame_transform_normalize2_difftest`, `collision::tf_spline_facing_accumulate_difftest`, `terrain::liquid_hit_flush_difftest`, `terrain::horizon_insert`, `cmath::transform_point4`

The wrap: `fadd`, `fcom` 2π, `test ah,0x41; jne` at `0x7c6656`, so only a sum above 2π takes
`fsub` 2π; otherwise `fcom 0.0`, `test ah,5; jp` at `0x7c666b` adds 2π only below 0. 2π is the
float `0x40c90fdb`, stored by the file's static initializer. An exact 2π stays 2π; NaN stays.
The heights are `fld [edi+0x28]`, `fmul`, `fadd [edi+0x38]`, `fstp`. The direction reads all
three old components before `0x7c670f` writes the first: x' = (m[8]·z + m[4]·y) + x·m[0],
y' = (m[1]·x + m[9]·z) + m[5]·y, z' = (m[2]·x + m[6]·y) + z·m[10], each rounded once to a float.
The length guard is `fabs`, `fcomp` 2⁻²², `test ah,5; jnp 0x7c676a` at `0x7c6752`, skipping
only for less; `0x7c6754` `d8 3d` is `fdivr` 1.0, giving 1/len in `st0` at 53 bits, which
`0x7c675c` and `0x7c6762` multiply into x and y before each float store (`0x7c66e1` to
`0x7c6765`). `0x7c6520` sizes its stack copy at
12 bytes a point.
The tests run `0x7c6645` to `0x7c6676`, `0x7c6676` to `0x7c66a7`, `0x7c66a7` to `0x7c66ca`,
`0x7c66ca` to `0x7c671c`, `0x7c672d` to `0x7c676c` and `0x7c6540` to `0x7c6577`.

`cmath::transform_point4` runs `0x7bca80` and `0x7bcc60` whole.

## Boarding and leaving transports

`0x617170` changes a mover's transport through these two. `0x7c6790(m)` leaves: `0x630ac0`
fills m for the transport at `+0x38`, `0x630b70` gives its facing f, and `0x7c6620(m, f)` moves
the mover into the world frame; then it unlinks the node at `+0x08`, calls `0x630c10` (an
immediate return), clears 0x8000000 through `0x7c6510` and returns f. `0x7c6840(guid, m, tm)`
boards: f = −`0x630b70(guid)`, the transport's matrix goes through a matrix routine with the
argument 1.0 into m, then `0x7c6620(m, f)` runs and `0x6309b0` links the mover to the
transport. It returns f.

Addresses: `0x7c6790`, `0x7c6840`, `0x617170`, `0x630ac0`, `0x630b70`, `0x630c10`, `0x6309b0`
Confidence: inferred
Proof: none
Test: `collision::tf_negate_transport_facing_difftest`

The call order and the arguments are read at the bytes: `0x7c67a8`, `0x7c67b5`, `0x7c67c4`,
the unlink from `0x7c67c9` to `0x7c6807`, `0x7c6820` behind a nonzero GUID, `0x7c6827`; in
`0x7c6840`, `0x7c6859` `fchs` and the eight-pass copy of 64 bytes into m at `0x7c6888` or
`0x7c68c0`. When `tm` is null, m receives the transport matrix itself before the routine runs.
What the callees compute (the transport's matrix, its facing, the matrix routine as an inverse,
the link) is not read here.
The tests run `0x7c684f` to `0x7c6861`, with `0x630b70` stubbed.

## Unreached functions

Six functions in this part have no caller. `0x7c6160` and `0x7c6180` store their argument times
1.0936 (`0x3f8bfb16`, `0x81da74`) at `0x87d894` and `0x87d898`. `0x7c61a0` returns
`[0x87d894]`. `0x7c7200` clears 0x400 for a nonzero argument, sets it for zero, and calls
`0x7c7240` with the argument. `0x7c7390(ms)` calls `0x7c73c0` with float(ms · 0.001).
`0x7c73c0(t)` returns 0.0005 times the larger of |`0x7c5d20`| at t + 0.0005 and at t − 0.0005,
the latter raised to 0 when negative.

Addresses: `0x7c6160`, `0x7c6180`, `0x7c61a0`, `0x7c7200`, `0x7c7390`, `0x7c73c0`, `0x81da74`
Confidence: verified
Proof: one
Test: none

Census over the image: every `e8` and `e9` whose rel32 target is the function, at every byte
offset of `.text`, and every little-endian occurrence of its address in every section. None for
the first five; `0x7c73c0` has one call, `0x7c73b0`, inside `0x7c7390`. Controls: the same scan
finds `0x7c61b0`'s call at `0x63706e` and `0x514120` stored at `0x850154`. Each function follows
a `ret` and padding, so none is reached by falling through. In `0x7c73c0` the halves are stored
as floats, the lower one becomes 0 when below 0 (`test ah,5; jp` at `0x7c73e9`), and
`0x7c7413` `fcomp st(1)`, `test ah,0x41; jne` keeps the later sample unless the earlier is
larger.
