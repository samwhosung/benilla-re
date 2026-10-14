# objectlayer: movement emitter

The unit function that every outbound movement message passes through, `0x60e0a0`, what it
sends and when, the reactions it runs first, and the second job it does at its tail: spending a
click-to-move verb the player is owed. The queue that calls it is `collision/replay` and
`collision/eventapply`; the verbs are `objectlayer/interaction`.

Roots: none

## Emitter routing

`0x60e0a0(time, opcode, send, value, counter)` sends the opcode for the unit only when `send` is
nonzero (`0x60e1bb test al,al; je 0x60e300`), but every call, sent or not, runs the tail. Before it
sends, the opcode picks one of three paths. Opcodes `0xb7`, `0xba`, `0xbc` to `0xbe`, `0xc7`, `0xd7` to
`0xda`, `0xe3`, `0xe5`, `0xe7`, `0xe9`, `0xeb`, `0x2ad`, `0x2cf`, `0x2db`, `0x2dd` and `0x2df` go straight to
the send; `0xbf` to `0xc1` and `0xdb` send only while the movement flags hold swimming (0x200000),
else return 0; every other opcode, including `0xf0`, first runs the reactions below. `0xc7`
builds its own message: the raw GUID, the counter and the value.

Addresses: `0x60e0a0`, `0x60e0aa`, `0x60e0b5`, `0x60e0bb`, `0x60e0cd`, `0x60e0d4`, `0x60e0da`, `0x60e0e1`, `0x60e0f1`, `0x60e0f9`, `0x60e105`, `0x60e107`, `0x60e116`, `0x60e11e`, `0x60e137`, `0x60e139`, `0x60e1b8`, `0x60e1bb`, `0x60e1c8`, `0x60e25a`, `0x60e408`, `0x60e3fc`
Confidence: verified
Proof: one
Test: none

The first compare `cmp ebx,0xe5` sends `0xe5` to `0x60e1b8`; opcodes from `0xb7` to `0xe2` index the
byte table `0x60e408` (45 bytes, value 0, 1 or 2) into `0x60e3fc` (`0x60e1b8`, `0x60e0d4`, `0x60e139`);
above `0xe5` the compare chain `0x60e0f1` to `0x60e137` sends `0xe7`, `0xe9`, `0xeb`, `0x2ad`, `0x2cf`,
`0x2db`, `0x2dd` and `0x2df` to `0x60e1b8` and the rest to `0x60e139`. Class 1 is `0x60e0d4 test
[+0x40],0x200000; jne 0x60e1b8`, else `xor eax,eax; ret 0x14`. `0x60e25a push 0xc7` builds the
teleport acknowledgement with `0x418190`, `0x418370`, `0x4181f0` and `0x4182b0`, sends it
(`0x5ab630`), sets bit 0x80 of the unit word `+0xc58` and re-arms the deadline (`0x615b80`).

## Emitter tail

After the send, every call reaches `0x60e300`, which acts only for the active mover (the unit's
GUID equals `0xc4da98` and `0xc4da9c`) and routes by opcode through the byte table `0x60e448`: `0xb7`
and `0xba` go to the drain, `0xe9` and `0xeb` apply the input commands and go to the drain, `0xc7` runs
the teleport leg and never drains, and every other opcode, `0xc9` among them, skips both. The drain
runs only when the movement word has none of the directions, turns, pitches (0xff) or falling
(0x2000) set. Every path ends in `0x60e480(opcode)`, the unit's animation reaction.

Addresses: `0x60e300`, `0x60e305`, `0x60e314`, `0x60e320`, `0x60e326`, `0x60e32f`, `0x60e336`, `0x60e33d`, `0x60e340`, `0x60e347`, `0x60e34c`, `0x60e352`, `0x60e359`, `0x60e361`, `0x60e369`, `0x60e37a`, `0x60e3e5`, `0x60e3e8`, `0x60e438`, `0x60e448`, `0x60e480`, `0x5143e0`, `0x514640`
Confidence: verified
Proof: one
Test: none

`0x60e320 lea eax,[ebx-0xb7]; cmp eax,0x34; ja 0x60e3e5` bounds the table. The 53 bytes of `0x60e448` are 3
except `0xb7` and `0xba` (0, `0x60e34c`), `0xc7` (1, `0x60e37a`) and `0xe9` and `0xeb` (2, `0x60e33d`);
the jump table `0x60e438` holds `0x60e34c`, `0x60e37a`, `0x60e33d`, `0x60e3e5`. `0x60e33d` pushes 1 and the
time, calls `0x5143e0` and `0x514640`, and falls into `0x60e34c`. `0x60e352 test [ecx+0x40],0x20ff; jne
0x60e3e5` guards `0x60e361 call 0x60fa20`.

