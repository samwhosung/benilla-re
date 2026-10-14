# collision: skipped time

When the client reports time its movement simulation did not integrate, what it changes about
itself when it does, and how it treats the same report from another unit. The wrapper and the
heartbeat are `collision/mover#heartbeat-and-skipped-time`; the stamp it moves on the receiving side
is `collision/status#time-skip`.

Roots: none

## Skipped time sites

A free advance of the mover's clock is reported by `0x615ae0(ms)` from four places, each
passing the milliseconds it did not simulate. The per-frame list walk `0x615b10` passes `now − last`
for a mover with bit 0x8000000 of `+0x40` set. The integrator `0x616620` passes `dt − 250` for a tick
over 250 ms and then clamps its own budget to 250. The resolve entry `0x634040` passes the
remaining milliseconds when the swept world query `0x633840` returned 0. The fall integrator
`0x635600` passes its fall time in milliseconds, rounded, when the walk-slide sub-resolver returned
2, which it does exactly when the earliest-contact sweep found no contact.

Addresses: `0x615ae0`, `0x615b10`, `0x615b4d`, `0x615b56`, `0x615b5c`, `0x616620`, `0x616642`, `0x61664b`, `0x616651`, `0x61665f`, `0x616664`, `0x634040`, `0x633840`, `0x634152`, `0x634154`, `0x6342a3`, `0x6342a6`, `0x635600`, `0x63573c`, `0x635742`, `0x6359ae`, `0x6359cf`, `0x6359e7`, `0x6351a0`, `0x635258`, `0x632ba0`
Confidence: verified
Proof: one
Test: none

A relative-call scan of `.text` finds exactly these four callers of `0x615ae0`, and no stored
address. `0x616642 cmp esi,0xfa; jbe` and `0x616651 lea eax,[esi-0xfa]` give the second site's
amount, then `0x616664 mov [ebp-4],0xfa`. `0x634154 je 0x6342a3` follows `0x633840`'s result. The
fall site multiplies by 1000.0 (`0x8022cc`), adds or subtracts 0.5 by the sign (`0x7ffa24`) and
truncates with `0x40a2b0`. The cases through the fall step are bounded by 250 ms because the tick's
budget is; the first two sites are not.

## Skipped time effects

A report sent by the active mover moves three pieces of the client's own state. On a send the
500 ms deadline `mgr+0x130` grows by the skipped milliseconds, so skipped time does not use up
the heartbeat budget. In the resolve-entry and fall cases the milliseconds are first taken off the
substep accumulator `+0x58`, and in the fall case also off the fall clock `+0x78`, so dead
reckoning does not advance through the window. The outgoing movement timestamp is not changed:
it is the movement clock in each case. The message is `0x2ce`, a raw 8-byte GUID of the active
mover and the u32, 12 bytes, built at one place only.

Addresses: `0x615b02`, `0x615bb0`, `0x6166d8`, `0x616743`, `0x6342ab`, `0x6342b3`, `0x6359d4`, `0x6359e1`, `0x6359e4`, `0x600be0`, `0x600bec`, `0x600bfb`, `0x600c07`, `0x600c19`, `0x600c1d`, `0x600c40`, `0x600c51`, `0x600c5c`, `0x418370`, `0x4181f0`, `0x7c6350`, `0x47ecb0`
Confidence: verified
Proof: one
Test: none

`0x615b02 add [esi+0x130],ebx` runs only when `0x600be0` returns nonzero. In the unit, movement and
resolution code the four accesses of `+0x130` are `0x615b02`, `0x615bb0` (arms it), `0x6166d8` (clamps
the substep) and `0x616743` (gates the heartbeat). `0x6342ab` loads `+0x58`, `sub eax,edi` and
`0x6342b3` stores it; `0x6359e1` and `0x6359e4` store `+0x58` and `+0x78` less the milliseconds. The
send requires the unit's GUID words to equal `0xc4da98` and `0xc4da9c` (`0x600bec`, `0x600bfb`) and a
null `+0xa4` or its bit 0x4 set (`0x600c19`, `0x600c1d`); the GUID is stored from the same two
dwords by `0x418370`, two dword stores and a cursor step of 8. `0x7c6350 mov [esi],eax` stores the
caller's timestamp as the status time.

## Free-advance route

Bit 0x8000000 of `+0x40` marks a mover whose attachment is not loaded, so it free-advances instead
of being integrated, and the list walk reports its whole frame. It is set only by `0x7c6500`, called
from two strategy-class slots (`0x5f55a7`, `0x5f6533`) when a link or model predicate fails, and
cleared by `0x7c6510`, called from six places: the same two classes' reconcile passes (`0x5f53f3`,
`0x5f5ffb`, `0x5f6206`) and the movement-source detach (`0x6189e4`, `0x7c5a02`, `0x7c6827`). Whether
a player boarding a transport reaches the setting code is not settled; the movement-source attach
`0x618970` does not call it.

Addresses: `0x7c6500`, `0x7c6510`, `0x5f55a7`, `0x5f6533`, `0x5f53f3`, `0x5f5ffb`, `0x5f6206`, `0x6189e4`, `0x7c5a02`, `0x7c6827`, `0x5f5500`, `0x5f6490`, `0x670950`, `0x614eb0`, `0x6a25b0`, `0x6a4270`, `0x7103d0`, `0x618970`
Confidence: inferred
Proof: none
Test: none

Relative-call scans: `0x7c6500` has the two callers, `0x7c6510` the six. `0x7c6500` is `or
[ecx+0x40],0x8000000` and `0x7c6510` `and [ecx+0x40],0xf7ffffff`. The two set sites are in the
functions `0x5f5500` and `0x5f6490`, reached through vtable slots, where the predicates `0x670950` and
`0x614eb0` call the load-state tests `0x6a4270` (all groups loaded) and `0x7103d0`.

## Skipped time received

The relayed `MSG_MOVE_TIME_SKIPPED` (0x319) is handled; the client's own `0x2ce` has no inbound
handler. `0x603b40` reads a packed GUID and finds the unit with type mask 8 (a miss discards the
rest of the store); it reads a u32 and passes it to `0x601560`, which adds it to `+0xac`, the
unit's last sender stamp, by `0x61ab90`. The status core keeps `+0xac` as a running maximum of the
wire time, so the add moves the baseline of the next status's step.

Addresses: `0x603b40`, `0x603b50`, `0x603b6c`, `0x603b75`, `0x603b84`, `0x603b8f`, `0x601560`, `0x60156d`, `0x61ab90`, `0x6035d5`, `0x6035e0`, `0x618c52`, `0x618c5e`, `0x618ca2`, `0x618cba`, `0x642ed0`, `0x468460`
Confidence: verified
Proof: one
Test: none

`0x6035d5` to `0x6035e0` register `0x603b40` for `0x319` with a null context. No `mov ecx,0x2ce` or
`mov edx,0x2ce` exists in the listing, against the registration of `0x319` found by the same pattern.
`0x603b9c` copies the cursor on a miss. `0x61ab90` is `mov edx,[ecx+0xac]; add edx,lag; mov
[ecx+0xac],edx; ret 4`.
