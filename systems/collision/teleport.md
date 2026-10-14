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

The registrations pair `mov edx,handler; mov ecx,opcode` before `call 0x5ab650` at the addresses above. `0x401900` calls `0x495920` (interaction cancel and cast cancel with reason 0x1c)
and then the loading screen, `0x4072c0` for the form with a transport and `0x406640` for the plain one; the two extra numbers of the transport form are read and not used here.
`0x401ba5 push 1; mov edx,0x401bc0; xor ecx,ecx; call 0x4200a0` posts the callback with argument 1;
`0x401e9b xor edx,edx` and `0x401ea4 call 0x401bc0` call it with 0. `0x401de0` does nothing when the map
equals the current one (`0x468580`). The callback runs on a later scheduler pass than the packet
dispatch (`core/events`).

## Transfer abort

`0x4019a0` handles `0x40`. It reads a reason byte and, for reason 1, 2, 3 or 5, shows the global string `TRANSFER_ABORT_MAX_PLAYERS`, `TRANSFER_ABORT_NOT_FOUND`,
`TRANSFER_ABORT_TOO_MANY_INSTANCES` or `TRANSFER_ABORT_ZONE_IN_COMBAT` in the chat frames as a message of type 10. Reason 4 and any other value show nothing. It then writes
`World transfer aborted...` to the console and ends the loading screen (`0x407e80`).

Addresses: `0x4019a0`, `0x4019aa`, `0x4019b4`, `0x4019b9`, `0x4019c0`, `0x4019c7`, `0x4019ce`, `0x4019d5`, `0x4019f3`, `0x4019f8`, `0x4019ff`, `0x401a04`, `0x401a11`, `0x703bf0`, `0x49a870`, `0x407e80`
Confidence: verified
Proof: one
Test: none

`0x4019b3 dec eax; cmp eax,4; ja 0x401a04` leaves a reason of 0 or above 5 at the console line. The jump table behind the function holds `0x4019c0`, `0x4019c7`, `0x4019ce`, `0x401a04` and `0x4019d5`, each of the first
three and the last loading `ecx` with a string name. `0x4019f3 call 0x703bf0` turns the name into text and `0x4019f8 mov edx,0xa; call 0x49a870` adds it with type 10.

## Neighbouring handlers

The block that binds the transfer handlers binds two more messages, unrelated to transfers. `0x1cb` goes to `0x401800`, which reads a string, shows it as an error message and writes
it to the console. `0x1cd` goes to `0x401850`, which reads the total time played and the time played at this level, writes `Time played:` and the two formatted times to the console and signals
event 0x103 with both numbers.

Addresses: `0x401728`, `0x40172a`, `0x401739`, `0x401800`, `0x401818`, `0x401828`, `0x40183b`, `0x401850`, `0x401863`, `0x40186e`, `0x401878`, `0x40188f`, `0x4018df`, `0x4945b0`, `0x63cd00`, `0x63cd50`, `0x49e9d0`, `0x703f50`
Confidence: verified
Proof: one
Test: none

`0x401818 call 0x4191b0` reads a string of at most 0x400 bytes and `0x401828 call 0x4945b0` adds it as an error message; `0x40183b call 0x63cd00` writes it with the format `%s`. In `0x401850`,
`0x401863` and `0x40186e` read the two numbers, `0x40188f` and `0x4018b7` format them as times for the lines `Total: %s` and `Level: %s`, and `0x4018df call 0x703f50` signals the event (`push 0x103`) with the format `%d%d`.

## World-port executor

`0x401bc0(sendAck)` runs as one synchronous block and acknowledges last. It marks the connection out of the world and waits for outstanding reads, destroys the movement manager and the object
manager (so no local player object exists), creates empty ones, arms the loading progress callbacks, blocks in `0x66fbe0` until every read of
the destination map has finished, stages the arrival transform, and only then sends opcode `0xdc`
with no body. Nothing builds a packet before that. No tick runs between the arrival of the new world and
the acknowledgement: the mover is still because nothing integrates, not because a flag holds it.

Addresses: `0x401bc0`, `0x401bcd`, `0x401bd4`, `0x401bd9`, `0x401bde`, `0x401be3`, `0x401bed`, `0x401bf7`, `0x401c22`, `0x401c38`, `0x401c44`, `0x401c5c`, `0x401c65`, `0x401c73`, `0x401ca0`, `0x401ca7`, `0x401cae`, `0x401cd7`, `0x66fbe0`, `0x6941f0`, `0x69447a`, `0x443d30`, `0x443d72`, `0x443d84`, `0x443ddf`, `0x616910`, `0x467700`, `0x464ff0`, `0x4818f0`
Confidence: verified
Proof: one
Test: none

`0x401cae push 0xdc` builds the message with `0x418190` and `0x401cd7 call 0x5ab630` sends it; the
branch around it is `0x401ca5 cmp edi,esi; je 0x401cfa` on `sendAck`. `0x66fbe0` recomputes the
streaming windows (`0x672730`) and calls `0x6941f0`, which calls the wait-all loop `0x443d30` at
`0x69447a`. That loop calls only the pending test `0x443e20`, the completion drain `0x443e70`, the
progress callback (`0x443ddf`) and `0x429d50`. A scan of the executor body finds no other `0x5ab630`
send.

