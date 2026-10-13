# collision: events

The move-event queue of a `CMovement` and its producers, in `Movement.cpp`. An event is an
80-byte `CPlayerMoveEvent` node in a list at `CMovement+0x150`, sorted by the time it takes
effect, and carries an internal tag from 0 to 38, never a wire opcode. Three adders allocate,
fill and insert a node; thin producers push one tag each for the local player's commands. The
unit handlers for received moves reach the apply-now functions, which apply a status at once
when it is due and otherwise leave a queued event. Two consumers read the tags: the per-frame
drain in `0x615c30`, which applies due events and sends their opcodes, and the flush `0x6185f0`
with its dispatcher `0x618080`, which empties the queue at once. The tags meet the opcodes in
those two switches. The flush, the applies it shares with the drain and the relay timing are in
`collision/eventapply`.

Roots: `0x617520`, `0x617840`, `0x617890`, `0x6178e0`, `0x617930`, `0x617970`, `0x617ac0`,
`0x617b00`, `0x617b20`, `0x617b60`, `0x617bd0`, `0x617c10`, `0x617ca0`, `0x617cf0`, `0x617d40`,
`0x617d80`, `0x617de0`, `0x617e30`, `0x617e80`, `0x618900`

## Event node

`0x6175d0(time, tag)` takes the first node of the free list `0xc4db88` and unlinks it, or, when
the list is empty, allocates 80 bytes with `SMemAlloc` (`0x6462e0`) under the type name
`.?AUCPlayerMoveEvent@@` (`0x8613e4`) and zeroes the link and position. It then stores the time,
the tag and 0 at +0x4c. A recycled node keeps its old pose and payload. `0x615910` moves a spent first
node to the free list's tail; nodes are freed only by `0x615a50` when no manager exists
(`SMemFree` at `0x615aaa`) and by the manager teardown `0x616910`, which frees the free list.

| offset | field | writers |
|---|---|---|
| +0x00, +0x04 | list link: previous link, next node (bit 0 set marks the end) | list code |
| +0x08 | time the event takes effect | `0x6175d0` |
| +0x0c | tag | `0x6175d0` |
| +0x10, +0x1c, +0x20 | position (3 floats), facing, pitch | `0x617570`, `0x617730` |
| +0x28 | transport GUID | `0x617730` |
| +0x30 | movement flags & 0x75a07dff | `0x617730` |
| +0x34 | fall time | `0x617730` |
| +0x38 | new speed (tags 20 to 25) or knockback direction x | `0x6176a0`, `0x6177a0` |
| +0x3c, +0x40, +0x44 | knockback direction y, horizontal and vertical speed | `0x6177a0` |
| +0x48 | server change counter | the three adders |
| +0x4c | 1 when a status snapshot filled +0x10 to +0x34 | `0x617730` |
| +0x4d | 1 when processing the event sends its opcode to the server | the three adders |

Addresses: `0x6175d0`, `0xc4db88`, `0x6462e0`, `0x8613e4`, `0x615910`, `0x615740`
Confidence: verified
Proof: one
Test: none

`0x6175d0` reads `ecx` (time) and `edx` (tag) and ends in a plain `ret`. It loads the first node
from `0xc4db90` (`0x6175d4`); bit 0 set or zero means empty and goes to `0x617650`: `push 8;
push -2; push 0x8613e4; push 0x50; call 0x6462e0`, then zero stores at +0x00, +0x04, +0x10,
+0x14, +0x18. The unlink at `0x6175f5` to `0x617613` follows the list idiom, a next pointer with
bit 0 set being the list's own link; a node with no previous link skips it (`0x6175ef`), which a
listed node never lacks. Every exit stores +0x0c, +0x4c and +0x08 (`0x617616` to `0x61761e`); a
failed allocation stores through null (`0x617685`). `0x615740` sets the list empty: offset 0, its
link pointing at itself, first node `0xc4db8c | 1`. `0x615910` relinks the popped node after
`[0xc4db8c]`, the free list's last link (`0x615997` to `0x6159a9`). The field meanings follow the
writers below and the two consumers.

## Insertion in time order

