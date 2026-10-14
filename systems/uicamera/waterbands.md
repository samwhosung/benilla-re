# uicamera: the water bands

How the surface and submerged bands choose the pivot height, where that height steps, what the eased height does after a
drop, how a repeated crossing is absorbed, and how the water switch reaches the world trace. The band classifier and the pitch a crossing arms are `uicamera/solver` ("The water bands", "Crossing the surface"). The classifier's inputs are in
`objectlayer/swim` and the solver's frame in `uicamera/solver`; the frame order is `uicamera/driver`. In the entries `d` is
the liquid surface height above the unit's position, `T` the target pivot height at `+0x1c8`, `L` the live pivot height at
`+0xfc` and `M` the larger of the two.

Roots: `0x511ad0`, `0x50e570`, `0x50e8d0`, `0x512830`, `0x50ca90`

## Pivot height by band

`0x50e570` starts from `M` and works with a floor `lo` and a cap `hi`. With the water switch off or neither band set, `lo` is
5/6 and `hi` is `L`. In the surface band `lo` is `d + 2/9` and `hi` is the larger of `L` and `lo`. In the submerged band `lo` is
5/6 and `hi` is the larger of `d - 5/6` and 5/6. The probe lifts the start by `lo`, asks for `M - lo` but at least 1/9, and with
nothing in the way the height is `lo` plus that. The floor is then applied, and the cap second, so the cap wins when `L` is below 5/6.

| band | unobstructed height |
|---|---|
| neither, or switch off | `L` |
| surface | the larger of `L` and `d + 2/9` |
| submerged | the smaller of (the larger of `T`, `L` and 17/18) and (the larger of `d - 5/6` and 5/6) |

Addresses: `0x50e570`, `0x50e5a9`, `0x50e5c6`, `0x50e623`, `0x50e629`, `0x50e63e`, `0x50e657`, `0x50e666`, `0x50e67f`, `0x50e687`, `0x50e690`, `0x50e756`, `0x50e767`, `0x50e778`, `0x8089cc`, `0x808a04`
Confidence: verified
Proof: one
Test: none

`0x50e5a9` to `0x50e5cc` select `M` (`jp` picks `+0xfc` when it is at least `+0x1c8`). `0x50e5f7` sets `lo` to 5/6 and `0x50e607` `hi` to `L`.
`0x50e63e` stores `lo = 2/9 + d` for the surface bit and `0x50e657` raises `hi` to it when `L` is not larger. `0x50e666` stores `hi = d - 5/6`, and
`0x50e67f` raises it to 5/6. `0x50e690` and `0x50e695` form `M - lo` with the 1/9 minimum; with no hit `0x50e745` adds `lo` back, which is
`max(M, lo + 1/9)`. `0x50e756` stores `lo` when the height is below it and `0x50e767` stores `hi` when it is above.

## Surface band follows the liquid

In the surface band the height is the larger of `L` and `d + 2/9`, so while `d + 2/9` exceeds `L` the pivot sits exactly 2/9
yard above the unit's position plus the liquid depth and does not depend on `L`, `T` or the ease. A change in the liquid
height reaches the pivot one for one. The liquid height is the cached value at `+0x98` of the unit's world node, copied by
`0x670630`, so it moves only when the node's cache is refreshed.

Addresses: `0x511b39`, `0x511b58`, `0x511b5b`, `0x670630`, `0x50ed5e`, `0x50ed67`, `0x50eddb`
Confidence: verified
Proof: one
Test: none

`0x511b39` reads the cache, `0x511b58` the unit's position through the virtual at `+0x14` and `0x511b5e` subtracts its z from the cache. The
height out of `0x50e570` is added to the pivot z at `0x50ed67` and the sum goes to `0x50de00` at `0x50eddb`, with no easing between them.

## Steps at the band edges

With `L` equal to `T` and `T` above 25/18, the height drops by 19/18 yard when `d - T` reaches 2/9 (from `T + 4/9` in the surface
band to `T - 11/18` in the submerged band) and rises by 5/18 yard when it reaches 5/9 (from `T - 5/18` to `T`). Both steps are
independent of `T` and reverse on the way back. For `T` at or below 25/18 the submerged cap rests on its 5/6 floor and the drop
is the larger of `L` and `T + 4/9`, minus 5/6. Nothing smooths the height between the solver and the eye.

Addresses: `0x50e570`, `0x50e63e`, `0x50e666`, `0x50ed59`, `0x50ed5e`, `0x50eddb`, `0x8089cc`, `0x8089d0`, `0x8089d4`
Confidence: verified
Proof: one
Test: none

The two limits follow from the table above. Just under `d = T + 2/9` the surface cap is `d + 2/9 = T + 4/9`, below the lifted value `d + 1/3`. At the
edge the submerged cap is `d - 5/6 = T - 11/18`, below `M = T`. Just under `d = T + 5/9` the cap is `T - 5/18`, and in the neither band the height is `L = T`.

## Recentre after a drop

When the live height `L` is more than 1/9 yard above the height the solver returned, the driver sets `L` and the glide start to
that height plus 1/9 plus 2^-20, sets bit 0x20000000 and restarts the 2 second glide at the current time. The height can only
fall this way. With nothing in the way the surface and neither bands return at least `L`, so only the submerged band sets it
off, and it does on the frame the drop of 19/18 occurs. A recovery takes the glide's 2 seconds, restarted on every frame the
test holds.

