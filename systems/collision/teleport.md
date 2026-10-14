# collision: teleport arrival

How the client takes a server teleport: a far transfer to another map, which tears the world
down, loads the destination and acknowledges last, and a near teleport inside the map, which
queues a position op, snaps, restarts a fall and then echoes. The queue and the snap are
`collision/eventapply#teleport-apply` and `collision/replay`; the map load is terrain's.

Roots: none

## Far transfer handlers

Four messages carry a far transfer, bound in one block at `0x401728`. `0x3f` (transfer pending)
`0x401900` reads the destination map, cancels interaction and casts, and raises the loading screen,
so it is up before the world arrives. `0x3e` (new world) `0x401b00` reads the map, a position and a
facing into globals, checks the map against the `Map.dbc` rows (`0xc0daa8`), and posts the world-port
executor `0x401bc0` with `sendAck` 1 on the engine's timed queue at no delay; it neither loads
nor acknowledges. `0x40` (transfer aborted) `0x4019a0` dismisses the screen. `0x236` (login verify
world) `0x401de0` stages the same globals and calls the executor directly with `sendAck` 0, so a
login load sends no acknowledgement.

Addresses: `0x401728`, `0x40174c`, `0x40175d`, `0x40176e`, `0x40177f`, `0x401900`, `0x40194c`, `0x401b00`, `0x401b74`, `0x401ba5`, `0x401ba7`, `0x401bae`, `0x4019a0`, `0x401de0`, `0x401e9b`, `0x401ea4`, `0x401bc0`, `0x4200a0`, `0x495920`, `0xc0daa8`, `0xc0daac`
Confidence: verified
Proof: one
Test: none

The registrations pair `mov edx,handler; mov ecx,opcode` before `call 0x5ab650` at the addresses above.
`0x401ba5 push 1; mov edx,0x401bc0; xor ecx,ecx; call 0x4200a0` posts the callback with argument 1;
`0x401e9b xor edx,edx` and `0x401ea4 call 0x401bc0` call it with 0. `0x401de0` does nothing when the map
equals the current one (`0x468580`). The callback runs on a later scheduler pass than the packet
dispatch (`core/events`).

## World-port executor

`0x401bc0(sendAck)` runs as one synchronous block and acknowledges last. It flushes the network
pointers and waits for outstanding reads, frees the pending move events, re-registers the frame
callback `0x616800`, destroys the object manager (so no local player object exists),
creates an empty one, arms the loading progress callbacks, blocks in `0x66fbe0` until every read of
the destination map has finished, stages the arrival transform, and only then sends opcode `0xdc`
with no body. Nothing builds a packet before that. No tick runs between the arrival of the new world and
the acknowledgement: the mover is still because nothing integrates, not because a flag holds it.

Addresses: `0x401bc0`, `0x401bcd`, `0x401bd9`, `0x401bde`, `0x401be3`, `0x401bed`, `0x401bf7`, `0x401c22`, `0x401c38`, `0x401c44`, `0x401c5c`, `0x401c65`, `0x401c73`, `0x401ca0`, `0x401ca7`, `0x401cae`, `0x401cd7`, `0x66fbe0`, `0x6941f0`, `0x69447a`, `0x443d30`, `0x443d72`, `0x443d84`, `0x443ddf`, `0x616910`, `0x467700`, `0x464ff0`, `0x4818f0`
Confidence: verified
Proof: one
Test: none

`0x401cae push 0xdc` builds the message with `0x418190` and `0x401cd7 call 0x5ab630` sends it; the
branch around it is `0x401ca5 cmp edi,esi; je 0x401cfa` on `sendAck`. `0x66fbe0` recomputes the
streaming windows (`0x672730`) and calls `0x6941f0`, which calls the wait-all loop `0x443d30` at
`0x69447a`. That loop calls only the pending test `0x443e20`, the completion drain `0x443e70`, the
progress callback (`0x443ddf`) and `0x429d50`. A scan of the executor body finds no other `0x5ab630`
send.

