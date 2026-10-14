# collision: fall on the wire

What the client sends when a fall begins and while it lasts. A fall begun by a jump command
announces itself; a fall begun any other way (a walk off an edge, an arrival after a near
teleport, a recourse that refuses a steep contact) sends nothing at the start. The fall itself is
`collision/falling`, the flag broadcaster `collision/handlers#state-change-broadcaster`.

Roots: none

## Fall start seeds

`0x7c61f0(v)` stores its argument at `+0xa0` as the fall's initial vertical speed, and its six call
sites seed 0.0 everywhere but the jump and the packet apply. The walk resolver's lost ground
(`0x6367df`), its settle (`0x637124`), the restored fall lookahead (`0x635ff7`) and the fall gate
`0x7c61c0` (`0x7c61e4`, which serves the near-teleport arrival and the active-mover switch) push 0.
The jump `0x7c6230` pushes -7.955547 (-9.096748 swimming). The packet apply `0x6179c0` pushes the
fourth float of the message. The jump returns 0 without a fall while hovering when its argument is nonzero (`0x7c6236`), on a live spline, or
with root or falling set. The fall gate `0x7c61c0` has no hover test.

Addresses: `0x7c61f0`, `0x7c61c0`, `0x7c61e4`, `0x7c6230`, `0x7c625c`, `0x7c627a`, `0x7c6236`, `0x6367df`, `0x637124`, `0x635ff7`, `0x6179dd`, `0x6179c0`
Confidence: verified
Proof: one
Test: none

A relative-call scan finds these six callers of `0x7c61f0`, all `call`, no stored address. Each of
the first four pushes with `6a 00`; `0x7c627a` follows the two constants of
`collision/knockback#fall-speed-sign`; `0x6179dd` pushes `[ebp+0x10]`. The jump refusal is `0x7c625c
test ah,0x30` (root 0x1000 or falling 0x2000), plus the hover and spline tests before it.

## Fall start silence

A fall that no jump command started puts nothing on the wire at its start. The state-change broadcaster elects a send from `(flags ^ old) & 0xf` and sends nothing when only the falling bit
changed, since 0x2000 lies in `ah`. One leg, `0x61a954` to `0x61a996`, sends `0xee` whenever its fifth argument is set, for the active mover with no spline or a finished one; `0x634260`
sets it when the transport GUID changed during the step (`0x63423d`). The only send of `0xbb` is the jump arm of the drain after a taken-off jump.

Addresses: `0x61a954`, `0x61a996`, `0x63423d`, `0x61a99b`, `0x61a99d`, `0x61a99f`, `0x61a9eb`, `0x615ed1`, `0x615e17`, `0x615e19`, `0x615e2d`, `0x615e32`, `0x617ac0`, `0x617ad3`, `0x617ad5`, `0x617aed`, `0x5f5059`, `0x5f5e79`, `0x61944f`, `0x61a820`, `0x619f00`, `0x6164d1`, `0x619b11`, `0x619bd6`, `0x634260`
Confidence: verified
Proof: one
Test: none

`0x61a99d test al,0xf; je 0x61a9eb` is the nibble test. The begin-fall command of the queue, tag 6, which would
send a heartbeat `0xee`, is queued by `0x617ac0` for three callers only: two transport-leaving
ticks (`0x5f5059`, `0x5f5e79`) and the apply of a received status (`0x61944f`). A walk-off and an
arrival call `0x7c61f0` directly and never reach it.

Relative-call scans: `0x617ac0` has the three callers; `0x61a820` has four (`0x6164d1`, `0x619b11`,
`0x619bd6`, `0x634260`). Tag 6 is the entry at `0x615e17` of the drain's table: `0x7c61c0`, then a
heartbeat through `0x600a30` only when the node's send byte (`+0x4d`, tested at `0x615e32`) is set.
`0x617ac0` pushes tag 6 and send byte 1 (`0x617ad5`, `0x617ad3`) before `0x617570`. The jump arm
`0x619f7d` of the opcode selector is unreached (`collision/knockback#jump-emission`).

## Airborne sends

While a mover falls the client sends a heartbeat `0xee` once the 500 ms deadline passes, measured
from its last movement packet of any kind, and `0xc9` at the landing; the falling bit alone keeps
both gates open. In the per-mover tick, the integrate gate `0x6166f5` tests `0x20ff` and the
heartbeat gate `0x616725` tests `0x200f`, both of which contain 0x2000; the heartbeat also needs the
active mover and a null or finished spline, and `now − deadline` not negative. Every sent packet
re-arms the deadline to its time plus 500 (`0x615b80`, called by `0x600a30`), so a facing change
or any start or stop in the air postpones the next heartbeat.

Addresses: `0x616620`, `0x6166f5`, `0x6166fc`, `0x616725`, `0x61672c`, `0x61673e`, `0x616743`, `0x616749`, `0x61674f`, `0x616755`, `0x600a30`, `0x600ad6`, `0x615b80`, `0x615ba5`, `0x615bb0`, `0x602d00`, `0x602e05`, `0x602e13`, `0x61a947`
Confidence: inferred
Proof: none
Test: none

The gates are quoted from `0x6166f5` to `0x616755` above. `0x615ba5 lea esi,[eax+0x1f4]` and the store
at `0x615bb0` arm the deadline for the active mover, and `0x600ad6` calls it after each send. That the
falling bit is the only input of those gates for a walk-off is read from the code; no step reads
whether a jump started the fall. The landing packet `0xc9` is pushed at `0x602e05` in the landing hook `0x602d00`, which
`0x61a947` calls when the falling bit clears, and goes to `0x600a30`.
