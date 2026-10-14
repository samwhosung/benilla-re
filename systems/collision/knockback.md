# collision: knockback

How a server knockback reaches the mover and what the client sends back, end to end: the two
inbound messages, the launch they share, the acknowledgement, what refuses a knockback, and the
fall speed's sign. The launch itself is `collision/eventapply#knockback-launch`; the queue it
rides is `collision/events` and `collision/replay`; the message handlers belong to
`objectlayer/unitmovement`, whose functions this doc names only as call order and arguments.

Roots: none

## Knockback messages

The controller message `0xef` moves the client's own unit; the relayed `0xf1` shows another
unit's launch. `0xef` binds `0x603f90`, which reads a packed GUID and resolves it with type mask 8
(a miss returns 1, no ack); `0x602780` reads a u32 counter and sends `0xef` to `0x602670`, which calls
`0x60e990(1, 0)`, reads four floats (a pair f1, f2, then f3, f4) and calls `0x617a30` on the unit's
`CMovement`. Body: packed GUID, u32, f32 × 4. `0xf1` binds `0x603bb0`, which reads a packed GUID;
`0x601580` parses a full status and takes the arm `0x6017b3`, the only one that forwards the store,
into `0x6026f0`: four floats, then `0x617a60`. Body: packed GUID, status, f32 × 4. `0xf0` has no
inbound handler.

Addresses: `0x603f90`, `0x603fa2`, `0x603fbe`, `0x603fc5`, `0x602780`, `0x6027a2`, `0x6028f9`, `0x602670`, `0x602678`, `0x6026da`, `0x6026f0`, `0x603bb0`, `0x601580`, `0x6015ce`, `0x6017b3`, `0x6017bb`, `0x6017e0`, `0x617a30`, `0x617a60`, `0x6018ac`, `0x601844`, `0x6036eb`, `0x60371b`, `0x642ed0`, `0x468460`
Confidence: verified
Proof: one
Test: none

`0x6036e6` to `0x6036f0` bind `0xef` to `0x603f90` and `0x603716` to `0x603720` bind `0xf1` to
`0x603bb0`, each through `0x5ab650`. `0x602780` reads the counter (`0x6027a2`, `0x418eb0`) for every
opcode of its family before it compares with `0xef` (`0x6027aa`, `je 0x6028f9`). `0x602670` zeroes
f1 and f2 (`0x60268c`, `0x602693`) and reads them with `0x419130` at `0x60269a` and `0x6026a5`, f3
and f4 at `0x6026b0` and `0x6026bb`. `0x601580` parses with `0x47eba0` (`0x6015ce`), subtracts `0xb5`
(`0x6015ed`) and indexes the byte table `0x6018ac`: its bytes for `0xf1` (index 60) and `0xf0` (index 59)
are `0x18` and `0x19`, jump entries `0x6017b3` and `0x6017e0`, and `0x6017e0` is `xor eax,eax; ret`.
`0x6026f0` reads f1 and f2, then f3 and f4 (`0x602711` to `0x602732`, the fourth into the argument slot).

## Knockback acknowledgement

A knockback sends `0xf0` from the drain, after it applies, never from the handler. `0x617a30` has
`0x6177a0` stamp a record of kind `0x1c` (`+0x4d` send byte 1, `+0x48` counter, `+0x38` and `+0x3c` the
pair, `+0x40` f3, `+0x44` f4) and insert it at `CMovement+0x150` in due-time order. The drain's
entry 28 calls `0x6179c0` with `&+0x38`, f3, f4, then `0x60e0a0(now, 0xf0, byte, 0, counter)`. The
body `0x600a30` builds is the raw 8-byte GUID read through the unit's `+8`, the u32 counter and
the status, with no trailing float. The relayed path `0x617a60` enqueues the same record with send
byte 0, so an observer applies a launch and never acknowledges.

Addresses: `0x617a30`, `0x617a4a`, `0x6177a0`, `0x6177bb`, `0x6177c1`, `0x6177c6`, `0x6177cf`, `0x6177d5`, `0x6177e0`, `0x615820`, `0x6165f0`, `0x61623f`, `0x61624d`, `0x616261`, `0x60e0a0`, `0x60e1b8`, `0x60e1c8`, `0x60e235`, `0x600a30`, `0x600860`, `0x7ccc10`, `0x7ccb90`, `0x6009b0`, `0x618f00`
Confidence: verified
Proof: one
Test: none

