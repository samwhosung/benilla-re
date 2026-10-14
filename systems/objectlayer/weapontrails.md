# objectlayer: weapon trails

The trail behind a swinging weapon, from the unit's side. A visual kit arms a colour and a lifetime
on the unit, the unit's next animation hands them to its weapon-trail objects, and each object
draws from a hook on the weapon's own model. The trail record, its ring of sampled points and the
fade step are the player-name file's (`0x6c6560` and its ring helpers); this file holds what arms
a trail, who builds and fires it, and the state it draws with. The kit procedure that arms it is
in `auras` and the object list in `unitdisplay`.

Roots: `0x60d7c0`, `0x5fe2f0`, `0x60e550`, `0x608d60`, `0x60ba00`, `0x60d5d0`

## Kit parameter columns

A kit procedure has four float parameters, read from four 4-wide columns at `+0x4c`, `+0x5c`,
`+0x6c` and `+0x7c` of the `SpellVisualKit` record. A kind 8 procedure reads columns 0, 2 and 3 and
never column 1. Column 1 is read by kinds 13 (`0x60d8df`), 15 (`0x60d9e8`), 6 (`0x60da55`) and by
the arm shared by kinds 0 and 12 (`0x60da79`). In the shipped record file 34 slots hold kind 8,
each in a kit whose other three kinds are −1; their column 1 truncates to 20 (22 slots), 15 (6),
25 (2), 0 (2), 17 (1) and 22 (1), their lifetime to 300 to 10000 ms and their alpha to 0 to 150.

Addresses: `0x60d7c0`, `0x60d80a`, `0x60d8df`, `0x60d92e`, `0x60d9e8`, `0x60da24`, `0x60da55`, `0x60da5d`, `0x60da79`, `0x60db35`
Confidence: verified
Proof: one
Test: none

`0x60d7c0` loops four slots with `edi` on the third column; the column-1 operand is `[edi-0x10]`.
Its four reads are `0x60d92e`, `0x60da24`, `0x60da5d` and `0x60db35`, each inside the arm named
above by the jump table `0x60dbfc` and the byte map `0x60dc20` (kind 0 and 12 share `0x60da79`).
Neither caller of `0x60d7c0` (`0x60edf0`, `0x612a30`) nor any function that names the kit store
`0xc0d74c` reads `+0x5c` to `+0x68` of a kit, except the second kit walker `0x4727f0`, which
advances `esi` by 0x4c and reads only the kind and column 0. The record file has
1772 rows of 35 fields; kind 8 sits in the four kind fields at `+0x3c`.

## The arm latch

A kind 8 procedure arms the unit; the first `PlayAnimation` that clears its gates spends the arm.
The colour `+0xd1c` and the lifetime `+0xd20` have eight references in the image: the constructor
zeroes (`0x5faf63`, `0x5faf69`), the teardown clears `+0xd20` (`0x5fbcdc`), the arm writes both
(`0x60d828`, `0x60d835`) with plain stores, and `0x5fe48e` reads `+0xd20` and takes `+0xd1c` by
address (`0x5fe4a3`). A later arm overwrites a pending one; the colour has no clear of its own.

Addresses: `0x60d828`, `0x60d835`, `0x5faf63`, `0x5faf69`, `0x5fbcdc`, `0x5fe48e`, `0x5fe4a3`, `0x5fe4b1`, `0x5fe2f0`, `0x5fbb60`
Confidence: verified
Proof: one
Test: none

The read at `0x5fe48e` stores −1 in `+0xd60` first, then, for a non-zero lifetime, calls
`0x60e550(&+0xd1c, lifetime)` and clears `+0xd20` (`0x5fe4b1`); `0x60e550` writes nothing back into
the unit. The read is reached only by the three `je 0x5fe48e` of the swing fast path (`0x5fe449`,
`0x5fe452`, `0x5fe45d`): when the current and the requested animation are both swing ids the call
returns at `0x5fe48b` without spending the arm. The nine early outs (`0x5fe305`, `0x5fe314`,
`0x5fe330`, `0x5fe33e`, `0x5fe353`, `0x5fe368`, `0x5fe390`, `0x5fe3a1`, `0x5fe3b6`) skip it too.
`0x5fbb60`, the unit teardown whose sole caller is `0x5e9b61`, clears `+0xd20` after destroying the
trail objects (`0x5fbc99`).

## Arming without a play