## Near teleport acknowledgement

The near teleport acknowledgement `0xc7` is queued, applied, then echoed, from the drain and not from
the packet. `0x603f90` reads the GUID and `0x602780` the counter; `0x602fb0` parses a full movement
status with `0x47eba0` and tests the manager's `+0xc8` (`0x468570`). With it clear, as it is in
this build, it cancels follow and click-to-move state (`0x60e990`) and queues tag `0x26` with the
counter, position and facing through `0x618720`. The drain's arm `0x6164db` calls `0x617e90` for
the snap and then `0x60e0a0(now, 0xc7, send, 0, counter)`: the echo is the opcode, the raw
GUID, the counter as received and the client time. No loading screen is raised on this chain.

Addresses: `0x603f90`, `0x602780`, `0x6027a2`, `0x602fb0`, `0x602ffb`, `0x603000`, `0x603005`, `0x603007`, `0x60300e`, `0x603029`, `0x603036`, `0x468570`, `0x618720`, `0x6164db`, `0x6164ff`, `0x616504`, `0x616519`, `0x61651f`, `0x617e90`, `0x60e25a`, `0x4650e7`, `0x464ff0`
Confidence: inferred
Proof: none
Test: none

`0x603007 jne 0x603036` sends a nonzero `+0xc8` straight to the echo, which builds `0xc7`, the
GUID, the counter and the time without applying the position. The only store to `+0xc8` in the manager's
code is the constructor's zero (`0x4650e7`); the other 45 stores to a `+0xc8` operand in the image lie in
other classes' code and were not traced one by one, so a writer through another base is not
excluded. The queued node carries no flags (`0x6164db` reads `+0x4c` for a transport GUID only), the
apply restarts a fall (`collision/fallwire`), and no stop packet is sent by the chain. Heartbeats in the
air follow only if the fall outlives the 500 ms deadline, which the echo itself re-arms.

## Forced position reload

A forced position op that moves the mover more than 30 yards, when the mover is the unit the world
view is anchored to, reloads the destination's terrain synchronously before the frame goes on. After the
snap `0x617e90` calls `0x630bd0` when `d² > 900`, with the mover's GUID; `0x630bd0` compares it with
the world view's GUID (`+0x88`, `+0x8c` of `0x4818f0`) and then calls `0x66fc80`, which recentres the
streaming windows and runs `0x694920`: it clears the loaded areas, updates the map once with the
synchronous-open flag `0x86a2d0` set and waits for every read, then `0x615020` re-places the type-1
objects. No loading screen is raised. Four other ops reach
`0x617e90` by the same node shape and share the reload.

Addresses: `0x617e90`, `0x617ec1`, `0x617ee5`, `0x617eeb`, `0x617ef1`, `0x617efe`, `0x617f00`, `0x617f14`, `0x617f92`, `0x617fa2`, `0x630bd0`, `0x630bd6`, `0x630bdb`, `0x630bf3`, `0x630bf8`, `0x66fc80`, `0x694920`, `0x69493d`, `0x694947`, `0x69494c`, `0x694951`, `0x86a2d0`, `0x615020`, `0x6184f8`, `0x6186a1`, `0x6186ee`, `0x61870c`, `0x80c730`
Confidence: verified
Proof: one
Test: `collision::facing_pos_gate_difftest`

`0x80c730` is 30.0; `fmul` of it by itself gives 900 and `fcompp; test ah,5; jp 0x617f07` falls
through to `mov ebx,1` only for `d² > 900`. `0x617f92 test ebx,ebx; je 0x617fa7` guards the call. The
relative-call scan finds five callers of `0x617e90` (`0x6164ff`, `0x6184f8`, `0x6186a1`, `0x6186ee`,
`0x61870c`) and one of `0x630bd0` (`0x617fa2`). `0x694920` sets `[0x86a2d0]` to 1 at `0x69493d`, calls `0x698390` and the wait-all loop, clears it at
`0x694951` and sets `[0xc62954]`. Below 30 yards the snap is alone.
