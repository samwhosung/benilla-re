# collision: status

How a movement status is timed, applied and reported. A status is the wire's movement record: a
time stamp, the transport GUID, the flags, the pose, the fall time and the jump data. Each unit's
`CMovement` gives an incoming status a fire time on its own replay clock, applies it at once when
that time has come, and otherwise queues it as a move event; before a queued event fires, the
mover blends its facing and position toward it.

The code is the middle of Movement.cpp, `0x618c30` through `0x61aca0`. This doc holds the timing
core, the apply, the entry points, the active-mover list, the two pre-fire blends, the create-block
apply and the spline completion; `handlers` holds the per-opcode handlers and the state-change
broadcaster. The rest of the client reaches this part through the unit's message handlers, which
pass the `CMovement` at the unit's `+0x9a8` as `this`, through the create-block apply `0x619320`,
the time-skip handler `0x61ab90`, the mover change `0x619d50` and the clock read `0x619d40`.

Roots: `0x619320`, `0x619d40`, `0x619d50`, `0x61a750`, `0x61ab90`

## Status fire time

`0x618c30` gives a status a fire time on the mover's replay clock and returns 1 when it applied
the status, 0 when the caller must queue it at `eventTime + *skew`. The arithmetic is 32-bit two's
complement in client milliseconds; the update time is the movement globals' `+0x128`. Steps, in
order:

| step | effect | at |
|---|---|---|
| 1 | a spline at `+0xa4` forces the apply | `0x618c43` |
| 2 | flag bit 31 clear: `+0xac` = status stamp, `+0xa8` = eventTime, set bit 31 | `0x618c54`, `0x618c6a` |
| 3 | eventTime before the update time: eventTime = update time | `0x618c81` |
| 4 | `w` = stamp − `+0xac`; `w` > 0 stores the stamp in `+0xac`, else `w` = 0 | `0x618cb6`, `0x618cb8` |
| 5 | `*skew` = `w` − (eventTime − `+0xa8`) | `0x618ccf` |
| 6 | `m` = `0x618b50(w, eventTime − +0xa8)` | `0x618cd1` |
| 7 | forced: `+0xa8` = eventTime + `*skew`, unclamped; apply | `0x618d67` |
| 8 | flags & 0x20ff = 0 and an empty queue: rebase (next entry) | `0x618ce4`, `0x618cf3` |
| 9 | `*skew` = clamp(`*skew`, −500, 1000) | `0x618d49` |
| 10 | `+0xa8` = eventTime + `*skew` | `0x618dcc` |
| 11 | update time − `+0xa8` ≥ 0: apply; else return 0 | `0x618dd2` |

Addresses: `0x618c30`, `0x618c43`, `0x618c54`, `0x618c6a`, `0x618c81`, `0x618cb6`, `0x618cb8`, `0x618ccf`, `0x618cd1`, `0x618ce4`, `0x618cf3`, `0x618d49`, `0x618d67`, `0x618dcc`, `0x618dd2`, `0x618ea0`, `0x630a40`
Confidence: verified
Proof: one
Test: none

`ret 0x14`: eventTime, status, the skew pointer, a fall flag and a force flag. Step 2 is
`js 0x618c6d` on the flags. Step 3 is `sub ecx,eax; jns` against `[0x630a40()+0x128]`; the
lowered case calls `0x618ea0`, a bare `ret`, with the format `eventTime=%d, lastUpdate=%d`, and
stores the update time back into the argument. Step 4 is `test eax,eax; jle` on the stamp
difference. The clamp is `lea eax,[ecx+0x1f4]; cmp eax,0x5dc; jbe`: an unsigned test that keeps
[−500, 1000]; otherwise `sar eax,0x1f; not eax; and eax,0x5dc; sub eax,0x1f4` gives 1000 for a
value above and −500 for one below, and −500 also for one within 500 of 2³¹, whose sum wraps
negative. Step 11 is `sub ecx,eax; jns 0x618d71`; the deferred exit is `xor eax,eax; ret 0x14`.
The relay dispatcher passes the async clock (`0x42c010`) as eventTime. Of the four callers, three
pass zero for both flags and `0x619320` passes force 1.

