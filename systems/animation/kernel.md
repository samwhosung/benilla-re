# animation: the animate pass

One call of `CM2Model::Animate` poses a model instance for a frame. It reads the instance's bone
states and the arrays of the shared model's header, and writes the bone matrices, the sampled values
of every animated element, the instance's world matrix and the inputs its draw colours come from.
Then it calls itself for each visible attached child. The code is one function of M2Model.cpp, which
`models` owns, so `animation` owns no function row and no object file. This doc holds the pass as a
whole: its gates and exits, what runs before the bone loop and after the element loops, and the
arrays it walks. The bones' clocks are `clock`, their matrices `pose` and `billboards`, the sampling
of a track `tracks`, the element loops `elements`, and the calls that start a sequence `playback`.

Roots: none

## Animate gates

`Animate(parentMatrix, a, b, alpha)` is a thiscall with four stack arguments (`ret 0x10`) and a
0x19c-byte frame. The three after the matrix are the colour, emissive and alpha inputs of
`models/animation#instance-colours`. It returns at once when the instance is not loaded (`+0x10` is
0) or its animated frame `+0x40` equals the scene's frame `[[+0x2c]+0x10]`. Otherwise it runs the
whole pass and stores the scene's frame into `+0x40` at its last step. The two early exits skip that
store. Callers: the scene's draw at three sites, `EnsureAnimated` at three, the child walk of this
function at one, and the model particles of `UpdateAfterAnimate` at one.

Addresses: `0x714260`, `0x714271`, `0x714280`, `0x718775`, `0x71877c`, `0x718780`, `0x707662`, `0x7077b6`, `0x707824`, `0x714069`, `0x714158`, `0x71417e`, `0x71875c`, `0x7191b2`
Confidence: verified
Proof: one
Test: none

Both gates jump to `0x718780`, which pops only `ebx`, since `esi` and `edi` are pushed after them
(`0x714291`). The normal exit `0x718775` loads the scene `+0x10` and stores it at `0x71877c`. A call
census of `e8` targets gives eight sites, listed above; no section stores the address.

## Parent-driven state

With a parent (`+0x1cc`) the pass sets `+0x50`, the effective visible flag, to 1 when the parent's
`+0x50` and this instance's own flag `+0x1d8` are both nonzero, else to 0, and copies the parent's
`+0x17c`, the transport matrix pointer, into `+0x17c`. Without a parent it writes neither. `+0x84`
is the parent's `+0x84` when there is a parent whose `+0x4` has bit 0 clear. Otherwise it is
`(x² + y²) + z²` of the world matrix translation `+0x12c`, `+0x130`, `+0x134`, summed at 53 bits and
rounded to a float once.

Addresses: `0x714260`, `0x71429c`, `0x7142a1`, `0x7142a5`, `0x7142b8`, `0x7142bb`, `0x7142c1`, `0x7143a0`, `0x7143ac`, `0x7143b2`, `0x7143c0`, `0x7143e2`, `0x710c50`
Confidence: verified
Proof: one
Test: none

`0x71429c` skips both writes without a parent. `SetVisible` (`0x710c50`) stores its argument at
`+0x1d8` for an attached instance and at `+0x50` for a top-level one, so the pass derives a child's
`+0x50` from the two. The sum is `fld c; fld b; fld a`, then `a·a`, `b·b`, add, `c·c`, add, one
`fstp dword` at `0x7143e2`. The scene reads `+0x84` for its depth keys (`models/scene#element-depth-keys`).

## World matrix and frame

The pass sets the world matrix `+0xfc` to the base matrix `+0xbc` times the parent matrix, through the
dispatched product `0x74a7c0(out, a, b)`, which computes `out = a · b` for row-major 4×4 matrices. It
then builds two identity matrices on its frame, the working parent matrix at `ebp-0x70` and the bone
rotation matrix at `ebp-0xe4`, reads the bone count from header `+0x34`, and skips the bone loop to the
element loops when it is 0.

Addresses: `0x714389`, `0x71438c`, `0x71438d`, `0x714393`, `0x714399`, `0x71439a`, `0x71439b`, `0x7143ee`, `0x714463`, `0x71451c`, `0x714527`, `0x74a7c0`, `0x754a66`
Confidence: verified
Proof: one
Test: none

The pushes at `0x71438c`, `0x714399` and `0x71439a` leave the destination on top, then `+0xbc`, then
the parent matrix, and the routines behind `0x74a7c0` take `(out, a, b)` in that order: the SSE2 product
`0x754a66` reads `[ebp+8]` as the destination (`Dispatched matrix math` in `models/animation`). The working matrix is rewritten per bone by the parent matrix legs (`pose`).

## Pause slide

While the pause stamp `+0x4c` is nonzero, the pass moves the cursors of every bone's slots forward by
the clock's advance, which holds a sequence's elapsed time constant. Per pass `delta` starts at 0. When
the stamp was nonzero and the scene clock `[+0x2c]+0xc` is nonzero, `delta = clock − stamp` and the stamp
becomes the clock. A bone with a primary sequence then adds `delta` to `+0xa8` and `+0xac` when the stamp
is nonzero, and a bone with a blend sequence adds it to `+0xd4` and `+0xd8`. The fade end `+0x100` is
not moved.

