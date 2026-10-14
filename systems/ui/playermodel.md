# ui: PlayerModel, DressUpModel and TabardModel

How the character model panes extend a `Model` frame: their classes and vtables, the model copy they
show, the turn animation behind `SetRotation`, and what a pane leaves playing. The camera they choose is
in `modelview`; their Lua method tables are in `model`.

A pane shows a copy of a unit's model, in a frame that adds a unit GUID, a turn latch and a deadline to
a `Model` frame. `DressUpModel` and `TabardModel` extend `PlayerModel`; all three are built by the same
base constructor, `CGCharacterModelBase` (`CharacterModelBase.cpp`), and the paper doll, pet, stable,
inspect, dress-up, auction dress-up and tabard panes are these types.

Roots: `0x505680`, `0x5041d0`, `0x502780`, `0x505b30`, `0x505c50`, `0x505f00`, `0x5062a0`

## Pane classes

Five frame classes share the model-frame paint. The pane constructors store their own vtables over the
`Model` frame's; the tables differ in slots 33, 34, 36, 38 and 39:

| class | vtable | size | slot 38 (per paint) | slot 39 (model ready) |
|---|---|---|---|---|
| `Model` | `0x81c608` | 0x3dc | `0x76d1a0` | `0x76ce00` |
| `ModelFFX` (glue) | `0x803960` | 0x3dc | `0x76d1a0` | `0x76ce00` |
| `PlayerModel` | `0x8084f8` | 0x3f8 | `0x505c50` | `0x505b30` |
| `DressUpModel` | `0x8083a8` | 0x410 | `0x504470` | `0x505b30` |
| `TabardModel` | `0x808260` | 0x410 | `0x505c50` | `0x503c10` |

Slot 33 is `0x505ce0` and slot 34 is `0x505d00` in the three pane tables, the base frame's `0x76ad50` and
`0x76ae10` in the other two. `DressUpModel` slot 36 is `0x504350`; the rest are the `Model` frame's.

Addresses: `0x505680`, `0x5056bc`, `0x5056c2`, `0x5041d0`, `0x5041e9`, `0x502780`, `0x50278f`, `0x504470`, `0x504473`, `0x503c10`, `0x504350`, `0x8084f8`, `0x8083a8`, `0x808260`, `0x803960`, `0x81c608`, `0x495bd0`, `0x495c00`, `0x495c30`, `0x46fa3f`, `0x6ee880`
Confidence: verified
Proof: one
Test: none

`0x4959xx` registers the type names through `0x6ee880`: `PlayerModel` with the factory `0x495bd0`
(which allocates 0x3f8 bytes and calls `0x505680`), `DressUpModel` with `0x495c00` and `TabardModel` with
`0x495c30` (0x410 bytes each). `0x505680` calls the `Model` constructor `0x76c8e0` (`0x50568e`) and
stores `0x8084f8` at `+0x0` (`0x5056bc`) and the geometry table `0x8084cc` at `+0x24`; `0x5041d0` and
`0x502780` call it and store their tables (`0x5041e9`, `0x50278f`). The tables are 40 slots each, ending
at the first non-code dword. `0x504470` runs `0x505c50` and then a hook on `+0x3f8`; `0x503c10` runs
`0x505b30` and moves the camera target (`tabard camera target`). `ModelFFX`'s constructor stores
`0x803960` at `0x46fa3f`.

## Pane fields

A pane keeps the unit GUID it shows at `+0x3e0` and `+0x3e4`, a latch at `+0x3e8` that a rotation sets
and the next paint clears, a deadline in milliseconds at `+0x3ec`, and a creature display record at `+0x3f0` for a
pane shown from a display instead of a unit. The constructor zeroes all five.

Addresses: `0x505680`, `0x50569e`, `0x5056a4`, `0x5056aa`, `0x5056b0`, `0x5056b6`, `0x505b50`, `0x505b64`, `0x505b70`, `0x505cb0`, `0x505cc1`, `0x505ccd`
Confidence: verified
Proof: one
Test: none

`0x505680` stores zero at `+0x3e0` to `+0x3f0` (`0x50569e` to `0x5056b6`). `SetUnit`'s helper `0x505b50`
stores the GUID (`0x505b64`, `0x505b6a`) and zero in `+0x3f0` (`0x505b70`); `0x505cb0` zeroes the GUID and
stores its argument as the display record (`0x505cc1` to `0x505ccd`) and, if the frame is effectively
visible, builds the model with `0x505a70`.

## The pane model