A pending arm waits for the next `PlayAnimation`, whatever animation that is and whenever it comes:
a kit that arms without playing, a channel start that arms and returns, and a run of arms before
any play all leave it pending, and the last arm wins. `PlayAnimation` has 40 call sites and
`0x5fd9e0`, which picks the base animation and calls it, has 38; `0x5fd9e0` itself calls the
picker `0x5fd8b0` (`0x5fda03`), its only call site; one of the 38 is the per-frame update `0x607ed0` (`0x6084e0`), which re-picks the
base animation when a turning unit may shuffle.

Addresses: `0x60edf0`, `0x60f35c`, `0x60f366`, `0x60f383`, `0x60f3c5`, `0x612a30`, `0x612b18`, `0x612b1d`, `0x5fd9e0`, `0x5fda20`, `0x6084e0`
Confidence: verified
Proof: one
Test: none

`0x60edf0` calls the dispatcher at `0x60f35c`, then skips the animation leg for a kit animation id
below 0 (`0x60f366 jl`). In the leg a kit played as kind 2 calls `0x5fd9e0(−1)` when the current
animation differs (`0x60f383`), ids 8 to 10 go to `0x60ea70(0)` (`0x60f510`), and the rest reach
`0x5fe2f0` at `0x60f3c5`. `0x612b18` arms inside the channel start and `0x612b1d` returns without a
play. Of the 34 kind 8 slots, 4 are in kits whose animation is −1 and 23 in kits whose animation is
a swing id (16 to 18, 57 or 85), so two consecutive swing procs take the fast path.

## Trail objects

A unit keeps one trail object per hand at `+0xd34` (main hand) and `+0xd38` (off hand). The ranged
hand has none: its bowstring is a separate object at `+0xd24`. `0x608d60(model, hand)` destroys the
hand's old object, asks the unit's virtual slot `+0x98(hand, 0)` for the hand's weapon record and,
if there is one, finds the child of the body model at the weapon's sheath attachment and allocates
the object on it. A player's slot is `0x5ec240` (its equipment) and any other unit's `0x605e30`
(its virtual item display). Neither compares an item's inventory type, so an off-hand trail exists
exactly when the slot returns a record.

Addresses: `0x608d60`, `0x608d71`, `0x608d73`, `0x608d95`, `0x608da8`, `0x608db1`, `0x608dbc`, `0x5ec240`, `0x605e30`, `0x5ed867`, `0x605e03`, `0x60ba00`, `0x60d5d0`, `0x608dd0`, `0x47a070`, `0x712f00`
Confidence: verified
Proof: one
Test: none

`0x608d6c` and `0x608d71` return for a hand other than 0 or 1. The destroy at `0x608d73` runs
before the record is asked, so an unarmed hand ends with its slot zeroed. The attachment is
`0x47a070(record byte +4, hand == 0)`. Only two callers build: `0x5ed867` in `0x5ed700`, with
equipment slots 15, 16 and 17 mapped to hands 0, 1 and 2 (`0x5ed845`, `0x5ed849`, `0x5ed850`), and
`0x605e03` in `0x605da0`, which the virtual item watcher `0x604cc0` calls for fields 37 to 39
(`0x6041f0` registers offsets 0x7c, 0x80 and 0x84). The census of `0x608d60` finds those two and the
two inside `0x608dd0`; `0x608dd0` has no call, jump or stored address. `0x60ba00` (from `0x5dee30`)
releases one hand and `0x60d5d0` (from `0x5fb672` and `0x5fbc99`) both.

## Trail hook

An object is made on the weapon's attached model, not on the body or the unit. `0x6c6750` installs
`0x6c67f0` on that model as its draw hook, through the setter `0x710db0(function, argument,
flag)`, with the object as the argument and flag 1. The setter is a plain store of three dwords at
model `+0x1b8`, `+0x1bc` and `+0x1c0`, so a model has one hook, and a weapon swap that frees the
model frees the hook with it.

Addresses: `0x6c67f0`, `0x6c67a4`, `0x6c67aa`, `0x710db0`, `0x70d960`, `0x70d987`, `0x70d99a`, `0x6c6710`, `0x6c6920`, `0x6c6970`
Confidence: verified
Proof: one
Test: none

