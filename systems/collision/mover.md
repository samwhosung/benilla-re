# collision: mover

The movement manager and the per-frame drive of every unit's movement, in Movement.cpp.

One manager holds the state the movers share: the list of movers with work pending, the
simulation clock and the active mover's heartbeat deadline. The object manager keeps its pointer,
and Movement_C.cpp's `0x630a40` returns it. `0x616860` builds it at world entry and `0x616910`
tears it down. The tick `0x616800` runs as an event handler: it hands the frame's time window to
`0x616620` for every listed mover, which cuts the window into steps at queued events and at the
heartbeat deadline, integrates each step in substeps (`0x616de0`), and replays the events due
(`0x615c30`). The functions here also build the mover half of `CMovement` (`0x616fa0`), set its
collision size from the model (`0x6174b0`) and change its transport. The event queue and its
replay are the `replay` doc.

The rest of the client reaches the constructor, the size setters, the transport
setter, the heartbeat re-arm `0x615b80` (from the unit's movement-message sender), the lifecycle
pair and the last-tick getter `0x6169f0`.

Roots: `0x615b80`, `0x616800`, `0x616860`, `0x616910`, `0x6169f0`, `0x616fa0`, `0x617100`, `0x617460`, `0x6174b0`

## Movement manager

One `MovementGlobals` of 312 bytes exists. Its pointer is the dword at `+0xd4` of the object
manager `0xb41414`: `0x630a40` returns it through `0x4685e0` (0 when there is no object manager),
and `0x630a30` stores it through `0x468600`.

| offset | holds |
|---|---|
| `+0x000` | 260 bytes of the log path given to `0x616860` |
| `+0x104`, `+0x108` | two log file handles, closed at teardown |
| `+0x118` | the mover list: link offset 0, then the terminator at `+0x11c` and `+0x120` |
| `+0x124` | bit 0 once initialised |
| `+0x128` | the clock at the last tick |
| `+0x12c` | the end of the step being simulated |
| `+0x130` | the active mover's heartbeat deadline |

`0x6169f0` writes `+0x128` through its argument and returns 1 when the manager exists with bit 0
set, else returns 0.

Addresses: `0x630a40`, `0x630a30`, `0x4685e0`, `0x468600`, `0xb41414`, `0x6169f0`
Confidence: verified
Proof: one
Test: none

`0x630a40` and `0x630a30` are `jmp` thunks. `0x4685e0` loads `[0xb41414]`, returns it when 0, else
`[eax+0xd4]`; `0x468600` stores `ecx` there only when the object manager exists. The size is the
allocation in `0x616860`. In the collision code `+0x128` is written by `0x616860` and `0x616800`;
`+0x12c` by those and by `0x616620` per step (`0x6166ed`); `+0x130` by `0x615b80` and `0x615ae0`,
never at initialisation. `0x6169f0` checks bit 0 (`test byte [eax+0x124],1`) before the copy; its one caller, in the
input code, raises a time that is earlier than `+0x128` to it.

## Initialisation and teardown

`0x616860` takes the log path in `ecx`. It allocates the manager, empties the mover list, zeroes
`+0x104`, `+0x108`, `+0x110`, `+0x114` and bit 0 of `+0x124`, stores it with `0x630a30`, copies 260
bytes of the path to `+0x000`, stamps `+0x128` and `+0x12c` with the clock and sets bit 0. Its
caller `0x401a40` then registers `0x616800` with `0x41fca0` as the handler of event 5, with the
values 0 and 0x40000000. `0x616910` closes both log files, unlinks every mover from the list
without freeing it, frees the manager, stores null with `0x630a30`, and frees every event node on
the free list `0xc4db88`. `0x401bc0` and one other teardown path call it and then unregister
`0x616800` from event 5 with `0x41fd80`.

Addresses: `0x616860`, `0x616910`, `0x401a40`, `0x41fca0`, `0x41fd80`, `0x401bc0`, `0x6462e0`, `0x64a5a0`, `0x646430`, `0x73f285`, `0x7c4e90`, `0x861418`, `0xc4db88`
Confidence: verified
Proof: one
Test: none

`0x616876` calls `SMemAlloc` (`0x6462e0`) with 0x138, the path `0x861418` and line 688: the size
is the last push, `0x61686f`, the first argument. The list is `+0x11c = &+0x11c`,
`+0x120 = &+0x11c | 1` and `+0x118 = 0` (`0x6168a3` to `0x6168b4`). `0x6168da` is `SStrCopy`
(`0x64a5a0`) with 0x104. The registration is `push 0x40000000; push 0; mov edx,0x616800;
mov ecx,5; call 0x41fca0` (`0x401ae0` to `0x401af1`). Teardown: `0x616923` calls `0x7c4e90`, which
closes `+0x104`; `0x616933` and `0x61694c` are `_fclose` (`0x73f285`); the loop from `0x616961`
unlinks each mover's link and frees nothing; `0x61698b` frees the manager with `SMemFree`
(`0x646430`) and the tag `delete`; the loop from `0x616998` frees each node with the tag
`.?AUCPlayerMoveEvent@@`. The unregistration follows the call in `0x401bc0` (`0x401bde` to
`0x401bed`).

## Per-frame tick

`0x616800` reads the clock `now` (`0x42c010`). When `now − +0x128` is positive as an int32, it
calls `0x630970(now, now − +0x128)`, then `0x615b10(now, +0x128)` when the mover list is not
empty, then stores `now` in `+0x12c` and `+0x128`. It returns 1.

`0x615b10` walks the list in order, reading each node's successor before acting on it. A mover
with flag 0x8000000 in `+0x40` gets `0x615ae0(now − previous)` and is not moved; every other mover
gets `0x616620(now, previous)`. The same window is simulated for every listed mover, the local
player and remote units alike; the active-mover tests inside `0x616620` gate only the
deadline cut and the heartbeat.

Addresses: `0x616800`, `0x615b10`, `0x42c010`, `0x630970`, `0x615ae0`, `0x616620`, `0x619ca0`, `0x619e80`
Confidence: verified
Proof: one
Test: none

`0x616818` `sub edx,ecx; test edx,edx; jle` is the signed gate; `0x616820` passes `now` in `ecx`
and the difference in `edx`. The list test at `0x616825` treats a null or odd `+0x120` as empty.
In `0x615b10` the successor is `mov esi,[eax+ecx+4]` (`0x615b47`), `eax` the link offset
`+0x118`, and `test [ecx+0x40],0x8000000` (`0x615b4d`) picks the branch. Movers join the list
through `0x619ca0` and leave through `0x619e80`. Event dispatch is not read here.

## Local player step

`0x616620(now, previous)` advances one mover over `w = now − previous` (uint32), from `s = previous`:

| step | action |
|---|---|
| window | if `w > 250`: `0x615ae0(w − 250)`, `s = previous + w − 250`, `w = 250`; if `w = 0`: go to the end |
| 1 | `L = w − done`; with a queued event, `L = min(L, max(head.time − s, 0))` |
| 2 | active mover, spline null or with bit 0x4, flags & 0x200f: `L = min(L, +0x130 − s)` unsigned |
| 3 | `s += L`; manager `+0x12c = s` |
| 4 | if `L > 0` and flags & 0x20ff: `0x616de0(s, L)`, then a heartbeat if the tests of 2 hold and `s − +0x130 ≥ 0` |
| 5 | if `L > 0`: `done += L` |
| 6 | `0x615c30(s)`; on 0, `0x619e80(1)` and go to the end; else back to 1 while `done < w` |
| end | `0x5fac70(now, 0)`; a NaN in `+0x10`, `+0x14` or `+0x18` prints a console line and nothing more |

The heartbeat is `0x600a30(s, 0xee, 0, 0)`.

Addresses: `0x616620`, `0x616642`, `0x6166a9`, `0x6166f5`, `0x616725`, `0x600a30`, `0x5fac70`, `0x40a1b8`, `0x63cd50`, `0x8613fc`, `0xc4da98`
Confidence: verified
Proof: one
Test: none

No step has a fixed length: 250 bounds the window, and steps end at events and the deadline.
`0x616642 cmp esi,0xfa; jbe` is the unsigned 250; `0x616651` and `0x616659` move the start,
`0x616664` sets the window to 250. Step 1 is `0x61668f` to `0x61669c` (`sub eax,ebx; jns`, then
`cmp esi,eax; jbe`). Step 2 is `0x6166a9` to `0x6166e4` with `jae`: a deadline already passed
gives no cut, a deadline equal to `s` gives `L = 0`. `0x6166f5` is the 0x20ff test; the heartbeat
tests are `0x616712` to `0x616749`, the last a signed `js`. With `L = 0`, `0x6166f3` skips steps 4
and 5. The active mover is the one whose unit's GUID (`+0x15c`, then `+0x8`) equals `0xc4da98`
(`0x6166a9`, `0x6166b4`, the same at `0x616712`). The end widens each coordinate to a double for
`_isnan` (`0x40a1b8`) and calls `0x63cd50` with `0x8613fc`, "Mover at invalid position";
`0x63cd50` formats the text into 1024 bytes and passes it to the console writer only when the
console exists, and `0x616620` returns after it.

## Heartbeat and skipped time

The manager's `+0x130` is the active mover's heartbeat deadline. `0x615b80(t)` sets it to
`t + 500` when the mover is the active mover. `0x600a30`, the unit's movement-message sender, calls
it with the message time after each message it sends, so every message re-arms the deadline.
`0x615ae0(d)` calls `0x600be0(d)`, which, for the active mover whose spline is null or has bit
0x4, sends opcode 0x2ce with the unit's GUID and `d` and returns 1; on 1, `0x615ae0` adds `d` to
the deadline. So a frame longer than 250 ms reports its first `w − 250` ms as skipped and moves the
heartbeat that far.

Addresses: `0x615b80`, `0x615ae0`, `0x600a30`, `0x600be0`
Confidence: verified
Proof: one
Test: none

`0x615ba5 lea esi,[eax+0x1f4]` with `eax` the time argument, stored at `0x615bb0` after the GUID
test (`0x615b8e`, `0x615b99`). `0x600a30` calls it at `0x600ad6` on its success path with its
first argument. `0x600be0` tests the GUID and the spline (`0x600bec` to `0x600c1d`) and pushes
0x2ce at `0x600c40`. `0x615b02 add [esi+0x130],ebx` runs only when `0x600be0` returns nonzero.

## Per-frame sequence

For the local player, each dispatch of event 5 runs, in this order:

| # | step |
|---|---|
| 1 | `0x616800`: `now` = the clock; nothing more unless `now − +0x128 > 0` |
| 2 | `0x630970(now, now − +0x128)` |
| 3 | `0x615b10`: in list order, `0x616620(now, +0x128)` for the player |
| 4 | window: over 250 ms, 0x2ce reports the excess, the deadline moves by it, the start becomes `now − 250` |
| 5 | step length: the rest of the window, cut at the next queued event and at the heartbeat deadline |
| 6 | manager `+0x12c` = the step's end |
| 7 | with flags & 0x20ff, `0x616de0` integrates the step in substeps through `0x616cb0` and `0x634040` |
| 8 | moving (0x200f) at or past the deadline: heartbeat 0xee at the step's end, re-armed to end + 500 |
| 9 | `0x615c30` applies every event due by the step's end and passes its opcode on |
| 10 | 5 to 9 repeat until the window is spent; the mover leaves the list when 9 returns 0 |
| 11 | `0x5fac70(now, 0)`, then the NaN check of the position |
| 12 | manager `+0x128` and `+0x12c` = `now` |

Addresses: `0x616800`, `0x615b10`, `0x616620`, `0x616de0`, `0x616cb0`, `0x634040`, `0x615c30`
Confidence: verified
Proof: one
Test: none

Each row is derived in Per-frame tick, Local player step, Heartbeat and skipped time, Substep loop
and, in `replay`, Event replay loop. A heartbeat goes out at the simulated time that equals the deadline when the deadline lies
ahead of the step's start, because step 5 ends a step there; a deadline already passed cuts
nothing (`jae`, unsigned) and the heartbeat goes out at the end of the next step. Remote units run
the same sequence without the report of row 4, the deadline cut of row 5 and row 8.

## Substep loop

`0x616de0(end, L)` returns at once when `0x616bf0` rejects the position or `L = 0`. Each pass,
while flags & 0x20ff, with `r = L − done`: `+0x58 += r`, `delta = 0`, then

- live spline (`+0xa4` set, its `+0x18` without bit 0x4): `0x616af0(end, r, &delta)`, returning on
  0; `acc = anchor + delta`, anchor `+0x44`;
- otherwise `0x7c5360(+0x58, 0x618920(), &delta)`, returning on 0; when `+0x14c ≠ 0` and
  `0x6191c0(end − r, &(anchor + delta))` returns nonzero, `delta` becomes that point − anchor;

then `done += 0x616cb0(end − r, r, &delta)`, until `done ≥ L`. After the loop, if the position is
valid: with a live spline `0x616d60(&acc)`, and with a live spline whose `+0x18` has bit 0x200,
`0x7c5cd0(0)`.

Addresses: `0x616de0`, `0x616e20`, `0x616bf0`, `0x616af0`, `0x7c5360`, `0x618920`, `0x6191c0`, `0x616cb0`, `0x616d60`, `0x7c5cd0`
Confidence: verified
Proof: one
Test: `collision::integrate_apply_add_difftest`, `collision::integrate_unapply_sub_difftest`

`0x616e20 test [esi+0x40],0x20ff` heads each pass; `0x616e36` to `0x616e3c` add `r`, not the time
consumed, to `+0x58` (`0x634040` adjusts `+0x58` on its own exits; resolve topic). The branch is
`0x616e54` (null spline) and `0x616e5a` (bit 0x4). `acc` and the blended point are float sums, each
`fadd` then `fstp dword`; `delta` is `fl(point − anchor)`. `acc` starts at (0, 0, 0) (`0x616e07`)
and only the spline pass writes it. `0x618920` returns `&+0x144` when `+0x148` compares unequal to
0.0 (NaN included), else null. The pass time `end − r` is the start of the unsimulated rest
(`0x616f33`); `0x616af0` gets `end`. A return through a 0 skips the commit, the snap and the
re-anchor.
The tests run `0x616e74` to `0x616ea1` and `0x616eff` to `0x616f2c`.

## Position validity

`0x616bf0` returns 1 when `x`, `y` and `z` at `+0x10` are finite, `17066.666 − y` (53-bit) lies in
`[2.0, 34131.332)` and `fl(17066.666 − x)` lies in the same range; else 0. For float coordinates
this is `−17064.666 < x, y ≤ 17064.666`; `z` is tested only for finiteness. The constants are
17066.666015625 (`0x46855555`, 32 × 533.33333), 2.0 and 34131.33203125 (`0x47055355`, `0x80c738`).
No NaN reaches the compares.

Addresses: `0x616bf0`, `0x40a1a4`, `0x80c738`
Confidence: verified
Proof: one
Test: `collision::pos_valid_difftest`

`0x616c11`, `0x616c2a` and `0x616c3f` call `_finite` (`0x40a1a4`) on `x`, `y`, `z` as doubles.
`0x616c4e` `fsub` (`d8 25`) then `fchs` gives `17066.666 − y`, kept in `st0`; `x`'s difference is
rounded by `fstp [ebp−8]` (`0x616c61`) and reloaded. For each: `fcom 2.0; test ah,1; jne` rejects
below 2.0, and `fcomp 34131.332; test ah,5; jp` rejects at or above it.

## Spline warp gate

`0x616af0(end, r, &delta)` gets the spline displacement from `0x7c5490(end, &delta)` and returns 0
when that does. Unless flags & 0x200f and the spline's `+0x18` lacks 0x200, it returns 1. Else
`target = fl(anchor + delta)` per axis, `D = dy² + dx²` from the position (53-bit) and
`T = r · 0.001` (`0x3a83126f`). If `D / T² > 3600` (`0x45610000`, `0x80c734`) or `D > 9.0`, each
test also passing when unordered, it stores `target` as the position, sets `+0x7c` to
`0x7c5f30(+0x78)` when flags & 0x2000, calls `0x7c4930` and `0x634030`, and returns 0, which ends
`0x616de0` without a commit. Otherwise it returns 1. A step over 3 yards or 60 yards
per second in `x` and `y` is taken without collision.

Addresses: `0x616af0`, `0x7c5490`, `0x80c734`, `0x7c5f30`, `0x7c4930`, `0x634030`
Confidence: verified
Proof: one
Test: `collision::anti_warp_difftest`

`0x616b7c` is `d8 fa`, `fdivr st,st(2)`: `D / T²`. The tests are `test ah,0x41; jp` at
`0x616b88` and `0x616b95`; parity is set for greater (no bits) and unordered (C0 and C3), and
either jump, to `0x616ba7` or `0x616ba9`, reaches the store at `0x616bb1`; only both tests failing
falls through to the `mov eax,1` at `0x616b9b`. With `r = 0`, `T = 0` and `D / T²` is +∞ or NaN,
so the step is always taken without collision. The flags are
read at `0x616b13`, after `0x7c5490`. `0x634030` is `ret 4`.

## Substep apply and commit

`0x616cb0(t, r, &delta)` forms `target = anchor + delta` and the displacement
`target − position`: `x` and `y` from the unrounded 53-bit target, `z` from the target rounded to a
float, each difference stored as a float. The vertical difference is set to 0 unless the spline is
live with bit 0x200 in its `+0x18`, or flags & 0x200800. It returns what `0x634040(t, r, &disp)` returns, its count of milliseconds (resolve topic).

Addresses: `0x616cb0`, `0x616cec`, `0x616cfa`, `0x616d03`, `0x634040`
Confidence: verified
Proof: one
Test: `collision::build_delta_difftest`

`0x616cd2 fstp [ebp−0x10]` rounds the target's `z`; `x` and `y` stay on the stack through
`fsub [ecx+0x10]` and `fsub [ecx+0x14]`. `0x616cec je` sends a null spline to the flags test;
`0x616cf1 test al,4; jne` does the same for bit 0x4; `0x616cf5 test ah,2; jne 0x616d0a` keeps the
difference; `0x616cfa test [ecx+0x40],0x200800; jne` keeps it; `0x616d03` stores 0.

## Snap beyond three yards

`0x616d60(p)` computes `S = (dz² + dx²) + dy²` from `p − position` in 53 bits. When `S < 9.0` it
does nothing; otherwise, NaN included, it stores `p` as the position and calls `0x7c6290`,
`0x7c4930` and `0x634030`. `0x616d30` calls it with the spline's point at `+0x58`, then sets the
spline's `+0x20` to its `+0x24` and ORs 1 into its `+0x18`.

Addresses: `0x616d60`, `0x616d30`, `0x7c6290`
Confidence: verified
Proof: one
Test: `collision::dead_band_difftest`

`fcomp 9.0; test ah,5; jnp` (`0x616d90` to `0x616da1`) skips only for C0 alone, below 9.0. The
squares are `fmul` on the stack in the order `dz`, `dx`, `dy`, summed by two `faddp`.

## Construction

`0x616fa0(position, facing, guid, unit)` runs `0x7c4850(position, facing, guid)` for `+0x00` to
`+0xab`, then sets the rest of the 352-byte `CMovement`, which has no vtable, in this order:

| field | value |
|---|---|
| `+0x13c`, `+0x148`, `+0x14c` | 0 |
| `+0xb0` | 0.33333334 (`0x3eaaaaab`) |
| `+0xb4` | 2.0277777 (`0x4001c71c`) |
| `+0xb8` | 1.0 |
| `+0x154`, `+0x158`, `+0x150` | `&+0x154`, `&+0x154` with bit 0 set, link offset 0 |
| `+0x15c` | `unit` |
| (call) | `0x7c5a20(0)` |
| `+0xbc` to `+0xf8` | sixteen −50 |
| `+0xfc` to `+0x138` | sixteen 50 |
| `+0x140` | 50 |

`+0x144` is left unset. It returns `this`.

Addresses: `0x616fa0`, `0x616fd8`, `0x7c4850`, `0x7c5a20`, `0x5fad50`
Confidence: verified
Proof: one
Test: none

`0x616fce`, `0x616fd8` and `0x616fe2` store the three floats; `0x617030` and `0x617042` are
`rep stos` of 0xffffffce and 0x32, sixteen dwords each, and `0x617044` stores `eax`, still 0x32.
`ret 0x10`. The unit constructor `0x5fad50` passes `this` = unit `+0x9a8`, then stores −1 at
`+0xb08`, the next member (`0x5faddc`, `0x5fade4`); no store in `0x616fa0` or `0x7c4850` writes a
vtable. The 32 dwords from `+0xbc` are one ring: `0x618b50` wraps its index with `and edx,0x1f`.

## Collision size

`0x6174b0(w, h, s, d)` sets `+0xb8` to 1.0 when `|d| < 2⁻²²` (`0x34800000`), else `fl(s / d)`, NaN
for a NaN `d`; `+0xb0` to `fl(w · s · 0.5)`; `+0xb4` to `fl(h · s)`; then calls `0x7c4930` and
`0x634030`. `0x617460(w, h, s)` takes `a = fl(w · s · 0.5)` and `b = fl(h · s)`; when
`a · 1.8493990` (`0x3fecb91b`, `0x80c740`) exceeds `s` it uses `a = fl(s · 0.54071623)`
(`0x3f0a6c61`, `0x80c73c`); equality or NaN keeps `a`. It returns `0x631670(b, a)` on this mover.
`0x60b270` sizes a unit's mover with them. `0x617430` returns `+0xb8` when `0x5fa550(unit)` is nonzero, else 2.0.

Addresses: `0x6174b0`, `0x617501`, `0x617460`, `0x631670`, `0x80c73c`, `0x80c740`, `0x60b270`, `0x617430`, `0x5fa550`
Confidence: verified
Proof: one
Test: `collision::param_set_difftest`, `collision::turn_clamp_difftest`, `objectlayer::box_dim_pf_even_difftest`

`0x6174b6` loads 1.0 first; `fabs; fcomp 2⁻²²; test ah,5; jnp` keeps it below the bound, else
`fld s; fdiv d`. `0x617501` stores `+0xb4`. `0x617484 fcomp [ebp+0x10]; test ah,0x41; jne`
compares with `s`; the clamp is `0x61748e`. The pushes before `0x6174a2` put `b` first. `0x60b270`
passes a model record's `+0x38` and `+0x3c` as `w` and `h`, the larger of the unit's scale and a
display value as `s` (`0x60b315 fcomp [ebx+0x10]; test ah,5; jp`) and that value as `d`; it calls
`0x617460` at `0x60b3b7` and skips `0x6174b0` (`0x60b3e4`) when that returns nonzero. Slices: box_dim_pf_even_difftest `0x60b325`-`0x60b332`.

## Transport change

`0x617170(g)` makes `g` the transport GUID at `+0x38`. It returns 0 when `g` equals it, or `g ≠ 0`
and `0x630a80(g)` is 0. Leaving a transport, `0x7c6790(&m)` fills a matrix `m` (from the
identity) and returns an angle `f`; the unit's model, `+0xdc` else `+0xd8`, gets `0x718910(0)`;
then `0x601050(f)` and `0x6115e0(&m, f)` on the unit, and `0x617080(f)`. Boarding repeats this
with `f = 0x7c6840(g, &m2, &m)`, `0x718910(&m)` and `0x6115e0(&m2, f)`. It stores `g`, calls
`0x6309f0(owner GUID, g)` and returns 1. `0x617100(g)` adds a heartbeat at the manager's `+0x128` for the
active mover off a live spline. `0x6173b0(g)` refuses to clear the transport while `0x630ba0` holds for the
position, and only for the active mover off a live spline calls `0x617170`.

Addresses: `0x617170`, `0x617100`, `0x6173b0`, `0x630a80`, `0x7c6790`, `0x7c6840`, `0x718910`, `0x601050`, `0x6115e0`, `0x617080`, `0x6309f0`, `0x630ba0`
Confidence: verified
Proof: one
Test: none

The compare is `0x617180` and `0x617189` (exact, both halves). The leave leg runs when the old GUID
is nonzero (`0x61721d`), the board leg when `g` is (`0x61728c`). `0x617100` sends
`0x600a30(+0x128, 0xee, 0, 0)` after the GUID and spline tests (`0x617119` to `0x617162`); it
returns 0 when nothing changed. `0x6173b0` tests `g = 0` with a transport set before calling
`0x630ba0` (`0x6173bc` to `0x6173dc`).

## Queued facing rotation

`0x617080(f)` walks the mover's queue and, for each event of type 16 or with byte `+0x4c`, adds `f`
to its facing `+0x1c`: a sum above 2π (`0x40c90fdb`) loses 2π once, else a sum below 0 gains 2π
once, and the result is stored as a float. Exactly 2π and NaN are stored unchanged. For `|f| < 2π` and a
facing in `[0, 2π)` the result lies in `[0, 2π]`.

Addresses: `0x617080`
Confidence: verified
Proof: one
Test: `collision::angle_wrap_difftest`

The walk starts at `+0x158` and follows `[node+4]` (`0x617083`, `0x6170d1`). `0x6170bb fcom 2π;
test ah,0x41; jne` keeps below, equal and unordered sums; `0x6170d6 fcom 0.0; test ah,5; jp` adds
only for C0 alone. The sum is 53-bit; the `fsub` or `fadd` of 2π precedes the one `fstp dword`.

## Unreached functions

`0x616a20` formats a mover's state, its GUID, position, facing, transport, times, fall height and
vertical speed, and whether it is the active mover, into an error report through `0x64de00` with
code 0x85100000. Nothing reaches it. Its row, of class `unreached`, keeps its meaning as `inferred`.

Addresses: `0x616a20`, `0x64de00`
Confidence: verified
Proof: one
Test: none

The census finds no `call` or `jmp` to `0x616a20` and no occurrence of its address as a dword in
any section; the same scan finds `0x514120` at `0x850154`. The instruction before it is padding
after `ret` (`0x6169f0` ends at `0x616a17`).
