# collision: riding a transport

How a unit rides: the monster move message that names a transport, the move base that holds the
transport's GUID, the frame a rider's pose is stored in, and the height rules that apply to a spline.
The transport itself is `objectlayer/transports`; the matrix and the boarding are
`collision/world#transport-matrix` and `collision/falling#boarding-and-leaving-transports`.

Roots: none

## Transport monster moves

The monster move messages `0xdd` and `0x2ae` share one handler, and the transport form differs only
by one packed GUID. `0x603f00` reads the mover's packed GUID, finds the unit with type mask 8 (a miss
consumes the packet), and compares the opcode with `0x2ae`: only then it reads a second packed GUID.
It passes that GUID, or the pair 0 for `0xdd`, to the move-base setter `0x617100` before any body
field is read, then calls the body `0x6018f0`, which never reads the opcode. So a plain `0xdd`
detaches the unit from a transport, and a `0x2ae` naming an object the client lacks or one that is
not a transport leaves the base alone while the spline still decodes.

Addresses: `0x603f00`, `0x603f13`, `0x603f2f`, `0x603f38`, `0x603f3a`, `0x603f40`, `0x603f47`, `0x603f56`, `0x603f60`, `0x603f69`, `0x617100`, `0x617180`, `0x61719a`, `0x630a80`, `0x6018f0`, `0x6018f8`, `0x6035e6`, `0x6035f6`, `0x6035eb`, `0x6035fb`
Confidence: verified
Proof: one
Test: none

`0x603f3a cmp ebx,0x2ae; jne 0x603f56` is the only opcode test in the chain. `0x603f56 push 0; push 0`
is the 0xdd pair. A scan of the whole body of `0x6018f0` finds no read of `[ebp+0xc]`, the opcode
argument (the control is the read of `[ebp+8]` at `0x6018f8`). `0x630a80` accepts a GUID only for a game
object whose flags dword at descriptor `+0xc` has bit 3, the transport flag; `0x617170` returns 0
without a change otherwise.

## Monster move body

The body is the same for both opcodes: the start point, a u32 spline id, a u8 type, a facing payload by
the type (none, a vector, a raw 8-byte GUID, a float) and, for types other than 1, the spline flags, a
duration and a point count with the points. Type 1 is the stop form: flags 0x100, duration 0 and one
point replace those fields, so a `0x2ae` stop is the two packed GUIDs, three floats, the spline id and
the type byte. A stop within 3 yards of the unit flushes and stops at the point (`0x618750`) and jumps
past the shared tail, so it does not clear the precast, the ranged pose or the hold.

Addresses: `0x6018f0`, `0x601982`, `0x6019be`, `0x6019ca`, `0x6019cf`, `0x6019df`, `0x6019e4`, `0x601a1d`, `0x601a21`, `0x601a28`, `0x601a37`, `0x6020d6`, `0x6020e8`, `0x6020ef`, `0x6020f6`, `0x6020fb`, `0x618750`, `0x80c5bc`, `0x602114`, `0x602130`
Confidence: verified
Proof: one
Test: `objectlayer::monster_move_case1_near_gate_difftest`

`0x601a1d cmp byte [ebp+0xb],1; jne 0x601a37` skips the flags, duration and count reads and stores
0x100, 0 and 1 (`0x601a28`, `0x601a32`). The near test compares the squared distance with 9.0 (`0x80c5bc`):
`fcomp; test ah,1; je 0x601a1d` continues when it is 9.0 or more, and the fall-through calls
`0x618750` and `jmp 0x6020fb`, past `0x6020d6`, `0x6020e8`, `0x6020ef` and `0x6020f6`. The facing payload is a
jump table at `0x602114` over the type minus 1; the packed deltas are `0x602130`.

## Spline flag word

The spline's flag word `+0x18` is the wire's flags, and its bit `0x200` decides five things. It is stored
verbatim by `0x7c6a50` (`0x7c6a63`), from the body's flags, and zeroed when the spline object is allocated
(`0x7c62e4`), so no flag survives a restart. Bit `0x200` set means: the points are explicit vectors
and not packed deltas (`0x601b75`); the speed cap is 10.0 times the speed and not 4.0 (`0x601f46`); the
spline's `+0x54` is 1 (`0x7c6a8f`); the spline's height step survives (`0x616cf5`); and the dispatch
takes the free-vertical resolver (`0x6341b1`). Its only writer is the packet. Bit `0x100` is the run
mode (`collision/walkmode#walk-mode-messages`) and bit `0x4` is the client's own finished mark.

Addresses: `0x7c6a50`, `0x7c6a63`, `0x7c62e4`, `0x601a37`, `0x601b72`, `0x601b75`, `0x601b78`, `0x601f46`, `0x601f49`, `0x601f4f`, `0x7c6a8f`, `0x7c6a94`, `0x616cf5`, `0x616cf8`, `0x6341b1`, `0x6341b4`, `0x619def`, `0x7c6a14`, `0x80306c`, `0x801624`
Confidence: verified
Proof: one
Test: none

The word reaches `0x7c6a50` through `0x601fda` and `0x6187fa`, and the local that holds it is not
modified after the read. The scan for stores to `[reg+0x18]` of a spline with an immediate carrying `0x200`
finds none (the control: it finds the `|4` at `0x619def` and the `0x10000`, `0x20000` and `0x40000`
stores at `0x618849`, `0x618879` and `0x6188a9`). `0x80306c` is 4.0 and `0x801624` is 10.0, picked by
`0x601f46 test ch,2; je 0x601f4f`.

## Move base writers