## Lateness window rebase

`0x618b50(w, l)` stores `+0x140 + l − w` in the 32-slot ring at `+0xbc`, at the cursor `+0x13c`,
advances the cursor modulo 32, and returns the largest slot, the new one included. The
constructor `0x616fa0` sets slots 0 to 15 to −50, the rest and `+0x140` to 50.
The rebase (idle, empty queue) adds `m − +0x140` to the skew, clamps it to
[−500, 1000], raises it to `+0xa8 − eventTime` when `eventTime + skew` would fall before the
previous fire, and stores `m` in `+0x140`. The clamp of step 9 then runs again, so the step back to
the previous fire holds only within 1000 ms. A busy or queued mover still records its sample but
keeps `+0x140`.

Addresses: `0x618b50`, `0x616fa0`, `0x618cfc`, `0x618d0d`, `0x618d3b`, `0x618d41`
Confidence: verified
Proof: one
Test: none

In `0x618b50`: `sub eax,edx` (`w − +0x140`), `sub eax,[ebp+0xc]`, `neg eax`, the store
`[ecx+edx*4+0xbc]`, `inc edx; and edx,0x1f`, then 32 rounds of `cmp ecx,eax; jle; mov eax,ecx`,
which keep the larger. The constructor stores `0xffffffce` with `rep stos` 16 dwords from `+0xbc`
(`0x617026`) and `0x32` 16 dwords from `+0xfc`, then `+0x140` (`0x617044`). The rebase:
`0x618cfc` loads `m`, `sub eax,ecx` with `+0x140`, adds the skew, clamps at `0x618d0d`, then
`sub eax,ecx; add eax,ebx; jns` at `0x618d3b` tests `skew − +0xa8 + eventTime`, a negative result
storing `+0xa8 − eventTime`; `0x618d41` stores `m`. The constructor values sit 50 above a zero
start in every slot that can win, so the window behaves as if it started at zero.

## Status merge and apply

An applied status runs, in order: the fall test, the flag merge, the pose, the transport, the fall
or landing, the catch-up, the idle hook and the queue reset. It returns 1. The merge keeps the
mover's bits outside the mask and takes the status's bits inside it: `old ^ ((old ^ new) & M)`.

| step | effect | at |
|---|---|---|
| 1 | `fallMoving` = old & 0x2000 and `+0xa0` ≠ 0.0 (NaN counts as nonzero) | `0x618d91` |
| 2 | M = 0x75a07dff; 0x75a01dff, which keeps 0x6000, only with the fall flag set and no force | `0x618de7`, `0x618deb`, `0x618db7` |
| 3 | `0x7c6420(status)`: pose and anchors from the status, `+0x58` = 0 | `0x618dfc` |
| 4 | `0x618b30(status)`: the transport GUID | `0x618e07` |
| 5 | new & 0x2000: `0x7c6490(status)` takes fall time and jump data | `0x618e1b` |
| 6 | otherwise `0x7c5a20(0)`, then old & 0x2000: landing hook `0x602d00(eventTime, old, fallMoving)` | `0x618e24`, `0x618e3c` |
| 7 | not forced: `0x618bb0(+0xa8, update time − +0xa8)` | `0x618e5d` |
| 8 | new & 0x20ff = 0: the owner's `0x5fac70(eventTime, 1)` | `0x618e77` |
| 9 | `0x6159c0` empties the move-event queue at `+0x150` | `0x618e82` |

Addresses: `0x618d91`, `0x618db7`, `0x618de7`, `0x618deb`, `0x618dfc`, `0x618e07`, `0x618e1b`, `0x618e24`, `0x618e3c`, `0x618e5d`, `0x618e77`, `0x618e82`, `0x7c6420`, `0x618b30`, `0x7c6490`, `0x7c5a20`, `0x602d00`, `0x618bb0`, `0x5fac70`, `0x6159c0`
Confidence: verified
Proof: one
Test: none