A pane shows a duplicate of the unit's model, never the unit's own. Showing the pane builds the duplicate
from the unit named by the GUID, or from the display record; hiding it destroys the model. A copy is built by
creating the unit's model in the pane's scene, resetting its partial attach state, handing it to the frame
(slot 36), registering the pane's completion callback, arming Stand with a random variation, and
re-applying the two hand grips.

Addresses: `0x5059a0`, `0x5059b2`, `0x5059ce`, `0x5059d5`, `0x5059fb`, `0x505a05`, `0x505a17`, `0x505a2f`, `0x505a38`, `0x505a48`, `0x505a58`, `0x505a5c`, `0x505d00`, `0x505d03`, `0x505d30`, `0x505d3c`, `0x505d56`, `0x505ce0`, `0x505ce7`, `0x505b50`, `0x505b8d`, `0x505b99`, `0x707400`, `0x47a230`, `0x711bb0`, `0x7121a0`, `0x712f00`, `0x479660`
Confidence: verified
Proof: one
Test: none

`0x5059a0(unit)` reads the unit's model from `+0xd8`, passes it with the pane's scene (`0x76cfc0`) to
`0x707400` (`0x5059d5`), calls `0x47a230` (`0x5059fb`) and slot 36 (`0x505a05`), registers `0x5062a0`
with the frame (`0x505a17`) and calls `0x7121a0(−1, 0, −1, 0, 1.0, 1, 1)` (`0x505a2f`), then queries the two
hand grips with `0x712f00` and sets them with `0x479660` (`0x505a48`, `0x505a5c`). The show hook `0x505d00`
(slot 34) builds the copy when the frame is not yet effectively visible (`0x505d03`): from the unit
found by the GUID with type mask 8 (`0x505d30`) through `0x5059a0`, or from `+0x3f0` through `0x505a70`
(`0x505d56`), then chains the base. The hide hook `0x505ce0` calls slot 36 with null (`0x505ce7`) and
chains the base. `0x505b50` builds at once if the frame is visible (`0x505b8d`, `0x505b99`).

## Turning the model

`SetRotation(angle)` plays a shuffle animation toward the angle, latches the turn and stores the angle
as the frame's facing. When the model's data is resident it compares the old facing with the new:
old below new plays animation 12 (ShuffleRight), old above new plays 11 (ShuffleLeft), and equal or NaN
plays 0 (Stand). The play is skipped if that id is already armed on key bone 0, so a held direction arms
once. Reversing replaces the shuffle at once, restarting it. Sending the same angle again plays Stand.
The latch is set to 1, the deadline to now + 100 ms and the facing is stored on every path.

Addresses: `0x505f00`, `0x505fac`, `0x505bb0`, `0x505bbf`, `0x505bcc`, `0x505bd6`, `0x505bde`, `0x505be0`, `0x505bf5`, `0x505bf7`, `0x505c04`, `0x505c09`, `0x505c23`, `0x505c28`, `0x505c32`, `0x505c37`, `0x505c44`, `0x712090`, `0x7121a0`, `0x42c010`, `0x8686a8`
Confidence: verified
Proof: one
Test: none

`0x505bb0` is called once, from `0x505fac` in the Lua glue. It exits to the convergence point `0x505c28`
without a model (`0x505bbf`) or without resident data (`0x505bcc`). `fcomp [argument]` of the facing
sets C0 when the facing is less: `test ah,5; jp` falls through only then (`0x505bde`), and `edi = 0xc`
(`0x505be0`). Otherwise a second compare with `test ah,0x41; jne` (`0x505bf5`) sets `edi = 0xb` only for
a greater facing. `0x712090(−1)` returns the id armed on bone 0 (`[bone + 0xf8]`, `0x7120d3`); equal to
`edi` skips the play (`0x505c09`). The play is `0x7121a0(−1, edi, −1, 0, 1.0, 1, 1)` (`0x505c23`).
`0x505c28` stores the latch 1, `0x42c010` plus 100 (a millisecond clock, `0x505c32`, `0x505c37`) and the
facing (`0x505c44`). The names are the table at `0x8686a8`: 0 Stand, 11 ShuffleLeft, 12 ShuffleRight,
15 HandsClosed, 26 Ready1H.

## The turn timer

Each paint of a pane rebuilds the root transform and then ends a turn: if no rotation was sent since the
previous paint, the deadline has passed (strictly), and the model's bone 0 has a nonzero id armed, it
arms Stand with a random variation and zeroes the deadline. The latch is cleared on every paint. While a
rotate button is held, `Model_OnUpdate` sends a rotation per frame, so the deadline moves forward and the
latch is set each paint, and the shuffle runs until about 100 ms plus one to two paints after the last
call.

