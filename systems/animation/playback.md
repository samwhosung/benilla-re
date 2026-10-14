# animation: starting a sequence

What a play call does to the bone states that the animate pass reads. `CM2Model::PlayAnimation` (`0x7121a0`) arms
one bone: it picks the sequence record, a variation and a repeat count, writes the bone's primary slot or its
blend slot with a play window, and starts or ends a fade. The animate pass then spreads the bone's time to the
bones below it (`clock`). Two setters retime a running play. A call made before the model has loaded is queued and
replayed (`models/model#deferred-calls`). `models/animation#playing-an-animation` states the arithmetic; this doc
gives the order of the steps, which bone and slot are written, and when a call changes nothing.

Roots: none

## Which bone and slot

`PlayAnimation(keyBone, id, variation, offset, speed, blend, primary)` queues itself while the instance is not
loaded. Once it is loaded, the call plays on exactly one bone: bone 0 for key bone −1, else the 16-bit entry `keyBone` of the header's key bone
lookup (`+0x3c`, `0x710310`); it does nothing when that bone index is not below the bone count. With `primary`
nonzero the call writes the bone's primary slot, `+0x98` to `+0xc0`; with `primary` 0 it writes the blend slot,
`+0xc4` to `+0xec`. No call loops over the bones: the bones whose own primary record is −1 take their parent's time in
the next pass.

Addresses: `0x7121a0`, `0x7121aa`, `0x7121ad`, `0x7121b3`, `0x712233`, `0x71223c`, `0x71224e`, `0x71225b`, `0x712261`, `0x712267`, `0x712514`, `0x712526`, `0x71252c`, `0x71268c`, `0x712692`, `0x710310`
Confidence: verified
Proof: one
Test: none

The unloaded branch `0x7121b3 jne 0x712233` queues kind 4 with the seven stack arguments at `+0xc` through `+0x24`.
`0x712239 cmp ecx,-1` gives bone 0; `0x71224e call 0x710310` is `M2LookupIndex`. The block is `[+0x90] + bone·0x118`
(`0x712272`). `0x712514 mov ecx,[ebp+0x20]; test; je 0x712647` forks on `primary`: the primary leg sets `esi = ebx+0x98`
(`0x71262d`, `0x71263f`), the other `esi = ebx+0xc4` (`0x71268c`), and the common tail at `0x712692` writes through `esi`.

## Replacing a running play

Every loaded call, stop or blend play included, starts by calling the model's done callback `+0x70` for the
sequence it may replace. The callback runs when it is set, the bone's primary record `+0xa4` is not −1, its latch
`+0xc0` is 0, and either the bone record's key bone id is not −1 or the bone index is 0. The call passes the key bone
in `edx` and the requested id `+0xf8`, 1, 0 and the stored arguments `+0x78`, `+0x7c` on the stack, between `AddRef`
(`0x710390`) and `HandleClose2` (`0x7103a0`), and ends if that release frees the instance.

Addresses: `0x71226d`, `0x71227e`, `0x712280`, `0x712289`, `0x712291`, `0x712297`, `0x71229d`, `0x7122a5`, `0x7122b1`, `0x7122bc`, `0x7122c1`, `0x7122c6`, `0x7122cb`, `0x710390`, `0x7103a0`
Confidence: verified
Proof: one
Test: none

`0x71227e je 0x7122d5` skips for a null callback, `0x712287 je` for a bone with no record, `0x71228f jne` for a
latched one. `0x712297 cmp [ecx+eax],-1` is the bone record's first dword. The pushes before `0x7122c1 call [esi+0x70]` are
`+0x7c`, `+0x78`, 0, 1 and `+0xf8`, with the key bone argument in `edx`. `HandleClose2` returns the new reference
count, so `0x7122cb test eax,eax; je 0x7127de` ends the call when the callback released the last reference.

## Variation and repeats

An id of −1 stops the bone (`Stopping a bone`). Otherwise the call resolves the id through `0x711bf0` and the header's
sequence lookup (`+0x24`), and returns when the record index is not below the sequence count (`+0x1c`). A variation
of −1 draws once with `rand`: from the head record it subtracts each record's probability `+0x14` from the draw while
the draw is not below it and follows the link `+0x40`, ending on the head record when the chain ends first. A variation
`n` above 0 follows the link `n` times without testing for the end mark. Every write draws a repeat count
`min + ((max − min) · rand()) / 32768`, truncating toward zero, or 1 when that is 0.

Addresses: `0x71245d`, `0x712464`, `0x712470`, `0x712482`, `0x712484`, `0x71248d`, `0x712490`, `0x71249a`, `0x7124b9`, `0x7124bf`, `0x7124c2`, `0x7124d4`, `0x7124de`, `0x7124f0`, `0x712502`, `0x712698`, `0x7126a5`, `0x7126a8`, `0x7126b9`, `0x7126c3`, `0x7126cd`, `0x7400e5`
Confidence: verified
Proof: one
Test: none

