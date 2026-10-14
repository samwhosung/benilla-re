# playername: weapon swing trails

The trail behind a swinging weapon. The unit code attaches a trail object to a model with a colour
and a lifetime; from then on the model's draw callback adds the weapon's two attachment points
each frame to a ring and draws a fading triangle strip through them.

Roots: `0x6c6920`, `0x6c6970`, `0x6c69c0`, `0x6c68f0`

## Trail objects

A trail object is 8 bytes: the swing record pointer at `+0` and the model at `+4`. A swing record
(0x618 bytes, tag `SWING`) holds the ring write counter at `+0x00`, the read counter at `+0x04`,
128 points of 12 bytes from `+0x08`, the colour dword at `+0x608`, the lifetime in
milliseconds at `+0x60c`, the start time at `+0x610` and the time of the last step at `+0x614`.

Addresses: `0x6c6920`, `0x6c6970`, `0x6c6710`, `0x6c69c0`, `0x6c6750`, `0x6c68f0`, `0x6c6773`, `0x6c67a3`, `0xce86d4`, `0xce86d8`, `0xce86ec`
Confidence: verified
Proof: one
Test: none

`0x6c6920` counts up `0xce86d4`, takes an object from `0xce86d8`, stores the model at `+4` and
adds a reference (`0x710390`). A failed allocation skips the zeroing (`0x6c6946`) but not the store, so
`0x6c6957` writes through a null pointer, a client fault; `0x6c6970` counts down, runs `0x6c6710` and frees the object.
`0x6c6710` clears the model's draw callback (`0x710db0` with a null callback), releases the
model (`0x7103a0`) and frees the swing record. `0x6c69c0` forwards to `0x6c6750` only when both the colour pointer and the lifetime are
non-null. `0x6c6750` reuses the object's swing record, zeroing both counters, or takes a new one
(`0x6c6773`), zeroes its points, counters and colour and installs the callback `0x6c67f0` on
the model (`0x6c67a3`; a reused record keeps its callback), then copies the colour dword and the lifetime, and stamps the clock into
`+0x610` and `+0x614`. `0x6c68f0` frees the blocks of both pools.

## Trail callback

Each time the model draws, the callback reads the positions of the model events `$WTB` and `$WTT`, sets the render state and steps the trail. The trail ends when the step returns 0:
the swing record is freed, the object's pointer cleared and the callback removed.

Addresses: `0x6c67f0`, `0x6c67f7`, `0x6c680b`, `0x6c6882`, `0x6c6893`, `0x6c68a2`, `0x6c68ab`, `0x6c68bc`, `0x6c6560`
Confidence: verified
Proof: one
Test: none

`0x6c67f0` calls `0x7130e0` for the event ids `0x42545724` and `0x54545724` (the bytes `$WTB` and
`$WTT`) and returns unless the model has both. It sets six device render states through `0x589e60`, reads the
clock, fetches the two event positions with `0x7131b0` and calls `0x6c6560` with the clock and
the two positions. The step is `0x6c6560(now, p2, p1)` with `p1` the `$WTB` position and `p2` the `$WTT` position; it stores `p1` first.

## Trail step

The step adds the pair of points while the trail is younger than its lifetime, then draws a strip
whose point count and fade follow the alpha byte at `+0x60b`.

Addresses: `0x6c6560`, `0x6c6580`, `0x6c6587`, `0x6c65a1`, `0x6c65c6`, `0x6c65d9`, `0x6c65df`, `0x6c65ea`, `0x6c65f4`, `0x6c6601`, `0x6c6606`, `0x6c6613`, `0x6c6619`, `0x6c6693`, `0x6c669a`, `0x6c66ae`, `0x6c66f2`, `0x6c64a0`, `0x6c6540`, `0x6c64d0`, `0x811278`
Confidence: verified
Proof: one
Test: `playername::pn_swing_step_count`

`0x6c6560` computes `age = now − [+0x610]`. While `age < [+0x60c]` (unsigned, `0x6c6583`) it
appends the `$WTB` point then the `$WTT` point through `0x6c64a0`, which returns slot
`head & 0x7f`, increments the head and raises the tail to `head − 128`. With `dt = now − [+0x614]`
(then stored) it takes `step = ftol(dt · (1/300) · alpha)`, `alpha` the byte at `+0x60b`, `1/300`
the float `0x3b5a740e`, low byte, at least 1. `q = alpha / step` (signed `idiv`) must exceed 1
or the step returns 0. The strip holds at most the newest `2q` points (`0x6c6540`; a count of 0
returns 0), in 16-byte vertices (`0x58a140`) coloured `[+0x608]`, or with the device flag
`[+0x1c] == 1` of `0x58a230` the bytes `+0x60b`, `+0x608`, `+0x609`, `+0x60a` from the top.
`0x6c64d0` lowers the colour's alpha by `step` at each even point index before writing that
vertex. The strip is primitive type 4 (`0x58a830`). When `age > lifetime/2` the alpha at `+0x60b`
drops by `step`. The step returns 1. The test runs `0x6c65c6` to `0x6c6606`: the step and
the quotient.

## Ring read slot

The reader of the point ring indexes it differently from the writer.

Addresses: `0x6c64a0`, `0x6c64d0`, `0x6c64e5`, `0x6c64ea`
Confidence: verified
Proof: one
Test: none

`0x6c64a0` writes point `i` at slot `i & 0x7f`, 128 slots. `0x6c64d0` reads point `i` from slot
`i mod 127` (`mov ecx,0x7f; idiv ecx`, `0x6c64e5` and `0x6c64ea`). The two agree for `i` below
127 only: from `i = 127` on, the strip's vertex is read from a slot that holds a different
point. A trail with more than 127 points stored since its start draws those vertices from the
wrong slots.