The drain jumps through `0x616580`; entry 28 is `0x6165f0`, holding `0x61623f`. `0x60e0a0` reaches
`0x60e1b8` for `0xf0` after its stand-state reaction (below); `mov al,[ebp+0x10]; test al,al; je
0x60e300` at `0x60e1c8` drops the send for a zero byte, and `0xf0` takes the `jg 0x60e235` exit
into `0x600a30`. `0x600860` builds only for opcodes `0x7ccc10` accepts (a chain of 14 `cmp`s, from
`0xe9` to `0x2d1`); `0xf0` is the 13th (`0x7ccc70`). The raw GUID is `0x418370` on the two dwords at the pointer
in `unit+8` (`0x600888` to `0x600894`); the counter is written when `0x7ccb90` accepts, and `0xf0` is its 13th
compare (`0x7ccbf0`, a set that lacks `0x2d1`); `0x6009b0` accepts nine opcodes and not `0xf0`.
The relayed enqueue is `0x617a60` through `0x618eb0`, whose deferred arm pushes 0 at `0x618f00`.

## Knockback refusals

Nothing about the unit's state gates a knockback at its handler; the refusals are an unresolved
GUID, the drain's root filter and a zero send byte. `0x618eb0` is a timing gate: it hands
`0x618c30` the time and applies at once when that accepts, else enqueues. On acceptance it drops
the spline (`0x7c6310`) and refreshes the active list. In the drain, a record popped while flag
`0x1000` is set goes to `0x618030(kind)` and kind 28 returns 0, so the record is discarded with
no launch and no ack. A launch applied with send byte 0 sends nothing. Swimming is cleared by the
launch, not tested; no dead, stunned, transport or spline test lies on the `0xef` path.

Addresses: `0x618eb0`, `0x618ec8`, `0x618ecf`, `0x618ed3`, `0x618eda`, `0x7c6310`, `0x618940`, `0x615c30`, `0x615c69`, `0x615c6e`, `0x615c71`, `0x615c79`, `0x615c80`, `0x618030`, `0x618054`, `0x6179c9`, `0x603fc5`
Confidence: verified
Proof: one
Test: none

`0x618eb0`'s one branch is `test eax,eax; je 0x618eeb` on `0x618c30`'s result; its only memory
operands are its arguments. The filter reads the flag word (`0x615c6e`, `test ah,0x10`), pops the
record first (`0x615c69`) and calls `0x618030` on its kind (`0x615c79`); `je 0x616539` continues
the loop. `0x618030` compares with `0x1c` (kinds above return 1) and reads `0x618054`: 1 for kinds
8 to 17 and 20 to 27, 0 for 0 to 7, 18, 19 and 28. The census of the `0xef` extent (`0x603f90`,
`0x602780`, `0x602670`, `0x617a30`, `0x6177a0`, `0x6179c0`, `0x7c61f0`) shows no read of a death,
stun, transport or spline field.

## Fall speed sign

The fall speed `+0xa0` is down-positive and no client code negates it: a rising jump stores a
negative value and a launch stores the received f4 as it comes. `0x7c6230`, the jump entry, calls
`0x7c61f0` with `-7.9555473` on land (`0xc0fe93d8`) and `-9.0967484` while swimming
(`0xc1118c48`), and returns 0 without a call when flag `0x1000` or `0x2000` is set. The status
the client sends copies `+0xa0`, `+0x68`, `+0x6c` and `+0x84` unchanged into the jump tail, in
that order, whenever `0x2000` is in the flags it sends. So the four floats of `0xef` return in
the ack's jump tail as f4, f1, f2, f3.

Addresses: `0x7c6230`, `0x7c625c`, `0x7c6261`, `0x7c6266`, `0x7c626f`, `0x7c627a`, `0x7c61f0`, `0x7c6213`, `0x7c621d`, `0x6179c0`, `0x7c6340`, `0x7c63d4`, `0x7c63e3`, `0x7c63ec`, `0x7c63f2`, `0x7c63f8`, `0x618900`, `0x47ecb0`, `0x47eba0`
Confidence: verified
Proof: one
Test: none

`0x7c625c test ah,0x30; jne 0x7c6288` returns 0 before the constants (`0x7c6266`, `0x7c626f`).
The tail at `0x7c63d4` tests `ah,0x20` of the status flags and copies four dwords with `mov`;
`0x47ecb0` writes `+0x3c`, `+0x40`, `+0x44`, `+0x48` in order and `0x47eba0` reads them in the
same order, so the tail is `+0xa0`, `+0x68`, `+0x6c`, `+0x84`. `0x6179c0`'s direction is world-absolute: it reads no `+0x1c` operand.