`0x7124bf cmp [ebp+8],esi; jb 0x7124e3` stops when the remaining draw is below the probability, and
`0x7124d4 cmp cx,0xffff` ends the chain. On the exhausted chain `edi` still holds the head. The ordinal of the
chosen record is stored for the bone's requested variation `+0xfc`. `0x7126a8 cdq; and edx,0x7fff; add; sar 15` is the
divide, `0x7126c3 sete al` and `0x7126cd add edi,eax` the 0-to-1 fix. The draw is `_rand`, whose seed
`random` gives; its range 0 to 0x7fff keeps the quotient below `max − min`, so the count is below `max` before the 1.

## Play window

A write takes `now` the scene clock, `speed` the argument, negated when the id's reverse flag (class 1 or 3 of
`0x711bf0`) is set and zeroed when its hold flag (class 2 or 3) is set, and `length = (end − start) · repeats`. The offset
field `+0xb8` is `length` when `speed` is negative before zeroing, else 0. `inv` is `1 / speed`, or 0 when
`|speed| ≤ 1e-5` or is not a number. `lo = now − ftol(float(offset) · |inv|)` and `hi = lo + ftol(float(length) · |inv|)`.
The slot receives the record index, `lo`, `hi`, `speed`, `inv`, the offset field, the repeats, and 0 for the latch.

Addresses: `0x7126d2`, `0x7126db`, `0x7126e4`, `0x7126ea`, `0x7126f4`, `0x7126fc`, `0x712701`, `0x712704`, `0x712718`, `0x71271c`, `0x712722`, `0x71272f`, `0x71273d`, `0x712742`, `0x712755`, `0x712768`, `0x71276b`, `0x712770`, `0x71277f`, `0x712784`, `0x712787`, `0x71278f`, `0x712792`, `0x712795`
Confidence: verified
Proof: one
Test: `models::keyframe_timing_difftest`, `models::keyframe_ratio_difftest`

`length` is an unsigned 32-bit product stored with a zero high dword and read by `fild qword`; the offset is `fild
dword`. The stores `0x71276b` (record), `0x712758` (`lo`), `0x712784` (`hi`), `0x71277f` (`speed`), `0x712787`
(`inv`), `0x71278f` (offset field), `0x712792` (repeats) and `0x712795` (latch) are at `esi+0xc` through `esi+0x28`.
A held play has speed 0, `inv` 0 and an empty window. The clock legs of `clock` read these fields: a record with flag
bit 0 clear loops by modulo for ever whatever the window, one with it set loops the repeat count inside the window
and then holds `end`, or `start` for a reversed play.

## Fade slot

A primary play with `blend` 0 sets `+0xd0` to −1. With `blend` set it copies the old primary slot (`+0x98` to `+0xc0`)
into the blend slot, sets the fade end `now + blendTime` (`+0x100`), `1 / blendTime` (1.0 for 0; `+0x104`) and 1.0
(`+0x108`), unless the old latch `+0xc0` is set and the new record is the one in `+0xa4`, or the blend slot is busy
with a weight `w` above 0.5 or not a number. A play with `primary` 0 takes the blend slot: the fade end
`now + (end − start)`, rate `1 / (end − start)` (1.0 for an empty range) and scale 0.75.

Addresses: `0x71252c`, `0x712541`, `0x712549`, `0x712553`, `0x71255f`, `0x712566`, `0x712576`, `0x712579`, `0x71257c`, `0x71259a`, `0x7125b2`, `0x7125c9`, `0x7125d1`, `0x7125d4`, `0x7125d6`, `0x7125f2`, `0x71260c`, `0x712617`, `0x712623`, `0x71262d`, `0x712635`, `0x712647`, `0x71265f`, `0x712682`
Confidence: verified
Proof: one
Test: `models::rate_reciprocal_difftest`, `models::anim_fade_difftest`

`w` is the kernel's weight computed again: `rem = [+0x100] − now`, `x = f32(float(rem) · [+0x104])`, 0 for `x < 0`, 1 for
`x > 1`, else `(3 − 2x)·x·x`, times `[+0x108]` (`0x712576` to `0x7125c9`). `0x7125c9 fcomp 0.5; test ah,0x41; jp 0x71262d`
keeps the slot when `w > 0.5` or unordered. `0x712549` to `0x712559` skip the fade for a latched same-record play.
A refused fade still writes the primary slot in full (`0x71262d`). The blend time is the new record's `+0x20`
(`0x7125f2`); `0x7125fe test eax,eax; je 0x712617` gives the 1.0 for a zero time.

## Stopping a bone

