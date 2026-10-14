# animation: model hosts

What plays the animation of a model that no unit controls: the first sequence a model arms, the completion callbacks the
placed-doodad and game object code install, the per-frame registration that puts a model on its scene's animate list, and the
clock a model's global sequences run from. A map doodad, a world game object and a spell effect all go through the same model
instance (`models/model`); they differ in who installs the callbacks and who registers the model each frame.

Roots: none

## Default sequence

When an instance is built from its shared model (`0x70ebd0`), it sets its loaded flag `+0x10` and arms a default sequence before it
replays the deferred calls. The sequence is animation 0 when the model has the animation that id 0 resolves to, else the first
dword of the first sequence record (animation id and variation). The call is `PlayAnimation(-1, id, 0, 0, 1.0, 0, 1)`: the variation
is 0, the head of the chain, and there is no blend.

Addresses: `0x70ebd0`, `0x71015b`, `0x710162`, `0x710169`, `0x71016e`, `0x710174`, `0x710179`, `0x710184`, `0x710187`, `0x71019b`, `0x711bf0`, `0x711960`, `0x7121a0`
Confidence: verified
Proof: one
Test: none

`0x710169 call 0x711bf0` resolves id 0 and `0x710174 call 0x711960` asks the model for the resolved id. A nonzero answer takes
`0x71017d xor eax,eax`; zero takes `[[md+0x20]]` (`0x710184`, `0x710187`). The pushes at `0x71018b` to `0x710197` are 0, 1.0, 0, 0 for blend,
speed, offset and variation, the id, and −1 for the key bone, with a primary flag of 1.

## Doodad model

`CreateDoodadModel(def, wait)` (`0x695100`) makes a placed doodad's model and arms it. It creates the instance through the scene, sets
its transform from the def, installs the done callback `DoodadModelDone` (`0x6951b0`), the event callback `0x6951e0` and the lighting
callback `0x672a20`, and calls `PlayAnimation(-1, 0, -1, 0, 1.0, 1, 1)`: Stand with a drawn variation and a blend. When the done
callback fires with reason 0, it plays the finished animation again on the same key bone with variation −1, speed 1.0 and no blend,
so a doodad redraws its variation every time a window ends.

Addresses: `0x695100`, `0x695114`, `0x69512c`, `0x695140`, `0x695153`, `0x695164`, `0x695180`, `0x69519b`, `0x6951b0`, `0x6951b6`, `0x6951c8`, `0x6951cc`, `0x694bb0`, `0x694e79`, `0x6950e9`
Confidence: verified
Proof: one
Test: none

`0x695140 call 0x711bb0` installs the done callback (`0x711bb0` stores it at `+0x70` and its two arguments at `+0x78` and `+0x7c`), `0x695153 call 0x7133a0` the event callback and `0x695164 call 0x7134b0` the lighting
callback. The pushes before `0x695180` are 1, 1, 1.0, 0, −1, 0, −1: primary, blend, speed, offset, variation, id, key bone. In
`0x6951b0` the `test eax,eax; jne` on the second stack argument lets only reason 0 through, and the pushes before `0x6951cc call
0x7121a0` are 1, 0, 1.0, 0, −1, the finished id and the key bone in `edx`. Three sites call `CreateDoodadModel`.

## Doodad set command

Activating a doodad set of a map object plays one animation on every doodad of the set. `0x6a20f0(animation)` walks the group's doodad
list and, for each doodad with a model, calls `PlayAnimation(-1, animation, -1, 0, 1.0, 1, 1)`: the animation with a drawn variation,
a blend, as primary.

Addresses: `0x6a20f0`, `0x6a20f8`, `0x6a2119`, `0x6a211c`, `0x6a2122`, `0x6a2126`, `0x6a2133`, `0x6a2136`, `0x6a213b`, `0x6a2150`
Confidence: verified
Proof: one
Test: none

The list head is `[this+0xb0]` with the link offset `[this+0xa8]` (`0x6a2141`). The doodad's model is `[node+4]` then `+0x88`
(`0x6a211c`). The pushes before `0x6a2136` are 1, 1, 1.0, 0, −1, the animation and −1.

## Animate list