## Knockback side effects

The handler `0x602670` first calls `0x60e990(1, 0)`: with its first argument set it cancels a
click-to-move or follow in progress (`0x6103a0`, then `0x60fb60(0, 1)` unless bit 0 of `0xc4da48`
is set), and with the second clear it closes the loot window of an active player. The send
`0x60e0a0` runs, for every opcode it does not exempt (`0xf0` among them), a stand-state reaction
before the send: when the unit's vtable `+0xa4` slot (`0x60be50`, the stand state byte) is
nonzero it calls `0x60be30(0)`; then `0x60d040` and `0x614150` on the spell id at unit `+0xc8c`,
which interrupt a movement-cancelled cast. The launch plays no animation or sound itself:
`0x6179c0` calls `0x7c6e80` and `0x7c61f0`, `0x7c61f0` calls `0x7c5cd0`, which calls `0x7c5a20`.

Addresses: `0x60e990`, `0x6103a0`, `0x60fb60`, `0x60e139`, `0x60e13d`, `0x60e14b`, `0x60e15d`, `0x60e16d`, `0x60e1b3`, `0x60be50`, `0x60be30`, `0x60d040`, `0x614150`, `0x6179c0`, `0x7c6e80`, `0x7c5cd0`, `0x7c5a20`
Confidence: verified
Proof: one
Test: none

`0x60e139` loads the unit's vtable and calls `[edx+0xa4]`, which is `0x60be50` (it loads
`[[unit+0x110]+0x210]` as a byte), then `0x60e150` loads `[esi+0xc8c]` for `0x60d040(spell, 1, 1)`
and `0x614150(spell, 1)`. A second `0x614150` follows when the descriptor dword at `+0x228` names a
spell with bit 8 of its `+0x5c` and the mover has a bit of `0x200f` (`0x60e19f` to `0x60e1b3`).

## Jump emission

The jump message `0xbb` has one live send: the jump arm of the drain, `0x615ed1`, after the
take-off `0x7c6230`; it reaches `0x60e0a0(now, 0xbb, byte, 0, 0)` through `0x6163c5`. The
opcode selector `0x619f00` also returns `0xbb` (`0x619f7d`), but its only caller `0x61a9da` runs
it only when the flags' low four bits change, and the selector's two earlier tests cover exactly
those bits, so that return cannot be reached. The hover grant `0x61a620` and the relayed jump
apply `0x617991` send nothing.

Addresses: `0x615eba`, `0x615ed1`, `0x6163c5`, `0x6163cf`, `0x619f00`, `0x619f0e`, `0x619f36`, `0x619f7d`, `0x61a99d`, `0x61a9da`, `0x61a620`, `0x617991`, `0x7c6230`, `0x619b50`
Confidence: verified
Proof: one
Test: none

Of the nine places the image loads `0xbb` as an immediate, `0x615ed1` is the only push into
`0x60e0a0`; `0x4bf7dc`, `0x6046d6` and `0x604bf3` pass it to ui calls (`0x496720`, `0x515e50`),
`0x602cba` calls the animation selector `0x5fe2f0`, `0x60342b` registers the inbound handler and
`0x604f81` unregisters it. `0x619f00` is called from `0x61a9da` only (relative-call and dword
scans over every section: no other reference; control `0x5ed570` shows its vtable entry), and
`0x61a99b xor eax,ebx; test al,0xf; je` skips the call when the low bits are equal. `0x619f0e`
tests the low two bits and `0x619f36` the next two. `0x7c6230` has four callers: `0x615ebe`,
`0x617991`, `0x61a597` and `0x61a630`. `0x619b50` has no `call`, `jmp` or stored reference.

## Not a knockback apply

`0x6187a0`, the spline path commit, is not reached by a knockback: its two callers are `0x5ee101`
and `0x601fe7`, the latter inside the monster-move parse chain `0x6018f0`, `0x603f00` (opcodes
`0xdd` and `0x2ae`).

Addresses: `0x6187a0`, `0x5ee101`, `0x601fe7`, `0x6018f0`, `0x603f00`, `0x603f69`
Confidence: verified
Proof: one
Test: none

Relative `call`/`jmp` scan over `.text`: exactly the two sites; dword scan of every section: no
stored reference (control `0x5ed570` finds its vtable slot at `0x80b01c`). `0x603f69` is `0x603f00`'s
call into `0x6018f0`. The knockback chain `0x602670`, `0x617a30`, `0x6177a0`, `0x6179c0` contains
no call to `0x6187a0`, `0x453360` or `0x7c6a50`.