A mover's transport GUID `+0x38` and `+0x3c` has four writers: the constructor, `0x617170`, `0x618970`
and the teardown `0x7c5950`. `0x617170` is reached from `0x617100` (the monster move, the transports'
rider lists, the collision layer's `0x6343b0`), from `0x6173b0` (the collision layer, for the active
mover with no live spline) and from the teleport apply `0x617e90`. `0x618970` is reached from the status
apply `0x618b30`, whose `0x7c6900` returns the zero pair for a status without a transport block, so such a
status detaches, and from the forced snapshot `0x61a770`. Nothing in the spline completion writes the
base: a unit whose spline ends stays on the transport. `0x6173b0` also refuses to clear the base while
the unit is inside the transport's geometry.

Addresses: `0x7c4899`, `0x7c489c`, `0x617380`, `0x617383`, `0x6189dc`, `0x6189df`, `0x618a4e`, `0x618a51`, `0x7c59e3`, `0x7c59e6`, `0x617170`, `0x617110`, `0x617410`, `0x617ea3`, `0x617100`, `0x618970`, `0x618b43`, `0x61a784`, `0x6173b0`, `0x6173d5`, `0x6173da`, `0x6173dc`, `0x6343b0`, `0x7c6900`, `0x7c690b`, `0x7c5950`, `0x630ba0`
Confidence: verified
Proof: one
Test: none

The scan of stores to `[reg+0x38]` and `[reg+0x3c]` in the unit, movement and resolution code and
the shared movement file lists these eight stores on a `CMovement`; the other hits are the fields of other
classes. Relative-call scans: `0x617170` three callers (`0x617110`, `0x617410`, `0x617ea3`), `0x617100`
four (`0x5f506d`, `0x5f5e8d`, `0x603f60`, `0x634631`), `0x618970` two (`0x618b43`, `0x61a784`); none
has a stored address. `0x6173da test eax,eax; jne 0x61741c` returns 0 on the guard `0x630ba0`.

## Ride frame

A rider's pose is stored in the transport's frame while the move base is set, and every world consumer
converts it on demand. The frame is the transport's position and its yaw only: `0x630ac0` starts from
the identity, translates by the position (vtable `+0x14`) and rotates about z by the angle (vtable
`+0x18`), with no pitch, roll or scale. A point maps as a row vector, `world = local · Rz(facing) +
position`, by `0x7c4930`; a facing adds the transport's and folds by 2π (`0x7c4ae0`). With a zero base
both return their input. The status writer stores both forms, and the collision queries convert at
query time.

Addresses: `0x630ac0`, `0x630ade`, `0x630afa`, `0x630b38`, `0x630b3e`, `0x630b4d`, `0x630b56`, `0x7bdc40`, `0x7bdd60`, `0x7bcc60`, `0x7c4930`, `0x7c4943`, `0x7c4ae0`, `0x7c4aef`, `0x630b70`, `0xcf5d00`, `0x7c6340`, `0x7c6367`, `0x7c6379`, `0x631e70`, `0x631f0c`, `0x631f17`, `0x631f26`, `0x633840`, `0x6338f0`, `0x633902`
Confidence: verified
Proof: one
Test: none

The call sites of `0x630ac0` are eleven relative calls (`0x60cf11`, `0x618ad3`, `0x631f17`, `0x6338f0`,
`0x671bc7`, `0x7c49d7`, `0x7c4aaa`, `0x7c4c29`, `0x7c67a8`, `0x7c6869`, `0x7c68a2`) and no stored address.
Between the identity store and the `ret 8` nothing but the two calls touches the matrix. `0x7c4930` copies
`in` to `out` for a zero base and otherwise builds the matrix and applies it; `0x7c6340` sets bit
`0x2000000` of the status flags with a nonzero base (`0x7c6367`) and clears it otherwise (`0x7c6379`).
The query `0x631e70` skips the matrix on a zero base (`0x631f0c`) and the swept query `0x633840` reads
the same pair at its entry.

## Spline height

A spline step takes one of three vertical paths. The warp gate (`collision/mover#spline-warp-gate`)
stores a step of more than 3 yards, or 60 yards a second, in x and y whole, wire z included, for a
mover with a direction bit and a spline lacking flag 0x200. Otherwise the commit `0x616cb0` zeroes the
step's z unless the spline has flag 0x200 or the mover 0x200800, and the dispatch sends a grounded
spline mover to the walk resolver, which settles z by a down sweep that includes the transport's own
mesh. With flag 0x200 or 0x200800 the free vertical path keeps z. The transport base is in neither test.

Addresses: `0x616af0`, `0x616b16`, `0x616b27`, `0x616cb0`, `0x616cec`, `0x616cf5`, `0x616cfa`, `0x616d03`, `0x6341a8`, `0x6341ad`, `0x6341b1`, `0x6341b6`, `0x6341dc`, `0x6341e3`, `0x6341ff`, `0x63421e`, `0x634640`, `0x635b00`, `0x6367b0`, `0x616f6a`, `0x616d60`
Confidence: verified
Proof: one
Test: none

`0x616b16 test ecx,0x200f` and `0x616b27 test ah,2` select the guard. A predicate that reads `+0x38` or
`+0x3c` is absent from `0x616cb0` and from the dispatch, which is the full list of the three arms'
selectors. The grounded fork's reach to the deck follows from the gather: the mover's class word
0x100111 has 0x100000, which enables the entity leg, so a transport's mesh is part of the floor
(`terrain/entities#entity-gather`). At the end of a frame `0x616f6a` calls `0x616d60`, which hard-writes the
position when the accumulated sample has drifted more than 3 yards.