## Drain sites

A drain happens only when the emitter is called for `0xb7`, `0xba`, `0xe9` or `0xeb` on the active
mover. The queue's stop arms call it for `0xb7` and `0xba` after the stop command returned nonzero
(`0x615d89`, `0x615e0d`), and its acknowledgement arms and the flush for `0xe9` and `0xeb`. The
state broadcaster calls it for `0xb7` with send 0, and the move-control teardown for `0xba` with
send 0, so a call that sends nothing still drains. The stop commands return 0 while falling (they
latch `0x8000` or `0x10000` instead), so a stop made in the air emits and drains nothing. A landing,
`0xc9` through the packet builder, never reaches the emitter.

Addresses: `0x615d89`, `0x615e0d`, `0x6163cf`, `0x616438`, `0x6164a9`, `0x6164b8`, `0x61845e`, `0x6184be`, `0x618018`, `0x61a8f5`, `0x61a8fb`, `0x61aa70`, `0x61ab02`, `0x61ab08`, `0x7c6ba0`, `0x7c6bd6`, `0x7c6d30`, `0x7c6d66`, `0x602d00`, `0x602e05`, `0x602e13`
Confidence: verified
Proof: one
Test: none

The emitter has 34 call sites, all inside `0x615bee` to `0x61ab08`, and no stored address (relative-call
scan). `0x615d72` and `0x615dfa` call the stop commands and `0x615d79`, `0x615e01` skip the emit on 0.
`0x7c6ba0` returns 1 only after `0x7c6c00` on the ground (`0x7c6bd6 test ah,0x20` sends falling to the
latch); `0x7c6d30` likewise for strafing. `0x61a8f5 push 0xb7` pushes send flag 0, as does `0x61ab02
push 0xba` in the teardown, whose outgoing mover is still the active one because `0x6006e0` calls
the teardown before it stores the new GUID. The landing `0x602e05 push 0xc9` goes to `0x600a30`
(`0x602e13`), not to the emitter.

## Pending interactions

A click-to-move to a target that needs a verb leaves the verb in one of four GUID cells, and
the drain `0x60fa20` spends it. The arrival `0x60f940` copies the target `0xc4d980` into the cell its
mode names (5 NPC `0xc4da28`, 6 loot `0xc4da78`, 7 game object `0xc4d950`, 9 spell `0xc4da20`) and
stops the walk; modes 4 and 10 only stop, others only release the commands. The drain handles the
first nonzero cell in the order loot, game object, spell, NPC, hands the resolved target to its verb
(`0x5df2a0`, `0x5f86b0`, `0x5f05e0`, `0x5f0130`; the loot, spell and NPC verbs only for the active
player) and clears the cell whether or not the target resolved. The world-entry setup zeroes all four.

Addresses: `0x60f940`, `0x60f945`, `0x60f954`, `0x60f95b`, `0x60f980`, `0x60f9a3`, `0x60f9c6`, `0x60f9de`, `0x60f9eb`, `0x60f9fc`, `0x60fa20`, `0x60fa46`, `0x60fa53`, `0x60fa61`, `0x60fa97`, `0x60faa4`, `0x60fada`, `0x60fae7`, `0x60faf9`, `0x60fb2f`, `0x60fb3c`, `0x60fb4a`, `0x603979`, `0x6039a5`, `0x6033c0`, `0xc4da78`, `0xc4d950`, `0xc4da20`, `0xc4da28`, `0xc4d980`, `0xc4d888`, `0x60fb60`, `0x5fa6d0`
Confidence: verified
Proof: one
Test: none

`0x60f945 add eax,-4; cmp eax,6; ja 0x60f9eb` indexes `0x60f9fc` (`0x60f9de`, `0x60f9c6`, `0x60f980`,
`0x60f95b`, `0x60f9eb`, `0x60f9a3`, `0x60f9de` for modes 4 to 10). `0x60fa20` resolves with type mask 8
(loot), `0x20` (game object), 1 (spell) and 8 (NPC), and `0x5fa6d0` tests the active player.
Each cell has exactly four references in the image: the arrival's store, the drain's read and
clear, and the world-entry reset (`0x603979` to `0x6039a5`) in `0x6033c0`, whose one caller is on the
enter-world setup path. Cancelling the walk (`0x60fb60`) does not touch the cells. An arrival made in the
air therefore keeps its verb through the landing, until a later ground stop, acknowledgement or
send-0 stop meets a clean movement word.
