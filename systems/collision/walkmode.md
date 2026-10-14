# collision: walk mode

The walk and run mode of a unit, end to end: bit `0x100` of `CMovement+0x40`, which queues it,
what goes on the wire, every message and path that rewrites it, the speeds it selects and how
it reaches the animation. The setter, the speed selector and the six speed fields are
`collision/commands` and `collision/kinematics`; the queue is `collision/events`; the Lua
function that starts it is `input#toggle-run`.

Roots: none

## Walk toggle request

`ToggleRun` queues a mode flip read off the live bit: `0x60e080` loads the unit's flags `+0x9e8`
and passes `flags & 0x100` and the input clock `0xcf0bc8` to `0x60e060`, which calls
`0x617de0` on the `CMovement`. A set bit gives kind `0xe`, a clear bit kind `0xf`; the event
carries send byte 1, a zero position vector and no counter. So running becomes walking by kind
`0xf`, and walking becomes running by kind `0xe`. The kinds apply through the drain, never in
the request.

Addresses: `0x60e080`, `0x60e083`, `0x60e08c`, `0x60e060`, `0x60e071`, `0x617de0`, `0x617df4`, `0x617df8`, `0x617dfc`, `0x617e16`, `0x617570`, `0x617585`, `0x6175d0`, `0xcf0bc8`
Confidence: verified
Proof: one
Test: none

`0x60e08c and eax,0x100` and `0x617df4 neg edx; sbb edx,edx; add edx,0xf` give `0xf − (bit ≠ 0)`.
`0x617e16` calls `0x617570(time, kind, 1, 0, &{0,0,0}, 0, 0)`; `0x617585` stores the send byte at
`+0x4d`. `0x617570` allocates with `0x6175d0`, inserts in time order (`collision/events`) and
activates the mover.

## Walk mode wire

A local flip sends `MSG_MOVE_SET_WALK_MODE` (`0xc3`) when it enters walk and
`MSG_MOVE_SET_RUN_MODE` (`0xc2`) when it leaves it, from the drain, after the bit is written.
Kind `0xe` clears the bit and sends `0xc2`; kind `0xf` sets it and sends `0xc3`. The packet's
status flags already hold the new bit: `0x618900` copies the live flags through the mask
`0x75a07dff`, which keeps `0x100`. The body is the opcode and the status only: neither opcode is
in the counter set `0x7ccb90`, the GUID set `0x7ccc10` beyond its own check, or the float set
`0x6009b0`. No reply is handled. After the send `0x600ad6` re-arms the heartbeat to the message
time plus 500.

Addresses: `0x615f52`, `0x615f56`, `0x615f6e`, `0x615f80`, `0x615f84`, `0x615f96`, `0x60e0a0`, `0x600a30`, `0x600860`, `0x600918`, `0x618900`, `0x618903`, `0x618909`, `0x7c71c0`, `0x7c71e0`, `0x7ccb90`, `0x7ccc10`, `0x6009b0`, `0x600ad6`, `0x615b80`
Confidence: verified
Proof: one
Test: none

The drain reads `0x616580` entry 14 (`0x615f52`) and entry 15 (`0x615f80`). Each calls
`0x7c71c0` (`0x615f56` with 1, `0x615f84` with 0) and then `0x60e0a0(time, op, +0x4d, 0, 0)` with
`0xc2` (`0x615f6e`) or `0xc3` (`0x615f96`); the counter argument is the literal 0. `0x7c71c0` stores
the flags at `0x7c71e0` before it re-anchors, so the bit is set before `0x618900` runs. `0x7ccc10`
and `0x7ccb90` list 13 opcodes with `0xf0` last and no `0xc2` or `0xc3`; a body for opcodes outside
`0x7ccc10` carries no GUID at all, and the status follows the opcode directly.

## Walk mode messages

A unit's mode is also set by four inbound messages and by every spline. `0xc2` and `0xc3` bind
`0x603bb0`; `0x601580` reaches their arms `0x6016bd` (run 1) and `0x6016d0` (run 0), which call
`0x602e30`, then `0x617e30`: the status gate `0x618f20` with kind `0xe` or `0xf`, then `0x7c71c0`
now, or a queued event with send byte 0 that the drain applies later without a reply. `0x30d` (run)
and `0x30e` (walk) bind `0x603c80`; `0x601420` reaches `0x6014db` (push 1) and `0x6014df` (push 0),
which call `0x617e80`, a bare `0x7c71c0` with no gate and no reply. A spline sets the mode inverted:
`0x7c6a50` passes `splineFlags & 0x100` to `0x7c71c0`, so a spline with `0x100` set runs and
one without walks.