Addresses: `0x710bf0`, `0x710c00`, `0x7143ee`, `0x7143f1`, `0x714503`, `0x714508`, `0x714512`, `0x714519`, `0x714561`, `0x71456b`, `0x714571`, `0x7146c1`, `0x7146cb`, `0x7146d1`
Confidence: verified
Proof: one
Test: none

`SetPaused(on)` (`0x710c00`) stores 0 for off and otherwise the scene clock, or 1 when the clock is 0.
The slide is `[ebp-4]`, zeroed at `0x714503`. The `je` at `0x714506` tests the old stamp and `0x71450e`
the clock, as the `cmp` and `je` show. The adds are gated on the live stamp `[ebx+0x4c]` (`0x714564`),
not on `delta`. On the first pass after a pause the stamp still holds the pause time, so `delta` is the
whole pause so far. The setter and getter are called only from the object layer's game object state code: `0x5f38e2`, `0x5f3b03`, `0x5f3dfe`.

## Arrays the pass walks

The pass walks the shared header's arrays in a fixed order and writes the instance's arrays beside
them. Each loop reads its count and array from the header, steps the record by the stride, and gates a
track on its key count against the first-update gate `+0x8c`.

| loop | header count, array, stride | instance array, stride |
|---|---|---|
| bones | `+0x34`, `+0x38`, 0x6c | `+0x90`, 0x118; matrices `+0x94`, 0x40 |
| colours | `+0x54`, `+0x58`, 0x38 | `+0xa0`, 0x50 |
| transparency | `+0x64`, `+0x68`, 0x1c | `+0xa8`, 0x20 |
| texture frames | `+0x6c`, `+0x70`, 0x1c | `+0xac`, 0x20 |
| texture transforms | `+0x74`, `+0x78`, 0x54 | `+0xb0`, 0x98; matrices `+0xb4`, 0x40 |
| lights | `+0x11c`, `+0x120`, 0xd4 | `+0x200`, 0x170 |
| cameras | `+0x124`, `+0x128`, 0x7c | `+0x3c4`, 0x84 |
| ribbons | `+0x134`, `+0x138`, 0xdc | `+0x3c8`, 0xd0 |
| particles | `+0x13c`, `+0x140`, 0x1f8 | `+0x3d0`, 0x16c |
| attachment bytes | `+0x104`, `+0x108`, 0x30 | `+0x1c8`, 0x20 |

Addresses: `0x714260`, `0x71451c`, `0x715966`, `0x715c87`, `0x715e46`, `0x715f25`, `0x7163bc`, `0x716ad9`, `0x71763e`, `0x717d6a`, `0x7185e3`, `0x717d4d`, `0x717d53`
Confidence: verified
Proof: one
Test: none

The loops run in the table's order, the bone loop first. Each loop's tail adds the two strides
and compares the index with the header count (`0x715c60`, `0x715e1d`, `0x715efe`, `0x716389`,
`0x716aa9`, `0x717611`, `0x717d4d` and `0x717d53` for ribbons, `0x7185ba` and `0x7185cd` for particles,
`0x71864a` and `0x71864d` for attachments). The identities come from the header and instance arrays
that `models/model#instance-arrays` and `models/shared` derive; the bones' and elements' contents are
`pose` and `elements`. The first-update gate is 0 until `UpdateAfterAnimate` first ends and 1 after
(`models/animation#pushing-animated-state`). A track with `count ≤ gate` is not sampled, so a
one-key track is sampled on the first pass only and keeps its state value afterwards.

## Attached children

After the particle loop, an instance with an attachment byte array (`+0x1c8`) first samples each
attachment byte track, then animates every child that is attached. A child is skipped when its
attachment index `+0x1d4` is 0xffff or the byte at `[+0x1c8] + index·0x20 + 0xc` is 0. For the others it
copies the bone matrix of the attachment's bone, adds the attachment's offset `(x, y, z)` to the copy's
translation as `((m20·z + m10·y) + m00·x) + m30` per column, and calls itself on the child with that
matrix and the instance's `+0x1a0`, `+0x1ac` and `+0x19c`.

Addresses: `0x7185e3`, `0x7185ed`, `0x718602`, `0x718608`, `0x718638`, `0x718657`, `0x718668`, `0x71866e`, `0x718684`, `0x7186a1`, `0x7186ba`, `0x7186eb`, `0x718716`, `0x718756`, `0x71875c`, `0x718764`, `0x71ae90`
Confidence: verified
Proof: one
Test: none

The attachment record is header `+0x108` at 0x30 bytes (`0x7186a1`): bone word `+0x4`, offset `+0x8`,
`+0xc`, `+0x10`, byte track `+0x14`, key count `+0x20`. The instance's `+0x1c8` payload is 0x20 per
attachment with the sampled byte at `+0xc`. `rep movsd` of 16 dwords copies the matrix to `ebp-0x19c`.
Each translation component is a three-term sum at 53 bits plus the matrix translation, narrowed at
`0x7186eb`, `0x718716` and `0x718756`. Without `+0x1c8` the pass ends at `0x7185ed`. The child walk follows
`+0x1dc` and then each child's `+0x1e4`.