`0x70d960` loads the drawn model from `[scene+0x3310]`, pushes its `+0x1bc`, puts its `+0x3b8` colour
object in `edx` and calls `[model+0x1b8]` (`0x70d99a`). The hook is therefore
`0x6c67f0(ecx = weapon model, edx = colour object, [esp+4] = object)`, and its `[ebp+8]` is read as
the object whose first dword is the swing record (`0x6c689d`, `0x6c68a0`). `0x6c6920` takes a
reference on the model (`0x710390`) and stores it at object `+4`; `0x6c6710` clears the hook with
`0x710db0(0, 0, 1)`, drops the reference (`0x7103a0`) and frees the swing record. The address
`0x6c67f0` is stored once in the image, as the push at `0x6c67a4`. Without both model events `$WTB`
and `$WTT` (`0x7130e0` on the ids `0x42545724` and `0x54545724`) the hook returns and draws
nothing.

## Trail draw state

The hook brackets its draw in a render-state save and restore (`0x589f40`, `0x589f50`) and sets six
states through `0x589e60`: id 0x14 (cull) to 0, 0x0f (fog) to 0, 0x0e (lighting) to 1, 0x07 (blend
mode) to 2, 0x12 (depth write) to 0 and 0x10 (depth test) to 0. The trail is a lit, alpha-blended,
two-sided ribbon with no fog that ignores the depth buffer and writes none. It sets no texture:
vertex format 7 is a position and a colour dword, stride 16, with no texture coordinate and no
normal.

Addresses: `0x6c6825`, `0x6c6831`, `0x6c683d`, `0x6c684c`, `0x6c685b`, `0x6c6867`, `0x589e60`, `0x589f40`, `0x589f50`, `0x593710`, `0x58a7c0`, `0x5a317e`, `0x5a320c`
Confidence: verified
Proof: one
Test: none

Each state is `xor edx,edx` or a `mov edx` value and a `mov ecx,id` before the call at the listed
address. Blend mode 2 takes the entries 5 and 6 of the Direct3D source and destination factor tables
(source alpha, inverse source alpha), and setting id 7 also sets id 8, the
alpha reference, to `0x85ad20[2]` = 1, so fragments with alpha 0 are rejected. Depth test 0 is
Direct3D's depth function 8, always (`0x5a31a1`), and culling 0 is the cull mode table's entry 1,
none (`0x5a321e`). No state in the hook is a texture or combine id (`0x17` to `0x26`), so stage 0 keeps what the model's
material setup `0x70ca50` left; whether that texture then takes part in a draw whose format has
no texture coordinate is not read here. Format 7 is the row `{descriptor, 2 elements, stride 0x10,
mask 0x11}` of the format table `0x809c00`.

## Trail vertices

Each frame the step `0x6c6560` reserves `n` 16-byte vertices (`0x58a140` with kind 0), locks them
(`0x58a080`), fills them with `0x6c64d0`, unlocks (`0x58a0a0`), selects format 7 and draws one
non-indexed record of primitive 4, a triangle strip, with vertex count `n`. `n` is the live point
count, always even and at least 2, so the strip has no triangle until the ring holds two frames. The
colour dword is the stored colour (`B,G,R,A` in memory) on Direct3D and `R,G,B,A` on OpenGL, chosen
by the device flag `+0x258` of the capability block (`0x58a230`).

Addresses: `0x6c6560`, `0x6c661e`, `0x6c662f`, `0x6c664d`, `0x6c6653`, `0x6c667b`, `0x6c66cf`, `0x6c66d4`, `0x6c66f2`, `0x58a830`, `0x58a230`, `0x58a140`, `0x58a080`, `0x58a0a0`, `0x597ac0`
Confidence: verified
Proof: one
Test: none

`0x6c6613` passes twice the segment count to the trim `0x6c6540`, which returns the live count; the
count `edi` sizes the buffer (`0x6c662f`), bounds the fill and is the descriptor's `+0x08`
(`0x6c66d6`). The descriptor is `{u32 4, u32 0, u16 n, u16 0, u16 n−1}`; its `+0x0c` is a vertex
index bound that only the indexed draw reads, and the indexed flag is 0 (`0x6c66db`). The head and
tail move by 0 or 2 per frame (`0x6c6587`, `0x6c65a1`), both reset to 0 at an arm, and the tail is
set to `head − 128` or `head − 2·segments`, so `n` is even; `0x6c6620` rejects 0. At the device
flag 1 the dword is rebuilt as `(A<<24)|(B<<16)|(G<<8)|R` from the bytes `+0x608` to `+0x60b`
(`0x6c6653` to `0x6c6677`); otherwise it is read whole (`0x6c667b`). The stored colour is `A,R,G,B`
from the top, built by `0x60d828` from a `0x00RRGGBB` parameter and the alpha byte.