Step 1: `fld [esi+0xa0]; fcomp 0.0; test ah,0x44; jp` keeps 1 unless the compare is equal.
Step 2: the force flag goes to `0x618de7`; the fall flag with no force goes to `0x618db3`, whose
`and eax,0x75a01dff` differs only in 0x2000 and 0x4000. No caller passes that pair, so 0x75a07dff
is the live mask; 0x2000000 and bit 31 lie outside it. `0x7c6420` copies status `+0x24` to `+0x10`
and `+0x44`, `+0x30` to `+0x1c` and `+0x50`, `+0x34` to `+0x20` and `+0x54`, `+0x4c` to `+0x80`.
`0x7c6490` copies `+0x38` to `+0x78`, `0x7c5f30(+0x38)` to `+0x7c`, `+0x3c` to `+0xa0`, and `+0x40` to `+0x48` into `+0x5c`, `+0x68` and `+0x84`; its
second argument, the old flags (`push edi`), is unread. `ebx` holds old & 0x2000 for step 6.
`0x6159c0` checks the list against the free list `0xc4db88` and moves every node onto it.

## Status entry points

`0x618f20` is the common entry: it calls `0x618c30(eventTime, status, &skew, 0, 0)`. Applied, it
drops the spline (`0x7c6310`), refreshes the active list (`0x618940`) and returns 1; deferred, it
queues `0x6176a0(eventTime + skew, tag, 0, 0, param, status)` and returns 0. `0x618eb0` does the
same but queues through `0x6177a0` with three more arguments; `0x619280` uses tag 0x10 with the
status facing as the parameter. `0x61a750`, the relay of opcodes 0xc9 and 0xee, passes tag 0x25.
The status record, as these read it:

| offset | field |
|---|---|
| `+0x00` | sender's time stamp, ms |
| `+0x08` | transport GUID, 8 bytes |
| `+0x10` | movement flags |
| `+0x14`, `+0x20` | transport-space position and facing |
| `+0x24`, `+0x30` | world position and facing |
| `+0x34` | pitch |
| `+0x38`, `+0x3c` | fall time and vertical speed |
| `+0x40` to `+0x4b` | jump data |
| `+0x4c` | `CMovement` `+0x80`, written by `0x7c6340` only under flag 0x4000000 (`0x7c6401`), read by `0x7c6490` (`0x7c6468`) |
| `+0x50` to `+0x64` | six speeds, create block only |
| `+0x68` | spline, create block only |

Addresses: `0x618f20`, `0x618eb0`, `0x619280`, `0x61a750`, `0x7c6310`, `0x618940`, `0x6176a0`, `0x6177a0`, `0x617730`
Confidence: verified
Proof: one
Test: none

`0x618f20`: `push 0; push 0; lea eax,[ebp+0xc]; push eax; push edi; push ebx`, so the status
pointer's own slot receives the skew; the deferred arm pushes `status, param, 0, 0, tag` and
`add eax,ebx`. `0x618eb0` (`ret 0x18`) pushes `status, a6, a5, a4, 0, 0, tag, fire` for
`0x6177a0`. `0x619280` (`ret 8`) pushes `0x10` and takes status `+0x20` when `+0x08 | +0x0c` is
nonzero, else `+0x30` (`0x6192be`). `0x61a750` is `push 0; push 0x25` and a call to `0x618f20`.
The record offsets follow `0x617730`, which takes `+0x14`/`+0x20` with a transport and
`+0x24`/`+0x30` without, and `0x7c6420`, `0x7c6490` and `0x619320`.

## Active mover list

`0x619ca0`
links a mover first in the list at globals `+0x11c` (link offset `+0x118`), unless `+0x04` is
already set. `0x619e80(report)` returns when `+0x04` is 0; otherwise a spline not yet done is
completed by `0x619de0` at the step time when `report` is nonzero, or just marked done (spline
`+0x18` |= 4) when it is 0; then it unlinks `+0x00`/`+0x04` and zeroes both. `0x618940` links a
mover whose flags & 0x20ff is nonzero or whose queue is non-empty and unlinks the rest with
`report` 1. `0x619d40` returns the step time, globals `+0x12c`, advanced per substep (`0x6166ed`);
the update time `+0x128` moves once per update (`0x616846`).