A scene animates the unattached instances on its animate list `+0x20` each frame and drains the list afterwards, so an instance stays
animated only while its owner puts it back on the list with `SetAnimated(model, 1)` every frame. Units do it in their model update
(`0x607e43`), missiles and effects in their frame code (`0x61db80`, `0x61e743`, `0x620537`, `0x620de5`), and model frames of the
interface from their draw. The terrain does it for map doodads and for game object entities, in the culls (`0x6834b3`,
`0x6836d6`, `0x6838d7`, `0x683a8f`) and the draws (`0x683e49`, `0x684155`); a cull passes 0 for an instance out of range, which removes it.
Attaching an instance to a parent passes 0.

Addresses: `0x710b90`, `0x607e43`, `0x61db80`, `0x61e743`, `0x620537`, `0x620de5`, `0x6834b3`, `0x6834c6`, `0x6836d6`, `0x6838c1`, `0x6838d7`, `0x683a8f`, `0x683e49`, `0x684155`, `0x712f79`, `0x683700`, `0x683f80`, `0x670db0`
Confidence: verified
Proof: one
Test: none

A scan of the listing finds 32 `call 0x710b90` sites and no stored address of it. The entity cull `0x683700` walks the entity records whose
model is at `+0x88`, the field the entity factory `0x670db0` fills with a game object's model; it calls `SetAnimated(model, 1)` for a record with
flag `+0x90 & 0x200` or inside a squared distance (`0x6838d7`) and `SetAnimated(model, 0)` otherwise (`0x6838c1`). `DrawEntityModels` (`0x683f80`)
passes 1 (`0x684153`, `0x684155`) for each queued entity its distance fade leaves visible, and then sets its visibility, lighting callback and alpha. The other sites are interface and sky code. `0x712f79` is the attach call.

## Global sequence phase

An instance's global sequences run from the scene clock at the moment it is attached. `0x70ea00` stores the scene clock `[[scene]+0xc]` in `+0x68`
(`0x70eae1`) and `models/animation#global-sequence-time` reduces `clock − +0x68` modulo each length. Every `CreateModel` builds a new
instance, so every play of a spell effect starts its global sequences at phase 0. The instance animates only once its shared model has been
read: when the data is resident the build runs inside the attach, and otherwise when the read completes, so a first frame after a cold read
sits at the read latency modulo the length.

Addresses: `0x70ea00`, `0x70ea0d`, `0x70ea5d`, `0x70eae1`, `0x70ebb6`, `0x71d5a0`, `0x71d5a3`, `0x71d5a7`, `0x71d5cd`, `0x71d640`, `0x71d66e`, `0x71d675`, `0x71d6a0`, `0x70ebd0`, `0x710d70`, `0x707350`
Confidence: verified
Proof: one
Test: none

`0x71d5a3 test byte [ecx+0x8],1; jne 0x71d5cd` calls `0x70ebd0` at once for a ready shared model; otherwise `0x71d5a9` to `0x71d5c1` link
the instance into the pending list `[shared+0x10]`. `0x71d640` sets the ready bit (`0x71d675`) and calls `0x70ebd0` on each queued instance
(`0x71d6a0`). The only other writer of `+0x68`, `SetGlobalTime` (`0x710d70`), has no caller and no stored address. `0x70ea5d` stores the
scene frame less 1 in `+0x40`, so a new instance animates in its first frame.

## Game object model

A game object's model reaches the type handler through two callbacks. `CGObject_C::SetModel` (`0x613cf0`) installs `ModelLoaded` (`0x613d70`) on
the model with the object as its argument, and the object's slot 13 runs once the model is loaded. The creation code (`0x5f7c40`) installs the
done callback `0x5f7df0` on the model, which calls the handler's slot 13 for a nonzero reason and slot 14 for reason 0. The state machine,
its animation ids and its replay of resting states are `objectlayer/gameobjecttypes#animation-states`.

Addresses: `0x613cf0`, `0x613d38`, `0x613d70`, `0x613d79`, `0x5f7c40`, `0x5f7d43`, `0x5f7d4a`, `0x5f7df0`, `0x5f7e1c`, `0x5f7e22`, `0x5f7e29`, `0x5f7e2b`, `0x5f4120`, `0x7105a0`
Confidence: verified
Proof: one
Test: none

`0x613d38 push 0x613d70; call 0x7105a0` registers the loaded callback. `0x613d70` calls `[[arg]+0x34]`, the object's slot 13. In `0x5f7df0`, `0x5f7e1c test eax,eax`
tests the second stack argument, the reason: nonzero calls `[edx+0x34]`, zero `[eax+0x38]`, slot 14, whose first case table is `0x5f41e4`. The
handler is `[object+0x210]`.
