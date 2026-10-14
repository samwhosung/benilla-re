# objectlayer: create pose

What a unit that streams in already moving shows on its first drawn frame: the create block's
movement status is applied to the mover before the unit has a model, so the swim pitch and the
direction flags are in place when the model is first posed. The apply is `collision/status`; the
reader of the block is `objectlayer/movement`; this doc holds the order between them and the pose
code that reads the result.

Roots: none

## Create block apply

A create block seeds the mover's pitch as well as its flags and speeds. The create handler
`0x4660a0` reads the movement block only when the block's update flags have LIVING (`0x20`), builds
the unit through `0x466a20`, whose post-create `0x5fad50` calls `0x5ff070` with its self guard 0.
`0x619320` zeroes the flags and merges the block's through `0x618c30` with force 1, which skips the
timing gate and the queue probe, so the path is straight to the commit `0x7c6420`. After the
apply the flags are `0x80000000 | (wire & 0x75a07dff)`, and the status pitch at block `+0x34`
(read only when the flags have 0x200000, else zero) lands in `CMovement+0x20` and `+0x54`.

Addresses: `0x4660a0`, `0x46618d`, `0x466199`, `0x46629f`, `0x466a20`, `0x466a63`, `0x5fad50`, `0x5fb131`, `0x5ff070`, `0x5ff09c`, `0x5ff044`, `0x5ff04a`, `0x619320`, `0x61936e`, `0x619414`, `0x61941f`, `0x618c30`, `0x618c4a`, `0x618c59`, `0x618c6a`, `0x618cd9`, `0x618cde`, `0x618d67`, `0x618dac`, `0x618db1`, `0x618de7`, `0x618df3`, `0x618dfc`, `0x7c6420`, `0x7c6454`, `0x7c645f`, `0x7c6462`, `0x7c6465`, `0x47ec34`, `0x47ec4a`
Confidence: verified
Proof: one
Test: none

`0x619320` stores the six speeds, `+0x58` = 0 and `+0x40` = 0 (`0x61936e`) and contains no x87
instruction. `0x619414 push 1` is the fifth argument of `0x618c30`, so `0x618cde jne 0x618d67`
always jumps and the `0x75a01dff` arm (`0x618db1 jne 0x618de7`) is not reached. `0x618c4d`
to `0x618c6a` sets bit 31 on the zeroed word (`js` is not taken), and bit 31 lies outside the
mask. `0x7c6454` to `0x7c6465`: `fld [block+0x30]; fst [+0x1c]; fstp [+0x50]`, then `fld [block+0x34];
fst [+0x20]; fstp [+0x54]`. `0x47ec34 test [esi+0x10],0x200000` gates the read and `0x47ec4a`
stores a zero otherwise.

## Pitch writers

The pitch cell `+0x20` of a `CMovement` has seven writers and its base `+0x54` five. `+0x20` is
zeroed by the constructor `0x7c4850`, clamped to plus or minus π/2 by the pitch integrator
`0x7c4f80`, set to `asin(dir.z)` by the spline follower `0x7c5490` while swimming, copied from a
status by `0x7c6420`, set with a deadband by `0x7c6f70`, and zeroed by the swim stop `0x7c6e80`
and the position snap `0x7c69a0`, which the near teleport uses. The base `+0x54` is copied from
`+0x20` by the anchor update `0x7c5cd0` and from a status by `0x7c6420`, zeroed by the
constructor and the swim stop, and restored by `0x635f10`.

Addresses: `0x7c4850`, `0x7c4886`, `0x7c48bd`, `0x7c4f80`, `0x7c4f9e`, `0x7c4fb9`, `0x7c4fc1`, `0x7c5490`, `0x7c55ee`, `0x7c6420`, `0x7c6462`, `0x7c6465`, `0x7c6f70`, `0x7c6f89`, `0x7c6f91`, `0x7c6e80`, `0x7c6e8c`, `0x7c6e93`, `0x7c69a0`, `0x7c69cc`, `0x7c5cd0`, `0x7c5cfb`, `0x635f10`, `0x635f36`
Confidence: verified
Proof: one
Test: none