Addresses: `0x619ca0`, `0x619e80`, `0x618940`, `0x619d40`, `0x619ec0`, `0x6166ed`, `0x616846`
Confidence: verified
Proof: one
Test: none

`0x619ca0`: `mov eax,[ebx+4]; test eax,eax; jne` to the exit, then `0x630a40`, `add ecx,ebx` with
`+0x118`, an unlink when the link's first dword is set, and the splice before `lea edx,[eax+0x11c]`.
`0x619e80` tests `[esi+4]`, then the spline's `test al,4`; `0x619eba` is `or eax,4`; the unlink
starts at `0x619ec0` and ends with two zero stores. `0x618940`: `test [edx+0x40],0x20ff; jne`
and `0x6158f0` (queue non-empty) to `jmp 0x619ca0`, else `push 1; call 0x619e80`. `0x619ca0` sends
nothing; the unlink sends only through `0x619de0`, opcode 0x2c9 for the active player.

## Facing rate toward the head

`0x619030(t)` arms the facing blend toward a queue head that carries a status, has a tag other
than 0x26 and the mover's transport GUID; otherwise it zeroes `+0x148`.
`0x618f80(t0, f0, t1, f1)` stores `s` = (t1 − t0, unsigned, as a 64-bit integer) × 0.001
(`0x3a83126f`), at 53 bits, to `+0x148`; `|s|` < 2⁻²² (`0x34800000`) becomes 0. A
nonzero `s` gives Δ = f1 − f0, minus 2π (6.2831855, `0x40c90fdb`) when Δ > π (`0x40490fdb`), plus
2π when Δ < −π, NaN unchanged, and stores Δ / s to `+0x144`. Then `|+0x144|` < 2⁻²⁰
(`0x35800000`) zeroes `+0x148`; NaN keeps it. A head due up to about 10⁹ ms ago wraps `s` above
3.29·10⁶, so the blend is off.

Addresses: `0x619030`, `0x618f80`, `0x61904b`, `0x618fd7`, `0x618fff`, `0x619011`, `0x618920`
Confidence: verified
Proof: one
Test: `collision::facing_interp_difftest`

`0x619030` gates at `0x619044` (`+0x4c`), `0x61904b` (`cmp [eax+0xc],0x26`) and the two GUID
compares, then passes `t`, `+0x1c`, head `+0x08` and head `+0x1c`. In `0x618f80`, `xor edx,edx`
stores a zero high dword before `fild qword`; `fst [ecx+0x148]` keeps `s` in `st0`.
`test ah,5; jp` after `fcomp` of `|s|` replaces `s` with 0.0 only when less. `fcom 0.0`, a second
`fst`, `test ah,0x44; jnp` skips on equal. `fsub [ebp+0xc]` (`d8 65 0c`) gives f1 − f0;
`test ah,0x41; jne` subtracts only when greater; `test ah,5; jp` adds only when less. `0x618fff`
is `d8 f1`, `fdiv st,st(1)`: Δ / s. The final test reads the stored float; `edx` is still 0 for the
store. `0x618920` returns `&+0x144` while `+0x148` ≠ 0.0, the rate the prediction uses.

## Position blend arming

`0x619090(t)` arms the position blend toward the queue head under the same three gates. With
`r` = head fire time − `t`, it predicts the displacement after `+0x58 + r` ms through `0x7c5360`
and the facing rate `0x618920()`; on success the predicted point is that displacement plus the
anchor `+0x44`. When swimming (flags & 0x200000) the miss is `(dz² + dy²) + dx²`, otherwise
`dy² + dx²`, each difference predicted minus head. A miss of at least 1/1296 (0.00077160494,
`0x3a4a4588`), or NaN, stores `r` in `+0x14c`; a smaller miss, a failed gate or a failed prediction
stores 0.

Addresses: `0x619090`, `0x6190bb`, `0x619111`, `0x619139`, `0x619183`, `0x7c5360`
Confidence: verified
Proof: one
Test: `collision::reconcile_gate_difftest`