Addresses: `0x603bb0`, `0x60346b`, `0x60347b`, `0x601580`, `0x6016bd`, `0x6016d0`, `0x602e30`, `0x617e30`, `0x617e50`, `0x617e5c`, `0x603c80`, `0x60381b`, `0x60382b`, `0x601420`, `0x6014db`, `0x6014df`, `0x617e80`, `0x617e87`, `0x7c6a50`, `0x7c6a63`, `0x7c6ac2`, `0x7c6acb`, `0x618801`, `0x6180ee`, `0x6180fe`
Confidence: verified
Proof: one
Test: none

`0x617e30` computes the kind as `0xf − (run ≠ 0)` (`0x617e3d neg; sbb; add 0xf`) and calls `0x618f20`;
zero returns 0 without a write. `0x7c6a50` is `CMovement`'s spline installer: it stores its fourth
argument at `spline+0x18` (`0x7c6a63`), the word whose bit `0x4` is the finalize latch, then
`and edi,0x100` and `call 0x7c71c0`. `0x7c6a50` has one call site, `0x618801`, and no stored
reference. The flush `0x618080` replays queued mode events through `0x6180ee` (run) and `0x6180fe`
(walk). The moves that carry a status (`0xb5` to `0x2b1`) also write the bit; see Mode bit writers.

## Mode bit writers

Bit `0x100` of `+0x40` changes at these places: `0x7c71c0` from seven call sites (`0x615f56`,
`0x615f84`, `0x617e5c`, `0x617e87`, `0x6180ee`, `0x6180fe`, `0x7c6acb`); the merge `0x618c30`,
which rewrites the word as `local ^ ((local ^ wire) & mask)` with the live mask `0x75a07dff`
(contains `0x100`); the create block `0x619320`, which zeroes the word at `0x61936e` and then
merges; the forced snapshot `0x61a770`, which keeps `local & 0x8a5f8200` and ORs the wire word
(so `0x100` comes from the wire); and the constructor `0x7c4850`, which zeroes it. The setters
for the other state changes mask other bits, and
`SetRoot`'s wipe `0xffe07f00` keeps `0x100`. A world port rebuilds the unit and takes the bit from the create block; a near teleport keeps it.

Addresses: `0x7c71c0`, `0x7c71d0`, `0x7c71d7`, `0x615f56`, `0x615f84`, `0x617e5c`, `0x617e87`, `0x6180ee`, `0x6180fe`, `0x7c6acb`, `0x618c30`, `0x618dbe`, `0x618df3`, `0x619320`, `0x61936e`, `0x61941f`, `0x61a770`, `0x61a796`, `0x61a799`, `0x61a7a1`, `0x7c4850`, `0x7c489f`, `0x7c7340`, `0x7c734f`
Confidence: verified
Proof: one
Test: none

Relative `call`/`jmp` scan of `.text` for `0x7c71c0`: exactly the seven sites, and no stored
address in any section (control: `0x5ed570` finds its vtable slot `0x80b01c`). In the unit, movement and
resolution code and the shared movement file, the only immediate `and` mask lacking bit 8 or `or` mask
with it that acts on a `CMovement` flag word is `0x7c71d0` and `0x7c71d7`; `0x60b180` and
`0x60b187` act on the unit word `+0xc58`. The whole-word stores to `[reg+0x40]` or `[reg+0x9e8]` there
number 72; the ones that take the word from a message or reset it are `0x618dbe`, `0x618df3`,
`0x61936e`, `0x61a7a1` and `0x7c489f`, and `0x617fe0`'s masked store keeps bit 8 (`or
eax,0xfffff7ff`). `0x618dbe` is the arm with mask `0x75a01dff`, which no caller selects
(`collision/status#status-merge-and-apply`); both masks keep `0x100`.

## Walk bit consumers

Four places read the bit and one copies it. `0x7c4d11` selects the walk arm of the
current-speed selector. `0x60e08c` reports it to the toggle request. `0x602cb5`, in the landing
animation picker `0x602c60`, sends a walking, non-backward unit with a direction bit to the
re-pick (`0x602c82`) instead of JumpLandRun. `0x610bca`, in the remote auto-move `0x6106b0`, takes
the walk speed `+0xa30` over the run speed `+0xa34` for a catch-up distance test. `0x618903`
copies the bit into every sent status. The moving animation `0x5fd100` never reads it (below).