An id of −1 clears the bone's request. With `primary` nonzero it unlinks the bone from the active list, and when `blend` is
set it snapshots the primary slot into the blend slot as above with the fixed fade 150 ms, rate `0x3bda740e`
(1/150) and scale 1.0, unless the blend slot is busy with `w` above 0.5 or not a number; with `blend` 0 it clears the blend record instead. It
then clears the requested id (−1) and variation (0), the primary record (−1) and the cursors, speed, inverse and offset.
With `primary` 0 it clears the blend record and its cursors, speed, inverse and offset, and nothing else.

Addresses: `0x7122d5`, `0x7122dd`, `0x7122e3`, `0x7122ec`, `0x712324`, `0x712333`, `0x712339`, `0x71233f`, `0x7123a2`, `0x7123af`, `0x7123bb`, `0x7123cb`, `0x7123d8`, `0x7123e2`, `0x7123f1`, `0x7123f9`, `0x712405`, `0x712429`, `0x712430`, `0x712432`, `0x71244a`
Confidence: verified
Proof: one
Test: none

`0x7123cb add eax,0x96` is the 150. `0x7123af` is the snapshot, reached when the slot is free (`0x71233f je`) or
`0x7123a2 fcomp 0.5; test ah,0x41; jp 0x7123f7` finds `w ≤ 0.5`; `0x7123f1` clears `+0xd0` for `blend` 0. The primary clears
`0x7123f9` to `0x712423` store −1, 0, −1 and zeros at `+0xf8`, `+0xfc`, `+0xa4`, `+0xa8`, `+0xac`, `+0xb0`, `+0xb4`,
`+0xb8`. The non-primary stop `0x712430` to `0x71244a` stores −1 at `+0xd0` and 0 at `+0xd4` to `+0xe4`.

## Active list

A primary play links the bone's state into the instance's active list when it is not linked: `+0x80` holds the head
bone index (−1 when empty), a state's `+0x110` the address of the cell that points to it and `+0x114` the index of the
next bone, and the new bone becomes the head. A stop unlinks it. A play with `primary` 0 does not touch the list.

Addresses: `0x7122ec`, `0x7122f6`, `0x7122fe`, `0x712308`, `0x71231a`, `0x712327`, `0x71232d`, `0x71279c`, `0x71279e`, `0x7127ab`, `0x7127b1`, `0x7127b7`, `0x7127c2`, `0x7127cc`, `0x7127d9`, `0x7127dc`
Confidence: verified
Proof: one
Test: none

The link at `0x71279e` tests `[ebx+0x110]` first, so a bone already linked is left in place. `0x7127ab lea ecx,[edx+0x80]`
is the head cell; `0x7127b1` stores it in `+0x110`; `0x7127b7 mov eax,[ecx]` copies the old head into `+0x114`, and
`0x7127cc imul eax,eax,0x118; 0x7127d2 mov [eax+edx+0x110],ebx` points the old head's cell at the new state; `0x7127dc`
stores the bone index at `+0x80`. `Update` walks this list (`models/animation#update-and-events`).

## Retiming a key bone

`SetKeyBoneAnimTime(keyBone, time)` (`0x7127f0`) and `SetKeyBoneAnimSpeed(keyBone, speed)` (`0x712910`) act on a bone
whose primary record is not −1, resolved as in a play, and are queued as kinds 5 and 6 while the model loads. The first
sets `lo = now − ftol(float(time) · |inv|)` with the stored `inv` (`+0xb4`). The second takes the position
`pos = ftol(float(now − lo) · speed) + offset` under the old speed, sets `inv = 1/speed` (0 when `|speed| ≤ 1e-5` or
is not a number) and `lo = now − ftol(float(pos) · |inv|)`, and stores `speed` and `inv`. Both then set
`hi = lo + ftol(float64((end − start) · repeats) · |inv|)`. The offset `+0xb8` is unchanged.

Addresses: `0x7127f0`, `0x71282c`, `0x712887`, `0x712895`, `0x7128a3`, `0x7128bb`, `0x7128c0`, `0x7128c9`, `0x7128e0`, `0x7128f4`, `0x712910`, `0x71294c`, `0x7129b9`, `0x7129db`, `0x7129ee`, `0x7129f4`, `0x712a12`, `0x712a23`, `0x712a28`, `0x712a33`, `0x712a42`, `0x712a68`, `0x712a76`
Confidence: verified
Proof: one
Test: `models::m2_set_bone_timed_difftest`, `models::bone_timed_value_difftest`

`0x7128a3 fld [edi+0xb4]; fabs` takes `|inv|`. The elapsed position of `0x7129db` onward reads `+0xb0` (`0x7129ee`),
the converted position is added to `+0xb8` (`0x7129fc`, `0x712a07`) and the divide is `fld 1.0; fdiv speed` (`0x712a28`,
`0x712a2e`) after `fcomp 1e-5; test ah,0x41; jne` picks 0 (`0x712a33`). The queue records of kinds 5 and 6 carry the
key bone and the time or speed at `+0xc` and `+0x10` (`0x712848`, `0x712968`).