Gates: `0x6190b0` (`+0x4c`), `0x6190bb` (tag 0x26), `0x6190c5` and `0x6190d1` (GUID).
`mov ebx,[edi+8]; sub ebx,ecx` gives `r`; the zeroed vector goes to `0x7c5360(+0x58 + r, rate)`
and `test eax,eax; je` at `0x619111` drops a failed prediction. The x sum stays in `st0` at 53
bits; y and z are rounded to floats by `fstp`. `test eax,0x200000` selects at `0x619139`: the swim
arm subtracts head `+0x10`, `+0x14`, `+0x18`, squares z and y, adds them, then adds x²; the flat
arm squares y, then adds x². `fcomp 1/1296; test ah,5; jnp 0x61919f` takes the zero store only
when less (`0x619183`).

## Position blend

`0x6191c0(t, p)` pulls a predicted point toward the queue head: with `r` = head fire time − `t` and
`D` = `+0x14c`, it writes `p = head + (p − head) · (r / D)` and returns 1. No head, or `r` < 0,
zeroes `+0x14c` and returns 0. Precision: `f` = r / D at 53 bits from two integers; x is
`(float)((float)(ex · f) + hx)`; y is `(float)(ey · f + hy)` with the product unrounded; z is
`(float)((float)((float)ez · f) + hz)`, where each `e` is `p − head` at 53 bits. The only caller,
`0x616de0`, passes a nonzero `+0x14c` and `p` = anchor plus the predicted displacement.

Addresses: `0x6191c0`, `0x6191f2`, `0x619214`, `0x619228`, `0x61922f`, `0x616de0`, `0x616ecb`
Confidence: verified
Proof: one
Test: `collision::pos_lerp_difftest`

`0x6191eb` is `sub esi,[ebp+8]` and `jns` at `0x6191f2`. `fild [ebp+8]; fidiv [ebp-8]` at
`0x619214` divides by `+0x14c` read as `int32`. The three `fsub` of head `+0x10`, `+0x14`, `+0x18`
leave ez on top, and `0x619228` rounds it to a float. `fxch; fmul st,st(2)` and `fstp` at
`0x61922f` round ex · f; `fmul st,st(1)` keeps ey · f on the stack, and `fadd [eax+0x14]` adds the
head y to it before its `fstp`; ez · f is rounded at `0x619239`. The caller tests
`mov eax,[esi+0x14c]; test eax,eax; je` at `0x616ecb`.

## Object update status

`0x619320(unused, status, a3)` applies a create block. It reads the clock (`0x42c010`),
copies six speeds (status `+0x50` to `+0x88`), zeroes `+0x58` and the whole flag word. With
status flags 0x400000 it restarts the spline at the position (`0x619300`) and copies the status
spline at `+0x68` over `+0xa4`, all of `+0x00` to `+0x63` but `+0x2c`, the arrays at `+0x34` and
`+0x44` by `0x61aca0` and `0x619480`. Else it drops the spline.
It applies the status forced at the clock (flags = new & 0x75a07dff, bit 31 set, queue emptied); refreshes the list; calls `0x615b80(clock)` for flags & 0x200f; and,
idle, queues tag 6 (`0x617ac0`) for a nonzero `a3`, else starts a fall (`0x617b00`); last,
`0x6309f0` gets the owner and transport GUIDs.

Addresses: `0x619320`, `0x61936e`, `0x61941f`, `0x619300`, `0x619480`, `0x61aca0`, `0x42c010`, `0x615b80`, `0x617ac0`, `0x617b00`, `0x6309f0`, `0x646320`
Confidence: verified
Proof: one
Test: none

`0x61936b`/`0x61936e` store 0 to `+0x58` and `+0x40` before the call at `0x61941f`, which pushes
`1, a3, &skew, status, clock`. `rep movs` copies six dwords, then `+0x18` to `+0x28` and `+0x30`
one by one. `0x619300` calls `0x7c62c0` with `+0x10`, which allocates a spline if needed, clears
its `+0x18` and writes `+0x58`, then `0x617fc0(0)`, which drains the queue and unlinks. The two
array copies (`ret 4`, returning `this`) set the count, `SMemReAlloc` (`0x646320`) the buffer to
4 or 12 bytes per element, each with its own allocation tag, copy each element and the
`+0x0c` dword; a self-copy copies only `+0x0c`.