Addresses: `0x505c50`, `0x505c54`, `0x505c5f`, `0x505c63`, `0x505c6a`, `0x505c70`, `0x505c7a`, `0x505c81`, `0x505c98`, `0x505c9d`, `0x505ca3`, `0x42c010`
Confidence: verified
Proof: one
Test: none

`0x505c54` runs `0x76d1a0` first. The three guards are `cmp [+0x3e8], 0; jne` (`0x505c63`),
`cmp now, [+0x3ec]; jbe` (`0x505c70`, unsigned) and `0x712090(−1)` zero (`0x505c81`). The arm is
`0x7121a0(−1, 0, −1, 0, 1.0, 1, 1)` (`0x505c98`); the deadline is stored as 0 (`0x505c9d`), not a sentinel,
so guard 2 passes on every paint outside a drag. The latch store at `0x505ca3` is on every path.

## Stand is the resting state

Any sequence armed on bone 0 of a pane other than Stand is replaced by Stand on the next paint outside a
rotation: `SetSequence(n)` does not hold on a `PlayerModel`, `DressUpModel` or `TabardModel`, while on a
plain `Model` it does. When a track completes, the pane's completion callback arms Stand again for
whole-body or Ready1H tracks, ignores HandsClosed, and stops the track on any other key bone.

Addresses: `0x505c50`, `0x505c81`, `0x5062a0`, `0x5062a8`, `0x5062aa`, `0x5062af`, `0x5062b4`, `0x5062ca`, `0x5062e4`, `0x76dec0`, `0x76cf50`, `0x7121a0`
Confidence: verified
Proof: one
Test: none

The guards of `0x505c50` leave Stand (id 0) alone and re-arm any other armed id (`0x505c81`), whatever
armed it. `SetSequence` arms through `0x76cf50` → `0x7121a0(−1, n, −1, 0, 1.0, 0, 1)`, which writes the
id into the bone block; the next paint stomps it. The callback `0x5062a0`, registered by `0x5059a0`, has
the mode at `[ebp+0xc]`: nonzero returns (`0x5062a8`); key bone −1 or 26 (`0x5062aa`, `0x5062af`) arms
Stand (`0x5062e4`); animation id 15 returns (`0x5062b4`); otherwise it stops the key bone with id −1
(`0x5062ca`).

## Pane models have no event callback

A pane's model has no event callback, so the keys in its animation's event track never run. The footfall
keys of the shuffle sequences (`$SL`, `$SR`) therefore draw no decal and play no sound. The pane
registers only a completion callback.

Addresses: `0x76cd30`, `0x76cd8a`, `0x76cd9b`, `0x76cdac`, `0x7133a0`, `0x7133a9`, `0x70e0a0`, `0x70dd70`, `0x719370`, `0x719524`, `0x719526`, `0x607831`, `0x5059a0`
Confidence: verified
Proof: one
Test: none

The model's event callback is at `+0x1f4`; `0x70dd70` initialises it to 0 (`0x70e0a0`) and only
`0x7133a0` (`0x7133a9`) stores it. `0x719370` skips the event scan of a track when it is null
(`0x719524`, `0x719526`). `0x7133a0` has 16 callers, all in the object layer, terrain and spell
effects (for units, `0x607831` in `CGUnit_C::SetModelCallbacks`); none is in interface code. `0x76cd30`
registers the completion callback (`0x76cd8a`), the light callback (`0x76cd9b`) and the ready hook
(`0x76cdac`) and no event callback, and a pane's model is built with a fresh instance (`0x707400`),
which does not copy the unit's callbacks.

## Tabard camera target

A `TabardModel` pane keeps the camera its model-ready hook chose, then moves the camera's target to
(0, 0.33·(max y − min y), 0.5·(max z − min z)) of the bounds of the model's animation 0. It does nothing
when there is no camera.

Addresses: `0x503c10`, `0x503c1e`, `0x503c23`, `0x503c2b`, `0x503c6e`, `0x503c7f`, `0x503c9c`, `0x503ca5`, `0x711a20`, `0x7ac250`, `0x808300`
Confidence: verified
Proof: one
Test: none

`0x503c10` calls `0x505b30` (`0x503c1e`), then reads `+0x31c` (`0x503c23`) and returns when it is null
(`0x503c2b`). `0x711a20(0, 0, &info)` fills the sequence record whose bounds are at `info + 0xc`: the
minima then maxima (`0x503c6e`). The target y is the y extent times the float 0.33 (`0x503c7f`) and z the z
extent times 0.5 (`0x503c9c`); `0x7ac250(camera, 8, &v, 0)` stores it (`0x503ca5`).
