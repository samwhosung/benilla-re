# collision: self-addressed moves

What the client does with a relayed movement message that names the unit it is moving itself. It
applies it exactly as it applies one for any other unit: nothing on the inbound path compares the
unit with the active mover. The handlers are `objectlayer/unitmovement#movement-message-handlers`;
the status core is `collision/status#status-merge-and-apply`.

Roots: none

## Self-addressed move

A relayed movement message for the active mover is applied. The dispatcher `0x537aa0` calls the
handler registered for the opcode and drops an opcode with no slot. `0x603bb0` reads a packed
GUID, finds the unit with type mask 8 and calls `0x601580` on it; a miss rewinds the store and
returns 0. For `0xc9` and `0xee` the byte table `0x6018ac` gives case 16, `0x6017a5`, which calls
`0x602c20`; that calls `0x61a750` on the unit's `CMovement`, tag `0x25`, and `0x618f20` applies the
status through `0x618c30` or queues it. The test is whether the GUID names a known unit, never
whether it is the client's own.

Addresses: `0x537aa0`, `0x537acf`, `0x537ad5`, `0x603bb0`, `0x603bc2`, `0x603bde`, `0x603be5`, `0x603bef`, `0x601580`, `0x6015fc`, `0x601603`, `0x6017a5`, `0x6017aa`, `0x602c20`, `0x602c31`, `0x61a750`, `0x61a75b`, `0x618f20`, `0x618f38`, `0x618c30`, `0xc4da98`, `0xc4da9c`
Confidence: verified
Proof: one
Test: none

Of the 158 instructions that name `0xc4da98` or `0xc4da9c` in the image, in 62 functions, none lies in
`0x603bb0`, `0x601580`, `0x602c20`, `0x61a750`, `0x618f20`, `0x618c30`, `0x7c6420`, `0x7c6490`, `0x47eba0` or
`0x6176a0` (the literals are absolute operands, so the scan is complete). The control is the
functions that do compare: `0x600be0`, `0x6006e0`, `0x615b80`, `0x6010e0` and `0x619de0` are in it.
The table byte of `0xc9` and `0xee` is 16. The registrations bind 30
opcodes to `0x603bb0` (`0xb5` to `0xc3`, `0xc5`, `0xc9`, `0xca`, `0xcb`, `0xd9` to `0xdb`, `0xec` to `0xee`,
`0xf1`, `0xf7`, `0x2ad`, `0x2b0`, `0x2b1`), 16 to `0x603f90`, 6 to `0x603ae0`.

## Self move effects

The applied status snaps the mover and nothing is acknowledged. The flags merge as
`old ^ ((old ^ wire) & 0x75a07dff)`, taking the wire's falling and direction bits. Position, facing
and pitch go to both the live cells and the integrator base (`+0x44` to `+0x4c`, `+0x50`, `+0x54`), so the
local integrator continues from the server's point, and a falling status seeds the arc through
`0x7c6490`. No packet is built on this path except `0x2c9`, which `0x619de0` sends, gated on the
mover GUID, when a live spline whose bit 0x4 is clear becomes idle. The 500 ms deadline is written
only by `0x615b80` and `0x615ae0`, which this path does not reach, so the next heartbeat reports the
server's pose back.

Addresses: `0x618dfc`, `0x618df3`, `0x7c6420`, `0x7c6490`, `0x618e0f`, `0x618e3c`, `0x602d00`, `0x616de0`, `0x616e77`, `0x616e83`, `0x616e92`, `0x618940`, `0x61895e`, `0x619e80`, `0x619eb3`, `0x619de0`, `0x619dff`, `0x619e0a`, `0x619e69`, `0x600b10`, `0x600b24`, `0x600b9a`, `0x600ba6`, `0x615b80`, `0x615ae0`
Confidence: verified
Proof: one
Test: none

The merge mask is `0x618deb and edx,0x75a07dff` for the status path (`collision/status`). The tail
`0x7c6490` runs when the merged flags hold 0x2000 (`0x618e0f test ah,0x20`); when they clear it,
`0x618e3c` calls the landing reaction `0x602d00`, which has no GUID test. `0x616e77`, `0x616e83` and
`0x616e92` add the step to `+0x44`, `+0x48`, `+0x4c`. `0x619e80` unlinks at once unless the mover has a
live spline, and `0x619de0` compares the GUID (`0x619dff`, `0x619e0a`) before `0x600b10` pushes `0x2c9`
(`0x600b24`) and sends (`0x600b9a`). The writers of `mgr+0x130` are the stores at `0x615bb0`
and `0x615b02` (`collision/timeskip`).

## Speed relay fork

The six speed relays are the one family with a self test. `0x6010e0` reads the status and then the
new speed for every unit, and compares the unit's GUID with `0xc4da98` and `0xc4da9c`
(`0x60114a`, `0x601159`). The client's own unit calls the speed-only queue setter with the time and
the speed and ignores the parsed status; any other unit passes the status to the validating
variant. Both branches apply the speed. The other relays, and the forced-change family `0x603f90`,
apply with no self test, and an opcode with no slot is dropped by the dispatcher.

Addresses: `0x6010e0`, `0x60112e`, `0x601140`, `0x60114a`, `0x601159`, `0x601188`, `0x601259`, `0x601176`, `0x601243`, `0x619530`, `0x6195c0`, `0x619650`, `0x6196e0`, `0x619770`, `0x619800`, `0x619550`, `0x6195e0`, `0x603ae0`, `0x603f90`
Confidence: verified
Proof: one
Test: none

`0x60112e call 0x47eba0` parses into `[ebp-0x50]` before the branch and `0x601140 call 0x419130`
reads the speed float. The self arm (`0x601188 call 0x619530`, the run case) pushes the speed and the
time; the other arm (`0x601259 call 0x619550`) passes the time, the status address and the speed. The registrations are read from the `mov ecx,opcode; mov edx,handler;
call 0x5ab650` tuples between `0x6033c0` and the end of the block.