## Queued status apply

`0x61a770(event)` applies a queued event's status. It sets the transport from event `+0x28`
through `0x618970` and returns 0 when that fails. Otherwise it stores `(flags & 0x8a5f8200) |
event +0x30`, the complement of the relay mask applied to flags that `0x617730` already masked
with 0x75a07dff; the position from `+0x10`, facing `+0x1c` and pitch `+0x20`; and, when the new
flags have 0x2000, the fall time `+0x34` to `+0x78` and `0x7c5f30(fall time)` to `+0x7c`. It ends
with `0x7c5cd0(0)` and `0x7c5c20(0)` and returns 1. The drain `0x615c30` calls it for an event
that carries a status.

Addresses: `0x61a770`, `0x61a799`, `0x618970`, `0x7c5f30`, `0x7c5cd0`, `0x7c5c20`, `0x615c30`
Confidence: verified
Proof: one
Test: none

`and edx,0x8a5f8200; or edx,ecx` at `0x61a799`; 0x8a5f8200 is the bitwise complement of
0x75a07dff. The fall test reads `+0x40` after the store (`test ah,0x20` at `0x61a7c0`), and
`fstp [esi+0x7c]` stores the callee's result. The caller is `0x615e86`, behind
`mov al,[edi+0x4c]; test al,al; je`.

## Mover change reset

`0x619d50(keep)` resets a mover when control changes. A spline marked done is completed again
through `0x619de0` at the step time, the queue is drained (`0x6185f0(1, 1)`), and flag bit 31 is
cleared so the next status reseeds the clock. A spline still running ends it there. Otherwise
`keep` 1 starts a fall and links the mover (`0x617b00`) and, when flags & 0x200f, calls
`0x615b80` at the step time; `keep` 0 stops every motion (`0x61aa70`).

Addresses: `0x619d50`, `0x619d8d`, `0x6185f0`, `0x61aa70`
Confidence: verified
Proof: one
Test: none

`test byte [eax+0x18],4; je` skips the completion for a spline not done; `and edx,0x7fffffff` at
`0x619d8d`. The second spline test, `test byte [eax+0x18],4; je 0x619dd4`, returns for a running
spline. Its two callers, in the owner's mover-change code, push 0 and 1.

## Spline completion

`0x619de0(t)` marks the spline done (`+0x18` |= 4) without a null test. When the owner's GUID is
the active player's, it computes the fraction flown: 1.0 for a zero duration (spline `+0x24`),
0.0 for a negative elapsed time (`+0x20`), 1.0 for spline flag 1, else elapsed / duration at 53
bits rounded to a float, the duration read as signed. It sends the spline id (`+0x28`) and the
fraction with opcode 0x2c9 through `0x600b10`, then frees the spline (`0x7c6310`).

Addresses: `0x619de0`, `0x619def`, `0x619e57`, `0x600b10`, `0xc4da98`
Confidence: verified
Proof: one
Test: `collision::fall_progress_difftest`

`or dword [eax+0x18],4` at `0x619def`. The GUID at owner `+0x08` is compared with `0xc4da98` and
`0xc4da9c`. `test edx,edx; ja` on the duration, `test edi,edi; jge` on elapsed, `test [eax+0x18],1`,
then `fild; fidiv` at `0x619e57` and `fstp`. `0x600b10` pushes `0x2c9` at `0x600b24`, writes the
id and the float, and sends.
The tests run `0x619e4a` to `0x619e5d`.

## Time skip

`0x61ab90(lag)` adds `lag` to `+0xac`, the last sender stamp, so the next status's wire step
excludes the time the sender reported skipped. Nothing else changes.

Addresses: `0x61ab90`
Confidence: verified
Proof: one
Test: none

`mov edx,[ecx+0xac]; add edx,eax; mov [ecx+0xac],edx; ret 4`. Its one caller is the owner's time-skip handler.