Addresses: `0x50eeb0`, `0x50eeb6`, `0x50eec1`, `0x50eecf`, `0x50eeda`, `0x50eee0`, `0x50eee6`, `0x50eef2`, `0x50eef8`, `0x808a04`, `0x8026bc`
Confidence: verified
Proof: one
Test: none

`0x50eeb0` loads `+0xfc`, subtracts the solved height at `[ebp-4]` and compares with 1/9; `jne` at `0x50eec4` skips for a difference of 1/9 or less or unordered.
`0x50eed5` sets the glide bit, `0x50eeda` stores the 2.0 length at `+0x1c4`, `0x50eee0` the start time at `+0x1c0`, `0x50eef2` the new live height and `0x50eef8` the glide start at `+0x1cc`. The target `+0x1c8` is left alone. The recentre
runs after the eye is stored at `0x50ede0`, so the snap seeds the next frame's `M`. A height shortened by an obstruction can trip it in any band.

## Swim pivot preset

The height preset a swimmer uses, `+0x124`, is lowered by the scale times the difference between the maximum z of the model's
stand sequence (0) bounds and of its swim sequence (0x2a) bounds, so a model whose swim box is shorter than its stand box
swims with a lower pivot. The step runs only when the model has both sequences, applies to `+0x124` alone and is followed by
the 5/6 to 15 clamp shared with `+0x11c` and `+0x120`.

Addresses: `0x50ca90`, `0x50cc43`, `0x50cc4f`, `0x50cc59`, `0x50cc61`, `0x50cccf`, `0x50ccde`, `0x50cce3`, `0x50ccf6`, `0x50ccfc`, `0x711960`, `0x711a20`, `0x50f880`
Confidence: verified
Proof: one
Test: none

`0x50cc43` and `0x50cc4f` ask `0x711960` for sequence 0 and 0x2a; `0x50cc59` and `0x50cc61` skip the block when either is missing. `0x50cccf` and `0x50ccde` fill two records with
`0x711a20`, whose six bounds floats land at record `+0xc`, so `+0x20` is the maximum z. `0x50cce3` and `0x50ccec` scale the two by `S` at `[ebp-4]`, and `0x50ccf6`
subtracts their difference from `+0x124`. `0x50f880` picks `+0x124` for a swimmer.

## Repeated crossings

The kick has no latch: the previous bits are taken from the flag word each frame, and the classifier rewrites them each frame, so a depth that
flips between the bands produces an edge on every flip. `0x512830` absorbs a repeat while a pitch transition runs (bit 0x2000000) toward a target
within 0.001 of the request with the same delay and factor, returning 1 without restarting it. Both final pitch variables register the same default text
"5.0", so their requests have one target. The mouselook path has no such check. The classifier is also called by `0x50ca90`, which discards its result.

Addresses: `0x50ca90`, `0x50d0a9`, `0x512830`, `0x512898`, `0x5128a4`, `0x5128b6`, `0x5128f5`, `0x50be6c`, `0x50be8f`, `0x801360`
Confidence: verified
Proof: one
Test: none

`0x5128a4` tests bit 0x2000000, `0x5128ab` to `0x5128f3` compare `+0x1e0`, `+0x1ec` and `+0x1e8` with the target, delay and factor to within 0.001 (`0x801360`), and `0x5128f5` returns 1. The two
registrations at `0x50be6c` and `0x50be8f` both push the string at 0x84fc0c. `0x50d0a9` calls `0x511ad0` and the result is popped at `0x50d0ae`, so a rebuild between two driver frames can change the bits the next snapshot reads.

## Water switch in the trace

The mask `cameraWaterCollision` adds to the camera's world queries is 0xf0000: the base 0x100171 plus it, built once in `0x50e570` and
passed to all three queries (the pivot probe, the boom sweep and the near box clip). A world trace tests its liquid slots one bit
each, 0x10000 for slot 0 up to 0x80000 for slot 3, so with the switch on a camera query can hit a liquid surface. The same
register decides whether the band limits above apply, and the cap and floor are applied before the boom start is built from the height.

Addresses: `0x50e5ec`, `0x50e629`, `0x50e6bd`, `0x50e6f6`, `0x50e756`, `0x50e786`, `0x50e7d0`, `0x50e816`, `0x50e859`, `0x50e861`, `0x672170`, `0x69cc13`, `0x69cc31`, `0x69cc4d`
Confidence: verified
Proof: one
Test: none

`0x50e5ec` to `0x50e61a` form `neg esi; sbb esi,esi; and esi,0xf0000; add esi,0x100171` from the switch. `0x50e629` tests the nibble before the band limits, and `0x50e6bd`, `0x50e7d0` and
`0x50e859` push it for `0x672170` (twice) and `0x50de90`. The call chain from `0x672170` reaches `0x69cc13`, `test [ebp+0xc],0xf0000`, which skips every liquid layer when clear; `0x69cc31` starts the slot bit at
0x10000 over four slots and `0x69cc4d` tests it against the mask (`terrain/collide`).