Addresses: `0x7c4d11`, `0x60e08c`, `0x602c60`, `0x602c9c`, `0x602cb5`, `0x602c82`, `0x610bc4`, `0x610bca`, `0x6106b0`, `0x618903`, `0x5fd100`, `0x7c4c90`
Confidence: verified
Proof: one
Test: none

Scan: each of the 456 loads of `[reg+0x40]` or `[reg+0x9e8]` into `eax`, `ebx`, `ecx` or `edx` in
the image, followed along both arms of each branch for 60 instructions (a call, redefinition or `ret`
ends a path), for `test` of bit 8 or `and` with `0x100`. It finds exactly `0x602cb5`, `0x60e08c`,
`0x610bca` and `0x7c4d11` (the control). Direct memory `test`, `and` and `or` on `+0x40`, `+0x41`,
`+0x9e8`, `+0x9e9` with bit 8 only preserve it.

## Moving animation choice

`0x5fd100` picks the moving clip by the speed `0x7c4c90(0)` returns, so the walk bit reaches
the Walk clip only by lowering that number. Order: no direction bit declines; swimming; a live
spline with bit `0x200` gives 135 Fly; backward gives 13 (Walkbackwards) for walking and running
alike; stealth 119; speed of 11 or more 143; above twice the walk speed `+0xa30` Run (5); else
Walk (4). With the vanilla speeds (walk 2.5, run 7.0) walking gives `min(2.5, 7.0)` against a
boundary of 5.0.

Addresses: `0x5fd100`, `0x5fd106`, `0x5fd10c`, `0x5fd137`, `0x5fd18c`, `0x5fd1bc`, `0x5fd1c3`, `0x5fd1d9`, `0x5fd1fd`, `0x5fd202`, `0x5fd224`, `0x5fd230`, `0x5fd23a`, `0x5fd24d`, `0x7c4c90`, `0x80c484`
Confidence: verified
Proof: one
Test: none

`0x5fd1bc test al,2` returns 13 before the first call to `0x7c4c90` (`0x5fd1fd`, argument 0).
`0x5fd202` compares the result with 11.0 (`0x80c484`); `0x5fd224` loads `+0xa30` (`CMovement+0x88`),
doubles it, and `0x5fd230` compares the speed with it (`test ah,0x41; jne` to Walk, so equal and NaN
walk). The fly test is `0x5fd19f test cl,4` (finalized: skip) and `0x5fd1a4 test ch,2`.

## Walk playback rate