## World-port teardown order

Before the load, the executor discards the old world in a fixed order. It marks the connection out of the world (`0x5aae70` with 0), waits for outstanding reads (`0x443d30`), destroys the movement
manager (`0x616910`) and unregisters its frame handler for event 5 (`0x41fd80`), frees the missiles (`0x61e8f0`), destroys the object manager (`0x467700`), unloads the map (`0x66fc40`), resets the weather
when `0xc6326c` is set (`0x67bac0`), sets the ambience (`0x4586b0`), clears the item and spell cooldowns (`0x6e9ad0`), creates the new object manager (`0x464ff0`), initializes movement (`0x401a40`) and
signals CANCEL_LOOT_ROLL (`0x61b290`). Then it arms the progress callbacks, loads, and disarms them with null arguments (`0x401c65`, `0x401c6e`).

Addresses: `0x401bc0`, `0x401bd4`, `0x401bd9`, `0x401bde`, `0x401bed`, `0x401bf2`, `0x401bf7`, `0x401bfc`, `0x401c01`, `0x401c0b`, `0x401c12`, `0x401c17`, `0x401c22`, `0x401c27`, `0x401c2c`, `0x401c38`, `0x401c44`, `0x401c5c`, `0x401c65`, `0x401c6e`, `0xc6326c`, `0x5aae70`, `0x443d30`, `0x616910`, `0x41fd80`, `0x61e8f0`, `0x467700`, `0x66fc40`, `0x67bac0`, `0x4586b0`, `0x6e9ad0`, `0x464ff0`, `0x401a40`, `0x61b290`, `0x443620`, `0x66fc70`
Confidence: verified
Proof: one
Test: none

The order is the call order of the straight-line body from `0x401bc0` to `0x401c73`. `0x401bd2 mov ecx,eax` takes the connection from `0x5ab490` and `0x5aae70` stores the 0 pushed at `0x401bca`.
`0x401c01 mov ecx,[0xc6326c]; cmp ecx,esi; je 0x401c10` guards the weather reset, and `0x401c12` calls `0x4586b0` with 0. The two arming calls pass the callbacks `0x407f70` and `0x407fa0`
(`0x401c33`, `0x401c3f`). No call in this stretch builds a packet.

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
other classes' code and are not traced one by one, so a writer through another base is not
excluded. The queued node carries no flags (`0x6164db` reads `+0x4c` for a transport GUID only), the
apply restarts a fall (`collision/fallwire`), and no stop packet is sent by the chain. Heartbeats in the
air follow only if the fall outlives the 500 ms deadline, which the echo itself re-arms.

## Forced position reload

A forced position op that moves the mover more than 30 yards, when the mover is the unit the world view is anchored to, reloads the destination's terrain synchronously before the frame
goes on. After the snap `0x617e90` calls `0x630bd0` when `d² > 900`, with the mover's GUID; `0x630bd0` compares it with the world view's GUID (`+0x88`, `+0x8c` of `0x4818f0`) and calls `0x66fc80`,
which recentres the streaming windows and runs `0x694920`. That frees the map objects, footprints and texture caches, updates the map once through `0x698390` with the synchronous-open flag
`0x86a2d0` set and waits for every read. `0x615020` then calls `0x613ef0(object, 1)` on every object. No loading screen is raised.

Addresses: `0x617e90`, `0x617ec1`, `0x617ee5`, `0x617eeb`, `0x617ef1`, `0x617efe`, `0x617f00`, `0x617f14`, `0x617f92`, `0x617fa2`, `0x630bd0`, `0x630bd6`, `0x630bdb`, `0x630bf3`, `0x630bf8`, `0x66fc80`, `0x694920`, `0x69493d`, `0x694947`, `0x69494c`, `0x694951`, `0x86a2d0`, `0x615020`, `0x615030`, `0x613ef0`, `0x6a3160`, `0x699d50`, `0x449010`, `0x698390`, `0x6184f8`, `0x6186a1`, `0x6186ee`, `0x61870c`, `0x80c730`
Confidence: verified
Proof: one
Test: `collision::facing_pos_gate_difftest`

Four other ops reach `0x617e90` by the same node shape and share the reload. `0x80c730` is 30.0; `fmul` of it by itself gives 900 and `fcompp; test ah,5; jp 0x617f07` falls
through to `mov ebx,1` only for `d² > 900`. `0x617f92 test ebx,ebx; je 0x617fa7` guards the call. The
relative-call scan finds five callers of `0x617e90` (`0x6164ff`, `0x6184f8`, `0x6186a1`, `0x6186ee`,
`0x61870c`) and one of `0x630bd0` (`0x617fa2`). `0x694920` sets `[0x86a2d0]` to 1 at `0x69493d`, calls `0x698390` and the wait-all loop, clears it at
`0x694951` and sets `[0xc62954]`. Below 30 yards the snap is alone.