The sites are the stores to `[reg+0x20]` and `[reg+0x54]` on a `CMovement` in the unit, movement and
shared movement code (a scan of the listing; the other hits are other classes' own fields). `0x7c4f9e`
and `0x7c4fc1` store `0xbfc90fdb` and `0x3fc90fdb`, minus and plus π/2. `0x7c6f89` is `fcomp 2^-20;
test ah,5; jnp`, so `0x7c6f91` stores when `|new − old|` is at least 2^-20. `0x7c5cfb` copies `+0x20`
(loaded at `0x7c5ce9`) into `+0x54`; `0x7c6e8c` zeroes `+0x54` and `0x7c6e93` `+0x20`.

## Swim body pose

The slot `+0x3c` of the unit vtable, `0x608560`, poses a swimming unit's model from the mover's
pitch. A unit with 0x200000 and a direction bit (`flags & 3`) gets the pitched transform: the euler
builder `0x7bf4b0` is given `2π − pitch` read raw from `CMovement+0x20` (`0x60a110`, `0x60a11a`) and
the render facing `0x7c4ae0` of the facing `+0xc94`, and the 64 bytes of the matrix are copied into
the model with `0x710620`, with no flag test on the copy. Any other unit takes the yaw-only base
implementation `0x614cd0`. The only test that gates a republish of the matrix is the one in
`0x608560` itself.

Addresses: `0x608560`, `0x608569`, `0x608572`, `0x60857d`, `0x6085fb`, `0x608616`, `0x608628`, `0x614cd0`, `0x60a110`, `0x60a11a`, `0x60a122`, `0x60a172`, `0x607eb0`, `0x607ebd`, `0x7c4ae0`, `0x7bf4b0`, `0x710620`, `0x80c534`, `0x80afb4`
Confidence: verified
Proof: one
Test: none

`0x608569` loads `[unit+0x118]` (the embedded mover) and tests 0x200000 and `al,3` (`0x608572`,
`0x60857d`). The matrix is the identity pushed to `[ebp-0x40]` and handed to `0x60a110`.
`0x608560` has no call site and two stored references, `0x80c534` and `0x80afb4`, the vtable slots of the
unit and player classes; `0x614cd0` has one call (`0x608628`) and five stored references. The yaw
getter slot `0x607ea0` jumps to `0x607eb0`, which calls `0x7c4ae0` on `+0x9a8` with `[unit+0xc94]`.

## Create order

The pitch and flags are written before the model exists and before the first render call, so a unit
that comes into view swimming is drawn pitched from its first frame. In the post-create, `0x5fad50`
runs the apply (`0x5fb131`) and `0x466a20` then calls `0x613e10`, which creates the model and
registers the per-frame render callback `0x481540`; the object is linked into the manager's list
after that. The callback reaches `0x4815d0`, which calls the pose slot `+0x3c`.

Addresses: `0x466a20`, `0x466a63`, `0x466a6a`, `0x613e10`, `0x613ecd`, `0x613ed2`, `0x670db0`, `0x670ff0`, `0x481540`, `0x48158a`, `0x4815d0`, `0x48161a`, `0x683e79`, `0x683ea3`, `0x4662a4`, `0x4662dd`, `0x5fad7c`, `0x5fad88`, `0x5fae9d`, `0x5faef1`
Confidence: verified
Proof: one
Test: none

`0x466a63 call 0x5fad50` is followed by `0x466a6a call 0x613e10`; the splice into the manager's
list is `0x4662a4` to `0x4662dd`. `0x613ecd mov edx,0x481540` and `0x613ed2 call 0x670db0` register the
callback, which `0x670ff0` stores at `node+0xb4`; `0x683e79` loads it and `0x683ea3 call eax` runs
it. `0x481540` has no call site and one stored occurrence, the immediate at `0x613ece`;
`0x4815d0`'s only call is `0x48158a`. `0x48161a call [eax+0x3c]` is the pose call. `0x5fad88`
stores the mover pointer at `+0x118` before any of this, and `0x5faef1` seeds `+0xc94` from the
create block's facing.