`0x5fe2f0` scales a locomotion clip's rate to the mover: rate = `0x7c4c90(0)` ÷
(`‖row 2 of the model matrix‖` × the sequence's authored speed), where `0x711a20` fills the denominator at
`out+8` from the model's matrix words `+0xbc`, `+0xc0`, `+0xc4` and the sequence field `+0x0c`
(stride `0x44`). The rate stays 1.0 when the animation is not one of `0x5fee80`'s moving ids or
when the denominator is not above 0. No walk-speed term enters it, and nothing branches on the
unit being the local player.

Addresses: `0x5fe2f0`, `0x5fe4be`, `0x5fe50a`, `0x5fe515`, `0x5fe520`, `0x5fe526`, `0x5fe528`, `0x5fe53c`, `0x5fe541`, `0x5fe54d`, `0x711a20`, `0x711ab6`, `0x711af0`, `0x711af3`, `0x5fee80`
Confidence: verified
Proof: one
Test: none

`0x5fe4be` stores `0x3f800000` in the rate local. `0x5fe50a` fills the output `[ebp-0x164]`; its
word at `+8` is tested with `fcomp 0.0; test ah,0x41; jne` (`0x5fe515`, `0x5fe520`, so zero, a
negative or NaN keeps 1.0), the animation id with `0x5fee80`, and `0x5fe541 fdiv` divides the speed
(`0x5fe53c`, argument 0) by it before the store at `0x5fe54d`. `0x711ab6` loads three words of the
matrix, squares and sums them, `fsqrt`s (`0x711ae8`), multiplies by `[eax-0x18]` (`0x711af0`) and
stores at `0x711af3`.

## Create block speeds

The speed block of a unit's create message fills `+0x88` to `+0x9c` for every unit, the local
player included. `0x619320` copies wire `+0x50` to `+0x64` into the six fields in order. It runs from
the unit's post-create `0x5fad50` with `isSelf` 0, after the constructor zeroed the fields in the
same call; the guard that skips the apply for the local player belongs to the re-create of an
object the client already holds (`0x466350`, called from the create handler only when its lookup
hits). A create whose update flags lack LIVING (`0x20`) skips the movement parse, so the block
holds its constructor zeros.

Addresses: `0x619320`, `0x619333`, `0x619360`, `0x61936e`, `0x5fad50`, `0x5fad98`, `0x5faddc`, `0x5fb11b`, `0x5fb131`, `0x5ff070`, `0x5ff086`, `0x5ff089`, `0x5ff030`, `0x466350`, `0x46613c`, `0x4660a0`, `0x46618d`, `0x466498`, `0x7c4850`, `0x616fa0`
Confidence: verified
Proof: one
Test: none

`0x5fad98 xor edi,edi` and nothing before `0x5fb11b push edi` changes it; `0x5ff070` tests that
argument at `0x5ff086` and skips `0x5ff030` only when it is nonzero. Relative-call scan:
`0x5ff070` has two callers, `0x5fb131` and `0x466498` (the latter in `0x466350`); `0x466350`
has one, `0x46613c`, after `0x464530` finds the GUID (`je 0x46615e` on a miss builds the object);
`0x616fa0` has one (`0x5faddc`); `0x46618d test al,0x20` guards the parse `0x47f050`.

## Walk speed families

Each of the six speed fields has three inbound messages and one setter, and only the forced
family acknowledges. The relay family (`0xcd`, `0xcf`, `0xd1`, `0xd3`, `0xd5`, `0xd8`) queues a
change with send byte 0; the forced family (`0xe2`, `0xe4`, `0xe6`, `0x2da`, `0x2dc`, `0x2de`)
queues it with send byte 1 and the counter, and the drain answers with `0xe3`, `0xe5`, `0xe7`,
`0x2db`, `0x2dd`, `0x2df`; the spline family (`0x2fe` to `0x303`) calls the setter at once. The
walk speed is `+0x88`: relay `0xd1`, forced `0x2da`, ack `0x2db`, spline `0x301`. No handler is
registered for any ack opcode.

Addresses: `0x6010e0`, `0x601165`, `0x601176`, `0x601243`, `0x601360`, `0x601368`, `0x602780`, `0x602992`, `0x602a17`, `0x619620`, `0x619630`, `0x619640`, `0x619650`, `0x61965e`, `0x619670`, `0x616180`, `0x61619c`, `0x618080`, `0x618197`, `0x60e1b8`, `0x7c70d0`
Confidence: verified
Proof: one
Test: none

`0x619630 push 1` stamps the forced walk producer's send byte, `0x61965e` and `0x61965c` push 0 for
the relay. The send gate is `0x60e1b8 mov al,[ebp+0x10]; test al,al; je` (`collision/events`).
`0x6010e0` selects between two tables by whether the GUID is the active unit (`0x601176`,
`0x601243`). The ack `push 0x2db` is at `0x61619c` (drain) and `0x618197` (flush); no
`mov ecx,0x2db` registration exists in the image.

## Walk mode in the speed selector

The mode bit changes the speed only after two higher tests, and it has no backward form. In
`0x7c4c90`: no direction bit gives 0; a live spline gives its own speed; swimming gives the swim
speeds; only then `0x100` (or a nonzero argument) gives the smaller of `+0x88` and `+0x8c`, whether
moving forward or backward. So a walking unit moves at the same speed in both directions, and
the standstill jump re-seed `0x7c6ae0` passes 1 to take the same value.

Addresses: `0x7c4c90`, `0x7c4caa`, `0x7c4cd9`, `0x7c4d11`, `0x7c4d14`, `0x7c4d1d`, `0x7c4d4d`, `0x7c6ae0`
Confidence: verified
Proof: one
Test: `collision::move_setters_difftest`

`0x7c4d14 jne 0x7c4d4d` fires on `test dh,1` before `0x7c4d1d test dl,2`, so the backward pair
never applies in walk mode; the table is `collision/kinematics#current-speed-selector`.