Each adder allocates a node, fills it, inserts it and adds the movement to the active list
(`0x619ca0`). The insert `0x615820` links the new node before the first node for which new time
minus node time is negative as a signed 32-bit difference, else at the end, so equal
times keep arrival order. `0x617570` copies a position, facing and pitch; `0x6176a0` a value at
+0x38 and, given a status, the snapshot through `0x617730`; `0x6177a0` the knockback payload and
an optional status. Each stores the send byte at +0x4d and the counter at +0x48. Given a status
and a queue empty before the insert, `0x6176a0` and `0x6177a0` then call `0x619030` and
`0x619090` with the time at globals +0x128 (`0x630a40`).

Addresses: `0x617570`, `0x6176a0`, `0x6177a0`, `0x615820`, `0x6158f0`, `0x619ca0`, `0x619030`, `0x619090`, `0x630a40`
Confidence: verified
Proof: one
Test: none

```
615849: mov    eax,DWORD PTR [edi+0x8]
61584c: sub    eax,DWORD PTR [ebx+0x8]
61584f: js     0x615856
615851: mov    ebx,DWORD PTR [ebx+0x4]
```

A negative `new − node` links before `ebx` (`0x615856`); the end marker (`0x615840`) links at
the tail (`0x615894`). The header is `CMovement+0x150` (`0x6175ae lea ecx,[esi+0x150]`), its
first node at +0x158, which `0x6158e0` reads and `0x6158f0` tests. In `0x6176a0` the emptiness
test `0x6176cf` runs before the snapshot (`0x6176e3`) and the insert (`0x6176eb`); its result,
kept in the third argument's slot, is tested at `0x617701`. The returns are `ret 0x1c`, `ret
0x18` and `ret 0x20`, and each adder passes `ecx` and `edx` to `0x6175d0`. `0x619ca0` links the
movement into the list at movement globals +0x11c when its own link at +0x04 is empty.

## Filling from a status

`0x617730(status, event)` copies the status's transport GUID (+0x08) to +0x28 and its flags
(+0x10) masked with 0x75a07dff to +0x30. With a nonzero GUID it copies the transport-local
position and facing (+0x14, +0x20), else the world ones (+0x24, +0x30), to +0x10 and +0x1c; then
pitch (+0x34) to +0x20, fall time (+0x38) to +0x34, and 1 to +0x4c. `0x618900(time, status)`
builds an outgoing status: it stores the `CMovement` flags & 0x75a07dff at status +0x10 first,
then calls `0x7c6340`, which reads that word back and sets 0x2000000 when on a transport, else
clears it. The mask clears 0x200, 0x8000, 0x10000 to 0x100000, 0x400000, 0x2000000, 0x8000000 and
0x80000000.

Addresses: `0x617730`, `0x61773f`, `0x618900`, `0x618909`, `0x7c6340`
Confidence: verified
Proof: one
Test: none

`0x617730` reads `ecx` (status) and `edx` (event) and ends in `ret`. `0x61773f and
eax,0x75a07dff` precedes the store to +0x30; the GUID test is `or eax,esi; je 0x61776e`;
`0x61778d` stores 1 at +0x4c. In `0x618900`, `0x618909 and edx,0x75a07dff` and the store to
`[eax+0x10]` come before `call 0x7c6340` (`ret 8`). `0x7c6340` stores the time at status +0x00
and then `mov eax,[esi+0x10]; or eax,0x2000000` with a transport, `and eax,0xfdffffff` without.
The status layout follows `0x7c6340`: `CMovement+0x10` at +0x14, `+0x1c` at +0x20, the world
position from `0x7c4930` at +0x24, the world facing from `0x7c4ae0` at +0x30, pitch at +0x34,
`CMovement+0x78` at +0x38.

## Tags and wire opcodes

Every tag is internal: 0 to 38 at +0x0c, none equal to the opcode it ends in. Tags meet the wire
in two switches. The drain `0x615c30` indexes a 39-dword table with the tag (`0x615cfd`), and
each case passes its opcode to `0x60e0a0` with +0x4d as the send byte; `0x60e0a0` builds a packet
only when that byte is nonzero (`0x60e1bb`). The flush dispatcher `0x618080` maps tags 6 to 38
(`0x6180b7`) and sends through `0x617fe0`. The local player's producers queue with +0x4d = 1; the
apply-now path through `0x618f20` and `0x618eb0` queues with 0. The four entries below give each
tag's event, the drain's opcode, the queue producers and the apply-now function.

Addresses: `0x615c30`, `0x615cfd`, `0x60e0a0`, `0x60e1bb`, `0x618080`, `0x6180b7`, `0x617fe0`, `0x618f20`, `0x618eb0`
Confidence: verified
Proof: one
Test: `objectlayer::move_event_move_deadband`

`0x615cf4 cmp eax,0x26; ja 0x615e51`, then the table jump over 39 dwords that follow the
function. Each case pushes its opcode as the second argument of `call 0x60e0a0`, for example
`0x615d28 push 0xb5`. In `0x60e0a0`, `mov al,[ebp+0x10]; test al,al; je 0x60e300` skips the send
path. The opcodes are given by value; the protocol's names are not in the image. A producer's tag
is the immediate pushed as the adder's second argument, or `neg; sbb; add k` on a flag argument,
which gives `k − 1` for nonzero and `k` for zero, or `sete`, which gives 1 for zero. `0x618f20`
returns nonzero when it applied the status at once, and otherwise queues the tag through
`0x6176a0` at the status's fire time (`0x618f70`).
The tests run `0x60e1f0` to `0x60e205`.

## Movement key tags

The queue producers here wrap `0x617570` with a zero position, facing, pitch and counter and
+0x4d = 1. The event column names the setter the drain calls; the setters write flag 0x1 forward,
0x2 backward, 0x4 left, 0x8 right, 0x10 turn left, 0x20 turn right.

| tag | event, drain setter | drain opcode | queued by | applied now by |
|---|---|---|---|---|
| 0 | start forward, `0x7c6ae0(1)` | 0xb5 | `0x617840`, argument nonzero | `0x617520` |
| 1 | start backward, `0x7c6ae0(0)` | 0xb6 | `0x617840`, argument zero | `0x617520` |
| 2 | stop, `0x7c6ba0` | 0xb7 | `0x617b20` | `0x617b60` |
| 3 | strafe left, `0x7c6c50(1)` | 0xb8 | `0x617890`, argument nonzero | `0x6178e0` |
| 4 | strafe right, `0x7c6c50(0)` | 0xb9 | `0x617890`, argument zero | `0x6178e0` |
| 5 | stop strafe, `0x7c6d30` | 0xba | `0x617bd0` | `0x617c10` |
| 6 | start falling, `0x7c61c0` | 0xee through `0x600a30` | `0x617ac0` | `0x617b00`, no status |
| 7 | jump, `0x7c6230(1)` | 0xbb | `0x617930` | `0x617970` |
| 8 | turn left, `0x617c80(1)` | 0xbc | `0x617ca0`, argument nonzero | `0x617cf0` |
| 9 | turn right, `0x617c80(0)` | 0xbd | `0x617ca0`, argument zero | `0x617cf0` |
| 10 | stop turn, `0x7c6dd0` | 0xbe | `0x617d40` | `0x617d80` |

Addresses: `0x617840`, `0x617858`, `0x617890`, `0x6178a4`, `0x617930`, `0x617ac0`, `0x617b20`, `0x617bd0`, `0x617ca0`, `0x617d40`, `0x600a30`
Confidence: verified
Proof: one
Test: none

`0x617858 sete dl` on the second argument; `0x6178a4 neg edx; sbb edx,edx; add edx,4`; pushes of
7 (`0x617945`), 6 (`0x617ad5`), 2 (`0x617b35`), 5 (`0x617be5`) and 0xa (`0x617d55`); `add edx,9`
at `0x617cbc`. Each pushes 1 as the send byte and the address of a zeroed vector. The drain's
cases push the opcodes in the table; the stop cases share a send tail. The tag 6 case calls `0x7c61c0` and,
when it starts a fall and +0x4d is set, `0x600a30` with 0xee (`0x615e3b`). The flag bits are
`0x7c6b3b and eax,0xfffffffd; or eax,1` against `0x7c6b46 or eax,2`, `0x7c6cae or eax,4` against
`0x7c6cb6 or eax,8`, and `0x7c6da0 or eax,0x10` against `0x7c6db7 or edx,0x20`.

## Pitch, mode and swim tags

| tag | event, drain setter | drain opcode | queued by | applied now by |
|---|---|---|---|---|
| 11 | pitch up, `0x7c6df0(1)` | 0xbf | `0x61a240`, argument nonzero | `0x61a290` |
| 12 | pitch down, `0x7c6df0(0)` | 0xc0 | `0x61a240`, argument zero | `0x61a290` |
| 13 | stop pitch, `0x7c6e30` | 0xc1 | `0x61a2e0` | `0x61a320` |
| 14 | run mode, `0x7c71c0(1)` clears 0x100 | 0xc2 | `0x617de0`, argument nonzero | `0x617e30` |
| 15 | walk mode, `0x7c71c0(0)` sets 0x100 | 0xc3 | `0x617de0`, argument zero | `0x617e30` |
| 16 | set facing, `0x7c6f30` | 0xda | `0x619860` | `0x619280` |
| 17 | set pitch, `0x7c6f70` through `0x615bc0` | 0xdb, or 0xee | `0x6198a0` | `0x619960` |
| 18 | start swim, `0x61a230` | 0xca | `0x619fb0` (send 1), `0x61a030` (send 0) | `0x61a190` |
| 19 | stop swim, `0x7c6e80` | 0xcb | `0x61a070` (send 1), `0x61a0f0` (send 0) | `0x61a1f0` |
| 37 | no case of its own | none | none | `0x61a750` |

`0x617de0` is reached only through 0x60e060, which 0x60e080 calls with the walk bit,
`[unit+0x9e8] & 0x100`, so that a walking unit queues 14 and a running one 15; 0x5ee10e calls
0x60e060 with 1, which queues 14.

Addresses: `0x617de0`, `0x617dfc`, `0x617e30`, `0x617e48`, `0x615bc0`, `0x61a750`
Confidence: verified
Proof: one
Test: none

`0x617df4 neg edx; sbb edx,edx; add edx,0xf` (`0x617dfc`) and the same at `0x617e48` give 14 for
nonzero. `0x7c71c0` clears 0x100 for a nonzero argument (`0x7c71d0`) and sets it otherwise.
`0x615bc0` sends 0xdb when `0x7c6f70` returns 0 and 0xee through `0x600a30` otherwise, the latter
only with +0x4d set. The status producers push 0x10 to 0x13 to `0x617570` or `0x6176a0`, and
the `add edx,0xc` before the call at `0x61a276` gives 11 or 12. Tag 37 reaches the drain's default exit `0x615e51`
through its table entry and has no dispatcher case. The caller census of `0x617de0` is one site,
0x60e071 in 0x60e060, whose only callers are 0x60e093 and 0x5ee10e; the one dword equal to its
address, at 0xcfc934, lies in the `.rsrc` resource data, not in code or a table.

## Forced change tags

| tag | event, setter | opcode | queued by (send 1, send 0) | applied now by |
|---|---|---|---|---|
| 20 | run speed, `0x7c7030` to `CMovement+0x8c` | 0xe3 | `0x619500`, `0x619530` | `0x619550` |
| 21 | run back speed, `0x7c7080` to +0x90 | 0xe5 | `0x619590`, `0x6195c0` | `0x6195e0` |
| 22 | walk speed, `0x7c70d0` to +0x88 | 0x2db | `0x619620`, `0x619650` | `0x619670` |
| 23 | swim speed, `0x7c7120` to +0x94 | 0xe7 | `0x6196b0`, `0x6196e0` | `0x619700` |
| 24 | swim back speed, `0x7c7170` to +0x98 | 0x2dd | `0x619740`, `0x619770` | `0x619790` |
| 25 | turn rate, `0x7c6ff0` to +0x9c | 0x2df | `0x6197d0`, `0x619800` | `0x619820` |
| 26, 27 | `0x7c7240(1)` clears 0x800, `(0)` sets it | 0x2ad | none | `0x619c50`, 27 when the status has 0x800 |
| 28 | knockback, `0x6179c0` | 0xf0 | `0x617a30` | `0x617a60` |

The speed producers queue through `0x6176a0` with the speed at +0x38. Both consumers pass +0x38
to the setter and send the same opcode with the speed and the counter.

Addresses: `0x61810a`, `0x61812d`, `0x61813f`, `0x618174`, `0x6181a9`, `0x6181de`, `0x618213`, `0x617a30`, `0x617a4a`, `0x619c50`
Confidence: verified
Proof: one
Test: none

Each speed case of `0x618080` loads `[esi+0x38]`, calls its setter, then pushes the opcode
(`0x61812d push 0xe3`, `0x618162 push 0xe5`, `0x618197 push 0x2db`, `0x6181cc push 0xe7`,
`0x618201 push 0x2dd`, `0x618236 push 0x2df`) for `0x617fe0`. Each setter stores the argument
only when it differs from the old speed by at least 2^−22 (`0x34800000`) or the difference is
NaN (`fabs; fcomp; test ah,5; jp`). The
drain's cases 20 to 25 push the same values. Its cases 26 and 27 push 1 and 0 to `0x7c7240`
and 0x2ad. `0x619c50` computes its tag from status +0x10 as `((flags & 0x800) | 0xd000) >> 11`
(`0x619c5c` to `0x619c6d`), 26 or 27, for `0x618f20`; no queue producer makes 26 or 27, a scan
of `Movement.cpp` for the immediates 0x1a and 0x1b finding none (control: 0x1c at `0x617a4a`
and `0x617a7a`). `0x617a30` pushes 0x1c and send 1 for `0x6177a0` (`0x617a4a`).

## Mode toggle tags

| tag | event, dispatcher action | opcode, value | queued by (send 1, send 0) | applied now by |
|---|---|---|---|---|
| 29 | feather fall on: `0x7c72e0(1)` sets 0x20000000 | 0x2cf, 1.0 | `0x61a490`, `0x61a500` | `0x61a550` |
| 30 | feather fall off: `0x7c72e0(0)` | 0x2cf, 0 | `0x61a490`, `0x61a500` | `0x61a550` |
| 31 | hover on: `0x61a580(1, v)` | 0xf6, 1.0 | `0x61a5d0`, `0x61a660` | `0x61a6b0` |
| 32 | hover off: `0x61a580(0, v)` | 0xf6, 0 | `0x61a5d0`, `0x61a660` | `0x61a6b0` |
| 33 | water walk on: `0x7c7280(1)` sets 0x10000000 | 0x2d0, 1.0 | `0x61a380`, `0x61a3e0` (unreached) | `0x61a430` |
| 34 | water walk off: `0x7c7280(0)`, then `0x7c61c0` when v | 0x2d0, 0 | `0x61a380`, `0x61a3e0` (unreached) | `0x61a430` |
| 35 | root: `0x616d30` when sent with a live spline, then `0x619a00` | 0xe9, 0 | `0x61a700` | `0x6199c0` |
| 36 | unroot: `0x7c7370(v)` clears 0x1000, then `0x7c61c0` when v | 0xeb, 0 | `0x61a700` | `0x619a60` |
| 38 | teleport, `0x617e90` | 0xc7, 0 | `0x618720` | `0x6186b0` |

`v` is 1 when +0x4d is 0 or the unit's GUID equals `0xc4da98`, the GUID of the unit this client
moves, and 0 otherwise.

Addresses: `0x618248`, `0x6182d1`, `0x6182e3`, `0x618394`, `0x6183c8`, `0x618425`, `0x618472`, `0xc4da98`, `0x619a00`
Confidence: verified
Proof: one
Test: none

The feather fall cases push 0x3f800000 or 0 with 0x2cf, then call `0x7c72b0` and a unit
animation hook. The `v` test is `mov al,[esi+0x4d]; test al,al; je` to 1, else the GUID at
`[[edi+0x15c]+0x8]` against `0xc4da98` and `0xc4da9c` (`0x6182e3`), equal giving 1. The root
case tests +0x4d, the spline at `CMovement+0xa4` and its bit 0x4 clear before `0x616d30`;
`0x619a00` calls `0x61aa70` and tail-jumps to `0x7c7340`, which sets 0x1000; `0x7c7370` clears it
(`0x7c7379`) and calls `0x7c61c0` only for a nonzero argument (`0x7c7384`). `0x61a580` takes
the on flag first and `v` second (`push eax; push 1` at `0x6182ff`).

## Apply now or queue

Each apply-now function passes its tag to `0x618f20(time, status, tag, value)`. When that returns
0 the event stays queued and the function returns 0; otherwise it applies:

| function | tag | on apply | returns |
|---|---|---|---|
| `0x617520`, `0x6178e0` | 0/1, 3/4 | `0x7c6ae0` or `0x7c6c50`, then `0x619ca0` if it returned nonzero | 1 when both ran |
| `0x617970` | 7 | `0x7c6230(1)`, then `0x619ca0` if nonzero | 1 when both ran |
| `0x617cf0` | 8/9 | `0x617c80(left)`, `0x619ca0` | 1 |
| `0x617b60` | 2 | `0x7c6b70(flags before)`; idle check | 1 when it stopped and no 0xc bit is set |
| `0x617c10` | 5 | `0x7c6d00(flags before)`; idle check | 1 when it stopped and no 0x3 bit is set |
| `0x617d80` | 10 | `0x7c6dd0`; idle check | what `0x7c6dd0` returned |
| `0x617e30` | 14/15 | `0x7c71c0(run)` | 1 |

The idle check removes the movement from the active list, `0x619e80(1)`, when no flag of 0x20ff
is set and the queue is empty; `0x618940` makes that test and otherwise adds it.

Addresses: `0x617520`, `0x6178e0`, `0x617970`, `0x617cf0`, `0x617b60`, `0x617c10`, `0x617d80`, `0x617e30`, `0x618940`, `0x617b00`, `0x617c80`, `0x617e80`, `0x619e80`
Confidence: verified
Proof: one
Test: none

`0x617b60` loads the flags into `edi` before the call (`0x617b6f`); `0x7c6b70` stops only when
that word has 0x3 and the current one lacks 0x2000. The idle tests are `test eax,0x20ff`
(`0x617b94`) and `0x6158f0`, then `push 1; call 0x619e80`; `0x618940` ends `jmp 0x619ca0`.
`0x617d80` keeps `0x7c6dd0`'s result in `edi` across the check (`0x617daa`). `0x617b00` calls
`0x7c61c0` and, on nonzero, `0x619ca0`, returning 1. `0x617c80` calls `0x7c6d90(left)` and
stores 0 at +0x148. `0x617e80` pushes its argument to `0x7c71c0` with `ecx` unchanged (`ret 4`).

## Root filter

While `CMovement` flag 0x1000 is set, the drain drops each due event whose tag `0x618030` maps
to 0: tags 0 to 7, 18, 19 and 28 (moves, strafes, the fall start, jump, swim and knockback).
Tags 8 to 17, 20 to 27 and above 28 pass. A dropped event is popped and discarded: no state, no
opcode. `0x7c7340` sets 0x1000 and `0x7c7370` clears it.

Addresses: `0x618030`, `0x61804c`, `0x618054`, `0x615c71`, `0x615c79`
Confidence: verified
Proof: one
Test: none

```
618030: cmp    ecx,0x1c
618033: ja     0x618046
618035: movzx  eax,BYTE PTR [ecx+0x618054]
61803c: jmp    DWORD PTR [eax*4+0x61804c]
618043: xor    eax,eax
618046: mov    eax,0x1
```

The dwords at `0x61804c` are `0x618043` and `0x618046`. The 29 bytes at `0x618054` are `00` × 8,
`01` × 10, `00 00`, `01` × 8, `00`. `0x615c71 test ah,0x10` gates `0x615c79 call 0x618030`, whose
0 jumps to `0x616539`, the drain's loop test, after the pop at `0x615c69`.
